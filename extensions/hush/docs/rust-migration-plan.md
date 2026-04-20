# Rust migration plan

This doc captures the plan to port Hush from JavaScript to a max-Rust
architecture. Detection engine, UI, content script, and main-world hooks
all move to Rust compiled to WASM. Only irreducible JS bootstrap shims
remain (~75 lines total across 5 entry points).

## Thesis

Hush today is DOM-manipulation glue. Hush's roadmap
(`docs/heuristic-roadmap.md` Tier 3 nav reads, Tier 4 supercookies, Tier 6
SW disclosure, eventual cross-site correlation, filter-list integration,
on-device ML classification) is a detection engine. Rust wins detection
engines at scale. Make the port now while the codebase is small and there
is no user-facing JS API to preserve.

## Why max-Rust wins at scale

### 1. Attack surface

`mainworld.js` runs inside hostile page JS context. 3000 lines of JS is
not auditable; 50 lines of bootstrap is. Once hook bodies are Rust
closures executing in WASM linear memory, page scripts cannot reach into
them with prototype pollution. The page can still poison a prototype
before we install, but once our closure is in place its body is inside
WASM where page JS cannot poke.

### 2. One type system, one language

The 0.5.0 `emit()` bug was schema drift across the JS-to-JS boundary
between `mainworld.js` and `background.js`. In max-Rust, both ends share
the same `#[derive(Serialize, Deserialize)]` struct. Drift becomes a
compile error. Mixed Rust+TS still has a JS-Rust serialization boundary
where drift is possible; max-Rust collapses that boundary.

### 3. UI perf

Popup cold-opens in Yew or Leptos are faster than React equivalents
because WASM skips JS parse + JIT warmup. On MV3 service-worker wake, the
JS popup has to re-parse `popup.js`; compiled WASM loads from a
cache-friendly binary. Measurable on slower machines.

### 4. Cargo tooling

`cargo test`, `cargo bench` (criterion), `cargo clippy`, `cargo miri`
(undefined behavior detector). JS has `vitest` / `jest` / `mocha` and a
handful of flaky microbench libraries. No JS equivalent of miri. The
tooling gap is a real quality gap.

### 5. Deterministic supply chain

No npm. No 1000-package transitive dependency sprawl. `Cargo.lock` with a
smaller, more-reviewed crate ecosystem. Supply-chain attack surface
collapses.

### 6. Shared-core everywhere

Rust engine + Yew UI + wasm-bindgen compile to WASM for the extension
today. The same crates build as:

- Native desktop shell (Tauri) for a companion privacy dashboard.
- Native CLI that ingests HAR files and emits detection reports
  (CI-usable against offline traffic captures).
- Mobile via `uniffi` for iOS/Android companion scanners.
- Node native module for a server-side replay test harness.

Mixed Rust+TS does not give you Yew UI on native. Max-Rust does.

### 7. Happiness dividend

The author ships Rust daily on abixio. Rust is the native cognitive mode;
TS is context-switch cost on every edit. Max-Rust is one language, flow
state, more features per session. Not vanity; velocity.

### 8. Long-term rot profile

Rust 2018 code still compiles clean in 2026. JS from 2020 with its
webpack/babel/rollup/esbuild churn may not. Over 5-year horizons Rust
extensions have lower maintenance cost.

### 9. Detection-engine trajectory

Every tier from here (Tier 3 nav reads, Tier 4 supercookies, Tier 6 SW
tracking, filter-list integration, cross-site correlation, eventual
client-side ML classification) is CPU work. Each is another point where
Rust WASM pulls ahead at scale. Starting Rust now pays compounding
interest.

### 10. Design pressure

Writing the UI in Yew forces explicit state machines, typed props,
signal-based reactivity. You cannot fling an ad-hoc `$stateful.X = "..."`
at a Yew component. Extensions written in Yew tend to have cleaner
architectures than their React or vanilla-JS siblings.

## Target architecture

```
extensions/hush/
  Cargo.toml                      # workspace root
  crates/
    types/                        # Suggestion, SignalPayload, MessageEnvelope, etc.
    engine/                       # computeSuggestions, buildSuggestion, LEARN_TEXT, dedup, canonicalization
    detectors/                    # per-signal aggregators: canvas-fp, webgl-fp, audio-fp, font-fp, replay, raf-waste
    storage/                      # chrome.storage wrapper, config + allowlist schema + validation
    bg-worker/                    # service worker entry: event listeners, message routing, DNR sync
    content-script/               # isolated-world entry: DOM scans, MutationObserver, hook bridge
    main-world/                   # MAIN-world entry: prototype hooks via wasm-bindgen closures
    ui-popup/                     # Yew or Leptos popup
    ui-options/                   # Yew or Leptos options page
  src/bootstrap/                  # tiny JS shims (~15 lines each)
    background.js
    content.js
    mainworld.js
    popup.js
    options.js
  static/                         # unchanged HTML + JSON + manifest
    popup.html
    options.html
    manifest.json
    sites.json
    allowlist.defaults.json
  dist/                           # esbuild + wasm-pack output; this is what Chrome loads
  test/                           # pure cargo test with proptest for property-based coverage
```

