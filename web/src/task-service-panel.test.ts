// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import { taskActionForStatus } from "./task-service-panel.ts";

test("task service actions are limited to acceptance and reward claims", () => {
  assert.equal(taskActionForStatus("available"), "accept");
  assert.equal(taskActionForStatus("reward-available"), "claim");
  for (const status of [
    "abandoned",
    "active",
    "completed",
    "failed",
    "locked",
    "paused",
    "taken",
  ]) {
    assert.equal(taskActionForStatus(status), undefined);
  }
});
