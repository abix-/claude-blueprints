---
description: Review conversation and update skills with learnings
disable-model-invocation: true
allowed-tools: Read, Edit, Glob, Bash
version: "1.1"
---
## Task

Two modes: conversation review (default) and commit review.

### Mode 1: Conversation Review (default)

1. Read all skills: `~/.claude/skills/*.md`
2. Review this conversation for:
   - Corrections made to your behavior
   - Patterns that worked well
   - User preferences discovered
   - Mistakes to avoid
3. For each learning, determine which skill it belongs to (or if new skill needed)
4. Propose specific edits - show diff-style changes
5. Wait for approval before editing

### Mode 2: Commit Review

Review git commits since the last skill update, extracting patterns into skills.

1. Read the target skill (e.g. `~/.claude/skills/bevy.md`) — note its `updated` date
2. List commits since that date: `git log --oneline --reverse --since=<date>`
3. For each commit, oldest first, ONE AT A TIME:
   a. `git show <hash> --stat` to see scope
   b. `git show <hash>` to read the full diff
   c. Extract skill-worthy learnings (new patterns, gotchas, API changes, architecture decisions)
   d. If learnings found: edit the skill file with concise additions
   e. If not: skip — most commits won't have skill-relevant content
4. After all commits reviewed: bump skill metadata (`version`, `updated`)

Keep skills lean. Don't add what's already covered.
