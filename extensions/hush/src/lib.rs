//! Hush detection engine.
//!
//! Single-crate layout matching the `endless` repo model. All modules
//! live flat under `src/`; the wasm-bindgen-exported entry points are
//! right here in `lib.rs`. For the stage-by-stage port plan see
//! `docs/roadmap.md`; for per-signal research notes see
//! `docs/heuristic-roadmap.md`.

#![forbid(unsafe_code)]

mod allowlist;
mod background;
mod canon;
mod chrome_bridge;
mod compute;
mod detectors;
mod learn;
mod main_world;
mod stack;
mod suggestion;
pub mod types;
mod ui_options;
mod ui_popup;

pub use allowlist::{is_legit_hidden_iframe, overlay_allowlisted};
pub use canon::{canonicalize_url, pattern_keyword};
pub use compute::compute_suggestions;
pub use learn::{learn_text, LearnKind};
pub use stack::script_origin_from_stack;
pub use suggestion::build_suggestion;
pub use types::{
    Allowlist, BehaviorState, BuildSuggestionInput, Config, IframeHit, JsCall, ReplayVendor,
    Resource, RuleEntry, SiteConfig, StickyHit, StickyRect, Suggestion, SuggestionDiag,
    SuggestionLayer,
};

use wasm_bindgen::prelude::*;

/// Called once from the JS bootstrap after `await init()`. Installs the
/// panic hook so Rust panics show up in DevTools with a proper stack
/// instead of a silent `unreachable executed` message.
#[wasm_bindgen(js_name = "initEngine")]
pub fn init_engine() {
    #[cfg(feature = "panic-hook")]
    console_error_panic_hook::set_once();
}

/// WASM-exported `buildSuggestion`. JS hands in a plain object; we parse
/// it via serde-wasm-bindgen, run the pure builder, and serialize the
/// result back. Any schema mismatch is a runtime error at this boundary
/// - not silent field loss like the 0.5.0 emit() bug.
#[wasm_bindgen(js_name = "buildSuggestion")]
pub fn build_suggestion_wasm(input: JsValue) -> Result<JsValue, JsValue> {
    let parsed: BuildSuggestionInput =
        serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let suggestion = build_suggestion(&parsed);
    crate::chrome_bridge::to_js(&suggestion).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// WASM-exported per-signal teaching text lookup. Accepts the signal
/// kind tag (e.g. "beacon", "canvas-fp") and returns the corresponding
/// learn paragraph, or empty string if the kind is unknown.
#[wasm_bindgen(js_name = "learnText")]
pub fn learn_text_wasm(kind: &str) -> String {
    learn_text(kind).unwrap_or("").to_string()
}

/// WASM-exported canonicalizer. Strips known-noise query parameters so
/// the polling-endpoint detector can cluster "same URL with different
/// timestamps" as one canonical URL.
#[wasm_bindgen(js_name = "canonicalizeUrl")]
pub fn canonicalize_url_wasm(url: &str) -> String {
    canonicalize_url(url)
}

/// WASM-exported DNR pattern-keyword extractor used by the popup's
/// "pattern broken?" diagnostic.
#[wasm_bindgen(js_name = "patternKeyword")]
pub fn pattern_keyword_wasm(pattern: &str) -> String {
    pattern_keyword(pattern).to_string()
}

/// WASM-exported iframe-allowlist membership check. Used by the hidden-
/// iframe detector to skip known-legit captcha/oauth/payment frames.
#[wasm_bindgen(js_name = "isLegitHiddenIframe")]
pub fn is_legit_hidden_iframe_wasm(src_url: &str, allowlist: JsValue) -> Result<bool, JsValue> {
    let list: Vec<String> =
        serde_wasm_bindgen::from_value(allowlist).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(is_legit_hidden_iframe(src_url, &list))
}

/// WASM-exported top-level `computeSuggestions`. The service worker
/// hands us the tab's behavior state, the full user config, and the
/// persisted allowlist; we return a fully-filtered, confidence-sorted
/// list of suggestions. Every detector path in the crate is invoked
/// internally; this is the single entry point the JS service worker
/// needs.
#[wasm_bindgen(js_name = "computeSuggestions")]
pub fn compute_suggestions_wasm(
    state: JsValue,
    config: JsValue,
    allowlist: JsValue,
) -> Result<JsValue, JsValue> {
    let state: BehaviorState =
        serde_wasm_bindgen::from_value(state).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let config: Config =
        serde_wasm_bindgen::from_value(config).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let allowlist: Allowlist =
        serde_wasm_bindgen::from_value(allowlist).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let suggestions = compute_suggestions(&state, &config, &allowlist);
    crate::chrome_bridge::to_js(&suggestions).map_err(|e| JsValue::from_str(&e.to_string()))
}
