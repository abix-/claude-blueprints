---
name: runtime-control-http
description: Pattern for an embedded HTTP control plane in a long-running process (game mod, simulator, GUI app, daemon) that exposes ALL runtime state plus the ability to drive ANY in-process operation. The first thing to build in a new project. It enables research, investigation, prototyping, and TDD. Use when starting any modding/embedding/long-running-process project, when adding observability or test surfaces to an existing one, or when answering "how do I see/poke this thing at runtime".
user-invocable: false
version: "1.5"
updated: "2026-05-09"
---
# Runtime Control HTTP

## What this is

An embedded HTTP server inside the long-running process (game mod,
simulator, daemon, GUI client) that lets an outside caller:

- **Read** any state worth introspecting, in one round-trip.
- **Drive** any operation the process can perform internally.
- **Provoke** events that wouldn't normally occur on demand
  (simulate damage, fire a save, push a message, advance time).

This is **not** "a debug endpoint we'll bolt on later." It is the
platform every other piece of work stands on. Build it FIRST.

## Why it goes first

Before you have a control plane, every research / investigation
loop looks like:

1. Edit code.
2. Rebuild.
3. Reload the host.
4. Reproduce conditions manually.
5. Read logs.
6. Guess.

That cycle is minutes per iteration. Bugs that need many
iterations (the bandage regression in Grounded2 took ~20 cycles
of in-game testing to localize) become days of work.

After you have a control plane:

1. POST a snapshot request, read state.
2. POST an op, observe the resulting state in the same response.
3. Assert in a test.

Iteration is sub-second. Bugs surface as one-line test failures.

The control plane is the **prerequisite** for serious TDD on a
mod or embedded system, because without it the test harness has
no way to set up state, trigger behavior, or assert outcomes.

## Architectural rules

### One endpoint, command-shaped

```
POST /debug   (or /control, /api, etc.)
Content-Type: application/json
Body: { "op": "<name>", "args": {...} }

Response:
{ "ok": bool, "op": "<echoed>", "error": null|str,
  "result": <op-specific>, "state": <FULL SNAPSHOT> }
```

Why one endpoint instead of REST:

- Adding an op = one match arm. No router rebuild, no path-design
  conversation. You're researching; routes will churn.
- The shape stays stable across projects. Test client code is
  reusable.
- Every response carries the full state snapshot, so callers
  always have context without a second round-trip.
- Tests get a deterministic post-op state with no extra request.

REST is right for a public API consumed by third parties.
Command-shape is right for a research/test surface that you and
your tools own.

### Full state in every response

The `state` field is a complete snapshot of everything worth
introspecting. Skill levels, captured baselines, live object
field reads, recent event ring, anything. Tests assert against
`state`; you almost never need a second request.

If the snapshot is too expensive to build every call, gate the
expensive parts behind `args.detail = "full"`. Don't paginate.

### Threading model: pick one based on the host

