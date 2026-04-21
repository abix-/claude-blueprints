//! Detector aggregators.
//!
//! Each `detect_*` function takes a reference to the current
//! [`BehaviorState`] and returns freshly-built suggestions. Dedup
//! against the user's existing site config is handled by the orchestrator
//! in [`crate::compute`]; detectors are not responsible for that.
//!
//! Every function below is a port of the corresponding JS block in the
//! old `background.js` `computeSuggestions`. Parity with the JS behavior
//! is covered by unit tests here and (eventually) a JS-Rust golden
//! snapshot test in Session B's follow-up.

// `foldhash::HashMap` is the std `HashMap<K, V, RandomState>` with
// foldhash swapped in. Beats SipHash / FxHash / aHash on small-string
// keys (all our per-host aggregation is hostname-keyed).
use foldhash::fast::RandomState as FoldHashState;
use std::collections::HashMap;
use std::sync::Arc;
type FastMap<K, V> = HashMap<K, V, FoldHashState>;

/// Default capacity for every per-detector aggregation HashMap. Sized
/// so the typical host-count / origin-count / signal-kind-count fits
/// without a grow operation (heavy_tab caps at ~10 unique hosts per
/// stream; 16 gives one doubling of headroom). Bump here, not per
/// call site.
const AGG_MAP_CAPACITY: usize = 16;

#[inline]
fn agg_map<K, V>() -> FastMap<K, V> {
    FastMap::with_capacity_and_hasher(AGG_MAP_CAPACITY, FoldHashState::default())
}

/// DNR block-pattern string for a host. Every block-layer detector
/// builds this shape; centralized so the pattern syntax can evolve
/// in one place (e.g. adding `^` anchors, wildcards, etc.).
#[inline]
fn block_value(host: &str) -> String {
    format!("||{host}")
}

/// Dedup key for a block suggestion. Mirrors what the popup uses when
/// filtering already-applied rules.
#[inline]
fn block_key(value: &str) -> String {
    format!("block::{value}")
}

/// English plural suffix for a count. Avoids
/// `if n > 1 { "s" } else { "" }` noise at every reason-string call
/// site.
#[inline]
const fn plural(n: usize) -> &'static str {
    if n > 1 { "s" } else { "" }
}

use crate::types::{
    Allowlist, BehaviorState, BuildSuggestionInput, IframeHit, JsCall, Resource, StickyHit,
    Suggestion, SuggestionLayer,
};

/// Uniform interface for every detection pass. Each implementor pulls
/// the slice of `BehaviorState` (and optionally `Allowlist`) it cares
/// about and returns freshly-built suggestions.
///
/// The trait lets [`crate::compute::compute_suggestions`] iterate over
/// a static [`DETECTORS`] list instead of calling each detector by
/// name. Adding a new detector = add a ZST + `impl Detector` block +
/// one entry in `DETECTORS`. No orchestrator edits needed.
///
/// `dyn Detector` adds one pointer indirection per call. With 7
/// detectors per `compute_suggestions` that's sub-nanosecond overhead
/// and well inside the noise floor; worth it for the extensibility.
pub(crate) trait Detector: Sync {
    fn detect(
        &self,
        ctx: &DetectCtx,
        state: &BehaviorState,
        allowlist: &Allowlist,
    ) -> Vec<Suggestion>;
}

pub(crate) struct BeaconDetector;
pub(crate) struct PixelDetector;
pub(crate) struct FirstPartyTelemetryDetector;
pub(crate) struct PollingDetector;
pub(crate) struct HiddenIframeDetector;
pub(crate) struct JsCallsDetector;
pub(crate) struct StickyOverlayDetector;

impl Detector for BeaconDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_beacon(ctx, &state.seen_resources)
    }
}
impl Detector for PixelDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_pixels(ctx, &state.seen_resources)
    }
}
impl Detector for FirstPartyTelemetryDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_first_party_telemetry(ctx, &state.seen_resources)
    }
}
impl Detector for PollingDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_polling(ctx, &state.seen_resources)
    }
}
impl Detector for HiddenIframeDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, a: &Allowlist) -> Vec<Suggestion> {
        detect_hidden_iframes(ctx, &state.latest_iframes, &a.iframes)
    }
}
impl Detector for JsCallsDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_from_js_calls(ctx, &state.js_calls)
    }
}
impl Detector for StickyOverlayDetector {
    fn detect(&self, ctx: &DetectCtx, state: &BehaviorState, _: &Allowlist) -> Vec<Suggestion> {
        detect_sticky_overlays(ctx, &state.latest_stickies)
    }
}

/// Ordered list of detectors run by [`crate::compute::compute_suggestions`].
/// Order here does not affect correctness - suggestions are
/// re-sorted by confidence+count at the end - but matches the
/// historical JS order for readability when comparing output.
pub(crate) static DETECTORS: &[&(dyn Detector + Sync)] = &[
    &BeaconDetector,
    &PixelDetector,
    &FirstPartyTelemetryDetector,
    &PollingDetector,
    &HiddenIframeDetector,
    &JsCallsDetector,
    &StickyOverlayDetector,
];

use crate::allowlist::is_legit_hidden_iframe;
use crate::canon::canonicalize_url;
use crate::learn::LearnKind;
use crate::stack::script_origin_from_stack;
use crate::suggestion::build_suggestion;

/// Map a detector signal kind to the spoof kind that neutralizes the
/// same underlying observation, if any. Used by `DetectCtx::is_covered`
/// to suppress block-origin suggestions when the user has already
/// picked the spoof lane for the corresponding fingerprint signal.
///
/// Only the fingerprint-vs-spoof relationships live here —
/// URL-layer equivalences (block covers neuter/silence) are structural
/// and handled directly in `is_covered` without a map.
pub(crate) fn spoof_kind_for_signal(signal_kind: &str) -> Option<&'static str> {
    match signal_kind {
        "webgl-fp-hot" => Some("webgl-unmasked"),
        "canvas-fp" => Some("canvas"),
        "audio-fp" => Some("audio"),
        "font-fp" => Some("font-enum"),
        _ => None,
    }
}

/// Opaque per-detect context the orchestrator fills in once and passes
/// to each detector. Bundling this keeps every detector signature narrow.
pub(crate) struct DetectCtx<'a> {
    pub hostname: &'a str,
    pub matched_key: Option<&'a str>,
    pub config_has_site: bool,
    // `Arc<[String]>` so fanout to every emitted suggestion is a
    // refcount bump, not a Vec data copy. compute.rs allocates the
    // Arcs once per compute_suggestions call; every detector push
    // clones cheaply from there.
    pub existing_block: Arc<[String]>,
    pub existing_remove: Arc<[String]>,
    pub existing_hide: Arc<[String]>,
    /// Enabled neuter rules from the merged config. Lets the
    /// replay-listener detector skip its Neuter suggestion when
    /// the user already has one for the same origin.
    pub existing_neuter: Arc<[String]>,
    /// Enabled silence rules from the merged config. Symmetric to
    /// existing_neuter for the silence-suggestion path (not yet
    /// emitted by any detector, but plumbed so future ones can).
    pub existing_silence: Arc<[String]>,
    /// Enabled spoof kind tags from the merged config. Lets
    /// fingerprint detectors skip their block suggestions when the
    /// equivalent spoof already neutralizes the signal — e.g. a
    /// user with `spoof: ["webgl-unmasked"]` doesn't need another
    /// "block this origin" nag for every WebGL-hot read.
    pub existing_spoof: Arc<[String]>,
}

