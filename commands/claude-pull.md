---
description: Pull claude-blueprints repo and apply to ~/.claude
allowed-tools: Bash(git:*), Bash(sync:*), Bash(go:*)
---

## Task

1. Pull latest: `git -C "C:/code/claude-blueprints" pull`

2. Sync repo to local (run as single command):

```bash
repo="C:/code/claude-blueprints"; local="$HOME/.claude"; for dir in skills hooks commands sanitizer; do mkdir -p "$local/$dir"; if [ -d "$repo/$dir" ]; then for f in "$local/$dir"/*; do [ -f "$f" ] && [ ! -f "$repo/$dir/$(basename "$f")" ] && rm -f "$f"; done; cp "$repo/$dir"/* "$local/$dir"/ 2>/dev/null; fi; done; cp "$repo/CLAUDE.md" "$local/" 2>/dev/null; cp "$repo/settings.json" "$local/" 2>/dev/null; echo "Sync complete"
```

3. Build Go sanitizer (if Go installed):

```bash
if [ -f "/c/Program Files/Go/bin/go.exe" ]; then cd "C:/code/claude-blueprints/sanitizer-go" && "/c/Program Files/Go/bin/go.exe" build -o sanitizer.exe ./cmd/sanitizer && mkdir -p "$HOME/.claude/bin" && cp sanitizer.exe "$HOME/.claude/bin/" && echo "Go sanitizer built"; else echo "Go not installed - skipping"; fi
```

Report what git pulled and confirm sync completed.
