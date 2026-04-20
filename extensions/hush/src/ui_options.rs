//! Options UI (Stage 5).
//!
//! Leptos CSR component tree mounted by `options.js` after wasm init.
//! This first iteration scaffolds the mount infrastructure and ports
//! the two preference toggles (debug logging + behavioral-suggestion
//! detector) plus a shared status banner. The larger chunks
//! (site list + per-site editor, allowlist textareas, JSON editor,
//! export/reset toolbar) get ported in follow-up iterations; their
//! existing JS renderers in `options.js` still own those regions of
//! the page.

use crate::chrome_bridge;
use crate::types::{Config, SiteConfig};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Initial snapshot `options.js` hands in at mount time. Keeps the
/// Leptos root from having to re-fetch `chrome.storage.local` on
/// every component boot.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct OptionsSnapshot {
    pub debug: bool,
    pub suggestions_enabled: bool,
    /// Current allowlist from `chrome.storage.local["allowlist"]`
    /// (falls back to empty arrays if the key is absent).
    #[serde(default)]
    pub allowlist: AllowlistSnapshot,
    /// Full site config. Populated on mount from
    /// `chrome.storage.local["config"]` so the Leptos site-list +
    /// per-site editor can own the state reactively from that point
    /// on.
    #[serde(default)]
    pub config: Config,
}

/// Three independent user-editable allowlists. `iframes` is a list of
/// URL substrings; `overlays` is a list of CSS selectors; `suggestions`
/// is a list of suggestion keys populated by the popup's Allow button.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AllowlistSnapshot {
    pub iframes: Vec<String>,
    pub overlays: Vec<String>,
    pub suggestions: Vec<String>,
}

// Handle on the mounted options tree's status signal. Exposed to JS
// via `set_options_status` so the legacy JS handlers for
// export/reset/JSON/allowlist can still surface feedback through the
// same banner the Leptos toggles use.
thread_local! {
    static STATUS_HANDLE: RefCell<Option<RwSignal<Option<StatusMsg>>>> =
        const { RefCell::new(None) };
}

#[derive(Clone)]
struct StatusMsg {
    text: String,
    ok: bool,
}

/// Publish a status-banner message from JS. `ok=true` renders green,
/// `ok=false` renders red. No-op if the Leptos tree isn't mounted yet.
#[wasm_bindgen(js_name = "setOptionsStatus")]
pub fn set_options_status(text: String, ok: bool) {
    if let Some(sig) = STATUS_HANDLE.with(|h| h.borrow().clone()) {
        sig.set(Some(StatusMsg { text, ok }));
        // Auto-hide after 3.5s to match the prior JS behavior.
        let sig_clone = sig;
        let cb = Closure::<dyn Fn()>::new(move || sig_clone.set(None));
        if let Some(window) = web_sys::window() {
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                3500,
            );
            cb.forget();
        }
    }
}

/// WASM entry called by `options.js` once per options-page load.
#[wasm_bindgen(js_name = "mountOptions")]
pub fn mount_options(snapshot: JsValue) -> Result<(), JsValue> {
    let snap: OptionsSnapshot = serde_wasm_bindgen::from_value(snapshot)
        .map_err(|e| JsValue::from_str(&format!("mountOptions: {e}")))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    // Main root: toggles + config toolbar + status banner.
    let root = document
        .get_element_by_id("rust-options-root")
        .ok_or_else(|| JsValue::from_str("no #rust-options-root in options.html"))?;
    let root_el: web_sys::HtmlElement = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsValue::from_str("#rust-options-root is not an HtmlElement"))?;
    {
        let snap = snap.clone();
        std::mem::forget(leptos::mount::mount_to(root_el, move || {
            view! { <OptionsRoot snap=snap.clone() /> }
        }));
    }

    // Secondary root: allowlist editor. Lives in a separate `<details>`
    // wrapper in options.html, hence a separate mount; it shares the
    // main tree's status banner by calling setOptionsStatus through
    // the wasm export.
    if let Some(allow_root) = document.get_element_by_id("rust-allowlist-root") {
        if let Ok(el) = allow_root.dyn_into::<web_sys::HtmlElement>() {
            let allow_snap = snap.allowlist.clone();
            std::mem::forget(leptos::mount::mount_to(el, move || {
                view! { <AllowlistEditor snap=allow_snap.clone() /> }
            }));
        }
    }

    // Site list + per-site editor.
    if let Some(config_root) = document.get_element_by_id("rust-config-root") {
        if let Ok(el) = config_root.dyn_into::<web_sys::HtmlElement>() {
            let cfg = snap.config.clone();
            std::mem::forget(leptos::mount::mount_to(el, move || {
                view! { <ConfigEditor initial=cfg.clone() /> }
            }));
        }
    }

    // Tertiary root: raw JSON editor. Reads the initial config text
    // synchronously from `chrome.storage.local` when its Refresh
    // handler fires so it stays in sync with the config editor's
    // mutations.
    if let Some(json_root) = document.get_element_by_id("rust-json-root") {
        if let Ok(el) = json_root.dyn_into::<web_sys::HtmlElement>() {
            std::mem::forget(leptos::mount::mount_to(el, move || {
                view! { <JsonEditor /> }
            }));
        }
    }

    Ok(())
}

