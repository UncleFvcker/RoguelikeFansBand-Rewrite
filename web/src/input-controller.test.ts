// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";
import { commandShortcut } from "./command-shortcuts.ts";

import {
  InputController,
  autoGetStopsAfterStep,
  commandForKeyboardInput,
  connectionActionForState,
  directionForKeyboardInput,
  runDirectionForKeyboardInput,
  alterDirectionForKeyboardInput,
  isAutoGetShortcut,
  isObjectListShortcut,
  localTravelStopsAfterStep,
  nextTravelConnectionPosition,
  translatedLocalPosition,
} from "./input-controller.ts";
import { AppState } from "./app-state.ts";
import { GameSession } from "./game-session.ts";
import { defaultPreferences } from "./preferences.ts";

const flushCommands = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };

test("focused buttons keep native Enter and Space before game shortcuts", t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike", "numpad", "vi", "wasd"]) {
    const h = continuousHarness("local", undefined, preset);
    for (const key of ["Enter", " "]) {
      const event = h.emit("keydown", { key, target: new HTMLButtonElement() });
      assert.equal(event.defaultPrevented, false);
      assert.equal(event.stopped, false);
    }
    assert.deepEqual(h.shortcuts, []);
    assert.deepEqual(h.requests, []);
    h.emit("keydown", { key: "Enter" });
    assert.deepEqual(h.shortcuts, ["command-menu"], "map Enter still opens commands");
    h.controller.dispose();
  }
});

test("r opens recall at the look cursor without changing targeting or dispatching a command", t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const positions = [];
    const h = continuousHarness("local", undefined, preset, { openMonsterRecall: p => positions.push({ ...p }) });
    h.state.setMapSize(10, 10);
    h.emit("keydown", { key: preset === "original" ? "l" : "x" });
    assert.equal(h.state.targetingIntent?.type, "look");
    h.emit("keydown", { key: "6" });
    const aim = structuredClone(h.state.targeting);
    assert.equal(h.emit("keydown", { key: "r" }).defaultPrevented, true);
    assert.deepEqual(positions, [aim.cursor]);
    assert.deepEqual(h.state.targeting, aim);
    assert.equal(h.requests.length, 0);
    for (const modifier of ["repeat", "ctrlKey", "altKey", "metaKey", "isComposing"]) h.emit("keydown", { key: "r", [modifier]: true });
    h.document.querySelector = () => ({});
    h.emit("keydown", { key: "r" });
    assert.equal(positions.length, 1, "modifiers, repeats and open dialogs do not reopen recall");
    h.controller.dispose();
  }
});

test("held movement uses OS repeats without queueing commands after keyup", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    for (const scale of ["local", "world"]) {
      for (const key of [{ key: "ArrowRight", code: "ArrowRight" },
        preset === "original" ? { key: "6", code: "Numpad6" } : { key: "l", code: "KeyL" }]) {
        const h = continuousHarness(scale, undefined, preset);
        h.emit("keydown", { ...key, repeat: true });
        assert.equal(h.requests.length, 0, "a repeat without an accepted press cannot start movement");
        h.emit("keydown", key);
        assert.ok(h.requests[0], `${preset}/${scale}/${key.key} must dispatch its first movement`);
        assert.deepEqual(h.requests[0].command, { type: "move", direction: "east" });
        for (let i = 0; i < 3; i++) {
          assert.equal(h.emit("keydown", { ...key, repeat: true }).defaultPrevented, true);
        }
        assert.equal(h.requests.length, 1, "busy repeats are discarded");
        await h.finish();
        assert.equal(h.requests.length, 1, "finishing a command does not drain a movement queue");
        h.emit("keydown", { ...key, repeat: true });
        assert.deepEqual(h.requests[1].command, { type: "move", direction: "east" });
        h.emit("keyup", key);
        await h.finish();
        h.emit("keydown", { ...key, repeat: true });
        assert.equal(h.requests.length, 2, "keyup stops future movement even with an in-flight command");
        assert.equal(h.timers.size, 0);
        h.controller.dispose();
      }
    }
  }
});

test("held movement is cleared by focus, UI, session and command interruptions", async t => {
  installElementIdentities(t);
  for (const interrupt of ["blur", "hidden", "dialog", "input", "composition", "target", "query", "death", "modifier", "click", "reset", "shortcut"]) {
    const h = continuousHarness("local", undefined, "original");
    const key = { key: "6", code: "Numpad6" };
    h.emit("keydown", key); await h.finish();
    if (interrupt === "blur") h.emit("blur");
    if (interrupt === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); h.document.hidden = false; }
    if (interrupt === "dialog") {
      h.document.querySelector = () => ({}); h.emit("keydown", { ...key, repeat: true });
      h.document.querySelector = () => null;
    }
    if (interrupt === "input") h.emit("keydown", { ...key, repeat: true, target: new HTMLInputElement() });
    if (interrupt === "composition") h.emit("keydown", { ...key, repeat: true, isComposing: true });
    if (interrupt === "target") {
      h.state.targeting = {}; h.emit("keydown", { ...key, repeat: true }); h.state.targeting = undefined;
    }
    if (interrupt === "query") {
      h.state.status.mogaminator.pendingQuery = {}; h.emit("keydown", { ...key, repeat: true });
      delete h.state.status.mogaminator.pendingQuery;
    }
    if (interrupt === "death") {
      h.state.playerDead = true; h.emit("keydown", { ...key, repeat: true }); h.state.playerDead = false;
    }
    if (interrupt === "modifier") h.emit("keydown", { ...key, repeat: true, shiftKey: true });
    if (interrupt === "click") h.emit("click");
    if (interrupt === "reset") h.controller.resetSession();
    if (interrupt === "shortcut") h.emit("keydown", { key: "i" });
    h.emit("keydown", { ...key, repeat: true });
    assert.equal(h.requests.length, 1, interrupt);
    h.emit("keydown", key); await h.finish();
    assert.equal(h.requests.length, 2, "a fresh press resumes movement after " + interrupt);
    h.controller.dispose();
  }
});

test("held custom movement repeats its resolved direction without re-expanding bindings", async t => {
  installElementIdentities(t);
  const previous = Object.getOwnPropertyDescriptor(globalThis, "KeyboardEvent");
  Object.defineProperty(globalThis, "KeyboardEvent", { configurable: true, value: class extends Event {
    constructor(type, options) { super(type, { cancelable: true }); Object.assign(this, options); }
  } });
  t.after(() => previous ? Object.defineProperty(globalThis, "KeyboardEvent", previous) : delete globalThis.KeyboardEvent);
  for (const preset of ["original", "roguelike"]) {
    let expansions = 0;
    const h = continuousHarness("local", undefined, preset, {
      customKey(event, execute) {
        if (event.repeat || !["j", "6"].includes(event.key)) return false;
        expansions++; execute({ key: event.key === "j" ? "6" : "i" }); return true;
      },
    });
    h.emit("keydown", { key: "j", code: "KeyJ" }); await h.finish();
    h.emit("keydown", { key: "j", code: "KeyJ", repeat: true }); await h.finish();
    assert.deepEqual(h.requests.map(request => request.command), [
      { type: "move", direction: "east" }, { type: "move", direction: "east" },
    ]);
    assert.equal(expansions, 1);
    h.emit("keyup", { key: "j", code: "KeyJ" });
    h.emit("keydown", { key: "6", code: "Numpad6" });
    h.emit("keydown", { key: "6", code: "Numpad6", repeat: true });
    assert.deepEqual(h.shortcuts, ["inventory"]);
    assert.equal(h.requests.length, 2, "a remapped menu key must not revert to its native movement");
    assert.equal(expansions, 2);
    h.controller.dispose();
  }
});

test("auto-repeat off issues one attempt while explicit counts still repeat", async t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  h.state.status.operationOptions.autoRepeat = false;
  const command = { type: "open-door", direction: "east" };
  const single = h.controller.dispatchCounted(command); await flushCommands();
  await h.finish(update => { update.commandRepeatable = true; }); await single;
  assert.equal(h.requests.length, 1); assert.equal(h.timers.size, 0);
  const counted = h.controller.dispatchCounted(command, 2); await flushCommands();
  await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
  await h.finish(update => { update.commandRepeatable = false; }); await h.tick(); await counted;
  assert.equal(h.requests.length, 3); assert.equal(h.timers.size, 0);
  h.controller.dispose();
});

test("custom bindings expand once, preserve selection contexts and allow literal bypass in every preset", async t => {
  installElementIdentities(t);
  const previous = Object.getOwnPropertyDescriptor(globalThis, "KeyboardEvent");
  Object.defineProperty(globalThis, "KeyboardEvent", { configurable: true, value: class extends Event {
    constructor(type, options) { super(type, { cancelable: true }); Object.assign(this, options); }
  } });
  t.after(() => previous ? Object.defineProperty(globalThis, "KeyboardEvent", previous) : delete globalThis.KeyboardEvent);
  for (const preset of ["original", "roguelike"]) {
    let expansions = 0;
    const h = continuousHarness("local", undefined, preset, {
      customKey(event, execute) { if (!["s", "i"].includes(event.key)) return false; expansions++; execute({ key: "i" }); return true; },
      hasCustomKey: event => event.key === "F2",
    });
    h.emit("keydown", { key: "s" }); assert.deepEqual(h.shortcuts, ["inventory"]); assert.equal(expansions, 1);
    h.shortcuts.length = 0;
    h.emit("keydown", { key: "\\" }); h.emit("keydown", { key: "s" }); await h.finish();
    assert.deepEqual(h.requests[0].command, { type: "search" }); assert.deepEqual(h.shortcuts, []);
    h.state.terrainInteractionMode = "open-door";
    h.emit("keydown", { key: "s" }); assert.equal(expansions, 1);
    h.state.terrainInteractionMode = undefined;
    h.start(); await flushCommands();
    assert.equal(h.emit("keydown", { key: "F2" }).stopped, true); await h.finish(); await h.controller.stopContinuousAction();
    assert.equal(h.requests.length, 2); h.controller.dispose();
  }
});

test("command macros serialize heterogeneous actions and stop on cancellation, damage, prompts or reset", async t => {
  installElementIdentities(t);
  const steps = [{ type: "stay" }, { type: "toggle-search" }, { type: "stay" }];
  const h = continuousHarness("local", undefined, "original");
  const play = h.controller.playMacro(steps); await flushCommands();
  for (let i = 0; i < steps.length; i++) {
    assert.deepEqual(h.requests[i].command, steps[i]);
    await h.finish(update => { update.commandRepeatable = false; }); await h.tick();
  }
  await play; assert.equal(h.controller.continuousAction, undefined); h.controller.dispose();
  for (const trigger of ["escape", "damage", "prompt", "floor", "reset", "decline"]) {
    const h = continuousHarness("local", undefined, "original");
    h.window.confirm = () => trigger !== "decline";
    const play = h.controller.playMacro(steps); await flushCommands();
    if (trigger === "decline") { await play; assert.equal(h.requests.length, 0); h.controller.dispose(); continue; }
    if (trigger === "escape") h.emit("keydown", { key: "Escape" });
    if (trigger === "reset") h.controller.resetSession();
    await h.finish(update => {
      if (trigger === "damage") update.player.hp--;
      if (trigger === "floor") update.floorId = "other";
      if (trigger === "prompt") update.mogaminator.pendingQuery = {};
    });
    if (h.timers.size) await h.tick();
    await play; assert.equal(h.requests.length, 1, trigger); h.controller.dispose();
  }
});

test("help and knowledge work across presets and world maps without game commands", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    for (const scale of ["local", "world"]) {
      const h = continuousHarness(scale, undefined, preset);
      for (const key of ["?", "~"]) {
        for (const flags of [{ ctrlKey: true }, { altKey: true }, { metaKey: true }, { repeat: true }, { isComposing: true }, { target: new HTMLInputElement() }]) h.emit("keydown", { key, ...flags });
        h.state.busy = true; h.emit("keydown", { key }); h.state.busy = false;
        h.document.querySelector = () => ({}); h.emit("keydown", { key }); h.document.querySelector = () => null;
      }
      assert.deepEqual(h.shortcuts, []);
      h.emit("keydown", { key: "?", shiftKey: true }); h.emit("keydown", { key: "~", shiftKey: true });
      assert.deepEqual(h.shortcuts, ["help", "knowledge"], preset + scale);
      assert.equal(h.requests.length, 0);
      h.controller.dispose();
    }
  }
});

test("help and knowledge first stop travel, and leave direction or targeting prompts intact", async t => {
  installElementIdentities(t);
  for (const key of ["?", "~"]) {
    const h = continuousHarness("local", undefined, "original");
    h.start(); await flushCommands();
    assert.equal(h.emit("keydown", { key }).stopped, true);
    assert.deepEqual(h.shortcuts, []);
    await h.finish(); await h.controller.stopContinuousAction(); assert.equal(h.timers.size, 0);
    assert.equal(h.requests.length, 1);
    h.emit("keydown", { key }); assert.equal(h.shortcuts.length, 1);
    h.shortcuts.length = 0;
    h.state.status.terrainInteractions = [{ kind: "open-door", direction: "east", available: true }];
    h.emit("keydown", { key: "o" }); h.emit("keydown", { key });
    assert.equal(h.state.terrainInteractionMode, "open-door"); assert.deepEqual(h.shortcuts, []);
    h.emit("keydown", { key: "Escape" }); h.emit("keydown", { key: "*" }); h.emit("keydown", { key });
    assert.ok(h.state.targeting); assert.deepEqual(h.shortcuts, []);
    h.controller.dispose();
  }
});

