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
  renderCharacterMutations,
  renderHudExperience,
  renderTaskLog,
  hudLocationText,
  attributeSourceCell,
  StatusPanel,
} from "./status-panel.ts";
import { AppState } from "./app-state.ts";

test("Maia choice requires a click and survives closing and rerendering", async () => {
  class Element {
    children = []; dataset = {}; handlers = {}; textContent = "";
    get ownerDocument() { return document; }
    querySelectorAll() { return []; }
    append(...children) { this.children.push(...children); }
    replaceChildren(...children) { this.children = children; }
    addEventListener(type, handler) { this.handlers[type] = handler; }
  }
  const document = { createElement: () => new Element() };
  const list = new Element();
  const commands = [];
  const localization = { format: key => key };
  const state = { busy: false, playerDead: false, campaignEnded: false };
  const maia = { maiaPath: null, pendingMaiaPathChoice: true };
  const render = () => renderCharacterMutations(list, [], null, state, localization, async command => commands.push(command), maia);
  render(); render();
  assert.deepEqual(commands, []);
  const buttons = () => list.children[0].children.filter(node => node.handlers.click);
  assert.deepEqual(buttons().map(button => button.textContent), ["maia-path-enlightened", "maia-path-corrupted"]);
  buttons()[1].handlers.click();
  assert.deepEqual(commands, [{ type: "choose-maia-path", path: "corrupted" }]);
  state.busy = true; render();
  assert.ok(buttons().every(button => button.disabled));
  state.busy = false; maia.pendingMaiaPathChoice = false; maia.maiaPath = "corrupted"; render();
  assert.equal(buttons().length, 0);
  assert.equal(list.children[0].children[1].textContent, "maia-path-corrupted");
});

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
  assert.equal(hudLocationText({ ...state, dungeon: { nameKey: 'floor-demo-hideout-depth-name', currentDepth: 8, maximumDepth: 8 } }, localization, contentName), 'floor-demo-hideout-depth-name · 8');
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
    progress: { level: 27, lifeForce: 1, experience: 9007199254740993n, maximumExperience: 9007199254740995n, experienceForNextLevel: 9007199254741995n },
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
    ["mana", "0 / 24"], ["other-resource", "3 / 5"], ["status-life-force", "1 / 1000"], ["sniper-concentration", "0 / 5"],
    ["character-armor-class", "104"], ["character-speed", "113"],
  ]);
  renderCharacterOverview(dom, { ...player, progress: { ...player.progress, lifeForce: 1000, experienceForNextLevel: null } }, 50000, localization);
  assert.equal(dom.characterNextExperienceValue.textContent, "character-no-next-level");
  assert.deepEqual(rows().find(([key]) => key === "status-life-force"), ["status-life-force", "1000 / 1000"]);
  renderCharacterOverview(dom, { ...player, build: null, progress: undefined, resources: [], sniperConcentration: null }, 75000, localization);
  assert.equal(dom.characterRaceValue.textContent, "progression-unavailable");
  assert.equal(dom.progressionExperienceValue.textContent, "progression-unavailable");
  assert.equal(dom.characterMaximumExperienceValue.textContent, "progression-unavailable");
  assert.equal(dom.characterNextExperienceValue.textContent, "progression-unavailable");
  assert.equal(rows().length, 3);
  assert.match(dom.characterWorldTimeValue.textContent, /"day":2,"hour":"00","minute":"00"/);
});

