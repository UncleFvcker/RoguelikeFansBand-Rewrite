// SPDX-License-Identifier: MPL-2.0
// @ts-nocheck -- Node's built-in TypeScript runner.
import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { HELP_TOPICS, KNOWLEDGE_GROUPS, archiveRows } from "./help-knowledge.ts";
import { Localization } from "./localization.ts";

test("manual and knowledge resolve dynamic Fluent keys in both locales", () => {
  const sources = Object.fromEntries(["en-US", "zh-CN"].map(locale => [locale,
    ["ui", "game", "content"].map(name => readFileSync(new URL(`../../locales/${locale}/${name}.ftl`, import.meta.url), "utf8"))]));
  for (const locale of ["en-US", "zh-CN"]) {
    const l = new Localization(locale, sources);
    const keys = HELP_TOPICS.flatMap(topic => [`guide-topic-${topic}`, ...(topic === "commands" ?
      ["original", "roguelike"].map(preset => `controls-${preset}`) : [`guide-text-${topic}`])]);
    keys.push(...["search", "filter", "all", "alive", "dead", "count", "depth", "conquered", "yes", "no", "kills", "life", "empty"].map(key => `archive-${key}`));
    for (const group of KNOWLEDGE_GROUPS) {
      keys.push(`guide-group-${group.id}`);
      for (const [, entry] of group.entries) keys.push(`guide-entry-${entry}`, `guide-scope-${entry}`);
    }
    for (const key of keys) {
      assert.ok(l.hasMessage(locale, key), `${locale}/${key}`);
      assert.ok(l.format(key).length > 0);
      assert.doesNotMatch(l.format(key), /\[guide-|\[controls-/);
    }
  }
});

test("archive views retain custom names, rank credited kills and filter known uniques without adding candidates", () => {
  const row = (id, kills, unique, alive) => ({ monster: { kindId: id, nameKey: id, glyph: "k", unique }, kills, alive });
  const archive = { objects: [], artifacts: [{ id: "lost", nameKey: "dagger", descriptionKey: "base", customName: "<script>" }], egos: [],
    monsters: [row("ordinary", 5, false, null), row("alive", 0, true, true), row("dead", 1, true, false)],
    dungeons: [{ dungeonId: "visited", nameKey: "warrens", maxDepth: 3, conquered: false }] };
  const before = structuredClone(archive);
  assert.deepEqual(archiveRows(archive, "kills", key => key).map(row => row.id), ["ordinary", "dead"]);
  assert.deepEqual(archiveRows(archive, "uniques", key => key).map(row => row.id), ["dead", "alive"]);
  assert.equal(archiveRows(archive, "artifacts", key => key)[0].name, "<script> · dagger");
  assert.equal(archiveRows(archive, "dungeons", key => key)[0].dungeon.maxDepth, 3);
  assert.deepEqual(archive, before);
});
