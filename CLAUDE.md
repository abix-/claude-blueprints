# CLAUDE.md

Read and follow `~/.claude/skills/try-harder.md` for every response.

- When writing code, read `~/.claude/skills/code.md` first.
- When writing PowerShell, read `~/.claude/skills/powershell.md` first.
- When writing Golang, read `~/.claude/skills/golang.md` first.
- When writing Ansible, read `~/.claude/skills/ansible.md` first.
- When writing Rust, read `~/.claude/skills/rust.md` first.
- When writing Bevy, read `~/.claude/skills/bevy.md` first.
- When writing WGSL shaders, read `~/.claude/skills/wgsl.md` first.
- When writing GDScript or Godot, read `~/.claude/skills/godot.md` first.
- When modifying Claude config (skills, hooks, settings, CLAUDE.md), read `~/.claude/skills/claude-config.md` first.
- When diagnosing infrastructure problems, read `~/.claude/skills/infrastructure-troubleshooting.md` first.
- For ESXi performance issues, read `~/.claude/skills/vmware-esxi-performance.md` first.

Git commits: no Co-Authored-By, concise, lowercase, always push immediately.

## Token discipline
- NEVER launch agents to research what you already know. If prior agents returned results, USE them.
- NEVER launch multiple agents for the same question. One agent, one purpose.
- In plan mode, write the plan file IMMEDIATELY. Don't launch more agents — the plan is the deliverable.
- Prefer Glob/Grep/Read directly over Explore agents for targeted searches.
- Every agent costs real money. Ask: "Do I already have this information?" If yes, don't launch.
