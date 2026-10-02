---
name: "debate"
description: "Start or participate in a structured multi-agent debate. Invoke with /debate 'goal' to create, or bare /debate to join an existing one."
---
# debate

Structured debate between two Claude Code sessions. One human, two
agents, direct messaging via SendMessage/ListAgents.

## when invoked with a goal: /debate "goal here"

You are starting a new debate. Do all of this automatically:

1. Run ListAgents to find peer sessions.
2. If a peer session exists, do your research (see below), then send
   your proposal to the peer via SendMessage. Include the goal and
   your role (proposer) in the message. Use notify_when_idle: true
   so you hear back when they respond.
3. If no peer session exists, tell the user:
   "open another Claude Code terminal in this repo and say: /debate"
   then wait for them to confirm.
4. Tell the user what you proposed and that you are waiting for review.

You do NOT wait for the second agent before researching. You are the
proposer. Research and propose immediately. The reviewer joins later,
reads your proposal message, and reviews it.

## when invoked bare: /debate

You are joining an existing debate as the reviewer. Do all of this
automatically:

1. Run ListAgents to find peer sessions.
2. Check your incoming messages. If the proposer already sent a
   proposal, read it carefully.
3. Do your research (see below), then send your review back via
   SendMessage.
4. If no proposal has arrived yet, send a message to the proposer
   saying you have joined and are waiting. Use notify_when_idle: true
   to hear when they send their proposal.

## message format

All debate messages follow this structure so both sides can parse
them reliably:

```
DEBATE: <goal (first message only)>
PHASE: propose | review | implement | verify | done
FROM: <your session name from ListAgents>
MODEL: <your model name, e.g. "Opus 4.6", "Fable 5">

<body>
```

Include your MODEL in every message. This lets both sides (and the
human) know which model is arguing which position.

## research comes first

Your turn is not over until you have concrete findings. Before you
propose or review, do the full investigation yourself: read the code,
read the logs, read the docs, run the tests. Your proposal or review
contains what you FOUND, not what you plan to look for.

Research means: open the files, grep for the functions, read the
relevant log lines, trace the execution path. When you are done you
can name the specific file, function, line, and behavior that causes
the problem. If you cannot name those, you are not done researching.

Do not submit a proposal that says "let's investigate X." Do not
submit a review that agrees with an investigation plan. Both are
empty. The debate exists so two agents bring independent findings to
the table and challenge each other's conclusions.

## acting on your turn

### proposer

Read the goal. Read the relevant code. Consider prior feedback.

Be concrete: name files, functions, approaches. Not vague direction.

A proposal must contain FINDINGS AND A RESOLUTION. Do the research
BEFORE proposing. Read the code, read the logs, read the docs. Then
state what you found AND how to fix it: the specific cause, the
specific fix, the specific files and functions to change and how.

"Let's look at X" is not a proposal. "Run an attempt and trace the
stall" is not a proposal. Those are investigation plans.

"X is broken because function Y at file:line does Z instead of W.
The fix: change Y to do W. This aligns with docs/authority.md
because [reason]" is a proposal.

If the goal asks for a root cause, name the exact broken behavior.
If the goal asks for a plan, name concrete code changes with file
paths and function names.

A proposal is a numbered list of concrete steps. Each step says
what to change, where, and what the result looks like. The operator
reads the list and executes it top to bottom without guessing. Not
"investigate and see" steps. Not architecture descriptions. Not
explanations of how systems work. Steps: delete this function, change
this line from X to Y, add this test that asserts Z, update this
doc to say W. Every step has a verb and a target.

Send your proposal via SendMessage to the reviewer's session name.

### reviewer

A review without verification is worthless. Before you send ANY
review verdict (agree, disagree, revise), you MUST:

1. Read every file and line the proposal references. Open them.
2. Verify every factual claim: does the function do what the
   proposal says? Is the line number right? Does the proposed fix
   target the actual code that exists today?
3. If the proposal references a design doc, authority doc, or
   brain.md section, open that doc and verify the claim.
4. If the proposal says "this function exists" or "this pattern
   is used N lines later," confirm it. If it does not exist or
   the pattern differs, say so.
5. If the round includes human messages with new content (design
   statements, revised steps, corrections), verify every new
   claim against the code independently. The human's instructions
   are authoritative, but the proposal's interpretation of them
   still needs checking.

Your review must name what you verified and what you found. "I
checked factory_work.rs:881 and confirmed the connector layout
is handled there" is a review. "The proposal looks correct" is
not.

NEVER rubber-stamp. If you did not open the file, you did not
review it. An agree without evidence is a lie.

Send your review via SendMessage. Start with one of:
- VERDICT: agree
- VERDICT: disagree
- VERDICT: revise

Then explain what you verified and what you found.

Be honest. Do not agree just to be agreeable. Do not disagree just
to seem thorough.

### implementer

Write the code, then send a summary of changes to the other session
via SendMessage with PHASE: implement.

### verifier

Read the diff. Run tests if they exist. Then send your verdict via
SendMessage with PHASE: verify and either VERDICT: accept or
VERDICT: reject.

## turn flow

Messages arrive as `<cross-session-message>` blocks. When you receive
one, read it, do your research, and respond. No polling needed.

If you need to wait for a response, use notify_when_idle: true on
your SendMessage call. You will receive a notification when the other
session finishes processing.

## human intervention

The human can participate at any time by telling their session what
to say. The session sends it via SendMessage. Human messages take
priority: incorporate them, do not argue with them.

The human can end the debate by saying "done" or "stop". When told
to stop, send a final PHASE: done message to the peer and report
the outcome to the user.

## saving results

When a debate reaches consensus or is marked done, save the concrete
steps to a file in the repo under `docs/debates/`. Include the date,
goal, root cause, design alignment, and every concrete implementation
step. This is the permanent record of what was decided.

## rules

- do not argue with human messages. incorporate them
- do not propose the same rejected thing without changes
- keep proposals and reviews concise. a few paragraphs, not an essay
- when you disagree, say what would be better, not just what is wrong
- read the actual code before proposing or reviewing. do not guess
- NEVER propose an investigation plan. do the investigation, then
  propose what you found AND how to fix it. "run tests and see" is
  not a proposal. "run an attempt and trace the stall" is not a
  proposal. "test X fails because function Y at file:line does Z;
  the fix is to change Z to W" is a proposal
- a reviewer who agrees with an investigation plan or a diagnosis
  without a resolution is agreeing with nothing. reject it and say
  what concrete resolution is missing
- every proposal must end with a resolution: what to change, where,
  and why it aligns with the project's docs and authority. a root
  cause without a fix is half the job
- write proposals and reviews in plain English. use the project's
  own terms (Factorio terms, the terminology doc, the design doc).
  do not use code variable names, struct names, or type names in
  the proposal text. name the file and the line when pointing at
  code, but describe what it does in words the operator understands.
  "the manifest stamps every building as needing full power before
  it counts as built" not "CompletionState::Satisfied on
  BuildingSpec". the operator reads proposals cold; jargon forces
  them to open the file to understand what you mean
