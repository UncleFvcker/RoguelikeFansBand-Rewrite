// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { AppState } from "./app-state.ts";
import {
  InventoryPanel,
  absorbableItemCandidates,
  filterInventoryItems,
  formatTenthsPound,
  itemIdentificationMessageKey,
  itemTargetCandidates,
  itemSelectionSource,
  itemFitsBodySlot,
  parseDropQuantity,
  selectedRechargingItems,
} from "./inventory-panel.ts";

test("inventory rows and search use the projected instance artifact name", (t) => {
  const { panel, dom } = createInventoryFixture(t);
  const known = item("dagger", { artifactName: "(永恒蘑菇)", equipmentSlot: "weapon" });
  panel.render([known, item("unknown")], []);
  assert.match(dom.inventoryList.children[0].children[0].children[2].textContent, /永恒蘑菇/);
  dom.inventorySearch.value = "永恒蘑菇";
  dom.inventorySearch.dispatchEvent(new Event("input"));
  assert.deepEqual(dom.inventoryList.children.map((row) => row.dataset.itemId), ["dagger"]);
});

test("inventory filters use public capabilities and distinguish lights from fuel", () => {
  const items = [
    item("sword", { equipmentSlot: "weapon" }),
    item("potion", { usable: true }),
    item("empty-wand", { charges: { current: 0, maximum: 5 } }),
    item("rechargeable", { canReceiveRecharge: true }),
    item("source", { canSupplyRecharge: true }),
    item("torch", { fuel: { kind: "torch", current: 0 } }),
    item("lantern", { fuel: { kind: "lantern", current: 5 } }),
    item("permanent-light", { equipmentSlot: "light" }),
    item("oil", { fuel: { kind: "oil", current: 5 } }),
  ];
  const names = (entry) => entry.displayNameKey;
  for (const [filter, ids] of [
    ["all", items.map((entry) => entry.id)],
    ["equippable", ["sword", "permanent-light"]],
    ["usable", ["potion"]],
    ["devices", ["empty-wand", "rechargeable", "source"]],
    ["light", ["torch", "lantern", "permanent-light"]],
  ]) {
    assert.deepEqual(filterInventoryItems(items, filter, "", names).map((entry) => entry.id), ids);
  }
});

test("inventory search matches only displayed names and inscriptions, combined with the filter", () => {
  const potion = item("secret-id", {
    kindId: "secret.kind.healing",
    displayNameKey: "appearance.potion.blue",
    inscription: "Keep 保留",
    usable: true,
    knowledge: "unknown",
  });
  const name = () => "Blue Potion 蓝色药水";
  for (const query of ["blue", " POTION ", "蓝色", "kEEP", "保留", " "]) {
    assert.deepEqual(filterInventoryItems([potion], "all", query, name), [potion]);
  }
  for (const query of ["healing", "secret-id", "appearance.potion", "unknown"]) {
    assert.deepEqual(filterInventoryItems([potion], "all", query, name), []);
  }
  assert.deepEqual(filterInventoryItems([potion], "equippable", "blue", name), []);
  assert.deepEqual(filterInventoryItems([potion], "usable", "blue", name), [potion]);
});

test("filter changes prune hidden selections without touching equipment or pack totals", (t) => {
  const { panel, dom, state, radio, commands } = createInventoryFixture(t);
  const items = [item("sword", { equipmentSlot: "weapon" }), item("potion", { usable: true })];
  const equipment = [{ ...item("worn"), slotId: "body" }];
  state.selectedInventoryIds.add("sword");
  state.selectedInventoryIds.add("potion");
  panel.render(items, equipment);
  const equippedRow = dom.equipmentList.children[0];
  const total = dom.inventoryCount.textContent;

  radio.value = "usable";
  dom.inventoryFilters.dispatchEvent(new Event("change"));
  assert.deepEqual(dom.inventoryList.children.map((row) => row.dataset.itemId), ["potion"]);
  assert.deepEqual([...state.selectedInventoryIds], ["potion"]);
  assert.deepEqual(state.inventory, items);
  assert.deepEqual(state.equipment, equipment);
  assert.equal(dom.equipmentList.children[0], equippedRow);
  assert.equal(dom.inventoryCount.textContent, total);
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  assert.deepEqual(commands, [{ type: "drop", itemIds: ["potion"] }]);

  dom.inventorySearch.value = "missing";
  dom.inventorySearch.dispatchEvent(new Event("input"));
  assert.equal(state.selectedInventoryIds.size, 0);
  assert.equal(dom.inventoryDrop.disabled, true);
  assert.equal(dom.inventoryDestroy.disabled, true);
  assert.match(dom.inventoryList.children[0].textContent, /^inventory-filter-empty/);
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  dom.inventoryDestroy.dispatchEvent(new Event("click"));
  assert.equal(commands.length, 1);
  assert.equal(dom.equipmentList.children[0], equippedRow);

  dom.inventoryFilterReset.dispatchEvent(new Event("click"));
  assert.equal(radio.value, "all");
  assert.equal(dom.inventorySearch.value, "");
  assert.equal(dom.inventoryList.children.length, 2);
  assert.equal(state.selectedInventoryIds.size, 0);

  radio.value = "usable";
  state.selectedInventoryIds.add("potion");
  panel.render([items[0], { ...items[1], usable: false }], equipment);
  assert.equal(state.selectedInventoryIds.size, 0);
  assert.match(dom.inventoryList.children[0].textContent, /^inventory-filter-empty/);
  panel.render([], equipment);
  assert.match(dom.inventoryList.children[0].textContent, /^inventory-empty /);
  panel.dispose();
  dom.inventorySearch.dispatchEvent(new Event("input"));
  assert.equal(commands.length, 1);
});

test("counted item commands use quantities, clamp to the stack, and retain destroy confirmation", async t => {
  const f = createInventoryFixture(t); f.state.mode = "playing";
  let confirmed = true, confirmations = 0;
  f.document.defaultView = { confirm: () => { confirmations++; return confirmed; } };
  const choose = async (command, count) => {
    f.panel.render([item("stack", { quantity: 4 })], []);
    f.panel.openCommand(command, count);
    const dialog = f.document.body.children.at(-1), form = dialog.children[0];
    form.children[1].children[1].value = "stack";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    await Promise.resolve();
  };
  await choose("drop", 2);
  assert.deepEqual(f.commands.at(-1), { type: "drop-quantity", itemId: "stack", quantity: 2 });
  await choose("drop", 99);
  assert.deepEqual(f.commands.at(-1), { type: "drop", itemIds: ["stack"] });
  await choose("destroy", 3);
  assert.deepEqual(f.commands.at(-1), { type: "destroy-item", itemId: "stack", quantity: 3 });
  const before = f.commands.length;
  confirmed = false;
  await choose("destroy", 99);
  assert.equal(confirmations, 2);
  assert.equal(f.commands.length, before);
});

