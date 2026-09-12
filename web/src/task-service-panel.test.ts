// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  TaskServicePanel,
  bountyMissionAction,
  facilityIdentificationCandidate,
  facilityMembershipKey,
  facilityServiceActionKey,
  facilityServiceUsesItem,
  filterResearchMonsters,
  taskActionForStatus,
  taskActionLabelKey,
} from "./task-service-panel.ts";

test("paid facility selection and closing are free; only confirmation dispatches the selected projection", (t) => {
  class Element extends EventTarget {
    children = [];
    dataset = {};
    open = false;
    selected = undefined;
    constructor(tag = "div") { super(); this.tag = tag; }
    get ownerDocument() { return document; }
    get value() { return this.selected ?? (this.tag === "select" ? this.children[0]?.value : "") ?? ""; }
    set value(value) { this.selected = value; }
    append(...children) { this.children.push(...children); for (const child of children) if (typeof child === "object") child.parentElement = this; }
    checkValidity() { return Number.isInteger(Number(this.value)) && Number(this.value) >= Number(this.min) && Number(this.value) <= Number(this.max); }
    replaceChildren(...children) { this.children = children; this.selected = undefined; }
    setAttribute() {}
    get selectedOptions() { return this.children.filter((child) => child.value === this.value); }
    querySelector(selector) {
      return this.children.flatMap((row) => row.children).find((element) =>
        element.tag === "select" && selector === `select[data-facility-service="${element.dataset.facilityService}"]`);
    }
    closest(selector) { return selector === "[data-facility-action]" && this.dataset.facilityAction ? this : undefined; }
    showModal() { this.open = true; }
    close() { this.open = false; this.dispatchEvent(new Event("close")); }
  }
  const elements = new Map();
  const document = {
    createElement: (tag) => new Element(tag),
    getElementById: (id) => {
      if (!elements.has(id)) elements.set(id, new Element());
      return elements.get(id);
    },
  };
  const previous = globalThis.HTMLElement;
  globalThis.HTMLElement = Element;
  t.after(() => { globalThis.HTMLElement = previous; });
  const commands = [];
  const state = { busy: false, inventory: [], equipment: [] };
  const panel = new TaskServicePanel({
    document, state, localization: { format: (key, args) => key + (args ? JSON.stringify(args) : "") },
    visibleItemName: (key) => key,
    dispatch: async (command) => { commands.push(command); }, beforeOpen: () => {},
  });
  panel.install();
  t.after(() => panel.dispose());
  const snapshot = { taskServices: [{ id: "tower", playerAtEntrance: true, membership: "visitor", tasks: [],
    teleportLevelCost: 100000, teleportDungeons: [{ dungeonId: "cave", nameKey: "cave-name", recallDepth: 15, depths: [15, 20, 27] }],
  }] };
  panel.render(snapshot);
  const list = elements.get("task-service-list");
  const [dungeon, depth, confirm] = list.children[0].children;
  assert.equal(confirm.disabled, true);
  dungeon.value = "cave";
  dungeon.dispatchEvent(new Event("change"));
  assert.deepEqual(depth.children.map((option) => option.value), ["15", "20", "27"]);
  depth.value = "27";
  depth.dispatchEvent(new Event("change"));
  assert.equal(confirm.disabled, false);
  assert.deepEqual(commands, []);
  elements.get("task-service-dialog").close();
  assert.deepEqual(commands, []);
  const click = new Event("click");
  Object.defineProperty(click, "target", { value: confirm });
  state.busy = true;
  list.dispatchEvent(click);
  assert.deepEqual(commands, []);
  state.busy = false;
  list.dispatchEvent(click);
  assert.deepEqual(commands, [{ type: "teleport-to-dungeon-level-at-facility", facilityId: "tower", dungeonId: "cave", depth: 27 }]);
  snapshot.taskServices[0].teleportDungeons = [];
  panel.render(snapshot);
  assert.equal(list.children[0].children[2].disabled, true);
  commands.length = 0;
  state.inventory = [{ id: "sword", displayNameKey: "sword-name", kindId: "sword-kind" }];
  snapshot.taskServices = [{ id: "guild", playerAtEntrance: true, membership: "owner", tasks: [],
    serviceActions: [{ kind: "enchant-weapon", cost: 0, targets: [{ itemId: "sword", choices: [
      { steps: 1, cost: 1050, result: { toHit: 1, toDamage: 1, toArmor: 0 } },
      { steps: 3, cost: 3150, result: { toHit: 3, toDamage: 3, toArmor: 0 } },
    ] }] }],
  }];
  panel.render(snapshot);
  const [tiers, enchant] = list.children[0].children;
  assert.match(tiers.children[1].textContent, /"cost":3150/);
  tiers.value = "sword:3";
  tiers.dispatchEvent(new Event("change"));
  elements.get("task-service-dialog").close();
  assert.deepEqual(commands, []);
  const enchantClick = new Event("click");
  Object.defineProperty(enchantClick, "target", { value: enchant });
  state.busy = true;
  list.dispatchEvent(enchantClick);
  assert.deepEqual(commands, []);
  state.busy = false;
  list.dispatchEvent(enchantClick);
  assert.deepEqual(commands, [{ type: "use-facility-service", facilityId: "guild",
    service: "enchant-weapon", itemId: "sword", enchantmentSteps: 3 }]);
  commands.length = 0;
  snapshot.taskServices = [{ id: "nature-tower", playerAtEntrance: true, membership: "visitor", tasks: [],
    serviceActions: [{ kind: "balance-ritual", cost: 14000 }],
  }];
  panel.render(snapshot);
  const [explanation, ritual] = list.children[0].children;
  assert.match(explanation.textContent, /facility-balance-ritual-description/);
  assert.match(ritual.textContent, /"cost":14000/);
  elements.get("task-service-dialog").close();
  assert.deepEqual(commands, []);
  const ritualClick = new Event("click");
  Object.defineProperty(ritualClick, "target", { value: ritual });
  state.busy = true;
  list.dispatchEvent(ritualClick);
  assert.deepEqual(commands, []);
  state.busy = false;
  list.dispatchEvent(ritualClick);
  assert.deepEqual(commands, [{ type: "use-facility-service", facilityId: "nature-tower",
    service: "balance-ritual", itemId: undefined, enchantmentSteps: undefined }]);
  commands.length = 0;
  const casino = { maximumWager: 200 };
  snapshot.taskServices = [{ id: "sorcery-tower", playerAtEntrance: true, membership: "visitor", tasks: [],
    innTravelDestinations: [{ townId: "outpost", townNameKey: "outpost-name", cost: 700 }],
  }];
  panel.render(snapshot);
  const travel = list.children[0].children[0];
  assert.match(travel.textContent, /"cost":700/);
  elements.get("task-service-dialog").close();
  assert.deepEqual(commands, []);
  const travelClick = new Event("click");
  Object.defineProperty(travelClick, "target", { value: travel });
  state.busy = true;
  list.dispatchEvent(travelClick);
  assert.deepEqual(commands, []);
  state.busy = false;
  list.dispatchEvent(travelClick);
  assert.deepEqual(commands, [{ type: "travel-from-inn", facilityId: "sorcery-tower", destinationTownId: "outpost" }]);
  commands.length = 0;
  snapshot.taskServices[0].innTravelDestinations = [];
  panel.render(snapshot);
  list.dispatchEvent(travelClick);
  assert.deepEqual(commands, []);
  snapshot.taskServices = [{ id: "casino", playerAtEntrance: true, tasks: [], casino }];
  panel.render(snapshot);
  const row = list.children[0];
  const game = row.children[0].children[0];
  const wager = row.children[1].children[0];
  const wheel = row.children[2].children[0];
  const start = row.children[3];
  game.value = "roulette"; game.dispatchEvent(new Event("change"));
  assert.equal(wheel.parentElement.hidden, false);
  wheel.value = "7"; wager.value = "201";
  start.dispatchEvent(new Event("click"));
  assert.deepEqual(commands, []);
  wager.value = "100"; start.dispatchEvent(new Event("click"));
  assert.deepEqual(commands.pop(), { type: "casino", facilityId: "casino", action: { type: "start", game: "roulette", wager: 100, rouletteChoice: 7 } });
  casino.session = { game: "poker", wager: 100, startingGold: 1000, round: { type: "poker", cards: [0, 14, 28, 42, 52] } };
  panel.render(snapshot);
  const cancel = new Event("cancel", { cancelable: true });
  elements.get("task-service-dialog").dispatchEvent(cancel);
  assert.equal(cancel.defaultPrevented, true);
  assert.equal(elements.get("task-service-close").disabled, true);
  const hand = list.children[0];
  hand.children[2].children[0].checked = true;
  hand.children[6].children[0].checked = true;
  state.busy = true; hand.children[7].dispatchEvent(new Event("click"));
  assert.deepEqual(commands, []);
  state.busy = false; hand.children[7].dispatchEvent(new Event("click"));
  assert.deepEqual(commands.pop(), { type: "casino", facilityId: "casino", action: { type: "draw", replaceMask: 17 } });
  casino.session.round = { type: "finished", values: [0, 14, 28, 42, 52], odds: 0, payout: 0, resultKey: "casino-loss" };
  panel.render(snapshot);
  elements.get("task-service-dialog").dispatchEvent(new Event("close"));
  assert.deepEqual(commands.pop(), { type: "casino", facilityId: "casino", action: { type: "leave" } });
});

