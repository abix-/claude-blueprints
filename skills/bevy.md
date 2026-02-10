---
name: bevy
description: Bevy 0.18 ECS patterns for the Endless colony sim. Use when writing Rust/WGSL for this project.
metadata:
  version: "1.8"
  updated: "2026-02-10"
---
# Bevy 0.18 — Endless Project

## Stack
- Bevy 0.18, bevy_egui 0.39, bytemuck 1, wgpu 27
- **Examples**: Always reference the matching release tag: `https://github.com/bevyengine/bevy/tree/release-0.18.0/examples`. Bevy's API changes significantly between versions — `main` branch and other release tags will have wrong signatures, removed types, or renamed modules.
- Rust edition 2024, rust-version 1.93
- Source: `rust/src/`, shaders: `shaders/`, assets: `assets/`
- Docs: `docs/README.md` (architecture), `docs/roadmap.md` (feature tracking)

## Key Files
- `rust/src/lib.rs` — `build_app()`, `Step` enum, system scheduling
- `rust/src/systems/behavior.rs` — decision system, `SystemParam` bundle examples
- `rust/src/tests/mod.rs` — test framework infrastructure
- `rust/src/tests/vertical_slice.rs` — 8-phase end-to-end test
- `rust/src/components.rs` — all ECS components
- `rust/src/render.rs` — camera, tilemap, sprite loading
- `rust/src/npc_render.rs` — instanced NPC rendering pipeline
- `rust/src/gpu.rs` — compute shader dispatch, readback

## Build & Run
```bash
cd /c/code/endless/rust && cargo build --release 2>&1
cd /c/code/endless/rust && cargo run --release 2>&1
# Tracy profiler support (connect with Tracy GUI while running):
cd /c/code/endless/rust && cargo run --release --features tracy 2>&1
```

## bevy_egui 0.39
- `EguiPlugin::default()` not `EguiPlugin` (struct with fields, not unit struct)
- `contexts.ctx_mut()` returns `Result` — use `let Ok(ctx) = contexts.ctx_mut() else { return };` (NOT `.unwrap()` — panics on first frame before fonts load)
- First-frame panic: fonts aren't loaded until after first render pass. Fix: `Local<bool>` guard to skip frame 1
- UI systems MUST use `EguiPrimaryContextPass` schedule, NOT `Update`. Systems in `Update` render visually but buttons won't respond to clicks
- UI system pattern: `fn my_ui(mut contexts: EguiContexts) -> Result { let ctx = contexts.ctx_mut()?; ... Ok(()) }`
- Don't use `.into()` on string literals when bevy_egui is in scope — ambiguous `From` impls. Pass `&str` directly or use `format!()`

## Bevy 0.18 Limits & States
- Max 16 system parameters per function. Use `#[derive(SystemParam)]` bundles to group related params (see `systems/behavior.rs`, `tests/mod.rs` CleanupCore/CleanupExtra)
- States: `#[derive(States, Default)]` with `#[default]` on variant, `app.init_state::<S>()`, `in_state(S::Variant)` run condition, `OnEnter`/`OnExit` for transitions, `ResMut<NextState<S>>` to trigger

## System Scheduling
Four ordered phases via `Step` enum:
```
Step::Drain → Step::Spawn → ApplyDeferred → Step::Combat → Step::Behavior → collect_gpu_updates
```
- **Drain**: reset, drain queues, sync GPU readback → `GpuReadState` resource
- **Spawn**: `spawn_npc_system`, `apply_targets_system`
- **Combat**: `process_proj_hits → cooldown → attack → damage → death → cleanup` (chained)
- **Behavior**: arrival, energy, healing, economy, decisions (parallel within set)
- `collect_gpu_updates` runs after Behavior, batches all `GpuUpdateMsg` into `GPU_UPDATE_QUEUE`

## Message Pattern
Bevy 0.18 uses `MessageWriter<T>` / `MessageReader<T>` (not `EventWriter`/`EventReader`):
```rust
fn my_system(mut writer: MessageWriter<DamageMsg>) {
    writer.write(DamageMsg { npc_index: idx, amount: 10.0 });
}
fn consume(mut reader: MessageReader<DamageMsg>) {
    for msg in reader.read() { /* ... */ }
}
```
Register with `.add_message::<T>()` not `.add_event::<T>()`.

## GPU Update Flow
Systems emit `GpuUpdateMsg(GpuUpdate::SetTarget { idx, x, y })` etc.
→ `collect_gpu_updates` drains into `GPU_UPDATE_QUEUE` (single Mutex lock)
→ `populate_buffer_writes` (PostUpdate) drains into `NpcBufferWrites` (per-index dirty tracking)
→ `ExtractResource` clones to render world
→ `write_npc_buffers` uploads only dirty slots via per-index `write_buffer` at byte offsets

## Data Authority
- **GPU owns**: positions, spatial grid, combat targets
- **Shared**: arrivals (GPU sets =1 on arrive, CPU clears =0 on new target)
- **CPU owns**: health, targets/goals, factions, speeds, behavior state
- **Render only**: sprite indices, colors (not in compute shader)
- 1-frame staleness budget (1.6px drift max). Never read GPU readback and write back same field in same frame.