test("item shortcuts choose authoritative categories and use the existing targeting and quantity flows", async t => {
  const f = createInventoryFixture(t); f.state.mode = "playing";
  f.document.defaultView = { confirm: () => true };
  const choose = (command, id) => {
    f.panel.openCommand(command);
    const dialog = f.document.body.children.at(-1);
    const form = dialog.children[0], select = form.children[1].children[1];
    const ids = select.children.map(option => option.value);
    select.value = id;
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    return ids;
  };
  for (const category of ["food", "potion", "scroll", "wand", "staff", "rod"]) {
    const source = item(category, { useCategory: category, usable: true, knowledge: "unknown" });
    f.panel.render([source, item("other", { useCategory: "potion", usable: false })], []);
    assert.deepEqual(choose(category, source.id), [source.id]);
    await Promise.resolve();
    assert.deepEqual(f.commands.at(-1), { type: "use-item", itemId: source.id });
  }
  const spec = { modes: ["direction"], range: 8, requiresLineOfEffect: true };
  f.panel.render([item("aim", { useCategory: "wand", usable: true, useTargetSpec: spec })], []);
  choose("wand", "aim");
  assert.deepEqual(f.targets.at(-1), [spec, { type: "item", itemId: "aim" }]);
  f.panel.render([item("stack", { quantity: 4 })], []);
  choose("drop", "stack");
  assert.equal(f.dom.inventoryActionDialog.open, true);
  f.dom.inventoryDropQuantity.value = "2";
  f.dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(f.commands.at(-1), { type: "drop-quantity", itemId: "stack", quantity: 2 });
  choose("destroy", "stack");
  f.dom.inventoryDropQuantity.value = "1";
  f.dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(f.commands.at(-1), { type: "destroy-item", itemId: "stack", quantity: 1 });
  choose("inscribe", "stack");
  f.dom.inventoryInscription.value = "keep";
  f.dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(f.commands.at(-1), { type: "inscribe-item", itemId: "stack", inscription: "keep" });

  f.panel.render([item("sword", { equipmentSlot: "weapon", inscription: "keep", throwTargetSpec: spec })], []);
  choose("equip", "sword");
  assert.deepEqual(f.commands.at(-1), { type: "equip", itemId: "sword" });
  choose("uninscribe", "sword");
  assert.deepEqual(f.commands.at(-1), { type: "inscribe-item", itemId: "sword", inscription: null });
  choose("throw", "sword");
  assert.deepEqual(f.targets.at(-1), [spec, { type: "throw", itemId: "sword" }]);
  const worn = { ...item("worn", { usable: true, activation: { nameKey: "effect" }, useTargetSpec: spec }), slotId: "body" };
  f.panel.render([], [worn]);
  choose("activate", "worn");
  assert.deepEqual(f.targets.at(-1), [spec, { type: "item", itemId: "worn" }]);
  choose("unequip", "worn");
  assert.deepEqual(f.commands.at(-1), { type: "unequip", slotId: "body" });
  const light = { ...item("lamp", { fuel: { kind: "lantern", current: 1, maximum: 20 } }), slotId: "light" };
  f.panel.render([item("oil", { fuel: { kind: "oil", current: 10 } })], [light]);
  choose("refuel", "lamp");
  assert.deepEqual(f.commands.at(-1), { type: "refuel-light", targetItemId: "lamp", sourceItemId: "oil" });
  const before = f.commands.length;
  f.panel.openCommand("inspect");
  f.document.body.children.at(-1).close();
  assert.equal(f.commands.length, before, "cancelled selection never dispatches");
  f.state.status.items = [{ id: "adjacent", kindId: "chest", displayNameKey: "chest", position: { x: 2, y: 1 } }];
  f.panel.selectChest("open-chest", f.state.status.items);
  const chestForm = f.document.body.children.at(-1).children[0];
  chestForm.children[1].children[1].value = "adjacent";
  chestForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(f.commands.at(-1), { type: "open-chest", itemId: "adjacent" });
  f.state.busy = true;
  f.panel.openCommand("refuel");
  assert.equal(f.document.body.children.length, 0);
});

function item(id, extra = {}) {
  return {
    id, kindId: `item.${id}`, displayNameKey: id, visual: { id: `item.${id}`, glyph: "?" },
    usable: false, equipmentSlot: null, quantity: 1, weightTenthsPound: 10,
    identification: "unexamined",
    modifiers: { attack: 0, defense: 0, maxHp: 0, speed: 0 },
    ...extra,
  };
}

test("ring swap dispatches the two body slots directly and blocks empty or unavailable contexts", t => {
  const f = createInventoryFixture(t);
  f.state.mode = "playing";
  f.state.bodySlots = [{ id: "finger-a", slotType: "ring" }, { id: "finger-b", slotType: "ring" }];
  f.panel.swapRings();
  assert.equal(f.commands.length, 0);
  f.state.equipment = [{ ...item("ring"), slotId: "finger-b" }];
  f.panel.swapRings();
  assert.deepEqual(f.commands, [{ type: "swap-rings", firstSlotId: "finger-a", secondSlotId: "finger-b" }]);
  assert.equal(f.document.body.children.length, 0);
  f.state.busy = true; f.panel.swapRings(); f.state.busy = false;
  f.state.status.mapScale = "world"; f.panel.swapRings();
  assert.equal(f.commands.length, 1);
});

test("multiple ring slots choose distinct fingers and cancellation at either step commits nothing", t => {
  const f = createInventoryFixture(t);
  f.state.mode = "playing";
  f.state.bodySlots = [1, 2, 3, 4, 5, 6].map(n => ({ id: `ring-${n}`, slotType: "ring" }));
  f.state.equipment = [{ ...item("ring"), slotId: "ring-6" }];
  const select = id => {
    const form = f.document.body.children[0].children[0];
    form.children[1].children[1].value = id;
    form.dispatchEvent(new Event("submit", { cancelable: true }));
  };
  f.panel.swapRings(); f.document.body.children[0].close();
  assert.equal(f.commands.length, 0);
  f.panel.swapRings(); select("ring-3");
  let second = f.document.body.children[0];
  assert.deepEqual(second.children[0].children[1].children[1].children.map(option => option.value),
    ["ring-1", "ring-2", "ring-4", "ring-5", "ring-6"]);
  second.close(); assert.equal(f.commands.length, 0);
  f.panel.swapRings(); select("ring-3"); select("ring-6");
  assert.deepEqual(f.commands, [{ type: "swap-rings", firstSlotId: "ring-3", secondSlotId: "ring-6" }]);
  assert.equal(f.document.body.children.length, 0);
});

test("body-slot cards retain repeated slots and empty-slot choices target the exact slot", (t) => {
  const { panel, dom, state, document, commands } = createInventoryFixture(t);
  state.bodySlots = [
    { id: "hand-a", slotType: "weapon" },
    { id: "hand-b", slotType: "weapon" },
    { id: "hand-c", slotType: "weapon" },
    { id: "shell", slotType: "body" },
  ];
  const sword = item("sword", { equipmentSlot: "weapon" });
  const tool = item("pick", { equipmentSlot: "tool" });
  const armor = item("armor", { equipmentSlot: "body" });
  const equipment = [{ ...item("worn-sword"), slotId: "hand-a" }];
  panel.render([sword, tool, armor], equipment);
  dom.equipmentList.scrollTop = 45;
  panel.render([sword, tool, armor], equipment);
  assert.equal(dom.equipmentList.scrollTop, 45);
  assert.deepEqual(dom.equipmentList.children.map((row) => row.dataset.slotId), ["hand-a", "hand-b", "hand-c", "shell"]);
  const third = dom.equipmentList.children[2].children[0];
  assert.match(third.children[0].textContent, /"ordinal":3/);
  assert.match(third.children[2].textContent, /^equipment-slot-vacant/);
  assert.equal(itemFitsBodySlot(tool, state.bodySlots[2]), true);
  assert.equal(itemFitsBodySlot(armor, state.bodySlots[2]), false);
  third.dispatchEvent(new Event("click"));
  const chooser = document.body.children[0];
  const form = chooser.children[0];
  const select = form.children[1].children[1];
  assert.deepEqual(select.children.map((option) => option.value), ["sword", "pick"]);
  select.value = "pick";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands, [{ type: "equip", itemId: "pick", slotId: "hand-c" }]);
});

test("compact rows keep multi-selection and show live details without losing list scroll", (t) => {
  const { panel, dom, state } = createInventoryFixture(t);
  const items = [item("sword", { equipmentSlot: "weapon", quantity: 3, modifiers: { attack: 7, defense: 0, maxHp: 0, speed: 0 } }), item("potion")];
  items[0].visual.glyph = "/";
  panel.render(items, []);
  const row = dom.inventoryList.children[0];
  const label = row.children[0];
  assert.equal(label.children.length, 6);
  assert.equal(label.children[1].textContent, "/");
  assert.match(label.children[5].textContent, /"weight":"3.0"/);
  assert.equal(label.children.some((child) => child.className === "item-modifier"), false);
  for (const checkbox of dom.inventoryList.querySelectorAll('input[type="checkbox"]')) {
    checkbox.checked = true;
    checkbox.dispatchEvent(new Event("change"));
  }
  row.children[1].dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryDetailDialog.open, true);
  assert.equal(state.selectedInventoryIds.size, 2);
  assert.ok(dom.inventoryDetailBody.children.some((child) => child.className === "item-modifier"));
  dom.inventoryList.scrollTop = 120;
  dom.equipmentList.scrollTop = 60;
  dom.inventoryDetailBody.scrollTop = 30;
  panel.render([{ ...items[0], displayNameKey: "identified-sword" }, items[1]], []);
  assert.equal(dom.inventoryList.scrollTop, 120);
  assert.equal(dom.inventoryDetailBody.scrollTop, 30);
  assert.equal(dom.inventoryDetailTitle.textContent, "identified-sword");
  panel.render([items[1]], []);
  assert.equal(dom.inventoryDetailDialog.open, false);
  assert.deepEqual([...state.selectedInventoryIds], ["potion"]);
});

test("throwing from item details uses the core target spec and respects busy input", (t) => {
  const { panel, dom, state, targets, commands } = createInventoryFixture(t);
  const throwTargetSpec = { modes: ["direction"], range: 9, requiresLineOfEffect: true };
  const weapon = item("hammer", { throwTargetSpec });
  panel.render([weapon], []);
  panel.openDetail(weapon.id);
  const button = dom.inventoryDetailActions.children.find((child) => child.textContent.startsWith("action-inventory-throw"));
  state.busy = true;
  button.dispatchEvent(new Event("click"));
  assert.equal(targets.length, 0);
  state.busy = false;
  button.dispatchEvent(new Event("click"));
  assert.deepEqual(targets, [[throwTargetSpec, { type: "throw", itemId: weapon.id }]]);
  assert.equal(dom.inventoryDetailDialog.open, false);
  assert.deepEqual(commands, []);
});