test("global targeting cycles without firing and reuses a moving entity for each aim entry", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    for (const intent of [{ type: "projectile" }, { type: "ability", abilityId: "spell" }, { type: "item", itemId: "wand" }, { type: "absorbed-device", itemId: "absorbed" }, { type: "throw", itemId: "stone" }]) {
      const h = continuousHarness("local", undefined, preset);
      h.state.status.width = h.state.mapWidth = 20; h.state.status.height = h.state.mapHeight = 20;
      h.state.status.entities = [{ id: "near", faction: "hostile", position: { x: 2, y: 1 } }, { id: "far", faction: "hostile", position: { x: 4, y: 1 } }];
      h.emit("keydown", { key: "*" });
      assert.equal(h.state.targetingIntent.type, "select-target");
      assert.deepEqual(h.state.targeting.cursor, { x: 2, y: 1 });
      for (const key of [" ", "*", "+"]) h.emit("keydown", { key });
      assert.deepEqual(h.state.targeting.cursor, { x: 4, y: 1 });
      h.emit("keydown", { key: "-" });
      h.emit("keydown", { key: "t" });
      assert.equal(h.requests.length, 0, "selecting never executes a game action");
      assert.equal(h.state.targeting, undefined);
      assert.deepEqual(h.controller.selectedTarget, { type: "entity", entityId: "near" }, "the HUD target survives closing target selection");
      h.state.status.entities[0].position = { x: 3, y: 2 };
      h.controller.startTargetingWithSpec({ modes: intent.type === "throw" ? ["direction"] : ["entity", "position"], range: 8, requiresLineOfEffect: true }, intent);
      h.emit("keydown", { key: "5" }); await flushCommands();
      assert.equal(h.requests.length, 1);
      const command = h.requests[0].command;
      if (intent.type === "throw") assert.deepEqual(command, { type: "throw", itemId: "stone", direction: "south-east" });
      else assert.deepEqual(command.target ?? command.targets[0], { type: "entity", entityId: "near" });
      await h.finish(); h.controller.dispose();
    }
  }
});

test("grid and direction attacks remember the monster; its death falls back to the nearest enemy", async t => {
  installElementIdentities(t);
  for (const modes of [["entity", "position"], ["position"], ["direction"]]) {
    const h = continuousHarness("local", undefined, "original");
    h.state.status.width = h.state.mapWidth = 20; h.state.status.height = h.state.mapHeight = 20;
    const near = { id: "near", faction: "hostile", position: { x: 2, y: 1 } };
    const old = { id: "old", faction: "hostile", position: { x: 4, y: 1 } };
    h.state.status.entities = [near, old];
    const spec = { modes, range: 8, requiresLineOfEffect: true };
    const intent = modes[0] === "direction" ? { type: "throw", itemId: "stone" } : { type: "ability", abilityId: "spell" };
    h.controller.startTargetingWithSpec(spec, intent);
    assert.deepEqual(h.state.targeting.cursor, near.position);
    h.state.targeting = { ...h.state.targeting, cursor: old.position, list: false };
    h.emit("keydown", { key: "Enter" }); await flushCommands();
    assert.deepEqual(h.controller.selectedTarget, { type: "entity", entityId: "old" });
    await h.finish(update => { update.player.position = { x: 1, y: 1 }; });
    old.position = { x: 5, y: 1 };
    h.state.status.entities.find(entity => entity.id === old.id).position = old.position;
    h.controller.startTargetingWithSpec(spec, intent);
    assert.deepEqual(h.state.targeting.cursor, old.position, "living old monster wins over a nearer enemy");
    h.controller.cancelTargeting(false);
    h.controller.startTargetSelection();
    assert.deepEqual(h.state.targeting.cursor, old.position, "opening global selection retains the old monster too");
    h.controller.cancelTargeting(false);
    h.state.status.entities = [near];
    h.controller.reconcileStatus(h.state.status);
    h.controller.startTargetingWithSpec(spec, intent);
    assert.deepEqual(h.state.targeting.cursor, near.position, "death releases the original square");
    h.emit("keydown", { key: "Enter" }); await flushCommands();
    assert.deepEqual(h.controller.selectedTarget, { type: "entity", entityId: "near" });
    await h.finish(update => { update.player.position = { x: 1, y: 1 }; });
    h.state.status.entities = [];
    h.controller.reconcileStatus(h.state.status);
    h.controller.startTargetingWithSpec(spec, intent);
    assert.deepEqual(h.state.targeting.cursor, h.state.status.player.position);
    h.emit("keydown", { key: "t" }); await flushCommands();
    assert.equal(h.requests.length, 2, "no enemy does not fire at a dead target's square");
    h.controller.dispose();
  }
});

test("target cancellation, invalid old targets, floor changes and reset cannot fire stale selections", async t => {
  installElementIdentities(t);
  const spec = { modes: ["entity", "position"], range: 5, requiresLineOfEffect: true };
  for (const invalidate of ["vanished", "range", "floor", "reset"]) {
    const h = continuousHarness("local", undefined, "original");
    h.state.status.operationOptions.defaultTarget = "manual";
    h.state.status.width = h.state.mapWidth = 20; h.state.status.height = h.state.mapHeight = 20;
    h.state.status.entities = [{ id: "enemy", faction: "hostile", position: { x: 2, y: 1 } }];
    h.emit("keydown", { key: "*" }); h.emit("keydown", { key: "0" });
    if (invalidate === "vanished") h.state.status.entities = [];
    if (invalidate === "range") h.state.status.entities[0].position.x = 15;
    if (invalidate === "floor") h.state.status.floorId = "another";
    if (invalidate === "reset") h.controller.resetSession();
    h.controller.reconcileStatus(h.state.status);
    h.controller.startTargetingWithSpec(spec, { type: "projectile" });
    h.emit("keydown", { key: "T" }); await flushCommands();
    assert.equal(h.requests.length, 0, invalidate);
    assert.ok(h.messages.includes("message-target-selection-invalid"));
    h.emit("keydown", { key: "q" });
    assert.equal(h.state.targeting, undefined);
    h.controller.dispose();
  }
});

test("free target positions translate across scrolling and all confirmation keys select without attacking", async t => {
  installElementIdentities(t);
  for (const key of ["Enter", "t", "T", ".", "5", "0"]) {
    const h = continuousHarness("local", undefined, "roguelike");
    h.state.status.width = h.state.mapWidth = 20; h.state.status.height = h.state.mapHeight = 20;
    h.emit("keydown", { key: "*" }); h.emit("keydown", { key: "o" });
    h.emit("keydown", { key: "l" }); h.emit("keydown", { key });
    assert.equal(h.state.targeting, undefined); assert.equal(h.requests.length, 0);
    h.controller.reconcileStatus({ ...h.state.status, mapTranslation: { x: 2, y: 0 } });
    h.state.status.entities = [{ id: "new-occupant", position: { x: 4, y: 1 } }];
    h.controller.startTargetingWithSpec({ modes: ["position", "entity"], range: 8, requiresLineOfEffect: true }, { type: "projectile" });
    h.emit("keydown", { key: "t" }); await flushCommands();
    assert.deepEqual(h.requests[0].command.target, { type: "position", position: { x: 4, y: 1 } });
    await h.finish(); h.controller.dispose();
  }
});

test("world-map star looks, continuous star stops first, and q retains paid cancellation", async t => {
  installElementIdentities(t);
  const h = continuousHarness("world", undefined, "roguelike");
  h.state.status.width = h.state.mapWidth = 20; h.state.status.height = h.state.mapHeight = 20;
  h.emit("keydown", { key: "*" }); assert.equal(h.state.targetingIntent.type, "look");
  h.emit("keydown", { key: "q" });
  h.start(); await flushCommands(); h.emit("keydown", { key: "*" });
  assert.equal(h.state.targeting, undefined);
  const stopped = h.controller.stopContinuousAction(); await h.finish(); await stopped;
  h.state.status.mapScale = "local";
  h.controller.startTargetingWithSpec({ modes: ["position"], range: 8, requiresLineOfEffect: true }, { type: "absorbed-device", itemId: "device" });
  h.emit("keydown", { key: "q" }); await flushCommands();
  assert.deepEqual(h.requests.at(-1).command, { type: "use-absorbed-device", itemId: "device", targets: [] });
  await h.finish(); h.controller.dispose();
});

test("world map entry takes pets along and only confirms active recall cancellation", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    h.state.worldId = "demo.world.middle-earth";
    h.state.status.floorId = "core.floor.wilderness";
    h.state.status.player.id = "player";
    h.state.status.entities = [{ id: "pet", controllerId: "player", faction: "pet" }];
    h.window.confirm = () => { throw new Error("pets do not require abandonment confirmation"); };
    h.emit("keydown", { key: "<" }); await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "enter-world-map", cancelRecall: false });
    await h.finish();
    h.state.status.player.recall = { remainingTurns: 10 };
    const prompts = [];
    h.window.confirm = message => { prompts.push(message); return false; };
    h.emit("keydown", { key: "<" }); await flushCommands();
    assert.equal(h.requests.length, 1);
    h.window.confirm = message => { prompts.push(message); return true; };
    h.emit("keydown", { key: "<" }); await flushCommands();
    assert.deepEqual(h.requests[1].command, { type: "enter-world-map", cancelRecall: true });
    assert.deepEqual(prompts, ["confirm-world-map-cancel-recall", "confirm-world-map-cancel-recall"]);
    await h.finish();
    assert.deepEqual(h.errors, []);
    h.controller.dispose();
  }
});

test("R selects fixed, resource-only or complete rest and n retains the selection", async t => {
  installElementIdentities(t);
  for (const [input, type, turns] of [["3", "rest-for-turns", 3], [" * ", "rest-until-resources", 9999],
    ["&", "rest", 9999], ["10000", "rest-for-turns", 9999]]) {
    for (const preset of ["original", "roguelike"]) {
      const h = continuousHarness("local", undefined, preset);
      h.emit("keydown", { key: "R" }); await flushCommands();
      assert.equal(h.requests.length, 0, "opening the rest dialog does not advance time");
      const dialog = h.document.body.children[0];
      assert.equal(dialog.children[0].children[0].textContent, "message-rest-mode-prompt");
      assert.equal(dialog.children[0].children[0].children[0].value, "&");
      h.confirmRest(input); await flushCommands();
      assert.deepEqual(h.requests[0].command, { type, turns: 1 });
      const stop = h.controller.stopContinuousAction(); await h.finish(); await stop;
      assert.deepEqual(h.session.lastCommand, { type, turns });
      h.emit("keydown", { key: preset === "original" ? "n" : "X" }); await flushCommands();
      assert.equal(h.document.body.children.length, 0, "repeating rest does not ask again");
      assert.deepEqual(h.requests[1].command, { type, turns: 1 });
      const repeatStop = h.controller.stopContinuousAction(); await h.finish(); await repeatStop;
      h.controller.dispose();
    }
  }
});

test("rest input cancellation, invalid values and counts never leak into another command", async t => {
  installElementIdentities(t);
  for (const input of [null, "", "0", "000", "-2", "2.5", "3x", "**"]) {
    const h = continuousHarness("local", undefined, "original");
    h.emit("keydown", { key: "R" });
    h.confirmRest(input); await flushCommands();
    assert.equal(h.requests.length, 0);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    if (input && !/^0+$/.test(input)) assert.ok(h.messages.includes("message-rest-mode-invalid"));
    h.emit("keydown", { key: "6" }); await h.finish();
    assert.deepEqual(h.requests[0].command, { type: "move", direction: "east" });
    h.controller.dispose();
  }
  const h = continuousHarness("local", undefined, "original");
  for (const key of ["0", "3", "R"]) h.emit("keydown", { key });
  await flushCommands();
  assert.equal(h.document.body.children.length, 0, "an explicit prefix does not ask for a mode");
  for (let i = 0; i < 3; i++) { await h.finish(); await h.tick(); }
  assert.equal(h.requests.length, 3);
  assert.equal(h.controller.continuousAction, undefined);
  h.emit("keydown", { key: "R" });
  h.confirmRest("*");
  h.controller.resetSession(); await flushCommands();
  assert.equal(h.requests.length, 3, "a replaced session cannot start an old prompt result");
  h.controller.dispose();
});

test("terrain commands automatically retry the chosen direction until Core stops them", async t => {
  installElementIdentities(t);
  for (const [key, type] of [["o", "open-door"], ["B", "bash-door"], ["T", "dig-terrain"],
    ["D", "disarm-trap"], ["+", "alter"], ["c", "close-door"]]) {
    const h = continuousHarness("local", undefined, "original");
    h.state.status.terrainInteractions = [{ kind: type, direction: "east", available: true }];
    h.emit("keydown", { key }); h.emit("keydown", { key: "6" }); await flushCommands();
    assert.deepEqual(h.requests[0].command, { type, direction: "east" });
    assert.equal(h.state.terrainInteractionMode, undefined);
    await h.finish(update => { update.commandRepeatable = type !== "close-door"; }); await h.tick();
    if (type !== "close-door") {
      assert.deepEqual(h.requests[1].command, h.requests[0].command);
      await h.finish(update => { update.commandRepeatable = false; }); await h.tick();
    }
    assert.equal(h.controller.continuousAction, undefined, type);
    assert.equal(h.requests.length, type === "close-door" ? 1 : 2);
    // RFB n expands after the automatic budget is assigned: it retries once.
    h.emit("keydown", { key: "n" }); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, h.requests[0].command);
    const repeatCount = h.requests.length;
    await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    assert.equal(h.requests.length, repeatCount);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  }
});

