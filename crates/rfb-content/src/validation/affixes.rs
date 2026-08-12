// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use crate::*;

use super::shared::{
    affix_property_bundle_out_of_range, attribute_modifiers_out_of_range,
    equipment_bonuses_out_of_range, insert_definition_id, normalize_tags, require_format_version,
    require_schema, validate_definition_id, validate_definition_text, validate_status_immunities,
};

pub(super) struct AffixValidationOutputs {
    pub(super) affix_ids: BTreeSet<String>,
}

pub(super) fn validate_affixes(
    affixes: &mut [AffixDefinition],
    all_ids: &mut BTreeSet<String>,
) -> Result<AffixValidationOutputs, ContentError> {
    let mut affix_ids = BTreeSet::new();
    for affix in affixes.iter_mut() {
        require_schema(&affix.schema, AFFIX_SCHEMA, &affix.id)?;
        require_format_version(affix.format_version, &affix.id)?;
        validate_definition_id(&affix.id, "affix")?;
        validate_definition_text(&affix.id, &affix.name_key, &affix.description_key)?;
        validate_status_immunities(&affix.id, &mut affix.status_immunities)?;
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
