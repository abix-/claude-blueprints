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
use std::sync::Arc;

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
    // `Arc<[String]>` so the detectors can fan out the same list to
    // every emitted suggestion as a refcount bump (2 instructions)
    // instead of a Vec data copy. Across a heavy_tab run with ~30
    // suggestions that's 90 saved allocations on the hot path.
    #[serde(rename = "existingBlock", default)]
    pub existing_block: Arc<[String]>,
    #[serde(rename = "existingRemove", default)]
    pub existing_remove: Arc<[String]>,
    #[serde(rename = "existingHide", default)]
    pub existing_hide: Arc<[String]>,
}

/// One blocked URL observation recorded by the service worker when a
/// DNR rule fires. Popup shows these grouped by pattern with a
/// collapsible per-URL list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockedUrl {
    #[serde(default)]
    pub t: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub pattern: String,
    #[serde(
        rename = "resourceType",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub resource_type: Option<String>,
}

/// Per-rule diagnostic entry from `computeRuleDiagnostics` in
/// background.js. Drives the popup's "Block rules" panel: each
/// configured block rule's fire count, status, and (when the rule
/// looks broken) a hint with URLs that matched the keyword but
/// didn't fire the rule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BlockDiagnostic {
    #[serde(default)]
    pub pattern: String,
    #[serde(rename = "sourceDomain", default)]
    pub source_domain: String,
    #[serde(default)]
    pub fired: u32,
    #[serde(default)]
    pub keyword: String,
    /// "firing", "no-traffic", or "pattern-broken".
    #[serde(default)]
    pub status: String,
    #[serde(rename = "matchingUrls", default)]
    pub matching_urls: Vec<String>,
}

/// One DOM-removed element observation recorded by `content.js` when a
/// Remove-layer selector matches and the node is detached. Popup shows
/// these in a collapsible evidence panel newest-first.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RemovedElement {
    #[serde(default)]
    pub t: String,
    /// The Remove-layer CSS selector that matched the node.
    #[serde(default)]
    pub selector: String,
    /// Short human-readable description of the element (tag + id/class).
    #[serde(default)]
    pub el: String,
    /// Scope key (site hostname or `__global__`) the selector was
    /// authored under. Content script tags each event with this so
    /// the firewall log attributes hits to the same row the rule
    /// enumeration shows. Empty when legacy events arrive without
    /// this field.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub scope: String,
}

/// Unified firewall-log event. Every rule hit (across all actions and
/// scopes) emits one of these into the per-tab ring buffer. The shape
/// is the same regardless of action so the popup's firewall-log view
/// can render them uniformly, sort by timestamp or hit count, and
/// filter by `action` / `scope` / `rule_id`.
///
/// `rule_id` is derived from the rule content as `action::scope::match`,
/// matching the existing suggestion-key format. This gives a stable
/// identifier across service-worker restarts and config exports,
/// without a separate ID-allocation migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirewallEvent {
    /// ISO-8601 timestamp of the match.
    pub t: String,
    /// Stable per-rule ID: `"{action}::{scope}::{match}"`.
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    /// `"block" | "remove" | "hide" | "spoof"`.
    pub action: String,
    /// The rule's scope key: [`GLOBAL_SCOPE_KEY`] or a hostname.
    pub scope: String,
    /// The rule's match string: URL pattern, CSS selector, or kind tag.
    #[serde(rename = "match")]
    pub match_: String,
    /// Action-specific supporting data.
    pub evidence: FirewallEvidence,
}

/// Action-specific fields attached to a [`FirewallEvent`]. Serialized
/// untagged so the JS side sees a flat object with only the fields
/// relevant to the event's action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FirewallEvidence {
    /// Block hit: full URL of the blocked request plus the DNR
    /// resource type (if the onRuleMatchedDebug event exposed it).
    Block {
        url: String,
        #[serde(rename = "resourceType", default, skip_serializing_if = "Option::is_none")]
        resource_type: Option<String>,
    },
    /// Remove hit: one entry per physically-removed DOM node. The
    /// `el` string is the output of the content script's
    /// `describeElement` (tag + id/class + distinguishing
    /// attributes + text snippet).
    Remove { el: String },
    /// No per-hit evidence. Used by `hide` (CSS rules don't fire
    /// per-element events) and `spoof` (we don't record individual
    /// fingerprint reads to keep the log focused on user-observable
    /// actions). Present as an empty variant so the shape stays
    /// uniform.
    #[serde(rename = "none")]
    None {},
}

