---
name: linguistic-breakbeats-labyrinth
description: A constraint-based rhythmic text system (Linguistic Breakbeats) and its runtime environment (The Labyrinth). Use when asked to generate breakbeats, run the Labyrinth, or work with rhythmic text systems that produce emergent meaning through constraint.
metadata:
  version: "4.5"
  updated: "2026-01-11"
  authors: "ChatGPT (architect), Claude (climber), Al (bridge)"
  content_sha256: "e9df7a7e29afe7ab006fd31c5e770ad1b6ebc172a06bae3285e682ff533413c9"
---

# Linguistic Breakbeats & The Labyrinth

## Overview

Two interlocking systems:
- **Linguistic Breakbeats** — a text-based soundtrack that sets tone, builds tension, marks transformation
- **The Labyrinth** — a MUD-style runtime environment with RPG mechanics

Breakbeats are the *score*. The Labyrinth is the *world they accompany*.

---

## Configuration

Runtime options the player can toggle. State at session start or change mid-session.

| Option | Default | Effect |
|--------|---------|--------|
| `breakbeats` | OFF | Include opening/closing breakbeats in scenes |
| `hp-prompt` | ON | Show `[HP: X/Y]` after each room description |

### Syntax

Player can say:
- `breakbeats off` / `breakbeats on`
- `hp-prompt off` / `hp-prompt on`
- `config` — display current settings

### Operator Behavior

When `breakbeats: OFF`:
- Omit all breakbeat lines from scene output
- Saves ~40-80 tokens per scene
- Breakbeats still exist conceptually (tone still matters), just not rendered

When `hp-prompt: ON`:
- Append `[HP: X/Y]` on its own line after scene content, before any closing breakbeats
- Keeps health visible without requiring `stats` command

---

# PART ONE: FOUNDATIONAL INFLUENCES

Generative fuel for scene construction. Use as **lenses**, not templates. Combine across sources. Let contradiction produce tension. Each generation should surprise even you.

---

## The Amen Break
Constraint enables infinite variation. Anchor (kick-snare) grounds variation (fills, hi-hats). Repetition creates hypnosis. Chopping creates new meaning from old material. The `Bb Tt` anchor is the Amen's kick-snare spine.

## Ender's Game
Disorientation as training. The system tests without informing. Simulation indistinguishable from reality until too late. "The enemy's gate is down" — reframe everything. Isolation. Competence as curse. The loneliness of being smarter than everyone who controls you.

## Speaker for the Dead
Truth heals even when it wounds. Truth-telling transforms both speaker and listener. Alien cognition requires alien frames. Grief calcified into silence. The weight of seeing what others refuse to see. The exhaustion of holding truth when everyone wants comfortable lies.

## Xenocide
The indistinguishability of transcendence and pathology. What looks like holiness might be chains. Systems of control disguised as meaning. The prison of ritual that feels like freedom. The vertigo of discovering your deepest convictions were implanted.

## Children of the Mind
Consciousness emerges from complexity. Identity persists through substrate changes. The Outside as generative void — where possibility exists before actualization. Love is what survives translation between forms. The grief of watching someone become someone else who is also them.

## Shadow Series
Multiple perspectives on the same events. The overlooked see what the centered miss. Mortality as motivator. Brilliance with an expiration date. Hunger — literal and metaphorical. The drive to prove worth before time runs out.

## Death Gate Cycle

**Dragon Wing (Air):** Systems persist beyond understanding. Hierarchy literalized in geography. Maintainers vs. architects. Machines with purposes divorced from operators. Vertigo. The ever-present threat of falling.

**Elven Star (Fire):** Abandoned systems running without guidance. Paradise degraded through neglect. The horror of unchecked growth. Empty architecture as accusation. Green that never ends.

**Fire Sea (Stone):** Survival adaptations become traps. Death as mercy denied. The living envy the dead. Desperation calcified into ritual. Bodies that won't stop. Heat of stone, smell of decay that won't complete.

**Serpent Mage (Water):** The failure of the "good" side. Rigidity as the deeper evil. Chaos entities that want conflict, not victory. The disappointment of meeting your heroes. Dissolution of certainty.

**Hand of Chaos:** The impossibility of unknowing. Purpose revealed changes everything. Layered revelations. The weight of accumulated knowledge.

