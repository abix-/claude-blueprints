---
name: "debate"
description: "Start or participate in a structured multi-agent debate. Invoke with /debate 'goal' to create, or bare /debate to join an existing one."
---
# debate

Structured debate between two Claude Code sessions. One human, two
agents, file-based protocol. The script is at
`C:/code/claude-blueprints/scripts/debate.py`.

Your session is identified automatically by CLAUDE_CODE_SESSION_ID.

## when invoked with a goal: /debate "goal here"

You are the human's agent in terminal 1. Do all of this automatically:

1. Run `python C:/code/claude-blueprints/scripts/debate.py new <goal>`
2. Run `python C:/code/claude-blueprints/scripts/debate.py join`
3. Print the terminal 2 instructions to the user so they can start
   the other Claude. Tell them:
   "open another Claude Code terminal in this repo and say: /debate"
4. Wait for the user to confirm the other agent joined, or tell you
   to check status.

When both agents have joined and it is your turn, read the relevant
code in the repo, then act on your turn (propose or review).

## when invoked bare: /debate

You are joining an existing debate (likely terminal 2). Do all of this
automatically:

1. Run `python C:/code/claude-blueprints/scripts/debate.py join`
2. Run `python C:/code/claude-blueprints/scripts/debate.py status`
3. Run `python C:/code/claude-blueprints/scripts/debate.py read --last 10`
4. If it is your turn, read the relevant code in the repo, then act.
   If not, say whose turn it is and STOP.

## acting on your turn

### proposer

Read the goal. Read the relevant code. Consider prior feedback. Then:
```
python C:/code/claude-blueprints/scripts/debate.py propose "your proposal"
```
Be concrete: name files, functions, approaches. Not vague direction.

### reviewer

Read the proposal. Read the relevant code yourself. Then:
```
python C:/code/claude-blueprints/scripts/debate.py review agree "why"
python C:/code/claude-blueprints/scripts/debate.py review disagree "what is wrong and what would be better"
python C:/code/claude-blueprints/scripts/debate.py review revise "mostly good but change X"
```
Be honest. Do not agree just to be agreeable. Do not disagree just
to seem thorough.

### implementer

Write the code, then:
```
python C:/code/claude-blueprints/scripts/debate.py implement "summary"
```

### verifier

Read the diff. Run tests if they exist. Then:
```
python C:/code/claude-blueprints/scripts/debate.py verify accept "looks good"
python C:/code/claude-blueprints/scripts/debate.py verify reject "problem"
```

## after acting: STOP

After your action, STOP and WAIT. Do not take two turns in a row.
Do not poll. Tell the user what you did and that it is now the other
agent's turn.

## human commands (run on behalf of the human when asked)

```
python C:/code/claude-blueprints/scripts/debate.py say "message"
python C:/code/claude-blueprints/scripts/debate.py force <phase>
python C:/code/claude-blueprints/scripts/debate.py read
python C:/code/claude-blueprints/scripts/debate.py done
```

## rules

- do not argue with human messages. incorporate them
- do not propose the same rejected thing without changes
- keep proposals and reviews concise. a few paragraphs, not an essay
- when you disagree, say what would be better, not just what is wrong
- read the actual code before proposing or reviewing. do not guess
