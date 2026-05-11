---
name: code
description: Universal development standards across every language. Sourced from patterns recurring in abix- Rust, Go, C#, Python, and TS code. Use when writing any code.
user-invocable: false
version: "4.0"
updated: "2026-05-11"
---
# Code

Cross-language rules. Language-specific skills (rust, golang, csharp,
python, typescript, etc.) override or extend these. Read this first
when writing any code; read the language skill for syntax-level
choices.

## Scope discipline

- Do exactly what was asked. Nothing more.
- If a task is "remove field X", remove the field. Don't refactor
  nearby code that bothers you.
- "While I'm here" is a bug source. Stop. Surface the observation,
  let the human decide.
- Three similar lines is better than a premature abstraction.
- Don't add backward-compat shims, feature flags, or unused public
  APIs "for later." YAGNI.

## MVP first

- Simplest working solution. Add complexity only when the requirement
  forces it.
- No half-finished implementations. If you stub something, mark it
  `TODO(name): why` and surface it.
- For one-shot scripts: no config files, no flags you don't need
  yet, no DI. Free functions in a single file.

## DRY but not premature

- 3+ copies of the same logic = extract. Fewer = leave it.
- When extracting, do it across **every** call site in one pass.
  Don't leave half the codebase using the old form.
- SystemParam bundles, helper structs, shared utility functions are
  the typical extractions in this codebase.

## Naming

- Names reflect purpose, not implementation. `GetConfigPath` not
  `ReadFromJsonAt`.
- Match the existing surrounding style. If the file uses `tc` for
  table-test loop vars, don't introduce `tt`.
- Match the source of truth. If config calls it `launch_dir`, the
  Go field is `LaunchDir`, not `LaunchPath`.
- Status / log messages use the same words as the corresponding
  fields / properties.

## Comments

- Comment the WHY. The code already shows the WHAT.
- Hidden invariants, race conditions, install order, empirical
  findings: those are the comments worth writing.
- Iteration notes are acceptable when they tie a decision to evidence
  (see `Schedule1Mods/EmployeeReset/Mod.cs` for the "ITERATION N:"
  pattern). They explain why an obvious-looking alternative is wrong.
- Don't restate the function name or parameter types.
- Don't add "added for issue #123" or "used by X". Belongs in the
  commit message and PR description.
- Default to **no comment**. If removing it wouldn't confuse a
  reader, don't write it.

## Errors

- Never silently suppress. Log with context, or propagate. Empty
  `catch { }` blocks are bugs.
- Wrap at boundaries with the local context: `fmt.Errorf("get
  issue %d: %w", n, err)` (Go), `.context("read config")?`
  (Rust), `throw new Error("X failed: " + e.message)` (TS).
- Validate at trust boundaries (user input, network, file I/O).
  Trust internal callers; don't re-validate at every layer.
- Don't catch and re-throw the same exception class with no added
  context. Either add context or let it propagate.
- Never use error suppression flags as a shortcut (`--no-verify`,
  `set +e`, empty `except:`, swallow-everything `try/catch`).
  Diagnose the root cause.

## Logging

- Tagged prefix per component: `[Hush bg]`, `[k3sc]`, `[EmployeeReset]`.
  Greppable across multi-component logs.
- Stderr for errors and diagnostics. Stdout is for results that
  another tool will consume.
- Verbose / diagnostic logs go behind a feature flag or pref.
  Default off.
- Log lines are one line each. Multi-line blobs go to a file or
  trace dump, not the console.

## Paths and I/O

- Use the language's path API (`filepath.Join`, `pathlib.Path`,
  `std::path::PathBuf`). Never `+` strings.
- Forward slashes are safe across Go, Rust, Python, .NET when passed
  to the path API.
- Resolve relative to `__file__` / executable / repo root, not the
  caller's CWD. CWD is volatile.
- `MkdirAll` / `ensure_dir` before writing. Don't assume.
- Strip UTF-8 BOM when reading files PowerShell may have written.

