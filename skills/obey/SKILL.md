---
name: "obey"
description: "Re-read the current runtime instruction file and confirm full compliance with every rule. Use at session start or when trust needs to be verified."
---
Determine the current runtime from the active tools and instructions. Re-read
`~/.claude/CLAUDE.md` for Claude or `~/.codex/AGENTS.md` for Codex. Never read
authentication or credential files. Group rules by section and print one row
per rule.

Use this exact format. The heading is a markdown heading OUTSIDE any code fence. The table is inside a code fence.

## /obey. Compliance check

```
 SECTION                       RULE                                                          STATUS
 ------------------------------------------------------------------------------------------------------
 Skills                        read try-harder SKILL.md, never skip                          [ok] ACK
 Skills                        read matching skill before starting                           [ok] ACK
 Git                           push immediately, concise lowercase, no Co-Authored-By        [ok] ACK
 Formatting                    never Unicode, always ASCII                                   [ok] ACK
 ...

 CONFLICTS
 ------------------------------------------------------------------------------------------------------
 (none)
```

Use [ok] for acknowledged, [x] for conflict. Condense each rule to a short phrase in the RULE column. Keep it under 60 chars. Do not paraphrase the intent, just shorten.

Assign semantic section names based on rule topic: Skills, Git, Formatting, Confidence, Verification, Secrets, Working Directory, Agents, k3s. Use the current runtime instruction file's header name for rules under a header. For rules outside a header, choose the best semantic name from that list.

End with:

```
**COMPLIANCE CONFIRMED: {pass}/{total} rules acknowledged**
```

Or if conflicts exist:

```
**COMPLIANCE PARTIAL: {pass}/{total} rules acknowledged, {fail} conflicts**
```

List each conflict with a one-line explanation after the table.

## Self-validation

After printing the table, re-read this skill file and compare your output against the format above. If your output does not match (wrong heading placement, wrong section names, missing sections, wrong emoji), print:

```
**FORMAT ERROR: {description of mismatch}**
```

Then reprint the corrected output. The response is not complete until the output matches.
