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
    Town,
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
    TeleportAwayPower,
    RechargePower,
    IdentifyPower,
    Radius,
    BeamChancePercent,
    StatusIntensity,
    StatusDurationTicks,
    StatusDurationSides,
    StatusDefense,
    StatusPower,
    StatusMeleeDamage,
    ControlPower,
    GenocidePower,
    SummonMaximumLevel,
    MaximumWeight,
    BanishDistance,
    DeviceMasteryDurationBase,
    DevicePowerBonus,
    MaximumRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilitySpellPowerField {
    FinalDamage,
    FinalHealing,
    DamageSides,
    DamageBonus,
    HealingAmount,
    HealingSides,
    Radius,
    StatusDurationTicks,
    StatusDurationSides,
    StatusPower,
    ControlPower,
    GenocidePower,
    IdentifyPower,
    TeleportAwayPower,
    RechargePower,
    RandomChoiceRoll,
    MaledictionDeathRayPower,
    MaledictionFearPower,
    MaximumWeight,
    BanishDistance,
    DeviceMasteryDurationBase,
    InvulnerabilityDuration,
    ClairvoyanceDurationSides,
    MaximumRange,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilitySpellPowerDefinition {
    pub effect_index: u8,
    pub field: AbilitySpellPowerField,
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
    #[serde(default = "default_level_scaling_linear_weight")]
    pub linear_weight: u16,
    #[serde(default)]
    pub quadratic_weight: u16,
    #[serde(default)]
    pub cubic_weight: u16,
}

const fn default_level_scaling_linear_weight() -> u16 {
    1
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilitySummonCandidateDefinition {
    pub actor_kind_id: String,
    pub weight: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityStatusStackingDefinition {
    Replace,
    Extend,
    KeepStrongest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityTerrainBeamOperationDefinition {
    JamDoors,
    DestroyTrapsAndDoors,
    StoneToMud,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum SniperShotModeDefinition {
    Shining,
    Retreat,
    Disarm,
    Burning,
    Shatter,
    Freezing,
    Knockback,
    Piercing,
    Evil,
    Holy,
    Exploding,
    Double,
    Thunder,
    Needle,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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
        #[serde(default)]
        maximum_range: Option<u16>,
    },
    LightLine {
        damage_dice: u16,
        damage_sides: u16,
    },
    LightArea {
        damage_dice: u16,
        damage_sides: u16,
        radius: u8,
        #[serde(default)]
        sunlight_burn_damage_dice: u16,
        #[serde(default)]
        sunlight_burn_damage_sides: u16,
    },
    BoltOrBeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        beam_chance_percent: u8,
        #[serde(default)]
        beam_chance_modifier: i8,
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
        #[serde(default)]
        damage_is_current_hp_percent: bool,
        #[serde(default)]
        nonlethal: bool,
    },
    DeathRay {
        power: u32,
    },
    TeleportAway {
        minimum_distance: u8,
        #[serde(default)]
        power: u16,
    },
    RechargeFromPlayer {
        power: u16,
    },
    Clairvoyance {
        telepathy_duration_ticks: u16,
        telepathy_duration_dice: u8,
        telepathy_duration_sides: u16,
    },
    CallSunlight {
        vampire_damage: u16,
    },
    NatureWrath,
    Probe,
    CreateDoor {
        terrain_id: String,
    },
    DeviceMastery {
        duration_base: u16,
        device_power_bonus: i32,
    },
    Banish {
        maximum_distance: u16,
    },
    Invulnerability {
        duration_dice: u16,
        duration_sides: u16,
        duration_bonus: u16,
    },
    BirdDrop,
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
    CreateItem {
        item_kind_id: String,
        quantity: u32,
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
    CreateStair {
        up_terrain_id: String,
        down_terrain_id: String,
    },
    TeleportTown,
    SelfKnowledge,
    DimensionDoor {
        range: u16,
    },
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        maximum_count: Option<u8>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        batch_candidates: Vec<AbilitySummonCandidateDefinition>,
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
    NatureGate {
        animal_category: String,
        hound_category: String,
        hydra_category: String,
        ent_actor_kind_id: String,
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
        #[serde(default)]
        through_walls: bool,
    },
    RefuelEquippedLight {
        maximum_fraction_divisor: u16,
    },
    TransformTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
        radius: u8,
    },
    CreateAdjacentTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
    },
    CreateCurrentTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
    },
    TerrainBeam {
        operation: AbilityTerrainBeamOperationDefinition,
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
    IdentifyOrMassIdentify {
        mass_at_level: u16,
        upgraded_name_key: String,
        upgraded_description_key: String,
        #[serde(skip)]
        #[cfg_attr(feature = "schemas", schemars(skip))]
        mass: bool,
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
    HealDice {
        dice: u16,
        sides: u16,
    },
    RemoveEquippedCurses {
        include_heavy: bool,
    },
    BeginFasting,
    TurnUndead {
        power: u16,
    },
    SustainAttributes {
        duration_ticks: u32,
    },
    CureMutation,
    ReduceStatus {
        status_kind_id: String,
        amount: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        current_divisor: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remaining_divisor: Option<u32>,
    },
    SatisfyHunger,
    VisibleDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        #[serde(default)]
        damage_type: ActorDamageType,
        #[serde(default)]
        target_category: Option<String>,
        #[serde(default)]
        unlife_change_on_hit: i8,
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
    Entangle {
        power: u16,
        duration_ticks: u32,
    },
    MassSleepOrStasis {
        stasis_at_level: u16,
        sleep_power_multiplier: u16,
        stasis_power_multiplier: u16,
        power_divisor: u16,
        upgraded_name_key: String,
        upgraded_description_key: String,
        #[serde(skip)]
        #[cfg_attr(feature = "schemas", schemars(skip))]
        stasis: bool,
        #[serde(skip)]
        #[cfg_attr(feature = "schemas", schemars(skip))]
        power: u16,
    },
    BrandWeapon {
        affix_id: String,
        #[serde(default)]
        brand: Option<WeaponBrand>,
        #[serde(default)]
        resistance: Option<ActorDamageType>,
    },
    ProtectFromCorrosion,
    RandomChoice {
        roll_sides: u16,
        #[serde(default)]
        level_bonus_divisor: u16,
        branches: Vec<AbilityRandomBranchDefinition>,
    },
    SniperShot {
        mode: SniperShotModeDefinition,
    },
    MeleeAdjacent,
    ProbeMonsters,
    Concentrate,
    Rodeo,
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
            | AbilityEffectDefinition::LightArea { damage_sides, .. }
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
            AbilityEffectDefinition::TeleportAway { power, .. },
            AbilityLevelScalingField::TeleportAwayPower,
        )
        | (
            AbilityEffectDefinition::RechargeFromPlayer { power },
            AbilityLevelScalingField::RechargePower,
        ) => Some((u64::from(*power), 1_000)),
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilityLevelScalingField::IdentifyPower,
        ) => Some((u64::from(*full_identify_power), 1_000)),
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::LightArea { radius, .. }
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
        (AbilityEffectDefinition::DimensionDoor { range }, AbilityLevelScalingField::Radius) => {
            Some((u64::from(*range), 255))
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
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::SustainAttributes { duration_ticks },
            AbilityLevelScalingField::StatusDurationTicks,
        ) => Some((u64::from(*duration_ticks), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. },
            AbilityLevelScalingField::StatusDurationSides,
        ) => Some((u64::from(*duration_sides), 1_000_000)),
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_modifiers, ..
            },
            AbilityLevelScalingField::StatusDefense,
        ) => Some((u64::try_from(granted_modifiers.defense).ok()?, 10_000)),
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::Entangle { power, .. }
            | AbilityEffectDefinition::TurnUndead { power },
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
        (
            AbilityEffectDefinition::Banish { maximum_distance },
            AbilityLevelScalingField::BanishDistance,
        ) => Some((u64::from(*maximum_distance), 1_000)),
        (
            AbilityEffectDefinition::DeviceMastery { duration_base, .. },
            AbilityLevelScalingField::DeviceMasteryDurationBase,
        ) => Some((u64::from(*duration_base), 1_000)),
        (
            AbilityEffectDefinition::DeviceMastery {
                device_power_bonus, ..
            },
            AbilityLevelScalingField::DevicePowerBonus,
        ) => Some((u64::try_from(*device_power_bonus).ok()?, 1_000)),
        (
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            },
            AbilityLevelScalingField::MaximumRange,
        ) => Some((u64::from(*maximum_range), 64)),
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
                        scaling.linear_weight == 1
                            && scaling.quadratic_weight == 0
                            && scaling.cubic_weight == 0
                    }
                    AbilityLevelScalingCurveDefinition::Prorated => {
                        scaling.divisor == 1
                            && scaling.level_offset == 0
                            && scaling.maximum.is_none()
                            && (1..=100).contains(&scaling.linear_weight)
                            && scaling.quadratic_weight <= 100
                            && scaling.cubic_weight <= 100
                    }
                }
                && scaling
                    .maximum
                    .is_none_or(|maximum| (base..=limit).contains(&maximum))
                && scaling_fields.insert((
                    scaling.effect_index,
                    scaling.field,
                    scaling.level_offset,
                ))
                && scaled.is_some_and(|value| value <= limit)
        })
        && level_scaling.iter().all(|scaling| {
            let matching = level_scaling
                .iter()
                .filter(|candidate| {
                    candidate.effect_index == scaling.effect_index
                        && candidate.field == scaling.field
                })
                .collect::<Vec<_>>();
            if matching.len() == 1 {
                return true;
            }
            let effect = &effect.ordered_effects()[usize::from(scaling.effect_index)];
            let Some((base, limit)) = ability_level_scaling_base_and_limit(effect, scaling.field)
            else {
                return false;
            };
            matching.iter().all(|candidate| {
                candidate.curve == AbilityLevelScalingCurveDefinition::Linear
                    && candidate.maximum.is_none()
            }) && matching
                .iter()
                .try_fold(base, |total, candidate| {
                    let addition = 100_u64
                        .saturating_sub(u64::from(candidate.level_offset))
                        .saturating_mul(u64::from(candidate.multiplier))
                        .checked_div(u64::from(candidate.divisor))?;
                    total.checked_add(addition)
                })
                .is_some_and(|value| value <= limit)
        })
}

