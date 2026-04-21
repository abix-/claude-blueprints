//! Per-signal teaching text.
//!
//! One short paragraph per detection category, shown inline under the
//! suggestion's reason in the popup. Technical, terse, explains what
//! the signal is and why it's worth blocking. Kept here as `const`
//! strings so new detectors add an enum variant + constant in one place
//! and new usage sites look up by [`LearnKind`].

/// Every signal kind that has a teaching paragraph. Use the
/// string-tagged form when JS hands us the kind over the wasm boundary;
/// use the enum when we're staying in Rust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearnKind {
    Beacon,
    Pixel,
    FirstPartyTelemetry,
    Polling,
    HiddenIframe,
    StickyOverlay,
    CanvasFp,
    WebglFpHot,
    WebglFp,
    AudioFp,
    FontFp,
    ReplayVendor,
    ReplayListener,
    AttentionTracking,
    ClipboardRead,
    DeviceApiProbe,
    RafWaste,
}

impl LearnKind {
    /// Map a string kind tag (as used in suggestion keys and main-world
    /// emit events) to the typed enum. Returns None for unknown kinds.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "beacon" => Self::Beacon,
            "pixel" => Self::Pixel,
            "first-party-telemetry" | "firstPartyTelemetry" => Self::FirstPartyTelemetry,
            "polling" => Self::Polling,
            "hidden-iframe" | "hiddenIframe" => Self::HiddenIframe,
            "sticky-overlay" | "stickyOverlay" => Self::StickyOverlay,
            "canvas-fp" => Self::CanvasFp,
            "webgl-fp-hot" | "webglFpHot" => Self::WebglFpHot,
            "webgl-fp" | "webglFp" => Self::WebglFp,
            "audio-fp" | "audioFp" => Self::AudioFp,
            "font-fp" | "fontFp" => Self::FontFp,
            "replay-vendor" | "replayVendor" => Self::ReplayVendor,
            "listener-density" | "replay-listener" | "replayListener" => Self::ReplayListener,
            "attention-tracking" | "attentionTracking" => Self::AttentionTracking,
            "clipboard-read" | "clipboardRead" => Self::ClipboardRead,
            "device-api-probe" | "deviceApiProbe" | "new-api-probe" => {
                Self::DeviceApiProbe
            }
            "raf-waste" | "rafWaste" => Self::RafWaste,
            _ => return None,
        })
    }

    /// Canonical string tag for this kind. Inverse of
    /// [`Self::from_tag`] — returns the form used in suggestion
    /// keys, main-world emit events, and `auto:<tag>` rule
    /// stamps. Must stay in sync with `from_tag`'s primary
    /// match arm for every variant.
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Beacon => "beacon",
            Self::Pixel => "pixel",
            Self::FirstPartyTelemetry => "first-party-telemetry",
            Self::Polling => "polling",
            Self::HiddenIframe => "hidden-iframe",
            Self::StickyOverlay => "sticky-overlay",
            Self::CanvasFp => "canvas-fp",
            Self::WebglFpHot => "webgl-fp-hot",
            Self::WebglFp => "webgl-fp",
            Self::AudioFp => "audio-fp",
            Self::FontFp => "font-fp",
            Self::ReplayVendor => "replay-vendor",
            Self::ReplayListener => "listener-density",
            Self::AttentionTracking => "attention-tracking",
            Self::ClipboardRead => "clipboard-read",
            Self::DeviceApiProbe => "device-api-probe",
            Self::RafWaste => "raf-waste",
        }
    }

    pub const fn text(self) -> &'static str {
        match self {
            Self::Beacon => TEXT_BEACON,
            Self::Pixel => TEXT_PIXEL,
            Self::FirstPartyTelemetry => TEXT_FIRST_PARTY_TELEMETRY,
            Self::Polling => TEXT_POLLING,
            Self::HiddenIframe => TEXT_HIDDEN_IFRAME,
            Self::StickyOverlay => TEXT_STICKY_OVERLAY,
            Self::CanvasFp => TEXT_CANVAS_FP,
            Self::WebglFpHot => TEXT_WEBGL_FP_HOT,
            Self::WebglFp => TEXT_WEBGL_FP,
            Self::AudioFp => TEXT_AUDIO_FP,
            Self::FontFp => TEXT_FONT_FP,
            Self::ReplayVendor => TEXT_REPLAY_VENDOR,
            Self::ReplayListener => TEXT_REPLAY_LISTENER,
            Self::AttentionTracking => TEXT_ATTENTION_TRACKING,
            Self::ClipboardRead => TEXT_CLIPBOARD_READ,
            Self::DeviceApiProbe => TEXT_DEVICE_API_PROBE,
            Self::RafWaste => TEXT_RAF_WASTE,
        }
    }
}

/// String-to-text lookup suitable for the wasm-bindgen export. Returns
/// `Some` iff the tag is a recognized kind; callers that want an empty
/// string for "unknown" should do the `unwrap_or` themselves.
pub fn learn_text(tag: &str) -> Option<&'static str> {
    LearnKind::from_tag(tag).map(LearnKind::text)
}

