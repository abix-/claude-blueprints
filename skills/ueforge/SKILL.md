---
name: ueforge
description: ueforge framework: the base layer every UE4SS Rust mod in the Grounded2Mods workspace builds on. Authoritative on the composition model (Effect/Trigger/Skill), the Def/Registry/Instance/Controller pattern, hot reload, discovery, hardening doctrine, and the five framework modules (rpg, stacks, difficulty, inventory, damage). Use when writing or modifying code under `ueforge/` in [abix-/Grounded2Mods](https://github.com/abix-/Grounded2Mods), or when promoting a pattern out of a game crate into the framework.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# ueforge: the framework

`ueforge/` is the base crate every UE4SS Rust mod in this
workspace consumes. It owns the lifecycle, the UE SDK, hooks,
HTTP control plane, ImGui bindings, the C++ shim, hot reload,
discovery, browsers, scanner, freeze, the deploy CLI, and an
opinionated set of composition modules.

Operating principle: **always change ueforge first.** If a need
is game-specific, prove it; otherwise the pattern belongs here.
If you find yourself writing the same scaffolding in two game
crates, that is a missing module: promote it.

`ueforge/docs/architecture.md` is the contract. Read it before
adding a new module.

`ueforge-deploy` is **no longer a separate crate**. The deploy
CLI is a `[[bin]]` target inside ueforge, exposed as
`cargo deploy` via the workspace `.cargo/config.toml`.

## The composition model: Effects + Triggers + Skills

> Each operation we figured out how to perform in the game is an
> **Effect**. A **Skill** is one or more Effects applied with
> parameters. **Triggers** decide WHEN to fire. Research each
> operation **once**.

Three concerns, three vocabularies, each with its own Def:

- **`Effect`** (the verb): the operation shape. `impl Effect`
  types own `apply(level, max_level, ctx: &TriggerCtx)` and
  `format(level, max_level)`. Framework ships
  `PlayerFloatEffect`, `SubcomponentFloatEffect`,
  `SubcomponentMultiplyEffect`, `SubcomponentAdditiveEffect`,
  `SubcomponentU32MaskEffect`, `ClassFieldsMultiplyEffect`,
  `RuntimeEffect`, `StatusEffectApply`. Game crates impl new
  Effects only for operations no other game shares.
  `EffectDef { kind, imp: &'static dyn Effect }`.

- **`Trigger`** (the WHEN): `TriggerDef { kind, imp: &'static dyn
  Trigger }`. Today: `ON_SLOT_CHANGE` (passive: fires on level /
  spend / refund / toggle). Future variants: kill / damage / fall
  / periodic event triggers. `TriggerCtx` is the typed event
  payload passed to `Effect::apply`.

- **`Hook`** (the mechanism): `HookDef` is the vtable patch on a
  UE5 class. **Hooks ≠ Triggers.** Triggers BUILD ON hooks plus
  filter + decode + typed dispatch. N:M between hooks and
  triggers. Some triggers (Periodic, OnSlotChange) install no
  hooks at all. Skill authors compose at the Trigger layer.

- **`Skill`** (the product): `SkillDef { id, display_name,
  max_level, effect: EffectDef, trigger: &'static TriggerDef }`.
  A catalog row pairs WHAT (Effect) with WHEN (Trigger).

There is NO `SkillEffect` enum. The old enum was deleted in the
trait migration. Do not reintroduce one. New game-specific
operation = `impl Effect` + `static FOO: FooEffect = ...` + one
catalog row. No central match arm.

## Def -> Registry -> Instance -> Controller (the k8s pattern)

Every subsystem follows this layering, modeled on Kubernetes CRDs:

| Role           | Meaning                                                |
| -------------- | ------------------------------------------------------ |
| **Def** (CRD)  | The schema. Static, immutable, no runtime state.       |
| **Registry**   | The collection of Defs + lookup.                       |
| **Instance**   | One runtime object derived from a Def.                 |
| **Controller** | Function/system that reads Def + writes Instance.      |

The Def is the source of truth. Controllers re-derive at every
reconcile. Instances never cache Def fields.

### Naming contract (mandatory)

- Schema: `<Subject>Def`. **Always Def-suffixed: no exceptions.**
  (`SkillDef`, `StackDef`, `StatusDef`, `HookDef`, `OpDef`,
  `SelectorDef`, `ShutdownHandlerDef`, `BuildingDef`, `TabDef`,
  `ModDef`, `TriggerDef`, `EffectDef`, `CreatureDef`,
  `DataTableDef`, `DifficultyDef`.)
- Registry wrapper: `<Subject>Registry` holding
  `entries: &'static [<Subject>Def]` + any registry-level config.
- Lookup: `registry.def(key) -> Option<&'static <Subject>Def>`.
- Stateful subjects: `<Subject>Tracker`. Stateless: bare
  functions (`apply_skill`, `dispatch_op`, `place_building`).
- Discriminator: `<Subject>Kind` enum (closed sets) or
  `&'static str` id (open sets). Field name is `kind` / `id`
  (never `name` / `job` / `activity`).

K8s-slot header line on every subject's module doc:
```
// K8s slot: Def=SkillDef, Registry=CATALOG, Instance=SkillsState.skill_levels, Controller=RpgApplier::apply_skill
```

### Bare-slice carve-out

Subjects with no registry-level config AND no foreseeable use
for one MAY use `&[<Subject>Def]` directly. Canonical example:
`MOD_INFO.tabs: &'static [TabDef]`. Mark the carve-out in the
module's K8s-slot header.

### Slice-of-refs (two-static pattern) for Drop-having Defs

When a Def carries non-Copy state with non-trivial `Drop`
(mutexes, atomic caches, owned heap), Rust's const-eval rejects
a temp array literal:

```rust
// E0493: destructor of [StackDef; N] cannot be evaluated at compile-time
pub static STACKS: StackRegistry =
    StackRegistry::new(&[StackDef::new(...)]);
```

The canonical pattern: not a workaround: is each Def as its
own named `static` + the registry stores `&[&'static <Subject>Def]`:

```rust
static MATERIALS_DEF: StackDef = StackDef::new("materials", ...);
pub static STACKS: StackRegistry =
    StackRegistry::new(&[&MATERIALS_DEF]);
```

Required for: `StackRegistry`, `DifficultyRegistry`,
`StatusRegistry`, runtime-populated `HookRegistry`.
Drop-free Defs (Skills, Creatures, Tabs) MAY use the simpler
slice-of-values shape.

### Compliance scorecard

`ueforge/docs/architecture.md` carries the full table. Current
state: Skills, Triggers, Effects, Creatures, Tabs, Mod, Stacks,
Difficulty, Data tables, Statuses, Debug ops, Selectors,
Shutdown handlers, Hooks all sit at 100% (Hooks at 90%:
imperatively populated). Buildings is designed-100%, not yet
built. Counters and PE-queue jobs are documented carve-outs.

## The five framework modules

Each wraps a low-level primitive with the universal apply-loop +
atomic-knob + status-counter pattern. Game crates pick from the
menu and write only game-specific knobs (class names, offsets,
parm shapes). Heterogeneous adoption is supported: a pure
stack-size mod consumes only `stacks`; an RPG-only mod only
`rpg`.

### `rpg`
Skill catalog + XP curve + bestiary + per-slot persistence +
ImGui tab. Public surface:
`SkillDef` / `SkillRegistry`, `EffectDef` + `impl Effect`,
`TriggerDef` + `impl Trigger`, `CreatureDef` / `CreatureRegistry`,
`Tracker`, `XpResult`, `Curve`, `SlotPoller` / `PollerHandle`,
`SlotStore`, `SlotKeyResolver`, `SkillsState`, `DisabledSkills`,
`VanillaCache`, `PercentFormat`, `StatusDef` / `StatusRegistry`,
`tab::render`, `ops::register`.

### `stacks`
Data-table stack-size tweak. `StackDef` captures vanilla on
first sight, applies `vanilla * multiplier_bits.load()`,
idempotent on re-apply. `StackRegistry::apply_all_now` returns
`Vec<(id, Result)>` for per-table telemetry. Multiplier is an
atomic f32-bits so hot reload doesn't disturb writers.

### `difficulty`
Game-difficulty CDO field tweak. Same capture / multiplier /
re-apply shape as stacks, but against a CDO instead of a data
table row. `DifficultyDef::apply_to_cdos` /
`apply_with_filter` / `DifficultyRegistry::apply_all_to_cdos`.
Skip-if-unity short-circuit at the Def level.

### `inventory`
Viewport-paging hook framework for widgets that bump capacity
beyond the visible grid. Owns mouse-wheel scroll + per-widget
viewport-start state + synthetic-refresh re-entrance guard +
post-refresh rebind. Game crate impls a thin
`inventory::viewport::ViewportBinder` trait with parm shapes
and bind logic.

### `damage`
Universal damage-event hook for the multicast / RPC every UE5
game fires per damage hit. `damage::DamageHook` owns the
trampoline + parm decode + `FDamageInfo` lookup + Player/Other
classification + `before` / `after` dispatch. Game crate impls
`DamageBinder` to do Critical (pre, mutate damage), Evasion
(pre), Lifesteal (post), Thorns (post), kill credit (post).

If a need is wholly framework-shaped but not in one of these
five, file it under "Open: more ueforge extraction candidates"
in `docs/todo.md` rather than re-inventing in a game crate.

## UE4SS CPPMod loading

ueforge ships `cpp/ueforge_shim.cpp`: a generic UE4SS factory
that subclasses `RC::CppUserModBase`. The game's `build.rs` calls
`ueforge::build::CppShim::new().compile()` to link it. UE4SS
loads `main.dll`, invokes the shim's `start_mod` factory, the
shim instantiates a generic `UespyMod`, and forwards every
callback to Rust via `extern "C"` hooks emitted by the
`ueforge::ue4ss_mod!` macro.

### `ModDef` is the root Def

```rust
static MOD_INFO: ueforge::ModDef = ueforge::ModDef {
    name: "GameName",
    version: "0.1.0",
    log_file: "game_name.log",
    console_title: "Game Name",
    console: cfg!(feature = "console"),
    on_unreal_init: || { game::start(); },
    on_shutdown: || { game::stop(); },
    tabs: &[ ueforge::TabDef { name: "Tweaks", render: game::render } ],
};
ueforge::ue4ss_mod!(MOD_INFO);
```

The macro emits `ueforge_mod_get_name`, `_get_version`,
`_get_log_file`, `_get_console_title`, `_get_tab_count`,
`_get_tab_name`, `_dll_attach` (forwards HMODULE into the
logger), `_unreal_init`, `_shutdown`, `_render_tab`, plus the
DllMain forwarder.

`ModDef` is **not** `#[non_exhaustive]`. Every consumer
constructs it as a struct literal. Adding a field IS a
breaking change for all consumers but the monorepo updates
atomically: document new fields in `changelog.md`.

### `extern "C"` ImGui bridge

The macro emits the bridge extern (`ueforge_ui_enable_imgui`)
at the **consumer** crate site, NOT inside `ueforge.rlib`. This
lets ueforge build scripts and test binaries that depend on
`ueforge` link cleanly without pulling in `ueforge_shim.cpp`'s
UE4SS deps.

## Hot reload (zero-touch)

`cargo deploy install` writes `main-new.dll` next to the
running `main.dll`. Three paths from there:

1. **Game running, focused**: `hot_reload::spawn_watcher`
   polls every 1.5 s. On detection it synthesizes Ctrl+R via
   `SendInput` to whichever window has foreground. UE4SS's
   keybind handler calls into `ueforge_mod_shutdown`, which
   runs `finalize_hot_reload_swap`:
   - `rename main.dll -> main-old.dll` (legal because
     `LoadLibraryExW` opens with `FILE_SHARE_DELETE`).
   - `rename main-new.dll -> main.dll`.
   - On failure of step 2, roll back step 1.
   - UE4SS then `LoadLibraryExW`: picks up the new image.

2. **Game closed when deployed**: `apply_pending_swap_at_init`
   runs on the next launch's `ueforge_mod_unreal_init`. Current
   generation runs from the renamed `main-old.dll` mapping; the
   next Ctrl+R or relaunch picks up the new code.

3. **`main-old.dll` cleanup**: `cleanup_old_dll` removes the
   leftover at init.

Caveats:
- Synthesized Ctrl+R reaches the **foreground window**. If the
  user has focus on another window, the reload is deferred to
  the next poll where the game is focused.
- One reload per deploy. After Ctrl+R the watcher exits; the
  freshly-loaded image's `on_unreal_init` spawns a new watcher
  for the next deploy generation.

## Lifecycle + shutdown registry

The macro's `ueforge_mod_shutdown` order:
1. Game's `on_shutdown` callback (mod-specific teardown that's
   awkward to express as a registered handler).
