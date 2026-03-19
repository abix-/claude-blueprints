---
name: endless-cli
description: endless-cli BRP client for the Endless game. Use when interacting with a running game instance, inspecting entities, placing buildings, checking perf, or running BRP tests.
version: "1.0"
---

# endless-cli

Go binary wrapping Bevy Remote Protocol (JSON-RPC on `localhost:15702`).
Binary: `C:\code\endless\llm-player\endless-cli.exe` (in PATH).
Source: `C:\code\endless\llm-player\main.go`.

Always use endless-cli instead of raw curl for BRP queries.

## BRP method names

The CLI prepends `endless/` to the command name automatically. The registered Rust BRP methods use verb prefixes (`get_`, `set_`, `create_`, etc.).

**IMPORTANT**: The CLI shorthand names in the help text do NOT match the registered BRP methods. Use the full method names below. The CLI accepts any method name -- just type it after `endless-cli`.

### Read

```bash
endless-cli get_summary                              # full game state
endless-cli get_perf                                 # FPS, UPS, entity counts, system timings
endless-cli debug <uid>                              # inspect entity by UID (hardcoded to get_entity)
endless-cli get_squad index:0                        # inspect squad
endless-cli list_buildings town:0                    # list buildings with entity IDs
endless-cli list_npcs town:0 job:Woodcutter          # list NPCs, filter by town/job
```

### Create / Delete

```bash
endless-cli create_building town:1 kind:Farm row:-5 col:0    # place building
endless-cli delete_building town:1 row:-5 col:0              # remove building
```

### Update

```bash
endless-cli set_time paused:false time_scale:4.0             # control time
endless-cli set_policy town:1 eat_food:true                  # set town policies
endless-cli set_ai_manager town:1 active:true                # configure AI manager
endless-cli set_squad_target squad:13 x:6944 y:11488         # move squad
```

### Actions

```bash
endless-cli apply_upgrade town:1 upgrade_idx:0               # apply upgrade
endless-cli send_chat town:1 to:0 message:hi friend          # send chat (spaces ok)
endless-cli recruit_squad town:1                             # recruit squad
endless-cli dismiss_squad squad:0                            # dismiss squad
```

### Tools (built into CLI, not BRP methods)

```bash
endless-cli test                                     # baseline BRP test (get_perf + get_summary)
endless-cli loop                                     # background state poller (10s)
endless-cli launch                                   # start LLM player Claude session
```

## Parameter syntax

All params use `key:value` format. Values are auto-typed (bool, int, float, string). Spaces in values work by appending to the previous key: `message:hi friend` becomes `{"message": "hi friend"}`.

Raw JSON also works: `endless-cli get_entity '{"entity":"489v9"}'`

## Usage patterns

**Check if game is running:**
```bash
endless-cli get_perf 2>/dev/null && echo "up" || echo "down"
```

**Wait for BRP after launch (~15s):**
```bash
for i in $(seq 1 30); do
  endless-cli get_perf 2>/dev/null | grep -q fps && break
  sleep 0.5
done
```

## Known issues

- **CLI shorthand broken**: The help text shows `summary`, `perf`, `build`, etc. but the Rust BRP registers `get_summary`, `get_perf`, `create_building`, etc. Use the full names.
- k3s pods have no GPU/display -- endless-cli won't work there.
- Connection refused = game not running.
- Silent placement failure: `create_building` may silently fail on occupied/water/OOB cells.
