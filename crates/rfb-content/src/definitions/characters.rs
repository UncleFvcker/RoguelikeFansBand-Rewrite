// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AbilityLevelScalingDefinition, ActorDamageType, ActorResistanceLevel, AmmunitionTypeDefinition,
    InnatePowerDefinition, ItemAttributeDefinition, StatModifiers, default_percent,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum_quantity: Option<u32>,
    #[serde(default)]
    pub equipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum RaceLevelStatDefinition {
    ArmorClass,
    Speed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceLevelStatScalingDefinition {
    pub stat: RaceLevelStatDefinition,
    pub multiplier: i32,
    pub divisor: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    /// Original RFB race or mimic index. Temporary-form mechanics use this
    /// stable source identity instead of depending on localized names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legacy_index: Option<u16>,
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
    /// Direct intrinsic armor-class adjustment supplied by the effective race.
    #[serde(default)]
    pub armor_class: i32,
    /// Intrinsic infravision range in map cells.
    #[serde(default)]
    pub infravision: i32,
    /// Whether the effective race can cross terrain that requires levitation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub levitation: bool,
    /// Whether this race contributes one intrinsic see-invisible source.
    #[serde(default, skip_serializing_if = "is_false")]
    pub see_invisible: bool,
    /// Character level at which this effective race begins contributing one
    /// intrinsic see-invisible source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub see_invisible_minimum_level: Option<u16>,
    /// Character level at which this effective race grants permanent telepathy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub telepathy_minimum_level: Option<u16>,
    /// Character level at which this effective race begins reflecting bolts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflects_bolts_minimum_level: Option<u16>,
    /// Character level at which this effective race gains intrinsic hold life.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_life_minimum_level: Option<u16>,
    /// Attributes this race innately prevents from being reduced.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub attribute_sustains: BTreeSet<ItemAttributeDefinition>,
    /// Additive percentage adjustment to the natural HP regeneration rate.
    #[serde(default)]
    pub regeneration_rate_modifier_percent: i32,
    /// Divisor applied to nutrition gained from ordinary food effects.
    #[serde(default = "default_food_nutrition_divisor")]
    pub food_nutrition_divisor: u16,
    /// Intrinsic derived-stat adjustments scaled from character level while
    /// this is the currently effective race.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_stat_scalings: Vec<RaceLevelStatScalingDefinition>,
    /// RFB `spell_cap` bonus in twentieths of the caster's base mana pool.
    #[serde(default)]
    pub spell_capacity_bonus: i32,
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
    /// Intrinsic resistance tiers gained at a character-level threshold while
    /// this is the currently effective race.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_resistances: Vec<LevelResistanceDefinition>,
    /// Status kind ids members of this race are innately immune to.
    #[serde(default)]
    pub status_immunities: Vec<String>,
    /// Level-gated race mutations. Completion is represented by the chosen
    /// mutation being present in the character's locked mutation set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_mutation_rewards: Vec<RaceLevelMutationRewardDefinition>,
    /// Birth-race-specific behavior for mutations granted by this race.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mutation_overrides: BTreeMap<String, RaceMutationOverrideDefinition>,
    /// Mutations omitted from a manual race reward for a specific class.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mutation_choice_exclusions_by_class: BTreeMap<String, BTreeSet<String>>,
    /// RFB innate powers supplied by the currently effective race.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<InnatePowerDefinition>,
    /// Actor category used by player-kin summons. Omission makes that race
    /// produce an observed zero-result summon instead of guessing ancestry.
    #[serde(default)]
    pub kin_category: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceLevelMutationRewardDefinition {
    pub id: String,
    pub minimum_level: u16,
    pub selection: RaceMutationSelectionDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RaceMutationOverrideDefinition {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<InnatePowerDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub armor_class: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resistances: Option<BTreeMap<ActorDamageType, ActorResistanceLevel>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_aura: Option<ActorDamageType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RaceMutationSelectionDefinition {
    Choice {
        mutation_ids: Vec<String>,
    },
    CastingAttribute {
        default_mutation_id: String,
        #[serde(default)]
        mutation_ids_by_attribute: BTreeMap<CastingAttribute, String>,
    },
}

const fn default_shop_adjust_percent() -> u16 {
    110
}

const fn default_food_nutrition_divisor() -> u16 {
    1
}

const fn is_false(value: &bool) -> bool {
    !*value
}

const fn default_pet_upkeep_divisor() -> u16 {
    40
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
    /// RFB per-base-weapon birth proficiency and class training ceiling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weapon_proficiency: Option<WeaponProficiencyDefinition>,
    /// RFB riding proficiency at birth and the class training ceiling.
    pub riding_proficiency: RidingProficiencyDefinition,
    /// Uses the Beastmaster/Cavalry mounted attack penalties instead of the
    /// ordinary rider formula.
    #[serde(default)]
    pub riding_combat_expert: bool,
    /// Optional RFB class cap applied to mounted non-arrow shooting speed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mounted_non_arrow_base_shot_cap: Option<u16>,
    #[serde(default)]
    pub uses_spell_scrolls: bool,
    #[serde(default)]
    pub casting_profile: Option<CastingProfileDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<ClassAbilityDefinition>,
    /// Optional RFB sniper shooting and concentration rules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sniping_profile: Option<SnipingProfileDefinition>,
    /// Intrinsic resistance tiers gained when the character reaches a class
    /// level threshold.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_resistances: Vec<LevelResistanceDefinition>,
    /// Percentage-point change to RFB's shooter breakage factor.
    #[serde(default)]
    pub ammunition_breakage_factor_modifier: i16,
    /// RFB ranged-critical chance bonus gained for each character level.
    #[serde(default)]
    pub projectile_critical_chance_bonus_percent_per_level: u8,
    /// RFB class divisor used to convert controlled monster levels into mana upkeep.
    #[serde(default = "default_pet_upkeep_divisor")]
    pub pet_upkeep_divisor: u16,
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnipingProfileDefinition {
    pub preferred_ammunition_type: AmmunitionTypeDefinition,
    pub preferred_ammunition_to_hit_base: i16,
    pub preferred_ammunition_to_hit_level_divisor: u16,
    /// Percentage of shooting speed above 100 retained by the class.
    pub base_shot_excess_percent: u16,
    pub preferred_ammunition_critical_chance_percent: u16,
    pub base_concentration_maximum: u8,
    pub concentration_level_offset: u16,
    pub concentration_level_divisor: u16,
    /// Per-concentration percentage used for ammunition damage, critical
    /// chance, and target-armor reduction.
    pub concentration_bonus_percent_per_level: u8,
}

impl SnipingProfileDefinition {
    #[must_use]
    pub fn maximum_concentration(self, level: u16) -> u8 {
        let level_bonus = level.saturating_add(self.concentration_level_offset)
            / self.concentration_level_divisor;
        u8::try_from(u16::from(self.base_concentration_maximum).saturating_add(level_bonus))
            .unwrap_or(u8::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WeaponProficiencyBoundsDefinition {
    pub initial: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WeaponProficiencyDefinition {
    pub default_initial: u16,
    pub default_maximum: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, WeaponProficiencyBoundsDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RidingProficiencyDefinition {
    pub initial: u16,
    pub maximum: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LevelResistanceDefinition {
    pub minimum_level: u16,
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingAttribute {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingCapacityFormula {
    #[default]
    Linear,
    RfbMana,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingLearningFormula {
    #[default]
    Linear,
    RfbSingleRealm,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingStudyMode {
    #[default]
    Chosen,
    DivineRandom,
}

impl CastingStudyMode {
    const fn is_chosen(&self) -> bool {
        matches!(self, Self::Chosen)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum CastingFailureFormula {
    #[default]
    Linear,
    RfbMagic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClassAbilityDefinition {
    pub ability_id: String,
    pub minimum_level: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_group_name_key: Option<String>,
    #[serde(default)]
    pub governing_attribute: Option<TechniqueAttribute>,
    #[serde(default)]
    pub resource_id: Option<String>,
    pub resource_cost: u32,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub minimum_concentration: u8,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub hit_point_cost: u32,
    pub base_failure_percent: u8,
    pub minimum_failure_percent: u8,
}

const fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}

const fn is_zero_u32(value: &u32) -> bool {
    *value == 0
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
pub struct CastingRealmProfileDefinition {
    pub realm_id: String,
    pub ability_book_ids: Vec<String>,
    #[serde(default)]
    pub learning_capacity_bonus: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ability_overrides: Vec<AbilityCastingOverrideDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CastingEncumbranceDefinition {
    pub maximum_weight_tenths_pound: u32,
    pub weapon_weight_percent: u16,
    pub penalty_weight_tenths_pound: u32,
    #[serde(default)]
    pub glove_encumbrance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CastingProfileDefinition {
    pub resource_id: String,
    pub casting_attribute: CastingAttribute,
    #[serde(default)]
    pub capacity_formula: CastingCapacityFormula,
    pub base_capacity: u32,
    pub capacity_per_level: u32,
    pub capacity_per_attribute_index: u32,
    #[serde(default = "default_percent")]
    pub capacity_percent: u16,
    #[serde(default)]
    pub learning_formula: CastingLearningFormula,
    #[serde(default, skip_serializing_if = "CastingStudyMode::is_chosen")]
    pub study_mode: CastingStudyMode,
    #[serde(default)]
    pub failure_formula: CastingFailureFormula,
    pub base_learning_capacity: u16,
    pub learning_capacity_per_level: u16,
    pub learning_capacity_per_attribute_index: u16,
    pub learning_capacity_cap: u16,
    #[serde(default = "default_percent")]
    pub resource_recovery_percent: u16,
    pub minimum_failure_percent: u8,
    #[serde(default)]
    pub beam_chance_level_multiplier: u8,
    #[serde(default = "default_beam_chance_level_divisor")]
    pub beam_chance_level_divisor: u8,
    #[serde(default)]
    pub beam_chance_bonus: i8,
    #[serde(default)]
    pub spell_damage_bonus_base: u16,
    #[serde(default)]
    pub spell_damage_bonus_per_level: u16,
    #[serde(default = "default_beam_chance_level_divisor")]
    pub spell_damage_bonus_level_divisor: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encumbrance: Option<CastingEncumbranceDefinition>,
    pub realm_profiles: Vec<CastingRealmProfileDefinition>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_actor_id: Option<String>,
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
