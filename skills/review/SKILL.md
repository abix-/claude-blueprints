---
description: "Review a PR against hard merge gates: linked issue acceptance checklist, PR checklist coverage, required docs, regression tests, benchmarks for perf work, authority/k8s/performance doc compliance, build/test/BRP verification, post findings, optionally merge"
argument-hint: "[repo] [PR-number]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Edit, Write
version: "4.8"
---
Automated human-review workflow for agent PRs. Multi-repo, workspace-aware.

Uses `endless-cli` for all BRP interaction.

## How to use

Invoke with one of these forms:

- `/review` -- auto-pick via `k3sc take`
- `/review 194` -- review PR `#194` in `endless`
- `/review k3sc` -- auto-pick via `k3sc take` for `k3sc` repo
- `/review k3sc 38` -- review PR `#38` in `k3sc`

## PR selection

`k3sc take` is the ONLY way to select PRs for review when no PR number is given. Do NOT manually pick from `gh pr list`.

- `k3sc take --worker claude-a` -- take next PR for worker `claude-a`
- `k3sc take --worker codex-a` -- take next PR for worker `codex-a`
- `k3sc release --repo endless --pr 194` -- release the reservation when the review is finished or abandoned

Worker names must start with `claude-` or `codex-`. Use the repo arg to filter by repo if provided.

`/review` is not read-only. It may update docs on the PR branch, commit review-only doc changes, post a PR findings comment, and then ask `Merge or skip?`.

## Standard

`/review` does not pass a PR on code quality alone. A PR can only pass if the implementation and the review artifacts both meet the checklist below.

Required before a PR can be marked `ready to merge`:

1. Linked issue exists.
2. Linked issue has explicit acceptance checklist items using markdown checkboxes.
3. PR body has explicit checklist coverage for the work:
   - linked issue reference
   - acceptance coverage
   - docs coverage
   - test coverage
   - benchmark coverage for perf work
4. Associated docs are updated for the change.
5. All PRs are checked against `authority.md` and `k8s.md`.
6. Perf PRs are checked against `performance.md`.
7. Any changed behavior has regression tests, or the absence is called out as a finding and the PR fails.
8. Any perf-sensitive change has scale benchmarks with before/after numbers, or the absence is called out as a finding and the PR fails.
9. All checked/unchecked issue and PR checklist items are reconciled during review. Unchecked required items are findings.

Scale benchmark means the test exercises the game at large-world size:
- 50k NPCs, or
- 50k buildings, or
- 50k trees/rocks (environment/resource instances), or
- a mixed scenario covering all of them

Prefer the mixed scenario when the changed system interacts with more than one of those populations. Small microbenchmarks are useful for diagnosis, but they do not satisfy `/review` on their own.

If the linked issue has no acceptance checklist, that is itself a review finding and the PR cannot pass `/review`.

## Step 0: Parse arguments

`$ARGUMENTS` format: `[repo] [PR-number]` or just `[PR-number]`.

- If two args: `$0` is the repo short name, `$1` is the PR number
- If one arg and numeric: PR number, repo defaults to `endless`
- If one arg and not numeric: repo name, auto-pick via `k3sc take`
- If no args: repo `endless`, auto-pick via `k3sc take`

Repo mapping:
- `endless` -> `abix-/endless`
- `k3sc` -> `abix-/k3sc`

All `gh` commands must use `--repo {owner/repo}`.

## Step 0.5: Print execution plan and wait for approval

Before taking a PR or doing any work, print the full execution plan. This lets the human verify the review will follow the correct process. Do NOT call `k3sc take`, `gh`, or any other command until the plan is approved.

The plan must show:
- parsed arguments (repo, PR number or "auto-pick")
- every step that will execute, in order, with the exact commands
- which gates are always required vs conditionally required
- what the final output will look like

Print the plan in this exact format:

