<#
.SYNOPSIS
    Manually unsanitizes working tree to a separate directory.

.EXAMPLE
    .\Unsanitize.ps1 -OutputDir C:\deploy\real
#>

[CmdletBinding()]
param(
    [string]$SourceDir = (Get-Location).Path,
    [Parameter(Mandatory)][string]$OutputDir,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer",
    [switch]$Force
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

if (-not (Test-Path $SourceDir)) { throw "Source not found: $SourceDir" }

if (Test-Path $OutputDir) {
    if ($Force) { Remove-Item -Path $OutputDir -Recurse -Force }
    else { throw "Output exists: $OutputDir (use -Force)" }
}

New-Item -Path $OutputDir -ItemType Directory -Force | Out-Null

foreach ($file in Get-ChildItem -Path $SourceDir -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($SourceDir.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $OutputDir $relativePath
    $destDir = Split-Path $destPath -Parent

    if (-not (Test-Path $destDir)) { New-Item -Path $destDir -ItemType Directory -Force | Out-Null }

    if (Test-BinaryFile -Path $file.FullName) {
        Copy-Item -Path $file.FullName -Destination $destPath -Force
    }
    else {
        try {
            $content = [System.IO.File]::ReadAllText($file.FullName)
            $content = ConvertTo-RenderedText -Text $content -ReverseMappings $reverseMappings
            [System.IO.File]::WriteAllText($destPath, $content)
        }
        catch { Copy-Item -Path $file.FullName -Destination $destPath -Force }
    }
}
