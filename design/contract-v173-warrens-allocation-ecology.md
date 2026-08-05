# Contract v173: Warrens Allocation and Runtime Ecology

Status: active baseline. Protocol `1.137`, demo pack `1.167.0`, save v1,
state hash Schema v63. Old development saves are not supported.

The built-in content hash is
`91743edbfd3459c7bf41216c78060baae59278e87c8977347983e3f3fc3cf48d`.

The fixed legacy source is commit
`191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`.

## Allocation boundary

- Warrens encounter tables select from the global actor allocation pool instead
  of a closed local roster. Legacy index, rarity, maximum depth, forced depth,
  and wild-only restrictions are explicit actor content.
- Eligible actors use the legacy `100 / rarity` base weight. Glyphs in
  `kKyYrRfFcCbB` retain their full weight; other eligible glyphs use the
  dungeon's `MONSTER_DIV_16` factor of `16 / 64` with deterministic remainder
  handling.
- Each allocation performs two independent `one_in_(40)` checks. A success adds
  `min(5, currentLevel / 10) + 2` before the next check, matching the legacy
  order. Warrens depth 9 can therefore reach level 14 through `9 -> 11 -> 14`,
  making Warg possible but extremely rare outside Pest Control.
- Allocation continues to apply legacy maximum-depth, force-depth, wild-only,
  low-level decay, and Unique availability filters.

## Groups and Unique lifecycle

- A selected leader is placed first. `FRIENDS` and `ESCORT` are expanded only
  afterward; insufficient space reduces companions without cancelling or
  rerolling the leader.
- `FRIENDS(XdY)` rolls actual dice and counts the leader in the result. Generic
  expansion is capped at 32 actors. Warg uses `3d3`.
- Escort selection uses the leader glyph, lower level, different kind,
  non-Unique status, faction compatibility, and the legacy bounded attempt
  count. Mughash uses this path without a hard-coded escort kind.
- `defeatedUniqueActorKindIds` records dead ordinary non-guardian Unique kinds.
  Current and stored floors are also scanned for living instances. Allocation,
  category/fixed summons, death, save loading, and validation share this
  authority. Guardians remain governed by dungeon state.

## Runtime ecology

- Ambient allocation runs before the monster action queue is built. Warrens
  uses the legacy base `one_in_(160)` chance with depth adjustment and requires
  a position more than 25 cells from the player. Ambient leaders can expand
  normal groups.
- After a sleeping monster is checked, `MULTIPLY` attempts reproduction before
  casting. A successful birth consumes that actor action. The level-wide
  breeder cap is 100; Giant White Mouse uses `MULTIPLY + RAND_50`.
- After casting has failed to produce an action, RAND movement is resolved
  before ordinary tracking. `RAND_50` and `RAND_25` use their legacy
  probabilities. Actor-defined RAND movement also applies to player-faction
  summons of that kind.
- Generated actors use the existing carried-loot path. Random per-instance HP,
  door interaction, movement domains, special melee effects, terrain/item
  destruction, actor light, and complete drop flags remain W7-W13 work.

## Verification

Focused core tests cover depth-9 double out-of-depth allocation,
`MONSTER_DIV_16`, Warg `3d3`, Mughash escort, Giant White Mouse reproduction,
Warg random movement, ambient allocation, and Unique death/save round-trip.
Existing Warrens tests assert four encounter leaders without assuming a fixed
post-group entity count. Content tests validate allocation metadata, encounter
configuration, actor references, and the supported legacy ecology.

Because the authoritative state-hash payload gains the defeated Unique set,
this contract performs the approved one-time full fixture refresh for Schema
v63. Later content-only changes continue to update the content lock, pack
version, and only behavior-affected fixture categories.
