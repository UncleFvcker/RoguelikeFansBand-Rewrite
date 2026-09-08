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
  weaponProficienciesByGroup,
  proficiencyRankMessageKey,
  wildernessClock,
  renderCharacterOverview,
  renderHudExperience,
  hudLocationText,
  attributeSourceCell,
  StatusPanel,
} from "./status-panel.ts";
import { AppState } from "./app-state.ts";

test("HUD experience uses exact cumulative XP, clamps the meter and labels the next threshold", () => {
  const meter = { setAttribute(key, value) { this[key] = value; } };
  const localization = { format: (key, args) => args ? `${args.experience} / ${args.next}` : key };
  for (const [experience, next, expected] of [
    [0, 10, 0], [25, 100, 250], [25n, 100, 250], [25, 100n, 250],
    [0n, 10n, 0], [25n, 100n, 250], [100n, 100n, 1000], [120n, 100n, 1000],
    [9007199254740993n, 18014398509481986n, 500], [10n, null, 1000], [10n, 0n, 0],
  ]) {
    renderHudExperience(meter, { experience, experienceForNextLevel: next }, localization);
    assert.equal(meter.hidden, false);
    assert.equal(meter.value, expected);
    assert.equal(meter.title, `${experience} / ${next ?? 'character-no-next-level'}`);
    assert.equal(meter['aria-valuetext'], meter.title);
  }
  renderHudExperience(meter, undefined, localization);
  assert.equal(meter.hidden, true);
});

test("HUD location distinguishes the world map, towns, wilderness and content dungeon depths", () => {
  const localization = { format: (key, args) => args ? `${args.name} · ${args.depth}` : key };
  const contentName = (id) => `content:${id}`;
  const state = { mapScale: 'local', floorId: 'demo.floor.hideout-depth-8', town: null };
  assert.equal(hudLocationText(state, localization, contentName), 'content:demo.floor.hideout-depth · 8');
  assert.equal(hudLocationText({ ...state, floorId: 'demo.floor.surface' }, localization, contentName), 'content:demo.floor.surface');
  assert.equal(hudLocationText({ ...state, town: { nameKey: 'town-existing-name' } }, localization, contentName), 'town-existing-name');
  assert.equal(hudLocationText({ ...state, floorId: 'core.floor.wilderness' }, localization, contentName), 'hud-location-wilderness');
  assert.equal(hudLocationText({ ...state, mapScale: 'world' }, localization, contentName), 'hud-location-world');
});

test("snapshot slots are available before any status render callback, including new and loaded games", () => {
  const state = new AppState();
  const slots = [{ id: "new-ring-2", slotType: "ring" }];
  const snapshot = { bodySlots: slots, player: { isDead: false }, campaign: { status: "active" } };
  const stop = new Error("stop after checking render inputs");
  const panel = new StatusPanel({
    state,
    reconcileTargeting: (value) => {
      assert.equal(state.status, value);
      assert.equal(state.bodySlots, slots);
      assert.equal(state.bodySlots.find((slot) => slot.id === "new-ring-2").slotType, "ring");
      throw stop;
    },
  });
  // First session starts with no slots; loading another body must replace stale slots.
  for (const previous of [[], [{ id: "old-hand", slotType: "weapon" }]]) {
    state.bodySlots = previous;
    assert.throws(() => panel.render(snapshot), (error) => error === stop);
  }
  const { bodySlots, ...update } = snapshot;
  assert.throws(() => panel.render(update), (error) => error === stop);
});

test("attribute source summaries preserve ordered steps, suppression and incomplete knowledge", () => {
  const localization = { format: (key, args) => args ? `${key}:${args.value}` : key };
  const source = (modifier, extra = {}) => ({ kind: "mutation", modifier, complete: true, suppressed: false, ...extra });
  assert.equal(attributeSourceCell([], localization), "—");
  assert.equal(attributeSourceCell([source(0)], localization), "0");
  assert.equal(attributeSourceCell([source(3), source(-2)], localization), "+3 · -2");
  assert.equal(attributeSourceCell([source(0, { complete: false })], localization), "attribute-source-known-value:0");
  assert.equal(attributeSourceCell([source(-3, { suppressed: true })], localization), "-3attribute-source-suppressed-short");
  assert.equal(attributeSourceCell([source(0, { kind: "normal-appearance" })], localization), "attribute-source-rule");
});

