// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { presentationLight, RenderWorld } from "./render-world.ts";

test("presentation light is independent from terrain semantics", () => {
  const world = new RenderWorld(3, 1);
  const snapshot = snapshotFixture();
  snapshot.width = 3;
  snapshot.cells = [
    cell(0, 0),
    cell(1, 0, "demo.actor.player.1"),
    { ...cell(2, 0), terrainId: "demo.terrain.wall" },
  ];
  snapshot.player = player(1, 0);
  const cells = world.applySnapshot(snapshot);
  assert.deepEqual(cells[0].light, cells[2].light);
  assert.equal(presentationLight(snapshot.player.position, snapshot.player.position).intensity, 1);
});

test("render world keeps item and actor layers separate", () => {
  const world = new RenderWorld(2, 1);
  const cells = world.applySnapshot(snapshotFixture());
  assert.equal(cells[0].terrainId, "demo.terrain.floor");
  assert.equal(cells[0].itemKindId, "demo.item.luminous-shard");
  assert.equal(cells[0].actorKindId, "demo.actor.explorer");
});

test("player movement dirties only the union of old and new light footprints", () => {
  const world = new RenderWorld(20, 20);
  world.applySnapshot(largeSnapshotFixture());
  const dirty = world.applyUpdate({
    baseRevision: 0,
    revision: 1,
    turn: 1,
    commandSeq: 1,
    events: [],
    changedCells: [cell(3, 3), cell(4, 3, "demo.actor.player.1")],
    player: player(4, 3),
    entities: [],
    items: [],
    inventory: [],
    removedEntities: [],
    stateHash: "hash",
  });
  assert.ok(dirty.length > 2);
  assert.ok(dirty.length < 400);
  assert.equal(new Set(dirty.map((entry) => entry.index)).size, dirty.length);
});

test("visibility changes use a separate render delta", () => {
  const world = new RenderWorld(2, 1);
  world.applySnapshot(snapshotFixture());
  const dirty = world.applyVisibilityDelta([
    { position: { x: 1, y: 0 }, visibility: "remembered" },
  ]);
  assert.equal(dirty.length, 1);
  assert.equal(dirty[0].visibility, "remembered");
  assert.ok(
    world.applySnapshot(snapshotFixture()).every((cell) => cell.visibility === "visible"),
  );
});

function snapshotFixture() {
  return {
    protocolVersion: "1.2",
    revision: 0,
    turn: 0,
    lastCommandSeq: 0,
    width: 2,
    height: 1,
    cells: [
      cell(0, 0, "demo.actor.player.1", "demo.item.luminous-shard.1"),
      cell(1, 0),
    ],
    player: player(0, 0),
    entities: [],
    items: [
      {
        id: "demo.item.luminous-shard.1",
        kindId: "demo.item.luminous-shard",
        position: { x: 0, y: 0 },
        quantity: 1,
      },
    ],
    inventory: [],
    contentId: "content",
    contentHash: "hash",
    contentVisuals: [],
    worldId: "world",
    stateHash: "state",
  };
}

function largeSnapshotFixture() {
  const cells = [];
  for (let y = 0; y < 20; y += 1) {
    for (let x = 0; x < 20; x += 1) {
      cells.push(cell(x, y, x === 3 && y === 3 ? "demo.actor.player.1" : undefined));
    }
  }
  return { ...snapshotFixture(), width: 20, height: 20, cells, player: player(3, 3) };
}

function cell(x, y, actorId, itemId) {
  return {
    position: { x, y },
    terrainId: "demo.terrain.floor",
    itemId,
    actorId,
  };
}

function player(x, y) {
  return {
    id: "demo.actor.player.1",
    kindId: "demo.actor.explorer",
    position: { x, y },
    hp: 10,
    maxHp: 10,
  };
}
