// SPDX-License-Identifier: MPL-2.0
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, readdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

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
assert.equal(source.sourceCommit, "a0d92b6378d148c5262cc236b8fa6ed2ca06a54c", "re-review generation condition applicability for the new source commit");
const { PLAYTEST_BUILD_IDS, PLAYTEST_RACE_IDS } = await import(pathToFileURL(path.join(root, "web/src/session-shell.ts")));
const builds = await definitions("builds");
const playableClasses = [...new Set(PLAYTEST_BUILD_IDS.map(id => {
  const build = builds.find(build => build.id === id);
  assert.ok(build, `missing playable build ${id}`);
  return build.classId;
}))].sort();
assert.deepEqual(playableClasses, ["demo.class.archer", "demo.class.cavalry", "demo.class.high-mage", "demo.class.paladin", "demo.class.sniper", "demo.class.warrior"], "new playable class requires source condition review");
const conditionScopes = [
  ["ego-theme", "ego.c", 80, 295, "implemented", "all playable builds; 13 formal monster themes", "filter ring/amulet/armor Ego candidates before weighting; empty themed pool uses unfiltered source pool", "ego.rs; loot.rs; ego/jewelry.rs; equipment/activation/save", "game::ego::applicability::; game::ego::jewelry::generation_tests::", null],
  ["bad-luck-randart", "ego.c", 303, 337, "implemented", "active Bad Luck mutation", "adjust random-artifact acceptance limits", "random_artifact/scheduling.rs", "game::random_artifact::scheduling::tests::", null],
  ["monster-ring-activation", "ego.c", 455, 455, "deferred-unavailable-build", "no Monster Ring creation entry", "activation chance 1/2 instead of 1/5", "jewelry activation generation and actual activation use", null, "real Monster Ring race, equipment/absorption and activation consumers"],
  ["mauler-weight", "ego.c", 1641, 1655, "deferred-unavailable-build", "no Mauler creation entry", "recompute melee weapon weight after dice improvement", "Ego/random artifact instance weight, burden and melee", null, "real Mauler class and heavy-weapon combat; same-input Ego/randart equip/save acceptance"],
  ["bad-luck-tomte-hat", "ego.c", 3910, 3918, "implemented", "active Bad Luck; Tomte or any real race using this hat", "roll continuation die, then stop speed pval growth after first increment", "shared armor materializer; natural/Craft/configured rewards; speed and save", "game::ego::applicability::real_tomte_bad_luck_hat_limits_speed_and_survives_equipping_and_save", null],
  ["berserker-telepathy", "artifact.c", 1320, 1326, "deferred-unavailable-build", "no Berserker creation entry", "telepathy resistance ability chance 10% instead of 90%", "random artifact abilities and senses", null, "real Berserker class and telepathy acceptance"],
  ["bard-slot-value", "artifact.c", 2040, 2047, "deferred-unavailable-build", "no Bard creation entry", "harp slot value percentage 50 instead of 40", "random artifact value acceptance and harp abilities", null, "real Bard class, harp use and same-input value comparison"],
  ["artifact-theme-bias", "artifact.c", 2172, 2210, "implemented", "13 formal monster themes", "set initial random artifact bias from drop theme", "random_artifact.rs; random_artifact/scheduling.rs", "game::random_artifact::", null],
  ["artifact-scroll-class-bias", "artifact.c", 2211, 2325, "deferred-unavailable-entry", "CREATE_ART_SCROLL has no player creation entry", "class/subclass bias on the scroll branch; ordinary natural generation does not run it", "random_artifact.rs initial_bias; future artifact scroll", "factory tests only, not playable class acceptance", "artifact scroll entry; each listed class needs actual new-game configuration"],
  ["artifact-scroll-virtues", "artifact.c", 3160, 3170, "deferred-unavailable-entry", "CREATE_ART_SCROLL has no player creation entry", "Individualism +2 and Enchantment +5", "virtue state/save", null, "artifact scroll success consumer"],
  ["fixed-artifact-identity", "artifact.c", 3254, 3406, "deferred-unavailable-build-or-content", "Dr Jones whip exists with ordinary behavior; Archaeologist unavailable; other nine identity-specific artifacts absent", "Gothmog, Twilight, Stormbringer, Destroyer, Terror Mask, Stone Mask, Muramasa, Dr Jones, Xiaolong, Dragonlance branches", "fixed artifact construction, curses/dice/pval/slays/abilities", null, "import each source artifact and implement both normal and applicable identity branches; Warrior/Cavalry alone do not make absent Terror Mask reachable"],
  ["inspired-smithing", "artifact.c", 3470, 3478, "deferred-unavailable-entry", "mutation defined with randomWeight 0; no reforge entry", "reforge min +1/8 and max +1/12", "reforge value limits", null, "actual mutation grant and reforge entry"],
  ["fixed-harp-bard", "artifact.c", 3673, 3678, "deferred-unavailable-content", "no fixed artifact harp in formal pack; no Bard", "halve pval for non-Bard when creating named harp", "fixed harp charisma and use", null, "fixed harp content and Bard entry"],
  ["politician-gold", "object2.c", 830, 875, "consumer-not-equipment-generation", "no Politician creation entry", "gold setters notify Politician", "politician_check_au", null, "real Politician gold consumer"],
  ["bad-luck-fixed-special", "object2.c", 1605, 1615, "implemented", "active Bad Luck", "each instant fixed artifact attempt reduces reference depth", "loot.rs fixed artifact scheduler", "game::ego::applicability::bad_luck_fixed_artifact_attempts_reduce_reference_depth_and_consume_town_roll", null],
  ["bad-luck-fixed-normal", "object2.c", 1680, 1690, "implemented", "active Bad Luck", "each normal fixed artifact attempt reduces reference depth", "loot.rs fixed artifact scheduler", "game::ego::applicability::bad_luck_fixed_artifact_attempts_reduce_reference_depth_and_consume_town_roll", null],
  ["luck-quality-virtue", "object2.c", 2050, 2105, "implemented", "Good Luck / Bad Luck mutations and Chance virtue on real builds", "local generation level; good/great chances", "mutations.rs; loot.rs; virtue state", "game::tests::mutations::; game::tests::virtue_state::", null],
  ["theme-jewelry-power", "object2.c", 2148, 2155, "implemented", "formal themed monster drops", "ordinary jewelry power 0 becomes +1", "loot.rs -> jewelry candidate/value retry", "game::ego::applicability::; game::ego::jewelry::generation_tests::", null],
  ["good-luck-fixed-retry", "object2.c", 2200, 2208, "implemented", "Good Luck mutation", "1/77 extra fixed artifact attempt", "loot.rs", "game::random_artifact::scheduling::tests::", null],
  ["fixed-harp-pval", "object2.c", 2241, 2248, "deferred-unavailable-content", "no fixed artifact harp", "non-Bard halves artifact harp pval", "fixed artifact constructor", null, "fixed harp import; Bard entry separately"],
  ["base-harp-bard", "object2.c", 2294, 2302, "implemented-current-builds", "all open builds use non-Bard m_bonus(1)", "Bard would use m_bonus(2)", "ego.rs harp intrinsic pval; charisma; launcher exclusion", "game::ego::tests::ordinary_harp_rolls_intrinsic_charisma_and_is_not_a_projectile_launcher", "Bard special path awaits actual class/harp consumer"],
  ["bikini-personality", "object2.c", 2374, 2384, "deferred-unavailable-build", "Sexy personality / Aphrodite demigod unavailable", "bikini adds +3 to all six attributes", "base properties and equipped attributes", null, "real personality/demigod selection and equip/save acceptance"],
  ["tailored-favorite", "object2.c", 2415, 2428, "implemented-current-builds", "Acquirement on six open classes", "Archer rejects melee; Sniper retains it; Cavalry uses source riding weapon flags", "loot/allocation.rs; mounted combat and projectile consumers", "game::loot::allocation::tests::tailored_uses_playable_class_equipment_realms_and_birth_race; game::tests::items::b4_tailored_launchers_equip_shoot_and_restore_for_archer_and_sniper; game::tests::items::b4_tailored_lance_keeps_riding_bonus_in_actual_mounted_combat", "unavailable classes require real entry and favorite-weapon consumer"],
  ["tailored-device-class", "object2.c", 2429, 2453, "implemented-current-builds", "High Mage open; other source device classes unavailable", "High Mage device preference and eligible source device kinds", "loot/allocation.rs; device generation and item use", "game::tests::items::b4_tailored_high_mage_device_is_usable_and_restores_its_charges", "other device classes and Monster pseudo class await real entries"],
  ["tailored-compatible-kinds", "object2.c", 2455, 2570, "implemented-current-builds", "six open classes; real body slots; birth Tomte knit cap", "slot/favorite/book filters; full materialized glove flags and pval reject caster encumbrance", "loot/allocation.rs; mod.rs slot helper; player_abilities.rs; inventory.rs", "game::loot::allocation::tests::; game::tests::items::tomte_tailored_acquirement_filters_headgear_by_birth_race_only; game::tests::items::b4_tailored_glove_egos_share_casting_encumbrance_and_rejection_keeps_rng", "unavailable identities/body forms remain separate; physical books retain the B1 one-instance representation"],
  ["great-book-count", "object2.c", 2634, 2645, "implemented-current-builds", "source Great book selection", "current classes reject advanced books after two discoveries", "loot/allocation.rs; item_knowledge.rs", "game::loot::allocation::tests::good_and_great_distinguish_damaged_bases_ammo_consumables_and_books; game::tests::book_discovery::", "Rage Mage limit 8 awaits real class; book stacking outside B1 representation"],
  ["good-book-count", "object2.c", 2740, 2750, "implemented-current-builds", "source Good book selection", "current classes reject advanced books after two discoveries", "loot/allocation.rs; item_knowledge.rs", "game::loot::allocation::tests::good_and_great_distinguish_damaged_bases_ammo_consumables_and_books; game::tests::book_discovery::", "Rage Mage limit 8 awaits real class; book stacking outside B1 representation"],
  ["ring-allocation-weight", "object2.c", 3068, 3075, "source-condition-unreachable-in-current-source-table", "Monster Ring absent; _kind_alloc_table has separate ring/amulet hooks", "branch compares kind_is_jewelry, which is not an entry hook in that table", "_kind_alloc_weight", null, "do not invent a working +20 weight branch; recheck if source table changes"],
  ["equipment-category-weight", "object2.c", 3076, 3082, "implemented-current-builds", "real body slot templates", "halve category weight when equip_has_slot_type is false; ANY excludes bow for this helper", "loot/allocation.rs; mod.rs item_can_occupy_slot_type", "game::loot::allocation::tests::category_weights_use_great_precedence_and_body_slots_not_equipped_items", "Vortex real evolving body/innate consumers remain unavailable"],
  ["needs-book", "object2.c", 3441, 3493, "implemented-current-builds", "High Mage/Paladin realms", "realm and persistent discoveries decide book preference independently of inventory quantity", "loot/allocation.rs; item_knowledge.rs; study/cast", "game::loot::allocation::tests::tailored_preference_draws_follow_class_then_book_then_device; game::tests::items::b4_tailored_death_books_are_counted_studied_cast_and_restored; game::tests::book_discovery::; game::tests::town::book_discovery_shop_groups_and_repurchase_do_not_count_as_found", "other source classes require real realm/study entries"],
  ["base-theme", "object2.c", 3500, 3558, "implemented-current-pool", "13 formal enum themes share source base pool; WARRIOR_SHOOT mapped; JUNK has a predicate but no source caller or formal table", "theme hook precedes tailored while quality intersection and later tailored checks remain; stochastic predicates precede allocation", "formal lootTables; loot/allocation.rs; monster death -> pickup/equipment/save", "game::loot::allocation::tests::; game::tests::tasks::warrior_shoot_monster_death_keeps_theme_through_pickup_equipment_and_save; game::ego::applicability::", "unimported source kinds remain listed in packs/rfb-demo-original/legacy-base-allocation-audit.json"],
  ["tailored-class-hooks", "object2.c", 3559, 3665, "implemented-current-builds", "Archer/Sniper/Cavalry/High Mage and other open classes", "bows 1/5; lances 1/7; ordered books/devices preference; source fallback category width", "Acquirement -> loot/allocation.rs -> loot.rs -> item use/equip/save", "game::loot::allocation::tests::; game::tests::items::b4_tailored_launchers_equip_shoot_and_restore_for_archer_and_sniper; game::tests::items::b4_tailored_death_books_are_counted_studied_cast_and_restored; game::tests::acquirement::", "unavailable classes, Monster body hooks and Draconian Metamorphosis need real entries"],
  ["karrot-replacement", "object2.c", 3800, 3810, "deferred-unavailable-build", "no Disciple Karrot entry", "replace generated artifact via disciple hook", "karrot_replace_art", null, "real Disciple and artifact replacement consumer"],
  ["theme-reset", "object2.c", 3834, 3842, "implemented", "each drop owns its LootContext", "clear source global theme after generation", "scoped immutable LootContext instead of a mutable source global", "game::ego::applicability::", null],
  ["gold-virtue-personality", "object2.c", 3959, 3979, "outside-equipment-contract", "Sacrifice virtue; Noble personality unavailable", "gold amount scaled by virtue; Noble +25%", "gold generation", null, "separate gold rule audit; no equipment generation modifier"],
  ["book-awareness", "object2.c", 4695, 4715, "consumer-not-equipment-generation", "Sorcerer/Red Mage unavailable", "realm-dependent book awareness", "object awareness/knowledge", null, "real classes and book knowledge consumer"],
].map(([id, file, start, end, status, entry, change, consumer, tests, prerequisite]) => ({ id, file: `src/${file}`, start, end, status, entry, change, consumer, tests, prerequisite }));
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
assert.deepEqual(items.filter(item => specialArtifactIndices.includes(item.artifactGeneration?.sourceIndex)).map(item => item.artifactGeneration.sourceIndex), [162], "new identity-sensitive artifact requires applicability implementation/review");
assert.equal(items.filter(item => item.artifactGeneration && item.rfbBaseKind?.tval === 19 && item.rfbBaseKind.sval === 70).length, 0, "new fixed harp requires Bard/non-Bard review");
for (const unavailable of ["mauler", "bard", "berserker"]) assert.ok(!playableClasses.some(id => id.endsWith(`.${unavailable}`)), `review newly playable ${unavailable}`);
for (const unavailable of ["mon-ring", "mon-vortex"]) assert.ok(!PLAYTEST_RACE_IDS.some(id => id.endsWith(`.${unavailable}`)), `review newly playable ${unavailable}`);
const report = {
  sourceRef: "master", sourceCommit: source.sourceCommit,
  identityContractsVerified: entries.length, craftSelectableCount: 121,
  runtimeRoundTripTest: "game::ego::contracts::all_160_source_egos_have_an_effect_and_save_stable_instances",
  runtimeParityComplete: false,
  currentPlayableSharedGenerationComplete: true,
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
    entrySource: "web/src/session-shell.ts PLAYTEST_BUILD_IDS / PLAYTEST_RACE_IDS",
    playableBuilds: PLAYTEST_BUILD_IDS, playableRaces: PLAYTEST_RACE_IDS, playableClasses,
    acceptanceRule: "new-game identity must be real; forged identity branch tests are not playable acceptance; listed test paths are references, not results of this audit command",
    conditionScopes, sourceConditions,
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
    { scope: "unavailable identities and entries", contract: "see buildApplicability.conditionScopes; Mauler/Bard/Monster Ring are examples, not exhaustive", source: "src/ego.c; src/object2.c; src/artifact.c" },
    { scope: "source kind coverage and object representation", contract: "B0-B6 close allocation/retry/consumer acceptance for current playable builds and the imported pool. Unimported canonical kinds remain in legacy-base-allocation-audit.json; physical books retain B1 quantity-one instances; source ordinary pile distributions and full obj_can_combine origin/inscription/discount semantics are not claimed", source: "lib/edit/k_info.txt; src/object2.c; src/obj.c" },
    { scope: "identity-sensitive fixed artifacts", contract: "absent content/entry prerequisites and ordinary identity branches require implementation when imported", source: "src/artifact.c:3254-3406; :3675" },
  ],
  retainedNonSourceAffixes: affixes.filter(affix => !affix.rfbEgo).map(affix => affix.id).sort(),
  equipmentBases, unmappedFlagReview, entries,
};
await writeFile(path.join(root, "design/ego-contract-audit.json"), `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ sourceCommit: source.sourceCommit, identityContractsVerified: entries.length, equipmentBasesReviewed: equipmentBases.length, unmappedFlagsReviewed: unmappedFlagReview.length, runtimeParityComplete: false, report: "design/ego-contract-audit.json" }, null, 2));
