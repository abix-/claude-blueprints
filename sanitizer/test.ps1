$ErrorActionPreference = 'Continue'
$sanitizer = "$PSScriptRoot/sanitizer.exe"

function Test-Command {
    param([string]$Name, [scriptblock]$Test)
    Write-Host "`n=== $Name ===" -ForegroundColor Cyan
    try {
        & $Test
        Write-Host "PASS" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "FAIL: $_" -ForegroundColor Red
        return $false
    }
}

$passed = 0
$failed = 0

# Build first
Write-Host "Building sanitizer.exe..." -ForegroundColor Yellow
Push-Location $PSScriptRoot
go build -o sanitizer.exe ./cmd/sanitizer
if ($LASTEXITCODE -ne 0) { throw "Build failed" }
Pop-Location
Write-Host "Build successful`n" -ForegroundColor Green

# sanitize-ips tests
if (Test-Command "sanitize-ips: Basic IP replacement" {
    $input = "Server at 111.91.241.85 and 111.229.14.114"
    $result = $input | & $sanitizer sanitize-ips
    if ($result -match "192\.168\." -or $result -match "10\.0\.0\.") {
        throw "IPs not sanitized: $result"
    }
    if ($result -notmatch "111\.\d+\.\d+\.\d+.*111\.\d+\.\d+\.\d+") {
        throw "Expected fake IPs (111.x.x.x), got: $result"
    }
    Write-Host "Input:  $input"
    Write-Host "Output: $result"
}) { $passed++ } else { $failed++ }

if (Test-Command "sanitize-ips: Deterministic (same input = same output)" {
    $r1 = "111.91.241.85" | & $sanitizer sanitize-ips
    $r2 = "111.91.241.85" | & $sanitizer sanitize-ips
    if ($r1 -ne $r2) { throw "Not deterministic: '$r1' vs '$r2'" }
    if ($r1 -match "192\.168\.") { throw "IP not sanitized: $r1" }
    Write-Host "Both runs: $r1"
}) { $passed++ } else { $failed++ }

if (Test-Command "sanitize-ips: Excluded IPs unchanged" {
    $result = "localhost 127.0.0.1 and broadcast 255.255.255.255" | & $sanitizer sanitize-ips
    if ($result -notmatch "127\.0\.0\.1" -or $result -notmatch "255\.255\.255\.255") {
        throw "Excluded IPs were sanitized: $result"
    }
    Write-Host "Output: $result"
}) { $passed++ } else { $failed++ }

# hook-bash BLOCK tests
if (Test-Command "hook-bash: BLOCK sanitizer.json access" {
    $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"cat ~/.claude/sanitizer/sanitizer.json"}}'
    $result = $input | & $sanitizer hook-bash
    $json = $result | ConvertFrom-Json
    if ($json.hookSpecificOutput.permissionDecision -ne "deny") { throw "Expected deny, got: $result" }
    Write-Host "Output: blocked"
}) { $passed++ } else { $failed++ }

if (Test-Command "hook-bash: BLOCK unsanitized directory access" {
    $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"ls ~/.claude/unsanitized/"}}'
    $result = $input | & $sanitizer hook-bash
    $json = $result | ConvertFrom-Json
    if ($json.hookSpecificOutput.permissionDecision -ne "deny") { throw "Expected deny, got: $result" }
    Write-Host "Output: blocked"
}) { $passed++ } else { $failed++ }

# hook-bash SANITIZED tests
if (Test-Command "hook-bash: SANITIZED (ls command)" {
    $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"ls -la"}}'
    $result = $input | & $sanitizer hook-bash
    if ($result -and $result.Trim()) {
        throw "Expected empty output for SANITIZED, got: $result"
    }
    Write-Host "Output: (empty = allow as-is)"
}) { $passed++ } else { $failed++ }

if (Test-Command "hook-bash: SANITIZED (git command)" {
    $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"git status"}}'
    $result = $input | & $sanitizer hook-bash
    if ($result -and $result.Trim()) {
        throw "Expected empty output for SANITIZED, got: $result"
    }
    Write-Host "Output: (empty = allow as-is)"
}) { $passed++ } else { $failed++ }

# hook-bash UNSANITIZED tests
if (Test-Command "hook-bash: UNSANITIZED (powershell command)" {
    $input = '{"hook_event_name":"PreToolUse","tool_input":{"command":"powershell.exe -Command Get-Date"}}'
    $result = $input | & $sanitizer hook-bash
    $json = $result | ConvertFrom-Json
    if (-not $json.hookSpecificOutput.updatedInput.command) { throw "Expected modified command, got: $result" }
    Write-Host "Modified command present: yes"
}) { $passed++ } else { $failed++ }

