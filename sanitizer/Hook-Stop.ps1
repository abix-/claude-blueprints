<#
.SYNOPSIS
    Stop hook - final sync to unsanitized directory.

.DESCRIPTION
    Copies working tree to unsanitized directory with fake → real replacement.
    Ensures all edits made during the session are captured.
#>

param(
    [string]$ProjectPath = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$Quiet
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

# Resolve unsanitized path
$projectName = Split-Path $ProjectPath -Leaf
$unsanitizedPath = $config.unsanitizedPath -replace '\{project\}', $projectName
$unsanitizedPath = $unsanitizedPath -replace '^~', $env:USERPROFILE

if (-not $Quiet) {
    Write-Host "Final sync to: $unsanitizedPath" -ForegroundColor Cyan
}

# Create unsanitized directory if needed
if (-not (Test-Path $unsanitizedPath)) {
    New-Item -Path $unsanitizedPath -ItemType Directory -Force | Out-Null
}

# Copy and unsanitize all files
$fileCount = 0
foreach ($file in Get-ChildItem -Path $ProjectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($ProjectPath.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $unsanitizedPath $relativePath
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) {
        New-Item -Path $destDir -ItemType Directory -Force | Out-Null
    }

    if (Test-BinaryFile -Path $file.FullName) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
    }
    else {
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            $content = ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings
            [System.IO.File]::WriteAllText($destPath, $content)
        }
        catch {
            Copy-Item -Path $file.FullName -Destination $destPath -Force
        }
    }
    $fileCount++
}

if (-not $Quiet) {
    Write-Host "Done. Synced $fileCount files." -ForegroundColor Cyan
}
