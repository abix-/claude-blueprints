---
description: "Review a PR or issue against hard merge gates: acceptance checklist, compliance, docs, regression tests, benchmarks, DRY, spec verification. Works for both PRs and standalone issues."
argument-hint: "[issue N | PR-number | repo PR-number]"
disable-model-invocation: false
allowed-tools: Bash, Read, Grep, Glob, Edit, Write
version: "5.0"
---
Automated human-review workflow for agent PRs and issues. Multi-repo, workspace-aware.

Uses `endless-cli` for all BRP interaction.

## How to use

Invoke with one of these forms:

- `/review` -- auto-pick PR via `k3sc take`
- `/review 194` -- review PR `#194` in `endless`
- `/review k3sc 38` -- review PR `#38` in `k3sc`
- `/review issue 247` -- review issue `#247` in `endless` (no PR required)
- `/review issue k3sc 15` -- review issue `#15` in `k3sc`

### PR mode vs Issue mode

**PR mode** (default): review a PR with its linked issue. Checks out the branch, builds, runs tests, posts findings on the PR.

**Issue mode** (`/review issue N`): review a standalone issue that has no PR (e.g., audits, spec reviews, research tasks). Reads the issue body and comments, verifies acceptance criteria against the codebase, posts findings on the issue. Skips checkout/build/test steps.

## Selection

For PR auto-pick: `k3sc take` is the ONLY method. Do NOT manually pick from `gh pr list`.

- `k3sc take --worker claude-a` -- reserve next PR for worker `claude-a`
- `k3sc release --repo endless --pr 194` -- release reservation when done

`/review` is not read-only. It may update docs on the PR branch, commit review-only doc changes, post findings, and ask `Merge or skip?`.

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

### Acceptance criteria gate

NEVER recommend merge unless ALL acceptance criteria checkboxes are satisfied. This is the single hardest gate.

1. **Read the issue body** and find every `- [ ]` checkbox. These are the acceptance criteria.
2. **Verify every single item** against the actual code on the branch. Do not trust the implementer's self-assessment. Read the code, run the tests, confirm the behavior.
3. **If ANY criterion is unmet**, the PR is NOT ready:
   - Fix-forward if the missing item is small and in scope.
   - Otherwise, document it as a blocker and fail the review.
   - NEVER recommend merge with unmet criteria.
4. An issue with 11/12 acceptance criteria met is NOT ready for merge. 100% or nothing.
5. **If the issue has no checkboxes**, state this as a finding -- the PR cannot pass.

## Step 0: Parse arguments

`$ARGUMENTS` formats:

**PR mode** (default):
- No args: auto-pick via `k3sc take`, repo = all
- `N`: PR number, repo = `endless`
- `repo N`: PR number in specified repo

**Issue mode** (starts with `issue`):
- `issue N`: issue number, repo = `endless`
- `issue repo N`: issue number in specified repo

Repo mapping:
- `endless` -> `abix-/endless`
- `k3sc` -> `abix-/k3sc`

All `gh` commands must use `--repo {owner/repo}`.

Set `review_mode` to `pr` or `issue` based on parsing. This controls which steps run.

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
 11     in-game verification (optional)     launch game, BRP, GPU log -- only when relevant
 12     perf verification (if perf PR)      Criterion bench or BRP get_perf
 13     update performance.md (if perf PR)  add before/after Criterion numbers
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
 - in-game verification      only when change affects runtime behavior (rendering, GPU, gameplay)
 - scale benchmarks (50k)    if perf claims exist (prefer Criterion over BRP)
```

After printing the plan, immediately proceed to Step 1. Do NOT wait for confirmation.

## Step 1: Resolve target

**Issue mode**: use the issue number directly. Print `Reviewing issue #{N}: {title}`.

**PR mode**: if PR number given, use it directly. If no PR number, use `k3sc take`:

```bash
k3sc take --worker $(basename "$(pwd)")
```

If a repo arg was provided, add `--repo {repo-short-name}` to the command.

Parse the PR number and repo from `k3sc take` output and remember them for `k3sc release` at the end. If `k3sc take` returns nothing, stop and report `no PRs available`.

Do NOT fall back to `gh pr list` or manual priority ordering. `k3sc take` is the only selection method.

Print `Reviewing PR #{N}: {title}`.

## Step 2: Read metadata and checklists

**PR mode**: Read PR metadata, extract linked issue from branch name `issue-{N}`, read the linked issue with comments.

```bash
gh pr view {N} --repo {owner/repo} --json headRefName,title,body,labels
gh issue view {issue_N} --repo {owner/repo} --comments
```

If the branch name does not link to an issue, record a finding and fail the review.

**Issue mode**: Read the issue directly with comments.

```bash
gh issue view {N} --repo {owner/repo} --json title,body,labels --comments
```

Check if a PR exists for this issue: `gh pr list --repo {owner/repo} --head issue-{N} --state open`. If a PR exists, suggest switching to PR mode instead.

**Both modes**: Collect:
- Issue acceptance checklist items: all `- [ ]` and `- [x]` lines from issue body and comments
- PR checklist items (PR mode only): all `- [ ]` and `- [x]` lines from PR body
- Issue title, labels, and any explicit perf/test requirements

