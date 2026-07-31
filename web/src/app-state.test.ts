// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { AppState } from "./app-state.ts";

test("application state starts with one empty, command-ready session", () => {
  const state = new AppState();

  assert.equal(state.busy, false);
  assert.equal(state.commandBlocked, false);
  assert.equal(state.connection, "starting");
  assert.deepEqual(state.inventory, []);
  assert.deepEqual(state.equipment, []);
  assert.equal(state.selectedInventoryIds.size, 0);
});

test("application state owns map dimensions and terminal command gating", () => {
  const state = new AppState();

  state.setMapSize(80, 45);
  state.campaignEnded = true;

  assert.equal(state.mapWidth, 80);
  assert.equal(state.mapHeight, 45);
  assert.equal(state.commandBlocked, true);
});
