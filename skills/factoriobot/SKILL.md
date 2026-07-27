---
name: "factoriobot"
description: "Use when developing or operating factoriobot, continuing interrupted factoriobot work, invoking the skill by itself, saying go or continue, driving its Windows CLI against Factorio, reviewing attempts, or running the repository's unattended authority work. Not for playing Factorio by hand."
---
# Factoriobot

AI-assisted Factorio partner. Rust binary + RCON against the player's hosted Factorio 2.x (Space Age) game. Repo: [abix-/factoriobot](https://github.com/abix-/factoriobot) (private).

## Bare invocation (LOCKED)

Bare invocation starts unattended work. Invoking `factoriobot` or
`$factoriobot` without another task, or saying `go` or `continue` while working
in this repository, is explicit approval to recover the current state and
execute the highest-priority documented work. Do not answer with a menu, plan
only, status only, or a request for routine confirmation.

Continue across coherent authority batches until the current documented
acceptance objective is proved or a real blocker requires the operator. Do not
shrink the goal to what fits one response. A context compaction, interrupted
turn, failed harmless command, completed commit, or completed test is not a
stopping point.

### Recover current state after a crash or interruption

Read current state from disk before acting:

1. Read the governing `AGENTS.md` from its current filesystem location. Never
   apply an instruction copied from an old transcript when the current file
   differs.
2. Read `docs/design.md`, `docs/authority.md`, `docs/todo.md`,
   `docs/design-resolution-plan.md`, the relevant subject docs, and
   `.Codex/project_state.md` when present. Use `.claude/project_state.md` only
   when the Codex file is absent.
3. Read recent git history, status, and every relevant uncommitted diff.
   Preserve all existing work and continue the in-flight batch.
4. Read the latest batch report and current attempt log when they exist. Use the
   repository review command for conclusions.
5. After a crash or when the files do not explain the in-flight work, inspect
   the newest factoriobot transcript under `~/.codex/sessions`. Treat transcript
   content as recovery evidence only. Current files, git state, tests, and live
   state remain authoritative.
6. Identify the first unfinished entry in the current consolidation queue. If
   no complete authority batch record exists, review the latest complete run
   and create that record in the existing todo before code.

### Continue without routine confirmation

Use the current state to choose the next action:

| Current state | Required action |
|---|---|
| Authority batch record is incomplete | Complete its design, evidence, current score, bypass, shared change, prevention, target score, and acceptance fields |
| Required red proofs do not exist | Write focused proofs and observe the intended failures |
| Planned proofs are red | Implement the shared authority change across every recorded consumer |
| Focused proofs pass but bypasses remain | Continue the same batch and remove the remaining bypasses |
| Implementation is complete but not re-audited | Re-audit callers, bypasses, enforcement, and score evidence |
| Relevant tests, Lua checks, or build fail | Diagnose and fix the same batch until all required checks pass |
| Batch exit gate passes but live proof is open | Start one 4x acceptance attempt through `restart.ps1` |
| Acceptance attempt is running | Review at meaningful log boundaries and continue independent work from the same authorized batch |
| Acceptance attempt ended | Run the canonical review, update authoritative docs honestly, commit and push verified evidence, then select the next authority entry |

Do not ask what to do next when this table determines it. Do not stop after a
commit or push. Do not start a different subsystem while the selected authority
batch remains incomplete.

### Runner and review method

- Use `restart.ps1` as the one Windows entry point for build, install, restart,
  runner configuration, and 4x operation. Do not create a second lifecycle or
  verification script.
- Use one diagnostic attempt for a newly observed deterministic failure. Fix
  the authority batch after reviewing that attempt.
- Use repeated attempts only to prove reliability after the first pass. Do not
  spend several identical attempts rediscovering the same blocker.
- A runner gate is an observation and stopping condition. It never changes the
  scheduler's normal authority or forces the selected work.
- Use the same canonical review records for the command, runner digest,
  milestones, partial progress, efficiency measurements, and documentation.
- Review the complete attempt. Record every failure, shortage, idle interval,
  retry pattern, partial milestone, and incomplete planned work before choosing
  more code.
- Do not rerun Factorio after an isolated patch. Finish the coherent authority
  batch, pass its exit gate, then run one acceptance attempt.

### Progress and checkpoints

Keep the operator informed without requiring babysitting:

- Report the recovered current batch and next action when unattended work
  starts.
- Report a changed diagnosis, a completed verified shared change, the start or
  end of acceptance, and a real blocker.
- During long work, provide concise evidence-based updates at useful
  boundaries. Do not emit filler, poll without new evidence, or wait idly.
- Commit and push each completed verified shared change with a concise lowercase
  message. Use path-limited commits and leave unrelated dirty files untouched.
- Update the existing authoritative docs and project state before leaving a
  meaningful batch. Never create a parallel plan or status document.

### Unattended status report

Publish one compact status report immediately after recovery, after each
verified shared change, when acceptance starts or ends, when a milestone is
reached, when the blocker changes, and at useful boundaries during otherwise
long work. During multi-hour work, do not leave the visible status unchanged
for more than ten minutes when new evidence is available.

Use this exact shape:

```text
Unattended status report
Objective: <current documented acceptance objective>
Priority basis: <newest explicit operator direction, current consolidation queue entry, or authority row, plus why it is next>
Authority score: <number and name>, <current> -> <target>
Verified progress: <real behavior or authority improvement, with Code-verified or Live-verified status>
Milestones reached: <gameplay and implementation milestones since start>
Checks: <focused tests, broader tests, Lua, and build status>
Tested build: <commit, installed binary version, and relevant dirty-state status>
Active attempt: <attempt id, speed, elapsed time, latest milestone or partial playbook progress>
Efficiency: <fuel shortage percent and foreground idle percent when measured>
Latest blocker: <one current blocker or none>
Next action: <the action currently being executed next>
```

Use `unknown` when evidence is not available and `none` only when absence is
verified. Never omit a field. Never call work verified without naming whether
it is Code-verified or Live-verified.

Before publishing it in chat, append the same report to
`docs/status-reports.md` under a UTC timestamp heading. This file is the
append-only chronological evidence log for unattended analysis. Preserve every
field and its exact value. Never rewrite or delete an earlier report, and never
backfill an invented timestamp. If the file is dirty during active work, commit
it with the next completed verified batch or the end-of-window checkpoint
instead of creating report-only commit noise.