test("bag details show the known final capacity and source ego without redundant identification labels", (t) => {
  const { panel, dom } = createInventoryFixture(t);
  const bag = item("bag", { equipmentSlot: "container" });
  panel.render([bag], []);
  dom.inventoryList.children[0].children[1].dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryDetailBody.children.some((child) => child.className === "inventory-bag-capacity"), false);
  panel.render([], [{ ...bag, slotId: "container", identification: "identified", bagCapacity: 8,
    knownProperties: [{ affixId: "rfb-legacy.affix.holding-quiver", nameKey: "affix-legacy-holding-quiver-name" }],
  }]);
  const details = dom.inventoryDetailBody.children;
  assert.match(details.find((child) => child.className === "inventory-bag-capacity").textContent, /"capacity":8/);
  assert.match(details.find((child) => child.className === "item-property").textContent, /affix-legacy-holding-quiver-name/);
  assert.equal(details.some((child) => child.className?.startsWith("item-identification")), false);
});

test("stored items can be inspected without inventory actions or commands", (t) => {
  const { panel, dom, state, commands } = createInventoryFixture(t);
  const stored = item("stored", { displayNameKey: "appearance.potion.blue", knowledge: "unknown" });
  state.status.homes = [{ storedItems: [{ id: stored.id, details: stored }], depositItems: [] }];
  panel.render([], []);
  panel.openDetail(stored.id);
  assert.equal(dom.inventoryDetailDialog.open, true);
  assert.equal(dom.inventoryDetailTitle.textContent, "appearance.potion.blue");
  assert.equal(dom.inventoryDetailActions.children.length, 0);
  assert.deepEqual(commands, []);
  state.status.homes = [];
  panel.render([], []);
  assert.equal(dom.inventoryDetailDialog.open, false);
});

test("equipped details reuse refuel and unequip commands and retain activation availability", (t) => {
  const { panel, dom, commands } = createInventoryFixture(t);
  const source = item("oil", { fuel: { kind: "oil", current: 20, maximum: 20 } });
  const lamp = { ...item("lamp", {
    fuel: { kind: "lantern", current: 5, maximum: 30 },
    activation: { nameKey: "activation", power: 1, cost: 1 },
  }), slotId: "light" };
  panel.render([source], [lamp]);
  dom.equipmentList.children[0].children[0].dispatchEvent(new Event("click"));
  const buttons = () => dom.inventoryDetailActions.querySelectorAll("button");
  assert.equal(buttons()[0].disabled, true);
  assert.equal(buttons()[1].disabled, false);
  panel.updateActions();
  assert.equal(buttons()[0].disabled, true);
  buttons()[1].dispatchEvent(new Event("click"));
  assert.deepEqual(commands[0], { type: "refuel-light", targetItemId: "lamp", sourceItemId: "oil" });
  panel.render([source], [{ ...lamp, usable: true }]);
  assert.equal(buttons()[0].disabled, false);
  buttons()[0].dispatchEvent(new Event("click"));
  assert.deepEqual(commands[1], { type: "use-item", itemId: "lamp" });
  buttons()[2].dispatchEvent(new Event("click"));
  assert.deepEqual(commands[2], { type: "unequip", slotId: "light" });
  panel.render([source], [{ ...lamp, usable: false, useUnavailableReason: "berserker" }]);
  assert.ok(dom.inventoryDetailBody.children.some(child => child.className === "inventory-use-unavailable"
    && child.textContent.includes("item-use-unavailable-berserker")));
  panel.updateActions();
  assert.equal(buttons()[0].disabled, true);
  buttons()[0].dispatchEvent(new Event("click"));
  assert.equal(commands.length, 3);
  panel.render([source, item("lamp")], []);
  assert.equal(dom.inventoryDetailDialog.open, true);
  assert.equal(buttons().length, 0);
});

test("sensed items display the feeling while retaining the appraisal action", (t) => {
  const { panel, dom, state } = createInventoryFixture(t);
  const sensed = item("sensed-blade", { equipmentSlot: "weapon", feeling: "excellent", identification: "unexamined" });
  panel.render([sensed], []);
  const row = dom.inventoryList.children[0];
  assert.ok(row.children[0].children.some((child) => child.textContent?.includes("item-feeling-excellent")));
  row.children[1].dispatchEvent(new Event("click"));
  assert.ok(dom.inventoryDetailBody.children.some((child) => child.className === "item-feeling"
    && child.textContent.includes("item-feeling-excellent")));
  state.selectedInventoryIds.add(sensed.id);
  panel.updateActions();
  assert.equal(dom.inventoryAppraise.disabled, false);
});

test("capture-ball details retain the core use restriction through action refresh", (t) => {
  const { panel, dom, commands } = createInventoryFixture(t);
  const ball = { ...item("ball", { captureBall: true, useTargetSpec: { modes: ["self"] } }), slotId: "light" };
  for (const useUnavailableReason of [undefined, "berserker"]) {
    panel.render([], [{ ...ball, useUnavailableReason }]);
    panel.openDetail("ball");
    panel.updateActions();
    const activate = dom.inventoryDetailActions.querySelectorAll("button")[0];
    assert.equal(activate.disabled, Boolean(useUnavailableReason));
    activate.dispatchEvent(new Event("click"));
  }
  assert.deepEqual(commands, [{ type: "use-item", itemId: "ball", target: { type: "self" } }]);
});

test("footer actions reflect selection capabilities while temporary unavailability disables them", (t) => {
  const { panel, dom, state } = createInventoryFixture(t);
  const potion = item("potion", { usable: true, mountUsable: true });
  panel.render([potion, item("sword", { equipmentSlot: "weapon" })], []);
  assert.equal(dom.inventoryUse.hidden, true);
  assert.equal(dom.inventoryMore.hidden, true);
  state.selectedInventoryIds.add("potion");
  panel.updateActions();
  assert.equal(dom.inventoryUse.hidden, false);
  assert.equal(dom.inventoryEquip.hidden, true);
  assert.equal(dom.inventoryUseOnMount.hidden, true);
  state.status.player.ridingActorId = "mount";
  panel.updateActions();
  assert.equal(dom.inventoryUseOnMount.hidden, false);
  assert.equal(dom.inventoryAppraise.hidden, false);
  state.busy = true;
  panel.updateActions();
  assert.equal(dom.inventoryUse.hidden, false);
  assert.equal(dom.inventoryUse.disabled, true);
  state.busy = false;
  state.selectedInventoryIds.add("sword");
  panel.updateActions();
  assert.equal(dom.inventoryUse.hidden, true);
  assert.equal(dom.inventoryInscribe.hidden, true);
  assert.equal(dom.inventoryDrop.hidden, false);
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryActionDialog.open, false);
});

test("quantity entry validates partial drops, preserves edits and cancels stale selection", (t) => {
  const { panel, dom, state, commands } = createInventoryFixture(t);
  const stack = item("stack", { quantity: 5 });
  panel.render([stack], []);
  state.selectedInventoryIds.add(stack.id);
  panel.updateActions();
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryActionDialog.open, true);
  assert.equal(dom.inventoryInscriptionField.hidden, true);
  assert.equal(dom.inventoryInscription.disabled, true);
  dom.inventoryDropQuantity.value = "6";
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(dom.inventoryActionConfirm.disabled, true);
  assert.equal(commands.length, 0);
  dom.inventoryDropQuantity.value = "2";
  panel.render([stack], []);
  assert.equal(dom.inventoryDropQuantity.value, "2");
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands, [{ type: "drop-quantity", itemId: "stack", quantity: 2 }]);
  assert.equal(dom.inventoryActionDialog.open, false);
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  dom.inventoryActionCancel.dispatchEvent(new Event("click"));
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(commands.length, 1);
  dom.inventoryDrop.dispatchEvent(new Event("click"));
  panel.render([item("replacement")], []);
  state.selectedInventoryIds.add("replacement");
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(dom.inventoryActionDialog.open, false);
  assert.equal(commands.length, 1);
});

