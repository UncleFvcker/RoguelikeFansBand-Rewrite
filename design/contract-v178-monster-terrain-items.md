# Contract v178: Monster Terrain and Item Destruction

Status: active slice in the cumulative contract-v180 baseline. Protocol
`1.137`, demo pack `1.172.0`, save v1, state hash Schema v63. Old development
saves are not supported.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Boundary

- Actor content expresses `KILL_WALL` and `KILL_ITEM` independently. Terrain
  content maps the original `FF_HURT_DISI` destruction result to a validated
  walkable, sight-open replacement.
- A wall destroyer treats only its own destroyable terrain as enterable. The
  capability does not make sealed Vault terrain globally connectable for
  generation validation.
- Destruction, entry, changed cells, item removal, and events use one monster
  movement transaction. Map boundaries and floor connections remain immutable
  runtime invariants of the rewrite.
- `KILL_ITEM` destroys ordinary ground stacks and gold. It preserves artifacts,
  matching slay/kill items, brands that can damage the monster, and Endurance
  ammunition, following the pinned original. No extra task-item immunity is
  invented.

## Verification

Focused movement tests independently cover wall transformation and entry,
ordinary item destruction, and gold destruction. The closed-Vault content test
uses ordinary destroyable wall terrain and still rejects a sealed interior.
The `dungeon`, `tasks`, and `monsters` fixture categories verify without
refresh.
