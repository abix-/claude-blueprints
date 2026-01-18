---
name: skill-management
description: How to create, version, and maintain Claude skills. Read this skill FIRST when creating or modifying any skill file.
metadata:
  version: "3.3"
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

## Avoiding Token Bloat

| Bloat | Lean |
|-------|------|
| "You should always..." | Just state the rule |
| "It is important to..." | Just state the rule |
| Headers for 2-3 lines of content | Skip the header |
| Paragraphs explaining simple concepts | Bullets |
| Same concept in multiple sections | Say it once |
| "When X, you should Y" | "X → Y" |

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
3. Add reference to `CLAUDE.md` (see format below)
4. Add entry to `README.md` skills table
5. Commit and push

## CLAUDE.md Format

Use explicit read instructions — "follow standards in X" doesn't trigger file reads.

**Good:** `When writing PowerShell, read ~/.claude/skills/ansible-powershell.md first.`
**Bad:** `Follow standards in ~/.claude/skills/ansible-powershell.md when writing PowerShell.`

Keep CLAUDE.md lean:
- No section headers (## waste tokens for small files)
- One skill reference per line
- Trigger condition + read instruction

## Modifying Existing Skills

1. Read current skill from `skills/`
2. Make changes
3. Bump version, update date
4. Commit and push

## Verification Checklist

- Frontmatter uses only allowed keys
- Version incremented, date updated
- CLAUDE.md has explicit read instruction (not "follow standards in")
- README.md skills table updated
