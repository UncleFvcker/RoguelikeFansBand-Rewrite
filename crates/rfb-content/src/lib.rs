// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod ability_programs;
mod artifact;
mod catalog;
mod definitions;
mod effect_programs;
mod player_ability_bindings;
#[cfg(feature = "schemas")]
mod schemas;
mod source;
mod validation;

#[cfg(feature = "schemas")]
use ability_programs::SourceAbilityDefinition;
pub use ability_programs::{
    AbilityProgramDefinition, AbilityProgramInputDefinition, AbilityProgramStepDefinition,
};
pub use artifact::{CompiledArtifact, decode_content, encode_content, read_compiled_file};
pub use catalog::{CompiledContentV1, ContentCatalog, ContentLockV1, ContentSummary};
pub use definitions::*;
pub use effect_programs::{
    EffectProgramDefinition, EffectProgramInputDefinition, EffectProgramStepDefinition,
};
pub use player_ability_bindings::PlayerAbilityBindingDefinition;
#[cfg(feature = "schemas")]
pub use schemas::generated_schema_documents;
#[cfg(feature = "schemas")]
use source::SourceItemDefinition;
pub use source::{compile_pack_dir, verify_pack_lock};

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
