// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";
import { formatManaRecovery } from "./mana-recovery.ts";

test("mana rates distinguish fractional recovery, no recovery, and upkeep", () => {
  assert.equal(formatManaRecovery(2297), "+0.035");
  assert.equal(formatManaRecovery(4070), "+0.062");
  assert.equal(formatManaRecovery(-2297), "−0.035");
  assert.equal(formatManaRecovery(0), "0");
  assert.equal(formatManaRecovery(1), "+<0.001");
});
