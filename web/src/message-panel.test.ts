// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { MessageHistory, MessagePanel } from "./message-panel.ts";

test("single rest steps stay quiet while terminal rest outcomes remain visible", () => {
  const element = () => ({ children: [], append(...children) { this.children.push(...children); } });
  const list = { ...element(), scrollHeight: 0, scrollTop: 0, clientHeight: 100, ownerDocument: { createElement: element } };
  const panel = new MessagePanel({ list, currentTurn: () => "1", historyLimit: 20, formatEvent: event => event.outcome.resolution.stopReason });
  const event = { kind: "rest.completed", outcome: { type: "rest", resolution: { requestedTurns: 1, completedTurns: 1, stopReason: "turn-limit" } } };
  panel.addEvent(event);
  assert.equal(list.children.length, 0);
  assert.equal(event.outcome.resolution.completedTurns, 1, "source receipt is preserved");
  for (const stopReason of ["full-resources", "damaged", "enemy-visible"]) {
    panel.addEvent({ ...event, outcome: { type: "rest", resolution: { ...event.outcome.resolution, stopReason } } });
  }
  assert.deepEqual(list.children.map(row => row.children[1].textContent), ["full-resources", "damaged", "enemy-visible"]);
});

test("message history preserves order and evicts only the oldest record", () => {
  const history = new MessageHistory(2, entry => entry.key);
  assert.equal(history.append(record("first")), "added");
  assert.equal(history.append(record("second")), "added");
  assert.equal(history.append(record("third")), "evicted");
  assert.deepEqual(
    history.records.map((entry) => entry.key),
    ["second", "third"],
  );

  history.clear();
  assert.deepEqual(history.records, []);
});

test("identical consecutive messages stack across turns but preserve intervening messages and severity", () => {
  const history = new MessageHistory(2, entry => `${entry.key}:${entry.args?.damage ?? ""}`);
  const first = record("hit");
  history.append(first);
  assert.equal(history.append({ ...first, turn: "2" }), "stacked");
  assert.equal(history.records.length, 1);
  assert.equal(history.records[0].count, 2);
  assert.equal(history.records[0].turn, "2");
  assert.equal(first.count, undefined, "the incoming event is not mutated");
  history.append({ ...first, args: { damage: 3 } });
  assert.equal(history.append(first), "evicted", "intervening messages break the stack");
  assert.equal(history.records.at(-1).count, 1);
  assert.equal(history.append({ ...first, kind: "error" }), "evicted", "different severity remains distinct");
  history.clear();
  history.append(first);
  assert.equal(history.records[0].count, 1);
});

test("stacked rows retain counts on translation, use the newest timestamp, and preserve manual scrolling", () => {
  const element = () => ({
    children: [],
    append(...children) { for (const child of children) { child.parent = this; this.children.push(child); } },
    replaceChildren(...children) { this.children = []; this.append(...children); },
    remove() { this.parent.children = this.parent.children.filter(child => child !== this); },
  });
  const list = { ...element(), scrollHeight: 200, scrollTop: 10, clientHeight: 50, ownerDocument: { createElement: element },
    get firstElementChild() { return this.children[0]; },
    get lastElementChild() { return this.children.at(-1); },
  };
  let turn = "48", text = "前方受阻。";
  const panel = new MessagePanel({ list, currentTurn: () => turn, historyLimit: 2,
    formatEvent: event => event.messageKey === "blocked" ? text : event.messageKey,
    localization: { format: () => text }, localizedArgs: () => undefined });
  panel.addEvent({ kind: "animation.projectile-start", messageKey: "", args: {} });
  panel.addEvent({ kind: "animation.projectile-end", messageKey: "", args: {} });
  assert.equal(list.children.length, 0, "presentation delimiters do not create message rows");
  panel.addEvent({ kind: "move.blocked", messageKey: "blocked", revision: 1 });
  const row = list.children[0];
  turn = "49";
  panel.addEvent({ kind: "move.blocked", messageKey: "blocked", revision: 2 });
  assert.equal(list.children.length, 1);
  assert.equal(list.children[0], row, "update in place without reordering the list");
  assert.equal(row.children[0].textContent, "49");
  assert.equal(row.children[1].textContent, "前方受阻。 ×2");
  assert.equal(list.scrollTop, 10);
  text = "The way is blocked.";
  panel.render();
  assert.equal(list.children[0].children[1].textContent, "The way is blocked. ×2");
  assert.equal(list.scrollTop, 10);
  panel.addLocalized("blocked", undefined, "move.blocked");
  assert.equal(list.children[0].children[1].textContent, "The way is blocked. ×3");
  panel.addEvent({ kind: "combat.hit", messageKey: "hit" });
  list.scrollTop = 150;
  panel.addEvent({ kind: "move.blocked", messageKey: "blocked" });
  assert.deepEqual(list.children.map(row => row.children[1].textContent), ["hit", "The way is blocked."]);
  assert.equal(list.scrollTop, 200, "follow new messages when already at the bottom");
  panel.clear();
  panel.addEvent({ kind: "move.blocked", messageKey: "blocked" });
  assert.equal(list.children.length, 1);
  assert.equal(list.children[0].children[1].textContent, "The way is blocked.");
});

function record(key: string) {
  return { source: "key", turn: "1", kind: "system", key };
}
