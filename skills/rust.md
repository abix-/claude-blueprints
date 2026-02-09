---
name: rust
description: Rust development standards and patterns. Use when writing Rust outside of Bevy/Endless.
metadata:
  version: "1.0"
  updated: "2026-02-09"
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

## Patterns
- Prefer iterators over index loops
- `collect()` with turbofish when type isn't inferred: `.collect::<Vec<_>>()`
- Scope borrows in blocks to satisfy borrow checker — don't fight it with clones
- `impl Trait` in args for simple cases, generics when you need the type param
- Derive order: `Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize`

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
