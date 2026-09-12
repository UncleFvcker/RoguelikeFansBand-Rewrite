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

function item(id, extra = {}) {
  return {
    id, kindId: `item.${id}`, displayNameKey: id,
    usable: false, equipmentSlot: null, quantity: 1, weightTenthsPound: 10,
    identification: "unexamined",
    modifiers: { attack: 0, defense: 0, maxHp: 0, speed: 0 },
    ...extra,
  };
}

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
  state.contentGlyphs.set(items[0].kindId, "/");
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
    assert.deepEqual(select.children.map(option => option.value), ["ring", "floor-ring"]);
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
    get ownerDocument() { return document; }
    setAttribute(key, value) { this.attributes.set(key, value); }
    getAttribute(key) { return this.attributes.get(key); }
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
        const matches = selector === "button" ? child.tag === "button" : child.tag === "input" && child.type === "checkbox";
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
  state.status = {
    items: [],
    player: { carriedWeightTenthsPound: 10, carryCapacityTenthsPound: 100, inventoryUsedSlots: 2, inventorySlotCapacity: 26 },
  };
  const commands = [];
  const targets = [];
  const panel = new InventoryPanel({
    dom, state,
    localization: { format: (key, args) => `${key} ${JSON.stringify(args)}` },
    formatter: { visibleItemName: (key, _kind, name) => name ? `${key} ${name}` : key, equipmentSlotName: (slot) => slot, itemPropertyName: (key) => key },
    dispatch: async (command) => { commands.push(command); },
    startTargeting: (...args) => targets.push(args), updateCampaignAction: () => {}, announce: () => {},
  });
  panel.install();
  t.after(() => panel.dispose());
  return { panel, dom, state, radio, commands, targets, document };
}

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
