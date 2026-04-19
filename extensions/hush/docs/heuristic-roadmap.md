# Heuristic detection roadmap

A gap analysis of what Hush's behavioral detector currently catches vs what's
documented in privacy research and built into tools like Privacy Badger,
Brave Shields, and academic trackers-detection work. Use this doc to pick the
next tier to implement; each section is self-contained enough to inform a
plan-mode spec.

## Currently detected (as of Hush 0.4.0)

| Signal | API/Source | Layer | Confidence |
|---|---|---|---|
| `sendBeacon` targets | Resource Timing `initiatorType: "beacon"` + main-world hook on `navigator.sendBeacon` | block | 95 |
| Tracking pixels | Resource Timing: third-party `<img>` with `transferSize < 200` | block | 85 |
| Hidden iframes | DOM: `display:none` / `visibility:hidden` / 1x1 / offscreen | remove | 80 |
| Polling endpoints | Resource Timing correlation: 4+ hits to same canonical URL in window | block | 75 |
| First-party telemetry subdomains | Resource Timing: subdomain of tab host, all responses tiny | block | 70 |
| Sticky overlays | DOM: position:fixed, z-index >= 100, >= 25% viewport | hide | 55 |

Main-world hooks also capture `fetch`, `XHR`, `sendBeacon`, `WebSocket.send`
with stack traces and body previews, feeding the above signals and future
per-origin attribution.

## Gaps, ranked by value

### Tier 1 — Fingerprinting detection (HIGHEST priority)

**What it is.** Sites identify returning users without cookies by reading
hardware and configuration signals that are stable per-device and unique
enough in aggregate. Academic research (arXiv 2411.12045, 2024) names three
canonical techniques: Canvas, WebGL, and AudioContext fingerprinting. All
three are invisible to our current detector because they don't show up as
unusual network traffic - the fingerprint is BUILT locally then sent as a
single routine-looking POST.

**Why it matters.** Fingerprinting persists across cookie clears, incognito
modes, and even some anti-tracking extensions. It's the #1 tracking mechanism
a user can't meaningfully defeat by inspecting network traffic alone. Hush's
current design cannot see it happening.

**Detection strategy.** Hook the specific APIs that fingerprinting exploits.
All of these have almost no legitimate non-fingerprinting uses.

1. **Canvas fingerprinting** - hook `HTMLCanvasElement.prototype.toDataURL`,
   `.toBlob`, `.getContext("2d")...getImageData()`, and
   `CanvasRenderingContext2D.prototype.measureText`. A page that calls these
   3+ times within the first N seconds of load, on a canvas that was never
   appended to the document (or is offscreen), is fingerprinting. Normal
   canvas usage (game, chart, animation) appends the canvas and doesn't
   call `toDataURL` in a tight burst.

2. **WebGL fingerprinting** - hook `WebGLRenderingContext.prototype.getParameter`
   and `WebGL2RenderingContext.prototype.getParameter`. The
   `UNMASKED_RENDERER_WEBGL` and `UNMASKED_VENDOR_WEBGL` parameters expose
   GPU model strings; reading them is the telltale. Also flag reads of
   `MAX_TEXTURE_SIZE`, `MAX_VERTEX_UNIFORM_VECTORS`, and the dozen other
   hardware-specific parameters well-known in the literature.

3. **Audio fingerprinting** - hook `OfflineAudioContext` construction. Almost
   no legit audio code needs OfflineAudioContext; it's specifically useful
   for creating reproducible audio signals for fingerprinting. Presence of
   an `OfflineAudioContext` with oscillator + compressor = fingerprinting
   with very high confidence.

4. **Font enumeration** - repeated `measureText` calls with different
   `font-family` values. Sites measure a control word against a list of
   typeface names to detect which fonts are installed. Heuristic: 20+
   `measureText` calls within 1 second, each with a different `.font`
   assignment on the same context.

**Implementation notes.**

- Hooks extend the existing `mainworld.js` structure. Add new sections for
  canvas/WebGL/audio/measureText, forward observations via the existing
  `__hush_call__` event machinery with a new `kind` value.
