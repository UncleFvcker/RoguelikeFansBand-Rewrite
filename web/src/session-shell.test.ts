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
    "rfb-legacy.race.half-giant",
    "rfb-legacy.race.half-troll",
    "rfb-legacy.race.half-titan",
    "rfb-legacy.race.cyclops",
    "rfb-legacy.race.yeek",
    "rfb-legacy.race.klackon",
    "rfb-legacy.race.dark-elf",
    "rfb-legacy.race.mindflayer",
    "rfb-legacy.race.imp",
    "rfb-legacy.race.draconian-red",
    "rfb-legacy.race.draconian-white",
    "rfb-legacy.race.draconian-blue",
    "rfb-legacy.race.draconian-black",
    "rfb-legacy.race.draconian-green",
    "rfb-legacy.race.draconian-bronze",
    "rfb-legacy.race.draconian-crystal",
    "rfb-legacy.race.draconian-gold",
    "rfb-legacy.race.draconian-shadow",
    "rfb-legacy.race.golem",
    "rfb-legacy.race.zombie",
    "rfb-legacy.race.skeleton",
    "rfb-legacy.race.wood-elf",
    "rfb-legacy.race.archon",
    "rfb-legacy.race.sprite",
    "rfb-legacy.race.snotling",
    "rfb-legacy.race.boit",
    "rfb-legacy.race.einheri",
    "rfb-legacy.race.kutar",
    "rfb-legacy.race.amberite",
    "rfb-legacy.race.beastman",
    "rfb-legacy.race.shadow-fairy",
    "rfb-legacy.race.ogre",
  ]);
});

