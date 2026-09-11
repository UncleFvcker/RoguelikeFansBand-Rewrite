// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { checkApplicability, loadApplicability } from "./generation-applicability.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceRoot = process.argv[2];
if (sourceRoot === "--check-applicability") {
  const reviewed = await checkApplicability(root);
  console.log(`Applicability check passed: ${reviewed.reviews.length} creation builds; ${reviewed.gaps.length} documented evidence gaps. Read-only; gameplay tests were not run.`);
  process.exit(0);
}
assert.ok(sourceRoot, "usage: node scripts/audit-egos.mjs <authoritative RFB repository> | --check-applicability");
const reviewed = await loadApplicability(root);
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
assert.equal(naturalTables.length, 14);
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
  const type = line.match(/^I:(\d+):(\d+):(-?\d+)/);
  if (type) Object.assign(kinds.get(index), { tval: Number(type[1]), sval: Number(type[2]), pval: Number(type[3]) });
}
const equipmentBases = [...new Set(pool.entries.map(entry => entry.itemKindId))].map(id => items.find(item => item.id === id)).filter(item => item.equipmentSlot || item.ammunitionProfile).map(item => {
  if (!item.rfbBaseKind) {
    assert.ok(item.captureBall, `unmapped equipment base ${item.id}`);
    return { itemId: item.id, status: "separate-capture-system", equipmentSlot: item.equipmentSlot };
  }
  const kind = kinds.get(item.rfbBaseKind.sourceIndex);
  assert.ok(kind, `unknown base ${item.id}`);
  assert.deepEqual([item.rfbBaseKind.tval, item.rfbBaseKind.sval], [kind.tval, kind.sval], item.id);
  if (kind.tval === 46 && kind.sval === 1) assert.equal(item.rfbValue.pval, kind.pval, item.id);
  return { itemId: item.id, status: "source-type-verified", ...item.rfbBaseKind };
});
// Reviewed against this master object, never against the source working tree.
// A changed source requires a new review rather than silently reusing line ranges.
assert.equal(source.sourceCommit, reviewed.sourceCommit, "re-review generation condition applicability for the new source commit");
const conditionScopes = reviewed.conditionScopes;
const conditionPattern = /p_ptr->(?:pclass|prace|psubrace|personality|realm1|realm2|good_luck)|\b(?:CLASS_|RACE_|MUT_|PERS_|DEMIGOD_|GIANT_|WARLOCK_|DISCIPLE_|DEVICEMASTER_)|obj_drop_theme|virtue_|(?:prace|personality|demigod|giant|warlock|disciple|devicemaster)_is_|personality_includes_|player_is_|equip_has_slot_type|equip_can_wield_kind/;
const sourceConditions = [];
for (const file of ["src/ego.c", "src/object2.c", "src/artifact.c"]) {
  const lines = execFileSync("git", ["-C", sourceRoot, "show", `${source.sourceCommit}:${file}`], { encoding: "utf8" }).split(/\r?\n/);
  for (const [offset, line] of lines.entries()) {
    if (!conditionPattern.test(line)) continue;
    const scope = conditionScopes.find(scope => scope.file === file && scope.start <= offset + 1 && offset + 1 <= scope.end);
    assert.ok(scope, `unreviewed source condition ${file}:${offset + 1}: ${line.trim()}`);
    sourceConditions.push({ source: `${file}:${offset + 1}`, expression: line.trim(), scope: scope.id });
  }
}
const specialArtifactIndices = [41, 78, 144, 145, 146, 162, 190, 212, 320, 322];
assert.deepEqual(items.filter(item => specialArtifactIndices.includes(item.artifactGeneration?.sourceIndex)).map(item => item.artifactGeneration.sourceIndex).sort((a, b) => a - b), [41, 145, 162, 322], "new identity-sensitive artifact requires applicability implementation/review");
const terror = items.find(item => item.artifactGeneration?.sourceIndex === 41);
assert.equal(terror.artifactGeneration.baseItemKindId, "demo.item.iron-helm");
assert.equal(terror.artifactGeneration.rarityOneIn, 7);
assert.ok(!terror.artifactGeneration.affixIds?.length, "Terror Mask extras must be selected by the generating identity");
assert.equal(items.filter(item => item.artifactGeneration && item.rfbBaseKind?.tval === 19 && item.rfbBaseKind.sval === 70).length, 0, "new fixed harp requires Bard/non-Bard review");
const report = {
  sourceRef: "master", sourceCommit: source.sourceCommit,
  identityContractsVerified: entries.length, craftSelectableCount: 121,
  runtimeRoundTripTest: "game::ego::contracts::all_160_source_egos_have_an_effect_and_save_stable_instances",
  runtimeParityComplete: false,
  currentPlayableSharedGenerationComplete: reviewed.reviews.every(build => build.complete),
  desktopAcceptance: {
    milestone: "E8.8",
    runner: "node web/e2e/tauri.e2e.mjs --ego",
    preparation: "fresh real High Mage Death character export; museum binding retained; shared generator selects representative instances",
    scenarios: ["negative Ego speed penalty and cursed removal", "random artifact name, properties and activation energy", "dragon base resistance plus Ego armor", "dynamic bag extra slots, carried weight and overflow"],
    persistence: "each scenario exports, changes state, restores exact hash and item details, then continues",
    report: "test-results/ego-desktop-report.json",
    limits: "Windows standalone WebDriver automation; natural probability remains covered by core tests; B6 Acquirement evidence reused; no claim of manual play or unavailable identities",
  },
  negativeEquipmentContract: "contract-v313-negative-equipment: ordinary/Ego generation, 1216 independent C cases and 26 curse consumers; E8.5 adds negative random artifacts",
  dragonBaseContract: "contract-v314-dragon-base-equipment: six source bases, 2048 independent C cases, power suppression, Craft and save; E8.5 integrates random artifacts",
  bagContract: "contract-v315-bag-containers: three source bases, 972 independent C cases, final capacity, non-ammunition slot allocation, all four ego consumers and save",
  naturalTablesUsingSharedPolicy: naturalTables.map(table => table.id).sort(),
  buildApplicability: {
    ...reviewed,
    acceptanceRule: "new-game identity must be real; forged identity branch tests are not playable acceptance; listed test paths are references, not results of this audit command",
    sourceConditions,
    vortex: { status: "equipment-template-and-consumer; indirect-base-allocation", entry: "unavailable", directNamedGenerationCondition: false, evidence: ["src/r_vortex.c:764-810 mon_vortex_get_race uses mon_get_equip_template and pseudo_class_idx Warrior", "lib/edit/b_info.txt:980-1010,1101-1115 Vortex3..8 ANY slots", "src/monster.c:13 current_r_idx -> r_info body_idx -> b_info template", "src/equip.c:372 ANY satisfies every slot type except BOW; src/object2.c:3078 therefore halves bow/quiver and ammo category weights for Vortex", "src/equip.c:1622 positive OF_BLOWS is halved for Vortex"], prerequisite: "real evolving body template, innate attack/positive-blows consumer and exact source allocator; no invented direct generator flag", tests: null },
  },
  randomArtifactContract: "E8.5: natural scheduler, complete fresh candidate/value retry, curses, name/RNG state and save",
  jewelryRetryContract: "E8.6: strict level/mode limits; full Ego/randart candidate; unconditional fresh attempt 1001",
  baseAllocationContract: {
    scope: "B0-B6: six playable classes and the imported canonical pool; no claim of all source kinds or unavailable identities",
    plan: "design/base-allocation-acquirement-plan.md",
    rules: "source category weights, good/great/tailored hooks, ordered stochastic theme preparation, OOD and max-depth selection, completed-item rejection; make_object 1/100/1000 and Acquirement outer 1000 with interleaved drop_near",
    bookDiscovery: "persistent per-kind found_count and per-instance book_counted; pickup/identification/player destruction count once; shops and repurchases do not count; save/hash validated",
    entryAcceptance: [
      { entry: "ordinary room and anywhere floor allocation", tests: "game::tests::generation::ordinary_room_and_anywhere_allocations_reach_pickup_and_save" },
      { entry: "vault and monster-carried initialization/death release", tests: "game::world::generation::tests::generated_vault_cells_survive_floor_serialization", limitation: "minimal vault/carried test configuration; no formal actor currently has carriedLootTableId" },
      { entry: "themed monster death, pickup and equipment", tests: "game::tests::tasks::warrior_shoot_monster_death_keeps_theme_through_pickup_equipment_and_save; game::ego::applicability::" },
      { entry: "Acquirement use, count/origin/knowledge/ID, failures and save", tests: "game::tests::acquirement::; game::tests::items::p3_5_acquirement_uses_stable_ids_current_position_and_exact_rng_draws" },
      { entry: "Tailored real weapon, riding, book study/cast and device consumers", tests: "game::tests::items::b4_tailored_" },
      { entry: "explicit base, Craft and configured fixed reward keep their entry contracts", tests: "game::tests::tasks::forced_base_ammunition_damage_dice_survive_generation_and_save; game::tests::items::e6_crafting_uses_shared_weighted_materialization_at_player_level; game::tests::tasks::old_castle_reward_is_forced_even_when_the_artifact_was_generated_before_claim" },
    ],
    sourceCoverage: "packs/rfb-demo-original/legacy-base-allocation-audit.json",
    representationLimits: "B1 physical books remain quantity-one instances; no complete source ordinary obj_make_pile distribution or obj_can_combine origin/notes/cost model",
  },
  unresolvedSharedGenerationContracts: [
    { scope: "current build acceptance evidence", contract: "See buildApplicability.gaps for reachable generation/consumer evidence still needed; source review and test references alone do not close acceptance", source: "design/generation-build-applicability.json" },
    { scope: "unavailable identities and entries", contract: "see buildApplicability.conditionScopes; Mauler/Bard/Monster Ring are examples, not exhaustive", source: "src/ego.c; src/object2.c; src/artifact.c" },
    { scope: "source kind coverage and object representation", contract: "B0-B6 close allocation/retry/consumer acceptance for the prior six playable classes and the imported pool. Unimported canonical kinds remain in legacy-base-allocation-audit.json; physical books retain B1 quantity-one instances; source ordinary pile distributions and full obj_can_combine origin/inscription/discount semantics are not claimed", source: "lib/edit/k_info.txt; src/object2.c; src/obj.c" },
    { scope: "identity-sensitive fixed artifacts", contract: "absent content/entry prerequisites and ordinary identity branches require implementation when imported", source: "src/artifact.c:3254-3406; :3675" },
  ],
  retainedNonSourceAffixes: affixes.filter(affix => !affix.rfbEgo).map(affix => affix.id).sort(),
  equipmentBases, unmappedFlagReview, entries,
};
await writeFile(path.join(root, "design/ego-contract-audit.json"), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ sourceCommit: source.sourceCommit, identityContractsVerified: entries.length, equipmentBasesReviewed: equipmentBases.length, unmappedFlagsReviewed: unmappedFlagReview.length, runtimeParityComplete: false, report: "design/ego-contract-audit.json" }, null, 2));
