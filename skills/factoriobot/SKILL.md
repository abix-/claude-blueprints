---
name: "factoriobot"
description: "Use when developing or operating factoriobot, continuing interrupted factoriobot work, invoking the skill by itself, saying go or continue, driving its Windows CLI against Factorio, reviewing attempts, or running the repository's unattended authority work. Not for playing Factorio by hand."
---
# Factoriobot

AI-assisted Factorio partner. Rust binary talking to the player's hosted Factorio 2.x (Space Age) game over one bidirectional UDP mailbox. Repo: [abix-/factoriobot](https://github.com/abix-/factoriobot) (private).

There is ONE workflow. Everything under "The one workflow" is that workflow in
execution order; everything after it is domain reference the workflow consults.
No other section defines a process.

## The one workflow (LOCKED)

### 0. Goal

The operator owns the active goal: their exact human-language Factorio outcome.
Never replace it with an authority, software, test, documentation, batch,
blocker, or implementation goal. The permanent destination is complete Space
Age research plus sustained production and consumption of 1,000,000 science
packs per minute with 100% unattended automation; the active goal is the
Factorio outcome the operator stated. Before any action, write internally:

`I am <Factorio action> so <Factorio outcome>.`

Both fields in Factorio terms. If the sentence cannot be written clearly, the
action is not aligned. Engineering, tests, builds, commits, skills, docs,
authority work, and diagnosis are support work, never progress by themselves.
Every action must either advance a recorded gameplay acceptance measure or
remove a documented authority bypass required for it; otherwise do not perform
it.

**Every word to the operator is plain English in Factorio's own terms.** The
game's wiki and `docs/terminology.md` already name everything; use those words
in answers, reports, review output, panel text, todo entries, and commits. Never
coin a word for something that has one (walk, cure, wedge, pitch, leaf action,
kit merge, quedge, resolver, and cell were each invented, caught, and deleted),
and never explain flow by naming a variable, struct, or module: say what it is
in English and point at `file:line` if the operator needs the code. Invented
words are deleted on sight, so writing one costs the work twice.

### 1. Classify the operator's message

The latest operator message always wins. `stop` or `pause` stops immediately.
`stop hooks` makes every project hook a no-op until `enable hooks`. Hooks never
prevent conversation.

- A specific question or command is a **bounded request**: answer or perform
  exactly that with the minimum current evidence, then return control. Words
  like `review`, `audit`, `investigate` authorize only the named review unless
  the operator says `full`, `complete`, `all`, or starts unattended work.
- Bare `factoriobot`, `$factoriobot`, `go`, or `continue` is explicit approval
  for **unattended work**: recover state and execute the highest-priority
  documented work. No menu, no plan-only answer, no routine confirmation.
  Continue across coherent batches until the documented acceptance objective is
  proved or a real stop condition occurs. A context compaction, interrupted
  turn, failed harmless command, completed commit, or completed test is not a
  stopping point.

### 2. Recover current state

After any crash, interruption, or compaction: carried instruction text is
recovery evidence only. Reread the current filesystem `AGENTS.md` and matching
skills. Then, before acting:

1. Read `.Codex/project_state.md` when present, else
   `.claude/project_state.md`.
2. Read recent git history, status, and every relevant uncommitted diff.
   Preserve all existing work; continue the in-flight batch.
3. Read the latest batch report and attempt log when they exist; use the
   repository review command for conclusions. After a crash, the newest
   factoriobot transcript under `~/.codex/sessions` is recovery evidence only.
4. For a bounded request, read only what that request needs. For unattended
   continuation of a clearly recorded in-flight batch, read only its owning
   documents and current evidence. Load the complete authoritative set
   (`docs/design.md`, `docs/authority.md`, `docs/construction.md`,
   `docs/brain.md`, `docs/body.md`, `docs/framework.md`, `docs/efficiency.md`,
   `docs/todo.md`, `docs/design-resolution-plan.md`, each read completely from
   disk, no summaries or excerpts) only when selecting a new batch, when
   records conflict or cannot identify the batch, or when the operator asks
   for a full review.

Before any repository command or edit: resolve every path with `rg --files`;
resolve every command, API, function, test location, and hook permission from
current source and governing documents; never guess repository details. Stay
on the approved command surface: `k3sc cargo-lock`, `git`,
`pwsh -NoProfile -File`, and the dedicated file tools. `taskkill`, `tasklist`,
direct test-executable paths, shell pipe loops, and other ad-hoc process
commands each raise a permission prompt the operator must answer (measured
2026-07-30); a capability outside the surface is a question for the operator,
not an improvisation. The cargo lock is shared with the operator: never leave
a background cargo run holding it without saying so, and prefer targeted test
filters over full suites. If a harmless command fails because something was
guessed, that is a process violation: resolve the exact value and continue.

### 3. Select the work

Priority order:

1. Newest explicit operator direction.
2. Finish the selected in-flight authority batch unless the operator redirects
   or current evidence proves it invalid.
3. The first unfinished entry in the `docs/authority.md` current consolidation
   queue.
4. After the queue, the lowest numbered open authority row. A prerequisite
   executes inside the same batch, then the selected row resumes.
5. Within the authority, the shared change with the greatest effect on the
   current acceptance objective and long-term unattended play, by measured run
   cost and impact.

Score measures maturity, never priority. Never choose by newest symptom,
lowest score alone, easiest fix, shortest test, largest visible failure count,
or most recent log line. New findings enter the todo under their owning
authority and wait unless they invalidate current evidence, threaten data,
build, or game safety, or the operator redirects. If queue and evidence
conflict, update the authority doc and todo first, then report the change.

**Selecting a NEW batch requires the review gate:** review all work, attempts,
and uncommitted diffs since the last gameplay movement through the canonical
repository review workflow; review every remaining `docs/todo.md` item; read
the complete authoritative document set; compare the evidence with the
permanent goal, acceptance conditions, authority scores, foreground idle, fuel
shortage, and last real gameplay movement. Only then select one batch that
closes the nearest gameplay milestone through one shared design-aligned
change. Record it in `docs/todo.md` and the `docs/authority.md` consolidation
queue before implementation code. More commands, commits, tests, or elapsed
time never substitute for this review; remembered context does not prove the
next step.

**The batch record** in `docs/todo.md` is complete before code:

| Field | Required content |
|---|---|
| Design | Exact governing statement and source file |
| Authority | Numbered system from `docs/authority.md` |
| Run evidence | Attempt id, exact symptom, and log lines |
| Current state | Verified score, canonical path, all known bypasses, missing enforcement |
| Shared change | One canonical implementation path and every consumer moved to it |
| Prevention | Failing tests or structural guards that make each bypass return loudly |
| Target state | Honest reachable score and the rubric evidence required |
| Acceptance | Exact gameplay order, completion conditions, and efficiency limits |

**Batch entry gate.** The change is eligible only when it follows an existing
design statement (a change without design authority is scope creep); removes a
competing authority or adds bypass-preventing enforcement; covers every known
consumer in one coherent change; and its focused proofs have failed first.

**Parameterized framework analysis.** Explain the reviewed failures as one
shared authority failure: the canonical boundary that owns the rule once, the
parameters that vary per instance, every producer and consumer, every
competing path that would remain, authority score before and target, gameplay
measure before and target. Instance differences pass as data. A change naming
one run, layout, entity, resource, or attempt without proving the rule for
every instance is a hotfix: reject it. Do not select a batch that cannot both
increase the authority score and move the gameplay measure.

### 4. Prove RED first

