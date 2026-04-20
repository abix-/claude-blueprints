//! Hush detection engine.
//!
//! Session A of the Rust migration (see `docs/rust-migration-plan.md`)
//! ports the following pure functions from `background.js`:
//!
//! - `LEARN_TEXT` - per-signal teaching strings shown in the popup.
//! - `buildSuggestion` - the single shape builder every detector goes
//!   through. Centralizes dedup-diag computation so the schema can't
//!   drift between detectors.
//! - Allowlist matching helpers: `is_legit_hidden_iframe`,
//!   `overlay_allowlisted`.
//! - URL canonicalization: `canonicalize_url`.
//! - `pattern_keyword` for DNR rule diagnostics.
//! - `script_origin_from_stack` for attributing observations to a script.
//!
//! Subsequent sessions port the detector aggregators (B), main-world
//! hooks (C), and popup/options UI (D/E). This crate is the single
//! ground truth for engine behavior; the JS shell calls into it via
//! wasm-bindgen.

#![forbid(unsafe_code)]

mod allowlist;
mod canon;
mod learn;
mod stack;
mod suggestion;

pub use allowlist::{is_legit_hidden_iframe, overlay_allowlisted};
pub use canon::{canonicalize_url, pattern_keyword};
pub use hush_types::{
    Allowlist, BuildSuggestionInput, Config, SiteConfig, Suggestion, SuggestionDiag,
    SuggestionLayer,
};
pub use learn::{learn_text, LearnKind};
pub use stack::script_origin_from_stack;
pub use suggestion::build_suggestion;

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
    serde_wasm_bindgen::to_value(&suggestion).map_err(|e| JsValue::from_str(&e.to_string()))
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