const TEXT_BEACON: &str = concat!(
    "navigator.sendBeacon() is a fire-and-forget browser API built for ",
    "telemetry. Requests are guaranteed to go out even on page unload, ",
    "and the caller can't abort them. Almost no legitimate non-tracking ",
    "use exists at scale - it was standardized specifically to deliver ",
    "analytics during page teardown."
);

const TEXT_PIXEL: &str = concat!(
    "A 1-pixel transparent image exists only to tell a third-party server ",
    "that you loaded the current page. It carries no content. Responses ",
    "under 200 bytes from a third-party img tag are the classic tracking ",
    "pixel pattern - the image itself is irrelevant, the request is the ",
    "signal."
);

const TEXT_FIRST_PARTY_TELEMETRY: &str = concat!(
    "A subdomain of the site you're on whose responses are all tiny (< 1KB ",
    "median). Almost always an internal analytics, logging, or session- ",
    "replay endpoint. First-party-owned subdomains like this don't show ",
    "up on curated filter lists (EasyPrivacy, Disconnect) because those ",
    "lists target cross-site trackers, not site-operated telemetry."
);

const TEXT_POLLING: &str = concat!(
    "The same canonical URL fetched 4+ times over several seconds with tiny ",
    "responses. Real-time chat and presence features use WebSockets for this ",
    "shape; HTTP polling to a small-response endpoint is overwhelmingly ",
    "analytics heartbeats or A/B-flag refreshers."
);

const TEXT_HIDDEN_IFRAME: &str = concat!(
    "An iframe from a different origin that is hidden (display:none, ",
    "visibility:hidden, opacity 0, 1x1, or offscreen). Invisible cross-origin ",
    "iframes are used to run third-party scripts in a partitioned JS ",
    "context (for cross-site tracking, CSP evasion, or silent third-party ",
    "cookie storage). Legitimate hidden iframes - captcha challenges, ",
    "OAuth popups, payment widgets - are already allowlisted."
);

const TEXT_STICKY_OVERLAY: &str = concat!(
    "A fixed- or sticky-position element with z-index >= 100 covering at ",
    "least 25% of the viewport. Typically a newsletter popup, cookie banner, ",
    "paywall nag, or app-store upsell. The visual presence and z-index ",
    "floor distinguish these from legitimate sticky headers."
);

const TEXT_CANVAS_FP: &str = concat!(
    "A script drew to an HTML canvas and read the pixel bytes back (via ",
    "toDataURL / toBlob / getImageData). GPU drivers, installed fonts, ",
    "and OS text-rendering stacks each produce subtly different pixels ",
    "for the same draw commands; those pixels hash to a stable per-device ",
    "fingerprint that persists across cookie clears and incognito mode. ",
    "3+ reads from one origin is the canonical fingerprinting pattern."
);

const TEXT_WEBGL_FP_HOT: &str = concat!(
    "The script read UNMASKED_VENDOR_WEBGL or UNMASKED_RENDERER_WEBGL - ",
    "the two WebGL parameters that directly expose your GPU's vendor and ",
    "model as a string. Combined with 2-3 other signals, this uniquely ",
    "identifies 90%+ of browser sessions. No legitimate rendering code ",
    "needs to know which GPU you have."
);

const TEXT_WEBGL_FP: &str = concat!(
    "Many WebGL getParameter() reads from one origin without the UNMASKED_* ",
    "reads above. Pulls per-hardware capability limits (max texture size, ",
    "uniform vector count, extension list, etc.) that vary by GPU. Hashed ",
    "together they form a fingerprint even without explicit model strings."
);

const TEXT_AUDIO_FP: &str = concat!(
    "The script constructed an OfflineAudioContext - an API that renders ",
    "audio deterministically without playing it. The rendered buffer ",
    "differs microscopically per CPU and audio stack, producing a per- ",
    "device fingerprint when hashed. Practically the only use of this API ",
    "in the wild is fingerprinting; legitimate audio code uses a plain ",
    "AudioContext."
);

const TEXT_FONT_FP: &str = concat!(
    "measureText() called 20+ times with different font-family assignments. ",
    "The script measures a control string under each candidate font; if ",
    "the rendered width differs from the browser's fallback, that font is ",
    "installed on your machine. Each installed font is an additional bit ",
    "of entropy in a cross-signal fingerprint."
);

const TEXT_REPLAY_VENDOR: &str = concat!(
    "A known session-replay SaaS (Hotjar, FullStory, Microsoft Clarity, ",
    "LogRocket, Smartlook, Mouseflow, PostHog) is loaded in the page. These ",
    "tools record every mouse movement, keystroke, form input, scroll, and ",
    "click, then replay your session as video for the site owner. You are ",
    "an unpaid test subject whose interactions are visible to anyone with ",
    "dashboard access."
);

