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
| `skills/advance-project` | [x] Retire advance-project: its recovery rule into onelog, its done rule into onelog-orchestrator, then delete it and its test. | `skills/advance-project` and `tests/test_advance_project_skill.py` are gone from the repo and both live skill folders, `grep -r advance-project` finds no file, and the onelog and onelog-orchestrator skills carry the two rules, installed live (15d8620). |
| `skills/onelog` | [x] onelog is updated when work starts and again when it is done (onelog skill 3.3). | The skill's "When onelog is updated" section says it, installed live (79dc307). |
