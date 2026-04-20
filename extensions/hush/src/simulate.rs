//! Rule simulator / test-match.
//!
//! Given a URL and the user's active config, enumerate every rule
//! whose match pattern fires and mark the winner per the evaluator's
//! semantics (allow > block at the same DNR priority; neuter and
//! silence match on host only and have no priority-vs-block
//! interaction — they report independently).
//!
//! The simulator is a read-only audit tool: it inspects config and
//! returns what WOULD happen, without firing any rule. It's the
//! firewall equivalent of a "test security policy match" dialog.
//!
//! Pattern semantics intentionally mirror what
//! `chrome.declarativeNetRequest`'s `urlFilter` does for block /
//! allow, plus the uBlock-style `||host^` shape documented in
//! `docs/architecture.md`. Full DNR parity (wildcards, path anchors)
//! is out of scope for the MVP — the cases users actually audit are
//! host-prefix and path-prefix.
//!
//! Nothing here is WASM-specific. Pure Rust with exhaustive unit
//! tests; the options-page UI imports through the wasm-bindgen shim
//! in `lib.rs`.

use crate::types::{rule_id, Config, RuleEntry, GLOBAL_SCOPE_KEY};
use serde::Serialize;

/// One rule that matched the simulator input. The `priority` field
/// reflects how DNR would resolve the URL: allow = 2, block = 1,
/// neuter/silence = 0 (non-network dimension; reported alongside but
/// not in the DNR winner calculation).
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct RuleMatch {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub scope: String,
    pub action: String,
    pub value: String,
    pub priority: u32,
    #[serde(rename = "isWinner")]
    pub is_winner: bool,
    #[serde(default)]
    pub disabled: bool,
}

/// Simulate a URL against the user's config. Walks global + site-
/// scoped block / allow / neuter / silence rules, returns every
/// matching rule with a `is_winner` flag on the DNR-resolved
/// winner (allow beats block at the same priority). Neuter and
/// silence matches are reported alongside but don't compete with
/// block/allow for the winner crown — they're a different dimension
/// (stack-origin vs. request URL).
///
/// `site_host` is the hostname the user is simulating "as if I
/// loaded a page on THIS site". Used to pick the site-scoped
/// SiteConfig (or none). Pass an empty string to simulate against
/// the global scope only.
pub fn simulate_url(config: &Config, site_host: &str, url: &str) -> Vec<RuleMatch> {
    let mut out: Vec<RuleMatch> = Vec::new();
    let url_trimmed = url.trim();
    if url_trimmed.is_empty() {
        return out;
    }

    let site_key = resolve_site_key(config, site_host);

    // Walk global and site configs; emit one RuleMatch per matching
    // rule.  Scope label is the authoring key (global or site).
    let scopes: &[(&str, Option<&str>)] = &[
        (GLOBAL_SCOPE_KEY, Some(GLOBAL_SCOPE_KEY)),
        ("<site>", site_key.as_deref()),
    ];
    for (label, key) in scopes {
        let Some(key) = key else {
            continue;
        };
        let Some(cfg) = config.get(*key) else {
            continue;
        };
        let scope_label = if *label == "<site>" {
            key.to_string()
        } else {
            (*label).to_string()
        };
        // block (priority 1) + allow (priority 2) — both operate on
        // the full URL.
        for entry in &cfg.block {
            if url_filter_matches(&entry.value, url_trimmed) {
                out.push(build_match(&scope_label, "block", entry, 1));
            }
        }
        for entry in &cfg.allow {
            if url_filter_matches(&entry.value, url_trimmed) {
                out.push(build_match(&scope_label, "allow", entry, 2));
            }
        }
        // neuter + silence — operate on the initiating-script
        // origin, which for the simulator means the host of the
        // URL under test. Priority 0 so they don't compete with
        // block/allow for the DNR winner.
        for entry in &cfg.neuter {
            if url_filter_matches(&entry.value, url_trimmed) {
                out.push(build_match(&scope_label, "neuter", entry, 0));
            }
        }
        for entry in &cfg.silence {
            if url_filter_matches(&entry.value, url_trimmed) {
                out.push(build_match(&scope_label, "silence", entry, 0));
            }
        }
    }

    // Winner resolution (DNR dimension only: block + allow). Highest
    // priority wins; ties go to allow (mirrors DNR's action-priority
    // resolution: allow > block).
    let mut best: Option<usize> = None;
    let mut best_priority: u32 = 0;
    let mut best_is_allow = false;
    for (i, m) in out.iter().enumerate() {
        if m.disabled {
            continue;
        }
        if m.action != "block" && m.action != "allow" {
            continue;
        }
        let is_allow = m.action == "allow";
        let better = match best {
            None => true,
            Some(_) => {
                m.priority > best_priority
                    || (m.priority == best_priority && is_allow && !best_is_allow)
            }
        };
        if better {
            best = Some(i);
            best_priority = m.priority;
            best_is_allow = is_allow;
        }
    }
    if let Some(idx) = best {
        out[idx].is_winner = true;
    }

    out
}

