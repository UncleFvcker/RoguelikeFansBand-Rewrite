// SPDX-License-Identifier: MPL-2.0

//! Read-only import of legacy `f_info.txt`/`r_info.txt` content into local
//! rfb-content JSON fragments. Everything expressible is written to a
//! git-ignored output directory; everything else is aggregated into a gap
//! report so rule work can be prioritised from data. No legacy text enters
//! the repository: unit tests use synthetic samples only.

use std::{
    collections::BTreeMap,
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
    pub unmapped_blow_methods: BTreeMap<String, usize>,
    pub unmapped_blow_effects: BTreeMap<String, usize>,
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
        Some("HURT" | "DAM") | None => ("physical", None),
        Some(other) => ("physical", Some(other)),
    }
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
        "tags": ["legacy-import"],
    });
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

/// Maps one legacy spell token to a generated ability id, registering the
/// shared ability definition on first use.
fn map_spell_token(
    token: &str,
    level: u16,
    abilities: &mut BTreeMap<String, serde_json::Value>,
) -> Option<String> {
    match token {
        "SCARE" => {
            let id = "rfb-legacy.ability.scare".to_owned();
            abilities
                .entry(id.clone())
                .or_insert_with(|| status_ability("scare", "fear", false));
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
        _ => None,
    }
}

pub fn convert_content(
    terrain: &[LegacyTerrainEntry],
    monsters: &[LegacyMonsterEntry],
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
        let mut frequency_percent: Option<u32> = None;
        let mut mapped_ability_ids: Vec<String> = Vec::new();
        let mut has_unmapped_spell = false;
        for spell in &entry.spells {
            if let Some(divisor) = spell.strip_prefix("1_IN_") {
                if let Ok(divisor) = divisor.parse::<u32>() {
                    frequency_percent = Some((100 / divisor.max(1)).clamp(1, 100));
                }
                continue;
            }
            if let Some(ability_id) = map_spell_token(
                spell,
                entry.level.unwrap_or(1).max(1),
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
            *report
                .unmapped_monster_flags
                .entry(flag.clone())
                .or_default() += 1;
        }
        let mut id = kebab(&entry.name);
        if id.is_empty() {
            id = format!("monster-{}", entry.index);
        }
        let duplicates = seen_actor_ids.entry(id.clone()).or_insert(0_u32);
        if *duplicates > 0 {
            id = format!("{id}-{}", entry.index);
        }
        *duplicates += 1;
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
    let outcome = convert_content(&parse_f_info(&f_info), &parse_r_info(&r_info));

    let terrain_dir = output.join("terrain");
    let actor_dir = output.join("actors");
    fs::create_dir_all(&terrain_dir)?;
    fs::create_dir_all(&actor_dir)?;
    for (directory, files) in [
        ("abilities", &outcome.ability_files),
        ("resources", &outcome.resource_files),
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
    let pack_manifest = serde_json::json!({
        "$schema": format!("{SCHEMA_BASE}/pack.schema.json"),
        "formatVersion": 1,
        "id": "rfb.legacy.frog-v1",
        "version": "0.1.0",
        "titleKey": "pack-rfb-legacy-title",
        "dependencies": [],
        "loadAfter": [],
        "contentRoots": if outcome.ability_files.is_empty() {
            serde_json::json!(["actors", "terrain"])
        } else {
            serde_json::json!(["abilities", "actors", "resources", "terrain"])
        },
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

        let outcome = convert_content(&terrain, &monsters);
        assert_eq!(outcome.report.terrain_imported, 2);
        assert_eq!(outcome.report.terrain_skipped, 1);
        assert_eq!(outcome.report.monsters_imported, 1);
        assert_eq!(outcome.report.monsters_skipped, 1);
        assert_eq!(outcome.report.monsters_with_unmapped_spells, 1);
        assert_eq!(outcome.report.monsters_with_melee_routine, 1);
        assert_eq!(outcome.report.monsters_with_inexpressible_blows, 0);
        assert_eq!(outcome.report.unmapped_spells.len(), 1);
        assert_eq!(outcome.report.spells_mapped["SCARE"], 1);
        assert_eq!(outcome.report.monsters_with_casting, 1);
        assert_eq!(outcome.ability_files.len(), 1);
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
}
