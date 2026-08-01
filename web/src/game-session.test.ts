// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { GameSession } from "./game-session.ts";

function sessionState() {
  return {
    busy: false,
    playerDead: false,
    campaignEnded: false,
    get commandBlocked() {
      return this.playerDead || this.campaignEnded;
    },
  };
}

test("game session applies successful updates only after clearing busy", async () => {
  const state = sessionState();
  const calls = [];
  const update = { turn: 1 };
  const session = new GameSession({
    state,
    execute: async (command) => {
      calls.push(["execute", command.type, state.busy]);
      return update;
    },
    applyUpdate: (value, command) =>
      calls.push(["apply", value.turn, command.type, state.busy]),
    refreshBusyControls: () => calls.push(["controls", state.busy]),
    showError: (error) => calls.push(["error", error]),
  });

  await session.dispatch({ type: "wait" });

  assert.deepEqual(calls, [
    ["controls", true],
    ["execute", "wait", true],
    ["apply", 1, "wait", false],
  ]);
});

test("game session restores controls after failure and blocks terminal commands", async () => {
  const state = sessionState();
  const calls = [];
  const failure = new Error("dispatch failed");
  const session = new GameSession({
    state,
    execute: async () => {
      calls.push(["execute", state.busy]);
      throw failure;
    },
    applyUpdate: () => calls.push(["apply"]),
    refreshBusyControls: () => calls.push(["controls", state.busy]),
    showError: (error) => calls.push(["error", error]),
  });

  await session.dispatch({ type: "wait" });
  state.playerDead = true;
  await session.dispatch({ type: "wait" });

  assert.equal(state.busy, false);
  assert.deepEqual(calls, [
    ["controls", true],
    ["execute", true],
    ["error", failure],
    ["controls", false],
  ]);
});
