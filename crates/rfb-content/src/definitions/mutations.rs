// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{
    AbilityStatusStackingDefinition, ActorDamageType, ActorResistanceLevel, StatModifiers,
    TechniqueAttribute,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MutationRatingDefinition {
    Awful,
    Bad,
    Average,
    Good,
    Great,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ChaosPatronRewardKind {
    PolymorphSelf,
    GainExperience,
    LoseExperience,
    GoodObject,
    GreatObject,
    ChaosWeapon,
    GoodObjects,
    GreatObjects,
    TyCurse,
    SummonMonsters,
    HighSummon,
    Havoc,
    GainAttribute,
    LoseAttribute,
    RuinAttributes,
    AugmentAttributes,
    PolymorphWounds,
    FullHealing,
    HurtBadly,
    CurseWeapon,
    CurseArmor,
    Anger,
    Wrath,
    Destruction,
    Genocide,
    MassGenocide,
    DispelMonsters,
    Ignore,
    UndeadServant,
    DemonServant,
    MonsterServant,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChaosPatronDefinition {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favored_attribute: Option<TechniqueAttribute>,
    pub rewards: Vec<ChaosPatronRewardKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationInnateAttackDefinition {
    pub name: String,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub damage_type: ActorDamageType,
    /// Original RFB attack weight used by the innate critical-hit roll.
    pub weight_tenths_pound: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationRatioDefinition {
    pub numerator: u16,
    pub denominator: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationActivationCostScalingDefinition {
    pub start_level: u16,
    pub level_interval: u16,
    pub amount: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationActivationDefinition {
    pub minimum_level: u16,
    pub governing_attribute: TechniqueAttribute,
    pub cost: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_scaling: Option<MutationActivationCostScalingDefinition>,
    pub base_failure_percent: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_failure_percent: Option<u8>,
    pub ability_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MutationPeriodicEffectDefinition {
    ApplyStatus {
        trigger_one_in: u32,
        #[serde(default)]
        skip_if_present: bool,
        status_kind_id: String,
        intensity: u16,
        duration_ticks: u32,
        duration_dice: u16,
        duration_sides: u32,
        stacking: AbilityStatusStackingDefinition,
    },
    BerserkRage,
    Cowardice,
    Alcohol,
    Hallucination,
    ProduceMana,
    SpeedFlux,
    Invulnerability,
    SpToHp,
    HpToSp,
    Hypochondria,
    RandomTeleport,
    RandomBanish,
    ShadowWalk,
    Fumbling,
    Flatulence,
    AttractDemon,
    EatLight,
    AttractAnimal,
    RawChaos,
    AttractDragon,
    Normality,
    Wraithform,
    PolymorphWounds,
    Wasting,
    RandomTelepathy,
    Nausea,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationDefinition {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub format_version: u16,
    pub id: String,
    pub name: String,
    pub description: String,
    pub rating: MutationRatingDefinition,
    pub source_index: u16,
    pub random_weight: u8,
    #[serde(default = "default_true")]
    pub random_selection_enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<MutationActivationDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_effect: Option<MutationPeriodicEffectDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chaos_patrons: Vec<ChaosPatronDefinition>,
    #[serde(default)]
    pub modifiers: StatModifiers,
    /// Direct armor-class adjustment from the original mutation bonus.
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub saving_throw_skill: i32,
    #[serde(default)]
    pub device_skill: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub ranged_skill: i32,
    #[serde(default)]
    pub saving_throw_skill_per_five_levels: i32,
    #[serde(default)]
    pub stealth_skill: i32,
    #[serde(default)]
    pub search_skill: i32,
    #[serde(default)]
    pub perception_skill: i32,
    #[serde(default)]
    pub infravision: i32,
    /// Additive percentage adjustment to the natural HP regeneration rate.
    #[serde(default)]
    pub regeneration_rate_modifier_percent: i32,
    /// Adds this many maximum hit points for each player level.
    #[serde(default)]
    pub max_hp_per_level: i32,
    /// Additive percentage applied by the shared player-healing transaction.
    #[serde(default)]
    pub healing_bonus_percent: u16,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    #[serde(default)]
    pub levitation: bool,
    #[serde(default)]
    pub telepathy: bool,
    /// Masks charisma changes from other mutations and guarantees the
    /// level-scaled minimum appearance used by the original Ill Norm mutation.
    #[serde(default)]
    pub normal_appearance: bool,
    #[serde(default)]
    pub contact_aura: Option<ActorDamageType>,
    #[serde(default)]
    pub light_radius: i32,
    #[serde(default)]
    pub mighty_throw: bool,
    #[serde(default)]
    pub innate_attack: Option<MutationInnateAttackDefinition>,
    #[serde(default)]
    pub spell_failure_modifier_percent: i32,
    #[serde(default)]
    pub kill_experience_bonus_percent: u16,
    #[serde(default)]
    pub relative_experience_multiplier: Option<MutationRatioDefinition>,
    #[serde(default)]
    pub auto_identify_items: bool,
    #[serde(default)]
    pub movement_energy_multiplier: Option<MutationRatioDefinition>,
    #[serde(default)]
    pub scroll_energy_multiplier: Option<MutationRatioDefinition>,
    #[serde(default)]
    pub potion_energy_multiplier: Option<MutationRatioDefinition>,
    #[serde(default)]
    pub black_market_standard_prices: bool,
    #[serde(default)]
    pub dispel_resistance_percent: u8,
    #[serde(default)]
    pub resource_drain_immunity: bool,
    #[serde(default)]
    pub device_charge_drain_immunity: bool,
    #[serde(default)]
    pub weapon_proficiency_maximum: Option<u16>,
    #[serde(default)]
    pub infernal_deal: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removes_on_gain: Vec<String>,
}

const fn default_true() -> bool {
    true
}