```
## /review execution plan

 PARSED
 ──────────────────────────────────────
 repo:       {repo-short-name} ({owner/repo})
 PR:         #{N} or auto-pick via k3sc take
 worker:     {basename of cwd}

 STEP   ACTION                              COMMAND / DETAIL
 ──────────────────────────────────────────────────────────────────────────────
  1     resolve PR                          k3sc take --worker {worker}
                                            OR use PR #{N} directly
  2     read PR metadata                    gh pr view {N} --repo {owner/repo} --json ...
  3     read linked issue + comments        gh issue view {issue} --repo {owner/repo} --comments
  4     collect checklists                  extract - [ ] and - [x] from issue + PR body
  5     load authority docs                 read docs/authority.md, docs/k8s.md
                                            + docs/performance.md if perf PR
  6     build gate checklist                decide required vs conditional gates
  7     checkout branch                     git fetch origin && git checkout {branch} && git pull
  8     inspect diff against gates          read diff, map to acceptance items, docs, tests
  9     run tests                           k3sc cargo-lock test --release 2>&1
                                            OR go test ./... for k3sc
 10     build                               k3sc cargo-lock build --release 2>&1
                                            OR go build ./... for k3sc
 11     launch game (endless only)          endless.exe --autostart --no-raiders --farms=4
 12     baseline BRP (endless only)         endless-cli test
 13     GPU log (endless only)              check wgpu_errors.log
 14     issue-aware BRP (endless only)      endless-cli commands based on issue type
 15     perf verification (if perf PR)      capture numbers, compare to PR claims
 16     update performance.md (if perf PR)  add before/after Criterion numbers
 17     BRP gaps (endless only)             add BRP support if missing
 18     leave game running (endless only)   do not kill endless.exe
 19     verdict                             ready to merge / needs work
 20     post PR comment                     gh pr comment with findings table
 21     print local verdict                 rich terminal summary
 22     ask merge or skip                   wait for human decision

 REQUIRED GATES (always)
 ──────────────────────────────────────
 - linked issue with acceptance checklist
 - PR checklist coverage (issue ref, acceptance, docs, tests, benchmarks)
 - associated docs updated
 - authority.md compliance
 - k8s.md compliance
 - build passes
 - automated tests pass

 CONDITIONAL GATES
 ──────────────────────────────────────
 - regression tests          if behavior changed / bug fix / logic change
 - benchmark coverage        if perf-related (title, labels, diff, claims)
 - performance.md review     if perf-related
 - BRP verification          if endless repo
 - scale benchmarks (50k)    if perf claims exist

 STEPS SKIPPED FOR NON-ENDLESS
 ──────────────────────────────────────
 - steps 8-15 (game launch, BRP, GPU log, perf verification)
```

After printing the plan, ask: `proceed?`

Do NOT continue until the human responds. If the human says no or requests changes, adjust and reprint the plan.

## Step 1: Resolve PR

If PR number given, use it directly.

If no PR number given, use `k3sc take` to select the next PR. The worker name is the basename of the current working directory (e.g., if you are in `C:\code\claude-4`, the worker name is `claude-4`):

```bash
k3sc take --worker $(basename "$(pwd)")
```

If a repo arg was provided, add `--repo {repo-short-name}` to the command.

Parse the PR number and repo from `k3sc take` output and remember them for `k3sc release` at the end of the review. If `k3sc take` returns nothing, stop and report `no PRs available`.

Do NOT fall back to `gh pr list` or manual priority ordering. `k3sc take` is the only selection method.

Print `Reviewing PR #{N}: {title}`.

## Step 2: Read PR, issue, and checklists

Read PR metadata, including body and labels:

```bash
gh pr view {N} --repo {owner/repo} --json headRefName,title,body,labels
```

Extract the linked issue number from the branch name using `issue-{N}`. If the branch name does not link to an issue, record a finding and fail the review.

Read the linked issue including comments:

```bash
gh issue view {issue_N} --repo {owner/repo} --comments
```

Collect:
- Issue acceptance checklist items: all `- [ ]` and `- [x]` lines from the issue body and issue comments
- PR checklist items: all `- [ ]` and `- [x]` lines from the PR body
- Issue title, labels, and any explicit perf/test requirements

For `endless`, load these reference docs before code review:
- `docs/authority.md`
- `docs/k8s.md`
- `docs/performance.md` for perf PRs

Treat those docs as review authority, not optional reading.

If the issue has no acceptance checklist items, record a finding: `linked issue lacks acceptance checklist`. The PR cannot pass.

## Step 3: Build the review gate checklist

Before touching code, decide which gates are required.

Always require:
- linked issue acceptance checklist
- PR checklist coverage
- associated docs
- `authority.md`
- `k8s.md`
- build
- automated tests

Require regression-test coverage if:
- the PR changes behavior
- fixes a bug
- changes logic, data flow, state transitions, UI behavior, or remote interfaces

