// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  completedPromptsForUpdate,
  selectJourneyDungeonStatus,
  selectOnboardingPrompt,
} from "./journey-guidance.ts";

function state(overrides = {}) {
  return {
    turn: 0,
    worldId: "demo.world.middle-earth",
    floorId: "demo.floor.surface",
    player: { position: { x: 3, y: 3 }, resources: [] },
    inventory: [],
    equipment: [],
    items: [],
    entities: [],
    campaign: { status: "active" },
    ...overrides,
  };
}

test("journey status shows dungeon depth and only an undefeated boss", () => {
  assert.deepEqual(
    selectJourneyDungeonStatus(state()),
    {
      dungeonNameKey: "floor-demo-surface-name",
      bossNameKey: "actor-demo-warrens-keeper-name",
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(state({ floorId: "demo.floor.warrens-depth-7" })),
    {
      dungeonNameKey: "dungeon-demo-warrens-name",
      currentDepth: 7,
      maximumDepth: 9,
      bossNameKey: "actor-demo-warrens-keeper-name",
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(
      state({ floorId: "demo.floor.warrens-depth-9", campaign: { status: "victorious" } }),
    ),
    {
      dungeonNameKey: "dungeon-demo-warrens-name",
      currentDepth: 9,
      maximumDepth: 9,
      bossNameKey: undefined,
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(state({ worldId: "demo.world.original-v1" })),
    { dungeonNameKey: "journey-dungeon-none" },
  );
});

test("onboarding distinguishes journey prompts from suppressible optional help", () => {
  const initial = state({ items: [{ id: "ground-item" }] });
  assert.equal(selectOnboardingPrompt(initial, new Set(), false)?.id, "movement");
  const afterMovement = new Set(["movement"]);
  assert.equal(selectOnboardingPrompt(initial, afterMovement, false)?.id, "look");
  assert.equal(selectOnboardingPrompt(initial, afterMovement, true)?.id, "pickup");
});

test("onboarding completion follows successful commands and state transitions", () => {
  const before = state({ turn: 3, items: [{ id: "item" }] });
  const update = state({
    turn: 4,
    floorId: "demo.floor.warrens-depth-1",
    player: { position: { x: 4, y: 3 }, resources: [] },
    inventory: [{ id: "item" }],
    items: [],
    baseRevision: 3,
    revision: 4,
    events: [{ kind: "item.picked-up", messageKey: "item-pickup-success", args: {} }],
  });
  assert.deepEqual(
    [...completedPromptsForUpdate({ type: "traverse-stairs" }, before, update, undefined)].sort(),
    ["movement", "pickup", "stairs"],
  );

  const rejected = state({
    turn: 4,
    baseRevision: 3,
    revision: 4,
    events: [{ kind: "floor.transition-unavailable", messageKey: "floor-transition-unavailable", args: {} }],
  });
  assert.deepEqual(
    [...completedPromptsForUpdate({ type: "traverse-stairs" }, before, rejected, undefined)],
    [],
  );
});
