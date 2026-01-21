---
name: godot
description: Godot 4.x game development patterns for colony/simulation games. Use when writing GDScript, optimizing NPC systems, implementing state machines, or scaling to thousands of entities.
metadata:
  version: "1.1"
  updated: "2026-01-20"
---
# Godot Development

## Scope

Colony/farm simulation in Godot 4.x with:
- Farmer NPCs with state machines (EATING, WORKING, SLEEPING, WALKING_HOME, WALKING_TO_FIELD, WAKING_UP, GOING_TO_BED)
- Food/hunger systems
- Time-based decision making via Clock/TimeManager autoload
- Target scale: 10,000 NPCs at 60 FPS

---

## GDScript Syntax & Gotchas

### String Formatting

GDScript does NOT support Python f-strings. Use `%` operator:

```gdscript
# ❌ WRONG - Python f-strings don't work
print(f"Value is {value:.2f}")

# ✅ CORRECT - Use % operator with array
print("Value is %.2f" % value)
print("Multiple values: %d, %.2f, %s" % [int_val, float_val, string_val])
```

### Modulo Requires Integers

```gdscript
# ❌ WRONG - float % int causes error
if minutes_eating % 10 == 0:

# ✅ CORRECT - convert to int first
if int(minutes_eating) % 10 == 0:
```

### Type Conversions

```gdscript
int()    # Truncates decimals (15.7 → 15)
float()  # Converts to decimal
str()    # Converts to string
```

### Debug Output Best Practices

Consolidate multi-line debug prints:

```gdscript
print("EATING COMPLETE: Farmer finished eating in %.2f hours (%d minutes), started with %.2f food, now at %.2f food, gained %.2f food (%.2f per hour)" % [
    hours_eating,
    minutes_eating,
    eating_start_food,
    food,
    food_gained,
    food_gained / max(1.0, hours_eating)
])
```

### Debug Mode Toggle Options

```gdscript
# Option 1: Export variable (recommended - editable in Inspector)
@export var debug_mode: bool = true

# Option 2: Constant
const DEBUG_MODE = true

# Option 3: Global singleton (DebugManager.gd autoload)
var debug_mode = true
```

---

## State Machine Implementation

### State Enum Pattern

States are integers for data-oriented design:

```gdscript
enum FarmerState {
    IDLE,
    SLEEPING,
    WAKING_UP,
    EATING,
    WORKING,
    WALKING_HOME,
    WALKING_TO_FIELD,
    GOING_TO_BED
}
```

### State Change Handler

```gdscript
func _handle_state_change(old_state, new_state):
    # Set cooldown to prevent rapid state changes
    decision_cooldown = 1.0

    match new_state:
        NPCStates.FarmerState.WALKING_TO_FIELD:
            current_destination = "Field"
            navigate_to(field_position)

        NPCStates.FarmerState.WALKING_HOME:
            current_destination = "Home"
            navigate_to(home_position)

        NPCStates.FarmerState.EATING:
            eating_start_food = food
            eating_start_hour = time_manager.hours
            eating_start_minute = time_manager.hours * 60 + time_manager.minutes

        NPCStates.FarmerState.SLEEPING:
            set_physics_process(false)
```

### Decision Making Function

```gdscript
func _make_decisions():
    # Report state with food level
    if state_machine.current_state != NPCStates.FarmerState.SLEEPING and debug_mode:
        print("Making decisions. Current state: %s, Food: %.2f/%.2f (%.2f%%), Hunger Status: %s" % [
            state_machine.get_state_name(state_machine.current_state),
            food,
            max_food,
            (food / max_food) * 100,
            _get_hunger_status()
        ])

    # Don't make decisions on cooldown
    if state_machine.is_on_cooldown():
        return

    # Handle extreme hunger
    if food <= starving_threshold and state_machine.current_state != NPCStates.FarmerState.EATING:
        if debug_mode:
            print("Farmer is too hungry to continue working")
        state_machine.change_state(NPCStates.FarmerState.WALKING_HOME)
        return

    # Normal decision making per state
    match state_machine.current_state:
        NPCStates.FarmerState.EATING:
            if food >= full_threshold:
                _handle_eating_complete()

        NPCStates.FarmerState.WORKING:
            if food < hungry_threshold:
                state_machine.change_state(NPCStates.FarmerState.WALKING_HOME)

        NPCStates.BaseState.IDLE:
            if time_manager.hours >= 22 or time_manager.hours < 7:
                state_machine.change_state(NPCStates.FarmerState.GOING_TO_BED)
            elif food < hungry_threshold:
                state_machine.change_state(NPCStates.FarmerState.WALKING_HOME)
            else:
                state_machine.change_state(NPCStates.FarmerState.WALKING_TO_FIELD)
```