2. `shutdown::register_builtins()` registers framework handlers
   (hooks=100, http=200, settings=300, freeze sweeper=400).
3. `SHUTDOWN_REGISTRY.run_all()` sorts by `order`, runs each,
   logs.
4. `finalize_hot_reload_swap()`: side-file rename so UE4SS's
   next `LoadLibraryExW` picks up the new image.

Game-specific cleanup goes via `SHUTDOWN_REGISTRY.register(
ShutdownHandlerDef { ... })` from the game's `worker()`.
Interleave at order `50` (pre-framework) or `500+`
(post-framework).

## Discovery + browsers

`ueforge::discovery` walks every live `UDataTable`, `UClass`,
`UScriptStruct` at load and caches metadata (name, super_path,
full_path, fields). Default mode is **eager-slim** (name only)
+ **lazy-deep** (full schema computed on demand via
`describe_data_table` / `describe_class` / `describe_struct`).

ImGui tabs `ui_data_table_browser`, `ui_class_browser`,
`ui_struct_browser` read directly from the discovery cache.
Any consumer mod gets them by adding them to `MOD_INFO.tabs`.

`describe_*` ops accept `name=` to fetch a single entry.
Auto-generated `list_*` ops surface the registry contents.

### Crash hardening (mandatory: do not regress)

