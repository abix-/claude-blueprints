---
name: schedule1
description: Modding Schedule 1 (TVGS, IL2CPP Unity + MelonLoader + Harmony). Authoritative on Schedule 1 game specifics: engine type, MelonLoader/Il2CppInterop references, eMployee mod root-cause findings, vanilla CookRoutine + StartMixingStationBehaviour internals, certainty-tracking discipline. Mod code lives in [`abix-/Schedule1Mods`](https://github.com/abix-/Schedule1Mods) (the `EmployeeReset` sidecar is the current shipped mod). Not for playing the game.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# Schedule 1: modding

Per-game modding skill for **Schedule 1** (TVGS). Engine is
**Unity IL2CPP**: mods load via **MelonLoader** and patch via
**Harmony**. This is a fundamentally different runtime from the
UE5 + UE4SS games (Grounded 2, Outworld Station): no UE4SS, no
ueforge, no Rust cdylib. Mods are C# DLLs (.NET 6) referencing
the IL2CPP-interop assemblies that MelonLoader generates from
the game's IL2CPP metadata.

Repo: [`abix-/Schedule1Mods`](https://github.com/abix-/Schedule1Mods). Current shipped mod: `EmployeeReset`
(a sidecar that fixes two chemist bugs caused by the
community `eMployee` v2.2.4 mod).

## Where things live (index)

| Path (repo-relative)                       | What lives there                                      |
| ------------------------------------------ | ----------------------------------------------------- |
| `README.md`                                | Mod inventory + build instructions + status table     |
| `EmployeeReset/EmployeeReset.csproj`       | Project file; refs MelonLoader / 0Harmony / Il2CppInterop |
| `EmployeeReset/src/Mod.cs`                 | The mod. MelonMod class, Harmony patches, F8 hotkey |
| `EmployeeReset/README.md`                  | User-facing install/config + technical notes          |
| `docs/employee-mod-bug-analysis.md`        | Full root-cause analysis of two stuck-chemist bug classes (line-cited into `eMployee.dll` v2.2.4 + Schedule 1 0.4.5f2) + upstream patch proposal |
| `docs/ingredient-gate-fix-plan.md`         | Implementation plan for the ingredient-availability fix (predicate design, Harmony wiring, validation matrix) |
| `docs/certainty-tracking.md`               | Verification matrix: which claims are empirically proven vs hypothesis; what evidence closes each gap |

## Tech stack

- **Engine**: Unity IL2CPP. Game binary is native; managed code
  is AOT-compiled to native and re-exposed to C# via
  `Il2CppInterop`.
- **Mod loader**: **MelonLoader**. Drops `MelonLoader/` next to
  the game exe; mods go in `<game-root>/Mods/`.
- **Patching**: **Harmony** (`0Harmony.dll` shipped with
  MelonLoader). Use `[HarmonyPatch(...)]` + Prefix / Postfix /
  Transpiler.
- **Target framework**: `net6.0`.
- **Game version under analysis**: Schedule 1 `0.4.5f2`.
- **eMployee mod version under analysis**: `2.2.4`.

## Game install layout

`cargo deploy` doesn't apply here (that's the UE4SS world). Build
+ install for Schedule 1 mods:

```powershell
# from a mod's folder, e.g. EmployeeReset/
dotnet build -c Release
# output DLL: bin/Release/net6.0/<ModName>.dll
# copy to: <game-root>/Mods/
```

The `csproj` defaults to a Steam install path and exposes a
`GameDir` MSBuild property: override on the CLI for non-default
installs:

```powershell
dotnet build -c Release -p:GameDir="D:\Games\Schedule I"
```

References resolve from `$(GameDir)/MelonLoader/net6/` and
`$(GameDir)/MelonLoader/Il2CppAssemblies/`. The repo does NOT
vendor IL2CPP assemblies (license + per-version drift); a local
MelonLoader install is required to build.

## EmployeeReset: what it patches

Two distinct chemist failure modes, both caused by the
community `eMployee` mod v2.2.4. **Read
`docs/employee-mod-bug-analysis.md` before changing anything**
-- it carries the line-cited root cause and the upstream patch
proposal.

### Symptom A: save/load NullReferenceException

`NullReferenceException` at
`StartMixingStationBehaviour+<<StartCook>g__CookRoutine|13_0>d.MoveNext`
fires every ~30 s after save load while a chemist is wedged.

**Root cause**: `eMployee`'s `ResetEmployeeCore` doesn't stop
coroutines. The vanilla `CookRoutine` keeps running with stale
station state; its `MoveNext` dereferences fields the reset
nuked.

**Fix in EmployeeReset**: hard-disable the eMployee Postfix that
re-registers the broken reset path; provide our own reset that
calls `StopCook()` first.

### Symptom B: mid-cook wedge on ingredient exhaustion

Vanilla `CookRoutine` yields on a condition (next ingredient
available) that becomes unreachable mid-cook. `eMployee`'s
AUTO-RESET fires three times then abandons; chemist stays
wedged.

**Fix in EmployeeReset**: **Harmony postfix on
`StartMixingStationBehaviour.CanCookStart`** that gates the
predicate on **ingredient availability**. The canonical gate is
"check input slot quantity via `GetMixQuantity`": discovered
via cross-reference to vanilla's own `ProduceMore` path, which
does exactly that check before queueing.

**The canonical gate** (from `docs/employee-mod-bug-analysis.md`
"For the eMployee mod author"): a predicate that returns
`bool CanCookStart` iff every required input slot has
`GetMixQuantity(...) >= required`. Approaches rejected and why
are documented in the same section: do NOT reimplement those
without re-reading the analysis.

### Smart-reset (F8 hotkey)

`F8` triggers a comprehensive reset that:
1. Stops all `CookRoutine` instances.
2. Disables the mixing-station behaviour cleanly.
3. Re-enables and clears stale state.
4. Logs each step to MelonLoader's console.

The reset is **deliberately scoped wider** than `eMployee`'s:
the analysis (Fix 3) shows the narrow reset is what made
`eMployee` give up after 3 retries.

## Reverse-engineering discipline: certainty tracking

`docs/certainty-tracking.md` is the **mandatory** verification
matrix. Each claim about vanilla game internals carries:

- **Status**: empirically proven, evidence-cited, or hypothesis.
- **Evidence**: log slice, dnSpy citation, in-game observation,
  cpp2il-recovered signature.
- **Gap**: what would close it.

The discipline exists because IL2CPP reverse-engineering produces
plausible-but-wrong claims constantly (cpp2il recovers
*signatures*, not bodies; Harmony patches that compile fine still
miss IL2CPP boxing/unboxing layers). **Never** assert a vanilla
behaviour from cpp2il alone: always pair with at least one
in-game observation OR a dnSpy citation against the MelonLoader-
generated Il2CppInterop assemblies.

When working on a new symptom:
1. Reproduce in-game; capture the MelonLoader log slice.
2. Trace the failing frame back to a method name via the log's
   coroutine state-machine names.
3. Cross-reference cpp2il output AND the Il2CppInterop assemblies
   to confirm the method exists and its signature is what you
   expect.
4. Update `docs/certainty-tracking.md` with the new evidence
   before patching.

## Harmony patterns for this game

- **Postfix over Prefix** when you want vanilla behaviour PLUS a
  gate. `CanCookStart` is the canonical example: vanilla
  computes a bool, we AND it with our ingredient check.
- **Prefix returning bool** to block vanilla: use sparingly,
  it's a stronger contract that breaks if vanilla's signature
  shifts.
- **Field-name resilience**: IL2CPP-generated field names can
  shift across game versions. Wrap field reads in
  `try { ... } catch { /* fall back */ }` and log the failure,
  rather than throwing. `docs/employee-mod-bug-analysis.md`
  "Field-name resilience" has the canonical pattern.
- **Coroutine state-machine class names** (`<<StartCook>g__CookRoutine|13_0>d`)
  are the IL2CPP-mangled form of the C# compiler's coroutine
  closures. They're version-stable for a given vanilla version
  but can shift on update; defensive Harmony attribute patterns
  in `Mod.cs` show how to handle that.

## Bootstrap status (current shipped mod)

| Item                                | Status         | Detail                                            |
| ----------------------------------- | -------------- | ------------------------------------------------- |
| Engine known                        | yes            | Unity IL2CPP                                      |
| Mod loader installed                | yes            | MelonLoader (per user)                            |
| `Il2CppInterop` assemblies          | yes            | from MelonLoader generation                       |
| Game version under analysis         | yes            | `0.4.5f2`                                         |
| `eMployee` version under analysis   | yes            | `2.2.4`                                           |
| Symptom A fix                       | verified once  | one in-game test                                  |
| Symptom B fix (ingredient gate)     | built          | awaiting in-game verification                     |
| Certainty tracking discipline       | active         | `docs/certainty-tracking.md`                      |

## Sibling mods (future)

The repo is named `Schedule1Mods` plural deliberately: room
for additional sidecar mods. Each new mod gets its own subdir +
`.csproj`, copies the build pattern from `EmployeeReset`, and
adds a row to the top-level `README.md` status table.

## Session etiquette

- Public repo. No IL2CPP assemblies vendored (license; per-version
  drift). Build requires user's local MelonLoader install.
- Read `docs/employee-mod-bug-analysis.md` before changing the
  employee fixes: it's the spec.
- Update `docs/certainty-tracking.md` whenever a new claim is
  asserted about vanilla. Hypotheses are tracked; assertions are
  evidence-cited.
- ASCII source/docs/commits; commits lowercase, push immediately.
- **Never run the game yourself**: no GPU, no display from the
  agent. Mark unverified work "untested" and stop.
- This is a fundamentally different stack from the UE4SS games.
  Do not import patterns from the `ueforge` skill here: the
  `ueforge` skill is for UE4SS Rust mods, not MelonLoader C# mods.
