// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "schemas")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    AbilityTargetDefinition, ActorDamageType, ActorResistanceLevel, AmmunitionProfileDefinition,
    ArtifactGenerationDefinition, AttackProfileDefinition, ContentError, EquipmentBonuses,
    EquipmentPassive, ItemChargeDefinition, ItemCurseSeverityDefinition, ItemDefinition,
    ItemDeviceActivationDefinition, ItemDeviceChargeRangeDefinition,
    ItemDeviceGenerationDefinition, ItemDeviceRecoveryDefinition, ItemFuelDefinition,
    ItemShatterEffectDefinition, ItemUseActionDefinition, ProjectileProfileDefinition, SlayLevel,
    SlayTarget, StatModifiers, ThrowProfileDefinition, WeaponBrand,
    effect_programs::{
        ResolvedEffectProgram, effect_program_input_matches_device_target,
        resolve_source_item_effect,
    },
};
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(feature = "schemas", schemars(title = "ItemDefinition"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceItemDefinition {
    #[serde(rename = "$schema")]
    schema: String,
    format_version: u16,
    id: String,
    name_key: String,
    #[serde(default)]
    appearance_name_key: Option<String>,
    description_key: String,
    glyph: String,
    #[serde(default)]
    generation_level: u16,
    #[serde(default)]
    mogaminator_rare: bool,
    weight_tenths_pound: u16,
    #[serde(default)]
    tunneling_pval: i16,
    max_stack: u32,
    #[serde(default)]
    base_value: u32,
    #[serde(default)]
    equipment_slot: Option<String>,
    #[serde(default)]
    weapon_proficiency_base_item_id: Option<String>,
    #[serde(default)]
    artifact_generation: Option<ArtifactGenerationDefinition>,
    #[serde(default)]
    inventory_slot_bonus: u16,
    #[serde(default)]
    ammunition_capacity: u16,
    /// Curse stamped onto newly generated instances. Save data remains
    /// authoritative after generation and never re-derives this field.
    #[serde(default)]
    initial_curse: Option<ItemCurseSeverityDefinition>,
    #[serde(default)]
    modifiers: StatModifiers,
    #[serde(default)]
    equipment_bonuses: EquipmentBonuses,
    #[serde(default)]
    melee_profile: Option<AttackProfileDefinition>,
    #[serde(default)]
    projectile_profile: Option<ProjectileProfileDefinition>,
    #[serde(default)]
    ammunition_profile: Option<AmmunitionProfileDefinition>,
    #[serde(default)]
    throw_profile: Option<ThrowProfileDefinition>,
    #[serde(default)]
    use_action: Option<SourceItemUseActionDefinition>,
    #[serde(default)]
    shatter_effect_program_id: Option<String>,
    #[serde(default)]
    shatter_radius: u8,
    #[serde(default)]
    device_generation: Option<SourceItemDeviceGenerationDefinition>,
    #[serde(default)]
    fuel: Option<ItemFuelDefinition>,
    #[serde(default)]
    ability_book_id: Option<String>,
    #[serde(default)]
    break_chance_percent: u8,
    /// Defensive resistance tiers granted while the item is equipped.
    #[serde(default)]
    resistances: BTreeMap<ActorDamageType, ActorResistanceLevel>,
    /// Status kind ids the wearer is immune to while the item is equipped.
    #[serde(default)]
    status_immunities: Vec<String>,
    /// Target categories receiving an original-compatible slay or kill
    /// multiplier from melee weapon dice while this item is equipped.
    #[serde(default)]
    slays: BTreeMap<SlayTarget, SlayLevel>,
    /// Elemental brands applied to melee weapon dice while this item is
    /// equipped.
    #[serde(default)]
    brands: BTreeSet<WeaponBrand>,
    /// Passive capabilities granted while this item is equipped.
    #[serde(default)]
    passives: BTreeSet<EquipmentPassive>,
    #[serde(default)]
    elemental_destruction_vulnerabilities: BTreeSet<crate::ItemDestructionElement>,
    #[serde(default)]
    elemental_destruction_immunities: BTreeSet<crate::ItemDestructionElement>,
    #[serde(default)]
    resists_projection_destruction: bool,
    #[serde(default)]
    resists_monster_destruction: bool,
    tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(feature = "schemas", schemars(rename = "ItemUseActionDefinition"))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceItemUseActionDefinition {
    #[serde(default)]
    device_check_difficulty: Option<i32>,
    #[serde(default)]
    charges: Option<ItemChargeDefinition>,
    effect_program_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(
    feature = "schemas",
    schemars(rename = "ItemDeviceActivationDefinition")
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceItemDeviceActivationDefinition {
    id: String,
    name_key: String,
    weight: u32,
    min_depth: u16,
    max_depth: u16,
    device_check_difficulty: i32,
    charges: ItemDeviceChargeRangeDefinition,
    target: AbilityTargetDefinition,
    effect_program_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schemas", derive(JsonSchema))]
#[cfg_attr(
    feature = "schemas",
    schemars(rename = "ItemDeviceGenerationDefinition")
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceItemDeviceGenerationDefinition {
    activations: Vec<SourceItemDeviceActivationDefinition>,
    #[serde(default)]
    recovery: Option<ItemDeviceRecoveryDefinition>,
}

impl SourceItemUseActionDefinition {
    fn into_compiled(
        self,
        owner_id: &str,
        programs: &BTreeMap<String, ResolvedEffectProgram>,
    ) -> Result<ItemUseActionDefinition, ContentError> {
        let (effect, _) = resolve_source_item_effect(owner_id, self.effect_program_id, programs)?;
        Ok(ItemUseActionDefinition {
            device_check_difficulty: self.device_check_difficulty,
            charges: self.charges,
            effect,
        })
    }
}

impl SourceItemDeviceActivationDefinition {
    fn into_compiled(
        self,
        programs: &BTreeMap<String, ResolvedEffectProgram>,
    ) -> Result<ItemDeviceActivationDefinition, ContentError> {
        let (effect, program_input) =
            resolve_source_item_effect(&self.id, self.effect_program_id, programs)?;
        if !effect_program_input_matches_device_target(program_input, &self.target) {
            return Err(ContentError::InvalidItemUseAction(self.id.clone()));
        }
        Ok(ItemDeviceActivationDefinition {
            id: self.id,
            name_key: self.name_key,
            weight: self.weight,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
            device_check_difficulty: self.device_check_difficulty,
            charges: self.charges,
            target: self.target,
            effect,
        })
    }
}

impl SourceItemDeviceGenerationDefinition {
    fn into_compiled(
        self,
        programs: &BTreeMap<String, ResolvedEffectProgram>,
    ) -> Result<ItemDeviceGenerationDefinition, ContentError> {
        Ok(ItemDeviceGenerationDefinition {
            activations: self
                .activations
                .into_iter()
                .map(|activation| activation.into_compiled(programs))
                .collect::<Result<Vec<_>, _>>()?,
            recovery: self.recovery,
        })
    }
}

impl SourceItemDefinition {
    pub(super) fn into_compiled(
        self,
        programs: &BTreeMap<String, ResolvedEffectProgram>,
    ) -> Result<ItemDefinition, ContentError> {
        if self.shatter_effect_program_id.is_none() && self.shatter_radius != 0 {
            return Err(ContentError::InvalidItemUseAction(self.id));
        }
        let use_action = self
            .use_action
            .map(|action| action.into_compiled(&self.id, programs))
            .transpose()?;
        let device_generation = self
            .device_generation
            .map(|generation| generation.into_compiled(programs))
            .transpose()?;
        let shatter_effect = self
            .shatter_effect_program_id
            .map(|program_id| {
                let (effect, input) = resolve_source_item_effect(&self.id, program_id, programs)?;
                if input != crate::EffectProgramInputDefinition::Area {
                    return Err(ContentError::InvalidItemUseAction(self.id.clone()));
                }
                Ok(ItemShatterEffectDefinition {
                    radius: self.shatter_radius,
                    effect,
                })
            })
            .transpose()?;
        Ok(ItemDefinition {
            schema: self.schema,
            format_version: self.format_version,
            id: self.id,
            name_key: self.name_key,
            appearance_name_key: self.appearance_name_key,
            description_key: self.description_key,
            glyph: self.glyph,
            generation_level: self.generation_level,
            mogaminator_rare: self.mogaminator_rare,
            weight_tenths_pound: self.weight_tenths_pound,
            tunneling_pval: self.tunneling_pval,
            max_stack: self.max_stack,
            base_value: self.base_value,
            equipment_slot: self.equipment_slot,
            weapon_proficiency_base_item_id: self.weapon_proficiency_base_item_id,
            artifact_generation: self.artifact_generation,
            inventory_slot_bonus: self.inventory_slot_bonus,
            ammunition_capacity: self.ammunition_capacity,
            initial_curse: self.initial_curse,
            modifiers: self.modifiers,
            equipment_bonuses: self.equipment_bonuses,
            melee_profile: self.melee_profile,
            projectile_profile: self.projectile_profile,
            ammunition_profile: self.ammunition_profile,
            throw_profile: self.throw_profile,
            use_action,
            shatter_effect,
            device_generation,
            fuel: self.fuel,
            ability_book_id: self.ability_book_id,
            break_chance_percent: self.break_chance_percent,
            resistances: self.resistances,
            status_immunities: self.status_immunities,
            slays: self.slays,
            brands: self.brands,
            passives: self.passives,
            elemental_destruction_vulnerabilities: self.elemental_destruction_vulnerabilities,
            elemental_destruction_immunities: self.elemental_destruction_immunities,
            resists_projection_destruction: self.resists_projection_destruction,
            resists_monster_destruction: self.resists_monster_destruction,
            tags: self.tags,
        })
    }
}