test("monster research combines name, symbol and uniqueness filters without changing knowledge", () => {
  const monsters = [
    { kindId: "wolf", nameKey: "Wolf", glyph: "C", unique: false },
    { kindId: "king", nameKey: "Wolf King", glyph: "C", unique: true },
    { kindId: "spider", nameKey: "Wolf Spider", glyph: "S", unique: false },
  ];
  const names = (m) => m.nameKey;
  const before = structuredClone(monsters);
  assert.deepEqual(filterResearchMonsters(monsters, " WOLF ", "C", "unique", names), [monsters[1]]);
  assert.deepEqual(filterResearchMonsters(monsters, "", "", "nonunique", names), [monsters[0], monsters[2]]);
  assert.deepEqual(filterResearchMonsters(monsters, "missing", "", "all", names), []);
  assert.deepEqual(filterResearchMonsters(monsters, "", "", "all", names), monsters);
  assert.deepEqual(monsters, before);
});

test("task service actions are limited to acceptance and reward claims", () => {
  assert.equal(taskActionForStatus("available"), "accept");
  assert.equal(taskActionForStatus("reward-available"), "claim");
  for (const status of [
    "abandoned",
    "active",
    "completed",
    "failed",
    "locked",
    "paused",
    "taken",
  ]) {
    assert.equal(taskActionForStatus(status), undefined);
  }
});

