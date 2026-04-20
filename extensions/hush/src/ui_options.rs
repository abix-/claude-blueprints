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

    // Tertiary root: raw JSON editor. Reads the initial config text
    // synchronously from `chrome.storage.local` when its Refresh
    // handler fires so it stays in sync with whatever the JS-owned
    // site list has done.
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
