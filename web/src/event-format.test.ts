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

test("Fast Recovery uses the localized regeneration status name", () => {
  assert.equal(formatter.statusName("rfb.status.regeneration"), "regeneration");
  localization.setLocale("zh-CN");
  assert.equal(formatter.statusName("rfb.status.regeneration"), "再生");
  localization.setLocale("en-US");
});

test("mutation events use their authoritative projected names", () => {
  const gained = {
    kind: "mutation.gained",
    messageKey: "mutation-gained",
    args: { target: "rfb.mutation.spit-acid", name: "喷吐酸液" },
  };
  const lost = {
    kind: "mutation.lost",
    messageKey: "mutation-lost",
    args: { target: "rfb.mutation.spit-acid", name: "喷吐酸液" },
  };

  assert.equal(formatter.formatEvent(gained), "You gain the 喷吐酸液 mutation.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(lost), "你失去了变异“喷吐酸液”。");
  localization.setLocale("en-US");
});

test("effectless BEG actions remain visible without fake damage", () => {
  const event = {
    kind: "combat.monster-beg",
    messageKey: "combat-monster-beg",
    args: { source: "demo.actor.small-kobold" },
  };

  assert.equal(formatter.formatEvent(event), "Small Kobold begs you.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "小狗头人向你乞求。");
  localization.setLocale("en-US");
});

test("weapon proficiency growth localizes the canonical base weapon", () => {
  const event = {
    kind: "progress.weapon-proficiency-improved",
    messageKey: "weapon-proficiency-improved",
    args: { target: "demo.item.short-bow" },
  };

  assert.equal(formatter.formatEvent(event), "Your skill with Short Bow is improving.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你的短弓技能正在提升。");
  localization.setLocale("en-US");
});

test("mining proficiency growth uses the original bilingual message", () => {
  const event = {
    kind: "progress.mining-proficiency-improved",
    messageKey: "mining-proficiency-improved",
    args: {},
  };

  assert.equal(formatter.formatEvent(event), "Your mining proficiency has improved.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你的挖矿熟练度提升了。");
  localization.setLocale("en-US");
});

test("Polymorph's rare cure uses the authoritative bilingual message", () => {
  const event = {
    kind: "mutation.all-cured",
    messageKey: "mutation-all-cured",
    args: {},
  };

  assert.equal(formatter.formatEvent(event), "All of your mutations are cured.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你所有的变异都被治愈了。");
  localization.setLocale("en-US");
});

test("mutation ability events report the SP and HP actually paid", () => {
  const event = {
    kind: "ability.cast-failure",
    messageKey: "ability-cast-failure",
    args: { target: "rfb.ability.mutation.spit-acid" },
    outcome: {
      type: "ability-cast",
      resolution: {
        abilityId: "rfb.ability.mutation.spit-acid",
        resourceId: "demo.resource.mana",
        baseResourceCost: 7,
        resourceCost: 7,
        resourceBefore: 3,
        resourceAfter: 0,
        resourcePaid: 3,
        hpPaid: 4,
        failurePercent: 35,
        percentileRoll: 22,
        succeeded: false,
        proficiencyBefore: 0,
        proficiencyAfter: 0,
        proficiencyRank: "unskilled",
        castCount: 0,
        failCount: 0,
        cooldownBefore: 0,
        cooldownAfter: 0,
      },
    },
  };

  assert.equal(
    formatter.formatEvent(event),
    "Your attempt to cast Spit Acid fails (roll 22, failure 35%, cost 3 Mana + 4 HP).",
  );
  localization.setLocale("zh-CN");
  assert.equal(
    formatter.formatEvent(event),
    "你施放喷吐酸液失败了（掷骰 22，失败率 35%，消耗 3 点法力 + 4 点生命）。",
  );
  localization.setLocale("en-US");
});

test("Archer ammunition creation reports the generated kind and quantity", () => {
  const event = {
    kind: "ability.effects",
    messageKey: "ability-effects",
    args: { target: "demo.ability.archer-create-arrows", count: "1" },
    outcome: {
      type: "ability-effects",
      resolution: {
        targetEntityId: null,
        targetKindId: null,
        effects: [
          {
            type: "create-ammunition",
            effectIndex: 0,
            sourceItemId: "test.skeleton",
            sourcePosition: null,
            itemKindId: "demo.item.arrow",
            quantity: 7,
            destinationItemIds: ["generated.item.1"],
          },
        ],
      },
    },
  };

  assert.equal(formatter.formatEvent(event), "Create Arrows creates Arrow × 7.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你通过制造箭矢获得了箭 × 7。");
  localization.setLocale("en-US");
});

test("M6-B mutation events describe fumbling and delayed reality changes", () => {
  const fumbling = {
    kind: "mutation.fumbled-drop",
    messageKey: "mutation-fumbled-drop",
    args: { target: "demo.item.dagger", damage: "7" },
  };
  const reality = {
    kind: "mutation.reality-changed",
    messageKey: "mutation-reality-changed",
    args: {},
  };

  assert.equal(
    formatter.formatEvent(fumbling),
    "You trip over your own feet, take 7 damage, and drop Dagger.",
  );
  localization.setLocale("zh-CN");
  assert.equal(
    formatter.formatEvent(reality),
    "你拒绝了这个现实，并用另一个现实取而代之！",
  );
  localization.setLocale("en-US");
});

test("M6-D warning events use the authoritative danger bands", () => {
  const warning = {
    kind: "mutation.warning.extreme",
    messageKey: "mutation-warning-extreme",
    args: { danger: "120" },
  };

  assert.equal(formatter.formatEvent(warning), "You feel utterly terrified!");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(warning), "你感到极度恐惧！");
  localization.setLocale("en-US");
});

test("mutation aura events use the typed damage element", () => {
  const event = {
    kind: "mutation.aura-hit",
    messageKey: "mutation-aura-hit",
    args: { target: "demo.actor.small-kobold", damage: "4" },
    outcome: {
      type: "damage",
      resolution: {
        rawDamage: 4,
        armorReduction: 0,
        resistanceAdjustment: 0,
        finalDamage: 4,
        damageType: "fire",
        resistance: "normal",
      },
    },
  };

  assert.equal(formatter.formatEvent(event), "Your fire aura hits Small Kobold for 4 damage.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你的火焰光环击中了小狗头人，造成 4 点伤害。");
  localization.setLocale("en-US");
});

test("mutation melee events retain the authoritative innate attack name", () => {
  const event = {
    kind: "mutation.melee-hit",
    messageKey: "mutation-melee-hit",
    args: {
      source: "rfb.mutation.scorpion-tail",
      attack: "尾巴",
      target: "demo.actor.small-kobold",
      damage: "7",
    },
  };

  assert.equal(formatter.formatEvent(event), "Your 尾巴 hits Small Kobold for 7 damage.");
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你的尾巴击中了小狗头人，造成 7 点伤害。");
  localization.setLocale("en-US");
});

test("wilderness ambush events use the dedicated localized message", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "wilderness.ambushed",
      messageKey: "wilderness-ambushed",
      args: {},
    }),
    "You are ambushed in the wilderness!",
  );
});

