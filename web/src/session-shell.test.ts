// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

test("the main window explicitly permits the close command used by both exit buttons", () => {
  const capability = JSON.parse(readFileSync(new URL("../src-tauri/capabilities/default.json", import.meta.url), "utf8"));
  assert.deepEqual(capability.windows, ["main"]);
  assert.deepEqual(capability.permissions, ["core:default", "core:window:allow-close"]);
});

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

test("the New Game race options match the formal race list", () => {
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const select = indexHtml.match(/<select\b[^>]*\bid="session-race"[^>]*>([\s\S]*?)<\/select>/)?.[1];
  assert.ok(select, "New Game should expose the race selector");
  const raceIds = [...select.matchAll(/<option\b[^>]*\bvalue="([^"]+)"/g)]
    .map((match) => match[1]);
  assert.deepEqual(raceIds, PLAYTEST_RACE_IDS);
  assert.equal(raceIds.length, 43);
  assert.ok(raceIds.includes("rfb-legacy.race.tomte"));
});

test("the New Game form groups all nine formal Draconian subraces", () => {
  const draconianRaceIds = [
    "rfb-legacy.race.draconian-red",
    "rfb-legacy.race.draconian-white",
    "rfb-legacy.race.draconian-blue",
    "rfb-legacy.race.draconian-black",
    "rfb-legacy.race.draconian-green",
    "rfb-legacy.race.draconian-bronze",
    "rfb-legacy.race.draconian-crystal",
    "rfb-legacy.race.draconian-gold",
    "rfb-legacy.race.draconian-shadow",
  ];
  const draconianStart = PLAYTEST_RACE_IDS.indexOf("rfb-legacy.race.draconian-red");
  assert.deepEqual(
    PLAYTEST_RACE_IDS.slice(draconianStart, draconianStart + draconianRaceIds.length),
    draconianRaceIds,
  );

  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  const groupStart = indexHtml.indexOf(
    '<optgroup label="" data-l10n-label="session-race-group-draconian">',
  );
  const groupEnd = indexHtml.indexOf("</optgroup>", groupStart);
  assert.ok(groupStart >= 0 && groupEnd > groupStart);
  const groupMarkup = indexHtml.slice(groupStart, groupEnd);
  for (const raceId of draconianRaceIds) {
    assert.ok(groupMarkup.includes(`<option value="${raceId}"`), raceId);
  }

  const english = readFileSync(
    new URL("../../locales/en-US/ui.ftl", import.meta.url),
    "utf8",
  );
  const chinese = readFileSync(
    new URL("../../locales/zh-CN/ui.ftl", import.meta.url),
    "utf8",
  );
  assert.match(english, /^session-race-group-draconian = Draconians$/m);
  assert.match(chinese, /^session-race-group-draconian = 龙人分支$/m);
});

test("new character requests preserve the selected setup", () => {
  assert.deepEqual(
    createNewSessionRequest("83", "demo.build.warrior", "rfb-legacy.race.half-orc", "Gorbag"),
    {
      seed: "83",
      buildId: "demo.build.warrior",
      raceId: "rfb-legacy.race.half-orc",
      playerName: "Gorbag",
    },
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
