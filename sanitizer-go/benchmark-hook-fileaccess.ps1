$testInput = '{ "hook_event_name": "PreToolUse", "tool_input": { "file_path": "C:/code/project/src/main.go" } }'

# Benchmark PowerShell cold start
$ps = Measure-Command {
    1..5 | ForEach-Object {
        $testInput | powershell.exe -NoProfile -ExecutionPolicy Bypass -File C:/Users/Abix/.claude/sanitizer/Hook-FileAccess.ps1
    }
}

# Benchmark Go cold start
$go = Measure-Command {
    1..5 | ForEach-Object {
        $testInput | C:/code/claude-blueprints/sanitizer-go/sanitizer.exe hook-file-access
    }
}

Write-Host "PowerShell: $([math]::Round($ps.TotalMilliseconds))ms (5 runs) = $([math]::Round($ps.TotalMilliseconds/5))ms/call"
Write-Host "Go:         $([math]::Round($go.TotalMilliseconds))ms (5 runs) = $([math]::Round($go.TotalMilliseconds/5))ms/call"
Write-Host "Go is $([math]::Round($ps.TotalMilliseconds / $go.TotalMilliseconds, 1))x faster"
