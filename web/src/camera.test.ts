// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { computeCameraOffset, computeFullMapScroll, parseZoomLevel } from "./camera.ts";

const base = {
  mode: "player-centered",
  worldWidth: 560,
  worldHeight: 560,
  viewportWidth: 420,
  viewportHeight: 420,
  cellSize: 28,
};

test("player-centered camera follows a player away from map edges", () => {
  assert.deepEqual(computeCameraOffset({ ...base, focus: { x: 8, y: 9 } }), {
    x: -28,
    y: -56,
  });
});

test("player-centered camera clamps at every map edge", () => {
  assert.deepEqual(computeCameraOffset({ ...base, focus: { x: 0, y: 0 } }), { x: 0, y: 0 });
  assert.deepEqual(computeCameraOffset({ ...base, focus: { x: 19, y: 19 } }), {
    x: -140,
    y: -140,
  });
});

test("camera centers a world that is smaller than the viewport", () => {
  assert.deepEqual(
    computeCameraOffset({
      ...base,
      focus: { x: 1, y: 1 },
      worldWidth: 84,
      worldHeight: 56,
    }),
    { x: 168, y: 182 },
  );
});

test("full-map mode never transforms the world", () => {
  assert.deepEqual(
    computeCameraOffset({ ...base, mode: "full-map", focus: { x: 19, y: 19 } }),
    { x: 0, y: 0 },
  );
});

test("full-map scrolling keeps a visible player stable", () => {
  assert.deepEqual(
    computeFullMapScroll({
      focus: { x: 8, y: 6 },
      worldWidth: 1848,
      worldHeight: 616,
      viewportWidth: 700,
      viewportHeight: 500,
      scrollX: 100,
      scrollY: 0,
      cellSize: 28,
    }),
    { x: 100, y: 0 },
  );
});

test("full-map scrolling follows a player beyond the safe viewport edge", () => {
  assert.deepEqual(
    computeFullMapScroll({
      focus: { x: 29, y: 20 },
      worldWidth: 1848,
      worldHeight: 616,
      viewportWidth: 700,
      viewportHeight: 420,
      scrollX: 0,
      scrollY: 0,
      cellSize: 28,
    }),
    { x: 196, y: 196 },
  );
});

test("full-map scrolling clamps at map edges and respects zoom", () => {
  assert.deepEqual(
    computeFullMapScroll({
      focus: { x: 19, y: 19 },
      worldWidth: 560,
      worldHeight: 560,
      viewportWidth: 420,
      viewportHeight: 420,
      scrollX: 0,
      scrollY: 0,
      cellSize: 28,
      zoom: 1.5,
    }),
    { x: 420, y: 420 },
  );
});

test("zoom scales the world and keeps the focus math deterministic", () => {
  assert.deepEqual(
    computeCameraOffset({ ...base, focus: { x: 8, y: 9 }, zoom: 1.5 }),
    { x: -147, y: -189 },
  );
});

test("zoom still clamps the scaled world at its far edge", () => {
  assert.deepEqual(
    computeCameraOffset({ ...base, focus: { x: 19, y: 19 }, zoom: 1.5 }),
    { x: -420, y: -420 },
  );
});

test("zoom persistence accepts only supported presets", () => {
  assert.equal(parseZoomLevel("1.25"), 1.25);
  assert.equal(parseZoomLevel("3"), 1);
  assert.equal(parseZoomLevel(null), 1);
});
