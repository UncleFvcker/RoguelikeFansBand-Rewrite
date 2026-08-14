# Location L0: Anambar and Orc Cave authority

## Selection boundary

`legacy-wilderness-selection.json` schema 6 separates active locations from
their authoritative town and dungeon plans. Outpost, Anambar, Warrens, and
Orc Cave are active.

The town plan and planned dungeon bind the route to the authoritative RFB
`master` Git objects:

- town 5, `阿南巴`, at `(26,39)`, sourced through `t_info.txt` and
  `t_ana.txt`;
- dungeon 3, `Orc cave`, at `(30,45)`, with depths 15–32;
- `MONSTER_DIV_16`, preferences `ORC | R_CHAR_oOTC | ANIMAL | TROLL`;
- generation flags `CAVE | WATER_RIVER | CAVERN | LAKE_TREE | DESTROY | BIG`;
- guardian 1185, `Othrod, Lord of the Orcs` / `半兽人之王奥斯罗德`, level 32;
- final object `(tval 45, sval 0)`, final ego 206, and substitute dungeon 36.

The Anambar town plan locks standard feature symbols 0–9 to the nine standard
shops plus shared Home, and locks the imported owners, prices, commands,
memberships, and source actions. L2 activates the town with supported stock;
P104 adds the shroomery and the library's research, identification, identify-all,
and town-overview services. P105 adds the Weapon Master, Warrior Guild, Mammon
Temple, Archer Guild, and Trump Tower with typed recovery, enchantment, armor
assessment, and recall services. The plan remains the source-drift check for
those imported facts.

## Machine-checked completion

After contract-v260, `audit-demo-monsters` keeps the completed Orc Cave selection
machine checked. The active allocation set contains 194 tagged records including
guardian 1185; 28 Camelot, other dedicated-dungeon, wilderness/ocean, or
fixed-unique records remain excluded from Orc Cave while already imported global
content stays available for its proper locations. Dungeon 3 now owns its 15–32
floor chain, global allocation policy, depth-compatible loot, guardian lifecycle,
and final Ring + ego 206 reward.
Anambar shop stock is no longer a gap: L2 exposes only item kinds whose current
runtime behavior is complete, and P104 gives its shroomery the same supported
mushroom stock as Outpost. Home has no stock and shares the Outpost storage.
The inn uses the existing shop transaction path for food and drink, so its
entrance is functional. The library uses explicit facility commands for normal
identification, full research, and identify-all; town overview is projected as
localized client information and consumes no turn. Lodging, rumors, town
teleport, and reputation remain deferred until those services have authoritative
runtime state. P105 facilities likewise use explicit zero-energy commands:
visitors retain access because every imported building action has restriction 0,
while Owner membership selects the owner price and the original higher guild
enchantment limit. A source price of 0 on weapon, armor, and bow enchantment maps
to the original 1500-gold minimum (750 for Owner); ammunition retains its
declared 44/22 gold per stack unit. Unsupported casino, bank, police, and
quest-building entrances are not drawn.

The focused `sync-demo-wilderness` command validates every imported fact against
the authoritative `master` objects before rewriting the world, preserves authored
town `mapOrigin` values, and emits only active locations. Source drift therefore
fails the sync instead of silently changing the playable route.

## Version boundary

L0 itself changed importer metadata only. L2 activates Anambar in pack 1.224.0.
Contract-v260 activates Orc Cave in pack 1.251.0. P104 updates Anambar in pack
1.368.0 and Protocol 1.222; State Hash Schema remains v104 and save schema remains
v2. P105 updates Anambar in pack 1.369.0 and Protocol 1.223; State Hash Schema
remains v104 and save schema remains v2.
