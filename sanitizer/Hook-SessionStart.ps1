<#
.SYNOPSIS
    SessionStart hook - sanitizes project files.

.EXAMPLE
    .\Hook-SessionStart.ps1
#>

[CmdletBinding()]
param(
    [string]$ProjectPath = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"

# Auto-initialize if sanitizer.json doesn't exist
$secretsPath = "$SanitizerDir\sanitizer.json"
if (-not (Test-Path $secretsPath)) {
    @($SanitizerDir, "$env:USERPROFILE\.claude\unsanitized") | ForEach-Object {
        if (-not (Test-Path $_)) { New-Item -Path $_ -ItemType Directory -Force | Out-Null }
    }
    @{
        mappings = @{}
        autoMappings = @{}
        patterns = @{ ipv4 = $true; hostnames = @() }
        unsanitizedPath = "~/.claude/unsanitized/{project}"
        excludePaths = @(".git", "node_modules", ".venv", "__pycache__")
    } | ConvertTo-Json -Depth 5 | Set-Content -Path $secretsPath -Encoding UTF8
}

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets

$autoMappings = @{}
foreach ($key in $config.autoMappings.Keys) { $autoMappings[$key] = $config.autoMappings[$key] }

# Gather files
$files = foreach ($file in Get-ChildItem -Path $ProjectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($ProjectPath.Length).TrimStart('\', '/')
    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -eq 0 -or $file.Length -gt 10MB) { continue }
    if (Test-BinaryFile -Path $file.FullName) { continue }
    $file
}

# Discover values
$discovered = @{}
foreach ($file in $files) {
    try {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        if ([string]::IsNullOrEmpty($content)) { continue }

        if ($config.patterns.ipv4) {
            foreach ($match in [regex]::Matches($content, $Ipv4Regex)) {
                if (-not (Test-ExcludedIp -Ip $match.Value)) { $discovered[$match.Value] = "ip" }
            }
        }

        if ($config.patterns.hostnames) {
            foreach ($pattern in $config.patterns.hostnames) {
                foreach ($match in [regex]::Matches($content, "[a-zA-Z0-9][-a-zA-Z0-9\.]*($pattern)", "IgnoreCase")) {
                    $discovered[$match.Value] = "hostname"
                }
            }
        }
    }
    catch { }
}

# Generate mappings for new discoveries
foreach ($real in $discovered.Keys) {
    if ($config.mappings.ContainsKey($real) -or $autoMappings.ContainsKey($real)) { continue }
    $autoMappings[$real] = if ($discovered[$real] -eq "ip") { New-FakeIp } else { New-FakeHostname }
}

if (-not $DryRun -and $autoMappings.Count -gt $config.autoMappings.Count) {
    Save-AutoMappings -AutoMappings $autoMappings -SecretsPath $paths.Secrets
}

# Build combined mappings
$allMappings = @{}
foreach ($k in $config.mappings.Keys) { $allMappings[$k] = $config.mappings[$k] }
foreach ($k in $autoMappings.Keys) { if (-not $allMappings.ContainsKey($k)) { $allMappings[$k] = $autoMappings[$k] } }

# Sanitize files
foreach ($file in $files) {
    try {
        $enc = Get-FileEncoding -Path $file.FullName
        $content = [System.IO.File]::ReadAllText($file.FullName, $enc)
        $sanitized = ConvertTo-SanitizedText -Text $content -Mappings $allMappings

        if ($sanitized -ne $content -and -not $DryRun) {
            [System.IO.File]::WriteAllText($file.FullName, $sanitized, $enc)
        }
    }
    catch { Write-Warning "Failed: $($file.FullName)" }
}
