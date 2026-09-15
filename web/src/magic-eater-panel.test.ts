// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in test runner; executed with the step-7 checks.
import assert from "node:assert/strict";
import test from "node:test";
import { AppState } from "./app-state.ts";
import { MagicEaterPanel } from "./magic-eater-panel.ts";
import { GameSession } from "./game-session.ts";

function fixture() {
  class Element extends EventTarget {
    dataset = {}; children = []; value = ""; open = false; disabled = false; hidden = false;
    classList = { add() {} };
    constructor(tag = "div") { super(); this.tagName = tag.toUpperCase(); this.ownerDocument = document; }
    append(...nodes) { nodes.forEach(node => { node.parent = this; this.children.push(node); }); }
    replaceChildren(...nodes) { this.children = []; this.append(...nodes); }
    contains(node) { return this === node || this.children.some(child => child.contains(node)); }
    matches(selector) { return selector === ":disabled" && this.disabled; }
    closest(tag) { return this.tagName.toLowerCase() === tag ? this : this.parent?.closest(tag); }
    querySelectorAll(selector) { return this.children.flatMap(child => {
      const matches = selector === "[data-label]" ? child.dataset.label !== undefined : selector === "[data-focus]" ? child.dataset.focus !== undefined : selector.split(", ").includes(child.tagName.toLowerCase());
      return [...(matches ? [child] : []), ...child.querySelectorAll(selector)];
    }); }
    focus() { document.activeElement = this; }
    showModal() { this.open = true; }
    close() { this.open = false; this.dispatchEvent(new Event("close")); }
    remove() { if (this.parent) this.parent.children = this.parent.children.filter(child => child !== this); }
    click() { if (!this.disabled) this.dispatchEvent(new Event("click")); }
  }
  const elements = new Map();
  const document = { createElement: tag => new Element(tag), getElementById(id) {
    if (!elements.has(id)) elements.set(id, new Element(id.endsWith("dialog") ? "dialog" : id.endsWith("category") ? "select" : "button"));
    return elements.get(id);
  } };
  document.body = new Element("body");
  const state = new AppState(); state.mode = "playing";
  const device = (id, modes = ["self"]) => ({ id, kindId: "device", displayNameKey: "device-name", usable: true, charges: { current: 14, maximum: 28 }, activation: { nameKey: "effect", cost: 7 }, useTargetSpec: { modes, range: 8, requiresLineOfEffect: true } });
  const slots = ["wand", "staff", "rod"].flatMap(category => Array.from({ length: 10 }, (_, slot) => ({ category, slot, item: slot === 0 ? device(`${category}.body`) : null,
    useLabel: slot === 0 ? "q" : String.fromCharCode(97 + slot), deviceLabel: slot === 0 ? "8" : String.fromCharCode(97 + slot), failurePerMille: 137, energyCost: 80, recoveryPerMille: 15, allowsMultipleTargets: false })));
  state.status = { mapScale: "local", player: { position: { x: 1, y: 1 }, magicEater: { slots, deviceCommands: ["wand", "staff", "rod"].map(category => ({ category, items: [] })), pendingAbsorption: null },
    abilities: [{ id: "absorb-power", canCast: true, effects: [{ type: "magic-eater-absorb" }], itemTargets: [{ itemId: "source" }] }] }, items: [] };
  const commands = [], targets = [], inspected = [], sources = [], itemChoices = [], confirmations = []; let exports = 0, imports = 0;
  const panel = new MagicEaterPanel({ document, state, localization: { format: (key, args) => key + (args ? JSON.stringify(args) : "") },
    dispatch: async command => commands.push(command), visibleItemName: key => key, inspectItem: id => inspected.push(id),
    selectItemTarget: (ids, select, cancel, command) => sources.push({ ids, select, cancel, command }), startTargeting: (spec, intent) => targets.push({ spec, intent }),
    confirmItemChoice: (id, command) => { confirmations.push({ id, command }); return true; },
    selectItemTargets: (excluded, select, cancel, command, multiple) => {
      const choice = { excluded, select, cancel, command, multiple, closed: false };
      itemChoices.push(choice); return () => { choice.closed = true; };
    },
    beforeOpen() {}, saveGame: async () => { exports++; }, loadGame: () => { imports++; },
  });
  const element = suffix => document.getElementById(`magic-eater-${suffix}`);
  element("category").value = "wand";
  const row = slot => element("slots").children.find(row => row.dataset.slot === String(slot));
  const action = (slot, name) => row(slot).children.at(-1).children.find(button => button.dataset.action === name);
  const key = value => { const event = new Event("keydown", { cancelable: true }); Object.assign(event, { key: value }); element("dialog").dispatchEvent(event); };
  return { state, panel, commands, targets, inspected, sources, itemChoices, confirmations, element, action, row, key, device, exports: () => exports, imports: () => imports };
}