test("more actions use temporary inscription input and retain explicit destruction confirmation", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const stack = item("stack", { quantity: 5, inscription: "old" });
  panel.render([stack], []);
  state.selectedInventoryIds.add(stack.id);
  panel.updateActions();
  dom.inventoryMore.dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryMoreDialog.open, true);
  dom.inventoryInscribe.dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryMoreDialog.open, false);
  assert.equal(dom.inventoryActionDialog.open, true);
  assert.equal(dom.inventoryDropQuantity.disabled, true);
  assert.equal(dom.inventoryQuantityField.hidden, true);
  dom.inventoryInscription.value = "new";
  document.activeElement = dom.inventoryActionConfirm;
  panel.render([stack], []);
  assert.equal(dom.inventoryInscription.value, "new");
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[0], { type: "inscribe-item", itemId: "stack", inscription: "new" });
  dom.inventoryInscribe.dispatchEvent(new Event("click"));
  dom.inventoryInscription.value = "";
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[1], { type: "inscribe-item", itemId: "stack", inscription: null });

  let accepted = false;
  document.defaultView = { confirm: () => accepted };
  dom.inventoryDestroy.dispatchEvent(new Event("click"));
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(commands.length, 2);
  accepted = true;
  dom.inventoryDestroy.dispatchEvent(new Event("click"));
  dom.inventoryDropQuantity.value = "3";
  dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[2], { type: "destroy-item", itemId: "stack", quantity: 3 });
});

test("using items starts map targeting only for map targets and preserves recharge pairing", (t) => {
  const { panel, dom, state, targets, commands, document } = createInventoryFixture(t);
  const potion = item("potion", { usable: true, useTargetSpec: { modes: ["self"] } });
  panel.render([potion], []);
  state.selectedInventoryIds.add("potion");
  dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.deepEqual(commands, [{ type: "use-item", itemId: "potion", target: { type: "self" } }]);
  assert.equal(targets.length, 0);
  const spec = { modes: ["entity"], range: 5 };
  panel.render([{ ...potion, useTargetSpec: spec }], []);
  dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.deepEqual(targets, [[spec, { type: "item", itemId: "potion" }]]);
  const wand = item("wand", { requiresRechargeTargets: true, usable: true });
  const source = item("source", { canSupplyRecharge: true });
  const target = item("target", { canReceiveRecharge: true });
  panel.render([wand, source, target], []);
  state.selectedInventoryIds.add("wand");
  state.selectedInventoryIds.add("source");
  panel.updateActions();
  assert.equal(dom.inventoryUse.hidden, false);
  dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.equal(targets.length, 1);
  const chooser = document.body.children[0];
  assert.equal(chooser.open, true);
  const form = chooser.children[0];
  form.children[1].children[1].value = "target";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[1], { type: "use-item-for-recharge", itemId: "wand", sourceItemId: "source", targetItemId: "target" });
});

test("Jewel activation offers both recall choices and cancels without using the item", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  panel.render([item("jewel", { usable: true, activation: { recallChoice: true }, useTargetSpec: { modes: ["self"] } })], []);
  state.selectedInventoryIds.add("jewel");
  const open = () => {
    dom.inventoryUse.dispatchEvent(new Event("click"));
    return document.body.children[0];
  };
  for (const choice of ["no", "yes"]) {
    const dialog = open();
    const form = dialog.children[0];
    assert.match(form.children[0].textContent, /^jewel-recall-title/);
    assert.deepEqual(form.children[1].children[1].children.map(option => option.value), ["no", "yes"]);
    form.children[1].children[1].value = choice;
    form.dispatchEvent(new Event("submit", { cancelable: true }));
  }
  assert.deepEqual(commands, [
    { type: "use-jewel", itemId: "jewel", recall: false },
    { type: "use-jewel", itemId: "jewel", recall: true },
  ]);
  open().close();
  assert.equal(commands.length, 2);
  const dialog = open();
  state.busy = true;
  dialog.children[0].dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(commands.length, 2);
  assert.equal(dialog.open, true);
  dialog.close();
  open();
  assert.equal(document.body.children.length, 0);
  state.busy = false;
  const stale = open();
  panel.reset();
  stale.children[0].dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(commands.length, 2);
});

test("recharge activation selects distinct pack or ground devices and cancels either stage once", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const cloak = item("cloak", { usable: true, requiresRechargeTargets: true, activation: {} });
  const donor = item("donor", { canSupplyRecharge: true, canReceiveRecharge: true });
  panel.render([cloak, donor], []);
  state.status = { ...state.status, player: { ...state.status.player, position: { x: 1, y: 1 } }, items: [item("ground", { canReceiveRecharge: true }), item("full")] };
  state.selectedInventoryIds.add("cloak");
  panel.updateActions();
  assert.equal(dom.inventoryUse.disabled, false);
  const start = () => {
    dom.inventoryUse.dispatchEvent(new Event("click"));
    return document.body.children[0];
  };
  const selectDonor = () => {
    const source = start();
    const form = source.children[0];
    assert.match(form.children[0].textContent, /^inventory-recharge-source-title/);
    assert.deepEqual(form.children[1].children[1].children.map(option => option.value), ["donor"]);
    form.children[1].children[1].value = "donor";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
    return document.body.children[0];
  };
  start().close();
  selectDonor().close();
  assert.deepEqual(commands, [{ type: "use-item", itemId: "cloak" }, { type: "use-item", itemId: "cloak" }]);
  const target = selectDonor().children[0];
  assert.match(target.children[0].textContent, /^inventory-recharge-target-title/);
  assert.deepEqual(target.children[1].children[1].children.map(option => option.value), ["ground"]);
  target.children[1].children[1].value = "ground";
  target.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[2], { type: "use-item-for-recharge", itemId: "cloak", sourceItemId: "donor", targetItemId: "ground" });
  panel.render([donor], [{ ...cloak, slotId: "cloak" }]);
  dom.equipmentList.children[0].children[0].dispatchEvent(new Event("click"));
  dom.inventoryDetailActions.querySelectorAll("button")[0].dispatchEvent(new Event("click"));
  assert.equal(dom.inventoryDetailDialog.open, false);
  assert.match(document.body.children[0].children[0].children[0].textContent, /^inventory-recharge-source-title/);
  document.body.children[0].close();
  assert.deepEqual(commands[3], { type: "use-item", itemId: "cloak" });
});

test("item-use target cancellation reaches core once, while a confirmed target does not cancel", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const staff = item("staff", { usable: true, useTargetSpec: { modes: ["item"] } });
  panel.render([staff, item("target")], []);
  state.selectedInventoryIds.add("staff");
  dom.inventoryUse.dispatchEvent(new Event("click"));
  document.body.children[0].close();
  assert.deepEqual(commands, [{ type: "use-item", itemId: "staff" }]);
  dom.inventoryUse.dispatchEvent(new Event("click"));
  const form = document.body.children[0].children[0];
  form.children[1].children[1].value = "target";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[1], { type: "use-item", itemId: "staff", target: { type: "item", itemId: "target" } });
  assert.equal(commands.length, 2);
});

test("crafting confirms risky whole stacks and cancelling dispatches nothing", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const source = item("craft", { usable: true, requiresCraftingTarget: true });
  let accepted = false;
  const prompts = [];
  document.defaultView = { confirm: (message) => { prompts.push(message); return accepted; } };
  const choose = (quantity) => {
    panel.render([source, item("arrows", { quantity })], []);
    state.selectedInventoryIds.add("craft");
    dom.inventoryUse.dispatchEvent(new Event("click"));
    const form = document.body.children[0].children[0];
    form.children[1].children[1].value = "arrows";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
  };
  choose(31);
  assert.equal(commands.length, 0);
  assert.match(prompts[0], /"chance":3/);
  accepted = true;
  choose(59);
  assert.match(prompts[1], /"chance":97/);
  assert.deepEqual(commands[0], { type: "use-item", itemId: "craft", target: { type: "crafting-item", itemId: "arrows", quantity: 59 } });
  choose(30);
  assert.equal(prompts.length, 2);
  assert.equal(commands[1].target.quantity, 30);
});

test("Mundanity uses core targets and cancels resistance loss without a command", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const target = { type: "mundanity-item", itemId: "dragon", quantity: 1, confirmResistanceLoss: true };
  const source = item("scroll", { usable: true, mundanityTargets: [
    { itemId: "dragon", target, confirmationKey: "item-mundanity-resistance-confirm" },
  ] });
  let accepted = false;
  const prompts = [];
  document.defaultView = { confirm: message => { prompts.push(message); return accepted; } };
  const choose = () => {
    panel.render([source, item("dragon"), item("ineligible")], []);
    state.selectedInventoryIds.add("scroll");
    dom.inventoryUse.dispatchEvent(new Event("click"));
    const form = document.body.children[0].children[0];
    const select = form.children[1].children[1];
    assert.deepEqual(select.children.map(option => option.value), ["dragon"]);
    select.value = "dragon";
    form.dispatchEvent(new Event("submit", { cancelable: true }));
  };
  choose();
  assert.equal(commands.length, 0);
  assert.match(prompts[0], /item-mundanity-resistance-confirm/);
  accepted = true;
  choose();
  assert.deepEqual(commands, [{ type: "use-item", itemId: "scroll", target }]);
});

