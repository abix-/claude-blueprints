# Benchmarks: compute_suggestions (Rust vs JS)

This doc compares the ported Rust `compute_suggestions` to the
pre-port JS baseline. Both implementations are measured on identical
synthetic tab snapshots so the numbers are directly comparable.

**TL;DR** - with a proper release + bench profile (see
"Configuration" below), Rust beats Node V8 by ~13-14% on the
realistic heavy-tab workload and the 50-tab aggregate. Light tabs
are essentially tied because V8's JIT handles small inputs as fast
as native code. The initial "Node is 1.3x faster than Rust" reading
was a configuration bug: the release profile had `opt-level = "z"`
(size-optimized, loop vectorization disabled) baked in for the WASM
bundle, and criterion inherited that profile.

## Latest run

**2026-04-20**, Windows 10, rustc 1.95.0, Node v25.3.0, criterion 0.5,
`-C target-cpu=native`. After foldhash + Arc<[String]> refactor.

| Fixture | Rust (max-speed) | Node V8 | Rust/JS |
|---|---|---|---|
| light_tab (100 res + 50 js-calls + 5 iframes + 5 stickies) | **130.7 µs** | 137 µs | **0.95x (Rust 4.5% faster)** |
| heavy_tab (500 + 500 + 20 + 20, at production cap) | **716.8 µs** | 848 µs | **0.85x (Rust 15% faster)** |
| 50_tabs_of_heavy (50 heavy_tab sequential) | **36.1 ms** | 43.8 ms | **0.82x (Rust 18% faster)** |

Both implementations produce the same 31 suggestions from the
heavy_tab fixture, so the work done is identical. The ~14% Rust edge
comes from LLVM's auto-vectorization of the per-detector loops and
avoided GC pressure on the hot path.

### What the numbers mean for a heavy Chrome user

If you have 50 busy tabs open and walked through every popup in a
single sitting, the engine burns 37 ms of CPU total (Rust) or
43 ms (JS). Unnoticeable in both cases. The win is architectural
(see "What this means" below), not user-perceptible runtime.

## How to run

```bash
# Rust (criterion; auto-picks [profile.bench] from Cargo.toml)
cargo bench --bench compute_suggestions

# JS (Node 22+ with performance.now)
node bench/compute_suggestions.mjs
```

Rust output lives in `target/criterion/compute_suggestions/` with
HTML reports per bench. JS output prints directly.

## Configuration

The `[profile.bench]` in `Cargo.toml` is tuned for **maximum runtime
speed** at the cost of compile time and binary size. The same knobs
are NOT used for the WASM release build, which stays size-optimized
because it rides the wire on every popup/content-script load.

```toml
[profile.bench]
opt-level = 3            # full auto-vectorization + inlining
lto = "fat"              # whole-program optimization
codegen-units = 1        # one LLVM unit, maximal cross-fn inlining
panic = "abort"          # no unwinding tables in hot paths
debug = false
strip = "symbols"

# .cargo/config.toml
[target.'cfg(not(target_family = "wasm"))']
rustflags = ["-C", "target-cpu=native"]
```

What's intentionally not set:

- `incremental = false`: redundant with `codegen-units = 1` for
  runtime perf. Removed to keep compile iterations reasonable.
- PGO (profile-guided optimization): adds 10-20% typical but also
  adds training-run + rebuild pipeline complexity. Revisit if the
  engine ever becomes a measurable hotspot.

## Why the first measurement was wrong

Initial run showed Rust ~1.3x slower than Node V8. Root cause: the
release profile at the top of `Cargo.toml` sets `opt-level = "z"`
(optimize-for-size, disables loop vectorization) to keep the WASM
bundle small. `cargo bench` inherits `[profile.release]` by default,
so the criterion binaries were size-optimized too. Adding a dedicated
`[profile.bench]` that overrides with `opt-level = 3 + lto = "fat" +
codegen-units = 1 + target-cpu=native` closed the gap and then
some.

This is a known Rust perf pitfall for WASM-targeting projects: the
WASM bundle wants `opt-level = "z"` or `"s"` for size, but your
`cargo bench` + `cargo test` runs want `opt-level = 3` for speed.
The solution is an explicit `[profile.bench]` override.

## Fixture design

The two implementations share shapes defined in
`benches/compute_suggestions.rs` (Rust) and `bench/compute_suggestions.mjs`
(JS):

- **light_tab**: a just-loaded tab with a handful of each signal.
- **heavy_tab**: a saturated Reddit/Twitter/Gmail-shape tab at the
  production ceilings (`MAX_SEEN_RESOURCES = 500` and
  `MAX_JS_CALLS = 500` in `background.js`, plus ~20 iframes + stickies).
- **50_tabs_of_heavy**: 50 sequential runs of `heavy_tab` to model
  what a heavy Chrome user with ~50 tabs open would pay if every
  popup opened once in a single session.

