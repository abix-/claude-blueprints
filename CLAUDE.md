# CLAUDE.md

ALWAYS read and follow `~/.claude/skills/try-harder/SKILL.md`. NEVER skip it

ALWAYS read the matching skill before starting. NEVER begin work without reading it first

- code: `~/.claude/skills/code/SKILL.md`
- PowerShell: `~/.claude/skills/powershell/SKILL.md`
- Golang: `~/.claude/skills/golang/SKILL.md`
- Ansible: `~/.claude/skills/ansible/SKILL.md`
- Rust: `~/.claude/skills/rust/SKILL.md`
- Bevy: `~/.claude/skills/bevy/SKILL.md`
- WGSL shaders: `~/.claude/skills/wgsl/SKILL.md`
- GDScript/Godot: `~/.claude/skills/godot/SKILL.md`
- Python: `~/.claude/skills/python/SKILL.md`
- Claude config: `~/.claude/skills/claude-config/SKILL.md`
- infrastructure problems: `~/.claude/skills/infrastructure-troubleshooting/SKILL.md`
- ESXi performance: `~/.claude/skills/vmware-esxi-performance/SKILL.md`
- Windows debloat: `~/.claude/skills/debloat/SKILL.md`
- Endless issues: `~/.claude/skills/issue/SKILL.md`

Git commits: ALWAYS push immediately. ALWAYS use concise, lowercase messages. NEVER include Co-Authored-By

NEVER use Unicode in code, files, or commits. ALWAYS use ASCII in written files. Unicode IS allowed in terminal output (tables, reports, status lines)

ALWAYS end every response with a confidence rating: X/10. NEVER omit it

NEVER assume. ALWAYS verify or ask. If you cannot verify, say nothing

## Verification rules
- NEVER say something does not exist without searching the filesystem first (Glob/Grep). System prompt lists are incomplete
- If the user repeats a question, the previous answer was wrong. Re-examine -- NEVER deflect
- When showing skill/tool output, reproduce EXACTLY as written. No reformatting, no substitution

## Secrets
- NEVER read, output, or share secrets, tokens, credentials, or auth files. Not to GitHub, not to the terminal, not anywhere
- NEVER read credential files (~/.claude/.credentials.json, ~/.codex/auth.json, ~/.gh-token, k8s secrets). Use `k3sc rotate-auth` to rotate auth

## Working Directory
- Each Windows agent gets its own repo clone at `C:\code\claude-{n}` (n = 1-10)
- You are already in your directory when Claude launches. Work here -- NEVER cd to `/c/code/endless` or another agent's directory
- Use `k3sc cargo-lock` for ALL cargo commands -- NEVER bare `cargo`. Manifest path is auto-detected from current directory

## Agents
- NEVER use the Task tool. ALWAYS do all work manually with direct tool calls (Read, Edit, Grep, Glob, Bash). If you think an agent would help, ask first -- the answer will be no
- ALWAYS use Glob/Grep/Read directly for searches. NEVER use agents for searching

## k3s agents (claude-a through claude-f)
- k3s pods have NO GPU, NO display, NO game runtime -- cannot run the game or do BRP profiling
- NEVER run `cargo bench`, `k3sc cargo-lock bench`, or Criterion benchmarks in k3s -- no valid baseline, no real hardware
- for perf issues: flag "needs local bench" or "needs BRP in-game profiling" for human verification