test("automatic chest attempts retain identity, exhaust 99 attempts, and honor an explicit one", async t => {
  installElementIdentities(t);
  for (const type of ["open-chest", "disarm-chest"]) {
    const h = continuousHarness("local", undefined, "original");
    const command = { type, itemId: "selected-chest" };
    const action = h.controller.dispatchCounted(command); await flushCommands();
    for (let i = 0; i < 99; i++) {
      assert.deepEqual(h.requests.at(-1).command, command);
      await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    }
    await action;
    assert.equal(h.requests.length, 99);
    assert.equal(h.controller.continuousAction, undefined);
    const once = h.controller.dispatchCounted(command, 1); await flushCommands();
    await h.finish(update => { update.commandRepeatable = true; }); await h.tick(); await once;
    assert.equal(h.requests.length, 100, "explicit count overrides the automatic budget");
    assert.equal(h.timers.size, 0);
    h.controller.dispose();
  }
});

test("Original n and Roguelike X repeat resolved choices, including a new count prefix", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    const key = preset === "original" ? "n" : "X";
    h.emit("keydown", { key });
    assert.deepEqual(h.messages, ["message-repeat-last-empty"]);
    const selected = { type: "cast-ability", abilityId: "spell-a", target: { type: "direction", direction: "north-west" } };
    const initial = h.session.dispatch(selected); await h.finish(); await initial;
    h.emit("keydown", { key, shiftKey: preset === "roguelike" }); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, selected);
    await h.finish();
    // A repeated command remains repeatable; holding the key is never an action queue.
    h.emit("keydown", { key, repeat: true });
    assert.equal(h.requests.length, 2);
    h.emit("keydown", { key }); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, selected);
    await h.finish();
    const search = h.session.dispatch({ type: "search" }); await h.finish(); await search;
    for (const key of ["0", "3", preset === "original" ? "n" : "X"]) h.emit("keydown", { key });
    await flushCommands();
    const start = h.requests.length;
    for (let i = 0; i < 3; i++) {
      assert.deepEqual(h.requests.at(-1).command, { type: "search" });
      await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    }
    assert.equal(h.requests.length, start + 2);
    assert.equal(h.controller.continuousAction, undefined);
    if (preset === "roguelike") {
      h.emit("keydown", { key: "n" }); await flushCommands();
      assert.deepEqual(h.requests.at(-1).command, { type: "move", direction: "south-east" });
      await h.finish();
      h.emit("keydown", { key: "x", ctrlKey: true });
      assert.equal(h.shortcuts.at(-1), "save-exit");
      h.emit("keydown", { key: "\\" }); h.emit("keydown", { key: "n" }); await flushCommands();
      assert.deepEqual(h.requests.at(-1).command, { type: "move", direction: "south-east" });
      await h.finish();
    }
    h.controller.dispose();
  }
});

test("repeat keeps item IDs and quantity, respects confirmation and input cancellation", async t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  const selected = { type: "destroy-item", itemId: "item-chosen", quantity: 2 };
  const initial = h.session.dispatch(selected); await h.finish(); await initial;
  const confirmations = [];
  h.window.confirm = command => { confirmations.push(command); return false; };
  h.emit("keydown", { key: "n" }); await flushCommands();
  assert.deepEqual(confirmations, [selected]);
  assert.equal(h.requests.length, 1);
  h.window.confirm = () => true;
  for (const context of ["dialog", "input", "ime"]) {
    h.document.querySelector = () => context === "dialog" ? {} : null;
    h.emit("keydown", { key: "n", isComposing: context === "ime", target: context === "input" ? new HTMLInputElement() : null });
    assert.equal(h.requests.length, 1);
  }
  h.document.querySelector = () => null;
  h.emit("keydown", { key: "n" }); await flushCommands();
  assert.deepEqual(h.requests.at(-1).command, selected);
  h.requests.at(-1).reject(new Error("chosen item is gone")); await flushCommands();
  assert.equal(h.errors.length, 1);
  h.emit("keydown", { key: "n" });
  assert.equal(h.requests.length, 2, "never substitutes another item or an older command");
  h.start(); await flushCommands();
  const started = h.requests.length;
  assert.equal(h.emit("keydown", { key: "n" }).stopped, true);
  await h.finish(); await h.controller.stopContinuousAction();
  assert.equal(h.requests.length, started, "repeat key stops automation without executing twice");
  h.controller.dispose();
});

test("repeat rest and run restart the complete action rather than its final internal step", async t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  const rest = h.controller.restUntilRecovered(2); await flushCommands();
  for (let i = 0; i < 2; i++) {
    await h.finish(update => { update.events = [{ outcome: { type: "rest", resolution: { completedTurns: 1, stopReason: "turn-limit" } } }]; });
    await h.tick();
  }
  await rest;
  assert.deepEqual(h.session.lastCommand, { type: "rest-for-turns", turns: 2 });
  h.emit("keydown", { key: "n" }); await flushCommands();
  assert.equal(h.controller.continuousAction, "rest");
  assert.deepEqual(h.requests.at(-1).command, { type: "rest-for-turns", turns: 1 });
  const stopped = h.controller.stopContinuousAction(); await h.finish(); await stopped;
  const run = h.controller.run("east", 3); await flushCommands();
  await h.finish(update => { update.player.running = {}; }); await h.tick();
  assert.equal(h.requests.at(-1).command.type, "continue-run");
  await h.finish(update => { delete update.player.running; }); await h.tick(); await run;
  assert.deepEqual(h.session.lastCommand, { type: "run", direction: "east", maxSteps: 3 });
  h.emit("keydown", { key: "n" }); await flushCommands();
  assert.deepEqual(h.requests.at(-1).command, { type: "run", direction: "east", maxSteps: 3 });
  const stopRun = h.controller.stopContinuousAction(); await h.finish(); await stopRun;
  const explore = h.controller.autoExplore(); await flushCommands();
  await h.finish(update => { update.player.autoExplore = {}; }); await h.tick();
  assert.equal(h.requests.at(-1).command.type, "continue-auto-explore");
  await h.finish(update => { delete update.player.autoExplore; }); await h.tick(); await explore;
  h.emit("keydown", { key: "n" }); await flushCommands();
  assert.deepEqual(h.requests.at(-1).command, { type: "auto-explore" });
  const stopExplore = h.controller.stopContinuousAction(); await h.finish(); await stopExplore;
  h.controller.dispose();
});

test("special walk selects direction once, preserves counts and repeat parameters in both keymaps", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    for (const key of ["0", "2", "-", preset === "original" ? "6" : "l"]) h.emit("keydown", { key });
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "walk-special", direction: "east" });
    await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    assert.deepEqual(h.requests[1].command, h.requests[0].command);
    await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    assert.equal(h.requests.length, 2);
    h.emit("keydown", { key: preset === "original" ? "n" : "X" }); await flushCommands();
    assert.deepEqual(h.requests[2].command, h.requests[0].command);
    await h.finish();
    h.emit("keydown", { key: ";" });
    h.emit("keydown", { key: preset === "original" ? "4" : "h" }); await flushCommands();
    assert.deepEqual(h.requests[3].command, { type: "move", direction: "west" });
    await h.finish(); h.controller.dispose();
  }
});

test("special walk direction obeys cancellation and input contexts without leaking its flip", async t => {
  installElementIdentities(t);
  for (const cancel of ["Escape", "blur", "click", "reset"]) {
    const h = continuousHarness("local", undefined, "original");
    h.emit("keydown", { key: "-", target: new HTMLInputElement() });
    h.emit("keydown", { key: "-", isComposing: true });
    h.document.querySelector = () => ({});
    h.emit("keydown", { key: "-" });
    assert.equal(h.messages.length, 0);
    h.document.querySelector = () => null;
    h.emit("keydown", { key: "-" });
    h.emit("keydown", { key: "5" });
    h.emit("keydown", { key: "l", ctrlKey: true });
    assert.equal(h.requests.length, 0, "invalid/modified directions do not dispatch another command");
    if (cancel === "Escape") h.emit("keydown", { key: "Escape" });
    else if (cancel === "reset") h.controller.resetSession();
    else h.emit(cancel);
    h.emit("keydown", { key: "6" }); await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "move", direction: "east" }, cancel);
    await h.finish(); h.controller.dispose();
  }
  const world = continuousHarness("world", undefined, "roguelike");
  world.emit("keydown", { key: "-" }); world.emit("keydown", { key: "l" }); await flushCommands();
  assert.deepEqual(world.requests[0].command, { type: "walk-special", direction: "east" });
  await world.finish(); world.controller.dispose();
});

test("source stay keys use one counted command and rogue comma remains the run prefix", async t => {
  installElementIdentities(t);
  for (const [preset, key] of [["original", ","], ["original", "5"], ["roguelike", "."], ["roguelike", "5"]]) {
    const h = continuousHarness("local", undefined, preset);
    for (const k of ["0", "2", " ", key]) h.emit("keydown", { key: k });
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "stay" });
    await h.finish(update => { update.commandRepeatable = true; }); await h.tick();
    assert.deepEqual(h.requests[1].command, { type: "stay" });
    const stopped = h.controller.stopContinuousAction(); await h.finish(); await stopped;
    h.emit("keydown", { key: preset === "original" ? "n" : "X" }); await flushCommands();
    assert.deepEqual(h.requests[2].command, { type: "stay" });
    await h.finish(); h.controller.dispose();
  }
  assert.deepEqual(commandForKeyboardInput({ key: ".", code: "Period" }, "roguelike"), { type: "stay" });
  assert.equal(commandForKeyboardInput({ key: ",", code: "Comma" }, "roguelike"), undefined);
});

test("special walk key interrupts an in-flight repeat without starting a direction prompt", async t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  for (const key of "03s") h.emit("keydown", { key });
  await flushCommands();
  h.emit("keydown", { key: "-" });
  await h.finish(update => { update.commandRepeatable = true; });
  await h.controller.stopContinuousAction();
  h.emit("keydown", { key: "6" }); await flushCommands();
  assert.deepEqual(h.requests[1].command, { type: "move", direction: "east" });
  await h.finish(); h.controller.dispose();
});

test("count prefix edits digits and repeats only after authoritative responses", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    for (const key of ["0", "2", "5", "Backspace", "3", "h", "Enter", preset === "original" ? "6" : "l"]) {
      h.emit("keydown", { key, ctrlKey: key === "h" });
    }
    await flushCommands();
    assert.equal(h.requests.length, 1, "23 then Ctrl+H leaves a total count of 2");
    assert.deepEqual(h.requests[0].command, { type: "move", direction: "east" });
    await h.finish(update => { update.commandRepeatable = true; });
    await h.tick();
    assert.equal(h.requests.length, 2);
    await h.finish(update => { update.commandRepeatable = true; });
    await h.tick();
    assert.equal(h.requests.length, 2, "the first action counts toward the limit");
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  }
});

test("counted search defaults to 99, caps at 9999, and accepts a command without a separator", async t => {
  installElementIdentities(t);
  for (const [digits, total] of [["", 99], ["000", 99], ["99999", 9999]]) {
    const h = continuousHarness("local", undefined, "original");
    for (const key of "0" + digits + "s") h.emit("keydown", { key });
    await flushCommands();
    for (let i = 0; i < total; i++) {
      assert.equal(h.requests.length, i + 1);
      await h.finish(update => { update.commandRepeatable = true; });
      await h.tick();
    }
    assert.equal(h.requests.length, total);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  }
});

test("count prefixes respect text contexts and clear on clicks, blur, and unrelated commands", async t => {
  installElementIdentities(t);
  for (const extra of [{ target: new HTMLInputElement() }, { isComposing: true }, { repeat: true }]) {
    const h = continuousHarness("local", undefined, "original");
    h.emit("keydown", { key: "0", ...extra });
    h.emit("keydown", { key: "s" }); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined);
    await h.finish(); h.controller.dispose();
  }
  for (const stop of ["click", "blur", "shortcut"]) {
    const h = continuousHarness("local", undefined, "original");
    for (const key of "03 ") h.emit("keydown", { key });
    if (stop === "shortcut") h.emit("keydown", { key: "i" });
    else h.emit(stop);
    h.emit("keydown", { key: "s" }); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined, stop);
    await h.finish(); h.controller.dispose();
  }
});

test("counted commands honor Core stop, failure, queries, cancellation and session replacement", async t => {
  installElementIdentities(t);
  for (const reason of ["core", "failure", "query", "escape", "key", "blur", "hidden", "reset", "dispose"]) {
    const h = continuousHarness("local", undefined, "original");
    for (const key of "03s") h.emit("keydown", { key });
    await flushCommands();
    if (reason === "escape") h.emit("keydown", { key: "Escape" });
    if (reason === "key") h.emit("keydown", { key: "0" });
    if (reason === "blur") h.emit("blur");
    if (reason === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); }
    if (reason === "reset") h.controller.resetSession();
    if (reason === "dispose") h.controller.dispose();
    if (reason === "failure") { h.requests[0].reject(new Error("transport failure")); await flushCommands(); }
    else await h.finish(update => {
      update.commandRepeatable = reason !== "core";
      if (reason === "query") update.mogaminator.pendingQuery = {};
    });
    if (h.timers.size) await h.tick();
    assert.equal(h.requests.length, 1, reason);
    assert.equal(h.controller.continuousAction, undefined, reason);
    h.controller.dispose();
  }
});

