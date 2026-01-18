<#
.SYNOPSIS
    Manually unsanitizes working tree to a separate directory.

.DESCRIPTION
    Use this script when:
    - Claude crashed and the Stop hook didn't run
    - You want to unsanitize to a specific custom location
    - You want to create multiple real versions

    Copies the sanitized working tree to a separate directory and applies
    reverse mappings (fake -> real) to get deployable code.

.PARAMETER SourceDir
    Source directory containing fake values. Defaults to current directory.

.PARAMETER OutputDir
    Destination directory for real values. REQUIRED.

.PARAMETER Force
    Overwrite output directory if it exists.

.EXAMPLE
    .\Unsanitize.ps1 -OutputDir C:\deploy\real

.EXAMPLE
    .\Unsanitize.ps1 -SourceDir C:\code\myproject -OutputDir C:\deploy\real -Force
#>

[CmdletBinding()]
param(
    [string]$SourceDir = (Get-Location).Path,

    [Parameter(Mandatory)]
    [string]$OutputDir,

    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",

    [switch]$Force
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Unsanitize to Real Values" -ForegroundColor Cyan
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

Write-Host "Loaded $($reverseMappings.Count) mappings" -ForegroundColor Cyan

if ($reverseMappings.Count -eq 0) {
    Write-Host "WARNING: No mappings found. Output will be identical to source." -ForegroundColor Yellow
}

# === CREATE OUTPUT DIR ===

New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null

# === COPY AND UNSANITIZE ===

Write-Host ""
Write-Host "Processing files..." -ForegroundColor Cyan

$modifiedCount = 0
$copiedCount = 0
$skippedCount = 0

foreach ($file in Get-ChildItem -Path $SourceDir -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($SourceDir.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) {
        $skippedCount++
        continue
    }

    if ($file.Length -gt 10MB) {
        $skippedCount++
        continue
    }

    $destPath = Join-Path $OutputDir $relativePath
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) {
        New-Item -Path $destDir -ItemType Directory -Force | Out-Null
    }

    $isBinary = Test-BinaryFile -Path $file.FullName

    if ($isBinary) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $copiedCount++
        continue
    }

    try {
        $content = [System.IO.File]::ReadAllText($file.FullName)
        if ([string]::IsNullOrEmpty($content)) {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
            $copiedCount++
            continue
        }

        $originalContent = $content
        $content = ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings

        [System.IO.File]::WriteAllText($destPath, $content)
        $copiedCount++

        if ($content -ne $originalContent) {
            Write-Host "  Unsanitized: $relativePath" -ForegroundColor Green
            $modifiedCount++
        }
    }
    catch {
        Write-Warning "Failed to process $relativePath`: $_"
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $copiedCount++
    }
}

# === SUMMARY ===

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Unsanitize Complete" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Output location:" -ForegroundColor White
Write-Host "  $OutputDir" -ForegroundColor Yellow
Write-Host ""
Write-Host "Files copied:   $copiedCount" -ForegroundColor Gray
Write-Host "Files modified: $modifiedCount" -ForegroundColor Gray
Write-Host "Files skipped:  $skippedCount" -ForegroundColor Gray
Write-Host ""
Write-Host "This directory contains REAL values." -ForegroundColor Red
Write-Host "Do NOT let Claude access it." -ForegroundColor Red
Write-Host ""
