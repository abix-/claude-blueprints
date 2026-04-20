# History

Retired rollout notes. Move material here when its stage is marked
complete in [roadmap.md](roadmap.md). Present-tense feature behavior
lives in [completed.md](completed.md) and the per-subsystem docs;
this file is chronology.

## Stage 3: Main-world hooks in Rust (0.10.0)

Typed `SignalPayload` enum landed in `src/types.rs` with one variant
per hook kind (fetch, xhr, beacon, ws-send, canvas-fp, font-fp,
webgl-fp, audio-fp, listener-added, replay-global, canvas-draw).
serde's `#[serde(tag = "kind")]` gives us a discriminated union that
rejects missing required fields at the wasm-bindgen boundary - the
0.5.0 bug class becomes a compile error in Rust tests and a runtime
error (with loud console.error) in the live extension.

`src/main_world.rs` exports `dispatchHook(detail)` and
`drainStubQueue(queue)` via wasm-bindgen. Both deserialize into
`SignalPayload`, re-serialize to canonical shape, and dispatch a
`__hush_call__` CustomEvent via `web_sys::CustomEvent` +
`CustomEventInit::set_detail` + `EventTarget::dispatch_event`.

`mainworld.js` rewired to the hybrid bootstrap pattern. At
document_start, synchronous JS stubs install on every target
prototype and push captured invocations onto `window.__hush_stub_q__`.
In parallel, the bootstrap dynamically imports
`chrome.runtime.getURL("dist/pkg/hush.js")` + `await init()` +
`initEngine()`. Once ready, queue is drained through
`drainStubQueue`, a flag flips, and new hook calls go through
`dispatchHook` directly.

Design deviation from the approved plan: the plan called for Rust to
re-patch prototypes via `js_sys::Reflect::set` + `Closure`.
wasm-bindgen's closures don't forward the implicit JS `this` binding;
the only way to install a `this`-capturing function from Rust without
per-hook JS shims is `new Function()` from a string, which requires
`unsafe-eval` CSP and is blocked on many target sites (including
github.com, several banks). JS therefore keeps the prototype
assignments. Rust owns everything after the capture - payload typing,
validation, CustomEvent construction, dispatch.

Net change: `mainworld.js` 412 -> 376 lines; payload typing moves to
Rust; 12 new cargo tests cover SignalPayload round-trip per variant
+ required-field enforcement.

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
