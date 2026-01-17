<#
.SYNOPSIS
    Sanitizes project files by replacing real values with fake values.

.DESCRIPTION
    - Loads manual mappings from secrets.json
    - Auto-discovers IPs and hostnames matching patterns
    - Generates random fake values and stores them in auto_mappings.json
    - Preserves file encoding

.EXAMPLE
    .\Sanitize.ps1
    .\Sanitize.ps1 -ProjectPath C:\code\myproject
    .\Sanitize.ps1 -DryRun
#>

param(
    [string]$ProjectPath = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$Quiet,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

$secretsPath = "$SanitizerDir\secrets.json"
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"

# === HELPER FUNCTIONS ===

# Generate a random fake IP in 11.x.x.x range
function New-FakeIp {
    $b2 = Get-Random -Minimum 1 -Maximum 255
    $b3 = Get-Random -Minimum 1 -Maximum 255
    $b4 = Get-Random -Minimum 1 -Maximum 255
    return "11.$b2.$b3.$b4"
}

# Generate a random fake hostname
function New-FakeHostname {
    $chars = 'abcdefghijklmnopqrstuvwxyz0123456789'
    $suffix = -join (1..8 | ForEach-Object { $chars[(Get-Random -Maximum $chars.Length)] })
    return "host-$suffix.example.test"
}

function Test-BinaryFile {
    param([string]$Path)
    try {
        $stream = [System.IO.File]::OpenRead($Path)
        $buffer = [byte[]]::new([Math]::Min(8192, $stream.Length))
        $bytesRead = $stream.Read($buffer, 0, $buffer.Length)
        $stream.Close(); $stream.Dispose()
        for ($i = 0; $i -lt $bytesRead; $i++) {
            if ($buffer[$i] -eq 0) { return $true }
        }
        return $false
    }
    catch { return $true }
}

function Get-FileEncoding {
    param([string]$Path)
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        return [System.Text.UTF8Encoding]::new($true)
    }
    elseif ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
        return [System.Text.UnicodeEncoding]::new($false, $true)
    }
    return [System.Text.UTF8Encoding]::new($false)
}

# IPs to never sanitize
$excludedIpPatterns = @('^127\.', '^0\.0\.0\.0$', '^255\.255\.255\.255$', '^169\.254\.', '^224\.', '^239\.', '^11\.\d+\.\d+\.\d+$')

function Test-ExcludedIp {
    param([string]$Ip)
    foreach ($p in $excludedIpPatterns) { if ($Ip -match $p) { return $true } }
    return $false
}

# === LOAD CONFIG ===

$config = @{
    mappings = @{}
    excludePaths = @(".git", "node_modules", ".claude", "bin", "obj", "__pycache__", "venv")
    excludeExtensions = @(".exe", ".dll", ".pdb", ".png", ".jpg", ".gif", ".ico")
    patterns = @{ ipv4 = $true; hostnames = @() }
}

if (Test-Path $secretsPath) {
    $loaded = Get-Content $secretsPath -Raw | ConvertFrom-Json
    if ($loaded.mappings) {
        foreach ($prop in $loaded.mappings.PSObject.Properties) {
            $config.mappings[$prop.Name] = $prop.Value
        }
    }
    if ($loaded.excludePaths) { $config.excludePaths = $loaded.excludePaths }
    if ($loaded.excludeExtensions) { $config.excludeExtensions = $loaded.excludeExtensions }
    if ($loaded.patterns) { $config.patterns = $loaded.patterns }
}

# Load existing auto mappings
$autoMappings = @{}
if (Test-Path $autoMappingsPath) {
    $loaded = Get-Content $autoMappingsPath -Raw | ConvertFrom-Json
    if ($loaded.mappings) {
        foreach ($prop in $loaded.mappings.PSObject.Properties) {
            $autoMappings[$prop.Name] = $prop.Value
        }
    }
}

if (-not $Quiet) {
    Write-Host "Sanitizing: $ProjectPath" -ForegroundColor Cyan
    Write-Host "  Manual mappings: $($config.mappings.Count)" -ForegroundColor Gray
    Write-Host "  Auto mappings: $($autoMappings.Count)" -ForegroundColor Gray
}

