# Phase 19: Legacy Item Integration

The fixed-source importer remains the authority for original RFB item identity,
weight, base value, equipment slot, combat dice, and defense. Formal gameplay
content is selected explicitly rather than copying the complete generated pack.

## Reproducible Selection

`packs/rfb-demo-original/legacy-item-selection.json` pins every selected item by
both fixed-source index and normalized id. Regenerate the selected item files
with:

```powershell
$env:RFB_LEGACY_SOURCE='D:/codex/Frogcomposband/master'
cargo run -p rfb-legacy-import -- sync-demo-items packs/rfb-demo-original/legacy-item-selection.json packs/rfb-demo-original/items
```

The command reads the pinned legacy commit, rejects index/name drift, duplicate
selection entries, and items with active behavior or unmapped functional flags.
It emits formal `demo.item.*` definitions and preserves original base values.
Town membership is expressed by the formal shop definitions, so the legacy
`TOWN` generation flag is allowed but not copied.

## First Batch

The first batch contains Dagger, Main Gauche, Rapier, Mace, Robe, Soft Leather
Armour, Metal Cap, and Large Leather Shield. All eight are sold by the matching
Outpost forge shop. The six original level 0-5 items also enter the Warrens loot
table at matching shallow depth bands; Metal Cap and Large Leather Shield remain
shop-only because their original levels are 10 and 15.

Artifacts, launchers/ammunition, devices, books, jewelry, light sources, and
items with incomplete active behavior remain outside this integration batch.

## Second Batch

The second batch extends the same explicit selection with Club, Whip, Tanto,
Small Sword, Cutlass, Cloak, Pair of Hard Leather Boots, Knit Cap, Small Metal
Shield, Soft Studded Leather, Cord Armour, and Padded Armour. All twelve enter
their matching Outpost forge shop. The eleven original level 0-5 items enter the
Warrens loot table at their fixed-source depth; the level 10 Small Metal Shield
remains shop-only.

Weapons carrying the legacy `RIDING` flag remain excluded until mounted combat
exists.

## Third Batch

The third batch adds Set of Studded Leather Gloves, Set of Gauntlets, Hard
Leather Armour, and Hard Studded Leather. Their original non-weapon `P:` hit
and damage values map to melee-only equipment bonuses: hard leather body armour
applies -1 hit, studded gloves apply +1 damage, and gauntlets apply +1 hit and
+1 damage. These bonuses do not affect shooting. The original level 5 gloves
and body armour enter the Warrens at depth 5; the level 10 pieces remain
armoury-only.

The refreshed complete fixed-source audit still compiles with 937 items, 122
affixes, 1,260 abilities, and 1,332 actors. Its content hash is
`84af1d0207367535d0742bdd695dc2b35ac686d7ec6463f0dbf27349f6101560`.

After the first three batches, the formal demo selection contained 24
fixed-source items. The demo pack was version 1.161.0 with content hash
`fd43ce44259adb782a4cffe9656111c650d0e2392bd5fdaf80f6016107f166d7`.

## Fourth Batch

The fourth batch first moves Short Sword, Chain Mail, Leather Gloves, Soft
Leather Boots, Hard Leather Cap, and Small Leather Shield under the fixed-source
selection while preserving their established demo ids. `sourceId` records the
normalized legacy name when that name is longer than the stable demo id. This
corrects Short Sword to its original 1d8 profile and Chain Mail to its original
-2 melee hit modifier.

Shovel and Pick then add the first utility-weapon choice. Both grant +2 digging;
Shovel deals 1d3 and enters Warrens loot at depth 1, while Pick deals 1d5 and
enters at depth 5. Both are sold by the Outpost Weaponsmith.

The formal demo selection now contains 32 fixed-source items. The demo pack is
version 1.162.0 with 137 items and content hash
`26042aed35ade88a712001d6af450785fbc771dd77ea855b38969bea22746c50`.

