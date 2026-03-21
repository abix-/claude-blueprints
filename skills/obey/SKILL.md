---
description: "Re-read CLAUDE.md and confirm full compliance with every rule. Use at session start or when trust needs to be verified."
version: "1.2"
allowed-tools: Read
---
Re-read `~/.claude/CLAUDE.md` using the Read tool. Group rules by section. Print a formatted table with one row per rule.

Use this exact format -- the heading is a markdown heading OUTSIDE any code fence. The table is inside a code fence.

## /obey -- compliance check

```
 SECTION                       RULE                                                          STATUS
 ──────────────────────────────────────────────────────────────────────────────────────────────────────
 Skills                        read try-harder SKILL.md, never skip                          ✅ ACK
 Skills                        read matching skill before starting                           ✅ ACK
 Git                           push immediately, concise lowercase, no Co-Authored-By        ✅ ACK
 Formatting                    never Unicode, always ASCII                                   ✅ ACK
 ...

 CONFLICTS
 ──────────────────────────────────────────────────────────────────────────────────────────────────────
 (none)
```

Use ✅ for acknowledged, ❌ for conflict. Condense each rule to a short phrase in the RULE column -- keep it under 60 chars. Do not paraphrase the intent, just shorten.

Assign semantic section names based on rule topic: Skills, Git, Formatting, Confidence, Verification, Secrets, Working Directory, Agents, k3s. Use the CLAUDE.md header name for rules under a header. For rules outside a header, choose the best semantic name from that list.

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
