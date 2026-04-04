# claude-blueprints

**Status: active -- skills and hooks evolve frequently**

Personal Claude configuration shared across Claude web and Claude Code instances.

## Structure

```
claude-blueprints/
  skills/             # All skills (directory format: skills/<name>/SKILL.md)
  hooks/              # General-purpose hooks (skill injection, etc.)
  scripts/            # Supporting scripts referenced by skills
  sanitizer/          # Credential sanitization system
  CLAUDE.md           # Global context (copy to ~/.claude/)
  settings.json       # Global settings with hooks (copy to ~/.claude/)
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
- IPs -> `111.x.x.x` sanitized range
- Hostnames matching patterns (e.g., `*.domain.local`) -> `host-xxxx.example.test`
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

All skills use directory format: `skills/<name>/SKILL.md`

#### Reference Skills (auto-loaded by Claude)

| Skill | Description |
|-------|-------------|
| [ansible](skills/ansible/) | Ansible playbook and role standards |
| [bevy](skills/bevy/) | Bevy 0.18 ECS patterns for the Endless project |
| [claude-config](skills/claude-config/) | Skills, hooks, settings, and sync workflow |
| [code](skills/code/) | Universal development standards |
| [godot](skills/godot/) | Godot 4.x game development, GDScript, NPC optimization |
| [golang](skills/golang/) | Go development standards |
| [infrastructure-troubleshooting](skills/infrastructure-troubleshooting/) | Diagnosing infrastructure problems |
| [linguistic-breakbeats-labyrinth](skills/linguistic-breakbeats-labyrinth/) | Constraint-based rhythmic text system and MUD runtime |
| [powershell](skills/powershell/) | PowerShell, VMware, and Pester standards |
| [python](skills/python/) | Python environment and usage on Windows |
| [rust](skills/rust/) | Rust development standards |
| [try-harder](skills/try-harder/) | Response calibration for accuracy and efficiency |
| [vmware-esxi-performance](skills/vmware-esxi-performance/) | ESXi storage/network performance troubleshooting |
| [vsphere-influxdb](skills/vsphere-influxdb/) | vSphere VM performance investigation via InfluxDB |
| [wgsl](skills/wgsl/) | WGSL shader standards |

#### Action Skills (user-invoked via /name)

| Skill | Description |
|-------|-------------|
| [/1note](skills/1note/) | Read and search OneNote notebooks via COM interop |
| [/benchmark](skills/benchmark/) | Run Criterion benchmarks and record results |
| [/debug](skills/debug/) | Check Rust compiler errors and runtime logs |
| [/deps](skills/deps/) | Check Rust dependencies for updates |
| [/dev](skills/dev/) | Trigger GitHub Actions dev build |
| [/dist](skills/dist/) | Build release and package for distribution |
| [/done](skills/done/) | Update docs, changelog, commit, and push |
| [/endless](skills/endless/) | Build and run Endless |
| [/entity](skills/entity/) | Inspect a Bevy entity via BRP endpoint |
| [/fix-auth](skills/fix-auth/) | Restore .claude.json from auto-backup |
| [/learn](skills/learn/) | Review conversation and update skills |
| [/load](skills/load/) | Pull repo and apply to ~/.claude |
| [/release](skills/release/) | Create GitHub release with notes from CHANGELOG |
| [/test](skills/test/) | Build, launch with --autostart, verify via BRP |

#### Hybrid Skills (user or Claude-invoked)

| Skill | Description |
|-------|-------------|
| [/kovarex](skills/kovarex/) | Brutally honest Factorio-style project review |
| [/rtfm](skills/rtfm/) | Search for existing solutions before building |

### Hooks

| Hook | Description |
|------|-------------|
| [Hook-SessionStart-Skills](hooks/Hook-SessionStart-Skills.ps1) | Injects skills at session start |

### claude-depester

Replaces whimsical spinner words ("Flibbertigibbeting") with "Thinking". Must run outside Claude (patches the binary). SessionStart hook re-applies after updates.

See: https://github.com/ominiverdi/claude-depester

## Claude Web

Upload skill files via Settings > Capabilities > Skills.
