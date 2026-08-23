// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { playerPageForShortcut } from "./player-ui-layout.ts";

test("player pages use conventional shortcuts without consuming movement keys", () => {
  assert.equal(playerPageForShortcut("i"), "inventory");
  assert.equal(playerPageForShortcut("I"), "inventory");
  assert.equal(playerPageForShortcut("m"), "ability");
  assert.equal(playerPageForShortcut("w"), undefined);
});
