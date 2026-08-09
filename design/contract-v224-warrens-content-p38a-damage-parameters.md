# Contract v224: Warrens P38A Damage Parameters

## Authority and scope

P38A reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds Potion mimic, Door
mimic, Uruk, Chaos beastman, Giant bronze dragon fly, Stone giant, Snow
golem, Bush ranger, Frost giant, Earth hound and Dark elven lord.

Chinese display names use the authoritative `master:src/monster_name_zh.inc`
and `master:lib/help/PossessorStats.csv` entries for the same source indexes.

## Implemented boundary

- Eleven generated ability records preserve the source damage dice, flat
  bonus, damage type, area radius or health-scaled breath cap.
- Monsters with identical parameters share the same ability record; the two
  mimics, Frost giant and Dark elven lord all reuse `bolt-cold-6d8-6`.
- Existing blind, confusion, fear, curse, escape, darkness and self-haste
  abilities remain shared rather than duplicated.
- `DETECT_MONSTERS` and `DETECT_TRAPS` remain possessor-only tokens and do not
  enter monster casting profiles.
- Actor movement, auras, melee, resistances, drops and allocation continue to
  use existing importer mappings.

No effect type, runtime branch, protocol field, save field, compatibility path
or generalized parameter framework is added.

## Acceptance

- Strict monster selection increases from 348 to 359 records.
- Demo content increases from 413 to 424 actors and from 152 to 163 abilities.
- Demo pack is 1.220.0 with content hash
  `60ccaee2d902a306b4e3615cfd22b61c98e41e5454af75af16c045566da8d82a`.
- Protocol remains 1.152 and State Hash Schema remains v72.
- Active baseline is contract-v224 with 470 exact fixtures and zero waivers.
