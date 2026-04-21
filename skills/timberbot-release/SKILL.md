---
name: timberbot-release
description: Build, test, and release Timberbot mod to GitHub and Steam Workshop
disable-model-invocation: true
argument-hint: "[version]"
version: "1.0"
---
# Timberbot Release

Bump version, run regression tests, build, tag, and publish.

## Steps

1. **Version bump** -- update both files to `$ARGUMENTS`:
   - `timberbot/src/Timberbot.csproj` (`<Version>`)
   - `timberbot/src/manifest.json` (`"Version"`)

2. **Build** -- `cd timberbot/src && dotnet build`

3. **Test** -- requires game running with Iron Teeth day-5 save:
   - `python timberbot/script/test_validation.py`
   - ALL tests must pass. If any fail, STOP.

4. **Commit + push**:
   - `git add timberbot/src/Timberbot.csproj timberbot/src/manifest.json`
   - `git commit -m "v$ARGUMENTS"`
   - `git push`

5. **Release** -- `python timberbot/script/release.py --release`

6. **Release notes** -- update via `gh release edit`:
   - Flat list only. No headers, no grouping, no summary line. Just `- [tag] description` lines
   - Tags: `[breaking]`, `[feature]`, `[fix]`, `[internal]`
   - Concise, player-facing, no implementation details
   - No periods at end of lines
   - Check previous releases (`gh release view`) to avoid overlapping content
   - Review git log since last release tag to build notes from actual changes
   - Example format:
     ```
     - [fix] unlock_building deducting science twice
     - [feature] Wellbeing breakdown endpoint
     - [breaking] /api/natural_resources removed -- use /api/trees and /api/crops
     - [internal] 118 integration tests
     ```

7. **Steam Workshop** -- remind user to upload from Timberborn Mod Manager

## Repo paths

- C# mod: `timberbot/src/`
- Python client: `timberbot/script/timberbot.py`
- Test suite: `timberbot/script/test_validation.py`
- Release script: `timberbot/script/release.py`
