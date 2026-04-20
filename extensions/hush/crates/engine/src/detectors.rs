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

use std::collections::HashMap;

use hush_types::{
    BuildSuggestionInput, IframeHit, JsCall, Resource, StickyHit, Suggestion, SuggestionLayer,
};

use crate::allowlist::is_legit_hidden_iframe;
use crate::canon::canonicalize_url;
use crate::learn::LearnKind;
use crate::stack::script_origin_from_stack;
use crate::suggestion::build_suggestion;

/// Opaque per-detect context the orchestrator fills in once and passes
/// to each detector. Bundling this keeps every detector signature narrow.
pub(crate) struct DetectCtx<'a> {
    pub hostname: &'a str,
    pub matched_key: Option<&'a str>,
    pub config_has_site: bool,
    pub existing_block: &'a [String],
    pub existing_remove: &'a [String],
    pub existing_hide: &'a [String],
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
    fn finish(&self, input: BuildSuggestionInput) -> Suggestion {
        build_suggestion(&input)
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
    let mut by_host: HashMap<&str, Vec<&Resource>> = HashMap::new();
    for r in resources {
        if r.initiator_type != "beacon" {
            continue;
        }
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        by_host.entry(&r.host).or_default().push(r);
    }
    let mut out = Vec::new();
    for (host, hits) in by_host {
        let value = format!("||{host}");
        if ctx.has_block(&value) {
            continue;
        }
        let from_frame = first_non_top_frame(hits.iter().copied(), ctx.hostname);
        let plural = if hits.len() > 1 { "s" } else { "" };
        out.push(ctx.finish(BuildSuggestionInput {
            key: format!("block::{value}"),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!("sendBeacon target ({} beacon{} sent)", hits.len(), plural),
            confidence: 95,
            count: hits.len() as u32,
            evidence: hits.iter().take(5).map(|h| h.url.clone()).collect(),
            from_frame,
            learn: LearnKind::Beacon.text().to_string(),
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
    }
    out
}

/// Detector 2: tracking pixels. Third-party `<img>` responses under
/// 200 bytes are the classic 1x1 pixel pattern.
pub(crate) fn detect_pixels(ctx: &DetectCtx, resources: &[Resource]) -> Vec<Suggestion> {
    let mut by_host: HashMap<&str, Vec<&Resource>> = HashMap::new();
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
    let mut out = Vec::new();
    for (host, hits) in by_host {
        let value = format!("||{host}");
        if ctx.has_block(&value) {
            continue;
        }
        let med = median(hits.iter().map(|h| h.transfer_size).collect());
        let from_frame = first_non_top_frame(hits.iter().copied(), ctx.hostname);
        let plural = if hits.len() > 1 { "s" } else { "" };
        out.push(ctx.finish(BuildSuggestionInput {
            key: format!("block::{value}"),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "tracking pixels: {} tiny image{} (median {}b)",
                hits.len(),
                plural,
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
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
    let mut by_host: HashMap<&str, Vec<&Resource>> = HashMap::new();
    for r in resources {
        if r.host.is_empty() || r.host == ctx.hostname {
            continue;
        }
        if !is_subdomain_of(&r.host, ctx.hostname) {
            continue;
        }
        by_host.entry(&r.host).or_default().push(r);
    }
    let mut out = Vec::new();
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
        let value = format!("||{host}");
        if ctx.has_block(&value) {
            continue;
        }
        let from_frame = first_non_top_frame(requests.iter().copied(), ctx.hostname);
        let plural = if requests.len() > 1 { "s" } else { "" };
        out.push(ctx.finish(BuildSuggestionInput {
            key: format!("block::{value}"),
            layer: SuggestionLayer::Block,
            value: value.clone(),
            reason: format!(
                "first-party subdomain with {} tiny response{} (median {}b)",
                requests.len(),
                plural,
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
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
    let mut by_canon: HashMap<String, PollEntry> = HashMap::new();
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
    let mut out = Vec::new();
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
        let value = format!("||{}^", info.host);
        if ctx.has_block(&value) {
            continue;
        }
        let key = format!("block::{value}");
        if out.iter().any(|s: &Suggestion| s.key == key) {
            continue;
        }
        out.push(ctx.finish(BuildSuggestionInput {
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
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
    let mut by_host: HashMap<&str, IframeInfo> = HashMap::new();
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
    let mut out = Vec::new();
    for (host, info) in by_host {
        let selector = format!("iframe[src*=\"{host}\"]");
        if ctx.has_remove(&selector) {
            continue;
        }
        let from_frame = first_non_top_frame(info.samples.iter().copied(), ctx.hostname);
        let reason_list: Vec<String> = info.reasons.into_iter().collect();
        out.push(ctx.finish(BuildSuggestionInput {
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
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
    let mut out = Vec::new();
    for s in stickies {
        if s.selector.is_empty() {
            continue;
        }
        if !seen.insert(s.selector.clone()) {
            continue;
        }
        if ctx.has_hide(&s.selector) {
            continue;
        }
        let from_frame = s
            .reporter_frame
            .as_deref()
            .filter(|f| !f.is_empty() && *f != ctx.hostname)
            .map(str::to_string);
        out.push(ctx.finish(BuildSuggestionInput {
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
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
    let value = format!("||{origin}");
    if ctx.has_block(&value) {
        return;
    }
    out.push(ctx.finish(BuildSuggestionInput {
        key: format!("block::{value}::{kind_tag}"),
        layer: SuggestionLayer::Block,
        value: value.clone(),
        reason,
        confidence,
        count: 1,
        evidence: vec![],
        from_frame: None,
        learn: learn.text().to_string(),
        tab_hostname: ctx.hostname.to_string(),
        matched_key: ctx.matched_key.map(str::to_string),
        config_has_site: ctx.config_has_site,
        existing_block: ctx.existing_block.to_vec(),
        existing_remove: ctx.existing_remove.to_vec(),
        existing_hide: ctx.existing_hide.to_vec(),
    }));
}

/// Shared aggregation pass over js_calls; every main-world detector
/// reads from this. Keeps a single O(n) loop rather than one per
/// detector.
struct JsCallSummary {
    seconds_since_first: i64,
    origins_by_kind: HashMap<String, HashMap<String, u32>>,
    hot_params_by_origin: HashMap<String, u32>,
    distinct_fonts_by_origin: HashMap<String, std::collections::BTreeSet<String>>,
    listener_types_by_origin: HashMap<String, ListenerInfo>,
    replay_vendors: HashMap<String, u32>,
    raf_waste_by_key: HashMap<String, RafWasteEntry>,
}

struct ListenerInfo {
    count: u32,
    types: std::collections::BTreeSet<String>,
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

    let mut origins_by_kind: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut hot_params_by_origin: HashMap<String, u32> = HashMap::new();
    let mut distinct_fonts_by_origin: HashMap<String, std::collections::BTreeSet<String>> =
        HashMap::new();
    let mut listener_types_by_origin: HashMap<String, ListenerInfo> = HashMap::new();
    let mut replay_vendors: HashMap<String, u32> = HashMap::new();
    let mut raf_waste_by_key: HashMap<String, RafWasteEntry> = HashMap::new();

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
                        let entry = listener_types_by_origin
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
    let mut out = Vec::new();

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
    for (origin, &hot_count) in &s.hot_params_by_origin {
        if hot_count >= 1 {
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                "WebGL fingerprinting (read UNMASKED_RENDERER_WEBGL or _VENDOR_WEBGL)".to_string(),
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
        let value = format!("||{vendor_host}");
        if ctx.has_block(&value) {
            continue;
        }
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
        out.push(ctx.finish(BuildSuggestionInput {
            key: format!("block::{value}::replay-{vendor}"),
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
    }

    // Listener density: 12+ interaction listeners from one origin in <60s.
    for (origin, info) in &s.listener_types_by_origin {
        if info.count >= 12 && info.types.len() >= 3 && s.seconds_since_first < 60 {
            let types: Vec<&str> = info.types.iter().map(String::as_str).collect();
            emit_origin_block(
                ctx,
                &mut out,
                origin,
                format!(
                    "session replay pattern ({} interaction listeners attached: {})",
                    info.count,
                    types.join(", ")
                ),
                80,
                "listener-density",
                LearnKind::ReplayListener,
            );
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
        let value = format!("||{}", entry.origin);
        if ctx.has_block(&value) {
            continue;
        }
        let seconds = ((span / 1000) as u32).max(1);
        let rate_per_sec = (entry.invisible as f64 / seconds as f64 * 10.0).round() / 10.0;
        out.push(ctx.finish(BuildSuggestionInput {
            key: format!(
                "block::{value}::raf-waste::{}",
                entry.canvas_sel
            ),
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
            tab_hostname: ctx.hostname.to_string(),
            matched_key: ctx.matched_key.map(str::to_string),
            config_has_site: ctx.config_has_site,
            existing_block: ctx.existing_block.to_vec(),
            existing_remove: ctx.existing_remove.to_vec(),
            existing_hide: ctx.existing_hide.to_vec(),
        }));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hush_types::{ReplayVendor, StickyRect};

    fn ctx<'a>(hostname: &'a str) -> DetectCtx<'a> {
        DetectCtx {
            hostname,
            matched_key: None,
            config_has_site: false,
            existing_block: &[],
            existing_remove: &[],
            existing_hide: &[],
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