## GPU Compute (gpu.rs)
- `GpuComputePlugin` adds render graph node `NpcComputeNode`
- 3-mode dispatch per frame: mode 0 (clear grid), mode 1 (build grid), mode 2 (separation + movement + combat targeting)
- Separate bind group per mode (each with own uniform buffer for mode value)
- Buffers: positions(0), goals(1), speeds(2), grid_counts(3), grid_data(4), arrivals(5), backoff(6), factions(7), healths(8), combat_targets(9), params(10)
- Workgroup size: 64. Max NPCs: 16384. Grid: 128x128, 64px cells, 48/cell

## GPU Readback Pattern (Async via Bevy `Readback`)
- 4 `ShaderStorageBuffer` assets as readback targets: npc_positions, combat_targets, proj_hits, proj_positions
- `ReadbackHandles` resource (ExtractResource) holds handles, extracted to render world
- Compute nodes `copy_buffer_to_buffer` from compute buffers → readback asset buffers (via `RenderAssets<GpuShaderStorageBuffer>`)
- `Readback::buffer(handle)` entities fire `ReadbackComplete` observers each frame (async, no blocking poll)
- Observers write directly to `Res<GpuReadState>`, `Res<ProjHitState>`, `Res<ProjPositionState>`
- `GpuReadState` + `ProjPositionState` have `ExtractResource` — cloned to render world for instanced rendering
- **Scope compute pass in a block** so it drops before encoder is used for copy commands
- Per-index dirty tracking on `NpcBufferWrites` — only changed slots get uploaded, not entire buffers

## Rendering Architecture
- **Static terrain/buildings**: `TilemapChunk` (Bevy built-in) — terrain (62K tiles) + buildings as two chunk layers with separate tilesets. Built once from `WorldGrid`, not per-frame. Replaces 62K instanced draw calls with Bevy's optimized tilemap path.
- **Dynamic NPCs/projectiles**: Custom `RenderCommand` pattern hooked into `Transparent2d` phase. 6 instanced layers (body + 5 overlay: weapon, helmet, armor, item, status/healing).
- **Render graph nodes are for compute/post-processing, NOT 2D geometry** — use `RenderCommand` + phase items instead
- `NpcInstanceData`: position[2] + sprite[2] + color[4] + health + flash + scale + atlas_id = per-instance
- `prepare_npc_buffers` builds instance buffer from `Res<GpuReadState>` (positions) + `NpcBufferWrites` (sprites, colors)
- Shader: `npc_render.wgsl` — camera extracted from Bevy `Camera2d` transform, atlas sampling with alpha discard

## Slot Management
`SlotAllocator` resource — LIFO free list. `alloc()` returns `Option<usize>`, `free(idx)` returns slot.
Dead NPCs: `death_cleanup_system` despawns entity, calls `HideNpc` (position = -9999), returns slot.
New spawns reuse freed slots.

