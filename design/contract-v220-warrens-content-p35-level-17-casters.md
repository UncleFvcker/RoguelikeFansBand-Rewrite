# Contract v220: Warrens P35 Level 17 Casters

## Scope and authority

P35 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds five level-17 records:
Hill giant, Imp, Nekomata, Grey seer, and Nar, the Dwarf. English identities
follow the authoritative `N:` records, while Chinese names exactly match the
authoritative runtime localization table.

## Parameterized abilities

The existing importer generates exactly five new ability/program pairs:

- `bolt-physical-1d1-50` for Hill giant `THROW`;
- `bolt-fire-9d8-5` for Imp `BO_FIRE`;
- `summon-legacy-import-l17-1d1` for Nekomata `S_MONSTER(1d1)`;
- `kin-grey-seer` for Grey seer `S_KIN`;
- `heal-51` for Nar `HEAL`.

All other spells reuse existing blink, displacement, level teleport, status,
curse, poison-ball, mind-blast, and fear abilities. Grey seer reuses the
existing `COMPOST → demo.task.the-sewer` allocation restriction. No new
ability effect or runtime path is introduced.

## Dwarf drops

`DROP_DWARF` maps through the existing themed-drop field to
`demo.loot-table.dwarf`. The table contains only already imported source-
appropriate candidates: shovel, pick, small metal shield, and amulet. It adds
no loot schema, generation rule, fallback, or compatibility layer.

## Content and acceptance

- Strict monster selection grows from 292 to 297 records; the demo pack grows
  from 357 to 362 actors and from 147 to 152 abilities.
- The pack now contains 20 loot tables.
- Demo pack is 1.216.0 with content hash
  `1a3ff3f7f41da01cc0a0860393b4b6cab1d86d1861f8b32476d2841f095f13c8`.
- Protocol remains 1.152, save remains v1, and state hash remains Schema v72.
- Active baseline is contract-v220 with 470 exact fixtures and zero waivers.
- Focused tests lock all five casting lists, generated effect parameters,
  sewer allocation, Dwarf themed drops, and reused movement/Unique facts.
