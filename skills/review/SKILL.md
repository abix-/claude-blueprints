---
description: Review a PR -- checkout, test, build, BRP verify, post findings, optionally merge
argument-hint: "[repo] [PR-number]"
disable-model-invocation: true
allowed-tools: Bash, Read, Grep, Glob, Edit, Write
version: "3.0"
---
Automated human-review workflow for agent PRs. Multi-repo, workspace-aware.

Uses `endless-cli` for all BRP interaction (it's in PATH).

## Step 0: Parse arguments

`$ARGUMENTS` format: `[repo] [PR-number]` or just `[PR-number]`.

- If two args: `$0` is the repo short name, `$1` is the PR number
- If one arg and numeric: PR number, repo defaults to `endless`
- If one arg and not numeric: repo name, auto-pick oldest `needs-human` PR
- If no args: repo `endless`, auto-pick oldest `needs-human` PR

Repo mapping (short name -> GitHub owner/repo):
- `endless` -> `abix-/endless`
- `claude-k3` -> `abix-/claude-k3`

All `gh` commands must use `--repo {owner/repo}` to target the correct repo.

## Step 1: Resolve PR

If PR number given, use it. Otherwise pick the oldest open `needs-human` PR:

```bash
gh pr list --repo {owner/repo} --label needs-human --state open --json number,headRefName,title --jq 'sort_by(.number) | .[0] | "\(.number) \(.headRefName) \(.title)"'
```

If no PR found, stop and report "no needs-human PRs open".

Print: `Reviewing PR #{N}: {title}`

## Step 2: Get PR metadata and linked issue

```bash
gh pr view {N} --repo {owner/repo} --json headRefName,title,body,labels
```

Extract the issue number from the branch name (pattern: `issue-{N}`). Read the issue:

```bash
gh issue view {issue_N} --repo {owner/repo} --comments
```

Note the acceptance criteria (any `- [ ]` or `- [x]` lines) and the issue title/labels for later.

## Step 3: Clone/fetch repo and checkout branch

All repo work happens inside the current working directory. Clone if needed, otherwise fetch.

```bash
# If repo directory doesn't exist, clone it
REPO_DIR="$(pwd)/{repo-short-name}"
if [ ! -d "$REPO_DIR" ]; then
    git clone https://github.com/{owner/repo}.git {repo-short-name}
fi
cd "$REPO_DIR" && git fetch origin && git checkout {headRefName} && git pull
```

For endless, the repo may already be the current working directory (check for `rust/` dir). If so, use `.` instead of cloning.

## Step 4: Detect project type and run tests

Check what's in the repo to determine project type:

- **Rust/Bevy** (has `rust/Cargo.toml`): `cd {repo}/rust && cargo test --release 2>&1`
- **Go** (has `go.mod`): `cd {repo} && go test ./... 2>&1`
- **Other**: skip tests, note "no test framework detected"

Record pass/fail and test count. If tests fail, note failures but continue to build.

## Step 5: Build

- **Rust/Bevy**: `cd {repo}/rust && cargo build --release 2>&1`
- **Go**: `cd {repo} && go build ./... 2>&1`
- **Other**: skip build

If build fails, skip BRP steps. Post findings with build failure and stop.

## Step 6-8: BRP steps (Endless only)

These steps only apply when reviewing the `endless` repo. Skip entirely for other repos.

### Step 6: Kill existing game and launch

```bash
taskkill //F //IM endless.exe 2>/dev/null
cd {repo}/rust && target/release/endless.exe --autostart --no-raiders --farms=4 &
GAME_PID=$!
echo "Game PID: $GAME_PID"
```

Use `--no-raiders` unless testing combat. Use `--farms=4` for faster food accumulation.

### Step 7: BRP baseline test

```bash
endless-cli test
```

This prints `PASS` or `FAIL` at the end. Record FPS, NPC count, town state from the output.

### Step 8: Issue-aware BRP checks

Based on the issue title and labels, run targeted checks using `endless-cli`:

- **combat/squad/damage/health** keywords: `endless-cli get_perf` for faction stats, `endless-cli get_squad index:0` to inspect a squad
- **build/building/farm/mine** keywords: `endless-cli create_building town:0 kind:Farm col:X row:Y` then `endless-cli get_summary`
- **UI/HUD/display/toggle** keywords: just verify stable FPS (>30) from test output
- **perf/performance/timing** keywords: `endless-cli get_perf` for detailed system timings
- **refactor** keywords: verify game state looks healthy from test output (towns exist, NPCs alive, FPS stable)
- **test** label: unit tests already ran; just confirm build + launch success
- **Default**: baseline test output is sufficient

Available `endless-cli` commands (verb_noun convention):

**Read:**
- `endless-cli get_summary` -- full game state
- `endless-cli get_perf` -- FPS, UPS, entity counts, timings
- `endless-cli get_entity entity:{id}` -- inspect one NPC or building by entity ID
- `endless-cli get_squad index:N` -- inspect squad
- `endless-cli list_buildings town:N` -- list all buildings with entity IDs, growth, claimed/present
- `endless-cli list_npcs town:N job:Woodcutter` -- list NPCs with entity IDs, filter by town/job

**Create/Delete:**
- `endless-cli create_building town:N kind:X col:N row:N` -- place building
- `endless-cli delete_building town:N col:N row:N` -- remove building

**Update:**
- `endless-cli set_time paused:false time_scale:4.0` -- control time
- `endless-cli set_policy town:N eat_food:true` -- set town policies

**Actions:**
- `endless-cli apply_upgrade town:N upgrade_idx:N` -- apply upgrade
- `endless-cli send_chat town:N to:M message:hello` -- send chat
- `endless-cli recruit_squad` / `endless-cli dismiss_squad` -- squad management

Adapt test sequence intelligently based on what the PR actually changes.

**ALL functionality must be testable through BRP.** If any feature needed for verification is missing from BRP, **fix it immediately** on the branch -- add the missing BRP support to `rust/src/systems/remote.rs`, rebuild, and re-test.

## Step 9: Leave game running (Endless only)

Do NOT kill the game. Leave it running so the human can manually inspect. Skip for non-Endless repos.

## Step 10: Post PR comment

Post a structured comment to the PR with all findings:

```bash
gh pr comment {N} --repo {owner/repo} --body "$(cat <<'COMMENT'
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

Replace all placeholders with actual values. Omit BRP/FPS/NPC rows for non-Endless repos. Add detail for any failures.

## Step 11: Print verdict to console

Print the same summary to console so the human sees it immediately.

## Step 12: Ask human -- merge or skip?

Ask the human whether to merge this PR or skip it.

### If merge:

```bash
# Squash merge and delete remote branch
gh pr merge {N} --repo {owner/repo} --squash --delete-branch

# Close the linked issue
gh issue close {issue_N} --repo {owner/repo}

# Return to default branch in the local clone
cd {repo} && git checkout {default-branch} && git pull
```

Print: `Merged PR #{N}, closed issue #{issue_N}`

### If skip:

Print: `Skipped PR #{N} -- left as-is`

## Rules

- ALL repo work happens inside the current working directory -- never cd outside it
- Always use `--repo {owner/repo}` on all `gh` commands
- Always use `endless-cli` for BRP calls -- never raw curl
- Always post the PR comment before asking merge/skip
- Do NOT delete local branches -- only remote via `--delete-branch`
- Do NOT modify issue labels (agents handle label transitions)
- If build fails, still post findings comment (with build failure noted) before stopping
- Game stays running after review -- human kills it when done
