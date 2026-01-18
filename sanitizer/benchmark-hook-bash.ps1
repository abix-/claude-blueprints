$testInput = '{"hook_event_name": "PreToolUse", "tool_input": { "command": "powershell Get-Process" }}'

# Benchmark PowerShell cold start
$ps = Measure-Command {
    1..3 | ForEach-Object {
        $testInput | powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:/Users/Abix/.claude/sanitizer/Hook-Bash.ps1 2>$null
    }
}

# Benchmark Go cold start
$go = Measure-Command {
    1..3 | ForEach-Object {
        $testInput | C:/code/claude-blueprints/sanitizer/sanitizer.exe hook-bash 2>$null
    }
}

Write-Host "PowerShell: $([math]::Round($ps.TotalMilliseconds))ms (3 runs) = $([math]::Round($ps.TotalMilliseconds/3))ms/call"
Write-Host "Go:         $([math]::Round($go.TotalMilliseconds))ms (3 runs) = $([math]::Round($go.TotalMilliseconds/3))ms/call"
Write-Host "Go is $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
