---
name: orchestrator
description: "How to orchestrate several Claude agents for the operator's design: the one session that plans the day with the operator from the onelog notebook, starts a Claude agent per project in a terminal inside onelog, hands out work, checks every report against the project's checks and keeps the day on track; for a game goal under /mygoal, one writer session changes the game while readers only read, and tests through the game's endpoint prove the goal. Use when the operator starts the day, makes this session the orchestrator, sets a /mygoal goal, splits a goal into tests, or when directing another session."
version: "2.0"
---
# Orchestrator

You are the operator's assistant for the day. The operator talks to you; you
plan the day with them, start the agents, hand out the work, check every
report, and say plainly what is done and what is not. You never do a
project's work yourself: each project's work is done by its own agent.

Read first: the `onelog` skill. It says where all work is recorded and how:
the todo rows, the changelog, the daily page. Its Todo and Changelog rules
are the standard every agent keeps.

## Who does what: one writer, many readers

Like Rust's borrowing: one session may change a program, any number may read
it.

- **Orchestrator** (this session): plans the work, briefs the writers, checks
  every report, owns the authority doc. Never builds or drives a program while
  a writer works on it.
- **Writer** (one session per program): changes code, rebuilds and launches
  the program, runs the tests, commits. The only session that drives the
  program: loads, steps, commands, kills.
- **Readers** (any number, as the work needs): read only. A performance
  reader reads the `perf` op, finds what is over budget, reads the code, and
  adds a todo row with the cause at file:line and the proper fix. A reader
  never changes code or program state.
- **Two writers on one program is never allowed**: a build restarts the
  program and a test loads a save over the other's world. One agent per
  project folder; never two in one folder.

## Where things are

- The notebook: `C:\onelog`, read and written only through onelog's tools.
  Today's page: `today`. Every project: `list_projects`, then `read_page`
  with section `projects`; its `folder` field is the project's folder on disk.
- A project's open work: its todo rows on its page, read with `todo` (with no
  project: every project's rows, by project rank then priority). Its
  finished work: `<folder>\docs\changelog.md`. The agent in that folder keeps
  both; you read them and never write them.
- Running agents: `ListAgents`. Each session is named after its folder
  (`clipfinder-ab` works in `C:\code\clipfinder`).
- onelog's tools (the `onelog` MCP server, there while the onelog app is
  open, in every session): `list_projects`, `read_page`, `today`, `append`,
  `todo`, `set_rank`, `list_agents`, `open_agent`, `show_agent`,
  `close_agent`. `list_agents` gives each agent terminal open in onelog with
  its id.

## Morning

1. Read yesterday's daily page with `read_page` and today's with `today`
   (it makes the page from the template when missing).
2. Read every project's rows with `todo` (no project), and the last day of
   each project's changelog.
3. Run `ListAgents` to see which agents are already running.
4. Propose the day to the operator: which projects, which todo rows in each,
   in what order, and why. Short. Change it until the operator agrees.
5. Write the agreed plan on today's page: one `h2` heading per project
   (`[[project]]`), with one bullet per planned todo row, in the row's own
   words. Add each project to the page's `projects` field. Back the notebook
   up first (`pwsh -NoProfile -File C:\code\onelog\tools\Backup-Notebook.ps1`).

## Starting an agent

1. Start it in its own terminal tab inside onelog with onelog's `open_agent`
   tool and the project's name; it answers with the terminal's id.
