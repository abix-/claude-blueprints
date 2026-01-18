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
2. Copy all:
   - `cp -r C:/code/claude-blueprints/skills/* ~/.claude/skills/`
   - `cp -r C:/code/claude-blueprints/hooks/* ~/.claude/hooks/`
   - `cp -r C:/code/claude-blueprints/commands/* ~/.claude/commands/`
   - `cp -r C:/code/claude-blueprints/sanitizer/* ~/.claude/sanitizer/`
   - `cp C:/code/claude-blueprints/CLAUDE.md ~/.claude/`
   - `cp C:/code/claude-blueprints/settings.json ~/.claude/`

Report what was updated.
