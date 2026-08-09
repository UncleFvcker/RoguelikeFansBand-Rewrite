# Wilderness W5: Original extension mechanisms

Status: implemented current-content slice. Protocol `1.146`, state-hash Schema
`69`, active baseline `contract-v199`; demo pack remains `1.193.0` with the W2
content hash.

## Authoritative source

Rules were checked from the `master` Git ref in `D:/codex/Frogcomposband/master`:
`src/wild.c`, `src/cmd2.c`, `src/bldg.c`, `src/cave.c`, `src/spells3.c`, and
`lib/edit/v_info.txt`. The implementation keeps the original `1/10` eligible
wilderness-room chance, the low-level `Ruined Home` layout, eight-direction
travel, mount-based movement, deep-water overload damage, lava damage,
snow damage/regeneration suppression, and the pet/recall prompts.

## Runtime slice

- Eligible low-level grass/dirt/desert cells without a road or formal location
  deterministically contain the original `Ruined Home` terrain layout. Daytime
  entry emits the original discovery message. Existing habitat allocation
  supplies occupants; no second encounter-content registry is introduced.
- `TravelWorld { destination }` recalculates one authoritative eight-direction
  path step per command. Every step reuses the W2 132x clock and W3 ambush
  pipeline. `worldTravelDestination` is saved, hashed, and projected; selecting
  a world cell with `x` and Enter starts travel, and uppercase `J` resumes it
  after an ambush, a return to the world map, or save load.
- An unmounted player may enter deep water as in RFB, but takes drowning damage
  while carrying over capacity. Mountains and deep lava still require a
  capable active traveller; shallow/deep lava apply fire damage, while snow,
  glacier, and pack ice can damage an unprotected player and suppress recovery.
- Entering the world map requires explicit flags when player-controlled pets
  would be left or recall is active. The ridden actor is excluded from the pet
  prompt and continues through the existing W2 mount transfer path. Cancelling
  recall preserves its destination and clears only the active countdown.

## Content gates

RFB's random single-level forest/volcano/mountain/sea/snow dungeons begin at
roughly levels 20–50 and reference dungeon/content sets not present in the
current formal pack. They remain inactive rather than being replaced by a
generic shallow floor. Town teleport likewise requires another visited formal
`TownDefinition`; Outpost is currently the only town, so no fake destination or
dead-end service is exposed. These gates are content work, not missing generic
runtime abstractions.
