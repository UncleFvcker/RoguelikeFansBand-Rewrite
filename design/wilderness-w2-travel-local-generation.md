# Wilderness W2: travel and local generation

## Authoritative source and boundary

W2 follows the RFB `master` implementation in `src/cmd2.c`, `src/wild.c`,
`src/monster1.c`, and `lib/edit/f_info.txt`:

- a world-map step spends normal movement energy multiplied by
  `(MAX_HGT + MAX_WID) / 2`, which is `132`;
- a formal town or dungeon coordinate uses its source danger level, while an
  ordinary coordinate uses the non-edge cells in its surrounding `3x3` area;
- local wilderness ecology follows the terrain habitat hook (water, shore,
  waste, grass, wood, volcano, mountain, snow, or swamp);
- roads extend only toward adjacent road coordinates.

This slice preserves those rules without porting the original `3x3 cave[][]`
scrolling implementation. It does not cache all `99x66` local maps and does not
activate source towns or dungeons that have no formal content in this project.

## World travel

`Move` is now valid at world scale. The target must be a non-edge wilderness
coordinate that the active player or mount can cross through the existing
terrain movement-domain rules. A successful or blocked attempt spends
`100 * 132` scheduler energy. The resulting world ticks continue to process
player statuses, hunger, regeneration, light fuel, equipment regeneration, and
device recovery. Existing shop maintenance observes the advanced world clock
when a shop is next visited.

Hidden actors on the stored local floor do not move, attack, tick statuses, or
lose spawn grace while the player is on the world map. This prevents an
unprojected local simulation from attacking the player during travel.

## Coordinate-seeded local wilderness

Leaving the world map on an ordinary coordinate activates the single dynamic
floor ID `core.floor.wilderness`. The local map keeps the world's established
`96x32` tactical dimensions. One `wildernessSeed` and the current `(x,y)` are
mixed into a coordinate seed; terrain and initial monster placement therefore
rebuild identically without storing 6,534 RNG states.

The current biome fills the interior. Cardinal neighbor biomes form a small,
deterministically irregular boundary band, and a road is carved from the center
only toward cardinal road neighbors. Local monsters reuse the P10 surface
habitat, allocation level, rarity, uniqueness, grouping, and movement checks.
The smoothed current danger level supplies the allocation level. Generation
uses a temporary coordinate RNG and restores the simulation RNG afterward.

Walking beyond a local wilderness edge advances the world coordinate and
generates only the neighbor, placing the player one cell inside the opposite
edge. The departed ordinary coordinate is discarded. Returning to the Outpost
coordinate restores the one preserved town surface floor rather than generating
a wilderness substitute.

## Persistence and compatibility

W2 uses the W1 `mapScale`, `wildernessPosition`, and `wildernessSeed` fields plus
the existing current/stored floor structures. A save may contain one active
`core.floor.wilderness` and the preserved initial town surface; inactive
ordinary wilderness coordinates are never stored. Save/replay containers stay
at v1, protocol stays at `1.145`, and state-hash Schema stays at v68 because no
protocol or hash input structure changed.

The demo pack advances to `1.193.0` and adds eight local terrain definitions.
Its content hash is
`5df3ee0a7bace5b35b805cd0ce22c0d373e3b9fbf38b379b8724d6bf64061b46`.
The contract-v197 exact baseline remains valid; W2 behavior is covered by
focused core and frontend tests.

## Deferred boundary

Later slices own explicit entry into formal towns and dungeons, activation of
additional original locations, and world-travel encounter interruptions. W2
does not create placeholder locations, persist an all-world local-map cache, or
add a second renderer or map protocol.
