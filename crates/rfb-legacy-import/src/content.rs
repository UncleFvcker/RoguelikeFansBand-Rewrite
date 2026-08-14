// SPDX-License-Identifier: MPL-2.0

//! Read-only import of legacy content tables into local
//! rfb-content JSON fragments. Everything expressible is written to a
//! git-ignored output directory; everything else is aggregated into a gap
//! report so rule work can be prioritised from data. No legacy text enters
//! the repository: unit tests use synthetic samples only.

mod mutation_audit;

pub use mutation_audit::{DemoMutationCoverageReport, audit_demo_mutations};

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use serde::{Deserialize, Serialize};

use crate::{LEGACY_CONTENT_REFERENCE, LegacyImportError};

pub const CONTENT_IMPORT_SCHEMA_VERSION: u16 = 2;
const SCHEMA_BASE: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1";
const F_INFO_SOURCE: &str = "lib/edit/f_info.txt";
const R_INFO_SOURCE: &str = "lib/edit/r_info.txt";
const K_INFO_SOURCE: &str = "lib/edit/k_info.txt";
const E_INFO_SOURCE: &str = "lib/edit/e_info.txt";
const A_INFO_SOURCE: &str = "lib/edit/a_info.txt";
const K_NAME_ZH_SOURCE: &str = "src/kind_name_zh.inc";
const B_INFO_SOURCE: &str = "lib/edit/b_info.txt";
const M_INFO_SOURCE: &str = "lib/edit/m_info.txt";
const S_INFO_SOURCE: &str = "lib/edit/s_info.txt";
const W_INFO_SOURCE: &str = "lib/edit/w_info.txt";
const D_INFO_SOURCE: &str = "lib/edit/d_info.txt";
const T_INFO_SOURCE: &str = "lib/edit/t_info.txt";
const T_PREF_SOURCE: &str = "lib/edit/t_pref.txt";
const R_NAME_ZH_SOURCE: &str = "src/monster_name_zh.inc";
const LEGACY_DROP_TABLE_ID: &str = "rfb-legacy.loot-table.monster-drops";
const LEGACY_WARRIOR_DROP_TABLE_ID: &str = "rfb-legacy.loot-table.monster-drops-warrior";
const DEMO_DROP_TABLE_ID: &str = "demo.loot-table.base-items";
const DEMO_WARRIOR_DROP_TABLE_ID: &str = "demo.loot-table.warrior";
const DEMO_ARCHER_DROP_TABLE_ID: &str = "demo.loot-table.archer";
const DEMO_MAGE_DROP_TABLE_ID: &str = "demo.loot-table.mage";
const DEMO_PRIEST_DROP_TABLE_ID: &str = "demo.loot-table.priest";
const DEMO_EVIL_PRIEST_DROP_TABLE_ID: &str = "demo.loot-table.evil-priest";
const DEMO_PALADIN_DROP_TABLE_ID: &str = "demo.loot-table.paladin";
const DEMO_EVIL_PALADIN_DROP_TABLE_ID: &str = "demo.loot-table.evil-paladin";
const DEMO_SAMURAI_DROP_TABLE_ID: &str = "demo.loot-table.samurai";
const DEMO_ROGUE_DROP_TABLE_ID: &str = "demo.loot-table.rogue";
const DEMO_DWARF_DROP_TABLE_ID: &str = "demo.loot-table.dwarf";
const DEMO_NINJA_DROP_TABLE_ID: &str = "demo.loot-table.ninja";
const DEMO_HOBBIT_DROP_TABLE_ID: &str = "demo.loot-table.hobbit";
const DEMO_CORPSE_ITEM_ID: &str = "demo.item.corpse-remains";
const DEMO_SKELETON_ITEM_ID: &str = "demo.item.skeleton-remains";

fn demo_drop_theme_table_id(theme: &str) -> Option<&'static str> {
    match theme {
        "DROP_WARRIOR" => Some(DEMO_WARRIOR_DROP_TABLE_ID),
        "DROP_WARRIOR_SHOOT" => Some(DEMO_ARCHER_DROP_TABLE_ID),
        "DROP_ARCHER" => Some(DEMO_ARCHER_DROP_TABLE_ID),
        "DROP_MAGE" => Some(DEMO_MAGE_DROP_TABLE_ID),
        "DROP_PRIEST" => Some(DEMO_PRIEST_DROP_TABLE_ID),
        "DROP_PRIEST_EVIL" => Some(DEMO_EVIL_PRIEST_DROP_TABLE_ID),
        "DROP_PALADIN" => Some(DEMO_PALADIN_DROP_TABLE_ID),
        "DROP_PALADIN_EVIL" => Some(DEMO_EVIL_PALADIN_DROP_TABLE_ID),
        "DROP_SAMURAI" => Some(DEMO_SAMURAI_DROP_TABLE_ID),
        "DROP_ROGUE" => Some(DEMO_ROGUE_DROP_TABLE_ID),
        "DROP_DWARF" => Some(DEMO_DWARF_DROP_TABLE_ID),
        "DROP_NINJA" => Some(DEMO_NINJA_DROP_TABLE_ID),
        "DROP_HOBBIT" => Some(DEMO_HOBBIT_DROP_TABLE_ID),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoMonsterSelection {
    schema_version: u16,
    #[serde(default)]
    deprecated_replacements: Vec<DemoDeprecatedMonsterReplacement>,
    monsters: Vec<DemoMonsterSelectionEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoDeprecatedMonsterReplacement {
    deprecated_source_index: u32,
    replacement_source_index: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoMonsterSelectionEntry {
    source_index: u32,
    #[serde(default)]
    source_id: Option<String>,
    id: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    omitted_flags: Vec<String>,
    #[serde(default)]
    omitted_spells: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoMonsterAuditReport {
    schema_version: u16,
    source_ref: &'static str,
    source_commit: String,
    minimum_level: u16,
    maximum_level: u16,
    record_count: usize,
    imported_count: usize,
    selected_count: usize,
    direct_count: usize,
    blocked_count: usize,
    excluded_count: usize,
    guardian_count: usize,
    entries: Vec<DemoMonsterAuditEntry>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DemoMonsterAuditEntry {
    source_index: u32,
    source_name: String,
    source_chinese_name: String,
    level: u16,
    imported: bool,
    location_eligible: bool,
    location_restrictions: Vec<String>,
    status: DemoMonsterAuditStatus,
    blockers: Vec<String>,
    suggested_id: String,
    suggested_tags: Vec<String>,
    omitted_flags: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum DemoMonsterAuditStatus {
    Selected,
    Direct,
    Blocked,
    Excluded,
    Guardian,
}

fn demo_monster_location_restrictions(entry: &LegacyMonsterEntry) -> Vec<String> {
    const ORC_CAVE_DUNGEON_INDEX: u16 = 3;

    let mut restrictions = Vec::new();
    let dungeon_indices = entry
        .flags
        .iter()
        .filter_map(|flag| flag.strip_prefix("DUNGEON_")?.parse::<u16>().ok())
        .collect::<BTreeSet<_>>();
    if dungeon_indices.contains(&2) {
        restrictions.push("camelot-only".to_owned());
    }
    if dungeon_indices
        .iter()
        .any(|index| *index != ORC_CAVE_DUNGEON_INDEX && *index != 2)
    {
        restrictions.push("other-dungeon".to_owned());
    }
    for (flag, reason) in [
        ("WILD_ONLY", "wilderness-only"),
        ("WILD_OCEAN", "ocean-only"),
    ] {
        if entry.flags.iter().any(|candidate| candidate == flag) {
            restrictions.push(reason.to_owned());
        }
    }
    restrictions
}

fn demo_monster_audit_status(
    selected: bool,
    source_index: u32,
    location_eligible: bool,
    blocked: bool,
) -> DemoMonsterAuditStatus {
    const OTHROD_SOURCE_INDEX: u32 = 1185;

    if source_index == OTHROD_SOURCE_INDEX {
        DemoMonsterAuditStatus::Guardian
    } else if !location_eligible {
        DemoMonsterAuditStatus::Excluded
    } else if blocked {
        DemoMonsterAuditStatus::Blocked
    } else if selected {
        DemoMonsterAuditStatus::Selected
    } else {
        DemoMonsterAuditStatus::Direct
    }
}

fn demo_monster_audit_omission_is_safe(flag: &str) -> bool {
    matches!(
        flag,
        "ATTR_ANY"
            | "ATTR_CLEAR"
            | "ATTR_MULTI"
            | "ATTR_SEMIRAND"
            | "AUSSIE"
            | "CAN_SPEAK"
            | "CHAR_CLEAR"
            | "CHAR_MULTI"
            | "FEMALE"
            | "MALE"
            | "NASTY_GLYPH"
            | "POS_GAIN_AC"
            | "POS_HOLD_LIFE"
            | "POS_BACKSTAB"
            | "POS_SEE_INVIS"
            | "POS_SUST_CON"
            | "POS_SUST_CHR"
            | "POS_SUST_DEX"
            | "POS_SUST_INT"
            | "POS_SUST_STR"
            | "POS_SUST_WIS"
            | "POS_TELEPATHY"
            | "EGYPTIAN2"
            | "HINDU2"
            | "NORSE2"
            | "OLYMPIAN2"
            | "RES_WALL"
            | "STUPID"
            | "KILL_EXP"
    )
}

impl DemoMonsterSelectionEntry {
    fn expected_source_id(&self) -> &str {
        self.source_id.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemSelection {
    schema_version: u16,
    items: Vec<DemoItemSelectionEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemSelectionEntry {
    source_index: u32,
    #[serde(default)]
    source_id: Option<String>,
    id: String,
}

impl DemoItemSelectionEntry {
    fn expected_source_id(&self) -> &str {
        self.source_id.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemAdaptationLedger {
    schema_version: u16,
    items: Vec<DemoItemAdaptation>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum DemoItemCoverageStatus {
    Active,
    MechanicsReady,
    Blocked,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemAdaptation {
    source_index: u32,
    source_name: String,
    source_id: String,
    item_id: String,
    status: DemoItemCoverageStatus,
    #[serde(default)]
    blocker: Option<String>,
    #[serde(default)]
    adaptation: Option<String>,
    contract: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemPlan {
    schema_version: u16,
    baseline: DemoItemPlanBaseline,
    batches: Vec<DemoItemPlanBatch>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemPlanBaseline {
    source_commit: String,
    source_items_total: usize,
    active_source_items: usize,
    mechanics_ready_source_items: usize,
    blocked_source_items: usize,
    formal_items_total: usize,
    mapped_formal_items: usize,
    original_formal_items: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemPlanBatch {
    id: String,
    families: Vec<DemoItemPlanFamily>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemPlanFamily {
    id: String,
    primary_blockers: Vec<String>,
    items: Vec<DemoItemPlanEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoItemPlanEntry {
    source_index: u32,
    source_name: String,
    source_id: String,
    #[serde(default)]
    secondary_blockers: Vec<String>,
    #[serde(default)]
    completed_requirements: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoWildernessSelection {
    schema_version: u16,
    world_id: String,
    towns: Vec<DemoWildernessLocationSelection>,
    dungeons: Vec<DemoWildernessLocationSelection>,
    town_plans: Vec<DemoWildernessTownPlan>,
    dungeon_plans: Vec<DemoWildernessDungeonPlan>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoWildernessLocationSelection {
    source_index: u32,
    source_name: String,
    id: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoWildernessPosition {
    x: u16,
    y: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoWildernessTownPlan {
    source_index: u32,
    source_name: String,
    id: String,
    position: DemoWildernessPosition,
    source_file: String,
    standard_facilities: Vec<DemoTownFacilityPlan>,
    inn: DemoTownInnPlan,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoTownFacilityPlan {
    symbol: char,
    source_tag: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoTownInnPlan {
    building_index: u16,
    name: String,
    owner_name: String,
    owner_race: String,
    access: String,
    services: Vec<DemoTownInnServicePlan>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoTownInnServicePlan {
    action_index: u16,
    name: String,
    minimum_cost: u32,
    maximum_cost: u32,
    command: char,
    action_id: u16,
    restriction: u16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoWildernessDungeonPlan {
    source_index: u32,
    source_name: String,
    id: String,
    position: DemoWildernessPosition,
    minimum_depth: u16,
    maximum_depth: u16,
    monster_divisor: u16,
    generation_flags: Vec<String>,
    monster_preferences: Vec<String>,
    guardian: DemoDungeonGuardianPlan,
    final_object: DemoDungeonObjectPlan,
    final_ego_source_index: u32,
    substitute_source_index: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoDungeonGuardianPlan {
    source_index: u32,
    source_name: String,
    chinese_name: String,
    level: u16,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DemoDungeonObjectPlan {
    tval: u16,
    sval: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyWildernessLegendEntry {
    symbol: char,
    terrain: u8,
    level: u16,
    town: u32,
    road: bool,
    name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyWildernessLocation {
    name: String,
    x: u16,
    y: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct LegacyDungeonRecord {
    name: String,
    position: Option<(u16, u16)>,
    minimum_depth: Option<u16>,
    maximum_depth: Option<u16>,
    flags: Vec<String>,
    monster_preferences: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LegacyWilderness {
    width: u16,
    height: u16,
    start_x: u16,
    start_y: u16,
    legend: Vec<LegacyWildernessLegendEntry>,
    rows: Vec<String>,
    towns: BTreeMap<u32, LegacyWildernessLocation>,
}

fn content_parse_error(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: impl Into<String>,
    reason: impl Into<String>,
) -> LegacyImportError {
    LegacyImportError::ContentParse {
        content_source: source,
        line,
        field,
        value: value.into(),
        reason: reason.into(),
    }
}

fn required_field<'a>(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&'a str>,
) -> Result<&'a str, LegacyImportError> {
    let value = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| content_parse_error(source, line, field, "", "field is required"))?;
    Ok(value)
}

fn parse_number<T>(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&str>,
) -> Result<T, LegacyImportError>
where
    T: FromStr,
    T::Err: Display,
{
    let value = required_field(source, line, field, value)?;
    value.parse::<T>().map_err(|error| {
        content_parse_error(
            source,
            line,
            field,
            value,
            format!("invalid number: {error}"),
        )
    })
}

fn parse_fields<'a>(
    source: &'static str,
    line: usize,
    record: &'static str,
    value: &'a str,
    expected: usize,
) -> Result<Vec<&'a str>, LegacyImportError> {
    let fields = value.split(':').map(str::trim).collect::<Vec<_>>();
    if fields.len() != expected {
        return Err(content_parse_error(
            source,
            line,
            record,
            value,
            format!("expected {expected} fields, found {}", fields.len()),
        ));
    }
    Ok(fields)
}

fn parse_dice<T>(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&str>,
) -> Result<(T, T), LegacyImportError>
where
    T: FromStr,
    T::Err: Display,
{
    let value = required_field(source, line, field, value)?;
    let (count, sides) = value.split_once('d').ok_or_else(|| {
        content_parse_error(
            source,
            line,
            field,
            value,
            "expected dice in <count>d<sides> form",
        )
    })?;
    Ok((
        parse_number(source, line, field, Some(count))?,
        parse_number(source, line, field, Some(sides))?,
    ))
}

fn parse_damage_or_multiplier<T>(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<(T, T)>, LegacyImportError>
where
    T: FromStr,
    T::Err: Display,
{
    let value = required_field(source, line, field, value)?;
    if let Some(multiplier) = value.strip_prefix('x') {
        let (whole, fraction) = multiplier.split_once('.').ok_or_else(|| {
            content_parse_error(
                source,
                line,
                field,
                value,
                "expected multiplier in x<whole>.<fraction> form",
            )
        })?;
        let _: u32 = parse_number(source, line, field, Some(whole))?;
        let _: u32 = parse_number(source, line, field, Some(fraction))?;
        return Ok(None);
    }
    Ok(Some(parse_dice(source, line, field, Some(value))?))
}

fn parse_launcher_multiplier(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<u16>, LegacyImportError> {
    let value = required_field(source, line, field, value)?;
    let Some(multiplier) = value.strip_prefix('x') else {
        return Ok(None);
    };
    let (whole, fraction) = multiplier.split_once('.').ok_or_else(|| {
        content_parse_error(
            source,
            line,
            field,
            value,
            "expected multiplier in x<whole>.<fraction> form",
        )
    })?;
    if fraction.len() != 2 {
        return Err(content_parse_error(
            source,
            line,
            field,
            value,
            "expected multiplier with two decimal places",
        ));
    }
    let whole = parse_number::<u16>(source, line, field, Some(whole))?;
    let fraction = parse_number::<u16>(source, line, field, Some(fraction))?;
    whole
        .checked_mul(100)
        .and_then(|value| value.checked_add(fraction))
        .map(Some)
        .ok_or_else(|| content_parse_error(source, line, field, value, "multiplier is too large"))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyTerrainEntry {
    pub index: u32,
    pub tag: String,
    pub display_name: Option<String>,
    pub glyph: Option<char>,
    pub flags: Vec<String>,
    pub destroyed_tag: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyBlowEffect {
    pub token: String,
    pub dice: Option<(u16, u16)>,
    pub chance_percent: Option<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyBlow {
    pub method: String,
    pub effects: Vec<LegacyBlowEffect>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyMonsterEntry {
    pub index: u32,
    pub name: String,
    pub glyph: Option<char>,
    pub speed: Option<u16>,
    pub hp_dice: Option<(u32, u32)>,
    pub armor_class: Option<i32>,
    pub level: Option<u16>,
    pub rarity: Option<u32>,
    pub max_level: Option<u16>,
    pub experience: Option<u64>,
    pub evolution_experience: Option<u64>,
    pub evolution_target_index: Option<u32>,
    pub blows: Vec<LegacyBlow>,
    pub auras: Vec<LegacyBlowEffect>,
    pub flags: Vec<String>,
    pub spells: Vec<String>,
    pub drop_theme: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyItemAllocation {
    pub level: u16,
    pub chance: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyItemEntry {
    pub index: u32,
    pub name: String,
    pub glyph: Option<char>,
    pub tval: u16,
    pub sval: u16,
    pub pval: i32,
    pub level: u16,
    pub max_level: u16,
    pub allocations: Vec<LegacyItemAllocation>,
    pub weight_tenths_pound: u16,
    pub base_value: u32,
    pub armor_class: i32,
    pub damage_dice: Option<(u16, u16)>,
    pub launcher_multiplier_percent: Option<u16>,
    pub to_hit: i32,
    pub to_damage: i32,
    pub to_armor: i32,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyEgoEntry {
    pub index: u32,
    pub name: String,
    pub slots: Vec<String>,
    pub level: u16,
    pub max_level: Option<u16>,
    pub max_to_hit: i32,
    pub max_to_damage: i32,
    pub max_to_armor: i32,
    pub max_pval: i32,
    pub flags: Vec<String>,
    pub has_activation: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyArtifactEntry {
    pub index: u32,
    pub name: String,
    pub tval: u16,
    pub sval: u16,
    pub pval: i32,
    pub level: u16,
    pub rarity_one_in: u16,
    pub weight_tenths_pound: u16,
    pub base_value: u32,
    pub armor_class: i32,
    pub damage_dice: Option<(u16, u16)>,
    pub launcher_multiplier_percent: Option<u16>,
    pub to_hit: i32,
    pub to_damage: i32,
    pub to_armor: i32,
    pub flags: Vec<String>,
    pub has_activation: bool,
}

/// One b_info body template: slot type tokens in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyBodyTemplate {
    pub index: u32,
    pub name: String,
    pub slots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyInnatePower {
    pub governing_attribute: String,
    pub minimum_level: u16,
    pub cost: u32,
    pub base_failure_percent: u8,
    pub ability_id: String,
}

/// A race or personality extracted from the legacy C sources. `dynamic`
/// marks blocks whose scalar fields are computed (rank-scaled monster
/// races); those cannot be represented as static content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyCharacterEntry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub stats: [i32; 6],
    pub skills: [i32; 8],
    pub extra_skills: [i32; 8],
    pub life: i32,
    pub base_hp: i32,
    pub exp: i32,
    pub infra: i32,
    pub shop_adjust: i32,
    pub flags: Vec<String>,
    pub hooks: Vec<String>,
    pub dynamic: bool,
    /// Right-hand symbol of `me.calc_bonuses = _fn;` when present, so the
    /// hook body can be mined for its static defensive surface.
    pub calc_bonuses_fn: Option<String>,
    pub get_powers_fn: Option<String>,
    pub abilities: Vec<LegacyInnatePower>,
    /// Damage-type/tier pairs recovered from top-level `res_add` family
    /// statements in the calc_bonuses hook.
    pub resistances: Vec<(String, String)>,
    pub free_act: bool,
    pub see_invisible: bool,
    pub attribute_sustains: Vec<String>,
    pub speed: i32,
}

impl Default for LegacyCharacterEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            description: String::new(),
            stats: [0; 6],
            skills: [0; 8],
            extra_skills: [0; 8],
            life: 100,
            base_hp: 0,
            exp: 100,
            infra: 0,
            shop_adjust: 110,
            flags: Vec::new(),
            hooks: Vec::new(),
            dynamic: false,
            calc_bonuses_fn: None,
            get_powers_fn: None,
            abilities: Vec::new(),
            resistances: Vec::new(),
            free_act: false,
            see_invisible: false,
            attribute_sustains: Vec::new(),
            speed: 0,
        }
    }
}

/// Character-line sources for [`convert_content`], grouped so the importer
/// signature stays within clippy's argument budget.
#[derive(Debug, Default)]
pub struct LegacyCharacterSources {
    pub bodies: Vec<LegacyBodyTemplate>,
    pub races: Vec<LegacyCharacterEntry>,
    pub personalities: Vec<LegacyCharacterEntry>,
    pub classes: Vec<LegacyClassEntry>,
    pub magic_profiles: Vec<LegacyMagicProfile>,
    pub proficiency_profiles: Vec<LegacyProficiencyProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyClassRegistration {
    pub index: u16,
    pub id: String,
    pub function: String,
    pub registered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyClassEntry {
    pub registration: LegacyClassRegistration,
    pub character: LegacyCharacterEntry,
    pub caster_profile: Option<LegacyCasterProfile>,
    pub pet_upkeep_divisor: u16,
    pub source_found: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyCasterProfile {
    pub dynamic: bool,
    pub casting_attribute: String,
    pub minimum_failure_percent: u8,
    pub minimum_level: u16,
    pub max_encumbrance_weight: i32,
    pub weapon_encumbrance_percent: i32,
    pub zero_mana_encumbrance_weight: i32,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyMagicProfile {
    pub class_index: u16,
    pub name_hint: String,
    pub book_type: String,
    pub casting_attribute: String,
    pub extra_flags: u32,
    pub spell_type: i32,
    pub first_spell_level: u16,
    pub spell_weight: i32,
    pub realms: Vec<LegacyRealmProfile>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyRealmProfile {
    pub index: u8,
    pub readable: bool,
    pub spells: Vec<LegacySpellProfile>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LegacySpellProfile {
    pub index: u8,
    pub level: u16,
    pub mana: u16,
    pub failure_percent: u8,
    pub experience: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyProficiencyProfile {
    pub class_index: u16,
    pub weapon_entries: Vec<LegacyWeaponProficiencyEntry>,
    pub skill_entries: Vec<LegacySkillProficiencyEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacyWeaponProficiencyEntry {
    pub weapon_type: u16,
    pub weapon_subtype: u16,
    pub initial_rank: u8,
    pub maximum_rank: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegacySkillProficiencyEntry {
    pub skill_index: u16,
    pub initial: u16,
    pub maximum: u16,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DemoWeaponProficiencyAuditReport {
    pub schema_version: u16,
    pub source_commit: String,
    pub classes_checked: usize,
    pub base_weapons_checked: usize,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentImportReport {
    pub schema_version: u16,
    pub source_commit: String,
    pub terrain_total: usize,
    pub terrain_imported: usize,
    pub terrain_skipped: usize,
    pub monsters_total: usize,
    pub monsters_imported: usize,
    pub monsters_skipped: usize,
    pub monsters_with_unmapped_spells: usize,
    pub monsters_with_casting: usize,
    pub spells_mapped: BTreeMap<String, usize>,
    pub monsters_with_melee_routine: usize,
    pub monsters_with_inexpressible_blows: usize,
    pub unmapped_terrain_flags: BTreeMap<String, usize>,
    pub unmapped_monster_flags: BTreeMap<String, usize>,
    pub unmapped_spells: BTreeMap<String, usize>,
    pub not_applicable_spells: BTreeMap<String, usize>,
    pub unmapped_blow_methods: BTreeMap<String, usize>,
    pub unmapped_blow_effects: BTreeMap<String, usize>,
    pub items_total: usize,
    pub items_imported: usize,
    pub items_skipped: usize,
    pub egos_total: usize,
    pub egos_imported: usize,
    pub artifacts_total: usize,
    pub artifacts_imported: usize,
    pub unmapped_item_flags: BTreeMap<String, usize>,
    pub unmapped_ego_flags: BTreeMap<String, usize>,
    pub unmapped_artifact_flags: BTreeMap<String, usize>,
    pub not_applicable_item_flags: BTreeMap<String, usize>,
    pub bodies_total: usize,
    pub races_total: usize,
    pub races_imported: usize,
    pub personalities_total: usize,
    pub personalities_imported: usize,
    pub classes_total: usize,
    pub classes_imported: usize,
    pub classes_registered: usize,
    pub classes_with_source: usize,
    pub magic_profiles_total: usize,
    pub classes_with_casting_shells: usize,
    pub class_caster_profiles_imported: usize,
    pub class_caster_profiles_dynamic: usize,
    pub classes_with_readable_realms: usize,
    pub magic_realm_rows: usize,
    pub magic_readable_realm_rows: usize,
    pub magic_spell_profile_rows: usize,
    pub realm_readability: BTreeMap<String, usize>,
    pub player_abilities_imported: usize,
    pub player_ability_books_imported: usize,
    pub classes_with_runtime_casting_profiles: usize,
    pub player_spell_parameter_overrides: usize,
    pub player_spell_mapped_rows: usize,
    pub player_spell_effect_gaps: BTreeMap<String, usize>,
    pub player_spell_behavior_gaps: BTreeMap<String, usize>,
    pub casting_attribute_gaps: BTreeMap<String, usize>,
    pub class_magic_gaps: BTreeMap<String, usize>,
    pub proficiency_profiles_total: usize,
    pub proficiency_weapon_rows: usize,
    pub proficiency_skill_rows: usize,
    pub class_proficiency_gaps: BTreeMap<String, usize>,
    pub unmapped_race_flags: BTreeMap<String, usize>,
    pub unmapped_class_flags: BTreeMap<String, usize>,
    pub race_hook_gaps: BTreeMap<String, usize>,
    pub class_hook_gaps: BTreeMap<String, usize>,
    pub body_slot_gaps: BTreeMap<String, usize>,
    pub item_behavior_gaps: BTreeMap<String, usize>,
    pub skip_reasons: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoItemCoverageReport {
    pub schema_version: u16,
    pub source_commit: String,
    pub source_items_total: usize,
    pub active_source_items: usize,
    pub mechanics_ready_source_items: usize,
    pub blocked_source_items: usize,
    pub formal_items_total: usize,
    pub mapped_formal_items: usize,
    pub original_formal_items: usize,
    pub blocker_counts: BTreeMap<String, usize>,
    pub mechanics_ready: Vec<DemoItemCoverageEntry>,
    pub blocked: Vec<DemoItemCoverageEntry>,
    pub original_item_ids: Vec<String>,
    pub p3_plan: DemoItemPlanProgress,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoItemCoverageEntry {
    pub source_index: u32,
    pub source_name: String,
    pub source_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoItemPlanProgress {
    pub baseline_source_commit: String,
    pub baseline_matches_current: bool,
    pub formal_items_delta: i64,
    pub mapped_rfb_formal_items_delta: i64,
    pub original_formal_items_delta: i64,
    pub active_requirements: Vec<String>,
    pub planned_source_items: usize,
    pub batches: Vec<DemoItemPlanBatchProgress>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoItemPlanBatchProgress {
    pub id: String,
    pub rule_families: Vec<String>,
    pub planned_source_items: usize,
    pub new_rfb_formal_items: usize,
    pub new_rfb_formal_item_ids: Vec<String>,
    pub blocked_to_active: Vec<DemoItemPlanProgressEntry>,
    pub blocked_to_mechanics_ready: Vec<DemoItemPlanProgressEntry>,
    pub still_blocked: Vec<DemoItemPlanProgressEntry>,
    pub unresolved_secondary_blockers: BTreeMap<String, usize>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DemoItemPlanProgressEntry {
    pub source_index: u32,
    pub source_name: String,
    pub source_id: String,
    pub rule_family: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub blockers: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unresolved_secondary_blockers: Vec<String>,
}

fn kebab(raw: &str) -> String {
    let mut id = String::with_capacity(raw.len());
    let mut last_dash = true;
    for ch in raw.chars() {
        let mapped = match ch {
            'A'..='Z' => Some(ch.to_ascii_lowercase()),
            'a'..='z' | '0'..='9' => Some(ch),
            _ => None,
        };
        match mapped {
            Some(ch) => {
                id.push(ch);
                last_dash = false;
            }
            None if !last_dash => {
                id.push('-');
                last_dash = true;
            }
            None => {}
        }
    }
    while id.ends_with('-') {
        id.pop();
    }
    id
}

fn resolve_legacy_content_commit(source: &Path) -> Result<String, LegacyImportError> {
    let resolved = Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("rev-parse")
        .arg(format!("{LEGACY_CONTENT_REFERENCE}^{{commit}}"))
        .output()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if !resolved.status.success() {
        return Err(LegacyImportError::LegacyGit(
            String::from_utf8_lossy(&resolved.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&resolved.stdout).trim().to_owned())
}

/// Reads one path from the authoritative legacy master ref via Git objects,
/// never the checked-out branch or working tree.
pub fn read_legacy_object(source: &Path, path: &str) -> Result<String, LegacyImportError> {
    let commit = resolve_legacy_content_commit(source)?;
    read_legacy_object_at(source, &commit, path)
}

fn read_legacy_object_at(
    source: &Path,
    commit: &str,
    path: &str,
) -> Result<String, LegacyImportError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("show")
        .arg(format!("{commit}:{path}"))
        .output()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyImportError::LegacyGit(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    String::from_utf8(output.stdout).map_err(|error| LegacyImportError::ContentEncoding {
        path: path.to_owned(),
        error: error.to_string(),
    })
}

fn parse_wilderness_legend(
    line_number: usize,
    record: &'static str,
    value: &str,
) -> Result<LegacyWildernessLegendEntry, LegacyImportError> {
    let fields = value.split(':').map(str::trim).collect::<Vec<_>>();
    let valid_length = if record == "W:E" {
        fields.len() == 6
    } else {
        (2..=5).contains(&fields.len())
    };
    if !valid_length {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            line_number,
            record,
            value,
            if record == "W:E" {
                "expected 6 fields".to_owned()
            } else {
                format!("expected 2 to 5 fields, found {}", fields.len())
            },
        ));
    }
    let symbol = required_field(
        W_INFO_SOURCE,
        line_number,
        "symbol",
        fields.first().copied(),
    )?;
    let mut symbols = symbol.chars();
    let Some(symbol) = symbols.next() else {
        unreachable!("required field cannot be empty");
    };
    if symbols.next().is_some() || !symbol.is_ascii() || symbol.is_ascii_control() {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            line_number,
            "symbol",
            symbol.to_string(),
            "expected one printable ASCII symbol",
        ));
    }
    let terrain = parse_number::<u8>(
        W_INFO_SOURCE,
        line_number,
        "terrain",
        fields.get(1).copied(),
    )?;
    if terrain > 14 {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            line_number,
            "terrain",
            terrain.to_string(),
            "expected an RFB wilderness terrain index from 0 through 14",
        ));
    }
    let level = fields
        .get(2)
        .map(|value| parse_number(W_INFO_SOURCE, line_number, "level", Some(value)))
        .transpose()?
        .unwrap_or(0);
    let town = fields
        .get(3)
        .map(|value| parse_number(W_INFO_SOURCE, line_number, "town", Some(value)))
        .transpose()?
        .unwrap_or(0);
    let road = fields
        .get(4)
        .map(|value| parse_number::<u8>(W_INFO_SOURCE, line_number, "road", Some(value)))
        .transpose()?
        .unwrap_or(0);
    if road > 1 {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            line_number,
            "road",
            road.to_string(),
            "expected 0 or 1",
        ));
    }
    let name = fields
        .get(5)
        .map(|value| required_field(W_INFO_SOURCE, line_number, "name", Some(value)))
        .transpose()?
        .map(str::to_owned);
    if (record == "W:E") != (town != 0 && name.is_some()) {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            line_number,
            record,
            value,
            "town entries require a non-zero town index and name; feature entries require neither",
        ));
    }
    Ok(LegacyWildernessLegendEntry {
        symbol,
        terrain,
        level,
        town,
        road: road == 1,
        name,
    })
}

fn parse_w_info(text: &str) -> Result<LegacyWilderness, LegacyImportError> {
    let mut normal = false;
    let mut found_normal = false;
    let mut legend = Vec::new();
    let mut symbols = BTreeSet::new();
    let mut rows = Vec::new();
    let mut start = None;
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim_end();
        if line == "?:[EQU $WILDERNESS NORMAL]" {
            if found_normal {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    line_number,
                    "section",
                    line,
                    "duplicate normal wilderness section",
                ));
            }
            normal = true;
            found_normal = true;
            continue;
        }
        if normal && line == "?:[EQU $WILDERNESS NONE]" {
            break;
        }
        if !normal || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(value) = line.strip_prefix("W:F:") {
            let entry = parse_wilderness_legend(line_number, "W:F", value)?;
            if !symbols.insert(entry.symbol) {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    line_number,
                    "symbol",
                    entry.symbol.to_string(),
                    "duplicate wilderness symbol",
                ));
            }
            legend.push(entry);
        } else if let Some(value) = line.strip_prefix("W:E:") {
            let entry = parse_wilderness_legend(line_number, "W:E", value)?;
            if !symbols.insert(entry.symbol) {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    line_number,
                    "symbol",
                    entry.symbol.to_string(),
                    "duplicate wilderness symbol",
                ));
            }
            legend.push(entry);
        } else if let Some(row) = line.strip_prefix("W:D:") {
            rows.push(row.to_owned());
        } else if let Some(value) = line.strip_prefix("W:P:") {
            if start.is_some() {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    line_number,
                    "W:P",
                    value,
                    "duplicate start position",
                ));
            }
            let fields = parse_fields(W_INFO_SOURCE, line_number, "W:P", value, 2)?;
            let y = parse_number(W_INFO_SOURCE, line_number, "W:P.y", fields.first().copied())?;
            let x = parse_number(W_INFO_SOURCE, line_number, "W:P.x", fields.get(1).copied())?;
            start = Some((x, y));
        } else if line.starts_with("W:") {
            return Err(content_parse_error(
                W_INFO_SOURCE,
                line_number,
                "directive",
                line,
                "unsupported normal wilderness directive",
            ));
        }
    }
    if !found_normal || legend.is_empty() || rows.is_empty() {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            0,
            "section",
            "",
            "normal wilderness section, legend, and rows are required",
        ));
    }
    let height = u16::try_from(rows.len()).map_err(|_| {
        content_parse_error(
            W_INFO_SOURCE,
            0,
            "height",
            rows.len().to_string(),
            "height exceeds u16",
        )
    })?;
    let width_count = rows[0].chars().count();
    let width = u16::try_from(width_count).map_err(|_| {
        content_parse_error(
            W_INFO_SOURCE,
            0,
            "width",
            width_count.to_string(),
            "width exceeds u16",
        )
    })?;
    if width < 3 || height < 3 || width > 512 || height > 512 {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            0,
            "dimensions",
            format!("{width}x{height}"),
            "expected dimensions from 3 through 512",
        ));
    }
    let legend_by_symbol = legend
        .iter()
        .map(|entry| (entry.symbol, entry))
        .collect::<BTreeMap<_, _>>();
    for (y, row) in rows.iter().enumerate() {
        let cells = row.chars().collect::<Vec<_>>();
        if cells.len() != usize::from(width) {
            return Err(content_parse_error(
                W_INFO_SOURCE,
                y + 1,
                "W:D",
                row,
                format!("expected row width {width}, found {}", cells.len()),
            ));
        }
        for (x, symbol) in cells.iter().enumerate() {
            let Some(entry) = legend_by_symbol.get(symbol) else {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    y + 1,
                    "W:D",
                    symbol.to_string(),
                    "row uses a symbol absent from the legend",
                ));
            };
            if (x == 0 || y == 0 || x + 1 == usize::from(width) || y + 1 == usize::from(height))
                && entry.terrain != 0
            {
                return Err(content_parse_error(
                    W_INFO_SOURCE,
                    y + 1,
                    "boundary",
                    symbol.to_string(),
                    "boundary cells must use edge terrain 0",
                ));
            }
        }
    }
    let (start_x, start_y) = start.ok_or_else(|| {
        content_parse_error(W_INFO_SOURCE, 0, "W:P", "", "start position is required")
    })?;
    if start_x >= width || start_y >= height {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            0,
            "W:P",
            format!("{start_y}:{start_x}"),
            "start position is outside the wilderness",
        ));
    }
    let start_symbol = rows[usize::from(start_y)]
        .chars()
        .nth(usize::from(start_x))
        .expect("validated row width must contain the start cell");
    if legend_by_symbol[&start_symbol].terrain == 0 {
        return Err(content_parse_error(
            W_INFO_SOURCE,
            0,
            "W:P",
            format!("{start_y}:{start_x}"),
            "start position cannot use edge terrain",
        ));
    }

    let mut towns = BTreeMap::new();
    for entry in legend.iter().filter(|entry| entry.town != 0) {
        let positions = rows
            .iter()
            .enumerate()
            .flat_map(|(y, row)| {
                row.chars()
                    .enumerate()
                    .filter(move |(_, symbol)| *symbol == entry.symbol)
                    .map(move |(x, _)| (x, y))
            })
            .collect::<Vec<_>>();
        if positions.len() != 1 || towns.contains_key(&entry.town) {
            return Err(content_parse_error(
                W_INFO_SOURCE,
                0,
                "town",
                entry.town.to_string(),
                "town indexes must be unique and occur exactly once",
            ));
        }
        let (x, y) = positions[0];
        towns.insert(
            entry.town,
            LegacyWildernessLocation {
                name: entry
                    .name
                    .clone()
                    .expect("town legend entry must have a name"),
                x: u16::try_from(x).expect("validated width fits u16"),
                y: u16::try_from(y).expect("validated height fits u16"),
            },
        );
    }
    Ok(LegacyWilderness {
        width,
        height,
        start_x,
        start_y,
        legend,
        rows,
        towns,
    })
}

fn parse_dungeon_records(
    text: &str,
) -> Result<BTreeMap<u32, LegacyDungeonRecord>, LegacyImportError> {
    let mut records = BTreeMap::<u32, LegacyDungeonRecord>::new();
    let mut current = None;
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim_end();
        if let Some(value) = line.strip_prefix("N:") {
            let mut fields = value.splitn(2, ':');
            let index = parse_number(D_INFO_SOURCE, line_number, "N.index", fields.next())?;
            let name = required_field(D_INFO_SOURCE, line_number, "N.name", fields.next())?;
            if records
                .insert(
                    index,
                    LegacyDungeonRecord {
                        name: name.to_owned(),
                        ..LegacyDungeonRecord::default()
                    },
                )
                .is_some()
            {
                return Err(content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "N.index",
                    index.to_string(),
                    "duplicate dungeon index",
                ));
            }
            current = Some(index);
        } else if let Some(value) = line.strip_prefix("P:") {
            let index = current.ok_or_else(|| {
                content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "P",
                    value,
                    "position appears before a dungeon record",
                )
            })?;
            let fields = parse_fields(D_INFO_SOURCE, line_number, "P", value, 2)?;
            let y = parse_number(D_INFO_SOURCE, line_number, "P.y", fields.first().copied())?;
            let x = parse_number(D_INFO_SOURCE, line_number, "P.x", fields.get(1).copied())?;
            let record = records
                .get_mut(&index)
                .expect("current dungeon record must exist");
            if record.position.replace((x, y)).is_some() {
                return Err(content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "P",
                    value,
                    "duplicate dungeon position",
                ));
            }
        } else if let Some(value) = line.strip_prefix("W:") {
            let index = current.ok_or_else(|| {
                content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "W",
                    value,
                    "depth appears before a dungeon record",
                )
            })?;
            let fields = value.split(':').collect::<Vec<_>>();
            if fields.len() < 2 {
                return Err(content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "W",
                    value,
                    "expected at least minimum and maximum depth",
                ));
            }
            let record = records
                .get_mut(&index)
                .expect("current dungeon record must exist");
            if record.minimum_depth.is_some() || record.maximum_depth.is_some() {
                return Err(content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "W",
                    value,
                    "duplicate dungeon depth",
                ));
            }
            record.minimum_depth = Some(parse_number(
                D_INFO_SOURCE,
                line_number,
                "W.minimumDepth",
                fields.first().copied(),
            )?);
            record.maximum_depth = Some(parse_number(
                D_INFO_SOURCE,
                line_number,
                "W.maximumDepth",
                fields.get(1).copied(),
            )?);
        } else if let Some(value) = line.strip_prefix("F:") {
            let index = current.ok_or_else(|| {
                content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "F",
                    value,
                    "flag appears before a dungeon record",
                )
            })?;
            records
                .get_mut(&index)
                .expect("current dungeon record must exist")
                .flags
                .extend(
                    value
                        .split('|')
                        .map(str::trim)
                        .filter(|flag| !flag.is_empty())
                        .map(str::to_owned),
                );
        } else if let Some(value) = line.strip_prefix("M:") {
            let index = current.ok_or_else(|| {
                content_parse_error(
                    D_INFO_SOURCE,
                    line_number,
                    "M",
                    value,
                    "monster preference appears before a dungeon record",
                )
            })?;
            records
                .get_mut(&index)
                .expect("current dungeon record must exist")
                .monster_preferences
                .extend(
                    value
                        .split('|')
                        .map(str::trim)
                        .filter(|flag| !flag.is_empty())
                        .map(str::to_owned),
                );
        }
    }
    Ok(records)
}

fn dungeon_location(record: &LegacyDungeonRecord) -> Option<LegacyWildernessLocation> {
    record.position.map(|(x, y)| LegacyWildernessLocation {
        name: record.name.clone(),
        x,
        y,
    })
}

fn list_legacy_c_sources(source: &Path, commit: &str) -> Result<Vec<String>, LegacyImportError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args(["ls-tree", "-r", "--name-only", commit, "--", "src"])
        .output()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if !output.status.success() {
        return Err(LegacyImportError::LegacyGit(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let mut paths = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|path| path.ends_with(".c"))
        .map(str::to_owned)
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

pub fn parse_f_info(text: &str) -> Result<Vec<LegacyTerrainEntry>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyTerrainEntry> = None;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parse_number(F_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let tag = required_field(F_INFO_SOURCE, line_number, "N.tag", parts.next())?.to_owned();
            current = Some(LegacyTerrainEntry {
                index,
                tag,
                ..LegacyTerrainEntry::default()
            });
            continue;
        }
        let recognized = ["E:", "G:", "F:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let entry = match current.as_mut() {
            Some(entry) => entry,
            None if recognized => {
                return Err(content_parse_error(
                    F_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("E:") {
            entry.display_name = Some(
                required_field(F_INFO_SOURCE, line_number, "E.displayName", Some(rest))?.to_owned(),
            );
        } else if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = Some(
                required_field(F_INFO_SOURCE, line_number, "G.glyph", Some(rest))?
                    .chars()
                    .next()
                    .expect("required glyph must contain a character"),
            );
        } else if let Some(rest) = line.strip_prefix("K:DESTROYED:") {
            entry.destroyed_tag = Some(
                required_field(F_INFO_SOURCE, line_number, "K.destroyed", Some(rest))?.to_owned(),
            );
        } else if let Some(rest) = line.strip_prefix("F:") {
            entry.flags.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

fn parse_blow(rest: &str, line_number: usize) -> Result<LegacyBlow, LegacyImportError> {
    let mut blow = LegacyBlow::default();
    for (ordinal, part) in rest.split(':').map(str::trim).enumerate() {
        if ordinal == 0 {
            blow.method =
                required_field(R_INFO_SOURCE, line_number, "B.method", Some(part))?.to_owned();
            continue;
        }
        if part.is_empty() {
            continue;
        }
        let (token, parameters) = if let Some((token, parameters)) = part.split_once('(') {
            let parameters = parameters.strip_suffix(')').ok_or_else(|| {
                content_parse_error(
                    R_INFO_SOURCE,
                    line_number,
                    "B.effect",
                    part,
                    "effect parameters are missing a closing parenthesis",
                )
            })?;
            (token, Some(parameters))
        } else {
            (part, None)
        };
        let token = required_field(R_INFO_SOURCE, line_number, "B.effect", Some(token))?;
        let mut effect = LegacyBlowEffect {
            token: token.to_owned(),
            ..LegacyBlowEffect::default()
        };
        if let Some(parameters) = parameters {
            for parameter in parameters.split(',').map(str::trim) {
                if let Some(percent) = parameter.strip_suffix('%') {
                    let chance: u8 = parse_number(
                        R_INFO_SOURCE,
                        line_number,
                        "B.effectChancePercent",
                        Some(percent),
                    )?;
                    effect.chance_percent = Some(chance.min(100));
                } else {
                    effect.dice = Some(parse_dice(
                        R_INFO_SOURCE,
                        line_number,
                        "B.effectDice",
                        Some(parameter),
                    )?);
                }
            }
        }
        blow.effects.push(effect);
    }
    if blow.method.is_empty() {
        return Err(content_parse_error(
            R_INFO_SOURCE,
            line_number,
            "B.method",
            "",
            "field is required",
        ));
    }
    Ok(blow)
}

pub fn parse_r_info(text: &str) -> Result<Vec<LegacyMonsterEntry>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyMonsterEntry> = None;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parse_number(R_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let name =
                required_field(R_INFO_SOURCE, line_number, "N.name", parts.next())?.to_owned();
            current = Some(LegacyMonsterEntry {
                index,
                name,
                ..LegacyMonsterEntry::default()
            });
            continue;
        }
        let recognized = ["G:", "I:", "W:", "B:", "A:", "F:", "S:", "O:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let entry = match current.as_mut() {
            Some(entry) => entry,
            None if recognized => {
                return Err(content_parse_error(
                    R_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = Some(
                required_field(R_INFO_SOURCE, line_number, "G.glyph", Some(rest))?
                    .chars()
                    .next()
                    .expect("required glyph must contain a character"),
            );
        } else if let Some(rest) = line.strip_prefix("I:") {
            // I:speed:HDdHS:aaf:ac:sleep:weight (init1.c sscanf order).
            let parts = parse_fields(R_INFO_SOURCE, line_number, "I", rest, 6)?;
            entry.speed = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "I.speed",
                parts.first().copied(),
            )?);
            entry.hp_dice = Some(parse_dice(
                R_INFO_SOURCE,
                line_number,
                "I.hitPoints",
                parts.get(1).copied(),
            )?);
            let _: i64 = parse_number(
                R_INFO_SOURCE,
                line_number,
                "I.awareness",
                parts.get(2).copied(),
            )?;
            entry.armor_class = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "I.armorClass",
                parts.get(3).copied(),
            )?);
            let _: i64 =
                parse_number(R_INFO_SOURCE, line_number, "I.sleep", parts.get(4).copied())?;
            let _: i64 = parse_number(
                R_INFO_SOURCE,
                line_number,
                "I.weight",
                parts.get(5).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("W:") {
            let parts = parse_fields(R_INFO_SOURCE, line_number, "W", rest, 6)?;
            entry.level = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.level",
                parts.first().copied(),
            )?);
            entry.rarity = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.rarity",
                parts.get(1).copied(),
            )?);
            entry.max_level = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.maxLevel",
                parts.get(2).copied(),
            )?);
            entry.experience = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.experience",
                parts.get(3).copied(),
            )?);
            entry.evolution_experience = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.evolution",
                parts.get(4).copied(),
            )?);
            entry.evolution_target_index = Some(parse_number(
                R_INFO_SOURCE,
                line_number,
                "W.nextEvolution",
                parts.get(5).copied(),
            )?);
        } else if let Some(rest) = line.strip_prefix("B:") {
            entry.blows.push(parse_blow(rest, line_number)?);
        } else if let Some(rest) = line.strip_prefix("A:") {
            entry
                .auras
                .extend(parse_blow(&format!("AURA:{rest}"), line_number)?.effects);
        } else if let Some(rest) = line.strip_prefix("F:") {
            entry.flags.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(rest) = line.strip_prefix("S:") {
            entry.spells.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|spell| !spell.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(rest) = line.strip_prefix("O:") {
            entry.drop_theme = Some(
                required_field(R_INFO_SOURCE, line_number, "O.dropTheme", Some(rest))?.to_owned(),
            );
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

const MAPPED_TERRAIN_FLAGS: [&str; 6] =
    ["MOVE", "LOS", "PROJECT", "PERMANENT", "HURT_DISI", "GLYPH"];

/// Parses k_info entries; `&` article and `~` plural markers strip out of
/// names, and the `N:*:` auto-index form continues the running counter.
pub fn parse_k_info(text: &str) -> Result<Vec<LegacyItemEntry>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyItemEntry> = None;
    let mut next_index = 0_u32;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let raw_index = required_field(K_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let index = if raw_index == "*" {
                next_index
            } else {
                parse_number(K_INFO_SOURCE, line_number, "N.index", Some(raw_index))?
            };
            next_index = index.checked_add(1).ok_or_else(|| {
                content_parse_error(
                    K_INFO_SOURCE,
                    line_number,
                    "N.index",
                    raw_index,
                    "item index cannot be incremented",
                )
            })?;
            let name = required_field(K_INFO_SOURCE, line_number, "N.name", parts.next())?
                .replace(['&', '~'], " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            current = Some(LegacyItemEntry {
                index,
                name,
                ..LegacyItemEntry::default()
            });
            continue;
        }
        let recognized = ["G:", "I:", "W:", "A:", "P:", "F:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let entry = match current.as_mut() {
            Some(entry) => entry,
            None if recognized => {
                return Err(content_parse_error(
                    K_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = Some(
                required_field(K_INFO_SOURCE, line_number, "G.glyph", Some(rest))?
                    .chars()
                    .next()
                    .expect("required glyph must contain a character"),
            );
        } else if let Some(rest) = line.strip_prefix("I:") {
            let parts = parse_fields(K_INFO_SOURCE, line_number, "I", rest, 3)?;
            entry.tval =
                parse_number(K_INFO_SOURCE, line_number, "I.tval", parts.first().copied())?;
            entry.sval = parse_number(K_INFO_SOURCE, line_number, "I.sval", parts.get(1).copied())?;
            entry.pval = parse_number(K_INFO_SOURCE, line_number, "I.pval", parts.get(2).copied())?;
        } else if let Some(rest) = line.strip_prefix("W:") {
            // level:extra:max_level:weight:cost per the pinned init1.c.
            let parts = parse_fields(K_INFO_SOURCE, line_number, "W", rest, 5)?;
            entry.level = parse_number(
                K_INFO_SOURCE,
                line_number,
                "W.level",
                parts.first().copied(),
            )?;
            let _: i64 =
                parse_number(K_INFO_SOURCE, line_number, "W.extra", parts.get(1).copied())?;
            entry.max_level = parse_number(
                K_INFO_SOURCE,
                line_number,
                "W.maximumLevel",
                parts.get(2).copied(),
            )?;
            entry.weight_tenths_pound = parse_number(
                K_INFO_SOURCE,
                line_number,
                "W.weight",
                parts.get(3).copied(),
            )?;
            entry.base_value =
                parse_number(K_INFO_SOURCE, line_number, "W.cost", parts.get(4).copied())?;
        } else if let Some(rest) = line.strip_prefix("A:") {
            for pair in rest.split(':') {
                let (level, chance) = pair
                    .split_once('/')
                    .map_or((pair, None), |(level, chance)| (level, Some(chance)));
                let level = parse_number(K_INFO_SOURCE, line_number, "A.level", Some(level))?;
                let chance = chance.map_or(Ok(1), |chance| {
                    parse_number::<u32>(K_INFO_SOURCE, line_number, "A.chance", Some(chance))
                        .map(|chance| chance.max(1))
                })?;
                entry
                    .allocations
                    .push(LegacyItemAllocation { level, chance });
            }
        } else if let Some(rest) = line.strip_prefix("P:") {
            let parts = parse_fields(K_INFO_SOURCE, line_number, "P", rest, 5)?;
            entry.armor_class = parse_number(
                K_INFO_SOURCE,
                line_number,
                "P.armorClass",
                parts.first().copied(),
            )?;
            entry.damage_dice = parse_damage_or_multiplier(
                K_INFO_SOURCE,
                line_number,
                "P.damage",
                parts.get(1).copied(),
            )?;
            entry.launcher_multiplier_percent = parse_launcher_multiplier(
                K_INFO_SOURCE,
                line_number,
                "P.damage",
                parts.get(1).copied(),
            )?;
            entry.to_hit =
                parse_number(K_INFO_SOURCE, line_number, "P.toHit", parts.get(2).copied())?;
            entry.to_damage = parse_number(
                K_INFO_SOURCE,
                line_number,
                "P.toDamage",
                parts.get(3).copied(),
            )?;
            entry.to_armor = parse_number(
                K_INFO_SOURCE,
                line_number,
                "P.toArmor",
                parts.get(4).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("F:") {
            entry.flags.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                    .map(str::to_owned),
            );
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

/// Attribute flags carry their bonus in pval; everything else stays in the
/// gap report.
const ITEM_ATTRIBUTE_FLAGS: [(&str, &str); 6] = [
    ("STR", "strength"),
    ("INT", "intelligence"),
    ("WIS", "wisdom"),
    ("DEX", "dexterity"),
    ("CON", "constitution"),
    ("CHR", "charisma"),
];

struct ItemShape {
    slot: Option<&'static str>,
    max_stack: u32,
    tags: Vec<&'static str>,
    melee: bool,
    launcher: bool,
    behavior_gap: Option<&'static str>,
}

fn item_shape(tval: u16) -> Option<ItemShape> {
    let shape = match tval {
        11 => ItemShape {
            slot: Some("shield"),
            max_stack: 1,
            tags: vec!["capture-ball", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        20 => ItemShape {
            slot: Some("tool"),
            max_stack: 1,
            tags: vec!["equipment", "legacy-import", "tool", "weapon"],
            melee: true,
            launcher: false,
            behavior_gap: None,
        },
        21..=23 => ItemShape {
            slot: Some("weapon"),
            max_stack: 1,
            tags: vec!["equipment", "legacy-import", "weapon"],
            melee: true,
            launcher: false,
            behavior_gap: None,
        },
        46 => ItemShape {
            slot: Some("container"),
            max_stack: 1,
            tags: vec!["container", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        19 => ItemShape {
            slot: Some("launcher"),
            max_stack: 1,
            tags: vec!["equipment", "launcher", "legacy-import"],
            melee: false,
            launcher: true,
            behavior_gap: None,
        },
        16..=18 => ItemShape {
            slot: None,
            max_stack: 99,
            tags: vec!["ammunition", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        36..=38 => ItemShape {
            slot: Some("body"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        32 | 33 => ItemShape {
            slot: Some("head"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        34 => ItemShape {
            slot: Some("shield"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        35 => ItemShape {
            slot: Some("cloak"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        31 => ItemShape {
            slot: Some("gloves"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        30 => ItemShape {
            slot: Some("boots"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        45 => ItemShape {
            slot: Some("ring"),
            max_stack: 1,
            tags: vec!["equipment", "jewelry", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("effect-jewelry"),
        },
        40 => ItemShape {
            slot: Some("amulet"),
            max_stack: 1,
            tags: vec!["equipment", "jewelry", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("effect-jewelry"),
        },
        39 => ItemShape {
            // The body template's light slot exists as of contract-v100;
            // radius/fuel semantics remain a gap.
            slot: Some("light"),
            max_stack: 1,
            tags: vec!["equipment", "legacy-import", "light-source"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        75 => ItemShape {
            slot: None,
            max_stack: 20,
            tags: vec!["consumable", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("consumable-effect"),
        },
        80 => ItemShape {
            slot: None,
            max_stack: 100,
            tags: vec!["consumable", "food", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("consumable-effect"),
        },
        70 | 71 => ItemShape {
            slot: None,
            max_stack: 20,
            tags: vec!["consumable", "legacy-import", "scroll"],
            melee: false,
            launcher: false,
            behavior_gap: Some("scroll-effect"),
        },
        55 | 65 | 66 => ItemShape {
            slot: None,
            max_stack: 10,
            tags: vec!["device", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("device-effect"),
        },
        90..=120 => ItemShape {
            slot: None,
            max_stack: 10,
            tags: vec!["book", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("book-system"),
        },
        _ => ItemShape {
            slot: None,
            max_stack: 10,
            tags: vec!["legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
    };
    Some(shape)
}

/// Canonical ammo partner ids per launcher class, resolved in a prepass so
/// bows/slings/crossbows can satisfy the single-ammo-kind launcher model.
#[derive(Debug, Default, Clone)]
pub struct LauncherAmmoIndex {
    shot: Option<String>,
    arrow: Option<String>,
    bolt: Option<String>,
}

fn launcher_ammo_index(items: &[LegacyItemEntry]) -> LauncherAmmoIndex {
    let mut ammo = LauncherAmmoIndex::default();
    for entry in items {
        if entry.name.is_empty() || entry.name == "something" || entry.glyph.is_none() {
            continue;
        }
        let slot = match entry.tval {
            16 => &mut ammo.shot,
            17 => &mut ammo.arrow,
            18 => &mut ammo.bolt,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(format!("rfb-legacy.item.{}", kebab(&entry.name)));
        }
    }
    ammo
}

fn launcher_ammunition_type(
    entry: &LegacyItemEntry,
    ammo: &LauncherAmmoIndex,
) -> Option<&'static str> {
    match entry.sval {
        2 if ammo.shot.is_some() => Some("shot"),
        12 | 13 if ammo.arrow.is_some() => Some("arrow"),
        23 | 24 if ammo.bolt.is_some() => Some("bolt"),
        _ => None,
    }
}

fn ammunition_type(tval: u16) -> Option<&'static str> {
    match tval {
        16 => Some("shot"),
        17 => Some("arrow"),
        18 => Some("bolt"),
        _ => None,
    }
}

fn launcher_range(multiplier_percent: u16) -> u16 {
    13_u16.saturating_add(multiplier_percent / 80).min(32)
}

fn player_ability_book_for_item(entry: &LegacyItemEntry) -> Option<&'static str> {
    match (entry.tval, entry.sval) {
        (DEATH_BOOK_TVAL, DEATH_FIRST_BOOK_SVAL) => Some(DEATH_FIRST_BOOK_ID),
        (DEATH_BOOK_TVAL, DEATH_SECOND_BOOK_SVAL) => Some(DEATH_SECOND_BOOK_ID),
        (DEATH_BOOK_TVAL, DEATH_THIRD_BOOK_SVAL) => Some(DEATH_THIRD_BOOK_ID),
        (DEATH_BOOK_TVAL, DEATH_FOURTH_BOOK_SVAL) => Some(DEATH_FOURTH_BOOK_ID),
        (SORCERY_BOOK_TVAL, SORCERY_FIRST_BOOK_SVAL) => Some(SORCERY_FIRST_BOOK_ID),
        (SORCERY_BOOK_TVAL, SORCERY_SECOND_BOOK_SVAL) => Some(SORCERY_SECOND_BOOK_ID),
        (SORCERY_BOOK_TVAL, SORCERY_THIRD_BOOK_SVAL) => Some(SORCERY_THIRD_BOOK_ID),
        (SORCERY_BOOK_TVAL, SORCERY_FOURTH_BOOK_SVAL) => Some(SORCERY_FOURTH_BOOK_ID),
        (ARCANE_BOOK_TVAL, ARCANE_FIRST_BOOK_SVAL) => Some(ARCANE_FIRST_BOOK_ID),
        (ARCANE_BOOK_TVAL, ARCANE_SECOND_BOOK_SVAL) => Some(ARCANE_SECOND_BOOK_ID),
        (ARCANE_BOOK_TVAL, ARCANE_THIRD_BOOK_SVAL) => Some(ARCANE_THIRD_BOOK_ID),
        (ARCANE_BOOK_TVAL, ARCANE_FOURTH_BOOK_SVAL) => Some(ARCANE_FOURTH_BOOK_ID),
        (ARMAGEDDON_BOOK_TVAL, ARMAGEDDON_FIRST_BOOK_SVAL) => Some(ARMAGEDDON_FIRST_BOOK_ID),
        (ARMAGEDDON_BOOK_TVAL, ARMAGEDDON_SECOND_BOOK_SVAL) => Some(ARMAGEDDON_SECOND_BOOK_ID),
        (ARMAGEDDON_BOOK_TVAL, ARMAGEDDON_THIRD_BOOK_SVAL) => Some(ARMAGEDDON_THIRD_BOOK_ID),
        (ARMAGEDDON_BOOK_TVAL, ARMAGEDDON_FOURTH_BOOK_SVAL) => Some(ARMAGEDDON_FOURTH_BOOK_ID),
        _ => None,
    }
}

fn mogaminator_kind_is_rare(entry: &LegacyItemEntry) -> bool {
    match entry.tval {
        19 => entry.sval == 70,
        21 => matches!(entry.sval, 20 | 21),
        22 => matches!(entry.sval, 30 | 50),
        23 => matches!(entry.sval, 30 | 31 | 32 | 33 | 35),
        34 => matches!(entry.sval, 6 | 10),
        32 => entry.sval == 8,
        30 => entry.sval == 4,
        35 => matches!(entry.sval, 2 | 5 | 6 | 7),
        31 => entry.sval == 6,
        36 => matches!(entry.sval, 13 | 50),
        38 => true,
        17 => matches!(entry.sval, 3 | 4),
        18 => matches!(entry.sval, 3..=5),
        16 => entry.sval == 3,
        _ => false,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TerrainCreationImportIds {
    source_terrain_ids: Vec<String>,
    floor_terrain_id: Option<String>,
    created_trap_terrain_id: Option<String>,
    glyph_terrain_id: Option<String>,
    tree_terrain_id: Option<String>,
    wall_terrain_id: Option<String>,
    quartz_terrain_id: Option<String>,
    magma_terrain_id: Option<String>,
}

fn terrain_creation_import_ids(terrain: &[LegacyTerrainEntry]) -> TerrainCreationImportIds {
    let mut ids = TerrainCreationImportIds::default();
    let mut seen = BTreeMap::new();
    for entry in terrain {
        if entry.tag.is_empty() || entry.tag == "NONE" || entry.glyph.is_none() {
            continue;
        }
        let mut id = kebab(&entry.tag);
        let duplicates = seen.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        let id = format!("rfb-legacy.terrain.{id}");
        if entry.flags.iter().any(|flag| flag == "FLOOR") {
            ids.source_terrain_ids.push(id.clone());
        }
        match entry.tag.as_str() {
            "FLOOR" => ids.floor_terrain_id = Some(id.clone()),
            "GLYPH" => ids.glyph_terrain_id = Some(id.clone()),
            "TREE" => ids.tree_terrain_id = Some(id),
            "GRANITE" => ids.wall_terrain_id = Some(id.clone()),
            "QUARTZ" => ids.quartz_terrain_id = Some(id.clone()),
            "MAGMA" => ids.magma_terrain_id = Some(id),
            _ => {}
        }
    }
    ids.source_terrain_ids.sort();
    if ids.floor_terrain_id.is_some() {
        ids.created_trap_terrain_id = Some("rfb-legacy.terrain.created-trap".to_owned());
    }
    ids
}

fn fixed_consumable_use_action_with_terrain(
    entry: &LegacyItemEntry,
    terrain_creation: Option<&TerrainCreationImportIds>,
) -> Option<serde_json::Value> {
    let remove_status = |status_kind_id: &str| serde_json::json!({"type": "remove-status", "statusKindId": status_kind_id});
    let sequence = |effects: Vec<serde_json::Value>| serde_json::json!({"type": "sequence", "effects": effects});
    let detect = |subject: &str, category: &str, persistent: bool| {
        serde_json::json!({
            "type": "detect",
            "subject": subject,
            "category": category,
            "radius": 8,
            "persistent": persistent,
            "throughWalls": true
        })
    };
    let detect_floor = |subject: &str, category: &str, persistent: bool| {
        serde_json::json!({
            "type": "detect",
            "subject": subject,
            "category": category,
            "radius": 255,
            "persistent": persistent,
            "throughWalls": true
        })
    };
    let bless = |duration_sides: u32, duration_bonus: u32| {
        serde_json::json!({
            "type": "bless",
            "durationDice": 1,
            "durationSides": duration_sides,
            "durationBonus": duration_bonus
        })
    };
    let summon = |selector: serde_json::Value,
                  maximum_level_source: &str,
                  hostile: bool,
                  count_sides: u8,
                  group_chance_percent: u8,
                  group_count_sides: u8,
                  group_count_bonus: u8,
                  allow_unique: bool| {
        serde_json::json!({
            "type": "summon-category",
            "selector": selector,
            "maximumLevelSource": maximum_level_source,
            "countDice": 1,
            "countSides": count_sides,
            "hostile": hostile,
            "groupChancePercent": group_chance_percent,
            "groupCountDice": 1,
            "groupCountSides": group_count_sides,
            "groupCountBonus": group_count_bonus,
            "allowUnique": allow_unique,
            "radius": 2,
            "durationTurns": 0
        })
    };
    if let Some(effect) = p3_1_food_effect(entry) {
        return Some(serde_json::json!({"effect": effect}));
    }
    let effect = match (entry.tval, entry.sval) {
        (70, 0) => sequence(vec![
            serde_json::json!({
                "type": "apply-blindness",
                "durationDice": 1,
                "durationSides": 5,
                "durationBonus": 3
            }),
            serde_json::json!({
                "type": "set-floor-glow",
                "glow": false,
                "radius": 3,
                "connectedGlow": true
            }),
        ]),
        (70, 1) => serde_json::json!({"type": "aggravate-monsters"}),
        (70, 2) => serde_json::json!({"type": "curse-equipped-item", "target": "armor"}),
        (70, 3) => serde_json::json!({"type": "curse-equipped-item", "target": "weapon"}),
        (70, 4) => summon(
            serde_json::json!({"type": "any-monster"}),
            "dungeon-depth",
            true,
            3,
            100,
            3,
            0,
            true,
        ),
        (70, 5) => summon(
            serde_json::json!({"type": "category", "category": "undead"}),
            "dungeon-depth",
            true,
            3,
            100,
            3,
            0,
            true,
        ),
        (70, 6) => summon(
            serde_json::json!({"type": "any-monster"}),
            "dungeon-depth",
            false,
            1,
            50,
            3,
            1,
            false,
        ),
        (70, 7) => {
            let terrain_creation = terrain_creation?;
            serde_json::json!({
                "type": "create-adjacent-terrain",
                "sourceTerrainIds": terrain_creation.source_terrain_ids,
                "targetTerrainId": terrain_creation.created_trap_terrain_id.as_ref()?
            })
        }
        (70, 8) => serde_json::json!({"type": "random-teleport", "maximumDistance": 10}),
        (70, 9) => serde_json::json!({"type": "random-teleport", "maximumDistance": 100}),
        (70, 10) => serde_json::json!({"type": "teleport-level"}),
        (70, 11) => serde_json::json!({
            "type": "recall",
            "delayDice": 1,
            "delaySides": 21,
            "delayBonus": 14
        }),
        (70, 12) => serde_json::json!({"type": "identify-item", "full": false}),
        (70, 13) => serde_json::json!({"type": "identify-item", "full": true}),
        (70, 14) => {
            serde_json::json!({"type": "remove-equipped-curses", "includeHeavy": false})
        }
        (70, 15) => {
            serde_json::json!({"type": "remove-equipped-curses", "includeHeavy": true})
        }
        (70, 16) => serde_json::json!({
            "type": "enchant-item",
            "toArmor": {"dice": 0, "sides": 0, "bonus": 1}
        }),
        (70, 17) => serde_json::json!({
            "type": "enchant-item",
            "toHit": {"dice": 0, "sides": 0, "bonus": 1}
        }),
        (70, 18) => serde_json::json!({
            "type": "enchant-item",
            "toDamage": {"dice": 0, "sides": 0, "bonus": 1}
        }),
        (70, 20) => serde_json::json!({
            "type": "enchant-item",
            "toArmor": {"dice": 1, "sides": 3, "bonus": 3}
        }),
        (70, 21) => serde_json::json!({
            "type": "enchant-item",
            "toHit": {"dice": 1, "sides": 3, "bonus": 3},
            "toDamage": {"dice": 1, "sides": 3, "bonus": 3}
        }),
        (70, 22) => serde_json::json!({
            "type": "recharge-from-device",
            "power": 100
        }),
        (70, 23) => serde_json::json!({"type": "mundanify-item"}),
        (70, 24) => serde_json::json!({
            "type": "set-floor-glow",
            "glow": true,
            "radius": 2
        }),
        (70, 43) => serde_json::json!({
            "type": "increase-spell-learning-capacity"
        }),
        (70, 25) => detect("terrain", "map", true),
        (70, 26) => detect("gold", "gold", false),
        (70, 27) => detect("item", "item", false),
        (70, 28) => detect("terrain", "trap", true),
        (70, 29) => detect("terrain", "passage", true),
        (70, 30) => detect("actor", "invisible", false),
        (70, 32) => serde_json::json!({"type": "satisfy-hunger"}),
        (70, 33) => bless(12, 6),
        (70, 34) => bless(24, 12),
        (70, 35) => bless(48, 24),
        (70, 36) => serde_json::json!({"type": "prepare-confusing-strike"}),
        (70, 37) => serde_json::json!({"type": "protection-from-evil"}),
        (70, 38) => {
            let terrain_creation = terrain_creation?;
            serde_json::json!({
                "type": "create-current-terrain",
                "sourceTerrainIds": terrain_creation.source_terrain_ids,
                "targetTerrainId": terrain_creation.glyph_terrain_id.as_ref()?
            })
        }
        (70, 39) => serde_json::json!({
            "type": "destroy-adjacent-traps-and-doors"
        }),
        (70, 41) => {
            let terrain_creation = terrain_creation?;
            serde_json::json!({
                "type": "area-destruction",
                "minimumRadius": 13,
                "maximumRadius": 17,
                "floorTerrainId": terrain_creation.floor_terrain_id.as_ref()?,
                "wallTerrainId": terrain_creation.wall_terrain_id.as_ref()?,
                "quartzTerrainId": terrain_creation.quartz_terrain_id.as_ref()?,
                "magmaTerrainId": terrain_creation.magma_terrain_id.as_ref()?
            })
        }
        (70, 42) => serde_json::json!({
            "type": "dispel-category",
            "category": "undead",
            "damage": 80
        }),
        (70, 44) => serde_json::json!({
            "type": "genocide",
            "power": 300
        }),
        (70, 45) => serde_json::json!({
            "type": "mass-genocide",
            "power": 300,
            "radius": 20
        }),
        (70, 46) => serde_json::json!({
            "type": "acquirement",
            "lootTableId": LEGACY_DROP_TABLE_ID,
            "minimumCount": 1,
            "maximumCount": 1
        }),
        (70, 47) => serde_json::json!({
            "type": "acquirement",
            "lootTableId": LEGACY_DROP_TABLE_ID,
            "minimumCount": 2,
            "maximumCount": 3
        }),
        (70, 48) => {
            let terrain_creation = terrain_creation?;
            serde_json::json!({
                "type": "create-adjacent-terrain",
                "sourceTerrainIds": terrain_creation.source_terrain_ids,
                "targetTerrainId": terrain_creation.tree_terrain_id.as_ref()?
            })
        }
        (70, 49) => {
            let terrain_creation = terrain_creation?;
            serde_json::json!({
                "type": "create-adjacent-terrain",
                "sourceTerrainIds": terrain_creation.source_terrain_ids,
                "targetTerrainId": terrain_creation.wall_terrain_id.as_ref()?
            })
        }
        (70, 50) => serde_json::json!({
            "type": "vengeance",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 25
        }),
        (70, 51) => serde_json::json!({
            "type": "show-rumour",
            "messageKey": "rumour-legacy-wilderness"
        }),
        (70, 58) => serde_json::json!({
            "type": "self-centered-elemental-blast",
            "baseDamage": 666,
            "damageType": "fire",
            "radius": 4,
            "backlashSides": 25,
            "backlashBonus": 25,
            "backlashDamageType": "fire",
            "backlashUsesResistance": true
        }),
        (70, 59) => serde_json::json!({
            "type": "self-centered-elemental-blast",
            "baseDamage": 800,
            "damageType": "ice",
            "radius": 4,
            "backlashSides": 30,
            "backlashBonus": 30,
            "backlashDamageType": "cold",
            "backlashUsesResistance": true
        }),
        (70, 60) => sequence(vec![
            serde_json::json!({"type": "identify-inventory"}),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.understanding",
                "durationDice": 1,
                "durationSides": 1,
                "durationBonus": 39,
                "stacking": "extend"
            }),
        ]),
        (70, 61) => serde_json::json!({
            "type": "self-centered-elemental-blast",
            "baseDamage": 1100,
            "damageType": "mana",
            "radius": 4,
            "backlashSides": 50,
            "backlashBonus": 50,
            "backlashDamageType": "mana",
            "backlashUsesResistance": false
        }),
        (70, 57) => detect("actor", "legacy-import", false),
        (70, 53) => serde_json::json!({"type": "reset-recall"}),
        (70, 54) => summon(
            serde_json::json!({"type": "player-kin"}),
            "player-level",
            false,
            1,
            50,
            3,
            1,
            false,
        ),
        (70, 55) => serde_json::json!({
            "type": "craft-item",
            "weaponAffixIds": [
                "rfb-legacy.affix.of-sharpness",
                "rfb-legacy.affix.of-slaying"
            ],
            "armorAffixIds": ["rfb-legacy.affix.of-protection"]
        }),
        (70, 62) => serde_json::json!({
            "type": "banish-visible",
            "maximumDistance": 150
        }),
        (70, 63) => serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.inventory-protection",
            "durationDice": 1,
            "durationSides": 1,
            "durationBonus": 24,
            "stacking": "extend"
        }),
        (75, 0..=2) => serde_json::json!({"type": "no-numeric-effect"}),
        (75, 13) => serde_json::json!({
            "type": "lose-experience-fraction",
            "divisor": 4
        }),
        (75, 15) => sequence(vec![
            serde_json::json!({"type": "self-damage", "damageDice": 10, "damageSides": 10}),
            serde_json::json!({"type": "drain-attribute", "attribute": "strength"}),
            serde_json::json!({"type": "drain-attribute", "attribute": "intelligence"}),
            serde_json::json!({"type": "drain-attribute", "attribute": "wisdom"}),
            serde_json::json!({"type": "drain-attribute", "attribute": "dexterity"}),
            serde_json::json!({"type": "drain-attribute", "attribute": "constitution"}),
            serde_json::json!({"type": "drain-attribute", "attribute": "charisma"}),
        ]),
        (75, 26) => sequence(vec![
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.sight",
                "durationDice": 1,
                "durationSides": 100,
                "durationBonus": 100,
                "stacking": "extend",
                "grantedEquipmentBonuses": {"infravision": 3}
            }),
            remove_status("rfb.status.blindness"),
        ]),
        (75, 27) => sequence(vec![
            serde_json::json!({
                "type": "reduce-status",
                "statusKindId": "rfb.status.poison",
                "minimumReduction": 4000,
                "reductionDivisor": 2
            }),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.poison-resistance",
                "durationDice": 1,
                "durationSides": 10,
                "durationBonus": 10,
                "stacking": "extend",
                "grantedResistances": {"poison": "resistant"}
            }),
        ]),
        (75, 4) => serde_json::json!({
            "type": "apply-slowness",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 15
        }),
        (75, 11) => serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.paralysis",
            "durationDice": 1,
            "durationSides": 4,
            "durationBonus": 0,
            "stacking": "extend"
        }),
        (75, 29) => serde_json::json!({
            "type": "apply-speed",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 15
        }),
        (75, 32) => serde_json::json!({
            "type": "apply-heroism",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 25
        }),
        (75, 33) => serde_json::json!({
            "type": "apply-berserk-strength",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 25
        }),
        (75, 14) => serde_json::json!({
            "type": "apply-poetic-inspiration",
            "durationDice": 1,
            "durationSides": 100,
            "durationBonus": 100
        }),
        (75, 69) => serde_json::json!({
            "type": "apply-stone-skin",
            "durationDice": 1,
            "durationSides": 20,
            "durationBonus": 20
        }),
        (75, 41) => serde_json::json!({
            "type": "restore-life-levels",
            "lifeForceAmount": 150
        }),
        (75, 54) => serde_json::json!({
            "type": "restore-all-vitality",
            "lifeForceAmount": 150
        }),
        (80, 17) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "strength"
        }),
        (80, 18) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "constitution"
        }),
        (80, 19) => serde_json::json!({
            "type": "restore-all-attributes"
        }),
        (80, 40) => serde_json::json!({
            "type": "apply-restorative-feast",
            "healingDice": 15,
            "healingSides": 15
        }),
        (75, 16) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "strength"
        }),
        (75, 17) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "intelligence"
        }),
        (75, 18) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "wisdom"
        }),
        (75, 19) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "dexterity"
        }),
        (75, 20) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "constitution"
        }),
        (75, 21) => serde_json::json!({
            "type": "drain-attribute",
            "attribute": "charisma"
        }),
        (75, 42) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "strength"
        }),
        (75, 43) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "intelligence"
        }),
        (75, 44) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "wisdom"
        }),
        (75, 45) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "dexterity"
        }),
        (75, 46) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "constitution"
        }),
        (75, 47) => serde_json::json!({
            "type": "restore-attribute",
            "attribute": "charisma"
        }),
        (75, 48) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "strength"
        }),
        (75, 49) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "intelligence"
        }),
        (75, 50) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "wisdom"
        }),
        (75, 51) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "dexterity"
        }),
        (75, 52) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "constitution"
        }),
        (75, 53) => serde_json::json!({
            "type": "increase-attribute",
            "attribute": "charisma"
        }),
        (75, 55) => serde_json::json!({
            "type": "augment-attributes"
        }),
        (75, 56) => sequence(vec![
            detect_floor("terrain", "map", true),
            detect_floor("item", "item", false),
            detect_floor("gold", "gold", false),
        ]),
        (75, 57) => sequence(vec![
            detect_floor("terrain", "map", true),
            detect_floor("item", "item", false),
            detect_floor("gold", "gold", false),
            serde_json::json!({"type": "increase-attribute", "attribute": "intelligence"}),
            serde_json::json!({"type": "increase-attribute", "attribute": "wisdom"}),
            detect_floor("terrain", "trap", true),
            detect_floor("terrain", "passage", true),
            serde_json::json!({"type": "identify-inventory"}),
            serde_json::json!({"type": "self-knowledge"}),
        ]),
        (75, 58) => serde_json::json!({"type": "self-knowledge"}),
        (75, 59) => serde_json::json!({
            "type": "gain-relative-experience",
            "divisor": 2,
            "bonus": 10,
            "maximumGain": 100000
        }),
        (75, 30) => serde_json::json!({
            "type": "apply-thermal-resistance",
            "durationDice": 1,
            "durationSides": 10,
            "durationBonus": 10
        }),
        (75, 60) => serde_json::json!({
            "type": "apply-basic-resistance",
            "durationDice": 1,
            "durationSides": 20,
            "durationBonus": 20
        }),
        (75, 61) => sequence(vec![
            remove_status("rfb.status.blindness"),
            serde_json::json!({
                "type": "reduce-status",
                "statusKindId": "rfb.status.poison",
                "minimumReduction": 2000,
                "reductionDivisor": 2
            }),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.bleeding"),
            remove_status("rfb.status.hallucination"),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 62) => serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.invulnerability",
            "durationDice": 1,
            "durationSides": 4,
            "durationBonus": 4,
            "stacking": "extend",
            "incomingDamagePercent": 0
        }),
        (75, 64) => sequence(vec![
            remove_status("rfb.status.hallucination"),
            serde_json::json!({
                "type": "apply-tsuyoshi",
                "durationDice": 1,
                "durationSides": 100,
                "durationBonus": 100
            }),
        ]),
        (75, 65) => sequence(vec![
            serde_json::json!({"type": "trigger-tsuyoshi-crash"}),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.hallucination",
                "durationDice": 1,
                "durationSides": 50,
                "durationBonus": 50,
                "stacking": "keep-strongest",
                "resistanceType": "chaos"
            }),
        ]),
        (75, 68) => serde_json::json!({
            "type": "apply-giant-strength",
            "durationDice": 1,
            "durationSides": 20,
            "durationBonus": 20
        }),
        (75, 6) => serde_json::json!({
            "type": "apply-poison",
            "durationDice": 1,
            "durationSides": 15,
            "durationBonus": 9
        }),
        (80, 0) => serde_json::json!({
            "type": "apply-poison",
            "durationDice": 1,
            "durationSides": 10,
            "durationBonus": 9
        }),
        (75, 7) => serde_json::json!({
            "type": "apply-blindness",
            "durationDice": 1,
            "durationSides": 100,
            "durationBonus": 99
        }),
        (75, 22) => serde_json::json!({
            "type": "apply-detonation",
            "damageDice": 50,
            "damageSides": 20,
            "stunTicks": 75,
            "bleedingTicks": 5000
        }),
        (80, 1) => serde_json::json!({
            "type": "apply-blindness",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 24
        }),
        (75, 23) => serde_json::json!({
            "type": "self-life-loss",
            "amount": 5000
        }),
        (80, 12) => remove_status("rfb.status.poison"),
        (80, 13) => remove_status("rfb.status.blindness"),
        (80, 14) => remove_status("rfb.status.fear"),
        (80, 15) => remove_status("rfb.status.confusion"),
        (75, 28) => remove_status("rfb.status.fear"),
        (75, 31) => sequence(vec![
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.slow"),
        ]),
        (75, 34) => sequence(vec![
            serde_json::json!({"type": "heal-dice", "dice": 4, "sides": 8}),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 35) => sequence(vec![
            serde_json::json!({"type": "heal-dice", "dice": 8, "sides": 8}),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 36) => sequence(vec![
            serde_json::json!({"type": "heal-dice", "dice": 12, "sides": 8}),
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.bleeding"),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 37) => sequence(vec![
            serde_json::json!({"type": "heal", "amount": 300}),
            remove_status("rfb.status.blindness"),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.bleeding"),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 38) => sequence(vec![
            serde_json::json!({"type": "heal", "amount": 1000}),
            remove_status("rfb.status.blindness"),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.poison"),
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.bleeding"),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 39) => serde_json::json!({
            "type": "apply-life-restoration",
            "healingAmount": 5000,
            "lifeForceAmount": 1000
        }),
        (75, 40) => sequence(vec![
            serde_json::json!({
                "type": "restore-resource-full",
                "resourceId": LEGACY_MANA_RESOURCE_ID
            }),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 67) => sequence(vec![
            serde_json::json!({"type": "heal", "amount": 200}),
            remove_status("rfb.status.blindness"),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.stun"),
        ]),
        (75, 70) => sequence(vec![
            serde_json::json!({
                "type": "restore-resource-dice",
                "resourceId": LEGACY_MANA_RESOURCE_ID,
                "dice": 3,
                "sides": 6,
                "bonus": 3
            }),
            remove_status("rfb.status.confusion"),
        ]),
        (75, 71) => sequence(vec![
            serde_json::json!({
                "type": "restore-resource-dice",
                "resourceId": LEGACY_MANA_RESOURCE_ID,
                "dice": 10,
                "sides": 10,
                "bonus": 15
            }),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.hallucination"),
        ]),
        _ => return None,
    };
    Some(serde_json::json!({"effect": effect}))
}

fn p3_1_food_effect(entry: &LegacyItemEntry) -> Option<serde_json::Value> {
    if entry.tval != 80 {
        return None;
    }
    if entry.sval == 37 {
        return Some(serde_json::json!({
            "type": "apply-elvish-waybread",
            "healingDice": 4,
            "healingSides": 8
        }));
    }
    let effect = match entry.sval {
        0 => Some(serde_json::json!({
            "type": "apply-poison",
            "durationDice": 1,
            "durationSides": 10,
            "durationBonus": 9
        })),
        1 => Some(serde_json::json!({
            "type": "apply-blindness",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 24
        })),
        2 => Some(serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.fear",
            "durationDice": 1,
            "durationSides": 10,
            "durationBonus": 9,
            "stacking": "extend"
        })),
        3 => Some(serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.confusion",
            "durationDice": 1,
            "durationSides": 10,
            "durationBonus": 9,
            "stacking": "extend",
            "resistanceType": "confusion"
        })),
        4 => Some(serde_json::json!({
            "type": "sequence",
            "effects": [
                {
                    "type": "apply-status",
                    "statusKindId": "rfb.status.hallucination",
                    "durationDice": 1,
                    "durationSides": 25,
                    "durationBonus": 24,
                    "stacking": "extend",
                    "resistanceType": "chaos"
                },
                {"type": "drain-resource-full", "resourceId": LEGACY_MANA_RESOURCE_ID}
            ]
        })),
        5 => Some(serde_json::json!({
            "type": "apply-status",
            "statusKindId": "rfb.status.paralysis",
            "durationDice": 1,
            "durationSides": 4,
            "durationBonus": 0,
            "stacking": "extend"
        })),
        6 => Some(food_damage_and_drain(6, 6, "strength")),
        7 => Some(food_damage_and_drain(6, 6, "constitution")),
        8 => Some(food_damage_and_drain(8, 8, "intelligence")),
        9 => Some(food_damage_and_drain(8, 8, "wisdom")),
        10 => Some(food_damage_and_drain(10, 10, "constitution")),
        11 => Some(food_damage_and_drain(10, 10, "strength")),
        12 => Some(serde_json::json!({
            "type": "remove-status",
            "statusKindId": "rfb.status.poison"
        })),
        13 => Some(serde_json::json!({
            "type": "remove-status",
            "statusKindId": "rfb.status.blindness"
        })),
        14 => Some(serde_json::json!({
            "type": "remove-status",
            "statusKindId": "rfb.status.fear"
        })),
        15 => Some(serde_json::json!({
            "type": "remove-status",
            "statusKindId": "rfb.status.confusion"
        })),
        17 => Some(serde_json::json!({
            "type": "restore-attribute",
            "attribute": "strength"
        })),
        18 => Some(serde_json::json!({
            "type": "restore-attribute",
            "attribute": "constitution"
        })),
        32 | 33 | 36 => None,
        _ => return None,
    };
    let nutrition = u16::try_from(entry.pval)
        .ok()
        .filter(|amount| *amount > 0)?;
    let nutrition = serde_json::json!({"type": "increase-nutrition", "amount": nutrition});
    Some(match effect {
        None => nutrition,
        Some(mut effect) if effect["type"] == "sequence" => {
            effect["effects"]
                .as_array_mut()
                .expect("food sequence must contain effects")
                .push(nutrition);
            effect
        }
        Some(effect) => serde_json::json!({"type": "sequence", "effects": [effect, nutrition]}),
    })
}

fn food_damage_and_drain(dice: u16, sides: u16, attribute: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "sequence",
        "effects": [
            {"type": "self-damage", "damageDice": dice, "damageSides": sides},
            {"type": "drain-attribute", "attribute": attribute}
        ]
    })
}

#[cfg(test)]
fn fixed_consumable_use_action(entry: &LegacyItemEntry) -> Option<serde_json::Value> {
    fixed_consumable_use_action_with_terrain(entry, None)
}

fn legacy_device_generation(entry: &LegacyItemEntry) -> Option<serde_json::Value> {
    let activation = |id: &str,
                      name_key: &str,
                      difficulty: i32,
                      minimum: u32,
                      maximum: u32,
                      cost: u32,
                      target: serde_json::Value,
                      effect: serde_json::Value| {
        serde_json::json!({
            "id": id,
            "nameKey": name_key,
            "weight": 1,
            "minDepth": 1,
            "maxDepth": 100,
            "deviceCheckDifficulty": difficulty,
            "charges": {
                "minimum": minimum,
                "maximum": maximum,
                "cost": cost
            },
            "target": target,
            "effect": effect
        })
    };
    let activations = match entry.tval {
        65 => vec![
            activation(
                "rfb-legacy.device-activation.frost-bolt",
                "device-activation-legacy-frost-bolt-name",
                30,
                12,
                24,
                7,
                serde_json::json!({
                    "modes": ["direction", "position", "entity"],
                    "range": 8,
                    "requiresLineOfEffect": true
                }),
                serde_json::json!({
                    "type": "damage",
                    "damageDice": 3,
                    "damageSides": 6,
                    "damageType": "cold"
                }),
            ),
            activation(
                "rfb-legacy.device-activation.magic-missile",
                "device-activation-legacy-magic-missile-name",
                10,
                12,
                24,
                3,
                serde_json::json!({
                    "modes": ["direction", "position", "entity"],
                    "range": 8,
                    "requiresLineOfEffect": true
                }),
                serde_json::json!({
                    "type": "damage",
                    "damageDice": 2,
                    "damageSides": 6,
                    "damageType": "physical"
                }),
            ),
        ],
        66 => vec![activation(
            "rfb-legacy.device-activation.detect-traps",
            "device-activation-legacy-detect-traps-name",
            10,
            18,
            32,
            9,
            serde_json::json!({
                "modes": ["self"],
                "range": 0,
                "requiresLineOfEffect": false
            }),
            serde_json::json!({
                "type": "detect",
                "subject": "terrain",
                "category": "trap",
                "radius": 8,
                "persistent": true
            }),
        )],
        55 => vec![activation(
            "rfb-legacy.device-activation.heal",
            "device-activation-legacy-heal-name",
            20,
            24,
            48,
            10,
            serde_json::json!({
                "modes": ["self"],
                "range": 0,
                "requiresLineOfEffect": false
            }),
            serde_json::json!({
                "type": "heal",
                "amount": 50
            }),
        )],
        _ => return None,
    };
    let interval_ticks = if entry.tval == 66 { 1 } else { 10 };
    Some(serde_json::json!({
        "activations": activations,
        "recovery": {
            "intervalTicks": interval_ticks,
            "energyPerMille": 10
        }
    }))
}

fn item_json_with_terrain(
    entry: &LegacyItemEntry,
    id: &str,
    ammo: &LauncherAmmoIndex,
    ability_book_id: Option<&str>,
    terrain_creation: Option<&TerrainCreationImportIds>,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let shape = item_shape(entry.tval).expect("every tval resolves a shape");
    let mut tags = shape.tags.clone();
    if entry.tval == 127 {
        tags.push("gold");
    }
    if entry.flags.iter().any(|flag| flag == "NO_REMOVE")
        || matches!((entry.tval, entry.sval), (23, 32 | 34))
    {
        tags.push("unbrandable");
    }
    let use_action = fixed_consumable_use_action_with_terrain(entry, terrain_creation);
    let device_generation = legacy_device_generation(entry);
    let behavior_gap = if entry.tval == 70 && entry.sval == 52 {
        Some("random-artifact-identity")
    } else {
        shape.behavior_gap
    };
    if let Some(gap) = behavior_gap
        && ability_book_id.is_none()
        && use_action.is_none()
        && device_generation.is_none()
    {
        *report.item_behavior_gaps.entry(gap.to_owned()).or_default() += 1;
    }
    if entry.tval == 80 && p3_1_food_effect(entry).is_none() {
        *report
            .item_behavior_gaps
            .entry("food-nutrition".to_owned())
            .or_default() += 1;
    }
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/item.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.item.{id}"),
        "nameKey": format!("item-legacy-{id}-name"),
        "descriptionKey": format!("item-legacy-{id}-description"),
        "glyph": entry.glyph.map_or_else(|| "?".to_owned(), |glyph| glyph.to_string()),
        "generationLevel": entry.level,
        "mogaminatorRare": mogaminator_kind_is_rare(entry),
        "weightTenthsPound": entry.weight_tenths_pound.max(1),
        "maxStack": shape.max_stack,
        "baseValue": entry.base_value,
        "tags": tags,
    });
    if entry.tval == 11 {
        value["captureBall"] = serde_json::json!(true);
    }
    if let Some(ability_book_id) = ability_book_id {
        value["maxStack"] = serde_json::json!(1);
        value["abilityBookId"] = serde_json::json!(ability_book_id);
    }
    if let Some(use_action) = use_action {
        value["useAction"] = use_action;
    }
    if let Some(device_generation) = device_generation {
        value["maxStack"] = serde_json::json!(1);
        value["deviceGeneration"] = device_generation;
    }
    if let Some((effect, radius)) = potion_shatter_effect(entry) {
        value["shatterEffect"] = effect;
        value["shatterRadius"] = serde_json::json!(radius);
    } else if potion_has_unimplemented_shatter_effect(entry) {
        *report
            .item_behavior_gaps
            .entry("potion-shatter-effect".to_owned())
            .or_default() += 1;
    }
    if let Some(slot) = shape.slot {
        value["equipmentSlot"] = serde_json::json!(slot);
    }
    if let Some(ammunition_type) = ammunition_type(entry.tval) {
        value["breakChancePercent"] = serde_json::json!(if entry.tval == 17 { 20 } else { 10 });
        if let Some((dice, sides)) = entry.damage_dice {
            value["ammunitionProfile"] = serde_json::json!({
                "ammunitionType": ammunition_type,
                "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
                "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
                "damageDice": dice.clamp(1, 100),
                "damageSides": sides.clamp(1, 10_000),
            });
        } else {
            *report
                .item_behavior_gaps
                .entry("ammo-dice-folded".to_owned())
                .or_default() += 1;
        }
    }
    if shape.melee {
        let (dice, sides) = entry.damage_dice.unwrap_or((1, 1));
        value["meleeProfile"] = serde_json::json!({
            "attacks": 1,
            "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
            "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
            "damageDice": dice.clamp(1, 100),
            "damageSides": sides.clamp(1, 10_000),
        });
    }
    if entry.flags.iter().any(|flag| flag == "RIDING") {
        value["ridingWeaponKind"] =
            serde_json::json!(if matches!((entry.tval, entry.sval), (22, 20 | 29)) {
                "lance"
            } else {
                "compatible"
            });
    }
    if shape.launcher {
        // Instruments and other exotic entries share the legacy bow tval;
        // only launchers with a canonical ammo partner keep the profile.
        // The rest stay equippable fake bows (legacy obj_is_fake_bow):
        // they occupy the launcher slot but cannot fire.
        if let Some(ammunition_type) = launcher_ammunition_type(entry, ammo) {
            if let Some(multiplier_percent) = entry.launcher_multiplier_percent {
                value["projectileProfile"] = serde_json::json!({
                    "range": launcher_range(multiplier_percent),
                    "damageMultiplierPercent": multiplier_percent,
                    "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
                    "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
                    "ammunitionType": ammunition_type,
                });
            } else {
                *report
                    .item_behavior_gaps
                    .entry("launcher-multiplier".to_owned())
                    .or_default() += 1;
            }
        } else {
            *report
                .item_behavior_gaps
                .entry("launcher-unpaired".to_owned())
                .or_default() += 1;
        }
    }
    // Armour class and to-armor bonuses fold into the defense modifier; base
    // items stay attribute-free — pval powers arrive via egos and artifacts.
    // Defensive flags (dragon scale resistances, Silver DSM free action) are
    // inherent base properties and fold when a slot exists to carry them.
    let fold = if shape.slot.is_some() {
        defensive_fold(&entry.flags, entry.pval)
    } else {
        DefensiveFold::default()
    };
    let offense = if shape.slot.is_some() {
        offensive_fold(&entry.flags)
    } else {
        OffensiveFold::default()
    };
    let mut equipment = if shape.slot.is_some() {
        equipment_fold(&entry.flags, entry.pval)
    } else {
        EquipmentFold::default()
    };
    if shape.slot.is_some() && !shape.melee && !shape.launcher {
        add_equipment_bonus(
            &mut equipment,
            "meleeSkill",
            entry.to_hit.clamp(-1_000_000, 1_000_000),
        );
        add_equipment_bonus(
            &mut equipment,
            "meleeDamage",
            entry.to_damage.clamp(-1_000_000, 1_000_000),
        );
    }
    let mut modifiers = serde_json::Map::new();
    let defense = entry.armor_class.saturating_add(entry.to_armor);
    if shape.slot.is_some() && defense != 0 {
        modifiers.insert("defense".to_owned(), serde_json::json!(defense));
    }
    if entry.tval == 46 && entry.sval == 1 {
        value["inventorySlotBonus"] =
            serde_json::json!(entry.pval.saturating_add(1).saturating_mul(4));
    }
    if fold.speed != 0 {
        modifiers.insert("speed".to_owned(), serde_json::json!(fold.speed));
    }
    fold_spell_power_modifier(&entry.flags, entry.pval, &mut modifiers, &mut equipment);
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    apply_item_destruction_properties(&mut value, entry.tval, &entry.flags);
    for flag in &entry.flags {
        if matches!(flag.as_str(), "NO_REMOVE" | "RIDING") || item_destruction_flag_is_mapped(flag)
        {
            continue;
        }
        account_item_flag(
            flag,
            &fold,
            &offense,
            &equipment,
            &mut report.unmapped_item_flags,
            &mut report.not_applicable_item_flags,
        );
    }
    value
}

fn potion_shatter_effect(entry: &LegacyItemEntry) -> Option<(serde_json::Value, u8)> {
    if entry.tval != 75 {
        return None;
    }
    let (effect, radius) = match entry.sval {
        6 => (
            serde_json::json!({
                "type": "damage",
                "damageDice": 0,
                "damageSides": 0,
                "damageBonus": 3,
                "damageType": "poison",
            }),
            2,
        ),
        15 | 22 => (
            serde_json::json!({
                "type": "damage",
                "damageDice": 25,
                "damageSides": 25,
                "damageType": "shards",
            }),
            2,
        ),
        34 => (
            serde_json::json!({"type": "heal-dice", "dice": 2, "sides": 3}),
            2,
        ),
        35 => (
            serde_json::json!({"type": "heal-dice", "dice": 4, "sides": 3}),
            2,
        ),
        36 => (
            serde_json::json!({"type": "heal-dice", "dice": 6, "sides": 3}),
            2,
        ),
        37 => (
            serde_json::json!({"type": "heal-dice", "dice": 10, "sides": 10}),
            2,
        ),
        38 => (
            serde_json::json!({"type": "heal-dice", "dice": 50, "sides": 50}),
            1,
        ),
        40 => (
            serde_json::json!({
                "type": "damage",
                "damageDice": 10,
                "damageSides": 10,
                "damageType": "mana",
            }),
            1,
        ),
        _ => return None,
    };
    Some((effect, radius))
}

fn potion_has_unimplemented_shatter_effect(entry: &LegacyItemEntry) -> bool {
    entry.tval == 75 && matches!(entry.sval, 4 | 7 | 9 | 11 | 12 | 23 | 29 | 39 | 41 | 59)
}

#[cfg(test)]
fn item_json(
    entry: &LegacyItemEntry,
    id: &str,
    ammo: &LauncherAmmoIndex,
    ability_book_id: Option<&str>,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    item_json_with_terrain(entry, id, ammo, ability_book_id, None, report)
}

fn demo_item_json(
    entry: &LegacyItemEntry,
    id: &str,
    ammo: &LauncherAmmoIndex,
) -> Result<serde_json::Value, LegacyImportError> {
    let shape = item_shape(entry.tval).expect("every tval resolves a shape");
    let bare_jewelry = matches!(shape.slot, Some("ring" | "amulet"));
    let ordinary_equipment = matches!(
        shape.slot,
        Some(
            "weapon"
                | "body"
                | "head"
                | "shield"
                | "cloak"
                | "gloves"
                | "boots"
                | "tool"
                | "container"
                | "launcher"
                | "ring"
        )
    );
    if (!ordinary_equipment && !shape.tags_contain("ammunition"))
        || (shape.behavior_gap.is_some() && !bare_jewelry)
        || fixed_consumable_use_action_with_terrain(entry, None).is_some()
        || legacy_device_generation(entry).is_some()
        || player_ability_book_for_item(entry).is_some()
    {
        return Err(LegacyImportError::InvalidDemoItemSelection(format!(
            "{id} is not a behavior-complete ordinary equipment or ammunition item"
        )));
    }

    let mut report = ContentImportReport::default();
    let mut value = item_json_with_terrain(entry, id, ammo, None, None, &mut report);
    report.unmapped_item_flags.remove("TOWN");
    if bare_jewelry {
        report.item_behavior_gaps.remove("effect-jewelry");
    }
    if !report.item_behavior_gaps.is_empty() || !report.unmapped_item_flags.is_empty() {
        return Err(LegacyImportError::InvalidDemoItemSelection(format!(
            "{id} still has import gaps"
        )));
    }

    value["id"] = serde_json::json!(format!("demo.item.{id}"));
    value["nameKey"] = serde_json::json!(format!("item-demo-{id}-name"));
    value["descriptionKey"] = serde_json::json!(format!("item-demo-{id}-description"));
    if let Some(tags) = value["tags"].as_array_mut() {
        for tag in tags.iter_mut() {
            if tag == "legacy-import" {
                *tag = serde_json::json!("rfb-compatibility");
            }
        }
        tags.sort_by_key(serde_json::Value::to_string);
        tags.dedup();
    }
    Ok(value)
}

impl ItemShape {
    fn tags_contain(&self, tag: &str) -> bool {
        self.tags.contains(&tag)
    }
}

/// Parses e_info ego templates: `C:` carries generation-time maximum
/// bonuses, `T:` accumulates the applicable slot classes.
pub fn parse_e_info(text: &str) -> Result<Vec<LegacyEgoEntry>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyEgoEntry> = None;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parse_number(E_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let name =
                required_field(E_INFO_SOURCE, line_number, "N.name", parts.next())?.to_owned();
            current = Some(LegacyEgoEntry {
                index,
                name,
                ..LegacyEgoEntry::default()
            });
            continue;
        }
        let recognized = ["T:", "W:", "C:", "F:", "E:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let entry = match current.as_mut() {
            Some(entry) => entry,
            None if recognized => {
                return Err(content_parse_error(
                    E_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("T:") {
            required_field(E_INFO_SOURCE, line_number, "T.slots", Some(rest))?;
            entry.slots.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|slot| !slot.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(rest) = line.strip_prefix("W:") {
            let parts = parse_fields(E_INFO_SOURCE, line_number, "W", rest, 3)?;
            entry.level = parse_number(
                E_INFO_SOURCE,
                line_number,
                "W.level",
                parts.first().copied(),
            )?;
            if parts[1] != "*" {
                entry.max_level = Some(parse_number(
                    E_INFO_SOURCE,
                    line_number,
                    "W.max_level",
                    parts.get(1).copied(),
                )?);
            }
            let _: i64 = parse_number(
                E_INFO_SOURCE,
                line_number,
                "W.rating",
                parts.get(2).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("C:") {
            let parts = parse_fields(E_INFO_SOURCE, line_number, "C", rest, 4)?;
            entry.max_to_hit = parse_number(
                E_INFO_SOURCE,
                line_number,
                "C.maximumToHit",
                parts.first().copied(),
            )?;
            entry.max_to_damage = parse_number(
                E_INFO_SOURCE,
                line_number,
                "C.maximumToDamage",
                parts.get(1).copied(),
            )?;
            entry.max_to_armor = parse_number(
                E_INFO_SOURCE,
                line_number,
                "C.maximumToArmor",
                parts.get(2).copied(),
            )?;
            entry.max_pval = parse_number(
                E_INFO_SOURCE,
                line_number,
                "C.maximumPval",
                parts.get(3).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("F:") {
            entry.flags.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                    .map(str::to_owned),
            );
        } else if line.starts_with("E:") {
            entry.has_activation = true;
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

/// Parses a_info fixed artifacts; unlike egos their pval and combat bonuses
/// are fixed values. `E:` activation lines (ASCII token form) mark the
/// activation gap; localized text lines are skipped outright.
pub fn parse_a_info(text: &str) -> Result<Vec<LegacyArtifactEntry>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyArtifactEntry> = None;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parse_number(A_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let name =
                required_field(A_INFO_SOURCE, line_number, "N.name", parts.next())?.to_owned();
            current = Some(LegacyArtifactEntry {
                index,
                name,
                ..LegacyArtifactEntry::default()
            });
            continue;
        }
        let recognized = ["I:", "W:", "P:", "F:", "E:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let entry = match current.as_mut() {
            Some(entry) => entry,
            None if recognized => {
                return Err(content_parse_error(
                    A_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("I:") {
            let parts = parse_fields(A_INFO_SOURCE, line_number, "I", rest, 3)?;
            entry.tval =
                parse_number(A_INFO_SOURCE, line_number, "I.tval", parts.first().copied())?;
            entry.sval = parse_number(A_INFO_SOURCE, line_number, "I.sval", parts.get(1).copied())?;
            entry.pval = parse_number(A_INFO_SOURCE, line_number, "I.pval", parts.get(2).copied())?;
        } else if let Some(rest) = line.strip_prefix("W:") {
            // level:rarity:weight:cost for artifacts.
            // The pinned parser reads the first four fields with sscanf and
            // ignores one known trailing value in the legacy source.
            let parts = rest.split(':').map(str::trim).collect::<Vec<_>>();
            if parts.len() < 4 {
                return Err(content_parse_error(
                    A_INFO_SOURCE,
                    line_number,
                    "W",
                    rest,
                    format!("expected at least 4 fields, found {}", parts.len()),
                ));
            }
            entry.level = parse_number(
                A_INFO_SOURCE,
                line_number,
                "W.level",
                parts.first().copied(),
            )?;
            entry.rarity_one_in = parse_number(
                A_INFO_SOURCE,
                line_number,
                "W.rarity",
                parts.get(1).copied(),
            )?;
            entry.weight_tenths_pound = parse_number(
                A_INFO_SOURCE,
                line_number,
                "W.weight",
                parts.get(2).copied(),
            )?;
            entry.base_value =
                parse_number(A_INFO_SOURCE, line_number, "W.cost", parts.get(3).copied())?;
        } else if let Some(rest) = line.strip_prefix("P:") {
            let parts = parse_fields(A_INFO_SOURCE, line_number, "P", rest, 5)?;
            entry.armor_class = parse_number(
                A_INFO_SOURCE,
                line_number,
                "P.armorClass",
                parts.first().copied(),
            )?;
            entry.damage_dice = parse_damage_or_multiplier(
                A_INFO_SOURCE,
                line_number,
                "P.damage",
                parts.get(1).copied(),
            )?;
            entry.launcher_multiplier_percent = parse_launcher_multiplier(
                A_INFO_SOURCE,
                line_number,
                "P.damage",
                parts.get(1).copied(),
            )?;
            entry.to_hit =
                parse_number(A_INFO_SOURCE, line_number, "P.toHit", parts.get(2).copied())?;
            entry.to_damage = parse_number(
                A_INFO_SOURCE,
                line_number,
                "P.toDamage",
                parts.get(3).copied(),
            )?;
            entry.to_armor = parse_number(
                A_INFO_SOURCE,
                line_number,
                "P.toArmor",
                parts.get(4).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("F:") {
            entry.flags.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|flag| !flag.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(rest) = line.strip_prefix("E:")
            && rest
                .split(':')
                .next()
                .is_some_and(|token| token.bytes().all(|b| b.is_ascii()) && !token.is_empty())
        {
            entry.has_activation = true;
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

/// Folds attribute flags (STR..CHR and their DEC_ inverses) around a pval
/// into a modifier map; shared by egos (maximum roll) and artifacts (fixed).
fn attribute_modifiers_from_flags(
    flags: &[String],
    pval: i32,
    modifiers: &mut serde_json::Map<String, serde_json::Value>,
) {
    // Sentinel pvals (e.g. the legacy chaos shield's 125) exceed the contract
    // range; clamp into the ±100 attribute window as a documented ceiling.
    let pval = pval.clamp(-100, 100);
    for (flag, attribute) in ITEM_ATTRIBUTE_FLAGS {
        if pval != 0 && flags.iter().any(|value| value == flag) {
            modifiers.insert((*attribute).to_owned(), serde_json::json!(pval));
        }
        let dec_flag = format!("DEC_{flag}");
        if pval != 0 && flags.contains(&dec_flag) {
            modifiers.insert((*attribute).to_owned(), serde_json::json!(-pval));
        }
    }
}

fn attribute_flag_is_mapped(flag: &str) -> bool {
    ITEM_ATTRIBUTE_FLAGS
        .iter()
        .any(|(known, _)| *known == flag || flag.strip_prefix("DEC_") == Some(*known))
}

/// Object-flag resistance suffixes shared by base items, egos, artifacts
/// and race hooks, mapped to the content damage-type vocabulary. BLIND has no
/// damage type and stays in the gap reports.
const DEFENSIVE_RESISTANCE_TYPES: [(&str, &str); 16] = [
    ("ACID", "acid"),
    ("ELEC", "electricity"),
    ("FIRE", "fire"),
    ("COLD", "cold"),
    ("POIS", "poison"),
    ("LITE", "light"),
    ("DARK", "dark"),
    ("CONF", "confusion"),
    ("NETHER", "nether"),
    ("NEXUS", "nexus"),
    ("SOUND", "sound"),
    ("SHARDS", "shards"),
    ("CHAOS", "chaos"),
    ("DISEN", "disenchant"),
    ("TIME", "time"),
    ("FEAR", "fear"),
];

fn defensive_resistance_type(token: &str) -> Option<&'static str> {
    DEFENSIVE_RESISTANCE_TYPES
        .iter()
        .find(|(known, _)| *known == token)
        .map(|(_, damage_type)| *damage_type)
}

fn item_destruction_vulnerabilities(tval: u16) -> Vec<&'static str> {
    let mut elements = Vec::new();
    if matches!(
        tval,
        1 | 2
            | 3
            | 7
            | 17
            | 18
            | 19
            | 21
            | 22
            | 23
            | 30
            | 31
            | 32
            | 33
            | 34
            | 35
            | 36
            | 37
            | 38
            | 55
            | 70
            | 71
    ) {
        elements.push("acid");
    }
    if matches!(tval, 45 | 65) {
        elements.push("electricity");
    }
    if matches!(
        tval,
        7 | 17 | 19 | 22 | 23 | 30 | 31 | 35 | 36 | 39 | 55 | 70 | 71 | 90..=120
    ) {
        elements.push("fire");
    }
    if matches!(tval, 2 | 75 | 77) {
        elements.push("cold");
    }
    elements
}

fn item_destruction_immunities(flags: &[String]) -> Vec<&'static str> {
    [
        ("IGNORE_ACID", "acid"),
        ("IGNORE_ELEC", "electricity"),
        ("IGNORE_FIRE", "fire"),
        ("IGNORE_COLD", "cold"),
    ]
    .into_iter()
    .filter_map(|(flag, element)| {
        flags
            .iter()
            .any(|candidate| candidate == flag)
            .then_some(element)
    })
    .collect()
}

fn item_destruction_flag_is_mapped(flag: &str) -> bool {
    matches!(
        flag,
        "IGNORE_ACID" | "IGNORE_ELEC" | "IGNORE_FIRE" | "IGNORE_COLD"
    )
}

fn apply_item_destruction_properties(value: &mut serde_json::Value, tval: u16, flags: &[String]) {
    let vulnerabilities = item_destruction_vulnerabilities(tval);
    if !vulnerabilities.is_empty() {
        value["elementalDestructionVulnerabilities"] = serde_json::json!(vulnerabilities);
    }
    let immunities = item_destruction_immunities(flags);
    if !immunities.is_empty() {
        value["elementalDestructionImmunities"] = serde_json::json!(immunities);
    }
}

/// Display-only object flags with no Rewrite behaviour to express.
fn item_flag_not_applicable(flag: &str) -> bool {
    matches!(flag, "SHOW_MODS" | "HIDE_TYPE" | "FULL_NAME")
}

#[derive(Debug, Default)]
struct DefensiveFold {
    resistances: BTreeMap<&'static str, &'static str>,
    status_immunities: Vec<&'static str>,
    speed: i32,
    consumed: BTreeSet<String>,
}

/// Folds RES_/IM_/VULN_/FREE_ACT/SPEED object flags into the defensive
/// content surface. Ranked overwrite keeps the strongest tier per damage
/// type; SPEED consumes the pval budget like the attribute fold and stays
/// unexpressed (a visible gap) when the entry carries no pval.
fn defensive_fold(flags: &[String], pval: i32) -> DefensiveFold {
    let mut fold = DefensiveFold::default();
    let mut ranked: BTreeMap<&'static str, (u8, &'static str)> = BTreeMap::new();
    for flag in flags {
        if flag == "RES_FEAR" {
            fold.status_immunities.push("rfb.status.fear");
            fold.consumed.insert(flag.clone());
            continue;
        }
        if flag == "RES_BLIND" {
            fold.status_immunities.push("rfb.status.blindness");
            fold.consumed.insert(flag.clone());
            continue;
        }
        let (token, rank, level) = if let Some(suffix) = flag.strip_prefix("VULN_") {
            (suffix, 1_u8, "vulnerable")
        } else if let Some(suffix) = flag.strip_prefix("RES_") {
            (suffix, 2, "resistant")
        } else if let Some(suffix) = flag.strip_prefix("IM_") {
            (suffix, 3, "immune")
        } else if flag == "FREE_ACT" {
            fold.status_immunities = vec!["rfb.status.paralysis"];
            fold.consumed.insert(flag.clone());
            continue;
        } else if flag == "SPEED" {
            if pval != 0 {
                fold.speed = pval.clamp(-100, 100);
                fold.consumed.insert(flag.clone());
            }
            continue;
        } else {
            continue;
        };
        let Some(damage_type) = defensive_resistance_type(token) else {
            continue;
        };
        let entry = ranked.entry(damage_type).or_insert((rank, level));
        if rank > entry.0 {
            *entry = (rank, level);
        }
        fold.consumed.insert(flag.clone());
    }
    fold.resistances = ranked
        .into_iter()
        .map(|(damage_type, (_, level))| (damage_type, level))
        .collect();
    fold.status_immunities.sort_unstable();
    fold.status_immunities.dedup();
    fold
}

fn apply_defensive_fold(value: &mut serde_json::Value, fold: &DefensiveFold) {
    if !fold.resistances.is_empty() {
        value["resistances"] = serde_json::json!(fold.resistances);
    }
    if !fold.status_immunities.is_empty() {
        value["statusImmunities"] = serde_json::json!(fold.status_immunities);
    }
}

const OFFENSIVE_SLAY_TARGETS: [(&str, &str); 11] = [
    ("ANIMAL", "animal"),
    ("EVIL", "evil"),
    ("GOOD", "good"),
    ("LIVING", "living"),
    ("HUMAN", "human"),
    ("UNDEAD", "undead"),
    ("DEMON", "demon"),
    ("ORC", "orc"),
    ("TROLL", "troll"),
    ("GIANT", "giant"),
    ("DRAGON", "dragon"),
];

const OFFENSIVE_BRANDS: [(&str, &str); 6] = [
    ("BRAND_ACID", "acid"),
    ("BRAND_ELEC", "electricity"),
    ("BRAND_FIRE", "fire"),
    ("BRAND_COLD", "cold"),
    ("BRAND_POIS", "poison"),
    ("BRAND_CHAOS", "chaos"),
];

#[derive(Debug, Default)]
struct OffensiveFold {
    slays: BTreeMap<&'static str, &'static str>,
    brands: Vec<&'static str>,
    consumed: BTreeSet<String>,
}

fn offensive_fold(flags: &[String]) -> OffensiveFold {
    let mut fold = OffensiveFold::default();
    for (suffix, target) in OFFENSIVE_SLAY_TARGETS {
        let slay = format!("SLAY_{suffix}");
        let kill = format!("KILL_{suffix}");
        if flags.contains(&kill) {
            fold.slays.insert(target, "kill");
            fold.consumed.insert(kill);
            if flags.contains(&slay) {
                fold.consumed.insert(slay);
            }
        } else if flags.contains(&slay) {
            fold.slays.insert(target, "slay");
            fold.consumed.insert(slay);
        }
    }
    for (flag, brand) in OFFENSIVE_BRANDS {
        if flags.iter().any(|value| value == flag) {
            fold.brands.push(brand);
            fold.consumed.insert(flag.to_owned());
        }
    }
    fold.brands.sort_unstable();
    fold
}

fn apply_offensive_fold(value: &mut serde_json::Value, fold: &OffensiveFold) {
    if !fold.slays.is_empty() {
        value["slays"] = serde_json::json!(fold.slays);
    }
    if !fold.brands.is_empty() {
        value["brands"] = serde_json::json!(fold.brands);
    }
}

#[derive(Debug, Default)]
struct EquipmentFold {
    bonuses: serde_json::Map<String, serde_json::Value>,
    passives: Vec<&'static str>,
    consumed: BTreeSet<String>,
}

fn equipment_fold(flags: &[String], pval: i32) -> EquipmentFold {
    let mut fold = EquipmentFold::default();
    let pval = pval.clamp(-1_000_000, 1_000_000);
    for (flag, field) in [
        ("BLOWS", "meleeAttacks"),
        ("TUNNEL", "diggingSkill"),
        ("STEALTH", "stealthSkill"),
        ("SEARCH", "searchSkill"),
        ("MAGIC_MASTERY", "deviceSkill"),
        ("INFRA", "infravision"),
        ("LITE", "lightRadius"),
    ] {
        if flags.iter().any(|value| value == flag) && (pval != 0 || flag == "LITE") {
            let amount = match flag {
                "BLOWS" | "LITE" => pval.clamp(-8, 8),
                "INFRA" => pval.clamp(-64, 64),
                _ => pval,
            };
            let amount = if flag == "LITE" && amount == 0 {
                1
            } else {
                amount
            };
            fold.bonuses
                .insert(field.to_owned(), serde_json::json!(amount));
            fold.consumed.insert(flag.to_owned());
        }
    }
    if flags.iter().any(|value| value == "DEC_STEALTH") && pval != 0 {
        fold.bonuses
            .insert("stealthSkill".to_owned(), serde_json::json!(-pval));
        fold.consumed.insert("DEC_STEALTH".to_owned());
    }
    for (flag, passive) in [
        ("REGEN", "regeneration"),
        ("SEE_INVIS", "see-invisible"),
        ("BRAND_VAMP", "vampiric"),
        ("HOLD_LIFE", "hold-life"),
        ("SUST_STR", "sustain-strength"),
        ("SUST_INT", "sustain-intelligence"),
        ("SUST_WIS", "sustain-wisdom"),
        ("SUST_DEX", "sustain-dexterity"),
        ("SUST_CON", "sustain-constitution"),
        ("SUST_CHR", "sustain-charisma"),
    ] {
        if flags.iter().any(|value| value == flag) {
            fold.passives.push(passive);
            fold.consumed.insert(flag.to_owned());
        }
    }
    fold.passives.sort_unstable();
    fold
}

fn fold_spell_power_modifier(
    flags: &[String],
    pval: i32,
    modifiers: &mut serde_json::Map<String, serde_json::Value>,
    equipment: &mut EquipmentFold,
) {
    let pval = pval.clamp(-100, 100);
    let mut bonus = 0_i32;
    if flags.iter().any(|flag| flag == "SPELL_POWER") {
        bonus = bonus.saturating_add(pval);
        equipment.consumed.insert("SPELL_POWER".to_owned());
    }
    if flags.iter().any(|flag| flag == "DEC_SPELL_POWER") {
        bonus = bonus.saturating_sub(pval);
        equipment.consumed.insert("DEC_SPELL_POWER".to_owned());
    }
    if bonus != 0 {
        modifiers.insert("spellPowerBonus".to_owned(), serde_json::json!(bonus));
    }
}

fn add_equipment_bonus(fold: &mut EquipmentFold, field: &str, amount: i32) {
    if amount == 0 {
        return;
    }
    let current = fold
        .bonuses
        .get(field)
        .and_then(serde_json::Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .unwrap_or_default();
    fold.bonuses.insert(
        field.to_owned(),
        serde_json::json!(current.saturating_add(amount)),
    );
}

fn apply_equipment_fold(value: &mut serde_json::Value, fold: &EquipmentFold) {
    if !fold.bonuses.is_empty() {
        value["equipmentBonuses"] = serde_json::Value::Object(fold.bonuses.clone());
    }
    if !fold.passives.is_empty() {
        value["passives"] = serde_json::json!(fold.passives);
    }
}

/// Routes one legacy object flag to the right report bucket unless the
/// defensive or offensive fold already consumed it.
fn account_item_flag(
    flag: &str,
    fold: &DefensiveFold,
    offense: &OffensiveFold,
    equipment: &EquipmentFold,
    unmapped: &mut BTreeMap<String, usize>,
    not_applicable: &mut BTreeMap<String, usize>,
) {
    if fold.consumed.contains(flag)
        || offense.consumed.contains(flag)
        || equipment.consumed.contains(flag)
    {
        return;
    }
    if item_flag_not_applicable(flag) {
        *not_applicable.entry(flag.to_owned()).or_default() += 1;
        return;
    }
    *unmapped.entry(flag.to_owned()).or_default() += 1;
}

fn ego_json(
    entry: &LegacyEgoEntry,
    id: &str,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    // C: maxima become fixed modifiers unless a recipe below materializes the
    // original generation-time roll.
    let mut modifiers = serde_json::Map::new();
    let attack = entry.max_to_hit.max(entry.max_to_damage);
    if attack != 0 {
        modifiers.insert("attack".to_owned(), serde_json::json!(attack));
    }
    if entry.max_to_armor != 0 && entry.index != 50 {
        modifiers.insert("defense".to_owned(), serde_json::json!(entry.max_to_armor));
    }
    attribute_modifiers_from_flags(&entry.flags, entry.max_pval, &mut modifiers);
    // Defensive flags ride the same generation-time ceiling as attributes:
    // SPEED folds the max pval, resistances and free action are binary.
    let fold = defensive_fold(&entry.flags, entry.max_pval);
    let offense = offensive_fold(&entry.flags);
    let mut equipment = equipment_fold(&entry.flags, entry.max_pval);
    if fold.speed != 0 {
        modifiers.insert("speed".to_owned(), serde_json::json!(fold.speed));
    }
    fold_spell_power_modifier(&entry.flags, entry.max_pval, &mut modifiers, &mut equipment);
    if entry.has_activation {
        *report
            .item_behavior_gaps
            .entry("ego-activation".to_owned())
            .or_default() += 1;
    }
    for flag in &entry.flags {
        if attribute_flag_is_mapped(flag) {
            continue;
        }
        if ego_roll_recipe_consumes(entry, flag) {
            continue;
        }
        if item_destruction_flag_is_mapped(flag) {
            continue;
        }
        account_item_flag(
            flag,
            &fold,
            &offense,
            &equipment,
            &mut report.unmapped_ego_flags,
            &mut report.not_applicable_item_flags,
        );
    }
    let mut tags: Vec<String> = entry
        .slots
        .iter()
        .map(|slot| slot.to_ascii_lowercase().replace('_', "-"))
        .collect();
    tags.push("legacy-import".to_owned());
    tags.sort();
    tags.dedup();
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/affix.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.affix.{id}"),
        "nameKey": format!("affix-legacy-{id}-name"),
        "descriptionKey": format!("affix-legacy-{id}-description"),
        "generationLevel": entry.level,
        "tags": tags,
    });
    if let Some(max_level) = entry.max_level {
        value["generationMaxLevel"] = serde_json::json!(max_level);
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    apply_ego_roll_recipe(&mut value, entry);
    let destruction_immunities = item_destruction_immunities(&entry.flags);
    if !destruction_immunities.is_empty() {
        value["elementalDestructionImmunities"] = serde_json::json!(destruction_immunities);
    }
    if entry.index == 184 && entry.name == "of Endurance" {
        value["resistsMonsterDestruction"] = serde_json::json!(true);
        value["resistsProjectionDestruction"] = serde_json::json!(true);
    }
    value
}

fn ego_roll_recipe_consumes(entry: &LegacyEgoEntry, flag: &str) -> bool {
    matches!(entry.index, 148 | 209) && flag == "SPEED"
}

fn apply_ego_roll_recipe(value: &mut serde_json::Value, entry: &LegacyEgoEntry) {
    let groups = match entry.index {
        // Original `_ego_create_weapon_slaying`: weighted target choice with
        // rare kill upgrades. The original roll count is level-scaled; this
        // first content recipe fixes it at two while retaining depth filters
        // and relative target/upgrade rarity.
        1 => vec![serde_json::json!({
            "rolls": 2,
            "candidates": slaying_roll_candidates(),
        })],
        // Original craft rolls one of five elemental brands and sometimes
        // grants the matching resistance.
        9 => vec![serde_json::json!({
            "rolls": 2,
            "candidates": elemental_craft_roll_candidates(),
        })],
        // Original `of Protection` rolls a uniform +1..+10 armor bonus.
        50 => vec![serde_json::json!({
            "rolls": 1,
            "candidates": (1..=10)
                .map(|defense| serde_json::json!({
                    "weight": 1,
                    "properties": {"modifiers": {"defense": defense}}
                }))
                .collect::<Vec<_>>(),
        })],
        // Original boots/ring speed pvals are depth-biased generation rolls.
        // Increasing minDepth thresholds keep high bonuses out of shallow
        // instances while preserving the materialized per-item value.
        148 | 209 => vec![serde_json::json!({
            "rolls": 1,
            "candidates": speed_roll_candidates(entry.index == 209),
        })],
        // `of Combat` jewelry rolls fighting attributes, accuracy, damage,
        // or fear resistance. Three materialized rolls preserve that mix for
        // the Orc Cave guardian reward without adding an item-only runtime.
        206 => vec![serde_json::json!({
            "rolls": 3,
            "candidates": combat_ring_roll_candidates(),
        })],
        _ => Vec::new(),
    };
    if !groups.is_empty() {
        value["rollGroups"] = serde_json::Value::Array(groups);
    }
}

fn combat_ring_roll_candidates() -> Vec<serde_json::Value> {
    vec![
        serde_json::json!({"weight": 10, "properties": {"modifiers": {"constitution": 1}}}),
        serde_json::json!({"weight": 10, "properties": {"modifiers": {"dexterity": 1}}}),
        serde_json::json!({"weight": 10, "properties": {"modifiers": {"strength": 1}}}),
        serde_json::json!({"weight": 20, "properties": {"equipmentBonuses": {"meleeSkill": 5}}}),
        serde_json::json!({"weight": 20, "properties": {"equipmentBonuses": {"meleeDamage": 5}}}),
        serde_json::json!({"weight": 20, "properties": {"equipmentBonuses": {"meleeSkill": 4, "meleeDamage": 4}}}),
        serde_json::json!({"weight": 10, "properties": {"statusImmunities": ["rfb.status.fear"]}}),
    ]
}

fn slaying_roll_candidates() -> Vec<serde_json::Value> {
    [
        ("orc", 2_u32, 20_u16),
        ("troll", 2, 30),
        ("giant", 2, 40),
        ("dragon", 3, 80),
        ("demon", 3, 90),
        ("undead", 3, 95),
        ("animal", 2, 60),
        ("human", 3, 50),
        ("evil", 5, u16::MAX),
        ("good", 5, u16::MAX),
        ("living", 20, u16::MAX),
    ]
    .into_iter()
    .flat_map(|(target, rarity, max_depth)| {
        let base = (255 / rarity).max(1);
        let scale = 400_u32;
        let kill_weight = base.saturating_mul(scale) / rarity.saturating_mul(rarity);
        let slay_weight = base.saturating_mul(scale).saturating_sub(kill_weight);
        [
            serde_json::json!({
                "weight": slay_weight.max(1),
                "maxDepth": max_depth,
                "properties": {"slays": {target: "slay"}}
            }),
            serde_json::json!({
                "weight": kill_weight.max(1),
                "maxDepth": max_depth,
                "properties": {"slays": {target: "kill"}}
            }),
        ]
    })
    .collect()
}

fn elemental_craft_roll_candidates() -> Vec<serde_json::Value> {
    ["acid", "electricity", "fire", "cold", "poison"]
        .into_iter()
        .flat_map(|element| {
            [
                serde_json::json!({
                    "weight": 2,
                    "properties": {"brands": [element]}
                }),
                serde_json::json!({
                    "weight": 1,
                    "properties": {
                        "brands": [element],
                        "resistances": {element: "resistant"}
                    }
                }),
            ]
        })
        .collect()
}

fn speed_roll_candidates(ring: bool) -> Vec<serde_json::Value> {
    let maximum = if ring { 12_i32 } else { 10_i32 };
    (1..=maximum)
        .map(|speed| {
            let min_depth = u16::try_from((speed - 1).saturating_mul(10)).unwrap_or(u16::MAX);
            serde_json::json!({
                "weight": u32::try_from(maximum - speed + 1).unwrap_or(1),
                "minDepth": min_depth,
                "properties": {"modifiers": {"speed": speed}}
            })
        })
        .collect()
}

fn artifact_json(
    entry: &LegacyArtifactEntry,
    id: &str,
    base_item_kind_id: Option<&str>,
    ammo: &LauncherAmmoIndex,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let shape = item_shape(entry.tval).expect("every tval resolves a shape");
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/item.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.item.artifact-{id}"),
        "nameKey": format!("item-legacy-artifact-{id}-name"),
        "descriptionKey": format!("item-legacy-artifact-{id}-description"),
        "glyph": "*",
        "generationLevel": entry.level,
        "weightTenthsPound": entry.weight_tenths_pound.max(1),
        "maxStack": 1,
        "baseValue": entry.base_value,
        "resistsProjectionDestruction": true,
        "tags": ["artifact", "legacy-import"],
    });
    if let Some(base_item_kind_id) = base_item_kind_id {
        value["artifactGeneration"] = serde_json::json!({
            "sourceIndex": entry.index,
            "baseItemKindId": base_item_kind_id,
            "rarityOneIn": entry.rarity_one_in,
            "instant": entry.flags.iter().any(|flag| flag == "INSTA_ART"),
        });
    }
    if let Some(slot) = shape.slot {
        value["equipmentSlot"] = serde_json::json!(slot);
    }
    if shape.melee && shape.slot == Some("weapon") {
        let (dice, sides) = entry.damage_dice.unwrap_or((1, 1));
        value["meleeProfile"] = serde_json::json!({
            "attacks": 1,
            "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
            "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
            "damageDice": dice.clamp(1, 100),
            "damageSides": sides.clamp(1, 10_000),
        });
    }
    if shape.launcher {
        let paired = launcher_ammunition_type(
            &LegacyItemEntry {
                sval: entry.sval,
                ..LegacyItemEntry::default()
            },
            ammo,
        );
        if let Some(ammunition_type) = paired
            && let Some(multiplier_percent) = entry.launcher_multiplier_percent
        {
            value["projectileProfile"] = serde_json::json!({
                "range": launcher_range(multiplier_percent),
                "damageMultiplierPercent": multiplier_percent,
                "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
                "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
                "ammunitionType": ammunition_type,
            });
        } else {
            // Fake bows (guns, harps): the slot and fixed bonuses stay, only
            // the ability to fire is lost; P: to-hit/to-damage would have
            // applied to shooting alone, so dropping them stays faithful.
            *report
                .item_behavior_gaps
                .entry("launcher-unpaired".to_owned())
                .or_default() += 1;
        }
    }
    // Artifacts carry fixed bonuses: armour folds into defense, the fixed
    // pval feeds the attribute flags and the defensive fold alike.
    let has_slot = shape.slot.is_some();
    let fold = if has_slot {
        defensive_fold(&entry.flags, entry.pval)
    } else {
        DefensiveFold::default()
    };
    let offense = if has_slot {
        offensive_fold(&entry.flags)
    } else {
        OffensiveFold::default()
    };
    let mut equipment = if has_slot {
        equipment_fold(&entry.flags, entry.pval)
    } else {
        EquipmentFold::default()
    };
    let mut modifiers = serde_json::Map::new();
    if has_slot {
        let defense = entry.armor_class.saturating_add(entry.to_armor);
        if defense != 0 {
            modifiers.insert("defense".to_owned(), serde_json::json!(defense));
        }
        if !shape.melee && !shape.launcher {
            let attack = entry.to_hit.max(entry.to_damage);
            if attack != 0 {
                modifiers.insert("attack".to_owned(), serde_json::json!(attack));
            }
        }
        attribute_modifiers_from_flags(&entry.flags, entry.pval, &mut modifiers);
        if fold.speed != 0 {
            modifiers.insert("speed".to_owned(), serde_json::json!(fold.speed));
        }
        fold_spell_power_modifier(&entry.flags, entry.pval, &mut modifiers, &mut equipment);
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    apply_item_destruction_properties(&mut value, entry.tval, &entry.flags);
    if entry.has_activation {
        *report
            .item_behavior_gaps
            .entry("artifact-activation".to_owned())
            .or_default() += 1;
    }
    for flag in &entry.flags {
        // Slotless shapes never applied the attribute or defensive folds,
        // so their flags stay visible in the gap report.
        if (has_slot && attribute_flag_is_mapped(flag))
            || flag == "INSTA_ART"
            || item_destruction_flag_is_mapped(flag)
        {
            continue;
        }
        account_item_flag(
            flag,
            &fold,
            &offense,
            &equipment,
            &mut report.unmapped_artifact_flags,
            &mut report.not_applicable_item_flags,
        );
    }
    value
}

/// Parses b_info body templates: `N:index:Name` then `S:TYPE:Label`
/// slot lines in template order.
pub fn parse_b_info(text: &str) -> Result<Vec<LegacyBodyTemplate>, LegacyImportError> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyBodyTemplate> = None;
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parse_number(B_INFO_SOURCE, line_number, "N.index", parts.next())?;
            let name =
                required_field(B_INFO_SOURCE, line_number, "N.name", parts.next())?.to_owned();
            current = Some(LegacyBodyTemplate {
                index,
                name,
                ..LegacyBodyTemplate::default()
            });
        } else if let Some(rest) = line.strip_prefix("S:") {
            let entry = current.as_mut().ok_or_else(|| {
                content_parse_error(
                    B_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                )
            })?;
            let parts = rest.split(':').map(str::trim).collect::<Vec<_>>();
            if !(2..=3).contains(&parts.len()) {
                return Err(content_parse_error(
                    B_INFO_SOURCE,
                    line_number,
                    "S",
                    rest,
                    format!("expected 2 or 3 fields, found {}", parts.len()),
                ));
            }
            let token = required_field(
                B_INFO_SOURCE,
                line_number,
                "S.token",
                parts.first().copied(),
            )?;
            required_field(B_INFO_SOURCE, line_number, "S.label", parts.get(1).copied())?;
            if parts.len() == 3 {
                let _: u16 = parse_number(
                    B_INFO_SOURCE,
                    line_number,
                    "S.ordinal",
                    parts.get(2).copied(),
                )?;
            }
            entry.slots.push(token.to_owned());
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

/// Extracts the body of the function that starts at `header_pos` by brace
/// balancing from the first `{{` after the header.
fn function_block(text: &str, header_pos: usize) -> Option<&str> {
    let open = text[header_pos..].find('{')? + header_pos;
    let mut depth = 0_usize;
    for (offset, ch) in text[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(&text[open..=open + offset]);
                }
            }
            _ => {}
        }
    }
    None
}

/// Finds `race_t *X_get_race(...)` definitions (not prototypes or call
/// sites) and returns `(snake_case_name, body)` pairs.
pub fn extract_race_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    for (position, _) in text.match_indices("_get_race(") {
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if !line.contains("race_t *") || line.contains(';') {
            continue;
        }
        let name_end = position;
        let name_start = text[line_start..name_end]
            .rfind(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'))
            .map_or(line_start, |index| line_start + index + 1);
        let name = &text[name_start..name_end];
        if name.is_empty() {
            continue;
        }
        if let Some(body) = function_block(text, position) {
            blocks.push((name.to_owned(), body.to_owned()));
        }
    }
    blocks
}

/// Finds the definition body of a `void <name>(...)` helper in the same
/// translation unit (hook functions assigned to `me.calc_bonuses`).
fn find_function_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    for (position, _) in text.match_indices(name) {
        if !text[position + name.len()..].starts_with('(') {
            continue;
        }
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if !line.contains("void") || line.contains(';') {
            continue;
        }
        if text[line_start..position]
            .bytes()
            .next_back()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            continue;
        }
        return function_block(text, position);
    }
    None
}

fn find_power_info_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    for (position, _) in text.match_indices(name) {
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if line.contains("power_info") && line.contains("[]") {
            return function_block(text, position);
        }
    }
    None
}

/// Extracts the statically expressible defensive surface from a race's
/// calc_bonuses hook: top-level `res_add` family calls, `free_act++`,
/// `see_inv++`, unconditional attribute sustains and literal `pspeed`
/// adjustments. Conditional (level-gated) and computed statements are ignored
/// and remain accounted as hook gaps.
pub fn parse_calc_bonuses_defenses(
    text: &str,
    hook: &str,
) -> (Vec<(String, String)>, bool, bool, Vec<String>, i32) {
    fn resistance_token(rest: &str) -> Option<&'static str> {
        let token = rest[..rest.find(')')?].trim();
        defensive_resistance_type(token.strip_prefix("RES_")?)
    }
    let Some(body) = find_function_body(text, hook) else {
        return (Vec::new(), false, false, Vec::new(), 0);
    };
    let mut adds: BTreeMap<&'static str, i32> = BTreeMap::new();
    let mut immune: BTreeSet<&'static str> = BTreeSet::new();
    let mut free_act = false;
    let mut see_invisible = false;
    let mut attribute_sustains = BTreeSet::new();
    let mut speed = 0_i32;
    let mut depth = 0_i32;
    let mut suppressed = false;
    for raw_line in body.lines() {
        let line = raw_line.trim();
        let depth_at_start = depth;
        depth += i32::try_from(line.matches('{').count()).unwrap_or(0);
        depth -= i32::try_from(line.matches('}').count()).unwrap_or(0);
        if depth_at_start != 1 || line.is_empty() {
            continue;
        }
        if line.starts_with("/*") || line.starts_with('*') || line.starts_with("//") {
            continue;
        }
        if suppressed {
            // A braceless conditional consumes the whole following
            // statement, however many lines it spans.
            if line.ends_with(';') || line.contains('{') {
                suppressed = false;
            }
            continue;
        }
        let keyword_guard = ["if", "else", "for", "while", "switch", "do"]
            .iter()
            .any(|keyword| {
                line == *keyword
                    || line.starts_with(&format!("{keyword} "))
                    || line.starts_with(&format!("{keyword}("))
            });
        if keyword_guard {
            if !line.contains('{') && !line.ends_with(';') {
                suppressed = true;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("res_add_immune(") {
            if let Some(damage_type) = resistance_token(rest) {
                immune.insert(damage_type);
            }
        } else if let Some(rest) = line.strip_prefix("res_add_vuln(") {
            if let Some(damage_type) = resistance_token(rest) {
                *adds.entry(damage_type).or_default() -= 1;
            }
        } else if let Some(rest) = line.strip_prefix("res_add(") {
            if let Some(damage_type) = resistance_token(rest) {
                *adds.entry(damage_type).or_default() += 1;
            }
        } else if line == "p_ptr->free_act++;" {
            free_act = true;
        } else if line == "p_ptr->see_inv++;" {
            see_invisible = true;
        } else if let Some(attribute) = line
            .strip_prefix("p_ptr->sustain_")
            .and_then(|line| line.strip_suffix(" = TRUE;"))
            .and_then(|attribute| match attribute {
                "str" => Some("strength"),
                "int" => Some("intelligence"),
                "wis" => Some("wisdom"),
                "dex" => Some("dexterity"),
                "con" => Some("constitution"),
                "chr" => Some("charisma"),
                _ => None,
            })
        {
            attribute_sustains.insert(attribute.to_owned());
        } else if let Some(rest) = line.strip_prefix("p_ptr->pspeed") {
            let rest = rest.trim_start();
            let (sign, tail) = if let Some(tail) = rest.strip_prefix("+=") {
                (1, tail)
            } else if let Some(tail) = rest.strip_prefix("-=") {
                (-1, tail)
            } else {
                continue;
            };
            // Only literal adjustments are static; level-scaled speed
            // stays a hook gap.
            if let Ok(amount) = tail.trim().trim_end_matches(';').trim().parse::<i32>() {
                speed += sign * amount;
            }
        }
    }
    let mut resistances = Vec::new();
    for damage_type in adds
        .keys()
        .copied()
        .chain(immune.iter().copied())
        .collect::<BTreeSet<_>>()
    {
        let level = if immune.contains(damage_type) {
            "immune"
        } else {
            match adds.get(damage_type).copied().unwrap_or(0) {
                i32::MIN..=-1 => "vulnerable",
                0 => continue,
                1 => "resistant",
                _ => "strong",
            }
        };
        resistances.push((damage_type.to_owned(), level.to_owned()));
    }
    (
        resistances,
        free_act,
        see_invisible,
        attribute_sustains.into_iter().collect(),
        speed,
    )
}

/// Finds `personality_ptr _get_X_personality(...)` definitions.
pub fn extract_personality_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    for (position, _) in text.match_indices("_personality(") {
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if !line.contains("personality_ptr") || line.contains(';') {
            continue;
        }
        let Some(get_index) = line.find("_get_") else {
            continue;
        };
        let name_start = line_start + get_index + "_get_".len();
        if name_start >= position {
            continue;
        }
        let name = &text[name_start..position];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            continue;
        }
        if let Some(body) = function_block(text, position) {
            blocks.push((name.to_owned(), body.to_owned()));
        }
    }
    blocks
}

const CHARACTER_STAT_KEYS: [&str; 6] = [
    "stats[A_STR]",
    "stats[A_INT]",
    "stats[A_WIS]",
    "stats[A_DEX]",
    "stats[A_CON]",
    "stats[A_CHR]",
];

const CHARACTER_SKILL_KEYS: [&str; 8] = ["dis", "dev", "sav", "stl", "srh", "fos", "thn", "thb"];

/// Parses the regular `me.<field> = <value>;` assignment block shared by
/// player races and personalities. Computed scalars (e.g. `100 + 5*rank`)
/// mark the entry as dynamic, which excludes it from static import.
pub fn parse_character_block(name: &str, body: &str) -> LegacyCharacterEntry {
    let mut entry = LegacyCharacterEntry {
        id: name.replace('_', "-"),
        ..LegacyCharacterEntry::default()
    };
    for raw_line in body.lines() {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("me.") else {
            continue;
        };
        let Some(eq_index) = rest.find('=') else {
            continue;
        };
        let lhs = rest[..eq_index].trim_end();
        if lhs.ends_with('+') || lhs.ends_with('-') || lhs.ends_with('*') || lhs.ends_with('/') {
            entry.hooks.push("dynamic-adjustment".to_owned());
            continue;
        }
        let rhs = rest[eq_index + 1..]
            .split("/*")
            .next()
            .expect("split always yields the leading assignment")
            .trim()
            .trim_end_matches(';')
            .trim_end();
        let literal: Option<i32> = rhs.parse().ok();
        if let Some(position) = CHARACTER_STAT_KEYS.iter().position(|key| *key == lhs) {
            match literal {
                Some(value) => entry.stats[position] = value,
                None => entry.dynamic = true,
            }
        } else if let Some(suffix) = lhs.strip_prefix("skills.") {
            match (
                CHARACTER_SKILL_KEYS.iter().position(|key| *key == suffix),
                literal,
            ) {
                (Some(position), Some(value)) => entry.skills[position] = value,
                _ => entry.dynamic = true,
            }
        } else if let Some(suffix) = lhs.strip_prefix("extra_skills.") {
            match (
                CHARACTER_SKILL_KEYS.iter().position(|key| *key == suffix),
                literal,
            ) {
                (Some(position), Some(value)) => entry.extra_skills[position] = value,
                _ => entry.dynamic = true,
            }
        } else {
            match lhs {
                "name" => {
                    entry.name = serde_json::from_str(rhs).unwrap_or_default();
                }
                "desc" => {
                    entry.description = serde_json::from_str(rhs).unwrap_or_default();
                }
                "subname" | "subdesc" => {}
                "shop_adjust" => match literal {
                    Some(value) => entry.shop_adjust = value,
                    None => entry.dynamic = true,
                },
                "life" => match literal {
                    Some(value) => entry.life = value,
                    None => entry.dynamic = true,
                },
                "base_hp" => match literal {
                    Some(value) => entry.base_hp = value,
                    None => entry.dynamic = true,
                },
                "exp" => match literal {
                    Some(value) => entry.exp = value,
                    None => entry.dynamic = true,
                },
                "infra" => match literal {
                    Some(value) => entry.infra = value,
                    None => entry.dynamic = true,
                },
                "flags" => {
                    entry.flags = rhs
                        .split('|')
                        .map(str::trim)
                        .filter(|token| !token.is_empty())
                        .map(str::to_owned)
                        .collect()
                }
                "skills" | "stats" | "extra_skills" => entry.dynamic = true,
                other => {
                    if other == "calc_bonuses"
                        && !rhs.is_empty()
                        && rhs.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                        })
                    {
                        entry.calc_bonuses_fn = Some(rhs.to_owned());
                    }
                    if other == "get_powers"
                        && !rhs.is_empty()
                        && rhs.bytes().all(|byte| {
                            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                        })
                    {
                        entry.get_powers_fn = Some(rhs.to_owned());
                    }
                    entry.hooks.push(other.to_owned());
                }
            }
        }
    }
    entry
}

fn parse_race_powers(text: &str, entry: &mut LegacyCharacterEntry) {
    let Some(table_name) = entry.get_powers_fn.as_deref() else {
        return;
    };
    let Some(body) = find_power_info_body(text, table_name) else {
        return;
    };
    let mut saw_literal_table = false;
    let mut powers = Vec::new();
    let mut gaps = Vec::new();
    for raw_line in body.lines() {
        let line = raw_line.trim();
        if !line.starts_with('{') || !line.contains(',') {
            continue;
        }
        let tokens = line
            .split(|character: char| {
                character.is_ascii_whitespace() || matches!(character, '{' | '}' | ',')
            })
            .filter(|token| !token.is_empty())
            .collect::<Vec<_>>();
        saw_literal_table = true;
        if tokens.first() == Some(&"-1") || tokens.last() == Some(&"NULL") {
            continue;
        }
        if tokens.len() != 5 {
            gaps.push("get_powers:non-literal".to_owned());
            continue;
        }
        let [attribute, level, cost, failure, spell] =
            [tokens[0], tokens[1], tokens[2], tokens[3], tokens[4]];
        let governing_attribute = match attribute {
            "A_STR" => "strength",
            "A_INT" => "intelligence",
            "A_WIS" => "wisdom",
            "A_DEX" => "dexterity",
            "A_CON" => "constitution",
            "A_CHR" => "charisma",
            _ => {
                gaps.push(format!("get_powers:{spell}"));
                continue;
            }
        };
        let (Ok(minimum_level), Ok(cost), Ok(base_failure_percent)) =
            (level.parse(), cost.parse(), failure.parse())
        else {
            gaps.push(format!("get_powers:{spell}"));
            continue;
        };
        let ability_id = match spell {
            "berserk_spell" => "rfb.ability.race.berserk",
            "create_food_spell" => "rfb.ability.race.create-food",
            "detect_doors_stairs_traps_spell" => "rfb.ability.race.detect-doors-stairs-traps",
            "detect_treasure_spell" => "rfb.ability.race.detect-treasure",
            "phase_door_spell" => "rfb.ability.race.phase-door",
            "poison_dart_spell" => "rfb.ability.race.poison-dart",
            _ => {
                gaps.push(format!("get_powers:{spell}"));
                continue;
            }
        };
        powers.push(LegacyInnatePower {
            governing_attribute: governing_attribute.to_owned(),
            minimum_level,
            cost,
            base_failure_percent,
            ability_id: ability_id.to_owned(),
        });
    }
    if saw_literal_table {
        entry.hooks.retain(|hook| hook != "get_powers");
        entry.abilities = powers;
        gaps.sort();
        gaps.dedup();
        entry.hooks.extend(gaps);
    }
}

/// Parses the selectable class registry by joining numeric `CLASS_*`
/// definitions with the single dispatch switch in `classes.c`.
pub fn parse_class_registrations(defines: &str, classes: &str) -> Vec<LegacyClassRegistration> {
    let indices = defines
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            if parts.next()? != "#define" {
                return None;
            }
            let token = parts.next()?;
            let index = parts.next()?.parse::<u16>().ok()?;
            token
                .starts_with("CLASS_")
                .then(|| (token.to_owned(), index))
        })
        .collect::<BTreeMap<_, _>>();
    let mut registrations = Vec::new();
    let mut current_token: Option<&str> = None;
    for raw_line in classes.lines() {
        let line = raw_line.trim();
        if let Some(token) = line
            .strip_prefix("case ")
            .and_then(|line| line.strip_suffix(':'))
            .filter(|token| token.starts_with("CLASS_"))
        {
            current_token = Some(token);
            continue;
        }
        let Some(token) = current_token else {
            continue;
        };
        let Some(rest) = line.strip_prefix("result = ") else {
            continue;
        };
        let Some(paren) = rest.find('(') else {
            continue;
        };
        let function = rest[..paren].trim();
        if function.is_empty()
            || !function
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        {
            continue;
        }
        if let Some(index) = indices.get(token) {
            registrations.push(LegacyClassRegistration {
                index: *index,
                id: token
                    .trim_start_matches("CLASS_")
                    .to_ascii_lowercase()
                    .replace('_', "-"),
                function: function.to_owned(),
                registered: true,
            });
        }
        current_token = None;
    }
    registrations.sort_by_key(|entry| entry.index);
    registrations
}

/// Finds one canonical `class_t *foo_get_class(...)` definition.
pub fn extract_class_block<'a>(text: &'a str, function: &str) -> Option<&'a str> {
    for (position, _) in text.match_indices(function) {
        if !text[position + function.len()..].starts_with('(') {
            continue;
        }
        if text[..position]
            .bytes()
            .next_back()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            continue;
        }
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if !line.contains("class_t *") || line.contains(';') {
            continue;
        }
        return function_block(text, position);
    }
    None
}

fn assignment_value<'a>(body: &'a str, field: &str) -> Option<&'a str> {
    let marker = format!("me.{field}");
    for line in body.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix(&marker) else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let rest = rest.trim();
        return Some(rest.trim_end_matches(';').trim());
    }
    None
}

fn parse_skill_initializer(body: &str, variable: &str) -> Option<[i32; 8]> {
    let marker = format!("skills_t {variable}");
    let start = body.find(&marker)?;
    let open = body[start..].find('{')? + start;
    let close = body[open..].find('}')? + open;
    let values = body[open + 1..close]
        .split(',')
        .map(str::trim)
        .map(str::parse::<i32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    values.try_into().ok()
}

fn parse_assigned_skill_set(body: &str, field: &str) -> Option<[i32; 8]> {
    let variable = assignment_value(body, field)?;
    if variable.is_empty()
        || !variable
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return None;
    }
    parse_skill_initializer(body, variable)
}

fn parse_multiline_flags(body: &str) -> Vec<String> {
    parse_multiline_assignment(body, "me.flags")
        .unwrap_or_default()
        .split('|')
        .map(str::trim)
        .filter(|token| {
            !token.is_empty()
                && *token != "0"
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        })
        .map(str::to_owned)
        .collect()
}

fn parse_multiline_assignment<'a>(body: &'a str, marker: &str) -> Option<&'a str> {
    let start = body.find(marker)?;
    let eq = body[start..].find('=')?;
    let value_start = start + eq + 1;
    let end = body[value_start..].find(';')?;
    Some(body[value_start..value_start + end].trim())
}

fn extract_caster_block<'a>(text: &'a str, function: &str) -> Option<&'a str> {
    for (position, _) in text.match_indices(function) {
        if !text[position + function.len()..].starts_with('(') {
            continue;
        }
        if text[..position]
            .bytes()
            .next_back()
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            continue;
        }
        let line_start = text[..position].rfind('\n').map_or(0, |index| index + 1);
        let line_end = text[position..]
            .find('\n')
            .map_or(text.len(), |index| position + index);
        let line = &text[line_start..line_end];
        if !line.contains("caster_info") || line.contains(';') {
            continue;
        }
        return function_block(text, position);
    }
    None
}

pub fn parse_class_caster_profile(source: &str, class_body: &str) -> Option<LegacyCasterProfile> {
    let function = assignment_value(class_body, "caster_info")?;
    let body = extract_caster_block(source, function)?;
    let casting_attribute = assignment_value(body, "which_stat")
        .and_then(|value| value.strip_prefix("A_"))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let parse_i32 = |field: &str| {
        assignment_value(body, field)
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(0)
    };
    let options = parse_multiline_assignment(body, "me.options")
        .unwrap_or_default()
        .split('|')
        .map(str::trim)
        .filter(|token| {
            !token.is_empty()
                && *token != "0"
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
        })
        .map(|token| {
            token
                .trim_start_matches("CASTER_")
                .to_ascii_lowercase()
                .replace('_', "-")
        })
        .collect();
    Some(LegacyCasterProfile {
        dynamic: casting_attribute.is_empty(),
        casting_attribute,
        minimum_failure_percent: u8::try_from(parse_i32("min_fail").clamp(0, 100)).unwrap_or(0),
        minimum_level: u16::try_from(parse_i32("min_level").max(0)).unwrap_or(0),
        max_encumbrance_weight: parse_i32("encumbrance.max_wgt"),
        weapon_encumbrance_percent: parse_i32("encumbrance.weapon_pct"),
        zero_mana_encumbrance_weight: parse_i32("encumbrance.enc_wgt"),
        options,
    })
}

/// Parses the static class shell. Subclass- and allocation-dependent values
/// remain zero/default and are reported as dynamic class gaps.
pub fn parse_class_block(registration: LegacyClassRegistration, body: &str) -> LegacyClassEntry {
    let mut character = parse_character_block(&registration.id, body);
    if let Some(skills) = parse_assigned_skill_set(body, "base_skills") {
        character.skills = skills;
    } else if body.contains("me.base_skills") || body.contains("skills_init(&me.base_skills)") {
        character.hooks.push("dynamic-base-skills".to_owned());
    }
    character.hooks.retain(|hook| hook != "base_skills");
    if let Some(skills) = parse_assigned_skill_set(body, "extra_skills") {
        character.extra_skills = skills;
    } else if body.contains("me.extra_skills") || body.contains("skills_init(&me.extra_skills)") {
        character.hooks.push("dynamic-extra-skills".to_owned());
    }
    character.hooks.retain(|hook| hook != "extra_skills");
    if !body.contains("static class_t me") {
        character.hooks.push("dynamic-class-delegation".to_owned());
    }
    if body.contains("me.flags") {
        character.flags = parse_multiline_flags(body);
    }
    character.hooks.sort();
    character.hooks.dedup();
    character.dynamic = character.hooks.iter().any(|hook| {
        matches!(
            hook.as_str(),
            "dynamic-adjustment"
                | "dynamic-base-skills"
                | "dynamic-class-delegation"
                | "dynamic-extra-skills"
        )
    }) || body.lines().any(|raw_line| {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix("me.") else {
            return false;
        };
        let Some(eq_index) = rest.find('=') else {
            return false;
        };
        let lhs = rest[..eq_index].trim_end();
        let scalar = CHARACTER_STAT_KEYS.contains(&lhs)
            || matches!(lhs, "life" | "base_hp" | "exp" | "infra")
            || lhs.starts_with("stats[");
        scalar
            && rest[eq_index + 1..]
                .trim()
                .trim_end_matches(';')
                .trim()
                .parse::<i32>()
                .is_err()
    });
    LegacyClassEntry {
        registration,
        character,
        caster_profile: None,
        pet_upkeep_divisor: assignment_value(body, "pets")
            .and_then(|value| value.parse::<u16>().ok())
            .unwrap_or(40),
        source_found: true,
    }
}

const LEGACY_REALM_IDS: [&str; 12] = [
    "life",
    "sorcery",
    "nature",
    "chaos",
    "death",
    "trump",
    "arcane",
    "craft",
    "daemon",
    "crusade",
    "necromancy",
    "armageddon",
];

fn parse_prefixed_u32(
    source: &'static str,
    line: usize,
    field: &'static str,
    value: Option<&str>,
) -> Result<u32, LegacyImportError> {
    let value = required_field(source, line, field, value)?;
    if let Some(hex) = value.strip_prefix("0x") {
        return u32::from_str_radix(hex, 16).map_err(|error| {
            content_parse_error(
                source,
                line,
                field,
                value,
                format!("invalid number: {error}"),
            )
        });
    }
    parse_number(source, line, field, Some(value))
}

/// Parses the complete per-class realm readability and spell parameter
/// matrix from `m_info.txt`.
pub fn parse_m_info(text: &str) -> Result<Vec<LegacyMagicProfile>, LegacyImportError> {
    let mut profiles = Vec::new();
    let mut current: Option<LegacyMagicProfile> = None;
    let mut current_realm: Option<usize> = None;
    let mut pending_name = String::new();
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.trim();
        if let Some(name) = line
            .strip_prefix("### ")
            .and_then(|line| line.strip_suffix(" ###"))
        {
            pending_name = name.trim().to_owned();
            continue;
        }
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(profile) = current.take() {
                profiles.push(profile);
            }
            let class_index = parse_number(M_INFO_SOURCE, line_number, "N.classIndex", Some(rest))?;
            current = Some(LegacyMagicProfile {
                class_index,
                name_hint: std::mem::take(&mut pending_name),
                ..LegacyMagicProfile::default()
            });
            current_realm = None;
            continue;
        }
        let recognized = ["I:", "R:", "T:"]
            .iter()
            .any(|prefix| line.starts_with(prefix));
        let profile = match current.as_mut() {
            Some(profile) => profile,
            None if recognized => {
                return Err(content_parse_error(
                    M_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                ));
            }
            None => continue,
        };
        if let Some(rest) = line.strip_prefix("I:") {
            let parts = parse_fields(M_INFO_SOURCE, line_number, "I", rest, 6)?;
            profile.book_type = required_field(
                M_INFO_SOURCE,
                line_number,
                "I.bookType",
                parts.first().copied(),
            )?
            .to_ascii_lowercase();
            profile.casting_attribute = required_field(
                M_INFO_SOURCE,
                line_number,
                "I.castingAttribute",
                parts.get(1).copied(),
            )?
            .to_ascii_lowercase();
            profile.extra_flags = parse_prefixed_u32(
                M_INFO_SOURCE,
                line_number,
                "I.extraFlags",
                parts.get(2).copied(),
            )?;
            profile.spell_type = parse_number(
                M_INFO_SOURCE,
                line_number,
                "I.spellType",
                parts.get(3).copied(),
            )?;
            profile.first_spell_level = parse_number(
                M_INFO_SOURCE,
                line_number,
                "I.firstSpellLevel",
                parts.get(4).copied(),
            )?;
            profile.spell_weight = parse_number(
                M_INFO_SOURCE,
                line_number,
                "I.spellWeight",
                parts.get(5).copied(),
            )?;
        } else if let Some(rest) = line.strip_prefix("R:") {
            let parts = parse_fields(M_INFO_SOURCE, line_number, "R", rest, 2)?;
            let realm_index: u8 = parse_number(
                M_INFO_SOURCE,
                line_number,
                "R.realmIndex",
                parts.first().copied(),
            )?;
            if usize::from(realm_index) >= LEGACY_REALM_IDS.len() {
                return Err(content_parse_error(
                    M_INFO_SOURCE,
                    line_number,
                    "R.realmIndex",
                    parts[0],
                    format!("realm index must be less than {}", LEGACY_REALM_IDS.len()),
                ));
            }
            let readable: u16 = parse_number(
                M_INFO_SOURCE,
                line_number,
                "R.readable",
                parts.get(1).copied(),
            )?;
            profile.realms.push(LegacyRealmProfile {
                index: realm_index,
                readable: readable != 0,
                spells: Vec::new(),
            });
            current_realm = Some(profile.realms.len() - 1);
        } else if let Some(rest) = line.strip_prefix("T:") {
            let realm_index = current_realm.ok_or_else(|| {
                content_parse_error(
                    M_INFO_SOURCE,
                    line_number,
                    "T.realm",
                    rest,
                    "spell entry appears before the first R record",
                )
            })?;
            let values = rest.split('#').next().unwrap_or(rest).trim_end();
            let parts = parse_fields(M_INFO_SOURCE, line_number, "T", values, 4)?;
            let level = parse_number(
                M_INFO_SOURCE,
                line_number,
                "T.level",
                parts.first().copied(),
            )?;
            let mana = parse_number(M_INFO_SOURCE, line_number, "T.mana", parts.get(1).copied())?;
            let failure_percent = parse_number(
                M_INFO_SOURCE,
                line_number,
                "T.failurePercent",
                parts.get(2).copied(),
            )?;
            let experience = parse_number(
                M_INFO_SOURCE,
                line_number,
                "T.experience",
                parts.get(3).copied(),
            )?;
            if profile.realms[realm_index].readable {
                let spell_index =
                    u8::try_from(profile.realms[realm_index].spells.len()).map_err(|_| {
                        content_parse_error(
                            M_INFO_SOURCE,
                            line_number,
                            "T.index",
                            profile.realms[realm_index].spells.len().to_string(),
                            "realm has more than 256 spell entries",
                        )
                    })?;
                profile.realms[realm_index].spells.push(LegacySpellProfile {
                    index: spell_index,
                    level,
                    mana,
                    failure_percent,
                    experience,
                });
            }
        }
    }
    if let Some(profile) = current.take() {
        profiles.push(profile);
    }
    Ok(profiles)
}

/// Parses the per-class weapon and miscellaneous proficiency rows.
pub fn parse_s_info(text: &str) -> Result<Vec<LegacyProficiencyProfile>, LegacyImportError> {
    let mut profiles = Vec::new();
    let mut current: Option<LegacyProficiencyProfile> = None;
    for (line_index, raw_line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = raw_line.split('#').next().unwrap_or(raw_line).trim();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(profile) = current.take() {
                profiles.push(profile);
            }
            let class_index = parse_number(S_INFO_SOURCE, line_number, "N.classIndex", Some(rest))?;
            current = Some(LegacyProficiencyProfile {
                class_index,
                ..LegacyProficiencyProfile::default()
            });
        } else if let Some(rest) = line.strip_prefix("W:") {
            let profile = current.as_mut().ok_or_else(|| {
                content_parse_error(
                    S_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                )
            })?;
            let parts = parse_fields(S_INFO_SOURCE, line_number, "W", rest, 4)?;
            let entry = LegacyWeaponProficiencyEntry {
                weapon_type: parse_number(
                    S_INFO_SOURCE,
                    line_number,
                    "W.weaponType",
                    parts.first().copied(),
                )?,
                weapon_subtype: parse_number(
                    S_INFO_SOURCE,
                    line_number,
                    "W.weaponSubtype",
                    parts.get(1).copied(),
                )?,
                initial_rank: parse_number(
                    S_INFO_SOURCE,
                    line_number,
                    "W.minimum",
                    parts.get(2).copied(),
                )?,
                maximum_rank: parse_number(
                    S_INFO_SOURCE,
                    line_number,
                    "W.maximum",
                    parts.get(3).copied(),
                )?,
            };
            if entry.weapon_type > 4
                || entry.weapon_subtype > 63
                || entry.initial_rank > 4
                || entry.maximum_rank > 4
            {
                return Err(content_parse_error(
                    S_INFO_SOURCE,
                    line_number,
                    "W",
                    rest,
                    "weapon proficiency row is outside RFB limits",
                ));
            }
            profile.weapon_entries.push(entry);
        } else if let Some(rest) = line.strip_prefix("S:") {
            let profile = current.as_mut().ok_or_else(|| {
                content_parse_error(
                    S_INFO_SOURCE,
                    line_number,
                    "record",
                    line,
                    "structured field appears before the first N record",
                )
            })?;
            let parts = parse_fields(S_INFO_SOURCE, line_number, "S", rest, 3)?;
            let skill_index = parse_number(
                S_INFO_SOURCE,
                line_number,
                "S.skillIndex",
                parts.first().copied(),
            )?;
            let initial = parse_number(
                S_INFO_SOURCE,
                line_number,
                "S.minimum",
                parts.get(1).copied(),
            )?;
            let maximum = parse_number(
                S_INFO_SOURCE,
                line_number,
                "S.maximum",
                parts.get(2).copied(),
            )?;
            if initial > maximum || maximum > 8_000 {
                return Err(content_parse_error(
                    S_INFO_SOURCE,
                    line_number,
                    "S",
                    rest,
                    "miscellaneous proficiency row is outside RFB limits",
                ));
            }
            profile.skill_entries.push(LegacySkillProficiencyEntry {
                skill_index,
                initial,
                maximum,
            });
        }
    }
    if let Some(profile) = current.take() {
        profiles.push(profile);
    }
    Ok(profiles)
}

/// Legacy skill roster: (id suffix, kind, index into the skills array,
/// following the dis/dev/sav/stl/srh/fos/thn/thb legacy order).
const LEGACY_SKILL_ROSTER: [(&str, &str); 8] = [
    ("disarming", "disarming"),
    ("device", "device"),
    ("saving-throw", "saving-throw"),
    ("stealth", "stealth"),
    ("search", "search"),
    ("perception", "perception"),
    ("melee", "melee"),
    ("ranged", "ranged"),
];

/// Maps legacy body slot tokens to the RFB slot vocabulary: alternating
/// weapon/shield hands, launcher for BOW, and numbered instances when a
/// type repeats. Unrepresentable tokens land in the gap report.
fn map_body_slots(
    template: &LegacyBodyTemplate,
    gaps: &mut BTreeMap<String, usize>,
) -> Vec<(String, String)> {
    let mut mapped_types: Vec<String> = Vec::new();
    let mut weapon_shield_seen = 0_usize;
    for token in &template.slots {
        let slot_type = match token.as_str() {
            "WEAPON_SHIELD" => {
                weapon_shield_seen += 1;
                if weapon_shield_seen % 2 == 1 {
                    "weapon"
                } else {
                    "shield"
                }
            }
            "WEAPON" => "weapon",
            "BOW" => "launcher",
            "RING" => "ring",
            "AMULET" => "amulet",
            "LITE" => "light",
            "BODY_ARMOR" => "body",
            "CLOAK" => "cloak",
            "HELMET" => "head",
            "GLOVES" => "gloves",
            "BOOTS" => "boots",
            other => {
                *gaps
                    .entry(format!(
                        "body-slot-{}",
                        other.to_ascii_lowercase().replace('_', "-")
                    ))
                    .or_default() += 1;
                continue;
            }
        };
        mapped_types.push(slot_type.to_owned());
    }
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    for slot_type in &mapped_types {
        *totals.entry(slot_type.as_str()).or_default() += 1;
    }
    let mut ordinals: BTreeMap<&str, usize> = BTreeMap::new();
    mapped_types
        .iter()
        .map(|slot_type| {
            let ordinal = ordinals.entry(slot_type.as_str()).or_default();
            *ordinal += 1;
            let id = if totals[slot_type.as_str()] > 1 {
                format!("{slot_type}-{ordinal}")
            } else {
                slot_type.clone()
            };
            (id, slot_type.clone())
        })
        .collect()
}

fn character_modifiers(entry: &LegacyCharacterEntry) -> serde_json::Map<String, serde_json::Value> {
    let mut modifiers = serde_json::Map::new();
    for (value, attribute) in entry.stats.iter().zip([
        "strength",
        "intelligence",
        "wisdom",
        "dexterity",
        "constitution",
        "charisma",
    ]) {
        if *value != 0 {
            modifiers.insert(
                attribute.to_owned(),
                serde_json::json!(value.clamp(&-100, &100)),
            );
        }
    }
    if entry.speed != 0 {
        modifiers.insert(
            "speed".to_owned(),
            serde_json::json!(entry.speed.clamp(-100, 100)),
        );
    }
    modifiers
}

fn character_skill_set_json(entry: &LegacyCharacterEntry, id: &str) -> serde_json::Value {
    let entries: Vec<serde_json::Value> = LEGACY_SKILL_ROSTER
        .iter()
        .enumerate()
        .filter_map(|(index, (suffix, _))| {
            let base = entry.skills[index];
            let growth = entry.extra_skills[index];
            if base == 0 && growth == 0 {
                return None;
            }
            let mut value = serde_json::json!({
                "skillId": format!("rfb-legacy.skill.{suffix}"),
                "base": base,
            });
            if growth != 0 {
                value["growthPerTenLevels"] = serde_json::json!(growth);
            }
            Some(value)
        })
        .collect();
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/skill-set.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.skill-set.{id}"),
        "entries": entries,
    })
}

fn character_gap_accounting(entry: &LegacyCharacterEntry, report: &mut ContentImportReport) {
    for flag in &entry.flags {
        *report.unmapped_race_flags.entry(flag.clone()).or_default() += 1;
    }
    for hook in &entry.hooks {
        *report.race_hook_gaps.entry(hook.clone()).or_default() += 1;
    }
}

fn legacy_race_kin_glyph(id: &str) -> char {
    match id {
        "human" | "amberite" | "barbarian" | "beastman" | "dunadan" | "demigod" | "einheri"
        | "beorning" | "igor" | "centaur" | "maia" | "mangy-leper" | "doppelganger"
        | "mon-mimic" | "mon-possessor" => 'p',
        "tonberry" | "hobbit" | "gnome" | "dwarf" | "high-elf" | "nibelung" | "dark-elf"
        | "mindflayer" | "kutar" | "shadow-fairy" | "tomte" | "wood-elf" => 'h',
        "snotling" | "half-orc" | "mon-orc" => 'o',
        "half-troll" => 'T',
        "ogre" => 'O',
        "half-giant" | "half-titan" | "cyclops" | "mon-giant" => 'P',
        "boit" | "yeek" => 'y',
        "klackon" => 'K',
        "kobold" | "small-kobold" => 'k',
        "imp" | "demon" | "mon-demon" => 'u',
        "demon-lord" | "balrog" => 'U',
        "draconian" | "mon-dragon" => 'd',
        "golem" | "android" | "clay-golem" | "iron-golem" | "mithril-golem" | "colossus" => 'g',
        "skeleton" => 's',
        "zombie" => 'z',
        "vampire" | "vampire-lord" | "vampire-lord-form" => 'V',
        "spectre" => 'G',
        "sprite" => 'I',
        "ent" => '#',
        "werewolf" => 'C',
        "archon" | "mon-angel" => 'A',
        "mon-elemental" => 'E',
        "mon-jelly" => 'j',
        "mon-lich" => 'L',
        "mon-spider" => 'S',
        _ => 'p',
    }
}

fn legacy_race_tags(entry: &LegacyCharacterEntry) -> Vec<&'static str> {
    if entry.id == "high-elf" {
        return vec![
            "humanoid",
            "legacy-import",
            "rfb-compatibility",
            "snow-adapted",
            "standard-body",
        ];
    }
    if matches!(
        entry.id.as_str(),
        "barbarian" | "dunadan" | "dwarf" | "gnome" | "hobbit" | "kobold" | "nibelung"
    ) {
        return vec![
            "humanoid",
            "legacy-import",
            "rfb-compatibility",
            "standard-body",
        ];
    }
    vec!["legacy-import"]
}

fn race_json(
    entry: &LegacyCharacterEntry,
    body_slots: &[(String, String)],
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/race.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.race.{}", entry.id),
        "nameKey": format!("race-legacy-{}-name", entry.id),
        "descriptionKey": format!("race-legacy-{}-description", entry.id),
        "lifePercent": entry.life.clamp(25, 400),
        "experiencePercent": entry.exp.clamp(25, 500),
        "shopAdjustPercent": entry.shop_adjust.clamp(50, 200),
        "baseHp": entry.base_hp.clamp(-1_000, 1_000),
        "infravision": entry.infra.clamp(0, 64),
        "skillSetId": format!("rfb-legacy.skill-set.race-{}", entry.id),
        "kinCategory": format!("kin-glyph-{}", u32::from(legacy_race_kin_glyph(&entry.id))),
        "bodySlots": body_slots
            .iter()
            .map(|(id, slot_type)| serde_json::json!({"id": id, "slotType": slot_type}))
            .collect::<Vec<_>>(),
        "tags": legacy_race_tags(entry),
    });
    let modifiers = character_modifiers(entry);
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    if !entry.resistances.is_empty() {
        value["resistances"] = serde_json::Value::Object(
            entry
                .resistances
                .iter()
                .map(|(damage_type, level)| (damage_type.clone(), serde_json::json!(level)))
                .collect(),
        );
    }
    if entry.free_act {
        value["statusImmunities"] = serde_json::json!(["rfb.status.paralysis"]);
    }
    if entry.see_invisible {
        value["seeInvisible"] = serde_json::json!(true);
    }
    if !entry.attribute_sustains.is_empty() {
        value["attributeSustains"] = serde_json::json!(entry.attribute_sustains);
    }
    if !entry.abilities.is_empty() {
        value["abilities"] = serde_json::json!(
            entry
                .abilities
                .iter()
                .map(|power| serde_json::json!({
                    "minimumLevel": power.minimum_level,
                    "governingAttribute": power.governing_attribute,
                    "cost": power.cost,
                    "baseFailurePercent": power.base_failure_percent,
                    "abilityId": power.ability_id,
                }))
                .collect::<Vec<_>>()
        );
    }
    character_gap_accounting(entry, report);
    value
}

fn personality_json(
    entry: &LegacyCharacterEntry,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/personality.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.personality.{}", entry.id),
        "nameKey": format!("personality-legacy-{}-name", entry.id),
        "descriptionKey": format!("personality-legacy-{}-description", entry.id),
        "lifePercent": entry.life.clamp(25, 400),
        "experiencePercent": entry.exp.clamp(25, 500),
        "baseHp": entry.base_hp.clamp(-1_000, 1_000),
        "skillSetId": format!("rfb-legacy.skill-set.personality-{}", entry.id),
        "tags": ["legacy-import"],
    });
    let modifiers = character_modifiers(entry);
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    character_gap_accounting(entry, report);
    value
}

fn class_gap_accounting(entry: &LegacyClassEntry, report: &mut ContentImportReport) {
    for flag in &entry.character.flags {
        *report.unmapped_class_flags.entry(flag.clone()).or_default() += 1;
    }
    for hook in &entry.character.hooks {
        *report.class_hook_gaps.entry(hook.clone()).or_default() += 1;
    }
    if entry.character.dynamic {
        *report
            .class_hook_gaps
            .entry("dynamic-static-surface".to_owned())
            .or_default() += 1;
    }
    if !entry.source_found {
        *report
            .class_magic_gaps
            .entry("class-source-missing".to_owned())
            .or_default() += 1;
    }
    if !entry.registration.registered {
        *report
            .class_magic_gaps
            .entry("class-not-in-current-registry".to_owned())
            .or_default() += 1;
    }
}

fn class_json(
    entry: &LegacyClassEntry,
    has_casting_shell: bool,
    runtime_casting_profile: Option<&serde_json::Value>,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let character = &entry.character;
    let mut tags = vec!["legacy-import"];
    if has_casting_shell {
        tags.push("legacy-casting-shell");
    }
    if character.dynamic || !entry.source_found {
        tags.push("legacy-dynamic-shell");
    }
    if !entry.registration.registered {
        tags.push("legacy-unregistered");
    }
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/class.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.class.{}", entry.registration.id),
        "nameKey": format!("class-legacy-{}-name", entry.registration.id),
        "descriptionKey": format!("class-legacy-{}-description", entry.registration.id),
        "lifePercent": character.life.clamp(25, 400),
        "experiencePercent": character.exp.clamp(25, 500),
        "baseHp": character.base_hp.clamp(-1_000, 1_000),
        "skillSetId": format!("rfb-legacy.skill-set.class-{}", entry.registration.id),
        "petUpkeepDivisor": entry.pet_upkeep_divisor,
        "tags": tags,
    });
    let modifiers = character_modifiers(character);
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    if let Some(profile) = runtime_casting_profile {
        value["castingProfile"] = profile.clone();
    }
    if legacy_class_uses_spell_scrolls(&entry.registration.id) {
        value["usesSpellScrolls"] = serde_json::Value::Bool(true);
    }
    value["favoriteWeaponTags"] = serde_json::json!(match entry.registration.id.as_str() {
        "ranger" | "archer" => vec!["shooter"],
        "mystic" => Vec::new(),
        _ => vec!["weapon", "shooter"],
    });
    if entry.caster_profile.as_ref().is_some_and(|caster| {
        caster
            .options
            .iter()
            .any(|option| option == "glove-encumbrance")
    }) {
        value["ickyEquipmentSlots"] = serde_json::json!(["gloves"]);
    }
    value["specialItemTags"] = serde_json::json!(match entry.registration.id.as_str() {
        "alchemist" => vec!["potion"],
        "archer" => vec!["skeleton"],
        _ => Vec::new(),
    });
    class_gap_accounting(entry, report);
    value
}

fn legacy_class_uses_spell_scrolls(class_id: &str) -> bool {
    !matches!(
        class_id,
        "warrior"
            | "mindcrafter"
            | "psion"
            | "sorcerer"
            | "archer"
            | "magic-eater"
            | "devicemaster"
            | "red-mage"
            | "samurai"
            | "cavalry"
            | "berserker"
            | "weaponsmith"
            | "mirror-master"
            | "time-lord"
            | "blood-knight"
            | "warlock"
            | "archaeologist"
            | "duelist"
            | "rune-knight"
            | "wild-talent"
            | "blue-mage"
            | "ninja"
            | "ninja-lawyer"
            | "scout"
            | "mystic"
            | "mauler"
            | "politician"
            | "alchemist"
            | "disciple"
            | "skillmaster"
    )
}

fn magic_profile_json(
    profile: &LegacyMagicProfile,
    class_id: &str,
    source_commit: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": source_commit,
        "classId": format!("rfb-legacy.class.{class_id}"),
        "legacyClassIndex": profile.class_index,
        "legacyNameHint": profile.name_hint,
        "bookType": profile.book_type,
        "castingAttribute": profile.casting_attribute,
        "extraFlags": profile.extra_flags,
        "spellType": profile.spell_type,
        "firstSpellLevel": profile.first_spell_level,
        "spellWeight": profile.spell_weight,
        "realms": profile.realms.iter().map(|realm| {
            let realm_id = LEGACY_REALM_IDS
                .get(usize::from(realm.index))
                .copied()
                .unwrap_or("unknown");
            serde_json::json!({
                "legacyRealmIndex": realm.index,
                "realmId": realm_id,
                "readable": realm.readable,
                "spells": realm.spells.iter().map(|spell| serde_json::json!({
                    "slot": spell.index,
                    "level": spell.level,
                    "mana": spell.mana,
                    "failurePercent": spell.failure_percent,
                    "experience": spell.experience,
                })).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    })
}

fn realm_readability_json(
    profiles: &[LegacyMagicProfile],
    class_ids: &BTreeMap<u16, String>,
    source_commit: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": source_commit,
        "realms": LEGACY_REALM_IDS.iter().enumerate().map(|(index, realm_id)| {
            let readable_by_class_ids = profiles
                .iter()
                .filter(|profile| {
                    profile.realms.iter().any(|realm| {
                        usize::from(realm.index) == index && realm.readable
                    })
                })
                .filter_map(|profile| class_ids.get(&profile.class_index))
                .map(|class_id| format!("rfb-legacy.class.{class_id}"))
                .collect::<Vec<_>>();
            serde_json::json!({
                "legacyRealmIndex": index,
                "realmId": realm_id,
                "readableByClassIds": readable_by_class_ids,
            })
        }).collect::<Vec<_>>(),
    })
}

fn class_casting_shells_json(
    classes: &[LegacyClassEntry],
    profiles: &[LegacyMagicProfile],
    source_commit: &str,
) -> serde_json::Value {
    let profiles = profiles
        .iter()
        .map(|profile| (profile.class_index, profile))
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": source_commit,
        "classes": classes.iter().map(|entry| {
            let profile = profiles.get(&entry.registration.index).copied();
            let readable_realm_ids = profile
                .into_iter()
                .flat_map(|profile| profile.realms.iter())
                .filter(|realm| realm.readable)
                .filter_map(|realm| LEGACY_REALM_IDS.get(usize::from(realm.index)))
                .copied()
                .collect::<Vec<_>>();
            let caster_info = entry.caster_profile.as_ref().map(|caster| {
                serde_json::json!({
                    "dynamic": caster.dynamic,
                    "castingAttribute": caster.casting_attribute,
                    "minimumFailurePercent": caster.minimum_failure_percent,
                    "minimumLevel": caster.minimum_level,
                    "maxEncumbranceWeight": caster.max_encumbrance_weight,
                    "weaponEncumbrancePercent": caster.weapon_encumbrance_percent,
                    "zeroManaEncumbranceWeight": caster.zero_mana_encumbrance_weight,
                    "options": caster.options,
                })
            });
            serde_json::json!({
                "classId": format!("rfb-legacy.class.{}", entry.registration.id),
                "legacyClassIndex": entry.registration.index,
                "registered": entry.registration.registered,
                "hasMInfoProfile": profile.is_some(),
                "readableRealmIds": readable_realm_ids,
                "casterInfo": caster_info,
            })
        }).collect::<Vec<_>>(),
    })
}

fn runtime_casting_attribute(entry: &LegacyClassEntry) -> Option<&'static str> {
    let caster = entry.caster_profile.as_ref()?;
    if caster.dynamic || caster.options.iter().any(|option| option == "use-hp") {
        return None;
    }
    match caster.casting_attribute.as_str() {
        "int" => Some("intelligence"),
        "wis" => Some("wisdom"),
        "chr" => Some("charisma"),
        _ => None,
    }
}

fn death_entropy_uses_three_halves(class_id: &str) -> bool {
    matches!(
        class_id,
        "mage" | "blood-mage" | "high-mage" | "sorcerer" | "yellow-mage" | "gray-mage"
    )
}

fn legacy_beam_chance_profile(class_id: &str) -> (u8, u8, i8) {
    match class_id {
        "mage" | "blood-mage" | "necromancer" | "yellow-mage" | "gray-mage" => (1, 1, 0),
        "high-mage" | "sorcerer" => (1, 1, 10),
        _ => (1, 2, 0),
    }
}

fn death_realm(profile: &LegacyMagicProfile) -> Option<&LegacyRealmProfile> {
    profile
        .realms
        .iter()
        .find(|realm| realm.index == DEATH_REALM_INDEX && realm.readable)
}

fn death_spell_power_fields(spell_index: u8) -> Vec<serde_json::Value> {
    let field =
        |effect_index, field| serde_json::json!({"effectIndex": effect_index, "field": field});
    match spell_index {
        1 => vec![
            field(0, "final-damage"),
            field(0, "malediction-death-ray-power"),
            field(0, "malediction-fear-power"),
        ],
        3 | 8 | 9 | 18 | 21 | 22 => vec![field(0, "final-damage")],
        4 | 6 => vec![field(0, "status-power")],
        5 | 16 | 27 | 31 => vec![
            field(0, "status-duration-ticks"),
            field(0, "status-duration-sides"),
        ],
        7 => vec![field(0, "control-power")],
        10 | 23 | 30 => vec![field(0, "final-damage"), field(0, "radius")],
        11 | 15 | 29 => vec![field(0, "genocide-power")],
        13 => vec![field(0, "damage-sides"), field(0, "damage-bonus")],
        17 => vec![field(0, "random-choice-roll")],
        19 => vec![
            field(0, "status-duration-ticks"),
            field(0, "status-duration-sides"),
            field(1, "status-duration-ticks"),
            field(1, "status-duration-sides"),
            field(2, "status-duration-ticks"),
        ],
        26 => vec![field(0, "identify-power")],
        _ => Vec::new(),
    }
}

fn death_spell_ability(
    spell: &LegacySpellProfile,
    terrain_creation: &TerrainCreationImportIds,
) -> Option<(String, serde_json::Value)> {
    let self_target = serde_json::json!({
        "modes": ["self"],
        "range": 0,
        "requiresLineOfEffect": false,
    });
    let directional_target = serde_json::json!({
        "modes": ["direction"],
        "range": 8,
        "requiresLineOfEffect": true,
    });
    let item_target = serde_json::json!({
        "modes": ["item"],
        "range": 0,
        "requiresLineOfEffect": false,
    });
    let (id, target, effect, level_scaling, tags) = match spell.index {
        0 => (
            "death-detect-unlife",
            self_target.clone(),
            serde_json::json!({
                "type": "detect",
                "subject": "actor",
                "category": "nonliving",
                "radius": 30,
            }),
            Vec::new(),
            vec!["death", "detection", "spell"],
        ),
        1 => (
            "death-malediction",
            directional_target.clone(),
            serde_json::json!({
                "type": "malediction",
                "damageDice": 3,
                "damageSides": 4,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-dice",
                "levelOffset": 1,
                "multiplier": 1,
                "divisor": 5,
            })],
            vec!["damage", "death", "hell-fire", "spell"],
        ),
        2 => (
            "death-detect-evil",
            self_target.clone(),
            serde_json::json!({
                "type": "detect",
                "subject": "actor",
                "category": "evil",
                "radius": 30,
            }),
            Vec::new(),
            vec!["death", "detection", "spell"],
        ),
        3 => (
            "death-stinking-cloud",
            directional_target.clone(),
            serde_json::json!({
                "type": "area-damage",
                "damageDice": 1,
                "damageSides": 1,
                "damageBonus": 9,
                "damageType": "poison",
                "radius": 2,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-bonus",
                "multiplier": 1,
                "divisor": 2,
            })],
            vec!["area", "death", "poison", "spell"],
        ),
        4 => (
            "death-black-sleep",
            directional_target.clone(),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.sleep",
                "intensity": 1,
                "durationTicks": 500,
                "stacking": "keep-strongest",
                "power": 2,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "status-power",
                "levelOffset": 1,
                "multiplier": 2,
                "divisor": 1,
            })],
            vec!["death", "sleep", "spell"],
        ),
        5 => (
            "death-necromantic-resistance",
            self_target.clone(),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.necromantic-resistance",
                "intensity": 1,
                "durationTicks": 20,
                "durationDice": 1,
                "durationSides": 20,
                "stacking": "replace",
                "grantedResistances": {
                    "cold": "resistant",
                    "poison": "resistant",
                },
            }),
            Vec::new(),
            vec!["death", "resistance", "spell"],
        ),
        6 => (
            "death-horrify",
            directional_target.clone(),
            serde_json::json!({
                "type": "sequence",
                "effects": [
                    {
                        "type": "apply-status",
                        "statusKindId": "rfb.status.fear",
                        "intensity": 1,
                        "durationTicks": 200,
                        "stacking": "extend",
                        "power": 2,
                    },
                    {
                        "type": "apply-status",
                        "statusKindId": "rfb.status.stun",
                        "intensity": 1,
                        "durationTicks": 5,
                        "stacking": "extend",
                    },
                ],
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "status-power",
                    "levelOffset": 1,
                    "multiplier": 2,
                    "divisor": 1,
                }),
                serde_json::json!({
                    "effectIndex": 1,
                    "field": "status-duration-ticks",
                    "multiplier": 1,
                    "divisor": 5,
                }),
            ],
            vec!["death", "fear", "spell", "stun"],
        ),
        7 => (
            "death-enslave-undead",
            directional_target.clone(),
            serde_json::json!({
                "type": "control",
                "category": "undead",
                "power": 2,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "control-power",
                "levelOffset": 1,
                "multiplier": 2,
                "divisor": 1,
            })],
            vec!["control", "death", "spell", "undead"],
        ),
        8 => (
            "death-entropy-orb",
            directional_target.clone(),
            serde_json::json!({
                "type": "area-damage",
                "damageDice": 3,
                "damageSides": 6,
                "damageType": "physical",
                "radius": 2,
                "targetCategory": "living",
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "damage-bonus",
                    "multiplier": 3,
                    "divisor": 2,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "radius",
                    "multiplier": 1,
                    "divisor": 30,
                    "maximum": 3,
                }),
            ],
            vec!["area", "death", "drain", "living", "spell"],
        ),
        9 => (
            "death-nether-bolt",
            directional_target.clone(),
            serde_json::json!({
                "type": "bolt-or-beam-damage",
                "damageDice": 8,
                "damageSides": 8,
                "damageType": "nether",
                "beamChancePercent": 0,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-dice",
                "levelOffset": 5,
                "multiplier": 1,
                "divisor": 4,
            })],
            vec!["beam", "bolt", "death", "nether", "spell"],
        ),
        10 => (
            "death-cloud-kill",
            self_target.clone(),
            serde_json::json!({
                "type": "area-damage",
                "damageDice": 1,
                "damageSides": 1,
                "damageBonus": 59,
                "damageType": "poison",
                "radius": 2,
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "damage-bonus",
                    "multiplier": 2,
                    "divisor": 1,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "radius",
                    "multiplier": 1,
                    "divisor": 10,
                }),
            ],
            vec!["area", "death", "poison", "spell"],
        ),
        11 => (
            "death-genocide-one",
            directional_target.clone(),
            serde_json::json!({
                "type": "genocide",
                "scope": "single",
                "power": 3,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "genocide-power",
                "levelOffset": 1,
                "multiplier": 3,
                "divisor": 1,
            })],
            vec!["death", "genocide", "spell"],
        ),
        12 => (
            "death-poison-branding",
            item_target.clone(),
            serde_json::json!({
                "type": "brand-weapon",
                "affixId": LEGACY_SLAYING_WEAPON_AFFIX_ID,
                "brand": "poison",
                "resistance": "poison",
            }),
            Vec::new(),
            vec!["brand", "death", "poison", "spell"],
        ),
        13 => (
            "death-vampiric-drain",
            directional_target.clone(),
            serde_json::json!({
                "type": "drain-life",
                "damageDice": 1,
                "damageSides": 2,
                "damageBonus": 2,
                "damageType": "physical",
                "targetCategory": "living",
                "feeds": true,
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "damage-sides",
                    "levelOffset": 1,
                    "multiplier": 2,
                    "divisor": 1,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "damage-bonus",
                    "levelOffset": 1,
                    "multiplier": 2,
                    "divisor": 1,
                }),
            ],
            vec!["death", "drain", "living", "spell"],
        ),
        14 => (
            "death-animate-dead",
            self_target.clone(),
            serde_json::json!({
                "type": "animate-dead",
                "actorKindId": "rfb-legacy.actor.skeleton-human",
                "corpseItemKindId": LEGACY_CORPSE_ITEM_ID,
                "radius": 8,
                "count": 8,
            }),
            Vec::new(),
            vec!["death", "spell", "summon", "undead"],
        ),
        15 => (
            "death-genocide",
            directional_target.clone(),
            serde_json::json!({
                "type": "genocide",
                "scope": "glyph",
                "power": 3,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "genocide-power",
                "levelOffset": 1,
                "multiplier": 3,
                "divisor": 1,
            })],
            vec!["death", "genocide", "spell"],
        ),
        16 => (
            "death-berserk",
            self_target.clone(),
            serde_json::json!({
                "type": "sequence",
                "effects": [
                    {
                        "type": "apply-status",
                        "statusKindId": "rfb.status.berserk",
                        "intensity": 1,
                        "durationTicks": 25,
                        "durationDice": 1,
                        "durationSides": 25,
                        "stacking": "replace",
                        "grantedModifiers": {"defense": -10, "maxHp": 30},
                        "grantedEquipmentBonuses": {
                            "meleeSkill": 12,
                            "meleeDamage": 3,
                            "rangedSkill": -12,
                            "throwingSkill": -20,
                            "deviceSkill": -20,
                            "savingThrowSkill": -30,
                            "stealthSkill": -7,
                            "searchSkill": -15,
                            "perceptionSkill": -15,
                            "diggingSkill": 30,
                        },
                        "grantedStatusImmunities": ["rfb.status.fear"],
                    },
                    {"type": "heal", "amount": 30},
                ],
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "status-melee-damage",
                "multiplier": 1,
                "divisor": 5,
            })],
            vec!["berserk", "death", "spell", "status"],
        ),
        17 => (
            "death-invoke-spirits",
            directional_target.clone(),
            serde_json::json!({
                "type": "random-choice",
                "rollSides": 100,
                "levelBonusDivisor": 5,
                "branches": [
                    {"maximumRoll": 7, "target": "self-target", "effect": {"type": "summon", "actorKindId": "rfb-legacy.actor.skeleton-human", "count": 1, "radius": 2, "durationTurns": 0, "hostile": true}},
                    {"maximumRoll": 13, "target": "self-target", "effect": {"type": "apply-status", "statusKindId": "rfb.status.fear", "intensity": 3, "durationTicks": 50, "stacking": "keep-strongest"}},
                    {"maximumRoll": 25, "target": "self-target", "effect": {"type": "apply-status", "statusKindId": "rfb.status.confusion", "intensity": 1, "durationTicks": 4, "durationDice": 1, "durationSides": 4, "stacking": "extend"}},
                    {"maximumRoll": 30, "effect": {"type": "polymorph-target"}},
                    {"maximumRoll": 35, "effect": {"type": "bolt-or-beam-damage", "damageDice": 3, "damageSides": 4, "damageType": "physical", "beamChancePercent": 0}, "levelScaling": [{"effectIndex": 0, "field": "damage-dice", "levelOffset": 1, "multiplier": 1, "divisor": 5}]},
                    {"maximumRoll": 40, "effect": {"type": "apply-status", "statusKindId": "rfb.status.confusion", "intensity": 1, "durationTicks": 10, "stacking": "keep-strongest", "power": 1}, "levelScaling": [{"effectIndex": 0, "field": "status-power", "levelOffset": 1, "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 45, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 19, "damageType": "poison", "radius": 3}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 2}]},
                    {"maximumRoll": 50, "effect": {"type": "light-line", "damageDice": 6, "damageSides": 8}},
                    {"maximumRoll": 55, "effect": {"type": "bolt-or-beam-damage", "damageDice": 3, "damageSides": 8, "damageType": "electricity", "beamChancePercent": 0}, "levelScaling": [{"effectIndex": 0, "field": "damage-dice", "levelOffset": 5, "multiplier": 1, "divisor": 4}]},
                    {"maximumRoll": 60, "effect": {"type": "bolt-or-beam-damage", "damageDice": 5, "damageSides": 8, "damageType": "cold", "beamChancePercent": 0}, "levelScaling": [{"effectIndex": 0, "field": "damage-dice", "levelOffset": 5, "multiplier": 1, "divisor": 4}]},
                    {"maximumRoll": 65, "effect": {"type": "bolt-or-beam-damage", "damageDice": 6, "damageSides": 8, "damageType": "acid", "beamChancePercent": 0}, "levelScaling": [{"effectIndex": 0, "field": "damage-dice", "levelOffset": 5, "multiplier": 1, "divisor": 4}]},
                    {"maximumRoll": 70, "effect": {"type": "bolt-or-beam-damage", "damageDice": 8, "damageSides": 8, "damageType": "fire", "beamChancePercent": 0}, "levelScaling": [{"effectIndex": 0, "field": "damage-dice", "levelOffset": 5, "multiplier": 1, "divisor": 4}]},
                    {"maximumRoll": 75, "effect": {"type": "drain-life", "damageDice": 1, "damageSides": 1, "damageBonus": 74, "damageType": "nether", "targetCategory": "living"}},
                    {"maximumRoll": 80, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 29, "damageType": "electricity", "radius": 2}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 2}]},
                    {"maximumRoll": 85, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 39, "damageType": "acid", "radius": 2}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 90, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 69, "damageType": "ice", "radius": 3}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 95, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 79, "damageType": "fire", "radius": 3}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 100, "effect": {"type": "drain-life", "damageDice": 1, "damageSides": 1, "damageBonus": 99, "damageType": "nether", "targetCategory": "living"}, "levelScaling": [{"effectIndex": 0, "field": "damage-bonus", "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 103, "target": "self-target", "effect": {
                        "type": "earthquake",
                        "radius": 12,
                        "affectChancePercent": 15,
                        "floorTerrainId": terrain_creation.floor_terrain_id.as_ref()?,
                        "wallTerrainIds": [
                            terrain_creation.wall_terrain_id.as_ref()?,
                            terrain_creation.quartz_terrain_id.as_ref()?,
                            terrain_creation.magma_terrain_id.as_ref()?,
                        ],
                    }},
                    {"maximumRoll": 105, "target": "self-target", "effect": {
                        "type": "area-destruction",
                        "minimumRadius": 13,
                        "maximumRadius": 17,
                        "floorTerrainId": terrain_creation.floor_terrain_id.as_ref()?,
                        "wallTerrainId": terrain_creation.wall_terrain_id.as_ref()?,
                        "quartzTerrainId": terrain_creation.quartz_terrain_id.as_ref()?,
                        "magmaTerrainId": terrain_creation.magma_terrain_id.as_ref()?,
                    }},
                    {"maximumRoll": 107, "effect": {"type": "genocide", "scope": "glyph", "power": 50}, "levelScaling": [{"effectIndex": 0, "field": "genocide-power", "multiplier": 1, "divisor": 1}]},
                    {"maximumRoll": 109, "target": "self-target", "effect": {"type": "visible-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 119}},
                    {"maximumRoll": 65535, "target": "self-target", "effect": {"type": "sequence", "effects": [
                        {"type": "visible-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 149},
                        {"type": "visible-apply-status", "statusKindId": "rfb.status.slow", "intensity": 1, "durationTicks": 50, "stacking": "extend", "power": 1},
                        {"type": "visible-apply-status", "statusKindId": "rfb.status.sleep", "intensity": 1, "durationTicks": 500, "stacking": "keep-strongest", "power": 1},
                        {"type": "heal", "amount": 300},
                    ]}, "levelScaling": [
                        {"effectIndex": 1, "field": "status-power", "levelOffset": 1, "multiplier": 1, "divisor": 1},
                        {"effectIndex": 2, "field": "status-power", "levelOffset": 1, "multiplier": 1, "divisor": 1},
                    ]},
                ],
            }),
            Vec::new(),
            vec!["death", "random", "spell", "spirits"],
        ),
        18 => (
            "death-dark-bolt",
            directional_target.clone(),
            serde_json::json!({
                "type": "bolt-or-beam-damage",
                "damageDice": 4,
                "damageSides": 8,
                "damageType": "dark",
                "beamChancePercent": 0,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-dice",
                "levelOffset": 5,
                "multiplier": 1,
                "divisor": 4,
            })],
            vec!["beam", "bolt", "dark", "death", "spell"],
        ),
        19 => (
            "death-battle-frenzy",
            self_target.clone(),
            serde_json::json!({
                "type": "sequence",
                "effects": [
                    {"type": "apply-status", "statusKindId": "rfb.status.hero", "intensity": 1, "durationTicks": 25, "durationDice": 1, "durationSides": 25, "stacking": "replace", "grantedModifiers": {"maxHp": 10}, "grantedEquipmentBonuses": {"meleeSkill": 12, "rangedSkill": 12}, "grantedStatusImmunities": ["rfb.status.fear"]},
                    {"type": "apply-status", "statusKindId": "rfb.status.blessed", "intensity": 1, "durationTicks": 25, "durationDice": 1, "durationSides": 25, "stacking": "replace", "grantedModifiers": {"defense": 5}, "grantedEquipmentBonuses": {"meleeSkill": 10, "rangedSkill": 10}},
                    {"type": "apply-status", "statusKindId": "rfb.status.haste", "intensity": 1, "durationTicks": 0, "durationDice": 1, "durationSides": 20, "stacking": "replace"},
                ],
            }),
            vec![
                serde_json::json!({"effectIndex": 2, "field": "status-duration-ticks", "multiplier": 1, "divisor": 2}),
                serde_json::json!({"effectIndex": 2, "field": "status-duration-sides", "multiplier": 1, "divisor": 2}),
            ],
            vec!["blessed", "death", "haste", "hero", "spell", "status"],
        ),
        20 => (
            "death-vampiric-branding",
            item_target,
            serde_json::json!({"type": "brand-weapon", "affixId": LEGACY_DEATH_WEAPON_AFFIX_ID}),
            Vec::new(),
            vec!["brand", "death", "permanent", "spell", "vampiric"],
        ),
        21 => (
            "death-vampirism-true",
            directional_target.clone(),
            serde_json::json!({
                "type": "drain-life",
                "damageDice": 1,
                "damageSides": 1,
                "damageBonus": 99,
                "damageType": "nether",
                "targetCategory": "living",
                "repeat": 3,
            }),
            Vec::new(),
            vec!["death", "drain", "repeat", "spell", "vampiric"],
        ),
        22 => (
            "death-nether-wave",
            self_target,
            serde_json::json!({
                "type": "visible-damage",
                "damageDice": 1,
                "damageSides": 3,
                "damageType": "nether",
                "targetCategory": "living",
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-sides",
                "levelOffset": 1,
                "multiplier": 3,
                "divisor": 1,
            })],
            vec!["death", "living", "nether", "spell", "visible"],
        ),
        23 => (
            "death-darkness-storm",
            directional_target.clone(),
            serde_json::json!({
                "type": "area-damage",
                "damageDice": 1,
                "damageSides": 1,
                "damageBonus": 99,
                "damageType": "dark",
                "radius": 4,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "damage-bonus",
                "multiplier": 200,
                "divisor": 1,
                "curve": "prorated",
                "quadraticWeight": 1,
                "cubicWeight": 2,
            })],
            vec!["area", "dark", "death", "prorated", "spell"],
        ),
        24 => (
            "death-death-ray",
            directional_target.clone(),
            serde_json::json!({"type": "death-ray", "power": 0}),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "death-ray-power",
                "multiplier": 2,
                "divisor": 1,
            })],
            vec!["death", "death-ray", "living", "spell"],
        ),
        25 => (
            "death-raise-dead",
            self_target.clone(),
            serde_json::json!({
                "type": "summon-category",
                "category": "undead",
                "upgradedCategory": "high-undead",
                "upgradeAtLevel": 48,
                "maximumLevel": 0,
                "countDice": 1,
                "countSides": 1,
                "hostileChancePercent": 67,
                "friendlyGroupChancePercent": 33,
                "hostileGroupChancePercent": 100,
                "groupCountDice": 1,
                "groupCountSides": 3,
                "groupCountBonus": 1,
                "allowUniqueHostile": true,
                "radius": 2,
                "durationTurns": 0,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "summon-maximum-level",
                "multiplier": 3,
                "divisor": 2,
            })],
            vec!["death", "spell", "summon", "undead"],
        ),
        26 => (
            "death-esoteria",
            serde_json::json!({
                "modes": ["item"],
                "range": 0,
                "requiresLineOfEffect": false,
            }),
            serde_json::json!({
                "type": "identify-item",
                "fullIdentifyPower": 0,
                "fullIdentifyRollSides": 50,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "identify-power",
                "multiplier": 50,
                "divisor": 1,
                "curve": "prorated",
                "quadraticWeight": 1,
                "cubicWeight": 2,
            })],
            vec!["death", "identify", "spell"],
        ),
        27 => (
            "death-vampiric-transformation",
            self_target.clone(),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.vampiric-transformation",
                "intensity": 1,
                "durationTicks": 10,
                "durationDice": 1,
                "durationSides": 10,
                "stacking": "replace",
                "grantedRaceId": LEGACY_VAMPIRE_LORD_RACE_ID,
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "status-duration-ticks",
                    "multiplier": 25,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "status-duration-sides",
                    "multiplier": 25,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
            ],
            vec!["death", "race", "spell", "status", "vampire"],
        ),
        28 => (
            "death-restore-life",
            self_target.clone(),
            serde_json::json!({"type": "restore-vitality", "lifeForce": 1000}),
            Vec::new(),
            vec!["death", "experience", "restoration", "spell"],
        ),
        29 => (
            "death-mass-genocide",
            self_target.clone(),
            serde_json::json!({
                "type": "genocide",
                "scope": "nearby",
                "power": 0,
                "radius": 20,
            }),
            vec![serde_json::json!({
                "effectIndex": 0,
                "field": "genocide-power",
                "multiplier": 150,
                "divisor": 1,
                "curve": "prorated",
                "quadraticWeight": 1,
                "cubicWeight": 2,
            })],
            vec!["death", "genocide", "spell"],
        ),
        30 => (
            "death-hellfire",
            directional_target.clone(),
            serde_json::json!({
                "type": "area-damage",
                "damageDice": 1,
                "damageSides": 1,
                "damageBonus": 4,
                "damageType": "nether",
                "radius": 1,
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "damage-bonus",
                    "multiplier": 600,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "radius",
                    "multiplier": 9,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
            ],
            vec!["area", "death", "nether", "spell"],
        ),
        31 => (
            "death-wraithform",
            self_target,
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.wraithform",
                "intensity": 1,
                "durationTicks": 1,
                "durationDice": 1,
                "durationSides": 1,
                "stacking": "replace",
                "grantsWallPassage": true,
                "incomingDamagePercent": 50,
            }),
            vec![
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "status-duration-ticks",
                    "multiplier": 24,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
                serde_json::json!({
                    "effectIndex": 0,
                    "field": "status-duration-sides",
                    "multiplier": 24,
                    "divisor": 1,
                    "curve": "prorated",
                    "quadraticWeight": 1,
                    "cubicWeight": 2,
                }),
            ],
            vec!["death", "spell", "status", "wall-passage", "wraith"],
        ),
        _ => return None,
    };
    let ability_id = format!("rfb-legacy.ability.{id}");
    let mut ability = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": ability_id,
        "nameKey": format!("ability-legacy-{id}-name"),
        "descriptionKey": format!("ability-legacy-{id}-description"),
        "minimumLevel": spell.level.max(1),
        "resourceId": LEGACY_MANA_RESOURCE_ID,
        "resourceCost": u32::from(spell.mana.max(1)),
        "baseFailurePercent": spell.failure_percent.min(95),
        "target": target,
        "effect": effect,
        "tags": tags,
    });
    if !level_scaling.is_empty() {
        ability["levelScaling"] = serde_json::Value::Array(level_scaling);
    }
    let spell_power_fields = death_spell_power_fields(spell.index);
    if !spell_power_fields.is_empty() {
        ability["spellPowerFields"] = serde_json::Value::Array(spell_power_fields);
    }
    Some((ability_id, ability))
}

fn death_first_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_FIRST_BOOK_ID,
        "nameKey": "ability-book-legacy-death-black-prayers-name",
        "descriptionKey": "ability-book-legacy-death-black-prayers-description",
        "realmId": "death",
        "rank": 1,
        "abilityIds": ability_ids,
        "tags": ["death", "legacy-import", "spellbook"],
    })
}

fn death_second_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_SECOND_BOOK_ID,
        "nameKey": "ability-book-legacy-death-black-mass-name",
        "descriptionKey": "ability-book-legacy-death-black-mass-description",
        "realmId": "death",
        "rank": 2,
        "abilityIds": ability_ids,
        "tags": ["death", "legacy-import", "spellbook"],
    })
}

fn death_third_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_THIRD_BOOK_ID,
        "nameKey": "ability-book-legacy-death-black-channels-name",
        "descriptionKey": "ability-book-legacy-death-black-channels-description",
        "realmId": "death",
        "rank": 3,
        "abilityIds": ability_ids,
        "tags": ["death", "legacy-import", "spellbook"],
    })
}

fn death_fourth_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_FOURTH_BOOK_ID,
        "nameKey": "ability-book-legacy-death-necronomicon-name",
        "descriptionKey": "ability-book-legacy-death-necronomicon-description",
        "realmId": "death",
        "rank": 4,
        "abilityIds": ability_ids,
        "tags": ["death", "legacy-import", "spellbook"],
    })
}

fn vampire_lord_skill_set_json() -> serde_json::Value {
    let bases = [6, 12, 8, 6, 2, 12, 30, 10];
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/skill-set.schema.json"),
        "formatVersion": 1,
        "id": LEGACY_VAMPIRE_LORD_SKILL_SET_ID,
        "entries": LEGACY_SKILL_ROSTER
            .iter()
            .zip(bases)
            .map(|((suffix, _), base)| serde_json::json!({
                "skillId": format!("rfb-legacy.skill.{suffix}"),
                "base": base,
            }))
            .collect::<Vec<_>>(),
    })
}

fn vampire_lord_race_json() -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/race.schema.json"),
        "formatVersion": 1,
        "id": LEGACY_VAMPIRE_LORD_RACE_ID,
        "nameKey": "race-legacy-vampire-lord-form-name",
        "descriptionKey": "race-legacy-vampire-lord-form-description",
        "modifiers": {
            "strength": 4,
            "intelligence": 4,
            "wisdom": 1,
            "dexterity": 1,
            "constitution": 2,
            "charisma": 3,
            "defense": 1,
            "speed": 30,
        },
        "lifePercent": 103,
        "experiencePercent": 300,
        "baseHp": 22,
        "skillSetId": LEGACY_VAMPIRE_LORD_SKILL_SET_ID,
        "kinCategory": "kin-glyph-86",
        "resistances": {
            "cold": "resistant",
            "dark": "immune",
            "light": "vulnerable",
            "nether": "resistant",
            "poison": "resistant",
        },
        "tags": ["legacy-import", "nonliving", "undead", "vampire"],
    })
}

fn legacy_skill_files() -> Vec<(String, serde_json::Value)> {
    LEGACY_SKILL_ROSTER
        .iter()
        .map(|(suffix, kind)| {
            (
                format!("{suffix}.json"),
                serde_json::json!({
                    "$schema": format!("{SCHEMA_BASE}/skill.schema.json"),
                    "formatVersion": 1,
                    "id": format!("rfb-legacy.skill.{suffix}"),
                    "nameKey": format!("skill-legacy-{suffix}-name"),
                    "descriptionKey": format!("skill-legacy-{suffix}-description"),
                    "kind": kind,
                    "maximum": 1000,
                    "tags": ["legacy-import"],
                }),
            )
        })
        .collect()
}

fn terrain_json(
    entry: &LegacyTerrainEntry,
    id: &str,
    monster_destroy_to_terrain_id: Option<&str>,
) -> serde_json::Value {
    let walkable = entry.flags.iter().any(|flag| flag == "MOVE");
    let blocks_sight = !entry.flags.iter().any(|flag| flag == "LOS");
    let mut tags = vec!["legacy-import"];
    if entry.flags.iter().any(|flag| flag == "TRAP") {
        tags.push("trap");
    }
    if entry.flags.iter().any(|flag| flag == "PERMANENT") {
        tags.push("permanent");
    }
    if entry.flags.iter().any(|flag| flag == "GLYPH") {
        tags.push("warding-glyph");
    }
    if entry.flags.iter().any(|flag| flag == "WATER") {
        tags.push("water");
    }
    if entry
        .flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "DOOR" | "STAIRS"))
    {
        tags.push("passage");
    }
    let glyph = match entry.glyph {
        Some(glyph) if glyph.is_ascii_alphabetic() && tags.contains(&"passage") => '+',
        Some(glyph) if glyph.is_ascii_alphabetic() && walkable => '.',
        Some(glyph) if glyph.is_ascii_alphabetic() => '#',
        Some(glyph) => glyph,
        None => '?',
    };
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/terrain.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.terrain.{id}"),
        "nameKey": format!("terrain-legacy-{id}-name"),
        "descriptionKey": format!("terrain-legacy-{id}-description"),
        "glyph": glyph.to_string(),
        "walkable": walkable,
        "blocksSight": blocks_sight,
        "tags": tags,
    });
    if let Some(target_id) = monster_destroy_to_terrain_id {
        value["monsterDestroyToTerrainId"] = serde_json::json!(target_id);
    }
    if entry.flags.iter().any(|flag| flag == "CAN_PASS") {
        value["allowsWallPassage"] = serde_json::json!(true);
    }
    let movement_modes = [
        ("CAN_CLIMB", "climb"),
        ("CAN_FLY", "fly"),
        ("CAN_SWIM", "swim"),
    ]
    .into_iter()
    .filter_map(|(flag, mode)| {
        entry
            .flags
            .iter()
            .any(|value| value == flag)
            .then_some(mode)
    })
    .collect::<Vec<_>>();
    if !movement_modes.is_empty() {
        value["movementModes"] = serde_json::json!(movement_modes);
    }
    value
}

fn melee_damage_type(token: &str) -> Option<&'static str> {
    match token {
        "HURT" | "DAM" | "VAMP" => Some("physical"),
        "FIRE" => Some("fire"),
        "COLD" => Some("cold"),
        "ACID" => Some("acid"),
        "ELEC" => Some("electricity"),
        "LIGHT" | "LITE" => Some("light"),
        "DARK" => Some("dark"),
        "NETHER" => Some("nether"),
        "NEXUS" => Some("nexus"),
        "SOUND" => Some("sound"),
        "SHARDS" => Some("shards"),
        "CHAOS" => Some("chaos"),
        "DISENCHANT" => Some("disenchant"),
        "TIME" => Some("time"),
        "MANA" => Some("mana"),
        "GRAVITY" => Some("gravity"),
        "INERTIA" => Some("inertia"),
        "PLASMA" => Some("plasma"),
        "FORCE" => Some("force"),
        "NUKE" => Some("nuke"),
        "DISINTEGRATE" => Some("disintegrate"),
        "STORM" => Some("storm"),
        "HOLY_FIRE" => Some("holy-fire"),
        "HELL_FIRE" => Some("hell-fire"),
        "CAUSE_1" | "CAUSE_2" | "CAUSE_3" | "CAUSE_4" => Some("curse"),
        "ICE" => Some("ice"),
        "WATER" => Some("water"),
        "POIS" => Some("poison"),
        "MIND_BLAST" | "BRAIN_SMASH" => Some("psi"),
        _ => None,
    }
}

fn melee_effect_json(
    effect: &LegacyBlowEffect,
    monster_level: Option<u16>,
) -> Option<serde_json::Value> {
    let mut value = match effect.token.as_str() {
        "POISON" => {
            let (damage_dice, damage_sides) = effect.dice?;
            serde_json::json!({
                "type": "poison",
                "damageDice": damage_dice.clamp(1, 100),
                "damageSides": damage_sides.clamp(1, 10_000),
            })
        }
        "DISEASE" => {
            let (damage_dice, damage_sides) = effect.dice.unwrap_or((0, 0));
            serde_json::json!({
                "type": "disease",
                "damageDice": damage_dice.min(100),
                "damageSides": damage_sides.min(10_000),
            })
        }
        "SHATTER" => {
            let (damage_dice, damage_sides) = effect.dice?;
            serde_json::json!({
                "type": "shatter",
                "damageDice": damage_dice.clamp(1, 100),
                "damageSides": damage_sides.clamp(1, 10_000),
            })
        }
        "BOMB" => {
            let (damage_dice, damage_sides) = effect.dice?;
            serde_json::json!({
                "type": "bomb",
                "damageDice": damage_dice.clamp(1, 100),
                "damageSides": damage_sides.clamp(1, 10_000),
            })
        }
        "CUT" => {
            let (duration_dice, duration_sides) = effect.dice?;
            serde_json::json!({
                "type": "bleeding",
                "durationDice": duration_dice.clamp(1, 100),
                "durationSides": duration_sides.clamp(1, 10_000),
            })
        }
        "BLIND" => serde_json::json!({ "type": "blind" }),
        "CONFUSE" => {
            let (damage_dice, damage_sides) = effect.dice.unwrap_or((0, 0));
            serde_json::json!({
                "type": "confusion",
                "damageDice": damage_dice.min(100),
                "damageSides": damage_sides.min(10_000),
            })
        }
        "PARALYZE" | "SLEEP" => serde_json::json!({ "type": "paralysis" }),
        "AMNESIA" => serde_json::json!({ "type": "amnesia" }),
        "TIME" if effect.dice.is_none() => serde_json::json!({ "type": "time" }),
        "SLOW" => serde_json::json!({ "type": "slow" }),
        "INERTIA" if effect.dice.is_none() => serde_json::json!({ "type": "inertia" }),
        "POLYMORPH" if effect.dice.is_none() => {
            serde_json::json!({ "type": "polymorph-player" })
        }
        "STUN" => {
            let (duration_dice, duration_sides) = effect.dice?;
            serde_json::json!({
                "type": "stun",
                "durationDice": duration_dice.clamp(1, 100),
                "durationSides": duration_sides.clamp(1, 10_000),
            })
        }
        "TERRIFY" => serde_json::json!({ "type": "terrify" }),
        "DISENCHANT" if effect.dice.is_none() => {
            serde_json::json!({ "type": "disenchant" })
        }
        "EAT_GOLD" => serde_json::json!({ "type": "eat-gold" }),
        "EAT_ITEM" => serde_json::json!({ "type": "eat-item" }),
        "EAT_FOOD" => serde_json::json!({ "type": "eat-food" }),
        "EAT_LITE" => serde_json::json!({ "type": "eat-light" }),
        "DRAIN_MANA" => {
            let (amount_dice, amount_sides) = effect
                .dice
                .or_else(|| monster_level.map(|level| (1, 1 + level / 2)))?;
            serde_json::json!({
                "type": "drain-resource",
                "amountDice": amount_dice.clamp(1, 100),
                "amountSides": amount_sides.clamp(1, 10_000),
            })
        }
        "DRAIN_CHARGES" => serde_json::json!({ "type": "drain-charges" }),
        "DRAIN_EXP" => {
            let (amount_dice, amount_sides) = effect.dice?;
            serde_json::json!({
                "type": "drain-experience",
                "amountDice": amount_dice.clamp(1, 100),
                "amountSides": amount_sides.clamp(1, 10_000),
            })
        }
        "UNLIFE" => {
            let (amount_dice, amount_sides) = effect.dice?;
            serde_json::json!({
                "type": "unlife",
                "amountDice": amount_dice.clamp(1, 100),
                "amountSides": amount_sides.clamp(1, 10_000),
            })
        }
        "LOSE_STR" | "LOSE_INT" | "LOSE_WIS" | "LOSE_DEX" | "LOSE_CON" | "LOSE_CHR"
        | "LOSE_ALL" => {
            let attributes: &[&str] = match effect.token.as_str() {
                "LOSE_STR" => &["strength"],
                "LOSE_INT" => &["intelligence"],
                "LOSE_WIS" => &["wisdom"],
                "LOSE_DEX" => &["dexterity"],
                "LOSE_CON" => &["constitution"],
                "LOSE_CHR" => &["charisma"],
                "LOSE_ALL" => &[
                    "strength",
                    "dexterity",
                    "constitution",
                    "intelligence",
                    "wisdom",
                    "charisma",
                ],
                _ => unreachable!(),
            };
            serde_json::json!({
                "type": "drain-attributes",
                "attributes": attributes,
            })
        }
        token => {
            let damage_type = melee_damage_type(token)?;
            let (damage_dice, damage_sides) = match effect.dice {
                Some((dice, sides)) => (dice.clamp(1, 100), sides.clamp(1, 10_000)),
                None if token == "HURT" => (0, 0),
                None => return None,
            };
            serde_json::json!({
                "type": "damage",
                "damageDice": damage_dice,
                "damageSides": damage_sides,
                "damageType": damage_type,
                "armorMitigated": token == "HURT",
            })
        }
    };
    if effect.token == "VAMP" {
        value["vampiric"] = serde_json::json!(true);
    }
    if let Some(chance_percent) = effect.chance_percent {
        value["chancePercent"] = serde_json::json!(chance_percent);
    }
    Some(value)
}

fn melee_effects_json(
    effect: &LegacyBlowEffect,
    monster_level: Option<u16>,
) -> Option<Vec<serde_json::Value>> {
    if effect.token == "MIND_BLAST" {
        if effect.chance_percent.is_some() {
            return None;
        }
        return Some(vec![
            melee_effect_json(effect, monster_level)?,
            serde_json::json!({
                "type": "confusion",
                "damageDice": 0,
                "damageSides": 0,
            }),
        ]);
    }
    if effect.token == "BRAIN_SMASH" {
        if effect.chance_percent.is_some() {
            return None;
        }
        return Some(vec![
            melee_effect_json(effect, monster_level)?,
            serde_json::json!({ "type": "blind" }),
            serde_json::json!({ "type": "confusion", "damageDice": 0, "damageSides": 0 }),
            serde_json::json!({ "type": "paralysis" }),
            serde_json::json!({ "type": "slow" }),
        ]);
    }
    melee_effect_json(effect, monster_level).map(|effect| vec![effect])
}

fn self_destruct_effect_is_supported(
    effect: &LegacyBlowEffect,
    monster_level: Option<u16>,
) -> bool {
    melee_effects_json(effect, monster_level).is_some_and(|effects| {
        effects.iter().all(|effect| {
            matches!(
                effect.get("type").and_then(serde_json::Value::as_str),
                Some("damage" | "poison" | "bomb" | "slow")
            )
        })
    })
}

fn blow_primary_dice(blow: &LegacyBlow) -> Option<(u16, u16)> {
    blow.effects.iter().find_map(|effect| {
        (effect.token == "POISON"
            || effect.token == "DISEASE"
            || effect.token == "CONFUSE"
            || melee_damage_type(&effect.token).is_some())
        .then_some(effect.dice)
        .flatten()
    })
}

/// Maps the first damage-bearing effect to the actor's legacy fallback attack.
fn damage_type_for(blow: &LegacyBlow) -> (&'static str, Option<&str>) {
    for effect in &blow.effects {
        if effect.dice.is_none() {
            continue;
        }
        if effect.token == "POISON" {
            return ("poison", None);
        }
        if effect.token == "DISEASE" {
            return ("physical", None);
        }
        if effect.token == "CONFUSE" {
            return ("confusion", None);
        }
        if let Some(damage_type) = melee_damage_type(&effect.token) {
            return (damage_type, None);
        }
        return ("physical", Some(&effect.token));
    }
    ("physical", None)
}

/// Mental attacks package psi damage with status riders; the legacy saving
/// throw and mana-drain side effects stay out of scope for now (documented
/// neutralisation, consistent with the earlier status families).
fn map_mental_spell_token(
    token: &str,
    level: u16,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let (base, explicit) = match token.split_once('(') {
        Some((base, rest)) => (base, Some(rest.strip_suffix(')')?)),
        None => (token, None),
    };
    let (default_dice, riders): (_, &[(&str, u32)]) = match base {
        "MIND_BLAST" => ((7, 7, 0), &[("confusion", 80)]),
        "BRAIN_SMASH" => (
            (12, 12, 0),
            &[
                ("blindness", 80),
                ("confusion", 80),
                ("paralysis", 20),
                ("slow", 80),
            ],
        ),
        "PSY_SPEAR" => {
            let (dice, sides, bonus) = match explicit {
                Some(spec) => parse_explicit_damage_dice(spec)?,
                None => (1, 3 * u32::from(level) / 2, 100),
            };
            let dice = dice.clamp(1, 100);
            let sides = sides.clamp(1, 10_000);
            let bonus = bonus.min(10_000);
            let mut suffix = format!("psy-spear-{dice}d{sides}");
            if bonus > 0 {
                suffix.push_str(&format!("-{bonus}"));
            }
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities
                .entry(id.clone())
                .or_insert_with(|| psi_beam_ability(&suffix, dice, sides, bonus));
            return Some(id);
        }
        _ => return None,
    };
    let (dice, sides, bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => default_dice,
    };
    let dice = dice.clamp(1, 100);
    let sides = sides.clamp(1, 10_000);
    let bonus = bonus.min(10_000);
    let base_slug = if base == "MIND_BLAST" {
        "mind-blast"
    } else {
        "brain-smash"
    };
    let mut suffix = format!("{base_slug}-{dice}d{sides}");
    if bonus > 0 {
        suffix.push_str(&format!("-{bonus}"));
    }
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities
        .entry(id.clone())
        .or_insert_with(|| psi_sequence_ability(&suffix, dice, sides, bonus, riders));
    Some(id)
}

/// The v99 misc pack: banishment, mana drain, amnesia and dispel map onto
/// their dedicated small effect forms; DISPEL_MAGIC strips the haste echo
/// (the only player buff status so far).
fn map_misc_spell_token(
    token: &str,
    level: u16,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    fn misc_ability(suffix: &str, effect: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
            "formatVersion": 1,
            "id": format!("rfb-legacy.ability.{suffix}"),
            "nameKey": format!("ability-legacy-{suffix}-name"),
            "descriptionKey": format!("ability-legacy-{suffix}-description"),
            "minimumLevel": 1,
            "resourceId": LEGACY_RESOURCE_ID,
            "resourceCost": 1,
            "baseFailurePercent": 20,
            "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
            "effect": effect,
            "tags": ["legacy-import", "misc"],
        })
    }
    match token {
        "TELE_OTHER" => {
            let id = "rfb-legacy.ability.banish".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                misc_ability(
                    "banish",
                    serde_json::json!({"type": "teleport-away", "minimumDistance": 10}),
                )
            });
            Some(id)
        }
        "DRAIN_MANA" => {
            let amount = (1 + u32::from(level) / 2).clamp(1, 1_000_000);
            let suffix = format!("drain-mana-{amount}");
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                misc_ability(
                    &suffix,
                    serde_json::json!({"type": "drain-resource", "amount": amount}),
                )
            });
            Some(id)
        }
        "AMNESIA" => {
            let id = "rfb-legacy.ability.amnesia".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| misc_ability("amnesia", serde_json::json!({"type": "amnesia"})));
            Some(id)
        }
        "DARKNESS" => {
            let id = "rfb-legacy.ability.darkness".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability =
                    misc_ability("darkness", serde_json::json!({"type": "darken-room"}));
                ability["target"]["requiresLineOfEffect"] = serde_json::json!(false);
                ability
            });
            Some(id)
        }
        "DISPEL_MAGIC" => {
            let id = "rfb-legacy.ability.dispel".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                misc_ability(
                    "dispel",
                    serde_json::json!({"type": "remove-status", "statusKindId": "rfb.status.haste"}),
                )
            });
            Some(id)
        }
        "ANTI_MAGIC" => {
            let id = "rfb-legacy.ability.anti-magic".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                misc_ability(
                    "anti-magic",
                    serde_json::json!({
                        "type": "apply-status",
                        "statusKindId": "rfb.status.anti-magic",
                        "intensity": 1,
                        "durationTicks": 3,
                        "durationDice": 1,
                        "durationSides": 3,
                        "stacking": "extend",
                        "resistanceType": "curse",
                        "power": level,
                    }),
                )
            });
            Some(id)
        }
        "ANIM_DEAD" => {
            let id = "rfb-legacy.ability.animate-dead".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability = misc_ability(
                    "animate-dead",
                    serde_json::json!({
                        "type": "sequence",
                        "effects": [
                            {
                                "type": "animate-dead",
                                "actorKindId": "demo.actor.risen-thrall",
                                "corpseItemKindId": DEMO_CORPSE_ITEM_ID,
                                "radius": 5,
                                "count": 8,
                                "failureChancePercent": 20
                            },
                            {
                                "type": "animate-dead",
                                "actorKindId": "demo.actor.risen-thrall",
                                "corpseItemKindId": DEMO_SKELETON_ITEM_ID,
                                "radius": 5,
                                "count": 8,
                                "failureChancePercent": 40
                            }
                        ]
                    }),
                );
                ability["target"] = serde_json::json!({
                    "modes": ["self"],
                    "range": 0,
                    "requiresLineOfEffect": false
                });
                ability["tags"] = serde_json::json!(["legacy-import", "summon", "undead"]);
                ability
            });
            Some(id)
        }
        "POLYMORPH" => {
            let id = "rfb-legacy.ability.polymorph-target".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                misc_ability(
                    "polymorph-target",
                    serde_json::json!({"type": "polymorph-target"}),
                )
            });
            Some(id)
        }
        _ => None,
    }
}

fn map_jump_spell_token(
    token: &str,
    level: u16,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let (base, explicit) = match token.split_once('(') {
        Some((base, rest)) => (base, Some(rest.strip_suffix(')')?)),
        None => (token, None),
    };
    let damage_type = match base {
        "JMP_FIRE" => "fire",
        "JMP_ICE" => "ice",
        "JMP_POISON" => "poison",
        "JMP_CONFUSION" => "confusion",
        "JMP_DARK" => "dark",
        "JMP_LIGHT" | "JMP_LITE" => "light",
        "JMP_NETHER" => "nether",
        "JMP_NEXUS" => "nexus",
        "JMP_SHARDS" => "shards",
        "JMP_DISINTEGRATE" => "disintegrate",
        "JMP_HELL_FIRE" => "hell-fire",
        _ => return None,
    };
    let (damage_dice, damage_sides, damage_bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => (0, 0, u32::from(level.max(1))),
    };
    let suffix = if damage_dice == 0 {
        format!("jump-{damage_type}-l{damage_bonus}")
    } else {
        let mut suffix = format!("jump-{damage_type}-{damage_dice}d{damage_sides}");
        if damage_bonus > 0 {
            suffix.push_str(&format!("-{damage_bonus}"));
        }
        suffix
    };
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities.entry(id.clone()).or_insert_with(|| {
        serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
            "formatVersion": 1,
            "id": id,
            "nameKey": format!("ability-legacy-{suffix}-name"),
            "descriptionKey": format!("ability-legacy-{suffix}-description"),
            "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
            "effect": {
                "type": "jump-damage",
                "damageDice": damage_dice,
                "damageSides": damage_sides,
                "damageBonus": damage_bonus,
                "damageMultiplierNumerator": 5,
                "damageMultiplierDenominator": 4,
                "damageType": damage_type,
                "radius": 5,
                "blinkRadius": 10
            },
            "tags": ["legacy-import", "innate", "offense"]
        })
    });
    Some(id)
}

/// CAUSE curses gate on the player's saving throw instead of armour or
/// resistances; HAND_DOOM additionally scales against current HP and cannot kill.
fn map_curse_spell_token(
    token: &str,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let (base, explicit) = match token.split_once('(') {
        Some((base, rest)) => (base, Some(rest.strip_suffix(')')?)),
        None => (token, None),
    };
    let default_dice = match base {
        "CAUSE_1" => (3, 8, 0),
        "CAUSE_2" => (8, 8, 0),
        "CAUSE_3" => (10, 15, 0),
        "CAUSE_4" => (15, 15, 0),
        "HAND_DOOM" => (1, 20, 40),
        _ => return None,
    };
    let (dice, sides, bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => default_dice,
    };
    let dice = dice.clamp(1, 100);
    let sides = sides.clamp(1, 10_000);
    let bonus = bonus.min(10_000);
    let mut suffix = if base == "HAND_DOOM" {
        "hand-of-doom".to_owned()
    } else {
        format!("curse-{dice}d{sides}")
    };
    if bonus > 0 && base != "HAND_DOOM" {
        suffix.push_str(&format!("-{bonus}"));
    }
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities.entry(id.clone()).or_insert_with(|| {
        let mut effect = serde_json::json!({
            "type": "curse-damage",
            "damageDice": dice,
            "damageSides": sides,
        });
        if bonus > 0 {
            effect["damageBonus"] = serde_json::json!(bonus);
        }
        if base == "HAND_DOOM" {
            effect["damageIsCurrentHpPercent"] = serde_json::json!(true);
            effect["nonlethal"] = serde_json::json!(true);
        }
        serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
            "formatVersion": 1,
            "id": format!("rfb-legacy.ability.{suffix}"),
            "nameKey": format!("ability-legacy-{suffix}-name"),
            "descriptionKey": format!("ability-legacy-{suffix}-description"),
            "minimumLevel": 1,
            "resourceId": LEGACY_RESOURCE_ID,
            "resourceCost": 1,
            "baseFailurePercent": 20,
            "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
            "effect": effect,
            "tags": ["curse", "legacy-import"],
        })
    });
    Some(id)
}

fn psi_damage_effect(dice: u32, sides: u32, bonus: u32) -> serde_json::Value {
    let mut effect = serde_json::json!({
        "type": "damage",
        "damageDice": dice,
        "damageSides": sides,
        "damageType": "psi",
    });
    if bonus > 0 {
        effect["damageBonus"] = serde_json::json!(bonus);
    }
    effect
}

fn psi_sequence_ability(
    suffix: &str,
    dice: u32,
    sides: u32,
    bonus: u32,
    riders: &[(&str, u32)],
) -> serde_json::Value {
    let mut effects = vec![psi_damage_effect(dice, sides, bonus)];
    for (status, duration_ticks) in riders {
        effects.push(serde_json::json!({
            "type": "apply-status",
            "statusKindId": format!("rfb.status.{status}"),
            "intensity": 1,
            "durationTicks": duration_ticks,
            "stacking": "extend",
            "resistanceType": "psi",
        }));
    }
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
        "effect": { "type": "sequence", "effects": effects },
        "tags": ["legacy-import", "psi"],
    })
}

fn psi_beam_ability(suffix: &str, dice: u32, sides: u32, bonus: u32) -> serde_json::Value {
    let mut effect = serde_json::json!({
        "type": "beam-damage",
        "damageDice": dice,
        "damageSides": sides,
        "damageType": "psi",
    });
    if bonus > 0 {
        effect["damageBonus"] = serde_json::json!(bonus);
    }
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
        "effect": effect,
        "tags": ["beam", "legacy-import", "psi"],
    })
}

/// Legacy resistance-flag suffixes and their damage-type names; applied in
/// RES -> IM -> HURT order so immunities and vulnerabilities win.
const RESISTANCE_FLAG_TYPES: [(&str, &str); 19] = [
    ("ACID", "acid"),
    ("ELEC", "electricity"),
    ("FIRE", "fire"),
    ("COLD", "cold"),
    ("POIS", "poison"),
    ("LITE", "light"),
    ("DARK", "dark"),
    ("NETH", "nether"),
    ("NEXU", "nexus"),
    ("SOUN", "sound"),
    ("SHAR", "shards"),
    ("CHAO", "chaos"),
    ("DISE", "disenchant"),
    ("TIME", "time"),
    ("GRAV", "gravity"),
    ("INER", "inertia"),
    ("PLAS", "plasma"),
    ("WATE", "water"),
    ("DISI", "disintegrate"),
];

const RESISTANCE_ALL_TYPES: [&str; 30] = [
    "acid",
    "electricity",
    "fire",
    "cold",
    "poison",
    "light",
    "dark",
    "confusion",
    "nether",
    "nexus",
    "sound",
    "shards",
    "chaos",
    "disenchant",
    "time",
    "mana",
    "gravity",
    "inertia",
    "plasma",
    "force",
    "nuke",
    "disintegrate",
    "storm",
    "holy-fire",
    "hell-fire",
    "ice",
    "water",
    "meteor",
    "rocket",
    "telekinesis",
];

const MONSTER_CONTACT_AURA_FLAGS: [(&str, &str); 3] = [
    ("AURA_FIRE", "fire"),
    ("AURA_COLD", "cold"),
    ("AURA_ELEC", "electricity"),
];

fn monster_flag_is_mapped(flag: &str) -> bool {
    if matches!(
        flag,
        "RES_ALL"
            | "RES_TELE"
            | "NO_CONF"
            | "NO_STUN"
            | "NEVER_MOVE"
            | "NEVER_BLOW"
            | "KILL_WALL"
            | "KILL_ITEM"
            | "TAKE_ITEM"
            | "HAS_LITE_1"
            | "HAS_LITE_2"
            | "SELF_LITE_1"
            | "SELF_LITE_2"
            | "HAS_DARK_1"
            | "HAS_DARK_2"
            | "SELF_DARK_1"
            | "SELF_DARK_2"
            | "FORCE_SLEEP"
            | "TRUMP"
            | "QUANTUM"
            | "CLEAR_HEAD"
            | "AMBERITE"
            | "ONLY_ITEM"
            | "ONLY_GOLD"
            | "DROP_60"
            | "DROP_90"
            | "DROP_1D2"
            | "DROP_2D2"
            | "DROP_3D2"
            | "DROP_4D2"
            | "DROP_GOOD"
            | "DROP_GREAT"
            | "ELDRITCH_HORROR"
            | "CHAMELEON"
            | "SHAPECHANGER"
            | "HURT_ROCK"
            | "CAN_CLIMB"
            | "COMPOST"
            | "FIXED_UNIQUE"
            | "NO_QUEST"
            | "COLD_BLOOD"
            | "NORSE"
            | "HINDU"
            | "EGYPTIAN"
            | "OLYMPIAN"
            | "NO_SUMMON"
            | "KNIGHT"
    ) {
        return true;
    }
    if flag.starts_with("DUNGEON_") {
        return true;
    }
    for (suffix, _) in RESISTANCE_FLAG_TYPES {
        if flag.strip_prefix("RES_") == Some(suffix)
            || flag.strip_prefix("IM_") == Some(suffix)
            || flag.strip_prefix("HURT_") == Some(suffix)
        {
            return true;
        }
    }
    false
}

fn monster_death_drop_json(entry: &LegacyMonsterEntry) -> Option<serde_json::Value> {
    let mut chance_rolls = Vec::new();
    if entry.flags.iter().any(|flag| flag == "DROP_60") {
        chance_rolls.push(serde_json::json!({ "percent": 60 }));
    }
    if entry.flags.iter().any(|flag| flag == "DROP_90") {
        chance_rolls.push(serde_json::json!({
            "percent": 90,
            "guaranteedForUnique": true,
        }));
    }
    let count_dice = [
        ("DROP_1D2", 1_u8),
        ("DROP_2D2", 2),
        ("DROP_3D2", 3),
        ("DROP_4D2", 4),
    ]
    .into_iter()
    .filter(|(flag, _)| entry.flags.iter().any(|candidate| candidate == flag))
    .map(|(_, dice)| serde_json::json!({ "dice": dice, "sides": 2 }))
    .collect::<Vec<_>>();
    if chance_rolls.is_empty() && count_dice.is_empty() {
        return None;
    }
    let only_items = entry.flags.iter().any(|flag| flag == "ONLY_ITEM");
    let only_gold = entry.flags.iter().any(|flag| flag == "ONLY_GOLD");
    let kind = if only_items {
        "items"
    } else if only_gold {
        "gold"
    } else {
        "items-and-gold"
    };
    let allows_items = !only_gold;
    let minimum_quality = if entry.flags.iter().any(|flag| flag == "DROP_GREAT") {
        "exceptional"
    } else if entry.flags.iter().any(|flag| flag == "DROP_GOOD") {
        "fine"
    } else {
        "ordinary"
    };
    let mut value = serde_json::json!({
        "kind": kind,
        "chanceRolls": chance_rolls,
        "countDice": count_dice,
        "minimumQuality": minimum_quality,
    });
    if allows_items {
        value["itemTableId"] = serde_json::json!(LEGACY_DROP_TABLE_ID);
        if entry.drop_theme.as_deref() == Some("DROP_WARRIOR") {
            value["themeTableId"] = serde_json::json!(LEGACY_WARRIOR_DROP_TABLE_ID);
            value["themeChancePercent"] = serde_json::json!(50);
        }
    }
    Some(value)
}

/// Folds RES_/IM_/HURT_ flags into a content resistance map; later tiers
/// (immunity, then vulnerability) overwrite earlier ones for the same type.
fn resistances_from_flags(flags: &[String]) -> BTreeMap<&'static str, &'static str> {
    let mut resistances = BTreeMap::new();
    if flags.iter().any(|flag| flag == "RES_ALL") {
        for damage_type in RESISTANCE_ALL_TYPES {
            resistances.insert(damage_type, "resistant");
        }
    }
    for (prefix, level) in [
        ("RES_", "resistant"),
        ("IM_", "immune"),
        ("HURT_", "vulnerable"),
    ] {
        for (suffix, damage_type) in RESISTANCE_FLAG_TYPES {
            if flags
                .iter()
                .any(|flag| flag.strip_prefix(prefix) == Some(suffix))
            {
                resistances.insert(damage_type, level);
            }
        }
    }
    if flags.iter().any(|flag| flag == "HURT_ROCK") {
        resistances.insert("disintegrate", "vulnerable");
    }
    resistances
}

fn monster_json(
    entry: &LegacyMonsterEntry,
    id: &str,
    blow: Option<&LegacyBlow>,
    damage_type: &str,
    melee_routine: Option<serde_json::Value>,
    monster_casting: Option<serde_json::Value>,
) -> serde_json::Value {
    let (hp_dice, hp_sides) = entry.hp_dice.unwrap_or((1, 1));
    let force_maximum = entry.flags.iter().any(|flag| flag == "FORCE_MAXHP");
    let max_hp = if force_maximum {
        hp_dice.saturating_mul(hp_sides)
    } else {
        (hp_dice.saturating_mul(hp_sides.saturating_add(1)) / 2).max(1)
    };
    let level = entry.level.unwrap_or(1);
    let (damage_dice, damage_sides) = blow.and_then(blow_primary_dice).unwrap_or((1, 1));
    // Legacy type flags become category tags so summon filters can select
    // by monster class; the shared legacy-import tag doubles as "any".
    let mut tags = vec!["legacy-import".to_owned()];
    if let Some(glyph) = entry.glyph {
        tags.push(format!("kin-glyph-{}", u32::from(glyph)));
    }
    if entry.glyph == Some('M') {
        tags.push("hydra".to_owned());
    }
    if matches!(entry.glyph, Some('C' | 'Z')) {
        tags.push("hound".to_owned());
    }
    if entry.glyph == Some('A')
        && entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "EVIL" | "GOOD"))
    {
        tags.push("angel".to_owned());
    }
    if entry.index == 286 {
        tags.push("gelatinous-cube".to_owned());
    }
    if entry.index == 622 {
        tags.push("night-mare".to_owned());
    }
    if entry.index == 816 {
        tags.push("cyber".to_owned());
    }
    for (flag, tag) in [
        ("ANIMAL", "animal"),
        ("EVIL", "evil"),
        ("GOOD", "good"),
        ("HUMAN", "human"),
        ("DEMON", "demon"),
        ("DRAGON", "dragon"),
        ("UNDEAD", "undead"),
        ("ORC", "orc"),
        ("TROLL", "troll"),
        ("GIANT", "giant"),
        ("NONLIVING", "nonliving"),
        ("COLD_BLOOD", "cold-blooded"),
        ("EMPTY_MIND", "empty-mind"),
        ("WEIRD_MIND", "weird-mind"),
        ("AURA_REVENGE", "aura-revenge"),
        ("AURA_FEAR", "aura-fear"),
        ("TANUKI", "tanuki"),
        ("UNIQUE2", "unique2"),
        ("NAZGUL", "unique"),
        ("TRUMP", "trump"),
        ("QUANTUM", "quantum"),
        ("CLEAR_HEAD", "clear-head"),
        ("AMBERITE", "amberite"),
        ("NORSE", "norse"),
        ("HINDU", "hindu"),
        ("EGYPTIAN", "egyptian"),
        ("OLYMPIAN", "olympian"),
        ("NO_SUMMON", "no-summon"),
        ("KNIGHT", "knight"),
    ] {
        if entry.flags.iter().any(|value| value == flag) {
            tags.push(tag.to_owned());
        }
    }
    if entry
        .flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "UNDEAD" | "DEMON" | "NONLIVING"))
        && !tags.iter().any(|tag| tag == "nonliving")
    {
        tags.push("nonliving".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "UNIQUE") {
        tags.push("unique".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "RES_ALL") {
        tags.push("resist-all".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "RES_TELE") {
        tags.push("resist-teleport".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "INVISIBLE") {
        tags.push("invisible".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "SHAPECHANGER") {
        tags.push("shapechanger".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "CHAMELEON") {
        tags.push("chameleon".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "ELDRITCH_HORROR") {
        tags.push("eldritch-horror".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "FIXED_UNIQUE") {
        tags.push("fixed-unique".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "NO_QUEST") {
        tags.push("no-quest".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "KNIGHT")
        && entry.flags.iter().any(|flag| flag == "DUNGEON_2")
    {
        tags.push("camelot-knight".to_owned());
    }
    if entry
        .glyph
        .is_some_and(|glyph| matches!(glyph, 'L' | 'V' | 'W'))
    {
        tags.push("high-undead".to_owned());
    }
    let living = !tags.iter().any(|tag| tag == "nonliving");
    if living {
        tags.push("living".to_owned());
    }
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/actor.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.actor.{id}"),
        "role": "monster",
        "nameKey": format!("actor-legacy-{id}-name"),
        "descriptionKey": format!("actor-legacy-{id}-description"),
        "glyph": entry.glyph.map_or_else(|| "?".to_owned(), |glyph| glyph.to_string()),
        "level": level,
        "experienceValue": u32::from(level) * 10,
        "maxHp": max_hp,
        "hitPointDice": {
            "dice": hp_dice,
            "sides": hp_sides,
            "forceMaximum": force_maximum
        },
        "speed": entry.speed.unwrap_or(110),
        "attack": (i32::from(level) / 4).max(1),
        "defense": (entry.armor_class.unwrap_or(0) / 10).max(0),
        "damageDice": damage_dice,
        "damageSides": damage_sides.max(1),
        "damageType": damage_type,
        "tags": tags,
    });
    let capture_policy = if entry
        .flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "UNIQUE2" | "QUESTOR"))
        || matches!(entry.index, 932..=934)
    {
        "immune"
    } else if entry
        .flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "UNIQUE" | "NAZGUL"))
    {
        "pet-only"
    } else {
        "normal"
    };
    if capture_policy != "normal" {
        value["capturePolicy"] = serde_json::json!(capture_policy);
    }
    if entry.flags.iter().any(|flag| flag == "NAZGUL") {
        value["lifetimeInstanceLimit"] = serde_json::json!(5);
    }
    let resistances = resistances_from_flags(&entry.flags);
    if !resistances.is_empty() {
        value["resistances"] = serde_json::json!(resistances);
    }
    let status_immunities = [
        ("NO_CONF", "rfb.status.confusion"),
        ("NO_STUN", "rfb.status.stun"),
    ]
    .into_iter()
    .filter_map(|(flag, status)| {
        entry
            .flags
            .iter()
            .any(|value| value == flag)
            .then_some(status)
    })
    .collect::<Vec<_>>();
    if !status_immunities.is_empty() {
        value["statusImmunities"] = serde_json::json!(status_immunities);
    }
    let movement_modes = [
        ("AQUATIC", "aquatic"),
        ("CAN_CLIMB", "climb"),
        ("CAN_FLY", "fly"),
        ("PASS_WALL", "pass-wall"),
        ("CAN_SWIM", "swim"),
    ]
    .into_iter()
    .filter_map(|(flag, mode)| {
        entry
            .flags
            .iter()
            .any(|value| value == flag)
            .then_some(mode)
    })
    .collect::<Vec<_>>();
    let never_moves = entry.flags.iter().any(|flag| flag == "NEVER_MOVE");
    if !movement_modes.is_empty() || never_moves {
        value["movement"] = serde_json::json!({ "modes": movement_modes });
        if never_moves {
            value["movement"]["neverMoves"] = serde_json::json!(true);
        }
    }
    for (flag, field) in [
        ("KILL_BODY", "killsWeakerBodies"),
        ("MOVE_BODY", "movesWeakerBodies"),
        ("REGENERATE", "regenerates"),
        ("REFLECTING", "reflectsBolts"),
        ("RANGED_MELEE", "rangedMelee"),
        ("RIDING", "rideable"),
        ("SILVER", "madeOfSilver"),
    ] {
        if entry.flags.iter().any(|value| value == flag) {
            value[field] = serde_json::json!(true);
        }
    }
    let opens = entry.flags.iter().any(|flag| flag == "OPEN_DOOR");
    let bashes = entry.flags.iter().any(|flag| flag == "BASH_DOOR");
    if opens || bashes {
        value["doorInteraction"] = serde_json::json!({
            "opens": opens,
            "bashes": bashes
        });
    }
    let destroys_walls = entry.flags.iter().any(|flag| flag == "KILL_WALL");
    let destroys_items = entry.flags.iter().any(|flag| flag == "KILL_ITEM");
    let picks_up_items = entry.flags.iter().any(|flag| flag == "TAKE_ITEM");
    if destroys_walls || destroys_items || picks_up_items {
        value["terrainInteraction"] = serde_json::json!({
            "destroysWalls": destroys_walls,
            "destroysItems": destroys_items,
            "picksUpItems": picks_up_items,
        });
    }
    let light_radius = i8::from(
        entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "HAS_LITE_1" | "SELF_LITE_1")),
    ) + 2 * i8::from(
        entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "HAS_LITE_2" | "SELF_LITE_2")),
    ) - i8::from(
        entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "HAS_DARK_1" | "SELF_DARK_1")),
    ) - 2 * i8::from(
        entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "HAS_DARK_2" | "SELF_DARK_2")),
    );
    if light_radius != 0 {
        let darkness = light_radius < 0;
        value["light"] = serde_json::json!({
            "radius": light_radius.unsigned_abs(),
            "intrinsic": entry.flags.iter().any(|flag| if darkness {
                matches!(flag.as_str(), "SELF_DARK_1" | "SELF_DARK_2")
            } else {
                matches!(flag.as_str(), "SELF_LITE_1" | "SELF_LITE_2")
            }),
        });
        if darkness {
            value["light"]["darkness"] = serde_json::json!(true);
        }
    }
    if entry.flags.iter().any(|flag| flag == "FORCE_SLEEP") {
        value["forceSleep"] = serde_json::json!(true);
    }
    if let Some(death_drop) = monster_death_drop_json(entry) {
        value["deathDrop"] = death_drop;
    }
    if let Some(routine) = melee_routine {
        value["meleeRoutine"] = routine;
    }
    let mut contact_auras = entry
        .auras
        .iter()
        .filter_map(|aura| {
            let damage_type = match aura.token.as_str() {
                "POISON" => "poison",
                "ACID" => "acid",
                "ELEC" => "electricity",
                "FIRE" => "fire",
                "ICE" => "ice",
                "LIGHT" | "LITE" => "light",
                "DARK" => "dark",
                "NETHER" => "nether",
                "PLASMA" => "plasma",
                "HELL_FIRE" => "hell-fire",
                "DISINTEGRATE" => "disintegrate",
                "HOLY_FIRE" => "holy-fire",
                "CAUSE_2" | "CAUSE_3" => "curse",
                "SHARDS" => "shards",
                "TIME" => "time",
                "CHAOS" => "chaos",
                "DISENCHANT" => "disenchant",
                _ => return None,
            };
            let (damage_dice, damage_sides) = aura.dice?;
            let mut value = serde_json::json!({
                "damageType": damage_type,
                "damageDice": damage_dice,
                "damageSides": damage_sides,
            });
            if let Some(chance_percent) = aura.chance_percent {
                value["chancePercent"] = serde_json::json!(chance_percent);
            }
            if aura.token == "TIME" {
                value["ravagesTime"] = serde_json::json!(true);
            }
            Some(value)
        })
        .collect::<Vec<_>>();
    let level = entry.level.unwrap_or_default();
    contact_auras.extend(
        MONSTER_CONTACT_AURA_FLAGS
            .iter()
            .filter(|(flag, _)| entry.flags.iter().any(|entry_flag| entry_flag == flag))
            .map(|(_, damage_type)| {
                serde_json::json!({
                    "damageType": damage_type,
                    "damageDice": 1 + level / 26,
                    "damageSides": 1 + level / 17,
                })
            }),
    );
    if !contact_auras.is_empty() {
        value["contactAuras"] = serde_json::json!(contact_auras);
    }
    let contact_effects = entry
        .auras
        .iter()
        .filter(|aura| matches!(aura.token.as_str(), "UNLIFE" | "STUN"))
        .filter_map(|aura| melee_effect_json(aura, entry.level))
        .collect::<Vec<_>>();
    if !contact_effects.is_empty() {
        value["contactEffects"] = serde_json::json!(contact_effects);
    }
    if let Some(casting) = monster_casting {
        value["monsterCasting"] = casting;
    }
    if living {
        value["corpseItemKindId"] = serde_json::json!(LEGACY_CORPSE_ITEM_ID);
    }
    if let Some(rarity) = entry.rarity.filter(|rarity| *rarity > 0) {
        let friends = entry.flags.iter().find_map(|flag| {
            if flag == "FRIENDS" {
                return Some(serde_json::json!({
                    "dice": 0,
                    "sides": 0,
                    "chancePercent": 0
                }));
            }
            let body = flag.strip_prefix("FRIENDS(")?.strip_suffix(')')?;
            let mut parts = body.split(',').map(str::trim);
            let (dice, sides): (u8, u8) =
                parse_dice(R_INFO_SOURCE, 0, "FRIENDS", parts.next()).ok()?;
            let chance_percent = parts
                .next()
                .and_then(|chance| chance.strip_suffix('%'))
                .and_then(|chance| chance.parse::<u8>().ok())
                .unwrap_or(0);
            Some(serde_json::json!({
                "dice": dice,
                "sides": sides,
                "chancePercent": chance_percent
            }))
        });
        let random_movement_percent = u8::from(entry.flags.iter().any(|flag| flag == "RAND_25"))
            * 25
            + u8::from(entry.flags.iter().any(|flag| flag == "RAND_50")) * 50;
        let mut allocation = serde_json::json!({
            "legacyIndex": entry.index,
            "rarity": rarity,
            "maxDepth": entry.max_level.unwrap_or(0),
            "forceDepth": entry.flags.iter().any(|flag| flag == "FORCE_DEPTH"),
            "wildOnly": entry.flags.iter().any(|flag| matches!(flag.as_str(), "WILD_ONLY" | "WILD_OCEAN")),
            "escort": entry.flags.iter().any(|flag| flag == "ESCORT"),
            "multiplies": entry.flags.iter().any(|flag| flag == "MULTIPLY"),
            "randomMovementPercent": random_movement_percent,
        });
        if entry.flags.iter().any(|flag| flag == "COMPOST") {
            allocation["taskId"] = serde_json::json!("demo.task.the-sewer");
        }
        let habitats = [
            ("WILD_ALL", "all"),
            ("WILD_GRASS", "grass"),
            ("WILD_MOUNTAIN", "mountain"),
            ("WILD_OCEAN", "ocean"),
            ("WILD_SHORE", "shore"),
            ("WILD_SNOW", "snow"),
            ("WILD_SWAMP", "swamp"),
            ("WILD_TOWN", "town"),
            ("WILD_VOLCANO", "volcano"),
            ("WILD_WASTE", "waste"),
            ("WILD_WOOD", "wood"),
        ]
        .into_iter()
        .filter_map(|(flag, habitat)| {
            entry
                .flags
                .iter()
                .any(|value| value == flag)
                .then_some(habitat)
        })
        .collect::<Vec<_>>();
        if !habitats.is_empty() {
            allocation["habitats"] = serde_json::json!(habitats);
        }
        let mut legacy_dungeon_indices = entry
            .flags
            .iter()
            .filter_map(|flag| flag.strip_prefix("DUNGEON_")?.parse::<u16>().ok())
            .collect::<Vec<_>>();
        legacy_dungeon_indices.sort_unstable();
        legacy_dungeon_indices.dedup();
        if !legacy_dungeon_indices.is_empty() {
            allocation["legacyDungeonIndices"] = serde_json::json!(legacy_dungeon_indices);
        }
        if let Some(friends) = friends {
            allocation["friends"] = friends;
        }
        value["allocation"] = allocation;
    }
    value
}

fn demo_monster_flag_is_handled(flag: &str) -> bool {
    MONSTER_CONTACT_AURA_FLAGS
        .iter()
        .any(|(aura_flag, _)| flag == *aura_flag)
        || monster_flag_is_mapped(flag)
        || matches!(
            flag,
            "FORCE_MAXHP"
                | "ANIMAL"
                | "EVIL"
                | "GOOD"
                | "HUMAN"
                | "DEMON"
                | "DRAGON"
                | "UNDEAD"
                | "ORC"
                | "TROLL"
                | "GIANT"
                | "NONLIVING"
                | "UNIQUE"
                | "GUARDIAN"
                | "THIEF"
                | "AQUATIC"
                | "CAN_FLY"
                | "CAN_SWIM"
                | "PASS_WALL"
                | "KILL_BODY"
                | "MOVE_BODY"
                | "REGENERATE"
                | "REFLECTING"
                | "RANGED_MELEE"
                | "RIDING"
                | "SILVER"
                | "FRIENDLY"
                | "KAGE"
                | "OPEN_DOOR"
                | "BASH_DOOR"
                | "DROP_CORPSE"
                | "DROP_SKELETON"
                | "NO_FEAR"
                | "NO_SLEEP"
                | "FORCE_DEPTH"
                | "WILD_ONLY"
                | "WILD_OCEAN"
                | "ESCORT"
                | "MULTIPLY"
                | "RAND_25"
                | "RAND_50"
                | "FRIENDS"
                | "INVISIBLE"
                | "EMPTY_MIND"
                | "WEIRD_MIND"
                | "SMART"
                | "ELDRITCH_HORROR"
                | "CHAMELEON"
                | "SHAPECHANGER"
                | "AURA_REVENGE"
                | "AURA_FEAR"
                | "TANUKI"
                | "UNIQUE2"
                | "NAZGUL"
                | "WILD_ALL"
                | "WILD_GRASS"
                | "WILD_MOUNTAIN"
                | "WILD_SHORE"
                | "WILD_SNOW"
                | "WILD_SWAMP"
                | "WILD_TOWN"
                | "WILD_VOLCANO"
                | "WILD_WASTE"
                | "WILD_WOOD"
        )
        || flag.starts_with("FRIENDS(")
        || flag.starts_with("DUNGEON_")
}

fn demo_monster_omitted_flags(entry: &LegacyMonsterEntry) -> BTreeSet<String> {
    entry
        .flags
        .iter()
        .filter(|flag| !demo_monster_flag_is_handled(flag))
        .cloned()
        .collect()
}

fn demo_monster_json(
    entry: &LegacyMonsterEntry,
    selection: &DemoMonsterSelectionEntry,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Result<serde_json::Value, LegacyImportError> {
    let declared_omissions = selection
        .omitted_flags
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if declared_omissions.len() != selection.omitted_flags.len() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} contains duplicate omitted flags",
            selection.id
        )));
    }
    let actual_omissions = demo_monster_omitted_flags(entry);
    if declared_omissions != actual_omissions {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} omitted flags differ: declared {declared_omissions:?}, source {actual_omissions:?}",
            selection.id
        )));
    }

    let declared_spell_omissions = selection
        .omitted_spells
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if declared_spell_omissions.len() != selection.omitted_spells.len() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} contains duplicate omitted spells",
            selection.id
        )));
    }
    let mut used_spell_omissions = BTreeSet::new();

    let mut frequency_percent = None;
    let mut ability_ids = Vec::new();
    let level = entry.level.unwrap_or(1).max(1);
    let breath_radius = if level >= 50 || entry.glyph == Some('D') {
        3
    } else {
        2
    };
    for spell in &entry.spells {
        if let Some(divisor) = spell.strip_prefix("1_IN_") {
            let divisor = divisor.parse::<u32>().map_err(|_| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "{} has invalid casting frequency {spell}",
                    selection.id
                ))
            })?;
            frequency_percent = Some((100 / divisor.max(1)).clamp(1, 100));
            continue;
        }
        if let Some(percent) = spell.strip_prefix("FREQ_") {
            let percent = percent.parse::<u32>().map_err(|_| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "{} has invalid casting frequency {spell}",
                    selection.id
                ))
            })?;
            frequency_percent = Some(percent.clamp(1, 100));
            continue;
        }
        let base_token = spell.split('(').next().unwrap_or(spell);
        if POSSESSOR_ONLY_SPELLS.contains(&base_token) {
            continue;
        }
        let ability_id = if base_token == "TRAPS" {
            let id = "rfb-legacy.ability.traps".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(demo_traps_ability);
            Some(id)
        } else {
            map_spell_token(
                spell,
                level,
                breath_radius,
                &format!("demo.actor.{}", selection.id),
                abilities,
            )
        };
        let Some(ability_id) = ability_id else {
            if declared_spell_omissions.contains(spell) {
                used_spell_omissions.insert(spell.clone());
                continue;
            }
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "{} has unsupported monster spell {spell}",
                selection.id
            )));
        };
        if !ability_ids.contains(&ability_id) {
            ability_ids.push(ability_id);
        }
    }
    if used_spell_omissions != declared_spell_omissions {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} omitted spells differ: declared {declared_spell_omissions:?}, unsupported {used_spell_omissions:?}",
            selection.id
        )));
    }
    let monster_casting = (!ability_ids.is_empty()).then(|| {
        let mut casting = serde_json::json!({
            "frequencyPercent": frequency_percent.unwrap_or(10),
            "abilities": ability_ids
                .iter()
                .map(|ability_id| serde_json::json!({ "abilityId": ability_id, "weight": 1 }))
                .collect::<Vec<_>>(),
        });
        if entry.flags.iter().any(|flag| flag == "SMART") {
            casting["smart"] = serde_json::json!(true);
        }
        casting
    });
    if entry
        .drop_theme
        .as_deref()
        .is_some_and(|theme| demo_drop_theme_table_id(theme).is_none())
    {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} requires a formal themed drop table",
            selection.id
        )));
    }
    if entry.auras.len()
        + MONSTER_CONTACT_AURA_FLAGS
            .iter()
            .filter(|(flag, _)| entry.flags.iter().any(|entry_flag| entry_flag == flag))
            .count()
        > 8
        || entry.auras.iter().any(|aura| {
            !matches!(
                aura.token.as_str(),
                "POISON"
                    | "ACID"
                    | "ELEC"
                    | "FIRE"
                    | "ICE"
                    | "LIGHT"
                    | "LITE"
                    | "DARK"
                    | "NETHER"
                    | "PLASMA"
                    | "HELL_FIRE"
                    | "DISINTEGRATE"
                    | "HOLY_FIRE"
                    | "CAUSE_2"
                    | "CAUSE_3"
                    | "SHARDS"
                    | "UNLIFE"
                    | "STUN"
                    | "TIME"
                    | "CHAOS"
                    | "DISENCHANT"
            ) || aura.dice.is_none()
        })
    {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} has an unsupported contact aura",
            selection.id
        )));
    }
    let primary_blow = entry.blows.first();
    if primary_blow.is_none()
        && selection.id != "artemis-the-moon-goddess"
        && !entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "NEVER_BLOW" | "KAGE" | "CHAMELEON"))
    {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "{} has no melee routine and is not marked NEVER_BLOW",
            selection.id
        )));
    }
    let mut blows = Vec::with_capacity(entry.blows.len());
    for blow in &entry.blows {
        // BEG is an always-successful, observable action despite having no
        // effect payload. Other presentational methods remain out of scope.
        if blow.effects.is_empty() && blow.method != "BEG" {
            continue;
        }
        if blow.method == "EXPLODE"
            && let Some(effect) = blow
                .effects
                .iter()
                .find(|effect| !self_destruct_effect_is_supported(effect, entry.level))
        {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "{} has unsupported self-destruct effect {}",
                selection.id, effect.token
            )));
        }
        let effects = blow
            .effects
            .iter()
            .map(|effect| {
                melee_effects_json(effect, entry.level).ok_or_else(|| {
                    LegacyImportError::InvalidDemoMonsterSelection(format!(
                        "{} has unsupported blow effect {}",
                        selection.id, effect.token
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let mut method = kebab(&blow.method);
        if method.is_empty() {
            method = "strike".to_owned();
        }
        let mut value = serde_json::json!({
            "methodId": format!("rfb.blow.{method}"),
            "toHit": 20,
            "effects": effects,
        });
        if blow.method == "EXPLODE" {
            value["selfDestructs"] = serde_json::json!(true);
        }
        blows.push(value);
    }
    let damage_type = primary_blow.map_or("physical", |blow| damage_type_for(blow).0);
    let routine = serde_json::json!({ "blows": blows });
    let mut value = monster_json(
        entry,
        &selection.id,
        primary_blow,
        damage_type,
        Some(routine),
        monster_casting,
    );
    value["id"] = serde_json::json!(format!("demo.actor.{}", selection.id));
    value["nameKey"] = serde_json::json!(format!("actor-demo-{}-name", selection.id));
    value["descriptionKey"] = serde_json::json!(format!("actor-demo-{}-description", selection.id));
    value["experienceValue"] = serde_json::json!(entry.experience.unwrap_or(0));
    value
        .as_object_mut()
        .expect("actor JSON must be an object")
        .remove("corpseItemKindId");
    if entry.flags.iter().any(|flag| flag == "FRIENDLY") {
        value["friendly"] = serde_json::json!(true);
    }
    if selection.tags.iter().any(|tag| tag == "fixed-placement") {
        value
            .as_object_mut()
            .expect("actor JSON must be an object")
            .remove("allocation");
    }
    // Rolento's grenade exists only as a fixed summon; legacy rarity 255 is
    // a sentinel, not a low-probability global allocation weight.
    if entry.index == 1023 {
        value
            .as_object_mut()
            .expect("actor JSON must be an object")
            .remove("allocation");
    }
    if entry.flags.iter().any(|flag| flag == "KAGE") {
        value
            .as_object_mut()
            .expect("actor JSON must be an object")
            .remove("allocation");
        value["meleeRoutine"] = serde_json::json!({ "blows": [] });
    }

    let mut tags = selection.tags.iter().cloned().collect::<BTreeSet<_>>();
    tags.insert("legacy-import".to_owned());
    if let Some(glyph) = entry.glyph {
        tags.insert(format!("kin-glyph-{}", u32::from(glyph)));
    }
    if entry.glyph == Some('M') {
        tags.insert("hydra".to_owned());
    }
    if matches!(entry.glyph, Some('C' | 'Z')) {
        tags.insert("hound".to_owned());
    }
    if entry.glyph == Some('A')
        && entry
            .flags
            .iter()
            .any(|flag| matches!(flag.as_str(), "EVIL" | "GOOD"))
    {
        tags.insert("angel".to_owned());
    }
    if entry.index == 286 {
        tags.insert("gelatinous-cube".to_owned());
    }
    if entry.index == 622 {
        tags.insert("night-mare".to_owned());
    }
    if entry.index == 816 {
        tags.insert("cyber".to_owned());
    }
    if entry.flags.iter().any(|flag| flag == "KAGE") {
        tags.insert("shadower-appearance".to_owned());
    }
    for (flag, tag) in [
        ("ANIMAL", "animal"),
        ("EVIL", "evil"),
        ("GOOD", "good"),
        ("HUMAN", "human"),
        ("DEMON", "demon"),
        ("DRAGON", "dragon"),
        ("UNDEAD", "undead"),
        ("ORC", "orc"),
        ("TROLL", "troll"),
        ("GIANT", "giant"),
        ("NONLIVING", "nonliving"),
        ("GUARDIAN", "guardian"),
        ("COLD_BLOOD", "cold-blooded"),
        ("UNIQUE", "unique"),
        ("THIEF", "thief"),
        ("INVISIBLE", "invisible"),
        ("RES_ALL", "resist-all"),
        ("RES_TELE", "resist-teleport"),
        ("ELDRITCH_HORROR", "eldritch-horror"),
        ("CHAMELEON", "chameleon"),
        ("SHAPECHANGER", "shapechanger"),
        ("FIXED_UNIQUE", "fixed-unique"),
        ("NO_QUEST", "no-quest"),
        ("EMPTY_MIND", "empty-mind"),
        ("WEIRD_MIND", "weird-mind"),
        ("AURA_REVENGE", "aura-revenge"),
        ("AURA_FEAR", "aura-fear"),
        ("TANUKI", "tanuki"),
        ("UNIQUE2", "unique2"),
        ("NAZGUL", "unique"),
        ("TRUMP", "trump"),
        ("QUANTUM", "quantum"),
        ("CLEAR_HEAD", "clear-head"),
        ("AMBERITE", "amberite"),
        ("NORSE", "norse"),
        ("HINDU", "hindu"),
        ("EGYPTIAN", "egyptian"),
        ("OLYMPIAN", "olympian"),
        ("NO_SUMMON", "no-summon"),
        ("KNIGHT", "knight"),
    ] {
        if entry.flags.iter().any(|candidate| candidate == flag) {
            tags.insert(tag.to_owned());
        }
    }
    if entry.flags.iter().any(|flag| flag == "KNIGHT")
        && entry.flags.iter().any(|flag| flag == "DUNGEON_2")
    {
        tags.insert("camelot-knight".to_owned());
    }
    value["tags"] = serde_json::json!(tags);
    if selection
        .tags
        .iter()
        .any(|tag| matches!(tag.as_str(), "guardian" | "questor"))
    {
        value["capturePolicy"] = serde_json::json!("immune");
    }

    let status_immunities = [
        ("NO_CONF", "rfb.status.confusion"),
        ("NO_FEAR", "rfb.status.fear"),
        ("NO_SLEEP", "rfb.status.sleep"),
        ("NO_STUN", "rfb.status.stun"),
    ]
    .into_iter()
    .filter_map(|(flag, status)| {
        entry
            .flags
            .iter()
            .any(|candidate| candidate == flag)
            .then_some(status)
    })
    .collect::<Vec<_>>();
    if status_immunities.is_empty() {
        value
            .as_object_mut()
            .expect("actor JSON must be an object")
            .remove("statusImmunities");
    } else {
        value["statusImmunities"] = serde_json::json!(status_immunities);
    }

    let has_corpse = entry.flags.iter().any(|flag| flag == "DROP_CORPSE");
    let has_skeleton = entry.flags.iter().any(|flag| flag == "DROP_SKELETON");
    if has_corpse || has_skeleton {
        let mut remains = serde_json::json!({
            "chanceDenominator": 3,
            "corpseWeight": if has_corpse && has_skeleton { 4 } else if has_corpse { 1 } else { 0 },
            "skeletonWeight": if has_skeleton { 1 } else { 0 },
        });
        if has_corpse {
            remains["corpseItemKindId"] = serde_json::json!(DEMO_CORPSE_ITEM_ID);
        }
        if has_skeleton {
            remains["skeletonItemKindId"] = serde_json::json!(DEMO_SKELETON_ITEM_ID);
        }
        value["remains"] = remains;
    }
    if let Some(death_drop) = value.get_mut("deathDrop") {
        if death_drop.get("itemTableId").is_some() {
            death_drop["itemTableId"] = serde_json::json!(if entry.index == 1185 {
                "demo.loot-table.base-items"
            } else {
                DEMO_DROP_TABLE_ID
            });
        }
        if entry.index == 1185 {
            death_drop["themeTableId"] = serde_json::json!(DEMO_WARRIOR_DROP_TABLE_ID);
            death_drop["themeChancePercent"] = serde_json::json!(50);
        } else if let Some(theme_table_id) = entry
            .drop_theme
            .as_deref()
            .and_then(demo_drop_theme_table_id)
        {
            death_drop["themeTableId"] = serde_json::json!(theme_table_id);
            death_drop["themeChancePercent"] = serde_json::json!(50);
        }
        if death_drop["countDice"]
            .as_array()
            .is_some_and(Vec::is_empty)
        {
            death_drop
                .as_object_mut()
                .expect("death drop JSON must be an object")
                .remove("countDice");
        }
        if death_drop["minimumQuality"] == "ordinary" {
            death_drop
                .as_object_mut()
                .expect("death drop JSON must be an object")
                .remove("minimumQuality");
        }
    }
    if value["hitPointDice"]["forceMaximum"] == false {
        value["hitPointDice"]
            .as_object_mut()
            .expect("hit point dice JSON must be an object")
            .remove("forceMaximum");
    }
    if value["movement"]["modes"]
        .as_array()
        .is_some_and(Vec::is_empty)
    {
        value["movement"]
            .as_object_mut()
            .expect("movement JSON must be an object")
            .remove("modes");
    }
    for (object, key) in [
        ("doorInteraction", "opens"),
        ("doorInteraction", "bashes"),
        ("terrainInteraction", "destroysWalls"),
        ("terrainInteraction", "destroysItems"),
        ("terrainInteraction", "picksUpItems"),
        ("allocation", "forceDepth"),
        ("allocation", "wildOnly"),
        ("allocation", "escort"),
        ("allocation", "multiplies"),
    ] {
        if value[object][key] == false {
            value[object]
                .as_object_mut()
                .expect("nested actor JSON must be an object")
                .remove(key);
        }
    }
    if value["allocation"]["randomMovementPercent"] == 0 {
        value["allocation"]
            .as_object_mut()
            .expect("allocation JSON must be an object")
            .remove("randomMovementPercent");
    }
    Ok(value)
}

pub struct ContentImportOutcome {
    pub report: ContentImportReport,
    pub terrain_files: Vec<(String, serde_json::Value)>,
    pub actor_files: Vec<(String, serde_json::Value)>,
    pub ability_files: Vec<(String, serde_json::Value)>,
    pub ability_book_files: Vec<(String, serde_json::Value)>,
    pub resource_files: Vec<(String, serde_json::Value)>,
    pub item_files: Vec<(String, serde_json::Value)>,
    pub loot_table_files: Vec<(String, serde_json::Value)>,
    pub affix_files: Vec<(String, serde_json::Value)>,
    pub race_files: Vec<(String, serde_json::Value)>,
    pub class_files: Vec<(String, serde_json::Value)>,
    pub personality_files: Vec<(String, serde_json::Value)>,
    /// Normalized `m_info` diagnostics. Runtime-ready subsets are emitted
    /// through regular class, ability, and ability-book definitions instead.
    pub magic_profile_files: Vec<(String, serde_json::Value)>,
    pub realm_readability: Option<serde_json::Value>,
    pub class_casting_shells: Option<serde_json::Value>,
    pub skill_files: Vec<(String, serde_json::Value)>,
    pub skill_set_files: Vec<(String, serde_json::Value)>,
}

const LEGACY_RESOURCE_ID: &str = "rfb-legacy.resource.essence";
const LEGACY_MANA_RESOURCE_ID: &str = "rfb-legacy.resource.mana";
const DEATH_REALM_INDEX: u8 = 4;
const DEATH_BOOK_TVAL: u16 = 94;
const DEATH_FIRST_BOOK_SVAL: u16 = 0;
const DEATH_SECOND_BOOK_SVAL: u16 = 1;
const DEATH_THIRD_BOOK_SVAL: u16 = 2;
const DEATH_FOURTH_BOOK_SVAL: u16 = 3;
const DEATH_FIRST_BOOK_ID: &str = "rfb-legacy.ability-book.death-black-prayers";
const DEATH_SECOND_BOOK_ID: &str = "rfb-legacy.ability-book.death-black-mass";
const DEATH_THIRD_BOOK_ID: &str = "rfb-legacy.ability-book.death-black-channels";
const DEATH_FOURTH_BOOK_ID: &str = "rfb-legacy.ability-book.death-necronomicon";
const SORCERY_BOOK_TVAL: u16 = 91;
const SORCERY_FIRST_BOOK_SVAL: u16 = 0;
const SORCERY_FIRST_BOOK_ID: &str = "rfb-legacy.ability-book.sorcery-beginners-handbook";
const SORCERY_SECOND_BOOK_SVAL: u16 = 1;
const SORCERY_SECOND_BOOK_ID: &str = "rfb-legacy.ability-book.sorcery-master-sorcerers-handbook";
const SORCERY_THIRD_BOOK_SVAL: u16 = 2;
const SORCERY_THIRD_BOOK_ID: &str = "rfb-legacy.ability-book.sorcery-pattern-sorcery";
const SORCERY_FOURTH_BOOK_SVAL: u16 = 3;
const SORCERY_FOURTH_BOOK_ID: &str = "rfb-legacy.ability-book.sorcery-grimoire-of-power";
const ARCANE_BOOK_TVAL: u16 = 96;
const ARCANE_FIRST_BOOK_SVAL: u16 = 0;
const ARCANE_FIRST_BOOK_ID: &str = "rfb-legacy.ability-book.arcane-cantrips-for-beginners";
const ARCANE_SECOND_BOOK_SVAL: u16 = 1;
const ARCANE_SECOND_BOOK_ID: &str = "rfb-legacy.ability-book.arcane-minor-arcana";
const ARCANE_THIRD_BOOK_SVAL: u16 = 2;
const ARCANE_THIRD_BOOK_ID: &str = "rfb-legacy.ability-book.arcane-major-arcana";
const ARCANE_FOURTH_BOOK_SVAL: u16 = 3;
const ARCANE_FOURTH_BOOK_ID: &str = "rfb-legacy.ability-book.arcane-manual-of-mastery";
const ARMAGEDDON_BOOK_TVAL: u16 = 101;
const ARMAGEDDON_FIRST_BOOK_SVAL: u16 = 0;
const ARMAGEDDON_FIRST_BOOK_ID: &str = "rfb-legacy.ability-book.armageddon-book-of-elements";
const ARMAGEDDON_SECOND_BOOK_SVAL: u16 = 1;
const ARMAGEDDON_SECOND_BOOK_ID: &str = "rfb-legacy.ability-book.armageddon-earth-wind-and-fire";
const ARMAGEDDON_THIRD_BOOK_SVAL: u16 = 2;
const ARMAGEDDON_THIRD_BOOK_ID: &str = "rfb-legacy.ability-book.armageddon-path-of-destruction";
const ARMAGEDDON_FOURTH_BOOK_SVAL: u16 = 3;
const ARMAGEDDON_FOURTH_BOOK_ID: &str = "rfb-legacy.ability-book.armageddon-day-of-ragnarok";
const LEGACY_VAMPIRE_LORD_RACE_ID: &str = "rfb-legacy.race.vampire-lord-form";
const LEGACY_VAMPIRE_LORD_SKILL_SET_ID: &str = "rfb-legacy.skill-set.race-vampire-lord-form";
const LEGACY_SLAYING_WEAPON_AFFIX_ID: &str = "rfb-legacy.affix.slaying";
const LEGACY_DEATH_WEAPON_AFFIX_ID: &str = "rfb-legacy.affix.death";
const LEGACY_CORPSE_ITEM_ID: &str = "rfb-legacy.item.corpse-remains";

fn status_ability(id: &str, status: &str, self_target: bool) -> serde_json::Value {
    let target = if self_target {
        serde_json::json!({ "modes": ["self"], "range": 0, "requiresLineOfEffect": false })
    } else {
        serde_json::json!({ "modes": ["position", "entity"], "range": 6, "requiresLineOfEffect": true })
    };
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{id}"),
        "nameKey": format!("ability-legacy-{id}-name"),
        "descriptionKey": format!("ability-legacy-{id}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": target,
        "effect": {
            "type": "apply-status",
            "statusKindId": format!("rfb.status.{status}"),
            "intensity": 1,
            "durationTicks": 25,
            "stacking": "extend",
        },
        "tags": ["legacy-import", "status"],
    })
}

fn displacement_ability(
    id: &str,
    effect: serde_json::Value,
    self_target: bool,
) -> serde_json::Value {
    let target = if self_target {
        serde_json::json!({ "modes": ["self"], "range": 0, "requiresLineOfEffect": false })
    } else {
        serde_json::json!({ "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true })
    };
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{id}"),
        "nameKey": format!("ability-legacy-{id}-name"),
        "descriptionKey": format!("ability-legacy-{id}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": target,
        "effect": effect,
        "tags": ["legacy-import", "mobility"],
    })
}

fn heal_ability(amount: u32) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.heal-{amount}"),
        "nameKey": format!("ability-legacy-heal-{amount}-name"),
        "descriptionKey": format!("ability-legacy-heal-{amount}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
        "effect": { "type": "heal", "amount": amount },
        "tags": ["legacy-import", "heal"],
    })
}

fn demo_traps_ability() -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": "rfb-legacy.ability.traps",
        "nameKey": "ability-legacy-traps-name",
        "descriptionKey": "ability-legacy-traps-description",
        "target": {
            "modes": ["position"],
            "range": 6,
            "requiresLineOfEffect": true
        },
        "effect": {
            "type": "transform-terrain",
            "sourceTerrainIds": ["demo.terrain.floor"],
            "targetTerrainId": "demo.terrain.warren-snare",
            "radius": 1
        },
        "tags": ["legacy-import", "utility"]
    })
}

/// Spells parsed from r_info that the legacy engine restricts to the
/// possessor/mimic player: monsters never cast them, so they are recorded as
/// not-applicable instead of unmapped gaps.
const POSSESSOR_ONLY_SPELLS: [&str; 11] = [
    "DETECT_TRAPS",
    "DETECT_EVIL",
    "DETECT_MONSTERS",
    "DETECT_OBJECTS",
    "IDENTIFY",
    "MAPPING",
    "CLAIRVOYANCE",
    "MULTIPLY",
    "BLESS",
    "HEROISM",
    "BERSERK",
];

/// Maps one legacy spell token to a generated ability id, registering the
/// shared ability definition on first use.
fn map_spell_token(
    token: &str,
    level: u16,
    breath_radius: u8,
    caster_kind_id: &str,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    if token == "NO_AIR" {
        let id = "rfb-legacy.ability.no-air-40".to_owned();
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": "ability-legacy-no-air-40-name",
                "descriptionKey": "ability-legacy-no-air-40-description",
                "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
                "effect": {
                    "type": "apply-status",
                    "statusKindId": "rfb.status.no-air",
                    "intensity": 1,
                    "durationTicks": 40,
                    "stacking": "keep-strongest"
                },
                "tags": ["legacy-import", "monster-only", "monster-no-air"],
            })
        });
        return Some(id);
    }
    if token == "WORLD" {
        let id = "rfb-legacy.ability.world".to_owned();
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": "ability-legacy-world-name",
                "descriptionKey": "ability-legacy-world-description",
                "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
                "effect": { "type": "no-op", "reason": "monster-world" },
                "tags": ["legacy-import", "monster-only", "monster-world"],
            })
        });
        return Some(id);
    }
    if token == "SPECIAL"
        && matches!(
            caster_kind_id.rsplit('.').next(),
            Some("banor-rupart" | "banor-the-prince-regent" | "rupart-the-general")
        )
    {
        let id = "rfb-legacy.ability.banor-rupart-transform".to_owned();
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": "ability-legacy-banor-rupart-transform-name",
                "descriptionKey": "ability-legacy-banor-rupart-transform-description",
                "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
                "effect": { "type": "no-op", "reason": "banor-rupart-transform" },
                "tags": ["legacy-import", "monster-only", "monster-banor-rupart-transform"],
            })
        });
        return Some(id);
    }
    if token == "BIRD_DROP" {
        let id = "rfb-legacy.ability.bird-drop".to_owned();
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": "ability-legacy-bird-drop-name",
                "descriptionKey": "ability-legacy-bird-drop-description",
                "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
                "effect": { "type": "bird-drop" },
                "tags": ["legacy-import", "monster-only"],
            })
        });
        return Some(id);
    }
    if token == "GAZE" {
        let id = "rfb-legacy.ability.gaze".to_owned();
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": "ability-legacy-gaze-name",
                "descriptionKey": "ability-legacy-gaze-description",
                "minimumLevel": 1,
                "resourceId": LEGACY_RESOURCE_ID,
                "resourceCost": 1,
                "baseFailurePercent": 20,
                "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
                "effect": {
                    "type": "damage",
                    "damageDice": 1,
                    "damageSides": 1,
                    "damageType": "physical"
                },
                "tags": ["legacy-import", "gaze"],
            })
        });
        return Some(id);
    }
    if let Some(id) = map_summon_spell_token(token, level, caster_kind_id, abilities) {
        return Some(id);
    }
    if let Some(id) = map_mental_spell_token(token, level, abilities) {
        return Some(id);
    }
    if let Some(id) = map_curse_spell_token(token, abilities) {
        return Some(id);
    }
    if let Some(id) = map_jump_spell_token(token, level, abilities) {
        return Some(id);
    }
    if let Some(id) = map_misc_spell_token(token, level, abilities) {
        return Some(id);
    }
    if let Some(amount) = token
        .strip_prefix("HEAL(")
        .and_then(|amount| amount.strip_suffix(')'))
        .and_then(|amount| amount.parse::<u32>().ok())
        .filter(|amount| (1..=1_000_000).contains(amount))
    {
        let id = format!("rfb-legacy.ability.heal-{amount}");
        abilities
            .entry(id.clone())
            .or_insert_with(|| heal_ability(amount));
        return Some(id);
    }
    match token {
        "SCARE" => {
            let id = "rfb-legacy.ability.scare".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("scare", "fear", false));
            Some(id)
        }
        "CONFUSE" => {
            let id = "rfb-legacy.ability.confuse".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("confuse", "confusion", false));
            Some(id)
        }
        "BLIND" => {
            let id = "rfb-legacy.ability.blind".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("blind", "blindness", false));
            Some(id)
        }
        "PARALYZE" => {
            let id = "rfb-legacy.ability.paralyze".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("paralyze", "paralysis", false));
            Some(id)
        }
        "SLOW" => {
            let id = "rfb-legacy.ability.slow".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("slow", "slow", false));
            Some(id)
        }
        "HASTE" => {
            let id = "rfb-legacy.ability.haste-self".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("haste-self", "haste", true));
            Some(id)
        }
        "INVULN" => {
            let id = "rfb-legacy.ability.invulnerability-self".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability = status_ability("invulnerability-self", "invulnerability", true);
                ability["effect"]["durationTicks"] = serde_json::json!(4);
                ability["effect"]["durationDice"] = serde_json::json!(1);
                ability["effect"]["durationSides"] = serde_json::json!(4);
                ability["effect"]["incomingDamagePercent"] = serde_json::json!(0);
                ability
            });
            Some(id)
        }
        "BLINK" => {
            let id = "rfb-legacy.ability.blink".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                displacement_ability(
                    "blink",
                    serde_json::json!({"type": "blink-self", "radius": 10}),
                    true,
                )
            });
            Some(id)
        }
        "BLINK_OTHER" => {
            let id = "rfb-legacy.ability.blink-other".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                displacement_ability(
                    "blink-other",
                    serde_json::json!({"type": "blink-target", "radius": 10}),
                    false,
                )
            });
            Some(id)
        }
        "TELE_SELF" | "TELEPORT" => {
            let id = "rfb-legacy.ability.escape".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                displacement_ability(
                    "escape",
                    serde_json::json!({"type": "teleport-self", "minimumDistance": 10}),
                    true,
                )
            });
            Some(id)
        }
        "TELE_TO" => {
            let id = "rfb-legacy.ability.drag".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                displacement_ability(
                    "drag",
                    serde_json::json!({"type": "teleport-target"}),
                    false,
                )
            });
            Some(id)
        }
        "TELE_LEVEL" => {
            let id = "rfb-legacy.ability.teleport-level".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                displacement_ability(
                    "teleport-level",
                    serde_json::json!({"type": "teleport-level"}),
                    false,
                )
            });
            Some(id)
        }
        "HEAL" => {
            let amount = u32::from(level).saturating_mul(3).clamp(5, 300);
            let id = format!("rfb-legacy.ability.heal-{amount}");
            abilities
                .entry(id.clone())
                .or_insert_with(|| heal_ability(amount));
            Some(id)
        }
        "SHRIEK" => {
            let id = "rfb-legacy.ability.shriek".to_owned();
            abilities.entry(id.clone()).or_insert_with(|| {
                serde_json::json!({
                    "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                    "formatVersion": 1,
                    "id": id,
                    "nameKey": "ability-legacy-shriek-name",
                    "descriptionKey": "ability-legacy-shriek-description",
                    "target": {
                        "modes": ["self"],
                        "range": 0,
                        "requiresLineOfEffect": false
                    },
                    "effect": { "type": "aggravate-monsters" },
                    "tags": ["legacy-import", "innate", "utility"]
                })
            });
            Some(id)
        }
        other => map_damage_spell_token(other, level, breath_radius, abilities),
    }
}

/// Direct-damage shapes harvested from legacy S: lines.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DamageSpellShape {
    Bolt,
    Ball,
    Beam,
    /// Legacy MSF_BALL4 storms explode with radius four.
    BigBall,
}

impl DamageSpellShape {
    const fn keyword(self) -> &'static str {
        match self {
            Self::Bolt => "bolt",
            Self::Ball | Self::BigBall => "ball",
            Self::Beam => "beam",
        }
    }

    const fn ball_radius(self) -> u8 {
        match self {
            Self::Bolt => 0,
            Self::Ball => 2,
            Self::BigBall => 4,
            Self::Beam => 0,
        }
    }
}

/// Looks up a bolt/ball token base: shape, damage type and the legacy default
/// dice `(dice, sides, bonus)` derived from the monster level. Elements with
/// no faithful damage-type mapping stay unmapped for a future damage-type
/// expansion instead of being folded into physical.
fn damage_spell_defaults(
    base: &str,
    level: u32,
) -> Option<(DamageSpellShape, &'static str, (u32, u32, u32))> {
    use DamageSpellShape::{Ball, Beam, BigBall, Bolt};
    let entry = match base {
        "BO_ACID" => (Bolt, "acid", (7, 8, level / 3)),
        "BO_ELEC" => (Bolt, "electricity", (4, 8, level / 3)),
        "BO_FIRE" => (Bolt, "fire", (9, 8, level / 3)),
        "BO_COLD" => (Bolt, "cold", (6, 8, level / 3)),
        "BO_ICE" => (Bolt, "ice", (6, 8, level)),
        "BO_PLASMA" => (Bolt, "plasma", (8, 7, 10 + level)),
        "BO_WATER" => (Bolt, "water", (10, 10, level)),
        "BO_MANA" => (Bolt, "mana", (1, 7 * level / 2, 50)),
        "BO_NETHER" => (Bolt, "nether", (5, 5, 30 + level)),
        "BO_TIME" => (Bolt, "time", (2, level, level / 3)),
        "MISSILE" => (Bolt, "physical", (2, 6, level / 3)),
        "SHOOT" => (
            Bolt,
            "physical",
            ((4 + level / 24).min(6), (level / 4).max(2), 0),
        ),
        // Legacy THROW is a radius-zero ball; a single-target bolt is the
        // faithful neutral shape. Its flat damage uses the 1d1+(F-1) identity.
        "THROW" => (Bolt, "physical", (1, 1, (3 * level).saturating_sub(1))),
        "HELL_LANCE" => (Beam, "hell-fire", (1, 1, (2 * level).saturating_sub(1))),
        "HOLY_LANCE" => (Beam, "holy-fire", (1, 1, (2 * level).saturating_sub(1))),
        "BA_ACID" => (Ball, "acid", (1, 3 * level, 15)),
        "BA_ELEC" => (Ball, "electricity", (1, 3 * level / 2, 8)),
        "BA_FIRE" => (Ball, "fire", (1, 7 * level / 2, 10)),
        "BA_COLD" => (Ball, "cold", (1, 3 * level / 2, 10)),
        "BA_POIS" | "BA_POISON" => (Ball, "poison", (12, 2, 0)),
        "BA_NUKE" => (Ball, "nuke", (10, 6, level)),
        "BA_WATER" => (Ball, "water", (1, level, 50)),
        // Rockets are shard bursts in the legacy resistance table.
        "ROCKET" => (Ball, "shards", (1, 1, (6 * level).saturating_sub(1))),
        "PULVERISE" => (Ball, "physical", (8, 8, 0)),
        "BA_NETHER" => (Ball, "nether", (10, 10, 50 + level)),
        "BA_NEXUS" => (Ball, "nexus", (10, 10, level)),
        "BA_CHAOS" => (BigBall, "chaos", (10, 10, level)),
        "BA_DARK" => (BigBall, "dark", (10, 10, 50 + 4 * level)),
        "BA_LITE" => (BigBall, "light", (10, 10, 50 + 4 * level)),
        "MANA_STORM" => (BigBall, "mana", (10, 10, 50 + 4 * level)),
        _ => return None,
    };
    Some(entry)
}

/// Parses an explicit r_info dice override: `XdY+Z`, `XdY` or a flat `N`
/// (encoded through the exact 1d1+(N-1) identity).
fn parse_explicit_damage_dice(spec: &str) -> Option<(u32, u32, u32)> {
    if let Some((dice_part, sides_part)) = spec.split_once('d') {
        let dice = dice_part.parse::<u32>().ok()?;
        let (sides, bonus) = match sides_part.split_once('+') {
            Some((sides_part, bonus_part)) => (
                sides_part.parse::<u32>().ok()?,
                bonus_part.parse::<u32>().ok()?,
            ),
            None => (sides_part.parse::<u32>().ok()?, 0),
        };
        return Some((dice, sides, bonus));
    }
    let flat = spec.parse::<u32>().ok()?;
    Some((1, 1, flat.saturating_sub(1)))
}

/// Looks up a summon token base: candidate category tag plus the legacy
/// default count dice `(dice, sides, bonus)`. Categories map through the
/// type-flag tags stamped on imported actors; glyph-class and unique summon
/// types stay unmapped until their candidate rule is explicitly supported.
fn summon_spell_defaults(base: &str) -> Option<(&'static str, (u32, u32, u32))> {
    let entry = match base {
        "S_MONSTER" => ("legacy-import", (1, 3, 1)),
        "S_UNDEAD" => ("undead", (1, 3, 1)),
        "S_HI_UNDEAD" => ("undead", (1, 3, 0)),
        "S_DEMON" => ("demon", (1, 3, 1)),
        "S_HI_DEMON" => ("demon", (1, 3, 0)),
        "S_DRAGON" => ("dragon", (1, 3, 1)),
        "S_HI_DRAGON" => ("dragon", (1, 3, 0)),
        "S_ANIMAL" => ("animal", (1, 3, 1)),
        "S_ANT" => ("ant", (1, 3, 1)),
        "S_SPIDER" => ("spider", (1, 3, 1)),
        "S_HOUND" => ("hound", (1, 2, 1)),
        "S_HYDRA" => ("hydra", (1, 3, 1)),
        "S_ANGEL" => ("angel", (1, 3, 1)),
        "S_EAGLE" => ("eagle", (1, 3, 1)),
        "S_LOUSE" => ("louse", (1, 3, 1)),
        "S_NIGHTMARE" => ("night-mare", (1, 3, 1)),
        "S_AMBERITE" => ("amberite", (1, 2, 0)),
        "S_SERPENT" => ("kin-glyph-74", (1, 3, 1)),
        "S_NAGA" => ("kin-glyph-110", (1, 3, 1)),
        "S_VANARA" => ("vanara", (1, 3, 1)),
        "S_CYBER" => ("cyber", (1, 3, 0)),
        "S_CAT" => ("cat", (1, 3, 1)),
        "S_UNIQUE" => ("unique", (1, 2, 0)),
        "S_GUARDIAN" => ("guardian", (1, 2, 0)),
        "S_CAMELOT" => ("camelot-knight", (1, 2, 0)),
        "S_KNIGHT" => ("knight", (1, 2, 0)),
        _ => return None,
    };
    Some(entry)
}

/// Imported summons approximate the legacy permanent lifetime with the
/// maximum expressible duration.
const LEGACY_SUMMON_DURATION_TURNS: u32 = 10_000;

fn map_summon_spell_token(
    token: &str,
    level: u16,
    caster_kind_id: &str,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let (base, explicit) = match token.split_once('(') {
        Some((base, rest)) => (base, Some(rest.strip_suffix(')')?)),
        None => (token, None),
    };
    if base == "S_SOFTWARE_BUG" {
        let suffix = "summon-software-bug-l14-1d3-1";
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            let mut ability = summon_category_ability(suffix, "bug", 14, 1, 3, 1, None);
            ability["effect"]["batchCandidates"] = serde_json::json!([{
                "actorKindId": "demo.actor.software-bug",
                "weight": 1,
            }]);
            ability
        });
        return Some(id);
    }
    if base == "S_DEAD_UNIQ" {
        let maximum_level = u32::from(level.max(1));
        let suffix = format!("summon-dead-unique-l{maximum_level}-1d2");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            let mut ability =
                summon_category_ability(&suffix, "unique", maximum_level, 1, 2, 0, None);
            ability["tags"] =
                serde_json::json!(["legacy-import", "summon", "monster-dead-unique-summon",]);
            ability
        });
        return Some(id);
    }
    if base == "S_PANTHEON" {
        let category = match caster_kind_id.rsplit('.').next()? {
            "heimdall-guardian-of-bifrost"
            | "frigg-queen-of-asgard"
            | "freyja-lady-of-the-slain"
            | "odin-the-all-father" => "norse",
            "indra-the-heavenly-king-of-meru" => "hindu",
            "aphrodite-the-goddess-of-love"
            | "hermes-the-messenger-god"
            | "zeus-king-of-the-olympians" => "olympian",
            "amun-the-mysterious" | "hathor-the-heavenly-cow" => "egyptian",
            _ => return None,
        };
        let suffix = format!("summon-{category}-l{level}-1d2");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            summon_category_ability(&suffix, category, u32::from(level), 1, 2, 0, None)
        });
        return Some(id);
    }
    if base == "S_SPECIAL" {
        let caster_tail = caster_kind_id.rsplit('.').next()?;
        if matches!(
            caster_tail,
            "athena-the-goddess-of-wisdom"
                | "ares-the-god-of-war"
                | "apollo-the-sun-god"
                | "artemis-the-moon-goddess"
                | "hephaestus-the-smith-god"
                | "hades-ruler-of-the-underworld"
                | "hera-queen-of-the-gods"
                | "osiris-the-reborn"
                | "lakshmi-the-goddess-of-prosperity"
                | "vishnu-the-preserver"
                | "shiva-the-destroyer"
                | "parvati-the-goddess-of-hidden-power"
        ) {
            let suffix = format!("summon-family-{caster_tail}");
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                serde_json::json!({
                    "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                    "formatVersion": 1,
                    "id": id,
                    "nameKey": format!("ability-legacy-{suffix}-name"),
                    "descriptionKey": format!("ability-legacy-{suffix}-description"),
                    "minimumLevel": 1,
                    "resourceId": LEGACY_RESOURCE_ID,
                    "resourceCost": 1,
                    "baseFailurePercent": 20,
                    "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
                    "effect": { "type": "no-op", "reason": "monster-family-summon" },
                    "tags": ["legacy-import", "summon", "monster-only", "monster-family-summon"],
                })
            });
            return Some(id);
        }
        if caster_kind_id.rsplit('.').next()? == "aegir-god-king-of-the-sea-giants" {
            let suffix = "summon-aegir-retinue-1d4";
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability = summon_category_ability(suffix, "ocean", 77, 1, 4, 0, None);
                ability["effect"]["batchCandidates"] = serde_json::json!([
                    {
                        "actorKindId": "demo.actor.sea-giant",
                        "weight": 1,
                    },
                    {
                        "actorKindId": "demo.actor.lesser-kraken",
                        "weight": 1,
                    },
                ]);
                ability["tags"] = serde_json::json!([
                    "legacy-import",
                    "summon",
                    "monster-only",
                    "monster-water-flow",
                ]);
                ability
            });
            return Some(id);
        }
        if caster_kind_id.rsplit('.').next()? == "gragomani-the-leprechaun-prophet" {
            let suffix = "summon-gragomani-followers-1d4-4";
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability =
                    summon_category_ability(suffix, "kin-glyph-104", 61, 1, 4, 4, None);
                ability["effect"]["batchCandidates"] = serde_json::json!([
                    {
                        "actorKindId": "demo.actor.malicious-leprechaun",
                        "weight": 1,
                    },
                    {
                        "actorKindId": "demo.actor.leprechaun-fanatic",
                        "weight": 3,
                    },
                ]);
                ability
            });
            return Some(id);
        }
        if caster_kind_id.rsplit('.').next()? == "odin-the-all-father" {
            let suffix = "summon-odin-retinue-1d4-max1";
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                let mut ability =
                    summon_category_ability(suffix, "kin-glyph-112", 65, 1, 4, 0, Some(1));
                ability["effect"]["batchCandidates"] = serde_json::json!([
                    {
                        "actorKindId": "demo.actor.einheri-berserker",
                        "weight": 1,
                    },
                    {
                        "actorKindId": "demo.actor.valkyrie",
                        "weight": 1,
                    },
                ]);
                ability
            });
            return Some(id);
        }
        if caster_tail == "gertrude" {
            let suffix = "summon-gertrude-sisters-l40-1d1-1";
            let id = format!("rfb-legacy.ability.{suffix}");
            abilities.entry(id.clone()).or_insert_with(|| {
                summon_category_ability(suffix, "witch-sister", 40, 1, 1, 1, Some(2))
            });
            return Some(id);
        }
        let (
            suffix,
            category,
            maximum_level,
            dice,
            sides,
            bonus,
            maximum_count,
            batch_candidate,
            water_flow,
        ) = match caster_kind_id.rsplit('.').next()? {
            "zoopi-the-cube-king" => (
                "summon-gelatinous-cube-l16-1d3",
                "gelatinous-cube",
                16,
                1,
                3,
                0,
                None,
                None,
                false,
            ),
            "rolento" => (
                "summon-hand-grenade-l38-1d3-1",
                "hand-grenade",
                38,
                1,
                3,
                1,
                None,
                None,
                false,
            ),
            "santa-claus" => (
                "summon-reindeer-l52-1d4",
                "reindeer",
                52,
                1,
                4,
                0,
                None,
                None,
                false,
            ),
            "jack-of-lanterns" => (
                "summon-death-pumpkin-l52-1d4",
                "death-pumpkin",
                52,
                1,
                4,
                0,
                None,
                None,
                false,
            ),
            "bull-gates" => (
                "summon-internet-exploder-l52-1d4",
                "internet-exploder",
                52,
                1,
                4,
                0,
                None,
                None,
                false,
            ),
            "the-gospel-of-mug" => (
                "summon-tracking-pixel-l56-1d4-max3",
                "tracking-pixel",
                56,
                1,
                4,
                0,
                Some(3),
                None,
                false,
            ),
            "the-nightmare-dragon" => (
                "summon-night-mare-l39-1d3-2",
                "night-mare",
                39,
                1,
                3,
                2,
                None,
                None,
                false,
            ),
            "caldarm-the-third" => (
                "summon-clone-of-locke-l65-1d3",
                "clone-of-locke",
                65,
                1,
                3,
                0,
                None,
                None,
                false,
            ),
            "zeus-king-of-the-olympians" => (
                "summon-shambler-l67-1d4",
                "kin-glyph-69",
                67,
                1,
                4,
                0,
                None,
                Some("demo.actor.shambler"),
                false,
            ),
            "hermes-the-messenger-god" => (
                "summon-magic-mushroom-patch-l15-1d16",
                "kin-glyph-44",
                15,
                1,
                16,
                0,
                None,
                Some("demo.actor.magic-mushroom-patch"),
                false,
            ),
            "varuna-lord-of-water" => (
                "summon-makara-l50-1d2-2",
                "mount-meru",
                50,
                1,
                2,
                2,
                None,
                Some("demo.actor.makara"),
                true,
            ),
            "demeter-the-goddess-of-nature" => (
                "summon-ent-l46-1d4",
                "giant",
                46,
                1,
                4,
                0,
                None,
                Some("demo.actor.ent"),
                false,
            ),
            "justshorn-sorcerer-king-of-the-sheeple" => (
                "summon-sheep-l3-1d4",
                "sheep",
                3,
                1,
                4,
                0,
                None,
                Some("demo.actor.sheep"),
                false,
            ),
            "poseidon-lord-of-seas-and-storm" => (
                "summon-greater-kraken-l63-1d4",
                "ocean",
                63,
                1,
                4,
                0,
                None,
                Some("demo.actor.greater-kraken"),
                true,
            ),
            "talos-masterwork-spellwarp-automaton" => (
                "summon-spellwarp-automaton-l80-1d3",
                "nonliving",
                80,
                1,
                3,
                0,
                None,
                Some("demo.actor.spellwarp-automaton"),
                false,
            ),
            "brahma-the-creating-spirit" => (
                "summon-saraswati-l90-1d1",
                "hindu",
                90,
                1,
                1,
                0,
                None,
                Some("demo.actor.saraswati-goddess-of-knowledge"),
                false,
            ),
            "saraswati-goddess-of-knowledge" => (
                "summon-brahma-l92-1d1",
                "hindu",
                92,
                1,
                1,
                0,
                None,
                Some("demo.actor.brahma-the-creating-spirit"),
                false,
            ),
            _ => return None,
        };
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            let mut ability = summon_category_ability(
                suffix,
                category,
                maximum_level,
                dice,
                sides,
                bonus,
                maximum_count,
            );
            if let Some(actor_kind_id) = batch_candidate {
                ability["effect"]["batchCandidates"] = serde_json::json!([{
                    "actorKindId": actor_kind_id,
                    "weight": 1,
                }]);
            }
            if water_flow {
                ability["tags"] = serde_json::json!([
                    "legacy-import",
                    "summon",
                    "monster-only",
                    "monster-water-flow",
                ]);
            }
            ability
        });
        return Some(id);
    }
    if base == "S_KIN" {
        let caster_tail = caster_kind_id.rsplit('.').next()?;
        let suffix = format!("kin-{caster_tail}");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            if caster_tail == "othrod-lord-of-the-orcs" {
                summon_category_ability(&suffix, "kin-glyph-111", u32::from(level), 1, 1, 1, None)
            } else if caster_tail == "bast-goddess-of-cats" {
                summon_category_ability(&suffix, "kin-glyph-102", u32::from(level), 1, 1, 1, None)
            } else if caster_tail == "dio-brando" {
                summon_category_ability(&suffix, "kin-glyph-86", u32::from(level), 1, 1, 1, None)
            } else {
                summon_kin_ability(&suffix, caster_kind_id)
            }
        });
        return Some(id);
    }
    let (category, default_dice) = summon_spell_defaults(base)?;
    let (dice, sides, bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => default_dice,
    };
    let dice = dice.clamp(1, 8);
    let sides = sides.clamp(1, 8);
    let bonus = bonus.min(8);
    if dice * sides + bonus > 8 {
        return None;
    }
    let maximum_level = if base == "S_GUARDIAN" {
        u32::from(level.clamp(1, 99))
    } else {
        u32::from(level.max(1))
    };
    let mut suffix = format!("summon-{category}-l{maximum_level}-{dice}d{sides}");
    if bonus > 0 {
        suffix.push_str(&format!("-{bonus}"));
    }
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities.entry(id.clone()).or_insert_with(|| {
        let mut ability =
            summon_category_ability(&suffix, category, maximum_level, dice, sides, bonus, None);
        if base == "S_KNIGHT" {
            ability["effect"]["batchCandidates"] = serde_json::json!([
                { "actorKindId": "demo.actor.novice-paladin", "weight": 1 },
                { "actorKindId": "demo.actor.paladin", "weight": 1 },
                { "actorKindId": "demo.actor.white-knight", "weight": 1 },
                { "actorKindId": "demo.actor.ultra-elite-paladin", "weight": 1 },
                { "actorKindId": "demo.actor.knight-templar", "weight": 1 },
            ]);
        }
        ability
    });
    Some(id)
}

fn summon_kin_ability(suffix: &str, caster_kind_id: &str) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
        "effect": {
            "type": "summon",
            "actorKindId": caster_kind_id,
            "count": 2,
            "radius": 2,
            "durationTurns": LEGACY_SUMMON_DURATION_TURNS,
        },
        "tags": ["legacy-import", "summon"],
    })
}

fn summon_category_ability(
    suffix: &str,
    category: &str,
    maximum_level: u32,
    dice: u32,
    sides: u32,
    bonus: u32,
    maximum_count: Option<u32>,
) -> serde_json::Value {
    let mut effect = serde_json::json!({
        "type": "summon-category",
        "category": category,
        "maximumLevel": maximum_level,
        "countDice": dice,
        "countSides": sides,
        "radius": 2,
        "durationTurns": LEGACY_SUMMON_DURATION_TURNS,
    });
    if bonus > 0 {
        effect["countBonus"] = serde_json::json!(bonus);
    }
    if let Some(maximum_count) = maximum_count {
        effect["maximumCount"] = serde_json::json!(maximum_count);
    }
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["self"], "range": 0, "requiresLineOfEffect": false },
        "effect": effect,
        "tags": ["legacy-import", "summon"],
    })
}

/// Looks up a breath token base: damage type plus the legacy default
/// `(hp_percent, max_damage)` pair. Exotic elements stay unmapped until the
/// damage-type expansion lands.
fn breath_spell_defaults(base: &str) -> Option<(&'static str, (u32, u32))> {
    let entry = match base {
        "BR_ACID" => ("acid", (20, 900)),
        "BR_ELEC" => ("electricity", (20, 900)),
        "BR_FIRE" => ("fire", (20, 900)),
        "BR_COLD" => ("cold", (20, 900)),
        "BR_POISON" | "BR_POIS" => ("poison", (17, 600)),
        "BR_NUKE" => ("nuke", (17, 600)),
        "BR_NETHER" => ("nether", (14, 550)),
        "BR_LITE" => ("light", (17, 400)),
        "BR_DARK" => ("dark", (17, 400)),
        "BR_CONFUSION" | "BR_CONF" => ("confusion", (17, 400)),
        "BR_SOUND" => ("sound", (17, 450)),
        "BR_CHAOS" => ("chaos", (17, 600)),
        "BR_DISENCHANT" => ("disenchant", (17, 500)),
        "BR_SHARDS" => ("shards", (17, 500)),
        "BR_NEXUS" => ("nexus", (33, 250)),
        "BR_STORM" => ("storm", (13, 250)),
        "BR_INERTIA" => ("inertia", (17, 250)),
        "BR_PLASMA" => ("plasma", (17, 250)),
        "BR_HELL_FIRE" => ("hell-fire", (17, 250)),
        "BR_HOLY_FIRE" => ("holy-fire", (17, 250)),
        "BR_GRAVITY" => ("gravity", (33, 200)),
        "BR_FORCE" => ("force", (33, 200)),
        "BR_MANA" => ("mana", (33, 250)),
        "BR_DISINTEGRATE" => ("disintegrate", (17, 150)),
        "BR_TIME" => ("time", (33, 150)),
        _ => return None,
    };
    Some(entry)
}

fn map_damage_spell_token(
    token: &str,
    level: u16,
    breath_radius: u8,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    let (base, explicit) = match token.split_once('(') {
        Some((base, rest)) => (base, Some(rest.strip_suffix(')')?)),
        None => (token, None),
    };
    if base == "BR_AIR" {
        let hp_percent = match explicit {
            Some(spec) => spec.strip_suffix('%')?.parse::<u32>().ok()?.clamp(1, 100),
            None => 17,
        };
        let suffix = format!("breath-air-{hp_percent}-250-r{breath_radius}");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            let mut ability =
                breath_spell_ability(&suffix, "force", hp_percent, 250, breath_radius);
            ability["tags"] =
                serde_json::json!(["breath", "damage", "legacy-import", "monster-air-breath",]);
            ability
        });
        return Some(id);
    }
    if base == "CHICKEN" {
        let (dice, sides, bonus) = explicit.and_then(parse_explicit_damage_dice).unwrap_or((
            1,
            1,
            (5 * u32::from(level) / 2).saturating_sub(1),
        ));
        let dice = dice.clamp(1, 100);
        let sides = sides.clamp(1, 10_000);
        let bonus = bonus.min(10_000);
        let mut suffix = format!("chicken-{dice}d{sides}");
        if bonus > 0 {
            suffix.push_str(&format!("-{bonus}"));
        }
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
                "formatVersion": 1,
                "id": id,
                "nameKey": format!("ability-legacy-{suffix}-name"),
                "descriptionKey": format!("ability-legacy-{suffix}-description"),
                "minimumLevel": 1,
                "resourceId": LEGACY_RESOURCE_ID,
                "resourceCost": 1,
                "baseFailurePercent": 20,
                "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
                "effect": {
                    "type": "damage",
                    "damageDice": dice,
                    "damageSides": sides,
                    "damageBonus": bonus,
                    "damageType": "physical"
                },
                "tags": ["legacy-import", "damage", "monster-chicken"],
            })
        });
        return Some(id);
    }
    if let Some((damage_type, (default_percent, max_damage))) = breath_spell_defaults(base) {
        // Explicit overrides use the legacy `BR_X(N%)` form and only replace
        // the percentage; the elemental cap stays.
        let hp_percent = match explicit {
            Some(spec) => spec.strip_suffix('%')?.parse::<u32>().ok()?.clamp(1, 100),
            None => default_percent,
        };
        let suffix = format!("breath-{damage_type}-{hp_percent}-{max_damage}-r{breath_radius}");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities.entry(id.clone()).or_insert_with(|| {
            breath_spell_ability(&suffix, damage_type, hp_percent, max_damage, breath_radius)
        });
        return Some(id);
    }
    let (shape, damage_type, default_dice) = damage_spell_defaults(base, u32::from(level))?;
    let (dice, sides, bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => default_dice,
    };
    let dice = dice.clamp(1, 100);
    let sides = sides.clamp(1, 10_000);
    let bonus = bonus.min(10_000);
    let element = damage_type;
    let mut suffix = format!("{}-{element}-{dice}d{sides}", shape.keyword());
    if bonus > 0 {
        suffix.push_str(&format!("-{bonus}"));
    }
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities
        .entry(id.clone())
        .or_insert_with(|| damage_spell_ability(shape, &suffix, damage_type, dice, sides, bonus));
    Some(id)
}

fn breath_spell_ability(
    suffix: &str,
    damage_type: &str,
    hp_percent: u32,
    max_damage: u32,
    radius: u8,
) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["direction"], "range": 8, "requiresLineOfEffect": true },
        "effect": {
            "type": "breath-damage",
            "hpPercent": hp_percent,
            "maxDamage": max_damage,
            "damageType": damage_type,
            "radius": radius,
        },
        "tags": ["breath", "damage", "legacy-import"],
    })
}

fn damage_spell_ability(
    shape: DamageSpellShape,
    suffix: &str,
    damage_type: &str,
    dice: u32,
    sides: u32,
    bonus: u32,
) -> serde_json::Value {
    let mut effect = serde_json::json!({
        "type": match shape {
            DamageSpellShape::Bolt => "damage",
            DamageSpellShape::Ball | DamageSpellShape::BigBall => "area-damage",
            DamageSpellShape::Beam => "beam-damage",
        },
        "damageDice": dice,
        "damageSides": sides,
        "damageType": damage_type,
    });
    if bonus > 0 {
        effect["damageBonus"] = serde_json::json!(bonus);
    }
    if shape.ball_radius() > 0 {
        effect["radius"] = serde_json::json!(shape.ball_radius());
    }
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.ability.{suffix}"),
        "nameKey": format!("ability-legacy-{suffix}-name"),
        "descriptionKey": format!("ability-legacy-{suffix}-description"),
        "minimumLevel": 1,
        "resourceId": LEGACY_RESOURCE_ID,
        "resourceCost": 1,
        "baseFailurePercent": 20,
        "target": { "modes": ["position", "entity"], "range": 8, "requiresLineOfEffect": true },
        "affectsGroundItems": true,
        "effect": effect,
        "tags": ["legacy-import", "damage"],
    })
}

pub fn convert_content(
    terrain: &[LegacyTerrainEntry],
    monsters: &[LegacyMonsterEntry],
    items: &[LegacyItemEntry],
    egos: &[LegacyEgoEntry],
    artifacts: &[LegacyArtifactEntry],
    characters: &LegacyCharacterSources,
) -> ContentImportOutcome {
    convert_content_from(
        terrain,
        monsters,
        items,
        egos,
        artifacts,
        characters,
        LEGACY_CONTENT_REFERENCE,
    )
}

fn convert_content_from(
    terrain: &[LegacyTerrainEntry],
    monsters: &[LegacyMonsterEntry],
    items: &[LegacyItemEntry],
    egos: &[LegacyEgoEntry],
    artifacts: &[LegacyArtifactEntry],
    characters: &LegacyCharacterSources,
    source_commit: &str,
) -> ContentImportOutcome {
    let mut report = ContentImportReport {
        schema_version: CONTENT_IMPORT_SCHEMA_VERSION,
        source_commit: source_commit.to_owned(),
        ..ContentImportReport::default()
    };
    let mut terrain_files = Vec::new();
    let mut seen_ids = BTreeMap::new();
    let mut terrain_creation = TerrainCreationImportIds::default();
    let mut planned_terrain = Vec::new();
    let mut terrain_ids_by_tag = BTreeMap::new();

    report.terrain_total = terrain.len();
    for entry in terrain {
        if entry.tag.is_empty() || entry.tag == "NONE" || entry.glyph.is_none() {
            report.terrain_skipped += 1;
            *report
                .skip_reasons
                .entry("terrain-placeholder-or-missing-glyph".to_owned())
                .or_default() += 1;
            continue;
        }
        for flag in &entry.flags {
            if !MAPPED_TERRAIN_FLAGS.contains(&flag.as_str()) {
                *report
                    .unmapped_terrain_flags
                    .entry(flag.clone())
                    .or_default() += 1;
            }
        }
        let mut id = kebab(&entry.tag);
        let duplicates = seen_ids.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        let terrain_id = format!("rfb-legacy.terrain.{id}");
        if entry.flags.iter().any(|flag| flag == "FLOOR") {
            terrain_creation.source_terrain_ids.push(terrain_id.clone());
        }
        match entry.tag.as_str() {
            "FLOOR" => terrain_creation.floor_terrain_id = Some(terrain_id.clone()),
            "GLYPH" => terrain_creation.glyph_terrain_id = Some(terrain_id.clone()),
            "TREE" => terrain_creation.tree_terrain_id = Some(terrain_id.clone()),
            "GRANITE" => terrain_creation.wall_terrain_id = Some(terrain_id.clone()),
            "QUARTZ" => terrain_creation.quartz_terrain_id = Some(terrain_id.clone()),
            "MAGMA" => terrain_creation.magma_terrain_id = Some(terrain_id.clone()),
            _ => {}
        }
        terrain_ids_by_tag
            .entry(entry.tag.clone())
            .or_insert(terrain_id);
        planned_terrain.push((entry, id));
        report.terrain_imported += 1;
    }
    for (entry, id) in planned_terrain {
        let destroyed_tag = entry
            .destroyed_tag
            .as_deref()
            .map(|tag| if tag == "*FLOOR*" { "FLOOR" } else { tag });
        let destroy_to = entry
            .flags
            .iter()
            .any(|flag| flag == "HURT_DISI")
            .then(|| destroyed_tag.and_then(|tag| terrain_ids_by_tag.get(tag)))
            .flatten()
            .map(String::as_str);
        terrain_files.push((format!("{id}.json"), terrain_json(entry, &id, destroy_to)));
    }
    terrain_creation.source_terrain_ids.sort();
    if let Some(floor_terrain_id) = &terrain_creation.floor_terrain_id {
        terrain_creation.created_trap_terrain_id =
            Some("rfb-legacy.terrain.created-trap".to_owned());
        terrain_files.push((
            "created-trap.json".to_owned(),
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/terrain.schema.json"),
                "formatVersion": 1,
                "id": "rfb-legacy.terrain.created-trap",
                "nameKey": "terrain-legacy-created-trap-name",
                "descriptionKey": "terrain-legacy-created-trap-description",
                "glyph": "^",
                "walkable": true,
                "blocksSight": false,
                "concealedAsTerrainId": floor_terrain_id,
                "searchCheckDifficulty": 10,
                "trap": {
                    "damage": 4,
                    "damageType": "physical",
                    "disarmToTerrainId": floor_terrain_id,
                    "disarmCheckDifficulty": 10
                },
                "tags": ["legacy-import", "trap"]
            }),
        ));
    }

    let mut actor_files = Vec::new();
    let mut seen_actor_ids = BTreeMap::new();
    let mut shared_abilities: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    report.monsters_total = monsters.len();
    for entry in monsters {
        if entry.name.is_empty() || entry.name == "player" || entry.glyph.is_none() {
            report.monsters_skipped += 1;
            *report
                .skip_reasons
                .entry("monster-placeholder".to_owned())
                .or_default() += 1;
            continue;
        }
        let expressible: Vec<&LegacyBlow> = entry
            .blows
            .iter()
            .filter(|blow| {
                blow.effects
                    .iter()
                    .any(|effect| melee_effects_json(effect, entry.level).is_some())
            })
            .collect();
        let intentional_no_melee = expressible.is_empty()
            && entry.blows.is_empty()
            && entry.flags.iter().any(|flag| flag == "NEVER_BLOW");
        let blow = expressible.first().copied();
        if blow.is_none() && !intentional_no_melee {
            report.monsters_skipped += 1;
            *report
                .skip_reasons
                .entry("monster-without-expressible-melee".to_owned())
                .or_default() += 1;
            for blow in &entry.blows {
                *report
                    .unmapped_blow_methods
                    .entry(blow.method.clone())
                    .or_default() += 1;
            }
            continue;
        }
        if expressible.len() < entry.blows.len() {
            report.monsters_with_inexpressible_blows += 1;
            for blow in &entry.blows {
                if !blow
                    .effects
                    .iter()
                    .any(|effect| melee_effects_json(effect, entry.level).is_some())
                {
                    *report
                        .unmapped_blow_methods
                        .entry(blow.method.clone())
                        .or_default() += 1;
                }
            }
        }
        // Legacy routines cap at four blows; the schema allows eight, so no
        // real entry ever truncates.
        let melee_routine = Some({
            report.monsters_with_melee_routine += 1;
            let blows: Vec<serde_json::Value> = expressible
                .iter()
                .take(8)
                .map(|blow| {
                    let effects = blow
                        .effects
                        .iter()
                        .filter_map(|effect| {
                            let mapped = melee_effects_json(effect, entry.level);
                            if mapped.is_none() {
                                *report
                                    .unmapped_blow_effects
                                    .entry(effect.token.clone())
                                    .or_default() += 1;
                            }
                            mapped
                        })
                        .flatten()
                        .collect::<Vec<_>>();
                    let mut method = kebab(&blow.method);
                    if method.is_empty() {
                        method = "strike".to_owned();
                    }
                    let mut value = serde_json::json!({
                        "methodId": format!("rfb-legacy.blow.{method}"),
                        "toHit": 20,
                        "effects": effects,
                    });
                    if blow.method == "EXPLODE" {
                        value["selfDestructs"] = serde_json::json!(true);
                    }
                    value
                })
                .collect();
            serde_json::json!({ "blows": blows })
        });
        // The stable actor id is settled before spell mapping so kin summons
        // can reference their own caster kind.
        let mut id = kebab(&entry.name);
        if id.is_empty() {
            id = format!("monster-{}", entry.index);
        }
        let duplicates = seen_actor_ids.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        let caster_kind_id = format!("rfb-legacy.actor.{id}");
        let mut frequency_percent: Option<u32> = None;
        let mut mapped_ability_ids: Vec<String> = Vec::new();
        let mut has_unmapped_spell = false;
        // Legacy breaths widen with stature: level 50+ casters and dragon
        // glyphs use the larger cone.
        let breath_radius = if entry.level.unwrap_or(1) >= 50 || entry.glyph == Some('D') {
            3
        } else {
            2
        };
        for spell in &entry.spells {
            if let Some(divisor) = spell.strip_prefix("1_IN_") {
                if let Ok(divisor) = divisor.parse::<u32>() {
                    frequency_percent = Some((100 / divisor.max(1)).clamp(1, 100));
                }
                continue;
            }
            if let Some(percent) = spell.strip_prefix("FREQ_") {
                if let Ok(percent) = percent.parse::<u32>() {
                    frequency_percent = Some(percent.clamp(1, 100));
                }
                continue;
            }
            let base_token = spell.split('(').next().unwrap_or(spell);
            if POSSESSOR_ONLY_SPELLS.contains(&base_token) {
                *report
                    .not_applicable_spells
                    .entry(spell.clone())
                    .or_default() += 1;
                continue;
            }
            if let Some(ability_id) = map_spell_token(
                spell,
                entry.level.unwrap_or(1).max(1),
                breath_radius,
                &caster_kind_id,
                &mut shared_abilities,
            ) {
                if !mapped_ability_ids.contains(&ability_id) {
                    mapped_ability_ids.push(ability_id);
                }
                *report.spells_mapped.entry(spell.clone()).or_default() += 1;
            } else {
                has_unmapped_spell = true;
                *report.unmapped_spells.entry(spell.clone()).or_default() += 1;
            }
        }
        if has_unmapped_spell {
            report.monsters_with_unmapped_spells += 1;
        }
        // The content schema caps a casting profile at 64 abilities; keep
        // declaration order and record any legacy kitchen-sink caster that
        // still overflows.
        if mapped_ability_ids.len() > 64 {
            mapped_ability_ids.truncate(64);
            *report
                .skip_reasons
                .entry("monster-casting-overflow".to_owned())
                .or_default() += 1;
        }
        let monster_casting = (!mapped_ability_ids.is_empty()).then(|| {
            report.monsters_with_casting += 1;
            serde_json::json!({
                "frequencyPercent": frequency_percent.unwrap_or(10),
                "abilities": mapped_ability_ids
                    .iter()
                    .map(|ability_id| serde_json::json!({ "abilityId": ability_id, "weight": 1 }))
                    .collect::<Vec<_>>(),
            })
        });
        for flag in &entry.flags {
            if monster_flag_is_mapped(flag)
                || MONSTER_CONTACT_AURA_FLAGS
                    .iter()
                    .any(|(aura_flag, _)| flag == *aura_flag)
            {
                continue;
            }
            *report
                .unmapped_monster_flags
                .entry(flag.clone())
                .or_default() += 1;
        }
        let (damage_type, unmapped_effect) = blow.map_or(("physical", None), damage_type_for);
        if melee_routine.is_none()
            && let Some(effect) = unmapped_effect
        {
            *report
                .unmapped_blow_effects
                .entry(effect.to_owned())
                .or_default() += 1;
        }
        actor_files.push((
            format!("{id}.json"),
            monster_json(
                entry,
                &id,
                blow,
                damage_type,
                melee_routine,
                monster_casting,
            ),
        ));
        report.monsters_imported += 1;
    }

    let mut item_files = Vec::new();
    let mut seen_item_ids = BTreeMap::new();
    let mut imported_item_ids = BTreeMap::new();
    let mut imported_item_ids_by_kind = BTreeMap::new();
    report.items_total = items.len();
    // Prepass: the first shot/arrow/bolt entry becomes the canonical ammo
    // partner for its launcher class.
    let ammo_index = launcher_ammo_index(items);
    for entry in items {
        if entry.name.is_empty() || entry.name == "something" || entry.glyph.is_none() {
            report.items_skipped += 1;
            *report
                .skip_reasons
                .entry("item-placeholder".to_owned())
                .or_default() += 1;
            continue;
        }
        let mut id = kebab(&entry.name);
        if id.is_empty() {
            id = format!("item-{}", entry.index);
        }
        let duplicates = seen_item_ids.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        imported_item_ids.insert(entry.index, format!("rfb-legacy.item.{id}"));
        imported_item_ids_by_kind
            .entry((entry.tval, entry.sval))
            .or_insert_with(|| format!("rfb-legacy.item.{id}"));
        item_files.push((
            format!("{id}.json"),
            item_json_with_terrain(
                entry,
                &id,
                &ammo_index,
                player_ability_book_for_item(entry),
                Some(&terrain_creation),
                &mut report,
            ),
        ));
        report.items_imported += 1;
    }
    item_files.push((
        "corpse-remains.json".to_owned(),
        serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/item.schema.json"),
            "formatVersion": 1,
            "id": LEGACY_CORPSE_ITEM_ID,
            "nameKey": "item-legacy-corpse-remains-name",
            "descriptionKey": "item-legacy-corpse-remains-description",
            "glyph": "%",
            "weightTenthsPound": 100,
            "maxStack": 1,
            "tags": ["corpse", "legacy-import"],
        }),
    ));
    report.items_imported += 1;

    let loot_entries_for = |eligible: fn(&LegacyItemEntry) -> bool| {
        items
            .iter()
            .filter(|entry| {
                eligible(entry)
                    && !entry.name.is_empty()
                    && entry.name != "something"
                    && entry.glyph.is_some()
            })
            .flat_map(|entry| {
                entry.allocations.iter().map(|allocation| {
                    let mut value = serde_json::json!({
                        "itemKindId": imported_item_ids[&entry.index],
                        "weight": 100 / allocation.chance,
                        "quantity": 1,
                        "minDepth": allocation.level,
                    });
                    if entry.max_level > 0 {
                        value["maxDepth"] = serde_json::json!(entry.max_level);
                    }
                    value
                })
            })
            .collect::<Vec<_>>()
    };
    let loot_entries = loot_entries_for(|_| true);
    let warrior_loot_entries = loot_entries_for(|entry| {
        matches!(entry.tval, 22 | 30 | 31 | 32 | 34 | 37 | 38 | 40 | 45)
            || (entry.tval == 23 && (11..32).contains(&entry.sval))
            || (entry.tval == 75 && matches!(entry.sval, 32 | 33))
    });
    let loot_table = |id: &str, entries: Vec<serde_json::Value>| {
        serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/loot-table.schema.json"),
            "formatVersion": 1,
            "id": id,
            "rolls": 1,
            "entries": entries,
            "qualityWeights": [{ "quality": "ordinary", "weight": 1 }],
            "affixWeights": [{ "weight": 1 }],
        })
    };
    let loot_table_files = vec![
        (
            "monster-drops.json".to_owned(),
            loot_table(LEGACY_DROP_TABLE_ID, loot_entries),
        ),
        (
            "monster-drops-warrior.json".to_owned(),
            loot_table(LEGACY_WARRIOR_DROP_TABLE_ID, warrior_loot_entries),
        ),
    ];

    let mut affix_files = Vec::new();
    let mut seen_affix_ids = BTreeMap::new();
    report.egos_total = egos.len();
    for entry in egos {
        if entry.name.is_empty() {
            continue;
        }
        let mut id = kebab(entry.name.trim_start_matches("of "));
        if id.is_empty() {
            id = format!("ego-{}", entry.index);
        }
        let duplicates = seen_affix_ids.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        let value = ego_json(entry, &id, &mut report);
        // Egos whose entire power set lives in unmappable flags produce no
        // substance at all, which the affix contract rejects; skip them but
        // keep their flags visible in the gap report above.
        if value.get("modifiers").is_none()
            && value.get("equipmentBonuses").is_none()
            && value.get("resistances").is_none()
            && value.get("statusImmunities").is_none()
            && value.get("slays").is_none()
            && value.get("brands").is_none()
            && value.get("passives").is_none()
            && value.get("elementalDestructionVulnerabilities").is_none()
            && value.get("elementalDestructionImmunities").is_none()
            && value.get("resistsProjectionDestruction").is_none()
            && value.get("resistsMonsterDestruction").is_none()
            && value.get("rollGroups").is_none()
        {
            *report
                .skip_reasons
                .entry("ego-inexpressible".to_owned())
                .or_default() += 1;
            continue;
        }
        affix_files.push((format!("{id}.json"), value));
        report.egos_imported += 1;
    }
    report.artifacts_total = artifacts.len();
    for entry in artifacts {
        if entry.name.is_empty() || entry.tval == 0 {
            continue;
        }
        let mut id = kebab(entry.name.trim_start_matches("of "));
        if id.is_empty() {
            id = format!("artifact-{}", entry.index);
        }
        let duplicates = seen_item_ids
            .entry(format!("artifact-{id}"))
            .or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
        item_files.push((
            format!("artifact-{id}.json"),
            artifact_json(
                entry,
                &id,
                imported_item_ids_by_kind
                    .get(&(entry.tval, entry.sval))
                    .map(String::as_str),
                &ammo_index,
                &mut report,
            ),
        ));
        report.artifacts_imported += 1;
    }

    let mut race_files = Vec::new();
    let mut class_files = Vec::new();
    let mut personality_files = Vec::new();
    let mut magic_profile_files = Vec::new();
    let mut skill_set_files = Vec::new();
    report.bodies_total = characters.bodies.len();
    // Slot-gap census runs over every template; only the Standard body has
    // a binding surface today (player races), the rest await possessor and
    // monster-race play.
    let mut standard_slots: Vec<(String, String)> = Vec::new();
    for body in &characters.bodies {
        let mapped = map_body_slots(body, &mut report.body_slot_gaps);
        if body.name == "Standard" {
            standard_slots = mapped;
        }
    }
    report.races_total = characters.races.len();
    for entry in &characters.races {
        if entry.dynamic {
            *report
                .skip_reasons
                .entry("race-code-dynamic".to_owned())
                .or_default() += 1;
            continue;
        }
        race_files.push((
            format!("{}.json", entry.id),
            race_json(entry, &standard_slots, &mut report),
        ));
        skill_set_files.push((
            format!("race-{}.json", entry.id),
            character_skill_set_json(entry, &format!("race-{}", entry.id)),
        ));
        report.races_imported += 1;
    }
    report.personalities_total = characters.personalities.len();
    for entry in &characters.personalities {
        if entry.dynamic {
            *report
                .skip_reasons
                .entry("personality-code-dynamic".to_owned())
                .or_default() += 1;
            continue;
        }
        personality_files.push((
            format!("{}.json", entry.id),
            personality_json(entry, &mut report),
        ));
        skill_set_files.push((
            format!("personality-{}.json", entry.id),
            character_skill_set_json(entry, &format!("personality-{}", entry.id)),
        ));
        report.personalities_imported += 1;
    }
    let class_ids = characters
        .classes
        .iter()
        .map(|entry| (entry.registration.index, entry.registration.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let magic_profile_indices = characters
        .magic_profiles
        .iter()
        .map(|profile| profile.class_index)
        .collect::<BTreeSet<_>>();
    let casting_shell_indices = characters
        .magic_profiles
        .iter()
        .filter(|profile| {
            profile.realms.iter().any(|realm| realm.readable)
                || profile.book_type != "none"
                || profile.first_spell_level != 99
        })
        .map(|profile| profile.class_index)
        .collect::<BTreeSet<_>>();
    report.classes_with_casting_shells = casting_shell_indices.len();
    report.classes_total = characters.classes.len();
    report.classes_registered = characters
        .classes
        .iter()
        .filter(|entry| entry.registration.registered)
        .count();
    report.classes_with_source = characters
        .classes
        .iter()
        .filter(|entry| entry.source_found)
        .count();
    report.class_caster_profiles_imported = characters
        .classes
        .iter()
        .filter(|entry| entry.caster_profile.is_some())
        .count();
    report.class_caster_profiles_dynamic = characters
        .classes
        .iter()
        .filter(|entry| {
            entry
                .caster_profile
                .as_ref()
                .is_some_and(|profile| profile.dynamic)
        })
        .count();
    let magic_profiles_by_class = characters
        .magic_profiles
        .iter()
        .map(|profile| (profile.class_index, profile))
        .collect::<BTreeMap<_, _>>();
    let mut runtime_casting_profiles = BTreeMap::new();
    let mut mapped_player_spell_rows = BTreeSet::new();
    let mut death_first_ability_ids = BTreeMap::new();
    let mut death_second_ability_ids = BTreeMap::new();
    let mut death_third_ability_ids = BTreeMap::new();
    let mut death_fourth_ability_ids = BTreeMap::new();
    for entry in &characters.classes {
        let Some(magic_profile) = magic_profiles_by_class
            .get(&entry.registration.index)
            .copied()
        else {
            continue;
        };
        let Some(realm) = death_realm(magic_profile) else {
            continue;
        };
        let Some(casting_attribute) = runtime_casting_attribute(entry) else {
            *report
                .class_magic_gaps
                .entry("runtime-casting-profile-unsupported".to_owned())
                .or_default() += 1;
            continue;
        };
        let mut has_first_book = false;
        let mut has_second_book = false;
        let mut has_third_book = false;
        let mut has_fourth_book = false;
        let overrides = realm
            .spells
            .iter()
            .filter_map(|spell| {
                let (ability_id, ability) = death_spell_ability(spell, &terrain_creation)?;
                shared_abilities.entry(ability_id.clone()).or_insert(ability);
                if spell.index < 8 {
                    has_first_book = true;
                    death_first_ability_ids.insert(spell.index, ability_id.clone());
                } else if spell.index < 16 {
                    has_second_book = true;
                    death_second_ability_ids.insert(spell.index, ability_id.clone());
                } else if spell.index < 24 {
                    has_third_book = true;
                    death_third_ability_ids.insert(spell.index, ability_id.clone());
                } else {
                    has_fourth_book = true;
                    death_fourth_ability_ids.insert(spell.index, ability_id.clone());
                }
                mapped_player_spell_rows.insert((
                    magic_profile.class_index,
                    realm.index,
                    spell.index,
                ));
                report.player_spell_parameter_overrides += 1;
                report.player_spell_mapped_rows += 1;
                let mut override_ = serde_json::json!({
                    "abilityId": ability_id,
                    "minimumLevel": spell.level.max(entry.caster_profile.as_ref()?.minimum_level).max(1),
                    "resourceCost": u32::from(spell.mana.max(1)),
                    "baseFailurePercent": spell.failure_percent.min(95),
                });
                if spell.index == 8 && !death_entropy_uses_three_halves(&entry.registration.id) {
                    override_["levelScaling"] = serde_json::json!([
                        {
                            "effectIndex": 0,
                            "field": "damage-bonus",
                            "multiplier": 5,
                            "divisor": 4,
                        },
                        {
                            "effectIndex": 0,
                            "field": "radius",
                            "multiplier": 1,
                            "divisor": 30,
                            "maximum": 3,
                        },
                    ]);
                }
                Some(override_)
            })
            .collect::<Vec<_>>();
        if overrides.is_empty() {
            continue;
        }
        let caster = entry
            .caster_profile
            .as_ref()
            .expect("runtime casting attribute requires a caster profile");
        let (beam_chance_level_multiplier, beam_chance_level_divisor, beam_chance_bonus) =
            legacy_beam_chance_profile(&entry.registration.id);
        let mut ability_book_ids = Vec::new();
        if has_first_book {
            ability_book_ids.push(DEATH_FIRST_BOOK_ID);
        }
        if has_second_book {
            ability_book_ids.push(DEATH_SECOND_BOOK_ID);
        }
        if has_third_book {
            ability_book_ids.push(DEATH_THIRD_BOOK_ID);
        }
        if has_fourth_book {
            ability_book_ids.push(DEATH_FOURTH_BOOK_ID);
        }
        runtime_casting_profiles.insert(
            entry.registration.index,
            serde_json::json!({
                "resourceId": LEGACY_MANA_RESOURCE_ID,
                "castingAttribute": casting_attribute,
                "baseCapacity": 0,
                "capacityPerLevel": 1,
                "capacityPerAttributeIndex": 1,
                "baseLearningCapacity": 4,
                "learningCapacityPerLevel": 1,
                "learningCapacityPerAttributeIndex": 0,
                "learningCapacityCap": 32,
                "minimumFailurePercent": caster.minimum_failure_percent,
                "beamChanceLevelMultiplier": beam_chance_level_multiplier,
                "beamChanceLevelDivisor": beam_chance_level_divisor,
                "beamChanceBonus": beam_chance_bonus,
                "abilityBookIds": ability_book_ids,
                "abilityOverrides": overrides,
            }),
        );
        report.classes_with_runtime_casting_profiles += 1;
        for gap in [
            "caster-encumbrance",
            "mana-capacity-formula",
            "spell-learning-formula",
        ] {
            *report
                .player_spell_behavior_gaps
                .entry(gap.to_owned())
                .or_default() += 1;
        }
    }
    let mut ability_book_files = Vec::new();
    if !death_first_ability_ids.is_empty() {
        let ability_ids = death_first_ability_ids.into_values().collect::<Vec<_>>();
        report.player_abilities_imported += ability_ids.len();
        report.player_ability_books_imported += 1;
        ability_book_files.push((
            "death-black-prayers.json".to_owned(),
            death_first_book_json(&ability_ids),
        ));
    }
    if !death_second_ability_ids.is_empty() {
        let ability_ids = death_second_ability_ids.into_values().collect::<Vec<_>>();
        report.player_abilities_imported += ability_ids.len();
        report.player_ability_books_imported += 1;
        ability_book_files.push((
            "death-black-mass.json".to_owned(),
            death_second_book_json(&ability_ids),
        ));
    }
    if !death_third_ability_ids.is_empty() {
        let ability_ids = death_third_ability_ids.into_values().collect::<Vec<_>>();
        report.player_abilities_imported += ability_ids.len();
        report.player_ability_books_imported += 1;
        ability_book_files.push((
            "death-black-channels.json".to_owned(),
            death_third_book_json(&ability_ids),
        ));
    }
    if !death_fourth_ability_ids.is_empty() {
        let ability_ids = death_fourth_ability_ids.into_values().collect::<Vec<_>>();
        report.player_abilities_imported += ability_ids.len();
        report.player_ability_books_imported += 1;
        ability_book_files.push((
            "death-necronomicon.json".to_owned(),
            death_fourth_book_json(&ability_ids),
        ));
        race_files.push((
            "vampire-lord-form.json".to_owned(),
            vampire_lord_race_json(),
        ));
        skill_set_files.push((
            "race-vampire-lord-form.json".to_owned(),
            vampire_lord_skill_set_json(),
        ));
    }
    for entry in &characters.classes {
        let id = &entry.registration.id;
        let has_magic_profile = magic_profile_indices.contains(&entry.registration.index);
        if !has_magic_profile {
            *report
                .class_magic_gaps
                .entry("m-info-profile-missing".to_owned())
                .or_default() += 1;
        }
        class_files.push((
            format!("{id}.json"),
            class_json(
                entry,
                casting_shell_indices.contains(&entry.registration.index),
                runtime_casting_profiles.get(&entry.registration.index),
                &mut report,
            ),
        ));
        skill_set_files.push((
            format!("class-{id}.json"),
            character_skill_set_json(&entry.character, &format!("class-{id}")),
        ));
        report.classes_imported += 1;
    }
    report.magic_profiles_total = characters.magic_profiles.len();
    for profile in &characters.magic_profiles {
        let Some(class_id) = class_ids.get(&profile.class_index) else {
            *report
                .class_magic_gaps
                .entry("m-info-class-missing".to_owned())
                .or_default() += 1;
            continue;
        };
        let readable_realms = profile.realms.iter().filter(|realm| realm.readable).count();
        if readable_realms > 0 {
            report.classes_with_readable_realms += 1;
        }
        report.magic_realm_rows += profile.realms.len();
        report.magic_readable_realm_rows += readable_realms;
        let has_casting_surface =
            readable_realms > 0 || profile.book_type != "none" || profile.first_spell_level != 99;
        if has_casting_surface
            && !matches!(profile.casting_attribute.as_str(), "int" | "wis" | "chr")
        {
            *report
                .casting_attribute_gaps
                .entry(profile.casting_attribute.clone())
                .or_default() += 1;
        }
        for realm in &profile.realms {
            let realm_id = LEGACY_REALM_IDS
                .get(usize::from(realm.index))
                .copied()
                .unwrap_or("unknown");
            if realm.readable {
                *report
                    .realm_readability
                    .entry(realm_id.to_owned())
                    .or_default() += 1;
            }
            report.magic_spell_profile_rows += realm.spells.len();
            let unmapped_spells = realm
                .spells
                .iter()
                .filter(|spell| {
                    !mapped_player_spell_rows.contains(&(
                        profile.class_index,
                        realm.index,
                        spell.index,
                    ))
                })
                .count();
            *report
                .player_spell_effect_gaps
                .entry(realm_id.to_owned())
                .or_default() += unmapped_spells;
        }
        magic_profile_files.push((
            format!("{class_id}.json"),
            magic_profile_json(profile, class_id, source_commit),
        ));
    }
    report.proficiency_profiles_total = characters.proficiency_profiles.len();
    for profile in &characters.proficiency_profiles {
        report.proficiency_weapon_rows += profile.weapon_entries.len();
        *report
            .class_proficiency_gaps
            .entry("weapon-proficiency".to_owned())
            .or_default() += profile.weapon_entries.len();
        for entry in &profile.skill_entries {
            let gap = match entry.skill_index {
                0 => "martial-arts-proficiency".to_owned(),
                1 => "dual-wielding-proficiency".to_owned(),
                2 => "riding-proficiency".to_owned(),
                other => format!("skill-{other}-proficiency"),
            };
            report.proficiency_skill_rows += 1;
            *report.class_proficiency_gaps.entry(gap).or_default() += 1;
        }
    }
    let realm_readability = (!characters.magic_profiles.is_empty())
        .then(|| realm_readability_json(&characters.magic_profiles, &class_ids, source_commit));
    let class_casting_shells = (!characters.classes.is_empty()).then(|| {
        class_casting_shells_json(
            &characters.classes,
            &characters.magic_profiles,
            source_commit,
        )
    });
    let skill_files =
        if race_files.is_empty() && personality_files.is_empty() && class_files.is_empty() {
            Vec::new()
        } else {
            legacy_skill_files()
        };

    let ability_files = shared_abilities
        .into_iter()
        .map(|(id, value)| {
            let name = id
                .rsplit('.')
                .next()
                .expect("generated ability id has a tail")
                .to_owned();
            (format!("{name}.json"), value)
        })
        .collect::<Vec<_>>();
    let mut resource_files = if ability_files.is_empty() {
        Vec::new()
    } else {
        vec![(
            "essence.json".to_owned(),
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/resource.schema.json"),
                "formatVersion": 1,
                "id": LEGACY_RESOURCE_ID,
                "nameKey": "resource-legacy-essence-name",
                "descriptionKey": "resource-legacy-essence-description",
                "waitRecoveryAmount": 0,
                "restRecoveryAmount": 0,
                "tags": ["legacy-import"],
            }),
        )]
    };
    if report.player_abilities_imported > 0 {
        resource_files.push((
            "mana.json".to_owned(),
            serde_json::json!({
                "$schema": format!("{SCHEMA_BASE}/resource.schema.json"),
                "formatVersion": 1,
                "id": LEGACY_MANA_RESOURCE_ID,
                "nameKey": "resource-legacy-mana-name",
                "descriptionKey": "resource-legacy-mana-description",
                "waitRecoveryAmount": 1,
                "restRecoveryAmount": 3,
                "tags": ["casting", "legacy-import", "mana"],
            }),
        ));
    }

    ContentImportOutcome {
        report,
        terrain_files,
        actor_files,
        ability_files,
        ability_book_files,
        resource_files,
        item_files,
        loot_table_files,
        affix_files,
        race_files,
        class_files,
        personality_files,
        magic_profile_files,
        realm_readability,
        class_casting_shells,
        skill_files,
        skill_set_files,
    }
}

fn effect_program_from_inline(
    id: &str,
    effect: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let steps = if effect.get("type").and_then(serde_json::Value::as_str) == Some("sequence") {
        effect
            .get("effects")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .ok_or_else(|| format!("{id} sequence has no effects array"))?
    } else {
        vec![effect]
    };
    let mut input = None;
    for step in &steps {
        let step_type = step
            .get("type")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{id} step has no effect type"))?;
        let step_input = match step_type {
            "damage" => "actor",
            "identify-item" | "enchant-item" | "recharge-from-device" => "item",
            "genocide" => "glyph",
            _ => "self",
        };
        if input
            .replace(step_input)
            .is_some_and(|input| input != step_input)
        {
            return Err(format!("{id} mixes incompatible effect inputs"));
        }
    }
    let input = input.ok_or_else(|| format!("{id} has no effect steps"))?;
    Ok(serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/effect-program.schema.json"),
        "formatVersion": 1,
        "id": id,
        "input": input,
        "steps": steps,
    }))
}

fn extract_item_effect_programs(
    item_files: &mut [(String, serde_json::Value)],
) -> Result<Vec<(String, serde_json::Value)>, String> {
    let mut programs = BTreeMap::new();
    for (file_name, item) in item_files {
        let stem = file_name
            .strip_suffix(".json")
            .ok_or_else(|| format!("item source file {file_name} has no .json suffix"))?;

        if let Some(action) = item
            .get_mut("useAction")
            .and_then(serde_json::Value::as_object_mut)
        {
            let effect = action
                .remove("effect")
                .ok_or_else(|| format!("{file_name} useAction has no inline effect"))?;
            let program_id = format!("rfb-legacy.effect.{stem}.use");
            action.insert(
                "effectProgramId".to_owned(),
                serde_json::Value::String(program_id.clone()),
            );
            let program = effect_program_from_inline(&program_id, effect)?;
            if programs
                .insert(format!("{stem}-use.json"), program)
                .is_some()
            {
                return Err(format!("duplicate effect program for {file_name}"));
            }
        }

        if let Some(effect) = item
            .as_object_mut()
            .and_then(|item| item.remove("shatterEffect"))
        {
            let program_id = format!("rfb-legacy.effect.{stem}.shatter");
            item["shatterEffectProgramId"] = serde_json::json!(program_id);
            let mut program = effect_program_from_inline(&program_id, effect)?;
            program["input"] = serde_json::json!("area");
            if programs
                .insert(format!("{stem}-shatter.json"), program)
                .is_some()
            {
                return Err(format!("duplicate shatter effect program for {file_name}"));
            }
        }

        let Some(activations) = item
            .get_mut("deviceGeneration")
            .and_then(|generation| generation.get_mut("activations"))
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for activation in activations {
            let activation = activation
                .as_object_mut()
                .ok_or_else(|| format!("{file_name} has a non-object device activation"))?;
            let activation_id = activation
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("{file_name} device activation has no id"))?
                .to_owned();
            let activation_stem = activation_id
                .rsplit('.')
                .next()
                .ok_or_else(|| format!("{activation_id} has no stable id suffix"))?;
            let program_id = format!("rfb-legacy.effect.{stem}.{activation_stem}");
            let effect = activation
                .remove("effect")
                .ok_or_else(|| format!("{activation_id} has no inline effect"))?;
            activation.insert(
                "effectProgramId".to_owned(),
                serde_json::Value::String(program_id.clone()),
            );
            let program = effect_program_from_inline(&program_id, effect)?;
            if programs
                .insert(format!("{stem}-{activation_stem}.json"), program)
                .is_some()
            {
                return Err(format!("duplicate effect program for {activation_id}"));
            }
        }
    }
    Ok(programs.into_iter().collect())
}

struct ExtractedAbilitySources {
    program_files: Vec<(String, serde_json::Value)>,
    player_binding_files: Vec<(String, serde_json::Value)>,
}

fn ability_effect_affects_ground_items(effect: &serde_json::Value) -> bool {
    match effect.get("type").and_then(serde_json::Value::as_str) {
        Some(
            "damage"
            | "malediction"
            | "area-damage"
            | "beam-damage"
            | "bolt-or-beam-damage"
            | "bolt-or-area-damage",
        ) => true,
        Some("sequence") => effect
            .get("effects")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|effects| effects.iter().any(ability_effect_affects_ground_items)),
        Some("random-choice") => effect
            .get("branches")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|branches| {
                branches.iter().any(|branch| {
                    branch
                        .get("effect")
                        .is_some_and(ability_effect_affects_ground_items)
                })
            }),
        _ => false,
    }
}

pub fn sync_demo_ability_ground_items(
    abilities_path: &Path,
    programs_path: &Path,
) -> Result<usize, LegacyImportError> {
    let mut programs = BTreeMap::new();
    for entry in fs::read_dir(programs_path)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
        let Some(id) = value.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let affects_ground_items = value
            .get("steps")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|steps| steps.iter().any(ability_effect_affects_ground_items));
        programs.insert(id.to_owned(), affects_ground_items);
    }

    let mut changed = 0;
    for entry in fs::read_dir(abilities_path)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        let program_id = value
            .get("abilityProgramId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "{} has no abilityProgramId",
                    path.display()
                ))
            })?;
        let affects_ground_items = programs.get(program_id).copied().ok_or_else(|| {
            LegacyImportError::InvalidDemoMonsterSelection(format!(
                "{} references missing program {program_id}",
                path.display()
            ))
        })?;
        let before = value.clone();
        if affects_ground_items {
            value["affectsGroundItems"] = serde_json::Value::Bool(true);
        } else {
            value
                .as_object_mut()
                .expect("ability definitions are JSON objects")
                .remove("affectsGroundItems");
        }
        if value != before {
            fs::write(path, serde_json::to_string_pretty(&value)? + "\n")?;
            changed += 1;
        }
    }
    Ok(changed)
}

fn extract_ability_programs_and_player_bindings(
    ability_files: &mut [(String, serde_json::Value)],
    player_ability_ids: &BTreeSet<String>,
) -> Result<ExtractedAbilitySources, String> {
    let mut programs = BTreeMap::new();
    let mut bindings = BTreeMap::new();
    for (file_name, ability) in ability_files {
        let stem = file_name
            .strip_suffix(".json")
            .ok_or_else(|| format!("ability source file {file_name} has no .json suffix"))?;
        let ability = ability
            .as_object_mut()
            .ok_or_else(|| format!("{file_name} ability source is not an object"))?;
        let ability_id = ability
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{file_name} ability source has no id"))?
            .to_owned();
        let target_modes = ability
            .get("target")
            .and_then(|target| target.get("modes"))
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("{file_name} ability source has no target modes"))?;
        let input = match target_modes.as_slice() {
            [mode] if mode.as_str() == Some("self") => "self",
            [mode] if mode.as_str() == Some("item") => "item",
            modes if !modes.is_empty() => "cast-target",
            _ => return Err(format!("{file_name} ability source has empty target modes")),
        };
        let affects_ground_items = ability
            .get("effect")
            .is_some_and(ability_effect_affects_ground_items);
        if affects_ground_items {
            ability.insert(
                "affectsGroundItems".to_owned(),
                serde_json::Value::Bool(true),
            );
        }
        let effect = ability
            .remove("effect")
            .ok_or_else(|| format!("{file_name} ability source has no inline effect"))?;
        let steps = if effect.get("type").and_then(serde_json::Value::as_str) == Some("sequence") {
            effect
                .get("effects")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .ok_or_else(|| format!("{file_name} sequence has no effects array"))?
        } else {
            vec![effect]
        };
        let program_id = format!("rfb-legacy.ability-program.{stem}");
        ability.insert(
            "abilityProgramId".to_owned(),
            serde_json::Value::String(program_id.clone()),
        );
        let program = serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/ability-program.schema.json"),
            "formatVersion": 1,
            "id": program_id,
            "input": input,
            "steps": steps,
        });
        if programs.insert(file_name.clone(), program).is_some() {
            return Err(format!("duplicate ability program for {file_name}"));
        }

        let minimum_level = ability.remove("minimumLevel");
        let resource_id = ability.remove("resourceId");
        let resource_cost = ability.remove("resourceCost");
        let base_failure_percent = ability.remove("baseFailurePercent");
        let proficiency = ability.remove("proficiency");
        let cooldown = ability.remove("cooldown");
        if !player_ability_ids.contains(&ability_id) {
            continue;
        }
        let mut binding = serde_json::json!({
            "$schema": format!("{SCHEMA_BASE}/player-ability-binding.schema.json"),
            "formatVersion": 1,
            "abilityId": ability_id,
            "minimumLevel": minimum_level
                .ok_or_else(|| format!("{file_name} player ability has no minimumLevel"))?,
            "resourceId": resource_id
                .ok_or_else(|| format!("{file_name} player ability has no resourceId"))?,
            "resourceCost": resource_cost
                .ok_or_else(|| format!("{file_name} player ability has no resourceCost"))?,
            "baseFailurePercent": base_failure_percent
                .ok_or_else(|| format!("{file_name} player ability has no baseFailurePercent"))?,
        });
        if let Some(value) = proficiency {
            binding["proficiency"] = value;
        }
        if let Some(value) = cooldown {
            binding["cooldown"] = value;
        }
        if bindings.insert(file_name.clone(), binding).is_some() {
            return Err(format!("duplicate player ability binding for {file_name}"));
        }
    }
    Ok(ExtractedAbilitySources {
        program_files: programs.into_iter().collect(),
        player_binding_files: bindings.into_iter().collect(),
    })
}

fn imported_player_ability_ids(
    ability_book_files: &[(String, serde_json::Value)],
) -> BTreeSet<String> {
    ability_book_files
        .iter()
        .flat_map(|(_, book)| {
            book.get("abilityIds")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

pub fn import_content(source: &Path, output: &Path) -> Result<PathBuf, LegacyImportError> {
    let canonical_source = source
        .canonicalize()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if output.starts_with(&canonical_source) {
        return Err(LegacyImportError::LegacyGit(
            "output directory must live outside the legacy source".to_owned(),
        ));
    }
    let source_commit = resolve_legacy_content_commit(source)?;
    let f_info = read_legacy_object_at(source, &source_commit, "lib/edit/f_info.txt")?;
    let r_info = read_legacy_object_at(source, &source_commit, "lib/edit/r_info.txt")?;
    let k_info = read_legacy_object_at(source, &source_commit, "lib/edit/k_info.txt")?;
    let e_info = read_legacy_object_at(source, &source_commit, "lib/edit/e_info.txt")?;
    let a_info = read_legacy_object_at(source, &source_commit, "lib/edit/a_info.txt")?;
    let b_info = read_legacy_object_at(source, &source_commit, "lib/edit/b_info.txt")?;
    let m_info = read_legacy_object_at(source, &source_commit, "lib/edit/m_info.txt")?;
    let s_info = read_legacy_object_at(source, &source_commit, "lib/edit/s_info.txt")?;
    let defines = read_legacy_object_at(source, &source_commit, "src/defines.h")?;
    let classes_source = read_legacy_object_at(source, &source_commit, "src/classes.c")?;
    let terrain = parse_f_info(&f_info)?;
    let monsters = parse_r_info(&r_info)?;
    let items = parse_k_info(&k_info)?;
    let egos = parse_e_info(&e_info)?;
    let artifacts = parse_a_info(&a_info)?;
    let bodies = parse_b_info(&b_info)?;
    let magic_profiles = parse_m_info(&m_info)?;
    let proficiency_profiles = parse_s_info(&s_info)?;
    let mut class_registrations = parse_class_registrations(&defines, &classes_source);
    let mut registered_indices = class_registrations
        .iter()
        .map(|entry| entry.index)
        .collect::<BTreeSet<_>>();
    for profile in &magic_profiles {
        if registered_indices.insert(profile.class_index) {
            let id = kebab(&profile.name_hint);
            class_registrations.push(LegacyClassRegistration {
                index: profile.class_index,
                function: format!("{}_get_class", id.replace('-', "_")),
                id,
                registered: false,
            });
        }
    }
    class_registrations.sort_by_key(|entry| entry.index);
    let mut characters = LegacyCharacterSources {
        bodies,
        magic_profiles,
        proficiency_profiles,
        ..LegacyCharacterSources::default()
    };
    let source_objects = list_legacy_c_sources(source, &source_commit)?
        .into_iter()
        .map(|path| {
            let text = read_legacy_object_at(source, &source_commit, &path)?;
            Ok((path, text))
        })
        .collect::<Result<Vec<_>, LegacyImportError>>()?;
    let mut seen_character_ids = BTreeSet::new();
    for (path, text) in &source_objects {
        for (name, body) in extract_race_blocks(text) {
            let mut entry = parse_character_block(&name, &body);
            if let Some(hook) = entry.calc_bonuses_fn.clone() {
                let (resistances, free_act, see_invisible, attribute_sustains, speed) =
                    parse_calc_bonuses_defenses(text, &hook);
                entry.resistances = resistances;
                entry.free_act = free_act;
                entry.see_invisible = see_invisible;
                entry.attribute_sustains = attribute_sustains;
                entry.speed = speed;
            }
            parse_race_powers(text, &mut entry);
            if seen_character_ids.insert(format!("race:{}", entry.id)) {
                characters.races.push(entry);
            }
        }
        if Path::new(path)
            .file_name()
            .is_some_and(|name| name == "personality.c")
        {
            for (name, body) in extract_personality_blocks(text) {
                let entry = parse_character_block(&name, &body);
                if seen_character_ids.insert(format!("personality:{}", entry.id)) {
                    characters.personalities.push(entry);
                }
            }
        }
    }
    for registration in class_registrations {
        let parsed = source_objects.iter().find_map(|(_, text)| {
            extract_class_block(text, &registration.function).map(|body| {
                let mut entry = parse_class_block(registration.clone(), body);
                entry.caster_profile = parse_class_caster_profile(text, body);
                if entry.caster_profile.is_some() {
                    entry.character.hooks.retain(|hook| hook != "caster_info");
                }
                if entry
                    .caster_profile
                    .as_ref()
                    .is_some_and(|profile| profile.dynamic)
                {
                    entry.character.dynamic = true;
                    entry.character.hooks.push("dynamic-caster-info".to_owned());
                }
                entry
            })
        });
        characters.classes.push(parsed.unwrap_or_else(|| {
            let mut character = LegacyCharacterEntry {
                id: registration.id.clone(),
                dynamic: true,
                ..LegacyCharacterEntry::default()
            };
            character.hooks.push("missing-class-source".to_owned());
            LegacyClassEntry {
                registration,
                character,
                caster_profile: None,
                pet_upkeep_divisor: 40,
                source_found: false,
            }
        }));
    }
    let mut outcome = convert_content_from(
        &terrain,
        &monsters,
        &items,
        &egos,
        &artifacts,
        &characters,
        &source_commit,
    );
    let effect_program_files = extract_item_effect_programs(&mut outcome.item_files)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let player_ability_ids = imported_player_ability_ids(&outcome.ability_book_files);
    let extracted_ability_sources = extract_ability_programs_and_player_bindings(
        &mut outcome.ability_files,
        &player_ability_ids,
    )
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let ability_program_files = extracted_ability_sources.program_files;
    let player_ability_binding_files = extracted_ability_sources.player_binding_files;

    let terrain_dir = output.join("terrain");
    let actor_dir = output.join("actors");
    fs::create_dir_all(&terrain_dir)?;
    fs::create_dir_all(&actor_dir)?;
    for (directory, files) in [
        ("abilities", &outcome.ability_files),
        ("abilityBooks", &outcome.ability_book_files),
        ("abilityPrograms", &ability_program_files),
        ("resources", &outcome.resource_files),
        ("effectPrograms", &effect_program_files),
        ("items", &outcome.item_files),
        ("lootTables", &outcome.loot_table_files),
        ("affixes", &outcome.affix_files),
        ("races", &outcome.race_files),
        ("classes", &outcome.class_files),
        ("personalities", &outcome.personality_files),
        ("playerAbilityBindings", &player_ability_binding_files),
        ("skills", &outcome.skill_files),
        ("skillSets", &outcome.skill_set_files),
    ] {
        if files.is_empty() {
            continue;
        }
        let target = output.join(directory);
        fs::create_dir_all(&target)?;
        for (name, value) in files {
            fs::write(
                target.join(name),
                serde_json::to_string_pretty(value)? + "\n",
            )?;
        }
    }
    if !outcome.magic_profile_files.is_empty() {
        let target = output.join("legacyMagicProfiles");
        fs::create_dir_all(&target)?;
        for (name, value) in &outcome.magic_profile_files {
            fs::write(
                target.join(name),
                serde_json::to_string_pretty(value)? + "\n",
            )?;
        }
        if let Some(value) = &outcome.realm_readability {
            fs::write(
                target.join("realm-readability.json"),
                serde_json::to_string_pretty(value)? + "\n",
            )?;
        }
        if let Some(value) = &outcome.class_casting_shells {
            fs::write(
                target.join("class-casting-shells.json"),
                serde_json::to_string_pretty(value)? + "\n",
            )?;
        }
    }
    for (name, value) in &outcome.terrain_files {
        fs::write(
            terrain_dir.join(name),
            serde_json::to_string_pretty(value)? + "\n",
        )?;
    }
    for (name, value) in &outcome.actor_files {
        fs::write(
            actor_dir.join(name),
            serde_json::to_string_pretty(value)? + "\n",
        )?;
    }
    let mut content_roots = Vec::new();
    if !outcome.ability_files.is_empty() {
        content_roots.push("abilities");
    }
    if !outcome.ability_book_files.is_empty() {
        content_roots.push("abilityBooks");
    }
    if !ability_program_files.is_empty() {
        content_roots.push("abilityPrograms");
    }
    content_roots.push("actors");
    if !outcome.affix_files.is_empty() {
        content_roots.push("affixes");
    }
    if !outcome.class_files.is_empty() {
        content_roots.push("classes");
    }
    if !effect_program_files.is_empty() {
        content_roots.push("effectPrograms");
    }
    if !outcome.item_files.is_empty() {
        content_roots.push("items");
    }
    if !outcome.loot_table_files.is_empty() {
        content_roots.push("lootTables");
    }
    if !outcome.personality_files.is_empty() {
        content_roots.push("personalities");
    }
    if !player_ability_binding_files.is_empty() {
        content_roots.push("playerAbilityBindings");
    }
    if !outcome.race_files.is_empty() {
        content_roots.push("races");
    }
    if !outcome.ability_files.is_empty() {
        content_roots.push("resources");
    }
    if !outcome.skill_files.is_empty() {
        content_roots.push("skills");
    }
    if !outcome.skill_set_files.is_empty() {
        content_roots.push("skillSets");
    }
    content_roots.push("terrain");
    let pack_manifest = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/pack.schema.json"),
        "formatVersion": 1,
        "id": "rfb.legacy.frog-v1",
        "version": "0.1.0",
        "titleKey": "pack-rfb-legacy-title",
        "dependencies": [],
        "loadAfter": [],
        "contentRoots": content_roots,
    });
    fs::write(
        output.join("pack.json"),
        serde_json::to_string_pretty(&pack_manifest)? + "\n",
    )?;
    let report_path = output.join("import-report.json");
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&outcome.report)? + "\n",
    )?;
    Ok(report_path)
}

fn wilderness_terrain_id(index: u8) -> &'static str {
    match index {
        0 => "edge",
        1 => "town",
        2 => "deep-water",
        3 => "shallow-water",
        4 => "swamp",
        5 => "dirt",
        6 => "grass",
        7 => "trees",
        8 => "desert",
        9 => "shallow-lava",
        10 => "deep-lava",
        11 => "mountain",
        12 => "glacier",
        13 => "snow",
        14 => "pack-ice",
        _ => unreachable!("wilderness parser accepts exactly the 15 RFB terrain indexes"),
    }
}

fn replace_wilderness_property(
    source: &str,
    wilderness: &serde_json::Value,
) -> Result<String, LegacyImportError> {
    let parsed: serde_json::Value = serde_json::from_str(source)?;
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let width_marker = format!("{newline}  \"width\":");
    let width_start = source.find(&width_marker).ok_or_else(|| {
        LegacyImportError::InvalidDemoWildernessSelection(
            "world output must use the committed two-space JSON layout".to_owned(),
        )
    })?;
    let property_marker = format!("{newline}  \"wilderness\":");
    let property_start = source.find(&property_marker);
    if parsed.get("wilderness").is_some() != property_start.is_some()
        || property_start.is_some_and(|start| start > width_start)
    {
        return Err(LegacyImportError::InvalidDemoWildernessSelection(
            "existing wilderness field is outside the expected world header".to_owned(),
        ));
    }
    let encoded = serde_json::to_string_pretty(wilderness)?.replace('\n', &format!("{newline}  "));
    let property = format!("{newline}  \"wilderness\": {encoded},");
    let mut output = source.to_owned();
    match property_start {
        Some(start) => output.replace_range(start..width_start, &property),
        None => output.insert_str(width_start, &property),
    }
    serde_json::from_str::<serde_json::Value>(&output)?;
    Ok(output)
}

fn invalid_wilderness_selection(message: impl Into<String>) -> LegacyImportError {
    LegacyImportError::InvalidDemoWildernessSelection(message.into())
}

fn selected_town_source_file(text: &str, source_index: u32) -> Option<&str> {
    let selector = format!("?:[EQU $TOWN {source_index}]");
    let mut selected = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with("?:") {
            selected = line == selector;
        } else if selected && let Some(path) = line.strip_prefix("%:") {
            return Some(path);
        }
    }
    None
}

fn town_feature_tags(text: &str) -> BTreeMap<char, String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("L:")?;
            let (symbol, tag) = rest.split_once(':')?;
            let mut chars = symbol.chars();
            let symbol = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            Some((symbol, tag.trim().to_owned()))
        })
        .collect()
}

fn source_has_town_symbol(text: &str, symbol: char) -> bool {
    text.lines()
        .filter_map(|line| line.trim_end().strip_prefix("M:"))
        .any(|row| row.contains(symbol))
}

fn dungeon_flag_number(record: &LegacyDungeonRecord, prefix: &str) -> Option<u32> {
    record
        .flags
        .iter()
        .find_map(|flag| flag.strip_prefix(prefix)?.parse().ok())
}

fn dungeon_final_object(record: &LegacyDungeonRecord) -> Option<DemoDungeonObjectPlan> {
    let value = record
        .flags
        .iter()
        .find_map(|flag| flag.strip_prefix("FINAL_OBJECT_"))?;
    let (tval, sval) = value.split_once('_')?;
    Some(DemoDungeonObjectPlan {
        tval: tval.parse().ok()?,
        sval: sval.parse().ok()?,
    })
}

fn validate_demo_wilderness_plans(
    source: &Path,
    source_commit: &str,
    selection: &DemoWildernessSelection,
    wilderness: &LegacyWilderness,
    dungeons: &BTreeMap<u32, LegacyDungeonRecord>,
) -> Result<(), LegacyImportError> {
    let t_info = read_legacy_object_at(source, source_commit, T_INFO_SOURCE)?;
    let t_pref = read_legacy_object_at(source, source_commit, T_PREF_SOURCE)?;
    let feature_tags = town_feature_tags(&t_pref);

    for town in &selection.town_plans {
        if !selection
            .towns
            .iter()
            .any(|selected| selected.source_index == town.source_index && selected.id == town.id)
        {
            return Err(invalid_wilderness_selection(format!(
                "town plan {} is not active",
                town.id
            )));
        }
        let location = wilderness.towns.get(&town.source_index).ok_or_else(|| {
            invalid_wilderness_selection(format!(
                "unknown town-plan wilderness index {}",
                town.source_index
            ))
        })?;
        if location.name != town.source_name
            || (location.x, location.y) != (town.position.x, town.position.y)
        {
            return Err(invalid_wilderness_selection(format!(
                "town plan {} source name or position drifted",
                town.id
            )));
        }
        let source_include = town
            .source_file
            .strip_prefix("lib/edit/")
            .unwrap_or(&town.source_file);
        if selected_town_source_file(&t_info, town.source_index) != Some(source_include) {
            return Err(invalid_wilderness_selection(format!(
                "town plan {} is not mapped to {} by {T_INFO_SOURCE}",
                town.id, town.source_file
            )));
        }
        let town_source = read_legacy_object_at(source, source_commit, &town.source_file)?;
        let mut symbols = BTreeSet::new();
        for facility in &town.standard_facilities {
            if !symbols.insert(facility.symbol)
                || feature_tags.get(&facility.symbol) != Some(&facility.source_tag)
                || !source_has_town_symbol(&town_source, facility.symbol)
            {
                return Err(invalid_wilderness_selection(format!(
                    "town plan {} facility {}:{} is absent or duplicated",
                    town.id, facility.symbol, facility.source_tag
                )));
            }
        }

        let inn = &town.inn;
        let inn_name = format!(
            "B:{}:N:{}:{}:{}",
            inn.building_index, inn.name, inn.owner_name, inn.owner_race
        );
        if !town_source.lines().any(|line| line.trim() == inn_name) {
            return Err(invalid_wilderness_selection(format!(
                "town plan {} inn identity drifted",
                town.id
            )));
        }
        let mut action_indexes = BTreeSet::new();
        for service in &inn.services {
            let service_line = format!(
                "B:{}:A:{}:{}:{}:{}:{}:{}:{}",
                inn.building_index,
                service.action_index,
                service.name,
                service.minimum_cost,
                service.maximum_cost,
                service.command,
                service.action_id,
                service.restriction
            );
            if !action_indexes.insert(service.action_index)
                || !town_source.lines().any(|line| line.trim() == service_line)
            {
                return Err(invalid_wilderness_selection(format!(
                    "town plan {} inn service {} is absent or duplicated",
                    town.id, service.action_index
                )));
            }
        }
        let access_line = format!("B:{}:R:*:{}", inn.building_index, inn.access);
        if !town_source.lines().any(|line| line.trim() == access_line) {
            return Err(invalid_wilderness_selection(format!(
                "town plan {} inn access drifted",
                town.id
            )));
        }
    }

    let monsters = parse_r_info(&read_legacy_object_at(
        source,
        source_commit,
        R_INFO_SOURCE,
    )?)?;
    let chinese_monster_names = parse_chinese_name_table(
        &read_legacy_object_at(source, source_commit, R_NAME_ZH_SOURCE)?,
        R_NAME_ZH_SOURCE,
    )?;
    for dungeon in &selection.dungeon_plans {
        let record = dungeons.get(&dungeon.source_index).ok_or_else(|| {
            invalid_wilderness_selection(format!(
                "unknown planned dungeon index {}",
                dungeon.source_index
            ))
        })?;
        if record.name != dungeon.source_name
            || record.position != Some((dungeon.position.x, dungeon.position.y))
            || record.minimum_depth != Some(dungeon.minimum_depth)
            || record.maximum_depth != Some(dungeon.maximum_depth)
            || dungeon_flag_number(record, "MONSTER_DIV_")
                != Some(u32::from(dungeon.monster_divisor))
        {
            return Err(invalid_wilderness_selection(format!(
                "planned dungeon {} identity, position, depth, or divisor drifted",
                dungeon.id
            )));
        }
        let source_generation_flags = record
            .flags
            .iter()
            .filter(|flag| {
                !flag.starts_with("FINAL_")
                    && !flag.starts_with("MONSTER_DIV_")
                    && !flag.starts_with("SUBSTITUTE_")
            })
            .cloned()
            .collect::<BTreeSet<_>>();
        if dungeon
            .generation_flags
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            != source_generation_flags
            || dungeon
                .monster_preferences
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                != record
                    .monster_preferences
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
        {
            return Err(invalid_wilderness_selection(format!(
                "planned dungeon {} generation or monster preferences drifted",
                dungeon.id
            )));
        }
        if dungeon_flag_number(record, "FINAL_GUARDIAN_") != Some(dungeon.guardian.source_index)
            || dungeon_final_object(record).as_ref() != Some(&dungeon.final_object)
            || dungeon_flag_number(record, "FINAL_EGO_") != Some(dungeon.final_ego_source_index)
            || dungeon_flag_number(record, "SUBSTITUTE_") != Some(dungeon.substitute_source_index)
        {
            return Err(invalid_wilderness_selection(format!(
                "planned dungeon {} guardian or final reward drifted",
                dungeon.id
            )));
        }
        let guardian = monsters
            .iter()
            .find(|monster| monster.index == dungeon.guardian.source_index)
            .ok_or_else(|| {
                invalid_wilderness_selection(format!(
                    "planned guardian index {} is absent",
                    dungeon.guardian.source_index
                ))
            })?;
        let guardian_chinese_name = chinese_monster_names
            .get(dungeon.guardian.source_index as usize)
            .and_then(Option::as_deref);
        if guardian.name != dungeon.guardian.source_name
            || guardian.level != Some(dungeon.guardian.level)
            || guardian_chinese_name != Some(&dungeon.guardian.chinese_name)
        {
            return Err(invalid_wilderness_selection(format!(
                "planned dungeon {} guardian identity drifted",
                dungeon.id
            )));
        }
    }

    Ok(())
}

pub fn sync_demo_wilderness(
    source: &Path,
    selection_path: &Path,
    output: &Path,
) -> Result<usize, LegacyImportError> {
    let canonical_source = source
        .canonicalize()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if output.starts_with(&canonical_source) {
        return Err(LegacyImportError::LegacyGit(
            "output file must live outside the legacy source".to_owned(),
        ));
    }
    let selection: DemoWildernessSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    if selection.schema_version != 4
        || selection.towns.is_empty()
        || selection.dungeons.is_empty()
        || selection.town_plans.is_empty()
        || selection.dungeon_plans.is_empty()
    {
        return Err(LegacyImportError::InvalidDemoWildernessSelection(
            "selection must use schemaVersion 4 and contain active towns, town plans, active dungeons, and dungeon plans"
                .to_owned(),
        ));
    }

    let source_commit = resolve_legacy_content_commit(source)?;
    let wilderness = parse_w_info(&read_legacy_object_at(
        source,
        &source_commit,
        W_INFO_SOURCE,
    )?)?;
    let dungeons = parse_dungeon_records(&read_legacy_object_at(
        source,
        &source_commit,
        D_INFO_SOURCE,
    )?)?;
    validate_demo_wilderness_plans(source, &source_commit, &selection, &wilderness, &dungeons)?;
    let world_source = fs::read_to_string(output)?;
    let world: serde_json::Value = serde_json::from_str(&world_source)?;
    if world.get("id").and_then(serde_json::Value::as_str) != Some(&selection.world_id) {
        return Err(LegacyImportError::InvalidDemoWildernessSelection(format!(
            "world output does not define {}",
            selection.world_id
        )));
    }
    let known_town_id = world.get("townId").and_then(serde_json::Value::as_str);
    let town_plan_ids = selection
        .town_plans
        .iter()
        .map(|town| town.id.as_str())
        .collect::<BTreeSet<_>>();
    let known_dungeon_ids = world
        .get("dungeons")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dungeon| dungeon.get("id").and_then(serde_json::Value::as_str))
        .collect::<BTreeSet<_>>();

    let mut selected_source_indexes = BTreeSet::new();
    let mut selected_ids = BTreeSet::new();
    let mut locations = Vec::with_capacity(selection.towns.len() + selection.dungeons.len());
    for selected in &selection.towns {
        if !selected_source_indexes.insert(("town", selected.source_index))
            || !selected_ids.insert(selected.id.as_str())
            || (known_town_id != Some(selected.id.as_str())
                && !town_plan_ids.contains(selected.id.as_str()))
        {
            return Err(LegacyImportError::InvalidDemoWildernessSelection(format!(
                "duplicate or unsupported town selection {}",
                selected.id
            )));
        }
        let location = wilderness
            .towns
            .get(&selected.source_index)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoWildernessSelection(format!(
                    "unknown wilderness town index {}",
                    selected.source_index
                ))
            })?;
        if location.name != selected.source_name {
            return Err(LegacyImportError::InvalidDemoWildernessSelection(format!(
                "wilderness town {} is {}, expected {}",
                selected.source_index, location.name, selected.source_name
            )));
        }
        let mut location_json = serde_json::json!({
            "kind": "town",
            "position": { "x": location.x, "y": location.y },
            "townId": selected.id,
        });
        if let Some(map_origin) = world
            .pointer("/wilderness/locations")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|candidate| {
                candidate.get("townId").and_then(serde_json::Value::as_str)
                    == Some(selected.id.as_str())
            })
            .and_then(|candidate| candidate.get("mapOrigin"))
        {
            location_json["mapOrigin"] = map_origin.clone();
        }
        locations.push(location_json);
    }
    for selected in &selection.dungeons {
        if !selected_source_indexes.insert(("dungeon", selected.source_index))
            || !selected_ids.insert(selected.id.as_str())
            || !known_dungeon_ids.contains(selected.id.as_str())
        {
            return Err(LegacyImportError::InvalidDemoWildernessSelection(format!(
                "duplicate or unsupported dungeon selection {}",
                selected.id
            )));
        }
        let record = dungeons.get(&selected.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoWildernessSelection(format!(
                "unknown positioned dungeon index {}",
                selected.source_index
            ))
        })?;
        let location = dungeon_location(record).ok_or_else(|| {
            LegacyImportError::InvalidDemoWildernessSelection(format!(
                "dungeon {} has no wilderness position",
                selected.source_index
            ))
        })?;
        if location.name != selected.source_name
            || location.x >= wilderness.width
            || location.y >= wilderness.height
        {
            return Err(LegacyImportError::InvalidDemoWildernessSelection(format!(
                "dungeon {} source name or position does not match the wilderness",
                selected.source_index
            )));
        }
        locations.push(serde_json::json!({
            "kind": "dungeon",
            "position": { "x": location.x, "y": location.y },
            "dungeonId": selected.id,
        }));
    }
    for planned in &selection.dungeon_plans {
        if !selection.dungeons.iter().any(|selected| {
            selected.source_index == planned.source_index && selected.id == planned.id
        }) || planned.position.x >= wilderness.width
            || planned.position.y >= wilderness.height
        {
            return Err(invalid_wilderness_selection(format!(
                "inactive or out-of-bounds dungeon plan {}",
                planned.id
            )));
        }
    }

    let legend = wilderness
        .legend
        .iter()
        .map(|entry| {
            serde_json::json!({
                "symbol": entry.symbol.to_string(),
                "terrain": wilderness_terrain_id(entry.terrain),
                "level": entry.level,
                "road": entry.road,
            })
        })
        .collect::<Vec<_>>();
    let wilderness_json = serde_json::json!({
        "width": wilderness.width,
        "height": wilderness.height,
        "startPosition": { "x": wilderness.start_x, "y": wilderness.start_y },
        "legend": legend,
        "rows": wilderness.rows,
        "locations": locations,
    });
    fs::write(
        output,
        replace_wilderness_property(&world_source, &wilderness_json)?,
    )?;
    Ok(selection.towns.len() + selection.dungeons.len())
}

const P62_POLYMORPH_RACES: &[(u16, &str, bool)] = &[
    (1, "tonberry", true),
    (3, "hobbit", true),
    (4, "gnome", true),
    (5, "dwarf", true),
    (6, "snotling", true),
    (7, "half-troll", true),
    (8, "amberite", true),
    (9, "high-elf", true),
    (10, "barbarian", true),
    (11, "ogre", true),
    (12, "half-giant", true),
    (13, "half-titan", true),
    (14, "cyclops", true),
    (15, "yeek", true),
    (16, "klackon", true),
    (17, "kobold", true),
    (18, "nibelung", true),
    (19, "dark-elf", true),
    (21, "mindflayer", true),
    (22, "imp", true),
    (23, "golem", true),
    (24, "skeleton", true),
    (25, "zombie", true),
    (26, "vampire", true),
    (27, "spectre", true),
    (28, "sprite", true),
    (29, "beastman", true),
    (30, "ent", true),
    (31, "archon", true),
    (32, "balrog", true),
    (33, "dunadan", true),
    (34, "shadow-fairy", true),
    (35, "kutar", true),
    (36, "android", false),
    (37, "doppelganger", true),
    (51, "centaur", true),
    (60, "wood-elf", true),
    (63, "half-orc", true),
    (64, "einheri", true),
    (67, "boit", true),
    (70, "tomte", true),
    (74, "maia", true),
    (1_007, "small-kobold", false),
    (1_008, "mangy-leper", false),
];

fn legacy_races_by_id(
    source: &Path,
    source_commit: &str,
) -> Result<BTreeMap<String, LegacyCharacterEntry>, LegacyImportError> {
    let source_objects = list_legacy_c_sources(source, source_commit)?
        .into_iter()
        .map(|path| {
            let text = read_legacy_object_at(source, source_commit, &path)?;
            Ok(text)
        })
        .collect::<Result<Vec<_>, LegacyImportError>>()?;
    let mut races = BTreeMap::new();
    for text in &source_objects {
        for (name, body) in extract_race_blocks(text) {
            let mut entry = parse_character_block(&name, &body);
            if let Some(hook) = entry.calc_bonuses_fn.clone() {
                let (resistances, free_act, see_invisible, attribute_sustains, speed) =
                    parse_calc_bonuses_defenses(text, &hook);
                entry.resistances = resistances;
                entry.free_act = free_act;
                entry.see_invisible = see_invisible;
                entry.attribute_sustains = attribute_sustains;
                entry.speed = speed;
            }
            parse_race_powers(text, &mut entry);
            races.entry(entry.id.clone()).or_insert(entry);
        }
    }
    Ok(races)
}

fn write_p62_locale_block(
    path: &Path,
    entries: &[(String, String, String)],
    chinese: bool,
) -> Result<(), LegacyImportError> {
    const START: &str = "# P62 polymorph forms (generated)";
    const END: &str = "# /P62 polymorph forms";
    let mut source = fs::read_to_string(path)?;
    if let Some(start) = source.find(START) {
        let end = source[start..]
            .find(END)
            .map(|offset| start + offset + END.len())
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "{} has an unterminated P62 locale block",
                    path.display()
                ))
            })?;
        source.replace_range(start..end, "");
    }
    source = source.trim_end().to_owned();
    source.push_str("\n\n");
    source.push_str(START);
    source.push('\n');
    for (id, name, description) in entries {
        let display_name = if chinese {
            name.clone()
        } else {
            id.split('-')
                .map(|part| {
                    let mut chars = part.chars();
                    chars.next().map_or_else(String::new, |first| {
                        first.to_uppercase().collect::<String>() + chars.as_str()
                    })
                })
                .collect::<Vec<_>>()
                .join(" ")
        };
        let display_description = if chinese && !description.is_empty() {
            description.replace('\n', "\n    ")
        } else if chinese {
            "RFB 临时变形形态。".to_owned()
        } else {
            "Temporary RFB polymorph form.".to_owned()
        };
        source.push_str(&format!(
            "race-legacy-{id}-name = {display_name}\nrace-legacy-{id}-description = {display_description}\n"
        ));
    }
    source.push_str(END);
    source.push('\n');
    fs::write(path, source)?;
    Ok(())
}

pub fn sync_demo_polymorph_races(
    source: &Path,
    pack_root: &Path,
) -> Result<usize, LegacyImportError> {
    let source_commit = resolve_legacy_content_commit(source)?;
    let races_by_id = legacy_races_by_id(source, &source_commit)?;
    let races_output = pack_root.join("races");
    let skill_sets_output = pack_root.join("skillSets");
    fs::create_dir_all(&races_output)?;
    fs::create_dir_all(&skill_sets_output)?;
    let standard_slots = serde_json::from_slice::<serde_json::Value>(&fs::read(
        races_output.join("rfb-human.json"),
    )?)?
    .get("bodySlots")
    .and_then(serde_json::Value::as_array)
    .ok_or_else(|| {
        LegacyImportError::InvalidDemoMonsterSelection(
            "demo human race has no standard body slots".to_owned(),
        )
    })?
    .iter()
    .map(|slot| {
        let id = slot.get("id").and_then(serde_json::Value::as_str);
        let slot_type = slot.get("slotType").and_then(serde_json::Value::as_str);
        id.zip(slot_type)
            .map(|(id, slot_type)| (id.to_owned(), slot_type.to_owned()))
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(
                    "demo human race has an invalid body slot".to_owned(),
                )
            })
    })
    .collect::<Result<Vec<_>, LegacyImportError>>()?;
    let mut locale_entries = Vec::with_capacity(P62_POLYMORPH_RACES.len());
    let mut report = ContentImportReport::default();
    for (legacy_index, id, random_candidate) in P62_POLYMORPH_RACES {
        let entry = races_by_id.get(*id).ok_or_else(|| {
            LegacyImportError::InvalidDemoMonsterSelection(format!("missing P62 race source {id}"))
        })?;
        if entry.dynamic || entry.name.is_empty() {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "P62 race source {id} is dynamic or unnamed"
            )));
        }
        let body_slots = if *id == "centaur" {
            standard_slots
                .iter()
                .filter(|(_, slot_type)| slot_type != "boots")
                .cloned()
                .collect::<Vec<_>>()
        } else {
            standard_slots.clone()
        };
        let mut race = race_json(entry, &body_slots, &mut report);
        race["legacyIndex"] = serde_json::json!(legacy_index);
        let mut tags = legacy_race_tags(entry);
        if *random_candidate {
            tags.push("polymorph-candidate");
        }
        if entry.flags.iter().any(|flag| flag == "RACE_NO_POLY") {
            tags.push("polymorph-immune");
        }
        race["tags"] = serde_json::json!(tags);
        fs::write(
            races_output.join(format!("{id}.json")),
            serde_json::to_string_pretty(&race)? + "\n",
        )?;
        let mut skill_set = character_skill_set_json(entry, &format!("race-{id}"));
        if let Some(entries) = skill_set
            .get_mut("entries")
            .and_then(serde_json::Value::as_array_mut)
        {
            for skill in entries {
                if let Some(skill_id) = skill.get_mut("skillId")
                    && let Some(id) = skill_id.as_str()
                {
                    *skill_id =
                        serde_json::json!(id.replacen("rfb-legacy.skill.", "demo.skill.", 1));
                }
            }
        }
        fs::write(
            skill_sets_output.join(format!("race-{id}.json")),
            serde_json::to_string_pretty(&skill_set)? + "\n",
        )?;
        locale_entries.push((
            (*id).to_owned(),
            entry.name.clone(),
            entry.description.clone(),
        ));
    }
    write_p62_locale_block(
        &pack_root
            .parent()
            .and_then(Path::parent)
            .unwrap_or(pack_root)
            .join("locales/en-US/content.ftl"),
        &locale_entries,
        false,
    )?;
    write_p62_locale_block(
        &pack_root
            .parent()
            .and_then(Path::parent)
            .unwrap_or(pack_root)
            .join("locales/zh-CN/content.ftl"),
        &locale_entries,
        true,
    )?;
    Ok(P62_POLYMORPH_RACES.len())
}

pub fn sync_demo_monsters(
    source: &Path,
    selection_path: &Path,
    output: &Path,
) -> Result<usize, LegacyImportError> {
    let canonical_source = source
        .canonicalize()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if output.starts_with(&canonical_source) {
        return Err(LegacyImportError::LegacyGit(
            "output directory must live outside the legacy source".to_owned(),
        ));
    }
    let selection: DemoMonsterSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    if selection.schema_version != 1 || selection.monsters.is_empty() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(
            "selection must use schemaVersion 1 and contain at least one monster".to_owned(),
        ));
    }
    let source_commit = resolve_legacy_content_commit(source)?;
    let entries = parse_r_info(&read_legacy_object_at(
        source,
        &source_commit,
        R_INFO_SOURCE,
    )?)?;
    let by_index = entries
        .iter()
        .filter(|entry| !entry.name.is_empty() && entry.name != "player" && entry.glyph.is_some())
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();
    let selected_source_indexes = selection
        .monsters
        .iter()
        .map(|monster| monster.source_index)
        .collect::<BTreeSet<_>>();
    let mut actor_ids_by_source_index = BTreeMap::new();
    for monster in &selection.monsters {
        by_index.get(&monster.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoMonsterSelection(format!(
                "unknown legacy source index {}",
                monster.source_index
            ))
        })?;
        if actor_ids_by_source_index
            .insert(monster.source_index, format!("demo.actor.{}", monster.id))
            .is_some()
        {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(
                "selected monster source indexes must be unique".to_owned(),
            ));
        }
    }
    if output.is_dir() {
        for file in fs::read_dir(output)? {
            let file = file?;
            let path = file.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| {
                    LegacyImportError::InvalidDemoMonsterSelection(
                        "actor file name is not valid UTF-8".to_owned(),
                    )
                })?;
            let actor: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
            let Some(actor_id) = actor["id"].as_str() else {
                continue;
            };
            let Some(source_index) = actor["allocation"]["legacyIndex"].as_u64() else {
                continue;
            };
            let Ok(source_index) = u32::try_from(source_index) else {
                continue;
            };
            if actor_id == format!("demo.actor.{stem}") {
                actor_ids_by_source_index
                    .entry(source_index)
                    .or_insert_with(|| actor_id.to_owned());
            }
        }
    }
    let mut deprecated_indexes = BTreeSet::new();
    let mut replacement_indexes = BTreeSet::new();
    for replacement in &selection.deprecated_replacements {
        if !deprecated_indexes.insert(replacement.deprecated_source_index)
            || !replacement_indexes.insert(replacement.replacement_source_index)
        {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(
                "deprecated replacement indexes must be unique".to_owned(),
            ));
        }
        let deprecated = by_index
            .get(&replacement.deprecated_source_index)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "unknown deprecated source index {}",
                    replacement.deprecated_source_index
                ))
            })?;
        let active = by_index
            .get(&replacement.replacement_source_index)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "unknown replacement source index {}",
                    replacement.replacement_source_index
                ))
            })?;
        if !deprecated.flags.iter().any(|flag| flag == "DEPRECATED")
            || active.flags.iter().any(|flag| flag == "DEPRECATED")
            || kebab(&deprecated.name) != kebab(&active.name)
            || selected_source_indexes.contains(&replacement.deprecated_source_index)
            || !selected_source_indexes.contains(&replacement.replacement_source_index)
        {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "invalid deprecated replacement {} -> {}",
                replacement.deprecated_source_index, replacement.replacement_source_index
            )));
        }
    }
    let mut selected_ids = BTreeSet::new();
    let mut selected_indexes = BTreeSet::new();
    let mut files = Vec::with_capacity(selection.monsters.len());
    let mut abilities = BTreeMap::new();
    for selected in selection.monsters {
        if !selected_ids.insert(selected.id.clone()) {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "duplicate monster id {}",
                selected.id
            )));
        }
        if !selected_indexes.insert(selected.source_index) {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "duplicate monster source index {}",
                selected.source_index
            )));
        }
        let entry = by_index.get(&selected.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoMonsterSelection(format!(
                "unknown legacy source index {}",
                selected.source_index
            ))
        })?;
        if entry.flags.iter().any(|flag| flag == "DEPRECATED") {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "selected source index {} is deprecated",
                selected.source_index
            )));
        }
        let actual_id = kebab(&entry.name);
        let expected_source_id = selected.expected_source_id();
        if actual_id != expected_source_id {
            return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
                "source index {} is {actual_id}, expected {expected_source_id}",
                selected.source_index
            )));
        }
        files.push((
            format!("{}.json", selected.id),
            demo_monster_json(entry, &selected, &mut abilities)?,
        ));
    }
    fs::create_dir_all(output)?;
    for (name, value) in &files {
        fs::write(
            output.join(name),
            serde_json::to_string_pretty(value)? + "\n",
        )?;
    }
    for entry in &entries {
        let Some(actor_id) = actor_ids_by_source_index.get(&entry.index) else {
            continue;
        };
        let actor_path = output.join(format!(
            "{}.json",
            actor_id
                .strip_prefix("demo.actor.")
                .expect("formal actor id must use the actor prefix")
        ));
        let mut actor: serde_json::Value = serde_json::from_slice(&fs::read(&actor_path)?)?;
        let before = actor.clone();
        actor
            .as_object_mut()
            .expect("actor JSON must be an object")
            .remove("evolution");
        if let (Some(required_experience), Some(target_index)) =
            (entry.evolution_experience, entry.evolution_target_index)
            && required_experience > 0
            && target_index > 0
        {
            by_index.get(&target_index).ok_or_else(|| {
                LegacyImportError::InvalidDemoMonsterSelection(format!(
                    "{actor_id} references unknown evolution target {target_index}"
                ))
            })?;
            if let Some(next_actor_kind_id) = actor_ids_by_source_index.get(&target_index) {
                actor["evolution"] = serde_json::json!({
                    "requiredExperience": required_experience,
                    "nextActorKindId": next_actor_kind_id,
                });
            }
        }
        if actor != before {
            fs::write(actor_path, serde_json::to_string_pretty(&actor)? + "\n")?;
        }
    }
    let pack_root = output.parent().ok_or_else(|| {
        LegacyImportError::InvalidDemoMonsterSelection(
            "actors output must have a pack parent directory".to_owned(),
        )
    })?;
    let mut ability_files = abilities
        .into_iter()
        .map(|(id, value)| {
            let stem = id
                .strip_prefix("rfb-legacy.ability.")
                .expect("generated monster ability id must use the legacy prefix");
            let mut name = format!("{stem}.json");
            let existing = pack_root.join("abilities").join(&name);
            if existing.is_file() {
                let existing: serde_json::Value = serde_json::from_slice(&fs::read(existing)?)?;
                if existing["id"].as_str() != Some(id.as_str()) {
                    name = format!("legacy-{stem}.json");
                }
            }
            Ok((name, value))
        })
        .collect::<Result<Vec<_>, LegacyImportError>>()?;
    let extracted =
        extract_ability_programs_and_player_bindings(&mut ability_files, &BTreeSet::new())
            .map_err(LegacyImportError::InvalidDemoMonsterSelection)?;
    for (directory, generated) in [
        ("abilities", ability_files),
        ("abilityPrograms", extracted.program_files),
    ] {
        let directory = pack_root.join(directory);
        fs::create_dir_all(&directory)?;
        for (name, value) in generated {
            fs::write(
                directory.join(name),
                serde_json::to_string_pretty(&value)? + "\n",
            )?;
        }
    }
    Ok(files.len())
}

pub fn audit_demo_monsters(
    source: &Path,
    selection_path: &Path,
    minimum_level: u16,
    maximum_level: u16,
) -> Result<DemoMonsterAuditReport, LegacyImportError> {
    if minimum_level > maximum_level {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(format!(
            "audit minimum level {minimum_level} exceeds maximum level {maximum_level}"
        )));
    }
    let selection: DemoMonsterSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    if selection.schema_version != 1 || selection.monsters.is_empty() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(
            "selection must use schemaVersion 1 and contain at least one monster".to_owned(),
        ));
    }
    let source_commit = resolve_legacy_content_commit(source)?;
    let monsters = parse_r_info(&read_legacy_object_at(
        source,
        &source_commit,
        R_INFO_SOURCE,
    )?)?;
    let chinese_names = parse_chinese_name_table(
        &read_legacy_object_at(source, &source_commit, R_NAME_ZH_SOURCE)?,
        R_NAME_ZH_SOURCE,
    )?;
    let selected = selection
        .monsters
        .iter()
        .map(|entry| (entry.source_index, entry))
        .collect::<BTreeMap<_, _>>();
    if selected.len() != selection.monsters.len() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(
            "monster source indexes must be unique".to_owned(),
        ));
    }
    let selected_ids = selection
        .monsters
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<BTreeSet<_>>();
    if selected_ids.len() != selection.monsters.len() {
        return Err(LegacyImportError::InvalidDemoMonsterSelection(
            "monster ids must be unique".to_owned(),
        ));
    }
    let mut entries = monsters
        .iter()
        .filter(|entry| {
            entry
                .level
                .is_some_and(|level| (minimum_level..=maximum_level).contains(&level))
                && entry.rarity.unwrap_or(0) > 0
                && entry.glyph.is_some()
                && !entry.flags.iter().any(|flag| flag == "DEPRECATED")
        })
        .map(|entry| {
            let level = entry.level.expect("audit range requires a level");
            let omitted_flags = demo_monster_omitted_flags(entry);
            let suggested_id = kebab(&entry.name);
            let location_restrictions = demo_monster_location_restrictions(entry);
            let location_eligible = location_restrictions.is_empty();

            let mut blockers = omitted_flags
                .iter()
                .filter(|flag| !demo_monster_audit_omission_is_safe(flag))
                .map(|flag| format!("flag:{flag}"))
                .collect::<Vec<_>>();
            if !selected.contains_key(&entry.index) && selected_ids.contains(suggested_id.as_str())
            {
                blockers.push(format!("id-collision:{suggested_id}"));
            }
            let audit_selection = DemoMonsterSelectionEntry {
                source_index: entry.index,
                source_id: None,
                id: suggested_id.clone(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: omitted_flags.iter().cloned().collect(),
                omitted_spells: Vec::new(),
            };
            let mut abilities = BTreeMap::new();
            if let Err(error) = demo_monster_json(entry, &audit_selection, &mut abilities) {
                let detail = match error {
                    LegacyImportError::InvalidDemoMonsterSelection(detail) => detail
                        .strip_prefix(&format!("{suggested_id} "))
                        .unwrap_or(&detail)
                        .to_owned(),
                    other => other.to_string(),
                };
                blockers.push(detail);
            }
            blockers.sort();
            blockers.dedup();

            let status = demo_monster_audit_status(
                selected.contains_key(&entry.index),
                entry.index,
                location_eligible,
                !blockers.is_empty(),
            );
            let mut suggested_tags = vec!["orc-cave".to_owned()];
            match entry.glyph {
                Some('a') => suggested_tags.push("ant".to_owned()),
                Some('S') => suggested_tags.push("spider".to_owned()),
                _ => {}
            }
            suggested_tags.sort();

            Ok(DemoMonsterAuditEntry {
                source_index: entry.index,
                source_name: entry.name.clone(),
                source_chinese_name: chinese_names
                    .get(entry.index as usize)
                    .and_then(Option::as_deref)
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        LegacyImportError::InvalidDemoMonsterSelection(format!(
                            "source index {} ({}) has no authoritative Chinese name",
                            entry.index, entry.name
                        ))
                    })?
                    .to_owned(),
                level,
                imported: selected.contains_key(&entry.index),
                location_eligible,
                location_restrictions,
                status,
                blockers,
                suggested_id,
                suggested_tags,
                omitted_flags: omitted_flags.into_iter().collect(),
            })
        })
        .collect::<Result<Vec<_>, LegacyImportError>>()?;
    entries.sort_by_key(|entry| (entry.level, entry.source_index));

    let count = |status| {
        entries
            .iter()
            .filter(|entry| entry.status == status)
            .count()
    };
    Ok(DemoMonsterAuditReport {
        schema_version: 1,
        source_ref: LEGACY_CONTENT_REFERENCE,
        source_commit,
        minimum_level,
        maximum_level,
        record_count: entries.len(),
        imported_count: entries.iter().filter(|entry| entry.imported).count(),
        selected_count: count(DemoMonsterAuditStatus::Selected),
        direct_count: count(DemoMonsterAuditStatus::Direct),
        blocked_count: count(DemoMonsterAuditStatus::Blocked),
        excluded_count: count(DemoMonsterAuditStatus::Excluded),
        guardian_count: count(DemoMonsterAuditStatus::Guardian),
        entries,
    })
}

fn item_coverage_system_blocker(tval: u16) -> Option<&'static str> {
    match tval {
        1 => Some("remains-system"),
        2 => Some("empty-container-system"),
        3 => Some("misc-tool-system"),
        4 => Some("instrument-system"),
        5 => Some("spike-system"),
        7 => Some("chest-system"),
        8 => Some("figurine-system"),
        9 => Some("statue-system"),
        10 => Some("corpse-system"),
        39 => Some("light-source-profile"),
        50 => Some("card-system"),
        68 => Some("device-book-system"),
        77 => Some("fuel-refill"),
        81 => Some("rune-system"),
        127 => Some("gold-wallet-model"),
        _ => None,
    }
}

fn item_coverage_blockers(
    entry: &LegacyItemEntry,
    ammo: &LauncherAmmoIndex,
    terrain_creation: &TerrainCreationImportIds,
) -> Vec<String> {
    let mut report = ContentImportReport::default();
    item_json_with_terrain(
        entry,
        &kebab(&entry.name),
        ammo,
        player_ability_book_for_item(entry),
        Some(terrain_creation),
        &mut report,
    );
    let mut blockers = report.item_behavior_gaps.into_keys().collect::<Vec<_>>();
    blockers.extend(
        report
            .unmapped_item_flags
            .into_keys()
            .filter(|flag| {
                !matches!(
                    flag.as_str(),
                    "TOWN" | "NO_SHUFFLE" | "FIXED_FLAVOR" | "PLURAL"
                )
            })
            .map(|flag| format!("item-flag:{flag}")),
    );
    if blockers.is_empty()
        && let Some(blocker) = item_coverage_system_blocker(entry.tval)
    {
        blockers.push(blocker.to_owned());
    }
    blockers.sort();
    blockers.dedup();
    blockers
}

fn formal_item_ids(items_dir: &Path) -> Result<BTreeSet<String>, LegacyImportError> {
    if !items_dir.is_dir() {
        return Err(LegacyImportError::InvalidDemoItemAudit(format!(
            "formal items directory does not exist: {}",
            items_dir.display()
        )));
    }
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(items_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        let id = value["id"].as_str().ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "formal item has no string id: {}",
                path.display()
            ))
        })?;
        if !ids.insert(id.to_owned()) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "duplicate formal item id {id}"
            )));
        }
    }
    Ok(ids)
}

const DEMO_ITEM_ACTIVE_REQUIREMENTS: [&str; 5] = [
    "behavior",
    "authoritative-zh-name",
    "source-identity",
    "flavor",
    "acquisition",
];

fn count_delta(current: usize, baseline: usize) -> i64 {
    current as i64 - baseline as i64
}

#[allow(clippy::too_many_arguments)]
fn build_demo_item_plan_progress(
    source_commit: &str,
    plan: &DemoItemPlan,
    by_index: &BTreeMap<u32, &LegacyItemEntry>,
    active_sources: &BTreeSet<u32>,
    mechanics_ready: &[DemoItemCoverageEntry],
    blocked: &[DemoItemCoverageEntry],
    formal_item_ids_by_source: &BTreeMap<u32, BTreeSet<String>>,
    source_items_total: usize,
    formal_items_total: usize,
    mapped_formal_items: usize,
    original_formal_items: usize,
) -> Result<DemoItemPlanProgress, LegacyImportError> {
    if plan.schema_version != 1 || plan.batches.is_empty() {
        return Err(LegacyImportError::InvalidDemoItemAudit(
            "P3 plan must use schemaVersion 1 and contain at least one batch".to_owned(),
        ));
    }
    let baseline = &plan.baseline;
    if baseline.source_commit.trim().is_empty()
        || baseline.source_items_total
            != baseline.active_source_items
                + baseline.mechanics_ready_source_items
                + baseline.blocked_source_items
        || baseline.formal_items_total
            != baseline.mapped_formal_items + baseline.original_formal_items
    {
        return Err(LegacyImportError::InvalidDemoItemAudit(
            "P3 plan baseline totals are inconsistent".to_owned(),
        ));
    }

    let mechanics_ready_by_index = mechanics_ready
        .iter()
        .map(|entry| (entry.source_index, entry))
        .collect::<BTreeMap<_, _>>();
    let blocked_by_index = blocked
        .iter()
        .map(|entry| (entry.source_index, entry))
        .collect::<BTreeMap<_, _>>();
    let required = DEMO_ITEM_ACTIVE_REQUIREMENTS
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut baseline_matches_current = source_commit == baseline.source_commit
        && source_items_total == baseline.source_items_total
        && active_sources.len() == baseline.active_source_items
        && mechanics_ready.len() == baseline.mechanics_ready_source_items
        && blocked.len() == baseline.blocked_source_items
        && formal_items_total == baseline.formal_items_total
        && mapped_formal_items == baseline.mapped_formal_items
        && original_formal_items == baseline.original_formal_items;
    let mut batch_ids = BTreeSet::new();
    let mut planned_sources = BTreeSet::new();
    let mut batches = Vec::new();

    for batch in &plan.batches {
        if batch.id.trim().is_empty()
            || !batch_ids.insert(batch.id.as_str())
            || batch.families.is_empty()
        {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "P3 plan has an empty or duplicate batch {}",
                batch.id
            )));
        }
        let mut family_ids = BTreeSet::new();
        let mut rule_families = Vec::new();
        let mut new_rfb_formal_item_ids = BTreeSet::new();
        let mut blocked_to_active = Vec::new();
        let mut blocked_to_mechanics_ready = Vec::new();
        let mut still_blocked = Vec::new();
        let mut unresolved_secondary_blockers = BTreeMap::new();
        let mut batch_item_count = 0;

        for family in &batch.families {
            if family.id.trim().is_empty()
                || !family_ids.insert(family.id.as_str())
                || family.primary_blockers.is_empty()
                || family.items.is_empty()
            {
                return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                    "P3 batch {} has an empty or duplicate rule family {}",
                    batch.id, family.id
                )));
            }
            let primary_blockers = family
                .primary_blockers
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if primary_blockers.len() != family.primary_blockers.len()
                || primary_blockers.iter().any(|blocker| blocker.is_empty())
            {
                return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                    "P3 rule family {} has empty or duplicate primary blockers",
                    family.id
                )));
            }
            rule_families.push(family.id.clone());

            for item in &family.items {
                batch_item_count += 1;
                if !planned_sources.insert(item.source_index) {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "P3 source index {} is planned more than once",
                        item.source_index
                    )));
                }
                let source = by_index.get(&item.source_index).ok_or_else(|| {
                    LegacyImportError::InvalidDemoItemAudit(format!(
                        "P3 plan names unknown source index {}",
                        item.source_index
                    ))
                })?;
                if source.name != item.source_name || kebab(&source.name) != item.source_id {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "P3 source index {} does not match name/id {} / {}",
                        item.source_index, item.source_name, item.source_id
                    )));
                }
                let secondary_blockers = item
                    .secondary_blockers
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                if secondary_blockers.len() != item.secondary_blockers.len()
                    || secondary_blockers.iter().any(|blocker| blocker.is_empty())
                    || !primary_blockers.is_disjoint(&secondary_blockers)
                {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "P3 source index {} has invalid secondary blockers",
                        item.source_index
                    )));
                }
                let completed_requirements = item
                    .completed_requirements
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>();
                if completed_requirements.len() != item.completed_requirements.len()
                    || !completed_requirements.is_subset(&required)
                {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "P3 source index {} has unknown or duplicate completion requirements",
                        item.source_index
                    )));
                }

                let (blockers, progress_target) = if active_sources.contains(&item.source_index) {
                    if completed_requirements != required {
                        let missing = required
                            .difference(&completed_requirements)
                            .copied()
                            .collect::<Vec<_>>()
                            .join(", ");
                        return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                            "active P3 source index {} is missing completion requirements: {missing}",
                            item.source_index
                        )));
                    }
                    let ids = formal_item_ids_by_source
                        .get(&item.source_index)
                        .ok_or_else(|| {
                            LegacyImportError::InvalidDemoItemAudit(format!(
                                "active P3 source index {} has no mapped formal item",
                                item.source_index
                            ))
                        })?;
                    new_rfb_formal_item_ids.extend(ids.iter().cloned());
                    (Vec::new(), &mut blocked_to_active)
                } else if mechanics_ready_by_index.contains_key(&item.source_index) {
                    (Vec::new(), &mut blocked_to_mechanics_ready)
                } else {
                    let blockers = blocked_by_index
                        .get(&item.source_index)
                        .expect("coverage partitions every source")
                        .blockers
                        .clone();
                    (blockers, &mut still_blocked)
                };
                let unresolved = blockers
                    .iter()
                    .filter(|blocker| !primary_blockers.contains(blocker.as_str()))
                    .cloned()
                    .collect::<Vec<_>>();
                for blocker in &unresolved {
                    *unresolved_secondary_blockers
                        .entry(blocker.clone())
                        .or_default() += 1;
                }
                let expected = primary_blockers
                    .union(&secondary_blockers)
                    .copied()
                    .collect::<BTreeSet<_>>();
                let actual = blockers.iter().map(String::as_str).collect::<BTreeSet<_>>();
                if actual != expected {
                    baseline_matches_current = false;
                }
                progress_target.push(DemoItemPlanProgressEntry {
                    source_index: item.source_index,
                    source_name: item.source_name.clone(),
                    source_id: item.source_id.clone(),
                    rule_family: family.id.clone(),
                    blockers,
                    unresolved_secondary_blockers: unresolved,
                });
            }
        }
        batches.push(DemoItemPlanBatchProgress {
            id: batch.id.clone(),
            rule_families,
            planned_source_items: batch_item_count,
            new_rfb_formal_items: new_rfb_formal_item_ids.len(),
            new_rfb_formal_item_ids: new_rfb_formal_item_ids.into_iter().collect(),
            blocked_to_active,
            blocked_to_mechanics_ready,
            still_blocked,
            unresolved_secondary_blockers,
        });
    }

    Ok(DemoItemPlanProgress {
        baseline_source_commit: baseline.source_commit.clone(),
        baseline_matches_current,
        formal_items_delta: count_delta(formal_items_total, baseline.formal_items_total),
        mapped_rfb_formal_items_delta: count_delta(
            mapped_formal_items,
            baseline.mapped_formal_items,
        ),
        original_formal_items_delta: count_delta(
            original_formal_items,
            baseline.original_formal_items,
        ),
        active_requirements: DEMO_ITEM_ACTIVE_REQUIREMENTS
            .into_iter()
            .map(str::to_owned)
            .collect(),
        planned_source_items: planned_sources.len(),
        batches,
    })
}

fn build_demo_item_coverage_report(
    source_commit: &str,
    entries: &[LegacyItemEntry],
    selection: &DemoItemSelection,
    adaptations: &DemoItemAdaptationLedger,
    plan: &DemoItemPlan,
    formal_ids: &BTreeSet<String>,
    terrain_creation: &TerrainCreationImportIds,
) -> Result<DemoItemCoverageReport, LegacyImportError> {
    if selection.schema_version != 1 || selection.items.is_empty() {
        return Err(LegacyImportError::InvalidDemoItemAudit(
            "selection must use schemaVersion 1 and contain at least one item".to_owned(),
        ));
    }
    if adaptations.schema_version != 1 {
        return Err(LegacyImportError::InvalidDemoItemAudit(
            "adaptation ledger must use schemaVersion 1".to_owned(),
        ));
    }
    let by_index = entries
        .iter()
        .filter(|entry| {
            !entry.name.is_empty() && entry.name != "something" && entry.glyph.is_some()
        })
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();
    let mut active_sources = BTreeSet::new();
    let mut mapped_formal_ids = BTreeSet::new();
    let mut formal_item_ids_by_source = BTreeMap::<u32, BTreeSet<String>>::new();
    let mut selected_ids = BTreeSet::new();
    for selected in &selection.items {
        if !active_sources.insert(selected.source_index) || !selected_ids.insert(&selected.id) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "duplicate selected item {} or source index {}",
                selected.id, selected.source_index
            )));
        }
        let source = by_index.get(&selected.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "unknown selected source index {}",
                selected.source_index
            ))
        })?;
        let source_id = kebab(&source.name);
        if source_id != selected.expected_source_id() {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "selected source index {} is {source_id}, expected {}",
                selected.source_index,
                selected.expected_source_id()
            )));
        }
        let item_id = format!("demo.item.{}", selected.id);
        if !formal_ids.contains(&item_id) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "selected formal item {item_id} does not exist"
            )));
        }
        formal_item_ids_by_source
            .entry(selected.source_index)
            .or_default()
            .insert(item_id.clone());
        mapped_formal_ids.insert(item_id);
    }

    let selected_sources = active_sources.clone();
    let mut adaptation_item_ids = BTreeSet::new();
    let mut adaptation_statuses = BTreeMap::new();
    let mut explicit_blockers = BTreeMap::new();
    for adaptation in &adaptations.items {
        if selected_sources.contains(&adaptation.source_index) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "source index {} appears in both selection and adaptation ledger",
                adaptation.source_index
            )));
        }
        if !adaptation_item_ids.insert(&adaptation.item_id) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "duplicate adaptation item id {}",
                adaptation.item_id
            )));
        }
        let source = by_index.get(&adaptation.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "unknown adaptation source index {}",
                adaptation.source_index
            ))
        })?;
        if source.name != adaptation.source_name || kebab(&source.name) != adaptation.source_id {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "adaptation source index {} does not match name/id {} / {}",
                adaptation.source_index, adaptation.source_name, adaptation.source_id
            )));
        }
        if adaptation.contract.trim().is_empty() {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "adaptation {} has no contract",
                adaptation.item_id
            )));
        }
        if adaptation
            .adaptation
            .as_deref()
            .is_some_and(|note| note.trim().is_empty())
        {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "adaptation {} has an empty adaptation note",
                adaptation.item_id
            )));
        }
        if let Some(previous) =
            adaptation_statuses.insert(adaptation.source_index, adaptation.status)
            && previous != adaptation.status
        {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "source index {} has conflicting adaptation statuses",
                adaptation.source_index
            )));
        }
        match adaptation.status {
            DemoItemCoverageStatus::Active => {
                if adaptation.blocker.is_some() || !formal_ids.contains(&adaptation.item_id) {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "active adaptation {} must exist and have no blocker",
                        adaptation.item_id
                    )));
                }
                active_sources.insert(adaptation.source_index);
                formal_item_ids_by_source
                    .entry(adaptation.source_index)
                    .or_default()
                    .insert(adaptation.item_id.clone());
                mapped_formal_ids.insert(adaptation.item_id.clone());
            }
            DemoItemCoverageStatus::MechanicsReady => {
                if adaptation.blocker.is_some() || formal_ids.contains(&adaptation.item_id) {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "mechanics-ready adaptation {} must be absent and have no blocker",
                        adaptation.item_id
                    )));
                }
            }
            DemoItemCoverageStatus::Blocked => {
                let blocker = adaptation
                    .blocker
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        LegacyImportError::InvalidDemoItemAudit(format!(
                            "blocked adaptation {} must name a blocker",
                            adaptation.item_id
                        ))
                    })?;
                if formal_ids.contains(&adaptation.item_id) {
                    return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                        "blocked adaptation {} must not exist in the formal pack",
                        adaptation.item_id
                    )));
                }
                explicit_blockers.insert(adaptation.source_index, blocker.to_owned());
            }
        }
    }

    let ammo = launcher_ammo_index(entries);
    let mut mechanics_ready = Vec::new();
    let mut blocked = Vec::new();
    let mut blocker_counts = BTreeMap::new();
    for entry in by_index.values() {
        if active_sources.contains(&entry.index) {
            continue;
        }
        let blockers = match adaptation_statuses.get(&entry.index) {
            Some(DemoItemCoverageStatus::MechanicsReady) => Vec::new(),
            Some(DemoItemCoverageStatus::Blocked) => vec![explicit_blockers[&entry.index].clone()],
            Some(DemoItemCoverageStatus::Active) => unreachable!("active sources were skipped"),
            None => item_coverage_blockers(entry, &ammo, terrain_creation),
        };
        let coverage = DemoItemCoverageEntry {
            source_index: entry.index,
            source_name: entry.name.clone(),
            source_id: kebab(&entry.name),
            blockers: blockers.clone(),
        };
        if blockers.is_empty() {
            mechanics_ready.push(coverage);
        } else {
            for blocker in blockers {
                *blocker_counts.entry(blocker).or_default() += 1;
            }
            blocked.push(coverage);
        }
    }
    mechanics_ready.sort_by_key(|entry| entry.source_index);
    blocked.sort_by_key(|entry| entry.source_index);
    let original_item_ids = formal_ids
        .difference(&mapped_formal_ids)
        .cloned()
        .collect::<Vec<_>>();
    let p3_plan = build_demo_item_plan_progress(
        source_commit,
        plan,
        &by_index,
        &active_sources,
        &mechanics_ready,
        &blocked,
        &formal_item_ids_by_source,
        by_index.len(),
        formal_ids.len(),
        mapped_formal_ids.len(),
        original_item_ids.len(),
    )?;
    let report = DemoItemCoverageReport {
        schema_version: 1,
        source_commit: source_commit.to_owned(),
        source_items_total: by_index.len(),
        active_source_items: active_sources.len(),
        mechanics_ready_source_items: mechanics_ready.len(),
        blocked_source_items: blocked.len(),
        formal_items_total: formal_ids.len(),
        mapped_formal_items: mapped_formal_ids.len(),
        original_formal_items: original_item_ids.len(),
        blocker_counts,
        mechanics_ready,
        blocked,
        original_item_ids,
        p3_plan,
    };
    debug_assert_eq!(
        report.source_items_total,
        report.active_source_items
            + report.mechanics_ready_source_items
            + report.blocked_source_items
    );
    Ok(report)
}

pub fn audit_demo_items(
    source: &Path,
    selection_path: &Path,
    adaptations_path: &Path,
    plan_path: &Path,
    items_dir: &Path,
) -> Result<DemoItemCoverageReport, LegacyImportError> {
    let source_commit = resolve_legacy_content_commit(source)?;
    let terrain = parse_f_info(&read_legacy_object_at(
        source,
        &source_commit,
        F_INFO_SOURCE,
    )?)?;
    let entries = parse_k_info(&read_legacy_object_at(
        source,
        &source_commit,
        K_INFO_SOURCE,
    )?)?;
    let selection: DemoItemSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    let adaptations: DemoItemAdaptationLedger =
        serde_json::from_slice(&fs::read(adaptations_path)?)?;
    let plan: DemoItemPlan = serde_json::from_slice(&fs::read(plan_path)?)?;
    build_demo_item_coverage_report(
        &source_commit,
        &entries,
        &selection,
        &adaptations,
        &plan,
        &formal_item_ids(items_dir)?,
        &terrain_creation_import_ids(&terrain),
    )
}

fn proficiency_rank_value(rank: u8) -> u16 {
    [0, 4_000, 6_000, 7_000, 8_000][usize::from(rank)]
}

/// Checks the formal class profiles against authoritative `master:s_info.txt`.
pub fn audit_demo_weapon_proficiencies(
    source: &Path,
    selection_path: &Path,
    adaptations_path: &Path,
    classes_dir: &Path,
) -> Result<DemoWeaponProficiencyAuditReport, LegacyImportError> {
    let source_commit = resolve_legacy_content_commit(source)?;
    let items = parse_k_info(&read_legacy_object_at(
        source,
        &source_commit,
        K_INFO_SOURCE,
    )?)?;
    let profiles = parse_s_info(&read_legacy_object_at(
        source,
        &source_commit,
        S_INFO_SOURCE,
    )?)?;
    let selection: DemoItemSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    let adaptations: DemoItemAdaptationLedger =
        serde_json::from_slice(&fs::read(adaptations_path)?)?;
    if selection.schema_version != 1 || adaptations.schema_version != 1 {
        return Err(LegacyImportError::InvalidDemoItemAudit(
            "weapon proficiency audit requires schemaVersion 1 ledgers".to_owned(),
        ));
    }

    let items_by_index = items
        .iter()
        .map(|item| (item.index, item))
        .collect::<BTreeMap<_, _>>();
    let mut base_weapons = BTreeMap::new();
    for entry in &selection.items {
        let item = items_by_index.get(&entry.source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "selected source item {} is missing",
                entry.source_index
            ))
        })?;
        if (19..=23).contains(&item.tval) {
            base_weapons.insert(format!("demo.item.{}", entry.id), *item);
        }
    }
    for entry in adaptations
        .items
        .iter()
        .filter(|entry| entry.status == DemoItemCoverageStatus::Active)
    {
        let Some(item) = items_by_index.get(&entry.source_index) else {
            continue;
        };
        if (19..=23).contains(&item.tval) {
            base_weapons.insert(entry.item_id.clone(), *item);
        }
    }

    for (file_name, class_index) in [
        ("warrior.json", 0),
        ("paladin.json", 5),
        ("high-mage.json", 10),
        ("archer.json", 15),
        ("cavalry.json", 22),
        ("sniper.json", 27),
    ] {
        let class: serde_json::Value =
            serde_json::from_slice(&fs::read(classes_dir.join(file_name))?)?;
        let proficiency = &class["weaponProficiency"];
        let default_initial = proficiency["defaultInitial"].as_u64().ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "{file_name} has no weapon proficiency default initial"
            ))
        })? as u16;
        let default_maximum = proficiency["defaultMaximum"].as_u64().ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "{file_name} has no weapon proficiency default maximum"
            ))
        })? as u16;
        let overrides = proficiency["overrides"].as_object().ok_or_else(|| {
            LegacyImportError::InvalidDemoItemAudit(format!(
                "{file_name} has no weapon proficiency overrides"
            ))
        })?;
        let source_profile = profiles
            .iter()
            .find(|profile| profile.class_index == class_index)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemAudit(format!(
                    "s_info has no class {class_index} profile"
                ))
            })?;
        let source_riding = source_profile
            .skill_entries
            .iter()
            .find(|entry| entry.skill_index == 2)
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemAudit(format!(
                    "s_info class {class_index} has no riding proficiency row"
                ))
            })?;
        let riding = &class["ridingProficiency"];
        let actual_riding = (
            riding["initial"].as_u64().map(|value| value as u16),
            riding["maximum"].as_u64().map(|value| value as u16),
        );
        if actual_riding != (Some(source_riding.initial), Some(source_riding.maximum)) {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "{file_name} riding proficiency is {:?}/{:?}, expected {}/{}",
                actual_riding.0, actual_riding.1, source_riding.initial, source_riding.maximum
            )));
        }

        for (item_id, item) in &base_weapons {
            let source_row = source_profile
                .weapon_entries
                .iter()
                .find(|row| row.weapon_type == item.tval - 19 && row.weapon_subtype == item.sval)
                .ok_or_else(|| {
                    LegacyImportError::InvalidDemoItemAudit(format!(
                        "s_info class {class_index} has no row for {item_id}"
                    ))
                })?;
            let expected_maximum = proficiency_rank_value(source_row.maximum_rank);
            let expected_initial = if source_row.initial_rank == 0 {
                2_000.min(expected_maximum)
            } else {
                proficiency_rank_value(source_row.initial_rank).min(expected_maximum)
            };
            let actual = overrides.get(item_id);
            let actual_initial = actual
                .and_then(|value| value["initial"].as_u64())
                .map_or(default_initial, |value| value as u16);
            let actual_maximum = actual
                .and_then(|value| value["maximum"].as_u64())
                .map_or(default_maximum, |value| value as u16);
            if (actual_initial, actual_maximum) != (expected_initial, expected_maximum) {
                return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                    "{file_name} {item_id} is {actual_initial}/{actual_maximum}, expected {expected_initial}/{expected_maximum}"
                )));
            }
        }
    }

    Ok(DemoWeaponProficiencyAuditReport {
        schema_version: 1,
        source_commit,
        classes_checked: 6,
        base_weapons_checked: base_weapons.len(),
    })
}

fn selected_demo_items<'a>(
    selection: &'a DemoItemSelection,
    entries: &'a [LegacyItemEntry],
) -> Result<Vec<(&'a DemoItemSelectionEntry, &'a LegacyItemEntry)>, LegacyImportError> {
    if selection.schema_version != 1 || selection.items.is_empty() {
        return Err(LegacyImportError::InvalidDemoItemSelection(
            "selection must use schemaVersion 1 and contain at least one item".to_owned(),
        ));
    }
    let by_index = entries
        .iter()
        .filter(|entry| {
            !entry.name.is_empty() && entry.name != "something" && entry.glyph.is_some()
        })
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();
    let mut selected = BTreeSet::new();
    selection
        .items
        .iter()
        .map(|selected_entry| {
            if !selected.insert(selected_entry.id.clone()) {
                return Err(LegacyImportError::InvalidDemoItemSelection(format!(
                    "duplicate item {}",
                    selected_entry.id
                )));
            }
            let entry = by_index.get(&selected_entry.source_index).ok_or_else(|| {
                LegacyImportError::InvalidDemoItemSelection(format!(
                    "unknown legacy source index {}",
                    selected_entry.source_index
                ))
            })?;
            let actual_id = kebab(&entry.name);
            let expected_source_id = selected_entry.expected_source_id();
            if actual_id != expected_source_id {
                return Err(LegacyImportError::InvalidDemoItemSelection(format!(
                    "source index {} is {actual_id}, expected {expected_source_id}",
                    selected_entry.source_index
                )));
            }
            Ok((selected_entry, *entry))
        })
        .collect()
}

fn parse_chinese_name_table(
    text: &str,
    source: &'static str,
) -> Result<Vec<Option<String>>, LegacyImportError> {
    let mut names = Vec::new();
    let mut in_array = false;
    let mut closed = false;
    for (offset, line) in text.lines().enumerate() {
        let line_number = offset + 1;
        let line = line.trim();
        if !in_array {
            in_array = line == "{";
            continue;
        }
        if line == "};" {
            closed = true;
            break;
        }
        let value = line.strip_suffix(',').unwrap_or(line);
        if value == "NULL" {
            names.push(None);
        } else if value.starts_with('"') {
            names.push(Some(serde_json::from_str(value).map_err(|error| {
                content_parse_error(source, line_number, "name", value, error.to_string())
            })?));
        }
    }
    if !closed || names.is_empty() {
        return Err(LegacyImportError::InvalidDemoItemAudit(format!(
            "{source} does not contain a complete Chinese name array"
        )));
    }
    Ok(names)
}

fn singular_chinese_kind_name(template: &str) -> String {
    let template = template.trim();
    let name = template
        .strip_prefix("& ")
        .and_then(|name| name.split_once('~').map(|(_, name)| name))
        .unwrap_or(template);
    name.replace('~', "").trim().to_owned()
}

fn singular_english_kind_name(template: &str) -> String {
    template
        .trim()
        .strip_prefix("& ")
        .unwrap_or(template.trim())
        .replace('~', "")
}

fn ftl_message_value<'a>(text: &'a str, key: &str) -> Result<&'a str, LegacyImportError> {
    let prefix = format!("{key} =");
    let mut matches = text
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix).map(str::trim));
    let value = matches.next().ok_or_else(|| {
        LegacyImportError::InvalidDemoItemAudit(format!("missing Chinese message {key}"))
    })?;
    if matches.next().is_some() {
        return Err(LegacyImportError::InvalidDemoItemAudit(format!(
            "duplicate Chinese message {key}"
        )));
    }
    Ok(value)
}

/// Verifies that formal demo items reuse the authoritative RFB master English
/// and Chinese names as their locale-specific Mogaminator matching sources.
pub fn audit_demo_item_names(
    source: &Path,
    selection_path: &Path,
    en_content_path: &Path,
    zh_content_path: &Path,
) -> Result<usize, LegacyImportError> {
    let source_commit = resolve_legacy_content_commit(source)?;
    let entries = parse_k_info(&read_legacy_object_at(
        source,
        &source_commit,
        K_INFO_SOURCE,
    )?)?;
    let chinese_names = parse_chinese_name_table(
        &read_legacy_object_at(source, &source_commit, K_NAME_ZH_SOURCE)?,
        K_NAME_ZH_SOURCE,
    )?;
    let selection: DemoItemSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    let selected = selected_demo_items(&selection, &entries)?;
    let en_content = fs::read_to_string(en_content_path)?;
    let zh_content = fs::read_to_string(zh_content_path)?;
    for (selected_entry, item) in &selected {
        let expected_en = singular_english_kind_name(&item.name);
        let expected_zh = chinese_names
            .get(selected_entry.source_index as usize)
            .and_then(Option::as_deref)
            .map(singular_chinese_kind_name)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemAudit(format!(
                    "source index {} ({}) has no authoritative Chinese name",
                    selected_entry.source_index, selected_entry.id
                ))
            })?;
        let key = format!("item-demo-{}-name", selected_entry.id);
        let actual_en = ftl_message_value(&en_content, &key)?;
        if actual_en != expected_en {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "en-US/{key} is {actual_en:?}, expected {expected_en:?} from {K_INFO_SOURCE}"
            )));
        }
        let actual_zh = ftl_message_value(&zh_content, &key)?;
        if actual_zh != expected_zh {
            return Err(LegacyImportError::InvalidDemoItemAudit(format!(
                "zh-CN/{key} is {actual_zh:?}, expected {expected_zh:?} from {K_NAME_ZH_SOURCE}"
            )));
        }
    }
    Ok(selected.len())
}

pub fn sync_demo_items(
    source: &Path,
    selection_path: &Path,
    output: &Path,
) -> Result<usize, LegacyImportError> {
    let canonical_source = source
        .canonicalize()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if output.starts_with(&canonical_source) {
        return Err(LegacyImportError::LegacyGit(
            "output directory must live outside the legacy source".to_owned(),
        ));
    }
    let selection: DemoItemSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    let source_commit = resolve_legacy_content_commit(source)?;
    let entries = parse_k_info(&read_legacy_object_at(
        source,
        &source_commit,
        K_INFO_SOURCE,
    )?)?;
    let ammo = launcher_ammo_index(&entries);
    let selected = selected_demo_items(&selection, &entries)?;
    let mut files = Vec::with_capacity(selection.items.len());
    for (selected_entry, entry) in selected {
        files.push((
            format!("{}.json", selected_entry.id),
            demo_item_json(entry, &selected_entry.id, &ammo)?,
        ));
    }
    fs::create_dir_all(output)?;
    for (name, value) in &files {
        fs::write(
            output.join(name),
            serde_json::to_string_pretty(value)? + "\n",
        )?;
    }
    Ok(files.len())
}

pub fn sync_demo_item_destruction(
    source: &Path,
    selection_path: &Path,
    adaptations_path: &Path,
    items_path: &Path,
) -> Result<usize, LegacyImportError> {
    let selection: DemoItemSelection = serde_json::from_slice(&fs::read(selection_path)?)?;
    let adaptations: DemoItemAdaptationLedger =
        serde_json::from_slice(&fs::read(adaptations_path)?)?;
    let source_commit = resolve_legacy_content_commit(source)?;
    let entries = parse_k_info(&read_legacy_object_at(
        source,
        &source_commit,
        K_INFO_SOURCE,
    )?)?;
    let entries = entries
        .iter()
        .map(|entry| (entry.index, entry))
        .collect::<BTreeMap<_, _>>();
    let mut selected = selection
        .items
        .iter()
        .map(|item| (item.source_index, format!("demo.item.{}", item.id)))
        .collect::<Vec<_>>();
    selected.extend(
        adaptations
            .items
            .iter()
            .filter(|item| item.status == DemoItemCoverageStatus::Active)
            .map(|item| (item.source_index, item.item_id.clone())),
    );
    selected.sort();
    selected.dedup();

    let mut files_by_id = BTreeMap::new();
    for entry in fs::read_dir(items_path)? {
        let path = entry?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
        if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
            files_by_id.insert(id.to_owned(), (path, value));
        }
    }

    let effect_programs_path = items_path
        .parent()
        .ok_or_else(|| {
            LegacyImportError::InvalidDemoItemSelection("items path has no parent".to_owned())
        })?
        .join("effectPrograms");
    fs::create_dir_all(&effect_programs_path)?;
    let mut changed = 0;
    for (source_index, item_id) in selected {
        let entry = entries.get(&source_index).ok_or_else(|| {
            LegacyImportError::InvalidDemoItemSelection(format!(
                "source index {source_index} is missing from {K_INFO_SOURCE}"
            ))
        })?;
        let (path, value) = files_by_id.get_mut(&item_id).ok_or_else(|| {
            LegacyImportError::InvalidDemoItemSelection(format!(
                "active item {item_id} has no JSON definition"
            ))
        })?;
        let before = value.clone();
        let vulnerabilities = item_destruction_vulnerabilities(entry.tval);
        if vulnerabilities.is_empty() {
            value
                .as_object_mut()
                .expect("item definitions are JSON objects")
                .remove("elementalDestructionVulnerabilities");
        } else {
            value["elementalDestructionVulnerabilities"] = serde_json::json!(vulnerabilities);
        }
        let immunities = item_destruction_immunities(&entry.flags);
        if immunities.is_empty() {
            value
                .as_object_mut()
                .expect("item definitions are JSON objects")
                .remove("elementalDestructionImmunities");
        } else {
            value["elementalDestructionImmunities"] = serde_json::json!(immunities);
        }
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                LegacyImportError::InvalidDemoItemSelection(format!(
                    "{} has no UTF-8 file stem",
                    path.display()
                ))
            })?;
        let shatter_program_path = effect_programs_path.join(format!("{stem}-shatter.json"));
        if let Some((effect, radius)) = potion_shatter_effect(entry) {
            let program_id = format!("demo.effect.{stem}.shatter");
            value["shatterEffectProgramId"] = serde_json::json!(program_id);
            value["shatterRadius"] = serde_json::json!(radius);
            let mut program = effect_program_from_inline(&program_id, effect)
                .map_err(LegacyImportError::InvalidDemoItemSelection)?;
            program["input"] = serde_json::json!("area");
            fs::write(
                shatter_program_path,
                serde_json::to_string_pretty(&program)? + "\n",
            )?;
        } else {
            value
                .as_object_mut()
                .expect("item definitions are JSON objects")
                .remove("shatterEffectProgramId");
            value
                .as_object_mut()
                .expect("item definitions are JSON objects")
                .remove("shatterRadius");
            if shatter_program_path.exists() {
                fs::remove_file(shatter_program_path)?;
            }
        }
        if *value != before {
            fs::write(path, serde_json::to_string_pretty(value)? + "\n")?;
            changed += 1;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monster_w_line_retains_evolution_fields() {
        let actors = parse_r_info("N:956:Horse\nG:q:w\nW:5:1:20:25:70:957\n")
            .expect("synthetic Horse should parse");
        assert_eq!(actors[0].evolution_experience, Some(70));
        assert_eq!(actors[0].evolution_target_index, Some(957));
    }

    #[test]
    fn demo_monster_audit_rejects_an_inverted_level_range() {
        assert!(matches!(
            audit_demo_monsters(Path::new("unused"), Path::new("unused"), 40, 33),
            Err(LegacyImportError::InvalidDemoMonsterSelection(detail))
                if detail == "audit minimum level 40 exceeds maximum level 33"
        ));
    }

    #[test]
    fn demo_monster_import_maps_hobbit_drop_theme() {
        assert_eq!(
            demo_drop_theme_table_id("DROP_HOBBIT"),
            Some("demo.loot-table.hobbit")
        );
    }

    #[test]
    fn demo_monster_audit_separates_location_scope_from_mechanism_status() {
        let camelot = LegacyMonsterEntry {
            flags: vec!["DUNGEON_2".to_owned()],
            ..LegacyMonsterEntry::default()
        };
        assert_eq!(
            demo_monster_location_restrictions(&camelot),
            vec!["camelot-only"]
        );

        let wilderness_fixed_unique = LegacyMonsterEntry {
            flags: vec![
                "WILD_ONLY".to_owned(),
                "WILD_OCEAN".to_owned(),
                "FIXED_UNIQUE".to_owned(),
            ],
            ..LegacyMonsterEntry::default()
        };
        assert_eq!(
            demo_monster_location_restrictions(&wilderness_fixed_unique),
            vec!["wilderness-only", "ocean-only"]
        );
        let fixed_unique = LegacyMonsterEntry {
            flags: vec!["FIXED_UNIQUE".to_owned()],
            ..LegacyMonsterEntry::default()
        };
        assert!(demo_monster_location_restrictions(&fixed_unique).is_empty());

        assert_eq!(
            demo_monster_audit_status(true, 1, false, true),
            DemoMonsterAuditStatus::Excluded
        );
        assert_eq!(
            demo_monster_audit_status(false, 1185, false, true),
            DemoMonsterAuditStatus::Guardian
        );
        assert_eq!(
            demo_monster_audit_status(false, 2, false, false),
            DemoMonsterAuditStatus::Excluded
        );
        assert_eq!(
            demo_monster_audit_status(false, 3, true, true),
            DemoMonsterAuditStatus::Blocked
        );
        assert_eq!(
            demo_monster_audit_status(false, 4, true, false),
            DemoMonsterAuditStatus::Direct
        );
        assert_eq!(
            demo_monster_audit_status(true, 5, true, false),
            DemoMonsterAuditStatus::Selected
        );
        assert!(demo_monster_audit_omission_is_safe("POS_GAIN_AC"));
        assert!(demo_monster_audit_omission_is_safe("POS_BACKSTAB"));
        assert!(demo_monster_audit_omission_is_safe("POS_SUST_CHR"));
        assert!(demo_monster_audit_omission_is_safe("POS_SUST_DEX"));
        assert!(demo_monster_audit_omission_is_safe("POS_SUST_INT"));
        assert!(demo_monster_audit_omission_is_safe("KILL_EXP"));
        assert!(demo_monster_flag_is_handled("EGYPTIAN"));
        assert!(demo_monster_audit_omission_is_safe("EGYPTIAN2"));
        assert!(demo_monster_audit_omission_is_safe("HINDU2"));
        assert!(demo_monster_audit_omission_is_safe("NORSE2"));
        assert!(demo_monster_audit_omission_is_safe("OLYMPIAN2"));
        assert!(demo_monster_flag_is_handled("AURA_REVENGE"));
        assert!(demo_monster_flag_is_handled("AURA_FEAR"));
        assert!(demo_monster_flag_is_handled("TANUKI"));
        assert!(demo_monster_flag_is_handled("UNIQUE2"));
        assert!(demo_monster_flag_is_handled("WILD_OCEAN"));
        assert!(demo_monster_flag_is_handled("NORSE"));
        assert!(demo_monster_flag_is_handled("HINDU"));
        assert!(demo_monster_flag_is_handled("OLYMPIAN"));
        assert!(demo_monster_flag_is_handled("NO_SUMMON"));
        assert!(demo_monster_flag_is_handled("KNIGHT"));
        assert!(!demo_monster_audit_omission_is_safe("KNIGHT"));
    }

    #[test]
    fn knight_flag_adds_generic_and_camelot_summon_tags() {
        let monsters = parse_r_info(
            "N:1:generic knight\nG:p:w\nI:110:1d1:1:1:1:1\nW:5:1:1:1:0:0\nB:HIT:HURT(1d1)\nF:KNIGHT\n\
             N:2:Camelot knight\nG:p:w\nI:110:1d1:1:1:1:1\nW:5:1:1:1:0:0\nB:HIT:HURT(1d1)\nF:KNIGHT | DUNGEON_2\n",
        )
        .expect("synthetic knights should parse");

        for (monster, id, is_camelot) in [
            (&monsters[0], "generic-knight", false),
            (&monsters[1], "camelot-knight", true),
        ] {
            let selection = DemoMonsterSelectionEntry {
                source_index: monster.index,
                source_id: None,
                id: id.to_owned(),
                tags: Vec::new(),
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            };
            let actor = demo_monster_json(monster, &selection, &mut BTreeMap::new())
                .expect("KNIGHT should import as a content tag");
            let tags = actor["tags"].as_array().expect("tags should be an array");
            assert!(tags.iter().any(|tag| tag == "knight"));
            assert_eq!(tags.iter().any(|tag| tag == "camelot-knight"), is_camelot);
        }
    }

    #[test]
    fn demo_monster_import_maps_ocean_only_allocation() {
        let mut monsters = parse_r_info(
            "N:1:test ocean monster\nG:D:g\nI:110:2d4:8:4:20:10\nW:20:2:999:40:0:0\nB:BITE:HURT(1d4)\nF:AQUATIC | WILD_OCEAN\n",
        )
        .expect("synthetic ocean monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-ocean-monster".to_owned(),
                tags: vec!["ocean".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("WILD_OCEAN should import directly");

        assert_eq!(actor["allocation"]["wildOnly"], true);
        assert_eq!(
            actor["allocation"]["habitats"],
            serde_json::json!(["ocean"])
        );
    }

    fn assert_content_parse_error<T>(
        result: Result<T, LegacyImportError>,
        expected_source: &'static str,
        expected_line: usize,
        expected_field: &'static str,
    ) {
        match result {
            Err(LegacyImportError::ContentParse {
                content_source,
                line,
                field,
                ..
            }) => {
                assert_eq!(content_source, expected_source);
                assert_eq!(line, expected_line);
                assert_eq!(field, expected_field);
            }
            Err(error) => panic!("expected content parse error, got {error}"),
            Ok(_) => panic!("expected content parse error"),
        }
    }

    #[test]
    fn structured_content_parsers_reject_malformed_fields() {
        assert_content_parse_error(
            parse_f_info("N:not-an-index:WALL\n"),
            F_INFO_SOURCE,
            1,
            "N.index",
        );
        assert_content_parse_error(
            parse_r_info("N:1:test monster\nI:fast:1d2:0:0:0:0\n"),
            R_INFO_SOURCE,
            2,
            "I.speed",
        );
        assert_content_parse_error(
            parse_k_info("N:7:valid item\nN:not-an-index:broken item\n"),
            K_INFO_SOURCE,
            2,
            "N.index",
        );
        assert_content_parse_error(
            parse_k_info("N:1:test item\nI:not-a-tval:0:0\n"),
            K_INFO_SOURCE,
            2,
            "I.tval",
        );
        assert_content_parse_error(
            parse_k_info("N:1:test item\nP:0:not-dice:0:0:0\n"),
            K_INFO_SOURCE,
            2,
            "P.damage",
        );
        assert_content_parse_error(
            parse_e_info("N:1:test ego\nC:0:not-a-number:0:0\n"),
            E_INFO_SOURCE,
            2,
            "C.maximumToDamage",
        );
        assert_content_parse_error(
            parse_a_info("N:1:test artifact\nP:0:not-dice:0:0:0\n"),
            A_INFO_SOURCE,
            2,
            "P.damage",
        );
        assert_content_parse_error(
            parse_m_info("N:1\nR:12:1\n"),
            M_INFO_SOURCE,
            2,
            "R.realmIndex",
        );
        assert_content_parse_error(
            parse_m_info("N:1\nR:0:1\nT:1:1:256:4\n"),
            M_INFO_SOURCE,
            3,
            "T.failurePercent",
        );
        assert_content_parse_error(
            parse_s_info("N:1\nS:not-a-skill:0:4000\n"),
            S_INFO_SOURCE,
            2,
            "S.skillIndex",
        );
        assert_content_parse_error(parse_s_info("N:1\nW:5:0:0:4\n"), S_INFO_SOURCE, 2, "W");
    }

    #[test]
    fn wilderness_parsers_keep_the_map_and_positioned_locations() {
        let wilderness = parse_w_info(
            "?:[EQU $WILDERNESS NORMAL]\n\
             W:F:#:0\n\
             W:F:.:6:10\n\
             W:E:1:6:0:1:1:前哨站\n\
             W:F:*:6:3:0:1\n\
             W:D:#####\n\
             W:D:#.1*#\n\
             W:D:#####\n\
             W:P:1:2\n\
             ?:[EQU $WILDERNESS NONE]\n",
        )
        .expect("synthetic normal wilderness should parse");
        assert_eq!((wilderness.width, wilderness.height), (5, 3));
        assert_eq!((wilderness.start_x, wilderness.start_y), (2, 1));
        assert_eq!(wilderness.legend.len(), 4);
        assert_eq!(
            wilderness.towns.get(&1),
            Some(&LegacyWildernessLocation {
                name: "前哨站".to_owned(),
                x: 2,
                y: 1,
            })
        );

        let dungeons = parse_dungeon_records(
            "N:30:Warrens\nP:1:2\nW:1:9:0\nF:CAVE | MONSTER_DIV_4\nM:ORC | TROLL\n",
        )
        .expect("synthetic dungeon record should parse");
        assert_eq!(
            dungeons.get(&30).and_then(dungeon_location).as_ref(),
            Some(&LegacyWildernessLocation {
                name: "Warrens".to_owned(),
                x: 2,
                y: 1,
            })
        );
        assert_eq!(dungeons[&30].minimum_depth, Some(1));
        assert_eq!(dungeons[&30].maximum_depth, Some(9));
        assert_eq!(dungeons[&30].flags, ["CAVE", "MONSTER_DIV_4"]);
        assert_eq!(dungeons[&30].monster_preferences, ["ORC", "TROLL"]);
        assert_eq!(
            (0..=14)
                .map(wilderness_terrain_id)
                .collect::<BTreeSet<_>>()
                .len(),
            15
        );
    }

    #[test]
    fn melee_effect_parser_preserves_order_dice_and_independent_chances() {
        let blow = parse_blow("SLASH:HURT(10d1):POISON(4d4, 50%):CUT(2d3, 30%)", 1)
            .expect("synthetic blow should parse");

        assert_eq!(blow.method, "SLASH");
        assert_eq!(blow.effects.len(), 3);
        assert_eq!(blow.effects[0].token, "HURT");
        assert_eq!(blow.effects[0].dice, Some((10, 1)));
        assert_eq!(blow.effects[0].chance_percent, None);
        assert_eq!(blow.effects[1].token, "POISON");
        assert_eq!(blow.effects[1].dice, Some((4, 4)));
        assert_eq!(blow.effects[1].chance_percent, Some(50));
        assert_eq!(blow.effects[2].token, "CUT");
        assert_eq!(blow.effects[2].dice, Some((2, 3)));
        assert_eq!(blow.effects[2].chance_percent, Some(30));
    }

    #[test]
    fn non_damage_melee_effects_map_without_inventing_damage() {
        let blow = parse_blow(
            "GAZE:CONFUSE:BLIND:PARALYZE(50%):SLOW:STUN(1d4, 10%):TERRIFY:EAT_GOLD:EAT_ITEM:EAT_FOOD:EAT_LITE",
            1,
        )
        .expect("synthetic status blow should parse");
        let effects = blow
            .effects
            .iter()
            .map(|effect| melee_effect_json(effect, None).expect("status effect should map"))
            .collect::<Vec<_>>();

        assert_eq!(effects[0]["type"], "confusion");
        assert_eq!(effects[0]["damageDice"], 0);
        assert_eq!(effects[1]["type"], "blind");
        assert_eq!(effects[2]["type"], "paralysis");
        assert_eq!(effects[2]["chancePercent"], 50);
        assert_eq!(effects[3]["type"], "slow");
        assert_eq!(effects[4]["type"], "stun");
        assert_eq!(effects[4]["durationDice"], 1);
        assert_eq!(effects[4]["durationSides"], 4);
        assert_eq!(effects[4]["chancePercent"], 10);
        assert_eq!(effects[5]["type"], "terrify");
        assert_eq!(effects[6]["type"], "eat-gold");
        assert_eq!(effects[7]["type"], "eat-item");
        assert_eq!(effects[8]["type"], "eat-food");
        assert_eq!(effects[9]["type"], "eat-light");
    }

    #[test]
    fn demo_monster_import_preserves_effectless_beg() {
        let mut monsters =
            parse_r_info("N:1:test beggar\nG:t:y\nI:110:2d3:10:1:0:175\nW:0:1:0:0:0:0\nB:BEG\n")
                .expect("synthetic beggar should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-beggar".to_owned(),
                tags: Vec::new(),
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("BEG should import directly");
        let blow = &actor["meleeRoutine"]["blows"][0];

        assert_eq!(blow["methodId"], "rfb.blow.beg");
        assert_eq!(blow["effects"], serde_json::json!([]));
    }

    #[test]
    fn dice_less_hurt_maps_to_exact_zero_damage_only() {
        let hurt = parse_blow("GAZE:HURT", 1).expect("dice-less HURT should parse");
        let effect = melee_effect_json(&hurt.effects[0], None).expect("HURT should map");

        assert_eq!(effect["type"], "damage");
        assert_eq!(effect["damageDice"], 0);
        assert_eq!(effect["damageSides"], 0);
        assert_eq!(effect["damageType"], "physical");
        assert_eq!(effect["armorMitigated"], true);
    }

    #[test]
    fn light_melee_alias_matches_lite_damage() {
        let light = parse_blow("BITE:LIGHT(1d3, 20%)", 1).expect("LIGHT melee damage should parse");
        let lite = parse_blow("BITE:LITE(1d3, 20%)", 1).expect("LITE melee damage should parse");

        assert_eq!(
            melee_effect_json(&light.effects[0], None),
            melee_effect_json(&lite.effects[0], None)
        );
        assert_eq!(
            melee_effect_json(&light.effects[0], None).expect("LIGHT should map")["damageType"],
            "light"
        );
    }

    #[test]
    fn vampiric_melee_maps_to_unarmored_physical_damage_and_healing() {
        let blow = parse_blow("BITE:VAMP(2d6)", 1).expect("VAMP melee should parse");
        let effect = melee_effect_json(&blow.effects[0], None).expect("VAMP melee should map");

        assert_eq!(effect["type"], "damage");
        assert_eq!(effect["damageDice"], 2);
        assert_eq!(effect["damageSides"], 6);
        assert_eq!(effect["damageType"], "physical");
        assert_eq!(effect["armorMitigated"], false);
        assert_eq!(effect["vampiric"], true);
    }

    #[test]
    fn unlife_melee_maps_to_life_force_drain_instead_of_hp_damage() {
        let blow = parse_blow("TOUCH:UNLIFE(3d4)", 1).expect("UNLIFE melee should parse");
        let effect = melee_effect_json(&blow.effects[0], None).expect("UNLIFE melee should map");
        assert_eq!(effect["type"], "unlife");
        assert_eq!(effect["amountDice"], 3);
        assert_eq!(effect["amountSides"], 4);
        assert!(effect.get("damageType").is_none());
    }

    #[test]
    fn dice_less_disenchant_maps_to_narrow_melee_effect() {
        let blow = parse_blow("GAZE:DISENCHANT", 1).expect("dice-less DISENCHANT should parse");
        let effect = melee_effect_json(&blow.effects[0], None).expect("DISENCHANT should map");

        assert_eq!(effect, serde_json::json!({ "type": "disenchant" }));

        let damaging =
            parse_blow("GAZE:DISENCHANT(1d4)", 1).expect("damaging DISENCHANT should parse");
        let effect =
            melee_effect_json(&damaging.effects[0], None).expect("damage should stay mapped");
        assert_eq!(effect["type"], "damage");
        assert_eq!(effect["damageDice"], 1);
        assert_eq!(effect["damageSides"], 4);
        assert_eq!(effect["damageType"], "disenchant");
    }

    #[test]
    fn monster_import_maps_self_destruct_terrain_light_and_drop_flags() {
        let monsters = parse_r_info(
            "N:1:test breach mote\nG:*:y\nI:110:1d3:8:4:20:10\nW:3:1:10:3:0:0\nB:EXPLODE:FIRE(2d4)\nF:KILL_WALL | KILL_ITEM | TAKE_ITEM | HAS_LITE_1 | SELF_LITE_2\nF:ONLY_ITEM | DROP_90 | DROP_1D2 | DROP_GOOD | UNIQUE\nO:DROP_WARRIOR\n",
        )
        .expect("synthetic monster should parse");
        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let actor = &outcome.actor_files[0].1;
        let blow = &actor["meleeRoutine"]["blows"][0];

        assert_eq!(blow["selfDestructs"], true);
        assert_eq!(blow["effects"][0]["damageType"], "fire");
        assert_eq!(actor["terrainInteraction"]["destroysWalls"], true);
        assert_eq!(actor["terrainInteraction"]["destroysItems"], true);
        assert_eq!(actor["terrainInteraction"]["picksUpItems"], true);
        assert_eq!(actor["light"]["radius"], 3);
        assert_eq!(actor["light"]["intrinsic"], true);
        assert_eq!(actor["deathDrop"]["kind"], "items");
        assert_eq!(actor["deathDrop"]["chanceRolls"][0]["percent"], 90);
        assert_eq!(actor["deathDrop"]["countDice"][0]["dice"], 1);
        assert_eq!(actor["deathDrop"]["minimumQuality"], "fine");
        assert_eq!(actor["deathDrop"]["themeChancePercent"], 50);
    }

    #[test]
    fn monster_capture_policy_follows_unique_questor_and_special_identity() {
        let policy = |index: u32, flags: &str| {
            let source = format!(
                "N:{index}:test capture target\nG:t:w\nI:110:1d3:8:4:20:10\nW:3:1:10:3:0:0\nF:{flags}\n"
            );
            let monsters = parse_r_info(&source).expect("synthetic monster should parse");
            monster_json(
                &monsters[0],
                "test-capture-target",
                None,
                "physical",
                Some(serde_json::json!({ "blows": [] })),
                None,
            )["capturePolicy"]
                .clone()
        };

        assert!(policy(1, "MALE").is_null());
        assert_eq!(policy(1, "UNIQUE"), "pet-only");
        assert_eq!(policy(1, "NAZGUL"), "pet-only");
        assert_eq!(policy(1, "QUESTOR"), "immune");
        assert_eq!(policy(1, "UNIQUE2"), "immune");
        assert_eq!(policy(932, "MALE"), "immune");
    }

    #[test]
    fn monster_import_preserves_source_level_zero() {
        let monsters =
            parse_r_info("N:1:test townsperson\nG:t:w\nI:110:1d3:8:4:20:10\nW:0:1:0:0:0:0\n")
                .expect("synthetic level-zero monster should parse");
        let actor = monster_json(
            &monsters[0],
            "test-townsperson",
            None,
            "physical",
            Some(serde_json::json!({ "blows": [] })),
            None,
        );

        assert_eq!(actor["level"], 0);
    }

    #[test]
    fn demo_monster_import_maps_special_mechanics_without_fallbacks() {
        let mut monsters = parse_r_info(
            "N:1:test special monster\nG:j:v\nI:110:1d3:8:4:20:10\nW:12:1:50:40:0:0\nB:TOUCH:DRAIN_EXP(10d6)\nA:POISON(1d2):ACID(2d3):ELEC(3d4):FIRE(4d5):CAUSE_2(5d6)\nF:SHAPECHANGER | MOVE_BODY | REGENERATE | REFLECTING | DUNGEON_31 | DROP_1D2\nO:DROP_WARRIOR_SHOOT\nS:1_IN_5 | POLYMORPH\n",
        )
        .expect("synthetic special monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-special-monster".to_owned(),
                tags: vec!["warrens".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("supported special mechanics should import directly");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][0]["type"],
            "drain-experience"
        );
        assert_eq!(actor["contactAuras"][0]["damageType"], "poison");
        assert_eq!(actor["contactAuras"][0]["damageDice"], 1);
        assert_eq!(actor["contactAuras"][0]["damageSides"], 2);
        assert_eq!(actor["contactAuras"][1]["damageType"], "acid");
        assert_eq!(actor["contactAuras"][1]["damageDice"], 2);
        assert_eq!(actor["contactAuras"][1]["damageSides"], 3);
        assert_eq!(actor["contactAuras"][2]["damageType"], "electricity");
        assert_eq!(actor["contactAuras"][2]["damageDice"], 3);
        assert_eq!(actor["contactAuras"][2]["damageSides"], 4);
        assert_eq!(actor["contactAuras"][3]["damageType"], "fire");
        assert_eq!(actor["contactAuras"][3]["damageDice"], 4);
        assert_eq!(actor["contactAuras"][3]["damageSides"], 5);
        assert_eq!(actor["contactAuras"][4]["damageType"], "curse");
        assert_eq!(actor["contactAuras"][4]["damageDice"], 5);
        assert_eq!(actor["contactAuras"][4]["damageSides"], 6);
        assert_eq!(
            actor["monsterCasting"]["abilities"][0]["abilityId"],
            "rfb-legacy.ability.polymorph-target"
        );
        assert_eq!(actor["movesWeakerBodies"], true);
        assert_eq!(actor["regenerates"], true);
        assert_eq!(actor["reflectsBolts"], true);
        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "shapechanger"))
        );
        assert_eq!(
            actor["allocation"]["legacyDungeonIndices"],
            serde_json::json!([31])
        );
        assert_eq!(actor["deathDrop"]["themeTableId"], "demo.loot-table.archer");
        assert_eq!(
            demo_drop_theme_table_id("DROP_DWARF"),
            Some("demo.loot-table.dwarf")
        );
    }

    #[test]
    fn demo_monster_import_maps_p45_shared_mechanics() {
        let mut monsters = parse_r_info(
            "N:1013:Rolento\nG:C:u\nI:150:90d10:70:150:4:170\nW:38:4:999:4000:0:0\nB:HIT:POIS(2d3)\nA:CAUSE_3(3d3)\nF:FORCE_MAXHP | ONLY_ITEM | DROP_1D2 | NORSE2 | POS_BACKSTAB\nO:DROP_ROGUE\nS:1_IN_3 | HELL_LANCE | S_HOUND | S_SPECIAL\n",
        )
        .expect("synthetic P45 monster should parse");
        let mut abilities = BTreeMap::new();
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1013,
                source_id: None,
                id: "rolento".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: vec!["NORSE2".to_owned(), "POS_BACKSTAB".to_owned()],
                omitted_spells: Vec::new(),
            },
            &mut abilities,
        )
        .expect("P45 mechanics should import directly");

        let effect = &actor["meleeRoutine"]["blows"][0]["effects"][0];
        assert_eq!(effect["type"], "damage");
        assert_eq!(effect["damageType"], "poison");
        assert_eq!(actor["contactAuras"][0]["damageType"], "curse");
        assert_eq!(actor["deathDrop"]["themeTableId"], "demo.loot-table.rogue");
        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "hound") })
        );

        let hell_lance = &abilities["rfb-legacy.ability.beam-hell-fire-1d1-75"];
        assert_eq!(hell_lance["effect"]["type"], "beam-damage");
        assert_eq!(hell_lance["effect"]["damageType"], "hell-fire");
        let hounds = &abilities["rfb-legacy.ability.summon-hound-l38-1d2-1"];
        assert_eq!(hounds["effect"]["category"], "hound");
        assert_eq!(hounds["effect"]["countDice"], 1);
        assert_eq!(hounds["effect"]["countSides"], 2);
        assert_eq!(hounds["effect"]["countBonus"], 1);
        let grenades = &abilities["rfb-legacy.ability.summon-hand-grenade-l38-1d3-1"];
        assert_eq!(grenades["effect"]["category"], "hand-grenade");
        assert_eq!(grenades["effect"]["countDice"], 1);
        assert_eq!(grenades["effect"]["countSides"], 3);
        assert_eq!(grenades["effect"]["countBonus"], 1);

        let explicit = map_spell_token(
            "HELL_LANCE(66)",
            33,
            2,
            "demo.actor.anti-paladin",
            &mut abilities,
        )
        .expect("explicit hell lance should map through the beam effect");
        assert_eq!(explicit, "rfb-legacy.ability.beam-hell-fire-1d1-65");
        assert_eq!(
            demo_drop_theme_table_id("DROP_PALADIN_EVIL"),
            Some("demo.loot-table.evil-paladin")
        );
        assert_eq!(
            demo_drop_theme_table_id("DROP_SAMURAI"),
            Some("demo.loot-table.samurai")
        );
    }

    #[test]
    fn demo_monster_import_maps_p49_shared_mechanics() {
        let mut monsters = parse_r_info(
            "N:661:test P49 monster\nG:A:v\nI:130:100d35:30:140:255:170\nW:45:5:999:8000:50000:942\nB:GAZE:MIND_BLAST(2d6)\nA:SHARDS(3d3)\nF:POS_SUST_WIS\nS:1_IN_3 | INVULN | HOLY_LANCE(77) | JMP_NEXUS | HEAL(200)\n",
        )
        .expect("synthetic P49 monster should parse");
        let mut abilities = BTreeMap::new();
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 661,
                source_id: None,
                id: "test-p49-monster".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: vec!["POS_SUST_WIS".to_owned()],
                omitted_spells: Vec::new(),
            },
            &mut abilities,
        )
        .expect("P49 mechanics should import directly");

        assert!(demo_monster_audit_omission_is_safe("POS_SUST_WIS"));
        let effects = actor["meleeRoutine"]["blows"][0]["effects"]
            .as_array()
            .expect("mind blast should expand to melee effects");
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0]["damageType"], "psi");
        assert_eq!(effects[0]["damageDice"], 2);
        assert_eq!(effects[0]["damageSides"], 6);
        assert_eq!(effects[1]["type"], "confusion");
        assert_eq!(effects[1]["damageDice"], 0);
        assert_eq!(actor["contactAuras"][0]["damageType"], "shards");
        assert_eq!(actor["contactAuras"][0]["damageDice"], 3);
        assert_eq!(actor["contactAuras"][0]["damageSides"], 3);

        let ability_ids = actor["monsterCasting"]["abilities"]
            .as_array()
            .expect("P49 monster should cast")
            .iter()
            .map(|ability| ability["abilityId"].as_str().expect("ability id"))
            .collect::<Vec<_>>();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.invulnerability-self",
                "rfb-legacy.ability.beam-holy-fire-1d1-76",
                "rfb-legacy.ability.jump-nexus-l45",
                "rfb-legacy.ability.heal-200",
            ]
        );
        let invulnerability = &abilities["rfb-legacy.ability.invulnerability-self"];
        assert_eq!(
            invulnerability["target"]["modes"],
            serde_json::json!(["self"])
        );
        assert_eq!(
            invulnerability["effect"]["statusKindId"],
            "rfb.status.invulnerability"
        );
        assert_eq!(invulnerability["effect"]["durationTicks"], 4);
        assert_eq!(invulnerability["effect"]["durationDice"], 1);
        assert_eq!(invulnerability["effect"]["durationSides"], 4);
        assert_eq!(invulnerability["effect"]["incomingDamagePercent"], 0);
        assert_eq!(
            abilities["rfb-legacy.ability.beam-holy-fire-1d1-76"]["effect"]["damageType"],
            "holy-fire"
        );
        assert_eq!(
            abilities["rfb-legacy.ability.jump-nexus-l45"]["effect"]["damageType"],
            "nexus"
        );
        assert_eq!(
            abilities["rfb-legacy.ability.heal-200"]["effect"]["amount"],
            200
        );
    }

    #[test]
    fn demo_monster_import_maps_trump_as_a_shared_turn_tag() {
        let mut monsters = parse_r_info(
            "N:517:Jurt the Living Trump\nG:p:R\nI:120:10d100:20:90:40:150\nW:34:5:999:2662:0:0\nB:HIT:HURT(5d5)\nF:FORCE_MAXHP | TRUMP\n",
        )
        .expect("synthetic trump monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 517,
                source_id: None,
                id: "jurt-the-living-trump".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("TRUMP should import through the shared tag");

        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "trump") })
        );
    }

    #[test]
    fn demo_monster_import_maps_quantum_as_a_shared_turn_tag() {
        let mut monsters = parse_r_info(
            "N:863:Quantum dot\nG:*:v\nI:130:10d10:10:70:0:20\nW:35:3:999:1600:0:0\nB:SPORE:HURT(2d4)\nF:QUANTUM | STUPID\n",
        )
        .expect("synthetic quantum monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 863,
                source_id: None,
                id: "quantum-dot".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: vec!["STUPID".to_owned()],
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("QUANTUM should import through the shared tag");

        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "quantum"))
        );
    }

    #[test]
    fn demo_monster_import_maps_clear_head_as_a_shared_turn_tag() {
        let mut monsters = parse_r_info(
            "N:683:Utgard-Loke\nG:P:w\nI:120:40d100:30:125:15:600\nW:44:3:999:26620:0:0\nB:HIT:HURT(8d12)\nF:FORCE_MAXHP | CLEAR_HEAD | GIANT\n",
        )
        .expect("synthetic clear-head monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 683,
                source_id: None,
                id: "utgard-loke".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("CLEAR_HEAD should import through the shared tag");

        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "clear-head"))
        );
    }

    #[test]
    fn demo_monster_import_maps_dieless_inertia() {
        let mut monsters = parse_r_info(
            "N:1256:Baba Yaga\nG:O:G\nI:121:33d66:33:99:5:110\nW:43:1:999:9339:0:0\nB:GAZE:PARALYZE:INERTIA\nF:UNIQUE | FEMALE\n",
        )
        .expect("synthetic inertia monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1256,
                source_id: None,
                id: "baba-yaga".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: vec!["FEMALE".to_owned()],
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("dieless INERTIA should import");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][1]["type"],
            "inertia"
        );
    }

    #[test]
    fn demo_monster_import_maps_amberite_as_a_death_trigger_tag() {
        let mut monsters = parse_r_info(
            "N:660:Rinaldo, Son of Brand\nG:p:w\nI:120:16d100:20:120:40:170\nW:41:3:999:9000:0:0\nB:HIT:HURT(8d6)\nF:UNIQUE | AMBERITE | HUMAN\n",
        )
        .expect("synthetic Amberite should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 660,
                source_id: None,
                id: "rinaldo-son-of-brand".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("AMBERITE should import through the shared tag");

        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "amberite"))
        );
    }

    #[test]
    fn demo_monster_import_maps_bomb_as_a_composite_self_destruct_effect() {
        let mut monsters = parse_r_info(
            "N:700:Leprechaun fanatic\nG:h:D\nI:123:6d6:8:13:8:50\nW:46:6:80:80:0:0\nB:EXPLODE:BOMB(12d12)\nF:MULTIPLY | RAND_25 | EVIL\n",
        )
        .expect("synthetic bomb monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 700,
                source_id: None,
                id: "leprechaun-fanatic".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("BOMB should import through the self-destruct effect");

        let blow = &actor["meleeRoutine"]["blows"][0];
        assert_eq!(blow["selfDestructs"], true);
        assert_eq!(
            blow["effects"][0],
            serde_json::json!({
                "type": "bomb",
                "damageDice": 12,
                "damageSides": 12,
            })
        );
    }

    #[test]
    fn demo_monster_import_maps_percent_only_mana_drain_with_level_power() {
        let mut monsters = parse_r_info(
            "N:1356:Draugr\nG:z:b\nI:121:12d99:30:145:10:350\nW:48:7:999:6400:0:0\nB:GAZE:TERRIFY(5d5):TERRIFY:DRAIN_MANA(25%)\nF:UNDEAD | EVIL\n",
        )
        .expect("synthetic percent mana drain should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1356,
                source_id: None,
                id: "draugr".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("percent-only DRAIN_MANA should import");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][2],
            serde_json::json!({
                "type": "drain-resource",
                "amountDice": 1,
                "amountSides": 25,
                "chancePercent": 25,
            })
        );
    }

    #[test]
    fn demo_monster_import_maps_slow_self_destruct_rider() {
        let mut monsters = parse_r_info(
            "N:921:Internet Exploder\nG:e:B\nI:140:20d20:25:0:1:300\nW:50:4:999:1000:0:0\nB:EXPLODE:TIME(10d20):SLOW\nF:NONLIVING\n",
        )
        .expect("synthetic self-destruct rider should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 921,
                source_id: None,
                id: "internet-exploder".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("SLOW self-destruct rider should import");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"],
            serde_json::json!([
                {
                    "type": "damage",
                    "damageDice": 10,
                    "damageSides": 20,
                    "damageType": "time",
                    "armorMitigated": false,
                },
                { "type": "slow" },
            ])
        );
    }

    #[test]
    fn demo_monster_import_maps_shatter_as_a_shared_melee_effect() {
        let mut monsters = parse_r_info(
            "N:558:Colossus\nG:g:w\nI:120:30d30:30:120:0:400\nW:36:3:999:10000:0:0\nB:HIT:SHATTER(8d8)\nF:GIANT\n",
        )
        .expect("synthetic shatter monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 558,
                source_id: None,
                id: "colossus".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("SHATTER should import through the shared melee effect");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][0]["type"],
            "shatter"
        );
    }

    #[test]
    fn demo_monster_import_maps_gaze_sleep_and_amnesia() {
        let mut monsters = parse_r_info(
            "N:603:Beholder\nG:e:v\nI:120:20d20:30:80:0:200\nW:38:3:999:5000:0:0\nB:GAZE:DAM(2d4):TERRIFY:SLEEP(35%)\nB:GAZE:DAM(2d4):STUN(3d3):AMNESIA(25%)\nF:EVIL\nS:FREQ_35 | GAZE\n",
        )
        .expect("synthetic beholder should parse");
        let mut abilities = BTreeMap::new();
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 603,
                source_id: None,
                id: "beholder".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut abilities,
        )
        .expect("Beholder mechanics should import");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][2]["type"],
            "paralysis"
        );
        assert_eq!(
            actor["meleeRoutine"]["blows"][1]["effects"][2]["type"],
            "amnesia"
        );
        assert!(
            abilities["rfb-legacy.ability.gaze"]["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "gaze"))
        );
    }

    #[test]
    fn demo_monster_import_maps_dice_less_time_without_inventing_damage() {
        let mut monsters = parse_r_info(
            "N:1092:Chronomage\nG:p:B\nI:120:20d20:30:80:0:200\nW:40:3:999:5000:0:0\nB:HIT:HURT(3d7):TIME(25%)\nF:SMART\n",
        )
        .expect("synthetic chronomage should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1092,
                source_id: None,
                id: "chronomage".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("dice-less TIME should import");

        assert_eq!(
            actor["meleeRoutine"]["blows"][0]["effects"][1],
            serde_json::json!({"type": "time", "chancePercent": 25})
        );
    }

    #[test]
    fn demo_monster_import_requires_exact_unsupported_spell_omissions() {
        let monsters = parse_r_info(
            "N:1:test old castle caster\nG:p:D\nI:110:1d3:8:4:20:10\nW:40:1:50:40:0:0\nB:HIT:HURT(1d1)\nS:1_IN_5 | TEST_UNSUPPORTED\n",
        )
        .expect("synthetic caster should parse");
        let selection = DemoMonsterSelectionEntry {
            source_index: 1,
            source_id: None,
            id: "test-old-castle-caster".to_owned(),
            tags: vec!["old-castle".to_owned()],
            omitted_flags: Vec::new(),
            omitted_spells: vec!["TEST_UNSUPPORTED".to_owned()],
        };
        let actor = demo_monster_json(&monsters[0], &selection, &mut BTreeMap::new())
            .expect("declared unsupported spell should be omitted");
        assert!(actor.get("monsterCasting").is_none());

        let mut supported = monsters[0].clone();
        supported.spells = vec!["SCARE".to_owned()];
        let stale = DemoMonsterSelectionEntry {
            omitted_spells: vec!["SCARE".to_owned()],
            ..selection
        };
        assert!(matches!(
            demo_monster_json(&supported, &stale, &mut BTreeMap::new()),
            Err(LegacyImportError::InvalidDemoMonsterSelection(_))
        ));
    }

    #[test]
    fn demo_monster_import_maps_elemental_contact_aura_flags() {
        let monsters = parse_r_info(
            "N:1:test fire aura\nG:*:r\nI:120:6d6:100:30:0:25\nW:17:1:50:50:0:0\nB:EXPLODE:FIRE(8d8)\nF:AURA_FIRE | NEVER_MOVE\n\
             N:2:test cold aura\nG:*:w\nI:120:6d6:100:30:0:25\nW:17:1:50:50:0:0\nB:EXPLODE:COLD(8d8)\nF:AURA_COLD | NEVER_MOVE\n\
             N:3:test electricity aura\nG:*:B\nI:120:6d6:100:30:0:25\nW:17:1:50:50:0:0\nB:EXPLODE:ELEC(8d8)\nF:AURA_ELEC | NEVER_MOVE\n",
        )
        .expect("synthetic elemental aura monsters should parse");

        for (entry, damage_type) in monsters.iter().zip(["fire", "cold", "electricity"]) {
            let actor = demo_monster_json(
                entry,
                &DemoMonsterSelectionEntry {
                    source_index: entry.index,
                    source_id: None,
                    id: kebab(&entry.name),
                    tags: vec!["warrens".to_owned()],
                    omitted_flags: Vec::new(),
                    omitted_spells: Vec::new(),
                },
                &mut BTreeMap::new(),
            )
            .expect("elemental contact aura should import directly");
            assert_eq!(actor["contactAuras"][0]["damageType"], damage_type);
            assert_eq!(actor["contactAuras"][0]["damageDice"], 1);
            assert_eq!(actor["contactAuras"][0]["damageSides"], 2);
        }

        let ice = parse_r_info(
            "N:4:test ice aura\nG:S:w\nI:120:6d6:100:30:0:25\nW:50:1:999:50:0:0\nB:BITE:ICE(1d8)\nA:ICE(3d3)\nF:NEVER_MOVE\n",
        )
        .expect("synthetic ice aura monster should parse")
        .remove(0);
        let actor = demo_monster_json(
            &ice,
            &DemoMonsterSelectionEntry {
                source_index: ice.index,
                source_id: None,
                id: kebab(&ice.name),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("explicit ice contact aura should import directly");
        assert_eq!(actor["contactAuras"][0]["damageType"], "ice");
        assert_eq!(actor["contactAuras"][0]["damageDice"], 3);
        assert_eq!(actor["contactAuras"][0]["damageSides"], 3);

        let light = parse_r_info(
            "N:5:test light aura\nG:p:o\nI:120:6d6:100:30:0:25\nW:52:1:999:50:0:0\nB:HIT:HURT(1d8)\nA:LITE(3d3)\nF:NEVER_MOVE\n",
        )
        .expect("synthetic light aura monster should parse")
        .remove(0);
        let actor = demo_monster_json(
            &light,
            &DemoMonsterSelectionEntry {
                source_index: light.index,
                source_id: None,
                id: kebab(&light.name),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("explicit light contact aura should import directly");
        assert_eq!(actor["contactAuras"][0]["damageType"], "light");
        assert_eq!(actor["contactAuras"][0]["damageDice"], 3);
        assert_eq!(actor["contactAuras"][0]["damageSides"], 3);

        for (index, token, damage_type) in [
            (6, "NETHER", "nether"),
            (7, "HOLY_FIRE", "holy-fire"),
            (8, "DARK", "dark"),
            (9, "DISINTEGRATE", "disintegrate"),
            (10, "PLASMA", "plasma"),
            (11, "HELL_FIRE", "hell-fire"),
        ] {
            let source = format!(
                "N:{index}:test P59A aura\nG:p:o\nI:120:6d6:100:30:0:25\nW:63:1:999:50:0:0\nB:HIT:HURT(1d8)\nA:{token}(3d3)\nF:NEVER_MOVE\n"
            );
            let entry = parse_r_info(&source)
                .expect("synthetic P59A aura monster should parse")
                .remove(0);
            let actor = demo_monster_json(
                &entry,
                &DemoMonsterSelectionEntry {
                    source_index: entry.index,
                    source_id: None,
                    id: kebab(&entry.name),
                    tags: vec!["orc-cave".to_owned()],
                    omitted_flags: Vec::new(),
                    omitted_spells: Vec::new(),
                },
                &mut BTreeMap::new(),
            )
            .expect("P59A contact aura should import directly");
            assert_eq!(actor["contactAuras"][0]["damageType"], damage_type);
            assert_eq!(actor["contactAuras"][0]["damageDice"], 3);
            assert_eq!(actor["contactAuras"][0]["damageSides"], 3);
        }

        let mut multiple = monsters[0].clone();
        multiple.flags.push("AURA_ELEC".to_owned());
        let actor = demo_monster_json(
            &multiple,
            &DemoMonsterSelectionEntry {
                source_index: multiple.index,
                source_id: None,
                id: kebab(&multiple.name),
                tags: vec!["warrens".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("multiple elemental contact auras should import directly");
        assert_eq!(actor["contactAuras"][0]["damageType"], "fire");
        assert_eq!(actor["contactAuras"][1]["damageType"], "electricity");
    }

    #[test]
    fn demo_monster_import_maps_o5_traits_without_omissions() {
        let mut monsters = parse_r_info(
            "N:1:test O5 monster\nG:p:w\nI:120:6d6:100:30:0:25\nW:30:1:50:50:0:0\nB:HIT:HURT(1d4)\nF:AURA_REVENGE | AURA_FEAR | TANUKI | UNIQUE2\n",
        )
        .expect("synthetic O5 monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-o5-monster".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("O5 traits should import directly");
        let tags = actor["tags"].as_array().expect("tags should be an array");
        for expected in ["aura-revenge", "aura-fear", "tanuki", "unique2"] {
            assert!(tags.iter().any(|tag| tag == expected), "missing {expected}");
        }
    }

    #[test]
    fn demo_monster_import_maps_nazgul_lifetime_limit() {
        let mut monsters = parse_r_info(
            "N:696:Nazgul\nG:W:D\nI:125:66d66:90:141:10:180\nW:63:7:999:22000:0:0\nB:HIT:HURT(10d6)\nF:FORCE_MAXHP | NAZGUL | UNDEAD\n",
        )
        .expect("synthetic Nazgul should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 696,
                source_id: None,
                id: "nazgul".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("NAZGUL should import through the shared lifetime field");

        assert_eq!(actor["lifetimeInstanceLimit"], 5);
        assert!(
            actor["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "unique"))
        );
    }

    #[test]
    fn demo_monster_import_maps_p33_blockers_without_fallbacks() {
        let mut monsters = parse_r_info(
            "N:1:test sewer climber\nG:u:w\nI:110:2d4:8:4:20:10\nW:16:2:50:40:0:0\nB:HIT:HURT(1d4)\nF:HURT_ROCK | CAN_CLIMB | COMPOST | FIXED_UNIQUE | NO_QUEST | SMART\nS:1_IN_10 | TELE_LEVEL\n",
        )
        .expect("synthetic P33 monster should parse");
        let mut abilities = BTreeMap::new();
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-sewer-climber".to_owned(),
                tags: vec!["warrens".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut abilities,
        )
        .expect("P33 mechanics should import directly");

        assert_eq!(actor["resistances"]["disintegrate"], "vulnerable");
        assert_eq!(actor["movement"]["modes"], serde_json::json!(["climb"]));
        assert_eq!(actor["allocation"]["taskId"], "demo.task.the-sewer");
        assert_eq!(actor["monsterCasting"]["smart"], true);
        assert!(actor["tags"].as_array().is_some_and(|tags| {
            tags.iter().any(|tag| tag == "fixed-unique") && tags.iter().any(|tag| tag == "no-quest")
        }));
        assert_eq!(
            abilities["rfb-legacy.ability.teleport-level"]["effect"]["type"],
            "teleport-level"
        );
    }

    #[test]
    fn terrain_import_maps_climbable_terrain_mode() {
        let terrain = parse_f_info("N:1:STEEP\nG:#:w\nF:LOS | PROJECT | CAN_CLIMB | MOUNTAIN\n")
            .expect("synthetic climb terrain should parse");
        let outcome = convert_content(
            &terrain,
            &[],
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );

        assert_eq!(
            outcome.terrain_files[0].1["movementModes"],
            serde_json::json!(["climb"])
        );
    }

    #[test]
    fn monster_import_maps_carried_darkness_as_negative_light() {
        let monsters = parse_r_info(
            "N:1:test shadow jelly\nG:j:D\nI:110:1d3:8:4:20:10\nW:3:1:10:3:0:0\nB:TOUCH:DAM(1d2)\nF:HAS_DARK_1\n",
        )
        .expect("synthetic darkness monster should parse");
        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let actor = &outcome.actor_files[0].1;

        assert_eq!(actor["light"]["radius"], 1);
        assert_eq!(actor["light"]["intrinsic"], false);
        assert_eq!(actor["light"]["darkness"], true);
    }

    #[test]
    fn terrain_import_uses_destroyed_target_for_disintegration() {
        let terrain = parse_f_info(
            "N:1:FLOOR\nG:.:w\nF:LOS | PROJECT | MOVE | FLOOR\nN:2:WALL\nG:#:w\nK:DESTROYED:*FLOOR*\nF:WALL | HURT_DISI\n",
        )
        .expect("synthetic terrain should parse");
        let outcome = convert_content(
            &terrain,
            &[],
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let wall = outcome
            .terrain_files
            .iter()
            .find(|(name, _)| name == "wall.json")
            .map(|(_, value)| value)
            .expect("wall should import");

        assert_eq!(
            wall["monsterDestroyToTerrainId"],
            "rfb-legacy.terrain.floor"
        );
    }

    // Synthetic samples in the legacy line format; no legacy content.
    const SYNTHETIC_F_INFO: &str = "\
N:0:NONE\n\
N:7:TEST_ARCH\n\
E:测试拱门\n\
G:':u\n\
F:LOS | PROJECT | MOVE | PLACE |\n\
F:DOOR | TELEPORTABLE\n\
N:8:TEST_WALL\n\
G:#:w\n\
F:PERMANENT | HURT_ROCK\n";

    const SYNTHETIC_R_INFO: &str = "\
N:1:test drifting lantern\n\
G:*:y\n\
I:120:3d5:12:14:20:40\n\
W:4:2:30:9:20:64\n\
B:TOUCH:FIRE(2d4)\n\
B:CRUSH:HURT(1d6):STUN(1d3)\n\
F:NEVER_MOVE | RES_FIRE\n\
S:1_IN_5 | SCARE | BR_FIRE\n\
N:2:test hollow shade\n\
G:G:w\n\
I:110:2d3:8:5:10:10\n\
W:2:1:20:4:10:30\n\
B:GAZE:TERRIFY\n";

    #[test]
    fn synthetic_entries_parse_and_convert_with_gap_accounting() {
        let terrain = parse_f_info(SYNTHETIC_F_INFO).expect("synthetic terrain should parse");
        assert_eq!(terrain.len(), 3);
        assert_eq!(terrain[1].tag, "TEST_ARCH");
        assert_eq!(terrain[1].glyph, Some('\''));
        assert_eq!(terrain[1].flags.len(), 6);

        let monsters = parse_r_info(SYNTHETIC_R_INFO).expect("synthetic monsters should parse");
        assert_eq!(monsters.len(), 2);
        assert_eq!(monsters[0].speed, Some(120));
        assert_eq!(monsters[0].hp_dice, Some((3, 5)));
        assert_eq!(monsters[0].armor_class, Some(14));
        assert_eq!(monsters[0].level, Some(4));
        assert_eq!(monsters[0].blows.len(), 2);
        assert_eq!(monsters[0].spells.len(), 3);

        let outcome = convert_content(
            &terrain,
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.terrain_imported, 2);
        assert_eq!(outcome.report.terrain_skipped, 1);
        assert_eq!(outcome.report.monsters_imported, 2);
        assert_eq!(outcome.report.monsters_skipped, 0);
        assert_eq!(outcome.report.monsters_with_unmapped_spells, 0);
        assert_eq!(outcome.report.monsters_with_melee_routine, 2);
        assert_eq!(outcome.report.monsters_with_inexpressible_blows, 0);
        assert_eq!(outcome.report.unmapped_spells.len(), 0);
        assert_eq!(outcome.report.spells_mapped["SCARE"], 1);
        assert_eq!(outcome.report.spells_mapped["BR_FIRE"], 1);
        assert_eq!(outcome.report.monsters_with_casting, 1);
        assert_eq!(outcome.ability_files.len(), 2);
        assert_eq!(outcome.resource_files.len(), 1);
        assert!(
            !outcome
                .report
                .skip_reasons
                .contains_key("monster-without-expressible-melee")
        );

        let (name, lantern) = &outcome.actor_files[0];
        assert_eq!(name, "test-drifting-lantern.json");
        assert_eq!(lantern["maxHp"], 9);
        assert_eq!(lantern["speed"], 120);
        assert_eq!(lantern["defense"], 1);
        assert_eq!(lantern["damageType"], "fire");
        assert_eq!(lantern["damageDice"], 2);
        let blows = lantern["meleeRoutine"]["blows"]
            .as_array()
            .expect("routine should list blows");
        assert_eq!(blows.len(), 2);
        assert_eq!(blows[0]["methodId"], "rfb-legacy.blow.touch");
        assert_eq!(blows[0]["effects"][0]["type"], "damage");
        assert_eq!(blows[0]["effects"][0]["damageType"], "fire");
        assert_eq!(blows[1]["methodId"], "rfb-legacy.blow.crush");
        assert_eq!(blows[1]["effects"][0]["damageType"], "physical");
        assert_eq!(blows[1]["effects"][0]["damageDice"], 1);
        assert_eq!(blows[1]["effects"][0]["damageSides"], 6);
        assert_eq!(blows[1]["effects"].as_array().unwrap().len(), 2);
        assert_eq!(blows[1]["effects"][1]["type"], "stun");
        // RES_FIRE folds into the content resistance map.
        assert_eq!(lantern["resistances"]["fire"], "resistant");
        assert_eq!(lantern["movement"]["neverMoves"], true);
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("NEVER_MOVE")
        );
        assert_eq!(lantern["monsterCasting"]["frequencyPercent"], 20);
        assert_eq!(
            lantern["monsterCasting"]["abilities"][0]["abilityId"],
            "rfb-legacy.ability.scare"
        );

        let (_, arch) = &outcome.terrain_files[0];
        assert_eq!(arch["walkable"], true);
        assert_eq!(arch["blocksSight"], false);
        assert_eq!(arch["id"], "rfb-legacy.terrain.test-arch");
    }

    #[test]
    fn class_shells_and_magic_profiles_parse_without_suffix_collisions() {
        const DEFINES: &str = "\
#define CLASS_WARRIOR 0
#define CLASS_MAGE 1
#define CLASS_BLOOD_MAGE 36
";
        const CLASSES: &str = "\
case CLASS_BLOOD_MAGE:
    result = blood_mage_get_class();
    break;
case CLASS_MAGE:
    result = mage_get_class();
    break;
case CLASS_WARRIOR:
    result = warrior_get_class();
    break;
";
        const CLASS_SOURCE: &str = r#"
static caster_info * _caster_info(void)
{
    static caster_info me = {0};
    me.which_stat = A_INT;
    me.min_fail = 5;
    me.min_level = 1;
    me.encumbrance.max_wgt = 430;
    me.encumbrance.weapon_pct = 100;
    me.encumbrance.enc_wgt = 600;
    me.options = CASTER_ALLOW_DEC_MANA |
                 CASTER_GLOVE_ENCUMBRANCE;
    return &me;
}

class_t *blood_mage_get_class(void)
{
    static class_t me = {0};
    me.stats[A_STR] = 5;
    return &me;
}

class_t *mage_get_class(void)
{
    static class_t me = {0};
    skills_t bs = { 30, 40, 38, 3, 16, 20, 34, 20 };
    skills_t xs = { 7, 15, 11, 0, 0, 0, 6, 7 };
    me.stats[A_STR] = -4;
    me.stats[A_INT] = 3;
    me.stats[A_DEX] = 1;
    me.stats[A_CON] = -2;
    me.stats[A_CHR] = -2;
    me.base_skills = bs;
    me.extra_skills = xs;
    me.life = 95;
    me.base_hp = 0;
    me.exp = 130;
    me.pets = 25;
    me.flags = CLASS_SENSE1_MED |
               CLASS_REGEN_MANA;
    me.birth = _birth;
    me.caster_info = _caster_info;
    return &me;
}
"#;
        const M_INFO: &str = "\
### Mage ###
N:1
I:SORCERY:INT:0x05:0:1:430
R:0:1
T:1:1:30:4
T:3:2:35:4
R:1:0
R:4:1
T:1:1:25:4
T:2:2:25:4
T:2:2:25:4
T:3:3:27:4
T:5:5:30:4
T:7:10:75:4
T:9:9:30:4
T:11:12:40:4
T:12:12:40:5
T:13:12:30:4
T:18:15:50:10
T:24:21:60:30
T:30:75:80:30
T:32:30:60:16
T:36:35:80:70
T:39:30:95:25
T:10:20:80:180
T:10:15:80:30
T:11:11:30:15
T:30:25:75:50
T:34:90:70:90
T:36:35:60:125
T:38:35:70:40
T:40:40:70:200
";
        const S_INFO: &str = "\
N:1
W:4:1:0:2
W:4:2:0:3
S:0:0:4000
S:1:0:2000
S:2:0:6000
";

        let registrations = parse_class_registrations(DEFINES, CLASSES);
        assert_eq!(
            registrations
                .iter()
                .map(|entry| (entry.index, entry.id.as_str()))
                .collect::<Vec<_>>(),
            [(0, "warrior"), (1, "mage"), (36, "blood-mage")]
        );
        let mage_registration = registrations[1].clone();
        let mage_body =
            extract_class_block(CLASS_SOURCE, "mage_get_class").expect("exact mage function");
        let mut mage = parse_class_block(mage_registration, mage_body);
        assert_eq!(mage.character.stats, [-4, 3, 0, 1, -2, -2]);
        assert_eq!(mage.character.skills, [30, 40, 38, 3, 16, 20, 34, 20]);
        assert_eq!(mage.character.extra_skills, [7, 15, 11, 0, 0, 0, 6, 7]);
        assert_eq!(mage.character.life, 95);
        assert_eq!(mage.character.exp, 130);
        assert_eq!(mage.pet_upkeep_divisor, 25);
        assert_eq!(
            mage.character.flags,
            ["CLASS_SENSE1_MED", "CLASS_REGEN_MANA"]
        );
        assert!(!mage.character.dynamic, "{:?}", mage.character);
        let caster = parse_class_caster_profile(CLASS_SOURCE, mage_body).expect("caster profile");
        assert!(!caster.dynamic);
        assert_eq!(caster.casting_attribute, "int");
        assert_eq!(caster.minimum_failure_percent, 5);
        assert_eq!(caster.max_encumbrance_weight, 430);
        assert_eq!(caster.options, ["allow-dec-mana", "glove-encumbrance"]);
        mage.caster_profile = Some(caster);

        let magic_profiles = parse_m_info(M_INFO).expect("synthetic magic profiles should parse");
        assert_eq!(magic_profiles.len(), 1);
        assert_eq!(magic_profiles[0].extra_flags, 5);
        assert_eq!(magic_profiles[0].realms.len(), 3);
        assert!(magic_profiles[0].realms[0].readable);
        assert_eq!(magic_profiles[0].realms[0].spells.len(), 2);
        assert_eq!(magic_profiles[0].realms[0].spells[1].mana, 2);
        assert!(!magic_profiles[0].realms[1].readable);
        assert_eq!(magic_profiles[0].realms[2].spells.len(), 24);

        let proficiency_profiles =
            parse_s_info(S_INFO).expect("synthetic proficiency profiles should parse");
        assert_eq!(proficiency_profiles.len(), 1);
        assert_eq!(
            proficiency_profiles[0].weapon_entries,
            [
                LegacyWeaponProficiencyEntry {
                    weapon_type: 4,
                    weapon_subtype: 1,
                    initial_rank: 0,
                    maximum_rank: 2,
                },
                LegacyWeaponProficiencyEntry {
                    weapon_type: 4,
                    weapon_subtype: 2,
                    initial_rank: 0,
                    maximum_rank: 3,
                },
            ]
        );
        assert_eq!(proficiency_profiles[0].skill_entries.len(), 3);
        assert_eq!(
            proficiency_profiles[0]
                .skill_entries
                .iter()
                .find(|entry| entry.skill_index == 2),
            Some(&LegacySkillProficiencyEntry {
                skill_index: 2,
                initial: 0,
                maximum: 6_000,
            })
        );

        let cavalry =
            parse_s_info("N:22\nS:2:2000:8000\n").expect("Cavalry riding proficiency should parse");
        assert_eq!(
            cavalry[0].skill_entries,
            [LegacySkillProficiencyEntry {
                skill_index: 2,
                initial: 2_000,
                maximum: 8_000,
            }]
        );

        let characters = LegacyCharacterSources {
            classes: vec![mage],
            magic_profiles,
            proficiency_profiles,
            ..LegacyCharacterSources::default()
        };
        let terrain = ["FLOOR", "GRANITE", "QUARTZ", "MAGMA"]
            .into_iter()
            .enumerate()
            .map(|(index, tag)| LegacyTerrainEntry {
                index: u32::try_from(index).expect("synthetic terrain index fits u32"),
                tag: tag.to_owned(),
                display_name: Some(tag.to_owned()),
                glyph: Some(if tag == "FLOOR" { '.' } else { '#' }),
                flags: if tag == "FLOOR" {
                    vec!["FLOOR".to_owned(), "MOVE".to_owned()]
                } else {
                    Vec::new()
                },
                destroyed_tag: None,
            })
            .collect::<Vec<_>>();
        let outcome = convert_content(&terrain, &[], &[], &[], &[], &characters);
        assert_eq!(outcome.report.classes_imported, 1);
        assert_eq!(outcome.report.magic_spell_profile_rows, 26);
        assert_eq!(outcome.report.realm_readability["life"], 1);
        assert_eq!(outcome.report.realm_readability["death"], 1);
        assert_eq!(outcome.report.player_abilities_imported, 24);
        assert_eq!(outcome.report.player_ability_books_imported, 3);
        assert_eq!(outcome.report.classes_with_runtime_casting_profiles, 1);
        assert_eq!(outcome.report.player_spell_parameter_overrides, 24);
        assert_eq!(
            outcome
                .report
                .player_spell_effect_gaps
                .get("death")
                .copied()
                .unwrap_or_default(),
            0
        );
        assert!(
            !outcome
                .report
                .player_spell_behavior_gaps
                .contains_key("player-level-effect-scaling")
        );
        assert!(
            !outcome
                .report
                .player_spell_behavior_gaps
                .contains_key("monster-status-power-resolution")
        );
        assert!(
            !outcome
                .report
                .player_spell_behavior_gaps
                .contains_key("random-resistance-duration")
        );
        assert!(
            !outcome
                .report
                .player_spell_behavior_gaps
                .contains_key("malediction-random-rider")
        );
        for gap in [
            "invoke-spirits-actor-polymorph",
            "invoke-spirits-line-light",
            "invoke-spirits-earthquake",
            "invoke-spirits-destroy-area",
        ] {
            assert!(!outcome.report.player_spell_behavior_gaps.contains_key(gap));
        }
        assert_eq!(
            outcome.report.class_proficiency_gaps["weapon-proficiency"],
            2
        );
        assert_eq!(outcome.class_files[0].1["modifiers"]["strength"], -4);
        assert_eq!(
            outcome.class_files[0].1["castingProfile"]["abilityOverrides"]
                .as_array()
                .map(Vec::len),
            Some(24)
        );
        assert_eq!(outcome.ability_book_files.len(), 3);
        assert_eq!(
            outcome.class_files[0].1["castingProfile"]["beamChanceLevelMultiplier"],
            1
        );
        assert_eq!(
            outcome.class_files[0].1["castingProfile"]["beamChanceLevelDivisor"],
            1
        );
        let black_sleep = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-black-sleep.json")
            .map(|(_, value)| value)
            .expect("black sleep ability should be generated");
        assert_eq!(black_sleep["effect"]["power"], 2);
        assert_eq!(black_sleep["levelScaling"][0]["field"], "status-power");
        let detect_unlife = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-detect-unlife.json")
            .map(|(_, value)| value)
            .expect("detect unlife ability should be generated");
        assert_eq!(
            detect_unlife["target"]["modes"],
            serde_json::json!(["self"])
        );
        assert_eq!(detect_unlife["effect"]["subject"], "actor");
        assert_eq!(detect_unlife["effect"]["radius"], 30);
        let detect_evil = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-detect-evil.json")
            .map(|(_, value)| value)
            .expect("detect evil ability should be generated");
        assert_eq!(detect_evil["effect"]["radius"], 30);
        let malediction = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-malediction.json")
            .map(|(_, value)| value)
            .expect("Malediction ability should be generated");
        assert_eq!(malediction["effect"]["type"], "malediction");
        assert_eq!(malediction["effect"]["damageDice"], 3);
        assert_eq!(malediction["effect"]["damageSides"], 4);
        assert_eq!(
            malediction["spellPowerFields"].as_array().map(Vec::len),
            Some(3)
        );
        let poison_branding = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-poison-branding.json")
            .map(|(_, value)| value)
            .expect("poison branding ability should be generated");
        assert_eq!(
            poison_branding["target"]["modes"],
            serde_json::json!(["item"])
        );
        assert_eq!(poison_branding["effect"]["type"], "brand-weapon");
        assert_eq!(
            poison_branding["effect"]["affixId"],
            LEGACY_SLAYING_WEAPON_AFFIX_ID
        );
        assert_eq!(poison_branding["effect"]["brand"], "poison");
        assert_eq!(poison_branding["effect"]["resistance"], "poison");
        let vampiric_branding = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-vampiric-branding.json")
            .map(|(_, value)| value)
            .expect("vampiric branding ability should be generated");
        assert_eq!(
            vampiric_branding["target"]["modes"],
            serde_json::json!(["item"])
        );
        assert_eq!(vampiric_branding["effect"]["type"], "brand-weapon");
        assert_eq!(
            vampiric_branding["effect"]["affixId"],
            LEGACY_DEATH_WEAPON_AFFIX_ID
        );
        let resistance = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-necromantic-resistance.json")
            .map(|(_, value)| value)
            .expect("necromantic resistance ability should be generated");
        assert_eq!(
            resistance["effect"]["grantedResistances"]["poison"],
            "resistant"
        );
        assert_eq!(resistance["effect"]["durationTicks"], 20);
        assert_eq!(resistance["effect"]["durationDice"], 1);
        assert_eq!(resistance["effect"]["durationSides"], 20);
        assert_eq!(
            resistance["spellPowerFields"],
            serde_json::json!([
                {"effectIndex": 0, "field": "status-duration-ticks"},
                {"effectIndex": 0, "field": "status-duration-sides"}
            ])
        );
        let vampiric_drain = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-vampiric-drain.json")
            .map(|(_, value)| value)
            .expect("vampiric drain ability should be generated");
        assert_eq!(vampiric_drain["effect"]["feeds"], true);
        let control = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-enslave-undead.json")
            .map(|(_, value)| value)
            .expect("enslave undead ability should be generated");
        assert_eq!(control["effect"]["type"], "control");
        assert_eq!(control["levelScaling"][0]["field"], "control-power");
        let entropy = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-entropy-orb.json")
            .map(|(_, value)| value)
            .expect("entropy orb ability should be generated");
        assert_eq!(entropy["effect"]["targetCategory"], "living");
        assert_eq!(entropy["levelScaling"][1]["maximum"], 3);
        let animate_dead = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-animate-dead.json")
            .map(|(_, value)| value)
            .expect("animate dead ability should be generated");
        assert_eq!(animate_dead["effect"]["type"], "animate-dead");
        assert_eq!(
            animate_dead["effect"]["corpseItemKindId"],
            LEGACY_CORPSE_ITEM_ID
        );
        let invoke_spirits = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-invoke-spirits.json")
            .map(|(_, value)| value)
            .expect("invoke spirits ability should be generated");
        assert_eq!(invoke_spirits["effect"]["type"], "random-choice");
        assert_eq!(
            invoke_spirits["spellPowerFields"][0]["field"],
            "random-choice-roll"
        );
        assert_eq!(
            invoke_spirits["effect"]["branches"]
                .as_array()
                .map(Vec::len),
            Some(23)
        );
        let invoke_branches = invoke_spirits["effect"]["branches"]
            .as_array()
            .expect("Invoke Spirits branches should be an array");
        assert_eq!(invoke_branches[3]["effect"]["type"], "polymorph-target");
        assert_eq!(invoke_branches[7]["effect"]["type"], "light-line");
        assert_eq!(invoke_branches[18]["effect"]["type"], "earthquake");
        assert_eq!(invoke_branches[19]["effect"]["type"], "area-destruction");
        assert_eq!(invoke_branches[4]["effect"]["damageDice"], 3);
        assert_eq!(
            invoke_branches[4]["levelScaling"][0]["field"],
            "damage-dice"
        );
        assert_eq!(invoke_branches[5]["effect"]["power"], 1);
        assert_eq!(invoke_branches[12]["effect"]["damageBonus"], 74);
        assert_eq!(invoke_branches[17]["effect"]["damageBonus"], 99);
        assert_eq!(invoke_branches[20]["effect"]["power"], 50);
        assert_eq!(invoke_branches[22]["effect"]["type"], "sequence");
        assert_eq!(
            invoke_branches[22]["effect"]["effects"]
                .as_array()
                .map(Vec::len),
            Some(4)
        );
        assert_eq!(invoke_branches[22]["effect"]["effects"][3]["amount"], 300);
        assert!(
            !serde_json::to_string(invoke_branches)
                .expect("Invoke Spirits branches should serialize")
                .contains("no-op")
        );
        let vampirism_true = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-vampirism-true.json")
            .map(|(_, value)| value)
            .expect("vampirism true ability should be generated");
        assert_eq!(vampirism_true["effect"]["repeat"], 3);
        let darkness_storm = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "death-darkness-storm.json")
            .map(|(_, value)| value)
            .expect("darkness storm ability should be generated");
        assert_eq!(darkness_storm["levelScaling"][0]["curve"], "prorated");
        assert_eq!(
            outcome.ability_book_files[2].1["abilityIds"],
            serde_json::json!([
                "rfb-legacy.ability.death-berserk",
                "rfb-legacy.ability.death-invoke-spirits",
                "rfb-legacy.ability.death-dark-bolt",
                "rfb-legacy.ability.death-battle-frenzy",
                "rfb-legacy.ability.death-vampiric-branding",
                "rfb-legacy.ability.death-vampirism-true",
                "rfb-legacy.ability.death-nether-wave",
                "rfb-legacy.ability.death-darkness-storm",
            ])
        );
        assert_eq!(
            outcome.magic_profile_files[0].1["realms"][0]["realmId"],
            "life"
        );
        assert!(outcome.realm_readability.is_some());
    }

    #[test]
    fn explicit_damage_dice_specs_parse_including_the_flat_identity() {
        assert_eq!(parse_explicit_damage_dice("18d8+26"), Some((18, 8, 26)));
        assert_eq!(parse_explicit_damage_dice("9d8"), Some((9, 8, 0)));
        assert_eq!(parse_explicit_damage_dice("140"), Some((1, 1, 139)));
        assert_eq!(parse_explicit_damage_dice("d8"), None);
        assert_eq!(parse_explicit_damage_dice("9d"), None);
        assert_eq!(parse_explicit_damage_dice("fire"), None);
    }

    #[test]
    fn bolt_and_ball_tokens_map_with_level_scaled_defaults() {
        const CASTER_R_INFO: &str = "\
N:3:test cinder mage\n\
G:p:r\n\
I:110:4d6:10:10:10:10\n\
W:30:3:20:9:10:40\n\
B:HIT:HURT(1d4)\n\
S:1_IN_2 | BO_FIRE | BA_ACID | BO_FIRE(18d8+26) | THROW | BO_MANA\n";
        let monsters = parse_r_info(CASTER_R_INFO).expect("synthetic caster should parse");
        assert_eq!(monsters.len(), 1);
        assert_eq!(monsters[0].level, Some(30));

        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.monsters_with_casting, 1);
        assert_eq!(outcome.report.spells_mapped["BO_FIRE"], 1);
        assert_eq!(outcome.report.spells_mapped["BO_FIRE(18d8+26)"], 1);
        // Mana bolts map to the real legacy element since the damage-type
        // roster expansion: 1d(7*30/2)+50 at level 30.
        assert_eq!(outcome.report.spells_mapped["BO_MANA"], 1);
        assert_eq!(outcome.report.unmapped_spells.len(), 0);

        let (_, mage) = &outcome.actor_files[0];
        let ability_ids: Vec<&str> = mage["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.bolt-fire-9d8-10",
                "rfb-legacy.ability.ball-acid-1d90-15",
                "rfb-legacy.ability.bolt-fire-18d8-26",
                "rfb-legacy.ability.bolt-physical-1d1-89",
                "rfb-legacy.ability.bolt-mana-1d105-50",
            ]
        );
        let mana_bolt = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "bolt-mana-1d105-50.json")
            .map(|(_, value)| value)
            .expect("mana bolt ability should be generated");
        assert_eq!(mana_bolt["effect"]["damageType"], "mana");

        let bolt = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "bolt-fire-9d8-10.json")
            .map(|(_, value)| value)
            .expect("default fire bolt ability should be generated");
        assert_eq!(bolt["effect"]["type"], "damage");
        assert_eq!(bolt["effect"]["damageDice"], 9);
        assert_eq!(bolt["effect"]["damageSides"], 8);
        assert_eq!(bolt["effect"]["damageBonus"], 10);
        assert_eq!(bolt["effect"]["damageType"], "fire");
        assert_eq!(bolt["target"]["range"], 8);

        let ball = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "ball-acid-1d90-15.json")
            .map(|(_, value)| value)
            .expect("acid ball ability should be generated");
        assert_eq!(ball["effect"]["type"], "area-damage");
        assert_eq!(ball["effect"]["radius"], 2);
        assert_eq!(ball["effect"]["damageSides"], 90);
        assert_eq!(ball["effect"]["damageBonus"], 15);

        let throw = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "bolt-physical-1d1-89.json")
            .map(|(_, value)| value)
            .expect("flat throw ability should be generated");
        assert_eq!(throw["effect"]["type"], "damage");
        assert_eq!(throw["effect"]["damageDice"], 1);
        assert_eq!(throw["effect"]["damageSides"], 1);
        assert_eq!(throw["effect"]["damageBonus"], 89);
    }

    #[test]
    fn p68_poison_and_nexus_ball_tokens_reuse_existing_damage_effects() {
        let mut abilities = BTreeMap::new();
        let poison = map_spell_token(
            "BA_POIS",
            71,
            2,
            "demo.actor.izanami-spirit-of-yomi",
            &mut abilities,
        )
        .expect("BA_POIS should map");
        assert_eq!(poison, "rfb-legacy.ability.ball-poison-12d2");
        assert_eq!(abilities[&poison]["effect"]["damageType"], "poison");
        assert_eq!(abilities[&poison]["effect"]["radius"], 2);

        let nexus = map_spell_token(
            "BA_NEXUS(10d10+158)",
            77,
            2,
            "demo.actor.nephthys-lady-of-the-night",
            &mut abilities,
        )
        .expect("BA_NEXUS should map");
        assert_eq!(nexus, "rfb-legacy.ability.ball-nexus-10d10-158");
        assert_eq!(abilities[&nexus]["effect"]["damageType"], "nexus");
        assert_eq!(abilities[&nexus]["effect"]["radius"], 2);
    }

    #[test]
    fn breath_freq_and_possessor_tokens_follow_legacy_semantics() {
        const DRAGON_R_INFO: &str = "\
N:4:test ashen dragon\n\
G:D:r\n\
I:110:10d10:20:30:10:10\n\
W:20:2:20:9:10:40\n\
B:BITE:HURT(2d6)\n\
S:FREQ_50 | BR_FIRE(40%) | BR_POISON | DETECT_MONSTERS | MAPPING\n";
        let monsters = parse_r_info(DRAGON_R_INFO).expect("synthetic dragon should parse");
        assert_eq!(monsters.len(), 1);

        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let (_, dragon) = &outcome.actor_files[0];
        // FREQ_50 is the direct-percentage frequency syntax.
        assert_eq!(dragon["monsterCasting"]["frequencyPercent"], 50);
        // The dragon glyph widens the breath cone even below level 50, and
        // the (40%) override replaces only the percentage.
        let ability_ids: Vec<&str> = dragon["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.breath-fire-40-900-r3",
                "rfb-legacy.ability.breath-poison-17-600-r3",
            ]
        );
        let breath = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "breath-fire-40-900-r3.json")
            .map(|(_, value)| value)
            .expect("fire breath ability should be generated");
        assert_eq!(breath["effect"]["type"], "breath-damage");
        assert_eq!(breath["effect"]["hpPercent"], 40);
        assert_eq!(breath["effect"]["maxDamage"], 900);
        assert_eq!(breath["effect"]["radius"], 3);
        assert_eq!(breath["target"]["modes"], serde_json::json!(["direction"]));
        // Possessor-only spells count as not-applicable, never as gaps.
        assert_eq!(outcome.report.not_applicable_spells["DETECT_MONSTERS"], 1);
        assert_eq!(outcome.report.not_applicable_spells["MAPPING"], 1);
        assert_eq!(outcome.report.unmapped_spells.len(), 0);
        assert_eq!(outcome.report.monsters_with_unmapped_spells, 0);

        let mut abilities = BTreeMap::new();
        let holy_fire = map_spell_token(
            "BR_HOLY_FIRE",
            89,
            2,
            "demo.actor.raphael-the-messenger",
            &mut abilities,
        )
        .expect("BR_HOLY_FIRE should map");
        assert_eq!(holy_fire, "rfb-legacy.ability.breath-holy-fire-17-250-r2");
        assert_eq!(abilities[&holy_fire]["effect"]["damageType"], "holy-fire");
        assert_eq!(abilities[&holy_fire]["effect"]["hpPercent"], 17);
        assert_eq!(abilities[&holy_fire]["effect"]["maxDamage"], 250);
    }

    #[test]
    fn jump_elements_map_exact_damage_multiplier_radius_and_blink() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "JMP_LIGHT(5d5)",
            19,
            2,
            "demo.actor.blinking-light",
            &mut abilities,
        )
        .expect("JMP_LIGHT should map");
        assert_eq!(id, "rfb-legacy.ability.jump-light-5d5");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "jump-damage");
        assert_eq!(effect["damageDice"], 5);
        assert_eq!(effect["damageSides"], 5);
        assert_eq!(effect["damageBonus"], 0);
        assert_eq!(effect["damageMultiplierNumerator"], 5);
        assert_eq!(effect["damageMultiplierDenominator"], 4);
        assert_eq!(effect["damageType"], "light");
        assert_eq!(effect["radius"], 5);
        assert_eq!(effect["blinkRadius"], 10);

        for (token, level, suffix, damage_type, dice, sides, bonus) in [
            ("JMP_FIRE", 31, "jump-fire-l31", "fire", 0, 0, 31),
            ("JMP_ICE", 50, "jump-ice-l50", "ice", 0, 0, 50),
            ("JMP_NETHER", 60, "jump-nether-l60", "nether", 0, 0, 60),
            ("JMP_SHARDS", 85, "jump-shards-l85", "shards", 0, 0, 85),
            (
                "JMP_DISINTEGRATE",
                70,
                "jump-disintegrate-l70",
                "disintegrate",
                0,
                0,
                70,
            ),
            (
                "JMP_HELL_FIRE(66)",
                80,
                "jump-hell-fire-1d1-65",
                "hell-fire",
                1,
                1,
                65,
            ),
            ("JMP_POISON", 32, "jump-poison-l32", "poison", 0, 0, 32),
            (
                "JMP_CONFUSION",
                32,
                "jump-confusion-l32",
                "confusion",
                0,
                0,
                32,
            ),
            ("JMP_DARK(2d4)", 29, "jump-dark-2d4", "dark", 2, 4, 0),
        ] {
            let id = map_spell_token(token, level, 2, "demo.actor.jump-test", &mut abilities)
                .unwrap_or_else(|| panic!("{token} should map"));
            assert_eq!(id, format!("rfb-legacy.ability.{suffix}"));
            let effect = &abilities[&id]["effect"];
            assert_eq!(effect["damageDice"], dice);
            assert_eq!(effect["damageSides"], sides);
            assert_eq!(effect["damageBonus"], bonus);
            assert_eq!(effect["damageType"], damage_type);
            assert_eq!(effect["damageMultiplierNumerator"], 5);
            assert_eq!(effect["damageMultiplierDenominator"], 4);
            assert_eq!(effect["radius"], 5);
            assert_eq!(effect["blinkRadius"], 10);
        }
    }

    #[test]
    fn angel_summon_uses_the_original_aligned_a_glyph_category() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token("S_ANGEL", 50, 4, "demo.actor.planetar", &mut abilities)
            .expect("S_ANGEL should map");
        assert_eq!(id, "rfb-legacy.ability.summon-angel-l50-1d3-1");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "summon-category");
        assert_eq!(effect["category"], "angel");
        assert_eq!(effect["maximumLevel"], 50);
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 3);
        assert_eq!(effect["countBonus"], 1);

        const ANGEL_R_INFO: &str = "N:1:aligned angel\nG:A:w\nI:110:8d8:20:20:10:10\nW:20:2:20:9:10:40\nB:HIT:HURT(1d6)\nF:GOOD\nN:2:unaligned a glyph\nG:A:w\nI:110:8d8:20:20:10:10\nW:20:2:20:9:10:40\nB:HIT:HURT(1d6)\nF:SMART\n";
        let monsters = parse_r_info(ANGEL_R_INFO).expect("synthetic angels should parse");
        let aligned = monster_json(&monsters[0], "aligned-angel", None, "physical", None, None);
        let unaligned = monster_json(
            &monsters[1],
            "unaligned-a-glyph",
            None,
            "physical",
            None,
            None,
        );
        assert!(
            aligned["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "angel"))
        );
        assert!(
            unaligned["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().all(|tag| tag != "angel"))
        );
    }

    #[test]
    fn chameleon_flag_marks_a_form_changer_without_requiring_a_base_blow() {
        const CHAMELEON_R_INFO: &str = "\
N:1040:Chameleon\n\
G:R:v\n\
I:110:10d100:20:0:0:150\n\
W:20:1:999:0:0:0\n\
F:CHAMELEON | ANIMAL | CAN_FLY | RES_FIRE\n";
        let monsters = parse_r_info(CHAMELEON_R_INFO).expect("synthetic chameleon should parse");
        let selection: DemoMonsterSelectionEntry = serde_json::from_value(serde_json::json!({
            "sourceIndex": 1040,
            "id": "chameleon",
            "tags": ["animal", "warrens"],
            "omittedFlags": []
        }))
        .expect("synthetic selection should parse");
        let chameleon = demo_monster_json(&monsters[0], &selection, &mut BTreeMap::new())
            .expect("chameleon should import without a base blow");

        assert!(
            chameleon["tags"]
                .as_array()
                .expect("tags should be an array")
                .iter()
                .any(|tag| tag == "chameleon")
        );
        assert_eq!(chameleon["meleeRoutine"]["blows"], serde_json::json!([]));
    }

    #[test]
    fn eldritch_horror_flag_marks_the_runtime_sanity_trigger() {
        const GHAST_R_INFO: &str = "\
N:327:Ghast\n\
G:z:U\n\
I:110:12d10:40:40:20:90\n\
W:19:1:70:75:0:0\n\
B:KICK:HURT(3d3)\n\
F:UNDEAD | ELDRITCH_HORROR\n";
        let monsters = parse_r_info(GHAST_R_INFO).expect("synthetic ghast should parse");
        let selection: DemoMonsterSelectionEntry = serde_json::from_value(serde_json::json!({
            "sourceIndex": 327,
            "id": "ghast",
            "tags": ["undead", "warrens"],
            "omittedFlags": []
        }))
        .expect("synthetic selection should parse");
        let ghast = demo_monster_json(&monsters[0], &selection, &mut BTreeMap::new())
            .expect("ghast should import with its sanity trigger");

        assert!(
            ghast["tags"]
                .as_array()
                .expect("tags should be an array")
                .iter()
                .any(|tag| tag == "eldritch-horror")
        );
    }

    #[test]
    fn monster_mind_flags_become_runtime_telepathy_tags() {
        const MIND_R_INFO: &str = "\
N:1041:Empty mind\n\
G:m:w\n\
I:110:1d1:1:1:1:1\n\
W:1:1:1:1:0:0\n\
F:EMPTY_MIND | NEVER_BLOW\n\
N:1042:Weird mind\n\
G:m:w\n\
I:110:1d1:1:1:1:1\n\
W:1:1:1:1:0:0\n\
F:WEIRD_MIND | NEVER_BLOW\n";
        let monsters = parse_r_info(MIND_R_INFO).expect("synthetic minds should parse");
        for (monster, id, tag) in [
            (&monsters[0], "empty-mind", "empty-mind"),
            (&monsters[1], "weird-mind", "weird-mind"),
        ] {
            let selection: DemoMonsterSelectionEntry = serde_json::from_value(serde_json::json!({
                "sourceIndex": monster.index,
                "id": id,
                "tags": ["warrens"],
                "omittedFlags": []
            }))
            .expect("synthetic selection should parse");
            let actor = demo_monster_json(monster, &selection, &mut BTreeMap::new())
                .expect("mind flag should be handled");
            assert!(
                actor["tags"]
                    .as_array()
                    .expect("tags should be an array")
                    .iter()
                    .any(|candidate| candidate == tag),
                "{tag}"
            );
        }
    }

    #[test]
    fn cold_blood_flag_becomes_an_infravision_tag() {
        const COLD_BLOOD_R_INFO: &str = "\
N:1043:Cold blooded beast\n\
G:R:w\n\
I:110:1d1:1:1:1:1\n\
W:1:1:1:1:0:0\n\
B:BITE:HURT(1d1)\n\
F:ANIMAL | COLD_BLOOD\n";
        let monsters =
            parse_r_info(COLD_BLOOD_R_INFO).expect("synthetic cold-blooded monster should parse");
        let selection: DemoMonsterSelectionEntry = serde_json::from_value(serde_json::json!({
            "sourceIndex": 1043,
            "id": "cold-blooded-beast",
            "tags": ["warrens"],
            "omittedFlags": []
        }))
        .expect("synthetic selection should parse");
        let actor = demo_monster_json(&monsters[0], &selection, &mut BTreeMap::new())
            .expect("cold blood should be handled");

        assert!(
            actor["tags"]
                .as_array()
                .expect("tags should be an array")
                .iter()
                .any(|candidate| candidate == "cold-blooded")
        );
    }

    #[test]
    fn olympian_and_no_summon_flags_become_runtime_tags() {
        let monsters = parse_r_info(
            "N:1044:Olympian test\nG:P:w\nI:110:1d1:1:1:1:1\nW:1:1:1:1:0:0\nB:HIT:HURT(1d1)\nF:UNIQUE | OLYMPIAN | NO_SUMMON\n",
        )
        .expect("synthetic Olympian should parse");
        let selection: DemoMonsterSelectionEntry = serde_json::from_value(serde_json::json!({
            "sourceIndex": 1044,
            "id": "olympian-test",
            "tags": ["warrens"],
            "omittedFlags": []
        }))
        .expect("synthetic selection should parse");
        let actor = demo_monster_json(&monsters[0], &selection, &mut BTreeMap::new())
            .expect("P75A flags should be handled");
        let tags = actor["tags"].as_array().expect("tags should be an array");
        assert!(tags.iter().any(|tag| tag == "olympian"));
        assert!(tags.iter().any(|tag| tag == "no-summon"));
    }

    #[test]
    fn summon_tokens_map_to_category_and_kin_abilities() {
        const SUMMONER_R_INFO: &str = "\
N:5:test bone caller\n\
G:L:w\n\
I:110:8d8:20:20:10:10\n\
W:20:2:20:9:10:40\n\
B:HIT:HURT(1d6)\n\
F:UNDEAD | DRAGON | RES_ALL | RES_TELE | NO_CONF | NO_STUN\n\
S:1_IN_3 | S_KIN | S_UNDEAD | S_MONSTER(1d1) | S_ANT | S_SPIDER | S_HYDRA | S_LOUSE | S_CYBER | S_CAT\n";
        let monsters = parse_r_info(SUMMONER_R_INFO).expect("synthetic summoner should parse");
        assert_eq!(monsters.len(), 1);

        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let (_, caller) = &outcome.actor_files[0];
        // Type flags become category tags alongside the shared import tag.
        assert_eq!(
            caller["tags"],
            serde_json::json!([
                "legacy-import",
                "kin-glyph-76",
                "dragon",
                "undead",
                "nonliving",
                "resist-all",
                "resist-teleport",
                "high-undead"
            ])
        );
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("RES_ALL")
        );
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("RES_TELE")
        );
        assert_eq!(
            caller["statusImmunities"],
            serde_json::json!(["rfb.status.confusion", "rfb.status.stun"])
        );
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("NO_CONF")
        );
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("NO_STUN")
        );
        let ability_ids: Vec<&str> = caller["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.kin-test-bone-caller",
                "rfb-legacy.ability.summon-undead-l20-1d3-1",
                "rfb-legacy.ability.summon-legacy-import-l20-1d1",
                "rfb-legacy.ability.summon-ant-l20-1d3-1",
                "rfb-legacy.ability.summon-spider-l20-1d3-1",
                "rfb-legacy.ability.summon-hydra-l20-1d3-1",
                "rfb-legacy.ability.summon-louse-l20-1d3-1",
                "rfb-legacy.ability.summon-cyber-l20-1d3",
                "rfb-legacy.ability.summon-cat-l20-1d3-1",
            ]
        );
        assert!(!outcome.report.unmapped_spells.contains_key("S_CYBER"));
        assert!(!outcome.report.unmapped_spells.contains_key("S_CAT"));

        let kin = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "kin-test-bone-caller.json")
            .map(|(_, value)| value)
            .expect("kin summon ability should be generated");
        assert_eq!(kin["effect"]["type"], "summon");
        assert_eq!(
            kin["effect"]["actorKindId"],
            "rfb-legacy.actor.test-bone-caller"
        );
        assert_eq!(kin["effect"]["count"], 2);
        assert_eq!(kin["effect"]["durationTurns"], 10_000);

        let undead = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-undead-l20-1d3-1.json")
            .map(|(_, value)| value)
            .expect("undead summon ability should be generated");
        assert_eq!(undead["effect"]["type"], "summon-category");
        assert_eq!(undead["effect"]["category"], "undead");
        assert_eq!(undead["effect"]["maximumLevel"], 20);
        assert_eq!(undead["effect"]["countDice"], 1);
        assert_eq!(undead["effect"]["countSides"], 3);
        assert_eq!(undead["effect"]["countBonus"], 1);

        let any = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-legacy-import-l20-1d1.json")
            .map(|(_, value)| value)
            .expect("any-monster summon ability should be generated");
        assert_eq!(any["effect"]["category"], "legacy-import");
        assert_eq!(any["effect"]["countDice"], 1);
        assert_eq!(any["effect"]["countSides"], 1);
        assert!(any["effect"].get("countBonus").is_none());

        let ant = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-ant-l20-1d3-1.json")
            .map(|(_, value)| value)
            .expect("ant summon ability should be generated");
        assert_eq!(ant["effect"]["category"], "ant");
        assert_eq!(ant["effect"]["countDice"], 1);
        assert_eq!(ant["effect"]["countSides"], 3);
        assert_eq!(ant["effect"]["countBonus"], 1);

        let spider = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-spider-l20-1d3-1.json")
            .map(|(_, value)| value)
            .expect("spider summon ability should be generated");
        assert_eq!(spider["effect"]["category"], "spider");
        assert_eq!(spider["effect"]["countDice"], 1);
        assert_eq!(spider["effect"]["countSides"], 3);
        assert_eq!(spider["effect"]["countBonus"], 1);

        let hydra = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-hydra-l20-1d3-1.json")
            .map(|(_, value)| value)
            .expect("hydra summon ability should be generated");
        assert_eq!(hydra["effect"]["category"], "hydra");
        assert_eq!(hydra["effect"]["countDice"], 1);
        assert_eq!(hydra["effect"]["countSides"], 3);
        assert_eq!(hydra["effect"]["countBonus"], 1);

        let louse = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "summon-louse-l20-1d3-1.json")
            .map(|(_, value)| value)
            .expect("louse summon ability should be generated");
        assert_eq!(louse["effect"]["category"], "louse");
        assert_eq!(louse["effect"]["countDice"], 1);
        assert_eq!(louse["effect"]["countSides"], 3);
        assert_eq!(louse["effect"]["countBonus"], 1);

        let mut demo_entry = monsters[0].clone();
        demo_entry.spells.retain(|spell| spell != "S_CYBER");
        let demo = demo_monster_json(
            &demo_entry,
            &DemoMonsterSelectionEntry {
                source_index: 5,
                source_id: None,
                id: "test-bone-caller".to_owned(),
                tags: vec!["warrens".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("demo monster should preserve the generic summon category");
        assert!(demo["tags"].as_array().is_some_and(|tags| {
            tags.iter().any(|tag| tag == "legacy-import")
                && tags.iter().any(|tag| tag == "kin-glyph-76")
        }));
    }

    #[test]
    fn camelot_and_knight_summons_keep_their_exact_candidate_rules() {
        let mut abilities = BTreeMap::new();
        let camelot_id = map_spell_token(
            "S_CAMELOT(1d2)",
            32,
            4,
            "demo.actor.arthur-pendragon",
            &mut abilities,
        )
        .expect("Camelot summon should map");
        assert_eq!(
            camelot_id,
            "rfb-legacy.ability.summon-camelot-knight-l32-1d2"
        );
        let camelot = &abilities[&camelot_id]["effect"];
        assert_eq!(camelot["category"], "camelot-knight");
        assert_eq!(camelot["maximumLevel"], 32);
        assert_eq!(camelot["countDice"], 1);
        assert_eq!(camelot["countSides"], 2);
        assert!(camelot.get("batchCandidates").is_none());

        let knight_id = map_spell_token(
            "S_KNIGHT(1d2)",
            26,
            10,
            "demo.actor.camelot-knight",
            &mut abilities,
        )
        .expect("exact knight summon should map");
        assert_eq!(knight_id, "rfb-legacy.ability.summon-knight-l26-1d2");
        let knight = &abilities[&knight_id]["effect"];
        assert_eq!(knight["category"], "knight");
        assert_eq!(knight["maximumLevel"], 26);
        assert_eq!(knight["countDice"], 1);
        assert_eq!(knight["countSides"], 2);
        assert_eq!(
            knight["batchCandidates"],
            serde_json::json!([
                { "actorKindId": "demo.actor.novice-paladin", "weight": 1 },
                { "actorKindId": "demo.actor.paladin", "weight": 1 },
                { "actorKindId": "demo.actor.white-knight", "weight": 1 },
                { "actorKindId": "demo.actor.ultra-elite-paladin", "weight": 1 },
                { "actorKindId": "demo.actor.knight-templar", "weight": 1 },
            ])
        );
    }

    #[test]
    fn othrod_summons_two_orc_kin_instead_of_cloning_the_unique() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "S_KIN",
            32,
            2,
            "demo.actor.othrod-lord-of-the-orcs",
            &mut abilities,
        )
        .expect("Othrod's kin summon should map");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "summon-category");
        assert_eq!(effect["category"], "kin-glyph-111");
        assert_eq!(effect["maximumLevel"], 32);
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 1);
        assert_eq!(effect["countBonus"], 1);
    }

    #[test]
    fn eagle_summon_uses_the_original_mountain_eagle_category() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token("S_EAGLE", 55, 2, "demo.actor.thorondor", &mut abilities)
            .expect("S_EAGLE should map");
        assert_eq!(id, "rfb-legacy.ability.summon-eagle-l55-1d3-1");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "summon-category");
        assert_eq!(effect["category"], "eagle");
        assert_eq!(effect["maximumLevel"], 55);
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 3);
        assert_eq!(effect["countBonus"], 1);
    }

    #[test]
    fn p64b_summons_reuse_exact_existing_categories() {
        let mut abilities = BTreeMap::new();
        for (token, level, caster, expected_id, category, sides, bonus) in [
            (
                "S_NIGHTMARE",
                65,
                "demo.actor.grand-fearlord",
                "rfb-legacy.ability.summon-night-mare-l65-1d3-1",
                "night-mare",
                3,
                1,
            ),
            (
                "S_AMBERITE",
                68,
                "demo.actor.tselakus-the-dreadlord",
                "rfb-legacy.ability.summon-amberite-l68-1d2",
                "amberite",
                2,
                0,
            ),
            (
                "S_NAGA",
                70,
                "demo.actor.vasuki-the-serpent-king",
                "rfb-legacy.ability.summon-kin-glyph-110-l70-1d3-1",
                "kin-glyph-110",
                3,
                1,
            ),
        ] {
            let id = map_spell_token(token, level, 2, caster, &mut abilities)
                .unwrap_or_else(|| panic!("{token} should map"));
            assert_eq!(id, expected_id);
            let effect = &abilities[&id]["effect"];
            assert_eq!(effect["type"], "summon-category");
            assert_eq!(effect["category"], category);
            assert_eq!(effect["maximumLevel"], level);
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], sides);
            assert_eq!(
                effect.get("countBonus").and_then(serde_json::Value::as_u64),
                (bonus > 0).then_some(bonus)
            );
        }
    }

    #[test]
    fn p72_vanara_summon_reuses_the_vanara_category() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "S_VANARA",
            76,
            3,
            "demo.actor.vali-king-of-the-vanaras",
            &mut abilities,
        )
        .expect("S_VANARA should map");
        assert_eq!(id, "rfb-legacy.ability.summon-vanara-l76-1d3-1");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "summon-category");
        assert_eq!(effect["category"], "vanara");
        assert_eq!(effect["maximumLevel"], 76);
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 3);
        assert_eq!(effect["countBonus"], 1);
    }

    #[test]
    fn p65_world_maps_to_the_monster_extra_action_marker() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token("WORLD", 66, 2, "demo.actor.dio-brando", &mut abilities)
            .expect("WORLD should map");
        assert_eq!(id, "rfb-legacy.ability.world");
        let ability = &abilities[&id];
        assert_eq!(ability["target"]["modes"], serde_json::json!(["self"]));
        assert_eq!(ability["effect"]["type"], "no-op");
        assert_eq!(ability["effect"]["reason"], "monster-world");
        assert!(
            ability["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "monster-world"))
        );

        let kin_id = map_spell_token("S_KIN", 66, 2, "demo.actor.dio-brando", &mut abilities)
            .expect("Dio kin summon should map");
        assert_eq!(kin_id, "rfb-legacy.ability.kin-dio-brando");
        assert_eq!(abilities[&kin_id]["effect"]["type"], "summon-category");
        assert_eq!(abilities[&kin_id]["effect"]["category"], "kin-glyph-86");
        assert_eq!(abilities[&kin_id]["effect"]["countDice"], 1);
        assert_eq!(abilities[&kin_id]["effect"]["countSides"], 1);
        assert_eq!(abilities[&kin_id]["effect"]["countBonus"], 1);
    }

    #[test]
    fn p71_banor_rupart_special_maps_to_the_shared_transform_marker() {
        let mut abilities = BTreeMap::new();
        for caster in [
            "demo.actor.banor-rupart",
            "demo.actor.banor-the-prince-regent",
            "demo.actor.rupart-the-general",
        ] {
            let id = map_spell_token("SPECIAL", 71, 2, caster, &mut abilities)
                .expect("Banor/Rupart SPECIAL should map");
            assert_eq!(id, "rfb-legacy.ability.banor-rupart-transform");
        }
        let ability = &abilities["rfb-legacy.ability.banor-rupart-transform"];
        assert_eq!(ability["effect"]["type"], "no-op");
        assert_eq!(ability["effect"]["reason"], "banor-rupart-transform");
        assert!(ability["tags"].as_array().is_some_and(|tags| {
            tags.iter()
                .any(|tag| tag == "monster-banor-rupart-transform")
        }));
    }

    #[test]
    fn special_summons_map_only_to_their_fixed_actor_categories() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "S_SPECIAL",
            25,
            2,
            "demo.actor.zoopi-the-cube-king",
            &mut abilities,
        )
        .expect("Zoopi special should map");
        assert_eq!(id, "rfb-legacy.ability.summon-gelatinous-cube-l16-1d3");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["type"], "summon-category");
        assert_eq!(effect["category"], "gelatinous-cube");
        assert_eq!(effect["maximumLevel"], 16);
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 3);
        assert!(effect.get("countBonus").is_none());
        for (caster, expected_id, category) in [
            (
                "demo.actor.santa-claus",
                "rfb-legacy.ability.summon-reindeer-l52-1d4",
                "reindeer",
            ),
            (
                "demo.actor.jack-of-lanterns",
                "rfb-legacy.ability.summon-death-pumpkin-l52-1d4",
                "death-pumpkin",
            ),
            (
                "demo.actor.bull-gates",
                "rfb-legacy.ability.summon-internet-exploder-l52-1d4",
                "internet-exploder",
            ),
        ] {
            let id = map_spell_token("S_SPECIAL", 52, 4, caster, &mut abilities)
                .unwrap_or_else(|| panic!("{caster} special should map"));
            assert_eq!(id, expected_id);
            let effect = &abilities[&id]["effect"];
            assert_eq!(effect["type"], "summon-category");
            assert_eq!(effect["category"], category);
            assert_eq!(effect["maximumLevel"], 52);
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], 4);
            assert!(effect.get("countBonus").is_none());
            assert!(effect.get("maximumCount").is_none());
        }
        let gospel_id = map_spell_token(
            "S_SPECIAL",
            56,
            4,
            "demo.actor.the-gospel-of-mug",
            &mut abilities,
        )
        .expect("The Gospel of Mug special should map");
        assert_eq!(
            gospel_id,
            "rfb-legacy.ability.summon-tracking-pixel-l56-1d4-max3"
        );
        let gospel = &abilities[&gospel_id]["effect"];
        assert_eq!(gospel["category"], "tracking-pixel");
        assert_eq!(gospel["countDice"], 1);
        assert_eq!(gospel["countSides"], 4);
        assert_eq!(gospel["maximumCount"], 3);
        let gragomani_id = map_spell_token(
            "S_SPECIAL",
            61,
            3,
            "demo.actor.gragomani-the-leprechaun-prophet",
            &mut abilities,
        )
        .expect("Gragomani special should map");
        assert_eq!(
            gragomani_id,
            "rfb-legacy.ability.summon-gragomani-followers-1d4-4"
        );
        let gragomani = &abilities[&gragomani_id]["effect"];
        assert_eq!(gragomani["category"], "kin-glyph-104");
        assert_eq!(gragomani["countDice"], 1);
        assert_eq!(gragomani["countSides"], 4);
        assert_eq!(gragomani["countBonus"], 4);
        assert_eq!(
            gragomani["batchCandidates"],
            serde_json::json!([
                { "actorKindId": "demo.actor.malicious-leprechaun", "weight": 1 },
                { "actorKindId": "demo.actor.leprechaun-fanatic", "weight": 3 },
            ])
        );
        let nightmare_id = map_spell_token(
            "S_SPECIAL",
            66,
            3,
            "demo.actor.the-nightmare-dragon",
            &mut abilities,
        )
        .expect("Nightmare Dragon special should map");
        assert_eq!(
            nightmare_id,
            "rfb-legacy.ability.summon-night-mare-l39-1d3-2"
        );
        let nightmare = &abilities[&nightmare_id]["effect"];
        assert_eq!(nightmare["category"], "night-mare");
        assert_eq!(nightmare["maximumLevel"], 39);
        assert_eq!(nightmare["countDice"], 1);
        assert_eq!(nightmare["countSides"], 3);
        assert_eq!(nightmare["countBonus"], 2);
        for (caster, expected_id, target, sides) in [
            (
                "demo.actor.zeus-king-of-the-olympians",
                "rfb-legacy.ability.summon-shambler-l67-1d4",
                "demo.actor.shambler",
                4,
            ),
            (
                "demo.actor.hermes-the-messenger-god",
                "rfb-legacy.ability.summon-magic-mushroom-patch-l15-1d16",
                "demo.actor.magic-mushroom-patch",
                16,
            ),
        ] {
            let id = map_spell_token("S_SPECIAL", 90, 2, caster, &mut abilities)
                .unwrap_or_else(|| panic!("{caster} special should map"));
            assert_eq!(id, expected_id);
            let effect = &abilities[&id]["effect"];
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], sides);
            assert_eq!(
                effect["batchCandidates"],
                serde_json::json!([{ "actorKindId": target, "weight": 1 }])
            );
        }
        let odin_id = map_spell_token(
            "S_SPECIAL",
            90,
            2,
            "demo.actor.odin-the-all-father",
            &mut abilities,
        )
        .expect("Odin special should map");
        assert_eq!(odin_id, "rfb-legacy.ability.summon-odin-retinue-1d4-max1");
        let odin = &abilities[&odin_id]["effect"];
        assert_eq!(odin["countDice"], 1);
        assert_eq!(odin["countSides"], 4);
        assert_eq!(odin["maximumCount"], 1);
        assert_eq!(
            odin["batchCandidates"],
            serde_json::json!([
                { "actorKindId": "demo.actor.einheri-berserker", "weight": 1 },
                { "actorKindId": "demo.actor.valkyrie", "weight": 1 },
            ])
        );
        let gertrude_id =
            map_spell_token("S_SPECIAL", 40, 3, "demo.actor.gertrude", &mut abilities)
                .expect("Gertrude special should map");
        assert_eq!(
            gertrude_id,
            "rfb-legacy.ability.summon-gertrude-sisters-l40-1d1-1"
        );
        let gertrude = &abilities[&gertrude_id]["effect"];
        assert_eq!(gertrude["type"], "summon-category");
        assert_eq!(gertrude["category"], "witch-sister");
        assert_eq!(gertrude["maximumLevel"], 40);
        assert_eq!(gertrude["countDice"], 1);
        assert_eq!(gertrude["countSides"], 1);
        assert_eq!(gertrude["countBonus"], 1);
        assert_eq!(gertrude["maximumCount"], 2);
        assert!(gertrude.get("batchCandidates").is_none());
        let caldarm_id = map_spell_token(
            "S_SPECIAL",
            79,
            2,
            "demo.actor.caldarm-the-third",
            &mut abilities,
        )
        .expect("Caldarm special should map");
        assert_eq!(
            caldarm_id,
            "rfb-legacy.ability.summon-clone-of-locke-l65-1d3"
        );
        let caldarm = &abilities[&caldarm_id]["effect"];
        assert_eq!(caldarm["category"], "clone-of-locke");
        assert_eq!(caldarm["maximumLevel"], 65);
        assert_eq!(caldarm["countDice"], 1);
        assert_eq!(caldarm["countSides"], 3);
        assert!(caldarm.get("countBonus").is_none());
        for (
            caster,
            expected_id,
            category,
            maximum_level,
            sides,
            bonus,
            actor_kind_id,
            water_flow,
        ) in [
            (
                "demo.actor.varuna-lord-of-water",
                "rfb-legacy.ability.summon-makara-l50-1d2-2",
                "mount-meru",
                50,
                2,
                2,
                "demo.actor.makara",
                true,
            ),
            (
                "demo.actor.demeter-the-goddess-of-nature",
                "rfb-legacy.ability.summon-ent-l46-1d4",
                "giant",
                46,
                4,
                0,
                "demo.actor.ent",
                false,
            ),
            (
                "demo.actor.justshorn-sorcerer-king-of-the-sheeple",
                "rfb-legacy.ability.summon-sheep-l3-1d4",
                "sheep",
                3,
                4,
                0,
                "demo.actor.sheep",
                false,
            ),
            (
                "demo.actor.poseidon-lord-of-seas-and-storm",
                "rfb-legacy.ability.summon-greater-kraken-l63-1d4",
                "ocean",
                63,
                4,
                0,
                "demo.actor.greater-kraken",
                true,
            ),
            (
                "demo.actor.talos-masterwork-spellwarp-automaton",
                "rfb-legacy.ability.summon-spellwarp-automaton-l80-1d3",
                "nonliving",
                80,
                3,
                0,
                "demo.actor.spellwarp-automaton",
                false,
            ),
            (
                "demo.actor.brahma-the-creating-spirit",
                "rfb-legacy.ability.summon-saraswati-l90-1d1",
                "hindu",
                90,
                1,
                0,
                "demo.actor.saraswati-goddess-of-knowledge",
                false,
            ),
            (
                "demo.actor.saraswati-goddess-of-knowledge",
                "rfb-legacy.ability.summon-brahma-l92-1d1",
                "hindu",
                92,
                1,
                0,
                "demo.actor.brahma-the-creating-spirit",
                false,
            ),
        ] {
            let id = map_spell_token("S_SPECIAL", 90, 2, caster, &mut abilities)
                .unwrap_or_else(|| panic!("{caster} special should map"));
            assert_eq!(id, expected_id);
            let ability = &abilities[&id];
            let effect = &ability["effect"];
            assert_eq!(effect["category"], category);
            assert_eq!(effect["maximumLevel"], maximum_level);
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], sides);
            assert_eq!(
                effect
                    .get("countBonus")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0),
                bonus
            );
            assert_eq!(
                effect["batchCandidates"],
                serde_json::json!([{ "actorKindId": actor_kind_id, "weight": 1 }])
            );
            assert_eq!(
                ability["tags"]
                    .as_array()
                    .is_some_and(|tags| tags.iter().any(|tag| tag == "monster-water-flow")),
                water_flow,
            );
        }
        assert!(
            map_spell_token(
                "S_SPECIAL",
                25,
                2,
                "demo.actor.someone-else",
                &mut abilities,
            )
            .is_none()
        );
    }

    #[test]
    fn variant_maintainer_summons_only_software_bugs() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "S_SOFTWARE_BUG",
            14,
            2,
            "demo.actor.the-variant-maintainer",
            &mut abilities,
        )
        .expect("software bug summon should map");
        assert_eq!(id, "rfb-legacy.ability.summon-software-bug-l14-1d3-1");
        let effect = &abilities[&id]["effect"];
        assert_eq!(effect["countDice"], 1);
        assert_eq!(effect["countSides"], 3);
        assert_eq!(effect["countBonus"], 1);
        assert_eq!(
            effect["batchCandidates"],
            serde_json::json!([
                { "actorKindId": "demo.actor.software-bug", "weight": 1 }
            ])
        );
    }

    #[test]
    fn aegir_special_summon_keeps_the_water_flow_and_single_batch_choice() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "S_SPECIAL",
            77,
            3,
            "demo.actor.aegir-god-king-of-the-sea-giants",
            &mut abilities,
        )
        .expect("Aegir special should map");
        assert_eq!(id, "rfb-legacy.ability.summon-aegir-retinue-1d4");
        let ability = &abilities[&id];
        assert_eq!(ability["effect"]["type"], "summon-category");
        assert_eq!(ability["effect"]["category"], "ocean");
        assert_eq!(ability["effect"]["countDice"], 1);
        assert_eq!(ability["effect"]["countSides"], 4);
        assert_eq!(
            ability["effect"]["batchCandidates"],
            serde_json::json!([
                { "actorKindId": "demo.actor.sea-giant", "weight": 1 },
                { "actorKindId": "demo.actor.lesser-kraken", "weight": 1 },
            ])
        );
        assert!(
            ability["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "monster-water-flow"))
        );
    }

    #[test]
    fn pantheon_summons_follow_the_casters_pantheon() {
        let mut abilities = BTreeMap::new();
        for (caster, category) in [
            ("heimdall-guardian-of-bifrost", "norse"),
            ("odin-the-all-father", "norse"),
            ("indra-the-heavenly-king-of-meru", "hindu"),
            ("zeus-king-of-the-olympians", "olympian"),
            ("amun-the-mysterious", "egyptian"),
        ] {
            let id = map_spell_token(
                "S_PANTHEON",
                77,
                3,
                &format!("demo.actor.{caster}"),
                &mut abilities,
            )
            .unwrap_or_else(|| panic!("{caster} pantheon summon should map"));
            assert_eq!(id, format!("rfb-legacy.ability.summon-{category}-l77-1d2"));
            let effect = &abilities[&id]["effect"];
            assert_eq!(effect["type"], "summon-category");
            assert_eq!(effect["category"], category);
            assert_eq!(effect["maximumLevel"], 77);
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], 2);
            assert!(effect.get("countBonus").is_none());
        }
        assert!(
            map_spell_token(
                "S_PANTHEON",
                77,
                3,
                "demo.actor.someone-else",
                &mut abilities,
            )
            .is_none()
        );
    }

    #[test]
    fn p76_spell_mappings_keep_unique_family_air_chicken_and_no_air_semantics() {
        let mut abilities = BTreeMap::new();

        let unique = map_spell_token(
            "S_UNIQUE",
            83,
            3,
            "demo.actor.ptah-the-divine-craftsman",
            &mut abilities,
        )
        .expect("S_UNIQUE should map");
        assert_eq!(abilities[&unique]["effect"]["category"], "unique");
        assert_eq!(abilities[&unique]["effect"]["maximumLevel"], 83);
        assert_eq!(abilities[&unique]["effect"]["countDice"], 1);
        assert_eq!(abilities[&unique]["effect"]["countSides"], 2);

        let family = map_spell_token(
            "S_SPECIAL",
            86,
            3,
            "demo.actor.athena-the-goddess-of-wisdom",
            &mut abilities,
        )
        .expect("Athena family summon should map");
        assert_eq!(abilities[&family]["effect"]["type"], "no-op");
        assert!(
            abilities[&family]["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "monster-family-summon") })
        );

        let air = map_spell_token(
            "BR_AIR",
            86,
            3,
            "demo.actor.vayu-the-embodied-wind",
            &mut abilities,
        )
        .expect("BR_AIR should map");
        assert_eq!(abilities[&air]["effect"]["type"], "breath-damage");
        assert_eq!(abilities[&air]["effect"]["damageType"], "force");
        assert_eq!(abilities[&air]["effect"]["hpPercent"], 17);
        assert_eq!(abilities[&air]["effect"]["maxDamage"], 250);

        let chicken = map_spell_token(
            "CHICKEN(200)",
            83,
            3,
            "demo.actor.aijem-the-walrus",
            &mut abilities,
        )
        .expect("CHICKEN should map");
        assert_eq!(abilities[&chicken]["effect"]["type"], "damage");
        assert_eq!(abilities[&chicken]["effect"]["damageDice"], 1);
        assert_eq!(abilities[&chicken]["effect"]["damageSides"], 1);
        assert_eq!(abilities[&chicken]["effect"]["damageBonus"], 199);

        let no_air = map_spell_token(
            "NO_AIR",
            86,
            3,
            "demo.actor.vayu-the-embodied-wind",
            &mut abilities,
        )
        .expect("NO_AIR should map");
        assert_eq!(
            abilities[&no_air]["effect"]["statusKindId"],
            "rfb.status.no-air"
        );
        assert_eq!(abilities[&no_air]["effect"]["durationTicks"], 40);

        let pantheon = map_spell_token(
            "S_PANTHEON",
            86,
            3,
            "demo.actor.aphrodite-the-goddess-of-love",
            &mut abilities,
        )
        .expect("Aphrodite pantheon summon should map");
        assert_eq!(abilities[&pantheon]["effect"]["category"], "olympian");

        let serpent = map_spell_token(
            "S_SERPENT",
            94,
            3,
            "demo.actor.shiva-the-destroyer",
            &mut abilities,
        )
        .expect("S_SERPENT should map");
        assert_eq!(abilities[&serpent]["effect"]["category"], "kin-glyph-74");
    }

    #[test]
    fn p76_contact_effects_preserve_stun_time_and_unlife_without_fake_damage() {
        let mut monsters = parse_r_info(
            "N:1:test P76 auras\nG:P:r\nI:130:70d100:30:120:0:100\nW:90:3:999:30000:0:0\nB:HIT:HURT(1d1)\nA:HOLY_FIRE(4d4):STUN(1d3, 50%):TIME(1d3, 20%):UNLIFE(2d6, 50%)\n",
        )
        .expect("synthetic P76 aura monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "test-p76-auras".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("P76 contact effects should import");

        assert_eq!(actor["contactAuras"][0]["damageType"], "holy-fire");
        assert_eq!(actor["contactAuras"][1]["damageType"], "time");
        assert_eq!(actor["contactAuras"][1]["ravagesTime"], true);
        assert_eq!(
            actor["contactEffects"],
            serde_json::json!([
                {"type": "stun", "durationDice": 1, "durationSides": 3, "chancePercent": 50},
                {"type": "unlife", "amountDice": 2, "amountSides": 6, "chancePercent": 50}
            ])
        );
    }

    #[test]
    fn p77_guardian_and_dead_unique_summons_keep_their_narrow_semantics() {
        let mut abilities = BTreeMap::new();
        let guardian = map_spell_token(
            "S_GUARDIAN",
            100,
            3,
            "demo.actor.the-serpent-of-chaos",
            &mut abilities,
        )
        .expect("S_GUARDIAN should map");
        assert_eq!(abilities[&guardian]["effect"]["category"], "guardian");
        assert_eq!(abilities[&guardian]["effect"]["countDice"], 1);
        assert_eq!(abilities[&guardian]["effect"]["countSides"], 2);
        assert_eq!(abilities[&guardian]["effect"]["maximumLevel"], 99);

        let dead_unique = map_spell_token(
            "S_DEAD_UNIQ",
            100,
            3,
            "demo.actor.the-resurrection-machine",
            &mut abilities,
        )
        .expect("S_DEAD_UNIQ should map");
        assert_eq!(abilities[&dead_unique]["effect"]["category"], "unique");
        assert_eq!(abilities[&dead_unique]["effect"]["countDice"], 1);
        assert_eq!(abilities[&dead_unique]["effect"]["countSides"], 2);
        assert!(
            abilities[&dead_unique]["tags"]
                .as_array()
                .expect("ability tags")
                .iter()
                .any(|tag| tag == "monster-dead-unique-summon")
        );
    }

    #[test]
    fn p77_guardian_and_serpent_contact_auras_import_without_omission() {
        let mut monsters = parse_r_info(
            "N:862:The Serpent of Chaos\nG:J:v\nI:155:200d150:25:130:0:100\nW:100:1:999:300000:0:0\nB:CRUSH:SHATTER(19d10)\nA:SHARDS(4d6):CHAOS(6d6, 20%):DISENCHANT(3d3, 10%)\nF:GUARDIAN | FIXED_UNIQUE\nS:1_IN_3 | S_GUARDIAN\n",
        )
        .expect("synthetic Serpent should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 862,
                source_id: None,
                id: "the-serpent-of-chaos".to_owned(),
                tags: vec!["fixed-placement".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("P77 Serpent mechanics should import");

        assert!(
            actor["tags"]
                .as_array()
                .expect("actor tags")
                .iter()
                .any(|tag| tag == "guardian")
        );
        assert_eq!(
            actor["contactAuras"],
            serde_json::json!([
                {"damageDice": 4, "damageSides": 6, "damageType": "shards"},
                {"chancePercent": 20, "damageDice": 6, "damageSides": 6, "damageType": "chaos"},
                {"chancePercent": 10, "damageDice": 3, "damageSides": 3, "damageType": "disenchant"}
            ])
        );
        assert!(actor.get("allocation").is_none());
    }

    #[test]
    fn p76_artemis_is_the_narrow_no_melee_family_summoner_exception() {
        let mut monsters = parse_r_info(
            "N:1103:Artemis, the Moon Goddess\nG:P:s\nI:130:70d100:30:120:0:100\nW:86:3:999:30000:0:0\nS:1_IN_3 | S_SPECIAL\n",
        )
        .expect("synthetic Artemis should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 1103,
                source_id: None,
                id: "artemis-the-moon-goddess".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("Artemis should import without invented blows");
        assert_eq!(
            actor["meleeRoutine"]["blows"],
            serde_json::json!([]),
            "Artemis keeps an explicitly empty routine instead of invented blows"
        );
    }

    #[test]
    fn fixed_placement_monsters_do_not_keep_random_allocation() {
        let mut monsters = parse_r_info(
            "N:732:Bull Gates\nG:p:D\nI:140:25d100:40:90:0:140\nW:52:3:999:18000:0:0\nB:CHARGE:HURT(5d5)\nF:FIXED_UNIQUE\nS:1_IN_6 | S_SPECIAL\n",
        )
        .expect("synthetic fixed monster should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 732,
                source_id: None,
                id: "bull-gates".to_owned(),
                tags: vec!["fixed-placement".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("fixed Bull Gates should import");

        assert!(actor.get("allocation").is_none());
    }

    #[test]
    fn bird_drop_maps_to_its_dedicated_monster_effect() {
        let mut abilities = BTreeMap::new();
        let id = map_spell_token(
            "BIRD_DROP",
            52,
            4,
            "demo.actor.the-ancient-roc-of-okeldad",
            &mut abilities,
        )
        .expect("bird drop should map");
        assert_eq!(id, "rfb-legacy.ability.bird-drop");
        assert_eq!(abilities[&id]["effect"]["type"], "bird-drop");
        assert_eq!(
            abilities[&id]["target"]["modes"],
            serde_json::json!(["position", "entity"])
        );
        assert_eq!(abilities[&id]["target"]["range"], 8);
        assert_eq!(abilities[&id]["target"]["requiresLineOfEffect"], true);
    }

    #[test]
    fn k_info_items_map_across_the_expressible_shapes() {
        const SYNTHETIC_K_INFO: &str = "V:1.1.0
N:0:something
G:&:w
N:1:Test Long Sword~
G:|:W
I:23:17:0
W:10:0:0:130:300
P:0:2d6:1:2:0
F:SHOW_MODS | TOWN
N:*:& Test Short Bow~
G:}:u
I:19:12:0
W:3:0:0:30:50
P:0:x2.00:0:0:0
N:*:Test Arrow~
G:{:U
I:17:1:0
W:3:0:0:2:1
P:0:1d4:0:0:0
N:*:Test Chain Mail~
G:[:s
I:37:4:0
W:20:0:0:220:750
P:14:1d4:-2:0:0
F:RES_ACID | FREE_ACT | IGNORE_FIRE
N:*:& Test Ring~
G:=:y
I:45:20:0
W:30:0:0:2:500
F:HIDE_TYPE
N:*:& Test Potion~ of Mending
G:!:b
I:75:32:0
W:5:0:0:4:20
N:*:& Test Torch~
G:~:u
I:39:0:5000
W:1:0:0:30:2
N:*:& Test Harp~
G:}:y
I:19:70:0
W:5:0:0:150:80
";
        let items = parse_k_info(SYNTHETIC_K_INFO).expect("synthetic items should parse");
        assert_eq!(items.len(), 9);

        let outcome = convert_content(
            &[],
            &[],
            &items,
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.items_total, 9);
        assert_eq!(outcome.report.items_imported, 9);
        assert_eq!(outcome.report.items_skipped, 1);

        let get = |name: &str| {
            outcome
                .item_files
                .iter()
                .find(|(file, _)| file == name)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("{name} should be generated"))
        };

        let sword = get("test-long-sword.json");
        assert_eq!(sword["equipmentSlot"], "weapon");
        assert_eq!(sword["maxStack"], 1);
        assert_eq!(sword["weightTenthsPound"], 130);
        assert_eq!(sword["baseValue"], 300);
        assert_eq!(sword["meleeProfile"]["damageDice"], 2);
        assert_eq!(sword["meleeProfile"]["damageSides"], 6);
        assert_eq!(sword["meleeProfile"]["toHit"], 1);
        assert_eq!(sword["meleeProfile"]["toDamage"], 2);

        let bow = get("test-short-bow.json");
        assert_eq!(bow["equipmentSlot"], "launcher");
        assert_eq!(bow["projectileProfile"]["damageMultiplierPercent"], 200);
        assert_eq!(bow["projectileProfile"]["range"], 15);
        assert_eq!(bow["projectileProfile"]["ammunitionType"], "arrow");

        let arrow = get("test-arrow.json");
        assert!(arrow.get("equipmentSlot").is_none());
        assert_eq!(arrow["maxStack"], 99);
        assert_eq!(arrow["breakChancePercent"], 20);
        assert_eq!(arrow["ammunitionProfile"]["damageDice"], 1);
        assert_eq!(arrow["ammunitionProfile"]["damageSides"], 4);
        assert_eq!(arrow["ammunitionProfile"]["ammunitionType"], "arrow");

        // Inherent defensive flags fold onto the base item (dragon scale
        // style); elemental durability flags map to destruction immunity.
        let mail = get("test-chain-mail.json");
        assert_eq!(mail["equipmentSlot"], "body");
        assert_eq!(mail["modifiers"]["defense"], 14);
        assert_eq!(mail["resistances"]["acid"], "resistant");
        assert_eq!(mail["statusImmunities"][0], "rfb.status.paralysis");
        assert!(!outcome.report.unmapped_item_flags.contains_key("RES_ACID"));
        assert!(!outcome.report.unmapped_item_flags.contains_key("FREE_ACT"));
        assert_eq!(
            mail["elementalDestructionImmunities"],
            serde_json::json!(["fire"])
        );
        assert!(
            !outcome
                .report
                .not_applicable_item_flags
                .contains_key("IGNORE_FIRE")
        );

        // Base jewelry is a generic shell: attributes and pval only arrive
        // via egos or fixed artifacts, mirroring the legacy generation model.
        let ring = get("test-ring.json");
        assert_eq!(ring["equipmentSlot"], "ring");
        let corpse = get("corpse-remains.json");
        assert_eq!(corpse["id"], LEGACY_CORPSE_ITEM_ID);
        assert_eq!(corpse["maxStack"], 1);
        assert!(ring.get("modifiers").is_none());
        assert_eq!(outcome.report.not_applicable_item_flags["HIDE_TYPE"], 1);
        assert_eq!(outcome.report.item_behavior_gaps["effect-jewelry"], 1);

        let potion = get("test-potion-of-mending.json");
        assert_eq!(potion["maxStack"], 20);
        assert_eq!(potion["useAction"]["effect"]["type"], "apply-heroism");
        assert!(
            !outcome
                .report
                .item_behavior_gaps
                .contains_key("consumable-effect")
        );

        let torch = get("test-torch.json");
        assert_eq!(torch["equipmentSlot"], "light");
        assert_eq!(torch["maxStack"], 1);
        assert!(
            torch["tags"]
                .as_array()
                .expect("torch tags")
                .iter()
                .any(|tag| tag == "light-source")
        );

        // Unpaired launchers stay equippable fake bows: slot without a
        // projectile profile, so they occupy the slot but cannot fire.
        let harp = get("test-harp.json");
        assert_eq!(harp["equipmentSlot"], "launcher");
        assert!(harp.get("projectileProfile").is_none());
        assert_eq!(outcome.report.item_behavior_gaps["launcher-unpaired"], 1);
    }

    #[test]
    fn k_info_allocations_drive_legacy_loot_entries() {
        const SYNTHETIC_K_INFO: &str = "N:1:Allocated Sword
G:|:w
I:23:17:0
W:42:0:60:100:200
A:5/2:20/4:30/255
N:2:Open Ended Potion
G:!:b
I:75:1:0
W:7:0:0:4:10
A:3/5
N:3:Unallocated Potion
G:!:r
I:75:2:0
W:1:0:0:4:5
";
        let items = parse_k_info(SYNTHETIC_K_INFO).expect("allocations should parse");
        assert_eq!(items[0].level, 42);
        assert_eq!(items[0].max_level, 60);
        assert_eq!(
            items[0].allocations,
            vec![
                LegacyItemAllocation {
                    level: 5,
                    chance: 2
                },
                LegacyItemAllocation {
                    level: 20,
                    chance: 4
                },
                LegacyItemAllocation {
                    level: 30,
                    chance: 255
                },
            ]
        );
        assert!(items[2].allocations.is_empty());

        let outcome = convert_content(
            &[],
            &[],
            &items,
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let item = outcome
            .item_files
            .iter()
            .find(|(name, _)| name == "allocated-sword.json")
            .map(|(_, value)| value)
            .expect("allocated item should import");
        assert_eq!(item["generationLevel"], 42);

        let entries = outcome
            .loot_table_files
            .iter()
            .find(|(name, _)| name == "monster-drops.json")
            .and_then(|(_, table)| table["entries"].as_array())
            .expect("legacy loot table should import");
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0]["minDepth"], 5);
        assert_eq!(entries[0]["weight"], 50);
        assert_eq!(entries[0]["maxDepth"], 60);
        assert_eq!(entries[1]["minDepth"], 20);
        assert_eq!(entries[1]["weight"], 25);
        assert_eq!(entries[2]["weight"], 0);
        assert_eq!(entries[3]["minDepth"], 3);
        assert_eq!(entries[3]["weight"], 20);
        assert!(entries[3].get("maxDepth").is_none());
        assert!(
            entries
                .iter()
                .all(|entry| { entry["itemKindId"] != "rfb-legacy.item.unallocated-potion" })
        );
        let warrior_entries = outcome
            .loot_table_files
            .iter()
            .find(|(name, _)| name == "monster-drops-warrior.json")
            .and_then(|(_, table)| table["entries"].as_array())
            .expect("legacy Warrior loot table should import");
        assert_eq!(warrior_entries.len(), 3);
        assert!(
            warrior_entries
                .iter()
                .all(|entry| entry["itemKindId"] == "rfb-legacy.item.allocated-sword")
        );
    }

    #[test]
    fn legacy_warrior_theme_uses_the_original_kind_predicate() {
        const SYNTHETIC_K_INFO: &str = "N:1:Arrow
G:|:w
I:17:1:0
W:1:0:0:1:1
A:1/1
N:2:Soft Armor
G:[:w
I:36:1:0
W:1:0:0:1:1
A:1/1
N:3:Ring
G:=:w
I:45:0:0
W:1:0:0:1:1
A:1/1
N:4:Amulet
G:\":w
I:40:0:0
W:1:0:0:1:1
A:1/1
N:5:Polearm
G:/:w
I:22:1:0
W:1:0:0:1:1
A:1/1
N:6:Short Sword
G:|:w
I:23:10:0
W:1:0:0:1:1
A:1/1
N:7:Sabre
G:|:w
I:23:11:0
W:1:0:0:1:1
A:1/1
N:8:Heroism
G:!:w
I:75:32:0
W:1:0:0:1:1
A:1/1
";
        let items = parse_k_info(SYNTHETIC_K_INFO).expect("theme kinds should parse");
        let outcome = convert_content(
            &[],
            &[],
            &items,
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let item_ids = outcome
            .loot_table_files
            .iter()
            .find(|(name, _)| name == "monster-drops-warrior.json")
            .and_then(|(_, table)| table["entries"].as_array())
            .expect("legacy Warrior loot table should import")
            .iter()
            .map(|entry| entry["itemKindId"].as_str().expect("item ID"))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            item_ids,
            BTreeSet::from([
                "rfb-legacy.item.amulet",
                "rfb-legacy.item.heroism",
                "rfb-legacy.item.polearm",
                "rfb-legacy.item.ring",
                "rfb-legacy.item.sabre",
            ])
        );
    }

    #[test]
    fn legacy_armor_tvals_map_to_the_original_body_slots() {
        let expected = [
            (30, "boots"),
            (31, "gloves"),
            (32, "head"),
            (33, "head"),
            (34, "shield"),
            (35, "cloak"),
        ];

        for (tval, slot) in expected {
            assert_eq!(item_shape(tval).and_then(|shape| shape.slot), Some(slot));
        }
    }

    #[test]
    fn armor_hit_and_glove_damage_modifiers_become_melee_equipment_bonuses() {
        let armor = LegacyItemEntry {
            name: "Hard Leather Armour".to_owned(),
            glyph: Some('('),
            tval: 36,
            weight_tenths_pound: 100,
            base_value: 150,
            armor_class: 6,
            to_hit: -1,
            flags: vec!["TOWN".to_owned()],
            ..LegacyItemEntry::default()
        };
        let gloves = LegacyItemEntry {
            name: "Studded Leather Gloves".to_owned(),
            glyph: Some(']'),
            tval: 31,
            weight_tenths_pound: 5,
            base_value: 3,
            armor_class: 1,
            to_damage: 1,
            flags: vec!["TOWN".to_owned()],
            ..LegacyItemEntry::default()
        };

        let armor = demo_item_json(&armor, "hard-leather-armour", &LauncherAmmoIndex::default())
            .expect("armor hit modifier should be behavior-complete");
        assert_eq!(armor["equipmentBonuses"]["meleeSkill"], -1);
        assert!(armor["equipmentBonuses"].get("meleeDamage").is_none());
        assert!(armor.get("meleeProfile").is_none());

        let gloves = demo_item_json(
            &gloves,
            "studded-leather-gloves",
            &LauncherAmmoIndex::default(),
        )
        .expect("glove damage modifier should be behavior-complete");
        assert_eq!(gloves["equipmentBonuses"]["meleeDamage"], 1);
        assert!(gloves["equipmentBonuses"].get("meleeSkill").is_none());
        assert!(gloves.get("meleeProfile").is_none());
    }

    #[test]
    fn authoritative_unbrandable_weapons_receive_a_content_tag() {
        for (sval, name, flags) in [
            (32, "Poison Needle", Vec::new()),
            (34, "Rune Sword", Vec::new()),
            (1, "Sticky Blade", vec!["NO_REMOVE".to_owned()]),
        ] {
            let item = item_json(
                &LegacyItemEntry {
                    name: name.to_owned(),
                    glyph: Some('|'),
                    tval: 23,
                    sval,
                    weight_tenths_pound: 10,
                    flags,
                    ..LegacyItemEntry::default()
                },
                &kebab(name),
                &LauncherAmmoIndex::default(),
                None,
                &mut ContentImportReport::default(),
            );
            assert!(
                item["tags"]
                    .as_array()
                    .expect("item tags")
                    .iter()
                    .any(|tag| tag == "unbrandable")
            );
        }
    }

    #[test]
    fn demo_item_selection_can_keep_a_stable_id_distinct_from_the_source_name() {
        let selection: DemoItemSelection = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "items": [{
                "sourceIndex": 227,
                "sourceId": "set-of-leather-gloves",
                "id": "leather-gloves"
            }]
        }))
        .expect("selection alias should parse");

        let entry = &selection.items[0];
        assert_eq!(entry.expected_source_id(), "set-of-leather-gloves");
        assert_eq!(entry.id, "leather-gloves");
    }

    #[test]
    fn demo_monster_selection_can_keep_a_stable_id_distinct_from_the_source_name() {
        let selection: DemoMonsterSelection = serde_json::from_value(serde_json::json!({
            "schemaVersion": 1,
            "monsters": [{
                "sourceIndex": 135,
                "sourceId": "mughash-the-kobold-lord",
                "id": "warrens-keeper",
                "tags": ["warrens"]
            }]
        }))
        .expect("selection alias should parse");

        let entry = &selection.monsters[0];
        assert_eq!(entry.expected_source_id(), "mughash-the-kobold-lord");
        assert_eq!(entry.id, "warrens-keeper");
    }

    #[test]
    fn demo_item_coverage_deduplicates_source_items_and_counts_originals() {
        let entries = vec![
            LegacyItemEntry {
                index: 1,
                name: "Dagger".to_owned(),
                glyph: Some('|'),
                tval: 23,
                ..LegacyItemEntry::default()
            },
            LegacyItemEntry {
                index: 2,
                name: "Staff".to_owned(),
                glyph: Some('_'),
                tval: 55,
                ..LegacyItemEntry::default()
            },
            LegacyItemEntry {
                index: 3,
                name: "Slowness".to_owned(),
                glyph: Some('!'),
                tval: 75,
                sval: 4,
                ..LegacyItemEntry::default()
            },
            LegacyItemEntry {
                index: 4,
                name: "Unmapped Book".to_owned(),
                glyph: Some('?'),
                tval: 90,
                ..LegacyItemEntry::default()
            },
        ];
        let selection = DemoItemSelection {
            schema_version: 1,
            items: vec![DemoItemSelectionEntry {
                source_index: 1,
                source_id: None,
                id: "dagger".to_owned(),
            }],
        };
        let adaptations = DemoItemAdaptationLedger {
            schema_version: 1,
            items: ["detect-staff", "identify-staff"]
                .into_iter()
                .map(|id| DemoItemAdaptation {
                    source_index: 2,
                    source_name: "Staff".to_owned(),
                    source_id: "staff".to_owned(),
                    item_id: format!("demo.item.{id}"),
                    status: DemoItemCoverageStatus::Active,
                    blocker: None,
                    adaptation: None,
                    contract: "contract-test".to_owned(),
                })
                .collect(),
        };
        let formal_ids = [
            "demo.item.dagger",
            "demo.item.detect-staff",
            "demo.item.identify-staff",
            "demo.item.original",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        let mut plan = DemoItemPlan {
            schema_version: 1,
            baseline: DemoItemPlanBaseline {
                source_commit: "baseline-commit".to_owned(),
                source_items_total: 4,
                active_source_items: 1,
                mechanics_ready_source_items: 1,
                blocked_source_items: 2,
                formal_items_total: 2,
                mapped_formal_items: 1,
                original_formal_items: 1,
            },
            batches: vec![DemoItemPlanBatch {
                id: "p3.1".to_owned(),
                families: vec![DemoItemPlanFamily {
                    id: "devices".to_owned(),
                    primary_blockers: vec!["device-system".to_owned()],
                    items: vec![DemoItemPlanEntry {
                        source_index: 2,
                        source_name: "Staff".to_owned(),
                        source_id: "staff".to_owned(),
                        secondary_blockers: Vec::new(),
                        completed_requirements: Vec::new(),
                    }],
                }],
            }],
        };

        let error = build_demo_item_coverage_report(
            "test-commit",
            &entries,
            &selection,
            &adaptations,
            &plan,
            &formal_ids,
            &TerrainCreationImportIds::default(),
        )
        .expect_err("active planned item should require completion evidence");
        assert!(
            error
                .to_string()
                .contains("missing completion requirements")
        );
        plan.batches[0].families[0].items[0].completed_requirements = DEMO_ITEM_ACTIVE_REQUIREMENTS
            .into_iter()
            .map(str::to_owned)
            .collect();

        let report = build_demo_item_coverage_report(
            "test-commit",
            &entries,
            &selection,
            &adaptations,
            &plan,
            &formal_ids,
            &TerrainCreationImportIds::default(),
        )
        .expect("coverage should be valid");

        assert_eq!(report.source_items_total, 4);
        assert_eq!(report.active_source_items, 2);
        assert_eq!(report.mechanics_ready_source_items, 0);
        assert_eq!(report.blocked_source_items, 2);
        assert_eq!(report.mapped_formal_items, 3);
        assert_eq!(report.original_formal_items, 1);
        assert_eq!(report.blocker_counts["book-system"], 1);
        assert_eq!(report.blocker_counts["potion-shatter-effect"], 1);
        assert_eq!(report.original_item_ids, ["demo.item.original"]);
        assert_eq!(report.p3_plan.formal_items_delta, 2);
        assert_eq!(report.p3_plan.mapped_rfb_formal_items_delta, 2);
        assert_eq!(report.p3_plan.original_formal_items_delta, 0);
        assert_eq!(report.p3_plan.batches[0].new_rfb_formal_items, 2);
        assert_eq!(report.p3_plan.batches[0].blocked_to_active.len(), 1);
    }

    #[test]
    fn item_effect_program_extraction_emits_flat_typed_references() {
        let mut items = vec![
            (
                "clarity.json".to_owned(),
                serde_json::json!({
                    "id": "rfb-legacy.item.clarity",
                    "useAction": {
                        "effect": {
                            "type": "sequence",
                            "effects": [
                                { "type": "restore-resource-full", "resourceId": LEGACY_MANA_RESOURCE_ID },
                                { "type": "remove-status", "statusKindId": "rfb.status.confusion" }
                            ]
                        }
                    }
                }),
            ),
            (
                "wand.json".to_owned(),
                serde_json::json!({
                    "id": "rfb-legacy.item.wand",
                    "deviceGeneration": {
                        "activations": [{
                            "id": "rfb-legacy.device-activation.frost-bolt",
                            "effect": {
                                "type": "damage",
                                "damageDice": 3,
                                "damageSides": 6,
                                "damageType": "cold"
                            }
                        }]
                    }
                }),
            ),
            (
                "venom.json".to_owned(),
                serde_json::json!({
                    "id": "rfb-legacy.item.venom",
                    "shatterEffect": {
                        "type": "damage",
                        "damageDice": 0,
                        "damageSides": 0,
                        "damageBonus": 3,
                        "damageType": "poison"
                    },
                    "shatterRadius": 2
                }),
            ),
        ];

        let programs = extract_item_effect_programs(&mut items)
            .expect("item effects should extract into source programs");
        assert_eq!(
            items[0].1["useAction"]["effectProgramId"],
            "rfb-legacy.effect.clarity.use"
        );
        assert!(items[0].1["useAction"].get("effect").is_none());
        assert_eq!(
            items[1].1["deviceGeneration"]["activations"][0]["effectProgramId"],
            "rfb-legacy.effect.wand.frost-bolt"
        );
        assert!(
            items[1].1["deviceGeneration"]["activations"][0]
                .get("effect")
                .is_none()
        );

        assert_eq!(programs.len(), 3);
        let clarity = programs
            .iter()
            .find(|(name, _)| name == "clarity-use.json")
            .map(|(_, value)| value)
            .expect("clarity program should be emitted");
        assert_eq!(clarity["input"], "self");
        assert_eq!(clarity["steps"].as_array().map(Vec::len), Some(2));
        assert_eq!(clarity["steps"][0]["type"], "restore-resource-full");
        assert!(clarity.get("effect").is_none());

        let frost = programs
            .iter()
            .find(|(name, _)| name == "wand-frost-bolt.json")
            .map(|(_, value)| value)
            .expect("device program should be emitted");
        assert_eq!(frost["input"], "actor");
        assert_eq!(frost["steps"].as_array().map(Vec::len), Some(1));
        assert_eq!(frost["steps"][0]["type"], "damage");

        assert_eq!(
            items[2].1["shatterEffectProgramId"],
            "rfb-legacy.effect.venom.shatter"
        );
        assert!(items[2].1.get("shatterEffect").is_none());
        let venom = programs
            .iter()
            .find(|(name, _)| name == "venom-shatter.json")
            .map(|(_, value)| value)
            .expect("shatter program should be emitted");
        assert_eq!(venom["input"], "area");
        assert_eq!(venom["steps"][0]["damageType"], "poison");
        assert_eq!(venom["steps"][0]["damageDice"], 0);
        assert_eq!(venom["steps"][0]["damageBonus"], 3);
    }

    #[test]
    fn item_destruction_properties_follow_original_tvals_and_ignore_flags() {
        assert_eq!(item_destruction_vulnerabilities(23), ["acid", "fire"]);
        assert_eq!(item_destruction_vulnerabilities(45), ["electricity"]);
        assert_eq!(item_destruction_vulnerabilities(75), ["cold"]);
        assert_eq!(item_destruction_vulnerabilities(94), ["fire"]);
        assert!(item_destruction_vulnerabilities(10).is_empty());
        assert_eq!(
            item_destruction_immunities(&["IGNORE_ACID".to_owned(), "IGNORE_COLD".to_owned()]),
            ["acid", "cold"]
        );
    }

    #[test]
    fn potion_shatter_mapping_keeps_drinking_and_breakage_distinct() {
        let venom = LegacyItemEntry {
            tval: 75,
            sval: 6,
            ..LegacyItemEntry::default()
        };
        let (effect, radius) = potion_shatter_effect(&venom).expect("venom should shatter");
        assert_eq!(effect["type"], "damage");
        assert_eq!(effect["damageType"], "poison");
        assert_eq!(effect["damageDice"], 0);
        assert_eq!(effect["damageBonus"], 3);
        assert_eq!(radius, 2);

        let healing = LegacyItemEntry {
            tval: 75,
            sval: 37,
            ..LegacyItemEntry::default()
        };
        let (effect, radius) = potion_shatter_effect(&healing).expect("healing should shatter");
        assert_eq!(
            effect,
            serde_json::json!({"type": "heal-dice", "dice": 10, "sides": 10})
        );
        assert_eq!(radius, 2);

        let salt_water = LegacyItemEntry {
            tval: 75,
            sval: 5,
            ..LegacyItemEntry::default()
        };
        assert!(potion_shatter_effect(&salt_water).is_none());
        assert!(!potion_has_unimplemented_shatter_effect(&salt_water));
    }

    #[test]
    fn ability_extraction_emits_required_programs_and_casting_bindings() {
        fn ability(
            id: &str,
            target: serde_json::Value,
            effect: serde_json::Value,
        ) -> serde_json::Value {
            serde_json::json!({
                "id": id,
                "minimumLevel": 3,
                "resourceId": LEGACY_RESOURCE_ID,
                "resourceCost": 5,
                "baseFailurePercent": 20,
                "target": target,
                "effect": effect,
            })
        }

        let mut abilities = vec![
            (
                "book-spell.json".to_owned(),
                ability(
                    "rfb-legacy.ability.book-spell",
                    serde_json::json!({
                        "modes": ["self"],
                        "range": 0,
                        "requiresLineOfEffect": false,
                    }),
                    serde_json::json!({
                        "type": "sequence",
                        "effects": [
                            { "type": "heal", "amount": 4 },
                            { "type": "remove-status", "statusKindId": "rfb.status.fear" },
                        ],
                    }),
                ),
            ),
            (
                "innate.json".to_owned(),
                ability(
                    "rfb-legacy.ability.innate",
                    serde_json::json!({
                        "modes": ["entity"],
                        "range": 6,
                        "requiresLineOfEffect": true,
                    }),
                    serde_json::json!({
                        "type": "damage",
                        "damageDice": 1,
                        "damageSides": 4,
                        "damageType": "physical",
                    }),
                ),
            ),
            (
                "monster-only.json".to_owned(),
                ability(
                    "rfb-legacy.ability.monster-only",
                    serde_json::json!({
                        "modes": ["entity"],
                        "range": 6,
                        "requiresLineOfEffect": true,
                    }),
                    serde_json::json!({
                        "type": "damage",
                        "damageDice": 1,
                        "damageSides": 4,
                        "damageType": "physical",
                    }),
                ),
            ),
        ];
        let player_ability_ids = BTreeSet::from([
            "rfb-legacy.ability.book-spell".to_owned(),
            "rfb-legacy.ability.innate".to_owned(),
        ]);
        let extracted =
            extract_ability_programs_and_player_bindings(&mut abilities, &player_ability_ids)
                .expect("ability source policy should extract deterministically");
        let programs = extracted.program_files;
        let bindings = extracted.player_binding_files;

        assert_eq!(programs.len(), 3);
        assert_eq!(bindings.len(), 2);
        assert_eq!(
            abilities[0].1["abilityProgramId"],
            "rfb-legacy.ability-program.book-spell"
        );
        assert!(abilities[0].1.get("effect").is_none());
        assert!(abilities[0].1.get("minimumLevel").is_none());
        assert!(abilities[2].1.get("minimumLevel").is_none());

        let book_program = programs
            .iter()
            .find(|(name, _)| name == "book-spell.json")
            .map(|(_, value)| value)
            .expect("book Program should be emitted");
        assert_eq!(book_program["input"], "self");
        assert_eq!(book_program["steps"].as_array().map(Vec::len), Some(2));
        let monster_program = programs
            .iter()
            .find(|(name, _)| name == "monster-only.json")
            .map(|(_, value)| value)
            .expect("monster Program should be emitted");
        assert_eq!(monster_program["input"], "cast-target");

        let book_binding = bindings
            .iter()
            .find(|(name, _)| name == "book-spell.json")
            .map(|(_, value)| value)
            .expect("book player binding should be emitted");
        assert_eq!(book_binding["abilityId"], "rfb-legacy.ability.book-spell");
        assert_eq!(book_binding["resourceCost"], 5);
        assert!(bindings.iter().all(|(name, _)| name != "monster-only.json"));
    }

    #[test]
    fn b_info_bodies_map_slots_with_gap_census() {
        const SYNTHETIC_B_INFO: &str = "V:1.1.0
N:0:Standard
S:WEAPON_SHIELD:Right Hand:0
S:WEAPON_SHIELD:Left Hand:1
S:BOW:Shooting
S:QUIVER:Back
S:RING:Right Ring:0
S:RING:Left Ring:1
S:AMULET:Neck
S:LITE:Light
S:BODY_ARMOR:Body
S:CLOAK:Cloak
S:HELMET:Head
S:GLOVES:Hands
S:BOOTS:Feet

N:4:Snake
S:RING:Ring
S:RING:Ring
S:RING:Ring
S:RING:Ring
S:AMULET:Amulet
S:ANY:Slot
";
        let bodies = parse_b_info(SYNTHETIC_B_INFO).expect("synthetic bodies should parse");
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0].slots.len(), 13);

        let characters = LegacyCharacterSources {
            bodies,
            races: vec![LegacyCharacterEntry {
                id: "test-folk".to_owned(),
                skills: [1, 0, 2, 0, 0, 10, 0, 0],
                stats: [1, 0, 0, -1, 0, 0],
                life: 98,
                base_hp: 22,
                exp: 120,
                infra: 3,
                ..LegacyCharacterEntry::default()
            }],
            personalities: Vec::new(),
            ..LegacyCharacterSources::default()
        };
        let outcome = convert_content(&[], &[], &[], &[], &[], &characters);
        assert_eq!(outcome.report.bodies_total, 2);
        assert_eq!(outcome.report.races_imported, 1);
        // Census over every template: quiver from Standard, any from Snake.
        assert_eq!(outcome.report.body_slot_gaps["body-slot-quiver"], 1);
        assert_eq!(outcome.report.body_slot_gaps["body-slot-any"], 1);

        let (name, race) = &outcome.race_files[0];
        assert_eq!(name, "test-folk.json");
        assert_eq!(race["id"], "rfb-legacy.race.test-folk");
        assert_eq!(race["lifePercent"], 98);
        assert_eq!(race["experiencePercent"], 120);
        assert_eq!(race["baseHp"], 22);
        assert_eq!(race["infravision"], 3);
        assert_eq!(race["kinCategory"], "kin-glyph-112");
        assert_eq!(race["modifiers"]["strength"], 1);
        assert_eq!(race["modifiers"]["dexterity"], -1);
        let slots = race["bodySlots"].as_array().expect("race body slots");
        assert_eq!(slots.len(), 12);
        assert_eq!(slots[0]["id"], "weapon");
        assert_eq!(slots[1]["id"], "shield");
        assert_eq!(slots[2]["slotType"], "launcher");
        assert!(slots.iter().any(|slot| slot["id"] == "ring-1"));
        assert!(slots.iter().any(|slot| slot["id"] == "ring-2"));
        assert!(slots.iter().any(|slot| slot["slotType"] == "light"));
        // The RFB-original charm slot is deliberately absent from legacy
        // bodies.
        assert!(!slots.iter().any(|slot| slot["slotType"] == "charm"));

        let (set_name, skill_set) = &outcome.skill_set_files[0];
        assert_eq!(set_name, "race-test-folk.json");
        let entries = skill_set["entries"].as_array().expect("skill entries");
        assert_eq!(entries.len(), 3);
        assert!(entries.iter().any(|entry| {
            entry["skillId"] == "rfb-legacy.skill.perception" && entry["base"] == 10
        }));
        assert_eq!(outcome.skill_files.len(), 8);
    }

    #[test]
    fn character_blocks_extract_regular_races_and_skip_dynamic_ones() {
        const SYNTHETIC_SOURCE: &str = r#"
static void _test_calc_bonuses(void)
{
    res_add(RES_FIRE);
    res_add(RES_FIRE);
    res_add_vuln(RES_LITE);
    res_add_immune(RES_ACID);
    /*res_add_vuln(RES_ELEC); disabled for parity checks*/
    p_ptr->free_act++;
    p_ptr->see_inv++;
    p_ptr->sustain_con = TRUE;
    p_ptr->pspeed += 3;
    p_ptr->pspeed += p_ptr->lev / 10;
    if (p_ptr->lev >= 45) res_add(RES_COLD);
    if (p_ptr->lev >= 40) p_ptr->see_inv++;
    if (p_ptr->lev >= 35) p_ptr->sustain_str = TRUE;
    if (p_ptr->lev >= 10)
        res_add(RES_POIS);
    if (p_ptr->lev >= 30)
    {
        res_add(RES_DARK);
        p_ptr->pspeed += 2;
    }
    p_ptr->hold_life++;
}

static void _conditional_calc_bonuses(void)
{
    if (p_ptr->lev >= 40) p_ptr->see_inv++;
}

race_t *test_folk_get_race(void)
{
    static race_t me = {0};
    static bool init = FALSE;

    if (!init)
    {
        me.name = "试验族";
        me.desc = "一段描述。";

        me.stats[A_STR] =  1;
        me.stats[A_INT] = -2;

        me.skills.dis = 3;
        me.skills.fos = 10;

        me.life = 102;
        me.base_hp = 24;
        me.exp = 135;
        me.infra = 3;
        me.shop_adjust = 100;

        me.flags = RACE_IS_NONLIVING | RACE_NO_POLY;
        me.calc_bonuses = _test_calc_bonuses;
        init = TRUE;
    }

    return &me;
}

race_t *test_beast_get_race(void)
{
    static race_t me = {0};

    me.life = 100 + 5*rank;
    me.skills.dis = 2;
    return &me;
}
"#;
        let blocks = extract_race_blocks(SYNTHETIC_SOURCE);
        assert_eq!(blocks.len(), 2);
        let folk = parse_character_block(&blocks[0].0, &blocks[0].1);
        assert_eq!(folk.id, "test-folk");
        assert!(!folk.dynamic);
        assert_eq!(folk.stats[0], 1);
        assert_eq!(folk.stats[1], -2);
        assert_eq!(folk.skills[0], 3);
        assert_eq!(folk.skills[5], 10);
        assert_eq!(folk.life, 102);
        assert_eq!(folk.base_hp, 24);
        assert_eq!(folk.exp, 135);
        assert_eq!(folk.infra, 3);
        assert_eq!(folk.flags, ["RACE_IS_NONLIVING", "RACE_NO_POLY"]);
        assert_eq!(folk.hooks, ["calc_bonuses"]);
        assert_eq!(folk.calc_bonuses_fn.as_deref(), Some("_test_calc_bonuses"));

        // The hook body yields only its top-level static statements: doubled
        // res_add stacks to strong, comments and level-gated branches are
        // ignored, and only the literal speed adjustment counts.
        let (resistances, free_act, see_invisible, attribute_sustains, speed) =
            parse_calc_bonuses_defenses(SYNTHETIC_SOURCE, "_test_calc_bonuses");
        assert_eq!(
            resistances,
            [
                ("acid".to_owned(), "immune".to_owned()),
                ("fire".to_owned(), "strong".to_owned()),
                ("light".to_owned(), "vulnerable".to_owned()),
            ]
        );
        assert!(free_act);
        assert!(see_invisible);
        assert_eq!(attribute_sustains, ["constitution"]);
        assert_eq!(speed, 3);
        let (_, _, conditional_see_invisible, conditional_sustains, _) =
            parse_calc_bonuses_defenses(SYNTHETIC_SOURCE, "_conditional_calc_bonuses");
        assert!(!conditional_see_invisible);
        assert!(conditional_sustains.is_empty());
        let mut folk = folk;
        folk.resistances = resistances;
        folk.free_act = free_act;
        folk.see_invisible = see_invisible;
        folk.attribute_sustains = attribute_sustains;
        folk.speed = speed;

        let beast = parse_character_block(&blocks[1].0, &blocks[1].1);
        assert!(beast.dynamic);

        let characters = LegacyCharacterSources {
            bodies: Vec::new(),
            races: vec![folk, beast],
            personalities: Vec::new(),
            ..LegacyCharacterSources::default()
        };
        let outcome = convert_content(&[], &[], &[], &[], &[], &characters);
        assert_eq!(outcome.report.races_total, 2);
        assert_eq!(outcome.report.races_imported, 1);
        assert_eq!(outcome.report.skip_reasons["race-code-dynamic"], 1);
        assert_eq!(outcome.report.unmapped_race_flags["RACE_IS_NONLIVING"], 1);
        assert_eq!(outcome.report.race_hook_gaps["calc_bonuses"], 1);
        assert!(!outcome.report.race_hook_gaps.contains_key("infra"));

        let (race_name, race) = &outcome.race_files[0];
        assert_eq!(race_name, "test-folk.json");
        assert_eq!(race["resistances"]["fire"], "strong");
        assert_eq!(race["resistances"]["acid"], "immune");
        assert_eq!(race["resistances"]["light"], "vulnerable");
        assert_eq!(race["statusImmunities"][0], "rfb.status.paralysis");
        assert_eq!(race["seeInvisible"], true);
        assert_eq!(
            race["attributeSustains"],
            serde_json::json!(["constitution"])
        );
        assert_eq!(race["modifiers"]["speed"], 3);
        assert_eq!(race["kinCategory"], "kin-glyph-112");
        let high_elf = LegacyCharacterEntry {
            id: "high-elf".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&high_elf),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "snow-adapted",
                "standard-body",
            ]
        );
        let dunadan = LegacyCharacterEntry {
            id: "dunadan".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&dunadan),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
        let hobbit = LegacyCharacterEntry {
            id: "hobbit".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&hobbit),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
        let kobold = LegacyCharacterEntry {
            id: "kobold".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&kobold),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
        let dwarf = LegacyCharacterEntry {
            id: "dwarf".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&dwarf),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
        let nibelung = LegacyCharacterEntry {
            id: "nibelung".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&nibelung),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
        let gnome = LegacyCharacterEntry {
            id: "gnome".to_owned(),
            ..LegacyCharacterEntry::default()
        };
        assert_eq!(
            legacy_race_tags(&gnome),
            [
                "humanoid",
                "legacy-import",
                "rfb-compatibility",
                "standard-body",
            ]
        );
    }

    #[test]
    fn literal_race_power_tables_map_known_spells_and_report_unknown_ones() {
        const SOURCE: &str = r#"
static power_info _barbarian_get_powers[] =
{
    { A_STR, {8, 10, 30, berserk_spell}},
    { A_INT, {15, 10, 50, create_food_spell}},
    { A_WIS, {5, 5, 50, detect_doors_stairs_traps_spell}},
    { A_CHR, {10, 5, 50, detect_treasure_spell}},
    { A_INT, {5, 2, 50, phase_door_spell}},
    { A_DEX, {12, 8, 50, poison_dart_spell}},
    { A_WIS, {12, 7, 40, mystery_spell}},
    { -1, {-1, -1, -1, NULL} }
};
static void _barbarian_calc_bonuses(void)
{
    res_add(RES_FEAR);
}
race_t *barbarian_get_race(void)
{
    static race_t me = {0};
    if (!init)
    {
        me.name = "野蛮人";
        me.calc_bonuses = _barbarian_calc_bonuses;
        me.get_powers = _barbarian_get_powers;
        init = TRUE;
    }
    return &me;
}
"#;
        let (name, body) = extract_race_blocks(SOURCE)
            .into_iter()
            .next()
            .expect("synthetic Barbarian should parse");
        let mut barbarian = parse_character_block(&name, &body);
        let (resistances, _, _, _, _) =
            parse_calc_bonuses_defenses(SOURCE, "_barbarian_calc_bonuses");
        barbarian.resistances = resistances;
        parse_race_powers(SOURCE, &mut barbarian);

        assert_eq!(
            barbarian.abilities,
            [
                LegacyInnatePower {
                    governing_attribute: "strength".to_owned(),
                    minimum_level: 8,
                    cost: 10,
                    base_failure_percent: 30,
                    ability_id: "rfb.ability.race.berserk".to_owned(),
                },
                LegacyInnatePower {
                    governing_attribute: "intelligence".to_owned(),
                    minimum_level: 15,
                    cost: 10,
                    base_failure_percent: 50,
                    ability_id: "rfb.ability.race.create-food".to_owned(),
                },
                LegacyInnatePower {
                    governing_attribute: "wisdom".to_owned(),
                    minimum_level: 5,
                    cost: 5,
                    base_failure_percent: 50,
                    ability_id: "rfb.ability.race.detect-doors-stairs-traps".to_owned(),
                },
                LegacyInnatePower {
                    governing_attribute: "charisma".to_owned(),
                    minimum_level: 10,
                    cost: 5,
                    base_failure_percent: 50,
                    ability_id: "rfb.ability.race.detect-treasure".to_owned(),
                },
                LegacyInnatePower {
                    governing_attribute: "intelligence".to_owned(),
                    minimum_level: 5,
                    cost: 2,
                    base_failure_percent: 50,
                    ability_id: "rfb.ability.race.phase-door".to_owned(),
                },
                LegacyInnatePower {
                    governing_attribute: "dexterity".to_owned(),
                    minimum_level: 12,
                    cost: 8,
                    base_failure_percent: 50,
                    ability_id: "rfb.ability.race.poison-dart".to_owned(),
                },
            ]
        );
        assert!(!barbarian.hooks.iter().any(|hook| hook == "get_powers"));
        assert!(
            barbarian
                .hooks
                .iter()
                .any(|hook| hook == "get_powers:mystery_spell")
        );

        let mut report = ContentImportReport::default();
        let race = race_json(&barbarian, &[], &mut report);
        assert_eq!(
            race["abilities"][0]["abilityId"],
            "rfb.ability.race.berserk"
        );
        assert_eq!(
            race["abilities"][1]["abilityId"],
            "rfb.ability.race.create-food"
        );
        assert_eq!(
            race["abilities"][2]["abilityId"],
            "rfb.ability.race.detect-doors-stairs-traps"
        );
        assert_eq!(
            race["abilities"][3]["abilityId"],
            "rfb.ability.race.detect-treasure"
        );
        assert_eq!(
            race["abilities"][4]["abilityId"],
            "rfb.ability.race.phase-door"
        );
        assert_eq!(
            race["abilities"][5]["abilityId"],
            "rfb.ability.race.poison-dart"
        );
        assert_eq!(race["abilities"].as_array().map(Vec::len), Some(6));
        assert_eq!(race["resistances"]["fear"], "resistant");
        assert!(
            race["tags"]
                .as_array()
                .is_some_and(|tags| tags.iter().any(|tag| tag == "rfb-compatibility"))
        );
        assert_eq!(report.race_hook_gaps["get_powers:mystery_spell"], 1);
        assert!(!report.race_hook_gaps.contains_key("get_powers"));
    }

    #[test]
    fn personality_blocks_extract_with_default_scalars() {
        const SYNTHETIC_SOURCE: &str = r#"
static personality_ptr _get_test_calm_personality(void)
{
    static personality_t me = {0};
    static bool init = FALSE;

    if (!init)
    {
        me.name = "沉静";
        me.desc = "一段描述。";

        me.stats[A_WIS] = 2;

        me.skills.sav = 3;

        me.life = 99;
        me.exp = 100;

        me.birth = _test_birth;
        init = TRUE;
    }
    return &me;
}
"#;
        let blocks = extract_personality_blocks(SYNTHETIC_SOURCE);
        assert_eq!(blocks.len(), 1);
        let calm = parse_character_block(&blocks[0].0, &blocks[0].1);
        assert_eq!(calm.id, "test-calm");
        assert_eq!(calm.stats[2], 2);
        assert_eq!(calm.skills[2], 3);
        assert_eq!(calm.life, 99);
        assert_eq!(calm.hooks, ["birth"]);

        let characters = LegacyCharacterSources {
            bodies: Vec::new(),
            races: Vec::new(),
            personalities: vec![calm],
            ..LegacyCharacterSources::default()
        };
        let outcome = convert_content(&[], &[], &[], &[], &[], &characters);
        assert_eq!(outcome.report.personalities_imported, 1);
        let (name, personality) = &outcome.personality_files[0];
        assert_eq!(name, "test-calm.json");
        assert_eq!(personality["id"], "rfb-legacy.personality.test-calm");
        assert_eq!(personality["modifiers"]["wisdom"], 2);
        assert!(personality.get("bodySlots").is_none());
    }

    #[test]
    fn e_info_egos_become_affixes_with_maxima_modifiers() {
        const SYNTHETIC_E_INFO: &str = "V:1.1.0
N:1:of Testing
T:WEAPON
W:0:35:2
C:8:6:0:0
F:SHOW_MODS

N:2:of the Test Bear
T:AMULET | RING
W:10:*:4
C:0:0:0:3
F:STR | DEC_INT | HIDE_TYPE | SPEED | SUST_STR | SUST_INT | SUST_WIS | SUST_DEX | SUST_CON | SUST_CHR
E:BERSERK:50:100

N:3:(Test Aura)
T:WEAPON
W:50:*:6
C:0:0:0:2
F:SPELL_POWER

N:4:of Test Warding
T:CLOAK
W:20:*:8
F:RES_FIRE | IM_COLD | VULN_LITE | FREE_ACT | RES_FEAR

N:5:of Test Dragonfire
T:WEAPON
W:30:*:5
F:SLAY_DRAGON | BRAND_FIRE

N:6:(Death)
T:WEAPON | DIGGER
W:20:*:4
F:BRAND_VAMP | HOLD_LIFE
";
        let egos = parse_e_info(SYNTHETIC_E_INFO).expect("synthetic egos should parse");
        assert_eq!(egos.len(), 6);
        assert_eq!(egos[0].max_level, Some(35));
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.egos_total, 6);
        assert_eq!(outcome.report.egos_imported, 6);
        assert_eq!(outcome.affix_files.len(), 6);
        assert!(
            !outcome
                .report
                .unmapped_ego_flags
                .contains_key("SPELL_POWER")
        );

        let (name, testing) = &outcome.affix_files[0];
        assert_eq!(name, "testing.json");
        assert_eq!(testing["id"], "rfb-legacy.affix.testing");
        assert_eq!(testing["generationMaxLevel"], 35);
        // C: maxima fold into a deterministic ceiling; attack takes the
        // larger of to-hit/to-damage.
        assert_eq!(testing["modifiers"]["attack"], 8);
        assert!(
            testing["tags"]
                .as_array()
                .expect("ego tags")
                .iter()
                .any(|tag| tag == "weapon")
        );

        let (name, bear) = &outcome.affix_files[1];
        assert_eq!(name, "the-test-bear.json");
        assert_eq!(bear["modifiers"]["strength"], 3);
        assert_eq!(bear["modifiers"]["intelligence"], -3);
        // SPEED rides the same C: pval ceiling as the attribute flags.
        assert_eq!(bear["modifiers"]["speed"], 3);
        assert!(bear["modifiers"].get("attack").is_none());
        assert!(
            bear["tags"]
                .as_array()
                .expect("bear tags")
                .iter()
                .any(|tag| tag == "amulet")
        );
        assert_eq!(outcome.report.item_behavior_gaps["ego-activation"], 1);

        let (_, aura) = &outcome.affix_files[2];
        assert_eq!(aura["modifiers"]["spellPowerBonus"], 2);
        assert_eq!(outcome.report.not_applicable_item_flags["HIDE_TYPE"], 1);
        assert!(!outcome.report.unmapped_ego_flags.contains_key("STR"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("DEC_INT"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("SPEED"));
        assert_eq!(
            bear["passives"],
            serde_json::json!([
                "sustain-charisma",
                "sustain-constitution",
                "sustain-dexterity",
                "sustain-intelligence",
                "sustain-strength",
                "sustain-wisdom"
            ])
        );
        for flag in [
            "SUST_STR", "SUST_INT", "SUST_WIS", "SUST_DEX", "SUST_CON", "SUST_CHR",
        ] {
            assert!(!outcome.report.unmapped_ego_flags.contains_key(flag));
        }

        // A purely defensive ego used to be inexpressible; the flag fold now
        // carries its elemental defenses and status immunities.
        let (name, warding) = &outcome.affix_files[3];
        assert_eq!(name, "test-warding.json");
        assert!(warding.get("modifiers").is_none());
        assert_eq!(warding["resistances"]["fire"], "resistant");
        assert_eq!(warding["resistances"]["cold"], "immune");
        assert_eq!(warding["resistances"]["light"], "vulnerable");
        let immunities = warding["statusImmunities"].as_array().unwrap();
        assert!(
            immunities
                .iter()
                .any(|value| value == "rfb.status.paralysis")
        );
        assert!(immunities.iter().any(|value| value == "rfb.status.fear"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("RES_FEAR"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("RES_FIRE"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("IM_COLD"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("VULN_LITE"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("FREE_ACT"));

        let (_, dragonfire) = &outcome.affix_files[4];
        assert_eq!(dragonfire["slays"]["dragon"], "slay");
        assert_eq!(dragonfire["brands"][0], "fire");
        assert!(
            !outcome
                .report
                .unmapped_ego_flags
                .contains_key("SLAY_DRAGON")
        );
        assert!(!outcome.report.unmapped_ego_flags.contains_key("BRAND_FIRE"));

        let (_, death) = &outcome.affix_files[5];
        assert_eq!(death["id"], LEGACY_DEATH_WEAPON_AFFIX_ID);
        assert!(
            death["passives"]
                .as_array()
                .expect("death passives")
                .iter()
                .any(|value| value == "vampiric")
        );
        assert!(
            death["passives"]
                .as_array()
                .expect("death passives")
                .iter()
                .any(|value| value == "hold-life")
        );
        assert!(!outcome.report.unmapped_ego_flags.contains_key("BRAND_VAMP"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("HOLD_LIFE"));
    }

    #[test]
    fn equipment_flags_map_spell_power_and_its_decrease() {
        let flags = vec!["SPELL_POWER".to_owned()];
        let mut modifiers = serde_json::Map::new();
        let mut equipment = equipment_fold(&flags, 3);
        fold_spell_power_modifier(&flags, 3, &mut modifiers, &mut equipment);
        assert_eq!(modifiers["spellPowerBonus"], 3);
        assert!(equipment.consumed.contains("SPELL_POWER"));

        let flags = vec!["DEC_SPELL_POWER".to_owned()];
        let mut modifiers = serde_json::Map::new();
        let mut equipment = equipment_fold(&flags, 4);
        fold_spell_power_modifier(&flags, 4, &mut modifiers, &mut equipment);
        assert_eq!(modifiers["spellPowerBonus"], -4);
        assert!(equipment.consumed.contains("DEC_SPELL_POWER"));
    }

    #[test]
    fn protection_ego_materializes_uniform_armor_rolls() {
        let egos = parse_e_info(
            "N:50:of Protection\nT:BODY_ARMOR | SHIELD | CLOAK | HELMET | GLOVES | BOOTS\nW:0:30:2\nC:0:0:10:0\nF:IGNORE_ACID\n",
        )
        .expect("Protection ego should parse");
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        let protection = &outcome.affix_files[0].1;
        assert_eq!(protection["id"], "rfb-legacy.affix.protection");
        assert_eq!(protection["generationMaxLevel"], 30);
        assert_eq!(
            protection["elementalDestructionImmunities"],
            serde_json::json!(["acid"])
        );
        assert!(protection.get("modifiers").is_none());
        let candidates = protection["rollGroups"][0]["candidates"]
            .as_array()
            .expect("Protection should retain armor candidates");
        assert_eq!(candidates.len(), 10);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate["properties"]["modifiers"]["defense"]
                    .as_i64()
                    .unwrap())
                .collect::<Vec<_>>(),
            (1..=10).collect::<Vec<_>>()
        );
        assert!(candidates.iter().all(|candidate| candidate["weight"] == 1));
    }

    #[test]
    fn endurance_ammunition_ego_resists_projection_destruction() {
        let egos = parse_e_info("N:184:of Endurance\nT:AMMO\nW:0:*:2\n")
            .expect("Endurance ego should parse");
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        let endurance = &outcome.affix_files[0].1;
        assert_eq!(endurance["id"], "rfb-legacy.affix.endurance");
        assert_eq!(endurance["resistsMonsterDestruction"], true);
        assert_eq!(endurance["resistsProjectionDestruction"], true);
    }

    #[test]
    fn combat_ring_ego_materializes_three_original_weighted_rolls() {
        let egos = parse_e_info("N:206:of Combat\nT:RING\nW:10:*:2\nF:HIDE_TYPE\n")
            .expect("Combat ego should parse");
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        let combat = &outcome.affix_files[0].1;
        assert_eq!(combat["id"], "rfb-legacy.affix.combat");
        assert_eq!(combat["rollGroups"][0]["rolls"], 3);
        let candidates = combat["rollGroups"][0]["candidates"]
            .as_array()
            .expect("Combat ego should retain weighted candidates");
        assert_eq!(candidates.len(), 7);
        assert_eq!(
            candidates
                .iter()
                .map(|candidate| candidate["weight"].as_u64().unwrap())
                .sum::<u64>(),
            100
        );
    }

    #[test]
    fn physical_spellbooks_use_explicit_realm_tvals_without_capturing_necromancy() {
        assert_eq!(
            death_first_book_json(&[])["nameKey"],
            "ability-book-legacy-death-black-prayers-name"
        );
        assert_eq!(
            death_second_book_json(&[])["nameKey"],
            "ability-book-legacy-death-black-mass-name"
        );

        for (sval, expected_book_id) in [
            (DEATH_FIRST_BOOK_SVAL, DEATH_FIRST_BOOK_ID),
            (DEATH_SECOND_BOOK_SVAL, DEATH_SECOND_BOOK_ID),
            (DEATH_THIRD_BOOK_SVAL, DEATH_THIRD_BOOK_ID),
            (DEATH_FOURTH_BOOK_SVAL, DEATH_FOURTH_BOOK_ID),
        ] {
            let death_book = LegacyItemEntry {
                tval: DEATH_BOOK_TVAL,
                sval,
                ..LegacyItemEntry::default()
            };
            assert_eq!(
                player_ability_book_for_item(&death_book),
                Some(expected_book_id)
            );

            let necromancy_book = LegacyItemEntry {
                tval: 100,
                sval,
                ..LegacyItemEntry::default()
            };
            assert_eq!(player_ability_book_for_item(&necromancy_book), None);
        }

        let arcane_book = LegacyItemEntry {
            tval: ARCANE_BOOK_TVAL,
            sval: ARCANE_FIRST_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&arcane_book),
            Some(ARCANE_FIRST_BOOK_ID)
        );
        let third_arcane_book = LegacyItemEntry {
            tval: ARCANE_BOOK_TVAL,
            sval: ARCANE_THIRD_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&third_arcane_book),
            Some(ARCANE_THIRD_BOOK_ID)
        );
        let second_arcane_book = LegacyItemEntry {
            tval: ARCANE_BOOK_TVAL,
            sval: ARCANE_SECOND_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&second_arcane_book),
            Some(ARCANE_SECOND_BOOK_ID)
        );
        let fourth_arcane_book = LegacyItemEntry {
            tval: ARCANE_BOOK_TVAL,
            sval: ARCANE_FOURTH_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&fourth_arcane_book),
            Some(ARCANE_FOURTH_BOOK_ID)
        );

        for (sval, expected_book_id) in [
            (SORCERY_FIRST_BOOK_SVAL, SORCERY_FIRST_BOOK_ID),
            (SORCERY_SECOND_BOOK_SVAL, SORCERY_SECOND_BOOK_ID),
            (SORCERY_THIRD_BOOK_SVAL, SORCERY_THIRD_BOOK_ID),
            (SORCERY_FOURTH_BOOK_SVAL, SORCERY_FOURTH_BOOK_ID),
        ] {
            let sorcery_book = LegacyItemEntry {
                tval: SORCERY_BOOK_TVAL,
                sval,
                ..LegacyItemEntry::default()
            };
            assert_eq!(
                player_ability_book_for_item(&sorcery_book),
                Some(expected_book_id)
            );
        }

        for (sval, expected_book_id) in [
            (ARMAGEDDON_FIRST_BOOK_SVAL, ARMAGEDDON_FIRST_BOOK_ID),
            (ARMAGEDDON_SECOND_BOOK_SVAL, ARMAGEDDON_SECOND_BOOK_ID),
            (ARMAGEDDON_THIRD_BOOK_SVAL, ARMAGEDDON_THIRD_BOOK_ID),
            (ARMAGEDDON_FOURTH_BOOK_SVAL, ARMAGEDDON_FOURTH_BOOK_ID),
        ] {
            let armageddon_book = LegacyItemEntry {
                tval: ARMAGEDDON_BOOK_TVAL,
                sval,
                ..LegacyItemEntry::default()
            };
            assert_eq!(
                player_ability_book_for_item(&armageddon_book),
                Some(expected_book_id)
            );
        }
    }

    #[test]
    fn fixed_healing_potions_add_ordered_status_recovery_and_leave_no_gap() {
        let expected = [
            (
                34,
                serde_json::json!({"type": "heal-dice", "dice": 4, "sides": 8}),
            ),
            (
                35,
                serde_json::json!({"type": "heal-dice", "dice": 8, "sides": 8}),
            ),
            (
                36,
                serde_json::json!({"type": "heal-dice", "dice": 12, "sides": 8}),
            ),
            (37, serde_json::json!({"type": "heal", "amount": 300})),
            (38, serde_json::json!({"type": "heal", "amount": 1000})),
        ];
        let mut report = ContentImportReport::default();
        for (sval, effect) in expected {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 75,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("healing-potion-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effects = value["useAction"]["effect"]["effects"]
                .as_array()
                .expect("healing potions should use ordered effects");
            assert_eq!(effects[0], effect);
            assert!(
                effects[1..]
                    .iter()
                    .all(|effect| effect["type"] == "remove-status")
            );
        }
        let life = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 39,
                ..LegacyItemEntry::default()
            },
            "life-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            life["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-life-restoration",
                "healingAmount": 5000,
                "lifeForceAmount": 1000
            })
        );
        let blood = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 67,
                ..LegacyItemEntry::default()
            },
            "blood-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            blood["useAction"]["effect"],
            serde_json::json!({
                "type": "sequence",
                "effects": [
                    {"type": "heal", "amount": 200},
                    {
                        "type": "remove-status",
                        "statusKindId": "rfb.status.blindness"
                    },
                    {
                        "type": "remove-status",
                        "statusKindId": "rfb.status.confusion"
                    },
                    {
                        "type": "remove-status",
                        "statusKindId": "rfb.status.stun"
                    }
                ]
            })
        );
        assert!(!report.item_behavior_gaps.contains_key("consumable-effect"));

        let berserk_strength = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 33,
                ..LegacyItemEntry::default()
            },
            "berserk-strength-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            berserk_strength["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-berserk-strength",
                "durationDice": 1,
                "durationSides": 25,
                "durationBonus": 25
            })
        );
        assert!(!report.item_behavior_gaps.contains_key("consumable-effect"));
    }

    #[test]
    fn restorative_foods_and_potions_map_status_and_mana_effects() {
        let mut report = ContentImportReport::default();
        let cases = [
            (80, 12, "remove-status"),
            (80, 13, "remove-status"),
            (80, 14, "remove-status"),
            (80, 15, "remove-status"),
            (75, 28, "remove-status"),
            (75, 31, "sequence"),
            (75, 40, "sequence"),
            (75, 70, "sequence"),
        ];
        for (tval, sval, effect_type) in cases {
            let value = item_json(
                &LegacyItemEntry {
                    tval,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("restorative-{tval}-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(value["useAction"]["effect"]["type"], effect_type);
        }
        assert!(!report.item_behavior_gaps.contains_key("consumable-effect"));

        let restore_mana = fixed_consumable_use_action(&LegacyItemEntry {
            tval: 75,
            sval: 40,
            ..LegacyItemEntry::default()
        })
        .expect("restore mana should map");
        assert_eq!(
            restore_mana["effect"]["effects"][0],
            serde_json::json!({
                "type": "restore-resource-full",
                "resourceId": LEGACY_MANA_RESOURCE_ID
            })
        );

        let clarity = fixed_consumable_use_action(&LegacyItemEntry {
            tval: 75,
            sval: 70,
            ..LegacyItemEntry::default()
        })
        .expect("clarity should map");
        assert_eq!(clarity["effect"]["effects"][0]["dice"], 3);
        assert_eq!(clarity["effect"]["effects"][0]["sides"], 6);
        assert_eq!(clarity["effect"]["effects"][0]["bonus"], 3);
        assert_eq!(
            clarity["effect"]["effects"][1]["statusKindId"],
            "rfb.status.confusion"
        );
    }

    #[test]
    fn p3_2_low_coupling_potions_map_authoritative_effects() {
        let effect = |sval| {
            fixed_consumable_use_action(&LegacyItemEntry {
                tval: 75,
                sval,
                ..LegacyItemEntry::default()
            })
            .expect("P3.2 potion should map")["effect"]
                .clone()
        };

        for sval in 0..=2 {
            assert_eq!(effect(sval)["type"], "no-numeric-effect");
        }
        assert_eq!(effect(13)["divisor"], 4);
        assert_eq!(effect(15)["effects"].as_array().map(Vec::len), Some(7));
        assert_eq!(effect(26)["effects"][0]["statusKindId"], "rfb.status.sight");
        assert_eq!(effect(27)["effects"][0]["minimumReduction"], 4000);
        assert_eq!(
            effect(27)["effects"][1]["grantedResistances"]["poison"],
            "resistant"
        );
        assert_eq!(effect(61)["effects"].as_array().map(Vec::len), Some(7));
        assert_eq!(effect(62)["incomingDamagePercent"], 0);
        assert_eq!(effect(68)["type"], "apply-giant-strength");
        assert_eq!(effect(71)["effects"][0]["dice"], 10);
        assert_eq!(effect(71)["effects"][0]["bonus"], 15);

        assert!(
            fixed_consumable_use_action(&LegacyItemEntry {
                tval: 75,
                sval: 3,
                ..LegacyItemEntry::default()
            })
            .is_none(),
            "salt water remains blocked"
        );
    }

    #[test]
    fn p3_3_knowledge_detection_and_protection_map_authoritative_effects() {
        let effect = |tval, sval| {
            fixed_consumable_use_action(&LegacyItemEntry {
                tval,
                sval,
                ..LegacyItemEntry::default()
            })
            .expect("P3.3 consumable should map")["effect"]
                .clone()
        };

        assert_eq!(effect(70, 26)["subject"], "gold");
        assert_eq!(effect(70, 60)["effects"][0]["type"], "identify-inventory");
        assert_eq!(
            effect(70, 60)["effects"][1]["statusKindId"],
            "rfb.status.understanding"
        );
        assert_eq!(
            effect(70, 63)["statusKindId"],
            "rfb.status.inventory-protection"
        );
        assert_eq!(effect(75, 56)["effects"][0]["radius"], 255);
        assert_eq!(effect(75, 57)["effects"].as_array().map(Vec::len), Some(9));
        assert_eq!(effect(75, 57)["effects"][5]["radius"], 255);
        assert_eq!(effect(75, 57)["effects"][6]["radius"], 255);
        assert_eq!(effect(75, 57)["effects"][8]["type"], "self-knowledge");
        assert_eq!(effect(75, 58)["type"], "self-knowledge");
    }

    #[test]
    fn p3_4_floor_and_area_scrolls_map_authoritative_effects() {
        let terrain = TerrainCreationImportIds {
            source_terrain_ids: vec!["rfb-legacy.terrain.floor".to_owned()],
            floor_terrain_id: Some("rfb-legacy.terrain.floor".to_owned()),
            created_trap_terrain_id: Some("rfb-legacy.terrain.created-trap".to_owned()),
            glyph_terrain_id: Some("rfb-legacy.terrain.glyph".to_owned()),
            wall_terrain_id: Some("rfb-legacy.terrain.granite".to_owned()),
            quartz_terrain_id: Some("rfb-legacy.terrain.quartz".to_owned()),
            magma_terrain_id: Some("rfb-legacy.terrain.magma".to_owned()),
            ..TerrainCreationImportIds::default()
        };
        let effect = |sval| {
            fixed_consumable_use_action_with_terrain(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                Some(&terrain),
            )
            .expect("P3.4 scroll should map")["effect"]
                .clone()
        };

        assert_eq!(effect(0)["effects"][0]["type"], "apply-blindness");
        assert_eq!(effect(0)["effects"][1]["connectedGlow"], true);
        assert_eq!(
            effect(7)["targetTerrainId"],
            "rfb-legacy.terrain.created-trap"
        );
        assert_eq!(effect(24)["type"], "set-floor-glow");
        assert_eq!(effect(38)["targetTerrainId"], "rfb-legacy.terrain.glyph");
        assert_eq!(effect(41)["minimumRadius"], 13);
        assert_eq!(effect(41)["maximumRadius"], 17);
        assert_eq!(effect(41)["quartzTerrainId"], "rfb-legacy.terrain.quartz");
    }

    #[test]
    fn p3_5_generation_and_mutation_scrolls_map_authoritative_effects() {
        let effect = |sval| {
            fixed_consumable_use_action(&LegacyItemEntry {
                tval: 70,
                sval,
                ..LegacyItemEntry::default()
            })
            .expect("P3.5 scroll should map")["effect"]
                .clone()
        };

        assert_eq!(effect(23)["type"], "mundanify-item");
        assert_eq!(effect(46)["minimumCount"], 1);
        assert_eq!(effect(46)["maximumCount"], 1);
        assert_eq!(effect(47)["minimumCount"], 2);
        assert_eq!(effect(47)["maximumCount"], 3);
        assert_eq!(effect(51)["type"], "show-rumour");
        assert_eq!(effect(55)["type"], "craft-item");

        let mut report = ContentImportReport::default();
        let _ = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 52,
                ..LegacyItemEntry::default()
            },
            "artifact-creation-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(report.item_behavior_gaps["random-artifact-identity"], 1);
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));
    }

    #[test]
    fn p3_7_growth_and_tsuyoshi_potions_map_without_fake_mutations() {
        let effect = |sval| {
            fixed_consumable_use_action(&LegacyItemEntry {
                tval: 75,
                sval,
                ..LegacyItemEntry::default()
            })
            .expect("supported P3.7 potion should map")["effect"]
                .clone()
        };

        assert_eq!(effect(59)["type"], "gain-relative-experience");
        assert_eq!(effect(59)["maximumGain"], 100_000);
        assert_eq!(effect(64)["effects"][0]["type"], "remove-status");
        assert_eq!(effect(64)["effects"][1]["type"], "apply-tsuyoshi");
        assert_eq!(effect(65)["effects"][0]["type"], "trigger-tsuyoshi-crash");
        assert_eq!(
            effect(65)["effects"][1]["statusKindId"],
            "rfb.status.hallucination"
        );

        for sval in [63, 66] {
            let mut report = ContentImportReport::default();
            let _ = item_json(
                &LegacyItemEntry {
                    tval: 75,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("blocked-p3-7-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(report.item_behavior_gaps["consumable-effect"], 1);
        }
    }

    #[test]
    fn food_nutrition_gap_is_independent_from_active_effect_gap() {
        let cases = [
            (80, 99, Some(1), Some(1)),
            (80, 12, None, Some(1)),
            (75, 99, Some(1), None),
        ];
        for (tval, sval, consumable_effect, food_nutrition) in cases {
            let mut report = ContentImportReport::default();
            item_json(
                &LegacyItemEntry {
                    tval,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("gap-{tval}-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(
                report.item_behavior_gaps.get("consumable-effect").copied(),
                consumable_effect
            );
            assert_eq!(
                report.item_behavior_gaps.get("food-nutrition").copied(),
                food_nutrition
            );
        }
    }

    #[test]
    fn p3_1_foods_apply_special_effects_before_source_nutrition() {
        let harmful = fixed_consumable_use_action(&LegacyItemEntry {
            tval: 80,
            sval: 6,
            pval: 500,
            ..LegacyItemEntry::default()
        })
        .expect("weakness food should map");
        assert_eq!(
            harmful["effect"]["effects"],
            serde_json::json!([
                {"type": "self-damage", "damageDice": 6, "damageSides": 6},
                {"type": "drain-attribute", "attribute": "strength"},
                {"type": "increase-nutrition", "amount": 500}
            ])
        );

        let plain = fixed_consumable_use_action(&LegacyItemEntry {
            tval: 80,
            sval: 33,
            pval: 1500,
            ..LegacyItemEntry::default()
        })
        .expect("venison should map");
        assert_eq!(
            plain["effect"],
            serde_json::json!({"type": "increase-nutrition", "amount": 1500})
        );

        assert_eq!(
            p3_1_food_effect(&LegacyItemEntry {
                tval: 80,
                sval: 37,
                pval: 7_500,
                ..LegacyItemEntry::default()
            }),
            Some(serde_json::json!({
                "type": "apply-elvish-waybread",
                "healingDice": 4,
                "healingSides": 8
            }))
        );
    }

    #[test]
    fn generic_legacy_devices_gain_dynamic_activation_tables() {
        let expected = [(55, "heal"), (65, "damage"), (66, "detect")];
        let mut report = ContentImportReport::default();
        for (tval, effect_type) in expected {
            let value = item_json(
                &LegacyItemEntry {
                    tval,
                    ..LegacyItemEntry::default()
                },
                &format!("device-{tval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(value["maxStack"], 1);
            let activations = value["deviceGeneration"]["activations"]
                .as_array()
                .expect("device shell should contain activation candidates");
            assert!(
                activations
                    .iter()
                    .any(|activation| activation["effect"]["type"] == effect_type)
            );
            assert_eq!(
                value["deviceGeneration"]["recovery"]["intervalTicks"],
                if tval == 66 { 1 } else { 10 }
            );
            assert_eq!(value["deviceGeneration"]["recovery"]["energyPerMille"], 10);
        }
        assert!(!report.item_behavior_gaps.contains_key("device-effect"));

        let _ = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 19,
                ..LegacyItemEntry::default()
            },
            "scroll-shell",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(report.item_behavior_gaps["scroll-effect"], 1);

        let trap = terrain_json(
            &LegacyTerrainEntry {
                flags: vec!["LOS".to_owned(), "MOVE".to_owned(), "TRAP".to_owned()],
                ..LegacyTerrainEntry::default()
            },
            "test-trap",
            None,
        );
        assert!(
            trap["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "trap") })
        );
    }

    #[test]
    fn legacy_letter_terrain_glyphs_are_remapped_without_using_actor_letters() {
        let passage = terrain_json(
            &LegacyTerrainEntry {
                glyph: Some('M'),
                flags: vec!["MOVE".to_owned(), "STORE".to_owned(), "DOOR".to_owned()],
                ..LegacyTerrainEntry::default()
            },
            "test-store",
            None,
        );
        assert_eq!(passage["glyph"], "+");

        let blocked = terrain_json(
            &LegacyTerrainEntry {
                glyph: Some('x'),
                ..LegacyTerrainEntry::default()
            },
            "test-hidden",
            None,
        );
        assert_eq!(blocked["glyph"], "#");
    }

    #[test]
    fn scrolls_map_to_curse_teleport_knowledge_enchantment_and_detection_effects() {
        let mut report = ContentImportReport::default();
        for (
            sval,
            selector,
            maximum_level_source,
            hostile,
            count_sides,
            group_chance_percent,
            group_count_sides,
            group_count_bonus,
            allow_unique,
        ) in [
            (
                4,
                serde_json::json!({"type": "any-monster"}),
                "dungeon-depth",
                true,
                3,
                100,
                3,
                0,
                true,
            ),
            (
                5,
                serde_json::json!({"type": "category", "category": "undead"}),
                "dungeon-depth",
                true,
                3,
                100,
                3,
                0,
                true,
            ),
            (
                6,
                serde_json::json!({"type": "any-monster"}),
                "dungeon-depth",
                false,
                1,
                50,
                3,
                1,
                false,
            ),
            (
                54,
                serde_json::json!({"type": "player-kin"}),
                "player-level",
                false,
                1,
                50,
                3,
                1,
                false,
            ),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("summoning-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], "summon-category");
            assert_eq!(effect["selector"], selector);
            assert_eq!(effect["maximumLevelSource"], maximum_level_source);
            assert_eq!(effect["countDice"], 1);
            assert_eq!(effect["countSides"], count_sides);
            assert_eq!(effect["hostile"], hostile);
            assert_eq!(effect["groupChancePercent"], group_chance_percent);
            assert_eq!(effect["groupCountDice"], 1);
            assert_eq!(effect["groupCountSides"], group_count_sides);
            assert_eq!(effect["groupCountBonus"], group_count_bonus);
            assert_eq!(effect["allowUnique"], allow_unique);
            assert_eq!(effect["radius"], 2);
            assert_eq!(effect["durationTurns"], 0);
        }
        for (sval, effect_type, field, expected) in [
            (
                2,
                "curse-equipped-item",
                "target",
                serde_json::json!("armor"),
            ),
            (
                3,
                "curse-equipped-item",
                "target",
                serde_json::json!("weapon"),
            ),
            (
                14,
                "remove-equipped-curses",
                "includeHeavy",
                serde_json::json!(false),
            ),
            (
                15,
                "remove-equipped-curses",
                "includeHeavy",
                serde_json::json!(true),
            ),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("curse-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], effect_type);
            assert_eq!(effect[field], expected);
        }
        for (sval, effect_type) in [
            (8, "random-teleport"),
            (9, "random-teleport"),
            (10, "teleport-level"),
            (11, "recall"),
            (53, "reset-recall"),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("travel-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], effect_type);
            if sval == 8 {
                assert_eq!(effect["maximumDistance"], 10);
            } else if sval == 9 {
                assert_eq!(effect["maximumDistance"], 100);
            } else if sval == 11 {
                assert_eq!(effect["delayDice"], 1);
                assert_eq!(effect["delaySides"], 21);
                assert_eq!(effect["delayBonus"], 14);
            }
        }
        for (sval, full) in [(12, false), (13, true)] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("identify-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(value["useAction"]["effect"]["type"], "identify-item");
            assert_eq!(value["useAction"]["effect"]["full"], full);
        }
        for (sval, component, dice, sides, bonus) in [
            (16, "toArmor", 0, 0, 1),
            (17, "toHit", 0, 0, 1),
            (18, "toDamage", 0, 0, 1),
            (20, "toArmor", 1, 3, 3),
            (21, "toHit", 1, 3, 3),
            (21, "toDamage", 1, 3, 3),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("enchantment-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], "enchant-item");
            assert_eq!(effect[component]["dice"], dice);
            assert_eq!(effect[component]["sides"], sides);
            assert_eq!(effect[component]["bonus"], bonus);
        }
        let recharging = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 22,
                ..LegacyItemEntry::default()
            },
            "recharging-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            recharging["useAction"]["effect"],
            serde_json::json!({"type": "recharge-from-device", "power": 100})
        );
        let spell = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 43,
                ..LegacyItemEntry::default()
            },
            "spell-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            spell["useAction"]["effect"],
            serde_json::json!({"type": "increase-spell-learning-capacity"})
        );
        let slowness = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 4,
                ..LegacyItemEntry::default()
            },
            "slowness-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            slowness["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-slowness",
                "durationDice": 1,
                "durationSides": 25,
                "durationBonus": 15
            })
        );
        let speed = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 29,
                ..LegacyItemEntry::default()
            },
            "speed-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            speed["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-speed",
                "durationDice": 1,
                "durationSides": 25,
                "durationBonus": 15
            })
        );
        let heroism = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 32,
                ..LegacyItemEntry::default()
            },
            "heroism-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            heroism["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-heroism",
                "durationDice": 1,
                "durationSides": 25,
                "durationBonus": 25
            })
        );
        let poetic_inspiration = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 14,
                ..LegacyItemEntry::default()
            },
            "poetic-inspiration-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            poetic_inspiration["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-poetic-inspiration",
                "durationDice": 1,
                "durationSides": 100,
                "durationBonus": 100
            })
        );
        let stone_skin = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 69,
                ..LegacyItemEntry::default()
            },
            "stone-skin-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            stone_skin["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-stone-skin",
                "durationDice": 1,
                "durationSides": 20,
                "durationBonus": 20
            })
        );
        let restore_life_levels = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 41,
                ..LegacyItemEntry::default()
            },
            "restore-life-levels-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            restore_life_levels["useAction"]["effect"],
            serde_json::json!({
                "type": "restore-life-levels",
                "lifeForceAmount": 150
            })
        );
        for (tval, sval, effect) in [
            (
                75,
                54,
                serde_json::json!({
                    "type": "restore-all-vitality",
                    "lifeForceAmount": 150
                }),
            ),
            (
                80,
                17,
                serde_json::json!({
                    "type": "restore-attribute",
                    "attribute": "strength"
                }),
            ),
            (
                80,
                18,
                serde_json::json!({
                    "type": "restore-attribute",
                    "attribute": "constitution"
                }),
            ),
            (
                80,
                19,
                serde_json::json!({"type": "restore-all-attributes"}),
            ),
            (
                80,
                40,
                serde_json::json!({
                    "type": "apply-restorative-feast",
                    "healingDice": 15,
                    "healingSides": 15
                }),
            ),
            (
                75,
                39,
                serde_json::json!({
                    "type": "apply-life-restoration",
                    "healingAmount": 5000,
                    "lifeForceAmount": 1000
                }),
            ),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("restoration-{tval}-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(value["useAction"]["effect"], effect);
        }
        for (sval, effect_type, attribute) in [
            (16, "drain-attribute", "strength"),
            (17, "drain-attribute", "intelligence"),
            (18, "drain-attribute", "wisdom"),
            (19, "drain-attribute", "dexterity"),
            (20, "drain-attribute", "constitution"),
            (21, "drain-attribute", "charisma"),
            (42, "restore-attribute", "strength"),
            (43, "restore-attribute", "intelligence"),
            (44, "restore-attribute", "wisdom"),
            (45, "restore-attribute", "dexterity"),
            (46, "restore-attribute", "constitution"),
            (47, "restore-attribute", "charisma"),
            (48, "increase-attribute", "strength"),
            (49, "increase-attribute", "intelligence"),
            (50, "increase-attribute", "wisdom"),
            (51, "increase-attribute", "dexterity"),
            (52, "increase-attribute", "constitution"),
            (53, "increase-attribute", "charisma"),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 75,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("attribute-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(
                value["useAction"]["effect"],
                serde_json::json!({"type": effect_type, "attribute": attribute})
            );
        }
        let augmentation = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 55,
                ..LegacyItemEntry::default()
            },
            "augmentation-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            augmentation["useAction"]["effect"],
            serde_json::json!({"type": "augment-attributes"})
        );
        let thermal = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 30,
                ..LegacyItemEntry::default()
            },
            "thermal-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            thermal["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-thermal-resistance",
                "durationDice": 1,
                "durationSides": 10,
                "durationBonus": 10
            })
        );
        let resistance = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 60,
                ..LegacyItemEntry::default()
            },
            "resistance-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            resistance["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-basic-resistance",
                "durationDice": 1,
                "durationSides": 20,
                "durationBonus": 20
            })
        );
        let poison = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 6,
                ..LegacyItemEntry::default()
            },
            "poison-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            poison["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-poison",
                "durationDice": 1,
                "durationSides": 15,
                "durationBonus": 9
            })
        );
        let poison_food = item_json(
            &LegacyItemEntry {
                tval: 80,
                sval: 0,
                ..LegacyItemEntry::default()
            },
            "poison-food",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            poison_food["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-poison",
                "durationDice": 1,
                "durationSides": 10,
                "durationBonus": 9
            })
        );
        for (tval, sval, sides, bonus) in [(75, 7, 100, 99), (80, 1, 25, 24)] {
            let blindness = item_json(
                &LegacyItemEntry {
                    tval,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("blindness-{tval}-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            assert_eq!(
                blindness["useAction"]["effect"],
                serde_json::json!({
                    "type": "apply-blindness",
                    "durationDice": 1,
                    "durationSides": sides,
                    "durationBonus": bonus
                })
            );
        }
        let death = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 23,
                ..LegacyItemEntry::default()
            },
            "death-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            death["useAction"]["effect"],
            serde_json::json!({"type": "self-life-loss", "amount": 5000})
        );
        let detonation = item_json(
            &LegacyItemEntry {
                tval: 75,
                sval: 22,
                ..LegacyItemEntry::default()
            },
            "detonation-potion",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            detonation["useAction"]["effect"],
            serde_json::json!({
                "type": "apply-detonation",
                "damageDice": 50,
                "damageSides": 20,
                "stunTicks": 75,
                "bleedingTicks": 5000
            })
        );
        for (sval, subject, category, persistent) in [
            (25, "terrain", "map", true),
            (26, "gold", "gold", false),
            (27, "item", "item", false),
            (28, "terrain", "trap", true),
            (29, "terrain", "passage", true),
            (30, "actor", "invisible", false),
            (57, "actor", "legacy-import", false),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("detect-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], "detect");
            assert_eq!(effect["subject"], subject);
            assert_eq!(effect["category"], category);
            assert_eq!(effect["persistent"], persistent);
            assert_eq!(effect["throughWalls"], true);
        }
        for (sval, duration_sides, duration_bonus) in [(33, 12, 6), (34, 24, 12), (35, 48, 24)] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("blessing-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], "bless");
            assert_eq!(effect["durationDice"], 1);
            assert_eq!(effect["durationSides"], duration_sides);
            assert_eq!(effect["durationBonus"], duration_bonus);
        }
        let trap_door_destruction = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 39,
                ..LegacyItemEntry::default()
            },
            "trap-door-destruction-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            trap_door_destruction["useAction"]["effect"]["type"],
            "destroy-adjacent-traps-and-doors"
        );
        for (sval, effect_type, field, expected) in [
            (42, "dispel-category", "damage", serde_json::json!(80)),
            (
                62,
                "banish-visible",
                "maximumDistance",
                serde_json::json!(150),
            ),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("visible-actor-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], effect_type);
            assert_eq!(effect[field], expected);
            if sval == 42 {
                assert_eq!(effect["category"], "undead");
            }
        }
        for (
            sval,
            base_damage,
            damage_type,
            backlash_sides,
            backlash_bonus,
            backlash_damage_type,
            backlash_uses_resistance,
        ) in [
            (58, 666, "fire", 25, 25, "fire", true),
            (59, 800, "ice", 30, 30, "cold", true),
            (61, 1100, "mana", 50, 50, "mana", false),
        ] {
            let value = item_json(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("elemental-blast-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                &mut report,
            );
            let effect = &value["useAction"]["effect"];
            assert_eq!(effect["type"], "self-centered-elemental-blast");
            assert_eq!(effect["baseDamage"], base_damage);
            assert_eq!(effect["damageType"], damage_type);
            assert_eq!(effect["radius"], 4);
            assert_eq!(effect["backlashSides"], backlash_sides);
            assert_eq!(effect["backlashBonus"], backlash_bonus);
            assert_eq!(effect["backlashDamageType"], backlash_damage_type);
            assert_eq!(effect["backlashUsesResistance"], backlash_uses_resistance);
        }
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let aggravation = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 1,
                ..LegacyItemEntry::default()
            },
            "aggravation-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            aggravation["useAction"]["effect"]["type"],
            "aggravate-monsters"
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let genocide = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 44,
                ..LegacyItemEntry::default()
            },
            "genocide-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            genocide["useAction"]["effect"],
            serde_json::json!({"type": "genocide", "power": 300})
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let mass_genocide = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 45,
                ..LegacyItemEntry::default()
            },
            "mass-genocide-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            mass_genocide["useAction"]["effect"],
            serde_json::json!({"type": "mass-genocide", "power": 300, "radius": 20})
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let terrain_creation = TerrainCreationImportIds {
            source_terrain_ids: vec![
                "rfb-legacy.terrain.floor".to_owned(),
                "rfb-legacy.terrain.grass".to_owned(),
            ],
            tree_terrain_id: Some("rfb-legacy.terrain.tree".to_owned()),
            wall_terrain_id: Some("rfb-legacy.terrain.granite".to_owned()),
            ..TerrainCreationImportIds::default()
        };
        for (sval, target_terrain_id) in [
            (48, "rfb-legacy.terrain.tree"),
            (49, "rfb-legacy.terrain.granite"),
        ] {
            let value = item_json_with_terrain(
                &LegacyItemEntry {
                    tval: 70,
                    sval,
                    ..LegacyItemEntry::default()
                },
                &format!("terrain-creation-scroll-{sval}"),
                &LauncherAmmoIndex::default(),
                None,
                Some(&terrain_creation),
                &mut report,
            );
            assert_eq!(
                value["useAction"]["effect"],
                serde_json::json!({
                    "type": "create-adjacent-terrain",
                    "sourceTerrainIds": [
                        "rfb-legacy.terrain.floor",
                        "rfb-legacy.terrain.grass"
                    ],
                    "targetTerrainId": target_terrain_id
                })
            );
        }
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let vengeance = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 50,
                ..LegacyItemEntry::default()
            },
            "vengeance-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            vengeance["useAction"]["effect"],
            serde_json::json!({
                "type": "vengeance",
                "durationDice": 1,
                "durationSides": 25,
                "durationBonus": 25
            })
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let monster_confusion = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 36,
                ..LegacyItemEntry::default()
            },
            "monster-confusion-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            monster_confusion["useAction"]["effect"],
            serde_json::json!({"type": "prepare-confusing-strike"})
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let protection_from_evil = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 37,
                ..LegacyItemEntry::default()
            },
            "protection-from-evil-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(
            protection_from_evil["useAction"]["effect"],
            serde_json::json!({"type": "protection-from-evil"})
        );
        assert!(!report.item_behavior_gaps.contains_key("scroll-effect"));

        let _ = item_json(
            &LegacyItemEntry {
                tval: 70,
                sval: 19,
                ..LegacyItemEntry::default()
            },
            "unsupported-scroll",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert_eq!(report.item_behavior_gaps["scroll-effect"], 1);

        let gold = item_json(
            &LegacyItemEntry {
                tval: 127,
                ..LegacyItemEntry::default()
            },
            "gold",
            &LauncherAmmoIndex::default(),
            None,
            &mut report,
        );
        assert!(
            gold["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "gold") })
        );
    }

    #[test]
    fn spell_scroll_class_eligibility_matches_legacy_exceptions() {
        for (class_id, expected) in [("mage", true), ("sorcerer", false), ("red-mage", false)] {
            assert_eq!(legacy_class_uses_spell_scrolls(class_id), expected);
        }
    }

    #[test]
    fn a_info_artifacts_import_with_fixed_bonuses() {
        const SYNTHETIC_A_INFO: &str = "V:1.1.0
N:1:of Test Radiance
I:39:4:3
W:30:1:10:10000
P:0:1d1:0:0:0
F:WIS | INSTA_ART | RES_DARK
E:LITE_AREA:10:15
E:某个中文名

N:2:'Test Fang'
I:23:17:2
W:20:5:130:50000
P:0:2d6:10:15:0
F:DEX | SLAY_EVIL | KILL_DRAGON | BRAND_FIRE | BRAND_CHAOS

N:3:of Test Warding
I:45:20:4
W:40:8:2:80000
F:CON | SPEED | FREE_ACT | IM_FIRE | VULN_COLD

N:4:of Test Melody
I:19:70:2
W:30:5:60:50000
P:0:0d0:5:5:0
F:CHR
";
        let artifacts = parse_a_info(SYNTHETIC_A_INFO).expect("synthetic artifacts should parse");
        assert_eq!(artifacts.len(), 4);
        assert_eq!(artifacts[0].rarity_one_in, 1);
        assert_eq!(artifacts[1].rarity_one_in, 5);
        let base_items = [LegacyItemEntry {
            index: 1,
            name: "Test Light".to_owned(),
            glyph: Some('~'),
            tval: 39,
            sval: 4,
            ..LegacyItemEntry::default()
        }];
        let outcome = convert_content(
            &[],
            &[],
            &base_items,
            &[],
            &artifacts,
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.artifacts_total, 4);
        assert_eq!(outcome.report.artifacts_imported, 4);

        let get = |name: &str| {
            outcome
                .item_files
                .iter()
                .find(|(file, _)| file == name)
                .map(|(_, value)| value)
                .unwrap_or_else(|| panic!("{name} should be generated"))
        };

        // Light artifacts occupy the body template's light slot, so their
        // fixed pval folds into attributes (contract-v100).
        let radiance = get("artifact-test-radiance.json");
        assert_eq!(radiance["id"], "rfb-legacy.item.artifact-test-radiance");
        assert_eq!(radiance["glyph"], "*");
        assert_eq!(radiance["generationLevel"], 30);
        assert_eq!(radiance["artifactGeneration"]["sourceIndex"], 1);
        assert_eq!(
            radiance["artifactGeneration"]["baseItemKindId"],
            "rfb-legacy.item.test-light"
        );
        assert_eq!(radiance["artifactGeneration"]["rarityOneIn"], 1);
        assert_eq!(radiance["artifactGeneration"]["instant"], true);
        assert_eq!(radiance["equipmentSlot"], "light");
        assert_eq!(radiance["modifiers"]["wisdom"], 3);
        assert_eq!(radiance["resistances"]["dark"], "resistant");
        assert!(!outcome.report.unmapped_artifact_flags.contains_key("WIS"));
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("RES_DARK")
        );
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("INSTA_ART")
        );
        assert_eq!(outcome.report.item_behavior_gaps["artifact-activation"], 1);

        let fang = get("artifact-test-fang.json");
        assert_eq!(fang["equipmentSlot"], "weapon");
        assert_eq!(fang["meleeProfile"]["damageDice"], 2);
        assert_eq!(fang["meleeProfile"]["damageSides"], 6);
        assert_eq!(fang["meleeProfile"]["toHit"], 10);
        assert_eq!(fang["meleeProfile"]["toDamage"], 15);
        assert_eq!(fang["modifiers"]["dexterity"], 2);
        assert_eq!(fang["slays"]["evil"], "slay");
        assert_eq!(fang["slays"]["dragon"], "kill");
        assert_eq!(fang["brands"][0], "chaos");
        assert_eq!(fang["brands"][1], "fire");
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("SLAY_EVIL")
        );
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("KILL_DRAGON")
        );
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("BRAND_FIRE")
        );
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("BRAND_CHAOS")
        );

        // Fixed-pval jewelry proves the split: the base ring stays bare while
        // the artifact carries the attribute bonus. The defensive fold rides
        // the same fixed pval for SPEED and maps the tiered flags.
        let warding = get("artifact-test-warding.json");
        assert_eq!(warding["equipmentSlot"], "ring");
        assert_eq!(warding["modifiers"]["constitution"], 4);
        assert_eq!(warding["modifiers"]["speed"], 4);
        assert_eq!(warding["resistances"]["fire"], "immune");
        assert_eq!(warding["resistances"]["cold"], "vulnerable");
        assert_eq!(warding["statusImmunities"][0], "rfb.status.paralysis");
        assert!(!outcome.report.unmapped_artifact_flags.contains_key("SPEED"));
        assert!(
            !outcome
                .report
                .unmapped_artifact_flags
                .contains_key("FREE_ACT")
        );
        assert_eq!(warding["maxStack"], 1);

        // Fake-bow artifacts keep the launcher slot and their fixed
        // attributes even without a projectile profile; the shooting-only
        // P: bonuses are dropped alongside the profile.
        let melody = get("artifact-test-melody.json");
        assert_eq!(melody["equipmentSlot"], "launcher");
        assert!(melody.get("projectileProfile").is_none());
        assert_eq!(melody["modifiers"]["charisma"], 2);
        assert!(melody["modifiers"].get("attack").is_none());
        assert_eq!(outcome.report.item_behavior_gaps["launcher-unpaired"], 1);
    }

    #[test]
    fn misc_tokens_map_to_small_effect_forms() {
        const WARDEN_R_INFO: &str = "N:8:test veil keeper
G:u:v
I:110:5d5:20:10:10:10
W:20:2:20:9:10:40
B:HIT:HURT(1d4):DRAIN_CHARGES
S:1_IN_3 | TELE_OTHER | DRAIN_MANA | AMNESIA | DISPEL_MAGIC | DARKNESS | ANIM_DEAD | ANTI_MAGIC
";
        let monsters = parse_r_info(WARDEN_R_INFO).expect("synthetic warden should parse");
        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let (_, keeper) = &outcome.actor_files[0];
        let ability_ids: Vec<&str> = keeper["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.banish",
                "rfb-legacy.ability.drain-mana-11",
                "rfb-legacy.ability.amnesia",
                "rfb-legacy.ability.dispel",
                "rfb-legacy.ability.darkness",
                "rfb-legacy.ability.animate-dead",
                "rfb-legacy.ability.anti-magic",
            ]
        );
        assert_eq!(
            keeper["meleeRoutine"]["blows"][0]["effects"][1]["type"],
            "drain-charges"
        );
        assert!(!outcome.report.unmapped_spells.contains_key("DARKNESS"));
        let drain = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "drain-mana-11.json")
            .map(|(_, value)| value)
            .expect("drain ability should be generated");
        assert_eq!(drain["effect"]["type"], "drain-resource");
        assert_eq!(drain["effect"]["amount"], 11);
        let banish = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "banish.json")
            .map(|(_, value)| value)
            .expect("banish ability should be generated");
        assert_eq!(banish["effect"]["type"], "teleport-away");
        assert_eq!(banish["effect"]["minimumDistance"], 10);
        let darkness = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "darkness.json")
            .map(|(_, value)| value)
            .expect("darkness ability should be generated");
        assert_eq!(darkness["effect"]["type"], "darken-room");
        assert_eq!(darkness["target"]["requiresLineOfEffect"], false);
        let animate_dead = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "animate-dead.json")
            .map(|(_, value)| value)
            .expect("animate dead ability should be generated");
        assert_eq!(animate_dead["target"]["modes"], serde_json::json!(["self"]));
        assert_eq!(animate_dead["effect"]["type"], "sequence");
        assert_eq!(
            animate_dead["effect"]["effects"][0]["corpseItemKindId"],
            DEMO_CORPSE_ITEM_ID
        );
        assert_eq!(
            animate_dead["effect"]["effects"][0]["failureChancePercent"],
            20
        );
        assert_eq!(
            animate_dead["effect"]["effects"][1]["corpseItemKindId"],
            DEMO_SKELETON_ITEM_ID
        );
        assert_eq!(
            animate_dead["effect"]["effects"][1]["failureChancePercent"],
            40
        );
    }

    #[test]
    fn curse_tokens_map_to_save_gated_damage() {
        const CURSER_R_INFO: &str = "N:7:test doom whisperer
G:p:D
I:110:5d5:20:10:10:10
W:25:2:20:9:10:40
B:HIT:HURT(1d4)
S:1_IN_3 | CAUSE_1 | CAUSE_4 | HAND_DOOM
";
        let monsters = parse_r_info(CURSER_R_INFO).expect("synthetic curser should parse");
        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let (_, whisperer) = &outcome.actor_files[0];
        let ability_ids: Vec<&str> = whisperer["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.curse-3d8",
                "rfb-legacy.ability.curse-15d15",
                "rfb-legacy.ability.hand-of-doom",
            ]
        );
        assert!(!outcome.report.unmapped_spells.contains_key("HAND_DOOM"));
        let curse = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "curse-15d15.json")
            .map(|(_, value)| value)
            .expect("heavy curse should be generated");
        assert_eq!(curse["effect"]["type"], "curse-damage");
        assert_eq!(curse["effect"]["damageDice"], 15);
        assert_eq!(curse["effect"]["damageSides"], 15);
        assert!(curse["effect"].get("damageType").is_none());
        let doom = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "hand-of-doom.json")
            .map(|(_, value)| value)
            .expect("Hand of Doom should be generated");
        assert_eq!(doom["effect"]["type"], "curse-damage");
        assert_eq!(doom["effect"]["damageDice"], 1);
        assert_eq!(doom["effect"]["damageSides"], 20);
        assert_eq!(doom["effect"]["damageBonus"], 40);
        assert_eq!(doom["effect"]["damageIsCurrentHpPercent"], true);
        assert_eq!(doom["effect"]["nonlethal"], true);
    }

    #[test]
    fn mental_tokens_map_to_psi_sequences_and_beam() {
        const PSION_R_INFO: &str = "N:6:test mind flenser
G:h:v
I:110:6d6:20:15:10:10
W:30:2:20:9:10:40
B:HIT:HURT(1d5)
S:1_IN_3 | MIND_BLAST | BRAIN_SMASH(200) | PSY_SPEAR
";
        let monsters = parse_r_info(PSION_R_INFO).expect("synthetic psion should parse");
        assert_eq!(monsters.len(), 1);

        let outcome = convert_content(
            &[],
            &monsters,
            &[],
            &[],
            &[],
            &LegacyCharacterSources::default(),
        );
        let (_, flenser) = &outcome.actor_files[0];
        let ability_ids: Vec<&str> = flenser["monsterCasting"]["abilities"]
            .as_array()
            .expect("casting should list abilities")
            .iter()
            .map(|entry| entry["abilityId"].as_str().expect("ability id"))
            .collect();
        assert_eq!(
            ability_ids,
            [
                "rfb-legacy.ability.mind-blast-7d7",
                "rfb-legacy.ability.brain-smash-1d1-199",
                "rfb-legacy.ability.psy-spear-1d45-100",
            ]
        );

        let blast = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "mind-blast-7d7.json")
            .map(|(_, value)| value)
            .expect("mind blast should be generated");
        let effects = blast["effect"]["effects"]
            .as_array()
            .expect("mind blast should be a sequence");
        assert_eq!(effects.len(), 2);
        assert_eq!(effects[0]["damageType"], "psi");
        assert_eq!(effects[1]["statusKindId"], "rfb.status.confusion");
        assert_eq!(effects[1]["resistanceType"], "psi");

        let smash = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "brain-smash-1d1-199.json")
            .map(|(_, value)| value)
            .expect("brain smash should be generated");
        let effects = smash["effect"]["effects"]
            .as_array()
            .expect("brain smash should be a sequence");
        // Flat 200 encodes through the 1d1+(N-1) identity, followed by the
        // four legacy riders in declaration order.
        assert_eq!(effects.len(), 5);
        assert_eq!(effects[0]["damageDice"], 1);
        assert_eq!(effects[0]["damageBonus"], 199);
        let riders: Vec<&str> = effects[1..]
            .iter()
            .map(|effect| effect["statusKindId"].as_str().expect("status id"))
            .collect();
        assert_eq!(
            riders,
            [
                "rfb.status.blindness",
                "rfb.status.confusion",
                "rfb.status.paralysis",
                "rfb.status.slow",
            ]
        );

        let spear = outcome
            .ability_files
            .iter()
            .find(|(name, _)| name == "psy-spear-1d45-100.json")
            .map(|(_, value)| value)
            .expect("psy spear should be generated");
        assert_eq!(spear["effect"]["type"], "beam-damage");
        assert_eq!(spear["effect"]["damageType"], "psi");
        assert_eq!(spear["effect"]["damageSides"], 45);
        assert_eq!(spear["effect"]["damageBonus"], 100);
    }

    #[test]
    fn melee_brain_smash_reuses_psi_damage_and_status_riders() {
        let mut monsters = parse_r_info(
            "N:781:Ultimate beholder\nG:e:o\nI:120:40d100:30:80:10:100\nW:66:4:999:18000:0:0\nB:GAZE:BRAIN_SMASH(5d5)\nF:FORCE_MAXHP\n",
        )
        .expect("synthetic beholder should parse");
        let actor = demo_monster_json(
            &monsters.remove(0),
            &DemoMonsterSelectionEntry {
                source_index: 781,
                source_id: None,
                id: "ultimate-beholder".to_owned(),
                tags: vec!["orc-cave".to_owned()],
                omitted_flags: Vec::new(),
                omitted_spells: Vec::new(),
            },
            &mut BTreeMap::new(),
        )
        .expect("melee Brain Smash should import");
        let effects = actor["meleeRoutine"]["blows"][0]["effects"]
            .as_array()
            .expect("Brain Smash should keep its effect sequence");
        assert_eq!(effects.len(), 5);
        assert_eq!(effects[0]["type"], "damage");
        assert_eq!(effects[0]["damageType"], "psi");
        assert_eq!(effects[0]["damageDice"], 5);
        assert_eq!(effects[0]["damageSides"], 5);
        assert_eq!(
            effects[1..]
                .iter()
                .map(|effect| effect["type"].as_str().expect("effect type"))
                .collect::<Vec<_>>(),
            ["blind", "confusion", "paralysis", "slow"]
        );
    }
}
