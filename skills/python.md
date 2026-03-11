---
name: python
description: Python environment on this Windows machine. Read when running Python scripts or using Python for data processing.
metadata:
  version: "1.0"
  updated: "2026-03-06"
---
# Python

## Available executables

| Command | Path | Works |
|---------|------|-------|
| `python` | `C:/Users/Abix/AppData/Local/Programs/Python/Python312/python` | Yes |
| `py` | `C:/Users/Abix/AppData/Local/Programs/Python/Launcher/py` | Yes |
| `python3` | Symlink to Python312 (was Store shim, fixed 2026-03-06) | Yes |

**Version:** Python 3.12.10

## Usage in bash

Always use `python` (not `python3`) in bash commands:

```bash
python -c "print('hello')"
curl -s http://example.com | python -c "import json,sys; print(json.load(sys.stdin))"
```

## File paths

When passing file paths to Python from bash, use Windows-style paths (`C:/tmp/file.json`), not bash paths (`/c/tmp/file.json`). Python's `fs.readFileSync` equivalent is `open()` / `pathlib.Path`.
