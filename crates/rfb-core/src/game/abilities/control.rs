// SPDX-License-Identifier: MPL-2.0

use crate::effect::{
    DamagePacket, STATUS_FEAR, STATUS_PARALYSIS, STATUS_SLEEP, STATUS_SLOW, resolve_damage,
};
use crate::event::{DomainEvent, ProjectileTrace};
use crate::game::abilities::AbilityTargetPlan;
use crate::game::ability_scaling::spell_power_value;
use crate::game::damage::FatalityPolicy;
use crate::game::item_use::VisibleBanishmentOutcome;
use crate::game::projectile_geometry::rfb_distance;
use crate::game::status_effects::{apply_ability_status_effect, remove_ability_status_effect};
use crate::game::{
    CRUSADE_ARREST_ABILITY_ID, Game, ability_genocide_scope_dto, actor_matches_category,
    chebyshev_distance,
};
use crate::resistance::{DamageType, ResistanceLevel, ResistanceProfile};
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilityGenocideScopeDefinition,
    AbilityStatusStackingDefinition, EquipmentBonuses, StatModifiers,
};
use rfb_protocol::{
    AbilityBanishTargetDto, AbilityControlOutcomeDto, AbilityEffectResolutionDto,
    AbilityEffectSkipReasonDto, AbilityEffectsResolutionDto, AbilityStatusChangeDto,
    AbilityStatusStackingDto, MonsterPackRoleDto, Position, VirtueKindDto,
};
use std::collections::{BTreeMap, BTreeSet};

impl Game {
    pub(in crate::game) fn resolve_ability_control(
        &mut self,
        target_index: usize,
        effect_index: u8,
        category: &str,
        power: u16,
    ) -> AbilityEffectResolutionDto {
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("controlled actor definition must remain available");
        let target_level = definition.level;
        let eligible =
            category == "any-monster" || definition.tags.iter().any(|tag| tag == category);
        let already_controlled = self.entity_is_player_aligned(target_index);
        let (roll, outcome) = if already_controlled {
            (None, AbilityControlOutcomeDto::AlreadyControlled)
        } else if !eligible {
            (None, AbilityControlOutcomeDto::Ineligible)
        } else {
            let range = power.saturating_sub(10).max(1);
            let roll = u16::try_from(self.rng.bounded(u64::from(range)) + 1)
                .expect("validated control power roll must fit u16");
            if target_level > u32::from(roll).saturating_add(10) {
                (Some(roll), AbilityControlOutcomeDto::Resisted)
            } else {
                let pack = self.entities[target_index].pack.clone();
                if let Some(pack) = pack {
                    if pack.role == MonsterPackRoleDto::Leader || pack.leader_id == target_entity_id
                    {
                        for entity in &mut self.entities {
                            if entity
                                .pack
                                .as_ref()
                                .is_some_and(|identity| identity.id == pack.id)
                            {
                                entity.pack = None;
                            }
                        }
                    } else {
                        self.entities[target_index].pack = None;
                    }
                }
                self.entities[target_index].controller_id = Some(self.player.id.clone());
                (Some(roll), AbilityControlOutcomeDto::Controlled)
            }
        };
        AbilityEffectResolutionDto::Control {
            effect_index,
            category: category.to_owned(),
            power,
            target_entity_id,
            target_kind_id,
            target_level,
            roll,
            outcome,
        }
    }

