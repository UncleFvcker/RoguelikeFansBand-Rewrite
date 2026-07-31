# Architecture Convergence Plan

Status: phases 0-12 complete. The implemented boundary covers protocol projection, validation, persistence orchestration, test organization, pure world-generation calculations, inventory, terrain, floor-lifecycle and recall plan/commit operations, frontend composition, character progression, task/campaign transition plans, and aggregate-local damage, death, and attack-family resolution.

This plan was initially based on repository state at commit `82d1eea5` on 2026-07-30 and was refreshed from the Phase 10 base commit `e9899b3c` on 2026-07-31. It is a behavior-preserving convergence plan, not an engine migration or a rewrite of `rfb-core`.

## 1. Current architecture

### Workspace and dependency boundaries

The Cargo workspace contains ten members:

- `rfb-protocol`: shared commands, events, snapshot/update DTOs, save DTOs, generated TypeScript bindings, and protocol/schema versions.
- `rfb-content`: content schemas, compiler, validation, and the built-in content catalog.
- `rfb-core`: the authoritative deterministic simulation.
- `rfb-save`: the checksummed save container around protocol save DTOs.
- `rfb-replay`: command recording, checkpoints, state-hash verification, and replay containers.
- `rfb-contract`: committed behavior fixtures and baseline verification.
- `rfb-localization`: Fluent resources and validation.
- `rfb-legacy-probe` and `rfb-legacy-import`: legacy inspection and migration tooling.
- `rfb-tauri`: the desktop/mobile host and native storage boundary.

The frontend uses TypeScript, PixiJS, Vite, and Tauri. It has no independent rules engine. `TauriNativeTransport` invokes Tauri commands, `AppState` owns one `ReplayRecorder`, and the recorder owns the authoritative `Game`.

The intended dependency direction is:

```text
rfb-protocol / rfb-content
             |
             v
rfb-core domain modules
             |
             v
Game aggregate root
             |
             v
snapshot / events / rfb-save / rfb-replay
             |
             v
Tauri transport -> TypeScript controller/UI -> RenderWorld -> PixiJS
```

Domain code must not depend on Tauri, TypeScript, DOM APIs, or PixiJS. `rfb-save` remains a container boundary and must not acquire game-rule migration logic.

### The `Game` aggregate today

At the initial audited commit, `crates/rfb-core/src/game/mod.rs` contained 26,254 lines. After Phase 10 it contains 21,856 lines, with progression calculations and command planning in a 516-line `progression.rs`, task/campaign planning in a 492-line `tasks.rs`, and terrain-interaction planning in a 386-line `terrain.rs`. `Game` is the correct authoritative aggregate root, but its implementation is still the home of most domain behavior.

`Game` currently owns:

- the immutable content catalog and world/current-floor identity;
- current terrain plus stored floor states and dungeon instance identity;
- player actor, build, body slots, progression, resources, learned abilities, and ability proficiency;
- entities and item instances across ground, inventory, equipment, and monster ownership;
- item-kind and affix-property knowledge;
- task, dungeon, campaign, summon-command, and recall state;
- exploration/revealed terrain, floor connections, and floor regions;
- deterministic RNG and item-instance allocation state;
- revision, player turn, world tick, and last command sequence;
- transient visual-delta cache and test/debug decision overrides.

Its implementation currently performs all of the following:

1. construction from built-in or supplied content and character builds;
2. save restoration, compatibility defaults, and content migration;
3. save projection and authoritative state hashing;
4. full snapshots, incremental updates, UI DTO projection, visibility, lighting, and terrain queries;
5. command validation, command dispatch, transaction ordering, and event projection;
6. player/monster/summon scheduling and status ticks;
7. melee, projectile, damage, death, and loot resolution;
8. abilities, item use, equipment, knowledge, recharge, and resources;
9. task, campaign, dungeon lifecycle, floor transition, recall, and world generation;
10. loaded-state and runtime invariant validation.

The aggregate is therefore both the transaction owner and the implementation module for unrelated domains. The problem is responsibility mixing and change blast radius, not the continued existence of `Game`.

### Function classification

The audit classifies current functions by effect:

- Pure calculations: distance and path calculations, area falloff, status duration scaling, modifier merging, target-spec projection, vault transforms/candidate construction, region allocation, terrain connectivity checks, and content-driven stat/profile calculations whose inputs are explicit.
- Read-only aggregate queries: `snapshot`, DTO projection methods, visibility/light projection, terrain interaction queries, task/campaign projection, effective stats, targeting plans, content/location lookup, and `state_hash`.
- Authoritative mutations: combat/ability/item resolution, scheduling, resource/cooldown/status changes, inventory/equipment moves, death/loot, task/campaign transitions, floor activation/generation, recall, and ID allocation.
- Command boundary: `dispatch` validates revision/sequence and terminal state, normalizes `GameCommand` into `GameAction`, owns the transaction, applies the action, advances scheduling, commits revision/sequence, and constructs `GameUpdate` in stable order.
- Persistence and migration: `from_save_with_content` restores all authoritative components and compatibility defaults; `to_save` projects the v1 payload; helpers in `rfb-core/src/save.rs` convert component DTOs; `rfb-save` encodes/checksums the outer container.
- Validation: `validate_runtime_invariants`, `validate_loaded_state`, actor/summon checks, floor connection/region/revealed-terrain checks, pack checks, item stack/affix checks, and content-reference checks.
- World generation: `generate_procedural_floor` and its room, cavern, hydrology, maze, destroyed-area, streamer, vault, region, encounter, connection, terrain-feature, and loot helpers.
- Combat and abilities: player/monster/summon melee, projectile tracing, damage, death, monster casting plans, player ability planning/resolution, ordered effects, targeting, statuses, and resources.
- Items/equipment/knowledge: pickup/drop/equip, DTO knowledge masking, appraisal/identification, enchantment/curse, consumable/device effects, recharge, instance generation, and affixes.

### Tests and compatibility assets

The original `crates/rfb-core/src/game/tests.rs` contained 17,067 lines and 268 tests. Phase 4 replaced it with a small `tests/mod.rs`, one centralized `tests/support.rs`, and 14 domain modules covering abilities, combat, deterministic replay, generation, inventory, items, monster AI, movement, persistence, progression, snapshots, summons, tasks, and world behavior. The test bodies and assertions are unchanged.

