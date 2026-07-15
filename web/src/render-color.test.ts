// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { contrastRatio, ensureContrast } from "./render-color.ts";

test("contrast guard preserves readable colors and replaces converged colors", () => {
  assert.equal(ensureContrast(0xffffff, 0x111820), 0xffffff);
  const guarded = ensureContrast(0x26303d, 0x26303d);
  assert.ok(contrastRatio(guarded, 0x26303d) >= 3);
});
