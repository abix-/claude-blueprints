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

`LOCAL-ONLY` is reported for skills only and never touched. A file you created
by hand is not the script's to delete. The config directories are the runtime's
own, holding caches, daemon state and credentials, so nothing there is reported
at all.

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

Every parameter defaults to doing everything, so bare `.\sync.ps1` checks all
kinds of file for both runtimes.

| Parameter | Default | Meaning |
|---|---|---|
| `-Action` | `check` | `check`, `install` or `resolve` |
| `-Runtime` | `all` | `claude`, `codex` or `all` |
| `-Include` | `all` | One or more of `skills`, `hooks`, `settings`, `instructions` |
| `-HomePath` | `$env:USERPROFILE` | Install somewhere other than the real profile, for a dry run against a scratch directory |

`-Include` decides by where a file lands, so the four kinds never overlap and
together they are the whole set:

| Kind | What it covers |
|---|---|
| `skills` | Anything under a `skills` folder, shared or runtime-specific |
| `hooks` | Anything under `hooks` |
| `settings` | `settings.json` and any other settings file |
| `instructions` | Everything else: `CLAUDE.md`, `failures.md`, `AGENTS.md` |

`Get-Help .\sync.ps1 -Full` prints all of this from the script itself.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Everything is `OK`, or the action completed with nothing left differing |
| 1 | Something differs, or a copy failed |
| 2 | Refused to run: a name conflict or a duplicate destination |

## Features, and how sure we are

Scored out of 10 on evidence, not intent. 10 means a test drove it and the
result was seen.

| Feature | Score | Evidence |
|---|---:|---|
| Skills, shared and per-runtime | 10 | Same 149 files and statuses as the tool it replaces, on the same profile |
| Config files: instructions, settings, hooks | 10 | Run against the real profile named exactly the files that were out of sync |
| Both runtimes | 10 | Claude installed 77 files, Codex 72, each re-checked clean and exiting 0 |
| `check` | 10 | Empty profile reports all MISSING and exits 1; a clean profile is all OK and exits 0 |
| `install` | 10 | Copied 77 files, and the check straight afterwards was clean |
| `resolve` | 10 | All six keys driven from input: `r` accepted the repo copy, `l` promoted a local edit and it landed in the repository, `d` re-showed the diff, `v` paged, `s` left both files differing, `q` quit, and an invalid key re-prompted |
| Compares by content | 10 | Same bytes reads OK regardless of timestamps |
| Says which side is newer | 10 | `LOCAL-NEWER`, `REPO-NEWER` and `DIFF` were each produced on purpose |
| Reports what only exists locally | 10 | A hand-made file reported `LOCAL-ONLY`, survived a second install untouched, and the config directories are excluded so no credential filename is ever printed |
| `-Include` by kind | 10 | Each kind alone, and `settings,hooks` together, returned exactly its own files; the four do not overlap and together they equal the default |
| Refuses a skill name conflict | 10 | Built the conflict, got one clear line and exit 2 |
| Refuses a duplicate destination | 10 | Built two sources landing on one file, got one line naming both and the destination, and exit 2 |
| Exit codes | 10 | 0, 1 and 2 all observed, including 1 from a copy that could not be written |
| `-HomePath` for a dry run | 10 | Every test above ran against a scratch profile |
| Built-in help | 10 | `Get-Help .\sync.ps1 -Full` renders name, syntax, description, every parameter and the examples |
| Never deletes | 10 | A hand-made file survived a second install, which copied nothing, and no delete call exists in the script |
| Status colours | 10 | The mapping is its own function and every status was checked through it, because colour does not survive a pipe |

One thing the table cannot claim: `-Include` with more than one value needs
PowerShell to parse the array, so `pwsh -File sync.ps1 -Include settings,hooks`
passes one string and is rejected by the parameter's own validation. Called as
`.\sync.ps1 -Include settings,hooks`, or through `pwsh -Command`, it works.

## Deliberately not included

**No delete.** Nothing on disk is ever removed, including `LOCAL-ONLY` files.
Removing a skill from the repository leaves it installed, and it is reported so
you can delete it yourself if that is what you meant.

**No timestamp-based copying.** Newer is reported so you can decide; it never
decides for you. A clock difference between machines is not a reason to
overwrite work.

**No automatic promotion.** `install` only goes repository to disk, and
`resolve` only moves a file after you answer for it.
