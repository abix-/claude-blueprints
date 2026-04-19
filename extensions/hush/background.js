const STORAGE_KEY = "config";
const OPTIONS_KEY = "options";
const ALLOWLIST_KEY = "allowlist";
const SESSION_TABSTATS_KEY = "tabStats";
const SESSION_BEHAVIOR_KEY = "tabBehavior";
const MAX_LOG_ENTRIES = 300;
const MAX_EVIDENCE = 50;
const MAX_SEEN_RESOURCES = 500;
const MAX_JS_CALLS = 500;

// Default allowlist - known-legit things the behavioral detector shouldn't
// surface as suggestions. Seeded into storage on first install; user can
// add/remove entries via the options page.
//
// iframes: URL substrings. If an iframe's src contains any entry, skip.
// overlays: CSS selectors. If a flagged sticky element matches any selector,
//           skip (covers React Portals, modal roots, framework root elements).
const DEFAULT_ALLOWLIST = {
  iframes: [
    "google.com/recaptcha",
    "gstatic.com/recaptcha",
    "hcaptcha.com",
    "challenges.cloudflare.com",
    "turnstile.cloudflare.com",
    "stripe.com",
    "paypal.com",
    "paypalobjects.com",
    "braintreegateway.com",
    "braintree-api.com",
    "adyen.com",
    "squareup.com",
    "squarecdn.com",
    "accounts.google.com",
    "accounts.youtube.com",
    "appleid.apple.com",
    "login.microsoftonline.com",
    "login.live.com",
    "firebaseapp.com",
    "auth0.com",
    "okta.com"
  ],
  overlays: [
    "#portal",
    "[id^=\"portal-\"]",
    "[id^=\"portal_\"]",
    "#modal-root",
    "#modal-container",
    "#overlay-root",
    "#__next",
    "#__nuxt",
    "#root",
    "#app",
    ".ReactModalPortal",
    ".MuiPopover-root",
    ".MuiModal-root",
    "[class*=\"radix-\"]"
  ]
};

let debugLogging = false;
const logBuffer = [];

function safeStringify(v) {
  if (v == null) return String(v);
  if (typeof v === "string") return v;
  if (v instanceof Error) return v.stack || v.message;
  try {
    return JSON.stringify(v);
  } catch (e) {
    return String(v);
  }
}

function pushLog(level, source, args) {
  logBuffer.push({
    t: new Date().toISOString(),
    level,
    source,
    msg: args.map(safeStringify).join(" ")
  });
  if (logBuffer.length > MAX_LOG_ENTRIES) {
    logBuffer.splice(0, logBuffer.length - MAX_LOG_ENTRIES);
  }
}

function log(...args) {
  pushLog("info", "bg", args);
  if (debugLogging) console.log("[Hush bg]", ...args);
}

function logError(...args) {
  pushLog("error", "bg", args);
  console.error("[Hush bg]", ...args);
}

async function loadOptions() {
  const data = await chrome.storage.local.get(OPTIONS_KEY);
  return data[OPTIONS_KEY] || {};
}

// In-memory cache of the allowlist. Updated whenever storage changes so
// detector paths don't hit async storage on every iframe/overlay check.
let allowlistCache = { iframes: [], overlays: [] };

async function loadAllowlist() {
  const data = await chrome.storage.local.get(ALLOWLIST_KEY);
  const raw = data[ALLOWLIST_KEY] || {};
  allowlistCache = {
    iframes: Array.isArray(raw.iframes) ? raw.iframes : [],
    overlays: Array.isArray(raw.overlays) ? raw.overlays : []
  };
  return allowlistCache;
}

async function seedAllowlistIfEmpty() {
  const existing = await chrome.storage.local.get(ALLOWLIST_KEY);
  if (existing[ALLOWLIST_KEY]) return;
  await chrome.storage.local.set({ [ALLOWLIST_KEY]: DEFAULT_ALLOWLIST });
}

async function refreshDebugFlag() {
  const opts = await loadOptions();
  debugLogging = !!opts.debug;
}

async function loadConfig() {
  const data = await chrome.storage.local.get(STORAGE_KEY);
  return data[STORAGE_KEY] || {};
}

function findConfigEntry(config, host) {
  if (config[host]) return { key: host, cfg: config[host] };
  for (const key of Object.keys(config)) {
    if (host === key || host.endsWith("." + key)) {
      return { key, cfg: config[key] };
    }
  }
  return null;
}

// Map ruleId -> { pattern, domain } so blocked-URL entries can show which rule matched.
const rulePatterns = new Map();