    pub(in crate::game) fn resolve_player_control_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Control { category, power } = &ability.effect else {
            unreachable!("control executor requires a control effect");
        };
        let AbilityTargetPlan::Projectile { path, .. } = target_plan else {
            unreachable!("control effects require a projectile target plan");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        self.entities[target_index].alerted = true;
        changed.insert(self.entities[target_index].position);
        let resolution = self.resolve_ability_control(target_index, 0, category, *power);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(in crate::game) fn resolve_player_actor_status_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        debug_assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. }
        ));
        match target_plan {
            AbilityTargetPlan::SelfTarget => {
                let resolution = match &ability.effect {
                    AbilityEffectDefinition::ApplyStatus {
                        status_kind_id,
                        intensity,
                        duration_ticks,
                        duration_dice,
                        duration_sides,
                        stacking,
                        resistance_type,
                        power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id,
                        grants_wall_passage,
                        incoming_damage_percent,
                    } => apply_ability_status_effect(
                        &mut self.player,
                        &ability.id,
                        0,
                        status_kind_id,
                        *intensity,
                        *duration_ticks,
                        *duration_dice,
                        *duration_sides,
                        *stacking,
                        *resistance_type,
                        *power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id.as_deref(),
                        *grants_wall_passage,
                        *incoming_damage_percent,
                        None,
                        None,
                        &mut self.rng,
                    ),
                    AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                        remove_ability_status_effect(&mut self.player, 0, status_kind_id)
                    }
                    _ => unreachable!("actor status executor requires a status effect"),
                };
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(self.player.id.clone()),
                        target_kind_id: Some(self.player.kind_id.clone()),
                        effects: vec![resolution],
                    },
                    trace: None,
                });
                self.refresh_player_resource_maxima();
            }
            AbilityTargetPlan::Projectile { path, .. } => {
                let (trace, target_index) = self.trace_projectile_path(path);
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: vec![AbilityEffectResolutionDto::Skipped {
                                effect_index: 0,
                                reason: AbilityEffectSkipReasonDto::NoTarget,
                            }],
                        },
                        trace: Some(trace),
                    });
                    return;
                };
                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                if ability.id == CRUSADE_ARREST_ABILITY_ID {
                    self.resolve_crusade_arrest_effect(
                        ability,
                        target_index,
                        trace,
                        events,
                        changed,
                    );
                    return;
                }
                let resolution = match &ability.effect {
                    AbilityEffectDefinition::ApplyStatus {
                        status_kind_id,
                        intensity,
                        duration_ticks,
                        duration_dice,
                        duration_sides,
                        stacking,
                        resistance_type,
                        power,
                        granted_resistances,
                        granted_brands,
                        granted_modifiers,
                        granted_equipment_bonuses,
                        granted_status_immunities,
                        granted_race_id,
                        grants_wall_passage,
                        incoming_damage_percent,
                    } => {
                        let target_level = self
                            .content
                            .actor(&target_kind_id)
                            .map(|definition| definition.level);
                        let target_resistances = self.entities[target_index].resistances.clone();
                        let mut target_immunities = BTreeSet::new();
                        if let Some(definition) =
                            self.actor_runtime_definition(&self.entities[target_index])
                        {
                            target_immunities.extend(definition.status_immunities.iter().cloned());
                            if definition.tags.iter().any(|tag| tag == "resist-all") {
                                target_immunities.insert(status_kind_id.clone());
                            }
                        }
                        for status in &self.entities[target_index].statuses {
                            target_immunities
                                .extend(status.granted_status_immunities.iter().cloned());
                        }
                        self.entities[target_index].alerted = true;
                        changed.insert(self.entities[target_index].position);
                        apply_ability_status_effect(
                            &mut self.entities[target_index],
                            &ability.id,
                            0,
                            status_kind_id,
                            *intensity,
                            *duration_ticks,
                            *duration_dice,
                            *duration_sides,
                            *stacking,
                            *resistance_type,
                            *power,
                            granted_resistances,
                            granted_brands,
                            granted_modifiers,
                            granted_equipment_bonuses,
                            granted_status_immunities,
                            granted_race_id.as_deref(),
                            *grants_wall_passage,
                            *incoming_damage_percent,
                            target_level,
                            Some((&target_resistances, &target_immunities, None)),
                            &mut self.rng,
                        )
                    }
                    AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                        self.entities[target_index].alerted = true;
                        changed.insert(self.entities[target_index].position);
                        remove_ability_status_effect(
                            &mut self.entities[target_index],
                            0,
                            status_kind_id,
                        )
                    }
                    _ => unreachable!("actor status executor requires a status effect"),
                };
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: vec![resolution],
                    },
                    trace: Some(trace),
                });
            }
            _ => unreachable!("actor status effects require a self or projectile target plan"),
        }
    }

    fn resolve_crusade_arrest_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_index: usize,
        trace: ProjectileTrace,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::ApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            power: Some(power),
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id,
            grants_wall_passage,
            incoming_damage_percent,
            ..
        } = &ability.effect
        else {
            unreachable!("Arrest requires a powered status effect");
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("an Arrest target must have an actor definition");
        let target_level = definition.level;
        let eligible = !definition.tags.iter().any(|tag| tag == "unique")
            && actor_matches_category(definition, "evil");
        let threshold = eligible.then(|| {
            let sides = power.saturating_sub(10).max(1);
            u16::try_from(self.rng.bounded(u64::from(sides)) + 11)
                .expect("Arrest threshold must fit u16")
        });
        let succeeded = threshold.is_some_and(|roll| target_level <= u32::from(roll));
        let forced_resistances = ResistanceProfile::default();
        let forced_immunities = BTreeSet::from([status_kind_id.clone()]);
        let mut resolution = apply_ability_status_effect(
            &mut self.entities[target_index],
            &ability.id,
            0,
            status_kind_id,
            *intensity,
            *duration_ticks,
            *duration_dice,
            *duration_sides,
            *stacking,
            None,
            if succeeded { None } else { Some(*power) },
            granted_resistances,
            granted_brands,
            granted_modifiers,
            granted_equipment_bonuses,
            granted_status_immunities,
            granted_race_id.as_deref(),
            *grants_wall_passage,
            *incoming_damage_percent,
            succeeded.then_some(target_level),
            (!succeeded).then_some((&forced_resistances, &forced_immunities, None)),
            &mut self.rng,
        );
        if let AbilityEffectResolutionDto::ApplyStatus {
            power: resolved_power,
            target_level: resolved_target_level,
            power_roll,
            target_roll,
            change,
            ..
        } = &mut resolution
        {
            *resolved_power = Some(*power);
            *resolved_target_level = Some(target_level);
            *power_roll = threshold;
            *target_roll = None;
            if eligible && !succeeded {
                *change = AbilityStatusChangeDto::Resisted;
            }
        }
        self.entities[target_index].alerted = true;
        changed.insert(self.entities[target_index].position);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(in crate::game) fn resolve_player_visible_status_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            duration_dice,
            duration_sides,
            stacking,
            resistance_type,
            power,
            target_category,
        } = &ability.effect
        else {
            unreachable!("visible status executor requires a visible status effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                status_kind_id,
                *intensity,
                *duration_ticks,
                *duration_dice,
                *duration_sides,
                *stacking,
                *resistance_type,
                *power,
                &empty_resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                target_level,
                None,
                &mut self.rng,
            );
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(in crate::game) fn resolve_player_entangle_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Entangle {
            power,
            duration_ticks,
        } = ability.effect
        else {
            unreachable!("entangle executor requires an entangle effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && self.entity_is_visible_to_player(entity))
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let Some(definition) = self.content.actor(&target_kind_id) else {
                continue;
            };
            let target_level = definition.level;
            let unique = definition.tags.iter().any(|tag| tag == "unique");
            let contest_roll = (!unique).then(|| {
                let sides = power.saturating_sub(10).max(1);
                u16::try_from(self.rng.bounded(u64::from(sides)) + 1)
                    .expect("entangle roll must fit u16")
                    .saturating_add(10)
            });
            let rejected =
                unique || contest_roll.is_some_and(|roll| target_level > u32::from(roll));
            let mut resolution = if rejected {
                AbilityEffectResolutionDto::ApplyStatus {
                    effect_index: 0,
                    status_kind_id: STATUS_SLOW.to_owned(),
                    intensity: 1,
                    requested_duration_ticks: duration_ticks,
                    applied_duration_ticks: 0,
                    stacking: AbilityStatusStackingDto::Extend,
                    resistance_type: None,
                    resistance: None,
                    power: Some(power),
                    target_level: Some(target_level),
                    power_roll: contest_roll,
                    target_roll: contest_roll.map(|_| target_level),
                    granted_resistances: Vec::new(),
                    granted_brands: Vec::new(),
                    granted_race_id: None,
                    grants_wall_passage: false,
                    incoming_damage_percent: 100,
                    change: if unique {
                        AbilityStatusChangeDto::Immune
                    } else {
                        AbilityStatusChangeDto::Resisted
                    },
                }
            } else {
                apply_ability_status_effect(
                    &mut self.entities[index],
                    &ability.id,
                    0,
                    STATUS_SLOW,
                    1,
                    duration_ticks,
                    0,
                    0,
                    AbilityStatusStackingDefinition::Extend,
                    None,
                    None,
                    &empty_resistances,
                    &empty_brands,
                    &StatModifiers::default(),
                    &EquipmentBonuses::default(),
                    &empty_immunities,
                    None,
                    false,
                    100,
                    None,
                    None,
                    &mut self.rng,
                )
            };
            if let AbilityEffectResolutionDto::ApplyStatus {
                power: resolved_power,
                target_level: resolved_level,
                power_roll,
                target_roll,
                ..
            } = &mut resolution
            {
                *resolved_power = Some(power);
                *resolved_level = Some(target_level);
                *power_roll = contest_roll;
                *target_roll = contest_roll.map(|_| target_level);
            }
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(in crate::game) fn resolve_player_mass_sleep_or_stasis_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::MassSleepOrStasis { stasis, power, .. } = ability.effect
        else {
            unreachable!("mass sleep executor requires a mass sleep effect");
        };
        let (status_kind_id, duration_ticks, stacking) = if stasis {
            (
                STATUS_PARALYSIS,
                20,
                AbilityStatusStackingDefinition::Extend,
            )
        } else {
            (
                STATUS_SLEEP,
                500,
                AbilityStatusStackingDefinition::KeepStrongest,
            )
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && (!stasis
                        || self
                            .content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| {
                                !definition
                                    .tags
                                    .iter()
                                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
                            }))
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let mut resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                status_kind_id,
                1,
                duration_ticks,
                0,
                0,
                stacking,
                None,
                Some(power),
                &empty_resistances,
                &empty_brands,
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &empty_immunities,
                None,
                false,
                100,
                target_level,
                None,
                &mut self.rng,
            );
            if stasis
                && let AbilityEffectResolutionDto::ApplyStatus {
                    requested_duration_ticks,
                    applied_duration_ticks,
                    change,
                    ..
                } = &mut resolution
                && !matches!(
                    change,
                    AbilityStatusChangeDto::Immune | AbilityStatusChangeDto::Resisted
                )
                && self.rng.bounded(15) == 0
            {
                *requested_duration_ticks += 10;
                *applied_duration_ticks += 10;
                if let Some(status) = self.entities[index]
                    .statuses
                    .iter_mut()
                    .find(|status| status.kind_id == STATUS_PARALYSIS)
                {
                    status.remaining_ticks = status.remaining_ticks.saturating_add(10);
                }
            }
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(super) fn resolve_player_sanctuary_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Sanctuary { power, radius } = ability.effect else {
            unreachable!("sanctuary executor requires a sanctuary effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && rfb_distance(self.player.position, entity.position) <= u32::from(radius)
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let resistance_profile = self.entities[index].resistances.clone();
            let unique = self
                .actor_runtime_definition(&self.entities[index])
                .is_some_and(|definition| {
                    definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
                });
            let status_immunities = if unique || self.actor_has_status_immunity(index, STATUS_SLEEP)
            {
                BTreeSet::from([STATUS_SLEEP.to_owned()])
            } else {
                BTreeSet::new()
            };
            let resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                STATUS_SLEEP,
                1,
                500,
                0,
                0,
                AbilityStatusStackingDefinition::KeepStrongest,
                None,
                Some(power),
                &BTreeMap::new(),
                &BTreeSet::new(),
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &BTreeSet::new(),
                None,
                false,
                100,
                target_level,
                Some((&resistance_profile, &status_immunities, None)),
                &mut self.rng,
            );
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
    }

    pub(super) fn resolve_player_turn_undead_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::TurnUndead { power } = ability.effect else {
            unreachable!("turn undead executor requires a turn undead effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && self
                        .content
                        .actor(&entity.kind_id)
                        .is_some_and(|definition| actor_matches_category(definition, "undead"))
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let mut affected = false;
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                STATUS_FEAR,
                1,
                1,
                3,
                u32::from((power / 2).max(1)),
                AbilityStatusStackingDefinition::Extend,
                None,
                Some(power),
                &BTreeMap::new(),
                &BTreeSet::new(),
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &BTreeSet::new(),
                None,
                false,
                100,
                target_level,
                None,
                &mut self.rng,
            );
            affected |= matches!(
                &resolution,
                AbilityEffectResolutionDto::ApplyStatus {
                    change: AbilityStatusChangeDto::Added
                        | AbilityStatusChangeDto::Replaced
                        | AbilityStatusChangeDto::Extended
                        | AbilityStatusChangeDto::Strengthened,
                    ..
                }
            );
            changed.insert(self.entities[index].position);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![resolution],
                },
                trace: None,
            });
        }
        if affected {
            self.add_virtue(VirtueKindDto::Unlife, -1);
        }
    }

    pub(super) fn resolve_player_banish_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Banish { maximum_distance } = ability.effect else {
            unreachable!("banish executor requires a banish effect");
        };
        let mut actor_ids = self.item_visible_actor_ids();
        actor_ids.sort();
        let before = actor_ids
            .iter()
            .filter_map(|actor_id| {
                self.entities
                    .iter()
                    .find(|entity| entity.id == *actor_id && entity.hp > 0)
                    .map(|entity| (actor_id.clone(), entity.position))
            })
            .collect::<Vec<_>>();
        let outcomes = self.banish_visible_actors(maximum_distance, actor_ids, changed);
        let targets = before
            .into_iter()
            .zip(outcomes)
            .map(|((entity_id, from), outcome)| match outcome {
                VisibleBanishmentOutcome::Resisted { target_kind_id } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: true,
                    from,
                    to: None,
                },
                VisibleBanishmentOutcome::NoSpace { target_kind_id } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: false,
                    from,
                    to: None,
                },
                VisibleBanishmentOutcome::Banished {
                    target_kind_id,
                    resolution,
                } => AbilityBanishTargetDto {
                    entity_id,
                    target_kind_id,
                    resisted: false,
                    from,
                    to: Some(resolution.to),
                },
            })
            .collect();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::Banish {
                    effect_index: 0,
                    maximum_distance,
                    targets,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_suppress_reproduction_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::SuppressMonsterReproduction {
            damage_dice,
            damage_sides,
            damage_bonus,
        } = ability.effect
        else {
            unreachable!("reproduction suppression executor requires its matching effect");
        };
        let damage = self
            .roll_damage(damage_dice, damage_sides)
            .saturating_add(i32::from(damage_bonus));
        let damage = self
            .apply_final_player_damage(
                resolve_damage(
                    DamagePacket::new(damage, DamageType::Physical),
                    ResistanceLevel::Normal,
                ),
                FatalityPolicy::BelowZero,
            )
            .damage
            .applied;
        let already_suppressed = self.reproduction_suppressed;
        self.reproduction_suppressed = true;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::SuppressMonsterReproduction {
                    effect_index: 0,
                    damage,
                    fatal: self.player_is_dead(),
                    already_suppressed,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_polymorph_target_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace: trace.clone(),
            });
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::Skipped {
                        effect_index: 0,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
                    }],
                },
                trace: Some(trace),
            });
            return;
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let resolution = self.resolve_actor_polymorph_target(
            target_index,
            u32::from(self.progress.level),
            0,
            events,
            changed,
        );
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![resolution],
            },
            trace: Some(trace),
        });
    }

    pub(super) fn resolve_player_aggravate_monsters_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (awakened, hastened, _) = self.aggravate_monsters(None, &ability.id, changed);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::AggravateMonsters {
                    effect_index: 0,
                    awakened,
                    hastened,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_banish_evil_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::BanishEvil = ability.effect else {
            unreachable!("banish-evil executor requires a banish-evil effect");
        };
        let power =
            spell_power_value(100, ability.spell_power_bonus).min(u64::from(u16::MAX)) as u16;
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.entity_is_visible_to_player(entity)
                    && self
                        .actor_runtime_definition(entity)
                        .is_some_and(|definition| actor_matches_category(definition, "evil"))
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let mut resolutions = Vec::with_capacity(target_ids.len());
        for target_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id && entity.hp > 0)
            else {
                continue;
            };
            resolutions.push(self.resolve_teleport_away_target(index, 0, 0, power, changed));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: resolutions,
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_genocide_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Option<Vec<Position>>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let AbilityEffectDefinition::Genocide {
            scope,
            power,
            radius,
            target_category,
            fatigue,
            unlife_change_on_success,
            chance_change_on_success,
        } = &ability.effect
        else {
            unreachable!("genocide executor requires a genocide effect");
        };
        let (trace, target_entity_id, target_kind_id, glyph) =
            if *scope == AbilityGenocideScopeDefinition::Nearby {
                (None, None, None, None)
            } else {
                let (trace, target_index) =
                    self.trace_projectile_path(path.expect("targeted genocide must retain a path"));
                let Some(target_index) = target_index else {
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: vec![AbilityEffectResolutionDto::Skipped {
                                effect_index: 0,
                                reason: AbilityEffectSkipReasonDto::NoTarget,
                            }],
                        },
                        trace: Some(trace),
                    });
                    return;
                };
                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let glyph = self
                    .content
                    .actor(&target_kind_id)
                    .map(|definition| definition.glyph.clone());
                (
                    Some(trace),
                    Some(target_entity_id),
                    Some(target_kind_id),
                    glyph,
                )
            };
        let mut candidate_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
                    && match scope {
                        AbilityGenocideScopeDefinition::Single => {
                            target_entity_id.as_deref() == Some(entity.id.as_str())
                        }
                        AbilityGenocideScopeDefinition::Glyph => self
                            .content
                            .actor(&entity.kind_id)
                            .zip(glyph.as_ref())
                            .is_some_and(|(definition, glyph)| &definition.glyph == glyph),
                        AbilityGenocideScopeDefinition::Nearby => {
                            chebyshev_distance(self.player.position, entity.position)
                                <= u32::from(*radius)
                        }
                    }
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        candidate_ids.sort();
        let resolution = self.resolve_genocide_candidates(
            candidate_ids,
            *scope,
            *power,
            *fatigue,
            changed,
            removed_entities,
        );
        if !resolution.removed_entity_ids.is_empty() {
            self.add_virtue(VirtueKindDto::Unlife, i16::from(*unlife_change_on_success));
            self.add_virtue(VirtueKindDto::Chance, i16::from(*chance_change_on_success));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id,
                target_kind_id,
                effects: vec![AbilityEffectResolutionDto::Genocide {
                    effect_index: 0,
                    scope: ability_genocide_scope_dto(*scope),
                    power: *power,
                    radius: *radius,
                    glyph: matches!(scope, AbilityGenocideScopeDefinition::Glyph)
                        .then_some(glyph)
                        .flatten(),
                    removed_entity_ids: resolution.removed_entity_ids,
                    resisted_entity_ids: resolution.resisted_entity_ids,
                    fatigue_damage: resolution.fatigue_damage,
                }],
            },
            trace,
        });
    }
}
