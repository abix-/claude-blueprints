---
name: rust
description: Rust development standards and patterns -- concurrency primitives, zero-alloc hot paths, unsafe + FFI doctrine, async patterns, workspace structure, build profiles, testing, crate preferences. Cites canonical examples in public abix- repos. **Rust is the default language for new work.** Use when writing Rust; for ECS / Bevy code read the `bevy` skill, for WGSL shaders the `wgsl` skill.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Rust

**Rust is the default language for new work in this account.**
Reach for another language only when forced by the target
environment (C# for Unity IL2CPP / MelonLoader; GDScript for
Godot; PowerShell for Windows admin; Go where an existing Go
codebase has gravity). When a task could be done in Rust OR
another language and there's no environmental constraint,
pick Rust.

Distilled from doing it across `abix-/Grounded2Mods` (`ueforge`
+ consumer mods), `abix-/abixio` + `abix-/abixio-ui` (S3
server), `abix-/chromium-extensions` (Rust/WASM extension),
and `abix-/endless` (Bevy game). When you need a concrete
canonical example, look in those repos.

For ECS / Bevy code read the `bevy` skill (different idiom).
For WGSL shaders, the `wgsl` skill. For the abixio S3 server,
the `abixio` skill. For UE4SS Rust mod doctrine, `ueforge`.

## Concurrency primitives -- which to reach for

The std types are rarely the right choice. Default picks:

| Need | Use | Why |
|---|---|---|
| Mutex (any) | **`parking_lot::Mutex`** | Faster, smaller, no poison. The standard pick across every project. |
| Read-mostly hot state | **`arc_swap::ArcSwap<T>`** | Snapshot via `load_full()` -- no lock on hot path. Writers swap an `Arc<T>` atomically. Reference site: ueforge's `ProcessEventHook` snapshots the live handler this way; abixio's hot config. |
| Concurrent map | **`dashmap::DashMap`** | Shard-locked. Used for abixio's write cache, read cache, hush's per-host detector state. |
| String-keyed map (small keys) | **`foldhash::HashMap`** | Faster than std SipHash + FxHash + aHash on small-string keys per maintainer benchmarks. Hush + abixio both swapped to it. |
| Atomic flag / counter | `std::sync::atomic::Atomic*` | Std is fine. Use `Relaxed` unless you need a memory order. |
| One-time init | `std::sync::OnceLock<T>` (sync) or `tokio::sync::OnceCell` (async) | Pinned by stable since 1.70. Use this, not `lazy_static` / `once_cell` crates. |
| Channel | `tokio::sync::mpsc` (async) or `crossbeam-channel` (sync) | `std::sync::mpsc` is fine for trivial cases but loses to crossbeam at any non-trivial throughput. |
| Empty-check fast path | `AtomicUsize` shadow | Mirror your real-state length in an atomic so the hot path bails with one `load(Relaxed)` instead of taking the mutex. ueforge's PE_QUEUE does this. |

`Arc<T>` is fine; **do not reach for `Rc`/`Arc` reflexively** to
work around ownership -- restructure ownership first.

## Zero allocations on hot paths

The non-negotiable default in performance-sensitive code:
per-frame callbacks, `ProcessEvent` trampolines, request
handlers, render loops, tight inner loops.

Hot-path budget is single-digit nanoseconds and **zero
allocator traffic per fire**. To meet that:

- Do NOT allocate `String` / `Vec` / `Box` / `HashMap` /
  formatted strings on hot paths. Use `&str`, slices, stack
  buffers (`[u8; N]`), or pre-allocated reusable storage.
- Do NOT lock mutexes unless work is actually about to happen.
  Use atomic shadows: an `AtomicUsize` mirroring queue length
  lets the hot path bail with one relaxed load when the queue
  is empty, without ever taking the mutex.