// Map ruleId -> fire count. Resets when the config is re-synced (new rule ids).
const ruleFireCount = new Map();

let syncChain = Promise.resolve();
function syncDynamicRules() {
  const next = syncChain.then(() => doSyncDynamicRules());
  syncChain = next.catch(() => {});
  return next;
}

async function doSyncDynamicRules() {
  const config = await loadConfig();
  const existing = await chrome.declarativeNetRequest.getDynamicRules();
  const removeRuleIds = existing.map(r => r.id);

  const addRules = [];
  rulePatterns.clear();
  ruleFireCount.clear();
  let nextId = 1;

  for (const [domain, cfg] of Object.entries(config)) {
    if (!cfg || !Array.isArray(cfg.block)) continue;
    for (const pattern of cfg.block) {
      if (typeof pattern !== "string" || !pattern.trim()) continue;
      const id = nextId++;
      rulePatterns.set(id, { pattern, domain });
      addRules.push({
        id,
        priority: 1,
        action: { type: "block" },
        condition: {
          urlFilter: pattern,
          initiatorDomains: [domain]
        }
      });
    }
  }

  try {
    await chrome.declarativeNetRequest.updateDynamicRules({
      removeRuleIds,
      addRules
    });
    log("synced dynamic rules: removed", removeRuleIds.length, "added", addRules.length);
  } catch (e) {
    logError("failed to update dynamic rules", e);
  }
}

async function seedIfEmpty() {
  const existing = await chrome.storage.local.get(STORAGE_KEY);
  if (existing[STORAGE_KEY]) return;
  try {
    const url = chrome.runtime.getURL("sites.json");
    const seed = await fetch(url).then(r => r.json());
    await chrome.storage.local.set({ [STORAGE_KEY]: seed });
  } catch (e) {
    logError("failed to load seed config", e);
    await chrome.storage.local.set({ [STORAGE_KEY]: {} });
  }
}

// ================= Per-tab activity stats =================

const tabStats = new Map();

function emptyStats() {
  return {
    matchedDomain: null,
    hide: {},
    remove: {},
    block: 0,
    blockedUrls: [],
    removedElements: []
  };
}

function getStats(tabId) {
  let s = tabStats.get(tabId);
  if (!s) {
    s = emptyStats();
    tabStats.set(tabId, s);
  }
  if (!Array.isArray(s.blockedUrls)) s.blockedUrls = [];
  if (!Array.isArray(s.removedElements)) s.removedElements = [];
  return s;
}

function resetStats(tabId) {
  tabStats.set(tabId, emptyStats());
  updateBadge(tabId);
  schedulePersist();
}

function totalActivity(stats) {
  const hideTotal = Object.values(stats.hide).reduce((a, b) => a + b, 0);
  const removeTotal = Object.values(stats.remove).reduce((a, b) => a + b, 0);
  return hideTotal + removeTotal + stats.block;
}

function updateBadge(tabId) {
  const stats = tabStats.get(tabId);
  const behavior = tabBehavior.get(tabId);
  const suggCount = behavior && Array.isArray(behavior.suggestions) ? behavior.suggestions.length : 0;
  const total = stats ? totalActivity(stats) : 0;

  // Suggestion warning takes priority - user needs to address those.
  if (suggCount > 0) {
    chrome.action.setBadgeText({ tabId, text: "!" }).catch(() => {});
    chrome.action.setBadgeBackgroundColor({ tabId, color: "#e8a200" }).catch(() => {});
    return;
  }

  // No suggestions - fall back to activity count in the default grey.
  const text = total > 0 ? String(total) : "";
  chrome.action.setBadgeText({ tabId, text }).catch(() => {});
  chrome.action.setBadgeBackgroundColor({ tabId, color: "#666" }).catch(() => {});
}

let persistTimer = null;
function schedulePersist() {
  if (persistTimer) return;
  persistTimer = setTimeout(async () => {
    persistTimer = null;
    const obj = {};
    for (const [tabId, stats] of tabStats) obj[tabId] = stats;
    try {
      await chrome.storage.session.set({ [SESSION_TABSTATS_KEY]: obj });
    } catch (e) {
      logError("persist tabStats failed", e);
    }
  }, 500);
}

