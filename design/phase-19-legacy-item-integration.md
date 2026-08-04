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

The refreshed complete fixed-source audit still compiles with 937 items, 122
affixes, 1,260 abilities, and 1,332 actors. Its content hash is
`2de99da81ead52c794264a537f64caa40d038b596ef2961bf36f345f19959e4b`.
