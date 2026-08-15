// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use crate::*;

use super::shared::{
    affix_property_bundle_out_of_range, attribute_modifiers_out_of_range,
    equipment_bonuses_out_of_range, insert_definition_id, normalize_tags, require_format_version,
    require_schema, validate_definition_id, validate_definition_text, validate_id,
    validate_message_key, validate_status_immunities,
};

pub(super) struct AffixValidationOutputs {
    pub(super) affix_ids: BTreeSet<String>,
}

pub(super) fn validate_affixes(
    affixes: &mut [AffixDefinition],
    all_ids: &mut BTreeSet<String>,
) -> Result<AffixValidationOutputs, ContentError> {
    let mut affix_ids = BTreeSet::new();
    let mut rfb_ego_source_indices = BTreeSet::new();
    for affix in affixes.iter_mut() {
        require_schema(&affix.schema, AFFIX_SCHEMA, &affix.id)?;
        require_format_version(affix.format_version, &affix.id)?;
        validate_definition_id(&affix.id, "affix")?;
        validate_definition_text(&affix.id, &affix.name_key, &affix.description_key)?;
        validate_status_immunities(&affix.id, &mut affix.status_immunities)?;
        if let Some(generation) = &affix.rfb_ego {
            let unique_types = generation.types.iter().copied().collect::<BTreeSet<_>>();
            if generation.source_index == 0
                || generation.types.is_empty()
                || unique_types.len() != generation.types.len()
                || !rfb_ego_source_indices.insert(generation.source_index)
            {
                return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
            }
        }
        if affix
            .device_generation
            .as_ref()
            .is_some_and(|generation| !valid_affix_device_generation(generation))
        {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        let mut roll_substance = false;
        if affix.roll_groups.len() > 16 {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        for group in &mut affix.roll_groups {
            if group.rolls == 0
                || group.rolls > 16
                || group.candidates.is_empty()
                || group.candidates.len() > 64
            {
                return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
            }
            let mut group_substance = false;
            for candidate in &mut group.candidates {
                validate_status_immunities(&affix.id, &mut candidate.properties.status_immunities)?;
                if candidate.weight == 0
                    || candidate.weight > 1_000_000
                    || candidate.min_depth > candidate.max_depth
                    || affix_property_bundle_out_of_range(&candidate.properties)
                {
                    return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
                }
                group_substance |= candidate.properties != AffixPropertyBundleDefinition::default();
            }
            if !group_substance {
                return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
            }
            roll_substance = true;
        }
        let modifiers = &affix.modifiers;
        let has_substance = modifiers != &StatModifiers::default()
            || affix.equipment_bonuses != EquipmentBonuses::default()
            || !affix.resistances.is_empty()
            || !affix.status_immunities.is_empty()
            || !affix.slays.is_empty()
            || !affix.brands.is_empty()
            || !affix.passives.is_empty()
            || !affix.elemental_destruction_vulnerabilities.is_empty()
            || !affix.elemental_destruction_immunities.is_empty()
            || affix.resists_projection_destruction
            || affix.resists_monster_destruction
            || affix.protects_quiver_ammunition
            || affix.device_generation.is_some()
            || roll_substance;
        if !has_substance
            || affix.generation_level > affix.generation_max_level
            || modifiers.max_hp < -1_000_000
            || modifiers.max_hp > 1_000_000
            || modifiers.attack < -1_000_000
            || modifiers.attack > 1_000_000
            || modifiers.defense < -1_000_000
            || modifiers.defense > 1_000_000
            || !(-100..=100).contains(&modifiers.speed)
            || attribute_modifiers_out_of_range(modifiers)
            || equipment_bonuses_out_of_range(&affix.equipment_bonuses)
        {
            return Err(ContentError::InvalidAffixModifiers(affix.id.clone()));
        }
        normalize_tags(&affix.id, &mut affix.tags)?;
        insert_definition_id(all_ids, &affix.id)?;
        affix_ids.insert(affix.id.clone());
    }
    Ok(AffixValidationOutputs { affix_ids })
}

fn valid_affix_device_generation(generation: &ItemDeviceGenerationDefinition) -> bool {
    if generation.activations.is_empty() || generation.activations.len() > 256 {
        return false;
    }
    let valid_recovery = |recovery: ItemDeviceRecoveryDefinition| {
        (1..=10_000).contains(&recovery.interval_ticks)
            && (1..=1_000).contains(&recovery.energy_per_mille)
    };
    let mut ids = BTreeSet::new();
    generation.recovery.is_none_or(valid_recovery)
        && generation.activations.iter().all(|activation| {
            let mut modes = BTreeSet::new();
            let modes_are_unique = activation
                .target
                .modes
                .iter()
                .all(|mode| modes.insert(*mode))
                && !activation.target.modes.is_empty();
            let self_target = activation.target.modes.as_slice()
                == [AbilityTargetModeDefinition::SelfTarget]
                && activation.target.range == 0
                && !activation.target.requires_line_of_effect;
            let projectile_target = !activation
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::SelfTarget)
                && activation.target.modes.iter().all(|mode| {
                    matches!(
                        mode,
                        AbilityTargetModeDefinition::Direction
                            | AbilityTargetModeDefinition::Position
                            | AbilityTargetModeDefinition::Entity
                    )
                })
                && (1..=64).contains(&activation.target.range)
                && activation.target.requires_line_of_effect;
            let valid_effect_target = match activation.effect {
                ItemUseEffectDefinition::ApplyBerserkStrength {
                    duration_dice: 1..=100,
                    duration_sides: 1..=1_000_000,
                    duration_bonus: 0..=1_000_000,
                }
                | ItemUseEffectDefinition::AreaDestruction { .. }
                | ItemUseEffectDefinition::RandomTeleport { .. } => self_target,
                ItemUseEffectDefinition::TerrainBeam { .. } => projectile_target,
                ItemUseEffectDefinition::RidingCharge => {
                    activation.target.modes.as_slice()
                        == [
                            AbilityTargetModeDefinition::Direction,
                            AbilityTargetModeDefinition::Entity,
                        ]
                        && activation.target.range == 7
                        && activation.target.requires_line_of_effect
                }
                _ => {
                    let item_target = activation.target.modes.as_slice()
                        == [AbilityTargetModeDefinition::Item]
                        && activation.target.range == 0
                        && !activation.target.requires_line_of_effect;
                    self_target || projectile_target || item_target
                }
            };
            validate_id(&activation.id).is_ok()
                && ids.insert(activation.id.clone())
                && validate_message_key(&activation.name_key).is_ok()
                && activation.effect != ItemUseEffectDefinition::NoNumericEffect
                && (1..=1_000_000).contains(&activation.weight)
                && activation.min_depth <= activation.max_depth
                && (1..=1_000_000).contains(&activation.device_check_difficulty)
                && (1..=1_000_000).contains(&activation.charges.minimum)
                && activation.charges.minimum <= activation.charges.maximum
                && activation.charges.maximum <= 1_000_000
                && (1..=activation.charges.minimum).contains(&activation.charges.cost)
                && activation.recovery.is_none_or(valid_recovery)
                && modes_are_unique
                && valid_effect_target
        })
        && (1..=100).all(|depth| {
            generation
                .activations
                .iter()
                .any(|activation| activation.min_depth <= depth && depth <= activation.max_depth)
        })
}