async function hydrateTabStats() {
  try {
    const data = await chrome.storage.session.get(SESSION_TABSTATS_KEY);
    const obj = data[SESSION_TABSTATS_KEY];
    if (obj && typeof obj === "object") {
      for (const [tabIdStr, stats] of Object.entries(obj)) {
        const tabId = parseInt(tabIdStr, 10);
        if (!Number.isNaN(tabId)) tabStats.set(tabId, stats);
      }
      log("hydrated tabStats for", tabStats.size, "tab(s) from session storage");
    }
  } catch (e) {
    logError("hydrate tabStats failed", e);
  }
}

function capList(arr, max) {
  if (arr.length > max) arr.splice(0, arr.length - max);
}

// ================= Per-tab behavioral state + suggestions =================

const tabBehavior = new Map();

function emptyBehavior() {
  return {
    pageHost: null,
    seenResources: [],
    latestIframes: [],
    latestStickies: [],
    jsCalls: [],         // deep trace from main-world hooks
    dismissed: [],       // array of suggestion keys
    suggestions: []
  };
}

function getBehavior(tabId) {
  let b = tabBehavior.get(tabId);
  if (!b) {
    b = emptyBehavior();
    tabBehavior.set(tabId, b);
  }
  if (!Array.isArray(b.seenResources)) b.seenResources = [];
  if (!Array.isArray(b.latestIframes)) b.latestIframes = [];
  if (!Array.isArray(b.latestStickies)) b.latestStickies = [];
  if (!Array.isArray(b.jsCalls)) b.jsCalls = [];
  if (!Array.isArray(b.dismissed)) b.dismissed = [];
  if (!Array.isArray(b.suggestions)) b.suggestions = [];
  return b;
}

function resetBehavior(tabId) {
  tabBehavior.set(tabId, emptyBehavior());
  schedulePersistBehavior();
}

let persistBehaviorTimer = null;
function schedulePersistBehavior() {
  if (persistBehaviorTimer) return;
  persistBehaviorTimer = setTimeout(async () => {
    persistBehaviorTimer = null;
    const obj = {};
    for (const [tabId, state] of tabBehavior) obj[tabId] = state;
    try {
      await chrome.storage.session.set({ [SESSION_BEHAVIOR_KEY]: obj });
    } catch (e) {
      logError("persist behavior failed", e);
    }
  }, 500);
}

async function hydrateBehavior() {
  try {
    const data = await chrome.storage.session.get(SESSION_BEHAVIOR_KEY);
    const obj = data[SESSION_BEHAVIOR_KEY];
    if (obj && typeof obj === "object") {
      for (const [tabIdStr, state] of Object.entries(obj)) {
        const tabId = parseInt(tabIdStr, 10);
        if (!Number.isNaN(tabId)) tabBehavior.set(tabId, state);
      }
      log("hydrated behavior for", tabBehavior.size, "tab(s)");
    }
  } catch (e) {
    logError("hydrate behavior failed", e);
  }
}

function hostOf(url) {
  try { return new URL(url).host; } catch (e) { return ""; }
}

// Given a DNR urlFilter pattern, pull out the longest stable "keyword"
// substring - the part with no wildcards or anchors. Used to diagnose
// whether observed URLs LOOK like they should be matched by the pattern,
// even if DNR itself isn't firing the rule.
function patternKeyword(pattern) {
  if (!pattern) return "";
  // Strip control chars: ||, ^, *, | leaving just the literal parts.
  const parts = pattern.replace(/[|^*]/g, " ").split(/\s+/).filter(Boolean);
  if (!parts.length) return "";
  // Prefer longer, more distinctive parts.
  parts.sort((a, b) => b.length - a.length);
  return parts[0];
}

// For each configured block rule, compute diagnostic info:
//   - fired: how many times it has matched a request
//   - observedMatches: URLs in this tab's observed traffic whose text
//     contains the pattern's keyword (heuristic: if the user requested
//     something that LOOKS like it should match, but the rule hasn't fired,
//     the pattern is likely wrong)
//   - status: "firing" | "no-traffic" | "pattern-broken"
function computeRuleDiagnostics(tabId, hostname) {
  const behavior = typeof tabId === "number" ? tabBehavior.get(tabId) : null;
  const observed = behavior ? behavior.seenResources : [];
  const host = hostname || (behavior && behavior.pageHost) || null;

  const diagnostics = [];
  for (const [ruleId, meta] of rulePatterns) {
    // Only include rules whose initiator-domain applies to this page.
    // A rule with initiator "example.com" matches hostname "www.example.com"
    // or any other subdomain.
    const initiator = meta.domain || "";
    if (host && initiator) {
      const hostMatches = host === initiator || host.endsWith("." + initiator);
      if (!hostMatches) continue;
    }

    const pattern = meta.pattern || "";
    const keyword = patternKeyword(pattern);
    const fired = ruleFireCount.get(ruleId) || 0;
    const matches = keyword
      ? observed.filter(r => r.url && r.url.includes(keyword)).slice(-5)
      : [];
    let status;
    if (fired > 0) {
      status = "firing";
    } else if (matches.length > 0) {
      status = "pattern-broken";
    } else {
      status = "no-traffic";
    }
    diagnostics.push({
      ruleId,
      pattern,
      initiator,
      fired,
      keyword,
      status,
      matchingUrls: matches.map(r => r.url)
    });
  }
  return diagnostics;
}

