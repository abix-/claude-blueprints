//! Rule-health analysis.
//!
//! Pure functions over `SiteConfig` / `RuleEntry` that surface the
//! three rule-health signals Stage 12 targets:
//!
//! - **Shadowed block rules**: an allow rule whose URL filter covers
//!   everything the block rule covers, making the block rule
//!   unreachable for DNR.
//! - **Dead rules**: configured rules that produced no `FirewallEvent`
//!   over the log window. Not computed here — the popup derives this
//!   from the firewall log directly, since "dead" is a property of
//!   runtime observation, not static config.
//! - **Zero-match selectors**: remove/hide selectors whose on-page
//!   match count is zero. Also runtime-derived; lives in the popup.
//!
//! Everything in this module is deterministic, Send, and cheaply
//! testable without a WASM harness.
//!
//! Pattern equivalence follows uBlock-style surface syntax: we strip
//! the `||` anchor prefix and the trailing `^` boundary marker before
//! comparing. Pattern-shape aware analysis (wildcards, path specifics)
//! would need full URL-filter parsing; the prefix heuristic here
//! catches the common "generic allow > specific block" case without
//! pulling in an abnf parser.

use crate::types::RuleEntry;

/// Canonical form of a URL filter for comparison. Strips `||` prefix
/// and `^` suffix. Returns a `&str` that borrows from `pat`.
fn normalize_pattern(pat: &str) -> &str {
    let s = pat.strip_prefix("||").unwrap_or(pat);
    s.strip_suffix('^').unwrap_or(s)
}

/// Does the given `block_pattern` have a shadowing allow rule in
/// `allow_rules`? Returns the first such entry; later allows only
/// matter for the rule-count display and don't affect DNR semantics
/// (any shadow makes the block rule unreachable).
///
/// Heuristic: an allow pattern shadows the block pattern if its
/// normalized form is a prefix of the block's normalized form.
/// Example: allow `||doubleclick.net` shadows block
/// `||doubleclick.net/adx/`. A narrower allow (e.g. `||dc.net/adx/`)
/// does NOT shadow a broader block (`||dc.net`) — it creates an
/// exception, not a shadow.
///
/// Disabled allow rules are ignored — a parked rule can't shadow.
pub fn block_shadowed_by<'a>(
    allow_rules: &'a [RuleEntry],
    block_pattern: &str,
) -> Option<&'a RuleEntry> {
    let bp = normalize_pattern(block_pattern);
    if bp.is_empty() {
        return None;
    }
    for a in allow_rules {
        if a.disabled {
            continue;
        }
        let ap = normalize_pattern(&a.value);
        if !ap.is_empty() && bp.starts_with(ap) {
            return Some(a);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allow(v: &str) -> RuleEntry {
        RuleEntry::new(v)
    }
    fn disabled_allow(v: &str) -> RuleEntry {
        RuleEntry {
            value: v.into(),
            disabled: true,
            ..Default::default()
        }
    }

    #[test]
    fn exact_match_shadows() {
        let allows = vec![allow("||doubleclick.net")];
        assert_eq!(
            block_shadowed_by(&allows, "||doubleclick.net").map(|e| e.value.as_str()),
            Some("||doubleclick.net")
        );
    }

    #[test]
    fn broader_allow_shadows_narrower_block() {
        let allows = vec![allow("||doubleclick.net")];
        assert_eq!(
            block_shadowed_by(&allows, "||doubleclick.net/adx/").map(|e| e.value.as_str()),
            Some("||doubleclick.net")
        );
    }

    #[test]
    fn narrower_allow_does_not_shadow_broader_block() {
        // allow is the exception, not the shadow. This is the
        // Stage 9 "global block + per-site allow" case and MUST
        // NOT be flagged as a shadow.
        let allows = vec![allow("||doubleclick.net/adx/")];
        assert!(block_shadowed_by(&allows, "||doubleclick.net").is_none());
    }

    #[test]
    fn unrelated_patterns_do_not_shadow() {
        let allows = vec![allow("||example.com")];
        assert!(block_shadowed_by(&allows, "||doubleclick.net").is_none());
    }

    #[test]
    fn disabled_allow_does_not_shadow() {
        let allows = vec![disabled_allow("||doubleclick.net")];
        assert!(block_shadowed_by(&allows, "||doubleclick.net/adx/").is_none());
    }

    #[test]
    fn caret_suffix_normalizes_both_sides() {
        // `||foo.com^` and `||foo.com` are the same shape; shadow
        // detection must tolerate either form on either side.
        let allows = vec![allow("||foo.com^")];
        assert!(block_shadowed_by(&allows, "||foo.com/path").is_some());
        let allows = vec![allow("||foo.com")];
        assert!(block_shadowed_by(&allows, "||foo.com/path^").is_some());
    }

    #[test]
    fn empty_allow_value_is_skipped() {
        let allows = vec![allow(""), allow("||doubleclick.net")];
        assert_eq!(
            block_shadowed_by(&allows, "||doubleclick.net/x").map(|e| e.value.as_str()),
            Some("||doubleclick.net")
        );
    }

    #[test]
    fn empty_block_pattern_returns_none() {
        let allows = vec![allow("||foo")];
        assert!(block_shadowed_by(&allows, "").is_none());
    }

    #[test]
    fn first_matching_allow_is_returned() {
        // Audit UI needs a deterministic "who shadowed me"
        // answer. First wins.
        let allows = vec![allow("||broad.net"), allow("||broad.net/x")];
        let hit = block_shadowed_by(&allows, "||broad.net/x/deep").unwrap();
        assert_eq!(hit.value, "||broad.net");
    }
}
