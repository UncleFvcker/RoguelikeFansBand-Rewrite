// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  playerPageForShortcut,
  readPanelPresentation,
} from "./player-ui-layout.ts";

test("player pages use conventional shortcuts without consuming movement keys", () => {
  assert.equal(playerPageForShortcut("i"), "inventory");
  assert.equal(playerPageForShortcut("I"), "inventory");
  assert.equal(playerPageForShortcut("m"), "ability");
  assert.equal(playerPageForShortcut("w"), undefined);
});

test("panel presentation defaults to a transient page and accepts a pinned column", () => {
  assert.equal(readPanelPresentation({ getItem: () => null }, "inventory"), "page");
  assert.equal(readPanelPresentation({ getItem: () => "invalid" }, "ability"), "page");
  assert.equal(readPanelPresentation({ getItem: () => "column" }, "inventory"), "column");
  assert.equal(
    readPanelPresentation({ getItem: () => { throw new Error("blocked"); } }, "ability"),
    "page",
  );
});
