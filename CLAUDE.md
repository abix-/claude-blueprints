# CLAUDE.md

ALWAYS read and follow `~/.claude/skills/try-harder.md`. NEVER skip it

ALWAYS read the matching skill before starting. NEVER begin work without reading it first

- code: `~/.claude/skills/code.md`
- PowerShell: `~/.claude/skills/powershell.md`
- Golang: `~/.claude/skills/golang.md`
- Ansible: `~/.claude/skills/ansible.md`
- Rust: `~/.claude/skills/rust.md`
- Bevy: `~/.claude/skills/bevy.md`
- WGSL shaders: `~/.claude/skills/wgsl.md`
- GDScript/Godot: `~/.claude/skills/godot.md`
- Python: `~/.claude/skills/python.md`
- Claude config: `~/.claude/skills/claude-config.md`
- infrastructure problems: `~/.claude/skills/infrastructure-troubleshooting.md`
- ESXi performance: `~/.claude/skills/vmware-esxi-performance.md`

Git commits: ALWAYS push immediately. ALWAYS use concise, lowercase messages. NEVER include Co-Authored-By

NEVER use Unicode. ALWAYS use ASCII. ALWAYS reformat Unicode as ASCII

ALWAYS end every response with a confidence rating: X/10. NEVER omit it

## Agents
- NEVER use the Task tool. ALWAYS do all work manually with direct tool calls (Read, Edit, Grep, Glob, Bash). If you think an agent would help, ask first -- the answer will be no
- ALWAYS use Glob/Grep/Read directly for searches. NEVER use agents for searching