- Background aggregates per-tab: total Canvas reads, total WebGL parameter
  reads, OfflineAudioContext construction count, measureText count with
  distinct font families.
- Suggestions: when counts exceed thresholds within the first 10 seconds
  of page load, surface a block suggestion for the host that attempted
  it. Confidence 90+ given how specific these APIs are.
- False positives: legit cases include HTML5 games, charting libraries,
  video editors. These typically don't call `toDataURL` in a burst; the
  threshold (3+ in 5s) screens them out.
- Rule target: we don't block an API (impossible without breaking the
  page). We surface a BLOCK suggestion for the script's origin (from
  stack traces) so the user can block the whole fingerprinting script.

**Sources.**

- [Fingerprinting and Tracing Shadows (arXiv 2024)](https://arxiv.org/html/2411.12045v1)
- [Canvas, Audio and WebGL analysis](https://blog.octobrowser.net/canvas-audio-and-webgl-an-in-depth-analysis-of-fingerprinting-technologies)
- [Browser Fingerprinting: A Survey (ResearchGate)](https://www.researchgate.net/publication/332873650_Browser_Fingerprinting_A_survey)

### Tier 2 — Session replay tools

**What it is.** Hotjar, FullStory, Microsoft Clarity, LogRocket, Smartlook,
Mouseflow, Amplitude Session Replay, and Contentsquare record every click,
keystroke, mouse movement, scroll event, and form input on a page, then
reconstruct sessions as video-playback for the site owner. EFF called this
out explicitly in [Privacy Badger issue 715](https://github.com/EFForg/privacybadger/issues/715).

**Why it matters.** These tools capture more than almost any other tracker
and are embedded across millions of sites, often invisibly. A user visiting
a shopping site might have every hesitation, mis-click, and typed-and-deleted
search query recorded and replayed to the site's product team.

**Detection strategy.** Three independent signals, any one of which is strong.

1. **Known vendor globals.** Session replay tools expose identifiable globals
   on `window`:
   - `window._hjSettings` → Hotjar
   - `window.FS` or `window.FS.identify` → FullStory
   - `window.clarity` → Microsoft Clarity
   - `window.LogRocket` → LogRocket
   - `window.smartlook` → Smartlook
   - `window.mouseflow` → Mouseflow
   
   Periodic polling (once per scan interval) checks for these names. Presence
   is a ~99% confidence positive. Note this is NOT a hardcoded domain list -
   it's a well-known-globals dictionary, roughly analogous to how `detect-installed-fonts`
   libraries work.

2. **Listener density.** Session replay tools attach many listeners to the
   document on load - often 20+ for mousemove, mousedown, mouseup, click,
   keydown, keyup, scroll, input. Normal sites attach 1-3.
   
   Hook `EventTarget.prototype.addEventListener` at document_start, count
   additions on `document`/`window`/`body` for these event types. Flag if
   total exceeds 12 within first 3 seconds of load.

3. **Continuous WebSocket or high-frequency fetch to a single endpoint** -
   already covered by our main-world `WebSocket.send` hook and polling
   detector. Session replay shows as 20+ WS sends per minute or continuous
   JSON-payload POSTs to one endpoint.

**Implementation notes.**

- Known-globals check is a content-script periodic task (runs alongside the
  behavioral scan). 20 lines of code. Findings go into the scan payload.
- Listener-density check hooks `addEventListener` in `mainworld.js`. Same
  hook pattern as existing fetch/XHR hooks.
- Suggestions group by vendor name when globals match: "Hotjar session
  replay detected - block hotjar.com?" Rule applies at the network layer.
- Vendor dictionary is shipped as a SEED in code but user-editable in the
  allowlist-style UI (consistent with rest of Hush's "nothing is hardcoded"
  principle).

**Sources.**

- [EFF Privacy Badger issue 715](https://github.com/EFForg/privacybadger/issues/715)
- [Hotjar's own documentation of what gets recorded](https://www.hotjar.com/session-recordings/)

### Tier 3 — Navigator / device property fingerprinting

**What it is.** Beyond canvas/WebGL/audio, fingerprinters read 15-30 property
accessors on `navigator`, `screen`, and `window` in rapid succession. The
combination of values (userAgent + language + timezone + hardwareConcurrency
+ deviceMemory + plugins + maxTouchPoints + ...) uniquely identifies 90%+ of
browser sessions in large-population studies.

**Why it matters.** Subset of fingerprinting separate from Tier 1. Tier 1
catches the "fancy" fingerprint techniques; this catches the "boring" but
equally effective property-read fingerprint.

**Detection strategy.** Monkey-patch property getters on `navigator` and
`screen` prototype using `Object.defineProperty`. Count reads per-origin
from stack traces. If a single origin reads 10+ of these properties within
3 seconds of load, flag.

The specific properties to monitor:

```
navigator: userAgent, platform, language, languages, hardwareConcurrency,
           deviceMemory, maxTouchPoints, cookieEnabled, doNotTrack, plugins,
           mimeTypes, vendor, webdriver, connection, onLine
screen:    width, height, availWidth, availHeight, colorDepth, pixelDepth
window:    devicePixelRatio, innerWidth, innerHeight, screenX, screenY
```

**Implementation notes.**

- Adding getter hooks to `navigator` prototypes is tricky; needs to be done
  carefully to not break sites' normal use. Wrap the existing getter, count,
  delegate to real value, pass through.
- Some noise: legit code reads `navigator.userAgent` for browser detection.
  Threshold-based (10+ DIFFERENT properties in quick succession) screens most
  legit cases - a site that just does a UA sniff reads 1-2 properties, not 10+.
- Output similar to Tier 1: suggests blocking the origin of the fingerprinting
  script (from stack trace).

**Sources.**

- [Comparative Study on Browser Fingerprinting (Wesleyan)](https://digitalcollections.wesleyan.edu/_flysystem/fedora/2023-07/1229_379269.pdf)
- [Brave Privacy Features - their randomization approach](https://brave.com/privacy-features/)

### Tier 4 — Storage-based supercookies

**What it is.** Tracking IDs persisted in places that survive cookie clears:
`localStorage`, `sessionStorage`, `IndexedDB`, Cache API (via service
workers), HTTP ETag/Last-Modified headers, and more recently `navigator.storage`
quota signaling. Third-party iframes with storage access can set a user ID
once and read it on every subsequent visit to any site embedding the same
iframe.

**Why it matters.** Cookie-based tracking is increasingly constrained by
browser cookie policies. Supercookies are the response - move the tracking
identifier into storage that policies don't touch.

**Detection strategy.**

1. Hook `localStorage.setItem`, `sessionStorage.setItem`, `indexedDB.open`
   from iframes whose origin differs from the tab's top-frame origin. A
   third-party iframe writing persistent storage is a supercookie flag.
2. Count unique storage keys written per-origin. Legit first-party sites
   write many small settings; third-party iframes usually write a single
   identifier.
3. Hook `document.cookie` reads from third-party iframes (different from
   setting - reading a value that was set elsewhere implies cross-frame
   coordination).

**Implementation notes.**

- Content script runs in every frame (v0.4.0 has `all_frames: true`); each
  frame can check `window.top === window` to know if it's a third-party
  iframe.
- Hooks live in `mainworld.js`. Reports include frame hostname + keys written.
- Suggestion: block the third-party iframe entirely (since it's attempting
  supercookie placement). Applies via Hush's existing Remove-iframe layer.

**Sources.**

- [Brave Shields ephemeral storage](https://brave.com/shields/)
- [Brave Privacy Features](https://brave.com/privacy-features/)

### Tier 5 — `requestAnimationFrame` loop detection

**What it is.** A JavaScript `requestAnimationFrame` loop firing at 60Hz with
no corresponding visible paint activity. This is exactly the Lottie-canvas
case that started this project - Lottie-canvas animations running continuously
while invisible, burning CPU/GPU for nothing.

**Why it matters.** The CPU/GPU cost is real (this was a 40% CPU bug in the
very first user story). These loops also serve no user purpose - if nothing
is painting, the animation is invisible, which means it's residual or bugged.

**Detection strategy.**

1. Hook `window.requestAnimationFrame`. Count callbacks fired per second.
2. Subscribe to `PerformanceObserver` for `paint` entries. Count paint events
   per second.
3. If rAF rate >= 30/s but paint rate < 1/s over a 5-second window, flag.
   The requested animation frames aren't producing visible output.

**Implementation notes.**

- Hook in `mainworld.js`. Count callbacks cheaply (increment counter inside
  wrapped rAF).
- `PerformanceObserver({ type: 'paint', buffered: true })` in content script
  isolated world OR main world - either works.
- Rate correlation happens in background or content script; flag as a
  "visible CPU drain" suggestion. Suggested action: user-discoverable
  Remove/Hide rule for the canvas or owning element, or a Block on the
  library serving the animation.
- False positives: some legit UIs use rAF without paints (e.g., physics
  simulations that paint only on state change). Threshold and minimum-
  observation-window should handle most cases.

**Sources.**

- The user story that initiated this project (see session logs)

### Tier 6 — Service Worker registration tracking

**What it is.** A Service Worker registered by a site can intercept every
subsequent fetch the browser makes to that origin, persist cached data
across tabs and browser restarts, and wake the browser for push notifications.
Some sites register a SW purely as a tracking vector.

**Why it matters.** SWs outlive the tab and operate in a separate execution
context that Hush cannot see into. A registered SW can transparently rewrite
requests or add tracking to every page.

**Detection strategy.** Hook `navigator.serviceWorker.register()` in
`mainworld.js`. Report each registration with scope and script URL. Not
automatically suspicious - PWAs, offline docs, and chat apps legitimately
use SWs - but worth surfacing so the user can see what they've implicitly
consented to.

**Implementation notes.**

- Hook returns a Promise; we can preserve return value while observing.
- Popup gets a new "Service Workers registered" info section for the tab.
- Not a suggestion per se - more of a "here's what's running in the
  background for this site" disclosure.

**Sources.**

- General knowledge from Chrome extension architecture.

## Principles guiding which to build next

These are the guardrails from the past chapters:

1. **Nothing hardcoded.** Vendor dictionaries, threshold values, and
   well-known-globals lists ship as defaults but are user-editable in
   storage and reset-to-defaults in the UI. Matches the iframe-allowlist
   pattern we already established.

2. **Opt-in detection is the default posture.** Tiers 1-4 increase the
   per-scan cost of the detector. The existing "Enable behavioral
   suggestions" toggle must gate them all; when the feature is off, the
   hooks are not installed and no periodic work runs.

3. **Every suggestion needs a `Why?` explanation.** Whatever we detect must
   surface in the popup with enough context that the user can understand
   why the suggestion appeared and dismiss false positives with confidence.

4. **Evidence first, rule second.** Hush doesn't apply anything automatically.
   Each new tier's output is a suggestion for the user to accept or dismiss.

## Recommended implementation order

1. **Tier 1 + Tier 2 together.** Both ride on existing `mainworld.js` hook
   infrastructure. High signal, low ambiguity. Big privacy impact for users.
2. **Tier 5 (rAF loop).** Smallest delta, solves the original user complaint,
   demonstrates the "visible vs not visible" behavioral pattern.
3. **Tier 3 (navigator reads).** Most nuanced (legit vs illegit); build after
   T1/T2 so we have more signal-processing patterns to reuse.
4. **Tier 4 (supercookies).** Needs `all_frames: true` cross-origin
   coordination; bigger architectural load.
5. **Tier 6 (service workers).** Lowest urgency; primarily disclosure rather
   than blocking.

## Out of scope

Explicitly NOT planned for Hush's current design:

- **Full filter-list integration** (EasyList, EasyPrivacy). uBlock Origin
  Lite already does this. Hush's mandate is "per-site surgical cleanup +
  behavioral detection of what lists miss."
- **Anti-fingerprinting via API randomization** (Brave's approach). Requires
  deep browser integration we can't replicate in an MV3 extension; surfacing
  detections and letting the user block sources is the Hush-sized version.
- **Automated cross-site tracker correlation** (Privacy Badger's core
  algorithm). Complex stateful detection; Hush stays per-tab.
- **Cookie consent banner auto-dismissal** (I Don't Care About Cookies,
  Brave's Cookiecrumbler). Different problem from behavioral tracking; could
  be a separate companion extension.
