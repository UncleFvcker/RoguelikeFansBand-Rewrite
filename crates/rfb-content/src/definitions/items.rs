// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    AbilityDetectSubjectDefinition, AbilityStatusStackingDefinition, AbilityTargetDefinition,
    ActorDamageType, ActorResistanceLevel, EquipmentBonuses, ItemAttributeDefinition,
    StatModifiers,
};

const fn default_incoming_damage_percent() -> u8 {
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
    SeeInvisible,
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
    /// Protects instances from `KILL_ITEM`; used by Endurance ammunition.
    #[serde(default)]
    pub resists_monster_destruction: bool,
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
    /// Original launcher multiplier scaled by 100 (x2.50 = 250).
    pub damage_multiplier_percent: u16,
    pub to_hit: i32,
    pub to_damage: i32,
    pub ammunition_type: AmmunitionTypeDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum AmmunitionTypeDefinition {
    Shot,
    Arrow,
    Bolt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AmmunitionProfileDefinition {
    pub ammunition_type: AmmunitionTypeDefinition,
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
    NoNumericEffect,
    IncreaseNutrition {
        amount: u16,
    },
    SatisfyHunger,
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
    ApplyStatus {
        status_kind_id: String,
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
        stacking: AbilityStatusStackingDefinition,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        resistance_type: Option<ActorDamageType>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        granted_resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
        #[serde(default)]
        granted_modifiers: StatModifiers,
        #[serde(default)]
        granted_equipment_bonuses: EquipmentBonuses,
        #[serde(default = "default_incoming_damage_percent")]
        incoming_damage_percent: u8,
    },
    ApplyGiantStrength {
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
    SelfDamage {
        damage_dice: u16,
        damage_sides: u16,
        #[serde(default)]
        damage_bonus: u16,
    },
    LoseExperienceFraction {
        divisor: u8,
    },
    GainRelativeExperience {
        divisor: u8,
        bonus: u64,
        maximum_gain: u64,
    },
    ApplyTsuyoshi {
        duration_dice: u16,
        duration_sides: u32,
        duration_bonus: u32,
    },
    TriggerTsuyoshiCrash,
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
    CreateCurrentTerrain {
        source_terrain_ids: Vec<String>,
        target_terrain_id: String,
    },
    SetFloorGlow {
        glow: bool,
        radius: u8,
        #[serde(default)]
        connected_glow: bool,
    },
    AreaDestruction {
        minimum_radius: u8,
        maximum_radius: u8,
        floor_terrain_id: String,
        wall_terrain_id: String,
        quartz_terrain_id: String,
        magma_terrain_id: String,
    },
    DestroyAdjacentTrapsAndDoors,
    RemoveStatus {
        status_kind_id: String,
    },
    ReduceStatus {
        status_kind_id: String,
        minimum_reduction: u32,
        reduction_divisor: u8,
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
    DrainResourceFull {
        resource_id: String,
    },
    IdentifyItem {
        #[serde(default)]
        full: bool,
    },
    IdentifyInventory,
    SelfKnowledge,
    Acquirement {
        loot_table_id: String,
        minimum_count: u8,
        maximum_count: u8,
    },
    MundanifyItem,
    CraftItem {
        weapon_affix_ids: Vec<String>,
        armor_affix_ids: Vec<String>,
    },
    ShowRumour {
        message_key: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "kebab-case")]
pub enum ItemFuelKindDefinition {
    Torch,
    Lantern,
    Oil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemFuelDefinition {
    pub kind: ItemFuelKindDefinition,
    pub initial: u16,
    pub maximum: u16,
    #[serde(default)]
    pub light_radius: u8,
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
    /// Fully-known, plain-instance value used by authoritative shop pricing.
    /// Zero means ordinary stores will not buy the item.
    #[serde(default)]
    pub base_value: u32,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    /// Extra shared-pack stack slots granted while this container is equipped.
    #[serde(default)]
    pub inventory_slot_bonus: u16,
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
    pub ammunition_profile: Option<AmmunitionProfileDefinition>,
    #[serde(default)]
    pub throw_profile: Option<ThrowProfileDefinition>,
    #[serde(default)]
    pub use_action: Option<ItemUseActionDefinition>,
    #[serde(default)]
    pub device_generation: Option<ItemDeviceGenerationDefinition>,
    #[serde(default)]
    pub fuel: Option<ItemFuelDefinition>,
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
    /// Protects the base kind from `KILL_ITEM`; fixed artifacts use their artifact tag.
    #[serde(default)]
    pub resists_monster_destruction: bool,
    pub tags: Vec<String>,
}
