# Sanitizer benchmarks (Go binary)

$sanitizer = "C:/code/claude-blueprints/sanitizer/sanitizer.exe"

# Cold start - basic sanitize
$data = "Server 111.148.25.233 connected to 111.83.13.238"
$cold = Measure-Command {
    1..5 | ForEach-Object { $data | & $sanitizer sanitize }
}
Write-Host "cold start:         $([math]::Round($cold.TotalMilliseconds/5))ms/call (5 runs)"

# hook-bash (powershell command)
$bashInput = '{"hook_event_name": "PreToolUse", "tool_input": { "command": "powershell Get-Process" }}'
$bash = Measure-Command {
    1..5 | ForEach-Object { $bashInput | & $sanitizer hook-bash 2>$null }
}
Write-Host "hook-bash:          $([math]::Round($bash.TotalMilliseconds/5))ms/call (5 runs)"

# hook-bash (no-op, regular command)
$bashNoop = '{"hook_event_name": "PreToolUse", "tool_input": { "command": "ls -la" }}'
$noop = Measure-Command {
    1..5 | ForEach-Object { $bashNoop | & $sanitizer hook-bash 2>$null }
}
Write-Host "hook-bash (no-op):  $([math]::Round($noop.TotalMilliseconds/5))ms/call (5 runs)"

# hook-file-access
$fileInput = '{"hook_event_name": "PreToolUse", "tool_input": { "file_path": "C:/code/project/src/main.go" }}'
$file = Measure-Command {
    1..5 | ForEach-Object { $fileInput | & $sanitizer hook-file-access 2>$null }
}
Write-Host "hook-file-access:   $([math]::Round($file.TotalMilliseconds/5))ms/call (5 runs)"

# hook-session-start (single run - modifies files)
$session = Measure-Command {
    & $sanitizer hook-session-start 2>$null
}
Write-Host "hook-session-start: $([math]::Round($session.TotalMilliseconds))ms (1 run)"
