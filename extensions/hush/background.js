const STORAGE_KEY = "config";
const OPTIONS_KEY = "options";
const SESSION_TABSTATS_KEY = "tabStats";
const MAX_LOG_ENTRIES = 300;
const MAX_EVIDENCE = 50;

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

async function refreshDebugFlag() {
  const opts = await loadOptions();
  debugLogging = !!opts.debug;
}

async function loadConfig() {
  const data = await chrome.storage.local.get(STORAGE_KEY);
  return data[STORAGE_KEY] || {};
}

// Map ruleId -> { pattern, domain } so blocked-URL entries can show which rule matched.
const rulePatterns = new Map();

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
  // Back-compat: hydrate any missing evidence arrays on older stats objects.
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
  const total = stats ? totalActivity(stats) : 0;
  const text = total > 0 ? String(total) : "";
  chrome.action.setBadgeText({ tabId, text }).catch(() => {});
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

// ================= Setup listeners =================

chrome.runtime.onInstalled.addListener(async () => {
  chrome.action.setBadgeBackgroundColor({ color: "#666" }).catch(() => {});
  await refreshDebugFlag();
  await seedIfEmpty();
  await syncDynamicRules();
});

chrome.runtime.onStartup.addListener(async () => {
  chrome.action.setBadgeBackgroundColor({ color: "#666" }).catch(() => {});
  await refreshDebugFlag();
  syncDynamicRules();
});

(async () => {
  await refreshDebugFlag();
  await hydrateTabStats();
  log("service worker started / woke up");
})();

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
});

chrome.webNavigation.onCommitted.addListener(details => {
  if (details.frameId !== 0) return;
  resetStats(details.tabId);
  log("nav committed, reset stats for tab", details.tabId, details.url);
});

chrome.tabs.onRemoved.addListener(tabId => {
  tabStats.delete(tabId);
  schedulePersist();
});

chrome.declarativeNetRequest.onRuleMatchedDebug.addListener(info => {
  const tabId = info.request && info.request.tabId;
  const ruleId = info.rule && info.rule.ruleId;
  const url = info.request && info.request.url;
  const ruleMeta = rulePatterns.get(ruleId) || {};
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

  if (msg.type === "hush:get-tab-stats") {
    const tabId = typeof msg.tabId === "number" ? msg.tabId : (sender.tab && sender.tab.id);
    if (typeof tabId !== "number") {
      sendResponse({ stats: null });
      return false;
    }
    sendResponse({ stats: tabStats.get(tabId) || emptyStats() });
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
      sendResponse({
        version: manifest.version,
        tabId,
        options,
        tabStats: tabId !== null ? (tabStats.get(tabId) || null) : null,
        allTabStatsCount: tabStats.size,
        configSiteCount: Object.keys(config).length,
        configSites: Object.keys(config),
        tabMatchConfig: tabId !== null && tabStats.get(tabId) && tabStats.get(tabId).matchedDomain
          ? config[tabStats.get(tabId).matchedDomain] || null
          : null,
        dynamicRules,
        logs: logBuffer.slice(),
        timestamp: new Date().toISOString()
      });
    })();
    return true;
  }
});