| Host | Listener thread | State-mutation thread | Queue mechanism |
| --- | --- | --- | --- |
| Unity (C#) | HttpListener bg thread | Unity main thread | ConcurrentQueue, drain in Update() |
| Bevy (Rust) | tokio task | Bevy main world | RemoteHttpPlugin (built-in) or app.add_systems |
| Native game mod (Rust DLL) | tiny_http worker | Game thread (one of our PE trampolines) | Mutex<VecDeque>, drain in trampoline callback |
| WPF / WinForms | HttpListener bg thread | WPF Dispatcher | Dispatcher.Invoke |
| Daemon / server | tokio | Same | None needed |

The rule is simple: **reads run on the listener thread, writes
that touch host state run on the host's main thread**, with the
listener blocking on the queue draining before responding so the
caller sees post-op state.

Picking the wrong thread for a write hangs the host. Observed:
calling UE `ProcessEvent` on a Net-flagged UFunction from any
non-game thread mid-session hangs Grounded 2 indefinitely on the
network replication marker.

### Off by default, opt-in via settings

```json
{ "debug": { "http_port": 17171 } }
```

Production builds don't bind. Devs and integration tests opt in.
Default port can be a project constant; tests honor an env var
(e.g. `BBP_DEBUG_PORT`) and skip cleanly if unset.

## Op set: GENERIC PRIMITIVES, NOT TEST OPS

**This is the most important architectural rule and the easiest
to get wrong.** The mod-side endpoint exposes a small set of
GENERIC primitives. The test client composes those primitives
into whatever scenarios it wants. Test logic NEVER lives in the
mod.

Why this matters: every new test idea that requires a new mod-side
op means rebuild + redeploy + relaunch the host. That's
minutes-per-iteration. With generic primitives, new tests are
test-file-only changes and run instantly against the running
host.

**The MAXIMUM-GENERIC primitive set.** These five ops cover ~95%
of "do anything" needs in any embedded host (UE mod, Unity mod,
GUI client, daemon, ECS sim). Once they're in, **the endpoint
should NEVER need to grow again**. Every new test or research
question is a test-file change, not a mod change.

| op | purpose |
| --- | --- |
| `snapshot` | returns the state struct. One default useful read; cheap discoverability |
| `read_bytes` | read raw bytes at `(instance_selector, offset, length)`. Tests parse the bytes themselves using SDK-shaped structs. |
| `write_bytes` | write raw bytes. Same shape, with `bytes_hex` arg. |
| `call` | invoke any method/UFunction by `(instance_selector, class, function, parms_hex)`. Returns parms post-call (engine writes OUT params). |
| `enumerate` | (a.k.a. `walk_class`) iterate instances matching a class/type, return addresses + summaries. Tests use the addresses as `addr:0x...` selectors in subsequent ops. |

**Selectors are how you target anything.** Universal grammar:

| selector | meaning |
| --- | --- |
| `addr:0x...` | raw object address (returned by `enumerate`) |
| `class:<Name>` or `first_class:<Name>` | first instance of a class |
| `singleton:<Name>` | singleton-style object (CDO, GameState, etc.) |
| `entity:<id>` | (ECS hosts) entity by id |
| `<host-specific-shorthand>` | well-known shortcuts (`live_player`, `current_save`, ...) |

A test composes: `enumerate` -> pick an `addr:0x...` -> `read_bytes` /
`write_bytes` / `call` against it. Five ops + a selector grammar
cover any combination of read / write / invoke on any object the
host has.

### What this does NOT cover (and what to do)

- **Dynamic hook installation.** Tests can't ask the mod to start
  intercepting `UFunction X` at runtime. Hooks live in the mod
  and require a build. Mitigations: cover the most-common hook
  surfaces in the mod's startup (a few well-chosen PE
  trampolines), expose the captured events through a named ring
  in the snapshot, and let tests poll. If a new hook surface is
  truly needed, that IS a mod change. But it's **one mod
  change per hook**, not per test.
- **Strongly-typed introspection.** Tests need to know the
  host's data layouts (e.g. UE SDK headers, Unity TypeTree).
  This is unavoidable. Distribute the SDK to test code; tests
  build `#[repr(C)]` parm structs and field-offset constants.
  The endpoint stays untyped (raw bytes); types live in tests.
- **Rich queries beyond class.** `walk_class` is class-based.
  For predicate filters, the test reads bytes and filters
  client-side. Almost always cheap enough.

### Optional in-mod ring buffers (the one acceptable mod-side
"events" surface)

When a host has hot event surfaces the test can't reach without
a hook (e.g. UE damage multicast, Unity collision events), the
mod installs the hook ONCE and pushes captured events into a
named ring in the snapshot. Tests read the ring; tests never
install hooks.

If you find yourself adding a SECOND ring of the same shape, ask
why: usually one wider ring + a filter on the test side is the
right call.

**Domain-specific ops are smells.** Before adding `simulate_X` or
`do_Y` to the mod, check: can the test compose this from `call` +
`read_field`? If yes, the op is wrong and belongs in the test.
Reasonable exceptions:
- Convenience ops that wrap a *coherent gameplay action* the
  user actually performs (`skill_spend`, `skill_toggle`). The
  mod-side function already exists, the op just exposes it.
- Ops that require in-mod state the test can't reach (event
  buffers populated by a PE trampoline that lives in the mod).

If you find yourself adding `simulate_apply_damage`,
`simulate_heal`, `simulate_status_effect_add` etc. as separate
ops, **stop**. Add `call` once and let tests invoke
`UHealthComponent::AddHealth`, `UHealthComponent::ApplyDamageFromInfo`,
`UStatusEffectComponent::CreateAndAddEffect` themselves.

