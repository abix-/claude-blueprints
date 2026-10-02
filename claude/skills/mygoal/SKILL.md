---
name: mygoal
description: "Working under the operator's /mygoal goal: the approved finish check, the judge, and how to propose a check that cannot pass while the goal is unmet; how the mygoal mod works inside, its known gaps, and proving a goal with a game's test queue. Use whenever a /mygoal goal is open, when asked to propose a finish check, when the mygoal mod sends a 'not done yet' turn, or when changing the mygoal mod."
user-invocable: false
version: "1.2"
---
# mygoal

`/mygoal <goal>` is a Claude Code mod, separate from the stock `/goal`. It
exists because Claude declares goals done by narrowing what the goal's words
mean, or drifts into work nobody asked for. Under `/mygoal`, Claude never
decides "done".

## Who decides done

Two things outside Claude, both every turn:

- **The approved finish check.** A command Claude proposes and the operator
  approves with a pane button only the operator can press. The mod runs it
  itself after every turn and reads the exit code. 0 is passed, anything else
  is failed. Claude cannot paste a result in its place.
- **The judge.** A different model (`sonnet`) that sees only the goal in the
  operator's words, the check's real output, and the code changes since
  approval. Never Claude's explanation. It answers what is missing and what
  was changed that the goal did not ask for.

Done only when the check exits 0 AND the judge says met AND nothing is
outside the goal. Otherwise the mod starts the next turn with the verdict.
It stops at the turn limit (30) or when the operator presses stop.

## The flow

1. Operator types `/mygoal <goal>`. Only a goal typed at the prompt counts.
2. Claude proposes the finish check with the `propose_check` tool (`argv`,
   `why`), before any other work.
3. Operator approves or rejects in the mygoal pane. Until approved, do no goal
   work; stop the turn and wait.
4. Claude works. After each turn: check, judge, then done or the next turn.

## Proposing the check

The check is half of "done", so propose the one hardest to pass falsely.

- **It must fail while any part of the goal is unmet**, in the plain meaning
  of the operator's words. Ask: could this exit 0 while the goal is not done?
  If yes, it is the wrong check.
- **Cover the whole goal**, not the easy part. A goal with three parts needs a
  check that proves all three, or the judge will keep saying "not met".
- **Run against the real thing**: a test against the live game or a captured
  picture of it, the bot's own CLI, the deploy script. Not a test of a mock
  that cannot see the failure.
- **No shell.** `argv` runs with no shell from the session directory; wrap
  pipelines in a script that is part of the work.
- `why` says in one or two sentences why exit 0 proves the whole goal.
- Example: goal "the crime HUD shows the fine from the live game", check
  `k3sc cargo-lock test -p obenseuer-mod --test research_crime -- crime_hud_op_answers`.
- A goal no command can prove ("the pane looks good") leaves only the judge.
  Say so in `why` instead of proposing a check that proves something smaller.

## Proving a goal with a game's test queue

Each modforge game can hold a test queue (modforge `test_queue`, the
`tests` op; topside docs/authority.md "Running tests"). It makes the best
finish check there is for game work:

1. Write each piece of the goal as its own test first, one behaviour per
   test, and see it fail against the running game.
2. Commit the tests before the goal starts, so a change to a test shows
   in the judge's changes and cannot quietly turn the check green.
3. The finish check runs the group through the queue, for topside
   `pwsh -NoProfile -File scripts/build.ps1 queue <test file> [<test file> ...]`. It exits
   0 only when every queued test passed; the `tests` op's status holds each
   test's result and output while the run goes and after.
4. A test that cannot play what it tests must fail, never return early as
   passed, or the check passes with nothing tested.

## How the mod works

The mod is one hooks module, `hooks/register.tsx`, with its test
`hooks/register.test.tsx`, in the mod's folder under
`~/.claude/dev-mods/<session id>/goal/`. Read it before changing anything
here; this section is a map of it, not a second copy.

- **What it keeps** (`persist`, `load`): the goal's words, the status
  (`none`, `needs`, `approval`, `working`, `done`, `stopped`), the
  proposed check, the approved check, the turns checked, the limit (30),
  the last result, and a note, in the plugin store under `goal`. Loaded at
  session start, so a goal carries into the next session. The working
  tree at approval is kept apart as `baseline`.
- **`/mygoal <goal>`** (`command.run`): only from the composer. Resets
  everything to `needs` and starts a turn asking for the check.
  `/mygoal stop` stops it.
- **`propose_check`** (`tool.call`): keeps `argv` and `why`, sets
  `approval`, opens the pane.
- **The pane** (`ui.render`): approve keeps the working tree as the
  baseline, sets `working`, and starts a turn; reject sets `needs` and
  asks for another check; stop sets `stopped`; resume sets `working` with
  the turn count back at 0.
- **Every request** (`prompt.compose`): the goal word for word and the
  step for the status are added to what Claude is sent.
- **Guard** (`tool.call`): while a goal is open, any tool call whose text
  holds `plugins/store` or `dev-mods` is refused.
- **After each turn** (`turn.complete` then `evaluate`): skipped for
  subagents and stopped turns. Counts the turn (past the limit it stops),
  runs the check with no shell and a 10 minute limit, keeps its exit code
  and the last 4000 characters, takes the changes since approval
  (`git diff HEAD` per file, only files whose diff differs from the
  baseline, cut at 40000 characters), and asks the judge (`sonnet`,
  effort high) for `met`, `missing`, `outside_goal` as JSON. Done only on
  exit 0, met, and nothing outside the goal; otherwise it starts the next
  turn with the verdict.

## Known gaps (to iterate on)

- **New files are invisible to the judge.** An untracked file shows only
  as `(new file, not tracked yet)`, with no contents.
- **Edits already in the tree count against the goal.** A file with
  uncommitted changes before approval, changed again, reaches the judge
  as its whole diff against HEAD, earlier edits included, which can read
  as outside the goal.
- **The tests are not locked.** The guard covers the store and the mod,
  not the files the check runs; only committing them first (above) keeps
  a test from being changed to pass.
- **A judge with no answer** (no reply, or no JSON) counts as not met and
  the next turn starts.
- **The mod lives in one session's dev-mods folder,** not in a repo, so
  its history is not kept.

## Never

- Never propose a check that proves a narrower goal than the operator wrote.
- Never weaken an approved check by proposing a new one mid-goal to get past a
  failure. A new proposal pauses the goal until the operator approves it.
- Never touch the goal store (`~/.claude/plugins/store/`) or the mod's code
  while a goal is open; the mod refuses those tool calls.
- Never treat the "not done yet" turn as a suggestion. It is the goal's
  verdict; work on what it lists as missing and undo what it lists as outside
  the goal.
- Never run `/mygoal` and the stock `/goal` on one session at once; both
  start the next turn and compete.