## Fifth Batch

The fifth batch adds the original Fabric Bag, Leather Pouch, and Dwarven
Backpack at fixed-source indexes 722-724. Every player has the original 26
shared inventory slots. Equipping one container expands that same inventory by
4, 8, or 12 slots respectively; containers do not own nested item lists.
Fabric Bag is the first container sold by the General Store.

Human body templates and the engine fallback body now expose `container` and
`tool` slots. Shovel and Pick can explicitly equip into either `tool` or
`weapon`. In `tool`, only their +2 digging bonus applies; their stored melee
profile, affixes, and all other equipment properties do not contribute. In
`weapon`, the complete melee profile and equipment properties apply, matching
the original utility-weapon choice.

The formal demo selection now contains 35 fixed-source items. The demo pack is
version 1.163.0 with 140 items and content hash
`d9e227cc7757ff82a66c7afadf8da2846a1751920f53fa3f1f0a74c640b8a0ac`.

## Sixth Batch

The sixth batch adds Broken Dagger, Broken Sword, Filthy Rag, Pointy Hat, and
Paper Armour from fixed-source indexes 45, 46, 246, 225, and 248. All five are
passive equipment with no unresolved active behavior. They enter the Warrens
loot table at their original object levels: the three damaged level-zero items
at depth 0, Pointy Hat at depth 3, and Paper Armour at depth 5.

This batch deliberately uses dungeon drops rather than expanding Outpost shop
stock. The formal selection now contains 40 fixed-source items. The demo pack
is version 1.179.0 with 146 items and content hash
`066a3e92b8ec8698438876ef1f264e106151ad22e06c93a1bd435829f0ade8ff`.

## Seventh Batch

The seventh batch changes reachability rather than item identity. Leather
Gloves enter the general Warrens table at original allocation depth 1; Soft
Leather Boots, Hard Leather Cap, and Small Leather Shield enter at depth 3.
All four were already fixed-source formal items and all have original
allocation rarity 1. They remain in their existing shops and Warrior-theme
drop table while becoming ordinary floor and general monster drops.

The formal selection remains 40 items. The demo pack is version 1.180.0 with
146 items and content hash
`7051765d5f3e57bf1967c1e305d63eb6949ca90cbb87ceb24b2a384e6162de93`.

## Eighth Batch Boundary

The level-four and level-five monster batch adds no item identity. The formal
selection already contains every passive fixed-source equipment kind eligible
for ordinary Warrens allocation at depth five or shallower, and those kinds
already appear in the general table at their source depth. Contract v186 adds
only new consumers of those tables, including the existing Warrior-theme table.

The formal selection remains 40 items. The demo pack is version 1.181.0 with
146 items and content hash
`da4f82376baca9fce9d3f8e4728bae42f5be60cdc69048f14e94b81458b68ab8`.

## Ninth Batch Boundary

The P3 caster and simple-Unique batch adds no item identity. Its four Unique
actors use only already formal ordinary/good drops and remains; no source
artifact is approximated or invented. The formal selection remains 40 items.
The P4 level-six and level-seven and P5 level-eight and level-nine monster
batches add no item identity. The formal selection remains 40 items. The demo
pack is version 1.184.0 with 146 items and content hash
`9d6be77bac135d2dad8f6c6067f34750c57f02121f905e8606197c2d043d606d`.

## Tenth Batch Boundary

P6 adds no item identity. It makes already-formal shallow items reachable from
the original 50-percent Mage, Archer, Priest, Evil Priest, and Paladin monster
drop branches through five explicit loot tables. Each table contains only the
currently formal subset accepted by the corresponding RFB `master` predicate
at Warrens depths; unsupported or deep items are not approximated.

The formal selection remains 40 fixed-source items and the pack remains at 146
items. The demo pack is version 1.185.0 with content hash
`e7a7697de6aab4160c2398cba429559fa7fd62c46b65f3bb929490d859395f3e`.
