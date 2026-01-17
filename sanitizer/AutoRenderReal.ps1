<#
.SYNOPSIS
    Stop hook - automatically renders real values to a separate directory.

.DESCRIPTION
    When Claude Code exits normally, this hook:
    1. Copies the working tree to %USERPROFILE%\.claude\rendered\{project}\
    2. Renders real values in that copy (fake -> real)
    3. Opens Explorer to show the location
    4. Working tree stays fake (safe for next session)

.PARAMETER SourceDir
    Source directory (the fake working tree). Defaults to current directory.

.PARAMETER NoExplorer
    Don't open Explorer after rendering.
#>

param(
    [string]$SourceDir = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [string]$RenderedBaseDir = "$env:USERPROFILE\.claude\rendered",
    [switch]$NoExplorer
)

$ErrorActionPreference = "SilentlyContinue"

$secretsPath = "$SanitizerDir\secrets.json"
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"

# === DETERMINE OUTPUT DIRECTORY ===

$projectName = Split-Path $SourceDir -Leaf
$outputDir = "$RenderedBaseDir\$projectName"

# === LOAD REVERSE MAPPINGS (fake -> real) ===

$reverseMappings = @{}

if (Test-Path $secretsPath) {
    try {
        $config = Get-Content $secretsPath -Raw | ConvertFrom-Json
        if ($config.mappings) {
            foreach ($prop in $config.mappings.PSObject.Properties) {
                $reverseMappings[$prop.Value] = $prop.Name
            }
        }
    }
    catch { }
}

if (Test-Path $autoMappingsPath) {
    try {
        $auto = Get-Content $autoMappingsPath -Raw | ConvertFrom-Json
        if ($auto.mappings) {
            foreach ($prop in $auto.mappings.PSObject.Properties) {
                if (-not $reverseMappings.ContainsKey($prop.Value)) {
                    $reverseMappings[$prop.Value] = $prop.Name
                }
            }
        }
    }
    catch { }
}

if ($reverseMappings.Count -eq 0) {
    Write-Host "No mappings found - skipping auto-render" -ForegroundColor Yellow
    exit 0
}

# Sort by fake value length descending
$sortedFakes = @($reverseMappings.Keys | Sort-Object { $_.Length } -Descending)

# === CLEAN AND CREATE OUTPUT DIR ===

if (Test-Path $outputDir) {
    Remove-Item -Path $outputDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -Path $outputDir -ItemType Directory -Force | Out-Null

# === GET EXCLUSIONS ===

$excludePaths = @(".git", "node_modules", ".claude", "bin", "obj", "__pycache__", "venv", ".venv")
$excludeExtensions = @(".exe", ".dll", ".pdb", ".png", ".jpg", ".jpeg", ".gif", ".ico", ".woff", ".woff2")

if (Test-Path $secretsPath) {
    try {
        $config = Get-Content $secretsPath -Raw | ConvertFrom-Json
        if ($config.excludePaths) { $excludePaths = $config.excludePaths }
        if ($config.excludeExtensions) { $excludeExtensions = $config.excludeExtensions }
    }
    catch { }
}

# === COPY AND RENDER ===

$modifiedCount = 0
$copiedCount = 0

Get-ChildItem -Path $SourceDir -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object {
    $file = $_
    $relativePath = $file.FullName.Substring($SourceDir.Length).TrimStart('\', '/')

    # Check path exclusions
    $excluded = $false
    foreach ($exclude in $excludePaths) {
        if ($relativePath -like "$exclude\*" -or $relativePath -like "$exclude/*" -or $relativePath -like "*\$exclude\*" -or $relativePath -eq $exclude) {
            $excluded = $true
            break
        }
    }
    if ($excluded) { return }

    # Skip huge files
    if ($file.Length -gt 10MB) { return }

    # Determine destination
    $destPath = Join-Path $outputDir $relativePath
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) {
        New-Item -Path $destDir -ItemType Directory -Force | Out-Null
    }

    # Check extension exclusions
    $isBinary = $false
    foreach ($ext in $excludeExtensions) {
        if ($file.Extension -ieq $ext) {
            $isBinary = $true
            break
        }
    }

    # Check if binary by content
    if (-not $isBinary) {
        try {
            $stream = [System.IO.File]::OpenRead($file.FullName)
            $buffer = [byte[]]::new([Math]::Min(8192, $stream.Length))
            $bytesRead = $stream.Read($buffer, 0, $buffer.Length)
            $stream.Close()
            $stream.Dispose()
            for ($i = 0; $i -lt $bytesRead; $i++) {
                if ($buffer[$i] -eq 0) { $isBinary = $true; break }
            }
        }
        catch { $isBinary = $true }
    }

    if ($isBinary) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $script:copiedCount++
        return
    }

    # Process text file - render real values
    try {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        if ([string]::IsNullOrEmpty($content)) {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
            $script:copiedCount++
            return
        }

        $originalContent = $content
        foreach ($fake in $sortedFakes) {
            $real = $reverseMappings[$fake]
            $content = $content -replace [regex]::Escape($fake), $real
        }

        [System.IO.File]::WriteAllText($destPath, $content)
        $script:copiedCount++

        if ($content -ne $originalContent) {
            $script:modifiedCount++
        }
    }
    catch {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $script:copiedCount++
    }
}

# === NOTIFY USER ===

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  REAL VERSION AUTO-RENDERED" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Location:" -ForegroundColor White
Write-Host "  $outputDir" -ForegroundColor Yellow
Write-Host ""
Write-Host "Files: $copiedCount copied, $modifiedCount with real values restored" -ForegroundColor Gray
Write-Host ""
Write-Host "This directory contains REAL secrets." -ForegroundColor Red
Write-Host "The working tree remains FAKE (safe)." -ForegroundColor Green
Write-Host ""

# === OPEN EXPLORER (if enabled in config) ===

$openExplorer = $false
if (-not $NoExplorer -and (Test-Path $secretsPath)) {
    try {
        $config = Get-Content $secretsPath -Raw | ConvertFrom-Json
        if ($config.openExplorerOnRender -eq $true) {
            $openExplorer = $true
        }
    }
    catch { }
}

if ($openExplorer) {
    Start-Process explorer.exe -ArgumentList $outputDir -ErrorAction SilentlyContinue
}

# === CREATE SHORTCUT IN SOURCE DIR ===

try {
    $shortcutPath = "$SourceDir\_REAL_VERSION.url"
    $shortcutContent = @"
[InternetShortcut]
URL=file:///$($outputDir -replace '\\', '/')
"@
    Set-Content -Path $shortcutPath -Value $shortcutContent -Encoding ASCII
}
catch { }