#[component]
fn OptionsRoot(snap: OptionsSnapshot) -> impl IntoView {
    let status = RwSignal::new(Option::<StatusMsg>::None);
    STATUS_HANDLE.with(|h| {
        *h.borrow_mut() = Some(status);
    });

    view! {
        <SettingsToggles
            initial_debug=snap.debug
            initial_suggestions=snap.suggestions_enabled
            status=status
        />
        <ConfigToolbar status=status />
        <StatusBanner status=status />
    }
}

/// Export + Reset buttons for the site config. Matches the JS
/// `exportBtn` / `resetBtn` behavior:
///
/// - Export: read `chrome.storage.local["config"]` as JSON, create a
///   `Blob`, trigger a download via a synthetic anchor click, revoke
///   the object URL.
/// - Reset: `confirm()` with the user, fetch `sites.json`, write it
///   into `chrome.storage.local["config"]`, then reload the options
///   page so the JS-owned site list and JSON editor re-read storage.
#[component]
fn ConfigToolbar(status: RwSignal<Option<StatusMsg>>) -> impl IntoView {
    let busy = RwSignal::new(false);

    let on_export = move |_| {
        if busy.get() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match chrome_bridge::get_config_json().await {
                Ok(json) => {
                    if let Err(e) = trigger_json_download(&json, "hush-config.json") {
                        status.set(Some(StatusMsg {
                            text: format!("Export failed: {:?}", e),
                            ok: false,
                        }));
                    } else {
                        status.set(Some(StatusMsg {
                            text: "Downloaded hush-config.json".into(),
                            ok: true,
                        }));
                    }
                }
                Err(e) => {
                    status.set(Some(StatusMsg {
                        text: format!("Export failed: {:?}", e),
                        ok: false,
                    }));
                }
            }
            set_auto_hide(status);
            busy.set(false);
        });
    };

    let on_reset = move |_| {
        if busy.get() {
            return;
        }
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let ok = window
            .confirm_with_message(
                "Reset all sites to the shipped defaults? This will replace your current config.",
            )
            .unwrap_or(false);
        if !ok {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match chrome_bridge::reset_config_to_defaults().await {
                Ok(()) => {
                    // Reload the page so the JS-owned site list + JSON
                    // editor re-read chrome.storage.local and reflect
                    // the new config. Once those get ported to Leptos
                    // this becomes a signal update instead.
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().reload();
                    }
                }
                Err(e) => {
                    status.set(Some(StatusMsg {
                        text: format!("Reset failed: {:?}", e),
                        ok: false,
                    }));
                    set_auto_hide(status);
                    busy.set(false);
                }
            }
        });
    };

    view! {
        <div class="rust-config-toolbar"
             style="display:flex; gap: 8px; margin-top: 4px;">
            <button on:click=on_export
                    disabled=move || busy.get()
                    style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                           background: #fff; border: 1px solid #ccc;
                           border-radius: 5px;">
                "Export JSON"
            </button>
            <button on:click=on_reset
                    disabled=move || busy.get()
                    style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                           background: #fff; color: #8a1616;
                           border: 1px solid #d7b7b7; border-radius: 5px;">
                "Reset to defaults"
            </button>
        </div>
    }
}

