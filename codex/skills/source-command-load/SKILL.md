---
name: source-command-load
description: Pull claude-blueprints and install its shared and Codex runtime files on Windows.
---

# Load

Use when the user asks to load or apply the latest shared Codex blueprints.

The git repository is the authority. Installed files are generated copies and
must not be edited as their source.

Run in PowerShell:

```powershell
$repo = "C:\code\claude-blueprints"
git -C $repo pull
& (Join-Path $repo "install.ps1") -Runtime "codex"
python (Join-Path $repo "sync-check.py") check --runtime codex
```

Stop if the pull would overwrite uncommitted repository work. The installer
copies tracked source files but does not remove unrelated local files.

Report the pull result, copied file count, and final drift-check result.
