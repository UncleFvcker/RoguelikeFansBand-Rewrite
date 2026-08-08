# Contract v186: Warrens Content Batch 3

Status: active baseline. Protocol `1.139`, demo pack `1.181.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`da4f82376baca9fce9d3f8e4728bae42f5be60cdc69048f14e94b81458b68ab8`.

## Authoritative source and selection

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The fixed-index selection in
`packs/rfb-demo-original/legacy-warrens-monster-selection.json` now contains 27
monsters. Chinese names follow `master:src/monster_name_zh.inc` exactly and
Chinese descriptions use the selected `master:lib/edit/r_info.txt` records.

`sync-demo-monsters` now permits the already formal `DROP_WARRIOR` theme and
binds it to `demo.loot-table.large-kobold`; every other unbound source theme is
still rejected.

## Third formal monster batch

Thirteen supported level-four and level-five monsters enter the global
allocation pool: Yellow Light, Frosty Jelly, Creeping Copper Coins, Giant White
Rat, White Worm Mass, Large Grey Snake, Skeleton Kobold, Slush Pile, Slimy
Jelly, Grey Icky Thing, Red Worm Mass, Copperhead Snake, and Novice Warrior.

The batch retains source index, level, rarity, maximum depth, experience, HP
dice, speed, armor, ordered blows, group chance, multiplication, random
movement, stationary/flying/swimming movement, doors, light, resistances,
status immunities, death drops, and remains. Yellow Light keeps its intrinsic
radius-three light and self-destructing light explosion. Novice Warrior keeps
`FRIENDS(2d3, 25%)`, three ordered blows, `DROP_60`, and its 50% Warrior-theme
drop selection.

Only non-runtime metadata is declared omitted: sex, cold-blooded/empty/stupid
mind markers, possessor bonuses, and wilderness habitat. The source
`MULTIPLY` spell tokens remain possessor-only and do not create monster
casting.

## Deferred level-four and level-five records

Records needing active behavior remain outside the formal selection: ranged or
other active monster spells, aquatic-only allocation, wall passing, ground-item
pickup, food/gold/item theft, disenchantment without damage dice, riding, and
unsupported empty blows. Unique Bullroarer is also deferred with theft and
unique-item behavior rather than imported partially.

## Items and drops

No new item identity is required. All fixed-source passive equipment eligible
for Warrens depth five or shallower is already in the formal pack and general
loot table. This batch expands drop behavior through the new monsters:
Creeping Copper Coins drops only gold, Slush Pile uses `DROP_60`, and Novice
Warrior combines `DROP_60` with the existing Warrior-theme table.

## Verification

The selective monster sync, fixed item sync, content lock verification,
importer/content/core checks, and only behavior-affected contract categories
form the acceptance boundary. Protocol, save, and state-hash inputs do not
change.
