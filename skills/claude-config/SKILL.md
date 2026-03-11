---
name: claude-config
description: Managing Claude configuration - skills, docs, plugins, CLAUDE.md, and hooks. Read first when modifying any Claude config.
user-invocable: false
version: "2.6"
---
# Claude Config

## Repo Structure

```
<repo>/
  CLAUDE.md              # rules and @ imports
  skills/                # SKILL.md directories (auto-discovered by plugin)
  docs/                  # reference material (@ imported from CLAUDE.md)
  memory/                # team/project context
  .claude-plugin/        # plugin marketplace config
```

### skills/ vs docs/

- `skills/` -- instructions that tell Claude *how to do something*: workflows, standards, troubleshooting steps, actions
- `docs/` -- reference material Claude reads for context: architecture docs, incident findings, migration plans, lookup tables

If it has no workflow or actionable instructions for Claude, it's a doc.

## Skills

### File Structure

Every skill is a directory with `SKILL.md`:

```
skills/
  my-skill/
    SKILL.md             # required -- instructions
    reference.md         # optional -- loaded on demand
    scripts/             # optional -- Claude can execute
```

Never use flat `.md` files in `skills/`. Always `skills/<name>/SKILL.md`.

### Frontmatter

All fields optional. Only `description` is recommended.

```yaml
---
name: skill-name                    # slash command name. defaults to directory name
description: When to use            # Claude uses this to decide when to load
version: "X.Y"                      # increment on every change (major.minor)
argument-hint: "[issue-number]"     # hint shown during autocomplete for expected args
disable-model-invocation: true      # only user can invoke via /name
user-invocable: false               # hidden from / menu, only Claude loads it
allowed-tools: Read, Grep, Glob     # tools allowed without asking permission
model: sonnet                       # model override when skill is active
context: fork                       # run in isolated subagent
agent: Explore                      # subagent type when context: fork
hooks:                              # hooks scoped to this skill's lifecycle
---
```

### Skill Types

| Type | Frontmatter | Who invokes | Examples |
|------|-------------|-------------|----------|
| **Action** | `disable-model-invocation: true` | User only, via `/name` | `/load`, `/deploy` |
| **Reference** | `user-invocable: false` | Claude only, when relevant | `code`, `powershell`, `ansible` |
| **Hybrid** | (defaults) | Both user and Claude | `infra-troubleshooting`, `rtfm`, `learn` |

- **Action**: description hidden from Claude's context. Claude can't trigger it.
- **Reference**: description always in context. Hidden from `/` menu.
- **Hybrid**: description in context and in `/` menu. Default behavior.

### Writing Skills

- Write for Claude, not humans
- Be explicit about scope -- when does this skill apply?
- Say it once. Cut repetition.
- Keep SKILL.md under 500 lines. Move reference material to separate files.

| Bloat | Lean |
|-------|------|
| "You should always..." | Just state the rule |
| Headers for 2-3 lines | Skip the header |
| Paragraphs | Bullets |
| Same concept repeated | Say it once |

### Versioning

- Increment on every change
- Format: `major.minor` (major = breaking, minor = additions/fixes)
- ALWAYS bump `version` in `.claude-plugin/marketplace.json` when adding or changing skills. NEVER skip this -- plugin cache won't update without it.

### Parameters and Substitutions

Skills accept arguments when invoked: `/fix-issue 123` passes `123` as arguments.

| Variable | What it does |
|----------|-------------|
| `$ARGUMENTS` | All arguments as a single string |
| `$ARGUMENTS[0]` or `$0` | First argument (0-based index) |
| `$ARGUMENTS[1]` or `$1` | Second argument |
| `${CLAUDE_SESSION_ID}` | Current session UUID |

If `$ARGUMENTS` is not in the content, Claude Code appends `ARGUMENTS: <value>` to the end automatically.

