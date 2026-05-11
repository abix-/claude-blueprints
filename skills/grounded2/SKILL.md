---
name: grounded2
description: Modding Grounded 2 (Obsidian survival, UE5 + UE4SS). Authoritative on Grounded 2 game specifics -- exes, image-relative offsets, ASurvivalCharacter/UHealthComponent field layouts, three damage paths (combat / fall / env), Table_StatusEffects, deploy folder, the mod inventory. Mod code lives in `abix-/Grounded2Mods` (the `grounded2-rpg` crate is the current shipped mod). For ueforge framework doctrine (composition model, k8s pattern, hot reload, discovery), read the `ueforge` skill. Not for playing the game.
user-invocable: false
version: "5.0"
updated: "2026-05-11"
---
# Grounded 2 -- modding

Per-game modding skill for **Grounded 2** (Obsidian, UE5 5.x +
UE4SS). Authoritative on what is specific to this game: exes,
image-relative offsets, key class field layouts, damage pipeline,
status-effect table, deploy folder. Framework doctrine (the
`ueforge` crate that this game's mod is built on) is in the
`ueforge` skill -- no overlap.

Repo: `abix-/Grounded2Mods`. Current shipped mod is the
`grounded2-rpg` crate (a Factorio-style RPG / level-up mod);
future Grounded 2 mods land in the same repo as additional crates
or features under the same `grounded2-rpg` mod, per the "one mod
per game" project rule.

## The shipped mod: `grounded2-rpg`

A Factorio-style RPG / level-up mod, loaded by UE4SS as a CPPMod.
Player kills creatures -> earns XP -> levels up -> spends skill
points on a flat catalog of skills (cap level 100,
`sqrt(level/100)` diminishing returns). Inspired by Factorio RPG
System, RimWorld RPG Mod, the War3CS / War3FT line.

The crate is `grounded2-rpg` (renamed from `better-backpack`).
Log file: `grounded2_rpg.log`. UE4SS mod name: `Grounded2RPG`.

## Project rule: ONE mod per game

There is exactly one Grounded 2 mod: `grounded2-rpg`.
Diagnostics, probes, traces, and feature work all live inside
it. Do NOT drop side-channel Lua mods or separate probe DLLs
into the install. Everything routes through Rust, gated behind
cargo features, skill levels, or `cfg!(debug_assertions)`. One
log file, one place to read what's happening.

If you need a transient probe, gate it behind a skill flag
(unlock the probe by leveling the relevant skill) or behind a
debug build, then strip it after diagnosis.

## Crate layout

```
grounded2-rpg/
  Cargo.toml                  # [package.metadata.ueforge] consumed by `cargo deploy`
  build.rs                    # ueforge::build::CppShim::new().compile()
  settings.example.json
  src/
    lib.rs                      # MOD_INFO: ueforge::ModDef + ueforge::ue4ss_mod!()
    counters.rs                 # bbp-specific AtomicU64 statics + snapshot_json
    debug.rs                    # snapshot op + per-op handlers; HTTP listener; registers game ops into OP_REGISTRY
    patch.rs                    # backpack DefaultMaxSize CDO patch (player-only)
    inv_hook.rs                 # WBP_InventoryInterface_C ProcessEvent hook + viewport rebind
    survival.rs                 # hunger/thirst CDO writes
    settings.rs                 # serde structs (loaded via ueforge::settings::Settings<T>)
    parms.rs                    # G2-specific UFunction parm structs (#[repr(C)])
    rpg/
      skills.rs                   # CATALOG: &[SkillDef] -- the single source of truth
      effects.rs                  # game-specific Effect impls (BackpackSlots, SurvivalDrain, fall composite)
      apply.rs                    # legacy per-skill helpers (Effects now self-apply via trait)
      tracker.rs                  # spend / refund / record_kill / debug grants
      save_slot.rs                # AInGameGameState.PlaythroughGuid resolver (+0x32C)
      world_loader.rs             # 1Hz poller, drives slot transitions
      kill_hook.rs                # HealthComponent ProcessEvent hook + drain pe_queue
      fall_hook.rs                # OnLanded velocity-stomp + sfx-list/probe diagnostics
      xp.rs                       # 100*N^1.8 curve + per-creature lookup
      tab.rs                      # RPG ImGui tab (calls tracker/skills/xp directly)
      mod.rs
  tests/                        # integration tests vs the running HTTP debug endpoint (uses ueforge::client::scenario)
  docs/                         # READ FIRST when investigating
    README.md                       # index + "where to look first" matrix
    rpg.md                          # catalog, math, persistence, code map
    damage.md                       # Grounded 2 damage internals -- MANDATORY reading
    inventory.md                    # backpack patch + viewport rebind
    engine.md                       # Grounded 2 platform: pak, exes, GObjects, shipping caveats
    building.md, features.md, performance.md, ongoing.md, testing.md
```

## ImGui tabs

```rust
tabs: &[
    ueforge::TabDef { name: "RPG",     render: rpg::tab::render },
    ueforge::TabDef { name: "Tables",  render: ueforge::ui_data_table_browser::render },
    ueforge::TabDef { name: "Classes", render: ueforge::ui_class_browser::render },
    ueforge::TabDef { name: "Structs", render: ueforge::ui_struct_browser::render },
],
```

The browsers come from ueforge; only the RPG tab is local.
Buttons use ImGui `##<skill_id>` label suffix for unique IDs
per row (no PushID/PopID).

## Engine facts (Grounded 2)

Tested against Steam build `++Augusta+release-0.4.0.2-CL-2673661`.

- **Game exes**:
  - Steam: `Grounded2-WinGRTS-Shipping.exe`
  - Xbox:  `Grounded2-WinGDK-Shipping.exe`
- **Steam image-relative offsets**: GObjects=0x09F67028,
  AppendString=0x01252060, ProcessEventIdx=0x4C.
- **Xbox offsets**: GObjects=0x09F36F28, AppendString=0x01250F80,
  ProcessEventIdx=0x4C.
- **ASurvivalCharacter** (player pawn class):
  - HealthComponent       @+0x1340
  - StatusEffectComponent @+0x1378
  - CharMovementComponent @+0x1380
  - CustomDamageMultiplier@+0x12B8
  - bTakeFallDamage       @+0x1571
  - MinimumFallDamageVelocity @+0x1574
  - FallDamageRatio       @+0x157C
- **UHealthComponent**:
  - BaseDamageReduction       @+0xEC
  - RequiredDamageTypeFlags   @+0xFC
  - MaxHealth                 @+0x328
- **AInGameGameState**: PlaythroughGuid @+0x32C (stable across
  save renames -- canonical slot key).
- **USurvivalGameModeSettings.FallDamageMultiplier** @+0x008C.
- **USurvivalModeManagerComponent.CustomSettings** @+0x0114
  (FCustomGameModeSettings struct, 0x20 bytes;
  FallDamageMultiplier at +0x1C within it -> SMMC offset 0x0130).
- **SurvivalComponent**:
  - HungerSettings.AdjustmentPerSecond @+0x0140
  - ThirstSettings.AdjustmentPerSecond @+0x0188

Layout primitives (workspace-wide, restated here for grep):
UObject vtable@0, flags@8, index@0xC, class@0x10, name@0x18,
outer@0x20, size=0x28. **TMap stride is 24 not 16** -- see
ueforge skill for the rationale.

## Damage pipeline (load-bearing)

Grounded 2 has three distinct damage paths. **Read
`grounded2-rpg/docs/damage.md` before touching anything
damage-related.** It captures every approach that did NOT work
and why.

1. **Combat damage** (creature hits) -- `ApplyDamage` with
   non-zero `type_flags`. Routes through
   `MulticastHandleEffectsWithDamageFlags`. This is the path
   the status-effect system (DamageReduction, AttackDamage)
   already covers.
2. **Fall damage** -- separate native `ApplyFallDamage` reading
   `CharMovementComponent.Velocity.Z`. Mitigation: velocity-
   stomp in `OnLanded` (PE hook in `fall_hook.rs`) -- scale
   `CMC.Velocity.Z` by `(1 - reduction)` before native
   `ApplyFallDamage` runs. Plus writes to GMS / SMMC fields +
   `UpdateCustomSettings` UFunction call to refresh the
   native cache.
3. **Environmental / hazard / impact damage** -- also through
   `ApplyDamage` but with `type_flags = 0`. Currently
   mitigated by writing `RequiredDamageTypeFlags = 0xFFFFFFFF`
   to player `UHealthComponent`; the native gate rejects
   type_flags=0 hits. This is **binary** (level 1 = level 100);
   the catalog row uses a `RuntimeEffect` until the status-
   effect migration lands.

### Status effects (the migration target)

Every gear bonus / perk / food buff / debuff in Grounded 2
flows through `UStatusEffectComponent` at `+0x1378` on the
player. The native damage code calls
`GetValueForStatForDamageTypeFlags(StatType, Flags)` and uses
the float as a multiplier. Every skill we plan to add has a
matching `EStatusEffectType`.

Every status effect flows through ONE data table:
**`/Game/Blueprints/Attacks/Table_StatusEffects.Table_StatusEffects`**.
`UStatusEffect` is row-driven -- the value lives in the row,
not the instance. Migration plan: resolve the table, pick a
row, mutate `Value` (and add a row if a new effect), call
`AddEffect` via process_event. Long-term backing for nearly
every damage skill.

`EStatusEffectValueType` per stat (Grounded 2 specifics):
- `mul` (vanilla 1.0): `FallDamage`, `DamageReduction`,
  `AttackDamage`. Contribution = `(1 ± bonus)`.
- `add` (vanilla 0.0): `LifeSteal`, `CriticalHitChance`,
  `CriticalDamage`, `ReflectDamage`, `MaxHealth`,
  `DamageReductionMultiplier`. Contribution = scaled bonus.

## Skill catalog

`grounded2-rpg/src/rpg/skills.rs::CATALOG_ENTRIES` is a
`&[SkillDef]` (the framework's k8s Def). Each row pairs id +
display name + max level with an `EffectDef` referencing a
`static <Effect>Effect` instance. **No central match arm to
update.** Adding a new skill of an existing shape = one
`static FOO: <SomeEffect> = ...` + one CATALOG row.

Universal scaling lives in `ueforge::rpg::progress::sqrt_progress`:
`level_progress(level) = sqrt(level / 100)`. Final value =
`max_bonus * level_progress(N)`. All Grounded 2 skills cap at
level 100.

Current Effect instances used by the catalog (see
`rpg/effects.rs` for game-specific ones):

| Effect type                    | Skills using it                                        |
| ------------------------------ | ------------------------------------------------------ |
| `BackpackSlots` (game-specific)| backpack                                              |
| `SurvivalDrain` (game-specific)| hunger, thirst                                        |
| `PlayerFloatEffect`            | attack_damage (writes +0x12B8 on ASurvivalCharacter)  |
| `SubcomponentFloatEffect`      | armor                                                  |
| `SubcomponentMultiplyEffect`   | move_speed, jump_height, leap_distance, glide_speed   |
| `SubcomponentAdditiveEffect`   | max_health, health_regen                              |
| `SubcomponentU32MaskEffect`    | (planned) impact_resistance once on status-effect path |
| `ClassFieldsMultiplyEffect`    | (planned) crowd-effect skills                         |
| `RuntimeEffect`                | lifesteal, impact_resistance (today)                  |
| `StatusEffectApply`            | (migration target -- not wired into Grounded 2 catalog yet)|

Game-specific Effect impls live in
`grounded2-rpg/src/rpg/effects.rs`. If a new Effect shape would
apply to another UE5 game, promote it to `ueforge` first.

## Persistence

`SkillsState` (from `ueforge::rpg::state`) lives in
`<DLL_dir>/saves/<playthrough-guid>.json` (via
`SlotStore`). GUID resolved by
`ueforge::rpg::SlotKeyResolver` reading
`AInGameGameState.PlaythroughGuid` at `+0x32C` -- stable across
save renames.

Schema is open (`#[serde(default)]`) + has a `schema_version`
field. `SlotPoller` (1Hz worker) drives slot
activate/deactivate transitions and runs `apply` on activate.

The DLL dir resolves via `ueforge::log::dll_dir()` reading the
HMODULE captured by ueforge's macro-emitted DllMain.
grounded2-rpg does NOT own a DllMain.

## Deployed mod location

After `cargo deploy install -p grounded2-rpg`, the DLL lands in
the user's Grounded 2 install under
`<game-root>/Augusta/Binaries/WinGRTS/ue4ss/Mods/Grounded2RPG/dlls/`
(Steam) or the equivalent `WinGDK` path on Xbox. `cargo deploy`
autodetects `<game-root>` by walking every Steam library for a
directory matching the crate's `[package.metadata.ueforge]`
`game_name_regex`.

Files in `dlls/`:

| File                  | Role                                                  |
| --------------------- | ----------------------------------------------------- |
| `main.dll`            | Built mod (cdylib)                                    |
| `main-new.dll`        | Transient: present between deploy and hot reload (see `ueforge` skill) |
| `main-old.dll`        | Transient: cleaned at next init                       |
| `grounded2_rpg.log`   | Mod log -- `ueforge::log` writes here, per-line flush |
| `settings.json`       | Live settings the mod reads at load                   |
| `saves/`              | Per-playthrough JSON: `<playthrough-guid>.json`       |

When investigating in-game behavior, **first place to look is
`grounded2_rpg.log`**. The mod logs every apply step, spend /
refund, kill credit, slot load, and skill toggle there.

`mods.txt` line `Grounded2RPG : 1` registers the mod with UE4SS
-- the deploy CLI handles it. For `cargo deploy` mechanics see
the `ueforge` skill.

## Debug HTTP endpoint (Grounded 2 ops)

Opt-in via `settings.json`:
```json
{ "debug": { "http_port": 17171 } }
```

`POST 127.0.0.1:<port>/debug` body `{"op": "<name>",
"args": {...}}` returns `{ok, op, error, result, state}` where
`state` is the full snapshot (skill levels, toggle flags,
vanilla baselines, live HealthComponent field reads).

Game-side ops registered into ueforge's `OP_REGISTRY` from
`grounded2-rpg/src/debug.rs`: `snapshot`, `skill_toggle`,
`skill_spend`, `skill_refund`, `simulate_*`,
`set_skill_points`, `call`, plus the kill / damage / fall
diagnostics gated on debug builds. Built-in ops
(`read_bytes`, `write_bytes`, `walk_class`, `inspect_address`,
`scan_*`, `freeze*`, `discover_*`, etc) come from ueforge.

Integration tests in `grounded2-rpg/tests/`. Each `.rs` is one
binary; shared client + types via `ueforge::client::Api`,
Pester-style assertions via `ueforge::client::scenario`.
Run pattern:

```bash
set BBP_DEBUG_PORT=17171
cargo test --test debug_snapshot -- --test-threads=1 --nocapture
```

Tests skip cleanly when `BBP_DEBUG_PORT` is unset.
**Always pass `--test-threads=1`** -- the tests share global
game state.

`grounded2-rpg/docs/testing.md` is authoritative on per-test
expectations.

## Concurrency / threading

- The 1Hz `SlotPoller` runs on a named Win32 worker thread
  (handle exposes stop flag + panic counter + last_panic).
  Safe for read-only GObjects walks.
- The ImGui tab callback (`rpg::tab::render`) runs on UE4SS's
  render-loop thread. Heavy GObjects walks + native UFunction
  calls from there are risky -- queue work to `bbp::debug::PE_QUEUE`
  and drain from `kill_hook` instead.
- `process_event` calls inside `inv_hook` run from the
  inventory PE trampoline (already on the game thread when
  called from BP events) and are safe.
- `kill_hook` drains `bbp::debug::PE_QUEUE` on every multicast
  fire (game thread). Read-only reads are also safe
  off-thread, but writes must go through the queue.

## In-flight work (as of 2026-05-11)

- **Status-effect-backed skill rewrite.** Migrate Impact Damage
  Resistance and Lifesteal first; rest of catalog follows. See
  `docs/todo.md` "RPG: Status-effect-backed skill rewrite".
- **Live-instance writes** for remaining combat skills (movement
  already does this).
- **pkg(0) instigator bug** -- some legitimate player kills
  attribute to `/Script/CoreUObject (Package)`, losing XP.
- **Catalog expansion** (Crit, Evasion, Stamina, Gear Hardiness,
  Thorns, Climb Speed) -- target ~25 skills.
- **Distribution polish.** Vortex / Nexus packaging refinements.

## Where to look when X is broken

| Symptom                          | First file to read                                  |
| -------------------------------- | --------------------------------------------------- |
| skill not applying in-game       | deployed `grounded2_rpg.log` (apply lines)          |
| skill state at-this-instant      | `POST /debug {op:snapshot}` -> `state` JSON         |
| ImGui tab missing / row off      | `grounded2_rpg.log` + `rpg::tab`                    |
| save not loading                 | `grounded2_rpg.log` (world_loader / slot lines)     |
| XP not awarded                   | `grounded2_rpg.log` (kill_hook, instigator)         |
| crash / hard hang                | `grounded2_rpg.log` last lines + ueforge log line   |
| build fails                      | cargo output, `ueforge/ue4ss/UE4SS.lib` exists?     |
| deploy fails                     | `cargo deploy install -p grounded2-rpg` output      |
| browser tab empty                | discovery cache not built? check init logs          |
| hot reload didn't fire           | game window focus + `hot_reload:` watcher log line  |
| damage skill behaves binary      | `docs/damage.md` -- type_flags gate or wrong path?  |

## Session etiquette

- Read the `rust` + `code` skills before writing code.
- Read the `ueforge` skill for the framework doctrine.
- For the HTTP debug endpoint pattern, read the
  `runtime-control-http` skill.
- For damage / fall / environmental work, you MUST read
  `grounded2-rpg/docs/damage.md` before changing anything.
- Each crate's `docs/` is authoritative on its subject; read
  the right one first instead of rediscovering.
- All meaningful sessions update `.claude/project_state.md` at
  end (git-tracked, no secrets).
- Commits on `main`, lowercase concise message, **push
  immediately**, NO Co-Authored-By trailer.
- ASCII in source/docs/commits. Unicode allowed in terminal
  output.
- **Never run the game yourself** -- no GPU, no display. In-game
  witness is the user's job. When work needs validation, mark
  it "untested" in docs and stop.
