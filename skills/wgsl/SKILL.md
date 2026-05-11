---
name: wgsl
description: WGSL shader patterns for Bevy 0.18 compute and instanced rendering. Use when writing or modifying .wgsl shaders.
user-invocable: false
version: "1.0"
updated: "2026-02-08"
---
# WGSL Shaders. Endless Project

## Files
- `shaders/npc_compute.wgsl`. 3-mode compute: clear grid, build grid, separation+movement+combat targeting
- `shaders/npc_render.wgsl`. Instanced quad renderer with sprite atlas + camera uniform
- `shaders/projectile_compute.wgsl`. Projectile movement + spatial grid collision detection

## WGSL vs GLSL (porting gotchas)
These bit us during the GLSL→WGSL port:
- `vec2(0.0)` → `vec2<f32>(0.0, 0.0)`. No implicit broadcast
- `int/uint/float` → `i32/u32/f32`. Explicit types everywhere
- `gl_GlobalInvocationID.x` → `@builtin(global_invocation_id) global_id: vec3<u32>` then `global_id.x`
- `layout(push_constant)` → `var<uniform>`. WGSL has no push constants, use uniform buffer
- `layout(set=0, binding=0, std430) buffer` → `@group(0) @binding(0) var<storage, read_write>`
- `atomicAdd(grid_counts[i], 1)` → `atomicAdd(&grid_counts[i], 1)`. Needs `&` reference
- `atomicStore` / `atomicLoad` also need `&` reference
- Atomic buffers: `array<int>` → `array<atomic<i32>>`. Must declare atomic type
- **Variable shadowing forbidden**. WGSL won't let you redeclare `dy` in nested loops. Use `dy2`, `dy3` etc. for separate grid scans.
- `clamp(int_val, 0, max)` works. `min()`/`max()` work on scalars.
- `#version 450` / `#[compute]`. Remove all GLSL preprocessor directives

## Compute Shader Pattern
```wgsl
struct Params {
    count: u32,
    delta: f32,
    // ... must be 16-byte aligned total (pad with _pad: f32 fields)
}

@group(0) @binding(0) var<storage, read_write> data: array<vec2<f32>>;
@group(0) @binding(N) var<uniform> params: Params;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let i = global_id.x;
    if (i >= params.count) { return; }
    // ...
}
```
- Workgroup size 64 is standard for NPC-scale compute
- Dispatch count: `ceil(count / 64)` workgroups
- All storage buffers `read_write` for simplicity (even if only read)

## Multi-Mode Dispatch
Single shader, dispatched 3 times per frame with different `params.mode`:
```wgsl
if (params.mode == 0u) { /* clear grid */ return; }
if (params.mode == 1u) { /* build grid */ return; }
// mode 2: main logic (separation, movement, targeting)
```
Rust side creates 3 bind groups, one per mode, each with its own uniform buffer containing the mode value.

## Spatial Grid (GPU-side)
```
Grid: 128x128 cells, 64px each, 48 NPCs/cell max
Memory: grid_counts = 64KB, grid_data = 3MB
```
- **Mode 0**: `atomicStore(&grid_counts[i], 0)`. One thread per cell
- **Mode 1**: `atomicAdd(&grid_counts[cell_idx], 1)`. One thread per NPC, claims slot, writes index to `grid_data[cell_idx * max_per_cell + slot]`
- **Mode 2**: Read grid via `atomicLoad(&grid_counts[cell_idx])` for neighbor queries
- Cell from position: `let cx = i32(pos.x / params.cell_size);`
- Hidden NPCs: `pos.x < -9000.0` means dead/hidden, skip in all modes
- Bounds check: `if (cx < 0 || cx >= gw || cy < 0 || cy >= gh) { return; }`

## Separation Physics
3x3 neighbor scan, asymmetric push strengths:
- Moving → settled neighbor: `push_strength = 0.2` (barely blocks me)
- Settled → moving neighbor: `push_strength = 2.0` (shove me aside)
- Exact overlap: golden angle spread `angle = f32(i) * 2.399 + f32(j) * 0.7`
- TCP dodge: perpendicular to movement direction, consistent side-picking via `if (i < u32(j))`
- Backoff: `persistence = 1.0 / f32(1 + my_backoff)`, cap at 200

## Combat Targeting
Wider search radius than separation:
```wgsl
let search_r = i32(ceil(params.combat_range / params.cell_size)) + 1;
```
Checks: different faction, alive (`health > 0`), not self. Tracks nearest by squared distance. Writes `-1` if no target.

## Render Shader Pattern
Two vertex buffer slots:
```wgsl
struct VertexInput {
    @location(0) quad_pos: vec2<f32>,     // slot 0: static quad
    @location(1) quad_uv: vec2<f32>,      // slot 0: static quad
    @location(2) instance_pos: vec2<f32>, // slot 1: per-instance
    @location(3) sprite_cell: vec2<f32>,  // slot 1: col, row in atlas
    @location(4) color: vec4<f32>,        // slot 1: RGBA tint
};
```

