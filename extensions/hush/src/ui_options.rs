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
use crate::types::{Config, RuleEntry, SiteConfig};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

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

    // Flat firewall-style rule table: one row per rule across every
    // scope and action, with scope/action as inline cells.
    if let Some(config_root) = document.get_element_by_id("rust-config-root") {
        if let Ok(el) = config_root.dyn_into::<web_sys::HtmlElement>() {
            let cfg = snap.config.clone();
            std::mem::forget(leptos::mount::mount_to(el, move || {
                view! { <RulesTable initial=cfg.clone() /> }
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
        <ProfileTools status=status />
        <UrlSimulator />
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

/// Self-describing header for a profile JSON file. Gated so an
/// accidental import of some unrelated JSON dies with a clear
/// error rather than nuking the user's config.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProfileHeader {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_profile_version")]
    version: u32,
}

fn default_profile_version() -> u32 {
    1
}

/// A profile file on disk. `hushProfile` header identifies the
/// shape; `config` is the same IndexMap<scope, SiteConfig> the
/// live config uses, so export/import is a pure round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Profile {
    #[serde(rename = "hushProfile")]
    header: ProfileHeader,
    config: Config,
}

#[derive(Default, Clone, Copy)]
struct MergeStats {
    added: usize,
    skipped: usize,
}

/// Union an incoming profile's config into the current one.
/// Dedup is by `(scope, action, value)`; existing rows keep their
/// `disabled` / `tags` / `comment` metadata untouched (the import
/// doesn't overwrite a user-maintained annotation with the
/// profile's default one). Returns added / skipped counts for the
/// confirmation banner.
fn merge_profile_into_config(current: &mut Config, incoming: &Config) -> MergeStats {
    let mut stats = MergeStats::default();
    for (scope, site_in) in incoming.iter() {
        let site_cur = current.entry(scope.clone()).or_default();
        for action in LayerKind::ALL {
            let to_add: Vec<RuleEntry> = action.read(site_in).to_vec();
            let cur = action.modify(site_cur);
            for entry in to_add {
                if cur.iter().any(|e| e.value == entry.value) {
                    stats.skipped += 1;
                } else {
                    cur.push(entry);
                    stats.added += 1;
                }
            }
        }
    }
    stats
}

/// Profile import / export row. Two buttons: Import merges a
/// profile JSON (with `hushProfile` header) into the current
/// config; Export wraps the current config with a user-supplied
/// name / description and triggers a download. Import uses a
/// dedup union — existing rules keep their metadata, new rules
/// append to the end of the target bucket.
#[component]
fn ProfileTools(status: RwSignal<Option<StatusMsg>>) -> impl IntoView {
    let busy = RwSignal::new(false);
    let file_input_id = "hush-profile-import";

    let on_import_click = move |_| {
        let Some(document) = web_sys::window().and_then(|w| w.document()) else {
            return;
        };
        let Some(el) = document.get_element_by_id(file_input_id) else {
            return;
        };
        if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
            input.click();
        }
    };

    let on_file_change = move |ev: web_sys::Event| {
        if busy.get() {
            return;
        }
        let Some(input) = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        else {
            return;
        };
        let Some(files) = input.files() else {
            return;
        };
        let Some(file) = files.get(0) else {
            return;
        };
        busy.set(true);
        // Drop the picked file out of the input so picking the
        // same file twice in a row still fires `change`.
        input.set_value("");
        spawn_local(async move {
            let text_promise = file.text();
            match JsFuture::from(text_promise).await {
                Ok(v) => {
                    let text = v.as_string().unwrap_or_default();
                    if let Err(e) = handle_profile_import(&text, status).await {
                        let msg = e
                            .as_string()
                            .unwrap_or_else(|| format!("{:?}", e));
                        set_options_status(format!("Import failed: {msg}"), false);
                    }
                }
                Err(e) => {
                    set_options_status(format!("Import failed: {:?}", e), false);
                }
            }
            busy.set(false);
        });
    };

    let on_export = move |_| {
        if busy.get() {
            return;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let name = match window
            .prompt_with_message_and_default("Profile name:", "my-profile")
        {
            Ok(Some(n)) => n.trim().to_string(),
            _ => return,
        };
        if name.is_empty() {
            return;
        }
        let description = match window.prompt_with_message_and_default(
            "Profile description (optional):",
            "",
        ) {
            Ok(Some(d)) => d.trim().to_string(),
            _ => String::new(),
        };
        busy.set(true);
        let name_for_file = sanitize_filename(&name);
        spawn_local(async move {
            let config = match chrome_bridge::get_popup_storage().await {
                Ok(s) => s.config,
                Err(e) => {
                    set_options_status(format!("Export failed: {:?}", e), false);
                    busy.set(false);
                    return;
                }
            };
            let profile = Profile {
                header: ProfileHeader { name, description, version: 1 },
                config,
            };
            match profile_to_pretty_json(&profile) {
                Ok(json) => {
                    let filename = format!("hush-profile-{name_for_file}.json");
                    if let Err(e) = trigger_json_download(&json, &filename) {
                        set_options_status(
                            format!("Export failed: {:?}", e),
                            false,
                        );
                    } else {
                        set_options_status(
                            format!("Downloaded {filename}"),
                            true,
                        );
                    }
                }
                Err(e) => {
                    set_options_status(format!("Export failed: {:?}", e), false);
                }
            }
            busy.set(false);
        });
    };

    let _ = status;

    view! {
        <div class="rust-profile-toolbar"
             style="display:flex; gap: 8px; margin-top: 8px; align-items: center;">
            <button on:click=on_import_click
                    disabled=move || busy.get()
                    style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                           background: #fff; border: 1px solid #ccc;
                           border-radius: 5px;">
                "Import profile..."
            </button>
            <button on:click=on_export
                    disabled=move || busy.get()
                    style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                           background: #fff; border: 1px solid #ccc;
                           border-radius: 5px;">
                "Export as profile..."
            </button>
            <span style="color:#888; font-size:12px;">
                "Profiles merge — they won't overwrite your existing rules."
            </span>
            <input type="file"
                   id=file_input_id
                   accept="application/json,.json"
                   style="display:none;"
                   on:change=on_file_change />
        </div>
    }
}

/// Shared import handler: parse profile JSON, confirm counts
/// with the user, merge into the live config, persist. Caller
/// handles the busy-state and error-surface wrapping.
async fn handle_profile_import(
    text: &str,
    _status: RwSignal<Option<StatusMsg>>,
) -> Result<(), JsValue> {
    let profile: Profile = parse_profile_json(text).map_err(|e| {
        JsValue::from_str(&format!("not a valid Hush profile: {:?}", e))
    })?;
    // Preview the merge without writing, so the user can confirm.
    let current = chrome_bridge::get_popup_storage().await?.config;
    let mut preview = current.clone();
    let stats = merge_profile_into_config(&mut preview, &profile.config);
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let desc = if profile.header.description.is_empty() {
        String::new()
    } else {
        format!("\n\n{}", profile.header.description)
    };
    let prompt = format!(
        "Import profile \"{}\"?{}\n\n\
         {} new rule{} will be added, {} already exist and will be skipped.\n\n\
         Existing rule metadata (disabled / tags / comments) is preserved.",
        profile.header.name,
        desc,
        stats.added,
        if stats.added == 1 { "" } else { "s" },
        stats.skipped,
    );
    let confirmed = window.confirm_with_message(&prompt).unwrap_or(false);
    if !confirmed {
        set_options_status("Import cancelled".into(), false);
        return Ok(());
    }
    chrome_bridge::set_config(&preview).await?;
    set_options_status(
        format!(
            "Imported \"{}\": {} added, {} skipped. Reloading...",
            profile.header.name, stats.added, stats.skipped
        ),
        true,
    );
    // Reload so the Leptos rules table re-reads the merged config.
    if let Some(w) = web_sys::window() {
        let _ = w.location().reload();
    }
    Ok(())
}

/// Parse a profile file via JS `JSON.parse` + `serde_wasm_bindgen`,
/// avoiding a direct `serde_json` runtime dependency.
fn parse_profile_json(text: &str) -> Result<Profile, JsValue> {
    let parsed = js_sys::JSON::parse(text)?;
    serde_wasm_bindgen::from_value(parsed)
        .map_err(|e| JsValue::from_str(&format!("{e}")))
}

/// Serialize a profile to pretty JSON via `chrome_bridge::to_js` +
/// `JSON.stringify`. Matches the existing serialization path for
/// config writes so we don't drift on map-vs-object handling.
fn profile_to_pretty_json(profile: &Profile) -> Result<String, JsValue> {
    let js = chrome_bridge::to_js(profile)
        .map_err(|e| JsValue::from_str(&format!("{e}")))?;
    let pretty = js_sys::JSON::stringify_with_replacer_and_space(
        &js,
        &JsValue::NULL,
        &JsValue::from_f64(2.0),
    )?;
    pretty
        .as_string()
        .ok_or_else(|| JsValue::from_str("JSON.stringify returned non-string"))
}

/// Trim a user-supplied profile name down to a safe filename
/// fragment. Keeps alphanumerics, dashes, and underscores;
/// everything else collapses to `-`.
fn sanitize_filename(name: &str) -> String {
    let mut out = String::new();
    let mut last_was_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            out.push('-');
            last_was_dash = true;
        }
    }
    out.trim_matches('-').to_string()
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

/// Which of the config-layer arrays a rule lives in. Drives column
/// labels, placeholders, and the action `<select>` options in the
/// flat rules table. Order here is the canonical top-down render
/// order within each scope (Block first, Spoof last).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum LayerKind {
    Block,
    Allow,
    Neuter,
    Silence,
    Remove,
    Hide,
    Spoof,
}

impl LayerKind {
    const ALL: [Self; 7] = [
        Self::Block,
        Self::Allow,
        Self::Neuter,
        Self::Silence,
        Self::Remove,
        Self::Hide,
        Self::Spoof,
    ];

    fn short_label(&self) -> &'static str {
        match self {
            Self::Block => "Block",
            Self::Allow => "Allow",
            Self::Neuter => "Neuter",
            Self::Silence => "Silence",
            Self::Remove => "Remove",
            Self::Hide => "Hide",
            Self::Spoof => "Spoof",
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Allow => "allow",
            Self::Neuter => "neuter",
            Self::Silence => "silence",
            Self::Remove => "remove",
            Self::Hide => "hide",
            Self::Spoof => "spoof",
        }
    }
    fn from_str(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|k| k.as_str() == s)
    }
    fn placeholder(&self) -> &'static str {
        match self {
            Self::Block => "||ads.example.com",
            Self::Allow => "||example.com/api  or  .allowed-node",
            Self::Neuter => "||hotjar.com",
            Self::Silence => "||hotjar.com",
            Self::Remove => ".modal-overlay",
            Self::Hide => ".sticky-promo",
            Self::Spoof => "webgl-unmasked",
        }
    }
    fn badge_color(&self) -> &'static str {
        match self {
            Self::Block => "#d85c4f",
            Self::Allow => "#2f9e4a",
            Self::Neuter => "#6b5cd4",
            Self::Silence => "#4fa89a",
            Self::Remove => "#c77a2b",
            Self::Hide => "#888",
            Self::Spoof => "#2b7cff",
        }
    }
    fn read<'a>(&self, cfg: &'a SiteConfig) -> &'a [RuleEntry] {
        match self {
            Self::Block => &cfg.block,
            Self::Allow => &cfg.allow,
            Self::Neuter => &cfg.neuter,
            Self::Silence => &cfg.silence,
            Self::Remove => &cfg.remove,
            Self::Hide => &cfg.hide,
            Self::Spoof => &cfg.spoof,
        }
    }
    fn modify<'a>(&self, cfg: &'a mut SiteConfig) -> &'a mut Vec<RuleEntry> {
        match self {
            Self::Block => &mut cfg.block,
            Self::Allow => &mut cfg.allow,
            Self::Neuter => &mut cfg.neuter,
            Self::Silence => &mut cfg.silence,
            Self::Remove => &mut cfg.remove,
            Self::Hide => &mut cfg.hide,
            Self::Spoof => &mut cfg.spoof,
        }
    }
}

