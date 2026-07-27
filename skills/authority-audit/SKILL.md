---
name: authority-audit
description: Use when the user asks for an "authority audit" of the jbot Rust code, when reviewing whether a subsystem has ONE canonical writer/reader, when scoring how close a system is to single-source-of-truth, or before/after refactors that consolidate duplicate writers. Produces per-system 1/10 scores against the authoritative path defined in docs/authority.md (rule 16).
---

# Authority audit

## What this is

A repeated, evidence-based audit that scores each jbot subsystem against the meta-rule from `C:/code/lotj/docs/authority.md` rule 16:

> Every operation has ONE authoritative implementation. Everything else reads through it or calls into it.

Each system gets a 1/10 score. 10/10 means EVERY caller of that operation goes through the one canonical path. 1/10 means there is no canonical path and every caller hand-rolls it.

This audit is run REPEATEDLY (after each consolidation pass) to measure progress toward single-source-of-truth.

## When to run

- User says "authority audit" or "audit authority" or "score the systems".
- After collapsing duplicate writers; verify the score moved.
- Before a refactor that adds a new path; check the existing one's score first.
- When choosing the next consolidation target; pick the lowest score with the highest blast radius.

## The systems to audit

Source: `docs/authority.md`. Rule 16's table is the seed list. Each row is one system. Add a row when a new mutation primitive emerges. Stable list as of 2026-06-02:

| # | System | Canonical path | Rule |
|---|---|---|---|
| 1 | Combat target | `combat::set_target` / stored target string | 1 |
| 2 | Live data reads | `state::current()` + queued capture | 2 |
| 3 | Current ship id | ship module setter (short id) | 5 |
| 4 | Default backpack | `inventory.backpack` var | 6 |
| 5 | Server-text triggers | `triggers.yaml` + `event::on` | 8 |
| 6 | Command contracts | `cmdschema.yaml` + `cmd_queue::run` | 8, 16 |
| 7 | Runtime settings (vars) | `var::register` / `var::get` / `var::set` | 11, 13 |
| 8 | Map coord / `.dat` writes | `mapper.rs` `set_room_user_data` (the ONE outbound `.dat` writer; galaxy setters call it) | 12 |
| 9 | Per-room state store | `crate::galaxy_map::room` (read) / `galaxy_map::set_*` (write). Migration COMPLETE 2026-06-02, Rule 14 6->10/10 | 14 |
| 10 | Outgoing MUD commands | `cmd_queue::send` / `cmd_queue::run` | 16 |
| 11 | Player gear table | equipment module per-slot list | 10 |
| 12 | Travel / movement | `go` composer + phase specialists | 17 |
| 13 | Tier-1 quests | `gather_quest` role + one row per spec | 18 |
| 14 | Operator CLI dispatch | `cli::dispatch` + spec `cli:` field | (implicit) |
| 15 | Op dispatch | `OP_REGISTRY.dispatch` | (implicit) |
| 16 | Inbound frames (pump) | `pump::server` POST `/frame` | (implicit) |
| 17 | Outbound frames (pump) | the 4-kind contract | (implicit) |
| 18 | Reset to boot | `lifecycle::reset_to_boot_state` | (implicit) |
| 19 | Times on the wire | `galaxy_map::EdtStamp` + `time::fmt_edt_now` (human EDT string; `from_storage` bridges legacy unix-ms) | 19 |

Add new rows when a new domain emerges; never delete rows.

## The score rubric

| Score | Meaning |
|---|---|
| 10 | Every caller goes through the canonical path. Anti-bypass enforced by code (compile-time, lint, or test). Zero ad-hoc raw sends/parses/writes. |
| 9 | Every caller goes through the canonical path; enforcement is convention only (no grep finds bypasses, but nothing prevents a future one). |
| 7-8 | One or two known bypasses, documented, with a planned fix. |
| 5-6 | Canonical path exists and most callers use it; meaningful minority hand-roll. |
| 3-4 | Canonical path exists but is one of several; competitors are roughly equal weight. |
| 1-2 | No canonical path, or the "canonical" path is itself a duplicate of another. |
| 0 | Domain is unowned. Do not use; if you see this, the audit is incomplete. |

## How to audit ONE system

For each row above, follow this procedure. Do NOT skip steps. Each step is evidence; the score is a synthesis, not a guess.