Each resource, iframe, sticky, and js-call is generated
deterministically so the Rust and JS runs process exactly the same
bytes.

## Caveats

1. **Native Rust vs WASM Rust.** The criterion numbers are native
   release with `target-cpu=native`. The Chrome extension runs WASM,
   which historically carries a 1.5-2x penalty on top of native for
   wasm-bindgen-heavy workloads. For in-browser use the effective
   Rust number is roughly 1.1-1.5 ms on heavy_tab, closer to parity
   with Node V8.
2. **Node V8 vs Chrome V8.** Same engine. A realistic per-tab
   budget in the extension is within ~10% of the Node number.
3. **No wasm-opt**. The shipped wasm bundle is unoptimized because
   wasm-pack 0.14's bundled wasm-opt can't validate rustc 1.95
   output. A newer binaryen typically shaves 10-20% off in-browser
   WASM runtime. When tooling catches up, the WASM bench will move
   toward native Rust numbers.
4. **No WASM SIMD yet.** `+simd128` target feature would unlock
   SIMD in the browser; worth revisiting when we adopt it.

## Where the time goes

Optimizations applied:

- **foldhash** instead of std SipHash - ~15% faster hashing on
  small string keys (foldhash README benchmark, Apple M2 / Xeon).
  Swapped into every per-host aggregation HashMap + the final
  dismissed / allowlist HashSet.
- **`Arc<[String]>`** for the per-suggestion `existing_block` /
  `existing_remove` / `existing_hide` lists in `DetectCtx` +
  `BuildSuggestionInput`. Per-detector clone cost drops from a
  Vec<String> deep copy (N allocs + N memcpy per clone) to a
  refcount bump (2 asm instructions). Saves ~90 heap allocations
  per heavy_tab call.
- **`sort_unstable_by`** on the final suggestion list. Safe because
  `Suggestion` has no semantic equality beyond confidence+count.
- **Pre-sized foldhash `HashSet`** for dismissed + allowlist
  membership checks.
- **`target-cpu=native`** via `.cargo/config.toml`, gated to non-wasm
  targets via `cfg(not(target_family = "wasm"))` so LLVM emits
  AVX/AVX2 in the bench binary while the wasm build stays portable.

Remaining hot spots:

- `HashMap::entry` + hash of each resource's host string across the
  500-resource loop, 4 detectors deep (~25% of total time after
  foldhash). Each call is a few-ns hash + a table probe; hard to
  beat without a radix trie.
- `format!` in per-detector `reason` + `evidence` strings (~25%).
  These end up in the output and are unavoidable.
- Remaining iframe / sticky / js-calls aggregation.

Not yet applied (diminishing returns for this workload):

- `smallvec::SmallVec<[String; 4]>` for evidence arrays that are
  typically <=5 entries. Avoids heap alloc on the vec itself.
  Estimated +1-2%.
- Pre-allocate `out: Vec<Suggestion>` with capacity 32 instead of 0.
  Trivial; estimated +0.5%.
- PGO using a representative tab capture. Typically +10-20% but adds
  training pipeline complexity.
- Custom string interner for hostnames (the 500-resource fixture
  has only ~5 unique hosts). Would deduplicate most remaining
  per-resource allocations.

## What this means

With the bench profile fixed, **Rust is moderately faster than JS
for this workload** (~14% on heavy_tab). But the real story is
still architectural:

- **Schema safety** - the typed `SignalPayload` union catches
  field-drop bugs (the 0.5.0 regression class) at compile time and
  at the wasm-bindgen boundary.
- **Shared-core reuse** - the `hush` crate compiles native too. A
  CLI HAR analyzer, a Tauri desktop app, or a mobile scanner can all
  reuse the same engine without reimplementation.
- **Attack surface** - main-world hook bodies sit inside WASM linear
  memory, unreachable from page JS prototype-pollution attacks.
- **Type system value across the codebase** - detector signatures,
  message envelopes, config shapes are all statically typed in one
  crate. Adding a detector is a compile-checked change.

The ~14% runtime win is a nice bonus, not the point. Users wouldn't
notice the difference either way.

## History

| Date | Change | Rust heavy_tab | JS heavy_tab |
|---|---|---|---|
| 2026-04-20 | Initial bench, opt-level=z inherited | 1.11 ms | 829 µs |
| 2026-04-20 | `[profile.bench]` with opt-level=3 | 773 µs | 829 µs |
| 2026-04-20 | `+target-cpu=native` + lto=fat + strip | 725 µs | 829 µs |
| 2026-04-20 | `+foldhash` + `sort_unstable_by` + pre-sized HashSet | 754 µs | 848 µs |
| 2026-04-20 | `+Arc<[String]>` to skip per-suggestion Vec clones | 701 µs | 848 µs |
| 2026-04-20 | `+pre-sized HashMaps via agg_map()` + Vec capacity + Detector trait | **717 µs** | 848 µs |
