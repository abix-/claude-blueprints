---
description: Sync ~/.claude to claude-blueprints repo and push
allowed-tools: Bash(cp:*), Bash(git:*)
---

## Context

- Repo status: !`git -C "C:/code/claude-blueprints" status --short`

## Task

Sync local Claude config to repo and push:

1. Copy all:
   - `cp -r ~/.claude/skills/* C:/code/claude-blueprints/skills/`
   - `cp -r ~/.claude/hooks/* C:/code/claude-blueprints/hooks/`
   - `cp -r ~/.claude/commands/* C:/code/claude-blueprints/commands/`
   - `cp -r ~/.claude/sanitizer/* C:/code/claude-blueprints/sanitizer/`
   - `cp ~/.claude/CLAUDE.md C:/code/claude-blueprints/`
   - `cp ~/.claude/settings.json C:/code/claude-blueprints/`
2. Stage all: `git -C "C:/code/claude-blueprints" add -A`
3. Show diff: `git -C "C:/code/claude-blueprints" diff --cached --stat`
4. If changes exist, commit with message describing what changed (lowercase, concise)
5. Push to origin

Report what was synced.
