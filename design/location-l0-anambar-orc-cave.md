# Location L0: Anambar and Orc Cave authority

## Selection boundary

`legacy-wilderness-selection.json` schema 3 separates active locations,
authoritative town plans, and planned dungeons. Outpost and Anambar are active;
only Orc Cave remains planned.

The town plan and planned dungeon bind the route to the authoritative RFB
`master` Git objects:

- town 5, `阿南巴`, at `(26,39)`, sourced through `t_info.txt` and
  `t_ana.txt`;
- dungeon 3, `Orc cave`, at `(30,45)`, with depths 15–32;
- `MONSTER_DIV_16`, preferences `ORC | R_CHAR_oOTC | ANIMAL | TROLL`;
- generation flags `CAVE | WATER_RIVER | CAVERN | LAKE_TREE | DESTROY | BIG`;
- guardian 1185, `Othrod, Lord of the Orcs` / `半兽人之王奥斯罗德`, level 32;
- final object `(tval 45, sval 0)`, final ego 206, and substitute dungeon 36.

The Anambar town plan locks the standard feature symbols 1–9 to the eight
standard shops plus shared Home, and locks the inn owner, access rule, prices,
commands, and five source actions. L2 activates the town with supported stock;
the plan remains the source-drift check for those imported facts.

## Machine-checked gaps

The same selection keeps three explicit deferred groups:

1. Orc Cave allocation candidates from levels 21–32.
2. Generic dungeon loot, guardian 1185, and the final object/ego reward.
Anambar shop stock is no longer a gap: L2 exposes only item kinds whose current
runtime behavior is complete. Home has no stock and shares the Outpost storage.
The inn uses the existing shop transaction path for food and drink, so its
entrance is functional. Lodging, rumors, town teleport, and reputation remain
deferred until those services have authoritative runtime state; unsupported
casino, bank, police, and quest-building entrances are not drawn.

The focused `sync-demo-wilderness` command validates every imported fact against
the authoritative `master` objects before rewriting the world, while emitting
only the active location arrays. Source drift therefore fails the sync instead
of silently activating incomplete content.

## Version boundary

L0 itself changed importer metadata only. L2 activates Anambar in pack 1.224.0
without changing protocol projections or state-hash input.
