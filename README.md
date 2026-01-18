# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── hooks/            # General-purpose hooks (skill injection, etc.)
├── sanitizer/        # Credential sanitization system (Windows/PowerShell)
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings with hooks (copy to ~/.claude/)
```

## Quick Setup

```powershell
# 1. Clone
git clone https://github.com/abix-/claude-blueprints.git

# 2. Copy config files
Copy-Item claude-blueprints/CLAUDE.md ~/.claude/
Copy-Item claude-blueprints/settings.json ~/.claude/
Copy-Item -Recurse claude-blueprints/skills ~/.claude/
Copy-Item -Recurse claude-blueprints/hooks ~/.claude/
Copy-Item -Recurse claude-blueprints/sanitizer ~/.claude/

# 3. Initialize sanitizer and edit secrets
~/.claude/sanitizer/Initialize.ps1
notepad ~/.claude/sanitizer/secrets.json

# 4. Patch spinner words (run outside Claude)
npx claude-depester --all

# 5. Start Claude Code
```

Or tell Claude: *"Clone https://github.com/abix-/claude-blueprints and help me set up my Claude config from it"*

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

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting") with "Thinking". Must run outside Claude (patches the binary). SessionStart hook re-applies after updates.

See: https://github.com/ominiverdi/claude-depester

## Claude Web

Upload skill files via Settings → Capabilities → Skills.