The deeper coupling remains explicit: the tests still directly access `Game` internals, including entities, player state, items, RNG counters, and private resolution/generation methods. This white-box access is valuable and remains available because the domain test modules are descendants of `game`; Phase 4 did not add public production setters or hide meaningful setup behind new defaults. Existing command, floor, item, ability, skill-check, summon, snapshot, and invariant helpers are now centralized with test-module-only `pub(super)` visibility.

The external compatibility layers are stronger and must remain primary refactor guards:

- 454 committed contract scenario fixtures, loaded in sorted path order;
- 33 replay tests, including checkpoint tamper detection, save/reload continuation, and 10,000-turn drift detection;
- state hashing over a stable borrowed save-shaped representation, excluding exploration memory by design;
- protocol binding and content-schema `--check` generators;
- save container corruption/round-trip tests and core migration tests;
- frontend DTO, targeting, terrain interaction, localization, renderer, and native-save tests;
- Tauri desktop WebDriver E2E.

Future test work should reduce direct state mutation only when a repeated setup pattern has stable semantics. A general `TestGameBuilder` was not introduced in Phase 4 because the current tests intentionally exercise many distinct historical and malformed states; assigning shared defaults would risk masking those invariants. New support abstractions should be added only for proven repetition and must preserve direct access for compatibility and corruption tests.

### Frontend composition

After Phase 7, `web/src/main.ts` contains 429 lines and acts as the composition root for startup, diagnostics, localization, transport, renderer, state, controllers, and panels. `AppState` owns session truth, `GameSession` owns the single command transaction path, and `InputController` owns input and targeting listener lifetime. Message, save, settings, status, and inventory presentation are separated into focused modules.

The lower boundaries remain unchanged: `RenderWorld` consumes authoritative cells, `MapRenderer` composes it with a `RendererBackend`, `PixiRendererBackend` renders only, targeting and terrain interaction logic are pure modules, and `TauriNativeTransport` only performs IPC. No gameplay panel or input path invokes the transport directly. The frontend suite contains 54 focused tests in addition to typecheck, production build, native release build, and desktop E2E coverage.

### Risks and non-problems

Risks caused by mixed responsibilities:

- a save migration edit sits beside runtime construction and command logic;
- snapshot projection shares private helpers with combat/item code, increasing accidental protocol drift;
- world generation and runtime transitions share `Game`, RNG, ID allocation, entities, and items;
- ability/item/monster effects converge on damage, death, status, knowledge, and event ordering;
- tests can preserve a private representation accidentally rather than public behavior;
- frontend panel logic can mutate shared interaction state or issue commands outside one obvious controller.

Likely dependency-cycle pressure points:

- snapshot projection -> aggregate helpers -> item/ability calculations -> protocol DTOs;
- validation -> aggregate queries/content helpers while construction needs validation;
- combat/ability/item -> death -> loot/task/campaign -> events;
- floor generation -> RNG/IDs/items/entities -> floor persistence/validation;
- frontend inventory/ability panels -> targeting -> dispatcher -> session state -> panel rerender.

Surface issues not worth immediate action:

- `Game` having many fields is expected for an aggregate root; replacing it with managers would obscure the transaction boundary.
- `rfb-protocol` is necessarily broad because it is a generated cross-language contract.
- large exhaustive enums/matches for ordered effects are safer than premature traits or plugins.
- the 454 fixture files are intentionally separate and do not represent harmful source aggregation.
- large content/design directories are data/history, not runtime architecture debt.

## 2. Target architecture

`Game` remains the sole authoritative aggregate root. Its long-term responsibilities are:

1. own authoritative state;
2. receive and preflight external commands;
3. create narrowly scoped domain inputs;
4. own RNG, ID allocation, and transaction boundaries;
5. commit domain outcomes in deterministic order;
6. emit public events, updates, snapshots, saves, and hashes.

Domain modules should prefer explicit immutable inputs and explicit outcomes. A planner may return an ordered outcome/change set, but `Game` commits it. This pattern is introduced only where it reduces an existing dependency; it is not a mandate to rewrite every `&mut self` method.

Visibility remains private where possible. Child-module operations called by the parent aggregate use `pub(super)`. `pub(crate)` is reserved for a proven crate-wide consumer, and `pub` is only for the existing external API.

The first implemented structure is deliberately small:

```text
crates/rfb-core/src/game/
  mod.rs          # state, construction, dispatch, domain mutations
  damage.rs       # shared HP application and status-damage plans
  death.rs        # ordinary actor-death plan and authoritative commit
  environment_combat.rs # trap damage adapter
  inventory.rs    # inventory/equipment plans, commits, item knowledge
  item_combat.rs  # item actor damage and backlash resolution
  monster_combat.rs # monster damage, melee, resistance observation, retaliation
  persistence.rs  # save projection, state hash, restoration, migrations
  player_combat.rs # player melee/projectile/throw and summon melee resolution
  progression.rs  # build/progression calculations and explicit growth plans
  snapshot.rs     # public snapshot and read-only protocol projections
  tasks.rs        # task reduction and task/campaign transition plans
  terrain.rs      # terrain-interaction plans and authoritative adapters
  floor.rs        # floor target/lifecycle/recall plans and adapters
  validation.rs   # command preflight and pure state-invariant predicates
  world/
    mod.rs        # private world-generation module boundary
    geometry.rs   # pathing, transforms, terrain geometry/connectivity
    generation.rs # ordered candidates, local terrain edits, region budgets
  tests/
    mod.rs        # domain test organization
    support.rs    # centralized test-only helpers
```

No duplicate implementation or forwarding “manager” is retained.

## 3. Phased implementation

### Phase 0: audit and baseline (completed)

- Scope: workspace/crate graph, `Game`, tests, save/replay/hash/fixtures, frontend/render/transport, CI, README/design docs, Cargo/npm/Tauri scripts, and content inventory.
- Not in scope: source movement or behavior changes.
- Files: this plan and `design/release-vertical-slice.md`.
- Risk: overlooking a non-CI platform or compatibility check.
- Acceptance: commands below are recorded; responsibilities and dependency risks are explicit.
- Rollback: delete documentation if it contradicts the repository; do not change code to fit the document.
- Tests: all baseline commands in section 4.

### Phase 1: read-only snapshot and validation boundaries (completed)

