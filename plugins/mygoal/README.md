# mygoal

`/mygoal <goal>` is a Claude Code mod, separate from the stock `/goal`. It
exists because Claude declares goals done by narrowing what the goal's words
mean, or drifts into work nobody asked for. Under `/mygoal`, Claude never
decides "done".

This file is for whoever changes the mod. Claude Code never loads it. What
Claude must follow while a goal is open, the mod tells it itself: how to
propose the check (`PROPOSING`, in the `propose_check` tool's description
and in the request while a check is needed) and what never happens
(`NEVER`, in every request while a goal is open), both in
`hooks/register.tsx`.

## Who decides done

Two things outside Claude, both every turn:

- **The approved finish check.** A command Claude proposes, with the git
  repo whose commits it tests, and the operator approves with a pane
  button only the operator can press. The mod runs it itself, once for
  each new commit in that repo, and reads the exit code. 0 is passed,
  anything else is failed. Claude cannot paste a result in its place.
- **The judge.** A separate call to Opus 5.5 (operator, 2026-10-02) that sees
  only the goal in the operator's words, the check's real output, and the
  code changes since approval. Never Claude's explanation. It answers what
  is missing and what was changed that the goal did not ask for.

Done only when the check exits 0 AND the judge says met AND nothing is
outside the goal, on one commit. A failed check is told to Claude once,
with what changed since the check before. There is no turn limit: a turn
with no new commit runs nothing and costs nothing (CI's rule, as Rust's
bors runs on a commit), so the operator's stop is the only limit.

## The flow

1. Operator types `/mygoal <goal>`. Only a goal typed at the prompt counts.
2. Claude proposes the finish check with the `propose_check` tool (`argv`,
   `why`), before any other work.
3. Operator approves or rejects in the mygoal pane. Until approved, Claude
   does no goal work.
4. Claude works. After each turn: check, judge, then done or the next turn.

## Where it lives and how it loads

The repo's `plugins/mygoal` is the plugin (at the repo root, not under
`claude/`, which sync.ps1 copies into `~/.claude`): `.claude-plugin/plugin.json`,
`hooks/hooks.json`, `hooks/register.tsx` with its test
`hooks/register.test.tsx`, and `types/index.d.ts`. The repo root's
`.claude-plugin/marketplace.json` lists it, and it is installed once:

```
claude plugin marketplace add C:\code\claude-blueprints
claude plugin install mygoal@claude-blueprints
```

A marketplace added from a local directory loads its plugins in place, so
the repo's files are what runs; sync.ps1 does not copy it. The plugin has no
skill: a plugin's skill also answers to its bare name, so a skill named
`mygoal` takes `/mygoal` and the mod's own command is refused at startup.

## How the mod works

`hooks/register.tsx` is the source; this section is a map of it, not a
second copy.

- **What it keeps** (`persist`, `load`): the goal's words, the status
  (`none`, `needs`, `approval`, `working`, `done`, `stopped`), the
  proposed check, the approved check, how many checks have run, the last
  result, the commit last checked (`checked`), the last thing Claude was
  told (`told`), and a note, in the plugin store under `goal`. Loaded at
  session start, so a goal carries into the next session. The commit at
  approval is kept apart as `base`, and the working tree then as
  `baseline`.
- **`/mygoal <goal>`** (`command.run`): only from the composer. Resets
  everything to `needs` and starts a turn asking for the check.
  `/mygoal stop` stops it. `/mygoal` with nothing after it opens the pane.
- **`propose_check`** (`tool.call`): keeps `argv` and `why`, sets
  `approval`, opens the pane.
- **The pane** (`ui.render`): approve keeps the working tree as the
  baseline, sets `working`, and starts a turn; reject sets `needs` and
  asks for another check; stop sets `stopped`; resume sets `working` with
  the turn count back at 0.
- **Every request** (`prompt.compose`): while a goal is open, the goal word
  for word, the step for the status, and `NEVER`. While a check is needed,
  the step carries `PROPOSING`; while working, it says to read the docs,
  decide, and never wait on the operator mid-goal.
- **Check-ups** (`session.send`, `session.receive`, `checkIn`): every
  session this one messages is tracked by name; its message or idle
  notice marks it heard. Once a minute while working, any tracked session
  silent for 10 minutes gets one turn started: check its repo, ask its
  status, decide what it waits on. The count starts again from that turn.
- **The pane's Agents list** (`agents`, `now`): each tracked session by
  name, how long since it was heard from (yellow past 10 minutes), what it
  last said (an idle notice adds "idle since"), and what it was last
  asked.
- **Guard** (`tool.call`): while a goal is open, any tool call whose text
  holds `plugins/store`, `plugins/mygoal` or `claude-blueprints/mygoal` is
  refused.
- **After each turn** (`turn.complete` then `evaluate`): skipped for
  subagents and stopped turns. Reads the check repo's HEAD.
  - **Same commit as last checked:** nothing runs, nothing is said. Only a
    turn whose words claim the goal done (`CLAIMS_DONE`) is sent back with
    the last result, once per commit.
  - **A new commit:** runs the check with no shell and a 10 minute limit,
    keeps its exit code and the last 4000 characters. If the output says
    it could not reach what it tests (`UNREACHABLE`, such as no game
    answering), that is not a result: the commit stays unchecked, so a
    later turn runs it again, and Claude is told once. Otherwise it takes
    the changes since approval (`git diff <base>` per file, only files
    whose diff differs from the baseline, cut at 40000 characters) and
    asks the judge (`claude-opus-5-5`, effort high) for `met`, `missing`,
    `outside_goal` as JSON. Done only on exit 0, met, and nothing outside
    the goal; otherwise Claude is told once, with the result lines that
    changed since the check before (`changedLines`).

## Known gaps (to iterate on)

- **New files are invisible to the judge.** An untracked file shows only
  as `(new file, not tracked yet)`, with no contents.
- **Edits already in the tree count against the goal.** A file with
  uncommitted changes before approval, changed again, reaches the judge
  as its whole diff against HEAD, earlier edits included, which can read
  as outside the goal.
- **The tests are not locked.** The guard covers the store and the mod,
  not the files the check runs; only committing them before the goal
  starts keeps a test from being changed to pass.
- **A judge with no answer** (no reply, or no JSON) counts as not met and
  the next turn starts.
- **Uncommitted work is not checked.** The check runs only on a new
  commit, so work left uncommitted is never tested until it is committed.
- **A turn that ends a goal without claiming it done is not sent back.**
  Only `CLAIMS_DONE`'s words are caught; a turn that drifts off the goal
  in other words is not.
