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
use crate::types::{BlockDiagnostic, BlockedUrl, Suggestion, SuggestionDiag, SuggestionLayer};
use js_sys::Reflect;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Fire-and-forget clipboard write. Logs the failure and moves on
/// instead of propagating the error back to the UI thread - the
/// button already shows "Copy failed" visual feedback through the
/// button-label swap in [`CopyButton`].
fn clipboard_write(text: &str) -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    // web_sys exposes Navigator::clipboard() behind a Clipboard feature.
    // Use Reflect to avoid panicking on browsers that don't expose it.
    let clipboard = Reflect::get(&navigator, &JsValue::from_str("clipboard"))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null());
    let Some(clipboard) = clipboard else {
        return false;
    };
    let write = match Reflect::get(&clipboard, &JsValue::from_str("writeText")) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let Ok(func) = write.dyn_into::<js_sys::Function>() else {
        return false;
    };
    func.call1(&clipboard, &JsValue::from_str(text)).is_ok()
}

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
    /// True when a site config matched this tab's hostname. Hides the
    /// per-section diagnostic panels when there's no config to
    /// interpret them against.
    #[serde(default)]
    pub is_matched: bool,
    /// Recent DNR-rule fires for this tab. Rendered as a by-pattern
    /// summary plus a collapsible URL list.
    #[serde(default)]
    pub blocked_urls: Vec<BlockedUrl>,
    /// Per-rule diagnostic rows for the Blocked section. Each entry
    /// represents one configured block rule with its fire count,
    /// status, and any broken-pattern hint.
    #[serde(default)]
    pub block_diagnostics: Vec<BlockDiagnostic>,
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
        <DetectorCta
            detector_enabled=snap.detector_enabled
            tab_id=tab_id
        />
        {if snap.is_matched {
            view! {
                <div class="rust-sections" style="padding: 4px 0;">
                    <BlockedSection
                        blocked_urls=snap.blocked_urls.clone()
                        diagnostics=snap.block_diagnostics.clone()
                        block_count=snap.block_count
                    />
                </div>
            }.into_any()
        } else { view! { <div /> }.into_any() }}
    }
}

