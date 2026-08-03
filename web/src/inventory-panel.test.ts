// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  formatTenthsPound,
  itemIdentificationMessageKey,
  parseDropQuantity,
  selectedRechargingItems,
} from "./inventory-panel.ts";

test("inventory quantity parsing preserves whole-stack boundaries", () => {
  assert.equal(parseDropQuantity("1", 3), 1);
  assert.equal(parseDropQuantity("3", 3), 3);
  assert.equal(parseDropQuantity("0", 3), undefined);
  assert.equal(parseDropQuantity("1.5", 3), undefined);
  assert.equal(parseDropQuantity("4", 3), undefined);
});

test("equipment identification distinguishes quality appraisal from ego knowledge", () => {
  assert.equal(
    itemIdentificationMessageKey("unexamined", 0),
    "item-identification-unexamined",
  );
  assert.equal(
    itemIdentificationMessageKey("appraised", 0),
    "item-identification-appraised",
  );
  assert.equal(
    itemIdentificationMessageKey("identified", 0),
    "item-identification-identified-ordinary",
  );
  assert.equal(
    itemIdentificationMessageKey("identified", 1),
    "item-identification-identified-ego",
  );
});

test("inventory recharge pairing remains order-independent", () => {
  const target = { id: "target", requiresRechargeTargets: true };
  const source = { id: "source", canSupplyRecharge: true };

  assert.deepEqual(selectedRechargingItems([source, target]), { item: target, source });
  assert.equal(selectedRechargingItems([target]), undefined);
  assert.equal(formatTenthsPound(123), "12.3");
});