- Scope: move full-snapshot construction and tightly related map/UI query projection into `snapshot.rs`; move command preflight and independent loaded-state predicates for monster packs, rolled affixes, floor regions/connections, and revealed terrain into `validation.rs`; preserve every loop, sort, field, error string, and call order.
- Not in scope: `dispatch`, mutation logic, `state_hash`, `to_save`/`from_save`, migration-coupled `validate_loaded_state` orchestration, player/entity/item DTO projection, world generation, combat, abilities, items, tests, or TypeScript behavior.
- Expected files: `game/mod.rs`, `game/snapshot.rs`, `game/validation.rs`.
- Risk: widening visibility, changing projection order, accidentally omitting an invariant, or creating a parent/child dependency tangle.
- Acceptance: no new `pub` API; only required `pub(super)` entry points; `cargo fmt`, focused core tests, replay tests, fixtures, workspace check/clippy/test, generators, frontend checks, and Tauri check/build/E2E remain green; generated files and fixtures are unchanged.
- Rollback: any state hash, replay, fixture, snapshot, save, event-order, or error compatibility difference; required visibility broader than `pub(super)`; or a cycle requiring unrelated edits.
- Tests: `cargo test -p rfb-core`, `cargo test -p rfb-replay`, `cargo test -p rfb-contract --test contract_fixtures`, then the full section 4 matrix.

### Phase 2: remaining read-only DTO projection (completed)

- Scope: move player, entity, ground-item, inventory, equipment, ability/resource, progression/build, campaign, and knowledge-masked item DTO construction into `snapshot.rs` in small cohesive batches; keep knowledge rules and other calculations with current domain owners until they have explicit inputs.
- Not in scope: mutation logic, save DTOs, state hashing, command dispatch, or changing protocol fields/order.
- Files: `game/snapshot.rs`, `game/mod.rs`, and an explicit protocol DTO import in `game/tests.rs`; no projection submodule was needed.
- Risk: knowledge leakage, unstable entity/item ordering, changed ability availability, or divergent full-snapshot and incremental-update projections.
- Acceptance: exact snapshot/update DTO fields and ordering, knowledge masking, generated TypeScript bindings, fixtures, replay/hash, frontend render tests, and Tauri E2E remain unchanged.
- Rollback: a projection needs mutable `Game`, moving it requires domain mutation code, or public visibility expands.
- Tests: snapshot/knowledge/ability/inventory-focused core tests, protocol check, fixtures, replay/hash, frontend tests/build, then the workspace matrix.

Implementation was deliberately split into three reviewable batches. The first moved player and entity projection, the second moved ground items, inventory, and equipment, and the third moved ability learning, resources, abilities, campaign state, progression, and build projection. ID and slot sorting, `BTreeSet` ability ordering, item-knowledge masking, field construction, and helper-call order remain unchanged.

The domain calculations consumed by these views remain in `mod.rs`: derived stats, equipment and knowledge rules, ability scaling/cost/failure/cooldown calculations, skill progression, character-definition lookup, campaign counts, and campaign scoring. Mutation, persistence, state hashing, dispatch, combat, abilities, items, and generation were not moved. Cross-module projection methods use `pub(super)` only when `mod.rs` also consumes them; all other new methods remain private to `snapshot.rs`.

### Phase 3: persistence orchestration boundary (completed)

- Scope: extract save payload projection, state-hash representation, decode restoration orchestration, and migration entry points only after their shared inputs are mapped; keep component conversion in `rfb-core/src/save.rs` and container encoding in `rfb-save`.
- Not in scope: save schema/version changes, removal of compatibility defaults, domain behavior, or new crates.
- Files: `game/persistence.rs`, `game/mod.rs`, and explicit save-helper/DTO imports in `game/tests.rs`; existing persistence assertions were unchanged.
- Risk: serialization field/order drift, default drift, migration branch loss, hash drift, or replay checkpoint drift.
- Acceptance: byte/JSON round trips, every migration test, representative historical fixtures, exact state hashes, and replay continuation all pass with unchanged versions.
- Rollback: any generated payload or hash difference without a documented pre-existing normalization rule.
- Tests: core save/migration filters, all `rfb-save`, all `rfb-replay`, fixtures, generators, workspace matrix.

Implementation was split into three compatibility-gated batches. The first moved `to_save`, the borrowed state-hash representation, `state_hash`, and their player/knowledge/task/dungeon/campaign projections. The second moved `from_save` and `from_save_with_content` with their existing content-hash migration sequence intact. The third moved the persistence-only dungeon, campaign, task, character-progress, and item-knowledge restoration parsers plus `TaskRestoreContext`.

At Phase 3 completion, runtime `TaskState`, `DungeonState`, `CampaignState`, item-knowledge state, build/body-slot rules, ability-state restoration, task objective calculations, loaded-state validation, and content lookup remained owned by `mod.rs`; Phase 9 later moved the task/campaign state and calculation boundary. Component DTO conversion remains in `rfb-core/src/save.rs`, and the checksummed container remains in `rfb-save`. The four existing public persistence methods retain their signatures; all new helpers and representations in `persistence.rs` are private.

### Phase 4: centralized test support and domain test modules (completed)

- Scope: create `game/tests/support.rs`; centralize builders and assertions; move unchanged tests into movement, combat, inventory, abilities, generation, tasks, persistence, deterministic replay/hash, and snapshot modules in reviewable batches.
- Not in scope: rewriting test semantics, hiding needed controls, or adding public production setters.
- Expected files: `game/tests/mod.rs`, `support.rs`, domain test modules; remove `tests.rs` only after the final batch.
- Risk: lost coverage, changed fixture setup, and helper defaults masking meaningful state.
- Acceptance: identical test count/intent, no ignored tests, stable RNG assertions, and fewer scattered direct field mutations.
- Rollback: a helper cannot express an existing invariant or test count/coverage drops.
- Tests: moved module filters after each batch, then all core/replay/contract/workspace tests.

Implementation used three compatibility-gated batches. The first changed `tests.rs` into the directory-backed `tests/mod.rs` without changing content. The second moved the eight file-header helpers into `tests/support.rs`. The third moved all remaining top-level helpers and all 268 unchanged test functions into 14 domain modules. A mechanical block comparison verified the pre/post test-function set with SHA-256 `8c5deab5547c37ae614c7550ebb7edc9c61748b27da8cb9b7a6263f36d693616` before compilation.

