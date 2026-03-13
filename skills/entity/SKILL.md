---
description: Inspect a Bevy entity via endless/debug BRP endpoint
disable-model-invocation: true
allowed-tools: Bash
version: "1.0"
---
Call the `endless/debug` BRP endpoint with the entity argument (e.g. `489v9`).

```bash
curl -s -X POST http://localhost:15702 -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"endless/debug","params":{"entity":"$ARGUMENTS"},"id":1}' | python3 -m json.tool
```

Print the formatted result. If the entity is an NPC, highlight key fields: job, activity, hp, energy, faction, home, combat_state, flags. If it's a building, highlight: kind, town, hp, occupants, growth.
