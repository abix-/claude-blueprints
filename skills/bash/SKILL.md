---
name: bash
description: Bash scripting standards for shell scripts, CI workflows, and one-off automation. Use when writing .sh files or non-trivial bash one-liners.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# Bash

## Header
Every script starts with:

```bash
#!/usr/bin/env bash
set -euo pipefail
```

- `-e` exit on error
- `-u` error on unset variable
- `-o pipefail` propagate failure through pipes
- Add `IFS=$'\n\t'` if iterating over lines.

## Quoting
- Always quote variable expansions: `"$var"`, `"$@"`, `"${arr[@]}"`. Unquoted breaks on spaces and globs.
- Single quotes preserve literals. Double quotes allow expansion.
- `"${var:-default}"` for default values. `"${var:?error msg}"` to fail if unset.

## Conditionals
- `[[ ... ]]` over `[ ... ]`. Better operators, no quoting bugs.
- String compare: `[[ "$a" == "$b" ]]`. Glob match: `[[ "$file" == *.txt ]]`.
- Regex: `[[ "$s" =~ ^foo ]]`. Captures in `${BASH_REMATCH[@]}`.
- Numeric: `(( a > b ))`. Don't use `[[ ]]` for arithmetic.

## Functions
- Declare with `name() { ... }`. No `function` keyword.
- Local vars: `local x="$1"` (always quote, always local).
- Return status via exit code. Output via stdout. Don't mix.

## Loops
- `for f in *.txt; do` works if files exist. Use `shopt -s nullglob` to handle no matches.
- `while IFS= read -r line; do ...; done < file` for line iteration. Always `read -r`.
- Never parse `ls`. Use globs or `find -print0 | xargs -0`.

## Process control
- `command || true` to continue past expected failures under `set -e`.
- `command1 && command2` for chained success; `||` for fallback.
- `trap 'cleanup' EXIT` to clean up temp files on any exit.
- `mktemp -d` for temp directories; never hardcode `/tmp/foo`.

## Cross-platform
- Use `#!/usr/bin/env bash` not `/bin/bash`. macOS ships ancient bash 3.2.
- For bash 4+ features (associative arrays, `${var,,}`), check `$BASH_VERSION` or just require bash 5+.
- On Windows (Git Bash, WSL), prefer forward slashes; the shell rewrites paths for native exes.

## Patterns
- Check command exists: `command -v foo >/dev/null || { echo "foo required"; exit 1; }`.
- Read script directory: `cd "$(dirname "$(readlink -f "$0")")"`.
- Iterate JSON: pipe to `jq`. Don't grep JSON.

## Avoid
- `eval`. Almost always wrong.
- Backticks. Use `$(...)`.
- `cat file | grep x`. Use `grep x file` or `<file grep x`.
- Parsing `ps` / `ls` / `find` output line-by-line without `-print0`.
- `[ $? -eq 0 ]`. Use `if cmd; then ...; fi`.
- Long inline scripts in CI YAML. Extract to `scripts/foo.sh` and call it.
