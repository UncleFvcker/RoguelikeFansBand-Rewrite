// SPDX-License-Identifier: MPL-2.0

//! Read-only import of legacy `f_info.txt`/`r_info.txt` content into local
//! rfb-content JSON fragments. Everything expressible is written to a
//! git-ignored output directory; everything else is aggregated into a gap
//! report so rule work can be prioritised from data. No legacy text enters
//! the repository: unit tests use synthetic samples only.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::Serialize;

use crate::{LEGACY_BASELINE_COMMIT, LegacyImportError};

pub const CONTENT_IMPORT_SCHEMA_VERSION: u16 = 1;
const SCHEMA_BASE: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1";

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
    pub bodies_total: usize,
    pub races_total: usize,
    pub races_imported: usize,
    pub personalities_total: usize,
    pub personalities_imported: usize,
    pub unmapped_race_flags: BTreeMap<String, usize>,
    pub race_hook_gaps: BTreeMap<String, usize>,
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
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn parse_f_info(text: &str) -> Vec<LegacyTerrainEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyTerrainEntry> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            let tag = parts.next().unwrap_or_default().trim().to_owned();
            current = Some(LegacyTerrainEntry {
                index,
                tag,
                ..LegacyTerrainEntry::default()
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("E:") {
            entry.display_name = Some(rest.trim().to_owned());
        } else if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = rest.chars().next();
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
    entries
}

fn parse_blow(rest: &str) -> LegacyBlow {
    let mut blow = LegacyBlow::default();
    for (ordinal, part) in rest.split(':').map(str::trim).enumerate() {
        if ordinal == 0 {
            blow.method = part.to_owned();
            continue;
        }
        if part.is_empty() {
            continue;
        }
        // Effects mostly carry their dice inline: HURT(2d6), POISON(1d4),
        // DAM(3d8)... the first diced effect provides the melee damage.
        if let Some((token, dice)) = part
            .split_once('(')
            .and_then(|(token, rest)| rest.strip_suffix(')').map(|dice| (token, dice)))
        {
            if blow.damage_dice.is_none()
                && let Some((dice_count, sides)) = dice.split_once('d')
                && let (Ok(dice_count), Ok(sides)) = (dice_count.parse(), sides.parse())
            {
                blow.damage_dice = Some((dice_count, sides));
                blow.effects.insert(0, token.to_owned());
                continue;
            }
            blow.effects.push(token.to_owned());
        } else {
            blow.effects.push(part.to_owned());
        }
    }
    blow
}

pub fn parse_r_info(text: &str) -> Vec<LegacyMonsterEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyMonsterEntry> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            let name = parts.next().unwrap_or_default().trim().to_owned();
            current = Some(LegacyMonsterEntry {
                index,
                name,
                ..LegacyMonsterEntry::default()
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = rest.chars().next();
        } else if let Some(rest) = line.strip_prefix("I:") {
            // I:speed:HDdHS:aaf:ac:sleep:weight (init1.c sscanf order).
            let parts: Vec<&str> = rest.split(':').collect();
            entry.speed = parts.first().and_then(|raw| raw.parse().ok());
            if let Some(dice) = parts.get(1) {
                let mut split = dice.splitn(2, 'd');
                let dice_count = split.next().and_then(|raw| raw.parse().ok());
                let sides = split.next().and_then(|raw| raw.parse().ok());
                if let (Some(dice_count), Some(sides)) = (dice_count, sides) {
                    entry.hp_dice = Some((dice_count, sides));
                }
            }
            entry.armor_class = parts.get(3).and_then(|raw| raw.parse().ok());
        } else if let Some(rest) = line.strip_prefix("W:") {
            let parts: Vec<&str> = rest.split(':').collect();
            entry.level = parts.first().and_then(|raw| raw.parse().ok());
            entry.rarity = parts.get(1).and_then(|raw| raw.parse().ok());
        } else if let Some(rest) = line.strip_prefix("B:") {
            entry.blows.push(parse_blow(rest));
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
    entries
}

const MAPPED_TERRAIN_FLAGS: [&str; 4] = ["MOVE", "LOS", "PROJECT", "PERMANENT"];

/// Parses k_info entries; `&` article and `~` plural markers strip out of
/// names, and the `N:*:` auto-index form continues the running counter.
pub fn parse_k_info(text: &str) -> Vec<LegacyItemEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyItemEntry> = None;
    let mut next_index = 0_u32;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let raw_index = parts.next().unwrap_or_default();
            let index = if raw_index == "*" {
                next_index
            } else {
                raw_index.parse().unwrap_or(next_index)
            };
            next_index = index + 1;
            let name = parts
                .next()
                .unwrap_or_default()
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
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("G:") {
            entry.glyph = rest.chars().next();
        } else if let Some(rest) = line.strip_prefix("I:") {
            let mut parts = rest.split(':');
            entry.tval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.sval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.pval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("W:") {
            // level:extra:max_level:weight:cost per the pinned init1.c.
            let values: Vec<i64> = rest
                .split(':')
                .map(|raw| raw.parse().unwrap_or(0))
                .collect();
            entry.level = u16::try_from(values.first().copied().unwrap_or(0).max(0)).unwrap_or(0);
            entry.weight_tenths_pound =
                u16::try_from(values.get(3).copied().unwrap_or(0).clamp(0, 10_000)).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("P:") {
            let mut parts = rest.split(':');
            entry.armor_class = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            if let Some(dice) = parts.next()
                && let Some((count, sides)) = dice.split_once('d')
                && let (Ok(count), Ok(sides)) = (count.parse(), sides.parse())
            {
                entry.damage_dice = Some((count, sides));
            }
            entry.to_hit = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.to_damage = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.to_armor = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
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
    entries
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
            behavior_gap: Some("device-effect"),
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

fn item_json(
    entry: &LegacyItemEntry,
    id: &str,
    ammo: &LauncherAmmoIndex,
    report: &mut ContentImportReport,
) -> serde_json::Value {
    let shape = item_shape(entry.tval).expect("every tval resolves a shape");
    if let Some(gap) = shape.behavior_gap {
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
        "tags": shape.tags,
    });
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
    let mut modifiers = serde_json::Map::new();
    let defense = entry.armor_class.saturating_add(entry.to_armor);
    if shape.slot.is_some() && defense != 0 {
        modifiers.insert("defense".to_owned(), serde_json::json!(defense));
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    for flag in &entry.flags {
        *report.unmapped_item_flags.entry(flag.clone()).or_default() += 1;
    }
    value
}

impl ItemShape {
    fn tags_contain(&self, tag: &str) -> bool {
        self.tags.contains(&tag)
    }
}

/// Parses e_info ego templates: `C:` carries generation-time maximum
/// bonuses, `T:` accumulates the applicable slot classes.
pub fn parse_e_info(text: &str) -> Vec<LegacyEgoEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyEgoEntry> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            let name = parts.next().unwrap_or_default().trim().to_owned();
            current = Some(LegacyEgoEntry {
                index,
                name,
                ..LegacyEgoEntry::default()
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("T:") {
            entry.slots.extend(
                rest.split('|')
                    .map(str::trim)
                    .filter(|slot| !slot.is_empty())
                    .map(str::to_owned),
            );
        } else if let Some(rest) = line.strip_prefix("W:") {
            entry.level = rest
                .split(':')
                .next()
                .and_then(|raw| raw.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("C:") {
            let values: Vec<i32> = rest
                .split(':')
                .map(|raw| raw.parse().unwrap_or(0))
                .collect();
            entry.max_to_hit = values.first().copied().unwrap_or(0);
            entry.max_to_damage = values.get(1).copied().unwrap_or(0);
            entry.max_to_armor = values.get(2).copied().unwrap_or(0);
            entry.max_pval = values.get(3).copied().unwrap_or(0);
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
    entries
}

/// Parses a_info fixed artifacts; unlike egos their pval and combat bonuses
/// are fixed values. `E:` activation lines (ASCII token form) mark the
/// activation gap; localized text lines are skipped outright.
pub fn parse_a_info(text: &str) -> Vec<LegacyArtifactEntry> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyArtifactEntry> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            let name = parts.next().unwrap_or_default().trim().to_owned();
            current = Some(LegacyArtifactEntry {
                index,
                name,
                ..LegacyArtifactEntry::default()
            });
            continue;
        }
        let Some(entry) = current.as_mut() else {
            continue;
        };
        if let Some(rest) = line.strip_prefix("I:") {
            let mut parts = rest.split(':');
            entry.tval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.sval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.pval = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("W:") {
            // level:rarity:weight:cost for artifacts.
            let values: Vec<i64> = rest
                .split(':')
                .map(|raw| raw.parse().unwrap_or(0))
                .collect();
            entry.level = u16::try_from(values.first().copied().unwrap_or(0).max(0)).unwrap_or(0);
            entry.weight_tenths_pound =
                u16::try_from(values.get(2).copied().unwrap_or(0).clamp(0, 10_000)).unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("P:") {
            let mut parts = rest.split(':');
            entry.armor_class = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            if let Some(dice) = parts.next()
                && let Some((count, sides)) = dice.split_once('d')
                && let (Ok(count), Ok(sides)) = (count.parse(), sides.parse())
            {
                entry.damage_dice = Some((count, sides));
            }
            entry.to_hit = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.to_damage = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            entry.to_armor = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
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
    entries
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
        *report.unmapped_ego_flags.entry(flag.clone()).or_default() += 1;
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
    value
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
    // pval feeds the attribute flags.
    let has_slot = shape.slot.is_some();
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
    }
    if !modifiers.is_empty() {
        value["modifiers"] = serde_json::Value::Object(modifiers);
    }
    if entry.has_activation {
        *report
            .item_behavior_gaps
            .entry("artifact-activation".to_owned())
            .or_default() += 1;
    }
    for flag in &entry.flags {
        // Slotless shapes never applied the attribute fold, so their
        // attribute flags stay visible in the gap report.
        if (has_slot && attribute_flag_is_mapped(flag)) || flag == "INSTA_ART" {
            continue;
        }
        *report
            .unmapped_artifact_flags
            .entry(flag.clone())
            .or_default() += 1;
    }
    value
}

/// Parses b_info body templates: `N:index:Name` then `S:TYPE:Label`
/// slot lines in template order.
pub fn parse_b_info(text: &str) -> Vec<LegacyBodyTemplate> {
    let mut entries = Vec::new();
    let mut current: Option<LegacyBodyTemplate> = None;
    for line in text.lines() {
        let line = line.trim_end();
        if let Some(rest) = line.strip_prefix("N:") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let mut parts = rest.splitn(2, ':');
            let index = parts.next().and_then(|raw| raw.parse().ok()).unwrap_or(0);
            let name = parts.next().unwrap_or_default().trim().to_owned();
            current = Some(LegacyBodyTemplate {
                index,
                name,
                ..LegacyBodyTemplate::default()
            });
        } else if let Some(rest) = line.strip_prefix("S:")
            && let Some(entry) = current.as_mut()
            && let Some(token) = rest.split(':').next()
        {
            let token = token.trim();
            if !token.is_empty() {
                entry.slots.push(token.to_owned());
            }
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }
    entries
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
                other => entry.hooks.push(other.to_owned()),
            }
        }
    }
    entry
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
    serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/terrain.schema.json"),
        "formatVersion": 1,
        "id": format!("rfb-legacy.terrain.{id}"),
        "nameKey": format!("terrain-legacy-{id}-name"),
        "descriptionKey": format!("terrain-legacy-{id}-description"),
        "glyph": entry.glyph.map_or_else(|| "?".to_owned(), |glyph| glyph.to_string()),
        "walkable": walkable,
        "blocksSight": blocks_sight,
        "tags": ["legacy-import"],
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

fn resistance_flag_is_mapped(flag: &str) -> bool {
    if flag == "RES_ALL" {
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
    for (flag, tag) in [
        ("ANIMAL", "animal"),
        ("DEMON", "demon"),
        ("DRAGON", "dragon"),
        ("UNDEAD", "undead"),
    ] {
        if entry.flags.iter().any(|value| value == flag) {
            tags.push(tag.to_owned());
        }
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
    if let Some(routine) = melee_routine {
        value["meleeRoutine"] = routine;
    }
    if let Some(casting) = monster_casting {
        value["monsterCasting"] = casting;
    }
    value
}

pub struct ContentImportOutcome {
    pub report: ContentImportReport,
    pub terrain_files: Vec<(String, serde_json::Value)>,
    pub actor_files: Vec<(String, serde_json::Value)>,
    pub ability_files: Vec<(String, serde_json::Value)>,
    pub resource_files: Vec<(String, serde_json::Value)>,
    pub item_files: Vec<(String, serde_json::Value)>,
    pub affix_files: Vec<(String, serde_json::Value)>,
    pub race_files: Vec<(String, serde_json::Value)>,
    pub personality_files: Vec<(String, serde_json::Value)>,
    pub skill_files: Vec<(String, serde_json::Value)>,
    pub skill_set_files: Vec<(String, serde_json::Value)>,
}

const LEGACY_RESOURCE_ID: &str = "rfb-legacy.resource.essence";

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
        terrain_files.push((format!("{id}.json"), terrain_json(entry, &id)));
        report.terrain_imported += 1;
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
            if resistance_flag_is_mapped(flag) {
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
            item_json(entry, &id, &ammo_index, &mut report),
        ));
        report.items_imported += 1;
    }

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
        // Egos whose entire power set lives in unmappable flags produce an
        // empty modifier map, which the affix contract rejects; skip them but
        // keep their flags visible in the gap report above.
        if value.get("modifiers").is_none() {
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
    let mut personality_files = Vec::new();
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
    let skill_files = if race_files.is_empty() && personality_files.is_empty() {
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
    let resource_files = if ability_files.is_empty() {
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

    ContentImportOutcome {
        report,
        terrain_files,
        actor_files,
        ability_files,
        resource_files,
        item_files,
        affix_files,
        race_files,
        personality_files,
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
    let mut characters = LegacyCharacterSources {
        bodies: parse_b_info(&b_info),
        ..LegacyCharacterSources::default()
    };
    let mut source_paths: Vec<PathBuf> = fs::read_dir(source.join("src"))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "c"))
        .collect();
    source_paths.sort();
    let mut seen_character_ids = BTreeSet::new();
    for path in source_paths {
        let bytes = fs::read(&path)?;
        let text = String::from_utf8_lossy(&bytes);
        for (name, body) in extract_race_blocks(&text) {
            let entry = parse_character_block(&name, &body);
            if seen_character_ids.insert(format!("race:{}", entry.id)) {
                characters.races.push(entry);
            }
        }
        if path.file_name().is_some_and(|name| name == "personality.c") {
            for (name, body) in extract_personality_blocks(&text) {
                let entry = parse_character_block(&name, &body);
                if seen_character_ids.insert(format!("personality:{}", entry.id)) {
                    characters.personalities.push(entry);
                }
            }
        }
    }
    let outcome = convert_content(
        &parse_f_info(&f_info),
        &parse_r_info(&r_info),
        &parse_k_info(&k_info),
        &parse_e_info(&e_info),
        &parse_a_info(&a_info),
        &characters,
    );

    let terrain_dir = output.join("terrain");
    let actor_dir = output.join("actors");
    fs::create_dir_all(&terrain_dir)?;
    fs::create_dir_all(&actor_dir)?;
    for (directory, files) in [
        ("abilities", &outcome.ability_files),
        ("resources", &outcome.resource_files),
        ("items", &outcome.item_files),
        ("affixes", &outcome.affix_files),
        ("races", &outcome.race_files),
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
                serde_json::to_string_pretty(value)?
                    + "
",
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
    content_roots.push("actors");
    if !outcome.affix_files.is_empty() {
        content_roots.push("affixes");
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
        serde_json::to_string_pretty(&pack_manifest)?
            + "
",
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
        let terrain = parse_f_info(SYNTHETIC_F_INFO);
        assert_eq!(terrain.len(), 3);
        assert_eq!(terrain[1].tag, "TEST_ARCH");
        assert_eq!(terrain[1].glyph, Some('\''));
        assert_eq!(terrain[1].flags.len(), 6);

        let monsters = parse_r_info(SYNTHETIC_R_INFO);
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
        let monsters = parse_r_info(CASTER_R_INFO);
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
        let monsters = parse_r_info(DRAGON_R_INFO);
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
F:UNDEAD | DRAGON\n\
S:1_IN_3 | S_KIN | S_UNDEAD | S_MONSTER(1d1) | S_CYBER\n";
        let monsters = parse_r_info(SUMMONER_R_INFO);
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
            serde_json::json!(["legacy-import", "dragon", "undead"])
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
P:0:0d0:0:0:0
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
N:*:& Test Ring~
G:=:y
I:45:20:0
W:30:0:0:2:500
F:HIDE_TYPE
N:*:& Test Potion~ of Mending
G:!:b
I:75:30:0
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
        let items = parse_k_info(SYNTHETIC_K_INFO);
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
        assert_eq!(outcome.report.items_imported, 8);
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

        let mail = get("test-chain-mail.json");
        assert_eq!(mail["equipmentSlot"], "body");
        assert_eq!(mail["modifiers"]["defense"], 14);

        // Base jewelry is a generic shell: attributes and pval only arrive
        // via egos or fixed artifacts, mirroring the legacy generation model.
        let ring = get("test-ring.json");
        assert_eq!(ring["equipmentSlot"], "ring");
        assert!(ring.get("modifiers").is_none());
        assert_eq!(outcome.report.unmapped_item_flags["HIDE_TYPE"], 1);
        assert_eq!(outcome.report.item_behavior_gaps["effect-jewelry"], 1);

        let potion = get("test-potion-of-mending.json");
        assert_eq!(potion["maxStack"], 20);
        assert_eq!(outcome.report.item_behavior_gaps["consumable-effect"], 1);

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
        let bodies = parse_b_info(SYNTHETIC_B_INFO);
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

        let beast = parse_character_block(&blocks[1].0, &blocks[1].1);
        assert!(beast.dynamic);

        let characters = LegacyCharacterSources {
            bodies: Vec::new(),
            races: vec![folk, beast],
            personalities: Vec::new(),
        };
        let outcome = convert_content(&[], &[], &[], &[], &[], &characters);
        assert_eq!(outcome.report.races_total, 2);
        assert_eq!(outcome.report.races_imported, 1);
        assert_eq!(outcome.report.skip_reasons["race-code-dynamic"], 1);
        assert_eq!(outcome.report.unmapped_race_flags["RACE_IS_NONLIVING"], 1);
        assert_eq!(outcome.report.race_hook_gaps["calc_bonuses"], 1);
        assert_eq!(outcome.report.race_hook_gaps["infra"], 1);
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
F:STR | DEC_INT | HIDE_TYPE
E:BERSERK:50:100

N:3:(Test Aura)
T:WEAPON
W:50:*:6
F:SPELL_POWER
";
        let egos = parse_e_info(SYNTHETIC_E_INFO);
        assert_eq!(egos.len(), 3);
        let outcome = convert_content(
            &[],
            &[],
            &[],
            &egos,
            &[],
            &LegacyCharacterSources::default(),
        );
        assert_eq!(outcome.report.egos_total, 3);
        // The aura ego has no expressible modifier surface: skipped with a
        // reason while its flag still lands in the gap report.
        assert_eq!(outcome.report.egos_imported, 2);
        assert_eq!(outcome.affix_files.len(), 2);
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
        assert!(bear["modifiers"].get("attack").is_none());
        assert!(
            bear["tags"]
                .as_array()
                .expect("bear tags")
                .iter()
                .any(|tag| tag == "amulet")
        );
        assert_eq!(outcome.report.item_behavior_gaps["ego-activation"], 1);
        assert_eq!(outcome.report.unmapped_ego_flags["HIDE_TYPE"], 1);
        assert!(!outcome.report.unmapped_ego_flags.contains_key("STR"));
        assert!(!outcome.report.unmapped_ego_flags.contains_key("DEC_INT"));
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
F:DEX | SLAY_EVIL

N:3:of Test Warding
I:45:20:4
W:40:8:2:80000
F:CON

N:4:of Test Melody
I:19:70:2
W:30:5:60:50000
P:0:0d0:5:5:0
F:CHR
";
        let artifacts = parse_a_info(SYNTHETIC_A_INFO);
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
        assert!(!outcome.report.unmapped_artifact_flags.contains_key("WIS"));
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
        assert_eq!(outcome.report.unmapped_artifact_flags["SLAY_EVIL"], 1);

        // Fixed-pval jewelry proves the split: the base ring stays bare while
        // the artifact carries the attribute bonus.
        let warding = get("artifact-test-warding.json");
        assert_eq!(warding["equipmentSlot"], "ring");
        assert_eq!(warding["modifiers"]["constitution"], 4);
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
        let monsters = parse_r_info(WARDEN_R_INFO);
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
        let monsters = parse_r_info(CURSER_R_INFO);
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
        let monsters = parse_r_info(PSION_R_INFO);
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
