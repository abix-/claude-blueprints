//! Popup UI (Stage 4).
//!
//! Leptos CSR component tree for `popup.html`. `popup.js` queries
//! chrome.tabs + chrome.runtime for an initial snapshot and hands it
//! to [`mount_popup`]. Leptos owns the matched-site header, the
//! activity summary, and the suggestions list (including the
//! Add / Dismiss / Allow actions, which call chrome.runtime.sendMessage
//! via [`crate::chrome_bridge`] directly). The per-section JS
//! renderers for blocked URLs, removed-element evidence, and block
//! diagnostics still live in popup.js and get ported in follow-up
//! iterations.

use crate::chrome_bridge;
use crate::types::{Suggestion, SuggestionLayer};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Handle to the popup's top-level suggestions signal + its tab id.
// Populated on mount, consumed by [`refresh_popup_suggestions`] when
// JS-side buttons (Enable / Scan once / Rescan) want to re-trigger a
// fetch without re-mounting the component tree. Single-threaded WASM
// so a `thread_local!(RefCell<Option<_>>)` is the right primitive.
thread_local! {
    static POPUP_HANDLE: RefCell<Option<PopupHandle>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct PopupHandle {
    suggestions: RwSignal<Vec<Suggestion>>,
    tab_id: Option<i32>,
}

/// Called by `popup.js` after enable/scan/rescan clicks so the Leptos
/// suggestion list refreshes without tearing down the whole component
/// tree. No-op if no popup is currently mounted or we have no tab id.
#[wasm_bindgen(js_name = "refreshPopupSuggestions")]
pub fn refresh_popup_suggestions() {
    let Some(handle) = POPUP_HANDLE.with(|h| h.borrow().clone()) else {
        return;
    };
    let Some(tab_id) = handle.tab_id else {
        return;
    };
    spawn_local(async move {
        match chrome_bridge::get_suggestions(tab_id).await {
            Ok(next) => handle.suggestions.set(next),
            Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                "[Hush popup] refresh_popup_suggestions: {:?}",
                e
            ))),
        }
    });
}

/// Snapshot popup.js hands in at mount time.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PopupSnapshot {
    pub hostname: String,
    pub matched_domain: Option<String>,
    pub block_count: u32,
    pub remove_count: u32,
    pub hide_count: u32,
    pub suggestion_count: u32,
    /// Active tab id. Needed for per-tab dismiss + re-fetch calls.
    /// `None` when the popup opens outside a normal tab context.
    pub tab_id: Option<i32>,
    /// Initial suggestions list. Leptos re-fetches after each action
    /// mutation, but the initial render avoids the round-trip.
    pub suggestions: Vec<Suggestion>,
    /// Whether the behavioral detector is enabled in user options.
    /// Affects the "enable detector" CTA copy.
    pub detector_enabled: bool,
}

/// WASM entry. Called by `popup.js` once per popup open.
#[wasm_bindgen(js_name = "mountPopup")]
pub fn mount_popup(snapshot: JsValue) -> Result<(), JsValue> {
    let snap: PopupSnapshot = serde_wasm_bindgen::from_value(snapshot)
        .map_err(|e| JsValue::from_str(&format!("mountPopup: {e}")))?;

    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no document"))?;
    let root = document
        .get_element_by_id("rust-popup-root")
        .ok_or_else(|| JsValue::from_str("no #rust-popup-root in popup.html"))?;
    let root_el: web_sys::HtmlElement = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsValue::from_str("#rust-popup-root is not an HtmlElement"))?;

    std::mem::forget(leptos::mount::mount_to(root_el, move || {
        view! { <PopupRoot snap=snap.clone() /> }
    }));
    Ok(())
}

#[component]
fn PopupRoot(snap: PopupSnapshot) -> impl IntoView {
    // Suggestions are the only reactive state on the popup today.
    // Other fields are init-only; we read them once.
    let suggestions = RwSignal::new(snap.suggestions.clone());
    let tab_id = snap.tab_id;
    let hostname = snap.hostname.clone();

    // Expose the signal to JS-side buttons via refresh_popup_suggestions.
    POPUP_HANDLE.with(|h| {
        *h.borrow_mut() = Some(PopupHandle { suggestions, tab_id });
    });

    view! {
        <MatchedSite
            hostname=snap.hostname.clone()
            matched_domain=snap.matched_domain.clone()
        />
        <ActivitySummary
            block_count=snap.block_count
            remove_count=snap.remove_count
            hide_count=snap.hide_count
            suggestion_count=snap.suggestion_count
        />
        <SuggestionsList
            suggestions=suggestions
            hostname=hostname
            tab_id=tab_id
        />
    }
}

