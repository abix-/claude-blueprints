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
    let root = document
        .get_element_by_id("rust-options-root")
        .ok_or_else(|| JsValue::from_str("no #rust-options-root in options.html"))?;
    let root_el: web_sys::HtmlElement = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsValue::from_str("#rust-options-root is not an HtmlElement"))?;

    std::mem::forget(leptos::mount::mount_to(root_el, move || {
        view! { <OptionsRoot snap=snap.clone() /> }
    }));
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
