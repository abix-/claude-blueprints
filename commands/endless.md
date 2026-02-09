---
description: Build and run Endless (pure Bevy)
allowed-tools: Bash
---

Build and run the Endless project (pure Bevy, no Godot):

1. Build Rust code (release for performance):
```bash
cd /c/code/endless/rust && cargo build --release 2>&1
```

2. Run the game:
```bash
cd /c/code/endless/rust && cargo run --release 2>&1
```

Report build errors if any. Confirm game window opened.

For debug builds (faster compile, slower runtime):
```bash
cd /c/code/endless/rust && cargo run 2>&1
```