test("artifact creation uses core candidates, confirms stack loss, and distinguishes target and name cancellation", (t) => {
  const { panel, dom, state, commands, document } = createInventoryFixture(t);
  const source = item("scroll", { usable: true, artifactCreationTargets: ["target"] });
  let accepted = false;
  let name = "圆月";
  const prompts = [];
  document.defaultView = {
    confirm: (message) => { prompts.push(message); return accepted; },
    prompt: () => name,
  };
  const choose = (quantity) => {
    panel.render([source, item("target", { quantity }), item("ineligible")], []);
    state.selectedInventoryIds.add("scroll");
    dom.inventoryUse.dispatchEvent(new Event("click"));
    const form = document.body.children[0].children[0];
    const select = form.children[1].children[1];
    assert.deepEqual(select.children.map((option) => option.value), ["target"]);
    select.value = "target";
    return form;
  };
  choose(4).children[2].children[0].dispatchEvent(new Event("click"));
  assert.equal(commands.length, 0);
  choose(4).dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(commands.length, 0);
  assert.match(prompts[0], /"quantity":3/);
  accepted = true;
  choose(4).dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands[0], { type: "use-item", itemId: "scroll", target: {
    type: "artifact-creation-item", itemId: "target", quantity: 4, name: "圆月",
  } });
  for (name of [null, ""]) {
    choose(1).dispatchEvent(new Event("submit", { cancelable: true }));
    assert.deepEqual(commands.at(-1).target, { type: "artifact-creation-item", itemId: "target", quantity: 1 });
  }
  assert.equal(prompts.length, 2);
});

test("inscription reading uses core pack and floor candidates without activation targeting", (t) => {
  const { panel, dom, state, document, commands, targets } = createInventoryFixture(t);
  const ring = item("ring", { readable: true, usable: true, useTargetSpec: null,
    charges: { current: 0, maximum: 1 } });
  state.status.items = [
    { id: "floor-ring", kindId: "demo.item.one-ring", displayNameKey: "one-ring", readable: true },
    { id: "out-of-reach", kindId: "demo.item.one-ring", displayNameKey: "one-ring", readable: false },
  ];
  panel.render([ring, item("darnya", { readable: false })], []);
  assert.equal(dom.inventoryRead.hidden, false);
  const choose = () => {
    dom.inventoryRead.dispatchEvent(new Event("click"));
    const form = document.body.children[0].children[0];
    const select = form.children[1].children[1];
    assert.deepEqual(select.children.map(option => option.value), ["ring"]);
    selectionKey(document.body.children[0], "f", { ctrlKey: true });
    assert.deepEqual(select.children.map(option => option.value), ["floor-ring"]);
    select.value = "floor-ring";
    return form;
  };
  choose().children[2].children[0].dispatchEvent(new Event("click"));
  assert.deepEqual(commands, []);
  choose().dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(commands, [{ type: "use-item", itemId: "floor-ring" }]);
  state.selectedInventoryIds.add("ring");
  dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.deepEqual(commands.at(-1), { type: "use-item", itemId: "ring" });
  assert.deepEqual(targets, []);
  state.status.items = [];
  panel.render([{ ...ring, readable: false, usable: false }], []);
  assert.equal(dom.inventoryRead.hidden, true);
});

function createInventoryFixture(t) {
  // The controller's DOM boundary only; this does not simulate browser layout.
  class Element extends EventTarget {
    children = [];
    dataset = {};
    style = {};
    value = "";
    scrollTop = 0;
    open = false;
    attributes = new Map();
    constructor(tag = "div") { super(); this.tag = tag; }
    get tagName() { return this.tag.toUpperCase(); }
    get ownerDocument() { return document; }
    setAttribute(key, value) { this.attributes.set(key, value); }
    getAttribute(key) { return this.attributes.get(key); }
    setCustomValidity(message) { this.validationMessage = message; }
    reportValidity() { return !this.validationMessage; }
    focus() { document.activeElement = this; }
    showModal() { this.open = true; }
    close() { this.open = false; this.dispatchEvent(new Event("close")); }
    remove() { this.parentElement.children = this.parentElement.children.filter((child) => child !== this); }
    append(...children) {
      for (const child of children) {
        if (typeof child !== "string") child.parentElement = this;
        this.children.push(child);
      }
    }
    replaceChildren(...children) { this.children = []; this.scrollTop = 0; this.append(...children); }
    querySelectorAll(selector) {
      return this.children.flatMap((child) => {
        if (typeof child === "string") return [];
        const matches = ["button", "option"].includes(selector) ? child.tag === selector : child.tag === "input" && child.type === "checkbox";
        return [...(matches ? [child] : []), ...child.querySelectorAll(selector)];
      });
    }
  }
  const document = { createElement: (tag) => new Element(tag), activeElement: undefined, body: new Element() };
  const dom = Object.fromEntries([
    "inventoryCount", "inventoryFilters", "inventorySearch", "inventoryFilterReset",
    "inventorySelectionCount", "inventoryUse", "inventoryAbsorb", "inventoryRead", "inventoryUseOnMount",
    "inventoryAppraise", "inventoryEquip", "inventoryDrop", "inventoryDropQuantity",
    "inventoryInscription", "inventoryInscribe", "inventoryDestroy", "inventoryList", "equipmentList",
    "inventoryDetailDialog", "inventoryDetailTitle", "inventoryDetailBody", "inventoryDetailClose",
    "inventoryDetailActions", "inventoryMore", "inventoryMoreDialog", "inventoryActionDialog",
    "inventoryActionForm", "inventoryActionTitle", "inventoryActionItem", "inventoryQuantityField",
    "inventoryInscriptionField", "inventoryActionCancel", "inventoryActionConfirm",
  ].map((key) => [key, new Element()]));
  new Element().append(dom.inventoryList, dom.equipmentList);
  const radio = { value: "all" };
  dom.inventoryFilters.querySelector = (selector) => selector === "input:checked"
    ? radio : { set checked(value) { if (value) radio.value = "all"; } };
  const state = new AppState();
  state.mode = "playing";
  state.status = {
    items: [],
    player: { visualCatalog: [], carriedWeightTenthsPound: 10, carryCapacityTenthsPound: 100, inventoryUsedSlots: 2, inventorySlotCapacity: 26 },
  };
  const commands = [];
  const targets = [];
  const messages = [];
  const panel = new InventoryPanel({
    dom, state,
    localization: { format: (key, args) => `${key} ${JSON.stringify(args)}` },
    formatter: { visibleItemName: (key, _kind, name) => name ? `${key} ${name}` : key, equipmentSlotName: (slot) => slot, itemPropertyName: (key) => key },
    dispatch: async (command) => { commands.push(command); },
    startTargeting: (...args) => targets.push(args), updateCampaignAction: () => {}, announce: key => messages.push(key),
  });
  panel.install();
  t.after(() => panel.dispose());
  return { panel, dom, state, radio, commands, targets, messages, document };
}

test("selection sources use Core quiver membership without changing eligible candidates", (t) => {
  const { panel, state, document, commands } = createInventoryFixture(t);
  const pack = item("pack"), arrows = item("arrows"), overflow = item("overflow");
  panel.render([pack, arrows, overflow], [{ ...item("worn"), slotId: "light" }]);
  state.status.player = { ...state.status.player, position: { x: 4, y: 7 }, quiverItemIds: ["arrows", "stale"] };
  state.status.items = [
    { ...item("floor"), position: { x: 4, y: 7 } },
    { ...item("distant"), position: { x: 5, y: 7 } },
  ];
  assert.deepEqual(["pack", "arrows", "overflow", "worn", "floor", "stale"].map(id => itemSelectionSource(state, id)),
    ["pack", "quiver", "pack", "equipment", "floor", undefined]);
  assert.equal(itemSelectionSource(state, "distant"), "floor", "source is not permission to select");
  assert.deepEqual(itemTargetCandidates(state, "worn", name => name).map(candidate => candidate.id),
    ["pack", "arrows", "overflow", "floor"]);
  panel.selectItemTarget("pack", async () => {}, undefined, ["arrows", "worn", "floor", "distant"]);
  const dialog = document.body.children[0];
  const options = [];
  for (const key of ["e", "q", "f"]) {
    selectionKey(dialog, key, { ctrlKey: true });
    options.push(...document.body.querySelectorAll("option").map(option => [option.value, option.dataset.source]));
  }
  assert.deepEqual(options, [["worn", "equipment"], ["arrows", "quiver"], ["floor", "floor"]]);
  assert.deepEqual(commands, [], "source projection and opening selection do not execute actions");
  state.status.player.quiverItemIds = [];
  assert.equal(itemSelectionSource(state, "arrows"), "pack");
  state.inventory = state.inventory.filter(entry => entry.id !== "arrows");
  assert.equal(itemSelectionSource(state, "arrows"), undefined);
});