### Hunger Status Helper

```gdscript
func _get_hunger_status() -> String:
    var food_percent = (food / max_food) * 100

    if food_percent <= starving_threshold:
        return "STARVING"
    elif food_percent <= hungry_threshold:
        return "HUNGRY"
    elif food_percent >= full_threshold:
        return "FULL"
    else:
        return "SATISFIED"
```

---

## Time Tracking System

### Precise Minute-Based Tracking

```gdscript
# Class-level variables
var eating_start_minute: int = 0
var eating_start_food: float = 0.0
var last_eating_report_hour: int = -1

# When starting an activity
func _on_start_eating():
    eating_start_minute = time_manager.hours * 60 + time_manager.minutes
    eating_start_food = food

# When completing an activity
func _handle_eating_complete():
    var current_minute = time_manager.hours * 60 + time_manager.minutes
    var minutes_eating = current_minute - eating_start_minute

    # Handle day wrapping (eating spans midnight)
    if minutes_eating < 0:
        minutes_eating += 24 * 60

    var hours_eating = minutes_eating / 60.0
    var food_gained = food - eating_start_food

    print("EATING COMPLETE: %.2f hours (%d minutes), gained %.2f food (%.2f per hour)" % [
        hours_eating, minutes_eating, food_gained,
        food_gained / max(1.0, hours_eating)
    ])
```

### Clock/TimeManager Reference

```gdscript
# Getting time manager reference
time_manager = Clock

# Accessing time values
time_manager.hours    # Current hour (0-23)
time_manager.minutes  # Current minutes (0-59)
```

---

## Data-Oriented Design (DOD)

### Traditional OOP vs Data-Oriented

**OOP (per-node approach):**
- Each NPC is a full Godot node with script instance
- Memory per NPC: ~2-4 KB
- 10,000 NPCs = 20-40 MB overhead + 20,000 function calls/frame

**Data-Oriented (Structure of Arrays):**
- NPCs are rows in arrays
- Memory per NPC: ~64-128 bytes
- 10,000 NPCs = 0.6-1.3 MB total
- ONE loop updates all NPCs

### Memory Layout Comparison

```
OOP (scattered in memory):
┌────────────────────────────────────────────────────────────────┐
│ [Farmer0 data...] [garbage] [Farmer1 data...] [other stuff]   │
│ [garbage] [Farmer2 data...] [unrelated] [Farmer3 data...]     │
│ ... CPU cache misses everywhere ...                            │
└────────────────────────────────────────────────────────────────┘

SoA (contiguous in memory):
┌────────────────────────────────────────────────────────────────┐
│ positions: [pos0][pos1][pos2][pos3][pos4][pos5]... (contiguous)│
│ food:      [f0  ][f1  ][f2  ][f3  ][f4  ][f5  ]... (contiguous)│
│ states:    [s0  ][s1  ][s2  ][s3  ][s4  ][s5  ]... (contiguous)│
│ ... CPU cache loves this ...                                   │
└────────────────────────────────────────────────────────────────┘
```

### Structure of Arrays (SoA) Layout

