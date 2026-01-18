# Test 1: DENY - access to sanitizer.json
Write-Host "Test 1: DENY case"
'{"hook_event_name": "PreToolUse", "tool_input": { "command": "cat ~/.claude/sanitizer/sanitizer.json" }}' | C:/code/claude-blueprints/sanitizer/sanitizer.exe hook-bash

# Test 2: FAKE - regular command (should return nothing = allow as-is)
Write-Host "`nTest 2: FAKE case (ls)"
'{"hook_event_name": "PreToolUse", "tool_input": { "command": "ls -la" }}' | C:/code/claude-blueprints/sanitizer/sanitizer.exe hook-bash
Write-Host "(empty = allow as-is)"

# Test 3: REAL - powershell command (should return wrapped command)
Write-Host "`nTest 3: REAL case (powershell)"
$result = '{"hook_event_name": "PreToolUse", "tool_input": { "command": "powershell Get-Process" }}' | C:/code/claude-blueprints/sanitizer/sanitizer.exe hook-bash
$result | ConvertFrom-Json | ConvertTo-Json -Depth 5