function isSubdomainOf(candidate, parent) {
  return candidate !== parent && candidate.endsWith("." + parent);
}

// Hidden iframes that match the user-configurable allowlist are skipped
// from suggestions. Each entry is a case-insensitive URL substring - if
// the iframe's src contains the entry, it's considered legit. Covers
// captcha, OAuth, payment, bot-management by default.
function isLegitHiddenIframe(srcUrl) {
  if (!srcUrl) return false;
  const url = String(srcUrl).toLowerCase();
  const list = (allowlistCache && allowlistCache.iframes) || [];
  for (const entry of list) {
    if (!entry || typeof entry !== "string") continue;
    if (url.includes(entry.toLowerCase())) return true;
  }
  return false;
}

function median(arr) {
  if (!arr.length) return 0;
  const s = arr.slice().sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
}

function canonicalizeUrl(url) {
  const noiseParams = ["t", "ts", "_", "nonce", "cb", "callback", "v", "_t", "rand"];
  try {
    const u = new URL(url);
    for (const key of [...u.searchParams.keys()]) {
      if (noiseParams.includes(key.toLowerCase())) u.searchParams.delete(key);
    }
    return u.origin + u.pathname + (u.searchParams.toString() ? "?" + u.searchParams.toString() : "");
  } catch (e) {
    return url;
  }
}

