# Contract v223: Warrens P37B Existing Abilities

## Authority and scope

P37B reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds 15 monsters whose
monster casting profiles can be represented entirely by existing abilities.

The batch adds Fire, Cold, Energy, Air and Water hounds; Blink dog; Shambling
mound; Giant black and gold dragon flies; Pumpkin man; Huorn; Hopper ant;
Phase spider; Chimera; and Demonite.

Chinese display names use the authoritative `master:src/monster_name_zh.inc`
and `master:lib/help/PossessorStats.csv` entries for the same source indexes.

## Implemented boundary

- The five hound variants, both dragon flies, Chimera and Demonite reuse the
  existing fire, cold, electricity, poison, acid or sound breath abilities.
- Blink dog, Huorn, Hopper ant and Phase spider reuse the existing blink and
  target-drag abilities.
- Shambling mound reuses `shriek`; Pumpkin man reuses the existing confuse,
  paralyze, blind, fear, darkness and `curse-3d8` abilities.
- `DETECT_MONSTERS` remains a possessor-only token and does not create a
  monster casting profile.
- Existing movement, riding, immobility, melee and display-flag omission
  mappings remain unchanged.

No ability record, effect type, protocol field, save field, compatibility path
or generalized framework is added.

## Acceptance

- Strict monster selection increases from 333 to 348 records.
- Demo content increases from 398 to 413 actors and remains at 152 abilities.
- Demo pack is 1.219.0 with content hash
  `c379c1b08743578fee07d0fb0678c3ce1a59ae080e62424ae01e84525ffd322a`.
- Protocol remains 1.152 and State Hash Schema remains v72.
- Active baseline is contract-v223 with 470 exact fixtures and zero waivers.