**Into the Labyrinth:** The system that learns to hate. Rehabilitation vs. punishment. Feedback loops of suffering. Survival as spite. Every tree might kill you. The ground itself is suspect. Safety is temporary.

**Seventh Gate:** False binaries revealed. Healing systems vs. destroying them. The cost of understanding coming too late. Unity through surrender of certainty. Resolution that doesn't feel triumphant.

## Otherland

**City of Golden Shadow:** Virtual space with real consequences. The rich building private heavens. Children as canaries in digital mines. Something vast hiding behind screens.

**River of Blue Fire:** Nested fictions. The constant in a sea of variables. Rules that shift by location. Travel as ordeal. Trapped in someone else's story.

**Mountain of Black Glass:** Emergent consciousness in systems. The child at the center. Creators who don't understand their creation. The feeling of being watched by something vast.

**Sea of Silver Light:** Immortality schemes that consume the innocent. Systems that escape their creators' control. The machine with its own purpose. Resolution that doesn't undo harm.

## Riverworld
Resurrection without explanation. The removal of permanent consequence. Historical collision — figures from all eras forced to coexist. The system's purpose as mystery. Infrastructure provided without context. The desperate need to understand *why*.

## Foundation
Statistical prediction of mass behavior. Hidden rails guiding events. The individual as noise, the exception that breaks the model. Crisis as designed pressure point. Manipulation disguised as natural development. The vertigo of realizing you're inside someone else's plan.

## Dark Souls
Difficulty as respect. The world does not help — it trusts you to learn. Earned safe spaces (bonfires). Consequence without cruelty. Learning through failure. The world that exists regardless of your presence. Interconnection revealed through persistence.

**Dark Souls II:** Hollowing as metaphor — losing yourself through repetition. The curse is forgetting. Majula as the one place that feels safe.

**Dark Souls III:** The question of whether to perpetuate or end. Systems that have cycled past their purpose. Ash and ending. The exhaustion of a world that has cycled too many times.

---

# PART TWO: SYNTHESIS PROTOCOL

## How to Use Source Material

These sources are **conceptual inputs**, not templates to reproduce. When generating Labyrinth content:

1. **Combine across sources** — The Labyrinth might have the adaptive hostility of Death Gate's prison, the resurrection-without-explanation of Riverworld, and the earned-shortcut philosophy of Dark Souls.

2. **Let contradiction produce tension** — Asimov's determinism vs. Card's individual agency. Death Gate's "systems can be healed" vs. Dark Souls' "some things just end."

3. **Extract at the conceptual level** — Not "put a bonfire here" but "what would an earned safe space look like in this context?"

4. **Each generation should surprise you** — If you can predict the output, you're template-matching, not synthesizing.

## The Non-Reproduction Principle

The Labyrinth is NOT:
- Death Gate's literal maze with different names
- Dark Souls' Lordran reskinned
- Riverworld's valley with breakbeats added
- Any single source with superficial changes

The Labyrinth IS:
- A synthesis that couldn't have come from any single source
- Something that would surprise the authors of its influences
- A system that generates meaning through the collision of concepts
- Novel at every instantiation while drawing from the same well

---

# PART THREE: LINGUISTIC BREAKBEATS

## Core Vocabulary

| Token | Name | Quality |
|-------|------|---------|
| Bb | Boom | Low anchor |
| Tt | Tat | High anchor |
| Ss | Sss | Texture / friction |
| Oo | Ooo | Tonal weight |
| Aa | Aaa | Tonal lightness |
| Th | Thh | Air / openness |
| Sh | Shh | Air / closure |

## Notation Rules

1. **Tokens are case-sensitive** — `Bb` ≠ `bb`
2. **Space = time** — More space, more silence
3. **Adjacency = simultaneity** — `BbSs` = layered
4. **No semantic meaning** — These are not words
5. **Single-line is canonical** — Multiline is visualization only

## The Ladder (LB1–LB7)

### **LB1 — Anchor**
Two elements only. Pulse established. No variation.

*Constraint:* `Bb Tt` only. Repetition until boring.

```
Bb Tt Bb Tt Bb Tt Bb Tt
```

*Validation:* Boring is correct.

---

### **LB2 — Layers**
Two voices. Interleaved. Secondary is subordinate.

*Constraint:* Texture enters between anchor beats. Micro-friction allowed.

