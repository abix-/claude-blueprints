const STORAGE_KEY = "config";
const OPTIONS_KEY = "options";
const ALLOWLIST_KEY = "allowlist";

// Defaults live in allowlist.defaults.json (same file background.js uses to
// seed storage). Loaded once at page init and kept in a module-scoped var so
// the "Reset to defaults" button doesn't need to re-fetch.
let DEFAULT_ALLOWLIST = { iframes: [], overlays: [] };
async function loadDefaultAllowlist() {
  const url = chrome.runtime.getURL("allowlist.defaults.json");
  const res = await fetch(url);
  return res.json();
}

const siteListEl = document.getElementById("site-list");
const detailEl = document.getElementById("detail");
const addSiteBtn = document.getElementById("add-site");
const debugToggleEl = document.getElementById("debug-toggle");
const suggestionsToggleEl = document.getElementById("suggestions-toggle");
const allowlistIframesEl = document.getElementById("allowlist-iframes");
const allowlistOverlaysEl = document.getElementById("allowlist-overlays");
const allowlistSuggestionsEl = document.getElementById("allowlist-suggestions");
const allowlistSaveBtn = document.getElementById("allowlist-save");
const allowlistResetBtn = document.getElementById("allowlist-reset");
const exportBtn = document.getElementById("export");
const resetBtn = document.getElementById("reset");
const statusEl = document.getElementById("status");
const jsonEl = document.getElementById("json-config");
const jsonApplyBtn = document.getElementById("json-apply");
const jsonRefreshBtn = document.getElementById("json-refresh");

let config = {};
let selectedDomain = null;

function setStatus(msg, ok) {
  statusEl.textContent = msg;
  statusEl.className = "status " + (ok ? "ok" : "err");
  statusEl.style.display = "inline-block";
  clearTimeout(setStatus._t);
  setStatus._t = setTimeout(() => { statusEl.style.display = "none"; }, 3500);
}

async function loadAll() {
  try {
    DEFAULT_ALLOWLIST = await loadDefaultAllowlist();
  } catch (e) {
    /* fall back to the empty default already in the module-scoped var */
  }
  const data = await chrome.storage.local.get([STORAGE_KEY, OPTIONS_KEY, ALLOWLIST_KEY]);
  config = data[STORAGE_KEY] || {};
  const opts = data[OPTIONS_KEY] || {};
  debugToggleEl.checked = !!opts.debug;
  suggestionsToggleEl.checked = !!opts.suggestionsEnabled;
  const al = data[ALLOWLIST_KEY] || DEFAULT_ALLOWLIST;
  allowlistIframesEl.value = (al.iframes || []).join("\n");
  allowlistOverlaysEl.value = (al.overlays || []).join("\n");
  allowlistSuggestionsEl.value = (al.suggestions || []).join("\n");
  render();
}

function linesToList(text) {
  return String(text || "")
    .split(/\r?\n/)
    .map(s => s.trim())
    .filter(Boolean);
}

async function save() {
  await chrome.storage.local.set({ [STORAGE_KEY]: config });
  jsonEl.value = JSON.stringify(config, null, 2);
}

function render() {
  renderSiteList();
  renderDetail();
  jsonEl.value = JSON.stringify(config, null, 2);
}

function renderSiteList() {
  siteListEl.innerHTML = "";
  const domains = Object.keys(config).sort();
  if (!domains.length) {
    const empty = document.createElement("div");
    empty.className = "site-list-empty";
    empty.textContent = "No sites yet. Click '+ Add site' to start.";
    siteListEl.appendChild(empty);
    return;
  }
  for (const domain of domains) {
    const li = document.createElement("li");
    if (domain === selectedDomain) li.classList.add("selected");
    const name = document.createElement("span");
    name.textContent = domain;
    li.appendChild(name);
    const entry = config[domain] || {};
    const badges = document.createElement("span");
    badges.className = "badges";
    const h = (entry.hide || []).length;
    const r = (entry.remove || []).length;
    const b = (entry.block || []).length;
    badges.textContent = `hide ${h}  rm ${r}  blk ${b}`;
    li.appendChild(badges);
    li.addEventListener("click", () => {
      selectedDomain = domain;
      render();
    });
    siteListEl.appendChild(li);
  }
}

