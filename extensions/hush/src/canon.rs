//! URL canonicalization and DNR pattern-keyword extraction.
//!
//! URL work goes through the `url` crate (the same parser used by
//! reqwest, hyper, and most of the Rust HTTP ecosystem). Keeps us out
//! of corner cases like punycode hosts, percent-encoded segments, and
//! IPv6 literals.

use url::Url;

/// Noise query parameters the JS canonicalizer strips before clustering
/// "same URL with different timestamps" into one canonical URL for the
/// polling-endpoint detector.
const NOISE_QUERY_PARAMS: &[&str] = &["t", "ts", "_", "nonce", "cb", "callback", "v", "_t", "rand"];

/// Canonicalize a URL: parse, drop known-noise query parameters, drop
/// the fragment, and reserialize. If parsing fails (not a valid URL),
/// return the input unchanged so the caller can still cluster on raw
/// string equality.
pub fn canonicalize_url(input: &str) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return input.to_string();
    };

    // Collect non-noise params first, then replace the query. The url
    // crate's query_pairs_mut returns a Serializer that needs the mut
    // borrow released before we can read the URL back.
    let filtered: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| {
            !NOISE_QUERY_PARAMS
                .iter()
                .any(|n| n.eq_ignore_ascii_case(k.as_ref()))
        })
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if filtered.is_empty() {
        url.set_query(None);
    } else {
        let mut ser = url.query_pairs_mut();
        ser.clear();
        for (k, v) in &filtered {
            ser.append_pair(k, v);
        }
    }
    url.set_fragment(None);

    url.as_str().to_string()
}

/// Extract the longest stable substring from a DNR urlFilter pattern.
/// Used by the popup's "pattern broken?" diagnostic to hint at what the
/// rule's intended keyword is when no actual matches have been observed.
///
/// Not a URL - DNR patterns like `||foo.com^` are their own grammar, so
/// no `url` crate involvement. Pure string math matching the JS helper.
pub fn pattern_keyword(pattern: &str) -> &str {
    if pattern.is_empty() {
        return "";
    }
    let mut best: &str = "";
    let mut start: Option<usize> = None;
    let bytes = pattern.as_bytes();
    for (i, &c) in bytes.iter().enumerate() {
        let is_control = matches!(c, b'|' | b'^' | b'*');
        if is_control {
            if let Some(s) = start.take() {
                let piece = &pattern[s..i];
                if piece.len() > best.len() {
                    best = piece;
                }
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        let piece = &pattern[s..];
        if piece.len() > best.len() {
            best = piece;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_strips_noise_params() {
        let canon = canonicalize_url("https://api.site.test/poll?t=1234&user=alice");
        assert_eq!(canon, "https://api.site.test/poll?user=alice");
    }

    #[test]
    fn canonicalize_removes_all_noise_leaves_no_trailing_question() {
        let canon = canonicalize_url("https://site.test/heartbeat?t=1&_=2&cb=x");
        assert_eq!(canon, "https://site.test/heartbeat");
    }

    #[test]
    fn canonicalize_preserves_order_of_non_noise_params() {
        let canon =
            canonicalize_url("https://site.test/path?a=1&t=noise&b=2&_=also&c=3");
        assert_eq!(canon, "https://site.test/path?a=1&b=2&c=3");
    }

    #[test]
    fn canonicalize_is_case_insensitive_on_param_names() {
        let canon = canonicalize_url("https://site.test/p?T=1&V=2&keep=yes");
        assert_eq!(canon, "https://site.test/p?keep=yes");
    }

    #[test]
    fn canonicalize_drops_fragment() {
        let canon = canonicalize_url("https://site.test/p?keep=1#section");
        assert_eq!(canon, "https://site.test/p?keep=1");
    }

    #[test]
    fn canonicalize_non_url_returns_input() {
        assert_eq!(canonicalize_url("//just/a/path"), "//just/a/path");
    }

    #[test]
    fn canonicalize_handles_punycode_host() {
        // url crate normalizes internationalized domain names to ASCII
        // form; we just check it doesn't blow up and returns a usable
        // canonical form.
        let canon = canonicalize_url("https://xn--bcher-kva.example/a?t=1&x=2");
        assert_eq!(canon, "https://xn--bcher-kva.example/a?x=2");
    }

    #[test]
    fn pattern_keyword_extracts_longest_literal() {
        assert_eq!(pattern_keyword("||collector.github.com^"), "collector.github.com");
    }

    #[test]
    fn pattern_keyword_handles_wildcards() {
        assert_eq!(pattern_keyword("*.ads.doubleclick.net"), ".ads.doubleclick.net");
    }

    #[test]
    fn pattern_keyword_empty_returns_empty() {
        assert_eq!(pattern_keyword(""), "");
        assert_eq!(pattern_keyword("||^*"), "");
    }

    #[test]
    fn pattern_keyword_picks_longer_of_two_pieces() {
        assert_eq!(pattern_keyword("short*muchlongerrun"), "muchlongerrun");
    }
}
