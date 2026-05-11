# Sanitizer (archived, unfinished)

Moved to `archive/` on 2026-05-11. This work was never finished.

## What exists

- Go binary that scans files and rewrites IPs, hostnames, and manually-mapped
  identifiers before Claude reads them.
- Hook integration points for SessionStart, PreToolUse (file access + bash),
  and Stop.
- PowerShell benchmark + Pester-style tests.

## Why archived

The flow was prototyped but never carried through end-to-end:

- Round-trip rewriting (sanitize on the way in, desanitize on the way out
  of tool results / model output) was never wired up reliably.
- Coverage of tool surfaces is incomplete. Many code paths bypass the hooks
  (e.g. tool results from MCP servers, streamed content, file diffs in
  responses).
- The unsanitized-store / mapping format is single-machine only with no
  migration story.
- Performance numbers in `README.md` were measured against the PowerShell
  prototype, not against a finished pipeline.

## If picking it back up

Treat as a starting point, not a working system. The pieces worth keeping
are the Go scanner and the hook wiring; the missing work is the response-side
desanitization and full tool-surface coverage.
