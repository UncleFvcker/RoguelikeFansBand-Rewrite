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
    worldMap: false,
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

test("world map accepts travel and zero-time character configuration", async () => {
  const state = sessionState();
  state.worldMap = true;
  const calls = [];
  const session = new GameSession({
    state,
    execute: async (command) => {
      calls.push(command.type);
      return { turn: 1 };
    },
    applyUpdate: () => {},
    refreshBusyControls: () => {},
    showError: () => {},
  });

  await session.dispatch({ type: "pick-up" });
  await session.dispatch({ type: "fire", direction: "east" });
  await session.dispatch({ type: "move", direction: "east" });
  await session.dispatch({ type: "travel-world", destination: { x: 30, y: 52 } });
  await session.dispatch({ type: "set-interface-locale", locale: "zh-CN" });
  await session.dispatch({
    type: "configure-mogaminator",
    enabled: true,
    leaveDestroyedItems: false,
    autoGetMode: "off",
    locale: "zh-CN",
    source: "物品",
  });
  await session.dispatch({ type: "leave-world-map" });

  assert.deepEqual(calls, [
    "move",
    "travel-world",
    "set-interface-locale",
    "configure-mogaminator",
    "leave-world-map",
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

test("pending directions block ordinary commands but accept their resolver and cancellation", async () => {
  const state = sessionState();
  state.pending = true;
  Object.defineProperty(state, "commandBlocked", {
    get() {
      return this.playerDead || this.campaignEnded || this.pending;
    },
  });
  const calls = [];
  const session = new GameSession({
    state,
    execute: async (command) => {
      calls.push(command.type);
      return { turn: 1 };
    },
    applyUpdate: () => {},
    refreshBusyControls: () => {},
    showError: () => {},
  });

  await session.dispatch({ type: "wait" });
  await session.dispatch({ type: "resolve-mutation-direction", direction: "east" });
  await session.dispatch({ type: "resolve-ability-direction", direction: "east" });
  await session.dispatch({ type: "cancel-ability-direction" });

  assert.deepEqual(calls, [
    "resolve-mutation-direction",
    "resolve-ability-direction",
    "cancel-ability-direction",
  ]);
});
