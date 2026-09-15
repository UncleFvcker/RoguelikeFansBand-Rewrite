// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import type { TargetSpecDto } from "./protocol";
import {
  beginTargeting,
  cycleTarget,
  defaultTargetState,
  moveTarget,
  moveTargetCursor,
  targetSelectionAtCursor,
  translateTargetingState,
} from "./targeting.ts";

const SPEC: TargetSpecDto = {
  modes: ["direction", "position", "entity"],
  range: 2,
  requiresLineOfEffect: true,
};

test("default targeting preserves all four intentions, line-of-effect and pet eligibility", () => {
  const state = beginTargeting({ x: 3, y: 3 }, SPEC);
  const entities = [
    { id: "blocked", faction: "hostile", position: { x: 3, y: 4 }, inLineOfEffect: false },
    { id: "near", faction: "hostile", position: { x: 4, y: 3 }, inLineOfEffect: true },
    { id: "old", faction: "hostile", position: { x: 5, y: 3 }, inLineOfEffect: true },
    { id: "pet", faction: "player", position: { x: 3, y: 2 }, inLineOfEffect: true },
  ];
  const old = { type: "entity", entityId: "old" };
  assert.deepEqual(defaultTargetState(state, "manual", old, entities).cursor, state.origin);
  assert.deepEqual(defaultTargetState(state, "old-target", old, entities).cursor, { x: 5, y: 3 });
  assert.deepEqual(defaultTargetState(state, "nearest-enemy", old, entities).cursor, { x: 4, y: 3 });
  assert.deepEqual(defaultTargetState(state, "old-then-nearest", old, entities).cursor, { x: 5, y: 3 });
  const missing = { type: "entity", entityId: "gone" };
  assert.deepEqual(defaultTargetState(state, "old-target", missing, entities).cursor, state.origin);
  assert.deepEqual(defaultTargetState(state, "old-then-nearest", missing, entities).cursor, { x: 4, y: 3 });
  const pet = { type: "entity", entityId: "pet" };
  assert.deepEqual(defaultTargetState(state, "old-target", pet, entities).cursor, state.origin);
  assert.deepEqual(defaultTargetState({ ...state, targetPets: true }, "old-target", pet, entities).cursor, { x: 3, y: 2 });
  assert.deepEqual(defaultTargetState({ ...state, targetPets: true }, "nearest-enemy", pet, entities.filter(e => e.faction !== "hostile")).cursor, state.origin);
});

test("target list skips pets and out-of-range entities; directional selection falls back to free grids", () => {
  const entities = [
    { id: "pet", faction: "player", position: { x: 3, y: 3 } },
    { id: "near", position: { x: 4, y: 3 } },
    { id: "east", position: { x: 5, y: 3 } },
    { id: "out", position: { x: 12, y: 3 } },
  ];
  let state = cycleTarget(beginTargeting({ x: 3, y: 3 }, SPEC), entities, 0);
  assert.deepEqual(state.cursor, { x: 4, y: 3 });
  state = moveTarget(state, "east", entities, 20, 20);
  assert.deepEqual(state.cursor, { x: 5, y: 3 }); assert.equal(state.list, true);
  state = moveTarget(state, "south", entities, 20, 20);
  assert.deepEqual(state.cursor, { x: 5, y: 4 }); assert.equal(state.list, false);
  state = cycleTarget(state, entities, 1);
  assert.deepEqual(state.cursor, { x: 5, y: 3 });
  assert.deepEqual(targetSelectionAtCursor({ ...state, list: false }, entities), { type: "position", position: { x: 5, y: 3 } });
  assert.deepEqual(targetSelectionAtCursor({ ...state, list: false, spec: { ...SPEC, modes: ["entity"] } }, entities), { type: "entity", entityId: "east" });
});

test("target mode accepts direction, grid, or entity selection", () => {
  assert.equal(beginTargeting({ x: 3, y: 3 }, undefined), undefined);
  assert.deepEqual(
    beginTargeting(
      { x: 3, y: 3 },
      { modes: ["direction"], range: 6, requiresLineOfEffect: true },
    )?.cursor,
    { x: 3, y: 3 },
  );
  assert.deepEqual(beginTargeting({ x: 3, y: 3 }, SPEC)?.cursor, { x: 3, y: 3 });
});

test("direction targeting projects the cursor delta to one of eight directions", () => {
  let state = beginTargeting(
    { x: 3, y: 3 },
    { modes: ["direction"], range: 6, requiresLineOfEffect: false },
  )!;
  state = moveTargetCursor(state, "north-east", 10, 10);
  assert.deepEqual(targetSelectionAtCursor(state, []), {
    type: "direction",
    direction: "north-east",
  });
});

test("target cursor stays inside both the map and Chebyshev range", () => {
  let state = beginTargeting({ x: 1, y: 1 }, SPEC)!;
  state = moveTargetCursor(state, "north-west", 4, 4);
  assert.deepEqual(state.cursor, { x: 0, y: 0 });
  state = moveTargetCursor(state, "north-west", 4, 4);
  assert.deepEqual(state.cursor, { x: 0, y: 0 });
  state = moveTargetCursor(state, "south-east", 4, 4);
  state = moveTargetCursor(state, "south-east", 4, 4);
  state = moveTargetCursor(state, "south-east", 4, 4);
  assert.deepEqual(state.cursor, { x: 3, y: 3 });
});

test("targeting coordinates follow wilderness map translations", () => {
  const state = {
    origin: { x: 64, y: 16 },
    cursor: { x: 70, y: 20 },
    spec: SPEC,
  };
  assert.deepEqual(translateTargetingState(state, { x: -32, y: 0 }, 96, 33), {
    ...state,
    origin: { x: 32, y: 16 },
    cursor: { x: 38, y: 20 },
  });
  assert.equal(
    translateTargetingState(
      { ...state, origin: { x: 64, y: 22 }, cursor: { x: 70, y: 5 } },
      { x: 0, y: -11 },
      96,
      33,
    ),
    undefined,
  );
});

test("confirmation prefers a stable entity id then falls back to a position", () => {
  let state = beginTargeting({ x: 3, y: 3 }, SPEC)!;
  state = moveTargetCursor(state, "east", 20, 20);
  assert.deepEqual(
    targetSelectionAtCursor(state, [
      { id: "monster.z", position: { x: 4, y: 3 } },
      { id: "monster.a", position: { x: 4, y: 3 } },
    ]),
    { type: "entity", entityId: "monster.a" },
  );
  assert.deepEqual(targetSelectionAtCursor(state, []), {
    type: "position",
    position: { x: 4, y: 3 },
  });
  assert.equal(
    targetSelectionAtCursor(beginTargeting({ x: 3, y: 3 }, SPEC)!, []),
    undefined,
  );
});

test("entity targeting can select a mount sharing the player's position", () => {
  const state = beginTargeting(
    { x: 3, y: 3 },
    { modes: ["entity"], range: 8, requiresLineOfEffect: true },
  )!;
  assert.deepEqual(
    targetSelectionAtCursor(state, [{ id: "mount", position: { x: 3, y: 3 } }]),
    { type: "entity", entityId: "mount" },
  );
});
