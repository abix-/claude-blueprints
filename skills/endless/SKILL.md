---
description: Build and run Endless (pure Bevy)
disable-model-invocation: true
allowed-tools: Bash
version: "1.1"
---
`cargo-lock run` stops any running instance, builds, and runs -- all in one command. NEVER run a separate build or stop step.

Use the manifest path matching the current working directory (e.g. `/c/code/claude-2/rust/Cargo.toml` if in claude-2).

IMPORTANT: NEVER use `run_in_background`. Always run in the foreground.

```bash
k3sc cargo-lock run --release --manifest-path /c/code/claude-2/rust/Cargo.toml 2>&1
```

Report build errors if any. Confirm game window opened.