Endgame ratio: ~95% Rust, ~5% JS bootstrap.

## What stays JavaScript (the irreducible minimum)

- Bootstrap shims that load the WASM module. ~15 lines per entry, 5
  entries, ~75 lines total.
- Static `<script type="module" src="bootstrap.js">` tags in
  `popup.html` / `options.html`.
- `manifest.json` because it is JSON.

That is it.

## Honest costs

- **Build time.** `wasm-pack --dev` in watch mode rebuilds in 1-2
  seconds. Not the <100ms that esbuild gives, but not crippling.
- **Bundle size.** WASM + bindings per entry is ~150-300KB vs ~10-30KB
  per equivalent TS bundle. Extensions have no hard size ceiling, but
  users notice install sizes above a few MB.
- **Learning curve for main-world closures.** Assigning
  `wasm_bindgen::closure::Closure<dyn FnMut(...)>` to
  `HTMLCanvasElement.prototype.toDataURL` is a new pattern. First
  implementation is hairy; subsequent hooks are mechanical.
- **Yew / Leptos verbosity.** Roughly 2x LOC vs equivalent plain JS for
  UI code, in exchange for types and reactivity.
- **Port effort.** 3-5 focused sessions versus 1-2 for TS.

## Session plan

### Session A - scaffold + first port

- Create `Cargo.toml` workspace at `extensions/hush/`.
- Create `crates/types`: `Suggestion`, `SuggestionLayer`, `SignalKind`,
  `SignalPayload` discriminated union, `Allowlist`, `SiteConfig`,
  `MessageEnvelope`.
- Create `crates/engine`: port `LEARN_TEXT` (as `const` module),
  `buildSuggestion` (as `fn build_suggestion`), allowlist matching
  (`isLegitHiddenIframe`, `matchesAllowlist`), URL canonicalization,
  `patternKeyword`, `scriptOriginFromStack`.
- Set up `wasm-pack` build into `dist/pkg/`.
- Create `src/bootstrap/background.js` that loads the WASM and wires
  chrome.runtime.onMessage into a single exported `handle_message` fn.
- Keep the rest of `background.js` calling `compute_suggestions` in JS
  temporarily so the extension still runs during transition.
- Update `build.mjs` (or replace it) to run `wasm-pack` + esbuild for
  the bootstrap shim + copy static assets to `dist/`.
- Update `manifest.json` to point at `dist/` bundles.
- Update `test/` to validate that `build_suggestion` Rust output matches
  the old JS output against a fixture set.

Outcome: the pipeline works end-to-end. First JS-to-Rust migration is
visible in the running extension.

### Session B - detection engine

- Port all detector aggregators to Rust:
  - canvas-fp
  - webgl-fp (hot + general)
  - audio-fp
  - font-fp
  - replay vendor globals
  - replay listener density
  - raf-waste
  - sendBeacon targets
  - tracking pixels
  - first-party telemetry subdomains
  - polling endpoints
  - hidden iframes
  - sticky overlays
- Port `computeSuggestions` to `fn compute_suggestions(state: &BehaviorState, config: &Config) -> Vec<Suggestion>`.
- Delete the JS `computeSuggestions` function. Background service worker
  becomes a thin `chrome.*` wrapper around the Rust engine.
- `cargo bench` + criterion baseline for the engine.

Outcome: the detection engine is 100% Rust. Background is ~90% JS glue
for chrome.* APIs + message routing.

### Session C - main-world hooks

- Port `mainworld.ts` hook logic to `crates/main-world`.
- Install hooks from Rust by constructing `wasm_bindgen::closure::Closure`
  instances and assigning them to prototype methods via `js_sys::Reflect::set`.
- The hook body itself is Rust; it captures the stack via a small JS
  `new Error().stack` call, serializes the payload into a `SignalPayload`
  struct, and dispatches the CustomEvent via `web_sys`.
- Allocate extra time. This is the trickiest session because of the
  prototype-patching dance and the "hook must never break the page" rule.

Outcome: `mainworld.js` shrinks to a ~20-line bootstrap that loads the
WASM and invokes `main_world_install()`.

### Session D - popup UI

- Choose framework: Yew (stable, React-like) or Leptos (fine-grained
  reactive). Recommend Leptos for better bundle size and perf, Yew for
  more contributor familiarity.
- Port popup rendering: matched-site header, activity section, block
  diagnostics, suggestions list, debug button.
- Keep `popup.html` as a thin shell with a `<div id="app">` root; the
  Rust UI renders into it.

Outcome: popup is Rust-driven. `popup.js` is a ~15-line bootstrap.

### Session E - options UI + content script + cleanup

- Port options page to same framework (Yew or Leptos).
- Port `content.js` DOM scanning to `crates/content-script` using
  `web_sys` for `querySelectorAll` / `getComputedStyle` /
  `getBoundingClientRect`.