The skill's earlier draft listed `simulate_<event>` as a starter
op; that was wrong. Replace those with `call` and let the tests
do the simulation.

### Why this enables the research-as-code loop

With generic primitives:
1. New research question -> new `tests/explore_*.rs` file.
2. Run with `cargo test --test explore_*`.
3. Test calls primitives, asserts state, prints findings.
4. Mod is untouched. Game keeps running. No relaunch.

With domain-specific ops:
1. New research question -> add op to mod.
2. Build, deploy, restart host.
3. Write test to call the op.
4. Realize you need slightly different parms -> back to step 1.

The first loop is sub-second iteration. The second is minutes
per iteration. The architectural choice IS the iteration speed.

## Test coverage principle: every user interaction is a test

**The control plane is what makes this possible, but the
discipline is yours: every feature a user can touch and every
expectation a user can hold becomes an integration test against
the endpoint.** This is the most critical layer of testing in a
mod / embedded / long-running system, because it is the layer
that proves the *user-observable* behavior works. Not the
internal logic, not a mock, the real host driving the real code.

Approach:

1. **Enumerate user-facing features.** For each one, write down
   what the user does and what they expect to see / not see /
   hear / feel. UI buttons, hotkeys, settings toggles, every
   feature page in the README.
2. **Each entry becomes a test (or a test family).** Use the
   `<feature>_test.rs` (or per-language equivalent) naming so
   coverage gaps are obvious at a glance.
3. **The test drives state via ops, observes via the snapshot,
   asserts the user's expectation.** No mocks, no fakes; the
   host is the truth.
4. **Edge cases get their own tests:** boundary values, invalid
   args, state-machine transitions you'd be surprised by, and
   especially **interactions between features** (the kind of
   bug you only catch by doing X *while* Y is enabled). The
   bandage / impact_resistance regression in Grounded2 was
   exactly this: each feature individually was correct, but the
   interaction blocked a separate user expectation (healing).
5. **A bug is a missing test.** When a regression slips, write
   the test before the fix. The test stays as a fence forever.

### What "every interaction" looks like

For a skill catalog mod (Grounded2):

- For EACH skill: spend points 1 -> max -> assert effect grows.
- For EACH skill: spend then refund -> assert effect reverts.
- For EACH skill: spend then toggle off -> assert effect off.
- For EACH skill: spend then toggle on -> assert effect on.
- For EACH skill: spend, reload save -> assert state persists.
- For EACH skill: leave at 0 -> assert no effect (vanilla).
- INTERACTIONS: every pair of skills that touches the same
  game subsystem (combat, movement, healing, damage gates).
- USER INPUTS: the in-game flows. Killing creatures, taking
  damage, using consumables (bandages!), changing equipment,
  reloading saves, switching characters.
- ERROR PATHS: invalid args, unknown ids, op-while-no-slot.

For an inventory mod: every interaction that touches an
item slot. For a UI client: every visible control.

This will look like a lot of tests. That's the point. The
control plane was built so this is *cheap*, and the cost is
amortized over every future bug it catches before the user
sees it.

### Two-layer test split

| Layer | Catches | Lives in |
| --- | --- | --- |
| **Integration (this layer, primary)** | Real user-observable behavior, real bugs, host-side regressions | `tests/integration/*.rs` over the HTTP endpoint |
| Unit | Pure-Rust math, parsing, formatters, edge cases that don't need the host | `#[cfg(test)] mod tests` in the source files |

Most projects under-invest in integration tests because the
infrastructure is hard. The control plane removes that excuse.
Default to integration; only write a unit test when the thing
under test is genuinely host-independent.

## TDD workflow this enables

Write the failing test FIRST against the endpoint. The endpoint's
existence makes the test possible; writing the test reveals which
ops you need next; building those ops drives the implementation.

```rust
#[test]
fn impact_resistance_does_not_block_bandages() {
    let api = common::Api::require();
    api.op("skill_spend",  json!({"id": "impact_resistance", "count": 100}));
    api.op("skill_toggle", json!({"id": "impact_resistance", "enabled": true}));
    let before = api.snapshot().player.hp;
    let r = api.op("simulate_heal", json!({"amount": 20}));
    let after  = r.state.player.hp;
    assert!((after - before - 20.0).abs() < 0.5,
        "heal blocked; got delta {}", after - before);
}
```