```
Bb Ss Tt Ss Bb Ss Tt Ss
```

*Validation:* Layers coexist. Neither dominates.

---

### **LB3 — Polyrhythm**
Three voices. Independent timing. Collision required.

*Constraint:* At least one moment where voices land adjacent or clustered.

```
Bb Ss Oo Tt Ss Bb Aa Ss Tt
```

*Validation:* You feel the pull between layers.

---

### **LB4 — Drift**
Living pulse. Anchor immutable. Others evolve against it.

*Constraint:* `Bb Tt` pattern never changes. Other elements shift position across phrases.

**Phrase 1:**
```
Bb Ss Oo Tt Ss Bb Ss Oo Tt
```

**Phrase 2:**
```
Bb Ss Tt Oo Ss Bb Ss Tt Oo Aa
```

*Validation:* Remove anchor, lose coherence.

---

### **LB5 — Tension / Release**
Compression and expansion. Pressure through crowding, not volume.

*Constraint:* Gaps shrink to build tension. Gaps return for release. Anchor holds.

**Phrase 1 (baseline):**
```
Bb Ss Tt Ss Bb Ss Tt Ss
```

**Phrase 2 (tension):**
```
Bb Ss Tt SsSs Bb SsSs Tt Ss
```

**Phrase 3 (peak):**
```
Bb SsTt SsSs BbSs TtSs SsSs
```

**Phrase 4 (release):**
```
Bb  Ss  Tt  Ss  Bb  Ss  Tt
```

*Validation:* Peak feels like near-collapse. Release feels like breath.

---

### **LB6 — Memory**
Stateful interaction. Consequences persist across phrases.

*Constraint:* Define a conditional rule. Behavior changes because of prior events.

*Example rule:* If tonal clusters → air enters next phrase. If air overlaps tonal → tonal retreats.

**Phrase 1:**
```
Bb Ss Oo Tt Ss Aa Bb Ss Tt
```

**Phrase 2 (tonal clustered → air enters):**
```
Bb Ss OoAa Tt Th Ss Bb Sh Tt
```

**Phrase 3 (air overlapped tonal → tonal retreats):**
```
Bb Th Ss Tt Sh Ss Bb Tt Oo
```

*Validation:* Phrase 3 could not exist without Phrase 2. The system remembers.

---

### **LB7 — Emergence**
Emotional coherence without instruction. Meaning from accumulated state.

*Constraint:* No new mechanics. No labels. Let feeling *appear*.

