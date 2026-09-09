// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceRoot = process.argv[2];
assert.ok(sourceRoot, "usage: node scripts/audit-egos.mjs <authoritative RFB repository>");
const source = JSON.parse(execFileSync("cargo", ["run", "-q", "-p", "rfb-legacy-import", "--", "audit-egos", sourceRoot], { cwd: root, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }));
const pack = path.join(root, "packs/rfb-demo-original");
async function definitions(folder) {
  return Promise.all((await readdir(path.join(pack, folder))).filter(name => name.endsWith(".json")).map(async name => JSON.parse(await readFile(path.join(pack, folder, name), "utf8"))));
}
function locale(text) {
  return new Map([...text.matchAll(/^([a-zA-Z0-9-]+) = (.*)$/gm)].map(([, key, value]) => [key, value.trim()]));
}
const [affixes, items, english, chinese, pool] = await Promise.all([
  definitions("affixes"), definitions("items"),
  readFile(path.join(root, "locales/en-US/content.ftl"), "utf8").then(locale),
  readFile(path.join(root, "locales/zh-CN/content.ftl"), "utf8").then(locale),
  readFile(path.join(pack, "lootTables/base-items.json"), "utf8").then(JSON.parse),
]);
const formal = affixes.filter(affix => affix.rfbEgo);
assert.equal(source.recordCount, 160);
assert.equal(formal.length, source.recordCount);
assert.equal(new Set(formal.map(affix => affix.rfbEgo.sourceIndex)).size, source.recordCount);
assert.equal(source.unresolvedChineseNameCount, 0);
assert.equal(pool.rfbEgoPolicy, "weapon-digger");
assert.equal(pool.affixWeights?.length ?? 0, 0, "formal pool must not retain a generic fallback");
const naturalTables = (await definitions("lootTables")).filter(table => table.qualityPolicy?.kind === "rfb-depth");
assert.equal(naturalTables.length, 13);
for (const table of naturalTables) {
  assert.equal(table.rfbEgoPolicy, "weapon-digger", table.id);
  assert.equal(table.affixWeights?.length ?? 0, 0, table.id);
}
const consumers = new Map();
for (const [flags, owner] of [
  ["AGGRAVATE DRAIN_EXP TELEPORT TY_CURSE", "crates/rfb-core/src/game/item_curses.rs"],
  ["CURSED HEAVY_CURSE RANDOM_CURSE2", "crates/rfb-core/src/game/ego.rs; crates/rfb-core/src/game/ego/armor.rs; crates/rfb-core/src/game/ego/jewelry.rs"],
  ["BLESSED BLOWS BRAND_MANA BRAND_ORDER BRAND_WILD VORPAL", "crates/rfb-core/src/game/player_combat.rs; crates/rfb-core/src/game/player_stats.rs"],
  ["ESP_ANIMAL ESP_GIANT ESP_ORC ESP_TROLL ESP_UNDEAD TELEPATHY INFRA SEARCH STEALTH DEC_STEALTH SPEED DEC_SPEED SLOW_DIGEST", "crates/rfb-core/src/game/player_stats.rs"],
  ["DEC_LIFE LIFE", "crates/rfb-core/src/game/player_stats.rs; crates/rfb-core/src/game/progression.rs"],
  ["MAGIC_MASTERY MAGIC_RESISTANCE SPELL_CAP DEVICE_POWER", "crates/rfb-core/src/game/player_abilities.rs; crates/rfb-core/src/game/item_use.rs; crates/rfb-core/src/game/monster_abilities.rs"],
  ["ONE_SUSTAIN XTRA_E_RES XTRA_H_RES XTRA_POWER XTRA_RES", "crates/rfb-core/src/game/ego.rs; crates/rfb-core/src/game/ego/armor.rs"],
  ["TUNNEL", "crates/rfb-core/src/game/mining.rs; crates/rfb-core/src/game/player_stats.rs"],
  ["XTRA_MIGHT XTRA_SHOTS", "crates/rfb-core/src/game/ego.rs; crates/rfb-core/src/game/player_stats.rs"],
]) for (const flag of flags.split(" ")) consumers.set(flag, owner);
const unmappedFlagReview = Object.entries(source.unmappedFlagOccurrences).map(([flag, occurrences]) => {
  if (flag === "AWARE") return { flag, occurrences, classification: "source-declaration-without-runtime-consumer", evidence: "master:src/ego.c; master:lib/edit/e_info.txt" };
  assert.ok(consumers.has(flag), `unreviewed importer flag ${flag}`);
  return { flag, occurrences, classification: "runtime-consumer", evidence: consumers.get(flag) };
});
function materializer(index) {
  if (index === 260) return "crates/rfb-core/src/game/inventory.rs: curse_equipped_item";
  if (index >= 235) return "crates/rfb-core/src/game/ego/noncraft.rs";
  if (index >= 200) return "crates/rfb-core/src/game/ego/jewelry.rs";
  if (index >= 50 && index <= 152) return "crates/rfb-core/src/game/ego/armor.rs";
  return "crates/rfb-core/src/game/ego.rs";
}
const entries = source.entries.map(entry => {
  const affix = formal.find(affix => affix.rfbEgo.sourceIndex === entry.sourceIndex);
  assert.ok(affix, `missing source ${entry.sourceIndex}`);
  assert.equal(affix.generationLevel ?? 0, entry.level, affix.id);
  assert.equal(affix.generationMaxLevel ?? 65535, entry.maxLevel ?? 65535, affix.id);
  assert.equal(affix.rfbEgo.rarity, entry.rarity, affix.id);
  assert.deepEqual([...affix.rfbEgo.types].sort(), entry.types.map(type => type.toLowerCase().replaceAll("_", "-")).sort(), affix.id);
  assert.equal(english.get(affix.nameKey), entry.englishName, affix.id);
  assert.equal(chinese.get(affix.nameKey), entry.chineseName, affix.id);
  assert.equal(affix.rollGroups?.length ?? 0, 0, `${affix.id}: replaced roll recipe`);
  if (entry.hasActivation) assert.ok(affix.deviceGeneration?.activations?.length, `${affix.id}: fixed activation missing`);
  return { sourceIndex: entry.sourceIndex, affixId: affix.id, chineseName: entry.chineseName, types: entry.types, rarity: entry.rarity, level: entry.level, maxLevel: entry.maxLevel, standardSelectable: entry.standardSelectable, craftSelectable: entry.craftType && entry.standardSelectable, materializer: materializer(entry.sourceIndex), fixedActivation: entry.hasActivation, flags: entry.flags, importerUnmappedFlags: entry.unmappedFlags };
});
assert.equal(entries.filter(entry => entry.craftSelectable).length, 121);
assert.deepEqual(entries.filter(entry => !entry.standardSelectable).map(entry => entry.sourceIndex), [103, 210, 211, 260]);

