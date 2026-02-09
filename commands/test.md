---
description: Clean build and run Endless (pure Bevy) - removes caches, rebuilds, runs
allowed-tools: Bash
---

Clean build and run the Endless project:

1. Clean old build artifacts:
```bash
rm -rf /c/code/endless/rust/target/debug/incremental
rm -rf /c/code/endless/rust/target/release/incremental
```

2. Build Rust code (debug for faster compile):
```bash
cd /c/code/endless/rust && cargo build 2>&1
```

3. Run the game (5 second timeout for quick test):
```bash
cd /c/code/endless/rust && timeout 5 cargo run 2>&1 || true
```

Report any build errors. Check console output for:
- "Endless ECS initialized - systems registered"
- "GPU compute initialized"
- "Sprite sheets loaded"
- "Tick: N NPCs active"

For full rebuild from scratch:
```bash
cd /c/code/endless/rust && cargo clean && cargo build 2>&1
```
