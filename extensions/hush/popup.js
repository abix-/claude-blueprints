// Popup is an ES module (popup.html uses type="module"). Import the
// wasm-bindgen glue statically so instantiation starts at parse time.
// The returned `wasmReady` promise is awaited in main() before mounting
// the Leptos subtree. All per-section renderers live in Rust now
// (src/ui_popup.rs); this file is just wasm init + chrome API queries
// + footer button handlers.
import initWasm, { initEngine, mountPopup, refreshPopupSuggestions } from "./dist/pkg/hush.js";

const wasmReady = initWasm().then(() => {
  try { initEngine(); } catch (e) { console.error("[Hush popup] initEngine failed", e); }
}).catch(e => console.error("[Hush popup] wasm init failed", e));

const OPTIONS_KEY = "options";
const STORAGE_KEY = "config";

async function main() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  const tabId = tab && tab.id;
  const hostname = tab && tab.url ? safeHostname(tab.url) : "";

  // The entire popup body (matched-site header, activity, suggestions,
  // detector CTA, and the three diagnostic sections) is rendered by the
  // Leptos tree at #rust-popup-root. This file owns footer buttons,
  // the unmatched placeholder, and the chrome API queries that feed
  // mountPopup.
  const unmatchedEl = document.getElementById("unmatched");
  const createSiteBtn = document.getElementById("create-site");

  document.getElementById("options").addEventListener("click", () => {
    chrome.runtime.openOptionsPage();
  });
  document.getElementById("reload").addEventListener("click", () => {
    if (typeof tabId === "number") chrome.tabs.reload(tabId);
  });
  document.getElementById("debug").addEventListener("click", async () => {
    const btn = document.getElementById("debug");
    const origText = btn.textContent;
    try {
      const debugInfo = await chrome.runtime.sendMessage({
        type: "hush:get-debug-info",
        tabId: typeof tabId === "number" ? tabId : null
      });
      const payload = {
        ...debugInfo,
        url: tab && tab.url,
        hostname: hostname
      };
      await navigator.clipboard.writeText(JSON.stringify(payload, null, 2));
      btn.textContent = "Copied!";
    } catch (e) {
      btn.textContent = "Copy failed";
      console.error(e);
    }
    setTimeout(() => { btn.textContent = origText; }, 2000);
  });

  // Detector enable / scan-once / rescan buttons are rendered by the
  // Leptos DetectorCta component and call chrome.storage.local +
  // chrome.tabs.sendMessage via src/chrome_bridge.rs.

  createSiteBtn.addEventListener("click", async () => {
    if (!hostname) return;
    const data = await chrome.storage.local.get(STORAGE_KEY);
    const config = data[STORAGE_KEY] || {};
    if (!config[hostname]) {
      config[hostname] = { hide: [], remove: [], block: [] };
      await chrome.storage.local.set({ [STORAGE_KEY]: config });
    }
    // Reload the popup view
    main();
  });

  if (typeof tabId !== "number") {
    try { await wasmReady; mountPopup({ hostname: "", matchedDomain: null, blockCount: 0, removeCount: 0, hideCount: 0, suggestionCount: 0 }); } catch (e) {}
    return;
  }

  // Load stats, suggestions, rule diagnostics, options, AND the config itself.
  // We check config directly (not stats.matchedDomain) to avoid a race where
  // the popup opens before content.js has sent its first stats message.
  const [statsResp, suggResp, diagResp, storedData] = await Promise.all([
    chrome.runtime.sendMessage({ type: "hush:get-tab-stats", tabId }).catch(() => null),
    chrome.runtime.sendMessage({ type: "hush:get-suggestions", tabId }).catch(() => null),
    chrome.runtime.sendMessage({ type: "hush:get-rule-diagnostics", tabId, hostname }).catch(() => null),
    chrome.storage.local.get([OPTIONS_KEY, STORAGE_KEY])
  ]);

  const stats = (statsResp && statsResp.stats) || {};
  const suggestions = (suggResp && suggResp.suggestions) || [];
  const detectorEnabled = !!(storedData[OPTIONS_KEY] && storedData[OPTIONS_KEY].suggestionsEnabled);
  const config = storedData[STORAGE_KEY] || {};

  const configMatch = hostname ? findConfigEntry(config, hostname) : null;
  const matchedDomain = stats.matchedDomain || (configMatch && configMatch.key) || null;
  const isMatched = !!matchedDomain;

  if (!isMatched) {
    unmatchedEl.hidden = false;
    createSiteBtn.style.display = "inline-block";
  }

  // All diagnostic sections (Blocked, Removed, Hidden) render inside
  // the Leptos tree via mountPopup. No JS-side per-section DOM work left.

  // Hand everything the Leptos root needs in one snapshot.
  const sumCounts = (m) => Object.values(m || {}).reduce((a, b) => a + (Number(b) || 0), 0);
  try {
    await wasmReady;
    mountPopup({
      hostname: hostname || "",
      matchedDomain: matchedDomain || null,
      blockCount: stats.block || 0,
      removeCount: sumCounts(stats.remove),
      hideCount: sumCounts(stats.hide),
      suggestionCount: suggestions.length,
      tabId,
      suggestions,
      detectorEnabled,
      isMatched,
      blockedUrls: stats.blockedUrls || [],
      blockDiagnostics: (diagResp && diagResp.diagnostics) || [],
      removeSelectors: stats.remove || {},
      hideSelectors: stats.hide || {},
      removedElements: stats.removedElements || [],
    });
  } catch (e) { console.error("[Hush popup] mountPopup failed", e); }
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

function safeHostname(url) {
  try {
    return new URL(url).hostname;
  } catch (e) {
    return "";
  }
}

main();