- Port `MutationObserver` setup to `web_sys::MutationObserver`.
- Port `PerformanceObserver` setup to `web_sys::PerformanceObserver`.
- Final cleanup: delete remaining JS logic. What survives: ~75 lines of
  bootstrap shims.

Outcome: ~95% Rust, ~5% JS bootstrap. HTML and CSS unchanged.

## Crate inventory

Principle: use popular maintained crates until they suck. Reinvent only
when the wheel is old, unmaintained, or genuinely wrong for the job.

### Adopted in Session A

| Crate | Role | Why |
|---|---|---|
| `serde` + `serde-wasm-bindgen` | JS/Rust value conversion | Official preferred approach per wasm-bindgen docs; smaller code size than JSON-based serde bindings. |
| `wasm-bindgen` + `js-sys` | FFI with JS | The foundation. No alternative. |
| `url` | RFC 3986 parsing | Used by reqwest, hyper, the entire Rust HTTP ecosystem. Handles punycode/IDN/encoded segments that hand-rolled parsing misses. ~80 KB WASM cost accepted. |
| `indexmap` | Ordered hash map | Config key order preservation. Matches JS object iteration semantics better than `BTreeMap`'s alphabetical sort. |
| `console_error_panic_hook` | Rust panic to DevTools | Behind `panic-hook` feature flag. ~4 KB, worth it for debuggable WASM. |

### Planned for Session B (detectors + computeSuggestions)

| Crate | Role |
|---|---|
| `thiserror` | Structured errors at the wasm-bindgen boundary. |
| `proptest` (dev-dep) | Property-based fuzz tests for aggregators and canonicalization. |
| `insta` (dev-dep) | Snapshot/golden tests for JS-vs-Rust parity of `computeSuggestions`. |

### Planned for Session C (main-world hooks)

| Crate | Role |
|---|---|
| `web-sys` | Typed bindings to browser APIs. Canonical wasm-bindgen companion. |
| `gloo-events` / `gloo-timers` | Ergonomic wrappers around DOM events and `setTimeout`. Smaller and more Rust-native than hand-rolling via `web-sys`. |
| `wasm-bindgen-test` (dev-dep) | Runs tests in a real browser via WASM. Needed once hooks touch `document`, `Performance`, etc. |

### Planned for Session D/E (UI)

| Crate | Role |
|---|---|
| `leptos` (recommended) or `yew` | Rust WASM UI framework. Leptos wins on bundle size and fine-grained reactivity; Yew wins on contributor familiarity. Decision deferred to Session D. |
| `gloo-utils` | DOM utilities both UI frameworks already use. |

### Benchmarking + tooling (any session)

| Crate | Role |
|---|---|
| `criterion` (dev-dep) | Statistical benchmarks with regression gates. Replaces the absence of JS benchmarking. |
| `log` + `console_log` | Bridge Rust `log!` macros to Chrome DevTools console. |

### Deferred until there is a reason

| Crate | Reason to skip now |
|---|---|
| `regex` / `regex-lite` | No Rust-side regex work yet. DNR does URL matching natively; if we add filter-list rules, revisit. |
| `blake3` / `xxhash-rust` | No fingerprint hashing yet. Revisit when we do cross-site correlation. |
| `ahash` / `foldhash` | No hot hash-table loops. std is fine. |
| `bitflags` | No large bitflag surfaces yet. |

## Testing strategy

- Replace `test/emit_contract.test.mjs` with `cargo test` in
  `crates/main-world/tests/`.
- `proptest` for property-based tests against the detector aggregators
  ("for any sequence of canvas-draw events, the output suggestion's
  invisible ratio is always within [0, 1]").
- `criterion` benchmarks for regression gates on the hot paths:
  `compute_suggestions`, per-detector aggregation, allowlist matching.
- End-to-end tests against the built extension via Puppeteer (the
  separately-recommended "#3 E2E test" from the kovarex review) still
  apply; they now run against the Rust-backed build.

## Rollout

Port runs on a feature branch. The Rust build produces the `dist/`
directory that Chrome loads. Old JS files remain in the repo root until
Session E so "git bisect" can find regressions and so partial rollback
is possible during each session.

Final commit of Session E deletes the legacy JS. Old files are not
retained; `git log` is sufficient history.

## When not to do this

This plan assumes:

- Hush remains actively developed on the detection-engine trajectory.
- At least 3-5 sessions are available for the port before more features
  are stacked on top.
- The author is comfortable with wasm-bindgen / web-sys idioms (or is
  willing to learn; Session A is the cheapest place to discover fit).

If Hush freezes at its current feature set and goes into pure
maintenance, the port is not worth it. The TS-migration alternative is
cheaper and solves the schema-drift problem without the bundle-size or
build-time costs.

## Related docs

- `docs/heuristic-roadmap.md` - detection tiers driving the trajectory
  this plan is designed around.
- `CHANGELOG.md` - history of the schema-drift bugs that motivate
  unifying the type system.
