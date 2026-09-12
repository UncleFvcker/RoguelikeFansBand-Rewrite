// SPDX-License-Identifier: MPL-2.0
// Level curves and spell/device power calculations.

use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilityLevelScalingCurveDefinition,
    AbilityLevelScalingDefinition, AbilityLevelScalingField, AbilitySpellPowerDefinition,
    AbilitySpellPowerField,
};

pub(super) fn scaled_ability_level_value(
    base: u64,
    scaling: &AbilityLevelScalingDefinition,
    level: u16,
) -> u64 {
    let addition = match scaling.curve {
        AbilityLevelScalingCurveDefinition::Linear => {
            u64::from(level.saturating_sub(scaling.level_offset))
                .saturating_mul(u64::from(scaling.multiplier))
                / u64::from(scaling.divisor)
        }
        AbilityLevelScalingCurveDefinition::Prorated => prorated_level_value(
            u64::from(scaling.multiplier),
            level,
            scaling.linear_weight,
            scaling.quadratic_weight,
            scaling.cubic_weight,
        ),
    };
    let scaled = base.saturating_add(addition);
    scaling
        .maximum
        .map_or(scaled, |maximum| scaled.min(maximum))
}

pub(super) fn prorated_level_value(
    amount: u64,
    level: u16,
    linear_weight: u16,
    quadratic_weight: u16,
    cubic_weight: u16,
) -> u64 {
    let level = u64::from(level.min(50));
    if level == 50 {
        return amount;
    }
    let linear_weight = u64::from(linear_weight);
    let quadratic_weight = u64::from(quadratic_weight);
    let cubic_weight = u64::from(cubic_weight);
    let total_weight = linear_weight + quadratic_weight + cubic_weight;
    amount * level * linear_weight / (50 * total_weight)
        + amount * level * level * quadratic_weight / (50 * 50 * total_weight)
        + (amount * level * level / 50) * level * cubic_weight / (50 * 50 * total_weight)
}