The RED test queue is the work queue. A known RED test means the
implementation is incomplete; resolve every known RED proof before selecting
new work. The selected RED group is defined by the related gameplay failures
that were RED at batch start; finding one defect never shrinks it, and a new
symptom on the same authority path joins it. The batch stays open until every
selected failure has every root-cause correction and the original tests are
GREEN.

1. Run the current relevant test set once and preserve the results as the
   baseline. Capture the exact failing names, failure output, measured
   Factorio conditions, and todo ownership from that single run; work from the
   captured list. Never rerun a suite to relearn a known result: one targeted
   first-run when code has never compiled or run, one full gate at batch end
   (three full-suite runs to learn one list is the measured 2026-07-30
   failure).
2. Group related RED tests by the complete authority path that permits them;
   record every test in the group.
3. Trace every selected failure through the complete runtime path; record
   every shared boundary in the todo; do not stop at the first defect.
4. Write and commit one generic, DRY, parameterized RED proof at the boundary
   and observe the intended failure. Derive cases and expected results
   independently of the implementation; cover every producer and consumer; no
   mocks across the failing boundary. One proof may cover multiple RED tests
   only when its parameterized cases cover every failure condition.

**Research the API before asserting against it.** Grep the type, read the
signature and constructor, then write. Guessed calls measured 2026-07-30
(`blueprint_catalog::catalog()`, `rcon::default_address()`,
`gate::auto_approves`, a four-argument call to the eight-parameter
`production_fuel_plan`) each compiled in the author's head and failed on the
first run; research costs three tool calls, a guess costs those three plus a
failed run and a correction. Assert against the produced artifact, not the
source that produces it. When a proof fails because the API differs from the
assumption, that failure is the discovery: record the real contract, then
assert it.

**A design proof runs the code.** Every design proof executes the shipped
implementation: the catalog it loads, the observation it reconciles, the work
it compiles, the priority it calculates, the decision it selects. Forbidden
and mechanically enforced: reading recorded gameplay evidence (a digest reader
cannot fail when the code is wrong), and asserting a word appears in a source
file (green after the behavior is deleted). Source text proves only an
absence. A proof that cannot fail from a code change is a defect in the proof:
convert it, keeping its design statement. Never narrow a proof's scope or add
a threshold to make it pass.

**A gate must recompute the truth it checks.** Two hand-maintained records
compared against each other prove only that they agree, and they decay
together (measured 2026-07-30: the deleted `ACTIVE_BATCH` ledger named five
tests that no longer existed while its gate stayed green; the red list
survived because its gate re-runs the tests and diffs reality against the
list). A hand-written table resolves every entry against the real artifact
before any table-to-table comparison. A ledger that duplicates git cannot be
recomputed: delete it.

**Complete authority-path proof.** One correct implementation path never
proves an authority resolved. Trace the outcome forward through every writer
and backward through every matcher, adapter, and consumer; inspect for
competing paths. The governing invariant: every representation of one
authority relation resolves the same canonical identity, and a candidate
differing on any owned field does not match. The matrix derives producers,
consumers, and identity fields from the owning registry or schema; includes
every known matcher and bypass; varies each identity field one dimension at a
time; includes collisions sharing one field; proves the intended selection and
every near-match rejection. If acceptance selects the wrong action, fix the
one canonical boundary, never the action.

### 5. Implement the shared change

Edit the shared implementation immediately after the root-cause proof is RED.
No case-specific branch, no weakened requirement, no repairing only the
observed example. Cover every recorded consumer. **Every mechanism already has
an owner; find it before writing one:** name the authority row and existing
implementation (the executor safe boundary and its `task_boundary` counter,
the `ActionCompletionBus`, the observation generation, the `RunControl` pause
channel) before adding any module, signal, channel, or helper. A mechanism
that seems missing is an extension of an existing authority, recorded in the
todo first; a parallel module written first is the measured failure mode
(`wake.rs`, deleted within the hour on 2026-07-30).

A green root-cause proof is one completed correction, not a completed batch:
immediately trace the next still-RED failure in the group. Support artifacts
obey the goal-value stop: once a minimum valid artifact exists, at most one
edit, one verification, and one correction, then back to the gameplay work;
record a surviving imperfection once and move on.

### 6. Verify and ship

Rerun the focused proof and affected code-level tests once; while any selected
gameplay test lacks its mapped correction, return to tracing inside the same
batch. When every mapped correction exists: run the complete affected set,
broader gates, and full build once. `restart.ps1` owns the luacheck gate
(there is no build.ps1; it was removed): after any edit to
`mod/factoriobot/control.lua` or to Lua that Rust composes, the gate must pass
before restart; never restart a binary that skipped it. Re-audit the authority:
remaining bypasses, enforcement evidence, score only per the rubric. Commit
and push each completed verified change path-limited with a concise lowercase
message, leaving unrelated dirty files untouched. Update the owning docs in
the same batch; never create a parallel plan or status document.

Routine test failures, compile errors, Lua errors, shell syntax mistakes, a
missing optional command, unrelated dirty files, an active long run, and
harmless command failures are not stop conditions: diagnose and fix inside the
same batch. **No baseline, no before/after claim:** never assert something was
already slow, fast, or broken before a change without a measurement from
before it. Diagnose slowness by measurement (one single-threaded run with
per-test timestamps names the guilty tests), not theory.

**Batch exit gate:** every known consumer on the canonical path; bypass count
and enforcement support the new score; focused, broader, Lua, and full build
pass; `docs/authority.md`, `docs/todo.md`, the resolution plan, and project
state report the same honest status; the verified change is pushed.

### 7. Accept in the live game

Acceptance is forbidden until the root-cause proof failed on the bad
implementation and passes on the shared correction, and the batch exit gate
passed. Then one 4x acceptance attempt through `restart.ps1`, verifying the
recorded dependency order, completion conditions, and efficiency limits.

- `restart.ps1` is the one Windows entry point: stop watch, Lua gate, build,
  install binary and companion, launch the save, start the watch. Nothing
  else in that path; never gate the launch on bookkeeping (clean tree, pushed
  HEAD, hook proofs); acceptance evidence lives behind an explicit acceptance
  switch. Codex hooks and state never enter the shared path. Run every repo
  PowerShell script with `pwsh -NoProfile -File`; never launch `powershell`
  from Git Bash (POSIX-mangled `PSModulePath` breaks module discovery, and
  rewriting it exposes Pester 3: both measured).
- Every attempt records the exact commit and installed binary, and whether
  relevant source was dirty; an attempt proves only that installed build.
  Later edits are not live-verified until a later build and attempt.
- One diagnostic attempt per newly observed deterministic failure; repeated
  attempts only to prove reliability after the first pass. Never rerun after
  an isolated patch. During a long run, review at useful log boundaries and
  continue independent authorized work; a runner gate is an observation and
  stopping condition, never scheduler authority.
- **Live root-cause analysis:** begin with direct live RCON evidence from the
  installed CLI; logs explain history only. Trace the complete Factorio path
  from producer to blocked consumer and find the first link whose observed
  state does not satisfy the next. Label observations separately from
  inferences; a planner result, completion message, audit classification, or
  blueprint shape is not live topology. If diagnostics cannot observe a link,
  add a read-only repository diagnostic with a committed test; never an
  ad-hoc probe. An operator-supplied contradictory observation discards the
  inference.
