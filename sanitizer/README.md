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
    │ host: 192.168.1.100 │ ──────────────► │ host: 11.22.33.44   │
    │ name: prod.internal │   (in place)    │ name: host-a1b.test │
    └─────────────────────┘                 └─────────────────────┘
                                                      ▲
    - Finds IPs matching patterns                     │
    - Finds hostnames matching patterns         Claude reads
    - Generates fake replacements               and edits this
    - Saves mappings to sanitizer.json


2. COMMAND EXECUTION - Sealed Temporary Environment
═══════════════════════════════════════════════════════════════════════════

When Claude runs a command like "powershell ./Deploy-App.ps1":

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 1: Hook intercepts command                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
          ┌────────────────────────────┼────────────────────────────┐
          ▼                            ▼                            ▼
    ┌───────────┐               ┌───────────┐               ┌───────────┐
    │  BLOCKED  │               │PASSTHROUGH│               │  SEALED   │
    │           │               │           │               │           │
    │sanitizer. │               │ git, ls   │               │powershell │
    │json, etc. │               │ cat, grep │               │ python    │
    │           │               │ mkdir, rm │               │ npm, make │
    └─────┬─────┘               └─────┬─────┘               └─────┬─────┘
          ▼                           ▼                           ▼
       denied                   run directly               continue below
                             (files already fake)

┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 2: Create temp directory                                           │
│         %TEMP%\claude-sealed-<guid>\                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 3: Copy working tree to temp, replace fake → real                  │
│                                                                         │
│     Working Tree                         Temp Directory                 │
│     ┌─────────────────┐    copy &        ┌─────────────────┐            │
│     │ 11.22.33.44     │    render        │ 192.168.1.100   │            │
│     │ host-a1b.test   │ ───────────────► │ prod.internal   │            │
│     └─────────────────┘                  └─────────────────┘            │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 4: Execute command inside temp directory                           │
│         (command runs with REAL values)                                 │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 5: Capture output                                                  │
│         "Deploying to prod.internal..."                                 │
│         "Connected to 192.168.1.100"                                    │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 6: Delete temp directory                                           │
│         (real values are GONE)                                          │
└─────────────────────────────────────────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────┐
│ STEP 7: Sanitize output, replace real → fake                            │
│         "Deploying to host-a1b.test..."   ◄── Claude sees this          │
│         "Connected to 11.22.33.44"                                      │
└─────────────────────────────────────────────────────────────────────────┘


3. SESSION END - Render for Deployment
═══════════════════════════════════════════════════════════════════════════

When you exit Claude normally:

    Working Tree                            Rendered Output
    ┌─────────────────────┐                 ┌─────────────────────┐
    │ 11.22.33.44         │    copy &       │ 192.168.1.100       │
    │ host-a1b.test       │    render       │ prod.internal       │
    │                     │ ──────────────► │                     │
    │ (stays fake)        │                 │ (ready to deploy)   │
    └─────────────────────┘                 └─────────────────────┘
                                                      ▲
                                                      │
                                                 you use this


WHERE REAL VALUES EXIST
═══════════════════════════════════════════════════════════════════════════

    Location                     Contains Real Values?
    ─────────────────────────    ─────────────────────────────────────────
    Working tree                 ✗ NO  - always fake
    Claude's view                ✗ NO  - only sees fake
    Anthropic servers            ✗ NO  - only receives fake
    Temp directory               ✓ YES - deleted immediately after use
    Rendered output              ✓ YES - for your deployment
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
  "renderPath": "~/.claude/rendered/{project}"
}
```

- `mappings`: Your manual real → fake mappings
- `autoMappings`: Auto-discovered IPs/hostnames (populated automatically)
- `patterns`: What to auto-discover (IPs and hostname patterns)
- `renderPath`: Where to render real version (default `~/.claude/rendered/{project}`)

### 3. Configure settings.json

```json
{
  "permissions": {
    "deny": [
      "~/.claude/sanitizer/sanitizer.json",
      "~/.claude/rendered/**"
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

1. Start Claude - files get sanitized
2. Work normally - everything is fake
3. Exit Claude - real version rendered to `~/.claude/rendered/{project}/`
4. Run/deploy from the rendered directory

### If Claude crashes

Working tree stays fake (safe). Manually render:

```powershell
.\RenderReal.ps1 -OutputDir C:\deploy\real
```

## Files

| File | Type | Purpose |
|------|------|---------|
| `sanitizer.json` | Config | All mappings (manual + auto) and settings |
| `Hook-SessionStart.ps1` | Hook (SessionStart) | Replaces real values with fake in working tree |
| `Hook-Bash.ps1` | Hook (PreToolUse) | Routes commands through sealed execution |
| `Hook-SessionStop.ps1` | Hook (Stop) | Renders real version on exit |
| `SealedExec.ps1` | Utility | Executes commands in isolated temp dir |
| `RenderReal.ps1` | Utility | Manual render (crash recovery) |
| `Initialize.ps1` | Utility | One-time setup |
| `Sanitizer.psm1` | Module | Shared functions used by all scripts |

## Reference

### Passthrough Commands

These run directly without sealed execution (they don't need real values):
- `git`, `gh`, `ls`, `cd`, `pwd`, `mkdir`, `rm`, `cp`, `mv`
- `cat`, `head`, `tail`, `grep`, `find`

### Files Blocked from Claude

- `~/.claude/sanitizer/sanitizer.json`
- `~/.claude/rendered/**`