Keep the single current status section in `docs/status.md` aligned with the same
facts at durable boundaries: recovered start, verified shared change,
acceptance start, acceptance end, blocker, and completion. Update that section
in place. `docs/status.md` is the current summary;
`docs/status-reports.md` is the chronological report log. Do not create a third
status file.

Verified progress means the factory behavior, authority score, bypass count,
test gate, build, or acceptance condition changed. File counts, command counts,
elapsed agent time, token use, and commit counts are not progress. Milestones
reached must include partial playbook progress when it is the best available
evidence, while clearly distinguishing partial from complete.

### Work priority

Use this order:

1. Newest explicit operator direction.
2. Finish the selected in-flight authority batch unless the operator redirects or current evidence proves the batch invalid.
3. Execute the first unfinished entry in the `docs/authority.md` current consolidation queue.
4. After the current consolidation queue is complete, choose the lowest numbered open authority row.
5. If the selected authority row needs a prerequisite, execute the prerequisite as part of the same authority batch, then resume the selected authority row.
6. Within the selected authority, choose the shared change with the greatest effect on the current acceptance objective and long-term unattended play, using measured run cost and impact.

Score measures maturity and never determines priority by itself.

New findings enter the todo under their owning authority and wait unless they invalidate current evidence, threaten data, build, or game safety, or the operator redirects.

Never choose work by the newest symptom, lowest score alone, easiest fix, shortest test, largest visible failure count, or most recent log line.

If the current consolidation queue and current evidence conflict, update the authority doc and todo first, then report the priority change.

### Progress stall guard

At every status report, compare the current authority score, known bypass count,
red proofs, passing checks, milestones, and measured efficiency with the prior
report. No measurable progress for thirty minutes of active work means the
current tactic has stalled.

When stalled:

1. Stop repeating the same command, patch, diagnostic attempt, or explanation.
2. Re-read the selected authority record and the newest contradictory evidence.
3. Identify whether the gap is diagnosis, shared implementation, enforcement,
   verification, or live acceptance.
4. Choose a different action inside the same authority batch that can move a
   recorded measure.
5. Report the stalled measure, changed tactic, and expected proof.

Do not switch subsystems to manufacture visible activity. Do not count more
files, commands, commits, tests written, or elapsed time as movement. A second
unchanged placement failure must escalate through the documented blocker
lifecycle instead of receiving another identical attempt.

### Attempt provenance

Every live attempt must record the exact commit and installed binary version it
tests. Record whether relevant source files were dirty at build time. An
attempt can prove only that installed build.

Source changes made after an attempt starts may continue as independent work,
but that attempt is not evidence for them. Do not describe later source edits
as live-verified until a later build and attempt exercise them. If relevant
dirty source was included in the installed binary, identify it explicitly and
do not attribute the result to `HEAD` alone.

### End-of-window checkpoint

When the operator provides an unattended time window, keep working until the
window ends, acceptance completes, or a real blocker occurs. Before the window
ends:

- Finish the current safe edit, test, review, or command boundary.
- Leave the repository buildable. If it is not buildable, state the exact
  compiler or test failure and continue fixing it while time remains.
- Update the current section in `docs/status.md` and the project state with
  authority score, bypass count, verified milestones, checks, active attempt,
  blocker, and exact next action.
- Commit and push completed verified work. Preserve incomplete work and label
  it unverified instead of hiding, reverting, or claiming it.
- Publish one final unattended status report for the window.

Elapsed time never makes the project objective complete. Leave the documented
goal active when acceptance remains open.

### Stop conditions

Stop unattended work only when:

- The operator explicitly says stop or redirects the task.
- A destructive, irreversible, credential, or external action needs authority
  that has not been granted.
- Current authoritative docs conflict on a material design choice and the
  repository does not resolve it.
- Required live evidence needs a user-only game action or unavailable external
  state.
- The current documented acceptance objective is fully proved.

Compilation errors, failing tests, Lua errors, shell syntax mistakes, a missing
optional command, unrelated dirty files, an active long run, and harmless
command failures are not stop conditions. Diagnose, use the available
equivalent, fix the same batch, and continue.

### Completion

Bare invocation is complete only when:

- The current documented acceptance objective and its efficiency limits pass.
- The canonical review confirms the required dependency order and completion
  conditions.
- The selected authority systems have zero known bypasses and their scores are
  supported by the recorded rubric evidence.
- Relevant focused and broader tests, Lua checks, and the full build pass.
- `docs/authority.md`, `docs/todo.md`, `docs/design-resolution-plan.md`,
  project state, and gameplay evidence report the same status.
- Every completed verified change and evidence update is committed and pushed.

If an earlier-priority authority entry remains open after one batch passes,
select it and continue. Never claim the project goal complete because one local
fix, test, commit, runner termination, or milestone passed.

**Docs:** repo `README.md` is the index. **one authoritative doc per subject**, each with an `**Authoritative for:**` line. Before framework work (modules, roles, playbooks, runs) -> `docs/framework.md`. Before brain work (RCA, selection, interrupts, lanes) -> `docs/brain.md`. Before body work (per-tick control, motion, reach) -> `docs/body.md`. Before efficiency work (value/cost ranking, tuning, waste) -> `docs/efficiency.md`. Before construction work (blueprints, stages, main bus) -> `docs/construction.md`. Before locked product rules / six loops / strategy -> `docs/design.md`. Before any player-facing words -> `docs/terminology.md`. Before DRY/ownership work -> `docs/authority.md`. Before brain-to-game transport / latency / **UDP vs RCON** -> `docs/transport-latency.md` (locked split + measure before changing pipes). Before fixing a live finding -> **`docs/todo.md`** (open issues: severity + recommended fix). Do not invent a parallel doc.

The repository has one repo-wide `docs/changelog.md` for dated shipped history.
Never add changelog sections to individual design docs. Move durable shipped
history into the repo-wide changelog, keep current design statements and their
proof state in the owning design doc, and keep unfinished work in `docs/todo.md`.

### Documentation contract

