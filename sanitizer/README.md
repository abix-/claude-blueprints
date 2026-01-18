# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## How It Works

```
1. SESSION START - Sanitize Working Tree
═══════════════════════════════════════════════════════════════════════════

When Claude Code launches, BEFORE Claude sees anything:

    Your Project Files                      Working Tree
    ┌─────────────────────┐                 ┌─────────────────────┐
    │ inventory.yml       │    scan &       │ inventory.yml       │
    │ ─────────────────── │    replace      │ ─────────────────── │
    │ host: 11.139.237.229 │ ──────────────► │ host: 11.22.33.44   │
    │ name: prod.internal │   (in place)    │ name: host-a1b.test │
    └─────────────────────┘                 └─────────────────────┘
                                                      ▲
    - Finds IPs matching patterns                     │
    - Finds hostnames matching patterns         Claude reads
    - Generates fake replacements               and edits this
    - Saves mappings to sanitizer.json


2. COMMAND EXECUTION - Run in Unsanitized Directory
═══════════════════════════════════════════════════════════════════════════

When Claude runs a command like "powershell ./Deploy-App.ps1":

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 1: Hook intercepts command                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
          ┌────────────────────────────┼────────────────────────────┐
          ▼                            ▼                            ▼
    ┌───────────┐               ┌───────────┐               ┌───────────┐
    │   DENY    │               │   FAKE    │               │   REAL    │
    └─────┬─────┘               └─────┬─────┘               └─────┬─────┘
          │                           │                           │
          ▼                           ▼                           ▼

    Command tries to         Command runs in            Command runs in
    access sanitizer.json    WORKING TREE               UNSANITIZED DIR
    or unsanitized/**        (fake values)              (real values)

    Examples:                Examples:                  Examples:
    - cat sanitizer.json     - git status               - powershell script.ps1
    - ls ~/.claude/uns...    - python, npm, etc         - ansible-playbook site.yml
                             - everything else          - awx job_templates launch

    ✗ Blocked                Runs directly              Syncs changes, runs
                                                        with real values,
                                                        output sanitized
                                                               │
                                                               ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 2: Sync changes to unsanitized directory                           │
│                                                                         │
│     Working Tree                         Unsanitized Directory          │
│     ┌─────────────────┐     copy &       ┌─────────────────┐            │
│     │ 11.22.33.44     │   unsanitize     │ 11.139.237.229   │            │
│     │ host-a1b.test   │ ───────────────► │ prod.internal   │            │
│     └─────────────────┘   (changed       └─────────────────┘            │
│                            files only)                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Execute command in unsanitized directory                        │
│         (command runs with REAL values)                                 │
│                                                                         │
│         "Deploying to prod.internal..."                                 │
│         "Connected to 11.139.237.229"                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Sanitize output (real → fake)                                   │
│                                                                         │
│         "Deploying to host-a1b.test..."   ◄── Claude sees this          │
│         "Connected to 11.22.33.44"                                      │
└─────────────────────────────────────────────────────────────────────────┘


WHERE REAL VALUES EXIST
═══════════════════════════════════════════════════════════════════════════

    Location                     Contains Real Values?
    ─────────────────────────    ─────────────────────────────────────────
    Working tree                 ✗ NO  - always fake
    Claude's view                ✗ NO  - only sees fake
    Anthropic servers            ✗ NO  - only receives fake
    Unsanitized directory        ✓ YES - for command execution & deployment
```

## Setup

### 1. Run Initialize.ps1

```powershell
cd $env:USERPROFILE\.claude\sanitizer
.\Initialize.ps1
```

### 2. Edit sanitizer.json

```json
{
  "mappings": {
    "real-server.internal.corp": "fake-server.example.test"
  },
  "autoMappings": {},
  "patterns": {
    "ipv4": true,
    "hostnames": ["\\.internal\\.corp$", "\\.local$"]
  },
  "unsanitizedPath": "~/.claude/unsanitized/{project}"
}
```

- `mappings`: Your manual real → fake mappings
- `autoMappings`: Auto-discovered IPs/hostnames (populated automatically)
- `patterns`: What to auto-discover (IPs and hostname patterns)
- `unsanitizedPath`: Where to write unsanitized version (default `~/.claude/unsanitized/{project}`)

### 3. Configure settings.json

```json
{
  "permissions": {
    "deny": [
      "~/.claude/sanitizer/sanitizer.json",
      "~/.claude/unsanitized/**"
    ]
  },
  "hooks": {
    "SessionStart": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/Hook-SessionStart.ps1\""
      }]
    }],
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/Hook-Bash.ps1\""
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/Hook-SessionStop.ps1\""
      }]
    }]
  }
}
```

### 4. Restart Claude Code

## Usage

1. Start Claude - files get sanitized, unsanitized copy created
2. Work normally - Claude sees fake values, commands run with real values
3. Deploy from `~/.claude/unsanitized/{project}/`

### If Claude crashes

Working tree stays fake (safe). Unsanitized directory already has real values.

## Files

| File | Type | Purpose |
|------|------|---------|
| `sanitizer.json` | Config | All mappings (manual + auto) and settings |
| `Hook-SessionStart.ps1` | Hook (SessionStart) | Sanitizes working tree, creates unsanitized copy |
| `Hook-Bash.ps1` | Hook (PreToolUse) | Syncs changes, routes commands to unsanitized directory |
| `Hook-SessionStop.ps1` | Hook (Stop) | Final sync to unsanitized directory |
| `Unsanitize.ps1` | Utility | Manual unsanitize |
| `Initialize.ps1` | Utility | One-time setup |
| `Sanitizer.psm1` | Module | Shared functions used by all scripts |

## Reference

### Command Routing (Implicit Deny)

**REAL** - These patterns run in unsanitized directory with real values:
- `powershell` / `pwsh` - PowerShell executables
- `*.ps1` - Any PowerShell script (any path format)
- `& ...` - Call operator
- `ansible*` - ansible, ansible-playbook, ansible-vault, etc.
- `awx` - AWX CLI

**FAKE** - Everything else runs in working tree with fake values (default)

### Files Blocked from Claude

- `~/.claude/sanitizer/sanitizer.json`
- `~/.claude/unsanitized/**`