```gdscript
# NPCSystem.gd - ONE autoload, no per-NPC nodes
extends Node

# All data in contiguous arrays
var count: int = 0
var positions: PackedVector2Array
var velocities: PackedVector2Array
var food: host-3uyaeoky.example.testArray
var max_food: host-3uyaeoky.example.testArray
var states: host-hh0o0585.example.testArray
var next_update_tick: host-m7np8yn5.example.testArray
var target_pos: PackedVector2Array
var home_ids: host-hh0o0585.example.testArray
var work_ids: host-hh0o0585.example.testArray
var flags: host-hh0o0585.example.testArray

func _ready():
    # Pre-allocate for 10K
    positions.resize(10000)
    velocities.resize(10000)
    food.resize(10000)
    max_food.resize(10000)
    states.resize(10000)
    next_update_tick.resize(10000)
    target_pos.resize(10000)
    home_ids.resize(10000)
    work_ids.resize(10000)
    flags.resize(10000)

func add_npc(pos: Vector2, home_id: int) -> int:
    var idx = count
    count += 1
    positions[idx] = pos
    food[idx] = 100.0
    max_food[idx] = 100.0
    states[idx] = State.IDLE
    next_update_tick[idx] = 0
    home_ids[idx] = home_id
    return idx

func tick(current_tick: int):
    # ONE loop, not 10,000 function calls
    for i in range(count):
        if current_tick >= next_update_tick[i]:
            _update_npc(i, current_tick)

func _update_npc(idx: int, current_tick: int):
    match states[idx]:
        State.SLEEPING:
            next_update_tick[idx] = current_tick + 600  # Wake in 10 sec
        State.EATING:
            food[idx] += 5.0
            if food[idx] >= 100.0:
                states[idx] = State.IDLE
            next_update_tick[idx] = current_tick + 20
```

---

## Optimization Techniques

### 1. Sleep/Wake System (Biggest Win)

Idle entities consume ZERO CPU:

```gdscript
var next_update_tick: int = 0

func _physics_process(_delta):
    if Engine.get_physics_frames() < next_update_tick:
        return  # Skip update entirely
    _do_actual_update()

func _schedule_next_update(ticks_from_now: int):
    next_update_tick = Engine.get_physics_frames() + ticks_from_now

func _enter_state(new_state):
    match new_state:
        FarmerState.SLEEPING:
            _schedule_next_update(3600)  # Wake in ~60 seconds
            set_physics_process(false)
        FarmerState.EATING:
            _schedule_next_update(60)    # Check every second
        FarmerState.WORKING:
            _schedule_next_update(30)    # Check every half second
```

### 2. Bucket/Batch Updates

Spread NPC updates across multiple frames:

```gdscript
class_name NPCManager
extends Node

var npcs: Array[Farmer] = []
var update_bucket: int = 0
const BUCKETS = 10  # Spread across 10 frames

func _physics_process(_delta):
    var bucket_npcs = npcs.filter(func(npc):
        return npc.get_instance_id() % BUCKETS == update_bucket)
    for npc in bucket_npcs:
        npc.do_update()
    update_bucket = (update_bucket + 1) % BUCKETS
```

### Priority Buckets for 10K Scale

```
HIGH PRIORITY (every tick):     ~100 NPCs   (in combat, etc)
MEDIUM PRIORITY (every 5 ticks): ~400 NPCs  (walking)
LOW PRIORITY (every 20 ticks):  ~9500 NPCs  (sleeping, idle)

Per-tick cost: 100 + 80 + 475 = 655 NPC updates
NOT 10,000!
```

### 3. Tick-Based Simulation (Not Frame-Based)

Decouple simulation from rendering:

```gdscript
const TICK_RATE: int = 20  # 20 ticks per second
const TICK_DELTA: float = 1.0 / TICK_RATE

var current_tick: int = 0
var tick_accumulator: float = 0.0

func _physics_process(delta: float):
    tick_accumulator += delta

    while tick_accumulator >= TICK_DELTA:
        _simulation_tick()
        tick_accumulator -= TICK_DELTA
        current_tick += 1

    # Interpolation factor for smooth rendering
    var alpha = tick_accumulator / TICK_DELTA
    RenderSystem.interpolate(alpha)
```

### 4. Event-Driven (Not Polling)

```gdscript
# ❌ BAD: Check hunger every frame
func _process(delta):
    if food < hungry_threshold:
        go_eat()

# ✅ GOOD: React only when food changes
signal food_depleted

func consume_food(amount: float):
    food -= amount
    if food < hungry_threshold and not is_eating:
        food_depleted.emit()

func _ready():
    food_depleted.connect(_on_food_depleted)
```