Require benchmark coverage if:
- the PR title, issue title, labels, or diff are perf-related
- the PR claims timing, FPS, allocation, query-count, or scaling improvements

Require `performance.md` review if the PR is perf-related.

For each required gate, decide whether the PR already contains the needed evidence. Missing evidence is a finding even before code inspection.

Examples of immediate findings:
- linked issue has no acceptance checklist
- PR body does not reference the linked issue
- PR body does not say how acceptance criteria were satisfied
- docs affected by the change were not updated
- change conflicts with `authority.md`
- change conflicts with `k8s.md`
- perf change conflicts with `performance.md`
- behavior changed but no regression tests were added or updated
- perf PR has no before/after benchmark numbers
- perf PR only has small synthetic benchmarks and no game-scale benchmark

## Step 4: Fetch and checkout branch

Each agent works in its own repo clone (the current working directory). Never cd elsewhere, never clone a new copy.

```bash
git fetch origin && git checkout {headRefName} && git pull
```

## Step 5: Inspect the diff against the gates

Read the diff and answer, explicitly:
- Which acceptance checklist items are implemented?
- Which associated docs should have changed, and did they?
- Does the PR comply with `authority.md`?
- Does the PR comply with `k8s.md`?
- If perf-related, does it comply with `performance.md`?
- Which files/tests cover each acceptance item?
- Which behavior changes have regression tests?
- Which perf claims have benchmark evidence?

Check associated docs before approving:
- README, docs, design notes, workflow docs, command/help text, inline usage docs, benchmark docs
- issue/PR checklist text that should be updated to reflect completed work

If docs are clearly missing or stale and the update is small and unambiguous, make the doc update on the PR branch, re-read the diff, and include that change in the review.

If you cannot map a changed code path to an acceptance item, required doc update, regression test, benchmark, or authority-doc requirement where required, record a finding.

Review findings are the primary output. Prioritize:
- bugs
- regressions
- stale or missing docs
- violations of `authority.md`
- violations of `k8s.md`
- violations of `performance.md` for perf PRs
- missing tests
- unsupported perf claims
- unchecked issue/PR checklist items

## Step 6: Run tests

Detect project type:
- Rust/Bevy: `k3sc cargo-lock test --release 2>&1`
- Go: `go test ./... 2>&1`
- Other: note `no test framework detected`

Record pass/fail and the relevant failing tests.

If behavior changed and no regression tests exist in the diff, record a finding even if the existing suite passes.

## Step 7: Build

- Rust/Bevy: `k3sc cargo-lock build --release 2>&1`
- Go: `go build ./... 2>&1`
- Other: skip build

If build fails, skip BRP steps, post findings, and stop.

## Step 8: Launch game (endless only)

Skip steps 8-13 for non-`endless` repos.

```bash
taskkill //F //IM endless.exe 2>/dev/null
k3sc cargo-lock run --release -- --autostart --no-raiders --farms=4 &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

## Step 9: Baseline BRP

```bash
endless-cli test
```

Record FPS, NPC count, town state, and PASS/FAIL.

## Step 10: GPU log

```bash
cat rust/target/release/wgpu_errors.log 2>/dev/null | tail -20
```

Any GPU validation error is a finding.

## Step 11: Issue-aware BRP checks

Use `endless-cli` based on the issue/PR:
- combat/squad/damage/health: `endless-cli get_perf`, `endless-cli get_squad index:0`
- build/building/farm/mine: create a building, then `endless-cli get_summary`
- perf/performance/timing: `endless-cli get_perf`
- default: baseline BRP is the minimum

For perf PRs, run these checks against a scale world, not a tiny dev scene.

## Step 12: Perf verification

For perf PRs, do not accept vague improvement claims.

You must capture concrete numbers and compare them to the PR claim:
- system timing
- query count / entity count / work size if relevant
- FPS if that is the claim

Required scale target for perf verification:
- use a workload with 50k NPCs, or 50k buildings, or 50k trees/rocks, or a mixed world that includes all of them
- choose the scale dimension that matches the changed system
- if the changed system is broad or interacts with multiple populations, require the mixed-world scenario
- if the existing perf harness cannot exercise the relevant 50k-scale case, that is a finding and the PR cannot pass

If the PR does not include before/after benchmark numbers, record a finding even if your spot-check looks better. Reviewer-collected numbers supplement the PR; they do not replace required PR benchmark evidence.

## Step 13: Update performance.md

For perf PRs, record benchmark results in `docs/performance.md` on the PR branch:
- Add or update the relevant system entry with before/after Criterion numbers
- Record the scale scenario explicitly, including counts used: NPCs, buildings, trees/rocks, or mixed-world counts
- Commit the update to the PR branch before posting the review comment

## Step 14: BRP gaps

All functionality needed for verification must be testable through BRP. If missing BRP support blocks verification, add it on the branch, rebuild, and re-test.

## Step 15: Leave game running

For `endless`, leave the game running after review.

## Step 16: Verdict rules

`ready to merge` is allowed only if all required gates pass.

Hard failures:
- any unmet required gate from the Standard section
- failed `endless` BRP verification where BRP is required

If any hard failure exists, verdict is `needs work`.

## Step 17: Post PR comment

Post a structured comment:

```bash
gh pr comment {N} --repo {owner/repo} --body "$(cat <<'COMMENT'
## /review findings

