// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const fn default_actor_speed() -> u16 {
    110
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
    pub inventory_slot_capacity: u16,
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
    /// Chance that a successful death-loot roll becomes gold instead of an item.
    #[serde(default)]
    pub gold_drop_chance_percent: Option<u8>,
    #[serde(default)]
    pub carried_loot_table_id: Option<String>,
    #[serde(default)]
    pub corpse_item_kind_id: Option<String>,
    #[serde(default)]
    pub remains: Option<MonsterRemainsDefinition>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterRemainsDefinition {
    pub chance_denominator: u16,
    #[serde(default)]
    pub corpse_item_kind_id: Option<String>,
    #[serde(default)]
    pub skeleton_item_kind_id: Option<String>,
    #[serde(default)]
    pub corpse_weight: u16,
    #[serde(default)]
    pub skeleton_weight: u16,
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
