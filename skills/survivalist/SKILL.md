---
name: "survivalist"
description: "Modding Survivalist: Invisible Strain (Unity, official StreamingAssets mod support, Harmony). Authoritative on the development loop: where the mod lives, how a tweak reaches the live game, the embedded-Harmony decision, the patch shapes this game's Mono accepts, and the deploy and log paths. Mod code is the survivalist-mod crate in the modforge workspace, on top of unityforge. Not for playing the game."
---
# Survivalist: Invisible Strain

A personal mod for the operator's own game: a simulation of brutal life in a
zombie apocalypse, driven by AI factions that fight, grow, scavenge, and can be
destroyed. Rust crate `survivalist-mod` in the modforge workspace, sitting on
`unityforge` plus the C# shim `unityforge/cs-shim-survivalist`. Not shipped to
the Workshop; the audience is the operator.

## Read these first, they carry the truth

The repo owns the design and the status; this skill owns the procedure. One
authoritative doc per subject, and every score lives in exactly one of them:

- `survivalist-mod/docs/status.md`. THE one place for x/10 scores, the goal,
  the priority order, and what is up next.
- `survivalist-mod/docs/research.md`. How to read and change the live game:
  control plane, Harmony facts, deploy facts. No scores.
- `survivalist-mod/docs/faction-war.md`. The vision, the design decisions, and
  the game-code research behind each pillar. No scores.
- repo `docs/todo.md`. The backlog; it points at status.md for status.

Never open a parallel plan, status, or scores file. Update the owning doc.

## How work is chosen

- **Features first, polish last.** What ranks highest is whatever makes the
  world more fun, more lifelike, and less boring. UI and cosmetics wait until
  the gameplay above them has earned them.
- **The bored-player test.** A change counts as gameplay only if a bored player
  safe behind their walls would sit up: something coming for them, a choice
  that costs them, stakes that rise. Trait drift in a camp across the map is
  simulation the player never sees. If they would not notice, it is scope
  creep, however clever the simulation.
- **Focused completion (operator-locked).** Drive ONE row to 10/10 at a time:
  take the highest-priority row below 10/10, finish it, then move on. No
  jumping between rows, no inventing features that are not already decided in
  status.md.
- **10/10 for this mod** means it could ship to the Workshop and be used by
  1000 players for 10 years with no bug fixes, everybody happy, everything
  working. Score against that bar or do not score.
- **Brutal but survivable is the line.** Pressure exists to make survival hard,
  costly, and meaningful, never to guarantee a wipeout. There must always be a
  road out. Two director layers, decided and never to be merged: Randy Random
  (unpredictable events, rolls what and when, never adaptive) and Mario Kart
  (adaptive pressure on whoever is winning). The storyteller is the umbrella
  over both.
- AI factions must not cheat. Growth, loot, and reinforcements are earned with
  real resources carried by real characters, the same rules the player lives
  under. Free respawns and spawned-in stock are the thing this mod removes.

## The loop

1. Change Rust in `survivalist-mod/src` (one file per pillar: war, growth,
   horde, quality, upgrade, scavenge, storyteller, and so on).
2. Deploy with `survivalist-mod/scripts/build_and_deploy.ps1
   [-ModName SurvivalistTweaks] [-Hot]`.
3. The operator drives the game and reports: "game closed", "loaded",
   "redeploy". Say plainly which of those the change needs. Quit to menu does
   NOT unload mod DLLs; only a story switch does, so a full restart is the
   honest ask unless the change is hot-reloadable.
4. Read the player log for what actually happened:
   `%USERPROFILE%\AppData\LocalLow\Ginormocorp Industries\Survivalist Invisible Strain\Player.log`.
   A failed patch names itself there. Read it before theorising.
5. Update the owning doc's score only from what the log or the operator
   confirms, never from code that was merely written.

The mod folder is
`<game>\Survivalist Invisible Strain_Data\StreamingAssets\<ModName>\`. Mods
created in the game's editor land under StreamingAssets; the setup guide's
"game's directory" wording is imprecise.

## Reading and changing the live game

The control plane answers on port 17173, the same shape as every other modforge
project (see the `runtime-control-http` skill for why this exists first).
`list_singletons` resolves a holder, `read_field` / `write_field` move data,
`inspect_object` dumps live values, and a Harmony patch is for when behaviour
rather than data must change. Op arguments nest under `"args"`.

- Handles reset on hot reload. Re-resolve every time; never persist a handle.
- `walk_class` only finds UnityEngine.Object subclasses. Plain classes need a
  singleton or static path, or a field walk from one.
- The game is plain C#, no obfuscation: decompile before guessing with
  `ilspycmd -t <Type> Assembly-CSharp.dll`.
- Data-only content (items, props, quests, dialog) is XML the in-game editors
  own. A tweak that only changes data belongs in the mod folder as XML, not in
  Rust.

## Harmony on this stack, hard-won

- **The shim embeds Lib.Harmony 2.4.2** (ILRepack, internalized, under the
  shim's own assembly identity). The Workshop "Harmony 2.0.4" is a community
  upload from 2021 that cannot rebuild method bodies containing generic calls;
  patching `Character.AddInjury` died on the copied `List<Injury>.Add` with
  "Invalid IL code in (wrapper dynamic-method)". Shipping a loose newer
  0Harmony.dll does not work either: it is not strong-named, Mono binds the
  plain name to the first-loaded copy, and another mod's 2.0.4 wins. Merging
  under our own identity is normal practice here and collision proof.
- **ILRepack must not merge over `$(TargetPath)`** (the next incremental build
  re-merges the merged dll and dies on duplicate types): output to `merged/`
  and deploy that. The merge target lives in the project-local
  `ILRepack.targets`, not the csproj, or the task runs twice and feeds the shim
  in twice.
- **Check `class` against `struct` before patching around an argument.** An
  indexed `object __N` binds a reference-type argument directly, but a value
  type arrives as a boxed COPY and mutations are silently lost. `Injury` is a
  struct, and the "working" infection patch mutated a copy while the operator
  got infected by a live bite. Use the bridge's Args0 kind for value types; the
  shim now refuses plain arg0 on a value-type first argument so this cannot
  ship again.
- **This game's Mono cannot resolve `__originalMethod`.** It emits `Ldtoken` +
  `MethodBase.GetMethodFromHandle`, which fails inside the dynamic wrapper at
  patch time. Patches route through pre-compiled static slot methods whose
  signatures use only token-free shapes: `()`, `(object __instance)`,
  `(object __0)`. Those are the shapes the game's working mods use too.
- **Resolve types namespace-qualified first.** Unity ships colliding short
  names: `UnityEngine.TextCore.Character` shadowed the game's global
  `Character` and silently killed a patch, returning handle 0 with no
  exception. Exact match across all assemblies first, short-name scan only as
  fallback, and a loud error on every miss.
- **Do not guess at Harmony surface.** Verify against the pardeike Harmony
  source at the tag being used, or against a working Survivalist mod (the dev's
  example mods, DisableHUD, SISLootRespawn). This game has official mod support
  and plenty of working examples; build on them.
- Register shutdown handlers for anything long lived. A hot reload once left
  the previous generation's HTTP listener holding the port and answering from a
  stale op registry because the shutdown registry ran empty.

## Reporting

Say what the log or the operator confirmed, and say what is only written. "The
patch installs" and "the bite left no infection" are different claims, and only
the second one closes the infection row.
