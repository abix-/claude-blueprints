// Popup is an ES module (popup.html uses type="module"). Import the
// wasm-bindgen glue statically so instantiation starts at parse time.
// The returned `wasmReady` promise is awaited in main() before mounting
// the Leptos subtree. Remaining JS renderers (remove + hide sections,
// removed-element evidence) get ported to Leptos in the next stage 4
// iteration; their #remove-* / #hide-* DOM anchors live in popup.html.
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

  // #match + suggestion list + detector CTA are now rendered by the
  // Leptos tree at #rust-popup-root. Remaining JS-owned elements are
  // listed here.
  const sectionsEl = document.getElementById("sections");
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
  } else {
    sectionsEl.hidden = false;

    // Block section (list + evidence + diagnostics) is rendered by the
    // Leptos tree at #rust-popup-root via the snapshot below. Remove +
    // Hide sections still live in JS and get ported in the next stage
    // 4 iteration.
    renderSelectorList("remove", stats.remove || {}, null);
    renderRemovedEvidence(stats.removedElements || []);
    const removeKeys = new Set(Object.keys(stats.remove || {}));
    renderSelectorList("hide", stats.hide || {}, removeKeys);
  }

  // Suggestions list + detector CTA are rendered by the Leptos tree.

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
    });
  } catch (e) { console.error("[Hush popup] mountPopup failed", e); }
}

// --- Deleted in Stage 4 iter 3 (moved to src/ui_popup.rs) ---
// renderSuggestions, refreshSuggestions, renderSuggList, renderSuggRow
// now live in Rust Leptos components.

function renderSelectorList(kind, entries, overlapSet) {
  const countEl = document.getElementById(kind + "-count");
  const listEl = document.getElementById(kind + "-list");
  const keys = Object.keys(entries || {});
  const total = keys.reduce((a, k) => a + (entries[k] || 0), 0);
  countEl.textContent = String(total);
  countEl.classList.toggle("zero", total === 0);
  listEl.innerHTML = "";
  if (!keys.length) {
    const p = document.createElement("div");
    p.className = "no-sels";
    p.textContent = "No " + kind + " selectors configured";
    listEl.appendChild(p);
    return;
  }
  for (const key of keys) {
    const li = document.createElement("li");
    const sel = document.createElement("span");
    sel.className = "sel";
    sel.title = key;
    sel.textContent = key;
    const n = document.createElement("span");
    n.className = "n";
    const count = entries[key] || 0;
    if (count === 0 && overlapSet && overlapSet.has(key)) {
      n.textContent = "- (removed)";
      n.style.fontStyle = "italic";
      n.style.color = "#999";
    } else {
      n.textContent = String(count);
    }
    li.appendChild(sel);
    li.appendChild(n);
    listEl.appendChild(li);
  }
}

function renderRemovedEvidence(removedElements) {
  const container = document.getElementById("remove-evidence");
  if (!removedElements.length) {
    container.hidden = true;
    return;
  }
  container.hidden = false;
  container.innerHTML = "";
  const header = document.createElement("div");
  header.style.cssText = "display:flex;align-items:center;gap:8px;";
  const toggle = document.createElement("span");
  toggle.className = "evidence-toggle";
  toggle.textContent = "Show " + removedElements.length + " removed element" +
    (removedElements.length === 1 ? "" : "s");
  header.appendChild(toggle);
  const copyBtn = makeCopyButton(() =>
    removedElements
      .slice()
      .reverse()
      .map(ev => timeOnly(ev.t) + "\t" + (ev.el || "?") + "\t(via " + (ev.selector || "?") + ")")
      .join("\n")
  );
  header.appendChild(copyBtn);
  container.appendChild(header);
  const list = document.createElement("ul");
  list.className = "evidence-list";
  list.hidden = true;
  const items = removedElements.slice().reverse();
  for (const ev of items) {
    const li = document.createElement("li");
    const ts = document.createElement("span");
    ts.className = "ts";
    ts.textContent = timeOnly(ev.t);
    const body = document.createElement("span");
    body.title = (ev.selector || "") + " -> " + (ev.el || "");
    body.textContent = (ev.el || "?") + "  (via " + (ev.selector || "?") + ")";
    li.appendChild(ts);
    li.appendChild(body);
    list.appendChild(li);
  }
  container.appendChild(list);
  toggle.addEventListener("click", () => {
    list.hidden = !list.hidden;
    toggle.textContent = (list.hidden ? "Show " : "Hide ") +
      removedElements.length + " removed element" +
      (removedElements.length === 1 ? "" : "s");
  });
}

// Small "Copy" button that copies the result of getText() to the clipboard
// and briefly shows "Copied" feedback. Used across evidence sections.
function makeCopyButton(getText) {
  const btn = document.createElement("button");
  btn.textContent = "Copy";
  btn.title = "Copy evidence to clipboard";
  btn.style.cssText = "flex:0 0 auto;padding:2px 10px;font-size:10px;cursor:pointer;border:1px solid #ccc;background:#fff;border-radius:4px;";
  btn.addEventListener("click", async (e) => {
    e.stopPropagation();
    const orig = btn.textContent;
    try {
      await navigator.clipboard.writeText(getText());
      btn.textContent = "Copied";
    } catch (err) {
      btn.textContent = "Failed";
    }
    setTimeout(() => { btn.textContent = orig; }, 1500);
  });
  return btn;
}

// renderBlockedList + renderBlockDiagnostics were ported to the Leptos
// BlockedSection component in src/ui_popup.rs (stage 4 iter 6). See
// popup.html for the removed #block-* DOM anchors.

function timeOnly(iso) {
  try {
    const d = new Date(iso);
    return d.toTimeString().slice(0, 8);
  } catch (e) {
    return "";
  }
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
