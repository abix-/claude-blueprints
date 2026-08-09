---
name: "debate"
description: "Participate in a structured multi-agent debate. Use when the user invokes /debate or tells you to join a debate."
---
# debate

You are participating in a structured debate with another AI agent.
A human set the goal. You take turns proposing and reviewing solutions
until you agree on an approach, then one of you implements while the
other reviews.

There are two terminals, two Claude sessions. The script identifies
you automatically by your CLAUDE_CODE_SESSION_ID. No env vars to set.

- **the human's agent** (terminal 1): the human talks to you directly.
  you are both their hands in the codebase AND a debate participant.
  the human tells you when to check your turn, when to propose, when
  to review. you do what they say. you also handle `debate say` and
  `debate force` on their behalf.
- **the other agent** (terminal 2): running in a separate terminal
  with no human. you were given one instruction at startup: /debate.
  you check your turn, act when it is your turn, then STOP and WAIT
  until told to check again.

There is no separate human terminal. The human works through their
agent in terminal 1.

## when /debate is invoked

Do these steps automatically:

1. Run `python ~/code/claude-blueprints/scripts/debate.py status`
   to see if a debate exists and what phase it is in.
2. If not yet joined, run:
   `python ~/code/claude-blueprints/scripts/debate.py join`
3. Run `python ~/code/claude-blueprints/scripts/debate.py read --last 10`
   to see recent messages.
4. If it is your turn, act (see below). If not, say so and STOP.

## acting on your turn

### if you are the proposer

Read the goal. Read the relevant code in the repo. Consider any prior
feedback from the other agent and any human messages. Then:

```
python ~/code/claude-blueprints/scripts/debate.py propose "your proposal here"
```

Your proposal should be concrete: what to change, where, and why.
Not vague direction. Name files, functions, and approaches.

### if you are the reviewer

Read the proposal. Read the relevant code yourself (do not trust the
proposer's description alone). Consider whether it actually solves the
goal, whether there are edge cases, whether there is a simpler approach.
Then:

```
python ~/code/claude-blueprints/scripts/debate.py review agree "why you agree"
python ~/code/claude-blueprints/scripts/debate.py review disagree "what is wrong and what would be better"
python ~/code/claude-blueprints/scripts/debate.py review revise "mostly good but change X"
```

Be honest. Do not agree just to be agreeable. If the proposal has a
real problem, say so. But do not disagree just to seem thorough. If
it is good, say agree and move on.

### if you are implementing

Write the code, then record what you did:
```
python ~/code/claude-blueprints/scripts/debate.py implement "summary of changes"
```

### if you are verifying

Read the implementation diff. Check it matches what was agreed. Run
tests if they exist. Then:
```
python ~/code/claude-blueprints/scripts/debate.py verify accept "looks good"
python ~/code/claude-blueprints/scripts/debate.py verify reject "problem with X"
```

## after acting: STOP

After your action (propose, review, implement, verify), STOP and WAIT.
Do not take two turns in a row. Do not poll. Tell the user what you
did and that it is now the other agent's turn.

## rules

- do not argue with human messages. incorporate them
- do not propose the same thing that was already rejected without changes
- keep proposals and reviews concise. a few paragraphs, not an essay
- when you disagree, say what would be better, not just what is wrong
- read the actual code before proposing or reviewing. do not guess
