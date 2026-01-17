<#
.SYNOPSIS
    Sanitizes command output (stdin -> stdout).

.DESCRIPTION
    Applies all known mappings plus fallback IP scrubbing for unknown values.
    Used by SealedExec.ps1 to sanitize command output before returning to Claude.

.EXAMPLE
    $output | .\SanitizeOutput.ps1
#>

param(
    [string]$SanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
)

Import-Module "$SanitizerDir\Common.psm1" -Force

$text = @($input) -join "`n"
if ([string]::IsNullOrEmpty($text)) { return }

$paths = Get-SanitizerPaths -SanitizerDir $SanitizerDir
$mappings = Get-SanitizerMappings -SecretsPath $paths.Secrets -AutoMappingsPath $paths.AutoMappings

ConvertTo-ScrubbedText -Text $text -Mappings $mappings