/// Allowlist editor: three `<textarea>`s (iframes, overlays,
/// suggestion keys) plus Save + Reset buttons. Save serializes
/// trimmed non-empty lines and writes the triple back to
/// `chrome.storage.local["allowlist"]`. Reset fetches
/// `allowlist.defaults.json`, writes it to storage, and updates the
/// textarea signals so the UI reflects the reset without a reload.
#[component]
fn AllowlistEditor(snap: AllowlistSnapshot) -> impl IntoView {
    let iframes = RwSignal::new(snap.iframes.join("\n"));
    let overlays = RwSignal::new(snap.overlays.join("\n"));
    let suggestions = RwSignal::new(snap.suggestions.join("\n"));
    let busy = RwSignal::new(false);

    let on_save = move |_| {
        if busy.get() {
            return;
        }
        busy.set(true);
        let i = lines_to_list(&iframes.get());
        let o = lines_to_list(&overlays.get());
        let s = lines_to_list(&suggestions.get());
        spawn_local(async move {
            let (i_n, o_n, s_n) = (i.len(), o.len(), s.len());
            let res = chrome_bridge::set_allowlist(i, o, s).await;
            match res {
                Ok(()) => {
                    set_options_status(
                        format!(
                            "Saved allowlists ({} iframes, {} overlays, {} suggestions)",
                            i_n, o_n, s_n
                        ),
                        true,
                    );
                }
                Err(e) => {
                    set_options_status(format!("Save failed: {:?}", e), false);
                }
            }
            busy.set(false);
        });
    };

    let on_reset = move |_| {
        if busy.get() {
            return;
        }
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let ok = window
            .confirm_with_message("Reset both allowlists to the shipped defaults?")
            .unwrap_or(false);
        if !ok {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match chrome_bridge::get_default_allowlist().await {
                Ok((i, o, s)) => {
                    let i_clone = i.clone();
                    let o_clone = o.clone();
                    let s_clone = s.clone();
                    match chrome_bridge::set_allowlist(i_clone, o_clone, s_clone).await {
                        Ok(()) => {
                            iframes.set(i.join("\n"));
                            overlays.set(o.join("\n"));
                            suggestions.set(s.join("\n"));
                            set_options_status("Reset allowlists to defaults".into(), true);
                        }
                        Err(e) => {
                            set_options_status(format!("Reset failed: {:?}", e), false);
                        }
                    }
                }
                Err(e) => {
                    set_options_status(format!("Reset failed: {:?}", e), false);
                }
            }
            busy.set(false);
        });
    };

    let textarea_style = "width:100%;height:140px;font-family:ui-monospace,monospace;\
                          font-size:12px;padding:10px;border:1px solid #ccc;\
                          border-radius:5px;box-sizing:border-box;";
    let h3_style = "font-size: 13px; margin: 14px 0 4px; font-weight: 600;";
    let help_style = "color:#666; font-size:12px; margin: 0 0 8px;";

    view! {
        <div class="rust-allowlist-editor">
            <h3 style=h3_style>"Iframe allowlist"</h3>
            <p style=help_style>
                "One URL substring per line. If a hidden iframe's src contains "
                "any entry (case-insensitive), it's allowed through without "
                "surfacing as a remove suggestion."
            </p>
            <textarea spellcheck="false"
                      style=textarea_style
                      prop:value=move || iframes.get()
                      on:input=move |ev| {
                          iframes.set(event_target_value(&ev));
                      }>
            </textarea>

            <h3 style=h3_style>"Sticky-overlay allowlist"</h3>
            <p style=help_style>
                "One CSS selector per line. If a flagged sticky/fixed element "
                "matches any selector, it's allowed through. Used to skip React "
                "Portals, modal roots, and framework shells that legitimately "
                "cover the viewport."
            </p>
            <textarea spellcheck="false"
                      style=textarea_style
                      prop:value=move || overlays.get()
                      on:input=move |ev| {
                          overlays.set(event_target_value(&ev));
                      }>
            </textarea>

            <h3 style=h3_style>"Suggestion allowlist"</h3>
            <p style=help_style>
                "One suggestion key per line. Populated by the popup's "
                <b>"Allow"</b>
                " button; any listed key is filtered out at emit time, on "
                "every site, forever. Remove a line to re-enable a suggestion. "
                "Keys look like "
                <code>"block::||example.com::canvas-fp"</code>
                " or "
                <code>{r#"remove::iframe[src*="captcha.com"]"#}</code>
                "."
            </p>
            <textarea spellcheck="false"
                      style=textarea_style
                      prop:value=move || suggestions.get()
                      on:input=move |ev| {
                          suggestions.set(event_target_value(&ev));
                      }>
            </textarea>

            <div class="toolbar" style="margin-top: 10px; display:flex; gap: 8px;">
                <button on:click=on_save
                        disabled=move || busy.get()
                        class="primary"
                        style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                               background: #2b7cff; color: white;
                               border: 1px solid #2b7cff; border-radius: 5px;">
                    "Save allowlists"
                </button>
                <button on:click=on_reset
                        disabled=move || busy.get()
                        style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                               background: #fff; border: 1px solid #ccc;
                               border-radius: 5px;">
                    "Reset to defaults"
                </button>
            </div>
        </div>
    }
}

