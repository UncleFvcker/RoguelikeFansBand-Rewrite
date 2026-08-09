# Wilderness W3: day, night, and ambushes

## Authoritative source and clock

W3 follows RFB `master` in `src/xtra1.c`, `src/cmd2.c`, `src/wild.c`, and
`src/monster1.c`. One original day is `TURNS_PER_TICK * TOWN_DAWN = 100000`
world ticks. The first half is daytime and the second half is nighttime; the
display clock applies the original quarter-day offset, so tick 0 is day 1 at
06:00, tick 50000 is 18:00, and tick 100000 is day 2 at 06:00.

The clock is derived only from the existing `worldTick`. No second time field
is saved or hashed. Surface environment light is 48 by day and 0 by night.
The same day/night result excludes monsters with Light vulnerability from
wilderness allocation during daytime, including eligible group companions.

## Original ambush roll

Each successful world-map step into a non-town coordinate evaluates the RFB
ambush gate after movement. With player level `P`, smoothed coordinate danger
`D`, and stealth skill `S`, the roll denominator is:

`max(1, 120 + P * 10 - D + 5)`

Road coordinates multiply that denominator by 8; nighttime divides it by 2.
The level gate remains `D + 5 > P / 2`, and an ambush occurs when a bounded
roll is below `max(0, 21 - S)`. Formal town coordinates never roll. The
original Thrall-only deep-water alternative is outside the current player-mode
boundary and is not represented by a speculative flag.

## Encounter transition and threat lock

An ambush immediately returns to the current coordinate's existing `96x32`
local wilderness. It reuses W2 terrain, smoothed danger, P10 habitat,
allocation, grouping, and movement checks, but performs the original 20
encounter allocation attempts instead of the world's ordinary surface count.
Encounter generation uses the coordinate seed with one fixed ambush salt, so
it is deterministic without consuming the simulation RNG beyond the ambush
roll itself.

The interrupted world movement spends one normal action (`100` energy), not
the completed world's `13200`; the existing scheduler therefore advances the
new local monsters before the player is ready again. Spawn IDs contain the
stable `.ambush.` marker. While any living hostile marked actor or its direct
hostile summon remains, Core rejects `EnterWorldMap` and the frontend hides the
same action. The lock is derived from current actors, so save, replay, and state
hash need no additional boolean. Clearing the hostile encounter actors clears
it naturally.

The encounter emits the existing event DTO shape as `wilderness.ambushed` /
`wilderness-ambushed`, with English and Simplified Chinese presentation.

## Versions and deferred boundary

W3 changes no DTO or state-hash input structure: protocol remains `1.145`,
save/replay containers remain v1, and state-hash Schema remains v68. The demo
pack remains `1.193.0` with content hash
`5df3ee0a7bace5b35b805cd0ce22c0d373e3b9fbf38b379b8724d6bf64061b46`.
Because daytime eligibility changes common surface initialization, the exact
fixture baseline advances once to contract-v198.

Formal location entry, additional source towns and dungeons, and the omitted
Thrall start mode remain later slices. W3 does not add encounter tables,
persist local maps for all world coordinates, or introduce a second clock or
renderer.
