// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(feature = "schemas")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const CONTENT_FORMAT: &str = "rfb-content";
pub const CONTENT_FORMAT_VERSION: u16 = 1;
pub const PACK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/pack.schema.json";
pub const TERRAIN_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain.schema.json";
pub const ACTOR_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/actor.schema.json";
pub const ITEM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/item.schema.json";
pub const AFFIX_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/affix.schema.json";
pub const ENCOUNTER_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/encounter-table.schema.json";
pub const LOOT_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/loot-table.schema.json";
pub const THEME_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/theme-table.schema.json";
pub const REGION_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/region-table.schema.json";
pub const TERRAIN_FEATURE_TABLE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain-feature-table.schema.json";
pub const VAULT_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/vault.schema.json";
pub const WORLD_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/world.schema.json";

const fn default_actor_speed() -> u16 {
    110
}

const MAGIC: &[u8; 8] = b"RFBCONT\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 8 + 32;
const MAX_SOURCE_FILE_LENGTH: usize = 1024 * 1024;
const MAX_SOURCE_TOTAL_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 2048;
const MAX_COMPILED_PAYLOAD_LENGTH: usize = 32 * 1024 * 1024;
const SUPPORTED_ROOTS: [&str; 11] = [
    "actors",
    "affixes",
    "encounterTables",
    "items",
    "lootTables",
    "regionTables",
    "terrain",
    "terrainFeatureTables",
    "themeTables",
    "vaults",
    "worlds",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackManifest {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub version: String,
    pub title_key: String,
    pub dependencies: Vec<PackDependency>,
    pub load_after: Vec<String>,
    pub content_roots: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PackDependency {
    pub id: String,
    pub version_requirement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub walkable: bool,
    pub blocks_sight: bool,
    #[serde(default)]
    pub open_to_terrain_id: Option<String>,
    #[serde(default)]
    pub open_check_difficulty: Option<i32>,
    #[serde(default)]
    pub close_to_terrain_id: Option<String>,
    #[serde(default)]
    pub bash_to_terrain_id: Option<String>,
    #[serde(default)]
    pub bash_check_difficulty: Option<i32>,
    #[serde(default)]
    pub dig_to_terrain_id: Option<String>,
    #[serde(default)]
    pub dig_check_difficulty: Option<i32>,
    #[serde(default)]
    pub concealed_as_terrain_id: Option<String>,
    #[serde(default)]
    pub search_check_difficulty: Option<i32>,
    #[serde(default)]
    pub trap: Option<TerrainTrapDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainTrapDefinition {
    pub damage: i32,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    pub disarm_to_terrain_id: String,
    pub disarm_check_difficulty: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorRole {
    Player,
    Monster,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorDamageType {
    #[default]
    Physical,
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeleeBlowDefinition {
    pub method_id: String,
    pub to_hit: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeleeRoutineDefinition {
    pub blows: Vec<MeleeBlowDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub role: ActorRole,
    pub name_key: String,
    pub description_key: String,
    pub glyph: String,
    pub level: u32,
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    pub attack: i32,
    pub defense: i32,
    #[serde(default)]
    pub door_skill: i32,
    #[serde(default)]
    pub bash_power: i32,
    #[serde(default)]
    pub search_skill: i32,
    #[serde(default)]
    pub disarm_skill: i32,
    #[serde(default)]
    pub dig_skill: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub carry_capacity_tenths_pound: u32,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    #[serde(default)]
    pub melee_routine: Option<MeleeRoutineDefinition>,
    #[serde(default)]
    pub loot_table_id: Option<String>,
    #[serde(default)]
    pub carried_loot_table_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatModifiers {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AffixDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub modifiers: StatModifiers,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AttackProfileDefinition {
    pub attacks: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectileProfileDefinition {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    pub ammo_kind_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThrowProfileDefinition {
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ItemUseEffectDefinition {
    Heal { amount: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemUseActionDefinition {
    pub effect: ItemUseEffectDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    #[serde(default)]
    pub appearance_name_key: Option<String>,
    pub description_key: String,
    pub glyph: String,
    pub weight_tenths_pound: u16,
    pub max_stack: u32,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default)]
    pub melee_profile: Option<AttackProfileDefinition>,
    #[serde(default)]
    pub projectile_profile: Option<ProjectileProfileDefinition>,
    #[serde(default)]
    pub throw_profile: Option<ThrowProfileDefinition>,
    #[serde(default)]
    pub use_action: Option<ItemUseActionDefinition>,
    #[serde(default)]
    pub break_chance_percent: u8,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentPosition {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainOverride {
    pub terrain_id: String,
    pub positions: Vec<ContentPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemQuality {
    #[default]
    Ordinary,
    Fine,
    Exceptional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootEntryDefinition {
    pub item_kind_id: String,
    pub weight: u32,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootQualityWeightDefinition {
    pub quality: ItemQuality,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootAffixWeightDefinition {
    #[serde(default)]
    pub affix_id: Option<String>,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LootTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    pub entries: Vec<LootEntryDefinition>,
    pub quality_weights: Vec<LootQualityWeightDefinition>,
    pub affix_weights: Vec<LootAffixWeightDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    pub entries: Vec<EncounterEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
    #[serde(default)]
    pub group: Option<EncounterGroupDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterGroupDefinition {
    #[serde(default)]
    pub friends: Option<EncounterFriendsDefinition>,
    #[serde(default)]
    pub escort: Option<EncounterEscortDefinition>,
    pub formation: EncounterFormation,
    #[serde(default)]
    pub pack_ai: EncounterPackAiDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterPackAiDefinition {
    pub leader: MonsterPackBehavior,
    pub friends: MonsterPackBehavior,
    pub escorts: MonsterPackBehavior,
}

impl Default for EncounterPackAiDefinition {
    fn default() -> Self {
        Self {
            leader: MonsterPackBehavior::Seek,
            friends: MonsterPackBehavior::Surround,
            escorts: MonsterPackBehavior::GuardLeader,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MonsterPackBehavior {
    #[default]
    Seek,
    Surround,
    GuardLeader,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterFriendsDefinition {
    pub min_count: u16,
    pub max_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEscortDefinition {
    pub min_count: u16,
    pub max_count: u16,
    pub entries: Vec<EncounterEscortEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncounterEscortEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EncounterFormation {
    Cluster,
    Ring,
}

impl EncounterGroupDefinition {
    #[must_use]
    pub fn min_companion_count(&self) -> u16 {
        self.friends
            .as_ref()
            .map_or(0, |friends| friends.min_count)
            .saturating_add(self.escort.as_ref().map_or(0, |escort| escort.min_count))
    }

    #[must_use]
    pub fn max_companion_count(&self) -> u16 {
        self.friends
            .as_ref()
            .map_or(0, |friends| friends.max_count)
            .saturating_add(self.escort.as_ref().map_or(0, |escort| escort.max_count))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub entries: Vec<ThemeEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeEntryDefinition {
    pub theme_id: String,
    pub floor_terrain_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
    #[serde(default)]
    pub vault_candidates: Vec<ThemeVaultCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeVaultCandidateDefinition {
    pub vault_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub entries: Vec<RegionEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegionEntryDefinition {
    pub region_id: String,
    pub theme_table_id: String,
    pub theme_id: String,
    pub encounter_table_id: String,
    pub loot_table_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainFeatureTableDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub rolls: u16,
    pub entries: Vec<TerrainFeatureEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TerrainFeatureEntryDefinition {
    pub terrain_id: String,
    pub placement: TerrainFeaturePlacement,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainFeaturePlacement {
    Room,
    Corridor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub theme_id: String,
    pub width: u16,
    pub height: u16,
    pub base_terrain_id: String,
    pub entrance_position: ContentPosition,
    #[serde(default)]
    pub transforms: Vec<VaultTransform>,
    pub terrain_overrides: Vec<TerrainOverride>,
    pub encounter_groups: Vec<VaultEncounterGroupDefinition>,
    pub loot_spawns: Vec<VaultLootSpawnDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum VaultTransform {
    Identity,
    Rotate90,
    Rotate180,
    Rotate270,
    MirrorHorizontal,
    MirrorVertical,
    MirrorMainDiagonal,
    MirrorAntiDiagonal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEncounterGroupDefinition {
    pub id: String,
    pub member_positions: Vec<ContentPosition>,
    pub entries: Vec<VaultEncounterEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultEncounterEntryDefinition {
    pub actor_kind_id: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VaultLootSpawnDefinition {
    pub id: String,
    pub position: ContentPosition,
    pub loot_table_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemSpawn {
    pub instance_id: String,
    pub kind_id: String,
    pub position: ContentPosition,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQuality,
    #[serde(default)]
    pub affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorldDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub initial_floor_id: String,
    pub width: u16,
    pub height: u16,
    pub fill_terrain_id: String,
    pub border_terrain_id: String,
    pub terrain_overrides: Vec<TerrainOverride>,
    pub player: ActorSpawn,
    pub actors: Vec<ActorSpawn>,
    pub items: Vec<ItemSpawn>,
    pub procedural_floors: Vec<ProceduralFloorDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorDefinition {
    pub id: String,
    pub name_key: String,
    pub return_floor_id: String,
    #[serde(default)]
    pub lifecycle: FloorLifecycle,
    #[serde(default)]
    pub dungeon_id: Option<String>,
    #[serde(default)]
    pub final_floor: bool,
    #[serde(default)]
    pub guardian: Option<DungeonGuardianDefinition>,
    #[serde(default)]
    pub theme_id: Option<String>,
    #[serde(default)]
    pub vault_id: Option<String>,
    #[serde(default)]
    pub encounter_table_id: Option<String>,
    #[serde(default)]
    pub loot_table_id: Option<String>,
    #[serde(default)]
    pub theme_table_id: Option<String>,
    #[serde(default)]
    pub region_table_id: Option<String>,
    #[serde(default)]
    pub terrain_feature_table_id: Option<String>,
    #[serde(default)]
    pub layout: Option<ProceduralLayoutDefinition>,
    #[serde(default)]
    pub generation_budget: Option<ProceduralGenerationBudgetDefinition>,
    #[serde(default)]
    pub nest: Option<ProceduralNestDefinition>,
    #[serde(default)]
    pub entry_terrain_id: Option<String>,
    #[serde(default)]
    pub entry_connection_id: Option<String>,
    #[serde(default)]
    pub completed_entry_terrain_id: Option<String>,
    #[serde(default)]
    pub failed_entry_terrain_id: Option<String>,
    #[serde(default)]
    pub abandoned_entry_terrain_id: Option<String>,
    #[serde(default = "default_allow_early_task_exit")]
    pub allow_early_task_exit: bool,
    #[serde(default)]
    pub retakeable: bool,
    #[serde(default)]
    #[cfg_attr(feature = "schemas", schemars(range(min = 1, max = 16)))]
    pub max_retakes: Option<u16>,
    #[serde(default)]
    pub retake_floor_policy: RetakeFloorPolicy,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub task_objective: Option<TaskObjectiveDefinition>,
    #[serde(default)]
    pub task_stages: Vec<TaskObjectiveDefinition>,
    #[serde(default)]
    pub task_reward: Option<TaskRewardDefinition>,
    #[serde(default)]
    pub next_floor_id: Option<String>,
    #[serde(default)]
    pub connections: Vec<ProceduralFloorConnectionDefinition>,
    pub depth: u16,
    pub width: u16,
    pub height: u16,
    pub wall_terrain_id: String,
    pub floor_terrain_id: String,
    pub up_stair_terrain_id: String,
    #[serde(default)]
    pub down_stair_terrain_id: Option<String>,
    pub closed_door_terrain_id: String,
    pub trap_terrain_id: String,
    pub actor_spawns: Vec<ProceduralActorSpawnDefinition>,
    pub loot_spawns: Vec<ProceduralLootSpawnDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorConnectionDefinition {
    pub id: String,
    pub kind: FloorConnectionKind,
    pub terrain_id: String,
    pub target_floor_id: String,
    #[serde(default)]
    pub target_connection_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FloorConnectionKind {
    Stairs,
    Shaft,
}

const fn default_allow_early_task_exit() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonGuardianDefinition {
    pub instance_id: String,
    pub actor_kind_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskObjectiveDefinition {
    pub kind: TaskObjectiveKind,
    #[serde(default)]
    pub floor_id: Option<String>,
    #[serde(default = "default_task_objective_required")]
    pub required: u32,
    #[serde(default)]
    pub item_instance_id: Option<String>,
    #[serde(default)]
    pub item_kind_id: Option<String>,
    #[serde(default)]
    pub actor_instance_id: Option<String>,
    #[serde(default)]
    pub actor_kind_id: Option<String>,
    #[serde(default)]
    pub spawn_count: Option<u32>,
}

const fn default_task_objective_required() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TaskObjectiveKind {
    CollectItem,
    EnterFloor,
    KillActor,
    KillActorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TaskRewardDefinition {
    pub item_instance_id: String,
    pub item_kind_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RetakeFloorPolicy {
    #[default]
    PreserveFloor,
    RegenerateFloor,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum FloorLifecycle {
    #[default]
    Dungeon,
    OneShot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralActorSpawnDefinition {
    pub instance_id: String,
    pub room_id: String,
    pub actor_kind_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLootSpawnDefinition {
    pub id: String,
    pub room_id: String,
    pub loot_table_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralGenerationBudgetDefinition {
    pub actor_slots: u16,
    pub loot_placements: u16,
    #[serde(default)]
    pub region_placements: Option<u16>,
    #[serde(default)]
    pub room_placements: Option<u16>,
    #[serde(default)]
    pub room_area_tiles: Option<u32>,
    #[serde(default)]
    pub cavern_area_tiles: Option<u32>,
    #[serde(default)]
    pub lake_area_tiles: Option<u32>,
    #[serde(default)]
    pub lake_deep_area_tiles: Option<u32>,
    #[serde(default)]
    pub river_area_tiles: Option<u32>,
    #[serde(default)]
    pub maze_floor_tiles: Option<u32>,
    #[serde(default)]
    pub destruction_centers: Option<u16>,
    #[serde(default)]
    pub destroyed_area_tiles: Option<u32>,
    #[serde(default)]
    pub streamer_placements: Option<u16>,
    #[serde(default)]
    pub streamer_area_tiles: Option<u32>,
    #[serde(default)]
    pub pit_placements: Option<u16>,
    #[serde(default)]
    pub pit_actor_slots: Option<u16>,
    #[serde(default)]
    pub vault_placements: Option<u16>,
    #[serde(default)]
    pub vault_area_tiles: Option<u32>,
    #[serde(default)]
    pub group_placements: Option<u16>,
    #[serde(default)]
    pub group_actor_slots: Option<u16>,
    #[serde(default)]
    pub feature_placements: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRoomGeometryDefinition {
    pub min_width: u16,
    pub max_width: u16,
    pub min_height: u16,
    pub max_height: u16,
    pub shapes: Vec<ProceduralRoomShapeCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLayoutDefinition {
    #[serde(default)]
    pub mode: ProceduralLayoutMode,
    #[serde(default)]
    pub rooms: Option<ProceduralRoomGeometryDefinition>,
    #[serde(default)]
    pub cavern: Option<ProceduralCavernDefinition>,
    #[serde(default)]
    pub lake: Option<ProceduralLakeDefinition>,
    #[serde(default)]
    pub river: Option<ProceduralRiverDefinition>,
    #[serde(default)]
    pub maze: Option<ProceduralMazeDefinition>,
    #[serde(default)]
    pub destroyed: Option<ProceduralDestroyedDefinition>,
    #[serde(default)]
    pub streamers: Vec<ProceduralStreamerCandidateDefinition>,
    #[serde(default)]
    pub pit: Option<ProceduralPitDefinition>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProceduralLayoutMode {
    #[default]
    Rooms,
    MazeOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralPitDefinition {
    pub encounter_table_id: String,
    pub inner_width: u16,
    pub inner_height: u16,
    pub roster_size: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralCavernDefinition {
    pub terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralLakeDefinition {
    pub deep_terrain_id: String,
    pub shallow_terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRiverDefinition {
    pub deep_terrain_id: String,
    pub shallow_terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralMazeDefinition {
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralDestroyedDefinition {
    pub terrain_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralStreamerCandidateDefinition {
    pub terrain_id: String,
    pub weight: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralRoomShapeCandidateDefinition {
    pub shape: ProceduralRoomShape,
    pub weight: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ProceduralRoomShape {
    Rectangle,
    Cross,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralNestDefinition {
    pub room_id: String,
    pub spawn_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompiledContentV1 {
    pub format: String,
    pub format_version: u16,
    pub pack_id: String,
    pub pack_version: String,
    pub title_key: String,
    pub dependencies: Vec<PackDependency>,
    pub load_after: Vec<String>,
    pub terrain: Vec<TerrainDefinition>,
    pub actors: Vec<ActorDefinition>,
    pub affixes: Vec<AffixDefinition>,
    pub items: Vec<ItemDefinition>,
    #[serde(default)]
    pub encounter_tables: Vec<EncounterTableDefinition>,
    #[serde(default)]
    pub loot_tables: Vec<LootTableDefinition>,
    #[serde(default)]
    pub theme_tables: Vec<ThemeTableDefinition>,
    #[serde(default)]
    pub region_tables: Vec<RegionTableDefinition>,
    #[serde(default)]
    pub terrain_feature_tables: Vec<TerrainFeatureTableDefinition>,
    #[serde(default)]
    pub vaults: Vec<VaultDefinition>,
    pub worlds: Vec<WorldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub content: CompiledContentV1,
    pub content_hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentCatalog {
    pack_id: String,
    pack_version: String,
    content_hash: String,
    terrain: BTreeMap<String, TerrainDefinition>,
    actors: BTreeMap<String, ActorDefinition>,
    affixes: BTreeMap<String, AffixDefinition>,
    items: BTreeMap<String, ItemDefinition>,
    encounter_tables: BTreeMap<String, EncounterTableDefinition>,
    loot_tables: BTreeMap<String, LootTableDefinition>,
    theme_tables: BTreeMap<String, ThemeTableDefinition>,
    region_tables: BTreeMap<String, RegionTableDefinition>,
    terrain_feature_tables: BTreeMap<String, TerrainFeatureTableDefinition>,
    vaults: BTreeMap<String, VaultDefinition>,
    worlds: BTreeMap<String, WorldDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSummary {
    pub pack_id: String,
    pub pack_version: String,
    pub content_hash: String,
    pub terrain_count: usize,
    pub actor_count: usize,
    pub affix_count: usize,
    pub item_count: usize,
    pub encounter_table_count: usize,
    pub loot_table_count: usize,
    pub theme_table_count: usize,
    pub region_table_count: usize,
    pub terrain_feature_table_count: usize,
    pub vault_count: usize,
    pub world_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentLockV1 {
    pub schema_version: u16,
    pub pack_id: String,
    pub pack_version: String,
    pub content_hash: String,
}

impl CompiledArtifact {
    #[must_use]
    pub fn summary(&self) -> ContentSummary {
        ContentSummary {
            pack_id: self.content.pack_id.clone(),
            pack_version: self.content.pack_version.clone(),
            content_hash: self.content_hash.clone(),
            terrain_count: self.content.terrain.len(),
            actor_count: self.content.actors.len(),
            affix_count: self.content.affixes.len(),
            item_count: self.content.items.len(),
            encounter_table_count: self.content.encounter_tables.len(),
            loot_table_count: self.content.loot_tables.len(),
            theme_table_count: self.content.theme_tables.len(),
            region_table_count: self.content.region_tables.len(),
            terrain_feature_table_count: self.content.terrain_feature_tables.len(),
            vault_count: self.content.vaults.len(),
            world_count: self.content.worlds.len(),
        }
    }
}

impl ContentCatalog {
    #[must_use]
    pub fn from_artifact(artifact: CompiledArtifact) -> Self {
        let CompiledArtifact {
            content,
            content_hash,
            ..
        } = artifact;
        Self {
            pack_id: content.pack_id,
            pack_version: content.pack_version,
            content_hash,
            terrain: content
                .terrain
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            actors: content
                .actors
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            affixes: content
                .affixes
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            items: content
                .items
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            encounter_tables: content
                .encounter_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            loot_tables: content
                .loot_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            theme_tables: content
                .theme_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            region_tables: content
                .region_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            terrain_feature_tables: content
                .terrain_feature_tables
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            vaults: content
                .vaults
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            worlds: content
                .worlds
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ContentError> {
        Ok(Self::from_artifact(decode_content(bytes)?))
    }

    #[must_use]
    pub fn pack_id(&self) -> &str {
        &self.pack_id
    }

    #[must_use]
    pub fn pack_version(&self) -> &str {
        &self.pack_version
    }

    #[must_use]
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    #[must_use]
    pub fn terrain(&self, id: &str) -> Option<&TerrainDefinition> {
        self.terrain.get(id)
    }

    #[must_use]
    pub fn actor(&self, id: &str) -> Option<&ActorDefinition> {
        self.actors.get(id)
    }

    #[must_use]
    pub fn item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    #[must_use]
    pub fn affix(&self, id: &str) -> Option<&AffixDefinition> {
        self.affixes.get(id)
    }

    #[must_use]
    pub fn loot_table(&self, id: &str) -> Option<&LootTableDefinition> {
        self.loot_tables.get(id)
    }

    #[must_use]
    pub fn encounter_table(&self, id: &str) -> Option<&EncounterTableDefinition> {
        self.encounter_tables.get(id)
    }

    #[must_use]
    pub fn theme_table(&self, id: &str) -> Option<&ThemeTableDefinition> {
        self.theme_tables.get(id)
    }

    #[must_use]
    pub fn region_table(&self, id: &str) -> Option<&RegionTableDefinition> {
        self.region_tables.get(id)
    }

    #[must_use]
    pub fn terrain_feature_table(&self, id: &str) -> Option<&TerrainFeatureTableDefinition> {
        self.terrain_feature_tables.get(id)
    }

    #[must_use]
    pub fn vault(&self, id: &str) -> Option<&VaultDefinition> {
        self.vaults.get(id)
    }

    #[must_use]
    pub fn world(&self, id: &str) -> Option<&WorldDefinition> {
        self.worlds.get(id)
    }

    #[must_use]
    pub fn visual_glyphs(&self) -> BTreeMap<String, String> {
        self.terrain
            .iter()
            .map(|(id, definition)| (id.clone(), definition.glyph.clone()))
            .chain(
                self.actors
                    .iter()
                    .map(|(id, definition)| (id.clone(), definition.glyph.clone())),
            )
            .chain(
                self.items
                    .iter()
                    .map(|(id, definition)| (id.clone(), definition.glyph.clone())),
            )
            .collect()
    }
}

pub fn compile_pack_dir(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidPackRoot(root.to_path_buf()));
    }

    let mut budget = SourceBudget::default();
    let manifest: PackManifest = read_json(&root.join("pack.json"), &mut budget)?;
    validate_manifest(&manifest)?;

    let roots = manifest
        .content_roots
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let content = CompiledContentV1 {
        format: CONTENT_FORMAT.to_owned(),
        format_version: CONTENT_FORMAT_VERSION,
        pack_id: manifest.id,
        pack_version: manifest.version,
        title_key: manifest.title_key,
        dependencies: manifest.dependencies,
        load_after: manifest.load_after,
        terrain: load_root(root, "terrain", &roots, &mut budget)?,
        actors: load_root(root, "actors", &roots, &mut budget)?,
        affixes: load_root(root, "affixes", &roots, &mut budget)?,
        items: load_root(root, "items", &roots, &mut budget)?,
        encounter_tables: load_root(root, "encounterTables", &roots, &mut budget)?,
        loot_tables: load_root(root, "lootTables", &roots, &mut budget)?,
        theme_tables: load_root(root, "themeTables", &roots, &mut budget)?,
        region_tables: load_root(root, "regionTables", &roots, &mut budget)?,
        terrain_feature_tables: load_root(root, "terrainFeatureTables", &roots, &mut budget)?,
        vaults: load_root(root, "vaults", &roots, &mut budget)?,
        worlds: load_root(root, "worlds", &roots, &mut budget)?,
    };
    encode_content(content)
}

pub fn verify_pack_lock(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let artifact = compile_pack_dir(root)?;
    let mut budget = SourceBudget::default();
    let content_lock: ContentLockV1 = read_json(&root.join("content.lock.json"), &mut budget)?;
    if content_lock.schema_version != 1
        || content_lock.pack_id != artifact.content.pack_id
        || content_lock.pack_version != artifact.content.pack_version
        || content_lock.content_hash != artifact.content_hash
    {
        return Err(ContentError::ContentLockMismatch);
    }
    Ok(artifact)
}

pub fn encode_content(mut content: CompiledContentV1) -> Result<CompiledArtifact, ContentError> {
    validate_and_normalize(&mut content)?;
    let payload = rmp_serde::to_vec_named(&content)?;
    if payload.len() > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload.len()));
    }
    let content_hash = sha256(&payload);
    let payload_length = u64::try_from(payload.len()).map_err(|_| ContentError::LengthOverflow)?;
    let capacity = FIXED_HEADER_LENGTH
        .checked_add(payload.len())
        .ok_or(ContentError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    Ok(CompiledArtifact {
        content,
        content_hash,
        bytes,
    })
}

pub fn decode_content(bytes: &[u8]) -> Result<CompiledArtifact, ContentError> {
    if bytes.len() < FIXED_HEADER_LENGTH || &bytes[..8] != MAGIC {
        return Err(ContentError::InvalidContainer);
    }
    let version = read_u16(bytes, 8)?;
    if version != CONTAINER_VERSION {
        return Err(ContentError::UnsupportedContainerVersion(version));
    }
    let flags = read_u16(bytes, 10)?;
    if flags != 0 {
        return Err(ContentError::UnsupportedContainerFlags(flags));
    }
    let payload_length =
        usize::try_from(read_u64(bytes, 12)?).map_err(|_| ContentError::LengthOverflow)?;
    if payload_length > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload_length));
    }
    let expected_length = FIXED_HEADER_LENGTH
        .checked_add(payload_length)
        .ok_or(ContentError::LengthOverflow)?;
    if bytes.len() != expected_length {
        return Err(ContentError::InvalidContainer);
    }
    let payload = &bytes[FIXED_HEADER_LENGTH..];
    let actual_checksum = Sha256::digest(payload);
    if bytes[20..52] != actual_checksum[..] {
        return Err(ContentError::ChecksumMismatch);
    }
    let content: CompiledContentV1 = rmp_serde::from_slice(payload)?;
    let mut normalized = content.clone();
    validate_and_normalize(&mut normalized)?;
    if normalized != content {
        return Err(ContentError::NonCanonicalCompiledContent);
    }
    Ok(CompiledArtifact {
        content,
        content_hash: sha256(payload),
        bytes: bytes.to_vec(),
    })
}

pub fn read_compiled_file(path: &Path) -> Result<CompiledArtifact, ContentError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((FIXED_HEADER_LENGTH + MAX_COMPILED_PAYLOAD_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    decode_content(&bytes)
}

fn validate_manifest(manifest: &PackManifest) -> Result<(), ContentError> {
    require_schema(&manifest.schema, PACK_SCHEMA, "pack.json")?;
    require_format_version(manifest.format_version, "pack.json")?;
    validate_id(&manifest.id)?;
    validate_semver(&manifest.version)?;
    validate_message_key(&manifest.title_key)?;

    let mut roots = BTreeSet::new();
    for root in &manifest.content_roots {
        if !SUPPORTED_ROOTS.contains(&root.as_str()) {
            return Err(ContentError::UnsupportedContentRoot(root.clone()));
        }
        if !roots.insert(root.as_str()) {
            return Err(ContentError::DuplicateContentRoot(root.clone()));
        }
    }
    validate_pack_relations(&manifest.id, &manifest.dependencies, &manifest.load_after)
}

fn validate_and_normalize(content: &mut CompiledContentV1) -> Result<(), ContentError> {
    if content.format != CONTENT_FORMAT || content.format_version != CONTENT_FORMAT_VERSION {
        return Err(ContentError::InvalidCompiledMetadata);
    }
    validate_id(&content.pack_id)?;
    validate_semver(&content.pack_version)?;
    validate_message_key(&content.title_key)?;
    validate_pack_relations(&content.pack_id, &content.dependencies, &content.load_after)?;
    content
        .dependencies
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.load_after.sort();
    content
        .terrain
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.actors.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .affixes
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.items.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .encounter_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .loot_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .theme_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .region_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .terrain_feature_tables
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.vaults.sort_by(|left, right| left.id.cmp(&right.id));
    content.worlds.sort_by(|left, right| left.id.cmp(&right.id));

    let mut all_ids = BTreeSet::new();
    let mut terrain_ids = BTreeSet::new();
    let mut terrain_walkability = BTreeMap::new();
    let mut terrain_tags = BTreeMap::new();
    let mut terrain_open_targets = BTreeMap::new();
    let mut terrain_traps = BTreeSet::new();
    for terrain in &mut content.terrain {
        require_schema(&terrain.schema, TERRAIN_SCHEMA, &terrain.id)?;
        require_format_version(terrain.format_version, &terrain.id)?;
        validate_definition_id(&terrain.id, "terrain")?;
        validate_definition_text(&terrain.id, &terrain.name_key, &terrain.description_key)?;
        validate_glyph(&terrain.id, &terrain.glyph)?;
        normalize_tags(&terrain.id, &mut terrain.tags)?;
        insert_definition_id(&mut all_ids, &terrain.id)?;
        terrain_ids.insert(terrain.id.clone());
        terrain_walkability.insert(terrain.id.clone(), terrain.walkable);
        terrain_tags.insert(
            terrain.id.clone(),
            terrain.tags.iter().cloned().collect::<BTreeSet<_>>(),
        );
        if let Some(target_id) = &terrain.open_to_terrain_id {
            terrain_open_targets.insert(terrain.id.clone(), target_id.clone());
        }
        if terrain.trap.is_some() {
            terrain_traps.insert(terrain.id.clone());
        }
    }
    for terrain in &content.terrain {
        if terrain.open_to_terrain_id.is_some() && terrain.close_to_terrain_id.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.open_check_difficulty.is_some() && terrain.open_to_terrain_id.is_none() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.bash_to_terrain_id.is_some() != terrain.bash_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.dig_to_terrain_id.is_some() != terrain.dig_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.concealed_as_terrain_id.is_some() != terrain.search_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain
            .open_check_difficulty
            .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .bash_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .dig_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .search_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
        {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if let Some(target_id) = &terrain.open_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
                || (terrain.open_check_difficulty.is_none()
                    && target.close_to_terrain_id.as_deref() != Some(terrain.id.as_str()))
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.close_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || !terrain.walkable
                || terrain.blocks_sight
                || target.walkable
                || !target.blocks_sight
                || target.open_to_terrain_id.as_deref() != Some(terrain.id.as_str())
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.bash_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.dig_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.concealed_as_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable != target.walkable
                || terrain.blocks_sight != target.blocks_sight
                || target.concealed_as_terrain_id.is_some()
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(trap) = &terrain.trap {
            require_reference(&terrain_ids, &trap.disarm_to_terrain_id, &terrain.id)?;
            let target = content
                .terrain
                .iter()
                .find(|candidate| candidate.id == trap.disarm_to_terrain_id)
                .expect("validated trap target must remain available");
            if trap.damage <= 0
                || !(1..=1_000_000).contains(&trap.disarm_check_difficulty)
                || trap.disarm_to_terrain_id == terrain.id
                || !terrain.walkable
                || terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
                || terrain.concealed_as_terrain_id.is_none()
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
    }

    let mut actor_roles = BTreeMap::new();
    let mut actor_levels = BTreeMap::new();
    let mut actor_loot_table_ids = Vec::new();
    for actor in &mut content.actors {
        require_schema(&actor.schema, ACTOR_SCHEMA, &actor.id)?;
        require_format_version(actor.format_version, &actor.id)?;
        validate_definition_id(&actor.id, "actor")?;
        validate_definition_text(&actor.id, &actor.name_key, &actor.description_key)?;
        validate_glyph(&actor.id, &actor.glyph)?;
        if actor.level > 10_000
            || actor.max_hp <= 0
            || actor.max_hp > 1_000_000
            || actor.speed > 199
            || actor.attack <= 0
            || actor.attack > 1_000_000
            || actor.defense < 0
            || actor.defense > 1_000_000
            || actor.door_skill < 0
            || actor.door_skill > 1_000_000
            || actor.bash_power < 0
            || actor.bash_power > 1_000_000
            || actor.search_skill < 0
            || actor.search_skill > 1_000_000
            || actor.damage_dice == 0
            || actor.damage_dice > 100
            || actor.damage_sides == 0
            || actor.damage_sides > 10_000
        {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if (actor.role == ActorRole::Player
            && (actor.carry_capacity_tenths_pound == 0
                || actor.carry_capacity_tenths_pound > 1_000_000))
            || (actor.role == ActorRole::Monster && actor.carry_capacity_tenths_pound != 0)
        {
            return Err(ContentError::InvalidActorCarryCapacity(actor.id.clone()));
        }
        if let Some(routine) = &actor.melee_routine
            && (actor.role != ActorRole::Monster
                || routine.blows.is_empty()
                || routine.blows.len() > 8
                || routine.blows.iter().any(|blow| {
                    validate_id(&blow.method_id).is_err()
                        || blow.to_hit < -1_000_000
                        || blow.to_hit > 1_000_000
                        || blow.damage_dice == 0
                        || blow.damage_dice > 100
                        || blow.damage_sides == 0
                        || blow.damage_sides > 10_000
                }))
        {
            return Err(ContentError::InvalidMeleeRoutine(actor.id.clone()));
        }
        if let Some(loot_table_id) = &actor.loot_table_id {
            if actor.role != ActorRole::Monster || validate_id(loot_table_id).is_err() {
                return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
            }
            actor_loot_table_ids.push((actor.id.clone(), loot_table_id.clone()));
        }
        if let Some(loot_table_id) = &actor.carried_loot_table_id {
            if actor.role != ActorRole::Monster || validate_id(loot_table_id).is_err() {
                return Err(ContentError::InvalidActorLootTable(actor.id.clone()));
            }
            actor_loot_table_ids.push((format!("{}#carried", actor.id), loot_table_id.clone()));
        }
        normalize_tags(&actor.id, &mut actor.tags)?;
        insert_definition_id(&mut all_ids, &actor.id)?;
        actor_roles.insert(actor.id.clone(), actor.role);
        actor_levels.insert(actor.id.clone(), actor.level);
    }

    let mut affix_ids = BTreeSet::new();
    for affix in &mut content.affixes {
        require_schema(&affix.schema, AFFIX_SCHEMA, &affix.id)?;
        require_format_version(affix.format_version, &affix.id)?;
        validate_definition_id(&affix.id, "affix")?;
        validate_definition_text(&affix.id, &affix.name_key, &affix.description_key)?;
        let modifiers = &affix.modifiers;
        if modifiers == &StatModifiers::default()
            || modifiers.max_hp < -1_000_000
            || modifiers.max_hp > 1_000_000
            || modifiers.attack < -1_000_000
            || modifiers.attack > 1_000_000
            || modifiers.defense < -1_000_000
            || modifiers.defense > 1_000_000
        {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        normalize_tags(&affix.id, &mut affix.tags)?;
        insert_definition_id(&mut all_ids, &affix.id)?;
        affix_ids.insert(affix.id.clone());
    }

    let mut item_limits = BTreeMap::new();
    for item in &mut content.items {
        require_schema(&item.schema, ITEM_SCHEMA, &item.id)?;
        require_format_version(item.format_version, &item.id)?;
        validate_definition_id(&item.id, "item")?;
        validate_definition_text(&item.id, &item.name_key, &item.description_key)?;
        if let Some(appearance_name_key) = &item.appearance_name_key {
            validate_message_key(appearance_name_key)?;
            if appearance_name_key == &item.name_key {
                return Err(ContentError::InvalidItemAppearance(item.id.clone()));
            }
        }
        validate_glyph(&item.id, &item.glyph)?;
        if item.weight_tenths_pound == 0 || item.weight_tenths_pound > 10_000 {
            return Err(ContentError::InvalidItemWeight(item.id.clone()));
        }
        if item.max_stack == 0 || item.max_stack > 1_000_000 {
            return Err(ContentError::InvalidItemStack(item.id.clone()));
        }
        if item.break_chance_percent > 100 {
            return Err(ContentError::InvalidItemBreakChance(item.id.clone()));
        }
        if let Some(slot) = &item.equipment_slot
            && (item.max_stack != 1 || validate_equipment_slot(slot).is_err())
        {
            return Err(ContentError::InvalidEquipmentSlot(item.id.clone()));
        }
        if item.modifiers.max_hp < 0
            || item.modifiers.max_hp > 1_000_000
            || item.modifiers.attack < -1_000_000
            || item.modifiers.attack > 1_000_000
            || item.modifiers.defense < -1_000_000
            || item.modifiers.defense > 1_000_000
            || (item.equipment_slot.is_none() && item.modifiers != StatModifiers::default())
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        if let Some(profile) = &item.melee_profile
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("weapon")
                || profile.attacks == 0
                || profile.attacks > 8
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidAttackProfile(item.id.clone()));
        }
        if let Some(profile) = &item.projectile_profile
            && (item.max_stack != 1
                || item.equipment_slot.as_deref() != Some("launcher")
                || profile.range == 0
                || profile.range > 32
                || profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
        if let Some(profile) = &item.throw_profile
            && (profile.to_hit < -1_000_000
                || profile.to_hit > 1_000_000
                || profile.to_damage < -1_000_000
                || profile.to_damage > 1_000_000
                || profile.damage_dice == 0
                || profile.damage_dice > 100
                || profile.damage_sides == 0
                || profile.damage_sides > 10_000)
        {
            return Err(ContentError::InvalidThrowProfile(item.id.clone()));
        }
        if let Some(action) = &item.use_action {
            let valid_effect = match action.effect {
                ItemUseEffectDefinition::Heal { amount } => (1..=1_000_000).contains(&amount),
            };
            if item.equipment_slot.is_some() || !valid_effect {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        normalize_tags(&item.id, &mut item.tags)?;
        insert_definition_id(&mut all_ids, &item.id)?;
        item_limits.insert(
            item.id.clone(),
            (item.max_stack, item.equipment_slot.is_some()),
        );
    }

    for item in &content.items {
        let Some(profile) = &item.projectile_profile else {
            continue;
        };
        let Some(ammo) = content
            .items
            .iter()
            .find(|candidate| candidate.id == profile.ammo_kind_id)
        else {
            return Err(ContentError::DanglingReference {
                owner: item.id.clone(),
                target: profile.ammo_kind_id.clone(),
            });
        };
        if ammo.max_stack <= 1 || !ammo.tags.iter().any(|tag| tag == "ammunition") {
            return Err(ContentError::InvalidProjectileProfile(item.id.clone()));
        }
    }

    let mut loot_table_ids = BTreeSet::new();
    for table in &mut content.loot_tables {
        require_schema(&table.schema, LOOT_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "loot-table")?;
        if table.rolls == 0
            || table.rolls > 16
            || table.entries.is_empty()
            || table.entries.len() > 64
            || table.quality_weights.is_empty()
            || table.quality_weights.len() > 3
            || table.affix_weights.is_empty()
            || table.affix_weights.len() > 64
        {
            return Err(ContentError::InvalidLootTable(table.id.clone()));
        }

        table.entries.sort_by(|left, right| {
            left.item_kind_id
                .cmp(&right.item_kind_id)
                .then(left.quantity.cmp(&right.quantity))
        });
        table.quality_weights.sort_by_key(|entry| entry.quality);
        table
            .affix_weights
            .sort_by(|left, right| left.affix_id.as_deref().cmp(&right.affix_id.as_deref()));

        let mut entry_ids = BTreeSet::new();
        let mut quality_ids = BTreeSet::new();
        let mut affix_entries = BTreeSet::new();
        let mut entry_weight = 0_u64;
        let mut quality_weight = 0_u64;
        let mut affix_weight = 0_u64;
        for entry in &table.entries {
            let Some((max_stack, equippable)) = item_limits.get(&entry.item_kind_id) else {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.item_kind_id.clone(),
                });
            };
            if entry.weight == 0
                || entry.quantity == 0
                || entry.quantity > *max_stack
                || !entry_ids.insert(entry.item_kind_id.as_str())
                || ((table
                    .quality_weights
                    .iter()
                    .any(|quality| quality.quality != ItemQuality::Ordinary)
                    || table
                        .affix_weights
                        .iter()
                        .any(|affix| affix.affix_id.is_some()))
                    && (*max_stack != 1 || entry.quantity != 1))
                || (table
                    .affix_weights
                    .iter()
                    .any(|affix| affix.affix_id.is_some())
                    && !equippable)
            {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            entry_weight = entry_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        for entry in &table.quality_weights {
            if entry.weight == 0 || !quality_ids.insert(entry.quality) {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            quality_weight = quality_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        for entry in &table.affix_weights {
            if entry.weight == 0 || !affix_entries.insert(entry.affix_id.as_deref()) {
                return Err(ContentError::InvalidLootTable(table.id.clone()));
            }
            if let Some(affix_id) = &entry.affix_id
                && !affix_ids.contains(affix_id)
            {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: affix_id.clone(),
                });
            }
            affix_weight = affix_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidLootTable(table.id.clone()))?;
        }
        if entry_weight == 0 || quality_weight == 0 || affix_weight == 0 {
            return Err(ContentError::InvalidLootTable(table.id.clone()));
        }
        insert_definition_id(&mut all_ids, &table.id)?;
        loot_table_ids.insert(table.id.clone());
    }

    for (actor_id, loot_table_id) in actor_loot_table_ids {
        require_reference(&loot_table_ids, &loot_table_id, &actor_id)?;
    }

    let mut encounter_tables_by_id = BTreeMap::new();
    for table in &mut content.encounter_tables {
        require_schema(&table.schema, ENCOUNTER_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "encounter-table")?;
        if table.rolls == 0
            || table.rolls > 16
            || table.entries.is_empty()
            || table.entries.len() > 64
        {
            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.actor_kind_id
                .cmp(&right.actor_kind_id)
                .then(left.min_depth.cmp(&right.min_depth))
                .then(left.max_depth.cmp(&right.max_depth))
        });
        let mut actor_ids = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &mut table.entries {
            require_actor_role(
                &actor_roles,
                &entry.actor_kind_id,
                ActorRole::Monster,
                &table.id,
            )?;
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || actor_levels
                    .get(&entry.actor_kind_id)
                    .is_none_or(|level| *level > u32::from(entry.max_depth))
                || !actor_ids.insert(entry.actor_kind_id.clone())
            {
                return Err(ContentError::InvalidEncounterTable(table.id.clone()));
            }
            if let Some(group) = &mut entry.group {
                let friends_are_valid = group.friends.as_ref().is_none_or(|friends| {
                    friends.max_count > 0
                        && friends.min_count <= friends.max_count
                        && friends.max_count <= 7
                });
                let escort_is_valid = group.escort.as_ref().is_none_or(|escort| {
                    escort.max_count > 0
                        && escort.min_count <= escort.max_count
                        && escort.max_count <= 7
                        && !escort.entries.is_empty()
                        && escort.entries.len() <= 64
                });
                if !friends_are_valid
                    || !escort_is_valid
                    || group.min_companion_count() == 0
                    || group.max_companion_count() > 7
                    || group.pack_ai.leader == MonsterPackBehavior::GuardLeader
                {
                    return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                }
                if let Some(escort) = &mut group.escort {
                    escort.entries.sort_by(|left, right| {
                        left.actor_kind_id
                            .cmp(&right.actor_kind_id)
                            .then(left.min_depth.cmp(&right.min_depth))
                            .then(left.max_depth.cmp(&right.max_depth))
                    });
                    let mut escort_actor_ids = BTreeSet::new();
                    let mut escort_weight = 0_u64;
                    for escort_entry in &escort.entries {
                        require_actor_role(
                            &actor_roles,
                            &escort_entry.actor_kind_id,
                            ActorRole::Monster,
                            &table.id,
                        )?;
                        if escort_entry.weight == 0
                            || escort_entry.min_depth < entry.min_depth
                            || escort_entry.min_depth > escort_entry.max_depth
                            || escort_entry.max_depth > entry.max_depth
                            || actor_levels
                                .get(&escort_entry.actor_kind_id)
                                .is_none_or(|level| *level > u32::from(escort_entry.max_depth))
                            || !escort_actor_ids.insert(escort_entry.actor_kind_id.clone())
                        {
                            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                        }
                        escort_weight = escort_weight
                            .checked_add(u64::from(escort_entry.weight))
                            .ok_or_else(|| {
                            ContentError::InvalidEncounterTable(table.id.clone())
                        })?;
                    }
                    if escort_weight == 0
                        || (entry.min_depth..=entry.max_depth).any(|depth| {
                            !escort.entries.iter().any(|escort_entry| {
                                escort_entry.min_depth <= depth
                                    && depth <= escort_entry.max_depth
                                    && actor_levels
                                        .get(&escort_entry.actor_kind_id)
                                        .is_some_and(|level| *level <= u32::from(depth))
                            })
                        })
                    {
                        return Err(ContentError::InvalidEncounterTable(table.id.clone()));
                    }
                }
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidEncounterTable(table.id.clone()))?;
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidEncounterTable(table.id.clone()));
        }
        insert_definition_id(&mut all_ids, &table.id)?;
        encounter_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut vaults_by_id = BTreeMap::new();
    for vault in &mut content.vaults {
        require_schema(&vault.schema, VAULT_SCHEMA, &vault.id)?;
        require_format_version(vault.format_version, &vault.id)?;
        validate_definition_id(&vault.id, "vault")?;
        validate_message_key(&vault.name_key)?;
        validate_definition_id(&vault.theme_id, "theme")?;
        vault.transforms.sort();
        let transform_count = vault
            .transforms
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        if !(2..=12).contains(&vault.width)
            || !(2..=12).contains(&vault.height)
            || vault.entrance_position.x >= vault.width
            || vault.entrance_position.y >= vault.height
            || !(vault.entrance_position.x == 0
                || vault.entrance_position.x + 1 == vault.width
                || vault.entrance_position.y == 0
                || vault.entrance_position.y + 1 == vault.height)
            || transform_count != vault.transforms.len()
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        require_reference(&terrain_ids, &vault.base_terrain_id, &vault.id)?;
        if terrain_walkability.get(&vault.base_terrain_id) != Some(&true) {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }

        for terrain_override in &mut vault.terrain_overrides {
            terrain_override.positions.sort();
        }
        vault.terrain_overrides.sort_by(|left, right| {
            left.terrain_id
                .cmp(&right.terrain_id)
                .then(left.positions.cmp(&right.positions))
        });
        let mut terrain_by_position = BTreeMap::new();
        let mut terrain_override_ids = BTreeSet::new();
        for terrain_override in &mut vault.terrain_overrides {
            require_reference(&terrain_ids, &terrain_override.terrain_id, &vault.id)?;
            if terrain_override.positions.is_empty()
                || !terrain_override_ids.insert(terrain_override.terrain_id.clone())
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
            for position in &terrain_override.positions {
                if position.x >= vault.width
                    || position.y >= vault.height
                    || terrain_by_position
                        .insert(*position, terrain_override.terrain_id.clone())
                        .is_some()
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
        }

        vault
            .encounter_groups
            .sort_by(|left, right| left.id.cmp(&right.id));
        vault
            .loot_spawns
            .sort_by(|left, right| left.id.cmp(&right.id));
        if vault.encounter_groups.is_empty()
            || vault.encounter_groups.len() > 16
            || vault.loot_spawns.is_empty()
            || vault.loot_spawns.len() > 16
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        let mut section_ids = BTreeSet::new();
        let mut occupied_positions = BTreeSet::new();
        for group in &mut vault.encounter_groups {
            validate_id(&group.id)?;
            group.member_positions.sort();
            group.entries.sort_by(|left, right| {
                left.actor_kind_id
                    .cmp(&right.actor_kind_id)
                    .then(left.min_depth.cmp(&right.min_depth))
                    .then(left.max_depth.cmp(&right.max_depth))
            });
            if !section_ids.insert(group.id.clone())
                || group.member_positions.is_empty()
                || group.member_positions.len() > 16
                || group.entries.is_empty()
                || group.entries.len() > 64
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
            let mut entry_ids = BTreeSet::new();
            for entry in &group.entries {
                require_actor_role(
                    &actor_roles,
                    &entry.actor_kind_id,
                    ActorRole::Monster,
                    &vault.id,
                )?;
                if entry.weight == 0
                    || entry.min_depth == 0
                    || entry.min_depth > entry.max_depth
                    || entry.max_depth > 1_000
                    || actor_levels
                        .get(&entry.actor_kind_id)
                        .is_none_or(|level| *level > u32::from(entry.max_depth))
                    || !entry_ids.insert(entry.actor_kind_id.clone())
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
            for position in &group.member_positions {
                let terrain_id = terrain_by_position
                    .get(position)
                    .unwrap_or(&vault.base_terrain_id);
                if position.x >= vault.width
                    || position.y >= vault.height
                    || terrain_walkability.get(terrain_id) != Some(&true)
                    || !occupied_positions.insert(*position)
                {
                    return Err(ContentError::InvalidVault(vault.id.clone()));
                }
            }
        }
        for spawn in &vault.loot_spawns {
            validate_id(&spawn.id)?;
            require_reference(&loot_table_ids, &spawn.loot_table_id, &vault.id)?;
            let terrain_id = terrain_by_position
                .get(&spawn.position)
                .unwrap_or(&vault.base_terrain_id);
            if !section_ids.insert(spawn.id.clone())
                || spawn.position.x >= vault.width
                || spawn.position.y >= vault.height
                || terrain_walkability.get(terrain_id) != Some(&true)
                || !occupied_positions.insert(spawn.position)
            {
                return Err(ContentError::InvalidVault(vault.id.clone()));
            }
        }
        insert_definition_id(&mut all_ids, &vault.id)?;
        vaults_by_id.insert(vault.id.clone(), vault.clone());
    }

    let mut theme_tables_by_id = BTreeMap::new();
    for table in &mut content.theme_tables {
        require_schema(&table.schema, THEME_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "theme-table")?;
        if table.entries.is_empty() || table.entries.len() > 64 {
            return Err(ContentError::InvalidThemeTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.min_depth
                .cmp(&right.min_depth)
                .then(left.max_depth.cmp(&right.max_depth))
                .then(left.theme_id.cmp(&right.theme_id))
                .then(left.floor_terrain_id.cmp(&right.floor_terrain_id))
        });
        let mut entry_keys = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &mut table.entries {
            validate_definition_id(&entry.theme_id, "theme")?;
            require_reference(&terrain_ids, &entry.floor_terrain_id, &table.id)?;
            entry.vault_candidates.sort_by(|left, right| {
                left.vault_id
                    .cmp(&right.vault_id)
                    .then(left.min_depth.cmp(&right.min_depth))
                    .then(left.max_depth.cmp(&right.max_depth))
            });
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || terrain_walkability.get(&entry.floor_terrain_id) != Some(&true)
                || entry.vault_candidates.len() > 64
                || !entry_keys.insert((entry.theme_id.clone(), entry.min_depth, entry.max_depth))
            {
                return Err(ContentError::InvalidThemeTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidThemeTable(table.id.clone()))?;
            let mut vault_ids = BTreeSet::new();
            let mut vault_weight = 0_u64;
            for candidate in &entry.vault_candidates {
                let Some(vault) = vaults_by_id.get(&candidate.vault_id) else {
                    return Err(ContentError::DanglingReference {
                        owner: table.id.clone(),
                        target: candidate.vault_id.clone(),
                    });
                };
                if candidate.weight == 0
                    || candidate.min_depth < entry.min_depth
                    || candidate.min_depth > candidate.max_depth
                    || candidate.max_depth > entry.max_depth
                    || vault.theme_id != entry.theme_id
                    || !vault_ids.insert(candidate.vault_id.clone())
                {
                    return Err(ContentError::InvalidThemeTable(table.id.clone()));
                }
                vault_weight = vault_weight
                    .checked_add(u64::from(candidate.weight))
                    .ok_or_else(|| ContentError::InvalidThemeTable(table.id.clone()))?;
            }
            if !entry.vault_candidates.is_empty() && vault_weight == 0 {
                return Err(ContentError::InvalidThemeTable(table.id.clone()));
            }
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidThemeTable(table.id.clone()));
        }
        insert_definition_id(&mut all_ids, &table.id)?;
        theme_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut region_tables_by_id = BTreeMap::new();
    for table in &mut content.region_tables {
        require_schema(&table.schema, REGION_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "region-table")?;
        if table.entries.len() < 2 || table.entries.len() > 32 {
            return Err(ContentError::InvalidRegionTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.region_id
                .cmp(&right.region_id)
                .then(left.min_depth.cmp(&right.min_depth))
                .then(left.max_depth.cmp(&right.max_depth))
        });
        let mut region_ids = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &table.entries {
            validate_definition_id(&entry.region_id, "region")?;
            let Some(theme_table) = theme_tables_by_id.get(&entry.theme_table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.theme_table_id.clone(),
                });
            };
            if !encounter_tables_by_id.contains_key(&entry.encounter_table_id) {
                return Err(ContentError::DanglingReference {
                    owner: table.id.clone(),
                    target: entry.encounter_table_id.clone(),
                });
            }
            require_reference(&loot_table_ids, &entry.loot_table_id, &table.id)?;
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || !region_ids.insert(entry.region_id.clone())
                || !theme_table.entries.iter().any(|theme| {
                    theme.theme_id == entry.theme_id
                        && theme.min_depth <= entry.min_depth
                        && entry.max_depth <= theme.max_depth
                })
            {
                return Err(ContentError::InvalidRegionTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidRegionTable(table.id.clone()))?;
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidRegionTable(table.id.clone()));
        }
        insert_definition_id(&mut all_ids, &table.id)?;
        region_tables_by_id.insert(table.id.clone(), table.clone());
    }

    let mut terrain_feature_tables_by_id = BTreeMap::new();
    for table in &mut content.terrain_feature_tables {
        require_schema(&table.schema, TERRAIN_FEATURE_TABLE_SCHEMA, &table.id)?;
        require_format_version(table.format_version, &table.id)?;
        validate_definition_id(&table.id, "terrain-feature-table")?;
        if !(1..=8).contains(&table.rolls) || table.entries.is_empty() || table.entries.len() > 64 {
            return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
        }
        table.entries.sort_by(|left, right| {
            left.min_depth
                .cmp(&right.min_depth)
                .then(left.max_depth.cmp(&right.max_depth))
                .then(left.placement.cmp(&right.placement))
                .then(left.terrain_id.cmp(&right.terrain_id))
        });
        let mut entry_keys = BTreeSet::new();
        let mut total_weight = 0_u64;
        for entry in &table.entries {
            require_reference(&terrain_ids, &entry.terrain_id, &table.id)?;
            let terrain = content
                .terrain
                .iter()
                .find(|terrain| terrain.id == entry.terrain_id)
                .expect("validated terrain feature must remain available");
            let placement_matches_terrain = match entry.placement {
                TerrainFeaturePlacement::Room => {
                    terrain.trap.is_some() || terrain.dig_to_terrain_id.is_some()
                }
                TerrainFeaturePlacement::Corridor => terrain.open_to_terrain_id.is_some(),
            };
            if entry.weight == 0
                || entry.min_depth == 0
                || entry.min_depth > entry.max_depth
                || entry.max_depth > 1_000
                || !placement_matches_terrain
                || !entry_keys.insert((
                    entry.terrain_id.clone(),
                    entry.placement,
                    entry.min_depth,
                    entry.max_depth,
                ))
            {
                return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
            }
            total_weight = total_weight
                .checked_add(u64::from(entry.weight))
                .ok_or_else(|| ContentError::InvalidTerrainFeatureTable(table.id.clone()))?;
        }
        if total_weight == 0 {
            return Err(ContentError::InvalidTerrainFeatureTable(table.id.clone()));
        }
        insert_definition_id(&mut all_ids, &table.id)?;
        terrain_feature_tables_by_id.insert(table.id.clone(), table.clone());
    }

    for world in &mut content.worlds {
        require_schema(&world.schema, WORLD_SCHEMA, &world.id)?;
        require_format_version(world.format_version, &world.id)?;
        validate_definition_id(&world.id, "world")?;
        validate_message_key(&world.name_key)?;
        insert_definition_id(&mut all_ids, &world.id)?;
        validate_world(
            world,
            &WorldValidationRefs {
                terrain_ids: &terrain_ids,
                terrain_walkability: &terrain_walkability,
                terrain_tags: &terrain_tags,
                terrain_open_targets: &terrain_open_targets,
                terrain_traps: &terrain_traps,
                actor_roles: &actor_roles,
                actor_levels: &actor_levels,
                item_limits: &item_limits,
                affix_ids: &affix_ids,
                encounter_tables: &encounter_tables_by_id,
                loot_table_ids: &loot_table_ids,
                theme_tables: &theme_tables_by_id,
                region_tables: &region_tables_by_id,
                terrain_feature_tables: &terrain_feature_tables_by_id,
                vaults: &vaults_by_id,
            },
        )?;
    }
    Ok(())
}

struct WorldValidationRefs<'a> {
    terrain_ids: &'a BTreeSet<String>,
    terrain_walkability: &'a BTreeMap<String, bool>,
    terrain_tags: &'a BTreeMap<String, BTreeSet<String>>,
    terrain_open_targets: &'a BTreeMap<String, String>,
    terrain_traps: &'a BTreeSet<String>,
    actor_roles: &'a BTreeMap<String, ActorRole>,
    actor_levels: &'a BTreeMap<String, u32>,
    item_limits: &'a BTreeMap<String, (u32, bool)>,
    affix_ids: &'a BTreeSet<String>,
    encounter_tables: &'a BTreeMap<String, EncounterTableDefinition>,
    loot_table_ids: &'a BTreeSet<String>,
    theme_tables: &'a BTreeMap<String, ThemeTableDefinition>,
    region_tables: &'a BTreeMap<String, RegionTableDefinition>,
    terrain_feature_tables: &'a BTreeMap<String, TerrainFeatureTableDefinition>,
    vaults: &'a BTreeMap<String, VaultDefinition>,
}

fn validate_task_objective(
    owner_id: &str,
    objective: &TaskObjectiveDefinition,
    floor_ids: &BTreeSet<String>,
    actor_roles: &BTreeMap<String, ActorRole>,
    item_limits: &BTreeMap<String, (u32, bool)>,
    instance_ids: &mut BTreeSet<String>,
) -> Result<(), ContentError> {
    if objective
        .floor_id
        .as_ref()
        .is_some_and(|floor_id| !floor_ids.contains(floor_id))
    {
        return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
    }
    match objective.kind {
        TaskObjectiveKind::CollectItem => {
            let (Some(instance_id), Some(kind_id)) =
                (&objective.item_instance_id, &objective.item_kind_id)
            else {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            };
            validate_id(instance_id)?;
            if !instance_ids.insert(instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(instance_id.clone()));
            }
            if !item_limits.contains_key(kind_id) {
                return Err(ContentError::DanglingReference {
                    owner: owner_id.to_owned(),
                    target: kind_id.clone(),
                });
            }
            if objective.required != 1
                || objective.spawn_count.is_some()
                || objective.actor_instance_id.is_some()
                || objective.actor_kind_id.is_some()
            {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::EnterFloor => {
            if objective.floor_id.is_none()
                || objective.required != 1
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
                || objective.actor_instance_id.is_some()
                || objective.actor_kind_id.is_some()
                || objective.spawn_count.is_some()
            {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::KillActor => {
            let (Some(instance_id), Some(kind_id)) =
                (&objective.actor_instance_id, &objective.actor_kind_id)
            else {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            };
            validate_id(instance_id)?;
            if !instance_ids.insert(instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(instance_id.clone()));
            }
            require_actor_role(actor_roles, kind_id, ActorRole::Monster, owner_id)?;
            if objective.required != 1
                || objective.spawn_count.is_some()
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
            {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::KillActorKind => {
            let Some(kind_id) = &objective.actor_kind_id else {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            };
            if objective.required < 2
                || objective.actor_instance_id.is_some()
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
                || objective
                    .spawn_count
                    .is_some_and(|count| count == 0 || count > objective.required)
            {
                return Err(ContentError::InvalidProceduralFloor(owner_id.to_owned()));
            }
            require_actor_role(actor_roles, kind_id, ActorRole::Monster, owner_id)?;
        }
    }
    Ok(())
}

fn validate_world(
    world: &mut WorldDefinition,
    refs: &WorldValidationRefs<'_>,
) -> Result<(), ContentError> {
    let WorldValidationRefs {
        terrain_ids,
        terrain_walkability,
        terrain_tags,
        terrain_open_targets,
        terrain_traps,
        actor_roles,
        actor_levels,
        item_limits,
        affix_ids,
        encounter_tables,
        loot_table_ids,
        theme_tables,
        region_tables,
        terrain_feature_tables,
        vaults,
    } = refs;
    if world.width < 3 || world.height < 3 || world.width > 512 || world.height > 512 {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    validate_definition_id(&world.initial_floor_id, "floor")?;
    let mut procedural_actor_ids = BTreeSet::new();
    let mut procedural_connection_ids = BTreeSet::new();
    world.procedural_floors.sort_by_key(|floor| floor.depth);
    let floor_ids = world
        .procedural_floors
        .iter()
        .map(|floor| floor.id.clone())
        .collect::<BTreeSet<_>>();
    if world.procedural_floors.is_empty()
        || floor_ids.len() != world.procedural_floors.len()
        || !world
            .procedural_floors
            .iter()
            .any(|floor| floor.return_floor_id == world.initial_floor_id)
    {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    for procedural in &mut world.procedural_floors {
        validate_definition_id(&procedural.id, "floor")?;
        validate_message_key(&procedural.name_key)?;
        let layout_mode = procedural
            .layout
            .as_ref()
            .map_or(ProceduralLayoutMode::Rooms, |layout| layout.mode);
        let maze_only = layout_mode == ProceduralLayoutMode::MazeOnly;
        procedural
            .connections
            .sort_by(|left, right| left.id.cmp(&right.id));
        if procedural.connections.len() > 16
            || (procedural.connections.is_empty() && procedural.entry_connection_id.is_some())
            || procedural.entry_connection_id.as_ref().is_some_and(|id| {
                !procedural
                    .connections
                    .iter()
                    .any(|connection| connection.id == *id)
            })
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        for connection in &procedural.connections {
            validate_definition_id(&connection.id, "connection")?;
            if !procedural_connection_ids.insert(connection.id.clone()) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            require_reference(terrain_ids, &connection.terrain_id, &procedural.id)?;
            let tags = terrain_tags
                .get(&connection.terrain_id)
                .expect("validated connection terrain must remain available");
            if !terrain_walkability
                .get(&connection.terrain_id)
                .copied()
                .unwrap_or(false)
                || (matches!(connection.kind, FloorConnectionKind::Shaft) != tags.contains("shaft"))
                || (!tags.contains("stairs-up") && !tags.contains("stairs-down"))
                || (tags.contains("stairs-up") && tags.contains("stairs-down"))
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some(target_connection_id) = &connection.target_connection_id {
                validate_definition_id(target_connection_id, "connection")?;
            }
        }
        if procedural.id == world.initial_floor_id
            || procedural.width != world.width
            || procedural.height != world.height
            || (procedural.return_floor_id != world.initial_floor_id
                && !floor_ids.contains(&procedural.return_floor_id))
            || procedural
                .next_floor_id
                .as_ref()
                .is_some_and(|id| !floor_ids.contains(id))
            || procedural.next_floor_id.is_some() != procedural.down_stair_terrain_id.is_some()
            || (procedural.lifecycle == FloorLifecycle::OneShot
                && (procedural.return_floor_id != world.initial_floor_id
                    || procedural.dungeon_id.is_some()
                    || procedural.final_floor
                    || procedural.guardian.is_some()
                    || procedural.entry_terrain_id.is_none()
                    || procedural.completed_entry_terrain_id.is_none()
                    || procedural.failed_entry_terrain_id.is_none()
                    || procedural.abandoned_entry_terrain_id.is_none()
                    || procedural.next_floor_id.is_some()))
            || (procedural.lifecycle == FloorLifecycle::Dungeon
                && (procedural.dungeon_id.is_none()
                    || procedural.completed_entry_terrain_id.is_some()
                    || procedural.failed_entry_terrain_id.is_some()
                    || procedural.abandoned_entry_terrain_id.is_some()
                    || !procedural.allow_early_task_exit
                    || procedural.retakeable
                    || procedural.max_retakes.is_some()
                    || procedural.retake_floor_policy != RetakeFloorPolicy::PreserveFloor
                    || procedural.task_id.is_some()
                    || procedural.task_objective.is_some()
                    || !procedural.task_stages.is_empty()
                    || procedural.task_reward.is_some()))
        {
            return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
        }
        if let Some(dungeon_id) = &procedural.dungeon_id {
            validate_definition_id(dungeon_id, "dungeon")?;
        }
        if let Some(task_id) = &procedural.task_id {
            validate_definition_id(task_id, "task")?;
        }
        if (!procedural.retakeable
            && (procedural.max_retakes.is_some()
                || procedural.retake_floor_policy != RetakeFloorPolicy::PreserveFloor))
            || procedural
                .max_retakes
                .is_some_and(|maximum| maximum == 0 || maximum > 16)
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(theme_id) = &procedural.theme_id {
            validate_definition_id(theme_id, "theme")?;
        }
        if procedural.encounter_table_id.is_some() && !procedural.actor_spawns.is_empty()
            || procedural.loot_table_id.is_some() && !procedural.loot_spawns.is_empty()
            || procedural.theme_table_id.is_some()
                && (procedural.theme_id.is_some() || procedural.vault_id.is_some())
            || procedural.region_table_id.is_some()
                && (procedural.encounter_table_id.is_some()
                    || procedural.loot_table_id.is_some()
                    || procedural.theme_table_id.is_some()
                    || procedural.theme_id.is_some()
                    || procedural.vault_id.is_some()
                    || procedural.terrain_feature_table_id.is_some()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty()
                    || procedural.nest.is_some()
                    || procedural.guardian.is_some()
                    || procedural.final_floor
                    || maze_only
                    || !procedural.connections.is_empty()
                    || procedural.generation_budget.is_none()
                    || procedural.layout.as_ref().is_some_and(|layout| {
                        layout.cavern.is_some()
                            || layout.lake.is_some()
                            || layout.river.is_some()
                            || layout.maze.is_some()
                            || layout.destroyed.is_some()
                            || !layout.streamers.is_empty()
                            || layout.pit.is_some()
                    }))
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        let eligible_encounter_entries = if let Some(table_id) = &procedural.encounter_table_id {
            let Some(table) = encounter_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            let entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= procedural.depth
                        && procedural.depth <= entry.max_depth
                        && actor_levels
                            .get(&entry.actor_kind_id)
                            .is_some_and(|level| *level <= u32::from(procedural.depth))
                })
                .collect::<Vec<_>>();
            if entries.is_empty() {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            entries
        } else {
            Vec::new()
        };
        if let Some(table_id) = &procedural.loot_table_id {
            require_reference(loot_table_ids, table_id, &procedural.id)?;
        }
        let eligible_theme_entries = if let Some(table_id) = &procedural.theme_table_id {
            let Some(table) = theme_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            let entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            if entries.is_empty() {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            entries
        } else {
            Vec::new()
        };
        let eligible_region_entries = if let Some(table_id) = &procedural.region_table_id {
            let Some(table) = region_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            let entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            if entries.len() < 2
                || entries.iter().any(|entry| {
                    let theme_is_valid = theme_tables
                        .get(&entry.theme_table_id)
                        .and_then(|table| {
                            table.entries.iter().find(|theme| {
                                theme.theme_id == entry.theme_id
                                    && theme.min_depth <= procedural.depth
                                    && procedural.depth <= theme.max_depth
                            })
                        })
                        .is_some_and(|theme| {
                            !theme.vault_candidates.iter().any(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                            })
                        });
                    let encounter_is_valid = encounter_tables
                        .get(&entry.encounter_table_id)
                        .is_some_and(|table| {
                            table.entries.iter().any(|candidate| {
                                candidate.group.is_none()
                                    && candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                                    && actor_levels
                                        .get(&candidate.actor_kind_id)
                                        .is_some_and(|level| *level <= u32::from(procedural.depth))
                            }) && table
                                .entries
                                .iter()
                                .all(|candidate| candidate.group.is_none())
                        });
                    !theme_is_valid || !encounter_is_valid
                })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            entries
        } else {
            Vec::new()
        };
        let eligible_terrain_feature_entries =
            if let Some(table_id) = &procedural.terrain_feature_table_id {
                let Some(table) = terrain_feature_tables.get(table_id) else {
                    return Err(ContentError::DanglingReference {
                        owner: procedural.id.clone(),
                        target: table_id.clone(),
                    });
                };
                let entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                    })
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                entries
            } else {
                Vec::new()
            };
        for entry in &eligible_theme_entries {
            for candidate in entry.vault_candidates.iter().filter(|candidate| {
                candidate.min_depth <= procedural.depth && procedural.depth <= candidate.max_depth
            }) {
                let vault = vaults
                    .get(&candidate.vault_id)
                    .expect("validated theme vault must remain available");
                if vault.encounter_groups.iter().any(|group| {
                    !group.entries.iter().any(|actor| {
                        actor.min_depth <= procedural.depth
                            && procedural.depth <= actor.max_depth
                            && actor_levels
                                .get(&actor.actor_kind_id)
                                .is_some_and(|level| *level <= u32::from(procedural.depth))
                    })
                }) {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
        }
        if let Some(nest) = &procedural.nest
            && (procedural.encounter_table_id.is_none()
                || procedural.vault_id.is_some()
                || maze_only
                || !matches!(nest.room_id.as_str(), "entry" | "remote")
                || !(2..=16).contains(&nest.spawn_count)
                || eligible_theme_entries.iter().any(|entry| {
                    entry.vault_candidates.iter().any(|candidate| {
                        candidate.min_depth <= procedural.depth
                            && procedural.depth <= candidate.max_depth
                    })
                }))
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(budget) = &procedural.generation_budget {
            let reserved_actor_slots = usize::from(u8::from(procedural.guardian.is_some()))
                + procedural
                    .nest
                    .as_ref()
                    .map_or(0, |nest| usize::from(nest.spawn_count))
                + budget.pit_actor_slots.map_or(0, usize::from);
            let pit_budget = match (
                procedural
                    .layout
                    .as_ref()
                    .and_then(|layout| layout.pit.as_ref())
                    .cloned(),
                budget.pit_placements,
                budget.pit_actor_slots,
            ) {
                (None, None, None) => None,
                (Some(pit), Some(placements), Some(actor_slots)) => {
                    Some((pit, placements, actor_slots))
                }
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let room_budget = match (
                procedural.layout.as_ref(),
                budget.room_placements,
                budget.room_area_tiles,
            ) {
                (None, None, None) => None,
                (Some(layout), None, None)
                    if layout.mode == ProceduralLayoutMode::MazeOnly && layout.rooms.is_none() =>
                {
                    None
                }
                (Some(layout), Some(placements), Some(area_tiles))
                    if layout.mode == ProceduralLayoutMode::Rooms && layout.rooms.is_some() =>
                {
                    Some((placements, area_tiles))
                }
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let spatial_vault_budget = match (budget.vault_placements, budget.vault_area_tiles) {
                (None, None) => None,
                (Some(placements), Some(area_tiles)) => Some((placements, area_tiles)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let group_budget = match (budget.group_placements, budget.group_actor_slots) {
                (None, None) => None,
                (Some(placements), Some(actor_slots)) => Some((placements, actor_slots)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let feature_budget = match (
                procedural.terrain_feature_table_id.as_ref(),
                budget.feature_placements,
            ) {
                (None, None) => None,
                (Some(table_id), Some(placements)) => Some((table_id, placements)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let region_budget = match (
                procedural.region_table_id.as_ref(),
                budget.region_placements,
            ) {
                (None, None) => None,
                (Some(_), Some(placements)) => Some(placements),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            if procedural.lifecycle != FloorLifecycle::Dungeon
                || (procedural.region_table_id.is_none()
                    && (procedural.encounter_table_id.is_none()
                        || procedural.loot_table_id.is_none()))
                || !(1..=128).contains(&budget.actor_slots)
                || !(1..=8).contains(&budget.loot_placements)
                || reserved_actor_slots >= usize::from(budget.actor_slots)
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some(placements) = region_budget {
                let room_count = budget.room_placements.unwrap_or(2);
                if !(2..=4).contains(&placements)
                    || placements > room_count
                    || usize::from(placements) > eligible_region_entries.len()
                    || budget.actor_slots < placements
                    || budget.loot_placements < placements
                    || spatial_vault_budget.is_some()
                    || group_budget.is_some()
                    || feature_budget.is_some()
                    || budget.cavern_area_tiles.is_some()
                    || budget.lake_area_tiles.is_some()
                    || budget.lake_deep_area_tiles.is_some()
                    || budget.river_area_tiles.is_some()
                    || budget.maze_floor_tiles.is_some()
                    || budget.destruction_centers.is_some()
                    || budget.destroyed_area_tiles.is_some()
                    || budget.streamer_placements.is_some()
                    || budget.streamer_area_tiles.is_some()
                    || budget.pit_placements.is_some()
                    || budget.pit_actor_slots.is_some()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            if !maze_only
                && room_budget.is_none()
                && (budget.cavern_area_tiles.is_some()
                    || budget.lake_area_tiles.is_some()
                    || budget.lake_deep_area_tiles.is_some()
                    || budget.river_area_tiles.is_some()
                    || budget.maze_floor_tiles.is_some()
                    || budget.destruction_centers.is_some()
                    || budget.destroyed_area_tiles.is_some()
                    || budget.streamer_placements.is_some()
                    || budget.streamer_area_tiles.is_some())
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if maze_only {
                let layout = procedural
                    .layout
                    .as_mut()
                    .expect("maze-only mode requires a layout");
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                if layout.rooms.is_some()
                    || layout.cavern.is_some()
                    || layout.lake.is_some()
                    || layout.river.is_some()
                    || layout.destroyed.is_some()
                    || layout.pit.is_some()
                    || budget.cavern_area_tiles.is_some()
                    || budget.lake_area_tiles.is_some()
                    || budget.lake_deep_area_tiles.is_some()
                    || budget.river_area_tiles.is_some()
                    || budget.destruction_centers.is_some()
                    || budget.destroyed_area_tiles.is_some()
                    || budget.pit_placements.is_some()
                    || budget.pit_actor_slots.is_some()
                    || spatial_vault_budget.is_some()
                    || group_budget.is_some()
                    || feature_budget.is_some()
                    || procedural.vault_id.is_some()
                    || procedural.nest.is_some()
                    || procedural.guardian.is_some()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.maze, budget.maze_floor_tiles) {
                    (Some(maze), Some(floor_tiles)) => {
                        let vertices =
                            u32::from(maze.width.div_ceil(2)) * u32::from(maze.height.div_ceil(2));
                        let expected_floor_tiles = vertices.saturating_mul(2).saturating_sub(1);
                        if !(9..=procedural.width.saturating_sub(2)).contains(&maze.width)
                            || !(9..=procedural.height.saturating_sub(2)).contains(&maze.height)
                            || maze.width % 2 == 0
                            || maze.height % 2 == 0
                            || floor_tiles != expected_floor_tiles
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
                }
                match (
                    layout.streamers.is_empty(),
                    budget.streamer_placements,
                    budget.streamer_area_tiles,
                ) {
                    (true, None, None) => {}
                    (false, Some(placements), Some(area_tiles)) => {
                        layout
                            .streamers
                            .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
                        let terrain_count = layout
                            .streamers
                            .iter()
                            .map(|candidate| candidate.terrain_id.as_str())
                            .collect::<BTreeSet<_>>()
                            .len();
                        for candidate in &layout.streamers {
                            require_reference(terrain_ids, &candidate.terrain_id, &procedural.id)?;
                        }
                        if layout.streamers.len() > 4
                            || terrain_count != layout.streamers.len()
                            || layout.streamers.iter().any(|candidate| {
                                !(1..=1_000_000).contains(&candidate.weight)
                                    || terrain_walkability.get(&candidate.terrain_id)
                                        != Some(&false)
                                    || candidate.terrain_id == procedural.wall_terrain_id
                                    || candidate.terrain_id == procedural.floor_terrain_id
                                    || eligible_theme_entries
                                        .iter()
                                        .any(|entry| entry.floor_terrain_id == candidate.terrain_id)
                            })
                            || !(1..=4).contains(&placements)
                            || !(u32::from(placements) * 4..=interior_area.saturating_div(4))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
                }
            }
            if let Some((placements, area_tiles)) = room_budget {
                let layout = procedural
                    .layout
                    .as_mut()
                    .expect("rooms mode requires a layout");
                let geometry = layout
                    .rooms
                    .as_mut()
                    .expect("rooms mode requires room geometry");
                geometry.shapes.sort_by_key(|candidate| candidate.shape);
                let shape_count = geometry
                    .shapes
                    .iter()
                    .map(|candidate| candidate.shape)
                    .collect::<BTreeSet<_>>()
                    .len();
                let columns = if placements <= 4 { 2 } else { 3 };
                let rows = placements.div_ceil(columns);
                let minimum_cell_width = procedural.width.saturating_sub(2) / columns;
                let minimum_cell_height = procedural.height.saturating_sub(2) / rows;
                let minimum_room_area = geometry
                    .shapes
                    .iter()
                    .map(|candidate| match candidate.shape {
                        ProceduralRoomShape::Rectangle => {
                            u32::from(geometry.min_width) * u32::from(geometry.min_height)
                        }
                        ProceduralRoomShape::Cross => {
                            u32::from(geometry.min_width) + u32::from(geometry.min_height) - 1
                        }
                    })
                    .min()
                    .unwrap_or(0);
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                if !(2..=6).contains(&placements)
                    || !(5..=9).contains(&geometry.min_width)
                    || !(geometry.min_width..=9).contains(&geometry.max_width)
                    || !(5..=9).contains(&geometry.min_height)
                    || !(geometry.min_height..=9).contains(&geometry.max_height)
                    || geometry.min_width > minimum_cell_width
                    || geometry.min_height > minimum_cell_height
                    || geometry.shapes.is_empty()
                    || geometry.shapes.len() > 2
                    || shape_count != geometry.shapes.len()
                    || geometry
                        .shapes
                        .iter()
                        .any(|candidate| !(1..=1_000_000).contains(&candidate.weight))
                    || area_tiles > interior_area
                    || u32::from(placements) * minimum_room_area > area_tiles
                    || procedural.vault_id.is_some()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.cavern, budget.cavern_area_tiles) {
                    (None, None) => {}
                    (Some(cavern), Some(cavern_area_tiles)) => {
                        require_reference(terrain_ids, &cavern.terrain_id, &procedural.id)?;
                        if terrain_walkability.get(&cavern.terrain_id) != Some(&true)
                            || cavern.terrain_id == procedural.floor_terrain_id
                            || cavern.terrain_id == procedural.wall_terrain_id
                            || eligible_theme_entries
                                .iter()
                                .any(|entry| entry.floor_terrain_id == cavern.terrain_id)
                            || !(16..=interior_area).contains(&cavern_area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                let validate_hydrology_terrain =
                    |deep_terrain_id: &str, shallow_terrain_id: &str| {
                        require_reference(terrain_ids, deep_terrain_id, &procedural.id)?;
                        require_reference(terrain_ids, shallow_terrain_id, &procedural.id)?;
                        if deep_terrain_id == shallow_terrain_id
                            || terrain_walkability.get(deep_terrain_id) != Some(&false)
                            || terrain_walkability.get(shallow_terrain_id) != Some(&true)
                            || [deep_terrain_id, shallow_terrain_id]
                                .contains(&procedural.floor_terrain_id.as_str())
                            || [deep_terrain_id, shallow_terrain_id]
                                .contains(&procedural.wall_terrain_id.as_str())
                            || layout.cavern.as_ref().is_some_and(|cavern| {
                                [deep_terrain_id, shallow_terrain_id]
                                    .contains(&cavern.terrain_id.as_str())
                            })
                            || eligible_theme_entries.iter().any(|entry| {
                                [deep_terrain_id, shallow_terrain_id]
                                    .contains(&entry.floor_terrain_id.as_str())
                            })
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                        Ok(())
                    };
                match (
                    &layout.lake,
                    budget.lake_area_tiles,
                    budget.lake_deep_area_tiles,
                ) {
                    (None, None, None) => {}
                    (Some(lake), Some(area_tiles), Some(deep_area_tiles)) => {
                        validate_hydrology_terrain(
                            &lake.deep_terrain_id,
                            &lake.shallow_terrain_id,
                        )?;
                        if !(24..=interior_area).contains(&area_tiles)
                            || deep_area_tiles < 4
                            || deep_area_tiles.saturating_add(8) > area_tiles
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (&layout.river, budget.river_area_tiles) {
                    (None, None) => {}
                    (Some(river), Some(area_tiles)) => {
                        validate_hydrology_terrain(
                            &river.deep_terrain_id,
                            &river.shallow_terrain_id,
                        )?;
                        let center_x = procedural.width / 2;
                        let center_y = procedural.height / 2;
                        let maximum_centerline_tiles = u32::from(
                            center_x
                                .saturating_sub(1)
                                .max(procedural.width.saturating_sub(2 + center_x))
                                + center_y
                                    .saturating_sub(1)
                                    .max(procedural.height.saturating_sub(2 + center_y))
                                + 1,
                        );
                        if !(maximum_centerline_tiles..=interior_area).contains(&area_tiles) {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                if let (Some(lake), Some(river)) = (&layout.lake, &layout.river)
                    && (lake.deep_terrain_id != river.deep_terrain_id
                        || lake.shallow_terrain_id != river.shallow_terrain_id)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.maze, budget.maze_floor_tiles) {
                    (None, None) => {}
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (
                    &layout.destroyed,
                    budget.destruction_centers,
                    budget.destroyed_area_tiles,
                ) {
                    (None, None, None) => {}
                    (Some(destroyed), Some(centers), Some(area_tiles)) => {
                        require_reference(terrain_ids, &destroyed.terrain_id, &procedural.id)?;
                        if terrain_walkability.get(&destroyed.terrain_id) != Some(&false)
                            || destroyed.terrain_id == procedural.wall_terrain_id
                            || destroyed.terrain_id == procedural.floor_terrain_id
                            || layout
                                .cavern
                                .as_ref()
                                .is_some_and(|cavern| cavern.terrain_id == destroyed.terrain_id)
                            || layout.lake.as_ref().is_some_and(|lake| {
                                [
                                    lake.deep_terrain_id.as_str(),
                                    lake.shallow_terrain_id.as_str(),
                                ]
                                .contains(&destroyed.terrain_id.as_str())
                            })
                            || eligible_theme_entries
                                .iter()
                                .any(|entry| entry.floor_terrain_id == destroyed.terrain_id)
                            || !(1..=4).contains(&centers)
                            || !(u32::from(centers) * 8..=interior_area.saturating_div(2))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (
                    layout.streamers.is_empty(),
                    budget.streamer_placements,
                    budget.streamer_area_tiles,
                ) {
                    (true, None, None) => {}
                    (false, Some(placements), Some(area_tiles)) => {
                        layout
                            .streamers
                            .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
                        let terrain_count = layout
                            .streamers
                            .iter()
                            .map(|candidate| candidate.terrain_id.as_str())
                            .collect::<BTreeSet<_>>()
                            .len();
                        for candidate in &layout.streamers {
                            require_reference(terrain_ids, &candidate.terrain_id, &procedural.id)?;
                        }
                        if layout.streamers.len() > 4
                            || terrain_count != layout.streamers.len()
                            || layout.streamers.iter().any(|candidate| {
                                !(1..=1_000_000).contains(&candidate.weight)
                                    || terrain_walkability.get(&candidate.terrain_id)
                                        != Some(&false)
                                    || candidate.terrain_id == procedural.wall_terrain_id
                                    || candidate.terrain_id == procedural.floor_terrain_id
                                    || layout.destroyed.as_ref().is_some_and(|destroyed| {
                                        destroyed.terrain_id == candidate.terrain_id
                                    })
                                    || layout.cavern.as_ref().is_some_and(|cavern| {
                                        cavern.terrain_id == candidate.terrain_id
                                    })
                                    || layout.lake.as_ref().is_some_and(|lake| {
                                        [
                                            lake.deep_terrain_id.as_str(),
                                            lake.shallow_terrain_id.as_str(),
                                        ]
                                        .contains(&candidate.terrain_id.as_str())
                                    })
                                    || eligible_theme_entries
                                        .iter()
                                        .any(|entry| entry.floor_terrain_id == candidate.terrain_id)
                            })
                            || !(1..=4).contains(&placements)
                            || !(u32::from(placements) * 4..=interior_area.saturating_div(4))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                if let Some((pit, placements, actor_slots)) = &pit_budget {
                    let Some(table) = encounter_tables.get(&pit.encounter_table_id) else {
                        return Err(ContentError::DanglingReference {
                            owner: procedural.id.clone(),
                            target: pit.encounter_table_id.clone(),
                        });
                    };
                    let eligible_pit_entries = table
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.min_depth <= procedural.depth
                                && procedural.depth <= entry.max_depth
                                && actor_levels
                                    .get(&entry.actor_kind_id)
                                    .is_some_and(|level| *level <= u32::from(procedural.depth))
                        })
                        .count();
                    let total_width = pit.inner_width.saturating_add(6);
                    let total_height = pit.inner_height.saturating_add(6);
                    if *placements != 1
                        || *actor_slots != pit.inner_width.saturating_mul(pit.inner_height)
                        || !(5..=15).contains(&pit.inner_width)
                        || !(5..=7).contains(&pit.inner_height)
                        || pit.inner_width % 2 == 0
                        || pit.inner_height % 2 == 0
                        || !(2..=10).contains(&pit.roster_size)
                        || eligible_pit_entries < 2
                        || total_width > procedural.width.saturating_sub(2)
                        || total_height > procedural.height.saturating_sub(2)
                        || procedural.nest.is_some()
                        || spatial_vault_budget.is_some()
                        || group_budget.is_some()
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
            }
            if let Some((placements, area_tiles)) = spatial_vault_budget {
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                if !(1..=4).contains(&placements)
                    || !(4..=512).contains(&area_tiles)
                    || area_tiles > interior_area
                    || procedural.theme_table_id.is_none()
                    || procedural.vault_id.is_some()
                    || procedural.nest.is_some()
                    || eligible_theme_entries.is_empty()
                    || eligible_theme_entries.iter().any(|entry| {
                        entry
                            .vault_candidates
                            .iter()
                            .filter(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                            })
                            .count()
                            < usize::from(placements)
                    })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            if let Some((placements, group_actor_slots)) = group_budget {
                let grouped_entries = eligible_encounter_entries
                    .iter()
                    .filter(|entry| entry.group.is_some())
                    .copied()
                    .collect::<Vec<_>>();
                let plain_entries = eligible_encounter_entries
                    .iter()
                    .filter(|entry| entry.group.is_none())
                    .copied()
                    .collect::<Vec<_>>();
                let maximum_minimum_companions = grouped_entries
                    .iter()
                    .filter_map(|entry| entry.group.as_ref())
                    .map(EncounterGroupDefinition::min_companion_count)
                    .max()
                    .unwrap_or(0);
                let required_companion_slots =
                    usize::from(placements) * usize::from(maximum_minimum_companions);
                let required_actor_slots =
                    reserved_actor_slots + usize::from(placements) + required_companion_slots + 1;
                let encounter_rolls = procedural
                    .encounter_table_id
                    .as_ref()
                    .and_then(|table_id| encounter_tables.get(table_id))
                    .map_or(0, |table| table.rolls);
                if !(1..=4).contains(&placements)
                    || !(1..=14).contains(&group_actor_slots)
                    || placements >= encounter_rolls
                    || grouped_entries.is_empty()
                    || plain_entries.is_empty()
                    || procedural.nest.is_some()
                    || spatial_vault_budget.is_some()
                    || required_companion_slots > usize::from(group_actor_slots)
                    || required_actor_slots > usize::from(budget.actor_slots)
                    || grouped_entries.iter().any(|entry| {
                        entry
                            .group
                            .as_ref()
                            .is_some_and(|group| group.min_companion_count() > group_actor_slots)
                    })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            } else if eligible_encounter_entries
                .iter()
                .any(|entry| entry.group.is_some())
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some((table_id, placements)) = feature_budget {
                let table = terrain_feature_tables
                    .get(table_id)
                    .expect("validated terrain feature table must remain available");
                if !(1..=8).contains(&placements)
                    || placements > table.rolls
                    || eligible_terrain_feature_entries.is_empty()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            for entry in &eligible_theme_entries {
                for candidate in entry.vault_candidates.iter().filter(|candidate| {
                    candidate.min_depth <= procedural.depth
                        && procedural.depth <= candidate.max_depth
                }) {
                    let vault = vaults
                        .get(&candidate.vault_id)
                        .expect("validated theme vault must remain available");
                    let vault_actor_slots = vault
                        .encounter_groups
                        .iter()
                        .map(|group| group.member_positions.len())
                        .sum::<usize>();
                    if reserved_actor_slots + vault_actor_slots >= usize::from(budget.actor_slots)
                        || vault.loot_spawns.len() >= usize::from(budget.loot_placements)
                        || spatial_vault_budget.is_some_and(|(_, area_tiles)| {
                            u32::from(vault.width) * u32::from(vault.height) > area_tiles
                        })
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
            }
        } else if eligible_encounter_entries
            .iter()
            .any(|entry| entry.group.is_some())
            || procedural.terrain_feature_table_id.is_some()
            || procedural.layout.is_some()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(vault_id) = &procedural.vault_id {
            let Some(vault) = vaults.get(vault_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: vault_id.clone(),
                });
            };
            if procedural.theme_id.as_ref() != Some(&vault.theme_id)
                || procedural
                    .actor_spawns
                    .iter()
                    .any(|spawn| spawn.room_id == "remote")
                || procedural
                    .loot_spawns
                    .iter()
                    .any(|spawn| spawn.room_id == "remote")
                || vault.encounter_groups.iter().any(|group| {
                    !group.entries.iter().any(|entry| {
                        entry.min_depth <= procedural.depth
                            && procedural.depth <= entry.max_depth
                            && actor_levels
                                .get(&entry.actor_kind_id)
                                .is_some_and(|level| *level <= u32::from(procedural.depth))
                    })
                })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        for terrain_id in [
            Some(&procedural.wall_terrain_id),
            Some(&procedural.floor_terrain_id),
            Some(&procedural.up_stair_terrain_id),
            Some(&procedural.closed_door_terrain_id),
            Some(&procedural.trap_terrain_id),
            procedural.down_stair_terrain_id.as_ref(),
            procedural.entry_terrain_id.as_ref(),
            procedural.completed_entry_terrain_id.as_ref(),
            procedural.failed_entry_terrain_id.as_ref(),
            procedural.abandoned_entry_terrain_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            require_reference(terrain_ids, terrain_id, &procedural.id)?;
        }
        if let Some(objective) = &procedural.task_objective {
            if objective.floor_id.is_some() {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            validate_task_objective(
                &procedural.id,
                objective,
                &floor_ids,
                actor_roles,
                item_limits,
                &mut procedural_actor_ids,
            )?;
        }
        for stage in &procedural.task_stages {
            validate_task_objective(
                &procedural.id,
                stage,
                &floor_ids,
                actor_roles,
                item_limits,
                &mut procedural_actor_ids,
            )?;
        }
        if let Some(guardian) = &procedural.guardian {
            validate_id(&guardian.instance_id)?;
            if !procedural_actor_ids.insert(guardian.instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(
                    guardian.instance_id.clone(),
                ));
            }
            require_actor_role(
                actor_roles,
                &guardian.actor_kind_id,
                ActorRole::Monster,
                &procedural.id,
            )?;
            if actor_levels
                .get(&guardian.actor_kind_id)
                .is_none_or(|level| *level > u32::from(procedural.depth))
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        if let Some(reward) = &procedural.task_reward {
            validate_id(&reward.item_instance_id)?;
            if !procedural_actor_ids.insert(reward.item_instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(
                    reward.item_instance_id.clone(),
                ));
            }
            let (max_stack, _) = item_limits.get(&reward.item_kind_id).ok_or_else(|| {
                ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: reward.item_kind_id.clone(),
                }
            })?;
            if reward.quantity == 0 || reward.quantity > *max_stack {
                return Err(ContentError::InvalidItemQuantity(
                    reward.item_instance_id.clone(),
                ));
            }
        }
        if terrain_walkability
            .get(&procedural.wall_terrain_id)
            .copied()
            .unwrap_or(true)
            || !terrain_walkability
                .get(&procedural.floor_terrain_id)
                .copied()
                .unwrap_or(false)
            || !terrain_walkability
                .get(&procedural.up_stair_terrain_id)
                .copied()
                .unwrap_or(false)
            || procedural
                .down_stair_terrain_id
                .as_ref()
                .is_some_and(|id| !terrain_walkability.get(id).copied().unwrap_or(false))
            || terrain_walkability
                .get(&procedural.closed_door_terrain_id)
                .copied()
                .unwrap_or(true)
            || !terrain_open_targets.contains_key(&procedural.closed_door_terrain_id)
            || !terrain_traps.contains(&procedural.trap_terrain_id)
            || procedural.depth == 0
            || procedural.depth > 1_000
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        procedural
            .actor_spawns
            .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        let mut room_spawn_counts = BTreeMap::new();
        for spawn in &mut procedural.actor_spawns {
            validate_id(&spawn.instance_id)?;
            if !procedural_actor_ids.insert(spawn.instance_id.clone())
                || !matches!(spawn.room_id.as_str(), "entry" | "remote")
                || spawn.actor_kind_ids.is_empty()
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            *room_spawn_counts.entry(spawn.room_id.clone()).or_insert(0) += 1;
            spawn.actor_kind_ids.sort();
            for actor_kind_id in &spawn.actor_kind_ids {
                require_actor_role(
                    actor_roles,
                    actor_kind_id,
                    ActorRole::Monster,
                    &procedural.id,
                )?;
            }
            if !spawn.actor_kind_ids.iter().any(|actor_kind_id| {
                actor_levels
                    .get(actor_kind_id)
                    .is_some_and(|level| *level <= u32::from(procedural.depth))
            }) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        procedural
            .loot_spawns
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut loot_ids = BTreeSet::new();
        for spawn in &procedural.loot_spawns {
            validate_id(&spawn.id)?;
            if !loot_ids.insert(spawn.id.clone()) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            *room_spawn_counts.entry(spawn.room_id.clone()).or_insert(0) += 1;
            require_reference(loot_table_ids, &spawn.loot_table_id, &procedural.id)?;
        }
    }
    for procedural in &world.procedural_floors {
        if procedural.connections.is_empty() {
            continue;
        }
        if procedural.return_floor_id == world.initial_floor_id
            && procedural
                .entry_connection_id
                .as_ref()
                .and_then(|id| {
                    procedural
                        .connections
                        .iter()
                        .find(|connection| connection.id == *id)
                })
                .is_none_or(|connection| connection.target_floor_id != world.initial_floor_id)
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if procedural.return_floor_id != world.initial_floor_id
            && procedural.entry_connection_id.is_some()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        for connection in &procedural.connections {
            if !floor_ids.contains(&connection.target_floor_id)
                && connection.target_floor_id != world.initial_floor_id
            {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: connection.target_floor_id.clone(),
                });
            }
            if connection.target_floor_id == world.initial_floor_id {
                if connection.target_connection_id.is_some()
                    || !matches!(connection.kind, FloorConnectionKind::Stairs)
                    || !terrain_tags
                        .get(&connection.terrain_id)
                        .is_some_and(|tags| tags.contains("stairs-up"))
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                continue;
            }
            let target = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == connection.target_floor_id)
                .expect("validated connection target must remain available");
            let Some(target_connection_id) = connection.target_connection_id.as_ref() else {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            };
            let Some(target_connection) = target
                .connections
                .iter()
                .find(|candidate| candidate.id == *target_connection_id)
            else {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            };
            let depth_delta = target.depth.abs_diff(procedural.depth);
            if target_connection.target_floor_id != procedural.id
                || target_connection.target_connection_id.as_deref() != Some(connection.id.as_str())
                || target_connection.kind != connection.kind
                || (matches!(connection.kind, FloorConnectionKind::Stairs) && depth_delta != 1)
                || (matches!(connection.kind, FloorConnectionKind::Shaft) && depth_delta != 2)
                || (target.lifecycle != procedural.lifecycle)
                || (target.dungeon_id != procedural.dungeon_id)
                || !terrain_tags
                    .get(&connection.terrain_id)
                    .is_some_and(|tags| {
                        if target.depth > procedural.depth {
                            tags.contains("stairs-down")
                        } else {
                            tags.contains("stairs-up")
                        }
                    })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
    }
    for procedural in world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
    {
        let task_id = procedural.task_id.as_deref().unwrap_or(&procedural.id);
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| {
                floor.lifecycle == FloorLifecycle::OneShot
                    && floor.task_id.as_deref().unwrap_or(&floor.id) == task_id
            })
            .collect::<Vec<_>>();
        if members
            .iter()
            .filter(|floor| floor.task_reward.is_some())
            .count()
            != 1
            || members.iter().any(|floor| {
                floor.retakeable != procedural.retakeable
                    || floor.max_retakes != procedural.max_retakes
                    || floor.retake_floor_policy != procedural.retake_floor_policy
            })
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        let staged_definitions = members
            .iter()
            .filter(|floor| !floor.task_stages.is_empty())
            .collect::<Vec<_>>();
        if staged_definitions.is_empty() {
            let Some(objective) = procedural.task_objective.as_ref() else {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            };
            if members.iter().any(|floor| {
                let Some(other) = floor.task_objective.as_ref() else {
                    return true;
                };
                other.kind != objective.kind
                    || other.required != objective.required
                    || other.item_kind_id != objective.item_kind_id
                    || other.actor_kind_id != objective.actor_kind_id
            }) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        } else {
            if staged_definitions.len() != 1
                || !procedural.retakeable
                || members.iter().any(|floor| floor.task_objective.is_some())
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            let stages = &staged_definitions[0].task_stages;
            let member_ids = members
                .iter()
                .map(|floor| floor.id.as_str())
                .collect::<BTreeSet<_>>();
            let mut actionable_floor_ids = BTreeSet::new();
            if stages.len() < 2
                || stages.iter().any(|stage| {
                    stage
                        .floor_id
                        .as_deref()
                        .is_none_or(|floor_id| !member_ids.contains(floor_id))
                        || (stage.kind != TaskObjectiveKind::EnterFloor
                            && !actionable_floor_ids.insert(
                                stage
                                    .floor_id
                                    .as_deref()
                                    .expect("staged objective floor must be validated"),
                            ))
                })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
    }
    for procedural in &world.procedural_floors {
        if procedural.return_floor_id == world.initial_floor_id
            && procedural.entry_terrain_id.is_none()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(next_id) = &procedural.next_floor_id {
            let next = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == *next_id)
                .expect("validated next floor must remain available");
            if next.return_floor_id != procedural.id
                || next.depth != procedural.depth.saturating_add(1)
                || next.lifecycle != procedural.lifecycle
                || next.dungeon_id != procedural.dungeon_id
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
    }
    let dungeon_ids = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        .filter_map(|floor| floor.dungeon_id.as_deref())
        .collect::<BTreeSet<_>>();
    for dungeon_id in dungeon_ids {
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        let roots = members
            .iter()
            .filter(|floor| floor.return_floor_id == world.initial_floor_id)
            .copied()
            .collect::<Vec<_>>();
        let finals = members
            .iter()
            .filter(|floor| floor.final_floor)
            .copied()
            .collect::<Vec<_>>();
        if roots.len() != 1 || finals.len() != 1 || roots[0].depth != 1 {
            return Err(ContentError::InvalidProceduralFloor(members[0].id.clone()));
        }
        let mut seen = BTreeSet::new();
        let mut current = roots[0];
        loop {
            if !seen.insert(current.id.as_str())
                || current.final_floor != current.guardian.is_some()
                || (current.final_floor && current.next_floor_id.is_some())
                || (!current.final_floor && current.next_floor_id.is_none())
            {
                return Err(ContentError::InvalidProceduralFloor(current.id.clone()));
            }
            let Some(next_id) = current.next_floor_id.as_deref() else {
                break;
            };
            current = members
                .iter()
                .find(|floor| floor.id == next_id)
                .copied()
                .ok_or_else(|| ContentError::InvalidProceduralFloor(current.id.clone()))?;
        }
        if seen.len() != members.len() || current.id != finals[0].id {
            return Err(ContentError::InvalidProceduralFloor(current.id.clone()));
        }
    }
    let mut entry_terrain_ids = BTreeSet::new();
    for floor in world
        .procedural_floors
        .iter()
        .filter(|floor| floor.return_floor_id == world.initial_floor_id)
    {
        if !entry_terrain_ids.insert(floor.entry_terrain_id.as_deref()) {
            return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
        }
    }
    require_reference(terrain_ids, &world.fill_terrain_id, &world.id)?;
    require_reference(terrain_ids, &world.border_terrain_id, &world.id)?;
    require_actor_role(
        actor_roles,
        &world.player.kind_id,
        ActorRole::Player,
        &world.id,
    )?;
    validate_position(world.player.position, world.width, world.height, &world.id)?;
    validate_id(&world.player.instance_id)?;

    let mut instance_ids = BTreeSet::new();
    instance_ids.insert(world.player.instance_id.clone());
    let mut actor_positions = BTreeSet::new();
    actor_positions.insert(world.player.position);

    world
        .actors
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for actor in &world.actors {
        validate_id(&actor.instance_id)?;
        if !instance_ids.insert(actor.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(actor.instance_id.clone()));
        }
        require_actor_role(actor_roles, &actor.kind_id, ActorRole::Monster, &world.id)?;
        validate_position(actor.position, world.width, world.height, &world.id)?;
        if !actor_positions.insert(actor.position) {
            return Err(ContentError::DuplicateActorPosition(world.id.clone()));
        }
    }
    for actor_id in procedural_actor_ids {
        if !instance_ids.insert(actor_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(actor_id));
        }
    }

    world
        .items
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for item in &mut world.items {
        validate_id(&item.instance_id)?;
        if !instance_ids.insert(item.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(item.instance_id.clone()));
        }
        let (max_stack, equippable) =
            item_limits
                .get(&item.kind_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: world.id.clone(),
                    target: item.kind_id.clone(),
                })?;
        if item.quantity == 0 || item.quantity > *max_stack {
            return Err(ContentError::InvalidItemQuantity(item.instance_id.clone()));
        }
        item.affix_ids.sort();
        let mut seen_affixes = BTreeSet::new();
        if (item.quality != ItemQuality::Ordinary && (*max_stack != 1 || item.quantity != 1))
            || (!item.affix_ids.is_empty()
                && (*max_stack != 1
                    || !equippable
                    || item.quantity != 1
                    || item.quality == ItemQuality::Ordinary))
            || item.affix_ids.iter().any(|affix_id| {
                !affix_ids.contains(affix_id) || !seen_affixes.insert(affix_id.as_str())
            })
        {
            return Err(ContentError::InvalidItemAffixes(item.instance_id.clone()));
        }
        validate_position(item.position, world.width, world.height, &world.id)?;
    }

    world
        .terrain_overrides
        .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
    let mut override_terrain = BTreeMap::new();
    for terrain_override in &mut world.terrain_overrides {
        require_reference(terrain_ids, &terrain_override.terrain_id, &world.id)?;
        terrain_override.positions.sort();
        for position in &terrain_override.positions {
            validate_position(*position, world.width, world.height, &world.id)?;
            if position.x == 0
                || position.y == 0
                || position.x == world.width - 1
                || position.y == world.height - 1
                || override_terrain
                    .insert(*position, terrain_override.terrain_id.clone())
                    .is_some()
            {
                return Err(ContentError::InvalidTerrainOverride(world.id.clone()));
            }
        }
    }

    require_walkable_spawn(
        world,
        world.player.position,
        &override_terrain,
        terrain_walkability,
    )?;
    for actor in &world.actors {
        require_walkable_spawn(
            world,
            actor.position,
            &override_terrain,
            terrain_walkability,
        )?;
    }
    for item in &world.items {
        require_walkable_spawn(world, item.position, &override_terrain, terrain_walkability)?;
    }
    Ok(())
}

fn require_walkable_spawn(
    world: &WorldDefinition,
    position: ContentPosition,
    override_terrain: &BTreeMap<ContentPosition, String>,
    terrain_walkability: &BTreeMap<String, bool>,
) -> Result<(), ContentError> {
    let terrain_id = if position.x == 0
        || position.y == 0
        || position.x == world.width - 1
        || position.y == world.height - 1
    {
        &world.border_terrain_id
    } else {
        override_terrain
            .get(&position)
            .unwrap_or(&world.fill_terrain_id)
    };
    if terrain_walkability.get(terrain_id) != Some(&true) {
        return Err(ContentError::SpawnOnBlockedTerrain(world.id.clone()));
    }
    Ok(())
}

fn load_root<T: DeserializeOwned>(
    pack_root: &Path,
    root: &str,
    enabled_roots: &BTreeSet<&str>,
    budget: &mut SourceBudget,
) -> Result<Vec<T>, ContentError> {
    if !enabled_roots.contains(root) {
        return Ok(Vec::new());
    }
    let directory = pack_root.join(root);
    let metadata = fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentDirectory(directory));
    }
    let mut paths = fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut definitions = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(ContentError::InvalidContentFile(path));
        }
        definitions.push(read_json(&path, budget)?);
    }
    Ok(definitions)
}

fn read_json<T: DeserializeOwned>(
    path: &Path,
    budget: &mut SourceBudget,
) -> Result<T, ContentError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentFile(path.to_path_buf()));
    }
    budget.files = budget
        .files
        .checked_add(1)
        .ok_or(ContentError::LengthOverflow)?;
    if budget.files > MAX_SOURCE_FILES {
        return Err(ContentError::TooManySourceFiles(budget.files));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_SOURCE_FILE_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_FILE_LENGTH {
        return Err(ContentError::SourceFileTooLarge(path.to_path_buf()));
    }
    budget.bytes = budget
        .bytes
        .checked_add(bytes.len())
        .ok_or(ContentError::LengthOverflow)?;
    if budget.bytes > MAX_SOURCE_TOTAL_LENGTH {
        return Err(ContentError::SourcePackTooLarge(budget.bytes));
    }
    serde_json::from_slice(&bytes).map_err(|source| ContentError::InvalidJson {
        path: path.to_path_buf(),
        source,
    })
}

fn validate_definition_id(id: &str, category: &str) -> Result<(), ContentError> {
    validate_id(id)?;
    if id.split('.').nth(1) != Some(category) {
        return Err(ContentError::WrongIdCategory {
            id: id.to_owned(),
            expected: category.to_owned(),
        });
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), ContentError> {
    if id.is_empty()
        || id.len() > 128
        || id.split('.').count() < 3
        || id.split('.').any(str::is_empty)
        || !id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
    {
        return Err(ContentError::InvalidStableId(id.to_owned()));
    }
    Ok(())
}

fn validate_semver(version: &str) -> Result<(), ContentError> {
    if version.is_empty() || version.len() > 64 || !version.is_ascii() {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core_and_prerelease, build) = version
        .split_once('+')
        .map_or((version, None), |(core, build)| (core, Some(build)));
    if version.matches('+').count() > 1
        || build.is_some_and(|value| !valid_semver_identifiers(value, false))
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let (core, prerelease) = core_and_prerelease
        .split_once('-')
        .map_or((core_and_prerelease, None), |(core, prerelease)| {
            (core, Some(prerelease))
        });
    if prerelease.is_some_and(|value| !valid_semver_identifiers(value, true)) {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    let parts = core.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return Err(ContentError::InvalidPackVersion(version.to_owned()));
    }
    Ok(())
}

fn valid_semver_identifiers(value: &str, reject_numeric_leading_zero: bool) -> bool {
    !value.is_empty()
        && value.split('.').all(|identifier| {
            !identifier.is_empty()
                && identifier
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !(reject_numeric_leading_zero
                    && identifier.len() > 1
                    && identifier.starts_with('0')
                    && identifier.bytes().all(|byte| byte.is_ascii_digit()))
        })
}

fn validate_pack_relations(
    pack_id: &str,
    dependencies: &[PackDependency],
    load_after_entries: &[String],
) -> Result<(), ContentError> {
    let mut dependency_ids = BTreeSet::new();
    for dependency in dependencies {
        validate_id(&dependency.id)?;
        if dependency.id == pack_id || !dependency_ids.insert(&dependency.id) {
            return Err(ContentError::InvalidDependency(dependency.id.clone()));
        }
        if dependency.version_requirement.trim().is_empty()
            || dependency.version_requirement.len() > 64
        {
            return Err(ContentError::InvalidVersionRequirement(
                dependency.version_requirement.clone(),
            ));
        }
    }
    let mut load_after = BTreeSet::new();
    for id in load_after_entries {
        validate_id(id)?;
        if id == pack_id || !load_after.insert(id) {
            return Err(ContentError::InvalidLoadAfter(id.clone()));
        }
    }
    Ok(())
}

fn validate_message_key(key: &str) -> Result<(), ContentError> {
    if key.is_empty()
        || key.len() > 128
        || !key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidMessageKey(key.to_owned()));
    }
    Ok(())
}

fn validate_equipment_slot(slot: &str) -> Result<(), ContentError> {
    if slot.is_empty()
        || slot.len() > 64
        || !slot.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
    {
        return Err(ContentError::InvalidEquipmentSlot(slot.to_owned()));
    }
    Ok(())
}

fn validate_definition_text(
    id: &str,
    name_key: &str,
    description_key: &str,
) -> Result<(), ContentError> {
    validate_message_key(name_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    validate_message_key(description_key)
        .map_err(|_| ContentError::InvalidDefinitionText(id.to_owned()))?;
    Ok(())
}

fn validate_glyph(id: &str, glyph: &str) -> Result<(), ContentError> {
    let mut characters = glyph.chars();
    if characters.next().is_none_or(char::is_control) || characters.next().is_some() {
        return Err(ContentError::InvalidGlyph(id.to_owned()));
    }
    Ok(())
}

fn normalize_tags(id: &str, tags: &mut [String]) -> Result<(), ContentError> {
    for tag in tags.iter() {
        if tag.is_empty()
            || tag.len() > 64
            || !tag.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
            })
        {
            return Err(ContentError::InvalidTag {
                id: id.to_owned(),
                tag: tag.clone(),
            });
        }
    }
    tags.sort();
    if tags.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(ContentError::DuplicateTag(id.to_owned()));
    }
    Ok(())
}

fn insert_definition_id(ids: &mut BTreeSet<String>, id: &str) -> Result<(), ContentError> {
    if !ids.insert(id.to_owned()) {
        return Err(ContentError::DuplicateDefinitionId(id.to_owned()));
    }
    Ok(())
}

fn require_schema(actual: &str, expected: &str, owner: &str) -> Result<(), ContentError> {
    if actual != expected {
        return Err(ContentError::SchemaMismatch(owner.to_owned()));
    }
    Ok(())
}

fn require_format_version(actual: u16, owner: &str) -> Result<(), ContentError> {
    if actual != CONTENT_FORMAT_VERSION {
        return Err(ContentError::UnsupportedSourceVersion {
            owner: owner.to_owned(),
            version: actual,
        });
    }
    Ok(())
}

fn require_reference(
    ids: &BTreeSet<String>,
    target: &str,
    owner: &str,
) -> Result<(), ContentError> {
    if !ids.contains(target) {
        return Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        });
    }
    Ok(())
}

fn require_actor_role(
    roles: &BTreeMap<String, ActorRole>,
    target: &str,
    expected: ActorRole,
    owner: &str,
) -> Result<(), ContentError> {
    match roles.get(target) {
        Some(actual) if *actual == expected => Ok(()),
        Some(_) => Err(ContentError::WrongActorRole(target.to_owned())),
        None => Err(ContentError::DanglingReference {
            owner: owner.to_owned(),
            target: target.to_owned(),
        }),
    }
}

fn validate_position(
    position: ContentPosition,
    width: u16,
    height: u16,
    owner: &str,
) -> Result<(), ContentError> {
    if position.x >= width || position.y >= height {
        return Err(ContentError::PositionOutOfBounds(owner.to_owned()));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ContentError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ContentError> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}

#[cfg(feature = "schemas")]
pub fn generated_schema_documents() -> Result<Vec<(&'static str, String)>, serde_json::Error> {
    Ok(vec![
        schema_document("pack.schema.json", PACK_SCHEMA, schema_for!(PackManifest))?,
        schema_document(
            "terrain.schema.json",
            TERRAIN_SCHEMA,
            schema_for!(TerrainDefinition),
        )?,
        schema_document(
            "actor.schema.json",
            ACTOR_SCHEMA,
            schema_for!(ActorDefinition),
        )?,
        schema_document("item.schema.json", ITEM_SCHEMA, schema_for!(ItemDefinition))?,
        schema_document(
            "affix.schema.json",
            AFFIX_SCHEMA,
            schema_for!(AffixDefinition),
        )?,
        schema_document(
            "encounter-table.schema.json",
            ENCOUNTER_TABLE_SCHEMA,
            schema_for!(EncounterTableDefinition),
        )?,
        schema_document(
            "loot-table.schema.json",
            LOOT_TABLE_SCHEMA,
            schema_for!(LootTableDefinition),
        )?,
        schema_document(
            "theme-table.schema.json",
            THEME_TABLE_SCHEMA,
            schema_for!(ThemeTableDefinition),
        )?,
        schema_document(
            "region-table.schema.json",
            REGION_TABLE_SCHEMA,
            schema_for!(RegionTableDefinition),
        )?,
        schema_document(
            "terrain-feature-table.schema.json",
            TERRAIN_FEATURE_TABLE_SCHEMA,
            schema_for!(TerrainFeatureTableDefinition),
        )?,
        schema_document(
            "vault.schema.json",
            VAULT_SCHEMA,
            schema_for!(VaultDefinition),
        )?,
        schema_document(
            "world.schema.json",
            WORLD_SCHEMA,
            schema_for!(WorldDefinition),
        )?,
    ])
}

#[cfg(feature = "schemas")]
fn schema_document<T: Serialize>(
    file_name: &'static str,
    schema_id: &str,
    schema: T,
) -> Result<(&'static str, String), serde_json::Error> {
    let mut value = serde_json::to_value(schema)?;
    if let Some(object) = value.as_object_mut() {
        object.insert(
            "$id".to_owned(),
            serde_json::Value::String(schema_id.to_owned()),
        );
    }
    let mut output = serde_json::to_string_pretty(&value)?;
    output.push('\n');
    Ok((file_name, output))
}

#[derive(Debug, Default)]
struct SourceBudget {
    files: usize,
    bytes: usize,
}

#[derive(Debug, Error)]
pub enum ContentError {
    #[error("content pack root is invalid or is a symlink: {0}")]
    InvalidPackRoot(PathBuf),
    #[error("content directory is invalid or is a symlink: {0}")]
    InvalidContentDirectory(PathBuf),
    #[error("content entry must be a regular .json file: {0}")]
    InvalidContentFile(PathBuf),
    #[error("content source file exceeds the 1 MiB limit: {0}")]
    SourceFileTooLarge(PathBuf),
    #[error("content source pack exceeds the 16 MiB limit: {0} bytes")]
    SourcePackTooLarge(usize),
    #[error("content source pack exceeds the file-count limit: {0}")]
    TooManySourceFiles(usize),
    #[error("invalid JSON in {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("schema identifier does not match for {0}")]
    SchemaMismatch(String),
    #[error("unsupported source format version {version} in {owner}")]
    UnsupportedSourceVersion { owner: String, version: u16 },
    #[error("unsupported content root {0}")]
    UnsupportedContentRoot(String),
    #[error("duplicate content root {0}")]
    DuplicateContentRoot(String),
    #[error("invalid stable content ID {0}")]
    InvalidStableId(String),
    #[error("content ID {id} must use category {expected}")]
    WrongIdCategory { id: String, expected: String },
    #[error("invalid pack semantic version {0}")]
    InvalidPackVersion(String),
    #[error("invalid dependency {0}")]
    InvalidDependency(String),
    #[error("invalid dependency version requirement {0}")]
    InvalidVersionRequirement(String),
    #[error("invalid loadAfter entry {0}")]
    InvalidLoadAfter(String),
    #[error("invalid localization message key {0}")]
    InvalidMessageKey(String),
    #[error("definition name or description key is invalid: {0}")]
    InvalidDefinitionText(String),
    #[error("definition glyph must contain one non-control Unicode scalar: {0}")]
    InvalidGlyph(String),
    #[error("terrain open/close transition is invalid: {0}")]
    InvalidTerrainTransition(String),
    #[error("invalid tag {tag} in {id}")]
    InvalidTag { id: String, tag: String },
    #[error("duplicate tag in {0}")]
    DuplicateTag(String),
    #[error("duplicate definition ID {0}")]
    DuplicateDefinitionId(String),
    #[error("actor stats are outside supported limits: {0}")]
    InvalidActorStats(String),
    #[error("actor carry capacity is invalid for its role: {0}")]
    InvalidActorCarryCapacity(String),
    #[error("actor melee routine is invalid or requires the monster role: {0}")]
    InvalidMeleeRoutine(String),
    #[error("actor loot table reference is invalid or requires the monster role: {0}")]
    InvalidActorLootTable(String),
    #[error("item stack limit is outside supported limits: {0}")]
    InvalidItemStack(String),
    #[error("item weight is outside supported limits: {0}")]
    InvalidItemWeight(String),
    #[error("item appearance must use a distinct valid message key: {0}")]
    InvalidItemAppearance(String),
    #[error("item break chance is outside 0..=100 percent: {0}")]
    InvalidItemBreakChance(String),
    #[error("item equipment slot is invalid or requires maxStack 1: {0}")]
    InvalidEquipmentSlot(String),
    #[error("item stat modifiers are invalid or require an equipment slot: {0}")]
    InvalidItemModifiers(String),
    #[error("item attack profile is invalid or requires the weapon slot: {0}")]
    InvalidAttackProfile(String),
    #[error("item projectile profile is invalid or requires the launcher slot: {0}")]
    InvalidProjectileProfile(String),
    #[error("item throw profile is invalid: {0}")]
    InvalidThrowProfile(String),
    #[error("item use action is invalid: {0}")]
    InvalidItemUseAction(String),
    #[error("affix stat modifiers are invalid: {0}")]
    InvalidAffixModifiers(String),
    #[error("loot table weights, entries, or generated item constraints are invalid: {0}")]
    InvalidLootTable(String),
    #[error("encounter table weights, depth ranges, or actor entries are invalid: {0}")]
    InvalidEncounterTable(String),
    #[error("theme table weights, depth ranges, terrain, or vault candidates are invalid: {0}")]
    InvalidThemeTable(String),
    #[error("region table weights, depth ranges, or local table references are invalid: {0}")]
    InvalidRegionTable(String),
    #[error("terrain feature table weights, depth ranges, terrain, or placements are invalid: {0}")]
    InvalidTerrainFeatureTable(String),
    #[error("vault terrain, encounters, or loot definition is invalid: {0}")]
    InvalidVault(String),
    #[error("world dimensions are outside supported limits: {0}")]
    InvalidWorldDimensions(String),
    #[error("procedural floor definition is invalid: {0}")]
    InvalidProceduralFloor(String),
    #[error("content reference from {owner} to {target} is unresolved")]
    DanglingReference { owner: String, target: String },
    #[error("actor has the wrong role for this spawn: {0}")]
    WrongActorRole(String),
    #[error("duplicate runtime instance ID {0}")]
    DuplicateInstanceId(String),
    #[error("two actors occupy the same world position: {0}")]
    DuplicateActorPosition(String),
    #[error("content position is outside world bounds: {0}")]
    PositionOutOfBounds(String),
    #[error("world spawn is placed on non-walkable terrain: {0}")]
    SpawnOnBlockedTerrain(String),
    #[error("terrain override is duplicated or touches the generated border: {0}")]
    InvalidTerrainOverride(String),
    #[error("item spawn quantity is invalid: {0}")]
    InvalidItemQuantity(String),
    #[error("item spawn affix references are invalid: {0}")]
    InvalidItemAffixes(String),
    #[error("compiled content metadata is invalid")]
    InvalidCompiledMetadata,
    #[error("compiled content payload exceeds the 32 MiB limit: {0} bytes")]
    CompiledPayloadTooLarge(usize),
    #[error("compiled content container is invalid or truncated")]
    InvalidContainer,
    #[error("unsupported compiled content container version {0}")]
    UnsupportedContainerVersion(u16),
    #[error("unsupported compiled content container flags 0x{0:04x}")]
    UnsupportedContainerFlags(u16),
    #[error("compiled content checksum does not match")]
    ChecksumMismatch,
    #[error("compiled content is not in canonical sorted form")]
    NonCanonicalCompiledContent,
    #[error("content.lock.json does not match the deterministic compiled pack")]
    ContentLockMismatch,
    #[error("content length overflow")]
    LengthOverflow,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("MessagePack encode error: {0}")]
    MessagePackEncode(#[from] rmp_serde::encode::Error),
    #[error("MessagePack decode error: {0}")]
    MessagePackDecode(#[from] rmp_serde::decode::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_pack_path() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("crate should be inside the workspace")
            .join("packs/rfb-demo-original")
    }

    #[test]
    fn original_pack_compiles_deterministically_and_round_trips() {
        let first = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
        let second = compile_pack_dir(&original_pack_path()).expect("recompile should succeed");
        let decoded = decode_content(&first.bytes).expect("compiled pack should decode");

        assert_eq!(first.content_hash, second.content_hash);
        assert_eq!(first.bytes, second.bytes);
        assert_eq!(decoded, first);
        assert_eq!(first.content.pack_id, "rfb.demo.original-v1");
        assert_eq!(first.content.terrain.len(), 44);
        assert_eq!(first.content.actors.len(), 10);
        assert_eq!(first.content.affixes.len(), 1);
        assert_eq!(first.content.items.len(), 5);
        assert_eq!(first.content.encounter_tables.len(), 6);
        assert_eq!(first.content.loot_tables.len(), 7);
        assert_eq!(first.content.theme_tables.len(), 3);
        assert_eq!(first.content.region_tables.len(), 1);
        assert_eq!(first.content.terrain_feature_tables.len(), 1);
        assert_eq!(first.content.vaults.len(), 5);
        assert_eq!(first.content.worlds.len(), 1);
    }

    #[test]
    fn compiled_catalog_exposes_stable_runtime_indexes() {
        let artifact =
            verify_pack_lock(&original_pack_path()).expect("original pack should verify");
        let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

        assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
        assert_eq!(catalog.pack_version(), "1.54.0");
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .and_then(|actor| actor.loot_table_id.as_deref()),
            Some("demo.loot-table.ember-mote")
        );
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .and_then(|actor| actor.carried_loot_table_id.as_deref()),
            Some("demo.loot-table.ember-mote-carried")
        );
        assert_eq!(
            catalog
                .loot_table("demo.loot-table.ember-mote")
                .map(|table| (table.rolls, table.entries.len())),
            Some((1, 2))
        );
        assert_eq!(
            catalog
                .encounter_table("demo.encounter-table.echo-depths")
                .map(|table| (table.rolls, table.entries.len())),
            Some((1, 5))
        );
        assert_eq!(
            catalog
                .encounter_table("demo.encounter-table.resonance-formations")
                .map(|table| {
                    table
                        .entries
                        .iter()
                        .filter(|entry| entry.group.is_some())
                        .count()
                }),
            Some(2)
        );
        assert_eq!(
            catalog
                .encounter_table("demo.encounter-table.resonance-formations")
                .and_then(|table| table.entries.iter().find_map(|entry| entry.group.as_ref()))
                .map(|group| group.pack_ai),
            Some(EncounterPackAiDefinition {
                leader: MonsterPackBehavior::Seek,
                friends: MonsterPackBehavior::Surround,
                escorts: MonsterPackBehavior::GuardLeader,
            })
        );
        assert_eq!(
            catalog
                .theme_table("demo.theme-table.echo-depths")
                .map(|table| table.entries[0].vault_candidates.len()),
            Some(2)
        );
        assert_eq!(
            catalog
                .region_table("demo.region-table.resonance-biomes")
                .map(|table| {
                    table
                        .entries
                        .iter()
                        .map(|entry| (entry.region_id.as_str(), entry.weight))
                        .collect::<Vec<_>>()
                }),
            Some(vec![
                ("demo.region.resonance-gallery", 1),
                ("demo.region.resonance-grotto", 3),
            ])
        );
        assert_eq!(
            catalog
                .terrain_feature_table("demo.terrain-feature-table.resonance-hazards")
                .map(|table| (table.rolls, table.entries.len())),
            Some((4, 4))
        );
        let world = catalog
            .world("demo.world.original-v1")
            .expect("demo world should remain available");
        assert_eq!(world.initial_floor_id, "demo.floor.surface");
        assert_eq!(world.procedural_floors.len(), 19);
        assert_eq!(world.procedural_floors[0].id, "demo.floor.echo-depth-1");
        assert_eq!(world.procedural_floors[0].depth, 1);
        let regional_floor = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-2")
            .expect("demo world should retain its regional floor");
        assert_eq!(
            regional_floor.region_table_id.as_deref(),
            Some("demo.region-table.resonance-biomes")
        );
        assert_eq!(
            regional_floor.generation_budget.as_ref().map(|budget| (
                budget.actor_slots,
                budget.loot_placements,
                budget.region_placements,
            )),
            Some((4, 2, Some(2)))
        );
        assert_eq!(
            world.procedural_floors[0].closed_door_terrain_id,
            "demo.terrain.door-secret"
        );
        assert!(world.procedural_floors[0].actor_spawns.is_empty());
        assert!(world.procedural_floors[0].loot_spawns.is_empty());
        assert_eq!(
            world.procedural_floors[0].encounter_table_id.as_deref(),
            Some("demo.encounter-table.echo-depths")
        );
        assert_eq!(
            world.procedural_floors[0].loot_table_id.as_deref(),
            Some("demo.loot-table.echo-depth-1-room")
        );
        assert_eq!(
            world.procedural_floors[0].theme_table_id.as_deref(),
            Some("demo.theme-table.echo-depths")
        );
        let final_floor = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("demo world should retain the budgeted cavern floor");
        assert_eq!(
            final_floor.generation_budget.as_ref().map(|budget| (
                budget.room_placements,
                budget.room_area_tiles,
                budget.cavern_area_tiles,
                budget.lake_area_tiles,
                budget.lake_deep_area_tiles,
                budget.river_area_tiles,
                budget.destruction_centers,
                budget.destroyed_area_tiles,
                budget.streamer_placements,
                budget.streamer_area_tiles,
            )),
            Some((
                Some(5),
                Some(112),
                Some(64),
                Some(76),
                Some(30),
                Some(52),
                Some(2),
                Some(48),
                Some(2),
                Some(24)
            ))
        );
        assert_eq!(
            final_floor.layout.as_ref().map(|layout| (
                layout.rooms.as_ref().map_or(0, |rooms| rooms.shapes.len()),
                layout
                    .cavern
                    .as_ref()
                    .map(|cavern| cavern.terrain_id.as_str()),
                layout
                    .lake
                    .as_ref()
                    .map(|lake| lake.deep_terrain_id.as_str()),
                layout
                    .river
                    .as_ref()
                    .map(|river| river.shallow_terrain_id.as_str()),
                layout
                    .destroyed
                    .as_ref()
                    .map(|destroyed| destroyed.terrain_id.as_str()),
                layout.streamers.len(),
            )),
            Some((
                2,
                Some("demo.terrain.resonance-cavern"),
                Some("demo.terrain.resonance-water-deep"),
                Some("demo.terrain.resonance-water-shallow"),
                Some("demo.terrain.resonance-ruin"),
                1
            ))
        );
        let maze_floor = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-9")
            .expect("demo world should retain the maze floor");
        assert_eq!(
            maze_floor.generation_budget.as_ref().map(|budget| (
                budget.maze_floor_tiles,
                budget.streamer_placements,
                budget.streamer_area_tiles
            )),
            Some((Some(127), Some(2), Some(24)))
        );
        assert_eq!(
            maze_floor.layout.as_ref().and_then(|layout| {
                layout
                    .maze
                    .as_ref()
                    .map(|maze| (layout.mode, maze.width, maze.height, layout.streamers.len()))
            }),
            Some((ProceduralLayoutMode::MazeOnly, 15, 15, 1))
        );
        assert_eq!(
            final_floor.layout.as_ref().and_then(|layout| {
                layout.pit.as_ref().map(|pit| {
                    (
                        pit.encounter_table_id.as_str(),
                        pit.inner_width,
                        pit.inner_height,
                        pit.roster_size,
                    )
                })
            }),
            Some(("demo.encounter-table.resonance-pit", 5, 5, 5))
        );
        assert_eq!(
            final_floor.generation_budget.as_ref().map(|budget| (
                budget.actor_slots,
                budget.pit_placements,
                budget.pit_actor_slots,
            )),
            Some((30, Some(1), Some(25)))
        );
        assert_eq!(
            world.procedural_floors[0]
                .generation_budget
                .as_ref()
                .map(|budget| (budget.actor_slots, budget.loot_placements)),
            Some((4, 1))
        );
        assert_eq!(
            world.procedural_floors[0]
                .nest
                .as_ref()
                .map(|nest| (nest.room_id.as_str(), nest.spawn_count)),
            Some(("remote", 3))
        );
        let pressure_final = world
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("demo world should contain the pressure final floor");
        assert!(pressure_final.final_floor);
        assert_eq!(pressure_final.depth, 10);
        assert_eq!(
            pressure_final
                .generation_budget
                .as_ref()
                .map(|budget| (budget.actor_slots, budget.loot_placements)),
            Some((30, 3))
        );
        assert_eq!(
            catalog
                .vault("demo.vault.harmonic-sepulcher")
                .map(|vault| (vault.theme_id.as_str(), vault.encounter_groups.len())),
            Some(("demo.theme.echo-depths", 1))
        );
        assert_eq!(
            catalog
                .terrain("demo.terrain.door-closed")
                .and_then(|terrain| terrain.open_to_terrain_id.as_deref()),
            Some("demo.terrain.door-open")
        );
        assert_eq!(
            catalog.terrain("demo.terrain.door-locked").map(|terrain| (
                terrain.open_check_difficulty,
                terrain.bash_to_terrain_id.as_deref(),
                terrain.bash_check_difficulty,
            )),
            Some((Some(24), Some("demo.terrain.door-broken"), Some(18)))
        );
        assert_eq!(
            catalog.terrain("demo.terrain.door-secret").map(|terrain| (
                terrain.concealed_as_terrain_id.as_deref(),
                terrain.search_check_difficulty,
            )),
            Some((Some("demo.terrain.wall"), Some(8)))
        );
        assert_eq!(
            catalog
                .terrain("demo.terrain.door-open")
                .and_then(|terrain| terrain.close_to_terrain_id.as_deref()),
            Some("demo.terrain.door-closed")
        );
        assert_eq!(
            catalog.actor("demo.actor.explorer").map(|actor| (
                actor.door_skill,
                actor.bash_power,
                actor.search_skill
            )),
            Some((24, 30, 24))
        );
        assert_eq!(
            catalog
                .actor("demo.actor.echo-hound")
                .and_then(|actor| actor.melee_routine.as_ref())
                .map(|routine| routine
                    .blows
                    .iter()
                    .map(|blow| blow.method_id.as_str())
                    .collect::<Vec<_>>()),
            Some(vec!["rfb.blow.echo-bite", "rfb.blow.echo-rake"])
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-blade")
                .and_then(|item| item.melee_profile.as_ref())
                .map(|profile| (profile.attacks, profile.to_hit, profile.to_damage)),
            Some((2, 10, 1))
        );
        assert_eq!(
            catalog
                .item("demo.item.resonance-sling")
                .and_then(|item| item.projectile_profile.as_ref())
                .map(|profile| (
                    profile.range,
                    profile.to_hit,
                    profile.to_damage,
                    profile.ammo_kind_id.as_str(),
                )),
            Some((6, 30, 1, "demo.item.resonance-pellet"))
        );
        assert_eq!(catalog.content_hash(), artifact.content_hash);
        assert_eq!(
            catalog
                .terrain("demo.terrain.wall")
                .map(|terrain| terrain.walkable),
            Some(false)
        );
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .map(|actor| actor.max_hp),
            Some(3)
        );
        assert_eq!(
            catalog
                .actor("demo.actor.ember-mote")
                .map(|actor| actor.damage_type),
            Some(ActorDamageType::Fire)
        );
        assert_eq!(
            catalog.actor("demo.actor.explorer").map(|actor| (
                actor.attack,
                actor.defense,
                actor.damage_dice,
                actor.damage_sides,
                actor.speed,
                actor.carry_capacity_tenths_pound,
            )),
            Some((2, 1, 1, 2, 110, 100))
        );
        assert_eq!(
            catalog
                .item("demo.item.luminous-shard")
                .map(|item| item.max_stack),
            Some(20)
        );
        assert!(matches!(
            catalog
                .item("demo.item.luminous-shard")
                .and_then(|item| item.use_action.as_ref())
                .map(|action| &action.effect),
            Some(ItemUseEffectDefinition::Heal { amount: 4 })
        ));
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .and_then(|item| item.equipment_slot.as_deref()),
            Some("charm")
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .map(|item| item.modifiers.max_hp),
            Some(4)
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-charm")
                .map(|item| (item.modifiers.attack, item.modifiers.defense)),
            Some((1, 1))
        );
        assert_eq!(
            catalog
                .affix("demo.affix.harmonic-edge")
                .map(|affix| affix.modifiers.attack),
            Some(1)
        );
        assert_eq!(
            catalog
                .world("demo.world.original-v1")
                .and_then(|world| world
                    .items
                    .iter()
                    .find(|item| item.kind_id == "demo.item.echo-charm")
                    .map(|item| (item.quality, item.affix_ids.as_slice()))),
            Some((
                ItemQuality::Fine,
                ["demo.affix.harmonic-edge".to_owned()].as_slice()
            ))
        );
        assert!(catalog.world("demo.world.original-v1").is_some());
        assert_eq!(
            catalog.visual_glyphs().get("demo.item.luminous-shard"),
            Some(&"!".to_owned())
        );
    }

    #[test]
    fn dangling_references_and_checksum_corruption_are_rejected() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        invalid.worlds[0].fill_terrain_id = "demo.terrain.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut blocked_spawn = artifact.content.clone();
        blocked_spawn.worlds[0].player.position = ContentPosition { x: 11, y: 3 };
        assert!(matches!(
            validate_and_normalize(&mut blocked_spawn),
            Err(ContentError::SpawnOnBlockedTerrain(_))
        ));

        let mut corrupted = artifact.bytes;
        let last = corrupted.len() - 1;
        corrupted[last] ^= 0x01;
        assert!(matches!(
            decode_content(&corrupted),
            Err(ContentError::ChecksumMismatch)
        ));
    }

    #[test]
    fn loot_tables_require_valid_weights_references_and_instance_shapes() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut zero_weight = artifact.content.clone();
        zero_weight
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.ember-mote")
            .expect("fixture should contain the death loot table")
            .entries[0]
            .weight = 0;
        assert!(matches!(
            validate_and_normalize(&mut zero_weight),
            Err(ContentError::InvalidLootTable(_))
        ));

        let mut dangling_affix = artifact.content.clone();
        dangling_affix
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.ember-mote")
            .expect("fixture should contain the death loot table")
            .affix_weights[1]
            .affix_id = Some("demo.affix.missing".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut dangling_affix),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut stackable_quality = artifact.content.clone();
        stackable_quality
            .loot_tables
            .iter_mut()
            .find(|table| table.id == "demo.loot-table.ember-mote")
            .expect("fixture should contain the death loot table")
            .entries[0]
            .item_kind_id = "demo.item.luminous-shard".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut stackable_quality),
            Err(ContentError::InvalidLootTable(_))
        ));

        let mut player_drop = artifact.content.clone();
        let player = player_drop
            .actors
            .iter_mut()
            .find(|actor| actor.role == ActorRole::Player)
            .expect("fixture should contain the player");
        player.loot_table_id = Some("demo.loot-table.ember-mote".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut player_drop),
            Err(ContentError::InvalidActorLootTable(_))
        ));

        let mut player_carry = artifact.content.clone();
        let player = player_carry
            .actors
            .iter_mut()
            .find(|actor| actor.role == ActorRole::Player)
            .expect("fixture should contain the player");
        player.carried_loot_table_id = Some("demo.loot-table.ember-mote-carried".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut player_carry),
            Err(ContentError::InvalidActorLootTable(_))
        ));
    }

    #[test]
    fn procedural_floor_tables_require_valid_depth_roles_and_references() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut zero_depth = artifact.content.clone();
        zero_depth.worlds[0].procedural_floors[0].depth = 0;
        assert!(matches!(
            validate_and_normalize(&mut zero_depth),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut player_candidate = artifact.content.clone();
        player_candidate.encounter_tables[0].entries[0].actor_kind_id =
            "demo.actor.explorer".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut player_candidate),
            Err(ContentError::WrongActorRole(_))
        ));

        let mut dangling_loot = artifact.content.clone();
        dangling_loot.worlds[0].procedural_floors[0].loot_table_id =
            Some("demo.loot-table.missing".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut dangling_loot),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut duplicate_actor = artifact.content.clone();
        duplicate_actor.worlds[0].procedural_floors[0].encounter_table_id = None;
        duplicate_actor.worlds[0].procedural_floors[0].generation_budget = None;
        duplicate_actor.worlds[0].procedural_floors[0].nest = None;
        duplicate_actor.worlds[0].procedural_floors[0]
            .actor_spawns
            .push(ProceduralActorSpawnDefinition {
                instance_id: "demo.monster.ember-mote.1".to_owned(),
                room_id: "remote".to_owned(),
                actor_kind_ids: vec!["demo.actor.echo-hound".to_owned()],
            });
        assert!(matches!(
            validate_and_normalize(&mut duplicate_actor),
            Err(ContentError::DuplicateInstanceId(_))
        ));

        let mut zero_weight = artifact.content.clone();
        zero_weight.encounter_tables[0].entries[0].weight = 0;
        assert!(matches!(
            validate_and_normalize(&mut zero_weight),
            Err(ContentError::InvalidEncounterTable(_))
        ));

        let mut missing_theme = artifact.content.clone();
        missing_theme.worlds[0].procedural_floors[0].theme_table_id =
            Some("demo.theme-table.missing".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut missing_theme),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut exhausted_actor_budget = artifact.content.clone();
        exhausted_actor_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-1")
            .expect("fixture should contain the nest floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .actor_slots = 3;
        assert!(matches!(
            validate_and_normalize(&mut exhausted_actor_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut exhausted_loot_budget = artifact.content.clone();
        exhausted_loot_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-2")
            .expect("fixture should contain the vault floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .loot_placements = 1;
        assert!(matches!(
            validate_and_normalize(&mut exhausted_loot_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incomplete_spatial_budget = artifact.content.clone();
        incomplete_spatial_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-8")
            .expect("fixture should contain the spatial Vault floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .vault_area_tiles = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_spatial_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incomplete_group_budget = artifact.content.clone();
        incomplete_group_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-6")
            .expect("fixture should contain the dynamic group floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .group_actor_slots = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_group_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut undersized_group_budget = artifact.content.clone();
        undersized_group_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-6")
            .expect("fixture should contain the dynamic group floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .group_actor_slots = Some(1);
        assert!(matches!(
            validate_and_normalize(&mut undersized_group_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut player_escort = artifact.content.clone();
        player_escort
            .encounter_tables
            .iter_mut()
            .find(|table| table.id == "demo.encounter-table.resonance-formations")
            .expect("fixture should contain the formation encounter table")
            .entries
            .iter_mut()
            .find_map(|entry| entry.group.as_mut())
            .and_then(|group| group.escort.as_mut())
            .expect("fixture should contain an escort table")
            .entries[0]
            .actor_kind_id = "demo.actor.explorer".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut player_escort),
            Err(ContentError::WrongActorRole(_))
        ));

        let mut self_guarding_leader = artifact.content.clone();
        self_guarding_leader
            .encounter_tables
            .iter_mut()
            .find(|table| table.id == "demo.encounter-table.resonance-formations")
            .expect("fixture should contain the formation encounter table")
            .entries
            .iter_mut()
            .find_map(|entry| entry.group.as_mut())
            .expect("fixture should contain a dynamic group")
            .pack_ai
            .leader = MonsterPackBehavior::GuardLeader;
        assert!(matches!(
            validate_and_normalize(&mut self_guarding_leader),
            Err(ContentError::InvalidEncounterTable(_))
        ));

        let mut invalid_feature_terrain = artifact.content.clone();
        invalid_feature_terrain.terrain_feature_tables[0].entries[0].terrain_id =
            "demo.terrain.floor".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid_feature_terrain),
            Err(ContentError::InvalidTerrainFeatureTable(_))
        ));

        let mut incomplete_feature_budget = artifact.content.clone();
        incomplete_feature_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-3")
            .expect("fixture should contain the feature-budget floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .feature_placements = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_feature_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut oversized_feature_budget = artifact.content.clone();
        oversized_feature_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-3")
            .expect("fixture should contain the feature-budget floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .feature_placements = Some(5);
        assert!(matches!(
            validate_and_normalize(&mut oversized_feature_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incomplete_room_budget = artifact.content.clone();
        incomplete_room_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the room-budget floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .room_area_tiles = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_room_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut undersized_room_budget = artifact.content.clone();
        undersized_room_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the room-budget floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .room_area_tiles = Some(35);
        assert!(matches!(
            validate_and_normalize(&mut undersized_room_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut blocked_cavern = artifact.content.clone();
        blocked_cavern.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the cavern floor")
            .layout
            .as_mut()
            .expect("fixture should contain a layout")
            .cavern
            .as_mut()
            .expect("fixture should contain a cavern")
            .terrain_id = "demo.terrain.wall".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut blocked_cavern),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incomplete_cavern_budget = artifact.content.clone();
        incomplete_cavern_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the cavern floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .cavern_area_tiles = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_cavern_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incomplete_lake_budget = artifact.content.clone();
        incomplete_lake_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the lake floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .lake_deep_area_tiles = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_lake_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut walkable_deep_water = artifact.content.clone();
        walkable_deep_water
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.resonance-water-deep")
            .expect("fixture should contain deep water")
            .walkable = true;
        assert!(matches!(
            validate_and_normalize(&mut walkable_deep_water),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut incompatible_river = artifact.content.clone();
        incompatible_river.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the river floor")
            .layout
            .as_mut()
            .expect("fixture should contain a layout")
            .river
            .as_mut()
            .expect("fixture should contain a river")
            .shallow_terrain_id = "demo.terrain.floor".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut incompatible_river),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut mismatched_maze_budget = artifact.content.clone();
        mismatched_maze_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-9")
            .expect("fixture should contain the maze floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .maze_floor_tiles = Some(126);
        assert!(matches!(
            validate_and_normalize(&mut mismatched_maze_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut maze_with_rooms = artifact.content.clone();
        let room_geometry = maze_with_rooms.worlds[0]
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .and_then(|floor| floor.layout.as_ref())
            .and_then(|layout| layout.rooms.clone())
            .expect("fixture should contain room geometry");
        maze_with_rooms.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-9")
            .and_then(|floor| floor.layout.as_mut())
            .expect("fixture should contain the maze-only layout")
            .rooms = Some(room_geometry);
        assert!(matches!(
            validate_and_normalize(&mut maze_with_rooms),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut room_overlay_maze = artifact.content.clone();
        let final_floor = room_overlay_maze.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the rooms floor");
        final_floor
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .maze_floor_tiles = Some(127);
        final_floor
            .layout
            .as_mut()
            .expect("fixture should contain a layout")
            .maze = Some(ProceduralMazeDefinition {
            width: 15,
            height: 15,
        });
        assert!(matches!(
            validate_and_normalize(&mut room_overlay_maze),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut mismatched_pit_budget = artifact.content.clone();
        mismatched_pit_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the pit floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .pit_actor_slots = Some(24);
        assert!(matches!(
            validate_and_normalize(&mut mismatched_pit_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut dangling_pit_table = artifact.content.clone();
        dangling_pit_table.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the pit floor")
            .layout
            .as_mut()
            .and_then(|layout| layout.pit.as_mut())
            .expect("fixture should contain a pit")
            .encounter_table_id = "demo.encounter-table.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut dangling_pit_table),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut incomplete_destroyed_budget = artifact.content.clone();
        incomplete_destroyed_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the destroyed floor")
            .generation_budget
            .as_mut()
            .expect("fixture should contain a generation budget")
            .destruction_centers = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_destroyed_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut walkable_streamer = artifact.content.clone();
        walkable_streamer
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.resonance-vein")
            .expect("fixture should contain the streamer terrain")
            .walkable = true;
        assert!(validate_and_normalize(&mut walkable_streamer).is_err());

        let mut duplicate_room_shape = artifact.content.clone();
        let shapes = &mut duplicate_room_shape.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .expect("fixture should contain the room-layout floor")
            .layout
            .as_mut()
            .expect("fixture should contain a layout")
            .rooms
            .as_mut()
            .expect("fixture should contain room geometry")
            .shapes;
        shapes[1].shape = shapes[0].shape;
        assert!(matches!(
            validate_and_normalize(&mut duplicate_room_shape),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn region_tables_require_depth_eligible_candidates_and_isolated_budgets() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        fn regional_floor(content: &mut CompiledContentV1) -> &mut ProceduralFloorDefinition {
            content.worlds[0]
                .procedural_floors
                .iter_mut()
                .find(|floor| floor.id == "demo.floor.resonance-depth-2")
                .expect("fixture should contain the regional floor")
        }

        let mut exhausted_depth = artifact.content.clone();
        regional_floor(&mut exhausted_depth).depth = 3;
        assert!(matches!(
            validate_and_normalize(&mut exhausted_depth),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut missing_budget = artifact.content.clone();
        regional_floor(&mut missing_budget)
            .generation_budget
            .as_mut()
            .expect("regional floor should retain a generation budget")
            .region_placements = None;
        assert!(matches!(
            validate_and_normalize(&mut missing_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut oversized_budget = artifact.content.clone();
        regional_floor(&mut oversized_budget)
            .generation_budget
            .as_mut()
            .expect("regional floor should retain a generation budget")
            .region_placements = Some(3);
        assert!(matches!(
            validate_and_normalize(&mut oversized_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut mixed_floor_tables = artifact.content.clone();
        regional_floor(&mut mixed_floor_tables).encounter_table_id =
            Some("demo.encounter-table.resonance-descent".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut mixed_floor_tables),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut mixed_features = artifact.content.clone();
        mixed_features.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-2")
            .expect("fixture should contain the regional floor")
            .terrain_feature_table_id =
            Some("demo.terrain-feature-table.resonance-hazards".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut mixed_features),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut missing_theme = artifact.content.clone();
        missing_theme.region_tables[0].entries[0].theme_id =
            "demo.theme.resonance-missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut missing_theme),
            Err(ContentError::InvalidRegionTable(_))
        ));

        let mut grouped_encounter = artifact.content.clone();
        let group = grouped_encounter
            .encounter_tables
            .iter()
            .find(|table| table.id == "demo.encounter-table.resonance-formations")
            .and_then(|table| table.entries.iter().find_map(|entry| entry.group.clone()))
            .expect("fixture should contain a dynamic group");
        grouped_encounter
            .encounter_tables
            .iter_mut()
            .find(|table| table.id == "demo.encounter-table.resonance-gallery")
            .expect("fixture should contain the regional encounter table")
            .entries[0]
            .group = Some(group);
        assert!(validate_and_normalize(&mut grouped_encounter).is_err());
    }

    #[test]
    fn vaults_require_walkable_unique_positions_and_depth_eligible_encounters() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut blocked_member = artifact.content.clone();
        blocked_member.vaults[0].encounter_groups[0].member_positions[0] =
            ContentPosition { x: 0, y: 0 };
        assert!(matches!(
            validate_and_normalize(&mut blocked_member),
            Err(ContentError::InvalidVault(_))
        ));

        let mut duplicate_transform = artifact.content.clone();
        let transform = duplicate_transform.vaults[0]
            .transforms
            .first()
            .copied()
            .unwrap_or(VaultTransform::Identity);
        duplicate_transform.vaults[0].transforms = vec![transform, transform];
        assert!(matches!(
            validate_and_normalize(&mut duplicate_transform),
            Err(ContentError::InvalidVault(_))
        ));

        let mut interior_entrance = artifact.content.clone();
        let vault = interior_entrance
            .vaults
            .iter_mut()
            .find(|vault| vault.width >= 4 && vault.height >= 4)
            .expect("fixture should contain a large Vault");
        vault.entrance_position = ContentPosition { x: 1, y: 1 };
        assert!(matches!(
            validate_and_normalize(&mut interior_entrance),
            Err(ContentError::InvalidVault(_))
        ));

        let mut theme_mismatch = artifact.content.clone();
        theme_mismatch.vaults[0].theme_id = "demo.theme.other".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut theme_mismatch),
            Err(ContentError::InvalidThemeTable(_))
        ));

        let mut no_depth_candidate = artifact.content.clone();
        for entry in &mut no_depth_candidate.vaults[0].encounter_groups[0].entries {
            entry.min_depth = 1;
            entry.max_depth = 1;
        }
        assert!(matches!(
            validate_and_normalize(&mut no_depth_candidate),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn staged_tasks_require_ordered_member_floor_objectives() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut outside_member = artifact.content.clone();
        outside_member.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-chain-rift")
            .expect("fixture should contain the staged task")
            .task_stages[1]
            .floor_id = Some("demo.floor.echo-bounty-rift".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut outside_member),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut duplicate_action_floor = artifact.content.clone();
        duplicate_action_floor.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-chain-rift")
            .expect("fixture should contain the staged task")
            .task_stages[2]
            .floor_id = Some("demo.floor.echo-chain-rift".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut duplicate_action_floor),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut non_retakeable = artifact.content.clone();
        for floor in non_retakeable.worlds[0]
            .procedural_floors
            .iter_mut()
            .filter(|floor| floor.task_id.as_deref() == Some("demo.task.echo-chain"))
        {
            floor.retakeable = false;
        }
        assert!(matches!(
            validate_and_normalize(&mut non_retakeable),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut zero_limit = artifact.content.clone();
        zero_limit.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-bounty-rift")
            .expect("fixture should contain the retakeable bounty")
            .max_retakes = Some(0);
        assert!(matches!(
            validate_and_normalize(&mut zero_limit),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut mismatched_policy = artifact.content.clone();
        mismatched_policy.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-bounty-annex-rift")
            .expect("fixture should contain the shared bounty member")
            .retake_floor_policy = RetakeFloorPolicy::PreserveFloor;
        assert!(matches!(
            validate_and_normalize(&mut mismatched_policy),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn dungeon_chains_require_one_guarded_final_floor() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut missing_guardian = artifact.content.clone();
        missing_guardian.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-3")
            .expect("fixture should contain the final floor")
            .guardian = None;
        assert!(matches!(
            validate_and_normalize(&mut missing_guardian),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut broken_chain = artifact.content.clone();
        broken_chain.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-3")
            .expect("fixture should contain the final floor")
            .dungeon_id = Some("demo.dungeon.other".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut broken_chain),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut final_with_descent = artifact.content.clone();
        let final_floor = final_with_descent.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-3")
            .expect("fixture should contain the final floor");
        final_floor.next_floor_id = Some("demo.floor.echo-depth-1".to_owned());
        final_floor.down_stair_terrain_id = Some("demo.terrain.stairs-down".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut final_with_descent),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn floor_connections_require_reciprocal_targets_and_matching_terrain_roles() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut broken_pair = artifact.content.clone();
        broken_pair.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-1")
            .expect("fixture should contain echo depth one")
            .connections
            .iter_mut()
            .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
            .expect("fixture should contain the first downward connection")
            .target_connection_id = Some("demo.connection.echo-depth-2.up-b".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut broken_pair),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut wrong_shaft_kind = artifact.content.clone();
        wrong_shaft_kind.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-1")
            .expect("fixture should contain echo depth one")
            .connections
            .iter_mut()
            .find(|connection| connection.id == "demo.connection.echo-depth-1.shaft-down")
            .expect("fixture should contain the downward shaft")
            .kind = FloorConnectionKind::Stairs;
        assert!(matches!(
            validate_and_normalize(&mut wrong_shaft_kind),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut missing_entry = artifact.content.clone();
        missing_entry.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-1")
            .expect("fixture should contain echo depth one")
            .entry_connection_id = None;
        assert!(matches!(
            validate_and_normalize(&mut missing_entry),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn door_terrain_transitions_are_reciprocal_and_match_collision() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut missing_reciprocal = artifact.content.clone();
        missing_reciprocal
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.door-closed")
            .expect("fixture should contain the closed door")
            .open_to_terrain_id = None;
        assert!(matches!(
            validate_and_normalize(&mut missing_reciprocal),
            Err(ContentError::InvalidTerrainTransition(_))
        ));

        let mut blocked_open_door = artifact.content.clone();
        blocked_open_door
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.door-open")
            .expect("fixture should contain the open door")
            .walkable = false;
        assert!(matches!(
            validate_and_normalize(&mut blocked_open_door),
            Err(ContentError::InvalidTerrainTransition(_))
        ));

        let mut incomplete_bash = artifact.content.clone();
        incomplete_bash
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.door-locked")
            .expect("fixture should contain the locked door")
            .bash_check_difficulty = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_bash),
            Err(ContentError::InvalidTerrainTransition(_))
        ));

        let mut invalid_lock = artifact.content.clone();
        invalid_lock
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.door-locked")
            .expect("fixture should contain the locked door")
            .open_check_difficulty = Some(0);
        assert!(matches!(
            validate_and_normalize(&mut invalid_lock),
            Err(ContentError::InvalidTerrainTransition(_))
        ));

        let mut incomplete_concealment = artifact.content.clone();
        incomplete_concealment
            .terrain
            .iter_mut()
            .find(|terrain| terrain.id == "demo.terrain.door-secret")
            .expect("fixture should contain the secret door")
            .search_check_difficulty = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_concealment),
            Err(ContentError::InvalidTerrainTransition(_))
        ));

        let mut non_door_generator = artifact.content.clone();
        non_door_generator.worlds[0].procedural_floors[0].closed_door_terrain_id =
            "demo.terrain.wall".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut non_door_generator),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn equippable_items_require_a_valid_slot_and_single_item_stack() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the shard");
        shard.equipment_slot = Some("charm".to_owned());

        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidEquipmentSlot(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the shard");
        shard.modifiers.max_hp = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemModifiers(_))
        ));

        let mut invalid = artifact.content.clone();
        let pellet = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-pellet")
            .expect("fixture should contain the ammunition");
        pellet.break_chance_percent = 101;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemBreakChance(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the throwable shard");
        shard.weight_tenths_pound = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemWeight(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the throwable shard");
        shard.appearance_name_key = Some(shard.name_key.clone());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemAppearance(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the usable shard");
        shard.use_action = Some(ItemUseActionDefinition {
            effect: ItemUseEffectDefinition::Heal { amount: 0 },
        });
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        invalid.affixes[0].modifiers = StatModifiers::default();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAffixModifiers(_))
        ));

        let mut invalid = artifact.content.clone();
        let charm = invalid.worlds[0]
            .items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.echo-charm")
            .expect("fixture should contain the charm");
        charm.affix_ids.push("demo.affix.harmonic-edge".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemAffixes(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid.worlds[0]
            .items
            .iter_mut()
            .find(|item| item.kind_id == "demo.item.luminous-shard")
            .expect("fixture should contain the shard");
        shard.quality = ItemQuality::Fine;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemAffixes(_))
        ));

        let mut invalid = artifact.content.clone();
        let shard = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.luminous-shard")
            .expect("fixture should contain the throwable shard");
        shard
            .throw_profile
            .as_mut()
            .expect("shard should have a throw profile")
            .damage_dice = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidThrowProfile(_))
        ));

        let mut invalid = artifact.content.clone();
        let blade = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.echo-blade")
            .expect("fixture should contain the blade");
        blade.equipment_slot = Some("charm".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAttackProfile(_))
        ));

        let mut invalid = artifact.content.clone();
        let sling = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-sling")
            .expect("fixture should contain the sling");
        sling.equipment_slot = Some("weapon".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidProjectileProfile(_))
        ));

        let mut invalid = artifact.content.clone();
        invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-sling")
            .expect("fixture should contain the sling")
            .projectile_profile
            .as_mut()
            .expect("sling should have a projectile profile")
            .ammo_kind_id = "demo.item.missing-ammunition".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::DanglingReference { .. })
        ));
    }

    #[test]
    fn player_carry_capacity_is_positive_and_monsters_cannot_declare_one() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        let player = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.role == ActorRole::Player)
            .expect("fixture should contain a player actor");
        player.carry_capacity_tenths_pound = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidActorCarryCapacity(_))
        ));

        let mut invalid = artifact.content.clone();
        let monster = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.role == ActorRole::Monster)
            .expect("fixture should contain a monster actor");
        monster.carry_capacity_tenths_pound = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidActorCarryCapacity(_))
        ));
    }

    #[test]
    fn melee_routines_require_monsters_and_valid_blow_profiles() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        let hound = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-hound")
            .expect("fixture should contain the echo hound");
        hound.role = ActorRole::Player;
        hound.carry_capacity_tenths_pound = 100;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidMeleeRoutine(_))
        ));

        let mut invalid = artifact.content;
        let hound = invalid
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-hound")
            .expect("fixture should contain the echo hound");
        hound
            .melee_routine
            .as_mut()
            .expect("hound should have a melee routine")
            .blows[0]
            .damage_dice = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidMeleeRoutine(_))
        ));
    }

    #[test]
    fn semantic_versions_are_checked_strictly() {
        assert!(validate_semver("1.2.3-alpha.1+build.5").is_ok());
        for invalid in ["01.2.3", "1.2", "1.2.3-", "1.2.3+", "1.2.3-alpha..1"] {
            assert!(matches!(
                validate_semver(invalid),
                Err(ContentError::InvalidPackVersion(_))
            ));
        }
    }
}