The support module contains 23 existing helper functions with `pub(super)` visibility, including command dispatch, floor traversal, controlled item/summon setup, ability/result lookup, skill-check assertions, snapshot lookup, and invariant assertions. No helper adds a new default or production API. After the Phase 8-10 boundary tests, core contains 274 game tests and 297 tests overall, with no ignored tests; replay and contract guards remain unchanged.

### Phase 5: pure world-generation calculations (completed)

- Scope: extract geometry, transforms, candidate enumeration, connectivity, and budget allocation using explicit parameters and ordered collections; leave RNG draws, IDs, state lookup, and commits in `Game`.
- Not in scope: changing algorithms, maps, weights, random draw counts, floor lifecycle, or content.
- Expected files: `game/world/mod.rs`, `world/geometry.rs`, `world/generation.rs`, existing generation tests.
- Risk: iteration order and RNG call-order drift, especially around `BTreeSet`/`BTreeMap`, candidate sorting, and fallback paths.
- Acceptance: exact generation fixtures/hashes for representative seeds and all previous-version “do not backfill” tests pass.
- Rollback: any terrain/entity/item/connection ordering or RNG counter difference.
- Tests: generation-focused core tests, fixtures, replay/hash suite, full workspace matrix.

Implementation used two compatibility-gated extraction batches. The first moved maze anchors, distances, and paths; vault dimensions, coordinate transforms, entrances, and connector paths; and terrain indexing, walkability, and connectivity into `world/geometry.rs`. The second moved ordered room, formation, maze, vault, terrain-feature, and wall candidate construction; vault painting; room/corridor carving; primary connection selection; room-to-region assignment; and actor/loot budget allocation into `world/generation.rs`.

All extracted functions consume explicit inputs and preserve their original collection types, iteration order, sorting, assertions, and local terrain mutation order. RNG draws, weighted and spatial selection, generated floor-connection placement, item/entity ID allocation, aggregate state lookup, floor lifecycle, and authoritative state commits remain in `Game`. The world modules add no crate or external public API: their callable surface is restricted to `pub(in crate::game)`, and the child modules themselves are `pub(super)`.

### Phase 6: one command domain at a time (completed)

- Scope: choose one cohesive domain after phase 5 evidence, likely inventory/equipment/knowledge before combat; introduce explicit plans/outcomes only where current dependencies support them.
- Not in scope: simultaneous combat, ability, item, task, and dispatcher redesign; plugin traits; new content.
- Expected files: one domain directory plus `game/mod.rs` and its tests.
- Risk: event ordering, atomic rejection boundaries, RNG calls, ID allocation, and cross-domain death/task effects.
- Acceptance: command/event/save/replay/hash compatibility is exact and diff remains reviewable.
- Rollback: outcome needs the entire mutable `Game`, public surface expands broadly, or more than one domain must change together.
- Tests: domain command tests, zero-RNG rejection tests, replay/fixtures/hash, full workspace matrix.

The first command-domain extraction chose the low-coupling inventory, equipment, appraisal, and item-knowledge boundary. `inventory.rs` now plans and commits `PickUp`, `Drop`, `DropQuantity`, `Equip`, `Unequip`, and `Appraise` operations. Pure planners receive explicit item, content, body-slot, position, capacity, and knowledge inputs and decide ordered stack transfers, stable batch membership, split requirements, equipment-slot selection, replacement, and unequip targets before mutation.

`Game` remains the transaction owner. It still allocates split-stack IDs, applies planned item location and quantity changes, clamps HP, refreshes resource maxima, emits domain events, advances scheduling, and commits the command revision. Item-use, throwing, recharging, enchantment, curse application/removal, combat, status, death, loot, RNG draws, and all cross-domain effect chains remain in `mod.rs`. Existing item-knowledge state and local appraisal/awareness operations moved with the domain, while persistence and snapshot consumers retain their previous effective visibility. No crate or external public API was added.

### Phase 7: frontend composition (completed)

- Scope: extract one low-risk unit at a time, beginning with message formatting/panel state, then save panel, DOM registry, app state, targeting/input, dispatcher, and remaining panels.
- Not in scope: framework migration, visual redesign, DOM contract changes, renderer changes, or protocol changes.
- Expected files: `message-panel.ts`, `save-panel.ts`, `app-state.ts`, `game-session.ts`, `input-controller.ts`, and later panel modules.
- Risk: shared global state, listener lifetime, localization refresh, duplicate command dispatch, and targeting mode races.
- Acceptance: exact visible behavior, DOM IDs, keyboard controls, command envelopes, native save behavior, render diagnostics, frontend tests/build, and Tauri E2E.
- Rollback: extraction requires protocol/DOM changes or introduces multiple sources of session truth.
- Tests: `npm test`, `npm run typecheck`, `npm run build:ui`, `npm run e2e`, plus protocol check.

Implementation proceeded in dependency order and reduced `web/src/main.ts` from the 3,707-line
baseline to 429 lines. Pure localized event and item presentation now lives in `event-format.ts`;
bounded message history and DOM rendering live in `message-panel.ts`; native slot persistence and
its controls live in `save-panel.ts`; and `app-dom.ts` resolves the existing DOM contract once into
a frozen, typed registry. All existing element IDs, localization keys, diagnostic `data-*`
attributes, and visible output remain unchanged.

`AppState` is the sole owner of busy and terminal flags, current snapshot/update, map dimensions,
inventory/equipment selections, and targeting/terrain-interaction state. `InputController` owns the
keyboard, resize, target-toggle, targeting, and terrain-mode listener lifetime. `GameSession` owns
the single command transaction path, including terminal gating, busy-state recovery, core dispatch,
and ordered update application. No gameplay panel or input path invokes `TauriNativeTransport`
directly.

The remaining UI composition is split by responsibility: `settings-panel.ts` owns persisted input,
tileset, camera, zoom, and locale controls; `status-panel.ts` owns status, progression, resources,
abilities, summons, tasks, and campaign views; and `inventory-panel.ts` owns inventory/equipment
rendering, selection, item dialogs, recharge selection, and inventory commands. Each controller has
idempotent listener installation and explicit disposal. The extraction added focused tests for DOM
contract failure, state defaults, command transaction ordering and recovery, keyboard presets,
inventory quantities/recharge pairing, settings validation, status formatting, message history,
save error categories, and localized presentation, bringing the frontend suite to 54 tests.