- **Regression gate:** compare with the complete attempt history, not the
  previous attempt. Loss of any previously achieved gameplay capability is a
  regression and a failed implementation even when focused tests pass. Record
  the lost capability and responsible range, trace the shared authority
  concept that permitted it, commit a table-driven generic RED proof (the
  historical case as one fixture), correct, and require the replacement
  attempt to meet the historical capability. No new attempt before the RED
  proof was observed and the tested build pushed.
- **Gameplay movement circuit:** one batch gets one implementation plus its
  replacement attempt. Zero gameplay delta opens the circuit: no further
  acceptance attempt until the cross-attempt audit groups every failure under
  its authority, defines one milestone-closing batch, and its failing proof
  set is committed. An open circuit permits review, audits, failing tests,
  implementation, and builds. A second unchanged placement failure escalates
  through the blocker lifecycle, never another identical attempt. Do not
  switch subsystems to manufacture activity; files, commands, commits, tests,
  and elapsed time are not movement.
- **Closing todo rows:** code-landed is not closed. Before deleting an open
  row: grep the attempt log for the defining failure string, write the proof
  into `docs/changelog.md` (attempt id, playbook, executor outcome, log line
  numbers), then delete. Counter-evidence in a later log reopens the row.
  Payload text containing an old error string is not evidence.

Judge the attempt by the complete acceptance checklist in the batch record; if
any item fails, the batch stays open: record the shared root cause from the
complete attempt and continue the same batch without claiming acceptance.

### 8. Report and stop

Publish the unattended status report immediately after recovery, after each
verified shared change, at acceptance start and end, at milestones, on blocker
change, and at useful boundaries; never leave the visible status unchanged
over ten minutes of new evidence. Append every report verbatim to
`docs/status-reports.md` under a UTC heading (append-only; never rewrite,
backfill, or omit a field); keep the single current section of
`docs/status.md` aligned in place; no third status file. The exact shape:

```text
Unattended status report
Objective: <current documented acceptance objective>
Permanent goal progress: <completed and automated conditions>/<all finite live research plus production and consumption of every required science pack at 1,000,000 per minute>, <percent>, <evidence>
Priority basis: <newest explicit operator direction, current consolidation queue entry, or authority row, plus why it is next>
Authority score: <number and name>, <current> -> <target>
Verified progress: <real behavior or authority improvement, with Code-verified or Live-verified status>
Milestones reached: <actual gameplay progress since start, or none>
Checks: <focused tests, broader tests, Lua, and build status>
Tested build: <commit, installed binary version, and relevant dirty-state status>
Active attempt: <attempt id, speed, elapsed time, latest milestone or partial playbook progress>
Efficiency: <fuel shortage percent and foreground idle percent when measured>
Latest blocker: <one current blocker or none>
Next action: <the action currently being executed next>
Time since last gameplay movement: <duration and evidence>
Gameplay delta: <completed capability count before -> after>
Progress circuit: <OPEN or CLOSED, with reason>
```

`unknown` when evidence is absent, `none` only when absence is verified, and
every verification named Code-verified or Live-verified. Verified progress
means factory behavior, score, bypass count, test gate, build, or acceptance
changed; file counts, command counts, agent time, and commit counts are not
progress. Milestones contain only actual gameplay progress. Permanent goal
progress uses the one typed denominator (every finite enabled non-hidden live
technology, the solar-system-edge checkpoint, production plus consumption of
every required science pack at 1,000,000 per minute), counting a condition
only when a fresh unattended attempt proves it and live state still satisfies
it; player help never counts.

**Stall guard:** compare each report with the prior one; no measurable
engineering or gameplay progress for thirty minutes means the tactic stalled;
reassess whether the batch is still the shortest path.

**Self-learning:** when current evidence proves a reusable lesson, update the
owning project docs and this skill in the same verified batch: project design
and state stay in the repository, reusable procedure in the skill, no dated
project status in the skill. Deploy through the blueprint workflow and verify
the installed copy matches the tracked source (a stale install silently drops
locked sections). Mechanically decidable lessons also update the project hook
and its tests. Learning is part of the batch, not a stopping point. Hooks
enforce only mechanical facts (destructive-action protection, acceptance
admission for the exact tested pushed commit, post-failure re-attempt
evidence, repetition never counting as progress); they fail open for support
work, closed for destructive actions and admission, and never own judgment,
priority, diagnosis, or a development phase.

**End-of-window checkpoint:** finish the current safe boundary, leave the
repository buildable (or state the exact failure and keep fixing while time
remains), update `docs/status.md` and project state, commit and push verified
work, label incomplete work unverified instead of hiding or claiming it, and
publish one final report. Elapsed time never completes an objective.

**Stop only when:** the operator says stop or redirects; a destructive,
irreversible, credential, or external action lacks authority; authoritative
docs conflict materially and the repository does not resolve it; required live
evidence needs a user-only action or unavailable external state; or the
documented acceptance objective is fully proved.

**Completion:** the acceptance objective and efficiency limits pass; the
canonical review confirms dependency order and completion conditions; the
selected authorities have zero known bypasses with rubric-supported scores;
focused, broader, Lua, and full build pass; `docs/authority.md`,
`docs/todo.md`, `docs/design-resolution-plan.md`, project state, and gameplay
evidence agree; everything is pushed. If an earlier-priority authority remains
open, select it and continue. Never claim the project goal complete from one
local fix, test, commit, runner termination, or milestone.

## Docs

Repo `README.md` is the index; **one authoritative doc per subject**, each
with an `**Authoritative for:**` line. Framework work -> `docs/framework.md`.
Brain (RCA, selection, interrupts, lanes) -> `docs/brain.md`. Body (per-tick
control, motion, reach) -> `docs/body.md`. Efficiency (value/cost, tuning,
waste) -> `docs/efficiency.md`. Construction (blueprints, stages, main bus) ->
`docs/construction.md`. Locked product rules -> `docs/design.md`.
Player-facing words -> `docs/terminology.md`. DRY/ownership ->
`docs/authority.md`. Transport/latency/UDP-vs-RCON ->
`docs/transport-latency.md` (locked split; measure before changing pipes).
Live findings -> `docs/todo.md`. Dated shipped history -> the one repo-wide
`docs/changelog.md`; never add changelog sections to design docs. Condensed
origin: `docs/history.md`.

**Documentation contract:** one canonical design statement in one owning doc,
never repeated; other docs refer by file and heading. Each owning doc's
normative content is the table `| Design statement | Committed proof | State |`
with states `RED`, `CODE-VERIFIED`, `LIVE-VERIFIED`, or `PROCESS` (software
cannot decide it; name the committed review check). The proof is the exact
committed test name and must fail when the implementation violates the
statement. Write and commit the failing proof before implementation; update
state only from current evidence; when a statement changes, change its proof
in the same documentation batch; never mark CODE-VERIFIED because code was
written or LIVE-VERIFIED because a unit test passed. Keep only the rationale
the statement needs; history goes to the changelog, unfinished work to the
todo.

## The shape

- One Rust binary, two roles: CLI subcommands now (ping, status), long-running
  watch mode later.
- **One bidirectional UDP mailbox (operator-locked 2026-07-31):** "we dont
  need rcon for anything. we can do it all with udp". Both sides read and
  write the same mailbox, every message carries an id and a status, and
  reading the mailbox is enough to see what was asked and what happened. There
  is no second transport and no fallback in the target design. Honest current
  state: `src/mailbox.rs` plus `src/transport_udp.rs` are the framework, and a
  few unmigrated readers still fall back to an RCON command until their kind
  moves; every such fallback is a migration debt recorded in the todo, never a
  design option. Lua string building in the mod is finished: Rust composes the
  message, the mod is the bridge to the game engine and nothing more.
