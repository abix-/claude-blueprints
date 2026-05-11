# claude-blueprints

**Status: active. Skills and hooks evolve frequently**

Personal Claude configuration shared across Claude Code instances.

## Structure

```
claude-blueprints/
  skills/             # All skills (directory format: skills/<name>/SKILL.md)
  hooks/              # General-purpose hooks (skill injection, etc.)
  scripts/            # Supporting scripts referenced by skills
  extensions/         # Browser extensions (Hush)
  wezterm/            # WezTerm config
  archive/            # Unfinished or retired work (see archive/*/STATUS.md)
  CLAUDE.md           # Global context (copy to ~/.claude/)
  settings.json       # Global settings with hooks (copy to ~/.claude/)
```

## Quick Setup

```bash
git clone https://github.com/abix-/claude-blueprints.git
```

Then tell Claude: *"Bootstrap my ~/.claude from claude-blueprints"*

After bootstrap, edit repo directly, commit, push, then `/load` to apply locally.

## Skills

All skills use directory format: `skills/<name>/SKILL.md`.

### Coding

| Skill | Description |
|-------|-------------|
| [code](skills/code/) | Universal development standards |
| [rust](skills/rust/) | Rust standards, concurrency, unsafe/FFI, async, workspaces (default language for new work) |
| [bevy](skills/bevy/) | Bevy 0.18 ECS patterns |
| [wgsl](skills/wgsl/) | WGSL shader patterns for Bevy compute and instanced rendering |
| [csharp](skills/csharp/) | C# / .NET / Unity mods / NuGet |
| [godot](skills/godot/) | Godot 4.x, GDScript, NPC optimization |
| [golang](skills/golang/) | Go development standards |
| [python](skills/python/) | Python environment and usage on Windows |
| [powershell](skills/powershell/) | PowerShell, VMware, and Pester standards |
| [bash](skills/bash/) | Bash scripting standards for shell scripts and CI |
| [typescript](skills/typescript/) | TypeScript / JavaScript standards (browser, Node, extensions) |
| [ansible](skills/ansible/) | Ansible playbook and role standards |
| [jinja](skills/jinja/) | Jinja2 templating (Ansible, AWX) |
| [yaml](skills/yaml/) | YAML standards (configs, Ansible, k8s, Actions) |
| [ahk](skills/ahk/) | AutoHotkey v2 Windows automation and game macros |
| [assembly](skills/assembly/) | x86-64 disassembly, RVAs, struct/vtable layout, hook trampolines |

### Infrastructure and Ops

| Skill | Description |
|-------|-------------|
| [infrastructure-troubleshooting](skills/infrastructure-troubleshooting/) | Starting framework for diagnosing infra problems |
| [vmware-esxi-performance](skills/vmware-esxi-performance/) | ESXi storage/network performance troubleshooting |
| [vsphere-influxdb](skills/vsphere-influxdb/) | vSphere VM performance via InfluxDB MCP server |
| [k3s](skills/k3s/) | k3s cluster on WSL2 Ubuntu 24.04 |
| [k3sc](skills/k3sc/) | k3sc Go binary. Claude agent operator, CLI, TUI |
| [wsl](skills/wsl/) | WSL2 Ubuntu 24.04 management on Windows 10 |
| [debloat](skills/debloat/) | Strip Windows of junk services, AppX, telemetry |

### Endless (Bevy game)

| Skill | Description |
|-------|-------------|
| [endless](skills/endless/) | Build and run Endless |
| [endless-cli](skills/endless-cli/) | BRP client for inspecting the running game |
| [/entity](skills/entity/) | Inspect a Bevy entity via endless-cli |
| [/test](skills/test/) | Build, launch with --autostart, verify via BRP |
| [/debug](skills/debug/) | Check Rust compiler errors and runtime logs |
| [/dev](skills/dev/) | Trigger GitHub Actions dev build |
| [/dist](skills/dist/) | Build release and package for distribution |
| [/release](skills/release/) | Create GitHub release with notes from CHANGELOG |
| [/deps](skills/deps/) | Check Rust dependencies for updates |
| [/benchmark](skills/benchmark/) | Run Criterion benchmarks and record results |

### Mods

