---
description: Sync ~/.claude to claude-blueprints repo and push
allowed-tools: Bash(sync:*), Bash(git:*)
---

## Context

- Repo status: !`git -C "C:/code/claude-blueprints" status --short`

## Task

1. Sync local to repo:

```bash
local="$HOME/.claude"
repo="C:/code/claude-blueprints"

# Sync directories (remove orphans, copy files)
for dir in skills hooks commands; do
    mkdir -p "$repo/$dir"
    if [ -d "$local/$dir" ]; then
        # Remove files in repo that don't exist in local
        for f in "$repo/$dir"/*; do
            [ -f "$f" ] && [ ! -f "$local/$dir/$(basename "$f")" ] && rm -f "$f"
        done
        cp "$local/$dir"/* "$repo/$dir"/ 2>/dev/null
    fi
done

# Sync root files
cp "$local/CLAUDE.md" "$repo/" 2>/dev/null
cp "$local/settings.json" "$repo/" 2>/dev/null
```

2. Stage and diff: `git -C "C:/code/claude-blueprints" add -A && git -C "C:/code/claude-blueprints" diff --cached --stat`
3. If changes, commit (lowercase, concise) and push

Report what was synced.
