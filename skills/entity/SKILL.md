---
description: Inspect a Bevy entity via endless-cli. See endless-cli skill for full command reference.
disable-model-invocation: true
allowed-tools: Bash
version: "1.2"
---
```bash
endless-cli get_entity $ARGUMENTS
```

Print the formatted result. If the entity is an NPC, highlight key fields: job, activity, hp, energy, faction, home, combat_state, flags. If it's a building, highlight: kind, town, hp, occupants, growth.
