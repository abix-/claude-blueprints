---
name: skill-management
description: How to create, version, validate, and maintain Claude skills. Read this skill FIRST when creating or modifying any skill file.
metadata:
  version: "2.0"
  updated: "2026-01-12"
  content_sha256: "51549ac1e91a7713b8722311a2fb576ea2ae7759a9af40d474889877c9e2e397"
---
# Skill Management

## Priority Rule
**Read this skill BEFORE reading or modifying any other skill.**

## Writing Effective Skills

- Write for Claude, not humans. Clear instructions > motivational framing.
- Be explicit about scope. When does this skill apply?
- Resolve ambiguities. If instructions could conflict, clarify which wins.
- Cut repetition. Say it once.
- Leaner is better. Same payload, fewer tokens.

## Frontmatter Structure

```yaml
---
name: skill-name
description: When to use this skill
metadata:
  version: "X.Y"
  updated: "YYYY-MM-DD"
  content_sha256: "hash-of-content-only"
---
```

### Allowed Frontmatter Keys
- `name` — skill identifier (required)
- `description` — when Claude should use this skill (required)
- `license` — optional
- `allowed-tools` — optional
- `compatibility` — optional
- `metadata` — nested object for custom fields (version, checksum, etc.)

Any key not in this list causes upload rejection. Put custom fields inside `metadata:`.

## Versioning Rules

1. **Increment version on every change** — no exceptions
2. **Format:** `major.minor` (e.g., "2.4")
   - Major: breaking changes or significant restructuring
   - Minor: additions, fixes, refinements
3. **Update the date** with every version change

## Checksum Validation

Checksums verify content integrity after upload. Checksum the **content only** (after frontmatter), not the whole file — this allows metadata changes without invalidating the checksum.

### Generate/Verify Checksum

```bash
tail -n +$(($(awk '/^---$/{n++; if(n==2){print NR; exit}}' SKILL.md) + 1)) SKILL.md | sha256sum
```

This auto-detects where frontmatter ends and hashes everything after. Use on `/home/claude/SKILL.md` during creation, then on `/mnt/skills/user/SKILL-NAME/SKILL.md` after upload to verify.

## Skill Structure

Each skill is a single folder containing one file:
```
skill-name/
└── SKILL.md
```

## Creating a New Skill

1. Write content (without frontmatter)
2. Add frontmatter with `content_sha256: "PENDING"`
3. Generate checksum, update metadata
4. Copy to outputs: `cp SKILL.md /mnt/user-data/outputs/`
5. Upload via Claude UI: Settings > Capabilities > Skills
6. Verify checksum in new conversation

## Modifying Existing Skills

1. Read current skill: `view /mnt/skills/user/skill-name/SKILL.md`
2. Make changes
3. Bump version, update date
4. Regenerate checksum, update metadata
5. Copy to outputs, re-upload, verify

## Verification Checklist

- Frontmatter uses only allowed keys
- Version incremented, date updated
- Checksum generated and added to metadata
- Checksum verified in current conversation (before upload)
- File copied to `/mnt/user-data/outputs/`
- Checksum verified after upload (new conversation)
