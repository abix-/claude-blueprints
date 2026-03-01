---
description: Trigger GitHub Actions dev build (windows, dev channel)
allowed-tools: Bash
---

Run the GitHub Actions `build` workflow with default inputs (windows target, dev release channel).

```bash
cd /c/code/endless && gh workflow run build 2>&1
```

After triggering, show the run URL:

```bash
sleep 3 && gh run list --workflow=build --limit=1 --json databaseId,status,url --jq '.[0] | "Run #\(.databaseId): \(.status) — \(.url)"' 2>&1
```
