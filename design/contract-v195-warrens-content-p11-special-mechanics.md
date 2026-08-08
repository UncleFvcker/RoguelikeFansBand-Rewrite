# Contract v195: Warrens P11 low-reuse special mechanics

## Scope and authority

P11 follows the RFB `master` Git objects at commit
`efd63661302866038f58d8cd2553b23e6af3bf9d`. It implements only the four
low-reuse facts needed by the selected shallow records: `KILL_BODY`,
`RANGED_MELEE`, `RIDING`, and `SILVER`. It does not introduce a general
material system, monster inventory abstraction, or a second combat routine.

## Monster movement and reach

`KILL_BODY` lets an actor treat a weaker living actor as a traversable route
candidate when its own `experienceValue * level` is strictly greater. On the
actual step it executes the existing actor-to-actor melee routine and remains
in place for that action. The current player mount is never a valid body
target, and the actor must be able to enter the blocked cell's terrain.

`RANGED_MELEE` reuses the complete melee routine at RFB's two-grid shape:
Chebyshev distance 2 with the smaller axis distance below 2. The diagonal
corner `(2, 2)` is excluded. Confusion and fear suppress the reach, and the
intervening projectile line must be free of terrain and actors.

## Riding

`GameCommand::Ride { direction }` mounts an adjacent `rideable` actor when the
level check succeeds, or dismounts the current mount into the selected empty
adjacent cell. The Web command is `V` followed by a direction. A successful
mount becomes player-controlled and leaves any monster pack; this is the
bounded current-product equivalent of the original pet/forced-riding entry
because no separate animal-taming command exists yet.

The mounted actor shares the player's cell, does not receive autonomous
energy actions, supplies its base speed, and uses its movement domain for
player movement. Normal relocation, teleportation, and floor transitions move
the mount with the player. Dismounting leaves the mount on the old cell. Mount
identity is projected, saved, hashed, and validated; death, expiry, or direct
removal clears it.

RFB's Sheep special case remains authoritative: each attempt consumes one
`bounded(3)` draw and emits one of the three exact Chinese refusal messages;
the player never mounts it.

## Silver and content

`madeOfSilver` records the source `SILVER` fact directly. No silver damage hook
is added because the formal player roster has no silver-vulnerable form.

The strict monster selection grows from 116 to 123. P11 adds 哭闹的恶心物、
新手考古学家、爬行银币、巨型鼻涕虫、马、难以驯服的马和绵羊. Existing
恰克波 also receives its authoritative `rideable` fact. Chinese names and
descriptions follow `master:src/monster_name_zh.inc` and
`master:lib/edit/r_info.txt` exactly.

The shallow formal roster is now 155 actors, 123 maintained by the strict
selection path; 18 surveyed level 1–9 records remain deferred. The whole demo
pack contains 188 actors, 146 items, 98 abilities, 78 terrains, and 19 loot
tables.

## Compatibility and acceptance

- Protocol is 1.143. `GameCommand` adds `ride`, and `PlayerDto` projects the
  optional `ridingActorId`.
- Save remains v1. `PlayerSaveDto.ridingActorId` is required; old development
  saves are not compatible. State hash advances to Schema v66.
- Demo pack is 1.190.0 with content hash
  `72e709d0f66adba524769d31809d1747f73daea7d5aeff1ccaf5744531f73f1b`.
- Active baseline is contract-v195 with 470 exact fixtures and zero waivers.
- Focused coverage locks body attacks, two-grid melee, mount movement and
  save round trips, dismounting, and the Sheep refusal.