The discovery walk visits ~150K UObjects. Several were unsafe
to read; the following hardening landed and must stay:

- **`AppendString` is SEH-wrapped** + the FName index is
  bounds-checked. Bogus FNames no longer crash the host.
- **FFieldClass + Children reads guarded with `VirtualQuery`**
  before deref.
- **`is_a` super-chain capped at 64** + super pointer guarded
  (prevents infinite loop in OWS GObjects walk).
- **FString walk caps total bytes + detects cycles.**
- **`catch_unwind` per object** during discovery iteration:
  one bad object never kills the walk.
- **Op dispatch happens OUTSIDE the registry mutex** so a SEH
  inside a handler does not poison the lock for every future
  op.
- **`describe_data_table` is eager-slim**: returns just the
  name + super_path. The schema walk is on-demand; this was a
  fix for an OWS crash at object 27152 during eager-deep walk.

### `NamedFieldTweak` / `ClassNamedFieldTweak`

Prefer these to hand-pasted offset constants when the field
name is stable. Both resolve the offset from the discovery
cache by FProperty name and write through the same
captured-vanilla + multiplier shape as `FieldTweak<T>` /
`ClassFieldTweak<T>`.

## Status effects (the universal UE5 pattern)

UE5 games routinely route every gear bonus / perk / food buff
/ debuff through a `UStatusEffectComponent` reading a single
master data table (in Grounded 2:
`/Game/Blueprints/Attacks/Table_StatusEffects.Table_StatusEffects`).
`UStatusEffect` is **row-driven**: the value lives in the
data-table row, not the instance.