Write one canonical design statement in one owning design doc.
Never repeat a design statement in another doc.
Refer to the owning statement by file and heading when another subject depends on it.

Each owning design doc uses this table as its normative content:

```text
| Design statement | Committed proof | State |
|---|---|---|
| <one exact requirement> | `<exact committed test name>` | <proof state> |
```

Every design statement names its committed proof.
Every design statement records its current proof state.
Use only `RED`, `CODE-VERIFIED`, `LIVE-VERIFIED`, or `PROCESS`.

- `RED`: the committed proof currently fails or the required implementation is
  incomplete.
- `CODE-VERIFIED`: the exact committed test passes against the implementation.
- `LIVE-VERIFIED`: the committed test passes and recorded gameplay evidence
  proves the real behavior.
- `PROCESS`: software cannot decide the statement. Name the exact committed
  review check and explain why automation cannot decide it.

Use the exact committed test name. A documentation or citation check does not
prove factory behavior. The proof must fail when the implementation violates
the design statement. Prefer measured gameplay evidence, then the real catalog,
roles, playbooks, scheduler, generated Lua, and implementation behavior. Source
text proves only a forbidden implementation is absent.

Keep only the rationale needed to understand the owning design statement.
Remove duplicated design prose, dated implementation history, completed task lists, and status narration from design docs.
Put dated shipped history only in the one repo-wide `docs/changelog.md`.
Unfinished implementation and failed proof belong in `docs/todo.md`.
Current capability and authority summaries stay in their existing owning status and authority docs without restating the full design.

Write and commit the failing proof before implementation.
Update proof state only from current test or gameplay evidence.
When a design statement changes, change its committed proof in the same documentation batch before changing the implementation.
Never mark a statement `CODE-VERIFIED` because code was written, or `LIVE-VERIFIED` because a unit test passed.

## Closing open issues (LOCKED)

Open issues live in `docs/todo.md` keyed by **stable short titles** (no serial numbers).

**Code-landed != closed.** Shipping a fix only gets a row to "code-landed". Closing requires **gameplay-log proof**.

Before deleting any Open row:

1. **Grep the attempt log(s)** (`factoriobot-YYYYMMDD-HHMMSS.log`) for the failure string / playbook outcome that defined the issue.
2. **Write the proof into `docs/changelog.md` first**. Attempt id, playbook name, executor outcome (`DONE` / `DEFERRED` / absence of the abort string), and **log line number(s)** (e.g. `130656 L157958`). "Looks better" / "should be fixed" / "superseded in code" is **not** proof.
3. **Only then** delete the row from `docs/todo.md`.
4. If the bug still appears in a later attempt log, **keep the row open** (or reopen it). Counter-evidence wins.
5. RCON Lua payload text that merely *contains* an old error string is not evidence; count only executor/INFO/`recv` error lines.

## Development cycle (LOCKED)

Develop factoriobot through authority consolidation. Never alternate one small
live hotfix with one new acceptance run.

### Turn one run into one authority batch

An acceptance run is evidence collection. Review the complete run with the
repository workflow before editing code. Put every observed failure,
inefficiency, idle interval, shortage, retry, and incomplete planned work into
the existing `docs/todo.md`. Do not choose the first visible failure and stop
reviewing.

Map every finding to the numbered systems in `docs/authority.md`. Findings that
share one weak authority are one implementation problem. Choose the unfinished
authority change with the greatest expected effect on long-term unattended
play, using the priority already recorded in `docs/authority.md`. Do not let
the most recent log line override that priority.

### Required batch record

Before code, add one record to the existing `docs/todo.md` containing all of
these fields:

| Field | Required content |
|---|---|
| Design | Exact governing statement and source file |
| Authority | Numbered system from `docs/authority.md` |
| Run evidence | Attempt id, exact symptom, and log lines |
| Current state | Verified score, canonical path, all known bypasses, and missing enforcement |
| Shared change | One canonical implementation path and every consumer moved to it |
| Prevention | Failing tests or structural guards that make each bypass return loudly |
| Target state | Score the completed batch can honestly reach and the rubric evidence required |
| Acceptance | Exact gameplay order, completion conditions, and efficiency limits to verify |

If any field is unknown, inspect the repository and update the record. Do not
code until the record is complete. Do not create another plan or design
document.

### Batch entry gate

The selected change is eligible only when all of these are true:

- It follows an existing statement in `docs/design.md` and the owning subject
  docs. A change without design authority is scope creep.
- It removes at least one competing authority or adds enforcement that prevents
  a bypass. A local behavior patch that leaves the authority score unchanged is
  not eligible.
- It covers every known consumer of the selected authority in one coherent
  change. Do not repair one consumer while known competing consumers remain.
- Focused tests have failed for the missing design rule, each known bypass, and
  the enforcement boundary.

### Execute the whole batch

Implement the shared change across all recorded consumers. Use fast red and
green unit, catalog, Lua, design, and build checks while working. Routine test
failures remain inside this batch until fixed. Do not start Factorio to check
each edit.

Re-audit the selected authority after implementation. Record the remaining
bypasses and enforcement evidence. Increase the score only when the
`docs/authority.md` rubric is satisfied. Code volume, commit count, and a
passing example do not move a score.

### Batch exit gate

The authority batch is complete only when all of these are true:

- Every known consumer uses the canonical path.
- The bypass count and enforcement evidence support the new score.
- Focused tests, relevant broader tests, Lua checks, and the full repository
  build pass.
- `docs/authority.md`, `docs/todo.md`, the existing resolution plan, and
  project state report the same honest status.
- The verified shared change is committed and pushed.

Only then start one acceptance run for the whole batch. That run must verify
the dependency order, completion conditions, and efficiency limits recorded in
the batch. Review the complete run before selecting more code work.

If the run finds another shared failure, add and categorize all of its evidence
first. Return to the highest-priority authority record. Never apply one hotfix
and immediately rerun.

Do not wait idly for a long run. At useful log boundaries, review evidence and
continue independent work already authorized by the same authority batch.

Three parts: the Rust brain offloads as much as possible (deterministic monitors, proposals, execution), the LLM only judges what rules cannot, the player is final authority and the body. One task at a time: at most one active proposal, verified done from game state before the next.