That test compiles before any of `skill_spend`, `skill_toggle`,
`simulate_heal` exists. Each is one ticket. Test fails red until
the implementation lands. When it goes green, the feature is
verified by a runnable contract that survives every future change.

This is the only honest TDD model for in-process work. You can't
unit-test a UE mod the way you unit-test a function: the host is
the truth, and the control plane is how you talk to it.

## Reference implementations

### Rust DLL mod (Grounded2 + Outworld Station)

- **Crate**: `tiny_http = "0.12"` (sync, single thread, ~150 LoC of
  glue). No async runtime needed.
- **Shared crate `ueforge`** (rlib in `grounded2mods/ueforge/`): the
  pattern extracted as a reusable library for any UE-game Rust mod.
  Modules:
  - `server`. Tiny_http listener + dispatch
  - `envelope`. `OpResponse<S>`, `parse_request`
  - `args`. JSON arg helpers
  - `pe_queue`. `Queue` with re-entrance guard, lock-free fast path,
    drain stats
  - `selector`. Generic `addr:0x...`, `first_class:Foo`
  - `hex`. Encode/decode codec
  - `ops`. `read_bytes`, `write_bytes`, `walk_class`, `exec_call`
  - `counters`. `bump`, `observe_peak`, `time_scope`, `TimeScope`
  - `ring`. Bounded drop-oldest ring buffer for hook events
  - `log`. File + console DLL logger (AllocConsole +
    GetModuleFileNameW + timestamped writer)
  - `winproc`. Windows process introspection (threads, CPU,
    regions, memory, thread sampler)
  - `ue`. UObject/UClass/UFunction/FName/FString/TArray/GObjects/
    Platform offsets, plus `ue::probe` (gobjects_population,
    class_outer_samples)
  New UE-mod projects add one workspace dep and only supply a
  `Snapshot` type + drain wiring.
- **Game crate (e.g. better-backpack)**: owns `DebugCmd`,
  `Snapshot`, `build_snapshot`, the `op_*` handlers, the
  drain-site PE trampoline, perf counters. Calls `ueforge::spawn`
  with a closure that calls game-side `handle()`. Calls
  `PE_QUEUE.drain()` from inside its trampoline.
- **Game-thread queue**: `static PE_QUEUE: ueforge::Queue =
  ueforge::Queue::new();` Drain inside an existing PE trampoline
  (e.g. kill_hook), guaranteed to run on game thread because UE
  calls our trampoline from there. `Queue::enqueue` returns the
  generic timeout error string; the game wraps with a host-specific
  hint ("Is kill_hook firing? Move around / take damage...").
- **Tests**: `better-backpack/tests/common/mod.rs` uses `ureq`
  (blocking, no tokio). Each `tests/<scenario>.rs` is a separate
  binary. Run with `--test-threads=1` (shared global state).
  Shared test-client crate `ueforge-client` provides
  `Api<S>` (generic over snapshot type) with `try_connect`, `op`,
  `op_ok`, `snapshot`, `call_ufunction`; matching `OpResponse<S>`
  deserializer; and `hex` + `parms` helpers for `#[repr(C)]`
  parm-buffer round-trips. Game test crates wrap with their own
  newtype to add per-game convenience methods (e.g. `skill_spend`).
- **Deployment**: framework-level Rust binary `ueforge-deploy`
  reads each mod's `[package.metadata.ueforge]` (mod folder name,
  game-detect regex, UE4SS subpath, zip prefix), then handles
  Steam library lookup + UE4SS presence check + DLL copy +
  `mods.txt` management. `cargo deploy install -p <mod>` (alias)
  drops `main.dll` into the game install. No PowerShell, no
  per-mod scripts; every mod uses the same binary.
- **Patterns extracted from the first two mods**:
  - `ueforge::settings::Settings<T>`. Atomic-save JSON-backed
    settings under `<DLL_dir>/settings.json` (load on construct,
    save-on-update with temp+rename).
  - `ueforge::ue::datatable::FieldTweak<T>`. Vanilla snapshot +
    idempotent re-apply for "mutate field N on every row by some
    transform of the vanilla value" features (stack-size mods,
    drop-rate adjusters, etc.). Generic over `T: Copy + PartialEq`.
  - `ueforge::ue::datatable::on_first_sight(name, timeout, cb)`.
    poll-for-DataTable worker that fires once on first
    sighting. Used to land DT mutations BEFORE any UI widget
    caches a row copy.

