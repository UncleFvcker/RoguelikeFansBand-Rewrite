// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import test from "node:test";
import assert from "node:assert/strict";
import { knownAimPath, MapDisplay } from "./map-display.ts";
import { AppState } from "./app-state.ts";

test("aiming centerline ends at unknown terrain, known obstacles and range", () => {
  const origin = { x: 0, y: 0 }, target = { x: 5, y: 0 };
  assert.deepEqual(knownAimPath(origin, target, 10, p => p.x < 3, () => true), [{ x: 1, y: 0 }, { x: 2, y: 0 }]);
  assert.deepEqual(knownAimPath(origin, target, 10, () => true, p => p.x < 2), [{ x: 1, y: 0 }, { x: 2, y: 0 }]);
  assert.equal(knownAimPath(origin, target, 1, () => true, () => true).length, 1);
  assert.deepEqual(knownAimPath(origin, origin, 10, () => true, () => true), []);
});

test("coverage alert fires once per crossing independently of stop policy and ignores floor transitions", () => {
  const state = new AppState(); state.mode = "playing"; state.display.alertTrapDetect = true;
  const covered = { x: 1, y: 1 }, outside = { x: 2, y: 1 };
  state.status = { turn: 0, floorId: "a", mapScale: "local", travelOptions: { disturbTrapDetect: false },
    player: { position: covered, trapDetectedGrids: [covered] } };
  const node = () => ({ classList: { add() {} }, setAttribute() {}, replaceChildren() {} });
  const host = { dataset: {}, ownerDocument: { createElementNS: node }, append() {} };
  const display = new MapDisplay();
  assert.equal(display.render(host, state, 1), false);
  state.status = { ...state.status, turn: 1, player: { ...state.status.player, position: outside } };
  assert.equal(display.render(host, state, 1), true);
  assert.equal(display.render(host, state, 1), false);
  state.status.player.position = covered; display.render(host, state, 1);
  state.status.floorId = "b"; state.status.player.position = outside;
  assert.equal(display.render(host, state, 1), false);
});