function selectionKey(dialog, key, extra = {}) {
  const event = new Event("keydown", { cancelable: true });
  for (const [name, value] of Object.entries({ key, ...extra })) {
    Object.defineProperty(event, name, { value });
  }
  dialog.dispatchEvent(event);
  return event;
}

test("item letters page all candidates and mouse/Enter resolve the displayed instance", t => {
  const f = createInventoryFixture(t), selected = [];
  f.panel.render(Array.from({ length: 53 }, (_, index) => item(`item-${index + 1}`)), []);
  const open = () => {
    f.panel.selectItemTarget(undefined, async id => { selected.push(id); });
    return f.document.body.children.at(-1);
  };
  let dialog = open(), form = dialog.children[0], select = form.children[1].children[1];
  assert.equal(select.children.length, 26);
  assert.equal(select.children[0].textContent, "a) item-1");
  assert.equal(select.children[25].textContent, "z) item-26");
  selectionKey(dialog, "z");
  assert.deepEqual(selected, ["item-26"]);
  dialog = open(); form = dialog.children[0]; select = form.children[1].children[1];
  selectionKey(dialog, "PageDown");
  assert.equal(select.children[0].textContent, "a) item-27");
  select.value = "item-1";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(selected.length, 1, "hidden pages cannot be submitted");
  selectionKey(dialog, "a");
  assert.equal(selected.at(-1), "item-27");
  dialog = open(); form = dialog.children[0]; select = form.children[1].children[1];
  selectionKey(dialog, "PageUp");
  assert.deepEqual(select.children.map(option => option.value), ["item-53"]);
  selectionKey(dialog, "b");
  assert.equal(dialog.open, true, "unused letters do not wrap onto another page");
  selectionKey(dialog, " ");
  assert.equal(select.value, "item-1", "last page wraps to first");
  form.children[3].children[0].dispatchEvent(new Event("click"));
  selectionKey(dialog, "Enter");
  assert.equal(selected.at(-1), "item-53");
  dialog = open(); form = dialog.children[0]; select = form.children[1].children[1];
  form.children[3].children[2].dispatchEvent(new Event("click"));
  select.value = "item-28";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(selected.at(-1), "item-28");
  assert.deepEqual(f.commands, [], "selection itself never dispatches a game command");
});

test("uppercase inspection is read-only and keyboard selection respects input boundaries", t => {
  const f = createInventoryFixture(t);
  f.panel.render([item("potion", { usable: true, useCategory: "potion", inscription: "keep" })], []);
  f.panel.openCommand("potion");
  const dialog = f.document.body.children.at(-1), form = dialog.children[0];
  assert.equal(form.children[3].hidden, true, "one candidate needs no paging controls");
  selectionKey(dialog, "A");
  assert.equal(dialog.open, true);
  assert.equal(form.children[5].hidden, false);
  assert.equal(form.children[5].children[0].textContent, "potion");
  assert.equal(form.children[5].querySelectorAll("button").length, 0);
  assert.deepEqual(f.commands, []);
  for (const extra of [{ isComposing: true }, { ctrlKey: true }, { altKey: true }, { metaKey: true }, { repeat: true },
    { target: f.document.createElement("input") }, { target: f.document.createElement("textarea") }, { target: { isContentEditable: true } }]) {
    selectionKey(dialog, "a", extra);
  }
  for (const key of ["Tab", "ArrowDown", "ArrowUp"]) {
    assert.equal(selectionKey(dialog, key).defaultPrevented, false);
  }
  for (const key of ["Enter", " "]) {
    assert.equal(selectionKey(dialog, key, { target: form.children[2].children[0] }).defaultPrevented, false);
  }
  f.state.busy = true; selectionKey(dialog, "a"); f.state.busy = false;
  f.state.mode = "title"; selectionKey(dialog, "a"); f.state.mode = "playing";
  f.state.status.mapScale = "world"; selectionKey(dialog, "a"); f.state.status.mapScale = "local";
  assert.deepEqual(f.commands, []);
  selectionKey(dialog, "a");
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "potion" }]);
  selectionKey(dialog, "a");
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(f.commands.length, 1, "closed callbacks cannot execute again");
});

test("item labels keep their IDs on reorder and reject removed, moved, or replaced snapshots", t => {
  const f = createInventoryFixture(t), selected = [];
  let cancelled = 0;
  const open = () => {
    f.panel.render([item("first"), item("second")], []);
    f.panel.selectItemTarget(undefined, async id => { selected.push(id); }, async () => { cancelled++; });
    return f.document.body.children.at(-1);
  };
  let dialog = open();
  f.state.inventory.reverse();
  selectionKey(dialog, "a");
  assert.deepEqual(selected, ["first"]);
  dialog = open();
  f.state.inventory.shift();
  selectionKey(dialog, "a");
  assert.equal(selected.length, 1, "removed a is not reassigned to b");
  assert.equal(f.messages.at(-1), "message-item-selection-stale");
  selectionKey(dialog, "b");
  assert.equal(selected.at(-1), "second");
  dialog = open();
  f.state.equipment = [{ ...f.state.inventory.shift(), slotId: "body" }];
  selectionKey(dialog, "a");
  assert.equal(selected.length, 2, "moving an instance to another source invalidates its old entry");
  selectionKey(dialog, "Escape");
  assert.equal(cancelled, 1);
  dialog = open();
  f.state.status = { ...f.state.status };
  selectionKey(dialog, "a");
  assert.equal(selected.length, 2);
  selectionKey(dialog, "Escape");
  assert.equal(cancelled, 1, "an old snapshot cannot invoke a paid cancellation callback");
});

test("empty and cancelled selections settle once while replacement and disposal are silent", t => {
  const f = createInventoryFixture(t);
  let cancelled = 0, selected = 0;
  const open = () => {
    f.panel.selectItemTarget(undefined, async () => { selected++; }, async () => { cancelled++; });
    return f.document.body.children.at(-1);
  };
  open();
  assert.equal(f.document.body.children.length, 0);
  assert.equal(f.messages.at(-1), "message-item-selection-empty");
  assert.equal(cancelled, 1);
  f.panel.render([item("only")], []);
  let dialog = open();
  selectionKey(dialog, "Escape");
  dialog.dispatchEvent(new Event("close"));
  assert.equal(cancelled, 2);
  dialog = open();
  dialog.children[0].children[2].children[0].dispatchEvent(new Event("click"));
  assert.equal(cancelled, 3);
  const replaced = open();
  dialog = open();
  assert.equal(replaced.open, false);
  assert.equal(cancelled, 3);
  selectionKey(dialog, "Enter");
  assert.equal(selected, 1);
  assert.equal(cancelled, 3);
  dialog = open();
  f.panel.dispose();
  assert.equal(dialog.open, false);
  selectionKey(dialog, "a");
  assert.equal(selected, 1);
  assert.equal(cancelled, 3);
});

test("source controls split Core quiver membership and retain each source's page", t => {
  const f = createInventoryFixture(t), chosen = [];
  const pack = Array.from({ length: 27 }, (_, i) => item(`pack-${i}`));
  const quiver = Array.from({ length: 28 }, (_, i) => item(`quiver-${i}`));
  f.panel.render([...pack, ...quiver], [{ ...item("worn"), slotId: "body" }]);
  f.state.status.player = { ...f.state.status.player, position: { x: 2, y: 3 }, quiverItemIds: quiver.map(item => item.id) };
  f.state.status.items = [item("floor", { position: { x: 2, y: 3 } }), item("distant", { position: { x: 3, y: 3 } })];
  f.panel.selectItemTarget(undefined, async id => { chosen.push(id); });
  const dialog = f.document.body.children[0], form = dialog.children[0], select = form.children[1].children[1];
  const buttons = form.children[6].children;
  assert.deepEqual(buttons.map(button => button.dataset.source), ["pack", "equipment", "quiver", "floor"]);
  assert.equal(buttons[0].getAttribute("aria-pressed"), "true");
  assert.equal(select.value, "pack-0");
  selectionKey(dialog, "PageDown");
  assert.equal(select.value, "pack-26");
  selectionKey(dialog, "/");
  assert.equal(select.value, "worn");
  assert.equal(form.children[3].hidden, true);
  selectionKey(dialog, "/");
  assert.equal(select.value, "quiver-0");
  selectionKey(dialog, "PageDown");
  assert.deepEqual(select.children.map(option => option.value), ["quiver-26", "quiver-27"]);
  assert.equal(select.children[0].textContent, "a) quiver-26");
  for (const extra of [{ repeat: true }, { isComposing: true }, { altKey: true }, { metaKey: true },
    { target: f.document.createElement("input") }]) {
    selectionKey(dialog, "p", { ctrlKey: true, ...extra });
    assert.equal(select.value, "quiver-26");
  }
  assert.equal(selectionKey(dialog, "P", { ctrlKey: true }).defaultPrevented, true);
  assert.equal(select.value, "pack-26");
  buttons[2].dispatchEvent(new Event("click"));
  assert.equal(select.value, "quiver-26", "mouse source switch restores the same page");
  selectionKey(dialog, "f", { ctrlKey: true });
  assert.deepEqual(select.children.map(option => option.value), ["floor"]);
  assert.equal(chosen.length, 0, "Ctrl+F only switches, even with a sole floor candidate");
  selectionKey(dialog, "/");
  assert.equal(select.value, "pack-26");
  selectionKey(dialog, "\\");
  assert.equal(select.value, "floor");
  selectionKey(dialog, "q", { ctrlKey: true });
  select.value = "pack-26";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(chosen.length, 0, "another source cannot be submitted by forging a select value");
  selectionKey(dialog, "b");
  assert.deepEqual(chosen, ["quiver-27"]);
});

