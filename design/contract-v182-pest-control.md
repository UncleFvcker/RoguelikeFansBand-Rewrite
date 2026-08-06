# Contract v182: Pest Control

Status: implemented for the first playable Outpost Pest Control commission; protocol 1.138, built-in pack 1.177.0, state-hash Schema v63.

## Authoritative RFB sources

The authority is the `master` ref in `D:/codex/Frogcomposband/master` at commit `8442f98ae8492afdbfb96aca4e92932669e817c4`:

- `lib/edit/q_info.txt`, quest 14: the Outpost Pest Control quest, bound to Warrens depth 5 with `KILL(^warg$, 8)`.
- `lib/edit/q_wargs.txt`: the Warg task map/encounter contract and default `Fur Cloak` reward.
- `lib/edit/t_outp.txt`: the Count facility unlock ordering after Thieves' Hideout.
- `src/quest.c`: target distance, incomplete-departure failure, and the completion down-stair behavior.

The task target is exactly eight Wargs. The Warg `FRIENDS(3d3)` flag is its ordinary ecology group rule and is not the Pest Control target count.

## Content contract

- `demo.task.pest-control` is sourced from the Outpost Count facility and requires the completed `demo.task.thieves-hideout` task.
- The target is `demo.actor.warg` on `demo.floor.warrens-depth-5`, with `required: 8`.
- The target floor uses the existing Warrens depth definition. The task does not own or replace the generic Warrens map definition.
- `completionExitTerrainId` is restricted to an external dungeon-depth task and identifies the down-stair terrain that becomes available only after completion.
- The reward is one content-defined `demo.item.fur-cloak`, using the original cloak slot, defense, weight, and base value.

## Runtime contract

- Accepting at the Count is a zero-time, zero-RNG transaction. The task becomes `taken` only after the Thieves' Hideout prerequisite is complete.
- While the task is active, Warrens depth 5 places only the remaining number of Wargs. Every target is at least ten cells from the player, and ordinary down stairs are suppressed.
- The task counts Warg deaths through the existing task event subscription. The final required kill changes the task to `reward-available`, reveals one deterministic magic stair, and consumes no additional RNG.
- Leaving Warrens 5 before all eight Wargs are dead changes the task to `failed` and discards the blocked stored floor. Partial kills are not preserved by an incomplete departure.
- Returning to the Count and claiming the reward is atomic: the Fur Cloak is created only when the task is `reward-available`, then the task becomes `completed`.
- The Web task service projects the locked/available/taken/active/reward-available/failed/completed states through the existing Count facility panel.

## Focused verification

- Content validation: `pest_control_matches_the_original_warrens_contract`.
- Core task tests: prerequisite lock/unlock, zero-RNG acceptance, remaining-Warg placement, distance and stair suppression, final magic stair, incomplete-departure failure, and Fur Cloak claim.
- Contract fixtures: `474-task-pest-control-accept.json` and `475-task-pest-control-claim.json`, both in the `tasks` category and each containing one command.

Routine verification remains focused on the affected content/world/tasks categories, schema and protocol generation, Web type checking, and the standalone Tauri debug build. No large desktop E2E suite is required for this contract.

## Explicit remaining differences

- The full original task-map and town-service breadth is not implied by this slice; task maps remain task-specific rather than a shared generic map definition.
- Thieves' Hideout's original `BEG`, melee `EAT_GOLD`/`EAT_ITEM`, `TAKE_ITEM`, complete trap allocator, complete depth-5 object allocator, and non-Warrior reward matrix remain separate backlog items.
