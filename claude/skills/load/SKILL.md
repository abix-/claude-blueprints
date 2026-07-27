---
description: Pull claude-blueprints and install its shared and Claude runtime files on Windows.
disable-model-invocation: true
allowed-tools: Bash(git:*), Bash(powershell.exe:*), Bash(python:*)
---

# Load

Use when the user asks to load or apply the latest shared Claude blueprints.

The git repository is the authority. Installed files are generated copies and
must not be edited as their source.

```bash
repo="C:/code/claude-blueprints"
git -C "$repo" pull
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$repo/install.ps1" -Runtime "claude"
python "$repo/sync-check.py" check --runtime claude
```

Stop if the pull would overwrite uncommitted repository work. The installer
copies tracked source files but does not remove unrelated local files.

Report the pull result, copied file count, and final drift-check result.
