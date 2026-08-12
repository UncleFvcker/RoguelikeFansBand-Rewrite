// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  abilityStatusMessageKey,
  formatAttributeValue,
  mutationRatingMessageKey,
  nutritionPercentage,
  wildernessClock,
} from "./status-panel.ts";

test("mutation presentation exposes ratings and the shared ability source", () => {
  assert.equal(mutationRatingMessageKey("awful"), "mutation-rating-awful");
  assert.equal(mutationRatingMessageKey("great"), "mutation-rating-great");
  assert.equal(
    abilityStatusMessageKey({ source: "mutation", learned: false }),
    "ability-status-mutation",
  );
  assert.equal(
    abilityStatusMessageKey({ source: "learned", learned: true }),
    "ability-status-learned",
  );
  assert.equal(
    abilityStatusMessageKey({ source: "class", learned: false }),
    "ability-status-class",
  );
});

test("status panel preserves exceptional attribute display values", () => {
  assert.equal(formatAttributeValue(18), "18");
  assert.equal(formatAttributeValue(19), "18/1");
  assert.equal(formatAttributeValue(27), "18/9");
});

test("status panel displays nutrition relative to the 10000 baseline", () => {
  assert.equal(nutritionPercentage(15_000), 150);
  assert.equal(nutritionPercentage(10_000), 100);
  assert.equal(nutritionPercentage(9_999), 99);
});

test("wilderness clock follows the original half-day boundaries", () => {
  assert.deepEqual(wildernessClock(0), { day: 1, hour: 6, minute: 0, daytime: true });
  assert.deepEqual(wildernessClock(49_999), {
    day: 1,
    hour: 17,
    minute: 59,
    daytime: true,
  });
  assert.deepEqual(wildernessClock(50_000), {
    day: 1,
    hour: 18,
    minute: 0,
    daytime: false,
  });
  assert.deepEqual(wildernessClock(75_000), {
    day: 2,
    hour: 0,
    minute: 0,
    daytime: false,
  });
  assert.deepEqual(wildernessClock(100_000), {
    day: 2,
    hour: 6,
    minute: 0,
    daytime: true,
  });
});
