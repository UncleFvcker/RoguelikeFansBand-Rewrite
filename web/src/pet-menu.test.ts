// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Minimal DOM fixture for Node's built-in test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { AppState } from "./app-state.ts";
import { PetMenu } from "./pet-menu.ts";

class Element extends EventTarget {
  children = []; attributes = {}; value = ""; open = false;
  constructor(tag) { super(); this.tag = tag; }
  append(...children) {
    for (const child of children) { child.parent = this; this.children.push(child); }
    if (this.tag === "select" && !this.value) this.value = children[0]?.value ?? "";
  }
  setAttribute(key, value) { this.attributes[key] = value; }
  showModal() { this.open = true; }
  close() { this.open = false; this.dispatchEvent(new Event("close")); }
  remove() { this.parent.children = this.parent.children.filter(child => child !== this); }
  focus() {}
  click() { if (!this.disabled) this.dispatchEvent(new Event("click")); }
}

const descendants = node => node.children.flatMap(child => [child, ...descendants(child)]);

function fixture() {
  const state = new AppState(); state.mode = "playing";
  state.status = { player: { summonCommand: { mode: "follow", targetActorId: "enemy" },
    pets: [{ actorId: "pet-a", nameKey: "horse", riding: true, canName: true }, { actorId: "pet-b", nameKey: "horse", canName: true }], ridingActorId: "pet-a" },
    entities: [{ id: "enemy", kindId: "golem", position: { x: 3, y: 5 }, faction: "hostile" }, { id: "friend", kindId: "horse", faction: "friendly" }] };
  const body = new Element("body");
  const document = { body, createElement: tag => new Element(tag), querySelector: () => body.children.find(child => child.open) };
  const commands = [], confirmations = [], rides = [];
  const control = { approve: true };
  const menu = new PetMenu({ document, state, localization: { format: (key, args) => key + (args ? JSON.stringify(args) : "") },
    contentName: id => id, dispatch: async command => { commands.push(command); },
    confirm: message => { confirmations.push(message); return control.approve; },
    startRiding: () => { assert.equal(body.children.length, 0); rides.push(true); } });
  const dialog = () => body.children[0];
  const button = key => descendants(dialog()).find(child => child.tag === "button" && child.textContent === key);
  const selects = () => descendants(dialog()).filter(child => child.tag === "label").map(label => label.children[1]).filter(input => input.tag === "select");
  return { state, body, menu, commands, confirmations, control, rides, dialog, button, selects };
}

test("names follow the selected pet, cancel does not dispatch, and dismissal confirms the name", () => {
  const h = fixture();
  h.state.status.player.pets[0].customName = "追风";
  h.state.status.player.pets[1].customName = "踏雪";
  const input = () => descendants(h.dialog()).find(child => child.tag === "label" && child.children[1].tag === "input").children[1];
  h.menu.open();
  assert.equal(input().value, "追风");
  h.selects()[1].value = "pet-b";
  h.selects()[1].dispatchEvent(new Event("change"));
  assert.equal(input().value, "踏雪");
  input().value = "新名字🐎";
  h.menu.close(); assert.deepEqual(h.commands, []);
  h.menu.open(); input().value = "新名字🐎";
  h.button("pet-menu-name-apply").click();
  assert.deepEqual(h.commands[0], { type: "set-pet-name", actorId: "pet-a", name: "新名字🐎" });
  h.menu.open(); h.button("pet-menu-name-clear").click();
  assert.deepEqual(h.commands[1], { type: "set-pet-name", actorId: "pet-a", name: null });
  h.menu.open(); h.button("pet-menu-dismiss-one").click();
  assert.match(h.confirmations[0], /追风/);
  h.state.status.player.pets[0].canName = false;
  h.menu.open(); h.button("pet-menu-name-apply").click(); h.button("pet-menu-name-clear").click();
  assert.equal(h.commands.length, 3);
  assert.equal(input().disabled, true);
});

test("map and list highlights toggle independently with source defaults", () => {
  const h = fixture(); h.menu.open();
  h.button("pet-menu-highlight-map-off").click();
  h.menu.open(); h.button("pet-menu-highlight-lists-on").click();
  assert.deepEqual(h.commands, [
    { type: "set-pet-option", option: "highlight-map", enabled: true },
    { type: "set-pet-option", option: "highlight-lists", enabled: false },
  ]);
});

test("mounted two-hand choice reports actual control, cancels freely and hides when unmounted", () => {
  const h = fixture(); h.menu.open();
  assert.equal(h.button("pet-menu-two-hands").attributes["aria-pressed"], "false");
  h.menu.close(); assert.deepEqual(h.commands, []);
  h.menu.open(); h.button("pet-menu-two-hands").click();
  assert.deepEqual(h.commands[0], { type: "set-pet-option", option: "riding-two-hands", enabled: true });
  h.state.status.player.summonCommand.ridingTwoHands = true;
  h.state.status.player.ridingWithoutReins = true;
  h.menu.open();
  assert.ok(descendants(h.dialog()).some(child => child.textContent === "pet-menu-without-reins"));
  h.button("pet-menu-reins").click();
  assert.deepEqual(h.commands[1], { type: "set-pet-option", option: "riding-two-hands", enabled: false });
  h.state.status.player.ridingActorId = null;
  h.menu.open();
  assert.equal(h.button("pet-menu-reins"), undefined);
  assert.equal(h.button("pet-menu-two-hands"), undefined);
});