- **Eyes and hands (operator-locked 2026-07-31):** the brain thinks and
  decides, the body moves in the world. In the mod that means eyes, which only
  see the game, and hands, which only play the game. The mod never does work
  the brain should do: no deciding, no ranking, no bookkeeping. Every eye feeds
  the one `FactoryObservation` the brain reads, so the brain has a single
  picture of the factory rather than per-eye answers.
- The current agent is the judgment layer and drives the CLI through its
  configured Windows shell. No MCP, no Python client.
- Framework: six loops (resource gathering, resource transit, manufacturing,
  power, research, defense). Each loop gets state readers, deterministic
  health checks, and next-step logic. Later game phases deepen loops, never
  add new structure.
- Three parts: the Rust brain offloads as much as possible (deterministic
  monitors, proposals, execution), the LLM only judges what rules cannot, the
  player is final authority and the body. One task at a time: at most one
  active proposal, verified done from game state before the next.

## Locked rules

- **TAS first:** tool-assisted speedrun. Fastest legal unattended wall-clock /
  game-time to each milestone. Personal bests are the scoreboard
  (`docs/personal-bests.md`). When fixing efficiency issues, follow the tuning
  doctrine in `docs/efficiency.md`: eliminate waste, do not add wait; never
  trade body progress for quieter logs unless wall-clock also improves. All
  time is accounted for: every gap longer than one second where the body does
  nothing is a finding with a named cause, and a review that reports
  unallocated time has not finished the job.
- Writes are player-legal actions gated by proposals (approve, reject, auto
  per category in chat). The lazy player principle: the bot does everything a
  UI click could do; the player is an approval and design gate plus the
  physical residue.
- Hard no-cheating line. Post-v1 hands place blueprints exactly as a player
  would.
- **The blueprint library places every building (operator-locked):** "ALL
  building placement should be from blueprints. period." The bot chooses which
  blueprint to use and where, and composes small blueprints into stages; it
  never designs geometry while the game runs. No single-item blueprint, and no
  separate 2-wide and 4-wide copy of one shape: one tileable blueprint with an
  overlapping entity (a shared power pole is the easy case) covers both. Later
  stages add to the earlier stage rather than destroying it.
- **The blueprint files in git are the source of truth (operator-locked
  2026-08-02):** we author blueprints in the repo, tests prove they are valid,
  the bot uses them. Geometry authored at runtime, or a shape that exists only
  as a Rust function returning coordinates, is the failure this rule closes. A
  dynamic blueprint that shows up in the in-game library and not in the repo's
  organized folders is drift: find it and migrate it.
- **Blueprint law tests read the wiki, they do not guess:** every blueprint is
  proved against the documented game rules by committed generic tests, at least
  inserter reach and orientation (plain inserters unless the operator asked for
  long-handed; they are faster), belt corners and side loading, drill output
  edges and belt sides, electric coverage of every machine in the blueprint,
  and tileable overlap. The wiki documents all of it; a geometry argument
  without a wiki citation is a guess and was rejected repeatedly.
- Any modded game must work: game knowledge from prototype data at runtime,
  never hardcoded vanilla lists.
- The bot-built starter base is a one-sided main bus running west to east. Raw
  resources enter from the west, every item produced by the starter base
  returns to an assigned eastbound bus lane, production occupies one side, and
  the opposite side remains reserved for lane expansion. The starter base
  researches and produces the live construction manifest for a separately
  built real base.
- Eight-unit iron and copper smelting is starter-base supply only after every
  individual furnace output is automatically extracted and delivered to its
  assigned eastbound main-bus lane. The generic transit role may use burner
  inserters before power and electric inserters afterward; it chooses from
  live capabilities, rate, construction cost, fuel service, and expected
  replacement time. Handcraft the exact initial belt/inserter/power/assembler
  shortfall, then replace handcrafting recipe by recipe with small
  bus-connected production blocks; replace those temporary blocks through the
  normal tier-cutover lifecycle when measured demand justifies larger designs.
- Shipped transit slice: scaled direct-output layouts pack into aligned rows
  and reserve a two-tile inserter/belt strip. `connect-items` /
  `ghost.connect-items` discovers real drill-fed furnaces or containers,
  selects enabled burner or fully power-covered electric inserters and the
  slowest enabled belt from live prototypes, ghosts continuous eastbound
  lanes, and completes only after real product flow. Production growth never
  composes transit implicitly; named runs on the focused `connect-items`
  playbook use this role only after resource-stage prerequisites exist.
- The scheduler orchestrates every transition from live state. Playbooks
  declare requirements, capabilities, rates, costs, replacement conditions,
  and completion; after each focused playbook the brain chooses the next
  largest time-adjusted bottleneck. Steam power is an early high-value action
  because it unlocks electric logistics/manufacturing and removes recurring
  burner fuel service, but it is not a hardcoded numbered step.
- The starter base has a distinct pre-main-bus mini-factory stage after
  bootstrap mining/smelting and live Automation plus electric power. Build
  small powered production cells immediately rather than handcrafting
  recurring products: direct-insert intermediates when useful, otherwise
  hand-feed bounded inputs collected from factory output and collect
  machine/chest output. One generic manufacturing role parameterizes recipe,
  product, target rate, batch, machine count, input mode, and output mode;
  gears, circuits, belts, inserters, science, ammunition, and building items
  are named parameter sets, never copied workflows. Once a compatible powered
  assembler owns a recipe, `craft.ensure` must supply, empty, or wait for it
  rather than handcrafting that product. These replaceable cells cut over to
  bus-fed production later.
- Opening electricity follows live prerequisites, not assumed unlocks: satisfy
  Steam Power's prototype-declared item trigger, then build one
  prototype-derived steam plant, then handcraft/build one powered lab, produce
  automation science packs, research Automation, and only then build an
  assembler mini-factory. The opening plant is one boiler increment; derive
  its generator count from live boiler energy usage divided by generator
  maximum output (vanilla 2.1 is 1.8 MW / 0.9 MW = two), select enabled
  placeable entity types rather than vanilla names, and join rotated live
  fluid-box connections rather than hardcoding tile offsets. Boiler fueling
  stays in the generic refueling interrupt.
- While any RCON fallback survives it is localhost only, with the password via
  FACTORIOBOT_RCON_PASSWORD env or --password, never committed. Migrating the
  last fallback deletes this rule with it.
- No arbitrary-execution command in the shipped CLI surface.

## Factory desired state (approved 2026-07-27)

- The manifest remains authority for intended buildings.
- The factory audit reports missing, nonfunctional, wrong, extra, and
  satisfied buildings.
- Compile every audit result into the existing available actions used by all
  body work. Do not add another scheduler or repair queue.
- Missing buildings use existing build or restore work. Nonfunctional
  buildings use existing fuel, input, power, or configuration work. Wrong and
  extra buildings use explicit adopt or remove work.
- Authored parent requirements are scheduling dependencies. The copper base
  must complete before the copper lab becomes an available action.
- Preserve each building's stable identity, exact arguments, completion
  condition, and blocker through execution.
- Applying work, probing progress, deciding completion, and reviewing logs use
  the same completion condition.
- The coal bootstrap keeps one persistent starting-fuel work item across
  gathering and transfer so exactly one piece of coal is loaded once.
- Existing playbooks execute selected work. They never rediscover the same
  desired state independently.
