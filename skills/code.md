---
name: code
description: Development standards for Ansible, PowerShell, and Golang
metadata:
  version: "2.1"
  updated: "2026-01-23"
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
8. Default configs match documentation — same order, same example values
9. Magic numbers → named constants when used in multiple places
10. Never silently suppress errors — log or propagate
11. Names reflect purpose — `GetConfigPath` not `GetSecretsPath`
12. Stdlib over custom — don't reimplement what's built-in

## Ansible
- Roles contain all logic; playbooks only call roles
- All variables in `vars/main.yml` (not `defaults/`)
- Always FQCN (`ansible.builtin.copy`, not `copy`)
- Always start YAML files with `---`
- Use `validate_certs: false` not `validate_certs: no`
- Prefer modules over shell/command when a module exists
- When shell is required: `ansible.builtin.shell` (Linux) or `ansible.windows.win_shell` (Windows)
- Inline PowerShell formatting for troubleshooting:
  - Pipelines → single line (easy to copy/paste)
  - foreach/for loops → multiline
  - PSCustomObject → multiline OK
  - Avoid dense semicolon chains

## PowerShell
- Functions only (not scripts); standard Verb-Noun naming
- Always `[CmdletBinding()]`; output `[PSCustomObject]`
- Output before action — "Adding disk X" before the call that might fail, not after
- Use `$()` for expansion (not `${}`), especially with properties or after colons
- `$var:` is a drive qualifier — `"$source: text"` parses as variable `$source:`, use `"$($source): text"`
- No empty lines between function parameters
- Never use automatic variables as iterators (`$event`, `$input`, `$args`, `$this`, `$_` outside pipeline)

## PowerShell + VMware
- `ReconfigVM_Task` returns `ManagedObjectReference`, not Task object
- Wait for async tasks: `Get-Task -Id "$taskMoRef" | Wait-Task` (vSphere 7+)
- Scope all queries with `-Server $vc` when multiple vCenters connected
- Use `-not` instead of `!` when running PowerShell via bash (bash escapes `!` to `\!`)
- Collection pattern: `$results = foreach ($item in $collection) { ... }` (avoid `+=`)
- Prefer splatting for cmdlets with 3+ parameters
- Use `Write-Verbose` for debug/progress messages (not `Write-Host`)
- Comment-based help: `.SYNOPSIS` required, 1+ `.EXAMPLE`, omit `.NOTES`

## Golang
- CLI tools: single binary with subcommands (`switch os.Args[1]`)
- Subcommand names: user-facing clarity > internal jargon
- Small projects: flat package structure under `internal/`
- Private packages in `internal/` — not importable externally
- Module path: `github.com/user/repo/subdir`
- Error handling: `if err != nil { return err }` — don't over-wrap
- JSON config: strip UTF-8 BOM before `json.Unmarshal` (Windows creates BOM)
- Regex: no negative lookahead `(?!...)` — use alternation or post-filtering
- When Golang > scripts: cold-start matters, single binary, cross-platform
- Exports: unexport functions only used within the same package
- Debug tracing: add temp `os.WriteFile` to trace values, rebuild, run, then remove — don't guess at runtime state

## Bash + PowerShell interop
- Single quotes prevent bash `$` expansion: `printf '%s' '$var'` → literal `$var`
- Escape single quotes for bash: `'` → `'\''`
- `cmd /c` mangles nested quotes — use PowerShell as shell instead
- Self-referential hooks: keep sensitive paths internal to tool, not in command string
- Windows `cd`: use `cd C:/code/path` not `cd /d C:\code\path` — `/d` is cmd.exe syntax
- `&&` fails in PowerShell — use `;` or separate commands when hooks intercept bash

## Avoid
- Excessive error handling — simple is fine, overblown is not
- Variables for single-use values
- Comments explaining obvious operations
- Golang: nested hierarchies, premature interfaces, channels when mutex suffices

## Testing
- Verify tests actually validate the change — input shouldn't already match expected pattern

## PowerShell/Pester
- Pester 5 syntax: `Describe`/`Context`/`It`, `BeforeAll`, `Should -Be`/`-Match`/`-Not`
- Use `[System.IO.File]::WriteAllText()` and `ReadAllText()` for deterministic test data (avoids cmdlet overhead)
- Store expected values in variables, compare after processing — don't hardcode in assertions
- Consolidate test data (IPs, paths, patterns) as variables in `BeforeAll` — single place to maintain
- Create test runner helpers (e.g., `Invoke-TestEnvironment`) to handle setup/teardown/environment swap
- Run with: `Invoke-Pester ./tests.ps1 -Output Detailed`

## Response Efficiency
- Single targeted change: describe it, don't output full file
- Multiple changes: output full file
- Change exactly what's asked — nothing more
