---
description: Pull claude-blueprints repo and apply to ~/.claude
disable-model-invocation: true
allowed-tools: Bash(git:*), Bash(sync:*), Bash(go:*)
version: "2.0"
---
## Task

1. Pull latest: `git -C "C:/code/claude-blueprints" pull`

2. Sync repo to local:

```bash
repo="C:/code/claude-blueprints"
local="$HOME/.claude"

# Skills: directory format (skills/<name>/SKILL.md)
mkdir -p "$local/skills"
# Remove skills in local that don't exist in repo
for d in "$local/skills"/*/; do
    [ -d "$d" ] && [ ! -d "$repo/skills/$(basename "$d")" ] && rm -rf "$d"
done
# Also remove any leftover flat skill files
for f in "$local/skills"/*.md; do
    [ -f "$f" ] && rm -f "$f"
done
cp -r "$repo/skills"/* "$local/skills"/ 2>/dev/null

# Hooks: flat files
mkdir -p "$local/hooks"
if [ -d "$repo/hooks" ]; then
    for f in "$local/hooks"/*; do
        [ -f "$f" ] && [ ! -f "$repo/hooks/$(basename "$f")" ] && rm -f "$f"
    done
    cp "$repo/hooks"/* "$local/hooks"/ 2>/dev/null
fi

# Scripts: flat files
mkdir -p "$local/scripts"
if [ -d "$repo/scripts" ]; then
    for f in "$local/scripts"/*; do
        [ -f "$f" ] && [ ! -f "$repo/scripts/$(basename "$f")" ] && rm -f "$f"
    done
    cp "$repo/scripts"/* "$local/scripts"/ 2>/dev/null
fi

# Root files
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