## Component Patterns
- **Two-enum state machine**: `Activity` (what they're doing) × `CombatState` (combat overlay). Replaced 13 marker components.
  - `Activity`: Idle, Working, OnDuty{ticks_waiting}, Patrolling, GoingToWork, GoingToRest, Resting{recover_until}, Wandering, Raiding{target}, Returning{has_food}
  - `CombatState`: None, Fighting{origin}, Fleeing
  - `activity.is_transit()` → true for movement activities (Patrolling, GoingToWork, GoingToRest, Wandering, Raiding, Returning)
  - CombatState is orthogonal — Activity is preserved through combat, NPC resumes when combat ends
- Jobs: `Job::Farmer(0)`, `Job::Guard(1)`, `Job::Raider(2)`, `Job::Fighter(3)`
- Key components: `NpcIndex(usize)`, `Health(f32)`, `MaxHealth(f32)`, `Energy(f32)`, `Faction(i32)`, `TownId(i32)`
- **Prefer enums over marker components** for mutually exclusive states. Enum variants avoid archetype churn (every insert/remove of a marker triggers an archetype move in Bevy's table storage). One component change vs N component adds/removes per transition.
- **`#[require]` for invariants**: When component B must always accompany component A, use `#[require(B)]` on A so the invariant is declarative, not a manual insert you can forget.
- **Single source of truth for camera**: Don't mirror Bevy Transform/Projection into a custom resource. Write inputs directly to Transform+Projection, extract to render world with a dedicated extract system. No sync systems needed.

## Bevy 0.18 Render API Changes
These broke during the migration and were fixed in commits. Reference when touching render code:

### RenderCommand / Pipeline
- `RenderSet::*` → `RenderSystems::*` (e.g. `Prepare` → `PrepareResources`, `Queue` → `Queue`)
- `entry_point` is `Option<Cow<str>>` — use `Some(Cow::from("vertex"))` not `"vertex".into()`
- Transparent2d **requires** `DepthStencilState`: format `Depth32Float`, `depth_write_enabled: false`, `depth_compare: GreaterEqual`. Without this, nothing renders.
- MSAA must be queried from `&Msaa` in `queue_npcs` and passed to pipeline specialization. `specialize()` key is `(bool, u32)` for (HDR, sample_count). `MultisampleState::count` must match the view's MSAA.

### Bind Groups & Layouts
- `render_device.create_bind_group_layout()` → `BindGroupLayoutDescriptor::new()` (deferred creation)
- Store `BindGroupLayoutDescriptor`, get actual layout via `pipeline_cache.get_bind_group_layout(&descriptor)` at bind group creation time
- `SetMesh2dViewBindGroup` removed from `bevy::sprite_render` — texture bind group goes in slot 0

### Entity Extraction
- `commands.get_or_spawn(entity).insert(Component)` no longer works
- Use `commands.spawn((Component, MainEntity::from(entity)))` instead

### Query Types
- `ROQueryItem` takes two lifetime params: `ROQueryItem<'w, 'w, ...>` (not one)

## Test Framework (Endless)
- `AppState::TestMenu` (default) / `AppState::Running` — state machine drives test lifecycle
- `TestState` resource: shared by all tests, tracks phase, counters, flags, pass/fail
- `TestRegistry` holds `Vec<TestEntry>` (name, description, phase_count, time_scale)
- `test_is("name")` run condition gates per-test systems
- Each test: `setup` (OnEnter Running) + `tick` (Update after Behavior), both gated by `test_is()`
- Cleanup on `OnExit(Running)`: despawn `NpcIndex` entities, reset all resources
- Run All: `RunAllState` with queue, `auto_start_next_test` fires on `OnEnter(TestMenu)`

### Test Gotchas
- **Cleanup must cover ALL spawned entity types**: `cleanup_test_world` despawns `NpcIndex` entities, but if a test spawns other entities (FarmReadyMarker, projectiles, etc.), add a query for them too. Leaked entities break subsequent tests in Run All.
- **Neutralize orthogonal systems**: When testing behavior X, force-satisfy unrelated needs so the test isn't derailed. E.g., guard patrol test sets `LastAteHour = game_time.total_hours()` each tick to prevent starvation.
- **Don't double-consume queues**: If a state transition already pops from a queue (e.g., `auto_start_next_test` pops `RunAllState.queue`), the completion handler should only check `is_empty()`, not also pop. Two consumers = skipped entries.

## Performance Patterns (16K NPCs)
- **Per-index GPU uploads**: `write_buffer` at byte offsets for only changed slots, not full buffer re-uploads. `position_dirty_indices: Vec<usize>` tracks which NPC indices changed this frame. Critical at 16K scale — uploading 2 changed positions vs 128KB.
- **Single-pass buffer writes**: Write all fields (with defaults) per-entity in one loop iteration. Don't clear-all-then-set (two O(n) passes). Dead NPCs are sentinel-culled by the renderer (x < -9000) so stale data is harmless.
- **Active-only iteration**: Loop over `buffer[..npc_count]` not `buffer[..MAX_NPCS]`. Flash decay, visual sync — only process live slots.
- **Enums over markers for state**: Archetype churn from insert/remove of marker components is expensive at scale. A single enum component mutated in-place has zero archetype cost.
- **Async GPU readback**: Use Bevy's `Readback::buffer(handle)` + `ReadbackComplete` observers instead of manual staging buffers + blocking `device.poll(Wait)`. Eliminates frame stalls and staging buffer management.
- **TilemapChunk for static geometry**: Use Bevy's `TilemapChunk` for terrain/buildings instead of custom instanced rendering. 62K terrain tiles as one chunk vs 62K individual instances — single draw call, Bevy handles batching. Only use custom instanced rendering for dynamic entities (NPCs, projectiles).

## Common Gotchas
- **Bevy 0.18 Messages**: `add_message` not `add_event`. `MessageWriter`/`MessageReader` not `EventWriter`/`EventReader`.
- **Instance count**: Use `NpcGpuData.npc_count`, NOT `positions.len() / 2`. Buffers are pre-allocated to MAX_NPCS.
- **Sprite size in shader**: `SPRITE_SIZE` must match atlas cell size (16px), not an arbitrary render size.
- **Per-index dirty tracking**: Push idx to the matching `*_dirty_indices` Vec when writing to `NpcBufferWrites`. Without it, changed data never reaches the GPU.
- **Compute pass scoping**: Scope `compute_pass` in a `{}` block so it drops before using `encoder` for copy commands. Borrow checker requires this.
- **Bash paths on Windows**: Use `/c/code/endless` not `C:\code\endless`
- **PowerShell error suppression**: `-ErrorAction SilentlyContinue` not `2>$null`
- **ExtractResource**: Main world resources cloned to render world each frame. Render world cannot write back.
- **ApplyDeferred**: Runs between Spawn and Combat to flush entity commands before combat queries.
- **Static queues**: Only for CPU→GPU update boundaries (GPU_UPDATE_QUEUE, PROJ_GPU_UPDATE_QUEUE). GPU→CPU uses Bevy's async `Readback` + `ReadbackComplete`. Prefer MessageWriter everywhere else.
- **Kill Godot before building**: `taskkill //F //IM Godot_v4.6-stable_win64.exe` if DLL is locked.
