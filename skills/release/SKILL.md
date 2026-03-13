---
description: Create a GitHub release with notes from CHANGELOG, triggers build workflow
disable-model-invocation: true
allowed-tools: Bash, Read, Grep
version: "1.0"
---
## Steps

1. **Determine version**: If an argument is provided (e.g. `/release v0.1.5`), use it. Otherwise, get the latest `v*` tag and bump the patch number.

```bash
cd /c/code/endless && git tag -l 'v*' | sort -V | tail -1
```

2. **Confirm on main and pushed**: Verify we're on main, working tree is clean, and HEAD is pushed to origin.

```bash
cd /c/code/endless && git status --porcelain && git log origin/main..HEAD --oneline
```

If dirty or unpushed, stop and tell the user.

3. **Find previous release tag**: Get the tag before the new one to determine the changelog range.

```bash
cd /c/code/endless && git tag -l 'v*' | sort -V | tail -1
```

4. **Build release notes**: Read CHANGELOG.md entries since the last release tag. Write notes that make a player want to download the update.

   **Voice**: Write for the player, not the developer. Every bullet should answer "what can I do now?" or "what's better now?" — never "what did we refactor."

   **Format**: `- **Short Title** — one sentence that sells the feature or explains why they should care`

   **Process**:
   - Group dozens of changelog entries into 6-10 player-facing bullets
   - Lead with the biggest new features (things the player can see and interact with)
   - Combine related entries (e.g. "added towers" + "tower inspector" + "tower stats tuning" = one Tower bullet)
   - Internal/architectural work gets ONE bullet max ("Performance & Stability" or similar), only if significant
   - Bug fixes fold into their parent feature or into the stability bullet — no standalone fix bullets
   - Add personality and specifics: "no more getting wiped before you can build" > "added town fountain auto-attack"
   - End with `**Full Changelog**: https://github.com/abix-/endless/compare/{prev_tag}...{new_tag}`

   **Reference** — match the tone of previous releases:
   ```
   - **Town Fountain**: Auto-shoots nearby enemies, no more getting wiped before you can build
   - **Smarter AI Economy**: AI players actually want food now — they build farms and homes faster when running low
   - **Direct Unit Control** — box-select + right-click move/attack, hold-fire and keep-fighting toggles
   ```

   **Avoid**: implementation details (ECS, GPU buffers, SystemParam), method names, file names, architectural patterns, internal metrics ("~90% bandwidth reduction"). If a player wouldn't understand the bullet without reading source code, rewrite it.

5. **Show the user** the version tag and release notes for confirmation before creating.

6. **Create the release**: This creates the tag on GitHub and triggers the build workflow.

```bash
cd /c/code/endless && gh release create {tag} --title "{tag}" --notes "{notes}" --target main
```

7. **Verify build triggered**:

```bash
cd /c/code/endless && sleep 3 && gh run list --workflow=build --limit=1 --json databaseId,status,url --jq '.[0] | "Run #\(.databaseId): \(.status) — \(.url)"'
```

Report the release URL and build run URL.

## Rules

- Never create a release if working tree is dirty or commits are unpushed
- Notes are a sales pitch to the player — every bullet should make them want to try the update
- No developer jargon: no ECS, GPU, SystemParam, HashMap, refactor, pipeline, etc.
- 6-10 bullets max — fewer is better if the features are strong
- Always include the Full Changelog comparison link
- Pull the tag locally after creation: `git fetch --tags`
