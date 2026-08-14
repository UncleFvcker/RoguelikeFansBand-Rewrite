// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  facilityIdentificationCandidate,
  taskActionForStatus,
} from "./task-service-panel.ts";

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

test("p104d Anambar library distinguishes identification from research candidates", () => {
  assert.equal(facilityIdentificationCandidate("unexamined", false), true);
  assert.equal(facilityIdentificationCandidate("appraised", false), false);
  assert.equal(facilityIdentificationCandidate("identified", false), false);

  assert.equal(facilityIdentificationCandidate("unexamined", true), true);
  assert.equal(facilityIdentificationCandidate("appraised", true), true);
  assert.equal(facilityIdentificationCandidate("identified", true), false);
});
