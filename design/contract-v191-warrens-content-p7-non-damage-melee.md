# Contract v191: Warrens P7 non-damage melee effects

## Scope

P7 adds six explicit ordered melee effects to `MeleeBlowEffectDefinition`:
`Blind`, `Confusion`, `Paralysis`, `Slow`, `Stun`, and `Terrify`. Each effect
retains its source-local optional chance. Confusion can carry its source damage
dice as well as the status rider; Stun carries duration dice. The importer no
longer flattens `CONFUSE` into damage-only behavior.

The runtime applies the same effect sequence to player and actor targets. It
reuses the established status IDs, stacking, status immunity, and confusion
resistance paths rather than adding a second status model. Blind lasts 12–15
ticks, confusion adds 11–30 ticks before existing resistance scaling,
paralysis lasts 1–3 ticks, slow uses the established 25-tick slow application,
Stun rolls its source dice, and Terrify uses source actor level with the
existing +3 Unique adjustment. Optional effect chance is resolved before the
effect's own rolls. The P6 `nice` cap continues to apply to damage-bearing
confusion and rolled Stun against the player.

The authoritative behavior and selected records were checked from the RFB
`master` Git objects in `D:/codex/Frogcomposband/master`. This rewrite keeps its
single typed status system: deterministic status immunity replaces a separate
legacy player saving-throw layer, and Slow maps to the already-established
timed slow status.

## Formal content

The strict shallow selection adds ten records whose remaining active behavior
is now completely expressible:

- Floating eye, Yellow mushroom patch, Brown mold, Bloodshot eye;
- Lagduf, the Snaga, Green mold, King cobra, Broken death sword;
- Balcmeg, the Relentless and Giant moth.

Bloodshot eye also generates the source-parameterized four-point Drain Mana
ability. Blinking dot remains manually authored but its `CONFUSE(1d6)` blow is
corrected from confusion damage alone to confusion damage plus the status
rider. Giant slug, Space monster, Portuguese man-o-war, and Poltergeist stay
deferred because they still require independent body-kill, wall passage,
aquatic-only, or ground-item pickup behavior.

The shallow formal roster is now 118 actors, 86 of them maintained by the
strict selection/sync path; 55 surveyed level 1–9 records remain deferred. The
whole demo pack contains 151 actors, 146 items, 94 abilities, and 19 loot
tables.

## Compatibility and acceptance

- Protocol remains 1.140 and save remains v1.
- State hash remains Schema v64; no state-hash input changed.
- Demo pack is 1.186.0 with content hash
  `d1cfa9470d91e068baf2bb47ddc2c0c0ad8b1a6dfe8822b02d3127b4d03e4317`.
- Active baseline is contract-v191 with 470 exact fixtures and zero waivers.
- Focused core coverage locks all six player status results and confusion
  damage; importer coverage locks dice-less confusion and independent chances.
