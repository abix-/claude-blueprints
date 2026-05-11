---
name: csharp
description: C# development standards. Use when writing C# code, .NET projects, Unity mods, or NuGet packages. Sourced from [abix-/TimberbornMods](https://github.com/abix-/TimberbornMods) (Timberbot, Bindito DI, publicizer) and [abix-/Schedule1Mods](https://github.com/abix-/Schedule1Mods) (MelonLoader + Harmony + Il2CppInterop).
user-invocable: false
version: "1.1"
updated: "2026-05-11"
---
# C# Development

## Environment

- Windows 10, .NET SDK installed (multiple versions available)
- Build: `dotnet build` (debug), `dotnet build -c Release` (release)
- Target framework depends on project. Check `<TargetFramework>` in csproj
- Unity mods target `netstandard2.1` (builds with any SDK 6+, output is always netstandard2.1)

## SDK vs Target Framework

The .NET SDK version (6, 7, 8, 9) is the build tool. The `<TargetFramework>` in csproj is the output. They're independent:
- `netstandard2.1`. Unity/game mods, maximum compatibility
- `net6.0` through `net9.0`. Standalone apps, pick latest stable
- `<LangVersion>` in csproj limits C# syntax features regardless of SDK

Building a `netstandard2.1` project with SDK 9 works fine. The SDK is just the compiler.

## Project Structure

```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>netstandard2.1</TargetFramework>
    <LangVersion>9.0</LangVersion>
  </PropertyGroup>
  <ItemGroup>
    <Reference Include="SomeLib">
      <Private>false</Private>           <!-- don't copy to output -->
      <HintPath>path\to\SomeLib.dll</HintPath>
    </Reference>
  </ItemGroup>
</Project>
```

`<Private>false</Private>` prevents copying referenced DLLs to output. Use when the host already has them (Unity, game runtime).

## Accessing Internal Types

When modding games or extending libraries with internal types, use BepInEx AssemblyPublicizer:

```xml
<PackageReference Include="BepInEx.AssemblyPublicizer.MSBuild" Version="0.4.2" PrivateAssets="all"/>

<Reference Include="SomeAssembly" Publicize="true">
  <Private>false</Private>
  <HintPath>$(GameDir)\SomeAssembly.dll</HintPath>
</Reference>
```

This makes all internal/private types accessible at compile time without modifying the DLL.

## Patterns

### Dependency Injection

Game mods often use DI frameworks (Bindito, Zenject, VContainer). Constructor injection:

```csharp
public class MyService
{
    private readonly ISomeService _someService;

    public MyService(ISomeService someService)
    {
        _someService = someService;
    }
}
```

### HTTP Server in Background Thread

For embedding an HTTP API in a Unity game/mod:

```csharp
private HttpListener _listener;
private Thread _listenerThread;
private ConcurrentQueue<PendingRequest> _pending = new ConcurrentQueue<PendingRequest>();

// start on background thread
_listener = new HttpListener();
_listener.Prefixes.Add("http://+:8085/");
_listenerThread = new Thread(ListenLoop) { IsBackground = true };
_listenerThread.Start();

// drain on main thread (Unity Update)
public void UpdateSingleton()
{
    while (_pending.TryDequeue(out var req))
        ProcessRequest(req);
}
```

**Key:** reads can run on the listener thread if data is thread-safe. Writes MUST queue to main thread.

### Reference-Compare Caching (RefChanged pattern)

When you derive a string from a reference that rarely changes, use a shared `RefChanged` helper to skip the derivation when the source hasn't changed. One pointer comparison per refresh instead of string allocation:

```csharp
// DRY helper -- shared by all refresh code
private static bool RefChanged(ref object cached, object current)
{
    if (ReferenceEquals(cached, current)) return false;
    cached = current;
    return true;
}

// in cached struct
public string Workplace;
public object LastWorkplaceRef;

// in refresh -- one-liner per field
var wp = c.Worker?.Workplace;
if (RefChanged(ref c.LastWorkplaceRef, wp))
    c.Workplace = wp != null ? CleanName(wp.GameObject.name) : null;
```

Use for: workplace names, district names, recipe names, any string derived from a game object reference. The `ReferenceEquals` check is a single pointer comparison. Essentially free.

### Immutable-at-Add-Time Pattern

Values that never change after entity creation (building coordinates, orientation, footprint tiles, effect radius) should be set ONCE in the add-time handler, not re-read every refresh:

