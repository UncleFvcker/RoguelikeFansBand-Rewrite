// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ABILITY_BOOK_SCHEMA, ABILITY_SCHEMA, AbilityBookDefinition, AbilityDefinition,
    AbilityDetectSubjectDefinition, AbilityEffectDefinition, AbilityGenocideScopeDefinition,
    AbilityLevelScalingField, AbilityRandomTargetDefinition, AbilityTargetModeDefinition,
    ActorRole, ContentError, MonsterCastingDefinition, RESOURCE_SCHEMA, ResourceDefinition,
    valid_ability_level_scaling, valid_ability_spell_power,
};

use super::shared::{
    attribute_modifiers_out_of_range, equipment_bonuses_out_of_range, insert_definition_id,
    normalize_tags, require_actor_role, require_format_version, require_reference, require_schema,
    validate_definition_id, validate_definition_text, validate_id,
};

fn effect_can_affect_ground_items(effect: &AbilityEffectDefinition) -> bool {
    match effect {
        AbilityEffectDefinition::Damage { .. }
        | AbilityEffectDefinition::Malediction { .. }
        | AbilityEffectDefinition::AreaDamage { .. }
        | AbilityEffectDefinition::BeamDamage { .. }
        | AbilityEffectDefinition::BoltOrBeamDamage { .. }
        | AbilityEffectDefinition::BoltOrAreaDamage { .. } => true,
        AbilityEffectDefinition::Sequence { effects } => {
            effects.iter().any(effect_can_affect_ground_items)
        }
        AbilityEffectDefinition::RandomChoice { branches, .. } => branches
            .iter()
            .any(|branch| effect_can_affect_ground_items(&branch.effect)),
        _ => false,
    }
}

pub(super) struct AbilityDefinitions<'a> {
    pub(super) resources: &'a mut [ResourceDefinition],
    pub(super) abilities: &'a mut [AbilityDefinition],
    pub(super) ability_books: &'a mut [AbilityBookDefinition],
}

pub(super) struct AbilityValidationRefs<'a> {
    pub(super) actor_tag_values: &'a BTreeSet<String>,
    pub(super) item_tag_values: &'a BTreeSet<String>,
    pub(super) terrain_tags: &'a BTreeMap<String, BTreeSet<String>>,
    pub(super) actor_roles: &'a BTreeMap<String, ActorRole>,
    pub(super) affix_ids: &'a BTreeSet<String>,
    pub(super) terrain_ids: &'a BTreeSet<String>,
    pub(super) actor_monster_casting: Vec<(String, MonsterCastingDefinition)>,
}

pub(super) struct AbilityValidationOutputs {
    pub(super) resource_ids: BTreeSet<String>,
    pub(super) ability_resources: BTreeMap<String, String>,
    pub(super) ability_ids: BTreeSet<String>,
    pub(super) ability_corpse_item_ids: Vec<(String, String)>,
    pub(super) ability_created_item_ids: Vec<(String, String)>,
    pub(super) ability_race_ids: Vec<(String, String)>,
    pub(super) ability_books_by_id: BTreeMap<String, AbilityBookDefinition>,
    pub(super) ability_book_ids: BTreeSet<String>,
}

