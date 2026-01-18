<#
.SYNOPSIS
    PreToolUse hook - injects ansible-powershell skill before writing PowerShell files.

.EXAMPLE
    # Called automatically by Claude Code via PreToolUse hook
#>

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

# Parse hook input
$inputText = @($input) -join ""
if ([string]::IsNullOrEmpty($inputText)) { exit 0 }

try { $hookData = $inputText | ConvertFrom-Json -ErrorAction Stop }
catch { exit 0 }

if (-not $hookData -or $hookData.hook_event_name -ne "PreToolUse") { exit 0 }

$filePath = $hookData.tool_input.file_path
if (-not $filePath) { exit 0 }

# Only trigger for PowerShell files (.ps1, .psm1, .psd1)
if ($filePath -notmatch '\.(ps1|psm1|psd1)$') { exit 0 }

# Read the skill file
$skillPath = "$env:USERPROFILE\.claude\skills\ansible-powershell.md"
if (-not (Test-Path $skillPath)) { exit 0 }

$skillContent = Get-Content $skillPath -Raw

# Output skill content as stdout - this gets shown to Claude
Write-Output "=== POWERSHELL SKILL REMINDER ==="
Write-Output $skillContent
Write-Output "=== YOU MUST INCLUDE CONFIDENCE RATING (1-10) ==="

exit 0