function renderDetail() {
  if (!selectedDomain || !(selectedDomain in config)) {
    detailEl.innerHTML = '<div class="detail-empty">Select a site on the left, or add a new one.</div>';
    return;
  }
  const entry = config[selectedDomain];

  detailEl.innerHTML = "";

  // Domain editor row
  const domainRow = document.createElement("div");
  domainRow.className = "domain-row";

  const domainInput = document.createElement("input");
  domainInput.type = "text";
  domainInput.value = selectedDomain;
  domainInput.spellcheck = false;
  domainInput.addEventListener("change", async () => {
    const newDomain = domainInput.value.trim();
    if (!newDomain || newDomain === selectedDomain) {
      domainInput.value = selectedDomain;
      return;
    }
    if (newDomain in config) {
      setStatus("A site named '" + newDomain + "' already exists", false);
      domainInput.value = selectedDomain;
      return;
    }
    config[newDomain] = config[selectedDomain];
    delete config[selectedDomain];
    selectedDomain = newDomain;
    await save();
    render();
    setStatus("Renamed site", true);
  });
  domainRow.appendChild(domainInput);

  const delBtn = document.createElement("button");
  delBtn.className = "danger";
  delBtn.textContent = "Delete site";
  delBtn.addEventListener("click", async () => {
    if (!confirm("Delete all rules for '" + selectedDomain + "'?")) return;
    delete config[selectedDomain];
    selectedDomain = null;
    await save();
    render();
    setStatus("Site deleted", true);
  });
  domainRow.appendChild(delBtn);

  detailEl.appendChild(domainRow);

  // Three layer sections, in aggressiveness order: block > remove > hide
  detailEl.appendChild(renderLayerSection(
    entry, "block", "Block (network)",
    "URL patterns blocked at the network layer. Matching requests never leave the browser.",
    "Add URL pattern like ||ads.example.com"
  ));
  detailEl.appendChild(renderLayerSection(
    entry, "remove", "Remove (DOM)",
    "CSS selectors whose matching elements are physically removed from the DOM (and kept out as the page mutates).",
    "Add CSS selector like .modal-overlay"
  ));
  detailEl.appendChild(renderLayerSection(
    entry, "hide", "Hide (CSS)",
    "CSS selectors applied with display: none !important. Elements stay in the DOM but don't render.",
    "Add CSS selector like .popup"
  ));
}

function renderLayerSection(entry, key, title, help, addPlaceholder) {
  if (!Array.isArray(entry[key])) entry[key] = [];
  const arr = entry[key];

  const fs = document.createElement("fieldset");
  fs.className = "layer-section";

  const lg = document.createElement("legend");
  lg.textContent = title;
  fs.appendChild(lg);

  const h = document.createElement("p");
  h.className = "layer-help";
  h.textContent = help;
  fs.appendChild(h);

  const ul = document.createElement("ul");
  ul.className = "entries";
  if (!arr.length) {
    const empty = document.createElement("li");
    empty.className = "entries-empty";
    empty.textContent = "(none)";
    ul.appendChild(empty);
  } else {
    for (let i = 0; i < arr.length; i++) {
      const li = document.createElement("li");
      const txt = document.createElement("span");
      txt.className = "text";
      txt.title = arr[i];
      txt.textContent = arr[i];
      li.appendChild(txt);
      const del = document.createElement("button");
      del.className = "del";
      del.textContent = "\u00d7";
      del.title = "Delete";
      del.addEventListener("click", async () => {
        arr.splice(i, 1);
        await save();
        render();
      });
      li.appendChild(del);
      ul.appendChild(li);
    }
  }
  fs.appendChild(ul);

  // Add row
  const addRow = document.createElement("div");
  addRow.className = "add-row";
  const input = document.createElement("input");
  input.type = "text";
  input.placeholder = addPlaceholder;
  input.spellcheck = false;
  const addBtn = document.createElement("button");
  addBtn.textContent = "+ Add";
  const onAdd = async () => {
    const v = input.value.trim();
    if (!v) return;
    if (arr.includes(v)) {
      setStatus("Already in the list", false);
      return;
    }
    arr.push(v);
    input.value = "";
    await save();
    render();
  };
  addBtn.addEventListener("click", onAdd);
  input.addEventListener("keydown", e => { if (e.key === "Enter") onAdd(); });
  addRow.appendChild(input);
  addRow.appendChild(addBtn);
  fs.appendChild(addRow);

  return fs;
}