```csharp
// in AddToIndexes (runs once when entity is created)
cb.X = coords.x; cb.Y = coords.y; cb.Z = coords.z;
cb.Orientation = OrientNames[(int)bo.Orientation];
cb.EffectRadius = ec.GetComponent<RangedEffectBuildingSpec>()?.EffectRadius ?? 0;

// in RefreshCachedState -- only read values that actually change
c.Finished = c.BlockObject.IsFinished;  // this changes
// c.X, c.Y, c.Z -- NEVER re-read (immutable)
```

Rule: if a value only changes when the entity is created or destroyed, it belongs in the add-time handler, not the per-second refresh.

### Fluent Zero-Alloc JSON Writer (TimberbotJw)

For all JSON serialization, use a fluent `TimberbotJw` instead of Dictionary+Newtonsoft. Allocate once as a field, `Reset()` per request. Auto-handles commas via depth-aware state. No manual separator tracking:

```csharp
// field -- allocated once, reused across all requests
private TimberbotJw _jw = new TimberbotJw(200000);

// usage -- fluent chaining, auto-commas, nesting-aware
public string CollectItems()
{
    var jw = _jw.Reset().OpenArr();
    foreach (var item in _items.Read)
    {
        jw.OpenObj()
            .Key("id").Int(item.Id)
            .Key("name").Str(item.Name)
            .Key("active").Bool(item.Active);
        if (item.Progress > 0)
            jw.Key("progress").Float(item.Progress, "F1");
        jw.CloseObj();
    }
    jw.CloseArr();
    return jw.ToString();
}
```

**Key:** `AutoSep()` inside `Key()`/`OpenObj()`/`OpenArr()` inserts commas automatically. No `bool first` tracking. Nested objects and arrays just work. Single shared `_jw` instance for all endpoints (serial on listener thread).

### Value Tuples Instead of Anonymous Objects

When you need to sort or compare intermediate results, use value tuples instead of anonymous objects. Avoids reflection for property access:

```csharp
// BAD -- anonymous objects require reflection to sort
var results = new List<object>();
results.Add(new { x = 1, score = 5 });
results.Sort((a, b) => (int)a.GetType().GetProperty("score").GetValue(a) - ...);

// GOOD -- tuples give direct field access
var results = new List<(int x, int y, int score, bool valid)>();
results.Add((1, 2, 5, true));
results.Sort((a, b) => b.score - a.score);
```

### Reflection for API Discovery

When working with publicized internals where you don't know property names:

```csharp
// temporary: dump all members
var members = obj.GetType()
    .GetMembers(BindingFlags.Public | BindingFlags.NonPublic | BindingFlags.Instance)
    .Where(m => m.MemberType == MemberTypes.Property || m.MemberType == MemberTypes.Field)
    .Select(m => m.Name);
entry["_debug"] = string.Join(", ", members);
```

Build, run, read the output, then replace with proper property access. Remove the reflection code after.

## C# 9.0 Features (netstandard2.1 compatible)

Available:
- `is` pattern matching (`obj is int v`, `obj is not null`)
- `switch` expressions
- Target-typed `new()` (`Dictionary<string, object> d = new()`)
- Null-coalescing assignment (`x ??= default`)
- `using` declarations (no braces)
- Tuple deconstruction
- Local functions

NOT available in netstandard2.1:
- File-scoped namespaces (`namespace Foo;`). Requires C# 10
- Global usings. Requires C# 10
- Raw string literals. Requires C# 11
- Primary constructors. Requires C# 12
- Collection expressions (`[1, 2, 3]`). Requires C# 12

## Common Gotchas

- **Variable name conflicts in nested scopes:** C# forbids reusing names from enclosing scopes in pattern matching (`is int g` fails if `g` exists in outer scope)
- **`foreach` on non-IEnumerable:** types like `GoodAmount` may look iterable but aren't. Check the actual type
- **`GetComponent<T>()` ambiguity:** game frameworks (Unity, Timberborn) may have their own `GetComponent` that shadows Unity's. Use the right one.
- **`Publicize="true"` doesn't mean types exist:** the type must actually be in that DLL. If you get "type not found," check `ls Managed/ | grep -i keyword` to find the right DLL.
- **Thread safety:** Unity APIs are main-thread only. Background HTTP threads must queue work, not call Unity directly.
- **Private fields via publicizer use `_` prefix:** e.g. `node._nominalPowerInput` (original private field naming preserved)

## Build Troubleshooting

```bash
dotnet build              # debug build
dotnet build -c Release   # release build
dotnet clean && dotnet build  # force full rebuild
```

- CS0136 "name used in enclosing scope". Rename the variable in the pattern match
- CS1061 "does not contain a definition". Wrong property name on publicized type, use reflection to discover
- CS1579 "foreach cannot operate". Type isn't iterable, access its properties directly
- "type not found". Add the DLL reference to csproj with Publicize="true"

