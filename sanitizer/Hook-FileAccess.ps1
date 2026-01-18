<#
.SYNOPSIS
    PreToolUse hook for Read/Edit/Write - blocks access to sensitive files.
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

# Get file path from tool input (Read/Edit/Write all use file_path)
$filePath = $hookData.tool_input.file_path
if (-not $filePath) { exit 0 }

# Normalize path for matching
$normalizedPath = $filePath -replace '\\', '/'

# Blocked patterns - files Claude should never access
$blockedPatterns = @(
    '\.claude/sanitizer/sanitizer\.json$',
    '\.claude/unsanitized/'
)

foreach ($pattern in $blockedPatterns) {
    if ($normalizedPath -match $pattern) {
        @{
            hookSpecificOutput = @{
                hookEventName = "PreToolUse"
                permissionDecision = "deny"
                reason = "Access blocked: sensitive sanitizer file"
            }
        } | ConvertTo-Json -Depth 5 -Compress
        exit 0
    }
}

# Allow access
exit 0
