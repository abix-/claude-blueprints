<#
.SYNOPSIS
    One-time initialization for the sanitizer system.

.DESCRIPTION
    Run this once before first use. Creates:
    - sanitizer.json: Template for mappings and config (if not exists)
    - unsanitized directory for real values

.EXAMPLE
    .\Initialize.ps1
#>

[CmdletBinding()]
param(
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "==============================" -ForegroundColor Cyan
Write-Host "  Sanitizer Initialization" -ForegroundColor Cyan
Write-Host "==============================" -ForegroundColor Cyan
Write-Host ""

# Create directories
$dirs = @(
    $SanitizerDir,
    "$env:USERPROFILE\.claude\unsanitized"
)

foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -Path $dir -ItemType Directory -Force | Out-Null
        Write-Host "Created: $dir" -ForegroundColor Green
    }
}

# Create template sanitizer.json if not exists
$secretsPath = "$SanitizerDir\sanitizer.json"
if (-not (Test-Path $secretsPath)) {
    @{
        mappings = @{
            "example-real-server.internal.corp" = "fake-server.example.test"
        }
        autoMappings = @{}
        patterns = @{
            ipv4 = $true
            hostnames = @("\.internal\.corp$", "\.local$")
        }
        unsanitizedPath = "~/.claude/unsanitized/{project}"
    } | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
    Write-Host "Created: sanitizer.json (edit this with your mappings)" -ForegroundColor Green
}

# Validate settings.json has permissions.deny
$settingsPath = "$env:USERPROFILE\.claude\settings.json"
$requiredDenyPaths = @(
    "~/.claude/sanitizer/sanitizer.json",
    "~/.claude/unsanitized/**"
)

if (Test-Path $settingsPath) {
    try {
        $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json
        $existingDeny = @()
        if ($settings.permissions -and $settings.permissions.deny) {
            $existingDeny = @($settings.permissions.deny)
        }

        $missingPaths = $requiredDenyPaths | Where-Object { $_ -notin $existingDeny }

        if ($missingPaths.Count -gt 0) {
            Write-Host ""
            Write-Host "WARNING: settings.json missing permissions.deny entries:" -ForegroundColor Yellow
            foreach ($path in $missingPaths) {
                Write-Host "  - $path" -ForegroundColor Yellow
            }
            Write-Host ""
            Write-Host "Add this to settings.json:" -ForegroundColor Cyan
            Write-Host '  "permissions": {' -ForegroundColor White
            Write-Host '    "deny": [' -ForegroundColor White
            Write-Host '      "~/.claude/sanitizer/sanitizer.json",' -ForegroundColor White
            Write-Host '      "~/.claude/unsanitized/**"' -ForegroundColor White
            Write-Host '    ]' -ForegroundColor White
            Write-Host '  },' -ForegroundColor White
        } else {
            Write-Host "Validated: settings.json permissions.deny" -ForegroundColor Green
        }
    }
    catch {
        Write-Host "WARNING: Could not parse settings.json" -ForegroundColor Yellow
    }
} else {
    Write-Host "WARNING: settings.json not found - copy from claude-blueprints repo" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "Done! Next steps:" -ForegroundColor Cyan
Write-Host "  1. Edit sanitizer.json with your real->fake mappings" -ForegroundColor White
Write-Host "  2. Copy settings.json from repo (has hooks + permissions.deny)" -ForegroundColor White
Write-Host "  3. Restart Claude Code" -ForegroundColor White
Write-Host ""