### Phase 8: character progression calculations and plans (completed)

- Scope: extract character-build lookup, initial attributes, ordered skill aggregation, percentage composition, effective attributes, HP/resource maxima, experience scaling, and proportional HP/resource refresh calculations; introduce explicit plans for experience gain and `IncreaseAttribute` before authoritative mutation.
- Not in scope: item-driven attribute drain/restore/permanent growth, campaign transition/scoring, death reward ownership, the combat derived-stat pipeline, ability effects, RNG, ID allocation, protocol/content changes, or new public APIs.
- Files: `game/progression.rs`, `game/mod.rs`, progression tests, and this plan.
- Risk: changing saturation or rounding points, skill-map ordering, temporary-race overlays, HP/resource proportional scaling, event order, or zero-RNG rejection behavior.
- Acceptance: exact build/progression snapshots, level and skill growth, successful and unavailable attribute commands, migration/save/hash/replay/fixture compatibility, generators, workspace checks, frontend build/tests, and Tauri release/E2E remain unchanged.
- Rollback: any RNG draw, event order, state hash, save/replay/fixture, resource ratio, or effective-stat difference; a planner needs mutable `Game`; or visibility must expand beyond `pub(super)`.
- Tests: progression-focused core tests, all core/replay/contract tests, then the full section 4 matrix.

Implementation used two compatibility-gated parts. Pure functions in `progression.rs` now resolve build definitions and initial attributes; aggregate skills in existing `BTreeMap` order; compose build percentages and attribute modifiers; calculate character HP and profile resource maxima; initialize resource pools; scale experience; and preserve the original saturating proportional HP/resource calculations. Read-only `Game` adapters retain temporary granted-race selection and explicit access to equipment and status inputs.

`IncreaseAttribute` and experience gain now plan against cloned `CharacterProgress` values before mutation. `Game` commits the planned authoritative fields, refreshes skills and resources, rescales current HP/resources, and emits the existing events in their original order. Item effects continue to own their RNG-backed attribute behavior and only reuse the existing `pub(super)` resource snapshot/refresh adapters. Death, campaign transitions, RNG, IDs, scheduling, revision updates, and public event projection remain outside the module. The extraction reduced `game/mod.rs` from the Phase 7 post-extraction 22,817 lines to 22,425 lines and added two focused zero-RNG command tests, bringing core to 293 tests.

### Phase 9: task and campaign transitions (completed)

- Scope: extract task objective lookup and success predicates, initial task states, command-event reduction, task departure/activation/abandonment outcomes, campaign victory/count/score calculations, and explicit victory/retirement plans.
- Not in scope: death resolution, loot generation, task reward item construction, terrain replacement, stored-floor cleanup, dungeon instance lifecycle, RNG, ID allocation, protocol/content changes, or event projection changes.
- Files: `game/tasks.rs`, `game/mod.rs`, task tests, and this plan.
- Risk: changing ordered event interpretation, staged-task rollover, task retake counts, campaign bonus/penalty timing, victory event order, or save/migration validation.
- Acceptance: staged and retakeable task fixtures, guardian/campaign/retirement fixtures, zero-RNG task reduction, ordered victory events, save/hash/replay compatibility, generators, workspace checks, frontend build/tests, and Tauri release/E2E remain unchanged.
- Rollback: any task state, score, RNG draw, event order, state hash, save/replay/fixture, floor reward, or terrain difference; a planner needs mutable `Game`; or visibility must expand beyond `pub(super)`.
- Tests: task-focused core tests, all core/replay/contract tests, then the full section 4 matrix.

`tasks.rs` now owns the private `TaskState` and `CampaignState` representations plus pure planners over explicit immutable inputs. Task event reduction receives the current world, task states, floor ID, items, entities, and ordered domain events, then returns one state replacement for `Game` to commit. Task completion, pause, failure, abandonment, activation, and retake calculations return cloned state outcomes; floor storage, terrain changes, and reward generation remain in the floor-transition transaction.

Campaign victory, counts, score, victory transition, and retirement transition now use explicit campaign, dungeon, task, status, surface, and turn inputs. `Game` commits the planned campaign state, appends `CampaignVictorious` before `PlayerLevelCapUnlocked`, and retains experience-unlock orchestration. Death and loot still produce the authoritative events consumed by task reduction and remain in `mod.rs`. The extraction reduced `game/mod.rs` from 22,425 to 22,127 lines, added a 492-line `tasks.rs`, and added two focused zero-RNG/order tests, bringing core to 295 tests.

The complete Phase 9 acceptance matrix passed on 2026-07-31: formatting, protocol and content generators, source-pack verification, focused task/replay/contract tests, workspace check/clippy/test, all 54 frontend tests, TypeScript and UI builds, Tauri all-targets check, the Windows release build, and desktop WebDriver E2E. Generated bindings, schemas, content hashes, fixtures, protocol versions, save versions, and state-hash schemas remain unchanged.

### Phase 10: terrain interaction plans (completed)

- Scope: extract explicit read-only plans for opening and closing doors, bashing doors, disarming traps, digging terrain, and ordered active-search candidates; keep the existing `Game` methods as authoritative RNG and commit adapters.
- Not in scope: trap triggering or damage, passive perception during movement, ability-driven terrain transformation, item-driven trap/door destruction, generation, pathfinding, protocol/content changes, or event projection changes.
- Files: `game/terrain.rs`, `game/mod.rs`, world tests, and this plan.
- Risk: changing concealed-versus-known terrain lookup, eight-direction search order, occupancy priority, check difficulty sources, RNG draw count/order, revealed-terrain cleanup, changed-cell/event ordering, or failed-action world-time semantics.
- Acceptance: unsupported and occupied interactions remain zero RNG before scheduling; exact open/close/bash/search/disarm/dig behavior, interaction DTO ordering, terrain persistence, fixtures, replay/hash, generators, workspace checks, frontend build/tests, and Tauri release/E2E remain unchanged.
- Rollback: any terrain/revealed state, RNG draw, event order, state hash, save/replay/fixture, collision/visibility, or interaction DTO difference; a planner needs unrelated mutable state; or visibility must expand beyond `pub(super)`.
- Tests: focused world terrain-interaction tests, fixtures 65-71 through the contract suite, all core/replay tests, then the full section 4 matrix.