ueforge ships:
- `StatusDef { id, table_finder, row_fname, value_offset,
   vanilla: AtomicU32 (f32 bits) }`: row identity decoupled
   from the operation.
- `StatusRegistry`: the two-static pattern.
- `StatusEffectApply` Effect: mutates the row Value +
  invokes `CreateAndAddEffect` via PE call.
- Future: `StatusEffectClear`, `StatusEffectMutate`.

Multiple Effects can target the same StatusDef. The game-side
table is still the runtime authority for `Type` / cooldown /
etc.; StatusDef captures only the bits we mutate.

`EStatusEffectValueType` distinguishes `mul` (vanilla 1.0,
contribution = `1 ± bonus`) and `add` (vanilla 0.0,
contribution = scaled bonus). See `ueforge/docs/status-effects.md`.

## Hooks (`ProcessEventHook`)

Vtable-patch a UE5 class's `ProcessEvent`. The trampoline:
- Snapshots the live handler via `arc_swap` (no lock on hot
  path).
- `catch_unwind` the handler so a Rust panic doesn't poison
  the engine.
- Calls the original `ProcessEvent` after.

`hook::install_many` + `hook::install_with_backoff` +
`LazyFunctionPtr` are the canonical helpers. `HookRegistry`
holds the owned `ProcessEventHook` handles + snapshot
accessors + `shutdown_all` for hot reload.