- Resolve external identifiers (UE object names, file paths,
  hash keys) **ONCE**; cache the resolved pointer / index /
  handle in an `AtomicUsize` / `OnceLock` / `ArcSwap`. Per-fire
  compare is then a pointer / integer compare, not a string
  compare. See ueforge's `LazyFunctionPtr`,
  `find_class_fast` (name-cached), `NameResolver::to_string`
  (FName u64 → String cache), `UClass::cached_native_properties`
  (`Arc<[NativeProperty]>` after first lookup).
- Branch out of the hot path **EARLY**. First lines of any
  per-frame callback: cheap atomic loads + branches that bail
  when there's nothing to do.
- Prefer **`Arc<str>` over `Arc<String>`** for shared read-only
  strings (one less allocation; `Arc::from(s.as_str())`).
- Prefer **`Arc<[T]>` over `Arc<Vec<T>>`** for shared read-only
  slices (same reason).
- For mmap-backed bytes: use **`bytes::Bytes::from_owner(...)`**
  to hand out a `Bytes` view without copying the mmap. abixio
  cold-GET populate uses this.
- For serialization into mmap-backed targets: use
  **`serialize_into(&mut writer, value)`** so bytes flow
  straight from serde into the mmap without an intermediate
  `Vec`. abixio's `needle::serialize_into` does this.

Cold paths (init, user clicks, error reporting) can allocate
freely. Discipline is about hot paths only. Apply from day
one -- don't ship-then-optimize.

## Async patterns (tokio)

- **`mpsc::try_send` for fire-and-forget back-pressure.** When
  the producer should NEVER block the hot path, use `try_send`
  + log-and-drop on `Full`. abixio's WAL writes use this:
  fire-and-forget channel send, ack after append not after
  send. Channel `try_send` returns `TrySendError` immediately.
- **Pipeline large writes via `mpsc(N)` + spawned writer task.**
  abixio's `encode_and_write` does this for >= 1MB PUTs:
  `mpsc(8)` between encode task and writer task. Matches
  rustfs's mpsc pattern + minio's ring-buffer-per-writer
  design. 1GB PUT went 317 → 449 MB/s.
- **Choose between buffering and streaming based on size.**
  abixio's `WalShardWriter` is dual-mode: <= 64KB buffered in
  RAM, >= 1MB promoted to streaming so disk write overlaps
  network receive.
- **Ack-after-work, not ack-after-channel.** If a worker
  consumes from a channel, the producer's "success" signal
  should fire after the worker has done the durable work --
  not after `send` returned. Channels are conduits, not
  promises.
- `tokio::sync::broadcast` for one-to-many fan-out; `watch` for
  current-value-only single-slot pub/sub.
- Don't sprinkle `.await` across hot paths that don't need it.
  Async tax is real; sync code in an async runtime is fine.

## Unsafe doctrine

- **Every `unsafe { ... }` block carries a `// SAFETY: <reason>`
  comment.** Enforce via the workspace lint:
  ```toml
  [workspace.lints.clippy]
  undocumented_unsafe_blocks = "warn"
  ```
  Set to `warn` while legacy blocks get annotated; bump to
  `deny` once clean. Reviewers grep `SAFETY:` to audit invariants.
- **Premature `unsafe` is a code smell.** Prove the safe
  alternative is insufficient first.
- **`catch_unwind` at every Rust → foreign boundary.** Any
  callback the host calls from C / FFI / a vtable / a
  `ProcessEvent` trampoline must `catch_unwind` so a Rust
  panic doesn't unwind into a frame that doesn't know what
  unwinding is. ueforge does this in every PE trampoline.
- **`catch_unwind` per item when walking foreign data**, so
  one bad item doesn't kill the walk. ueforge's discovery
  iterator wraps each UObject visit this way.
- **On Windows: SEH-wrap calls into native code that may
  raise structured exceptions** (page faults from bad reads,
  etc.). `catch_unwind` does NOT catch SEH; use the
  `windows-sys` `SetUnhandledExceptionFilter` /
  `AddVectoredExceptionHandler` family, or a small C shim.
  ueforge's `AppendString` and FString walker do this.
