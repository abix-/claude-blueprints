//! Main-world Rust runtime.
//!
//! Stage 3 of the Rust port (see `docs/roadmap.md`). The hybrid pattern:
//!
//! 1. `mainworld.js` runs synchronously at `document_start`. It patches
//!    every target prototype method with a small JS stub that:
//!      - captures `new Error().stack` (must happen in JS - WASM has no
//!        access to the JS call stack)
//!      - builds a `{kind, t, stack, ...fields}` object
//!      - hands it to `dispatch_hook` if WASM is loaded, else pushes it
//!        onto an in-page buffer queue
//!      - applies the original method with the original `this` + args
//! 2. In parallel, `mainworld.js` dynamically imports the wasm-bindgen
//!    glue and awaits `init()` + `initEngine()`.
//! 3. Once WASM is ready, `mainworld.js` flips a flag and invokes
//!    [`drain_stub_queue`] with the pending queue. Subsequent hook
//!    invocations go through [`dispatch_hook`] directly.
//!
//! Both entry points validate the incoming detail against the typed
//! [`SignalPayload`] variant for its kind. If any required field is
//! missing (the 0.5.0 bug class), serde rejects it and the event is
//! logged and dropped instead of silently reaching the detectors.
//!
//! ## Why JS still does the prototype re-patching
//!
//! The approved plan asked for Rust to re-patch prototypes via
//! `js_sys::Reflect::set` + `Closure`. wasm-bindgen closures don't
//! forward the implicit JS `this` binding, and `new Function()` (the
//! only CSP-compatible way to build a `this`-capturing function from
//! Rust without per-hook JS shims) requires `unsafe-eval`, which many
//! target sites block. JS therefore owns the physically-required
//! prototype assignment; Rust owns every other step after the stub
//! captures.

use crate::types::SignalPayload;
use js_sys::Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CustomEvent, CustomEventInit, Document};

/// Called by `mainworld.js` once WASM is ready. Walks the in-page
/// pre-WASM queue, validates each entry against [`SignalPayload`], and
/// dispatches a `__hush_call__` CustomEvent for each valid entry so
/// the isolated-world content script relay picks it up. Invalid
/// entries are logged via `console.error` and dropped.
#[wasm_bindgen(js_name = "drainStubQueue")]
pub fn drain_stub_queue(queue: JsValue) -> Result<(), JsValue> {
    let arr = queue
        .dyn_into::<Array>()
        .map_err(|_| JsValue::from_str("drainStubQueue: argument must be an array"))?;
    let len = arr.length();
    for i in 0..len {
        let entry = arr.get(i);
        if let Err(e) = validate_and_dispatch(entry) {
            log_error(&format!("stub queue entry invalid: {}", js_err_str(&e)));
        }
    }
    Ok(())
}

/// Called by each post-WASM hook stub on every invocation. Validates
/// and dispatches. Returns `()` on success, or a `JsValue` error with
/// a string message on validation failure - stubs are expected to
/// swallow errors so they never throw back into page JS.
#[wasm_bindgen(js_name = "dispatchHook")]
pub fn dispatch_hook(detail: JsValue) -> Result<(), JsValue> {
    validate_and_dispatch(detail)
}

/// Returns the canonical kind tag for a payload. Used by mainworld.js
/// tests / diagnostics; not part of the normal hook path.
#[wasm_bindgen(js_name = "signalKindOf")]
pub fn signal_kind_of(detail: JsValue) -> Result<String, JsValue> {
    let payload: SignalPayload = serde_wasm_bindgen::from_value(detail)
        .map_err(|e| JsValue::from_str(&format!("invalid payload: {e}")))?;
    Ok(kind_tag(&payload).to_string())
}

fn validate_and_dispatch(detail: JsValue) -> Result<(), JsValue> {
    let payload: SignalPayload = serde_wasm_bindgen::from_value(detail.clone())
        .map_err(|e| JsValue::from_str(&format!("SignalPayload deserialize: {e}")))?;
    let canonical = serde_wasm_bindgen::to_value(&payload)
        .map_err(|e| JsValue::from_str(&format!("SignalPayload re-serialize: {e}")))?;
    dispatch_custom_event(&canonical)?;
    Ok(())
}

fn dispatch_custom_event(detail: &JsValue) -> Result<(), JsValue> {
    let document = document_ref()?;
    let init = CustomEventInit::new();
    init.set_detail(detail);
    let event = CustomEvent::new_with_event_init_dict("__hush_call__", &init)?;
    document.dispatch_event(&event)?;
    Ok(())
}

fn document_ref() -> Result<Document, JsValue> {
    web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("no window.document available in main world"))
}

fn kind_tag(p: &SignalPayload) -> &'static str {
    match p {
        SignalPayload::Fetch { .. } => "fetch",
        SignalPayload::Xhr { .. } => "xhr",
        SignalPayload::Beacon { .. } => "beacon",
        SignalPayload::WsSend { .. } => "ws-send",
        SignalPayload::CanvasFp { .. } => "canvas-fp",
        SignalPayload::FontFp { .. } => "font-fp",
        SignalPayload::WebglFp { .. } => "webgl-fp",
        SignalPayload::AudioFp { .. } => "audio-fp",
        SignalPayload::ListenerAdded { .. } => "listener-added",
        SignalPayload::ReplayGlobal { .. } => "replay-global",
        SignalPayload::CanvasDraw { .. } => "canvas-draw",
    }
}

fn log_error(msg: &str) {
    web_sys::console::error_1(&JsValue::from_str(&format!("[Hush] {msg}")));
}

fn js_err_str(v: &JsValue) -> String {
    v.as_string()
        .unwrap_or_else(|| format!("{:?}", v))
}
