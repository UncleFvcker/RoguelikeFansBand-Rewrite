# Release Vertical Slice

Status: release definition only. This document does not authorize new content or a behavior change in the current refactor.

## 1. Slice definition

The smallest credible external playtest should use the existing `demo.dungeon.echo-depths` expedition:

- three logical depths using the already defined branch/mirror/shaft variants;
- the existing `demo.actor.resonant-warden` as the final guardian;
- the existing `demo.encounter-table.echo-depths` pool: Acid Seep, Echo Hound, Frost Wisp, Storm Spark, and Venom Spore;
- the existing Echo Depths theme, vault, trap, terrain, and generation definitions;
- the existing `demo.loot-table.echo-depth-1-room` and guardian loot, plus current starting/surface equipment;
- existing build presets rather than new races, classes, personalities, abilities, monsters, or items.

This is deliberately smaller than the current ten-depth Resonance Descent campaign. It exercises procedural generation, branches, combat, loot, equipment, item knowledge, saving/loading, a final-floor guardian, dungeon conquest, scoring, and deterministic replay without importing any content.

The compiled pack already contains 1 world, 4 races, 6 classes, 3 personalities, 6 build presets, 28 actors, 90 items, 68 abilities, 5 ability books, 48 terrain definitions, 6 encounter tables, 8 loot tables, 6 vaults, and 3 dungeons. Quantity is not the release blocker.

For the first playtest, expose a small preset choice using existing builds: Explorer for the baseline path, Vanguard for direct melee, and Scholar for the existing ability-book path. The other three existing builds remain valid content but are outside the slice acceptance matrix. No preset data is copied or changed.

## 2. Current gaps

The repository already has the authoritative dungeon run, save/load, native save slots, death state, campaign state, scoring, rendering, localization, and desktop packaging. It does not yet present them as a complete release flow:

- desktop startup immediately creates the default Explorer session instead of showing new/load choices;
- there is no user-facing build selection path even though six build definitions and `Game::new_with_build` exist;
- campaign victory currently requires conquering the ten-depth `resonance-descent`, not the selected three-depth `echo-depths` slice;
- death and victory prevent/limit further commands but do not provide a complete result -> restart/menu flow;
- controls are present in the game UI, but first-run guidance is not staged around the initial actions;
- external installer production exists for Windows NSIS in Tauri config, while release-signing/distribution and the Android CI artifact need a release checklist.

Closing these gaps may require a later, explicitly reviewed release-scope change. It must reuse existing content and must not be bundled into architecture movement.

## 3. Player flow

```text
Launch
-> choose New Game or an existing native save
-> for New Game, choose Explorer, Vanguard, or Scholar
-> see the surface, current objective, and contextual controls
-> enter Echo Depths
-> explore, fight, collect/use/equip existing items, and descend
-> traverse one of the existing depth-two branches or shaft routes
-> reach logical depth three and defeat the Resonant Warden
-> receive an explicit victory/result screen with score and turn count
-> start a new run, return to the main menu, or exit

At any safe point:
-> save to a native slot and exit
-> launch later and load the slot
-> resume at the same authoritative state

On death:
-> receive an explicit death/result screen
-> start a new run, return to the main menu, or exit
```

## 4. Required release work

Only the following items are required for this slice:

1. A startup/session shell with New Game and Load Game, using existing native-save storage.
2. A preset build selector wired through Tauri to existing `Game::new_with_build`; Explorer remains the default/fallback.
3. A slice configuration that treats conquest of existing Echo Depths and defeat of its existing Resonant Warden as the playtest victory condition, without changing combat values or content definitions unrelated to campaign selection.
4. Clear in-game objective and floor/depth feedback using existing snapshot/task/campaign information or the narrowest compatible projection needed.
5. Contextual first-run prompts for movement, pickup/use/equip, combat/targeting, stairs, save, and current resources. They must use existing commands and localized text.
6. Explicit death and victory result states showing outcome, build, score, turn count, and restart/menu/exit actions.
7. Save/load coverage for surface, mid-dungeon, and pre/post-guardian states; corrupted/recovery-backup behavior remains visible and actionable.
8. Visible feedback for every major command: rejection, damage/healing, item use, targeting, floor transition, save/load, death, and victory.
9. A release checklist that produces and smoke-tests the Windows installer; keep Android artifact generation in CI if Android is included in the advertised test platforms.
10. One end-to-end playthrough test that uses only normal UI commands and verifies new game -> dungeon -> guardian -> result -> restart/menu, plus a save/resume checkpoint.

The slice should continue to use deterministic seeds internally. A player-facing random seed can be generated by the host, but it must be recorded in the session/replay exactly once and must not alter core RNG semantics.

## 5. Explicit non-goals

The playtest does not include:

- a town, wilderness, home storage, reputation, shops, or a large economy;
- new races, classes, personalities, builds, monsters, bosses, items, potions, scrolls, artifacts, abilities, terrain, vaults, or dungeons;
- bulk import of additional legacy content;
- balance changes to existing actors, items, generation weights, damage, resources, or progression;
- the full ten-depth Resonance Descent campaign as a release requirement;
- the Archive Depths or task-rift content as a required route;
- a complete lore codex, large narrative campaign, achievements, or online services;
- React, Vue, Svelte, a new frontend state framework, or a visual redesign;
- changes to save container format, replay format, state-hash semantics, or protocol without a separately justified compatibility review.

Existing out-of-slice content need not be deleted. The release shell should keep the intended run focused without invalidating saves or changing the compiled catalog solely to reduce visible counts.

## 6. Acceptance criteria

The vertical slice is releasable when all of the following are true:

- a new player can start without reading repository documentation;
- New Game, build selection, Load Game, and exit are discoverable at launch;
- Explorer, Vanguard, and Scholar can each enter and finish the same existing three-depth objective;
- the player can understand current HP, resources, abilities, equipment, task/objective, floor, and available targeting mode;
- all major operations produce visible localized feedback;
- a player can complete a full run without debug commands or save-file editing;
- native save/exit/load restores the same authoritative state and replay continuation remains verifiable;
- death always reaches a result state and offers restart/menu/exit;
- defeating the existing Resonant Warden reaches an unambiguous victory result and offers restart/menu/exit;
- no known severity-1/flow-blocking defect remains in startup, dungeon progression, save/load, death, victory, or restart;
- deterministic fixed-seed runs preserve command/event order and final state hash across direct core, replay verification, and Tauri transport;
- protocol bindings, content schemas, save migrations, contract fixtures, Rust/frontend tests, Tauri E2E, and production builds pass;
- a Windows installer can be generated and installed on a clean external-test machine; any advertised Android build is generated by CI and smoke-tested;
- the release notes state that this is a focused Echo Depths playtest and do not imply the out-of-slice systems are complete.

## 7. Release evidence

Before handing the build to external testers, archive:

- the source commit and built-in content hash;
- protocol, save, replay, and state-hash schema versions;
- full CI results and the installer checksum;
- fixed-seed full-run replay(s), including one save/reload continuation;
- a short manual smoke-test record for startup, each supported build, targeting, save/load, death, victory, restart, and exit;
- known issues classified by whether they can block a complete run.

No acceptance criterion should be satisfied through a debug-only Tauri fixture, direct `Game` field mutation, or developer console command.
