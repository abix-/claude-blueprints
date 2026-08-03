# Sync

One script owns installing and comparing this repository against the live
runtime directories. It replaces all four of the current tools.

## What it replaces

| Today | Language | Covers | Can it copy? |
|---|---|---|---|
| `sync-check.py` | Python | Both runtimes, skills and config | Repo to disk only |
| `install.ps1` | PowerShell | Nothing; wraps the Python | Through the Python |
| `sync-check.sh` | Bash | Claude skills only | No |
| `sync-resolve.sh` | Bash | Claude skills only | Both directions, interactive |

Two of them answer "what is out of sync" and disagree. The Bash pair does not
know that `CLAUDE.md`, `failures.md`, `settings.json` or hooks exist, so it
reports everything clean while four files differ. The Python cannot promote a
local edit back into the repository, which is the only reason the Bash pair is
still here.

## What the one script does

### Covers

Both runtimes. Every file, not just skills.

| Source in the repository | Lands on disk at |
|---|---|
| `skills/<name>/` | `~/.claude/skills/<name>/` and `~/.agents/skills/<name>/` |
| `claude/skills/<name>/` | `~/.claude/skills/<name>/` |
| `codex/skills/<name>/` | `~/.agents/skills/<name>/` |
| anything else under `claude/` | `~/.claude/` at the same relative path |
| anything else under `codex/` | `~/.codex/` at the same relative path |

So `claude/CLAUDE.md` becomes `~/.claude/CLAUDE.md`, and
`claude/hooks/Hook-PreToolUse-Blocked.py` becomes
`~/.claude/hooks/Hook-PreToolUse-Blocked.py`.

### Compares

Content decides whether a file differs: same bytes means identical, and
timestamps are never used for that decision. Timestamps are read only when the
contents already differ, to report which side was edited more recently, which is
what tells you whether the change was yours or the repository's.

Per file, one of:

| Status | Meaning |
|---|---|
| `OK` | Same bytes on both sides |
| `MISSING` | In the repository, not on disk |
| `LOCAL-ONLY` | On disk, not in the repository |
| `REPO-NEWER` | Differs, and the repository copy was edited more recently |
| `LOCAL-NEWER` | Differs, and the copy on disk was edited more recently |
| `DIFF` | Differs, and both were edited at the same moment |

`LOCAL-ONLY` is reported and never touched. A file you created by hand is not
the script's to delete.

### Actions

**`check`** compares and prints the table. Changes nothing. Exits non-zero if
anything is not `OK`, so it can gate a commit or a pipeline. This is the
default when no action is given.

**`install`** copies the repository over the live files. Skips anything already
identical, so a re-run after a clean install copies nothing. Reports how many
files it wrote. This is the one-directional path: the repository wins.

**`resolve`** walks each differing file one at a time, shows the diff, and asks
what to do with it:

| Key | Action |
|---|---|
| `l` | Copy local to the repository, promoting a hand edit into git |
| `r` | Copy the repository over local, accepting the committed version |
| `d` | Show the diff again |
| `v` | Show it side by side in a pager |
| `s` | Skip this file and decide later |
| `q` | Quit, leaving everything untouched from here on |

Resolve is the only way a local edit gets back into the repository, and it never
acts without an answer for that exact file.

### Refuses to run when

- A runtime skill has the same name as a shared skill. One of them would win
  silently and the other would vanish.
- Two source files would land on the same destination. Same reason.

Both are reported with the names involved, and nothing is copied.

### Parameters

| Parameter | Default | Meaning |
|---|---|---|
| `-Action` | `check` | `check`, `install` or `resolve` |
| `-Runtime` | `all` | `claude`, `codex` or `all` |
| `-HomePath` | `$env:USERPROFILE` | Install somewhere other than the real profile, for a dry run against a scratch directory |

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Everything is `OK`, or the action completed with nothing left differing |
| 1 | Something differs, or a copy failed |
| 2 | Refused to run: a name conflict or a duplicate destination |

## Deliberately not included

**No delete.** Nothing on disk is ever removed, including `LOCAL-ONLY` files.
Removing a skill from the repository leaves it installed, and it is reported so
you can delete it yourself if that is what you meant.

**No timestamp-based copying.** Newer is reported so you can decide; it never
decides for you. A clock difference between machines is not a reason to
overwrite work.

**No automatic promotion.** `install` only goes repository to disk, and
`resolve` only moves a file after you answer for it.