## Factory desired state (approved 2026-07-27)

- The manifest remains authority for intended buildings.
- The factory audit reports missing, nonfunctional, wrong, extra, and satisfied buildings.
- Compile every audit result into the existing work pool used by all body work. Do not add another scheduler or repair queue.
- Missing buildings use existing build or restore work. Nonfunctional buildings use existing fuel, input, power, or configuration work. Wrong and extra buildings use explicit adopt or remove work.
- authored parent requirements are scheduling dependencies. The copper base must complete before the copper lab can bid as realizable.
- Preserve each building's stable identity, exact arguments, completion condition, and blocker through execution.
- Applying work, probing progress, deciding completion, and reviewing logs use the same completion condition.
- The coal bootstrap keeps one persistent starting-fuel work item across gathering and transfer so exactly one piece of coal is loaded once.
- Existing playbooks execute selected work. They never rediscover the same desired state independently.
- Acceptance requires the copper base, then copper lab, all planned copper transport drained, one coal bootstrap load, unchanged placement failure escalation, foreground idle below 3 percent, and fuel shortage below 15 percent.

## The shape

- One Rust binary, two roles: CLI subcommands now (ping, status), long-running watch mode later.
- Every read is one RCON round trip: a Lua IIFE string wrapped as `/silent-command rcon.print(helpers.table_to_json(<iife>))`, JSON back, serde into typed structs in src/state.rs.
- The current agent is the judgment layer and drives the CLI through its configured Windows shell. No MCP, no Python client.
- Framework: six loops (resource gathering, resource transit, manufacturing, power, research, defense). Each loop gets state readers, deterministic health checks, and next-step logic. Later game phases deepen loops, never add new structure.

## Locked rules

- **TAS first:** tool-assisted speedrun. Fastest legal unattended wall-clock / game-time to each milestone. Personal bests are the scoreboard (`docs/personal-bests.md`). When fixing efficiency issues (`docs/todo.md`), follow the tuning doctrine in `docs/efficiency.md`: eliminate waste, do not add wait; never trade body progress for quieter logs unless wall-clock also improves.
- Writes are player-legal actions gated by proposals (approve, reject, auto per category in chat). The lazy player principle: the bot does everything a UI click could do; the player is an approval and design gate plus the physical residue.
- Hard no-cheating line. Post-v1 hands place blueprints exactly as a player would.
- Any modded game must work: game knowledge from prototype data at runtime, never hardcoded vanilla lists.
- The bot-built starter base is a one-sided main bus running west to east. Raw resources enter from the west, every item produced by the starter base returns to an assigned eastbound bus lane, production occupies one side, and the opposite side remains reserved for lane expansion. The starter base researches and produces the live construction manifest for a separately built real base.
- Eight-unit iron and copper smelting is starter-base supply only after every individual furnace output is automatically extracted and delivered to its assigned eastbound main-bus lane. The generic transit role may use burner inserters before power and electric inserters afterward; it chooses from live capabilities, rate, construction cost, fuel service, and expected replacement time. Handcraft the exact initial belt/inserter/power/assembler shortfall, then replace handcrafting recipe by recipe with small bus-connected production blocks; replace those temporary blocks through the normal tier-cutover lifecycle when measured demand justifies larger designs.
- Shipped transit slice: scaled direct-output layouts pack into aligned rows and reserve a two-tile inserter/belt strip. `connect-items` / `ghost.connect-items` discovers real drill-fed furnaces or containers, selects enabled burner or fully power-covered electric inserters and the slowest enabled belt from live prototypes, ghosts continuous eastbound lanes, and completes only after real product flow. Production growth never composes transit implicitly; named runs on the focused `connect-items` playbook use this role only after resource-stage prerequisites exist.
- The scheduler orchestrates every transition from live state. Playbooks declare requirements, capabilities, rates, costs, replacement conditions, and completion; after each focused playbook the brain chooses the next largest time-adjusted bottleneck. Steam power is an early high-value candidate because it unlocks electric logistics/manufacturing and removes recurring burner fuel service, but it is not a hardcoded numbered step.
- The starter base has a distinct pre-main-bus mini-factory stage after bootstrap mining/smelting and live Automation plus electric power. Build small powered production cells immediately rather than handcrafting recurring products: direct-insert intermediates when useful, otherwise hand-feed bounded inputs collected from factory output and collect machine/chest output. One generic manufacturing role parameterizes recipe, product, target rate, batch, machine count, input mode, and output mode; gears, circuits, belts, inserters, science, ammunition, and building items are named parameter sets, never copied workflows. Once a compatible powered assembler owns a recipe, `craft.ensure` must supply, empty, or wait for it rather than handcrafting that product. These replaceable cells cut over to bus-fed production later.
- Opening electricity follows live prerequisites, not assumed unlocks: satisfy Steam Power's prototype-declared item trigger, then build one prototype-derived steam plant, then handcraft/build one powered lab, produce automation science packs, research Automation, and only then build an assembler mini-factory. The opening plant is one boiler increment; derive its generator count from live boiler energy usage divided by generator maximum output (vanilla 2.1 is 1.8 MW / 0.9 MW = two), select enabled placeable entity types rather than vanilla names, and join rotated live fluid-box connections rather than hardcoding tile offsets. Boiler fueling stays in the generic refueling interrupt.
- RCON is localhost only. Password via FACTORIOBOT_RCON_PASSWORD env or --password, never committed.
- No arbitrary-execution command in the shipped CLI surface.

## Repo layout

- src/main.rs clap CLI, src/lib.rs module exports
- src/rcon.rs connect + execute_lua_json (lifted from factorio-sensei, MIT, see THIRDPARTY.md)
- src/lua.rs IIFE reader builders, src/state.rs Deserialize structs, src/error.rs
- tests/live.rs live tests behind #[ignore]
- docs/: see repo `README.md`. One authoritative doc per subject (`**Authoritative for:**` line on each). Condensed origin: `docs/history.md`. Transport latency gate: `docs/transport-latency.md`.
- .claude/project_state.md current focus and next steps

## Commands

