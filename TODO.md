# TODO

## dehyphen rollout (em-dash + double-hyphen sweep)

Status: source-file support shipped for the languages we actually
use. Pre-commit hook and cross-repo sweep tooling still open.

The user's rule (CLAUDE.md "Absolute rules"): no em-dashes and no
double-hyphens as punctuation in prose. The cleanup script lives at
`scripts/dehyphen.py`. Idempotent. `--check` flag exits 1 on violations.

### Done

- [x] `scripts/dehyphen.py` v1: rewrites prose ` -- ` and em-dash forms
      in Markdown.
- [x] Preserves fenced code blocks, HR rules, YAML front matter,
      CLI flags like `--release`, numeric ranges like `2020--2025`,
      inline backtick code spans (the `cargo test --test foo -- --nocapture`
      cargo separator stays intact).
- [x] `--check` mode uses rewriter-equivalence (no substring false
      positives from `---` containing ` -- `).
- [x] Default replacement is period + capitalize next word
      (was colon; periods read more human).

- [x] **Markdown sweep across all owned repos**:
      `abix-/claude-blueprints`, `abix-/Grounded2Mods`,
      `abix-/abixio`, `abix-/abixio-ui`, `abix-/chromium-extensions`,
      `abix-/Schedule1Mods`, `abix-/endless`, `abix-/k3sc`.
      ~130 markdown files, ~2200 prose violations rewritten.

- [x] **`--lang rust`** shipped. Tokenizer handles `//`, `///`, `//!`,
      `/* ... */` (nested), preserves `"..."` strings, raw strings
      `r"..."` / `r#"..."#` (any hash count), char literals, lifetimes
      vs char literal disambiguation.

- [x] **`--lang python`** shipped. Rewrites `#` line comments. Preserves
      all strings (single/triple, with `r`/`b`/`f`/`u` prefixes).
      Triple-string docstrings intentionally NOT rewritten so test
      fixtures and regexes stay intact.

- [x] **`--lang csharp`** shipped. Handles `//`, `///` (XML doc),
      `/* ... */` (not nested). Preserves regular, verbatim `@"..."`,
      interpolated `$"..."`, verbatim-interpolated, raw `"""..."""`
      (C# 11+), and char literals.

- [x] **`--lang go`** shipped. Handles `//`, `/* ... */`. Preserves
      interpreted strings, raw backtick strings (multi-line),
      rune literals.

- [x] **`--lang toml`** shipped. Handles `#` line comments. Preserves
      `"..."`, `'...'`, triple-double, triple-single string forms.

- [x] **`--lang shell`** shipped. Handles `#` comments (only at
      whitespace boundary so `${foo#bar}` parameter expansion stays
      code). Preserves `"..."` and `'...'` strings.

- [x] **`--lang js`** shipped (covers `.js`, `.mjs`, `.cjs`, `.ts`,
      `.tsx`, `.jsx`). Handles `//`, `/* ... */`. Preserves
      `"..."`, `'...'`, and template literals `` `...` `` with
      balanced `${...}` interpolation.

- [x] **`--lang wgsl`** shipped. Comments-only (no strings in WGSL).

- [x] **`--lang yaml`** shipped. `#` comments at whitespace boundary;
      basic and literal scalars preserved.

- [x] **`--lang plain`** shipped. Whole file is prose.

- [x] **Auto-detect** by file extension. `.md`, `.rs`, `.py`, `.cs`,
      `.go`, `.toml`, `.sh`, `.bash`, `.js`/`.ts`/etc, `.wgsl`,
      `.yaml`/`.yml`, `.txt`/`.rst`. Override via `--lang`.

- [x] **Source-file sweep across all owned repos**:
      Rust: 238 files (`grounded2mods` 108, `abixio` 35,
      `abixio-ui` 19, `endless` 76, plus `chromium-extensions/hush`
      after audit caught it).
      Go: k3sc 9, endless 1.
      Python: 2 in claude-blueprints scripts.
      C#: 1 in Schedule1Mods.
      Shell: 2 (claude-blueprints, k3sc).
      JS/TS: ~6 in chromium-extensions.
      WGSL: 3 in endless.
      YAML: 1 in k3sc.
      All clean on final audit.

### Still open

- [ ] **Pre-commit hook** in `claude-blueprints/hooks/` that runs
      `dehyphen.py --check` on staged files. Block commit if any
      prose violation. Wire into install instructions.

- [ ] **Cross-repo sweep helper** `scripts/dehyphen_sweep.py` (or
      `.sh`): given a list of repo paths, find supported files,
      run dehyphen, summarize per-repo changes, commit per-file
      with the standard message format. The kind of tool we wished
      we had at the start of this rollout.

- [ ] **Docstring rewriting** for Python (currently OFF for safety).
      If user wants their module docstrings cleaned automatically,
      add `--lang python --include-docstrings` opt-in flag. Risk:
      multi-line string fixtures inside test files get mangled.

### Known gaps documented

- Rust: char-literal recognizer is conservative. Lifetimes vs char
  literals: we err on NOT treating ambiguous `'X` as char literals
  (walked as code, which is safe because code is never rewritten).
- Shell: heredoc bodies are treated as code. Prose violations inside
  a heredoc are missed but also never mangled.
- The period-vs-colon heuristic is not perfect. Sometimes a comma
  would read more natural. Reviewer responsibility; the script
  never lengthens content so reviewing is cheap.
- Inline triple-dash `Three --- dashes` is left alone (not the same
  rule).
- Numeric ranges `2020--2025` are left alone (no spaces).
- Vendored content (e.g. `ueforge/cpp/imgui/`,
  `abixio/site/assets/javascripts/lunr/`) is excluded from sweeps.

### Acceptance

1. `python scripts/dehyphen.py --check <every committed file across
   all owned repos>` exits 0. **DONE** for the file types we sweep.
2. Pre-commit hook prevents new violations from landing. **OPEN.**
