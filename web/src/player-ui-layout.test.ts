// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { PlayerUiLayout, playerPageForShortcut } from "./player-ui-layout.ts";

test("Maia choice opens the character Other page and can be reopened after closing", (t) => {
  const { layout, element } = createLayoutFixture(t);
  layout.showMaiaChoice();
  assert.equal(element("player-page-dialog").open, true);
  assert.equal(element("character-tab-other").getAttribute("aria-selected"), "true");
  layout.closePage();
  layout.showMaiaChoice();
  assert.equal(element("character-tab-other").getAttribute("aria-selected"), "true");
});

test("player pages use conventional shortcuts without consuming movement keys", () => {
  assert.equal(playerPageForShortcut("i"), "inventory");
  assert.equal(playerPageForShortcut("I"), "inventory");
  assert.equal(playerPageForShortcut("m"), "ability");
  assert.equal(playerPageForShortcut("w"), undefined);
});

test("navigation switches original panels without closing or reopening the dialog", (t) => {
  const { layout, element, document } = createLayoutFixture(t);
  const dialog = element("player-page-dialog");
  const host = element("player-page-host");
  const parking = element("player-page-parking");
  const support = element("support-panel-host");
  assert.equal(element("task-log-panel").parentNode, parking);
  assert.equal(element("task-log-entry").parentNode, support);
  for (const id of ["campaign-panel", "dungeon-info-panel", "summon-command-panel", "native-save-panel"]) {
    assert.equal(element(id).parentNode, support);
  }

  element("player-ui-inventory-open").dispatchEvent(new Event("click"));
  const detail = element("inventory-detail-dialog");
  detail.open = true;
  layout.open("ability");
  assert.equal(detail.open, false);
  layout.open("inventory");
  for (const [page, panelId] of [
    ["inventory", "inventory-panel"],
    ["ability", "ability-panel"],
    ["character", "character-details-panel"],
    ["tasks", "task-log-panel"],
  ]) {
    const tab = element(`player-page-tab-${page}`);
    tab.dispatchEvent(new Event("click"));
    assert.deepEqual(host.children, [element(panelId)]);
    assert.equal(dialog.dataset.page, page);
    assert.equal(dialog.open, true);
    assert.equal(dialog.showCount, 1);
    assert.equal(dialog.closeCount, 0);
    assert.equal(document.activeElement, tab);
    assert.equal(host.getAttribute("aria-labelledby"), tab.id);
    for (const other of ["ability", "character", "inventory", "tasks"]) {
      const otherTab = element(`player-page-tab-${other}`);
      assert.equal(otherTab.getAttribute("aria-selected"), String(other === page));
      assert.equal(otherTab.tabIndex, other === page ? 0 : -1);
    }
  }

  assert.equal(press(element("player-page-tab-tasks"), "ArrowRight").defaultPrevented, true);
  assert.equal(dialog.dataset.page, "ability");
  press(element("player-page-tab-ability"), "ArrowLeft");
  assert.equal(dialog.dataset.page, "tasks");
  press(element("player-page-tab-tasks"), "Home");
  assert.equal(dialog.dataset.page, "ability");
  press(element("player-page-tab-ability"), "End");
  assert.equal(dialog.dataset.page, "tasks");

  element("player-page-close").dispatchEvent(new Event("click"));
  assert.equal(dialog.open, false);
  assert.equal(element("task-log-panel").parentNode, parking);
  element("player-ui-tasks-open").dispatchEvent(new Event("click"));
  // Native close notifications are asynchronous; a stale one must not clear a reopened page.
  dialog.dispatchEvent(new Event("close"));
  assert.deepEqual(host.children, [element("task-log-panel")]);
  assert.equal(dialog.showCount, 2);
  layout.closePage();
});

test("I and M keep their shortcuts while other dialogs and editing retain keyboard focus", (t) => {
  const { layout, element, window } = createLayoutFixture(t);
  const dialog = element("player-page-dialog");
  press(window, "I");
  assert.equal(dialog.dataset.page, "inventory");
  press(window, "m");
  assert.equal(dialog.dataset.page, "ability");
  assert.equal(dialog.showCount, 1);
  assert.equal(dialog.closeCount, 0);
  press(window, "m");
  assert.equal(dialog.open, false);
  press(window, "i");
  const childDialog = element("object-list-dialog");
  childDialog.open = true;
  assert.equal(press(window, "m").defaultPrevented, false);
  assert.equal(dialog.dataset.page, "inventory");
  childDialog.open = false;
  const editingEvent = new Event("keydown", { cancelable: true });
  Object.defineProperty(editingEvent, "target", { value: new HTMLInputElement() });
  Object.assign(editingEvent, { key: "m" });
  window.dispatchEvent(editingEvent);
  assert.equal(editingEvent.defaultPrevented, false);
  assert.equal(dialog.dataset.page, "inventory");

  // An Escape/native close returns the panel without destroying its DOM.
  dialog.close();
  dialog.dispatchEvent(new Event("close"));
  assert.equal(element("inventory-panel").parentNode, element("player-page-parking"));
  layout.dispose();
  press(window, "i");
  element("player-page-tab-tasks").dispatchEvent(new Event("click"));
  assert.equal(dialog.open, false);
});