`terrain.rs` now owns an immutable interaction context over content, terrain, revealed positions, entities, items, map bounds, and player origin. Pure planners preserve the original distinctions: open, bash, and dig inspect known terrain; close inspects authoritative terrain; disarm requires a revealed authoritative trap; and active search enumerates unrevealed authoritative candidates in the existing eight-direction order. Actor occupancy still takes priority over ground-item occupancy, and every plan owns its stable position, terrain index, source, target, difficulty, and revealed-knowledge cleanup policy.

The `Game` adapters remain the authority for derived player skills, check RNG, terrain and revealed-state commits, changed cells, domain events, scheduling, turn/revision updates, save/hash state, and snapshot projection. Checked mutations commit only after success; unsupported or occupied targets return before RNG. Trap triggering and damage, passive perception, ability terrain transforms, item terrain effects, movement, and generation remain with their existing owners. No crate or external public API was added; the module exposes only the `pub(super)` outcomes and adapters required by dispatch and snapshot projection.

The extraction reduced `game/mod.rs` from 22,127 to 21,856 lines and added a 386-line `terrain.rs`. Two focused tests cover zero-RNG rejection across all six operations and checked disarm/dig commits, bringing core to 297 tests. The complete Phase 10 acceptance matrix passed on 2026-07-31: 20 focused world tests, all core/replay/contract tests, formatting, generators, source-pack verification, workspace check/clippy/test, all 54 frontend tests, TypeScript and UI builds, Tauri all-targets check, Windows release build, and desktop WebDriver E2E. Fixtures 65-71, generated files, content hash, protocol/save versions, and state-hash schema remain unchanged.

### Phase 11: floor lifecycle and recall plans (complete)

- Scope: extract stable floor/dungeon-instance identity helpers; stair, explicit-connection, teleport-level, and recall target resolution; dungeon-entry and one-shot preflight; retained/reset/TTL instance decisions; source/destination storage decisions; arrival-connection patching; following-summon selection and placement outcomes; floor activation; recall use/reset/countdown/destination calculations; and explicit transition outcomes for the existing event commit path.
- Not in scope: procedural generation algorithms, weighted generation RNG, entity/item ID allocation, task/campaign rule changes, task reward item construction, item consumption/awareness, recall delay dice, monster scheduling, movement/trap handling, save migrations, protocol/content changes, or a new manager/trait abstraction.
- Files: `game/floor.rs`, `game/mod.rs`, shared identity imports used by `game/persistence.rs`, world/items/summons/task tests, and this plan. `DungeonState` remains in the parent module to avoid a `floor`/`tasks` dependency cycle.
- Risk: changing `Ok(None)` versus invariant errors; source or destination storage keys; dungeon ordinal allocation; retained-instance TTL comparison and cleanup; generation and reward RNG order; ground/carried versus global item partitioning; summon eligibility, sorting, fallback, or placement; connection repair only on newly generated floors; task close/resume timing; recall countdown timing; destination-based changed-cell coverage; or domain-event order.
- Acceptance: exact floor IDs, instance IDs, terrain/entities/items, stored-floor keys, recall state, task state, RNG/ID counters, state hashes, errors, and event order remain unchanged across stairs, shafts, dynamic branches, teleport-level, one-shot tasks, reset/persistent/TTL dungeons, summon following, save/reload, and recall start/cancel/reset/trigger flows. No new crate/public API; only necessary `pub(super)` adapters.
- Rollback: any fixture/replay/hash/save/protocol difference; any reordered RNG draw, allocation, collection, changed cell, or event; a plan must clone the full aggregate to be useful; generation or task reward logic must move with the transition; visibility must expand beyond `pub(super)`; or the extraction creates a `floor`/`tasks`/`persistence` module cycle.
- Tests: add focused pure-plan and zero-mutation preflight tests, then run floor/connection/lifecycle world tests, recall and teleport-level item tests, summon-follow and one-shot task tests, all 297+ core tests, 33 replay tests, all 454 fixtures, generators, workspace check/clippy/test, all 54 frontend tests, Tauri all-targets/release build, and desktop WebDriver E2E.

Implementation was delivered in four compatibility-gated batches:

1. **Identity and target selection.** Move `dungeon_instance_storage_key`, dungeon/floor lookup and instance ID parse/format helpers, `FloorTransitionTarget`, stair/connection resolution, teleport-level target ordering/deduplication, and read-only entry predicates. Inputs are explicit world, current floor/terrain/connection, task/dungeon/item state, and recall state. Rejections must remain zero RNG and zero mutation.
2. **Transition preflight.** Introduce a private `FloorTransitionPlan` that records source/target identities, arrival/departure connection IDs, storage keys, dungeon instance reuse/allocation/retention action, one-shot departure/resume action, and ordered following-summon identities. Planning must complete before authoritative collections are taken and must preserve normal unavailability versus exact `CoreError` strings.
3. **Authoritative floor commit.** Move the existing `Game` adapters into `floor.rs` only after the first two batches pass. `Game` still partitions floor/global items, stores the source, restores or calls `generate_procedural_floor` at the original point, repairs dynamic arrival links, commits instance/task state, creates task rewards with the existing RNG timing, activates the destination, places summons, resets Guard position, and emits the current ordered outcome. No duplicate transition implementation is retained.
4. **Recall state machine.** Add explicit plans for start/cancel/reset, deepest-known destination updates, countdown decrement, and surface/dungeon trigger targets. Item use continues to own awareness, consumption, delay dice, and start/cancel/reset events; scheduling continues to decide when recall advances. Triggering continues through the common floor adapter and preserves `RecallTriggered` before `FloorTransitioned`, followed by summon, expedition, and task events.

Each batch passed its focused core filters plus replay and contract fixtures before the next began. The implementation preserves `delay + 1` initialization followed by same-command countdown, `turn + 1` retained timestamps, `turn - retained_at >= ttl` expiry, actor-before-item and ID-sorted summon behavior, global-items-before-destination-items activation order, and full changed-cell enumeration using destination dimensions.

