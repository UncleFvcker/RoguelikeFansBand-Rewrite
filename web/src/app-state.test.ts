// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { AppState } from "./app-state.ts";

test("application state starts at the title screen without a game session", () => {
  const state = new AppState();

  assert.equal(state.busy, false);
  assert.equal(state.mode, "title");
  assert.equal(state.commandBlocked, true);
  assert.equal(state.connection, "starting");
  assert.deepEqual(state.inventory, []);
  assert.deepEqual(state.equipment, []);
  assert.equal(state.selectedInventoryIds.size, 0);
});

test("application state owns map dimensions and terminal command gating", () => {
  const state = new AppState();

  state.setMapSize(80, 45);
  state.mode = "playing";
  state.campaignEnded = true;

  assert.equal(state.mapWidth, 80);
  assert.equal(state.mapHeight, 45);
  assert.equal(state.commandBlocked, true);
});

test("application state maintains authoritative visibility deltas for look mode", () => {
  const state = new AppState();
  state.replaceVisualCells([
    { position: { x: 1, y: 2 }, visibility: "visible", light: {} },
  ]);
  state.updateVisualCells([
    { position: { x: 1, y: 2 }, visibility: "remembered", light: {} },
    { position: { x: 2, y: 2 }, visibility: "visible", light: {} },
  ]);

  assert.equal(state.cellVisibility.get("1,2"), "remembered");
  assert.equal(state.cellVisibility.get("2,2"), "visible");
});