test("eldritch horror events localize the monster and sanity outcome", () => {
  const event = {
    kind: "monster.eldritch-horror",
    messageKey: "monster-eldritch-horror",
    args: {
      source: "demo.actor.ghast",
      sourceEntity: "test.ghast",
      power: "9",
      outcome: "amnesia",
    },
  };

  assert.equal(
    formatter.formatEvent(event),
    "The sight of Ghast tears away your memories of this place.",
  );
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "妖鬼的面容抹去了你对这里的记忆。");
  localization.setLocale("en-US");
});

test("item event formatting follows projected knowledge and locale changes", () => {
  const event = {
    kind: "item.pickup",
    messageKey: "item-pickup-success",
    args: { target: "demo.item.light-healing-potion", quantity: "3" },
  };

  assert.equal(formatter.formatEvent(event), "You pick up Potion of Cure Light Wounds ×3.");
  state.currentInventory = [
    {
      kindId: "demo.item.light-healing-potion",
      displayNameKey: "item-demo-light-healing-potion-appearance",
    },
  ];
  assert.equal(formatter.formatEvent(event), "You pick up Gold Potion ×3.");

  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(event), "你将 3 个金色药水收入了背包。");
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
    "出售了 1 件木火把，获得 1 金币。余额：242。",
  );
  localization.setLocale("en-US");
});

