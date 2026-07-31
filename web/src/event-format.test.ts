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

function readLocale(locale: "en-US" | "zh-CN"): string[] {
  return ["ui.ftl", "game.ftl", "content.ftl"].map((file) =>
    readFileSync(new URL(`../../locales/${locale}/${file}`, import.meta.url), "utf8"),
  );
}
