---
description: Review a PR -- checkout, test, build, BRP verify, post findings, optionally merge
argument-hint: "[PR-number]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Edit, Write
version: "2.0"
---
Automated human-review workflow for agent PRs. Takes a PR number or auto-picks the oldest `needs-human` PR.

Uses `endless-cli` for all BRP interaction (it's in PATH).

## Step 1: Resolve PR

If `$ARGUMENTS` is a number, that is the PR number. Otherwise pick the oldest open `needs-human` PR:

```bash
gh pr list --label needs-human --state open --json number,headRefName,title --jq 'sort_by(.number) | .[0] | "\(.number) \(.headRefName) \(.title)"'
```

If no PR found, stop and report "no needs-human PRs open".

Print: `Reviewing PR #{N}: {title}`

## Step 2: Get PR metadata and linked issue

```bash
gh pr view {N} --json headRefName,title,body,labels
```

Extract the issue number from the branch name (pattern: `issue-{N}`). Read the issue:

```bash
gh issue view {issue_N} --comments
```

Note the acceptance criteria (any `- [ ]` or `- [x]` lines) and the issue title/labels for later.

## Step 3: Checkout branch

```bash
cd /c/code/endless && git fetch origin && git checkout {headRefName} && git pull
```

## Step 4: Run unit tests

```bash
cd /c/code/endless/rust && claude-k3 cargo-lock test --release 2>&1
```

Record pass/fail and test count. If tests fail, note failures but continue to build.

## Step 5: Build release

```bash
cd /c/code/endless/rust && claude-k3 cargo-lock build --release 2>&1
```

If build fails, skip BRP steps. Post findings with build failure and stop.

## Step 6: Kill existing game and launch

```bash
taskkill //F //IM endless.exe 2>/dev/null
cd /c/code/endless/rust && target/release/endless.exe --autostart &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

## Step 7: BRP baseline test

Run the built-in test suite which waits for BRP, then runs perf + summary:

```bash
endless-cli test
```

This prints `PASS` or `FAIL` at the end. Record FPS, NPC count, town state from the output.

## Step 8: Issue-aware BRP checks

Based on the issue title and labels, run targeted checks using `endless-cli`:

- **combat/squad/damage/health** keywords: `endless-cli perf` for faction stats, `endless-cli debug kind:squad index:0` to inspect a squad
- **build/building/farm/mine** keywords: `endless-cli build town:0 kind:Farm col:X row:Y` then `endless-cli summary`
- **UI/HUD/display/toggle** keywords: just verify stable FPS (>30) from test output
- **perf/performance/timing** keywords: `endless-cli perf` for detailed system timings
- **refactor** keywords: verify game state looks healthy from test output (towns exist, NPCs alive, FPS stable)
- **test** label: unit tests already ran; just confirm build + launch success
- **Default**: baseline test output is sufficient

Available `endless-cli` commands:
- `endless-cli summary` -- full game state
- `endless-cli perf` -- FPS, UPS, entity counts, timings
- `endless-cli debug {uid}` -- inspect entity by UID
- `endless-cli debug kind:squad index:N` -- inspect squad/town/policy
- `endless-cli build town:N kind:X col:N row:N` -- place building
- `endless-cli destroy town:N col:N row:N` -- remove building
- `endless-cli time paused:false time_scale:4.0` -- control time

Adapt test sequence intelligently based on what the PR actually changes.

**ALL functionality must be testable through BRP.** If any feature needed for verification is missing from BRP (building kind not in `parse_building_kind`, missing endpoint, missing debug field, etc.), **fix it immediately** on the branch -- add the missing BRP support to `rust/src/systems/remote.rs`, rebuild, and re-test. Do not flag gaps -- close them. A PR cannot pass review if its changes cannot be verified in a live game via BRP.

## Step 9: Leave game running

Do NOT kill the game. Leave it running so the human can manually inspect.

## Step 10: Post PR comment

Post a structured comment to the PR with all findings:

```bash
gh pr comment {N} --body "$(cat <<'COMMENT'
## /review findings

| Check | Result |
|-------|--------|
| Build | pass/fail |
| Unit tests | pass/fail (N passed, M failed) |
| BRP launch | pass/fail (Xs to ready) |
| FPS | X |
| NPCs | X |
| Issue-specific | [results] |

**Verdict**: ready to merge / needs work

[details of any failures or concerns]
COMMENT
)"
```

Replace all placeholders with actual values. Add detail for any failures.

## Step 11: Print verdict to console

Print the same summary to console so the human sees it immediately.

## Step 12: Ask human -- merge or skip?

Ask the human whether to merge this PR or skip it.

### If merge:

```bash
# Squash merge and delete remote branch
gh pr merge {N} --squash --delete-branch

# Close the linked issue
gh issue close {issue_N}

# Return to dev
cd /c/code/endless && git checkout dev && git pull
```

Print: `Merged PR #{N}, closed issue #{issue_N}, on dev branch`

### If skip:

Print: `Skipped PR #{N} -- left as-is`

## Rules

- Always use `endless-cli` for BRP calls -- never raw curl
- Always use `claude-k3 cargo-lock` for cargo commands (test, build, clippy)
- Always post the PR comment before asking merge/skip
- Do NOT delete local branches -- only remote via `--delete-branch`
- Do NOT modify issue labels (agents handle label transitions)
- If build fails, still post findings comment (with build failure noted) before stopping
- Game stays running after review -- human kills it when done
