(async function main() {
  // ================================================================
  // Hush content script - runs at document_start on every page.
  //
  // Layer ordering (most aggressive to mildest):
  //   1. Block  - enforced in background.js via declarativeNetRequest
  //   2. Remove - physically deletes matching elements from the DOM
  //   3. Hide   - passive CSS stylesheet suppressing render of remaining matches
  //
  // Behavioral detector (opt-in, gated by options.suggestionsEnabled):
  //   observes resource requests, hidden iframes, and sticky overlays, emits
  //   scan snapshots to background for suggestion generation.
  // ================================================================

  const STORAGE_KEY = "config";
  const OPTIONS_KEY = "options";
  const ALLOWLIST_KEY = "allowlist";
  const MAX_LOCAL_REMOVED = 200;
  const MAX_BUFFERED_RESOURCES = 500;

  function findConfigEntry(config, host) {
    if (config[host]) return { key: host, cfg: config[host] };
    for (const key of Object.keys(config)) {
      if (host === key || host.endsWith("." + key)) {
        return { key, cfg: config[key] };
      }
    }
    return null;
  }

  function hostOf(url) {
    try { return new URL(url, location.href).host; } catch (e) { return ""; }
  }

  let config;
  let options;
  let allowlist = { iframes: [], overlays: [] };
  try {
    const data = await chrome.storage.local.get([STORAGE_KEY, OPTIONS_KEY, ALLOWLIST_KEY]);
    config = data[STORAGE_KEY];
    options = data[OPTIONS_KEY] || {};
    const al = data[ALLOWLIST_KEY];
    if (al && typeof al === "object") {
      allowlist.iframes = Array.isArray(al.iframes) ? al.iframes : [];
      allowlist.overlays = Array.isArray(al.overlays) ? al.overlays : [];
    }
  } catch (e) {
    return;
  }

  const debug = !!options.debug;
  const detectorEnabled = !!options.suggestionsEnabled;

  function safeStringifyArg(v) {
    if (v == null) return String(v);
    if (typeof v === "string") return v;
    if (v instanceof Error) return v.stack || v.message;
    try { return JSON.stringify(v); } catch (e) { return String(v); }
  }
  function log(...args) {
    if (debug) console.log("[Hush]", ...args);
    try {
      const p = chrome.runtime.sendMessage({
        type: "hush:log",
        level: "info",
        args: args.map(safeStringifyArg)
      });
      if (p && typeof p.catch === "function") p.catch(() => {});
    } catch (e) { /* extension context gone */ }
  }

  // --------------------------------------------------------------
  // Behavioral detector (always handles hush:scan-once messages;
  // continuous scanning only when detectorEnabled).
  // --------------------------------------------------------------

  const collectedResources = [];
  let resourceObserverInstalled = false;

  function installResourceObserver() {
    if (resourceObserverInstalled) return;
    resourceObserverInstalled = true;
    try {
      const obs = new PerformanceObserver((list) => {
        for (const e of list.getEntries()) {
          collectedResources.push({
            url: e.name,
            host: hostOf(e.name),
            initiatorType: e.initiatorType,
            transferSize: e.transferSize,
            duration: Math.round(e.duration),
            startTime: Math.round(e.startTime)
          });
        }
        if (collectedResources.length > MAX_BUFFERED_RESOURCES) {
          collectedResources.splice(0, collectedResources.length - MAX_BUFFERED_RESOURCES);
        }
      });
      obs.observe({ type: "resource", buffered: true });
    } catch (e) {
      log("PerformanceObserver not available:", e.message);
    }
  }

  function scanHiddenIframes() {
    const hits = [];
    document.querySelectorAll("iframe").forEach(f => {
      let cs;
      try { cs = getComputedStyle(f); } catch (e) { return; }
      const rect = f.getBoundingClientRect();
      const reasons = [];
      if (cs.display === "none") reasons.push("display:none");
      if (cs.visibility === "hidden") reasons.push("visibility:hidden");
      if (parseFloat(cs.opacity) === 0) reasons.push("opacity:0");
      if (rect.width <= 1 || rect.height <= 1) reasons.push("1x1 size");
      const vw = window.innerWidth, vh = window.innerHeight;
      if (rect.right < 0 || rect.bottom < 0 || rect.left > vw || rect.top > vh) {
        reasons.push("offscreen");
      }
      if (reasons.length) {
        const src = f.src || f.getAttribute("src") || "";
        hits.push({
          src,
          host: hostOf(src),
          reasons,
          width: Math.round(rect.width),
          height: Math.round(rect.height),
          outerHTMLPreview: (f.outerHTML || "").slice(0, 300)
        });
      }
    });
    return hits;
  }

  function matchesAllowlist(el, selectors) {
    if (!selectors || !selectors.length) return false;
    for (const sel of selectors) {
      if (!sel || typeof sel !== "string") continue;
      try {
        if (el.matches && el.matches(sel)) return true;
      } catch (e) { /* invalid selector in allowlist, skip */ }
    }
    return false;
  }

  function scanStickyOverlays() {
    const hits = [];
    const vw = window.innerWidth, vh = window.innerHeight;
    const viewportArea = Math.max(1, vw * vh);
    const all = document.querySelectorAll("body *");
    if (all.length > 20000) return hits; // bail on enormous DOMs
    let checked = 0;
    for (const el of all) {
      if (++checked > 5000) break; // cap
      let cs;
      try { cs = getComputedStyle(el); } catch (e) { continue; }
      if (cs.position !== "fixed" && cs.position !== "sticky") continue;
      const z = parseInt(cs.zIndex, 10);
      if (!Number.isFinite(z) || z < 100) continue;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;
      const area = rect.width * rect.height;
      const coverage = area / viewportArea;
      if (coverage < 0.25) continue;
      // User-configurable allowlist of structural containers
      // (React Portals, modal roots, framework shells) that shouldn't
      // be surfaced as sticky-overlay suggestions.
      if (matchesAllowlist(el, allowlist.overlays)) continue;
      const tag = el.tagName.toLowerCase();
      let cls = "";
      if (typeof el.className === "string") cls = el.className;
      else if (el.className && typeof el.className.baseVal === "string") cls = el.className.baseVal;
      const classes = cls.trim().split(/\s+/).filter(Boolean).slice(0, 2).join(".");
      const sel = tag + (classes ? "." + classes : "") + (el.id ? "#" + el.id : "");
      hits.push({
        selector: sel,
        coverage: Math.round(coverage * 100),
        zIndex: z,
        rect: { w: Math.round(rect.width), h: Math.round(rect.height) }
      });
    }
    return hits;
  }

  function runScan(reason) {
    const snapshot = {
      hostname: location.hostname,
      observedAt: new Date().toISOString(),
      reason: reason || "scheduled",
      resources: collectedResources.slice(),
      iframes: scanHiddenIframes(),
      stickies: scanStickyOverlays()
    };
    try {
      const p = chrome.runtime.sendMessage({ type: "hush:scan", ...snapshot });
      if (p && typeof p.catch === "function") p.catch(() => {});
    } catch (e) { /* extension context gone */ }
    log("scan:", reason, "resources", snapshot.resources.length, "iframes", snapshot.iframes.length, "stickies", snapshot.stickies.length);
  }

  // Install the resource observer unconditionally when detector is either
  // enabled OR the user might fire a one-shot scan. Cost is near-zero -
  // a single PerformanceObserver with buffered:true already running in Chrome.
  installResourceObserver();

  // Always handle scan-once messages from the popup.
  chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
    if (msg && msg.type === "hush:scan-once") {
      runScan("manual");
      sendResponse({ ok: true });
      return false;
    }
  });

  // --------------------------------------------------------------
  // Main-world hook bridge.
  //
  // mainworld.js (injected into the page's own JS context) monkey-patches
  // fetch/XHR/sendBeacon/WebSocket and dispatches CustomEvent("__hush_call__")
  // for every call with URL, method, body preview, and stack trace. Here
  // in the isolated world we buffer those events and forward them to
  // background in throttled batches.
  // --------------------------------------------------------------
  const jsCallBuffer = [];
  const MAX_LOCAL_JS_CALLS = 300;
  let jsCallSendTimer = null;

  document.addEventListener("__hush_call__", (ev) => {
    if (!detectorEnabled) return; // gate by user's suggestions toggle
    const d = ev && ev.detail;
    if (!d || typeof d !== "object") return;
    // Preserve kind-specific fields alongside the common ones so background
    // can process fingerprinting/replay observations with full context.
    const entry = {
      kind: String(d.kind || "?"),
      t: String(d.t || new Date().toISOString()),
      stack: Array.isArray(d.stack) ? d.stack.slice(0, 6) : []
    };
    // Common fetch/xhr/beacon/ws fields
    if ("url" in d) entry.url = String(d.url || "");
    if ("method" in d) entry.method = String(d.method || "");
    if ("bodyPreview" in d) entry.bodyPreview = d.bodyPreview == null ? null : String(d.bodyPreview);
    // Fingerprinting/replay-specific fields
    if ("param" in d) entry.param = String(d.param);
    if ("hotParam" in d) entry.hotParam = !!d.hotParam;
    if ("font" in d) entry.font = String(d.font || "");
    if ("text" in d) entry.text = String(d.text || "");
    if ("eventType" in d) entry.eventType = String(d.eventType || "");
    if (Array.isArray(d.vendors)) entry.vendors = d.vendors.slice(0, 20);
    // Tier 5 canvas-draw fields
    if ("op" in d) entry.op = String(d.op || "");
    if ("visible" in d) entry.visible = !!d.visible;
    if ("canvasSel" in d) entry.canvasSel = String(d.canvasSel || "");
    jsCallBuffer.push(entry);
    if (jsCallBuffer.length > MAX_LOCAL_JS_CALLS) {
      jsCallBuffer.splice(0, jsCallBuffer.length - MAX_LOCAL_JS_CALLS);
    }
    scheduleJsCallSend();
  });

  function scheduleJsCallSend() {
    if (jsCallSendTimer) return;
    jsCallSendTimer = setTimeout(() => {
      jsCallSendTimer = null;
      const batch = jsCallBuffer.splice(0, jsCallBuffer.length);
      if (!batch.length) return;
      try {
        const p = chrome.runtime.sendMessage({
          type: "hush:js-calls",
          calls: batch
        });
        if (p && typeof p.catch === "function") p.catch(() => {});
      } catch (e) { /* extension context gone */ }
    }, 500);
  }

  // Continuous scheduled scans only when the feature is enabled.
  if (detectorEnabled) {
    const fireInitial = () => runScan("dom-content-loaded");
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", fireInitial, { once: true });
    } else {
      fireInitial();
    }
    setTimeout(() => runScan("post-load-idle"), 5000);
  }

  // --------------------------------------------------------------
  // Config matching + hide/remove pass (unchanged from prior versions)
  // --------------------------------------------------------------

  if (!config || typeof config !== "object") {
    log(location.hostname, "- no config loaded");
    return;
  }

  // Firewall-style evaluation: global rules apply to every tab AND
  // site-scoped rules add on top. A site match is NOT required - if
  // only the reserved `__global__` entry exists, its rules still
  // fire. Duplicates (same selector / pattern / spoof tag) are
  // deduplicated so a value present in both scopes only fires once.
  //
  // Rule entries are `{value, disabled?, tags?, comment?}` objects
  // after the Stage 9 migration. Old configs stored bare strings;
  // the extension runs a one-shot migration on install/startup to
  // convert those. This reader is defensive for the window where
  // migration hasn't landed yet (first page load after upgrade):
  // string entries are treated as `{value: s}`.
  const GLOBAL_KEY = "__global__";
  function ruleValue(e) {
    if (typeof e === "string") return e;
    if (e && typeof e === "object") return String(e.value || "");
    return "";
  }
  function ruleDisabled(e) {
    return !!(e && typeof e === "object" && e.disabled);
  }
  function toValueList(arr) {
    if (!Array.isArray(arr)) return [];
    const out = [];
    for (const e of arr) {
      if (ruleDisabled(e)) continue;
      const v = ruleValue(e);
      if (v) out.push(v);
    }
    return out;
  }
  function mergeArrays(a, b) {
    const out = toValueList(a);
    for (const v of toValueList(b)) if (!out.includes(v)) out.push(v);
    return out;
  }
  const globalCfg = (config[GLOBAL_KEY] && typeof config[GLOBAL_KEY] === "object")
    ? config[GLOBAL_KEY]
    : null;
  const match = findConfigEntry(config, location.hostname);
  const siteCfg = match ? match.cfg : null;
  const matchedDomain = match ? match.key : (globalCfg ? GLOBAL_KEY : null);
  if (!globalCfg && !match) {
    log(location.hostname, "- no matching site config. configured sites:", Object.keys(config));
    return;
  }

  const cfg = {
    hide:   mergeArrays(globalCfg && globalCfg.hide,   siteCfg && siteCfg.hide),
    remove: mergeArrays(globalCfg && globalCfg.remove, siteCfg && siteCfg.remove),
    block:  mergeArrays(globalCfg && globalCfg.block,  siteCfg && siteCfg.block),
    spoof:  mergeArrays(globalCfg && globalCfg.spoof,  siteCfg && siteCfg.spoof)
  };
  // Build a selector -> originating-scope map so the firewall-log
  // event stream can attribute each hit back to the same row the
  // rule enumeration shows. Without this, the popup's FirewallLog
  // component shows the rule under its authoring scope (e.g.
  // `__global__`) but records events under the site scope, producing
  // confusing "double entry" rows (one rule with 0 hits + an orphan
  // event row for the site scope). Site-scoped rules win when a
  // selector appears in both layers (most-specific scope wins).
  const scopeOf = {};
  const siteScopeKey = match ? match.key : null;
  if (globalCfg) {
    for (const s of toValueList(globalCfg.hide))   scopeOf["hide::"   + s] = GLOBAL_KEY;
    for (const s of toValueList(globalCfg.remove)) scopeOf["remove::" + s] = GLOBAL_KEY;
  }
  if (siteScopeKey && siteCfg) {
    for (const s of toValueList(siteCfg.hide))   scopeOf["hide::"   + s] = siteScopeKey;
    for (const s of toValueList(siteCfg.remove)) scopeOf["remove::" + s] = siteScopeKey;
  }
  function scopeForSelector(action, selector) {
    return scopeOf[action + "::" + selector]
      || siteScopeKey
      || GLOBAL_KEY;
  }
  log(location.hostname, "- matched:", matchedDomain,
      "global:", globalCfg ? "yes" : "no",
      "site:", match ? match.key : "no");

  // Signal the main-world hook about any fingerprint signals the user
  // has opted to spoof. Comma-separated kind tags. The main-world
  // `getParameter` wrapper reads this dataset attribute at call time,
  // so the write ordering between content-script and main-world
  // install is irrelevant - by the time the page calls WebGL,
  // documentElement.dataset.hushSpoof is set.
  if (cfg.spoof.length) {
    try {
      document.documentElement.dataset.hushSpoof = cfg.spoof.join(",");
    } catch (e) { /* documentElement not ready yet */ }
  }

  const removeSelectors = cfg.remove.slice();
  const hideSelectors   = cfg.hide.slice();

  const stats = {
    matchedDomain,
    hide: {},
    remove: {}
  };
  for (const s of removeSelectors) stats.remove[s] = 0;
  for (const s of hideSelectors)   stats.hide[s]   = 0;

  const pendingRemovedEvents = [];

  function describeElement(el) {
    const tag = el.tagName ? el.tagName.toLowerCase() : "?";
    let clsString = "";
    if (typeof el.className === "string") {
      clsString = el.className;
    } else if (el.className && typeof el.className.baseVal === "string") {
      clsString = el.className.baseVal;
    }
    const classes = clsString.trim().split(/\s+/).filter(Boolean).slice(0, 2).join(".");
    const id = el.id ? "#" + el.id : "";
    const base = tag + (classes ? "." + classes : "") + id;

    // Distinguishing attributes worth surfacing so the user can tell
    // apart identically-tagged removals (e.g. Reddit's many RelatedCommunityRec
    // blocks, or individual feed posts, or <iframe> removals).
    const interestingAttrs = [
      "name", "data-testid", "data-post-id", "aria-label",
      "post-title", "post-type", "subreddit-prefixed-name",
      "author", "post-id", "data-promoted", "data-ad", "data-ad-type",
      "src", "href", "title", "alt"
    ];
    const attrParts = [];
    try {
      for (const attr of interestingAttrs) {
        if (!el.hasAttribute || !el.hasAttribute(attr)) continue;
        let v = el.getAttribute(attr);
        if (!v) continue;
        if (v.length > 70) v = v.slice(0, 67) + "...";
        attrParts.push(attr + '="' + v + '"');
        if (attrParts.length >= 3) break;
      }
    } catch (e) { /* ignore */ }

    let textSnippet = "";
    try {
      const txt = (el.textContent || "").replace(/\s+/g, " ").trim();
      if (txt) textSnippet = txt.length > 80 ? txt.slice(0, 77) + "..." : txt;
    } catch (e) { /* ignore */ }

    const parts = [base];
    if (attrParts.length) parts.push(attrParts.join(" "));
    if (textSnippet) parts.push('"' + textSnippet + '"');
    return parts.join("  |  ");
  }

  function applyRemove() {
    let anyChanged = false;
    for (const sel of removeSelectors) {
      try {
        const nodes = document.querySelectorAll(sel);
        if (nodes.length) {
          for (const el of nodes) {
            pendingRemovedEvents.push({
              t: new Date().toISOString(),
              selector: sel,
              el: describeElement(el),
              scope: scopeForSelector("remove", sel)
            });
            el.remove();
          }
          if (pendingRemovedEvents.length > MAX_LOCAL_REMOVED) {
            pendingRemovedEvents.splice(0, pendingRemovedEvents.length - MAX_LOCAL_REMOVED);
          }
          stats.remove[sel] = (stats.remove[sel] || 0) + nodes.length;
          anyChanged = true;
        }
      } catch (e) { /* invalid selector, skip */ }
    }
    return anyChanged;
  }

  function injectHideCSS() {
    if (!hideSelectors.length) return;
    const css = hideSelectors.map(s => s + " { display: none !important; }").join("\n");
    const style = document.createElement("style");
    style.textContent = css;
    style.setAttribute("data-hush", "hide");
    (document.head || document.documentElement).appendChild(style);
  }

  function recountHide() {
    let anyChanged = false;
    for (const sel of hideSelectors) {
      try {
        const n = document.querySelectorAll(sel).length;
        if (stats.hide[sel] !== n) {
          stats.hide[sel] = n;
          anyChanged = true;
        }
      } catch (e) { /* invalid selector */ }
    }
    return anyChanged;
  }

  function pass() {
    const a = applyRemove();
    const b = recountHide();
    if (a || b) scheduleSend();
  }

  pass();
  log("initial pass: remove", stats.remove, "hide", stats.hide);

  injectHideCSS();

  if (hideSelectors.length || removeSelectors.length) {
    new MutationObserver(pass).observe(document.documentElement, {
      childList: true,
      subtree: true
    });
    const logAfterLoad = () => {
      log("post-DOMContentLoaded pass: remove", stats.remove, "hide", stats.hide);
    };
    if (document.readyState === "loading") {
      document.addEventListener("DOMContentLoaded", logAfterLoad, { once: true });
    } else {
      logAfterLoad();
    }
  }

  let sendTimer = null;
  function scheduleSend() {
    if (sendTimer) return;
    sendTimer = setTimeout(() => {
      sendTimer = null;
      sendStats();
    }, 500);
  }

  function sendStats() {
    const toSend = pendingRemovedEvents.splice(0, pendingRemovedEvents.length);
    try {
      const p = chrome.runtime.sendMessage({
        type: "hush:stats",
        matchedDomain: stats.matchedDomain,
        hide: stats.hide,
        remove: stats.remove,
        newRemovedElements: toSend
      });
      if (p && typeof p.catch === "function") p.catch(() => {});
    } catch (e) {
      // extension reloaded or context invalidated, ignore
    }
  }

  sendStats();
})();