test("p104d Anambar library distinguishes identification from research candidates", () => {
  assert.equal(facilityIdentificationCandidate("unexamined", false), true);
  assert.equal(facilityIdentificationCandidate("appraised", false), false);
  assert.equal(facilityIdentificationCandidate("identified", false), false);

  assert.equal(facilityIdentificationCandidate("unexamined", true), true);
  assert.equal(facilityIdentificationCandidate("appraised", true), true);
  assert.equal(facilityIdentificationCandidate("identified", true), false);
});

test("p105d Anambar facility roles and typed service actions stay stable", () => {
  assert.equal(facilityMembershipKey("visitor"), "facility-membership-visitor");
  assert.equal(facilityMembershipKey("member"), "facility-membership-member");
  assert.equal(facilityMembershipKey("owner"), "facility-membership-owner");

  assert.equal(facilityServiceUsesItem("heal"), false);
  assert.equal(facilityServiceUsesItem("assess-armor"), false);
  assert.equal(facilityServiceUsesItem("recall"), false);
  assert.equal(facilityServiceUsesItem("enchant-weapon"), true);
  assert.equal(facilityServiceUsesItem("enchant-armor"), true);
  assert.equal(facilityServiceUsesItem("enchant-ammunition"), true);
  assert.equal(facilityServiceUsesItem("enchant-bow"), true);
  assert.equal(facilityServiceActionKey("restore-vitality"), "action-facility-restore-vitality");
  assert.equal(facilityServiceActionKey("cure-mutation"), "action-facility-cure-mutation");
});

test("p106d bounty mission controls follow the authoritative mission state", () => {
  assert.equal(bountyMissionAction(undefined), "request-mission");
  assert.equal(bountyMissionAction("active"), "abandon-mission");
  assert.equal(bountyMissionAction("reward-available"), "claim-mission-reward");
});

test("p107k rewardless task conclusions use a distinct action label", () => {
  assert.equal(taskActionLabelKey("accept", false), "action-task-accept");
  assert.equal(taskActionLabelKey("claim", true), "action-task-claim");
  assert.equal(taskActionLabelKey("claim", false), "action-task-conclude");
});