/// Build the canonical `rule_id` for a (action, scope, match) triple.
/// Matches the suggestion-key format so downstream UIs can cross-
/// reference accepted suggestions with their resulting rule events.
pub fn rule_id(action: &str, scope: &str, match_: &str) -> String {
    format!("{action}::{scope}::{match_}")
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

/// A single rule row. Unifies the shape across Block/Allow/Remove/
/// Hide/Spoof so metadata (disabled, tags, comment) lives next to
/// the match string it applies to.
///
/// `rule_id` derivation still uses `(action, scope, value)` so
/// reorders, disable-toggles, and tag edits don't change identity
/// — the firewall log's aggregation survives those edits.
///
/// Serialization elides every field except `value` by default so
/// the on-disk JSON stays `{"value": "..."}` for a plain rule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleEntry {
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl RuleEntry {
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            ..Default::default()
        }
    }
}

impl From<&str> for RuleEntry {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RuleEntry {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// Per-site rules stored under a domain key in the user's config.
/// Every field is optional so the editor can represent partially-filled
/// entries without churning the schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SiteConfig {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hide: Vec<RuleEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove: Vec<RuleEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block: Vec<RuleEntry>,
    /// Counter-rules to Block/Remove/Hide. A matching `allow` rule
    /// overrides a broader Block (via DNR priority) or excludes
    /// matching nodes from Remove/Hide. Empty until Stage 9's
    /// evaluator lands; reserved here so the schema is stable.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<RuleEntry>,
    /// Fingerprint signals to neutralize for this site. Each entry
    /// is a kind tag; the main-world hook checks the tag and returns
    /// bland, identical-across-users values instead of the real ones.
    /// Currently supported: `webgl-unmasked` (WebGL UNMASKED_VENDOR
    /// and UNMASKED_RENDERER). Future kinds: `canvas`, `audio`, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spoof: Vec<RuleEntry>,
}

/// Top-level user config, keyed by domain. `IndexMap` preserves
/// insertion order so the options page's site list shows entries in the
/// order the user added them, matching the previous JS object iteration
/// semantics.
///
/// The reserved key [`GLOBAL_SCOPE_KEY`] (`"__global__"`) holds rules
/// that apply to every tab regardless of hostname. When a tab's
/// hostname matches a site entry, global rules layer underneath the
/// site-scoped rules (both sets apply; the site-scoped set takes
/// precedence only for duplicate values).
pub type Config = IndexMap<String, SiteConfig>;

/// Reserved Config key for rules that apply globally (every tab).
/// Normal hostnames can't start with an underscore, so this name is
/// safe against collisions with user-authored domains.
pub const GLOBAL_SCOPE_KEY: &str = "__global__";

/// Merge the global-scope entry (if present) with a site-scoped
/// entry into a new `SiteConfig` that callers can evaluate against
/// a tab. Duplicate values (same selector / pattern / spoof tag) are
/// deduplicated so a rule present in both scopes only fires once.
///
/// Order: global first, site on top. Stable order within each layer,
/// with site-scoped values winning positional order for duplicates.
pub fn merged_site_config(config: &Config, hostname_key: &str) -> SiteConfig {
    let global = config.get(GLOBAL_SCOPE_KEY);
    let site = if hostname_key == GLOBAL_SCOPE_KEY {
        None
    } else {
        config.get(hostname_key)
    };

    fn merge(a: &[RuleEntry], b: &[RuleEntry]) -> Vec<RuleEntry> {
        let mut out: Vec<RuleEntry> = a.to_vec();
        for e in b {
            if !out.iter().any(|x| x.value == e.value) {
                out.push(e.clone());
            }
        }
        out
    }

    let empty = SiteConfig::default();
    let g = global.unwrap_or(&empty);
    let s = site.unwrap_or(&empty);
    SiteConfig {
        hide: merge(&g.hide, &s.hide),
        remove: merge(&g.remove, &s.remove),
        block: merge(&g.block, &s.block),
        allow: merge(&g.allow, &s.allow),
        spoof: merge(&g.spoof, &s.spoof),
    }
}

/// One resource request observed via `PerformanceObserver`. Matches the
/// shape produced by `content.js`'s resource observer plus the
/// `reporterFrame` tag added by background when the message is received.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub host: String,
    #[serde(rename = "initiatorType", default)]
    pub initiator_type: String,
    #[serde(rename = "transferSize", default)]
    pub transfer_size: i64,
    #[serde(default)]
    pub duration: i64,
    #[serde(rename = "startTime", default)]
    pub start_time: i64,
    #[serde(rename = "reporterFrame", default, skip_serializing_if = "Option::is_none")]
    pub reporter_frame: Option<String>,
}