Closure-populated runtime registry (no compile-time `static
HOOK_REGISTRY = ...` const). This is the documented carve-out.

## PE_QUEUE / DrainSite / EventRing

`pe_queue::Queue` is the canonical game-thread work queue with
bounded depth + cancel flag + `DRAINING` re-entrance guard.
Drain it from inside a game-thread trampoline (`kill_hook`'s
multicast fire is the reference site).

`DrainSite` wraps a `Queue` + counters + `time_ns` in one
static. Use it instead of three loose statics.

`EventRing<T>` is the bounded drop-oldest ring for diagnostic
event streams (samplers, traces, kill log).

## Ops + selectors + shutdown registries

All three are closure-populated singleton registries with a
`def(key)` lookup, an auto-generated `list_*` op for client
discovery, and `register_builtins()` registration at worker
init:

- `OP_REGISTRY`: `OpDef { name, summary, args, handler }`.
  Replaced three match dispatchers (`handle_builtin`,
  `dispatch_standard_op`, `dispatch_pe_ops`). New op = one
  `OP_REGISTRY.register(OpDef::new(...))` line. Game-side ops
  register from their `debug.rs`.
- `SELECTOR_REGISTRY`: `SelectorDef { prefix, summary,
  resolver: fn }`. Framework ships `addr:`, `class:`,
  `first_class:`, `singleton:`. Game crates extend without
  touching framework code.
- `SHUTDOWN_REGISTRY`: `ShutdownHandlerDef { name, order,
  run: fn() }`. See the lifecycle section.

## `cargo deploy`

The deploy binary is `ueforge`'s `[[bin]]` target (no separate
`ueforge-deploy` crate). Cargo alias is in
`.cargo/config.toml`.

```sh
cargo deploy install   -p <mod>        # dev iteration
cargo deploy package   -p <mod>        # build zip in dist/
cargo deploy uninstall -p <mod>
cargo deploy install   -p <mod> --skip-build
cargo deploy install   -p <mod> --game-path '<path>'
```

GamePath autodetect walks every Steam library for a dir
matching `[package.metadata.ueforge].game_name_regex`
containing `game_sub_path`. Per-mod `target_dir` keeps two
cdylibs from colliding on `target/release/main.dll`.

## Hardening doctrine (kovarex review outcomes: do not regress)

These landed across the kovarex P0/P1 waves; new code must
respect them:

- `parking_lot::Mutex` everywhere (faster + poison-free).
- Dev profile uses `panic=unwind` so a stray panic in a worker
  leaves a backtrace. Release is `panic=abort` for size/perf.
- Hot paths use `try_runtime` + soft fallback (no allocs on
  miss).
- Address-validated, selector-recoverable freeze ops.
- `arc_swap` for `DisabledSkills`: cheap clone, no Mutex on
  read.
