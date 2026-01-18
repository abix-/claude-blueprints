---
description: Pull claude-blueprints repo and apply to ~/.claude
allowed-tools: Bash(powershell:*), Bash(git:*), Bash(go:*)
---

## Task

1. Pull and sync config:

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

2. Build Go sanitizer:

```powershell
$goExe = 'C:/Program Files/Go/bin/go.exe'
if (Test-Path $goExe) {
    Push-Location "C:/code/claude-blueprints/sanitizer-go"
    & $goExe build -o sanitizer.exe ./cmd/sanitizer
    if (-not (Test-Path "$env:USERPROFILE/.claude/bin")) { mkdir "$env:USERPROFILE/.claude/bin" -Force | Out-Null }
    Copy-Item sanitizer.exe "$env:USERPROFILE/.claude/bin/" -Force
    Pop-Location
    Write-Host "Go sanitizer built and installed"
} else {
    Write-Host "Go not installed - skipping sanitizer build"
}
```

Report what git pulled and confirm sync completed.