/// Snapshot of one rule for table rendering. Captured per render pass
/// so row components don't have to re-walk the `Config` signal to
/// pull their own values out of the bucket.
#[derive(Clone, Debug)]
struct FlatRow {
    scope: String,
    action: LayerKind,
    idx: usize,
    bucket_len: usize,
    entry: RuleEntry,
}

/// Rule health surfaced in the options editor as a colored dot +
/// tooltip. Computed from the persistent firewall log, the union
/// of broken-selector sets across every tab, and the shadow-lint
/// pass on block rules. Stage 12 phase B rule-health audit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RuleHealth {
    Disabled,
    Broken,
    Shadowed,
    Firing,
    NoHits,
}

impl RuleHealth {
    fn color(&self) -> &'static str {
        match self {
            Self::Disabled => "#bbb",
            Self::Broken => "#d85c4f",
            Self::Shadowed => "#e0a048",
            Self::Firing => "#2f9e4a",
            Self::NoHits => "#d0d0d0",
        }
    }
    fn tooltip(&self, hits: u32, shadowed_by: Option<&str>) -> String {
        match self {
            Self::Disabled => "disabled (evaluator skips this rule)".to_string(),
            Self::Broken => {
                "invalid selector: threw on querySelectorAll / element.matches"
                    .to_string()
            }
            Self::Shadowed => match shadowed_by {
                Some(a) => format!("shadowed by allow: {a}"),
                None => "shadowed by an allow rule".to_string(),
            },
            Self::Firing => {
                if hits == 1 {
                    "1 hit this session".to_string()
                } else {
                    format!("{hits} hits this session")
                }
            }
            Self::NoHits => "no hits this session".to_string(),
        }
    }
}

