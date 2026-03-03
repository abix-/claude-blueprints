---
name: claude-config
description: Managing Claude configuration - skills, hooks, settings, and sync workflow. Read first when modifying any Claude config.
metadata:
  version: "2.0"
  updated: "2026-03-03"
---
# Claude Config

## Skills

Commands are merged into skills. `.claude/commands/` still works but is a legacy alias. Use `skills/` for everything.

### Skill Formats
- `skills/my-skill.md` -- flat file, simple skills
- `skills/my-skill/SKILL.md` -- directory, when you have supporting files (templates, scripts, examples)

### Invocation Control

| Frontmatter | You invoke (/name) | Claude auto-invokes | Use for |
|---|---|---|---|
| (default) | Yes | Yes | General-purpose skills |
| `disable-model-invocation: true` | Yes | No | Side effects (/deploy, /commit, /load) |
| `user-invocable: false` | No | Yes | Background knowledge |

### Frontmatter
```yaml
---
name: skill-name                    # optional, defaults to directory/file name
description: When to use            # recommended
disable-model-invocation: true      # optional, user-only invocation
user-invocable: false               # optional, Claude-only invocation
allowed-tools: Read, Grep, Glob     # optional, auto-approved tools
context: fork                       # optional, run in subagent
agent: Explore                      # optional, subagent type
---
```

### Writing Effective Skills
- Write for Claude, not humans. Clear instructions > motivational framing.
- Be explicit about scope. When does this skill apply?
- Cut repetition. Say it once.
- Keep SKILL.md under 500 lines. Move detailed reference to supporting files.

### Token Bloat
| Bloat | Lean |
|-------|------|
| "You should always..." | Just state the rule |
| Headers for 2-3 lines | Skip the header |
| Paragraphs | Bullets |
| Same concept repeated | Say it once |

### String Substitutions
- `$ARGUMENTS` -- all args passed to skill
- `$ARGUMENTS[N]` or `$N` -- specific arg by index
- `${CLAUDE_SESSION_ID}` -- current session ID
- `` !`command` `` -- dynamic context injection (runs shell command, injects output)

### Versioning
- Increment version on every change
- Format: `major.minor` (major = breaking, minor = additions/fixes)
- Update date with every version change

## CLAUDE.md Format

Use `@path` imports to load files into context:
- `@skills/powershell.md` -- relative to CLAUDE.md location
- `@~/path/to/file.md` -- absolute home-relative
- Max 5 hops of recursive imports

Explicit read instructions -- "follow standards in X" doesn't trigger file reads.

**Good:** `When writing PowerShell, ALWAYS read skills/powershell.md first. NEVER write PowerShell without reading it.`
**Bad:** `Follow standards in skills/powershell.md`

ALWAYS pair ALWAYS with NEVER (and vice versa) in directives. NEVER use one without the other.

Keep lean: no headers, one skill per line, trigger + read instruction.

### Path-Scoped Rules
`.claude/rules/*.md` with `paths` frontmatter loads only when working with matching files:
```yaml
---
paths:
  - "src/api/**/*.ts"
---
```

## Repos

| Repo | Purpose | Scope |
|---|---|---|
| `claude-blueprints` | Personal skills, commands, hooks | Individual |
| `claude-blueprints-dc` | Team skills, docs, project context | Infrastructure team |

`/load` in personal repo syncs to `~/.claude/`. Team repo uses `@` imports from CLAUDE.md.

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

## Notes

`permissions.deny` in settings.json is broken -- [#6699](https://github.com/anthropics/claude-code/issues/6699). Use hooks instead.
