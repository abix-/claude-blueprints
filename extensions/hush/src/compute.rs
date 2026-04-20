//! Top-level suggestion orchestrator.
//!
//! [`compute_suggestions`] is the single entry point that the service
//! worker will call (via WASM). It runs every detector, applies the
//! user's per-tab-session dismissal set + the cross-session allowlist,
//! and returns suggestions sorted by confidence then by count.

use std::sync::Arc;

use crate::types::{Allowlist, BehaviorState, Config, SiteConfig, Suggestion};

use crate::detectors::{DetectCtx, DETECTORS};

/// Run every detector against `state` + `config`, applying dismissals
/// and the allowlist. This is the direct Rust port of the old JS
/// `computeSuggestions(state, config)` plus the cross-session
/// `allowlist.suggestions` filter that now lives in the allowlist arg.
pub fn compute_suggestions(
    state: &BehaviorState,
    config: &Config,
    allowlist: &Allowlist,
) -> Vec<Suggestion> {
    let hostname = state.page_host.as_deref().unwrap_or("");
    if hostname.is_empty() {
        return Vec::new();
    }

    let matched = find_config_entry(config, hostname);
    let matched_key: Option<&str> = matched.as_ref().map(|(k, _)| k.as_str());

    // Dedup against BOTH the site-matched rules AND the reserved
    // `__global__` scope, because a rule under `__global__` still
    // fires on every tab. Without this merge, a user-authored
    // global block rule wouldn't suppress a matching suggestion.
    let merged = crate::types::merged_site_config(
        config,
        matched_key.unwrap_or(crate::types::GLOBAL_SCOPE_KEY),
    );

    // Allocate the three existing-rule lists once per compute_suggestions
    // call. `Arc<[String]>` means every detector's emit-time clone is a
    // refcount bump (2 instructions) rather than a Vec data copy.
    // Across a heavy_tab run with ~30 suggestions that's ~90 heap
    // allocations avoided.
    // Dedup only against ENABLED rules. A disabled rule is parked —
    // the user wants the detector to keep surfacing suggestions for
    // that match so the "what gets caught if I turn this back on?"
    // workflow stays live.
    let existing_block: Arc<[String]> =
        Arc::from(normalize_block_patterns(&merged.block));
    let existing_remove: Arc<[String]> = Arc::from(
        merged
            .remove
            .iter()
            .filter(|e| !e.disabled)
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
    );
    let existing_hide: Arc<[String]> = Arc::from(
        merged
            .hide
            .iter()
            .filter(|e| !e.disabled)
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
    );
    let existing_neuter: Arc<[String]> = Arc::from(
        merged
            .neuter
            .iter()
            .filter(|e| !e.disabled)
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
    );
    let existing_silence: Arc<[String]> = Arc::from(
        merged
            .silence
            .iter()
            .filter(|e| !e.disabled)
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
    );
    let existing_spoof: Arc<[String]> = Arc::from(
        merged
            .spoof
            .iter()
            .filter(|e| !e.disabled)
            .map(|e| e.value.clone())
            .collect::<Vec<_>>(),
    );

    let ctx = DetectCtx {
        hostname,
        matched_key,
        config_has_site: matched_key.is_some(),
        existing_block,
        existing_remove,
        existing_hide,
        existing_neuter,
        existing_silence,
        existing_spoof,
    };

    // Heavy tabs typically emit 20-40 suggestions; pre-sizing avoids
    // the first ~4 Vec growth reallocs as each detector extends `out`.
    let mut out = Vec::with_capacity(64);
    for detector in DETECTORS {
        out.extend(detector.detect(&ctx, state, allowlist));
    }

    apply_filters_and_sort(out, &state.dismissed, &allowlist.suggestions)
}

