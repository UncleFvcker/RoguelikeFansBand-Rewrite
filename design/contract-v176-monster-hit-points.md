# Contract v176: Monster Hit Point Dice

Status: active baseline. Protocol `1.137`, demo pack `1.170.0`, save v1, state
hash Schema v63. Old development saves are not supported.

The built-in content hash is
`b434df67e19e3f7986ee796870365c3e60deb792ff87478a48856194607b75b7`.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Monster content stores the original HP dice. Ordinary births roll every die
  once and assign the result to both instance `maxHp` and current `hp`.
  `FORCE_MAXHP` uses `dice * sides` without consuming RNG.
- Fixed world actors, procedural leaders and groups, guardians, ambient births,
  reproduction, fixed/category summons, and animate-dead actors share one
  spawn helper. The player continues to use the character progression path.
- A saved instance's rolled `maxHp` remains authoritative. Loading validates
  that it lies within the content dice range, or equals the forced maximum,
  without rerolling or replacing it with the definition mean.
- New-character supply rolls retain their established order; fixed world
  monster HP is rolled afterward. This is a shared initialization/RNG change,
  so contract-v176 performs the project-rule exception of refreshing all
  fixture categories once. Later content-only changes return to category-only
  verification.

## Verification

The `monster_hit_points` unit module independently covers ordinary die count,
forced maximum without RNG, and save/load preservation of an instance roll.
W7 and W8 retain their own separate modules.

