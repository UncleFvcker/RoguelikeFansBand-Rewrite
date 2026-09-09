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
    append(...children) { this.children.push(...children); }
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