function computeSuggestions(state, config) {
  const hostname = state.pageHost || "";
  if (!hostname) return [];
  const match = findConfigEntry(config, hostname);
  const cfg = match ? match.cfg : {};
  // Normalize existing block patterns by stripping the optional trailing ^.
  // Suggestions are always generated without trailing ^ (the ^ is functionally
  // redundant after a ||domain anchor and can cause match failures on
  // hyphenated subdomains in Chrome's DNR), so dedup needs to compare
  // normalized forms to recognize existing rules written either way.
  const existingBlock = new Set(
    (Array.isArray(cfg.block) ? cfg.block : []).map(p =>
      typeof p === "string" && p.endsWith("^") ? p.slice(0, -1) : p
    )
  );
  const existingRemove = new Set(Array.isArray(cfg.remove) ? cfg.remove : []);
  const existingHide = new Set(Array.isArray(cfg.hide) ? cfg.hide : []);
  const dismissed = new Set(state.dismissed);

  const resources = state.seenResources;
  const out = [];

  // 1. sendBeacon targets -> block (very high confidence)
  const beaconByHost = new Map();
  for (const r of resources) {
    if (r.initiatorType !== "beacon") continue;
    if (!r.host || r.host === hostname) continue;
    const arr = beaconByHost.get(r.host) || [];
    arr.push(r);
    beaconByHost.set(r.host, arr);
  }
  for (const [host, hits] of beaconByHost) {
    const value = "||" + host;
    if (existingBlock.has(value)) continue;
    out.push({
      key: "block::" + value,
      layer: "block",
      value,
      reason: "sendBeacon target (" + hits.length + " beacon" + (hits.length > 1 ? "s" : "") + " sent)",
      confidence: 95,
      count: hits.length,
      evidence: hits.slice(0, 5).map(h => h.url)
    });
  }

  // 2. Tracking pixels -> block (high)
  const pixelByHost = new Map();
  for (const r of resources) {
    if (r.initiatorType !== "img") continue;
    if (!r.host || r.host === hostname) continue;
    if (r.transferSize <= 0 || r.transferSize >= 200) continue;
    const arr = pixelByHost.get(r.host) || [];
    arr.push(r);
    pixelByHost.set(r.host, arr);
  }
  for (const [host, hits] of pixelByHost) {
    const value = "||" + host;
    if (existingBlock.has(value)) continue;
    const med = median(hits.map(h => h.transferSize));
    out.push({
      key: "block::" + value,
      layer: "block",
      value,
      reason: "tracking pixels: " + hits.length + " tiny image" + (hits.length > 1 ? "s" : "") + " (median " + med + "b)",
      confidence: 85,
      count: hits.length,
      evidence: hits.slice(0, 5).map(h => h.url + " (" + h.transferSize + "b)")
    });
  }

  // 3. First-party telemetry subdomains -> block (medium)
  const subByHost = new Map();
  for (const r of resources) {
    if (!r.host || r.host === hostname) continue;
    if (!isSubdomainOf(r.host, hostname)) continue;
    const arr = subByHost.get(r.host) || [];
    arr.push(r);
    subByHost.set(r.host, arr);
  }
  for (const [host, requests] of subByHost) {
    const sizes = requests.filter(r => r.transferSize > 0).map(r => r.transferSize);
    if (!sizes.length) continue;
    const med = median(sizes);
    const max = Math.max(...sizes);
    if (med >= 1024 || max >= 5120) continue;
    const value = "||" + host;
    if (existingBlock.has(value)) continue;
    out.push({
      key: "block::" + value,
      layer: "block",
      value,
      reason: "first-party subdomain with " + requests.length + " tiny response" + (requests.length > 1 ? "s" : "") + " (median " + med + "b)",
      confidence: 70,
      count: requests.length,
      evidence: requests.slice(0, 5).map(r => r.url + " (" + r.transferSize + "b, " + r.initiatorType + ")")
    });
  }

  // 4. Polling -> block (medium-high)
  const byCanon = new Map();
  for (const r of resources) {
    if (!r.host || r.host === hostname) continue;
    const canon = canonicalizeUrl(r.url);
    const entry = byCanon.get(canon) || { count: 0, sizes: [], firstSeen: r.startTime, lastSeen: r.startTime, host: r.host, sample: r.url };
    entry.count++;
    entry.sizes.push(r.transferSize);
    if (r.startTime < entry.firstSeen) entry.firstSeen = r.startTime;
    if (r.startTime > entry.lastSeen) entry.lastSeen = r.startTime;
    byCanon.set(canon, entry);
  }
  for (const [canon, info] of byCanon) {
    if (info.count < 4) continue;
    const span = info.lastSeen - info.firstSeen;
    if (span < 5000 || span > 600000) continue;
    const med = median(info.sizes);
    if (med >= 2048) continue;
    const value = "||" + info.host + "^";
    const key = "block::" + value;
    if (existingBlock.has(value)) continue;
    if (out.find(s => s.key === key)) continue; // already added via another signal
    out.push({
      key,
      layer: "block",
      value,
      reason: "polled " + info.count + "x over " + Math.round(span / 1000) + "s (median " + med + "b)",
      confidence: 75,
      count: info.count,
      evidence: [info.sample]
    });
  }

  // 5. Hidden iframes -> remove (high), excluding known-legit sources
  const iframeByHost = new Map();
  for (const f of state.latestIframes) {
    if (!f.src || !f.host) continue;
    if (isLegitHiddenIframe(f.src)) continue; // captcha / oauth / payment / etc.
    const entry = iframeByHost.get(f.host) || { host: f.host, reasons: new Set(), samples: [] };
    for (const r of f.reasons || []) entry.reasons.add(r);
    entry.samples.push(f);
    iframeByHost.set(f.host, entry);
  }
  for (const [host, info] of iframeByHost) {
    const selector = 'iframe[src*="' + host + '"]';
    if (existingRemove.has(selector)) continue;
    out.push({
      key: "remove::" + selector,
      layer: "remove",
      value: selector,
      reason: "hidden iframe: " + Array.from(info.reasons).join(", "),
      confidence: 80,
      count: info.samples.length,
      evidence: info.samples.slice(0, 3).map(s => s.outerHTMLPreview)
    });
  }

  // 6. Sticky overlays -> hide (medium)
  const stickySeen = new Set();
  for (const s of state.latestStickies) {
    if (!s.selector || stickySeen.has(s.selector)) continue;
    stickySeen.add(s.selector);
    if (existingHide.has(s.selector)) continue;
    out.push({
      key: "hide::" + s.selector,
      layer: "hide",
      value: s.selector,
      reason: "fixed overlay covering " + s.coverage + "% of viewport (z-index " + s.zIndex + ")",
      confidence: 55,
      count: 1,
      evidence: [s.rect.w + "x" + s.rect.h + " at z-index " + s.zIndex]
    });
  }

  // Apply dismissals
  return out
    .filter(s => !dismissed.has(s.key))
    .sort((a, b) => (b.confidence - a.confidence) || (b.count - a.count));
}