1. **Identify the canonical path.** Read the source file. Confirm the function/op exists and quote `file.rs:line`.
2. **List the expected callers.** Read the doc explaining what should call in (e.g. for triggers: every regex match; for `cmd_queue::send`: every server command).
3. **Grep for bypasses.** For each system there is a "bypass pattern" you grep for. Examples:
   - Triggers: `Regex::new\(` and `regex::Regex::new\(` outside `triggers.rs`.
   - `cmd_queue::send`: any `send` to the pump outbound channel that does not go through `cmd_queue`.
   - Vars: any `static` / `OnceLock` / `Mutex<HashMap` holding per-module config.
   - Per-room state: any `mapper::get_room_user_data` / `mapper::set_room_user_data` caller OUTSIDE `galaxy_map.rs`, or any module-local `HashMap`/`OnceLock` holding per-room state (the migration deleted `explorer.tried` / `door.retries` / `dig`). The `.dat` write itself still goes through `mapper::set_room_user_data` (Rule 12), but only via the `galaxy_map::set_*` setters.
   - Op dispatch: any direct fn call to an op's body instead of `OP_REGISTRY.dispatch`.
4. **Count the bypasses.** Each match is a candidate violation; read it to confirm. Note real ones with `file.rs:line`.
5. **Check anti-bypass enforcement.** Is there a compile-time guard? A test? A clippy lint? A doc-only rule? Score reflects enforcement strength.
6. **Score and justify.** Pick a score from the rubric. ALWAYS cite at least one piece of evidence per score point above 1 (a file:line, a count, a grep result).

## Definition of done (per system)

A system is "audited" only when ALL of these are recorded:

- [ ] Canonical path identified with `file.rs:line`.
- [ ] Expected callers listed (by category, not exhaustive).
- [ ] Bypass grep pattern run; raw count recorded.
- [ ] Confirmed bypasses listed with `file.rs:line` (or "none found").
- [ ] Enforcement mechanism stated (code / test / convention / none).
- [ ] Score 1-10 with one-line justification citing the evidence above.
- [ ] Next consolidation step named (or "system is at 10/10, no action").

A system is at **10/10 (done)** only when:

- [ ] Bypass grep returns 0 results across the crate.
- [ ] At least one anti-bypass guard exists (compile-time type, integration test, or `jbot/AGENTS.md` anti-bypass table entry that a reviewer enforces).
- [ ] The canonical path's doc-comment cites the authority rule it implements.

## Output format

Produce a markdown report at `C:/code/lotj/docs/audits/authority-YYYY-MM-DD.md` with this shape:

```markdown
# Authority audit YYYY-MM-DD

Prior audit: <link or "first run">.

## Summary table

| # | System | Score | Delta vs prior | Top action |
|---|---|---|---|---|
| 1 | Combat target | 8/10 | +1 | Collapse approach.rs raw kill |
...

## Per-system findings

### 1. Combat target -- 8/10

- Canonical path: `combat::set_target` at `src/modules/combat.rs:NNN`.
- Expected callers: every combat verb dispatcher.
- Bypass grep: `rg 'send.*"kill ' src/` -> 3 hits.
- Confirmed bypasses: `src/modules/approach.rs:142` reads enemy name from GMCP directly.
- Enforcement: doc rule only; no test.
- Justification: one known bypass, planned fix in todo.md.
- Next: move approach kill through `combat::engage`.

...repeated per system.

## Movement since last audit

- 3 systems improved (+1 each).
- 0 systems regressed.
- 1 new system added (row 18).
```

## Anti-patterns to reject

- Scoring from memory. Every score cites a grep or file read run THIS session.
- Scoring 10 because "I cannot think of a bypass." 10 requires a grep returning 0 AND an enforcement guard.
- Skipping a row because "I'm pretty sure it's fine." Run the grep.
- Inventing systems not in the table without first adding the row.
- Conflating "the path exists" with "everyone uses it." A path used by half the callers is at most 5/10.
- Padding the report with prose. The table + per-system bullets ARE the report.

## How to use the result

1. Pick the lowest-scored system with the highest blast radius (most callers).
2. Open ONE focused PR: collapse the duplicate writer per Rule 16 ("COLLAPSE, don't delete").
3. Re-run the audit. Confirm the score moved. Confirm no other system regressed.
4. Commit the new report to `docs/audits/`. The old report stays; the diff IS the progress record.
