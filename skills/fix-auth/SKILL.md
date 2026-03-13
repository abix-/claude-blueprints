---
description: Restore .claude.json from the most recent auto-backup when auth is corrupted.
disable-model-invocation: true
version: "1.0"
---
Restore .claude.json from the most recent auto-backup when auth is corrupted.

Steps:
1. Check if `~/.claude.json` exists and is valid JSON with `python -c "import json; json.load(open('C:/Users/Abix/.claude.json'))"`
2. If valid, tell the user the file is fine — no restore needed.
3. If invalid or missing, find the most recent `~/.claude/backups/.claude.json.backup.*` file (these are auto-created by Claude Code before each session).
4. Verify the backup is valid JSON too before restoring.
5. Copy the backup to `~/.claude.json`.
6. Clean up old `.corrupted.*` files from `~/.claude/backups/` (keep only the 5 most recent).
7. Tell the user to restart Claude Code.
