---
description: Pull claude-blueprints repo and apply to ~/.claude
allowed-tools: Bash(cp:*), Bash(git:*)
---

## Context

- Local skills: !`ls ~/.claude/skills/`
- Local hooks: !`ls ~/.claude/hooks/`

## Task

Pull repo changes and apply to local Claude config:

1. Pull latest: `git -C "C:/code/claude-blueprints" pull`
2. Copy skills: `cp -r C:/code/claude-blueprints/skills/* ~/.claude/skills/`
3. Copy hooks: `cp -r C:/code/claude-blueprints/hooks/* ~/.claude/hooks/`

Report what was updated.
