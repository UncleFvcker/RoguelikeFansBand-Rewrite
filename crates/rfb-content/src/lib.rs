// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

mod ability_programs;
mod artifact;
mod catalog;
mod effect_programs;
mod player_ability_bindings;
#[cfg(feature = "schemas")]
mod schemas;
mod source;

pub use ability_programs::{
    AbilityProgramDefinition, AbilityProgramInputDefinition, AbilityProgramStepDefinition,
};
use ability_programs::{SourceAbilityDefinition, compile_ability_program_catalog};
pub use artifact::{CompiledArtifact, decode_content, encode_content, read_compiled_file};
pub use catalog::{CompiledContentV1, ContentCatalog, ContentLockV1, ContentSummary};
pub use effect_programs::{
    EffectProgramDefinition, EffectProgramInputDefinition, EffectProgramStepDefinition,
};
use effect_programs::{compile_effect_program_catalog, validate_effect_program_catalog};
pub use player_ability_bindings::PlayerAbilityBindingDefinition;
use player_ability_bindings::{
    compile_player_ability_binding_catalog, validate_player_ability_binding_references,
};
#[cfg(feature = "schemas")]
pub use schemas::generated_schema_documents;
use source::{SourceItemDefinition, compile_source_item};

pub const CONTENT_FORMAT: &str = "rfb-content";
pub const CONTENT_FORMAT_VERSION: u16 = 1;
pub const PACK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/pack.schema.json";
pub const TERRAIN_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/terrain.schema.json";
pub const ACTOR_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/actor.schema.json";
pub const ITEM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/item.schema.json";
pub const EFFECT_PROGRAM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/effect-program.schema.json";
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
pub const ABILITY_PROGRAM_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability-program.schema.json";
pub const PLAYER_ABILITY_BINDING_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/player-ability-binding.schema.json";
pub const ABILITY_BOOK_SCHEMA: &str = "https://raw.githubusercontent.com/UncleFvcker/RoguelikeFansBand-Rewrite/main/schemas/content-v1/ability-book.schema.json";

const fn default_actor_speed() -> u16 {
    110
}

