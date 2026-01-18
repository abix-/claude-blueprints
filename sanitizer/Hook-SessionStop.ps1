<#
.SYNOPSIS
    Stop hook - final sync to unsanitized directory.

.EXAMPLE
    # Called automatically by Claude Code via Stop hook
#>

[CmdletBinding()]
param(
    [string]$ProjectPath = (Get-Location).Path,
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

$ErrorActionPreference = "Stop"

Import-Module "$SanitizerDir\Sanitizer.psm1" -Force

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$config = Get-SanitizerConfig -SecretsPath $paths.Secrets
$reverseMappings = Get-ReverseMappings -SecretsPath $paths.Secrets

$projectName = Split-Path $ProjectPath -Leaf

# Default unsanitizedPath if not configured
$unsanitizedPathTemplate = if ($config.unsanitizedPath) { $config.unsanitizedPath } else { "~/.claude/unsanitized/{project}" }
$unsanitizedPath = $unsanitizedPathTemplate -replace '\{project\}', $projectName
$unsanitizedPath = $unsanitizedPath -replace '^~', $env:USERPROFILE

if (-not (Test-Path $unsanitizedPath)) { New-Item -Path $unsanitizedPath -ItemType Directory -Force | Out-Null }

foreach ($file in Get-ChildItem -Path $ProjectPath -Recurse -File -ErrorAction SilentlyContinue) {
    $relativePath = $file.FullName.Substring($ProjectPath.Length).TrimStart('\', '/')

    if (Test-ExcludedPath -RelativePath $relativePath -ExcludePaths $config.excludePaths) { continue }
    if ($file.Length -gt 10MB) { continue }

    $destPath = Join-Path $unsanitizedPath $relativePath
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