- **Bounds-check before deref** when reading foreign indices.
  Don't trust an FName index, an FProperty offset, a
  GObjectsView slot count, etc. Bound the read, then deref.
- **Cap recursive walks + detect cycles.** ueforge caps
  is_a super-chain at 64; FString walk caps total bytes +
  detects cycles. Fix what looked like an infinite loop ONCE,
  in the walker, not in every caller.
- **`VirtualQuery` guard pointer reads on Windows** when
  walking foreign data structures that may have stale or
  freed pointers (FFieldClass, Children, etc.).

## Project structure

### Workspace layout

For multiple crates:

```toml
# Cargo.toml at repo root
[workspace]
resolver = "3"
members = ["crate-a", "crate-b", "framework"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.95"
license = "MIT"
publish = false

[workspace.dependencies]
# Pinned shared deps -- crates opt in with `serde.workspace = true`.
serde = { version = "1", features = ["derive"] }
serde_json = "1"
parking_lot = "0.12"
arc-swap = "1"
anyhow = "1"
# etc.

[workspace.lints.clippy]
undocumented_unsafe_blocks = "warn"
# project-wide lints land here; crates opt in with `[lints] workspace = true`
```

Per crate:

```toml
[package]
name = "my-crate"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
serde_json.workspace = true
local-thing = { path = "../local-thing" }

[lints]
workspace = true
```

Pin shared deps in `[workspace.dependencies]`. Project-wide
lints in `[workspace.lints]`. Per-tool config goes in
`[package.metadata.<tool>]` -- the cargo-tool ecosystem reads
that section without polluting the public manifest.

### `rust-toolchain.toml` + `.cargo/config.toml`

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
components = ["clippy", "rustfmt"]
targets = ["x86_64-pc-windows-msvc"]

# .cargo/config.toml
[build]
target = "x86_64-pc-windows-msvc"

[alias]
deploy = "run --release --bin <deploy-cli> --"

[target.'cfg(not(target_arch = "wasm32"))']
rustflags = ["-C", "target-cpu=native"]
```

Pinning toolchain via `rust-toolchain.toml` makes builds
reproducible; cargo auto-installs the target. Aliases let you
expose project-specific subcommands (`cargo deploy`).

### `[[bin]]` targets vs separate crates

When a project needs a helper CLI built on top of its library
(deploy tool, fixture generator, fuzzer), prefer **`[[bin]]`
targets inside the same crate** over a sibling crate:

```toml
[[bin]]
name = "deploy"
path = "src/bin/deploy.rs"
```

ueforge collapsed `ueforge-deploy` (separate crate) into a
`[[bin]]` after this lesson. Wins: one fewer crate, the bin
shares lints / dev-deps / `[lints]` config, faster compile
because the library is built once. Use a sibling crate only
when the bin has fundamentally different dep tree (e.g. needs
heavy GUI deps the library shouldn't carry).

### Multiple cdylibs in one workspace: per-crate `target_dir`

Two cdylibs both compile to `main.dll` -- they collide on
`target/release/main.dll`. Fix:

```toml
[package.metadata.ueforge]
target_dir = "target/my-mod"
```

Then your deploy tool (or `cargo build -p <mod> --target-dir
target/<mod> --release`) honors that path.

## Build profiles

### Release: optimize for runtime

```toml
[profile.release]
lto = "fat"
codegen-units = 1
panic = "abort"
opt-level = 3
strip = "symbols"
```

- `lto = "fat"` -- whole-program optimization across the crate
  graph. Big win for inlining + dead-code elim.
- `codegen-units = 1` -- one LLVM unit so the inliner can
  reason across the whole crate. Supersedes `incremental=false`
  for runtime perf (different angles on the same thing).
- `panic = "abort"` -- no unwinding tables, smaller hot paths,
  better icache.
- `opt-level = 3` -- full auto-vectorization, inlining, loop
  unrolling. Don't drop to `s` / `z` unless binary size is the
  product goal.
- `strip = "symbols"` -- smaller binary, better icache.

### Dev: optimize for debuggability

```toml
[profile.dev]
panic = "unwind"
```

Keep dev unwinding so a stray panic in a worker leaves a
backtrace instead of aborting the whole binary. Release stays
on `abort` for size/perf. This split caught real bugs in
ueforge that `abort` would have masked.

### Bench: matches release knobs

```toml
[profile.bench]
opt-level = 3
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = false
```

Criterion picks this up automatically.

### WASM: runtime > size (usually)

```toml
[profile.release]
opt-level = 3     # NOT "s" / "z" if runtime perf matters
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
debug = false

