// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ActorDamageType, ActorResistanceLevel, EquipmentBonuses, StatModifiers, WeaponBrand};

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
    pub tags: Vec<String>,
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
    Gold,
    Curse,
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
    MaximumWeight,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_scaling: Vec<AbilityLevelScalingDefinition>,
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
    Malediction {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
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
    JumpDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_multiplier_numerator: u8,
        damage_multiplier_denominator: u8,
        #[serde(default)]
        damage_type: ActorDamageType,
        radius: u8,
        blink_radius: u8,
    },
    BeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
    },
    LightLine {
        damage_dice: u16,
        damage_sides: u16,
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
    BoltOrAreaDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        area_from_level: u16,
        radius: u8,
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
    DarkenRoom,
    AggravateMonsters,
    Teleport,
    FetchItem {
        maximum_weight_tenths_pound: u32,
    },
    ConsumeTerrain {
        nutrition: u16,
    },
    CreateAmmunition {
        item_kind_ids: Vec<String>,
        quantity_minimum: u32,
        quantity_maximum: u32,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        source_item_tags: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        source_terrain_tags: Vec<String>,
    },
    TransmuteItemToGold {
        value_divisor: u8,
        unit_value_cap: u32,
    },
    DrainItemMagic {
        base_power: u16,
        level_multiplier: u16,
        level_divisor: u16,
    },
    ReportMagic,
    Earthquake {
        radius: u8,
        affect_chance_percent: u8,
        floor_terrain_id: String,
        wall_terrain_ids: Vec<String>,
    },
    AreaDestruction {
        minimum_radius: u8,
        maximum_radius: u8,
        floor_terrain_id: String,
        wall_terrain_id: String,
        quartz_terrain_id: String,
        magma_terrain_id: String,
    },
    SuppressMonsterReproduction {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
    },
    MeleeThenTeleport {
        radius: u8,
        failure_threshold: u16,
    },
    PolymorphSelf,
    PolymorphTarget,
    SwapPosition,
    Recall {
        delay_dice: u16,
        delay_sides: u16,
        #[serde(default)]
        delay_bonus: u16,
    },
    ResistElements {
        duration_dice: u16,
        duration_sides: u32,
        #[serde(default)]
        duration_bonus: u32,
    },
    BlinkSelf {
        radius: u8,
    },
    BlinkTarget {
        radius: u8,
    },
    TeleportSelf {
        minimum_distance: u8,
    },
    TeleportTarget,
    TeleportLevel,
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
        #[serde(default)]
        feeds: bool,
    },
    Genocide {
        scope: AbilityGenocideScopeDefinition,
        power: u16,
        #[serde(default)]
        radius: u8,
        #[serde(default)]
        target_category: Option<String>,
        #[serde(default = "default_true")]
        fatigue: bool,
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
        #[serde(default)]
        failure_chance_percent: u8,
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
        resistance_type: Option<ActorDamageType>,
        #[serde(default)]
        power: Option<u16>,
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

const fn default_true() -> bool {
    true
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
            | AbilityEffectDefinition::Malediction { damage_dice, .. }
            | AbilityEffectDefinition::AreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::JumpDamage { damage_dice, .. }
            | AbilityEffectDefinition::BeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::ConeDamage { damage_dice, .. }
            | AbilityEffectDefinition::CurseDamage { damage_dice, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_dice, .. }
            | AbilityEffectDefinition::DrainLife { damage_dice, .. },
            AbilityLevelScalingField::DamageDice,
        ) => Some((u64::from(*damage_dice), 100)),
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::Malediction { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::JumpDamage { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::CurseDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilityLevelScalingField::DamageSides,
        ) => Some((u64::from(*damage_sides), 10_000)),
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::Malediction { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
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
            | AbilityEffectDefinition::JumpDamage { radius, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. }
            | AbilityEffectDefinition::Detect { radius, .. },
            AbilityLevelScalingField::Radius,
        ) => Some((u64::from(*radius), 16)),
        (AbilityEffectDefinition::BlinkSelf { radius }, AbilityLevelScalingField::Radius) => {
            Some((u64::from(*radius), 255))
        }
        (
            AbilityEffectDefinition::BoltOrBeamDamage {
                beam_chance_percent,
                ..
            },
            AbilityLevelScalingField::BeamChancePercent,
        ) => Some((u64::from(*beam_chance_percent), 100)),
        (
            AbilityEffectDefinition::ApplyStatus { intensity, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { intensity, .. },
            AbilityLevelScalingField::StatusIntensity,
        ) => Some((u64::from(*intensity), 1_000)),
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. },
            AbilityLevelScalingField::StatusDurationTicks,
        ) => Some((u64::from(*duration_ticks), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. },
            AbilityLevelScalingField::StatusDurationSides,
        ) => Some((u64::from(*duration_sides), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
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
        (
            AbilityEffectDefinition::FetchItem {
                maximum_weight_tenths_pound,
            },
            AbilityLevelScalingField::MaximumWeight,
        ) => Some((u64::from(*maximum_weight_tenths_pound), 1_000_000)),
        _ => None,
    }
}

pub(crate) fn valid_ability_level_scaling(
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
pub struct PlayerAbilityDefinition {
    pub minimum_level: u16,
    pub resource_id: String,
    pub resource_cost: u32,
    pub base_failure_percent: u8,
    #[serde(default)]
    pub proficiency: AbilityProficiencyDefinition,
    #[serde(default)]
    pub cooldown: Option<AbilityCooldownDefinition>,
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
    pub target: AbilityTargetDefinition,
    pub effect: AbilityEffectDefinition,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_scaling: Vec<AbilityLevelScalingDefinition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<PlayerAbilityDefinition>,
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
    /// Stable spell realm identity used by character rules and book filters.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realm_id: Option<String>,
    /// One-based RFB book rank within its realm.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u8>,
    pub ability_ids: Vec<String>,
    pub tags: Vec<String>,
}
