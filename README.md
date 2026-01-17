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

Add this to `~/.claude/CLAUDE.md`:

```markdown
## My Blueprints
My Claude configuration is at https://github.com/abix-/claude-blueprints

When I ask for automation code help, fetch and follow the skill at:
https://raw.githubusercontent.com/abix-/claude-blueprints/main/skills/automation-code.md
```

Or tell Claude in conversation:

> "Clone https://github.com/abix-/claude-blueprints and help me set up my Claude config from it"

### Applying Config Files

Before copying `CLAUDE.md` or `settings.json` to `~/.claude/`, check if the user already has these files:
- **Already matches** — do nothing
- **Differs** — ask whether to merge or overwrite

### Claude Web

Copy skill content into project custom instructions.

## Future Folders

Add when you have real content: `hooks/`, `commands/`, `agents/`, `mcp/`