[package.metadata.wasm-pack.profile.release]
wasm-opt = false  # if rustc emits features the bundled wasm-opt can't validate
```

Hush traded ~700 KB at `opt-level=z` for ~1.0-1.5 MB at
`opt-level=3` to win in-browser hot-path speed. Bundle loads
once per tab session; hot paths run thousands of times. Make
this trade explicitly, with evidence.

## Error handling

- Binary crates: `anyhow::Result` for app-level errors.
- Library crates: custom error enum with `thiserror` at the
  public boundary; `anyhow` is fine internally.
- `?` propagation over explicit `match` unless you need to
  transform the error.
- **Never `.unwrap()` in production paths.** Use `.expect("<why
  this can't fail>")` if truly impossible (and the message is
  documentation). Better: restructure to `let-else` or `?`.
  abixio's "unwrap plague" close (commit `29ec3df`) found 530
  of 533 unwraps were test-only -- the one production unwrap
  became `let-else`.
- Prefer **`let-else`** over `match` / `if let` for the
  "extract or bail" pattern:
  ```rust
  let Some(x) = thing.get() else {
      return Err(MyError::Missing);
  };
  ```
- `io::Result<T>` (alias for `Result<T, std::io::Error>`) at
  any I/O boundary, not `Result<T, String>`. Real error types
  preserve the kind and chain.

## Code organization patterns

### Static registries of Drop-having types (the two-static pattern)

When you want `static REGISTRY: Registry = Registry::new(&[Def::new(...)])`
and `Def` contains state with non-trivial `Drop` (mutex,
atomic-with-cache, owned heap), const-eval rejects the
temporary array literal: `E0493: destructor of [Def; N] cannot
be evaluated at compile-time`.

**Don't reach for the hoisted-array workaround**
(`static DEFS: [Def; N] = [...]; static REGISTRY = Registry::new(&DEFS);`)
-- that's clunky.

**Do** declare each Def as its own named `static` and store
`&[&'static Def]` in the registry. References are `Copy` +
`Drop`-free, so the slice literal const-evaluates fine:

```rust
static MATERIALS_DEF: StackDef = StackDef::new("materials", ...);
static CRAFTING_DEF:  StackDef = StackDef::new("crafting",  ...);

pub static STACKS: StackRegistry =
    StackRegistry::new(&[&MATERIALS_DEF, &CRAFTING_DEF]);
```

Bonus: each Def is a named symbol -- better for debug,
introspection, and direct ref access from other modules.

For Drop-free Defs (`&'static str`s, primitives, fn pointers),
`&[Def]` (slice of values) works without the indirection -- pick
based on whether the Def has Drop-having state.

### Def → Registry → Instance → Controller (k8s-style)

For any subsystem with "schema + storage + runtime instance +
behavior" shape (skills, hooks, ops, building types, debug
selectors, shutdown handlers, etc.), layer it as four roles:

- **Def** (CRD): the schema. Static, immutable, no runtime state.
  Named `<Subject>Def`. **Always Def-suffixed -- no exceptions.**
- **Registry** (etcd): the collection. `<Subject>Registry`
  holding `entries: &'static [<Subject>Def]` (or `&[&'static
  <Subject>Def]` for Drop-having Defs). Lookup is
  `registry.def(key) -> Option<&'static <Subject>Def>`.