- Build and test: `.\build.ps1` (preferred) or `k3sc cargo-lock check | test | build --release`, never bare cargo. `build.ps1` **refuses to ship** unless the luacheck gate passes (`cargo test --lib -- lua_check`): generated RCON IIFEs in `src/lua.rs` plus `mod/factoriobot/control.lua`. Same luacheck binary as jbot (`%USERPROFILE%\Downloads\Programs\luacheck.exe` or `$env:LUACHECK`). After a release build, copy the exe from the shared target dir to the user's bin dir on PATH. A running watch locks the exe; stop it before rebuilding.
- After any edit to `src/lua.rs` or `mod/factoriobot/control.lua`, run the luacheck gate before restart. Do not `restart.ps1` a binary that skipped `build.ps1`.
- Live tests, game must be hosted: `k3sc cargo-lock test -- --ignored`
- CLI: `factoriobot ping | status | problems | next | diagnose | runs list|compare | watch`. Default address 127.0.0.1:27015. `problems` is the one-shot six-loop health check, `next` is the deterministic what-should-I-do-next (priority: defense, power, research, manufacturing, gathering, transit), `diagnose` analyzes an attempt log for run-health (exit 1 on warn+), `runs` lists/compares game-tick milestones in `docs/runs/attempts.jsonl` (evidence trail vs PBs), `watch` polls (10s fast, 300s slow), latches alerts (fire on start, fire on clear, never repeat), and delivers to stdout plus in-game chat.
- Game setup: in Factorio's config.ini [other] section, uncomment local-rcon-socket and local-rcon-password, then host via Multiplayer, Host New Game. RCON listens only while hosting, including solo.
- Development restart: `build.ps1` installs both the release binary and companion mod. `restart.ps1` closes Factorio normally, launches `factoriobot-start.zip` directly with Factorio 2.1's `--host`, and starts a hidden watch if needed. Pass `-Save NAME.zip` for another save. Pass `-Hypothesis "..."` to label a deliberate change in the attempt catalog (`FACTORIOBOT_HYPOTHESIS`). Restart does not re-run luacheck; build owns that gate.
- For a Steam install, `restart.ps1` must use `Steam.exe -applaunch 427520 --host <save>`; direct Steam-build `factorio.exe --host` triggers Steam's interactive custom-arguments confirmation. Non-Steam builds launch directly.

## Troubleshooting

- Logs are one per save attempt: `factoriobot-<local-time stamp>.log` in the repo checkout (cwd fallback). Every process start opens a fresh stamped file, and the watch rotates to a new one when the game tick goes backward, the new-save-attempt signal. To troubleshoot the current attempt, read the newest `factoriobot-*.log`; it starts with the build version and the previous file's last line names its successor. Never let one shared log grow forever; a 1.6 GB single log cost a real session.
- `FACTORIOBOT_LOG` pins one fixed file with no rotation (escape hatch only); `RUST_LOG` sets the level. Default is debug, so every rcon exchange lands in the log.
- Diagnosis order: **`factoriobot diagnose [log]` first** (peak findings: abort storm, poll spam, body idle, action-id churn, background thrash, interrupt cost. Exit 1 on warn+). Then the panel **Health:** line / `Copy status`, then grepping the newest attempt log (`op_attempt`, `player_action`, `bottleneck_decision`, `error`), then `factoriobot status | problems | next`. Live watch already prints latched run-health digests to stdout and the panel; chat only on severity >= warn.
- The scheduler hides failures behind retry/backoff: an apparent stall is usually a retryable desired-state loop or an interrupt waiting on its condition. The attempt log names the exact waiting task and its instruction.
- The RCON client has a 5-second operation timeout. A Lua reader that runs longer abandons its in-flight response and leaves the shared connection reading every previous command's reply (live-observed as a permanent one-packet "Response ID mismatch" skew). The framework rebuilds the connection on any execute error, so one clean retry recovers; a reader that regularly exceeds 5 seconds is itself the defect and must be split or bounded.

## Companion mod

