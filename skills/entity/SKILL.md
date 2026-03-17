---
description: Inspect a Bevy entity via endless-cli debug
disable-model-invocation: true
allowed-tools: Bash
version: "1.1"
---
Call `endless-cli debug` with the entity argument (e.g. `489v9`).

```bash
endless-cli debug $ARGUMENTS
```

Print the formatted result. If the entity is an NPC, highlight key fields: job, activity, hp, energy, faction, home, combat_state, flags. If it's a building, highlight: kind, town, hp, occupants, growth.
