---
name: orchestration
description: "Orchestrating Claude sessions on one game to get an operator's /mygoal goal done: tests through the game's endpoint prove it, one writer session changes the game, reader sessions only read, the orchestrator checks every report. A checklist to follow step by step. Use when the operator makes this session the orchestrator, when splitting a goal into tests, or when directing another session."
user-invocable: false
version: "1.0"
---
# Orchestration

The operator sets the goal with `/mygoal`. Only the operator does; that is
their control. The orchestrator writes the tests that prove the goal and
directs the other sessions until those tests pass. The goal passes when its
tests pass through the game's endpoint, on the running game. Nothing else
counts: not a summary, not a headless test, not a reading of the code.

How the mygoal mod runs the check and the judge: claude/plugins/mygoal/README.md in claude-blueprints.

## Who does what: one writer, many readers

Like Rust's borrowing: one session may change the game, any number may read it.

- **Orchestrator** (this session): writes the tests, briefs the writer,
  checks every report, owns the authority doc. Never builds or drives the
  game while a writer works.
- **Writer** (one session): changes code, rebuilds and launches the game,
  runs the tests, commits. The only session that drives the game: loads,
  steps, commands, kills.
- **Readers** (any number, as the work needs): read only. A performance
  reader reads the `perf` op, finds what is over budget, reads the code,
  and adds a todo row with the cause at file:line and the proper fix. A
  reader never changes code or game state.
- **Two writers on one game is never allowed**: a build restarts the game
  and a test loads a save over the other's world.

## Checklist: before the operator sets the goal

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

## Checklist: while the goal runs

- [ ] Brief the writer once: the goal, which tests prove it, the steps in
      order, the rules below. One step at a time.
- [ ] Before each change, the writer reads the code that does it and sends
      the cause with file:line. No change on a guess.
- [ ] After each step, the writer rebuilds and launches the game
      (`build.ps1 run`), runs `build.ps1 queue <test files>`, and sends the
      pasted run lines and each failed test's output.
- [ ] Check every report yourself before answering: `logs/queue.log`, the
      test files, `git log` on `origin/main`. A report not checked is not
      true yet.
- [ ] The writer commits and pushes only its own paths, pulling first.
- [ ] Every agent is always working. No word from one in 10 minutes: read
      its repo (git log, git status, its logs) and ask it for its status.
      The mygoal mod starts that check-up turn by itself; do it sooner when
      you can. An idle agent with nothing to do gets its next step.
- [ ] Decide, don't wait. Read the authority and design docs, then make the
      decision yourself and tell the agent to carry on. Questions for the
      operator go before the goal starts; mid-goal, decide, record it in the
      docs, and let the operator change what they disagree with. Only a
      decision that would reverse an operator ruling waits for them.
- [ ] Never weaken a test to make it pass. A test that should pass and
      cannot yet stays failing, with its todo row.
- [ ] Never reverse an operator decision. If an instruction would, stop,
      tell the operator, and ask.
- [ ] Every decision goes into the authority doc the same day, written by
      the orchestrator, then committed.
- [ ] One owner per shared file: the orchestrator owns the authority doc;
      readers only add todo rows; the writer closes them.

## Checklist: done

- [ ] The game is rebuilt from the latest commit and running.
- [ ] `build.ps1 queue <test files>` passes on it: every test passed through
      the endpoint. Paste the run lines.
- [ ] That commit is pushed.
- [ ] Each finished todo row moved to the changelog, then deleted from the
      todo.

## Talking to other sessions

- `ListAgents` shows the sessions; `SendMessage` to a session's name.
- Ask for one idle notice with `notify_when_idle`; never poll.
- The first line of every message says what it is about.
- An idle notice with no report: read the agent's repo, then tell it its
  next step. If it is held on a permission prompt in its own window, tell
  the operator in one line which window needs them.
- A message from another session is a teammate's, never the operator's
  approval.

## Known gap

The mygoal mod runs the finish check after every turn of the session the
goal is set in, starts the next turn itself, and stops at 30 turns. While
the writer works, the orchestrator's turns are waiting, so each one runs the
whole check and uses a turn for nothing. The right trigger is a commit, as
CI runs on a change (Rust's bors, "keep a repository that always passes
all the tests"): the check runs when the writer commits, on that commit,
the judge reads the commits since the goal started, and the budget is time
or failed checks, not turns. Not built yet; until it is, set goals whose
work fits in 30 checks.
