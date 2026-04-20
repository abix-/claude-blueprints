//! Top-level suggestion orchestrator.
//!
//! [`compute_suggestions`] is the single entry point that the service
//! worker will call (via WASM). It runs every detector, applies the
//! user's per-tab-session dismissal set + the cross-session allowlist,
//! and returns suggestions sorted by confidence then by count.

use crate::types::{Allowlist, BehaviorState, Config, SiteConfig, Suggestion};

use crate::detectors::{
    detect_beacon, detect_first_party_telemetry, detect_from_js_calls, detect_hidden_iframes,
    detect_pixels, detect_polling, detect_sticky_overlays, DetectCtx,
};

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
    let (matched_key, cfg): (Option<&str>, Option<&SiteConfig>) = match matched {
        Some((k, c)) => (Some(k.as_str()), Some(c)),
        None => (None, None),
    };

    let existing_block = normalize_block_patterns(cfg.map(|c| c.block.as_slice()).unwrap_or(&[]));
    let existing_remove: Vec<String> =
        cfg.map(|c| c.remove.clone()).unwrap_or_default();
    let existing_hide: Vec<String> = cfg.map(|c| c.hide.clone()).unwrap_or_default();

    let ctx = DetectCtx {
        hostname,
        matched_key,
        config_has_site: matched_key.is_some(),
        existing_block: &existing_block,
        existing_remove: &existing_remove,
        existing_hide: &existing_hide,
    };

    let mut out = Vec::new();
    out.extend(detect_beacon(&ctx, &state.seen_resources));
    out.extend(detect_pixels(&ctx, &state.seen_resources));
    out.extend(detect_first_party_telemetry(&ctx, &state.seen_resources));
    out.extend(detect_polling(&ctx, &state.seen_resources));
    out.extend(detect_hidden_iframes(
        &ctx,
        &state.latest_iframes,
        &allowlist.iframes,
    ));
    out.extend(detect_from_js_calls(&ctx, &state.js_calls));
    out.extend(detect_sticky_overlays(&ctx, &state.latest_stickies));

    apply_filters_and_sort(out, &state.dismissed, &allowlist.suggestions)
}

fn apply_filters_and_sort(
    mut out: Vec<Suggestion>,
    dismissed: &[String],
    allowlist_suggestions: &[String],
) -> Vec<Suggestion> {
    let dismissed_set: std::collections::HashSet<&str> =
        dismissed.iter().map(String::as_str).collect();
    let allow_set: std::collections::HashSet<&str> =
        allowlist_suggestions.iter().map(String::as_str).collect();
    out.retain(|s| !dismissed_set.contains(s.key.as_str()));
    out.retain(|s| !allow_set.contains(s.key.as_str()));
    // Higher confidence first, then higher count.
    out.sort_by(|a, b| {
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
fn normalize_block_patterns(raw: &[String]) -> Vec<String> {
    raw.iter()
        .map(|p| {
            if let Some(stripped) = p.strip_suffix('^') {
                stripped.to_string()
            } else {
                p.clone()
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
