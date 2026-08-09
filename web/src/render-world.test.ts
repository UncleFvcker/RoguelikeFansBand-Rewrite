// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { RenderWorld } from "./render-world.ts";

test("render world consumes authoritative light independently from terrain semantics", () => {
  const world = new RenderWorld(3, 1);
  const snapshot = snapshotFixture();
  snapshot.width = 3;
  snapshot.cells = [
    cell(0, 0),
    cell(1, 0, "demo.actor.player.1"),
    { ...cell(2, 0), terrainId: "demo.terrain.wall" },
  ];
  snapshot.visualCells = [
    visual(0, 0, "visible", 0x8ad9ff, 64),
    visual(1, 0, "visible", 0xffd7a3, 100),
    visual(2, 0, "visible", 0x8ad9ff, 64),
  ];
  snapshot.player = player(1, 0);
  const cells = world.applySnapshot(snapshot);
  assert.deepEqual(cells[0].light, cells[2].light);
  assert.equal(cells[1].light.intensity, 1);
});

test("render world keeps item and actor layers separate", () => {
  const world = new RenderWorld(2, 1);
  const cells = world.applySnapshot(snapshotFixture());
  assert.equal(cells[0].terrainId, "demo.terrain.floor");
  assert.equal(cells[0].itemKindId, "demo.item.ration-of-food");
  assert.equal(cells[0].actorKindId, "demo.actor.explorer");
});

test("render world resolves gold pile IDs to their stable visual IDs", () => {
  const world = new RenderWorld(2, 1);
  const snapshot = snapshotFixture();
  snapshot.cells[0] = cell(0, 0, "demo.actor.player.1", "generated.gold.7");
  snapshot.items = [];
  snapshot.goldPiles = [
    {
      id: "generated.gold.7",
      position: { x: 0, y: 0 },
      amount: 117,
      appearance: "rubies",
    },
  ];

  assert.equal(world.applySnapshot(snapshot)[0].itemKindId, "core.gold.rubies");
});

test("game updates dirty only authoritative cell and visual deltas", () => {
  const world = new RenderWorld(20, 20);
  world.applySnapshot(largeSnapshotFixture());
  const dirty = world.applyUpdate({
    baseRevision: 0,
    revision: 1,
    turn: 1,
    commandSeq: 1,
    events: [],
    changedCells: [cell(3, 3), cell(4, 3, "demo.actor.player.1")],
    changedVisualCells: [
      visual(3, 3, "remembered", 0xffd7a3, 72),
      visual(4, 3, "visible", 0xffd7a3, 100),
      visual(5, 3, "visible", 0xffd7a3, 80),
    ],
    player: player(4, 3),
    entities: [],
    items: [],
    goldPiles: [],
    inventory: [],
    equipment: [],
    removedEntities: [],
    stateHash: "hash",
  });
  assert.equal(dirty.length, 3);
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
  assert.deepEqual(
    world.applySnapshot(snapshotFixture()).map((cell) => cell.visibility),
    ["visible", "hidden"],
  );
});

test("remembered and hidden cells do not expose current occupants", () => {
  const world = new RenderWorld(2, 1);
  world.applySnapshot(snapshotFixture());
  const remembered = world.applyVisibilityDelta([
    { position: { x: 0, y: 0 }, visibility: "remembered" },
  ])[0];
  assert.equal(remembered.itemKindId, undefined);
  assert.equal(remembered.actorKindId, undefined);
});

test("hallucination scrambles occupants deterministically without randomness", () => {
  const snapshot = snapshotFixture();
  snapshot.worldTick = 10;
  snapshot.width = 3;
  snapshot.cells = [
    cell(0, 0, "demo.actor.player.1", "demo.item.shard.1"),
    cell(1, 0, "demo.actor.rat.1", "demo.item.ration.1"),
    cell(2, 0, "demo.actor.orc.1"),
  ];
  snapshot.visualCells = [
    visual(0, 0, "visible", 0xffffff, 100),
    visual(1, 0, "visible", 0xffffff, 100),
    visual(2, 0, "visible", 0xffffff, 100),
  ];
  snapshot.player.statuses = [{ kindId: "rfb.status.hallucination" }];
  snapshot.entities = [
    { id: "demo.actor.rat.1", kindId: "demo.actor.rat" },
    { id: "demo.actor.orc.1", kindId: "demo.actor.orc" },
  ];
  snapshot.items = [
    { id: "demo.item.shard.1", kindId: "demo.item.shard" },
    { id: "demo.item.ration.1", kindId: "demo.item.ration" },
  ];

  const first = new RenderWorld(3, 1).applySnapshot(snapshot);
  const repeated = new RenderWorld(3, 1).applySnapshot(snapshot);
  assert.deepEqual(repeated, first);
  assert.equal(first[0].actorKindId, "demo.actor.explorer");

  const world = new RenderWorld(3, 1);
  world.applySnapshot(snapshot);
  const next = world.applyUpdate({
    ...snapshot,
    baseRevision: 0,
    revision: 1,
    commandSeq: 1,
    worldTick: 11,
    events: [],
    changedCells: [],
    changedVisualCells: [],
    removedEntities: [],
  });
  assert.equal(next.length, 3);
  assert.notEqual(next[1].itemKindId, first[1].itemKindId);
});

function snapshotFixture() {
  return {
    protocolVersion: "1.5",
    revision: 0,
    turn: 0,
    worldTick: 0,
    lastCommandSeq: 0,
    width: 2,
    height: 1,
    cells: [
      cell(0, 0, "demo.actor.player.1", "demo.item.ration-of-food.1"),
      cell(1, 0),
    ],
    visualCells: [
      visual(0, 0, "visible", 0xffd7a3, 100),
      visual(1, 0, "hidden", 0xffffff, 28),
    ],
    player: player(0, 0),
    entities: [],
    items: [
      {
        id: "demo.item.ration-of-food.1",
        kindId: "demo.item.ration-of-food",
        position: { x: 0, y: 0 },
        quantity: 1,
      },
    ],
    goldPiles: [],
    inventory: [],
    equipment: [],
    contentId: "content",
    contentHash: "hash",
    contentVisuals: [],
    worldId: "world",
    stateHash: "state",
  };
}

function largeSnapshotFixture() {
  const cells = [];
  const visualCells = [];
  for (let y = 0; y < 20; y += 1) {
    for (let x = 0; x < 20; x += 1) {
      cells.push(cell(x, y, x === 3 && y === 3 ? "demo.actor.player.1" : undefined));
      visualCells.push(visual(x, y, "visible", 0xffd7a3, 50));
    }
  }
  return {
    ...snapshotFixture(),
    width: 20,
    height: 20,
    cells,
    visualCells,
    player: player(3, 3),
  };
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
    statuses: [],
  };
}

function visual(x, y, visibility, color, intensity) {
  return {
    position: { x, y },
    visibility,
    light: { color, intensity },
  };
}
