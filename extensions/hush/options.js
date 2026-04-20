// Options page is an ES module so it can statically import the
// wasm-bindgen glue (matches popup.js pattern from stage 4). The
// Leptos tree mounted via mountOptions owns the two preference
// toggles + status banner. The rest (site list, per-site editor,
// allowlist textareas, JSON editor, export/reset) is still rendered
// here and gets ported in follow-up stage 5 iterations.
import initWasm, { initEngine, mountOptions, setOptionsStatus } from "./dist/pkg/hush.js";

const wasmReady = initWasm().then(() => {
  try { initEngine(); } catch (e) { console.error("[Hush options] initEngine failed", e); }
}).catch(e => console.error("[Hush options] wasm init failed", e));

const STORAGE_KEY = "config";
const OPTIONS_KEY = "options";
const ALLOWLIST_KEY = "allowlist";

const siteListEl = document.getElementById("site-list");
const detailEl = document.getElementById("detail");
const addSiteBtn = document.getElementById("add-site");

let config = {};
let selectedDomain = null;

// Route status feedback through the Leptos StatusBanner. Before wasm
// is ready, swallow the call; the Leptos tree boots within a frame
// of first user interaction.
function setStatus(msg, ok) {
  try { setOptionsStatus(msg, !!ok); } catch (e) { /* wasm not ready yet */ }
}

async function loadAll() {
  const data = await chrome.storage.local.get([STORAGE_KEY, OPTIONS_KEY, ALLOWLIST_KEY]);
  config = data[STORAGE_KEY] || {};
  const opts = data[OPTIONS_KEY] || {};
  const al = data[ALLOWLIST_KEY] || { iframes: [], overlays: [], suggestions: [] };
  render();

  // Mount the Leptos preference toggles + config toolbar + allowlist
  // editor + status banner with initial values read from storage.
  // Each component re-fetches storage on user actions, so the
  // snapshot here is just the boot-time state.
  try {
    await wasmReady;
    mountOptions({
      debug: !!opts.debug,
      suggestionsEnabled: !!opts.suggestionsEnabled,
      allowlist: {
        iframes: al.iframes || [],
        overlays: al.overlays || [],
        suggestions: al.suggestions || [],
      },
    });
  } catch (e) { console.error("[Hush options] mountOptions failed", e); }
}

async function save() {
  await chrome.storage.local.set({ [STORAGE_KEY]: config });
}

function render() {
  renderSiteList();
  renderDetail();
  // The JSON editor (Leptos JsonEditor) reads from storage on its
  // Refresh button and on mount. After the JS site list mutates
  // `config`, the user can click Refresh to resync the textarea.
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

// The suggestions + debug toggles are owned by the Leptos
// SettingsToggles component now (src/ui_options.rs). Toggle clicks
// flip the matching field in chrome.storage.local["options"] via
// chrome_bridge::set_option_bool and surface status through the
// shared StatusBanner.

// The Export JSON + Reset to defaults buttons moved into the Leptos
// ConfigToolbar component (src/ui_options.rs). Export builds a Blob
// via web_sys and triggers a synthetic anchor click; Reset fetches
// sites.json and reloads the page so the remaining JS-owned UI
// re-reads chrome.storage.local.

// The raw JSON editor (Apply / Refresh + textarea) is owned by the
// Leptos JsonEditor component now (src/ui_options.rs). Apply writes
// to chrome.storage.local and reloads the page so the site list
// re-renders from storage.

// The three allowlist textareas and the Save / Reset buttons are
// owned by the Leptos AllowlistEditor component now
// (src/ui_options.rs). Writes go through chrome_bridge::set_allowlist
// and reads through chrome_bridge::get_default_allowlist.

loadAll();