const MAX_SOURCE_FILE_LENGTH: usize = 1024 * 1024;
const MAX_SOURCE_TOTAL_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 32_768;
const SUPPORTED_ROOTS: [&str; 23] = [
    "abilities",
    "abilityBooks",
    "abilityPrograms",
    "actors",
    "affixes",
    "builds",
    "classes",
    "effectPrograms",
    "encounterTables",
    "items",
    "lootTables",
    "personalities",
    "playerAbilityBindings",
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
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
    /// Actor category used by player-kin summons. Omission makes that race
    /// produce an observed zero-result summon instead of guessing ancestry.
    #[serde(default)]
    pub kin_category: Option<String>,
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
    pub uses_spell_scrolls: bool,
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
    Item,
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
#[serde(rename_all = "kebab-case")]
pub enum ItemAttributeDefinition {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
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
    Regeneration,
    Vampiric,
    SustainStrength,
    SustainIntelligence,
    SustainWisdom,
    SustainDexterity,
    SustainConstitution,
    SustainCharisma,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemCurseSeverityDefinition {
    Normal,
    Heavy,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemCurseTargetDefinition {
    Weapon,
    Armor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemSummonLevelSourceDefinition {
    DungeonDepth,
    PlayerLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ItemSummonSelectorDefinition {
    AnyMonster,
    Category { category: String },
    PlayerKin,
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
    Bless {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplySlowness {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplySpeed {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyHeroism {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyBerserkStrength {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyPoeticInspiration {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyStoneSkin {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    RestoreLifeLevels {
        life_force_amount: u16,
    },
    RestoreAllAttributes,
    RestoreAllVitality {
        life_force_amount: u16,
    },
    ApplyRestorativeFeast {
        healing_dice: u16,
        healing_sides: u16,
    },
    ApplyLifeRestoration {
        healing_amount: u32,
        life_force_amount: u16,
    },
    DrainAttribute {
        attribute: ItemAttributeDefinition,
    },
    RestoreAttribute {
        attribute: ItemAttributeDefinition,
    },
    IncreaseAttribute {
        attribute: ItemAttributeDefinition,
    },
    AugmentAttributes,
    ApplyThermalResistance {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyBasicResistance {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyPoison {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyBlindness {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ApplyDetonation {
        damage_dice: u16,
        damage_sides: u16,
        stun_ticks: u32,
        bleeding_ticks: u32,
    },
    SelfLifeLoss {
        amount: u32,
    },
    Vengeance {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    ProtectionFromEvil,
    PrepareConfusingStrike,
    SelfCenteredElementalBlast {
        base_damage: u32,
        damage_type: ActorDamageType,
        radius: u8,
        backlash_sides: u16,
        backlash_bonus: u16,
        backlash_damage_type: ActorDamageType,
        backlash_uses_resistance: bool,
    },
    AggravateMonsters,
    MassGenocide {
        power: u16,
        radius: u8,
    },
    Genocide {
        power: u16,
    },
    IncreaseSpellLearningCapacity,
    RechargeFromDevice {
        power: u16,
    },
    CreateAdjacentTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
    },
    DestroyAdjacentTrapsAndDoors,
    RemoveStatus {
        status_kind_id: String,
    },
    RestoreResource {
        resource_id: String,
        amount: u32,
    },
    RestoreResourceDice {
        resource_id: String,
        dice: u16,
        sides: u16,
        #[serde(default)]
        bonus: u32,
    },
    RestoreResourceFull {
        resource_id: String,
    },
    IdentifyItem {
        #[serde(default)]
        full: bool,
    },
    EnchantItem {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_hit: Option<ItemEnchantmentRollDefinition>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_damage: Option<ItemEnchantmentRollDefinition>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_armor: Option<ItemEnchantmentRollDefinition>,
    },
    CurseEquippedItem {
        target: ItemCurseTargetDefinition,
    },
    RemoveEquippedCurses {
        #[serde(default)]
        include_heavy: bool,
    },
    SummonCategory {
        selector: ItemSummonSelectorDefinition,
        maximum_level_source: ItemSummonLevelSourceDefinition,
        count_dice: u8,
        count_sides: u8,
        #[serde(default)]
        count_bonus: u8,
        #[serde(default)]
        hostile: bool,
        #[serde(default)]
        group_chance_percent: u8,
        #[serde(default)]
        group_count_dice: u8,
        #[serde(default)]
        group_count_sides: u8,
        #[serde(default)]
        group_count_bonus: u8,
        #[serde(default)]
        allow_unique: bool,
        radius: u8,
        /// Item summons are permanent in v117; this field is fixed at zero
        /// so the shared resolver cannot create an invalid ability identity.
        duration_turns: u16,
    },
    Sequence {
        effects: Vec<Self>,
    },
    Damage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
    },
    DispelCategory {
        category: String,
        damage: u32,
    },
    BanishVisible {
        maximum_distance: u16,
    },
    Detect {
        #[serde(default)]
        subject: AbilityDetectSubjectDefinition,
        category: String,
        radius: u8,
        #[serde(default)]
        persistent: bool,
        #[serde(default)]
        through_walls: bool,
    },
    RandomTeleport {
        maximum_distance: u16,
    },
    TeleportLevel,
    Recall {
        delay_dice: u16,
        delay_sides: u16,
        #[serde(default)]
        delay_bonus: u16,
    },
    ResetRecall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemEnchantmentRollDefinition {
    pub dice: u16,
    pub sides: u16,
    #[serde(default)]
    pub bonus: u16,
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
    /// Curse stamped onto newly generated instances. Save data remains
    /// authoritative after generation and never re-derives this field.
    #[serde(default)]
    pub initial_curse: Option<ItemCurseSeverityDefinition>,
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
    let effect_programs =
        compile_effect_program_catalog(load_root(root, "effectPrograms", &roots, &mut budget)?)?;
    let items = load_root::<SourceItemDefinition>(root, "items", &roots, &mut budget)?
        .into_iter()
        .map(|item| compile_source_item(item, &effect_programs))
        .collect::<Result<Vec<_>, _>>()?;
    let ability_programs =
        compile_ability_program_catalog(load_root(root, "abilityPrograms", &roots, &mut budget)?)?;
    let player_ability_bindings = compile_player_ability_binding_catalog(load_root(
        root,
        "playerAbilityBindings",
        &roots,
        &mut budget,
    )?)?;
    let abilities = load_root::<SourceAbilityDefinition>(root, "abilities", &roots, &mut budget)?
        .into_iter()
        .map(|ability| ability.into_compiled(&ability_programs, &player_ability_bindings))
        .collect::<Result<Vec<_>, _>>()?;
    validate_player_ability_binding_references(&player_ability_bindings, &abilities)?;
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
        items,
        resources: load_root(root, "resources", &roots, &mut budget)?,
        abilities,
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
    validate_effect_program_catalog(&effect_programs, &content)?;
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

fn valid_item_effect(
    effect: &ItemUseEffectDefinition,
    terrain_tags: &BTreeMap<String, BTreeSet<String>>,
    actor_tag_values: &BTreeSet<String>,
    item_tag_values: &BTreeSet<String>,
    resource_ids: &BTreeSet<String>,
) -> bool {
    match effect {
        ItemUseEffectDefinition::Heal { amount }
        | ItemUseEffectDefinition::SelfLifeLoss { amount } => (1..=1_000_000).contains(amount),
        ItemUseEffectDefinition::ApplyDetonation {
            damage_dice,
            damage_sides,
            stun_ticks,
            bleeding_ticks,
        } => {
            (1..=100).contains(damage_dice)
                && (1..=10_000).contains(damage_sides)
                && *stun_ticks > 0
                && *bleeding_ticks > 0
        }
        ItemUseEffectDefinition::RestoreLifeLevels { life_force_amount } => {
            (1..=1_000).contains(life_force_amount)
        }
        ItemUseEffectDefinition::RestoreAllVitality { life_force_amount } => {
            (1..=1_000).contains(life_force_amount)
        }
        ItemUseEffectDefinition::ApplyRestorativeFeast {
            healing_dice,
            healing_sides,
        } => (1..=100).contains(healing_dice) && (1..=10_000).contains(healing_sides),
        ItemUseEffectDefinition::ApplyLifeRestoration {
            healing_amount,
            life_force_amount,
        } => (1..=1_000_000).contains(healing_amount) && (1..=1_000).contains(life_force_amount),
        ItemUseEffectDefinition::RestoreAllAttributes
        | ItemUseEffectDefinition::DrainAttribute { .. }
        | ItemUseEffectDefinition::RestoreAttribute { .. }
        | ItemUseEffectDefinition::IncreaseAttribute { .. }
        | ItemUseEffectDefinition::AugmentAttributes => true,
        ItemUseEffectDefinition::HealDice { dice, sides } => {
            (1..=100).contains(dice) && (1..=10_000).contains(sides)
        }
        ItemUseEffectDefinition::Bless {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplySlowness {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplySpeed {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyHeroism {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBerserkStrength {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyPoeticInspiration {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyStoneSkin {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyThermalResistance {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBasicResistance {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyPoison {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::ApplyBlindness {
            duration_dice,
            duration_sides,
            duration_bonus,
        }
        | ItemUseEffectDefinition::Vengeance {
            duration_dice,
            duration_sides,
            duration_bonus,
        } => {
            (1..=100).contains(duration_dice)
                && (1..=10_000).contains(duration_sides)
                && *duration_bonus <= 1_000_000
        }
        ItemUseEffectDefinition::SelfCenteredElementalBlast {
            base_damage,
            radius,
            backlash_sides,
            backlash_bonus,
            ..
        } => {
            (1..=1_000_000).contains(base_damage)
                && (1..=8).contains(radius)
                && (1..=10_000).contains(backlash_sides)
                && *backlash_bonus <= 10_000
        }
        ItemUseEffectDefinition::ProtectionFromEvil
        | ItemUseEffectDefinition::PrepareConfusingStrike
        | ItemUseEffectDefinition::AggravateMonsters
        | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
        | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors => true,
        ItemUseEffectDefinition::MassGenocide { power, radius } => *power > 0 && *radius > 0,
        ItemUseEffectDefinition::Genocide { power } => (1..=1_000).contains(power),
        ItemUseEffectDefinition::RechargeFromDevice { power } => (1..=1_000).contains(power),
        ItemUseEffectDefinition::CreateAdjacentTerrain {
            source_terrain_ids,
            target_terrain_id,
        } => {
            !source_terrain_ids.is_empty()
                && source_terrain_ids.len() <= 32
                && source_terrain_ids.windows(2).all(|pair| pair[0] != pair[1])
                && source_terrain_ids
                    .iter()
                    .all(|source_id| terrain_tags.contains_key(source_id))
                && terrain_tags.contains_key(target_terrain_id)
                && source_terrain_ids
                    .iter()
                    .all(|source_id| source_id != target_terrain_id)
        }
        ItemUseEffectDefinition::RemoveStatus { status_kind_id } => {
            validate_id(status_kind_id).is_ok()
        }
        ItemUseEffectDefinition::RestoreResource {
            resource_id,
            amount,
        } => resource_ids.contains(resource_id) && (1..=1_000_000).contains(amount),
        ItemUseEffectDefinition::RestoreResourceDice {
            resource_id,
            dice,
            sides,
            bonus,
        } => {
            resource_ids.contains(resource_id)
                && (1..=100).contains(dice)
                && (1..=10_000).contains(sides)
                && *bonus <= 1_000_000
        }
        ItemUseEffectDefinition::RestoreResourceFull { resource_id } => {
            resource_ids.contains(resource_id)
        }
        ItemUseEffectDefinition::IdentifyItem { .. } => true,
        ItemUseEffectDefinition::EnchantItem {
            to_hit,
            to_damage,
            to_armor,
        } => {
            let valid_roll = |roll: &ItemEnchantmentRollDefinition| {
                (roll.dice == 0 && roll.sides == 0 && (1..=100).contains(&roll.bonus))
                    || ((1..=10).contains(&roll.dice)
                        && (1..=100).contains(&roll.sides)
                        && roll.bonus <= 100)
            };
            let weapon_rolls = to_hit.iter().chain(to_damage).count();
            let armor_rolls = usize::from(to_armor.is_some());
            ((weapon_rolls > 0 && armor_rolls == 0) || (weapon_rolls == 0 && armor_rolls == 1))
                && to_hit
                    .iter()
                    .chain(to_damage)
                    .chain(to_armor)
                    .all(valid_roll)
        }
        ItemUseEffectDefinition::CurseEquippedItem { .. }
        | ItemUseEffectDefinition::RemoveEquippedCurses { .. } => true,
        ItemUseEffectDefinition::SummonCategory {
            selector,
            count_dice,
            count_sides,
            count_bonus,
            hostile,
            group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            allow_unique,
            radius,
            duration_turns,
            ..
        } => {
            let selector_is_valid = match selector {
                ItemSummonSelectorDefinition::AnyMonster
                | ItemSummonSelectorDefinition::PlayerKin => true,
                ItemSummonSelectorDefinition::Category { category } => {
                    actor_tag_values.contains(category)
                }
            };
            selector_is_valid
                && (1..=8).contains(count_dice)
                && (1..=8).contains(count_sides)
                && u16::from(*count_dice) * u16::from(*count_sides) + u16::from(*count_bonus) <= 8
                && *group_chance_percent <= 100
                && if *group_chance_percent == 0 {
                    *group_count_dice == 0 && *group_count_sides == 0 && *group_count_bonus == 0
                } else {
                    (1..=8).contains(group_count_dice)
                        && (1..=8).contains(group_count_sides)
                        && u16::from(*group_count_dice) * u16::from(*group_count_sides)
                            + u16::from(*group_count_bonus)
                            <= 8
                }
                && (!*allow_unique || *hostile)
                && (1..=8).contains(radius)
                && *duration_turns == 0
        }
        ItemUseEffectDefinition::Sequence { effects } => {
            (2..=8).contains(&effects.len())
                && effects.iter().all(|effect| {
                    matches!(
                        effect,
                        ItemUseEffectDefinition::Heal { .. }
                            | ItemUseEffectDefinition::HealDice { .. }
                            | ItemUseEffectDefinition::RemoveStatus { .. }
                            | ItemUseEffectDefinition::RestoreResource { .. }
                            | ItemUseEffectDefinition::RestoreResourceDice { .. }
                            | ItemUseEffectDefinition::RestoreResourceFull { .. }
                    ) && valid_item_effect(
                        effect,
                        terrain_tags,
                        actor_tag_values,
                        item_tag_values,
                        resource_ids,
                    )
                })
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
        ItemUseEffectDefinition::DispelCategory { category, damage } => {
            actor_tag_values.contains(category) && (1..=1_000_000).contains(damage)
        }
        ItemUseEffectDefinition::BanishVisible { maximum_distance } => {
            (1..=200).contains(maximum_distance)
        }
        ItemUseEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
            through_walls,
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
                        if category == "map" {
                            *persistent && *through_walls
                        } else {
                            terrain_tags.values().any(|tags| tags.contains(category))
                        }
                    }
                    AbilityDetectSubjectDefinition::Actor => {
                        !persistent && actor_tag_values.contains(category)
                    }
                    AbilityDetectSubjectDefinition::Item => {
                        !persistent && (category == "item" || item_tag_values.contains(category))
                    }
                }
        }
        ItemUseEffectDefinition::RandomTeleport { maximum_distance } => {
            (1..=200).contains(maximum_distance)
        }
        ItemUseEffectDefinition::TeleportLevel | ItemUseEffectDefinition::ResetRecall => true,
        ItemUseEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } => {
            (1..=10).contains(delay_dice)
                && (1..=100).contains(delay_sides)
                && *delay_bonus <= 1_000
        }
    }
}

fn item_effect_is_self_targeted(effect: &ItemUseEffectDefinition) -> bool {
    match effect {
        ItemUseEffectDefinition::Damage { .. }
        | ItemUseEffectDefinition::IdentifyItem { .. }
        | ItemUseEffectDefinition::EnchantItem { .. } => false,
        ItemUseEffectDefinition::Sequence { effects } => {
            effects.iter().all(item_effect_is_self_targeted)
        }
        _ => true,
    }
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
        validate_status_immunities(&actor.id, &mut actor.status_immunities)?;
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
            || (actor.role != ActorRole::Monster && !actor.status_immunities.is_empty())
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

    let item_tag_values = content
        .items
        .iter()
        .flat_map(|item| item.tags.iter().cloned())
        .collect::<BTreeSet<_>>();

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
                            AbilityDetectSubjectDefinition::Item => {
                                !persistent
                                    && (category == "item" || item_tag_values.contains(category))
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
                    | ItemUseEffectDefinition::Bless { .. }
                    | ItemUseEffectDefinition::ApplySlowness { .. }
                    | ItemUseEffectDefinition::ApplySpeed { .. }
                    | ItemUseEffectDefinition::ApplyHeroism { .. }
                    | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                    | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                    | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                    | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                    | ItemUseEffectDefinition::RestoreAllAttributes
                    | ItemUseEffectDefinition::RestoreAllVitality { .. }
                    | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                    | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                    | ItemUseEffectDefinition::DrainAttribute { .. }
                    | ItemUseEffectDefinition::RestoreAttribute { .. }
                    | ItemUseEffectDefinition::IncreaseAttribute { .. }
                    | ItemUseEffectDefinition::AugmentAttributes
                    | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                    | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                    | ItemUseEffectDefinition::ApplyPoison { .. }
                    | ItemUseEffectDefinition::ApplyBlindness { .. }
                    | ItemUseEffectDefinition::ApplyDetonation { .. }
                    | ItemUseEffectDefinition::SelfLifeLoss { .. }
                    | ItemUseEffectDefinition::Vengeance { .. }
                    | ItemUseEffectDefinition::ProtectionFromEvil
                    | ItemUseEffectDefinition::PrepareConfusingStrike
                    | ItemUseEffectDefinition::SelfCenteredElementalBlast { .. }
                    | ItemUseEffectDefinition::AggravateMonsters
                    | ItemUseEffectDefinition::MassGenocide { .. }
                    | ItemUseEffectDefinition::Genocide { .. }
                    | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                    | ItemUseEffectDefinition::CreateAdjacentTerrain { .. }
                    | ItemUseEffectDefinition::DestroyAdjacentTrapsAndDoors
                    | ItemUseEffectDefinition::RemoveStatus { .. }
                    | ItemUseEffectDefinition::RestoreResource { .. }
                    | ItemUseEffectDefinition::RestoreResourceDice { .. }
                    | ItemUseEffectDefinition::RestoreResourceFull { .. }
                    | ItemUseEffectDefinition::Sequence { .. }
                    | ItemUseEffectDefinition::Detect { .. }
                    | ItemUseEffectDefinition::RandomTeleport { .. }
                    | ItemUseEffectDefinition::TeleportLevel
                    | ItemUseEffectDefinition::Recall { .. }
                    | ItemUseEffectDefinition::ResetRecall
                    | ItemUseEffectDefinition::CurseEquippedItem { .. }
                    | ItemUseEffectDefinition::RemoveEquippedCurses { .. }
                    | ItemUseEffectDefinition::SummonCategory { .. }
                    | ItemUseEffectDefinition::DispelCategory { .. }
                    | ItemUseEffectDefinition::BanishVisible { .. } => self_target,
                    ItemUseEffectDefinition::RechargeFromDevice { .. } => false,
                    ItemUseEffectDefinition::Damage { .. } => projectile_target,
                    ItemUseEffectDefinition::IdentifyItem { .. }
                    | ItemUseEffectDefinition::EnchantItem { .. } => {
                        target.modes.as_slice() == [AbilityTargetModeDefinition::Item]
                            && target.range == 0
                            && !target.requires_line_of_effect
                    }
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
            || (item.initial_curse.is_some() && item.equipment_slot.is_none())
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
        if let Some(action) = &mut item.use_action
            && let ItemUseEffectDefinition::CreateAdjacentTerrain {
                source_terrain_ids, ..
            } = &mut action.effect
        {
            source_terrain_ids.sort();
        }
        if let Some(action) = &item.use_action {
            let valid_effect = valid_item_effect(
                &action.effect,
                &terrain_tags,
                &actor_tag_values,
                &item_tag_values,
                &resource_ids,
            ) && (item_effect_is_self_targeted(&action.effect)
                || matches!(
                    action.effect,
                    ItemUseEffectDefinition::IdentifyItem { .. }
                        | ItemUseEffectDefinition::EnchantItem { .. }
                ));
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
                || (matches!(
                    action.effect,
                    ItemUseEffectDefinition::RechargeFromDevice { .. }
                        | ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                        | ItemUseEffectDefinition::ApplySlowness { .. }
                        | ItemUseEffectDefinition::ApplySpeed { .. }
                        | ItemUseEffectDefinition::ApplyHeroism { .. }
                        | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                        | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                        | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                        | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                        | ItemUseEffectDefinition::RestoreAllAttributes
                        | ItemUseEffectDefinition::RestoreAllVitality { .. }
                        | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                        | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                        | ItemUseEffectDefinition::IncreaseAttribute { .. }
                        | ItemUseEffectDefinition::AugmentAttributes
                        | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                        | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                        | ItemUseEffectDefinition::ApplyPoison { .. }
                        | ItemUseEffectDefinition::ApplyBlindness { .. }
                        | ItemUseEffectDefinition::ApplyDetonation { .. }
                        | ItemUseEffectDefinition::SelfLifeLoss { .. }
                ) && (action.device_check_difficulty.is_some()
                    || action.charges.is_some()
                    || !item.tags.iter().any(|tag| tag == "consumable")))
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
            for activation in &mut generation.activations {
                if let ItemUseEffectDefinition::CreateAdjacentTerrain {
                    source_terrain_ids, ..
                } = &mut activation.effect
                {
                    source_terrain_ids.sort();
                }
            }
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
                        && valid_item_effect(
                            &activation.effect,
                            &terrain_tags,
                            &actor_tag_values,
                            &item_tag_values,
                            &resource_ids,
                        )
                        && !matches!(
                            activation.effect,
                            ItemUseEffectDefinition::IncreaseSpellLearningCapacity
                                | ItemUseEffectDefinition::ApplySlowness { .. }
                                | ItemUseEffectDefinition::ApplySpeed { .. }
                                | ItemUseEffectDefinition::ApplyHeroism { .. }
                                | ItemUseEffectDefinition::ApplyBerserkStrength { .. }
                                | ItemUseEffectDefinition::ApplyPoeticInspiration { .. }
                                | ItemUseEffectDefinition::ApplyStoneSkin { .. }
                                | ItemUseEffectDefinition::RestoreLifeLevels { .. }
                                | ItemUseEffectDefinition::RestoreAllAttributes
                                | ItemUseEffectDefinition::RestoreAllVitality { .. }
                                | ItemUseEffectDefinition::ApplyRestorativeFeast { .. }
                                | ItemUseEffectDefinition::ApplyLifeRestoration { .. }
                                | ItemUseEffectDefinition::IncreaseAttribute { .. }
                                | ItemUseEffectDefinition::AugmentAttributes
                                | ItemUseEffectDefinition::ApplyThermalResistance { .. }
                                | ItemUseEffectDefinition::ApplyBasicResistance { .. }
                                | ItemUseEffectDefinition::ApplyPoison { .. }
                                | ItemUseEffectDefinition::ApplyBlindness { .. }
                                | ItemUseEffectDefinition::ApplyDetonation { .. }
                                | ItemUseEffectDefinition::SelfLifeLoss { .. }
                        )
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
        if let Some(category) = &race.kin_category
            && (category.is_empty()
                || category.len() > 64
                || !category.bytes().all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || matches!(byte, b'-' | b'_')
                })
                || !actor_tag_values.contains(category))
        {
            return Err(ContentError::InvalidCharacterSource(race.id.clone()));
        }
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
    #[error("effect program definition is invalid: {0}")]
    InvalidEffectProgram(String),
    #[error("ability program definition is invalid: {0}")]
    InvalidAbilityProgram(String),
    #[error("player ability binding is duplicated for ability: {0}")]
    DuplicatePlayerAbilityBinding(String),
    #[error("player ability binding definition is invalid: {0}")]
    InvalidPlayerAbilityBinding(String),
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
mod tests;