test("device menu displays core values, uses projected labels/identities, and respects ordinary-device precedence", () => {
  const f = fixture(); f.panel.open();
  for (const extra of [{ isComposing: true }, { repeat: true }, { ctrlKey: true }, { metaKey: true }, { altKey: true }]) {
    const event = Object.assign(new Event("keydown", { cancelable: true }), { key: "q", ...extra });
    f.element("dialog").dispatchEvent(event);
    assert.equal(event.defaultPrevented, false);
  }
  assert.equal(f.commands.length, 0, "modified or composing labels never use a device");
  assert.equal(f.element("slots").children.length, 10);
  assert.match(f.row(0).children[1].textContent, /13\.7/);
  assert.match(f.row(0).children[1].textContent, /80/);
  f.key("q"); assert.deepEqual(f.commands.pop(), { type: "use-absorbed-device", itemId: "wand.body", targets: [{ type: "self" }] });
  f.action(0, "inspect").click(); assert.deepEqual(f.inspected, ["wand.body"]);
  const ordinary = f.device("wand.pack"); ordinary.usable = false;
  f.state.status.player.magicEater.deviceCommands[0].items = [ordinary];
  f.panel.openDeviceCommand("a");
  assert.equal(f.element("slots").children[0].dataset.itemId, "wand.pack");
  assert.equal(f.element("slots").children[0].children.at(-1).children[0].disabled, true, "empty or unavailable ordinary devices still prevent automatic body fallback");
  f.state.status.player.magicEater.deviceCommands[0].items = [];
  f.panel.openDeviceCommand("a"); f.key("8");
  assert.equal(f.commands.pop().itemId, "wand.body");
  f.state.status.player.magicEater.slots[0].item.useTargetSpec.modes = ["direction"];
  f.action(0, "use").click(); assert.equal(f.targets.at(-1).intent.type, "absorbed-device");
  assert.equal(f.targets.at(-1).intent.itemId, "wand.body");
});

test("absorption reopens saved slot/replace prompts; busy rejects duplicate answers and cancel sends only the resolver", async () => {
  const f = fixture(); f.panel.open(); f.element("absorb").click();
  assert.deepEqual(f.sources[0].ids, ["source"]); await f.sources[0].select("source");
  assert.deepEqual(f.commands.pop(), { type: "cast-ability", abilityId: "absorb-power", target: { type: "item", itemId: "source" } });
  f.panel.reset();
  f.state.status.player.magicEater.pendingAbsorption = { category: "wand", sourceItemId: "source", replacement: null };
  f.panel.render(); assert.equal(f.element("dialog").open, true); assert.equal(f.state.commandBlocked, true);
  f.action(0, "select").click(); assert.deepEqual(f.commands.pop(), { type: "select-magic-absorption-slot", slot: 0 });
  f.state.status.player.magicEater.pendingAbsorption.replacement = { itemId: "wand.body", slot: 0 }; f.panel.render();
  assert.equal(f.element("replacement").hidden, false);
  f.element("inherit").checked = true; f.element("save").click(); f.element("load").click();
  assert.equal(f.exports(), 1); assert.equal(f.imports(), 1);
  f.state.busy = true; f.panel.render(); f.element("confirm").click();
  f.element("dialog").dispatchEvent(new Event("cancel", { cancelable: true })); assert.equal(f.commands.length, 0);
  f.state.busy = false; f.panel.render(); f.element("confirm").click();
  assert.deepEqual(f.commands.pop(), { type: "resolve-magic-absorption", confirm: true, inheritInscription: true });
  f.element("dialog").dispatchEvent(new Event("cancel", { cancelable: true }));
  assert.deepEqual(f.commands.pop(), { type: "resolve-magic-absorption", confirm: false, inheritInscription: false });
});