# === GATHER FILES ===

$files = @(Get-ChildItem -Path $ProjectPath -Recurse -File -ErrorAction SilentlyContinue | Where-Object {
    $f = $_
    $rel = $f.FullName.Substring($ProjectPath.Length).TrimStart('\', '/')
    foreach ($ex in $config.excludePaths) {
        if ($rel -like "$ex\*" -or $rel -like "*\$ex\*") { return $false }
    }
    foreach ($ext in $config.excludeExtensions) {
        if ($f.Extension -ieq $ext) { return $false }
    }
    if ($f.Length -eq 0 -or $f.Length -gt 10MB) { return $false }
    if (Test-BinaryFile $f.FullName) { return $false }
    return $true
})

# === DISCOVER VALUES ===

$ipv4Regex = '\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b'
$discovered = @{}

foreach ($file in $files) {
    try {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        if ([string]::IsNullOrEmpty($content)) { continue }

        # Find IPs
        if ($config.patterns.ipv4) {
            foreach ($m in [regex]::Matches($content, $ipv4Regex)) {
                $ip = $m.Value
                if (-not (Test-ExcludedIp $ip)) { $discovered[$ip] = "ip" }
            }
        }

        # Find hostnames
        if ($config.patterns.hostnames) {
            foreach ($pattern in $config.patterns.hostnames) {
                foreach ($m in [regex]::Matches($content, "[a-zA-Z0-9][-a-zA-Z0-9\.]*($pattern)", "IgnoreCase")) {
                    $discovered[$m.Value] = "hostname"
                }
            }
        }
    }
    catch { }
}

# === GENERATE MAPPINGS FOR NEW DISCOVERIES ===

$newMappings = @{}
foreach ($real in $discovered.Keys) {
    if ($config.mappings.ContainsKey($real)) { continue }
    if ($autoMappings.ContainsKey($real)) { continue }

    $fake = if ($discovered[$real] -eq "ip") { New-FakeIp } else { New-FakeHostname }
    $newMappings[$real] = $fake
    $autoMappings[$real] = $fake

    if (-not $Quiet) { Write-Host "  Discovered: $real -> $fake" -ForegroundColor DarkYellow }
}

# Save new auto mappings
if ($newMappings.Count -gt 0 -and -not $DryRun) {
    @{ mappings = $autoMappings } | ConvertTo-Json -Depth 5 | Set-Content -Path $autoMappingsPath -Encoding UTF8
    if (-not $Quiet) { Write-Host "  Saved $($newMappings.Count) new mappings" -ForegroundColor Green }
}

# === BUILD REPLACEMENTS ===

$replacements = @()
foreach ($k in $config.mappings.Keys) { $replacements += @{ Real = $k; Fake = $config.mappings[$k] } }
foreach ($k in $autoMappings.Keys) {
    if (-not $config.mappings.ContainsKey($k)) { $replacements += @{ Real = $k; Fake = $autoMappings[$k] } }
}
$replacements = @($replacements | Sort-Object { $_.Real.Length } -Descending)

# === SANITIZE FILES ===

$modifiedCount = 0
foreach ($file in $files) {
    try {
        $enc = Get-FileEncoding -Path $file.FullName
        $content = [System.IO.File]::ReadAllText($file.FullName, $enc)
        $original = $content

        foreach ($r in $replacements) {
            $content = $content -replace [regex]::Escape($r.Real), $r.Fake
        }

        if ($content -ne $original) {
            if (-not $DryRun) { [System.IO.File]::WriteAllText($file.FullName, $content, $enc) }
            if (-not $Quiet) { Write-Host "  Sanitized: $($file.Name)" -ForegroundColor Green }
            $modifiedCount++
        }
    }
    catch { Write-Warning "Failed: $($file.FullName): $_" }
}

if (-not $Quiet) {
    Write-Host ""
    Write-Host "Done. Modified $modifiedCount files." -ForegroundColor Cyan
}