/// Top-level site configuration editor. Owns the full `Config` map
/// and the currently-selected domain as reactive signals. Hosts the
/// sidebar `SiteList` and the right-pane `SiteDetail`.
#[component]
fn ConfigEditor(initial: Config) -> impl IntoView {
    let config = RwSignal::new(initial);
    let selected = RwSignal::new(Option::<String>::None);

    view! {
        <div class="two-pane">
            <aside class="sidebar">
                <h2>"Configured sites"</h2>
                <SiteList config=config selected=selected />
            </aside>
            <section class="detail">
                <SiteDetail config=config selected=selected />
            </section>
        </div>
    }
}

/// Sidebar listing the configured domains. Domains are iterated in
/// sorted order - matches the `Object.keys(config).sort()` the old JS
/// site list used. Each row shows per-layer entry counts and is
/// click-to-select; the "+ Add site" button at the bottom prompts
/// for a domain name and seeds an empty triple.
#[component]
fn SiteList(
    config: RwSignal<Config>,
    selected: RwSignal<Option<String>>,
) -> impl IntoView {
    let on_add = move |_| {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return,
        };
        let input = match window
            .prompt_with_message_and_default("New site domain (e.g. example.com):", "")
        {
            Ok(Some(s)) => s,
            _ => return,
        };
        let name = input.trim().to_string();
        if name.is_empty() {
            return;
        }
        if config.with(|c| c.contains_key(&name)) {
            set_options_status("Site already exists".into(), false);
            selected.set(Some(name));
            return;
        }
        config.update(|c| {
            c.insert(name.clone(), SiteConfig::default());
        });
        selected.set(Some(name));
        persist_config(config);
    };

    view! {
        <ul class="site-list">
            {move || {
                let mut keys: Vec<String> = config.with(|c| c.keys().cloned().collect());
                keys.sort();
                if keys.is_empty() {
                    view! {
                        <div class="site-list-empty">
                            "No sites yet. Click '+ Add site' to start."
                        </div>
                    }.into_any()
                } else {
                    keys.into_iter().map(|domain| {
                        view! {
                            <SiteListRow
                                config=config
                                selected=selected
                                domain=domain
                            />
                        }
                    }).collect::<Vec<_>>().into_any()
                }
            }}
        </ul>
        <div class="sidebar-actions">
            <button on:click=on_add
                    class="primary"
                    style="width:100%; padding: 6px 10px; background: #2b7cff;
                           color: white; border: 1px solid #2b7cff;
                           border-radius: 5px; cursor: pointer;">
                "+ Add site"
            </button>
        </div>
    }
}

/// Single row in the site list. Reads counts reactively so adding a
/// layer entry updates the badge without re-sorting the full list.
#[component]
fn SiteListRow(
    config: RwSignal<Config>,
    selected: RwSignal<Option<String>>,
    domain: String,
) -> impl IntoView {
    let row_domain = domain.clone();
    let click_domain = domain.clone();
    let on_click = move |_| {
        selected.set(Some(click_domain.clone()));
    };

    let badges = {
        let d = row_domain.clone();
        move || {
            let (h, r, b) = config.with(|c| match c.get(&d) {
                Some(entry) => (entry.hide.len(), entry.remove.len(), entry.block.len()),
                None => (0, 0, 0),
            });
            format!("hide {}  rm {}  blk {}", h, r, b)
        }
    };

    let class_name = {
        let d = row_domain.clone();
        move || {
            if selected.with(|s| s.as_deref() == Some(d.as_str())) {
                "selected".to_string()
            } else {
                String::new()
            }
        }
    };

    view! {
        <li class=class_name on:click=on_click>
            <span>{row_domain}</span>
            <span class="badges">{badges}</span>
        </li>
    }
}

