// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  chunkIndexForCell,
  createRenderChunkLayout,
  updateTerrainChunkState,
  visibleRenderChunkIndexes,
} from "./render-chunks.ts";

test("20 by 20 maps use four clipped 16-cell chunks", () => {
  const layout = createRenderChunkLayout(20, 20);
  assert.equal(layout.chunksAcross, 2);
  assert.equal(layout.chunksDown, 2);
  assert.equal(layout.chunks.length, 4);
  assert.deepEqual(layout.chunks.at(-1), {
    index: 3,
    column: 1,
    row: 1,
    cellX: 16,
    cellY: 16,
    cellWidth: 4,
    cellHeight: 4,
  });
  assert.equal(chunkIndexForCell(0, 0, layout.chunksAcross), 0);
  assert.equal(chunkIndexForCell(8, 7, layout.chunksAcross), 0);
  assert.equal(chunkIndexForCell(19, 19, layout.chunksAcross), 3);
});

test("full-map mode keeps every chunk renderable", () => {
  const layout = createRenderChunkLayout(20, 20);
  const visible = visibleRenderChunkIndexes(layout.chunks, {
    x: 0,
    y: 0,
    zoom: 1,
    viewportWidth: 280,
    viewportHeight: 280,
    cullingEnabled: false,
  });
  assert.equal(visible.size, 4);
});

test("player-centered viewport culls chunks outside the camera with one-cell overscan", () => {
  const layout = createRenderChunkLayout(20, 20);
  const edge = visibleRenderChunkIndexes(layout.chunks, {
    x: 0,
    y: 0,
    zoom: 1,
    viewportWidth: 420,
    viewportHeight: 420,
    cullingEnabled: true,
  });
  assert.deepEqual([...edge], [0]);

  const followed = visibleRenderChunkIndexes(layout.chunks, {
    x: -28,
    y: 0,
    zoom: 1,
    viewportWidth: 420,
    viewportHeight: 420,
    cullingEnabled: true,
  });
  assert.deepEqual([...followed], [0, 1]);
});

test("culling converts the scaled viewport back into world coordinates", () => {
  const layout = createRenderChunkLayout(20, 20);
  const visible = visibleRenderChunkIndexes(layout.chunks, {
    x: -147,
    y: 0,
    zoom: 1.5,
    viewportWidth: 420,
    viewportHeight: 420,
    cullingEnabled: true,
  });
  assert.deepEqual([...visible], [0]);
});

test("terrain invalidation rebuilds only chunks whose terrain identity changed", () => {
  const terrainIds = new Array(400);
  const initial = updateTerrainChunkState(
    terrainIds,
    [
      { index: 0, x: 0, y: 0, terrainId: "floor" },
      { index: 16, x: 16, y: 0, terrainId: "wall" },
    ],
    2,
    4,
    true,
  );
  assert.deepEqual([...initial], [0, 1, 2, 3]);

  assert.equal(
    updateTerrainChunkState(
      terrainIds,
      [{ index: 0, x: 0, y: 0, terrainId: "floor" }],
      2,
      4,
      false,
    ).size,
    0,
  );
  assert.deepEqual(
    [
      ...updateTerrainChunkState(
        terrainIds,
        [{ index: 16, x: 16, y: 0, terrainId: "floor" }],
        2,
        4,
        false,
      ),
    ],
    [1],
  );
});
