// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  buildObjectListEntries,
  nextEntryStartingWith,
  objectListMovementForKey,
} from "./object-list.ts";

const visibility = new Map([
  ["2,1", "remembered"],
  ["7,7", "hidden"],
  ["3,1", "visible"],
  ["4,1", "remembered"],
  ["5,1", "hidden"],
]);
const glyphs = new Map([
  ["demo.terrain.general-store-entrance", "1"],
  ["demo.terrain.home-entrance", "8"],
  ["demo.terrain.stairs-down", ">"],
  ["demo.terrain.task-rift", ">"],
  ["demo.item.ration", ","],
]);

function projection(includeStairs = false, floorId = "demo.floor.warrens-depth-1") {
  return {
    playerPosition: { x: 1, y: 1 },
    floorId,
    cells: [
      { position: { x: 3, y: 1 }, terrainId: "demo.terrain.stairs-down" },
      { position: { x: 4, y: 1 }, terrainId: "demo.terrain.task-rift" },
      { position: { x: 5, y: 1 }, terrainId: "demo.terrain.archive-rift" },
    ],
    shops: [
      {
        id: "general-store",
        nameKey: "shop-general-store",
        entrancePosition: { x: 2, y: 1 },
        entranceTerrainId: "demo.terrain.general-store-entrance",
      },
    ],
    homes: [
      {
        id: "home",
        nameKey: "home-name",
        entrancePosition: { x: 7, y: 7 },
        entranceTerrainId: "demo.terrain.home-entrance",
      },
    ],
    taskServices: [],
    items: [
      {
        id: "ration.1",
        kindId: "demo.item.ration",
        displayNameKey: "item-ration",
        position: { x: 6, y: 1 },
        quantity: 2,
      },
    ],
    includeStairs,
    visibilityAt: (position) => visibility.get(`${position.x},${position.y}`),
    glyphFor: (id) => glyphs.get(id),
    localize: (key) => `l10n:${key}`,
    contentName: (id) => `content:${id}`,
    visibleItemName: (key) => `item:${key}`,
  };
}

test("known-object entries reuse discovered facilities, entrances, and items", () => {
  const entries = buildObjectListEntries(projection());

  assert.deepEqual(
    entries.map(({ id, category }) => [id, category]),
    [
      ["shop:general-store", "interesting"],
      ["terrain:4,1", "interesting"],
      ["item:ration.1", "items"],
    ],
  );
  assert.equal(entries[0].name, "l10n:shop-general-store");
  assert.deepEqual(
    { offsetX: entries[0].offsetX, offsetY: entries[0].offsetY },
    { offsetX: 1, offsetY: 0 },
  );
  assert.equal(entries[2].quantity, 2);
});

test("equal-position object sorting uses stable instance ids", () => {
  const options = projection();
  options.items = [
    options.items[0],
    { ...options.items[0], id: "ration.0" },
  ];
  assert.deepEqual(
    buildObjectListEntries(options)
      .filter((entry) => entry.category === "items")
      .map((entry) => entry.id),
    ["item:ration.0", "item:ration.1"],
  );
});

test("S-style stair toggling adds explored stairs without revealing hidden terrain", () => {
  const entries = buildObjectListEntries(projection(true));
  assert.deepEqual(
    entries.filter((entry) => entry.category === "interesting").map((entry) => entry.id),
    ["shop:general-store", "terrain:3,1", "terrain:4,1"],
  );
});

test("an explored surface stair is treated as a dungeon entrance by default", () => {
  const entries = buildObjectListEntries(projection(false, "demo.floor.surface"));
  assert.equal(entries.some((entry) => entry.id === "terrain:3,1"), true);
});

test("list navigation supports arrows, paging, ends, and cycling first letters", () => {
  assert.equal(objectListMovementForKey("ArrowLeft"), -1);
  assert.equal(objectListMovementForKey("PageDown"), 10);
  assert.equal(objectListMovementForKey("Home"), "start");
  assert.equal(objectListMovementForKey("Escape"), undefined);
  assert.equal(
    nextEntryStartingWith([{ name: "axe" }, { name: "book" }, { name: "amulet" }], 0, "a"),
    2,
  );
});
