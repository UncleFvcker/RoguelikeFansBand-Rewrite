// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  commandForKeyboardInput,
  directionForKeyboardInput,
} from "./input-controller.ts";

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