/// Side data fed into the rules table so each row can compute its
/// health without reaching back into the background. Populated
/// async at mount time; empty until the first fetch resolves.
/// Shadow-lint needs the full allow set from the live config,
/// which is supplied fresh per render pass by the caller since
/// it mutates reactively.
#[derive(Clone, Default)]
struct HealthData {
    hits_by_rule: std::collections::HashMap<String, u32>,
    broken_remove: std::collections::HashSet<String>,
    broken_hide: std::collections::HashSet<String>,
    broken_allow: std::collections::HashSet<String>,
}

impl HealthData {
    fn health_for(
        &self,
        row: &FlatRow,
        all_allows: &[RuleEntry],
    ) -> (RuleHealth, Option<String>, u32) {
        let value = row.entry.value.as_str();
        let id = crate::types::rule_id(row.action.as_str(), &row.scope, value);
        let hits = self.hits_by_rule.get(&id).copied().unwrap_or(0);
        if row.entry.disabled {
            return (RuleHealth::Disabled, None, hits);
        }
        // Broken only applies to DOM-selector actions; invalid URL
        // patterns for block/allow don't throw synchronously.
        match row.action {
            LayerKind::Remove if self.broken_remove.contains(value) => {
                return (RuleHealth::Broken, None, hits);
            }
            LayerKind::Hide if self.broken_hide.contains(value) => {
                return (RuleHealth::Broken, None, hits);
            }
            LayerKind::Allow if self.broken_allow.contains(value) => {
                return (RuleHealth::Broken, None, hits);
            }
            _ => {}
        }
        if row.action == LayerKind::Block {
            if let Some(shadow) = crate::lint::block_shadowed_by(all_allows, value) {
                return (RuleHealth::Shadowed, Some(shadow.value.clone()), hits);
            }
        }
        if hits > 0 {
            (RuleHealth::Firing, None, hits)
        } else {
            (RuleHealth::NoHits, None, hits)
        }
    }
}

