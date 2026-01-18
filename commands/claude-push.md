---
description: Sync ~/.claude to claude-blueprints repo and push
allowed-tools: Bash(powershell:*), Bash(git:*)
---

## Context

- Repo status: !`git -C "C:/code/claude-blueprints" status --short`

## Task

1. Run this PowerShell to sync local to repo:

```powershell
$local = "$env:USERPROFILE/.claude"; $repo = 'C:/code/claude-blueprints'
foreach ($dir in 'skills','hooks','commands','sanitizer') {
    $s, $d = "$local/$dir", "$repo/$dir"
    if (-not (Test-Path $d)) { mkdir $d -Force | Out-Null }
    if (Test-Path $s) {
        $srcNames = (Get-ChildItem $s -File -ErrorAction SilentlyContinue).Name
        Get-ChildItem $d -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -notin $srcNames } | Remove-Item -Force
        Copy-Item "$s/*" $d -Force -ErrorAction SilentlyContinue
    }
}
'CLAUDE.md','settings.json' | ForEach-Object { Copy-Item "$local/$_" $repo -Force -ErrorAction SilentlyContinue }
```

2. Stage: `git -C "C:/code/claude-blueprints" add -A`
3. Diff: `git -C "C:/code/claude-blueprints" diff --cached --stat`
4. If changes, commit (lowercase, concise) and push

Report what was synced.