#[component]
fn MatchedSite(hostname: String, matched_domain: Option<String>) -> impl IntoView {
    let matched = matched_domain.clone();
    let hostname_owned = hostname.clone();
    let show_suffix = match (&matched, hostname.is_empty()) {
        (Some(m), false) => m != &hostname,
        _ => false,
    };

    view! {
        <div class="rust-matched-site"
             style="padding: 6px 10px; font-size: 12px;
                    background: #f5f7fb; border: 1px solid #e0e6ef;
                    border-radius: 4px; margin: 6px 0;">
            {move || match matched.clone() {
                Some(m) => view! {
                    <span>
                        "Matched: "
                        <b style="color: #2d4d8a;">{m.clone()}</b>
                        {if show_suffix {
                            view! { <span style="color:#999;"> " (" {hostname_owned.clone()} ")" </span> }.into_any()
                        } else {
                            view! { <span /> }.into_any()
                        }}
                    </span>
                }.into_any(),
                None if hostname.is_empty() => view! {
                    <span style="color:#999;">"No active tab"</span>
                }.into_any(),
                None => view! {
                    <span style="color:#666;">{hostname.clone()} " (no config matched)"</span>
                }.into_any(),
            }}
        </div>
    }
}

#[component]
fn ActivitySummary(
    block_count: u32,
    remove_count: u32,
    hide_count: u32,
    suggestion_count: u32,
) -> impl IntoView {
    let pill = |label: &'static str, n: u32, color: &'static str| {
        let bg = if n == 0 { "#f0f0f0" } else { color };
        let text = if n == 0 { "#999" } else { "#fff" };
        view! {
            <span style=format!(
                "display:inline-block; padding: 2px 8px; margin-right: 4px;
                 background: {bg}; color: {text}; border-radius: 10px;
                 font-size: 11px; font-weight: 600;"
            )>
                {label} ": " {n}
            </span>
        }
    };
    view! {
        <div class="rust-activity-summary"
             style="padding: 4px 0; margin: 6px 0;">
            {pill("block", block_count, "#d85c4f")}
            {pill("remove", remove_count, "#d89a4f")}
            {pill("hide", hide_count, "#6b8ad4")}
            {pill("suggestions", suggestion_count, "#e8a200")}
        </div>
    }
}

#[component]
fn SuggestionsList(
    suggestions: RwSignal<Vec<Suggestion>>,
    hostname: String,
    tab_id: Option<i32>,
) -> impl IntoView {
    // Refresh helper: re-fetch suggestions after any action.
    let refresh = {
        let suggestions = suggestions;
        move || {
            let Some(tab_id) = tab_id else { return };
            spawn_local(async move {
                match chrome_bridge::get_suggestions(tab_id).await {
                    Ok(next) => suggestions.set(next),
                    Err(e) => web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] get_suggestions failed: {:?}",
                        e
                    ))),
                }
            });
        }
    };

    view! {
        <div class="rust-suggestions"
             style="margin-top: 10px;">
            <h2 style="font-size: 12px; font-weight: 600; color: #555; margin: 0 0 6px;">
                "Suggestions"
            </h2>
            {move || {
                let list = suggestions.get();
                if list.is_empty() {
                    view! {
                        <div style="padding: 8px 10px; background: #fafafa;
                                    color: #999; font-size: 11px; font-style: italic;
                                    border-radius: 3px;">
                            "No suggestions for this tab yet."
                        </div>
                    }.into_any()
                } else {
                    let rows: Vec<_> = list.into_iter().map(|s| {
                        let refresh = refresh.clone();
                        view! {
                            <SuggestionRow
                                suggestion=s
                                hostname=hostname.clone()
                                tab_id=tab_id
                                on_mutated=refresh
                            />
                        }
                    }).collect();
                    view! { <ul style="list-style:none; padding:0; margin:0;">{rows}</ul> }.into_any()
                }
            }}
        </div>
    }
}

