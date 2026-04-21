---
name: issue
description: Create, claim, and work GitHub issues across project repos (abix-/endless, abix-/k3sc). Use when the user invokes `issue` with an explicit issue number, wants the next eligible issue claimed, or wants to create new issues. For claim/work flows, read and execute `C:/code/endless/docs/ai-collab-workflow.md`.
argument-hint: "[repo issue-number | issue-number | description of issues to create]"
disable-model-invocation: false
allowed-tools: Bash, Read, Grep, Glob, Edit, Write
version: "6.0"
---
## Modes

This skill operates in three modes based on arguments:

### 1. Create mode (args are not a bare issue number)

When `$ARGUMENTS` is freeform text (not a bare number), create issues. Determine the target repo from context:

| Repo | When |
|------|------|
| `abix-/endless` | Rust/Bevy code, gameplay, ECS, shaders, game features |
| `abix-/k3sc` | Go CLI, k8s manifests, operator, TUI, agent pods, Docker image |

Use `gh issue create -R <owner/repo>` with `--title` and `--body`. The `-R` flag means you can run this from any directory -- do NOT cd to the target repo. Include acceptance criteria as `- [ ]` checkboxes when the scope is clear. Add labels if obvious (bug, feature, etc.).

**Always add `--label ready`** so the k3sc operator can dispatch the issue to agents.

For batch creation (multiple issues at once), create them sequentially and report all URLs at the end.

### 2. Claim/work mode (`/issue [repo] <number>` or `/issue` with no args)

**Repo detection from arguments:**
- `/issue 42` -- bare number, repo = `endless` (default)
- `/issue endless 42` -- explicit repo
- `/issue k3sc 8` -- explicit repo
- `/issue` -- no args, auto-pick from all repos

When repo is specified, add `-R abix-/<repo>` to ALL `gh issue` and `gh pr` commands. This is critical -- without `-R`, gh defaults to the cwd's repo which may be wrong.

**Repo-specific behavior:**
- **endless**: full workflow with compliance docs, cargo-lock, spec gate, regression tests
- **k3sc**: Go project. Use `go build ./...`, `go vet ./...`, `go test ./...` instead of cargo. Skip compliance gate (k8s.md/authority.md/performance.md), skip spec gate, skip feature spec gate. The issue body is the spec for all k3sc issues.

For endless issues, use this as a thin executor for `C:/code/endless/docs/ai-collab-workflow.md`.

The workflow doc (endless repo only) is the source of truth for:

- state machine
- claim protocol
- role selection
- architecture guardrails
- comment formats
- implementation and review flow
- close criteria
- label transitions

Do not duplicate or reinterpret workflow policy here. If this skill and the workflow doc ever drift, follow the doc and then fix the skill.

Operate on exactly one GitHub issue at a time.

## Workspace and identity

Agent identity is derived from the clone path. No registration script, no settings.json, no process-tree walking.

- Windows agents use numbers: `C:\code\endless-claude-3` -> `claude-3`
- k3s agents use letters: `/workspaces/endless-claude-a` -> `claude-a`
- Pattern: `{repo}-{family}-{id}` -> `{family}-{id}`
- Extract from cwd: folder name minus the repo prefix (e.g. `endless-` or `k3sc-`)

Each agent is launched via `k3sc launch` (`Ctrl+Shift+N` in WezTerm) into its own clone. It uses PID-based lockfiles to find free slots.

If the workspace directory already exists, reuse it. Do not recreate or remove existing workspaces.

All work happens in the agent's workspace, not in `C:\code\endless`. Stay in the clone for all git, cargo, file read/edit, and grep/glob operations.

## Branch and PR dedup

Before creating a branch or PR for an issue, check for existing work:

1. Check for an existing PR: `gh pr list --head issue-{N} --state open --json number,title`
2. Check for an existing remote branch: `git ls-remote --heads origin issue-{N}`

If an open PR already exists for the issue, work on that PR's branch -- do not create a new branch or PR. Check it out with `git fetch origin issue-{N} && git checkout issue-{N}`.

Never create duplicate issue branches (e.g., `issue-{N}-v2`). One issue = one branch = one PR.

## Merge conflict check

After checking out an existing `issue-{N}` branch (whether resuming your own work or picking up a review), check the PR for merge conflicts before doing any other work:

