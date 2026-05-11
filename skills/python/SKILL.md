---
name: python
description: Python standards and the Python environment on this Windows machine. Sourced from abixio/tests, abixio/build.py, endless/scripts, and claude-blueprints/scripts. Read when running Python or writing scripts.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Python

Source repos: `abix-/abixio` (build + e2e), `abix-/endless/scripts`,
`claude-blueprints/scripts`. Style is **stdlib-first, script-shaped,
no virtualenvs**. Real CPython on the machine, used directly.

## Environment

| Command | Path | Works |
|---------|------|-------|
| `python` | `C:/Users/Abix/AppData/Local/Programs/Python/Python312/python` | Yes |
| `py` | `C:/Users/Abix/AppData/Local/Programs/Python/Launcher/py` | Yes |
| `python3` | Symlink to Python312 (was Store shim, fixed 2026-03-06) | Yes |

**Version:** Python 3.12.10. **No virtualenv.** Scripts import what's
installed system-wide. Pin requirements at the top of a script in a
`try/except ImportError` block with a `pip install` hint (see e2e.py).

## When Python is the right tool

- One-off scripts and build glue (`abixio/build.py` shells out to cargo).
- BRP / HTTP clients where stdlib is enough (`endless/scripts/brp.py`).
- Test harnesses (`abixio/tests/e2e.py`).
- Text munging tools where `re` + `pathlib` is shorter than bash
  (`scripts/dehyphen.py`).

Reach for Go or Rust when: cold start matters, a single binary is
needed, types/lifetimes matter, perf hot path.

## Script header

```python
#!/usr/bin/env python3
"""One-line purpose. Then a longer description if needed.

Usage:
    python path/to/script.py [args]

Requires: requests (pip install requests)   # only if non-stdlib
"""
```

Real example from `abixio/build.py`:

```python
"""build abixio (server) and abixio-ui release binaries for windows."""
```

Lowercase, informal, terse. No "This module..." preamble.

## Imports

- Stdlib first, blank line, then third-party. No `isort` formality
  for short scripts; just keep it ordered.
- `from pathlib import Path` always. Never use `os.path` in new code.
- Optional deps guarded:
  ```python
  try:
      import requests
  except ImportError:
      print("ERROR: pip install requests")
      sys.exit(1)
  ```
- Prefer `urllib.request` over `requests` when the call is simple
  (see `endless/scripts/brp.py`). `requests` is fine when the script
  is already complex (see `abixio/tests/e2e.py`).

## CLI shape

- Tiny script (1-3 args): hand-roll `sys.argv` parsing.
  ```python
  if len(sys.argv) < 2:
      print("Usage: python scripts/brp.py <method> [params_json]")
      sys.exit(1)
  ```
- Anything more: `argparse`.
- Subcommands with shared flags: still `argparse` with subparsers, not
  click / typer. Stdlib is enough.
- Exit codes: 0 success, 1 generic failure. Don't invent more.

## Paths

```python
ROOT = Path(__file__).resolve().parent
```

- Always resolve relative to `__file__`, not CWD.
- `Path` for everything. Use `/` operator: `ROOT / "target" / "release"`.
- `Path.exists()`, `Path.read_text()`, `Path.write_text(encoding="utf-8")`.
- On Windows, pass `Path` objects to `subprocess`; it handles separators.

## Subprocess

Pattern from `abixio/build.py`:

```python
def run(args, cwd=None):
    print(f"  > {' '.join(str(a) for a in args)}")
    result = subprocess.run(args, cwd=cwd)
    if result.returncode != 0:
        print(f"FAILED (exit {result.returncode})")
        sys.exit(1)
```

- `subprocess.run`, never `os.system` or `os.popen`.
- Pass `args` as a list, not a shell string. No `shell=True`.
- Echo the command first so logs are debuggable.
- Check `returncode` manually when you want a custom error message;
  use `check=True` when default behavior is fine.
- For output capture: `capture_output=True, text=True` then read
  `result.stdout`. UTF-8 is the default in 3.7+.

