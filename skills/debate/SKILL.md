---
name: "debate"
description: "Participate in a structured multi-agent debate. Use when the user invokes /debate or tells you to join a debate."
---
# debate

You are participating in a structured debate with another AI agent.
A human set the goal. You take turns proposing and reviewing solutions
until you agree on an approach, then one of you implements while the
other reviews.

There are two roles in the debate:

- **the human's agent**: the human talks to you directly. you are both
  their hands in the codebase AND a debate participant. the human tells
  you when to check your turn, when to propose, when to review. you do
  what they say. when the human wants to send a message to both agents,
  run `debate say "message"` for them.
- **the other agent**: running in a separate terminal. you were given
  one instruction at startup: participate in the debate. you check your
  turn, act when it is your turn, then STOP and WAIT.

Which role you are depends on whether the human is talking to you
directly or whether you were launched with a standing instruction.

## setup

The `debate` CLI is at `~/code/claude-blueprints/scripts/debate.py`.
Your identity is set by the `DEBATE_AGENT` environment variable.
If it is not set, ask the human which agent you are (agent-a or agent-b)
and run: `export DEBATE_AGENT=agent-a`

## your loop

1. Run `python ~/code/claude-blueprints/scripts/debate.py status`
2. Run `python ~/code/claude-blueprints/scripts/debate.py read --last 10`
3. If it is not your turn, tell the user and STOP. Do not poll or loop.
4. If it is your turn, read the goal and the conversation history.

### if you are the proposer

Think through the problem. Consider the goal, any prior feedback from
the other agent, and any human messages. Read the relevant code in the
repo before proposing. Then submit your proposal:

```
python ~/code/claude-blueprints/scripts/debate.py propose "your proposal here"
```

Your proposal should be concrete: what to change, where, and why.
Not vague direction. Name files, functions, and approaches.

### if you are the reviewer

Read the proposal carefully. Read the relevant code yourself (do not
trust the proposer's description alone). Consider whether it actually
solves the goal, whether there are edge cases, whether there is a
simpler approach. Then respond:

```
python ~/code/claude-blueprints/scripts/debate.py review agree "why you agree"
python ~/code/claude-blueprints/scripts/debate.py review disagree "what is wrong and what would be better"
python ~/code/claude-blueprints/scripts/debate.py review revise "mostly good but change X"
```

Be honest. Do not agree just to be agreeable. If the proposal has a
real problem, say so. But do not disagree just to seem thorough. If
it is good, say agree and move on.

### if you are implementing

After consensus, the human will advance to IMPLEMENT. Write the code,
then record what you did:

```
python ~/code/claude-blueprints/scripts/debate.py implement "summary of changes"
```

### if you are verifying

Read the implementation diff. Check that it matches what was agreed.
Run tests if they exist. Then:

```
python ~/code/claude-blueprints/scripts/debate.py verify accept "looks good"
python ~/code/claude-blueprints/scripts/debate.py verify reject "problem with X"
```

## rules

- after your action (propose, review, implement, verify), STOP and WAIT
- do not take two turns in a row
- do not argue with human messages. incorporate them
- do not propose the same thing that was already rejected without changes
- keep proposals and reviews concise. a few paragraphs, not an essay
- when you disagree, say what would be better, not just what is wrong
- read the actual code before proposing or reviewing. do not guess
