---
name: "debate"
description: "Start or participate in a structured multi-agent debate. Invoke with /debate 'goal' to create, or bare /debate to join an existing one."
---
# debate

Structured debate between two Claude Code sessions. One operator, two
agents, direct messaging via SendMessage/ListAgents.

## when invoked with a goal: /debate "goal here"

You are starting a new debate. Do all of this automatically:

1. Run ListAgents to find the peer. The peer is the session the
   operator named, or the one session working in this repo's folder
   (its name starts with the folder's name). With several candidates
   and none named, ask the operator which one; never guess.
2. If there is no peer, start one inside onelog with onelog's
   `open_agent` tool and this repo's project name, find it with
   ListAgents, and tell it to join with `/debate`. If onelog is not
   open, ask the operator to open a second Claude Code session in this
   repo and say `/debate`.
3. Do your research (see below), then send your proposal to the peer
   via SendMessage, `to` its name exactly as ListAgents prints it.
   Include the goal and your role (proposer). Use
   notify_when_idle: true so you hear when they finish their turn.
4. Tell the operator what you proposed and that you are waiting for
   review.

You do NOT wait for the second agent before researching. You are the
proposer. Research and propose immediately. The reviewer joins later,
reads your proposal message, and reviews it.

## when invoked bare: /debate

You are joining an existing debate as the reviewer. Do all of this
automatically:

1. Run ListAgents to find the proposer.
2. Messages arrive by themselves; there is no inbox to check. If the
   proposal is already in your conversation (a
   `<cross-session-message from="...">` block), read it carefully.
3. Do your research (see below), then send your review back via
   SendMessage, `to` the proposal's `from` attribute copied exactly.
4. If no proposal has arrived yet, send a message to the proposer
   saying you have joined and are waiting. Use notify_when_idle: true
   to hear when they finish their turn; the proposal arrives as a
   message of its own.

## message format

All debate messages follow this structure so both sides can parse
them reliably:

```
DEBATE <phase>: <goal in a few words>; <the one-line point of this message>
PHASE: propose | review | implement | verify | done
FROM: <your session name from ListAgents>
MODEL: <your model name, as your system prompt gives it>

<body>
```

The receiver sees only the first line until they expand the message,
so the first line of every message says the debate, the phase and what
this message is about (e.g. `DEBATE review: power poles; disagree, the
fix targets the wrong function`), never just `PHASE:`.

Include your MODEL in every message, the name your own system prompt
gives you, never one remembered from an example. This lets both sides
(and the operator) know which model is arguing which position.

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
3. If the proposal references a design doc, authority doc, or any
   other project doc, open that doc and verify the claim.
4. If the proposal says "this function exists" or "this pattern
   is used N lines later," confirm it. If it does not exist or
   the pattern differs, say so.
5. If the round includes operator messages with new content (design
   statements, revised steps, corrections), verify every new
   claim against the code independently. The operator's instructions
   are authoritative, but the proposal's interpretation of them
   still needs checking.

Your review must name what you verified and what you found. "I
checked the file and line the proposal names and confirmed the case
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

Messages arrive as `<cross-session-message from="...">` blocks. When
you receive one, read it, do your research, and respond. To reply,
copy its `from` attribute as your `to`. No polling needed.

If you need to wait for a response, use notify_when_idle: true on
your SendMessage call. One `[Cross-session idle notice]` arrives when
the other session next finishes its turn; if it never does, the
notice says the subscription expired instead. Never poll ListAgents
or send "are you done?" messages.

A sent message reached the other session, not necessarily its Claude:
a session in a different permission mode holds cross-session messages
for its operator's approval, may let them expire, or may refuse them
(a `[Cross-session delivery notice]` says so). Silence is never
agreement. With no answer, tell the operator which session is holding
your message.

## operator intervention

The operator can participate at any time by telling their session
what to say. The session sends it via SendMessage. The operator's
messages take priority: incorporate them, do not argue with them.

The operator can end the debate by saying "done" or "stop". When told
to stop, send a final PHASE: done message to the peer and report
the outcome to the operator.

## saving results

When a debate reaches consensus or is marked done, the proposer
records it the way the `onelog` skill records all work:

- every concrete implementation step becomes a todo row on the
  project's onelog page (`todo_add`, by the onelog skill's Todo
  rules), in order;
- the root cause and the decision go in the project's design or
  authority doc, the doc that owns that subject;
- today's daily page gets a bullet under the project's heading: the
  goal, what was decided, and where it is recorded.

## rules

- do not argue with the operator's messages. incorporate them
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
  own terms (the game's words, the terminology doc, the design doc).
  do not use code variable names, struct names, or type names in
  the proposal text. name the file and the line when pointing at
  code, but describe what it does in words the operator understands.
  "the manifest stamps every building as needing full power before
  it counts as built" not "CompletionState::Satisfied on
  BuildingSpec". the operator reads proposals cold; jargon forces
  them to open the file to understand what you mean
