// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  PLAYTEST_BUILD_IDS,
  PLAYTEST_RACE_IDS,
  canonicalCharacterName,
  canonicalSessionSeed,
  createNewSessionRequest,
  randomSessionSeed,
} from "./session-shell.ts";

test("character names are trimmed and bounded", () => {
  assert.equal(canonicalCharacterName("  Beren  "), "Beren");
  assert.equal(canonicalCharacterName(""), undefined);
  assert.equal(canonicalCharacterName("a".repeat(33)), undefined);
  assert.equal(canonicalCharacterName("bad\nname"), undefined);
});

test("new character creation exposes all formal class slices", () => {
  assert.deepEqual(PLAYTEST_BUILD_IDS, [
    "demo.build.warrior",
    "demo.build.high-mage-death",
    "demo.build.archer",
    "demo.build.paladin-death",
    "demo.build.cavalry",
    "demo.build.sniper",
  ]);
  assert.equal(PLAYTEST_BUILD_IDS.some((id) => id.startsWith("rfb-legacy.")), false);
});

test("new character creation exposes only formal race slices", () => {
  assert.deepEqual(PLAYTEST_RACE_IDS, [
    "demo.race.rfb-human",
    "rfb-legacy.race.half-orc",
    "rfb-legacy.race.high-elf",
    "rfb-legacy.race.dunadan",
    "rfb-legacy.race.barbarian",
    "rfb-legacy.race.hobbit",
    "rfb-legacy.race.kobold",
    "rfb-legacy.race.dwarf",
    "rfb-legacy.race.nibelung",
    "rfb-legacy.race.gnome",
  ]);
});

test("the New Game form renders every formal race slice", () => {
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  for (const raceId of PLAYTEST_RACE_IDS) {
    assert.ok(indexHtml.includes(`<option value="${raceId}"`), raceId);
  }
});

test("new character requests preserve the selected formal race", () => {
  assert.deepEqual(
    createNewSessionRequest(
      "83",
      "demo.build.warrior",
      "rfb-legacy.race.half-orc",
      "Gorbag",
    ),
    {
      seed: "83",
      buildId: "demo.build.warrior",
      raceId: "rfb-legacy.race.half-orc",
      playerName: "Gorbag",
    },
  );
  assert.equal(
    createNewSessionRequest(
      "84",
      "demo.build.warrior",
      "rfb-legacy.race.high-elf",
      "Finrod",
    ).raceId,
    "rfb-legacy.race.high-elf",
  );
  assert.equal(
    createNewSessionRequest(
      "85",
      "demo.build.warrior",
      "rfb-legacy.race.dunadan",
      "Aragorn",
    ).raceId,
    "rfb-legacy.race.dunadan",
  );
  assert.equal(
    createNewSessionRequest(
      "86",
      "demo.build.warrior",
      "rfb-legacy.race.barbarian",
      "Conan",
    ).raceId,
    "rfb-legacy.race.barbarian",
  );
  assert.equal(
    createNewSessionRequest(
      "87",
      "demo.build.warrior",
      "rfb-legacy.race.hobbit",
      "Bilbo",
    ).raceId,
    "rfb-legacy.race.hobbit",
  );
  assert.equal(
    createNewSessionRequest(
      "88",
      "demo.build.warrior",
      "rfb-legacy.race.kobold",
      "Kob",
    ).raceId,
    "rfb-legacy.race.kobold",
  );
  assert.equal(
    createNewSessionRequest(
      "89",
      "demo.build.warrior",
      "rfb-legacy.race.dwarf",
      "Gimli",
    ).raceId,
    "rfb-legacy.race.dwarf",
  );
  assert.equal(
    createNewSessionRequest(
      "90",
      "demo.build.warrior",
      "rfb-legacy.race.nibelung",
      "Alberich",
    ).raceId,
    "rfb-legacy.race.nibelung",
  );
  assert.equal(
    createNewSessionRequest(
      "91",
      "demo.build.high-mage-sorcery",
      "rfb-legacy.race.gnome",
      "Fizzwick",
    ).raceId,
    "rfb-legacy.race.gnome",
  );
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
