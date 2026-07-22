// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  terrainInteractionCommand,
  terrainInteractionForDirection,
  terrainInteractionsForMode,
  terrainInteractionModeForKey,
  terrainSearchCommandForKey,
} from "./terrain-interaction.ts";

test("door interaction keys enter stable open, close, and bash modes", () => {
  assert.equal(terrainInteractionModeForKey("o"), "open-door");
  assert.equal(terrainInteractionModeForKey("O"), "open-door");
  assert.equal(terrainInteractionModeForKey("c"), "close-door");
  assert.equal(terrainInteractionModeForKey("b"), undefined);
  assert.equal(terrainInteractionModeForKey("B"), "bash-door");
  assert.equal(terrainInteractionModeForKey("d"), undefined);
  assert.equal(terrainInteractionModeForKey("D"), "disarm-trap");
  assert.equal(terrainInteractionModeForKey("t"), undefined);
  assert.equal(terrainInteractionModeForKey("T"), "dig-terrain");
  assert.equal(terrainInteractionModeForKey("x"), undefined);
});

test("search uses uppercase S without stealing south movement", () => {
  assert.equal(terrainSearchCommandForKey("s"), undefined);
  assert.deepEqual(terrainSearchCommandForKey("S"), { type: "search" });
});

test("door interaction mode produces a directional core command", () => {
  assert.deepEqual(terrainInteractionCommand("open-door", "east"), {
    type: "open-door",
    direction: "east",
  });
  assert.deepEqual(terrainInteractionCommand("close-door", "north-west"), {
    type: "close-door",
    direction: "north-west",
  });
  assert.deepEqual(terrainInteractionCommand("bash-door", "south"), {
    type: "bash-door",
    direction: "south",
  });
  assert.deepEqual(terrainInteractionCommand("disarm-trap", "north"), {
    type: "disarm-trap",
    direction: "north",
  });
  assert.deepEqual(terrainInteractionCommand("dig-terrain", "south-east"), {
    type: "dig-terrain",
    direction: "south-east",
  });
});

test("authoritative terrain interactions are filtered by mode and direction", () => {
  const interactions = [
    {
      kind: "open-door",
      direction: "east",
      position: { x: 10, y: 4 },
      terrainId: "demo.terrain.door-locked",
      requiresCheck: true,
      available: true,
    },
    {
      kind: "bash-door",
      direction: "east",
      position: { x: 10, y: 4 },
      terrainId: "demo.terrain.door-locked",
      requiresCheck: true,
      available: false,
      unavailableReason: "occupied-by-actor",
    },
  ];

  assert.deepEqual(terrainInteractionsForMode(interactions, "open-door"), [
    interactions[0],
  ]);
  assert.equal(
    terrainInteractionForDirection(interactions, "bash-door", "east"),
    interactions[1],
  );
  assert.equal(
    terrainInteractionForDirection(interactions, "close-door", "east"),
    undefined,
  );
});
