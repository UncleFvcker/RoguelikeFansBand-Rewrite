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
  assert.equal(dom.inventoryInscription.id, "inventory-inscription");
  assert.equal(dom.inventoryInscribe.id, "inventory-inscribe");
  assert.equal(dom.inventoryDestroy.id, "inventory-destroy");
  assert.equal(dom.nativeSaveList.id, "native-save-list");
  assert.equal(dom.journeyDungeonName.id, "journey-dungeon-name");
  assert.equal(dom.journeyDepth.id, "journey-depth");
  assert.equal(dom.journeyBoss.id, "journey-boss");
  assert.equal(dom.journeyResult.id, "journey-result");
  assert.equal(dom.resultRestart.id, "result-restart");
  assert.equal(dom.resultExit.id, "result-exit");
  assert.equal(dom.lookModeToggle.id, "look-mode-toggle");
  assert.equal(dom.traverseStairs.id, "traverse-stairs");
  assert.equal(dom.healthMeterFill.id, "health-meter-fill");
  assert.equal(dom.goldValue.id, "gold-value");
  assert.equal(dom.nutritionValue.id, "nutrition-value");
  assert.equal(dom.lightValue.id, "light-value");
  assert.equal(dom.nearbyList.id, "nearby-list");
  assert.equal(dom.virtueList.id, "virtue-list");
  assert.equal(dom.mutationList.id, "mutation-list");
  assert.equal(dom.summonCommandButtons["keep-distance"].id, "summon-command-keep-distance");
  assert.equal(dom.dismissPets.id, "dismiss-pets");
  assert.equal(elements.size, 105);
  assert.equal(Object.isFrozen(dom), true);
  assert.equal(Object.isFrozen(dom.summonCommandButtons), true);
});

test("the application DOM registry fails fast when a required element is missing", () => {
  assert.throws(
    () => createAppDom({ getElementById: () => null }),
    /Missing element #map-host/,
  );
});
