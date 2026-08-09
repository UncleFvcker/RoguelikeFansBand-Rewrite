# Contract v213: Warrens P28/P29 Ant Summon and Target Blink

## Scope and authority

P28/P29 follow the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. They add two level-15 records:

- Plaguebearer of Nurgle (`纳垢携疫者`);
- Gnome mage (`侏儒法师`).

Their English identities follow the authoritative `N:` records. Chinese names
and descriptions exactly follow the authoritative localization table and `D:`
records.

## Narrow mechanics

P28 maps `S_ANT` to the existing summon-category effect. The generated stable
ability `summon-ant-l15-1d3-1` summons `1d3+1` actors from category `ant`, with
maximum summoned level 15. No new summon framework or fallback is added.

P29 adds the monster-only `blink-target { radius }` effect and generates
`blink-other` with radius 10. It traces the existing projectile target, builds
a row-major list of unoccupied passable cells within Chebyshev distance 10 of
the target's current position, and consumes one bounded RNG draw to choose the
destination. Player and summon relocation reuse their existing authoritative
paths. An empty candidate list leaves the target in place; the effect never
falls back to `teleport-away` or another displacement approximation.

The Plaguebearer otherwise reuses fear, slow, and `8d8` curse content. The
Gnome mage otherwise reuses self blink, darkness, `6d8+5` cold bolt, and the
level-15 single-monster summon. Possessor-only `DETECT_MONSTERS` and
`DETECT_TRAPS` hints do not become monster abilities.

## Content and acceptance

- Strict monster selection grows from 264 to 266 records; the demo pack grows
  from 329 to 331 actors and from 141 to 143 abilities.
- Demo pack is 1.209.0 with content hash
  `b0f60081b2b1971d643f93c619df721c43997661a496a8f4549b6bac8ce16cde`.
- Protocol advances to 1.148 for `AbilityEffectSpecDto::BlinkTarget`; save
  remains v1 and state hash remains Schema v70.
- Active baseline is contract-v213 with 470 exact fixtures and zero waivers.
- Adding the undead Plaguebearer changes only the existing raise-dead pool
  fixture; the other 469 fixture results remain exact and unchanged.
- Focused tests lock actor spell rosters, summon parameters, target-blink
  radius, deterministic candidate selection, relocation, event projection,
  and the single destination RNG draw.