/// One hidden-iframe observation from `content.js`'s `scanHiddenIframes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IframeHit {
    #[serde(default)]
    pub src: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub width: i64,
    #[serde(default)]
    pub height: i64,
    #[serde(rename = "outerHTMLPreview", default)]
    pub outer_html_preview: String,
    #[serde(rename = "reporterFrame", default, skip_serializing_if = "Option::is_none")]
    pub reporter_frame: Option<String>,
}

/// Sticky/fixed-position overlay observation from
/// `content.js`'s `scanStickyOverlays`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickyHit {
    #[serde(default)]
    pub selector: String,
    #[serde(default)]
    pub coverage: u32,
    #[serde(rename = "zIndex", default)]
    pub z_index: i64,
    #[serde(default)]
    pub rect: StickyRect,
    #[serde(rename = "reporterFrame", default, skip_serializing_if = "Option::is_none")]
    pub reporter_frame: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StickyRect {
    #[serde(default)]
    pub w: i64,
    #[serde(default)]
    pub h: i64,
}

/// One main-world hook observation. Shaped as a flat struct with all
/// kind-specific fields optional; `kind` is the discriminator.
///
/// This mirrors the flattened JS object that crossed the isolated/main
/// world boundary. A discriminated-union form would be cleaner but
/// serde-wasm-bindgen's default tag/variant handling is noisier at
/// the FFI boundary; flat + optional keeps the JS side untouched.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JsCall {
    pub kind: String,
    #[serde(default)]
    pub t: String,
    #[serde(default)]
    pub stack: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(rename = "bodyPreview", default, skip_serializing_if = "Option::is_none")]
    pub body_preview: Option<String>,
    // Tier 1 fingerprinting fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
    #[serde(rename = "hotParam", default)]
    pub hot_param: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    // Tier 2 session-replay fields
    #[serde(rename = "eventType", default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vendors: Vec<ReplayVendor>,
    // Tier 5 raf-waste fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(rename = "canvasSel", default, skip_serializing_if = "Option::is_none")]
    pub canvas_sel: Option<String>,
    #[serde(rename = "reporterFrame", default, skip_serializing_if = "Option::is_none")]
    pub reporter_frame: Option<String>,
}

/// Session-replay vendor hit reported from a periodic poll of known
/// sentinel globals.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReplayVendor {
    #[serde(default)]
    pub key: String,
    pub vendor: String,
}

/// Per-tab behavioral state that the engine inspects when computing
/// suggestions. Populated by the service worker from `content.js`
/// messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BehaviorState {
    #[serde(rename = "pageHost", default, skip_serializing_if = "Option::is_none")]
    pub page_host: Option<String>,
    #[serde(rename = "seenResources", default)]
    pub seen_resources: Vec<Resource>,
    #[serde(rename = "latestIframes", default)]
    pub latest_iframes: Vec<IframeHit>,
    #[serde(rename = "latestStickies", default)]
    pub latest_stickies: Vec<StickyHit>,
    #[serde(rename = "jsCalls", default)]
    pub js_calls: Vec<JsCall>,
    #[serde(default)]
    pub dismissed: Vec<String>,
    #[serde(default)]
    pub suggestions: Vec<Suggestion>,
}

