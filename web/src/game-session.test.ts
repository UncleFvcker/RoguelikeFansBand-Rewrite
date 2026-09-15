// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { GameSession } from "./game-session.ts";
import { AppState } from "./app-state.ts";

test("recorders observe normalized successful commands, not internal updates or stale replies", async () => {
  const records = [], state = sessionState();
  let pending;
  const session = new GameSession({ state, execute: async () => pending ? pending.promise : {},
    applyUpdate() {}, refreshBusyControls() {}, showError() {}, onRememberedCommand: command => records.push(command) });
  await session.dispatch({ type: "wait" }, { type: "rest-for-turns", turns: 3 });
  await session.dispatch({ type: "configure-travel", options: {} });
  assert.deepEqual(records, [{ type: "rest-for-turns", turns: 3 }]);
  pending = Promise.withResolvers(); const old = session.dispatch({ type: "search" });
  session.resetCommandHistory(); pending.resolve({}); await old;
  assert.equal(records.length, 1);
  pending = Promise.withResolvers(); const failed = session.dispatch({ type: "search" });
  pending.reject(new Error("rejected")); await failed;
  assert.deepEqual(records, [{ type: "rest-for-turns", turns: 3 }, undefined]);
});

test("last command keeps independent copies of chosen parameters and ignores internal updates", async () => {
  const state = sessionState();
  const session = new GameSession({ state, execute: async () => ({}),
    applyUpdate() {}, refreshBusyControls() {}, showError() {},
  });
  assert.equal(session.lastCommand, undefined);
  const command = { type: "use-absorbed-device", itemId: "device-a", targets: [{ type: "item", itemId: "item-b" }] };
  await session.dispatch(command);
  command.targets[0].itemId = "unrelated";
  const replay = session.lastCommand;
  assert.equal(replay.targets[0].itemId, "item-b");
  replay.targets.length = 0;
  for (const type of ["continue-run", "cancel-run", "resolve-ability-direction", "set-interface-locale", "configure-travel"]) {
    await session.dispatch({ type });
  }
  assert.equal(session.lastCommand.targets[0].itemId, "item-b");
  state.busy = true;
  assert.equal(await session.dispatch({ type: "move", direction: "east" }), "blocked");
  state.busy = false;
  assert.equal(session.lastCommand.itemId, "device-a");
  await session.dispatch({ type: "buy-from-shop", shopId: "shop", itemId: "x", quantity: 1 });
  assert.equal(session.lastCommand, undefined, "unsupported user actions clear history instead of replaying an older action");
  await session.dispatch({ type: "pick-up-item", itemId: "chosen" });
  assert.deepEqual(session.lastCommand, { type: "pick-up-item", itemId: "chosen" });
});

test("history clears on failures, new sessions, floor changes and map translations", async () => {
  const state = sessionState();
  let next = {}, pending;
  const session = new GameSession({ state, execute: async () => {
    if (pending) return pending.promise;
    if (next instanceof Error) throw next;
    return next;
  }, applyUpdate() {}, refreshBusyControls() {}, showError() {} });
  await session.dispatch({ type: "search" });
  next = new Error("item no longer exists");
  assert.equal(await session.dispatch({ type: "use-item", itemId: "gone" }), "failed");
  assert.equal(session.lastCommand, undefined);
  next = {};
  await session.dispatch({ type: "search" });
  pending = Promise.withResolvers();
  const inFlight = session.dispatch({ type: "move", direction: "east" });
  session.resetCommandHistory();
  pending.resolve({});
  await inFlight;
  assert.equal(session.lastCommand, undefined, "old replies cannot repopulate a new session");
  pending = undefined;
  state.status = { floorId: "a", mapScale: "local" };
  for (const update of [{ floorId: "b", mapScale: "local" },
    { floorId: "a", mapScale: "world" },
    { floorId: "a", mapScale: "local", mapTranslation: { x: -10, y: 0 } }]) {
    next = update;
    await session.dispatch({ type: "cast-ability", abilityId: "spell", target: { type: "position", position: { x: 2, y: 3 } } });
    assert.equal(session.lastCommand, undefined);
  }
});

test("a pending Maia choice blocks play and accepts an explicit choice on the world map", async () => {
  const state = new AppState();
  state.mode = "playing";
  state.status = { player: { pendingMaiaPathChoice: true }, mapScale: "world" };
  const calls = [];
  const session = new GameSession({ state, execute: async command => { calls.push(command); return {}; }, applyUpdate: () => {}, refreshBusyControls: () => {}, showError: error => { throw error; } });
  assert.equal(state.commandBlocked, true);
  await session.dispatch({ type: "wait" });
  assert.equal(calls.length, 0);
  await session.dispatch({ type: "choose-maia-path", path: "enlightened" });
  assert.deepEqual(calls, [{ type: "choose-maia-path", path: "enlightened" }]);
});

test("global behavior updates work during a pending choice and on the world map without becoming repeatable", async () => {
  const state = new AppState(); state.mode = "playing";
  state.status = { player: { pendingMaiaPathChoice: true }, mapScale: "world" };
  const calls = [];
  const session = new GameSession({ state, execute: async command => { calls.push(command); return {}; }, applyUpdate() {}, refreshBusyControls() {}, showError: error => { throw error; } });
  assert.equal(await session.dispatch({ type: "configure-preferences", preferences: {} }), "applied");
  assert.equal(calls.length, 1); assert.equal(session.lastCommand, undefined);
  state.playerDead = true;
  assert.equal(await session.dispatch({ type: "configure-preferences", preferences: {} }), "blocked");
  assert.equal(calls.length, 1);
});

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

test("whenIdle waits for a committed update or failure, never a blocked duplicate", async () => {
  for (const fails of [false, true]) {
    const state = sessionState();
    const pending = Promise.withResolvers();
    let applied = false, idle = false;
    const session = new GameSession({ state, execute: () => pending.promise,
      applyUpdate() { applied = true; }, refreshBusyControls() {}, showError() {},
    });
    const dispatch = session.dispatch({ type: "wait" });
    const boundary = session.whenIdle().then(() => { idle = true; });
    assert.equal(await session.dispatch({ type: "wait" }), "blocked");
    assert.equal(idle, false);
    assert.equal(session.isDispatching, true);
    if (fails) pending.reject(new Error("failed")); else pending.resolve({});
    await dispatch; await boundary;
    assert.equal(applied, !fails);
    assert.equal(idle, true);
    assert.equal(session.isDispatching, false);
    assert.equal(state.busy, false);
  }
});

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

  assert.equal(await session.dispatch({ type: "wait" }), "applied");

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
  await session.dispatch({ type: "configure-travel", options: { alwaysPickup: false, autoDetectTraps: true, autoMapArea: true, disturbTrapDetect: false } });
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
    "configure-travel",
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

  assert.equal(await session.dispatch({ type: "wait" }), "failed");
  state.playerDead = true;
  assert.equal(await session.dispatch({ type: "wait" }), "blocked");

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
  await session.dispatch({ type: "resolve-realm-change", confirm: false });

  assert.deepEqual(calls, [
    "resolve-mutation-direction",
    "resolve-ability-direction",
    "cancel-ability-direction",
    "resolve-realm-change",
  ]);
});