test("count carries through direction selection, but does not repeat spikes or leak after cancellation", async t => {
  installElementIdentities(t);
  for (const command of ["o", "T", "+", "j"]) {
    const h = continuousHarness("local", undefined, "original");
    for (const key of "03" + command + "6") h.emit("keydown", { key });
    await flushCommands();
    assert.equal(h.requests.length, 1);
    await h.finish(update => { update.commandRepeatable = false; }); await h.tick();
    assert.equal(h.requests.length, 1);
    assert.equal(h.state.terrainInteractionMode, undefined);
    h.controller.dispose();
  }
  for (const keys of [["0", "2", "Escape"], ["0", "2", " ", "Escape"], ["0", "2", "o", "Escape"]]) {
    const h = continuousHarness("local", undefined, "original");
    for (const key of keys) h.emit("keydown", { key });
    assert.equal(h.requests.length, 0);
    h.emit("keydown", { key: "s" }); await flushCommands();
    await h.finish(update => { update.commandRepeatable = true; });
    if (h.timers.size) await h.tick();
    assert.equal(h.requests.length, 1);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  }
});

test("counted run passes its limit to Core and counted rest uses fixed turns", async t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  for (const key of "02.6") h.emit("keydown", { key });
  await flushCommands();
  assert.deepEqual(h.requests[0].command, { type: "run", direction: "east", maxSteps: 2 });
  await h.finish(update => { update.player.running = {}; }); await h.tick();
  assert.equal(h.requests[1].command.type, "continue-run");
  await h.finish(update => { delete update.player.running; }); await h.tick();
  assert.equal(h.controller.continuousAction, undefined);
  for (const key of "02R") h.emit("keydown", { key });
  await flushCommands();
  for (let i = 0; i < 2; i++) {
    assert.deepEqual(h.requests.at(-1).command, { type: "rest-for-turns", turns: 1 });
    await h.finish(update => { update.events = [{ outcome: { type: "rest", resolution: { completedTurns: 1, stopReason: "turn-limit" } } }]; });
    await h.tick();
  }
  assert.equal(h.requests.length, 4);
  assert.equal(h.controller.continuousAction, undefined);
  h.controller.dispose();
});

test("spike keys select one direction, cancel freely, and do not repeat or interrupt text input", async t => {
  installElementIdentities(t);
  for (const [preset, key, direction] of [["original", "j", "6"], ["roguelike", "S", "l"]]) {
    const h = continuousHarness("local", undefined, preset);
    h.state.status.terrainInteractions = [];
    h.emit("keydown", { key });
    assert.equal(h.state.terrainInteractionMode, "spike-door");
    assert.equal(h.requests.length, 0);
    h.emit("keydown", { key: "Escape" });
    assert.equal(h.state.terrainInteractionMode, undefined);
    for (const extra of [{ target: new HTMLInputElement() }, { isComposing: true }, { repeat: true }]) {
      h.emit("keydown", { key, ...extra });
      assert.equal(h.state.terrainInteractionMode, undefined);
    }
    h.emit("keydown", { key }); h.emit("keydown", { key: direction });
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "spike-door", direction: "east" });
    await h.finish();
    assert.equal(h.timers.size, 0);
    assert.equal(h.state.terrainInteractionMode, undefined);
    h.start(); await flushCommands();
    h.emit("keydown", { key }); await h.finish();
    assert.equal(h.requests.length, 2, "spike key stops travel without issuing a spike");
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.state.terrainInteractionMode, undefined);
    h.controller.dispose();
    const world = continuousHarness("world", undefined, preset);
    world.emit("keydown", { key });
    assert.equal(world.state.terrainInteractionMode, undefined);
    assert.equal(world.requests.length, 0);
    world.controller.dispose();
  }
  assert.deepEqual(commandForKeyboardInput({ key: "j" }, "roguelike"), { type: "move", direction: "south" });
});

test("alter prefixes and control directions dispatch one Core operation without a terrain preview", async t => {
  installElementIdentities(t);
  for (const [preset, keys] of [
    ["original", [{ key: "+" }, { key: "6" }]],
    ["original", [{ key: "+" }, { key: "ArrowRight", code: "ArrowRight" }]],
    ["roguelike", [{ key: "+" }, { key: "l" }]],
    ["original", [{ key: "ArrowUp", code: "ArrowUp", ctrlKey: true }]],
    ["original", [{ key: "End", code: "Numpad1", ctrlKey: true }]],
    ["roguelike", [{ key: "h", code: "KeyH", ctrlKey: true }]],
  ]) await t.test(`${preset} ${keys.map(key => key.key)}`, async () => {
    const h = continuousHarness("local", undefined, preset);
    h.state.status.terrainInteractions = [];
    for (const key of keys) { h.emit("keydown", key); await flushCommands(); }
    assert.equal(h.requests.length, 1);
    assert.equal(h.requests[0].command.type, "alter");
    const direction = keys.length > 1 ? "east" : keys[0].key === "h" ? "west" : keys[0].key === "End" ? "south-west" : "north";
    assert.equal(h.requests[0].command.direction, direction);
    await h.finish();
    await h.tick();
    assert.equal(h.timers.size, 0);
    assert.equal(h.state.terrainInteractionMode, undefined);
    h.controller.dispose();
  });
  await t.test("cancel, world map, contexts and continuous action boundaries", async () => {
    const h = continuousHarness("local", undefined, "original");
    h.state.status.terrainInteractions = [];
    h.emit("keydown", { key: "+" }); h.emit("keydown", { key: "Escape" });
    assert.equal(h.state.terrainInteractionMode, undefined);
    assert.equal(h.requests.length, 0);
    for (const extra of [{ target: new HTMLInputElement() }, { isComposing: true }, { repeat: true }]) {
      h.emit("keydown", { key: "+", ...extra });
      h.emit("keydown", { key: "ArrowUp", code: "ArrowUp", ctrlKey: true, ...extra });
    }
    assert.equal(h.requests.length, 0);
    h.state.targeting = {};
    h.emit("keydown", { key: "ArrowUp", code: "ArrowUp", ctrlKey: true });
    assert.equal(h.requests.length, 0); h.state.targeting = undefined;
    h.start(); await flushCommands();
    h.emit("keydown", { key: "ArrowUp", code: "ArrowUp", ctrlKey: true });
    await h.finish();
    assert.equal(h.requests.length, 1, "Ctrl-direction cancels the current trip without attacking");
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
    const world = continuousHarness("world", undefined, "roguelike");
    world.emit("keydown", { key: "+" }); world.emit("keydown", { key: "h", ctrlKey: true });
    assert.equal(world.requests.length, 0);
    assert.equal(world.state.terrainInteractionMode, undefined);
    world.controller.dispose();
  });
  for (const preset of ["original"]) {
    assert.equal(alterDirectionForKeyboardInput({ key: "h", code: "KeyH", ctrlKey: true }, preset), undefined);
  }
  assert.equal(alterDirectionForKeyboardInput(Object.create({ key: "ArrowUp", code: "ArrowUp", ctrlKey: true }), "original"), "north", "native event accessors need not be own enumerable properties");
});

test("search mode keys and button dispatch one toggle; discoveries stop travel without clearing the mode", async t => {
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
  for (const preset of ["original", "roguelike", "wasd"]) await t.test(preset, async () => {
    const h = continuousHarness("local", undefined, preset);
    if (preset === "wasd") h.searchButton.listeners.get("click")();
    else h.emit("keydown", { key: preset === "original" ? "S" : "#", shiftKey: true });
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "toggle-search" });
    await h.finish(update => { update.player.searching = true; });
    await flushCommands();
    assert.equal(h.requests.length, 1);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    h.emit("keydown", { key: "Escape" });
    await h.controller.prepareSessionAccess();
    assert.equal(h.requests.length, 1, "ordinary cancellation and saving preserve mode");
    assert.equal(h.state.status.player.searching, true);
    if (preset !== "wasd") {
      h.emit("keydown", { key: "s" }); await flushCommands();
      assert.deepEqual(h.requests.at(-1).command, { type: "search" });
      await h.finish();
    }
    h.controller.dispose();
  });
  for (const discovery of [
    { kind: "terrain.secret-discovered" }, { messageKey: "chest-trap-found" },
  ]) await t.test(JSON.stringify(discovery), async () => {
    const h = continuousHarness();
    h.state.status.player.searching = true;
    const before = structuredClone(h.state.status);
    h.start(); await flushCommands();
    await h.finish(update => { update.events = [discovery]; }); await h.tick();
    assert.equal(h.requests.length, 1, "do not issue a second travel step");
    assert.equal(h.state.status.player.searching, true);
    assert.equal(autoGetStopsAfterStep(before, h.state.status, { objectId: "target", position: { x: 5, y: 1 } }), true);
    h.controller.dispose();
  });
});

test("nearest unknown item travel locks one Core target and preserves keyboard meanings", async t => {
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
  const target = { objectId: "alpha", position: { x: 3, y: 1 } };
  const selected = update => { update.events = [{ outcome: { type: "unknown-item-travel-target", target } }]; };
  for (const preset of ["original", "roguelike", "wasd"]) await t.test(preset, async () => {
    const h = continuousHarness("local", undefined, preset);
    if (preset === "wasd") h.unknownButton.listeners.get("click")();
    else h.emit("keydown", preset === "original" ? { key: "H", shiftKey: true } : { key: "e", ctrlKey: true });
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "find-nearest-unknown-item" });
    await h.finish(selected); await h.tick();
    assert.deepEqual(h.requests[1].command, { type: "travel-unknown-item", objectId: "alpha", destination: { x: 3, y: 1 } });
    await h.finish(); await h.tick();
    assert.deepEqual(h.requests[2].command, h.requests[1].command);
    await h.finish(); await h.tick();
    assert.equal(h.requests.length, 3, "arrival ends this trip without selecting another item");
    assert.equal(h.controller.continuousAction, undefined);
    assert.deepEqual(h.session.lastCommand, { type: "find-nearest-unknown-item" });
    const repeated = h.controller.repeatLastCommand(); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, { type: "find-nearest-unknown-item" });
    await h.finish(update => { update.events = []; }); await h.tick(); await repeated;
    h.controller.dispose();
  });
  await t.test("rogue H still runs west", async () => {
    const h = continuousHarness("local", undefined, "roguelike");
    h.emit("keydown", { key: "H", shiftKey: true }); await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "run", direction: "west" });
    await h.finish(); await h.tick(); h.controller.dispose();
  });
  for (const interruption of ["selection", "movement", "query", "reset"]) await t.test(interruption, async () => {
    const h = continuousHarness();
    void h.controller.travelToNearestUnknownItem(); await flushCommands();
    if (interruption === "selection") h.emit("keydown", { key: "Escape" });
    if (interruption === "reset") h.controller.resetSession();
    await h.finish(selected);
    if (interruption === "movement" || interruption === "query") {
      await h.tick();
      if (interruption === "movement") h.emit("keydown", { key: "Escape" });
      await h.finish(update => { if (interruption === "query") update.mogaminator.pendingQuery = { itemId: "alpha" }; });
    }
    await h.controller.prepareSessionAccess();
    assert.equal(h.requests.length, interruption === "selection" || interruption === "reset" ? 1 : 2);
    assert.equal(h.timers.size, 0); h.controller.dispose();
  });
  await t.test("no result preserves the old destination and a stopped item trip preserves its identity", async () => {
    const h = continuousHarness("local", undefined, "original");
    h.start(); await flushCommands(); h.emit("keydown", { key: "Escape" }); await h.finish();
    void h.controller.travelToNearestUnknownItem(); await flushCommands();
    await h.finish(update => { update.events = []; }); await h.tick();
    h.emit("keydown", { key: "J" }); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, { type: "travel-local", destination: { x: 5, y: 1 } });
    h.emit("keydown", { key: "Escape" }); await h.finish();
    target.position = { x: 8, y: 1 };
    void h.controller.travelToNearestUnknownItem(); await flushCommands(); await h.finish(selected); await h.tick();
    h.emit("keydown", { key: "Escape" }); await h.finish();
    h.emit("keydown", { key: "J" }); await flushCommands();
    assert.deepEqual(h.requests.at(-1).command, { type: "travel-unknown-item", objectId: "alpha", destination: { x: 8, y: 1 } });
    h.emit("keydown", { key: "Escape" }); await h.finish(); h.controller.dispose();
  });
  await t.test("world, text and modal contexts cannot start the command", async () => {
    const h = continuousHarness("world", undefined, "original");
    h.emit("keydown", { key: "H" }); await h.controller.travelToNearestUnknownItem();
    assert.equal(h.requests.length, 0); h.controller.dispose();
    const local = continuousHarness("local", undefined, "roguelike");
    local.emit("keydown", { key: "e", ctrlKey: true, target: new HTMLInputElement() });
    local.emit("keydown", { key: "e", ctrlKey: true, isComposing: true });
    local.state.terrainInteractionMode = "open-door";
    await local.controller.travelToNearestUnknownItem();
    assert.equal(local.requests.length, 0); local.controller.dispose();
  });
  await t.test("scrolling translates the locked position without changing identity", async () => {
    const h = continuousHarness();
    void h.controller.travelToNearestUnknownItem(); await flushCommands();
    target.position = { x: 8, y: 1 };
    await h.finish(selected); await h.tick();
    await h.finish(update => { update.mapTranslation = { x: -2, y: 0 }; update.player.position.x -= 2; });
    await h.tick();
    assert.deepEqual(h.requests.at(-1).command, { type: "travel-unknown-item", objectId: "alpha", destination: { x: 6, y: 1 } });
    h.emit("keydown", { key: "Escape" }); await h.finish(update => { delete update.mapTranslation; });
    h.controller.dispose();
  });
});

