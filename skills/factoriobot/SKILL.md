---
name: factoriobot
description: factoriobot development and operation. Use when writing factoriobot Rust code, driving its CLI against a live Factorio game, or extending its monitors. Not for playing Factorio by hand.
user-invocable: false
version: "1.4"
---
# Factoriobot

AI-assisted Factorio partner. Rust binary + RCON against the player's hosted Factorio 2.x (Space Age) game. Repo: [abix-/factoriobot](https://github.com/abix-/factoriobot) (private). Read docs/factorio-design.md (the ONE living design doc) before design work. Three parts: the Rust brain offloads as much as possible (deterministic monitors, proposals, execution), the LLM only judges what rules cannot, the player is final authority and the body. One task at a time: at most one active proposal, verified done from game state before the next.

## The shape

- One Rust binary, two roles: CLI subcommands now (ping, status), long-running watch mode later.
- Every read is one RCON round trip: a Lua IIFE string wrapped as `/silent-command rcon.print(helpers.table_to_json(<iife>))`, JSON back, serde into typed structs in src/state.rs.
- Claude Code is the judgment layer and drives the CLI through Bash. No MCP, no Python client.
- Framework: six loops (resource gathering, resource transit, manufacturing, power, research, defense). Each loop gets state readers, deterministic health checks, and next-step logic. Later game phases deepen loops, never add new structure.

## Locked rules

- Writes are player-legal actions gated by proposals (approve, reject, auto per category in chat). The lazy player principle: the bot does everything a UI click could do; the player is an approval and design gate plus the physical residue.
- Hard no-cheating line. Post-v1 hands place blueprints exactly as a player would.
- Any modded game must work: game knowledge from prototype data at runtime, never hardcoded vanilla lists.
- Spaghetti is the player's initial base design. Advice works within whatever exists, never pushes a layout.
- RCON is localhost only. Password via FACTORIOBOT_RCON_PASSWORD env or --password, never committed.
- No arbitrary-execution command in the shipped CLI surface.

## Repo layout

- src/main.rs clap CLI, src/lib.rs module exports
- src/rcon.rs connect + execute_lua_json (lifted from factorio-sensei, MIT, see THIRDPARTY.md)
- src/lua.rs IIFE reader builders, src/state.rs Deserialize structs, src/error.rs
- tests/live.rs live tests behind #[ignore]
- docs/ factorio-vision.md, factorio-research.md, factorio-design.md (the living design), factorio-v1-plan-*.md (execution history)
- .claude/project_state.md current focus and next steps

## Commands

- Build and test: `k3sc cargo-lock check | test | build --release`, never bare cargo. After a release build, copy the exe from the shared target dir to the user's bin dir on PATH. A running watch locks the exe; stop it before rebuilding.
- Live tests, game must be hosted: `k3sc cargo-lock test -- --ignored`
- CLI: `factoriobot ping | status | problems | next | watch`. Default address 127.0.0.1:27015. `problems` is the one-shot six-loop health check, `next` is the deterministic what-should-I-do-next (priority: defense, power, research, manufacturing, gathering, transit), `watch` polls (10s fast, 300s slow), latches alerts (fire on start, fire on clear, never repeat), and delivers to stdout plus in-game chat.
- Game setup: in Factorio's config.ini [other] section, uncomment local-rcon-socket and local-rcon-password, then host via Multiplayer, Host New Game. RCON listens only while hosting, including solo.

## Companion mod

- Lives at mod/factoriobot (info.json + control.lua), installed by copying that folder into the game's mods directory. factorio_version must match the player's game (currently "2.1", they run experimental).
- Observes and relays only, changes nothing: in-game `/factoriobot <message>` stores to a capped inbox and acks in orange; entity deaths on the player force and finished researches store to a capped event buffer.
- RCON-only drains: `/factoriobot_poll_inbox` and `/factoriobot_poll_events` return JSON arrays and clear. The daemon polls them each fast tick, degrades gracefully when the mod is absent (warns once, latched conditions keep working).
- Event alerts are one-shot, not latched: deaths group into one "N structures lost near (x, y)" per poll; research completions announce by name.

## Lua reader rules

- IIFE form `(function() ... end)()` returning plain Lua tables only, no userdata.
- Player-dependent readers start with the connected-player check and return {error="no_player"} without one.
- Factorio 2.x dot syntax. helpers.table_to_json is the 2.x name.
- Cap entity result sizes. The lua runs inside the player's game session; its stutter is our fault.
- Surface-aware from day one (Space Age: nauvis, platforms, planets).

## Prior art

- Local clones of every relevant project live in a factorio-refs directory next to the repo checkout. docs/factorio-research.md is the annotated catalog: what is liftable versus ideas-only, with licenses.
- Lifted code: factorio-sensei's rcon wrapper and lua readers (MIT, attributed in THIRDPARTY.md). FLE's action vocabulary is the reference when hands arrive.
- Timberbot ([abix-/TimberbornMods](https://github.com/abix-/TimberbornMods)) is the architectural precedent: mod does mechanics, external brain does judgment, errors written for an AI caller, live test harness.

## Doctrine

- Every command sent to the game gets an expected settle signal. Silent failure is the number one killer.
- Alerts latch: fire once when a condition starts, not on every poll.
- Bound every queue at creation. Stable entity ids (unit_number), never session-scoped ones.
- Errors tell the caller what went wrong AND what to do next, with valid options listed.

## Factorio 2.1 API drift (live-verified 2026-07-19)

- LuaRecipe has no `category` and LuaRecipePrototype renamed `category` to `categories` (array of strings). All prior-art projects (FLE included) predate this. When a reader errors with "doesn't contain key", check lua-api.factorio.com/latest before guessing, and read factoriobot.log: every rcon exchange is in it.
- Trigger technologies (research_trigger on the prototype) cannot be queued with add_research; they complete via in-world actions. Exclude them from research proposals, surface their requirements as goals.

## Live-verified facts (2026-07-18)

- ping and status work end to end against a hosted Space Age game. helpers.table_to_json and the power reader confirmed live (tests/live.rs, run with `-- --ignored`).
- RCON answers with EMPTY responses while the game is still loading a save. Treat an empty response shortly after connect as retry-able, not as a code bug.

## Open items

- RESOLVED: lua empty tables ({} vs []) handled by the lua_array deserializer on every list field.
- Whether RCON silent-commands disable achievements for the save: pending the player checking in-game, record in docs/factorio-design.md.