pub(super) fn validate_abilities(
    definitions: AbilityDefinitions<'_>,
    refs: AbilityValidationRefs<'_>,
    all_ids: &mut BTreeSet<String>,
) -> Result<AbilityValidationOutputs, ContentError> {
    let AbilityValidationRefs {
        actor_tag_values,
        item_tag_values,
        terrain_tags,
        actor_roles,
        affix_ids,
        terrain_ids,
        actor_monster_casting,
    } = refs;
    let mut resource_ids = BTreeSet::new();
    for resource in definitions.resources.iter_mut() {
        require_schema(&resource.schema, RESOURCE_SCHEMA, &resource.id)?;
        require_format_version(resource.format_version, &resource.id)?;
        validate_definition_id(&resource.id, "resource")?;
        validate_definition_text(&resource.id, &resource.name_key, &resource.description_key)?;
        if resource.wait_recovery_amount > 1_000_000 || resource.rest_recovery_amount > 1_000_000 {
            return Err(ContentError::InvalidResource(resource.id.clone()));
        }
        normalize_tags(&resource.id, &mut resource.tags)?;
        insert_definition_id(all_ids, &resource.id)?;
        resource_ids.insert(resource.id.clone());
    }

    let mut ability_resources = BTreeMap::new();
    let mut ability_ids = BTreeSet::new();
    let mut ability_corpse_item_ids = Vec::new();
    let mut ability_created_item_ids = Vec::new();
    let mut ability_race_ids = Vec::new();
    for ability in definitions.abilities.iter_mut() {
        require_schema(&ability.schema, ABILITY_SCHEMA, &ability.id)?;
        require_format_version(ability.format_version, &ability.id)?;
        validate_definition_id(&ability.id, "ability")?;
        validate_definition_text(&ability.id, &ability.name_key, &ability.description_key)?;
        ability.target.modes.sort();
        ability
            .level_scaling
            .sort_by_key(|scaling| (scaling.effect_index, scaling.field));
        ability
            .spell_power_fields
            .sort_by_key(|definition| (definition.effect_index, definition.field));
        if let AbilityEffectDefinition::RandomChoice { branches, .. } = &mut ability.effect {
            for branch in branches {
                branch
                    .level_scaling
                    .sort_by_key(|scaling| (scaling.effect_index, scaling.field));
            }
        }
        let ordered_effects = match &mut ability.effect {
            AbilityEffectDefinition::Sequence { effects } => effects.as_mut_slice(),
            effect => std::slice::from_mut(effect),
        };
        for effect in ordered_effects {
            if let AbilityEffectDefinition::TransformTerrain {
                source_terrain_ids, ..
            } = effect
            {
                source_terrain_ids.sort();
            }
            if let AbilityEffectDefinition::Earthquake {
                wall_terrain_ids, ..
            } = effect
            {
                wall_terrain_ids.sort();
            }
            if let AbilityEffectDefinition::CreateAmmunition {
                source_item_tags,
                source_terrain_tags,
                ..
            } = effect
            {
                normalize_tags(&ability.id, source_item_tags)?;
                normalize_tags(&ability.id, source_terrain_tags)?;
            }
        }
        let mut modes = BTreeSet::new();
        let valid_single_effect = |effect: &AbilityEffectDefinition, effect_index: usize| {
            let has_level_scaling = |field| {
                ability.level_scaling.iter().any(|scaling| {
                    usize::from(scaling.effect_index) == effect_index && scaling.field == field
                })
            };
            match effect {
                AbilityEffectDefinition::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    ..
                }
                | AbilityEffectDefinition::Malediction {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::AreaDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    radius,
                    target_category,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (1..=16).contains(radius)
                        && target_category.as_ref().is_none_or(|category| {
                            !category.is_empty()
                                && category.len() <= 64
                                && category.bytes().all(|byte| {
                                    byte.is_ascii_lowercase()
                                        || byte.is_ascii_digit()
                                        || matches!(byte, b'-' | b'_')
                                })
                                && actor_tag_values.contains(category)
                        })
                }
                AbilityEffectDefinition::JumpDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_multiplier_numerator,
                    damage_multiplier_denominator,
                    radius,
                    blink_radius,
                    ..
                } => {
                    (((1..=100).contains(damage_dice) && (1..=10_000).contains(damage_sides))
                        || (*damage_dice == 0 && *damage_sides == 0 && *damage_bonus > 0))
                        && *damage_bonus <= 10_000
                        && (1..=100).contains(damage_multiplier_numerator)
                        && (1..=100).contains(damage_multiplier_denominator)
                        && (1..=16).contains(radius)
                        && (1..=64).contains(blink_radius)
                }
                AbilityEffectDefinition::BeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::LightLine {
                    damage_dice,
                    damage_sides,
                } => (1..=100).contains(damage_dice) && (1..=10_000).contains(damage_sides),
                AbilityEffectDefinition::LightArea {
                    damage_dice,
                    damage_sides,
                    radius,
                } => {
                    (1..=100).contains(damage_dice)
                        && ((*damage_sides == 0
                            && has_level_scaling(AbilityLevelScalingField::DamageSides))
                            || (1..=10_000).contains(damage_sides))
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::BoltOrBeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    beam_chance_percent,
                    beam_chance_modifier,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && *beam_chance_percent <= 100
                        && (-100..=100).contains(beam_chance_modifier)
                }
                AbilityEffectDefinition::BoltOrAreaDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    area_from_level,
                    radius,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (1..=100).contains(area_from_level)
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::ConeDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    radius,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::BreathDamage {
                    hp_percent,
                    max_damage,
                    radius,
                    ..
                } => {
                    (1..=100).contains(hp_percent)
                        && (1..=10_000).contains(max_damage)
                        && (1..=16).contains(radius)
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_is_current_hp_percent,
                    nonlethal,
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && (!*damage_is_current_hp_percent
                            || u32::from(*damage_dice)
                                .saturating_mul(u32::from(*damage_sides))
                                .saturating_add(u32::from(*damage_bonus))
                                <= 100)
                        && (!*nonlethal || *damage_is_current_hp_percent)
                }
                AbilityEffectDefinition::DeathRay { power } => {
                    (1..=1_000_000).contains(power)
                        || (*power == 0
                            && has_level_scaling(AbilityLevelScalingField::DeathRayPower))
                }
                AbilityEffectDefinition::TeleportAway {
                    minimum_distance,
                    power,
                } => (1..=64).contains(minimum_distance) && *power <= 1_000,
                AbilityEffectDefinition::RechargeFromPlayer { power } => {
                    (1..=1_000).contains(power)
                        || (*power == 0
                            && has_level_scaling(AbilityLevelScalingField::RechargePower))
                }
                AbilityEffectDefinition::Clairvoyance {
                    telepathy_duration_ticks,
                    telepathy_duration_dice,
                    telepathy_duration_sides,
                } => {
                    *telepathy_duration_ticks <= 10_000
                        && (1..=100).contains(telepathy_duration_dice)
                        && (1..=10_000).contains(telepathy_duration_sides)
                }
                AbilityEffectDefinition::BirdDrop => true,
                AbilityEffectDefinition::DrainResource { amount } => {
                    (1..=1_000_000).contains(amount)
                }
                AbilityEffectDefinition::Amnesia
                | AbilityEffectDefinition::DarkenRoom
                | AbilityEffectDefinition::AggravateMonsters
                | AbilityEffectDefinition::SwapPosition => true,
                AbilityEffectDefinition::Teleport => true,
                AbilityEffectDefinition::BlinkSelf { radius } => {
                    (1..=255).contains(radius)
                        && (*radius <= 10 || has_level_scaling(AbilityLevelScalingField::Radius))
                }
                AbilityEffectDefinition::BlinkTarget { radius } => (1..=10).contains(radius),
                AbilityEffectDefinition::TeleportSelf { minimum_distance } => {
                    (1..=64).contains(minimum_distance)
                }
                AbilityEffectDefinition::TeleportTarget
                | AbilityEffectDefinition::TeleportLevel => true,
                AbilityEffectDefinition::FetchItem {
                    maximum_weight_tenths_pound,
                } => {
                    (1..=1_000_000).contains(maximum_weight_tenths_pound)
                        || (*maximum_weight_tenths_pound == 0
                            && has_level_scaling(AbilityLevelScalingField::MaximumWeight))
                }
                AbilityEffectDefinition::ConsumeTerrain { nutrition } => {
                    (1..=65_535).contains(nutrition)
                }
                AbilityEffectDefinition::CreateAmmunition {
                    item_kind_ids,
                    quantity_minimum,
                    quantity_maximum,
                    source_item_tags,
                    source_terrain_tags,
                } => {
                    let mut item_ids = BTreeSet::new();
                    (1..=4).contains(&item_kind_ids.len())
                        && item_kind_ids
                            .iter()
                            .all(|id| validate_id(id).is_ok() && item_ids.insert(id.as_str()))
                        && (1..=99).contains(quantity_minimum)
                        && quantity_minimum <= quantity_maximum
                        && *quantity_maximum <= 99
                        && (source_item_tags.is_empty() != source_terrain_tags.is_empty())
                        && source_item_tags
                            .iter()
                            .all(|tag| item_tag_values.contains(tag))
                        && source_terrain_tags
                            .iter()
                            .all(|tag| terrain_tags.values().any(|terrain| terrain.contains(tag)))
                }
                AbilityEffectDefinition::TransmuteItemToGold {
                    value_divisor,
                    unit_value_cap,
                } => (1..=100).contains(value_divisor) && (1..=1_000_000).contains(unit_value_cap),
                AbilityEffectDefinition::DrainItemMagic {
                    base_power,
                    level_multiplier,
                    level_divisor,
                } => {
                    *base_power <= 10_000
                        && *level_multiplier <= 1_000
                        && (1..=1_000).contains(level_divisor)
                }
                AbilityEffectDefinition::ReportMagic
                | AbilityEffectDefinition::PolymorphSelf
                | AbilityEffectDefinition::PolymorphTarget => true,
                AbilityEffectDefinition::Earthquake {
                    radius,
                    affect_chance_percent,
                    floor_terrain_id,
                    wall_terrain_ids,
                } => {
                    (1..=12).contains(radius)
                        && (1..=100).contains(affect_chance_percent)
                        && !floor_terrain_id.is_empty()
                        && !wall_terrain_ids.is_empty()
                        && wall_terrain_ids.len() <= 8
                        && wall_terrain_ids.windows(2).all(|pair| pair[0] != pair[1])
                        && wall_terrain_ids.iter().all(|id| id != floor_terrain_id)
                }
                AbilityEffectDefinition::AreaDestruction {
                    minimum_radius,
                    maximum_radius,
                    floor_terrain_id,
                    wall_terrain_id,
                    quartz_terrain_id,
                    magma_terrain_id,
                } => {
                    (1..=32).contains(minimum_radius)
                        && minimum_radius <= maximum_radius
                        && *maximum_radius <= 32
                        && [
                            floor_terrain_id,
                            wall_terrain_id,
                            quartz_terrain_id,
                            magma_terrain_id,
                        ]
                        .iter()
                        .all(|id| !id.is_empty())
                }
                AbilityEffectDefinition::SuppressMonsterReproduction {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                }
                AbilityEffectDefinition::MeleeThenTeleport {
                    radius,
                    failure_threshold,
                } => (1..=255).contains(radius) && *failure_threshold <= 1_000,
                AbilityEffectDefinition::Recall {
                    delay_dice,
                    delay_sides,
                    delay_bonus,
                } => {
                    (1..=100).contains(delay_dice)
                        && (1..=10_000).contains(delay_sides)
                        && *delay_bonus <= 10_000
                }
                AbilityEffectDefinition::ResistElements {
                    duration_dice,
                    duration_sides,
                    duration_bonus,
                } => {
                    (1..=100).contains(duration_dice)
                        && (1..=1_000_000).contains(duration_sides)
                        && *duration_bonus <= 1_000_000
                }
                AbilityEffectDefinition::Summon {
                    actor_kind_id,
                    count,
                    radius,
                    duration_turns,
                    hostile,
                } => {
                    validate_id(actor_kind_id).is_ok()
                        && (1..=8).contains(count)
                        && (1..=64).contains(radius)
                        && ((*hostile && *duration_turns <= 10_000)
                            || (!*hostile && (1..=10_000).contains(duration_turns)))
                }
                AbilityEffectDefinition::SummonCategory {
                    category,
                    upgraded_category,
                    upgrade_at_level,
                    maximum_level,
                    count_dice,
                    count_sides,
                    count_bonus,
                    maximum_count,
                    batch_candidates,
                    hostile_chance_percent,
                    friendly_group_chance_percent,
                    hostile_group_chance_percent,
                    group_count_dice,
                    group_count_sides,
                    group_count_bonus,
                    radius,
                    duration_turns,
                    ..
                } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && actor_tag_values.contains(category)
                        && match (upgraded_category, upgrade_at_level) {
                            (None, None) => true,
                            (Some(category), Some(level)) => {
                                actor_tag_values.contains(category) && (1..=100).contains(level)
                            }
                            _ => false,
                        }
                        && ((1..=1_000).contains(maximum_level)
                            || (*maximum_level == 0
                                && has_level_scaling(AbilityLevelScalingField::SummonMaximumLevel)))
                        && (1..=8).contains(count_dice)
                        && (1..=8).contains(count_sides)
                        && u16::from(*count_dice) * u16::from(*count_sides)
                            + u16::from(*count_bonus)
                            <= 8
                        && maximum_count.is_none_or(|maximum_count| {
                            (1..=8).contains(&maximum_count)
                                && u16::from(maximum_count)
                                    <= u16::from(*count_dice) * u16::from(*count_sides)
                                        + u16::from(*count_bonus)
                        })
                        && batch_candidates.len() <= 8
                        && batch_candidates
                            .iter()
                            .all(|candidate| candidate.weight > 0)
                        && batch_candidates
                            .iter()
                            .map(|candidate| candidate.actor_kind_id.as_str())
                            .collect::<BTreeSet<_>>()
                            .len()
                            == batch_candidates.len()
                        && (batch_candidates.is_empty() || ability.player.is_none())
                        && *hostile_chance_percent <= 100
                        && *friendly_group_chance_percent <= 100
                        && *hostile_group_chance_percent <= 100
                        && if *friendly_group_chance_percent == 0
                            && *hostile_group_chance_percent == 0
                        {
                            *group_count_dice == 0
                                && *group_count_sides == 0
                                && *group_count_bonus == 0
                        } else {
                            (1..=8).contains(group_count_dice)
                                && (1..=8).contains(group_count_sides)
                                && u16::from(*group_count_dice) * u16::from(*group_count_sides)
                                    + u16::from(*group_count_bonus)
                                    <= 8
                        }
                        && (1..=64).contains(radius)
                        && *duration_turns <= 10_000
                }
                AbilityEffectDefinition::Detect {
                    subject,
                    category,
                    radius,
                    persistent,
                    ..
                } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && (1..=64).contains(radius)
                        && match subject {
                            AbilityDetectSubjectDefinition::Terrain => {
                                category == "map"
                                    || terrain_tags.values().any(|tags| tags.contains(category))
                            }
                            AbilityDetectSubjectDefinition::Actor => {
                                !persistent
                                    && (category == "any-monster"
                                        || category == "normal-monster"
                                        || actor_tag_values.contains(category))
                            }
                            AbilityDetectSubjectDefinition::Item => {
                                !persistent
                                    && (category == "item"
                                        || category == "magic-item"
                                        || item_tag_values.contains(category))
                            }
                            AbilityDetectSubjectDefinition::Gold => {
                                !persistent && category == "gold"
                            }
                            AbilityDetectSubjectDefinition::Curse => {
                                !persistent && category == "curse"
                            }
                        }
                }
                AbilityEffectDefinition::RefuelEquippedLight {
                    maximum_fraction_divisor,
                } => (1..=100).contains(maximum_fraction_divisor),
                AbilityEffectDefinition::TransformTerrain {
                    source_terrain_ids,
                    target_terrain_id,
                    radius,
                } => {
                    !source_terrain_ids.is_empty()
                        && source_terrain_ids.len() <= 32
                        && !target_terrain_id.is_empty()
                        && *radius <= 8
                        && source_terrain_ids.windows(2).all(|pair| pair[0] != pair[1])
                        && source_terrain_ids
                            .iter()
                            .all(|source_id| source_id != target_terrain_id)
                }
                AbilityEffectDefinition::TerrainBeam { .. } => true,
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    duration_dice,
                    duration_sides,
                    power,
                    granted_resistances,
                    granted_brands,
                    granted_modifiers,
                    granted_equipment_bonuses,
                    granted_status_immunities,
                    granted_race_id,
                    incoming_damage_percent,
                    ..
                } => {
                    validate_id(status_kind_id).is_ok()
                        && (1..=1_000).contains(intensity)
                        && (*duration_ticks > 0 || *duration_dice > 0)
                        && *duration_ticks <= 1_000_000
                        && *duration_dice <= 100
                        && ((*duration_dice == 0 && *duration_sides == 0)
                            || (*duration_dice > 0 && (1..=1_000_000).contains(duration_sides)))
                        && power.is_none_or(|power| (1..=1_000).contains(&power))
                        && granted_resistances.len() <= 29
                        && granted_brands.len() <= 5
                        && granted_modifiers.max_hp.abs() <= 1_000_000
                        && granted_modifiers.attack.abs() <= 1_000_000
                        && granted_modifiers.defense.abs() <= 1_000_000
                        && (-100..=100).contains(&granted_modifiers.speed)
                        && !attribute_modifiers_out_of_range(granted_modifiers)
                        && !equipment_bonuses_out_of_range(granted_equipment_bonuses)
                        && granted_status_immunities.len() <= 32
                        && granted_status_immunities
                            .iter()
                            .all(|status_id| validate_id(status_id).is_ok())
                        && granted_race_id
                            .as_ref()
                            .is_none_or(|race_id| validate_id(race_id).is_ok())
                        && *incoming_damage_percent <= 100
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    validate_id(status_kind_id).is_ok()
                }
                AbilityEffectDefinition::Control { category, power } => {
                    !category.is_empty()
                        && category.len() <= 64
                        && category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && (category == "any-monster" || actor_tag_values.contains(category))
                        && (1..=1_000).contains(power)
                }
                AbilityEffectDefinition::DrainLife {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    target_category,
                    repeat,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && !target_category.is_empty()
                        && target_category.len() <= 64
                        && target_category.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                        && actor_tag_values.contains(target_category)
                        && (1..=16).contains(repeat)
                }
                AbilityEffectDefinition::Genocide {
                    scope,
                    power,
                    radius,
                    target_category,
                    ..
                } => {
                    ((1..=1_000).contains(power)
                        || (*power == 0
                            && has_level_scaling(AbilityLevelScalingField::GenocidePower)))
                        && match scope {
                            AbilityGenocideScopeDefinition::Single
                            | AbilityGenocideScopeDefinition::Glyph => *radius == 0,
                            AbilityGenocideScopeDefinition::Nearby => (1..=64).contains(radius),
                        }
                        && target_category.as_ref().is_none_or(|category| {
                            category == "any-monster" || actor_tag_values.contains(category)
                        })
                }
                AbilityEffectDefinition::IdentifyItem {
                    full_identify_power,
                    full_identify_roll_sides,
                } => {
                    (*full_identify_power == 0 && *full_identify_roll_sides == 0)
                        || (((1..=1_000).contains(full_identify_power)
                            || (*full_identify_power == 0
                                && has_level_scaling(AbilityLevelScalingField::IdentifyPower)))
                            && (1..=1_000).contains(full_identify_roll_sides))
                }
                AbilityEffectDefinition::RestoreVitality { life_force } => {
                    (1..=1_000).contains(life_force)
                }
                AbilityEffectDefinition::AnimateDead {
                    actor_kind_id,
                    corpse_item_kind_id,
                    radius,
                    count,
                    failure_chance_percent,
                } => {
                    validate_id(actor_kind_id).is_ok()
                        && validate_id(corpse_item_kind_id).is_ok()
                        && (1..=8).contains(radius)
                        && (1..=8).contains(count)
                        && *failure_chance_percent <= 100
                }
                AbilityEffectDefinition::Heal { amount } => (1..=1_000_000).contains(amount),
                AbilityEffectDefinition::HealDice { dice, sides } => {
                    (1..=100).contains(dice) && (1..=10_000).contains(sides)
                }
                AbilityEffectDefinition::ReduceStatus {
                    status_kind_id,
                    amount,
                    current_divisor,
                    remaining_divisor,
                } => {
                    validate_id(status_kind_id).is_ok()
                        && (1..=1_000_000).contains(amount)
                        && current_divisor.is_none_or(|divisor| (1..=1_000_000).contains(&divisor))
                        && remaining_divisor
                            .is_none_or(|divisor| (1..=1_000_000).contains(&divisor))
                        && !(current_divisor.is_some() && remaining_divisor.is_some())
                        && (remaining_divisor.is_none() || status_kind_id == "rfb.status.bleeding")
                }
                AbilityEffectDefinition::SatisfyHunger => true,
                AbilityEffectDefinition::VisibleDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    target_category,
                    ..
                } => {
                    (1..=100).contains(damage_dice)
                        && (1..=10_000).contains(damage_sides)
                        && *damage_bonus <= 10_000
                        && target_category
                            .as_ref()
                            .is_none_or(|category| actor_tag_values.contains(category))
                }
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id,
                    intensity,
                    duration_ticks,
                    power,
                    target_category,
                    ..
                } => {
                    validate_id(status_kind_id).is_ok()
                        && (1..=1_000).contains(intensity)
                        && (1..=1_000_000).contains(duration_ticks)
                        && power.is_none_or(|power| (1..=1_000).contains(&power))
                        && target_category
                            .as_ref()
                            .is_none_or(|category| actor_tag_values.contains(category))
                }
                AbilityEffectDefinition::BrandWeapon { affix_id, .. } => {
                    validate_id(affix_id).is_ok()
                }
                AbilityEffectDefinition::RandomChoice { .. } => false,
                AbilityEffectDefinition::NoOp { reason } => {
                    !reason.is_empty() && reason.len() <= 128 && reason.is_ascii()
                }
                AbilityEffectDefinition::Sequence { .. } => false,
            }
        };
        let valid_effect = match &ability.effect {
            AbilityEffectDefinition::Sequence { effects } => {
                (2..=8).contains(&effects.len())
                    && effects
                        .iter()
                        .enumerate()
                        .all(|(index, effect)| valid_single_effect(effect, index))
            }
            AbilityEffectDefinition::RandomChoice {
                roll_sides,
                level_bonus_divisor,
                branches,
            } => {
                let maximum_roll = u32::from(*roll_sides)
                    + if *level_bonus_divisor == 0 {
                        0
                    } else {
                        100 / u32::from(*level_bonus_divisor)
                    };
                (2..=10_000).contains(roll_sides)
                    && (*level_bonus_divisor == 0 || *level_bonus_divisor <= 100)
                    && (2..=64).contains(&branches.len())
                    && branches.iter().all(|branch| {
                        valid_ability_level_scaling(&branch.effect, &branch.level_scaling)
                            && match branch.target {
                                AbilityRandomTargetDefinition::SelfTarget => {
                                    match branch.effect.as_ref() {
                                        AbilityEffectDefinition::Sequence { effects } => {
                                            (2..=8).contains(&effects.len())
                                                && effects.iter().enumerate().all(
                                                    |(index, effect)| {
                                                        valid_single_effect(effect, index)
                                                            && matches!(
                                                                effect,
                                                                AbilityEffectDefinition::Heal {
                                                                    ..
                                                                } | AbilityEffectDefinition::VisibleDamage {
                                                                    ..
                                                                } | AbilityEffectDefinition::VisibleApplyStatus {
                                                                    ..
                                                                }
                                                            )
                                                    },
                                                )
                                        }
                                        effect => {
                                            valid_single_effect(effect, usize::MAX)
                                                && matches!(
                                                    effect,
                                                    AbilityEffectDefinition::Heal { .. }
                                                        | AbilityEffectDefinition::ApplyStatus { .. }
                                                        | AbilityEffectDefinition::Summon { .. }
                                                        | AbilityEffectDefinition::VisibleDamage { .. }
                                                        | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                                        | AbilityEffectDefinition::Earthquake { .. }
                                                        | AbilityEffectDefinition::AreaDestruction { .. }
                                                        | AbilityEffectDefinition::NoOp { .. }
                                                )
                                        }
                                    }
                                }
                                AbilityRandomTargetDefinition::CastTarget => {
                                    valid_single_effect(&branch.effect, usize::MAX)
                                        && matches!(
                                            branch.effect.as_ref(),
                                            AbilityEffectDefinition::Damage { .. }
                                                | AbilityEffectDefinition::AreaDamage { .. }
                                                | AbilityEffectDefinition::BeamDamage { .. }
                                                | AbilityEffectDefinition::LightLine { .. }
                                                | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                                                | AbilityEffectDefinition::ApplyStatus { .. }
                                                | AbilityEffectDefinition::DrainLife { .. }
                                                | AbilityEffectDefinition::Genocide { .. }
                                                | AbilityEffectDefinition::PolymorphTarget
                                                | AbilityEffectDefinition::NoOp { .. }
                                        )
                                }
                            }
                    })
                    && branches
                        .windows(2)
                        .all(|pair| pair[0].maximum_roll < pair[1].maximum_roll)
                    && branches
                        .last()
                        .is_some_and(|branch| u32::from(branch.maximum_roll) >= maximum_roll)
            }
            effect => valid_single_effect(effect, 0),
        };
        let valid_level_scaling =
            valid_ability_level_scaling(&ability.effect, &ability.level_scaling);
        let valid_spell_power =
            valid_ability_spell_power(&ability.effect, &ability.spell_power_fields);
        let self_targeted = ability
            .target
            .modes
            .contains(&AbilityTargetModeDefinition::SelfTarget);
        let directional_effect = matches!(
            &ability.effect,
            AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. }
                | AbilityEffectDefinition::TerrainBeam { .. }
        ) || matches!(
            &ability.effect,
            AbilityEffectDefinition::CreateAmmunition {
                source_terrain_tags,
                ..
            } if !source_terrain_tags.is_empty()
        );
        let self_target_rule = ability.target.modes.as_slice()
            == [AbilityTargetModeDefinition::SelfTarget]
            && ability.target.range == 0
            && !ability.target.requires_line_of_effect;
        let projectile_target_rule = !self_targeted
            && (1..=64).contains(&ability.target.range)
            && ability.target.requires_line_of_effect;
        let room_target_rule = !self_targeted
            && (1..=64).contains(&ability.target.range)
            && ability
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::Entity)
            && !ability.target.requires_line_of_effect;
        let item_target_rule = ability.target.modes.as_slice()
            == [AbilityTargetModeDefinition::Item]
            && ability.target.range == 0
            && !ability.target.requires_line_of_effect;
        let valid_target = match &ability.effect {
            AbilityEffectDefinition::Damage { .. }
            | AbilityEffectDefinition::Malediction { .. }
            | AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::LightLine { .. }
            | AbilityEffectDefinition::TerrainBeam { .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { .. }
            | AbilityEffectDefinition::BoltOrAreaDamage { .. }
            | AbilityEffectDefinition::ConeDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::BirdDrop
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::PolymorphTarget
            | AbilityEffectDefinition::DrainLife { .. }
            | AbilityEffectDefinition::DeathRay { .. }
            | AbilityEffectDefinition::RandomChoice { .. } => projectile_target_rule,
            AbilityEffectDefinition::TeleportLevel => self_target_rule || projectile_target_rule,
            AbilityEffectDefinition::FetchItem { .. }
            | AbilityEffectDefinition::ConsumeTerrain { .. }
            | AbilityEffectDefinition::MeleeThenTeleport { .. }
            | AbilityEffectDefinition::SwapPosition => projectile_target_rule,
            AbilityEffectDefinition::CreateAmmunition {
                source_item_tags, ..
            } => {
                if source_item_tags.is_empty() {
                    projectile_target_rule
                } else {
                    item_target_rule
                }
            }
            AbilityEffectDefinition::DarkenRoom => room_target_rule,
            AbilityEffectDefinition::Genocide { scope, .. } => match scope {
                AbilityGenocideScopeDefinition::Nearby => self_target_rule,
                AbilityGenocideScopeDefinition::Single | AbilityGenocideScopeDefinition::Glyph => {
                    projectile_target_rule
                }
            },
            AbilityEffectDefinition::IdentifyItem { .. }
            | AbilityEffectDefinition::BrandWeapon { .. }
            | AbilityEffectDefinition::TransmuteItemToGold { .. }
            | AbilityEffectDefinition::DrainItemMagic { .. }
            | AbilityEffectDefinition::RechargeFromPlayer { .. } => item_target_rule,
            AbilityEffectDefinition::AreaDamage { .. } => {
                self_target_rule || projectile_target_rule
            }
            AbilityEffectDefinition::JumpDamage { .. } => self_target_rule,
            AbilityEffectDefinition::Control { .. } => projectile_target_rule,
            AbilityEffectDefinition::BreathDamage { .. } => projectile_target_rule,
            AbilityEffectDefinition::Teleport => {
                !self_targeted
                    && ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Position]
                    && (1..=64).contains(&ability.target.range)
                    && ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::Summon { .. }
            | AbilityEffectDefinition::SummonCategory { .. }
            | AbilityEffectDefinition::AnimateDead { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                    && ability.target.range == 0
                    && !ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::Heal { .. }
            | AbilityEffectDefinition::HealDice { .. }
            | AbilityEffectDefinition::ReduceStatus { .. }
            | AbilityEffectDefinition::SatisfyHunger
            | AbilityEffectDefinition::Clairvoyance { .. }
            | AbilityEffectDefinition::RefuelEquippedLight { .. }
            | AbilityEffectDefinition::LightArea { .. }
            | AbilityEffectDefinition::AggravateMonsters
            | AbilityEffectDefinition::Recall { .. }
            | AbilityEffectDefinition::ResistElements { .. }
            | AbilityEffectDefinition::VisibleDamage { .. }
            | AbilityEffectDefinition::VisibleApplyStatus { .. }
            | AbilityEffectDefinition::RestoreVitality { .. }
            | AbilityEffectDefinition::ReportMagic
            | AbilityEffectDefinition::Earthquake { .. }
            | AbilityEffectDefinition::AreaDestruction { .. }
            | AbilityEffectDefinition::SuppressMonsterReproduction { .. }
            | AbilityEffectDefinition::PolymorphSelf
            | AbilityEffectDefinition::NoOp { .. } => self_target_rule,
            AbilityEffectDefinition::Detect { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::SelfTarget]
                    && ability.target.range == 0
                    && !ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::TransformTerrain { .. } => {
                ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Position]
                    && (1..=64).contains(&ability.target.range)
                    && ability.target.requires_line_of_effect
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. } => {
                self_target_rule || projectile_target_rule
            }
            AbilityEffectDefinition::BlinkSelf { .. }
            | AbilityEffectDefinition::TeleportSelf { .. } => self_target_rule,
            AbilityEffectDefinition::BlinkTarget { .. }
            | AbilityEffectDefinition::TeleportTarget => projectile_target_rule,
            AbilityEffectDefinition::Sequence { effects } => {
                (self_target_rule
                    && effects.iter().all(|effect| {
                        matches!(
                            effect,
                            AbilityEffectDefinition::Heal { .. }
                                | AbilityEffectDefinition::HealDice { .. }
                                | AbilityEffectDefinition::ReduceStatus { .. }
                                | AbilityEffectDefinition::ApplyStatus { .. }
                                | AbilityEffectDefinition::RemoveStatus { .. }
                                | AbilityEffectDefinition::AnimateDead { .. }
                                | AbilityEffectDefinition::VisibleDamage { .. }
                                | AbilityEffectDefinition::VisibleApplyStatus { .. }
                                | AbilityEffectDefinition::AreaDamage { .. }
                                | AbilityEffectDefinition::AggravateMonsters
                                | AbilityEffectDefinition::Detect { .. }
                                | AbilityEffectDefinition::NoOp { .. }
                        )
                    }))
                    || (projectile_target_rule
                        && effects.iter().all(|effect| {
                            matches!(
                                effect,
                                AbilityEffectDefinition::Damage { .. }
                                    | AbilityEffectDefinition::ApplyStatus { .. }
                                    | AbilityEffectDefinition::RemoveStatus { .. }
                            )
                        }))
            }
        };
        let directional_target = !directional_effect
            || ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Direction];
        let valid_player = ability.player.as_ref().is_none_or(|player| {
            (1..=100).contains(&player.minimum_level)
                && (1..=1_000_000).contains(&player.resource_cost)
                && player.base_failure_percent <= 95
                && player.first_success_experience <= 1_000_000
                && player.proficiency.initial <= player.proficiency.cap
                && player.proficiency.cap <= 1600
                && player
                    .proficiency
                    .success_gain
                    .saturating_add(player.proficiency.failure_gain)
                    <= 10_000
                && player
                    .cooldown
                    .as_ref()
                    .is_none_or(|cooldown| cooldown.turns > 0)
                && player
                    .cooldown
                    .as_ref()
                    .and_then(|cooldown| cooldown.group_id.as_deref())
                    .is_none_or(|group_id| validate_id(group_id).is_ok())
        });
        if !valid_player
            || ability.target.modes.is_empty()
            || ability.target.modes.len() > 5
            || ability.target.modes.iter().any(|mode| !modes.insert(*mode))
            || !valid_target
            || !valid_effect
            || !valid_level_scaling
            || !valid_spell_power
            || !directional_target
            || (ability.affects_ground_items && !effect_can_affect_ground_items(&ability.effect))
        {
            return Err(ContentError::InvalidAbility(ability.id.clone()));
        }
        if let Some(player) = &ability.player {
            require_reference(&resource_ids, &player.resource_id, &ability.id)?;
        }
        let referenced_effects = match &ability.effect {
            AbilityEffectDefinition::RandomChoice { branches, .. } => branches
                .iter()
                .flat_map(|branch| branch.effect.ordered_effects())
                .collect::<Vec<_>>(),
            effect => vec![effect],
        };
        for effect in referenced_effects {
            if let AbilityEffectDefinition::Summon { actor_kind_id, .. } = effect {
                require_actor_role(actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
            }
            if let AbilityEffectDefinition::SummonCategory {
                batch_candidates, ..
            } = effect
            {
                for candidate in batch_candidates {
                    require_actor_role(
                        actor_roles,
                        &candidate.actor_kind_id,
                        ActorRole::Monster,
                        &ability.id,
                    )?;
                }
            }
            if let AbilityEffectDefinition::BrandWeapon { affix_id, .. } = effect {
                require_reference(affix_ids, affix_id, &ability.id)?;
            }
            if let AbilityEffectDefinition::ApplyStatus {
                granted_race_id: Some(race_id),
                ..
            } = effect
            {
                ability_race_ids.push((ability.id.clone(), race_id.clone()));
            }
            if let AbilityEffectDefinition::Earthquake {
                floor_terrain_id,
                wall_terrain_ids,
                ..
            } = effect
            {
                require_reference(terrain_ids, floor_terrain_id, &ability.id)?;
                for wall_terrain_id in wall_terrain_ids {
                    require_reference(terrain_ids, wall_terrain_id, &ability.id)?;
                }
            }
            if let AbilityEffectDefinition::AreaDestruction {
                floor_terrain_id,
                wall_terrain_id,
                quartz_terrain_id,
                magma_terrain_id,
                ..
            } = effect
            {
                for terrain_id in [
                    floor_terrain_id,
                    wall_terrain_id,
                    quartz_terrain_id,
                    magma_terrain_id,
                ] {
                    require_reference(terrain_ids, terrain_id, &ability.id)?;
                }
            }
        }
        if let AbilityEffectDefinition::Summon { actor_kind_id, .. } = &ability.effect {
            require_actor_role(actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
        }
        for effect in ability.effect.ordered_effects() {
            if let AbilityEffectDefinition::AnimateDead {
                actor_kind_id,
                corpse_item_kind_id,
                ..
            } = effect
            {
                require_actor_role(actor_roles, actor_kind_id, ActorRole::Monster, &ability.id)?;
                ability_corpse_item_ids.push((ability.id.clone(), corpse_item_kind_id.clone()));
            }
            if let AbilityEffectDefinition::CreateAmmunition { item_kind_ids, .. } = effect {
                ability_created_item_ids.extend(
                    item_kind_ids
                        .iter()
                        .cloned()
                        .map(|item_id| (ability.id.clone(), item_id)),
                );
            }
        }
        if let AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            ..
        } = &ability.effect
        {
            for source_terrain_id in source_terrain_ids {
                require_reference(terrain_ids, source_terrain_id, &ability.id)?;
            }
            require_reference(terrain_ids, target_terrain_id, &ability.id)?;
        }
        if let AbilityEffectDefinition::Earthquake {
            floor_terrain_id,
            wall_terrain_ids,
            ..
        } = &ability.effect
        {
            require_reference(terrain_ids, floor_terrain_id, &ability.id)?;
            for wall_terrain_id in wall_terrain_ids {
                require_reference(terrain_ids, wall_terrain_id, &ability.id)?;
            }
        }
        normalize_tags(&ability.id, &mut ability.tags)?;
        insert_definition_id(all_ids, &ability.id)?;
        if let Some(player) = &ability.player {
            ability_resources.insert(ability.id.clone(), player.resource_id.clone());
        }
        ability_ids.insert(ability.id.clone());
    }
    for (actor_id, casting) in actor_monster_casting {
        for candidate in casting.abilities {
            let Some(ability) = definitions
                .abilities
                .iter()
                .find(|ability| ability.id == candidate.ability_id)
            else {
                return Err(ContentError::DanglingReference {
                    owner: actor_id.clone(),
                    target: candidate.ability_id,
                });
            };
            let self_target = ability.target.modes.as_slice()
                == [AbilityTargetModeDefinition::SelfTarget]
                && ability.target.range == 0
                && !ability.target.requires_line_of_effect;
            let projectile_target = ability
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::Entity)
                && !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                && ability.target.requires_line_of_effect;
            let position_target = ability.target.modes.as_slice()
                == [AbilityTargetModeDefinition::Position]
                && ability.target.requires_line_of_effect;
            let room_target = ability
                .target
                .modes
                .contains(&AbilityTargetModeDefinition::Entity)
                && !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                && !ability.target.requires_line_of_effect;
            let supported = match &ability.effect {
                AbilityEffectDefinition::Damage { .. }
                | AbilityEffectDefinition::AreaDamage { .. }
                | AbilityEffectDefinition::BeamDamage { .. }
                | AbilityEffectDefinition::BoltOrBeamDamage { .. }
                | AbilityEffectDefinition::CurseDamage { .. }
                | AbilityEffectDefinition::TeleportAway { .. }
                | AbilityEffectDefinition::BirdDrop
                | AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia
                | AbilityEffectDefinition::TeleportLevel
                | AbilityEffectDefinition::PolymorphTarget => projectile_target,
                AbilityEffectDefinition::DarkenRoom => room_target,
                AbilityEffectDefinition::ConeDamage { .. }
                | AbilityEffectDefinition::BreathDamage { .. } => {
                    ability.target.modes.as_slice() == [AbilityTargetModeDefinition::Direction]
                        && ability.target.requires_line_of_effect
                }
                AbilityEffectDefinition::Heal { .. }
                | AbilityEffectDefinition::AggravateMonsters
                | AbilityEffectDefinition::Summon { .. }
                | AbilityEffectDefinition::SummonCategory { .. }
                | AbilityEffectDefinition::JumpDamage { .. } => self_target,
                AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. } => self_target || projectile_target,
                AbilityEffectDefinition::BlinkSelf { .. }
                | AbilityEffectDefinition::TeleportSelf { .. } => self_target,
                AbilityEffectDefinition::BlinkTarget { .. }
                | AbilityEffectDefinition::TeleportTarget => projectile_target,
                AbilityEffectDefinition::DrainLife { .. }
                | AbilityEffectDefinition::Genocide { .. } => projectile_target,
                AbilityEffectDefinition::AnimateDead { .. } => self_target,
                AbilityEffectDefinition::TransformTerrain { .. } => position_target,
                AbilityEffectDefinition::Sequence { effects } => {
                    (self_target
                        && effects.iter().all(|effect| {
                            matches!(
                                effect,
                                AbilityEffectDefinition::Heal { .. }
                                    | AbilityEffectDefinition::ApplyStatus { .. }
                                    | AbilityEffectDefinition::RemoveStatus { .. }
                                    | AbilityEffectDefinition::AnimateDead { .. }
                            )
                        }))
                        || (projectile_target
                            && effects.iter().all(|effect| {
                                matches!(
                                    effect,
                                    AbilityEffectDefinition::Damage { .. }
                                        | AbilityEffectDefinition::ApplyStatus { .. }
                                        | AbilityEffectDefinition::RemoveStatus { .. }
                                )
                            }))
                }
                _ => false,
            };
            if !supported {
                return Err(ContentError::InvalidMonsterCasting(actor_id.clone()));
            }
        }
    }

    let mut ability_books_by_id = BTreeMap::new();
    let mut ability_book_ids = BTreeSet::new();
    for book in definitions.ability_books.iter_mut() {
        require_schema(&book.schema, ABILITY_BOOK_SCHEMA, &book.id)?;
        require_format_version(book.format_version, &book.id)?;
        validate_definition_id(&book.id, "ability-book")?;
        validate_definition_text(&book.id, &book.name_key, &book.description_key)?;
        if book.realm_id.is_some() != book.rank.is_some()
            || book.rank.is_some_and(|rank| !(1..=4).contains(&rank))
            || book.realm_id.as_deref().is_some_and(|realm_id| {
                realm_id.is_empty()
                    || realm_id.len() > 64
                    || !realm_id.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-'
                    })
            })
        {
            return Err(ContentError::InvalidAbilityBook(book.id.clone()));
        }
        book.ability_ids.sort();
        let mut members = BTreeSet::new();
        if book.ability_ids.is_empty()
            || book.ability_ids.len() > 64
            || book
                .ability_ids
                .iter()
                .any(|ability_id| !members.insert(ability_id.clone()))
        {
            return Err(ContentError::InvalidAbilityBook(book.id.clone()));
        }
        for ability_id in &book.ability_ids {
            require_reference(&ability_ids, ability_id, &book.id)?;
        }
        normalize_tags(&book.id, &mut book.tags)?;
        insert_definition_id(all_ids, &book.id)?;
        ability_book_ids.insert(book.id.clone());
        ability_books_by_id.insert(book.id.clone(), book.clone());
    }
    Ok(AbilityValidationOutputs {
        resource_ids,
        ability_resources,
        ability_ids,
        ability_corpse_item_ids,
        ability_created_item_ids,
        ability_race_ids,
        ability_books_by_id,
        ability_book_ids,
    })
}