const kinds = new Map();
let index = 0;
for (const line of execFileSync("git", ["-C", sourceRoot, "show", `${source.sourceCommit}:lib/edit/k_info.txt`], { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 }).split(/\r?\n/)) {
  const name = line.match(/^N:([^:]+):(.*)$/);
  if (name) { index = name[1] === "*" ? index + 1 : Number(name[1]); kinds.set(index, { name: name[2] }); }
  const type = line.match(/^I:(\d+):(\d+):/);
  if (type) Object.assign(kinds.get(index), { tval: Number(type[1]), sval: Number(type[2]) });
}
const equipmentBases = [...new Set(pool.entries.map(entry => entry.itemKindId))].map(id => items.find(item => item.id === id)).filter(item => item.equipmentSlot || item.ammunitionProfile).map(item => {
  if (!item.rfbBaseKind) {
    assert.ok(item.equipmentSlot === "container" || item.captureBall, `unmapped equipment base ${item.id}`);
    return { itemId: item.id, status: "separate-container-or-capture-system", equipmentSlot: item.equipmentSlot };
  }
  const kind = kinds.get(item.rfbBaseKind.sourceIndex);
  assert.ok(kind, `unknown base ${item.id}`);
  assert.deepEqual([item.rfbBaseKind.tval, item.rfbBaseKind.sval], [kind.tval, kind.sval], item.id);
  return { itemId: item.id, status: "source-type-verified", ...item.rfbBaseKind };
});
const report = {
  sourceRef: "master", sourceCommit: source.sourceCommit,
  identityContractsVerified: entries.length, craftSelectableCount: 121,
  runtimeRoundTripTest: "game::ego::contracts::all_160_source_egos_have_an_effect_and_save_stable_instances",
  runtimeParityComplete: false,
  negativeEquipmentContract: "contract-v313-negative-equipment: ordinary/Ego generation, 1216 independent C cases and 26 curse consumers; negative random artifacts remain pending",
  naturalTablesUsingSharedPolicy: naturalTables.map(table => table.id).sort(),
  unresolvedSharedGenerationContracts: [
    { scope: "non-ammunition random artifacts", contract: "_check_rand_art / _art_create_random", source: "src/ego.c:303" },
    { scope: "rings and amulets", contract: "value limits and up to 1000 candidate retries (real scoring implemented in E8.1)", source: "src/ego.c:411" },
    { scope: "dragon base kinds", contract: "dragon_resist and its pre-ego suppression roll", source: "src/object2.c:2312" },
    { scope: "bags", contract: "SV_BAG capacity and quiver-ego behavior in the container system", source: "src/ego.c:3691" },
    { scope: "unavailable classes and races", contract: "Mauler, Bard and Monster Ring special generation modifiers", source: "src/ego.c; src/object2.c" },
  ],
  retainedNonSourceAffixes: affixes.filter(affix => !affix.rfbEgo).map(affix => affix.id).sort(),
  equipmentBases, unmappedFlagReview, entries,
};
await writeFile(path.join(root, "design/ego-contract-audit.json"), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ sourceCommit: source.sourceCommit, identityContractsVerified: entries.length, equipmentBasesReviewed: equipmentBases.length, unmappedFlagsReviewed: unmappedFlagReview.length, runtimeParityComplete: false, report: "design/ego-contract-audit.json" }, null, 2));
