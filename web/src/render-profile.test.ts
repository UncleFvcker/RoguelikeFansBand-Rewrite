// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  RENDER_PROFILE_HEIGHT,
  RENDER_PROFILE_WIDTH,
  createDynamicProfileUpdates,
  createRendererProfileCells,
  createTerrainProfileUpdates,
  summarizeFrameIntervals,
} from "./render-profile-scenario.ts";

test("large original render profile is deterministic and covers the whole map", () => {
  const first = createRendererProfileCells();
  const second = createRendererProfileCells();
  assert.equal(first.length, RENDER_PROFILE_WIDTH * RENDER_PROFILE_HEIGHT);
  assert.deepEqual(first, second);
  assert.deepEqual(first[0], {
    index: 0,
    x: 0,
    y: 0,
    terrainId: "profile.terrain.wall",
    itemKindId: "profile.item.light",
    actorKindId: "profile.actor.mote",
    visibility: "visible",
    light: { color: 0xffb060, intensity: 0 },
  });
  assert.equal(first.at(-1)?.x, RENDER_PROFILE_WIDTH - 1);
  assert.equal(first.at(-1)?.y, RENDER_PROFILE_HEIGHT - 1);
});

test("profile updates are sparse, stable, and keep indexes unique", () => {
  const source = createRendererProfileCells();
  const dynamic = createDynamicProfileUpdates(source);
  const terrain = createTerrainProfileUpdates(source);
  assert.equal(dynamic.length, 256);
  assert.equal(terrain.length, 96);
  assert.equal(new Set(dynamic.map((cell) => cell.index)).size, dynamic.length);
  assert.equal(new Set(terrain.map((cell) => cell.index)).size, terrain.length);
  for (const cell of dynamic) {
    assert.equal(cell.terrainId, source[cell.index]?.terrainId);
  }
  for (const cell of terrain) {
    assert.notEqual(cell.terrainId, source[cell.index]?.terrainId);
  }
});

test("frame timing summary uses stable nearest-rank percentiles", () => {
  assert.deepEqual(summarizeFrameIntervals([]), {
    sampleCount: 0,
    medianMs: 0,
    p95Ms: 0,
    maxMs: 0,
  });
  assert.deepEqual(summarizeFrameIntervals([20, 10, 40, 30]), {
    sampleCount: 4,
    medianMs: 20,
    p95Ms: 40,
    maxMs: 40,
  });
});