#[component]
fn SuggestionRow<F>(
    suggestion: Suggestion,
    hostname: String,
    tab_id: Option<i32>,
    on_mutated: F,
) -> impl IntoView
where
    F: Fn() + Clone + 'static,
{
    let layer_str = match suggestion.layer {
        SuggestionLayer::Block => "block",
        SuggestionLayer::Remove => "remove",
        SuggestionLayer::Hide => "hide",
    };
    let layer_color = match suggestion.layer {
        SuggestionLayer::Block => "#d85c4f",
        SuggestionLayer::Remove => "#d89a4f",
        SuggestionLayer::Hide => "#6b8ad4",
    };

    // Disable the row during an in-flight action so double-click
    // doesn't fire the same message twice.
    let busy = RwSignal::new(false);

    let add_action = {
        let hostname = hostname.clone();
        let layer = layer_str.to_string();
        let value = suggestion.value.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            busy.set(true);
            let hostname = hostname.clone();
            let layer = layer.clone();
            let value = value.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::accept_suggestion(&hostname, &layer, &value).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] accept_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let dismiss_action = {
        let key = suggestion.key.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            let Some(tab_id) = tab_id else { return };
            busy.set(true);
            let key = key.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::dismiss_suggestion(tab_id, &key).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] dismiss_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let allow_action = {
        let key = suggestion.key.clone();
        let on_mutated = on_mutated.clone();
        move |_| {
            if busy.get() {
                return;
            }
            busy.set(true);
            let key = key.clone();
            let on_mutated = on_mutated.clone();
            spawn_local(async move {
                if let Err(e) = chrome_bridge::allowlist_suggestion(&key).await {
                    web_sys::console::error_1(&JsValue::from_str(&format!(
                        "[Hush popup] allowlist_suggestion: {:?}",
                        e
                    )));
                }
                busy.set(false);
                on_mutated();
            });
        }
    };

    let value = suggestion.value.clone();
    let reason = suggestion.reason.clone();
    let learn = suggestion.learn.clone();
    let confidence = suggestion.confidence;
    let count = suggestion.count;
    let from_iframe = suggestion.from_iframe;
    let frame_host = suggestion.frame_hostname.clone();

    view! {
        <li class="rust-sugg-row"
            style="padding: 8px 10px; margin-bottom: 8px;
                   background: #fff; border: 1px solid #e0e0e0;
                   border-radius: 4px;">
            <div style="display:flex; align-items:center; gap: 6px;
                        font-size: 11px; margin-bottom: 4px;">
                <span style=format!(
                    "display:inline-block; padding: 1px 8px; background: {layer_color};
                     color: #fff; border-radius: 10px; font-weight: 600;"
                )>
                    {layer_str}
                </span>
                {if from_iframe {
                    if let Some(fh) = frame_host.clone() {
                        view! {
                            <span style="font-size: 10px; color: #888; font-style: italic;"
                                  title="Observation came from an iframe on this tab">
                                "from iframe " {fh}
                            </span>
                        }.into_any()
                    } else {
                        view! { <span /> }.into_any()
                    }
                } else { view! { <span /> }.into_any() }}
                <span style="margin-left:auto; color: #999;">
                    "conf " {confidence} "  |  count " {count}
                </span>
            </div>
            <div style="font-family: ui-monospace, monospace; font-size: 11px;
                        color: #333; word-break: break-all;">
                {value}
            </div>
            <div style="font-size: 11px; color: #666; margin: 3px 0;">
                {reason}
            </div>
            {if !learn.is_empty() {
                view! {
                    <div style="font-size: 11px; line-height: 1.5; color: #555;
                                background: #fafafa; border-left: 2px solid #c7d5e9;
                                padding: 6px 9px; margin: 6px 0 2px;
                                border-radius: 3px;">
                        {learn}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
            <div style="display:flex; gap: 6px; margin-top: 6px;">
                <button
                    disabled=move || busy.get()
                    on:click=add_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #2b7cff; color: #fff;
                           border: 1px solid #2b7cff; border-radius: 3px;
                           cursor: pointer;"
                    title="Write this suggestion into the matched site's config">
                    "+ Add"
                </button>
                <button
                    disabled=move || busy.get()
                    on:click=dismiss_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #999;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer;"
                    title="Hide for this tab session only">
                    "Dismiss"
                </button>
                <button
                    disabled=move || busy.get()
                    on:click=allow_action
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #2d8a3e;
                           border: 1px solid #b7d7bf; border-radius: 3px;
                           cursor: pointer;"
                    title="Permanently allow on every site. Revocable from Options.">
                    "Allow"
                </button>
            </div>
        </li>
    }
}
