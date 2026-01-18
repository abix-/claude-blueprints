---
name: claude-config
description: Managing Claude configuration - skills, hooks, settings, and sync workflow. Read first when modifying any Claude config.
metadata:
  version: "1.3"
  updated: "2026-01-18"
---
# Claude Config

## Skills

### Writing Effective Skills
- Write for Claude, not humans. Clear instructions > motivational framing.
- Be explicit about scope. When does this skill apply?
- Cut repetition. Say it once.

### Token Bloat
| Bloat | Lean |
|-------|------|
| "You should always..." | Just state the rule |
| Headers for 2-3 lines | Skip the header |
| Paragraphs | Bullets |
| Same concept repeated | Say it once |

### Frontmatter
```yaml
---
name: skill-name           # required
description: When to use   # required
metadata:                  # optional
  version: "X.Y"
  updated: "YYYY-MM-DD"
---
```
Only these keys + `license`, `allowed-tools`, `compatibility` at root. Unknown keys → upload rejection on Claude web.

### Versioning
- Increment version on every change
- Format: `major.minor` (major = breaking, minor = additions/fixes)
- Update date with every version change

## CLAUDE.md Format

Explicit read instructions — "follow standards in X" doesn't trigger file reads.

**Good:** `When writing PowerShell, read ~/.claude/skills/ansible-powershell.md first.`
**Bad:** `Follow standards in ~/.claude/skills/ansible-powershell.md`

Keep lean: no headers, one skill per line, trigger + read instruction.

## Hooks

| Event | When | Use Case |
|-------|------|----------|
| `SessionStart` | Session begins | Init, inject skills |
| `PreToolUse` | Before tool executes | Block/modify tool calls |
| `PostToolUse` | After tool executes | Sanitize output |
| `Stop` | Session ends | Cleanup |

### Hook Config (settings.json)
```json
"PreToolUse": [{
  "matcher": "Read|Edit|Write",
  "hooks": [{ "type": "command", "command": "powershell.exe -NoProfile -File hook.ps1" }]
}]
```
Matcher: `"Bash"` (single), `"Read|Edit|Write"` (multiple), `""` (all)

### Blocking via Hook
Return JSON with `"permissionDecision": "deny"` or exit code 2.

### Hook Input
JSON on stdin with `hook_event_name` and `tool_input`. Check `tool_input.file_path` for file ops.

## Sync Workflow

**Always edit ~/.claude first, then /claude-push. Never edit repo directly.**

`/claude-push` — sync ~/.claude to repo and push
`/claude-pull` — pull repo and apply to ~/.claude

Both sync: skills/, hooks/, commands/, sanitizer/, CLAUDE.md, settings.json

### Command Files
- Embed code directly in command files — don't create temp/helper scripts
- When a command exists, use it — don't manually recreate its embedded code
- Code > instructions for reliability (deterministic, no interpretation variance)
- Sync operations should remove orphan files, not just overwrite
- Bash mangles PowerShell $ variables — use -EncodedCommand (base64) when needed

### After Changes
1. Make changes locally in ~/.claude
2. Run `/claude-push` to sync to repo

### Getting Updates
1. Run `/claude-pull` to get latest from repo

## Notes

`permissions.deny` in settings.json is broken — [#6699](https://github.com/anthropics/claude-code/issues/6699). Use hooks instead.
