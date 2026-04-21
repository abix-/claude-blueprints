---
description: List all available slash commands grouped by workflow stage
disable-model-invocation: false
user-invocable: true
---
# Slash Command Reference

## AI Workflow (issue -> code -> review -> merge)

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/issue [N]` | Create issues with `ready` label, or claim and implement an issue | Create bugs/features or start work on an issue |
| `/done` | Update docs, changelog, commit, push. Also runs as part of /review when docs are missing | After finishing feature work, or standalone |
| `/review [repo] [N]` | Full review with all gates: compliance, acceptance, spec, DRY, regression tests, benchmarks | Review a PR before merge -- all review logic lives here |
| `/approve [N]` | Merge a PR that passed /review | After /review says ready to merge |
| `/reject [N]` | Close failed PR, comment on issue, reset | After /review says needs work |
| `/n` | Peek at next item needing review (read-only, no reservation) | Quick look at what needs attention |

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
| `/load` | Pull claude-blueprints repo and apply to ~/.claude | Sync skills/config from repo |

## Research & Quality

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/rtfm` | Search for existing solutions before building | Before writing new code |
| `/simplify` | Review changed code for reuse, quality, efficiency | After writing code, before commit |
| `/1note` | Read and search OneNote notebooks | Need info from OneNote |

## Meta & Config

| Command | What it does | When to use |
|---------|-------------|-------------|
| `/obey` | Re-read CLAUDE.md and confirm compliance | Session start or trust verification |
| `/learn` | Review conversation and update skills | After discovering something worth remembering |
| `/why` | Trace why Claude made its previous response | Debug unexpected behavior |
| `/kovarex` | Brutally honest project review | Reality check on roadmap and priorities |
| `/loop <interval> <cmd>` | Run a slash command on a recurring interval | Poll status, recurring checks |
| `/debloat` | Strip Windows bloatware, disable telemetry | Clean up Windows |
