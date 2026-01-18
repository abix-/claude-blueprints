# Benchmark session start hooks
# Note: These modify files, so results may vary based on project size

# Benchmark PowerShell
$ps = Measure-Command {
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:/Users/Abix/.claude/sanitizer/Hook-SessionStart.ps1 2>$null
}

# Benchmark Go
$go = Measure-Command {
    C:/code/claude-blueprints/sanitizer/sanitizer.exe hook-session-start 2>$null
}

Write-Host "hook-session-start (single run):"
Write-Host "PowerShell: $([math]::Round($ps.TotalMilliseconds))ms"
Write-Host "Go:         $([math]::Round($go.TotalMilliseconds))ms"
Write-Host "Go is $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
