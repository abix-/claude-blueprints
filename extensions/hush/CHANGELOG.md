# Changelog

All notable changes to the Hush extension.

Format is loosely based on Keep-a-Changelog. Each release bumps
`manifest.json` `version` field; any bump requires an entry here.

## [Unreleased]

## [0.6.0] - 2026-04-19

### Added
- **Tier 5 invisible-animation-loop detection** (the original Hush user
  story). Main-world hooks on the hot 2D canvas draw ops (`fillRect`,
  `strokeRect`, `clearRect`, `drawImage`, `fill`, `stroke`, `putImageData`)
  sample visibility of the target canvas (viewport intersection +
  `display:none` / `visibility:hidden` / `opacity:0` / sub-2px dimensions)
  and emit `canvas-draw` observations. Background detection: if one script
  origin sustains 20+ invisible-canvas draws over a window >= 3 seconds
  with >= 80% invisibility ratio, a block suggestion is emitted at
  confidence 70 with the canvas selector + sample count in evidence.
- Sampling is throttled to one observation per canvas per 100ms so 60Hz
  loops produce ~10 samples/sec per canvas instead of 60. Layout-read cost
  is bounded.
- 6 new tests in `test/emit_contract.test.mjs` covering visible / offscreen
  / `display:none` / 1x1 / throttle / per-canvas-throttle cases.

### Changed
- `content.js` relay now preserves `op`, `visible`, and `canvasSel` fields
  from the main-world CustomEvent detail.
- `docs/heuristic-roadmap.md` moves Tier 5 to the shipped table; next-up
  is Tier 3 (navigator/screen property reads).

## [0.5.1] - 2026-04-19

### Fixed
- **Tier 1/2 detectors now actually work.** `mainworld.js` `emit()` was
  cherry-picking only `url`/`method`/`bodyPreview`/`stack`/`t`/`kind` into
  the CustomEvent detail, dropping every signal-specific field
  (`hotParam`, `font`, `text`, `eventType`, `vendors`, `param`). Downstream
  detectors in `background.js` gated on those missing fields and silently
  did nothing. Affected signals that were dead in 0.5.0: WebGL UNMASKED
  fingerprint read (conf 95), font-enumeration fingerprint (conf 85),
  session-replay listener density (conf 80), session-replay vendor-global
  detection for Hotjar/FullStory/Clarity/LogRocket/Smartlook/Mouseflow/
  PostHog (conf 95). After the fix `emit()` spreads all data into detail.

### Added
- Contract test suite (`test/emit_contract.test.mjs`) that loads
  `mainworld.js` into a sandboxed context and asserts every `__hush_call__`
  kind round-trips its signal-specific fields. Covers fetch, XHR, beacon,
  WebSocket, canvas-fp, webgl-fp (including WebGL2 + UNMASKED hot-param),
  audio-fp, font-fp, listener-added, replay-global. Run with `npm test`.
- Root `package.json` with `test` script and jsdom-free node:test harness.
- `.gitignore` for `node_modules/` and `package-lock.json`.

## [0.5.0] - 2026-04-17

### Added
- **Tier 1 fingerprinting detection** per `docs/heuristic-roadmap.md`:
  canvas (`toDataURL`/`toBlob`/`getImageData`), WebGL
  (`getParameter` with UNMASKED_RENDERER_WEBGL and UNMASKED_VENDOR_WEBGL
  flagged hot), audio (`OfflineAudioContext` construction), and font
  enumeration (`measureText` across distinct font families). Each emits
  a block suggestion targeting the script's origin.
- **Tier 2 session-replay detection**: vendor-global polling
  (`_hjSettings`, `FS`, `clarity`, `LogRocket`, `smartlook`, `mouseflow`,
  `__posthog`) and listener-density heuristic (12+ interaction listeners
  on document/window/body from one script origin).
- `docs/heuristic-roadmap.md` now source material for future tiers.

### Note
Shipped broken; signal-specific fields never crossed the main/isolated
world boundary. See 0.5.1 for the fix.

## [0.4.0] - 2026-04-11

### Changed
- **DNR rules are now global URL-pattern matches**, not restricted by
  `initiatorDomains`. Chrome's `initiatorDomains` only matches the
  initiating frame's origin, which misses cross-origin iframe traffic
  (e.g. redgifs iframes embedded on reddit). Rules declared under a site
  config now fire wherever the URL appears; the site key is retained
  in-memory for display only.
- Suggestions attribute to the tab's top-frame hostname, not whichever
  frame emitted the observation.

### Added
- Popup "Why?" button per suggestion shows inline dedup diagnostic:
  the value being checked, the matched config key, existing-rule count
  and sample, and the dedup outcome. No DevTools trip required.
- `docs/heuristic-roadmap.md` gap analysis for future detection tiers.

### Fixed
- Silent `.catch` on pass-through fetch promise so Chrome doesn't
  attribute site-level fetch rejections to Hush's hook frame.
- `accounts.youtube.com` added to iframe allowlist defaults (YouTube
  silent auth).

## [0.3.0] - 2026-04-08

### Added
- **Main-world hooks** (`mainworld.js`) for `fetch`,
  `XMLHttpRequest.open/send`, `navigator.sendBeacon`, and
  `WebSocket.send`. Captures URL, method, body preview, and top-6-frame
  stack trace per call. Runs in `content_scripts` with `world: MAIN` and
  `all_frames: true` so cross-origin iframe traffic is observable.
- In-popup block-rule diagnostics: per-rule fire count, status
  (`firing` / `no-traffic` / `pattern-broken`), and suggestive hint when
  observed traffic contains the pattern's keyword but the rule never
  fired.
- User-configurable allowlists for iframes and sticky overlays, seeded
  with known-legit captcha / OAuth / payment / modal-root defaults.
  Options page exposes the raw text with a reset button.
- Copy buttons on every evidence section (blocked URLs, removed
  elements, suggestion evidence) so the full untruncated text can be
  grabbed for bug reports.
- Amazon case study (`docs/amazon.md`), observation-only.
- Reddit case study (`docs/reddit.md`) with full rule rationale.

### Changed
- Default block patterns drop the trailing `^` (which causes match
  failures on hyphenated subdomains in Chrome DNR). Dedup tolerates
  either form.
- Service worker rebuilds `rulePatterns` map from live DNR rules on
  wake, so the popup still shows per-URL evidence after the SW idled.

## [0.2.0] - 2026-04-04

### Added
- Behavioral suggestions detector (opt-in): `sendBeacon` targets,
  tracking pixels, first-party telemetry subdomains, polling endpoints,
  hidden iframes, sticky overlays. Yellow-`!` badge when suggestions
  pending. One-click accept or dismiss from popup, with inline evidence.

## [0.1.0] - 2026-04-02

### Added
- Initial release. Three layers: block (via
  `declarativeNetRequest`), remove (via DOM + `MutationObserver`),
  hide (via injected stylesheet).
- Two-pane options editor with raw-JSON escape hatch.
- Per-tab activity popup with matched site, counts per layer,
  blocked-URL and removed-element evidence lists, and debug
  clipboard button.
