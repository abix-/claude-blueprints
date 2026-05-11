---
name: lua
description: Lua scripting standards for game modding (Project Zomboid Build 42, Factorio) and WezTerm configuration. The only user-authored Lua in scope is the WezTerm config; the PZ + Factorio repos here are vendored Steam Workshop / community-mod uploads, used as reference patterns, not as user-style sources.
user-invocable: false
version: "1.1"
updated: "2026-05-11"
---
# Lua

**Provenance note:** Of the three Lua surfaces below, only the
WezTerm config is user-authored. The PZ Build 42 and Factorio
samples come from vendored community mods uploaded for backup; they
are reference patterns for the respective engines, not user style.

Source surfaces:

- [claude-blueprints/wezterm/.wezterm.lua](https://github.com/abix-/claude-blueprints/blob/main/wezterm/.wezterm.lua)
  -- the only user-authored Lua. WezTerm 5.4, full standard library,
  event hooks.
- [abix-/CustomizableContainers](https://github.com/abix-/CustomizableContainers)
  -- vendored Project Zomboid Build 42 mod (Kahlua, PZ event API).
- [abix-/Fluid-Void-Extra](https://github.com/abix-/Fluid-Void-Extra)
  -- vendored Factorio mod (Lua 5.2, sandboxed, data + control phases).

## Target versions

- **Project Zomboid Build 42:** Kahlua (PZ's custom Lua interpreter,
  Java-hosted, roughly Lua 5.1 syntax + zombie of features).
- **Factorio (mods):** Lua 5.2 (sandboxed). 2.0+ uses `storage`
  instead of `global`.
- **WezTerm:** Lua 5.4. Standalone interpreter, full standard
  library.
- **Default for general Lua tasks:** Lua 5.4 syntax + LuaJIT-
  compatible patterns. PUC-Lua and LuaJIT are the two reference
  implementations.

## Syntax fundamentals

- **Local by default.** `local x = 1`. Bare assignment is global,
  pollutes the namespace. Globals are the root cause of mod
  conflicts.
- **Tables are everything.** Arrays, dicts, classes, modules, all
  tables. `t[1]` (1-indexed!) for arrays, `t.key` or `t["key"]`
  for maps.
- **Functions are values.** Assign, pass, return them like any
  data. `local function name() end` is sugar for
  `local name; name = function() end` (which handles recursion).
- **Strings:** single OR double quotes, identical semantics.
  `[[long]]` for raw multi-line strings.
- **Concatenation:** `..`. NOT `+` (that's numeric add and coerces).
- **Comments:** `--` line, `--[[ block ]]`.
- **No `++` or `+=`.** `x = x + 1`. Lua is intentionally austere.
- **`nil` is a value**; tables can have `nil` holes that confuse
  `#t` length operator. Use explicit counters when you need a
  stable length.

## Idioms

```lua
-- standard module shape
local M = {}

local function private_helper(x)
    return x * 2
end

function M.compute(input)
    return private_helper(input) + 1
end

return M
```

- **Module pattern:** local table at top, populate with functions,
  `return` at bottom. Imports: `local M = require("module.name")`.
- **Default args:** `local x = arg or default`. Only works when
  `arg` can't legitimately be falsy; otherwise check `arg == nil`.
- **Multiple return:** `local a, b = func()`. Excess discarded,
  missing become nil.
- **Variadic:** `function f(...) local args = {...} end`. Access
  with `select('#', ...)` for count.
- **Idiomatic ternary:** `cond and a or b`. Pitfall: returns `b`
  if `a` is false or nil. For `a == false`, use an if.

## Project Zomboid Build 42

```lua
-- event-driven, register handlers
local function OnLoad()
    if SandboxVars.MyMod.SomeOption == true then
        getPlayer():getKnownRecipes():add("RecipeName")
    end
end

Events.OnLoad.Add(OnLoad)
```

- **PZ exposes a global API**: `getPlayer()`, `getWorld()`,
  `getInventory()`, `Events.*.Add(fn)`. Treat as the runtime
  contract.
- **`SandboxVars.<ModName>.<Key>`** for user-configured options.
  Always check `== true` / `== false` explicitly; PZ sometimes
  returns Java booleans that aren't `truthy` to Lua in unexpected
  ways.
- **Files under `media/lua/`**:
  - `client/` -- runs on the player's machine, can read input,
    UI, sounds.
  - `server/` -- multiplayer authoritative logic.
  - `shared/` -- both. Used for tables, constants, definitions.
- **`media/lua/shared/01_*.lua`** -- the numeric prefix forces
  load order. Lower numbers load first.
- **Build 42 changes**: many APIs moved or were renamed. Always
  check the mod is targeting B42 (`apiVersion = 12.0` in
  `mod.info`).
- **Performance**: PZ runs Lua on the main thread. Heavy work in a
  per-tick event will stutter. Throttle or move to async via
  `setTimeout` (PZ-specific).

## Factorio

```lua
-- control.lua: runtime mod code
script.on_event(defines.events.on_built_entity, function(event)
    save_entity(event.entity)
end)

script.on_event(defines.events.on_tick, function(event)
    process_tick(event)
end)

-- data.lua: prototype definitions, runs at load time only
data:extend({
    {
        type = "item",
        name = "my-item",
        icon = "__MyMod__/graphics/my-item.png",
        ...
    }
})
```

- **Two phases**:
  - **Data stage** (`data.lua`, `data-updates.lua`,
    `data-final-fixes.lua`): defines prototypes (items, entities,
    recipes). Runs once at game load. No `game`, no `storage`.
  - **Runtime stage** (`control.lua`): handles events. No
    `data:extend`, no prototype changes.
- **`storage` (2.0+) / `global` (1.x)**: persistent mod state.
  Survives save/load. Always check `if storage.thing == nil then
  storage.thing = {} end` on first access.
- **`script.on_event(defines.events.X, handler)`**: register for
  game events. Per-tick handlers are expensive; throttle with
  a tick counter.
- **`script.on_nth_tick(N, handler)`**: handler every N ticks.
  Use over per-tick + counter for periodic work.
- **`game.print(...)`** for in-game debug. `log(...)` writes to
  `factorio-current.log`.
- **Migrations**: when mod data structure changes between versions,
  write a migration script in `migrations/<version>.lua`.

## WezTerm config

```lua
local wezterm = require 'wezterm'
local config = wezterm.config_builder()

config.front_end = "WebGpu"
config.default_prog = { 'pwsh.exe' }
config.font = wezterm.font('Cascadia Code', { weight = 'Regular' })
config.font_size = 12.0
config.color_scheme = 'Catppuccin Mocha'

-- event handlers
wezterm.on('format-tab-title', function(tab, _, _, _, _)
    return pane_label(tab.active_pane)
end)

return config
```

- **`wezterm` global** is the entry. `wezterm.config_builder()`
  for new configs (validates keys).
- **Event hooks**: `wezterm.on('event-name', fn)`. Common:
  `format-tab-title`, `format-window-title`, `update-status`,
  `bell`.
- **Plain Lua 5.4**: no sandbox, full standard library, file I/O
  via `io.open`.
- **Reload**: WezTerm watches the config file; saves trigger
  re-execution. Avoid expensive work at module scope.

## Performance

Lua is **fast for a dynamic language**. PUC-Lua is interpreted;
LuaJIT (used by Factorio internally, not exposed) is the fastest
dynamic language by a wide margin. Knowing what hits the slow path
is the whole game.

### Established wins

- **Cache `local` references to globals** inside hot loops:
  ```lua
  local sin = math.sin            -- one global lookup
  for i = 1, 1000000 do
      x[i] = sin(i)              -- local read, fast
  end
  ```
  Global access is a table lookup; locals are register/stack.
  20-30% speedup on tight loops.
- **`#t` (length) is O(n) for tables with nil holes.** Cache the
  length in a local before looping if the table is static.
- **`table.insert(t, x)`** is slower than `t[#t + 1] = x` when
  `t` has no nil holes. The function call has overhead.
- **`ipairs` over `pairs` for arrays.** `ipairs` stops at the
  first nil; `pairs` walks every hash slot.
- **String concatenation in a loop is O(n^2)** because strings
  are immutable. Use `table.concat(parts)` instead:
  ```lua
  local parts = {}
  for i = 1, 100 do parts[i] = tostring(i) end
  return table.concat(parts, ",")
  ```
- **Avoid creating tables in hot loops.** Reuse a single table
  across iterations and clear it (`for k in pairs(t) do t[k] = nil end`).
- **`tonumber` / `tostring` are not free.** Cache the result.
- **Closures cost a heap allocation each.** A closure created
  inside a loop allocates per iteration. Hoist if possible.
- **Multiple return is fast.** Don't pack/unpack tables to pass
  multiple values.

### Game-specific performance

- **Factorio per-tick event handlers run 60x/sec.** A 5ms handler
  is the entire frame budget. `script.on_nth_tick(30, ...)` for
  half-second cadence is more typical.
- **PZ Lua runs on the main game thread.** Anything heavy stutters
  the game. Profile with the in-game profiler (F11 debug menu).
- **Avoid `ipairs` over a 1000-entry table every tick** unless
  you're truly using every element. Cache indices when possible.

### Profiling

- **`os.clock()`** for microbenchmarks. Returns CPU seconds.
- **LuaJIT's `-jp` flag** for profile output when running standalone.
- **Factorio profiler:** `/measured-command` and the time-tracking
  in `factorio-current.log` when running with `--profile`.
- **PZ profiler:** F11 in dev mode. Shows per-frame Lua time.

## Error handling

```lua
local ok, err = pcall(risky_function, arg1, arg2)
if not ok then
    log("risky_function failed: " .. tostring(err))
    return
end

-- or with xpcall for a traceback
local ok, err = xpcall(risky_function, debug.traceback, arg1)
```

- **`pcall(fn, args...)`** runs `fn` in protected mode. Returns
  `true, results...` on success; `false, error` on failure.
- **`xpcall(fn, handler, args...)`** lets you pass a handler that
  receives the error message; use `debug.traceback` for a full
  stack.
- **`error("msg")`** raises. `error("msg", 2)` reports the
  caller's line. `assert(cond, "msg")` is the standard contract.
- **`error` accepts any value**, not just strings. Tables for
  structured errors.
- **Don't `pcall` what you can prevent.** Validate inputs at
  boundaries; let internal errors propagate.

## Tables: arrays vs maps

- **Arrays** (1-indexed, contiguous): use `#t` for length,
  `ipairs` to iterate.
- **Maps** (any key, any value): use `pairs` to iterate. Order
  is undefined.
- **Mixed tables work** but make `#t` undefined. Don't.
- **`table.sort(t, comparator)`** is in-place quicksort.
- **`table.remove(t, i)`** shifts all elements; O(n). For
  frequent removals from the middle, swap-and-pop (`t[i] =
  t[#t]; t[#t] = nil`).

## Strings

- **`s:method()`** sugar for `string.method(s, ...)`. Use freely.
- **Patterns** are NOT regex. Similar but simpler. `%d` digit,
  `%a` letter, `%s` whitespace, `*` greedy, `-` lazy, `?` optional,
  `+` one-or-more. No alternation, no lookahead.
- **`s:match(pattern)`** returns captures.
- **`s:gmatch(pattern)`** iterator over matches.
- **`s:gsub(pattern, repl)`** substitution. `repl` can be a string
  with `%1` backrefs or a function.
- **`string.format("%d: %s\n", n, name)`** for safe formatting.
- **`tostring(x)`** vs `..`: `..` calls `__tostring` metamethod;
  `tostring` is explicit. Use `tostring` for clarity in interpolations.

## Metatables and OOP

```lua
local Counter = {}
Counter.__index = Counter

function Counter.new(start)
    return setmetatable({count = start or 0}, Counter)
end

function Counter:inc(n)
    self.count = self.count + (n or 1)
end

function Counter:get()
    return self.count
end

local c = Counter.new(10)
c:inc()
print(c:get())   -- 11
```

- **`setmetatable(t, mt)`** attaches a metatable.
- **`__index = T`** makes `t.method` fall through to `T.method`.
  Standard OOP pattern.
- **`obj:method(a)`** sugar for `obj.method(obj, a)`. Always use
  `:` for methods, `.` for plain functions.
- **`__add`, `__eq`, `__lt`, `__tostring`, `__call`**: operator
  overloading. Heavy meta-metaprogramming gets confusing fast;
  use sparingly.

## Avoid

- **Globals in modules.** `function foo()` without `local` is a
  global. Always `local`.
- **`require` in a loop.** Cached but still costs a table lookup.
- **`print` in production mods.** Use the game's logger (`log`
  in Factorio, `getPlayer():showOutput` or similar in PZ).
- **`os.execute` in sandboxed contexts** (Factorio). It's not
  available and your mod will fail to load.
- **`io.*` in Factorio mods.** Use `game.write_file` for save
  files; everything else is sandboxed.
- **String concat in tight loops.** Always `table.concat`.
- **`==` on tables.** Compares identity, not contents. Write a
  `deepeq` helper.
- **Bare `for k, v in pairs(t)`** when you mean `for i, v in ipairs(t)`.
  Hash iteration is unordered and slower for arrays.
- **Mutating a table while iterating it.** Undefined behavior.
  Collect changes, apply after the loop.
- **Functions / closures stored in tables that get serialized.**
  Save/load systems can't serialize functions; you'll lose state.
