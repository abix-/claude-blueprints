async function main() {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  const tabId = tab && tab.id;
  const hostname = tab && tab.url ? safeHostname(tab.url) : "";

  const matchEl = document.getElementById("match");
  const sectionsEl = document.getElementById("sections");
  const unmatchedEl = document.getElementById("unmatched");

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

  if (typeof tabId !== "number") {
    matchEl.textContent = "No active tab";
    return;
  }

  let stats = null;
  try {
    const resp = await chrome.runtime.sendMessage({
      type: "hush:get-tab-stats",
      tabId
    });
    stats = resp && resp.stats;
  } catch (e) {
    stats = null;
  }

  if (!stats || !stats.matchedDomain) {
    matchEl.textContent = hostname || "-";
    unmatchedEl.hidden = false;
    return;
  }

  matchEl.innerHTML = "Matched: <b>" + escapeHtml(stats.matchedDomain) + "</b>" +
    (hostname && hostname !== stats.matchedDomain
      ? " <span style=\"color:#999\">(" + escapeHtml(hostname) + ")</span>"
      : "");
  sectionsEl.hidden = false;

  // Render in aggressiveness order: block > remove > hide.
  renderBlockedList(stats.blockedUrls || [], stats.block || 0);
  renderSelectorList("remove", stats.remove, null);
  renderRemovedEvidence(stats.removedElements || []);
  const removeKeys = new Set(Object.keys(stats.remove || {}));
  renderSelectorList("hide", stats.hide, removeKeys);
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

  const toggle = document.createElement("span");
  toggle.className = "evidence-toggle";
  toggle.textContent = "Show " + removedElements.length + " removed element" +
    (removedElements.length === 1 ? "" : "s");
  container.appendChild(toggle);

  const list = document.createElement("ul");
  list.className = "evidence-list";
  list.hidden = true;
  // Show newest first.
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

function renderBlockedList(blockedUrls, blockCount) {
  const countEl = document.getElementById("block-count");
  const listEl = document.getElementById("block-list");
  const evidenceEl = document.getElementById("block-evidence");

  countEl.textContent = String(blockCount);
  countEl.classList.toggle("zero", blockCount === 0);
  listEl.innerHTML = "";

  // Group by pattern for the per-rule breakdown.
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

  const toggle = document.createElement("span");
  toggle.className = "evidence-toggle";
  toggle.textContent = "Show " + blockedUrls.length + " blocked URL" +
    (blockedUrls.length === 1 ? "" : "s");
  evidenceEl.appendChild(toggle);

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

function timeOnly(iso) {
  try {
    const d = new Date(iso);
    return d.toTimeString().slice(0, 8);
  } catch (e) {
    return "";
  }
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
