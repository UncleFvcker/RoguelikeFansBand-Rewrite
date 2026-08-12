// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import test from "node:test";

import {
  PLAYTEST_BUILD_IDS,
  canonicalCharacterName,
  canonicalSessionSeed,
  randomSessionSeed,
} from "./session-shell.ts";

test("character names are trimmed and bounded", () => {
  assert.equal(canonicalCharacterName("  Beren  "), "Beren");
  assert.equal(canonicalCharacterName(""), undefined);
  assert.equal(canonicalCharacterName("a".repeat(33)), undefined);
  assert.equal(canonicalCharacterName("bad\nname"), undefined);
});

test("new character creation exposes the formal Warrior, Death High-Mage and Archer slices", () => {
  assert.deepEqual(PLAYTEST_BUILD_IDS, [
    "demo.build.warrior",
    "demo.build.high-mage-death",
    "demo.build.archer",
  ]);
  assert.equal(PLAYTEST_BUILD_IDS.some((id) => id.startsWith("rfb-legacy.")), false);
});

test("session seeds canonicalize the complete unsigned 64-bit range", () => {
  assert.equal(canonicalSessionSeed(" 00042 "), "42");
  assert.equal(canonicalSessionSeed("0"), "0");
  assert.equal(canonicalSessionSeed("18446744073709551615"), "18446744073709551615");
  assert.equal(canonicalSessionSeed("18446744073709551616"), undefined);
  assert.equal(canonicalSessionSeed("-1"), undefined);
  assert.equal(canonicalSessionSeed("4.2"), undefined);
  assert.equal(canonicalSessionSeed(""), undefined);
});

test("random session seeds combine two entropy words without truncation", () => {
  const source = {
    getRandomValues(values) {
      values[0] = 0x12345678;
      values[1] = 0x9abcdef0;
      return values;
    },
  };

  assert.equal(randomSessionSeed(source), "1311768467463790320");
});
