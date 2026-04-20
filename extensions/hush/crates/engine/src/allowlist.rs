//! Allowlist-membership helpers.
//!
//! Both use case-insensitive substring matching so the user can enter
//! entries like "google.com/recaptcha" and have them apply regardless of
//! subdomain casing, path case, or URL normalization.

/// Returns `true` if the iframe's src URL contains any entry from the
/// allowlist (case-insensitive). Ports the JS `isLegitHiddenIframe`
/// helper used by the hidden-iframe detector to skip
/// captcha/OAuth/payment iframes.
pub fn is_legit_hidden_iframe<S: AsRef<str>>(src_url: &str, allowlist: &[S]) -> bool {
    if src_url.is_empty() {
        return false;
    }
    let src_lower = src_url.to_ascii_lowercase();
    allowlist.iter().any(|entry| {
        let e = entry.as_ref();
        !e.is_empty() && src_lower.contains(&e.to_ascii_lowercase())
    })
}

/// Returns `true` if the given element-matcher function matches any
/// selector in the overlay allowlist. The matcher is provided by the
/// caller (content-script side uses `Element.matches()`; tests pass a
/// literal comparator). Invalid selectors are silently skipped to match
/// the JS behavior (sites occasionally write non-standard selectors
/// into the allowlist textarea).
pub fn overlay_allowlisted<'a, F>(selectors: &'a [String], mut matches: F) -> bool
where
    F: FnMut(&'a str) -> bool,
{
    selectors
        .iter()
        .filter(|s| !s.is_empty())
        .any(|s| matches(s.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legit_iframe_matches_substring_case_insensitively() {
        let allow = ["google.com/recaptcha".to_string()];
        assert!(is_legit_hidden_iframe(
            "https://Google.com/recaptcha/enterprise.js",
            &allow
        ));
    }

    #[test]
    fn legit_iframe_rejects_non_match() {
        let allow = ["google.com/recaptcha".to_string()];
        assert!(!is_legit_hidden_iframe(
            "https://ads.example.com/pixel",
            &allow
        ));
    }

    #[test]
    fn legit_iframe_empty_src_is_not_legit() {
        let allow = ["stripe.com".to_string()];
        assert!(!is_legit_hidden_iframe("", &allow));
    }

    #[test]
    fn legit_iframe_empty_allowlist_never_matches() {
        let allow: [String; 0] = [];
        assert!(!is_legit_hidden_iframe("https://any.site", &allow));
    }

    #[test]
    fn overlay_allowlist_matches_via_caller_function() {
        let selectors = vec!["#portal".to_string(), ".modal-root".to_string()];
        let matches = overlay_allowlisted(&selectors, |s| s == ".modal-root");
        assert!(matches);
    }

    #[test]
    fn overlay_allowlist_returns_false_when_nothing_matches() {
        let selectors = vec!["#portal".to_string()];
        assert!(!overlay_allowlisted(&selectors, |_| false));
    }

    #[test]
    fn overlay_allowlist_skips_empty_selectors() {
        let selectors = vec!["".to_string(), "#portal".to_string()];
        let mut seen = Vec::new();
        overlay_allowlisted(&selectors, |s| {
            seen.push(s.to_string());
            false
        });
        // Empty selector is not passed to the matcher.
        assert_eq!(seen, vec!["#portal".to_string()]);
    }
}
