---
name: rust
description: Rust development standards and patterns. Use when writing Rust outside of Bevy/Endless.
user-invocable: false
version: "1.1"
updated: "2026-05-10"
---
# Rust

## Project Layout
- Small projects: flat `src/` with `main.rs` + modules
- Larger: `src/lib.rs` + `src/main.rs`, or workspace with `crates/`
- Private internals in `src/internal/` or `pub(crate)`
- `Cargo.lock` committed for binaries, not libraries

## Error Handling
- Binary crates: `anyhow::Result` for app-level errors
- Library crates: custom error enum with `thiserror`
- `?` propagation over explicit match unless you need to transform
- Don't `.unwrap()` in production paths — `.expect("reason")` if truly impossible

## Performance principle: zero allocations on hot paths

The default in this codebase is **zero heap allocations unless
this specific call needs one**. Applies anywhere the code runs
at high frequency: per-frame callbacks, ProcessEvent
trampolines, render loops, tight iterator inner-loops.

Hot-path budget is single-digit nanoseconds and zero
allocator traffic per fire. To meet that:

- Do NOT allocate `String`, `Vec`, `Box`, `HashMap`, formatted
  strings, etc. on hot paths. Use `&str`, slices, stack
  buffers (`[u8; N]`), or pre-allocated reusable storage.
- Do NOT lock mutexes unless work is actually about to happen.
  Use lock-free shadows: an `AtomicUsize` mirroring queue
  length lets the hot path bail with one relaxed load when
  the queue is empty, without ever taking the mutex.
- Resolve external identifiers (UE object names, file paths,
  hash keys) ONCE, then cache the resolved pointer / index /
  handle in an `AtomicUsize` / `OnceLock`. Per-fire compare
  is a pointer/integer compare, not a string compare.
- Branch out of the hot path EARLY. The first lines of any
  per-frame callback should be a series of cheap atomic loads
  + branches that bail when there's nothing to do.

Cold paths (init, user clicks, error reporting) can allocate
freely. The discipline is about hot paths only.

When you write a new hot-path code path, apply this from day
one. Don't ship-then-optimize.

## Patterns
- Prefer iterators over index loops
- `collect()` with turbofish when type isn't inferred: `.collect::<Vec<_>>()`
- Scope borrows in blocks to satisfy borrow checker — don't fight it with clones
- `impl Trait` in args for simple cases, generics when you need the type param
- Derive order: `Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize`

### Static registries of Drop-having types
When you want a `static REGISTRY: Registry = Registry::new(&[Def::new(...)])` and `Def` contains state with non-trivial `Drop` (mutex, atomic-with-cache, owned heap), Rust's const-eval rejects the temporary array literal with `E0493: destructor of [Def; N] cannot be evaluated at compile-time`.

**Don't reach for the hoisted-array workaround** (`static DEFS: [Def; N] = [...]; static REGISTRY = Registry::new(&DEFS);`) — that's clunky.

**Do** declare each Def as its own named `static` and store `&[&'static Def]` in the registry. References are `Copy` + `Drop`-free, so the slice literal const-evaluates fine:

```rust
static MATERIALS_DEF: StackDef = StackDef::new("materials", ...);
static CRAFTING_DEF: StackDef = StackDef::new("crafting", ...);

pub static STACKS: StackRegistry =
    StackRegistry::new(&[&MATERIALS_DEF, &CRAFTING_DEF]);
```

Bonus: each Def is now a named symbol — better for debug, introspection, and direct ref access from other modules.

For Drop-free Defs (just `&'static str`s, primitives, fn pointers), `&[Def]` (slice of values) works without the indirection — pick based on whether the Def has Drop-having state.

## Cargo
- `cargo clippy` before committing — fix warnings, don't suppress
- `cargo fmt` — no custom rustfmt.toml unless necessary
- Features: additive only, no feature that removes functionality
- `--release` for benchmarks and perf testing, debug for development

## Testing
- Unit tests in `#[cfg(test)] mod tests` at bottom of file
- Integration tests in `tests/` directory
- Use `assert_eq!` over `assert!(a == b)` for better error messages
- Test names describe the scenario: `fn empty_input_returns_none()`

## Windows
- Use `std::path::PathBuf` not string concatenation for paths
- `std::fs::canonicalize` returns `\\?\` prefix on Windows — handle or avoid
- Line endings: read with `.lines()` which handles both `\n` and `\r\n`
- BOM: strip UTF-8 BOM before parsing (same as Golang rule)

## Crate Preferences
- Serialization: `serde` + `serde_json`
- CLI args: `clap` with derive
- HTTP: `reqwest` (async) or `ureq` (blocking)
- Async: `tokio` (multi-thread) — only add if actually needed
- Logging: `tracing` over `log`

## Avoid
- `Rc`/`Arc` as first resort — restructure ownership first
- Channels when a `Mutex` is simpler
- Premature `unsafe` — prove safe alternatives insufficient
- `.clone()` to silence borrow errors — scope borrows instead
