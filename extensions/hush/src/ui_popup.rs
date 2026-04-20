//! Popup UI (Stage 4).
//!
//! Leptos CSR component tree for `popup.html`. `popup.js` queries
//! chrome.tabs + chrome.runtime for a snapshot of per-tab state, then
//! calls [`mount_popup`] with a single serializable object. Leptos
//! owns the matched-site header and the activity summary line; the
//! per-section JS renderers (suggestions, block diagnostics, removed
//! evidence) stay wired to their own `#block-list` / `#sugg-list`
//! roots and get ported in follow-up iterations.
//!
//! Interactivity is out of scope for this commit. Buttons and
//! click-driven state (Add / Dismiss / Allow) remain in popup.js
//! until the next Leptos pass can own them end-to-end.

use leptos::prelude::*;
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Snapshot popup.js hands in at mount time. Fields mirror the
/// existing JS flow: hostname + matched domain + top-level counts.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PopupSnapshot {
    pub hostname: String,
    pub matched_domain: Option<String>,
    pub block_count: u32,
    pub remove_count: u32,
    pub hide_count: u32,
    pub suggestion_count: u32,
}

/// WASM entry. Called by `popup.js` once after `init()` + `initEngine()`
/// resolve the snapshot. Deserializes the snapshot, resolves the mount
/// root, hands off to Leptos.
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

    // mount_to returns an UnmountHandle the caller should keep alive;
    // popup mounts for the tab-session lifetime so forget() is correct.
    std::mem::forget(leptos::mount::mount_to(root_el, move || {
        view! { <PopupRoot snap=snap.clone() /> }
    }));
    Ok(())
}

/// Top-level popup component. Renders the matched-site header (owning
/// the display that popup.js used to write into `#match`) plus a
/// one-line activity summary.
#[component]
fn PopupRoot(snap: PopupSnapshot) -> impl IntoView {
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
    }
}

/// Matched-site header. Shows:
/// - "Matched: <domain>" in bold when a site config matches the tab
/// - Plain hostname when there's no match
/// - "No active tab" when popup opened on a non-tab page
#[component]
fn MatchedSite(hostname: String, matched_domain: Option<String>) -> impl IntoView {
    let matched = matched_domain.clone();
    let show_suffix = match (&matched, hostname.is_empty()) {
        (Some(m), false) => m != &hostname,
        _ => false,
    };
    let hostname_for_suffix = hostname.clone();

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
                            view! { <span style="color:#999;"> " (" {hostname_for_suffix.clone()} ")" </span> }.into_any()
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

/// One-line activity summary pulled from the tab-stats snapshot.
/// Displays block / remove / hide / suggestion counts as a compact
/// row. The per-section details (URL lists, per-selector counts,
/// diagnostics) still render via popup.js below this.
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
