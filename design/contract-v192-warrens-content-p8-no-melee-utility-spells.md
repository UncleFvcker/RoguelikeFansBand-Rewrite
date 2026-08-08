# Contract v192: Warrens P8 no-melee monsters and utility spells

## Scope and authoritative behavior

P8 gives `NEVER_BLOW` a deliberate content representation and adds the first
monster utility spell. An explicit `meleeRoutine` with an empty `blows` array
means the actor has no melee attack; an absent routine continues to request the
existing default innate attack. This keeps the new rule local to source records
that explicitly carry `NEVER_BLOW`.

`SHRIEK` follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`: it excludes the caster, wakes
monsters within twice sight range, and gives 100 ticks of Haste to hostile
monsters in the player's line of sight. The implementation reuses the existing
item aggravation mutation instead of introducing a second wake/haste model.
The effect projects typed `awakened` and `hastened` counts.

## Formal content

The strict selection grows from 86 to 89 monsters:

- Shrieker mushroom patch retains stationary movement, no melee, spawn grace,
  poison resistance, status immunities, corpse remains, and 25% Shriek;
- White harpy retains flight, random movement, three ordered blows, corpse
  remains, and 16% Shriek;
- Culverin retains no melee, random movement, and its every-turn `SHOOT(4d6)`.

Chinese names and descriptions exactly follow
`master:src/monster_name_zh.inc` and `master:lib/edit/r_info.txt`: 尖叫蘑菇丛、
白鹰身女妖 and 射石兽. Shriek uses the original spell name 尖叫.

The shallow formal roster is now 121 actors, 89 maintained by the strict
selection path; 52 surveyed level 1–9 records remain deferred. The whole demo
pack contains 154 actors, 146 items, 96 abilities, and 19 loot tables.

## Deferred utility spells

`DARKNESS` and `TRAPS` remain explicit infrastructure gaps. RFB Darkness clears
room glow, but the rewrite currently derives illumination without persistent
per-cell room glow. RFB Traps creates new traps around the target, but the
rewrite has triggering and removal without a general monster trap-allocation
path. Neither spell is represented as amnesia, an unrelated terrain change, or
a no-op.

## Compatibility and acceptance

- Protocol is 1.141; ability specs and resolutions include
  `aggravate-monsters`.
- Save remains v1 and state hash remains Schema v64; no persisted or hashed
  state changed.
- Demo pack is 1.187.0 with content hash
  `17ed668316b674de5baaa54d9b5a1fd817a7c5d2ea11b8d914dba462215b5359`.
- Active baseline is contract-v192 with 470 exact fixtures and zero waivers.
- Focused core coverage locks empty-melee behavior and Shriek's caster
  exclusion, waking, haste duration, affected cells, and typed counts.
