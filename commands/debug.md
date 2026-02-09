---
description: Check Rust compiler errors and runtime logs
allowed-tools: Bash, Read, Edit, Grep, Write
---

Debug the Endless Bevy project:

1. Check for compiler errors:
```bash
cd /c/code/endless/rust && cargo check 2>&1
```

2. If build succeeds, run and capture output:
```bash
cd /c/code/endless/rust && timeout 3 cargo run 2>&1 || true
```

3. Look for:
- Rust compiler errors (fix in source)
- Runtime panics (check stack trace)
- Bevy ERROR/WARN logs (investigate cause)
- Missing assets (check file paths)

Common fixes:
- "Path not found" for assets: Check AssetPlugin file_path config
- Panic in system: Check query filters and component access
- wgpu errors: Check GPU buffer sizes and shader bindings

Summarize what was found and fixed.