// ================= Setup listeners =================

chrome.runtime.onInstalled.addListener(async () => {
  chrome.action.setBadgeBackgroundColor({ color: "#666" }).catch(() => {});
  await refreshDebugFlag();
  await seedIfEmpty();
  await seedAllowlistIfEmpty();
  await loadAllowlist();
  await syncDynamicRules();
});

chrome.runtime.onStartup.addListener(async () => {
  chrome.action.setBadgeBackgroundColor({ color: "#666" }).catch(() => {});
  await refreshDebugFlag();
  await loadAllowlist();
  syncDynamicRules();
});

(async () => {
  await refreshDebugFlag();
  await loadAllowlist();
  await hydrateTabStats();
  await hydrateBehavior();
  await rehydrateRulePatterns();
  log("service worker started / woke up");
})();

// Rebuild the ruleId -> { pattern, domain } map from whatever dynamic rules
// are currently live. Needed because the SW can shut down on idle and lose
// in-memory state, but Chrome persists the DNR rules themselves. Without
// this, onRuleMatchedDebug would see ruleIds it can't map back to patterns
// and the popup's Blocked section shows "(unknown rule)".
async function rehydrateRulePatterns() {
  try {
    const existing = await chrome.declarativeNetRequest.getDynamicRules();
    rulePatterns.clear();
    for (const rule of existing) {
      const pattern = rule.condition && rule.condition.urlFilter;
      const domains = rule.condition && rule.condition.initiatorDomains;
      rulePatterns.set(rule.id, {
        pattern: pattern || "",
        domain: (domains && domains[0]) || ""
      });
    }
    log("rehydrated rulePatterns for", rulePatterns.size, "rule(s)");
  } catch (e) {
    logError("rehydrateRulePatterns failed", e);
  }
}

chrome.storage.onChanged.addListener((changes, area) => {
  if (area !== "local") return;
  if (OPTIONS_KEY in changes) {
    const v = changes[OPTIONS_KEY].newValue;
    debugLogging = !!(v && v.debug);
    log("debug logging ->", debugLogging);
  }
  if (STORAGE_KEY in changes) {
    syncDynamicRules();
  }
  if (ALLOWLIST_KEY in changes) {
    loadAllowlist();
    log("allowlist updated");
  }
});

chrome.webNavigation.onCommitted.addListener(details => {
  if (details.frameId !== 0) return;
  resetStats(details.tabId);
  resetBehavior(details.tabId);
  log("nav committed, reset tab", details.tabId, details.url);
});

chrome.tabs.onRemoved.addListener(tabId => {
  tabStats.delete(tabId);
  tabBehavior.delete(tabId);
  schedulePersist();
  schedulePersistBehavior();
});