### 5. State-Based Deactivation

```gdscript
func _enter_state(new_state):
    match new_state:
        FarmerState.SLEEPING:
            # Disable collision, navigation, animation
            $CollisionShape2D.set_deferred("disabled", true)
            $NavigationAgent2D.set_physics_process(false)
            $AnimationPlayer.stop()
            set_physics_process(false)  # THE BIG ONE
```

### 6. Path Caching

```gdscript
var path_cache: Dictionary = {}  # {start_chunk_end_chunk: path}

func get_path(from: Vector2, to: Vector2) -> PackedVector2Array:
    var cache_key = "%d_%d" % [_get_chunk(from), _get_chunk(to)]
    if path_cache.has(cache_key):
        return path_cache[cache_key]

    var path = NavigationServer2D.map_get_path(map_rid, from, to, true)
    path_cache[cache_key] = path

    # Expire after 60 seconds
    get_tree().create_timer(60.0).timeout.connect(func(): path_cache.erase(cache_key))
    return path
```

### NavigationServer2D Direct Access

For performance, use NavigationServer2D directly instead of NavigationAgent nodes:

```gdscript
var map_rid = get_world_2d().navigation_map

func get_path(from: Vector2, to: Vector2) -> PackedVector2Array:
    return NavigationServer2D.map_get_path(map_rid, from, to, true)
```

### Hierarchical Pathfinding (Factorio-inspired)

For large maps:
- **Coarse path**: Region → Region → Region
- **Fine path**: Only compute within current region
- **Negative path cache**: Remember unreachable destinations
- **Unit groups**: NPCs pathfind as groups, not individuals

### 7. Object Pooling

```gdscript
class_name ItemPool
extends Node

var pool: Dictionary = {}  # item_type -> Array of inactive items

func get_item(item_type: String) -> Node2D:
    if pool.has(item_type) and pool[item_type].size() > 0:
        var item = pool[item_type].pop_back()
        item.set_process(true)
        item.show()
        return item
    return _create_new_item(item_type)

func return_item(item: Node2D, item_type: String):
    item.set_process(false)
    item.hide()
    pool.get_or_add(item_type, []).append(item)
```

### 8. Deferred State Transitions

```gdscript
var pending_state_change: int = -1

func request_state_change(new_state: int):
    pending_state_change = new_state

func _physics_process(_delta):
    if pending_state_change >= 0:
        _actually_change_state(pending_state_change)
        pending_state_change = -1
```

---

## MultiMesh Rendering

Draw 10,000 NPCs with 1 draw call:

```gdscript
# NPCRenderer.gd
extends MultiMeshInstance2D

var prev_positions: PackedVector2Array
var curr_positions: PackedVector2Array

func _ready():
    multimesh = MultiMesh.new()
    multimesh.mesh = preload("res://meshes/npc_quad.tres")
    multimesh.transform_format = MultiMesh.TRANSFORM_2D
    multimesh.instance_count = 10000
    multimesh.visible_instance_count = 0

func sync_from_simulation():
    prev_positions = curr_positions.duplicate()
    curr_positions = NPCSystem.positions.duplicate()
    multimesh.visible_instance_count = NPCSystem.active_count

func interpolate(alpha: float):
    for i in range(multimesh.visible_instance_count):
        var pos = prev_positions[i].lerp(curr_positions[i], alpha)
        var xform = Transform2D(0, pos)
        multimesh.set_instance_transform_2d(i, xform)
```

For animations: Use texture atlas + custom shader to select sprite frame based on NPC state.

---

## Spatial Grid System