test("character overview projects exact experience, actual resources and current combat values", () => {
  class Element {
    children = [];
    textContent = "";
    get ownerDocument() { return document; }
    append(...children) { this.children.push(...children); }
    replaceChildren(...children) { this.children = children; }
  }
  const document = { createElement: () => new Element() };
  const dom = Object.fromEntries([
    "characterNameValue", "characterRaceValue", "characterClassValue", "characterLevelValue",
    "characterMaximumExperienceValue", "characterNextExperienceValue", "characterGoldValue",
    "characterWorldTimeValue", "characterVitalsList", "progressionExperienceValue", "progressionPersonalityValue",
  ].map((key) => [key, new Element()]));
  const localization = { locale: "en-US", format: (key, args) => args ? `${key} ${JSON.stringify(args)}` : key };
  const player = {
    name: "Long character name", gold: 12345, hp: 12, maxHp: 56, armorClass: 104, defense: 7, speed: 113,
    build: { raceNameKey: "race", classNameKey: "class", personalityNameKey: "personality" },
    progress: { level: 27, experience: 9007199254740993n, maximumExperience: 9007199254740995n, experienceForNextLevel: 9007199254741995n },
    resources: [{ nameKey: "mana", current: 0, maximum: 24 }, { nameKey: "other-resource", current: 3, maximum: 5 }],
    sniperConcentration: { current: 0, maximum: 5 },
  };
  renderCharacterOverview(dom, player, 50000, localization);
  assert.equal(dom.characterNameValue.textContent, player.name);
  assert.equal(dom.characterLevelValue.textContent, "27");
  assert.equal(dom.progressionExperienceValue.textContent, "9007199254740993");
  assert.equal(dom.characterMaximumExperienceValue.textContent, "9007199254740995");
  assert.equal(dom.characterNextExperienceValue.textContent, "9007199254741995");
  assert.equal(dom.characterGoldValue.textContent, "12,345");
  assert.match(dom.characterWorldTimeValue.textContent, /"day":1,"hour":"18","minute":"00"/);
  const rows = () => dom.characterVitalsList.children.map((row) => row.children.map((cell) => cell.textContent));
  assert.deepEqual(rows().slice(1), [
    ["mana", "0 / 24"], ["other-resource", "3 / 5"], ["sniper-concentration", "0 / 5"],
    ["character-armor-class", "104"], ["character-speed", "113"],
  ]);
  renderCharacterOverview(dom, { ...player, progress: { ...player.progress, experienceForNextLevel: null } }, 50000, localization);
  assert.equal(dom.characterNextExperienceValue.textContent, "character-no-next-level");
  renderCharacterOverview(dom, { ...player, build: null, progress: undefined, resources: [], sniperConcentration: null }, 75000, localization);
  assert.equal(dom.characterRaceValue.textContent, "progression-unavailable");
  assert.equal(dom.progressionExperienceValue.textContent, "progression-unavailable");
  assert.equal(dom.characterMaximumExperienceValue.textContent, "progression-unavailable");
  assert.equal(dom.characterNextExperienceValue.textContent, "progression-unavailable");
  assert.equal(rows().length, 3);
  assert.match(dom.characterWorldTimeValue.textContent, /"day":2,"hour":"00","minute":"00"/);
});

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
  const melee = { itemKindId: "sword", group: "sword", rank: "beginner" };
  const launcher = { itemKindId: "bow", group: "bow", rank: "expert" };

  assert.deepEqual(weaponProficienciesByGroup([launcher, melee], "sword"), [melee]);
  assert.deepEqual(weaponProficienciesByGroup([launcher, melee], "bow"), [launcher]);
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