### C# Unity mod (Timberbot)

- **Source**: `timberbot/src/TimberbotHttpServer.cs`. `HttpListener`
  on a `Thread`. GET handled inline (snapshot reads from a
  pre-built thread-safe view). POST routed to a
  `ConcurrentQueue<PendingRequest>`, drained in
  `TimberbotService.Tick()` from Unity's main thread, max 10 per
  frame to avoid frame-time spikes.
- **JSON**: Newtonsoft.Json with a hand-rolled zero-alloc
  `TimberbotJw` writer for the snapshot hot path.
- **Tests**: Python harness in `timberbot/script/test_v2.py`
  hits the endpoint over HTTP. Test specs in
  `test_v2_specs.py`.

### Bevy game (Endless)

- **Plugin**: `bevy_remote::RemoteHttpPlugin` provides the HTTP
  layer; you register custom methods on top. JSON-RPC at
  `localhost:15702`. Methods are namespaced (`endless/get_perf`).
- **Threading**: Bevy's BRP runs methods as Bevy systems; you
  write them as normal `Query` / `ResMut` systems and the plugin
  handles the IO + scheduling.
- **Client**: Go binary `endless-cli` wraps BRP with `key:value`
  CLI args. Source `llm-player/main.go`. See
  `~/.claude/skills/endless-cli/SKILL.md`.

### WPF / desktop client

- `System.Net.HttpListener` on a worker `Thread`. POST handler
  marshals state changes via `Application.Current.Dispatcher.Invoke`
  so they run on the UI thread (where bindings + view-models
  live). Same pattern, different "main thread" name.

## DRY: one authoritative path for everything

Once the control plane exists, **resist any pattern that
duplicates its job**. Common temptations and what to do instead:

- **"Let's add a separate /healthcheck route."** No. `op:
  "snapshot"` already returns full state; the absence of a
  response IS the health check. If you really need a
  zero-op-cost ping, put it in the snapshot path under a
  cheap field, not a new route.
- **"Let's add a quick GET /api/skill/{id}."** No. `op:
  "snapshot"` already returns every skill in one call. If
  one read is too big, gate detail on `args`. Still one
  endpoint.
- **"Let's add CLI args that bypass the endpoint and poke
  state directly."** No. The CLI calls the endpoint. The
  endpoint is the only authoritative path.
- **"Let's add a separate test fixture that mocks the
  state."** No. The endpoint snapshot IS the fixture.
  Tests use `api.snapshot()` and never simulate state in
  test code.
- **"This op is similar to an existing one, let's
  copy-paste."** No. Refactor the shared logic into a
  helper, parameterize the existing op, or compose. Two
  ops that both `set_X_with_special_rules` are a smell.
- **"The test client wants its own data types."** No. Test
  client deserializes the SAME `Snapshot` / `OpResponse`
  types the server defines (or a deliberately narrower
  view). Drift between server-side and test-side types is
  how you ship a bug that passes tests.

The rule is: **one endpoint, one set of types, one set of
ops, one shared client.** If you find yourself building a
second one, the first one has the wrong shape. Fix that,
don't fork.

### What "shared" looks like in code

| Concern | Server side | Test client side |
| --- | --- | --- |
| HTTP shape | one POST /debug handler | one `Api::op()` method |
| Response type | `OpResponse { ok, op, error, result, state }` | identical, deserialized |
| Snapshot shape | `Snapshot { ... }` | identical, deserialized |
| Per-op convenience | match arm | `api.skill_spend(id, count) -> Snapshot` |

The test client's `api.skill_spend(...)` is sugar over
`api.op("skill_spend", json!({...}))`. It exists because
test code reads better that way, not because the protocol
is different. Both call the same endpoint; the convenience
layer is one helper, not a parallel implementation.

## Anti-patterns

- **Multiple endpoints / RESTful routes for a research surface.**
  You'll be churning them every session. Single command-shaped
  endpoint scales.
- **Reading game state on the listener thread when the read isn't
  thread-safe.** Some hosts (Unity, UE) require even reads to be
  on the main thread. When in doubt, queue.
