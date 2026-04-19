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
  try {
    const data = await chrome.storage.local.get([STORAGE_KEY, OPTIONS_KEY]);
    config = data[STORAGE_KEY];
    options = data[OPTIONS_KEY] || {};
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

  const match = findConfigEntry(config, location.hostname);
  if (!match) {
    log(location.hostname, "- no matching site config. configured sites:", Object.keys(config));
    return;
  }
  const cfg = match.cfg;
  const matchedDomain = match.key;
  log(location.hostname, "- matched:", matchedDomain);

  const removeSelectors = Array.isArray(cfg.remove) ? cfg.remove.slice() : [];
  const hideSelectors   = Array.isArray(cfg.hide)   ? cfg.hide.slice()   : [];

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
    const classes = clsString.trim().split(/\s+/).filter(Boolean).slice(0, 3).join(".");
    const id = el.id ? "#" + el.id : "";
    return tag + (classes ? "." + classes : "") + id;
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
              el: describeElement(el)
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