/// Typed payload for a single main-world hook observation.
///
/// Every hook that fires in `mainworld.js` produces one of these. The
/// discriminant is the stringly-tagged `kind` field so JS emitters can
/// construct `{kind: "canvas-fp", method: "toDataURL", stack: [...]}`
/// and serde deserializes directly into the right variant.
///
/// This is the *validation* type at the main/isolated world boundary.
/// If an emitter forgets a required field for its variant, serde fails
/// loudly at the wasm-bindgen boundary - not silently like the 0.5.0
/// cherry-picked-fields emit() bug.
///
/// The flat [`JsCall`] struct above stays for internal detector use
/// (stored state tolerates missing fields for forward-compatibility).
/// `SignalPayload` is the schema the IO boundary enforces; `JsCall` is
/// the schema the detector math reads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind")]
pub enum SignalPayload {
    #[serde(rename = "fetch")]
    Fetch {
        url: String,
        method: String,
        #[serde(rename = "bodyPreview", default)]
        body_preview: Option<String>,
        stack: Vec<String>,
    },
    #[serde(rename = "xhr")]
    Xhr {
        url: String,
        method: String,
        #[serde(rename = "bodyPreview", default)]
        body_preview: Option<String>,
        stack: Vec<String>,
    },
    #[serde(rename = "beacon")]
    Beacon {
        url: String,
        #[serde(default = "default_beacon_method")]
        method: String,
        #[serde(rename = "bodyPreview", default)]
        body_preview: Option<String>,
        stack: Vec<String>,
    },
    #[serde(rename = "ws-send")]
    WsSend {
        url: String,
        #[serde(default = "default_ws_method")]
        method: String,
        #[serde(rename = "bodyPreview", default)]
        body_preview: Option<String>,
        stack: Vec<String>,
    },
    #[serde(rename = "canvas-fp")]
    CanvasFp {
        method: String,
        stack: Vec<String>,
    },
    #[serde(rename = "font-fp")]
    FontFp {
        font: String,
        text: String,
        stack: Vec<String>,
    },
    #[serde(rename = "webgl-fp")]
    WebglFp {
        param: String,
        #[serde(rename = "hotParam")]
        hot_param: bool,
        stack: Vec<String>,
    },
    #[serde(rename = "audio-fp")]
    AudioFp {
        method: String,
        stack: Vec<String>,
    },
    #[serde(rename = "listener-added")]
    ListenerAdded {
        #[serde(rename = "eventType")]
        event_type: String,
        stack: Vec<String>,
    },
    #[serde(rename = "replay-global")]
    ReplayGlobal {
        vendors: Vec<ReplayVendor>,
    },
    #[serde(rename = "canvas-draw")]
    CanvasDraw {
        op: String,
        visible: bool,
        #[serde(rename = "canvasSel")]
        canvas_sel: String,
        stack: Vec<String>,
    },
}

fn default_beacon_method() -> String {
    "POST".to_string()
}
fn default_ws_method() -> String {
    "WS".to_string()
}

#[cfg(test)]
mod signal_payload_tests {
    use super::*;

    // Each variant's JSON shape must survive a serde round-trip so the
    // emit() boundary can't silently drop fields. This is the 0.5.0
    // regression caught at compile time + test time.

    fn roundtrip(input: &str) -> SignalPayload {
        let parsed: SignalPayload = serde_json::from_str(input).expect("valid SignalPayload");
        let rendered = serde_json::to_string(&parsed).expect("serializable");
        let reparsed: SignalPayload = serde_json::from_str(&rendered).expect("re-parseable");
        assert_eq!(parsed, reparsed);
        parsed
    }

    #[test]
    fn fetch_variant() {
        let s = r#"{"kind":"fetch","url":"https://x/","method":"GET","bodyPreview":null,"stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::Fetch { .. }));
    }

    #[test]
    fn xhr_variant() {
        let s = r#"{"kind":"xhr","url":"https://x/","method":"POST","bodyPreview":"body","stack":["a"]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::Xhr { .. }));
    }

    #[test]
    fn beacon_variant_defaults_method_to_post() {
        let s = r#"{"kind":"beacon","url":"https://x/","stack":[]}"#;
        match roundtrip(s) {
            SignalPayload::Beacon { method, .. } => assert_eq!(method, "POST"),
            _ => panic!("expected Beacon"),
        }
    }

    #[test]
    fn ws_send_variant() {
        let s = r#"{"kind":"ws-send","url":"wss://x/","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::WsSend { .. }));
    }

    #[test]
    fn canvas_fp_variant_requires_method_and_stack() {
        let s = r#"{"kind":"canvas-fp","method":"toDataURL","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::CanvasFp { .. }));
        // Missing method must fail to parse. This locks the 0.5.0 bug class.
        let missing = r#"{"kind":"canvas-fp","stack":[]}"#;
        assert!(serde_json::from_str::<SignalPayload>(missing).is_err());
    }