- **Instance** (CR): one runtime object derived from a Def.
- **Controller**: function/system that reads Def + writes
  Instance. Re-derives at every reconcile -- never caches Def
  fields in Instance.

Naming contract is strict. See ueforge's `architecture.md` for
the full compliance scorecard. Adopt the pattern wherever a
subsystem has more than one "kind" of a thing.

### Closure-populated runtime registries

Some registries can't be `static` const literals because their
entries are runtime closures (PE hooks holding handler
closures; debug ops with `Box<dyn Fn(...)>` handlers). Shape:

- One singleton: `pub static FOO_REGISTRY: FooRegistry = FooRegistry::new();`
- Init function: `pub fn register_builtins() { FOO_REGISTRY.register(...); }`
- Called once at worker init.

Same surface as compile-time registries (`def(key)`,
`shutdown_all`, etc.); just imperative population. ueforge's
`HOOK_REGISTRY`, `OP_REGISTRY`, `SELECTOR_REGISTRY`,
`SHUTDOWN_REGISTRY` all follow this shape.

## Testing

- Unit tests in `#[cfg(test)] mod tests` at the bottom of each
  file. Test names describe the scenario:
  `fn empty_input_returns_none()`.
- Integration tests in `tests/`. Each `.rs` is a separate test
  binary -- structure for parallel compilation.
- `assert_eq!` over `assert!(a == b)` for better error messages.
- **`--test-threads=1` when tests share global state** (a
  process-singleton resource, a running HTTP server, a static
  Mutex). Document the requirement in the test file's header
  comment.
- **Skip cleanly when external prereqs are missing.** A test
  that needs a running game / server / database should early-
  return with `eprintln!("skipping: $VAR unset")` when the env
  isn't there, so `cargo test` stays green for CI / smoke
  checks. grounded2-rpg's `BBP_DEBUG_PORT` pattern.
- **Scenario DSL for HTTP-driven integration tests.** When you
  have an integration test surface (an HTTP debug endpoint, a
  CLI), build a thin Pester-style DSL that expresses
  "do this, expect this state change" as a chain. ueforge's
  `client::scenario` -- migration from imperative tests cut
  ~340 LoC and added 9 assertions.
- **Diff helpers for snapshot comparison.** When tests assert
  "X changed and nothing else did", a `client::diff` helper
  beats per-field asserts. ueforge ships this; abixio's
  benchmark baseline-compare uses it too.
- **Criterion benches in `benches/`** with `harness = false`:
  ```toml
  [[bench]]
  name = "compute_suggestions"
  harness = false
  ```
  Criterion's HTML reports + outlier filtering are worth it.
  Compare to a saved baseline JSON across changes.

## Cargo hygiene

- `cargo clippy -- -D warnings` in CI. Fix; don't suppress.
  Hush + chromium-extensions land this gate. Annotation
  comments (`#[allow(...)]`) need a reason or a follow-up
  ticket.
