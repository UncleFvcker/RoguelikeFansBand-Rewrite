# Contract v150: Warrens first player journey

Status: Phase 17 Gate 3 completed baseline.

Contract v150 changes the production player journey from the original Echo prototype to a bounded Warrens compatibility slice. Protocol remains `1.123`, the save container remains v1, and state hash Schema remains `55`. The demo pack advances to `1.141.0`; the active baseline contains 455 exact fixtures with zero waivers.

## Fixed source audit

The behavior reference is RFB v1.3.0.7 at commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9`. The audit reads Git objects rather than the legacy working tree.

- `lib/edit/d_info.txt`, dungeon record 30: Warrens spans logical depths 1-9, uses the smallest dungeon size class, enables cave/cavern generation, filters toward early kobold/beast glyph families, and has a final guardian.
- `lib/edit/r_info.txt`, guardian record 135: the final guardian is a depth-appropriate kobold lord with an escort-oriented role.
- `lib/edit/k_info.txt`, tval/sval 75/29: the fixed guardian reward role is a speed consumable.
- `lib/edit/q_info.txt`, quest record 14, plus the help behavior: Pest Control is a separate depth-five task to kill eight Wargs and depends on an earlier town quest. It is not part of the unconditional Warrens dungeon progression.

## Promoted scope and clean-room differences

The committed pack does not copy legacy maps, prose, algorithms, exact monster numeric records, or assets. It independently expresses the selected flow through the current engine:

- one compact nine-depth linear dungeon with ordinary paired stairs;
- an early roster of a giant mouse, small kobold, and Warg archetype;
- a depth-nine Kobold Lord guardian and a speed-themed guardian reward;
- a plain short sword and light-healing supply on the surface, plus bounded healing/escape floor loot;
- a one-dungeon campaign whose conquest produces the existing victorious state.

Known differences are explicit. Current floors are the engine's compact 20x20 format rather than RFB's 66x22 smallest layout. The existing procedural room generator approximates the cave/cavern role. The complete RFB depth-filtered monster roster, guardian escort generation, wilderness/town entrance, shops, food/light economy, and Pest Control prerequisite chain are deferred. The published guardian uses the generic role name `Kobold Lord`; the legacy proper name is not promoted without a separate provenance decision.

## Runtime ownership

`demo.world.warrens-journey` is a second built-in world, not a debug mutation of `demo.world.original-v1`. Production Tauri New Game constructs `Game::new_warrens_journey_with_build`; build and seed selection remain unchanged. World ID, floor state, dungeon conquest, campaign status, score, RNG, and stored floors already live in saves, replays, and state hashes, so no campaign-profile field or protocol bump is introduced.

The old Original Lab and Echo content remain available only to historical/system regression tests. The debug webdriver suite still uses its established Original Lab state assumptions until Phase 17 Gate 6 replaces that script with the normal no-mutation Warrens acceptance run; it is not cited as Gate 3 full-journey proof.

## Evidence

- core construction selects the Warrens world without removing the Original Lab catalog;
- 16 fixed seeds generate all nine descents and all nine normal return transitions;
- guardian death emits dungeon conquest before campaign victory;
- victorious state survives save/load at depth nine, returns through depths 8-1, retires only on the surface, and round-trips the frozen result hash;
- native session save and replay verification retain `demo.world.warrens-journey`;
- fixture 455 fixes the conquered Warrens surface retirement and post-retirement rejection contract;
- the frontend objective model presents preparation, entry, depths 1-8, the depth-nine Kobold Lord, return, and retirement from authoritative state.

Gate 3 does not claim final balance, the optional Warg quest, or a complete RFB content port. Those remain bounded Gate 5 or later work.
