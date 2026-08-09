# Contract v216: Warrens P31 Level 16 Direct Harvest

## Scope and authority

P31 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the twelve level-16
records whose observable behavior is already represented by current content
and runtime contracts:

- Pink horror, Rust monster, Orc captain, and Gelatinous cube;
- Giant green dragon fly, Hummerhorn, Lizardman, and Ulfast, Son of Ulfang;
- Hammerhead, Berserker, Ogrillon, and Orc berserker.

English identities follow the authoritative `N:` records. Chinese names match
the authoritative runtime localization table exactly. Pack descriptions are
independently written factual summaries.

## Reused contracts

Pink horror reuses confusion, fear, and their melee riders. Orc captain reuses
`bolt-physical-3d6`; Giant green dragon fly reuses
`breath-poison-17-600-r2`. Rust monster and Gelatinous cube reuse ground-item
destruction and pickup, while Hummerhorn reuses the existing multiplication
allocation fact. The remaining actors reuse current melee, resistance,
aquatic/swimming, group, typed light, Unique, dungeon-index, remains, drop,
and allocation contracts.

Possessor-only `DETECT_OBJECTS`, `MULTIPLY`, and `BERSERK` hints do not become
monster abilities. P31 adds no ability, effect, protocol field, state-hash
input, save field, compatibility path, or generic framework.

## Content and acceptance

- Strict monster selection grows from 267 to 279 records; the demo pack grows
  from 332 to 344 actors, while abilities remain at 143.
- Demo pack is 1.212.0 with content hash
  `08dfc525ebb4f1f2af4e110dcc1490ffc37647f63feb088c0dfa4e8113e22f3f`.
- Protocol remains 1.151, save remains v1, and state hash remains Schema v72.
- Active baseline is contract-v216 with 470 exact fixtures and zero waivers.
- Full verification leaves all 470 fixture results unchanged; no fixture is
  refreshed.
- Focused tests lock the complete P31 roster, reused casting identities, item
  interaction, multiplication, swimming/aquatic movement, and Unique identity.
