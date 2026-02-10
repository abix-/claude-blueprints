---
description: Build and run Endless (pure Bevy)
allowed-tools: Bash
---

Stop any running instance, build, and run in one command:

```bash
taskkill //F //IM endless.exe 2>/dev/null; cd /c/code/endless/rust && cargo build --release 2>&1 && cargo run --release 2>&1
```

Report build errors if any. Confirm game window opened.
