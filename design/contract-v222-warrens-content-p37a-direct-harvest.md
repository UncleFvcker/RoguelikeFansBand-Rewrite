# Contract v222: Warrens P37A Direct Harvest

## Authority and scope

P37A reads the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It selects monsters by current
mechanism risk rather than by level and adds the 33 records that do not cast
monster abilities.

The batch adds fire spirit; Shagrat and Gorbag; white shark; stunwall; quartz
vein; Monkey of Nikko; tiger snake; Ozmanian devil; stone, aquatic and tin
golems; red mold; Orc digger; lizard king; landmine; wyvern; livingstone;
sabre-tooth tiger; sasquatch; weir; whale; electric eel; werewolf; Ugluk;
noxious fume; nue; spider bomb; glyptodont; frost spirit; blue-ringed octopus;
box jellyfish; and yowie.

Chinese display names use the authoritative `master:src/monster_name_zh.inc`
and `master:lib/help/PossessorStats.csv` entries for the same source indexes.

## Implemented boundary

- All 33 records use the existing strict monster selection and importer.
- Fire and frost spirits reuse elemental contact damage; box jellyfish reuses
  the existing poison contact aura.
- Quartz vein, livingstone and noxious fume reuse deterministic reproduction.
- Landmine and spider bomb reuse self-destruct melee damage.
- Stone creatures reuse disintegration vulnerability; Orc digger reuses wall
  destruction and weaker-body displacement.
- Wyvern and glyptodont reuse riding; aquatic actors reuse the current habitat
  and movement contracts.
- Unique escorts, groups, corpse/skeleton remains and existing themed drops
  reuse their current content definitions.
- `DETECT_OBJECTS`, `DETECT_MONSTERS`, `BERSERK` and spell-side `MULTIPLY` are
  possessor-only tokens and do not create monster casting profiles.

No ability, effect type, protocol field, save field or compatibility path is
added. Display-only flags listed in each strict selection entry remain explicit
omissions.

## Acceptance

- Strict monster selection increases from 300 to 333 records.
- Demo content increases from 365 to 398 actors and remains at 152 abilities.
- Demo pack is 1.218.0 with content hash
  `845f251ddaf432b3e870ae30a365a1777706051a1d9fcd37a6ae8d7f55d17de5`.
- Protocol remains 1.152 and State Hash Schema remains v72.
- Active baseline is contract-v222 with 470 exact fixtures and zero waivers.
