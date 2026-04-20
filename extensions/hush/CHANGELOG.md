# Changelog

All notable changes to the Hush extension.

Format is loosely based on Keep-a-Changelog. Each release bumps
`manifest.json` `version` field; any bump requires an entry here.

## [Unreleased]

## [0.11.0] - 2026-04-20

### Stage 9 phase 1: RuleEntry schema migration
- `types::RuleEntry { value, disabled, tags, comment }` replaces
  bare-string entries in every `SiteConfig` action array. Default
  serialization elides non-`value` fields so simple rules still
  round-trip as `{"value": "..."}` on disk.
- `SiteConfig.allow: Vec<RuleEntry>` added (empty; not yet
  enforced). Reserves the shape so allow-override evaluator work
  in phase 2 doesn't require another schema bump.
- Hard migration: `background.js` runs a one-shot
  `migrateConfigSchema()` at service-worker bootstrap (gated on a
  `configSchemaVersion` storage key). Converts every string entry
  under `block` / `remove` / `hide` / `spoof` into
  `{value: s}` and writes the result back before WASM init. Rust
  only ever sees the new shape.
- `content.js` is defensive for the brief window between
  extension upgrade and the first background-worker run:
  `toValueList()` treats either string or object entries as a
  value; `disabled: true` entries are skipped on the client side
  too so the toggle in phase 2 works without a reload.
- `sites.json` seed updated to the new shape.
- 5 new regression tests in `types::rule_entry_tests` lock the
  on-disk JSON shape, reject bare strings at the Rust boundary,
  and verify `merged_site_config` still dedupes by value.

### Stage 9 phase 2: allow-action enforcement
- DNR sync now emits Block rules at `priority: 1` and Allow rules
  at `priority: 2`, so Chrome's own first-match-wins resolution
  picks Allow over any overlapping Block. Two-pass loop in
  `do_sync_dynamic_rules` guarantees the priority assignment
  regardless of authoring order.
