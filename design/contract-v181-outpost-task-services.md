# Contract v181: Outpost Task Services and The Thieves' Hideout

Status: implemented for the first playable Outpost commission; protocol 1.138, built-in pack 1.176.0, state-hash Schema v63.

## Authoritative RFB sources

The authority is the `master` ref in `D:/codex/Frogcomposband/master` at commit `efd63661302866038f58d8cd2553b23e6af3bf9d`:

- `lib/edit/q_info.txt`, quest 1: level 5 town-generated quest using `q_thieves.txt`.
- `lib/edit/q_thieves.txt`: Chinese briefing/failure/reward text, the 21x8 permanent-wall map, five doors, eight independent 50% random-trap candidates, four random object cells, and the ten-member depth+5 THIEF formation indexed by `2/5/7/4/0/8`.
- `lib/edit/t_outp.txt`: Count Uldric II, the Count building, and the northeastern quest entrance state.
- `lib/edit/r_info.txt`: the seven allocation-eligible THIEF candidates and their Chinese runtime names.

## Content contract

- `TownFacilityDefinition` exposes a strict quest-giver category, owner name, and ordered task IDs.
- `TaskDefinition` owns its source facility, dedicated floors or dungeon-depth location, objectives, target placements, prerequisite, and reward.
- A target placement contains only an objective index, an existing floor ID, and a spawn count. Actor candidates belong to the objective; distance and friend/formation policy are not task-content fields.
- A dedicated fixed task floor uses its own inline map definition. It is not a reusable shared quest map.
- Conditional inline terrain records an explicit percentage and fallback terrain. The Thieves' Hideout uses 50% trap placement over ordinary floor.
- The fixed map uses a dedicated permanent wall terrain with the original Chinese name `永久墙`; monsters cannot destroy it.
- The seven THIEF candidates are ordinary actor content and remain eligible anywhere their normal allocation rules permit.

## Runtime contract

- Facility tasks are projected without materializing untouched state. Prerequisites project as `locked`; a directly available task projects as `available`.
- Accepting at the correct entrance is a zero-time, zero-RNG transaction. It changes the task to `taken` and opens only that task's entrance.
- Entering the dedicated floor changes the task to `active`. The inline map consumes RNG only for its declared random terrain, four loot rolls, and ten weighted formation draws.
- Formation candidates are sorted by descending level and stable legacy index. Only indices `0/2/4/5/7/8` are placed, with no `FRIENDS` expansion.
- Leaving before the floor is clear fails the non-retakeable task and closes its entrance. Clearing it changes the state to `reward-available`; returning to town does not grant the reward automatically.
- Claiming at Count Uldric II is atomic. The Warrior slice receives the content-defined Broad Sword instance, then the task becomes `completed`.
- The Web client opens a task-service dialog at the facility entrance and exposes commands only for `available` and `reward-available` states.

## Explicit remaining differences

- `TRAP(*)` still resolves to the currently supported Warrens snare. The original depth-filtered random trap allocation remains a separate content/mechanism import.
- `$:OBJ(*)` currently uses the supported Warrens loot table rather than the complete original depth-5 object allocator.
- The production task currently locks the Warrior Broad Sword reward. The full original race/class conditional reward matrix is not represented yet.
- `BEG`, melee `EAT_GOLD`/`EAT_ITEM`, and ground-item `TAKE_ITEM` are not replaced with generic damage. They remain explicit monster-mechanism backlog items.
- Pest Control remains W14 and requires this completed task as its prerequisite.

Routine verification stays focused on content/world/task tests, protocol/schema generation, localization, Web tests/type checking, and relevant builds. No large desktop E2E is required for this contract.

The active baseline contains 468 exact fixtures with zero waivers. Six single-command task fixtures independently cover acceptance, dedicated-floor entry, clear-floor completion, early-exit failure, return after completion, and reward claim. Location-dependent setup uses direct player positions; non-movement behaviors do not add movement commands.