impl<'a> DetectCtx<'a> {
    fn has_block(&self, v: &str) -> bool {
        self.existing_block.iter().any(|e| e == v)
    }
    fn has_remove(&self, v: &str) -> bool {
        self.existing_remove.iter().any(|e| e == v)
    }
    fn has_hide(&self, v: &str) -> bool {
        self.existing_hide.iter().any(|e| e == v)
    }
    fn has_neuter(&self, v: &str) -> bool {
        self.existing_neuter.iter().any(|e| e == v)
    }
    fn has_silence(&self, v: &str) -> bool {
        self.existing_silence.iter().any(|e| e == v)
    }
    fn has_spoof(&self, kind: &str) -> bool {
        self.existing_spoof.iter().any(|e| e == kind)
    }

    /// Central cross-layer dedup predicate. Returns true if an
    /// enabled rule equivalent to the (layer, value, kind) triple
    /// already covers the signal the caller is about to emit as a
    /// suggestion. One source of truth — adding a new equivalence
    /// (e.g. "block covers remove-of-iframe") is a match-arm edit
    /// here instead of scattered per-detector ad-hoc checks.
    ///
    /// Rules:
    /// - Same-layer same-value always dedups (trivial).
    /// - Block covers Neuter and Silence: a URL-blocked script
    ///   never runs, so there are no listeners to neuter and no
    ///   calls to silence.
    /// - Spoof covers the equivalent fingerprint-signal block
    ///   suggestion (webgl-fp-hot → `webgl-unmasked` spoof, etc.)
    ///   via [`spoof_kind_for_signal`].
    /// - Remove / Hide cross-layer: not currently — those are DOM
    ///   selectors, not URL filters; no dedup relationship to
    ///   network-layer actions.
    fn is_covered(&self, layer: SuggestionLayer, value: &str, kind: &str) -> bool {
        let same_layer = match layer {
            SuggestionLayer::Block => self.has_block(value),
            SuggestionLayer::Remove => self.has_remove(value),
            SuggestionLayer::Hide => self.has_hide(value),
            SuggestionLayer::Neuter => self.has_neuter(value),
            SuggestionLayer::Silence => self.has_silence(value),
        };
        if same_layer {
            return true;
        }
        // Cross-layer: block subsumes neuter/silence (script can't run).
        if matches!(layer, SuggestionLayer::Neuter | SuggestionLayer::Silence)
            && self.has_block(value)
        {
            return true;
        }
        // Cross-layer: matching spoof neutralizes the fingerprint
        // signal the block suggestion would otherwise target. Only
        // applies to Block suggestions — spoof has no bearing on
        // DOM-selector (remove/hide) rules even if the detector
        // kind coincidentally lines up.
        if layer == SuggestionLayer::Block {
            if let Some(spoof_kind) = spoof_kind_for_signal(kind) {
                if self.has_spoof(spoof_kind) {
                    return true;
                }
            }
        }
        false
    }

    /// Build a suggestion if the triple isn't already covered.
    /// Detectors wrap pushes in `if let Some(s) = ctx.try_finish(...)`
    /// instead of re-implementing dedup at every call site. Single
    /// source of truth — adding a new equivalence rule is an edit
    /// in [`Self::is_covered`] and every detector benefits.
    fn try_finish(&self, input: BuildSuggestionInput) -> Option<Suggestion> {
        if self.is_covered(input.layer, &input.value, &input.kind) {
            return None;
        }
        Some(build_suggestion(&input))
    }
    /// Returns a `BuildSuggestionInput` with every *context* field
    /// (tab_hostname, matched_key, config_has_site, existing_*) already
    /// filled from this `DetectCtx`. Detectors use this with struct
    /// update syntax to avoid repeating those six fields on every push:
    ///
    /// ```ignore
    /// if let Some(s) = ctx.try_finish(BuildSuggestionInput {
    ///     key: "...".into(),
    ///     layer: SuggestionLayer::Block,
    ///     value: "...".into(),
    ///     reason: "...".into(),
    ///     confidence: 95,
    ///     count: 1,
    ///     evidence: vec![],
    ///     from_frame: None,
    ///     learn: LearnKind::Beacon.text().into(),
    ///     kind: LearnKind::Beacon.tag().into(),
    ///     ..ctx.ctx_fields()
    /// }) {
    ///     out.push(s);
    /// }
    /// ```
    ///
    /// Non-context fields are filled with sentinel defaults that the
    /// struct-update caller is expected to override.
    fn ctx_fields(&self) -> BuildSuggestionInput {
        BuildSuggestionInput {
            // Context (the real purpose of this helper):
            tab_hostname: self.hostname.to_string(),
            matched_key: self.matched_key.map(str::to_string),
            config_has_site: self.config_has_site,
            existing_block: Arc::clone(&self.existing_block),
            existing_remove: Arc::clone(&self.existing_remove),
            existing_hide: Arc::clone(&self.existing_hide),
            // Sentinel defaults - every detector overrides these:
            key: String::new(),
            layer: SuggestionLayer::Block,
            value: String::new(),
            reason: String::new(),
            confidence: 0,
            count: 0,
            evidence: Vec::new(),
            from_frame: None,
            learn: String::new(),
            kind: String::new(),
        }
    }
}

/// Find the first frame in the set that reported from an iframe (i.e.,
/// whose reporterFrame differs from the tab's top-frame hostname).
fn first_non_top_frame<'a, I, T>(items: I, hostname: &str) -> Option<String>
where
    I: IntoIterator<Item = &'a T>,
    T: 'a + HasReporterFrame,
{
    for item in items {
        if let Some(f) = item.reporter_frame() {
            if !f.is_empty() && f != hostname {
                return Some(f.to_string());
            }
        }
    }
    None
}

trait HasReporterFrame {
    fn reporter_frame(&self) -> Option<&str>;
}

impl HasReporterFrame for Resource {
    fn reporter_frame(&self) -> Option<&str> {
        self.reporter_frame.as_deref()
    }
}
impl HasReporterFrame for IframeHit {
    fn reporter_frame(&self) -> Option<&str> {
        self.reporter_frame.as_deref()
    }
}
impl HasReporterFrame for StickyHit {
    fn reporter_frame(&self) -> Option<&str> {
        self.reporter_frame.as_deref()
    }
}

fn median(mut arr: Vec<i64>) -> i64 {
    if arr.is_empty() {
        return 0;
    }
    arr.sort_unstable();
    arr[arr.len() / 2]
}

fn is_subdomain_of(candidate: &str, parent: &str) -> bool {
    candidate != parent && candidate.ends_with(&format!(".{parent}"))
}

fn parse_ts_millis(t: &str) -> Option<i64> {
    // RFC 3339 / ISO 8601 date parsing without pulling chrono: this is a
    // rough parser good enough for canvas-draw timestamps produced by
    // `new Date().toISOString()`. We fall back to Date.parse behavior on
    // failure (return None -> caller substitutes "now").
    //
    // The JS Date.parse implementation accepts a lot of formats; we only
    // need to accept its own toISOString output:
    // "2026-04-19T12:34:56.789Z"
    let bytes = t.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year: i64 = std::str::from_utf8(&bytes[0..4]).ok()?.parse().ok()?;
    let month: i64 = std::str::from_utf8(&bytes[5..7]).ok()?.parse().ok()?;
    let day: i64 = std::str::from_utf8(&bytes[8..10]).ok()?.parse().ok()?;
    let hour: i64 = std::str::from_utf8(&bytes[11..13]).ok()?.parse().ok()?;
    let minute: i64 = std::str::from_utf8(&bytes[14..16]).ok()?.parse().ok()?;
    let second: i64 = std::str::from_utf8(&bytes[17..19]).ok()?.parse().ok()?;
    let millis: i64 = if bytes.len() >= 24 && bytes[19] == b'.' {
        std::str::from_utf8(&bytes[20..23]).ok()?.parse().ok()?
    } else {
        0
    };
    // Days-from-epoch using Howard Hinnant's civil_from_days (simplified
    // for dates in the Gregorian range; avoids chrono dep).
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let m = month as u64;
    let d = day as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i64 - 719_468;
    Some((days * 86_400 + hour * 3600 + minute * 60 + second) * 1000 + millis)
}

