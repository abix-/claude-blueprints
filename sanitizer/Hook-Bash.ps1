<#
.SYNOPSIS
    PreToolUse hook for Bash - routes commands through sealed execution.

.DESCRIPTION
    Intercepts Bash commands and either:
    - Passes through safe commands (git, ls, etc.)
    - Routes other commands through SealedExec.ps1 for isolated execution
#>

$sanitizerDir = "$env:USERPROFILE\.claude\sanitizer"
$sealedExecScript = "$sanitizerDir\SealedExec.ps1"

# Parse hook input from stdin
$inputText = @($input) -join ""

if ([string]::IsNullOrEmpty($inputText)) {
    exit 0
}

try {
    $hookData = $inputText | ConvertFrom-Json -ErrorAction Stop
}
catch {
    exit 0
}

if (-not $hookData) { exit 0 }

$hookEvent = $hookData.hook_event_name
$command = $hookData.tool_input.command

# Only handle PreToolUse for Bash
if ($hookEvent -ne "PreToolUse") { exit 0 }
if (-not $command) { exit 0 }

# === BLOCK DANGEROUS COMMANDS ===

$blockedPatterns = @(
    '[\\/]secrets\.json(?![.\w])'           # exact file, not secrets.json.example
    '[\\/]auto_mappings\.json(?![.\w])'     # exact file
    '[\\/]ip_mappings_temp\.json(?![.\w])'  # exact file
    'claude-sealed-'                        # temp sealed directories
    '\.claude[\\/]rendered'                 # rendered output directory
)

foreach ($pattern in $blockedPatterns) {
    if ($command -match $pattern) {
        @{
            hookSpecificOutput = @{
                hookEventName = "PreToolUse"
                permissionDecision = "deny"
                reason = "Access to sanitizer files is blocked for security"
            }
        } | ConvertTo-Json -Depth 5 -Compress
        exit 0
    }
}

# === PASSTHROUGH COMMANDS (run directly, sanitize output only) ===
# These commands don't need real values and must run in actual working directory

$passthroughPatterns = @(
    '^\s*git\s'           # git commands
    '^\s*gh\s'            # GitHub CLI
    '^\s*cd\s'            # directory changes
    '^\s*ls\b'            # list files
    '^\s*dir\b'           # list files (Windows)
    '^\s*pwd\b'           # print working directory
    '^\s*echo\s'          # echo (often used for simple output)
    '^\s*mkdir\s'         # create directories
    '^\s*rm\s'            # remove files
    '^\s*cp\s'            # copy files
    '^\s*mv\s'            # move files
    '^\s*cat\s'           # cat (reading already-sanitized files is fine)
    '^\s*head\s'          # head
    '^\s*tail\s'          # tail
    '^\s*wc\s'            # word count
    '^\s*find\s'          # find files
    '^\s*grep\s'          # grep (searching sanitized content)
    '^\s*which\s'         # which command
    '^\s*where\s'         # where command (Windows)
    '^\s*test\s'          # test conditions
    '^\s*\[\s'            # test conditions [ ]
)

$isPassthrough = $false
foreach ($pattern in $passthroughPatterns) {
    if ($command -match $pattern) {
        $isPassthrough = $true
        break
    }
}

if ($isPassthrough) {
    # Allow command to run directly - output will be sanitized by PostToolUse if needed
    # No modification needed, just allow it
    exit 0
}

# === WRAP COMMAND FOR SEALED EXECUTION ===

# Escape the command for passing to SealedExec
$escapedCommand = $command -replace '"', '\"' -replace "'", "''"

# Build the wrapped command that executes through SealedExec
# SealedExec returns JSON with {output, exitCode}
# We parse it and output just the output text

$wrappedCommand = @"
powershell.exe -ExecutionPolicy Bypass -NoProfile -Command "
    `$result = & '$sealedExecScript' -Command '$escapedCommand' | ConvertFrom-Json
    `$result.output
    exit `$result.exitCode
"
"@

# Return the modified command
@{
    hookSpecificOutput = @{
        hookEventName = "PreToolUse"
        permissionDecision = "allow"
        updatedInput = @{
            command = $wrappedCommand
        }
    }
} | ConvertTo-Json -Depth 5 -Compress
