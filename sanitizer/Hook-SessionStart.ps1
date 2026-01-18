<#
.SYNOPSIS
    SessionStart hook - sanitizes project files.

.DESCRIPTION
    Replaces real values with fake values in the working tree:
    - Loads manual mappings from secrets.json
    - Auto-discovers IPs and hostnames matching patterns
    - Generates fake values and stores in secrets.json (autoMappings)

.EXAMPLE
    .\Hook-SessionStart.ps1

.EXAMPLE
    .\Hook-SessionStart.ps1 -ProjectPath C:\code\myproject -DryRun
#>

[CmdletBinding()]
param(
    [string]$ProjectPath = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$Quiet,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets

# Load existing auto mappings from config
$autoMappings = @{}
foreach ($key in $config.autoMappings.Keys) {
    $autoMappings[$key] = $config.autoMappings[$key]
}

if (-not $Quiet) {
    Write-Host "Sanitizing: $ProjectPath" -ForegroundColor Cyan
    Write-Host "  Manual mappings: $($config.mappings.Count)" -ForegroundColor Gray
    Write-Host "  Auto mappings: $($autoMappings.Count)" -ForegroundColor Gray
}

# === GATHER FILES ===

$files = foreach ($file in Get-ChildItem -Path $ProjectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($ProjectPath.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -eq 0 -or $file.Length -gt 10MB) { continue }
    if (Test-BinaryFile -Path $file.FullName) { continue }

    $file
}

# === DISCOVER VALUES ===

$discovered = @{}

foreach ($file in $files) {
    try {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        if ([string]::IsNullOrEmpty($content)) { continue }

        # Find IPs
        if ($config.patterns.ipv4) {
            foreach ($match in [regex]::Matches($content, $Ipv4Regex)) {
                $ip = $match.Value
                if (-not (Test-ExcludedIp -Ip $ip)) {
                    $discovered[$ip] = "ip"
                }
            }
        }

        # Find hostnames
        if ($config.patterns.hostnames) {
            foreach ($pattern in $config.patterns.hostnames) {
                foreach ($match in [regex]::Matches($content, "[a-zA-Z0-9][-a-zA-Z0-9\.]*($pattern)", "IgnoreCase")) {
                    $discovered[$match.Value] = "hostname"
                }
            }
        }
    }
    catch { Write-Verbose "Failed to scan $($file.FullName): $_" }
}

# === GENERATE MAPPINGS FOR NEW DISCOVERIES ===

$newMappings = @{}
foreach ($real in $discovered.Keys) {
    if ($config.mappings.ContainsKey($real)) { continue }
    if ($autoMappings.ContainsKey($real)) { continue }

    $fake = if ($discovered[$real] -eq "ip") { New-FakeIp } else { New-FakeHostname }
    $newMappings[$real] = $fake
    $autoMappings[$real] = $fake

    if (-not $Quiet) {
        Write-Host "  Discovered: $real -> $fake" -ForegroundColor DarkYellow
    }
}

# Save new auto mappings
if ($newMappings.Count -gt 0 -and -not $DryRun) {
    Save-AutoMappings -AutoMappings $autoMappings -SecretsPath $paths.Secrets
    if (-not $Quiet) {
        Write-Host "  Saved $($newMappings.Count) new mappings" -ForegroundColor Green
    }
}

# === BUILD COMBINED MAPPINGS ===

$allMappings = @{}
foreach ($k in $config.mappings.Keys) { $allMappings[$k] = $config.mappings[$k] }
foreach ($k in $autoMappings.Keys) {
    if (-not $allMappings.ContainsKey($k)) { $allMappings[$k] = $autoMappings[$k] }
}

# === SANITIZE FILES ===

$modifiedCount = 0
foreach ($file in $files) {
    try {
        $enc = Get-FileEncoding -Path $file.FullName
        $content = [System.IO.File]::ReadAllText($file.FullName, $enc)
        $original = $content

        $content = ConvertTo-SanitizedText -Text $content -Mappings $allMappings

        if ($content -ne $original) {
            if (-not $DryRun) {
                [System.IO.File]::WriteAllText($file.FullName, $content, $enc)
            }
            if (-not $Quiet) {
                Write-Host "  Sanitized: $($file.Name)" -ForegroundColor Green
            }
            $modifiedCount++
        }
    }
    catch {
        Write-Warning "Failed: $($file.FullName): $_"
    }
}

if (-not $Quiet) {
    Write-Host ""
    Write-Host "Done. Modified $modifiedCount files." -ForegroundColor Cyan
}