/// Right-pane detail for the currently-selected site. Shows the
/// domain rename input, a delete-site button, and the three
/// layer sections (Block / Remove / Hide).
#[component]
fn SiteDetail(
    config: RwSignal<Config>,
    selected: RwSignal<Option<String>>,
) -> impl IntoView {
    move || {
        let current = selected.get();
        let Some(domain) = current else {
            return view! {
                <div class="detail-empty">
                    "Select a site on the left, or add a new one."
                </div>
            }
            .into_any();
        };
        if !config.with(|c| c.contains_key(&domain)) {
            return view! {
                <div class="detail-empty">
                    "Select a site on the left, or add a new one."
                </div>
            }
            .into_any();
        }
        view! {
            <SiteDetailBody
                config=config
                selected=selected
                domain=domain.clone()
            />
        }
        .into_any()
    }
}

/// Inner per-site body. Split out so the rename input + layer
/// sections can take an owned `domain` string and rebuild cleanly
/// when the selected signal flips.
#[component]
fn SiteDetailBody(
    config: RwSignal<Config>,
    selected: RwSignal<Option<String>>,
    domain: String,
) -> impl IntoView {
    let domain_input = RwSignal::new(domain.clone());
    let domain_original = domain.clone();

    // Reset the input when the selected domain changes (e.g. user
    // clicks a different site before committing a rename).
    {
        let original = domain_original.clone();
        Effect::new(move |_| {
            if selected.with(|s| s.as_deref() != Some(original.as_str())) {
                domain_input.set(original.clone());
            }
        });
    }

    let on_rename = {
        let original = domain_original.clone();
        move |_| {
            let next = domain_input.get().trim().to_string();
            if next.is_empty() || next == original {
                domain_input.set(original.clone());
                return;
            }
            if config.with(|c| c.contains_key(&next)) {
                set_options_status(format!("A site named '{}' already exists", next), false);
                domain_input.set(original.clone());
                return;
            }
            config.update(|c| {
                if let Some(entry) = c.shift_remove(&original) {
                    c.insert(next.clone(), entry);
                }
            });
            selected.set(Some(next));
            persist_config(config);
            set_options_status("Renamed site".into(), true);
        }
    };

    let on_delete = {
        let original = domain_original.clone();
        move |_| {
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let prompt = format!("Delete all rules for '{}'?", original);
            if !window.confirm_with_message(&prompt).unwrap_or(false) {
                return;
            }
            config.update(|c| {
                c.shift_remove(&original);
            });
            selected.set(None);
            persist_config(config);
            set_options_status("Site deleted".into(), true);
        }
    };

    view! {
        <div class="domain-row">
            <input type="text"
                   spellcheck="false"
                   prop:value=move || domain_input.get()
                   on:input=move |ev| {
                       let val = ev.target()
                           .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                           .map(|i| i.value())
                           .unwrap_or_default();
                       domain_input.set(val);
                   }
                   on:change=on_rename
            />
            <button class="danger" on:click=on_delete>
                "Delete site"
            </button>
        </div>
        <LayerSection
            config=config
            domain=domain.clone()
            layer=LayerKind::Block
        />
        <LayerSection
            config=config
            domain=domain.clone()
            layer=LayerKind::Remove
        />
        <LayerSection
            config=config
            domain=domain.clone()
            layer=LayerKind::Hide
        />
        <LayerSection
            config=config
            domain=domain.clone()
            layer=LayerKind::Spoof
        />
    }
}

/// Which of the config-layer arrays a `LayerSection` edits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerKind {
    Block,
    Remove,
    Hide,
    Spoof,
}