fn flatten_rules(cfg: &Config) -> Vec<FlatRow> {
    let mut out = Vec::new();
    let mut keys: Vec<&String> = cfg.keys().collect();
    // Global pinned first; everything else in IndexMap order.
    keys.sort_by_key(|k| (k.as_str() != crate::types::GLOBAL_SCOPE_KEY, k.to_string()));
    for scope in keys {
        let Some(site) = cfg.get(scope) else { continue };
        for action in LayerKind::ALL {
            let bucket = action.read(site);
            for (idx, entry) in bucket.iter().enumerate() {
                out.push(FlatRow {
                    scope: scope.clone(),
                    action,
                    idx,
                    bucket_len: bucket.len(),
                    entry: entry.clone(),
                });
            }
        }
    }
    out
}

fn site_keys_for_scope_select(cfg: &Config) -> Vec<String> {
    let mut keys: Vec<String> = cfg
        .keys()
        .filter(|k| k.as_str() != crate::types::GLOBAL_SCOPE_KEY)
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// Append a rule to the `(scope, action)` bucket, creating the scope
/// entry if it doesn't exist yet.
fn append_rule(cfg: &mut Config, scope: &str, action: LayerKind, entry: RuleEntry) {
    let site = cfg.entry(scope.to_string()).or_default();
    action.modify(site).push(entry);
}

/// Pop the rule at `(scope, action, idx)` out of its bucket. Returns
/// `None` if the bucket has fewer entries than `idx`.
fn take_rule(
    cfg: &mut Config,
    scope: &str,
    action: LayerKind,
    idx: usize,
) -> Option<RuleEntry> {
    let site = cfg.get_mut(scope)?;
    let bucket = action.modify(site);
    if idx >= bucket.len() {
        return None;
    }
    Some(bucket.remove(idx))
}

/// Flat firewall-style rule table. One row per rule across every
/// scope and action, with scope/action as inline `<select>` cells. The
/// underlying storage stays [`Config = IndexMap<scope, SiteConfig>`]
/// with seven [`Vec<RuleEntry>`] fields per scope; this table
/// flattens on read and routes every write back to the right bucket.
#[component]
fn RulesTable(initial: Config) -> impl IntoView {
    let config = RwSignal::new(initial);
    let scope_filter = RwSignal::new(String::new());
    let action_filter = RwSignal::new(String::new());
    let search = RwSignal::new(String::new());
    let health = RwSignal::new(HealthData::default());

    // Async fetch: persistent firewall events (for hit counts) +
    // union of broken-selector sets across every tab. Both feed the
    // per-row health dot. Allows list is derived from the config
    // signal inside the render closure so reorder/edit updates
    // re-evaluate shadow without refetching.
    spawn_local(async move {
        let events = chrome_bridge::get_firewall_events(-1)
            .await
            .unwrap_or_default();
        let broken = chrome_bridge::get_all_broken_selectors()
            .await
            .unwrap_or_default();
        let mut hits: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        for ev in events {
            *hits.entry(ev.rule_id).or_insert(0) += 1;
        }
        health.update(|h| {
            h.hits_by_rule = hits;
            h.broken_remove = broken.remove.into_iter().collect();
            h.broken_hide = broken.hide.into_iter().collect();
            h.broken_allow = broken.allow.into_iter().collect();
        });
    });

    let rows_view = move || {
        let scope_f = scope_filter.get();
        let action_f = action_filter.get();
        let q = search.get().to_lowercase();
        let rows: Vec<FlatRow> = config.with(flatten_rules);
        let filtered: Vec<FlatRow> = rows
            .into_iter()
            .filter(|r| {
                if !scope_f.is_empty() && r.scope != scope_f {
                    return false;
                }
                if !action_f.is_empty() && r.action.as_str() != action_f {
                    return false;
                }
                if !q.is_empty() {
                    let hay = format!(
                        "{} {} {} {}",
                        r.entry.value,
                        r.entry.tags.join(" "),
                        r.entry.comment.clone().unwrap_or_default(),
                        r.scope
                    )
                    .to_lowercase();
                    if !hay.contains(&q) {
                        return false;
                    }
                }
                true
            })
            .collect();

        if filtered.is_empty() {
            return view! {
                <tr>
                    <td colspan="8" class="rules-empty">
                        "No rules match. Clear filters or add a rule below."
                    </td>
                </tr>
            }
            .into_any();
        }

        // Health input for this render pass. Allows are derived
        // from the live config, so shadow detection re-evaluates
        // whenever the user adds / reorders / disables an allow
        // rule. Hit counts + broken sets come from the async fetch
        // and stay stable for the life of the options page.
        let all_allows: Vec<RuleEntry> = config.with(|c| {
            let mut v: Vec<RuleEntry> = Vec::new();
            for site in c.values() {
                v.extend(site.allow.iter().cloned());
            }
            v
        });
        let h_snap = health.get();

        filtered
            .into_iter()
            .enumerate()
            .map(|(flat_idx, row)| {
                let (status, shadowed_by, hits) =
                    h_snap.health_for(&row, &all_allows);
                view! {
                    <RuleRow
                        config=config
                        row=row
                        flat_idx=flat_idx + 1
                        status=status
                        shadowed_by=shadowed_by
                        hits=hits
                    />
                }
            })
            .collect::<Vec<_>>()
            .into_any()
    };

    let on_add = move |_| {
        let entry = RuleEntry::new(String::new());
        let scope = crate::types::GLOBAL_SCOPE_KEY.to_string();
        let action = LayerKind::Block;
        config.update(|c| append_rule(c, &scope, action, entry));
        persist_config(config);
    };

    view! {
        <div class="rules-table">
            <FilterBar
                config=config
                scope_filter=scope_filter
                action_filter=action_filter
                search=search
            />
            <table class="rules-grid">
                <thead>
                    <tr>
                        <th class="col-on">"On"</th>
                        <th class="col-num">"#"</th>
                        <th class="col-scope">"Scope"</th>
                        <th class="col-action">"Action"</th>
                        <th class="col-match">"Match"</th>
                        <th class="col-tags">"Tags"</th>
                        <th class="col-comment">"Comment"</th>
                        <th class="col-ops">""</th>
                    </tr>
                </thead>
                <tbody>
                    {rows_view}
                </tbody>
            </table>
            <div class="rules-footer">
                <button class="primary" on:click=on_add>
                    "+ Add rule"
                </button>
                <span class="rules-hint">
                    "Rules evaluate first-match-wins within each action. \
                     Scope and action are editable per row."
                </span>
            </div>
        </div>
    }
}

/// Filter bar above the rules table. Scope + action dropdowns
/// driven by what's actually in the config; free-text search over
/// value / tags / comment / scope.
#[component]
fn FilterBar(
    config: RwSignal<Config>,
    scope_filter: RwSignal<String>,
    action_filter: RwSignal<String>,
    search: RwSignal<String>,
) -> impl IntoView {
    let scope_options = move || {
        let sites = config.with(site_keys_for_scope_select);
        let mut out: Vec<(String, String)> = vec![
            ("".into(), "All scopes".into()),
            (
                crate::types::GLOBAL_SCOPE_KEY.into(),
                "Global".into(),
            ),
        ];
        for s in sites {
            out.push((s.clone(), s));
        }
        out.into_iter()
            .map(|(val, label)| {
                let selected = scope_filter.with(|s| s.as_str() == val);
                view! {
                    <option value=val prop:selected=selected>{label}</option>
                }
            })
            .collect::<Vec<_>>()
    };

    let action_options = move || {
        let mut out: Vec<(String, String)> =
            vec![("".into(), "All actions".into())];
        for a in LayerKind::ALL {
            out.push((a.as_str().into(), a.short_label().into()));
        }
        out.into_iter()
            .map(|(val, label)| {
                let selected = action_filter.with(|s| s.as_str() == val);
                view! {
                    <option value=val prop:selected=selected>{label}</option>
                }
            })
            .collect::<Vec<_>>()
    };

    let on_clear = move |_| {
        scope_filter.set(String::new());
        action_filter.set(String::new());
        search.set(String::new());
    };

    view! {
        <div class="rules-filter-bar">
            <select
                class="filter-scope"
                on:change=move |ev| scope_filter.set(select_value(&ev))
            >
                {scope_options}
            </select>
            <select
                class="filter-action"
                on:change=move |ev| action_filter.set(select_value(&ev))
            >
                {action_options}
            </select>
            <input
                type="text"
                class="filter-search"
                placeholder="Search match / tags / comment / scope"
                prop:value=move || search.get()
                on:input=move |ev| search.set(input_value(&ev))
            />
            <button class="filter-clear" on:click=on_clear>"Clear"</button>
        </div>
    }
}

/// One row in the rules table. Receives a snapshot and the live
/// `config` signal; every cell change routes back through the
/// snapshot's `(scope, action, idx)` coordinate.
#[component]
fn RuleRow(
    config: RwSignal<Config>,
    row: FlatRow,
    flat_idx: usize,
    status: RuleHealth,
    shadowed_by: Option<String>,
    hits: u32,
) -> impl IntoView {
    let FlatRow { scope, action, idx, bucket_len, entry } = row;
    let scope_c = scope.clone();
    let val_c = entry.value.clone();
    let tags_c = entry.tags.join(", ");
    let comment_c = entry.comment.clone().unwrap_or_default();
    let disabled = entry.disabled;

    let scope_options = {
        let current_scope = scope.clone();
        move || {
            let existing = config.with(site_keys_for_scope_select);
            let mut out: Vec<(String, String)> = vec![(
                crate::types::GLOBAL_SCOPE_KEY.into(),
                "Global".into(),
            )];
            for s in existing {
                out.push((s.clone(), s));
            }
            out.push(("__add_new__".into(), "+ New site...".into()));
            out.into_iter()
                .map(|(val, label)| {
                    let selected = val == current_scope;
                    view! {
                        <option value=val prop:selected=selected>{label}</option>
                    }
                })
                .collect::<Vec<_>>()
        }
    };

    let action_options = move || {
        LayerKind::ALL
            .into_iter()
            .map(|k| {
                let selected = k == action;
                view! {
                    <option value=k.as_str() prop:selected=selected>
                        {k.short_label()}
                    </option>
                }
            })
            .collect::<Vec<_>>()
    };

    let on_toggle = {
        let s = scope.clone();
        move |_| {
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s)
                    && let Some(row) = action.modify(site).get_mut(idx)
                {
                    row.disabled = !row.disabled;
                }
            });
            persist_config(config);
        }
    };

    let on_match_change = {
        let s = scope.clone();
        move |ev: web_sys::Event| {
            let val = input_value(&ev).trim().to_string();
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s)
                    && let Some(row) = action.modify(site).get_mut(idx)
                {
                    row.value = val.clone();
                }
            });
            persist_config(config);
        }
    };

    let on_tags_change = {
        let s = scope.clone();
        move |ev: web_sys::Event| {
            let raw = input_value(&ev);
            let tags: Vec<String> = raw
                .split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect();
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s)
                    && let Some(row) = action.modify(site).get_mut(idx)
                {
                    row.tags = tags.clone();
                }
            });
            persist_config(config);
        }
    };

    let on_comment_change = {
        let s = scope.clone();
        move |ev: web_sys::Event| {
            let raw = input_value(&ev).trim().to_string();
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s)
                    && let Some(row) = action.modify(site).get_mut(idx)
                {
                    row.comment = if raw.is_empty() { None } else { Some(raw.clone()) };
                }
            });
            persist_config(config);
        }
    };

    let on_scope_change = {
        let s = scope.clone();
        move |ev: web_sys::Event| {
            let next = select_value(&ev);
            let s = s.clone();
            let target_scope = if next == "__add_new__" {
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => return,
                };
                let input = match window.prompt_with_message_and_default(
                    "New site hostname (e.g. example.com):",
                    "",
                ) {
                    Ok(Some(v)) => v.trim().to_string(),
                    _ => return,
                };
                if input.is_empty() {
                    return;
                }
                input
            } else {
                next
            };
            if target_scope == s {
                return;
            }
            config.update(|c| {
                if let Some(entry) = take_rule(c, &s, action, idx) {
                    append_rule(c, &target_scope, action, entry);
                }
            });
            persist_config(config);
        }
    };

    let on_action_change = {
        let s = scope.clone();
        move |ev: web_sys::Event| {
            let next = select_value(&ev);
            let Some(target_action) = LayerKind::from_str(&next) else {
                return;
            };
            if target_action == action {
                return;
            }
            let s = s.clone();
            config.update(|c| {
                if let Some(entry) = take_rule(c, &s, action, idx) {
                    append_rule(c, &s, target_action, entry);
                }
            });
            persist_config(config);
        }
    };

    let on_up = {
        let s = scope.clone();
        move |_| {
            if idx == 0 {
                return;
            }
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s) {
                    let b = action.modify(site);
                    if idx < b.len() {
                        b.swap(idx, idx - 1);
                    }
                }
            });
            persist_config(config);
        }
    };

    let on_down = {
        let s = scope.clone();
        move |_| {
            let s = s.clone();
            config.update(|c| {
                if let Some(site) = c.get_mut(&s) {
                    let b = action.modify(site);
                    if idx + 1 < b.len() {
                        b.swap(idx, idx + 1);
                    }
                }
            });
            persist_config(config);
        }
    };

    let on_delete = {
        let s = scope.clone();
        move |_| {
            let s = s.clone();
            config.update(|c| {
                take_rule(c, &s, action, idx);
            });
            persist_config(config);
        }
    };

    let up_disabled = idx == 0;
    let down_disabled = idx + 1 >= bucket_len;
    let row_class = if disabled { "rule-row disabled" } else { "rule-row" };
    let match_class = if disabled { "match-input strike" } else { "match-input" };
    let _ = scope_c;

    view! {
        <tr class=row_class>
            <td class="col-on">
                <input type="checkbox"
                       prop:checked=!disabled
                       on:change=on_toggle />
            </td>
            <td class="col-num">
                <span class="status-dot"
                      title=status.tooltip(hits, shadowed_by.as_deref())
                      style=format!("background:{};", status.color())></span>
                {flat_idx.to_string()}
            </td>
            <td class="col-scope">
                <select on:change=on_scope_change>
                    {scope_options}
                </select>
            </td>
            <td class="col-action">
                <span class="action-dot"
                      style=format!("background:{};", action.badge_color())></span>
                <select on:change=on_action_change>
                    {action_options}
                </select>
            </td>
            <td class="col-match">
                <input type="text"
                       class=match_class
                       spellcheck="false"
                       placeholder=action.placeholder()
                       prop:value=val_c
                       on:change=on_match_change />
            </td>
            <td class="col-tags">
                <input type="text"
                       class="meta-input"
                       spellcheck="false"
                       placeholder="tags, comma-separated"
                       prop:value=tags_c
                       on:change=on_tags_change />
            </td>
            <td class="col-comment">
                <input type="text"
                       class="meta-input"
                       spellcheck="false"
                       placeholder="comment"
                       prop:value=comment_c
                       on:change=on_comment_change />
            </td>
            <td class="col-ops">
                <button class="op"
                        title="Move up in bucket"
                        prop:disabled=up_disabled
                        on:click=on_up>"\u{2191}"</button>
                <button class="op"
                        title="Move down in bucket"
                        prop:disabled=down_disabled
                        on:click=on_down>"\u{2193}"</button>
                <button class="op del"
                        title="Delete rule"
                        on:click=on_delete>"\u{00d7}"</button>
            </td>
        </tr>
    }
}

