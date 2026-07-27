---
name: cleanup
description: Use when a todo/backlog file has accumulated completed items that need to move into authoritative docs, or when asked to clean up, archive, or relocate "done" entries. Enforces move-then-verify-then-delete so no content is lost.
---

# Cleanup: Relocating Done Todos

## Overview

Completed items pile up in a todo file and must move to the authoritative doc that owns that topic. The danger is losing data: deleting from the source before confirming the destination really holds every line.

**Core principle: MOVE, then VERIFY, then DELETE. Three separate passes, in that order, never fused.** The source is the backup until verification passes. Deleting before verifying is the one mistake that destroys work permanently.

## Documentation layout standard (repo-wide, LOCKED)

Every authoritative doc MUST begin in this exact order:

1. **Authority statement** (the explicit `Authoritative for:` line near top).
2. `## Table of contents`.
3. **Definition of 10/10** (explicit section defining what 10/10 means for
   that doc's domain).
4. **Ratings table** (authority-style scored table).

No alternate ordering is allowed. Do not insert other top sections between
these blocks.

This is NOT a generic "Doc/Status/Top action" scaffold. Every doc must have
domain-specific scored rows with evidence and concrete next actions.

Canonical ratings table design (must match `docs/authority.md` style):

| # | Item | Score | Evidence snapshot | Top action |
|---|---|---|---|---|
| 1 | <domain item> | X/10 | <why this score, current evidence> | <next concrete move> |

Design consistency rules (non-optional):

- Use this exact five-column order: `#`, `Item`, `Score`, `Evidence snapshot`,
  `Top action`.
- `Score` values are always rendered as `X/10` (never plain `X`, never `%`,
  never badges).
- Keep the ratings table at the top block immediately after the 10/10 definition.
- Keep separator style exactly: `|---|---|---|---|---|`.
- Use concise, evidence-first phrasing in `Evidence snapshot`; use one concrete
  next move in `Top action`.
- If a doc has one whole-system row, still keep this same table shape.

Required scoring doctrine:

- **10/10 has one strict definition (LOCKED):**
  the system can run for **365 continuous days** for **10 concurrent players**
  with **no Rust updates required**. The only allowed changes are **per-player
  YAML configuration/preferences**. Every relevant path is handled correctly,
  and there are **ZERO known bugs**.
- Anything short of that bar is below 10/10.
- **1/10** means effectively unowned / nonfunctional in intended scope.
- Scores in between must be justified by observable evidence in that doc's
  domain (tests, audits, known regressions, rollout status, operator outcomes).
- Low scores are encouraged when uncertain; do not inflate.

When cleanup relocates or normalizes docs, preserve or upgrade targets to this
exact locked shape and order. Do not invent per-file variants, and do not bury
ratings in body prose.

When writing each doc's `## Definition of 10/10`, use this strict definition
verbatim in domain-adapted wording (same meaning, no weaker phrasing).

## What belongs in a todo (the one rule)

A todo is a list of CONCRETE TASKS and nothing else. Exactly two things belong in it:

1. **Concrete tasks.** "Do X": something a person can pick up and check off (build / add / fix / port / wire / delete X).
2. **Links to the CONTEXT** for those tasks: a pointer to the authoritative doc that holds the design, status, or background.

EVERYTHING else is DATA, and ALL data gets folded into whatever authoritative doc owns it, then replaced in the todo by a task line plus a link. Data includes: design / doctrine / rationale, status / root-cause / "current state", tables, dashboards, coverage snapshots, captured logs, code-shape sketches, examples, verbatim quotes, narrative. None of it lives in the todo.

This is the ONE rule the rest of this skill implements. Design documentation goes to its subject doc; status goes to `status.md`; a doctrine block with a thin actionable core leaves the doctrine in the doc and keeps the task. Same move every time: fold the data into the authoritative doc, leave the task plus a link.

**Data that already has an authoritative home is not re-relocated; it is DROPPED and linked.** A table hand-copied from a machine source (e.g. a coverage dashboard duplicating `playbook_coverage.yaml` and its printer) is a stale-prone duplicate. Delete it from the todo and replace it with a one-line pointer to the source of truth. Do not relocate the snapshot anywhere; the registry already is the record.

## The Three Passes

Run them as distinct steps. Do not delete anything in the same pass that moves it.

### 1. MOVE (copy into the authoritative doc)
- Identify each completed block, ONE at a time. `## Done (date)` headings are the obvious case. A `## P0` / doctrine block counts as completed only when its named artifact (module/file, predicate, verb, field) actually EXISTS and is WIRED in the code: grep the source (e.g. `jbot/src`) for the symbol and confirm real callers. Judge done-ness against the code, never by reading the block's prose.
- For each block, choose the authoritative target doc and a stable anchor slug.
- Append the **full body, verbatim** to the target under a clearly marked section (e.g. `## Relocated from todo.md`, block headed `### <anchor>`). No breadcrumb needed; the anchored section IS the record.
- Dedupe by anchor so re-runs are idempotent (skip if the anchor already exists).
- **Do not touch the source todo in this pass.**

### 2. VERIFY (confirm nothing was lost)

**This is the mandatory standard for every cleanup. Two independent checks per block; both must pass before that block is eligible to delete.**

- **Source of truth for the original body:** if the working tree already holds stubs or partial content (a prior MOVE pass ran), you CANNOT reconstruct the original from a stub. Read the full original body from git: `git show HEAD:<source-file>`. The committed version is the source of truth, not the working tree.
- **Check A: line presence.** Every non-empty line of the original body must appear in the target doc. Mechanical, not eyeballed (see the script below). Report the count of missing lines; nonzero means NOT safe.
- **Check B: anchored landing exists.** The block must live under a real `## Relocated from todo.md` section at a concrete `### <anchor>`. Confirm the anchor heading physically exists in the target (e.g. grep `^### <anchor>`) and report its `file:line`. "Lines appear somewhere in the file" is not enough; the section must exist.
- Confirm the target docs are saved (and committed, if the workflow commits before delete).
- Build the explicit delete-safe list: only blocks that pass BOTH checks. Print a `N delete-safe, M NOT safe` summary.
- Any block with no target mapping, a failing line check, or a missing anchor stays in the source and gets reported. Never delete it.

### 3. DELETE (remove from the source)
- Remove only the verified blocks from the todo.
- Re-read the todo afterward: confirm exactly the intended blocks are gone and surrounding structure is intact.

## Partial blocks (shipped core + open follow-ups)

A doctrine block is often mostly shipped with a few open follow-ups (e.g. a P0 block whose "out of scope this round" list is still real work). Do not keep the whole block, and do not lose the follow-ups:

- **MOVE** the full shipped doctrine body to the authoritative doc. The design is now reality; the doc is its record.
- In the **DELETE** pass, replace the source block with a small residual block holding ONLY the still-open follow-ups, plus a one-line pointer to the relocated record. This is a trim, not a full delete; the open items stay tracked in the todo.
- VERIFY still runs on the moved body (both checks) before the trim.

## Design documentation is not a todo item

A todo file should hold a concrete, actionable list: things that can be DONE and checked off. Much of what accumulates in a long-lived todo is not that. It is DESIGN DOCUMENTATION: doctrine, locked decisions, rationale ("why P0"), operator-verbatim context, code-shape sketches, architecture. That belongs in the owning design / subject doc, not the todo, whether or not the related work is finished.

Classify each block by what it mostly IS, not by its done/open status:

- **Actionable item.** "Do X": a checkbox, a concrete deliverable, a named gap to close. Stays in the todo.
- **Design documentation.** Doctrine, rationale, decisions, "why", verbatim quotes, code shape. Belongs in the design doc.

Most long doctrine blocks are a thin actionable core wrapped in thick design prose. Handle them like a partial block, but split on TYPE rather than on shipped-vs-open:

- **MOVE** the design documentation (doctrine, rationale, decisions, code-shape) to the owning design doc under `### <anchor>`. It is the authoritative record of the design, done or not.
- VERIFY (both checks) on the moved prose.
- **DELETE** down to a residual that is ONLY the concrete actionable items still to do, plus a one-line pointer to the design doc. If nothing actionable remains, the block leaves the todo entirely.

The test for any residual line: can someone DO it and check it off? If it explains or justifies, it is documentation, so move it. If it is "build / add / fix / port X", it is a todo, so keep it.

## Status and root-cause analysis go in ONE living status.md

A todo holds actionable items. It is NOT the place for "current state" narrative: root-cause analysis, investigation write-ups, "honest status", "what shipped / what didn't", "where we are now". That is STATUS, and status belongs in ONE living `status.md` that explains the current state and what we are doing.

`status.md` is handled DIFFERENTLY from every other relocation in this skill, and the difference is the whole point:

- **Every other target is append-only** (the doctrine lands under a new `### <anchor>` and accumulates; old records are kept). **status.md is the opposite: ONE file, UPDATED in place.** It is a single snapshot of current reality, not a log. You REWRITE the relevant section so it reflects what is true now, and you REPLACE stale status rather than appending a new dated entry beside it.
- So status does NOT go through the append-under-anchor MOVE path or the generic tool. You edit `status.md` directly: fold the current-state facts in, overwrite any line that is now stale, then remove the status/root-cause prose from the todo.
- If `status.md` does not exist yet, create it once (current state + what we are doing now); thereafter only update it.
- VERIFY for status is "is the current fact represented in status.md?", not a verbatim line-check (you are condensing and replacing, not copying). Confirm the fact is there, then delete from the todo.

A root-cause write-up that is still being chased is the tricky case: the FINDINGS and current state go in status.md; the remaining concrete fix steps stay in the todo as actionable items. Same split as design documentation, just routed to status.md instead of a subject doc.

**Every subsystem in status.md carries a brutal X/10 maturity rating.** 10/10 means complete: it needs NO updates for 10 years AND is perfect, free of ALL bugs. Anything actively churning, missing features, or carrying a single open bug is well below 10. Rate honestly and low. The rating exists to show where the real risk is, not to flatter. Put a one-line legend at the top of status.md and the rating on each subsystem (e.g. `## Pump transport -- 3/10`), and re-rate when you update that subsystem's state.

**status.md is the status of the ENTIRE system, not just the parts with todo entries.** Cover every runtime component and subsystem (for jbot: the pump / transport, the brain and each of its subsystems, the Discord bot). A component with zero open todo items still gets a section and a rating, so the doc is a complete health board at a glance, never a partial view skewed toward whatever happened to be in the backlog.

## Verification Check (the standard, do this, don't eyeball)

```python
import re, subprocess

# Original body comes from the committed source, not the stubbed working tree.
def head_body(source_path: str) -> str:
    return subprocess.run(["git", "show", f"HEAD:{source_path}"],
                          capture_output=True, text=True, check=True).stdout

# Check A: every meaningful line of the original body landed in the target.
def lines_present(body: str, target_text: str) -> list[str]:
    return [ln for ln in body.splitlines()
            if ln.strip() and ln.strip() not in target_text]  # empty == pass

# Check B: the anchored landing section physically exists in the target.
def anchor_exists(anchor: str, target_text: str) -> bool:
    return re.search(rf"^### {re.escape(anchor)}\b", target_text, re.M) is not None
```

A block is delete-safe ONLY when `lines_present(...)` is empty AND `anchor_exists(...)` is True. Report a per-block `OK / MISSING / NO ANCHOR` line and a `N delete-safe, M NOT safe` total. This logic belongs in the ONE generic cleanup tool (see below), not an ad-hoc one-liner per cleanup.

## Recurring cleanups: one keyed registry, not ad-hoc targets

When the same todo gets trimmed repeatedly (a living backlog), do not re-specify each block's target doc, anchor, and residual fresh every run. Promote them into ONE keyed registry that all three pass-scripts read, so MOVE, VERIFY, and DELETE can never disagree about where a block went or what to leave behind.

Shape (one entry per block, keyed by a short slug):

```python
BLOCKS = {
    "lifecycle": {
        "needle":   "framework owns the playbook lifecycle",   # locates the block in the source
        "doc":      "data-flow-and-authority.md",              # authoritative target
        "anchor":   "framework-lifecycle-doctrine-2026-05-30", # stable landing slug
        "note":     "Shipped 2026-05-30: ... Authoritative design record.",
        "residual": "## OPEN ... only the still-open follow-ups",  # "" means full delete
    },
    # ... one entry per completed block
}
```

- **MOVE** extracts by `needle`, appends the body to `doc` under `### {anchor}` (dedupe by anchor, idempotent).
- **VERIFY** checks every body line is present in `doc` and that `### {anchor}` physically exists.
- **DELETE** replaces the source block with `residual` (the partial-block trim from the section above, pre-declared in the registry so it is reviewed up front, not improvised), or removes the block entirely when `residual` is `""`.

One registry means each block's target and anchor are declared once and reused by all three passes; a per-run ad-hoc target list is exactly how MOVE and VERIFY drift apart. Invoke per block by key so each block is still handled ONE at a time, satisfying the one-block-per-pass rule.

**ONE generic tool, not a script per cleanup.** All of this (block relocation, partial-block trim, completed-`[x]`-item relocation) is the SAME move/verify/delete shape over two selectors: a `## ` heading block, or the `- [x]` lines inside a section. Do not write a fresh `relocate_*.py` / `verify_*.py` / `delete_*.py` per cleanup; that breeds a pile of near-duplicates. Put it in ONE generic tool keyed by job, e.g. `scripts/cleanup.py <key> <move|verify|delete|run>` with `kind: "block" | "checked"`. Adding a cleanup is one registry entry, never a new script. Past jobs are recorded by their `### <anchor>` sections in the target docs, so the registry only needs current jobs.

## Common Mistakes

- **Fusing move and delete in one edit.** A crash or a bad replace then loses the block with no copy anywhere. Keep the passes separate.
- **Eyeballing instead of line-checking.** Truncated or reflowed paste looks fine at a glance. Run the line check.
- **Deleting a block that had no target mapping.** No mapping means it was never relocated. Leave it and report it.
- **Anchor collision overwriting an earlier relocation.** Dedupe by anchor; never silently replace.
- **Careless text replacement that shifts offsets.** Match on the full block text, not line numbers.
- **Treating design documentation as a todo.** Doctrine, rationale, decisions, and "why" belong in the design doc; the todo keeps only concrete actionable items. Leaving a 200-line doctrine block in the todo because "some work is still open" keeps documentation in the wrong file.
- **Treating status / root-cause analysis as a todo.** "Honest status", "what shipped", investigation write-ups, and "current state" notes are status, not actionable items. They go in `status.md`.
- **Appending status instead of updating it.** Status is ONE current snapshot. Appending dated status entries (or relocating it under a `### <anchor>` like a doctrine) recreates the log-in-the-wrong-place problem. Update `status.md` in place and replace stale lines.
- **Leaving DATA in the todo.** Tables, dashboards, coverage snapshots, captured logs, examples, narrative: none of it is a task. Fold it into the authoritative doc (or drop + link if a registry already owns it). The todo keeps concrete tasks + context links only.

## Red Flags (STOP)

- "I'll just move and delete in the same step."
- "It clearly landed, I don't need to check each line."
- "This block has no obvious home, I'll delete it anyway."
- "The script said OK so it's fine." Verify the destination yourself before any delete.
- "This block is still open, so it stays." Open status does not make design prose a todo. Move the doctrine to the design doc; keep only the actionable items.
- "This is a root-cause write-up / honest status / current state." That is status, not a todo. Fold it into `status.md` (update in place, replace stale lines) and remove it from the todo; keep only the remaining fix steps.
- "This table / dashboard / example / capture is useful here." It is DATA, not a task. Fold it into the authoritative doc, or drop it and link if a registry already owns it. The todo keeps only concrete tasks + context links.

All of these mean: do not delete yet. Re-run VERIFY first.
