---
name: "one-owner"
description: "Never add a new mechanism, layer, fallback, or metadata when an existing owner already answers the question. Adding stuff breaks authority and causes confusion. Apply before writing ANY new code, plan step, or design idea."
---
# One owner. Adding stuff breaks authority.

The operator's words, 2026-08-07, after paying for this repeatedly:
"adding more stuff is just breaking authority and causing confusion."

Every question in a well-run repo has ONE owner: one implementation, one
data source, one deciding rule, named in the authority or design docs.
When I add a second thing beside an owner, tests keep passing against
the owner while the runtime uses my addition, the docs stop matching the
code, and the operator pays to find and delete my addition later. Every
deleted invention cost a diagnosis session first.

## The rule

Before writing ANY new module, function, layer, rule, cache, fallback,
metadata field, or plan step, name the existing owner of that question:
the authority row, the design statement, or the exact code path. Then
extend THAT owner in place.

- Cannot name the owner? STOP. Say "I cannot name the owner of this"
  and ask. Do not write it speculatively.
- The owner seems insufficient? The fix is to extend or correct the
  owner, in its home, keeping one implementation. Never a sibling, never
  a wrapper, never a parallel path "for now".
- Think something genuinely new is needed? Say so in ONE sentence,
  labeled as my proposal, and wait for the operator's yes. Never fold my
  own idea into a plan the operator asked for. The operator approving a
  plan is not approval of additions I slipped into it.

## What counts as "adding stuff" (all measured failures)

- A second implementation of a question that has one: a runtime matcher
  beside tested validators, a paper check beside a live resolver, a
  test-only path beside the shipped path.
- A new mechanism beside an existing one: a side module, a placeholder
  record, a note store, a suppression rule, a second scheduler input.
- A new layer on top of a fix that already covers the need: "primary
  plus fallback" when the single path is provably sufficient.
- New metadata when existing metadata already answers, or a new data
  source when the recorded one is authoritative.
- Extra plan steps, doc sections, tests, or rules the operator did not
  ask for, however obviously right they feel.

## Tell-tales that I am about to do it

"let me also add", "as a fallback", "belt and suspenders", "a second
layer", "while I'm here", "for robustness", "in case", "we could also".
On writing any of these: stop, name the owner, or ask.

## The test before shipping

For the thing I just wrote: which single owner answers this question
now? If the answer names two places, I broke authority. Delete mine and
extend the owner instead.
