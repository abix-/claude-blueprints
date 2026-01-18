---
name: golang
description: Go development standards
metadata:
  version: "1.0"
  updated: "2026-01-18"
---
# Go Standards

## Project Structure
- CLI tools: single binary with subcommands (`switch os.Args[1]`)
- Small projects: flat package structure under `internal/`
- Private packages in `internal/` — not importable externally
- Module path: `github.com/user/repo/subdir`

## Patterns
- Error handling: `if err != nil { return err }` — don't over-wrap
- JSON config: strip UTF-8 BOM before `json.Unmarshal` (Windows creates BOM)
- Regex: no negative lookahead `(?!...)` — use alternation or post-filtering

## When Go > Scripts
- Hooks/CLI where cold-start matters (Go ~10ms vs PowerShell ~200ms)
- Single binary deployment — no runtime dependencies
- Cross-platform with minimal changes

## Avoid
- Nested package hierarchies for small projects
- Interfaces before you need them
- Channels when a mutex suffices