// ============================================================================
// Resource-stream detectors
// ============================================================================

/// Detector 1: sendBeacon targets. A third-party host receiving any
/// `navigator.sendBeacon()` call is treated as telemetry.
pub(crate) fn detect_beacon(ctx: &DetectCtx, resources: &[Resource]) -> Vec<Suggestion> {
    let mut by_host: FastMap<&str, Vec<&Resource>> = agg_map();
    for r in resources {
        if r.initiator_type != "beacon" {
            continue;
        }
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        by_host.entry(&r.host).or_default().push(r);
    }
    let mut out = Vec::with_capacity(8);
    for (host, hits) in by_host {
        let value = block_value(host);
        let from_frame = first_non_top_frame(hits.iter().copied(), ctx.hostname);
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: block_key(&value),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!("sendBeacon target ({} beacon{} sent)", hits.len(), plural(hits.len())),
            confidence: 95,
            count: hits.len() as u32,
            evidence: hits.iter().take(5).map(|h| h.url.clone()).collect(),
            from_frame,
            learn: LearnKind::Beacon.text().to_string(),
            kind: LearnKind::Beacon.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }
    out
}

/// Detector 2: tracking pixels. Third-party `<img>` responses under
/// 200 bytes are the classic 1x1 pixel pattern.
pub(crate) fn detect_pixels(ctx: &DetectCtx, resources: &[Resource]) -> Vec<Suggestion> {
    let mut by_host: FastMap<&str, Vec<&Resource>> = agg_map();
    for r in resources {
        if r.initiator_type != "img" {
            continue;
        }
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        if r.transfer_size <= 0 || r.transfer_size >= 200 {
            continue;
        }
        by_host.entry(&r.host).or_default().push(r);
    }
    let mut out = Vec::with_capacity(8);
    for (host, hits) in by_host {
        let value = block_value(host);
        let med = median(hits.iter().map(|h| h.transfer_size).collect());
        let from_frame = first_non_top_frame(hits.iter().copied(), ctx.hostname);
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: block_key(&value),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "tracking pixels: {} tiny image{} (median {}b)",
                hits.len(),
                plural(hits.len()),
                med
            ),
            confidence: 85,
            count: hits.len() as u32,
            evidence: hits
                .iter()
                .take(5)
                .map(|h| format!("{} ({}b)", h.url, h.transfer_size))
                .collect(),
            from_frame,
            learn: LearnKind::Pixel.text().to_string(),
            kind: LearnKind::Pixel.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }
    out
}

/// Detector 3: first-party telemetry subdomains. Subdomains of the tab
/// host whose responses are consistently tiny are internal
/// analytics/logging endpoints that curated lists don't cover.
pub(crate) fn detect_first_party_telemetry(
    ctx: &DetectCtx,
    resources: &[Resource],
) -> Vec<Suggestion> {
    let mut by_host: FastMap<&str, Vec<&Resource>> = agg_map();
    for r in resources {
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        if !is_subdomain_of(&r.host, ctx.hostname) {
            continue;
        }
        by_host.entry(&r.host).or_default().push(r);
    }
    let mut out = Vec::with_capacity(8);
    for (host, requests) in by_host {
        let sizes: Vec<i64> = requests
            .iter()
            .filter(|r| r.transfer_size > 0)
            .map(|r| r.transfer_size)
            .collect();
        if sizes.is_empty() {
            continue;
        }
        let max = *sizes.iter().max().unwrap_or(&0);
        let med = median(sizes);
        if med >= 1024 || max >= 5120 {
            continue;
        }
        let value = block_value(host);
        let from_frame = first_non_top_frame(requests.iter().copied(), ctx.hostname);
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: block_key(&value),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "first-party subdomain with {} tiny response{} (median {}b)",
                requests.len(),
                plural(requests.len()),
                med
            ),
            confidence: 70,
            count: requests.len() as u32,
            evidence: requests
                .iter()
                .take(5)
                .map(|r| {
                    format!(
                        "{} ({}b, {})",
                        r.url, r.transfer_size, r.initiator_type
                    )
                })
                .collect(),
            from_frame,
            learn: LearnKind::FirstPartyTelemetry.text().to_string(),
            kind: LearnKind::FirstPartyTelemetry.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }
    out
}

/// Detector 4: polling endpoints. Same canonical URL hit 4+ times over
/// a 5-600 second window with tiny responses.
pub(crate) fn detect_polling(ctx: &DetectCtx, resources: &[Resource]) -> Vec<Suggestion> {
    struct PollEntry<'a> {
        count: u32,
        sizes: Vec<i64>,
        first_seen: i64,
        last_seen: i64,
        host: &'a str,
        sample: &'a str,
    }
    let mut by_canon: FastMap<String, PollEntry> = agg_map();
    for r in resources {
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        let canon = canonicalize_url(&r.url);
        let entry = by_canon.entry(canon).or_insert_with(|| PollEntry {
            count: 0,
            sizes: Vec::new(),
            first_seen: r.start_time,
            last_seen: r.start_time,
            host: &r.host,
            sample: &r.url,
        });
        entry.count += 1;
        entry.sizes.push(r.transfer_size);
        if r.start_time < entry.first_seen {
            entry.first_seen = r.start_time;
        }
        if r.start_time > entry.last_seen {
            entry.last_seen = r.start_time;
        }
    }
    let mut out = Vec::with_capacity(8);
    for (_canon, info) in by_canon {
        if info.count < 4 {
            continue;
        }
        let span = info.last_seen - info.first_seen;
        if !(5_000..=600_000).contains(&span) {
            continue;
        }
        let med = median(info.sizes.clone());
        if med >= 2048 {
            continue;
        }
        // Polling keeps a trailing "^" anchor so the rule doesn't
        // collide with same-host non-polling block rules on dedup.
        let value = format!("{}^", block_value(info.host));
        let key = block_key(&value);
        if out.iter().any(|s: &Suggestion| s.key == key) {
            continue;
        }
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key,
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "polled {}x over {}s (median {}b)",
                info.count,
                (span / 1000).max(0),
                med
            ),
            confidence: 75,
            count: info.count,
            evidence: vec![info.sample.to_string()],
            from_frame: None,
            learn: LearnKind::Polling.text().to_string(),
            kind: LearnKind::Polling.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }
    out
}

// ============================================================================
// DOM-scan detectors
// ============================================================================

