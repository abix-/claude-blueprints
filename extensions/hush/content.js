(async function main() {
  // ================================================================
  // Hush content script — runs at document_start on every page.
  //
  // Layer ordering (most aggressive to mildest):
  //   1. Block  — enforced in background.js via declarativeNetRequest
  //               (network layer; no content script involvement needed)
  //   2. Remove — physically deletes matching elements from the DOM
  //   3. Hide   — passive CSS stylesheet suppressing render of remaining matches
  // ================================================================

  const STORAGE_KEY = "config";
  const OPTIONS_KEY = "options";
  const MAX_LOCAL_REMOVED = 200;

  function findConfigEntry(config, host) {
    if (config[host]) return { key: host, cfg: config[host] };
    for (const key of Object.keys(config)) {
      if (host === key || host.endsWith("." + key)) {
        return { key, cfg: config[key] };
      }
    }
    return null;
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

  // ================================================================
  // Layer 2: Remove — physical DOM deletion (active on every pass).
  // Runs before hide-counting so overlapping selectors show the honest
  // "matched by remove, gone before hide could count it" story.
  // ================================================================
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

  // ================================================================
  // Layer 3: Hide — passive CSS stylesheet + recount of currently
  // matching elements (what Remove didn't capture).
  // ================================================================
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

  // Unified pass: remove first (aggressive), then count what the CSS is still hiding.
  function pass() {
    const a = applyRemove();
    const b = recountHide();
    if (a || b) scheduleSend();
  }

  // Initial pass (largely a no-op at document_start since the body hasn't parsed yet).
  pass();
  log("initial pass: remove", stats.remove, "hide", stats.hide);

  // Install the hide stylesheet so anything parsed-in later gets suppressed
  // instantly, before the MutationObserver has a chance to run.
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

  // ================================================================
  // Throttled stats reporting to the background service worker.
  // ================================================================
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
