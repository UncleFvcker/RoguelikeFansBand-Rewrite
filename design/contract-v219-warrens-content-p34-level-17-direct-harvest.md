# Contract v219: Warrens P34 Level 17 Direct Harvest

## Scope and authority

P34 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the eight level-17
records whose observable behavior is already represented by current content
and runtime contracts:

- Sphinx, Forest troll, 2-headed hydra, and Swamp thing;
- Water spirit, Giant pink scorpion, Earth spirit, and Wutugu, the Chief of
  Southerings.

English identities follow the authoritative `N:` records. Chinese names match
the authoritative runtime localization table exactly. Pack descriptions are
independently written factual summaries.

## Reused contracts

Sphinx reuses fear, confusion, flying, riding, and mountain allocation.
Forest troll reuses regeneration and light vulnerability; its `BERSERK` spell
token remains possessor-only metadata. 2-headed hydra reuses fear, swimming,
riding, and `MOVE_BODY`. Swamp thing and Giant pink scorpion reuse terrify and
attribute-drain melee riders. Earth spirit reuses pass-wall movement and the
existing `HURT_ROCK` to disintegration-vulnerability mapping. Wutugu reuses
`DUNGEON_31`, Unique identity, typed light, and Warrior drops.

The authoritative Water spirit record has `RAND_25`, `CAN_FLY`, and
`NONLIVING`, but no `PASS_WALL`; P34 preserves that distinction. P34 adds no
ability, effect, protocol field, state-hash input, save field, compatibility
path, or generic framework.

## Content and acceptance

- Strict monster selection grows from 284 to 292 records; the demo pack grows
  from 349 to 357 actors, while abilities remain at 147.
- Demo pack is 1.215.0 with content hash
  `795cf896433897af45dbf9b6d7f1519fbec70f8cf926843cefe4179a37f83f97`.
- Protocol remains 1.152, save remains v1, and state hash remains Schema v72.
- Active baseline is contract-v219 with 470 exact fixtures and zero waivers.
- Focused tests lock the complete P34 roster and its reused casting, movement,
  melee, regeneration, resistance, region, Unique, and drop identities.
