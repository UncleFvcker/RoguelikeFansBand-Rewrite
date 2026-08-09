// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  InputController,
  autoGetStopsAfterStep,
  commandForKeyboardInput,
  connectionActionForState,
  directionForKeyboardInput,
  isAutoGetShortcut,
  isObjectListShortcut,
  localTravelStopsAfterStep,
  nextTravelConnectionPosition,
  translatedLocalPosition,
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

test("Ctrl+G is distinct from lowercase pickup", () => {
  const modifiers = { shiftKey: false, altKey: false, metaKey: false };
  assert.equal(isAutoGetShortcut({ key: "g", ctrlKey: true, ...modifiers }), true);
  assert.equal(isAutoGetShortcut({ key: "g", ctrlKey: false, ...modifiers }), false);
  assert.equal(
    isAutoGetShortcut({ key: "G", ctrlKey: true, ...modifiers, shiftKey: true }),
    false,
  );
  assert.deepEqual(commandForKeyboardInput({ key: "g", code: "KeyG" }, "vi"), {
    type: "pick-up",
  });
});

test("the object list keeps Shift+O distinct from lowercase open-door input", () => {
  const modifiers = { ctrlKey: false, altKey: false, metaKey: false };
  assert.equal(isObjectListShortcut({ key: "O", shiftKey: true, ...modifiers }), true);
  assert.equal(isObjectListShortcut({ key: "]", shiftKey: false, ...modifiers }), true);
  assert.equal(isObjectListShortcut({ key: "o", shiftKey: false, ...modifiers }), false);
  assert.equal(isObjectListShortcut({ key: "O", shiftKey: false, ...modifiers }), false);
  assert.equal(
    isObjectListShortcut({ key: "O", shiftKey: true, ...modifiers, ctrlKey: true }),
    false,
  );
});

test("connection actions distinguish the Warrens entrance and generated stairs", () => {
  const state = new AppState();
  state.worldId = "demo.world.middle-earth";
  state.contentGlyphs.set("demo.terrain.stairs-down", ">");
  state.contentGlyphs.set("demo.terrain.stairs-up", "<");
  state.status = {
    mapScale: "local",
    floorId: "demo.floor.surface",
    player: { position: { x: 3, y: 4 } },
    entities: [],
  };
  state.replaceCells([
    { position: { x: 3, y: 4 }, terrainId: "demo.terrain.stairs-down", itemId: null, actorId: null },
  ]);
  assert.equal(connectionActionForState(state), "enter-warrens");

  state.status = {
    mapScale: "local",
    floorId: "demo.floor.warrens-depth-1",
    player: { position: { x: 6, y: 6 } },
    entities: [],
  };
  state.replaceCells([
    { position: { x: 6, y: 6 }, terrainId: "demo.terrain.stairs-up", itemId: null, actorId: null },
  ]);
  assert.equal(connectionActionForState(state), "ascend");

  state.status = {
    mapScale: "local",
    floorId: "demo.floor.surface",
    player: { position: { x: 44, y: 16 } },
    entities: [],
  };
  state.replaceCells([
    { position: { x: 44, y: 16 }, terrainId: "demo.terrain.surface-path", itemId: null, actorId: null },
  ]);
  assert.equal(connectionActionForState(state), "enter-world-map");

  state.status = {
    mapScale: "local",
    floorId: "core.floor.wilderness",
    player: { position: { x: 44, y: 16 } },
    entities: [{ id: "core.floor.wilderness.29.52.ambush.1", faction: "hostile" }],
  };
  assert.equal(connectionActionForState(state), undefined);
  state.status.entities = [
    {
      id: "summon.test.ambush-threat",
      faction: "hostile",
      summon: { ownerId: "core.floor.wilderness.29.52.ambush.1" },
    },
  ];
  assert.equal(connectionActionForState(state), undefined);
  state.status.entities = [];
  assert.equal(connectionActionForState(state), "enter-world-map");

  state.status = {
    mapScale: "world",
    floorId: "demo.floor.surface",
    player: { position: { x: 28, y: 52 } },
    entities: [],
  };
  assert.equal(connectionActionForState(state), "leave-world-map");
});

