---
name: skill-management
description: How to create, version, and maintain Claude skills. Read this skill FIRST when creating or modifying any skill file.
metadata:
  version: "3.1"
  updated: "2026-01-18"
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
name: skill-name           # required
description: When to use   # required
license: MIT               # optional
allowed-tools: [...]       # optional
compatibility: "1.0"       # optional
metadata:                  # optional, for custom fields
  version: "X.Y"
  updated: "YYYY-MM-DD"
---
```

Only these keys allowed at root level. Unknown keys cause upload rejection on Claude web.

## Versioning Rules

1. **Increment version on every change** — no exceptions
2. **Format:** `major.minor` (e.g., "2.4")
   - Major: breaking changes or significant restructuring
   - Minor: additions, fixes, refinements
3. **Update the date** with every version change

## Skill Structure

Skills live in the `skills/` directory as flat files:
```
skills/
├── skill-name.md
└── another-skill.md
```

## Creating a New Skill

1. Write content with frontmatter
2. Save to `skills/skill-name.md`
3. Add reference to `CLAUDE.md` describing when to use it
4. Add entry to `README.md` skills table
5. Commit and push

## Modifying Existing Skills

1. Read current skill from `skills/`
2. Make changes
3. Bump version, update date
4. Commit and push

## Verification Checklist

- Frontmatter uses only allowed keys
- Version incremented, date updated
- CLAUDE.md references the skill with usage context
- README.md skills table updated
