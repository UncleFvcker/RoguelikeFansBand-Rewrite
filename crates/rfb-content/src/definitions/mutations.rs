// SPDX-License-Identifier: MPL-2.0

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::{ActorDamageType, ActorResistanceLevel, StatModifiers};

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
    #[serde(default)]
    pub modifiers: StatModifiers,
    /// Direct armor-class adjustment from the original mutation bonus.
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub saving_throw_skill: i32,
    #[serde(default)]
    pub saving_throw_skill_per_five_levels: i32,
    #[serde(default)]
    pub stealth_skill: i32,
    #[serde(default)]
    pub infravision: i32,
    /// Additive percentage adjustment to the natural HP regeneration rate.
    #[serde(default)]
    pub regeneration_rate_modifier_percent: i32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    #[serde(default)]
    pub levitation: bool,
    #[serde(default)]
    pub telepathy: bool,
    #[serde(default)]
    pub contact_aura: Option<ActorDamageType>,
    #[serde(default)]
    pub light_radius: i32,
    #[serde(default)]
    pub mighty_throw: bool,
    #[serde(default)]
    pub innate_attack: Option<MutationInnateAttackDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removes_on_gain: Vec<String>,
}
