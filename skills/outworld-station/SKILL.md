---
name: outworld-station
description: Modding Outworld Station (Spacescape Salvage Station, UE 5.4.4 + UE4SS). Authoritative on OWS-specific facts -- the Steam exe, image-relative offsets, GObjects layout (WrappedChunked), DataTable cache-propagation gotcha, deploy folder, the shipped feature inventory. Mod code is the `outworld-station-tweaks` crate in `abix-/Grounded2Mods`. For ueforge framework doctrine, read the `ueforge` skill. Not for playing the game.
user-invocable: false
version: "1.0"
updated: "2026-05-11"
---
# Outworld Station -- modding

Per-game modding skill for **Outworld Station** (UE 5.4.4 +
UE4SS). Authoritative on what is specific to this game: exe,
image-relative offsets, the GObjects layout variant, the
DataTable mutation-timing finding, deploy folder. Framework
doctrine (`ueforge`) is a separate skill -- no overlap.

The mod is a multi-feature scaffold (`outworld-station-tweaks`),
not single-purpose. First feature live: 4x item stack tweak
(`DT_Materials.MaxCanStack`). Future features layer onto the same
scaffold (hunger / thirst / inventory / movement tweaks, all
driven through DataTable mutations + CDO writes).

Repo: `abix-/Grounded2Mods` (same workspace as Grounded 2; the
mods share the ueforge framework). Crate path:
`outworld-station-tweaks/`.

## Where things live (index)

| Path (repo-relative)                       | What lives there                                      |
| ------------------------------------------ | ----------------------------------------------------- |
| `outworld-station-tweaks/Cargo.toml`       | `[package.metadata.ueforge]` deploy config (mod_folder_name, game_name_regex, etc.) |
| `outworld-station-tweaks/build.rs`         | `ueforge::build::CppShim::new().compile()`            |
| `outworld-station-tweaks/src/lib.rs`       | `MOD_INFO: ueforge::ModDef` + `PLATFORMS` table + on_unreal_init / on_shutdown |
| `outworld-station-tweaks/src/stacks.rs`    | DT_Materials worker (the shipped feature)             |
| `outworld-station-tweaks/src/debug.rs`     | Game-specific debug ops + Snapshot type               |
| `outworld-station-tweaks/src/settings.rs`  | Settings serde structs                                |
| `outworld-station-tweaks/docs/research.md` | Bootstrap status, DataTable cache-propagation finding, per-feature plans |

## Game facts (the OWS specifics)

Tested against Steam build (UE 5.4.4, exe `FileVersion`
verified). Image base captured by UE4SS pattern scanner on
2026-05-09; image-relative offsets computed by subtracting the
base.

- **Steam exe**: `OutworldStation-Win64-Shipping.exe`.
- **Steam install layout** (per `cargo deploy` autodetect):
  `<game-root>/OutworldStation/Binaries/Win64/`.
- **Image-relative offsets**:
  - GObjects (`GUObjectArray`): `0x07A938D0`
  - `AppendString` (`FName::ToString`): `0x010DF9D0`
  - `ProcessEvent`: `0x012AF540`
  - `ProcessEventIdx`: `0x4C` (stable across UE 5.x)
- **GObjects layout**: `GObjectsLayout::WrappedChunked` --
  `FUObjectArray` wraps `FChunkedFixedUObjectArray` at `+0x10`.
  Verified live: `NumElements=142650`, `NumChunks=3`.
- **`g_names` / `g_world`**: not yet logged by UE4SS scanner on
  this exe. Not required by `walk_class` / `read_bytes` /
  `write_bytes` / `call`; fill in only if a feature needs them.
