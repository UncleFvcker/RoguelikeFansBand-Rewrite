// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  abilityAttributeAbbreviation,
  abilityConfirmationMessageKey,
  abilityPresentation,
  abilityStatusMessageKey,
  formatAttributeValue,
  mutationRatingMessageKey,
  nutritionPercentage,
  weaponProficienciesByCategory,
  proficiencyRankMessageKey,
  wildernessClock,
} from "./status-panel.ts";

test("Snotling Devour Flesh requires its dedicated confirmation", () => {
  assert.equal(
    abilityConfirmationMessageKey("rfb.ability.race.devour-flesh"),
    "confirm-ability-devour-flesh",
  );
  assert.equal(abilityConfirmationMessageKey("demo.ability.life-heal"), undefined);
});

test("Archer Create Ammo presents one level-gated menu", () => {
  const group = "ability-group-demo-archer-create-ammo-name";
  const abilities = [
    { id: "shots", minimumLevel: 1, uiGroupNameKey: group },
    { id: "arrows", minimumLevel: 10, uiGroupNameKey: group },
    { id: "bolts", minimumLevel: 20, uiGroupNameKey: group },
  ];
  const labels = (level: number) =>
    abilityPresentation(abilities, level).map((entry) =>
      entry.type === "heading" ? `heading:${entry.nameKey}` : `ability:${entry.ability.id}`,
    );

  assert.deepEqual(labels(1), [`heading:${group}`, "ability:shots"]);
  assert.deepEqual(labels(10), [`heading:${group}`, "ability:shots", "ability:arrows"]);
  assert.deepEqual(labels(20), [
    `heading:${group}`,
    "ability:shots",
    "ability:arrows",
    "ability:bolts",
  ]);
});

test("spellbook headings expose one divine study action", () => {
  const book = "ability-book-test-prayers-name";
  const entries = abilityPresentation(
    [
      {
        id: "first",
        minimumLevel: 1,
        bookNameKey: book,
        bookItemId: "item.prayers",
        canStudy: false,
      },
      {
        id: "second",
        minimumLevel: 2,
        bookNameKey: book,
        bookItemId: "item.prayers",
        canStudy: true,
      },
    ],
    1,
  );

  assert.deepEqual(entries[0], {
    type: "heading",
    nameKey: book,
    bookItemId: "item.prayers",
    canStudy: true,
  });
});

test("mutation presentation exposes ratings and the shared ability source", () => {
  assert.equal(mutationRatingMessageKey("awful"), "mutation-rating-awful");
  assert.equal(mutationRatingMessageKey("great"), "mutation-rating-great");
  assert.equal(
    abilityStatusMessageKey({ source: "mutation", learned: false }),
    "ability-status-mutation",
  );
  assert.equal(
    abilityStatusMessageKey({ source: "learned", learned: true }),
    "ability-status-learned",
  );
  assert.equal(
    abilityStatusMessageKey({ source: "class", learned: false }),
    "ability-status-class",
  );
  assert.equal(
    abilityStatusMessageKey({ source: "race", learned: false }),
    "ability-status-innate",
  );
  assert.equal(abilityAttributeAbbreviation("strength"), "STR");
});

test("Paladin Hell Lance stays visible as a level-gated class power", () => {
  const hellLance = {
    id: "demo.ability.paladin-hell-lance",
    minimumLevel: 30,
    source: "class",
    learned: false,
    canCast: false,
  };

  assert.deepEqual(abilityPresentation([hellLance], 29), [
    { type: "ability", ability: hellLance },
  ]);
  assert.equal(abilityStatusMessageKey(hellLance), "ability-status-class");
});

test("status panel preserves exceptional attribute display values", () => {
  assert.equal(formatAttributeValue(18), "18");
  assert.equal(formatAttributeValue(19), "18/1");
  assert.equal(formatAttributeValue(27), "18/9");
});

test("status panel displays nutrition relative to the 10000 baseline", () => {
  assert.equal(nutritionPercentage(15_000), 150);
  assert.equal(nutritionPercentage(10_000), 100);
  assert.equal(nutritionPercentage(9_999), 99);
});

test("weapon proficiency presentation keeps original groups and rank names", () => {
  const melee = { itemKindId: "sword", category: "melee", rank: "beginner" };
  const launcher = { itemKindId: "bow", category: "launcher", rank: "expert" };

  assert.deepEqual(weaponProficienciesByCategory([launcher, melee], "melee"), [melee]);
  assert.deepEqual(weaponProficienciesByCategory([launcher, melee], "launcher"), [launcher]);
  assert.equal(proficiencyRankMessageKey("unskilled"), "proficiency-rank-unskilled");
  assert.equal(proficiencyRankMessageKey("master"), "proficiency-rank-master");
});

test("wilderness clock follows the original half-day boundaries", () => {
  assert.deepEqual(wildernessClock(0), { day: 1, hour: 6, minute: 0, daytime: true });
  assert.deepEqual(wildernessClock(49_999), {
    day: 1,
    hour: 17,
    minute: 59,
    daytime: true,
  });
  assert.deepEqual(wildernessClock(50_000), {
    day: 1,
    hour: 18,
    minute: 0,
    daytime: false,
  });
  assert.deepEqual(wildernessClock(75_000), {
    day: 2,
    hour: 0,
    minute: 0,
    daytime: false,
  });
  assert.deepEqual(wildernessClock(100_000), {
    day: 2,
    hour: 6,
    minute: 0,
    daytime: true,
  });
});