test("task service events use the current dotted event kinds", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "task.accepted",
      messageKey: "task-accepted",
      args: { task: "demo.task.thieves-hideout" },
    }),
    "Task accepted: The Thieves' Hideout (Outpost).",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "task.reward-claim-unavailable",
      messageKey: "task-reward-claim-unavailable",
      args: {},
    }),
    "That reward cannot be claimed here right now.",
  );
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
      kind: "item.use-hunger-satisfied",
      messageKey: "item-use-hunger-satisfied",
      args: {
        target: "demo.item.satisfy-hunger-scroll",
        nameKey: "item-demo-satisfy-hunger-scroll-name",
        nutrition: "14999",
      },
    }),
    "You use Scroll of Satisfy Hunger; your food rises to 14999 / 15000.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-status-applied",
      messageKey: "item-use-status-applied",
      args: {
        source: "demo.item.hallucination-mushroom",
        nameKey: "item-demo-hallucination-mushroom-name",
        status: "rfb.status.hallucination",
        duration: "30",
      },
    }),
    "You use Mushroom of Hallucination and gain hallucination for 30 ticks.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-resource-drained",
      messageKey: "item-use-resource-drained",
      args: {
        source: "demo.item.hallucination-mushroom",
        nameKey: "item-demo-hallucination-mushroom-name",
        resource: "demo.resource.mana",
        amount: "50",
      },
    }),
    "Mushroom of Hallucination drains 50 Mana.",
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

test("P3.2 potion events format experience loss and partial status curing", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "item.experience-lost",
      messageKey: "item-experience-lost",
      args: {
        source: "demo.item.lose-memories-potion",
        nameKey: "item-demo-lose-memories-potion-name",
        amount: "250",
        remaining: "750",
      },
    }),
    "You use Potion of Lose Memories and lose 250 experience (750 remaining).",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-status-reduced",
      messageKey: "item-use-status-reduced",
      args: {
        target: "demo.item.antidote-potion",
        nameKey: "item-demo-antidote-potion-name",
        status: "rfb.status.poison",
        before: "10000",
        after: "5000",
      },
    }),
    "You use Potion of Antidote, reducing poison from 10000 to 5000 ticks.",
  );
});

test("P3.3 knowledge events localize inventory identification and current reports", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-inventory-identified",
      messageKey: "item-use-inventory-identified",
      args: {
        source: "demo.item.understanding-scroll",
        nameKey: "item-demo-understanding-scroll-name",
        count: "3",
      },
    }),
    "You use Scroll of Understanding and identify 3 carried item stacks.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.auto-identified",
      messageKey: "item-auto-identified",
      args: { count: "1" },
    }),
    "Your understanding identifies 1 newly carried item stack.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-self-knowledge",
      messageKey: "item-use-self-knowledge",
      args: {
        source: "demo.item.self-knowledge-potion",
        nameKey: "item-demo-self-knowledge-potion-name",
        level: "7",
        hp: "32",
        maxHp: "40",
        gold: "123",
        nutrition: "9000",
        attack: "11",
        defense: "12",
        meleeSkill: "13",
        armorClass: "14",
        speed: "110",
        strength: "18/18/18",
        intelligence: "17/17/17",
        wisdom: "16/16/16",
        dexterity: "15/15/15",
        constitution: "14/14/14",
        charisma: "13/13/13",
        statuses: "rfb.status.understanding:1:400",
        resistances: "Fire:Resistant",
        resources: "demo.resource.mana:20/30",
      },
    }),
    "Potion of Self Knowledge reveals: level 7; HP 32/40; gold 123; food 9000; attack 11; defense 12; melee 13; armour 14; speed 110; STR 18/18/18, INT 17/17/17, WIS 16/16/16, DEX 15/15/15, CON 14/14/14, CHR 13/13/13; statuses [rfb.status.understanding:1:400]; resistances [Fire:Resistant]; resources [demo.resource.mana:20/30].",
  );
});

