# Contract v194: Warrens P10 movement domains, invisibility, and surface habitats

## Scope and authority

P10 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It adds the smallest shared
runtime surfaces required by the shallow `PASS_WALL`, `AQUATIC`, `INVISIBLE`,
and `WILD_*` records. It does not treat every impassable terrain as a wall,
allow aquatic actors on ordinary land, or expose hidden entity IDs to the
client.

## Movement domains

`PASS_WALL` is a content movement mode. A terrain must separately declare
`allowsWallPassage`; ordinary walls, closed doors, veins, rubble, and
non-permanent building walls opt in, while permanent walls, Outpost
fortifications, and deep water do not. Pathfinding, direct movement, summon
placement, generated group placement, and stored-floor actor placement all
use the same crossing predicate. Passing through a wall does not damage or
transform it.

`AQUATIC` is distinct from the existing `swim` capability. Aquatic actors may
enter terrain tagged `water` and may not enter ordinary walkable land. An
actor that is both aquatic and flying may also enter walkable or fly-enabled
terrain, matching the original flying exception.

## Invisibility

An actor tagged `invisible` is omitted from `EntityDto` and from the cell's
`actorId` unless the cell is otherwise visible and the actor's authoritative
`visibleInvisible` flag is set. Position- or direction-based attacks still
resolve against an unknown occupied cell; commands that name an unseen entity
ID are rejected by the existing target validation boundary.

Each equipped non-tool source of the `see-invisible` passive performs the
original check when the player enters a new visibility situation or the
invisible actor moves:

```text
bounded(50 + monsterLevel / 2) < playerSearchSkill
```

A successful source makes the actor visible. A stationary actor retains that
result while it remains in view; leaving visibility or losing all sources
clears it. `ActorSaveDto.visibleInvisible` preserves the current result across
current-floor and stored-floor save/load without re-rolling.

## Surface allocation and content

World content may declare bounded surface allocation rolls and a level.
Eligible actors must have ordinary allocation metadata and either be
`WILD_ONLY` with a terrain-matching habitat or be aquatic on water. Rarity,
level, maximum depth, living-Unique availability, movement, friends, escorts,
and pack behavior remain authoritative. The Outpost surface adds open
woodland and a shallow/deep pond, with grass, wood, town, shore, swamp, and
water tags used by this allocator.

The strict monster selection grows from 97 to 116. P10 adds 透明恶心物、巨型绿蛙、
乌鸦、吵闹鬼、渡鸦、食人鱼、透明蠕虫团、剑鱼、巨型水蛭、绿色贪吃鬼、
巨型粉红蛙、迷失的灵魂、太空怪兽、幻影战士、受伤的熊、僧帽水母、血牙狼、
老鹰和地缚灵. These names and their Chinese descriptions follow
`master:src/monster_name_zh.inc` and `master:lib/edit/r_info.txt` exactly.

迷失的灵魂 also requires `DRAIN_MANA(1d6)`. The formal `drain-resource`
melee effect drains an available player casting resource by the rolled amount
and heals the attacker by six times the amount actually drained; it does not invent
damage when no resource is available.

The shallow formal roster is now 148 actors, 116 maintained by the strict
selection path; 25 surveyed level 1–9 records remain deferred. The whole demo
pack contains 181 actors, 146 items, 97 abilities, 78 terrains, and 19 loot
tables.

## Compatibility and acceptance

- Protocol is 1.142. `EquipmentPassiveDto` adds `see-invisible`, and
  `ActorSaveDto` adds required `visibleInvisible`.
- Save remains v1. State hash advances to Schema v65.
- Demo pack is 1.189.0 with content hash
  `91008be8add20b2a75cbdc7f73dbd5267e3beb9e9a31b9dc3fae31c2805dcc35`.
- Active baseline is contract-v194 with 470 exact fixtures and zero waivers.
- Focused coverage locks aquatic/land and pass-wall/permanent-wall boundaries,
  invisible projection and persistence, formal content counts, and the full
  imported allocation roster.