- **`FSMaterialData::MaxCanStack`** offset: `+0x48` within the
  row struct (per the SDK dump's `SpaceSalvageStation.hpp`).

## DataTable cache-propagation finding (load-bearing)

**The OWS-specific lesson that drives every DT-backed feature here.**

Per UE4SS docs, DataTable reads return **copies** of the row
struct. UI widgets like `UI_Item.MaterialData` (offset 0x3C0,
size 0x170) hold their own copy by value, populated at widget
creation. **Mutating the DT after widgets exist leaves them
stale.**

Empirical evidence (`docs/research.md`):
- Mutate `DT_Materials.MaxCanStack` 4x AFTER a save loads -> the
  in-memory DT shows the new value, but inventory tooltips
  still show the old value (widgets cached at slot spawn).
- Mutate from `on_unreal_init` (before any save loads) ->
  tooltips show the new value from the start.

This is why the community "Better Item Stacks" pak mod chose a
`_P` pak: paks override `.uasset` on disk so the DT loads with
modded values, no caching window. The Rust/DLL approach matches
that by mutating early enough that no widget has copied yet.

### The reusable pattern (DT-backed value tweaks)

1. Find the DataTable + row-struct field offset (SDK dump grep).
2. Spawn a worker from `on_unreal_init` that polls for the DT
   (up to ~30 s -- DTs can lazy-load).
3. On first sight, write all rows.
4. One pass is enough -- DT memory persists for the session;
   later widget creations cache the mutated values.

Belt-and-suspenders for lazy-loaded DTs: hook
`UFunctionWeWantToOverride` via UE4SS-style RegisterPostHook and
override the return value directly. Bulletproof against any
caching path. Backlog item -- not needed for stacks since
on-init mutation works for OWS.

ueforge's `ueforge::stacks` module wraps exactly this pattern:
`StackDef` + `StackRegistry` + on-first-sight worker + idempotent
re-apply + captured-vanilla baseline. OWS's `src/stacks.rs`
shrinks to one `StackDef` literal + register.

## Bootstrap status (per-game checklist)

Per the `ueforge/README.md` "Bootstrapping a new game" checklist:

| Item                                | Status         | Detail                                            |
| ----------------------------------- | -------------- | ------------------------------------------------- |
| Engine version known                | yes            | UE 5.4.4 (exe `FileVersion`)                      |
| Anti-cheat                          | none observed  | no EAC / BattlEye folders in install              |
| UE4SS installed                     | yes            | `OutworldStation/Binaries/Win64/ue4ss/` -- main HEAD commit `06474186`, built 2026-05-08 |
| `UE4SS.lib` regenerated             | yes            | ~4063 exports; tracked in `abix-/Grounded2Mods`   |
| Mod scaffold                        | yes            | `outworld-station-tweaks/`                        |
| `/debug` endpoint                   | yes            | `127.0.0.1:17172`, smoke test passes 3/3          |
| SDK dump                            | yes            | `OutworldStation/Binaries/Win64/ue4ss/CXXHeaderDump/` (956 .hpp files, 8.4 MB) |
| `PlatformOffsets` filled in         | yes            | values above                                      |
| First mod target                    | done           | stack tweaks                                      |

Until the SDK dump exists, `walk_class` / `read_bytes` /
`write_bytes` / `call` all error with
`"ueforge: ue runtime not initialized"`. The control plane works;
it just can't see GObjects.

## Build + deploy

Same `cargo deploy` flow as every ueforge consumer:

```sh
cargo deploy install   -p outworld-station-tweaks      # dev iteration
cargo deploy package   -p outworld-station-tweaks      # zip for distribution
cargo deploy uninstall -p outworld-station-tweaks
```

`cargo deploy` autodetects the game root by walking Steam libraries
for a directory matching `game_name_regex = "Outworld"` containing
`game_sub_path = "OutworldStation/Binaries/Win64"`. Drops the DLL
at `<game-root>/OutworldStation/Binaries/Win64/ue4ss/Mods/OutworldStationTweaks/dlls/main.dll`.

Per-mod target dir: `target/outworld-station-tweaks/` (keeps it
from colliding with the other workspace cdylib).

## Tabs (ImGui)

OWS-Tweaks registers (current state -- see `src/lib.rs` for the
live list):

- **Tweaks** -- the OWS-specific feature controls
- **Scanner** -- ueforge's built-in Cheat-Engine-style scanner
  (`ueforge::ui_scanner::render`)
- **Tables** / **Classes** / **Structs** -- ueforge's browsers
  reading the discovery cache

The browsers are reference consumers of ueforge's discovery
machinery; they're useful for OWS because the SDK dump is
present and discovery walks succeed.

## Debug HTTP endpoint

`POST 127.0.0.1:17172/debug` -- see the `runtime-control-http`
skill for the pattern. OWS-specific ops registered from
`src/debug.rs` into `OP_REGISTRY`; built-in ops
(`read_bytes`/`write_bytes`/`walk_class`/`discover_*`/`scan_*`/
`freeze*`) come from ueforge.

## Session etiquette

- Public repo. Generic content -- no machine-specific paths.
- Read `outworld-station-tweaks/docs/research.md` first when
  adding a new feature; it carries the per-game lessons learned.
- For framework doctrine (composition model, k8s pattern,
  hot reload, discovery, modules), read the `ueforge` skill.
- For the HTTP debug endpoint, read `runtime-control-http`.
- ASCII source/docs/commits; commits lowercase, push immediately.
- **Never run the game yourself** -- no GPU, no display from
  the agent. Mark unverified work "untested" and stop.
