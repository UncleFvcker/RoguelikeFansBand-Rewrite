# Contract v264: shared RFB base item pool

- `demo.loot-table.base-items` is the shared ordinary item pool for dungeon
  floors, ordinary monster item drops, and acquirement effects.
- Its entries use the authoritative RFB `master` `k_info.txt` allocation pairs:
  `minDepth` is `A.level`, weight is integer `100 / A.chance`, and a positive
  `W.maximumLevel` becomes `maxDepth`. `W.level` remains the item definition's
  `generationLevel` and is not an allocation fallback.
- The pool contains every active RFB source item that has an `A:` record:
  249 item kinds and 283 allocation entries. The ten active source items without
  an `A:` record remain available only through their explicit shop, reward, or
  other acquisition paths.
- Source index 313 is one original `Staff` allocation adapted into two formal
  device items. Its first ledger mapping owns the one base-pool allocation so
  the split does not double the original source weight; both formal items keep
  their existing shop and Mage-theme paths.
- The pool uses the RFB depth quality policy with good/great caps `75/20`.
- The `9:none / 1:Slaying` affix weights are a temporary global approximation
  while the remaining original egos are not implemented. They are not an
  original RFB probability claim.
- Fixed rewards and monster theme tables remain separate.
