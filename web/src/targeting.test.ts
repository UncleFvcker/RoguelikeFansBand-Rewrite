// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import type { TargetSpecDto } from "./protocol";
import {
  beginTargeting,
  moveTargetCursor,
  targetSelectionAtCursor,
  translateTargetingState,
} from "./targeting.ts";

const SPEC: TargetSpecDto = {
  modes: ["direction", "position", "entity"],
  range: 2,
  requiresLineOfEffect: true,
};

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
