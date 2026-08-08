# Contract v196: Warrens P12 special lifecycles

## Scope and authority

P12 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It closes only the lifecycle
facts needed by Nami, Shadower, Goomba, and the five shallow `DEPRECATED`
records. Existing faction targeting, terrain transformation, monster
allocation, and actor save paths remain the owners of the behavior.

## Friendly Nami and monster traps

`ActorDefinition.friendly` marks an autonomous monster as player-side without
making it player-controlled. Friendly actors do not target the player or
player-side actors; hostile monsters can target them, their snapshot faction
is `friendly`, and clear-floor objectives ignore them. They still use the
ordinary monster scheduler, casting, movement, melee, loot, and Unique
lifecycle.

航海士娜美 keeps the original level 8 Unique record, `FRIENDLY`, spawn grace,
swimming, doors, item pickup, theft blows, drops, `HEAL`, and `TRAPS`. The
monster `TRAPS` spell reuses `transform-terrain`: it targets a hostile actor
through line of effect and changes eligible unoccupied floor cells within
radius 1 into the existing `demo.terrain.warren-snare`. It does not replace
stairs, connections, occupied cells, items, borders, or non-floor terrain.

## Shadower appearance

追踪者 is a non-allocating appearance definition, not an independent shallow
enemy. Each ordinary original-allocation spawn has the RFB `1/333` appearance
chance only when its real actor is level 10 or above and is not Unique. The
real `kindId`, statistics, AI, loot, death, and Unique state remain unchanged;
only the projected entity kind uses the appearance. The optional appearance
kind is saved, hashed, and validated so reloads cannot reroll or reveal it.

Fixed encounters, guardians, summons, reproduction, and the Shadower
definition itself do not receive this draw.

## Content selection and deprecated records

板栗崽 enters ordinary shallow allocation with its original index, level,
rarity, hit points, bite, corpse, and text. The strict selection grows from
123 to 126 records; the shallow formal roster grows from 155 to 158.

The selection manifest permanently binds five obsolete records to their
active same-name replacements:

- 43 → 110 Novice warrior
- 46 → 93 Novice mage
- 83 → 142 Novice ranger
- 97 → 147 Novice paladin
- 1053 → 1054 Novice mindcrafter

Synchronization rejects a selected `DEPRECATED` source, a non-deprecated old
side, a deprecated replacement, mismatched names, duplicate mappings, or a
replacement absent from the strict selection. These five records are not
content omissions and never become standalone actor definitions.

Ten active surveyed shallow records remain outside the P12 selection:
Greater Hell-Beast, Duck, Yellow jelly, Silver jelly, Zog, Disenchanter eye,
Dark elf, Wormtongue, Robin Hood, and Lousy. P12 does not broaden its scope to
those independent content decisions.

## Compatibility and acceptance

- Protocol is 1.144. `EntityFactionDto` adds `friendly`, and
  `ActorSaveDto.appearanceKindId` is optional.
- Save remains v1. The appearance field enters state hash Schema v67; old
  development saves are not compatible.
- Demo pack is 1.191.0 with 191 actors, 146 items, 100 abilities, 78 terrains,
  and 19 loot tables. Content hash is
  `c3440aa696805626dcde6222cc058bcb12b7b0f8a9213fd4f2ff8f7d5f28fdea`.
- Active baseline is contract-v196 with 470 exact fixtures and zero waivers.
- Existing content, importer, protocol, and 418 core tests pass; no new test
  fixture is added for this batch.