function press(target, key) {
  const event = Object.assign(new Event("keydown", { cancelable: true }), { key });
  target.dispatchEvent(event);
  return event;
}

test("character tabs switch only their panes and support keyboard navigation", (t) => {
  const { layout, element, document } = createLayoutFixture(t);
  layout.open("character");
  const dialog = element("player-page-dialog");
  for (const selected of ["details", "proficiencies", "other", "overview"]) {
    const tab = element(`character-tab-${selected}`);
    tab.dispatchEvent(new Event("click"));
    tab.dispatchEvent(new Event("click"));
    assert.equal(document.activeElement, tab);
    for (const page of ["overview", "details", "proficiencies", "other"]) {
      assert.equal(element(`character-page-${page}`).hidden, page !== selected);
      assert.equal(element(`character-tab-${page}`).getAttribute("aria-selected"), String(page === selected));
      assert.equal(element(`character-tab-${page}`).tabIndex, page === selected ? 0 : -1);
    }
    assert.equal(dialog.dataset.page, "character");
    assert.equal(dialog.showCount, 1);
    assert.equal(dialog.closeCount, 0);
  }
  press(element("character-tab-overview"), "ArrowLeft");
  assert.equal(element("character-page-other").hidden, false);
  press(element("character-tab-other"), "ArrowRight");
  assert.equal(element("character-page-overview").hidden, false);
  press(element("character-tab-overview"), "End");
  assert.equal(element("character-page-other").hidden, false);
  press(element("character-tab-other"), "Home");
  assert.equal(element("character-page-overview").hidden, false);
  layout.open("inventory");
  layout.open("character");
  assert.equal(element("character-page-overview").hidden, false);
  layout.dispose();
  element("character-tab-other").dispatchEvent(new Event("click"));
  assert.equal(element("character-page-overview").hidden, false);
});

test("attribute detail categories retain independent scroll positions and keyboard focus", (t) => {
  const { layout, element, document } = createLayoutFixture(t);
  const click = (id) => element(id).dispatchEvent(new Event("click"));
  layout.open("character");
  click("character-tab-details");
  for (const [index, category] of ["sources", "defenses", "offense"].entries()) {
    click(`character-detail-tab-${category}`);
    click(`character-detail-tab-${category}`);
    for (const other of ["sources", "defenses", "offense"]) {
      assert.equal(element(`character-detail-${other}`).hidden, other !== category);
      assert.equal(element(`character-detail-tab-${other}`).tabIndex, other === category ? 0 : -1);
    }
    element(`character-detail-${category}`).scrollTop = (index + 1) * 100;
  }
  press(element("character-detail-tab-offense"), "ArrowRight");
  assert.equal(document.activeElement, element("character-detail-tab-sources"));
  assert.equal(element("character-detail-sources").scrollTop, 100);
  press(element("character-detail-tab-sources"), "ArrowLeft");
  assert.equal(element("character-detail-offense").scrollTop, 300);
  press(element("character-detail-tab-offense"), "Home");
  press(element("character-detail-tab-sources"), "End");
  assert.equal(document.activeElement, element("character-detail-tab-offense"));
  click("character-tab-overview");
  element("character-detail-offense").scrollTop = 0; // Hidden DOM must not overwrite saved offsets.
  click("character-tab-other");
  click("character-tab-details");
  assert.equal(element("character-detail-offense").scrollTop, 300);
  layout.open("inventory");
  element("character-detail-offense").scrollTop = 0;
  layout.open("character");
  assert.equal(element("character-detail-offense").scrollTop, 300);
  assert.equal(element("player-page-dialog").showCount, 1);
  layout.closePage();
  element("character-detail-offense").scrollTop = 0;
  layout.open("character");
  assert.equal(element("character-detail-offense").scrollTop, 300);
  const dialog = element("player-page-dialog");
  dialog.dispatchEvent(new Event("cancel"));
  dialog.close();
  element("character-detail-offense").scrollTop = 0;
  dialog.dispatchEvent(new Event("close"));
  layout.open("character");
  assert.equal(element("character-detail-offense").scrollTop, 300);
  layout.dispose();
  click("character-detail-tab-sources");
  assert.equal(element("character-detail-tab-offense").getAttribute("aria-selected"), "true");
});

