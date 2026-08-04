# Contract v172: Optional Player Ability Policy and State Hash Boundary

Status: active baseline. Protocol `1.136`, demo pack `1.166.0`, save v1,
state hash Schema v62. Old development saves are not supported.

The built-in content hash is
`eb6dded2ca73a46535357886d44561040ca571387353feaeefa6873b0afeb7c0`.

## Ability compilation boundary

- Every ability still compiles an identity, target contract, level scaling, and
  executable Ability Program. Player casting parameters are an optional
  `player` policy on the compiled ability.
- A player binding supplies minimum level, resource and cost, failure rate,
  proficiency, and cooldown. It does not grant the ability to a player.
- Ability books, class innate ability lists, and future race or game-mode
  ownership remain the authority for player availability. Any such path must
  reference an ability that has a player policy.
- Abilities used only by current monsters are unbound by default, but they are
  not permanently classified as monster-only. A later player race or monster
  mode can add a binding without changing the Ability Program.
- Actor-owned `monsterCasting` remains independent of player policy and keeps
  its own selection, targeting, frequency, weight, and tactical parameters.
- Unknown, duplicate, or malformed player bindings are rejected. A missing
  binding is rejected only when a current player ownership path needs it;
  binding presence alone still grants no availability.

The demo keeps bindings for the 48 abilities currently reachable through
ability books or class innate lists and removes 21 placeholder bindings for
abilities currently used only by monsters. The legacy importer derives the
player set from books and class innate lists, emits Programs for all abilities,
and emits bindings only for that derived set.

## Content and state hashes

- Save loading requires an exact match of the current `contentId` and
  `contentHash`. The historical built-in hash compatibility table is removed.
- Replay playback continues to validate its independent `contentHash` before
  simulation.
- `contentId` remains in the authoritative state-hash payload, while
  `contentHash` is removed. The state hash now describes simulated state under
  an already validated content set instead of duplicating content identity.
- The state hash Schema is v62 because its explicit payload changed. This
  requires one global active-fixture refresh. Later content-only hash changes
  do not by themselves invalidate every state-hash assertion.

## Verification

Focused content tests cover bound and unbound abilities, binding validation,
and demo census. Core tests prove that changing only the reported content hash
does not change state hash, while loading a save with a different content hash
fails. Replay tests retain their independent mismatch check. Schema freshness,
source-lock verification, importer extraction, and the active contract baseline
complete the non-E2E acceptance set.

The one-time v62 fixture refresh changes only state-hash assertions except for
the existing random monster and pet summoning scenarios. Those two scenarios
now observe the Warrens species added in v171; their selected kinds, HP, and
immediate scheduled actions are reviewed as the expected consequence of that
earlier ecology expansion.
