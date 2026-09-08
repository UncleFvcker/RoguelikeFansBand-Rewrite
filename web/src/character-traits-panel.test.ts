// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in test runner.
import assert from "node:assert/strict";
import test from "node:test";
import { traitActionProtection, traitStatSourceValue, traitStatValue } from "./character-traits-panel.ts";

test("free action uses published paralysis immunity without inventing counts or revealing unknown gear", () => {
  const data = (statusImmunities, known = []) => ({ statusImmunities, sources: [{ statusImmunities: known }] });
  assert.equal(traitActionProtection(data([])), false);
  assert.equal(traitActionProtection(data(["rfb.status.paralysis"])), true);
  assert.equal(traitActionProtection(data(["rfb.status.stun"])), false);
  assert.equal(traitActionProtection(data(null)), null);
  assert.equal(traitActionProtection(data(null, ["rfb.status.paralysis"])), true);
  assert.equal(traitActionProtection(data(null, ["rfb.status.stun"])), null);
});

test("additive life and recovery sources use percentage points while totals remain percentages", () => {
  const localization = { format: (key, args) => `${key}:${args.value}` };
  assert.equal(traitStatSourceValue({ id: "equipment-life" }, 5, localization), "+trait-unit-percentage-points:5");
  assert.equal(traitStatSourceValue({ id: "natural-regeneration" }, -20, localization), "trait-unit-percentage-points:-20");
  assert.equal(traitStatSourceValue({ id: "speed" }, 2, localization), "+trait-unit-points:2");
  assert.equal(traitStatSourceValue({ id: "infravision" }, 3, localization), "+trait-unit-tiles:3");
});

test("trait values distinguish unknown, points, percentages and range without inventing ranks", () => {
  const localization = { format: (key, args) => args ? `${key}:${args.value}` : key };
  const value = (id, value) => traitStatValue({ id, value, sources: [] }, localization);
  assert.equal(value("speed", null), "trait-value-unknown");
  assert.equal(value("speed", 110), "trait-unit-points:110");
  assert.equal(value("stealth", 0), "trait-unit-points:0");
  assert.equal(value("equipment-life", 85), "trait-unit-percent:85");
  assert.equal(value("natural-regeneration", 200), "trait-unit-percent:200");
  assert.equal(value("mutation-regeneration", 10), "trait-unit-percent:10");
  assert.equal(value("infravision", 3), "trait-unit-tiles:3");
  assert.equal(value("melee-attacks", 2), "trait-unit-attacks:2");
  assert.equal(value("ranged-energy", 50), "trait-unit-energy:50");
  assert.equal(value("ranged-base-shot", 150), "trait-unit-percent:150");
});
