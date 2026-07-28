// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
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
pub const SKILL_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/skill.schema.json";
pub const SKILL_SET_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/skill-set.schema.json";
pub const RACE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/race.schema.json";
pub const CLASS_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/class.schema.json";
pub const PERSONALITY_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/personality.schema.json";
pub const BUILD_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/build.schema.json";
pub const RESOURCE_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/resource.schema.json";
pub const ABILITY_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability.schema.json";
pub const ABILITY_BOOK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability-book.schema.json";

const fn default_actor_speed() -> u16 {
    110
}

const MAGIC: &[u8; 8] = b"RFBCONT\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 8 + 32;
const MAX_SOURCE_FILE_LENGTH: usize = 1024 * 1024;
const MAX_SOURCE_TOTAL_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 32_768;
const MAX_COMPILED_PAYLOAD_LENGTH: usize = 32 * 1024 * 1024;
const SUPPORTED_ROOTS: [&str; 20] = [
    "abilities",
    "abilityBooks",
    "actors",
    "affixes",
    "builds",
    "classes",
    "encounterTables",
    "items",
    "lootTables",
    "personalities",
    "races",
    "regionTables",
    "resources",
    "skills",
    "skillSets",
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
    pub perception_check_difficulty: Option<i32>,
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
    #[serde(default)]
    pub saving_throw_difficulty: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorRole {
    Player,
    Monster,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
    Light,
    Dark,
    Confusion,
    Nether,
    Nexus,
    Sound,
    Shards,
    Chaos,
    Disenchant,
    Time,
    Mana,
    Gravity,
    Inertia,
    Plasma,
    Force,
    Nuke,
    Disintegrate,
    Storm,
    HolyFire,
    HellFire,
    Ice,
    Water,
    Psi,
}

/// Content-declared resistance tier; `normal` is expressed by omission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorResistanceLevel {
    Vulnerable,
    Resistant,
    Strong,
    Immune,
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
    #[serde(default)]
    pub experience_value: u64,
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    #[serde(default)]
    pub melee_routine: Option<MeleeRoutineDefinition>,
    #[serde(default)]
    pub awareness: Option<ActorAwarenessDefinition>,
    #[serde(default)]
    pub monster_casting: Option<MonsterCastingDefinition>,
    #[serde(default)]
    pub loot_table_id: Option<String>,
    #[serde(default)]
    pub carried_loot_table_id: Option<String>,
    #[serde(default)]
    pub corpse_item_kind_id: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorAwarenessDefinition {
    pub detection_difficulty: i32,
    pub detection_range: u16,
    #[serde(default)]
    pub starts_alerted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterCastingDefinition {
    pub frequency_percent: u8,
    #[serde(default)]
    pub smart: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_distance: Option<u8>,
    #[serde(default)]
    pub flee_hp_percent: u8,
    pub abilities: Vec<MonsterAbilityCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterAbilityCandidateDefinition {
    pub ability_id: String,
    pub weight: u32,
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
    #[serde(default)]
    pub strength: i32,
    #[serde(default)]
    pub intelligence: i32,
    #[serde(default)]
    pub wisdom: i32,
    #[serde(default)]
    pub dexterity: i32,
    #[serde(default)]
    pub constitution: i32,
    #[serde(default)]
    pub charisma: i32,
    #[serde(default)]
    pub speed: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentBonuses {
    #[serde(default)]
    pub melee_attacks: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub melee_damage: i32,
    #[serde(default)]
    pub ranged_skill: i32,
    #[serde(default)]
    pub throwing_skill: i32,
    #[serde(default)]
    pub device_skill: i32,
    #[serde(default)]
    pub saving_throw_skill: i32,
    #[serde(default)]
    pub stealth_skill: i32,
    #[serde(default)]
    pub search_skill: i32,
    #[serde(default)]
    pub perception_skill: i32,
    #[serde(default)]
    pub disarming_skill: i32,
    #[serde(default)]
    pub digging_skill: i32,
    #[serde(default)]
    pub infravision: i32,
    #[serde(default)]
    pub light_radius: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub kind: SkillKind,
    pub maximum: i32,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SkillKind {
    Disarming,
    Device,
    SavingThrow,
    Stealth,
    Search,
    Perception,
    Melee,
    Ranged,
    Throwing,
    Digging,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillSetDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub entries: Vec<SkillSetEntryDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillSetEntryDefinition {
    pub skill_id: String,
    pub base: i32,
    #[serde(default)]
    pub growth_per_ten_levels: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartingItemDefinition {
    pub item_kind_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub equipped: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default = "default_percent")]
    pub life_percent: u16,
    #[serde(default = "default_percent")]
    pub experience_percent: u16,
    #[serde(default)]
    pub base_hp: i32,
    pub skill_set_id: String,
    #[serde(default)]
    pub starting_items: Vec<StartingItemDefinition>,
    /// Equipment slot instances this race's body offers. Empty means the
    /// engine's standard body template applies.
    #[serde(default)]
    pub body_slots: Vec<BodySlotDefinition>,
    /// Intrinsic resistance tiers every member of this race carries.
    #[serde(default)]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    /// Status kind ids members of this race are innately immune to.
    #[serde(default)]
    pub status_immunities: Vec<String>,
    pub tags: Vec<String>,
}

/// One equipment slot instance on a body: `slot_type` is the item-facing
/// class (matches `ItemDefinition.equipment_slot`), `id` names the instance
/// so a body can carry several slots of the same type (e.g. two rings).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BodySlotDefinition {
    pub id: String,
    pub slot_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClassDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default = "default_percent")]
    pub life_percent: u16,
    #[serde(default = "default_percent")]
    pub experience_percent: u16,
    #[serde(default)]
    pub base_hp: i32,
    pub skill_set_id: String,
    #[serde(default)]
    pub casting_profile: Option<CastingProfileDefinition>,
    #[serde(default)]
    pub technique_profiles: Vec<TechniqueProfileDefinition>,
    #[serde(default)]
    pub device_recharge_profile: Option<DeviceRechargeProfileDefinition>,
    #[serde(default)]
    pub starting_items: Vec<StartingItemDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingAttribute {
    Intelligence,
    Wisdom,
    Charisma,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityCastingOverrideDefinition {
    pub ability_id: String,
    pub minimum_level: u16,
    pub resource_cost: u32,
    pub base_failure_percent: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_scaling: Vec<AbilityLevelScalingDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CastingProfileDefinition {
    pub resource_id: String,
    pub casting_attribute: CastingAttribute,
    pub base_capacity: u32,
    pub capacity_per_level: u32,
    pub capacity_per_attribute_index: u32,
    pub base_learning_capacity: u16,
    pub learning_capacity_per_level: u16,
    pub learning_capacity_per_attribute_index: u16,
    pub learning_capacity_cap: u16,
    pub minimum_failure_percent: u8,
    #[serde(default)]
    pub beam_chance_level_multiplier: u8,
    #[serde(default = "default_beam_chance_level_divisor")]
    pub beam_chance_level_divisor: u8,
    #[serde(default)]
    pub beam_chance_bonus: i8,
    pub ability_book_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ability_overrides: Vec<AbilityCastingOverrideDefinition>,
}

const fn default_beam_chance_level_divisor() -> u8 {
    1
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum TechniqueAttribute {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TechniqueProfileDefinition {
    pub resource_id: String,
    pub governing_attribute: TechniqueAttribute,
    pub base_capacity: u32,
    pub capacity_per_level: u32,
    pub capacity_per_attribute_index: u32,
    pub minimum_failure_percent: u8,
    pub innate_ability_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceRechargeProfileDefinition {
    pub resource_id: String,
    pub governing_attribute: TechniqueAttribute,
    pub base_capacity: u32,
    pub capacity_per_level: u32,
    pub capacity_per_attribute_index: u32,
    pub power: u16,
    pub source_item_destruction_one_in: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonalityDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default = "default_percent")]
    pub life_percent: u16,
    #[serde(default = "default_percent")]
    pub experience_percent: u16,
    #[serde(default)]
    pub base_hp: i32,
    pub skill_set_id: String,
    #[serde(default)]
    pub starting_items: Vec<StartingItemDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InitialAttributeSetDefinition {
    pub strength: u16,
    pub intelligence: u16,
    pub wisdom: u16,
    pub dexterity: u16,
    pub constitution: u16,
    pub charisma: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBuildDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub race_id: String,
    pub class_id: String,
    pub personality_id: String,
    pub attributes: InitialAttributeSetDefinition,
    #[serde(default)]
    pub starting_items: Vec<StartingItemDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    #[serde(default)]
    pub wait_recovery_amount: u32,
    #[serde(default)]
    pub rest_recovery_amount: u32,
    #[serde(default = "default_initial_fill_percent")]
    pub initial_fill_percent: u8,
    #[serde(default)]
    pub melee_hit_gain_amount: u32,
    #[serde(default)]
    pub melee_kill_gain_amount: u32,
    #[serde(default)]
    pub turn_decay_amount: u32,
    pub tags: Vec<String>,
}

const fn default_initial_fill_percent() -> u8 {
    100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityTargetModeDefinition {
    Direction,
    Position,
    Entity,
    Item,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityTargetDefinition {
    pub modes: Vec<AbilityTargetModeDefinition>,
    pub range: u16,
    pub requires_line_of_effect: bool,
}

const fn default_ability_proficiency_cap() -> u16 {
    1600
}

const fn default_ability_proficiency_gain() -> u16 {
    128
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityProficiencyDefinition {
    #[serde(default)]
    pub initial: u16,
    #[serde(default = "default_ability_proficiency_cap")]
    pub cap: u16,
    #[serde(default = "default_ability_proficiency_gain")]
    pub success_gain: u16,
    #[serde(default)]
    pub failure_gain: u16,
}

impl Default for AbilityProficiencyDefinition {
    fn default() -> Self {
        Self {
            initial: 0,
            cap: default_ability_proficiency_cap(),
            success_gain: default_ability_proficiency_gain(),
            failure_gain: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityCooldownDefinition {
    pub turns: u16,
    #[serde(default)]
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityDetectSubjectDefinition {
    #[default]
    Terrain,
    Actor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityLevelScalingField {
    DamageDice,
    DamageSides,
    DamageBonus,
    DeathRayPower,
    IdentifyPower,
    Radius,
    BeamChancePercent,
    StatusIntensity,
    StatusDurationTicks,
    StatusDurationSides,
    StatusPower,
    StatusMeleeDamage,
    ControlPower,
    GenocidePower,
    SummonMaximumLevel,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityLevelScalingCurveDefinition {
    #[default]
    Linear,
    Prorated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityGenocideScopeDefinition {
    Single,
    Glyph,
    Nearby,
}

const fn default_incoming_damage_percent() -> u8 {
    100
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityLevelScalingDefinition {
    pub effect_index: u8,
    pub field: AbilityLevelScalingField,
    pub multiplier: u32,
    pub divisor: u32,
    #[serde(default)]
    pub level_offset: u16,
    #[serde(default)]
    pub maximum: Option<u64>,
    #[serde(default)]
    pub curve: AbilityLevelScalingCurveDefinition,
    #[serde(default)]
    pub quadratic_weight: u16,
    #[serde(default)]
    pub cubic_weight: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityRandomTargetDefinition {
    #[default]
    CastTarget,
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityRandomBranchDefinition {
    pub maximum_roll: u16,
    #[serde(default)]
    pub target: AbilityRandomTargetDefinition,
    pub effect: Box<AbilityEffectDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityStatusStackingDefinition {
    Replace,
    Extend,
    KeepStrongest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum AbilityEffectDefinition {
    Damage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
    },
    AreaDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        radius: u8,
        #[serde(default)]
        target_category: Option<String>,
    },
    BeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
    },
    BoltOrBeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        beam_chance_percent: u8,
    },
    ConeDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        radius: u8,
    },
    BreathDamage {
        hp_percent: u8,
        max_damage: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        radius: u8,
    },
    CurseDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
    },
    DeathRay {
        power: u32,
    },
    TeleportAway {
        minimum_distance: u8,
    },
    DrainResource {
        amount: u32,
    },
    Amnesia,
    Teleport,
    BlinkSelf {
        radius: u8,
    },
    TeleportSelf {
        minimum_distance: u8,
    },
    TeleportTarget,
    Summon {
        actor_kind_id: String,
        count: u8,
        radius: u8,
        duration_turns: u16,
        #[serde(default)]
        hostile: bool,
    },
    SummonCategory {
        category: String,
        #[serde(default)]
        upgraded_category: Option<String>,
        #[serde(default)]
        upgrade_at_level: Option<u16>,
        maximum_level: u16,
        count_dice: u8,
        count_sides: u8,
        #[serde(default)]
        count_bonus: u8,
        #[serde(default)]
        hostile_chance_percent: u8,
        #[serde(default)]
        friendly_group_chance_percent: u8,
        #[serde(default)]
        hostile_group_chance_percent: u8,
        #[serde(default)]
        group_count_dice: u8,
        #[serde(default)]
        group_count_sides: u8,
        #[serde(default)]
        group_count_bonus: u8,
        #[serde(default)]
        allow_unique_hostile: bool,
        radius: u8,
        duration_turns: u16,
    },
    Detect {
        #[serde(default)]
        subject: AbilityDetectSubjectDefinition,
        category: String,
        radius: u8,
        #[serde(default)]
        persistent: bool,
    },
    TransformTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
        radius: u8,
    },
    ApplyStatus {
        status_kind_id: String,
        intensity: u16,
        duration_ticks: u32,
        #[serde(default)]
        duration_dice: u16,
        #[serde(default)]
        duration_sides: u32,
        stacking: AbilityStatusStackingDefinition,
        #[serde(default)]
        resistance_type: Option<ActorDamageType>,
        #[serde(default)]
        power: Option<u16>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        granted_resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        granted_brands: BTreeSet<WeaponBrand>,
        #[serde(default)]
        granted_modifiers: StatModifiers,
        #[serde(default)]
        granted_equipment_bonuses: EquipmentBonuses,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        granted_status_immunities: BTreeSet<String>,
        #[serde(default)]
        granted_race_id: Option<String>,
        #[serde(default)]
        grants_wall_passage: bool,
        #[serde(default = "default_incoming_damage_percent")]
        incoming_damage_percent: u8,
    },
    RemoveStatus {
        status_kind_id: String,
    },
    Control {
        category: String,
        power: u16,
    },
    DrainLife {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        target_category: String,
        #[serde(default = "default_ability_effect_repeat")]
        repeat: u8,
    },
    Genocide {
        scope: AbilityGenocideScopeDefinition,
        power: u16,
        #[serde(default)]
        radius: u8,
    },
    IdentifyItem {
        full_identify_power: u16,
        full_identify_roll_sides: u16,
    },
    RestoreVitality {
        life_force: u16,
    },
    AnimateDead {
        actor_kind_id: String,
        corpse_item_kind_id: String,
        radius: u8,
        count: u8,
    },
    Heal {
        amount: u32,
    },
    VisibleDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        #[serde(default)]
        target_category: Option<String>,
    },
    VisibleApplyStatus {
        status_kind_id: String,
        intensity: u16,
        duration_ticks: u32,
        stacking: AbilityStatusStackingDefinition,
        #[serde(default)]
        target_category: Option<String>,
    },
    EnchantEquippedWeapon {
        affix_id: String,
    },
    RandomChoice {
        roll_sides: u16,
        #[serde(default)]
        level_bonus_divisor: u16,
        branches: Vec<AbilityRandomBranchDefinition>,
    },
    NoOp {
        reason: String,
    },
    Sequence {
        effects: Vec<AbilityEffectDefinition>,
    },
}

const fn default_ability_effect_repeat() -> u8 {
    1
}

impl AbilityEffectDefinition {
    #[must_use]
    pub fn ordered_effects(&self) -> &[Self] {
        match self {
            Self::Sequence { effects } => effects,
            effect => std::slice::from_ref(effect),
        }
    }
}

fn ability_level_scaling_base_and_limit(
    effect: &AbilityEffectDefinition,
    field: AbilityLevelScalingField,
) -> Option<(u64, u64)> {
    match (effect, field) {
        (
            AbilityEffectDefinition::Damage { damage_dice, .. }
            | AbilityEffectDefinition::AreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::BeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::ConeDamage { damage_dice, .. }
            | AbilityEffectDefinition::CurseDamage { damage_dice, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_dice, .. }
            | AbilityEffectDefinition::DrainLife { damage_dice, .. },
            AbilityLevelScalingField::DamageDice,
        ) => Some((u64::from(*damage_dice), 100)),
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::CurseDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilityLevelScalingField::DamageSides,
        ) => Some((u64::from(*damage_sides), 10_000)),
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
            | AbilityEffectDefinition::CurseDamage { damage_bonus, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. },
            AbilityLevelScalingField::DamageBonus,
        ) => Some((u64::from(*damage_bonus), 10_000)),
        (AbilityEffectDefinition::DeathRay { power }, AbilityLevelScalingField::DeathRayPower) => {
            Some((u64::from(*power), 1_000_000))
        }
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilityLevelScalingField::IdentifyPower,
        ) => Some((u64::from(*full_identify_power), 1_000)),
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. },
            AbilityLevelScalingField::Radius,
        ) => Some((u64::from(*radius), 16)),
        (
            AbilityEffectDefinition::BoltOrBeamDamage {
                beam_chance_percent,
                ..
            },
            AbilityLevelScalingField::BeamChancePercent,
        ) => Some((u64::from(*beam_chance_percent), 100)),
        (
            AbilityEffectDefinition::ApplyStatus { intensity, .. },
            AbilityLevelScalingField::StatusIntensity,
        ) => Some((u64::from(*intensity), 1_000)),
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. },
            AbilityLevelScalingField::StatusDurationTicks,
        ) => Some((u64::from(*duration_ticks), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. },
            AbilityLevelScalingField::StatusDurationSides,
        ) => Some((u64::from(*duration_sides), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            },
            AbilityLevelScalingField::StatusPower,
        ) => Some((u64::from(*power), 1_000)),
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_equipment_bonuses,
                ..
            },
            AbilityLevelScalingField::StatusMeleeDamage,
        ) => Some((
            u64::try_from(granted_equipment_bonuses.melee_damage).ok()?,
            10_000,
        )),
        (
            AbilityEffectDefinition::Control { power, .. },
            AbilityLevelScalingField::ControlPower,
        ) => Some((u64::from(*power), 1_000)),
        (
            AbilityEffectDefinition::Genocide { power, .. },
            AbilityLevelScalingField::GenocidePower,
        ) => Some((u64::from(*power), 1_000)),
        (
            AbilityEffectDefinition::SummonCategory { maximum_level, .. },
            AbilityLevelScalingField::SummonMaximumLevel,
        ) => Some((u64::from(*maximum_level), 1_000)),
        _ => None,
    }
}

fn valid_ability_level_scaling(
    effect: &AbilityEffectDefinition,
    level_scaling: &[AbilityLevelScalingDefinition],
) -> bool {
    let mut scaling_fields = BTreeSet::new();
    level_scaling.len() <= 32
        && level_scaling.iter().all(|scaling| {
            let Some(effect) = effect
                .ordered_effects()
                .get(usize::from(scaling.effect_index))
            else {
                return false;
            };
            let Some((base, limit)) = ability_level_scaling_base_and_limit(effect, scaling.field)
            else {
                return false;
            };
            let level_delta = 100_u64.saturating_sub(u64::from(scaling.level_offset));
            let scaled = match scaling.curve {
                AbilityLevelScalingCurveDefinition::Linear => level_delta
                    .saturating_mul(u64::from(scaling.multiplier))
                    .checked_div(u64::from(scaling.divisor))
                    .and_then(|addition| base.checked_add(addition))
                    .map(|value| scaling.maximum.map_or(value, |maximum| value.min(maximum))),
                AbilityLevelScalingCurveDefinition::Prorated => {
                    base.checked_add(u64::from(scaling.multiplier))
                }
            };
            scaling.multiplier > 0
                && scaling.multiplier <= 1_000_000
                && scaling.divisor > 0
                && scaling.divisor <= 1_000_000
                && scaling.level_offset <= 100
                && match scaling.curve {
                    AbilityLevelScalingCurveDefinition::Linear => {
                        scaling.quadratic_weight == 0 && scaling.cubic_weight == 0
                    }
                    AbilityLevelScalingCurveDefinition::Prorated => {
                        scaling.divisor == 1
                            && scaling.level_offset == 0
                            && scaling.maximum.is_none()
                            && scaling.quadratic_weight <= 100
                            && scaling.cubic_weight <= 100
                    }
                }
                && scaling
                    .maximum
                    .is_none_or(|maximum| (base..=limit).contains(&maximum))
                && scaling_fields.insert((scaling.effect_index, scaling.field))
                && scaled.is_some_and(|value| value <= limit)
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub minimum_level: u16,
    pub resource_id: String,
    pub resource_cost: u32,
    pub base_failure_percent: u8,
    pub target: AbilityTargetDefinition,
    pub effect: AbilityEffectDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_scaling: Vec<AbilityLevelScalingDefinition>,
    #[serde(default)]
    pub proficiency: AbilityProficiencyDefinition,
    #[serde(default)]
    pub cooldown: Option<AbilityCooldownDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityBookDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub ability_ids: Vec<String>,
    pub tags: Vec<String>,
}

const fn default_percent() -> u16 {
    100
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SlayTarget {
    Animal,
    Evil,
    Good,
    Living,
    Human,
    Undead,
    Demon,
    Orc,
    Troll,
    Giant,
    Dragon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SlayLevel {
    Slay,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum WeaponBrand {
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum EquipmentPassive {
    SeeInvisible,
    Telepathy,
    Levitation,
    Regeneration,
    HoldLife,
    SustainStrength,
    SustainIntelligence,
    SustainWisdom,
    SustainDexterity,
    SustainConstitution,
    SustainCharisma,
    Blessed,
    EasySpell,
    DevicePower,
    Vampiric,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AffixPropertyBundleDefinition {
    #[serde(default)]
    pub modifiers: StatModifiers,
    #[serde(default)]
    pub equipment_bonuses: EquipmentBonuses,
    #[serde(default)]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    #[serde(default)]
    pub status_immunities: Vec<String>,
    #[serde(default)]
    pub slays: BTreeMap<SlayTarget, SlayLevel>,
    #[serde(default)]
    pub brands: BTreeSet<WeaponBrand>,
    #[serde(default)]
    pub passives: BTreeSet<EquipmentPassive>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AffixRollCandidateDefinition {
    pub weight: u32,
    #[serde(default)]
    pub min_depth: u16,
    #[serde(default = "default_u16_max")]
    pub max_depth: u16,
    #[serde(default)]
    pub properties: AffixPropertyBundleDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AffixRollGroupDefinition {
    pub rolls: u8,
    pub candidates: Vec<AffixRollCandidateDefinition>,
}

const fn default_u16_max() -> u16 {
    u16::MAX
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
    /// Equipment-only combat, skill, and sensory bonuses.
    #[serde(default)]
    pub equipment_bonuses: EquipmentBonuses,
    /// Defensive resistance tiers granted while the affixed item is
    /// equipped.
    #[serde(default)]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    /// Status kind ids the wearer is immune to while the affixed item is
    /// equipped.
    #[serde(default)]
    pub status_immunities: Vec<String>,
    /// Target categories receiving an original-compatible slay or kill
    /// multiplier from melee weapon dice.
    #[serde(default)]
    pub slays: BTreeMap<SlayTarget, SlayLevel>,
    /// Elemental brands multiplying melee weapon dice unless the target is
    /// immune to the matching element.
    #[serde(default)]
    pub brands: BTreeSet<WeaponBrand>,
    /// Passive capabilities granted while the affixed item is equipped.
    #[serde(default)]
    pub passives: BTreeSet<EquipmentPassive>,
    /// Generation-time weighted rolls. Results are materialized into the
    /// item instance and never recomputed while loading a save.
    #[serde(default)]
    pub roll_groups: Vec<AffixRollGroupDefinition>,
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
    Heal {
        amount: u32,
    },
    HealDice {
        dice: u16,
        sides: u16,
    },
    Damage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
    },
    Detect {
        #[serde(default)]
        subject: AbilityDetectSubjectDefinition,
        category: String,
        radius: u8,
        #[serde(default)]
        persistent: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemChargeDefinition {
    pub initial: u32,
    pub maximum: u32,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemUseActionDefinition {
    #[serde(default)]
    pub device_check_difficulty: Option<i32>,
    #[serde(default)]
    pub charges: Option<ItemChargeDefinition>,
    pub effect: ItemUseEffectDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDeviceChargeRangeDefinition {
    pub minimum: u32,
    pub maximum: u32,
    pub cost: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDeviceActivationDefinition {
    pub id: String,
    pub name_key: String,
    pub weight: u32,
    pub min_depth: u16,
    pub max_depth: u16,
    pub device_check_difficulty: i32,
    pub charges: ItemDeviceChargeRangeDefinition,
    pub target: AbilityTargetDefinition,
    pub effect: ItemUseEffectDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDeviceGenerationDefinition {
    pub activations: Vec<ItemDeviceActivationDefinition>,
    #[serde(default)]
    pub recovery: Option<ItemDeviceRecoveryDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemDeviceRecoveryDefinition {
    pub interval_ticks: u16,
    pub energy_per_mille: u16,
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
    pub equipment_bonuses: EquipmentBonuses,
    #[serde(default)]
    pub melee_profile: Option<AttackProfileDefinition>,
    #[serde(default)]
    pub projectile_profile: Option<ProjectileProfileDefinition>,
    #[serde(default)]
    pub throw_profile: Option<ThrowProfileDefinition>,
    #[serde(default)]
    pub use_action: Option<ItemUseActionDefinition>,
    #[serde(default)]
    pub device_generation: Option<ItemDeviceGenerationDefinition>,
    #[serde(default)]
    pub ability_book_id: Option<String>,
    #[serde(default)]
    pub break_chance_percent: u8,
    /// Defensive resistance tiers granted while the item is equipped.
    #[serde(default)]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    /// Status kind ids the wearer is immune to while the item is equipped.
    #[serde(default)]
    pub status_immunities: Vec<String>,
    /// Target categories receiving an original-compatible slay or kill
    /// multiplier from melee weapon dice while this item is equipped.
    #[serde(default)]
    pub slays: BTreeMap<SlayTarget, SlayLevel>,
    /// Elemental brands applied to melee weapon dice while this item is
    /// equipped.
    #[serde(default)]
    pub brands: BTreeSet<WeaponBrand>,
    /// Passive capabilities granted while this item is equipped.
    #[serde(default)]
    pub passives: BTreeSet<EquipmentPassive>,
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
    GuardPosition,
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
    #[serde(default)]
    pub entrance_position: Option<ContentPosition>,
    #[serde(default)]
    pub entrance_positions: Vec<ContentPosition>,
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
    #[serde(default)]
    pub player_build_id: Option<String>,
    pub actors: Vec<ActorSpawn>,
    pub items: Vec<ItemSpawn>,
    #[serde(default)]
    pub dungeons: Vec<DungeonDefinition>,
    #[serde(default)]
    pub campaign: Option<CampaignDefinition>,
    pub procedural_floors: Vec<ProceduralFloorDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CampaignDefinition {
    pub victory_dungeon_ids: Vec<String>,
    pub dungeon_conquest_points: u32,
    pub task_completion_points: u32,
    pub victory_bonus: u32,
    pub turn_penalty_interval: u32,
    pub turn_penalty_points: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonDefinition {
    pub id: String,
    pub root_floor_id: String,
    pub guardian_actor_kind_id: String,
    #[serde(default)]
    pub instance_lifecycle: DungeonInstanceLifecycle,
    #[serde(default)]
    pub entrance_guardian: Option<DungeonEntranceGuardianDefinition>,
    #[serde(default)]
    pub entry_requirements: Vec<DungeonEntryRequirementDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
#[derive(Default)]
pub enum DungeonInstanceLifecycle {
    #[default]
    ResetOnSurface,
    Persistent,
    TurnTtl {
        #[cfg_attr(feature = "schemas", schemars(range(min = 1)))]
        ttl_turns: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DungeonEntranceGuardianDefinition {
    pub instance_id: String,
    pub actor_kind_id: String,
    pub position: ContentPosition,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum DungeonEntryRequirementDefinition {
    TaskStatus {
        task_id: String,
        status: DungeonEntryTaskStatus,
    },
    DungeonConquered {
        dungeon_id: String,
    },
    CarriedItem {
        item_kind_id: String,
        quantity: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum DungeonEntryTaskStatus {
    Available,
    Active,
    Paused,
    Completed,
    Failed,
    Abandoned,
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
    #[serde(default)]
    pub target_candidates: Vec<ProceduralFloorConnectionCandidateDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProceduralFloorConnectionCandidateDefinition {
    pub target_floor_id: String,
    pub target_connection_id: String,
    #[cfg_attr(feature = "schemas", schemars(range(min = 1)))]
    pub weight: u16,
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
    pub resources: Vec<ResourceDefinition>,
    #[serde(default)]
    pub abilities: Vec<AbilityDefinition>,
    #[serde(default)]
    pub ability_books: Vec<AbilityBookDefinition>,
    #[serde(default)]
    pub skills: Vec<SkillDefinition>,
    #[serde(default)]
    pub skill_sets: Vec<SkillSetDefinition>,
    #[serde(default)]
    pub races: Vec<RaceDefinition>,
    #[serde(default)]
    pub classes: Vec<ClassDefinition>,
    #[serde(default)]
    pub personalities: Vec<PersonalityDefinition>,
    #[serde(default)]
    pub builds: Vec<CharacterBuildDefinition>,
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
    resources: BTreeMap<String, ResourceDefinition>,
    abilities: BTreeMap<String, AbilityDefinition>,
    ability_books: BTreeMap<String, AbilityBookDefinition>,
    skills: BTreeMap<String, SkillDefinition>,
    skill_sets: BTreeMap<String, SkillSetDefinition>,
    races: BTreeMap<String, RaceDefinition>,
    classes: BTreeMap<String, ClassDefinition>,
    personalities: BTreeMap<String, PersonalityDefinition>,
    builds: BTreeMap<String, CharacterBuildDefinition>,
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
    pub resource_count: usize,
    pub ability_count: usize,
    pub ability_book_count: usize,
    pub skill_count: usize,
    pub skill_set_count: usize,
    pub race_count: usize,
    pub class_count: usize,
    pub personality_count: usize,
    pub build_count: usize,
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
            resource_count: self.content.resources.len(),
            ability_count: self.content.abilities.len(),
            ability_book_count: self.content.ability_books.len(),
            skill_count: self.content.skills.len(),
            skill_set_count: self.content.skill_sets.len(),
            race_count: self.content.races.len(),
            class_count: self.content.classes.len(),
            personality_count: self.content.personalities.len(),
            build_count: self.content.builds.len(),
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
            resources: content
                .resources
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            abilities: content
                .abilities
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            ability_books: content
                .ability_books
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            skills: content
                .skills
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            skill_sets: content
                .skill_sets
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            races: content
                .races
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            classes: content
                .classes
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            personalities: content
                .personalities
                .into_iter()
                .map(|definition| (definition.id.clone(), definition))
                .collect(),
            builds: content
                .builds
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

    /// All actor definitions in stable id order (BTree iteration), so
    /// category filters enumerate candidates deterministically.
    pub fn actor_definitions(&self) -> impl Iterator<Item = &ActorDefinition> {
        self.actors.values()
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
    pub fn resource(&self, id: &str) -> Option<&ResourceDefinition> {
        self.resources.get(id)
    }

    #[must_use]
    pub fn ability(&self, id: &str) -> Option<&AbilityDefinition> {
        self.abilities.get(id)
    }

    pub fn abilities(&self) -> impl Iterator<Item = &AbilityDefinition> {
        self.abilities.values()
    }

    #[must_use]
    pub fn ability_book(&self, id: &str) -> Option<&AbilityBookDefinition> {
        self.ability_books.get(id)
    }

    #[must_use]
    pub fn skill(&self, id: &str) -> Option<&SkillDefinition> {
        self.skills.get(id)
    }

    #[must_use]
    pub fn skill_by_kind(&self, kind: SkillKind) -> Option<&SkillDefinition> {
        self.skills.values().find(|skill| skill.kind == kind)
    }

    #[must_use]
    pub fn skill_set(&self, id: &str) -> Option<&SkillSetDefinition> {
        self.skill_sets.get(id)
    }

    #[must_use]
    pub fn race(&self, id: &str) -> Option<&RaceDefinition> {
        self.races.get(id)
    }

    #[must_use]
    pub fn class(&self, id: &str) -> Option<&ClassDefinition> {
        self.classes.get(id)
    }

    #[must_use]
    pub fn personality(&self, id: &str) -> Option<&PersonalityDefinition> {
        self.personalities.get(id)
    }

    #[must_use]
    pub fn build(&self, id: &str) -> Option<&CharacterBuildDefinition> {
        self.builds.get(id)
    }

    pub fn builds(&self) -> impl Iterator<Item = &CharacterBuildDefinition> {
        self.builds.values()
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
        resources: load_root(root, "resources", &roots, &mut budget)?,
        abilities: load_root(root, "abilities", &roots, &mut budget)?,
        ability_books: load_root(root, "abilityBooks", &roots, &mut budget)?,
        skills: load_root(root, "skills", &roots, &mut budget)?,
        skill_sets: load_root(root, "skillSets", &roots, &mut budget)?,
        races: load_root(root, "races", &roots, &mut budget)?,
        classes: load_root(root, "classes", &roots, &mut budget)?,
        personalities: load_root(root, "personalities", &roots, &mut budget)?,
        builds: load_root(root, "builds", &roots, &mut budget)?,
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
        .resources
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .abilities
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .ability_books
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.skills.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .skill_sets
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.races.sort_by(|left, right| left.id.cmp(&right.id));
    content
        .classes
        .sort_by(|left, right| left.id.cmp(&right.id));
    content
        .personalities
        .sort_by(|left, right| left.id.cmp(&right.id));
    content.builds.sort_by(|left, right| left.id.cmp(&right.id));
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
    let mut terrain_connectability = BTreeMap::new();
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
        terrain_connectability.insert(
            terrain.id.clone(),
            terrain.walkable
                || terrain.open_to_terrain_id.is_some()
                || terrain.bash_to_terrain_id.is_some()
                || terrain.dig_to_terrain_id.is_some(),
        );
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
        if terrain.perception_check_difficulty.is_some()
            && terrain.concealed_as_terrain_id.is_none()
        {
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
            || terrain
                .perception_check_difficulty
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
                || trap
                    .saving_throw_difficulty
                    .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
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
    let mut actor_tag_values = BTreeSet::new();
    let mut actor_levels = BTreeMap::new();
    let mut actor_loot_table_ids = Vec::new();
    let mut actor_monster_casting = Vec::new();
    let mut actor_corpse_item_ids = Vec::new();
    for actor in &mut content.actors {
        require_schema(&actor.schema, ACTOR_SCHEMA, &actor.id)?;
        require_format_version(actor.format_version, &actor.id)?;
        validate_definition_id(&actor.id, "actor")?;
        validate_definition_text(&actor.id, &actor.name_key, &actor.description_key)?;
        validate_glyph(&actor.id, &actor.glyph)?;
        if actor.level > 10_000
            || actor.experience_value > 999_999_999
            || (actor.role == ActorRole::Player && actor.experience_value != 0)
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
        if actor.awareness.as_ref().is_some_and(|awareness| {
            actor.role != ActorRole::Monster
                || !(1..=1_000_000).contains(&awareness.detection_difficulty)
                || awareness.detection_range == 0
                || awareness.detection_range > 64
        }) {
            return Err(ContentError::InvalidActorStats(actor.id.clone()));
        }
        if let Some(casting) = &actor.monster_casting {
            let mut ability_ids = BTreeSet::new();
            if actor.role != ActorRole::Monster
                || !(1..=100).contains(&casting.frequency_percent)
                || casting
                    .preferred_distance
                    .is_some_and(|distance| !(2..=16).contains(&distance))
                || casting.flee_hp_percent > 99
                || casting.abilities.is_empty()
                || casting.abilities.len() > 64
                || casting.abilities.iter().any(|candidate| {
                    validate_id(&candidate.ability_id).is_err()
                        || !(1..=1_000_000).contains(&candidate.weight)
                        || !ability_ids.insert(candidate.ability_id.clone())
                })
            {
                return Err(ContentError::InvalidMonsterCasting(actor.id.clone()));
            }
            actor_monster_casting.push((actor.id.clone(), casting.clone()));
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
        if let Some(corpse_item_kind_id) = &actor.corpse_item_kind_id {
            if actor.role != ActorRole::Monster || validate_id(corpse_item_kind_id).is_err() {
                return Err(ContentError::InvalidActorStats(actor.id.clone()));
            }
            actor_corpse_item_ids.push((actor.id.clone(), corpse_item_kind_id.clone()));
        }
        normalize_tags(&actor.id, &mut actor.tags)?;
        for tag in &actor.tags {
            actor_tag_values.insert(tag.clone());
        }
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
        validate_status_immunities(&affix.id, &mut affix.status_immunities)?;
        let mut roll_substance = false;
        if affix.roll_groups.len() > 16 {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        for group in &mut affix.roll_groups {
            if group.rolls == 0
                || group.rolls > 16
                || group.candidates.is_empty()
                || group.candidates.len() > 64
            {
                return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
            }
            let mut group_substance = false;
            for candidate in &mut group.candidates {
                validate_status_immunities(&affix.id, &mut candidate.properties.status_immunities)?;
                if candidate.weight == 0
                    || candidate.weight > 1_000_000
                    || candidate.min_depth > candidate.max_depth
                    || affix_property_bundle_out_of_range(&candidate.properties)
                {
                    return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
                }
                group_substance |= candidate.properties != AffixPropertyBundleDefinition::default();
            }
            if !group_substance {
                return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
            }
            roll_substance = true;
        }
        let modifiers = &affix.modifiers;
        let has_substance = modifiers != &StatModifiers::default()
            || affix.equipment_bonuses != EquipmentBonuses::default()
            || !affix.resistances.is_empty()
            || !affix.status_immunities.is_empty()
            || !affix.slays.is_empty()
            || !affix.brands.is_empty()
            || !affix.passives.is_empty()
            || roll_substance;
        if !has_substance
            || modifiers.max_hp < -1_000_000
            || modifiers.max_hp > 1_000_000
            || modifiers.attack < -1_000_000
            || modifiers.attack > 1_000_000
            || modifiers.defense < -1_000_000
            || modifiers.defense > 1_000_000
            || !(-100..=100).contains(&modifiers.speed)
            || attribute_modifiers_out_of_range(modifiers)
            || equipment_bonuses_out_of_range(&affix.equipment_bonuses)
        {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        normalize_tags(&affix.id, &mut affix.tags)?;
        insert_definition_id(&mut all_ids, &affix.id)?;
        affix_ids.insert(affix.id.clone());
    }

    let mut resource_ids = BTreeSet::new();
    for resource in &mut content.resources {
        require_schema(&resource.schema, RESOURCE_SCHEMA, &resource.id)?;
        require_format_version(resource.format_version, &resource.id)?;
        validate_definition_id(&resource.id, "resource")?;
        validate_definition_text(&resource.id, &resource.name_key, &resource.description_key)?;
        if resource.wait_recovery_amount > 1_000_000
            || resource.rest_recovery_amount > 1_000_000
            || resource.initial_fill_percent > 100
            || resource.melee_hit_gain_amount > 1_000_000
            || resource.melee_kill_gain_amount > 1_000_000
            || resource.turn_decay_amount > 1_000_000
        {
            return Err(ContentError::InvalidResource(resource.id.clone()));
        }
        normalize_tags(&resource.id, &mut resource.tags)?;
        insert_definition_id(&mut all_ids, &resource.id)?;
        resource_ids.insert(resource.id.clone());
    }

    let mut ability_resources = BTreeMap::new();
    let mut ability_ids = BTreeSet::new();
    let mut ability_corpse_item_ids = Vec::new();
    let mut ability_race_ids = Vec::new();
    for ability in &mut content.abilities {
        require_schema(&ability.schema, ABILITY_SCHEMA, &ability.id)?;
        require_format_version(ability.format_version, &ability.id)?;
        validate_definition_id(&ability.id, "ability")?;
        validate_definition_text(&ability.id, &ability.name_key, &ability.description_key)?;
        ability.target.modes.sort();
        ability
            .level_scaling
            .sort_by_key(|scaling| (scaling.effect_index, scaling.field));
        let ordered_effects = match &mut ability.effect {
            AbilityEffectDefinition::Sequence { effects } => effects.as_mut_slice(),
            effect => std::slice::from_mut(effect),
        };
        for effect in ordered_effects {
            if let AbilityEffectDefinition::TransformTerrain {
                source_terrain_ids, ..
            } = effect
            {
                source_terrain_ids.sort();
            }
        }
        let mut modes = BTreeSet::new();
        let valid_single_effect = |effect: &AbilityEffectDefinition, effect_index: usize| {
            let has_level_scaling = |field| {
                ability.level_scaling.iter().any(|scaling| {
                    usize::from(scaling.effect_index) == effect_index && scaling.field == field
                })
            };
            match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::AreaDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    radius,
                    target_category,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (1..=16).contains(radius)
                        && target_category.as_ref().is_none_or(|category| {
                            !category.is_empty()
                                && category.len() <= 64
                                && category.bytes().all(|byte| {
                                    byte.is_ascii_lowercase()
                                        || byte.is_ascii_digit()
                                        || matches!(byte, b'-' | b'_')
                                })
                                && actor_tag_values.contains(category)
                        })
                }
                AbilityEffectDefinition::BeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::BoltOrBeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    beam_chance_percent,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && *beam_chance_percent <= 100
                }
                AbilityEffectDefinition::ConeDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    radius,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::BreathDamage {
                    hp_percent,
                    max_damage,
                    radius,
                    ..
                } => {
                    (1..=100).contains(hp_percent)
                        && (1..=10_000).contains(max_damage)
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::DeathRay { power } => {
                    (1..=1_000_000).contains(power)
                        || (*power == 0
                            && has_level_scaling(AbilityLevelScalingField::DeathRayPower))
                }
                AbilityEffectDefinition::TeleportAway { minimum_distance } => {
                    (1..=64).contains(minimum_distance)
                }
                AbilityEffectDefinition::DrainResource { amount } => {
                    (1..=1_000_000).contains(amount)
                }
                AbilityEffectDefinition::Amnesia => true,
                AbilityEffectDefinition::Teleport => true,
                AbilityEffectDefinition::BlinkSelf { radius } => (1..=10).contains(radius),
                AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
                    (1..=64).contains(minimum_distance)
                }
                AbilityEffectDefinition::TeleportTarget => true,
                AbilityEffectDefinition::Summon {
                    actor_kind_id,
                    count,
                    radius,
                    duration_turns,
                    hostile,
                } => {
                    validate_id(actor_kind_id).is_ok()
                        && (1..=8).contains(count)
                        && (1..=8).contains(radius)
                        && ((*hostile && *duration_turns <= 10_000)
                            || (!*hostile && (1..=10_000).contains(duration_turns)))
                }
                AbilityEffectDefinition::SummonCategory {
                    category,
                    upgraded_category,
                    upgrade_at_level,
                    maximum_level,
                    count_dice,
                    count_sides,
                    count_bonus,
                    hostile_chance_percent,
                    friendly_group_chance_percent,
                    hostile_group_chance_percent,
                    group_count_dice,
                    group_count_sides,
                    group_count_bonus,
                    radius,
                    duration_turns,
                    ..
                } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && actor_tag_values.contains(category)
                        && match (upgraded_category, upgrade_at_level) {
                            (None, None) => true,
                            (Some(category), Some(level)) => {
                                actor_tag_values.contains(category) && (1..=100).contains(level)
                            }
                            _ => false,
                        }
                        && ((1..=1_000).contains(maximum_level)
                            || (*maximum_level == 0
                                && has_level_scaling(AbilityLevelScalingField::SummonMaximumLevel)))
                        && (1..=8).contains(count_dice)
                        && (1..=8).contains(count_sides)
                        && u16::from(*count_dice) * u16::from(*count_sides)
                            + u16::from(*count_bonus)
                            <= 8
                        && *hostile_chance_percent <= 100
                        && *friendly_group_chance_percent <= 100
                        && *hostile_group_chance_percent <= 100
                        && if *friendly_group_chance_percent == 0
                            && *hostile_group_chance_percent == 0
                        {
                            *group_count_dice == 0
                                && *group_count_sides == 0
                                && *group_count_bonus == 0
                        } else {
                            (1..=8).contains(group_count_dice)
                                && (1..=8).contains(group_count_sides)
                                && u16::from(*group_count_dice) * u16::from(*group_count_sides)
                                    + u16::from(*group_count_bonus)
                                    <= 8
                        }
                        && (1..=8).contains(radius)
                        && *duration_turns <= 10_000
                }
                AbilityEffectDefinition::Detect {
                    subject,
                    category,
                    radius,
                    persistent,
                } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && (1..=8).contains(radius)
                        && match subject {
                            AbilityDetectSubjectDefinition::Terrain => {
                                terrain_tags.values().any(|tags| tags.contains(category))
                            }
                            AbilityDetectSubjectDefinition::Actor => {
                                !persistent && actor_tag_values.contains(category)
                            }
                        }
                }
                AbilityEffectDefinition::TransformTerrain {
                    source_terrain_ids,
                    target_terrain_id,
                    radius,
                } => {
                    !source_terrain_ids.is_empty()
                        && source_terrain_ids.len() <= 32
                        && !target_terrain_id.is_empty()
                        && *radius <= 8
                        && source_terrain_ids.windows(2).all(|pair| pair[0] != pair[1])
                        && source_terrain_ids
                            .iter()
                            .all(|source_id| source_id != target_terrain_id)
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    incoming_damage_percent,
                    ..
                } => {
                    validate_id(status_kind_id).is_ok()
                        && (1..=1_000).contains(intensity)
                        && (*duration_ticks > 0 || *duration_dice > 0)
                        && *duration_ticks <= 1_000_000
                        && *duration_dice <= 100
                        && ((*duration_dice == 0 && *duration_sides == 0)
                            || (*duration_dice > 0 && (1..=1_000_000).contains(duration_sides)))
                        && power.is_none_or(|power| (1..=1_000).contains(&power))
                        && granted_resistances.len() <= 29
                        && granted_brands.len() <= 5
                        && granted_modifiers.max_hp.abs() <= 1_000_000
                        && granted_modifiers.attack.abs() <= 1_000_000
                        && granted_modifiers.defense.abs() <= 1_000_000
                        && (-100..=100).contains(&granted_modifiers.speed)
                        && !attribute_modifiers_out_of_range(granted_modifiers)
                        && !equipment_bonuses_out_of_range(granted_equipment_bonuses)
                        && granted_status_immunities.len() <= 32
                        && granted_status_immunities
                            .iter()
                            .all(|status_id| validate_id(status_id).is_ok())
                        && granted_race_id
                            .as_ref()
                            .is_none_or(|race_id| validate_id(race_id).is_ok())
                        && (1..=100).contains(incoming_damage_percent)
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    validate_id(status_kind_id).is_ok()
                }
                AbilityEffectDefinition::Control { category, power } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && actor_tag_values.contains(category)
                        && (1..=1_000).contains(power)
                }
                AbilityEffectDefinition::DrainLife {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    target_category,
                    repeat,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && !target_category.is_empty()
                        && target_category.len() <= 64
                        && target_category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && actor_tag_values.contains(target_category)
                        && (1..=16).contains(repeat)
                }
                AbilityEffectDefinition::Genocide {
                    scope,
                    power,
                    radius,
                } => {
                    ((1..=1_000).contains(power)
                        || (*power == 0
                            && has_level_scaling(AbilityLevelScalingField::GenocidePower)))
                        && match scope {
                            AbilityGenocideScopeDefinition::Single
                            | AbilityGenocideScopeDefinition::Glyph => *radius == 0,
                            AbilityGenocideScopeDefinition::Nearby => (1..=64).contains(radius),
                        }
                }
                AbilityEffectDefinition::IdentifyItem {
                    full_identify_power,
                    full_identify_roll_sides,
                } => {
                    ((1..=1_000).contains(full_identify_power)
                        || (*full_identify_power == 0
                            && has_level_scaling(AbilityLevelScalingField::IdentifyPower)))
                        && (1..=1_000).contains(full_identify_roll_sides)
                }
                AbilityEffectDefinition::RestoreVitality { life_force } => {
                    (1..=1_000).contains(life_force)
                }
                AbilityEffectDefinition::AnimateDead {
                    actor_kind_id,
                    corpse_item_kind_id,
                    radius,
                    count,
                } => {
                    validate_id(actor_kind_id).is_ok()
                        && validate_id(corpse_item_kind_id).is_ok()
                        && (1..=8).contains(radius)
                        && (1..=8).contains(count)
                }
                AbilityEffectDefinition::Heal { amount } => (1..=1_000_000).contains(amount),
                AbilityEffectDefinition::VisibleDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    target_category,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && target_category
                            .as_ref()
                            .is_none_or(|category| actor_tag_values.contains(category))
                }
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    target_category,
                    ..
                } => {
                    validate_id(status_kind_id).is_ok()
                        && (1..=1_000).contains(intensity)
                        && (1..=1_000_000).contains(duration_ticks)
                        && target_category
                            .as_ref()
                            .is_none_or(|category| actor_tag_values.contains(category))
                }
                AbilityEffectDefinition::EnchantEquippedWeapon { affix_id } => {
                    validate_id(affix_id).is_ok()
                }
                AbilityEffectDefinition::RandomChoice { .. } => false,
                AbilityEffectDefinition::NoOp { reason } => {
                    !reason.is_empty() && reason.len() <= 128 && reason.is_ascii()
                }
                AbilityEffectDefinition::Sequence { .. } => false,
            }
        };
        let valid_effect = match &ability.effect {
            AbilityEffectDefinition::Sequence { effects } => {
                (2..=8).contains(&effects.len())
                    && effects
                        .iter()
                        .enumerate()
                        .all(|(index, effect)| valid_single_effect(effect, index))
            }
            AbilityEffectDefinition::RandomChoice {
                roll_sides,
                level_bonus_divisor,
                branches,
            } => {
                let maximum_roll = u32::from(*roll_sides)
                    + if *level_bonus_divisor == 0 {
                        0
                    } else {
                        100 / u32::from(*level_bonus_divisor)
                    };
                (2..=10_000).contains(roll_sides)
                    && (*level_bonus_divisor == 0 || *level_bonus_divisor <= 100)
                    && (2..=64).contains(&branches.len())
                    && branches.iter().all(|branch| {
                        valid_single_effect(&branch.effect, usize::MAX)
                            && match branch.target {
                                AbilityRandomTargetDefinition::SelfTarget => matches!(
                                    branch.effect.as_ref(),
                                    AbilityEffectDefinition::Heal { .. }
                                        | AbilityEffectDefinition::ApplyStatus { .. }
                                        | AbilityEffectDefinition::Summon { .. }
                                        | AbilityEffectDefinition::VisibleDamage { .. }
                                        | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                        | AbilityEffectDefinition::EnchantEquippedWeapon { .. }
                                        | AbilityEffectDefinition::NoOp { .. }
                                ),
                                AbilityRandomTargetDefinition::CastTarget => matches!(
                                    branch.effect.as_ref(),
                                    AbilityEffectDefinition::Damage { .. }
                                        | AbilityEffectDefinition::AreaDamage { .. }
                                        | AbilityEffectDefinition::BeamDamage { .. }
                                        | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                                        | AbilityEffectDefinition::ApplyStatus { .. }
                                        | AbilityEffectDefinition::DrainLife { .. }
                                        | AbilityEffectDefinition::Genocide { .. }
                                        | AbilityEffectDefinition::NoOp { .. }
                                ),
                            }
                    })
                    && branches
                        .windows(2)
                        .all(|pair| pair[0].maximum_roll < pair[1].maximum_roll)
                    && branches
                        .last()
                        .is_some_and(|branch| u32::from(branch.maximum_roll) >= maximum_roll)
            }
            effect => valid_single_effect(effect, 0),
        };
        let valid_level_scaling =
            valid_ability_level_scaling(&ability.effect, &ability.level_scaling);
        let self_targeted = ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::SelfTarget);
        let directional_effect = matches!(
            &ability.effect,
            AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. }
        );
        let self_target_rule = ability.target.modes.as_slice()
            == [AbilityTargetModeDefinition::SelfTarget]
            && ability.target.range == 0
            && !ability.target.requires_line_of_effect;
        let projectile_target_rule = !self_targeted
            && (1..=64).contains(&ability.target.range)
            && ability.target.requires_line_of_effect;
        let item_target_rule = ability.target.modes.as_slice()
            == [AbilityTargetModeDefinition::Item]
            && ability.target.range == 0
            && !ability.target.requires_line_of_effect;
        let valid_target = match &ability.effect {
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { .. }
            | AbilityEffectDefinition::ConeDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::DrainLife { .. }
            | AbilityEffectDefinition::DeathRay { .. }
            | AbilityEffectDefinition::RandomChoice { .. } => projectile_target_rule,
            AbilityEffectDefinition::Genocide { scope, .. } => match scope {
                AbilityGenocideScopeDefinition::Nearby => self_target_rule,
                AbilityGenocideScopeDefinition::Single | AbilityGenocideScopeDefinition::Glyph => {
                    projectile_target_rule
                }
            },
            AbilityEffectDefinition::IdentifyItem { .. } => item_target_rule,
            AbilityEffectDefinition::AreaDamage { .. } => {
                self_target_rule || projectile_target_rule
            }
            AbilityEffectDefinition::Control { .. } => projectile_target_rule,
            AbilityEffectDefinition::BreathDamage { .. } => projectile_target_rule,
            AbilityEffectDefinition::Teleport => {
                !self_targeted
                    && ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Position]
                    && (1..=64).contains(&ability.target.range)
                    && ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::Summon { .. }
            | AbilityEffectDefinition::SummonCategory { .. }
            | AbilityEffectDefinition::AnimateDead { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                    && ability.target.range == 0
                    && !ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::Heal { .. }
            | AbilityEffectDefinition::VisibleDamage { .. }
            | AbilityEffectDefinition::VisibleApplyStatus { .. }
            | AbilityEffectDefinition::EnchantEquippedWeapon { .. }
            | AbilityEffectDefinition::RestoreVitality { .. }
            | AbilityEffectDefinition::NoOp { .. } => self_target_rule,
            AbilityEffectDefinition::Detect { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                    && ability.target.range == 0
                    && !ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::TransformTerrain { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Position]
                    && (1..=64).contains(&ability.target.range)
                    && ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. } => {
                self_target_rule || projectile_target_rule
            }
            AbilityEffectDefinition::BlinkSelf { .. }
            | AbilityEffectDefinition::TeleportSelf { .. } => self_target_rule,
            AbilityEffectDefinition::TeleportTarget => projectile_target_rule,
            AbilityEffectDefinition::Sequence { effects } => {
                (self_target_rule
                    && effects.iter().all(|effect| {
                        matches!(
                            effect,
                            AbilityEffectDefinition::Heal { .. }
                                | AbilityEffectDefinition::ApplyStatus { .. }
                                | AbilityEffectDefinition::RemoveStatus { .. }
                                | AbilityEffectDefinition::VisibleDamage { .. }
                                | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                | AbilityEffectDefinition::NoOp { .. }
                        )
                    }))
                    || (projectile_target_rule
                        && effects.iter().all(|effect| {
                            matches!(
                                effect,
                                AbilityEffectDefinition::Damage { .. }
                                    | AbilityEffectDefinition::ApplyStatus { .. }
                                    | AbilityEffectDefinition::RemoveStatus { .. }
                            )
                        }))
            }
        };
        let directional_target = !directional_effect
            || ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Direction];
        if !(1..=100).contains(&ability.minimum_level)
            || !(1..=1_000_000).contains(&ability.resource_cost)
            || ability.base_failure_percent > 95
            || ability.proficiency.initial > ability.proficiency.cap
            || ability.proficiency.cap > 1600
            || ability
                .proficiency
                .success_gain
                .saturating_add(ability.proficiency.failure_gain)
                > 10_000
            || ability
                .cooldown
                .as_ref()
                .is_some_and(|cooldown| cooldown.turns == 0)
            || ability
                .cooldown
                .as_ref()
                .and_then(|cooldown| cooldown.group_id.as_deref())
                .is_some_and(|group_id| validate_id(group_id).is_err())
            || ability.target.modes.is_empty()
            || ability.target.modes.len() > 5
            || ability.target.modes.iter().any(|mode| !modes.insert(*mode))
            || !valid_target
            || !valid_effect
            || !valid_level_scaling
            || !directional_target
        {
            return Err(ContentError::InvalidAbility(ability.id.clone()));
        }
        require_reference(&resource_ids, &ability.resource_id, &ability.id)?;
        let referenced_effects = match &ability.effect {
            AbilityEffectDefinition::RandomChoice { branches, .. } => branches
                .iter()
                .map(|branch| branch.effect.as_ref())
                .collect::<Vec<_>>(),
            effect => vec![effect],
        };
        for effect in referenced_effects {
            if let AbilityEffectDefinition::Summon { actor_kind_id, .. } = effect {
                require_actor_role(&actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
            }
            if let AbilityEffectDefinition::EnchantEquippedWeapon { affix_id } = effect {
                require_reference(&affix_ids, affix_id, &ability.id)?;
            }
            if let AbilityEffectDefinition::ApplyStatus {
                granted_race_id: Some(race_id),
                ..
            } = effect
            {
                ability_race_ids.push((ability.id.clone(), race_id.clone()));
            }
        }
        if let AbilityEffectDefinition::Summon { actor_kind_id, .. } = &ability.effect {
            require_actor_role(&actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
        }
        if let AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            ..
        } = &ability.effect
        {
            require_actor_role(&actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
            ability_corpse_item_ids.push((ability.id.clone(), corpse_item_kind_id.clone()));
        }
        if let AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            ..
        } = &ability.effect
        {
            for source_terrain_id in source_terrain_ids {
                require_reference(&terrain_ids, source_terrain_id, &ability.id)?;
            }
            require_reference(&terrain_ids, target_terrain_id, &ability.id)?;
        }
        normalize_tags(&ability.id, &mut ability.tags)?;
        insert_definition_id(&mut all_ids, &ability.id)?;
        ability_resources.insert(ability.id.clone(), ability.resource_id.clone());
        ability_ids.insert(ability.id.clone());
    }
    for (actor_id, casting) in actor_monster_casting {
        for candidate in casting.abilities {
            let Some(ability) = content
                .abilities
                .iter()
                .find(|ability| ability.id == candidate.ability_id)
            else {
                return Err(ContentError::DanglingReference {
                    owner: actor_id.clone(),
                    target: candidate.ability_id,
                });
            };
            let self_target = ability.target.modes.as_slice()
                == [AbilityTargetModeDefinition::SelfTarget]
                && ability.target.range == 0
                && !ability.target.requires_line_of_effect;
            let projectile_target = ability
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::Entity)
                && !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                && ability.target.requires_line_of_effect;
            let supported = match &ability.effect {
                AbilityEffectDefinition::Damage { .. }
                | AbilityEffectDefinition::AreaDamage { .. }
                | AbilityEffectDefinition::BeamDamage { .. }
                | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                | AbilityEffectDefinition::CurseDamage { .. }
                | AbilityEffectDefinition::TeleportAway { .. }
                | AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia => projectile_target,
                AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. } => {
                    ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Direction]
                        && ability.target.requires_line_of_effect
                }
                AbilityEffectDefinition::Heal { .. }
                | AbilityEffectDefinition::Summon { .. }
                | AbilityEffectDefinition::SummonCategory { .. } => self_target,
                AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. } => self_target || projectile_target,
                AbilityEffectDefinition::BlinkSelf { .. }
                | AbilityEffectDefinition::TeleportSelf { .. } => self_target,
                AbilityEffectDefinition::TeleportTarget => projectile_target,
                AbilityEffectDefinition::DrainLife { .. }
                | AbilityEffectDefinition::Genocide { .. } => projectile_target,
                AbilityEffectDefinition::AnimateDead { .. } => self_target,
                AbilityEffectDefinition::Sequence { effects } => {
                    (self_target
                        && effects.iter().all(|effect| {
                            matches!(
                                effect,
                                AbilityEffectDefinition::Heal { .. }
                                    | AbilityEffectDefinition::ApplyStatus { .. }
                                    | AbilityEffectDefinition::RemoveStatus { .. }
                            )
                        }))
                        || (projectile_target
                            && effects.iter().all(|effect| {
                                matches!(
                                    effect,
                                    AbilityEffectDefinition::Damage { .. }
                                        | AbilityEffectDefinition::ApplyStatus { .. }
                                        | AbilityEffectDefinition::RemoveStatus { .. }
                                )
                            }))
                }
                _ => false,
            };
            if !supported {
                return Err(ContentError::InvalidMonsterCasting(actor_id.clone()));
            }
        }
    }

    let mut ability_books_by_id = BTreeMap::new();
    let mut ability_book_ids = BTreeSet::new();
    for book in &mut content.ability_books {
        require_schema(&book.schema, ABILITY_BOOK_SCHEMA, &book.id)?;
        require_format_version(book.format_version, &book.id)?;
        validate_definition_id(&book.id, "ability-book")?;
        validate_definition_text(&book.id, &book.name_key, &book.description_key)?;
        book.ability_ids.sort();
        let mut members = BTreeSet::new();
        if book.ability_ids.is_empty()
            || book.ability_ids.len() > 64
            || book
                .ability_ids
                .iter()
                .any(|ability_id| !members.insert(ability_id.clone()))
        {
            return Err(ContentError::InvalidAbilityBook(book.id.clone()));
        }
        for ability_id in &book.ability_ids {
            require_reference(&ability_ids, ability_id, &book.id)?;
        }
        normalize_tags(&book.id, &mut book.tags)?;
        insert_definition_id(&mut all_ids, &book.id)?;
        ability_book_ids.insert(book.id.clone());
        ability_books_by_id.insert(book.id.clone(), book.clone());
    }

    let valid_item_effect = |effect: &ItemUseEffectDefinition| match effect {
        ItemUseEffectDefinition::Heal { amount } => (1..=1_000_000).contains(amount),
        ItemUseEffectDefinition::HealDice { dice, sides } => {
            (1..=100).contains(dice) && (1..=10_000).contains(sides)
        }
        ItemUseEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            ..
        } => {
            (1..=100).contains(damage_dice)
                && (1..=10_000).contains(damage_sides)
                && *damage_bonus <= 10_000
        }
        ItemUseEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
        } => {
            !category.is_empty()
                && category.len() <= 64
                && category.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
                && (1..=8).contains(radius)
                && match subject {
                    AbilityDetectSubjectDefinition::Terrain => {
                        terrain_tags.values().any(|tags| tags.contains(category))
                    }
                    AbilityDetectSubjectDefinition::Actor => {
                        !persistent && actor_tag_values.contains(category)
                    }
                }
        }
    };
    let valid_item_effect_target =
        |effect: &ItemUseEffectDefinition, target: &AbilityTargetDefinition| {
            let mut modes = BTreeSet::new();
            let modes_are_unique =
                target.modes.iter().all(|mode| modes.insert(*mode)) && !target.modes.is_empty();
            let self_target = target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                && target.range == 0
                && !target.requires_line_of_effect;
            let projectile_target = !target
                .modes
                .contains(&AbilityTargetModeDefinition::SelfTarget)
                && target.modes.iter().all(|mode| {
                    matches!(
                        mode,
                        AbilityTargetModeDefinition::Direction
                            | AbilityTargetModeDefinition::Position
                            | AbilityTargetModeDefinition::Entity
                    )
                })
                && (1..=64).contains(&target.range)
                && target.requires_line_of_effect;
            modes_are_unique
                && match effect {
                    ItemUseEffectDefinition::Heal { .. }
                    | ItemUseEffectDefinition::HealDice { .. }
                    | ItemUseEffectDefinition::Detect { .. } => self_target,
                    ItemUseEffectDefinition::Damage { .. } => projectile_target,
                }
        };

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
            || !(-100..=100).contains(&item.modifiers.speed)
            || attribute_modifiers_out_of_range(&item.modifiers)
            || equipment_bonuses_out_of_range(&item.equipment_bonuses)
            || (item.equipment_slot.is_none()
                && (item.modifiers != StatModifiers::default()
                    || item.equipment_bonuses != EquipmentBonuses::default()))
        {
            return Err(ContentError::InvalidItemModifiers(item.id.clone()));
        }
        validate_status_immunities(&item.id, &mut item.status_immunities)?;
        if item.equipment_slot.is_none()
            && (!item.resistances.is_empty()
                || !item.status_immunities.is_empty()
                || !item.slays.is_empty()
                || !item.brands.is_empty()
                || !item.passives.is_empty())
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
                ItemUseEffectDefinition::HealDice { dice, sides } => {
                    (1..=100).contains(&dice) && (1..=10_000).contains(&sides)
                }
                ItemUseEffectDefinition::Damage { .. } | ItemUseEffectDefinition::Detect { .. } => {
                    false
                }
            };
            let valid_charges = action.charges.is_none_or(|charges| {
                charges.maximum > 0
                    && charges.maximum <= 1_000_000
                    && charges.initial <= charges.maximum
                    && charges.cost > 0
                    && charges.cost <= charges.maximum
            });
            if item.equipment_slot.is_some()
                || !valid_effect
                || !valid_charges
                || action
                    .device_check_difficulty
                    .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
                || (action.device_check_difficulty.is_some()
                    && !item.tags.iter().any(|tag| tag == "device"))
                || (action.charges.is_some()
                    && (item.max_stack != 1
                        || action.device_check_difficulty.is_none()
                        || !item.tags.iter().any(|tag| tag == "device")))
            {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(generation) = &mut item.device_generation {
            generation
                .activations
                .sort_by(|left, right| left.id.cmp(&right.id));
            let mut activation_ids = BTreeSet::new();
            let valid_activations = (1..=256).contains(&generation.activations.len())
                && generation.activations.iter().all(|activation| {
                    activation_ids.insert(activation.id.clone())
                        && validate_id(&activation.id).is_ok()
                        && validate_message_key(&activation.name_key).is_ok()
                        && (1..=1_000_000).contains(&activation.weight)
                        && (1..=100).contains(&activation.min_depth)
                        && activation.min_depth <= activation.max_depth
                        && activation.max_depth <= 100
                        && (1..=1_000_000).contains(&activation.device_check_difficulty)
                        && (1..=1_000_000).contains(&activation.charges.minimum)
                        && activation.charges.minimum <= activation.charges.maximum
                        && activation.charges.maximum <= 1_000_000
                        && (1..=activation.charges.minimum).contains(&activation.charges.cost)
                        && valid_item_effect(&activation.effect)
                        && valid_item_effect_target(&activation.effect, &activation.target)
                })
                && (1..=100).all(|depth| {
                    generation.activations.iter().any(|activation| {
                        activation.min_depth <= depth && depth <= activation.max_depth
                    })
                });
            if item.use_action.is_some()
                || item.equipment_slot.is_some()
                || item.max_stack != 1
                || !item.tags.iter().any(|tag| tag == "device")
                || generation.recovery.is_some_and(|recovery| {
                    !(1..=10_000).contains(&recovery.interval_ticks)
                        || !(1..=1_000).contains(&recovery.energy_per_mille)
                })
                || !valid_activations
            {
                return Err(ContentError::InvalidItemUseAction(item.id.clone()));
            }
        }
        if let Some(ability_book_id) = &item.ability_book_id {
            if item.max_stack != 1
                || item.equipment_slot.is_some()
                || item.use_action.is_some()
                || item.device_generation.is_some()
            {
                return Err(ContentError::InvalidAbilityBookItem(item.id.clone()));
            }
            require_reference(&ability_book_ids, ability_book_id, &item.id)?;
        }
        normalize_tags(&item.id, &mut item.tags)?;
        insert_definition_id(&mut all_ids, &item.id)?;
        item_limits.insert(
            item.id.clone(),
            (item.max_stack, item.equipment_slot.is_some()),
        );
    }

    for (owner, corpse_item_kind_id) in actor_corpse_item_ids
        .into_iter()
        .chain(ability_corpse_item_ids)
    {
        if !item_limits.contains_key(&corpse_item_kind_id) {
            return Err(ContentError::DanglingReference {
                owner,
                target: corpse_item_kind_id,
            });
        }
        let corpse_item = content
            .items
            .iter()
            .find(|item| item.id == corpse_item_kind_id)
            .expect("validated corpse item must remain available");
        if corpse_item.equipment_slot.is_some()
            || corpse_item.max_stack != 1
            || !corpse_item.tags.iter().any(|tag| tag == "corpse")
        {
            return Err(ContentError::InvalidItemModifiers(corpse_item.id.clone()));
        }
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

    let item_starting_metadata = content
        .items
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                (item.max_stack, item.equipment_slot.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut skill_maxima = BTreeMap::new();
    let mut skill_kinds = BTreeSet::new();
    for skill in &mut content.skills {
        require_schema(&skill.schema, SKILL_SCHEMA, &skill.id)?;
        require_format_version(skill.format_version, &skill.id)?;
        validate_definition_id(&skill.id, "skill")?;
        validate_definition_text(&skill.id, &skill.name_key, &skill.description_key)?;
        if !(1..=1_000_000).contains(&skill.maximum) || !skill_kinds.insert(skill.kind) {
            return Err(ContentError::InvalidSkill(skill.id.clone()));
        }
        normalize_tags(&skill.id, &mut skill.tags)?;
        insert_definition_id(&mut all_ids, &skill.id)?;
        skill_maxima.insert(skill.id.clone(), skill.maximum);
    }
    for (required, kind, name) in [
        (
            content.items.iter().any(|item| {
                item.use_action
                    .as_ref()
                    .is_some_and(|action| action.device_check_difficulty.is_some())
            }),
            SkillKind::Device,
            "device",
        ),
        (
            content.terrain.iter().any(|terrain| {
                terrain
                    .trap
                    .as_ref()
                    .is_some_and(|trap| trap.saving_throw_difficulty.is_some())
            }),
            SkillKind::SavingThrow,
            "saving-throw",
        ),
        (
            content.actors.iter().any(|actor| actor.awareness.is_some()),
            SkillKind::Stealth,
            "stealth",
        ),
        (
            content
                .terrain
                .iter()
                .any(|terrain| terrain.perception_check_difficulty.is_some()),
            SkillKind::Perception,
            "perception",
        ),
    ] {
        if required && !skill_kinds.contains(&kind) {
            return Err(ContentError::MissingRequiredSkillKind(name.to_owned()));
        }
    }

    let mut skill_sets_by_id = BTreeMap::new();
    for skill_set in &mut content.skill_sets {
        require_schema(&skill_set.schema, SKILL_SET_SCHEMA, &skill_set.id)?;
        require_format_version(skill_set.format_version, &skill_set.id)?;
        validate_definition_id(&skill_set.id, "skill-set")?;
        skill_set
            .entries
            .sort_by(|left, right| left.skill_id.cmp(&right.skill_id));
        if skill_set.entries.len() > 64 {
            return Err(ContentError::InvalidSkillSet(skill_set.id.clone()));
        }
        let mut skill_ids = BTreeSet::new();
        for entry in &skill_set.entries {
            let Some(maximum) = skill_maxima.get(&entry.skill_id) else {
                return Err(ContentError::DanglingReference {
                    owner: skill_set.id.clone(),
                    target: entry.skill_id.clone(),
                });
            };
            if !skill_ids.insert(entry.skill_id.clone())
                || !(-1_000_000..=1_000_000).contains(&entry.base)
                || !(-1_000_000..=1_000_000).contains(&entry.growth_per_ten_levels)
                || entry.base > *maximum
            {
                return Err(ContentError::InvalidSkillSet(skill_set.id.clone()));
            }
        }
        insert_definition_id(&mut all_ids, &skill_set.id)?;
        skill_sets_by_id.insert(skill_set.id.clone(), skill_set.clone());
    }

    let mut race_ids = BTreeSet::new();
    for race in &mut content.races {
        require_schema(&race.schema, RACE_SCHEMA, &race.id)?;
        require_format_version(race.format_version, &race.id)?;
        validate_definition_id(&race.id, "race")?;
        validate_definition_text(&race.id, &race.name_key, &race.description_key)?;
        validate_character_source(
            &race.id,
            CharacterSourceValidation {
                modifiers: &race.modifiers,
                life_percent: race.life_percent,
                experience_percent: race.experience_percent,
                base_hp: race.base_hp,
                skill_set_id: &race.skill_set_id,
                starting_items: &mut race.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        if race.body_slots.len() > 64 {
            return Err(ContentError::InvalidBodySlots(race.id.clone()));
        }
        validate_status_immunities(&race.id, &mut race.status_immunities)?;
        let mut body_slot_ids = BTreeSet::new();
        for slot in &race.body_slots {
            if validate_equipment_slot(&slot.id).is_err()
                || validate_equipment_slot(&slot.slot_type).is_err()
                || !body_slot_ids.insert(slot.id.as_str())
            {
                return Err(ContentError::InvalidBodySlots(race.id.clone()));
            }
        }
        normalize_tags(&race.id, &mut race.tags)?;
        insert_definition_id(&mut all_ids, &race.id)?;
        race_ids.insert(race.id.clone());
    }
    for (owner, race_id) in ability_race_ids {
        require_reference(&race_ids, &race_id, &owner)?;
    }

    let mut class_ids = BTreeSet::new();
    for class in &mut content.classes {
        require_schema(&class.schema, CLASS_SCHEMA, &class.id)?;
        require_format_version(class.format_version, &class.id)?;
        validate_definition_id(&class.id, "class")?;
        validate_definition_text(&class.id, &class.name_key, &class.description_key)?;
        validate_character_source(
            &class.id,
            CharacterSourceValidation {
                modifiers: &class.modifiers,
                life_percent: class.life_percent,
                experience_percent: class.experience_percent,
                base_hp: class.base_hp,
                skill_set_id: &class.skill_set_id,
                starting_items: &mut class.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        if let Some(profile) = &mut class.casting_profile {
            profile.ability_book_ids.sort();
            profile
                .ability_overrides
                .sort_by(|left, right| left.ability_id.cmp(&right.ability_id));
            let mut books = BTreeSet::new();
            let mut overrides = BTreeSet::new();
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            let maximum_learning_capacity = u64::from(profile.base_learning_capacity)
                .saturating_add(u64::from(profile.learning_capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.learning_capacity_per_attribute_index).saturating_mul(100),
                );
            if profile.minimum_failure_percent > 95
                || profile.beam_chance_level_divisor == 0
                || profile.beam_chance_level_multiplier > 4
                || !(-100..=100).contains(&profile.beam_chance_bonus)
                || profile.ability_book_ids.is_empty()
                || profile.ability_book_ids.len() > 16
                || profile
                    .ability_book_ids
                    .iter()
                    .any(|book_id| !books.insert(book_id.clone()))
                || profile.ability_overrides.len() > 1_024
                || profile.ability_overrides.iter().any(|override_| {
                    !overrides.insert(override_.ability_id.clone())
                        || !(1..=100).contains(&override_.minimum_level)
                        || !(1..=1_000_000).contains(&override_.resource_cost)
                        || override_.base_failure_percent > 95
                })
                || maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || profile.learning_capacity_cap == 0
                || profile.base_learning_capacity > profile.learning_capacity_cap
                || maximum_learning_capacity > u64::from(u16::MAX)
            {
                return Err(ContentError::InvalidCastingProfile(class.id.clone()));
            }
            require_reference(&resource_ids, &profile.resource_id, &class.id)?;
            let mut supported_ability_ids = BTreeSet::new();
            for book_id in &profile.ability_book_ids {
                require_reference(&ability_book_ids, book_id, &class.id)?;
                let book = ability_books_by_id
                    .get(book_id)
                    .expect("validated ability book must remain available");
                if book.ability_ids.iter().any(|ability_id| {
                    ability_resources.get(ability_id) != Some(&profile.resource_id)
                }) {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
                supported_ability_ids.extend(book.ability_ids.iter().cloned());
            }
            if profile
                .ability_overrides
                .iter()
                .any(|override_| !supported_ability_ids.contains(&override_.ability_id))
            {
                return Err(ContentError::InvalidCastingProfile(class.id.clone()));
            }
            for override_ in &profile.ability_overrides {
                if override_.level_scaling.is_empty() {
                    continue;
                }
                let ability = content
                    .abilities
                    .iter()
                    .find(|ability| ability.id == override_.ability_id)
                    .expect("supported casting ability must remain available");
                if !valid_ability_level_scaling(&ability.effect, &override_.level_scaling) {
                    return Err(ContentError::InvalidCastingProfile(class.id.clone()));
                }
            }
        }
        let mut technique_resource_ids = class
            .casting_profile
            .as_ref()
            .map(|profile| profile.resource_id.clone())
            .into_iter()
            .collect::<BTreeSet<_>>();
        if class.technique_profiles.len() > 8 {
            return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
        }
        for profile in &mut class.technique_profiles {
            profile.innate_ability_ids.sort();
            let mut innate = BTreeSet::new();
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            if profile.minimum_failure_percent > 95
                || profile.innate_ability_ids.is_empty()
                || profile.innate_ability_ids.len() > 16
                || profile
                    .innate_ability_ids
                    .iter()
                    .any(|ability_id| !innate.insert(ability_id.clone()))
                || maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || !technique_resource_ids.insert(profile.resource_id.clone())
            {
                return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
            }
            require_reference(&resource_ids, &profile.resource_id, &class.id)?;
            for ability_id in &profile.innate_ability_ids {
                require_reference(&ability_ids, ability_id, &class.id)?;
                if ability_resources.get(ability_id) != Some(&profile.resource_id) {
                    return Err(ContentError::InvalidTechniqueProfile(class.id.clone()));
                }
            }
        }
        if let Some(profile) = &class.device_recharge_profile {
            let maximum_capacity = u64::from(profile.base_capacity)
                .saturating_add(u64::from(profile.capacity_per_level).saturating_mul(100))
                .saturating_add(
                    u64::from(profile.capacity_per_attribute_index).saturating_mul(100),
                );
            if maximum_capacity == 0
                || maximum_capacity > 1_000_000_000
                || !(1..=u16::MAX).contains(&profile.power)
                || !(2..=u16::MAX).contains(&profile.source_item_destruction_one_in)
                || !technique_resource_ids.insert(profile.resource_id.clone())
            {
                return Err(ContentError::InvalidDeviceRechargeProfile(class.id.clone()));
            }
            require_reference(&resource_ids, &profile.resource_id, &class.id)?;
        }
        normalize_tags(&class.id, &mut class.tags)?;
        insert_definition_id(&mut all_ids, &class.id)?;
        class_ids.insert(class.id.clone());
    }

    let mut personality_ids = BTreeSet::new();
    for personality in &mut content.personalities {
        require_schema(&personality.schema, PERSONALITY_SCHEMA, &personality.id)?;
        require_format_version(personality.format_version, &personality.id)?;
        validate_definition_id(&personality.id, "personality")?;
        validate_definition_text(
            &personality.id,
            &personality.name_key,
            &personality.description_key,
        )?;
        validate_character_source(
            &personality.id,
            CharacterSourceValidation {
                modifiers: &personality.modifiers,
                life_percent: personality.life_percent,
                experience_percent: personality.experience_percent,
                base_hp: personality.base_hp,
                skill_set_id: &personality.skill_set_id,
                starting_items: &mut personality.starting_items,
            },
            &skill_sets_by_id,
            &item_starting_metadata,
        )?;
        normalize_tags(&personality.id, &mut personality.tags)?;
        insert_definition_id(&mut all_ids, &personality.id)?;
        personality_ids.insert(personality.id.clone());
    }

    let races_by_id = content
        .races
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let classes_by_id = content
        .classes
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let personalities_by_id = content
        .personalities
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut build_ids = BTreeSet::new();
    for build in &mut content.builds {
        require_schema(&build.schema, BUILD_SCHEMA, &build.id)?;
        require_format_version(build.format_version, &build.id)?;
        validate_definition_id(&build.id, "build")?;
        validate_definition_text(&build.id, &build.name_key, &build.description_key)?;
        let Some(race) = races_by_id.get(build.race_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.race_id.clone(),
            });
        };
        let Some(class) = classes_by_id.get(build.class_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.class_id.clone(),
            });
        };
        let Some(personality) = personalities_by_id.get(build.personality_id.as_str()) else {
            return Err(ContentError::DanglingReference {
                owner: build.id.clone(),
                target: build.personality_id.clone(),
            });
        };
        if [
            build.attributes.strength,
            build.attributes.intelligence,
            build.attributes.wisdom,
            build.attributes.dexterity,
            build.attributes.constitution,
            build.attributes.charisma,
        ]
        .into_iter()
        .any(|value| !(3..=18).contains(&value))
        {
            return Err(ContentError::InvalidCharacterBuild(build.id.clone()));
        }
        validate_starting_items(
            &build.id,
            &mut build.starting_items,
            &item_starting_metadata,
        )?;
        validate_combined_starting_items(
            &build.id,
            race.starting_items
                .iter()
                .chain(class.starting_items.iter())
                .chain(personality.starting_items.iter())
                .chain(build.starting_items.iter()),
            &item_starting_metadata,
        )?;
        normalize_tags(&build.id, &mut build.tags)?;
        insert_definition_id(&mut all_ids, &build.id)?;
        build_ids.insert(build.id.clone());
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
        if vault.entrance_positions.is_empty() {
            if let Some(legacy_position) = vault.entrance_position.take() {
                vault.entrance_positions.push(legacy_position);
            }
        } else if vault.entrance_position.is_some() {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        vault.entrance_positions.sort();
        vault.transforms.sort();
        let transform_count = vault
            .transforms
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len();
        if !(2..=12).contains(&vault.width)
            || !(2..=12).contains(&vault.height)
            || !(1..=8).contains(&vault.entrance_positions.len())
            || vault
                .entrance_positions
                .windows(2)
                .any(|positions| positions[0] == positions[1])
            || vault.entrance_positions.iter().any(|position| {
                position.x >= vault.width
                    || position.y >= vault.height
                    || !(position.x == 0
                        || position.x + 1 == vault.width
                        || position.y == 0
                        || position.y + 1 == vault.height)
            })
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

        let connectable_positions = (0..vault.height)
            .flat_map(|y| (0..vault.width).map(move |x| ContentPosition { x, y }))
            .filter(|position| {
                let terrain_id = terrain_by_position
                    .get(position)
                    .unwrap_or(&vault.base_terrain_id);
                terrain_connectability
                    .get(terrain_id)
                    .copied()
                    .unwrap_or(false)
            })
            .collect::<BTreeSet<_>>();
        if vault
            .entrance_positions
            .iter()
            .any(|position| !connectable_positions.contains(position))
        {
            return Err(ContentError::InvalidVault(vault.id.clone()));
        }
        let mut reached = BTreeSet::new();
        let mut pending = VecDeque::from([vault.entrance_positions[0]]);
        while let Some(position) = pending.pop_front() {
            if !connectable_positions.contains(&position) || !reached.insert(position) {
                continue;
            }
            for (dx, dy) in [(0_i32, -1_i32), (1, 0), (0, 1), (-1, 0)] {
                let x = i32::from(position.x) + dx;
                let y = i32::from(position.y) + dy;
                if x >= 0 && y >= 0 && x < i32::from(vault.width) && y < i32::from(vault.height) {
                    pending.push_back(ContentPosition {
                        x: u16::try_from(x).expect("bounded Vault x must fit u16"),
                        y: u16::try_from(y).expect("bounded Vault y must fit u16"),
                    });
                }
            }
        }
        if reached != connectable_positions {
            return Err(ContentError::InvalidVault(vault.id.clone()));
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
                build_ids: &build_ids,
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
    build_ids: &'a BTreeSet<String>,
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
        build_ids,
    } = refs;
    if world.width < 3 || world.height < 3 || world.width > 512 || world.height > 512 {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    validate_definition_id(&world.initial_floor_id, "floor")?;
    let mut procedural_actor_ids = BTreeSet::new();
    let mut procedural_connection_ids = BTreeSet::new();
    world.procedural_floors.sort_by_key(|floor| floor.depth);
    world.dungeons.sort_by(|left, right| left.id.cmp(&right.id));
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
    let mut dungeon_definition_ids = BTreeSet::new();
    for dungeon in &mut world.dungeons {
        validate_definition_id(&dungeon.id, "dungeon")?;
        validate_definition_id(&dungeon.root_floor_id, "floor")?;
        dungeon.entry_requirements.sort();
        if dungeon
            .entry_requirements
            .windows(2)
            .any(|requirements| requirements[0] == requirements[1])
        {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
        if !dungeon_definition_ids.insert(dungeon.id.clone())
            || !floor_ids.contains(&dungeon.root_floor_id)
        {
            return Err(ContentError::InvalidProceduralFloor(
                dungeon.root_floor_id.clone(),
            ));
        }
        require_actor_role(
            actor_roles,
            &dungeon.guardian_actor_kind_id,
            ActorRole::Monster,
            &dungeon.id,
        )?;
        if matches!(
            dungeon.instance_lifecycle,
            DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 }
        ) {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
        if let Some(guardian) = &dungeon.entrance_guardian {
            validate_id(&guardian.instance_id)?;
            require_actor_role(
                actor_roles,
                &guardian.actor_kind_id,
                ActorRole::Monster,
                &dungeon.id,
            )?;
            validate_position(guardian.position, world.width, world.height, &dungeon.id)?;
            if !procedural_actor_ids.insert(guardian.instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(
                    guardian.instance_id.clone(),
                ));
            }
        }
    }
    if let Some(campaign) = &mut world.campaign {
        campaign.victory_dungeon_ids.sort();
        if campaign.victory_dungeon_ids.is_empty()
            || campaign.turn_penalty_interval == 0
            || campaign
                .victory_dungeon_ids
                .windows(2)
                .any(|ids| ids[0] == ids[1])
            || campaign
                .victory_dungeon_ids
                .iter()
                .any(|id| !dungeon_definition_ids.contains(id))
        {
            return Err(ContentError::InvalidProceduralFloor(world.id.clone()));
        }
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
            for candidate in &connection.target_candidates {
                if candidate.weight == 0 {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                validate_definition_id(&candidate.target_floor_id, "floor")?;
                validate_definition_id(&candidate.target_connection_id, "connection")?;
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
                    || procedural.theme_id.is_some()
                    || procedural.vault_id.is_some()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty()
                    || procedural.nest.is_some()
                    || maze_only
                    || procedural.generation_budget.is_none())
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
        let regional_groups_enabled = procedural.generation_budget.as_ref().is_some_and(|budget| {
            budget.group_placements.is_some() && budget.group_actor_slots.is_some()
        });
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
                            let mut eligible = table.entries.iter().filter(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                                    && actor_levels
                                        .get(&candidate.actor_kind_id)
                                        .is_some_and(|level| *level <= u32::from(procedural.depth))
                            });
                            let has_plain =
                                eligible.clone().any(|candidate| candidate.group.is_none());
                            let has_group = eligible.any(|candidate| candidate.group.is_some());
                            has_plain && (regional_groups_enabled == has_group)
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
                let regional_room_count =
                    room_count.saturating_sub(u16::from(pit_budget.is_some()));
                if !(2..=4).contains(&placements)
                    || placements > regional_room_count
                    || usize::from(placements) > eligible_region_entries.len()
                    || reserved_actor_slots + usize::from(placements)
                        > usize::from(budget.actor_slots)
                    || budget.loot_placements < placements
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
                let group_source_entries = if procedural.region_table_id.is_some() {
                    eligible_region_entries
                        .iter()
                        .flat_map(|region| {
                            encounter_tables[&region.encounter_table_id]
                                .entries
                                .iter()
                                .filter(|entry| {
                                    entry.min_depth <= procedural.depth
                                        && procedural.depth <= entry.max_depth
                                        && actor_levels.get(&entry.actor_kind_id).is_some_and(
                                            |level| *level <= u32::from(procedural.depth),
                                        )
                                })
                        })
                        .collect::<Vec<_>>()
                } else {
                    eligible_encounter_entries.clone()
                };
                let grouped_entries = group_source_entries
                    .iter()
                    .filter(|entry| entry.group.is_some())
                    .copied()
                    .collect::<Vec<_>>();
                let plain_entries = group_source_entries
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
                let ordinary_actor_reserve = region_budget.map_or(1, usize::from);
                let required_actor_slots = reserved_actor_slots
                    + usize::from(placements)
                    + required_companion_slots
                    + ordinary_actor_reserve;
                let encounter_rolls = if procedural.region_table_id.is_some() {
                    eligible_region_entries
                        .iter()
                        .map(|region| encounter_tables[&region.encounter_table_id].rolls)
                        .min()
                        .unwrap_or(0)
                } else {
                    procedural
                        .encounter_table_id
                        .as_ref()
                        .and_then(|table_id| encounter_tables.get(table_id))
                        .map_or(0, |table| table.rolls)
                };
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
                    let ordinary_reserve = region_budget.map_or(1, usize::from);
                    if reserved_actor_slots + vault_actor_slots + ordinary_reserve
                        > usize::from(budget.actor_slots)
                        || vault.loot_spawns.len() + ordinary_reserve
                            > usize::from(budget.loot_placements)
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
            for candidate in &connection.target_candidates {
                if candidate.target_floor_id == world.initial_floor_id
                    || !floor_ids.contains(&candidate.target_floor_id)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                let target = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == candidate.target_floor_id)
                    .expect("validated dynamic connection target must remain available");
                let Some(target_connection) = target.connections.iter().find(|target_connection| {
                    target_connection.id == candidate.target_connection_id
                }) else {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                };
                let depth_delta = target.depth.abs_diff(procedural.depth);
                if target_connection.kind != connection.kind
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
                    || !terrain_tags
                        .get(&target_connection.terrain_id)
                        .is_some_and(|tags| {
                            if target.depth > procedural.depth {
                                tags.contains("stairs-up")
                            } else {
                                tags.contains("stairs-down")
                            }
                        })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
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
    if dungeon_ids.len() != world.dungeons.len()
        || dungeon_ids
            .iter()
            .any(|dungeon_id| !dungeon_definition_ids.contains(*dungeon_id))
    {
        return Err(ContentError::InvalidProceduralFloor(world.id.clone()));
    }
    for dungeon_id in dungeon_ids {
        let dungeon = world
            .dungeons
            .iter()
            .find(|definition| definition.id == dungeon_id)
            .expect("validated dungeon definition must remain available");
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
        let Some(root) = members
            .iter()
            .find(|floor| floor.id == dungeon.root_floor_id)
            .copied()
        else {
            return Err(ContentError::InvalidProceduralFloor(members[0].id.clone()));
        };
        if roots.len() != 1 || roots[0].id != root.id || root.depth != 1 {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }

        let member_ids = members
            .iter()
            .map(|floor| floor.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut children_by_floor = BTreeMap::<&str, Vec<&str>>::new();
        let mut final_count = 0usize;
        for floor in &members {
            let mut parents = if floor.connections.is_empty() {
                (floor.return_floor_id != world.initial_floor_id)
                    .then_some(floor.return_floor_id.as_str())
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                floor
                    .connections
                    .iter()
                    .filter_map(|connection| {
                        let target = members
                            .iter()
                            .find(|candidate| candidate.id == connection.target_floor_id)?;
                        (target.depth < floor.depth).then_some(target.id.as_str())
                    })
                    .collect::<Vec<_>>()
            };
            parents.sort_unstable();
            parents.dedup();
            if (floor.id == root.id && !parents.is_empty())
                || (floor.id != root.id
                    && (parents.len() != 1 || floor.return_floor_id != parents[0]))
            {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }

            let mut children = if floor.connections.is_empty() {
                floor
                    .next_floor_id
                    .as_deref()
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                floor
                    .connections
                    .iter()
                    .filter_map(|connection| {
                        let target = members
                            .iter()
                            .find(|candidate| candidate.id == connection.target_floor_id)?;
                        (target.depth > floor.depth).then_some(target.id.as_str())
                    })
                    .collect::<Vec<_>>()
            };
            let child_count = children.len();
            children.sort_unstable();
            children.dedup();
            if children.len() != child_count
                || children.iter().any(|child| !member_ids.contains(child))
            {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }
            let is_leaf = children.is_empty();
            if floor.final_floor != is_leaf || floor.guardian.is_some() != is_leaf {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }
            if let Some(guardian) = &floor.guardian {
                final_count += 1;
                if guardian.actor_kind_id != dungeon.guardian_actor_kind_id {
                    return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
                }
            }
            children_by_floor.insert(floor.id.as_str(), children);
        }
        if final_count == 0 {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }

        let mut pending = vec![root.id.as_str()];
        let mut seen = BTreeSet::new();
        while let Some(floor_id) = pending.pop() {
            if !seen.insert(floor_id) {
                return Err(ContentError::InvalidProceduralFloor(floor_id.to_owned()));
            }
            pending.extend(
                children_by_floor
                    .get(floor_id)
                    .into_iter()
                    .flat_map(|children| children.iter().copied()),
            );
        }
        if seen.len() != members.len() {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }
    }
    let task_ids = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
        .map(|floor| floor.task_id.as_deref().unwrap_or(&floor.id))
        .collect::<BTreeSet<_>>();
    for dungeon in &world.dungeons {
        for requirement in &dungeon.entry_requirements {
            match requirement {
                DungeonEntryRequirementDefinition::TaskStatus { task_id, .. } => {
                    validate_definition_id(task_id, "task")?;
                    if !task_ids.contains(task_id.as_str()) {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
                DungeonEntryRequirementDefinition::DungeonConquered { dungeon_id } => {
                    validate_definition_id(dungeon_id, "dungeon")?;
                    if !dungeon_definition_ids.contains(dungeon_id) || dungeon_id == &dungeon.id {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
                DungeonEntryRequirementDefinition::CarriedItem {
                    item_kind_id,
                    quantity,
                } => {
                    if *quantity == 0 || !item_limits.contains_key(item_kind_id) {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
            }
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
    if let Some(build_id) = &world.player_build_id {
        require_reference(build_ids, build_id, &world.id)?;
    }
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
    for dungeon in &world.dungeons {
        let Some(guardian) = &dungeon.entrance_guardian else {
            continue;
        };
        if !actor_positions.insert(guardian.position) {
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
    for dungeon in &world.dungeons {
        let Some(guardian) = &dungeon.entrance_guardian else {
            continue;
        };
        require_walkable_spawn(
            world,
            guardian.position,
            &override_terrain,
            terrain_walkability,
        )?;
        let terrain_id = override_terrain
            .get(&guardian.position)
            .unwrap_or(&world.fill_terrain_id);
        if world.procedural_floors.iter().any(|floor| {
            floor.return_floor_id == world.initial_floor_id
                && floor.entry_terrain_id.as_deref() == Some(terrain_id.as_str())
        }) {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
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

struct CharacterSourceValidation<'a> {
    modifiers: &'a StatModifiers,
    life_percent: u16,
    experience_percent: u16,
    base_hp: i32,
    skill_set_id: &'a str,
    starting_items: &'a mut Vec<StartingItemDefinition>,
}

fn validate_character_source(
    owner_id: &str,
    source: CharacterSourceValidation<'_>,
    skill_sets: &BTreeMap<String, SkillSetDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    if source.modifiers.max_hp < -1_000_000
        || source.modifiers.max_hp > 1_000_000
        || source.modifiers.attack < -1_000_000
        || source.modifiers.attack > 1_000_000
        || source.modifiers.defense < -1_000_000
        || source.modifiers.defense > 1_000_000
        || !(-100..=100).contains(&source.modifiers.speed)
        || attribute_modifiers_out_of_range(source.modifiers)
        || !(25..=400).contains(&source.life_percent)
        || !(25..=500).contains(&source.experience_percent)
        || !(-1_000..=1_000).contains(&source.base_hp)
    {
        return Err(ContentError::InvalidCharacterSource(owner_id.to_owned()));
    }
    if !skill_sets.contains_key(source.skill_set_id) {
        return Err(ContentError::DanglingReference {
            owner: owner_id.to_owned(),
            target: source.skill_set_id.to_owned(),
        });
    }
    validate_starting_items(owner_id, source.starting_items, item_metadata)
}

fn validate_starting_items(
    owner_id: &str,
    starting_items: &mut Vec<StartingItemDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    starting_items.sort_by(|left, right| {
        left.item_kind_id
            .cmp(&right.item_kind_id)
            .then(left.equipped.cmp(&right.equipped))
    });
    if starting_items.len() > 32 {
        return Err(ContentError::InvalidStartingItems(owner_id.to_owned()));
    }
    let mut item_ids = BTreeSet::new();
    let mut equipment_slots = BTreeSet::new();
    for item in starting_items {
        let Some((max_stack, slot)) = item_metadata.get(&item.item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner: owner_id.to_owned(),
                target: item.item_kind_id.clone(),
            });
        };
        if item.quantity == 0
            || item.quantity > *max_stack
            || !item_ids.insert(item.item_kind_id.clone())
            || (item.equipped
                && (item.quantity != 1
                    || slot
                        .as_ref()
                        .is_none_or(|slot| !equipment_slots.insert(slot.clone()))))
        {
            return Err(ContentError::InvalidStartingItems(owner_id.to_owned()));
        }
    }
    Ok(())
}

fn validate_combined_starting_items<'a>(
    owner_id: &str,
    items: impl Iterator<Item = &'a StartingItemDefinition>,
    item_metadata: &BTreeMap<String, (u32, Option<String>)>,
) -> Result<(), ContentError> {
    let mut quantities = BTreeMap::<&str, u32>::new();
    let mut equipment_slots = BTreeSet::new();
    let mut count = 0_usize;
    for item in items {
        count += 1;
        let Some((max_stack, slot)) = item_metadata.get(&item.item_kind_id) else {
            return Err(ContentError::DanglingReference {
                owner: owner_id.to_owned(),
                target: item.item_kind_id.clone(),
            });
        };
        let quantity = quantities.entry(item.item_kind_id.as_str()).or_default();
        *quantity = quantity.saturating_add(item.quantity);
        if *quantity > *max_stack
            || (item.equipped
                && slot
                    .as_ref()
                    .is_none_or(|slot| !equipment_slots.insert(slot.clone())))
        {
            return Err(ContentError::InvalidCharacterBuild(owner_id.to_owned()));
        }
    }
    if count > 32 {
        return Err(ContentError::InvalidCharacterBuild(owner_id.to_owned()));
    }
    Ok(())
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

/// Status immunity lists carry engine status kind ids: normalized to a
/// sorted, unique list with a small size budget.
fn validate_status_immunities(
    owner_id: &str,
    immunities: &mut Vec<String>,
) -> Result<(), ContentError> {
    immunities.sort();
    immunities.dedup();
    if immunities.len() > 16
        || immunities.iter().any(|kind_id| {
            kind_id.is_empty()
                || kind_id.len() > 64
                || !kind_id.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'.')
                })
        })
    {
        return Err(ContentError::InvalidStatusImmunities(owner_id.to_owned()));
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

fn attribute_modifiers_out_of_range(modifiers: &StatModifiers) -> bool {
    [
        modifiers.strength,
        modifiers.intelligence,
        modifiers.wisdom,
        modifiers.dexterity,
        modifiers.constitution,
        modifiers.charisma,
    ]
    .into_iter()
    .any(|value| !(-100..=100).contains(&value))
}

fn equipment_bonuses_out_of_range(bonuses: &EquipmentBonuses) -> bool {
    !(-8..=8).contains(&bonuses.melee_attacks)
        || [
            bonuses.melee_skill,
            bonuses.melee_damage,
            bonuses.ranged_skill,
            bonuses.throwing_skill,
            bonuses.device_skill,
            bonuses.saving_throw_skill,
            bonuses.stealth_skill,
            bonuses.search_skill,
            bonuses.perception_skill,
            bonuses.disarming_skill,
            bonuses.digging_skill,
        ]
        .into_iter()
        .any(|value| !(-1_000_000..=1_000_000).contains(&value))
        || !(-64..=64).contains(&bonuses.infravision)
        || !(-8..=8).contains(&bonuses.light_radius)
}

fn affix_property_bundle_out_of_range(bundle: &AffixPropertyBundleDefinition) -> bool {
    bundle.modifiers.max_hp < -1_000_000
        || bundle.modifiers.max_hp > 1_000_000
        || bundle.modifiers.attack < -1_000_000
        || bundle.modifiers.attack > 1_000_000
        || bundle.modifiers.defense < -1_000_000
        || bundle.modifiers.defense > 1_000_000
        || !(-100..=100).contains(&bundle.modifiers.speed)
        || attribute_modifiers_out_of_range(&bundle.modifiers)
        || equipment_bonuses_out_of_range(&bundle.equipment_bonuses)
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
            "resource.schema.json",
            RESOURCE_SCHEMA,
            schema_for!(ResourceDefinition),
        )?,
        schema_document(
            "ability.schema.json",
            ABILITY_SCHEMA,
            schema_for!(AbilityDefinition),
        )?,
        schema_document(
            "ability-book.schema.json",
            ABILITY_BOOK_SCHEMA,
            schema_for!(AbilityBookDefinition),
        )?,
        schema_document(
            "skill.schema.json",
            SKILL_SCHEMA,
            schema_for!(SkillDefinition),
        )?,
        schema_document(
            "skill-set.schema.json",
            SKILL_SET_SCHEMA,
            schema_for!(SkillSetDefinition),
        )?,
        schema_document("race.schema.json", RACE_SCHEMA, schema_for!(RaceDefinition))?,
        schema_document(
            "class.schema.json",
            CLASS_SCHEMA,
            schema_for!(ClassDefinition),
        )?,
        schema_document(
            "personality.schema.json",
            PERSONALITY_SCHEMA,
            schema_for!(PersonalityDefinition),
        )?,
        schema_document(
            "build.schema.json",
            BUILD_SCHEMA,
            schema_for!(CharacterBuildDefinition),
        )?,
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
    #[error("actor monster casting profile is invalid or references an unsupported ability: {0}")]
    InvalidMonsterCasting(String),
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
    #[error("race body slots are invalid: {0}")]
    InvalidBodySlots(String),
    #[error("status immunity list is invalid: {0}")]
    InvalidStatusImmunities(String),
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
    #[error("resource definition is invalid: {0}")]
    InvalidResource(String),
    #[error("ability definition is invalid: {0}")]
    InvalidAbility(String),
    #[error("ability book definition is invalid: {0}")]
    InvalidAbilityBook(String),
    #[error("ability book item must be a single non-equippable, non-usable item: {0}")]
    InvalidAbilityBookItem(String),
    #[error("class casting profile is invalid: {0}")]
    InvalidCastingProfile(String),
    #[error("class technique profile is invalid: {0}")]
    InvalidTechniqueProfile(String),
    #[error("class device recharge profile is invalid: {0}")]
    InvalidDeviceRechargeProfile(String),
    #[error("affix stat modifiers are invalid: {0}")]
    InvalidAffixModifiers(String),
    #[error("skill definition is invalid: {0}")]
    InvalidSkill(String),
    #[error("content rule requires a missing skill kind: {0}")]
    MissingRequiredSkillKind(String),
    #[error("skill set definition is invalid: {0}")]
    InvalidSkillSet(String),
    #[error("race, class, or personality definition is invalid: {0}")]
    InvalidCharacterSource(String),
    #[error("starting item definition is invalid: {0}")]
    InvalidStartingItems(String),
    #[error("character build definition is invalid: {0}")]
    InvalidCharacterBuild(String),
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
        assert_eq!(first.content.terrain.len(), 47);
        assert_eq!(first.content.actors.len(), 28);
        assert_eq!(first.content.affixes.len(), 4);
        assert_eq!(first.content.items.len(), 23);
        assert_eq!(first.content.resources.len(), 3);
        assert_eq!(first.content.abilities.len(), 68);
        assert_eq!(first.content.ability_books.len(), 5);
        assert_eq!(first.content.skills.len(), 10);
        assert_eq!(first.content.skill_sets.len(), 13);
        assert_eq!(first.content.races.len(), 4);
        assert_eq!(first.content.classes.len(), 6);
        assert_eq!(first.content.personalities.len(), 3);
        assert_eq!(first.content.builds.len(), 6);
        assert_eq!(first.content.encounter_tables.len(), 6);
        assert_eq!(first.content.loot_tables.len(), 8);
        assert_eq!(first.content.theme_tables.len(), 3);
        assert_eq!(first.content.region_tables.len(), 1);
        assert_eq!(first.content.terrain_feature_tables.len(), 1);
        assert_eq!(first.content.vaults.len(), 6);
        assert_eq!(first.content.worlds.len(), 1);
    }

    #[test]
    fn compiled_catalog_exposes_stable_runtime_indexes() {
        let artifact =
            verify_pack_lock(&original_pack_path()).expect("original pack should verify");
        let catalog = ContentCatalog::from_bytes(&artifact.bytes).expect("catalog should decode");

        assert_eq!(catalog.pack_id(), "rfb.demo.original-v1");
        assert_eq!(catalog.pack_version(), "1.101.0");
        assert_eq!(
            catalog.resource("demo.resource.mana").map(|resource| (
                resource.name_key.as_str(),
                resource.wait_recovery_amount,
                resource.rest_recovery_amount,
            )),
            Some(("resource-demo-mana-name", 1, 3))
        );
        assert_eq!(
            catalog
                .ability_book("demo.ability-book.echo-primer")
                .map(|book| book.ability_ids.as_slice()),
            Some(
                [
                    "demo.ability.death-black-sleep".to_owned(),
                    "demo.ability.death-detect-evil".to_owned(),
                    "demo.ability.death-detect-unlife".to_owned(),
                    "demo.ability.death-enslave-undead".to_owned(),
                    "demo.ability.death-horrify".to_owned(),
                    "demo.ability.death-malediction".to_owned(),
                    "demo.ability.death-necromantic-resistance".to_owned(),
                    "demo.ability.death-stinking-cloud".to_owned(),
                    "demo.ability.echo-binding".to_owned(),
                    "demo.ability.echo-burst".to_owned(),
                    "demo.ability.echo-companion".to_owned(),
                    "demo.ability.echo-delving".to_owned(),
                    "demo.ability.echo-fan".to_owned(),
                    "demo.ability.echo-lance".to_owned(),
                    "demo.ability.echo-pulse".to_owned(),
                    "demo.ability.echo-quickening".to_owned(),
                    "demo.ability.echo-rampart".to_owned(),
                    "demo.ability.echo-sight".to_owned(),
                    "demo.ability.echo-step".to_owned(),
                    "demo.ability.harmonic-spark".to_owned(),
                    "demo.ability.resonant-bolt".to_owned(),
                ]
                .as_slice()
            )
        );
        assert_eq!(
            catalog
                .item("demo.item.echo-primer")
                .and_then(|item| item.ability_book_id.as_deref()),
            Some("demo.ability-book.echo-primer")
        );
        assert_eq!(
            catalog
                .class("demo.class.mage")
                .and_then(|class| class.casting_profile.as_ref())
                .map(|profile| (
                    profile.resource_id.as_str(),
                    profile.casting_attribute,
                    profile.base_capacity,
                    profile.capacity_per_level,
                    profile.capacity_per_attribute_index,
                    profile.base_learning_capacity,
                    profile.learning_capacity_per_level,
                    profile.learning_capacity_per_attribute_index,
                    profile.learning_capacity_cap,
                    profile.minimum_failure_percent,
                    profile.ability_book_ids.as_slice(),
                )),
            Some((
                "demo.resource.mana",
                CastingAttribute::Intelligence,
                4,
                2,
                1,
                2,
                1,
                0,
                16,
                5,
                [
                    "demo.ability-book.black-channels".to_owned(),
                    "demo.ability-book.echo-primer".to_owned(),
                    "demo.ability-book.necronomicon".to_owned(),
                    "demo.ability-book.sepulchral-ways".to_owned(),
                    "demo.ability-book.stillwater-notes".to_owned(),
                ]
                .as_slice(),
            ))
        );
        assert_eq!(
            catalog
                .class("demo.class.artificer")
                .and_then(|class| class.device_recharge_profile.as_ref())
                .map(|profile| (
                    profile.resource_id.as_str(),
                    profile.governing_attribute,
                    profile.base_capacity,
                    profile.capacity_per_level,
                    profile.capacity_per_attribute_index,
                    profile.power,
                    profile.source_item_destruction_one_in,
                )),
            Some((
                "demo.resource.resonance",
                TechniqueAttribute::Intelligence,
                8,
                2,
                1,
                90,
                3,
            ))
        );
        assert_eq!(
            catalog.build("demo.build.vanguard").map(|build| (
                build.race_id.as_str(),
                build.class_id.as_str(),
                build.personality_id.as_str(),
            )),
            Some((
                "demo.race.human",
                "demo.class.warrior",
                "demo.personality.combat",
            ))
        );
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
        assert_eq!(world.dungeons.len(), 3);
        assert_eq!(world.procedural_floors.len(), 24);
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
                .item("demo.item.resonance-stabilizer")
                .and_then(|item| item.use_action.as_ref())
                .and_then(|action| action.device_check_difficulty),
            Some(60)
        );
        assert_eq!(
            catalog
                .actor("demo.actor.echo-listener")
                .and_then(|actor| actor.awareness.as_ref())
                .map(|awareness| (
                    awareness.detection_difficulty,
                    awareness.detection_range,
                    awareness.starts_alerted,
                )),
            Some((7, 8, false))
        );
        assert_eq!(
            catalog
                .terrain("demo.terrain.echo-rune-hidden")
                .and_then(|terrain| terrain.perception_check_difficulty),
            Some(24)
        );
        assert_eq!(
            catalog
                .terrain("demo.terrain.trap-resonance-ward")
                .and_then(|terrain| terrain.trap.as_ref())
                .and_then(|trap| trap.saving_throw_difficulty),
            Some(40)
        );
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
    fn observable_rule_entries_require_their_skill_kinds() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        for (kind, expected) in [
            (SkillKind::Device, "device"),
            (SkillKind::SavingThrow, "saving-throw"),
            (SkillKind::Stealth, "stealth"),
            (SkillKind::Perception, "perception"),
        ] {
            let mut invalid = artifact.content.clone();
            invalid.skills.retain(|skill| skill.kind != kind);
            assert!(matches!(
                validate_and_normalize(&mut invalid),
                Err(ContentError::MissingRequiredSkillKind(actual)) if actual == expected
            ));
        }
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
    fn region_tables_require_depth_eligible_candidates_and_composable_budgets() {
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
        regional_floor(&mut exhausted_depth).depth = 11;
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

        let mut composable_features = artifact.content.clone();
        composable_features.terrain_feature_tables[0].entries[0].min_depth = 2;
        let floor = regional_floor(&mut composable_features);
        floor.terrain_feature_table_id =
            Some("demo.terrain-feature-table.resonance-hazards".to_owned());
        floor
            .generation_budget
            .as_mut()
            .expect("regional floor should retain a generation budget")
            .feature_placements = Some(1);
        validate_and_normalize(&mut composable_features)
            .expect("regional feature, theme, vault, and connections should compose");

        let mut missing_theme = artifact.content.clone();
        missing_theme.region_tables[0].entries[0].theme_id =
            "demo.theme.resonance-missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut missing_theme),
            Err(ContentError::InvalidRegionTable(_))
        ));

        let mut incomplete_group_budget = artifact.content.clone();
        let budget = incomplete_group_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-6")
            .and_then(|floor| floor.generation_budget.as_mut())
            .expect("fixture should contain the regional group budget");
        budget.group_actor_slots = None;
        assert!(matches!(
            validate_and_normalize(&mut incomplete_group_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut exhausted_special_actor_budget = artifact.content.clone();
        exhausted_special_actor_budget.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .and_then(|floor| floor.generation_budget.as_mut())
            .expect("fixture should contain the regional pit budget")
            .actor_slots = 27;
        assert!(matches!(
            validate_and_normalize(&mut exhausted_special_actor_budget),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut pit_consumes_too_many_rooms = artifact.content.clone();
        pit_consumes_too_many_rooms.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.resonance-depth-10")
            .and_then(|floor| floor.generation_budget.as_mut())
            .expect("fixture should contain the regional pit budget")
            .room_placements = Some(2);
        assert!(matches!(
            validate_and_normalize(&mut pit_consumes_too_many_rooms),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
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
        vault.entrance_positions = vec![ContentPosition { x: 1, y: 1 }];
        assert!(matches!(
            validate_and_normalize(&mut interior_entrance),
            Err(ContentError::InvalidVault(_))
        ));

        let mut duplicate_entrance = artifact.content.clone();
        let entrance = duplicate_entrance.vaults[0].entrance_positions[0];
        duplicate_entrance.vaults[0].entrance_positions = vec![entrance, entrance];
        assert!(matches!(
            validate_and_normalize(&mut duplicate_entrance),
            Err(ContentError::InvalidVault(_))
        ));

        let mut disconnected_interior = artifact.content.clone();
        let vault = disconnected_interior
            .vaults
            .iter_mut()
            .find(|vault| vault.id == "demo.vault.harmonic-sepulcher")
            .expect("fixture should contain the sepulcher Vault");
        vault
            .terrain_overrides
            .iter_mut()
            .find(|terrain| terrain.terrain_id == "demo.terrain.wall")
            .expect("fixture should contain Vault walls")
            .positions
            .extend((1..5).map(|x| ContentPosition { x, y: 2 }));
        assert!(matches!(
            validate_and_normalize(&mut disconnected_interior),
            Err(ContentError::InvalidVault(_))
        ));

        let mut legacy_entrance = artifact.content.clone();
        let entrance = legacy_entrance.vaults[0].entrance_positions[0];
        legacy_entrance.vaults[0].entrance_positions.clear();
        legacy_entrance.vaults[0].entrance_position = Some(entrance);
        validate_and_normalize(&mut legacy_entrance)
            .expect("legacy single Vault entrance should normalize");
        assert_eq!(legacy_entrance.vaults[0].entrance_position, None);
        assert_eq!(legacy_entrance.vaults[0].entrance_positions, [entrance]);

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
    fn dungeon_trees_require_shared_guardian_mirrors() {
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

        let mut mismatched_guardian = artifact.content.clone();
        mismatched_guardian.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
            .expect("fixture should contain a guardian mirror")
            .guardian
            .as_mut()
            .expect("mirror should retain a guardian")
            .actor_kind_id = "demo.actor.echo-hound".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut mismatched_guardian),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut converging_tree = artifact.content.clone();
        let child_parent = converging_tree.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-2-mirror")
            .expect("fixture should contain the mirror branch");
        child_parent
            .connections
            .push(ProceduralFloorConnectionDefinition {
                id: "demo.connection.test.second-parent-down".to_owned(),
                kind: FloorConnectionKind::Stairs,
                terrain_id: "demo.terrain.stairs-down".to_owned(),
                target_floor_id: "demo.floor.echo-depth-3-mirror".to_owned(),
                target_connection_id: Some("demo.connection.test.second-parent-up".to_owned()),
                target_candidates: Vec::new(),
            });
        let child = converging_tree.worlds[0]
            .procedural_floors
            .iter_mut()
            .find(|floor| floor.id == "demo.floor.echo-depth-3-mirror")
            .expect("fixture should contain the existing mirror final");
        child.connections.push(ProceduralFloorConnectionDefinition {
            id: "demo.connection.test.second-parent-up".to_owned(),
            kind: FloorConnectionKind::Stairs,
            terrain_id: "demo.terrain.stairs-up".to_owned(),
            target_floor_id: "demo.floor.echo-depth-2-mirror".to_owned(),
            target_connection_id: Some("demo.connection.test.second-parent-down".to_owned()),
            target_candidates: Vec::new(),
        });
        assert!(matches!(
            validate_and_normalize(&mut converging_tree),
            Err(ContentError::InvalidProceduralFloor(_))
        ));
    }

    #[test]
    fn dungeon_entrance_guardians_and_entry_requirements_are_validated() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let world = &artifact.content.worlds[0];
        let resonance = world
            .dungeons
            .iter()
            .find(|dungeon| dungeon.id == "demo.dungeon.resonance-descent")
            .expect("demo should contain the resonance dungeon");
        let entrance = resonance
            .entrance_guardian
            .as_ref()
            .expect("resonance should declare an entrance guardian");
        assert_eq!(entrance.position, ContentPosition { x: 2, y: 1 });
        assert!(resonance.entry_requirements.is_empty());

        let mut zero_ttl = artifact.content.clone();
        zero_ttl.worlds[0]
            .dungeons
            .iter_mut()
            .find(|dungeon| dungeon.id == "demo.dungeon.archive-depths")
            .expect("archive dungeon should remain available")
            .instance_lifecycle = DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 };
        assert!(matches!(
            validate_and_normalize(&mut zero_ttl),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut blocked_guardian = artifact.content.clone();
        blocked_guardian.worlds[0]
            .dungeons
            .iter_mut()
            .find(|dungeon| dungeon.id == resonance.id)
            .expect("resonance should remain available")
            .entrance_guardian
            .as_mut()
            .expect("entrance guardian should remain available")
            .position = ContentPosition { x: 3, y: 2 };
        assert!(matches!(
            validate_and_normalize(&mut blocked_guardian),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut duplicate_requirement = artifact.content.clone();
        let dungeon = duplicate_requirement.worlds[0]
            .dungeons
            .iter_mut()
            .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
            .expect("echo dungeon should remain available");
        let requirement = DungeonEntryRequirementDefinition::CarriedItem {
            item_kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 1,
        };
        dungeon.entry_requirements = vec![requirement.clone(), requirement];
        assert!(matches!(
            validate_and_normalize(&mut duplicate_requirement),
            Err(ContentError::InvalidProceduralFloor(_))
        ));

        let mut dangling_requirement = artifact.content.clone();
        dangling_requirement.worlds[0]
            .dungeons
            .iter_mut()
            .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
            .expect("echo dungeon should remain available")
            .entry_requirements = vec![DungeonEntryRequirementDefinition::TaskStatus {
            task_id: "demo.task.missing".to_owned(),
            status: DungeonEntryTaskStatus::Completed,
        }];
        assert!(matches!(
            validate_and_normalize(&mut dangling_requirement),
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
            device_check_difficulty: None,
            charges: None,
            effect: ItemUseEffectDefinition::Heal { amount: 0 },
        });
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        invalid.affixes[0].modifiers = StatModifiers::default();
        invalid.affixes[0].equipment_bonuses = EquipmentBonuses::default();
        invalid.affixes[0].resistances.clear();
        invalid.affixes[0].status_immunities.clear();
        invalid.affixes[0].slays.clear();
        invalid.affixes[0].brands.clear();
        invalid.affixes[0].passives.clear();
        invalid.affixes[0].roll_groups.clear();
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
    fn charged_item_actions_require_bounded_single_instance_devices() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let action = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == "demo.item.resonance-mender")
            .and_then(|item| item.use_action.as_ref())
            .expect("fixture should contain the charged device action");
        assert_eq!(
            action.charges,
            Some(ItemChargeDefinition {
                initial: 3,
                maximum: 3,
                cost: 1,
            })
        );
        assert!(matches!(
            action.effect,
            ItemUseEffectDefinition::HealDice { dice: 2, sides: 4 }
        ));

        let mut invalid = artifact.content.clone();
        let mender = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-mender")
            .expect("fixture should contain the charged device");
        mender
            .use_action
            .as_mut()
            .and_then(|action| action.charges.as_mut())
            .expect("charged action should exist")
            .maximum = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        let mender = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-mender")
            .expect("fixture should contain the charged device");
        mender.max_stack = 2;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        let mender = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-mender")
            .expect("fixture should contain the charged device");
        mender
            .use_action
            .as_mut()
            .expect("charged action should exist")
            .device_check_difficulty = None;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));
    }

    #[test]
    fn dynamic_devices_require_stable_profiles_depth_coverage_and_capacity() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let wand = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == "demo.item.resonance-wand")
            .and_then(|item| item.device_generation.as_ref())
            .expect("fixture should contain dynamic wand profiles");
        assert_eq!(
            wand.activations
                .iter()
                .map(|activation| activation.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "demo.device-activation.frost-bolt",
                "demo.device-activation.spark-bolt",
            ]
        );
        assert_eq!(
            wand.recovery,
            Some(ItemDeviceRecoveryDefinition {
                interval_ticks: 10,
                energy_per_mille: 10,
            })
        );

        let mut invalid = artifact.content.clone();
        let profiles = &mut invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-wand")
            .expect("fixture should contain the dynamic wand")
            .device_generation
            .as_mut()
            .expect("dynamic generation should exist")
            .activations;
        profiles
            .iter_mut()
            .find(|profile| profile.id == "demo.device-activation.spark-bolt")
            .expect("shallow profile should exist")
            .min_depth = 2;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-wand")
            .and_then(|item| item.device_generation.as_mut())
            .and_then(|generation| generation.recovery.as_mut())
            .expect("dynamic wand should recover")
            .energy_per_mille = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        let wand = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-wand")
            .expect("fixture should contain the dynamic wand");
        wand.device_generation
            .as_mut()
            .expect("dynamic generation should exist")
            .activations[0]
            .charges
            .cost = 1_000_001;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));

        let mut invalid = artifact.content.clone();
        let wand = invalid
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.resonance-wand")
            .expect("fixture should contain the dynamic wand");
        wand.use_action = Some(ItemUseActionDefinition {
            device_check_difficulty: None,
            charges: None,
            effect: ItemUseEffectDefinition::Heal { amount: 1 },
        });
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidItemUseAction(_))
        ));
    }

    #[test]
    fn device_recharge_profiles_require_distinct_bounded_resources() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut invalid = artifact.content.clone();
        invalid
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.artificer")
            .and_then(|class| class.device_recharge_profile.as_mut())
            .expect("artificer should recharge devices")
            .source_item_destruction_one_in = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidDeviceRechargeProfile(_))
        ));

        let mut invalid = artifact.content.clone();
        let mage = invalid
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.mage")
            .expect("mage class should exist");
        mage.device_recharge_profile = Some(DeviceRechargeProfileDefinition {
            resource_id: "demo.resource.mana".to_owned(),
            governing_attribute: TechniqueAttribute::Intelligence,
            base_capacity: 1,
            capacity_per_level: 0,
            capacity_per_attribute_index: 0,
            power: 90,
            source_item_destruction_one_in: 3,
        });
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidDeviceRechargeProfile(_))
        ));
    }

    #[test]
    fn ability_books_require_consistent_resources_items_and_casting_profiles() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut invalid_recovery = artifact.content.clone();
        invalid_recovery.resources[0].rest_recovery_amount = 1_000_001;
        assert!(matches!(
            validate_and_normalize(&mut invalid_recovery),
            Err(ContentError::InvalidResource(_))
        ));

        let mut invalid_healing_target = artifact.content.clone();
        invalid_healing_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .expect("fixture should contain the healing ability")
            .target
            .range = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid_healing_target),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_area_radius = artifact.content.clone();
        let AbilityEffectDefinition::AreaDamage { radius, .. } = &mut invalid_area_radius
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-burst")
            .expect("fixture should contain the area damage ability")
            .effect
        else {
            panic!("echo burst should use area damage");
        };
        *radius = 17;
        assert!(matches!(
            validate_and_normalize(&mut invalid_area_radius),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_beam_target = artifact.content.clone();
        invalid_beam_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-lance")
            .expect("fixture should contain the beam damage ability")
            .target
            .modes = vec![AbilityTargetModeDefinition::SelfTarget];
        assert!(matches!(
            validate_and_normalize(&mut invalid_beam_target),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_cone_radius = artifact.content.clone();
        let AbilityEffectDefinition::ConeDamage { radius, .. } = &mut invalid_cone_radius
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-fan")
            .expect("fixture should contain the cone damage ability")
            .effect
        else {
            panic!("echo fan should use cone damage");
        };
        *radius = 17;
        assert!(matches!(
            validate_and_normalize(&mut invalid_cone_radius),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_cone_target = artifact.content.clone();
        invalid_cone_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-fan")
            .expect("fixture should contain the cone damage ability")
            .target
            .modes = vec![AbilityTargetModeDefinition::Position];
        assert!(matches!(
            validate_and_normalize(&mut invalid_cone_target),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_teleport_target = artifact.content.clone();
        invalid_teleport_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-step")
            .expect("fixture should contain the teleport ability")
            .target
            .modes = vec![AbilityTargetModeDefinition::Entity];
        assert!(matches!(
            validate_and_normalize(&mut invalid_teleport_target),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_detect_category = artifact.content.clone();
        let AbilityEffectDefinition::Detect { category, .. } = &mut invalid_detect_category
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-sight")
            .expect("fixture should contain the persistent detection ability")
            .effect
        else {
            panic!("echo sight should use detection");
        };
        *category = "missing-category".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid_detect_category),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_detect_radius = artifact.content.clone();
        let AbilityEffectDefinition::Detect { radius, .. } = &mut invalid_detect_radius
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-sight")
            .expect("fixture should contain the persistent detection ability")
            .effect
        else {
            panic!("echo sight should use detection");
        };
        *radius = 9;
        assert!(matches!(
            validate_and_normalize(&mut invalid_detect_radius),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_detect_target = artifact.content.clone();
        invalid_detect_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-pulse")
            .expect("fixture should contain the transient detection ability")
            .target
            .range = 1;
        assert!(matches!(
            validate_and_normalize(&mut invalid_detect_target),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut duplicate_transform_source = artifact.content.clone();
        let AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids, ..
        } = &mut duplicate_transform_source
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-delving")
            .expect("fixture should contain the digging terrain ability")
            .effect
        else {
            panic!("echo delving should transform terrain");
        };
        source_terrain_ids.push("demo.terrain.wall".to_owned());
        assert!(matches!(
            validate_and_normalize(&mut duplicate_transform_source),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_transform_target = artifact.content.clone();
        let AbilityEffectDefinition::TransformTerrain {
            target_terrain_id, ..
        } = &mut invalid_transform_target
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-rampart")
            .expect("fixture should contain the terrain creation ability")
            .effect
        else {
            panic!("echo rampart should transform terrain");
        };
        *target_terrain_id = "demo.terrain.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut invalid_transform_target),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut invalid_transform_radius = artifact.content.clone();
        let AbilityEffectDefinition::TransformTerrain { radius, .. } =
            &mut invalid_transform_radius
                .abilities
                .iter_mut()
                .find(|ability| ability.id == "demo.ability.echo-delving")
                .expect("fixture should contain the digging terrain ability")
                .effect
        else {
            panic!("echo delving should transform terrain");
        };
        *radius = 9;
        assert!(matches!(
            validate_and_normalize(&mut invalid_transform_radius),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_transform_target_mode = artifact.content.clone();
        invalid_transform_target_mode
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-rampart")
            .expect("fixture should contain the terrain creation ability")
            .target
            .modes = vec![AbilityTargetModeDefinition::Direction];
        assert!(matches!(
            validate_and_normalize(&mut invalid_transform_target_mode),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_empty_sequence = artifact.content.clone();
        let AbilityEffectDefinition::Sequence { effects } = &mut invalid_empty_sequence
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-quickening")
            .expect("fixture should contain the self status sequence")
            .effect
        else {
            panic!("echo quickening should use an effect sequence");
        };
        effects.clear();
        assert!(matches!(
            validate_and_normalize(&mut invalid_empty_sequence),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_nested_sequence = artifact.content.clone();
        let AbilityEffectDefinition::Sequence { effects } = &mut invalid_nested_sequence
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-binding")
            .expect("fixture should contain the target status sequence")
            .effect
        else {
            panic!("echo binding should use an effect sequence");
        };
        effects[0] = AbilityEffectDefinition::Sequence {
            effects: effects.clone(),
        };
        assert!(matches!(
            validate_and_normalize(&mut invalid_nested_sequence),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_status_duration = artifact.content.clone();
        let AbilityEffectDefinition::Sequence { effects } = &mut invalid_status_duration
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-binding")
            .expect("fixture should contain the target status sequence")
            .effect
        else {
            panic!("echo binding should use an effect sequence");
        };
        let AbilityEffectDefinition::ApplyStatus { duration_ticks, .. } = &mut effects[1] else {
            panic!("echo binding should apply slow second");
        };
        *duration_ticks = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid_status_duration),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_self_sequence_member = artifact.content.clone();
        let AbilityEffectDefinition::Sequence { effects } = &mut invalid_self_sequence_member
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.echo-quickening")
            .expect("fixture should contain the self status sequence")
            .effect
        else {
            panic!("echo quickening should use an effect sequence");
        };
        effects.push(AbilityEffectDefinition::Damage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 0,
            damage_type: ActorDamageType::Physical,
        });
        assert!(matches!(
            validate_and_normalize(&mut invalid_self_sequence_member),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_proficiency = artifact.content.clone();
        invalid_proficiency.abilities[0].proficiency.cap = 1_601;
        assert!(matches!(
            validate_and_normalize(&mut invalid_proficiency),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut invalid_cooldown = artifact.content.clone();
        invalid_cooldown
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .expect("fixture should contain the healing ability")
            .cooldown
            .as_mut()
            .expect("healing ability should declare a cooldown")
            .turns = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid_cooldown),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut dangling_resource = artifact.content.clone();
        dangling_resource.abilities[0].resource_id = "demo.resource.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut dangling_resource),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut invalid_book_item = artifact.content.clone();
        let primer = invalid_book_item
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.echo-primer")
            .expect("fixture should contain the ability book item");
        primer.max_stack = 2;
        assert!(matches!(
            validate_and_normalize(&mut invalid_book_item),
            Err(ContentError::InvalidAbilityBookItem(_))
        ));

        let mut mismatched_profile = artifact.content;
        let mut focus = mismatched_profile.resources[0].clone();
        focus.id = "demo.resource.focus".to_owned();
        mismatched_profile.resources.push(focus);
        mismatched_profile
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.mage")
            .expect("fixture should contain the mage class")
            .casting_profile
            .as_mut()
            .expect("mage should have a casting profile")
            .resource_id = "demo.resource.focus".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut mismatched_profile),
            Err(ContentError::InvalidCastingProfile(_))
        ));
    }

    #[test]
    fn casting_profiles_validate_per_ability_parameter_overrides() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        let mut overridden = artifact.content;
        let profile = overridden
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.mage")
            .and_then(|class| class.casting_profile.as_mut())
            .expect("fixture should contain the mage casting profile");
        profile
            .ability_overrides
            .push(AbilityCastingOverrideDefinition {
                ability_id: "demo.ability.mending-echo".to_owned(),
                minimum_level: 7,
                resource_cost: 11,
                base_failure_percent: 42,
                level_scaling: Vec::new(),
            });
        validate_and_normalize(&mut overridden).expect("valid override should compile");

        let mut duplicate = overridden.clone();
        duplicate
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.mage")
            .and_then(|class| class.casting_profile.as_mut())
            .expect("fixture should contain the mage casting profile")
            .ability_overrides
            .push(AbilityCastingOverrideDefinition {
                ability_id: "demo.ability.mending-echo".to_owned(),
                minimum_level: 8,
                resource_cost: 12,
                base_failure_percent: 43,
                level_scaling: Vec::new(),
            });
        assert!(matches!(
            validate_and_normalize(&mut duplicate),
            Err(ContentError::InvalidCastingProfile(_))
        ));

        let mut unsupported = overridden;
        unsupported
            .classes
            .iter_mut()
            .find(|class| class.id == "demo.class.mage")
            .and_then(|class| class.casting_profile.as_mut())
            .expect("fixture should contain the mage casting profile")
            .ability_overrides[0]
            .ability_id = "demo.ability.not-in-a-mage-book".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut unsupported),
            Err(ContentError::InvalidCastingProfile(_))
        ));
    }

    #[test]
    fn abilities_validate_actor_detection_control_and_level_scaling() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut valid = artifact.content.clone();
        let malediction = valid
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.death-malediction")
            .expect("fixture should contain level-scaled damage");
        assert_eq!(malediction.level_scaling.len(), 1);
        let unlife = valid
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.death-detect-unlife")
            .expect("fixture should contain actor detection");
        assert!(matches!(
            unlife.effect,
            AbilityEffectDefinition::Detect {
                subject: AbilityDetectSubjectDefinition::Actor,
                persistent: false,
                ..
            }
        ));
        validate_and_normalize(&mut valid).expect("P54 abilities should compile");

        let mut duplicate = artifact.content.clone();
        let malediction = duplicate
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.death-malediction")
            .expect("fixture should contain level-scaled damage");
        malediction
            .level_scaling
            .push(malediction.level_scaling[0].clone());
        assert!(matches!(
            validate_and_normalize(&mut duplicate),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut out_of_bounds = artifact.content.clone();
        out_of_bounds
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.death-horrify")
            .expect("fixture should contain a scaled sequence")
            .level_scaling[0]
            .effect_index = 2;
        assert!(matches!(
            validate_and_normalize(&mut out_of_bounds),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut persistent_actor_detection = artifact.content.clone();
        let AbilityEffectDefinition::Detect { persistent, .. } = &mut persistent_actor_detection
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.death-detect-unlife")
            .expect("fixture should contain actor detection")
            .effect
        else {
            panic!("detect unlife should use actor detection");
        };
        *persistent = true;
        assert!(matches!(
            validate_and_normalize(&mut persistent_actor_detection),
            Err(ContentError::InvalidAbility(_))
        ));

        let mut missing_control_category = artifact.content;
        let AbilityEffectDefinition::Control { category, .. } = &mut missing_control_category
            .abilities
            .iter_mut()
            .find(|ability| ability.id == "demo.ability.death-enslave-undead")
            .expect("fixture should contain actor control")
            .effect
        else {
            panic!("enslave undead should use actor control");
        };
        *category = "missing-category".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut missing_control_category),
            Err(ContentError::InvalidAbility(_))
        ));
    }

    #[test]
    fn zero_ability_bases_require_matching_level_scaling() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");
        for ability_id in [
            "demo.ability.death-death-ray",
            "demo.ability.death-raise-dead",
            "demo.ability.death-esoteria",
            "demo.ability.death-mass-genocide",
        ] {
            let mut invalid = artifact.content.clone();
            invalid
                .abilities
                .iter_mut()
                .find(|ability| ability.id == ability_id)
                .unwrap_or_else(|| panic!("fixture should contain {ability_id}"))
                .level_scaling
                .clear();
            assert!(matches!(
                validate_and_normalize(&mut invalid),
                Err(ContentError::InvalidAbility(id)) if id == ability_id
            ));
        }
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
        hound.experience_value = 0;
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
    fn monster_casting_requires_weighted_supported_abilities() {
        let artifact =
            compile_pack_dir(&original_pack_path()).expect("original pack should compile");

        let mut invalid_frequency = artifact.content.clone();
        invalid_frequency
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-cantor")
            .expect("fixture should contain the echo cantor")
            .monster_casting
            .as_mut()
            .expect("echo cantor should cast")
            .frequency_percent = 0;
        assert!(matches!(
            validate_and_normalize(&mut invalid_frequency),
            Err(ContentError::InvalidMonsterCasting(_))
        ));

        let mut invalid_tactics = artifact.content.clone();
        let casting = invalid_tactics
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-cantor")
            .expect("fixture should contain the echo cantor")
            .monster_casting
            .as_mut()
            .expect("echo cantor should cast");
        casting.preferred_distance = Some(1);
        casting.flee_hp_percent = 100;
        assert!(matches!(
            validate_and_normalize(&mut invalid_tactics),
            Err(ContentError::InvalidMonsterCasting(_))
        ));

        let mut duplicate_ability = artifact.content.clone();
        let casting = duplicate_ability
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-cantor")
            .expect("fixture should contain the echo cantor")
            .monster_casting
            .as_mut()
            .expect("echo cantor should cast");
        casting.abilities.push(casting.abilities[0].clone());
        assert!(matches!(
            validate_and_normalize(&mut duplicate_ability),
            Err(ContentError::InvalidMonsterCasting(_))
        ));

        let mut dangling_ability = artifact.content.clone();
        dangling_ability
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-cantor")
            .expect("fixture should contain the echo cantor")
            .monster_casting
            .as_mut()
            .expect("echo cantor should cast")
            .abilities[0]
            .ability_id = "demo.ability.missing".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut dangling_ability),
            Err(ContentError::DanglingReference { .. })
        ));

        let mut unsupported_ability = artifact.content;
        unsupported_ability
            .actors
            .iter_mut()
            .find(|actor| actor.id == "demo.actor.echo-cantor")
            .expect("fixture should contain the echo cantor")
            .monster_casting
            .as_mut()
            .expect("echo cantor should cast")
            .abilities[0]
            .ability_id = "demo.ability.echo-step".to_owned();
        assert!(matches!(
            validate_and_normalize(&mut unsupported_ability),
            Err(ContentError::InvalidMonsterCasting(_))
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