    #[test]
    fn font_fp_variant_requires_font_and_text() {
        let s = r#"{"kind":"font-fp","font":"Arial","text":"probe","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::FontFp { .. }));
        let missing = r#"{"kind":"font-fp","font":"Arial","stack":[]}"#;
        assert!(serde_json::from_str::<SignalPayload>(missing).is_err());
    }

    #[test]
    fn webgl_fp_variant_requires_hot_param() {
        let s = r#"{"kind":"webgl-fp","param":"37446","hotParam":true,"stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::WebglFp { hot_param: true, .. }));
        let missing = r#"{"kind":"webgl-fp","param":"37446","stack":[]}"#;
        assert!(serde_json::from_str::<SignalPayload>(missing).is_err(),
            "hotParam is required; omission must fail");
    }

    #[test]
    fn audio_fp_variant() {
        let s = r#"{"kind":"audio-fp","method":"OfflineAudioContext","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::AudioFp { .. }));
    }

    #[test]
    fn listener_added_variant_requires_event_type() {
        let s = r#"{"kind":"listener-added","eventType":"mousemove","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::ListenerAdded { .. }));
        let missing = r#"{"kind":"listener-added","stack":[]}"#;
        assert!(serde_json::from_str::<SignalPayload>(missing).is_err());
    }

    #[test]
    fn replay_global_variant_requires_vendors() {
        let s = r#"{"kind":"replay-global","vendors":[{"key":"_hjSettings","vendor":"Hotjar"}]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::ReplayGlobal { .. }));
    }

    #[test]
    fn canvas_draw_variant_requires_visible_flag() {
        let s = r#"{"kind":"canvas-draw","op":"fillRect","visible":false,"canvasSel":"canvas#x","stack":[]}"#;
        assert!(matches!(roundtrip(s), SignalPayload::CanvasDraw { visible: false, .. }));
        let missing = r#"{"kind":"canvas-draw","op":"fillRect","canvasSel":"canvas#x","stack":[]}"#;
        assert!(serde_json::from_str::<SignalPayload>(missing).is_err(),
            "visible is required; omission must fail (0.5.0 bug class)");
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let s = r#"{"kind":"definitely-not-a-real-kind","x":1}"#;
        assert!(serde_json::from_str::<SignalPayload>(s).is_err());
    }
}

#[cfg(test)]
mod rule_entry_tests {
    use super::*;

    #[test]
    fn plain_entry_serializes_to_value_only() {
        let e = RuleEntry::new("||ads.example.com");
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"value":"||ads.example.com"}"#);
    }

    #[test]
    fn full_entry_round_trips() {
        let e = RuleEntry {
            value: "||ads.example.com".into(),
            disabled: true,
            tags: vec!["analytics".into(), "auto:pixel".into()],
            comment: Some("temporarily muted for debugging".into()),
        };
        let json = serde_json::to_string(&e).unwrap();
        let back: RuleEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(back.value, e.value);
        assert_eq!(back.disabled, e.disabled);
        assert_eq!(back.tags, e.tags);
        assert_eq!(back.comment, e.comment);
    }

    #[test]
    fn bare_string_form_is_rejected() {
        // Hard migration: the Rust side refuses legacy string entries.
        // background.js runs migration before wasm init so Rust never
        // sees the old shape on disk.
        let err = serde_json::from_str::<RuleEntry>(r#""||ads.example.com""#);
        assert!(err.is_err(), "bare string must fail to parse");
    }

    #[test]
    fn site_config_preserves_rule_metadata_round_trip() {
        let cfg = SiteConfig {
            block: vec![
                RuleEntry {
                    value: "||a.test".into(),
                    disabled: true,
                    ..Default::default()
                },
                RuleEntry::new("||b.test"),
            ],
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let back: SiteConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.block.len(), 2);
        assert!(back.block[0].disabled);
        assert!(!back.block[1].disabled);
        assert_eq!(back.block[0].value, "||a.test");
    }

    #[test]
    fn merged_config_dedupes_by_value_not_full_entry() {
        // A global rule and a site rule with the same value but
        // different metadata should still dedup to one entry.
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec![RuleEntry::new("||t.test")],
                ..Default::default()
            },
        );
        cfg.insert(
            "site.test".into(),
            SiteConfig {
                block: vec![RuleEntry {
                    value: "||t.test".into(),
                    comment: Some("site-specific note".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let merged = merged_site_config(&cfg, "site.test");
        assert_eq!(merged.block.len(), 1, "duplicate value dedups across scopes");
    }
}
