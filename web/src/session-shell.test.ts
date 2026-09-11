// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Executed directly by Node's built-in TypeScript test runner.

import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { CREATION_RACES, DRACONIAN_RACES, RACE_GROUPS, CAREER_GROUPS, CREATION_BUILDS, MAGE_REALMS, creationLeaves } from "./character-creation.ts";
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
  assert.deepEqual([...PLAYTEST_BUILD_IDS].sort(), [
    "demo.build.warrior",
    "demo.build.high-mage-death",
    "demo.build.high-mage-craft",
    "demo.build.archer",
    "demo.build.paladin-death",
    "demo.build.cavalry",
    "demo.build.sniper",
    "demo.build.mindcrafter",
    "demo.build.berserker",
    "demo.build.duelist",
    ...MAGE_REALMS.flatMap(first => MAGE_REALMS.filter(second => second !== first).map(second => `demo.build.mage-${first}-${second}`)),
  ].sort());
  assert.equal(PLAYTEST_BUILD_IDS.some((id) => id.startsWith("rfb-legacy.")), false);
});

test("the race menu exposes exactly the races accepted by core creation", () => {
  const directory = new URL("../../packs/rfb-demo-original/races/", import.meta.url);
  const formal = readdirSync(directory).map(file => JSON.parse(readFileSync(new URL(file, directory), "utf8")))
    .filter(race => race.tags.includes("rfb-compatibility"));
  assert.deepEqual([...PLAYTEST_RACE_IDS].sort(), formal.map(race => race.id).sort());
  assert.equal(PLAYTEST_RACE_IDS.length, 46);
  assert.equal(new Set(PLAYTEST_RACE_IDS).size, 46);
  assert.equal(RACE_GROUPS.length, 8);
  assert.ok(RACE_GROUPS.every(group => group.options.length > 0));
  for (const entry of CREATION_RACES) {
    const source = formal.find(race => race.id === entry.id);
    assert.equal(entry.nameKey, source.nameKey);
    assert.equal(entry.descriptionKey, source.descriptionKey);
  }
});

test("all nine Draconian subraces are leaves of the same parent", () => {
  const parent = RACE_GROUPS.flatMap(group => group.options).find(entry => "children" in entry);
  assert.equal(parent.id, "draconian");
  assert.deepEqual(parent.children, DRACONIAN_RACES);
  assert.deepEqual(DRACONIAN_RACES.map(race => race.id), [
    "red", "white", "blue", "black", "green", "bronze", "crystal", "gold", "shadow",
  ].map(color => `rfb-legacy.race.draconian-${color}`));
  assert.equal(PLAYTEST_RACE_IDS.includes("draconian"), false);
});

test("every race description and migrated special note is localized", () => {
  for (const locale of ["en-US", "zh-CN"]) {
    const text = ["content", "ui"].map(file => readFileSync(new URL(`../../locales/${locale}/${file}.ftl`, import.meta.url), "utf8")).join("\n");
    const keys = new Set([...text.matchAll(/^([a-z][a-z0-9-]*) =/gm)].map(match => match[1]));
    for (const race of [...CREATION_RACES, ...CAREER_GROUPS.flatMap(group => group.options), ...CREATION_BUILDS]) {
      for (const key of [race.nameKey, race.descriptionKey, ...race.notes]) assert.ok(keys.has(key), `${locale}: ${key}`);
    }
    for (const group of RACE_GROUPS) assert.ok(keys.has(`session-race-category-${group.id}`));
    for (const group of CAREER_GROUPS) assert.ok(keys.has(`session-career-category-${group.id}`));
  }
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

test("career leaves retain the existing class and realm mapping", () => {
  assert.equal(CAREER_GROUPS.length, 6);
  assert.equal(new Set(PLAYTEST_BUILD_IDS).size, 66);
  assert.deepEqual(CAREER_GROUPS.find(group => group.id === "melee").options.map(entry => entry.id), ["demo.build.warrior", "demo.build.berserker", "demo.build.duelist"]);
  assert.equal(CAREER_GROUPS.find(group => group.id === "mind").options[0].id, "demo.build.mindcrafter");
  assert.deepEqual(createNewSessionRequest("83", "demo.build.mindcrafter", "demo.race.rfb-human", "心灵术士"), {
    seed: "83", buildId: "demo.build.mindcrafter", raceId: "demo.race.rfb-human", playerName: "心灵术士",
  });
  for (const entry of CAREER_GROUPS.flatMap(group => group.options)) {
    const leaves = creationLeaves([entry]);
    for (const leaf of leaves) {
      const slug = leaf.id.slice("demo.build.".length);
      const build = JSON.parse(readFileSync(new URL(`../../packs/rfb-demo-original/builds/${slug}.json`, import.meta.url), "utf8"));
      const cls = JSON.parse(readFileSync(new URL(`../../packs/rfb-demo-original/classes/${build.classId.slice("demo.class.".length)}.json`, import.meta.url), "utf8"));
      assert.equal(entry.nameKey, cls.nameKey);
      assert.equal(entry.descriptionKey, cls.descriptionKey);
      if ("children" in entry) {
        assert.equal(leaves.length, entry.id === "mage" ? 56 : entry.id === "high-mage" ? 2 : 1);
        if (entry.id === "mage") {
          assert.ok(MAGE_REALMS.includes(build.firstRealmId));
          assert.ok(MAGE_REALMS.includes(build.secondRealmId));
          assert.notEqual(build.firstRealmId, build.secondRealmId);
        } else assert.equal(build.firstRealmId, leaf.id.endsWith("-craft") ? "craft" : "death");
        assert.equal(leaf.descriptionKey, build.descriptionKey);
        assert.ok(!PLAYTEST_BUILD_IDS.includes(entry.id));
      } else assert.equal(build.firstRealmId, undefined);
    }
  }
});

test("Mage realm branches exclude repeats and match every formal ordered Build", () => {
  const mage = CAREER_GROUPS.find(group => group.id === "magic").options.find(entry => entry.id === "mage");
  const sourceClass = JSON.parse(readFileSync(new URL("../../packs/rfb-demo-original/classes/mage.json", import.meta.url), "utf8"));
  assert.deepEqual([...MAGE_REALMS].sort(), sourceClass.castingProfile.realmProfiles.map(realm => realm.realmId).sort());
  assert.equal(mage.children.length, 8);
  for (const first of mage.children) {
    assert.equal(first.children.length, 7);
    const firstId = first.id.slice("mage-".length);
    assert.deepEqual(first.children.map(second => second.id), MAGE_REALMS.filter(second => second !== firstId).map(second => `demo.build.mage-${firstId}-${second}`));
    for (const locale of ["en-US", "zh-CN"]) {
      const ui = readFileSync(new URL(`../../locales/${locale}/ui.ftl`, import.meta.url), "utf8");
      for (const key of [first.nameKey, first.descriptionKey, first.childLabelKey]) assert.ok(ui.includes(`${key} =`));
    }
  }
});
