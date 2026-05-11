---
name: golang
description: Go development standards. Use when writing Go. Sourced from k3sc, the canonical reference repo.
user-invocable: false
version: "2.0"
updated: "2026-05-11"
---
# Go

Source repo: [`abix-/k3sc`](https://github.com/abix-/k3sc) (12K LOC, Go 1.25). When in doubt, match how
k3sc does it. Quoted examples below are real file paths in that repo.

## Project layout

```
<repo>/
  main.go              # 3 lines: package main, import cmd, cmd.Execute()
  cmd/                 # one .go file per subcommand, all `package cmd`
    root.go            # rootCmd, init(), Execute()
    <subcmd>.go        # one file per subcommand
    <subcmd>_test.go   # co-located tests
  internal/            # everything else, one subdir per domain
    github/
    k8s/
    config/
    types/             # shared types only
    ...
```

- `main.go` does nothing but call `cmd.Execute()`. See `k3sc/main.go`.
- Subcommands register themselves in their own `init()`. No central registry.
- `internal/types/types.go` holds cross-package types and package-level
  vars (`var Namespace = "..."`) set at startup. Other packages import
  it but not the other way around.
- Avoid deep nesting. Flat package structure under `internal/`.

## CLI: cobra + pflag

Standard pattern per subcommand:

```go
package cmd

import (
    "github.com/spf13/cobra"
)

var claimRepo string
var claimOwner string

func init() {
    claimCmd.Flags().StringVar(&claimRepo, "repo", "endless", "repo name")
    claimCmd.Flags().StringVar(&claimOwner, "owner", "human", "owner label")
    rootCmd.AddCommand(claimCmd)
}

var claimCmd = &cobra.Command{
    Use:   "claim <issue>",
    Short: "Claim a GitHub issue or PR",
    Args:  cobra.ExactArgs(1),
    RunE:  runClaim,
}

func runClaim(cmd *cobra.Command, args []string) error {
    ctx := cmd.Context()
    // ...
}
```

- Use `RunE`, never `Run`. Errors propagate to `Execute()` which prints
  and exits 1. See `k3sc/cmd/root.go`.
- Flag vars live in the same file as the command, lowercase package-level.
- Pull context from `cmd.Context()`, pass as first arg to every IO call.
- `cobra.ExactArgs(N)` / `cobra.MinimumNArgs(N)` for arity. Don't hand-roll.

## Errors

- Wrap with `%w` whenever adding context. Never `%v` for errors.
  ```go
  return "", fmt.Errorf("get issue %d: %w", issueNumber, err)
  ```
- Bare return when just propagating. Don't `return fmt.Errorf("%w", err)`.
- Validation errors get a plain `fmt.Errorf` (no wrap) since there's no
  underlying err to chain.
- `errors.Is` / `errors.As` for matching. No string compare on `err.Error()`.
- Don't panic in library code. Panic only in `main` or impossible-state
  asserts. Operator/CLI processes return non-zero exit, never crash.

## Naming

- Package names: short, lowercase, no underscores. `github`, `types`,
  `format`, `dispatch`. Match the directory name.
- Exported types: CamelCase. `Repo`, `AgentFamily`, `Config`.
- Unexported helpers: lowercase. `newClient`, `parseIssueLabels`,
  `findManifestPath`.
- Constants: `CamelCase` for exported. `DispatchStateName`, not
  `DISPATCH_STATE_NAME`.
- Test names: `TestSubjectVerbsObject`. See `TestSortPRReviewCandidatesPrioritizesPerfThenFixThenOldest`
  in `k3sc/cmd/take_test.go`. Long is fine; intent is what matters.
- Receivers: one or two letters, consistent across methods of a type
  (`r Repo`, `d Duration`, `c *Config`).

## Globals and init

- Package-level vars in `internal/types` and `internal/config` are the
  k3sc pattern: defaults compiled in, overridden by `config.Load()` at
  startup. See `var Namespace = "claude-agents"` in `types/types.go`.
- `init()` is fine for cobra command registration and for `config.Load()`.
  Don't put business logic in `init()`.
- One `var C Config` global in `internal/config`. Other packages read it,
  never write. Mutation happens once in `Load()`.

## Context

- Every IO function takes `ctx context.Context` as first arg.
- `cmd.Context()` from cobra. `context.Background()` only in tests.
- `exec.CommandContext(ctx, ...)` not `exec.Command(...)`. Lets the
  process be killed if context cancels.
- Never store context in a struct field. Pass through.

## Config

- YAML on disk, JSON tags on struct. Use `sigs.k8s.io/yaml` (converts
  YAML to JSON, then unmarshals). Standard YAML libraries don't honor
  `json:` tags.
- Defaults: `defaults() Config { return Config{...} }`. Merge file
  contents on top. See `k3sc/internal/config/config.go`.
- Custom unmarshalers for human-readable types. `Duration` wraps
  `time.Duration` and parses `"2m"` / `"1h"`.

## Logging

- `go.uber.org/zap` for structured logs in long-running processes
  (operator, reconcilers).
- `fmt.Fprintln(os.Stderr, ...)` and `fmt.Printf(...)` are fine for
  CLI output. Don't drag a logger into a 50-line subcommand.
- For controller-runtime / k8s code, use `sigs.k8s.io/controller-runtime/pkg/log`
  so log lines flow through the operator's logger.

## Concurrency

- Mutex over channels for simple shared state. `sync.Mutex` is the right
  call for protecting a map. Channels for ownership transfer / fan-out.
- `sync.Once` for lazy init.
- `errgroup.Group` (golang.org/x/sync/errgroup) for parallel ops where
  any failure cancels the rest.
- No `goroutine` without `defer wg.Done()` or an explicit lifecycle.
  Leaked goroutines are bugs.
- `context.WithTimeout` on every outbound network call.

## File and process I/O

- Paths: `filepath.Join`, never `+` or `"\\"`. Forward slashes work
  everywhere `filepath` is used.
- `os.MkdirAll(dir, 0o755)` before writing. Octal literal with leading
  `0o` (Go 1.13+).
- `runtime.GOOS == "windows"` guards for platform-specific code (killing
  processes, exe paths). See `killEndlessExe()` in `k3sc/cmd/cargo_lock.go`.
- File locking: `github.com/gofrs/flock` is the chosen library. Cross-platform.
- `exec.CommandContext` for shelling out. Always check `err`. Don't
  parse stdout with regex; pipe through `json.Decoder` or a real parser.

## Testing

- Stdlib `testing` only. No testify, no ginkgo in main code.
- Table-driven tests:
  ```go
  tests := []struct {
      worker string
      family types.AgentFamily
      ok     bool
  }{
      {worker: "claude-a", family: types.FamilyClaude, ok: true},
      {worker: "human-a", ok: false},
  }
  for _, tc := range tests {
      got, ok := types.ParseWorkerFamily(tc.worker)
      if ok != tc.ok || got != tc.family {
          t.Fatalf("ParseWorkerFamily(%q) = (%q, %v), want (%q, %v)",
              tc.worker, got, ok, tc.family, tc.ok)
      }
  }
  ```
- `t.Fatalf` for setup failures; `t.Errorf` for assertions where you
  want to keep running.
- Use `tc` not `tt` for loop variable. Matches k3sc style.
- Test files co-located: `foo.go` -> `foo_test.go`.
- `_test` package suffix only when testing the exported API of a
  package from outside (rare).

## Dependencies (preferred libraries)

- CLI: `github.com/spf13/cobra` + `github.com/spf13/pflag`.
- TUI: `github.com/charmbracelet/bubbletea` + `github.com/charmbracelet/lipgloss`.
- File locking: `github.com/gofrs/flock`.
- GitHub API: `github.com/google/go-github/v68` + `golang.org/x/oauth2`.
- K8s: `sigs.k8s.io/controller-runtime` + `k8s.io/client-go`.
- Logging: `go.uber.org/zap`.
- YAML: `sigs.k8s.io/yaml` (NOT `gopkg.in/yaml.v3` for structs with
  json tags).
- Standard library first. Bring in a dep when stdlib is genuinely missing
  the capability.

## Performance

- Preallocate slices when length is known: `make([]T, 0, n)`.
- **Struct field ordering matters for cache lines.** Group hot fields
  together; pack same-size fields to avoid padding. `go vet
  -fieldalignment` (now in `golang.org/x/tools/go/analysis/passes/fieldalignment`)
  flags layout issues.
- **Escape analysis:** `go build -gcflags='-m'` shows what escapes to
  the heap. Returning a pointer to a local var, capturing in a
  closure, or putting a value behind an interface forces a heap
  allocation. Audit hot paths.
- **`sync.Pool` for transient allocations** in hot paths (buffers,
  small structs). Standard pattern: `pool.Get().(*Buf)`, defer
  `pool.Put(buf)`. Don't hold pooled objects past the request scope.
- **`bytes.Buffer` and `strings.Builder` reuse:** call `Reset()`
  between uses instead of allocating new.
- **Avoid `interface{}` / `any` in hot paths.** Each interface value
  is a 2-word struct (type pointer + data pointer); method calls
  through interfaces are indirect. Use concrete types or generics.
- **Map lookup cost:** ~10ns for small maps; degrades with size and
  hash quality. For frequent lookups on a fixed set, consider
  `[N]struct{}` with a switch, or generate a perfect hash.
- **Channel cost:** ~50-100ns per send/recv. For very high frequency
  events, mutex + condition variable can be cheaper.
- **Goroutines have setup cost** (~2us + 8KB initial stack). For
  small N or short work, sequential beats parallel.

### Profiling and benchmarking

- **`go test -bench=. -benchmem`** for microbenchmarks. Always
  include `-benchmem` to track allocations.
- **`benchstat` (`golang.org/x/perf/cmd/benchstat`)** to compare
  before/after benchmark runs with statistical significance:
  ```
  go test -bench=. -count=10 > old.txt
  # make change
  go test -bench=. -count=10 > new.txt
  benchstat old.txt new.txt
  ```
- **`pprof`** for production profiling: `go test -bench=. -cpuprofile=cpu.out`
  then `go tool pprof cpu.out` for an interactive profile. `-memprofile`
  for heap, `-blockprofile` for blocking events.
- **`net/http/pprof`** for live process profiling: import as side
  effect, hit `/debug/pprof/profile` for a 30s CPU sample.
- **`trace`** (`-trace=trace.out` + `go tool trace`) for scheduler
  / GC / goroutine analysis. Visualization shows when goroutines
  block and on what.
- **`runtime.ReadMemStats`** for in-process heap snapshots. Useful
  to verify "no allocations in hot path" assertions.
- **Avoid microbenchmarks that the compiler optimizes away.** Use
  `b.N` correctly and `_ = result` to keep the operation live.

- Avoid `fmt.Sprintf` in hot paths; use `strings.Builder` for
  concatenation loops.
- `strings.HasPrefix` / `strings.TrimSpace` over regex for fixed
  patterns. Compiles aren't free.
- For map lookups in hot paths, `_, ok := m[k]` once, not separate
  `Contains` then `Get`.
- Don't reach for goroutines for parallelism without measuring. Sequential
  often wins for small N due to scheduler overhead.

## Cross-platform Windows

- Many agents run on Windows under Git Bash / PowerShell. Test paths
  with forward slashes via `filepath.ToSlash` if surfacing to shell.
- BOM handling: `sigs.k8s.io/yaml` does not strip UTF-8 BOM. If reading
  files PowerShell may have written, strip `﻿` from the head.
- Killing processes: `taskkill /F /IM name.exe` on Windows;
  `os.Process.Kill()` elsewhere. Guard with `runtime.GOOS`.

## Comments and docs

- Doc comment on every exported symbol. Starts with the symbol name.
  ```go
  // GetIssueOwner returns the owner label (e.g. "claude-a") for an issue,
  // or "" if unclaimed.
  func GetIssueOwner(...) {
  ```
- No restating-the-code comments. Comment the WHY: invariants, gotchas,
  non-obvious constraints.
- Terse. One sentence is usually enough.

## Avoid

- `interface{}` / `any` outside of `encoding/json` boundaries. Prefer
  concrete types or generics.
- Premature interfaces. Define the interface where it's consumed, not
  where it's implemented.
- Channels as a substitute for a mutex. Channels are for ownership.
- `init()` doing real work. Keep it to registration.
- Negative lookahead in regex; Go's `regexp` doesn't support it. Use
  alternation or post-filter in code.
- Shadowing `err`: `if err := f(); err != nil` inside a function that
  already has `err` causes silent bugs. Different name, or restructure.
- Naked returns in functions longer than 5 lines. Explicit returns are
  always clearer.
