// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { MessageHistory } from "./message-panel.ts";

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
