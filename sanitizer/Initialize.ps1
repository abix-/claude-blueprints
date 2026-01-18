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

# Create template sanitizer.json if not exists
$secretsPath = "$SanitizerDir\sanitizer.json"
if (-not (Test-Path $secretsPath)) {
    @{
        mappings = @{ "example-real-server.internal.corp" = "fake-server.example.test" }
        autoMappings = @{}
        patterns = @{ ipv4 = $true; hostnames = @("\.internal\.corp$", "\.local$") }
        unsanitizedPath = "~/.claude/unsanitized/{project}"
    } | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
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