# hook-file-access tests
if (Test-Command "hook-file-access: BLOCK sanitizer.json" {
    $input = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"C:/Users/Abix/.claude/sanitizer/sanitizer.json"}}'
    $result = $input | & $sanitizer hook-file-access
    $json = $result | ConvertFrom-Json
    if ($json.hookSpecificOutput.permissionDecision -ne "deny") { throw "Expected deny, got: $result" }
    Write-Host "Output: blocked"
}) { $passed++ } else { $failed++ }

if (Test-Command "hook-file-access: BLOCK unsanitized path" {
    $input = '{"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"C:/Users/Abix/.claude/unsanitized/project/file.txt"}}'
    $result = $input | & $sanitizer hook-file-access
    $json = $result | ConvertFrom-Json
    if ($json.hookSpecificOutput.permissionDecision -ne "deny") { throw "Expected deny, got: $result" }
    Write-Host "Output: blocked"
}) { $passed++ } else { $failed++ }

if (Test-Command "hook-file-access: ALLOW normal file" {
    $input = '{"hook_event_name":"PreToolUse","tool_name":"Read","tool_input":{"file_path":"C:/code/project/main.go"}}'
    $result = $input | & $sanitizer hook-file-access
    if ($result -and $result.Trim()) {
        throw "Expected empty (allow), got: $result"
    }
    Write-Host "Output: (empty = allow)"
}) { $passed++ } else { $failed++ }

# hook-session-start test
if (Test-Command "hook-session-start: Sanitize test project" {
    $testDir = "$env:TEMP/sanitizer-test-project"
    if (Test-Path $testDir) { Remove-Item $testDir -Recurse -Force }
    New-Item -ItemType Directory -Path $testDir -Force | Out-Null

    $testContent = @"
# Config file
server = 111.67.177.64
backup = 111.229.14.114
gateway = 111.189.164.227
"@
    Set-Content "$testDir/config.txt" $testContent

    Push-Location $testDir
    try {
        $input = '{"hook_event_name":"SessionStart"}'
        $null = $input | & $sanitizer hook-session-start 2>&1

        $sanitized = Get-Content "$testDir/config.txt" -Raw
        if ($sanitized -match "192\.168\.50\.100") {
            throw "IP not sanitized: $sanitized"
        }
        if ($sanitized -notmatch "111\.\d+\.\d+\.\d+") {
            throw "No fake IPs found: $sanitized"
        }
        Write-Host "Original IPs: 111.67.177.64, 111.229.14.114, 111.189.164.227"
        Write-Host "Sanitized content:"
        Write-Host $sanitized
    } finally {
        Pop-Location
    }

    Remove-Item $testDir -Recurse -Force
}) { $passed++ } else { $failed++ }

# hook-session-stop test
if (Test-Command "hook-session-stop: Sync to unsanitized" {
    $testDir = "$env:TEMP/sanitizer-test-project2"
    $unsanitizedDir = "$env:USERPROFILE/.claude/unsanitized/sanitizer-test-project2"

    if (Test-Path $testDir) { Remove-Item $testDir -Recurse -Force }
    if (Test-Path $unsanitizedDir) { Remove-Item $unsanitizedDir -Recurse -Force }

    # Start with real IP
    New-Item -ItemType Directory -Path $testDir -Force | Out-Null
    Set-Content "$testDir/test.txt" "server = 111.8.230.60"

    Push-Location $testDir
    try {
        # Session start sanitizes real -> fake
        $null = '{"hook_event_name":"SessionStart"}' | & $sanitizer hook-session-start 2>&1

        $sanitized = Get-Content "$testDir/test.txt"
        Write-Host "After session-start: $sanitized"
        if ($sanitized -notmatch "111\.\d+\.\d+\.\d+") {
            throw "IP not sanitized after session-start: $sanitized"
        }

        # Session stop should create unsanitized copy with real IP
        $null = '{"hook_event_name":"Stop"}' | & $sanitizer hook-session-stop 2>&1

        if (-not (Test-Path "$unsanitizedDir/test.txt")) {
            throw "Unsanitized file not created at $unsanitizedDir/test.txt"
        }
        $unsanitized = Get-Content "$unsanitizedDir/test.txt"
        if ($unsanitized -notmatch "192\.168\.1\.50") {
            throw "Real IP not restored: $unsanitized"
        }
        Write-Host "After session-stop (unsanitized): $unsanitized"
    } finally {
        Pop-Location
    }

    Remove-Item $testDir -Recurse -Force
    Remove-Item $unsanitizedDir -Recurse -Force
}) { $passed++ } else { $failed++ }

# Summary
Write-Host "`n========================================" -ForegroundColor White
Write-Host "RESULTS: $passed passed, $failed failed" -ForegroundColor $(if ($failed -eq 0) { "Green" } else { "Red" })
Write-Host "========================================" -ForegroundColor White

if ($failed -gt 0) { exit 1 }
