---
description: Analyze why Claude made its previous response -- trace reasoning to system prompt, CLAUDE.md, memory, skills, or context
disable-model-invocation: true
allowed-tools: Read, Glob, Grep
version: "1.0"
---
## Task

Analyze your most recent response (before this skill was invoked) and explain WHY you produced it. This is not about right/wrong -- it is about tracing the reasoning chain so the user can identify what to change when behavior drifts.

## Process

1. Identify the response being analyzed (the one immediately before `/why` was invoked)
2. For each significant choice in that response, trace it to one or more sources:

| Source | What to check |
|--------|--------------|
| System prompt | Built-in instructions from Anthropic (tool usage rules, tone, safety) |
| CLAUDE.md | User's global rules (`~/.claude/CLAUDE.md`) |
| Workspace CLAUDE.md | Repo-level rules (`<repo>/CLAUDE.md`) |
| Skill | A skill file that was loaded (`~/.claude/skills/<name>/SKILL.md`) |
| Memory | A memory file (`~/.claude/projects/*/memory/`) |
| Context | Something the user said in this conversation |
| Model default | Claude's training -- not from any instruction, just default behavior |
| Inference | You inferred/assumed something not explicitly stated anywhere |

3. Read the actual source files to confirm. Do not guess -- open the file and quote the line.
4. Flag any inference or assumption that was NOT grounded in a source file.

## Output format

Use this exact format:

## /why -- reasoning trace

```
 CHOICE                        SOURCE              EVIDENCE
 ------------------------------------------------------------------------------------------
 used ASCII table format       CLAUDE.md:24        "NEVER use Unicode...ALWAYS use ASCII"
 ended with confidence rating  CLAUDE.md:26        "ALWAYS end every response with..."
 read file before editing      system prompt       "read it first...before suggesting modifications"
 assumed user wanted X         inference           (no source -- I filled in a gap)
 ...
```

For each row:
- CHOICE: what you did (short phrase, under 40 chars)
- SOURCE: where the instruction came from (file:line, "system prompt", "context", "model default", or "inference")
- EVIDENCE: the quoted text or brief explanation

## After the table

Print a summary section:

```
GROUNDED: {n} choices traced to explicit instructions
INFERRED: {n} choices based on assumption or model default
```

If any choices were inferred, add:

```
INFERENCES:
- {choice}: {why you inferred this and what would have changed your behavior}
```

This section is the most valuable -- it shows the user exactly where to add a rule or clarification to prevent future drift.
