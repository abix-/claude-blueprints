<#
.SYNOPSIS
    One-time initialization for the sanitizer system.

.EXAMPLE
    .\Initialize.ps1
#>

[CmdletBinding()]
param(
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

$ErrorActionPreference = "Stop"

# Create directories
@($SanitizerDir, "$env:USERPROFILE\.claude\unsanitized") | ForEach-Object {
    if (-not (Test-Path $_)) { New-Item -Path $_ -ItemType Directory -Force | Out-Null }
}

# Create sanitizer.json with defaults
$secretsPath = "$SanitizerDir\sanitizer.json"
$defaults = @{
    mappings = @{}
    autoMappings = @{}
    patterns = @{ ipv4 = $true; hostnames = @() }
    unsanitizedPath = "~/.claude/unsanitized/{project}"
    excludePaths = @(".git", "node_modules", ".venv", "__pycache__")
}

if (Test-Path $secretsPath) {
    Write-Warning "sanitizer.json already exists at: $secretsPath"
    $response = Read-Host "Overwrite with defaults? (y/N)"
    if ($response -eq 'y') {
        $defaults | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
        Write-Host "Overwritten with defaults"
    }
    else {
        Write-Host "Skipped - keeping existing file"
    }
}
else {
    $defaults | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
}

# Validate settings.json
$settingsPath = "$env:USERPROFILE\.claude\settings.json"
$requiredDeny = @("~/.claude/sanitizer/sanitizer.json", "~/.claude/unsanitized/**")

if (Test-Path $settingsPath) {
    try {
        $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json
        $existingDeny = if ($settings.permissions.deny) { @($settings.permissions.deny) } else { @() }
        $missing = $requiredDeny | Where-Object { $_ -notin $existingDeny }
        if ($missing) {
            Write-Warning "settings.json missing permissions.deny: $($missing -join ', ')"
        }
    }
    catch { Write-Warning "Could not parse settings.json" }
}
else {
    Write-Warning "settings.json not found"
}
