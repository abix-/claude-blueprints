# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains) from being sent to Anthropic.

## How It Works

**Working tree is ALWAYS fake.** Real values never exist in the working directory during a Claude session.

- IPs become `11.x.x.x`
- Hostnames become `host-xxxxx.example.test`
- Manual mappings use your chosen fake values

Real values only exist in:
1. **Sealed temp directories** during command execution (deleted immediately after)
2. **Rendered output** when you exit Claude (for actual use)

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

## Scripts

### Hooks (registered in settings.json)

| Script | Hook | Purpose |
|--------|------|---------|
| `Hook-SessionStart.ps1` | SessionStart | Replaces real values with fake in working tree |
| `Hook-Bash.ps1` | PreToolUse (Bash) | Routes commands through sealed execution |
| `Hook-SessionStop.ps1` | Stop | Renders real version to ~/.claude/rendered/ on exit |

### Utilities

| Script | Purpose |
|--------|---------|
| `SealedExec.ps1` | Executes commands in isolated temp dir with real values |
| `RenderReal.ps1` | Manual render (crash recovery) |
| `Initialize.ps1` | One-time setup |

### Module

| File | Purpose |
|------|---------|
| `Sanitizer.psm1` | Shared functions used by all scripts |

## Setup

### 1. Run Initialize.ps1

```powershell
cd $env:USERPROFILE\.claude\sanitizer
.\Initialize.ps1
```

### 2. Edit secrets.json

```json
{
  "mappings": {
    "real-server.internal.corp": "fake-server.example.test",
    "secret-api-key-12345": "FAKE_API_KEY"
  },
  "excludePaths": [".git", "node_modules", ".claude"],
  "excludeExtensions": [".exe", ".dll", ".png", ".jpg"],
  "patterns": {
    "ipv4": true,
    "hostnames": ["\\.internal\\.corp$", "\\.local$"]
  },
  "openExplorerOnRender": false
}
```

IPs are auto-discovered. Only add manual mappings for non-IP secrets.

### 3. Configure settings.json

```json
{
  "permissions": {
    "deny": [
      "~/.claude/sanitizer/secrets.json",
      "~/.claude/sanitizer/auto_mappings.json",
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

## Passthrough Commands

These run directly without sealed execution (they don't need real values):
- `git`, `gh`, `ls`, `cd`, `pwd`, `mkdir`, `rm`, `cp`, `mv`
- `cat`, `head`, `tail`, `grep`, `find`

## Files Blocked from Claude

- `~/.claude/sanitizer/secrets.json`
- `~/.claude/sanitizer/auto_mappings.json`
- `~/.claude/rendered/**`
