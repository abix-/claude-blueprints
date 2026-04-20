// Popup is an ES module (see popup.html) so we can dynamically load the
// Rust/Leptos runtime. Stage 4 scaffold: mount the Leptos component
// tree into #rust-popup-root at the top of the popup. Over subsequent
// commits, the per-section JS renderers below get ported to Leptos
// components and their corresponding `#block-list` / `#sugg-list` roots
// disappear. The hybrid coexists until the port is complete.
(async () => {
  try {
    const url = chrome.runtime.getURL("dist/pkg/hush.js");
    const mod = await import(url);
    await mod.default();
    if (typeof mod.initEngine === "function") mod.initEngine();
    if (typeof mod.mountPopup === "function") mod.mountPopup();
  } catch (e) {
    console.error("[Hush popup] leptos mount failed", e);
  }
})();

const OPTIONS_KEY = "options";
const STORAGE_KEY = "config";

async function main() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  const tabId = tab && tab.id;
  const hostname = tab && tab.url ? safeHostname(tab.url) : "";

  const matchEl = document.getElementById("match");
  const sectionsEl = document.getElementById("sections");
  const unmatchedEl = document.getElementById("unmatched");
  const createSiteBtn = document.getElementById("create-site");
  const suggBlock = document.getElementById("suggestions-block");
  const suggDisabled = document.getElementById("sugg-disabled");
  const suggList = document.getElementById("sugg-list");
  const suggCount = document.getElementById("sugg-count");
  const suggSub = document.getElementById("sugg-sub");
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
    setTimeout(() => refreshSuggestions(tabId, hostname), 400);
  });

  document.getElementById("sugg-scan-once").addEventListener("click", async () => {
    if (typeof tabId !== "number") return;
    try { await chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" }); } catch (e) {}
    setTimeout(() => refreshSuggestions(tabId, hostname), 400);
  });

  document.getElementById("sugg-rescan").addEventListener("click", async () => {
    if (typeof tabId !== "number") return;
    try { await chrome.tabs.sendMessage(tabId, { type: "hush:scan-once" }); } catch (e) {}
    setTimeout(() => refreshSuggestions(tabId, hostname), 400);
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
    matchEl.textContent = "No active tab";
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
    matchEl.textContent = hostname || "-";
    unmatchedEl.hidden = false;
    createSiteBtn.style.display = "inline-block";
  } else {
    matchEl.innerHTML = "Matched: <b>" + escapeHtml(matchedDomain) + "</b>" +
      (hostname && hostname !== matchedDomain
        ? " <span style=\"color:#999\">(" + escapeHtml(hostname) + ")</span>"
        : "");
    sectionsEl.hidden = false;

    // Render in aggressiveness order: block > remove > hide.
    renderBlockedList(stats.blockedUrls || [], stats.block || 0);
    renderBlockDiagnostics((diagResp && diagResp.diagnostics) || []);
    renderSelectorList("remove", stats.remove || {}, null);
    renderRemovedEvidence(stats.removedElements || []);
    const removeKeys = new Set(Object.keys(stats.remove || {}));
    renderSelectorList("hide", stats.hide || {}, removeKeys);
  }

  // Suggestions block is always visible; content varies by enabled state.
  suggBlock.hidden = false;
  renderSuggestions(tabId, hostname, suggestions, detectorEnabled, isMatched);
}

async function refreshSuggestions(tabId, hostname) {
  const resp = await chrome.runtime.sendMessage({ type: "hush:get-suggestions", tabId }).catch(() => null);
  const optsData = await chrome.storage.local.get(OPTIONS_KEY);
  const enabled = !!(optsData[OPTIONS_KEY] && optsData[OPTIONS_KEY].suggestionsEnabled);
  const statsResp = await chrome.runtime.sendMessage({ type: "hush:get-tab-stats", tabId }).catch(() => null);
  const isMatched = !!(statsResp && statsResp.stats && statsResp.stats.matchedDomain);
  renderSuggestions(tabId, hostname, (resp && resp.suggestions) || [], enabled, isMatched);
}

function renderSuggestions(tabId, hostname, suggestions, enabled, isMatched) {
  const suggDisabled = document.getElementById("sugg-disabled");
  const suggList = document.getElementById("sugg-list");
  const suggCount = document.getElementById("sugg-count");
  const suggSub = document.getElementById("sugg-sub");
  const rescanRow = document.getElementById("rescan-row");

  suggCount.textContent = String(suggestions.length);
  suggCount.classList.toggle("zero", suggestions.length === 0);

  if (!enabled) {
    suggDisabled.hidden = false;
    rescanRow.hidden = true;
    suggSub.textContent = "";
    // still show the list if a one-shot scan produced results
    renderSuggList(tabId, hostname, suggestions, isMatched);
    return;
  }

  suggDisabled.hidden = true;
  rescanRow.hidden = false;
  suggSub.textContent = suggestions.length
    ? "click + Add to append to config"
    : "no suspicious behavior observed yet";
  renderSuggList(tabId, hostname, suggestions, isMatched);
}

function renderSuggList(tabId, hostname, suggestions, isMatched) {
  const suggList = document.getElementById("sugg-list");
  suggList.innerHTML = "";
  if (!suggestions.length) return;
  for (const s of suggestions) {
    suggList.appendChild(renderSuggRow(tabId, hostname, s, isMatched));
  }
}

function renderSuggRow(tabId, hostname, s, isMatched) {
  const li = document.createElement("li");

  const top = document.createElement("div");
  top.className = "sugg-row-top";
  const layer = document.createElement("span");
  layer.className = "sugg-layer " + s.layer;
  layer.textContent = s.layer;
  top.appendChild(layer);
  if (s.fromIframe && s.frameHostname) {
    const iframeChip = document.createElement("span");
    iframeChip.className = "sugg-iframe";
    iframeChip.textContent = "from iframe " + s.frameHostname;
    iframeChip.title = "This request came from an embedded " + s.frameHostname +
      " iframe on the current tab. Hush checks your tab's site config for dedup, " +
      "so your existing rules still cover it.";
    top.appendChild(iframeChip);
  }
  const conf = document.createElement("span");
  conf.className = "sugg-conf";
  conf.textContent = "conf " + (s.confidence || 0) + "  |  count " + (s.count || 1);
  top.appendChild(conf);
  li.appendChild(top);

  const value = document.createElement("div");
  value.className = "sugg-value";
  value.title = s.value;
  value.textContent = s.value;
  li.appendChild(value);

  const reason = document.createElement("div");
  reason.className = "sugg-reason";
  reason.textContent = s.reason;
  li.appendChild(reason);

  // Always-visible teaching text explaining what the signal is and why
  // it's worth blocking. Technical but short. Only rendered if the
  // background attached a `learn` string to the suggestion.
  if (s.learn) {
    const learn = document.createElement("div");
    learn.className = "sugg-learn";
    learn.textContent = s.learn;
    li.appendChild(learn);
  }

  const actions = document.createElement("div");
  actions.className = "sugg-actions";
  const addBtn = document.createElement("button");
  addBtn.className = "add";
  addBtn.textContent = "+ Add";
  addBtn.addEventListener("click", async () => {
    addBtn.disabled = true;
    addBtn.textContent = "Adding...";
    const resp = await chrome.runtime.sendMessage({
      type: "hush:accept-suggestion",
      hostname,
      layer: s.layer,
      value: s.value
    }).catch(() => null);
    if (resp && resp.ok) {
      addBtn.textContent = "Added";
      setTimeout(() => {
        li.remove();
        refreshSuggestions(tabId, hostname);
      }, 400);
    } else {
      addBtn.disabled = false;
      addBtn.textContent = "+ Add";
    }
  });
  actions.appendChild(addBtn);

  const dismissBtn = document.createElement("button");
  dismissBtn.className = "dismiss";
  dismissBtn.textContent = "Dismiss";
  dismissBtn.title = "Hide this suggestion for the current tab session only. A reload will bring it back.";
  dismissBtn.addEventListener("click", async () => {
    await chrome.runtime.sendMessage({
      type: "hush:dismiss-suggestion",
      tabId,
      key: s.key
    }).catch(() => null);
    li.remove();
    refreshSuggestions(tabId, hostname);
  });
  actions.appendChild(dismissBtn);

  // "Allow" persists the suggestion key in the allowlist so this exact
  // suggestion never surfaces again on any site. Use for legit things Hush
  // misidentifies (new captcha provider, a real hidden widget you use, etc).
  // Revocable from the options page's allowlist editor.
  const allowBtn = document.createElement("button");
  allowBtn.className = "allow";
  allowBtn.textContent = "Allow";
  allowBtn.title = "Permanently allow this detection on all sites. Manage the full allowlist in Options.";
  allowBtn.addEventListener("click", async () => {
    allowBtn.disabled = true;
    allowBtn.textContent = "Allowing...";
    const resp = await chrome.runtime.sendMessage({
      type: "hush:allowlist-add-suggestion",
      key: s.key
    }).catch(() => null);
    if (resp && resp.ok) {
      allowBtn.textContent = "Allowed";
      setTimeout(() => {
        li.remove();
        refreshSuggestions(tabId, hostname);
      }, 400);
    } else {
      allowBtn.disabled = false;
      allowBtn.textContent = "Allow";
    }
  });
  actions.appendChild(allowBtn);

  // "Why here?" - inline diagnostic explaining the dedup decision,
  // so the user can see WHY a suggestion appears even when they think
  // they have a rule for it.
  if (s.diag) {
    const whyBtn = document.createElement("button");
    whyBtn.textContent = "Why?";
    whyBtn.style.flex = "0 0 auto";
    whyBtn.title = "Show dedup diagnostic";
    const whyPanel = document.createElement("div");
    whyPanel.className = "sugg-evidence";
    whyPanel.hidden = true;
    whyBtn.addEventListener("click", () => {
      whyPanel.hidden = !whyPanel.hidden;
      whyBtn.textContent = whyPanel.hidden ? "Why?" : "Hide why";
      if (!whyPanel.hidden) {
        whyPanel.innerHTML = "";
        const info = s.diag;
        const list = document.createElement("ul");
        list.className = "sugg-evidence-list";
        const rows = [
          ["Checked value", info.value],
          ["Tab hostname (used for config match)", info.tabHostname || "(unknown)"],
          ["Observed from frame", info.frameHostname || info.tabHostname || "(unknown)"],
          ["From iframe?", info.isFromIframe ? "yes" : "no"],
          ["Matched config key", info.matchedKey || "(no site config matched)"],
          ["Existing " + info.layer + " rules count", String(info.existingBlockCount)],
          ["Dedup result", info.dedupResult]
        ];
        for (const [k, v] of rows) {
          const li = document.createElement("li");
          li.innerHTML = "<b>" + escapeHtml(k) + ":</b> " + escapeHtml(String(v));
          list.appendChild(li);
        }
        if (Array.isArray(info.existingBlockSample) && info.existingBlockSample.length) {
          const li = document.createElement("li");
          li.innerHTML = "<b>Existing rules sample (first 10):</b>";
          list.appendChild(li);
          for (const entry of info.existingBlockSample) {
            const liE = document.createElement("li");
            liE.style.paddingLeft = "12px";
            liE.title = entry;
            liE.textContent = entry + " (len=" + entry.length + ")";
            list.appendChild(liE);
          }
          const li2 = document.createElement("li");
          li2.innerHTML = "<b>Candidate value:</b> " + escapeHtml(info.value) + " (len=" + info.value.length + ")";
          list.appendChild(li2);
        }
        whyPanel.appendChild(list);
      }
    });
    actions.appendChild(whyBtn);
    li.appendChild(whyPanel);
  }

  const evBtn = document.createElement("button");
  evBtn.textContent = "Evidence";
  evBtn.style.flex = "0 0 auto";
  const ev = document.createElement("div");
  ev.className = "sugg-evidence";
  ev.hidden = true;
  const evSrc = Array.isArray(s.evidence) ? s.evidence : [];
  if (evSrc.length) {
    const evHeader = document.createElement("div");
    evHeader.style.cssText = "display:flex;justify-content:flex-end;margin:4px 0;";
    const copyBtn = makeCopyButton(() => evSrc.map(e => String(e)).join("\n"));
    evHeader.appendChild(copyBtn);
    ev.appendChild(evHeader);
  }
  const evList = document.createElement("ul");
  evList.className = "sugg-evidence-list";
  if (!evSrc.length) {
    const liE = document.createElement("li");
    liE.style.fontStyle = "italic";
    liE.textContent = "(no captured evidence)";
    evList.appendChild(liE);
  } else {
    for (const e of evSrc) {
      const liE = document.createElement("li");
      liE.title = String(e);
      liE.textContent = String(e);
      evList.appendChild(liE);
    }
  }
  ev.appendChild(evList);
  evBtn.addEventListener("click", () => {
    ev.hidden = !ev.hidden;
    evBtn.textContent = ev.hidden ? "Evidence" : "Hide evidence";
  });
  actions.appendChild(evBtn);
  li.appendChild(actions);
  li.appendChild(ev);

  return li;
}

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
