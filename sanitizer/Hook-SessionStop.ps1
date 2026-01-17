<#
.SYNOPSIS
    Stop hook - renders real values when Claude exits.

.DESCRIPTION
    Copies working tree to ~/.claude/rendered/{project}/ with real values restored.
    Working tree stays fake (safe for next session).

.PARAMETER SourceDir
    Source directory (the fake working tree). Defaults to current directory.

.PARAMETER NoExplorer
    Don't open Explorer after rendering (overrides config).
#>

param(
    [string]$SourceDir = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$NoExplorer
)

$ErrorActionPreference = "SilentlyContinue"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

if ($reverseMappings.Count -eq 0) {
    Write-Host "No mappings found - skipping auto-render" -ForegroundColor Yellow
    exit 0
}

# === DETERMINE OUTPUT DIRECTORY ===

$projectName = Split-Path $SourceDir -Leaf
$outputDir = "$($paths.RenderedBase)\$projectName"

# === CLEAN AND CREATE OUTPUT DIR ===

if (Test-Path $outputDir) {
    Remove-Item -Path $outputDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -Path $outputDir -ItemType Directory -Force | Out-Null

# === COPY AND RENDER ===

$modifiedCount = 0
$copiedCount = 0

foreach ($file in Get-ChildItem -Path $SourceDir -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($SourceDir.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $outputDir $relativePath
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
            $modifiedCount++
        }
    }
    catch {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
        $copiedCount++
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

if (-not $NoExplorer -and $config.openExplorerOnRender -eq $true) {
    Start-Process explorer.exe -ArgumentList $outputDir -ErrorAction SilentlyContinue
}

# === CREATE SHORTCUT IN SOURCE DIR ===

try {
    $shortcutContent = @"
[InternetShortcut]
URL=file:///$($outputDir -replace '\\', '/')
"@
    Set-Content -Path "$SourceDir\_REAL_VERSION.url" -Value $shortcutContent -Encoding ASCII
}
catch { }