- Acceptance requires the copper base, then copper lab, all planned copper
  transport drained, one coal bootstrap load, unchanged placement failure
  escalation, foreground idle below 3 percent, and fuel shortage below 15
  percent.

## Repo layout

- src/main.rs clap CLI, src/lib.rs module exports
- src/mailbox.rs the one mailbox (queued messages with id and status),
  src/transport_udp.rs the wire, src/state.rs Deserialize structs, src/error.rs.
  There is no src/rcon.rs and no src/lua.rs: the 6,400-line Lua string builder
  was deleted with the mailbox migration, so resolve every transport path from
  current source.
- tests/live.rs live tests behind #[ignore]
- docs/: see repo `README.md`; one authoritative doc per subject.
- .claude/project_state.md current focus and next steps

## Commands

- There is NO build.ps1 (removed; stale references cost a real session
  2026-07-31). `restart.ps1` is the ONE lifecycle script: it stops the watch,
  runs the luacheck gate (the `lua_check::` tests in the library test binary:
  every Lua chunk Rust composes plus `mod/factoriobot/control.lua`, same
  luacheck binary as jbot: `%USERPROFILE%\Downloads\Programs\luacheck.exe` or
  `$env:LUACHECK`),
  builds release, installs the binary to the user's bin dir and the companion
  mod, then hosts the save and starts the watch. The launch always passes
  `--enable-lua-udp=<port>` (25002 unless `FACTORIOBOT_UDP_GAME_PORT` says
  otherwise); without that switch the mailbox has no wire and the bot sees no
  game. `-BuildOnly` stops after install without launching the game or watch. NEVER hand-copy the exe: a
  running watch locks it and `restart.ps1` owns stopping the watch.
- Tests only: `k3sc cargo-lock check | test`, never bare cargo.
- Live tests, game must be hosted: `k3sc cargo-lock test -- --ignored`
- CLI: `factoriobot ping | status | problems | next | diagnose | runs
  list|compare | watch`. Default address 127.0.0.1:27015. `problems` is the
  one-shot six-loop health check, `next` is the deterministic
  what-should-I-do-next (priority: defense, power, research, manufacturing,
  gathering, transit), `diagnose` analyzes an attempt log for run-health (exit
  1 on warn+), `runs` lists/compares game-tick milestones in
  `docs/runs/attempts.jsonl`, `watch` polls (10s fast, 300s slow), latches
  alerts (fire on start, fire on clear, never repeat), and delivers to stdout
  plus in-game chat.
- Game setup: in Factorio's config.ini [other] section, uncomment
  local-rcon-socket and local-rcon-password, then host via Multiplayer, Host
  New Game. RCON listens only while hosting, including solo.
- Development restart: `restart.ps1` (default save `factoriobot-start.zip`,
  hidden watch). Pass `-Save NAME.zip` for another save, `-Checkpoint` or
  `-Milestone` for autosave resume points, `-Hypothesis "..."` to label a
  deliberate change in the attempt catalog (`FACTORIOBOT_HYPOTHESIS`),
  `-SkipBuild` to restart without rebuilding (the only mode that skips the
  luacheck gate), `-NoWatch` to host without the watch, and runner parameters
  (`-Gate`, `-AttemptBudget`, `-NoProgressMinutes`, `-PracticeSpeed`) for
  attempt batches.
- For a Steam install, `restart.ps1` must use
  `Steam.exe -applaunch 427520 --host <save>`; direct Steam-build
  `factorio.exe --host` triggers Steam's interactive custom-arguments
  confirmation. Non-Steam builds launch directly.

## Troubleshooting

- Logs are one per save attempt: `factoriobot-<local-time stamp>.log` in the
  repo checkout (cwd fallback). Every process start opens a fresh stamped
  file, and the watch rotates to a new one when the game tick goes backward,
  the new-save-attempt signal. To troubleshoot the current attempt, read the
  newest `factoriobot-*.log`; it starts with the build version and the
  previous file's last line names its successor. Never let one shared log grow
  forever; a 1.6 GB single log cost a real session.
- There is NO log override env var (removed 2026-07-31: a leftover empty
  `FACTORIOBOT_LOG` value silently redirected the watch log to an unopenable
  empty path and two attempts ran with no log). The stamped repo log is the
  one location, with the temp dir as a loud fallback recorded in the attempt
  row. `RUST_LOG` sets the level. Default is debug, so every exchange lands
  in the log.
- Diagnosis order: **`factoriobot diagnose [log]` first** (peak findings:
  abort storm, poll spam, body idle, action-id churn, background thrash,
  interrupt cost; exit 1 on warn+). Then the panel **Health:** line /
  `Copy status`, then grepping the newest attempt log (`op_attempt`,
  `player_action`, `bottleneck_decision`, `error`), then
  `factoriobot status | problems | next`. Live watch already prints latched
  run-health digests to stdout and the panel; chat only on severity >= warn.
- The scheduler hides failures behind holds: an apparent stall is usually a
  held desired-state run or an interrupt waiting on its condition. The attempt
  log names the exact waiting task and its instruction.
- On an unmigrated RCON fallback reader: the client has a 5-second operation
  timeout. A Lua reader that runs
  longer abandons its in-flight response and leaves the shared connection
  reading every previous command's reply (live-observed as a permanent
  one-packet "Response ID mismatch" skew). The framework rebuilds the
  connection on any execute error, so one clean retry recovers; a reader that
  regularly exceeds 5 seconds is itself the defect and must be split or
  bounded.

## Companion mod

- Lives at mod/factoriobot (info.json + control.lua), installed by copying
  that folder into the game's mods directory. factorio_version must match the
  player's game (currently "2.1", they run experimental).
- Owns the in-game approval panel, capped inbox/event/decision buffers, and
  approved per-tick player controls. The generic gather controller uses
  Factorio pathfinding plus normal walking and timed mining input; never use
  teleportation or instant `mine_entity`. One shared multi-target resolver
  computes raw demand from live recipes and selects rocks, trees, or resources
  by useful co-products plus walking and mining time. The panel's Emergency
  stop must remain enabled during every player action.
- Item movement uses the shared `inventory.transfer` operation plus
  `transfer-items` YAML role. It pathfinds into reach, uses player-equivalent
  cursor transfer/split actions, and verifies equal source/destination deltas.
  Fuel, recipe input, and output collection are parameters of this one path,
  never separate mechanics.
- Factory output is authoritative. Acquisition consumes player inventory,
  queued crafts, machine output, and production already in progress before
  manual gathering. Keep machines fueled, supplied, emptied, and unblocked;
  hand mining/chopping is only the smallest proven bootstrap, recovery, or
  expansion shortfall. Recurring work that a running machine can perform must
  never be assigned back to the player.
- The minimum self-sustaining coal mine is two touching burner mining drills
  facing each other. Insert exactly one piece of coal after both real drills
  close the loop; each output fuels the other, and completion requires both
  fueled and producing. Use its coal to hand-feed the rest of the burner
  factory until belts/inserters automate delivery. Four-drill clockwise
  squares are parameterized expansion layouts, not bootstrap.
- `connect-items` is the ONE item connection across direct insertion,
  belts/inserters, bots, trains, rockets, and cargo pods. Output and
  destination (bus lane, chest, machine input, fuel inventory) plus rate are
  parameters; mining, smelting, and manufacturing never receive
  product-specific logistics roles. Shipped paths on `ghost.connect-items`:
  `destination=main-bus` (drill-fed output -> lane) and `destination=fuel`
  (surplus fuel-mine spine -> burner fuel). Words: `docs/terminology.md`.
