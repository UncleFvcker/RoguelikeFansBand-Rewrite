// SPDX-License-Identifier: MPL-2.0

use super::*;

pub(super) fn ability_status_stacking_dto(
    stacking: AbilityStatusStackingDefinition,
) -> AbilityStatusStackingDto {
    match stacking {
        AbilityStatusStackingDefinition::Replace => AbilityStatusStackingDto::Replace,
        AbilityStatusStackingDefinition::Extend => AbilityStatusStackingDto::Extend,
        AbilityStatusStackingDefinition::KeepStrongest => AbilityStatusStackingDto::KeepStrongest,
    }
}

fn ability_status_stacking(stacking: AbilityStatusStackingDefinition) -> StatusStacking {
    match stacking {
        AbilityStatusStackingDefinition::Replace => StatusStacking::Replace,
        AbilityStatusStackingDefinition::Extend => StatusStacking::Extend,
        AbilityStatusStackingDefinition::KeepStrongest => StatusStacking::KeepStrongest,
    }
}

fn ability_status_change_dto(change: StatusChange) -> AbilityStatusChangeDto {
    match change {
        StatusChange::Added => AbilityStatusChangeDto::Added,
        StatusChange::Replaced => AbilityStatusChangeDto::Replaced,
        StatusChange::Extended => AbilityStatusChangeDto::Extended,
        StatusChange::Strengthened => AbilityStatusChangeDto::Strengthened,
        StatusChange::Unchanged => AbilityStatusChangeDto::Unchanged,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_ability_status_effect(
    actor: &mut Actor,
    ability_id: &str,
    effect_index: u8,
    status_kind_id: &str,
    intensity: u16,
    duration_ticks: u32,
    duration_dice: u16,
    duration_sides: u32,
    stacking: AbilityStatusStackingDefinition,
    resistance_type: Option<rfb_content::ActorDamageType>,
    power: Option<u16>,
    granted_resistances: &BTreeMap<rfb_content::ActorDamageType, rfb_content::ActorResistanceLevel>,
    granted_brands: &BTreeSet<WeaponBrand>,
    granted_modifiers: &StatModifiers,
    granted_equipment_bonuses: &EquipmentBonuses,
    granted_status_immunities: &BTreeSet<String>,
    granted_race_id: Option<&str>,
    grants_wall_passage: bool,
    incoming_damage_percent: u8,
    target_level: Option<u32>,
    defenses: Option<(&ResistanceProfile, &BTreeSet<String>)>,
    rng: &mut RfbRng,
) -> AbilityEffectResolutionDto {
    let requested_duration_ticks = if duration_sides == 0 {
        duration_ticks
    } else {
        (0..duration_dice).fold(duration_ticks, |total, _| {
            total.saturating_add(
                u32::try_from(rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("status duration roll must fit u32"),
            )
        })
    };
    let granted_resistances_dto = granted_resistances
        .iter()
        .map(|(damage_type, level)| ResistanceDto {
            damage_type: DamageType::from(*damage_type).into(),
            level: ResistanceLevel::from(*level).into(),
        })
        .collect::<Vec<_>>();
    let granted_brands_dto = granted_brands
        .iter()
        .copied()
        .map(weapon_brand_dto)
        .collect::<Vec<_>>();
    // Gear- or race-granted immunity blocks the status outright before any
    // resistance scaling; the resolution reuses the immune shape.
    if defenses.is_some_and(|(_, immunities)| immunities.contains(status_kind_id)) {
        return AbilityEffectResolutionDto::ApplyStatus {
            effect_index,
            status_kind_id: status_kind_id.to_owned(),
            intensity,
            requested_duration_ticks,
            applied_duration_ticks: 0,
            stacking: ability_status_stacking_dto(stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            resistance: None,
            power,
            target_level: None,
            power_roll: None,
            target_roll: None,
            granted_resistances: granted_resistances_dto,
            granted_brands: granted_brands_dto,
            granted_race_id: granted_race_id.map(str::to_owned),
            grants_wall_passage,
            incoming_damage_percent,
            change: AbilityStatusChangeDto::Immune,
        };
    }
    let (resolved_target_level, power_roll, target_roll) =
        if let (Some(power), Some(target_level)) = (power, target_level) {
            let target_roll = u32::try_from(rng.bounded(u64::from(target_level.max(1))) + 1)
                .expect("status target-level roll must fit u32");
            let power_roll = u16::try_from(rng.bounded(u64::from(power.max(1))) + 1)
                .expect("status power roll must fit u16");
            (Some(target_level), Some(power_roll), Some(target_roll))
        } else {
            (None, None, None)
        };
    if power_roll
        .zip(target_roll)
        .is_some_and(|(power_roll, target_roll)| target_roll >= u32::from(power_roll))
    {
        return AbilityEffectResolutionDto::ApplyStatus {
            effect_index,
            status_kind_id: status_kind_id.to_owned(),
            intensity,
            requested_duration_ticks,
            applied_duration_ticks: 0,
            stacking: ability_status_stacking_dto(stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            resistance: None,
            power,
            target_level: resolved_target_level,
            power_roll,
            target_roll,
            granted_resistances: granted_resistances_dto,
            granted_brands: granted_brands_dto,
            granted_race_id: granted_race_id.map(str::to_owned),
            grants_wall_passage,
            incoming_damage_percent,
            change: AbilityStatusChangeDto::Resisted,
        };
    }
    let resistance = resistance_type.map(DamageType::from).map(|damage_type| {
        defenses.map_or_else(
            || actor.resistances.level(damage_type),
            |(profile, _)| profile.level(damage_type),
        )
    });
    let applied_duration_ticks = resistance.map_or(requested_duration_ticks, |level| {
        resisted_status_duration(requested_duration_ticks, level)
    });
    if applied_duration_ticks == 0 {
        return AbilityEffectResolutionDto::ApplyStatus {
            effect_index,
            status_kind_id: status_kind_id.to_owned(),
            intensity,
            requested_duration_ticks,
            applied_duration_ticks,
            stacking: ability_status_stacking_dto(stacking),
            resistance_type: resistance_type.map(DamageType::from).map(Into::into),
            resistance: resistance.map(Into::into),
            power,
            target_level: resolved_target_level,
            power_roll,
            target_roll,
            granted_resistances: granted_resistances_dto,
            granted_brands: granted_brands_dto,
            granted_race_id: granted_race_id.map(str::to_owned),
            grants_wall_passage,
            incoming_damage_percent,
            change: AbilityStatusChangeDto::Immune,
        };
    }
    let outcome = apply_status_application(
        &mut actor.statuses,
        StatusApplication {
            status: StatusInstance {
                kind_id: status_kind_id.to_owned(),
                intensity,
                remaining_ticks: applied_duration_ticks,
                source_id: Some(ability_id.to_owned()),
                granted_resistances: granted_resistances
                    .iter()
                    .map(|(damage_type, level)| {
                        (
                            DamageType::from(*damage_type),
                            ResistanceLevel::from(*level),
                        )
                    })
                    .collect(),
                granted_brands: granted_brands.clone(),
                granted_modifiers: stat_modifiers_dto(granted_modifiers),
                granted_equipment_bonuses: equipment_bonuses_dto(granted_equipment_bonuses),
                granted_status_immunities: granted_status_immunities.clone(),
                granted_race_id: granted_race_id.map(str::to_owned),
                grants_wall_passage,
                incoming_damage_percent,
            },
            stacking: ability_status_stacking(stacking),
        },
    );
    AbilityEffectResolutionDto::ApplyStatus {
        effect_index,
        status_kind_id: outcome.kind_id,
        intensity,
        requested_duration_ticks,
        applied_duration_ticks,
        stacking: ability_status_stacking_dto(stacking),
        resistance_type: resistance_type.map(DamageType::from).map(Into::into),
        resistance: resistance.map(Into::into),
        power,
        target_level: resolved_target_level,
        power_roll,
        target_roll,
        granted_resistances: granted_resistances_dto,
        granted_brands: granted_brands_dto,
        granted_race_id: granted_race_id.map(str::to_owned),
        grants_wall_passage,
        incoming_damage_percent,
        change: ability_status_change_dto(outcome.change),
    }
}

pub(super) fn remove_ability_status_effect(
    actor: &mut Actor,
    effect_index: u8,
    status_kind_id: &str,
) -> AbilityEffectResolutionDto {
    let outcome = apply_status_removal(
        &mut actor.statuses,
        StatusRemovalRequest::new(status_kind_id),
    );
    AbilityEffectResolutionDto::RemoveStatus {
        effect_index,
        status_kind_id: outcome.kind_id,
        removed: outcome.removed,
    }
}
