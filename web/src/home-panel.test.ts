// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { parseHomeQuantity } from "./home-panel.ts";

test("home quantity parsing accepts bounded positive integers", () => {
  assert.equal(parseHomeQuantity("1", 4), 1);
  assert.equal(parseHomeQuantity("4", 4), 4);
  assert.equal(parseHomeQuantity("0", 4), undefined);
  assert.equal(parseHomeQuantity("5", 4), undefined);
  assert.equal(parseHomeQuantity("1.5", 4), undefined);
});
