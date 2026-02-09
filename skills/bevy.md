---
name: bevy
description: Bevy 0.18 ECS patterns for the Endless colony sim. Use when writing Rust/WGSL for this project.
metadata:
  version: "1.1"
  updated: "2026-02-08"
---
# Bevy 0.18 — Endless Project

## Stack
- Bevy 0.18, bevy_egui 0.39, bytemuck 1, wgpu 27
- Rust edition 2024, rust-version 1.93
- Source: `rust/src/`, shaders: `shaders/`, assets: `assets/`
- Docs: `docs/README.md` (architecture), `docs/roadmap.md` (feature tracking)

## Build & Run
```bash
cd /c/code/endless/rust && cargo build --release 2>&1
cd /c/code/endless/rust && cargo run --release 2>&1
```

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
→ `populate_buffer_writes` (PostUpdate) drains into `NpcBufferWrites` (per-field dirty flags)
→ `ExtractResource` clones to render world
→ `write_npc_buffers` uploads only dirty fields to GPU storage buffers

## Data Authority
- **GPU owns**: positions, spatial grid, combat targets, arrivals
- **CPU owns**: health, targets/goals, factions, speeds, behavior state
- **Render only**: sprite indices, colors (not in compute shader)
- 1-frame staleness budget (1.6px drift max). Never read GPU readback and write back same field in same frame.

## GPU Compute (gpu.rs)
- `GpuComputePlugin` adds render graph node `NpcComputeNode`
- 3-mode dispatch per frame: mode 0 (clear grid), mode 1 (build grid), mode 2 (separation + movement + combat targeting)
- Separate bind group per mode (each with own uniform buffer for mode value)
- Buffers: positions(0), goals(1), speeds(2), grid_counts(3), grid_data(4), arrivals(5), backoff(6), factions(7), healths(8), combat_targets(9), params(10)
- Workgroup size: 64. Max NPCs: 16384. Grid: 128x128, 64px cells, 48/cell

## GPU Readback Pattern
- Staging buffer created with `BufferUsages::MAP_READ | BufferUsages::COPY_DST`
- `readback_npc_positions` runs in `RenderSystems::Cleanup` phase
- **Scope compute pass in a block** so it drops before encoder is used for copy commands
- Poll with `render_device.poll(wgpu::PollType::wait_indefinitely())` (not old `wgpu::Maintain::Wait`)
- Map staging → read positions + combat_targets → write to `GPU_READ_STATE` static Mutex → unmap
- Per-field dirty flags on `NpcBufferWrites` prevent CPU from overwriting GPU-computed positions each frame

## Instanced Rendering (npc_render.rs)
- `NpcRenderPlugin` uses `RenderCommand` pattern hooked into `Transparent2d` phase
- **Render graph nodes are for compute/post-processing, NOT 2D geometry** — use `RenderCommand` + phase items instead
- Single instanced draw call: 4-vertex quad × instance_count
- `NpcInstanceData`: position[2] + sprite[2] + color[4] = 32 bytes/NPC
- `prepare_npc_buffers` builds instance buffer from GPU_READ_STATE (positions) + NpcBufferWrites (sprites, colors)
- `queue_npcs` adds `Transparent2d` phase item at sort_key 0.0
- Shader: `npc_render.wgsl` — hardcoded camera (known issue), atlas sampling with alpha discard

## Slot Management
`SlotAllocator` resource — LIFO free list. `alloc()` returns `Option<usize>`, `free(idx)` returns slot.
Dead NPCs: `death_cleanup_system` despawns entity, calls `HideNpc` (position = -9999), returns slot.
New spawns reuse freed slots.

## Component Patterns
- State markers: `Dead`, `InCombat`, `Resting`, `Working`, `OnDuty`, `Patrolling`, `Raiding`, `Returning`, etc.
- `derive_npc_state()` checks markers in priority order to get display name
- Jobs: `Job::Farmer(0)`, `Job::Guard(1)`, `Job::Raider(2)`, `Job::Fighter(3)`
- Key components: `NpcIndex(usize)`, `Health(f32)`, `MaxHealth(f32)`, `Energy(f32)`, `Faction(i32)`, `TownId(i32)`

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

## Common Gotchas
- **Bevy 0.18 Messages**: `add_message` not `add_event`. `MessageWriter`/`MessageReader` not `EventWriter`/`EventReader`.
- **Instance count**: Use `NpcGpuData.npc_count`, NOT `positions.len() / 2`. Buffers are pre-allocated to MAX_NPCS.
- **Sprite size in shader**: `SPRITE_SIZE` must match atlas cell size (16px), not an arbitrary render size.
- **Per-field dirty flags**: Always set dirty flag when writing to `NpcBufferWrites`. Without it, stale CPU data overwrites GPU-computed positions.
- **Compute pass scoping**: Scope `compute_pass` in a `{}` block so it drops before using `encoder` for copy commands. Borrow checker requires this.
- **Bash paths on Windows**: Use `/c/code/endless` not `C:\code\endless`
- **PowerShell error suppression**: `-ErrorAction SilentlyContinue` not `2>$null`
- **ExtractResource**: Main world resources cloned to render world each frame. Render world cannot write back.
- **ApplyDeferred**: Runs between Spawn and Combat to flush entity commands before combat queries.
- **Static queues**: Only for boundaries Bevy scheduler can't reach (GPU readback, render world). Prefer MessageWriter everywhere else.
- **Kill Godot before building**: `taskkill //F //IM Godot_v4.6-stable_win64.exe` if DLL is locked.
