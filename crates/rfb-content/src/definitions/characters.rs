// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AbilityLevelScalingDefinition, ActorDamageType, ActorResistanceLevel, StatModifiers,
    default_percent,
};

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
    /// Percentage applied by shopkeepers before charisma and greed. RFB's
    /// neutral human factor is 100; unspecified races use the 110 default.
    #[serde(default = "default_shop_adjust_percent")]
    pub shop_adjust_percent: u16,
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

const fn default_shop_adjust_percent() -> u16 {
    110
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
    /// Item tags accepted by Mogaminator's favorite-weapon predicate.
    #[serde(default)]
    pub favorite_weapon_tags: Vec<String>,
    /// Equipment slot types known to be uncomfortable for this class.
    #[serde(default)]
    pub icky_equipment_slots: Vec<String>,
    /// Item tags with class-specific utility that ordinary rules must retain.
    #[serde(default)]
    pub special_item_tags: Vec<String>,
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
    #[serde(default)]
    pub subclass_name_key: Option<String>,
    #[serde(default)]
    pub speciality_name_key: Option<String>,
    #[serde(default)]
    pub first_realm_id: Option<String>,
    #[serde(default)]
    pub second_realm_id: Option<String>,
    pub attributes: InitialAttributeSetDefinition,
    #[serde(default)]
    pub starting_items: Vec<StartingItemDefinition>,
    pub tags: Vec<String>,
}