test("local travel selection cycles only remembered stairs of the requested direction", () => {
  const state = new AppState();
  state.status = {
    mapScale: "local",
    floorId: "demo.floor.warrens-depth-1",
    player: { position: { x: 5, y: 5 } },
  };
  state.contentGlyphs.set("demo.terrain.stairs-up", "<");
  state.contentGlyphs.set("demo.terrain.stairs-down", ">");
  state.replaceCells([
    { position: { x: 3, y: 3 }, terrainId: "demo.terrain.stairs-up" },
    { position: { x: 7, y: 7 }, terrainId: "demo.terrain.stairs-up" },
    { position: { x: 6, y: 5 }, terrainId: "demo.terrain.stairs-down" },
  ]);
  state.cellVisibility.set("3,3", "remembered");
  state.cellVisibility.set("7,7", "visible");
  state.cellVisibility.set("6,5", "hidden");

  assert.deepEqual(nextTravelConnectionPosition(state, "<", { x: 5, y: 5 }), {
    x: 3,
    y: 3,
  });
  assert.deepEqual(nextTravelConnectionPosition(state, "<", { x: 3, y: 3 }), {
    x: 7,
    y: 7,
  });
  assert.equal(nextTravelConnectionPosition(state, ">", { x: 5, y: 5 }), undefined);
});

test("local travel stops after damage, a visible enemy, or blocked movement", () => {
  const before = {
    mapScale: "local",
    floorId: "floor.1",
    player: {
      position: { x: 1, y: 1 },
      hp: 10,
      isDead: false,
      statuses: [],
    },
    entities: [],
  };
  const after = {
    ...before,
    player: { ...before.player, position: { x: 2, y: 1 } },
  };
  const destination = { x: 4, y: 1 };

  assert.equal(localTravelStopsAfterStep(before, after, destination), false);
  assert.equal(
    localTravelStopsAfterStep(
      before,
      { ...after, player: { ...after.player, hp: 9 } },
      destination,
    ),
    true,
  );
  assert.equal(
    localTravelStopsAfterStep(
      before,
      { ...after, entities: [{ faction: "hostile" }] },
      destination,
    ),
    true,
  );
  assert.equal(localTravelStopsAfterStep(before, before, destination), true);
});

test("local travel destinations follow wilderness map translations", () => {
  assert.deepEqual(translatedLocalPosition({ x: 80, y: 25 }, { x: -32, y: -11 }), {
    x: 48,
    y: 14,
  });
});

test("look and targeting cursors follow wilderness map translations", () => {
  const state = new AppState();
  const focused = [];
  const controller = new InputController({
    state,
    dom: {},
    localization: {},
    window: {},
    getInputPreset: () => "vi",
    getZoom: () => 1,
    dispatch: async () => {},
    describeLook: () => "",
    openObjectList: () => {},
    openMogaminator: () => {},
    onLookOrTargeting: () => {},
    onLookFocusChange: (position) => focused.push(position),
    announce: () => {},
  });
  const targetSpec = {
    modes: ["position"],
    range: 80,
    requiresLineOfEffect: false,
  };
  const update = {
    mapScale: "local",
    mapTranslation: { x: -32, y: 0 },
    width: 96,
    height: 33,
    floorId: "core.floor.wilderness",
    worldTravelDestination: null,
    player: { position: { x: 32, y: 16 }, projectileProfile: { targetSpec } },
  };

  for (const intent of [{ type: "look" }, { type: "projectile" }]) {
    state.targeting = {
      origin: { x: 64, y: 16 },
      cursor: { x: 70, y: 20 },
      spec: targetSpec,
    };
    state.targetingIntent = intent;
    controller.reconcileStatus(update);
    assert.deepEqual(state.targeting?.origin, { x: 32, y: 16 });
    assert.deepEqual(state.targeting?.cursor, { x: 38, y: 20 });
  }
  assert.deepEqual(focused, [{ x: 38, y: 20 }]);
});

