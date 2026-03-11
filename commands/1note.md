---
description: Read and search OneNote notebooks via COM interop
allowed-tools: Bash
---

Run the OneNote PowerShell script with the user's arguments:

```bash
powershell.exe -NoProfile -File "$HOME/.claude/scripts/1note.ps1" $ARGUMENTS
```

If no arguments provided, run without arguments to show usage help.

Subcommands:
- `list` — show all notebooks/sections/pages. Use `-section <name>` to filter.
- `read <page>` — read page content. Supports partial name matching. Use `-section <name>` to narrow scope.
- `search <term>` — full-text search across all notebooks.

After getting results from `search`, offer to `read` specific pages the user is interested in.
Tables are returned as markdown tables. Text content has HTML tags stripped.
