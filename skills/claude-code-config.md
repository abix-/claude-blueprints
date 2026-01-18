---
name: claude-code-config
description: How to configure Claude Code settings, hooks, and permissions correctly
metadata:
  version: "1.4"
  updated: "2026-01-18"
---
# Claude Code Configuration

## Permissions Deny Rules (BROKEN)

**The `permissions.deny` array in settings.json does NOT work** — known bug ([#6699](https://github.com/anthropics/claude-code/issues/6699)).

```json
"permissions": {
  "deny": ["~/.claude/sanitizer/sanitizer.json"]
}
```

This config is ignored. Claude can still read/write denied files.

## Enforcing File Access Restrictions

Use `PreToolUse` hooks that return `permissionDecision: "deny"`:

### Hook Script (PowerShell)
```powershell
<#
.SYNOPSIS
    PreToolUse hook - blocks access to sensitive files.

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

# Normalize and check against blocked patterns
$normalizedPath = $filePath -replace '\\', '/'
$blockedPatterns = @('\.claude/sanitizer/sanitizer\.json$', '\.claude/unsanitized/')

foreach ($pattern in $blockedPatterns) {
    if ($normalizedPath -match $pattern) {
        @{
            hookSpecificOutput = @{
                hookEventName = "PreToolUse"
                permissionDecision = "deny"
                reason = "Access blocked: sensitive file"
            }
        } | ConvertTo-Json -Depth 5 -Compress
        exit 0
    }
}

exit 0
```

### settings.json Hook Config
```json
"PreToolUse": [
  {
    "matcher": "Read|Edit|Write",
    "hooks": [{
      "type": "command",
      "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"path/to/hook.ps1\""
    }]
  }
]
```

## Hook Types

| Event | When | Use Case |
|-------|------|----------|
| `SessionStart` | Session begins | Init, validation |
| `PreToolUse` | Before tool executes | Block/modify tool calls |
| `PostToolUse` | After tool executes | Sanitize output |
| `Stop` | Session ends | Cleanup, sync |

## Hook Response Format

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "allow|deny",
    "reason": "optional message",
    "updatedInput": { }
  }
}
```

- `deny` — blocks the tool call
- `allow` — permits (can modify input via `updatedInput`)
- Exit code 2 also blocks (alternative to JSON response)

## Matcher Syntax

- Single tool: `"Bash"`
- Multiple tools: `"Read|Edit|Write"`
- Empty string: matches all

## Workflow: Syncing Changes

After modifying skills, hooks, or Claude config in `~/.claude/`:

1. Update `~/.claude/CLAUDE.md` if adding/changing skill references
2. Copy updated files to `C:\Code\claude-blueprints\`
3. Update `C:\Code\claude-blueprints\README.md` if structure/components changed
4. Commit and push to git

This keeps the repo as the source of truth for all Claude configuration.