/// Read the value off a DOM event's target as a select element.
fn select_value(ev: &web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

/// Read the value off a DOM event's target as a text input element.
fn input_value(ev: &web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default()
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

/// Rule simulator / test-match. User types a URL (and optionally a
/// site hostname to simulate "as if on this site"); the pure
/// `simulate::simulate_url` walks the active config and returns
/// every rule that would fire plus the DNR winner. Read-only —
/// nothing fires, no network egress, no config write.
#[component]
fn UrlSimulator() -> impl IntoView {
    let url = RwSignal::new(String::new());
    let site = RwSignal::new(String::new());
    let busy = RwSignal::new(false);
    let matches: RwSignal<Vec<crate::simulate::RuleMatch>> = RwSignal::new(Vec::new());
    let ran = RwSignal::new(false);

    let on_simulate = move |_| {
        if busy.get() {
            return;
        }
        let url_val = url.get().trim().to_string();
        if url_val.is_empty() {
            set_options_status("Enter a URL first".into(), false);
            return;
        }
        busy.set(true);
        let site_val = site.get().trim().to_string();
        // If site is empty, infer the URL's host so the site-scope
        // suffix match still resolves. Keeps the "just paste a URL"
        // workflow one-click.
        let inferred_site = if site_val.is_empty() {
            infer_host(&url_val)
        } else {
            site_val
        };
        spawn_local(async move {
            let config = match chrome_bridge::get_popup_storage().await {
                Ok(s) => s.config,
                Err(e) => {
                    set_options_status(format!("Load failed: {:?}", e), false);
                    busy.set(false);
                    return;
                }
            };
            let result = crate::simulate::simulate_url(&config, &inferred_site, &url_val);
            let n = result.len();
            matches.set(result);
            ran.set(true);
            busy.set(false);
            set_options_status(
                format!(
                    "Simulated: {} rule{} matched",
                    n,
                    if n == 1 { "" } else { "s" }
                ),
                true,
            );
        });
    };

    let input_style = "width:100%; font-size: 12px; padding: 4px 8px; \
                       border: 1px solid #ccc; border-radius: 4px; \
                       box-sizing: border-box; font-family: ui-monospace, monospace;";

    view! {
        <details class="rust-simulator" style="margin-top: 16px;">
            <summary style="cursor: pointer; font-weight: 600; font-size: 13px;">
                "Test a URL against your rules"
            </summary>
            <p style="color:#666; font-size:12px; margin: 4px 0 8px;">
                "Read-only audit. Enter a URL; optionally pin a site scope
                 (defaults to the URL's host). Every matching rule across
                 global + site scopes is listed, with the DNR winner flagged."
            </p>
            <div style="display: flex; gap: 8px; margin-bottom: 6px;">
                <input type="text"
                       placeholder="https://doubleclick.net/adx/ad"
                       style=format!("{} flex: 2;", input_style)
                       prop:value=move || url.get()
                       on:input=move |ev| {
                           let v = ev.target()
                               .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                               .map(|i| i.value())
                               .unwrap_or_default();
                           url.set(v);
                       } />
                <input type="text"
                       placeholder="site hostname (optional)"
                       style=format!("{} flex: 1;", input_style)
                       prop:value=move || site.get()
                       on:input=move |ev| {
                           let v = ev.target()
                               .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                               .map(|i| i.value())
                               .unwrap_or_default();
                           site.set(v);
                       } />
                <button on:click=on_simulate
                        disabled=move || busy.get()
                        class="primary"
                        style="padding: 6px 14px; font-size: 13px; cursor: pointer;
                               background: #2b7cff; color: white;
                               border: 1px solid #2b7cff; border-radius: 5px;
                               white-space: nowrap;">
                    "Simulate"
                </button>
            </div>
            {move || {
                if !ran.get() {
                    return view! { <div /> }.into_any();
                }
                let ms = matches.get();
                if ms.is_empty() {
                    return view! {
                        <div style="color:#666; font-size:12px; padding: 8px 0;
                                    font-style: italic;">
                            "No rules matched."
                        </div>
                    }.into_any();
                }
                let rows = ms.into_iter().map(render_match_row).collect::<Vec<_>>();
                view! {
                    <table style="width: 100%; font-size: 12px;
                                  border-collapse: collapse; margin-top: 6px;">
                        <thead>
                            <tr style="border-bottom: 2px solid #ddd; color: #666;">
                                <th style="text-align: left; padding: 3px 6px;">""</th>
                                <th style="text-align: left; padding: 3px 6px;">"action"</th>
                                <th style="text-align: left; padding: 3px 6px;">"scope"</th>
                                <th style="text-align: left; padding: 3px 6px;">"match"</th>
                                <th style="text-align: left; padding: 3px 6px;">"priority"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {rows}
                        </tbody>
                    </table>
                }.into_any()
            }}
        </details>
    }
}

fn render_match_row(m: crate::simulate::RuleMatch) -> impl IntoView {
    let (badge_bg, badge_label) = match m.action.as_str() {
        "block" => ("#d85c4f", "BLOCK"),
        "allow" => ("#2f9e4a", "ALLOW"),
        "neuter" => ("#6b5cd4", "NEUTER"),
        "silence" => ("#4fa89a", "SILENCE"),
        _ => ("#666", m.action.as_str()),
    };
    let winner_marker = if m.is_winner { "\u{2713}" } else { "" };
    let winner_style = if m.is_winner {
        "color: #2f9e4a; font-weight: 700; font-size: 14px;"
    } else {
        ""
    };
    let text_style = if m.disabled {
        "color: #999; text-decoration: line-through; font-family: ui-monospace, monospace;"
    } else {
        "font-family: ui-monospace, monospace;"
    };
    view! {
        <tr style="border-bottom: 1px dotted #e8e8e8;">
            <td style=format!("padding: 4px 6px; {}", winner_style)>
                {winner_marker}
            </td>
            <td style="padding: 4px 6px;">
                <span style=format!(
                    "display:inline-block; padding: 1px 6px; background: {};
                     color: #fff; border-radius: 3px; font-size: 10px;
                     font-weight: 600; font-family: ui-monospace, monospace;",
                    badge_bg
                )>
                    {badge_label.to_string()}
                </span>
            </td>
            <td style="padding: 4px 6px; color: #555;">{m.scope}</td>
            <td style=format!("padding: 4px 6px; {}", text_style)>{m.value}</td>
            <td style="padding: 4px 6px; color: #777; font-family: ui-monospace, monospace;">
                {m.priority}
            </td>
        </tr>
    }
}

/// Best-effort host extraction from a user-typed URL. Used so the
/// simulator's "site scope" input can default to the URL's own host
/// when the user leaves it blank. No `url` crate dependency here —
/// we already have `web_sys::Url` from Leptos transitively.
fn infer_host(raw: &str) -> String {
    let with_scheme = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("https://{raw}")
    };
    web_sys::Url::new(&with_scheme)
        .ok()
        .map(|u| u.host())
        .unwrap_or_default()
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
