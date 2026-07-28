// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { nativeSaveErrorCategory } from "./native-save-storage.ts";

test("native save error codes retain actionable categories", () => {
  const cases = {
    "name-invalid": ["native-save-name-invalid"],
    "not-found": ["native-save-not-found"],
    corrupt: ["native-save-invalid"],
    read: [
      "native-save-list",
      "native-save-read",
      "native-save-verify-read",
      "native-save-commit-read",
      "native-save-temp-list",
    ],
    write: [
      "native-save-temp-create",
      "native-save-write",
      "native-save-sync",
      "native-save-commit",
      "native-save-delete",
      "native-save-backup-remove",
      "native-save-backup-rotate",
      "native-save-backup-create",
      "native-save-temp-clean",
    ],
    unavailable: ["native-save-directory", "native-save-lock"],
    internal: [
      "native-save-clock",
      "native-save-id-exhausted",
      "native-save-id-invalid",
      "native-save-encode",
      "native-save-load",
      "desktop-storage-unknown",
    ],
  };

  for (const [category, codes] of Object.entries(cases)) {
    for (const code of codes) {
      assert.equal(nativeSaveErrorCategory(code), category, code);
    }
  }
  assert.equal(nativeSaveErrorCategory("native-save-future-error"), "internal");
});