test("auto-get locks one target, then requests the next Core target", async () => {
  const state = new AppState();
  state.mode = "playing";
  const item = (id, x) => ({ id, position: { x, y: 1 } });
  const status = (x, target, items) => ({
    mapScale: "local",
    floorId: "floor.1",
    player: {
      position: { x, y: 1 },
      hp: 10,
      isDead: false,
      statuses: [],
      inventoryUsedSlots: 0,
      inventorySlotCapacity: 10,
    },
    entities: [],
    items,
    goldPiles: [],
    mogaminator: { autoGetTarget: target },
  });
  const alpha = { objectId: "alpha", position: { x: 3, y: 1 } };
  const beta = { objectId: "beta", position: { x: 3, y: 1 } };
  state.status = status(1, alpha, [item("alpha", 3), item("beta", 3)]);
  const updates = [
    status(1, alpha, [item("alpha", 3), item("beta", 3)]),
    status(2, beta, [item("alpha", 3), item("beta", 3)]),
    status(3, beta, [item("beta", 3)]),
    status(3, undefined, []),
  ];
  const commands = [];
  const controller = new InputController({
    state,
    dom: {},
    localization: {},
    window: {},
    getInputPreset: () => "vi",
    getZoom: () => 1,
    dispatch: async (command) => {
      commands.push(command);
      state.status = updates.shift();
    },
    describeLook: () => "",
    openObjectList: () => {},
    openMogaminator: () => {},
    onLookOrTargeting: () => {},
    onLookFocusChange: () => {},
    announce: () => {},
  });

  await controller.autoGet();

  assert.deepEqual(commands, [
    { type: "pick-up" },
    { type: "auto-get", objectId: "alpha" },
    { type: "auto-get", objectId: "alpha" },
    { type: "auto-get", objectId: "beta" },
  ]);

  state.status = { ...status(1, alpha, [item("alpha", 3)]), mapScale: "world" };
  await controller.autoGet();
  assert.equal(commands.length, 4);
});

test("auto-get stops on every authoritative interruption", () => {
  const target = { objectId: "alpha", position: { x: 3, y: 1 } };
  const before = {
    mapScale: "local",
    floorId: "floor.1",
    player: {
      position: { x: 1, y: 1 },
      hp: 10,
      isDead: false,
      statuses: [],
      inventoryUsedSlots: 0,
      inventorySlotCapacity: 10,
    },
    entities: [],
    items: [{ id: "alpha" }],
    goldPiles: [],
    mogaminator: {},
  };
  const moved = {
    ...before,
    player: { ...before.player, position: { x: 2, y: 1 } },
  };

  assert.equal(autoGetStopsAfterStep(before, moved, target), false);
  assert.equal(
    autoGetStopsAfterStep(
      before,
      { ...moved, player: { ...moved.player, hp: 9 } },
      target,
    ),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(
      before,
      {
        ...moved,
        player: {
          ...moved.player,
          statuses: [{ kindId: "rfb.status.confusion" }],
        },
      },
      target,
    ),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(
      before,
      { ...moved, player: { ...moved.player, isDead: true } },
      target,
    ),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(before, { ...moved, entities: [{ faction: "hostile" }] }, target),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(
      before,
      {
        ...moved,
        player: {
          ...moved.player,
          statuses: [{ kindId: "rfb.status.blindness" }],
        },
      },
      target,
    ),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(
      before,
      {
        ...moved,
        player: { ...moved.player, inventoryUsedSlots: 10 },
      },
      target,
    ),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(
      before,
      { ...moved, mogaminator: { pendingQuery: { itemId: "alpha" } } },
      target,
    ),
    true,
  );
  assert.equal(autoGetStopsAfterStep(before, before, target), true);
  assert.equal(
    autoGetStopsAfterStep(before, { ...moved, floorId: "floor.2" }, target),
    true,
  );
  assert.equal(
    autoGetStopsAfterStep(before, { ...moved, mapScale: "world" }, target),
    true,
  );
  assert.equal(autoGetStopsAfterStep(before, undefined, target), true);
  assert.equal(
    autoGetStopsAfterStep(before, { ...before, items: [] }, target),
    false,
  );
});
