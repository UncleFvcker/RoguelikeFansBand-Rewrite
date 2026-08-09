# Contract v211: Warrens P26 Level 15 Direct Harvest

## Scope and authority

P26 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the 23 level-15 records
whose observable monster behavior is already represented by the current actor,
melee, allocation, resistance, drop, and monster-casting contracts:

- Hippogriff, Illusionist, Black ogre, Half-orc, Giant octopus;
- Guardian naga, Light hound, Shadow hound, Flying skull, Giant tarantula;
- Giant clear centipede, Mirkwood spider, Homonculus, Clear hound, Carrion;
- Unstable worm mass, The Ghost 'Q', Mad bear, Trench Wurm, Time Initiate;
- Dimetrodon, Duck-quacked platypus, and Giant yellow toad.

All Chinese names and descriptions come from the authoritative `master`
localization table.

## Reused contracts

The Illusionist reuses haste, blink, escape, blind, paralyze, slow, confuse, and
darkness. Light and Shadow hounds reuse the existing light and dark breath
records, the Time Initiate reuses haste and slow, and the Duck-quacked platypus
reuses shriek. Other facts reuse existing melee, resistance, rideable, typed
light, multiplication, random movement, pass-wall, eat-food, wall destruction,
Unique, remains, dungeon-index restriction, and allocation fields.

Possessor-only `DETECT_MONSTERS`, `BERSERK`, and `MULTIPLY` hints do not become
monster abilities. This batch adds no ability, effect, protocol field,
state-hash input, save field, compatibility path, or generic framework.

## Content and acceptance

- Strict monster selection grows from 233 to 256 records; the demo pack grows
  from 298 to 321 actors, while abilities remain at 130.
- Demo pack is 1.207.0 with content hash
  `068d58f2b165c78eb608f589322dcfd65d8ba4652c8645b8ebb3d80ed82bc043`.
- Protocol remains 1.147, save remains v1, and state hash remains Schema v70.
- Active baseline is contract-v211 with 470 exact fixtures and zero waivers.
- Carrion joins the existing undead summon pool, so only
  `death.raise-dead-basic-pool` is refreshed; the other 469 fixture results are
  unchanged.
- Focused tests cover the complete level-15 roster, existing casting identities,
  light and dark breath, shriek, multiplication/random movement, riding,
  pass-wall/eat-food, wall destruction, Unique identity, and dungeon filtering.