fn build_match(scope: &str, action: &str, entry: &RuleEntry, priority: u32) -> RuleMatch {
    RuleMatch {
        rule_id: rule_id(action, scope, &entry.value),
        scope: scope.to_string(),
        action: action.to_string(),
        value: entry.value.clone(),
        priority,
        is_winner: false,
        disabled: entry.disabled,
    }
}

/// Pick the site-scope key that would apply for `site_host`. Mirrors
/// `compute::find_config_entry` — exact match wins, otherwise a
/// suffix-matching entry.
fn resolve_site_key(config: &Config, site_host: &str) -> Option<String> {
    if site_host.is_empty() {
        return None;
    }
    if config.contains_key(site_host) {
        return Some(site_host.to_string());
    }
    for key in config.keys() {
        if key == GLOBAL_SCOPE_KEY {
            continue;
        }
        if site_host == key.as_str() || site_host.ends_with(&format!(".{key}")) {
            return Some(key.clone());
        }
    }
    None
}

/// uBlock-style URL-filter match. Supports the shapes the editor
/// produces today:
///
/// - `||host[/path][^]` — anchored on domain: host must exactly equal
///   `host` OR be a subdomain of it; optional path prefix after the
///   host; optional `^` boundary treated as end-of-host or `/`.
/// - bare `substring` — substring-matched anywhere in the URL.
///
/// Not supported (MVP): `*` wildcards, `|` terminators, regex
/// alternation. Add when user-demand hits a gap.
pub fn url_filter_matches(pattern: &str, url: &str) -> bool {
    let pat = pattern.trim();
    let url = url.trim();
    if pat.is_empty() || url.is_empty() {
        return false;
    }

    // Parse host + path from the URL. Fall back to empty host on
    // parse failure so bare-substring patterns still apply.
    let (url_host, url_path) = split_url_host_path(url);

    if let Some(rest) = pat.strip_prefix("||") {
        // `||host[/path][^]` — split rest at first `/` or `^`.
        let (host_part, tail) = split_anchored_pattern(rest);
        if host_part.is_empty() {
            return false;
        }
        if !host_matches(&url_host, host_part) {
            return false;
        }
        if tail.is_empty() {
            return true;
        }
        // Tail is the path segment plus optional trailing `^`.
        let (tail_path, caret) = if let Some(stripped) = tail.strip_suffix('^') {
            (stripped, true)
        } else {
            (tail, false)
        };
        if tail_path.is_empty() && caret {
            // `||host^` — matches any URL on that host (we already
            // confirmed the host). Caret only enforces a boundary,
            // which the host-match already gave us.
            return true;
        }
        if !url_path.starts_with(tail_path) {
            return false;
        }
        if caret {
            let after = &url_path[tail_path.len()..];
            // Boundary = end of URL, `/`, `?`, `#`, or `^` chars.
            let boundary_ok = after.is_empty()
                || after.starts_with('/')
                || after.starts_with('?')
                || after.starts_with('#');
            if !boundary_ok {
                return false;
            }
        }
        return true;
    }

    // Bare substring.
    url.contains(pat)
}

fn split_url_host_path(url: &str) -> (String, String) {
    // Strip scheme://.
    let no_scheme = url
        .find("://")
        .map(|i| &url[i + 3..])
        .unwrap_or(url);
    match no_scheme.find('/') {
        Some(i) => (no_scheme[..i].to_string(), no_scheme[i..].to_string()),
        None => (no_scheme.to_string(), String::new()),
    }
}

fn split_anchored_pattern(rest: &str) -> (&str, &str) {
    // Find the first `/` or `^` that ends the host portion.
    for (i, c) in rest.char_indices() {
        if c == '/' || c == '^' {
            return (&rest[..i], &rest[i..]);
        }
    }
    (rest, "")
}

