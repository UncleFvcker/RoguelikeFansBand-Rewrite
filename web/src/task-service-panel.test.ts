// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  bountyMissionAction,
  facilityIdentificationCandidate,
  facilityMembershipKey,
  facilityServiceActionKey,
  facilityServiceUsesItem,
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

test("p105d Anambar facility roles and typed service actions stay stable", () => {
  assert.equal(facilityMembershipKey("visitor"), "facility-membership-visitor");
  assert.equal(facilityMembershipKey("member"), "facility-membership-member");
  assert.equal(facilityMembershipKey("owner"), "facility-membership-owner");

  assert.equal(facilityServiceUsesItem("heal"), false);
  assert.equal(facilityServiceUsesItem("assess-armor"), false);
  assert.equal(facilityServiceUsesItem("recall"), false);
  assert.equal(facilityServiceUsesItem("enchant-weapon"), true);
  assert.equal(facilityServiceUsesItem("enchant-armor"), true);
  assert.equal(facilityServiceUsesItem("enchant-ammunition"), true);
  assert.equal(facilityServiceUsesItem("enchant-bow"), true);
  assert.equal(facilityServiceActionKey("restore-vitality"), "action-facility-restore-vitality");
  assert.equal(facilityServiceActionKey("cure-mutation"), "action-facility-cure-mutation");
});

test("p106d bounty mission controls follow the authoritative mission state", () => {
  assert.equal(bountyMissionAction(undefined), "request-mission");
  assert.equal(bountyMissionAction("active"), "abandon-mission");
  assert.equal(bountyMissionAction("reward-available"), "claim-mission-reward");
});
