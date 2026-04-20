# History

Retired rollout notes. Move material here when its stage is marked
complete in [roadmap.md](roadmap.md). Present-tense feature behavior
lives in [completed.md](completed.md) and the per-subsystem docs;
this file is chronology.

## Stage 5 (in progress): Options + content-script cleanup

**Iter 3 (allowlist editor)**: three user-editable allowlists
(iframes / overlays / suggestion keys) moved into the Leptos
`AllowlistEditor` component. `mount_options` grew a second mount
call so the editor renders into a separate `#rust-allowlist-root`
inside the existing `<details>` wrapper - the surrounding
`<summary>` + intro paragraph stay in plain HTML. New chrome_bridge
helpers: `set_allowlist` writes the three-field triple back to
`chrome.storage.local["allowlist"]`; `get_default_allowlist` fetches
`allowlist.defaults.json` and parses the triple out of the JSON. The
editor owns three `RwSignal<String>`s, splits them on save via a
`lines_to_list` helper that matches the JS `linesToList` semantics,
and renders status through the shared `setOptionsStatus` export.
`loadDefaultAllowlist` + `linesToList` + `DEFAULT_ALLOWLIST` deleted
from `options.js`.

**Iter 2 (config toolbar)**: Export JSON + Reset to defaults ported
to the Leptos `ConfigToolbar` component. Two new chrome_bridge
helpers: `get_config_json` reads `chrome.storage.local["config"]` and
stringifies with `js_sys::JSON::stringify_with_replacer_and_space` so
indentation matches the old JS `JSON.stringify(config, null, 2)`;
`reset_config_to_defaults` hits `chrome.runtime.getURL("sites.json")`
+ `fetch` + `.json()` + writes the seed back to storage. Export
triggers a download by creating a `web_sys::Blob::new_with_str_sequence`,
an object URL, and a synthetic anchor click. Reset issues
`window.location().reload()` so the still-JS-owned site list + JSON
editor pick up the new config from storage. `options.html` lost the
`<div class="toolbar">` wrapper that held the two buttons.

**Iter 1 (options scaffold + preference toggles)**: new
`src/ui_options.rs` mirrors the shape of `src/ui_popup.rs`.
`mountOptions(snap)` wasm-bindgen export takes an `OptionsSnapshot`
with the two boolean preferences (`debug`, `suggestionsEnabled`) and
mounts a Leptos subtree at `#rust-options-root`. `SettingsToggles`
owns the two checkboxes; each click calls the newly-generalized
`chrome_bridge::set_option_bool(key, value)` helper (extracted from
the old `enable_detector` body). `StatusBanner` renders the
transient green/red save-confirmation message via an
`RwSignal<Option<StatusMsg>>` stored in a `thread_local!` - `options.js`
and any other legacy JS handler can publish messages into the same
banner via the exported `setOptionsStatus(msg, ok)` wasm function.
`options.js` converted from a classic `<script src>` to
`<script type="module">` so the static `import initWasm, { ... } from
"./dist/pkg/hush.js"` works the same way `popup.js` does.

## Stage 4 (in progress): Popup UI in Leptos

Framework pick: Leptos 0.8. Smallest bundle of the Rust WASM web
frameworks (~25KB framework cost vs Yew's ~110KB), fine-grained
signals match how the popup actually mutates (suggestion list changes
on Add / Dismiss / Allow clicks), strongest community trajectory.
See [roadmap.md](roadmap.md) Stage 4 entry for the per-component
checklist.

Shipped across four iterations:

**Iter 1 (scaffold)**: Leptos 0.8 added to `Cargo.toml` (csr feature
only). New `src/ui_popup.rs` with a stub `PopupHeader`. Bootstrap in
`popup.js` dynamically imports the WASM bundle and calls a
`mountPopup` wasm-bindgen export. `popup.html` grew a
`<div id="rust-popup-root">` mount point.

**Iter 2 (matched-site + activity summary)**: `MatchedSite` +
`ActivitySummary` components own the popup header. `popup.js` hands
the computed hostname / matched domain / per-layer counts to Leptos
in one snapshot via `mountPopup(snap)`. The old `<div id="match">` +
its JS `.textContent` / `.innerHTML` writes deleted.

**Iter 3 (suggestions list + actions)**: full suggestions UI in Rust.
`SuggestionsList` + `SuggestionRow` components render the list; Add /
Dismiss / Allow buttons call new async helpers in
`src/chrome_bridge.rs` (`accept_suggestion`, `dismiss_suggestion`,
`allowlist_suggestion`). Each helper serializes the message, awaits
the returned Promise via `wasm-bindgen-futures::JsFuture`,
deserializes the reply. Rows go busy/disabled during in-flight
mutations. The 255-line block of JS renderers
(`renderSuggestions` / `refreshSuggestions` / `renderSuggList` /
`renderSuggRow`) deleted. `POPUP_HANDLE` thread_local holds the
popup's suggestions signal + tab id so external callers (JS enable /
scan-once / rescan) can trigger a re-fetch via an exported
`refreshPopupSuggestions` without remounting.