```yaml
---
name: fix-issue
description: Fix a GitHub issue
argument-hint: "[issue-number]"
disable-model-invocation: true
---
Fix GitHub issue $ARGUMENTS following our coding standards.
```

Positional example: `/migrate-component SearchBar React Vue`

```yaml
---
name: migrate-component
description: Migrate a component between frameworks
argument-hint: "[component] [from] [to]"
---
Migrate the $0 component from $1 to $2.
```

### Dynamic Context Injection

`` !`command` `` runs a shell command before the skill content is sent to Claude. Output replaces the placeholder.

```yaml
---
name: pr-summary
description: Summarize a pull request
context: fork
agent: Explore
---
PR diff: !`gh pr diff`
PR comments: !`gh pr view --comments`

Summarize this pull request.
```

Commands execute during preprocessing -- Claude only sees the output, not the commands.

### Subagent Execution

`context: fork` runs the skill in an isolated subagent. The skill content becomes the subagent's task. No access to conversation history.

- `agent: Explore` -- read-only codebase exploration
- `agent: Plan` -- architecture and design
- `agent: general-purpose` -- full tool access (default if omitted)
- `agent: <custom>` -- any agent defined in `.claude/agents/`

Only use `context: fork` for skills with explicit tasks. Reference-only skills (guidelines, conventions) should run inline.

## CLAUDE.md

### @ Imports

`@` inlines another file -- Claude reads the target as if it were pasted in place.

```markdown
@skills/code/SKILL.md
@docs/some-reference-doc.md
@memory/team.md
```

