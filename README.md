# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── hooks/            # General-purpose hooks (skill injection, etc.)
├── commands/         # Slash commands (/claude-push, /claude-pull)
├── sanitizer/        # Credential sanitization system (Windows/PowerShell)
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings with hooks (copy to ~/.claude/)
```

## Quick Setup

```bash
git clone https://github.com/abix-/claude-blueprints.git
```

Then tell Claude: *"Bootstrap my ~/.claude from claude-blueprints"*

After bootstrap, use `/claude-pull` and `/claude-push` to sync.

### Post-Bootstrap (optional)

```bash
# Initialize sanitizer (creates sanitizer.json with defaults)
~/.claude/sanitizer/Initialize.ps1

# Patch spinner words (run outside Claude)
npx claude-depester --all
```

## Components

### [Sanitizer](sanitizer/README.md)

Prevents sensitive identifiers (server names, IPs, domains) from being sent to Anthropic. Working tree stays fake; real values only exist in sealed temp directories during execution.

### Skills

| Skill | Description |
|-------|-------------|
| [ansible-powershell](skills/ansible-powershell.md) | Ansible and PowerShell development standards |
| [claude-code-config](skills/claude-code-config.md) | Claude Code settings, hooks, and permissions |
| [infrastructure-troubleshooting](skills/infrastructure-troubleshooting.md) | Systematic methodology for diagnosing infrastructure problems |
| [skill-management](skills/skill-management.md) | How to create, version, validate, and maintain Claude skills |
| [try-harder](skills/try-harder.md) | Response calibration for accuracy, efficiency, and honest self-assessment |
| [vmware-esxi-performance](skills/vmware-esxi-performance.md) | ESXi storage/network performance troubleshooting |

### Hooks

| Hook | Description |
|------|-------------|
| [Hook-SessionStart-Skills](hooks/Hook-SessionStart-Skills.ps1) | Injects skills at session start |

### Commands

| Command | Description |
|---------|-------------|
| [/claude-pull](commands/claude-pull.md) | Pull repo and apply to ~/.claude |
| [/claude-push](commands/claude-push.md) | Sync ~/.claude to repo and push |

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting") with "Thinking". Must run outside Claude (patches the binary). SessionStart hook re-applies after updates.

See: https://github.com/ominiverdi/claude-depester

## Claude Web

Upload skill files via Settings → Capabilities → Skills.