- `cargo fmt`. No custom `rustfmt.toml` unless necessary.
- `Cargo.lock` committed for binaries; not for libraries.
- Features: **additive only.** No feature that REMOVES
  functionality (that's a different crate / shape). Default
  feature set is what `cargo build` users get; document the
  trade.
- `--release` for benchmarks and perf testing; debug for
  iteration.

## Crate preferences (updated)

| Need | Preferred crate |
|---|---|
| Serialization | `serde` + `serde_json`; `simd-json` (~30% faster) only when a hot path benchmarks as the bottleneck |
| CLI | `clap` with derive |
| HTTP client (async) | `reqwest` |
| HTTP client (blocking) | `ureq` |
| HTTP server (low-ceremony) | `tiny_http` |
| HTTP server (production) | `hyper` + tower / `s3s` for S3-flavored protocol |
| Async runtime | `tokio` (multi-thread) -- only if actually needed |
| Logging / tracing | `tracing` over `log` |
| Mutex | `parking_lot::Mutex` |
| Read-mostly state | `arc_swap::ArcSwap` |
| Concurrent map | `dashmap::DashMap` |
| Small-string-key hashing | `foldhash` |
| Bounded byte buffers | `bytes::Bytes` / `BytesMut` |
| Raft consensus | `openraft` |
| Memory mapping | `memmap2` |
| WASM bindings | `wasm-bindgen` + `web-sys` + `js-sys` + `serde-wasm-bindgen` |
| WASM UI | `leptos` (csr feature for browser) |
| Criterion bench | `criterion` with `harness = false` |
| `#[non_exhaustive]` consideration | Only on types crossing API boundaries. NOT on types every consumer constructs as a struct literal in the same workspace (you update atomically). |
| Once-init | `std::sync::OnceLock` (stdlib); skip `lazy_static` / `once_cell` |
| Error types | `thiserror` (library boundary) + `anyhow` (binary) |

## Windows specifics

- `std::path::PathBuf`, not string concat.
- `std::fs::canonicalize` returns `\\?\` prefix -- handle or
  avoid in user-facing paths.
- `.lines()` handles `\n` and `\r\n` uniformly.
- Strip UTF-8 BOM before parsing if you accept user-edited
  files.
- Loopback TCP connect is ~0.2 ms on Windows vs ~0.03 ms on
  Linux -- factor into benchmark methodology.
- Use `127.0.0.1`, never `localhost` (Windows DNS adds ~200 ms).
- `TCP_NODELAY` must be set explicitly.
- hyper needs `writev(true)` + `max_buf_size(4 MB)` for
  optimal throughput.

## TLS gotcha

When your dep tree pulls both `aws-lc-rs` and `ring`, rustls
0.23 refuses to auto-pick. Top of `main()`:

```rust
tokio_rustls::rustls::crypto::ring::default_provider()
    .install_default()
    .expect("install rustls ring crypto provider");
```

abixio's pattern. Same fix in any rustls 0.23+ binary with both
providers in the tree.

## Patterns to avoid

- `Rc` / `Arc` as first resort -- restructure ownership first.
- Channels when a `Mutex` is simpler.
- Premature `unsafe` -- prove the safe alternative insufficient.
- `.clone()` to silence borrow errors -- scope the borrow instead.
- `.unwrap()` in production paths.
- Adding error handling for scenarios that can't happen
  (CLAUDE.md rule). Validate at system boundaries; trust
  internal code + framework guarantees.
- `#[non_exhaustive]` on workspace-internal types every
  consumer constructs as a struct literal -- you update
  atomically.
- Single-letter / cryptic variable names (one-letter loop
  indices are OK).
- Comments that explain WHAT (the well-named identifier already
  does). Comments only for WHY -- hidden invariants,
  workarounds, surprising behavior.

## Hardening doctrine (from kovarex reviews)

These landed across multiple projects. Apply by default:

- `parking_lot::Mutex` everywhere.
- Dev `panic=unwind`, release `panic=abort`.
- Hot paths use `try_*` + soft fallback; no allocs on miss.
- Persisted state carries a `schema_version` field for
  forward-compat; mutation only via the tracker / store API,
  not by external mutation.
- Atomic file saves: temp + fsync + rename. Validate the
  destination path before writing.
- Worker threads return a handle with stop flag + panic
  counter + last_panic + named thread. No orphaned threads.
  ueforge's `PollerHandle` pattern.
- Bound recursive walks; detect cycles.
- Dispatch handlers **outside** the registry mutex so a panic
  / SEH inside a handler doesn't poison the lock.
- Every `unsafe { ... }` carries a `// SAFETY:` line.
- Every Rust → foreign boundary `catch_unwind`s.

## When in doubt

- Read the `code` skill for universal cross-language rules
  (terse functions, no over-engineering, no comments-as-what).
- Read the `try-harder` skill for response calibration.
- Concrete examples of every pattern above live in
  `abix-/Grounded2Mods` (ueforge crate), `abix-/abixio`, or
  `abix-/chromium-extensions` (hush crate). Grep for the
  pattern name.