const TEXT_REPLAY_LISTENER: &str = concat!(
    "12+ interaction event listeners (mousemove, mousedown, keydown, click, ",
    "scroll, touch*) attached to document/window/body from one script ",
    "origin. This density is characteristic of session-replay capture ",
    "even when the vendor's global name is unknown or custom-built."
);

const TEXT_ATTENTION_TRACKING: &str = concat!(
    "4+ page-lifecycle / visibility listeners (visibilitychange, focus, ",
    "blur, pagehide, pageshow, beforeunload) attached to document / ",
    "window / body from one script origin, shortly after page load. ",
    "This is how session-replay vendors, engagement analytics, and A/B ",
    "test frameworks measure dwell time, tab-aways, and exit timing. ",
    "Legitimate sites rarely attach more than one or two of these, and ",
    "usually not clustered. Neutering these listeners stops the capture ",
    "without blocking the rest of the site's script."
);

const TEXT_CLIPBOARD_READ: &str = concat!(
    "A script called navigator.clipboard.readText() - reading the ",
    "contents of your system clipboard. Chrome gesture-gates the API ",
    "but legitimate page scripts almost never need it: password ",
    "managers and clipboard-inspector tools run as extensions, not ",
    "page scripts. What a page script reading the clipboard usually ",
    "means is checking for coupon codes, competitor URLs, or paste-in ",
    "tracking parameters. Blocking the script origin stops the sniff."
);

const TEXT_DEVICE_API_PROBE: &str = concat!(
    "A script called one of the hardware-device APIs ",
    "(Bluetooth.requestDevice / USB.requestDevice / HID.requestDevice / ",
    "Serial.requestPort). These APIs are user-gesture-gated and show a ",
    "native permission prompt, but calling them tells the site the API ",
    "exists on your browser / OS and opens a prompt that's already an ",
    "entropy signal. Legitimate uses are rare and tend to be explicit ",
    "industrial / maker-space / dev-tool contexts. Random web pages ",
    "calling these are suspicious - block the origin so the prompt ",
    "never appears."
);

const TEXT_RAF_WASTE: &str = concat!(
    "A script is continuously painting to a canvas that is hidden ",
    "(display:none, offscreen, sub-2px, or opacity 0). Burns CPU and ",
    "drains battery for no visible output. Typical cause: a Lottie or ",
    "canvas-based animation left running inside a collapsed panel or ",
    "off-screen widget."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tag_roundtrips() {
        // Every kind must have a stable canonical tag that from_tag
        // accepts. This is the contract the popup relies on.
        let kinds = [
            ("beacon", LearnKind::Beacon),
            ("pixel", LearnKind::Pixel),
            ("first-party-telemetry", LearnKind::FirstPartyTelemetry),
            ("polling", LearnKind::Polling),
            ("hidden-iframe", LearnKind::HiddenIframe),
            ("sticky-overlay", LearnKind::StickyOverlay),
            ("canvas-fp", LearnKind::CanvasFp),
            ("webgl-fp-hot", LearnKind::WebglFpHot),
            ("webgl-fp", LearnKind::WebglFp),
            ("audio-fp", LearnKind::AudioFp),
            ("font-fp", LearnKind::FontFp),
            ("replay-vendor", LearnKind::ReplayVendor),
            ("listener-density", LearnKind::ReplayListener),
            ("attention-tracking", LearnKind::AttentionTracking),
            ("clipboard-read", LearnKind::ClipboardRead),
            ("device-api-probe", LearnKind::DeviceApiProbe),
            ("raf-waste", LearnKind::RafWaste),
        ];
        for (tag, expected) in kinds {
            assert_eq!(LearnKind::from_tag(tag), Some(expected), "tag {tag}");
            assert!(!expected.text().is_empty(), "text for {tag} is empty");
        }
    }

    #[test]
    fn unknown_tag_returns_none() {
        assert_eq!(LearnKind::from_tag("unknown"), None);
        assert_eq!(learn_text("unknown"), None);
    }

    #[test]
    fn all_text_is_ascii_only() {
        // User preference: no Unicode in shipped strings. These are
        // surface text in the popup; they must render identically
        // regardless of font or locale.
        for kind in [
            LearnKind::Beacon,
            LearnKind::Pixel,
            LearnKind::FirstPartyTelemetry,
            LearnKind::Polling,
            LearnKind::HiddenIframe,
            LearnKind::StickyOverlay,
            LearnKind::CanvasFp,
            LearnKind::WebglFpHot,
            LearnKind::WebglFp,
            LearnKind::AudioFp,
            LearnKind::FontFp,
            LearnKind::ReplayVendor,
            LearnKind::ReplayListener,
            LearnKind::AttentionTracking,
            LearnKind::ClipboardRead,
            LearnKind::DeviceApiProbe,
            LearnKind::RafWaste,
        ] {
            let t = kind.text();
            assert!(
                t.is_ascii(),
                "learn text for {:?} contains non-ASCII: {:?}",
                kind,
                t
            );
        }
    }
}
