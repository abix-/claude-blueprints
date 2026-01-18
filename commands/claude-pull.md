---
description: Pull claude-blueprints repo and apply to ~/.claude
allowed-tools: Bash(git:*), Bash(sync:*), Bash(go:*)
---

## Task

1. Pull latest: `git -C "C:/code/claude-blueprints" pull`

2. Sync repo to local:

```bash
repo="C:/code/claude-blueprints"
local="$HOME/.claude"

# Sync directories (remove orphans, copy files)
for dir in skills hooks commands; do
    mkdir -p "$local/$dir"
    if [ -d "$repo/$dir" ]; then
        # Remove files in local that don't exist in repo
        for f in "$local/$dir"/*; do
            [ -f "$f" ] && [ ! -f "$repo/$dir/$(basename "$f")" ] && rm -f "$f"
        done
        cp "$repo/$dir"/* "$local/$dir"/ 2>/dev/null
    fi
done

# Sync root files
cp "$repo/CLAUDE.md" "$local/" 2>/dev/null
cp "$repo/settings.json" "$local/" 2>/dev/null
```

3. Build Go sanitizer (if Go installed):

```bash
if [ -f "/c/Program Files/Go/bin/go.exe" ]; then
    cd "C:/code/claude-blueprints/sanitizer"
    "/c/Program Files/Go/bin/go.exe" build -o sanitizer.exe ./cmd/sanitizer
    mkdir -p "$HOME/.claude/sanitizer"
    cp sanitizer.exe "$HOME/.claude/sanitizer/"
    echo "Go sanitizer built"
else
    echo "Go not installed - skipping"
fi
```

Report what git pulled and confirm sync completed.
