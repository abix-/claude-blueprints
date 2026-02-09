---
name: powershell
description: PowerShell development standards including VMware and Pester. Use when writing PowerShell.
metadata:
  version: "1.0"
  updated: "2026-02-09"
---
# PowerShell

## Core
- Functions only (not scripts); standard Verb-Noun naming
- Always `[CmdletBinding()]`; output `[PSCustomObject]`
- Output before action — "Adding disk X" before the call that might fail, not after
- Use `$()` for expansion (not `${}`), especially with properties or after colons
- `$var:` is a drive qualifier — `"$source: text"` parses as variable `$source:`, use `"$($source): text"`
- No empty lines between function parameters
- Never use automatic variables as iterators (`$event`, `$input`, `$args`, `$this`, `$_` outside pipeline)

## VMware
- `ReconfigVM_Task` returns `ManagedObjectReference`, not Task object
- Wait for async tasks: `Get-Task -Id "$taskMoRef" | Wait-Task` (vSphere 7+)
- Scope all queries with `-Server $vc` when multiple vCenters connected
- Use `-not` instead of `!` when running PowerShell via bash (bash escapes `!` to `\!`)
- Collection pattern: `$results = foreach ($item in $collection) { ... }` (avoid `+=`)
- Prefer splatting for cmdlets with 3+ parameters
- Use `Write-Verbose` for debug/progress messages (not `Write-Host`)
- Comment-based help: `.SYNOPSIS` required, 1+ `.EXAMPLE`, omit `.NOTES`

## Pester
- Pester 5 syntax: `Describe`/`Context`/`It`, `BeforeAll`, `Should -Be`/`-Match`/`-Not`
- Use `[System.IO.File]::WriteAllText()` and `ReadAllText()` for deterministic test data (avoids cmdlet overhead)
- Store expected values in variables, compare after processing — don't hardcode in assertions
- Consolidate test data (IPs, paths, patterns) as variables in `BeforeAll` — single place to maintain
- Create test runner helpers (e.g., `Invoke-TestEnvironment`) to handle setup/teardown/environment swap
- Run with: `Invoke-Pester ./tests.ps1 -Output Detailed`

## Bash Interop
- Single quotes prevent bash `$` expansion: `printf '%s' '$var'` → literal `$var`
- Escape single quotes for bash: `'` → `'\''`
- `cmd /c` mangles nested quotes — use PowerShell as shell instead
- Self-referential hooks: keep sensitive paths internal to tool, not in command string
- Windows `cd`: use `cd C:/code/path` not `cd /d C:\code\path` — `/d` is cmd.exe syntax
- `&&` fails in PowerShell — use `;` or separate commands when hooks intercept bash