- A role call inside a loop uses `args_from: item` to fill every declared role
  param from the loop item's same-named fields; explicit args override. Never
  copy one-to-one `${item.<param>}` mapping blocks.
- DRY parameter doctrine: roles encode reusable mechanics and playbooks encode
  focused reusable workflows. Resource, item, recipe, technology, entity,
  count, endpoint, rate, and threshold are runtime parameters unless they
  change the Factorio workflow itself. Never copy a playbook merely to change
  iron to copper, one recipe to another, or one count to another.
- Playbook `params` are the built-in AWX-survey equivalent: every parameter
  has help and a typed default, with optional min/max/choices. `runs` are
  named parameter sets used for scheduler order, panel identity, checkpoints,
  and completion tracking; they do not own copied task lists. Manual overrides
  use `factoriobot run PLAYBOOK --param NAME=VALUE`.
- Playbook selection is live analysis, not a fixed route. Each playbook
  declares `requires_research` and the six-loop bottlenecks it `addresses`;
  named runs declare concrete item production targets in items/minute. Only
  after the focused playbook completes, read Factorio's one-minute production
  and consumption flows plus power satisfaction, identify the available
  actions, score the largest deficit, and choose exactly one next run. `order`
  is only a deterministic tie-breaker. Never switch ordinary growth work
  mid-playbook.
- Continuous factory growth toward completed Space Age research and sustained
  production and consumption of 1,000,000 science packs per minute is the
  standing objective. Select the exact limiting science pack or unfinished
  research condition, trace backward to its first actionable dependency,
  remove it with the smallest complete playbook, observe again, and repeat.
  Unrelated stockpiles and surplus production have zero goal value. Healthy
  loops trigger scale-up analysis; they do not justify idling.
- Urgent condition-driven work uses Factorio train-style interrupt semantics.
  The active growth playbook is the main schedule; every completed task and
  every unsuccessful retry is a pause-aware safe boundary that makes the bot
  re-read urgent conditions immediately. Active defense is the only global
  interrupt. Fuel, input, power, and missing-building Work stay in the one
  production priority. Physical factory growth always outranks unrelated
  upkeep, while exact upkeep required by selected growth inherits that
  growth's priority through the existing dependency graph. This is one
  framework mechanism, never service steps copied into growth playbooks.
  Require hysteresis, a bounded stack,
  explicit `allow_inside_interrupt`, and reactive self-retrigger prevention: a
  completed interrupt re-admits when the shared observation changes or its
  condition latches anew, never on a clock (operator-locked 2026-07-30,
  reactive not time based).
- Fuel and power health remain visible while the factory grows. Live one-minute
  coal production and consumption drive self-sustaining mine scale-out with
  25% reserve headroom; demand-driven fuel-mine scale-out is physical factory
  growth, while refueling existing unrelated consumers is upkeep. Before
  relying on burner output, use the shared
  `fuel.plan` plus `fuel-for-targets` path: recursive target demand ->
  remaining ore/crafts -> live mining/crafting seconds -> 60 ticks/second
  times machine energy usage -> divide by live fuel value -> subtract
  burning/stored energy -> gather/load/verify only the next work interval's
  shortfall. Never use a fixed coal guess or treat one nonempty fuel slot as
  enough.
- Opening defense is a capability gate, not cleanup after scale. Hostile units
  are checked every second around the player and every remembered active
  building; the priority-1000 defense interrupt must run while growth is
  active, another allowed interrupt is active, or the scheduler is idle. It
  uses a live equipped gun/ammo slot, closes only into range, shoots
  continuously, strafes, retreats inside the kite threshold, protects the
  threatened asset coordinates, verifies clear for three seconds, and yields
  to Emergency stop immediately. Before loaded turrets, permit only the
  minimum defense capability path: bootstrap iron, two-drill coal, 4x iron, 1x
  copper, Steam Power, powered lab, prototype-discovered ammo-turret research,
  and individually loaded opening turrets. Larger coal/copper/stone growth and
  bus work declare `requires_defense`. Poll nests touching pollution every 30
  seconds. Resolve the ammo-turret recipe and its prerequisite-first
  technology chain from live prototypes; never encode a vanilla technology
  name.
- Schedule useful player work while machines run. Gather the next proven
  shortfall, build a ready ghost, collect another output, or move toward the
  next site; wait only when the dependency graph proves no independent task is
  ready. Automated coal delivery removes hand-loading, not fuel monitoring;
  electric power gets equivalent capacity/fuel/satisfaction checks.
- A built mining drill owns that raw resource. `craft.ensure` must not
  hand-mine a temporary shortage for any resource with a built compatible
  drill; refuel, unblock, collect, or wait for the factory instead. Hand
  mining is only bootstrap/recovery when no operable factory path exists.
- Starter scale-out shapes are parameterized, but runtime identity is always
  per building. Reserve complete research-tier capacity invisibly, then
  materialize exactly the current staged unit before construction; reconciling
  an earlier stage must never expose or prune future reserved units.
  `build-layout-ghosts` is the ONE construction lifecycle: planners tag ghosts
  with a layout id, `ghost.targets` returns **at most one** remaining ghost
  (`limit: 1`), and the role acquires/builds/verifies that building before
  re-reading remaining until complete. Never snapshot a full ghost list for
  the whole loop. Stale/vanished targets abort the playbook (live 21:01).
  Never use aggregate resource counts as construction completion because
  independent layouts may mine the same resource.
- Opening electricity through Automation is **live-verified 2026-07-21:**
  Steam Power trigger -> steam plant -> powered lab on copper
  `opening-science-island` -> `research.feed` loads packs -> Automation
  researched. Then named mini-factory cells (science/gear) start on the same
  layout. Defense/turrets and factory-made science still need live-confirm.
- Smelted resources use `units` (one copper drill feeding one furnace
  initially, then 4, 8, or later measured stages through the same playbook).
  The first starter growth boundary is 4x iron, 4x copper, and 4x stone before
  main-bus transit becomes eligible. Resources needed raw and smelted use
  `raw_drills`, `raw_output_container`, and `smelting_units`: stone has one
  drill feeding an iron chest for raw stone plus a separate drill feeding a
  furnace for stone bricks. Every output container is independently built and
  its inventory participates in the shared factory-first collection path by
  stable `unit_number`. Never fork per-resource or per-count playbooks.
- Burner-tier iron and copper scale through named parameter sets on the same
  `produce-smelted-resource` playbook. Immediately after the two-drill coal
  bootstrap, copper bootstrap and 4x iron are both eligible; iron's larger
  strategic production deficit must select 4x iron first. Stone needed for
  furnaces is an exact acquisition shortfall, not a prerequisite production
  playbook. Later stages reuse the same parameters. Both resources remain
  serviced by the shared hand-refueling interrupt until automated coal
  delivery is built as its own mechanics-focused playbook.
- When an interrupt completes, immediately repaint the paused playbook's
  authoritative panel-task snapshot before resuming execution; the panel must
  never remain visually stuck on a completed interrupt.
- Layouts are scoped to research tiers. Reuse roles and parameterize
  size/resource within a tier, but do not force one blueprint to span burner,
  electric, module/beacon, bot, or later eras. New research usually triggers a
  purpose-built replacement with explicit prerequisites and rates.
- Tier cutover defaults to build new in the best location, verify sustained
  flow, redirect consumers, then deconstruct or abandon the old factory based
  on recovery/transit cost. Space is cheap; preserving an obsolete layout is
  not a goal.