The extraction moved floor identity, stair and teleport target resolution, dungeon-entry predicates, retained-instance decisions, the private `FloorTransitionPlan`, authoritative floor commit adapters, following-summon placement, transition event recording, and recall planning/countdown/triggering into `floor.rs`. Planning completes before authoritative entity, item, terrain, exploration, connection, or region collections are taken. Procedural generation, RNG and item-state rolls, ID allocation, task reward construction, item consumption and awareness, scheduling, and public projection remain with their existing owners. `game/mod.rs` decreased from 21,856 to 20,850 lines; `floor.rs` contains 1,450 lines including three focused identity, zero-mutation preflight, and recall timing tests, bringing core to 301 tests.

The complete Phase 11 acceptance matrix passed on 2026-07-31: all 301 core tests, 33 replay tests, all 454 contract fixtures, formatting, protocol/content generators, source-pack verification, workspace check/clippy/test, all 54 frontend tests, TypeScript and UI builds, Tauri all-targets check, Windows release build, and desktop WebDriver E2E. The committed fixtures and generated files are unchanged; content hash `cf977b882f1650f641035e1e12b22cca6430106a4992cceefd2e496060f51774`, protocol/save versions, and state-hash schema remain unchanged.

### Phase 12: damage, death, and attack resolution boundaries (completed)

- Purpose: establish one explicit aggregate-local contract for applying already-resolved damage, then extract the authoritative actor-death transaction, and only after both foundations are compatibility-locked move each attack family behind focused private adapters. The dependency order is mandatory: attack resolution may consume the shared damage and death contracts, while those contracts must not depend on a particular attack parser.
- Scope: characterize every damage and death entry point; represent target identity, HP before/after, applied damage, fatality policy, wake/alert result, and changed position as explicit private outcomes; extract ordinary actor death planning and commit orchestration; migrate player melee, projectiles, throwing, ability/item actor damage, monster attacks, retaliation, statuses, and environmental damage in separate compatibility-gated batches.
- Not in scope: combat balance changes, damage formula or armor changes in crate-level `rfb-core/src/combat.rs` and `effect.rs`, protocol/content/save/hash schema changes, new public APIs, new event kinds, task-credit policy changes, loot-table changes, player-death semantics, scheduler redesign, AI redesign, or forcing genocide and removal-only summon deaths through the ordinary death transaction.
- Target files: add aggregate-local `game/damage.rs` and `game/death.rs`; add narrowly owned attack modules only as each family is migrated; reduce the corresponding orchestration in `game/mod.rs`; extend combat, ability, item, monster-AI, summon, world, task, replay, and persistence tests. The existing crate-level `rfb-core/src/combat.rs` remains the pure armor/damage arithmetic boundary and is not replaced by the aggregate-local modules.
- Ownership rule: RNG draws, ID allocation, loot construction, progression rewards, task/campaign event reduction, scheduling, and final state/event commit remain authoritative `Game` operations unless a later batch names and tests their exact ordered contract. Attack owners retain source-specific checks, rolls, traces, resource changes, ammunition settlement, item awareness, resistance observation, status expiry, and event vocabulary.
- Risk: changing the player fatality threshold (`hp < 0`) versus ordinary actor death (`hp <= 0`); hit/wake/slay event order; RNG or item/corpse ID allocation order; kill-credit classification; carried-item sorting; guardian, mirror, pack, or changed-cell cleanup; lethal-hit short-circuiting; target relookup after an earlier death; or scheduling continuation after player death.
- Acceptance: all damage sources preserve exact HP, death threshold, RNG/ID counters, actor/item collections, corpse and loot contents, experience, task/campaign state, wake state, changed cells, errors, and ordered events. Save payloads, state hashes, replays, fixtures, generated files, protocol/save versions, and public APIs remain unchanged. New modules expose only the minimum `pub(super)` surface.
- Rollback: any fixture, replay, state-hash, save, protocol, event-order, RNG, allocation, corpse/loot, kill-credit, wake, or scheduling difference; an abstraction erases a source-specific death policy; a planner needs to clone the full aggregate; visibility must expand beyond `pub(super)`; or an attack-family move requires simultaneous changes to death, loot, progression, or task reduction.
- Tests: add characterization coverage for fatal and non-fatal damage, zero damage, both death thresholds, hit/wake/slay ordering, ordinary versus removal-only death, kill-credit event classification, corpse-before-loot RNG/ID order, carried-item ordering, guardian and mirror cleanup, pack dissolution, changed cells, lethal multi-hit short-circuit, and player-death scheduling interruption. Run focused tests after every batch, then all core tests, 33 replay tests, all 454 contract fixtures, generators, workspace check/clippy/test, all frontend tests and builds, Tauri all-targets/release build, and desktop WebDriver E2E.

Implementation must be split into the following reviewable commits. No commit may combine attack parsing/resolution movement with restructuring of death, drops, progression rewards, or task/campaign reduction:

1. **Contract census and characterization only.** Record the complete damage/death call-site matrix and add tests for the observable distinctions above. Do not move production code or change behavior.
2. **Shared damage application contract.** Introduce private damage application inputs/outcomes with an explicit fatality policy. Convert only the existing common HP-application helpers and their direct adapters. Attack validation, hit and damage rolls, source-specific events, wake ordering, death aftermath, scheduling, RNG, and IDs stay in place.
3. **Ordinary actor-death plan and authoritative commit.** Introduce a private `ActorDeathPlan` and move the existing ordinary death transaction without changing its sequence: determine corpse state and allocate its ID; generate ordinary loot; enumerate carried items in item-ID order; remove the actor and dissolve pack membership; append the caller-selected death event; grant experience and progression events; update guardian and mirror state; drop carried items; append generated loot; add the corpse; and mark changed cells under the existing condition. Player death, removal-only deaths, genocide, task reduction, and attack parsing remain outside this commit.
4. **Player melee.** Move validation, the multi-attack loop, damage planning, lethal short-circuiting, vampiric healing, melee resource gains, confusing strike, wake behavior, and the call into the stable death adapter. Do not alter the shared damage/death modules in this commit except for proven mechanical visibility fixes.
5. **Projectiles and throwing.** Migrate these paths in separate commits if either diff stops being locally reviewable. Preserve collision and target ordering, ammunition extraction/allocation and settlement, breakage/recovery, landing, source-specific slew events, and RNG order.
6. **Player ability and item actor damage.** Move one family per commit. Preserve shared-roll semantics, target relookup after earlier deaths, effect trace/DTO aggregation, item awareness and backlash, wake ordering, and the current distinction between ability, dispel, blast, and ordinary kill-credit events.
7. **Monster attacks and retaliation.** Preserve monster resistance observation, protection from evil, player fatality semantics, player-aligned removal-only deaths, vengeance aggregation, and scheduler stopping behavior. Split player-target and entity-target paths if they cannot share a contract without policy flags.
8. **Statuses, traps, and environmental damage.** Migrate last, and only where the shared contract expresses their existing semantics directly. Preserve status expiry and wake/event order, pure status processing, trap ownership, and player self-damage behavior. Genocide remains an explicit bypass unless separate compatibility evidence justifies changing that boundary.
9. **Consolidation audit.** Remove only superseded private helpers, verify that every original call site is mapped, update module ownership documentation, and run the complete acceptance matrix. Do not use this batch for behavioral cleanup or balance changes.

