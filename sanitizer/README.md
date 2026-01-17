# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## How It Works

**Working tree is ALWAYS fake.** Real values never exist in the working directory during a Claude session.

```
WORKING TREE (fake)                 SEALED EXEC (temp)                 RENDERED (on exit)
┌─────────────────────┐             ┌─────────────────────┐           ┌─────────────────────┐
│ server: fake.test   │──command───>│ server: real.corp   │           │ server: real.corp   │
│ ip: 11.22.33.44     │             │ ip: 192.168.1.100   │           │ ip: 192.168.1.100   │
└─────────────────────┘             │ (runs, then deleted)│           └─────────────────────┘
        ^                           └─────────────────────┘                    ^
        │                                     │                                │
   Claude sees                         sanitized output                  you use this
```

### Hook Lifecycle

**1. SessionStart → Sanitize working tree**
- Scans all text files in project
- Discovers IPs and hostnames matching configured patterns
- Generates fake values: IPs → `11.x.x.x`, hostnames → `host-xxxxx.example.test`
- Saves discovered mappings to `autoMappings` in sanitizer.json
- Replaces all real values with fake values in working tree
- Claude only ever sees the sanitized version

**2. PreToolUse (Bash) → Route commands**

Commands are classified into three categories:

| Category | Examples | Behavior |
|----------|----------|----------|
| **Blocked** | Access to `sanitizer.json`, `rendered/` | Denied with error |
| **Passthrough** | `git`, `ls`, `cat`, `grep`, `mkdir` | Run directly (files already sanitized) |
| **Sealed** | `ansible-playbook`, `npm run`, `python` | Routed through SealedExec |

**3. SealedExec → Isolated execution with real values**

For commands that need real credentials/hosts to execute:

1. Creates temp directory (`%TEMP%\claude-sealed-<guid>`)
2. Copies entire working tree to temp
3. Renders fake→real in the temp copy
4. Executes command inside temp directory
5. Captures stdout/stderr
6. **Deletes temp directory** (always, even on error)
7. Sanitizes output (real→fake) before returning to Claude

Real values exist only in the ephemeral temp directory during execution.

**4. SessionStop → Render for deployment**
- Copies working tree to `renderPath` (default: `~/.claude/rendered/{project}/`)
- Renders fake→real in the output copy
- You deploy/run from the rendered directory

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