1. Query the PR: `gh pr view --head issue-{N} --json mergeStateStatus,mergeable`
2. If `mergeable` is `CONFLICTING` or `mergeStateStatus` is `DIRTY`:
   - Rebase onto base branch: `git fetch origin {base} && git rebase origin/{base}`
   - Resolve any conflicts (prefer the dev side for mechanical conflicts like Cargo.lock; use judgment for code conflicts)
   - Force-push the rebased branch: `git push --force-with-lease origin issue-{N}`
   - Verify the PR is now clean: re-run the `gh pr view` check
3. If `mergeable` is `MERGEABLE` or `mergeStateStatus` is `CLEAN`, proceed normally.
4. If the PR has no mergeable status yet (`UNKNOWN`), wait a few seconds and retry once.

Do not begin implementation or review work on a branch with merge conflicts -- fix them first.

## Branch model

Each issue gets its own branch: `issue-{N}`.

The base branch depends on the repo:
- **endless**: base = `dev`
- **k3sc**: base = `master`

- New issue (no existing branch/PR): `git fetch origin && git checkout -b issue-{N} origin/{base}`
- Continuing work: `git checkout issue-{N} && git pull --rebase origin {base}`
- Push and verify the remote branch before handoff: `git push -u origin issue-{N}` then `git fetch origin && git rev-parse --verify origin/issue-{N}`

## Startup (before any other work)

1. Determine repo from `$ARGUMENTS` (first word if it matches a known repo name, otherwise default to `endless`).
2. If repo is `endless`, read `C:/code/endless/docs/ai-collab-workflow.md`.
3. Derive agentId from the current working directory:
   - Get the folder name of cwd (e.g. `endless-claude-3` or `k3sc-claude-a`)
   - Strip the repo prefix -> `claude-3` or `claude-a` is the agentId
4. Verify the workspace is a git repo on the base branch (`dev` for endless, `master` for k3sc) or an `issue-*` branch. If not, checkout the base branch.

No registration script, no settings.json. The path is the identity.

## Assignment

The k3sc operator assigns issues to agents. By the time you start, the issue is already claimed with your owner label. Just start working.

If the issue has another agent's owner label, do not act on it.

## GitHub access discipline

- keep `gh issue ...` reads sequential and minimal
- never use parallel `gh issue` reads
- prefer one `gh issue list` to identify a candidate, then one `gh issue view <number> --comments` for the selected issue
- reuse existing approval if GitHub access is already approved

## Label management -- operator only

**Agents do NOT touch GitHub labels.** The k3sc operator owns all label transitions:
- Operator adds owner label when dispatching
- Operator removes owner label and adds `needs-review`/`needs-human` when the pod completes
- Operator handles orphan cleanup if a pod dies

Agents focus on: writing code, creating branches, pushing commits, creating PRs.

## Merge prohibition

Agents NEVER merge PRs, close issues, or delete remote branches. Only the human does these.

## Execution

- If `$ARGUMENTS` is empty, follow the no-argument claim flow from the workflow doc.
- If `$ARGUMENTS` contains `<repo> <number>`, use that repo. If bare `<number>`, default to endless.
- Use the workflow doc's exact comment formats and label transitions.
- Include the PR link in handoff comments.
- Do not hand off, request review, or transition labels until the issue branch is pushed and `origin/issue-{N}` verifies locally.
- Complete one workflow step end-to-end before stopping: tests or an explicit blocker, GitHub comment, and label transition.
- Do NOT merge PRs, close issues, or delete remote branches -- human only.

### endless repo execution
- Always run `k3sc cargo-lock fmt` before committing any code changes.
- Always run `k3sc cargo-lock clippy --release -- -D warnings` before committing. Fix all warnings before commit -- this matches the CI build gate.
- Use `k3sc cargo-lock` for all cargo commands (build, check, clippy, fmt, test) to serialize builds across agents sharing one target dir.

### k3sc repo execution
- Use `go build ./...` to build, `go vet ./...` for linting, `go test ./...` for tests.
- No cargo-lock, no compliance docs, no spec gate.
- Default branch is `master` (not `dev`). Branch from `origin/master`, rebase onto `origin/master`.
- After code changes, cross-compile the linux binary: `GOOS=linux GOARCH=amd64 go build -o image/k3sc .`

## Review gates

Review gates (compliance, feature spec, DRY, regression tests, acceptance criteria, performance standards) are enforced by `/review`, not `/issue`. See the `/review` skill for all gate definitions.

## Branch cleanup

Agents do not delete remote branches or close issues. The human handles all post-merge cleanup after confirming the branch is good.
