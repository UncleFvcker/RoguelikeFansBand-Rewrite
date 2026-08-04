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
