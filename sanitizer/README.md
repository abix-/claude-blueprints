# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## How It Works

**Working tree is ALWAYS fake.** Real values never exist in the working directory.

```
SESSION START
─────────────────────────────────────────────────────────────────────────────
Your files get sanitized. Claude only sees fake values.

    192.168.1.100  ──►  11.22.33.44
    prod.internal  ──►  host-abc.test


DURING SESSION (when commands need real values)
─────────────────────────────────────────────────────────────────────────────
Example: Claude runs "ansible-playbook site.yml"

    ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
    │ WORKING TREE │ copy │   TEMP DIR   │ run  │    OUTPUT    │
    │    (fake)    │─────►│    (real)    │─────►│    (fake)    │
    │              │      │              │      │              │
    │ 11.22.33.44  │      │192.168.1.100 │      │ 11.22.33.44  │
    │host-abc.test │      │prod.internal │      │host-abc.test │
    └──────────────┘      └──────┬───────┘      └──────────────┘
                                 │                     ▲
           Claude sees this      ▼ deleted        Claude sees this
                            immediately


SESSION END
─────────────────────────────────────────────────────────────────────────────
Real version rendered for deployment.

    Working Tree          Rendered Output
    (still fake)    ──►   (real values)
                          └─► you deploy from here


COMMAND ROUTING
─────────────────────────────────────────────────────────────────────────────
    BLOCKED              PASSTHROUGH           SEALED
    sanitizer.json       git, ls, cat          everything else
    rendered/**          grep, mkdir           (ansible, npm, python...)
         │                    │                       │
         ▼                    ▼                       ▼
      denied            run directly           temp dir execution
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
