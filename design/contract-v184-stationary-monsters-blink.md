# Contract v184: Stationary Monsters and Blink Binding

Status: active baseline. Protocol `1.139`, demo pack `1.179.0`, save v1, and
state hash Schema v63. The active baseline remains 470 exact fixtures with no
waivers.

The built-in content hash is
`066a3e92b8ec8698438876ef1f264e106151ad22e06c93a1bd435829f0ade8ff`.

## Authoritative source

The source is the `master` Git ref in `D:/codex/Frogcomposband/master`, read
through Git objects at commit `efd63661302866038f58d8cd2553b23e6af3bf9d`.
`RF1_NEVER_MOVE` is defined there as never making a physical move, while the
monster action path still permits adjacent melee and spells. The `BLINK`
monster spell is displayed as `闪现` and teleports its caster within distance
10.

## Contract

- Actor movement definitions add `neverMoves`. It is valid only for monsters
  and cannot be combined with original random-movement percentages.
- A never-moving monster may detect, cast, blink, be displaced, and attack an
  adjacent hostile target, but it cannot chase, flee, keep distance, wander,
  follow, or otherwise take an ordinary physical movement step.
- The legacy importer maps `NEVER_MOVE` into this typed movement rule and no
  longer reports the flag as unresolved.
- `demo.ability.blink` binds the existing `blink-self` program step at the
  original radius of 10.
- Grey Mold (legacy index 20) and Blinking Dot (legacy index 22) enter global
  allocation with their original level, rarity, HP, speed, armor, blows,
  poison resistance, status immunities, remains, and maximum depth. Blinking
  Dot casts Blink at the original 50% frequency.

The pack now contains 65 actors, 146 items, and 70 abilities. Remaining
`STUPID`, `EMPTY_MIND`, `NASTY_GLYPH`, and `POS_GAIN_AC` metadata stays explicit
in the Warrens mechanism backlog rather than being approximated by tags.

## Verification

Acceptance covers stationary ranged behavior, adjacent melee, Blink bypassing
the physical-movement restriction, importer mapping, source compilation and
lock verification, schema freshness, content tests, contract fixtures, and a
standalone desktop build. No protocol DTO, save shape, or state-hash input is
changed.