test("unavailable sources stay absent and floor shortcut uses only eligible underfoot items", t => {
  const f = createInventoryFixture(t);
  f.panel.render([item("potion", { usable: true, useCategory: "potion" }), item("junk")], [{ ...item("worn"), slotId: "body" }]);
  f.state.status.player.position = { x: 2, y: 3 };
  f.state.status.items = [item("floor", { position: { x: 2, y: 3 } }), item("distant", { position: { x: 3, y: 3 } })];
  f.panel.openCommand("potion");
  let dialog = f.document.body.children[0], form = dialog.children[0];
  assert.deepEqual(form.children[6].children.map(button => button.dataset.source), ["pack"]);
  for (const key of ["e", "q", "f"]) {
    assert.equal(selectionKey(dialog, key, { ctrlKey: true }).defaultPrevented, true);
  }
  selectionKey(dialog, "-");
  assert.equal(form.children[1].children[1].value, "potion");
  assert.deepEqual(f.commands, []);
  selectionKey(dialog, "a");
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "potion" }]);
  f.panel.openCommand("unequip");
  dialog = f.document.body.children[0]; form = dialog.children[0];
  assert.deepEqual(form.children[6].children.map(button => button.dataset.source), ["equipment"]);
  selectionKey(dialog, "a");
  assert.deepEqual(f.commands.at(-1), { type: "unequip", slotId: "body" });
  const selected = [];
  f.panel.selectItemTarget(undefined, async id => { selected.push(id); }, undefined, ["potion", "floor", "distant"]);
  dialog = f.document.body.children[0];
  selectionKey(dialog, "-", { repeat: true });
  assert.deepEqual(selected, []);
  selectionKey(dialog, "-");
  assert.deepEqual(selected, ["floor"], "the distant object cannot prevent or receive sole-item selection");
  assert.equal(dialog.open, false);
  f.state.status.items.push(item("second", { position: { x: 2, y: 3 } }));
  f.panel.selectItemTarget(undefined, async id => { selected.push(id); });
  dialog = f.document.body.children[0];
  selectionKey(dialog, "-");
  assert.equal(dialog.open, true, "multiple floor items require an explicit choice");
  assert.deepEqual(dialog.children[0].children[1].children[1].children.map(option => option.value), ["floor", "second"]);
  selectionKey(dialog, "b");
  assert.deepEqual(selected, ["floor", "second"]);
});

test("source cycling preserves item-use cancellation cost and floor shortcut dispatches the resolved target", t => {
  const f = createInventoryFixture(t);
  const staff = item("staff", { usable: true, useTargetSpec: { modes: ["item"] } });
  f.panel.render([staff, item("pack-target")], []);
  f.state.status.player.position = { x: 0, y: 0 };
  f.state.status.items = [item("floor-target", { position: { x: 0, y: 0 } })];
  f.state.selectedInventoryIds.add("staff");
  const open = () => {
    f.dom.inventoryUse.dispatchEvent(new Event("click"));
    return f.document.body.children[0];
  };
  let dialog = open();
  selectionKey(dialog, "f", { ctrlKey: true });
  selectionKey(dialog, "/");
  assert.deepEqual(f.commands, []);
  selectionKey(dialog, "Escape");
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "staff" }]);
  dialog = open();
  selectionKey(dialog, "-");
  assert.deepEqual(f.commands.at(-1), { type: "use-item", itemId: "staff", target: { type: "item", itemId: "floor-target" } });
  assert.equal(f.commands.length, 2);
});

test("inscription labels override keys before paging and @ only toggles labels", t => {
  const f = createInventoryFixture(t), chosen = [];
  f.panel.render(Array.from({ length: 27 }, (_, i) => item(`target-${i}`, {
    inscription: i === 0 ? "中文 @m3" : i === 1 ? "@mA" : i === 2 ? "@0" : i === 26 ? "@m9" : null,
  })), []);
  const open = () => {
    f.panel.selectItemTarget(undefined, async id => { chosen.push(id); }, undefined, undefined, "cast");
    return f.document.body.children[0];
  };
  let dialog = open(), select = dialog.children[0].children[1].children[1];
  assert.equal(select.children[0].textContent, "3) target-0");
  selectionKey(dialog, "3");
  assert.deepEqual(chosen, ["target-0"]);
  dialog = open();
  selectionKey(dialog, "A");
  assert.equal(chosen.at(-1), "target-1", "an exact uppercase label selects instead of inspecting");
  dialog = open(); selectionKey(dialog, "0");
  assert.equal(chosen.at(-1), "target-2");
  dialog = open(); select = dialog.children[0].children[1].children[1];
  selectionKey(dialog, "@");
  assert.equal(select.children[0].textContent, "a) target-0");
  assert.equal(dialog.children[0].children[7].getAttribute("aria-pressed"), "true");
  selectionKey(dialog, "3");
  assert.equal(select.value, "target-26", "unassigned 3 is page down");
  dialog.children[0].children[7].dispatchEvent(new Event("click"));
  assert.equal(select.children[0].textContent, "9) target-26");
  selectionKey(dialog, "9");
  assert.equal(chosen.at(-1), "target-26");
  dialog = open(); selectionKey(dialog, "@"); selectionKey(dialog, "9"); selectionKey(dialog, "9");
  assert.equal(dialog.children[0].children[1].children[1].value, "target-0", "unassigned 9 is page up with wrap");
  assert.equal(f.state.inventory[0].inscription, "中文 @m3", "the toggle never edits inscription text");
});

test("inscription confirmation gates letters, mouse and Enter even when labels are ignored", t => {
  const f = createInventoryFixture(t);
  f.panel.render([item("potion", { usable: true, useCategory: "potion", inscription: "@q1 !q!*" })], []);
  let answers = [false], confirmations = 0;
  f.document.defaultView = { confirm: () => { confirmations++; return answers.shift() ?? false; } };
  f.panel.openCommand("potion");
  const dialog = f.document.body.children[0], form = dialog.children[0];
  selectionKey(dialog, "1");
  assert.equal(dialog.open, true);
  assert.equal(confirmations, 1);
  answers = [true, false];
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(confirmations, 3);
  assert.deepEqual(f.commands, []);
  selectionKey(dialog, "@");
  selectionKey(dialog, "A");
  assert.equal(confirmations, 3, "read-only uppercase inspection needs no action confirmation");
  answers = [true, true];
  selectionKey(dialog, "Enter");
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "potion" }]);
  assert.equal(confirmations, 5);
  assert.equal(dialog.open, false);
});

test("floor single selection shares inscription guards and revalidates after confirmation", t => {
  const f = createInventoryFixture(t), chosen = [];
  f.state.status.player.position = { x: 0, y: 0 };
  f.state.status.items = [item("floor", { position: { x: 0, y: 0 }, inscription: "!*" })];
  let accept = false;
  f.document.defaultView = { confirm: () => accept };
  const open = () => {
    f.panel.selectItemTarget(undefined, async id => { chosen.push(id); }, undefined, undefined, "cast");
    return f.document.body.children[0];
  };
  let dialog = open(); selectionKey(dialog, "-");
  assert.equal(dialog.open, true);
  assert.deepEqual(chosen, []);
  accept = true; selectionKey(dialog, "-");
  assert.deepEqual(chosen, ["floor"]);
  dialog = open();
  f.document.defaultView.confirm = () => { f.state.status = { ...f.state.status }; return true; };
  selectionKey(dialog, "a");
  assert.equal(chosen.length, 1, "confirmation cannot authorize a replaced snapshot");
});

