# Contract v69: Configurable Dungeon Instance Lifecycle

Status: protocol 1.69 / contract-v69 active baseline; content pack 1.61.0; state hash Schema v28

## Scope

`DungeonDefinition.instanceLifecycle` now selects one of three deterministic policies:

- `reset-on-surface`: the default. Returning to the surface deletes the current instance immediately.
- `persistent`: returning to the surface retains one paused instance, which is resumed on the next entry.
- `turn-ttl { ttlTurns }`: retains one paused instance until `currentTurn - retainedAtTurn >= ttlTurns`; expiry is checked lazily on the next entry.

Echo Depths and Resonance Descent omit the field and keep the established reset behavior. The demo adds Archive Depths at surface position `(7,2)` with `turn-ttl` set to 3 turns.

## Authority And Saves

`DungeonStateSaveDto` adds optional `retainedInstanceId` and `retainedAtTurn`. A retained ID must belong to the dungeon, refer to at least one stored floor, use a non-reset lifecycle, and have a retention turn no later than the current turn. Missing fields in v68 and older saves migrate to no retained instance without generating content or advancing RNG.

Resuming removes the retained marker and restores the existing floor without consuming generation RNG. TTL expiry deletes every stored floor for that instance, removes property knowledge whose concrete item instance was deleted, and then allocates the next stable instance ordinal. Kind-level item knowledge remains global.

Only one automatically retained instance per dungeon is in scope. Parallel instance selection, cross-instance teleportation, eager background expiry, and runtime Vault reconnection are deferred.

## Determinism And Coverage

Retained state enters state hash Schema v28. Content pack 1.61.0 has hash `06c054a8c083e05b9d0396aa1076fbe2133a6a1ce5f6c32f101e5d1dabd14b70`; the v68 hash remains accepted for save migration.

The active baseline contains 140 exact fixtures with zero waivers. Fixtures 138-140 cover retained surface save round-trip, immediate `.instance.1` resume, and deterministic TTL replacement by `.instance.2`. Core tests additionally cover malformed retained saves and cleanup of per-instance item property knowledge.

## Next Boundary

The next Stage E candidate is runtime connectivity repair after destructive terrain changes, starting with deterministic reconnection of a damaged Vault or corridor. It must not introduce cross-instance collaboration as an incidental side effect.
