---
name: k3sc
description: k3sc Go binary -- Claude agent operator, CLI, and TUI. Use when developing k3sc features, fixing bugs, adding commands, or modifying the operator/scanner/controller.
version: "1.0"
---

# k3sc

Go binary that manages Claude Code agents as k8s pods. One binary serves as both the in-cluster operator and the Windows CLI.

- **Repo**: `C:\code\k3sc`
- **Branch**: `master`
- **Build**: `cd /c/code/k3sc && go build -o k3sc.exe .`
- **Test**: `cd /c/code/k3sc && go test ./...`
- **Lint**: `cd /c/code/k3sc && go vet ./...`

## Package layout

```
cmd/              # cobra subcommands (one file per command)
internal/
  operator/       # controller + scanner (runs in-cluster)
    controller.go # reconciles AgentJob CRDs through state machine
    scanner.go    # polls GitHub, creates AgentJobs, orphan cleanup, usage-limit
    types.go      # AgentJob CRD types, TaskPhase enum
  github/         # GitHub API client (issues, labels, PRs)
  k8s/            # k8s client helpers (pods, jobs, logs, templates)
  dispatch/       # slot allocation, template loading
  tui/            # bubbletea TUI for `k3sc top`
  config/         # ~/.k3sc.yaml config loading
  types/          # shared types (Repo, AgentPod, TaskInfo)
  format/         # output formatting helpers
image/            # Dockerfile + entrypoint for agent pods
manifests/        # k8s manifests (CRD, operator deployment, job template)
```

## Operator architecture

The operator runs as a k8s deployment (`k3sc operator`). Two components:

**Scanner** (`scanner.go`): goroutine polling GitHub on a timer (2min base, exponential backoff to 1hr).
- Fetches eligible issues (`ready` + `needs-review`)
- Creates AgentJob CRDs for unclaimed issues
- Orphan cleanup: detects owner labels with no active pod/job
- Usage-limit detection: skips dispatch if a pod recently hit Claude rate limits
- TTL cleanup: deletes terminal AgentJobs after 24h

**Controller** (`controller.go`): reconciles AgentJob CRDs through phases:
```
Pending -> Assigned -> Running -> Succeeded/Failed -> (Reported)
```
- Pending: assign slot + agent name
- Assigned: claim issue on GitHub, create k8s Job
- Running: poll job status, detect success/failure
- Succeeded/Failed: post status comment, transition GitHub labels, mark reported

## State machine (GitHub labels)

```
ready -> claude-{letter} -> needs-review -> claude-{letter} -> needs-human
```

Label transitions in `handleCompleted`:
- Success from `ready` (OriginState) -> `needs-review`
- Success from `needs-review` -> `needs-human`
- Failure from `ready` -> `ready` (retry, max 3)
- Failure from `needs-review` -> `needs-human` (escalate, don't loop)

`OriginState` is captured when the scanner creates the AgentJob from the issue's current label state.

## cargo-lock

`k3sc cargo-lock <subcommand> [args]` wraps cargo with a file lock so concurrent agents don't clobber shared target dirs.

- Auto-detects `--manifest-path` (inserts before `--` for test/run)
- `run` and `test` build first under lock, then execute
- Build args strip everything after `--` (test filters, app args)
- Lock file: `$CARGO_TARGET_DIR/.cargo-build.lock`

## Deploy cycle

1. Build Windows binary: `go build -o k3sc.exe .`
2. Cross-compile linux: `GOOS=linux GOARCH=amd64 go build -o image/k3sc .`
3. Build image via WSL nerdctl (into k3s containerd namespace)
4. `kubectl rollout restart deployment k3sc-operator -n claude-agents`

## Key patterns

- **cobra** for CLI subcommands
- **controller-runtime** for the operator (CRD reconciler)
- **bubbletea** for TUI
- **go-github** for GitHub API
- Config via `~/.k3sc.yaml` merged with defaults
- Agent identity: slot number -> letter (1=a, 2=b, ..., 26=z)
- All kubectl commands go through WSL wrapper (not on Windows PATH)

## Testing

- Unit tests alongside source files (`*_test.go`)
- k8s functions that need a cluster are not unit tested -- verify manually
- `go test ./...` must pass before commit