- **Async in the mod-side server.** Adds tokio, increases binary
  size, fights the host's threading model. Sync `HttpListener` /
  `tiny_http` is enough for ~hundreds of requests/sec; if you
  need more, you have a different architecture problem.
- **Draining the PE queue from inside another PE trampoline.**
  If your "game thread" surface is a ProcessEvent hook and the
  op you're executing is itself a `process_event` call, you are
  re-entering ProcessEvent. For most UFunctions that's fine;
  for any that triggers replication, blueprint events, or
  network RPCs, the inner call can deadlock or AV the host.
  Observed: in Grounded2, draining `simulate_apply_damage` from
  `kill_hook`'s trampoline crashed the game because
  `ApplyDamageFromInfo` triggers damage replication. Two fixes:
  pick a quieter drain site (a hook that fires less often, on a
  function that doesn't replicate), or use the host's official
  "post to game thread" primitive (UE4SS's
  `RegisterProcessEventPreCallback`, Unity's `Update()`,
  Bevy's system schedule). Re-entrancy is the most common
  "everything compiled, host hangs" symptom on first roll-out.
- **Auth on a localhost-only endpoint with the port off by
  default.** Skip it. The settings flag IS the auth.
- **Running tests against a process that isn't yours.** Tests
  must own their setup; have an op like `reset_state` or
  `reload_slot` that returns the host to a known baseline.
- **Test harness in a different language for no reason.** Match
  the host: Rust mod -> Rust tests, C# mod -> Python (Timberbot's
  choice; both work, Python won there because the test author was
  more fluent). The control plane is language-agnostic; only the
  test client picks a language.
- **Logging instead of state.** Logs are write-only history. The
  snapshot endpoint is queryable current state. Use logs for the
  story-after, the snapshot for the now.

## Order of operations: the day-to-day loop

Once the control plane exists, every feature, every bug, every
research question follows the **same loop**. Internalize this.
it's the discipline that turns the endpoint from a "debug tool"
into the platform you build on.

### Phase 1: Build the control plane (one time per project)

1. **Pick the port + settings flag.** Default off. Document in
   the project's settings.example.json.
2. **Bind the server on a worker thread.** Smallest possible
   handler that returns `{ok: true, state: {}}`.
3. **Snapshot type with one field.** Compile, run, hit it from a
   browser or `curl`. Verify roundtrip works.
4. **Wire the test client.** Shared `Api` wrapper, `try_connect`
   skip-when-not-set, the response/snapshot types matching the
   server.
5. **One smoke test that calls `snapshot` and asserts shape.**

You're now ready for the loop.

### Phase 2: The feature/bug/research loop

For every "I want to do X with the host":

1. **Decide the user-observable expectation.** Concretely: "after
   I do A, B, C, the player's HP should go up by 20." If you
   can't say it concretely, you haven't understood it yet.
2. **Make sure the snapshot exposes the observable.** If the
   field you need to assert against isn't in `state` yet, ADD IT.
   This is cheap: one field on the server, one field on the test
   client. You're going to need it again later anyway.
3. **Write the failing test.** It calls the ops it needs (which
   may not exist yet), reads the snapshot, asserts. Compile-clean
   but red at runtime ("unknown op" or assertion failed).
4. **Use the endpoint to research.** While the host is running,
   `curl -X POST .../debug -d '{"op":"snapshot"}'` (or call from
   a REPL / quick script) to **peek at the live state**. What is
   the field's vanilla value? What range? What's around it?
   Hand-poke ops as you implement them; verify they do what you
   expect before adding them to the formal test.
5. **Implement the op.** One match arm + one helper. Touch the
   shared `OpResponse` / `Snapshot` types if needed (rare; mostly
   you're adding a snapshot field, not changing the envelope).
6. **Run the test. Watch it go green.** This is the
   "implementation done" signal. If it doesn't go green,
   the bug is real. You found it before the user did.
7. **Land the test, the op, and any snapshot additions in one
   commit.** They're a unit; they ship together.

Repeat. Forever. There is no "I'll write the test later" in this
loop. The test is what proves the feature exists.

### When you find a bug

1. **Write the regression test FIRST.** Use the endpoint to
   reproduce the bug. The test fails red. The test is the bug.
2. **Fix the code.** Don't touch the test.
3. **Test goes green.** Land both in one commit. The test stays
   as a fence forever.

### Research is code, not curl

When you're investigating runtime behavior, **don't reach for
ad-hoc curl from a shell**. Write the experiment as code that
uses the test client, runs against the live endpoint, and prints
or asserts the observation. Three reasons:

1. **The experiment becomes the regression test.** The code that
   proves "calling X with args Y produces state Z" is structurally
   identical to a test asserting that. Once you've confirmed the
   behavior, change `eprintln!` to `assert!` and the experiment
   IS the test. Throw nothing away.
2. **Reproducibility.** A test file in the repo is reproducible
   forever. A curl line in your shell history is reproducible
   for ten minutes. Future-you (and future-Claude) will need to
   re-run the experiment when the host updates. Have it sitting
   in `tests/` ready to go.
3. **Layered abstractions.** Research code uses the same `Api`
   wrapper, the same `Snapshot` types, the same conveniences as
   tests. The investment compounds: every helper added for
   research is one more thing tests can use.

Pattern:

```rust
// tests/explore_<topic>.rs -- exploratory research, kept in the
// repo. Starts as `eprintln!` of observations; converges to
// `assert!`s once the behavior is understood. Final form is a
// regression test indistinguishable from any other.

#[test]
fn explore_apply_damage_gate() {
    let api = common::Api::require();
    api.skill_toggle("impact_resistance", true);
    let with_mask = api.op("simulate_apply_damage",
        json!({"amount": -20.0, "type_flags": 0}));
    eprintln!("mask=on, type_flags=0: {:#?}", with_mask.result);

    api.skill_toggle("impact_resistance", false);
    let no_mask = api.op("simulate_apply_damage",
        json!({"amount": -20.0, "type_flags": 0}));
    eprintln!("mask=off, type_flags=0: {:#?}", no_mask.result);
    // Once we know which one heals: assert the right shape and
    // promote `eprintln!` to `assert!`.
}
```

Run with `cargo test --test explore_apply_damage_gate -- --nocapture`.
Capture observations. Promote to assertions. Commit.

If you find yourself typing a curl command, stop. That command
should be a `cargo test --test ...` invocation against a Rust
file you'd land. The endpoint exists so the test client can
drive it; bypassing the test client wastes the leverage.

The only acceptable curl is the one-time check that the endpoint
is alive (e.g. snapshot returns). And even that is better as a
test (`tests/debug_snapshot.rs`).

### When you're stuck or confused about runtime behavior

1. **Don't add more logs.** Logs are write-only and require
   another iteration to read.
2. **Add a snapshot field.** If you need to know "what is the
   value of X right now", expose X in the snapshot. Hit the
   endpoint. Read the answer.
3. **Try ops directly.** Want to know what happens when you
   call ApplyDamage with negative damage? Add `simulate_damage`
   as a one-line op, hit it from `curl`, see what state changes
   in the response. You don't need a hypothesis to investigate;
   you need the surface to investigate WITH.

This is what "the endpoint is the platform" means: it's the
**observation, control, AND research surface**, all at once. You
will know it's working when "let me check the snapshot" replaces
"let me add a print statement and rebuild."

### Anti-loop: don't slip into these

- **"I'll just edit the code and rebuild and look at logs."**
  No. Add the snapshot field. Use the endpoint.
- **"I'll write the test after the feature is working."** No.
  The test is the definition of "working."
- **"This change is too small for a test."** Probably wrong.
  But if true, it's also too small to ship without one.
- **"I'll add a separate /debug-only endpoint for this case."**
  No. One endpoint. Add an op or a snapshot field.
- **"Logs are easier."** Logs are easier *to write*, much harder
  *to use*. The snapshot is the opposite trade.

## When the user invokes this skill

Likely in one of these shapes:

- "How do we debug X at runtime?" -> Add a snapshot field for X.
- "How do we test X?" -> Write the test against the endpoint;
  add ops as needed.
- "Adding feature X to a mod / sim / client." -> Make sure the
  control plane exists first; if not, build it; then add the op
  for X and write the test.
- "How does the {endless, timberbot, grounded2} debug endpoint
  work?" -> Point at the Reference implementations section
  + the project's source.
