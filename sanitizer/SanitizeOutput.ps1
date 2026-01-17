<#
.SYNOPSIS
    Sanitizes command output (stdin -> stdout).

.DESCRIPTION
    - Applies all known mappings (manual + auto-discovered)
    - Falls back to pattern-based scrubbing for unknown IPs
#>

param(
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

$secretsPath = "$SanitizerDir\secrets.json"
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"

# Read input
$text = @($input) -join "`n"
if ([string]::IsNullOrEmpty($text)) { return }

# Simple hash for fallback IP scrubbing
function Get-SimpleHash {
    param([string]$Value)
    $md5 = [System.Security.Cryptography.MD5]::Create()
    $hash = $md5.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($Value))
    $md5.Dispose()
    return [BitConverter]::ToString($hash).Replace("-", "").ToLower()
}

function Get-FallbackFakeIp {
    param([string]$RealIp)
    $hash = Get-SimpleHash "ip:$RealIp"
    $b2 = ([Convert]::ToInt32($hash.Substring(0, 2), 16) % 254) + 1
    $b3 = ([Convert]::ToInt32($hash.Substring(2, 2), 16) % 254) + 1
    $b4 = ([Convert]::ToInt32($hash.Substring(4, 2), 16) % 254) + 1
    return "11.$b2.$b3.$b4"
}

# Load mappings
$mappings = @{}

if (Test-Path $secretsPath) {
    try {
        $c = Get-Content $secretsPath -Raw | ConvertFrom-Json
        if ($c.mappings) {
            foreach ($p in $c.mappings.PSObject.Properties) { $mappings[$p.Name] = $p.Value }
        }
    } catch { }
}

if (Test-Path $autoMappingsPath) {
    try {
        $a = Get-Content $autoMappingsPath -Raw | ConvertFrom-Json
        if ($a.mappings) {
            foreach ($p in $a.mappings.PSObject.Properties) {
                if (-not $mappings.ContainsKey($p.Name)) { $mappings[$p.Name] = $p.Value }
            }
        }
    } catch { }
}

# Apply known mappings (longest first)
foreach ($real in ($mappings.Keys | Sort-Object { $_.Length } -Descending)) {
    $text = $text -replace [regex]::Escape($real), $mappings[$real]
}

# Fallback: scrub any remaining IPs not in mappings
$ipRegex = '\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b'
$excluded = @('^127\.', '^0\.0\.0\.0$', '^255\.255\.255\.255$', '^169\.254\.', '^224\.', '^239\.', '^11\.\d+\.\d+\.\d+$')

$text = [regex]::Replace($text, $ipRegex, {
    param($m)
    $ip = $m.Value
    foreach ($p in $excluded) { if ($ip -match $p) { return $ip } }
    return Get-FallbackFakeIp $ip
})

$text
