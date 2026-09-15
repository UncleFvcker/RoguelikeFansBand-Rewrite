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
  const history = new MessageHistory(2);
  assert.equal(history.append(record("first")), false);
  assert.equal(history.append(record("second")), false);
  assert.equal(history.append(record("third")), true);
  assert.deepEqual(
    history.records.map((entry) => entry.key),
    ["second", "third"],
  );

  history.clear();
  assert.deepEqual(history.records, []);
});

function record(key: string) {
  return { source: "key", turn: "1", kind: "system", key };
}