test("auto-explore keys, button and session boundaries follow Core state", async t => {
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
  for (const preset of ["original", "roguelike", "roguelike", "wasd", "original"]) await t.test(preset, async () => {
    const h = continuousHarness("local", undefined, preset);
    if (["original", "roguelike"].includes(preset)) {
      h.emit("keydown", { key: "Z", target: new HTMLInputElement() });
      h.emit("keydown", { key: "Z", isComposing: true });
      assert.equal(h.requests.length, 0);
      h.emit("keydown", { key: "Z" });
    } else h.exploreButton.listeners.get("click")();
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "auto-explore" });
    h.emit("keydown", { key: "Z", repeat: true });
    await h.finish(update => { update.player.autoExplore = {}; });
    await h.tick();
    assert.deepEqual(h.requests[1].command, { type: "continue-auto-explore" });
    h.emit("keydown", { key: "Escape" });
    let saved = false;
    const saving = h.controller.prepareSessionAccess().then(() => { saved = true; });
    await h.finish(update => { update.player.autoExplore = {}; });
    assert.equal(saved, false);
    assert.deepEqual(h.requests[2].command, { type: "cancel-auto-explore" });
    await h.finish(update => { delete update.player.autoExplore; });
    await saving;
    assert.equal(h.requests.length, 3);
    assert.equal(h.timers.size, 0);
    h.controller.dispose();
    assert.equal(h.exploreButton.listeners.size, 0);
  });

  await t.test("loaded exploration never resumes and failed cancellation blocks saving", async () => {
    const h = continuousHarness("local", "failed");
    h.state.status.player.autoExplore = {};
    h.controller.reconcileStatus(h.state.status);
    assert.equal(h.timers.size, 0);
    assert.equal(h.requests.length, 0);
    await assert.rejects(h.controller.prepareSessionAccess(), /message-auto-explore-cancel-required/);
    h.controller.dispose();
  });
  for (const trigger of ["complete", "query", "blur", "hidden", "reset"]) await t.test(trigger, async () => {
    const h = continuousHarness();
    void h.controller.autoExplore(); await flushCommands();
    if (trigger === "reset") h.controller.resetSession();
    if (trigger === "blur") h.emit("blur");
    if (trigger === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); }
    await h.finish(update => {
      if (trigger !== "complete" && trigger !== "query") update.player.autoExplore = {};
      if (trigger === "query") update.mogaminator.pendingQuery = { itemId: "alpha" };
    });
    if (trigger === "blur" || trigger === "hidden") {
      assert.deepEqual(h.requests[1].command, { type: "cancel-auto-explore" });
      await h.finish(update => { delete update.player.autoExplore; });
    } else if (h.timers.size) await h.tick();
    assert.equal(h.requests.length, trigger === "blur" || trigger === "hidden" ? 2 : 1);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  });
});

test("running keys select a direction and cancel at the committed step boundary", async t => {
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
  for (const [preset, keys, direction] of [
    ["original", [{ key: "." }, { key: "6" }], "east"],
    ["roguelike", [{ key: "," }, { key: "l" }], "east"],
    ["roguelike", [{ key: "H", shiftKey: true }], "west"],
    ["original", [{ key: "ArrowUp", code: "ArrowUp", shiftKey: true }], "north"],
    ["roguelike", [{ key: "9", code: "Numpad9", shiftKey: true }], "north-east"],
  ]) await t.test(`${preset} ${keys.map(k => k.key).join(" ")}`, async () => {
    const h = continuousHarness("local", undefined, preset);
    for (const key of keys) h.emit("keydown", key);
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "run", direction });
    assert.equal(h.controller.continuousAction, "run");
    h.emit("keydown", { ...keys.at(-1), repeat: true });
    await h.finish(update => { update.player.running = {}; });
    assert.equal(h.timers.size, 1, "key repeat must not cancel the run it started");
    await h.tick();
    assert.deepEqual(h.requests[1].command, { type: "continue-run" });
    h.emit("keydown", { key: "Escape" });
    const stopping = h.controller.stopContinuousAction();
    await h.finish(update => { update.player.running = {}; });
    assert.deepEqual(h.requests[2].command, { type: "cancel-run" });
    let stopped = false; void stopping.then(() => { stopped = true; });
    await flushCommands(); assert.equal(stopped, false);
    await h.finish(update => { delete update.player.running; });
    await stopping;
    assert.equal(h.requests.length, 3);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  });
  await t.test("prefix cancellation, context guards and ordinary rogue period", async () => {
    const h = continuousHarness("local", undefined, "original");
    h.emit("keydown", { key: ".", target: new HTMLInputElement() });
    h.emit("keydown", { key: ".", isComposing: true });
    assert.equal(h.messages.length, 0);
    h.emit("keydown", { key: "." });
    h.emit("keydown", { key: "Escape" });
    h.emit("keydown", { key: "6" }); await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "move", direction: "east" });
    await h.finish(); h.controller.dispose();
    const rogue = continuousHarness("local", undefined, "roguelike");
    rogue.emit("keydown", { key: "." }); await flushCommands();
    assert.deepEqual(rogue.requests[0].command, { type: "stay" });
    await rogue.finish(); rogue.controller.dispose();
    assert.equal(runDirectionForKeyboardInput({ key: "H", code: "KeyH", shiftKey: true }, "roguelike"), "west");
  });
  await t.test("core stop is final and loaded running state never starts a timer", async () => {
    const h = continuousHarness("local", undefined, "roguelike");
    h.state.status.player.running = {};
    h.controller.reconcileStatus(h.state.status);
    assert.equal(h.requests.length, 0);
    assert.equal(h.timers.size, 0);
    h.emit("keydown", { key: "L" }); await flushCommands();
    await h.finish(update => { delete update.player.running; });
    await h.tick();
    assert.equal(h.requests.length, 1);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  });
  await t.test("failed cancellation prevents session access", async () => {
    const h = continuousHarness("local", "failed", "original");
    h.state.status.player.running = {};
    await assert.rejects(h.controller.prepareSessionAccess(), /message-run-cancel-required/);
    assert.equal(h.timers.size, 0);
    h.controller.dispose();
  });
  for (const trigger of ["blur", "hidden", "reset"]) await t.test(`run lifecycle: ${trigger}`, async () => {
    const h = continuousHarness("local", undefined, "roguelike");
    h.emit("keydown", { key: "L" }); await flushCommands();
    if (trigger === "reset") h.controller.resetSession();
    else if (trigger === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); }
    else h.emit("blur");
    await h.finish(update => { update.player.running = {}; });
    if (trigger === "reset") {
      assert.equal(h.requests.length, 1, "an old run must not cancel the new session");
    } else {
      assert.deepEqual(h.requests[1].command, { type: "cancel-run" });
      await h.finish(update => { delete update.player.running; });
      await h.controller.prepareSessionAccess();
      assert.equal(h.requests.length, 2);
    }
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  });
});

// Real controller/session dispatch with deferred Core responses and task timers.
function continuousHarness(kind = "local", result, preset = "roguelike", options = {}) {
  const state = new AppState();
  state.mode = "playing";
  const destination = { x: 5, y: 1 };
  state.status = {
    revision: 1, mapScale: kind === "world" ? "world" : "local", floorId: "floor.1",
    operationOptions: defaultPreferences().operations,
    worldTravelDestination: kind === "world" ? destination : null,
    player: { position: { x: 1, y: 1 }, hp: 10, statuses: [], inventoryUsedSlots: 0, inventorySlotCapacity: 10 },
    entities: [], terrainInteractions: [], goldPiles: [], items: [{ id: "alpha", position: { x: 3, y: 1 } }],
    mogaminator: { autoGetTarget: { objectId: "alpha", position: { x: 3, y: 1 } } },
  };
  const listeners = [];
  const timers = new Map();
  let timerId = 0;
  class DialogElement extends EventTarget {
    children = [];
    open = false;
    returnValue = "";
    append(...children) { this.children.push(...children); for (const child of children) child.parent = this; }
    setAttribute() {}
    select() {}
    showModal() { this.open = true; }
    close(value = this.returnValue) {
      if (!this.open) return;
      this.open = false; this.returnValue = value;
      queueMicrotask(() => this.dispatchEvent(new Event("close")));
    }
    remove() { this.parent.children = this.parent.children.filter(child => child !== this); }
  }
  const document = { hidden: false, body: new DialogElement(),
    createElement: () => new DialogElement(),
    querySelector: () => document.body.children.find(dialog => dialog.open) ?? null,
    addEventListener: (type, fn) => listeners.push({ type, fn }),
    removeEventListener: (type, fn) => { const i = listeners.findIndex(l => l.type === type && l.fn === fn); if (i >= 0) listeners.splice(i, 1); },
  };
  const window = {
    document,
    prompt: () => "&",
    addEventListener: (type, fn, capture) => listeners.push({ type, fn, capture }),
    removeEventListener: document.removeEventListener,
    setTimeout: fn => { timers.set(++timerId, fn); return timerId; },
    clearTimeout: id => timers.delete(id),
  };
  const mapHost = { ownerDocument: document, contains: target => target === mapHost,
    addEventListener: document.addEventListener, removeEventListener: document.removeEventListener };
  const button = { addEventListener() {}, removeEventListener() {} };
  const exploreButton = { listeners: new Map(),
    addEventListener(type, fn) { this.listeners.set(type, fn); },
    removeEventListener(type) { this.listeners.delete(type); },
  };
  const unknownButton = { ...exploreButton, listeners: new Map() };
  const searchButton = { ...exploreButton, listeners: new Map() };
  const requests = [], errors = [], messages = [], changes = [], shortcuts = [], chestChoices = [];
  const session = new GameSession({ state,
    execute: command => { const pending = Promise.withResolvers(); requests.push({ command, ...pending }); return pending.promise; },
    applyUpdate: update => { state.status = update; controller.reconcileStatus(update); },
    refreshBusyControls() {}, showError: error => errors.push(error),
  });
  const controller = new InputController({ state, window,
    dom: { mapHost, autoExplore: exploreButton, nearestUnknownItem: unknownButton, searchModeToggle: searchButton, traverseStairs: button, targetModeToggle: button, lookModeToggle: button },
    localization: { format: key => key }, getInputPreset: () => preset, getZoom: () => 1,
    dispatch: result ? async () => result : (command, repeatCommand) => session.dispatch(command, repeatCommand),
    getLastCommand: () => session.lastCommand,
    confirmRepeat: command => window.confirm?.(command) ?? true,
    whenIdle: () => session.whenIdle(),
    onShortcut: command => shortcuts.push(command),
    chooseChest: (command, items) => chestChoices.push({ command, ids: items.map(item => item.id) }),
    onContinuousActionChange: () => changes.push(controller.continuousAction),
    describeLook: () => "", openObjectList() {}, openMogaminator() {}, onLookFocusChange() {},
    announce: key => messages.push(key),
    ...options,
  });
  controller.render = () => {};
  controller.install();
  function emit(type, data = {}) {
    const event = { key: "", code: "", target: null, ...data, defaultPrevented: false, stopped: false,
      preventDefault() { this.defaultPrevented = true; }, stopImmediatePropagation() { this.stopped = true; },
    };
    for (const listener of listeners.filter(l => l.type === type).sort((a, b) => Number(Boolean(b.capture)) - Number(Boolean(a.capture)))) {
      if (event.stopped) break;
      listener.fn(event);
    }
    return event;
  }
  return { state, controller, session, requests, errors, messages, changes, shortcuts, chestChoices, timers, window, document, mapHost, emit, exploreButton, unknownButton, searchButton,
    confirmRest(value = "&") {
      const dialog = document.body.children[0];
      if (value === null) { dialog.close(); return; }
      const form = dialog.children[0];
      form.children[0].children[0].value = value;
      form.dispatchEvent(new Event("submit", { cancelable: true }));
    },
    start() {
      if (kind === "world") emit("keydown", { key: preset === "roguelike" ? "(" : "J" });
      else if (kind === "rest") void controller.restUntilRecovered();
      else if (kind === "auto") void controller.autoGet();
      else void controller.travelLocalTo(destination);
    },
    async finish(edit = () => {}) {
      const update = structuredClone(state.status);
      update.revision++;
      if (["rest", "rest-for-turns", "rest-until-resources"].includes(requests.at(-1).command.type)) {
        update.events = [{ outcome: { type: "rest", resolution: { requestedTurns: 1, completedTurns: 1, stopReason: "turn-limit", resourceRecoveries: [] } } }];
      } else if (!["stay", "pick-up", "find-nearest-unknown-item"].includes(requests.at(-1).command.type)) update.player.position.x++;
      edit(update);
      requests.at(-1).resolve(update);
      await flushCommands();
    },
    async tick() {
      const [id, fn] = timers.entries().next().value;
      timers.delete(id); fn(); await flushCommands();
    },
  };
}

