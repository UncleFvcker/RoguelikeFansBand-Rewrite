// SPDX-License-Identifier: MPL-2.0

//! Read-only import of legacy `f_info.txt`/`r_info.txt` content into local
//! rfb-content JSON fragments. Everything expressible is written to a
//! git-ignored output directory; everything else is aggregated into a gap
//! report so rule work can be prioritised from data. No legacy text enters
//! the repository: unit tests use synthetic samples only.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use serde::Serialize;

use crate::{LEGACY_BASELINE_COMMIT, LegacyImportError};

pub const CONTENT_IMPORT_SCHEMA_VERSION: u16 = 2;
const SCHEMA_BASE: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1";
const F_INFO_SOURCE: &str = "lib/edit/f_info.txt";
const R_INFO_SOURCE: &str = "lib/edit/r_info.txt";
const K_INFO_SOURCE: &str = "lib/edit/k_info.txt";
const E_INFO_SOURCE: &str = "lib/edit/e_info.txt";
const A_INFO_SOURCE: &str = "lib/edit/a_info.txt";
const B_INFO_SOURCE: &str = "lib/edit/b_info.txt";
const M_INFO_SOURCE: &str = "lib/edit/m_info.txt";
const S_INFO_SOURCE: &str = "lib/edit/s_info.txt";

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyTerrainEntry {
    pub index: u32,
    pub tag: String,
    pub display_name: Option<String>,
    pub glyph: Option<char>,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LegacyBlow {
    pub method: String,
    pub damage_dice: Option<(u16, u16)>,
    pub effects: Vec<String>,
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
    pub blows: Vec<LegacyBlow>,
    pub flags: Vec<String>,
    pub spells: Vec<String>,
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
    pub weight_tenths_pound: u16,
    pub armor_class: i32,
    pub damage_dice: Option<(u16, u16)>,
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
    pub weight_tenths_pound: u16,
    pub armor_class: i32,
    pub damage_dice: Option<(u16, u16)>,
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

/// A race or personality extracted from the legacy C sources. `dynamic`
/// marks blocks whose scalar fields are computed (rank-scaled monster
/// races); those cannot be represented as static content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyCharacterEntry {
    pub id: String,
    pub stats: [i32; 6],
    pub skills: [i32; 8],
    pub extra_skills: [i32; 8],
    pub life: i32,
    pub base_hp: i32,
    pub exp: i32,
    pub infra: i32,
    pub flags: Vec<String>,
    pub hooks: Vec<String>,
    pub dynamic: bool,
    /// Right-hand symbol of `me.calc_bonuses = _fn;` when present, so the
    /// hook body can be mined for its static defensive surface.
    pub calc_bonuses_fn: Option<String>,
    /// Damage-type/tier pairs recovered from top-level `res_add` family
    /// statements in the calc_bonuses hook.
    pub resistances: Vec<(String, String)>,
    pub free_act: bool,
    pub speed: i32,
}

impl Default for LegacyCharacterEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            stats: [0; 6],
            skills: [0; 8],
            extra_skills: [0; 8],
            life: 100,
            base_hp: 0,
            exp: 100,
            infra: 0,
            flags: Vec::new(),
            hooks: Vec::new(),
            dynamic: false,
            calc_bonuses_fn: None,
            resistances: Vec::new(),
            free_act: false,
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
    pub weapon_entries: usize,
    pub skill_entries: BTreeMap<u16, usize>,
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

/// Reads one path from the pinned legacy commit via git objects, never the
/// working tree.
pub fn read_legacy_object(source: &Path, path: &str) -> Result<String, LegacyImportError> {
    let resolved = Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("rev-parse")
        .arg(format!("{LEGACY_BASELINE_COMMIT}^{{commit}}"))
        .output()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if !resolved.status.success() {
        return Err(LegacyImportError::LegacyGit(
            String::from_utf8_lossy(&resolved.stderr).trim().to_owned(),
        ));
    }
    let commit = String::from_utf8_lossy(&resolved.stdout).trim().to_owned();
    if commit != LEGACY_BASELINE_COMMIT {
        return Err(LegacyImportError::LegacyGit(format!(
            "resolved commit {commit} does not match the pinned baseline"
        )));
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .arg("show")
        .arg(format!("{LEGACY_BASELINE_COMMIT}:{path}"))
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

fn list_legacy_c_sources(source: &Path) -> Result<Vec<String>, LegacyImportError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source)
        .args([
            "ls-tree",
            "-r",
            "--name-only",
            LEGACY_BASELINE_COMMIT,
            "--",
            "src",
        ])
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
        // Effects mostly carry their dice inline: HURT(2d6), POISON(1d4),
        // DAM(3d8)... the first diced effect provides the melee damage.
        if let Some((token, parameters)) = part.split_once('(') {
            let dice = parameters.strip_suffix(')').ok_or_else(|| {
                content_parse_error(
                    R_INFO_SOURCE,
                    line_number,
                    "B.effect",
                    part,
                    "effect parameters are missing a closing parenthesis",
                )
            })?;
            let token = required_field(R_INFO_SOURCE, line_number, "B.effect", Some(token))?;
            if blow.damage_dice.is_none()
                && let Some((dice_count, sides)) = dice.split_once('d')
                && dice_count.bytes().all(|byte| byte.is_ascii_digit())
                && sides.bytes().all(|byte| byte.is_ascii_digit())
            {
                let dice_count =
                    parse_number(R_INFO_SOURCE, line_number, "B.damageDice", Some(dice_count))?;
                let sides = parse_number(R_INFO_SOURCE, line_number, "B.damageSides", Some(sides))?;
                blow.damage_dice = Some((dice_count, sides));
                blow.effects.insert(0, token.to_owned());
                continue;
            }
            blow.effects.push(token.to_owned());
        } else {
            blow.effects.push(part.to_owned());
        }
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
        let recognized = ["G:", "I:", "W:", "B:", "F:", "S:"]
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
            for (field, value) in [
                ("W.extra", parts.get(2).copied()),
                ("W.experience", parts.get(3).copied()),
                ("W.evolution", parts.get(4).copied()),
                ("W.nextEvolution", parts.get(5).copied()),
            ] {
                let _: i64 = parse_number(R_INFO_SOURCE, line_number, field, value)?;
            }
        } else if let Some(rest) = line.strip_prefix("B:") {
            entry.blows.push(parse_blow(rest, line_number)?);
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
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    Ok(entries)
}

const MAPPED_TERRAIN_FLAGS: [&str; 4] = ["MOVE", "LOS", "PROJECT", "PERMANENT"];

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
        let recognized = ["G:", "I:", "W:", "P:", "F:"]
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
            let _: i64 = parse_number(
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
            let _: i64 = parse_number(K_INFO_SOURCE, line_number, "W.cost", parts.get(4).copied())?;
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
        20..=23 => ItemShape {
            slot: Some("weapon"),
            max_stack: 1,
            tags: vec!["equipment", "legacy-import", "weapon"],
            melee: true,
            launcher: false,
            behavior_gap: None,
        },
        19 => ItemShape {
            slot: Some("launcher"),
            max_stack: 1,
            tags: vec!["equipment", "launcher", "legacy-import"],
            melee: false,
            launcher: true,
            behavior_gap: Some("launcher-multiplier"),
        },
        16..=18 => ItemShape {
            slot: None,
            max_stack: 99,
            tags: vec!["ammunition", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: Some("ammo-dice-folded"),
        },
        36..=38 => ItemShape {
            slot: Some("body"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        34 | 35 => ItemShape {
            slot: Some("head"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        33 => ItemShape {
            slot: Some("shield"),
            max_stack: 1,
            tags: vec!["armor", "equipment", "legacy-import"],
            melee: false,
            launcher: false,
            behavior_gap: None,
        },
        32 => ItemShape {
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
        75 | 80 => ItemShape {
            slot: None,
            max_stack: 20,
            tags: vec!["consumable", "legacy-import"],
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

fn launcher_ammo_for(entry: &LegacyItemEntry, ammo: &LauncherAmmoIndex) -> Option<String> {
    match entry.sval {
        2 => ammo.shot.clone(),
        12 | 13 => ammo.arrow.clone(),
        23 | 24 => ammo.bolt.clone(),
        _ => None,
    }
}

fn player_ability_book_for_item(entry: &LegacyItemEntry) -> Option<&'static str> {
    if entry.tval != DEATH_BOOK_TVAL {
        return None;
    }
    match entry.sval {
        DEATH_FIRST_BOOK_SVAL => Some(DEATH_FIRST_BOOK_ID),
        DEATH_SECOND_BOOK_SVAL => Some(DEATH_SECOND_BOOK_ID),
        DEATH_THIRD_BOOK_SVAL => Some(DEATH_THIRD_BOOK_ID),
        DEATH_FOURTH_BOOK_SVAL => Some(DEATH_FOURTH_BOOK_ID),
        _ => None,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct TerrainCreationImportIds {
    source_terrain_ids: Vec<String>,
    tree_terrain_id: Option<String>,
    wall_terrain_id: Option<String>,
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
    let effect = match (entry.tval, entry.sval) {
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
        (70, 43) => serde_json::json!({
            "type": "increase-spell-learning-capacity"
        }),
        (70, 25) => detect("terrain", "map", true),
        (70, 26) => detect("item", "gold", false),
        (70, 27) => detect("item", "item", false),
        (70, 28) => detect("terrain", "trap", true),
        (70, 29) => detect("terrain", "passage", true),
        (70, 30) => detect("actor", "invisible", false),
        (70, 33) => bless(12, 6),
        (70, 34) => bless(24, 12),
        (70, 35) => bless(48, 24),
        (70, 36) => serde_json::json!({"type": "prepare-confusing-strike"}),
        (70, 37) => serde_json::json!({"type": "protection-from-evil"}),
        (70, 39) => serde_json::json!({
            "type": "destroy-adjacent-traps-and-doors"
        }),
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
        (70, 62) => serde_json::json!({
            "type": "banish-visible",
            "maximumDistance": 150
        }),
        (75, 4) => serde_json::json!({
            "type": "apply-slowness",
            "durationDice": 1,
            "durationSides": 25,
            "durationBonus": 15
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
        (75, 6) => serde_json::json!({
            "type": "apply-poison",
            "durationDice": 1,
            "durationSides": 15,
            "durationBonus": 9
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
        (75, 39) => sequence(vec![
            serde_json::json!({"type": "heal", "amount": 5000}),
            remove_status("rfb.status.poison"),
            remove_status("rfb.status.blindness"),
            remove_status("rfb.status.confusion"),
            remove_status("rfb.status.stun"),
            remove_status("rfb.status.bleeding"),
            remove_status("rfb.status.slow"),
            remove_status("rfb.status.berserk"),
        ]),
        (75, 40) => sequence(vec![
            serde_json::json!({
                "type": "restore-resource-full",
                "resourceId": LEGACY_MANA_RESOURCE_ID
            }),
            remove_status("rfb.status.berserk"),
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
        _ => return None,
    };
    Some(serde_json::json!({"effect": effect}))
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
    let use_action = fixed_consumable_use_action_with_terrain(entry, terrain_creation);
    let device_generation = legacy_device_generation(entry);
    if let Some(gap) = shape.behavior_gap
        && ability_book_id.is_none()
        && use_action.is_none()
        && device_generation.is_none()
    {
        *report.item_behavior_gaps.entry(gap.to_owned()).or_default() += 1;
    }
    let mut value = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/item.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.item.{id}"),
        "nameKey": format!("item-legacy-{id}-name"),
        "descriptionKey": format!("item-legacy-{id}-description"),
        "glyph": entry.glyph.map_or_else(|| "?".to_owned(), |glyph| glyph.to_string()),
        "weightTenthsPound": entry.weight_tenths_pound.max(1),
        "maxStack": shape.max_stack,
        "tags": tags,
    });
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
    if let Some(slot) = shape.slot {
        value["equipmentSlot"] = serde_json::json!(slot);
    }
    if shape.max_stack > 1 && shape.tags_contain("ammunition") {
        value["breakChancePercent"] = serde_json::json!(25);
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
    if shape.launcher {
        // Instruments and other exotic entries share the legacy bow tval;
        // only launchers with a canonical ammo partner keep the profile.
        // The rest stay equippable fake bows (legacy obj_is_fake_bow):
        // they occupy the launcher slot but cannot fire.
        if let Some(ammo_kind_id) = launcher_ammo_for(entry, ammo) {
            value["projectileProfile"] = serde_json::json!({
                "range": 12,
                "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
                "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
                "damageDice": 2,
                "damageSides": 5,
                "ammoKindId": ammo_kind_id,
            });
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
    let equipment = if shape.slot.is_some() {
        equipment_fold(&entry.flags, entry.pval)
    } else {
        EquipmentFold::default()
    };
    let mut modifiers = serde_json::Map::new();
    let defense = entry.armor_class.saturating_add(entry.to_armor);
    if shape.slot.is_some() && defense != 0 {
        modifiers.insert("defense".to_owned(), serde_json::json!(defense));
    }
    if fold.speed != 0 {
        modifiers.insert("speed".to_owned(), serde_json::json!(fold.speed));
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    for flag in &entry.flags {
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
                let _: i64 = parse_number(
                    E_INFO_SOURCE,
                    line_number,
                    "W.rarity",
                    parts.get(1).copied(),
                )?;
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
            let _: i64 = parse_number(
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
            let _: i64 = parse_number(A_INFO_SOURCE, line_number, "W.cost", parts.get(3).copied())?;
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
/// and race hooks, mapped to the content damage-type vocabulary. FEAR and
/// BLIND have no damage type and stay in the gap reports.
const DEFENSIVE_RESISTANCE_TYPES: [(&str, &str); 15] = [
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
];

fn defensive_resistance_type(token: &str) -> Option<&'static str> {
    DEFENSIVE_RESISTANCE_TYPES
        .iter()
        .find(|(known, _)| *known == token)
        .map(|(_, damage_type)| *damage_type)
}

/// Durability and display flags with no RFB behaviour to express: items
/// never corrode and names always render from content keys.
fn item_flag_not_applicable(flag: &str) -> bool {
    flag.starts_with("IGNORE_") || matches!(flag, "SHOW_MODS" | "HIDE_TYPE" | "FULL_NAME")
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

const OFFENSIVE_BRANDS: [(&str, &str); 5] = [
    ("BRAND_ACID", "acid"),
    ("BRAND_ELEC", "electricity"),
    ("BRAND_FIRE", "fire"),
    ("BRAND_COLD", "cold"),
    ("BRAND_POIS", "poison"),
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
    for (flag, passive) in [("REGEN", "regeneration"), ("BRAND_VAMP", "vampiric")] {
        if flags.iter().any(|value| value == flag) {
            fold.passives.push(passive);
            fold.consumed.insert(flag.to_owned());
        }
    }
    fold.passives.sort_unstable();
    fold
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
    // The C: maxima become fixed modifiers: a deterministic ceiling standing
    // in for the legacy generation-time random rolls (documented difference).
    let mut modifiers = serde_json::Map::new();
    let attack = entry.max_to_hit.max(entry.max_to_damage);
    if attack != 0 {
        modifiers.insert("attack".to_owned(), serde_json::json!(attack));
    }
    if entry.max_to_armor != 0 {
        modifiers.insert("defense".to_owned(), serde_json::json!(entry.max_to_armor));
    }
    attribute_modifiers_from_flags(&entry.flags, entry.max_pval, &mut modifiers);
    // Defensive flags ride the same generation-time ceiling as attributes:
    // SPEED folds the max pval, resistances and free action are binary.
    let fold = defensive_fold(&entry.flags, entry.max_pval);
    let offense = offensive_fold(&entry.flags);
    let equipment = equipment_fold(&entry.flags, entry.max_pval);
    if fold.speed != 0 {
        modifiers.insert("speed".to_owned(), serde_json::json!(fold.speed));
    }
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
        "tags": tags,
    });
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    apply_ego_roll_recipe(&mut value, entry);
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
        // Original boots/ring speed pvals are depth-biased generation rolls.
        // Increasing minDepth thresholds keep high bonuses out of shallow
        // instances while preserving the materialized per-item value.
        148 | 209 => vec![serde_json::json!({
            "rolls": 1,
            "candidates": speed_roll_candidates(entry.index == 209),
        })],
        _ => Vec::new(),
    };
    if !groups.is_empty() {
        value["rollGroups"] = serde_json::Value::Array(groups);
    }
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
        "weightTenthsPound": entry.weight_tenths_pound.max(1),
        "maxStack": 1,
        "tags": ["artifact", "legacy-import"],
    });
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
        let paired = launcher_ammo_for(
            &LegacyItemEntry {
                sval: entry.sval,
                ..LegacyItemEntry::default()
            },
            ammo,
        );
        if let Some(ammo_kind_id) = paired {
            value["projectileProfile"] = serde_json::json!({
                "range": 12,
                "toHit": entry.to_hit.clamp(-1_000_000, 1_000_000),
                "toDamage": entry.to_damage.clamp(-1_000_000, 1_000_000),
                "damageDice": 2,
                "damageSides": 5,
                "ammoKindId": ammo_kind_id,
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
    let equipment = if has_slot {
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
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    apply_defensive_fold(&mut value, &fold);
    apply_offensive_fold(&mut value, &offense);
    apply_equipment_fold(&mut value, &equipment);
    if entry.has_activation {
        *report
            .item_behavior_gaps
            .entry("artifact-activation".to_owned())
            .or_default() += 1;
    }
    for flag in &entry.flags {
        // Slotless shapes never applied the attribute or defensive folds,
        // so their flags stay visible in the gap report.
        if (has_slot && attribute_flag_is_mapped(flag)) || flag == "INSTA_ART" {
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

/// Extracts the statically expressible defensive surface from a race's
/// calc_bonuses hook: top-level `res_add` family calls, `free_act++` and
/// literal `pspeed` adjustments. Conditional (level-gated) and computed
/// statements are ignored and remain accounted as hook gaps.
pub fn parse_calc_bonuses_defenses(text: &str, hook: &str) -> (Vec<(String, String)>, bool, i32) {
    fn resistance_token(rest: &str) -> Option<&'static str> {
        let token = rest[..rest.find(')')?].trim();
        defensive_resistance_type(token.strip_prefix("RES_")?)
    }
    let Some(body) = find_function_body(text, hook) else {
        return (Vec::new(), false, 0);
    };
    let mut adds: BTreeMap<&'static str, i32> = BTreeMap::new();
    let mut immune: BTreeSet<&'static str> = BTreeSet::new();
    let mut free_act = false;
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
    (resistances, free_act, speed)
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
        let rhs = rest[eq_index + 1..].trim().trim_end_matches(';').trim_end();
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
                "name" | "desc" | "subname" | "subdesc" | "shop_adjust" => {}
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
                    entry.hooks.push(other.to_owned());
                }
            }
        }
    }
    entry
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

/// Parses `s_info.txt` only far enough to quantify the proficiency systems
/// that the current content model cannot yet represent.
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
            for (field, value) in [
                ("W.weaponType", parts.first().copied()),
                ("W.weaponSubtype", parts.get(1).copied()),
                ("W.minimum", parts.get(2).copied()),
                ("W.maximum", parts.get(3).copied()),
            ] {
                let _: i64 = parse_number(S_INFO_SOURCE, line_number, field, value)?;
            }
            profile.weapon_entries += 1;
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
            let _: i64 = parse_number(
                S_INFO_SOURCE,
                line_number,
                "S.minimum",
                parts.get(1).copied(),
            )?;
            let _: i64 = parse_number(
                S_INFO_SOURCE,
                line_number,
                "S.maximum",
                parts.get(2).copied(),
            )?;
            *profile.skill_entries.entry(skill_index).or_default() += 1;
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
    if entry.infra != 0 {
        *report.race_hook_gaps.entry("infra".to_owned()).or_default() += 1;
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
        "baseHp": entry.base_hp.clamp(-1_000, 1_000),
        "skillSetId": format!("rfb-legacy.skill-set.race-{}", entry.id),
        "kinCategory": format!("kin-glyph-{}", u32::from(legacy_race_kin_glyph(&entry.id))),
        "bodySlots": body_slots
            .iter()
            .map(|(id, slot_type)| serde_json::json!({"id": id, "slotType": slot_type}))
            .collect::<Vec<_>>(),
        "tags": ["legacy-import"],
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

fn magic_profile_json(profile: &LegacyMagicProfile, class_id: &str) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": LEGACY_BASELINE_COMMIT,
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
) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": LEGACY_BASELINE_COMMIT,
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
) -> serde_json::Value {
    let profiles = profiles
        .iter()
        .map(|profile| (profile.class_index, profile))
        .collect::<BTreeMap<_, _>>();
    serde_json::json!({
        "schemaVersion": 1,
        "sourceCommit": LEGACY_BASELINE_COMMIT,
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

fn death_spell_ability(spell: &LegacySpellProfile) -> Option<(String, serde_json::Value)> {
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
    let (id, target, effect, level_scaling, tags) = match spell.index {
        0 => (
            "death-detect-unlife",
            self_target.clone(),
            serde_json::json!({
                "type": "detect",
                "subject": "actor",
                "category": "nonliving",
                "radius": 8,
            }),
            Vec::new(),
            vec!["death", "detection", "spell"],
        ),
        1 => (
            "death-malediction",
            directional_target.clone(),
            serde_json::json!({
                "type": "damage",
                "damageDice": 3,
                "damageSides": 4,
                "damageType": "hell-fire",
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
                "radius": 8,
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
                "durationTicks": 300,
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
            self_target.clone(),
            serde_json::json!({
                "type": "apply-status",
                "statusKindId": "rfb.status.poison-branding",
                "intensity": 1,
                "durationTicks": 500,
                "stacking": "replace",
                "grantedBrands": ["poison"],
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
                    {"maximumRoll": 30, "effect": {"type": "no-op", "reason": "actor-polymorph-pending"}},
                    {"maximumRoll": 35, "effect": {"type": "bolt-or-beam-damage", "damageDice": 4, "damageSides": 4, "damageType": "physical", "beamChancePercent": 0}},
                    {"maximumRoll": 40, "effect": {"type": "apply-status", "statusKindId": "rfb.status.confusion", "intensity": 1, "durationTicks": 10, "stacking": "keep-strongest"}},
                    {"maximumRoll": 45, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 24, "damageType": "poison", "radius": 3}},
                    {"maximumRoll": 50, "effect": {"type": "no-op", "reason": "line-light-pending"}},
                    {"maximumRoll": 55, "effect": {"type": "bolt-or-beam-damage", "damageDice": 4, "damageSides": 8, "damageType": "electricity", "beamChancePercent": 0}},
                    {"maximumRoll": 60, "effect": {"type": "bolt-or-beam-damage", "damageDice": 6, "damageSides": 8, "damageType": "cold", "beamChancePercent": 0}},
                    {"maximumRoll": 65, "effect": {"type": "bolt-or-beam-damage", "damageDice": 7, "damageSides": 8, "damageType": "acid", "beamChancePercent": 0}},
                    {"maximumRoll": 70, "effect": {"type": "bolt-or-beam-damage", "damageDice": 9, "damageSides": 8, "damageType": "fire", "beamChancePercent": 0}},
                    {"maximumRoll": 75, "effect": {"type": "drain-life", "damageDice": 1, "damageSides": 1, "damageBonus": 74, "damageType": "nether", "targetCategory": "living"}},
                    {"maximumRoll": 80, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 34, "damageType": "electricity", "radius": 2}},
                    {"maximumRoll": 85, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 49, "damageType": "acid", "radius": 2}},
                    {"maximumRoll": 90, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 79, "damageType": "ice", "radius": 3}},
                    {"maximumRoll": 95, "effect": {"type": "area-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 89, "damageType": "fire", "radius": 3}},
                    {"maximumRoll": 100, "effect": {"type": "drain-life", "damageDice": 1, "damageSides": 1, "damageBonus": 109, "damageType": "nether", "targetCategory": "living"}},
                    {"maximumRoll": 103, "target": "self-target", "effect": {"type": "no-op", "reason": "earthquake-pending"}},
                    {"maximumRoll": 105, "target": "self-target", "effect": {"type": "no-op", "reason": "destroy-area-pending"}},
                    {"maximumRoll": 107, "effect": {"type": "genocide", "scope": "glyph", "power": 60}},
                    {"maximumRoll": 109, "target": "self-target", "effect": {"type": "visible-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 119}},
                    {"maximumRoll": 120, "target": "self-target", "effect": {"type": "visible-damage", "damageDice": 1, "damageSides": 1, "damageBonus": 149}},
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
            self_target.clone(),
            serde_json::json!({"type": "enchant-equipped-weapon", "affixId": LEGACY_DEATH_WEAPON_AFFIX_ID}),
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
    Some((ability_id, ability))
}

fn death_first_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_FIRST_BOOK_ID,
        "nameKey": "ability-book-legacy-death-stench-of-death-name",
        "descriptionKey": "ability-book-legacy-death-stench-of-death-description",
        "abilityIds": ability_ids,
        "tags": ["death", "legacy-import", "spellbook"],
    })
}

fn death_second_book_json(ability_ids: &[String]) -> serde_json::Value {
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/ability-book.schema.json"),
        "formatVersion": 1,
        "id": DEATH_SECOND_BOOK_ID,
        "nameKey": "ability-book-legacy-death-sepulchral-ways-name",
        "descriptionKey": "ability-book-legacy-death-sepulchral-ways-description",
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

fn terrain_json(entry: &LegacyTerrainEntry, id: &str) -> serde_json::Value {
    let walkable = entry.flags.iter().any(|flag| flag == "MOVE");
    let blocks_sight = !entry.flags.iter().any(|flag| flag == "LOS");
    let mut tags = vec!["legacy-import"];
    if entry.flags.iter().any(|flag| flag == "TRAP") {
        tags.push("trap");
    }
    if entry
        .flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "DOOR" | "STAIRS"))
    {
        tags.push("passage");
    }
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/terrain.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.terrain.{id}"),
        "nameKey": format!("terrain-legacy-{id}-name"),
        "descriptionKey": format!("terrain-legacy-{id}-description"),
        "glyph": entry.glyph.map_or_else(|| "?".to_owned(), |glyph| glyph.to_string()),
        "walkable": walkable,
        "blocksSight": blocks_sight,
        "tags": tags,
    })
}

/// Maps the dice-bearing effect token to our damage types; unknown tokens
/// fall back to physical dice and are counted in the gap report.
fn damage_type_for(blow: &LegacyBlow) -> (&'static str, Option<&str>) {
    match blow.effects.first().map(String::as_str) {
        Some("POISON") => ("poison", None),
        Some("FIRE") => ("fire", None),
        Some("COLD") => ("cold", None),
        Some("ACID") => ("acid", None),
        Some("ELEC") => ("electricity", None),
        Some("LITE") => ("light", None),
        Some("NETHER") => ("nether", None),
        Some("NEXUS") => ("nexus", None),
        Some("SHARDS") => ("shards", None),
        Some("DISENCHANT") => ("disenchant", None),
        Some("TIME") => ("time", None),
        Some("INERTIA") => ("inertia", None),
        Some("PLASMA") => ("plasma", None),
        Some("DISINTEGRATE") => ("disintegrate", None),
        Some("HELL_FIRE") => ("hell-fire", None),
        Some("HURT" | "DAM") | None => ("physical", None),
        Some(other) => ("physical", Some(other)),
    }
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
        _ => None,
    }
}

/// CAUSE curses gate on the player's saving throw instead of armour or
/// resistances; HAND_DOOM (percent-of-current-HP) stays a gap.
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
        _ => return None,
    };
    let (dice, sides, bonus) = match explicit {
        Some(spec) => parse_explicit_damage_dice(spec)?,
        None => default_dice,
    };
    let dice = dice.clamp(1, 100);
    let sides = sides.clamp(1, 10_000);
    let bonus = bonus.min(10_000);
    let mut suffix = format!("curse-{dice}d{sides}");
    if bonus > 0 {
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

const RESISTANCE_ALL_TYPES: [&str; 27] = [
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
];

fn monster_flag_is_mapped(flag: &str) -> bool {
    if matches!(flag, "RES_ALL" | "RES_TELE" | "NO_CONF") {
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
    resistances
}

fn monster_json(
    entry: &LegacyMonsterEntry,
    id: &str,
    blow: &LegacyBlow,
    damage_type: &str,
    melee_routine: Option<serde_json::Value>,
    monster_casting: Option<serde_json::Value>,
) -> serde_json::Value {
    let (hp_dice, hp_sides) = entry.hp_dice.unwrap_or((1, 1));
    let max_hp = ((hp_dice * (hp_sides + 1)) / 2).max(1);
    let level = entry.level.unwrap_or(1).max(1);
    let (damage_dice, damage_sides) = blow.damage_dice.unwrap_or((1, 1));
    // Legacy type flags become category tags so summon filters can select
    // by monster class; the shared legacy-import tag doubles as "any".
    let mut tags = vec!["legacy-import".to_owned()];
    if let Some(glyph) = entry.glyph {
        tags.push(format!("kin-glyph-{}", u32::from(glyph)));
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
        "speed": entry.speed.unwrap_or(110),
        "attack": (i32::from(level) / 4).max(1),
        "defense": (entry.armor_class.unwrap_or(0) / 10).max(0),
        "damageDice": damage_dice,
        "damageSides": damage_sides.max(1),
        "damageType": damage_type,
        "tags": tags,
    });
    let resistances = resistances_from_flags(&entry.flags);
    if !resistances.is_empty() {
        value["resistances"] = serde_json::json!(resistances);
    }
    if entry.flags.iter().any(|flag| flag == "NO_CONF") {
        value["statusImmunities"] = serde_json::json!(["rfb.status.confusion"]);
    }
    if let Some(routine) = melee_routine {
        value["meleeRoutine"] = routine;
    }
    if let Some(casting) = monster_casting {
        value["monsterCasting"] = casting;
    }
    if living {
        value["corpseItemKindId"] = serde_json::json!(LEGACY_CORPSE_ITEM_ID);
    }
    value
}

pub struct ContentImportOutcome {
    pub report: ContentImportReport,
    pub terrain_files: Vec<(String, serde_json::Value)>,
    pub actor_files: Vec<(String, serde_json::Value)>,
    pub ability_files: Vec<(String, serde_json::Value)>,
    pub ability_book_files: Vec<(String, serde_json::Value)>,
    pub resource_files: Vec<(String, serde_json::Value)>,
    pub item_files: Vec<(String, serde_json::Value)>,
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
const DEATH_BOOK_TVAL: u16 = 100;
const DEATH_FIRST_BOOK_SVAL: u16 = 0;
const DEATH_SECOND_BOOK_SVAL: u16 = 1;
const DEATH_THIRD_BOOK_SVAL: u16 = 2;
const DEATH_FOURTH_BOOK_SVAL: u16 = 3;
const DEATH_FIRST_BOOK_ID: &str = "rfb-legacy.ability-book.death-stench-of-death";
const DEATH_SECOND_BOOK_ID: &str = "rfb-legacy.ability-book.death-sepulchral-ways";
const DEATH_THIRD_BOOK_ID: &str = "rfb-legacy.ability-book.death-black-channels";
const DEATH_FOURTH_BOOK_ID: &str = "rfb-legacy.ability-book.death-necronomicon";
const LEGACY_VAMPIRE_LORD_RACE_ID: &str = "rfb-legacy.race.vampire-lord-form";
const LEGACY_VAMPIRE_LORD_SKILL_SET_ID: &str = "rfb-legacy.skill-set.race-vampire-lord-form";
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
    if let Some(id) = map_summon_spell_token(token, level, caster_kind_id, abilities) {
        return Some(id);
    }
    if let Some(id) = map_mental_spell_token(token, level, abilities) {
        return Some(id);
    }
    if let Some(id) = map_curse_spell_token(token, abilities) {
        return Some(id);
    }
    if let Some(id) = map_misc_spell_token(token, level, abilities) {
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
        "HEAL" => {
            let amount = u32::from(level).saturating_mul(3).clamp(5, 300);
            let id = format!("rfb-legacy.ability.heal-{amount}");
            abilities
                .entry(id.clone())
                .or_insert_with(|| heal_ability(amount));
            Some(id)
        }
        other => map_damage_spell_token(other, level, breath_radius, abilities),
    }
}

/// The two dice-based direct-damage shapes harvested from legacy S: lines.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DamageSpellShape {
    Bolt,
    Ball,
    /// Legacy MSF_BALL4 storms explode with radius four.
    BigBall,
}

impl DamageSpellShape {
    const fn keyword(self) -> &'static str {
        match self {
            Self::Bolt => "bolt",
            Self::Ball | Self::BigBall => "ball",
        }
    }

    const fn ball_radius(self) -> u8 {
        match self {
            Self::Bolt => 0,
            Self::Ball => 2,
            Self::BigBall => 4,
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
    use DamageSpellShape::{Ball, BigBall, Bolt};
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
        "BA_ACID" => (Ball, "acid", (1, 3 * level, 15)),
        "BA_ELEC" => (Ball, "electricity", (1, 3 * level / 2, 8)),
        "BA_FIRE" => (Ball, "fire", (1, 7 * level / 2, 10)),
        "BA_COLD" => (Ball, "cold", (1, 3 * level / 2, 10)),
        "BA_POISON" => (Ball, "poison", (12, 2, 0)),
        "BA_NUKE" => (Ball, "nuke", (10, 6, level)),
        "BA_WATER" => (Ball, "water", (1, level, 50)),
        // Rockets are shard bursts in the legacy resistance table.
        "ROCKET" => (Ball, "shards", (1, 1, (6 * level).saturating_sub(1))),
        "PULVERISE" => (Ball, "physical", (8, 8, 0)),
        "BA_NETHER" => (Ball, "nether", (10, 10, 50 + level)),
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
/// types stay unmapped.
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
    if base == "S_KIN" {
        let caster_tail = caster_kind_id.rsplit('.').next()?;
        let suffix = format!("kin-{caster_tail}");
        let id = format!("rfb-legacy.ability.{suffix}");
        abilities
            .entry(id.clone())
            .or_insert_with(|| summon_kin_ability(&suffix, caster_kind_id));
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
    let maximum_level = u32::from(level.max(1));
    let mut suffix = format!("summon-{category}-l{maximum_level}-{dice}d{sides}");
    if bonus > 0 {
        suffix.push_str(&format!("-{bonus}"));
    }
    let id = format!("rfb-legacy.ability.{suffix}");
    abilities.entry(id.clone()).or_insert_with(|| {
        summon_category_ability(&suffix, category, maximum_level, dice, sides, bonus)
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
    let mut report = ContentImportReport {
        schema_version: CONTENT_IMPORT_SCHEMA_VERSION,
        source_commit: LEGACY_BASELINE_COMMIT.to_owned(),
        ..ContentImportReport::default()
    };
    let mut terrain_files = Vec::new();
    let mut seen_ids = BTreeMap::new();
    let mut terrain_creation = TerrainCreationImportIds::default();

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
            "TREE" => terrain_creation.tree_terrain_id = Some(terrain_id),
            "GRANITE" => terrain_creation.wall_terrain_id = Some(terrain_id),
            _ => {}
        }
        terrain_files.push((format!("{id}.json"), terrain_json(entry, &id)));
        report.terrain_imported += 1;
    }
    terrain_creation.source_terrain_ids.sort();

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
            .filter(|blow| blow.damage_dice.is_some())
            .collect();
        let Some(blow) = expressible.first().copied() else {
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
        };
        if expressible.len() < entry.blows.len() {
            report.monsters_with_inexpressible_blows += 1;
            for blow in &entry.blows {
                if blow.damage_dice.is_none() {
                    *report
                        .unmapped_blow_methods
                        .entry(blow.method.clone())
                        .or_default() += 1;
                }
            }
        }
        // Legacy routines cap at four blows; the schema allows eight, so no
        // real entry ever truncates.
        let melee_routine = (expressible.len() > 1).then(|| {
            report.monsters_with_melee_routine += 1;
            let blows: Vec<serde_json::Value> = expressible
                .iter()
                .take(8)
                .map(|blow| {
                    let (blow_type, unmapped) = damage_type_for(blow);
                    if let Some(effect) = unmapped {
                        *report
                            .unmapped_blow_effects
                            .entry(effect.to_owned())
                            .or_default() += 1;
                    }
                    let (dice, sides) = blow.damage_dice.expect("expressible blow carries dice");
                    let mut method = kebab(&blow.method);
                    if method.is_empty() {
                        method = "strike".to_owned();
                    }
                    serde_json::json!({
                        "methodId": format!("rfb-legacy.blow.{method}"),
                        "toHit": 20,
                        "damageDice": dice.clamp(1, 100),
                        "damageSides": sides.clamp(1, 10_000),
                        "damageType": blow_type,
                    })
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
            if monster_flag_is_mapped(flag) {
                continue;
            }
            *report
                .unmapped_monster_flags
                .entry(flag.clone())
                .or_default() += 1;
        }
        let (damage_type, unmapped_effect) = damage_type_for(blow);
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
    report.items_total = items.len();
    // Prepass: the first shot/arrow/bolt entry becomes the canonical ammo
    // partner for its launcher class.
    let mut ammo_index = LauncherAmmoIndex::default();
    for entry in items {
        if entry.name.is_empty() || entry.name == "something" || entry.glyph.is_none() {
            continue;
        }
        let slot = match entry.tval {
            16 => &mut ammo_index.shot,
            17 => &mut ammo_index.arrow,
            18 => &mut ammo_index.bolt,
            _ => continue,
        };
        if slot.is_none() {
            *slot = Some(format!("rfb-legacy.item.{}", kebab(&entry.name)));
        }
    }
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
            artifact_json(entry, &id, &ammo_index, &mut report),
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
                let (ability_id, ability) = death_spell_ability(spell)?;
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
                if spell.index == 1 {
                    *report
                        .player_spell_behavior_gaps
                        .entry("malediction-random-rider".to_owned())
                        .or_default() += 1;
                } else if spell.index == 5 {
                    *report
                        .player_spell_behavior_gaps
                        .entry("random-resistance-duration".to_owned())
                        .or_default() += 1;
                } else if spell.index == 17 {
                    for gap in [
                        "invoke-spirits-actor-polymorph",
                        "invoke-spirits-line-light",
                        "invoke-spirits-earthquake",
                        "invoke-spirits-destroy-area",
                    ] {
                        *report
                            .player_spell_behavior_gaps
                            .entry(gap.to_owned())
                            .or_default() += 1;
                    }
                }
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
            "death-stench-of-death.json".to_owned(),
            death_first_book_json(&ability_ids),
        ));
    }
    if !death_second_ability_ids.is_empty() {
        let ability_ids = death_second_ability_ids.into_values().collect::<Vec<_>>();
        report.player_abilities_imported += ability_ids.len();
        report.player_ability_books_imported += 1;
        ability_book_files.push((
            "death-sepulchral-ways.json".to_owned(),
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
            magic_profile_json(profile, class_id),
        ));
    }
    report.proficiency_profiles_total = characters.proficiency_profiles.len();
    for profile in &characters.proficiency_profiles {
        report.proficiency_weapon_rows += profile.weapon_entries;
        *report
            .class_proficiency_gaps
            .entry("weapon-proficiency".to_owned())
            .or_default() += profile.weapon_entries;
        for (skill_index, count) in &profile.skill_entries {
            let gap = match skill_index {
                0 => "martial-arts-proficiency".to_owned(),
                1 => "dual-wielding-proficiency".to_owned(),
                2 => "riding-proficiency".to_owned(),
                other => format!("skill-{other}-proficiency"),
            };
            report.proficiency_skill_rows += *count;
            *report.class_proficiency_gaps.entry(gap).or_default() += *count;
        }
    }
    let realm_readability = (!characters.magic_profiles.is_empty())
        .then(|| realm_readability_json(&characters.magic_profiles, &class_ids));
    let class_casting_shells = (!characters.classes.is_empty())
        .then(|| class_casting_shells_json(&characters.classes, &characters.magic_profiles));
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

pub fn import_content(source: &Path, output: &Path) -> Result<PathBuf, LegacyImportError> {
    let canonical_source = source
        .canonicalize()
        .map_err(|error| LegacyImportError::LegacyGit(error.to_string()))?;
    if output.starts_with(&canonical_source) {
        return Err(LegacyImportError::LegacyGit(
            "output directory must live outside the legacy source".to_owned(),
        ));
    }
    let f_info = read_legacy_object(source, "lib/edit/f_info.txt")?;
    let r_info = read_legacy_object(source, "lib/edit/r_info.txt")?;
    let k_info = read_legacy_object(source, "lib/edit/k_info.txt")?;
    let e_info = read_legacy_object(source, "lib/edit/e_info.txt")?;
    let a_info = read_legacy_object(source, "lib/edit/a_info.txt")?;
    let b_info = read_legacy_object(source, "lib/edit/b_info.txt")?;
    let m_info = read_legacy_object(source, "lib/edit/m_info.txt")?;
    let s_info = read_legacy_object(source, "lib/edit/s_info.txt")?;
    let defines = read_legacy_object(source, "src/defines.h")?;
    let classes_source = read_legacy_object(source, "src/classes.c")?;
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
    let source_objects = list_legacy_c_sources(source)?
        .into_iter()
        .map(|path| {
            let text = read_legacy_object(source, &path)?;
            Ok((path, text))
        })
        .collect::<Result<Vec<_>, LegacyImportError>>()?;
    let mut seen_character_ids = BTreeSet::new();
    for (path, text) in &source_objects {
        for (name, body) in extract_race_blocks(text) {
            let mut entry = parse_character_block(&name, &body);
            if let Some(hook) = entry.calc_bonuses_fn.clone() {
                let (resistances, free_act, speed) = parse_calc_bonuses_defenses(text, &hook);
                entry.resistances = resistances;
                entry.free_act = free_act;
                entry.speed = speed;
            }
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
                source_found: false,
            }
        }));
    }
    let outcome = convert_content(&terrain, &monsters, &items, &egos, &artifacts, &characters);

    let terrain_dir = output.join("terrain");
    let actor_dir = output.join("actors");
    fs::create_dir_all(&terrain_dir)?;
    fs::create_dir_all(&actor_dir)?;
    for (directory, files) in [
        ("abilities", &outcome.ability_files),
        ("abilityBooks", &outcome.ability_book_files),
        ("resources", &outcome.resource_files),
        ("items", &outcome.item_files),
        ("affixes", &outcome.affix_files),
        ("races", &outcome.race_files),
        ("classes", &outcome.class_files),
        ("personalities", &outcome.personality_files),
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
    content_roots.push("actors");
    if !outcome.affix_files.is_empty() {
        content_roots.push("affixes");
    }
    if !outcome.class_files.is_empty() {
        content_roots.push("classes");
    }
    if !outcome.item_files.is_empty() {
        content_roots.push("items");
    }
    if !outcome.personality_files.is_empty() {
        content_roots.push("personalities");
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(outcome.report.monsters_imported, 1);
        assert_eq!(outcome.report.monsters_skipped, 1);
        assert_eq!(outcome.report.monsters_with_unmapped_spells, 0);
        assert_eq!(outcome.report.monsters_with_melee_routine, 1);
        assert_eq!(outcome.report.monsters_with_inexpressible_blows, 0);
        assert_eq!(outcome.report.unmapped_spells.len(), 0);
        assert_eq!(outcome.report.spells_mapped["SCARE"], 1);
        assert_eq!(outcome.report.spells_mapped["BR_FIRE"], 1);
        assert_eq!(outcome.report.monsters_with_casting, 1);
        assert_eq!(outcome.ability_files.len(), 2);
        assert_eq!(outcome.resource_files.len(), 1);
        assert_eq!(
            outcome.report.skip_reasons["monster-without-expressible-melee"],
            1
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
        assert_eq!(blows[0]["damageType"], "fire");
        assert_eq!(blows[1]["methodId"], "rfb-legacy.blow.crush");
        assert_eq!(blows[1]["damageType"], "physical");
        assert_eq!(blows[1]["damageDice"], 1);
        assert_eq!(blows[1]["damageSides"], 6);
        // RES_FIRE folds into the content resistance map.
        assert_eq!(lantern["resistances"]["fire"], "resistant");
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
        assert_eq!(proficiency_profiles[0].weapon_entries, 2);
        assert_eq!(proficiency_profiles[0].skill_entries.len(), 3);

        let characters = LegacyCharacterSources {
            classes: vec![mage],
            magic_profiles,
            proficiency_profiles,
            ..LegacyCharacterSources::default()
        };
        let outcome = convert_content(&[], &[], &[], &[], &[], &characters);
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
        assert_eq!(
            outcome.report.player_spell_behavior_gaps["random-resistance-duration"],
            1
        );
        for gap in [
            "invoke-spirits-actor-polymorph",
            "invoke-spirits-line-light",
            "invoke-spirits-earthquake",
            "invoke-spirits-destroy-area",
        ] {
            assert_eq!(outcome.report.player_spell_behavior_gaps[gap], 1);
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
            invoke_spirits["effect"]["branches"]
                .as_array()
                .map(Vec::len),
            Some(23)
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
    }

    #[test]
    fn summon_tokens_map_to_category_and_kin_abilities() {
        const SUMMONER_R_INFO: &str = "\
N:5:test bone caller\n\
G:L:w\n\
I:110:8d8:20:20:10:10\n\
W:20:2:20:9:10:40\n\
B:HIT:HURT(1d6)\n\
F:UNDEAD | DRAGON | RES_ALL | RES_TELE | NO_CONF\n\
S:1_IN_3 | S_KIN | S_UNDEAD | S_MONSTER(1d1) | S_CYBER\n";
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
            serde_json::json!(["rfb.status.confusion"])
        );
        assert!(
            !outcome
                .report
                .unmapped_monster_flags
                .contains_key("NO_CONF")
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
            ]
        );
        // Uniques and cyber summons stay honest gaps.
        assert_eq!(outcome.report.unmapped_spells["S_CYBER"], 1);

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
        assert_eq!(sword["meleeProfile"]["damageDice"], 2);
        assert_eq!(sword["meleeProfile"]["damageSides"], 6);
        assert_eq!(sword["meleeProfile"]["toHit"], 1);
        assert_eq!(sword["meleeProfile"]["toDamage"], 2);

        let bow = get("test-short-bow.json");
        assert_eq!(bow["equipmentSlot"], "launcher");
        assert!(bow.get("projectileProfile").is_some());

        let arrow = get("test-arrow.json");
        assert!(arrow.get("equipmentSlot").is_none());
        assert_eq!(arrow["maxStack"], 99);
        assert_eq!(arrow["breakChancePercent"], 25);

        // Inherent defensive flags fold onto the base item (dragon scale
        // style); durability flags are not applicable to RFB items.
        let mail = get("test-chain-mail.json");
        assert_eq!(mail["equipmentSlot"], "body");
        assert_eq!(mail["modifiers"]["defense"], 14);
        assert_eq!(mail["resistances"]["acid"], "resistant");
        assert_eq!(mail["statusImmunities"][0], "rfb.status.paralysis");
        assert!(!outcome.report.unmapped_item_flags.contains_key("RES_ACID"));
        assert!(!outcome.report.unmapped_item_flags.contains_key("FREE_ACT"));
        assert_eq!(outcome.report.not_applicable_item_flags["IGNORE_FIRE"], 1);

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
    p_ptr->pspeed += 3;
    p_ptr->pspeed += p_ptr->lev / 10;
    if (p_ptr->lev >= 45) res_add(RES_COLD);
    if (p_ptr->lev >= 10)
        res_add(RES_POIS);
    if (p_ptr->lev >= 30)
    {
        res_add(RES_DARK);
        p_ptr->pspeed += 2;
    }
    p_ptr->hold_life++;
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
        let (resistances, free_act, speed) =
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
        assert_eq!(speed, 3);
        let mut folk = folk;
        folk.resistances = resistances;
        folk.free_act = free_act;
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
        assert_eq!(outcome.report.race_hook_gaps["infra"], 1);

        let (race_name, race) = &outcome.race_files[0];
        assert_eq!(race_name, "test-folk.json");
        assert_eq!(race["resistances"]["fire"], "strong");
        assert_eq!(race["resistances"]["acid"], "immune");
        assert_eq!(race["resistances"]["light"], "vulnerable");
        assert_eq!(race["statusImmunities"][0], "rfb.status.paralysis");
        assert_eq!(race["modifiers"]["speed"], 3);
        assert_eq!(race["kinCategory"], "kin-glyph-112");
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
W:0:*:2
C:8:6:0:0
F:SHOW_MODS

N:2:of the Test Bear
T:AMULET | RING
W:10:*:4
C:0:0:0:3
F:STR | DEC_INT | HIDE_TYPE | SPEED
E:BERSERK:50:100

N:3:(Test Aura)
T:WEAPON
W:50:*:6
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
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.egos_total, 6);
        // The aura ego has no expressible modifier surface: skipped with a
        // reason while its flag still lands in the gap report.
        assert_eq!(outcome.report.egos_imported, 5);
        assert_eq!(outcome.affix_files.len(), 5);
        assert_eq!(outcome.report.skip_reasons["ego-inexpressible"], 1);
        assert_eq!(outcome.report.unmapped_ego_flags["SPELL_POWER"], 1);

        let (name, testing) = &outcome.affix_files[0];
        assert_eq!(name, "testing.json");
        assert_eq!(testing["id"], "rfb-legacy.affix.testing");
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
        assert_eq!(outcome.report.not_applicable_item_flags["HIDE_TYPE"], 1);
        assert!(!outcome.report.unmapped_ego_flags.contains_key("STR"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("DEC_INT"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("SPEED"));

        // A purely defensive ego used to be inexpressible; the flag fold now
        // carries its elemental defenses and status immunities.
        let (name, warding) = &outcome.affix_files[2];
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

        let (_, dragonfire) = &outcome.affix_files[3];
        assert_eq!(dragonfire["slays"]["dragon"], "slay");
        assert_eq!(dragonfire["brands"][0], "fire");
        assert!(
            !outcome
                .report
                .unmapped_ego_flags
                .contains_key("SLAY_DRAGON")
        );
        assert!(!outcome.report.unmapped_ego_flags.contains_key("BRAND_FIRE"));

        let (_, death) = &outcome.affix_files[4];
        assert_eq!(death["id"], LEGACY_DEATH_WEAPON_AFFIX_ID);
        assert!(
            death["passives"]
                .as_array()
                .expect("death passives")
                .iter()
                .any(|value| value == "vampiric")
        );
        assert!(!outcome.report.unmapped_ego_flags.contains_key("BRAND_VAMP"));
        assert_eq!(outcome.report.unmapped_ego_flags["HOLD_LIFE"], 1);
    }

    #[test]
    fn death_third_physical_book_maps_to_black_channels() {
        let item = LegacyItemEntry {
            tval: DEATH_BOOK_TVAL,
            sval: DEATH_THIRD_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&item),
            Some(DEATH_THIRD_BOOK_ID)
        );
    }

    #[test]
    fn death_fourth_physical_book_maps_to_necronomicon() {
        let item = LegacyItemEntry {
            tval: DEATH_BOOK_TVAL,
            sval: DEATH_FOURTH_BOOK_SVAL,
            ..LegacyItemEntry::default()
        };
        assert_eq!(
            player_ability_book_for_item(&item),
            Some(DEATH_FOURTH_BOOK_ID)
        );
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
            (39, serde_json::json!({"type": "heal", "amount": 5000})),
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
        assert!(!report.item_behavior_gaps.contains_key("consumable-effect"));

        let _ = item_json(
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
        assert_eq!(report.item_behavior_gaps["consumable-effect"], 1);
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
        );
        assert!(
            trap["tags"]
                .as_array()
                .is_some_and(|tags| { tags.iter().any(|tag| tag == "trap") })
        );
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
        for (sval, subject, category, persistent) in [
            (25, "terrain", "map", true),
            (26, "item", "gold", false),
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
                sval: 0,
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
        let outcome = convert_content(
            &[],
            &[],
            &[],
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
        assert_eq!(fang["brands"][0], "fire");
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
        assert_eq!(outcome.report.unmapped_artifact_flags["BRAND_CHAOS"], 1);

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
B:HIT:HURT(1d4)
S:1_IN_3 | TELE_OTHER | DRAIN_MANA | AMNESIA | DISPEL_MAGIC | DARKNESS
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
            ]
        );
        // Room unlighting has no neutral state yet.
        assert_eq!(outcome.report.unmapped_spells["DARKNESS"], 1);
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
            ]
        );
        assert_eq!(outcome.report.unmapped_spells["HAND_DOOM"], 1);
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
}
