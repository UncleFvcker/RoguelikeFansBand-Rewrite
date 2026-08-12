# Contract v264: shared RFB base item pool

- `demo.loot-table.base-items` is the shared ordinary item pool for dungeon
  floors, ordinary monster item drops, and acquirement effects.
- Its entries use the authoritative RFB `master` `k_info.txt` allocation pairs:
  `minDepth` is `A.level`, weight is integer `100 / A.chance`, and a positive
  `W.maximumLevel` becomes `maxDepth`. `W.level` remains the item definition's
  `generationLevel` and is not an allocation fallback.
- The initial candidate set is the union of the former Warrens and Orc Cave
  tables. `Satisfy Hunger` and `Blessing` have no `A:` record, so the resulting
  pool contains 98 item kinds and 110 allocation entries rather than inventing
  entries for all 100 candidates.
- The pool uses the RFB depth quality policy with good/great caps `75/20`.
- The `9:none / 1:Slaying` affix weights are a temporary global approximation
  while the remaining original egos are not implemented. They are not an
  original RFB probability claim.
- Fixed rewards and monster theme tables remain separate.
