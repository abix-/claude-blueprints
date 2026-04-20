//! Shared types for the Hush extension.
//!
//! These types define the schema at every JS/Rust boundary: suggestion
//! objects flowing from the engine back to the popup, the allowlist
//! shape in chrome.storage, per-site config, and the main-world signal
//! payloads that cross from the hooked page context to the service worker.
//!
//! A single authoritative definition here is the whole point of the Rust
//! port: schema drift across these boundaries is what produced the 0.5.0
//! emit() bug. With serde + derived type contracts, drift becomes a
//! compile error.

#![forbid(unsafe_code)]

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Which of the three layers a suggestion targets. Matches the JS runtime
/// `layer` string field exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionLayer {
    Block,
    Remove,
    Hide,
}

/// A suggestion surfaced in the popup. Every detector path in the engine
/// emits this shape so the popup's renderer can stay data-driven.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub key: String,
    pub layer: SuggestionLayer,
    pub value: String,
    pub reason: String,
    pub confidence: u8,
    pub count: u32,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default, rename = "fromIframe")]
    pub from_iframe: bool,
    #[serde(rename = "frameHostname", skip_serializing_if = "Option::is_none")]
    pub frame_hostname: Option<String>,
    pub diag: SuggestionDiag,
    #[serde(default)]
    pub learn: String,
}

/// Dedup diagnostic attached to every suggestion so the popup's "Why?"
/// panel can explain why the suggestion surfaced even when the user
/// believes they have a matching rule. Mirrors the shape of the prior
/// JS `makeDiag()` helper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionDiag {
    pub value: String,
    pub layer: SuggestionLayer,
    #[serde(rename = "tabHostname")]
    pub tab_hostname: String,
    #[serde(rename = "frameHostname")]
    pub frame_hostname: String,
    #[serde(rename = "isFromIframe")]
    pub is_from_iframe: bool,
    #[serde(rename = "matchedKey")]
    pub matched_key: Option<String>,
    #[serde(rename = "configHasSite")]
    pub config_has_site: bool,
    #[serde(rename = "existingBlockCount")]
    pub existing_block_count: usize,
    #[serde(rename = "existingBlockSample")]
    pub existing_block_sample: Vec<String>,
    #[serde(rename = "dedupResult")]
    pub dedup_result: String,
}

/// Input parameters to [`build_suggestion`]. The engine's single
/// suggestion-construction helper. Centralizes the schema so new fields
/// propagate from one place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSuggestionInput {
    pub key: String,
    pub layer: SuggestionLayer,
    pub value: String,
    pub reason: String,
    pub confidence: u8,
    pub count: u32,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(rename = "fromFrame", default)]
    pub from_frame: Option<String>,
    #[serde(default)]
    pub learn: String,
    // Context needed to compute the dedup diag. The JS call site used to
    // capture these by closure; we make them explicit parameters so the
    // function is pure.
    #[serde(rename = "tabHostname", default)]
    pub tab_hostname: String,
    #[serde(rename = "matchedKey", default)]
    pub matched_key: Option<String>,
    #[serde(rename = "configHasSite", default)]
    pub config_has_site: bool,
    #[serde(rename = "existingBlock", default)]
    pub existing_block: Vec<String>,
    #[serde(rename = "existingRemove", default)]
    pub existing_remove: Vec<String>,
    #[serde(rename = "existingHide", default)]
    pub existing_hide: Vec<String>,
}

/// Persistent allowlist in `chrome.storage.local`. All three lists are
/// independent user-editable arrays. `suggestions` is the per-key
/// cross-session allowlist populated by the popup's "Allow" button;
/// `iframes` and `overlays` are URL-substring and CSS-selector arrays
/// consumed at detection time.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Allowlist {
    #[serde(default)]
    pub iframes: Vec<String>,
    #[serde(default)]
    pub overlays: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<String>,
}

/// Per-site rules stored under a domain key in the user's config.
/// Every field is optional so the editor can represent partially-filled
/// entries without churning the schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hide: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block: Vec<String>,
}

/// Top-level user config, keyed by domain. `IndexMap` preserves
/// insertion order so the options page's site list shows entries in the
/// order the user added them, matching the previous JS object iteration
/// semantics.
pub type Config = IndexMap<String, SiteConfig>;