- Fuel acquisition and refueling are separate upkeep playbooks in the one
  production priority. `maintain-player-fuel-reserve` triggers below 10 carried
  prototype-derived fuel. It collects safe-to-take mining-drill output toward
  100, but a real bootstrap hand-gather fallback stops at 10: one complete
  refueling delivery, never 100 hand-mined fuel. `refuel-starved-burners`
  requires a complete 10-item delivery on hand and only transfers fuel. Neither
  fuel playbook is globally mandatory. Exact fuel required by selected growth
  inherits growth priority through the dependency graph. When carried fuel
  cannot cover every starved burner, the shared producer sorts mining drills
  whose live `mining_target` produces that fuel first, then every other
  individual machine by stable `unit_number`; allocation happens only after
  this sort.
  `inventory.transfer` counts stored plus currently-burning fuel and completes
  immediately after an exact load; the producer exposes only machines fully
  coverable by current player fuel. Growth playbooks never copy acquisition or
  routine refueling. `fuel-for-targets` is reserved for duration-aware
  coverage of a declared production interval, and a self-sustaining coal mine
  owns its single starting piece of coal.
- `restock-player-inventory` is the ONE general factory-output material
  acquisition interrupt: trigger below 10, snapshot currently available
  outputs once, collect each available stack once toward 100, complete
  immediately after that one round, priority 50. Recurrence is reactive like
  every interrupt; never loop inside the interrupt waiting for the high
  threshold. Fuel stays in its dedicated lifecycle because preserving the coal
  mine's last piece of coal and bootstrap fallback are fuel-specific.
- Runtime `LuaEntityPrototype` does not expose the prototype-stage
  mining-drill output vector. Direct-output layouts must ghost/build the drill
  first, then read the real `LuaEntity.drop_position`, ghost the output, and
  build it. A one-tile chest occupies `floor(drop_position) + 0.5`; the
  pre-build planner reserves the full output edge and never guesses which tile
  receives output.
- In-game `/factoriobot <message>` stores to a capped inbox and acks in
  orange; entity deaths on the player force and finished researches store to a
  capped event buffer.
- RCON-only drains: `/factoriobot_poll_inbox` and `/factoriobot_poll_events`
  return JSON arrays and clear. The daemon polls them each fast tick, degrades
  gracefully when the mod is absent (warns once, latched conditions keep
  working).
- Event alerts are one-shot, not latched: deaths group into one "N structures
  lost near (x, y)" per poll; research completions announce by name.

## Lua reader rules