## Bind Groups (render shader)
- **Group 0**: texture + sampler (sprite atlas)
- **Group 1**: camera uniform (pos, zoom, viewport)

Keep texture in group 0. Bevy's Transparent2d phase expects this layout.

## Camera Uniform
```wgsl
struct Camera {
    pos: vec2<f32>,
    zoom: f32,
    _pad: f32,
    viewport: vec2<f32>,
}
@group(1) @binding(0) var<uniform> camera: Camera;

// Orthographic projection:
let world_pos = in.instance_pos + in.quad_pos * SPRITE_SIZE;
let offset = (world_pos - camera.pos) * camera.zoom;
let ndc = offset / (camera.viewport * 0.5);
```

## Sprite Atlas Sampling
```wgsl
const SPRITE_SIZE: f32 = 16.0;   // must match atlas cell pixels
const CELL_SIZE: f32 = 17.0;     // 16px sprite + 1px margin
const TEXTURE_WIDTH: f32 = 918.0;
const TEXTURE_HEIGHT: f32 = 203.0;

// UV from atlas cell:
let pixel_x = sprite_cell.x * CELL_SIZE + quad_uv.x * 16.0;
let pixel_y = sprite_cell.y * CELL_SIZE + quad_uv.y * 16.0;
let uv = vec2<f32>(pixel_x / TEXTURE_WIDTH, pixel_y / TEXTURE_HEIGHT);
```
- `SPRITE_SIZE` must match actual atlas cell pixels (16), not desired render size
- Character atlas: 918x203 (roguelikeChar). World atlas: 968x526 (roguelikeSheet).
- Alpha discard: `if tex_color.a < 0.1 { discard; }`

## Struct Alignment
WGSL uniform structs must be 16-byte aligned. Pad with `_pad: f32` fields:
```wgsl
struct Params {
    count: u32,           // 4 bytes
    separation_radius: f32, // 4 bytes
    separation_strength: f32, // 4 bytes
    delta: f32,           // 4 bytes — 16 aligned ✓
    grid_width: u32,      // ...
    grid_height: u32,
    cell_size: f32,
    max_per_cell: u32,    // 16 aligned ✓
    arrival_threshold: f32,
    mode: u32,
    combat_range: f32,
    _pad2: f32,           // pad to 48 bytes (16-aligned) ✓
}
```
Rust side must match with `#[repr(C)]` + bytemuck. Field order and padding must be identical.

## Shader Loading
- **Compute shaders**: loaded via raw wgpu `include_str!` → `ShaderModuleDescriptor` in gpu.rs
- **Render shaders**: loaded via Bevy asset system (`shader_defs: vec![]` in `RenderPipelineDescriptor`). Bevy handles compilation.

## Common Gotchas
- **No variable shadowing**. Use `dy2`, `dx3`, `n2` etc. for separate loop scopes
- **Atomic requires `&`**. `atomicAdd(&grid_counts[i], 1)` not `atomicAdd(grid_counts[i], 1)`
- **`SPRITE_SIZE` ≠ render size**. Must match atlas cell pixels (16px), quad expansion handles visual size
- **UV Y-flip not needed**. Wgpu texture coordinates are top-left origin, matching the atlas layout. Don't flip.
- **Bind group numbering matters**. Texture in group 0, camera in group 1. Swapping breaks Transparent2d.
- **`read_write` for all storage**. Even read-only buffers use `read_write` in compute. WGSL is lenient here and it avoids needing separate bind group layouts.
- **Hidden NPC sentinel**. `pos.x < -9000.0` means dead/hidden. Skip in all modes. Set position to `vec2<f32>(-9999.0, -9999.0)` to hide.
- **Deactivated projectile sentinel**. `proj_hits[i] = vec2<i32>(-1, 0)` means no hit. Set on deactivation to prevent re-trigger.

## Canonical references

- **WGSL spec** (`gpuweb.github.io/gpuweb/wgsl/`). Authoritative on
  syntax, type rules, alignment, and built-in functions.
- **wgpu examples** (`github.com/gfx-rs/wgpu/tree/trunk/examples`).
  Match the version Bevy uses (currently wgpu 27).
- **Tour of WGSL** (`google.github.io/tour-of-wgsl/`). Interactive,
  good for refreshing syntax.
- **Naga** (`github.com/gfx-rs/wgpu/tree/trunk/naga`) is the
  reference compiler; its diagnostics are the ground truth for what
  WGSL accepts.

## Workgroup and Dispatch Sizing

- **Workgroup size should be a multiple of 32** on NVIDIA (warp) and
  **64 on AMD** (wavefront). 64 is a safe default that works well on
  both.
