# Claude Code Secret Sanitizer

Prevent sensitive identifiers (server names, IPs, domains, paths) from being sent to Anthropic's servers.

## The Problem

Everything Claude sees gets sent to Anthropic: file contents, command outputs, grep results. If your code contains internal server names, IP addresses, or domain names, they get sent too.

## The Solution

**Working tree is ALWAYS fake.** Real values never exist in the working directory during a Claude session.

- IPs become `11.x.x.x`
- Hostnames become `host-xxxxx.example.test`
- Your manual mappings (server names, paths, etc.) use your chosen fake values

Real values only exist in:
1. **Sealed temp directories** during command execution (deleted immediately after)
2. **Rendered output directory** when you exit Claude (for actual use)

## Files

Only 2 config files:

| File | Purpose |
|------|---------|
| `secrets.json` | Your manual mappings (you create) |
| `auto_mappings.json` | Auto-discovered IPs/hostnames (auto-generated) |

## Architecture

```
WORKING TREE (always fake)          SEALED EXEC (temp, deleted)       RENDERED (on exit)
┌─────────────────────┐             ┌─────────────────────┐           ┌─────────────────────┐
│ server: fake.test   │──command───►│ server: real.corp   │           │ server: real.corp   │
│ ip: 11.22.33.44     │             │ ip: 192.168.1.100   │           │ ip: 192.168.1.100   │
└─────────────────────┘             │ (runs, then deleted)│           └─────────────────────┘
        ▲                           └─────────────────────┘                    ▲
        │                                     │                                │
   Claude sees                         sanitized output                  you use this
```

## Setup

### 1. Directory structure

```
%USERPROFILE%\.claude\
├── settings.json
└── sanitizer\
    ├── secrets.json          # your manual mappings
    ├── auto_mappings.json    # auto-generated
    ├── Initialize.ps1
    ├── Sanitize.ps1
    ├── SanitizeOutput.ps1
    ├── SealedExec.ps1
    ├── RunWrapper.ps1
    ├── AutoRenderReal.ps1
    └── RenderReal.ps1
```

### 2. Run Initialize.ps1

```powershell
cd $env:USERPROFILE\.claude\sanitizer
.\Initialize.ps1
```

### 3. Edit secrets.json

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
  }
}
```

IPs are auto-discovered. Only add manual mappings for non-IP secrets.

### 4. Configure settings.json

```json
{
  "permissions": {
    "deny": [
      "~/.claude/sanitizer/secrets.json",
      "~/.claude/sanitizer/auto_mappings.json",
      "~/.claude/sanitizer/ip_mappings_temp.json",
      "~/.claude/rendered/**"
    ]
  },
  "hooks": {
    "SessionStart": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/Sanitize.ps1\""
      }]
    }],
    "PreToolUse": [{
      "matcher": "Bash",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/RunWrapper.ps1\""
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "powershell.exe -ExecutionPolicy Bypass -NoProfile -File \"%USERPROFILE%/.claude/sanitizer/AutoRenderReal.ps1\""
      }]
    }]
  }
}
```

### 5. Restart Claude Code

## Usage

1. Start Claude - files get sanitized
2. Work normally - everything is fake
3. Exit Claude - real version auto-rendered to `%USERPROFILE%\.claude\rendered\{project}\`
4. Explorer opens showing the real version
5. Run/deploy from the rendered directory

### If Claude crashes

Working tree stays fake (safe). Manually render:

```powershell
.\RenderReal.ps1 -OutputDir C:\deploy\real
```

## What's Protected

| Vector | Status |
|--------|--------|
| File reads | Safe - working tree always fake |
| Grep/search | Safe - working tree always fake |
| Bash output | Safe - sealed exec + output scrubbing |
| Crash | Safe - working tree stays fake |
| Mapping files | Safe - hard deny rules |

## Scripts

| Script | Purpose |
|--------|---------|
| `Initialize.ps1` | First-time setup |
| `Sanitize.ps1` | Discovers and replaces real → fake |
| `SanitizeOutput.ps1` | Scrubs command output |
| `SealedExec.ps1` | Runs commands in isolated temp dir |
| `RunWrapper.ps1` | Hook that routes Bash to SealedExec |
| `AutoRenderReal.ps1` | Stop hook, renders real version |
| `RenderReal.ps1` | Manual render (crash recovery) |

## Passthrough Commands

These run directly without sealed execution (they don't need real values):
- `git`, `gh` (GitHub CLI)
- `npm`, `npx`, `node`
- File operations: `ls`, `cd`, `pwd`, `mkdir`, `rm`, `cp`, `mv`
- Read commands: `cat`, `head`, `tail`, `grep`, `find`

## Files Blocked from Claude

Blocked via `permissions.deny` (hard) and `RunWrapper.ps1` (bash commands):

- `~/.claude/sanitizer/secrets.json`
- `~/.claude/sanitizer/auto_mappings.json`
- `~/.claude/sanitizer/ip_mappings_temp.json`
- `~/.claude/rendered/**`