## GC / Allocation Patterns

### Per-frame code (Update loops). Zero alloc target

- **Never** use `new Dictionary`, `new List`, LINQ, `string.Format`, `ToString()` on enums in per-frame code
- **Do** use pre-allocated structs, arrays, static lookup tables, `StringBuilder` reuse
- Cache component references at add-time, read cached primitives per-frame
- Cadence expensive refreshes (e.g. 1Hz instead of 60Hz) when staleness is acceptable
- Use double-buffering for thread safety: main thread writes to buffer A, background reads from buffer B, swap refs

### foreach enumerator boxing

`foreach` over an interface (`IEnumerable<T>`, `IReadOnlyList<T>`) boxes the struct enumerator on the heap (~40 bytes). Safe in per-request code, avoid in per-frame:

```csharp
// BAD in hot path -- boxes enumerator if AllInventories returns IEnumerable<T>
foreach (var inv in building.AllInventories) { ... }

// GOOD -- use indexer if the type supports it
var list = building.AllInventories;
for (int i = 0; i < list.Count; i++) { var inv = list[i]; ... }
```

Only matters in per-frame/per-second refresh paths. Per-request code (HTTP responses) can use foreach freely.

### Why TimberbotJw instead of Newtonsoft

Avoids: Dictionary allocs per item, Newtonsoft reflection, intermediate string allocs. 10x+ faster than `JsonConvert.SerializeObject(list)`. All endpoints use a single shared `TimberbotJw` instance. Serial on the listener thread, no concurrency concern.

### LINQ in hot paths

Never use LINQ (`.Select()`, `.Where()`, `.ToList()`, `.ToArray()`) in per-frame or per-second code. Each LINQ call allocates iterator objects + closures. Use simple loops instead:

```csharp
// BAD -- allocates iterator + anonymous objects + list
tile["occupants"] = occList.Select(o => new { o.name, o.z }).ToList();

// GOOD -- simple loop, explicit types
var stacked = new List<object>(occList.Count);
foreach (var o in occList)
    stacked.Add(new Dictionary<string, object> { ["name"] = o.name, ["z"] = o.z });
tile["occupants"] = stacked;
```

LINQ is fine in per-request code and one-time initialization.

## MelonLoader + Harmony (Schedule 1)