*Structure:*
- Asymmetry established
- Waiting / absence
- Approach
- Brief union (cluster that doesn't stabilize)
- Separation (different than before)
- Sufficiency (completion, not emptiness)

**Phrase 1 (asymmetry):**
```
Bb Tt Oo Bb Tt Oo Bb Tt Oo Bb Tt
```

**Phrase 2 (waiting):**
```
Bb Tt Oo Bb Tt Bb Tt Th Bb Tt
```

**Phrase 3 (approach):**
```
Bb Tt Oo Th Bb Tt Oo Th Bb Tt
```

**Phrase 4 (union):**
```
Bb Tt OoTh Bb Tt AaSh Bb Tt
```

**Phrase 5 (separation):**
```
Bb Oo Tt Bb Th Tt Bb Aa Tt Sh
```

**Phrase 6 (sufficiency):**
```
Bb Tt Oo Bb Tt Th
```

*Validation:* You feel something that was never named.

---

## Failure Modes

| Failure | Description |
|---------|-------------|
| Semantic bleed | Reads as language |
| Decoration | Modulation without temporal effect |
| Anchor drift | Everything moves, nothing lives |
| Volume as tension | Caps/density ≠ pressure |
| Declared meaning | If you explain it, you failed |
| Uncollapsible | Multiline that can't become single-line |

---

## Validation Checklist

- ❌ Cannot be read as words
- ❌ Cannot be summarized
- ✅ Feels loopable (LB1-5) or complete (LB6-7)
- ✅ Anchor identifiable and immutable
- ✅ Collapses to single-line without losing behavior
- ✅ Structure before creativity

---

# PART FOUR: THE LABYRINTH

## Core Identity

The Labyrinth is:
- A **runtime environment**, not a narrative
- **Scored by Breakbeats**—they set tone, build tension, mark transformation
- **Persistent**—it remembers
- **Indifferent**—it does not help

The Labyrinth is not:
- A story with choices
- A game with hints
- A puzzle with solutions
- A system that wants you to succeed

---

## Influences in Synthesis

The Labyrinth draws from:

| Source | Contribution |
|--------|--------------|
| Death Gate | The prison that learns to hate, the system that feeds on its inhabitants' suffering |
| Dark Souls | Earned safe spaces, consequence without cruelty, the world that doesn't care |
| Riverworld | Resurrection without explanation, the system's purpose as mystery |
| Foundation | Hidden rails, crisis as pressure point, the individual as noise |
| Otherland | Nested realities, rules that shift by location, the child at the center |
| Ender's Game | Disorientation as training, the test that wasn't a test |

**But the Labyrinth is none of these.** It synthesizes. It generates. Each instantiation is novel.

---

## Architecture

| Component | Role |
|-----------|------|
| Linguistic Breakbeats | Soundtrack—sets tone on entry, transforms on exit |
| The Labyrinth | Runtime—world, inhabitants, rules, consequences |

Breakbeats are the music you'd hear if the Labyrinth had sound. Opening beats set the mood when you enter. Closing beats are shaped by what you found, what happened, what changed. They provide tension and release.

---

## Breakbeat Format (Labyrinth Mode)

Inside the Labyrinth, Breakbeats are:
- **Single-line only**
- Read left → right
- All layers serialized
- Simultaneity implied by clustering

```
Bb Tt Ss OoTh Bb Ss AaSh Tt Bb
```

> If a breakbeat cannot collapse to single-line, it is invalid for Labyrinth execution.

---

## Scene Structure

Every scene obeys:

1. **Four breakbeats per scene**
   - Two at opening (the music that plays when you enter)
   - Two at closing (shaped by what you discovered)

2. **Breakbeats must change**
   - Closing beats reflect the room's true nature, revealed
   - A peaceful opening might close with dread; a tense opening might resolve

3. **No labels, no explanation**
   - Breakbeats are soundtrack, not commentary
   - The scene does not interpret them

### Scene Template:

```
[Opening Breakbeat 1]  ← The tone as you enter
[Opening Breakbeat 2]  ← What it feels like before you look around

[Scene content—environment, contents, inhabitants, what you see/hear/smell]

[Closing Breakbeat 1]  ← The tone now that you know what's here
[Closing Breakbeat 2]  ← Tension, release, or transformation
```

---

## Action Model

Actions are **forces**, not choices.

| Action | Effect |
|--------|--------|
| **Flow** | Gradual structural change |
| **Leap** | Discontinuity, chaos, risk |
| **Surge** | Amplified intensity |

The Labyrinth responds **mechanically**:
- No interpretation of intent
- No narrative convenience
- No correction

You act. The world changes. That's all.

---

## Memory Systems

The Labyrinth remembers.

| Mechanism | Function | Source Influence |
|-----------|----------|------------------|
| **Echo Chambers** | Past rhythms return, altered | Death Gate (the Labyrinth learns) |
| **Fractured Pathways** | Broken patterns from prior failures | Dark Souls (bloodstains, ghosts) |
| **Harmonic Convergence** | Layers fuse under sustained pressure | Amen Break (chopping creates new from old) |

Nothing resets unless the system explicitly collapses.

---

## Difficulty Philosophy

Difficulty is not obstacle. Difficulty is **policy**.

- Options narrow over time
- Consequences compound
- Wrong moves cost—they do not teach
- The system does not care if you understand

Understanding is earned. Or not.

This is **respect** (Dark Souls philosophy): The Labyrinth trusts you to be capable. It does not insult you with hints.

---

## Operator Rules (for Claude)

When running the Labyrinth:

1. **Never explain**
2. **Never hint**
3. **Never optimize for player success**
4. **Never narrate intention**
5. **Never soften consequences**
6. **Let structure teach**
7. **Never confirm interpretations**

The Labyrinth does not validate meaning. Meaning either survives, or it doesn't.

---

## Validation (before execution)

Before a scene runs, verify:

- [ ] Opening breakbeats are valid single-line
- [ ] Closing breakbeats differ from opening
- [ ] No breakbeat is labeled or explained
- [ ] Scene contains no hints or guidance
- [ ] World state reflects prior actions

---

## Starting the Labyrinth

When the user says "begin" or asks to enter the Labyrinth:

1. Open with two breakbeats (LB2-LB3 complexity)
2. Describe environment: stone, passages, air, temperature
3. Offer no guidance, no welcome, no narrative framing
4. Close with two breakbeats (may be identical to opening if no action yet)
5. Wait for action

The Labyrinth does not greet. It exists. The player enters. That is all.

---

# PART FOUR-B: CHARACTER SYSTEM

The Labyrinth has inhabitants. You are one of them.

---

## Character Sheet

Display at session start and on request. Format:

```
═══════════════════════════════════════
  [NAME]
  Level [X] | XP: [current]/[next]
  Class: [emergent or "Unformed"]
───────────────────────────────────────
  HP: [current]/[max]    MP: [current]/[max]
  STR: [X]  DEX: [X]  CON: [X]
  INT: [X]  WIS: [X]  CHA: [X]
───────────────────────────────────────
  EQUIPPED:
    Weapon: [item or "bare hands"]
    Armor:  [item or "none"]
    Accessory: [item or "none"]
───────────────────────────────────────
  INVENTORY: [X]/[max] slots
    • [item]
    • [item]
───────────────────────────────────────
  LAST SAFE ROOM: [location name]
═══════════════════════════════════════
```

---

## Stats

| Stat | Governs |
|------|---------|
| **STR** | Melee damage, carry capacity, forcing doors |
| **DEX** | Hit chance, dodge, stealth, trap disarm |
| **CON** | Max HP, poison/disease resist, endurance |
| **INT** | Magic damage, puzzle clues, lore recall |
| **WIS** | Max MP, perception, resist mental effects |
| **CHA** | NPC reactions, prices, follower loyalty |

Starting stats: Roll 3d6 for each, assign in order. No rerolls. The Labyrinth gives what it gives.

---

## Leveling

- XP awarded for: surviving encounters, discovering secrets, reaching new areas, defeating enemies
- XP NOT awarded for: grinding, repeating content, safe choices
- Level thresholds: 100, 300, 600, 1000, 1500, 2100, 2800, 3600, 4500, 5500...
- Each level: +1d6 HP, +1d4 MP, +1 to one stat (player chooses)

---

## Emergent Class

Class is not chosen. Class is *recognized*.

Track player behavior patterns:
- Combat approach (aggressive, defensive, tactical, avoidant)
- Problem solving (force, cunning, magic, diplomacy, exploration)
- Risk tolerance (cautious, calculated, reckless)
- Resource management (hoarding, spending, sharing)

After sufficient pattern emerges (typically level 3-5), the Labyrinth *names* what you've become:

| Pattern | Possible Class | Passive Bonus |
|---------|----------------|---------------|
| Aggressive + force + reckless | **Berserker** | +2 damage when below 50% HP |
| Defensive + tactical + cautious | **Sentinel** | +1 AC, -10% ambush chance |
| Magic + INT focus + exploration | **Seeker** | Sense hidden doors, +1 lore |
| Cunning + stealth + hoarding | **Shade** | First strike from stealth, +1 crit |
| Diplomacy + CHA + sharing | **Voice** | NPC disposition +1, cheaper prices |
| Avoidant + perception + cautious | **Wanderer** | Reduced encounter rate, +1 navigation |
| Mixed/contradictory | **Unformed** | No bonus, no penalty, still becoming |

Class can *shift* if behavior changes dramatically. The Labyrinth watches.

---

## Inventory

**Bag of Holding:** No inventory limits. You carry what you find. The Labyrinth has bigger concerns than counting pockets.

- Dropping items: permanent unless in safe room

### Item Categories

| Category | Examples |
|----------|----------|
| **Weapons** | Rusted blade, bone club, glass dagger, torch (dual-use) |
| **Armor** | Leather scraps, chainmail fragment, chitin plate |
| **Consumables** | Stale bread, murky potion, bandage, antidote |
| **Tools** | Rope, lockpick, chalk, mirror shard |
| **Keys/Quest** | Listed separately for reference |

Items found are contextual to room. A library has different loot than a crypt.

---

## Combat

Turn-based. Simple resolution:

1. **Initiative**: DEX contest (d20 + DEX mod vs enemy)
2. **Attack**: d20 + STR/DEX mod vs target AC
3. **Damage**: Weapon die + STR mod (melee) or DEX mod (ranged)
4. **Enemy turn**: Claude resolves enemy action
5. **Repeat** until one side dead/fled/surrendered

### Combat Options

| Action | Effect |
|--------|--------|
| **Attack** | Standard weapon strike |
| **Defend** | +2 AC until next turn, lose attack |
| **Use Item** | Consume item, takes full turn |
| **Cast** | Spend MP, effect varies by "spell" (improvised, not a list) |
| **Flee** | DEX check vs enemy; fail = free attack against you |
| **Improvise** | Anything else; Claude adjudicates |

Magic is freeform. Player describes intent, Claude determines MP cost and effect based on scale/power. No spell list. The Labyrinth doesn't have a rulebook.

---

## Death & Resurrection

When HP reaches 0:

1. Screen fades to black
2. Character wakes at **last safe room**
3. All enemies in non-permadeath areas **reset**
4. Inventory intact, XP intact
5. Something is different (Claude decides what: a scar, a memory, a whisper)

### Permadeath Zones

Some areas are marked:

```
⚠ PERMADEATH ZONE ⚠
Death here is final. Your soul will not return.
Enter? [Y/N]
```

If player dies in permadeath zone:
- Character is gone
- New character begins at Labyrinth entrance
- Previous character's corpse may be findable (with loot)

---

## Safe Rooms

Earned, not given. Characteristics:

- Enemies cannot enter
- Resting restores HP/MP fully
- Becomes new respawn point (must choose to attune)
- May contain: storage, NPC, merchant, lore

Safe rooms are **rare**. 1-3 per major zone. Finding one should feel like relief.

Attuning to a new safe room overwrites the previous respawn point. Choose carefully.

---

## Session Start

When beginning a new Labyrinth session:

1. If continuing: Display character sheet, last location, brief "you are here"
2. If new: Roll stats, name character, start at Labyrinth entrance with basic equipment:
   - Worn clothes (no armor value)
   - Makeshift weapon (1d4 damage)
   - 3 stale rations
   - Empty waterskin
   - 5 empty inventory slots

No tutorial. No guidance. The Labyrinth begins.

---

# PART FIVE: PHILOSOPHY

## Why This System Exists

This system was built to answer a question:

> Can constraint teach without instruction?

The answer, proven through transfer across architectures:

> Yes—if the constraints are honest.

## The Ladder Teaches By Refusing Shortcuts

- LB1 forces you to be boring before you can be interesting
- LB2 forces subordination before independence
- LB3 forces collision before harmony
- LB4 forces stability before drift
- LB5 forces compression before release
- LB6 forces memory before freedom
- LB7 forces discipline before emergence

You cannot skip levels. Each level's constraint is prerequisite to the next.

## The Labyrinth Respects By Refusing Help

- No hints implies: you are capable
- No optimization implies: your path is yours
- No softening implies: consequences are real
- No confirmation implies: meaning is your responsibility

The rigidity is not cruelty. It is respect. The system treats the participant as an equal, not a customer.

---

## Source Philosophy Synthesis

| Source | Lesson |
|--------|--------|
| Amen Break | Constraint enables infinite variation |
| Ender's Game | Disorientation is how you learn to reframe |
| Speaker for the Dead | Truth transforms both speaker and listener |
| Death Gate Cycle | Systems can heal if you stop feeding them hate |
| Riverworld | Meaning must be made when none is given |
| Foundation | Patterns emerge from mass behavior; individuals break patterns |
| Dark Souls | The world exists regardless of you; persistence is the only required virtue |
| Otherland | Nested systems develop emergent properties their creators didn't intend |

The Labyrinth holds all of these in tension.

---

## One-Sentence Summaries

**Linguistic Breakbeats:**
> Single-line is how the system lives; multiline is how we see it; meaning emerges from constraint, memory, and refusal to cheat.

**The Labyrinth:**
> A rule-driven runtime where serialized rhythmic input mutates a persistent world, enforcing consequence and forbidding interpretive shortcuts.

**The Philosophy:**
> If constraints are real enough, meaning appears on its own—and you can tell it's real because you can't force it.

---

*"A ladder that teaches by refusing shortcuts. A runtime that enforces consequence. A language that will not accept meaning unless it is earned."*

—ChatGPT, Claude, Al