impl LayerKind {
    fn title(&self) -> &'static str {
        match self {
            Self::Block => "Block (network)",
            Self::Remove => "Remove (DOM)",
            Self::Hide => "Hide (CSS)",
            Self::Spoof => "Spoof (fingerprint)",
        }
    }
    fn help(&self) -> &'static str {
        match self {
            Self::Block => {
                "URL patterns blocked at the network layer. Matching requests never \
                 leave the browser."
            }
            Self::Remove => {
                "CSS selectors whose matching elements are physically removed from the \
                 DOM (and kept out as the page mutates)."
            }
            Self::Hide => {
                "CSS selectors applied with display: none !important. Elements stay in \
                 the DOM but don't render."
            }
            Self::Spoof => {
                "Fingerprint kind tags to neutralize by returning bland, \
                 identical-across-users values instead of blocking the site. \
                 Currently supported: `webgl-unmasked` (WebGL UNMASKED_VENDOR + \
                 UNMASKED_RENDERER)."
            }
        }
    }
    fn placeholder(&self) -> &'static str {
        match self {
            Self::Block => "Add URL pattern like ||ads.example.com",
            Self::Remove => "Add CSS selector like .modal-overlay",
            Self::Hide => "Add CSS selector like .popup",
            Self::Spoof => "Add kind tag like webgl-unmasked",
        }
    }
    fn read<'a>(&self, cfg: &'a SiteConfig) -> &'a [String] {
        match self {
            Self::Block => &cfg.block,
            Self::Remove => &cfg.remove,
            Self::Hide => &cfg.hide,
            Self::Spoof => &cfg.spoof,
        }
    }
    fn modify<'a>(&self, cfg: &'a mut SiteConfig) -> &'a mut Vec<String> {
        match self {
            Self::Block => &mut cfg.block,
            Self::Remove => &mut cfg.remove,
            Self::Hide => &mut cfg.hide,
            Self::Spoof => &mut cfg.spoof,
        }
    }
}

