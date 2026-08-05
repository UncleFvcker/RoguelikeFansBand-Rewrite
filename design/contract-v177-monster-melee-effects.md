# Contract v177: Monster Melee Effects

Status: active slice in the cumulative contract-v180 baseline. Protocol
`1.137`, demo pack `1.171.0`, save v1, state hash Schema v63. Old development
saves are not supported.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- A melee blow owns an ordered effect list. Damage, poison, disease,
  attribute drain, and bleeding keep their source dice and independent chance.
  Resolution stops when the target dies and does not execute later effects.
- Physical `HURT` effects use armor. Other projected damage types use their
  resistance without silently inheriting physical armor mitigation.
- An `EXPLODE` blow that hits skips its single-target effects and kills the
  attacker. A miss does not destroy it.
- Every ordinary death path also checks the first `EXPLODE` blow. Its projected
  effects roll before corpse and loot generation and affect the radius-three
  footprint in stable distance/position order. Nearby deaths can chain through
  the same death pipeline.
- Explosion blows accept only effects that the current projected damage
  vocabulary can express. Unsupported status-only explosion effects remain an
  importer gap instead of receiving invented behavior.

## Verification

Focused combat tests independently cover declared blow order, skipped direct
damage on a successful exploding blow, and radius-three explosion on an
ordinary death. The `combat` and `monsters` fixture categories remain unchanged
and verify without refresh.
