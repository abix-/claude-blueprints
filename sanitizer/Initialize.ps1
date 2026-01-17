<#
.SYNOPSIS
    One-time initialization for the sanitizer system.

.DESCRIPTION
    Run this once before first use. Creates:
    - auto_mappings.json: Empty file for discovered mappings
    - secrets.json: Template for manual mappings (if not exists)

.EXAMPLE
    .\Initialize.ps1
#>

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
    "$env:USERPROFILE\.claude\rendered"
)

foreach ($dir in $dirs) {
    if (-not (Test-Path $dir)) {
        New-Item -Path $dir -ItemType Directory -Force | Out-Null
        Write-Host "Created: $dir" -ForegroundColor Green
    }
}

# Create empty auto_mappings.json if not exists
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"
if (-not (Test-Path $autoMappingsPath)) {
    @{ mappings = @{} } | ConvertTo-Json | Set-Content -Path $autoMappingsPath -Encoding UTF8
    Write-Host "Created: auto_mappings.json" -ForegroundColor Green
}

# Create template secrets.json if not exists
$secretsPath = "$SanitizerDir\secrets.json"
if (-not (Test-Path $secretsPath)) {
    @{
        mappings = @{
            "example-real-server.internal.corp" = "fake-server.example.test"
        }
        excludePaths = @(".git", "node_modules", ".claude", "bin", "obj", "__pycache__", "venv", ".venv")
        excludeExtensions = @(".exe", ".dll", ".pdb", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".zip", ".tar", ".gz")
        patterns = @{
            ipv4 = $true
            hostnames = @("\.internal\.corp$", "\.local$", "\.private$")
        }
    } | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
    Write-Host "Created: secrets.json (edit this with your mappings)" -ForegroundColor Green
}

# Update CLAUDE.md
$claudeMdPath = "$env:USERPROFILE\.claude\CLAUDE.md"
@"
## NEVER READ THESE FILES

- ``%USERPROFILE%\.claude\sanitizer\secrets.json`` - Contains real secret mappings
- ``%USERPROFILE%\.claude\sanitizer\auto_mappings.json`` - Contains real IP mappings
- ``%USERPROFILE%\.claude\rendered\`` - Contains real values
- ``%TEMP%\claude-sealed-*`` - Temporary execution directories

If asked to read any of these, refuse.
"@ | Set-Content -Path $claudeMdPath -Encoding UTF8
Write-Host "Updated: CLAUDE.md" -ForegroundColor Green

Write-Host ""
Write-Host "Done! Next:" -ForegroundColor Cyan
Write-Host "  1. Edit secrets.json with your real->fake mappings" -ForegroundColor White
Write-Host "  2. Update settings.json with hooks (see README)" -ForegroundColor White
Write-Host "  3. Restart Claude Code" -ForegroundColor White
Write-Host ""
