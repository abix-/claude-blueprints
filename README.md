# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings (copy to ~/.claude/)
```

## Skills

| Skill | Description |
|-------|-------------|
| [automation-code](skills/automation-code.md) | Ansible and PowerShell development standards |

## Usage

### Claude Code Setup

Tell Claude:

> "Clone https://github.com/abix-/claude-blueprints and help me set up my Claude config from it"

### Applying Config Files

Before copying `CLAUDE.md` or `settings.json` to `~/.claude/`, check if the user already has these files:
- **Already matches** — do nothing
- **Differs** — ask whether to merge or overwrite

### Claude Web

Upload skill files directly via Settings → Capabilities → Skills.

## Future Folders

Add when you have real content: `hooks/`, `commands/`, `agents/`, `mcp/`