/// Detector 5: hidden iframes. Cross-origin iframes marked hidden by
/// content-script scanning, excluding known-legit captcha/OAuth/payment
/// hosts per the user allowlist.
pub(crate) fn detect_hidden_iframes(
    ctx: &DetectCtx,
    iframes: &[IframeHit],
    iframe_allowlist: &[String],
) -> Vec<Suggestion> {
    struct IframeInfo<'a> {
        reasons: std::collections::BTreeSet<String>,
        samples: Vec<&'a IframeHit>,
    }
    let mut by_host: FastMap<&str, IframeInfo> = agg_map();
    for f in iframes {
        if f.src.is_empty() || f.host.is_empty() {
            continue;
        }
        if is_legit_hidden_iframe(&f.src, iframe_allowlist) {
            continue;
        }
        let entry = by_host.entry(&f.host).or_insert_with(|| IframeInfo {
            reasons: Default::default(),
            samples: Vec::new(),
        });
        for r in &f.reasons {
            entry.reasons.insert(r.clone());
        }
        entry.samples.push(f);
    }
    let mut out = Vec::with_capacity(8);
    for (host, info) in by_host {
        let selector = format!("iframe[src*=\"{host}\"]");
        let from_frame = first_non_top_frame(info.samples.iter().copied(), ctx.hostname);
        let reason_list: Vec<String> = info.reasons.into_iter().collect();
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: format!("remove::{selector}"),
            layer: SuggestionLayer::Remove,
            value: selector.clone(),
            reason: format!("hidden iframe: {}", reason_list.join(", ")),
            confidence: 80,
            count: info.samples.len() as u32,
            evidence: info
                .samples
                .iter()
                .take(3)
                .map(|s| s.outer_html_preview.clone())
                .collect(),
            from_frame,
            learn: LearnKind::HiddenIframe.text().to_string(),
            kind: LearnKind::HiddenIframe.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }
    out
}

/// Detector 13: sticky overlays. Viewport-covering fixed-position
/// elements; dedup per selector.
pub(crate) fn detect_sticky_overlays(
    ctx: &DetectCtx,
    stickies: &[StickyHit],
) -> Vec<Suggestion> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::with_capacity(8);
    for s in stickies {
        if s.selector.is_empty() {
            continue;
        }
        if !seen.insert(s.selector.clone()) {
            continue;
        }
        let from_frame = s
            .reporter_frame
            .as_deref()
            .filter(|f| !f.is_empty() && *f != ctx.hostname)
            .map(str::to_string);
        if let Some(suggestion) = ctx.try_finish(BuildSuggestionInput {
            key: format!("hide::{}", s.selector),
            layer: SuggestionLayer::Hide,
            value: s.selector.clone(),
            reason: format!(
                "fixed overlay covering {}% of viewport (z-index {})",
                s.coverage, s.z_index
            ),
            confidence: 55,
            count: 1,
            evidence: vec![format!(
                "{}x{} at z-index {}",
                s.rect.w, s.rect.h, s.z_index
            )],
            from_frame,
            learn: LearnKind::StickyOverlay.text().to_string(),
            kind: LearnKind::StickyOverlay.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(suggestion);
        }
    }
    out
}

// ============================================================================
// Main-world jsCalls detectors
// ============================================================================

/// Emit an origin-block suggestion via the engine's shared helper.
#[allow(clippy::too_many_arguments)]
fn emit_origin_block(
    ctx: &DetectCtx,
    out: &mut Vec<Suggestion>,
    origin: &str,
    reason: String,
    confidence: u8,
    kind_tag: &str,
    learn: LearnKind,
) {
    if origin.is_empty() {
        return;
    }
    let value = block_value(origin);
    if let Some(s) = ctx.try_finish(BuildSuggestionInput {
        key: format!("{}::{kind_tag}", block_key(&value)),
        layer: SuggestionLayer::Block,
        value: value.clone(),
        reason,
        confidence,
        count: 1,
        evidence: vec![],
        from_frame: None,
        learn: learn.text().to_string(),
        kind: learn.tag().to_string(),
        ..ctx.ctx_fields()
    }) {
        out.push(s);
    }
}

/// Shared aggregation pass over js_calls; every main-world detector
/// reads from this. Keeps a single O(n) loop rather than one per
/// detector.
struct JsCallSummary {
    seconds_since_first: i64,
    origins_by_kind: FastMap<String, FastMap<String, u32>>,
    hot_params_by_origin: FastMap<String, u32>,
    distinct_fonts_by_origin: FastMap<String, std::collections::BTreeSet<String>>,
    listener_types_by_origin: FastMap<String, ListenerInfo>,
    attention_types_by_origin: FastMap<String, ListenerInfo>,
    replay_vendors: FastMap<String, u32>,
    raf_waste_by_key: FastMap<String, RafWasteEntry>,
}

struct ListenerInfo {
    count: u32,
    types: std::collections::BTreeSet<String>,
}

/// Page-lifecycle / visibility event names tracked by the
/// attention-tracking detector. Must match `ATTENTION_EVENT_TYPES`
/// in `mainworld.js`.
const ATTENTION_EVENT_NAMES: &[&str] = &[
    "visibilitychange",
    "focus",
    "blur",
    "pagehide",
    "pageshow",
    "beforeunload",
];

fn is_attention_event(t: &str) -> bool {
    ATTENTION_EVENT_NAMES.iter().any(|n| *n == t)
}

struct RafWasteEntry {
    origin: String,
    canvas_sel: String,
    total: u32,
    invisible: u32,
    first_t: String,
    last_t: String,
}

fn summarize_js_calls(js_calls: &[JsCall]) -> JsCallSummary {
    let now_ts = 0i64; // relative seconds; first-call baseline is what matters
    let first_ts = js_calls
        .first()
        .and_then(|c| parse_ts_millis(&c.t))
        .unwrap_or(now_ts);
    let latest_ts = js_calls
        .iter()
        .rev()
        .find_map(|c| parse_ts_millis(&c.t))
        .unwrap_or(first_ts);
    let seconds_since_first = ((latest_ts - first_ts) / 1000).max(1);

    // Every aggregation map starts pre-sized via `agg_map()` so the
    // first 2-3 grows are free; 11 signal kinds + typically <20
    // distinct origins means AGG_MAP_CAPACITY is enough headroom.
    let mut origins_by_kind: FastMap<String, FastMap<String, u32>> = agg_map();
    let mut hot_params_by_origin: FastMap<String, u32> = agg_map();
    let mut distinct_fonts_by_origin: FastMap<String, std::collections::BTreeSet<String>> =
        agg_map();
    let mut listener_types_by_origin: FastMap<String, ListenerInfo> = agg_map();
    let mut attention_types_by_origin: FastMap<String, ListenerInfo> = agg_map();
    let mut replay_vendors: FastMap<String, u32> = agg_map();
    let mut raf_waste_by_key: FastMap<String, RafWasteEntry> = agg_map();

    for c in js_calls {
        let origin = {
            let h = script_origin_from_stack(&c.stack);
            if h.is_empty() {
                "(unknown script)".to_string()
            } else {
                h
            }
        };
        origins_by_kind
            .entry(c.kind.clone())
            .or_default()
            .entry(origin.clone())
            .and_modify(|v| *v += 1)
            .or_insert(1);

        match c.kind.as_str() {
            "webgl-fp" if c.hot_param => {
                *hot_params_by_origin.entry(origin.clone()).or_insert(0) += 1;
            }
            "font-fp" => {
                if let Some(font) = c.font.as_deref() {
                    if !font.is_empty() {
                        distinct_fonts_by_origin
                            .entry(origin.clone())
                            .or_default()
                            .insert(font.to_string());
                    }
                }
            }
            "listener-added" => {
                if let Some(t) = c.event_type.as_deref() {
                    if !t.is_empty() {
                        // Attention events (visibilitychange / focus /
                        // blur / pagehide / pageshow / beforeunload)
                        // feed the attention-tracking detector;
                        // everything else feeds the interaction-
                        // density replay-listener detector.
                        let bucket = if is_attention_event(t) {
                            &mut attention_types_by_origin
                        } else {
                            &mut listener_types_by_origin
                        };
                        let entry = bucket
                            .entry(origin.clone())
                            .or_insert_with(|| ListenerInfo {
                                count: 0,
                                types: Default::default(),
                            });
                        entry.count += 1;
                        entry.types.insert(t.to_string());
                    }
                }
            }
            "replay-global" => {
                for v in &c.vendors {
                    if !v.vendor.is_empty() {
                        *replay_vendors.entry(v.vendor.clone()).or_insert(0) += 1;
                    }
                }
            }
            "canvas-draw" => {
                let sel = c.canvas_sel.clone().unwrap_or_else(|| "canvas".to_string());
                let key = format!("{origin}|{sel}");
                let entry = raf_waste_by_key
                    .entry(key)
                    .or_insert_with(|| RafWasteEntry {
                        origin: origin.clone(),
                        canvas_sel: sel.clone(),
                        total: 0,
                        invisible: 0,
                        first_t: c.t.clone(),
                        last_t: c.t.clone(),
                    });
                entry.total += 1;
                if c.visible == Some(false) {
                    entry.invisible += 1;
                }
                entry.last_t = c.t.clone();
            }
            _ => {}
        }
    }

    JsCallSummary {
        seconds_since_first,
        origins_by_kind,
        hot_params_by_origin,
        distinct_fonts_by_origin,
        listener_types_by_origin,
        attention_types_by_origin,
        replay_vendors,
        raf_waste_by_key,
    }
}

