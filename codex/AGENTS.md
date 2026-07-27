# AGENTS.md

## Working agreements

- Treat the user's task as approval for normal in-scope work. Continue through implementation and verification without asking between routine actions. Ask before destructive actions, external changes, or meaningful scope expansion. If the user says stop, stop.
- Read current governing files from disk before acting. Do not use stale injected text, cached text, or an earlier conversation copy as current.
- Answer exactly what was asked. Lead with the literal answer and use concise plain English.
- Follow direct instructions promptly. Ask only when genuine ambiguity cannot be resolved from the repository.
- Keep all work in the foreground. Never use subagents.
- Use PowerShell on Windows. Use another shell only when the user explicitly requests it.
- Use the user's terminology. Do not invent synonyms, structural labels, or architecture jargon.
- Do not use code identifiers as unexplained prose. When an identifier is necessary, pair it with a verified file and line.
- Never use em dashes or double hyphens as prose punctuation. Literal command flags and code syntax are allowed.
- Do not expand scope. Record newly discovered work in the existing todo when it belongs to the approved project plan.
- Preserve all uncommitted work. Never use destructive Git commands or `git stash`. Commit only the intended paths.
- Let dependencies determine tool call shape. Batch independent read-only checks when useful, then inspect results before dependent actions.
- Do not emit filler commands, sleeps, or repeated probes while waiting for output.
- Never probe a live system with an ad-hoc one-liner. A live probe must remain in the repository as a test or permanent diagnostic operation invoked by a test.

## Skills

- Always read and follow `~/.agents/skills/try-harder/SKILL.md`.
- Read the matching skill before starting specialized work.
- Keep reusable workflows in skills. Keep this file limited to durable cross-repository guidance.

## Development and verification

- Use tests first for behavior changes. Commit useful tests to the repository.
- Do not claim completion until the relevant build and tests pass.
- Compiler and diagnostic errors in changed code must be resolved before committing.
- Volume is not progress. Report verified behavior, moved design state, and remaining blockers.
- When repeated small fixes expose related failures, stop the patch and rerun cycle. Record the symptoms in the existing todo, group them under their shared design or authority issue, update the existing plan, and implement the coherent change before another full acceptance run.
- For repository claims, verify the source and cite the file and line. If evidence is unavailable, say that there is not enough information to assess it.
- Use `k3sc cargo-lock` for Cargo commands. Never run bare `cargo`.
- Use concise lowercase commit messages. Do not add `Co-Authored-By`. Push every completed verified commit immediately.
- Use ASCII in code, written files, and commit messages. Unicode is allowed in terminal output.

## Documentation and project state

- Use existing authoritative documents. Do not create parallel design documents.
- Keep unfinished work in the existing todo, shipped history in the repository changelog, and durable design in the owning design document.
- Update `.Codex/project_state.md` after meaningful work. Record the current focus, design goals, last session summary, next steps, and open questions.
- Never put secrets, tokens, credentials, or authentication data in project state or documentation.

## Safety

- Never read, output, or share credential files, authentication files, tokens, or secrets.
- Never destroy uncommitted work unless the user specifies the exact destructive command and exact path in the current turn.
- Never use `git stash`.
- In grounded2mods repositories, use `patternsleuth` through `modforge::patterns::sleuth` for signature, cross-reference, and data scans. Add missing capability upstream instead of writing a separate scanner.
- k3s agents have no GPU, display, or game runtime. Do not run game profiling or hardware benchmarks there. Mark those checks for local verification.

End every response with `Confidence: X/10.` The rating reflects confidence in the correctness of the last action or statement.
