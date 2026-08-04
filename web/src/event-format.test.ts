// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { createPresentationFormatter } from "./event-format.ts";
import { Localization } from "./localization.ts";

const localization = new Localization("en-US", {
  "en-US": readLocale("en-US"),
  "zh-CN": readLocale("zh-CN"),
});
const state = {
  currentInventory: [],
  currentEquipment: [],
  currentStatus: undefined,
};
const formatter = createPresentationFormatter(localization, () => state, {
  formatAttributeValueArgument: (value) => value ?? "?",
  formatTenthsPoundArgument: (value) => value ?? "?",
  itemCurseSeverityName: () => "?",
});

test("item event formatting follows projected knowledge and locale changes", () => {
  const event = {
    kind: "item.pickup",
    messageKey: "item-pickup-success",
    args: { target: "demo.item.luminous-shard", quantity: "3" },
  };

  assert.equal(formatter.formatEvent(event), "You pick up unfamiliar pale shard ×3.");
  state.currentInventory = [
    {
      kindId: "demo.item.luminous-shard",
      displayNameKey: "item-demo-luminous-shard-name",
    },
  ];
  assert.equal(formatter.formatEvent(event), "You pick up luminous shard ×3.");

  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你将 3 个发光碎片收入了背包。");
  localization.setLocale("en-US");
});

test("item names preserve observable base kinds before property identification", () => {
  assert.equal(
    formatter.visibleItemName(
      "item-demo-unfamiliar-potion-name",
      "demo.item.light-healing-potion",
    ),
    "unfamiliar potion",
  );
  assert.equal(
    formatter.visibleItemName("item-demo-chain-mail-name", "demo.item.chain-mail"),
    "Chain Mail",
  );
});

test("gold events report gained balance and monster drops", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "gold.pickup",
      messageKey: "gold-pickup-success",
      args: { amount: "37", balance: "412" },
    }),
    "You collect 37 gold (412 total).",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "gold.drop",
      messageKey: "gold-drop",
      args: { source: "demo.actor.small-kobold", amount: "19" },
    }),
    "Small Kobold drops 19 gold.",
  );
});

test("shop events report localized item names, totals, balances, and rejection reasons", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "shop.purchase",
      messageKey: "shop-purchase-success",
      args: {
        target: "demo.item.ration-of-food",
        quantity: "2",
        totalPrice: "6",
        balance: "241",
      },
    }),
    "Purchased 2 Ration of Food for 6 gold. Balance: 241.",
  );
  localization.setLocale("zh-CN");
  assert.equal(
    formatter.formatEvent({
      kind: "shop.sale",
      messageKey: "shop-sale-success",
      args: {
        target: "demo.item.wooden-torch",
        quantity: "1",
        totalPrice: "1",
        balance: "242",
      },
    }),
    "出售了 1 件木制火把，获得 1 金币。余额：242。",
  );
  localization.setLocale("en-US");
});

test("food events report eating hunger changes fainting and starvation", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-food",
      messageKey: "item-use-food",
      args: {
        target: "demo.item.ration-of-food",
        nameKey: "item-demo-ration-of-food-name",
        amount: "5000",
        nutrition: "14999",
      },
    }),
    "You eat Ration of Food, restoring 5000 food (14999 / 15000).",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "hunger.state-changed",
      messageKey: "hunger-state-changed",
      args: { from: "normal", to: "hungry", nutrition: "1990" },
    }),
    "You are now Hungry.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "hunger.fainted",
      messageKey: "hunger-fainted",
      args: { duration: "3" },
    }),
    "Hunger makes you faint for 3 ticks.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "hunger.starvation-damage",
      messageKey: "hunger-starvation-damage",
      args: { damage: "7" },
    }),
    "Starvation deals 7 damage.",
  );
});

test("damage event formatting preserves typed resistance outcomes", () => {
  const event = {
    kind: "combat.hit",
    messageKey: "combat-player-hit",
    args: { target: "demo.actor.echo-hound", damage: "10" },
    outcome: {
      type: "damage",
      resolution: {
        rawDamage: 10,
        armorReduction: 0,
        resistanceAdjustment: 3,
        finalDamage: 7,
        damageType: "fire",
        resistance: "resistant",
      },
    },
  };

  assert.equal(
    formatter.formatEvent(event),
    "You hit echo hound for 7 fire damage (3 resisted).",
  );
  localization.setLocale("zh-CN");
  assert.equal(
    formatter.formatEvent(event),
    "你击中了回声猎犬，造成 7 点火焰伤害（抵抗了 3 点）。",
  );
  localization.setLocale("en-US");
});

test("Warrens transitions name the Outpost and stairs without legacy Echo text", () => {
  state.currentWorldId = "demo.world.warrens-journey";
  const event = {
    kind: "floor.transition",
    messageKey: "floor-transition",
    args: {
      from: "demo.floor.surface",
      to: "demo.floor.warrens-depth-1",
    },
  };

  assert.equal(
    formatter.formatEvent(event),
    "You leave Outpost and enter Warrens.",
  );
  assert.equal(formatter.contentName("demo.terrain.stairs-down"), "descending stairs");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你离开了前哨站，进入兽穴。");
  assert.equal(formatter.contentName("demo.terrain.stairs-up"), "向上楼梯");
  localization.setLocale("en-US");
});

function readLocale(locale: "en-US" | "zh-CN"): string[] {
  return ["ui.ftl", "game.ftl", "content.ftl"].map((file) =>
    readFileSync(new URL(`../../locales/${locale}/${file}`, import.meta.url), "utf8"),
  );
}
