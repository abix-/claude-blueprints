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

## 2026-10-11

| System | Item | Done when |
|---|---|---|
| `claude/skills/one-owner` | [x] Put the live one-owner skill in the repo. | `claude/skills/one-owner/SKILL.md` is the same as `~/.claude/skills/one-owner/SKILL.md` (diff empty, sync.ps1 says OK). |
| `claude/CLAUDE.md` | [x] Never take screenshots of the operator's computer. | The absolute rules in `claude/CLAUDE.md` and the live `~/.claude/CLAUDE.md` both carry "NEVER take screenshots of the operator's computer". |
| `skills/debate` | [x] Fix debate's seven stale parts: no inbox step, reply to the message's from, delivery and silence, start the peer with open_agent, a useful first line, results into onelog todo rows and design docs, current examples. | grep finds no "check your incoming", `docs/debates`, old model names, `brain.md`, `factory_work` or Factorio in it (only "there is no inbox to check"), each new part is there, and it is installed for Claude and Codex. |
| `skills/onelog-orchestrator` | [x] Merge onelog-orchestrator and orchestration into one orchestrator skill, with review's merge checks in its report check. | `skills/orchestrator` holds both skills' rules (a line check against the old files found 39 lines not word for word: renamed headings, "game" made "program" for the one-writer rules, game checklist boxes made rules; none lost) and a "Checking a report" section from review's gates; both old folders are gone from the repo and the live folders, and grep finds nothing pointing at them (aa4f415). |
| `archive/` | [x] Deprecate the GitHub issue skills: move issue, n, review, approve and reject to archive/ and out of the live folders. | The five are under `archive/skills/`, gone from `~/.claude/skills` and `~/.agents/skills`, and help and the README say where they went (aa4f415). |
| `skills/advance-project` | [x] Retire advance-project: its recovery rule into onelog, its done rule into onelog-orchestrator, then delete it and its test. | `skills/advance-project` and `tests/test_advance_project_skill.py` are gone from the repo and both live skill folders, `grep -r advance-project` finds no file, and the onelog and onelog-orchestrator skills carry the two rules, installed live (15d8620). |
| `skills/onelog` | [x] Roll the todo, changelog, cleanup and done skills into the onelog skill with every detail kept, then remove the four. | A line check of the four old skills against the new onelog skill (3.4) found 29 lines not word for word, each one a renamed heading or "skill" made "section", no rule lost; the four are gone from the repo and both live folders, and grep finds nothing pointing at them (cea41ed). |
| `skills/onelog` | [x] onelog is updated when work starts and again when it is done (onelog skill 3.3). | The skill's "When onelog is updated" section says it, installed live (79dc307). |
