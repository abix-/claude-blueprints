//! Script-origin extraction from V8 stack frames.
//!
//! The main-world hooks capture a call-stack as an array of strings;
//! each frame looks like
//! `  at name (https://cdn.example.com/tracker.js:10:5)`.
//! To attribute an observation to the script that fired it, find the
//! first frame whose URL is NOT Hush's own `mainworld.js`, parse its
//! host, and return that host.
//!
//! URL parsing goes through the `url` crate so punycode/IDN/encoded
//! hosts are handled correctly. Empty or unparseable input returns an
//! empty string; caller treats that as "unknown script."

use url::Url;

/// Given a stack captured by the main-world hook, return the hostname
/// of the first non-Hush script frame. Returns empty string when no
/// parseable frame is found.
pub fn script_origin_from_stack<S: AsRef<str>>(stack: &[S]) -> String {
    for frame in stack {
        let frame = frame.as_ref();
        if frame.contains("mainworld.js") {
            continue;
        }
        if let Some(host) = extract_host(frame) {
            return host;
        }
    }
    String::new()
}

/// Pull the first http/https URL out of a frame string and return its
/// host. None when no URL is present or the substring isn't parseable.
fn extract_host(frame: &str) -> Option<String> {
    let http_idx = frame.find("http://");
    let https_idx = frame.find("https://");
    let start = match (http_idx, https_idx) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return None,
    };
    let rest = &frame[start..];
    // URL ends at first whitespace or closing paren (V8 wraps URLs in
    // parens for named frames).
    let end = rest
        .find(|c: char| c == ')' || c.is_whitespace())
        .unwrap_or(rest.len());
    let candidate = &rest[..end];
    // V8 appends `:line:col` to every URL. `url::Url` parses that fine
    // as part of the path/port segment - we only read `.host_str()`,
    // which is unaffected.
    Url::parse(candidate)
        .ok()
        .and_then(|u| u.host_str().map(|s| s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_typical_v8_frame() {
        let frame = "    at emitBeacon (https://cdn.example.com/tracker.js:10:5)";
        assert_eq!(extract_host(frame).as_deref(), Some("cdn.example.com"));
    }

    #[test]
    fn extract_handles_http_and_https() {
        assert_eq!(
            extract_host("    at x (http://plain.test/a.js:1:1)").as_deref(),
            Some("plain.test")
        );
    }

    #[test]
    fn skip_mainworld_frames() {
        let stack = vec![
            "    at emit (https://site.test/mainworld.js:80:3)",
            "    at fingerprint (https://trackers.test/fp.js:20:8)",
            "    at main (https://site.test/app.js:5:1)",
        ];
        assert_eq!(script_origin_from_stack(&stack), "trackers.test");
    }

    #[test]
    fn empty_stack_returns_empty() {
        let stack: Vec<&str> = vec![];
        assert_eq!(script_origin_from_stack(&stack), "");
    }

    #[test]
    fn all_mainworld_frames_returns_empty() {
        let stack = vec!["at emit (https://a/mainworld.js:1:1)"];
        assert_eq!(script_origin_from_stack(&stack), "");
    }

    #[test]
    fn frame_without_url_is_skipped() {
        let stack = vec![
            "    at [native code]",
            "    at realFrame (https://good.test/a.js:1:1)",
        ];
        assert_eq!(script_origin_from_stack(&stack), "good.test");
    }

    #[test]
    fn punycode_host_returns_ascii_form() {
        // url crate normalizes to ASCII; we get the xn--... back.
        let frame = "at fn (https://xn--bcher-kva.example/a.js:1:1)";
        assert_eq!(extract_host(frame).as_deref(), Some("xn--bcher-kva.example"));
    }
}