For `endless`, load these reference docs before review:
- `docs/authority.md`
- `docs/k8s.md`
- `docs/performance.md` for perf-related work

Treat those docs as review authority, not optional reading.

If the issue has no acceptance checklist items, record a finding: `issue lacks acceptance checklist`. Cannot pass.

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

## k3s agent mode

Detect k3s environment: cwd starts with `/workspaces/` or `JOB_KIND` env var is set.

In k3s mode, skip these steps (no GPU, no display, no shared target dir):
- Step 6: build and test (slow, no GPU for compute shaders)
- Steps 8-15: in-game verification (no display)
- Step 13: Criterion benchmarks (no valid baseline hardware)

k3s agents CAN do: read PR/issue metadata, read diffs, check compliance docs, verify acceptance criteria, check DRY/generalization, verify spec, check docs/changelog, post structured findings.

Note in the execution plan output which steps are skipped and why: "k3s mode: no build/test/BRP (no GPU/display)".

## Step 4: Fetch and checkout branch (PR mode only)

**Issue mode**: skip this step. Work against the current codebase on the base branch.

**PR mode**: each agent works in its own repo clone. Never cd elsewhere, never clone a new copy.

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

### Doc and changelog update (/done checklist)

If the PR is missing doc updates, changelog entry, or has stale docs:
1. Identify which docs in docs/ cover the changed systems (use docs/README.md file map)
2. Update affected architecture docs to match the new code
3. Add CHANGELOG.md entry if missing
4. Commit doc updates on the PR branch before continuing review

This is the same checklist as `/done` -- the reviewer applies it when the implementer missed it.

If you cannot map a changed code path to an acceptance item, required doc update, regression test, benchmark, or authority-doc requirement where required, record a finding.

### Compliance gate

**Read all three docs** at the start of every review:
- `docs/k8s.md` -- Def/Instance/Controller architecture
- `docs/authority.md` -- data ownership and source-of-truth rules
- `docs/performance.md` -- hot-path patterns, anti-patterns, review procedure

**Check every changed file** against these rules:
- **k8s.md**: base values come from registry Defs, never cached on instances. Adding a new variant = 1 enum + 1 registry entry. Systems read Def at spawn/reconcile time.
- **authority.md**: GPU-authoritative data is never used as hard gameplay gates. ECS wins over GPU readback for identity/ownership. Throttled readback fields are heuristic-only.
- **performance.md**: no O(n^2) in hot paths, no repeated scans, no nested membership checks, no unbounded debug cost. Follow the PR Review Procedure (section in performance.md) for every PR.

A PR that has not been checked against all three docs is not ready for merge, regardless of whether clippy and tests pass. If a violation is found, fix-forward or document as a blocker.

### Feature spec gate

If the issue is labeled `feature`:
- A spec doc must exist (linked from issue body under `## Spec Doc`), or the issue body must say "Spec: self-contained in issue body"
- Read the spec doc and verify the PR matches it 100%
- Approval without spec verification is invalid
- If the spec says X and the code does Y, that is a blocker
- Any unmet spec item is a blocker, not a "nice to have"

Bug and test issues are exempt -- the issue body is the spec.

### DRY and generalization check

Every review must check for DRY violations and missed generalization:

1. **DRY check**: look for duplicated logic, hardcoded lists of variants, or copy-pasted code paths that should be consolidated. If a PR adds a new variant by copying an existing block and changing names, flag it.
2. **Generalization check**: prefer extending specific systems into generic ones following k8s.md patterns. When a PR adds behavior for one specific BuildingKind, tower type, NPC job, etc., ask whether the logic should use `def.is_tower`, `def.some_field`, or a registry lookup instead of matching on specific enum variants. Goal: adding a new variant = 1 enum + 1 registry entry, not N match arms.
3. **Examples of violations**:
   - `match kind { BowTower => ..., CrossbowTower => ..., CatapultTower => ... }` when `def.tower_stats` already distinguishes them
   - Hardcoded `iter_kind(A).chain(iter_kind(B)).chain(iter_kind(C))` when a `def.is_X` flag or registry filter would future-proof it
   - A new system that duplicates logic from an existing system instead of parameterizing it
   - Copy-pasting a function with minor tweaks instead of adding a parameter
4. **Fix-forward when possible**: if the fix is small and clear, make it in the review. If large or design-ambiguous, document as a finding.

### Performance issue standards

For perf-related PRs (title, labels, or diff indicate optimization work):
1. Issue body must reference `docs/performance.md` and `docs/k8s.md` as mandatory reading
2. Acceptance criteria must include compliance verification against all three docs
3. Before/after metrics must document measurable improvement (timing, allocation count, or complexity reduction)
4. No new hot-path violations: any change touching hot paths must be verified against the anti-patterns list in performance.md

### Review findings priorities

Findings are the primary output. Prioritize:
- bugs
- regressions
- stale or missing docs
- violations of `authority.md`
- violations of `k8s.md`
- violations of `performance.md` for perf PRs
- DRY violations and missed generalization
- missing tests
- unsupported perf claims
- unchecked issue/PR checklist items