// onRuleMatchedDebug only fires for unpacked extensions.
chrome.declarativeNetRequest.onRuleMatchedDebug.addListener(info => {
  const tabId = info.request && info.request.tabId;
  const ruleId = info.rule && info.rule.ruleId;
  const url = info.request && info.request.url;
  const ruleMeta = rulePatterns.get(ruleId) || {};
  if (typeof ruleId === "number") {
    ruleFireCount.set(ruleId, (ruleFireCount.get(ruleId) || 0) + 1);
  }
  log("rule matched:", ruleId, "pattern:", ruleMeta.pattern, "url:", url, "tabId:", tabId);
  if (typeof tabId !== "number" || tabId < 0) return;
  const stats = getStats(tabId);
  stats.block += 1;
  stats.blockedUrls.push({
    t: new Date().toISOString(),
    url: url || "",
    ruleId,
    pattern: ruleMeta.pattern || "",
    domain: ruleMeta.domain || "",
    resourceType: info.request && info.request.type
  });
  capList(stats.blockedUrls, MAX_EVIDENCE);
  updateBadge(tabId);
  schedulePersist();
});

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (!msg || typeof msg !== "object") return;

  if (msg.type === "hush:stats") {
    const tabId = sender.tab && sender.tab.id;
    if (typeof tabId !== "number") return;
    const stats = getStats(tabId);
    if (msg.matchedDomain !== undefined) stats.matchedDomain = msg.matchedDomain;
    if (msg.hide && typeof msg.hide === "object") stats.hide = msg.hide;
    if (msg.remove && typeof msg.remove === "object") stats.remove = msg.remove;
    if (Array.isArray(msg.newRemovedElements) && msg.newRemovedElements.length) {
      for (const ev of msg.newRemovedElements) stats.removedElements.push(ev);
      capList(stats.removedElements, MAX_EVIDENCE);
    }
    updateBadge(tabId);
    schedulePersist();
    return;
  }

  if (msg.type === "hush:log") {
    const tabId = sender.tab && sender.tab.id;
    const level = msg.level === "error" ? "error" : "info";
    const args = Array.isArray(msg.args) ? msg.args : [msg.msg];
    pushLog(level, "content@tab" + (typeof tabId === "number" ? tabId : "?"), args);
    if (debugLogging) {
      (level === "error" ? console.error : console.log)(
        "[Hush content]", ...args
      );
    }
    return;
  }

  if (msg.type === "hush:js-calls") {
    const tabId = sender.tab && sender.tab.id;
    if (typeof tabId !== "number") return;
    if (!Array.isArray(msg.calls) || !msg.calls.length) return;
    const state = getBehavior(tabId);
    for (const c of msg.calls) state.jsCalls.push(c);
    if (state.jsCalls.length > MAX_JS_CALLS) {
      state.jsCalls.splice(0, state.jsCalls.length - MAX_JS_CALLS);
    }
    schedulePersistBehavior();
    return;
  }

  if (msg.type === "hush:scan") {
    const tabId = sender.tab && sender.tab.id;
    if (typeof tabId !== "number") return;
    (async () => {
      const state = getBehavior(tabId);
      state.pageHost = msg.hostname || state.pageHost;
      // Merge resources (dedupe by url + startTime)
      if (Array.isArray(msg.resources)) {
        const seen = new Set(state.seenResources.map(r => r.url + "@" + r.startTime));
        for (const r of msg.resources) {
          const k = r.url + "@" + r.startTime;
          if (seen.has(k)) continue;
          seen.add(k);
          state.seenResources.push(r);
        }
        if (state.seenResources.length > MAX_SEEN_RESOURCES) {
          state.seenResources.splice(0, state.seenResources.length - MAX_SEEN_RESOURCES);
        }
      }
      if (Array.isArray(msg.iframes)) state.latestIframes = msg.iframes;
      if (Array.isArray(msg.stickies)) state.latestStickies = msg.stickies;

      const config = await loadConfig();
      state.suggestions = computeSuggestions(state, config);
      schedulePersistBehavior();
      updateBadge(tabId);
      log("scan merged for tab", tabId, "- suggestions:", state.suggestions.length);
    })();
    return;
  }

  if (msg.type === "hush:get-tab-stats") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : (sender.tab && sender.tab.id);
    if (typeof tabId !== "number") {
      sendResponse({ stats: null });
      return false;
    }
    sendResponse({ stats: tabStats.get(tabId) || emptyStats() });
    return false;
  }

  if (msg.type === "hush:get-rule-diagnostics") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : (sender.tab && sender.tab.id);
    const hostname = typeof msg.hostname === "string" ? msg.hostname : null;
    sendResponse({ diagnostics: computeRuleDiagnostics(tabId, hostname) });
    return false;
  }

  if (msg.type === "hush:get-suggestions") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : (sender.tab && sender.tab.id);
    if (typeof tabId !== "number") {
      sendResponse({ suggestions: [], pageHost: null });
      return false;
    }
    (async () => {
      const state = getBehavior(tabId);
      const config = await loadConfig();
      // Recompute on read in case config changed since last scan.
      const suggestions = computeSuggestions(state, config);
      state.suggestions = suggestions;
      updateBadge(tabId);
      sendResponse({ suggestions, pageHost: state.pageHost });
    })();
    return true;
  }

  if (msg.type === "hush:accept-suggestion") {
    (async () => {
      try {
        const { hostname, layer, value } = msg;
        if (!hostname || !layer || !value) {
          sendResponse({ ok: false, error: "missing hostname/layer/value" });
          return;
        }
        const config = await loadConfig();
        const match = findConfigEntry(config, hostname);
        let targetKey;
        if (match) {
          targetKey = match.key;
        } else {
          targetKey = hostname;
          config[targetKey] = { hide: [], remove: [], block: [] };
        }
        const entry = config[targetKey];
        if (!Array.isArray(entry[layer])) entry[layer] = [];
        if (!entry[layer].includes(value)) entry[layer].push(value);
        await chrome.storage.local.set({ [STORAGE_KEY]: config });
        // storage.onChanged will re-sync DNR rules.

        // Drop the accepted suggestion from every tab's state + refresh badges.
        const acceptedKey = layer + "::" + value;
        for (const [tabId, state] of tabBehavior) {
          const before = state.suggestions.length;
          state.suggestions = state.suggestions.filter(s => s.key !== acceptedKey);
          if (state.suggestions.length !== before) {
            updateBadge(tabId);
          }
        }
        schedulePersistBehavior();
        sendResponse({ ok: true, configKey: targetKey });
      } catch (e) {
        logError("accept-suggestion failed", e);
        sendResponse({ ok: false, error: String(e) });
      }
    })();
    return true;
  }

  if (msg.type === "hush:dismiss-suggestion") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : (sender.tab && sender.tab.id);
    if (typeof tabId !== "number" || !msg.key) {
      sendResponse({ ok: false });
      return false;
    }
    const state = getBehavior(tabId);
    if (!state.dismissed.includes(msg.key)) state.dismissed.push(msg.key);
    state.suggestions = state.suggestions.filter(s => s.key !== msg.key);
    schedulePersistBehavior();
    updateBadge(tabId);
    sendResponse({ ok: true });
    return false;
  }

  if (msg.type === "hush:get-debug-info") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : null;
    (async () => {
      const manifest = chrome.runtime.getManifest();
      const config = await loadConfig();
      const options = await loadOptions();
      let dynamicRules = [];
      try {
        dynamicRules = await chrome.declarativeNetRequest.getDynamicRules();
      } catch (e) {
        dynamicRules = [{ error: String(e) }];
      }

      const stats = tabId !== null ? tabStats.get(tabId) : null;
      const behavior = tabId !== null ? tabBehavior.get(tabId) : null;
      const matchedDomain = stats && stats.matchedDomain;

      // Compact network rules: one line per rule.
      const compactRules = dynamicRules.map(r => ({
        id: r.id,
        pattern: r.condition && r.condition.urlFilter,
        initiator: r.condition && r.condition.initiatorDomains && r.condition.initiatorDomains[0]
      }));

      // Summarize behavior instead of dumping all 500 seen resources.
      const jsCallsByKind = {};
      if (behavior && Array.isArray(behavior.jsCalls)) {
        for (const c of behavior.jsCalls) {
          jsCallsByKind[c.kind] = (jsCallsByKind[c.kind] || 0) + 1;
        }
      }
      const behaviorSummary = behavior ? {
        pageHost: behavior.pageHost,
        seenResourceCount: behavior.seenResources.length,
        uniqueThirdPartyHostCount: new Set(behavior.seenResources.map(r => r.host).filter(h => h && h !== behavior.pageHost)).size,
        latestHiddenIframeCount: behavior.latestIframes.length,
        latestStickyCount: behavior.latestStickies.length,
        jsCallCount: behavior.jsCalls.length,
        jsCallsByKind,
        recentJsCalls: behavior.jsCalls.slice(-10).map(c => ({
          kind: c.kind,
          method: c.method,
          url: (c.url || "").slice(0, 150),
          bodyPreview: c.bodyPreview && c.bodyPreview.slice(0, 200),
          stackTop: (c.stack && c.stack[0]) || ""
        })),
        dismissedKeyCount: behavior.dismissed.length,
        suggestionCount: behavior.suggestions.length,
        suggestions: behavior.suggestions.map(s => ({
          layer: s.layer,
          value: s.value,
          reason: s.reason,
          confidence: s.confidence,
          count: s.count
        }))
      } : null;

      sendResponse({
        version: manifest.version,
        tabId,
        timestamp: new Date().toISOString(),
        options,
        configSiteCount: Object.keys(config).length,
        configSites: Object.keys(config),
        matchedDomain,
        matchedConfig: matchedDomain ? (config[matchedDomain] || null) : null,
        tabActivity: stats ? {
          totalBlocks: stats.block,
          totalHide: Object.values(stats.hide).reduce((a, b) => a + b, 0),
          totalRemove: Object.values(stats.remove).reduce((a, b) => a + b, 0),
          hide: stats.hide,
          remove: stats.remove,
          recentBlockedUrls: (stats.blockedUrls || []).slice(-10).map(b => ({
            t: b.t,
            url: (b.url || "").slice(0, 200),
            pattern: b.pattern,
            type: b.resourceType
          })),
          recentRemovedElements: (stats.removedElements || []).slice(-10)
        } : null,
        behavior: behaviorSummary,
        dynamicRules: compactRules,
        dynamicRuleCount: compactRules.length,
        recentLogs: logBuffer.slice(-40)
      });
    })();
    return true;
  }
});