addSiteBtn.addEventListener("click", async () => {
  let name = prompt("New site domain (e.g. example.com):", "");
  if (name === null) return;
  name = name.trim();
  if (!name) return;
  if (name in config) {
    setStatus("Site already exists", false);
    selectedDomain = name;
    render();
    return;
  }
  config[name] = { hide: [], remove: [], block: [] };
  selectedDomain = name;
  await save();
  render();
});

debugToggleEl.addEventListener("change", async () => {
  const data = await chrome.storage.local.get(OPTIONS_KEY);
  const options = data[OPTIONS_KEY] || {};
  options.debug = debugToggleEl.checked;
  await chrome.storage.local.set({ [OPTIONS_KEY]: options });
  setStatus(debugToggleEl.checked ? "Verbose logging ON" : "Verbose logging OFF", true);
});

suggestionsToggleEl.addEventListener("change", async () => {
  const data = await chrome.storage.local.get(OPTIONS_KEY);
  const options = data[OPTIONS_KEY] || {};
  options.suggestionsEnabled = suggestionsToggleEl.checked;
  await chrome.storage.local.set({ [OPTIONS_KEY]: options });
  setStatus(
    suggestionsToggleEl.checked
      ? "Behavioral suggestions ON (reload tabs to start scanning)"
      : "Behavioral suggestions OFF",
    true
  );
});

exportBtn.addEventListener("click", async () => {
  const data = await chrome.storage.local.get(STORAGE_KEY);
  const json = JSON.stringify(data[STORAGE_KEY] || {}, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = "hush-config.json";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
  setStatus("Downloaded hush-config.json", true);
});

resetBtn.addEventListener("click", async () => {
  if (!confirm("Reset all sites to the shipped defaults? This will replace your current config.")) return;
  try {
    const seed = await fetch(chrome.runtime.getURL("sites.json")).then(r => r.json());
    config = seed;
    selectedDomain = null;
    await save();
    render();
    setStatus("Reset to defaults", true);
  } catch (e) {
    setStatus("Reset failed: " + e.message, false);
  }
});

jsonApplyBtn.addEventListener("click", async () => {
  let parsed;
  try {
    parsed = JSON.parse(jsonEl.value);
  } catch (e) {
    setStatus("Invalid JSON: " + e.message, false);
    return;
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    setStatus("Config must be a JSON object (keys are domain names).", false);
    return;
  }
  config = parsed;
  if (!(selectedDomain in config)) selectedDomain = null;
  await save();
  render();
  setStatus("Applied JSON", true);
});

jsonRefreshBtn.addEventListener("click", () => {
  jsonEl.value = JSON.stringify(config, null, 2);
  setStatus("Refreshed from current state", true);
});

allowlistSaveBtn.addEventListener("click", async () => {
  const allowlist = {
    iframes: linesToList(allowlistIframesEl.value),
    overlays: linesToList(allowlistOverlaysEl.value),
    suggestions: linesToList(allowlistSuggestionsEl.value)
  };
  await chrome.storage.local.set({ [ALLOWLIST_KEY]: allowlist });
  setStatus(
    "Saved allowlists (" + allowlist.iframes.length + " iframes, " +
    allowlist.overlays.length + " overlays, " +
    allowlist.suggestions.length + " suggestions)",
    true
  );
});

allowlistResetBtn.addEventListener("click", async () => {
  if (!confirm("Reset both allowlists to the shipped defaults?")) return;
  try {
    DEFAULT_ALLOWLIST = await loadDefaultAllowlist();
  } catch (e) {
    setStatus("Reset failed: " + e.message, false);
    return;
  }
  await chrome.storage.local.set({ [ALLOWLIST_KEY]: DEFAULT_ALLOWLIST });
  allowlistIframesEl.value = (DEFAULT_ALLOWLIST.iframes || []).join("\n");
  allowlistOverlaysEl.value = (DEFAULT_ALLOWLIST.overlays || []).join("\n");
  allowlistSuggestionsEl.value = (DEFAULT_ALLOWLIST.suggestions || []).join("\n");
  setStatus("Reset allowlists to defaults", true);
});

loadAll();