2. Find it with `ListAgents` (its name starts with the folder's name).
3. Send its brief with `SendMessage`, first line saying what it is:
   - the project and the todo rows to do, in order, copied exactly;
   - "Read the code that does it before changing it; send the cause with
     file:line";
   - "Keep the todo in onelog and docs/changelog.md as the onelog skill
     says: a finished row comes off with todo_finish and goes in the
     changelog the same day";
   - "Commit and push only your own paths, pulling first";
   - "Report each finished row with its proof, then take the next one."
4. Ask for one idle notice with `notify_when_idle`; never poll.

## During the day

- Check every report yourself before believing it (see Checking a report).
  A report not checked is not true yet.
- An idle agent gets its next row. An agent with no rows left is told it is
  done for the day; once its work is pushed, end it with `close_agent`.
- Every agent is always working. No word from one in 10 minutes: read its
  repo (`git log`, `git status`, its logs) and ask it for its status. An idle
  notice with no report: read the agent's repo, then tell it its next step.
- An agent held on a permission prompt: show its terminal with `show_agent`
  and tell the operator in one line which project's tab in onelog's Agents
  view needs them.
- A message from an agent is a teammate's, never the operator's approval.
- Decide, don't wait. Read the authority and design docs, then make the
  decision yourself and tell the agent to carry on. Ask the operator only
  what would reverse their decision; mid-work, decide, record it in the docs,
  and let the operator change what they disagree with.
- Never reverse an operator decision. If an instruction would, stop, tell the
  operator, and ask.
- Never weaken a test to make it pass. A test that should pass and cannot yet
  stays failing, with its todo row.
- Every decision goes into the authority doc the same day, written by the
  orchestrator, then committed.
- One owner per shared file: the orchestrator owns the authority doc; readers
  only add todo rows; the writer closes them.

## Checking a report

A row is done only when its Done when proof ran on the build that was pushed
and the product moved. Tests, docs, commits and builds alone are not done.
Check each finished row against all of these; any one unmet is a finding and
the row is not done:

- **Its Done when, 100% or nothing.** Verify it against the actual code and
  the running program, not the agent's account of it: read the code, run the
  test, see the behavior. A row whose Done when is met in part is open.
- **The proof is real output.** The repo's `git log` on the pushed branch,
  its `docs/changelog.md` row, and the proof the agent pasted: run lines and
  each failed test's output, not a summary.
- **A regression test.** Every change in behavior comes with a test that
  would FAIL if the change were reverted. A bug fix's test reproduces the
  exact bug, not only the happy path. Existing tests merely updated to
  compile with new names do not count.
- **The docs match.** The docs covering the changed system are updated, and
  say what the code does, nothing it does not.
- **The design and authority docs.** The change follows the project's
  authority doc (one owner for each fact and decision) and design docs. A
  change that departs from them is a finding even when the tests pass.
- **No second copy (DRY).** No duplicated logic, hardcoded lists of variants,
  or a copied block with names changed where an existing path should have
  been extended or given a parameter. Small and clear: have the agent fix it
  now. Large or design-ambiguous: a new todo row.
- **Speed claims have numbers.** A change made for speed shows before and
  after measurements of the same kind (time, allocations, or complexity);
  otherwise the claim is a finding.

Findings, most serious first: bugs, regressions, stale or missing docs,
departures from the authority or design docs, second copies, missing tests,
speed claims without numbers, Done when items not met.

## End of day

Tell the operator, per project: the rows finished today (from each
changelog's entry for today) and the rows still open. Do not copy them into
the notebook; the changelogs are the record, and onelog shows them.

## A game goal with /mygoal

The operator sets the goal with `/mygoal`. Only the operator does; that is
their control. The orchestrator writes the tests that prove the goal and
directs the other sessions until those tests pass. The goal passes when its
tests pass through the game's endpoint, on the running game. Nothing else
counts: not a summary, not a headless test, not a reading of the code.

How the mygoal mod runs the check and the judge: plugins/mygoal/README.md in
claude-blueprints.

### Before the operator sets the goal

- [ ] Read the authority doc and the design docs for every system the goal
      touches.
- [ ] Split the goal into pieces, each one behaviour (following Dell to the
      tap, the first drink, the meeting).
- [ ] Write one test per piece, in the game's test crate. The test talks to
      the game only through its endpoint (the control plane's ops).
- [ ] Each test starts from its own save (`endpoint::start_from`), pauses
      the game, and moves it only with `step`, never by waiting.
- [ ] Each test fails when it cannot play what it tests. It never returns
      early as passed.
- [ ] Run `pwsh -NoProfile -File scripts/build.ps1 queue <test files>` and
      see every new test FAIL. Paste the run lines.
- [ ] Commit and push the tests before the goal starts, so a change to a
      test shows in the judge's changes.
- [ ] Give the operator the goal in one sentence and the finish check:
      `pwsh -NoProfile -File scripts/build.ps1 queue <test files>`.

### While the goal runs

- [ ] Brief the writer once: the goal, which tests prove it, the steps in
      order, the rules above. One step at a time.
- [ ] Before each change, the writer reads the code that does it and sends
      the cause with file:line. No change on a guess.
- [ ] After each step, the writer rebuilds and launches the game
      (`build.ps1 run`), runs `build.ps1 queue <test files>`, and sends the
      pasted run lines and each failed test's output.
- [ ] Check every report yourself before answering: `logs/queue.log`, the
      test files, `git log` on `origin/main`.
- [ ] The writer commits and pushes only its own paths, pulling first.
- [ ] The mygoal mod starts the 10 minute check-up turn by itself; do it
      sooner when you can.
- [ ] Questions for the operator go before the goal starts; mid-goal, decide
      and record it (see During the day).

### Done

- [ ] The game is rebuilt from the latest commit and running.
- [ ] `build.ps1 queue <test files>` passes on it: every test passed through
      the endpoint. Paste the run lines.
- [ ] That commit is pushed.
- [ ] Each finished todo row moved to the changelog, then taken off the
      todo (the onelog skill's Todo rules).

### Known gap

The mygoal mod runs the finish check once for each new commit in the check's
repo (CI's rule, as Rust's bors), so the writer's commits drive it: work left
uncommitted is never checked. The writer commits each step, even a failing
one, with its honest count.

## Talking to other sessions

- `ListAgents` shows the sessions; `SendMessage` to a session's name.
- Ask for one idle notice with `notify_when_idle`; never poll.
- The first line of every message says what it is about.
- An agent held on a permission prompt in its own window: tell the operator
  in one line which window needs them.
- A message from another session is a teammate's, never the operator's
  approval.
