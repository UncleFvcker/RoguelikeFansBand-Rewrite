// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  absorbableItemCandidates,
  formatTenthsPound,
  itemIdentificationMessageKey,
  itemTargetCandidates,
  parseDropQuantity,
  selectedRechargingItems,
} from "./inventory-panel.ts";

test("device absorption candidates include the pack and only devices underfoot", () => {
  const state = {
    inventory: [
      { id: "pack", kindId: "item.pack", displayNameKey: "pack", absorbable: true },
      { id: "food", kindId: "item.food", displayNameKey: "food", absorbable: false },
    ],
    status: {
      player: { position: { x: 4, y: 7 } },
      items: [
        {
          id: "floor",
          kindId: "item.floor",
          displayNameKey: "floor",
          position: { x: 4, y: 7 },
          absorbable: true,
        },
        {
          id: "distant",
          kindId: "item.distant",
          displayNameKey: "distant",
          position: { x: 5, y: 7 },
          absorbable: true,
        },
      ],
    },
  };

  assert.deepEqual(
    absorbableItemCandidates(state, (displayNameKey) => displayNameKey),
    [
      { id: "pack", label: "pack" },
      { id: "floor", label: "floor" },
    ],
  );
});

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

test("item targeting includes only ground items at the player's feet", () => {
  const state = {
    inventory: [{ id: "pack", kindId: "item.pack", displayNameKey: "pack" }],
    equipment: [{ id: "worn", kindId: "item.worn", displayNameKey: "worn" }],
    status: {
      player: { position: { x: 4, y: 7 } },
      items: [
        {
          id: "floor",
          kindId: "item.floor",
          displayNameKey: "floor",
          position: { x: 4, y: 7 },
        },
        {
          id: "distant",
          kindId: "item.distant",
          displayNameKey: "distant",
          position: { x: 5, y: 7 },
        },
      ],
    },
  };

  assert.deepEqual(
    itemTargetCandidates(state, "worn", (name) => name),
    [
      { id: "pack", label: "pack" },
      { id: "floor", label: "floor" },
    ],
  );
});