- `SlotPoller` returns a `PollerHandle` with stop flag + panic
  counter + last_panic + named thread: no orphaned threads.
- `SlotStore::save -> io::Result`, fsync temp before rename,
  slot-path validation, `last_error` surface.
- `SkillsState` has a `schema_version` field and **non-pub**
  `spend` / `refund`; mutation only through `Tracker`.
- `Curve` upper guard against absurd XP values.
- Workspace lint `clippy::undocumented_unsafe_blocks = "warn"`
 : every new `unsafe { ... }` needs a `// SAFETY:` comment.

## Performance principle: zero allocations on hot paths

Hot-path code MUST:
- Not allocate `String` / `Vec` / `Box` / formatted strings.
- Not lock mutexes unless work is actually about to happen.
  Use `AtomicUsize` shadows for empty-check fast paths.
- Resolve UE objects by name ONCE, cache pointer in
  `AtomicUsize`, pointer-compare per fire.
- Bail early via atomic-load-and-branch.

Hot paths in any consumer mod: every `ProcessEventHook`
trampoline; every ImGui tab render (per frame, UE4SS render
thread); every PE_QUEUE drain.

ueforge primitives that maintain this:
`find_class_fast` (name-cached), `NameResolver::to_string`
(FName u64 -> String cache), `UClass::cached_native_properties`
(`Arc<[NativeProperty]>`), `EventRing<T>`, `DrainSite`,
`LazyFunctionPtr` + pointer-identity dispatch via cached
`&UFunction`.

## UE5 layout facts (workspace-wide)

These hold across UE5 mod targets in this workspace, modulo
per-game offsets that game crates own:

- **UObject**: vtable@0, flags@8, index@0xC, class@0x10,
  name@0x18, outer@0x20, size=0x28.
- **UClass**: ClassDefaultObject@0x110, ChildProperties@+0x50.
- **UFunction**: FunctionFlags@0xB0.
- **FField**: Next@+0x20, NamePrivate@+0x28.
- **FProperty**: ElementSize@+0x34, Offset_Internal@+0x4C.
- **TMap stride is 24, NOT 16** (TSetElement adds HashNextId +
  HashIndex; TSparseArray slot is union of element + free-list
  link sized to max=24). Walks must honor the TSparseArray
  bitarray, not just stride. This bit OWS first; fixing it is
  what made Table_StatusEffects walk complete in Grounded 2.

Per-game offsets (image-relative `GObjects`, `AppendString`,
`ProcessEventIdx`, class field layouts) live in the game
crate's `ue/offsets.rs`, not here.

## Where things live (index)

