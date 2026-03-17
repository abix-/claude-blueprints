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
cd /c/code/endless/rust && claude-k3 cargo-lock build --release 2>&1
```
If build fails, stop and report errors.

2. **Launch game with --autostart** (background + PID):
```bash
cd /c/code/endless/rust && target/release/endless.exe --autostart &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

3. **Wait for BRP to come up** (poll until responsive, ~15s timeout):
```bash
for i in $(seq 1 30); do
  if endless-cli perf 2>/dev/null | grep -q fps; then
    echo "BRP ready after ${i}s"
    break
  fi
  sleep 0.5
done
```

4. **Run baseline tests via endless-cli**:
```bash
endless-cli perf
endless-cli summary
```

Then run **feature-specific tests** based on what was just built. Examples:
- New building type: `endless-cli build town:0 kind:Farm col:172 row:125` then `endless-cli summary`
- NPC behavior change: `endless-cli debug <uid>` to inspect NPC state
- Combat change: wait, then `endless-cli perf` for faction stats
- UI change: check game launches without crash, FPS is stable

Available endless-cli commands:
- `summary` -- full game state (towns, NPCs, buildings, squads)
- `perf` -- FPS, UPS, entity counts, system timings
- `build town:N kind:X col:N row:N` -- place buildings
- `destroy town:N col:N row:N` -- remove buildings
- `upgrade town:N upgrade_idx:N` -- apply upgrades
- `time paused:false time_scale:4.0` -- control time
- `debug <uid>` -- inspect entity by UID
- `debug kind:squad index:N` -- inspect squad/town/policy
- `squad_target squad:N x:N y:N` -- move squad
- `policy town:N eat_food:true` -- set town policies
- `chat town:N to:N message:text` -- send chat
- `ai_manager town:N active:true` -- configure AI
- `test` -- built-in baseline test suite
- `loop` -- background state poller (10s)

5. **Do NOT kill the game process.** Leave it running so the user can interact with it. Only kill if the user explicitly asks.

6. **Write findings** to `docs/findings-$(date +%Y-%m-%d-%H%M).md` with:
- Build status (pass/fail)
- Game launch status (did BRP come up?)
- Baseline perf (FPS, UPS, NPC count)
- Feature-specific test results (what was tested, pass/fail, actual vs expected)
- Any warnings (FPS < 60, errors in responses, unexpected state)
