//! Popup UI (Stage 4 scaffold).
//!
//! Leptos CSR component tree for `popup.html`. The `mount_popup` wasm-
//! bindgen export is called by `popup.js` after `init()` + `initEngine()`
//! to mount the component into the page's `<div id="rust-popup-root">`.
//!
//! This is the initial scaffold: a matched-site header pulled from the
//! active tab plus a placeholder for the per-section content that will
//! move over from `popup.js` incrementally. Existing JS renderers
//! (blocked list, suggestions, etc.) stay wired up to their own
//! `#block-list`, `#sugg-list` root divs until each gets ported. The
//! Leptos root and the JS roots coexist inside `popup.html`.

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// WASM entry. `popup.js` calls this once the WASM bundle is ready.
/// Resolves the `#rust-popup-root` div and hands it to Leptos.
#[wasm_bindgen(js_name = "mountPopup")]
pub fn mount_popup() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    let document = window.document().ok_or_else(|| JsValue::from_str("no document"))?;
    let root = document
        .get_element_by_id("rust-popup-root")
        .ok_or_else(|| JsValue::from_str("no #rust-popup-root in popup.html"))?;
    let root_el: web_sys::HtmlElement = root
        .dyn_into::<web_sys::HtmlElement>()
        .map_err(|_| JsValue::from_str("#rust-popup-root is not an HtmlElement"))?;

    // `mount_to` returns an UnmountHandle the caller is expected to keep
    // alive. For the popup, the component should live for the tab
    // session, so we leak the handle intentionally via `forget`.
    std::mem::forget(leptos::mount::mount_to(root_el, || view! { <PopupHeader /> }));
    Ok(())
}

/// Top-of-popup header. Renders a small marker confirming the Rust
/// component tree mounted. Subsequent commits port over the matched-
/// site display, activity summary, block diagnostics, suggestions
/// list, and action row.
#[component]
fn PopupHeader() -> impl IntoView {
    view! {
        <div class="rust-popup-header"
             style="padding: 6px 10px; margin: 4px 0; font-size: 11px;
                    color: #2d8a3e; background: #f0f9f2;
                    border-left: 2px solid #2d8a3e; border-radius: 3px;">
            "Leptos runtime active. Stage 4 scaffold."
        </div>
    }
}
