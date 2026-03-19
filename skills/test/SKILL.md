---
description: Build, launch with --autostart, verify via endless-cli, write findings doc
disable-model-invocation: true
allowed-tools: Bash, Read, Write, Grep, Glob
version: "1.1"
---
Build and verify the Endless game through endless-cli (BRP wrapper, in PATH).

## Steps

1. **Build release**:
```bash
k3sc cargo-lock build --release --manifest-path /c/code/endless/rust/Cargo.toml 2>&1
```
If build fails, stop and report errors.

2. **Launch game with --autostart** (background + PID):
```bash
/c/code/endless/rust/target/release/endless.exe --autostart &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

3. **Wait for BRP to come up** (poll until responsive, ~15s timeout):
```bash
for i in $(seq 1 30); do
  if endless-cli get_perf 2>/dev/null | grep -q fps; then
    echo "BRP ready after ${i}s"
    break
  fi
  sleep 0.5
done
```

4. **Run baseline tests via endless-cli**:
```bash
endless-cli get_perf
endless-cli get_summary
```

Then run **feature-specific tests** based on what was just built. Examples:
- New building type: `endless-cli create_building town:0 kind:Farm col:172 row:125` then `endless-cli get_summary`
- NPC behavior change: `endless-cli get_entity <uid>` to inspect NPC state
- Combat change: wait, then `endless-cli get_perf` for faction stats
- UI change: check game launches without crash, FPS is stable

See `endless-cli` skill for full command reference.

5. **Do NOT kill the game process.** Leave it running so the user can interact with it. Only kill if the user explicitly asks.

6. **Write findings** to `docs/findings-$(date +%Y-%m-%d-%H%M).md` with:
- Build status (pass/fail)
- Game launch status (did BRP come up?)
- Baseline perf (FPS, UPS, NPC count)
- Feature-specific test results (what was tested, pass/fail, actual vs expected)
- Any warnings (FPS < 60, errors in responses, unexpected state)
