# Changelog

All notable changes to the Hush extension.

Format is loosely based on Keep-a-Changelog. Each release bumps
`manifest.json` `version` field; any bump requires an entry here.

## [Unreleased]

### Stage 4 progress: popup UI porting to Leptos
- Iter 1 scaffold: Leptos 0.8 + `src/ui_popup.rs` + `mountPopup`
  wasm-bindgen export + `popup.js` bootstrap + `<div
  id="rust-popup-root">` mount point in `popup.html`.
- Iter 2: `MatchedSite` + `ActivitySummary` components. The old
  `<div id="match">` + its JS writes deleted.
- Iter 3: `SuggestionsList` + `SuggestionRow` with Add / Dismiss /
  Allow. New `src/chrome_bridge.rs` async helpers call
  `chrome.runtime.sendMessage` via `js_sys::Reflect` +
  `wasm-bindgen-futures`. 255-line JS render block deleted.
- Iter 4: Why? (dedup diag) + Evidence (raw observations with Copy
  button) expandable per-row panels.
- Iter 5: `DetectorCta` component owns the Enable / Scan-once /
  Rescan row with `chrome.storage.local` + `chrome.tabs.sendMessage`
  from Rust. `#suggestions-block` deleted from `popup.html`.
- Iter 6: `BlockedSection` component ports the Blocked (network)
  section. Groups blocked URLs by pattern, adds a collapsible
  evidence list with a Copy button, and renders per-rule diagnostics
  (firing / no-traffic / pattern-broken). New `BlockedUrl` +
  `BlockDiagnostic` types in `src/types.rs`. ~170 LOC of JS deleted
  (`renderBlockedList`, `renderBlockDiagnostics`, `escapeHtml`) plus
  the four `#block-*` DOM anchors in `popup.html`.
- Iter 7: `RemovedSection` + `RemovedEvidence` + `HiddenSection`
  components port the last two diagnostic sections. `PopupSnapshot`
  carries `remove_selectors` / `hide_selectors` as
  `IndexMap<String, u32>` so selector insertion order survives the
  JS -> Rust boundary. New `RemovedElement` type. `renderSelectorList`,
  `renderRemovedEvidence`, `makeCopyButton`, `timeOnly` deleted from
  `popup.js`; the `#sections` container (plus `#remove-count` /
  `#remove-list` / `#remove-evidence` / `#hide-count` / `#hide-list`)
  removed from `popup.html`. All popup diagnostic sections now live
  inside the Leptos tree.

Remaining before Stage 4 is fully complete: verify cold-popup-open
render time against the 100ms budget (measurement pass, no code
changes expected).

### Stage 5 progress: options + content-script porting to Leptos / web_sys
- Iter 1 scaffold: `src/ui_options.rs` + `mountOptions` wasm-bindgen
  export + `<div id="rust-options-root">` in `options.html` +
  `options.js` converted to an ES module. `SettingsToggles` component
  owns the behavioral-suggestions and verbose-logging checkboxes;
  `StatusBanner` owns the transient save-confirmation message.
  `chrome_bridge::enable_detector` refactored into the generalized
  `set_option_bool(key, value)` helper the toggles share. Exported
  `setOptionsStatus(msg, ok)` so the remaining JS handlers
  (export/reset/JSON/allowlist) feed the same banner.
- Iter 2: `ConfigToolbar` component ports the Export JSON + Reset to
  defaults buttons. `chrome_bridge::get_config_json` pretty-prints
  `chrome.storage.local["config"]` via `js_sys::JSON::stringify`;
  `chrome_bridge::reset_config_to_defaults` fetches `sites.json` and
  writes it back. Export downloads via a `web_sys::Blob` + synthetic
  anchor click; Reset calls `window.location().reload()` so the
  still-JS-owned site list + JSON editor re-read storage.
- Iter 3: `AllowlistEditor` component ports the three allowlist
  textareas (iframes / overlays / suggestion keys) + Save / Reset.
  `mountOptions` mounts to a second root `#rust-allowlist-root`
  inside the existing `<details>` wrapper. New chrome_bridge helpers
  `set_allowlist` + `get_default_allowlist`. `loadDefaultAllowlist`,
  `linesToList`, and `DEFAULT_ALLOWLIST` deleted from `options.js`.
- Iter 4: `JsonEditor` component ports the raw-JSON textarea +
  Apply / Refresh to a third mount root `#rust-json-root`. New
  `chrome_bridge::set_config_from_json` parses via `js_sys::JSON`,
  validates the top-level shape, writes to `chrome.storage.local`.
  Apply reloads the page so the still-JS-owned site list re-reads
  storage.

## [0.10.0] - 2026-04-19

### Licensing
- Project is now GPL-3.0-or-later. `LICENSE` file added at repo root
  (matches the license on abix-/endless and other sibling repos).
  Previous label was MIT; the code did not ship on a release so there
  are no downstream obligations.

