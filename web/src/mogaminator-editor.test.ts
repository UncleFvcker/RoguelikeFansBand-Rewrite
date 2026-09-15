// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript test runner; execution deferred to O8.
import assert from "node:assert/strict";
import test from "node:test";
import { MogaminatorEditor } from "./mogaminator-editor.ts";
import { defaultPreferences } from "./preferences.ts";

test("reload uses the shared callback and keeps unsaved source and policy on success or failure", async () => {
  class Element extends EventTarget {
    value = ""; textContent = ""; open = false; checked = false; disabled = false; selectionStart = 0;
    setAttribute() {} focus() {} showModal() { this.open = true; } close() { this.open = false; }
    replaceChildren() { this.textContent = ""; }
  }
  const nodes = new Map();
  const element = id => { if (!nodes.has(id)) nodes.set(id, new Element()); return nodes.get(id); };
  const document = { getElementById: element, createElement: () => new Element() };
  for (const node of nodes.values()) node.ownerDocument = document;
  const status = { locale: "zh-CN", enabled: false, leaveDestroyedItems: false, autoGetMode: "off",
    source: "物品", defaultSource: "物品", protectionTemplates: [], diagnostics: [], lines: [], matches: [] };
  const state = { busy: false, status: { mogaminator: status } };
  let calls = 0, fail = false;
  const editor = new MogaminatorEditor({ document, window: { requestAnimationFrame: f => f() }, state,
    localization: { locale: "zh-CN", format: (key, args) => key + JSON.stringify(args ?? {}) },
    preferences: { snapshot: { revision: 1, preferences: defaultPreferences() } },
    commit: () => assert.fail("reload must not save"),
    reloadSaved: async () => {
      calls++; if (fail) throw new Error("preferences-corrupt");
      editor.render({ ...status, source: "!物品", enabled: true, autoGetMode: "wanted" });
    },
  });
  for (const node of nodes.values()) node.ownerDocument = document;
  editor.install(); editor.open();
  element("mogaminator-source").value = "draft text";
  element("mogaminator-source").dispatchEvent(new Event("input"));
  element("mogaminator-enabled").checked = false;
  element("mogaminator-auto-get-mode").value = "ammo";
  element("mogaminator-reload").dispatchEvent(new Event("click"));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls, 1);
  assert.equal(element("mogaminator-source").value, "draft text");
  assert.equal(element("mogaminator-enabled").checked, false);
  assert.equal(element("mogaminator-auto-get-mode").value, "ammo");
  fail = true; element("mogaminator-reload").dispatchEvent(new Event("click"));
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(calls, 2);
  assert.match(element("mogaminator-diagnostics").textContent, /preferences-corrupt/);
  assert.equal(element("mogaminator-source").value, "draft text");
  assert.equal(element("mogaminator-reload").disabled, false);
  editor.dispose();
});