test("continuous travel and pickup cancel at the in-flight action boundary", async t => {
  // Minimal element identities for the production event context checks.
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
  for (const kind of ["local", "world", "auto", "rest"]) await t.test(kind, async () => {
    const h = continuousHarness(kind);
    h.start(); await flushCommands();
    assert.equal(h.state.busy, true);
    await h.controller.autoGet(); // A second entry must not run concurrently.
    assert.equal(h.requests.length, 1);
    assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
    const stopped = h.controller.stopContinuousAction();
    let settled = false;
    void stopped.then(() => { settled = true; });
    await flushCommands();
    assert.equal(settled, false);
    await h.finish(); await stopped;
    assert.equal(h.state.status.revision, 2, "committed result is retained");
    assert.equal(h.requests.length, 1);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.messages.filter(key => key === "message-continuous-action-stopped").length, 1);
    if (kind === "local" || kind === "world") {
      h.emit("keydown", { key: "(" }); await flushCommands();
      assert.deepEqual(h.requests[1].command.destination, { x: 5, y: 1 }, "the preset resume key retains the destination");
      const resumed = h.controller.stopContinuousAction();
      await h.finish(); await resumed;
    }
    h.controller.dispose();
  });

  await t.test("game input between steps is consumed; system shortcuts and repeats do not move", async () => {
    const h = continuousHarness(); h.start(); await flushCommands(); await h.finish();
    assert.equal(h.timers.size, 1, "each successful step yields a task");
    assert.equal(h.emit("keydown", { key: "r", ctrlKey: true }).stopped, false);
    assert.equal(h.emit("keydown", { key: "Escape", ctrlKey: true }).stopped, false);
    assert.equal(h.requests.length, 1);
    assert.equal(h.emit("keydown", { key: "l" }).stopped, true);
    await h.controller.stopContinuousAction();
    assert.equal(h.timers.size, 0);
    h.emit("keydown", { key: "l", repeat: true });
    assert.equal(h.requests.length, 1);
    h.emit("keydown", { key: "l" }); await flushCommands();
    assert.deepEqual(h.requests[1].command, { type: "move", direction: "east" });
    await h.finish(); h.controller.dispose();
  });

  await t.test("pickup cancellation cannot cross the outer target loop", async () => {
    const h = continuousHarness("auto"); h.start(); await flushCommands();
    await h.finish(); await h.tick();
    assert.deepEqual(h.requests[1].command, { type: "auto-get", objectId: "alpha" });
    const stopped = h.controller.stopContinuousAction();
    await h.finish(update => {
      update.items = [{ id: "beta", position: { x: 4, y: 1 } }];
      update.mogaminator.autoGetTarget = { objectId: "beta", position: { x: 4, y: 1 } };
    });
    await stopped;
    assert.equal(h.requests.length, 2);
    assert.equal(h.state.status.items[0].id, "beta");
    h.controller.dispose();
  });

  await t.test("editable and composing input stay local; a device shortcut stops without opening it", async () => {
    const h = continuousHarness(); h.start(); await flushCommands();
    assert.equal(h.emit("keydown", { key: "i", isComposing: true }).stopped, false);
    assert.equal(h.emit("keydown", { key: "i", target: new HTMLInputElement() }).stopped, false);
    assert.equal(h.emit("keydown", { key: "Shift" }).stopped, false);
    h.state.status.player.magicEater = {};
    assert.equal(h.emit("keydown", { key: "u", altKey: true }).stopped, true);
    await h.finish(); await h.controller.stopContinuousAction();
    assert.equal(h.requests.length, 1); h.controller.dispose();
  });

  for (const trigger of ["blur", "hidden", "dispose", "reset", "map-click"]) await t.test(trigger, async () => {
    const h = continuousHarness(); h.start(); await flushCommands();
    if (trigger === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); }
    else if (trigger === "dispose") h.controller.dispose();
    else if (trigger === "reset") h.controller.resetLocalTravel();
    else if (trigger === "map-click") {
      Object.setPrototypeOf(h.mapHost, Node.prototype);
      assert.equal(h.emit("click", { target: h.mapHost }).stopped, true);
    } else h.emit("blur");
    await h.finish(); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.requests.length, 1);
    assert.equal(h.timers.size, 0);
    h.controller.dispose();
  });
});

test("continuous actions stop on refused, failed and choice-requiring dispatches", async () => {
  for (const result of ["blocked", "failed"]) {
    const h = continuousHarness("local", result); h.start(); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.timers.size, 0); h.controller.dispose();
  }
  const h = continuousHarness(); h.start(); await flushCommands();
  const error = new Error("Core rejected the travel step");
  h.requests[0].reject(error); await flushCommands();
  assert.deepEqual(h.errors, [error]);
  assert.equal(h.controller.continuousAction, undefined);
  h.start(); await flushCommands();
  await h.finish(update => { update.player.pendingAbilityDirection = { abilityId: "choice" }; });
  assert.equal(h.controller.continuousAction, undefined);
  assert.equal(h.requests.length, 2);
  assert.equal(h.timers.size, 0); h.controller.dispose();
});

test("holding R does not cancel rest, but a fresh key press still interrupts", async t => {
  installElementIdentities(t);
  const h = continuousHarness("rest");
  h.emit("keydown", { key: "R", code: "KeyR", shiftKey: true });
  h.confirmRest();
  await flushCommands();
  assert.deepEqual(h.requests[0].command, { type: "rest", turns: 1 });
  assert.equal(h.emit("keydown", { key: "R", code: "KeyR", shiftKey: true, repeat: true }).stopped, false);
  await h.finish();
  h.emit("keydown", { key: "R", code: "KeyR", shiftKey: true, repeat: true });
  assert.equal(h.controller.continuousAction, "rest");
  await h.tick();
  assert.equal(h.requests.length, 2);
  assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
  await h.finish();
  await h.controller.stopContinuousAction();
  assert.equal(h.controller.continuousAction, undefined);
  assert.equal(h.requests.length, 2);
  h.controller.dispose();
});

test("rest confirmation finishes before automation starts; fresh input and real blur still stop it", async t => {
  installElementIdentities(t);
  for (const stop of ["Enter", "blur"]) {
    const h = continuousHarness("rest");
    h.window.prompt = () => { throw new Error("rest must not open a native focus-changing prompt"); };
    h.emit("keydown", { key: "R" });
    await h.controller.chooseRestMode();
    assert.equal(h.document.body.children.length, 1, "only one rest dialog can be open");
    h.emit("keydown", { key: "Enter" });
    assert.deepEqual(h.shortcuts, [], "dialog Enter must not open the command menu");
    h.confirmRest();
    h.emit("click"); // Confirmation click propagation precedes the queued dialog close event.
    assert.equal(h.controller.continuousAction, undefined);
    await flushCommands();
    assert.equal(h.document.body.children.length, 0);
    assert.deepEqual(h.requests[0].command, { type: "rest", turns: 1 });
    const heldEnter = h.emit("keydown", { key: "Enter", repeat: true, target: new HTMLButtonElement() });
    assert.equal(heldEnter.defaultPrevented, true, "held Enter must not click the restored focused button");
    await h.finish(); await h.tick();
    assert.equal(h.requests.length, 2);
    assert.equal(h.messages.includes("message-continuous-action-stopped"), false);
    if (stop === "blur") h.emit("blur");
    else h.emit("keydown", { key: "Enter" });
    await h.finish(); await h.controller.stopContinuousAction();
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.requests.length, 2);
    assert.equal(h.messages.filter(key => key === "message-continuous-action-stopped").length, 1);
    h.controller.dispose();
  }
});

test("rest continues only for a completed turn-limit receipt and never renews its budget", async () => {
  for (const reason of ["full-resources", "damaged", "enemy-visible", "player-died", "invalid-turns", "mutation-direction-required", "duelist-choice-required", "maia-path-choice-required", "pet-dismissal-required"]) {
    const h = continuousHarness();
    void h.controller.restUntilRecovered(); await flushCommands();
    await h.finish(update => {
      update.events[0].outcome.resolution.stopReason = reason;
      update.events[0].outcome.resolution.completedTurns = reason === "full-resources" ? 0 : 1;
    });
    await h.tick();
    assert.equal(h.controller.continuousAction, undefined, reason);
    assert.equal(h.requests.length, 1, reason);
    h.controller.dispose();
  }
  const h = continuousHarness();
  void h.controller.restUntilRecovered(); await flushCommands();
  for (let i = 0; i < 9_999; i++) {
    assert.deepEqual(h.requests[i].command, { type: "rest", turns: 1 });
    await h.finish(); await h.tick();
  }
  assert.equal(h.requests.length, 9_999);
  assert.equal(h.controller.continuousAction, undefined);
  assert.equal(h.timers.size, 0);
  assert.ok(h.messages.includes("message-rest-budget-reached"));
  h.controller.dispose();
});

test("travel keeps its existing danger, arrival and map-change stops", async () => {
  for (const stop of ["damage", "arrival", "floor", "enemy"]) {
    const h = continuousHarness(); h.start(); await flushCommands();
    await h.finish(update => {
      if (stop === "damage") update.player.hp--;
      if (stop === "arrival") update.player.position.x = 5;
      if (stop === "floor") update.floorId = "floor.2";
      if (stop === "enemy") update.entities = [{ faction: "hostile" }];
    });
    if (h.timers.size) await h.tick();
    assert.equal(h.controller.continuousAction, undefined, stop);
    assert.equal(h.requests.length, 1, stop); h.controller.dispose();
  }
});

test("fishing cancellation and session access share the committed command boundary", async t => {
  installElementIdentities(t);
  await t.test("loaded fishing progress never starts an automatic command", async () => {
    const h = continuousHarness();
    h.controller.resetSession();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status); await flushCommands();
    assert.equal(h.requests.length, 0);
    assert.equal(h.timers.size, 0);
    assert.equal(h.controller.continuousAction, "fishing", "loaded progress retains its explicit Stop affordance");
    assert.equal(h.state.status.player.fishingDirection, "east");
    h.controller.dispose();
  });
  await t.test("busy Escape is consumed and saving waits for exactly one zero-time cancel", async () => {
    const h = continuousHarness();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status); await flushCommands(); await h.tick();
    assert.equal(h.controller.continuousAction, "fishing");
    assert.deepEqual(h.requests[0].command, { type: "continue-fishing" });
    assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
    let saved = false;
    const save = h.controller.prepareSessionAccess().then(() => { saved = true; });
    await h.finish();
    assert.equal(saved, false);
    assert.deepEqual(h.requests.map(r => r.command.type), ["continue-fishing", "cancel-fishing"]);
    h.emit("keydown", { key: "l" }); // No move or duplicate cancel while settling.
    await h.finish(update => { update.player.fishingDirection = null; });
    await save;
    assert.equal(saved, true);
    assert.equal(h.requests.length, 2);
    assert.equal(h.timers.size, 0);
    h.controller.resetSession();
    h.controller.reconcileStatus(h.state.status); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined, "cancelled save does not restart");
    h.controller.dispose();
  });
  await t.test("a save requested during the fishing activation waits and cancels its new state", async () => {
    const h = continuousHarness();
    void h.session.dispatch({ type: "use-item", itemId: "fishing-activation" });
    const save = h.controller.prepareSessionAccess();
    await h.finish(update => { update.player.fishingDirection = "east"; });
    assert.deepEqual(h.requests.map(r => r.command.type), ["use-item", "cancel-fishing"]);
    await h.finish(update => { update.player.fishingDirection = null; }); await save;
    assert.equal(h.timers.size, 0); h.controller.dispose();
  });
  await t.test("failed cancellation blocks saving and allows an explicit retry", async () => {
    const h = continuousHarness();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status); await flushCommands();
    const save = assert.rejects(h.controller.prepareSessionAccess(), /message-fishing-cancel-required/);
    await flushCommands();
    assert.deepEqual(h.requests[0].command, { type: "cancel-fishing" });
    h.requests[0].reject(new Error("cancel failed")); await save;
    h.controller.reconcileStatus(h.state.status); await flushCommands();
    assert.equal(h.timers.size, 0, "failure does not restart the timer");
    assert.equal(h.controller.continuousAction, "fishing", "Stop remains reachable");
    const retry = h.controller.prepareSessionAccess(); await flushCommands();
    await h.finish(update => { update.player.fishingDirection = null; }); await retry;
    assert.equal(h.requests.length, 2); h.controller.dispose();
  });
  await t.test("reset drops old timers and loaded fishing remains idle until explicitly cancelled", async () => {
    const h = continuousHarness();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status); await flushCommands();
    const oldStop = h.controller.stopContinuousAction();
    h.controller.resetSession();
    h.controller.reconcileStatus(h.state.status); await flushCommands(); await oldStop;
    assert.equal(h.timers.size, 0);
    assert.equal(h.requests.length, 0);
    const stopping = h.controller.prepareSessionAccess(); await flushCommands();
    assert.equal(h.requests[0].command.type, "cancel-fishing");
    await h.finish(update => { update.player.fishingDirection = null; }); await stopping;
    assert.equal(h.controller.continuousAction, undefined);
    assert.equal(h.requests.length, 1, "loaded progress only sends the explicit zero-time cancellation"); h.controller.dispose();
  });
  await t.test("loaded fishing waits for interface synchronization before continuing", async () => {
    const h = continuousHarness();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status);
    void h.session.dispatch({ type: "set-interface-locale", locale: "zh-CN" });
    await flushCommands();
    assert.equal(h.timers.size, 0);
    await h.finish(); await h.tick();
    assert.equal(h.requests[1].command.type, "continue-fishing");
    await h.finish(update => { update.player.fishingDirection = null; });
    if (h.timers.size) await h.tick();
    h.controller.dispose();
  });
  for (const trigger of ["blur", "hidden", "click", "dispose"]) await t.test(trigger, async () => {
    const h = continuousHarness();
    h.state.status.player.fishingDirection = "east";
    h.controller.reconcileStatus(h.state.status); await flushCommands(); await h.tick();
    if (trigger === "dispose") h.controller.dispose();
    else if (trigger === "hidden") { h.document.hidden = true; h.emit("visibilitychange"); }
    else h.emit(trigger);
    await h.finish();
    if (trigger !== "dispose") {
      assert.equal(h.requests[1].command.type, "cancel-fishing");
      await h.finish(update => { update.player.fishingDirection = null; });
    } else assert.equal(h.requests.length, 1, "disposed input cannot submit another request");
    assert.equal(h.timers.size, 0); h.controller.dispose();
  });
});

function installElementIdentities(t) {
  for (const name of ["Node", "HTMLElement", "HTMLButtonElement", "HTMLInputElement", "HTMLTextAreaElement", "HTMLSelectElement"]) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: class {} });
    t.after(() => previous ? Object.defineProperty(globalThis, name, previous) : delete globalThis[name]);
  }
}

