# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── sanitizer/        # Credential sanitization system (Windows/PowerShell)
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings with hooks (copy to ~/.claude/)
```

## Dependencies

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting", "Discombobulating") with standard "Thinking" text.

**Run outside of Claude Code** (patches the binary, can't patch itself while running):

```powershell
npx claude-depester --all
```

The `settings.json` SessionStart hook runs `npx claude-depester --all --silent` to re-apply after updates.

See: https://github.com/ominiverdi/claude-depester

## Skills

| Skill | Description |
|-------|-------------|
| [automation-code](skills/automation-code.md) | Ansible and PowerShell development standards |

## Sanitizer

The sanitizer system prevents Claude from seeing real credentials, IPs, and hostnames while still allowing code execution. It works by:

1. **On session start** — Replaces real values with fake ones in your working directory
2. **During execution** — Runs commands in a sealed temp environment with real values restored
3. **On session end** — Auto-renders a copy with real values to `~/.claude/rendered/`

Your working directory always contains fake values, so it's safe for Claude to read.

### Sanitizer Scripts

| Script | Purpose |
|--------|---------|
| `Sanitize.ps1` | Replaces real→fake values on session start |
| `SealedExec.ps1` | Executes commands in isolated temp environment with real values |
| `RunWrapper.ps1` | Hook that routes Bash commands through SealedExec |
| `AutoRenderReal.ps1` | Stop hook that renders real version on exit |
| `RenderReal.ps1` | Manual render to custom location |
| `SanitizeOutput.ps1` | Sanitizes command output |
| `Initialize.ps1` | One-time setup, creates template files |

### Passthrough Commands

These commands run directly without sealed execution (they don't need real values):
- `git`, `gh` (GitHub CLI)
- File operations: `ls`, `cd`, `pwd`, `mkdir`, `rm`, `cp`, `mv`
- Read commands: `cat`, `head`, `tail`, `grep`, `find`

## Setup

### New Machine Setup

```powershell
# 1. Clone the repo
git clone https://github.com/abix-/claude-blueprints.git

# 2. Copy config files
Copy-Item claude-blueprints/CLAUDE.md ~/.claude/
Copy-Item claude-blueprints/settings.json ~/.claude/

# 3. Copy sanitizer scripts
Copy-Item -Recurse claude-blueprints/sanitizer ~/.claude/

# 4. Initialize (creates secrets.json template)
~/.claude/sanitizer/Initialize.ps1

# 5. Edit secrets.json with your real->fake mappings
notepad ~/.claude/sanitizer/secrets.json

# 6. Restart Claude Code
```

### secrets.json Format

```json
{
  "mappings": {
    "real-server.internal.corp": "fake-server.example.test",
    "192.168.1.100": "11.22.33.44",
    "my-api-key": "FAKE_API_KEY"
  },
  "patterns": {
    "ipv4": true,
    "hostnames": ["\\.internal\\.corp$", "\\.local$"]
  }
}
```

- **mappings** — Explicit real→fake replacements
- **patterns.ipv4** — Auto-discover and replace private IPs
- **patterns.hostnames** — Regex patterns for hostnames to auto-discover

### Claude Code Quick Setup

Tell Claude:

> "Clone https://github.com/abix-/claude-blueprints and help me set up my Claude config from it"

### Claude Web

Upload skill files directly via Settings → Capabilities → Skills.

## Files Never Read by Claude

These are blocked via `CLAUDE.md` instructions and `RunWrapper.ps1`:

- `~/.claude/sanitizer/secrets.json` — Contains real mappings
- `~/.claude/sanitizer/auto_mappings.json` — Auto-discovered mappings
- `~/.claude/rendered/` — Contains real values
- `%TEMP%/claude-sealed-*` — Temporary execution directories