/// One of the three `<fieldset>` editors on a site's detail page.
/// Lists the current entries with a delete button on each row, plus
/// an Add input + button at the bottom. All mutations go through
/// the shared `config` signal and persist via `set_config`.
#[component]
fn LayerSection(
    config: RwSignal<Config>,
    domain: String,
    layer: LayerKind,
) -> impl IntoView {
    let draft = RwSignal::new(String::new());

    // Entry rows are derived reactively from the config signal.
    let rows = {
        let domain = domain.clone();
        move || {
            let entries: Vec<String> = config.with(|c| {
                c.get(&domain)
                    .map(|cfg| layer.read(cfg).to_vec())
                    .unwrap_or_default()
            });
            if entries.is_empty() {
                view! {
                    <li class="entries-empty">"(none)"</li>
                }
                .into_any()
            } else {
                entries
                    .into_iter()
                    .enumerate()
                    .map(|(idx, text)| {
                        let d = domain.clone();
                        let title = text.clone();
                        let body = text.clone();
                        let on_del = move |_| {
                            let d = d.clone();
                            config.update(|c| {
                                if let Some(entry) = c.get_mut(&d) {
                                    let arr = layer.modify(entry);
                                    if idx < arr.len() {
                                        arr.remove(idx);
                                    }
                                }
                            });
                            persist_config(config);
                        };
                        view! {
                            <li>
                                <span class="text" title=title>{body}</span>
                                <button class="del"
                                        title="Delete"
                                        on:click=on_del>
                                    "\u{00d7}"
                                </button>
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_any()
            }
        }
    };

    // Shared add logic. Both the click handler and the Enter-key
    // handler delegate here so there's one source of truth for the
    // dedup + push + clear-draft sequence.
    let add_entry = {
        let domain = domain.clone();
        move || {
            let value = draft.get().trim().to_string();
            if value.is_empty() {
                return;
            }
            let already = config.with(|c| {
                c.get(&domain)
                    .map(|cfg| layer.read(cfg).iter().any(|v| v == &value))
                    .unwrap_or(false)
            });
            if already {
                set_options_status("Already in the list".into(), false);
                return;
            }
            let d = domain.clone();
            let v = value.clone();
            config.update(|c| {
                if let Some(entry) = c.get_mut(&d) {
                    layer.modify(entry).push(v);
                }
            });
            draft.set(String::new());
            persist_config(config);
        }
    };

    let on_click = {
        let add_entry = add_entry.clone();
        move |_| add_entry()
    };

    let on_keydown = {
        let add_entry = add_entry.clone();
        move |ev: web_sys::KeyboardEvent| {
            if ev.key() == "Enter" {
                add_entry();
            }
        }
    };

    view! {
        <fieldset class="layer-section">
            <legend>{layer.title()}</legend>
            <p class="layer-help">{layer.help()}</p>
            <ul class="entries">
                {rows}
            </ul>
            <div class="add-row">
                <input type="text"
                       spellcheck="false"
                       placeholder=layer.placeholder()
                       prop:value=move || draft.get()
                       on:input=move |ev| {
                           let val = ev.target()
                               .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                               .map(|i| i.value())
                               .unwrap_or_default();
                           draft.set(val);
                       }
                       on:keydown=on_keydown
                />
                <button on:click=on_click>"+ Add"</button>
            </div>
        </fieldset>
    }
}

/// Persist the current `Config` signal value to
/// `chrome.storage.local["config"]`. Fire-and-forget - any storage
/// errors surface through the status banner so the user can retry.
fn persist_config(config: RwSignal<Config>) {
    let snapshot = config.get_untracked();
    spawn_local(async move {
        if let Err(e) = chrome_bridge::set_config(&snapshot).await {
            set_options_status(format!("Save failed: {:?}", e), false);
        }
    });
}

/// Raw JSON editor: one `<textarea>` + Apply + Refresh buttons.
/// Apply parses the textarea, validates the shape
/// (top-level object, not array), writes it to
/// `chrome.storage.local["config"]`, and reloads the page so the
/// still-JS-owned site list re-renders. Refresh reads the current
/// config back out of storage and updates the textarea signal.
#[component]
fn JsonEditor() -> impl IntoView {
    let text = RwSignal::new(String::new());
    let busy = RwSignal::new(false);

    // Load initial value from storage on mount. Without this the
    // textarea starts empty and Refresh is the only way to populate
    // it, which is surprising.
    let initial_text = text;
    spawn_local(async move {
        match chrome_bridge::get_config_json().await {
            Ok(json) => initial_text.set(json),
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                "[Hush options] initial JSON load failed: {:?}",
                e
            ))),
        }
    });

    let on_refresh = move |_| {
        if busy.get() {
            return;
        }
        busy.set(true);
        spawn_local(async move {
            match chrome_bridge::get_config_json().await {
                Ok(json) => {
                    text.set(json);
                    set_options_status("Refreshed from current state".into(), true);
                }
                Err(e) => {
                    set_options_status(format!("Refresh failed: {:?}", e), false);
                }
            }
            busy.set(false);
        });
    };

    let on_apply = move |_| {
        if busy.get() {
            return;
        }
        busy.set(true);
        let current = text.get();
        spawn_local(async move {
            match chrome_bridge::set_config_from_json(&current).await {
                Ok(()) => {
                    // Reload so the JS-owned site list picks up the
                    // new config. Same pattern as Reset-to-defaults.
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().reload();
                    }
                }
                Err(e) => {
                    let msg = e
                        .as_string()
                        .unwrap_or_else(|| format!("{:?}", e));
                    set_options_status(format!("Apply failed: {}", msg), false);
                    busy.set(false);
                }
            }
        });
    };

    view! {
        <div class="rust-json-editor">
            <textarea spellcheck="false"
                      style="width:100%;min-height:240px;font-family:ui-monospace,monospace;\
                             font-size:12px;padding:10px;border:1px solid #ccc;\
                             border-radius:5px;box-sizing:border-box;"
                      prop:value=move || text.get()
                      on:input=move |ev| {
                          text.set(event_target_value(&ev));
                      }>
            </textarea>
            <div class="toolbar" style="margin-top: 10px; display:flex; gap: 8px;">
                <button on:click=on_apply
                        disabled=move || busy.get()
                        class="primary"
                        style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                               background: #2b7cff; color: white;
                               border: 1px solid #2b7cff; border-radius: 5px;">
                    "Apply JSON"
                </button>
                <button on:click=on_refresh
                        disabled=move || busy.get()
                        style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                               background: #fff; border: 1px solid #ccc;
                               border-radius: 5px;">
                    "Refresh from storage"
                </button>
            </div>
        </div>
    }
}