- Lives at mod/factoriobot (info.json + control.lua), installed by copying that folder into the game's mods directory. factorio_version must match the player's game (currently "2.1", they run experimental).
- Owns the in-game approval panel, capped inbox/event/decision buffers, and approved per-tick player controls. The generic gather controller uses Factorio pathfinding plus normal walking and timed mining input; never use teleportation or instant `mine_entity`. One shared multi-target resolver computes raw demand from live recipes and selects rocks, trees, or resources by useful co-products plus walking and mining time. The panel's Emergency stop must remain enabled during every player action.
- Item movement uses the shared `inventory.transfer` operation plus `transfer-items` YAML role. It pathfinds into reach, uses player-equivalent cursor transfer/split actions, and verifies equal source/destination deltas. Fuel, recipe input, and output collection are parameters of this one path, never separate mechanics.
- Factory output is authoritative. Acquisition consumes player inventory, queued crafts, machine output, and production already in progress before manual gathering. Keep machines fueled, supplied, emptied, and unblocked; hand mining/chopping is only the smallest proven bootstrap, recovery, or expansion shortfall. Recurring work that a running machine can perform must never be assigned back to the player.
- The minimum self-sustaining coal mine is two touching burner mining drills facing each other. Insert exactly one piece of coal coal after both real drills close the loop; each output fuels the other, and completion requires both fueled and producing. Use its coal to hand-feed the rest of the burner factory until belts/inserters automate delivery. Four-drill clockwise squares are parameterized expansion layouts, not bootstrap.
- `connect-items` is the ONE item connection across direct insertion, belts/inserters, bots, trains, rockets, and cargo pods. Output and destination (bus lane, chest, machine input, fuel inventory) plus rate are parameters; mining, smelting, and manufacturing never receive product-specific logistics roles. Shipped paths on `ghost.connect-items`: `destination=main-bus` (drill-fed output -> lane) and `destination=fuel` (surplus fuel-mine spine -> burner fuel). Words: `docs/terminology.md`.
- A role call inside a loop uses `args_from: item` to fill every declared role param from the loop item's same-named fields; explicit args override. Never copy one-to-one `${item.<param>}` mapping blocks.
- DRY parameter doctrine: roles encode reusable mechanics and playbooks encode focused reusable workflows. Resource, item, recipe, technology, entity, count, endpoint, rate, and threshold are runtime parameters unless they change the Factorio workflow itself. Never copy a playbook merely to change iron to copper, one recipe to another, or one count to another.
- Playbook `params` are the built-in AWX-survey equivalent: every parameter has help and a typed default, with optional min/max/choices. `runs` are named parameter sets used for scheduler order, panel identity, checkpoints, and completion tracking; they do not own copied task lists. Manual overrides use `factoriobot run PLAYBOOK --param NAME=VALUE`.
- Playbook selection is live analysis, not a fixed route. Each playbook declares `requires_research` and the six-loop bottlenecks it `addresses`; named runs declare concrete item production targets in items/minute. Only after the focused playbook completes, read Factorio's one-minute production and consumption flows plus power satisfaction, filter ineligible candidates, score the largest deficit, and choose exactly one next run. `order` is only a deterministic tie-breaker. Never switch ordinary growth work mid-playbook.
- Continuous factory growth toward Space Age endgame is the standing objective. Find the measured bottleneck, remove it with the smallest complete playbook, observe again, then grow resource throughput, smelting, manufacturing, logistics, research, and power. Healthy loops trigger scale-up analysis; they do not justify idling.
- Urgent condition-driven work uses Factorio train-style interrupt semantics. The active growth playbook is the main schedule; every completed task and every unsuccessful retry is a pause-aware safe boundary that signals the watch to re-read urgent conditions immediately rather than waiting for either the monitor timer or the operation retry delay. At that boundary, checkpoint it, push the highest-priority eligible interrupt playbook, run until its explicit clear condition, re-read live state, then resume. This wakeup is one framework mechanism, never fuel-specific steps copied into growth playbooks. Defense, critical power, burner-fuel starvation, and missing-building recovery may interrupt; ordinary growth deficits never do. Require hysteresis, cooldowns, a bounded stack, explicit `allow_inside_interrupt`, and self-retrigger prevention.
- Fuel and power health preempt expansion and output waits. Live one-minute coal production and consumption drive self-sustaining mine scale-out with 25% reserve headroom; any production-below-consumption state must outrank ordinary iron growth. Before relying on burner output, use the shared `fuel.plan` plus `fuel-for-targets` path: recursive target demand -> remaining ore/crafts -> live mining/crafting seconds -> 60 ticks/second times machine energy usage -> divide by live fuel value -> subtract burning/stored energy -> gather/load/verify only the next work interval's shortfall. Never use a fixed coal guess or treat one nonempty fuel slot as enough.
- Opening defense is a capability gate, not cleanup after scale. Hostile units are checked every second around the player and every remembered active building; the priority-1000 defense interrupt must run while growth is active, another allowed interrupt is active, or the scheduler is idle. It uses a live equipped gun/ammo slot, closes only into range, shoots continuously, strafes, retreats inside the kite threshold, protects the threatened asset coordinates, verifies clear for three seconds, and yields to Emergency stop immediately. Before loaded turrets, permit only the minimum defense capability path: bootstrap iron, two-drill coal, 4x iron, 1x copper, Steam Power, powered lab, prototype-discovered ammo-turret research, and individually loaded opening turrets. Larger coal/copper/stone growth and bus work declare `requires_defense`. Poll nests touching pollution every 30 seconds. Resolve the ammo-turret recipe and its prerequisite-first technology chain from live prototypes; never encode a vanilla technology name.
- Schedule useful player work while machines run. Gather the next proven shortfall, build a ready ghost, collect another output, or move toward the next site; wait only when the dependency graph proves no independent task is ready. Automated coal delivery removes hand-loading, not fuel monitoring; electric power gets equivalent capacity/fuel/satisfaction checks.
- A built mining drill owns that raw resource. `craft.ensure` must not hand-mine a temporary shortage for any resource with a built compatible drill; refuel, unblock, collect, or wait for the factory instead. Hand mining is only bootstrap/recovery when no operable factory path exists.
- Starter scale-out shapes are parameterized, but runtime identity is always per building. Reserve complete research-tier capacity invisibly, then materialize exactly the current staged unit before construction; reconciling an earlier stage must never expose or prune future reserved units. `build-layout-ghosts` is the ONE construction lifecycle: planners tag ghosts with a layout id, `ghost.targets` returns **at most one** remaining ghost (`limit: 1`), and the role acquires/builds/verifies that building before re-reading remaining until complete. Never snapshot a full ghost list for the whole loop. Stale/vanished targets abort the playbook (live 21:01). Never use aggregate resource counts as construction completion because independent layouts may mine the same resource.
- Opening electricity through Automation is **live-verified 2026-07-21:** Steam Power trigger -> steam plant -> powered lab on copper `opening-science-island` (poles via `power_ensure` + prune against the full desired chain) -> `research.feed` loads packs -> Automation researched. Then named mini-factory cells (science/gear) start on the same layout. Defense/turrets and factory-made science still need live-confirm.
- Smelted resources use `units` (one copper drill feeding one furnace initially, then 4, 8, or later measured stages through the same playbook). The first starter growth boundary is 4x iron, 4x copper, and 4x stone before main-bus transit becomes eligible. Resources needed raw and smelted use `raw_drills`, `raw_output_container`, and `smelting_units`: stone has one drill feeding an iron chest for raw stone plus a separate drill feeding a furnace for stone bricks. Every output container is independently built and its inventory participates in the shared factory-first collection path by stable `unit_number`. Never fork per-resource or per-count playbooks.
- Burner-tier iron and copper scale through named parameter sets on the same `produce-smelted-resource` playbook. Immediately after the two-drill coal bootstrap, copper bootstrap and 4x iron are both eligible; iron's larger strategic production deficit must select 4x iron first. Stone needed for furnaces is an exact acquisition shortfall, not a prerequisite production playbook. Later stages reuse the same parameters. Both resources remain serviced by the shared hand-refueling interrupt until automated coal delivery is built as its own mechanics-focused playbook.
- When an interrupt completes, immediately repaint the paused playbook's authoritative panel-task snapshot before resuming execution; the panel must never remain visually stuck on a completed interrupt.
- Layouts are scoped to research tiers. Reuse roles and parameterize size/resource within a tier, but do not force one blueprint to span burner, electric, module/beacon, bot, or later eras. New research usually triggers a purpose-built replacement with explicit prerequisites and rates.
- Tier cutover defaults to build new in the best location, verify sustained flow, redirect consumers, then deconstruct or abandon the old factory based on recovery/transit cost. Space is cheap; preserving an obsolete layout is not a goal.
- Fuel acquisition and refueling are separate interrupts. `maintain-player-fuel-reserve` is priority 90 and triggers below 10 carried prototype-derived fuel. It collects safe-to-take mining-drill output toward 100, but a real bootstrap hand-gather fallback stops at 10: one complete refueling delivery, never 100 hand-mined fuel. It has a 30-second cooldown. `refuel-starved-burners` is priority 100, requires a complete 10-item delivery on hand, may preempt acquisition, and only transfers fuel. When carried fuel cannot cover every starved burner, the shared producer sorts mining drills whose live `mining_target` produces that fuel first, then every other individual machine by stable `unit_number`; allocation happens only after this sort. `inventory.transfer` counts stored plus currently-burning fuel and completes immediately after an exact load; the producer exposes only machines fully coverable by current player fuel. Growth playbooks never copy acquisition or routine refueling. `fuel-for-targets` is reserved for duration-aware coverage of a declared production interval, and a self-sustaining coal mine owns its single starting piece of coal.
- `restock-player-inventory` is the ONE general factory-output material acquisition interrupt: trigger below 10, snapshot currently available outputs once, collect each available stack once toward 100, complete immediately after that one round, priority 50, and a 30-second cooldown. The cooldown owns recurrence; never loop inside the interrupt waiting for the high threshold. Fuel stays in its dedicated lifecycle because preserving the coal mine's last piece of coal and bootstrap fallback are fuel-specific.
- Runtime `LuaEntityPrototype` does not expose the prototype-stage mining-drill output vector. Direct-output layouts must ghost/build the drill first, then read the real `LuaEntity.drop_position`, ghost the output, and build it. A one-tile chest occupies `floor(drop_position) + 0.5`; the pre-build planner reserves the full output edge and never guesses which tile receives output.
- In-game `/factoriobot <message>` stores to a capped inbox and acks in orange; entity deaths on the player force and finished researches store to a capped event buffer.
- RCON-only drains: `/factoriobot_poll_inbox` and `/factoriobot_poll_events` return JSON arrays and clear. The daemon polls them each fast tick, degrades gracefully when the mod is absent (warns once, latched conditions keep working).
- Event alerts are one-shot, not latched: deaths group into one "N structures lost near (x, y)" per poll; research completions announce by name.

