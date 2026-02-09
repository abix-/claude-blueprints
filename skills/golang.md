---
name: golang
description: Golang development standards. Use when writing Go.
metadata:
  version: "1.0"
  updated: "2026-02-09"
---
# Golang

## Core
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

## Avoid
- Nested hierarchies, premature interfaces
- Channels when mutex suffices