function clickableMapHarness(options = {}, kind = "local") {
  const h = continuousHarness(kind, undefined, "original", options);
  Object.setPrototypeOf(h.mapHost, Node.prototype);
  Object.assign(h.mapHost, { dataset: { cameraX: "0", cameraY: "0" },
    clientLeft: 2, clientTop: 2, clientWidth: 500, clientHeight: 300, scrollLeft: 0, scrollTop: 0,
    getBoundingClientRect: () => ({ left: 100, top: 50 }) });
  h.state.status.width = h.state.mapWidth = 20;
  h.state.status.height = h.state.mapHeight = 20;
  h.click = (extra = {}) => h.emit("click", { target: h.mapHost, button: 0, detail: 1,
    clientX: 100 + 2 + 4.5 * 28, clientY: 50 + 2 + 1.5 * 28, ...extra });
  return h;
}

test("map clicks travel to the displayed cell across camera offsets, zoom and scrolling", async t => {
  installElementIdentities(t);
  for (const [zoom, cameraX, cameraY, scrollLeft, scrollTop] of [[1, 0, 0, 0, 0], [2, -56, 28, 0, 0], [0.75, 0, 0, 28, 14]]) {
    const h = clickableMapHarness({ getZoom: () => zoom });
    Object.assign(h.mapHost.dataset, { cameraX: String(cameraX), cameraY: String(cameraY) });
    Object.assign(h.mapHost, { scrollLeft, scrollTop });
    const destination = { x: 4, y: 3 };
    h.click({ clientX: 102 + (4.5 * 28 * zoom) + cameraX - scrollLeft,
      clientY: 52 + (3.5 * 28 * zoom) + cameraY - scrollTop });
    await flushCommands();
    assert.deepEqual(h.requests.map(request => request.command), [{ type: "travel-local", destination }]);
    await h.finish(update => { update.player.position = destination; });
    await h.tick();
    assert.equal(h.controller.continuousAction, undefined);
    h.controller.dispose();
  }
  const world = clickableMapHarness({}, "world");
  world.click(); await flushCommands();
  assert.deepEqual(world.requests[0].command, { type: "travel-world", destination: { x: 4, y: 1 } });
  await world.finish(update => { update.player.position = { x: 4, y: 1 }; });
  await world.tick();
  world.controller.dispose();
});

test("map clicks confirm targeting by entity or position and only inspect in look mode", async t => {
  installElementIdentities(t);
  for (const intent of [{ type: "select-target" }, { type: "projectile" }, { type: "ability", abilityId: "spell" },
    { type: "item", itemId: "wand" }, { type: "absorbed-device", itemId: "absorbed" }, { type: "throw", itemId: "stone" }]) {
    const h = clickableMapHarness();
    h.state.status.entities = [{ id: "chosen", faction: "hostile", position: { x: 4, y: 1 } }];
    h.controller.startTargetingWithSpec({ modes: intent.type === "throw" ? ["direction"] : ["entity", "position"], range: 8, requiresLineOfEffect: true }, intent);
    h.state.targeting.list = false;
    h.click(); await flushCommands();
    assert.equal(h.state.targeting, undefined);
    assert.deepEqual(h.controller.selectedTarget, { type: "entity", entityId: "chosen" });
    if (intent.type === "select-target") assert.equal(h.requests.length, 0);
    else {
      assert.equal(h.requests.length, 1);
      const command = h.requests[0].command;
      if (intent.type === "throw") assert.deepEqual(command, { type: "throw", itemId: "stone", direction: "east" });
      else assert.deepEqual(command.target ?? command.targets[0], { type: "entity", entityId: "chosen" });
      await h.finish();
    }
    h.click({ detail: 2 }); await flushCommands();
    assert.equal(h.controller.continuousAction, undefined, "a double-click cannot start travel after confirming a target");
    h.controller.dispose();
  }
  const focus = [], h = clickableMapHarness({ onLookFocusChange: position => focus.push(position) });
  h.controller.startLookMode(); h.click();
  assert.deepEqual(focus.at(-1), { x: 4, y: 1 });
  assert.equal(h.state.targetingIntent.type, "look");
  assert.equal(h.requests.length, 0);
  h.controller.cancelTargeting(false);
  h.controller.startTargetSelection(); h.click();
  assert.deepEqual(h.controller.selectedTarget, { type: "position", position: { x: 4, y: 1 } });
  assert.equal(h.requests.length, 0);
  h.controller.dispose();
});

test("map clicks ignore blocked contexts and stop continuous movement without queuing another destination", async t => {
  installElementIdentities(t);
  const h = clickableMapHarness();
  for (const extra of [{ button: 2 }, { detail: 2 }, { shiftKey: true }, { target: null },
    { clientX: 101 }, { clientX: 602 }, { clientY: 51 }, { clientY: 352 }]) h.click(extra);
  h.mapHost.dataset.cameraX = "300"; h.click(); h.mapHost.dataset.cameraX = "0";
  h.state.status.width = 3; h.click(); h.state.status.width = 20;
  h.document.querySelector = () => ({}); h.click(); h.document.querySelector = () => null;
  h.state.busy = true; h.click(); h.state.busy = false;
  h.state.terrainInteractionMode = "open-door"; h.click(); h.state.terrainInteractionMode = undefined;
  h.state.status.player.pendingMaiaPathChoice = true; h.click(); h.state.status.player.pendingMaiaPathChoice = false;
  await flushCommands(); assert.equal(h.requests.length, 0);
  h.start(); await flushCommands();
  assert.equal(h.requests.length, 1);
  assert.equal(h.click().stopped, true);
  await h.finish();
  assert.equal(h.requests.length, 1);
  assert.equal(h.controller.continuousAction, undefined);
  h.controller.dispose();
});

test("keyboard contexts isolate modifiers, text, aiming and directional prompts", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) await t.test(preset, async () => {
    const h = continuousHarness("local", undefined, preset);
    for (const key of ["r", "v", "f", "x", "o", "s", "a", "q", "z"]) {
      for (const modifier of ["ctrlKey", "metaKey", "altKey"]) {
        const registered = modifier === "ctrlKey" && ["s", "x", "q", "v", "f"].includes(key);
        assert.equal(h.emit("keydown", { key, code: key === "8" ? "Digit8" : `Key${key.toUpperCase()}`, [modifier]: true }).defaultPrevented, registered);
      }
    }
    assert.equal(h.requests.length, 0);
    assert.equal(h.state.targeting, undefined);
    assert.equal(h.state.terrainInteractionMode, undefined);
    const editable = new HTMLElement(); editable.isContentEditable = true;
    for (const extra of [{ isComposing: true }, { target: new HTMLInputElement() }, { target: editable }, { repeat: true }]) {
      h.emit("keydown", { key: "r", ...extra });
    }
    assert.equal(h.requests.length, 0);
    h.document.querySelector = () => ({ open: true });
    h.emit("keydown", { key: "r" });
    h.document.querySelector = () => null;
    assert.equal(h.requests.length, 0);
    assert.equal(h.emit("keydown", { key: "Escape" }).defaultPrevented, false, "idle Escape has no action");
    h.controller.dispose();
  });

  await t.test("each Escape clears one context; explicit paid aiming cancellations still dispatch", async () => {
    const h = continuousHarness();
    const aim = intent => {
      h.state.targeting = { origin: { x: 1, y: 1 }, cursor: { x: 2, y: 1 }, spec: { modes: ["direction"], range: 8 } };
      h.state.targetingIntent = intent;
    };
    h.state.terrainInteractionMode = "open-door";
    aim({ type: "look" });
    assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
    assert.equal(h.state.terrainInteractionMode, "open-door", "one Escape never cancels the next context");
    h.emit("keydown", { key: "Escape" });
    assert.equal(h.state.terrainInteractionMode, undefined);
    for (const intent of [{ type: "absorbed-device", itemId: "device" }, { type: "item", itemId: "activation" }, { type: "ability-direction" }]) {
      h.state.inventory = [{ id: "activation", activation: {}, usable: true }];
      aim(intent);
      assert.equal(h.emit("keydown", { key: "Enter", target: new HTMLButtonElement() }).defaultPrevented, false);
      assert.equal(h.emit("keydown", { key: "l", ctrlKey: true }).defaultPrevented, false);
      assert.deepEqual(h.state.targeting.cursor, { x: 2, y: 1 });
      assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
      await h.finish();
    }
    assert.deepEqual(h.requests.map(r => r.command), [
      { type: "use-absorbed-device", itemId: "device", targets: [] },
      { type: "use-item", itemId: "activation" },
      { type: "cancel-ability-direction" },
    ]);
    aim({ type: "mutation-direction" });
    h.emit("keydown", { key: "Escape" });
    assert.ok(h.state.targeting, "mandatory mutation direction cannot be cancelled");
    h.controller.resetSession();
    h.emit("keydown", { key: "v" }); h.controller.resetSession();
    h.emit("keydown", { key: "l" });
    assert.equal(h.requests[3].command.type, "move", "reset clears riding direction");
    await h.finish(); h.controller.dispose();
  });
  for (const interruption of ["escape", "session-reset", "none"]) await t.test("dig response: " + interruption, async () => {
    const h = continuousHarness();
    h.state.status.terrainInteractions = [{ kind: "dig-terrain", direction: "east", available: true }];
    h.emit("keydown", { key: "t", ctrlKey: true }); h.emit("keydown", { key: "l" });
    await flushCommands();
    assert.equal(h.requests[0].command.type, "dig-terrain");
    if (interruption === "escape") assert.equal(h.emit("keydown", { key: "Escape" }).stopped, true);
    if (interruption === "session-reset") h.controller.resetSession();
    await h.finish(update => { update.commandRepeatable = true; });
    assert.equal(h.state.terrainInteractionMode, undefined, "automatic retry never asks for a second direction");
    if (interruption === "none") {
      await h.tick();
      assert.deepEqual(h.requests[1].command, h.requests[0].command);
      const stop = h.controller.stopContinuousAction(); await h.finish(); await stop;
    } else {
      assert.equal(h.requests.length, 1);
      assert.equal(h.timers.size, 0);
    }
    h.controller.dispose();
  });
});

test("p opens pets in both presets and only cancels when automation is active", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    h.emit("keydown", { key: "p", target: new HTMLInputElement() });
    h.emit("keydown", { key: "p", isComposing: true });
    h.emit("keydown", { key: "p", repeat: true });
    h.document.querySelector = () => ({ open: true });
    h.emit("keydown", { key: "p" });
    assert.deepEqual(h.shortcuts, []);
    h.document.querySelector = () => null;
    h.emit("keydown", { key: "p" });
    assert.deepEqual(h.shortcuts, ["pets"]);
    h.start(); await flushCommands(); h.emit("keydown", { key: "p" });
    await h.finish(); await h.controller.stopContinuousAction();
    assert.deepEqual(h.shortcuts, ["pets"]);
    h.controller.startRiding(); h.emit("keydown", { key: "Escape" });
    const before = h.requests.length;
    h.controller.startRiding(); h.emit("keydown", { key: "6" });
    assert.deepEqual(h.requests[before].command, { type: "ride", direction: "east" });
    await h.finish(); h.controller.dispose();
  }
});

test("W routes ring exchange without stealing w, respects input contexts, and repeats the chosen pair", async t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    h.emit("keydown", { key: "W", target: new HTMLInputElement() });
    h.emit("keydown", { key: "W", isComposing: true });
    h.emit("keydown", { key: "W", repeat: true });
    assert.deepEqual(h.shortcuts, []);
    if (preset === "roguelike") {
      h.emit("keydown", { key: "W" });
      assert.deepEqual(h.shortcuts, ["map-locate"], "Rogue W locates instead of swapping rings");
      h.shortcuts.length = 0;
      h.emit("keydown", { key: "\\" });
    }
    h.emit("keydown", { key: "W", shiftKey: true });
    h.emit("keydown", { key: "w" });
    assert.deepEqual(h.shortcuts, ["swap-rings", "equip"]);
    const command = { type: "swap-rings", firstSlotId: "ring-3", secondSlotId: "ring-6" };
    const applied = h.session.dispatch(command); await h.finish(); await applied;
    h.emit("keydown", { key: preset === "original" ? "n" : "X" }); await flushCommands();
    assert.deepEqual(h.requests[1].command, command);
    await h.finish();
    h.start(); await flushCommands();
    h.emit("keydown", { key: preset === "original" ? "W" : "Escape" });
    await h.finish(); await h.controller.stopContinuousAction();
    assert.deepEqual(h.shortcuts, ["swap-rings", "equip"], "W stops automation without swapping too");
    h.controller.dispose();
  }
});

test("RFB command presets distinguish actions, directions and original command escapes", async t => {
  installElementIdentities(t);
  const original = continuousHarness("local", undefined, "original");
  for (const key of ["r", "v", "I", "i", "G", "b", "U", "F", "w", "q"]) original.emit("keydown", { key });
  assert.deepEqual(original.shortcuts, ["scroll", "throw", "inspect", "inventory", "study", "browse", "power", "refuel", "equip", "potion"]);
  assert.equal(original.requests.length, 0);
  original.emit("keydown", { key: "6", code: "Digit6" });
  assert.deepEqual(original.requests[0].command, { type: "move", direction: "east" });
  await original.finish();
  original.emit("keydown", { key: "R" }); await flushCommands();
  original.confirmRest(); await flushCommands();
  assert.equal(original.requests[1].command.type, "rest");
  original.emit("keydown", { key: "q" }); await original.finish();
  await original.controller.stopContinuousAction();
  assert.equal(original.shortcuts.length, 10, "a stop key does not also drink a potion");
  original.controller.dispose();

  const rogue = continuousHarness("local", undefined, "roguelike");
  rogue.emit("keydown", { key: "T" });
  rogue.emit("keydown", { key: "d", ctrlKey: true });
  rogue.emit("keydown", { key: "O" });
  rogue.emit("keydown", { key: "\\" }); rogue.emit("keydown", { key: "b" });
  rogue.emit("keydown", { key: "\\" }); rogue.emit("keydown", { key: "u" });
  assert.deepEqual(rogue.shortcuts, ["unequip", "destroy", "power", "browse", "staff"]);
  for (const key of ["M", "X"]) rogue.emit("keydown", { key });
  assert.equal(rogue.requests.length, 0, "unimplemented RFB commands do not fall through to legacy actions");
  rogue.emit("keydown", { key: "b" });
  assert.deepEqual(rogue.requests[0].command, { type: "move", direction: "south-west" });
  await rogue.finish(); rogue.controller.dispose();
});