Public repo: [`abix-/Grounded2Mods`](https://github.com/abix-/Grounded2Mods). The framework crate is
`ueforge/` at the repo root. All paths below are relative to the
crate root unless noted.

### Documentation (`ueforge/docs/`): read these first

| File                 | Authoritative on                                      |
| -------------------- | ----------------------------------------------------- |
| `architecture.md`    | Composition model + k8s pattern + compliance scorecard. **THE contract.** |
| `PERFORMANCE.md`     | Hot-path discipline doctrine                          |
| `RESEARCH.md`        | TDD investigation methodology                         |
| `lifecycle.md`       | UE4SS load/unload + shutdown registry order + side-file pattern |
| `ue-sdk.md`          | UObject/UClass/UFunction/FName/TArray/TMap/GObjects   |
| `hooks.md`           | `ProcessEventHook` + vtable patch + `install_many`    |
| `pe-queue.md`        | `Queue` + `DrainSite` + drain-in-trampoline canonical sites |
| `counters.md`        | `AtomicU64` counter macros                            |
| `rpg.md`             | Skill/Effect/Trigger composition (the framework module) |
| `http.md`            | HTTP control plane (`tiny_http`, `OpResponse`, `parse_request`) |
| `imgui.md`           | ImGui bindings + safe Rust wrappers                   |
| `settings.md`        | `Settings<T>` JSON store with `watch` hot-reload      |
| `worker.md`          | Worker thread spawn + name + panic hooks              |
| `data-table.md`      | `DataTableDef` + `FieldTweak<T>` + `NamedFieldTweak`  |
| `memory-tools.md`    | scanner, freeze, inspect_address                      |
| `status-effects.md`  | UE5 row-driven status effect pattern + `StatusDef`    |
| `ue-engine.md`       | UE5 process layout, pak, GObjects, shipping caveats   |
| `native.md`          | C++ surface inventory + doctrine                      |
| `uasset.md`          | `.uasset` parser library + CLI                        |
| `mod-formats.md`     | Pak vs DLL vs UE4SS comparison                        |
| `inspection.md`      | UE inspection methodology                             |
| `rust-port.md`       | The original port history                             |
| `ue4ss-port.md`      | UE4SS-specific port notes                             |

### Source (`ueforge/src/`)

| Path                                | Subject                                              |
| ----------------------------------- | ---------------------------------------------------- |
| `lib.rs`                            | Public surface; module declarations                  |
| `mod_main.rs`                       | `ModDef` + `TabDef` + `ue4ss_mod!` macro             |
| `build.rs` + `bin/`                 | `CppShim::compile()` builder; `cargo deploy` CLI lives in `bin/` |
| `log.rs`                            | File + console logger; `dll_dir()`; `set_dll_module` |
| `settings.rs`                       | `Settings<T>` + `watch` hot-reload                   |
| `counters.rs` + `ring.rs`           | `AtomicU64` counters + `EventRing<T>`                |
| `args.rs` + `envelope.rs` + `server.rs` | HTTP op surface                                  |
| `pe_queue.rs`                       | `Queue` + `DrainSite` + `DRAINING` guard             |
| `selector.rs`                       | `SelectorDef` + `SELECTOR_REGISTRY` + `resolve`      |
| `ops.rs`                            | `OpDef` + `OP_REGISTRY` + `dispatch`                 |
| `shutdown.rs`                       | `ShutdownHandlerDef` + `SHUTDOWN_REGISTRY`           |
| `scanner.rs` + `ui_scanner.rs`      | Cheat-Engine-style memory scanner                    |
| `hot_reload.rs`                     | Background watcher synthesizes Ctrl+R                |
| `hook/process_event.rs` + `hook/vtable.rs` | `ProcessEventHook` + VirtualProtect-flipping write |
| `ue/offsets.rs`                     | `PlatformOffsets` + UObject/UClass/UFunction/FField/FProperty offsets |
| `ue/platform.rs`                    | `host_image_base` / `host_exe_name` / `detect()`     |
| `ue/uobject.rs`                     | UObject/UClass/UFunction/GObjectsView/Runtime + caches |
| `ue/fname.rs` + `ue/fstring.rs`     | FName resolver + FString walker (SEH-wrapped)        |
| `ue/tarray.rs` + `ue/tmap.rs`       | Generic walkers (TMap stride=24)                     |
| `ue/core_types.rs`                  | FGuid + FWeakObjectPtr + FDataTableRowHandle + EStatusEffectValueType |
| `ue/probe.rs`                       | gobjects_population + class_outer_samples            |
| `discovery.rs`                      | UDataTable/UClass/UScriptStruct walk + describe_* lazy + crash hardening |
| `data_table.rs`                     | `DataTableDef` + `FieldTweak<T>` + `NamedFieldTweak` + `ClassNamedFieldTweak` |
| `dynamic_tweaks.rs`                 | `apply_all_when_ready` per-table on_first_sight workers |
| `stacks.rs` + `difficulty.rs`       | Framework modules (data-table + CDO tweaks)          |
| `damage/`                           | Framework module: `DamageHook` + `DamageBinder`      |
| `inventory/`                        | Framework module: `viewport` (paging + scroll + rebind) |
| `rpg/`                              | Framework module: see breakdown below                |
| `uasset.rs`                         | `.uasset` lib + dump-strings/read-property CLIs (replaces old Python scripts) |
| `ui.rs`                             | Safe Rust ImGui wrappers (text/button/slider/Disabled RAII/...) |
| `ui_data_table_browser.rs`, `ui_class_browser.rs`, `ui_struct_browser.rs` | Tabs reading the discovery cache |
| `ui_dynamic_tweaks.rs`              | Tab for dynamic_tweaks                               |
| `client/`                           | Test-side `Api<S>`, scenario DSL, diff helpers, research helpers, perf/timeseries/thread/cdo |
| `winproc.rs`                        | threads / cpu / regions / module sampler             |
| `parms.rs`                          | `as_bytes`/`from_bytes` for `#[repr(C)]` parm round-trip |

### `ueforge/src/rpg/` breakdown

| File              | Owns                                                  |
| ----------------- | ----------------------------------------------------- |
| `mod.rs`          | Public re-exports                                     |
| `skill.rs`        | `SkillDef` + `SkillRegistry`                          |
| `effect.rs`       | `EffectDef` + `Effect` trait + standard Effect types  |
| `trigger.rs`      | `TriggerDef` + `Trigger` trait + `TriggerCtx` + `ON_SLOT_CHANGE` |
| `status.rs`       | `StatusDef` + `StatusRegistry`                        |
| `state.rs`        | `SkillsState` (xp/level/skill_points/skill_levels + schema_version) |
| `store.rs`        | `SlotStore` (atomic save, fsync temp, slot-path validation) |
| `poller.rs`       | `SlotPoller` + `PollerHandle` (stop flag + panic counter) |
| `slot_key.rs`     | `SlotKeyResolver` (class_name + guid_offset → filename) |
| `tracker.rs`      | `Tracker<A>` + `XpResult`                             |
| `disabled.rs`     | `DisabledSkills` (arc_swap)                           |
| `vanilla.rs`      | `VanillaCache`                                        |
| `xp.rs`           | `Curve` + `CreatureDef` + `CreatureRegistry`          |
| `progress.rs`     | `sqrt_progress` diminishing-returns helper            |
| `format.rs`       | `PercentFormat`                                       |
| `health.rs`       | `HealthBinding` + `register()`                        |
| `std_effect.rs`   | `StandardEffect` variant menu                         |
| `tab.rs`          | Framework ImGui RPG tab (consumers wire it via `MOD_INFO.tabs`) |
| `ops.rs`          | `register()`: adds skill_toggle/spend/refund/etc to OP_REGISTRY |

### C++ (`ueforge/cpp/`)

| File                              | Subject                                          |
| --------------------------------- | ------------------------------------------------ |
| `ueforge_shim.cpp`                | Generic UE4SS factory + DllMain + ImGui glue     |
| `ueforge_cppusermodbase.hpp`      | Vetted CppUserModBase mirror                     |
| `ueforge_imgui_bridge.hpp`        | UE4SS ImGui context bridge                       |
| `ueforge_ui.cpp`                  | `extern "C"` wrappers around ImGui calls         |
| `imgui/`                          | Vendored ImGui v1.92.1 (git submodule, matches UE4SS) |

### Pre-built import lib

| File                  | Subject                                        |
| --------------------- | ---------------------------------------------- |
| `ueforge/ue4ss/UE4SS.lib` | Import lib generated from the user's installed `UE4SS.dll` exports |

## Session etiquette

- Read the `rust` + `code` skills before writing code.
- For the HTTP debug endpoint pattern, read the
  `runtime-control-http` skill.
- For per-game specifics (offsets, damage paths, deploy targets),
  read the matching game skill: `grounded2`,
  `outworld-station`, etc.
- ASCII in source, docs, commits. Unicode allowed in terminal
  output.
- Commits on `main`, lowercase concise message, push
  immediately, NO Co-Authored-By trailer.
- **Never run the game yourself**: no GPU, no display. Mark
  unverified work "untested" in docs and stop.
- All meaningful sessions update `.claude/project_state.md` at
  end (git-tracked, no secrets).
- When promoting a pattern: change ueforge first, then migrate
  game crates. Document the lift in `docs/changelog.md`.
