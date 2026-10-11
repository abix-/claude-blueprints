---
name: "help"
description: "List all available slash commands grouped by workflow stage"
---
# Slash Command Reference

## Recording and running work

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/onelog` | Record and handle all work: todo rows, docs/changelog.md, the daily page, cleanup, finishing work (endless's docs and changelog checklist included) | Starting or finishing work, `/todo`, `/changelog`, `/done` |
| `/orchestrator` | Run several agents for the operator's design: the day's plan, one agent per project, checking every report; a `/mygoal` game goal | Starting the day, or making this session the orchestrator |
| `/debate` | Two sessions review one design by message | A design worth a second session's review |

The GitHub issue skills (`/issue`, `/n`, `/review`, `/approve`, `/reject`) are deprecated and kept in claude-blueprints `archive/skills/`.

## Build & Run

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/endless` | Build and run the game | Launch the game locally |
| `/test` | Build, launch with --autostart, verify via BRP | Automated smoke test |
| `/debug` | Check compiler errors and runtime logs | Something broke |
| `/benchmark` | Run Criterion benchmarks, record to performance.md | Perf work or baseline collection |
| `/dev` | Trigger GitHub Actions dev build | Push a dev channel build |
| `/dist` | Build release and package for distribution | Ship a release build |
| `/release` | Create GitHub release from CHANGELOG | Tag and publish a release |

## Game Interaction (requires running game)

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/entity <uid>` | Inspect a Bevy entity via BRP | Debug a specific entity |
| `/endless-cli` | Load BRP command reference | Need to query/control the running game |

## Infrastructure & Ops

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/ctop` | Dashboard of agent pods, issues, cluster health | Check what agents are doing |
| `/deps` | Check Rust dependencies for updates | Dependency maintenance |
| `/fix-auth` | Restore .claude.json from auto-backup | Auth token corrupted |
| `/load` | Pull claude-blueprints and install the current runtime | Sync skills and runtime config from repo |

## Research & Quality

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/rtfm` | Search for existing solutions before building | Before writing new code |
| `/simplify` | Review changed code for reuse, quality, efficiency | After writing code, before commit |
| `/1note` | Read and search OneNote notebooks | Need info from OneNote |

## Meta & Config

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/obey` | Re-read the current runtime instructions and confirm compliance | Session start or trust verification |
| `/learn` | Review conversation and update skills | After discovering something worth remembering |
| `/why` | Trace why Claude made its previous response | Debug unexpected behavior |
| `/kovarex` | Brutally honest project review | Reality check on roadmap and priorities |
| `/loop <interval> <cmd>` | Run a slash command on a recurring interval | Poll status, recurring checks |
| `/debloat` | Strip Windows bloatware, disable telemetry | Clean up Windows |
