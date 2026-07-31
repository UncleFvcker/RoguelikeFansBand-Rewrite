// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { nativeSaveErrorKey } from "./save-panel.ts";

test("native save panel preserves actionable error message categories", () => {
  assert.equal(nativeSaveErrorKey("native-save-name-invalid"), "native-save-error-name-invalid");
  assert.equal(nativeSaveErrorKey("native-save-not-found"), "native-save-error-not-found");
  assert.equal(nativeSaveErrorKey("native-save-invalid"), "native-save-error-corrupt");
  assert.equal(nativeSaveErrorKey("native-save-read"), "native-save-error-read");
  assert.equal(nativeSaveErrorKey("native-save-write"), "native-save-error-write");
  assert.equal(nativeSaveErrorKey("native-save-lock"), "native-save-error-unavailable");
  assert.equal(nativeSaveErrorKey("unexpected"), "native-save-error-internal");
});
