# claude-blueprints changelog

## Changelog rules

These rules govern `docs/changelog.md`. Follow them exactly.

- Each day has exactly one `## YYYY-MM-DD` heading and exactly one table below
  it.
- Every daily table has exactly three columns: `System`, `Item`, and
  `Done when`.
- Start every Item with `[x]`. No preceding hyphen. Changelog rows are
  completed work only.
- Never combine completed items into one changelog row. Never replace them with
  a summary paragraph, grouped bullet, status paragraph, or count.
- When recording a completed todo row, copy its System into `System`, its
  completed instruction into `Item`, and its completion proof into `Done when`.
- Verify the exact result before adding the changelog row.
- Leave unfinished work in the todo. Do not add partial work to the changelog.
- Put new days above older days. Add rows to the existing table when that date
  already exists. Never create a second heading or second table for the same
  day.
- Use plain terms. Do not use vague descriptions, invented language, or design
  essays.

## 2026-10-18

| System | Item | Done when |
|---|---|---|
| claude/settings.json | [x] Turn off idle compaction (compacting a long conversation while the terminal is idle) with idleCompaction false | claude/settings.json and ~/.claude/settings.json both parse and read idleCompaction false; claude --version 2.1.302, the key needs 2.1.290 or later |

## 2026-10-17

| System | Item | Done when |
|---|---|---|
| skills/rust | [x] Release is the build that runs; release profile is Cargo's defaults plus panic abort and opt-level 3, with the measured build times | onelog one-file release rebuild 52 s without LTO against 7m 22s with thin LTO, recorded in the skill's Build profiles table |
| skills/rust | [x] Say why release is the build that runs and why each release setting is set or left out (opt-level 3, panic abort, no LTO, no codegen-units 1, no strip), when the slow settings are right, and cargo build --timings | the skill's Build profiles section has the Why release paragraph and the Setting, What it does, Why table; ~/.claude/skills/rust/SKILL.md is the same file (cmp) (78e3e17) |
| skills/rust | [x] Say how to do a fast build and a release build for handing out: one fast release profile, the slow settings switched on for one command with CARGO_PROFILE_RELEASE_* into target/ship | the Build profiles section has the How to do each table with both commands; ~/.claude/skills/rust/SKILL.md is the same file (cmp) |
| skills/eufy, skills/debloat | [x] Point at the release eufy-capture build | both skills name target\release\eufy-capture.exe |

## 2026-10-11

| System | Item | Done when |
|---|---|---|
| `claude/settings.json` | [x] Keep Claude's project history 36500 days (100 years) instead of the default 30. | `cleanupPeriodDays` reads 36500 in `claude/settings.json` and the live `~/.claude/settings.json`, both parsed as JSON. |
| `claude/skills/one-owner` | [x] Put the live one-owner skill in the repo. | `claude/skills/one-owner/SKILL.md` is the same as `~/.claude/skills/one-owner/SKILL.md` (diff empty, sync.ps1 says OK). |
| `claude/CLAUDE.md` | [x] Never take screenshots of the operator's computer. | The absolute rules in `claude/CLAUDE.md` and the live `~/.claude/CLAUDE.md` both carry "NEVER take screenshots of the operator's computer". |
| `skills/debate` | [x] Fix debate's seven stale parts: no inbox step, reply to the message's from, delivery and silence, start the peer with open_agent, a useful first line, results into onelog todo rows and design docs, current examples. | grep finds no "check your incoming", `docs/debates`, old model names, `brain.md`, `factory_work` or Factorio in it (only "there is no inbox to check"), each new part is there, and it is installed for Claude and Codex. |
| `skills/onelog-orchestrator` | [x] Merge onelog-orchestrator and orchestration into one orchestrator skill, with review's merge checks in its report check. | `skills/orchestrator` holds both skills' rules (a line check against the old files found 39 lines not word for word: renamed headings, "game" made "program" for the one-writer rules, game checklist boxes made rules; none lost) and a "Checking a report" section from review's gates; both old folders are gone from the repo and the live folders, and grep finds nothing pointing at them (aa4f415). |
| `archive/` | [x] Deprecate the GitHub issue skills: move issue, n, review, approve and reject to archive/ and out of the live folders. | The five are under `archive/skills/`, gone from `~/.claude/skills` and `~/.agents/skills`, and help and the README say where they went (aa4f415). |
| `skills/advance-project` | [x] Retire advance-project: its recovery rule into onelog, its done rule into onelog-orchestrator, then delete it and its test. | `skills/advance-project` and `tests/test_advance_project_skill.py` are gone from the repo and both live skill folders, `grep -r advance-project` finds no file, and the onelog and onelog-orchestrator skills carry the two rules, installed live (15d8620). |
| `skills/onelog` | [x] Roll the todo, changelog, cleanup and done skills into the onelog skill with every detail kept, then remove the four. | A line check of the four old skills against the new onelog skill (3.4) found 29 lines not word for word, each one a renamed heading or "skill" made "section", no rule lost; the four are gone from the repo and both live folders, and grep finds nothing pointing at them (cea41ed). |
| `skills/onelog` | [x] onelog is updated when work starts and again when it is done (onelog skill 3.3). | The skill's "When onelog is updated" section says it, installed live (79dc307). |