## Cross-platform Windows

- Many agents and users are on Windows. Test on Windows or guard
  with `runtime.GOOS` / `cfg!(windows)` / `os.name`.
- Kill processes with `taskkill /F /IM name.exe` on Windows,
  `kill(pid)` elsewhere. Always guard the platform branch.
- Line endings: LF in files; let git handle CRLF locally. Never
  commit CRLF.
- ASCII in source files. Unicode only in user-facing output
  (terminal tables, status lines, UI strings).

## Stdlib over custom

- Don't reimplement what the stdlib has. Sort, hash, JSON encode,
  HTTP, regex, path: use the language's built-ins first.
- Bring in a dependency when the stdlib genuinely doesn't have it
  or when the alternative is materially worse.
- Check the existing `Cargo.toml` / `go.mod` / `package.json` before
  adding a new dep. Reuse what's already there.

## Idempotency

- Scripts and one-shot ops should be safe to re-run. Re-running the
  setup doesn't double the data; re-running the build doesn't
  corrupt the output.
- Migrations, dehyphen, sanitization: idempotent or document why
  not.
- Don't append-only without a header / dedup mechanism.

## Testing

- When fixing a bug, reproduce it in a test first. Red, then green.
- When adding behavior, test the behavior, not the implementation.
- Test names describe the asserted property:
  `TestSortPRReviewCandidatesPrioritizesPerfThenFixThenOldest`. Long
  is fine.
- Table-driven tests for input/output matrices. Loop variable `tc`
  in Go, parametrized in Python/Rust.
- After any code change, check which tests cover the change and
  verify they still pass. Don't just check that the file compiles.
- Don't mock what you can run for real. Hit the real database / API
  / file system in integration tests when feasible. Mocks lie under
  real-world conditions.

## Performance

- Don't optimize without a measurement. "Faster" is not a fact until
  it's a benchmark.
- Per-frame / per-second hot paths get zero-alloc treatment: no
  LINQ, no `fmt.Sprintf` loops, no `Dictionary` allocs in the loop.
  See `csharp` skill for the Timberbot patterns.
- Cache derived values when the source rarely changes
  (`RefChanged` pattern in `csharp`, memo tables in Rust).
- Read once, write many: pull the value into a local before looping
  over it.
- Goroutines / async tasks have setup cost. For small N, sequential
  often wins. Measure.

## Files and formatting

- UTF-8, LF, no BOM, final newline.
- ASCII only in source files (the dehyphen rule applies to comments
  and strings; non-ASCII data is fine as data).
- 2-space indent for YAML / JSON, 4-space for Python, tabs for Go,
  language default elsewhere.
- One concept per file. Don't pile unrelated helpers in `utils.ts`.
- Co-locate tests with sources: `foo.go` / `foo_test.go`,
  `foo.ts` / `foo.spec.ts`.

## Git hygiene

- Concise, lowercase commit messages. One line, ~70 chars.
- Body explains WHY when non-obvious. Never copy the diff into the
  message.
- No `Co-Authored-By`. No emoji.
- Don't commit secrets, large binaries, build artifacts. Use
  `.gitignore`. Stray `.cargo-build.lock` in this repo (now gone)
  is the cautionary tale.
- Push immediately after commit. Long-lived local branches drift.

## Verification before claiming done

- Build it.
- Run the tests.
- For UI / runtime features, exercise them. A type-check is not
  proof of correctness.
- Cite the verification command in the PR / status update.

## Avoid

- Excessive error handling. One layer of context is plenty.
- Variables for single-use values.
- Premature abstractions. Three concrete copies first.
- Comments restating the code.
- Magic numbers in multiple call sites. Extract a named constant.
- "Defensive" code at internal call sites. Validate at the boundary,
  trust the rest.
- Renaming unused `_var`, re-exporting types "just in case",
  `// removed code` markers. Delete it. Git remembers.
