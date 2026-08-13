// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ItemAttributeDefinition, ItemQuality};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorMovementMode {
    Aquatic,
    Climb,
    Fly,
    PassWall,
    Swim,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorHabitat {
    All,
    Grass,
    Mountain,
    Ocean,
    Shore,
    Snow,
    Swamp,
    Town,
    Volcano,
    Waste,
    Wood,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorMovementDefinition {
    #[serde(default)]
    pub modes: Vec<ActorMovementMode>,
    #[serde(default)]
    pub never_moves: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorDoorInteractionDefinition {
    #[serde(default)]
    pub opens: bool,
    #[serde(default)]
    pub bashes: bool,
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
    Blindness,
    Fear,
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
    Curse,
    Meteor,
    Rocket,
    Telekinesis,
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
    #[serde(default)]
    pub self_destructs: bool,
    pub effects: Vec<MeleeBlowEffectDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum MeleeBlowEffectDefinition {
    Damage {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        #[serde(default)]
        armor_mitigated: bool,
        #[serde(default)]
        vampiric: bool,
    },
    Shatter {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
    },
    Bomb {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
    },
    Poison {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
    },
    Disease {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
    },
    DrainAttributes {
        #[serde(default)]
        chance_percent: Option<u8>,
        attributes: Vec<ItemAttributeDefinition>,
    },
    DrainResource {
        #[serde(default)]
        chance_percent: Option<u8>,
        amount_dice: u16,
        amount_sides: u16,
    },
    DrainCharges {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    DrainExperience {
        #[serde(default)]
        chance_percent: Option<u8>,
        amount_dice: u16,
        amount_sides: u16,
    },
    Unlife {
        #[serde(default)]
        chance_percent: Option<u8>,
        amount_dice: u16,
        amount_sides: u16,
    },
    Bleeding {
        #[serde(default)]
        chance_percent: Option<u8>,
        duration_dice: u16,
        duration_sides: u16,
    },
    Blind {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Confusion {
        #[serde(default)]
        chance_percent: Option<u8>,
        damage_dice: u16,
        damage_sides: u16,
    },
    Paralysis {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Amnesia {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Time {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Slow {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Inertia {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    PolymorphPlayer {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Stun {
        #[serde(default)]
        chance_percent: Option<u8>,
        duration_dice: u16,
        duration_sides: u16,
    },
    Terrify {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    Disenchant {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    EatGold {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    EatItem {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    EatFood {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
    EatLight {
        #[serde(default)]
        chance_percent: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorContactAuraDefinition {
    pub damage_type: ActorDamageType,
    pub damage_dice: u16,
    pub damage_sides: u16,
    #[serde(default)]
    pub chance_percent: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeleeRoutineDefinition {
    pub blows: Vec<MeleeBlowDefinition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorHitPointDiceDefinition {
    pub dice: u16,
    pub sides: u16,
    #[serde(default)]
    pub force_maximum: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorTerrainInteractionDefinition {
    #[serde(default)]
    pub destroys_walls: bool,
    #[serde(default)]
    pub destroys_items: bool,
    #[serde(default)]
    pub picks_up_items: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLightDefinition {
    pub radius: u8,
    /// Intrinsic light remains active while the actor sleeps. Carried light does not.
    #[serde(default)]
    pub intrinsic: bool,
    /// Darkness suppresses permanent room glow, but not carried light sources.
    #[serde(default)]
    pub darkness: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum MonsterDropKindDefinition {
    Items,
    Gold,
    ItemsAndGold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterDropChanceDefinition {
    pub percent: u8,
    #[serde(default)]
    pub guaranteed_for_unique: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterDropDiceDefinition {
    pub dice: u8,
    pub sides: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterDropDefinition {
    pub kind: MonsterDropKindDefinition,
    #[serde(default)]
    pub item_table_id: Option<String>,
    #[serde(default)]
    pub theme_table_id: Option<String>,
    #[serde(default)]
    pub theme_chance_percent: u8,
    #[serde(default)]
    pub base_rolls: u8,
    #[serde(default)]
    pub chance_rolls: Vec<MonsterDropChanceDefinition>,
    #[serde(default)]
    pub count_dice: Vec<MonsterDropDiceDefinition>,
    #[serde(default)]
    pub minimum_quality: ItemQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorEvolutionDefinition {
    pub required_experience: u64,
    pub next_actor_kind_id: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ActorCapturePolicyDefinition {
    #[default]
    Normal,
    PetOnly,
    Immune,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evolution: Option<ActorEvolutionDefinition>,
    /// Capture-ball eligibility imported from RFB monster flags.
    #[serde(default)]
    pub capture_policy: ActorCapturePolicyDefinition,
    pub max_hp: i32,
    #[serde(default)]
    pub hit_point_dice: Option<ActorHitPointDiceDefinition>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contact_auras: Vec<ActorContactAuraDefinition>,
    #[serde(default)]
    pub terrain_interaction: ActorTerrainInteractionDefinition,
    #[serde(default)]
    pub light: Option<ActorLightDefinition>,
    #[serde(default)]
    pub awareness: Option<ActorAwarenessDefinition>,
    /// Whether the monster receives RFB's one-player-action spawn grace.
    #[serde(default)]
    pub force_sleep: bool,
    /// Whether this monster starts on the player's side without being controlled.
    #[serde(default)]
    pub friendly: bool,
    #[serde(default)]
    pub monster_casting: Option<MonsterCastingDefinition>,
    #[serde(default)]
    pub loot_table_id: Option<String>,
    /// Chance that a successful death-loot roll becomes gold instead of an item.
    #[serde(default)]
    pub gold_drop_chance_percent: Option<u8>,
    #[serde(default)]
    pub death_drop: Option<MonsterDropDefinition>,
    #[serde(default)]
    pub carried_loot_table_id: Option<String>,
    #[serde(default)]
    pub corpse_item_kind_id: Option<String>,
    #[serde(default)]
    pub remains: Option<MonsterRemainsDefinition>,
    #[serde(default)]
    pub allocation: Option<ActorAllocationDefinition>,
    #[serde(default)]
    pub movement: ActorMovementDefinition,
    /// Whether this actor attacks weaker actors that block its movement.
    #[serde(default)]
    pub kills_weaker_bodies: bool,
    /// Whether this actor swaps places with weaker actors that block its movement.
    #[serde(default)]
    pub moves_weaker_bodies: bool,
    /// Whether this actor receives twice the shared monster regeneration amount.
    #[serde(default)]
    pub regenerates: bool,
    /// Whether this actor reflects single-target bolts.
    #[serde(default)]
    pub reflects_bolts: bool,
    /// Whether this actor can use its melee routine at RFB's two-grid reach.
    #[serde(default)]
    pub ranged_melee: bool,
    /// Whether the player can use this actor as a mount.
    #[serde(default)]
    pub rideable: bool,
    /// Whether the original monster is materially silver.
    #[serde(default)]
    pub made_of_silver: bool,
    /// Maximum number of instances that may ever exist across one campaign.
    /// Ordinary `unique` actors implicitly use one when this is absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifetime_instance_limit: Option<u16>,
    #[serde(default)]
    pub door_interaction: ActorDoorInteractionDefinition,
    pub tags: Vec<String>,
}

impl ActorDefinition {
    #[must_use]
    pub fn finite_lifetime_instance_limit(&self) -> Option<u16> {
        self.lifetime_instance_limit
            .or_else(|| self.tags.iter().any(|tag| tag == "unique").then_some(1))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorAllocationDefinition {
    /// Stable order from the original global monster allocation table.
    pub legacy_index: u32,
    pub rarity: u32,
    /// Original maximum allocation depth; zero means unrestricted.
    pub max_depth: u16,
    /// Optional task that exclusively owns this monster's allocation.
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub force_depth: bool,
    #[serde(default)]
    pub wild_only: bool,
    #[serde(default)]
    pub habitats: Vec<ActorHabitat>,
    #[serde(default)]
    pub legacy_dungeon_indices: Vec<u16>,
    #[serde(default)]
    pub friends: Option<ActorFriendsDefinition>,
    #[serde(default)]
    pub escort: bool,
    #[serde(default)]
    pub multiplies: bool,
    #[serde(default)]
    pub random_movement_percent: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorFriendsDefinition {
    /// Zero dice and sides select the original depth-adjusted 1d10 rule.
    pub dice: u8,
    pub sides: u8,
    /// Zero means unconditional, matching an original FRIENDS flag without a percentage.
    #[serde(default)]
    pub chance_percent: u8,
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
    #[serde(default)]
    pub spell_power_bonus: i32,
    #[serde(default)]
    pub device_power_bonus: i32,
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