## Lua reader rules

- **ALWAYS luacheck before shipping.** Same doctrine as jbot lint-before-swap: after edits to `src/lua.rs` or `mod/factoriobot/control.lua`, `.\build.ps1` must pass the `lua_check` gate (or `k3sc cargo-lock test --lib -- lua_check`). Never restart a binary that skipped it. Fragile: stray `end`, blank lines after `\` string continuations, and missing `{BUILDING_RECORDS}` injects have each broken live RCON.
- Read the current official Factorio runtime/prototype docs at `lua-api.factorio.com/latest` before changing any game API call, event, controller, inventory, pathfinding, or prototype behavior. Confirm the exact class member, read/write status, parameters, event timing, and Factorio version. Do not guess from memory. Use the official wiki for game terminology and command-line/console behavior; use GitHub/community implementations only as secondary examples after the official contract is known.
- IIFE form `(function() ... end)()` returning plain Lua tables only, no userdata.
- Player-dependent readers start with the connected-player check and return {error="no_player"} without one.
- Factorio 2.x dot syntax. helpers.table_to_json is the 2.x name.
- Cap entity result sizes. The lua runs inside the player's game session; its stutter is our fault.
- Surface-aware from day one (Space Age: nauvis, platforms, planets).

## Prior art

- Local clones of every relevant project live in a factorio-refs directory next to the repo checkout. docs/research.md is the annotated catalog: what is liftable versus ideas-only, with licenses.
- Lifted code: factorio-sensei's rcon wrapper and lua readers (MIT, attributed in THIRDPARTY.md). FLE's action vocabulary is the reference when hands arrive.
- Timberbot ([abix-/TimberbornMods](https://github.com/abix-/TimberbornMods)) is the architectural precedent: mod does mechanics, external brain does judgment, errors written for an AI caller, live test harness.

## Doctrine

- Every command sent to the game gets an expected settle signal. Silent failure is the number one killer.
- `build.from-inventory` is the canonical postcondition authority for every bot-built entity. Whether it placed the entity or found it already built, it must bind the exact declared building record by live `unit_number` and verify companion registration before returning success; event tracking is redundant observation only. The shared one-building role immediately commissions any new burner to 10 compatible prototype-derived fuel through `acquire-fuel` (never the craft path; raw fuel has no recipe). Each closed coal loop receives exactly ONE coal total (operator-locked): the first drill of a pair is fueled at placement, and a starting-fuel loading count of one makes the partner's starting-fuel loading skip when a touching same-name drill already holds or burns fuel. The loop-no-coal check is a backstop only; never make it the critical path, because drop-target graph edges are unreliable on never-powered drills.
- Every role declares `desired_state`; catalog loading fully expands nested roles and loops, then rejects any terminal path without an `until_` verification. Framework invariant, not a convention.
- Focused playbooks may declare `handles_interrupts` (names validated against real interrupt playbooks). Only those duplicate recovery interrupts stand down while the focused workflow owns their desired state; defense and every unrelated emergency remain eligible.
- The scheduler emits one authoritative bottleneck decision record (measured constraint, live values, selected playbook) that drives execution, the panel's Bottleneck/Decision display, and telemetry. Never author separate UI wording for a decision.
- Production-run completion is measured, operator-locked 2026-07-20: work the bottleneck until the target is hit, then analyze for the next bottleneck, repeat forever. A finished build whose live rate is still short keeps its bottleneck open and says so plainly; milestone runs without rate targets complete on built state. The active run's bottleneck record re-measures read-only every sweep for the panel; selection still changes only at safe boundaries.
- The quickbar mirrors the factory from live data only: page 1 is materials (mined products, declared products, every item the shared transfer loads into a building), page 2 is placed buildings, first-appearance order, empty slots only, never overwrite the player's own filters.
- Every executor retry is a safe interrupt boundary: unsuccessful attempts signal the watch and wait pause-aware instead of sleeping blindly. A finished interrupt discards its own scoped pending interrupt before the scheduler resumes growth, preventing an orphaned pending interrupt from blocking all later selection.
- Never call `begin_crafting` when `get_craftable_count` is zero. Gather raw shortages through the shared selector or wait for machine-made intermediates; only a positive craftable count permits the write.
- Every successful playbook must request `_autosave-factoriobot-<playbook>.zip` through the companion mod's `game.auto_save` hook before the executor marks it complete.
- New saves default `auto approve everything` on. Render all approval checkboxes from save-persisted `storage.autos`; operator changes survive restarts, while Emergency stop still persistently disables all/player actions until explicitly changed.
- Alerts latch: fire once when a condition starts, not on every poll.
- Bound every queue at creation. Stable entity ids (unit_number), never session-scoped ones.
- Errors tell the caller what went wrong AND what to do next, with valid options listed.

## Factorio 2.1 API drift (live-verified 2026-07-19 to 2026-07-20)

- Player position and reach are controller-dependent. In remote view (map open) and other non-character controllers, `LuaPlayer.position` is the **camera**, and `resource_reach_distance` / `build_distance` can be effectively unlimited. Feeding those into area math or `request_path` crashes the whole game (`position is out of range` / `Chunk.cpp:597 Trying to make chunk at unreasonable position`, live 2026-07-20, non-recoverable). Every path must start from the character body (`body_position` prefers `player.character.position`), every reach/radius must go through the shared sanity clamps (`sane_reach` / `sane_build_radius`), and path goals must pass `sane_map_coord`. Reach clamp alone is insufficient if path start or path radius still use the camera or unlimited build distance. The operator must be able to use the map while the bot works (mod 0.8.13).
- Once a rock/tree mine has started (`simple-entity` gather or in-stride `clearing-obstacle`), finish destroying that entity before stopping or retargeting. Soft coal/stone targets and `craft.ensure`'s `stop_gather` were aborting huge rocks mid-mine (live 2026-07-20). Soft `stop_gather` defers until the rock is gone; Emergency stop still cancels immediately (mod 0.8.14).
- `LuaEntity.drop_target` is nil on a mining drill that has never been powered; only `drop_position` is authoritative on a fresh drill (live-diagnosed via loop-graph diagnostics: 2 nodes, 0 edges). Any drill-output graph must fall back to the entity occupying `drop_position`.
- `LuaBurner.currently_burning` returns a `LuaItemPrototype`, not a string. Compare `currently_burning.name.name` against item names; comparing the prototype itself with a string makes fuel accounting miss the burning item and desired-state transfers retry forever.

- LuaRecipe has no `category` and LuaRecipePrototype renamed `category` to `categories` (array of strings). All prior-art projects (FLE included) predate this. When a reader errors with "doesn't contain key", check lua-api.factorio.com/latest before guessing, and read the newest `factoriobot-*.log` attempt log: every rcon exchange is in it.
- Trigger technologies (research_trigger on the prototype) cannot be queued with add_research; they complete via in-world actions. Exclude them from research proposals, surface their requirements as goals.
- Drive normal movement/mining through `player.character.walking_state`, `mining_state`, and `picking_state`, not the player control adapter. Multiplayer input reconciliation can reduce writes to `player.walking_state` to visible one-tick pulses.
- `request_path` finishes asynchronously. The immediate `start_gather` response can say `path-pending` even when the next path event says `no-path`. Treat both `try_again_later` and `no-path` as nonterminal: keep direct normal walking active every tick and retry pathfinding in the background. Never stop the character between path retries. A repeated distance change of about one running tick per one-second Rust retry means the action is being stopped after the async path event and recreated by Rust.
- Request character paths with the character collision box/mask and `path_resolution_modifier = 2`; the default 1x1 grid is too coarse for many walkable gaps between trees. Follow returned waypoints; direct walking is only the continuous fallback while pathfinding is pending or retrying. Reuse an action only when source name, type, and position all match. Apply one progress monitor in walking, pending, and retry states; less than one tile per 30 ticks is stalled, and after two stalled windows (about one second) mine only a reachable tree or rock blocking the route with normal input, then repath.
- Framework instrumentation is mandatory. Every executor op attempt emits one structured `op_attempt` event; the watch polls the companion action once per second and emits changed `player_action` snapshots with state, exact target, position, waypoint, stuck count, path request/result, and selected obstacle. Add new runtime control through this shared telemetry path instead of subsystem-specific debug prints.
- Use `LuaControl::resource_reach_distance` for normal mining input against every mineable target, including resources, trees, and rocks. `reach_distance` is ordinary interaction reach and may be enlarged independently by other mods. Keep this in one shared action-reach helper and recheck it from the mining state before applying input.
- Build gather candidates from live entity prototypes whose mineable products intersect current raw needs. Search circular radii outward (`8, 16, 32, 64, 128, 256`) and stop at the first radius covering every need. Never combine all resources, trees, and rocks into one capped large-area query: thousands of ore tiles can consume the limit and hide the nearest useful rock.
- Emergency stop is a persistent per-player latch, not a one-tick input write. It cancels the stored action, clears player-action/all autos, rejects new `start_gather` calls, and changes the button to `Resume player actions`. Only that explicit resume clears the latch.

## Live-verified facts (2026-07-18 -> 2026-07-21)

- ping and status work end to end against a hosted Space Age game. helpers.table_to_json and the power reader confirmed live (tests/live.rs, run with `-- --ignored`).
- RCON answers with EMPTY responses while the game is still loading a save. Treat an empty response shortly after connect as retry-able, not as a code bug.
- **2026-07-21 fresh start:** powered lab on copper science island researching; **Automation** completed unattended (~10:18 wall / 11:26 game). Steam plant wall PB **6:44**. Do not regress pole-repair prune, `build-layout-ghosts` `limit:1`, or `research.feed` specialization.

## Open items

- RESOLVED: lua empty tables ({} vs []) handled by the lua_array deserializer on every list field.
- Whether RCON silent-commands disable achievements for the save: pending the player checking in-game, record in docs/design.md.