test("inventory child dialogs close before the page and inventory actions no longer close it eagerly", (t) => {
  const { layout, element, window } = createLayoutFixture(t);
  const dialog = element("player-page-dialog");
  layout.open("inventory");
  for (const id of ["inventory-detail-dialog", "inventory-more-dialog", "inventory-action-dialog"]) {
    const child = element(id);
    child.showModal();
    press(window, "i");
    assert.equal(dialog.open, true);
    child.close(); // Native Escape closes only the topmost modal.
    child.dispatchEvent(new Event("close"));
    assert.equal(dialog.open, true);
    assert.equal(element("inventory-panel").parentNode, element("player-page-host"));
    child.showModal();
    layout.open("ability");
    assert.equal(child.open, false);
    layout.open("inventory");
  }
  for (const id of ["inventory-use", "inventory-absorb"]) {
    const action = element(id);
    action.closest = () => action;
    const click = new Event("click");
    Object.defineProperty(click, "target", { value: action });
    dialog.dispatchEvent(click);
    assert.equal(dialog.open, true);
  }
  layout.closePage();
  assert.equal(dialog.open, false);
});

function createLayoutFixture(t) {
  // Only the DOM operations used by the controller; CSS sizing is not simulated here.
  class Element extends EventTarget {
    children = [];
    dataset = {};
    attributes = new Map();
    open = false;
    showCount = 0;
    closeCount = 0;
    scrollTop = 0;
    constructor(id) { super(); this.id = id; }
    append(...nodes) {
      for (const node of nodes) {
        if (node.parentNode) node.parentNode.children = node.parentNode.children.filter((child) => child !== node);
        node.parentNode = this;
        this.children.push(node);
      }
    }
    closest() { return null; }
    setAttribute(key, value) { this.attributes.set(key, value); }
    getAttribute(key) { return this.attributes.get(key); }
    focus() { document.activeElement = this; }
    showModal() { assert.equal(this.open, false); this.open = true; this.showCount++; }
    close() { this.open = false; this.closeCount++; }
  }
  for (const [name, constructor] of Object.entries({
    HTMLElement: Element,
    HTMLInputElement: class extends Element {},
    HTMLTextAreaElement: class extends Element {},
    HTMLSelectElement: class extends Element {},
  })) {
    const descriptor = Object.getOwnPropertyDescriptor(globalThis, name);
    Object.defineProperty(globalThis, name, { configurable: true, value: constructor });
    t.after(() => {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    });
  }
  const html = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const elements = new Map([...html.matchAll(/\bid="([^"]+)"/g)].map((match) => [match[1], new Element(match[1])]));
  const element = (id) => elements.get(id);
  for (const match of html.matchAll(/id="(player-page-tab-[^"]+)"[^>]+data-player-page="([^"]+)"/g)) {
    element(match[1]).dataset.playerPage = match[2];
  }
  for (const match of html.matchAll(/id="(character-tab-[^"]+)"[^>]+data-character-page="([^"]+)" aria-controls="([^"]+)"/g)) {
    element(match[1]).dataset.characterPage = match[2];
    element(match[1]).setAttribute("aria-controls", match[3]);
  }
  for (const match of html.matchAll(/id="(character-detail-tab-[^"]+)"[^>]+data-character-detail="([^"]+)" aria-controls="([^"]+)"/g)) {
    element(match[1]).dataset.characterDetail = match[2];
    element(match[1]).setAttribute("aria-controls", match[3]);
  }
  for (const match of html.matchAll(/id="([^"]+)"[^>]*aria-selected="([^"]+)"/g)) {
    element(match[1]).setAttribute("aria-selected", match[2]);
  }
  const document = {
    getElementById: element,
    querySelector: () => [...elements.values()].find((node) => node.open && node.id !== "player-page-dialog"),
  };
  const window = new EventTarget();
  const layout = new PlayerUiLayout({ document, window, localization: { format: (key) => key } });
  layout.initialize();
  layout.install();
  t.after(() => layout.dispose());
  return { layout, document, window, element };
}