| Check | Result |
|-------|--------|
| Linked issue | pass/fail |
| Issue acceptance checklist | pass/fail |
| PR checklist coverage | pass/fail |
| Associated docs | pass/fail |
| authority.md | pass/fail |
| k8s.md | pass/fail |
| performance.md | pass/fail/not required |
| Build | pass/fail |
| Automated tests | pass/fail |
| Regression tests | pass/fail/not required |
| Benchmarks | pass/fail/not required |
| BRP launch | pass/fail/not run |
| Issue-specific verification | pass/fail/not run |

**Verdict**: ready to merge / needs work

### Findings
- finding 1
- finding 2

### Acceptance coverage
- issue item -> evidence or missing evidence

### Docs and policy coverage
- affected doc -> updated or missing
- authority.md -> compliant or violation
- k8s.md -> compliant or violation
- performance.md -> compliant or violation/not required

### Regression coverage
- changed behavior -> test or missing test

### Benchmark coverage
- perf claim -> benchmark or missing benchmark
- scale scenario used -> counts and whether it matches the changed system
COMMENT
)"
```

Rules for the comment:
- findings come first
- include every hard failure explicitly
- if there are no findings, say that explicitly
- do not say `ready to merge` unless every required gate passed

## Step 18: Print verdict locally

Print a rich, colorful summary to the terminal (wezterm). Use this format:

```
## /review PR #{N}: {short title}

 CHECK                        RESULT
 ─────────────────────────────────────
 ✅ Linked issue               #{issue}
 ✅ Acceptance checklist        N/N
 ✅ PR checklist                covered
 ✅ Docs                        updated / n/a
 ✅ authority.md                compliant
 ✅ k8s.md                      compliant
 ✅ performance.md              compliant / n/a
 ✅ Build                       pass
 ✅ Tests                       N/N pass
 ✅ Regression tests            N new
 ✅ Benchmarks                  Criterion verified / n/a
 ⏭️  BRP launch                 not run
 ✅ Perf verification           numbers confirmed / n/a

Use ✅ for pass, ❌ for fail, ⏭️ for skipped/not run.
```

**VERDICT: READY TO MERGE** or **VERDICT: NEEDS WORK**

For perf PRs, always include a before/after benchmark block:

```
BEFORE ({source}):
  scenario: 50k NPCs / 50k buildings / 50k trees-rocks / mixed
  avg:  Xus
  peak: Xus

AFTER (Criterion, reviewer-collected):
  scenario: 50k NPCs / 50k buildings / 50k trees-rocks / mixed
  scenario_1 .... Xus    (Nx faster)
  scenario_2 .... Xus
```

List regression tests with one-line descriptions of what they verify.
End with "Merge or skip?"

## Step 19: Ask human -- merge or skip?

Ask only after posting the review comment.

### If merge

```bash
gh pr merge {N} --repo {owner/repo} --squash --delete-branch
gh issue close {issue_N} --repo {owner/repo}
```

If this review started with `k3sc take`, release the reservation:

```bash
k3sc release --repo {repo-short-name} --pr {N}
```

Print `Merged PR #{N}, closed issue #{issue_N}`.

### If skip

If this review started with `k3sc take`, release the reservation:

```bash
k3sc release --repo {repo-short-name} --pr {N}
```

Print `Skipped PR #{N} -- left as-is`.

## Rules

- All repo work happens inside the current working directory
- Do not delete local branches
- Do not modify issue labels
- If build fails, still post findings before stopping
- Findings must be the primary focus of the final review output