## Step 6: Run tests + regression test gate (PR mode only)

**Issue mode**: skip build and test steps. Verify acceptance criteria against the existing codebase instead.

**PR mode**: detect project type:
- Rust/Bevy: `k3sc cargo-lock test --release 2>&1`
- Go: `go test ./... 2>&1`
- Other: note `no test framework detected`

Record pass/fail and the relevant failing tests.

### Regression test gate

Every code change MUST have regression tests. No exceptions. No "will add later". No "it's too simple to test".

1. Every PR with code changes must include at least one test that would **FAIL if the change were reverted**. This proves the change is actually tested, not just that the code compiles.
2. **Bug fixes**: the test must reproduce the exact bug scenario and verify the fix. A test that only checks the happy path is NOT a regression test.
3. **New features**: tests must cover the core behavior described in acceptance criteria.
4. **Refactors**: tests must verify the refactored behavior matches the original.
5. **What counts**: a unit test, integration test, or ECS world test that sets up specific conditions and asserts the correct outcome.
6. **What does NOT count**: existing tests merely updated to compile with new API names. Renaming `set_occupancy` -> `set_present` in existing tests is mechanical, not a regression test.

If behavior changed and no regression tests exist in the diff, record a finding even if the existing suite passes. This is a **BLOCKER** -- fix-forward by writing the test, or fail the review.

## Step 7: Build

- Rust/Bevy: `k3sc cargo-lock build --release 2>&1`
- Go: `go build ./... 2>&1`
- Other: skip build

If build fails, skip BRP steps, post findings, and stop.

## Steps 8-15: In-game verification (optional)

BRP launch, GPU log, perf verification, and in-game benchmarks are **optional**. Only run them when the change is relevant to runtime behavior that cannot be verified by tests alone.

**Run in-game verification when:**
- the PR changes rendering, GPU shaders, or visual output
- the PR claims FPS or in-game timing improvements that need live measurement
- the PR changes gameplay behavior that BRP can verify (combat, building, NPC state)
- the human explicitly requests it

**Skip in-game verification when:**
- the change is dead code removal, refactoring, or code cleanup
- the change is test-only, doc-only, or CI/tooling
- the change is a scheduling/policy fix verified by unit tests
- the branch is far behind dev and BRP endpoints may not exist

When running in-game verification:

```bash
taskkill //F //IM endless.exe 2>/dev/null
k3sc cargo-lock run --release -- --autostart --no-raiders --farms=4 &
```

- `endless-cli test` for baseline BRP
- `endless-cli get_perf` for perf PRs
- Check `rust/target/release/wgpu_errors.log` for GPU errors
- Leave game running after review

For perf PRs with scale claims, run Criterion benchmarks locally (`k3sc cargo-lock bench`) rather than relying on in-game BRP with a tiny dev scene.

## Step 13: Update performance.md (perf PRs only)

For perf PRs, record benchmark results in `docs/performance.md` on the PR branch:
- Add or update the relevant system entry with before/after Criterion numbers
- Record the scale scenario explicitly, including counts used: NPCs, buildings, trees/rocks, or mixed-world counts
- Commit the update to the PR branch before posting the review comment

## Step 16: Verdict rules

`ready to merge` is allowed only if all required gates pass.

Hard failures:
- any unmet required gate from the Standard section
- failed `endless` BRP verification when BRP was run

If any hard failure exists, verdict is `needs work`.

## Step 17: Post review comment

**PR mode**: post on the PR. **Issue mode**: post on the issue.

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
 repo: {owner/repo}

 WHAT THIS PR DOES
 ──────────────────────────────────────
 {1-3 sentence plain-english summary of what changed and why}

 BEFORE -> AFTER
 ──────────────────────────────────────
 {concrete before/after description of the change -- what existed before, what exists after}
 {for code changes: old behavior -> new behavior}
 {for docs/assets: file didn't exist -> file added, or old content -> new content}
 {for perf PRs: old timing -> new timing with numbers}

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

For perf PRs, expand the BEFORE -> AFTER block with benchmark numbers:

```
 BEFORE -> AFTER
 ──────────────────────────────────────
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

## Step 19: Ask human -- merge/close or skip?

Ask only after posting the review comment.

### PR mode

**If merge**:
```bash
gh pr merge {N} --repo {owner/repo} --squash --delete-branch
gh issue close {issue_N} --repo {owner/repo}
```
If started with `k3sc take`: `k3sc release --repo {repo-short} --pr {N}`
Print `Merged PR #{N}, closed issue #{issue_N}`.

**If skip**: release reservation if any. Print `Skipped PR #{N} -- left as-is`.

### Issue mode

**If close**: `gh issue close {N} --repo {owner/repo}`. Print `Closed issue #{N}`.

**If skip**: Print `Skipped issue #{N} -- left as-is`.

## Rules

- All repo work happens inside the current working directory
- Do not delete local branches
- Do not modify issue labels
- If build fails, still post findings before stopping
- Findings must be the primary focus of the final review output
