# Contract v185: Warrens Content Batch 2

Status: active baseline. Protocol `1.139`, demo pack `1.180.0`, save v1, and
state hash Schema v63. This is a content-only milestone and adds no development
save compatibility path.

The built-in content hash is
`7051765d5f3e57bf1967c1e305d63eb6949ca90cbb87ceb24b2a384e6162de93`.

## Authoritative source and selection

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, resolved
through Git objects to commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. The formal selection is pinned by
source index and normalized id in
`packs/rfb-demo-original/legacy-warrens-monster-selection.json`.

`sync-demo-monsters` rejects index/name drift, duplicate selections, active
monster spells, unsupported melee effects, undeclared source flags, and stale
omission declarations. It generates only the selected `demo.actor.*` files:

```powershell
$env:RFB_LEGACY_SOURCE='D:/codex/Frogcomposband/master'
cargo run -p rfb-legacy-import -- sync-demo-monsters packs/rfb-demo-original/legacy-warrens-monster-selection.json packs/rfb-demo-original/actors
```

Chinese display names exactly follow `master:src/monster_name_zh.inc`; Chinese
descriptions follow the selected records' `D:` text.

## Second formal monster batch

Fourteen supported level-two and level-three monsters enter the global Warrens
allocation pool: Metallic Green Centipede, Giant Black Ant, Salamander, Slimy
Worm Mass, Large Yellow Snake, Cave Spider, Slimy Ooze, Metallic Blue
Centipede, Giant White Louse, Spotted Mushroom Patch, Giant White Ant, Yellow
Mold, Metallic Red Centipede, and Yellow Worm Mass.

Their formal definitions retain original index, level, rarity, maximum depth,
experience, HP dice, speed, armor, ordered blows, group dice, multiplication,
random movement, flying/swimming, door bashing, resistances, status
immunities, `DROP_90`, and corpse/skeleton choices. Cave Spider's
`DETECT_MONSTERS` and the two worm/louse `MULTIPLY` spell tokens are
possessor-only metadata in the original and do not create monster casting.

The selection file records every omitted non-W1-W13 flag exactly. These are
wilderness habitats, `STUPID`, `WEIRD_MIND`, `EMPTY_MIND`, and `POS_GAIN_AC`;
none is represented by a gameplay tag.

## General Warrens drops

Four already fixed-source items now enter the general floor and monster drop
table at their original allocation depths: Leather Gloves at depth 1, and Soft
Leather Boots, Hard Leather Cap, and Small Leather Shield at depth 3. All four
have original allocation rarity 1, so they share the existing ordinary weight.
No new item definition or active item behavior is introduced.

## Verification

The selective monster sync, fixed item sync, deterministic source inspection,
content lock verification, importer/content suites, workspace checks, and only
the behavior-affected Warrens fixtures form the acceptance boundary. Protocol,
save, and state-hash inputs do not change.
