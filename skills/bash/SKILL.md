---
name: bash
description: Bash scripting standards for shell scripts, CI workflows, and one-off automation. Built from Google Shell Style Guide, ShellCheck rules, and BashFAQ canonical patterns.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Bash

Target: bash 5+. macOS ships ancient bash 3.2 (license-locked at GPLv2);
on macOS use `#!/usr/bin/env bash` and install bash 5 via Homebrew, or
write POSIX `/bin/sh` that runs anywhere.

**Default tool:** ShellCheck. Run on every script. Most rules in this
skill correspond to a SC#### rule that ShellCheck catches automatically.

## Header

Every script:

```bash
#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
```

- `-e` exit on any non-zero return.
- `-u` error on unset variable expansion.
- `-o pipefail` propagate failure through pipes (without it, `false | true`
  is success).
- `IFS=$'\n\t'`. Avoid space-splitting traps when reading lines.
  Adds discipline; some scripts that intentionally word-split need to
  un-set this locally.
- `-x` for debugging; remove before shipping.

For very strict scripts add `shopt -s inherit_errexit` so `-e` applies
inside command substitutions (bash 4.4+).

## Variables

- **Always quote expansions:** `"$var"`, `"$@"`, `"${arr[@]}"`,
  `"${arr[*]}"`. Unquoted variables word-split and glob-expand.
- **Use `${var}` braces** when the variable is followed by alphanumerics:
  `"${dir}-backup"`.
- **Defaults:** `"${var:-default}"` (use default if unset/empty),
  `"${var:=default}"` (also assign), `"${var:?error msg}"` (fail with
  message if unset).
- **String ops:** `"${var#prefix}"`, `"${var%suffix}"`,
  `"${var/from/to}"`, `"${var//from/to}"` (global), `"${var,,}"`
  (lowercase), `"${var^^}"` (uppercase).
- **Lengths and slices:** `"${#var}"`, `"${var:offset:length}"`.
- **Arrays:** `arr=(a b c)`, `"${arr[@]}"` (separate words),
  `"${arr[*]}"` (one string), `"${#arr[@]}"` (length),
  `"${arr[@]:1:2}"` (slice).
- **Associative arrays (bash 4+):** `declare -A m; m[key]="val"`.

## Conditionals

- **`[[ ... ]]`** over `[ ... ]`. Better operators, no field splitting,
  glob and regex support.
- String compare: `[[ "$a" == "$b" ]]`, `[[ "$a" != "$b" ]]`.
- Glob match: `[[ "$file" == *.txt ]]`, `[[ "$file" == prod-* ]]`.
- Regex: `[[ "$s" =~ ^foo([0-9]+)$ ]]`. Captures land in
  `${BASH_REMATCH[@]}`. Use unquoted RHS for regex (quoting it makes
  it literal).
- Numeric: `(( a > b ))`. Returns exit code (`0` = true). Inside
  `(( ))`, `$` is optional: `(( count++ ))`.
- File tests: `[[ -f file ]]` (exists, regular file), `[[ -d dir ]]`,
  `[[ -L link ]]`, `[[ -r file ]]` (readable), `[[ -s file ]]`
  (non-empty), `[[ -e path ]]` (exists, any type).
- Combine: `[[ a && b ]]`, `[[ a || b ]]`. Inside `[[`, `&&` and `||`
  are boolean (no short-circuit between separate `[[ ]]` blocks).

## Functions

```bash
# Doc comment describes what the function does and the return contract.
print_config() {
    local path="$1"
    [[ -r "$path" ]] || { echo "cannot read $path" >&2; return 1; }
    cat "$path"
}
```

- Declare with `name() { ... }`. No `function` keyword (POSIX).
- `local` for all internal vars. Without it, you mutate caller's
  scope.
- `local x="$1"`. Quote even the assignment.
- Return status via exit code (`return N`), data via stdout. Mixing
  the two is a common bug.