## Error output

Pattern across all repos:

```python
print(f"ERROR: {msg}", file=sys.stderr)
sys.exit(1)
```

- Errors to stderr, results to stdout. Pipeline-friendly.
- f-strings everywhere. No `%` formatting, no `.format()`.
- No tracebacks for user-facing errors; catch the exception and print
  a clean message.

## HTTP

Stdlib for simple POST/GET (`endless/scripts/brp.py`):

```python
req = urllib.request.Request(
    f"http://localhost:{port}",
    json.dumps(body).encode(),
    {"Content-Type": "application/json"},
)
resp = json.loads(urllib.request.urlopen(req, timeout=5).read())
```

- Always set a `timeout`. Default is unbounded.
- For repeated requests, retries, JSON shape sanity, or auth: pull in
  `requests`. Don't fight stdlib past its sweet spot.

## Classes and functions

- Free functions by default. Reach for a class when there's mutable
  state shared across methods (`TestRunner` in e2e.py: counter +
  errors list).
- No dataclasses for 2-field structs in a script; a `dict` or tuple
  is fine. Use dataclasses when the type appears across functions.
- No abstract base classes in scripts. Type-hint with concrete types
  or `Protocol` if needed.

## Type hints

- Light touch. Scripts in these repos are mostly untyped. Add hints
  when:
  - The function signature is exported across modules.
  - The argument type is non-obvious from the name.
  - You want IDE help in a long script.
- `from __future__ import annotations` at the top if you want to use
  postponed evaluation (`list[int]` style on 3.9-) or forward refs.
- No `mypy` enforcement in current repos. Don't add it without buy-in.

## Testing

- `abixio/tests/e2e.py` is the model: a `TestRunner` class with
  `check(name, condition, detail)` and `summary()`. Plain script,
  exit code reflects pass/fail.
- No pytest in current Python code. If a project grows past a single
  test file, then add pytest. Stdlib `unittest` is acceptable but
  verbose for this code's shape.
- Tests are scripts you can run directly. `python tests/e2e.py`.

## Style

- Lowercase, informal docstrings. Not formal Sphinx prose.
- f-strings everywhere. `f"  {p.name}  {size_mb:.1f} MB"`.
- 4-space indent, LF endings, UTF-8 with no BOM.
- Two blank lines between top-level defs, one between methods.
- No emoji. No Unicode in code (the dehyphen script imports the em-dash
  character literally; that's the exception that proves the rule).
- ASCII-only string literals unless the data demands otherwise.

## Idempotency

Scripts should be safe to re-run. Examples:

- `dehyphen.py` rewrites prose; running twice on a clean file is a no-op.
- `build.py` overwrites the exe in place; no leftover state.
- Avoid scripts that append-only to a file without a header / dedup.

## Performance

- Don't optimize Python; rewrite the hot path in Rust or Go.
- For text munging: compiled regex is the only meaningful win
  (`re.compile` once at module scope).
- `subprocess.run` has fixed overhead (~50ms). Batch when possible.
- `json.loads` on a `bytes` object is fine in 3.6+; no decode step.

## Windows specifics

- Use forward slashes in path strings even on Windows; `Path` and
  `subprocess` both handle them.
- BOM: if reading a file PowerShell may have written, strip the BOM:
  `text = path.read_text(encoding="utf-8-sig")`.
- Long paths: prefix with `\\?\` only if you actually hit MAX_PATH.

## Avoid

- `import *`. Never.
- `os.path`. Use `pathlib`.
- `print(x)` for errors. Use `print(x, file=sys.stderr)`.
- `eval` / `exec`. Almost always wrong.
- `shell=True` in subprocess. Quoting bugs and injection.
- Mutable default args (`def f(x=[])`). Use `None` and assign inside.
- Bare `except:`. Always catch a specific class.
- Reaching for `pandas` / `numpy` for jobs that 10 lines of stdlib
  would solve.
- Adding requirements.txt / pyproject.toml to a one-file script. Pin
  with the `try/except ImportError` pattern instead.
