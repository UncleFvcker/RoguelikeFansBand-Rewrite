# Contract v265: global RFB monster drop themes

- `demo.loot-table.warrior` replaces the former Kobold-, Warrens-, and Orc
  Cave-specific Warrior tables. All 58 existing Warrior theme bindings use it
  with the existing 50 percent theme-selection chance.
- The formal `warrior`, `archer`, `mage`, `priest`, `evil-priest`, `paladin`,
  `dwarf`, and `ninja` tables contain every currently implemented RFB item that
  matches the corresponding predicate in authoritative RFB `master`
  `efd63661302866038f58d8cd2553b23e6af3bf9d` and has an `A:` allocation.
- Every entry uses `A.level` as `minDepth`, integer `100 / A.chance` as weight,
  and a positive `W.maximumLevel` as `maxDepth`. Multiple allocation pairs are
  retained and dungeon depth caps are not added.
- All eight tables use the RFB depth quality policy with good/great caps
  `75/20`. Warrior retains `9:none / 1:Slaying` as the current ego
  approximation; the other themes retain an explicit no-affix fallback.
- The original Warrior hook admits rings and amulets with a per-kind
  `one_in_(3)` check. Static theme membership records kinds that the hook can
  admit and retains their `A:` weights; it does not add a second random filter.
- `DROP_WARRIOR_SHOOT` continues to use the existing Archer approximation.
  A dedicated Warrior Shoot table remains outside this batch.
- Retired bespoke Kobold/Archer entries that do not match a formal predicate
  are not kept as fake themed acquisition paths. Expanding the shared base pool
  beyond its current initial candidate set is a separate content batch.
- Fixed reward tables remain static and separate, including Othrod's Fine
  Combat ring and the Warrens reward.
