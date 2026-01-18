<#
.SYNOPSIS
    SessionStart hook - injects skills once at session start.

.EXAMPLE
    # Called automatically by Claude Code via SessionStart hook
#>

[CmdletBinding()]
param()

$skills = @(
    "$env:USERPROFILE\.claude\skills\try-harder.md"
    "$env:USERPROFILE\.claude\skills\ansible-powershell.md"
    "$env:USERPROFILE\.claude\skills\claude-config.md"
)

foreach ($skill in $skills) {
    if (Test-Path $skill) {
        Get-Content $skill -Raw
    }
}