- **Total workgroup size <= 256** for portability. Some integrated
  GPUs cap lower; check `Limits::max_compute_invocations_per_workgroup`
  at runtime if shipping broadly.
- **Bevy default dispatch is 1D**. For 2D grids (spatial scans),
  match dispatch shape to data layout: `@workgroup_size(8, 8)` for a
  64-thread workgroup that maps naturally to tiles.
- **Round dispatch UP** to the workgroup size and bounds-check inside
  the shader (`if (gid.x >= count) { return; }`). Don't try to
  short the last workgroup.

## Performance

GPU shader perf is about memory bandwidth, divergence, and atomic
contention more than ALU.

- **Coalesce reads.** Adjacent threads should read adjacent memory.
  Random access from a storage buffer is 10x slower than linear.
- **Avoid `if/else` divergence inside a warp.** A 32-thread warp
  serializes both branches when half take the `if` and half the
  `else`. Use `select(a, b, cond)` for short conditionals.
- **`textureLoad` over `textureSample`** when you don't need
  filtering. No sampler overhead.
- **Storage buffer reads are cached per-workgroup.** Reading the
  same value 100 times across a workgroup costs roughly the same
  as 1.
- **Atomic contention murders performance.** 1000 threads doing
  `atomicAdd` on the same counter serialize completely. Use
  per-workgroup accumulators in workgroup memory, then one atomic
  per workgroup at the end.
- **Workgroup memory (`var<workgroup>`)** is ~100x faster than
  storage. Use for inter-thread communication and accumulators.
  Limit is 16-32KB depending on GPU.
- **Avoid 64-bit atomics** in WGSL -- not supported on all GPUs
  yet. Use 32-bit indexes and ranges.
- **Minimize bind group switches.** Each `set_bind_group` is a
  small but non-zero cost. Batch related buffers into one group.
- **Push constants** (if your backend supports them) over uniform
  buffers for small frequently-changing data. wgpu exposes them
  via `Features::PUSH_CONSTANTS`.

## Bind Group Layout Discipline

- **Layout must match between pipeline creation and `set_bind_group`
  call.** Mismatches surface as cryptic validation errors at draw
  time, not at pipeline creation.
- **Group 0 for per-frame data** (camera, time), group 1 for
  per-material, group 2 for per-instance. Stable layout means
  fewer rebinds.
- **`@binding` indices are flat per group.** Don't reuse indices.
- **`read_write` is the safe default in compute.** WGSL is lenient
  about it and avoids needing separate bind group layouts. In
  performance-critical paths, mark truly read-only buffers as
  `read` so the driver can place them in a cacheable region.

## Vendor and Driver Gotchas

- **NVIDIA**: warp size 32, very forgiving WGSL acceptance.
- **AMD**: wavefront 64, stricter alignment requirements. Some
  WGSL constructs that work on NVIDIA fail validation on AMD.
- **Intel iGPU**: smaller workgroup limits, slower atomics. Test
  here if shipping to a general audience.
- **Apple Silicon (Metal backend)**: WGSL transpiles to MSL; some
  features (subgroup ops) unavailable. Best perf comes from larger
  workgroups (128-256) due to threadgroup memory hierarchy.
- **WebGPU vs native**: Web build has stricter validation (no
  bindless, smaller buffer limits, no subgroups in current spec).
  Test in browser if you ship a web target.

## Debugging

- **Naga validation errors** are read carefully -- they cite the
  exact line and rule.
- **`@compute @workgroup_size(1)` for serial debugging**: forces
  one thread per workgroup. Slow but deterministic.
- **`textureStore` debug output**: write intermediate values to a
  spare RGBA texture and inspect with RenderDoc. The poor man's
  printf for shaders.
- **RenderDoc** (renderdoc.org): capture a frame, inspect every
  bind group, buffer, and dispatch. Indispensable.
- **wgpu validation layers** are on by default in debug builds.
  Don't disable. The error messages are verbose but accurate.
- **Bevy `RenderDebug`** (if exposed in 0.18): toggles
  `RUST_LOG=wgpu_core=warn,wgpu_hal=warn` style filters at runtime.

## Avoid

- Variable shadowing inside loops. WGSL allows it but readers don't.
- Mismatched Rust struct + WGSL struct. Always update both in one
  commit and bench-test alignment.
- Long shader chains where one would do. Combining a separation pass
  and a movement pass saves a buffer roundtrip.
- Writing to storage buffers in vertex shaders -- legal but slow
  and often ill-supported.
- Recompiling shaders mid-frame. Bevy caches; manual `ShaderModule`
  creation per frame stalls.
- Tiny dispatches. The fixed overhead per dispatch (~5us) dominates
  for <1k invocations. Batch into one larger pass.
