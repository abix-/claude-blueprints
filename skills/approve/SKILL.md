---
name: approve
description: Approve and merge a PR after human review. Posts approval comments, merges, closes issue, cleans up branch.
user-invocable: true
version: "1.0"
---

# Approve

Human-only approval command. Merges a PR that passed /review.

## Arguments

- `/approve` -- approve the most recently reviewed PR in this conversation
- `/approve <PR>` -- approve PR #{PR} in endless
- `/approve <repo> <PR>` -- approve PR #{PR} in the specified repo

Repo mapping: `endless` -> `abix-/endless`, `k3sc` -> `abix-/k3sc`

## Steps

1. **Post approval comment on the PR:**

```bash
gh pr comment {N} --repo {owner/repo} --body "approved -- merging"
```

2. **Post approval comment on the linked issue:**

Extract issue number from branch name (`issue-{N}`).

```bash
gh issue comment {issue_N} --repo {owner/repo} --body "approved via PR #{N} -- merging and closing"
```

3. **Merge the PR:**

```bash
gh pr merge {N} --repo {owner/repo} --squash --delete-branch
```

4. **Close the linked issue:**

```bash
gh issue close {issue_N} --repo {owner/repo}
```

5. **Release k3sc reservation (if active):**

```bash
k3sc release --repo {repo-short-name} --pr {N}
```

6. **Return to base branch and pull:**

```bash
git checkout {base-branch} && git pull
```

7. **Print confirmation:**

```
Approved and merged PR #{N}, closed issue #{issue_N}.
```