test("the New Game form renders every formal race slice", () => {
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  for (const raceId of PLAYTEST_RACE_IDS) {
    assert.ok(indexHtml.includes(`<option value="${raceId}"`), raceId);
  }
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

test("New Game exposes and submits Golem with the device absorption action", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.golem"));
  assert.equal(
    createNewSessionRequest(
      "368",
      "demo.build.warrior",
      "rfb-legacy.race.golem",
      "Talos",
    ).raceId,
    "rfb-legacy.race.golem",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.golem"/);
  assert.match(indexHtml, /<button id="inventory-absorb"/);
});

test("New Game exposes and submits Zombie with the device absorption action", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.zombie"));
  assert.equal(
    createNewSessionRequest(
      "377",
      "demo.build.warrior",
      "rfb-legacy.race.zombie",
      "Morgoth",
    ).raceId,
    "rfb-legacy.race.zombie",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.zombie"/);
  assert.match(indexHtml, /<button id="inventory-absorb"/);
});

test("New Game exposes and submits Skeleton with the device absorption action", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.skeleton"));
  assert.equal(
    createNewSessionRequest(
      "378",
      "demo.build.warrior",
      "rfb-legacy.race.skeleton",
      "Skully",
    ).raceId,
    "rfb-legacy.race.skeleton",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.skeleton"/);
  assert.match(indexHtml, /<button id="inventory-absorb"/);
});

test("New Game exposes and submits Wood Elf", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.wood-elf"));
  assert.equal(
    createNewSessionRequest(
      "385",
      "demo.build.warrior",
      "rfb-legacy.race.wood-elf",
      "Legolas",
    ).raceId,
    "rfb-legacy.race.wood-elf",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.wood-elf"/);
});

test("New Game exposes and submits Archon", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.archon"));
  assert.equal(
    createNewSessionRequest(
      "387",
      "demo.build.warrior",
      "rfb-legacy.race.archon",
      "Raphael",
    ).raceId,
    "rfb-legacy.race.archon",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.archon"/);
});

test("New Game exposes and submits Sprite", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.sprite"));
  assert.equal(
    createNewSessionRequest(
      "393",
      "demo.build.warrior",
      "rfb-legacy.race.sprite",
      "Puck",
    ).raceId,
    "rfb-legacy.race.sprite",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.sprite"/);
});

test("New Game exposes and submits Snotling", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.snotling"));
  assert.equal(
    createNewSessionRequest(
      "395",
      "demo.build.warrior",
      "rfb-legacy.race.snotling",
      "Snaga",
    ).raceId,
    "rfb-legacy.race.snotling",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.snotling"/);
});

test("New Game exposes and submits Boit", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.boit"));
  assert.equal(
    createNewSessionRequest(
      "401",
      "demo.build.warrior",
      "rfb-legacy.race.boit",
      "Boit",
    ).raceId,
    "rfb-legacy.race.boit",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.boit"/);
});

test("New Game exposes and submits Einheri", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.einheri"));
  assert.equal(
    createNewSessionRequest(
      "409",
      "demo.build.warrior",
      "rfb-legacy.race.einheri",
      "Brynhild",
    ).raceId,
    "rfb-legacy.race.einheri",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.einheri"/);
});

test("New Game exposes and submits Kutar", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.kutar"));
  assert.equal(
    createNewSessionRequest(
      "411",
      "demo.build.warrior",
      "rfb-legacy.race.kutar",
      "Kutar",
    ).raceId,
    "rfb-legacy.race.kutar",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.kutar"/);
});

test("New Game exposes and submits Amberite", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.amberite"));
  assert.equal(
    createNewSessionRequest(
      "413",
      "demo.build.warrior",
      "rfb-legacy.race.amberite",
      "Corwin",
    ).raceId,
    "rfb-legacy.race.amberite",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.amberite"/);
});

test("New Game exposes and submits Beastman", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.beastman"));
  assert.equal(
    createNewSessionRequest(
      "419",
      "demo.build.warrior",
      "rfb-legacy.race.beastman",
      "Ghor",
    ).raceId,
    "rfb-legacy.race.beastman",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.beastman"/);
});

test("New Game exposes and submits Shadow-Fairy", () => {
  assert.ok(PLAYTEST_RACE_IDS.includes("rfb-legacy.race.shadow-fairy"));
  assert.equal(
    createNewSessionRequest(
      "421",
      "demo.build.warrior",
      "rfb-legacy.race.shadow-fairy",
      "Nyx",
    ).raceId,
    "rfb-legacy.race.shadow-fairy",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.shadow-fairy"/);
});

test("New Game exposes and submits Ogre", () => {
  assert.equal(PLAYTEST_RACE_IDS.at(-1), "rfb-legacy.race.ogre");
  assert.equal(
    createNewSessionRequest(
      "423",
      "demo.build.warrior",
      "rfb-legacy.race.ogre",
      "Shagrat",
    ).raceId,
    "rfb-legacy.race.ogre",
  );
  const indexHtml = readFileSync(new URL("../index.html", import.meta.url), "utf8");
  assert.match(indexHtml, /<option value="rfb-legacy\.race\.ogre"/);
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
  assert.equal(
    createNewSessionRequest(
      "92",
      "demo.build.warrior",
      "rfb-legacy.race.half-giant",
      "Gor",
    ).raceId,
    "rfb-legacy.race.half-giant",
  );
  assert.equal(
    createNewSessionRequest(
      "93",
      "demo.build.warrior",
      "rfb-legacy.race.half-troll",
      "Grish",
    ).raceId,
    "rfb-legacy.race.half-troll",
  );
  assert.equal(
    createNewSessionRequest(
      "94",
      "demo.build.high-mage-death",
      "rfb-legacy.race.half-titan",
      "Atlas",
    ).raceId,
    "rfb-legacy.race.half-titan",
  );
  assert.equal(
    createNewSessionRequest(
      "95",
      "demo.build.warrior",
      "rfb-legacy.race.cyclops",
      "Polyphemus",
    ).raceId,
    "rfb-legacy.race.cyclops",
  );
  assert.equal(
    createNewSessionRequest(
      "96",
      "demo.build.high-mage-death",
      "rfb-legacy.race.yeek",
      "Yip",
    ).raceId,
    "rfb-legacy.race.yeek",
  );
  assert.equal(
    createNewSessionRequest(
      "97",
      "demo.build.high-mage-death",
      "rfb-legacy.race.klackon",
      "Klick",
    ).raceId,
    "rfb-legacy.race.klackon",
  );
  assert.equal(
    createNewSessionRequest(
      "98",
      "demo.build.high-mage-death",
      "rfb-legacy.race.dark-elf",
      "Eol",
    ).raceId,
    "rfb-legacy.race.dark-elf",
  );
  assert.equal(
    createNewSessionRequest(
      "99",
      "demo.build.high-mage-death",
      "rfb-legacy.race.mindflayer",
      "Ilsensine",
    ).raceId,
    "rfb-legacy.race.mindflayer",
  );
  assert.equal(
    createNewSessionRequest(
      "100",
      "demo.build.high-mage-death",
      "rfb-legacy.race.imp",
      "Azazel",
    ).raceId,
    "rfb-legacy.race.imp",
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
