// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { createAppDom } from "./app-dom.ts";

test("the application DOM registry is immutable and preserves stable element IDs", () => {
  const elements = new Map();
  const document = {
    getElementById(id) {
      const found = { id };
      elements.set(id, found);
      return found;
    },
  };

  const dom = createAppDom(document);

  assert.equal(dom.mapHost.id, "map-host");
  assert.equal(dom.inventoryDropQuantity.id, "inventory-drop-quantity");
  assert.equal(dom.nativeSaveList.id, "native-save-list");
  assert.equal(dom.journeyDungeonName.id, "journey-dungeon-name");
  assert.equal(dom.journeyDepth.id, "journey-depth");
  assert.equal(dom.journeyBoss.id, "journey-boss");
  assert.equal(dom.lookModeToggle.id, "look-mode-toggle");
  assert.equal(dom.summonCommandButtons["keep-distance"].id, "summon-command-keep-distance");
  assert.equal(elements.size, 73);
  assert.equal(Object.isFrozen(dom), true);
  assert.equal(Object.isFrozen(dom.summonCommandButtons), true);
});

test("the application DOM registry fails fast when a required element is missing", () => {
  assert.throws(
    () => createAppDom({ getElementById: () => null }),
    /Missing element #map-host/,
  );
});