**Iter 4 (Why? / Evidence panels)**: per-row expandable panels for
the dedup diagnostic (`WhyPanel` over `SuggestionDiag`) and the raw
evidence array (`EvidencePanel` with a Copy button that writes to
`navigator.clipboard.writeText`). Each panel is an independent
`RwSignal<bool>` so one expanded doesn't collapse the other.

**Iter 5 (detector CTA)**: the Enable / Scan-once / Rescan row is now
a Leptos `DetectorCta` component. Uses two new `chrome_bridge`
helpers: `enable_detector` (reads chrome.storage.local["options"],
merges `suggestionsEnabled: true`, writes back) and `scan_once`
(chrome.tabs.sendMessage with `hush:scan-once`). Deleted the JS
handlers for `#sugg-enable` / `#sugg-scan-once` / `#sugg-rescan` and
the `<div id="suggestions-block">` wrapper from `popup.html`.

**Iter 6 (Blocked section)**: the Blocked (network) section ported to
Leptos as `BlockedSection`. Groups recent blocked URLs by pattern,
renders a collapsible per-URL evidence list with a Copy button, and
shows the per-rule diagnostic panel (firing / no-traffic /
pattern-broken status badges plus broken-pattern hints). New types
`BlockedUrl` + `BlockDiagnostic` landed in `src/types.rs`. Deleted the
JS renderers `renderBlockedList` + `renderBlockDiagnostics` +
`escapeHtml` (~170 LOC) and the `#block-count` / `#block-list` /
`#block-evidence` / `#block-diagnostics` DOM anchors in `popup.html`.

**Iter 7 (Removed + Hidden sections)**: the last two per-section JS
renderers ported. `RemovedSection` + `RemovedEvidence` +
`HiddenSection` components replace `renderSelectorList` +
`renderRemovedEvidence` + `makeCopyButton` + `timeOnly` (Rust's
`js_sys::Date::to_time_string` slice replaces the JS helper). New
type `RemovedElement` in `src/types.rs`; `PopupSnapshot` carries
`remove_selectors` + `hide_selectors` as `IndexMap<String, u32>` to
preserve the content script's insertion order. The `#sections` div
is gone from `popup.html` - all diagnostic sections live inside the
Leptos tree now. Stage 4 is fully `[x]` modulo the 100ms cold-open
verification.

Bundle trajectory: ~552KB (pre-Leptos) -> ~580KB (iter 2) -> ~652KB
(iter 3, adds signals + async runtime) -> ~688KB (iter 4, clipboard
+ expandable panels) -> ~NNNKB (iter 5) -> ~NNNKB (iter 6) ->
~NNNKB (iter 7). Numbers are unoptimized (wasm-opt still disabled
until the bundled binaryen catches up with rustc 1.95's
nontrapping-fptoint).

Remaining before Stage 4 is fully [x]: verify cold-open render time
against the 100ms budget in DevTools Performance (no code changes
expected, only measurement).

## Stage 3: Main-world hooks in Rust (0.10.0)

Typed `SignalPayload` enum landed in `src/types.rs` with one variant
per hook kind (fetch, xhr, beacon, ws-send, canvas-fp, font-fp,
webgl-fp, audio-fp, listener-added, replay-global, canvas-draw).
serde's `#[serde(tag = "kind")]` gives a discriminated union that
rejects missing required fields at the wasm-bindgen boundary - the
0.5.0 bug class becomes a compile error in Rust tests and a runtime
error (with loud console.error) in the live extension.

`src/main_world.rs` exports `hush_install_from_js(orig, makeWrapper)`,
`dispatchHook(detail)`, and `drainStubQueue(queue)` via wasm-bindgen.
`install_from_js` re-patches target prototypes via
`js_sys::Reflect::set` with Rust-backed `Closure`s. The `this`-binding
problem that wasm-bindgen closures can't solve natively is handled by
a one-line JS factory (`makeWrapper`) that receives the Rust closure
+ the original method + a kind tag and returns a `this`-capturing
wrapper. Rust calls the factory for each hook and does the
`Reflect::set` itself.