fn host_matches(url_host: &str, pat_host: &str) -> bool {
    if url_host == pat_host {
        return true;
    }
    url_host.ends_with(&format!(".{pat_host}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SiteConfig;

    fn host_only(pat: &str, url: &str) -> bool {
        url_filter_matches(pat, url)
    }

    #[test]
    fn anchored_host_exact_match() {
        assert!(host_only("||doubleclick.net", "https://doubleclick.net/"));
    }

    #[test]
    fn anchored_host_subdomain_match() {
        assert!(host_only("||doubleclick.net", "https://ads.doubleclick.net/"));
    }

    #[test]
    fn anchored_host_unrelated_does_not_match() {
        assert!(!host_only("||doubleclick.net", "https://example.com/"));
        assert!(!host_only("||doubleclick.net", "https://notdoubleclick.net/"));
    }

    #[test]
    fn anchored_host_with_path_prefix() {
        assert!(host_only(
            "||doubleclick.net/adx/",
            "https://doubleclick.net/adx/ad?id=1"
        ));
        assert!(!host_only(
            "||doubleclick.net/adx/",
            "https://doubleclick.net/other"
        ));
    }

    #[test]
    fn anchored_host_with_caret_boundary() {
        // `||foo.com^` should match any URL on foo.com.
        assert!(host_only("||foo.com^", "https://foo.com/"));
        assert!(host_only("||foo.com^", "https://foo.com/any/path"));
        assert!(host_only("||foo.com^", "https://a.foo.com/x"));
        assert!(!host_only("||foo.com^", "https://notfoo.com/"));
    }

    #[test]
    fn bare_substring_matches_anywhere_in_url() {
        assert!(host_only("tracker", "https://example.com/path?t=tracker"));
        assert!(!host_only("tracker", "https://example.com/"));
    }

    #[test]
    fn empty_pattern_never_matches() {
        assert!(!host_only("", "https://example.com/"));
    }

    #[test]
    fn simulate_url_reports_block_and_allow_allow_wins() {
        // Global block + site-scoped allow — allow should win via
        // priority.
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec![RuleEntry::new("||doubleclick.net")],
                ..Default::default()
            },
        );
        cfg.insert(
            "site.test".into(),
            SiteConfig {
                allow: vec![RuleEntry::new("||doubleclick.net/adx/")],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "site.test", "https://doubleclick.net/adx/ad");
        assert_eq!(matches.len(), 2);
        let winner = matches.iter().find(|m| m.is_winner).unwrap();
        assert_eq!(winner.action, "allow");
        assert_eq!(winner.scope, "site.test");
    }

    #[test]
    fn simulate_url_only_global_block_that_fires() {
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec![RuleEntry::new("||doubleclick.net")],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "site.test", "https://doubleclick.net/any");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, "block");
        assert!(matches[0].is_winner);
    }

    #[test]
    fn simulate_url_no_matches_returns_empty() {
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec![RuleEntry::new("||foo.com")],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "site.test", "https://bar.com/path");
        assert!(matches.is_empty());
    }

    #[test]
    fn simulate_url_reports_neuter_without_winner_flag() {
        // Neuter is a different dimension (stack-origin) than DNR
        // block/allow. It should surface in the match list so the
        // user sees it would fire, but not claim winner over an
        // absent block/allow.
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                neuter: vec![RuleEntry::new("||hotjar.com")],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "site.test", "https://hotjar.com/script.js");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].action, "neuter");
        assert!(!matches[0].is_winner);
    }

    #[test]
    fn simulate_url_disabled_rule_is_reported_but_not_winner() {
        let mut cfg = Config::new();
        cfg.insert(
            GLOBAL_SCOPE_KEY.into(),
            SiteConfig {
                block: vec![RuleEntry {
                    value: "||doubleclick.net".into(),
                    disabled: true,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "", "https://doubleclick.net/x");
        assert_eq!(matches.len(), 1);
        assert!(matches[0].disabled);
        assert!(!matches[0].is_winner);
    }

    #[test]
    fn simulate_url_site_scope_suffix_match() {
        // User simulates on "m.site.test"; a rule authored under
        // "site.test" should still apply via suffix match.
        let mut cfg = Config::new();
        cfg.insert(
            "site.test".into(),
            SiteConfig {
                block: vec![RuleEntry::new("||tracker.test")],
                ..Default::default()
            },
        );
        let matches = simulate_url(&cfg, "m.site.test", "https://tracker.test/x");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].scope, "site.test");
    }
}
