// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { Hotbar, emptyHotbar, hotbarItems, itemHotbarBinding, validHotbar } from "./hotbar.ts";
import { PreferencesClient, defaultPreferences } from "./preferences.ts";
import { directionForKeyboardInput } from "./input-controller.ts";

test("item bindings resolve current stacks and preserve artifact and inscription distinctions", () => {
  const potion = { id: "old-stack", kindId: "potion", useCategory: "potion", artifactName: null, inscription: "!q" };
  const binding = itemHotbarBinding(potion);
  const replacement = { ...potion, id: "new-stack" };
  assert.deepEqual(hotbarItems(binding, [replacement, { ...potion, inscription: null }, { ...potion, artifactName: "artifact" }], []), [replacement]);
  const slots = emptyHotbar(); slots[0] = binding; slots[59] = { type: "ability", id: "heal" };
  assert.equal(validHotbar(slots), true);
  assert.equal(validHotbar(slots.slice(0, 10)), false);
  assert.equal(validHotbar([{ ...binding, action: "destroy" }, ...slots.slice(1)]), false);
});

test("number row uses six banks while numpad, repeats and modal editing keep their existing input", async t => {
  class Element extends EventTarget {
    dataset = {}; textContent = ""; attributes = new Map();
    setAttribute(name, value) { this.attributes.set(name, value); }
    matches() { return false; }
  }
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "HTMLElement");
  Object.defineProperty(globalThis, "HTMLElement", { configurable: true, value: Element });
  t.after(() => { if (descriptor) Object.defineProperty(globalThis, "HTMLElement", descriptor); else delete globalThis.HTMLElement; });
  const slots = Array.from({ length: 10 }, (_, index) => Object.assign(new Element(), { dataset: { shortcutSlot: String(index + 1) } }));
  const pages = [0, 1, 2, 3, 4, 5].map(page => Object.assign(new Element(), { dataset: { hotbarPage: String(page) } }));
  let modal = false, blocked = false, fail = false, numberInput = false;
  const preferences = defaultPreferences();
  preferences.hotbar[0] = { type: "ability", id: "first" };
  preferences.hotbar[10] = { type: "ability", id: "second" };
  preferences.hotbar[19] = { type: "ability", id: "last" };
  for (let page = 3; page <= 6; page++) preferences.hotbar[page * 10 - 1] = { type: "ability", id: `bank-${page}-last` };
  const client = new PreferencesClient({ defaults: async () => defaultPreferences(),
    load: async () => ({ revision: 1, preferences }),
    save: async (p, revision) => { if (fail) throw Error("disk full"); return { revision: revision + 1, preferences: structuredClone(p) }; },
  });
  await client.load();
  const window = new EventTarget(), calls = [], errors = [];
  const hotbar = new Hotbar({ window, document: {
    querySelectorAll: selector => selector === "[data-shortcut-slot]" ? slots : pages,
    querySelector: () => modal ? {} : null,
  }, state: { mode: "playing", inventory: [], equipment: [] }, preferences: client,
    localization: { format: key => key }, itemName: key => key, cast: id => calls.push(id), use() {}, blocked: () => blocked, commandNumberInputActive: () => numberInput, error: error => errors.push(error),
  });
  hotbar.install(); t.after(() => hotbar.dispose());
  const press = (key, code, extra = {}) => {
    const event = new Event("keydown", { cancelable: true });
    Object.assign(event, { key, code, ...extra }); window.dispatchEvent(event); return event;
  };
  assert.equal(press("1", "Digit1").defaultPrevented, true);
  assert.equal(press("2", "Digit2", { altKey: true }).defaultPrevented, true);
  press("1", "Digit1"); press("0", "Digit0");
  assert.deepEqual(calls, ["first", "second", "last"]);
  for (let page = 3; page <= 6; page++) {
    assert.equal(press(String(page), `Digit${page}`, { altKey: true }).defaultPrevented, true);
    press("0", "Digit0");
    assert.equal(calls.at(-1), `bank-${page}-last`);
    assert.equal(pages[page - 1].attributes.get("aria-pressed"), "true");
  }
  assert.equal(press("7", "Digit7", { altKey: true }).defaultPrevented, false);
  press("2", "Digit2", { altKey: true });
  const expectedCalls = [...calls];
  for (const key of ["1", "End"]) {
    const event = press(key, "Numpad1");
    assert.equal(event.defaultPrevented, false);
    assert.equal(directionForKeyboardInput(event, "original"), "south-west");
  }
  press("1", "Digit1", { repeat: true });
  modal = true; assert.equal(press("1", "Digit1").defaultPrevented, false); modal = false;
  numberInput = true; assert.equal(press("1", "Digit1").defaultPrevented, false); numberInput = false;
  blocked = true; press("1", "Digit1"); blocked = false;
  assert.deepEqual(calls, expectedCalls);
  fail = true; slots[0].dispatchEvent(new Event("contextmenu", { cancelable: true }));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(client.snapshot.preferences.hotbar[10].id, "second"); assert.equal(errors.length, 1);
  fail = false; slots[0].dispatchEvent(new Event("contextmenu", { cancelable: true }));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(client.snapshot.preferences.hotbar[10], null);
  assert.equal(client.snapshot.preferences.hotbar[0].id, "first");
});
