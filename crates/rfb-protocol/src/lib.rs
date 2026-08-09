// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "bindings")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
#[cfg(feature = "bindings")]
use ts_rs::{Config, TS};

pub const PROTOCOL_VERSION: &str = "1.147";
pub const SAVE_HEADER_SCHEMA_VERSION: u16 = 1;
pub const SAVE_PAYLOAD_SCHEMA_VERSION: u16 = 1;

const fn default_actor_speed() -> u16 {
    110
}

pub const PLAYER_NUTRITION_MAXIMUM: u16 = 15_000;
pub const PLAYER_NUTRITION_BIRTH: u16 = 9_999;

const fn default_player_nutrition() -> u16 {
    PLAYER_NUTRITION_BIRTH
}

const fn default_monster_energy_need() -> i32 {
    100
}

const fn default_actor_alerted() -> bool {
    true
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_default_player_progress(value: &PlayerProgressDto) -> bool {
    value == &PlayerProgressDto::default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GameCommand {
    AcceptTask {
        facility_id: String,
        task_id: String,
    },
    AbandonTask,
    AbandonPausedTask {
        task_id: String,
    },
    IncreaseAttribute {
        attribute: AttributeKindDto,
    },
    Appraise {
        item_id: String,
    },
    BashDoor {
        direction: Direction,
    },
    BuyFromShop {
        shop_id: String,
        item_id: String,
        quantity: u32,
    },
    ClaimTaskReward {
        facility_id: String,
        task_id: String,
    },
    DepositAtHome {
        facility_id: String,
        item_id: String,
        quantity: u32,
    },
    CastAbility {
        ability_id: String,
        target: TargetSelection,
    },
    CloseDoor {
        direction: Direction,
    },
    DisarmTrap {
        direction: Direction,
    },
    DigTerrain {
        direction: Direction,
    },
    Drop {
        item_ids: Vec<String>,
    },
    DropQuantity {
        item_id: String,
        quantity: u32,
    },
    Equip {
        item_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot_id: Option<String>,
    },
    Fire {
        direction: Direction,
    },
    FireTarget {
        target: TargetSelection,
    },
    EnterWorldMap {
        #[serde(default)]
        leave_pets: bool,
        #[serde(default)]
        cancel_recall: bool,
    },
    LeaveWorldMap,
    TravelWorld {
        destination: Position,
    },
    Move {
        direction: Direction,
    },
    Ride {
        direction: Direction,
    },
    OpenDoor {
        direction: Direction,
    },
    PickUp,
    Retire,
    Rest {
        #[cfg_attr(feature = "bindings", schemars(range(min = 1, max = 100)))]
        turns: u16,
    },
    Search,
    WithdrawFromHome {
        facility_id: String,
        item_id: String,
        quantity: u32,
    },
    SellToShop {
        shop_id: String,
        item_id: String,
        quantity: u32,
    },
    RechargeItem {
        target_item_id: String,
        source: DeviceRechargeSourceDto,
    },
    RefuelLight {
        target_item_id: String,
        source_item_id: String,
    },
    SetSummonCommand {
        mode: SummonCommandModeDto,
    },
    ForgetAbility {
        ability_id: String,
    },
    StudyAbility {
        book_item_id: String,
        ability_id: String,
    },
    Throw {
        item_id: String,
        direction: Direction,
    },
    TraverseStairs,
    UseItem {
        item_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target: Option<TargetSelection>,
    },
    UseItemByGlyph {
        item_id: String,
        glyph: String,
    },
    UseItemForRecharge {
        item_id: String,
        source_item_id: String,
        target_item_id: String,
    },
    Unequip {
        slot_id: String,
    },
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum DeviceRechargeSourceDto {
    Resource,
    Item { item_id: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum SummonCommandModeDto {
    #[default]
    Follow,
    Attack,
    KeepDistance,
    Guard,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonCommandDto {
    pub mode: SummonCommandModeDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guard_position: Option<Position>,
}

fn is_default_summon_command(value: &SummonCommandDto) -> bool {
    value == &SummonCommandDto::default()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct StatModifiersDto {
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EquipmentBonusesDto {
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

impl EquipmentBonusesDto {
    fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum EquipmentPassiveDto {
    Regeneration,
    SeeInvisible,
    Vampiric,
    SustainStrength,
    SustainIntelligence,
    SustainWisdom,
    SustainDexterity,
    SustainConstitution,
    SustainCharisma,
}

fn migrate_rolled_affix_passives<E>(passives: Vec<String>) -> Result<Vec<EquipmentPassiveDto>, E>
where
    E: serde::de::Error,
{
    passives
        .into_iter()
        .filter_map(|passive| match passive.as_str() {
            "regeneration" => Some(Ok(EquipmentPassiveDto::Regeneration)),
            "see-invisible" => Some(Ok(EquipmentPassiveDto::SeeInvisible)),
            "vampiric" => Some(Ok(EquipmentPassiveDto::Vampiric)),
            "sustain-strength" => Some(Ok(EquipmentPassiveDto::SustainStrength)),
            "sustain-intelligence" => Some(Ok(EquipmentPassiveDto::SustainIntelligence)),
            "sustain-wisdom" => Some(Ok(EquipmentPassiveDto::SustainWisdom)),
            "sustain-dexterity" => Some(Ok(EquipmentPassiveDto::SustainDexterity)),
            "sustain-constitution" => Some(Ok(EquipmentPassiveDto::SustainConstitution)),
            "sustain-charisma" => Some(Ok(EquipmentPassiveDto::SustainCharisma)),
            "telepathy" | "levitation" | "hold-life" | "blessed" | "easy-spell"
            | "device-power" => None,
            _ => Some(Err(serde::de::Error::custom(format!(
                "unknown rolled affix passive `{passive}`"
            )))),
        })
        .collect()
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageDiceDto {
    pub dice: u16,
    pub sides: u16,
    #[serde(default)]
    pub damage_type: DamageTypeDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AttackProfileDto {
    pub attacks: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
}

impl Default for AttackProfileDto {
    fn default() -> Self {
        Self {
            attacks: 1,
            to_hit: 0,
            to_damage: 0,
            damage: DamageDiceDto::default(),
            source_item_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MeleeBlowDto {
    pub method_id: String,
    pub to_hit: i32,
    pub damage: DamageDiceDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MeleeRoutineDto {
    pub blows: Vec<MeleeBlowDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TargetModeDto {
    Direction,
    Position,
    Entity,
    Item,
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TargetSpecDto {
    pub modes: Vec<TargetModeDto>,
    pub range: u16,
    pub requires_line_of_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum TargetSelection {
    Direction {
        direction: Direction,
    },
    Position {
        position: Position,
    },
    Entity {
        entity_id: String,
    },
    Item {
        item_id: String,
    },
    #[serde(rename = "self")]
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ProjectileProfileDto {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    #[serde(default)]
    pub ammo_kind_id: String,
    #[serde(default)]
    pub target_spec: TargetSpecDto,
    pub source_item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ThrowProfileDto {
    pub range: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub damage: DamageDiceDto,
    pub source_item_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ProjectileTraceDto {
    pub origin: Position,
    pub impact: Position,
    #[serde(default)]
    pub landing: Position,
    pub traversed: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameCommandEnvelope {
    pub command_seq: u32,
    pub expected_revision: u32,
    pub command: GameCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum MapScaleDto {
    #[default]
    Local,
    World,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum WildernessLocationKindDto {
    Town,
    Dungeon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct WildernessLocationDto {
    pub kind: WildernessLocationKindDto,
    pub id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainInteractionKindDto {
    OpenDoor,
    CloseDoor,
    BashDoor,
    DisarmTrap,
    DigTerrain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ResourcePoolDto {
    pub id: String,
    pub name_key: String,
    pub current: u32,
    pub maximum: u32,
    #[serde(default)]
    pub wait_recovery_amount: u32,
    #[serde(default)]
    pub rest_recovery_amount: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub melee_hit_gain_amount: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub melee_kill_gain_amount: u32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub turn_decay_amount: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityLearningDto {
    pub learned_count: u16,
    pub capacity: u16,
    pub remaining_slots: u16,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityProficiencyRankDto {
    #[default]
    Unskilled,
    Beginner,
    Skilled,
    Expert,
    Master,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityStatusStackingDto {
    Replace,
    Extend,
    KeepStrongest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityDetectSubjectDto {
    #[default]
    Terrain,
    Actor,
    Item,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityControlOutcomeDto {
    Ineligible,
    Resisted,
    Controlled,
    AlreadyControlled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityGenocideScopeDto {
    Single,
    Glyph,
    Nearby,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityRandomTargetDto {
    #[default]
    CastTarget,
    SelfTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityRandomBranchSpecDto {
    pub maximum_roll: u16,
    pub target: AbilityRandomTargetDto,
    pub effect: Box<AbilityEffectSpecDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum AbilityEffectSpecDto {
    Damage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_type: DamageTypeDto,
    },
    AreaDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_type: DamageTypeDto,
        radius: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_category: Option<String>,
    },
    BeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_type: DamageTypeDto,
    },
    BoltOrBeamDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_type: DamageTypeDto,
        beam_chance_percent: u8,
    },
    ConeDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
        damage_type: DamageTypeDto,
        radius: u8,
    },
    BreathDamage {
        hp_percent: u8,
        max_damage: u16,
        damage_type: DamageTypeDto,
        radius: u8,
    },
    CurseDamage {
        damage_dice: u16,
        damage_sides: u16,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upgraded_category: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        upgrade_at_level: Option<u16>,
        maximum_level: u16,
        count_dice: u8,
        count_sides: u8,
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
        subject: AbilityDetectSubjectDto,
        category: String,
        radius: u8,
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
        stacking: AbilityStatusStackingDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resistance_type: Option<DamageTypeDto>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        power: Option<u16>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        granted_resistances: Vec<ResistanceDto>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        granted_brands: Vec<WeaponBrandDto>,
        #[serde(default)]
        granted_modifiers: StatModifiersDto,
        #[serde(default)]
        granted_equipment_bonuses: EquipmentBonusesDto,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        granted_status_immunities: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        granted_race_id: Option<String>,
        #[serde(default)]
        grants_wall_passage: bool,
        #[serde(default = "default_incoming_damage_percent")]
        incoming_damage_percent: u8,
    },
    BlinkSelf {
        radius: u8,
    },
    TeleportSelf {
        minimum_distance: u8,
    },
    TeleportTarget,
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
        damage_type: DamageTypeDto,
        target_category: String,
        #[serde(default = "default_ability_effect_repeat")]
        repeat: u8,
    },
    Genocide {
        scope: AbilityGenocideScopeDto,
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
        damage_bonus: u16,
        damage_type: DamageTypeDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_category: Option<String>,
    },
    VisibleApplyStatus {
        status_kind_id: String,
        intensity: u16,
        duration_ticks: u32,
        stacking: AbilityStatusStackingDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_category: Option<String>,
    },
    EnchantEquippedWeapon {
        affix_id: String,
    },
    RandomChoice {
        roll_sides: u16,
        level_bonus_divisor: u16,
        branches: Vec<AbilityRandomBranchSpecDto>,
    },
    NoOp {
        reason: String,
    },
}

const fn default_ability_effect_repeat() -> u8 {
    1
}

const fn default_incoming_damage_percent() -> u8 {
    100
}

const fn default_life_force() -> u16 {
    1_000
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityDto {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub minimum_level: u16,
    #[serde(default, skip_serializing_if = "is_false")]
    pub innate: bool,
    pub resource_id: String,
    #[serde(default)]
    pub base_resource_cost: u32,
    pub resource_cost: u32,
    pub failure_percent: u8,
    #[serde(default)]
    pub proficiency: u16,
    #[serde(default)]
    pub proficiency_cap: u16,
    #[serde(default)]
    pub proficiency_rank: AbilityProficiencyRankDto,
    #[serde(default)]
    pub cast_count: u32,
    #[serde(default)]
    pub fail_count: u32,
    #[serde(default)]
    pub cooldown_remaining: u16,
    #[serde(default)]
    pub cooldown_turns: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cooldown_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area_radius: Option<u8>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub beam_damage: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cone_radius: Option<u8>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub teleport: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<AbilitySummonSpecDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detect: Option<AbilityDetectSpecDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terrain_transform: Option<AbilityTerrainTransformSpecDto>,
    #[serde(default)]
    pub effects: Vec<AbilityEffectSpecDto>,
    pub target_spec: TargetSpecDto,
    pub learned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub book_item_id: Option<String>,
    pub can_study: bool,
    #[serde(default)]
    pub can_forget: bool,
    pub can_cast: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilitySummonSpecDto {
    pub actor_kind_id: String,
    pub count: u8,
    pub radius: u8,
    pub duration_turns: u16,
    #[serde(default)]
    pub hostile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityDetectSpecDto {
    #[serde(default)]
    pub subject: AbilityDetectSubjectDto,
    pub category: String,
    pub radius: u8,
    #[serde(default)]
    pub persistent: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityTerrainTransformSpecDto {
    pub source_terrain_ids: Vec<String>,
    pub target_terrain_id: String,
    pub radius: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AttributeKindDto {
    Strength,
    Intelligence,
    Wisdom,
    Dexterity,
    Constitution,
    Charisma,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AttributeValueDto {
    pub natural: u16,
    #[serde(default)]
    pub maximum_natural: u16,
    pub effective: u16,
    pub index: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AttributeSetDto {
    pub strength: AttributeValueDto,
    pub intelligence: AttributeValueDto,
    pub wisdom: AttributeValueDto,
    pub dexterity: AttributeValueDto,
    pub constitution: AttributeValueDto,
    pub charisma: AttributeValueDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerProgressDto {
    pub level: u16,
    pub max_level: u16,
    pub experience: u64,
    #[serde(default)]
    pub maximum_experience: u64,
    #[serde(default = "default_life_force")]
    pub life_force: u16,
    pub level_cap: u16,
    #[serde(default)]
    pub attribute_cap: u16,
    pub attribute_index_cap: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience_for_next_level: Option<u64>,
    pub pending_attribute_increases: u16,
    pub victory_level_cap_unlocked: bool,
    pub attributes: AttributeSetDto,
    #[serde(default)]
    pub skills: Vec<SkillProgressDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct SkillProgressDto {
    pub id: String,
    pub name_key: String,
    pub current: i32,
    pub maximum: i32,
    pub base: i32,
    pub growth_per_ten_levels: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerBuildDto {
    pub build_id: String,
    pub build_name_key: String,
    pub race_id: String,
    pub race_name_key: String,
    pub class_id: String,
    pub class_name_key: String,
    pub personality_id: String,
    pub personality_name_key: String,
    pub life_percent: u16,
    pub experience_percent: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TerrainInteractionUnavailableReasonDto {
    OccupiedByActor,
    OccupiedByItem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TerrainInteractionDto {
    pub kind: TerrainInteractionKindDto,
    pub direction: Direction,
    pub position: Position,
    pub terrain_id: String,
    pub requires_check: bool,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<TerrainInteractionUnavailableReasonDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatusKindDto {
    Abandoned,
    Available,
    Active,
    Completed,
    Failed,
    Locked,
    Paused,
    RewardAvailable,
    Taken,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskStatusDto {
    #[serde(default)]
    pub task_id: String,
    pub floor_id: String,
    pub name_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_facility_id: Option<String>,
    pub status: TaskStatusKindDto,
    #[serde(default)]
    pub current: u32,
    #[serde(default = "default_task_required")]
    pub required: u32,
    #[serde(default = "default_task_stage")]
    pub stage: u32,
    #[serde(default = "default_task_stage")]
    pub stages: u32,
    #[serde(default)]
    pub retakes_used: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "bindings", schemars(range(min = 1, max = 16)))]
    pub max_retakes: Option<u16>,
}

const fn default_task_required() -> u32 {
    1
}

const fn default_task_stage() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellDto {
    pub position: Position,
    pub terrain_id: String,
    pub item_id: Option<String>,
    pub actor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub danger_level: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<WildernessLocationDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityState {
    Visible,
    Remembered,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellLightDto {
    pub color: u32,
    pub intensity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellVisualDto {
    pub position: Position,
    pub visibility: VisibilityState,
    pub light: CellLightDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ContentVisualDto {
    pub id: String,
    pub glyph: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum DamageTypeDto {
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
    Curse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ResistanceLevelDto {
    Vulnerable,
    Normal,
    Resistant,
    Strong,
    Immune,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ResistanceDto {
    pub damage_type: DamageTypeDto,
    pub level: ResistanceLevelDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum SlayTargetDto {
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
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum SlayLevelDto {
    Slay,
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum WeaponBrandDto {
    Acid,
    Electricity,
    Fire,
    Cold,
    Poison,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct SlayDto {
    pub target: SlayTargetDto,
    pub level: SlayLevelDto,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RolledAffixSaveDto {
    pub affix_id: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default, skip_serializing_if = "EquipmentBonusesDto::is_empty")]
    pub equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slays: Vec<SlayDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brands: Vec<WeaponBrandDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passives: Vec<EquipmentPassiveDto>,
}

impl<'de> Deserialize<'de> for RolledAffixSaveDto {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Wire {
            affix_id: String,
            #[serde(default)]
            modifiers: StatModifiersDto,
            #[serde(default)]
            equipment_bonuses: EquipmentBonusesDto,
            #[serde(default)]
            resistances: Vec<ResistanceDto>,
            #[serde(default)]
            status_immunities: Vec<String>,
            #[serde(default)]
            slays: Vec<SlayDto>,
            #[serde(default)]
            brands: Vec<WeaponBrandDto>,
            #[serde(default)]
            passives: Vec<String>,
        }

        let wire = Wire::deserialize(deserializer)?;
        Ok(Self {
            affix_id: wire.affix_id,
            modifiers: wire.modifiers,
            equipment_bonuses: wire.equipment_bonuses,
            resistances: wire.resistances,
            status_immunities: wire.status_immunities,
            slays: wire.slays,
            brands: wire.brands,
            passives: migrate_rolled_affix_passives(wire.passives)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageResolutionDto {
    pub raw_damage: i32,
    pub armor_reduction: i32,
    pub resistance_adjustment: i32,
    pub final_damage: i32,
    pub damage_type: DamageTypeDto,
    pub resistance: ResistanceLevelDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum CheckOutcomeDto {
    AutomaticSuccess,
    AutomaticFailure,
    Success,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CheckResolutionDto {
    pub skill_id: String,
    pub ability: i32,
    pub difficulty: i32,
    pub percentile_roll: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contest_roll: Option<i32>,
    pub threshold: i32,
    pub outcome: CheckOutcomeDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityCastResolutionDto {
    pub ability_id: String,
    pub resource_id: String,
    #[serde(default)]
    pub base_resource_cost: u32,
    pub resource_cost: u32,
    pub resource_before: u32,
    pub resource_after: u32,
    pub failure_percent: u8,
    pub percentile_roll: u8,
    pub succeeded: bool,
    #[serde(default)]
    pub proficiency_before: u16,
    #[serde(default)]
    pub proficiency_after: u16,
    #[serde(default)]
    pub proficiency_rank: AbilityProficiencyRankDto,
    #[serde(default)]
    pub cast_count: u32,
    #[serde(default)]
    pub fail_count: u32,
    #[serde(default)]
    pub cooldown_before: u16,
    #[serde(default)]
    pub cooldown_after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityAreaDamageResolutionDto {
    pub center: Position,
    pub radius: u8,
    pub base_raw_damage: i32,
    pub damage_type: DamageTypeDto,
    pub affected_positions: Vec<Position>,
    pub target_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityVisibleDamageResolutionDto {
    pub base_raw_damage: i32,
    pub damage_type: DamageTypeDto,
    pub affected_positions: Vec<Position>,
    pub target_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityBeamDamageResolutionDto {
    pub base_raw_damage: i32,
    pub damage_type: DamageTypeDto,
    pub affected_positions: Vec<Position>,
    pub target_count: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityConeDamageResolutionDto {
    pub radius: u8,
    pub base_raw_damage: i32,
    pub damage_type: DamageTypeDto,
    pub affected_positions: Vec<Position>,
    pub target_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityTeleportResolutionDto {
    pub from: Position,
    pub to: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilitySummonResolutionDto {
    pub owner_id: String,
    pub actor_kind_id: String,
    pub entity_ids: Vec<String>,
    pub positions: Vec<Position>,
    pub duration_turns: u16,
    #[serde(default)]
    pub hostile: bool,
    #[serde(default)]
    pub group: bool,
    /// Per-entity kinds for category summons; empty for fixed-kind summons,
    /// where `actor_kind_id` already names the summoned definition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub summoned_kind_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityDetectResolutionDto {
    #[serde(default)]
    pub subject: AbilityDetectSubjectDto,
    pub category: String,
    pub radius: u8,
    #[serde(default)]
    pub persistent: bool,
    #[serde(default)]
    pub detected_positions: Vec<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub detected_entity_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityTerrainTransformResolutionDto {
    pub center: Position,
    pub radius: u8,
    pub source_terrain_ids: Vec<String>,
    pub target_terrain_id: String,
    #[serde(default)]
    pub transformed_positions: Vec<Position>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityStatusChangeDto {
    Added,
    Replaced,
    Extended,
    Strengthened,
    Unchanged,
    Immune,
    Resisted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum AbilityEffectSkipReasonDto {
    NoTarget,
    TargetDead,
    Saved,
    Ineligible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum AbilityEffectResolutionDto {
    RandomChoice {
        effect_index: u8,
        roll: u16,
        branch_index: u16,
        maximum_roll: u16,
    },
    Damage {
        effect_index: u8,
        resolution: DamageResolutionDto,
    },
    DeathRay {
        effect_index: u8,
        power: u32,
        target_level: u32,
        living: bool,
        unique: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        unique_roll: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_level_roll: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        caster_level_roll: Option<u32>,
        resisted: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resolution: Option<DamageResolutionDto>,
    },
    Heal {
        effect_index: u8,
        resolution: HealingResolutionDto,
    },
    ApplyStatus {
        effect_index: u8,
        status_kind_id: String,
        intensity: u16,
        requested_duration_ticks: u32,
        applied_duration_ticks: u32,
        stacking: AbilityStatusStackingDto,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resistance_type: Option<DamageTypeDto>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resistance: Option<ResistanceLevelDto>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        power: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_level: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        power_roll: Option<u16>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_roll: Option<u32>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        granted_resistances: Vec<ResistanceDto>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        granted_brands: Vec<WeaponBrandDto>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        granted_race_id: Option<String>,
        #[serde(default)]
        grants_wall_passage: bool,
        #[serde(default = "default_incoming_damage_percent")]
        incoming_damage_percent: u8,
        change: AbilityStatusChangeDto,
    },
    RemoveStatus {
        effect_index: u8,
        status_kind_id: String,
        removed: bool,
    },
    Skipped {
        effect_index: u8,
        reason: AbilityEffectSkipReasonDto,
    },
    DrainResource {
        effect_index: u8,
        resource_id: String,
        requested: u32,
        drained: u32,
        caster_healed: u32,
    },
    Amnesia {
        effect_index: u8,
        cleared_cells: u32,
    },
    DarkenRoom {
        effect_index: u8,
        cleared_cells: u32,
    },
    AggravateMonsters {
        effect_index: u8,
        awakened: u32,
        hastened: u32,
    },
    Control {
        effect_index: u8,
        category: String,
        power: u16,
        target_entity_id: String,
        target_kind_id: String,
        target_level: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        roll: Option<u16>,
        outcome: AbilityControlOutcomeDto,
    },
    DrainLife {
        effect_index: u8,
        resolution: DamageResolutionDto,
        healing: HealingResolutionDto,
    },
    Genocide {
        effect_index: u8,
        scope: AbilityGenocideScopeDto,
        power: u16,
        #[serde(default)]
        radius: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        glyph: Option<String>,
        removed_entity_ids: Vec<String>,
        resisted_entity_ids: Vec<String>,
        fatigue_damage: i32,
    },
    IdentifyItem {
        effect_index: u8,
        item_id: String,
        item_kind_id: String,
        full_identify_power: u16,
        full_identify_roll_sides: u16,
        roll: u16,
        full: bool,
        changed: bool,
    },
    RestoreVitality {
        effect_index: u8,
        experience_before: u64,
        experience_after: u64,
        life_force_before: u16,
        life_force_after: u16,
    },
    AnimateDead {
        effect_index: u8,
        actor_kind_id: String,
        consumed_corpse_item_ids: Vec<String>,
        entity_ids: Vec<String>,
        positions: Vec<Position>,
    },
    EnchantEquippedWeapon {
        effect_index: u8,
        item_id: String,
        item_kind_id: String,
        affix_id: String,
        added: bool,
    },
    NoOp {
        effect_index: u8,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct AbilityEffectsResolutionDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_kind_id: Option<String>,
    pub effects: Vec<AbilityEffectResolutionDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum MonsterAbilityRejectionReasonDto {
    InvalidTarget,
    OutOfRange,
    Blocked,
    FriendlyRisk,
    NoSpace,
    NoCandidates,
    NoUtility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MonsterAbilityCandidateResolutionDto {
    pub ability_id: String,
    pub base_weight: u32,
    pub effective_weight: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_entity_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_kind_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_position: Option<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_positions: Vec<Position>,
    #[serde(default)]
    pub enemy_target_count: u16,
    #[serde(default)]
    pub friendly_risk_count: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<MonsterAbilityRejectionReasonDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MonsterAbilityDecisionResolutionDto {
    pub source_entity_id: String,
    pub source_kind_id: String,
    pub frequency_percent: u8,
    pub frequency_roll: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidates: Vec<MonsterAbilityCandidateResolutionDto>,
    pub viable_ability_ids: Vec<String>,
    pub total_weight: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_roll: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_ability_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MonsterAbilityCastResolutionDto {
    pub source_entity_id: String,
    pub source_kind_id: String,
    pub ability_id: String,
    pub target_entity_id: String,
    pub target_kind_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_positions: Vec<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<AbilitySummonResolutionDto>,
    pub effects: Vec<AbilityEffectResolutionDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<MonsterAbilityTargetResolutionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MonsterAbilityTargetResolutionDto {
    pub target_entity_id: String,
    pub target_kind_id: String,
    pub target_position: Position,
    pub effects: Vec<AbilityEffectResolutionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonCommandResolutionDto {
    pub command: SummonCommandDto,
    pub affected_summons: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ResourceRecoveryResolutionDto {
    pub resource_id: String,
    pub before: u32,
    pub after: u32,
    pub recovered: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ResourceGainSourceDto {
    MeleeHit,
    MeleeKill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct MonsterDisplacementResolutionDto {
    pub actor_id: String,
    pub from: Position,
    pub to: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ResourceGainResolutionDto {
    pub resource_id: String,
    pub source: ResourceGainSourceDto,
    pub before: u32,
    pub after: u32,
    pub gained: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum RestStopReasonDto {
    Damaged,
    EnemyVisible,
    FullResources,
    InvalidTurns,
    PlayerDied,
    TurnLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct RestResolutionDto {
    pub requested_turns: u16,
    pub completed_turns: u16,
    pub stop_reason: RestStopReasonDto,
    pub resource_recoveries: Vec<ResourceRecoveryResolutionDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GameEventOutcomeDto {
    AbilityAreaDamage {
        resolution: AbilityAreaDamageResolutionDto,
    },
    AbilityVisibleDamage {
        resolution: AbilityVisibleDamageResolutionDto,
    },
    AbilityBeamDamage {
        resolution: AbilityBeamDamageResolutionDto,
    },
    AbilityConeDamage {
        resolution: AbilityConeDamageResolutionDto,
    },
    AbilityTeleport {
        resolution: AbilityTeleportResolutionDto,
    },
    AbilitySummon {
        resolution: AbilitySummonResolutionDto,
    },
    AbilityDetect {
        resolution: AbilityDetectResolutionDto,
    },
    AbilityTerrainTransform {
        resolution: AbilityTerrainTransformResolutionDto,
    },
    AbilityEffects {
        resolution: AbilityEffectsResolutionDto,
    },
    MonsterAbilityDecision {
        resolution: MonsterAbilityDecisionResolutionDto,
    },
    MonsterAbilityCast {
        resolution: MonsterAbilityCastResolutionDto,
    },
    SummonCommand {
        resolution: SummonCommandResolutionDto,
    },
    AbilityCast {
        resolution: AbilityCastResolutionDto,
    },
    Check {
        resolution: CheckResolutionDto,
    },
    Damage {
        resolution: DamageResolutionDto,
    },
    Death {
        resolution: DamageResolutionDto,
    },
    Heal {
        resolution: HealingResolutionDto,
    },
    ResourceRecovery {
        resolution: ResourceRecoveryResolutionDto,
    },
    ResourceGain {
        resolution: ResourceGainResolutionDto,
    },
    ItemIdentify {
        resolution: ItemIdentifyResolutionDto,
    },
    ItemEnchantment {
        resolution: ItemEnchantmentResolutionDto,
    },
    ItemCurse {
        resolution: ItemCurseResolutionDto,
    },
    ItemCurseRemoval {
        resolution: ItemCurseRemovalResolutionDto,
    },
    ItemSummon {
        resolution: AbilitySummonResolutionDto,
    },
    MonsterDisplacement {
        resolution: MonsterDisplacementResolutionDto,
    },
    Rest {
        resolution: RestResolutionDto,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct HealingResolutionDto {
    pub requested: i32,
    pub applied: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct StatusDto {
    pub kind_id: String,
    pub intensity: u16,
    pub remaining_ticks: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_brands: Vec<WeaponBrandDto>,
    #[serde(default)]
    pub granted_modifiers: StatModifiersDto,
    #[serde(default)]
    pub granted_equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_status_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_race_id: Option<String>,
    #[serde(default)]
    pub grants_wall_passage: bool,
    #[serde(default = "default_incoming_damage_percent")]
    pub incoming_damage_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    #[serde(default)]
    pub gold: u32,
    #[serde(default = "default_player_nutrition")]
    pub nutrition: u16,
    #[serde(default)]
    pub nutrition_state: NutritionStateDto,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    #[serde(default)]
    pub energy_need: i32,
    #[serde(default)]
    pub carried_weight_tenths_pound: u32,
    #[serde(default)]
    pub carry_capacity_tenths_pound: u32,
    #[serde(default)]
    pub encumbrance_speed_penalty: u16,
    #[serde(default)]
    pub inventory_used_slots: u16,
    #[serde(default)]
    pub inventory_slot_capacity: u16,
    #[serde(default)]
    pub base_max_hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub base_attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub base_defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
    #[serde(default)]
    pub melee_profile: AttackProfileDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default)]
    pub is_dead: bool,
    #[serde(default)]
    pub equipment_modifiers: StatModifiersDto,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
    #[serde(default)]
    pub confusing_strike_ready: bool,
    #[serde(default)]
    pub resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "is_default_player_progress")]
    pub progress: PlayerProgressDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<PlayerBuildDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourcePoolDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_recharge: Option<DeviceRechargeDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ability_learning: Option<AbilityLearningDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<AbilityDto>,
    #[serde(default, skip_serializing_if = "is_default_summon_command")]
    pub summon_command: SummonCommandDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recall: Option<RecallStateDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub riding_actor_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum NutritionStateDto {
    Bloated,
    Full,
    #[default]
    Normal,
    Hungry,
    Weak,
    Faint,
    Starving,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecallStateDto {
    pub dungeon_id: String,
    pub floor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_turns: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DeviceRechargeDto {
    pub resource_id: String,
    pub power: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EntityDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub speed: u16,
    #[serde(default = "default_monster_energy_need")]
    pub energy_need: i32,
    #[serde(default = "default_actor_alerted")]
    pub alerted: bool,
    #[serde(default)]
    pub casting_cooldown_remaining: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_player_resistances: Vec<ResistanceDto>,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
    #[serde(default)]
    pub melee_profile: AttackProfileDto,
    #[serde(default)]
    pub melee_routine: MeleeRoutineDto,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
    #[serde(default)]
    pub faction: EntityFactionDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<SummonDto>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum EntityFactionDto {
    #[default]
    Hostile,
    Player,
    Friendly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct SummonDto {
    pub owner_id: String,
    pub source_ability_id: String,
    pub remaining_turns: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    pub position: Position,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum GoldAppearanceDto {
    Copper,
    Silver,
    Garnets,
    Gold,
    Opals,
    Sapphires,
    Rubies,
    Diamonds,
    Emeralds,
    Mithril,
    Adamantite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GoldPileDto {
    pub id: String,
    pub position: Position,
    pub amount: u32,
    pub appearance: GoldAppearanceDto,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemKnowledgeDto {
    Unknown,
    Tried,
    #[default]
    Aware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemChargesDto {
    pub current: u32,
    pub maximum: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemFuelKindDto {
    Torch,
    Lantern,
    Oil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemFuelDto {
    pub kind: ItemFuelKindDto,
    pub current: u16,
    pub maximum: u16,
    pub light_radius: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemActivationDto {
    pub profile_id: String,
    pub name_key: String,
    pub power: u16,
    pub cost: u32,
    pub device_check_difficulty: i32,
    pub target_spec: TargetSpecDto,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemQualityDto {
    #[default]
    Ordinary,
    Fine,
    Exceptional,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemIdentificationDto {
    #[default]
    Unexamined,
    Appraised,
    Identified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemIdentifyResolutionDto {
    pub item_id: String,
    pub item_kind_id: String,
    pub full: bool,
    pub changed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemEnchantmentsDto {
    pub to_hit: u16,
    pub to_damage: u16,
    pub to_armor: u16,
}

impl ItemEnchantmentsDto {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.to_hit == 0 && self.to_damage == 0 && self.to_armor == 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemEnchantmentComponentResolutionDto {
    pub attempts: u16,
    pub successes: u16,
    pub before: u16,
    pub after: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemEnchantmentResolutionDto {
    pub item_id: String,
    pub item_kind_id: String,
    pub to_hit: ItemEnchantmentComponentResolutionDto,
    pub to_damage: ItemEnchantmentComponentResolutionDto,
    pub to_armor: ItemEnchantmentComponentResolutionDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ItemCurseSeverityDto {
    Normal,
    Heavy,
    Permanent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemCurseResolutionDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_kind_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<ItemCurseSeverityDto>,
    pub resisted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemCurseRemovalResolutionDto {
    pub include_heavy: bool,
    pub removed_item_ids: Vec<String>,
    pub retained_permanent_item_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemPropertyDto {
    pub affix_id: String,
    pub name_key: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default, skip_serializing_if = "EquipmentBonusesDto::is_empty")]
    pub equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passives: Vec<EquipmentPassiveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    #[serde(default)]
    pub usable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_target_spec: Option<TargetSpecDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub requires_target_glyph: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub requires_recharge_targets: bool,
    #[serde(default)]
    pub can_receive_recharge: bool,
    #[serde(default)]
    pub can_supply_recharge: bool,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default)]
    pub weight_tenths_pound: u16,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default, skip_serializing_if = "EquipmentBonusesDto::is_empty")]
    pub equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slays: Vec<SlayDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brands: Vec<WeaponBrandDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passives: Vec<EquipmentPassiveDto>,
    #[serde(default)]
    pub identification: ItemIdentificationDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<ItemQualityDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_properties: Vec<ItemPropertyDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub melee_profile: Option<AttackProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throw_profile: Option<ThrowProfileDto>,
}

/// One equipment slot instance on the player's body: `slot_type` matches
/// item `equipmentSlot` declarations, `id` names the concrete instance so
/// several slots of one type (two rings) stay addressable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct BodySlotDto {
    pub id: String,
    pub slot_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default)]
    pub display_name_key: String,
    #[serde(default)]
    pub knowledge: ItemKnowledgeDto,
    pub quantity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default)]
    pub weight_tenths_pound: u16,
    pub slot_id: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
    #[serde(default, skip_serializing_if = "EquipmentBonusesDto::is_empty")]
    pub equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slays: Vec<SlayDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub brands: Vec<WeaponBrandDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub passives: Vec<EquipmentPassiveDto>,
    #[serde(default)]
    pub identification: ItemIdentificationDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<ItemQualityDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_properties: Vec<ItemPropertyDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub melee_profile: Option<AttackProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_profile: Option<ProjectileProfileDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub throw_profile: Option<ThrowProfileDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameEventDto {
    pub kind: String,
    pub message_key: String,
    pub args: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<GameEventOutcomeDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<ProjectileTraceDto>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum CampaignStatusDto {
    Active,
    Victorious,
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CampaignStateDto {
    pub status: CampaignStatusDto,
    pub score: u64,
    pub conquered_dungeons: u32,
    pub completed_tasks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory_turn: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_turn: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TownDto {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub floor_id: String,
    pub visited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum ShopCategoryDto {
    GeneralStore,
    Armoury,
    Weaponsmith,
    Temple,
    Alchemist,
    MagicShop,
    BlackMarket,
    Bookstore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ShopOwnerDto {
    pub id: String,
    pub name_key: String,
    pub race_id: String,
    pub greed_percent: u16,
    pub purchase_price_cap: u32,
    pub price_factor_percent: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ShopStockItemDto {
    pub id: String,
    pub kind_id: String,
    pub display_name_key: String,
    pub quantity: u32,
    pub maximum_quantity: u32,
    pub unit_price: u32,
    pub weight_tenths_pound: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    pub quality: ItemQualityDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ShopSellQuoteDto {
    pub item_id: String,
    pub kind_id: String,
    pub unit_price: u32,
    pub maximum_quantity: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ShopDto {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub category: ShopCategoryDto,
    pub entrance_position: Position,
    pub entrance_terrain_id: String,
    pub visited: bool,
    pub player_at_entrance: bool,
    pub owner: ShopOwnerDto,
    #[serde(default)]
    pub stock: Vec<ShopStockItemDto>,
    #[serde(default)]
    pub sell_quotes: Vec<ShopSellQuoteDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct HomeItemDto {
    pub id: String,
    pub kind_id: String,
    pub display_name_key: String,
    pub quantity: u32,
    pub maximum_quantity: u32,
    pub weight_tenths_pound: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct HomeDto {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub entrance_position: Position,
    pub entrance_terrain_id: String,
    pub visited: bool,
    pub player_at_entrance: bool,
    #[serde(default)]
    pub stored_items: Vec<HomeItemDto>,
    #[serde(default)]
    pub deposit_items: Vec<HomeItemDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct TaskServiceDto {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub owner_name_key: String,
    pub entrance_position: Position,
    pub entrance_terrain_id: String,
    pub player_at_entrance: bool,
    #[serde(default)]
    pub tasks: Vec<TaskStatusDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshot {
    pub protocol_version: String,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub last_command_seq: u32,
    #[serde(default)]
    pub map_scale: MapScaleDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_travel_destination: Option<Position>,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<CellDto>,
    #[serde(default)]
    pub visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    #[serde(default)]
    pub gold_piles: Vec<GoldPileDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
    #[serde(default)]
    pub body_slots: Vec<BodySlotDto>,
    pub content_id: String,
    pub content_hash: String,
    pub content_visuals: Vec<ContentVisualDto>,
    pub world_id: String,
    pub floor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub town: Option<TownDto>,
    #[serde(default)]
    pub shops: Vec<ShopDto>,
    #[serde(default)]
    pub homes: Vec<HomeDto>,
    #[serde(default)]
    pub task_services: Vec<TaskServiceDto>,
    #[serde(default)]
    pub terrain_interactions: Vec<TerrainInteractionDto>,
    #[serde(default)]
    pub tasks: Vec<TaskStatusDto>,
    pub campaign: CampaignStateDto,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameUpdate {
    pub base_revision: u32,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub command_seq: u32,
    #[serde(default)]
    pub map_scale: MapScaleDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_travel_destination: Option<Position>,
    pub width: u16,
    pub height: u16,
    pub floor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub town: Option<TownDto>,
    #[serde(default)]
    pub shops: Vec<ShopDto>,
    #[serde(default)]
    pub homes: Vec<HomeDto>,
    #[serde(default)]
    pub task_services: Vec<TaskServiceDto>,
    pub events: Vec<GameEventDto>,
    pub changed_cells: Vec<CellDto>,
    #[serde(default)]
    pub changed_visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    #[serde(default)]
    pub gold_piles: Vec<GoldPileDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
    pub removed_entities: Vec<String>,
    #[serde(default)]
    pub terrain_interactions: Vec<TerrainInteractionDto>,
    #[serde(default)]
    pub tasks: Vec<TaskStatusDto>,
    pub campaign: CampaignStateDto,
    pub state_hash: String,
}

/// Schema bundle for the types crossing the CoreTransport boundary.
#[cfg(feature = "bindings")]
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolSchemaV1 {
    pub command: GameCommandEnvelope,
    pub snapshot: GameSnapshot,
    pub update: GameUpdate,
}

#[must_use]
#[cfg(feature = "bindings")]
pub fn generated_typescript() -> String {
    let config = Config::default();
    let mut output = String::from(
        "// SPDX-License-Identifier: MPL-2.0\n\
         // @generated by `cargo run -p rfb-protocol --features bindings --bin generate-bindings`; do not edit.\n\n",
    );

    macro_rules! push_declaration {
        ($type:ty) => {{
            let declaration = format!("export {}", <$type as TS>::decl(&config));
            for line in declaration.lines() {
                output.push_str(line.trim_end());
                output.push('\n');
            }
            output.push('\n');
        }};
    }

    push_declaration!(Direction);
    push_declaration!(DeviceRechargeSourceDto);
    push_declaration!(GameCommand);
    push_declaration!(GameCommandEnvelope);
    push_declaration!(StatModifiersDto);
    push_declaration!(EquipmentBonusesDto);
    push_declaration!(EquipmentPassiveDto);
    push_declaration!(AttributeKindDto);
    push_declaration!(AttributeValueDto);
    push_declaration!(AttributeSetDto);
    push_declaration!(PlayerProgressDto);
    push_declaration!(SkillProgressDto);
    push_declaration!(PlayerBuildDto);
    push_declaration!(DamageDiceDto);
    push_declaration!(AttackProfileDto);
    push_declaration!(MeleeBlowDto);
    push_declaration!(MeleeRoutineDto);
    push_declaration!(TargetModeDto);
    push_declaration!(TargetSpecDto);
    push_declaration!(ResourcePoolDto);
    push_declaration!(AbilityLearningDto);
    push_declaration!(AbilityProficiencyRankDto);
    push_declaration!(AbilityStatusStackingDto);
    push_declaration!(AbilityDetectSubjectDto);
    push_declaration!(AbilityControlOutcomeDto);
    push_declaration!(AbilityGenocideScopeDto);
    push_declaration!(AbilityRandomTargetDto);
    push_declaration!(AbilityRandomBranchSpecDto);
    push_declaration!(AbilityEffectSpecDto);
    push_declaration!(AbilitySummonSpecDto);
    push_declaration!(AbilityDetectSpecDto);
    push_declaration!(AbilityTerrainTransformSpecDto);
    push_declaration!(AbilityDto);
    push_declaration!(TargetSelection);
    push_declaration!(ProjectileProfileDto);
    push_declaration!(ThrowProfileDto);
    push_declaration!(ProjectileTraceDto);
    push_declaration!(Position);
    push_declaration!(MapScaleDto);
    push_declaration!(WildernessLocationKindDto);
    push_declaration!(WildernessLocationDto);
    push_declaration!(TerrainInteractionKindDto);
    push_declaration!(TerrainInteractionUnavailableReasonDto);
    push_declaration!(TerrainInteractionDto);
    push_declaration!(TaskStatusKindDto);
    push_declaration!(TaskStatusDto);
    push_declaration!(CellDto);
    push_declaration!(VisibilityState);
    push_declaration!(CellLightDto);
    push_declaration!(CellVisualDto);
    push_declaration!(ContentVisualDto);
    push_declaration!(DamageTypeDto);
    push_declaration!(ResistanceLevelDto);
    push_declaration!(ResistanceDto);
    push_declaration!(SlayTargetDto);
    push_declaration!(SlayLevelDto);
    push_declaration!(WeaponBrandDto);
    push_declaration!(SlayDto);
    push_declaration!(DamageResolutionDto);
    push_declaration!(CheckOutcomeDto);
    push_declaration!(CheckResolutionDto);
    push_declaration!(AbilityCastResolutionDto);
    push_declaration!(AbilityAreaDamageResolutionDto);
    push_declaration!(AbilityVisibleDamageResolutionDto);
    push_declaration!(AbilityBeamDamageResolutionDto);
    push_declaration!(AbilityConeDamageResolutionDto);
    push_declaration!(AbilityTeleportResolutionDto);
    push_declaration!(AbilitySummonResolutionDto);
    push_declaration!(AbilityDetectResolutionDto);
    push_declaration!(AbilityTerrainTransformResolutionDto);
    push_declaration!(AbilityStatusChangeDto);
    push_declaration!(AbilityEffectSkipReasonDto);
    push_declaration!(AbilityEffectResolutionDto);
    push_declaration!(AbilityEffectsResolutionDto);
    push_declaration!(MonsterAbilityRejectionReasonDto);
    push_declaration!(MonsterAbilityCandidateResolutionDto);
    push_declaration!(MonsterAbilityDecisionResolutionDto);
    push_declaration!(MonsterAbilityTargetResolutionDto);
    push_declaration!(MonsterAbilityCastResolutionDto);
    push_declaration!(SummonCommandModeDto);
    push_declaration!(SummonCommandDto);
    push_declaration!(SummonCommandResolutionDto);
    push_declaration!(ResourceRecoveryResolutionDto);
    push_declaration!(ResourceGainSourceDto);
    push_declaration!(MonsterDisplacementResolutionDto);
    push_declaration!(ResourceGainResolutionDto);
    push_declaration!(RestStopReasonDto);
    push_declaration!(RestResolutionDto);
    push_declaration!(HealingResolutionDto);
    push_declaration!(GameEventOutcomeDto);
    push_declaration!(StatusDto);
    push_declaration!(DeviceRechargeDto);
    push_declaration!(RecallStateDto);
    push_declaration!(NutritionStateDto);
    push_declaration!(PlayerDto);
    push_declaration!(EntityFactionDto);
    push_declaration!(SummonDto);
    push_declaration!(EntityDto);
    push_declaration!(ItemDto);
    push_declaration!(ItemFuelKindDto);
    push_declaration!(ItemFuelDto);
    push_declaration!(GoldAppearanceDto);
    push_declaration!(GoldPileDto);
    push_declaration!(ItemKnowledgeDto);
    push_declaration!(ItemChargesDto);
    push_declaration!(ItemActivationDto);
    push_declaration!(ItemQualityDto);
    push_declaration!(ItemIdentificationDto);
    push_declaration!(ItemIdentifyResolutionDto);
    push_declaration!(ItemEnchantmentsDto);
    push_declaration!(ItemEnchantmentComponentResolutionDto);
    push_declaration!(ItemEnchantmentResolutionDto);
    push_declaration!(ItemCurseSeverityDto);
    push_declaration!(ItemCurseResolutionDto);
    push_declaration!(ItemCurseRemovalResolutionDto);
    push_declaration!(ItemPropertyDto);
    push_declaration!(InventoryItemDto);
    push_declaration!(BodySlotDto);
    push_declaration!(EquipmentItemDto);
    push_declaration!(GameEventDto);
    push_declaration!(CampaignStatusDto);
    push_declaration!(CampaignStateDto);
    push_declaration!(TownDto);
    push_declaration!(ShopCategoryDto);
    push_declaration!(ShopOwnerDto);
    push_declaration!(ShopStockItemDto);
    push_declaration!(ShopSellQuoteDto);
    push_declaration!(ShopDto);
    push_declaration!(HomeItemDto);
    push_declaration!(HomeDto);
    push_declaration!(TaskServiceDto);
    push_declaration!(GameSnapshot);
    push_declaration!(GameUpdate);

    output
}

#[cfg(feature = "bindings")]
pub fn generated_json_schema() -> Result<String, serde_json::Error> {
    let mut output = serde_json::to_string_pretty(&schema_for!(ProtocolSchemaV1))?;
    output.push('\n');
    Ok(output)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainSaveDto {
    pub width: u16,
    pub height: u16,
    pub terrain_ids: Vec<String>,
    pub glow: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RngSaveDto {
    pub algorithm: String,
    pub state: [u64; 4],
    pub draw_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub gold: u32,
    #[serde(default = "default_player_nutrition")]
    pub nutrition: u16,
    #[serde(default)]
    pub base_max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub base_speed: u16,
    #[serde(default)]
    pub energy_need: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub confusing_strike_ready: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<PlayerProgressSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<PlayerBuildSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<ResourcePoolSaveDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub bonus_spell_learning_capacity: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub learned_ability_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ability_progress: Vec<AbilityProgressSaveDto>,
    #[serde(default, skip_serializing_if = "is_default_summon_command")]
    pub summon_command: SummonCommandDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub body_slots: Vec<BodySlotSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recall: Option<RecallStateDto>,
    pub riding_actor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BodySlotSaveDto {
    pub id: String,
    pub slot_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AbilityProgressSaveDto {
    pub id: String,
    pub proficiency: u16,
    pub proficiency_cap: u16,
    #[serde(default)]
    pub cast_count: u32,
    #[serde(default)]
    pub fail_count: u32,
    #[serde(default)]
    pub cooldown_remaining: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourcePoolSaveDto {
    pub id: String,
    pub current: u32,
    pub maximum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerBuildSaveDto {
    pub build_id: String,
    pub race_id: String,
    pub class_id: String,
    pub personality_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NaturalAttributeSetSaveDto {
    pub strength: u16,
    pub intelligence: u16,
    pub wisdom: u16,
    pub dexterity: u16,
    pub constitution: u16,
    pub charisma: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlayerProgressSaveDto {
    pub attributes: NaturalAttributeSetSaveDto,
    #[serde(default)]
    pub maximum_attributes: Option<NaturalAttributeSetSaveDto>,
    pub experience: u64,
    #[serde(default)]
    pub maximum_experience: u64,
    #[serde(default = "default_life_force")]
    pub life_force: u16,
    pub level: u16,
    pub max_level: u16,
    pub pending_attribute_increases: u16,
    pub hp_progression: Vec<i32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<SkillProgressSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SkillProgressSaveDto {
    pub id: String,
    pub current: i32,
    pub maximum: i32,
    pub base: i32,
    pub growth_per_ten_levels: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorSaveDto {
    pub id: String,
    pub kind_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub appearance_kind_id: Option<String>,
    pub position: Position,
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
    #[serde(default = "default_actor_speed")]
    pub base_speed: u16,
    #[serde(default = "default_monster_energy_need")]
    pub energy_need: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alerted: Option<bool>,
    pub nice: bool,
    #[serde(default)]
    pub visible_invisible: bool,
    #[serde(default)]
    pub casting_cooldown_remaining: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_player_resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<MonsterPackSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summon: Option<SummonSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummonSaveDto {
    pub owner_id: String,
    pub source_ability_id: String,
    pub remaining_turns: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MonsterPackSaveDto {
    pub id: String,
    pub leader_id: String,
    pub role: MonsterPackRoleDto,
    pub behavior: MonsterPackBehaviorDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MonsterPackRoleDto {
    Leader,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MonsterPackBehaviorDto {
    Seek,
    Surround,
    GuardLeader,
    GuardPosition,
    Lure,
    Shoot,
    MaintainDistance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusSaveDto {
    pub kind_id: String,
    pub intensity: u16,
    pub remaining_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_resistances: Vec<ResistanceSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_brands: Vec<WeaponBrandDto>,
    #[serde(default)]
    pub granted_modifiers: StatModifiersDto,
    #[serde(default)]
    pub granted_equipment_bonuses: EquipmentBonusesDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_status_immunities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub granted_race_id: Option<String>,
    #[serde(default)]
    pub grants_wall_passage: bool,
    #[serde(default = "default_incoming_damage_percent")]
    pub incoming_damage_percent: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResistanceSaveDto {
    pub damage_type: DamageTypeDto,
    pub level: ResistanceLevelDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_affixes: Vec<RolledAffixSaveDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub device_recovery_progress: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_affixes: Vec<RolledAffixSaveDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub device_recovery_progress: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub slot_id: String,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_affixes: Vec<RolledAffixSaveDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub device_recovery_progress: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CarriedItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub actor_id: String,
    #[serde(default)]
    pub quality: ItemQualityDto,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affix_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolled_affixes: Vec<RolledAffixSaveDto>,
    #[serde(default, skip_serializing_if = "ItemEnchantmentsDto::is_empty")]
    pub enchantments: ItemEnchantmentsDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curse: Option<ItemCurseSeverityDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charges: Option<ItemChargesDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuel: Option<ItemFuelDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activation: Option<ItemActivationDto>,
    #[serde(default, skip_serializing_if = "is_zero_u16")]
    pub device_recovery_progress: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorConnectionSaveDto {
    pub id: String,
    pub position: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_floor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_connection_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorRegionSaveDto {
    pub region_id: String,
    pub theme_id: String,
    pub encounter_table_id: String,
    pub loot_table_id: String,
    pub cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorSaveDto {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dungeon_instance_id: Option<String>,
    pub player_position: Position,
    pub terrain: TerrainSaveDto,
    pub entities: Vec<ActorSaveDto>,
    #[serde(default)]
    pub items: Vec<ItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gold_piles: Vec<GoldPileDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_items: Vec<CarriedItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub connections: Vec<FloorConnectionSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<FloorRegionSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemKnowledgeSaveDto {
    pub kind_id: String,
    #[serde(default)]
    pub tried: bool,
    #[serde(default)]
    pub aware: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPropertyKnowledgeSaveDto {
    pub item_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub appraised: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub identified: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub known_affix_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgressSaveDto {
    #[serde(alias = "floorId")]
    pub task_id: String,
    pub current: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskStateSaveDto {
    pub task_id: String,
    pub status: TaskStatusKindDto,
    #[serde(default)]
    pub stage_index: u32,
    pub current: u32,
    pub required: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_floor_id: Option<String>,
    #[serde(default)]
    pub retakes_used: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DungeonStateSaveDto {
    pub dungeon_id: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub guardian_defeated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entrance_guardian_defeated: Option<bool>,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub next_instance_ordinal: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_at_turn: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CampaignStateSaveDto {
    pub status: CampaignStatusDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory_turn: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retired_turn: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub final_score: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TownStateSaveDto {
    pub town_id: String,
    pub visited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShopStateSaveDto {
    pub shop_id: String,
    pub visited: bool,
    pub owner_id: String,
    pub last_maintenance_world_tick: u32,
    pub inventory: Vec<InventoryItemSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeStateSaveDto {
    pub facility_id: String,
    pub visited: bool,
    pub inventory: Vec<InventoryItemSaveDto>,
}

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_zero_u16(value: &u16) -> bool {
    *value == 0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePayloadV1 {
    pub schema_version: u16,
    pub revision: u32,
    pub turn: u32,
    #[serde(default)]
    pub world_tick: u32,
    pub last_command_seq: u32,
    #[serde(default)]
    pub map_scale: MapScaleDto,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wilderness_position: Option<Position>,
    #[serde(default)]
    pub wilderness_seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_travel_destination: Option<Position>,
    pub terrain: TerrainSaveDto,
    pub player: PlayerSaveDto,
    pub entities: Vec<ActorSaveDto>,
    #[serde(default)]
    pub items: Vec<ItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gold_piles: Vec<GoldPileDto>,
    #[serde(default)]
    pub inventory: Vec<InventoryItemSaveDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_items: Vec<CarriedItemSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_knowledge: Vec<ItemKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub item_property_knowledge: Vec<ItemPropertyKnowledgeSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_progress: Vec<TaskProgressSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_states: Vec<TaskStateSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dungeon_states: Vec<DungeonStateSaveDto>,
    pub defeated_unique_actor_kind_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub town_states: Vec<TownStateSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shop_states: Vec<ShopStateSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub home_states: Vec<HomeStateSaveDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_state: Option<CampaignStateSaveDto>,
    #[serde(default)]
    pub next_item_instance_serial: u64,
    #[serde(default)]
    pub next_gold_pile_serial: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub revealed_terrain: Vec<Position>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floor_connections: Vec<FloorConnectionSaveDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floor_regions: Vec<FloorRegionSaveDto>,
    pub rng: RngSaveDto,
    pub content_id: String,
    pub content_hash: String,
    #[serde(default)]
    pub world_id: String,
    #[serde(default)]
    pub current_floor_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_dungeon_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stored_floors: Vec<FloorSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub display_name: String,
    pub level: u32,
    pub location_key: String,
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHeaderV1 {
    pub format: String,
    pub save_schema_version: u16,
    pub game_version: String,
    pub protocol_version: String,
    #[serde(default)]
    pub slot_name: String,
    pub created_at: String,
    pub saved_at: String,
    pub character_summary: CharacterSummary,
    pub content_id: String,
    pub content_hash: String,
    pub payload_encoding: String,
}

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("failed to encode MessagePack: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("failed to decode MessagePack: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    Ok(rmp_serde::to_vec_named(value)?)
}

pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacySavePayloadV1 {
        schema_version: u16,
        revision: u32,
        turn: u32,
        last_command_seq: u32,
        terrain: TerrainSaveDto,
        player: PlayerDto,
        entities: Vec<EntityDto>,
        items: Vec<ItemDto>,
        defeated_unique_actor_kind_ids: Vec<String>,
        inventory: Vec<InventoryItemDto>,
        equipment: Vec<EquipmentItemDto>,
        next_item_instance_serial: u64,
        explored: Vec<bool>,
        rng: RngSaveDto,
        content_id: String,
        content_hash: String,
        world_id: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DerivedFieldProbe {
        #[serde(default)]
        attack: Option<i32>,
        #[serde(default)]
        defense: Option<i32>,
        #[serde(default)]
        melee_skill: Option<i32>,
        #[serde(default)]
        armor_class: Option<i32>,
        #[serde(default)]
        equipment_modifiers: Option<StatModifiersDto>,
    }

    #[test]
    fn command_messagepack_round_trip() {
        for (index, command) in [
            GameCommand::Appraise {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
            GameCommand::BashDoor {
                direction: Direction::South,
            },
            GameCommand::BuyFromShop {
                shop_id: "demo.shop.outpost-general-store".to_owned(),
                item_id: "generated.item.9".to_owned(),
                quantity: 2,
            },
            GameCommand::OpenDoor {
                direction: Direction::East,
            },
            GameCommand::CloseDoor {
                direction: Direction::West,
            },
            GameCommand::Move {
                direction: Direction::SouthEast,
            },
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: "demo.item.shovel.1".to_owned(),
                slot_id: Some("weapon".to_owned()),
            },
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
            GameCommand::Drop {
                item_ids: vec!["demo.item.echo-charm.1".to_owned()],
            },
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
            GameCommand::Fire {
                direction: Direction::East,
            },
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            },
            GameCommand::EnterWorldMap {
                leave_pets: false,
                cancel_recall: false,
            },
            GameCommand::LeaveWorldMap,
            GameCommand::TravelWorld {
                destination: Position { x: 29, y: 52 },
            },
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::North,
            },
            GameCommand::TraverseStairs,
            GameCommand::Search,
            GameCommand::SellToShop {
                shop_id: "demo.shop.outpost-general-store".to_owned(),
                item_id: "generated.item.1".to_owned(),
                quantity: 1,
            },
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
            GameCommand::UseItemByGlyph {
                item_id: "demo.item.glyph-severance-scroll.1".to_owned(),
                glyph: "o".to_owned(),
            },
            GameCommand::UseItemForRecharge {
                item_id: "demo.item.recharging-scroll.1".to_owned(),
                source_item_id: "demo.item.resonance-wand.1".to_owned(),
                target_item_id: "demo.item.resonance-rod.1".to_owned(),
            },
            GameCommand::RechargeItem {
                target_item_id: "demo.item.resonance-rod.1".to_owned(),
                source: DeviceRechargeSourceDto::Resource,
            },
            GameCommand::RechargeItem {
                target_item_id: "demo.item.resonance-rod.1".to_owned(),
                source: DeviceRechargeSourceDto::Item {
                    item_id: "demo.item.resonance-wand.1".to_owned(),
                },
            },
            GameCommand::CastAbility {
                ability_id: "demo.ability.mending-echo".to_owned(),
                target: TargetSelection::SelfTarget,
            },
            GameCommand::StudyAbility {
                book_item_id: "generated.item.2".to_owned(),
                ability_id: "demo.ability.resonant-bolt".to_owned(),
            },
            GameCommand::ForgetAbility {
                ability_id: "demo.ability.resonant-bolt".to_owned(),
            },
            GameCommand::Rest { turns: 100 },
            GameCommand::Wait,
        ]
        .into_iter()
        .enumerate()
        {
            let envelope = GameCommandEnvelope {
                command_seq: index as u32 + 1,
                expected_revision: index as u32,
                command,
            };
            let encoded = to_msgpack(&envelope).expect("command should encode");
            let decoded: GameCommandEnvelope =
                from_msgpack(&encoded).expect("command should decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn legacy_game_event_without_outcome_remains_compatible() {
        let legacy = serde_json::json!({
            "kind": "wait",
            "messageKey": "event-wait",
            "args": {}
        });

        let event: GameEventDto =
            serde_json::from_value(legacy).expect("legacy event should decode");
        assert_eq!(event.outcome, None);

        let encoded = serde_json::to_value(&event).expect("event should encode");
        assert_eq!(encoded.get("outcome"), None);
    }

    #[test]
    fn rolled_affix_save_migrates_supported_passives_and_rejects_unknown_values() {
        let migrated: RolledAffixSaveDto = serde_json::from_value(serde_json::json!({
            "affixId": "demo.affix.adaptive-echo",
            "passives": [
                "hold-life",
                "regeneration",
                "sustain-strength",
                "sustain-intelligence",
                "sustain-wisdom",
                "sustain-dexterity",
                "sustain-constitution",
                "sustain-charisma",
                "telepathy",
                "vampiric"
            ]
        }))
        .expect("known legacy passives should migrate");
        assert_eq!(
            migrated.passives,
            [
                EquipmentPassiveDto::Regeneration,
                EquipmentPassiveDto::SustainStrength,
                EquipmentPassiveDto::SustainIntelligence,
                EquipmentPassiveDto::SustainWisdom,
                EquipmentPassiveDto::SustainDexterity,
                EquipmentPassiveDto::SustainConstitution,
                EquipmentPassiveDto::SustainCharisma,
                EquipmentPassiveDto::Vampiric
            ]
        );

        let encoded = to_msgpack(&serde_json::json!({
            "affixId": "demo.affix.adaptive-echo",
            "passives": [
                "hold-life",
                "regeneration",
                "sustain-strength",
                "sustain-intelligence",
                "sustain-wisdom",
                "sustain-dexterity",
                "sustain-constitution",
                "sustain-charisma",
                "telepathy",
                "vampiric"
            ]
        }))
        .expect("legacy rolled affix should encode");
        let migrated_from_msgpack: RolledAffixSaveDto =
            from_msgpack(&encoded).expect("legacy MessagePack should migrate");
        assert_eq!(migrated_from_msgpack, migrated);

        let unknown = serde_json::from_value::<RolledAffixSaveDto>(serde_json::json!({
            "affixId": "demo.affix.adaptive-echo",
            "passives": ["unknown-passive"]
        }));
        assert!(unknown.is_err(), "unknown passives must remain load errors");
    }

    #[test]
    fn v1_projection_migration_requires_current_actor_state() {
        let legacy = LegacySavePayloadV1 {
            schema_version: 1,
            revision: 2,
            turn: 2,
            last_command_seq: 2,
            terrain: TerrainSaveDto {
                width: 1,
                height: 1,
                terrain_ids: vec!["demo.terrain.floor".to_owned()],
                glow: vec![false],
            },
            player: PlayerDto {
                id: "demo.player".to_owned(),
                kind_id: "demo.actor.explorer".to_owned(),
                position: Position { x: 0, y: 0 },
                hp: 8,
                max_hp: 14,
                gold: 0,
                nutrition: PLAYER_NUTRITION_BIRTH,
                nutrition_state: NutritionStateDto::Normal,
                speed: 110,
                energy_need: 0,
                carried_weight_tenths_pound: 5,
                carry_capacity_tenths_pound: 100,
                encumbrance_speed_penalty: 0,
                inventory_used_slots: 1,
                inventory_slot_capacity: 26,
                base_max_hp: 10,
                attack: 3,
                base_attack: 2,
                defense: 2,
                base_defense: 1,
                melee_skill: 60,
                armor_class: 20,
                melee_damage: DamageDiceDto {
                    dice: 1,
                    sides: 2,
                    damage_type: DamageTypeDto::Physical,
                },
                melee_profile: AttackProfileDto::default(),
                projectile_profile: None,
                is_dead: false,
                equipment_modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                    ..StatModifiersDto::default()
                },
                statuses: Vec::new(),
                confusing_strike_ready: false,
                resistances: Vec::new(),
                progress: PlayerProgressDto::default(),
                build: None,
                resources: Vec::new(),
                device_recharge: None,
                ability_learning: None,
                abilities: Vec::new(),
                summon_command: SummonCommandDto::default(),
                recall: None,
                riding_actor_id: None,
            },
            entities: vec![EntityDto {
                id: "demo.monster.1".to_owned(),
                kind_id: "demo.actor.monster".to_owned(),
                position: Position { x: 1, y: 0 },
                hp: 3,
                max_hp: 3,
                speed: 110,
                energy_need: 100,
                alerted: true,
                casting_cooldown_remaining: 0,
                observed_player_resistances: Vec::new(),
                attack: 1,
                defense: 1,
                melee_skill: 32,
                armor_class: 10,
                melee_damage: DamageDiceDto {
                    dice: 1,
                    sides: 2,
                    damage_type: DamageTypeDto::Physical,
                },
                melee_profile: AttackProfileDto::default(),
                melee_routine: MeleeRoutineDto::default(),
                statuses: Vec::new(),
                faction: EntityFactionDto::Hostile,
                controller_id: None,
                summon: None,
            }],
            items: vec![ItemDto {
                id: "demo.item.ground.1".to_owned(),
                kind_id: "demo.item.shard".to_owned(),
                display_name_key: "item-demo-shard-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                position: Position { x: 0, y: 0 },
                quantity: 2,
                fuel: None,
                enchantments: ItemEnchantmentsDto::default(),
                curse: None,
            }],
            defeated_unique_actor_kind_ids: Vec::new(),
            inventory: vec![InventoryItemDto {
                id: "demo.item.inventory.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                display_name_key: "item-demo-charm-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                usable: false,
                charges: None,
                fuel: None,
                activation: None,
                use_target_spec: None,
                requires_target_glyph: false,
                requires_recharge_targets: false,
                can_receive_recharge: false,
                can_supply_recharge: false,
                quantity: 1,
                enchantments: ItemEnchantmentsDto::default(),
                curse: None,
                weight_tenths_pound: 5,
                equipment_slot: Some("charm".to_owned()),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                    ..StatModifiersDto::default()
                },
                equipment_bonuses: EquipmentBonusesDto::default(),
                resistances: Vec::new(),
                status_immunities: Vec::new(),
                slays: Vec::new(),
                brands: Vec::new(),
                passives: Vec::new(),
                identification: ItemIdentificationDto::Unexamined,
                quality: None,
                known_properties: Vec::new(),
                melee_profile: None,
                projectile_profile: None,
                throw_profile: None,
            }],
            equipment: vec![EquipmentItemDto {
                id: "demo.item.equipment.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                display_name_key: "item-demo-charm-name".to_owned(),
                knowledge: ItemKnowledgeDto::Aware,
                quantity: 1,
                fuel: None,
                enchantments: ItemEnchantmentsDto::default(),
                curse: None,
                weight_tenths_pound: 5,
                slot_id: "charm".to_owned(),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                    ..StatModifiersDto::default()
                },
                equipment_bonuses: EquipmentBonusesDto::default(),
                resistances: Vec::new(),
                status_immunities: Vec::new(),
                slays: Vec::new(),
                brands: Vec::new(),
                passives: Vec::new(),
                identification: ItemIdentificationDto::Unexamined,
                quality: None,
                known_properties: Vec::new(),
                melee_profile: None,
                projectile_profile: None,
                throw_profile: None,
            }],
            next_item_instance_serial: 4,
            explored: vec![true],
            rng: RngSaveDto {
                algorithm: "rfb-rng-xoshiro256ss-v1".to_owned(),
                state: [1, 2, 3, 4],
                draw_counter: 5,
            },
            content_id: "demo.content".to_owned(),
            content_hash: "0".repeat(64),
            world_id: "demo.world".to_owned(),
        };

        let encoded = to_msgpack(&legacy).expect("legacy payload should encode");
        assert!(
            from_msgpack::<SavePayloadV1>(&encoded).is_err(),
            "pre-v190 actor saves without nice must be rejected"
        );
        let mut current = serde_json::to_value(&legacy).expect("fixture should serialize");
        current["entities"][0]["nice"] = serde_json::json!(false);
        let encoded = to_msgpack(&current).expect("current payload should encode");
        let decoded: SavePayloadV1 =
            from_msgpack(&encoded).expect("current actor state should decode");

        assert_eq!(decoded.player.base_max_hp, 10);
        assert_eq!(decoded.entities[0].max_hp, 3);
        assert_eq!(decoded.inventory[0].kind_id, "demo.item.charm");
        assert_eq!(decoded.equipment[0].slot_id, "charm");
        assert!(decoded.item_knowledge.is_empty());
        assert!(decoded.item_property_knowledge.is_empty());
        assert!(decoded.revealed_terrain.is_empty());
        assert!(decoded.floor_regions.is_empty());
        assert!(decoded.inventory[0].affix_ids.is_empty());
        assert_eq!(decoded.inventory[0].quality, ItemQualityDto::Ordinary);
    }

    #[test]
    fn legacy_task_state_defaults_to_zero_retakes() {
        let decoded: TaskStateSaveDto = serde_json::from_value(serde_json::json!({
            "taskId": "demo.task.legacy",
            "status": "paused",
            "stageIndex": 0,
            "current": 1,
            "required": 2
        }))
        .expect("legacy task state should decode");

        assert_eq!(decoded.retakes_used, 0);
    }

    #[test]
    fn authoritative_player_save_omits_derived_combat_fields() {
        let player = PlayerSaveDto {
            id: "demo.player".to_owned(),
            kind_id: "demo.actor.explorer".to_owned(),
            position: Position { x: 0, y: 0 },
            hp: 10,
            gold: 0,
            nutrition: PLAYER_NUTRITION_BIRTH,
            base_max_hp: 10,
            base_speed: 110,
            energy_need: 0,
            statuses: Vec::new(),
            confusing_strike_ready: false,
            resistances: Vec::new(),
            progress: None,
            build: None,
            resources: Vec::new(),
            bonus_spell_learning_capacity: 0,
            learned_ability_ids: Vec::new(),
            ability_progress: Vec::new(),
            summon_command: SummonCommandDto::default(),
            body_slots: Vec::new(),
            recall: None,
            riding_actor_id: None,
        };

        let encoded = to_msgpack(&player).expect("player save should encode");
        let probe: DerivedFieldProbe =
            from_msgpack(&encoded).expect("derived field probe should decode");

        assert_eq!(probe.attack, None);
        assert_eq!(probe.defense, None);
        assert_eq!(probe.melee_skill, None);
        assert_eq!(probe.armor_class, None);
        assert_eq!(probe.equipment_modifiers, None);
    }

    #[cfg(feature = "bindings")]
    #[test]
    fn generated_bindings_follow_the_serialized_contract() {
        let typescript = generated_typescript();
        assert!(typescript.contains("actorId: string | null"));
        assert!(typescript.contains("commandSeq: number"));
        assert!(typescript.contains("itemIds: Array<string>"));
        assert!(typescript.contains("equipmentModifiers: StatModifiersDto"));
        assert!(typescript.contains("baseAttack: number"));
        assert!(typescript.contains("baseDefense: number"));
        assert!(typescript.contains("attack: number"));
        assert!(typescript.contains("defense: number"));
        assert!(typescript.contains("CheckOutcomeDto"));
        assert!(typescript.contains("{ \"type\": \"check\""));
        assert!(typescript.contains("alerted: boolean"));
        assert!(typescript.contains("equipment: Array<EquipmentItemDto>"));
        assert!(typescript.contains("deviceRecharge?: DeviceRechargeDto | null"));
        assert!(typescript.contains("canReceiveRecharge: boolean"));
        assert!(typescript.contains("requiresTargetGlyph?: boolean"));
        assert!(typescript.contains("requiresRechargeTargets?: boolean"));
        assert!(typescript.contains("{ \"type\": \"recharge-item\""));
        assert!(typescript.contains("{ \"type\": \"use-item-by-glyph\""));
        assert!(typescript.contains("{ \"type\": \"use-item-for-recharge\""));
        assert!(typescript.contains("{ \"type\": \"wait\" }"));
        assert!(
            typescript
                .lines()
                .all(|line| line.trim_end().len() == line.len())
        );

        let schema: serde_json::Value = serde_json::from_str(
            &generated_json_schema().expect("protocol schema should serialize"),
        )
        .expect("generated protocol schema should be valid JSON");
        assert_eq!(schema["title"], "ProtocolSchemaV1");
        assert!(schema["$defs"]["GameCommand"].is_object());
    }
}