```gdscript
# 64x64 grid with cap of 64 NPCs per cell
const GRID_SIZE = 64
const MAX_PER_CELL = 64

var grid_cell_counts: host-hh0o0585.example.testArray
var grid_cells: Array[host-hh0o0585.example.testArray]  # NPC indices per cell
var npc_cells: host-hh0o0585.example.testArray  # Which cell each NPC is in

func _grid_cell_index(pos: Vector2) -> int:
    var x = int(pos.x / cell_size) % GRID_SIZE
    var y = int(pos.y / cell_size) % GRID_SIZE
    return y * GRID_SIZE + x

# Incremental update (better than full rebuild)
func _grid_update_incremental() -> void:
    for i in range(count):
        if healths[i] <= 0:
            continue

        var new_cell: int = _grid_cell_index(positions[i])
        var old_cell: int = npc_cells[i]

        if new_cell != old_cell:
            _grid_remove(i, old_cell)
            _grid_add(i, new_cell)
            npc_cells[i] = new_cell
```

---

## Performance Benchmarks

### Expected Performance by NPC Count

| NPC Count | Naive (every frame) | Optimized | Improvement | Naive FPS | Optimized FPS |
|-----------|---------------------|-----------|-------------|-----------|---------------|
| 10 | 1.0ms | 0.05ms | 20x | 60 | 60 |
| 100 | 10.0ms | 0.5ms | 20x | 55 | 60 |
| 500 | 50.0ms | 2.5ms | 20x | 18 | 60 |
| 1,000 | 100.0ms | 5.0ms | 20x | 9 | 60 |
| 5,000 | 500.0ms | 25.0ms | 20x | 2 | 38 |
| 10,000 | 1000.0ms | 50.0ms | 20x | 1 | 18 |

### Optimization Impact Breakdown

| Technique | Multiplier | Why |
|-----------|------------|-----|
| Sleep/Wake System | 5-10x | 70-90% of NPCs are idle |
| Bucket Updates | 5-10x | 1/10th work per frame |
| Event-Driven vs Polling | 2-3x | No wasted cycles |
| Navigation Caching | 3-5x | Pathfinding is expensive |
| Deactivate Physics/Collision | 2x | Godot physics isn't free |
| Object Pooling | 1.5-2x | Avoids GC spikes |

**Combined realistic improvement: 15-25x**

---

## Profiling

```gdscript
# Wrap expensive operations
func _expensive_operation():
    var start = Time.get_ticks_usec()
    # ... do work ...
    var elapsed = Time.get_ticks_usec() - start
    if elapsed > 1000:  # > 1ms
        print("WARNING: Expensive operation took %d μs" % elapsed)

# Per-system timing
func _process(delta):
    var t1 := Time.get_ticks_usec()
    _combat.process_scanning(delta)
    var t2 := Time.get_ticks_usec()
    print("Scanning: %.2f ms" % ((t2-t1)/1000.0))
```

---

## Architecture Overview (10K Target)

```
┌─────────────────────────────────────────────────────────────────┐
│                        GAME MANAGER                              │
│  - Fixed timestep simulation (20 ticks/sec)                     │
│  - Rendering interpolates between ticks                         │
└─────────────────────────────────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        ▼                       ▼                       ▼
┌───────────────┐    ┌───────────────────┐    ┌─────────────────┐
│  NPC SYSTEM   │    │  WORLD SYSTEM     │    │  RENDER SYSTEM  │
│  (GDScript/   │    │  (GDScript OK)    │    │  (MultiMesh)    │
│   C++/GDExt)  │    │                   │    │                 │
│               │    │                   │    │                 │
│ - State data  │    │ - Spatial grid    │    │ - Batched draws │
│ - Bucket exec │    │ - Resource nodes  │    │ - LOD states    │
│ - Pathfinding │    │ - Buildings       │    │ - Interpolation │
└───────────────┘    └───────────────────┘    └─────────────────┘
```

### System Separation

Architecture includes:
- `NPCSystem.gd` - All data arrays + update loop
- `NPCState.gd` - State machine logic
- `NPCNavigation.gd` - Movement/pathfinding
- `NPCCombat.gd` - Combat scanning/processing
- `NPCNeeds.gd` - Hunger, sleep, etc.
- `SpatialGrid` - Insertion, removal, queries
- `PathCache` - Path storage and lookup

---

## Implementation Roadmap to 10K