pub(super) fn apply_ability_level_scaling(
    effect: &mut AbilityEffectDefinition,
    scaling: &AbilityLevelScalingDefinition,
    level: u16,
) {
    match (effect, scaling.field) {
        (
            AbilityEffectDefinition::Damage { damage_dice, .. }
            | AbilityEffectDefinition::Malediction { damage_dice, .. }
            | AbilityEffectDefinition::AreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::LavaFlow { damage_dice, .. }
            | AbilityEffectDefinition::BeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. }
            | AbilityEffectDefinition::Stardust { damage_dice, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_dice, .. }
            | AbilityEffectDefinition::ConeDamage { damage_dice, .. }
            | AbilityEffectDefinition::CurseDamage { damage_dice, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_dice, .. }
            | AbilityEffectDefinition::DrainLife { damage_dice, .. },
            AbilityLevelScalingField::DamageDice,
        ) => {
            *damage_dice = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_dice),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage dice must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::Malediction { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::LavaFlow { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::LightArea { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::CurseDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilityLevelScalingField::DamageSides,
        ) => {
            *damage_sides = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_sides),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage sides must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::Malediction { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::LavaFlow { damage_bonus, .. }
            | AbilityEffectDefinition::InsanityCircle { damage_bonus, .. }
            | AbilityEffectDefinition::Hellfire { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
            | AbilityEffectDefinition::CurseDamage { damage_bonus, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. },
            AbilityLevelScalingField::DamageBonus,
        ) => {
            *damage_bonus = u16::try_from(scaled_ability_level_value(
                u64::from(*damage_bonus),
                scaling,
                level,
            ))
            .expect("validated level-scaled damage bonus must fit u16");
        }
        (AbilityEffectDefinition::DeathRay { power }, AbilityLevelScalingField::DeathRayPower) => {
            *power = u32::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled death ray power must fit u32");
        }
        (
            AbilityEffectDefinition::TeleportAway { power, .. },
            AbilityLevelScalingField::TeleportAwayPower,
        )
        | (
            AbilityEffectDefinition::RechargeFromPlayer { power },
            AbilityLevelScalingField::RechargePower,
        ) => {
            *power = u16::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled effect power must fit u16");
        }
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilityLevelScalingField::IdentifyPower,
        ) => {
            *full_identify_power = u16::try_from(scaled_ability_level_value(
                u64::from(*full_identify_power),
                scaling,
                level,
            ))
            .expect("validated level-scaled identify power must fit u16");
        }
        (
            AbilityEffectDefinition::BoltOrBeamDamage {
                beam_chance_percent,
                ..
            },
            AbilityLevelScalingField::BeamChancePercent,
        ) => {
            *beam_chance_percent = u8::try_from(scaled_ability_level_value(
                u64::from(*beam_chance_percent),
                scaling,
                level,
            ))
            .expect("validated level-scaled beam chance must fit u8");
        }
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::LavaFlow { radius, .. }
            | AbilityEffectDefinition::InsanityCircle { radius, .. }
            | AbilityEffectDefinition::Hellfire { radius, .. }
            | AbilityEffectDefinition::LightArea { radius, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::BreathDamage { radius, .. }
            | AbilityEffectDefinition::Detect { radius, .. }
            | AbilityEffectDefinition::BlinkSelf { radius, .. },
            AbilityLevelScalingField::Radius,
        ) => {
            *radius = u8::try_from(scaled_ability_level_value(
                u64::from(*radius),
                scaling,
                level,
            ))
            .expect("validated level-scaled radius must fit u8");
        }
        (
            AbilityEffectDefinition::DimensionDoor { range }
            | AbilityEffectDefinition::Jump { range },
            AbilityLevelScalingField::Radius,
        ) => {
            *range = u16::try_from(scaled_ability_level_value(
                u64::from(*range),
                scaling,
                level,
            ))
            .expect("validated level-scaled dimension door range must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { intensity, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { intensity, .. },
            AbilityLevelScalingField::StatusIntensity,
        ) => {
            *intensity = u16::try_from(scaled_ability_level_value(
                u64::from(*intensity),
                scaling,
                level,
            ))
            .expect("validated level-scaled status intensity must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::SustainAttributes { duration_ticks },
            AbilityLevelScalingField::StatusDurationTicks,
        ) => {
            *duration_ticks = u32::try_from(scaled_ability_level_value(
                u64::from(*duration_ticks),
                scaling,
                level,
            ))
            .expect("validated level-scaled status duration must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_sides, .. },
            AbilityLevelScalingField::StatusDurationSides,
        ) => {
            *duration_sides = u32::try_from(scaled_ability_level_value(
                u64::from(*duration_sides),
                scaling,
                level,
            ))
            .expect("validated level-scaled status duration sides must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_modifiers, ..
            },
            AbilityLevelScalingField::StatusDefense,
        ) => {
            granted_modifiers.defense = i32::try_from(scaled_ability_level_value(
                u64::try_from(granted_modifiers.defense)
                    .expect("validated status defense must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated level-scaled status defense must fit i32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::Sanctuary { power, .. }
            | AbilityEffectDefinition::Entangle { power, .. }
            | AbilityEffectDefinition::TurnUndead { power },
            AbilityLevelScalingField::StatusPower,
        )
        | (
            AbilityEffectDefinition::Control { power, .. }
            | AbilityEffectDefinition::Domination { power, .. }
            | AbilityEffectDefinition::InsanityCircle {
                control_power: power,
                ..
            },
            AbilityLevelScalingField::ControlPower,
        )
        | (
            AbilityEffectDefinition::Genocide { power, .. },
            AbilityLevelScalingField::GenocidePower,
        ) => {
            *power = u16::try_from(scaled_ability_level_value(
                u64::from(*power),
                scaling,
                level,
            ))
            .expect("validated level-scaled effect power must fit u16");
        }
        (
            AbilityEffectDefinition::SummonCategory { maximum_level, .. },
            AbilityLevelScalingField::SummonMaximumLevel,
        ) => {
            *maximum_level = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_level),
                scaling,
                level,
            ))
            .expect("validated level-scaled summon maximum level must fit u16");
        }
        (
            AbilityEffectDefinition::FetchItem {
                maximum_weight_tenths_pound,
            },
            AbilityLevelScalingField::MaximumWeight,
        ) => {
            *maximum_weight_tenths_pound = u32::try_from(scaled_ability_level_value(
                u64::from(*maximum_weight_tenths_pound),
                scaling,
                level,
            ))
            .expect("validated level-scaled fetch weight must fit u32");
        }
        (
            AbilityEffectDefinition::Banish { maximum_distance },
            AbilityLevelScalingField::BanishDistance,
        ) => {
            *maximum_distance = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_distance),
                scaling,
                level,
            ))
            .expect("validated level-scaled banish distance must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery { duration_base, .. },
            AbilityLevelScalingField::DeviceMasteryDurationBase,
        ) => {
            *duration_base = u16::try_from(scaled_ability_level_value(
                u64::from(*duration_base),
                scaling,
                level,
            ))
            .expect("validated device mastery duration must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery {
                device_power_bonus, ..
            },
            AbilityLevelScalingField::DevicePowerBonus,
        ) => {
            *device_power_bonus = i32::try_from(scaled_ability_level_value(
                u64::try_from(*device_power_bonus)
                    .expect("validated device power bonus must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated device power bonus must fit i32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                granted_equipment_bonuses,
                ..
            },
            AbilityLevelScalingField::StatusMeleeDamage,
        ) => {
            granted_equipment_bonuses.melee_damage = i32::try_from(scaled_ability_level_value(
                u64::try_from(granted_equipment_bonuses.melee_damage)
                    .expect("validated status melee damage must be non-negative"),
                scaling,
                level,
            ))
            .expect("validated level-scaled status melee damage must fit i32");
        }
        (
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            },
            AbilityLevelScalingField::MaximumRange,
        ) => {
            *maximum_range = u16::try_from(scaled_ability_level_value(
                u64::from(*maximum_range),
                scaling,
                level,
            ))
            .expect("validated level-scaled beam range must fit u16");
        }
        _ => unreachable!("content validation must reject incompatible level scaling fields"),
    }
}

pub(super) fn spell_power_value(value: u64, bonus: i32) -> u64 {
    let value = i128::from(value);
    let adjusted = value + value * i128::from(bonus) / 13;
    u64::try_from(adjusted.max(0)).expect("non-negative spell power must fit u64")
}

pub(super) fn device_power_value(value: u64, bonus: i32) -> u64 {
    let value = i128::from(value);
    let adjusted = value + value * i128::from(bonus) / 20;
    u64::try_from(adjusted.max(0)).expect("non-negative device power must fit u64")
}

pub(super) fn apply_ability_spell_power(
    effect: &mut AbilityEffectDefinition,
    definition: AbilitySpellPowerDefinition,
    bonus: i32,
) {
    let scaled = |value| spell_power_value(value, bonus);
    match (effect, definition.field) {
        (
            AbilityEffectDefinition::Stardust { damage_dice, .. },
            AbilitySpellPowerField::DamageDice,
        ) => {
            *damage_dice = u16::try_from(scaled(u64::from(*damage_dice)))
                .expect("spell-powered damage dice must fit u16");
        }
        (
            AbilityEffectDefinition::Damage { damage_sides, .. }
            | AbilityEffectDefinition::Malediction { damage_sides, .. }
            | AbilityEffectDefinition::AreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::LavaFlow { damage_sides, .. }
            | AbilityEffectDefinition::BeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::LightArea { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_sides, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_sides, .. }
            | AbilityEffectDefinition::ConeDamage { damage_sides, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_sides, .. }
            | AbilityEffectDefinition::DrainLife { damage_sides, .. },
            AbilitySpellPowerField::DamageSides,
        ) => {
            *damage_sides = u16::try_from(scaled(u64::from(*damage_sides)))
                .expect("spell-powered damage sides must fit u16");
        }
        (AbilityEffectDefinition::HealDice { sides, .. }, AbilitySpellPowerField::HealingSides) => {
            *sides = u16::try_from(scaled(u64::from(*sides)))
                .expect("spell-powered healing sides must fit u16");
        }
        (AbilityEffectDefinition::Heal { amount }, AbilitySpellPowerField::HealingAmount) => {
            *amount = u32::try_from(scaled(u64::from(*amount)))
                .expect("spell-powered healing amount must fit u32");
        }
        (
            AbilityEffectDefinition::Damage { damage_bonus, .. }
            | AbilityEffectDefinition::Malediction { damage_bonus, .. }
            | AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::LavaFlow { damage_bonus, .. }
            | AbilityEffectDefinition::InsanityCircle { damage_bonus, .. }
            | AbilityEffectDefinition::Hellfire { damage_bonus, .. }
            | AbilityEffectDefinition::BeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { damage_bonus, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::ConeDamage { damage_bonus, .. }
            | AbilityEffectDefinition::VisibleDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. },
            AbilitySpellPowerField::DamageBonus,
        ) => {
            *damage_bonus = u16::try_from(scaled(u64::from(*damage_bonus)))
                .expect("spell-powered damage bonus must fit u16");
        }
        (
            AbilityEffectDefinition::AreaDamage { radius, .. }
            | AbilityEffectDefinition::LavaFlow { radius, .. }
            | AbilityEffectDefinition::InsanityCircle { radius, .. }
            | AbilityEffectDefinition::Hellfire { radius, .. }
            | AbilityEffectDefinition::LightArea { radius, .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { radius, .. }
            | AbilityEffectDefinition::ConeDamage { radius, .. }
            | AbilityEffectDefinition::Earthquake { radius, .. },
            AbilitySpellPowerField::Radius,
        ) => {
            *radius =
                u8::try_from(scaled(u64::from(*radius))).expect("spell-powered radius must fit u8");
        }
        (AbilityEffectDefinition::DimensionDoor { range }, AbilitySpellPowerField::Radius) => {
            *range = u16::try_from(scaled(u64::from(*range)))
                .expect("spell-powered dimension door range must fit u16");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_ticks, .. }
            | AbilityEffectDefinition::SustainAttributes { duration_ticks },
            AbilitySpellPowerField::StatusDurationTicks,
        ) => {
            *duration_ticks = u32::try_from(scaled(u64::from(*duration_ticks)))
                .expect("spell-powered status duration must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus { duration_sides, .. }
            | AbilityEffectDefinition::VisibleApplyStatus { duration_sides, .. },
            AbilitySpellPowerField::StatusDurationSides,
        ) => {
            *duration_sides = u32::try_from(scaled(u64::from(*duration_sides)))
                .expect("spell-powered status duration sides must fit u32");
        }
        (
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::VisibleApplyStatus {
                power: Some(power), ..
            }
            | AbilityEffectDefinition::Entangle { power, .. },
            AbilitySpellPowerField::StatusPower,
        )
        | (
            AbilityEffectDefinition::Control { power, .. }
            | AbilityEffectDefinition::Domination { power, .. },
            AbilitySpellPowerField::ControlPower,
        )
        | (
            AbilityEffectDefinition::InsanityCircle {
                control_power: power,
                ..
            },
            AbilitySpellPowerField::ControlPower,
        )
        | (
            AbilityEffectDefinition::Genocide { power, .. },
            AbilitySpellPowerField::GenocidePower,
        ) => {
            *power = u16::try_from(scaled(u64::from(*power)))
                .expect("spell-powered effect power must fit u16");
        }
        (
            AbilityEffectDefinition::SummonCategory { maximum_level, .. },
            AbilitySpellPowerField::SummonMaximumLevel,
        ) => {
            *maximum_level = u16::try_from(scaled(u64::from(*maximum_level)))
                .expect("spell-powered summon maximum level must fit u16");
        }
        (
            AbilityEffectDefinition::IdentifyItem {
                full_identify_power,
                ..
            },
            AbilitySpellPowerField::IdentifyPower,
        ) => {
            *full_identify_power = u16::try_from(scaled(u64::from(*full_identify_power)))
                .expect("spell-powered identify power must fit u16");
        }
        (
            AbilityEffectDefinition::TeleportAway { power, .. },
            AbilitySpellPowerField::TeleportAwayPower,
        )
        | (
            AbilityEffectDefinition::RechargeFromPlayer { power },
            AbilitySpellPowerField::RechargePower,
        ) => {
            *power = u16::try_from(scaled(u64::from(*power)))
                .expect("spell-powered effect power must fit u16");
        }
        (
            AbilityEffectDefinition::FetchItem {
                maximum_weight_tenths_pound,
            },
            AbilitySpellPowerField::MaximumWeight,
        ) => {
            *maximum_weight_tenths_pound =
                u32::try_from(scaled(u64::from(*maximum_weight_tenths_pound)))
                    .expect("spell-powered fetch weight must fit u32");
        }
        (
            AbilityEffectDefinition::Banish { maximum_distance },
            AbilitySpellPowerField::BanishDistance,
        ) => {
            *maximum_distance = u16::try_from(scaled(u64::from(*maximum_distance)))
                .expect("spell-powered banish distance must fit u16");
        }
        (
            AbilityEffectDefinition::DeviceMastery { duration_base, .. },
            AbilitySpellPowerField::DeviceMasteryDurationBase,
        ) => {
            *duration_base = u16::try_from(scaled(u64::from(*duration_base)))
                .expect("spell-powered device mastery duration must fit u16");
        }
        (
            AbilityEffectDefinition::Clairvoyance {
                telepathy_duration_sides,
                ..
            },
            AbilitySpellPowerField::ClairvoyanceDurationSides,
        ) => {
            *telepathy_duration_sides = u16::try_from(scaled(u64::from(*telepathy_duration_sides)))
                .expect("spell-powered clairvoyance duration must fit u16");
        }
        (
            AbilityEffectDefinition::BeamDamage {
                maximum_range: Some(maximum_range),
                ..
            },
            AbilitySpellPowerField::MaximumRange,
        ) => {
            *maximum_range = u16::try_from(scaled(u64::from(*maximum_range)))
                .expect("spell-powered beam range must fit u16");
        }
        (
            _,
            AbilitySpellPowerField::FinalDamage
            | AbilitySpellPowerField::FinalHealing
            | AbilitySpellPowerField::RandomChoiceRoll
            | AbilitySpellPowerField::MaledictionDeathRayPower
            | AbilitySpellPowerField::MaledictionFearPower
            | AbilitySpellPowerField::InvulnerabilityDuration,
        ) => {}
        _ => unreachable!("content validation must reject incompatible spell power fields"),
    }
}

pub(super) fn ability_has_spell_power_field(
    ability: &AbilityDefinition,
    effect_index: u8,
    field: AbilitySpellPowerField,
) -> bool {
    ability
        .spell_power_fields
        .iter()
        .any(|definition| definition.effect_index == effect_index && definition.field == field)
}

pub(super) fn spell_powered_ability_value(
    ability: &AbilityDefinition,
    effect_index: u8,
    field: AbilitySpellPowerField,
    value: u64,
) -> u64 {
    if ability_has_spell_power_field(ability, effect_index, field) {
        spell_power_value(value, ability.spell_power_bonus)
    } else {
        value
    }
}
