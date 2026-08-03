#!/usr/bin/env python3
"""Refuse the two things that are never allowed, before they run.

PreToolUse hook. Reads the tool call on stdin, exits 2 to block and prints the
reason on stderr for the model to read. Exits 0 for everything else.

Both rules come from CLAUDE.md and both are pure pattern matching, so this can
never block ordinary work:

  Subagents      the Task and Agent tools are the same tool under two names,
                 and both are banned; they burn the operator's tokens for work
                 direct tool calls do better.

  Destructive    git commands that can delete uncommitted work. Violated on
  git            2026-06-07, destroying ~150 lines of operator edits, and again
                 on 2026-08-02 with an unasked-for revert. The operator runs
                 these themselves when they want them.
"""

import json
import re
import sys

SUBAGENT_TOOLS = {"task", "agent"}

# Each pattern is anchored on the git subcommand, so an unrelated command that
# merely contains the word (a grep for "reset --hard", a path called restore)
# does not match.
DESTRUCTIVE_GIT = [
    (r"\bgit\s+checkout\s+--(\s|$)", "git checkout -- <path>"),
    (r"\bgit\s+restore\b", "git restore"),
    (r"\bgit\s+reset\s+--hard\b", "git reset --hard"),
    (r"\bgit\s+clean\s+-[a-z]*f", "git clean -f"),
    (r"\bgit\s+stash\s+drop\b", "git stash drop"),
    (r"\bgit\s+branch\s+-D\b", "git branch -D"),
    (r"\bgit\s+rm\b", "git rm"),
    (r"\bgit\s+revert\b", "git revert"),
    (r"\bgit\s+push\s+.*--force\b", "git push --force"),
    (r"\bgit\s+push\s+.*(\s|^)-f(\s|$)", "git push -f"),
]


def block(reason):
    print(reason, file=sys.stderr)
    sys.exit(2)


def main():
    try:
        call = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    tool = str(call.get("tool_name", ""))
    args = call.get("tool_input") or {}

    if tool.lower() in SUBAGENT_TOOLS:
        block(
            f"BLOCKED: {tool} is a subagent and subagents are banned (CLAUDE.md). "
            "Do the work yourself with Read, Edit, Grep, Glob and Bash, including "
            "every search. If you believe a subagent is genuinely needed, ask the "
            "operator first."
        )

    if tool == "Bash":
        command = str(args.get("command", ""))
        for pattern, name in DESTRUCTIVE_GIT:
            if re.search(pattern, command):
                block(
                    f"BLOCKED: `{name}` can delete work that is not yours "
                    "(CLAUDE.md: NEVER destroy uncommitted work). Commit only your "
                    "own paths with `git commit <path> -m ...`, leave everything "
                    "else in the tree, and tell the operator what you found. If a "
                    "revert really is the answer, they will run it."
                )

    sys.exit(0)


if __name__ == "__main__":
    main()
