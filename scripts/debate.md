# agent-debate

Structured debate loop for multiple AI agents with consensus gating.

## problem

You have multiple AI coding agents (Claude Code, Codex) running as
subscription CLI sessions. You want them to collaborate on a task:
propose solutions, review each other's work, reach agreement, then
implement. Today nothing connects them. You copy-paste between terminals.

## what this is

A Python CLI (`debate`) and a shared skill that lets any agent participate
in a file-based debate. No API keys, no MCP server, no cloud. Just files
in a shared directory that enforce a protocol.

## participants

- **human**: sets the goal, can intervene at any time, watches the debate
- **agent A**: a running Claude Code or Codex session
- **agent B**: another running Claude Code or Codex session

The human is always the authority. Agents cannot override the human.

## the protocol

### phases

```
GOAL        human sets the objective and constraints
PROPOSE     one agent proposes a solution
REVIEW      the other agent reviews the proposal
  (loop: if disagreement, swap roles and repeat)
CONSENSUS   both agents agree on an approach
IMPLEMENT   one agent writes the code
VERIFY      the other agent reviews the implementation
DONE        human signs off or sends back to PROPOSE
```

### turn order

Roles swap every round. If agent A proposed in round 1, agent B proposes
in round 2. This prevents one agent from dominating.

### consensus gate

Work does not proceed to IMPLEMENT until both agents agree. Agreement
means the reviewer responds with `agree` and optionally a summary of
what was agreed. If the reviewer responds with `disagree` or `revise`,
the debate continues with swapped roles.

### human intervention

The human can at any time:
- send a message visible to both agents (guidance, correction, new info)
- force a phase change (skip to implement, restart debate, abort)
- override consensus (veto an agreement, force agreement)

## file layout

All state lives in a single debate directory. Default: `~/.debate/active/`.

```
~/.debate/
  active/           current debate (only one at a time to start)
    state.json      current phase, round, turn, participants
    messages.jsonl   append-only conversation log
    goal.md         the human's objective (plain text, not structured)
  archive/          completed debates moved here
```

### state.json

```json
{
  "id": "debate-20260809-130000",
  "phase": "REVIEW",
  "round": 2,
  "turn": "agent-b",
  "proposer": "agent-b",
  "reviewer": "agent-a",
  "participants": {
    "human": {"name": "abix", "type": "human"},
    "agent-a": {"name": "claude-1", "type": "claude-code"},
    "agent-b": {"name": "codex-1", "type": "codex"}
  },
  "created": "2026-08-09T13:00:00Z",
  "updated": "2026-08-09T13:05:00Z"
}
```

### messages.jsonl

One JSON object per line, append-only.

```json
{"ts": "2026-08-09T13:00:00Z", "from": "human", "phase": "GOAL", "body": "see goal.md"}
{"ts": "2026-08-09T13:01:00Z", "from": "agent-a", "phase": "PROPOSE", "round": 1, "body": "I propose we..."}
{"ts": "2026-08-09T13:02:00Z", "from": "agent-b", "phase": "REVIEW", "round": 1, "verdict": "disagree", "body": "That won't work because..."}
{"ts": "2026-08-09T13:03:00Z", "from": "agent-b", "phase": "PROPOSE", "round": 2, "body": "Instead, we should..."}
{"ts": "2026-08-09T13:04:00Z", "from": "agent-a", "phase": "REVIEW", "round": 2, "verdict": "agree", "body": "Yes, that approach handles the edge case."}
{"ts": "2026-08-09T13:04:01Z", "from": "system", "phase": "CONSENSUS", "body": "Both agents agree. Moving to IMPLEMENT."}
```

## the cli: `debate`

Python script. Single file, no dependencies beyond stdlib. Installed
into PATH or run directly.

```
debate new "make goons retaliate when attacked"    create a new debate, write goal.md
debate join <name>                                  register as a participant
debate status                                       show current phase, whose turn, round
debate read [--last N]                              show conversation (last N messages)
debate propose <message>                            submit a proposal (must be your turn, must be proposer)
debate review agree|disagree|revise <message>       review a proposal (must be your turn, must be reviewer)
debate say <message>                                human says something (always allowed)
debate force <phase>                                human forces a phase change
debate implement <message>                          submit implementation notes/summary
debate verify accept|reject <message>               review an implementation
debate done                                         archive the debate
debate watch                                        tail -f the conversation log, formatted
```

The CLI enforces:
- only the current turn holder can propose/review
- only humans can use `say` and `force`
- phase transitions follow the protocol
- roles swap automatically on disagreement

## the skill

A shared skill installed to both `~/.claude/skills/` and `~/.agents/skills/`
(via the existing sync infrastructure). It tells the agent:

- you are in a debate, use the `debate` CLI
- at the start of your turn, run `debate status` and `debate read --last 5`
- when proposing, think through the problem, then run `debate propose "..."`
- when reviewing, read the proposal carefully, then run `debate review agree|disagree|revise "..."`
- after your action, STOP and WAIT (do not take another turn)
- if the human sends a message, read it and incorporate it

## what this does NOT do

- no automatic agent launching (you start each session yourself)
- no API calls (subscription CLI only)
- no MCP server (files only)
- no web UI (terminal only)
- no multi-debate (one active debate at a time, to start)
- no git integration (agents use their own git tools as normal)

## implementation plan

1. `debate` CLI in Python (single file, stdlib only)
2. skill file for Claude Code
3. skill file for Codex
4. test it manually with two terminals

## open questions

- should agents be able to request more information from the human?
  (a "question" message type that pauses the debate until answered)
- should the implement phase track which files were changed?
- should there be a time limit per turn?
- codex skill path: is it `~/.agents/skills/` or `~/.codex/skills/`?
