# Contract v179: Monster Light

Status: active slice in the cumulative contract-v180 baseline. Protocol
`1.137`, demo pack `1.173.0`, save v1, state hash Schema v63. Old development
saves are not supported.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Actor light is a typed content field consumed by the authoritative Rust
  lighting calculation. It replaces the old presentation-only `light-source`
  tag.
- Original `HAS_LITE_1/2` contributes radius one/two. `SELF_LITE` marks the
  source intrinsic; multiple original light flags combine their radii.
- Sleeping suppresses carried `HAS_LITE` light but does not suppress intrinsic
  `SELF_LITE`. Light continues to use the normal line-of-effect and map bounds.
- Snapshots derive visible cell light from actor state. No save or protocol
  field is added because the light definition belongs to the matched content
  pack and sleep already belongs to authoritative actor state.

## Verification

One focused lighting test compares sleeping carried and intrinsic sources with
the same radius. The `system`, `dungeon`, and `monsters` fixture categories
verify without refresh.