- **ALWAYS luacheck before shipping.** Same doctrine as jbot lint-before-swap:
  after edits to `mod/factoriobot/control.lua` or to Lua that Rust composes,
  the `lua_check` gate must pass; `restart.ps1` runs it unless `-SkipBuild`.
  Never restart a binary that skipped it.
  Fragile: stray `end`, blank lines after `\` string continuations, and
  missing `{BUILDING_RECORDS}` injects have each broken live RCON.
- Read the current official Factorio runtime/prototype docs at
  `lua-api.factorio.com/latest` before changing any game API call, event,
  controller, inventory, pathfinding, or prototype behavior. Confirm the exact
  class member, read/write status, parameters, event timing, and Factorio
  version. Do not guess from memory. Use the official wiki for game
  terminology and command-line/console behavior; use GitHub/community
  implementations only as secondary examples after the official contract is
  known.
- Factorio 2.1 prototype filter methods require an array of typed filter
  tables. Use `get_entity_filtered{{filter="type",type="mining-drill"}}`.
  Never pass the typed filter table directly.
- IIFE form `(function() ... end)()` returning plain Lua tables only, no
  userdata.
- Player-dependent readers start with the connected-player check and return
  {error="no_player"} without one.
- Factorio 2.x dot syntax. helpers.table_to_json is the 2.x name.
- Cap entity result sizes. The lua runs inside the player's game session; its
  stutter is our fault.
- Surface-aware from day one (Space Age: nauvis, platforms, planets).

## Prior art

- Local clones of every relevant project live in a factorio-refs directory
  next to the repo checkout. docs/research.md is the annotated catalog: what
  is liftable versus ideas-only, with licenses.
- Lifted code: factorio-sensei's rcon wrapper and lua readers (MIT, attributed
  in THIRDPARTY.md). FLE's action vocabulary is the reference when hands
  arrive.
- Timberbot ([abix-/TimberbornMods](https://github.com/abix-/TimberbornMods))
  is the architectural precedent: mod does mechanics, external brain does
  judgment, errors written for an AI caller, live test harness.

## Doctrine

- Every command sent to the game gets an expected settle signal. Silent
  failure is the number one killer.
- `build.from-inventory` is the canonical postcondition authority for every
  bot-built entity. Whether it placed the entity or found it already built, it
  must bind the exact declared building record by live `unit_number` and
  verify companion registration before returning success; event tracking is
  redundant observation only. Burner commissioning uses the same measured
  next-work-interval fuel calculation as every other consumer through
  `acquire-fuel` (never the craft path; raw fuel has no recipe). Each closed
  coal loop receives exactly ONE coal total (operator-locked): the first drill
  of a pair is fueled at placement, and a starting-fuel loading count of one
  makes the partner's starting-fuel loading skip when a touching same-name
  drill already holds or burns fuel. The loop-no-coal check is a backstop
  only; never make it the critical path, because drop-target graph edges are
  unreliable on never-powered drills.
- Every role declares `desired_state`; catalog loading fully expands nested
  roles and loops, then rejects any terminal path without an `until_`
  verification. Framework invariant, not a convention.
- Focused playbooks may declare `handles_interrupts` (names validated against
  real interrupt playbooks). Only those duplicate recovery interrupts stand
  down while the focused workflow owns their desired state; defense and every
  unrelated emergency remain eligible.
- The scheduler emits one authoritative bottleneck decision record (measured
  constraint, live values, selected playbook) that drives execution, the
  panel's Bottleneck/Decision display, and telemetry. Never author separate UI
  wording for a decision.
- Production-run completion is measured, operator-locked 2026-07-20: work the
  bottleneck until the target is hit, then analyze for the next bottleneck,
  repeat forever. A finished build whose live rate is still short keeps its
  bottleneck open and says so plainly; milestone runs without rate targets
  complete on built state. The active run's bottleneck record re-measures
  read-only every sweep for the panel; selection still changes only at safe
  boundaries.
- The quickbar mirrors the factory from live data only: page 1 is materials,
  page 2 is placed buildings, first-appearance order, empty slots only, never
  overwrite the player's own filters.
- **One root cause analysis, available on demand (operator-locked):** root
  cause analysis is a huge part of making the factory work, and the bot and the
  player both need it at any moment, not as a separate step that runs once.
  There is ONE implementation, used by the bot when it decides, by
  `factoriobot review`, and by the player whenever they ask. It starts from the
  goal, walks back through the recipes and the six loops to the first link whose
  state does not satisfy the next, and it reads what the factory already holds:
  plates sitting in a furnace are factory materials the bot must use to build
  more factory before it mines anything by hand. A stopped machine is one factor,
  never the answer: collect every candidate cause, rank them by value toward the
  goal, and name the one worth the next few seconds.
- **What a job is worth comes from the game, not from a constant
  (operator-locked):** value is measured in science packs per minute for the
  research that is active right now, because each research consumes different
  packs. Query the live prototype and recipe data, compute the numbers once per
  game configuration, and recompute when the configuration changes (mods and
  game versions move the recipes). Endless research counts when it raises
  growth. Magic magnitudes standing in for value (ordinary work 1, growth 1
  followed by 18 zeros, emergency 1 followed by 20 zeros) are the measured
  failure: they only reorder ordinary work and they hide the real ranking.
  Record cost only when the job actually succeeded and changed something; a run
  that found nothing to do is not a cheap success.
- **No invented bookkeeping (operator-locked):** the factory is observed and
  then acted on. Nothing else may sit between the two. Specifically forbidden,
  each one deleted after it cost a session: a note per machine saying why it is
  stuck and gating retries, a stand-in job in the work list (the
  `__limiting-factorio-condition` placeholder) instead of a real job for real
  work, and a second way to choose the next job. When something needs doing,
  add a real job, rank it with every other job, and run the most valuable one.
- **Reactive, not time based (operator-locked 2026-07-30, no exemptions):**
  elapsed time is measured for cost and never consulted to decide when work
  may proceed, defer, or become eligible again. A held run is re-evaluated
  when the shared observation changes, in process where the observation
  decides its completion condition; a module the observation cannot decide is
  re-dispatched only because the observation changed. A producer loop defers
  on the first unchanged pass; there are no retry budgets, no probe intervals,
  no cooldown clocks, and no sleeps in the control path; every wait resumes on
  the signal it waits for (executor task boundary, action completion bus,
  observation generation, pause channel) with pacing only for observation
  acquisition. The attempt runner's no-progress bound counts observed game
  ticks. If a completion condition cannot be decided from
  `FactoryObservation`, extend the observation, never keep a side-channel
  probe.
- Every executor retry is a safe interrupt boundary: unsuccessful attempts
  signal the watch and wait pause-aware. A finished interrupt discards its own
  scoped pending interrupt before the scheduler resumes growth.
- Never call `begin_crafting` when `get_craftable_count` is zero. Gather raw
  shortages through the shared selector or wait for machine-made
  intermediates; only a positive craftable count permits the write.
- Every successful playbook must request
  `_autosave-factoriobot-<playbook>.zip` through the companion mod's
  `game.auto_save` hook before the executor marks it complete.
- New saves default `auto approve everything` on. Render all approval
  checkboxes from save-persisted `storage.autos`; operator changes survive
  restarts, while Emergency stop still persistently disables all/player
  actions until explicitly changed.
- Alerts latch: fire once when a condition starts, not on every poll.
- Bound every queue at creation. Stable entity ids (unit_number), never
  session-scoped ones.
- Errors tell the caller what went wrong AND what to do next, with valid
  options listed.

## Factorio 2.1 API drift (live-verified 2026-07-19 to 2026-07-20)

- Player position and reach are controller-dependent. In remote view (map
  open) and other non-character controllers, `LuaPlayer.position` is the
  **camera**, and `resource_reach_distance` / `build_distance` can be
  effectively unlimited. Feeding those into area math or `request_path`
  crashes the whole game (`position is out of range` /
  `Chunk.cpp:597 Trying to make chunk at unreasonable position`, live
  2026-07-20, non-recoverable). Every path must start from the character body
  (`body_position` prefers `player.character.position`), every reach/radius
  must go through the shared sanity clamps (`sane_reach` /
  `sane_build_radius`), and path goals must pass `sane_map_coord`. The
  operator must be able to use the map while the bot works (mod 0.8.13).
- Once a rock/tree mine has started (`simple-entity` gather or in-stride
  `clearing-obstacle`), finish destroying that entity before stopping or
  retargeting. Soft `stop_gather` defers until the rock is gone; Emergency
  stop still cancels immediately (mod 0.8.14).
- `LuaEntity.drop_target` is nil on a mining drill that has never been
  powered; only `drop_position` is authoritative on a fresh drill. Any
  drill-output graph must fall back to the entity occupying `drop_position`.
- `LuaBurner.currently_burning` returns a `LuaItemPrototype`, not a string.
  Compare `currently_burning.name.name` against item names.
- LuaRecipe has no `category` and LuaRecipePrototype renamed `category` to
  `categories` (array of strings). All prior-art projects (FLE included)
  predate this. When a reader errors with "doesn't contain key", check
  lua-api.factorio.com/latest before guessing, and read the newest
  `factoriobot-*.log` attempt log: every rcon exchange is in it.
- Trigger technologies (research_trigger on the prototype) cannot be queued
  with add_research; they complete via in-world actions. Exclude them from
  research proposals, surface their requirements as goals.
- Drive normal movement/mining through `player.character.walking_state`,
  `mining_state`, and `picking_state`, not the player control adapter.
  Multiplayer input reconciliation can reduce writes to `player.walking_state`
  to visible one-tick pulses.
- `request_path` finishes asynchronously. The immediate `start_gather`
  response can say `path-pending` even when the next path event says
  `no-path`. Treat both `try_again_later` and `no-path` as nonterminal: keep
  direct normal walking active every tick and retry pathfinding in the
  background. Never stop the character between path retries.
- Request character paths with the character collision box/mask and
  `path_resolution_modifier = 2`; the default 1x1 grid is too coarse for many
  walkable gaps between trees. Follow returned waypoints; direct walking is
  only the continuous fallback while pathfinding is pending or retrying. Reuse
  an action only when source name, type, and position all match. Apply one
  progress monitor in walking, pending, and retry states; less than one tile
  per 30 ticks is stalled, and after two stalled windows (about one second)
  mine only a reachable tree or rock blocking the route with normal input,
  then repath.
- Framework instrumentation is mandatory. Every executor op attempt emits one
  structured `op_attempt` event; the watch polls the companion action once per
  second and emits changed `player_action` snapshots with state, exact target,
  position, waypoint, stuck count, path request/result, and selected obstacle.
  Add new runtime control through this shared telemetry path instead of
  subsystem-specific debug prints.
- Use `LuaControl::resource_reach_distance` for normal mining input against
  every mineable target, including resources, trees, and rocks.
  `reach_distance` is ordinary interaction reach and may be enlarged
  independently by other mods. Keep this in one shared action-reach helper and
  recheck it from the mining state before applying input.
- Build gather sources from live entity prototypes whose mineable products
  intersect current raw needs. Search circular radii outward
  (`8, 16, 32, 64, 128, 256`) and stop at the first radius covering every
  need. Never combine all resources, trees, and rocks into one capped
  large-area query: thousands of ore tiles can consume the limit and hide the
  nearest useful rock.
- Emergency stop is a persistent per-player latch, not a one-tick input write.
  It cancels the stored action, clears player-action/all autos, rejects new
  `start_gather` calls, and changes the button to `Resume player actions`.
  Only that explicit resume clears the latch.

## Live-verified facts (2026-07-18 -> 2026-07-21)

- ping and status work end to end against a hosted Space Age game.
  helpers.table_to_json and the power reader confirmed live (tests/live.rs,
  run with `-- --ignored`).
- RCON answers with EMPTY responses while the game is still loading a save.
  Treat an empty response shortly after connect as retry-able, not as a code
  bug.
- **2026-07-21 fresh start:** powered lab on copper science island
  researching; **Automation** completed unattended (~10:18 wall / 11:26 game).
  Steam plant wall PB **6:44**. Do not regress pole-repair prune,
  `build-layout-ghosts` `limit:1`, or `research.feed` specialization.

## Open items

- RESOLVED: lua empty tables ({} vs []) handled by the lua_array deserializer
  on every list field.
- Whether RCON silent-commands disable achievements for the save: pending the
  player checking in-game, record in docs/design.md.