test("retirement follows the core projection, confirms once and respects busy state", () => {
  let accepted = false;
  const prompts = [];
  const button = Object.assign(new EventTarget(), { disabled: false, ownerDocument: { defaultView: {
    confirm: (message) => { prompts.push(message); return accepted; },
  } } });
  const state = new AppState();
  state.status = { floorId: "any-projected-town", mapScale: "local", campaign: { status: "victorious", canRetire: true } };
  const commands = [];
  const panel = new StatusPanel({ state, localization: { format: key => key }, dispatch: async command => { commands.push(command); },
    dom: { campaignRetire: button, resourceRest: new EventTarget(), dismissPets: new EventTarget(), summonCommandButtons: {} } });
  panel.install();
  panel.updateCampaignAction();
  assert.equal(button.disabled, false);
  button.dispatchEvent(new Event("click"));
  assert.deepEqual(commands, []);
  accepted = true;
  button.dispatchEvent(new Event("click"));
  assert.deepEqual(commands, [{ type: "retire" }]);
  assert.deepEqual(prompts, ["confirm-campaign-retire", "confirm-campaign-retire"]);
  state.busy = true;
  panel.updateCampaignAction();
  button.dispatchEvent(new Event("click"));
  assert.equal(commands.length, 1);
  state.busy = false;
  state.status.campaign.canRetire = false;
  panel.updateCampaignAction();
  assert.equal(button.disabled, true);
  panel.dispose();
});

test("task log renders projected depth, target, skipped history and abandon eligibility", () => {
  class Element {
    children = []; dataset = {}; handlers = {}; textContent = "";
    parentElement = { scrollTop: 0 };
    get ownerDocument() { return document; }
    append(...children) { this.children.push(...children); }
    replaceChildren(...children) { this.children = children; }
    querySelectorAll() { return []; }
    addEventListener(type, listener) { this.handlers[type] = listener; }
  }
  const document = { activeElement: null, createElement: (tag) => Object.assign(new Element(), { tagName: tag.toUpperCase() }) };
  const list = new Element();
  const localization = { format: (key, args) => args ? `${key} ${JSON.stringify(args)}` : key };
  const base = { floorId: "floor", nameKey: "quest", current: 0, required: 1, stage: 1, stages: 1, retakesUsed: 0 };
  const tasks = [
    { ...base, taskId: "random", status: "active", canAbandon: true, depth: 44, targetNameKey: "target-random" },
    { ...base, taskId: "serpent", status: "active", canAbandon: false, depth: 100, targetNameKey: "target-serpent" },
    { ...base, taskId: "skipped", status: "skipped", canAbandon: false },
  ];
  const commands = [];
  const flatten = (node) => [node, ...node.children.flatMap(flatten)];
  renderTaskLog(list, tasks, localization, false, task => commands.push(task.taskId));
  const nodes = flatten(list);
  assert.ok(nodes.some(node => node.textContent === 'task-log-depth {"depth":100}'));
  assert.ok(nodes.some(node => node.textContent === 'task-log-target {"target":"target-serpent"}'));
  assert.ok(nodes.some(node => node.textContent === "task-status-skipped"));
  const buttons = nodes.filter(node => node.tagName === "BUTTON");
  assert.equal(buttons.length, 1);
  buttons[0].handlers.click();
  assert.deepEqual(commands, ["random"]);
  renderTaskLog(list, tasks, localization, true, () => {});
  assert.ok(flatten(list).filter(node => node.tagName === "BUTTON").every(node => node.disabled));
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
    realmId: undefined,
    bookItemId: "item.prayers",
    canStudy: true,
  });
});

test("Mage books keep primary and secondary ordering and show authoritative learning states", () => {
  const ability = (realm, rank) => ({ id: `${realm}-${rank}`, minimumLevel: 1, source: "learned", bookRealmId: realm, bookRank: rank, bookNameKey: `${realm}-book-${rank}` });
  const entries = abilityPresentation([ability("life", 1), ability("death", 2), ability("death", 1)], 1, ["death", "life"]);
  assert.deepEqual(entries.filter(entry => entry.type === "heading").map(entry => [entry.realmId, entry.nameKey]), [
    ["death", "death-book-1"], ["death", "death-book-2"], ["life", "life-book-1"],
  ]);
  assert.equal(abilityStatusMessageKey({ source: "learned", learned: false, forgotten: true, canStudy: false }), "ability-status-forgotten");
  assert.equal(abilityStatusMessageKey({ source: "learned", learned: false, forgotten: false, canStudy: true }), "ability-status-study-available");
  assert.equal(abilityStatusMessageKey({ source: "learned", learned: true, forgotten: false, canStudy: true }), "ability-status-restudy");
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