fn apply_filters_and_sort(
    mut out: Vec<Suggestion>,
    dismissed: &[String],
    allowlist_suggestions: &[String],
) -> Vec<Suggestion> {
    // Swiss-table + foldhash sets for O(1) membership checks. Typical
    // dismissed + allowlist sizes are <100 each so capacity hint is
    // small; foldhash's string-key throughput is what matters.
    let hasher = foldhash::fast::RandomState::default();
    let mut dismissed_set: std::collections::HashSet<&str, foldhash::fast::RandomState> =
        std::collections::HashSet::with_capacity_and_hasher(dismissed.len(), hasher.clone());
    for s in dismissed {
        dismissed_set.insert(s.as_str());
    }
    let mut allow_set: std::collections::HashSet<&str, foldhash::fast::RandomState> =
        std::collections::HashSet::with_capacity_and_hasher(allowlist_suggestions.len(), hasher);
    for s in allowlist_suggestions {
        allow_set.insert(s.as_str());
    }
    out.retain(|s| !dismissed_set.contains(s.key.as_str()) && !allow_set.contains(s.key.as_str()));
    // Higher confidence first, then higher count. sort_unstable is safe
    // here because Suggestion has no semantic "equal" ordering beyond
    // confidence+count - any ordering among ties is acceptable.
    out.sort_unstable_by(|a, b| {
        b.confidence
            .cmp(&a.confidence)
            .then(b.count.cmp(&a.count))
    });
    out
}

/// Find the site-config entry whose key equals or is a suffix of
/// `host`. Mirrors the JS `findConfigEntry` behavior.
fn find_config_entry<'a>(config: &'a Config, host: &str) -> Option<(&'a String, &'a SiteConfig)> {
    if let Some(exact) = config.get_key_value(host) {
        return Some(exact);
    }
    for (k, v) in config {
        if host == k.as_str() || host.ends_with(&format!(".{k}")) {
            return Some((k, v));
        }
    }
    None
}

