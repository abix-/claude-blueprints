---
name: claude-code-config
description: How to configure Claude Code settings, hooks, and permissions correctly
metadata:
  version: "1.5"
  updated: "2026-01-18"
---
# Claude Code Configuration

## Permissions Deny (BROKEN)

`permissions.deny` in settings.json does NOT work — bug [#6699](https://github.com/anthropics/claude-code/issues/6699). Use hooks instead.

## Hook Types

| Event | When | Use Case |
|-------|------|----------|
| `SessionStart` | Session begins | Init, inject skills |
| `PreToolUse` | Before tool executes | Block/modify tool calls |
| `PostToolUse` | After tool executes | Sanitize output |
| `Stop` | Session ends | Cleanup |

## Blocking File Access via Hook

Return JSON with `permissionDecision: "deny"`:

```json
{
  "hookSpecificOutput": {
    "hookEventName": "PreToolUse",
    "permissionDecision": "deny",
    "reason": "Access blocked"
  }
}
```

Exit code 2 also blocks (alternative to JSON).

## Hook Config (settings.json)

```json
"PreToolUse": [{
  "matcher": "Read|Edit|Write",
  "hooks": [{ "type": "command", "command": "powershell.exe -NoProfile -File hook.ps1" }]
}]
```

**Matcher syntax:** `"Bash"` (single), `"Read|Edit|Write"` (multiple), `""` (all)

## Hook Input

Hooks receive JSON on stdin with `hook_event_name` and `tool_input`. Parse and check `tool_input.file_path` for file operations.
