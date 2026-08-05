# Contract v180: Monster Death Drops

Status: active baseline. Protocol `1.137`, demo pack `1.174.0`, save v1, state
hash Schema v63. Old development saves are not supported.

The built-in content hash is
`fed9c01421e0ee68a6cde5d0b864aee32f4a218d58457cc0d0d06ab6b7d6334f`.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Monster death content composes `DROP_60`, `DROP_90`, `DROP_XD2`,
  `ONLY_ITEM`, `ONLY_GOLD`, `DROP_GOOD`, and `DROP_GREAT` without hiding the
  roll count in an item table.
- Non-unique, non-great counts above two use the original compression
  `2 + (count - 2) / 2`. A `DROP_90` roll is guaranteed for Unique monsters.
- `ONLY_ITEM` and `ONLY_GOLD` constrain each generated drop. Otherwise each
  drop has the original 20-percent gold choice. Eligible monsters can replace
  the general item table with their class-theme table on the original
  50-percent branch.
- Minimum quality is applied before affix generation. Corpse/skeleton remains
  stay in the existing independent death stage after generated drops.
- The importer creates general and Warrior-theme item allocation tables and
  maps original drop flags into the same death definition.

## Verification

The Warrens keeper test independently proves `1d2`, item-only, minimum-good
quality. Existing probability/remains tests retain their separate scope.
`combat`, `monsters`, `dungeon`, `tasks`, and `system` total 171 relevant exact
fixtures and all verify unchanged; no fixture was refreshed for v177-v180.
