// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { formatAttributeValue } from "./status-panel.ts";

test("status panel preserves exceptional attribute display values", () => {
  assert.equal(formatAttributeValue(18), "18");
  assert.equal(formatAttributeValue(19), "18/1");
  assert.equal(formatAttributeValue(27), "18/9");
});