test("keyboard exchange accepts an empty slot; subsequent labels and inscriptions resolve the moved instance", () => {
  const f = fixture(); f.panel.open(); f.key("X"); f.key("a"); f.key("b");
  assert.deepEqual(f.commands.pop(), { type: "swap-absorbed-devices", category: "wand", firstSlot: 0, secondSlot: 1 });
  const slots = f.state.status.player.magicEater.slots; slots[1].item = slots[0].item; slots[0].item = null;
  slots[1].useLabel = "q"; slots[0].useLabel = "a"; f.panel.render(); f.key("q");
  assert.equal(f.commands.pop().itemId, "wand.body");
  f.key("Z"); f.key("b");
  const dialog = f.element("dialog").ownerDocument.body.children.at(-1), form = dialog.children[0];
  form.children[0].children[0].value = "@mR @5";
  form.dispatchEvent(new Event("submit", { cancelable: true }));
  assert.deepEqual(f.commands.pop(), { type: "inscribe-item", itemId: "wand.body", inscription: "@mR @5" });
});

test("session blocks movement, ordinary use and travel settings during saved absorption, but permits slot/confirmation and locale", async () => {
  const f = fixture(); f.state.status.player.magicEater.pendingAbsorption = { category: "wand", sourceItemId: "source" };
  const calls = []; let release;
  const session = new GameSession({ state: f.state, execute: command => { calls.push(command); return new Promise(resolve => { release = resolve; }); }, applyUpdate() {}, refreshBusyControls() {}, showError: error => { throw error; } });
  for (const type of ["move", "use-item", "configure-travel"]) await session.dispatch({ type });
  assert.equal(calls.length, 0);
  const command = { type: "resolve-magic-absorption", confirm: false, inheritInscription: false };
  const first = session.dispatch(command); await session.dispatch(command); assert.equal(calls.length, 1); release({}); await first;
  const locale = session.dispatch({ type: "set-interface-locale", locale: "en-US" }); release({}); await locale;
  assert.equal(calls.at(-1).type, "set-interface-locale");
});

test("ordinary device commands use canonical item selection and do not reconfirm an accepted source", async () => {
  const f = fixture();
  const ordinary = f.device("wand.pack");
  f.state.status.player.magicEater.deviceCommands[0].items = [ordinary, { ...f.device("empty"), usable: false }];
  f.panel.openDeviceCommand("a");
  const choice = f.sources.at(-1);
  assert.equal(choice.command, "wand");
  assert.deepEqual(choice.ids, ["wand.pack"]);
  await choice.select("wand.pack");
  assert.equal(f.confirmations.length, 0, "the shared source chooser already confirmed this item");
  assert.deepEqual(f.commands, [{ type: "use-item", itemId: "wand.pack", target: { type: "self" } }]);
});

test("multiple identification uses the shared ordered chooser and reset closes it silently", async () => {
  const f = fixture(), staff = f.state.status.player.magicEater.slots[10];
  staff.item.useTargetSpec.modes = ["item"]; staff.allowsMultipleTargets = true;
  f.state.inventory = ["first", "second"].map(id => ({ id, kindId: id, displayNameKey: id }));
  f.panel.open(); f.key("S"); f.action(0, "use").click();
  let choice = f.itemChoices.at(-1);
  assert.equal(choice.multiple, true);
  assert.equal(choice.command, "cast");
  assert.equal(choice.excluded, "staff.body");
  await choice.select(["second", "first"]);
  assert.deepEqual(f.commands.pop(), { type: "use-absorbed-device", itemId: "staff.body", targets: [{ type: "item", itemId: "second" }, { type: "item", itemId: "first" }] });
  f.action(0, "use").click(); choice = f.itemChoices.at(-1); f.panel.reset();
  assert.equal(choice.closed, true); assert.equal(f.commands.length, 0);
  f.panel.open(); f.action(0, "use").click(); await f.itemChoices.at(-1).cancel();
  assert.deepEqual(f.commands.pop(), { type: "use-absorbed-device", itemId: "staff.body", targets: [] });
});
