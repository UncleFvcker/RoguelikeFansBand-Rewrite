# Wilderness W4: Outpost and Warrens location loop

## Original behavior

W4 follows RFB `master` in `src/cmd2.c`, `src/floors.c`, and `src/wild.c`.
A dungeon is not entered from wilderness travel mode. The player first returns
to the local map and then uses an `FF_ENTRANCE` terrain whose `special` value
identifies that dungeon. On the first ascent, RFB restores the dungeon's
wilderness coordinate and finds the matching entrance on the regenerated local
surface.

The rewrite keeps that boundary without porting the original `3x3 cave[][]`
scrolling cache. World scale still rejects `TraverseStairs`; it only supports
travel and returning to local scale.

## Active location binding

W0 already records both active locations at `(28,52)`:

- `demo.town.outpost`
- `demo.dungeon.warrens`

Outpost continues to use the existing fixed `demo.floor.surface`, including
its map, shops, Home, facilities, tasks, actors, and the Warrens stair at
`(74,16)`. A local stair transition into a dungeon now also requires the
current wilderness coordinate to contain a `Dungeon` location with the same
dungeon ID. The existing stair therefore opens Warrens at `(28,52)` but cannot
be reused after the world position changes.

This is a runtime authority check over the W0 location data. It adds no second
entrance registry and does not infer dungeon identity from coordinates alone.

## Return semantics

The existing floor transition already saves the complete source floor before
entering a dungeon. Returning from Warrens restores that saved surface, whose
player position is the exact entrance cell used for departure. The independent
`wildernessPosition` remains `(28,52)` across the dungeon transition. Task and
shop state are global authoritative state and are preserved by the same
transition.

No new return coordinate, entrance marker, save migration, or compatibility
path is needed.

## Versions and deferred boundary

W4 changes no protocol, save/replay container, state-hash input, or content
definition. Protocol remains `1.145`, state-hash Schema remains v68, the demo
pack remains `1.193.0` with content hash
`5df3ee0a7bace5b35b805cd0ce22c0d373e3b9fbf38b379b8724d6bf64061b46`,
and the exact fixture baseline remains contract-v198.

Orc Cave is not activated by W4. Its formal route, entrance terrain, guardian,
dungeon floors, and recall destination remain deferred until the corresponding
level 10–15 monsters and dungeon content exist. Other source towns and dungeons
are likewise added one vertical slice at a time rather than as placeholders.