test("pet menu chooses visible hostiles by ID and clears the saved target", () => {
  const h = fixture(); h.menu.open();
  assert.deepEqual(h.selects()[0].children.map(option => option.value), ["enemy"]);
  assert.equal(h.selects()[0].children[0].textContent, "golem (3, 5)");
  h.button("pet-menu-set-target").click();
  assert.deepEqual(h.commands, [{ type: "set-pet-target", actorId: "enemy" }]);
  assert.equal(h.body.children.length, 0);
  h.menu.open(); h.button("pet-menu-clear-target").click();
  assert.deepEqual(h.commands[1], { type: "set-pet-target", actorId: null });
});

test("pet menu mode selection closes once, with current mode exposed accessibly", () => {
  const h = fixture(); h.menu.open();
  assert.equal(h.button("action-summon-command-follow").attributes["aria-pressed"], "true");
  h.button("action-summon-command-give-space").click();
  assert.deepEqual(h.commands, [{ type: "set-summon-command", mode: "give-space" }]);
  assert.equal(h.body.children.length, 0);
});

test("pet door and pickup permissions default off, toggle explicitly and preserve cancellation", () => {
  const h = fixture(); h.menu.open();
  assert.equal(h.button("pet-menu-open-doors-off").attributes["aria-pressed"], "false");
  h.button("pet-menu-open-doors-off").click();
  assert.deepEqual(h.commands, [{ type: "set-pet-option", option: "open-doors", enabled: true }]);
  h.state.status.player.summonCommand.pickupItems = true;
  h.menu.open();
  assert.equal(h.button("pet-menu-pickup-items-on").attributes["aria-pressed"], "true");
  h.button("pet-menu-pickup-items-on").click();
  assert.deepEqual(h.commands[1], { type: "set-pet-option", option: "pickup-items", enabled: false });
  h.menu.open(); h.menu.close(); assert.equal(h.commands.length, 2);
});

test("prevent breeding defaults off and toggles without conflating other pet options", () => {
  const h = fixture(); h.menu.open();
  assert.equal(h.button("pet-menu-no-breeding-off").attributes["aria-pressed"], "false");
  h.button("pet-menu-no-breeding-off").click();
  assert.deepEqual(h.commands, [{ type: "set-pet-option", option: "no-breeding", enabled: true }]);
  h.state.status.player.summonCommand.noBreeding = true;
  h.menu.open(); h.button("pet-menu-no-breeding-on").click();
  assert.deepEqual(h.commands[1], { type: "set-pet-option", option: "no-breeding", enabled: false });
});

test("spell controls expose source defaults and dispatch independent toggles", () => {
  const h = fixture();
  for (const [option, field, defaultValue] of [
    ["attack-spells", "attackSpells", true],
    ["summon-spells", "summonSpells", true],
    ["teleport", "teleport", true],
    ["allow-player-damage", "allowPlayerDamage", false],
  ]) {
    h.menu.open();
    const button = h.button(`pet-menu-${option}-${defaultValue ? "on" : "off"}`);
    assert.equal(button.attributes["aria-pressed"], String(defaultValue));
    button.click();
    assert.deepEqual(h.commands.at(-1), { type: "set-pet-option", option, enabled: !defaultValue });
    h.state.status.player.summonCommand[field] = !defaultValue;
    h.menu.open();
    h.button(`pet-menu-${option}-${defaultValue ? "off" : "on"}`).click();
    assert.deepEqual(h.commands.at(-1), { type: "set-pet-option", option, enabled: defaultValue });
  }
  h.menu.open(); h.menu.close();
  assert.equal(h.commands.length, 8);
});

test("dismissal confirms the selected pet ID, supports cancellation and all pets", () => {
  const h = fixture(); h.menu.open(); h.selects()[1].value = "pet-b";
  h.control.approve = false; h.button("pet-menu-dismiss-one").click();
  assert.deepEqual(h.commands, []); assert.equal(h.dialog().open, true);
  h.control.approve = true; h.button("pet-menu-dismiss-one").click();
  assert.deepEqual(h.commands, [{ type: "dismiss-pet", actorId: "pet-b" }]);
  h.menu.open(); h.control.approve = false; h.button("action-dismiss-pets").click();
  assert.equal(h.commands.length, 1);
  h.control.approve = true; h.button("action-dismiss-pets").click();
  assert.deepEqual(h.commands[1], { type: "dismiss-pets" });
  assert.equal(h.confirmations.length, 4);
});

test("riding hands control to the direction prompt; closing and session reset issue no command", () => {
  const h = fixture(); h.menu.open(); h.button("pet-menu-dismount").click();
  assert.equal(h.rides.length, 1); assert.deepEqual(h.commands, []);
  h.menu.open(); h.button("action-dialog-close").click();
  h.menu.open(); h.menu.close(); assert.equal(h.body.children.length, 0);
  assert.deepEqual(h.commands, []);
});

test("empty lists disable unavailable actions and busy, world, modal contexts do not open", () => {
  const h = fixture(); h.state.status.player.pets = []; h.state.status.entities = [];
  h.state.status.player.summonCommand = undefined; h.menu.open();
  for (const key of ["pet-menu-set-target", "pet-menu-clear-target", "pet-menu-dismiss-one", "action-dismiss-pets"])
    assert.equal(h.button(key).disabled, true);
  h.menu.open(); assert.equal(h.body.children.length, 1); h.menu.close();
  h.state.busy = true; h.menu.open(); assert.equal(h.body.children.length, 0);
  h.state.busy = false; h.state.status.mapScale = "world"; h.menu.open(); assert.equal(h.body.children.length, 0);
  h.state.status.mapScale = "local"; h.state.playerDead = true; h.menu.open(); assert.equal(h.body.children.length, 0);
});
