---
name: horsey
description: Modding Horsey Game (native PE Windows, no managed runtime). Authoritative on the horsey-mod architecture, address-resolution discipline (TargetRegistry + patternsleuth), content-authoring loop, where horses live in memory (HORSE-PLACES catalog), the modforge primitives horsey-mod consumes (vanilla.invoke, input axis, hook+seh, testkit, research), and the launch/inject loop. The mod lives in [`abix-/modforge`](https://github.com/abix-/modforge) under `horsey-mod`. Not for playing the game.
---
# Horsey: modding

Per-game modding skill for **Horsey Game** (Steam appid 3602570, native PE, no managed runtime, no plugin loader). horsey-mod ships as a proxy `steam_api64.dll` (build script forwards every export to the renamed real DLL) and is also injectable via `horsey-inject.exe` (`CreateRemoteThread` + `LoadLibraryW`).

Repo: [`abix-/modforge`](https://github.com/abix-/modforge) under `horsey-mod`. modforge supplies the cross-game framework (HTTP server, op registry, harness/testkit, patternsleuth + sleuth wrapper, hudhook overlay, vanilla-invoke, input axis, seh+hook detours, research helpers); horsey-mod is the per-game binding.

Authoritative docs (READ FIRST when investigating):
- `horsey-mod/docs/HORSE-PLACES.md`. Every memory location that holds horse data, with decomp line citations.
- `horsey-mod/docs/todo.md`. Live working state, recent findings, current P0s.
- `horsey-mod/docs/ADDRESS-RESOLUTION.md`. R1-R5 plan + migration table + R3/R4 validation primitives.
- `horsey-mod/docs/SAVE-FORMAT.md`. Save pipeline + sidecar format.
- `horsey-mod/docs/HOOKING-STRATEGY.md` sections 8/9. D1/D3/D5 detour implementation status + full implementation log.
- `horsey-mod/docs/HK1-SHIFT-CLICK-TRANSFER-PLAN.md`. Shift-click transfer plan (now blocked on synthetic input).
- `horsey-mod/docs/input-surface.md`. L3 InputSurface design (decomp pass, surfaces, plan).
- `horsey-mod/docs/world-map-detection.md`. Two-layer world-map context (gamestate `active_scene_id` for "in map vs in location"; pHash sliding window over auto-collected icon library for "which location at cursor"). Tests-first plan.
- `modforge/docs/screen-axis.md`. Cross-game design for the third I/O axis: capture (L1 windows-capture / L2 hudhook present-hook readback / L3 per-game surface) + recognition (groundtruth annotation + vision/OCR/template-match) + Set-of-Mark output. Parallel to the input axis.
- `horsey-mod/docs/CHANGELOG.md`. Session log; commit-by-commit history.
- `horsey-mod/docs/PRIOR-ART-HorseyLiveTweaks.md`. HLT cross-validation.

## Project rule: ONE mod per game

Exactly one Horsey mod: `horsey-mod`. Diagnostics, probes, content extensions, hotkeys, bestiary, all live inside it. NO side-channel probe DLLs or external tools that touch the game; everything routes through the HTTP control plane and the mod's op registry. One log file (`horsey.log`), one place to read state.

## Launch / iteration loop

**EVERY TEST = FRESH LAUNCH. NO EXCEPTIONS.** The canonical test cycle is:

1. Kill any running Horsey.exe
2. Steam-launch appid 3602570 (Steam auto-loads the most recent save into the truck on the world map)
3. Inject `horsey.dll` via `horsey-inject.exe --fresh`
4. Wait for HTTP plane on 33077
5. Run the actual test logic
6. Teardown (taskkill the game)

This is exactly what `common::launch()` does when called with NO env vars. **DO NOT use `MODFORGE_ATTACH=1`.** Attach mode is for interactive probes only (capturing coords by hand while the user positions the cursor); production tests must always relaunch.

Reason: tests run against KNOWN-FRESH state. Attached runs inherit whatever state the user / previous test left behind (truck moved, scene changed, horses mid-drag, etc.) and silently produce wrong answers. The user has caught this repeatedly. Always relaunch.

**Codex drives every test.** The user is for visual confirmation only. Asking "is your save loaded" is a banned question; if the test needs a horse, gate on `wait_for_target_horse`. If it needs the Home scene, drive the house-click via synthetic input as part of the test's own setup.

Never inject `horsey-inject.exe` ad hoc into a game the user already launched; that stacks duplicate overlays and breaks state. Always go through `horsey-play` (for free-play sessions) or `common::launch()` (for tests).

**The easy button (for free-play sessions, NOT tests):**
```
k3sc cargo-lock run -p horsey-mod --bin horsey-play --release
```
Builds release, kills any running Horsey, Steam-launches, injects, `mem::forget`s the handle so the game stays up.

**Test invocation pattern:**
```
k3sc cargo-lock test -p horsey-mod --test <name> --release -- --test-threads=1 --nocapture
```
No `MODFORGE_ATTACH`. No `MODFORGE_SKIP_BUILD`. Let the harness do its job.

Env knobs (use sparingly):
- `MODFORGE_SKIP_BUILD=1` skip cargo build (use only in tight iteration loops where you JUST rebuilt)
- `MODFORGE_NO_GAME=1` no-op for CI
- `MODFORGE_EXPECT_LOADED=1` test gates the bounded-value contract on a loaded save
- `MODFORGE_ATTACH=1` BANNED for production tests. Interactive coord capture only.

**Hot-reload is NOT used.** `--reload` is crash-prone; canonical loop is full relaunch.

**Single cargo at a time.** Use `k3sc cargo-lock`. Tests run with `--test-threads=1` because there's a single game process.

## Address resolution discipline

**RULE 1: Every game address goes through patternsleuth + a sanity gate.** No hand-rolled byte scanners. `modforge::patterns::sleuth` handles literal patterns, wildcards (`??`), capture groups, and `X<target>` xref constraints, SIMD-accelerated. Companion `scan_all_matches` returns every match (vs `resolve_all`'s first-per-name).

**RULE 2: NEVER use the hardcoded RVA as the resolver's sanity check.** Bit us hard 2026-05-16: gate rejected the CORRECT new-build address because it differed from the stale hardcoded by more than 4 KB; silent fall-back to stale slot. Use `is_addr_readable` + alignment + structural validation. A well-anchored resolver should never cross-check with the hardcoded; the hardcoded is the dev's eyeball reference only.

**RULE 3: Probes ship as tests, not curl one-liners.** Every memory probe goes in `horsey-mod/tests/`, calls an existing or newly-added HTTP op, and either asserts a value or dumps it for inspection. The op + test become permanent infrastructure.

**RULE 4: `patterns.read_bytes` must SEH-guard both endpoints via `is_addr_readable`.** A single faulting deref takes down the HTTP worker and disconnects every client.

**RULE 5: Decomp-first when game has the feature.** When the game has an in-game editor for X (CRISPR Lab for genes), READ ITS DECOMP first. CRISPR decomp at `research/decompiled/annotated/0x140089510_crispr_lab_state_machine.c` + `funcs/1400b/1400b39b0_FUN_1400b39b0.c` was ground truth for diploid genome; single pass beat hours of guess-and-check.

## TargetRegistry (B-phase, shipped 2026-05-17)

Every address (data globals, function entries, field offsets) is now declared as a `modforge::patterns::sleuth::TargetDef` in `horsey-mod/src/targets_registry.rs`. The Resolver controller in modforge walks the registry at attach, runs validators per target, caches results. The legacy `targets::resolve::*` module was DELETED in `a35cdbca`; all 24 legacy call sites migrated to the registry. Parity protected by `tests/registry_parity.rs`.

- **41 entries** in the registry (B4 expansion).
- **Recipes**: closed-form decoders (`RipDisp`, `RipDispWithRelOffset`, `PairedRipDispWithDelta`, `CallSiteLookback`, `ImmInWindow`, ...) for the common cases; `Recipe::Custom` for targets whose decoder doesn't fit (NAME_TABLE heap scan, CRISPR chromosome table). New recipes added 2026-05-17: `PairedRipDispWithDelta`, `RipDispWithRelOffset`.
- **B6 cross-game proof**: `grounded2-mod` adopted the same TargetRegistry, confirming the abstraction is not Horsey-specific.

## Resolver state (as of 2026-05-17)

- **Data globals: 6/6 on R.** GAMESTATE_PTR (constructor-1.0f-literal anchor), NO_TIRE_TOGGLE, DEBUG_MODE_ACTIVE, DEBUG_LOG_GATE, RACES_COUNTER, SAVE_VERSION_GLOBAL.
- **Function entries: 31/31 on R.** Including HORSE_CONSTRUCTOR/DESTRUCTOR, GENE_COMBINATOR, SAVE_WRITER, LOAD_GAME, HORSE_SAVE_WRITER/LOADER, APPLY_GENE_TO_HORSE, EVAL_DIPLOID_BLEND_A/B, GENE_DEATH_DRIFT, GENE_ALLELE_SWAP, GENE_TABLE_LOADER/XML_WRITER, POP_XML_LOADER, TMX_MAP_PARSER, CRISPR_LAB, BREEDING, DRAW_PAUSE_STATUS, RETIRE_HORSE_HANDLER, CHECK_HORSE_ELIGIBILITY, COMPUTE_HORSE_PRICE, plus render-trampoline and lifecycle targets.
- **Field offsets: 29/37 on R via R4 toolkit.** All `must` + `should` done. Remaining 9 are `H-gb-low` (diag-only). R4 toolkit lives in `modforge::research` with 4 generic recipes (`decode_field_offset_via_string`, `decode_imm_in_window`, `decode_disp_pair_with_delta`, `decode_imm_at_call_site`) + `find_function_bounds_via_int3`. Per-mod tests are thin env-driven wrappers; full set in `tests/research_*.rs`.
- **NAME_TABLE**: heap-allocated table (moved out of `.data` in current build). Custom resolver scans `.text` for 8 ModR/M variants of `mov r64, [rip+disp32]`, derefs each slot, scores by MSVC `std::string` SSO shape at 0x88 stride.
- **CHROMOSOME_TABLE** (CRISPR `DAT_14030d110`): resolved via patternsleuth; `chromosomes::{chromosome_map, flat_to_chromosome}` cache it. Powers chromosome-strip rendering in the Details panel.
- **Owned-horse chain**: `scene_table[0]` via `GS+0x438` (NOT `active_scene_id*8`; slot 0 is the canonical owned list, holds player's horses even when `active_scene_id = -1` on the overworld).

## Key code map

```
horsey-mod/
  Cargo.toml                # cdylib + rlib + horsey-inject + horsey-play binaries
  src/
    lib.rs                  # DllMain + worker thread; bootstraps log, ops, hudhook overlay, L3 input surface
    bin/inject.rs           # horsey-inject.exe (CreateRemoteThread injector)
    bin/play.rs             # horsey-play (build + launch + inject easy-button)
    targets.rs              # Hardcoded RVAs only (legacy resolve module DELETED)
    targets_registry.rs     # Declarative TargetRegistry (41 entries) consumed by modforge::patterns::sleuth::Resolver
    gamestate.rs            # GameState ptr() + looks_loaded() + owned_horse_count/ptr() + diag() + money/year/sleeps
    horse.rs                # Horse* field accessors, name lookup, diploid genome writes (writes BOTH banks)
    chromosomes.rs          # CRISPR chromosome map (flat idx -> chromosome,position) + dump op
    gene_names.rs           # Vanilla gene names sourced from game's data/genes.xml
    fatigue.rs              # Fatigue / no-tire helpers
    hk1.rs                  # Shift-click transfer machinery (LOC poke + drag stage + cursor calibration)
    input_surface.rs        # HorseyInputSurface (L3): writes LOC cursor floats, registers via modforge::input
    overlay.rs              # hudhook-based in-game ImGui overlay (Present hook); tabs Overview/Horses/Debug/Details
    ops.rs                  # HTTP op registry (game.*, gamestate.*, horse.*, mem.*, patterns.*, genes.ext.*, input.*, hk1.*, chromosomes.*, targets.*)
    snapshot.rs             # HorseyState snapshot returned with every HTTP response
    genes.rs / genes_xml.rs # EXT_HORSE_GENOMES + bestiary XML parser
    patches.rs              # Binary patches (revert-on-detach)
    patches/
      combinator.rs         # Uses modforge::hook::Hook + SEH-guarded vanilla call
      lifecycle.rs          # Horse ctor/dtor detours
      render_trampoline.rs
      save_sidecar.rs       # D4 -- unsafe-to-arm pending re-derive
      ext_genes.rs
  bestiary/
    genes-extended.xml      # Ext genes authoring file
    pop-extended.xml        # Population/spawn weights extension (D2.5/D2.6/D2.7 plumbing in progress)
  research/
    decompiled/             # Ghidra dump (older binary; verify offsets against live game)
    prior-art/HorseyLiveTweaks/
  tests/                    # 82+ integration tests via modforge::testkit + HTTP client
  docs/
```

## In-game overlay (hudhook)

UI is now an **in-game ImGui overlay** via `hudhook` (Present hook on the game's DXGI swap chain). The separate-Windows-window backend (`modforge::ui::native`) was RETIRED. Bootstrap: `overlay::arm()` runs from the worker thread. Tabs: Overview / Horses / Debug / per-horse Details (chromosome-strip layout in CRISPR style; cells editable for the 480-gene grid).

## modforge primitives horsey-mod consumes

These shipped after the original skill snapshot; learn them, they replace bespoke per-mod code:

- **`modforge::patterns::sleuth`**. patternsleuth wrapper. `resolve_all`, `scan_all_matches`, `TargetRegistry`, `Resolver`, `Recipe::{RipDisp, PairedRipDispWithDelta, RipDispWithRelOffset, CallSiteLookback, ImmInWindow, Custom, ...}`, `Validator`. `hint_rva` fallback works for pre-rebased VAs (fix in `befe13ad`).
- **`modforge::vanilla`**. Call into vanilla game functions from outside. `vanilla.invoke` + `vanilla.list` HTTP cmdlets dispatch on registered `Signature` (ArgKind/RetKind). Lets tests / overlay buttons invoke real game functions without bespoke ops. V5 production call sites: `horse.rebuild`, `rng.next_modulo`.
- **`modforge::input`**. Synthetic mouse/keyboard. Three backends: **L1** `SendInput` (OS-level, works for cursor + GetAsyncKeyState path), **L2** `PostMessage` (WndProc; partial. Engine polls `GetAsyncKeyState` so L2 clicks don't register button-held), **L3** `InputSurface` trait (per-mod impl that writes engine state directly, bypasses Win32 pump). Cmdlets: `input.mouse.{move, click, drag, scroll}`, `input.key.*`, `input.combo`. Discovery ops: `input.find_hwnd_by_pid`, `input.self.hwnd`. Backend default: L3 if surface registered, else L2 if hwnd supplied, else L1.
- **`HorseyInputSurface`** (L3 impl in `input_surface.rs`). Writes `LOC+0x174/+0x178` cursor floats directly (resolved by HK1 work). Registered at attach via `input_surface::register()`. Click and key are deferred to v2; v1 ships move only.
- **`modforge::hook::Hook`**. Generic detour wrapper (MinHook-style) with revert-on-drop. `patches/combinator.rs` migrated to it 2026-05-16; +vanilla_crashes stat.
- **`modforge::seh::guard`**. Catches access violations from foreign calls so a crashing vanilla function logs + counts a stat instead of taking the process down. ALWAYS wrap vanilla calls in `seh::guard`.
- **`modforge::testkit`**. Cross-game test harness (relaunch + inject + HTTP + assert + taskkill, with timestamped logs at `target/test-runs/<name>-<ts>.log`). All horsey-mod tests migrated. ~7-20s per harness test.
- **`modforge::research`**. R4 recipe helpers (decode operand, scan in window, decode field offset via string, decode imm at call site, find function bounds via int3, decode disp32 pair). Used by both production resolvers and standalone `tests/research_*.rs`.
- **`modforge::patterns::sleuth::TargetRegistry`**. See B-phase section above.

## HTTP control plane

Listens on `127.0.0.1:33077`, endpoint `/op`. **Auth is DISABLED** (lib.rs comment): localhost-only bind made the per-launch token pure friction; if the port ever goes off-localhost auth needs to come back.

Key ops (see `src/ops.rs` for the full registry):
- Core: `ping`, `list_ops`, `game.read`, `game.build_info` (SHA-256 + size + image base)
- Cheats: `game.money.set/add`, `game.year.set`, `cheats.no_tire.set`, `cheats.debug_mode.set`
- Gamestate: `gamestate.diag` (full field dump + 16KB hex), `gamestate.owned_horses`, `gamestate.scan_438_slots`
- Horse: `horse.read`, `horse.set_age`, `horse.set_max_age`, `horse.clear_tiredness`, `horse.name_diag`, `horse.rebuild`
- Memory: `patterns.scan`, `patterns.read_bytes` (SEH-guarded), `patterns.sleuth.scan_all`, `mem.scan_data`, `mem.scan_rdata`, `mem.scan_heap_string`, `mem.find_xrefs`, `mem.poke`, `mem.alias_check`
- Genes: `genes.ext.get/set`, `genes.ext.save.*`, `genes.ext.dump`
- Targets: `targets.resolve.*`, `targets.resolve.field_offsets`
- Vanilla: `vanilla.invoke`, `vanilla.list`
- Input: `input.mouse.{move, click, drag, scroll}`, `input.key.*`, `input.combo`, `input.find_hwnd_by_pid`, `input.self.hwnd`
- HK1 (shift-click transfer research): `hk1.read_cursor`, `hk1.set_target`, related dump/probe ops
- Chromosomes: `chromosomes.dump`

## HK1 status (shift-click transfer)

**Vtable `[+0x78]` drop-commit is UNSAFE to call.** Crashes the game even with the full 4-field LOC stage (drag_idx + click_state + grabbed + cursor populated). Suspected: `+0x300` sub-struct uninitialized when invoked outside its expected call site. The 3-arg signature (this, drag_idx, param3) was correct; the fault at `+0x31` was uninit `rdx`. Path is currently DISABLED.

**Reopened via synthetic input (modforge::input L3).** Plan: drive shift-click through the real game input path now that the input axis exists, bypass the dangerous vtable call. See `docs/HK1-SHIFT-CLICK-TRANSFER-PLAN.md` and `docs/input-surface.md`.

Findings worth keeping:
- `horse + 0x1d0` = container kind (truck=7, pasture=9). Pokeable via `mem.poke`.
- Scene table slot `0x00` = home/pasture. tomtato lived there.
- `LOC + 0x174 / +0x178` = client-coord cursor floats; L3 surface writes these.
- Engine per-frame input pump (`FUN_14018d160` area) does: Win32 message pump (cap 3/frame), `GetKeyState` modifier reconcile, `GetCursorPos`+`ScreenToClient`+`GetAsyncKeyState(1/2/4/5/6)` mouse polling. L1 SendInput works on both halves; L2 PostMessage only on the WndProc half.

## Known traps (one-liners; details in linked docs)

1. **Roster at GS+0x280 is NOT owned.** It's the all-horse pool (player + NPC + ancestors), 36-byte stride. Use `gamestate::owned_horse_count/_ptr` (scene table slot 0). See HORSE-PLACES.md.
2. **GS+0x130/+0x138 is always zero.** Real per-scene horse vector is on a sub-struct via `*(GS+0x438) + slot*8`. `live_horse_*` accessors are DEPRECATED. See HORSE-PLACES.md.
3. **Horse names are in a heap NAME_TABLE.** `horse+0x1f8` is a u32 name_id into a 0x88-stride table of MSVC `std::string`. See HORSE-PLACES.md "Name table".
4. **MSVC `std::string`: size at +0x10, capacity at +0x18.** Old decomp swapped them. SSO cap=15; `cap>15` means heap str at `*(entry+0x00)`.
5. **THE GAME ALWAYS AUTO-LOADS THE SAVE ON LAUNCH. NO EXCEPTIONS.** Steam launch -> save auto-loads -> player spawns in the truck on the world map. ALWAYS. Every time. Do not ask the user "is the save loaded", do not gate on "wait for save", do not propose a Continue-button click flow. If `gamestate::ptr()` returns 0 after a fresh launch, the problem is NEVER "save didn't load". It is ALWAYS one of: (a) the GAMESTATE_PTR resolver is broken for the current build, (b) the hint_tolerance rejects a valid drifted slot, or (c) a different resolver bug. Diagnose THAT, do not blame the user's save flow. The user has corrected this exact wrong assumption many times. DO NOT use `MODFORGE_ATTACH=1` to dodge a relaunch; relaunch is the contract.
6. **Vanilla genome is DIPLOID.** Always write BOTH banks (`+0x2b8` and `+0x3a8`); render samples the paired bank. CRISPR decomp = ground truth. See `docs/HORSE-PLACES.md` + memory `feedback_decomp_first`.
7. **L2 PostMessage clicks don't register.** Engine polls `GetAsyncKeyState`; favor L1 or L3 for clicks. See `docs/input-surface.md`.
8. **One cargo at a time.** Always `k3sc cargo-lock`. Tests need `--test-threads=1`.

## DLL deployment (proxy `steam_api64.dll`)

horsey-mod ships as the proxy `steam_api64.dll`. The build script enumerates the real DLL's exports and emits a `.DEF` with forwarders so every existing Steam API call passes through transparently. User flow: rename the real DLL to `steam_api64_real.dll`; drop ours in its place. The injector path (`horsey-inject.exe`) is an alternative for live attach (uses `CreateRemoteThread` + `LoadLibraryW`); `horsey-play` automates inject.

## DllMain + worker bootstrap order

DllMain only stashes the module handle (`modforge::log::set_dll_module`) and spawns `worker_main`. NEVER do work in DllMain (loader-lock trap). `worker_main` runs in order:

1. `modforge::log::init` -> `horsey.log` next to the DLL.
2. `install_panic_hook` + `install_seh_logger`.
3. `modforge::log::dll_dir_wait` to resolve the DLL directory.
4. (Auth disabled; was per-launch token written to `horsey.auth`.)
5. `ops::register_all()`. Horsey ops on the modforge global registry.
6. `modforge::vanilla` cmdlets bound to horsey's `TargetRegistry` (`vanilla.invoke` / `vanilla.list`).
7. `input_surface::register()`. L3 surface so `input.*` cmdlets dispatch through `HorseyInputSurface`.
8. `overlay::arm()`. Hudhook in-game ImGui overlay (Present hook).
9. Auto-load `genes-extended.xml` from the DLL dir if present (non-fatal if missing).
10. `patches::sleep_safe_no_tire::apply()`. NOP the `+0x206` zero in the no_tire per-frame loop, then enable `gamestate::set_no_tire(true)`. If the patch fails, leave no_tire OFF so sleep still works.
11. `modforge::server` starts HTTP on `127.0.0.1:33077`.

DllMain `DLL_PROCESS_DETACH` calls `patches::revert_all()` so binary patches roll back on detach.

## Field-offset cheatsheet

Quick-lookup form of the canonical catalog. Full notes (decomp cites, scene-slot inventory across all 256 slots, roster-pool record layout, name-table SSO details): [`docs/HORSE-PLACES.md`](../../code/modforge/horsey-mod/docs/HORSE-PLACES.md).

GameState (resolved via GAMESTATE_PTR -> deref):
- `+0x130 / +0x138`. ALWAYS-ZERO field (DO NOT USE; legacy decomp label).
- `+0x254`. FRAME_TICK (low-confidence diag).
- `+0x25C`. `active_scene_id` i32, range `[-1, 256)` (-1 = overworld).
- `+0x260 / +0x268`. sim_horses vector.
- `+0x278 / +0x27c`. map_width / map_height.
- `+0x280 / +0x288`. ALL-HORSE pool vector, 36-byte stride. NOT owned.
- `+0x308`. money i32.
- `+0x314`. year.
- `+0x318`. sleeps.
- `+0x438`. Scene/subsystem table (ptr -> array of 256 slot ptrs).
- `+0x448`. alloc size = 0x440-ish.

Scene-table slots (`*(GS+0x438) + slot*8`, then `+0x130/+0x138` is the slot's `vector<Horse*>`):
- `0x00`. **OWNED horses (canonical)**. Invariant under `active_scene_id`.
- `0x08..0x38`. Race lanes 0..6 (7 lanes, each `vector<Horse*>`).
- `0x90`. Currently-selected-horse subsystem.
- `0xd0`. Mirror of owned (possibly "owned visible in current scene"); unverified.
- `0x120`. "Copy-all-horses" source (race-roster / current-event; NOT canonical owned).

Horse object (one allocation = 0x498 bytes; ctor `FUN_1400aac60`, dtor `FUN_1400bf1f0`):
- `+0x00`. vtable.
- `+0x1c`. type_or_species.
- `+0x1d0`. container kind (truck=7, pasture=9).
- `+0x1f8`. name_id u32 (indexes heap NAME_TABLE at 0x88 stride).
- `+0x1fc / +0x200`. age / max_age.
- `+0x204`. on_track_flag.
- `+0x205 / +0x206`. tired_flag_a / tired_flag_b (b is sleep-gate counter).
- `+0x207`. breeding_flag.
- `+0x21c`. skill.
- `+0x254`. litter_size_stat.
- `+0x2b8`. Vanilla genome PRIMARY bank (240 bytes).
- `+0x3a8`. Vanilla genome PAIRED bank (240 bytes; primary + 0xF0). DIPLOID: write both.

LOC (Home location sub-struct on scene slot 0):
- `+0x174 / +0x178`. Client-coord cursor floats (L3 InputSurface writes here).

## Save file (`save1.dat`)

20-byte header + variable-length horse-roster records + champion block + ~54 KB world-state binary. Ext-gene alleles ride in a **sidecar** (`save1.dat.bxsavext`), gated by D4 detours on save/load. D4 is currently unsafe-to-arm pending re-derive. We never mutate the main `.dat`.

Full: [`horsey-mod/docs/SAVE-FORMAT.md`](../../code/modforge/horsey-mod/docs/SAVE-FORMAT.md) (header layout, roster record schema, world-state hypotheses). Prior art: `alexjthomson/horsey-save-editor`.

## Hooking strategy (D-phase)

**S2 post-hook trampoline on `FUN_14009f680`** (14 KB gene-effect engine; called as the pair `FUN_14009f680(buf, horse+0x2b8); FUN_1400ab3d0(horse, buf);` from 6 sites) is the locked approach. Implementation: `modforge::hook::Hook` + `modforge::seh::guard` around the vanilla call.

D-phase shipped: D1 (3/5 detours: EVAL_DIPLOID_BLEND_A/B, GENE_ALLELE_SWAP; deferred GENE_DEATH_DRIFT, CRISPR_UI), D3.1/D3.2 lifecycle, D3.4 combinator, D5 render trampoline. Open: D2 pop-weight extension, D4 sidecar arming (resolved but currently unsafe on this build). `tests/arm_full_safe_stack` is the canonical regression for the 4-subsystem detour stack.

Full: [`docs/HOOKING-STRATEGY.md`](../../code/modforge/horsey-mod/docs/HOOKING-STRATEGY.md) (full S1-S6 candidate evaluation, decision rationale, section 8 implementation status, section 9 full implementation log).

## modforge module map (reuse audit)

Everything below is free to horsey-mod; never reimplement:

| Module | What |
|---|---|
| `envelope` | Op request/response envelope |
| `ops` | Op registry + dispatch |
| `selector` | Grammar parser for selecting game objects |
| `server` | tiny_http HTTP server with auth + body cap |
| `settings` | JSON config + debounced save + hot reload |
| `counters` | Atomic counters + `TimeScope!` macros |
| `ring` | Bounded ring buffer |
| `scanner` | Process memory scanner |
| `winproc` | Win32 process probes (`is_addr_readable`, etc.) |
| `shutdown` | Ordered shutdown registry |
| `log` | File + stdout sinks; DLL-relative dir resolution |
| `hot_reload` | Protocol types for live-reload |
| `args` | JSON arg helpers |
| `rpg` | Effect / Trigger / Skill traits + XP curve + Tracker + persistence + catalog (unused by horsey today) |
| `snapshots` | Generic projection-snapshot types |
| `debug` | Dispatch glue |
| `ui` | Declarative tab API (native window backend retired) |
| `worker` | Worker handle trait |
| `patterns` + `patterns::sleuth` | patternsleuth wrapper: `Pattern`, `resolve_all`, `scan_all_matches`, `TargetRegistry`, `Resolver`, `Recipe::*`, `Validator` |
| `vanilla` | `Signature` (ArgKind/RetKind), vanilla.invoke/list cmdlets |
| `input` | L1 SendInput, L2 PostMessage, L3 `InputSurface` trait, mouse/key/drag/scroll/combo cmdlets |
| `hook` | Generic detour wrapper (MinHook-shaped), revert-on-drop |
| `seh` | `seh::guard` catches AVs from foreign calls |
| `testkit` | Cross-game harness (relaunch + inject + HTTP + assert + taskkill; logs to `target/test-runs/<name>-<ts>.log`) |
| `research` | R4 helpers: decode operand, scan in window, decode field offset via string, decode imm at call site, find function bounds via int3, decode disp32 pair |
| `harness` | Older path; superseded by `testkit` but still used by some tests |

Cross-game proof: `grounded2-mod` adopted `TargetRegistry` (B6 starter).

## Key tests (the load-bearing ones)

- `tests/smoke.rs`. Fresh-launch e2e via `testkit`; verifies harness end-to-end.
- `tests/registry_parity.rs`. Guards TargetRegistry vs any remaining hardcoded RVAs.
- `tests/r4_field_offsets.rs`. Table-driven from `targets.resolve.field_offsets` op; every resolver auto-validated.
- `tests/wait_for_horse.rs`. Parameterized poller for "save loaded, expected horse appears". Reuse this; do NOT ask the user.
- `tests/find_owned_horses.rs`. Asserts owned chain returns a plausible list.
- `tests/dryrun_d3_d4.rs`. Catches stale save-function addresses (test-first regression net).
- `tests/arm_full_safe_stack.rs`. 4-subsystem detour stack arm/idle/disarm.
- `tests/arm_combinator.rs` / `arm_lifecycle.rs` / `arm_render_trampoline.rs`. Per-phase arming smoke.
- `tests/r2_*.rs`, `tests/r3_*.rs`. Resolver-tier regression nets (gamestate ptr, cheat globals, races/save-version, save signatures, function resolvers, save e2e roundtrip).
- `tests/research_*.rs`. Generic env-driven probes layered on `modforge::research` helpers.
- `tests/hk1_*.rs`. Shift-click transfer research probes (path currently pivoted to synthetic input).

Run a SINGLE test: `k3sc cargo-lock test -p horsey-mod --test <name> -- --nocapture --test-threads=1`.

## Crash diagnosis

Three-tier instrumentation is **mandatory** (don't weaken without replacement): (1) panic hook, (2) SEH vectored exception handler at priority 1 logging thread/RIP/bad_addr/kind, (3) step-by-step `HorseyState::capture()` logging that brackets every game-state read. Together they give line-precision crash localization without a debugger.

Diagnosis loop: `horsey.log` last line = high-water mark; first SEH line = crash signature. Compute `rva = rip - dll_base` (DLL base is in the inject success line). Symbols via `dumpbin`/windbg against `target/.../release/horsey.pdb`. Thread `horsey-http` -> HTTP handler bug; unnamed thread -> a game thread entered us via a detour/callback. Minidumps at `%PROGRAMDATA%\Microsoft\Windows\WER\Temp\WER*.tmp.dmp`.

**Detour-handler rules** (Cause A + Cause B at DEBUGGING.md section 4):
- Handler body = atomics + integer math + <= 1 indirect call. **NO** `format!`, `modforge::log!`, stack-buffered `OutputDebugStringA`, or mutex acquisition. (Cause A: handler stack frame too big -> page-fault on first `MOVAPS` spill.)
- **NO** `parking_lot`, `OnceLock` init, or anything else TLS-implicit. Game threads have no Rust TLS. Publish detours via lock-free `AtomicPtr<GenericDetour<T>>` (`Box::into_raw` + `AtomicPtr::store(Release)`); handler does `load(Acquire)` then calls. Reference: `patches/ext_genes.rs`.

**Pre-arm**: dump first 16 bytes at target, byte-compare expected prologue, fail loud. Every detour logs once on first call via a `static AtomicBool`.

**`.data` slot rule**: if decomp shows `*(T*)(SYM + N)`, SYM is a POINTER slot -> deref then offset, and gate the dereffed value (heap-range + alignment via `is_plausible_gamestate_pointer`). If decomp shows `*(T*)&SYM`, SYM IS the struct. The 2026-05-14 GAMESTATE_PTR clusterfuck cost half a day to a misread of this.

Full: [`docs/DEBUGGING.md`](../../code/modforge/horsey-mod/docs/DEBUGGING.md) (section 1 log locations, section 2 always-on instrumentation, section 3 standard loop, section 4 known-good signatures, section 4b handler discipline rules, section 5 pre-arm verification, section 6 first-call markers).

## Tests-first rule (locked 2026-05-15)

Every new feature, patch, or research finding ships as a test FIRST. The test asserts the contract before the implementation exists. We confirm fail, write code until pass, commit both together. Applies to detours (test asserts prologue bytes + post-arm `call_count > 0`), resolvers (R-tier table-driven test), gene authoring (round-trip test before runtime support), and research findings (`research_*.rs` env-driven probe).

## Binary identity, decomp, and external knowledge

Stripped native PE x64. Engine: SDL3 + cute_sound + stb_image_write. No PDB. Decomp: 10,332 functions via Ghidra 12.1 (pyghidra headless); ~1,000 are game-logic, the rest vendor. 20 high-value functions extracted to `research/decompiled/annotated/`. The decomp source binary may not match the live binary; `game.build_info` returns SHA-256 + size + image base for cross-check before trusting any offset.

Full references:
- [`docs/RE-NOTES.md`](../../code/modforge/horsey-mod/docs/RE-NOTES.md). String-anchor tiers (Tier 2 debug/format strings are highest-leverage for finding game-logic), calling convention, vtable patterns, per-update fragility.
- [`docs/DECOMPILATION-STATUS.md`](../../code/modforge/horsey-mod/docs/DECOMPILATION-STATUS.md). Pipeline, numbers, quality notes.
- [`docs/FUNCTION-BREAKDOWN.md`](../../code/modforge/horsey-mod/docs/FUNCTION-BREAKDOWN.md). Size distribution; vendor vs game-logic split.
- [`docs/STRUCTS.md`](../../code/modforge/horsey-mod/docs/STRUCTS.md). 73 observed Horse field offsets + GameState fields.
- [`docs/FIELD-READERS.md`](../../code/modforge/horsey-mod/docs/FIELD-READERS.md). Per-offset reader tables; horse-handler fingerprint heuristic.
- [`docs/MECHANICS.md`](../../code/modforge/horsey-mod/docs/MECHANICS.md). Decoded mechanics (no_tire, debug mode, money, retirement, loaded cheat).
- [`docs/VIABILITY.md`](../../code/modforge/horsey-mod/docs/VIABILITY.md). Q-gene-*, Q-pop-*, Q-save-*, Q-render-* viability answers with file:line cites.
- [`docs/CONTENT-CREATION.md`](../../code/modforge/horsey-mod/docs/CONTENT-CREATION.md). Item IDs, vanilla pops, vanilla genes, CLI flags, modding paths.
- [`docs/ENGINE-EXTENSION.md`](../../code/modforge/horsey-mod/docs/ENGINE-EXTENSION.md). Cost ladder, hard ceilings, "horse as a Rust trait" mental model.
- [`docs/EXTERNAL-KNOWLEDGE.md`](../../code/modforge/horsey-mod/docs/EXTERNAL-KNOWLEDGE.md). Steam guide by JumboDS64, miraheze wiki, prior-art repos (`alexjthomson/horsey-save-editor`, `NickPetrone/HorseyLiveTweaks`).
- [`docs/TOOL-RESEARCH.md`](../../code/modforge/horsey-mod/docs/TOOL-RESEARCH.md). Decompiler / hooking / injection tool landscape.
- [`docs/MODFORGE-INTEGRATION.md`](../../code/modforge/horsey-mod/docs/MODFORGE-INTEGRATION.md). How horsey-mod consumes modforge; reuse audit.
- [`docs/TESTING.md`](../../code/modforge/horsey-mod/docs/TESTING.md). Tests-first rule, `MODFORGE_EXPECT_LOADED` regression-catcher pattern.

## Mental model: layer on top of vanilla

Locked design principle from `VIABILITY.md` + `ENGINE-EXTENSION.md`: we EXTEND vanilla rather than replace. Examples that bake the rule in:

- Vanilla 240 genes -> ext 240 genes ride alongside, total 480; consumer call path is `FUN_14009f680(buf, horse+0x2b8); FUN_1400ab3d0(horse, buf);` and we post-hook the pair, never replace.
- Save format: ext alleles go in a `.bxsavext` sidecar, NEVER mutate the main `.dat` layout.
- No-tire: rather than reimplement the loop, NOP the single `+0x206` zero so the dev's own cheat does the work without breaking sleep.
- CRISPR-shaped writes: read what the in-game editor does (decomp-first); the apply function writes both diploid banks via `*(u8*)(horse + off) = val; *(u8*)(horse + off + 0xF0) = val;`. We mirror it.

When a feature has an in-game equivalent (CRISPR, debug mode, money cheat), the engine already has working code paths. Find them, hook around them, never reinvent.

## Content-authoring discipline (D-phase ext genes)

Loop: edit `bestiary/genes-extended.xml` -> `horsey-play` relaunch+inject -> validation tests with `MODFORGE_ATTACH=1` -> user visual confirmation. No hot-reload. Stash the stale `target/x86_64-pc-windows-msvc/release/genes-extended.xml` between iterations (the injector "leaves user copy" instead of overwriting).

Open blockers for "add content and see it in game" (see todo):
- **D2/D2.5/D2.6/D2.7** (per-pop weight extension via `pop-extended.xml`): without this, freshly spawned horses have ext-allele = 0. Set per-horse alleles via HTTP until shipped.
- **D4** (save sidecar): ext alleles don't survive save/load until D4 detour targets are armed; currently unsafe-to-arm.
