# TODO

## dehyphen rollout (em-dash + double-hyphen sweep)

Status: Markdown handling validated. Source-file handling not built.

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
- [x] Swept session skills clean: `ueforge`, `grounded2`,
      `outworld-station`, `schedule1`, `abixio`, `hush`, `rust`.

### Next: Markdown sweep across all repos

Run the script on every `.md` in every owned repo, review the diff per
repo, commit per repo. Reverse-chronological priority (skills are
public; user-facing docs next; internal notes last).

Repos to sweep (Markdown only, this pass):

- [ ] `abix-/claude-blueprints` (skills not yet swept: bevy, code,
      try-harder, abixio is already done, others; plus `README.md`,
      `CLAUDE.md` if appropriate)
- [ ] `abix-/Grounded2Mods` (`docs/`, every crate's `docs/`,
      `README.md`s, `CHANGELOG.md`s)
- [ ] `abix-/abixio` (huge `docs/` tree)
- [ ] `abix-/abixio-ui`
- [ ] `abix-/chromium-extensions` (each extension's `docs/` and
      `README.md`, especially `hush/`)
- [ ] `abix-/Schedule1Mods` (`docs/` and per-mod `README.md`s)
- [ ] `abix-/endless` (any remaining docs)
- [ ] `abix-/k3sc` (if any prose docs)

Process per repo: `python <path>/dehyphen.py <glob>` then `git diff`
review, then commit. The user expects ALL of them.

### Next: source-file support

Extend the script to handle non-Markdown files where prose lives in
comments and docstrings. Risk: mangling actual code (`x--` in C-family,
`a -- b` in Haskell pragmas, etc.).

Plan: add a `--lang <kind>` flag with these modes. Default stays
markdown.

- [ ] `--lang rust`: rewrite inside `//`, `///`, `//!`, `/* ... */`
      block comments only. Skip string literals (`"..."`, `r"..."`,
      `r#"..."#`). Skip code outside comments entirely.
- [ ] `--lang python`: rewrite inside `#` line comments and docstrings
      (`"""..."""`, `'''...'''`). Skip string literals.
- [ ] `--lang go`: rewrite inside `//` and `/* ... */` comments only.
- [ ] `--lang csharp`: rewrite inside `//`, `///`, `/* ... */` comments.
- [ ] `--lang shell`: rewrite inside `#` comments. Skip strings.
- [ ] `--lang toml`: rewrite inside `#` comments. Leave values alone.
- [ ] `--lang plain`: treat whole file as prose. For `.txt`, `.rst`.

Test fixture for each lang in `scripts/test/dehyphen/<lang>.<ext>`
with golden output.

### Next: auto-detect file type

- [ ] By extension: `.md`/`.markdown` -> markdown; `.rs` -> rust;
      `.py` -> python; etc. Drop `--lang` from common call sites.
- [ ] `--mode strict` for the audit: any prose violation in any
      supported file type is an error.

### Next: pre-commit hook

- [ ] Optional pre-commit hook in `hooks/` that runs
      `dehyphen.py --check` on staged Markdown files. Block the
      commit if any prose violation. Document in README.

### Next: cross-repo sweep tooling

- [ ] `scripts/sweep_repos.sh` (or `.py`): given a list of repo paths,
      run dehyphen on every supported file type, summarize per-repo
      changes, commit per repo with a standard message. The user
      expects to run this once to clean up all 30 days of damage.

### Known gaps to document

- The script's prose-rewrite heuristic picks `. ` (next char uppercase)
  or `: ` (otherwise). Sometimes a comma would be more natural. Reviewer
  responsibility, not script responsibility. Reviewing is cheap; the
  script never lengthens content.
- Inline triple-dash `Three --- dashes` is left alone (correct: not
  the same rule).
- Numeric ranges `2020--2025` are left alone (correct).

### Acceptance

The sweep is done when:

1. `python scripts/dehyphen.py --check <every committed .md across all
   owned repos>` exits 0.
2. Same for source files after `--lang` support lands.
3. Pre-commit hook prevents new violations from landing.
