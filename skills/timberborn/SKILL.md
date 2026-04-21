---
name: timberborn
description: Timberbot mod DEVELOPMENT. Use when writing C# mod code, Python client code, tests, or docs. Not for playing the game. Use /timberbot for gameplay.
user-invocable: false
version: "5.0"
updated: "2026-03-28"
---
# Timberbot Development

This skill is for DEVELOPING the Timberbot mod (C# code, Python client, tests, docs, releases). It is NOT for playing the game. For gameplay, use the `/timberbot` skill instead.

## Project Layout

```
C:/code/timberborn/               # repo: abix-/TimberbornMods
  timberbot/
    src/                              # C# Unity mod (official mod system)
      TimberbotConfigurator.cs            # Bindito DI, [Context("Game")]
      TimberbotAutoLoadConfigurator.cs    # Bindito DI, [Context("MainMenu")]
      TimberbotService.cs                 # singleton orchestrator
      TimberbotReadV2.cs                  # all GET endpoints, projection snapshots
      TimberbotEntityRegistry.cs          # GUID-backed entity lookup, numeric ID bridge
      TimberbotWrite.cs                   # all POST write endpoints
      TimberbotPlacement.cs               # building placement, path routing
      TimberbotWebhook.cs                 # batched push notifications
      TimberbotDebug.cs                   # reflection inspector, benchmark
      TimberbotHttpServer.cs              # background HttpListener, routes
      TimberbotAutoLoad.cs                # auto-load save via autoload.json
      TimberbotAgent.cs                   # AI agent loop (claude/codex/custom binary)
      TimberbotJw.cs                      # fluent zero-alloc JSON writer
      TimberbotLog.cs                     # logging to timberbot.log
      ITimberbotWriteJob.cs               # write job interface for budgeted execution
      Timberbot.csproj                    # netstandard2.1, publicized DLLs, auto-deploy
      manifest.json                       # mod manifest (id: claude.Timberbot)
      settings.json                       # runtime config
    script/
      timberbot.py                        # API + CLI + dashboard (single file)
      test_v2.py                          # primary test harness
      test_v2_specs.py                    # test spec definitions
      test_validation.py                  # legacy test suite (74 tests)
      release.py                          # build + package + GitHub release script
    agent-prompt.md                       # default system prompt for AI agent
  docs/                               # getting-started, api-reference, developing
```

## Architecture

See `docs/architecture.md` for full thread model, snapshot pipeline, and request flow diagrams.

### Threading

- **GET requests**: served on background listener thread from published snapshots. Zero main-thread cost.
- **POST requests**: queued to main thread via `DrainRequests()`.
- **ReadV2 snapshots**: fresh-on-request projection snapshots. Main thread captures live state under per-frame budget, background worker finalizes and publishes immutable snapshots. Concurrent readers coalesce onto shared publishes.
- **`TimberbotJw`**: fluent zero-alloc JSON writer. `_jw.Reset().OpenObj().Key("id").Int(1).CloseObj().ToString()`. Auto-handles commas via depth-aware state.
- **EventBus**: `EntityInitializedEvent`/`EntityDeletedEvent` keep entity registry in sync.
- **Webhooks**: batched (200ms window, configurable), circuit breaker (30 failures = disabled). `TimberbotJw` on hot path, zero Newtonsoft.
- **Test suite**: `test_v2.py` primary harness (smoke, freshness, write_to_read, performance, concurrency). `test_validation.py` legacy suite (74 tests). Any save game, any faction. `validate` endpoint compares snapshot vs live state. CLI: `--perf`, `--benchmark`, `--list`, `-n`, individual test names.
- **Benchmark**: `/api/benchmark` profiles all endpoints + hot path. Confirmed zero-alloc (0 GC0 across 760K calls).

### Settings

`settings.json` in mod folder (`Documents/Timberborn/Mods/Timberbot/`):

| Setting | Default | Read by | Description |
|---|---|---|---|
| `debugEndpointEnabled` | true | C# | enable `/api/debug` reflection endpoint |
| `httpPort` | 8085 | C# + Python | HTTP server port |
| `httpHost` | `"127.0.0.1"` | Python only | host for Python client remote connections |
| `webhooksEnabled` | true | C# | enable webhook push notifications |
| `webhookBatchMs` | 200 | C# | batching window in ms (0 = immediate) |
| `webhookCircuitBreaker` | 30 | C# | consecutive failures before disabling webhook |
| `writeBudgetMs` | 1.0 | C# | per-frame budget for write job execution on main thread |

### C# Mod

8 DI-injected classes + 1 standalone:
- `TimberbotService` (7 DI). Lifecycle, settings, orchestration
- `TimberbotReadV2`. All GET endpoints, tracked refs, projection snapshots, background finalize
- `TimberbotWrite` (22 DI). All POST endpoints
- `TimberbotPlacement` (14 DI). Building placement, path routing
- `TimberbotEntityRegistry`. GUID-backed entity lookup, numeric ID bridge
- `TimberbotWebhook` (5 DI). Batched push notifications, circuit breaker
- `TimberbotDebug` (1 DI). Benchmark, reflection inspector
- `TimberbotHttpServer`. HTTP listener, routing
- `TimberbotAgent` (no DI). AI agent loop, spawns binary per cycle, instantiated by TimberbotService

Key patterns:
- All GET requests on background listener thread (zero main-thread cost)
- POST requests queued via `ConcurrentQueue`, drained on Unity main thread
- Entity lookup via `EntityComponent.GetComponent<T>()` (NOT Unity's `GameObject.GetComponent`)
- `TimberbotJw` for all JSON serialization (zero Newtonsoft on hot paths)
- `BepInEx.AssemblyPublicizer.MSBuild` to access internal types
- All errors via `TimberbotLog.Error()` with full stack traces to `timberbot.log`
- `PlaceBuildingResult` struct for placement returns. ToJson() at HTTP boundary only
- `PlaceBuilding` returns struct with Success/Error, not JSON string. Internal callers check `.Success`
- `ValidatePlacement` returns reason string (null = valid) using game's `BlockValidator` per-block checks
- `RoutePath` uses two-pass: plan all tiles first, then execute. No demolishing needed
- All endpoints accept `format` param (toon/json). `CollectTiles` format-aware: json=array occupants, toon=flat string
- Entrance coords: `PositionedEntrance.Coordinates` = path tile (find_placement), `DoorstepCoordinates` = building tile (entity cache)

### Python Client

- Single file: `timberbot/script/timberbot.py`
- Class: `Timberbot` (import with `from timberbot import Timberbot`)
- CLI: `timberbot.py <method> key:value ...` (on PATH, see getting-started.md)
- Args use `key:value` syntax (e.g. `prefab:Path x:120 y:130 z:2`)
- CLI output uses TOON format (compact, token-efficient for AI). Requires `pip install toons`
- `Timberbot()` = toon format (CLI default), `Timberbot(json_mode=True)` = json format (programmatic)
- `_post_json()` forces json format for internal methods that parse data (e.g. `map()` needs structured occupants array)
- `beavers()` returns wellbeing score and critical needs per beaver
- `map` renders colored ASCII grid (for humans), `tiles` returns raw tile data
- `top` shows live colony dashboard
- Naming: nouns for reads (`buildings()`, `trees()`), `verb_noun` for writes (`pause_building()`, `place_building()`)
- For levers/adapters/sensors, use Timberborn's built-in HTTP API on port 8080 directly

## Python API Quick Reference

```python
from timberbot import Timberbot
bot = Timberbot()

# read state (nouns)
bot.summary()       bot.time()          bot.weather()
bot.population()    bot.resources()     bot.districts()
bot.buildings()     bot.trees()         bot.crops()
bot.prefabs()       bot.speed()         bot.ping()
bot.buildings(detail="full")     # all fields including effectRadius
bot.buildings(id=-123)           # single building, all fields
bot.beavers()          # position, district, wellbeing, needs, carrying, deterioration
bot.beavers(detail="full")      # all needs with group category
bot.beavers(id=-123)            # single beaver/bot, all fields
bot.map(x1, y1, x2, y2, name=None)  # colored ASCII map, name saves to memory/
bot.tiles(x1, y1, x2, y2)     # raw tile data: terrain, water, occupants, seedlings, badwater
bot.gatherables()              # berry bushes, etc
bot.power()            # power networks: supply, demand, buildings per network
bot.science()          # science points + unlockable buildings
bot.wellbeing()        # wellbeing by category (Social, Fun, Nutrition, Aesthetics, Awe)
bot.distribution()     # import/export settings per district
bot.notifications()    # game event history
bot.workhours()        # work schedule (endHours, areWorkingHours)
bot.alerts()           # computed: unstaffed, unpowered, unreachable
bot.tree_clusters()    # top 5 grown tree clusters
bot.food_clusters()    # top 5 gatherable food clusters (berries, bushes)
bot.building_range(id) # work range tiles (farmhouse, lumberjack, forester, gatherer, DC)

# server-side filtering and pagination
bot.find(source="buildings", name="Farm")   # server-side name filter
bot.find(source="trees", x=120, y=140, radius=20)  # proximity filter
# sources: buildings, trees, gatherables, beavers
bot.buildings(limit=10, offset=20)  # paginated (server default limit=100, Python default=0 unlimited)
bot.trees(limit=0)                  # explicit unlimited

# spatial memory (CLI-only, per-settlement persistence)
bot.brain()            # live summary + persistent goal/tasks/maps
bot.brain(goal="...")  # set persistent goal
bot.list_maps()        # list saved map files
bot.clear_brain()      # wipe settlement memory folder
bot.add_task(action)   # add pending task to brain
bot.update_task(id, status)  # update task: pending/active/done/failed
bot.list_tasks()       # list all tasks
bot.clear_tasks()      # remove done tasks

# placement and pathing (read-then-write)
bot.find_placement(prefab, x1, y1, x2, y2)  # find valid spots with entrance, path, reachability
bot.place_building(prefab, x, y, z, orientation="south")
bot.place_path(x1, y1, x2, y2, z=0, style="direct", sections=0)  # A* path routing
bot.find_planting(crop, id=0, x1=0, y1=0, x2=0, y2=0, z=0)  # find valid planting spots

# write actions (verb_noun)
bot.set_speed(0-3)  # 0=pause, 1=normal, 2=fast, 3=fastest
bot.pause_building(id)              bot.unpause_building(id)
bot.set_priority(id, "VeryHigh")    bot.set_workers(id, count)
bot.set_floodgate(id, height)
bot.set_workhours(end_hours)        # 1-24, when work ends
bot.demolish_building(id)
bot.demolish_crop(id)
bot.mark_trees(x1, y1, x2, y2, z)  bot.clear_trees(x1, y1, x2, y2, z)
bot.plant_crop(x1, y1, x2, y2, z, crop)
bot.clear_planting(x1, y1, x2, y2, z)
bot.set_capacity(id, capacity)      bot.set_good(id, good)
bot.unlock_building(building)       # unlock with science
bot.set_distribution(district, good, import_option, export_threshold)
bot.migrate(from_district, to_district, count)  # move beavers between districts
bot.set_haul_priority(id, True)    # haulers deliver here first
bot.set_recipe(id, "RecipeId")     # set manufactory recipe
bot.set_farmhouse_action(id, "planting")  # planting or harvesting
bot.set_plantable_priority(id, "Pine")    # forester tree type priority
bot.set_clutch(id, engaged)        # engage/disengage power clutch

# AI agent control
bot.agent_status()     # status, binary, model, goal, currentCmd, lastError
bot.agent_stop()       # stop running agent

# for levers/adapters, use Timberborn's built-in HTTP API on port 8080 directly
```

**CLI-only commands (not on Timberbot class):**
```
timberbot.py start binary:claude [model:MODEL] [timeout:120] [goal:"survive and grow"]
timberbot.py top [interval:5]
timberbot.py manager
timberbot.py launch settlement:<name> [save:<filename>] [timeout:120]
```

## HTTP Endpoints (port 8085)

### Read (GET)

| Endpoint | Returns |
|----------|---------|
| `/api/ping` | `{status, ready}` |
| `/api/summary` | time + weather + districts (full snapshot) |
| `/api/resources` | resource stocks per district |
| `/api/population` | beaver/bot counts per district |
| `/api/time` | dayNumber, dayProgress, partialDayNumber |
| `/api/weather` | cycle, cycleDay, isHazardous, durations |
| `/api/districts` | districts with resources + population |
| `/api/buildings` | compact: id, name, coords, finished, paused, priority, workers. `?detail=full` for all fields incl effectRadius, productionProgress, readyToProduce. `?detail=id:<id>` for single building |
| `/api/trees` | trees only (Pine, Birch, Oak, etc): id, name, coords, marked, alive, grown, growth |
| `/api/crops` | crops only (Kohlrabi, Soybean, Corn, etc): id, name, coords, marked, alive, grown, growth |
| `/api/gatherables` | id, name, coords, alive |
| `/api/beavers` | position, district, wellbeing, needs, carrying, deterioration. `?detail=full` for all needs with group. `?detail=id:<id>` for single beaver |
| `/api/power` | power networks: [{id, supply, demand, buildings}] |
| `/api/prefabs` | name, sizeX, sizeY, sizeZ |
| `/api/distribution` | per district: goods with importOption, exportThreshold |
| `/api/science` | points, unlockables with name, cost, and unlocked status |
| `/api/speed` | current speed level (0-3) |
| `/api/wellbeing` | wellbeing by category (Social, Fun, Nutrition, Aesthetics, Awe) |
| `/api/alerts` | unstaffed, unpowered, unreachable buildings |
| `/api/tree_clusters` | densest grown tree clusters with coords |
| `/api/food_clusters` | densest gatherable food clusters (berries, bushes) |
| `/api/settlement` | current settlement / save name |
| `/api/notifications` | game event history |
| `/api/workhours` | work schedule (endHours, areWorkingHours) |
| `/api/tiles` | `?x1&y1&x2&y2` terrain + water + occupants + seedlings + badwater + contaminated |
| `/api/agent/status` | agent loop status: status, turn, totalTurns, binary, lastResponse, lastError |

### Webhooks (POST)

| Endpoint | Body | Description |
|----------|------|-------------|
| `POST /api/webhooks` | `{url, events?}` | register webhook, omit events for all |
| `POST /api/webhooks/delete` | `{id}` | remove webhook by id |
| `GET /api/webhooks` | | list registered webhooks |

68 push events: drought, building, beaver, weather, power, wonders, game state, etc. See `docs/webhooks.md`.

### Write (POST)

| Endpoint | Body | Description |
|----------|------|-------------|
| `/api/speed` | `{speed: 0-3}` | 0=pause, 1=normal, 2=fast, 3=fastest |
| `/api/building/pause` | `{id, paused}` | pause/unpause |
| `/api/building/place` | `{prefab, x, y, z, orientation}` | place building |
| `/api/building/demolish` | `{id}` | demolish building |
| `/api/crop/demolish` | `{id}` | demolish crop |
| `/api/building/floodgate` | `{id, height}` | floodgate height |
| `/api/building/priority` | `{id, priority}` | VeryLow/Normal/VeryHigh |
| `/api/building/workers` | `{id, count}` | desired workers |
| `/api/cutting/area` | `{x1, y1, x2, y2, z, marked}` | mark/clear cutting area |
| `/api/stockpile/capacity` | `{id, capacity}` | stockpile capacity |
| `/api/stockpile/good` | `{id, good}` | allowed good |
| `/api/planting/mark` | `{x1, y1, x2, y2, z, crop}` | plant crops |
| `/api/planting/find` | `{crop, building_id}` or `{crop, x1, y1, x2, y2, z}` | find valid planting spots |
| `/api/planting/clear` | `{x1, y1, x2, y2, z}` | clear planting |
| `/api/building/range` | `{id}` | work range tiles (farmhouse, lumberjack, forester, gatherer, scavenger, DC) |
| `/api/science/unlock` | `{building}` | unlock building with science |
| `/api/distribution` | `{district, good, import, exportThreshold}` | set import/export |
| `/api/workhours` | `{endHours: 1-24}` | set work end hour |
| `/api/district/migrate` | `{from, to, count}` | move beavers between districts |
| `/api/building/clutch` | `{id, engaged}` | engage/disengage power clutch |
| `/api/building/hauling` | `{id, prioritized}` | haulers deliver here first |
| `/api/building/recipe` | `{id, recipe}` | set manufactory recipe |
| `/api/building/farmhouse` | `{id, action}` | planting or harvesting priority |
| `/api/building/plantable` | `{id, plantable}` | forester/gatherer tree type priority |
| `/api/path/place` | `{x1, y1, x2, y2}` | route path with auto-stairs + platforms |
| `/api/placement/find` | `{prefab, x1, y1, x2, y2}` | find valid spots. JSON: nested `{prefab, sizeX, sizeY, placements}`. TOON: flat array (same keys, no wrapper) |
| `/api/debug` | `{target, path, ...}` | reflection inspector (requires debugEndpointEnabled) |
| `/api/benchmark` | `{iterations?}` | profile all endpoints (requires debugEndpointEnabled) |
| `/api/agent/start` | `{binary, turns, model?, interval?, prompt?, timeout?}` | start AI agent loop |
| `/api/agent/stop` | `{}` | stop running agent loop |

## Troubleshooting Rules

**ALWAYS use the debug endpoint to investigate BEFORE making code changes.** The debug endpoint lets you inspect live game state, call methods, and verify assumptions without rebuilding.

```powershell
# inspect fields on any injected service
timberbot.py debug target:fields path:_serviceName

# get a value
timberbot.py debug target:get path:_scienceService.SciencePoints

# call a method (result stored in $ for chaining)
timberbot.py debug target:call method:FindEntity arg0:-507504

# chain: get entity, then inspect its components
timberbot.py debug target:call method:FindEntity arg0:-507504
timberbot.py debug target:get path:$.AllComponents

# get a specific component by index
timberbot.py debug target:get path:$.AllComponents.[20].Overridable
```

**Debug-first workflow:**
1. Reproduce the issue with the running game
2. Use debug to inspect relevant game state (entities, components, service fields)
3. Verify your assumptions about how the game works
4. THEN make code changes
5. Test via debug again before rebuilding if possible

**Search other mods FIRST** (`/rtfm`). Use `gh search code` to find how other Timberborn mods solve the same problem. Key repos: thomaswp/BeaverBuddies, datvm/TimberbornMods, Timberborn-KyP-Mods/TimberPrint, ihsoft/TimberbornMods.

## Key Implementation Details

- Building IDs are Unity `GameObject.GetInstanceID()`, ephemeral per session
- Prefab names from `BuildingService.GetBuildingTemplate(name)` (e.g. "LumberjackFlag.IronTeeth")
- Tree marking uses `TreeCuttingArea.AddCoordinates(List<Vector3Int>)`. It is coordinate-based, not per-tree
- `Workplace.DesiredWorkers` controls worker assignment (0 = no workers)
- `Orientation`: south, west, north, east
- Priority has two types: `construction` (while building) and `workplace` (when finished). Set both on new buildings
- Building placement uses `PreviewFactory.Create()` + `BlockObject.IsValid()`, the game's own 9 validators. On failure, iterates `BlockValidator` (occupied/terrain/blocked) and `BlockObjectValidationService` validators for specific reason
- `BlockValidator.BlockConflictsWithExistingObject()` detects occupancy. Look up blocker name from entity cache
- Flood check: only on `MatterBelow.GroundOrStackable` tiles, not water intake tiles
- Water buildings: `WaterInputSpec.WaterInputCoordinates` identifies intake tile, `GetWaterDepth()` reads from water columns
- Crop planting uses `PlantingAreaValidator.CanPlant()`, same green/red tile check as player UI (from CordialGnom's ForestTool pattern)
- Occupancy: `BlockObject.Overridable` determines if an entity can be built over (empty cut tree stumps = overridable, dead standing trees = NOT overridable)
- Badwater: `WaterColumns[index3D].Contamination` via `MapIndexService.CellToIndex()` + `VerticalStride`
- Soil contamination: `ISoilContaminationService.SoilIsContaminated()` on land tiles near badwater
- Power: transfers through adjacent buildings only, paths don't conduct
- Never reimplement game validation manually. Use game-native services (PreviewFactory, PlantingAreaValidator, etc.)

## Timberbot Skill Maintenance

The `/timberbot` skill (`docs/timberbot.md` in repo, `~/.claude/skills/timberbot/SKILL.md` local) is a **game reference**, not a strategy guide.

**Design principles:**
- Facts only. No behavioral directives (no NEVER/ALWAYS/MUST/CRITICAL). State what IS true, not what to do
- The skill empowers play through knowledge, not by constraining behavior
- Goals come from the user prompt, not the skill
- No content duplication. The API table is the single source for method syntax

**Content categories:**
- Game mechanics: rates, ratios, durations, consequences (e.g. "beavers live ~50 days", "set_recipe destroys in-progress materials")
- Faction-specific facts: building prefab names, crop lists, wellbeing tables, production chains, all per faction
- API reference: error codes, endpoint syntax, response formats
- Wiki pointer: for anything not worth embedding (exact building costs, niche mechanics)

**Faction awareness is critical.** Folktails and Iron Teeth have different buildings, crops, food chains, wellbeing needs, power sources, and population mechanics. Every section that varies by faction must cover both. A wrong prefab name (e.g. "FarmHouse.Folktails" which doesn't exist) wastes an entire game turn.

**When updating the timberbot skill:**
1. Edit the repo file (`docs/timberbot.md`) first
2. Copy to local (`~/.claude/skills/timberbot/SKILL.md`), fix frontmatter (remove install blurb, set title to "Timberbot - Game Reference")
3. Bump version in both
4. New API endpoints: add to the API quick reference table
5. New error codes: add to the Error codes table with code prefix and meaning
6. New game mechanics: add under "Game mechanics" section as facts
7. Error format changes: keep the Error codes section in sync with `TimberbotJw.Error()` output format
8. Grep for behavioral language (`NEVER|ALWAYS|MUST|CRITICAL|Do NOT|Should`) and convert to factual statements

**Keep in sync:** repo and local must be identical except frontmatter. After editing repo, always `cp docs/timberbot.md ~/.claude/skills/timberbot/SKILL.md` and fix the header.

## API Rules

- C# mod validates ALL tiles before placing: occupancy, water, terrain
- Orientation: south, west, north, east
- Origin correction: coords always refer to bottom-left corner regardless of orientation
- All CLI output uses TOON format. Requires `pip install toons`
- `beavers()` returns wellbeing score and critical needs per beaver
- `buildings` includes `reachable`, `powered`, `isGenerator`, `isConsumer`, `powerDemand`, `powerSupply` fields
- `distribution()` returns import/export settings per good per district
- `tree_clusters()` finds densest clusters of grown trees with coords
- `timberbot.py` is on PATH. Run it directly (e.g. `timberbot.py summary`), never via `python timberbot/script/timberbot.py`
- NEVER inline python or pipe through python -c

## Build and Deploy

```powershell
cd timberbot/src
dotnet build          # compiles + auto-deploys to Documents\Timberborn\Mods\Timberbot\
```

Launch the game to a known save/settlement before live validation when needed:

```powershell
timberbot.py launch settlement:"Potato Tomato" save:bot
```

This launches Timberborn directly into that save context so HTTP validation can run immediately after the game finishes loading.

Release: `python release.py` (build + ZIP) or `python release.py --release` (+ tag + GitHub release)

### Pre-release doc audit (ALWAYS do before release)

1. List all endpoints in `TimberbotHttpServer.cs` (GET + POST routes)
2. For each endpoint, verify it appears in:
   - `docs/api-reference.md` (full endpoint docs with request/response)
   - `docs/timberbot.md` (API quick reference table)
   - `docs/features.md` (feature matrix row)
   - `~/.claude/skills/timberbot/SKILL.md` (gameplay skill API table)
   - `~/.claude/skills/timberborn/SKILL.md` (dev skill endpoint table)
3. For new response fields (like `flooded` on find_placement), verify the api-reference response table includes them
4. Check `docs/steam-workshop-description.txt` and `docs/getting-started.md` for stale command examples
5. Grep for old endpoint names across all docs to catch stragglers

Mods folder: `C:\Users\Abix\Documents\Timberborn\Mods\Timberbot\`
Game DLLs: `C:\Games\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed`

## Referenced DLLs

Publicized in .csproj:
- Bindito.Core, BaseComponentSystem, Common, EntitySystem, Coordinates
- GameCycleSystem, GameDistricts, Goods, HazardousWeatherSystem
- ResourceCountingSystem, SingletonSystem, TickSystem, TimeSystem
- WeatherSystem, WorldPersistence, Persistence
- Buildings, WaterBuildings, PrioritySystem, BlockSystem, BuilderPrioritySystem
- Demolishing, Cutting, Forestry, NaturalResourcesLifecycle
- WorkSystem, InventorySystem, Stockpiles, DistributionSystem
- BlockObjectTools, BuildingTools, TemplateInstantiation, TemplateSystem
- BlueprintSystem, BlueprintPrefabSystem
- Carrying, DeteriorationSystem, Bots, RangedEffectSystem
- SoilContaminationSystem, SoilMoistureSystem, NeedSystem, NeedSpecs, Wellbeing, LifeSystem


## Community Mod Patterns

- **BeaverBuddies** (thomaswp): Preview validation using `TemplateInstantiator.Instantiate()` + `MarkAsPreviewAndInitialize()` + `Reposition()` + `IsValid()` + `Destroy()`
- **TimberPrint** (KyP-Mods): `GetBuildableCoordinates()` for batch placement
- **datvm/TimberbornMods**: `UnstableCoreSpawner` preview-in-Load() reuse pattern