Rust-installed wrappers cover the simple prototype methods:

- `HTMLCanvasElement.toDataURL` -> canvas-fp
- `HTMLCanvasElement.toBlob` -> canvas-fp
- `CanvasRenderingContext2D.getImageData` -> canvas-fp
- `CanvasRenderingContext2D.measureText` -> font-fp
- `WebGLRenderingContext.getParameter` -> webgl-fp
- `WebGL2RenderingContext.getParameter` -> webgl-fp

The Rust dispatch path builds a typed payload, validates via
`SignalPayload` serde, and dispatches `__hush_call__` through
`web_sys::CustomEvent`. Closures live forever in a `thread_local!`
`RefCell<Vec<Closure>>` (WASM is single-threaded; `thread_local!` is
the right primitive).

Complex hooks stay JS-installed because each needs handling that
doesn't map cleanly to the factory pattern:

- fetch / XHR / sendBeacon / WebSocket.send - custom arg extraction
  for URL/method/body previews
- OfflineAudioContext - constructor replacement via
  `Reflect.construct`, not a prototype method
- EventTarget.addEventListener - filter by `this === document ||
  window || body` + replay-event-type set
- Canvas 2D draw ops - per-canvas `WeakMap` throttle for visibility
  sampling
- Replay-global poll - not a hook at all, just a periodic
  `setTimeout` poll

These hooks still flow through Rust validation via
`emit() -> dispatchHook()`.

`mainworld.js` rewired to the hybrid bootstrap: synchronous JS stubs
install on every target prototype at document_start and push captured
invocations onto `window.__hush_stub_q__`. In parallel, the bootstrap
dynamically imports `chrome.runtime.getURL("dist/pkg/hush.js")` +
`await init()` + `initEngine()`. Once ready, Rust's
`hush_install_from_js` swaps the simple stubs for Rust-backed
wrappers and drains the queue through the typed dispatch path.

`manifest.json` gained `web_accessible_resources` entries for
`dist/pkg/hush.js` and `dist/pkg/hush_bg.wasm` so the MAIN-world
bootstrap can dynamically import the WASM glue.

## Layout restructure: workspace -> single crate (0.9.0)