Schedule 1 mods use a different stack from Timberborn: IL2CPP Unity +
MelonLoader runtime + HarmonyLib for patching + Il2CppInterop for
managed bindings. Reference: [`abix-/Schedule1Mods/EmployeeReset`](https://github.com/abix-/Schedule1Mods).

### Mod entry point

```csharp
using HarmonyLib;
using Il2CppInterop.Runtime.InteropTypes;
using MelonLoader;
using Il2CppScheduleOne.NPCs.Behaviour;

namespace EmployeeReset;

public class Mod : MelonMod
{
    private const string PrefCategory = "EmployeeReset";
    private static MelonPreferences_Category _prefs;
    private static MelonPreferences_Entry<KeyCode> _hotkeyPref;

    public override void OnInitializeMelon()
    {
        _prefs = MelonPreferences.CreateCategory(PrefCategory, "Display Name");
        _hotkeyPref = _prefs.CreateEntry("Hotkey", KeyCode.F8,
            "Hotkey", "Press to ...");

        TryPatchSomething();
        MelonLogger.Msg($"{nameof(EmployeeReset)} initialized");
    }
}
```

- `class Mod : MelonMod`, `OnInitializeMelon` override is the entry.
- `MelonPreferences` for user config. Each entry is typed; the value
  persists to `UserData/MelonPreferences.cfg`.
- `MelonLogger.Msg` / `Warning` / `Error` for output. Routes to the
  MelonLoader console with the mod's color.

### Manual Harmony patching

Prefer manual `HarmonyInstance.Patch(...)` over `[HarmonyPatch]`
attributes when the target type is in an Il2Cpp namespace, because
attribute-based patches resolve at load time and can't easily handle
type-not-found gracefully. Manual patches let you wrap in try/catch:

```csharp
private void TryPatchCanStartMix()
{
    try
    {
        MethodInfo target = typeof(MixingStation).GetMethod(
            "CanStartMix", BindingFlags.Public | BindingFlags.Instance);
        if (target == null)
        {
            MelonLogger.Warning("CanStartMix not found; skipping patch");
            return;
        }
        MethodInfo postfix = typeof(Mod).GetMethod(
            nameof(CanStartMixIngredientGatePostfix),
            BindingFlags.NonPublic | BindingFlags.Static);
        HarmonyInstance.Patch(target, postfix: new HarmonyMethod(postfix));
    }
    catch (Exception e)
    {
        MelonLogger.Error($"failed to patch CanStartMix: {e}");
    }
}

private static void CanStartMixIngredientGatePostfix(
    MixingStation __instance, ref bool __result)
{
    if (!__result) return;  // already false; nothing to add
    if (SomeExtraCheck(__instance)) return;
    __result = false;
}
```

- `__instance` is the target object. `__result` is the return value,
  passed by ref so a postfix can mutate it.
- Static patch methods only. Harmony can't call instance methods.
- `BindingFlags.NonPublic | BindingFlags.Static` to discover the
  postfix on your own type.
- Wrap every patch attempt in try/catch with a Warning log; one
  broken patch should never break the rest of the mod.

### Il2CppInterop quirks

- Game types live under `Il2CppScheduleOne.*` namespaces. Use
  `using Il2CppScheduleOne.X.Y;` and import individually.
- Name clashes: `Il2CppScheduleOne.NPCs.Behaviour.Behaviour` collides
  with `UnityEngine.Behaviour`. Alias one:
  ```csharp
  using SBehaviour = Il2CppScheduleOne.NPCs.Behaviour.Behaviour;
  ```
- Arrays from Il2Cpp use `Il2CppReferenceArray<T>` /
  `Il2CppStructArray<T>`. Convert with `.ToArray()` for managed code.
- Field access works via Il2CppInterop's generated bindings; you
  don't need a publicizer like in Timberborn. But fields that are
  private in C# source may need to be reached via reflection if the
  binding skipped them.

### Iteration comments

Schedule 1 modding is empirical: you observe behavior, hypothesize,
patch, repeat. The convention in `EmployeeReset/Mod.cs` is to leave
numbered iteration markers explaining WHY a piece of code is the way
it is, with the failed alternatives:

```csharp
// ITERATION 14: default to FALSE. Recovered IL from Cpp2IL revealed
// that our SmartReset's StopCook and Deactivate calls internally
// invoke MixingStation.SetNPCUser(null), nulling state vanilla
// expects intact. Default off; opt-in for users who genuinely
// need the post-eMployee cleanup.
```

This is doctrine, not bloat. Each iteration note ties a decision to
the empirical finding (Cpp2IL output, F8 telemetry, crash dump) that
forced it. When in doubt, leave the note.

### Logging discipline

- `MelonLogger.Msg` for normal events. One line. Identify the mod.
- `MelonLogger.Warning` for "didn't crash, but we degraded".
- `MelonLogger.Error` for caught exceptions, with `$"{e}"` (the full
  stack). Never swallow without logging.
- Verbose / diagnostic logs go behind a `_verboseLogPref.Value`
  guard. Default off. Turn on for instrumentation passes.

## Auto-Deploy Pattern

Deploy built DLL to a target folder automatically:

```xml
<Target Name="Deploy" AfterTargets="Build">
  <MakeDir Directories="$(TargetDir)" />
  <Copy SourceFiles="$(OutputPath)MyMod.dll" DestinationFolder="$(TargetDir)" />
</Target>
```

## API Design Patterns (Unity Mods)

### Return types: struct not JSON
Internal methods return typed structs. JSON serialization happens ONLY at the HTTP boundary. Never pass JSON strings between internal methods. Use the struct directly.

```csharp
// good: struct for internal use, ToJson() at HTTP boundary
public PlaceBuildingResult PlaceBuilding(...) { return new PlaceBuildingResult { Id = id }; }
// HTTP handler: return result.ToJson(jw);

// bad: returning JSON string, then parsing it with reflection
public object PlaceBuilding(...) { return _jw.Result(("id", id)); }
// caller: result.GetType().GetProperty("id") -- broken on strings
```

### Game validation over custom checks
Trust the game engine's validators (IsValid, BlockValidator). Use them for error reasons instead of reimplementing:
- `BlockValidator.BlockConflictsWithExistingObject`. What's blocking a tile
- `BlockObjectValidationService._blockObjectValidators`. 9 validators with reason strings
- `PreviewFactory.Create` + `Reposition` + `IsValid`. The player's green/red overlay
- `PositionedBlocks.GetAllBlocks()`. World-space blocks after rotation, no manual rotation math

### Data formats: toon vs json
All list endpoints accept `format` param. Server writes different output per format:
- toon: flat strings, optional fields omitted, token-efficient for LLM
- json: nested objects, all fields present, for programmatic access