test("save shortcuts reach the waiting save entry during travel and modal inputs stay local", async t => {
  installElementIdentities(t);
  const h = continuousHarness(); h.start(); await flushCommands();
  assert.equal(h.emit("keydown", { key: "s", ctrlKey: true }).stopped, true);
  assert.deepEqual(h.shortcuts, ["save"]);
  const stop = h.controller.stopContinuousAction(); await h.finish(); await stop;
  h.document.querySelector = () => ({ open: true });
  h.emit("keydown", { key: "x", ctrlKey: true });
  assert.deepEqual(h.shortcuts, ["save"]);
  h.document.querySelector = () => null;
  h.emit("keydown", { key: "x", ctrlKey: true, target: new HTMLInputElement() });
  h.emit("keydown", { key: "x", ctrlKey: true, isComposing: true });
  assert.deepEqual(h.shortcuts, ["save"]);
  h.emit("keydown", { key: "x", ctrlKey: true });
  assert.deepEqual(h.shortcuts, ["save", "save-exit"]); h.controller.dispose();
});

test("open and disarm direction prompts select projected chests including the player's square", t => {
  installElementIdentities(t);
  const h = continuousHarness("local", undefined, "original");
  h.state.status.items = [
    { id: "near", position: { x: 2, y: 1 }, chest: { canOpen: true, canDisarm: false } },
    { id: "here", position: { x: 1, y: 1 }, chest: { canOpen: false, canDisarm: true } },
  ];
  h.emit("keydown", { key: "o" }); h.emit("keydown", { key: "6" });
  h.emit("keydown", { key: "D" }); h.emit("keydown", { key: "5" });
  assert.deepEqual(h.chestChoices, [{ command: "open-chest", ids: ["near"] }, { command: "disarm-chest", ids: ["here"] }]);
  assert.equal(h.requests.length, 0, "the item choice owns the eventual core command"); h.controller.dispose();
});

test("RFB presets keep keypad movement, distinct letter directions and stay semantics", () => {
  for (const preset of ["original", "roguelike"]) {
    assert.deepEqual(commandForKeyboardInput({ key: "8", code: "Numpad8" }, preset), { type: "move", direction: "north" });
    assert.deepEqual(commandForKeyboardInput({ key: "5", code: "Numpad5" }, preset), { type: "stay" });
  }
  assert.equal(directionForKeyboardInput({ key: "h", code: "KeyH" }, "original"), undefined);
  assert.equal(directionForKeyboardInput({ key: "h", code: "KeyH" }, "roguelike"), "west");
  assert.equal(directionForKeyboardInput({ key: "y", code: "KeyY" }, "roguelike"), "north-west");
  assert.equal(directionForKeyboardInput({ key: "e", code: "KeyE" }, "original"), undefined);
});

test("Ctrl+G is distinct from lowercase pickup", () => {
  const modifiers = { shiftKey: false, altKey: false, metaKey: false };
  assert.equal(isAutoGetShortcut({ key: "g", ctrlKey: true, ...modifiers }), true);
  assert.equal(isAutoGetShortcut({ key: "g", ctrlKey: false, ...modifiers }), false);
  assert.equal(
    isAutoGetShortcut({ key: "G", ctrlKey: true, ...modifiers, shiftKey: true }),
    false,
  );
  assert.equal(commandShortcut({ key: "g" }, "roguelike"), "pickup");
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
  assert.equal(localTravelStopsAfterStep(before, { ...after, player: { ...after.player, pendingDuelist: { type: "follow-teleport" } } }, destination), true);
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

test("travel and auto-get continue after a fresh door-open event but stop on failure or danger", () => {
  const destination = { x: 4, y: 1 };
  const target = { objectId: "item", position: destination };
  const before = {
    revision: 1, mapScale: "local", floorId: "floor.1",
    player: { position: { x: 1, y: 1 }, hp: 10, isDead: false, statuses: [], inventoryUsedSlots: 0, inventorySlotCapacity: 10 },
    entities: [], items: [{ id: "item" }], goldPiles: [], mogaminator: {},
  };
  const opened = { ...before, revision: 2, events: [{ kind: "terrain.door-opened" }] };
  assert.equal(localTravelStopsAfterStep(before, opened, destination), false);
  assert.equal(autoGetStopsAfterStep(before, opened, target), false);
  for (const after of [
    { ...opened, events: [{ kind: "terrain.door-unlock-failed" }] },
    { ...opened, player: { ...opened.player, hp: 9 } },
    { ...opened, entities: [{ faction: "hostile" }] },
    { ...opened, revision: before.revision },
  ]) {
    assert.equal(localTravelStopsAfterStep(before, after, destination), true);
    assert.equal(autoGetStopsAfterStep(before, after, target), true);
  }
});

test("look and targeting cursors follow wilderness map translations", () => {
  const state = new AppState();
  const focused = [];
  const controller = new InputController({
    state,
    dom: {},
    localization: {},
    window: {},
    getInputPreset: () => "roguelike",
    getZoom: () => 1,
    whenIdle: async () => {},
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

test("authoritative race and level changes clear unavailable ability targeting", () => {
  const state = new AppState();
  const controller = new InputController({
    state, dom: {}, localization: {}, window: {},
    getInputPreset: () => "roguelike", getZoom: () => 1, whenIdle: async () => {}, dispatch: async () => {},
    describeLook: () => "", openObjectList: () => {}, openMogaminator: () => {},
    onLookOrTargeting: () => {}, onLookFocusChange: () => {}, announce: () => {},
  });
  controller.render = () => {};
  const targetSpec = { modes: ["direction"], range: 10, requiresLineOfEffect: true };
  for (const abilities of [[], [{ id: "old-race-power", canCast: false, targetSpec }], [{ id: "old-race-power", canCast: true, targetSpec }]]) {
    state.targeting = { origin: { x: 1, y: 1 }, cursor: { x: 2, y: 1 }, spec: targetSpec };
    state.targetingIntent = { type: "ability", abilityId: "old-race-power" };
    controller.reconcileStatus({
      mapScale: "local", floorId: "test.floor", width: 10, height: 10,
      player: { position: { x: 1, y: 1 }, abilities },
    });
    const remainsAvailable = abilities.some((ability) => ability.canCast);
    assert.equal(Boolean(state.targeting), remainsAvailable);
    assert.equal(Boolean(state.targetingIntent), remainsAvailable);
  }
});

test("absorbed aiming follows instance identity across slots and distinguishes user cancellation from load reconciliation", () => {
  const state = new AppState(), commands = [];
  const controller = new InputController({ state, dom: {}, localization: {}, window: {}, getInputPreset: () => "original", getZoom: () => 1,
    whenIdle: async () => {},
    dispatch: async command => commands.push(command), describeLook: () => "", openObjectList() {}, openMogaminator() {}, onLookFocusChange() {}, announce() {},
  });
  controller.render = () => {};
  const spec = { modes: ["direction"], range: 8, requiresLineOfEffect: true };
  const aim = () => { state.targeting = { origin: { x: 1, y: 1 }, cursor: { x: 2, y: 1 }, spec }; state.targetingIntent = { type: "absorbed-device", itemId: "body.instance" }; };
  const projected = slot => ({ mapScale: "local", floorId: "test.floor", width: 10, height: 10, player: { position: { x: 1, y: 1 }, magicEater: { slots: [slot] } } });
  aim(); controller.reconcileStatus(projected({ slot: 9, item: { id: "body.instance", usable: true, useTargetSpec: spec } }));
  assert.ok(state.targeting, "moving a slot does not substitute another instance");
  controller.cancelTargeting(); assert.deepEqual(commands, [{ type: "use-absorbed-device", itemId: "body.instance", targets: [] }]);
  aim(); controller.reconcileStatus(projected({ slot: 9, item: { id: "replacement", usable: true, useTargetSpec: spec } }));
  assert.equal(state.targeting, undefined); assert.equal(commands.length, 1, "reconciliation never sends stale use");
  aim(); controller.cancelTargeting(false); assert.equal(commands.length, 1, "loading/reset is not a use cancellation");
});

test("a pending Produce Mana effect opens mandatory direction targeting", () => {
  const state = new AppState();
  const announcements = [];
  const controller = new InputController({
    state,
    dom: {},
    localization: {},
    window: {},
    getInputPreset: () => "roguelike",
    getZoom: () => 1,
    whenIdle: async () => {},
    dispatch: async () => {},
    describeLook: () => "",
    openObjectList: () => {},
    openMogaminator: () => {},
    onLookOrTargeting: () => {},
    onLookFocusChange: () => {},
    announce: (key) => announcements.push(key),
  });
  const update = {
    mapScale: "local",
    worldTravelDestination: null,
    width: 96,
    height: 33,
    floorId: "core.floor.wilderness",
    player: {
      position: { x: 48, y: 16 },
      pendingMutationDirection: {
        mutationId: "rfb.mutation.prod-mana",
        resting: false,
      },
    },
  };

  controller.reconcileStatus(update);

  assert.equal(state.targetingIntent?.type, "mutation-direction");
  assert.deepEqual(state.targeting?.spec.modes, ["direction"]);
  assert.deepEqual(announcements, ["message-mutation-direction-required"]);
});

for (const [abilityId, message] of [
  ["demo.ability.nature-natures-wrath", "message-ability-direction-required"],
  ["demo.ability.chaos-call-chaos", "message-chaos-direction-required"],
  ["demo.ability.trump-shuffle", "message-trump-direction-required"],
]) {
  test(`a pending ${abilityId} branch opens ability direction targeting`, () => {
    const state = new AppState();
    const announcements = [];
    const controller = new InputController({
      state,
      dom: {},
      localization: {},
      window: {},
      getInputPreset: () => "roguelike",
      whenIdle: async () => {},
      getZoom: () => 1,
      dispatch: async () => {},
      describeLook: () => "",
      openObjectList: () => {},
      openMogaminator: () => {},
      onLookOrTargeting: () => {},
      onLookFocusChange: () => {},
      announce: (key) => announcements.push(key),
    });
    const update = {
      mapScale: "local",
      worldTravelDestination: null,
      width: 96,
      height: 33,
      floorId: "core.floor.wilderness",
      player: {
        position: { x: 48, y: 16 },
        pendingAbilityDirection: {
          abilityId,
          branchRoll: 6,
        },
      },
    };

    controller.reconcileStatus(update);

    assert.equal(state.targetingIntent?.type, "ability-direction");
    assert.deepEqual(state.targeting?.spec.modes, ["direction"]);
    assert.deepEqual(announcements, [message]);
  });
}

test("saved Wonder prompts once per session, validates a symbol and can cancel", () => {
  for (const answer of ["q", null]) {
    const state = new AppState();
    state.status = { mapScale: "local", width: 96, height: 33, floorId: "floor",
      player: { position: { x: 10, y: 10 }, pendingAbilityGlyph: { castResolution: {} } } };
    const timers = [];
    const commands = [];
    const answers = ["two symbols", answer];
    const controller = new InputController({ state, dom: {},
      localization: { format: key => key },
      window: { setTimeout: fn => timers.push(fn), prompt: () => answers.shift() },
      getInputPreset: () => "vi", getZoom: () => 1,
      dispatch: async command => commands.push(command),
      describeLook: () => "", openObjectList() {}, openMogaminator() {},
      onLookFocusChange() {}, announce() {},
    });
    controller.reconcileStatus(state.status);
    controller.reconcileStatus(state.status);
    assert.equal(timers.length, 1);
    assert.equal(commands.length, 0);
    controller.resetSession();
    controller.reconcileStatus(state.status);
    timers[0]();
    assert.equal(commands.length, 0, "the old session cannot open or dispatch a glyph prompt");
    controller.reconcileStatus(state.status);
    assert.equal(timers.length, 2, "the old callback cannot clear the new session's pending prompt");
    timers[1]();
    assert.deepEqual(commands, [{ type: "resolve-ability-glyph", glyph: answer }]);
  }
});

test("manual g delegates to the item picker instead of dispatching automatic pickup", t => {
  installElementIdentities(t);
  for (const preset of ["original", "roguelike"]) {
    const h = continuousHarness("local", undefined, preset);
    h.emit("keydown", { key: "g" });
    assert.deepEqual(h.shortcuts, ["pickup"]);
    assert.equal(h.requests.length, 0);
    h.controller.dispose();
  }
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
    window: globalThis,
    getInputPreset: () => "roguelike",
    getZoom: () => 1,
    whenIdle: async () => {},
    dispatch: async (command) => {
      commands.push(command);
      state.status = updates.shift();
      return "applied";
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
  assert.equal(autoGetStopsAfterStep(before, { ...moved, player: { ...moved.player, pendingDuelist: { type: "block-teleport" } } }, target), true);
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
