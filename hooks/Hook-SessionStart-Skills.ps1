<#
.SYNOPSIS
    SessionStart hook - injects skills once at session start.
#>

[CmdletBinding()]
param()

$skills = @(
    "$env:USERPROFILE\.claude\skills\try-harder.md"
    "$env:USERPROFILE\.claude\skills\code.md"
    "$env:USERPROFILE\.claude\skills\claude-config.md"
)

foreach ($skill in $skills) {
    if (Test-Path $skill) {
        Get-Content $skill -Raw
    }
}
