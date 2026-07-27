---
name: learn
description: Review conversations or recent work across repositories, extract verified reusable lessons, and update repository-authoritative skills. Use when the user asks to learn from recent work, review the last N days, or improve skills from corrections and commit history.
---

# Learn

Turn verified experience into concise skill guidance. Invocation is approval to
perform the requested review and edit the applicable skills. Do not pause for a
second approval before editing.

## Authority

Edit the repository-authoritative skill collection, not installed copies under
an agent home directory. Locate the blueprint repository that owns
`skills/learn/SKILL.md`. If more than one candidate exists and the user did not
identify the authority, ask which repository is authoritative.

Preserve uncommitted work. Never revert, clean, stash, or overwrite unrelated
changes. Commit only files changed by this review.

## Review scope

Use the scope requested by the user:

- Conversation review: inspect the current conversation for corrections,
  preferences, useful patterns, and failures.
- Repository review: inspect every Git repository under the requested code
  root. Default to the last 30 days when no time window is given.
- Combined review: use both sources and merge duplicate lessons.

For repository review, first discover every Git repository under the selected
root. Exclude dependency caches, generated output, runtime plugin caches, and
duplicate worktrees when they represent the same repository and commits.

## Evidence collection

For every Git repository:

1. Record its path, current branch, remote, dirty state, and commit count in the
   review window.
2. Run `git log` for the complete review window. Include commit hash, date,
   subject, and changed paths.
3. Use commit subjects and changed paths to identify likely learning signals:
   fixes, reverts, follow-up corrections, failure reports, review findings,
   design changes, repeated rewrites, test gaps, and operational incidents.
4. Use `git show --stat` before opening a full diff. Read the full `git show`
   diff for every likely learning signal. Read nearby authoritative design,
   todo, changelog, project-state, and failure documentation when the change
   depends on that context.
5. Record the repository path and commit hash for every candidate lesson. A
   recollection without source evidence is not enough.

Do not treat commit volume as progress. Distinguish verified results from code
that was only written or committed. Repeated corrective commits are evidence
that the earlier approach did not work.

## Select lessons

A lesson belongs in a skill only when it is reusable:

- It recurred across commits or repositories.
- It prevented or caused a costly failure.
- It captures a stable tool, API, platform, testing, or design constraint.
- It changes how future work should be performed.

Keep repository-specific architecture and commands in that repository's
existing skill. Put language or tool behavior in the matching existing skill.
Put general development behavior in the general code skill. Update the global
agent instructions only when the lesson is truly universal and the user asked
for that scope.

Do not copy a failure log into a skill. Convert evidence into the shortest
actionable rule that would have prevented the failure. Do not add a rule already
covered by an existing skill. Create a new skill only when no existing skill has
the right scope.

## Edit and validate

1. Read each complete target skill before editing it.
2. Make the smallest change that captures the lesson.
3. Keep terminology consistent with the source repository and the user.
4. Keep written files ASCII unless the repository explicitly requires another
   encoding.
5. Run the skill validator for every changed skill.
6. Run the blueprint repository tests and installation drift checks.
7. Review the final diff for unsupported frontmatter, agent-specific paths in
   shared skills, duplicated guidance, and unrelated edits.
8. Commit the verified changes with a concise lowercase message and push.

## Report

Report:

- repositories and commits reviewed
- evidence-backed lessons accepted and rejected
- skills changed
- validation and test results
- commit hash and push result

If a repository could not be read or validated, name it explicitly. Never imply
that all repositories were reviewed when any were skipped.
