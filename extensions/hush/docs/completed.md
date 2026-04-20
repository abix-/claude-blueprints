# Current user-facing features

Short version of what's in Hush right now. Systems list, not changelog.

## Three-layer defense

- **Block** at the network layer via `chrome.declarativeNetRequest`.
  uBlock-style URL patterns; matching requests fail with
  `net::ERR_BLOCKED_BY_CLIENT` before DNS.
- **Remove** via DOM: matching elements are physically deleted from the
  page; a MutationObserver keeps them out as the page evolves.
- **Hide** via CSS: `display: none !important` in a stylesheet injected
  at `document_start`. Skips render but leaves the element in the DOM.

Read more: [../README.md](../README.md) for layer semantics and runtime
order.

## Behavioral detector

Observes resource requests, hidden iframes, and sticky overlays; emits
inline suggestions to the popup with per-signal teaching text.
Opt-in; off by default.

- `sendBeacon` targets (confidence 95)
- Tracking pixels (confidence 85)
- First-party telemetry subdomains (confidence 70)
- Polling endpoints (confidence 75)
- Hidden iframes (confidence 80, with captcha/OAuth/payment allowlist)
- Sticky overlays (confidence 55)

Read more: [../README.md](../README.md) "Behavioral suggestions"
section.

## Fingerprint detection

Main-world hooks watch the APIs fingerprinters lean on. Aggregation
thresholds flag a script origin for blocking.

- Canvas fingerprinting (toDataURL / toBlob / getImageData) - confidence
  90
- WebGL UNMASKED_RENDERER_WEBGL / UNMASKED_VENDOR_WEBGL reads -
  confidence 95
- WebGL general getParameter density (8+) - confidence 75
- OfflineAudioContext construction - confidence 90
- Font enumeration via `measureText` (20+ distinct font families) -
  confidence 85

## Session-replay detection

- Vendor global poll (Hotjar, FullStory, Clarity, LogRocket, Smartlook,
  Mouseflow, PostHog) - confidence 95
- Listener density on document/window/body (12+ interaction listeners
  from one origin within 60 s) - confidence 80

## Invisible animation loop

Hot 2D canvas draw ops sample target-canvas visibility once per 100 ms
per canvas. Script origin sustaining invisible-canvas draws gets a
block suggestion. Confidence 70. This is the original Hush user story:
Lottie-style widgets running inside collapsed panels.

## Per-suggestion actions

Every suggestion has three buttons:

- **+ Add** writes the rule into the matched site's config.
- **Dismiss** suppresses the suggestion for the current tab session
  only.
- **Allow** persists the suggestion's key in `allowlist.suggestions`,
  filtering it out on every site forever. Revocable from the options
  page's Suggestion allowlist editor.

Plus two diagnostic panels:

- **Why?** shows the dedup diagnostic (tab hostname vs matched config
  key, existing rule count + sample, dedup outcome).
- **Evidence** shows the raw observations (URLs, sizes, timestamps,
  outerHTML snippets) with a copy button.

## Allowlist

One object with three sections, user-editable from the options page:

- **iframes** - URL substrings. Hidden iframes whose src contains any
  entry are skipped at detection time. Defaults cover captcha, OAuth,
  payment widgets.
- **overlays** - CSS selectors. Sticky/fixed elements matching any
  selector are skipped. Defaults cover React portals, modal roots,
  framework shells.
- **suggestions** - full suggestion keys. Populated by the popup's
  Allow button. Filters out the exact suggestion at emit time across
  all sites.

## Popup debug payload

One-click clipboard copy of a JSON snapshot: manifest version, active
config, dynamic DNR rules + per-rule fire counts, per-tab activity +
behavior summary (seen-resource count, latest iframe + sticky counts,
js-call breakdown by kind), recent blocked URLs + removed elements,
dismissed suggestions count, recent log ring buffer. Drop-in for bug
reports.

## Case studies under [case studies](README.md)

- [reddit.md](reddit.md)
- [amazon.md](amazon.md)
- [github.md](github.md)