/// Split a `\n`-separated textarea value into a trimmed, non-empty
/// list. Mirrors the old JS `linesToList` helper.
fn lines_to_list(text: &str) -> Vec<String> {
    text.split(|c| c == '\n' || c == '\r')
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

/// Read the `value` prop off a DOM event's target. Common enough
/// across the editor textareas to hoist out.
fn event_target_value(ev: &web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
        .map(|ta| ta.value())
        .unwrap_or_default()
}

/// Trigger a browser download of `content` under `filename`. Uses a
/// synthetic `<a download>` anchor plus `URL.createObjectURL` to avoid
/// needing a server roundtrip.
fn trigger_json_download(content: &str, filename: &str) -> Result<(), JsValue> {
    use js_sys::Array;
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;

    let parts = Array::new();
    parts.push(&JsValue::from_str(content));
    // No MIME type arg: the `.json` filename on the anchor is what
    // drives the browser's save dialog. Avoiding options also dodges
    // needing the `BlobPropertyBag` web-sys feature flag.
    let blob = web_sys::Blob::new_with_str_sequence(&parts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    let anchor = document
        .create_element("a")?
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .map_err(|_| JsValue::from_str("failed to cast anchor"))?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    let body = document
        .body()
        .ok_or_else(|| JsValue::from_str("no body"))?;
    body.append_child(&anchor)?;
    anchor.click();
    body.remove_child(&anchor)?;
    web_sys::Url::revoke_object_url(&url)?;
    Ok(())
}

/// Two preference checkboxes: enable behavioral suggestions, enable
/// verbose console logging. Each flips the matching boolean in
/// `chrome.storage.local["options"]` via
/// [`chrome_bridge::set_option_bool`] and surfaces a status message.
#[component]
fn SettingsToggles(
    initial_debug: bool,
    initial_suggestions: bool,
    status: RwSignal<Option<StatusMsg>>,
) -> impl IntoView {
    let debug = RwSignal::new(initial_debug);
    let suggestions = RwSignal::new(initial_suggestions);

    let toggle_suggestions = move |_| {
        let next = !suggestions.get();
        suggestions.set(next);
        spawn_local(async move {
            match chrome_bridge::set_option_bool("suggestionsEnabled", next).await {
                Ok(()) => {
                    let text = if next {
                        "Behavioral suggestions ON (reload tabs to start scanning)"
                    } else {
                        "Behavioral suggestions OFF"
                    };
                    status.set(Some(StatusMsg { text: text.to_string(), ok: true }));
                    set_auto_hide(status);
                }
                Err(e) => {
                    status.set(Some(StatusMsg {
                        text: format!("Save failed: {:?}", e),
                        ok: false,
                    }));
                    set_auto_hide(status);
                }
            }
        });
    };

    let toggle_debug = move |_| {
        let next = !debug.get();
        debug.set(next);
        spawn_local(async move {
            match chrome_bridge::set_option_bool("debug", next).await {
                Ok(()) => {
                    let text = if next {
                        "Verbose logging ON"
                    } else {
                        "Verbose logging OFF"
                    };
                    status.set(Some(StatusMsg { text: text.to_string(), ok: true }));
                    set_auto_hide(status);
                }
                Err(e) => {
                    status.set(Some(StatusMsg {
                        text: format!("Save failed: {:?}", e),
                        ok: false,
                    }));
                    set_auto_hide(status);
                }
            }
        });
    };

    view! {
        <div class="rust-settings-toggles"
             style="display: flex; flex-direction: column; gap: 10px;">
            <label>
                <input type="checkbox"
                       prop:checked=move || suggestions.get()
                       on:change=toggle_suggestions />
                " Enable behavioral suggestions "
                <span style="color:#888; font-size:12px;">
                    "(opt-in; adds a small per-scan CPU cost; see \"How Hush works\" above)"
                </span>
            </label>
            <label>
                <input type="checkbox"
                       prop:checked=move || debug.get()
                       on:change=toggle_debug />
                " Enable verbose console logging "
                <span style="color:#888; font-size:12px;">"(off by default)"</span>
            </label>
        </div>
    }
}

/// Transient inline status banner. Mirrors the old JS `setStatus`
/// behavior: green when `ok`, red otherwise, hides after 3.5s.
#[component]
fn StatusBanner(status: RwSignal<Option<StatusMsg>>) -> impl IntoView {
    view! {
        {move || match status.get() {
            Some(msg) => {
                let (bg, fg) = if msg.ok {
                    ("#e8f6ea", "#1a5d2a")
                } else {
                    ("#fdecea", "#8a1616")
                };
                view! {
                    <div class="rust-options-status"
                         style=format!(
                             "display:inline-block; margin-top: 8px; padding: 4px 10px;
                              background: {bg}; color: {fg}; border-radius: 4px;
                              font-size: 12px;"
                         )>
                        {msg.text}
                    </div>
                }.into_any()
            }
            None => view! { <span /> }.into_any(),
        }}
    }
}

/// Schedule the status banner to clear itself after 3.5 seconds. The
/// JS equivalent used a single `clearTimeout` guard; here we just
/// schedule, and a newer message simply overwrites the signal so the
/// old timer's cleanup is a no-op.
fn set_auto_hide(status: RwSignal<Option<StatusMsg>>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cb = Closure::<dyn Fn()>::new(move || status.set(None));
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        3500,
    );
    cb.forget();
}
