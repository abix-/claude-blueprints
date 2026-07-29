---
name: "advance-project"
description: "Use when continuing project development from design docs, authority maps, todo or issue backlogs, failing tests, changelogs, project state, or acceptance evidence; especially after repeated hotfixes, duplicated decision paths, stalled progress, or a request to choose and complete the next coherent batch."
---
# Advance Project

Preserve the operator's exact outcome in the operator's words.
Only measured product-state movement is progress.
Support artifacts are not milestones.

## Recover

After compaction or interruption, treat carried context as recovery evidence.
If governing instructions, matching skills, or required project records are not
fully current in context, read them again from disk. Read the repository's
owning design, priority, authority, todo or issue, changelog, project state, and
status records as applicable. Review recent history, status, relevant diffs,
and the newest acceptance evidence. Preserve uncommitted work.
Then resume the in-flight batch. Do not repeatedly reload unrelated material.

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

For a live system, diagnose from current live state through the repository's
permanent diagnostic and review path. Logs and reports explain history, but
historical evidence cannot prove current state. Use established project and domain terminology
in code, records, tests, status, and user-facing text.

## Record and prove

Record the root cause and coherent batch before production code. Update existing
records only:

- owning design doc: durable requirement and committed proof;
- authority map: canonical path, bypasses, current and target score;
- todo or issue: prioritized unfinished batch, root cause, and measured product
  acceptance without duplicating the repository's queue;
- project state: current focus, recovered context, and next action;
- status record: timestamped measured movement, verified work, and blockers;
- changelog: only verified shipped history.

The batch must name the operator outcome, current evidence, shared root cause,
owning boundary, every producer and consumer, known bypasses, current and target
score, and measured product acceptance. Do not create a parallel plan, status,
design, or history document.

Write and commit permanent failing behavioral proofs before production code.
Test intermediate authority flow and final behavior. Never use disposable
probes or change production behavior to satisfy an impossible fixture.

## Complete

Implement the entire shared change across every known consumer and delete the
competing paths. Run focused tests, broader relevant tests, and the repository
build. Re-audit bypasses, update existing records honestly, commit, and push.

Run one acceptance on the exact tested and pushed build through the
repository's canonical runner. Review the complete result and compare its
product measures with the prior accepted evidence. If the product goal does
not move, keep the same batch open, record the newly proved bypass, add its
failing proof, and continue. Do not hotfix and rerun.

Hooks may enforce only mechanical gates that can be proved from command input
and repository state. They must never decide priority, diagnosis, design, or
implementation, and must never block operator conversation. Instructions and
project records guide judgment.

When verified work proves a reusable workflow lesson, update the owning skill,
validate and push it, then resume the product batch. Skill maintenance never
replaces the product outcome.

Do not stop at analysis, tests, documentation, commits, builds, or harmless
failures. Stop only for operator direction, required external authority, a real
design conflict, unavailable required state, or proved acceptance.