- Capture output: `result="$(my_func arg)"`. The `$(...)` strips
  trailing newlines.
- Pass arrays by name: `process arr_name` then in callee
  `local -n arr=$1`. Bash 4.3+ namerefs.

## Loops

```bash
# Glob with nullglob to handle empty matches.
shopt -s nullglob
for f in *.txt; do
    echo "$f"
done

# Read a file line by line.
while IFS= read -r line; do
    echo "[$line]"
done < input.txt

# C-style numeric loop.
for ((i = 0; i < 10; i++)); do
    echo "$i"
done
```

- `shopt -s nullglob` makes `*.txt` expand to nothing instead of the
  literal pattern when no files match.
- `IFS= read -r` -- `IFS=` preserves leading/trailing whitespace,
  `-r` disables backslash escaping. Without `-r`, `\n` in the file
  gets interpreted.
- Iterate over `find`: use `-print0` and `xargs -0` (or `mapfile`):
  ```bash
  while IFS= read -r -d '' f; do
      echo "$f"
  done < <(find . -type f -print0)
  ```
- Never `for f in $(ls)`. Use globs.

## Process control

```bash
# Cleanup on any exit.
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

# Allow command to fail without aborting.
command || true

# Chain success / fallback.
command1 && command2
command1 || fallback

# Capture exit code separately from output.
output="$(command)" || exit_code=$?
```

- `trap '...' EXIT` runs on any exit (normal, error, signal). Make
  the handler idempotent.
- `trap '...' ERR` runs on uncaught error (with `-e`). Useful for
  logging then re-exiting.
- `mktemp -d` for temp directories. Never hardcode `/tmp/foo`.
- `mktemp` (no `-d`) for a temp file.

## Subshells and command substitution

```bash
# $(...) preferred over backticks.
date_str="$(date +%F)"

# Process substitution: command output as a file.
diff <(cmd_a) <(cmd_b)

# Subshell scope: vars set inside don't leak.
( cd /some/dir; ls; )
```

- `$(...)` over backticks. Nestable, readable, ShellCheck-approved.
- `<(...)` and `>(...)` for process substitution. Linux/macOS only;
  not POSIX.
- Subshells `( ... )` for scoped changes (cd, set, IFS). They cost
  a fork; avoid in tight loops.

## Error handling

```bash
# Function that may fail.
fetch_data() {
    local url="$1"
    local body
    if ! body="$(curl -sS --fail "$url")"; then
        echo "fetch failed: $url" >&2
        return 1
    fi
    echo "$body"
}

# Caller.
if data="$(fetch_data "$URL")"; then
    process "$data"
else
    log_error "fetch failed; using cache"
    data="$(cat cache.txt)"
fi
```

- Errors to stderr (`>&2`). Results to stdout.
- Don't rely on `$?` after a chain; capture explicitly.
- Exit codes: 0 success, 1 generic failure, 2 usage error, 64-78
  reserved (`sysexits.h`), 126 command not executable, 127 not found,
  128+N killed by signal N.

## Patterns

```bash
# Check command exists.
command -v jq >/dev/null || { echo "jq required" >&2; exit 1; }

# Get script's own directory (resolves symlinks).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Realpath if available.
SCRIPT_PATH="$(realpath "$0" 2>/dev/null || readlink -f "$0")"

# Iterate JSON via jq.
jq -r '.items[] | .name' < data.json | while IFS= read -r name; do
    echo "$name"
done

# Pass an array to a function (bash 4.3+).
process_files() {
    local -n files=$1
    for f in "${files[@]}"; do
        echo "$f"
    done
}
my_files=(a.txt b.txt)
process_files my_files
```

## Performance

- **Avoid forking in hot loops.** Every external command is a fork +
  exec. Built-ins (`echo`, `[[ ]]`, `(( ))`, `printf`, `read`) stay in
  the bash process. `grep`, `awk`, `sed` fork.
- **One `awk` beats five `grep | cut | sort`.** Awk processes the
  stream once.
