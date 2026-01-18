<#
.SYNOPSIS
    PreToolUse hook for Bash - routes commands (DENY/FAKE/REAL).

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
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

# Parse hook input
$inputText = @($input) -join ""
if ([string]::IsNullOrEmpty($inputText)) { exit 0 }

try { $hookData = $inputText | ConvertFrom-Json -ErrorAction Stop }
catch { exit 0 }

if (-not $hookData -or $hookData.hook_event_name -ne "PreToolUse") { exit 0 }

$command = $hookData.tool_input.command
if (-not $command) { exit 0 }

# === DENY ===
$blockedPatterns = @('[\\/]sanitizer\.json(?![.\w])', '\.claude[\\/]unsanitized')
foreach ($pattern in $blockedPatterns) {
    if ($command -match $pattern) {
        @{ hookSpecificOutput = @{ hookEventName = "PreToolUse"; permissionDecision = "deny"; reason = "Blocked" } } | ConvertTo-Json -Depth 5 -Compress
        exit 0
    }
}

# === REAL ===
$realPatterns = @('^\s*powershell', '^\s*pwsh', '^\s*\.\\.*\.ps1', '^\s*&\s')
$isReal = $false
foreach ($pattern in $realPatterns) { if ($command -match $pattern) { $isReal = $true; break } }

# === FAKE (default) ===
if (-not $isReal) { exit 0 }

# === Execute in unsanitized ===
$projectPath = (Get-Location).Path
$projectName = Split-Path $projectPath -Leaf
$unsanitizedPath = ($config.unsanitizedPath -replace '\{project\}', $projectName) -replace '^~', $env:USERPROFILE

if (-not (Test-Path $unsanitizedPath)) { New-Item -Path $unsanitizedPath -ItemType Directory -Force | Out-Null }

# Sync to unsanitized
foreach ($file in Get-ChildItem -Path $projectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($projectPath.Length).TrimStart('\', '/')
    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $unsanitizedPath $relativePath
    $destDir = Split-Path $destPath -Parent
    if (-not (Test-Path $destDir)) { New-Item -Path $destDir -ItemType Directory -Force | Out-Null }

    if (Test-BinaryFile -Path $file.FullName) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
    }
    else {
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            [System.IO.File]::WriteAllText($destPath, (ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings))
        }
        catch { Copy-Item -Path $file.FullName -Destination $destPath -Force }
    }
}

$escapedCommand = $command -replace '"', '\"'
$wrappedCommand = @"
powershell.exe -ExecutionPolicy Bypass -NoProfile -Command "
    Import-Module '$sanitizerDir\Sanitizer.psm1' -Force
    `$m = Get-SanitizerMappings -SecretsPath '$($paths.Secrets)'
    Set-Location '$unsanitizedPath'
    `$o = cmd /c `"$escapedCommand`" 2>&1 | Out-String
    ConvertTo-SanitizedText -Text `$o -Mappings `$m
"
"@

@{ hookSpecificOutput = @{ hookEventName = "PreToolUse"; permissionDecision = "allow"; updatedInput = @{ command = $wrappedCommand } } } | ConvertTo-Json -Depth 5 -Compress
