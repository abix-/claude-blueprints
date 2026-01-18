# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── hooks/            # General-purpose hooks (skill injection, etc.)
├── commands/         # Slash commands (/claude-pull, /learn, etc.)
├── sanitizer/        # Credential sanitization system
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings with hooks (copy to ~/.claude/)
```

## Quick Setup

```bash
git clone https://github.com/abix-/claude-blueprints.git
```

Then tell Claude: *"Bootstrap my ~/.claude from claude-blueprints"*

After bootstrap, edit repo directly, commit, push, then `/claude-pull` to apply locally.

## Components

### Sanitizer

Prevents sensitive identifiers (server names, IPs, domains) from being sent to Anthropic. Working tree stays fake; real values only exist in sealed temp directories during execution.

See [sanitizer](sanitizer/README.md) for setup and usage.

### Skills

| Skill | Description |
|-------|-------------|
| [claude-config](skills/claude-config.md) | Skills, hooks, settings, and sync workflow |
| [code](skills/code.md) | Development standards (Ansible, PowerShell, Golang) |
| [godot](skills/godot.md) | Godot 4.x game development, GDScript, NPC optimization |
| [infrastructure-troubleshooting](skills/infrastructure-troubleshooting.md) | Diagnosing infrastructure problems |
| [try-harder](skills/try-harder.md) | Response calibration for accuracy and efficiency |
| [vmware-esxi-performance](skills/vmware-esxi-performance.md) | ESXi storage/network performance troubleshooting |

### Hooks

| Hook | Description |
|------|-------------|
| [Hook-SessionStart-Skills](hooks/Hook-SessionStart-Skills.ps1) | Injects skills at session start |

### Commands

| Command | Description |
|---------|-------------|
| [/claude-pull](commands/claude-pull.md) | Pull repo and apply to ~/.claude |
| [/learn](commands/learn.md) | Review conversation and update skills with learnings |
| [/rtfm](commands/rtfm.md) | Search for existing solutions before building |

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting") with "Thinking". Must run outside Claude (patches the binary). SessionStart hook re-applies after updates.

See: https://github.com/ominiverdi/claude-depester

## Claude Web

Upload skill files via Settings → Capabilities → Skills.
