---
description: Pull claude-blueprints repo and apply to ~/.claude
allowed-tools: Bash(powershell:*), Bash(git:*)
---

## Task

Run this PowerShell to pull and sync:

```powershell
git -C "C:/code/claude-blueprints" pull
$repo = 'C:/code/claude-blueprints'; $local = "$env:USERPROFILE/.claude"
foreach ($dir in 'skills','hooks','commands','sanitizer') {
    $s, $d = "$repo/$dir", "$local/$dir"
    if (-not (Test-Path $d)) { mkdir $d -Force | Out-Null }
    if (Test-Path $s) {
        $srcNames = (Get-ChildItem $s -File -ErrorAction SilentlyContinue).Name
        Get-ChildItem $d -File -ErrorAction SilentlyContinue | Where-Object { $_.Name -notin $srcNames } | Remove-Item -Force
        Copy-Item "$s/*" $d -Force -ErrorAction SilentlyContinue
    }
}
'CLAUDE.md','settings.json' | ForEach-Object { Copy-Item "$repo/$_" $local -Force -ErrorAction SilentlyContinue }
```

Report what git pulled and confirm sync completed.