/// Blocked (network) section. Replaces the `#block-count` / `#block-list`
/// / `#block-evidence` / `#block-diagnostics` renderers that previously
/// lived in `popup.js`. Groups recent blocked URLs by pattern, shows a
/// collapsible per-URL evidence list, and renders the per-rule
/// diagnostic panel with "firing" / "no traffic" / "pattern broken"
/// status badges plus a broken-pattern hint when present.
#[component]
fn BlockedSection(
    blocked_urls: Vec<BlockedUrl>,
    diagnostics: Vec<BlockDiagnostic>,
    block_count: u32,
) -> impl IntoView {
    let evidence_open = RwSignal::new(false);
    let copy_label = RwSignal::new("Copy");

    // Group URLs by pattern for the summary list.
    let mut by_pattern: Vec<(String, u32)> = Vec::new();
    {
        use std::collections::BTreeMap;
        let mut map: BTreeMap<String, u32> = BTreeMap::new();
        for b in &blocked_urls {
            let key = if b.pattern.is_empty() {
                "(unknown rule)".to_string()
            } else {
                b.pattern.clone()
            };
            *map.entry(key).or_insert(0) += 1;
        }
        for (k, v) in map {
            by_pattern.push((k, v));
        }
    }

    let has_patterns = !by_pattern.is_empty();
    let has_urls = !blocked_urls.is_empty();
    let has_diag = !diagnostics.is_empty();
    let blocked_len = blocked_urls.len();

    let count_class = if block_count == 0 { "count zero" } else { "count" };

    // Evidence copy payload: reverse order so newest first, join with newlines.
    let copy_payload: String = blocked_urls
        .iter()
        .rev()
        .map(|b| {
            format!(
                "{}\t[{}]\t{}\t(pattern: {})",
                time_only(&b.t),
                b.resource_type.clone().unwrap_or_else(|| "?".into()),
                b.url,
                if b.pattern.is_empty() { "?".into() } else { b.pattern.clone() },
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let on_copy = move |_| {
        let ok = clipboard_write(&copy_payload);
        copy_label.set(if ok { "Copied" } else { "Failed" });
        let label = copy_label;
        if let Some(window) = web_sys::window() {
            let cb = Closure::<dyn Fn()>::new(move || label.set("Copy"));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1500,
                );
            cb.forget();
        }
    };

    // Newest first for the visible evidence list too.
    let evidence_rows: Vec<_> = blocked_urls
        .iter()
        .rev()
        .map(|b| {
            let ts = time_only(&b.t);
            let rtype_suffix = b
                .resource_type
                .as_ref()
                .map(|t| format!(" [{}]", t))
                .unwrap_or_default();
            let title = b.url.clone();
            let body = format!("{}{}", b.url, rtype_suffix);
            view! {
                <li>
                    <span class="ts">{ts}</span>
                    <span title=title>{body}</span>
                </li>
            }
        })
        .collect();

    view! {
        <details class="section" open=true>
            <summary class="section-head">
                <span class="section-name">"Blocked (network)"</span>
                <span class=count_class>{block_count}</span>
            </summary>
            <ul>
                {if has_patterns {
                    by_pattern.into_iter().map(|(pattern, n)| {
                        let title = pattern.clone();
                        view! {
                            <li>
                                <span class="sel" title=title>{pattern}</span>
                                <span class="n">{n}</span>
                            </li>
                        }
                    }).collect::<Vec<_>>().into_any()
                } else {
                    let msg = if block_count > 0 {
                        "Blocked, but URL evidence not yet captured (try reloading)"
                    } else {
                        "No network blocks yet"
                    };
                    view! {
                        <div class="no-sels">{msg}</div>
                    }.into_any()
                }}
            </ul>
            {if has_urls {
                let toggle_text = move || {
                    let plural = if blocked_len == 1 { "" } else { "s" };
                    let verb = if evidence_open.get() { "Hide" } else { "Show" };
                    format!("{} {} blocked URL{}", verb, blocked_len, plural)
                };
                view! {
                    <div class="evidence">
                        <div style="display:flex;align-items:center;gap:8px;">
                            <span class="evidence-toggle"
                                  on:click=move |_| evidence_open.update(|v| *v = !*v)>
                                {toggle_text}
                            </span>
                            <button on:click=on_copy
                                    title="Copy evidence to clipboard"
                                    style="flex:0 0 auto;padding:2px 10px;font-size:10px;cursor:pointer;border:1px solid #ccc;background:#fff;border-radius:4px;">
                                {move || copy_label.get()}
                            </button>
                        </div>
                        {move || if evidence_open.get() {
                            view! {
                                <ul class="evidence-list">
                                    {evidence_rows.clone()}
                                </ul>
                            }.into_any()
                        } else { view! { <span /> }.into_any() }}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
            {if has_diag {
                let diag_len = diagnostics.len();
                let rows: Vec<_> = diagnostics.into_iter().map(|d| {
                    view! { <RuleRow diag=d /> }
                }).collect();
                view! {
                    <div class="diagnostics">
                        <div class="diagnostics-title">
                            "Block rules (" {diag_len} ")"
                        </div>
                        {rows}
                    </div>
                }.into_any()
            } else { view! { <div /> }.into_any() }}
        </details>
    }
}

/// One row in the Block-rules diagnostic panel. Rendered as a standalone
/// component so the conditional hint (broken-pattern vs. no-traffic) can
/// be expressed as straight view branches without cloning the whole
/// parent context.
#[component]
fn RuleRow(diag: BlockDiagnostic) -> impl IntoView {
    let status_label = match diag.status.as_str() {
        "firing" => "FIRING",
        "no-traffic" => "no traffic",
        "pattern-broken" => "PATTERN BROKEN?",
        other => {
            // Fallback: render whatever the backend sent. Lets future
            // status values land without a Rust change.
            return view! {
                <div class="rule-row">
                    <div class="rule-pattern" title=diag.pattern.clone()>
                        {diag.pattern.clone()}
                    </div>
                    <div class="rule-meta">
                        <span class="rule-fired">
                            "fired " {diag.fired} "x  |  declared under "
                            {if diag.source_domain.is_empty() {
                                "-".to_string()
                            } else { diag.source_domain.clone() }}
                        </span>
                        <span class=format!("rule-status {other}")>
                            {other.to_string()}
                        </span>
                    </div>
                </div>
            }.into_any();
        }
    };
    let status_class = format!("rule-status {}", diag.status);
    let source = if diag.source_domain.is_empty() {
        "-".to_string()
    } else {
        diag.source_domain.clone()
    };

    let hint = if diag.status == "pattern-broken" && !diag.matching_urls.is_empty() {
        let urls: Vec<_> = diag
            .matching_urls
            .iter()
            .map(|u| {
                let title = u.clone();
                view! { <div title=title>{u.clone()}</div> }
            })
            .collect();
        view! {
            <div class="rule-hint">
                <b>"Diagnosis: "</b>
                "this page requested URLs containing "
                <code>{diag.keyword.clone()}</code>
                " but the rule never fired. Your pattern probably doesn't match. "
                "Try a simpler form - e.g., drop wildcards, or use the distinctive "
                "substring anchored with "
                <code>"||domain"</code>
                "."
                <div class="urls">
                    <div style="margin-top:6px;color:#999">
                        "URLs that should have matched:"
                    </div>
                    {urls}
                </div>
            </div>
        }.into_any()
    } else if diag.status == "no-traffic" && diag.fired == 0 {
        view! {
            <div class="rule-hint"
                 style="background:#f0f0f0;color:#666;">
                <b>"No matching traffic yet. "</b>
                "Either the site hasn't requested this URL in the current "
                "session, or a DOM Remove rule is killing the element before "
                "it can fetch. Not necessarily a bug - scroll/reload the page "
                "to generate more traffic if you want to verify."
            </div>
        }.into_any()
    } else {
        view! { <div /> }.into_any()
    };

    view! {
        <div class="rule-row">
            <div class="rule-pattern" title=diag.pattern.clone()>
                {diag.pattern.clone()}
            </div>
            <div class="rule-meta">
                <span class="rule-fired">
                    "fired " {diag.fired} "x  |  declared under " {source}
                </span>
                <span class=status_class>{status_label}</span>
            </div>
            {hint}
        </div>
    }.into_any()
}

/// HH:MM:SS from an ISO timestamp. Uses the browser's JS Date so
/// locale + timezone match the rest of the UI. Returns an empty
/// string for unparseable input.
fn time_only(iso: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    let s = d.to_time_string();
    // Format is "HH:MM:SS GMT...". Slice to 8 chars for time only.
    s.as_string()
        .unwrap_or_default()
        .chars()
        .take(8)
        .collect()
}

/// Call-to-action row under the suggestions list. When the behavioral
/// detector is off, shows "Enable" + "Scan this tab now". When it's
/// on, shows "Rescan now". Both paths end by calling
/// [`refresh_popup_suggestions`] so new findings surface without a
/// full popup reload.
#[component]
fn DetectorCta(detector_enabled: bool, tab_id: Option<i32>) -> impl IntoView {
    let enabled = RwSignal::new(detector_enabled);
    let busy = RwSignal::new(false);

    let enable_click = move |_| {
        if busy.get() {
            return;
        }
        let Some(tab_id) = tab_id else {
            return;
        };
        busy.set(true);
        spawn_local(async move {
            if let Err(e) = chrome_bridge::enable_detector().await {
                web_sys::console::error_1(&JsValue::from_str(&format!(
                    "[Hush popup] enable_detector: {:?}",
                    e
                )));
            }
            let _ = chrome_bridge::scan_once(tab_id).await;
            // Refresh after content-script has had a moment to scan.
            set_timeout(300, move || {
                refresh_popup_suggestions();
                busy.set(false);
                enabled.set(true);
            });
        });
    };

    let scan_click = move |_| {
        if busy.get() {
            return;
        }
        let Some(tab_id) = tab_id else {
            return;
        };
        busy.set(true);
        spawn_local(async move {
            let _ = chrome_bridge::scan_once(tab_id).await;
            set_timeout(300, move || {
                refresh_popup_suggestions();
                busy.set(false);
            });
        });
    };

    view! {
        <div class="rust-detector-cta"
             style="margin-top: 12px; padding: 8px 10px;
                    background: #fffdf5; border: 1px solid #f0e4b0;
                    border-radius: 4px; font-size: 11px;">
            {move || if enabled.get() {
                view! {
                    <div style="display: flex; align-items: center; gap: 8px;">
                        <span style="color: #555; flex: 1;">
                            "Behavioral detector is on."
                        </span>
                        <button
                            disabled=move || busy.get()
                            on:click=scan_click
                            style="padding: 4px 10px; font-size: 11px;
                                   background: #fff; border: 1px solid #ccc;
                                   border-radius: 3px; cursor: pointer;">
                            "Rescan now"
                        </button>
                    </div>
                }.into_any()
            } else {
                view! {
                    <div>
                        <div style="color: #8a6500; margin-bottom: 6px;">
                            "Behavioral suggestions are off (zero CPU cost)."
                        </div>
                        <div style="display: flex; gap: 6px;">
                            <button
                                disabled=move || busy.get()
                                on:click=enable_click
                                style="padding: 4px 10px; font-size: 11px;
                                       background: #2b7cff; color: #fff;
                                       border: 1px solid #2b7cff;
                                       border-radius: 3px; cursor: pointer;">
                                "Enable"
                            </button>
                            <button
                                disabled=move || busy.get()
                                on:click=scan_click
                                style="padding: 4px 10px; font-size: 11px;
                                       background: #fff; border: 1px solid #ccc;
                                       border-radius: 3px; cursor: pointer;">
                                "Scan this tab now"
                            </button>
                        </div>
                    </div>
                }.into_any()
            }}
        </div>
    }
}

/// One-shot setTimeout wrapper. Allocates a Closure that the browser
/// fires once after `ms` milliseconds, then leaks it (it's a one-shot
/// callback so the lifetime is implicit in the timer's own bounded
/// duration). Any `.forget()` alternative ends up the same way.
fn set_timeout(ms: i32, f: impl FnOnce() + 'static) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let cell = std::cell::Cell::new(Some(f));
    let cb = Closure::<dyn Fn()>::new(move || {
        if let Some(f) = cell.take() {
            f();
        }
    });
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        ms,
    );
    cb.forget();
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
    let evidence = suggestion.evidence.clone();
    let diag = suggestion.diag.clone();

    // Why? / Evidence panels are independently collapsible per row.
    let why_open = RwSignal::new(false);
    let evidence_open = RwSignal::new(false);

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
                <button
                    on:click=move |_| why_open.update(|v| *v = !*v)
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #555;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer; flex: 0 0 auto;"
                    title="Why is this suggestion showing even if I have a rule?">
                    {move || if why_open.get() { "Hide why" } else { "Why?" }}
                </button>
                <button
                    on:click=move |_| evidence_open.update(|v| *v = !*v)
                    style="padding: 4px 10px; font-size: 11px;
                           background: #fff; color: #555;
                           border: 1px solid #ccc; border-radius: 3px;
                           cursor: pointer; flex: 0 0 auto;"
                    title="Raw observations that triggered this suggestion">
                    {move || if evidence_open.get() { "Hide evidence" } else { "Evidence" }}
                </button>
            </div>
            {move || if why_open.get() {
                view! { <WhyPanel diag=diag.clone() /> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
            {move || if evidence_open.get() {
                view! { <EvidencePanel evidence=evidence.clone() /> }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </li>
    }
}

/// Dedup diagnostic panel. Explains why a suggestion surfaced even
/// when the user thinks they have a matching rule - drawn from the
/// engine's `SuggestionDiag` attached to every suggestion.
#[component]
fn WhyPanel(diag: SuggestionDiag) -> impl IntoView {
    let layer_str = match diag.layer {
        SuggestionLayer::Block => "block",
        SuggestionLayer::Remove => "remove",
        SuggestionLayer::Hide => "hide",
    };
    let tab_host = if diag.tab_hostname.is_empty() {
        "(unknown)".to_string()
    } else {
        diag.tab_hostname.clone()
    };
    let frame_host = if diag.frame_hostname.is_empty() {
        tab_host.clone()
    } else {
        diag.frame_hostname.clone()
    };
    let is_iframe = if diag.is_from_iframe { "yes" } else { "no" };
    let matched_key = diag
        .matched_key
        .clone()
        .unwrap_or_else(|| "(no site config matched)".into());
    let rule_count = diag.existing_block_count.to_string();
    let sample_rows: Vec<_> = diag
        .existing_block_sample
        .iter()
        .map(|entry| {
            let line = format!("{} (len={})", entry, entry.len());
            view! {
                <li style="padding-left: 12px; font-family: ui-monospace, monospace;">
                    {line}
                </li>
            }
        })
        .collect();
    let has_sample = !diag.existing_block_sample.is_empty();
    let candidate_line = format!("{} (len={})", diag.value, diag.value.len());

    view! {
        <div style="margin-top: 6px; padding: 8px 10px;
                    background: #fafafa; border: 1px solid #eee;
                    border-radius: 3px; font-size: 11px;">
            <ul style="list-style:none; padding:0; margin:0;">
                <li><b>"Checked value: "</b> {diag.value.clone()}</li>
                <li><b>"Tab hostname (used for config match): "</b> {tab_host}</li>
                <li><b>"Observed from frame: "</b> {frame_host}</li>
                <li><b>"From iframe?: "</b> {is_iframe}</li>
                <li><b>"Matched config key: "</b> {matched_key}</li>
                <li><b>"Existing " {layer_str} " rules count: "</b> {rule_count}</li>
                <li><b>"Dedup result: "</b> {diag.dedup_result.clone()}</li>
                {if has_sample {
                    view! {
                        <li style="margin-top: 4px;"><b>"Existing rules sample (first 10):"</b></li>
                        {sample_rows}
                        <li style="margin-top: 4px;"><b>"Candidate value: "</b> {candidate_line}</li>
                    }.into_any()
                } else { view! { <li /> }.into_any() }}
            </ul>
        </div>
    }
}

/// Evidence panel: the raw observations the engine used to justify
/// the suggestion. Shows one line per entry with a Copy button that
/// writes all entries newline-joined to the clipboard.
#[component]
fn EvidencePanel(evidence: Vec<String>) -> impl IntoView {
    let copy_label = RwSignal::new("Copy");
    let joined = evidence.join("\n");
    let on_copy = move |_| {
        let ok = clipboard_write(&joined);
        copy_label.set(if ok { "Copied" } else { "Failed" });
        let label = copy_label;
        // Revert after 1.5s. Leptos has no built-in setTimeout wrapper
        // that's tiny; use gloo-less Window.setTimeout via web_sys.
        if let Some(window) = web_sys::window() {
            let cb = Closure::<dyn Fn()>::new(move || label.set("Copy"));
            let _ = window
                .set_timeout_with_callback_and_timeout_and_arguments_0(
                    cb.as_ref().unchecked_ref(),
                    1500,
                );
            cb.forget();
        }
    };
    let entries = evidence.clone();
    let empty = entries.is_empty();

    view! {
        <div style="margin-top: 6px; padding: 8px 10px;
                    background: #fafafa; border: 1px solid #eee;
                    border-radius: 3px; font-size: 11px;">
            <div style="display:flex; justify-content:flex-end; margin-bottom: 4px;">
                <button on:click=on_copy
                        disabled=move || empty
                        style="padding: 2px 10px; font-size: 10px;
                               border: 1px solid #ccc; background: #fff;
                               border-radius: 4px; cursor: pointer;"
                        title="Copy all evidence to clipboard">
                    {move || copy_label.get()}
                </button>
            </div>
            <ul style="list-style:none; padding:0; margin:0;">
                {if empty {
                    view! {
                        <li style="font-style: italic; color: #999;">
                            "(no captured evidence)"
                        </li>
                    }.into_any()
                } else {
                    entries.into_iter().map(|e| view! {
                        <li style="font-family: ui-monospace, monospace;
                                   word-break: break-all; margin-bottom: 2px;">
                            {e}
                        </li>
                    }).collect::<Vec<_>>().into_any()
                }}
            </ul>
        </div>
    }
}
