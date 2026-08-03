// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  commandForKeyboardInput,
  connectionActionForState,
  directionForKeyboardInput,
} from "./input-controller.ts";
import { AppState } from "./app-state.ts";

test("input presets preserve their movement and wait command mappings", () => {
  assert.deepEqual(
    commandForKeyboardInput({ key: "8", code: "Numpad8" }, "numpad"),
    { type: "move", direction: "north" },
  );
  assert.deepEqual(
    commandForKeyboardInput({ key: "5", code: "Numpad5" }, "numpad"),
    { type: "wait" },
  );
  assert.deepEqual(commandForKeyboardInput({ key: "h", code: "KeyH" }, "vi"), {
    type: "move",
    direction: "west",
  });
  assert.deepEqual(commandForKeyboardInput({ key: " ", code: "Space" }, "wasd"), {
    type: "wait",
  });
  assert.deepEqual(commandForKeyboardInput({ key: "r", code: "KeyR" }, "wasd"), {
    type: "rest",
    turns: 9_999,
  });
});

test("shared commands and diagonal directions remain preset-aware", () => {
  assert.deepEqual(commandForKeyboardInput({ key: "g", code: "KeyG" }, "vi"), {
    type: "pick-up",
  });
  assert.deepEqual(commandForKeyboardInput({ key: ">", code: "Period" }, "wasd"), {
    type: "traverse-stairs",
  });
  assert.equal(directionForKeyboardInput({ key: "e", code: "KeyE" }, "wasd"), "north-east");
  assert.equal(directionForKeyboardInput({ key: "x", code: "KeyX" }, "vi"), undefined);
});

test("connection actions distinguish the Warrens entrance and generated stairs", () => {
  const state = new AppState();
  state.worldId = "demo.world.warrens-journey";
  state.contentGlyphs.set("demo.terrain.stairs-down", ">");
  state.contentGlyphs.set("demo.terrain.stairs-up", "<");
  state.status = {
    floorId: "demo.floor.surface",
    player: { position: { x: 3, y: 4 } },
  };
  state.replaceCells([
    { position: { x: 3, y: 4 }, terrainId: "demo.terrain.stairs-down", itemId: null, actorId: null },
  ]);
  assert.equal(connectionActionForState(state), "enter-warrens");

  state.status = {
    floorId: "demo.floor.warrens-depth-1",
    player: { position: { x: 6, y: 6 } },
  };
  state.replaceCells([
    { position: { x: 6, y: 6 }, terrainId: "demo.terrain.stairs-up", itemId: null, actorId: null },
  ]);
  assert.equal(connectionActionForState(state), "ascend");
});