Each attack-family commit starts from the already-tested shared damage and death contracts. If a family exposes a missing policy, add and verify that policy in a dedicated contract commit before moving the family; do not modify the contract opportunistically inside the extraction commit.

Implementation followed the required dependency order. `damage.rs` now owns the private `DamageApplicationPlan`, explicit below-zero versus at-or-below-zero fatality policies, saturating HP application, surviving-target wake eligibility, incoming-damage scaling, and ordered actor status ticks. The existing crate-level `combat.rs` and `effect.rs` still own armor, resistance, and pure damage arithmetic. Player death remains `hp < 0`; ordinary entities remain fatal at `hp <= 0`.

`death.rs` owns the private `ActorDeathPlan` and the single ordinary actor-death adapter. Its preparation still rolls corpse runtime state and allocates the corpse ID before generating ordinary death loot, then collects carried items in item-ID order. Commit preserves actor removal and pack dissolution, caller-selected death event, experience/progression events, guardian and stored-floor mirror cleanup, carried and generated loot events, corpse insertion, and changed-cell behavior. Player death, removal-only summon deaths, and genocide remain separate paths.

Attack families were then moved without altering the stable damage/death contracts. `player_combat.rs` owns player melee, projectile, throwing, ability-to-entity damage, melee resource gains, confusing strike, and player-summon melee. `item_combat.rs` owns dispel, elemental blast/backlash, and detonation damage. `monster_combat.rs` owns monster ability damage, melee against player and aligned entities, resistance observation, protection from evil, and vengeance. `environment_combat.rs` owns trap damage after the existing saving-throw gate. Death-ray and dynamic item-projectile parsing remain with their effect owners but commit HP through the shared damage contract. Genocide fatigue and explicit item life loss remain direct HP-loss bypasses because their tested semantics intentionally bypass ordinary damage reduction and death aftermath.

The extraction reduced `game/mod.rs` from 20,850 to 19,309 lines and added 210 lines in `damage.rs`, 251 in `death.rs`, 616 in `player_combat.rs`, 261 in `item_combat.rs`, 406 in `monster_combat.rs`, and 73 in `environment_combat.rs`. Two focused damage-contract tests bring core to 303 tests. No crate-level public API, protocol/content/save version, state-hash schema, generated file, or committed fixture changed.

The complete Phase 12 acceptance matrix passed on 2026-07-31: all 303 core tests, 33 replay tests including the 10,000-turn no-drift case, all 454 contract fixtures, formatting, protocol/content generators, source-pack verification, workspace check/clippy/test, all 54 frontend tests, TypeScript and UI builds, Tauri all-targets check, Windows release build, and desktop WebDriver E2E. Content hash `cf977b882f1650f641035e1e12b22cca6430106a4992cceefd2e496060f51774`, generated bindings and schemas, fixtures, protocol/save versions, and state-hash schema remain unchanged.

Only one phase is active at a time. A phase does not begin until the prior phase passes its complete acceptance matrix.

## 4. Baseline on 2026-07-30

| Command | Result | Evidence |
| --- | --- | --- |
| `cargo fmt --all -- --check` | success | no formatting diff |
| `cargo run -p rfb-protocol --features bindings --bin generate-bindings -- --check` | success | committed TypeScript bindings current |
| `cargo run -p rfb-content --features schemas --bin generate-content-schemas -- --check` | success | committed schemas current |
| `cargo run -p rfb-content --bin rfb-contentc -- verify-source packs/rfb-demo-original` | success | pack `1.140.0`, hash `cf977b882f1650f641035e1e12b22cca6430106a4992cceefd2e496060f51774`, 28 actors, 90 items, 68 abilities, 6 builds |
| `cargo check --workspace --exclude rfb-tauri --all-targets` | success | all non-Tauri workspace targets |
| `cargo clippy --workspace --exclude rfb-tauri --all-targets -- -D warnings` | success | zero warnings |
| `cargo test --workspace --exclude rfb-tauri` | success | core 291, replay 33, contract fixtures 2, and all other crate/doc tests |
| `cargo check -p rfb-tauri --all-targets` | success | desktop host all targets |
| `npm test` in `web` | success | 37 frontend tests |
| `npm run typecheck` in `web` | success | strict TypeScript check |
| `npm run build:ui` in `web` | success | Vite production UI build |
| `npm run build -- --no-bundle` in `web` | success | Windows release executable produced |
| `npm run e2e` in `web` | success | debug WebDriver desktop E2E passed |
| `npm run android:build:debug` in `web` | skipped | Android Rust targets are installed, but this machine exposes no Android SDK/NDK environment; CI remains authoritative |

The baseline includes the 10,000-turn no-drift replay, save/reload continuation, deterministic fixed-seed tests, save migrations, state-hash normalization, and all committed contract fixtures.

## 5. Determinism and compatibility rules

Every extraction must preserve:

- collection type and iteration/sort order;
- RNG draw count and order, including zero-RNG rejection paths;
- actor/entity/item processing order and ID allocation order;
- integer/floating-point calculations and clamp points;
- command, domain-event, DTO-event, changed-cell, and removed-entity order;
- serde field order/defaults, save versions, hash schema, protocol version, and generated bindings;
- current error variants and migration error messages where tests or callers may observe them.

File movement alone is not evidence of equivalence. The replay, fixture, migration, hash, generator, frontend, and Tauri checks are the acceptance evidence.