- Relative paths resolve from the file containing the `@`
- `@/c/code/path` uses absolute path
- Skills with `user-invocable: false` or default frontmatter are auto-discovered by description -- no `@` import needed
- Docs always need `@` imports (they're not auto-discovered)

### Directives

ALWAYS pair ALWAYS with NEVER. NEVER use one without the other.

Keep lean: one rule per line, no headers for short sections.

## Plugins

Team skills are distributed via plugins. No clone required -- marketplace can be a git URL.

### What plugins load

| Loaded | Not loaded |
|--------|-----------|
| `skills/` | `CLAUDE.md` |
| `commands/` | `memory/` |
| `agents/` | `docs/` |
| `hooks/hooks.json` | |
| `.mcp.json` | |

Plugins do NOT load CLAUDE.md or memory files. Use a `SessionStart` hook to inject that content (see Hooks section below).

### Install

```
/plugin marketplace add <git-url>
/plugin install <plugin-name>
```

### Cache and Versioning

Plugins are **copied** to `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/`. Claude loads from cache, not the live repo.

**Critical**: if the `version` in `marketplace.json` hasn't changed, update is a no-op. **Always bump the version** when changing skills, hooks, or docs.

To force-refresh: `rm -rf ~/.claude/plugins/cache/<plugin>/` then `/plugin marketplace update`.

### Auto-Update

Enable via `/plugin` > Marketplaces > select marketplace > Enable auto-update. Per-user toggle, no way to default it on.

## Memory

Two memory systems:

### Team memory (`memory/` in repo)

Shared team context loaded via `@` imports in CLAUDE.md. Checked into git.

- `memory/team.md` -- team stack, responsibilities, priorities, custom tooling
- Add new files for stable team knowledge that all members need

Only put things here that are:
- Stable across sessions (not in-progress work)
- Relevant to the whole team (not personal preferences)
- Factual (verified, not speculative)

### Personal memory (`~/.claude/projects/<project>/memory/`)

Claude's auto-memory per project. Persists across conversations for one user. Claude manages this automatically -- it writes learnings, patterns, and project state here.

- `MEMORY.md` is always loaded into context (keep under 200 lines)
- Create topic files for detailed notes, link from MEMORY.md
- Don't duplicate what's in team memory or CLAUDE.md

## Hooks

| Event | When | Use Case |
|-------|------|----------|
| `SessionStart` | Session begins | Inject context, set env vars |
| `PreToolUse` | Before tool executes | Block/modify tool calls |
| `PostToolUse` | After tool executes | Sanitize output |
| `Stop` | Session ends | Cleanup |

### Plugin hooks

Plugin hooks live in `hooks/hooks.json` at the plugin root. They ship with the plugin and run when the plugin is enabled. Use `${CLAUDE_PLUGIN_ROOT}` for paths.

### SessionStart context injection

`SessionStart` and `UserPromptSubmit` are the only hooks whose stdout is added as context Claude can see. Use this to inject always-on team rules from the plugin cache -- replaces the need for `@` imports and local clones.

```json
{
  "hooks": {
    "SessionStart": [{
      "matcher": "startup",
      "hooks": [{
        "type": "command",
        "command": "python ${CLAUDE_PLUGIN_ROOT}/scripts/session-start.py"
      }]
    }]
  }
}
```

Script outputs JSON with `additionalContext`:

```python
import json, os
root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
parts = []
for path in [os.path.join(root, "CLAUDE.md"), os.path.join(root, "memory", "team.md")]:
    try:
        with open(path, encoding="utf-8") as f:
            parts.append(f.read())
    except OSError:
        pass
output = {"hookSpecificOutput": {"hookEventName": "SessionStart", "additionalContext": "\n\n".join(parts)}}
print(json.dumps(output))
```

### Config (settings.json)

```json
"PreToolUse": [{
  "matcher": "Read|Edit|Write",
  "hooks": [{ "type": "command", "command": "powershell.exe -NoProfile -File hook.ps1" }]
}]
```

Matcher: `"Bash"` (single), `"Read|Edit|Write"` (multiple), `""` (all)

### Blocking

Return JSON with `"permissionDecision": "deny"` or exit code 2.

### Input

JSON on stdin with `hook_event_name` and `tool_input`.

## Status Line

Custom bash script that renders session info below the prompt. File: `~/.claude/statusline.sh`

### Windows: DISABLED -- zombie process bug

On Windows, statusline spawns `bash.exe` (Git Bash) per update. These are never killed when the parent exits -- no signal propagation through MSYS2. Produces ~1 orphan/minute. 88 zombies observed in under 2 hours.

Tracked in [#18405](https://github.com/anthropics/claude-code/issues/18405). Inline `jq` command still spawns bash.exe, so no user-side fix exists. Statusline is disabled in settings.json until resolved.

### Setup

Create `~/.claude/statusline.sh` (must be executable). Claude Code pipes JSON to stdin on every render.

### Available JSON Fields

```
session_id                                      # UUID for current session
transcript_path                                 # path to .jsonl transcript file
cwd                                             # current working directory
model.id                                        # full model ID (us.anthropic.claude-opus-4-6-v1)
model.display_name                              # display name (same as id usually)
workspace.current_dir                           # same as cwd
workspace.project_dir                           # project root (where CLAUDE.md lives)
workspace.added_dirs                            # array of extra dirs added via /add-dir
version                                         # Claude Code version (e.g. 2.1.63)
output_style.name                               # "default" or "streamlined" (/compact toggle)
cost.total_cost_usd                             # session cost in USD
cost.total_duration_ms                          # total wall clock time
cost.total_api_duration_ms                      # time waiting on API only
cost.total_lines_added                          # lines of code added this session
cost.total_lines_removed                        # lines of code removed this session
context_window.total_input_tokens               # cumulative input tokens (all turns)
context_window.total_output_tokens              # cumulative output tokens (all turns)
context_window.context_window_size              # max context window (e.g. 200000)
context_window.used_percentage                  # percent of context used
context_window.remaining_percentage             # percent of context remaining
context_window.current_usage.input_tokens       # input tokens on last API call
context_window.current_usage.output_tokens      # output tokens on last API call
context_window.current_usage.cache_creation_input_tokens  # tokens written to cache (last call)
context_window.current_usage.cache_read_input_tokens      # tokens served from cache (last call, ~90% cheaper)
exceeds_200k_tokens                             # true if context exceeds 200k
```

### Gotchas

- `printf '%b'` interprets backslash escapes -- Windows paths get mangled (`\Users` -> bell + `Users`). Escape with `${VAR//\\/\\\\}` before printing.
- `cwd`, `workspace.current_dir`, and `workspace.project_dir` are usually identical. Only differ if you `cd` or `/add-dir`.
- `printf '%b'` does NOT eat `%` signs (unlike `printf` format strings). Single `%` works.

### Example (single-line, compact)

```bash
#!/usr/bin/env bash
input=$(cat)

CWD=$(echo "$input" | jq -r '.cwd // "?"')
CWD="${CWD//\\/\\\\}"
USED_PCT=$(echo "$input" | jq -r '.context_window.used_percentage // 0')
TOTAL_IN=$(echo "$input" | jq -r '.context_window.total_input_tokens // 0')
TOTAL_OUT=$(echo "$input" | jq -r '.context_window.total_output_tokens // 0')
TURN_IN=$(echo "$input" | jq -r '.context_window.current_usage.input_tokens // 0')
TURN_OUT=$(echo "$input" | jq -r '.context_window.current_usage.output_tokens // 0')
CACHE_W=$(echo "$input" | jq -r '.context_window.current_usage.cache_creation_input_tokens // 0')
CACHE_R=$(echo "$input" | jq -r '.context_window.current_usage.cache_read_input_tokens // 0')
TOTAL_COST=$(echo "$input" | jq -r '.cost.total_cost_usd // 0')
TOTAL_DURATION_MS=$(echo "$input" | jq -r '.cost.total_duration_ms // 0')
API_DURATION_MS=$(echo "$input" | jq -r '.cost.total_api_duration_ms // 0')
LINES_ADDED=$(echo "$input" | jq -r '.cost.total_lines_added // 0')
LINES_REMOVED=$(echo "$input" | jq -r '.cost.total_lines_removed // 0')

G='\033[32m'; Y='\033[33m'; R='\033[31m'; C='\033[36m'; D='\033[2m'; N='\033[0m'
if [ "$USED_PCT" -ge 90 ]; then BC="$R"; elif [ "$USED_PCT" -ge 70 ]; then BC="$Y"; else BC="$G"; fi
TM=$((TOTAL_DURATION_MS / 60000)); TS=$(((TOTAL_DURATION_MS % 60000) / 1000))
AM=$((API_DURATION_MS / 60000)); AS=$(((API_DURATION_MS % 60000) / 1000))
COST=$(printf '$%.4f' "$TOTAL_COST")

printf '%b\n' "${C}${CWD}${N} ${BC}${USED_PCT}% Context${N} ${D}|${N} session_in:${TOTAL_IN} session_out:${TOTAL_OUT} ${D}|${N} last_turn_in:${TURN_IN} last_turn_out:${TURN_OUT} cached_new:${CACHE_W} cached_reused:${CACHE_R} ${D}|${N} ${Y}${COST}${N} ${D}${TM}m${TS}s api:${AM}m${AS}s${N} ${G}+${LINES_ADDED}${N}/${R}-${LINES_REMOVED}${N}"
```

Output: `C:\code\test 34% Context | session_in:3344 session_out:18428 | last_turn_in:1 last_turn_out:125 cached_new:382 cached_reused:65746 | $1.9855 13m55s api:5m12s +190/-181`

### Debug

To dump raw JSON and see all available fields, temporarily replace the script:

```bash
#!/usr/bin/env bash
input=$(cat)
echo "$input" | jq '.' 2>/dev/null || echo "$input"
```

## Notes

`permissions.deny` in settings.json is broken -- [#6699](https://github.com/anthropics/claude-code/issues/6699). Use hooks instead.
