// Popup is an ES module (popup.html uses type="module"). Import the
// wasm-bindgen glue statically so instantiation starts at parse time.
// The returned `wasmReady` promise is awaited in main() before mounting
// the Leptos subtree. Over subsequent commits, the per-section JS
// renderers below get ported to Leptos components and their
// corresponding `#block-list` / `#sugg-list` roots disappear.
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

  // #match + suggestion list are now rendered by the Leptos tree at
  // #rust-popup-root. Remaining JS-owned elements are listed here.
  const sectionsEl = document.getElementById("sections");
  const unmatchedEl = document.getElementById("unmatched");
  const createSiteBtn = document.getElementById("create-site");
  const suggBlock = document.getElementById("suggestions-block");
  const suggDisabled = document.getElementById("sugg-disabled");
  const rescanRow = document.getElementById("rescan-row");

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

  document.getElementById("sugg-enable").addEventListener("click", async () => {
    const d = await chrome.storage.local.get(OPTIONS_KEY);
    const opts = d[OPTIONS_KEY] || {};
    opts.suggestionsEnabled = true;
    await chrome.storage.local.set({ [OPTIONS_KEY]: opts });
    // Trigger a scan now; continuous scheduled scans need a page reload.
    if (typeof tabId === "number") {
      try { await chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" }); } catch (e) {}
    }
    // Leptos owns the suggestion list; tell it to re-fetch. No full
    // popup reload needed.
    setTimeout(() => { try { refreshPopupSuggestions(); } catch (e) {} }, 400);
  });

  document.getElementById("sugg-scan-once").addEventListener("click", async () => {
    if (typeof tabId !== "number") return;
    try { await chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" }); } catch (e) {}
    // Leptos owns the suggestion list; tell it to re-fetch. No full
    // popup reload needed.
    setTimeout(() => { try { refreshPopupSuggestions(); } catch (e) {} }, 400);
  });

  document.getElementById("sugg-rescan").addEventListener("click", async () => {
    if (typeof tabId !== "number") return;
    try { await chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" }); } catch (e) {}
    // Leptos owns the suggestion list; tell it to re-fetch. No full
    // popup reload needed.
    setTimeout(() => { try { refreshPopupSuggestions(); } catch (e) {} }, 400);
  });

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

    // Render in aggressiveness order: block > remove > hide.
    renderBlockedList(stats.blockedUrls || [], stats.block || 0);
    renderBlockDiagnostics((diagResp && diagResp.diagnostics) || []);
    renderSelectorList("remove", stats.remove || {}, null);
    renderRemovedEvidence(stats.removedElements || []);
    const removeKeys = new Set(Object.keys(stats.remove || {}));
    renderSelectorList("hide", stats.hide || {}, removeKeys);
  }

  // Suggestions list is rendered by the Leptos component tree (see
  // src/ui_popup.rs). The per-tab detector-enable / scan-once / rescan
  // buttons remain JS-driven for now (handlers wired earlier in main).
  suggBlock.hidden = false;
  suggDisabled.hidden = detectorEnabled;
  rescanRow.hidden = !detectorEnabled;

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

function renderBlockedList(blockedUrls, blockCount) {
  const countEl = document.getElementById("block-count");
  const listEl = document.getElementById("block-list");
  const evidenceEl = document.getElementById("block-evidence");

  countEl.textContent = String(blockCount);
  countEl.classList.toggle("zero", blockCount === 0);
  listEl.innerHTML = "";

  const byPattern = {};
  for (const b of blockedUrls) {
    const key = b.pattern || "(unknown rule)";
    byPattern[key] = (byPattern[key] || 0) + 1;
  }
  const patterns = Object.keys(byPattern);
  if (!patterns.length) {
    const p = document.createElement("div");
    p.className = "no-sels";
    p.textContent = blockCount > 0
      ? "Blocked, but URL evidence not yet captured (try reloading)"
      : "No network blocks yet";
    listEl.appendChild(p);
  } else {
    for (const pattern of patterns) {
      const li = document.createElement("li");
      const sel = document.createElement("span");
      sel.className = "sel";
      sel.title = pattern;
      sel.textContent = pattern;
      const n = document.createElement("span");
      n.className = "n";
      n.textContent = String(byPattern[pattern]);
      li.appendChild(sel);
      li.appendChild(n);
      listEl.appendChild(li);
    }
  }

  if (!blockedUrls.length) {
    evidenceEl.hidden = true;
    return;
  }
  evidenceEl.hidden = false;
  evidenceEl.innerHTML = "";
  const header = document.createElement("div");
  header.style.cssText = "display:flex;align-items:center;gap:8px;";
  const toggle = document.createElement("span");
  toggle.className = "evidence-toggle";
  toggle.textContent = "Show " + blockedUrls.length + " blocked URL" +
    (blockedUrls.length === 1 ? "" : "s");
  header.appendChild(toggle);
  const copyBtn = makeCopyButton(() =>
    blockedUrls
      .slice()
      .reverse()
      .map(b => timeOnly(b.t) + "\t[" + (b.resourceType || "?") + "]\t" + b.url + "\t(pattern: " + (b.pattern || "?") + ")")
      .join("\n")
  );
  header.appendChild(copyBtn);
  evidenceEl.appendChild(header);
  const list = document.createElement("ul");
  list.className = "evidence-list";
  list.hidden = true;
  const items = blockedUrls.slice().reverse();
  for (const b of items) {
    const li = document.createElement("li");
    const ts = document.createElement("span");
    ts.className = "ts";
    ts.textContent = timeOnly(b.t);
    const body = document.createElement("span");
    const resType = b.resourceType ? " [" + b.resourceType + "]" : "";
    body.title = b.url;
    body.textContent = b.url + resType;
    li.appendChild(ts);
    li.appendChild(body);
    list.appendChild(li);
  }
  evidenceEl.appendChild(list);
  toggle.addEventListener("click", () => {
    list.hidden = !list.hidden;
    toggle.textContent = (list.hidden ? "Show " : "Hide ") +
      blockedUrls.length + " blocked URL" +
      (blockedUrls.length === 1 ? "" : "s");
  });
}

// Per-rule diagnostic panel inside the Blocked section. Shows each configured
// block rule with its fire count and status, plus a hint if the pattern looks
// broken (observed traffic that matches the pattern's keyword but no fires).
function renderBlockDiagnostics(diagnostics) {
  const container = document.getElementById("block-diagnostics");
  if (!container) return;
  container.innerHTML = "";
  if (!diagnostics.length) {
    container.hidden = true;
    return;
  }
  container.hidden = false;

  const title = document.createElement("div");
  title.className = "diagnostics-title";
  title.textContent = "Block rules (" + diagnostics.length + ")";
  container.appendChild(title);

  for (const d of diagnostics) {
    const row = document.createElement("div");
    row.className = "rule-row";

    const patternEl = document.createElement("div");
    patternEl.className = "rule-pattern";
    patternEl.title = d.pattern;
    patternEl.textContent = d.pattern;
    row.appendChild(patternEl);

    const meta = document.createElement("div");
    meta.className = "rule-meta";
    const fired = document.createElement("span");
    fired.className = "rule-fired";
    fired.textContent = "fired " + d.fired + "x  |  declared under " + (d.sourceDomain || "-");
    meta.appendChild(fired);
    const statusLabel = {
      "firing": "FIRING",
      "no-traffic": "no traffic",
      "pattern-broken": "PATTERN BROKEN?"
    }[d.status] || d.status;
    const status = document.createElement("span");
    status.className = "rule-status " + d.status;
    status.textContent = statusLabel;
    meta.appendChild(status);
    row.appendChild(meta);

    if (d.status === "pattern-broken" && d.matchingUrls && d.matchingUrls.length) {
      const hint = document.createElement("div");
      hint.className = "rule-hint";
      hint.innerHTML = "<b>Diagnosis:</b> this page requested URLs containing " +
        "<code>" + escapeHtml(d.keyword) + "</code> but the rule never fired. " +
        "Your pattern probably doesn't match. Try a simpler form - e.g., drop wildcards, " +
        "or use the distinctive substring anchored with <code>||domain</code>.";
      const urls = document.createElement("div");
      urls.className = "urls";
      urls.innerHTML = "<div style=\"margin-top:6px;color:#999\">URLs that should have matched:</div>";
      for (const u of d.matchingUrls) {
        const line = document.createElement("div");
        line.title = u;
        line.textContent = u;
        urls.appendChild(line);
      }
      hint.appendChild(urls);
      row.appendChild(hint);
    } else if (d.status === "no-traffic" && d.fired === 0) {
      const hint = document.createElement("div");
      hint.className = "rule-hint";
      hint.style.background = "#f0f0f0";
      hint.style.color = "#666";
      hint.innerHTML = "<b>No matching traffic yet.</b> Either the site hasn't requested " +
        "this URL in the current session, or a DOM Remove rule is killing the element " +
        "before it can fetch. Not necessarily a bug - scroll/reload the page to generate " +
        "more traffic if you want to verify.";
      row.appendChild(hint);
    }

    container.appendChild(row);
  }
}

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

function escapeHtml(s) {
  return String(s).replace(/[&<>"']/g, c => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "\"": "&quot;",
    "'": "&#39;"
  }[c]));
}

main();