| Phase | Deliverable | NPCs Supported |
|-------|-------------|----------------|
| 1 | Data-oriented NPC arrays (GDScript) | 1,000 |
| 2 | Bucket execution + tick system | 2,500 |
| 3 | Spatial grid | 2,500 |
| 4 | MultiMesh rendering | 5,000 |
| 5 | Path caching + batching | 5,000 |
| 6 | GDExtension port of NPCSystem | 10,000 |
| 7 | GDExtension spatial grid | 10,000 |
| 8 | Polish + edge cases | 10,000 @ 60 FPS |

**Note:** For full 10K at 60 FPS, porting hot loops to GDExtension (C++) is necessary.

---

## Frame Time Budget (16.67ms for 60 FPS)

```
Simulation tick (amortized): ████████░░░░░░░░  8ms
  - NPC updates (500/tick):  ████░░░░░░░░░░░░  4ms
  - Pathfinding (10 req):    ██░░░░░░░░░░░░░░  2ms
  - World updates:           █░░░░░░░░░░░░░░░  1ms
  - Spatial grid maint:      █░░░░░░░░░░░░░░░  1ms

Rendering:                   ██████░░░░░░░░░░  6ms
  - MultiMesh update:        ███░░░░░░░░░░░░░  3ms
  - Interpolation:           ██░░░░░░░░░░░░░░  2ms
  - Draw calls:              █░░░░░░░░░░░░░░░  1ms

Headroom:                    ██░░░░░░░░░░░░░░  2.67ms

TOTAL:                       ████████████████  16.67ms ✓
```

---

## Quick Reference: Factorio Technique Mapping

| Factorio Technique | Your Implementation |
|--------------------|---------------------|
| Inserter sleep | Disable `_process()` for sleeping/idle farmers |
| Belt segments | Group crops/items by field, update per-field not per-item |
| Path caching | Cache farmer routes home↔field |
| Event-driven wake | Signal when food runs low, don't poll |
| Bucket updates | Spread NPCs across frames |
| Determinism | Fixed timestep, seeded RNG for saves/replay |

**Core Philosophy:** Don't update what doesn't need updating. Schedule future updates instead of polling.

---

## GDExtension Threshold

GDScript maxes out around 5,000-7,000 NPCs at 60 FPS. For 10K at 60 FPS, port hot loops to C++:
- NPC update loop
- Spatial grid operations
- State transitions
- Movement/interpolation

---

## Endless Project Patterns

### Enum Access
Access enums via class name, not instance:
```gdscript
# ❌ WRONG
npc_manager.jobs[i] != npc_manager.Job.FARMER

# ✅ CORRECT
npc_manager.jobs[i] != NPCState.Job.FARMER
```

### Sprite-Based Radii
Never hardcode pixel distances. Define sprites centrally:
```gdscript
const SPRITES := {
    "farm": {"pos": Vector2i(2, 15), "size": Vector2i(2, 2)},
    "fountain": {"pos": Vector2i(50, 9), "size": Vector2i(1, 1), "scale": 2.0},
}
```
Derive radii from definitions: `(cells * 16px * scale) / 2`

### Arrival System
- Target building **centers** (no offset)
- Use **edge radius** (center to edge, not corner)
- Separation forces spread NPCs naturally once arrived

### MultiMesh Instance Hiding
```gdscript
multimesh.set_instance_transform_2d(i, Transform2D(0, Vector2(-9999, -9999)))
```

### Exponential Cost Scaling
```gdscript
static func get_upgrade_cost(level: int) -> int:
    return int(10 * pow(1.001, level))  # 10 at level 0, ~220k at level 9999
```

### Sqrt Stat Scaling
```gdscript
static func get_stat_scale(level: int) -> float:
    return sqrt(float(level + 1))  # 1x at level 0, 100x at level 9999
```

### Pool Pattern (Projectiles, Audio, etc.)
```gdscript
var free_indices: Array[int] = []

func acquire() -> int:
    if free_indices.is_empty(): return -1
    return free_indices.pop_back()

func release(i: int) -> void:
    free_indices.append(i)
```

### Non-NPC Shooter Indices
Guard posts use negative indices: `-1000 - post_idx`
Check `if shooter < 0` to skip XP/aggro logic.

### Grid Spacing
34px for 32px buildings (1px border each side): `const TOWN_GRID_SPACING := 34`
