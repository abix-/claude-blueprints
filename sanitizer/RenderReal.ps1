<#
.SYNOPSIS
    Manually renders real values to a separate directory.

.DESCRIPTION
    Use this script when:
    - Claude crashed and the Stop hook didn't run
    - You want to render to a specific custom location
    - You want to create multiple real versions

    This script copies the fake working tree to a separate directory
    and applies reverse mappings (fake -> real) to get usable code.

.PARAMETER SourceDir
    Source directory containing fake values. Defaults to current directory.

.PARAMETER OutputDir
    Destination directory for real values. REQUIRED.

.PARAMETER Force
    Overwrite output directory if it exists.

.EXAMPLE
    .\RenderReal.ps1 -SourceDir C:\code\myproject -OutputDir C:\deploy\real
    .\RenderReal.ps1 -OutputDir C:\deploy\real -Force
#>

param(
    [string]$SourceDir = (Get-Location).Path,

    [Parameter(Mandatory=$true)]
    [string]$OutputDir,

    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",

    [switch]$Force
)

$ErrorActionPreference = "Stop"

$secretsPath = "$SanitizerDir\secrets.json"
$autoMappingsPath = "$SanitizerDir\auto_mappings.json"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Render Real Values" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Source (fake):  $SourceDir" -ForegroundColor Gray
Write-Host "Output (real):  $OutputDir" -ForegroundColor Gray
Write-Host ""

# === VALIDATE ===

if (-not (Test-Path $SourceDir)) {
    Write-Host "ERROR: Source directory does not exist: $SourceDir" -ForegroundColor Red
    exit 1
}

if (Test-Path $OutputDir) {
    if ($Force) {
        Write-Host "Removing existing output directory..." -ForegroundColor Yellow
        Remove-Item -Path $OutputDir -Recurse -Force
    }
    else {
        Write-Host "ERROR: Output directory already exists: $OutputDir" -ForegroundColor Red
        Write-Host "Use -Force to overwrite." -ForegroundColor Yellow
        exit 1
    }
}

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
    catch {
        Write-Warning "Failed to parse secrets.json: $_"
    }
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
    catch {
        Write-Warning "Failed to parse auto_mappings.json: $_"
    }
}

Write-Host "Loaded $($reverseMappings.Count) mappings" -ForegroundColor Cyan

if ($reverseMappings.Count -eq 0) {
    Write-Host "WARNING: No mappings found. Output will be identical to source." -ForegroundColor Yellow
}

# Sort by fake value length descending
$sortedFakes = @($reverseMappings.Keys | Sort-Object { $_.Length } -Descending)

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

# === CREATE OUTPUT DIR ===

New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null

# === COPY AND RENDER ===

Write-Host ""
Write-Host "Processing files..." -ForegroundColor Cyan

$modifiedCount = 0
$copiedCount = 0
$skippedCount = 0

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
    if ($excluded) {
        $script:skippedCount++
        return
    }

    # Skip huge files
    if ($file.Length -gt 10MB) {
        $script:skippedCount++
        return
    }

    # Determine destination
    $destPath = Join-Path $OutputDir $relativePath
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

    # Process text file
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
            Write-Host "  Rendered: $relativePath" -ForegroundColor Green
            $script:modifiedCount++
        }
    }
    catch {
        Write-Warning "Failed to process $relativePath`: $_"
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $script:copiedCount++
    }
}

# === SUMMARY ===

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Render Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output location:" -ForegroundColor White
Write-Host "  $OutputDir" -ForegroundColor Yellow
Write-Host ""
Write-Host "Files copied:   $copiedCount" -ForegroundColor Gray
Write-Host "Files modified: $modifiedCount" -ForegroundColor Gray
Write-Host "Files skipped:  $skippedCount" -ForegroundColor Gray
Write-Host ""
Write-Host "This directory contains REAL secrets." -ForegroundColor Red
Write-Host "Do NOT let Claude access it." -ForegroundColor Red
Write-Host ""
