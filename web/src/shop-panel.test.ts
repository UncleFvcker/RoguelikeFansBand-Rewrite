// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { Localization } from "./localization.ts";
import {
  calculateShopTransactionPreview,
  equippedLightText,
  parseShopQuantity,
  shopTransactionReason,
  stayAtInnCommand,
  travelFromInnCommand,
} from "./shop-panel.ts";

const localization = new Localization("en-US", {
  "en-US": readLocale("en-US"),
  "zh-CN": readLocale("zh-CN"),
});

test("shop quantity parsing accepts only bounded positive integers", () => {
  assert.equal(parseShopQuantity("1", 4), 1);
  assert.equal(parseShopQuantity("4", 4), 4);
  assert.equal(parseShopQuantity("0", 4), undefined);
  assert.equal(parseShopQuantity("5", 4), undefined);
  assert.equal(parseShopQuantity("1.5", 4), undefined);
  assert.equal(parseShopQuantity("", 4), undefined);
});

test("shop previews show post-transaction gold and carried weight without mutating state", () => {
  assert.deepEqual(calculateShopTransactionPreview("buy", 2, 3, 10, 247, 302), {
    totalPrice: 6,
    goldAfter: 241,
    weightAfterTenthsPound: 322,
  });
  assert.deepEqual(calculateShopTransactionPreview("sell", 3, 2, 10, 241, 322), {
    totalPrice: 6,
    goldAfter: 247,
    weightAfterTenthsPound: 292,
  });
});

test("content-configured inns dispatch narrow stay and visited-town travel commands", () => {
  assert.deepEqual(stayAtInnCommand({ id: "demo.shop.anambar-inn", innStayCost: 25 }), {
    type: "stay-at-inn",
    facilityId: "demo.shop.anambar-inn",
  });
  assert.equal(
    stayAtInnCommand({ id: "demo.shop.outpost-general-store", innStayCost: undefined }),
    undefined,
  );
  const inn = {
    id: "demo.shop.outpost-white-horse",
    innTravelDestinations: [
      { townId: "demo.town.anambar", townNameKey: "town-demo-anambar-name", cost: 500 },
    ],
  };
  assert.deepEqual(travelFromInnCommand(inn, "demo.town.anambar"), {
    type: "travel-from-inn",
    facilityId: "demo.shop.outpost-white-horse",
    destinationTownId: "demo.town.anambar",
  });
  assert.equal(travelFromInnCommand(inn, "demo.town.unknown"), undefined);
});

test("shop rejection reasons and equipped light summaries are localized", () => {
  assert.equal(shopTransactionReason("insufficient-gold", localization), "Not enough gold.");
  assert.equal(shopTransactionReason("future-reason", localization), "The transaction is unavailable.");
  assert.equal(equippedLightText([], localization, () => "?"), "None equipped");
  assert.equal(
    equippedLightText(
      [{
        kindId: "demo.item.brass-lantern",
        slotId: "light",
        fuel: { kind: "lantern", current: 4200, maximum: 7500, lightRadius: 2 },
      }],
      localization,
      () => "Brass Lantern",
    ),
    "Brass Lantern, fuel 4,200/7,500",
  );

  localization.setLocale("zh-CN");
  assert.equal(equippedLightText([], localization, () => "?"), "未装备");
  localization.setLocale("en-US");
});

function readLocale(locale: "en-US" | "zh-CN"): string[] {
  return ["ui.ftl", "game.ftl", "content.ftl"].map((file) =>
    readFileSync(new URL(`../../locales/${locale}/${file}`, import.meta.url), "utf8"),
  );
}
