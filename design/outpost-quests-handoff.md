# Outpost quests branch handoff

## Git boundary

- Branch: `codex/outpost-quests`
- Integration base: `3fb94bcdc` (`generalize task rewards and reassign outpost quests`)
- Submitted sequence before 4.6: `b74d72c5c` (quest 50), `2cb5bc09f` (quest 62),
  `8b66c397a` (quest 31), `5187159ae` (quest 20).
- This branch exclusively edited `packs/rfb-demo-original/worlds/middle-earth.json` for the
  White Horse sequence. Treat that file, the pack version/hash, shared localization tails and
  active fixture hashes as high-conflict integration points.

## Stable IDs and ownership

- Tasks: `demo.task.trouble-at-home`, `demo.task.crows-nest`,
  `demo.task.old-man-willow`, `demo.task.vapor-quest`, `demo.task.old-castle`.
- Floors use the matching `demo.floor.*` suffixes. All five tasks are owned by
  `demo.town-facility.outpost-white-horse` and form the prerequisite chain 50 → 62 → 31 → 20 → 27.
- Old Castle artifacts: `demo.item.crisdurian`, `demo.item.slayer`, `demo.item.pain`.
- Surface service remains `(63,13)`; task entries are `(63,11)`, `(72,23)`, `(69,11)`,
  `(62,13)`, `(65,13)` in chain order.

## Compatibility and observable changes

- Protocol remains 1.175; save container remains v1; State Hash Schema remains v87.
- Pack is 1.261.0 with content hash
  `3cf3a2e3f1ce0314aa5782acd700d81133a06cbab0444f24b6cba4537c47cca5`.
- Each added surface entry changes common initial terrain. Contract v271 therefore contains a
  deliberate full refresh of all 21 active fixtures and no waivers.
- Quest 50 and quest 62 add one bounded(2) content-generation draw. Quest 27 adds seven weighted
  formation draws on entry and one weighted reward draw for Warrior. No protocol or shared
  initialization RNG order was added outside the affected floor/reward operations.

## Known bounded adaptations

- Quest 50 booze and quest 62/31/20 item-generation boundaries are documented in their individual
  contract files.
- Quest 27 fixes the two source `SCRAMBLE` actor groups to declaration order, narrows `MON(*)` to
  its quest roster, uses the general loot table for random/depth+7 objects, and omits four books
  co-located with Raal's tomes because inline entities have unique positions.
- Unsupported quest-27 source spells are explicit selection data, not silent fallbacks:
  `HELL_LANCE(66)` and `INVULN`.

## Integration verification

Run from the branch worktree:

```powershell
cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original
cargo test -p rfb-content -p rfb-core -p rfb-localization -p rfb-legacy-import
cargo run -p rfb-contract -- verify-all tests/fixtures/active/baseline-policy.json
cargo fmt --check
cargo clippy -p rfb-content -p rfb-core -p rfb-localization -p rfb-legacy-import --all-targets -- -D warnings
```

No unfinished 4.2–4.6 implementation remains on this branch. Merge conflicts should preserve the
stable IDs and recompile the final combined pack rather than choosing either branch's lock hash.
