---
description: Sync ~/.claude to claude-blueprints repo and push
allowed-tools: Bash(cp:*), Bash(git:*)
---

## Context

- Repo status: !`git -C "C:/code/claude-blueprints" status --short`

## Task

Sync local Claude config to repo and push:

1. Copy skills: `cp -r ~/.claude/skills/* C:/code/claude-blueprints/skills/`
2. Copy hooks: `cp -r ~/.claude/hooks/* C:/code/claude-blueprints/hooks/`
3. Stage all: `git -C "C:/code/claude-blueprints" add -A`
4. Show diff: `git -C "C:/code/claude-blueprints" diff --cached --stat`
5. If changes exist, commit with message describing what changed (lowercase, concise)
6. Push to origin

Report what was synced.
