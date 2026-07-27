# Project State

> Git-tracked. Never put secrets, tokens, or credentials in this file.

## Current focus

One git repository owns shared skills plus separate Claude and Codex runtime
files. The repository name remains `claude-blueprints` for clone and automation
continuity.

## Design goals

- `skills/` is authority for agent-neutral skills.
- `claude/` contains only Claude runtime files.
- `codex/` contains only Codex runtime files.
- Installed files are generated copies, never authority.
- Windows setup uses `install.ps1`.
- `sync-check.py` owns installation and drift checks for both runtimes.
- Codex work stays in the foreground and uses PowerShell on Windows.

## Last session

On 2026-07-27:

- Reworked `learn` to review every git repository over a 30 day window and
  route reusable findings into authoritative skills.
- Inventoried 75 repositories and 852 recent commits. Seven repositories had
  recent commits.
- Updated code, C#, Eufy, and Factoriobot skills from reviewed evidence.
- Separated Claude-only and Codex-only runtime files.
- Added tested Windows installation for Claude, Codex, or both.
- Normalized shared skill frontmatter for Codex compatibility.
- Corrected stale Claude skill paths and the Claude session hook.
- Verified 64 shared and Codex-only skills with the official Codex validator.

## Next steps

- Install the committed runtime files into the live Claude and Codex homes.
- Run drift checks for both live installations.
- Return to the Factoriobot desired-state work pool implementation.

## Open questions

- None for the shared runtime layout.
