---
description: Build, launch with --autostart, verify via BRP, write findings doc
allowed-tools: Bash, Read, Write, Grep, Glob
---

Build and verify the Endless game through BRP (Bevy Remote Protocol).

## Steps

1. **Build release**:
```bash
cd /c/code/endless/rust && cargo build --release 2>&1
```
If build fails, stop and report errors.

2. **Launch game with --autostart** (foreground won't work for BRP polling, so background + PID):
```bash
cd /c/code/endless/rust && target/release/endless.exe --autostart &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

3. **Wait for BRP to come up** (poll until responsive, ~15s timeout):
```bash
for i in $(seq 1 30); do
  if curl -s -X POST http://localhost:15702 -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"endless/perf","id":1}' 2>/dev/null | grep -q fps; then
    echo "BRP ready after ${i}s"
    break
  fi
  sleep 0.5
done
```

4. **Run tests via BRP**. Always run these baseline checks:
```bash
# Perf check
curl -s -X POST http://localhost:15702 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"endless/perf","id":1}'

# Game state check
curl -s -X POST http://localhost:15702 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"endless/summary","id":2}'
```

Then run **feature-specific tests** based on what was just built. Examples:
- New building type: `endless/build` to place it, `endless/summary` to confirm
- NPC behavior change: `endless/debug` to inspect NPC state
- Combat change: wait, then check `FactionStats` via `endless/perf`
- UI change: check game launches without crash, FPS is stable

Adapt your test sequence to whatever feature you just implemented. Available BRP endpoints:
- `endless/summary` — full game state (towns, NPCs, buildings, squads)
- `endless/perf` — FPS, UPS, entity counts, system timings
- `endless/build` — place buildings: `{"town":0,"kind":"FarmerHome","col":172,"row":125}`
- `endless/destroy` — remove buildings: `{"town":0,"col":172,"row":125}`
- `endless/upgrade` — apply upgrades: `{"town":0,"upgrade":0}`
- `endless/time` — control time: `{"paused":false,"time_scale":4.0}`
- `endless/debug` — inspect entities/resources: `{"uid":450}` (auto-detect npc/building), `{"kind":"squad","index":14}`, `{"kind":"town","index":1}`, `{"kind":"policy","index":1}`
- `endless/squad_target` — move squads
- `endless/policy` — set town policies
- `endless/chat` — send chat messages between towns

5. **Do NOT kill the game process.** Leave it running so the user can interact with it. Only kill if the user explicitly asks.

6. **Write findings** to `docs/findings-$(date +%Y-%m-%d-%H%M).md` with:
- Build status (pass/fail)
- Game launch status (did BRP come up?)
- Baseline perf (FPS, UPS, NPC count)
- Feature-specific test results (what was tested, pass/fail, actual vs expected)
- Any warnings (FPS < 60, errors in responses, unexpected state)
