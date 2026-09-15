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
    campaign: { status: "active", targetNameKey: "actor-demo-the-serpent-of-chaos-name" },
    ...overrides,
  };
}

test("journey status uses only the current floor guardian, including defeated guardians", () => {
  assert.deepEqual(
    selectJourneyDungeonStatus(state()),
    {
      dungeonNameKey: "floor-demo-surface-name",
      currentDepth: undefined,
      maximumDepth: undefined,
      bossNameKey: undefined,
      bossDefeated: undefined,
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(state({ floorId: "demo.floor.angband-depth-99", dungeon: { nameKey: "floor-demo-angband-depth-name", currentDepth: 99, maximumDepth: 127, guardian: { nameKey: "actor-demo-oberon-king-of-amber-name", defeated: false } } })),
    {
      dungeonNameKey: "floor-demo-angband-depth-name",
      currentDepth: 99,
      maximumDepth: 127,
      bossNameKey: "actor-demo-oberon-king-of-amber-name",
      bossDefeated: false,
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(
      state({ floorId: "demo.floor.angband-depth-101", dungeon: { nameKey: "floor-demo-angband-depth-name", currentDepth: 101, maximumDepth: 127 }, campaign: { status: "victorious" } }),
    ),
    {
      dungeonNameKey: "floor-demo-angband-depth-name",
      currentDepth: 101,
      maximumDepth: 127,
      bossNameKey: undefined,
      bossDefeated: undefined,
    },
  );
  assert.deepEqual(
    selectJourneyDungeonStatus(state({ worldId: "demo.world.original-v1" })),
    { dungeonNameKey: "journey-dungeon-none" },
  );
  const defeated = selectJourneyDungeonStatus(state({ dungeon: { nameKey: "warrens", currentDepth: 9, maximumDepth: 9, guardian: { nameKey: "guardian", defeated: true } } }));
  assert.equal(defeated.bossNameKey, "guardian");
  assert.equal(defeated.bossDefeated, true);
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
