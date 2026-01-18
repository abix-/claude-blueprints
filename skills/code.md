---
name: code
description: Development standards for Ansible, PowerShell, and Go
metadata:
  version: "1.0"
  updated: "2026-01-18"
---
# Coding Standards

## Universal Principles
1. **MVP first** — simplest working solution; add complexity only when requested
2. **Assume fallibility** — first attempts are often suboptimal; verify syntax/parameters via docs or web search when uncertain
3. **Self-assess** — include confidence rating (1-10) and note uncertainties
4. **Code review first** — present code in fenced blocks with confidence rating; implement only after approval
5. **Minimal docs** — SE2 should follow; explain *what/why*, not *how*
6. Status messages match property names — use same terms in messages as output properties
7. Comments mark phases — explain why and major sections, not individual lines

## Ansible
- Roles contain all logic; playbooks only call roles
- All variables in `vars/main.yml` (not `defaults/`)
- Always FQCN (`ansible.builtin.copy`, not `copy`)
- Prefer modules over shell/command when a module exists
- When shell is required: `ansible.builtin.shell` (Linux) or `ansible.windows.win_shell` (Windows)

## PowerShell
- Functions only (not scripts); standard Verb-Noun naming
- Always `[CmdletBinding()]`; output `[PSCustomObject]`
- Use `$()` for expansion (not `${}`), especially with properties or after colons
- Use `-not` instead of `!` when running PowerShell via bash (bash escapes `!` to `\!`)
- Collection pattern: `$results = foreach ($item in $collection) { ... }` (avoid `+=`)
- Prefer splatting for cmdlets with 3+ parameters
- Use `Write-Verbose` for debug/progress messages (not `Write-Host`)
- Comment-based help: `.SYNOPSIS` required, 1+ `.EXAMPLE`, omit `.NOTES`

## Go
- CLI tools: single binary with subcommands (`switch os.Args[1]`)
- Small projects: flat package structure under `internal/`
- Private packages in `internal/` — not importable externally
- Module path: `github.com/user/repo/subdir`
- Error handling: `if err != nil { return err }` — don't over-wrap
- JSON config: strip UTF-8 BOM before `json.Unmarshal` (Windows creates BOM)
- Regex: no negative lookahead `(?!...)` — use alternation or post-filtering
- When Go > scripts: cold-start matters, single binary, cross-platform

## Avoid
- Excessive error handling — simple is fine, overblown is not
- Variables for single-use values
- Comments explaining obvious operations
- Guessing parameters — verify syntax via docs or web search before writing code
- Inventing plausible-sounding syntax — "looks right" is not verification
- Unverified code without disclosure — if not verified, explicitly state it
- Go: nested hierarchies, premature interfaces, channels when mutex suffices

## Response Efficiency
- Single targeted change: describe it, don't output full file
- Multiple changes: output full file
- Change exactly what's asked — nothing more
