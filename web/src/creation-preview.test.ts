// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { CreationPreview } from "./creation-preview.ts";

// Only the DOM operations used by the preview; no browser or native initialization.
function harness() {
  class Element {
    children = []; attributes = {}; dataset = {}; textContent = ""; listeners = {};
    ownerDocument = document; parentElement = null;
    contains(node) { return node === this || this.children.some(child => child.contains(node)); }
    focus() { document.activeElement = this; }
    get lastElementChild() { return this.children.at(-1); }
    setAttribute(key, value) { this.attributes[key] = value; }
    append(...children) { for (const child of children) { child.remove(); child.parentElement = this; this.children.push(child); } }
    prepend(child) { child.remove(); child.parentElement = this; this.children.unshift(child); }
    replaceChildren(...children) { for (const child of [...this.children]) child.remove(); this.append(...children); }
    remove() { if (this.contains(document.activeElement)) document.activeElement = null; if (this.parentElement) this.parentElement.children = this.parentElement.children.filter(child => child !== this); this.parentElement = null; }
    addEventListener(key, handler) { this.listeners[key] = handler; }
    insertRow() { const row = document.createElement("tr"); this.append(row); return row; }
    createTHead() { const head = document.createElement("thead"); this.append(head); return head; }
    createTBody() { const body = document.createElement("tbody"); this.append(body); return body; }
  }
  const document = { activeElement: null, createElement: tag => Object.assign(new Element(), { tag }) };
  const localization = { locale: "zh-CN", format: key => key };
  const requests = [];
  const preview = new CreationPreview(document, localization, (buildId, raceId) => new Promise((resolve, reject) => requests.push({ buildId, raceId, resolve, reject })));
  const host = Object.assign(document.createElement("div"), { id: "overview" });
  const text = () => { const flatten = node => [node.textContent, ...node.children.flatMap(flatten)]; return flatten(host).join(" "); };
  return { document, preview, requests, host, text };
}

const data = (raceNameKey) => ({
  build: { raceNameKey, buildNameKey: "warrior", personalityNameKey: "ordinary", lifePercent: 115, experiencePercent: 100 },
  baseHp: 28, castingAttribute: "intelligence", experienceNoteKey: null,
  sources: [], features: [],
  attributes: [{ attribute: "intelligence", natural: 18, modifier: 1, effective: 28 }],
  skills: [{ skill: { id: "device", nameKey: "device", base: 18, growthPerTenLevels: 7 }, ratingKey: "creation-rating-bad" }],
});

test("creation preview renders only the latest candidate and deduplicates repeated focus", async () => {
  const h = harness();
  const first = h.preview.show(h.host, { buildId: "warrior", raceId: "human" });
  const second = h.preview.show(h.host, { buildId: "warrior", raceId: "elf" });
  await h.preview.show(h.host, { buildId: "warrior", raceId: "elf" });
  assert.equal(h.requests.length, 2);
  h.requests[1].resolve(data("elf")); await second;
  assert.match(h.text(), /elf/);
  assert.match(h.text(), /18\/10/);
  assert.match(h.text(), /creation-preview-casting/);
  h.requests[0].reject(new Error("old failure")); await first;
  assert.match(h.text(), /elf/);
  assert.doesNotMatch(h.text(), /old failure/);
});

test("incomplete choices and leaving creation invalidate pending data", async () => {
  const h = harness();
  const pending = h.preview.show(h.host, { buildId: "warrior", raceId: "human" });
  await h.preview.show(h.host, undefined);
  h.requests[0].resolve(data("human")); await pending;
  assert.match(h.text(), /creation-preview-incomplete/);
  assert.doesNotMatch(h.text(), /human/);
  const retry = h.preview.show(h.host, { buildId: "warrior", raceId: "elf" });
  h.preview.reset(); h.requests[1].resolve(data("elf")); await retry;
  assert.equal(h.host.children.length, 0);
});