test("recharge selectors inherit activation inscriptions through source and target steps", t => {
  const f = createInventoryFixture(t);
  f.panel.render([
    item("cloak", { usable: true, requiresRechargeTargets: true, activation: {} }),
    item("donor", { canSupplyRecharge: true, inscription: "@A3 !A" }),
    item("target", { canReceiveRecharge: true, inscription: "@A4 !A" }),
  ], []);
  let confirmations = 0;
  f.document.defaultView = { confirm: () => { confirmations++; return true; } };
  f.state.selectedInventoryIds.add("cloak");
  f.dom.inventoryUse.dispatchEvent(new Event("click"));
  selectionKey(f.document.body.children[0], "3");
  assert.deepEqual(f.commands, []);
  selectionKey(f.document.body.children[0], "4");
  assert.equal(confirmations, 2);
  assert.deepEqual(f.commands, [{ type: "use-item-for-recharge", itemId: "cloak", sourceItemId: "donor", targetItemId: "target" }]);
});

test("secondary item choices retain the invoked command when an item has both category and activation", t => {
  const f = createInventoryFixture(t);
  for (const [command, label] of [["wand", "1"], ["activate", "2"]]) {
    f.panel.render([
      item("source", { usable: true, useCategory: "wand", activation: {}, useTargetSpec: { modes: ["item"] } }),
      item("target", { inscription: "@a1 @A2" }),
    ], []);
    f.panel.openCommand(command);
    selectionKey(f.document.body.children[0], "a");
    const dialog = f.document.body.children[0];
    assert.equal(dialog.children[0].children[1].children[1].children[0].textContent, `${label}) target`);
    selectionKey(dialog, label);
    assert.deepEqual(f.commands.at(-1), { type: "use-item", itemId: "source", target: { type: "item", itemId: "target" } });
  }
  assert.equal(f.commands.length, 2);
});

test("direct actions share inscription guards without prompting twice after shortcut selection", t => {
  const f = createInventoryFixture(t);
  let accepted = false, prompts = 0;
  f.document.defaultView = { confirm: () => { prompts++; return accepted; } };
  const potion = item("potion", { usable: true, useCategory: "potion", inscription: "!q" });
  f.panel.render([potion], []); f.state.selectedInventoryIds.add("potion");
  f.dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.deepEqual(f.commands, []);
  accepted = true;
  f.dom.inventoryUse.dispatchEvent(new Event("click"));
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "potion" }]);
  f.panel.openCommand("potion");
  selectionKey(f.document.body.children[0], "a");
  assert.equal(prompts, 3, "a confirmed selector does not ask again at the action method");
  f.panel.render([item("first", { inscription: "!d" }), item("second", { inscription: "!d" })], []);
  f.state.selectedInventoryIds.add("first"); f.state.selectedInventoryIds.add("second");
  let answers = [true, false];
  f.document.defaultView.confirm = () => answers.shift();
  f.dom.inventoryDrop.dispatchEvent(new Event("click"));
  assert.equal(f.commands.length, 2, "rejecting one item cancels the entire batch");
  answers = [true, true]; f.dom.inventoryDrop.dispatchEvent(new Event("click"));
  assert.deepEqual(f.commands.at(-1), { type: "drop", itemIds: ["first", "second"] });
});

test("counted quantities retain one inscription confirmation and reset invalidates all pending input", t => {
  const f = createInventoryFixture(t);
  let prompts = 0;
  f.document.defaultView = { confirm: () => { prompts++; return true; } };
  f.panel.render([item("stack", { quantity: 5, inscription: "!d" })], []);
  f.panel.openCommand("drop", 3); selectionKey(f.document.body.children[0], "a");
  assert.equal(prompts, 1);
  assert.deepEqual(f.commands, [{ type: "drop-quantity", itemId: "stack", quantity: 3 }]);
  f.panel.openCommand("drop"); selectionKey(f.document.body.children[0], "a");
  assert.equal(f.dom.inventoryActionDialog.open, true);
  f.panel.reset();
  f.dom.inventoryActionForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(f.commands.length, 1);
  f.state.bodySlots = [{ id: "hand", slotType: "weapon" }, { id: "tool", slotType: "tool" }];
  f.panel.render([item("pick", { equipmentSlot: "tool" })], []);
  f.panel.openCommand("equip"); selectionKey(f.document.body.children[0], "a");
  const slotDialog = f.document.body.children[0], slotForm = slotDialog.children[0];
  slotForm.children[1].children[1].value = "hand";
  f.panel.reset(); slotForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(slotDialog.open, false);
  assert.equal(f.commands.length, 1);
  f.panel.render([item("scroll", { usable: true, useCategory: "scroll", requiresTargetGlyph: true })], []);
  f.panel.openCommand("scroll"); selectionKey(f.document.body.children[0], "a");
  const glyphDialog = f.document.body.children[0], glyphForm = glyphDialog.children[0];
  glyphForm.children[1].children[1].value = "D";
  f.state.status = { ...f.state.status };
  f.panel.render(f.state.inventory, f.state.equipment);
  glyphForm.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.equal(glyphDialog.open, false);
  assert.equal(f.commands.length, 1);
});

test("ordered multi-target choice uses source labels, finish and existing paid cancellation", t => {
  const f = createInventoryFixture(t), selections = [];
  let cancelled = 0;
  f.panel.render([item("first"), item("second"), item("third")], []);
  const open = () => f.panel.selectItemTargets("source", async ids => { selections.push(ids); }, async () => { cancelled++; }, "cast", true);
  open();
  selectionKey(f.document.body.children[0], "b");
  assert.deepEqual(f.document.body.children[0].children[0].children[1].children[1].children.map(option => option.value), ["first", "third"]);
  selectionKey(f.document.body.children[0], "a");
  f.document.body.children[0].children[0].children[8].dispatchEvent(new Event("click"));
  assert.deepEqual(selections, [["second", "first"]]);
  assert.equal(cancelled, 0);
  open(); selectionKey(f.document.body.children[0], "a"); selectionKey(f.document.body.children[0], "Escape");
  assert.equal(cancelled, 1);
  assert.equal(selections.length, 1);
  const close = open(), old = f.document.body.children[0];
  close(); selectionKey(old, "a");
  assert.equal(cancelled, 1);
  assert.equal(selections.length, 1);
});

test("repeat confirmation resolves saved IDs and never transfers a removed item to its old label", t => {
  const f = createInventoryFixture(t);
  f.panel.render([item("saved", { useCategory: "potion", inscription: "!q" }), item("other")], []);
  let accept = false;
  f.document.defaultView = { confirm: () => accept };
  const command = { type: "use-item", itemId: "saved" };
  assert.equal(f.panel.confirmRepeatedCommand(command), false);
  accept = true; f.state.inventory.reverse();
  assert.equal(f.panel.confirmRepeatedCommand(command), true);
  f.state.inventory = f.state.inventory.filter(item => item.id !== "saved");
  assert.equal(f.panel.confirmRepeatedCommand(command), false);
  assert.deepEqual(command, { type: "use-item", itemId: "saved" });
});

test("refuel selects among compatible sources with F inscriptions and rejects incompatible fuel", t => {
  const f = createInventoryFixture(t);
  f.panel.render([
    item("oil", { fuel: { kind: "oil", current: 5 }, inscription: "@F1" }),
    item("spare", { fuel: { kind: "lantern", current: 8 }, inscription: "@F2 !F" }),
    item("torch", { fuel: { kind: "torch", current: 8 } }),
  ], [{ ...item("lamp", { fuel: { kind: "lantern", current: 0, maximum: 20 } }), slotId: "light" }]);
  let accept = false;
  f.document.defaultView = { confirm: () => accept };
  f.panel.openCommand("refuel"); selectionKey(f.document.body.children[0], "a");
  const dialog = f.document.body.children[0];
  assert.deepEqual(dialog.children[0].children[1].children[1].children.map(option => option.value), ["oil", "spare"]);
  selectionKey(dialog, "2"); assert.deepEqual(f.commands, []);
  accept = true; selectionKey(dialog, "2");
  assert.deepEqual(f.commands, [{ type: "refuel-light", targetItemId: "lamp", sourceItemId: "spare" }]);
});

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

test("equipment identification only labels unresolved knowledge", () => {
  assert.equal(
    itemIdentificationMessageKey("unexamined"),
    "item-identification-unexamined",
  );
  assert.equal(
    itemIdentificationMessageKey("appraised"),
    "item-identification-appraised",
  );
  assert.equal(itemIdentificationMessageKey("identified"), undefined);
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
