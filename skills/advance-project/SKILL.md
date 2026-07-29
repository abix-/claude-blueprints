---
name: "advance-project"
description: "Use when continuing project development from design docs, authority maps, todo or issue backlogs, failing tests, changelogs, project state, or acceptance evidence; especially after repeated hotfixes, duplicated decision paths, stalled progress, or a request to choose and complete the next coherent batch."
---
# Advance Project

Move the operator's product goal, not the support workflow.

## Recover

Read current governing instructions, project state, priority and authority
records, relevant owning design docs, recent history and diffs, and the newest
runtime or acceptance evidence. Preserve uncommitted work. Do not repeatedly
reload unrelated material.

## Select

Group symptoms, failing tests, todo items, and live findings by shared design
or authority cause. Choose the largest actionable group with the greatest
verified impact on the operator's goal. Priority comes from the operator, then
the repository's current queue, never from recency, ease, raw failure count, or
lowest score.

## Investigate

Trace the group through inputs, state, dependencies, decisions, completion, and
consumers. Separate real defects from stale tests and impossible fixtures.
Identify every competing writer or decision path and the one boundary that
must own the rule.

One fact or decision has one authority. Observers provide facts, the authority
decides, and executors apply selected work without rediscovering intent.

## Record and prove

Update existing records only:

- owning design doc: durable requirement and committed proof;
- authority map: canonical path, bypasses, current and target score;
- todo or issue: unfinished coherent batch and acceptance;
- project state: current focus and next action;
- changelog: only verified shipped history.

Write and commit permanent failing behavioral proofs before production code.
Test intermediate authority flow and final behavior. Never use disposable
probes or change production behavior to satisfy an impossible fixture.

## Complete

Implement the entire shared change across every known consumer and delete the
competing paths. Run focused tests, broader relevant tests, and the repository
build. Re-audit bypasses, update existing records honestly, commit, and push.

Run one acceptance on the exact tested build. Review the complete result. If
the product goal does not move, keep the same batch open, record the newly
proved bypass, add its failing proof, and continue. Do not hotfix and rerun.

Do not stop at analysis, tests, documentation, commits, builds, or harmless
failures. Stop only for operator direction, required external authority, a real
design conflict, unavailable required state, or proved acceptance.
