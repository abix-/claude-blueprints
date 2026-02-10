---
name: code
description: Universal development standards. Use when writing any code.
metadata:
  version: "3.1"
  updated: "2026-02-09"
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

## DRY
- Before writing new code, check for existing patterns that do the same thing. Extract shared logic into helpers proactively — don't wait to be asked.
- When you see 3+ copies of a pattern (boilerplate params, guard clauses, setup sequences), extract it immediately. Propose the helper, then apply it across all call sites in one pass.
- SystemParam bundles over repeated parameter lists. Shared helper functions over inline logic.

## Testing / TDD
- Verify tests actually validate the change — input shouldn't already match expected pattern
- When adding new behavior, write or update the test first (red), then implement (green). Don't leave test updates as an afterthought.
- When fixing a bug, reproduce it in a test before writing the fix.
- After any code change, check which tests cover the changed code and verify they still pass.

## Avoid
- Excessive error handling — simple is fine, overblown is not
- Variables for single-use values
- Comments explaining obvious operations

## Response Efficiency
- Single targeted change: describe it, don't output full file
- Multiple changes: output full file
- Change exactly what's asked — nothing more