/// Detector 6: canvas fingerprinting.
/// Detector 7: webgl fingerprinting (hot and general).
/// Detector 8: audio fingerprinting.
/// Detector 9: font-enumeration fingerprinting.
/// Detector 10: session-replay vendor globals.
/// Detector 11: session-replay listener density.
/// Detector 12: invisible-animation-loop (raf-waste).
pub(crate) fn detect_from_js_calls(
    ctx: &DetectCtx,
    js_calls: &[JsCall],
) -> Vec<Suggestion> {
    let s = summarize_js_calls(js_calls);
    let mut out = Vec::with_capacity(8);

    // Canvas fingerprinting: 3+ toDataURL/toBlob/getImageData from one origin.
    if let Some(origins) = s.origins_by_kind.get("canvas-fp") {
        for (origin, &cnt) in origins {
            if cnt >= 3 {
                emit_origin_block(
                    ctx,
                    &mut out,
                    origin,
                    format!("canvas fingerprinting ({cnt} toDataURL/getImageData reads)"),
                    90,
                    "canvas-fp",
                    LearnKind::CanvasFp,
                );
            }
        }
    }

    // WebGL UNMASKED reads: the hot 95-confidence case.
    // Suppress when the user already has `webgl-unmasked` spoof
    // active — the spoof neutralizes the exact signal (UNMASKED_
    // VENDOR/RENDERER return bland strings). The detector is
    // still OBSERVING the reads (that's what generates the signal
    // in the first place), but surfacing a new block-the-origin
    // suggestion for a user who already picked the spoof lane is
    // just nagging. If the user ever removes the spoof rule, the
    // suggestion will resurface on the next tab.
    // `webgl-fp-hot` → `webgl-unmasked` spoof equivalence is
    // encoded in `spoof_kind_for_signal` + `DetectCtx::is_covered`,
    // so emit_origin_block short-circuits when the spoof rule is
    // already active. No per-detector skip needed.
    for (origin, &hot_count) in &s.hot_params_by_origin {
        if hot_count >= 1 {
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                "WebGL fingerprinting (read UNMASKED_RENDERER_WEBGL or _VENDOR_WEBGL)"
                    .to_string(),
                95,
                "webgl-fp-hot",
                LearnKind::WebglFpHot,
            );
        }
    }
    // WebGL general getParameter flurry (excluded from origins already flagged for UNMASKED).
    if let Some(origins) = s.origins_by_kind.get("webgl-fp") {
        for (origin, &cnt) in origins {
            let hot = *s.hot_params_by_origin.get(origin).unwrap_or(&0);
            if cnt >= 8 && hot < 1 {
                emit_origin_block(
                    ctx,
                    &mut out,
                    origin,
                    format!("WebGL fingerprinting ({cnt} getParameter reads)"),
                    75,
                    "webgl-fp",
                    LearnKind::WebglFp,
                );
            }
        }
    }

    // Audio fingerprinting: any OfflineAudioContext construction is flagged.
    if let Some(origins) = s.origins_by_kind.get("audio-fp") {
        for (origin, &cnt) in origins {
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                format!("audio fingerprinting (OfflineAudioContext constructed {cnt}x)"),
                90,
                "audio-fp",
                LearnKind::AudioFp,
            );
        }
    }

    // Font enumeration: 20+ distinct fonts measured from one origin.
    for (origin, fonts) in &s.distinct_fonts_by_origin {
        if fonts.len() >= 20 {
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                format!(
                    "font enumeration fingerprinting ({} distinct fonts probed)",
                    fonts.len()
                ),
                85,
                "font-fp",
                LearnKind::FontFp,
            );
        }
    }

    // Session-replay vendor globals.
    for (vendor, &cnt) in &s.replay_vendors {
        let vendor_host: Option<&str> = match vendor.as_str() {
            "Hotjar" => Some("hotjar.com"),
            "FullStory" => Some("fullstory.com"),
            "Microsoft Clarity" => Some("clarity.ms"),
            "LogRocket" => Some("logrocket.com"),
            "Smartlook" => Some("smartlook.com"),
            "Mouseflow" => Some("mouseflow.com"),
            "PostHog" => Some("posthog.com"),
            _ => None,
        };
        let Some(vendor_host) = vendor_host else {
            continue;
        };
        let value = block_value(vendor_host);
        let sentinel_name = match vendor.as_str() {
            "Hotjar" => "_hjSettings",
            "FullStory" => "FS",
            "Microsoft Clarity" => "clarity",
            "LogRocket" => "LogRocket",
            "Smartlook" => "smartlook",
            "Mouseflow" => "mouseflow",
            "PostHog" => "__posthog",
            _ => "?",
        };
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: format!("{}::replay-{vendor}", block_key(&value)),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "{vendor} session replay detected (captures clicks, keystrokes, scrolls)"
            ),
            confidence: 95,
            count: cnt,
            evidence: vec![format!("Global sentinel found in page: window.{sentinel_name}")],
            from_frame: None,
            learn: LearnKind::ReplayVendor.text().to_string(),
            kind: LearnKind::ReplayVendor.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }

    // Hardware device-API probes: any call to
    // navigator.bluetooth / usb / hid / serial .requestDevice (or
    // .requestPort) from a page script is high-signal. These APIs
    // are gesture-gated and show a native permission prompt, but
    // the prompt itself is an entropy signal (the site learns the
    // API exists on the browser / OS) and legit uses are rare and
    // contextual. One call → block suggestion for the calling
    // origin. Confidence 90.
    if let Some(origins) = s.origins_by_kind.get("new-api-probe") {
        for (origin, &cnt) in origins {
            if origin.is_empty() {
                continue;
            }
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                format!(
                    "hardware-device API probe ({cnt} requestDevice / requestPort calls)"
                ),
                90,
                "device-api-probe",
                LearnKind::DeviceApiProbe,
            );
        }
    }

    // Clipboard read: any call to navigator.clipboard.readText() from
    // a page script is high-signal. Chrome gesture-gates the API, so
    // the site had to convince the user to interact first — but legit
    // page scripts almost never need it. One call is enough to emit
    // a block suggestion for the calling origin; confidence 95.
    if let Some(origins) = s.origins_by_kind.get("clipboard-fp") {
        for (origin, &cnt) in origins {
            if origin.is_empty() {
                continue;
            }
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                format!(
                    "clipboard read (navigator.clipboard.readText called {cnt}x)"
                ),
                95,
                "clipboard-read",
                LearnKind::ClipboardRead,
            );
        }
    }

    // Attention-tracking: 4+ page-lifecycle / visibility listeners
    // from one origin with 3+ distinct event types in <60s. Threshold
    // tighter than the interaction-density detector because attention
    // events are much rarer — a legit site attaches maybe one
    // `visibilitychange` to resume a video and one `beforeunload` to
    // warn on unsaved data. 4+ across 3+ types is session-replay /
    // engagement-analytics density, not normal page code.
    //
    // Emits Neuter (same primitive as replay-listener) — the
    // main-world hook denies the registration so the capture path
    // never runs. Dedup against existing neuter or block rules for
    // the origin, mirroring replay-listener's is_covered check.
    for (origin, info) in &s.attention_types_by_origin {
        if info.count >= 4 && info.types.len() >= 3 && s.seconds_since_first < 60 {
            if origin.is_empty() {
                continue;
            }
            let value = block_value(origin);
            let types: Vec<&str> = info.types.iter().map(String::as_str).collect();
            if let Some(sg) = ctx.try_finish(BuildSuggestionInput {
                key: format!("neuter::{value}::attention-tracking"),
                layer: SuggestionLayer::Neuter,
                value: value.clone(),
                reason: format!(
                    "attention-tracking pattern ({} page-lifecycle listeners: {})",
                    info.count,
                    types.join(", ")
                ),
                confidence: 75,
                count: info.count,
                evidence: vec![],
                from_frame: None,
                learn: LearnKind::AttentionTracking.text().to_string(),
                kind: LearnKind::AttentionTracking.tag().to_string(),
                ..ctx.ctx_fields()
            }) {
                out.push(sg);
            }
        }
    }

    // Listener density: 12+ interaction listeners from one origin in <60s.
    // Emits a `neuter` suggestion (not block) — neuter denies the
    // listener registrations upstream so no capture loop runs. The
    // rule's match value is the script-origin URL pattern, not the
    // request URL — main-world enforcement checks the stack origin
    // against it at addEventListener call time.
    //
    // Dedup: skip when the user already has a neuter OR block rule
    // covering this origin. Block is enough because a URL-blocked
    // script never runs, so no listeners can register in the first
    // place. Neuter is the exact primitive this suggestion proposes
    // — duplicate suggestion is pure noise.
    for (origin, info) in &s.listener_types_by_origin {
        if info.count >= 12 && info.types.len() >= 3 && s.seconds_since_first < 60 {
            if origin.is_empty() {
                continue;
            }
            let value = block_value(origin);
            let types: Vec<&str> = info.types.iter().map(String::as_str).collect();
            if let Some(sg) = ctx.try_finish(BuildSuggestionInput {
                key: format!("neuter::{value}::listener-density"),
                layer: SuggestionLayer::Neuter,
                value: value.clone(),
                reason: format!(
                    "session replay pattern ({} interaction listeners attached: {})",
                    info.count,
                    types.join(", ")
                ),
                confidence: 80,
                count: info.count,
                evidence: vec![],
                from_frame: None,
                learn: LearnKind::ReplayListener.text().to_string(),
                kind: LearnKind::ReplayListener.tag().to_string(),
                ..ctx.ctx_fields()
            }) {
                out.push(sg);
            }
        }
    }

    // Invisible animation-loop (raf-waste).
    for entry in s.raf_waste_by_key.values() {
        if entry.total < 20 {
            continue;
        }
        let first = parse_ts_millis(&entry.first_t).unwrap_or(0);
        let last = parse_ts_millis(&entry.last_t).unwrap_or(0);
        let span = last - first;
        if span < 3000 {
            continue;
        }
        let invisible_ratio = entry.invisible as f64 / entry.total as f64;
        if invisible_ratio < 0.8 {
            continue;
        }
        if entry.origin.is_empty() || entry.origin == "(unknown script)" {
            continue;
        }
        let value = block_value(&entry.origin);
        let seconds = ((span / 1000) as u32).max(1);
        let rate_per_sec = (entry.invisible as f64 / seconds as f64 * 10.0).round() / 10.0;
        if let Some(s) = ctx.try_finish(BuildSuggestionInput {
            key: format!("{}::raf-waste::{}", block_key(&value), entry.canvas_sel),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "invisible animation loop ({} draws to {} in {}s, ~{}/s)",
                entry.invisible, entry.canvas_sel, seconds, rate_per_sec
            ),
            confidence: 70,
            count: entry.invisible,
            evidence: vec![
                format!(
                    "{}: {}/{} samples invisible",
                    entry.canvas_sel, entry.invisible, entry.total
                ),
                format!("origin: {}", entry.origin),
                format!("window: {seconds}s"),
            ],
            from_frame: None,
            learn: LearnKind::RafWaste.text().to_string(),
            kind: LearnKind::RafWaste.tag().to_string(),
            ..ctx.ctx_fields()
        }) {
            out.push(s);
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ReplayVendor, StickyRect};

    fn ctx<'a>(hostname: &'a str) -> DetectCtx<'a> {
        DetectCtx {
            hostname,
            matched_key: None,
            config_has_site: false,
            existing_block: Arc::from([] as [String; 0]),
            existing_remove: Arc::from([] as [String; 0]),
            existing_hide: Arc::from([] as [String; 0]),
            existing_neuter: Arc::from([] as [String; 0]),
            existing_silence: Arc::from([] as [String; 0]),
            existing_spoof: Arc::from([] as [String; 0]),
        }
    }

    fn resource(url: &str, host: &str, init: &str, size: i64) -> Resource {
        Resource {
            url: url.into(),
            host: host.into(),
            initiator_type: init.into(),
            transfer_size: size,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }
    }

    #[test]
    fn beacon_detects_third_party_sendbeacon_target() {
        let resources = vec![
            resource("https://tracker.test/p", "tracker.test", "beacon", 0),
            resource("https://tracker.test/p", "tracker.test", "beacon", 0),
        ];
        let out = detect_beacon(&ctx("site.test"), &resources);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "||tracker.test");
        assert_eq!(out[0].confidence, 95);
        assert_eq!(out[0].count, 2);
    }

    #[test]
    fn beacon_skips_first_party_beacons() {
        let resources = vec![resource(
            "https://site.test/p",
            "site.test",
            "beacon",
            0,
        )];
        let out = detect_beacon(&ctx("site.test"), &resources);
        assert!(out.is_empty());
    }

    #[test]
    fn pixel_detects_tiny_third_party_image() {
        let resources = vec![
            resource("https://ads.test/p.gif", "ads.test", "img", 43),
            resource("https://ads.test/q.gif", "ads.test", "img", 43),
        ];
        let out = detect_pixels(&ctx("site.test"), &resources);
        assert_eq!(out.len(), 1);
        assert!(out[0].reason.contains("tracking pixels"));
    }

    #[test]
    fn pixel_skips_large_images() {
        let resources = vec![resource(
            "https://cdn.test/big.jpg",
            "cdn.test",
            "img",
            50_000,
        )];
        let out = detect_pixels(&ctx("site.test"), &resources);
        assert!(out.is_empty());
    }

    #[test]
    fn first_party_telemetry_detects_subdomain_with_tiny_responses() {
        let resources = vec![
            resource("https://log.site.test/a", "log.site.test", "fetch", 100),
            resource("https://log.site.test/b", "log.site.test", "fetch", 200),
            resource("https://log.site.test/c", "log.site.test", "fetch", 150),
        ];
        let out = detect_first_party_telemetry(&ctx("site.test"), &resources);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "||log.site.test");
    }

    #[test]
    fn first_party_telemetry_skips_large_responses() {
        let resources = vec![resource(
            "https://log.site.test/big",
            "log.site.test",
            "fetch",
            50_000,
        )];
        let out = detect_first_party_telemetry(&ctx("site.test"), &resources);
        assert!(out.is_empty());
    }

    #[test]
    fn polling_detects_4_plus_hits_in_window() {
        let base_time = 0;
        let spacing = 2_000; // 2s apart
        let mut resources = Vec::new();
        for i in 0..4 {
            resources.push(Resource {
                url: "https://api.test/poll?t=1".into(),
                host: "api.test".into(),
                initiator_type: "fetch".into(),
                transfer_size: 50,
                duration: 0,
                start_time: base_time + (i as i64 * spacing),
                reporter_frame: None,
            });
        }
        let out = detect_polling(&ctx("site.test"), &resources);
        assert_eq!(out.len(), 1, "should detect polling");
        assert!(out[0].value.starts_with("||api.test"));
    }

    #[test]
    fn polling_requires_window_above_5s() {
        let mut resources = Vec::new();
        for i in 0..5 {
            resources.push(Resource {
                url: "https://api.test/p".into(),
                host: "api.test".into(),
                initiator_type: "fetch".into(),
                transfer_size: 50,
                duration: 0,
                start_time: i as i64 * 500, // 500ms apart, total 2s
                reporter_frame: None,
            });
        }
        let out = detect_polling(&ctx("site.test"), &resources);
        assert!(out.is_empty());
    }

    #[test]
    fn hidden_iframe_detected_unless_allowlisted() {
        let iframes = vec![IframeHit {
            src: "https://ads.test/ad.html".into(),
            host: "ads.test".into(),
            reasons: vec!["display:none".into()],
            width: 0,
            height: 0,
            outer_html_preview: "<iframe ...>".into(),
            reporter_frame: None,
        }];
        let allow: Vec<String> = vec![];
        let out = detect_hidden_iframes(&ctx("site.test"), &iframes, &allow);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].layer, SuggestionLayer::Remove);
        assert!(out[0].value.contains("ads.test"));

        let allow = vec!["ads.test".to_string()];
        let out = detect_hidden_iframes(&ctx("site.test"), &iframes, &allow);
        assert!(out.is_empty(), "allowlisted iframe skipped");
    }

    #[test]
    fn sticky_overlay_detected() {
        let stickies = vec![StickyHit {
            selector: "div.popup".into(),
            coverage: 50,
            z_index: 9999,
            rect: StickyRect { w: 400, h: 300 },
            reporter_frame: None,
        }];
        let out = detect_sticky_overlays(&ctx("site.test"), &stickies);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].layer, SuggestionLayer::Hide);
        assert_eq!(out[0].confidence, 55);
    }

    fn canvas_fp_call(origin_host: &str) -> JsCall {
        JsCall {
            kind: "canvas-fp".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack: vec![format!("at x (https://{origin_host}/fp.js:1:1)")],
            ..Default::default()
        }
    }

    #[test]
    fn canvas_fp_triggers_at_3_calls() {
        let calls = vec![
            canvas_fp_call("fp.test"),
            canvas_fp_call("fp.test"),
            canvas_fp_call("fp.test"),
        ];
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "||fp.test");
        assert_eq!(out[0].confidence, 90);
    }

    #[test]
    fn canvas_fp_below_threshold_no_suggestion() {
        let calls = vec![canvas_fp_call("fp.test"), canvas_fp_call("fp.test")];
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert!(out.is_empty());
    }

    #[test]
    fn webgl_fp_hot_unmasked_fires_even_for_single_read() {
        let call = JsCall {
            kind: "webgl-fp".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
            hot_param: true,
            ..Default::default()
        };
        let out = detect_from_js_calls(&ctx("site.test"), &[call]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, 95);
        assert!(out[0].key.ends_with("::webgl-fp-hot"));
    }

    #[test]
    fn is_covered_central_dedup_rules() {
        // Same-layer same-value.
        let mut c = ctx("site.test");
        c.existing_block = Arc::from(["||foo.com".to_string()]);
        assert!(c.is_covered(SuggestionLayer::Block, "||foo.com", "beacon"));
        assert!(!c.is_covered(SuggestionLayer::Block, "||bar.com", "beacon"));

        // Block covers Neuter: URL-blocked script can't run.
        let mut c = ctx("site.test");
        c.existing_block = Arc::from(["||hotjar.com".to_string()]);
        assert!(c.is_covered(SuggestionLayer::Neuter, "||hotjar.com", "replay-listener"));

        // Block covers Silence: same reasoning.
        assert!(c.is_covered(SuggestionLayer::Silence, "||hotjar.com", "replay-listener"));

        // Neuter does NOT cover Block (narrower primitive).
        let mut c = ctx("site.test");
        c.existing_neuter = Arc::from(["||hotjar.com".to_string()]);
        assert!(c.is_covered(SuggestionLayer::Neuter, "||hotjar.com", "replay-listener"));
        assert!(!c.is_covered(SuggestionLayer::Block, "||hotjar.com", "beacon"));

        // Spoof covers the equivalent fingerprint-signal block
        // suggestion via `spoof_kind_for_signal`.
        let mut c = ctx("site.test");
        c.existing_spoof = Arc::from(["webgl-unmasked".to_string()]);
        assert!(c.is_covered(SuggestionLayer::Block, "||fp.test", "webgl-fp-hot"));
        // Non-hot webgl-fp is NOT covered by webgl-unmasked spoof
        // (spoof only neutralizes UNMASKED reads).
        assert!(!c.is_covered(SuggestionLayer::Block, "||fp.test", "webgl-fp"));

        // Spoof doesn't cross over to Remove/Hide.
        assert!(!c.is_covered(SuggestionLayer::Remove, ".foo", "webgl-fp-hot"));
    }

    #[test]
    fn replay_listener_suppressed_when_neuter_or_block_rule_exists() {
        // Regression: after accepting the replay-listener
        // suggestion (which now lands a neuter rule for the
        // origin), the detector must not keep re-surfacing the
        // same suggestion. Also covers the block-rule case — if
        // the user blocked the script URL entirely, the listeners
        // can't exist anyway.
        // Detector requires count >= 12 AND 3+ distinct types.
        let types = ["click", "keydown", "mousemove", "scroll"];
        let calls: Vec<JsCall> = (0..16)
            .map(|i| JsCall {
                kind: "listener-added".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at emit (https://replay.test/r.js:1:1)".into()],
                event_type: Some(types[i % types.len()].into()),
                ..Default::default()
            })
            .collect();

        // Baseline: no rules → suggestion fires.
        let baseline = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(baseline.len(), 1, "baseline must emit the suggestion");
        assert_eq!(baseline[0].value, "||replay.test");

        // With neuter rule for the origin → dedup.
        let mut neutered = ctx("site.test");
        neutered.existing_neuter = Arc::from(["||replay.test".to_string()]);
        assert!(
            detect_from_js_calls(&neutered, &calls).is_empty(),
            "neuter rule must dedup replay-listener suggestion"
        );

        // With block rule for the origin → dedup.
        let mut blocked = ctx("site.test");
        blocked.existing_block = Arc::from(["||replay.test".to_string()]);
        assert!(
            detect_from_js_calls(&blocked, &calls).is_empty(),
            "block rule must dedup replay-listener suggestion (no script, no listeners)"
        );
    }

    #[test]
    fn webgl_fp_hot_suppressed_when_webgl_unmasked_spoof_active() {
        // When the user has `spoof: ["webgl-unmasked"]` enabled,
        // the equivalent block suggestion should not surface —
        // the spoof already neutralizes the exact signal.
        // Regression lock for the "spoof already active" nag case.
        let mut ctx = ctx("site.test");
        ctx.existing_spoof = Arc::from(["webgl-unmasked".to_string()]);
        let call = JsCall {
            kind: "webgl-fp".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
            hot_param: true,
            ..Default::default()
        };
        let out = detect_from_js_calls(&ctx, &[call]);
        assert!(
            out.is_empty(),
            "webgl-unmasked spoof must suppress the webgl-fp-hot block suggestion"
        );
    }

    #[test]
    fn font_fp_fires_at_20_distinct_fonts() {
        let calls: Vec<JsCall> = (0..25)
            .map(|i| JsCall {
                kind: "font-fp".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
                font: Some(format!("font-{i}")),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, 85);
    }

    #[test]
    fn replay_vendor_hotjar_triggers_block_for_vendor_host() {
        let call = JsCall {
            kind: "replay-global".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            vendors: vec![ReplayVendor {
                key: "_hjSettings".into(),
                vendor: "Hotjar".into(),
            }],
            ..Default::default()
        };
        let out = detect_from_js_calls(&ctx("site.test"), &[call]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].value, "||hotjar.com");
        assert_eq!(out[0].confidence, 95);
    }

    #[test]
    fn listener_density_fires_at_12_interactions_within_60s() {
        let calls: Vec<JsCall> = ["mousemove", "mousedown", "click", "keydown", "scroll"]
            .iter()
            .cycle()
            .take(12)
            .map(|t| JsCall {
                kind: "listener-added".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at fp (https://replay.test/r.js:1:1)".into()],
                event_type: Some(t.to_string()),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].confidence, 80);
    }

    #[test]
    fn device_api_probe_fires_on_single_call() {
        let calls = vec![JsCall {
            kind: "new-api-probe".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack: vec!["at probe (https://hw.test/p.js:1:1)".into()],
            method: Some("Bluetooth.requestDevice".into()),
            ..Default::default()
        }];
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1, "device-api-probe should fire on first call");
        assert_eq!(out[0].kind, "device-api-probe");
        assert_eq!(out[0].confidence, 90);
        assert_eq!(out[0].layer, SuggestionLayer::Block);
        assert!(
            out[0].reason.contains("hardware-device API"),
            "reason: {}",
            out[0].reason
        );
    }

    #[test]
    fn device_api_probe_consolidates_multi_api_calls_per_origin() {
        // One script origin hitting multiple device APIs (Bluetooth
        // + USB in a single probe sweep) still produces ONE
        // suggestion for that origin, not one per API. The count
        // in the reason reflects all calls.
        let calls = vec![
            JsCall {
                kind: "new-api-probe".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at probe (https://hw.test/p.js:1:1)".into()],
                method: Some("Bluetooth.requestDevice".into()),
                ..Default::default()
            },
            JsCall {
                kind: "new-api-probe".into(),
                t: "2026-04-19T12:00:01.000Z".into(),
                stack: vec!["at probe (https://hw.test/p.js:1:1)".into()],
                method: Some("USB.requestDevice".into()),
                ..Default::default()
            },
        ];
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        let device_suggestions: Vec<_> = out
            .iter()
            .filter(|s| s.kind == "device-api-probe")
            .collect();
        assert_eq!(device_suggestions.len(), 1);
        assert!(device_suggestions[0].reason.contains("2"));
    }

    #[test]
    fn clipboard_read_fires_on_single_call() {
        let calls = vec![JsCall {
            kind: "clipboard-fp".into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack: vec!["at sniff (https://clip.test/s.js:1:1)".into()],
            method: Some("readText".into()),
            ..Default::default()
        }];
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1, "clipboard-read should fire on first call");
        assert_eq!(out[0].kind, "clipboard-read");
        assert_eq!(out[0].confidence, 95);
        assert_eq!(out[0].layer, SuggestionLayer::Block);
        assert!(
            out[0].reason.contains("clipboard read"),
            "reason: {}",
            out[0].reason
        );
    }

    #[test]
    fn clipboard_read_counts_repeat_calls_in_reason() {
        let calls: Vec<JsCall> = (0..3)
            .map(|_| JsCall {
                kind: "clipboard-fp".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at sniff (https://clip.test/s.js:1:1)".into()],
                method: Some("readText".into()),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1);
        assert!(
            out[0].reason.contains("3x"),
            "expected call count in reason, got: {}",
            out[0].reason
        );
    }

    #[test]
    fn attention_tracking_fires_at_4_listeners_3_types_within_60s() {
        let calls: Vec<JsCall> = ["visibilitychange", "focus", "blur", "pagehide"]
            .iter()
            .map(|t| JsCall {
                kind: "listener-added".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at init (https://attn.test/a.js:1:1)".into()],
                event_type: Some(t.to_string()),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1, "attention-tracking should fire");
        assert_eq!(out[0].layer, SuggestionLayer::Neuter);
        assert_eq!(out[0].kind, "attention-tracking");
        assert!(
            out[0].reason.contains("attention-tracking pattern"),
            "reason: {}",
            out[0].reason
        );
    }

    #[test]
    fn attention_tracking_below_threshold_no_suggestion() {
        // Only 3 listeners across 3 types — one short of the count
        // threshold. Should not fire.
        let calls: Vec<JsCall> = ["visibilitychange", "focus", "blur"]
            .iter()
            .map(|t| JsCall {
                kind: "listener-added".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at init (https://attn.test/a.js:1:1)".into()],
                event_type: Some(t.to_string()),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert!(
            out.iter().all(|s| s.kind != "attention-tracking"),
            "should not suggest attention-tracking below threshold"
        );
    }

    #[test]
    fn attention_listeners_dont_also_fire_replay_listener() {
        // Attention listeners must feed the attention-tracking
        // detector ONLY, not also the interaction-density
        // replay-listener detector. Otherwise a site that attaches
        // 12 visibilitychange listeners would (incorrectly) trip
        // the replay-listener heuristic.
        let calls: Vec<JsCall> = (0..12)
            .map(|_| JsCall {
                kind: "listener-added".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at init (https://attn.test/a.js:1:1)".into()],
                event_type: Some("visibilitychange".into()),
                ..Default::default()
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert!(
            out.iter().all(|s| s.kind != "listener-density"),
            "replay-listener must not fire on pure-attention events"
        );
    }

    #[test]
    fn raf_waste_fires_on_sustained_invisible_draws() {
        let calls: Vec<JsCall> = (0..30)
            .map(|i| {
                let secs = i as i64; // 0..29s
                let t = format!(
                    "2026-04-19T12:00:{:02}.000Z",
                    secs.min(59)
                );
                JsCall {
                    kind: "canvas-draw".into(),
                    t,
                    stack: vec!["at paint (https://script.test/a.js:1:1)".into()],
                    op: Some("fillRect".into()),
                    visible: Some(false),
                    canvas_sel: Some("canvas#stage".into()),
                    ..Default::default()
                }
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert_eq!(out.len(), 1, "raf-waste should fire");
        assert!(out[0].reason.contains("invisible animation loop"));
    }

    #[test]
    fn raf_waste_does_not_fire_for_visible_canvas() {
        let calls: Vec<JsCall> = (0..30)
            .map(|i| {
                let secs = i as i64;
                let t = format!(
                    "2026-04-19T12:00:{:02}.000Z",
                    secs.min(59)
                );
                JsCall {
                    kind: "canvas-draw".into(),
                    t,
                    stack: vec!["at paint (https://script.test/a.js:1:1)".into()],
                    op: Some("fillRect".into()),
                    visible: Some(true),
                    canvas_sel: Some("canvas#stage".into()),
                    ..Default::default()
                }
            })
            .collect();
        let out = detect_from_js_calls(&ctx("site.test"), &calls);
        assert!(out.is_empty());
    }
}
