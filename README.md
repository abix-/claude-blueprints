# claude-blueprints

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
├── skills/           # Reusable prompts and standards
├── hooks/            # General-purpose hooks (skill injection, etc.)
├── commands/         # Slash commands (/load, /learn, etc.)
├── sanitizer/        # Credential sanitization system
├── CLAUDE.md         # Global context (copy to ~/.claude/)
└── settings.json     # Global settings with hooks (copy to ~/.claude/)
```

## Quick Setup

```bash
git clone https://github.com/abix-/claude-blueprints.git
```

Then tell Claude: *"Bootstrap my ~/.claude from claude-blueprints"*

After bootstrap, edit repo directly, commit, push, then `/load` to apply locally.

## Components

### Sanitizer

Go binary that prevents infrastructure details from reaching Anthropic's servers.

**What gets replaced:**
- IPs → `111.x.x.x` sanitized range
- Hostnames matching patterns (e.g., `*.domain.local`) → `host-xxxx.example.test`
- Manual mappings you define (server names, paths, project names)

**How it works:**
- Hooks run automatically via `settings.json` (SessionStart, PreToolUse, Stop)
- On file read/edit: sanitizes content before Claude sees it
- Unsanitized values stored in `~/.claude/unsanitized/{project}/` (never sent to API)
- Idempotent: re-sanitizes if you modify files mid-session

| Sent to Anthropic | Stays local |
|-------------------|-------------|
| Sanitized values only | Unsanitized values + mappings |

See [sanitizer/README.md](sanitizer/README.md) for setup.

### Skills

| Skill | Description |
|-------|-------------|
| [ansible](skills/ansible.md) | Ansible playbook and role standards |
| [bevy](skills/bevy.md) | Bevy 0.18 ECS patterns for the Endless project |
| [claude-config](skills/claude-config.md) | Skills, hooks, settings, and sync workflow |
| [code](skills/code.md) | Universal development standards |
| [godot](skills/godot.md) | Godot 4.x game development, GDScript, NPC optimization |
| [golang](skills/golang.md) | Go development standards |
| [infrastructure-troubleshooting](skills/infrastructure-troubleshooting.md) | Diagnosing infrastructure problems |
| [powershell](skills/powershell.md) | PowerShell, VMware, and Pester standards |
| [rust](skills/rust.md) | Rust development standards |
| [try-harder](skills/try-harder.md) | Response calibration for accuracy and efficiency |
| [vmware-esxi-performance](skills/vmware-esxi-performance.md) | ESXi storage/network performance troubleshooting |
| [wgsl](skills/wgsl.md) | WGSL shader standards |

### Hooks

| Hook | Description |
|------|-------------|
| [Hook-SessionStart-Skills](hooks/Hook-SessionStart-Skills.ps1) | Injects skills at session start |

### Commands

| Command | Description |
|---------|-------------|
| [/debug](commands/debug.md) | Check Rust compiler errors and runtime logs |
| [/done](commands/done.md) | Update docs, changelog, commit, and push |
| [/endless](commands/endless.md) | Build and run Endless |
| [/learn](commands/learn.md) | Review conversation and update skills with learnings |
| [/load](commands/load.md) | Pull repo and apply to ~/.claude |
| [/rtfm](commands/rtfm.md) | Search for existing solutions before building |
| [/test](commands/test.md) | Clean build and run Endless |

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting") with "Thinking". Must run outside Claude (patches the binary). SessionStart hook re-applies after updates.

See: https://github.com/ominiverdi/claude-depester

## Claude Web

Upload skill files via Settings → Capabilities → Skills.