pub(crate) fn valid_ability_spell_power(
    effect: &AbilityEffectDefinition,
    fields: &[AbilitySpellPowerDefinition],
) -> bool {
    let mut unique = BTreeSet::new();
    fields.len() <= 32
        && fields.iter().all(|definition| {
            let Some(effect) = effect
                .ordered_effects()
                .get(usize::from(definition.effect_index))
            else {
                return false;
            };
            let valid = match definition.field {
                AbilitySpellPowerField::FinalDamage => matches!(
                    effect,
                    AbilityEffectDefinition::Damage { .. }
                        | AbilityEffectDefinition::Malediction { .. }
                        | AbilityEffectDefinition::AreaDamage { .. }
                        | AbilityEffectDefinition::BeamDamage { .. }
                        | AbilityEffectDefinition::LightArea { .. }
                        | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                        | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                        | AbilityEffectDefinition::ConeDamage { .. }
                        | AbilityEffectDefinition::VisibleDamage { .. }
                        | AbilityEffectDefinition::DrainLife { .. }
                ),
                AbilitySpellPowerField::FinalHealing => {
                    matches!(effect, AbilityEffectDefinition::HealDice { .. })
                }
                AbilitySpellPowerField::DamageSides => matches!(
                    effect,
                    AbilityEffectDefinition::Damage { .. }
                        | AbilityEffectDefinition::Malediction { .. }
                        | AbilityEffectDefinition::AreaDamage { .. }
                        | AbilityEffectDefinition::BeamDamage { .. }
                        | AbilityEffectDefinition::LightArea { .. }
                        | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                        | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                        | AbilityEffectDefinition::ConeDamage { .. }
                        | AbilityEffectDefinition::VisibleDamage { .. }
                        | AbilityEffectDefinition::DrainLife { .. }
                ),
                AbilitySpellPowerField::HealingSides => {
                    matches!(effect, AbilityEffectDefinition::HealDice { .. })
                }
                AbilitySpellPowerField::HealingAmount => {
                    matches!(effect, AbilityEffectDefinition::Heal { .. })
                }
                AbilitySpellPowerField::DamageBonus => matches!(
                    effect,
                    AbilityEffectDefinition::Damage { .. }
                        | AbilityEffectDefinition::Malediction { .. }
                        | AbilityEffectDefinition::AreaDamage { .. }
                        | AbilityEffectDefinition::BeamDamage { .. }
                        | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                        | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                        | AbilityEffectDefinition::ConeDamage { .. }
                        | AbilityEffectDefinition::VisibleDamage { .. }
                        | AbilityEffectDefinition::DrainLife { .. }
                ),
                AbilitySpellPowerField::Radius => matches!(
                    effect,
                    AbilityEffectDefinition::AreaDamage { .. }
                        | AbilityEffectDefinition::LightArea { .. }
                        | AbilityEffectDefinition::BoltOrAreaDamage { .. }
                        | AbilityEffectDefinition::ConeDamage { .. }
                        | AbilityEffectDefinition::Earthquake { .. }
                        | AbilityEffectDefinition::DimensionDoor { .. }
                ),
                AbilitySpellPowerField::StatusDurationTicks => matches!(
                    effect,
                    AbilityEffectDefinition::ApplyStatus { .. }
                        | AbilityEffectDefinition::VisibleApplyStatus { .. }
                        | AbilityEffectDefinition::SustainAttributes { .. }
                ),
                AbilitySpellPowerField::StatusDurationSides => {
                    matches!(effect, AbilityEffectDefinition::ApplyStatus { .. })
                }
                AbilitySpellPowerField::StatusPower => matches!(
                    effect,
                    AbilityEffectDefinition::ApplyStatus { power: Some(_), .. }
                        | AbilityEffectDefinition::VisibleApplyStatus { power: Some(_), .. }
                        | AbilityEffectDefinition::MassSleepOrStasis { .. }
                        | AbilityEffectDefinition::Entangle { .. }
                ),
                AbilitySpellPowerField::ControlPower => {
                    matches!(effect, AbilityEffectDefinition::Control { .. })
                }
                AbilitySpellPowerField::GenocidePower => {
                    matches!(effect, AbilityEffectDefinition::Genocide { .. })
                }
                AbilitySpellPowerField::IdentifyPower => {
                    matches!(effect, AbilityEffectDefinition::IdentifyItem { .. })
                }
                AbilitySpellPowerField::TeleportAwayPower => {
                    matches!(effect, AbilityEffectDefinition::TeleportAway { .. })
                }
                AbilitySpellPowerField::RechargePower => {
                    matches!(effect, AbilityEffectDefinition::RechargeFromPlayer { .. })
                }
                AbilitySpellPowerField::RandomChoiceRoll => {
                    matches!(effect, AbilityEffectDefinition::RandomChoice { .. })
                }
                AbilitySpellPowerField::MaledictionDeathRayPower
                | AbilitySpellPowerField::MaledictionFearPower => {
                    matches!(effect, AbilityEffectDefinition::Malediction { .. })
                }
                AbilitySpellPowerField::MaximumWeight => {
                    matches!(effect, AbilityEffectDefinition::FetchItem { .. })
                }
                AbilitySpellPowerField::BanishDistance => {
                    matches!(effect, AbilityEffectDefinition::Banish { .. })
                }
                AbilitySpellPowerField::DeviceMasteryDurationBase => {
                    matches!(effect, AbilityEffectDefinition::DeviceMastery { .. })
                }
                AbilitySpellPowerField::InvulnerabilityDuration => {
                    matches!(effect, AbilityEffectDefinition::Invulnerability { .. })
                }
                AbilitySpellPowerField::ClairvoyanceDurationSides => {
                    matches!(effect, AbilityEffectDefinition::Clairvoyance { .. })
                }
                AbilitySpellPowerField::MaximumRange => matches!(
                    effect,
                    AbilityEffectDefinition::BeamDamage {
                        maximum_range: Some(_),
                        ..
                    }
                ),
            };
            valid && unique.insert((definition.effect_index, definition.field))
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
    pub first_success_experience: u32,
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
    /// Whether this projected ability applies RFB object-destruction rules to
    /// ground items on the bolt, beam, or blast footprint.
    #[serde(default)]
    pub affects_ground_items: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub level_scaling: Vec<AbilityLevelScalingDefinition>,
    /// Adds the selected effective attribute's RFB saving-throw adjustment to
    /// a direct status effect's level-scaled power.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_power_attribute: Option<ItemAttributeDefinition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_power_fields: Vec<AbilitySpellPowerDefinition>,
    #[serde(skip)]
    #[cfg_attr(feature = "schemas", schemars(skip))]
    pub spell_power_bonus: i32,
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
