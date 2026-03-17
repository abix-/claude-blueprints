---
description: Build and run Endless (pure Bevy)
disable-model-invocation: true
allowed-tools: Bash
version: "1.0"
---
Stop any running instance, build, and run in one command.

IMPORTANT: NEVER use `run_in_background`. Always run in the foreground.

```bash
taskkill //F //IM endless.exe 2>/dev/null; cd /c/code/endless/rust && claude-k3 cargo-lock build --release 2>&1 && cargo run --release 2>&1
```

Report build errors if any. Confirm game window opened.
