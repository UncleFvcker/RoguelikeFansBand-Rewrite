# Wilderness W1: authoritative map state and display

## Original behavior and scope

W1 follows the authoritative RFB `master` behavior in `src/dungeon.c`,
`src/wild.c`, and `src/xtra2.c`: `<` enters the world map from the surface,
`>` returns to the local map, roads/towns/dungeon entrances override the base
wilderness display, and look mode reports the source wilderness danger level.

This slice deliberately stops before travel. Switching scale is therefore a
zero-time authoritative state change: advancing hidden local monsters before
W2 defines world travel time would be incorrect.

## Authoritative state and protocol

`Game` and save v1 now carry `mapScale`, `wildernessPosition`, and
`wildernessSeed`. All three enter state-hash Schema v68; saves reject a world
scale outside the initial surface, a missing wilderness, or an edge/out-of-map
position. Replay records the explicit `enter-world-map` and `leave-world-map`
commands and verifies the same final hash.

Protocol 1.145 adds:

- `MapScaleDto` (`local` or `world`) on snapshots and updates;
- the two explicit scale commands;
- optional `dangerLevel` and `locations` metadata on existing `CellDto` cells.

The world projection reuses `width`, `height`, `cells`, `visualCells`, and the
projected player position. At the current W0 start this is the authoritative
`99 x 66` map at `(28,52)`, whose cell exposes danger level 0 and both the
Outpost and Warrens locations. No parallel map DTO or renderer exists.

## Frontend and command boundary

The existing Pixi renderer resizes its existing `RenderWorld`; the existing
camera, zoom, and `x` look cursor continue to operate on the projected cells.
The shared connection button and keyboard mapping use `<` to enter and `>` to
leave. Look text includes terrain, danger level, and active location names.

While `mapScale` is `world`, Core accepts only `leave-world-map`. The frontend
also blocks movement, pickup, combat/targeting, inventory mutation, shops,
tasks, resting, riding, and tactical terrain operations. Local actors, items,
shops, and terrain remain unchanged and are projected again on return.

## Deferred boundary

W2 still owns world movement, travel energy/time, terrain movement domains,
encounters, and changing `wildernessPosition`. Later slices own entering
locations and activating additional original towns/dungeons. W1 does not add
fallback locations or placeholder content.

The demo pack remains `1.192.0` with content hash
`02577f7c9262ee49d7f73ec13e3271a674cedc4e1af297e9359032cfb5532962`.
The runtime advances to protocol `1.145`, state-hash Schema v68, and the
contract-v197 470-fixture baseline; save and replay containers remain v1.