- `RuleMeta.action` carries the rule's action through the DNR
  hit handler so `onRuleMatchedDebug` emits a FirewallEvent with
  `action: "allow"` when an allow rule fires, separate from the
  blocked-URLs panel (allow hits don't bump the block counter).
- `handle_accept_suggestion` accepts `layer: "allow"` so future
  suggestions can promote to an allow rule just like block.
- `content.js` allow-selector exclusion: `applyRemove()` skips
  any node matching an allow selector; hide CSS appends
  `:not(<allow-selector>)` (modern :not() selector-list syntax,
  Chrome 88+) per hide rule so allowed nodes render.
- Options editor gains an **Allow (exception)** section
  alongside Block / Remove / Hide / Spoof.
- Firewall log enumerates allow rules in the per-rule breakdown.

### Stage 9 phase 3: persistent searchable firewall log
- `BackgroundState.tab_events: HashMap<i32, VecDeque<_>>` replaced
  by `firewall_log: VecDeque<FirewallEvent>` — one global FIFO
  across every tab. Cap raised from 500/tab to 10k total (~2 MB
  serialized; comfortable under the 10 MB `chrome.storage.session`
  quota).
- `FirewallEvent` gains a `tabId` field stamped at emit time by
  `push_firewall_event`. Drives the popup's "This tab" vs "All
  tabs" filter.
- Persistence: `schedule_persist_firewall_log` writes the deque
  to `chrome.storage.session["firewallLog"]` on a 500ms debounce;
  `hydrate_firewall_log` restores it during SW init, same pattern
  as `tabStats` / `tabBehavior`. Log now survives SW cold-wake
  and tab close (used to wipe on both).
- `handle_get_firewall_events` returns the full global log; the
  popup filters client-side. No pagination yet; revisit if the
  per-popup marshal ever feels slow.
- Popup `FirewallLog` component gains filter controls:
  - **All tabs** checkbox (defaults to This-tab; inverts when
    popup opens on a chrome:// page with no active tab).
  - **Action** dropdown: all / block / allow / remove / hide /
    spoof. When narrowed, rows whose action doesn't match are
    hidden entirely (not just greyed out), so "show me just the
    allow hits" actually declutters.
  - **Search** input: substring match across rule_id / match /
    URL / element-description.
  - Row aggregation re-runs reactively on any filter change via
    Leptos signals. `Arc`-shared event + config inputs so the
    closures don't deep-copy on every keystroke.

### Stage 9 phase 4: hide/spoof events + per-rule disable toggle
- Hide events: `content.js` tracks which hide selectors have
  matched at least once on the current page; one
  `FirewallEvent { action: "hide" }` fires per selector per
  navigation. Piggybacks on the existing `hush:stats` message via
  a new `newHideEvents` field — no extra round-trips.
- Spoof events: `mainworld.js` dispatches a
  `__hush_spoof_hit__` CustomEvent on the first
  getParameter-returns-bland hit per (kind, page). `content.js`
  relays that to background as `hush:spoof-hit`. Background maps
  the kind back to its authoring scope (site if matched, global
  otherwise) and emits a `FirewallEvent { action: "spoof" }`.
- Every action now flows through the unified firewall log. The
  Hide/Spoof evidence variants stay `None {}` (CSS hides don't
  have per-element events worth logging; fingerprint reads are
  recorded by rule fire, not by value).
- Per-rule disable toggle in the options editor: every rule row
  gets an enable checkbox. Disabled rows render greyed-out
  (strikethrough + muted colour) but stay in the config so the
  user can flip them back. Evaluator skips `disabled` entries:
  DNR sync excludes them, content-script applier (`toValueList`)
  skips them, `compute_suggestions` ignores them for dedup so
  the detector keeps surfacing matches while a rule is parked.
  Regression test locks the dedup behaviour.

### Stage 11: auto-tags
- Every `Suggestion` now carries the originating detector's
  canonical signal kind in a new `kind` field (`"beacon"`,
  `"canvas-fp"`, etc.). Populated via a new `LearnKind::tag()`
  method that mirrors `from_tag()`. Every detector's
  `BuildSuggestionInput` sets `kind` alongside `learn`.
- Accept-suggestion flow forwards the kind to background. New
  `RuleEntry::from_accepted_suggestion(value, kind)` helper
  stamps `auto:<kind>` into `tags` when the kind is non-empty.
  Empty kind (manual entry, JSON paste) produces an untagged
  rule. Regression test
  `from_accepted_suggestion_stamps_auto_tag` locks both paths.
- Popup firewall log gains a tag-chip filter row. Chips are
  built from the distinct tag set across every authored rule,
  sorted deterministically. Clicking a chip toggles it into the
  active filter; events pass only if their rule carries at
  least one selected tag. Empty tag set renders no chip row.
- `rule_id` -> tags map is pre-built at component mount from
  the config so the per-event filter is a HashMap lookup, not a
  config rewalk on every keystroke.

### Stage 9 phase 5: rule reorder + tags + comments
- Every rule row in the options editor now has up/down move
  buttons. `Vec::swap` in place, persisted immediately.
  `first` row's up-arrow and last row's down-arrow render
  disabled so the UI telegraphs the bounds.
- Second sub-line on every row: `tags` (comma-separated; parsed
  on change into `Vec<String>`, empty entries trimmed) and
  `comment` (free-form; stored as `Option<String>`, `None` when
  the field is blank). Both use the `on:change` event so the
  write doesn't fire on every keystroke.
- Stored shape on disk stays identical — `skip_serializing_if`
  on the new `tags` + `comment` fields means a rule without
  metadata still serializes as `{"value": "..."}` and a rule
  with metadata adds only the fields in use. No migration.

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
- Iter 5: site list + per-site editor ported. `ConfigEditor` owns
  the full `Config` tree and the selected-domain signal; `SiteList`
  + `SiteListRow` render the sidebar with reactive per-layer
  counts; `SiteDetail` handles rename + delete; `LayerSection`
  renders one of the three Block / Remove / Hide fieldsets with
  add + delete. Every mutation persists via new
  `chrome_bridge::set_config<C: Serialize>`. `options.js` collapsed
  from 285 lines to 34 (pure bootstrap); the `.two-pane` /
  `.sidebar` / `.detail` HTML removed in favor of a single
  `#rust-config-root` div.
- Iter 6: content script ported to Rust/WASM. `src/content.rs`
  owns layer application (Remove + Hide CSS injection), DOM scans
  (hidden iframes + sticky overlays + element description), the
  `PerformanceObserver` resource stream, the `MutationObserver`
  that re-applies Remove on mutations, and the `__hush_call__`
  main-world bridge that validates payloads against `SignalPayload`
  before buffering them as `JsCall` entries. `content.js` collapsed
  from 464 LOC to a 32-line dynamic-import bootstrap. Cargo.toml
  grew web-sys features for `PerformanceObserver`,
  `MutationObserver`, `HtmlIFrameElement`, `HtmlStyleElement`,
  `Location`, and related init bags.
- Iter 7: popup bootstrap consolidation. New async wasm-bindgen
  entry `hushPopupMain` owns tab query + all chrome.runtime/storage
  fetches + matched-domain resolution + mount. New Leptos
  components `UnmatchedBanner` + `FooterButtons` replace the
  `<div id="unmatched">` + `<footer>` blocks in `popup.html`. New
  chrome_bridge helpers: `get_active_tab`, `open_options_page`,
  `reload_tab`, `get_debug_info`, `get_tab_stats`,
  `get_rule_diagnostics`, `get_popup_storage`. `popup.js` collapsed
  from 148 LOC to 20. Stage 5 bootstrap-LOC goal met: popup 20 +
  options 34 + content 32 = 86 across the three shims.

### Stage 7: Firewall-style rule engine (in progress)
- Iter 1: global scope. Reserved `__global__` key in the existing
  `Config` map; rules under it apply to every tab alongside
  site-scoped rules. Content script merges global + site at
  evaluation time; options UI pins "Global (all sites)" at the top
  of the site list with a locked header (no rename, no delete).
  New `types::merged_site_config` helper; `types::GLOBAL_SCOPE_KEY`
  constant.
- Iter 2: unified firewall-event platform. `FirewallEvent` type
  (`{t, rule_id, action, scope, match, evidence}`) with
  action-tagged `FirewallEvidence` (Block / Remove / None). Stable
  `rule_id` format: `"{action}::{scope}::{match}"` — matches
  suggestion-key format. Per-tab ring buffer (cap 500) in
  `BackgroundState.tab_events`; wired into DNR `onRuleMatchedDebug`
  and `hush:stats.newRemovedElements` so block + remove hits both
  emit events. `hush:get-firewall-events` message handler +
  `chrome_bridge::get_firewall_events` helper.
- Iter 3: popup Firewall Log section. `FirewallLog` +
  `FirewallLogRow` + `FirewallEvidence` Leptos components at the
  top of the diagnostic sections. Aggregates events by `rule_id`;
  one row per rule with BLOCK/REMOVE/HIDE/SPOOF badge, scope tag
  (`global` vs hostname), match string, hit count, last-hit
  timestamp, click-to-expand recent-evidence panel.
- Fix: `compute_suggestions` dedup now merges global + site
  scopes. Without the merge, a global block rule wouldn't suppress
  a matching suggestion (the suggestion kept re-firing every
  scan). Regression test
  (`global_scope_block_rule_suppresses_suggestion`) locks it.
- Fix: remove events now carry their originating scope so the
  firewall log attributes hits to the same row the rule
  enumeration shows. Previously a rule authored under `__global__`
  showed a zero-hit row and the events landed on the matched-site
  scope, producing confusing double entries.

### Service worker port (background.js -> Rust)
- `src/background.rs` ports the 988-line background service worker
  to Rust. The new module owns every listener (onInstalled,
  onStartup, storage.onChanged, webNavigation.onCommitted,
  tabs.onRemoved, declarativeNetRequest.onRuleMatchedDebug,
  runtime.onMessage), every message handler (hush:stats, hush:log,
  hush:js-calls, hush:scan, hush:get-tab-stats,
  hush:get-rule-diagnostics, hush:get-suggestions,
  hush:accept-suggestion, hush:allowlist-add-suggestion,
  hush:dismiss-suggestion, hush:get-debug-info), DNR rule sync with
  serialize-chain, rule fire tracking, per-tab stats + behavior with
  chrome.storage.session persistence, rule diagnostics. State lives
  in `thread_local! RefCell<BackgroundState>`; SW cold-wake rebuilds
  it via the hydrate path. `background.js` collapsed from 988 LOC
  to 18 (pure wasm bootstrap). Added `manifest.json`
  `content_security_policy.extension_pages` with `wasm-unsafe-eval`
  (required by MV3 for WASM), and a pinned `"key"` so the unpacked
  extension ID no longer churns on reload.

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
