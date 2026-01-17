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

### Claude Code

Clone this repo, then copy or symlink files to `~/.claude/`

**Important**: Before copying `CLAUDE.md` or `settings.json`, check if the user already has these files. If they do:
- **Already matches** — do nothing
- **Differs** — ask whether to merge or overwrite

### Claude Web

Copy skill content into project custom instructions.

## Future Folders

Add when you have real content: `hooks/`, `commands/`, `agents/`, `mcp/`
