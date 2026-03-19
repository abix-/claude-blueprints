---
description: Update docs, changelog, commit, and push after completing feature work.
disable-model-invocation: true
allowed-tools: Bash, Read, Edit, Grep, Write, Glob
version: "1.0"
---
## Steps

1. **Run clippy**: `k3sc cargo-lock clippy --manifest-path /c/code/endless/rust/Cargo.toml 2>&1`. Fix any warnings before proceeding.

2. **Identify changed files**: Run `git diff --name-only` and `git diff --cached --name-only` to see what changed.

3. **Read docs/README.md**: Find which architecture doc(s) cover the changed systems. Use the System Map and File Map to match changed files to docs.

4. **Update architecture docs**: For each relevant doc in docs/:
   - Read the current doc
   - Read the changed source code
   - Update the doc to match the new code (data flow, components, known issues, ratings)
   - If a new system was added, create a new doc and add it to the docs/README.md index

5. **Update CHANGELOG.md**: Add entry describing what changed. Follow existing format.

6. **Commit and push**: Stage all changed files (source + docs + changelog), write a concise lowercase commit message, push immediately.

## Rules

- Don't update docs that aren't affected by the code changes
- Don't add content to docs that isn't in the code -- docs reflect reality
- If a known issue was fixed, remove it from the doc and from docs/README.md aggregate list
- If a new known issue was discovered, add it
- Adjust ratings if the change improved or degraded the system