test("P3.4 terrain events localize lighting destruction and warding glyphs", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-floor-light",
      messageKey: "item-use-floor-light",
      args: {
        source: "demo.item.light-scroll",
        nameKey: "item-demo-light-scroll-name",
        count: "12",
      },
    }),
    "Scroll of Light permanently lights 12 spaces.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-area-destruction",
      messageKey: "item-use-area-destruction",
      args: {
        source: "demo.item.destruction-scroll",
        nameKey: "item-demo-destruction-scroll-name",
        count: "41",
        entities: "2",
        items: "3",
        gold: "4",
      },
    }),
    "Scroll of Destruction remakes 41 spaces, removing 2 creatures, 3 items, and 4 treasure piles.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "monster.warding-glyph-held",
      messageKey: "monster-warding-glyph-held",
      args: { source: "demo.actor.bloodfang-the-wolf" },
    }),
    "The glyph of warding repels Bloodfang the Wolf.",
  );
});

test("P3.5 item generation mutation and rumour events localize", () => {
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-acquirement",
      messageKey: "item-use-acquirement",
      args: {
        source: "demo.item.acquirement-scroll",
        nameKey: "item-demo-acquirement-scroll-name",
        count: "1",
      },
    }),
    "Scroll of Acquirement creates 1 excellent item at your feet.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-mundanity",
      messageKey: "item-use-mundanity",
      args: {
        source: "demo.item.mundanity-scroll",
        nameKey: "item-demo-mundanity-scroll-name",
        target: "demo.item.arrow",
      },
    }),
    "Scroll of Mundanity strips Arrow back to the mundane.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-crafting",
      messageKey: "item-use-crafting",
      args: {
        source: "demo.item.crafting-scroll",
        nameKey: "item-demo-crafting-scroll-name",
        target: "demo.item.arrow",
        affix: "demo.affix.frost-hunter",
      },
    }),
    "Scroll of Crafting crafts Arrow with frost hunter.",
  );
  assert.equal(
    formatter.formatEvent({
      kind: "item.use-rumour",
      messageKey: "item-use-rumour",
      args: {
        source: "demo.item.rumour-scroll",
        nameKey: "item-demo-rumour-scroll-name",
        rumourKey: "rumour-demo-warrens-depths",
      },
    }),
    "Scroll of Rumour reads: “The oldest warrens hide their best steel below the roots.”",
  );
});

test("damage event formatting preserves typed resistance outcomes", () => {
  const event = {
    kind: "combat.hit",
    messageKey: "combat-player-hit",
    args: { target: "demo.actor.small-kobold", damage: "10" },
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
    "You hit Small Kobold for 7 fire damage (3 resisted).",
  );
  localization.setLocale("zh-CN");
  assert.equal(
    formatter.formatEvent(event),
    "你击中了小狗头人，造成 7 点火焰伤害（抵抗了 3 点）。",
  );
  localization.setLocale("en-US");
});

test("inn stay and travel results use focused bilingual messages", () => {
  const completed = {
    kind: "inn.stay",
    messageKey: "inn-stay-completed",
    args: { cost: "25", balance: "75" },
  };
  const unavailable = {
    kind: "inn.stay-unavailable",
    messageKey: "inn-stay-unavailable",
    args: { reason: "needs-healer" },
  };

  assert.equal(
    formatter.formatEvent(completed),
    "You stay overnight and wake refreshed. Paid 25 gold; balance 75.",
  );
  localization.setLocale("zh-CN");
  assert.equal(formatter.formatEvent(unavailable), "无法住宿：你需要的是治疗师，而不是房间。");
  assert.equal(
    formatter.formatEvent({
      kind: "inn.travel-unavailable",
      messageKey: "inn-travel-unavailable",
      args: { reason: "town-unvisited" },
    }),
    "无法从旅店出发：只能前往亲自到访过且设有旅店的城镇。",
  );
  localization.setLocale("en-US");
  assert.equal(
    formatter.formatEvent({
      kind: "inn.travel",
      messageKey: "inn-travel-completed",
      args: { cost: "500", balance: "125" },
    }),
    "You depart from the inn and arrive at your destination. Paid 500 gold; balance 125.",
  );
});

test("Warrens transitions name the Outpost and stairs without legacy Echo text", () => {
  state.currentWorldId = "demo.world.middle-earth";
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