/// Normalize block patterns by stripping trailing `^` so dedup compares
/// the same canonical form as the suggestion builders emit. Parity with
/// the JS `existingBlock` Set construction in the old computeSuggestions.
fn normalize_block_patterns(raw: &[crate::types::RuleEntry]) -> Vec<String> {
    raw.iter()
        .filter(|e| !e.disabled)
        .map(|e| {
            if let Some(stripped) = e.value.strip_suffix('^') {
                stripped.to_string()
            } else {
                e.value.clone()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{IframeHit, JsCall, ReplayVendor, Resource};

    fn state(hostname: &str) -> BehaviorState {
        BehaviorState {
            page_host: Some(hostname.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn no_page_host_returns_empty() {
        let s = BehaviorState::default();
        let out = compute_suggestions(&s, &Config::new(), &Allowlist::default());
        assert!(out.is_empty());
    }

    #[test]
    fn beacon_and_pixel_run_together_sorted_by_confidence() {
        let mut s = state("site.test");
        s.seen_resources = vec![
            Resource {
                url: "https://t1.test/p".into(),
                host: "t1.test".into(),
                initiator_type: "beacon".into(),
                transfer_size: 0,
                duration: 0,
                start_time: 0,
                reporter_frame: None,
            },
            Resource {
                url: "https://t2.test/p.gif".into(),
                host: "t2.test".into(),
                initiator_type: "img".into(),
                transfer_size: 43,
                duration: 0,
                start_time: 0,
                reporter_frame: None,
            },
            Resource {
                url: "https://t2.test/q.gif".into(),
                host: "t2.test".into(),
                initiator_type: "img".into(),
                transfer_size: 43,
                duration: 0,
                start_time: 0,
                reporter_frame: None,
            },
        ];
        let out = compute_suggestions(&s, &Config::new(), &Allowlist::default());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].confidence, 95); // beacon wins
        assert_eq!(out[1].confidence, 85); // pixel
    }

    #[test]
    fn suggestion_allowlist_filters_matching_keys() {
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let allow = Allowlist {
            iframes: vec![],
            overlays: vec![],
            suggestions: vec!["block::||t.test".into()],
        };
        let out = compute_suggestions(&s, &Config::new(), &allow);
        assert!(out.is_empty(), "allowlist entry filters the suggestion");
    }

    #[test]
    fn dismissed_keys_are_filtered() {
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        s.dismissed = vec!["block::||t.test".into()];
        let out = compute_suggestions(&s, &Config::new(), &Allowlist::default());
        assert!(out.is_empty());
    }

    #[test]
    fn existing_block_rule_suppresses_suggestion() {
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let mut config = Config::new();
        config.insert(
            "site.test".into(),
            SiteConfig {
                block: vec!["||t.test".into()],
                ..Default::default()
            },
        );
        let out = compute_suggestions(&s, &config, &Allowlist::default());
        assert!(out.is_empty(), "existing block rule dedups");
    }

    #[test]
    fn global_scope_block_rule_suppresses_suggestion() {
        // A block rule under the reserved `__global__` scope must
        // dedup a matching suggestion even when the tab's hostname
        // doesn't have a site-specific entry. Without the merge in
        // compute_suggestions, this regresses: the suggestion fires
        // every scan because the dedup only reads the site-scoped
        // block list.
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let mut config = Config::new();
        config.insert(
            crate::types::GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec!["||t.test".into()],
                ..Default::default()
            },
        );
        let out = compute_suggestions(&s, &config, &Allowlist::default());
        assert!(
            out.is_empty(),
            "global-scope block rule dedups suggestions"
        );
    }

    #[test]
    fn trailing_caret_tolerated_in_existing_block() {
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let mut config = Config::new();
        config.insert(
            "site.test".into(),
            SiteConfig {
                block: vec!["||t.test^".into()],
                ..Default::default()
            },
        );
        let out = compute_suggestions(&s, &config, &Allowlist::default());
        assert!(
            out.is_empty(),
            "caret-suffixed and unsuffixed forms dedup against each other"
        );
    }

    #[test]
    fn disabled_block_rule_does_not_suppress_suggestion() {
        // A disabled rule is parked — the detector should keep
        // surfacing the suggestion so the user can see what it
        // would catch if flipped back on. Regression lock for
        // Stage 9 phase 4 per-rule disable.
        use crate::types::RuleEntry;
        let mut s = state("site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let mut config = Config::new();
        config.insert(
            "site.test".into(),
            SiteConfig {
                block: vec![RuleEntry {
                    value: "||t.test".into(),
                    disabled: true,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let out = compute_suggestions(&s, &config, &Allowlist::default());
        assert_eq!(
            out.len(),
            1,
            "disabled block rule must NOT dedup the suggestion"
        );
    }

    #[test]
    fn subdomain_of_site_matches_site_config() {
        let mut s = state("m.site.test");
        s.seen_resources = vec![Resource {
            url: "https://t.test/p".into(),
            host: "t.test".into(),
            initiator_type: "beacon".into(),
            transfer_size: 0,
            duration: 0,
            start_time: 0,
            reporter_frame: None,
        }];
        let mut config = Config::new();
        config.insert(
            "site.test".into(),
            SiteConfig {
                block: vec!["||t.test".into()],
                ..Default::default()
            },
        );
        let out = compute_suggestions(&s, &config, &Allowlist::default());
        assert!(out.is_empty(), "m.site.test tab should match site.test config");
    }

    #[test]
    fn iframe_allowlist_applied_at_detector_not_orchestrator() {
        let mut s = state("site.test");
        s.latest_iframes = vec![IframeHit {
            src: "https://captcha.new/widget".into(),
            host: "captcha.new".into(),
            reasons: vec!["1x1 size".into()],
            width: 1,
            height: 1,
            outer_html_preview: String::new(),
            reporter_frame: None,
        }];
        let allow = Allowlist {
            iframes: vec!["captcha.new".into()],
            overlays: vec![],
            suggestions: vec![],
        };
        let out = compute_suggestions(&s, &Config::new(), &allow);
        assert!(out.is_empty(), "iframe in allowlist should never surface");
    }

    #[test]
    fn end_to_end_canvas_fp_plus_replay_vendor_both_fire() {
        let mut s = state("site.test");
        s.js_calls = vec![
            JsCall {
                kind: "canvas-fp".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
                ..Default::default()
            },
            JsCall {
                kind: "canvas-fp".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
                ..Default::default()
            },
            JsCall {
                kind: "canvas-fp".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                stack: vec!["at fp (https://fp.test/a.js:1:1)".into()],
                ..Default::default()
            },
            JsCall {
                kind: "replay-global".into(),
                t: "2026-04-19T12:00:00.000Z".into(),
                vendors: vec![ReplayVendor {
                    key: "_hjSettings".into(),
                    vendor: "Hotjar".into(),
                }],
                ..Default::default()
            },
        ];
        let out = compute_suggestions(&s, &Config::new(), &Allowlist::default());
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].confidence, 95);
        assert!(out[0].value.contains("hotjar"));
        assert_eq!(out[1].confidence, 90);
    }
}