Rust work was initially packaged as a Cargo workspace (`crates/types`,
`crates/engine`, `crates/main-world`) with a "Session A through E"
plan doc. Restructured to match the
[endless](https://github.com/abix-/endless) model: one crate, flat
`src/`, numbered Stages with "Done when:" criteria,
[completed.md](completed.md) + [history.md](history.md) rotation.

Why: the workspace added mental overhead for a project this size. Three
crates compiling to separate bundles didn't buy anything the single
crate doesn't already give us. The existing `hush-types` and
`hush-engine` + would-be `hush-main-world` crates collapsed into
`src/types.rs`, `src/compute.rs`, `src/detectors.rs`, etc. One
`Cargo.toml` with `[package]`. `wasm-pack build` produces
`dist/pkg/hush.js` + `dist/pkg/hush_bg.wasm` - one binary shared by
both the service worker and (Stage 3) the main-world bootstrap.

Bundle size trade-off acknowledged: single binary is the union of what
each entry point would need separately. Acceptable at current scope;
Stage 3 can feature-gate heavy deps if any end up unused by
main-world.

## Stage 2: Detection engine + service-worker wiring (0.9.0)

All 13 JS detector aggregators ported to Rust. Previous JS
`computeSuggestions` was 478 LOC; the Rust replacement is 90 LOC in
`src/compute.rs` + 13 detector functions in `src/detectors.rs` +
shared `build_suggestion` in `src/suggestion.rs`.

`background.js` became a `type: "module"` service worker that imports
`computeSuggestions` from the WASM bundle. The 478-line JS
implementation and the 90-line `LEARN_TEXT` constant were deleted;
only a 16-line async wrapper remains (awaits WASM init, hands the
single allowlist object through, catches engine errors).

Orchestrator details: `find_config_entry` handles exact + suffix match
(site.test config covers m.site.test tab); `normalize_block_patterns`
strips trailing `^` so pattern dedup tolerates either form; dismissed
keys and the cross-session `allowlist.suggestions` list are both
applied at the filter step.

Test coverage: 62 cargo tests pass (per-detector unit tests + 7
orchestrator integration tests). JS `emit_contract.test.mjs` stays at
18/18.

## Stage 1: Rust engine core (0.9.0)

Initial scaffold. Rust 1.95, edition 2024, resolver 3. First wave of
pure functions ported from `background.js`:

- `LEARN_TEXT` -> `src/learn.rs` `const` strings + `LearnKind` enum
- `buildSuggestion` -> `src/suggestion.rs` `build_suggestion`
- Allowlist helpers (`isLegitHiddenIframe`, `overlay_allowlisted`) ->
  `src/allowlist.rs`
- URL canonicalization + DNR pattern-keyword extraction via the `url`
  crate -> `src/canon.rs`
- V8 stack-trace host extraction -> `src/stack.rs`

Types in `src/types.rs`: `Suggestion`, `SuggestionDiag`,
`BuildSuggestionInput`, `Allowlist`, `SiteConfig`, `Config` (IndexMap
for insertion-order preservation matching JS object iteration
semantics).

Crate choices: `serde`, `serde-wasm-bindgen`, `wasm-bindgen`, `js-sys`,
`url`, `indexmap`, `console_error_panic_hook`. See
[roadmap.md](roadmap.md) "Related" for the crate-inventory rationale
this used to live in.

wasm-pack 0.14's bundled wasm-opt doesn't validate rustc 1.95 output
(nontrapping-fptoint feature). Disabled wasm-opt in
`Cargo.toml`'s `package.metadata.wasm-pack.profile.*`. Binary ships
unoptimized until tooling catches up.

## LEARN_TEXT teaching inline (0.8.0)

Every suggestion now carries a `learn` field rendered always-visible
under the reason in the popup. One technical paragraph per detection
category; user sees what the signal is and why it's worth blocking
without a click. Covers all 14 current signal kinds. 0.5.0 had the
text hardcoded in background.js; this version centralized it.
The 0.9.0 restructure moved the same strings to `src/learn.rs`.

## Universal Allow button (0.7.0)

Every suggestion row now shows Allow alongside Add / Dismiss. Allow
persists the suggestion's key in `allowlist.suggestions` so the same
detection never surfaces on any site again. Options page grew a third
editable section ("Suggestion allowlist") for manual removal.
Previously Dismiss was per-tab-session only; legit-but-flagged things
came back on every page load.

## Tier 5 invisible-animation-loop detection (0.6.0)

Hooked the hot 2D canvas draw ops (fillRect / strokeRect / clearRect /
drawImage / fill / stroke / putImageData). Each call samples the
target canvas's visibility (viewport intersection +
display:none / visibility:hidden / opacity:0 / sub-2px dimensions)
throttled to at most one sample per 100 ms per canvas. Background
flags origins with 20+ invisible-canvas draws over a 3 s window with
80%+ invisibility ratio. Confidence 70. Closes the original Hush
user story (a hidden Lottie-canvas animation burning 40 %+ CPU).

## Tier 1/2 fix (0.5.1)

`mainworld.js` `emit()` was cherry-picking only
`url`/`method`/`bodyPreview`/`stack`/`t`/`kind` into the CustomEvent
detail, dropping every signal-specific field (`hotParam`, `font`,
`text`, `eventType`, `vendors`, `param`). Downstream detectors in
`background.js` gated on those missing fields and silently did
nothing. Affected signals were: WebGL UNMASKED hot-param read,
font-enumeration, session-replay listener density, session-replay
vendor globals. Fix: spread `data` into detail.

Added `test/emit_contract.test.mjs` to lock the emit contract. Runs
`mainworld.js` in `node:vm` with minimal DOM stubs and asserts every
`__hush_call__` kind round-trips its signal-specific fields. Catches
the original bug; 18 cases covered.

This incident is the concrete motivation for the Rust port: a single
serde-typed boundary between hook and engine would have made the
regression impossible.

## Tier 1 + Tier 2 shipped (0.5.0, then broken until 0.5.1)

Canvas / WebGL / Audio / Font fingerprint detection and session-replay
vendor+listener detection. Shipped with the emit bug above; 0.5.1
fixed it.

## Earlier releases

- **0.4.0**: DNR rules switched to global URL-pattern matches (dropped
  initiatorDomains so iframe traffic is caught). Popup "Why?"
  diagnostic. `docs/heuristic-roadmap.md` gap analysis added.
- **0.3.0**: Main-world hooks for fetch / XHR / sendBeacon /
  WebSocket.send with stack traces. User-configurable
  iframe + overlay allowlists. In-popup block-rule diagnostics.
  Amazon + Reddit case studies.
- **0.2.0**: Behavioral suggestions detector with opt-in toggle.
- **0.1.0**: Initial release. Three layers + two-pane options editor.

Dated release notes live in [../CHANGELOG.md](../CHANGELOG.md).
