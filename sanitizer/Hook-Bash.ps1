<#
.SYNOPSIS
    PreToolUse hook for Bash - routes commands to appropriate execution path.

.DESCRIPTION
    Intercepts Bash commands and routes them:
    - DENY: Block access to sanitizer config and unsanitized directory
    - FAKE: Run directly in working tree (git, ls, cat, etc.)
    - REAL: Sync to unsanitized directory, run there, sanitize output

.EXAMPLE
    # Called automatically by Claude Code via PreToolUse hook
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$sanitizerDir = "$env:USERPROFILE\.claude\sanitizer"

Import-Module "$sanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $sanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$forwardMappings = Get-SanitizerMappings -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

# Parse hook input from stdin
$inputText = @($input) -join ""

if ([string]::IsNullOrEmpty($inputText)) {
    exit 0
}

try {
    $hookData = $inputText | ConvertFrom-Json -ErrorAction Stop
}
catch {
    exit 0
}

if (-not $hookData) { exit 0 }

$hookEvent = $hookData.hook_event_name
$command = $hookData.tool_input.command

if ($hookEvent -ne "PreToolUse") { exit 0 }
if (-not $command) { exit 0 }

# === DENY - Block dangerous commands ===

$blockedPatterns = @(
    '[\\/]sanitizer\.json(?![.\w])'
    '\.claude[\\/]unsanitized'
)

foreach ($pattern in $blockedPatterns) {
    if ($command -match $pattern) {
        @{
            hookSpecificOutput = @{
                hookEventName = "PreToolUse"
                permissionDecision = "deny"
                reason = "Access to sanitizer files is blocked"
            }
        } | ConvertTo-Json -Depth 5 -Compress
        exit 0
    }
}

# === FAKE - Passthrough commands (run in working tree with fake values) ===

$passthroughPatterns = @(
    '^\s*git\s'
    '^\s*gh\s'
    '^\s*cd\s'
    '^\s*ls\b'
    '^\s*dir\b'
    '^\s*pwd\b'
    '^\s*echo\s'
    '^\s*mkdir\s'
    '^\s*rm\s'
    '^\s*cp\s'
    '^\s*mv\s'
    '^\s*cat\s'
    '^\s*head\s'
    '^\s*tail\s'
    '^\s*wc\s'
    '^\s*find\s'
    '^\s*grep\s'
    '^\s*which\s'
    '^\s*where\s'
    '^\s*test\s'
    '^\s*\[\s'
)

foreach ($pattern in $passthroughPatterns) {
    if ($command -match $pattern) {
        exit 0
    }
}

# === REAL - Run in unsanitized directory ===

$projectPath = (Get-Location).Path
$projectName = Split-Path $projectPath -Leaf
$unsanitizedPath = $config.unsanitizedPath -replace '\{project\}', $projectName
$unsanitizedPath = $unsanitizedPath -replace '^~', $env:USERPROFILE

# Ensure unsanitized directory exists
if (-not (Test-Path $unsanitizedPath)) {
    New-Item -Path $unsanitizedPath -ItemType Directory -Force | Out-Null
}

# Sync working tree to unsanitized directory
foreach ($file in Get-ChildItem -Path $projectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($projectPath.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $unsanitizedPath $relativePath
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) {
        New-Item -Path $destDir -ItemType Directory -Force | Out-Null
    }

    if (Test-BinaryFile -Path $file.FullName) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
    }
    else {
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            $content = ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings
            [System.IO.File]::WriteAllText($destPath, $content)
        }
        catch {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
        }
    }
}

# Build wrapped command that runs in unsanitized dir and sanitizes output
$escapedCommand = $command -replace '"', '\"'
$escapedUnsanitizedPath = $unsanitizedPath -replace '\\', '\\'

$wrappedCommand = @"
powershell.exe -ExecutionPolicy Bypass -NoProfile -Command "
    Import-Module '$sanitizerDir\Sanitizer.psm1' -Force
    `$mappings = Get-SanitizerMappings -SecretsPath '$($paths.Secrets)'

    Set-Location '$unsanitizedPath'
    `$output = cmd /c `"$escapedCommand`" 2>&1 | Out-String

    `$output = ConvertTo-SanitizedText -Text `$output -Mappings `$mappings
    `$output
"
"@

@{
    hookSpecificOutput = @{
        hookEventName = "PreToolUse"
        permissionDecision = "allow"
        updatedInput = @{
            command = $wrappedCommand
        }
    }
} | ConvertTo-Json -Depth 5 -Compress