| Skill | Description |
|-------|-------------|
| [ueforge](skills/ueforge/) | Base framework every UE4SS Rust mod builds on |
| [grounded2](skills/grounded2/) | Grounded 2 modding (UE5 + UE4SS, Rust) |
| [outworld-station](skills/outworld-station/) | Outworld Station modding (UE5.4 + UE4SS, Rust) |
| [schedule1](skills/schedule1/) | Schedule 1 modding (IL2CPP + MelonLoader + Harmony, C#) |
| [timberborn](skills/timberborn/) | Timberbot mod development (C# + Python) |
| [/timberbot](skills/timberbot/) | Timberborn gameplay client |
| [/timberbot-release](skills/timberbot-release/) | Release Timberbot to GitHub and Steam Workshop |

### Other Projects

| Skill | Description |
|-------|-------------|
| [abixio](skills/abixio/) | AbixIO S3-compatible erasure-coded object server |
| [hush](skills/hush/) | Firewall-style rule engine Chrome extension (Rust/WASM + Leptos) |
| [linguistic-breakbeats-labyrinth](skills/linguistic-breakbeats-labyrinth/) | Constraint-based rhythmic text system and MUD runtime |

### Workflow and Review

| Skill | Description |
|-------|-------------|
| [/issue](skills/issue/) | Create, claim, and work GitHub issues |
| [/n](skills/n/) | Auto-pick next PR/issue and start reviewing |
| [/review](skills/review/) | Review a PR or issue against hard merge gates |
| [/approve](skills/approve/) | Approve and merge a PR after review |
| [/reject](skills/reject/) | Close a failed PR, comment findings, reset |
| [/done](skills/done/) | Update docs, changelog, commit, and push |
| [/learn](skills/learn/) | Review conversation and update skills |
| [/why](skills/why/) | Trace why Claude made its previous response |

### Claude Behavior

| Skill | Description |
|-------|-------------|
| [try-harder](skills/try-harder/) | Response calibration for accuracy and efficiency |
| [/obey](skills/obey/) | Re-read CLAUDE.md and confirm full compliance |
| [/kovarex](skills/kovarex/) | Brutally honest Factorio-style project review |
| [/rtfm](skills/rtfm/) | Search for existing solutions before building |
| [/help](skills/help/) | List all slash commands grouped by workflow stage |

### Claude Config

| Skill | Description |
|-------|-------------|
| [claude-config](skills/claude-config/) | Skills, hooks, settings, and sync workflow |
| [claude-code-deep-dive](skills/claude-code-deep-dive/) | Deep reference for Claude Code internals (query loop, prompt cache, tools) |
| [docs](skills/docs/) | Build, preview, and deploy MkDocs Material sites |
| [/load](skills/load/) | Pull repo and apply to ~/.claude |
| [/fix-auth](skills/fix-auth/) | Restore .claude.json from auto-backup |

### Utilities

| Skill | Description |
|-------|-------------|
| [/1note](skills/1note/) | Read and search OneNote notebooks via COM interop |

## Hooks

| Hook | Description |
|------|-------------|
| [Hook-SessionStart-Skills](hooks/Hook-SessionStart-Skills.ps1) | Injects skills at session start |

## Scripts

| Script | Description |
|--------|-------------|
| [dehyphen.py](scripts/dehyphen.py) | Strip em-dashes and double-hyphens from prose |
| [chrome_cpu_profile.py](scripts/chrome_cpu_profile.py) | Capture CPU profiles from a running Chrome |
| [google_search.py](scripts/google_search.py) | Headless Google search helper |
| [1note.ps1](scripts/1note.ps1) | OneNote COM reader (used by `/1note`) |

## Settings Highlights

- `spinnerVerbs` replaces the whimsical thinking words with a custom verb. No external binary patcher needed.
- Permission allowlist / deny / `filePermissions` keep secret-bearing paths (`.env`, `**/*.key`, `**/credentials/**`) out of reach.
- Hooks wire up SessionStart skill injection, prompt logging, and memory updates.

## Archive

Prototyped but unfinished work lives in `archive/`. Each subdir has a `STATUS.md`.

- `archive/sanitizer/`. Go content sanitizer for hooks. Response-side desanitization and full tool-surface coverage were never finished.

## Claude Web

Upload skill files via Settings > Capabilities > Skills.