### Added
- **Stage 3 of the Rust port**: main-world hook payloads now round-trip
  through a typed `SignalPayload` discriminated union in Rust. Every
  `__hush_call__` event is validated by serde at the wasm-bindgen
  boundary before it reaches the detector engine. Missing required
  fields (the 0.5.0 bug class) fail loudly instead of silently
  dropping.
- `src/main_world.rs`: `dispatchHook(detail)` validates a single event
  and dispatches the canonical CustomEvent; `drainStubQueue(queue)`
  drains the pre-WASM in-page queue on WASM ready. Both reject
  malformed payloads with console.error and continue.
- `src/types.rs`: `SignalPayload` enum with 11 variants (fetch, xhr,
  beacon, ws-send, canvas-fp, font-fp, webgl-fp, audio-fp,
  listener-added, replay-global, canvas-draw). 12 new cargo tests
  covering serde round-trip per variant + required-field enforcement.

### Changed
- `mainworld.js` rewired to the hybrid bootstrap: synchronous stubs at
  document_start push to `window.__hush_stub_q__`; WASM loads via
  dynamic `import(chrome.runtime.getURL("dist/pkg/hush.js"))`; once
  ready, queue is drained through `drainStubQueue` and subsequent hook
  calls go through `dispatchHook` directly. Pre-load coverage
  preserved via the in-page queue; steady state is typed Rust.
- `manifest.json` adds `dist/pkg/hush.js` + `dist/pkg/hush_bg.wasm`
  to `web_accessible_resources` so the MAIN-world bootstrap can
  dynamically import the WASM glue.
- `test/emit_contract.test.mjs` updated: captured queue now read from
  `window.__hush_stub_q__` instead of the old CustomEvent dispatch
  capture. 18/18 still pass.

### Design note
The approved plan asked for Rust to re-patch prototypes via
`js_sys::Reflect::set` + `Closure`. wasm-bindgen's Closure doesn't
forward implicit JS `this`, and `new Function()` (the alternative
that captures `this`) requires `unsafe-eval` CSP which many target
sites block. JS therefore owns the physically-required prototype
assignment; Rust owns every step after the capture. Mainworld.js
shrunk modestly (412 -> 376 lines) but the content of those lines is
now stubs + queue + WASM bootstrap, not typed-payload construction.

## [0.8.0] - 2026-04-19

### Added
- **Inline teaching text on every suggestion.** Each suggestion now carries
  a `learn` field: one short technical paragraph that explains what the
  signal is and why it's worth blocking. Rendered always-visible below
  the reason in the popup, styled as a muted note-block. Covers all 14
  detection types (beacon, pixel, first-party telemetry, polling, hidden
  iframe, sticky overlay, canvas-fp, webgl-fp hot, webgl-fp general,
  audio-fp, font-fp, replay vendors, replay listener density, invisible
  animation loop). Copy is in `LEARN_TEXT` at the top of `background.js`
  so new detectors can add entries in one place.
- `docs/github.md` case study: first-party `collector.github.com`
  sendBeacon telemetry, the gap curated filter lists don't reach.
  Documents the observed rule + why sendBeacon is worth blocking by
  default.

### Changed
- **`buildSuggestion()` helper** in `background.js` collapses the
  8 suggestion-push sites to a single shape builder. Fields like `diag`,
  `fromIframe`, `frameHostname`, and `learn` are computed once. Prevents
  schema drift between detectors (the emit() bug was the same class of
  problem at the main/isolated world boundary; this is the in-SW
  equivalent guard).
- README's case-studies index now lists GitHub alongside Reddit and Amazon.
- Popup CSS: new `.sugg-learn` style (muted grey background, left border
  accent) sits between the reason and the action buttons.

## [0.7.0] - 2026-04-19

### Added
- **Permanent "Allow" button on every suggestion.** Dismiss remained
  per-tab-session only, which meant any false positive (a new captcha
  provider, a legit hidden widget) came back on every page load. The
  Allow button writes the suggestion's key to `allowlist.suggestions`
  and `computeSuggestions` filters it out on every site, across sessions.
  Covers all suggestion types uniformly: block, remove, and hide, across
  every tier (beacons, pixels, polling, hidden iframes, fingerprinting,
  session replay, invisible animation loops, sticky overlays).
- New "Suggestion allowlist" section in the options page. Editable
  textarea, one key per line. Remove a line to re-enable detection of
  that specific suggestion.
- New message handler `hush:allowlist-add-suggestion` in background
  persists the key, refreshes the in-memory cache, and drops the
  allowed suggestion from every tab's state.

### Changed
- `allowlist.defaults.json` gains an empty `suggestions` array.
- `allowlistCache` shape is now `{iframes, overlays, suggestions}`.
- README's "Hidden-iframe allowlist" section renamed+expanded to cover
  the unified Allow behavior for all suggestion types.
- Popup CSS: new `.allow` button variant (green outline) distinguishes
  it from the blue primary Add button.

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
