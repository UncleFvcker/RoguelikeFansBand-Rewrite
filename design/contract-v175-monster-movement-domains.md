# Contract v175: Monster Movement Domains

Status: active slice in the cumulative contract-v176 baseline. Protocol
`1.137`, demo pack `1.170.0`, save v1, state hash Schema v63. Old development
saves are not supported.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Actor movement modes are strong content fields, not classification tags.
  Newt, Giant White Mouse, Rock Lizard, Cave Lizard, and Night Lizard swim;
  Fruit Bat and the Hunting Hawk of Julian fly. Chiokovo does not fly because
  the pinned original does not give it `CAN_FLY`.
- Terrain independently declares accepted modes. Deep and shallow resonance
  water accept `fly` and `swim`; normal walkable terrain remains available to
  every actor.
- Pursuit, retreat, random movement, reproduction, initial and ambient
  allocation, friend/escort placement, fixed/category summons, animation,
  displacement, and save validation consume the same movement query.
- Trap avoidance is explicit per trap through `avoidedByMovementModes`.
  Flying is not a universal trap immunity. No built-in trap gains an avoidance
  mode without a matching original rule.
- A monster entering a non-avoided trap receives the trap's typed damage. A
  matching explicit movement mode suppresses that trigger.

## Verification

The `monster_movement` unit module independently covers water entry for walkers,
swimmers, and flyers; explicit trap-mode matching; and non-avoided monster trap
damage. It does not use a movement-heavy contract fixture to establish state.

