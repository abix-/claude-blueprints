---
description: Check Rust dependencies for updates and apply them
disable-model-invocation: true
allowed-tools: Bash, Read, Edit, WebFetch
version: "1.0"
---
Check all Endless Rust dependencies for available updates:

1. Run cargo-outdated on direct dependencies only:
```bash
cargo outdated --root-deps-only --manifest-path /c/code/endless/rust/Cargo.toml 2>&1
```

2. Present results as a table with columns: Crate, Current, Compatible, Latest, Action Needed.

3. Categorize updates:
   - **Safe** (patch/minor within semver): can be pulled with `cargo update`
   - **Breaking** (major version bump): needs Cargo.toml edit and possible code changes
   - **Blocked**: dependency pinned by another dep (e.g., wgpu pinned by bevy)

4. For breaking updates, check the crate's changelog or release notes on crates.io/GitHub to summarize what changed.

5. Ask the user which updates to apply before making changes.

6. For safe updates: run `cargo update -p <crate>` one at a time.
7. For breaking updates: edit Cargo.toml version, run `k3sc cargo-lock check --manifest-path /c/code/endless/rust/Cargo.toml`, fix any compile errors.
8. After all updates: run `k3sc cargo-lock build --release --manifest-path /c/code/endless/rust/Cargo.toml` to verify everything compiles.