test("a current failure clears old numbers and permits an explicit retry", async () => {
  const h = harness();
  const initial = h.preview.show(h.host, { buildId: "warrior", raceId: "human" });
  h.requests[0].resolve(data("human")); await initial;
  const next = h.preview.show(h.host, { buildId: "warrior", raceId: "elf" });
  assert.doesNotMatch(h.text(), /human|115%/);
  h.requests[1].reject(new Error("unavailable")); await next;
  assert.match(h.text(), /creation-preview-error/);
  const button = h.host.children[0].children.find(child => child.tag === "button");
  button.focus();
  button.listeners.click();
  assert.equal(h.document.activeElement, h.host.children[0], "retry keeps focus inside the preview");
  assert.equal(h.host.children[0].dataset.state, "loading");
  assert.equal(h.requests.length, 3);
  assert.equal(h.requests[2].raceId, "elf");
  h.requests[2].resolve(data("elf")); await new Promise(resolve => setImmediate(resolve));
  assert.match(h.text(), /elf/);
  assert.equal(h.document.activeElement, h.host.children[0]);
  assert.equal(h.host.children[0].dataset.state, "ready");
});

test("a completed preview does not steal focus from the next control", async () => {
  const h = harness();
  const show = h.preview.show(h.host, { buildId: "warrior", raceId: "human" });
  h.requests[0].reject(new Error("offline")); await show;
  const button = h.host.children[0].children.find(child => child.tag === "button");
  button.focus(); button.listeners.click();
  const next = h.document.createElement("button"); next.focus();
  h.requests[1].resolve(data("human")); await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.document.activeElement, next);
  assert.equal(h.host.children[0].dataset.state, "ready");
});

test("comparison uses the confirmed build and disappears after confirming the candidate", async () => {
  const h = harness();
  const elf = { buildId: "mage", raceId: "elf" }, human = { buildId: "warrior", raceId: "human" };
  const show = h.preview.show(h.host, elf, human);
  const baseline = data("human"); baseline.attributes[0].effective = 18;
  h.requests[0].resolve(data("elf")); h.requests[1].resolve(baseline); await show;
  assert.match(h.text(), /18 → 18\/10/);
  assert.match(h.text(), /creation-compare-with/);
  assert.deepEqual([h.requests[1].buildId, h.requests[1].raceId], ["warrior", "human"]);
  const confirm = h.preview.show(h.host, elf, elf);
  assert.equal(h.requests.length, 3);
  h.requests[2].resolve(data("elf")); await confirm;
  assert.doesNotMatch(h.text(), /creation-compare-with|18 →/);
});

test("an invalid comparison baseline does not discard a valid candidate", async () => {
  const h = harness();
  const show = h.preview.show(h.host, { buildId: "mage", raceId: "elf" }, { buildId: "bad", raceId: "human" });
  h.requests[0].resolve(data("elf")); h.requests[1].reject(new Error("incompatible baseline")); await show;
  assert.match(h.text(), /elf/);
  assert.match(h.text(), /creation-compare-unavailable/);
  assert.doesNotMatch(h.text(), /creation-preview-error/);
});

test("source contributions and acquisition conditions remain distinct", async () => {
  const h = harness(), sample = data("elf");
  sample.sources = [{ kind: "race", nameKey: "elf", modifiers: { intelligence: 2 }, lifePercent: 95, baseHp: 10, experiencePercent: 120, skills: [] }];
  sample.features = [
    { sourceNameKey: "elf", nameKey: "poison", name: null, description: null, detailKey: "resistance-level-vulnerable", value: null, minimumLevel: 1, maximumLevel: null, acquisitionKey: "creation-acquire-automatic", negative: true },
    { sourceNameKey: "mage", nameKey: "books", name: null, description: null, detailKey: null, value: null, minimumLevel: 3, maximumLevel: null, acquisitionKey: "creation-acquire-study", negative: false },
  ];
  const pending = h.preview.show(h.host, { buildId: "mage", raceId: "elf" });
  h.requests[0].resolve(sample); await pending;
  assert.match(h.text(), /attribute-source-kind-race/);
  assert.match(h.text(), /\+2/);
  assert.match(h.text(), /creation-features-restrictions.*resistance-level-vulnerable/);
  assert.match(h.text(), /creation-features-growth.*creation-acquire-study/);
});