- **`mapfile -t arr < file`** beats `while read` loops for reading a
  whole file into an array. ~10x faster on large inputs.
- **Parameter expansion** beats `sed`/`tr` for simple substitutions:
  `"${var//foo/bar}"` instead of `sed 's/foo/bar/g'`.
- **`[[ "$var" == *.txt ]]`** beats `echo "$var" | grep '\.txt$'`.
  No fork.
- **`(( a + b ))`** beats `expr` or `echo "$a + $b" | bc` for
  arithmetic. No fork.
- **Pipe-heavy scripts** are I/O bound; rewriting in Python or Awk
  is often the right call past ~200 lines.
- **`printf '%s\n'` over multiple `echo`** for many lines:
  `printf '%s\n' "${arr[@]}"`.
- **Avoid `cat file | cmd`.** UUOC ("useless use of cat"). Use
  `cmd < file` or `cmd file`.

## Strict-mode caveats

`set -euo pipefail` is the right default but has sharp edges:

- `set -e` is silenced inside conditions: `if cmd; ...`, `cmd || true`,
  `cmd && cmd2`, `! cmd`. This is intentional but can mask bugs.
- `set -e` does NOT propagate into command substitutions in older bash.
  `shopt -s inherit_errexit` (4.4+) fixes it.
- `set -u` aborts on unset arrays even when they should be empty.
  Use `"${arr[@]:-}"` to default to empty.
- `pipefail` makes a pipe's status the rightmost non-zero. A normal
  `cmd | head` will fail when `cmd` writes more than `head` reads
  (SIGPIPE = 141). Use `cmd | head || true` or restructure.

## Cross-platform

- `#!/usr/bin/env bash` (not `/bin/bash`). macOS bash is 3.2; users
  install bash 5 via brew.
- For maximum portability (POSIX), write `#!/bin/sh` and avoid
  `[[ ]]`, arrays, `${var,,}`, `local`, process substitution. POSIX
  shell is a much smaller language.
- Check bash version: `((BASH_VERSINFO[0] >= 4)) || { echo "bash 4+ required"; exit 1; }`.
- On Windows (Git Bash, WSL):
  - Forward slashes everywhere; the shell rewrites for native exes.
  - `MSYS_NO_PATHCONV=1` to disable path translation for one command.
  - Line endings: keep LF in scripts; CRLF causes `\r` in args and
    breaks shebangs.
- macOS BSD utilities differ from GNU: `sed -i ''` requires the empty
  string on BSD, omits it on GNU. Use `gsed` or wrap.

## ShellCheck

Run `shellcheck script.sh` on every file. Common rules to fix
(don't disable without reason):

- SC2086: double-quote variables.
- SC2046: word-splitting in command substitution.
- SC2155: assigning and declaring on one line hides errors.
  Use `local var; var="$(...)"`.
- SC2068: `$@` instead of `"$@"`.
- SC2206/SC2207: assigning command output to an array splits on
  whitespace; use `mapfile`.
- SC2164: `cd` without `||`. If it fails, the rest of the script
  ruins your day.

Disable inline with `# shellcheck disable=SC#### # reason`. Always
with a reason.

## Avoid

- `eval`. Almost always a shell injection bug.
- Backticks. Use `$(...)`.
- `cat file | grep x`. Use `grep x file` or `<file grep x`.
- Parsing `ps` / `ls` / `find` line-by-line without `-print0`.
- `[ $? -eq 0 ]`. Use `if cmd; then ...; fi`.
- Long inline scripts in CI YAML. Extract to `scripts/foo.sh` and
  call it; CI YAML lacks the editing affordances of a real script.
- `for x in $list`. Word splits, glob expands. Use arrays.
- `bash -c "cmd $var"`. Injection. Pass args, don't interpolate.
- Two passes of `sed` when one `awk` does it.
- Building strings with repeated `+=` in a tight loop. Use printf to
  a buffer or just collect in an array.
