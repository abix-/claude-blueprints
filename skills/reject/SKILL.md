---
name: reject
description: Close a failed PR, comment findings on the issue, and reset. Use after /review when verdict is needs work.
argument-hint: "[repo] <PR-number> <issue-number>"
disable-model-invocation: true
version: "1.0"
---

# Reject

Close a PR that failed /review, post findings on the linked issue, and reset.

## Arguments

- `/reject <PR> <issue>` -- endless repo (default)
- `/reject <repo> <PR> <issue>` -- explicit repo

Repo mapping: `endless` -> `abix-/endless`, `k3sc` -> `abix-/k3sc`

## Steps

1. Post review findings as a comment on the issue (not the PR -- the PR comment was already posted by /review).
2. Close the PR: `gh pr close {PR} --repo {owner/repo}`
3. Release k3sc reservation: `k3sc release --repo {repo-short} --pr {PR}`
4. Reset the issue: `k3sc reset {issue}`
5. Return to main/master: `git checkout {base-branch} && git pull`
6. Print: `Rejected PR #{PR}, issue #{issue} reset to ready.`

## Issue comment format

```
## /review failed PR #{PR}

PR #{PR} closed -- needs work before re-submission.

### Required before next attempt
{bullet list of what's missing -- from the /review findings}

### What passed
{bullet list of what was good}
```

## Integration with /review

When /review verdict is "needs work", invoke `/reject` automatically instead of asking "merge or skip?". Do not prompt the human for confirmation on failed PRs.
