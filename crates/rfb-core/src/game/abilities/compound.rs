// SPDX-License-Identifier: MPL-2.0

use crate::effect::{
    STATUS_CONFUSION, STATUS_FEAR, STATUS_HASTE, STATUS_PARALYSIS, STATUS_PROTECTION_FROM_EVIL,
    STATUS_SLOW, STATUS_STUN,
};
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::abilities::AbilityTargetPlan;
use crate::game::abilities::terrain::EarthquakeSource;
use crate::game::ability_scaling::{spell_power_value, spell_powered_ability_value};
use crate::game::status_effects::{
    ability_status_stacking_dto, apply_ability_status_effect, remove_ability_status_effect,
};
use crate::game::{CategorySummonSpec, Game, actor_matches_category};
use crate::resistance::DamageType;
use crate::stats::AttributeKind;
use rfb_content::{
    AbilityDefinition, AbilityEffectDefinition, AbilitySpellPowerField,
    AbilityStatusStackingDefinition, ActorDamageType, EquipmentBonuses, StatModifiers,
};
use rfb_protocol::{
    AbilityControlOutcomeDto, AbilityEffectResolutionDto, AbilityEffectSkipReasonDto,
    AbilityEffectsResolutionDto, AbilityStatusChangeDto, AbilitySummonResolutionDto, Direction,
    HealingResolutionDto, MonsterPackRoleDto, Position, TargetSelection,
};
use std::collections::{BTreeMap, BTreeSet};

impl Game {
    pub(in crate::game) fn resolve_player_ordered_sequence_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Sequence { effects } = &ability.effect else {
            unreachable!("ordered sequence executor requires a sequence effect");
        };
        if matches!(target_plan, AbilityTargetPlan::SelfTarget)
            && effects.iter().any(|effect| {
                !matches!(
                    effect,
                    AbilityEffectDefinition::Heal { .. }
                        | AbilityEffectDefinition::HealDice { .. }
                        | AbilityEffectDefinition::ReduceStatus { .. }
                        | AbilityEffectDefinition::ApplyStatus { .. }
                        | AbilityEffectDefinition::RemoveStatus { .. }
                )
            })
        {
            for (index, effect) in effects.iter().enumerate() {
                let mut step = ability.clone();
                step.effect = effect.clone();
                let effect_index =
                    u8::try_from(index).expect("validated ability effect index must fit u8");
                step.spell_power_fields
                    .retain(|definition| definition.effect_index == effect_index);
                for definition in &mut step.spell_power_fields {
                    definition.effect_index = 0;
                }
                let plan = match effect {
                    AbilityEffectDefinition::AreaDamage { .. } => AbilityTargetPlan::Projectile {
                        path: Vec::new(),
                        stop_at_actor: false,
                    },
                    AbilityEffectDefinition::Detect { .. } => AbilityTargetPlan::Detect,
                    AbilityEffectDefinition::Heal { .. }
                    | AbilityEffectDefinition::HealDice { .. }
                    | AbilityEffectDefinition::ReduceStatus { .. }
                    | AbilityEffectDefinition::RestoreVitality { .. }
                    | AbilityEffectDefinition::LightArea { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::VisibleDamage { .. }
                    | AbilityEffectDefinition::VisibleApplyStatus { .. }
                    | AbilityEffectDefinition::AggravateMonsters
                    | AbilityEffectDefinition::CallSunlight { .. }
                    | AbilityEffectDefinition::CreateCurrentTerrain { .. }
                    | AbilityEffectDefinition::NoOp { .. } => AbilityTargetPlan::SelfTarget,
                    AbilityEffectDefinition::CreateAdjacentTerrain {
                        source_terrain_ids,
                        target_terrain_id,
                    } => AbilityTargetPlan::AdjacentTerrain {
                        replacements: self.adjacent_terrain_creation_replacements(
                            source_terrain_ids,
                            target_terrain_id,
                        ),
                    },
                    _ => unreachable!("validated self sequence must remain self-targeted"),
                };
                self.resolve_player_ability_effect(step, plan, events, changed, removed_entities)?;
            }
            return Ok(());
        }
        match target_plan {
            AbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.player.id.clone();
                let target_kind_id = self.player.kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let resolution = match effect {
                        AbilityEffectDefinition::Heal { amount } => {
                            let amount = i32::try_from(*amount)
                                .expect("validated healing amount must fit i32");
                            let outcome = self.apply_player_healing(amount);
                            AbilityEffectResolutionDto::Heal {
                                effect_index,
                                resolution: HealingResolutionDto {
                                    requested: outcome.requested,
                                    applied: outcome.applied,
                                },
                            }
                        }
                        AbilityEffectDefinition::HealDice { dice, sides } => {
                            let amount = self.roll_damage(*dice, *sides).max(0);
                            let amount = i32::try_from(spell_powered_ability_value(
                                ability,
                                effect_index,
                                AbilitySpellPowerField::FinalHealing,
                                u64::try_from(amount).expect("healing must be non-negative"),
                            ))
                            .expect("spell-powered healing must fit i32");
                            let outcome = self.apply_player_healing(amount);
                            AbilityEffectResolutionDto::Heal {
                                effect_index,
                                resolution: HealingResolutionDto {
                                    requested: outcome.requested,
                                    applied: outcome.applied,
                                },
                            }
                        }
                        AbilityEffectDefinition::ReduceStatus {
                            status_kind_id,
                            amount,
                            current_divisor,
                            remaining_divisor,
                        } => {
                            let (before, after) = self.reduce_player_status(
                                status_kind_id,
                                *amount,
                                *current_divisor,
                                *remaining_divisor,
                            );
                            AbilityEffectResolutionDto::ReduceStatus {
                                effect_index,
                                status_kind_id: status_kind_id.clone(),
                                before,
                                after,
                            }
                        }
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
                            effect_index,
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
                            remove_ability_status_effect(
                                &mut self.player,
                                effect_index,
                                status_kind_id,
                            )
                        }
                        _ => unreachable!(
                            "validated self-target effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: None,
                });
                self.refresh_player_resource_maxima();
            }
            AbilityTargetPlan::Projectile { path, .. } => {
                let (trace, target_index) = self.trace_projectile_path(path);
                let Some(target_index) = target_index else {
                    let resolutions = effects
                        .iter()
                        .enumerate()
                        .map(|(index, _)| AbilityEffectResolutionDto::Skipped {
                            effect_index: u8::try_from(index)
                                .expect("validated ability effect index must fit u8"),
                            reason: AbilityEffectSkipReasonDto::NoTarget,
                        })
                        .collect();
                    events.push(DomainEvent::AbilityLanded {
                        ability_id: ability.id.clone(),
                        trace: trace.clone(),
                    });
                    events.push(DomainEvent::AbilityEffectsResolved {
                        ability_id: ability.id.clone(),
                        resolution: AbilityEffectsResolutionDto {
                            target_entity_id: None,
                            target_kind_id: None,
                            effects: resolutions,
                        },
                        trace: Some(trace),
                    });
                    return Ok(());
                };

                let target_entity_id = self.entities[target_index].id.clone();
                let target_kind_id = self.entities[target_index].kind_id.clone();
                let mut resolutions = Vec::with_capacity(effects.len());
                for (index, effect) in effects.iter().enumerate() {
                    let effect_index =
                        u8::try_from(index).expect("validated ability effect index must fit u8");
                    let Some(current_index) = self
                        .entities
                        .iter()
                        .position(|entity| entity.id == target_entity_id && entity.hp > 0)
                    else {
                        resolutions.push(AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::TargetDead,
                        });
                        continue;
                    };
                    let resolution = match effect {
                        AbilityEffectDefinition::Damage {
                            damage_dice,
                            damage_sides,
                            damage_bonus,
                            damage_type,
                        } => {
                            let raw_damage = self
                                .roll_damage(*damage_dice, *damage_sides)
                                .saturating_add(i32::from(*damage_bonus))
                                .max(0);
                            let damage = self.resolve_ability_damage_to_entity(
                                current_index,
                                &ability.id,
                                DamageType::from(*damage_type),
                                raw_damage,
                                trace.clone(),
                                events,
                                changed,
                                removed_entities,
                            )?;
                            AbilityEffectResolutionDto::Damage {
                                effect_index,
                                resolution: damage.into(),
                            }
                        }
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
                                .actor(&self.entities[current_index].kind_id)
                                .map(|definition| definition.level);
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            apply_ability_status_effect(
                                &mut self.entities[current_index],
                                &ability.id,
                                effect_index,
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
                                None,
                                &mut self.rng,
                            )
                        }
                        AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            remove_ability_status_effect(
                                &mut self.entities[current_index],
                                effect_index,
                                status_kind_id,
                            )
                        }
                        AbilityEffectDefinition::Control { category, power } => {
                            self.entities[current_index].alerted = true;
                            changed.insert(self.entities[current_index].position);
                            self.resolve_ability_control(
                                current_index,
                                effect_index,
                                category,
                                *power,
                            )
                        }
                        _ => unreachable!(
                            "validated projectile effect sequences contain only actor effects"
                        ),
                    };
                    resolutions.push(resolution);
                }
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: resolutions,
                    },
                    trace: Some(trace),
                });
            }
            _ => unreachable!("effect sequences require a self or projectile target plan"),
        }
        Ok(())
    }

    pub(in crate::game) fn resolve_player_no_op_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::NoOp { reason } = &ability.effect else {
            unreachable!("no-op executor requires a no-op effect");
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::NoOp {
                    effect_index: 0,
                    reason: reason.clone(),
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_melee_then_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_entity_id: &str,
        teleport_candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::MeleeThenTeleport {
            failure_threshold, ..
        } = ability.effect
        else {
            unreachable!("panic melee executor requires a melee-then-teleport effect");
        };
        let index = self
            .entities
            .iter()
            .position(|entity| entity.id == target_entity_id)
            .expect("planned panic-hit target must remain available");
        let target_kind_id = self.entities[index].kind_id.clone();
        let player_from = self.player.position;
        self.resolve_player_melee(index, false, events, changed, removed_entities)?;
        let skill =
            u64::try_from(self.player_derived_stats().disarm_skill.value.max(1)).unwrap_or(1);
        let teleport_attempted = self.rng.bounded(skill) >= u64::from(failure_threshold);
        let candidates = teleport_candidates
            .into_iter()
            .filter(|position| {
                self.is_walkable(*position)
                    && self
                        .entities
                        .iter()
                        .all(|entity| entity.position != *position)
            })
            .collect::<Vec<_>>();
        let teleported = teleport_attempted && !candidates.is_empty() && !self.player_is_dead();
        if teleported {
            let destination_index = usize::try_from(
                self.rng
                    .bounded(u64::try_from(candidates.len()).unwrap_or(u64::MAX)),
            )
            .expect("panic teleport candidate index must fit usize");
            self.resolve_player_teleport_effect(
                ability,
                candidates[destination_index],
                events,
                changed,
            );
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id.to_owned()),
                target_kind_id: Some(target_kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::MeleeThenTeleport {
                    effect_index: 0,
                    target_entity_id: target_entity_id.to_owned(),
                    target_kind_id,
                    player_from,
                    player_to: self.player.position,
                    teleport_attempted,
                    teleported,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_nature_wrath_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let roll =
            u8::try_from(self.rng.bounded(6) + 1).expect("Nature's Wrath branch roll must fit u8");
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RandomChoice {
                    effect_index: 0,
                    roll: i32::from(roll),
                    branch_index: u16::from(roll - 1),
                    maximum_roll: 6,
                }],
            },
            trace: None,
        });
        if matches!(roll, 2 | 6) {
            return Ok(());
        }
        self.resolve_nature_wrath_branch(ability, roll, None, events, changed, removed_entities)
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn resolve_nature_wrath_branch(
        &mut self,
        ability: &AbilityDefinition,
        roll: u8,
        direction: Option<Direction>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let level = self.progress.level;
        let spell_damage_bonus = self.casting_profile().map_or(0, |profile| {
            profile.spell_damage_bonus_base.saturating_add(
                profile
                    .spell_damage_bonus_per_level
                    .saturating_mul(level / u16::from(profile.spell_damage_bonus_level_divisor)),
            )
        });
        let powered = |value: u64| spell_power_value(value, ability.spell_power_bonus);
        let powered_damage =
            |value: u64| i32::try_from(powered(value)).expect("Nature's Wrath damage must fit i32");
        let powered_radius =
            |value: u64| u8::try_from(powered(value)).expect("Nature's Wrath radius must fit u8");

        match roll {
            1 => {
                self.resolve_player_visible_damage_with_base(
                    &ability.id,
                    DamageType::Physical,
                    None,
                    powered_damage(
                        u64::from(level)
                            .saturating_mul(4)
                            .saturating_add(u64::from(spell_damage_bonus)),
                    ),
                    events,
                    changed,
                    removed_entities,
                )?;
                self.resolve_earthquake(
                    self.player.position,
                    powered_radius(20 + u64::from(level / 2)),
                    15,
                    "demo.terrain.floor",
                    &[
                        "demo.terrain.wall".to_owned(),
                        "demo.terrain.quartz-vein".to_owned(),
                        "demo.terrain.magma-vein".to_owned(),
                    ],
                    EarthquakeSource::Ability(ability.id.clone()),
                    events,
                    changed,
                    removed_entities,
                )?;
                self.resolve_player_area_damage_with_base_policy(
                    &ability.id,
                    Vec::new(),
                    false,
                    DamageType::Disintegrate,
                    powered_radius(1 + u64::from(level / 12)),
                    None,
                    powered_damage(
                        (100 + u64::from(level) + u64::from(spell_damage_bonus)).saturating_mul(2),
                    ),
                    true,
                    false,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            2 => {
                let direction = direction.expect("Nature's Wrath lightning requires a direction");
                let path = self
                    .projectile_path(
                        &TargetSelection::Direction { direction },
                        self.width.max(self.height),
                    )
                    .expect("directional Nature's Wrath must produce a path");
                self.resolve_player_projectile_damage_with_base(
                    &ability.id,
                    path,
                    DamageType::Electricity,
                    powered_damage(
                        u64::from(level)
                            .saturating_mul(8)
                            .saturating_add(u64::from(spell_damage_bonus)),
                    ),
                    false,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            3 => {
                self.resolve_player_visible_damage_with_base(
                    &ability.id,
                    DamageType::Sound,
                    None,
                    powered_damage(
                        u64::from(level)
                            .saturating_mul(5)
                            .saturating_add(u64::from(spell_damage_bonus)),
                    ),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            4 => {
                self.resolve_player_visible_damage_with_base(
                    &ability.id,
                    DamageType::Gravity,
                    None,
                    powered_damage(
                        u64::from(level)
                            .saturating_mul(4)
                            .saturating_add(u64::from(spell_damage_bonus)),
                    ),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            5 => {
                let radius = powered_radius(1 + u64::from(level / 12));
                let damage = powered_damage(
                    (120 + u64::from(level) + u64::from(spell_damage_bonus)).saturating_mul(2),
                );
                for damage_type in [DamageType::Fire, DamageType::Cold, DamageType::Electricity] {
                    self.resolve_player_area_damage_with_base(
                        &ability.id,
                        Vec::new(),
                        false,
                        damage_type,
                        radius,
                        None,
                        damage,
                        true,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
            6 => {
                let direction = direction.expect("Nature's Wrath shards require a direction");
                let path = self
                    .projectile_path(
                        &TargetSelection::Direction { direction },
                        self.width.max(self.height),
                    )
                    .expect("directional Nature's Wrath must produce a path");
                let damage = powered_damage(70 + u64::from(level) + u64::from(spell_damage_bonus));
                for _ in 0..3 {
                    self.resolve_player_area_damage_with_base(
                        &ability.id,
                        path.clone(),
                        true,
                        DamageType::Shards,
                        1,
                        None,
                        damage,
                        true,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
            }
            _ => unreachable!("validated Nature's Wrath roll must be in 1..=6"),
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_crusade_actor_status(
        &mut self,
        index: usize,
        ability_id: &str,
        status_kind_id: &str,
        duration_ticks: u32,
        stacking: AbilityStatusStackingDefinition,
        power: u16,
        power_roll: Option<u16>,
        target_roll: Option<u32>,
        rejection: Option<AbilityStatusChangeDto>,
    ) -> AbilityEffectResolutionDto {
        let target_level = self
            .actor_runtime_definition(&self.entities[index])
            .map_or(0, |definition| definition.level);
        if let Some(change) = rejection {
            return AbilityEffectResolutionDto::ApplyStatus {
                effect_index: 0,
                status_kind_id: status_kind_id.to_owned(),
                intensity: 1,
                requested_duration_ticks: duration_ticks,
                applied_duration_ticks: 0,
                stacking: ability_status_stacking_dto(stacking),
                resistance_type: None,
                resistance: None,
                power: Some(power),
                target_level: Some(target_level),
                power_roll,
                target_roll,
                granted_resistances: Vec::new(),
                granted_brands: Vec::new(),
                granted_race_id: None,
                grants_wall_passage: false,
                incoming_damage_percent: 100,
                change,
            };
        }
        let mut resolution = apply_ability_status_effect(
            &mut self.entities[index],
            ability_id,
            0,
            status_kind_id,
            1,
            duration_ticks,
            0,
            0,
            stacking,
            None,
            None,
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        if let AbilityEffectResolutionDto::ApplyStatus {
            power: resolved_power,
            target_level: resolved_level,
            power_roll: resolved_power_roll,
            target_roll: resolved_target_roll,
            ..
        } = &mut resolution
        {
            *resolved_power = Some(power);
            *resolved_level = Some(target_level);
            *resolved_power_roll = power_roll;
            *resolved_target_roll = target_roll;
        }
        resolution
    }

    fn push_actor_effect_resolution(
        &self,
        ability_id: &str,
        index: usize,
        resolution: AbilityEffectResolutionDto,
        events: &mut Vec<DomainEvent>,
    ) {
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability_id.to_owned(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.entities[index].id.clone()),
                target_kind_id: Some(self.entities[index].kind_id.clone()),
                effects: vec![resolution],
            },
            trace: None,
        });
    }

    fn divine_intervention_targets(&self) -> Vec<String> {
        self.entities
            .iter()
            .filter(|entity| entity.hp > 0 && self.entity_is_visible_to_player(entity))
            .map(|entity| entity.id.clone())
            .collect()
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn resolve_player_divine_intervention_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DivineIntervention = ability.effect else {
            unreachable!("Divine Intervention executor requires its dedicated effect");
        };
        let level = self.progress.level;
        let powered = |value: u64| spell_power_value(value, ability.spell_power_bonus);
        let adjacent_damage = i32::try_from(powered(u64::from(level).saturating_mul(11)))
            .expect("Divine Intervention adjacent damage must fit i32");
        let visible_damage = i32::try_from(powered(
            u64::from(level)
                .saturating_mul(4)
                .saturating_add(u64::from(self.casting_spell_damage_bonus())),
        ))
        .expect("Divine Intervention visible damage must fit i32");
        let healing =
            i32::try_from(powered(100)).expect("Divine Intervention healing must fit i32");
        let power = powered(u64::from(level).saturating_mul(4)).min(u64::from(u16::MAX)) as u16;

        self.resolve_player_area_damage_with_base_policy(
            &ability.id,
            Vec::new(),
            false,
            DamageType::HolyFire,
            1,
            None,
            adjacent_damage,
            false,
            false,
            events,
            changed,
            removed_entities,
        )?;
        self.resolve_player_visible_damage_with_base(
            &ability.id,
            DamageType::Mana,
            None,
            visible_damage,
            events,
            changed,
            removed_entities,
        )?;

        for target_id in self.divine_intervention_targets() {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("Divine Intervention target definition must remain available");
            let unique = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
            let target_level = definition.level;
            let threshold = (!unique).then(|| {
                let sides = power.saturating_sub(10).max(1);
                u16::try_from(self.rng.bounded(u64::from(sides)) + 11)
                    .expect("slow threshold must fit u16")
            });
            let resisted = unique || threshold.is_some_and(|roll| target_level > u32::from(roll));
            let resolution = self.apply_crusade_actor_status(
                index,
                &ability.id,
                STATUS_SLOW,
                50,
                AbilityStatusStackingDefinition::Extend,
                power,
                threshold,
                threshold.map(u32::from),
                resisted.then_some(if unique {
                    AbilityStatusChangeDto::Immune
                } else {
                    AbilityStatusChangeDto::Resisted
                }),
            );
            changed.insert(self.entities[index].position);
            self.push_actor_effect_resolution(&ability.id, index, resolution, events);
        }

        for target_id in self.divine_intervention_targets() {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
            else {
                continue;
            };
            let immune = self.actor_has_status_immunity(index, STATUS_STUN);
            let resolution = self.apply_crusade_actor_status(
                index,
                &ability.id,
                STATUS_STUN,
                u32::from(5_u16.saturating_add(level / 5)),
                AbilityStatusStackingDefinition::Extend,
                power,
                None,
                None,
                immune.then_some(AbilityStatusChangeDto::Immune),
            );
            changed.insert(self.entities[index].position);
            self.push_actor_effect_resolution(&ability.id, index, resolution, events);
        }

        for target_id in self.divine_intervention_targets() {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("Divine Intervention target definition must remain available");
            let unique_bonus = u32::from(
                definition
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "unique" | "unique2")),
            ) * 3;
            let target_level = definition.level.saturating_add(unique_bonus);
            let confusion_power =
                u16::try_from(u32::from(level / 2).max(u32::from(power.min(100))))
                    .expect("confusion power must fit u16");
            let duration_sides = u64::from((confusion_power / 2).max(1));
            let duration = (0..3).fold(1_u32, |total, _| {
                total.saturating_add(
                    u32::try_from(self.rng.bounded(duration_sides) + 1)
                        .expect("confusion duration must fit u32"),
                )
            });
            let immune = self.actor_has_status_immunity(index, STATUS_CONFUSION);
            let power_roll = (!immune).then(|| {
                u16::try_from(self.rng.bounded(u64::from(confusion_power.max(1))) + 1)
                    .expect("confusion power roll must fit u16")
            });
            let target_roll = power_roll.map(|_| {
                u32::try_from(self.rng.bounded(u64::from(target_level.max(1))) + 1)
                    .expect("confusion target roll must fit u32")
            });
            let resisted = power_roll
                .zip(target_roll)
                .is_some_and(|(left, right)| right >= u32::from(left));
            let resolution = self.apply_crusade_actor_status(
                index,
                &ability.id,
                STATUS_CONFUSION,
                duration,
                AbilityStatusStackingDefinition::Extend,
                confusion_power,
                power_roll,
                target_roll,
                if immune {
                    Some(AbilityStatusChangeDto::Immune)
                } else if resisted {
                    Some(AbilityStatusChangeDto::Resisted)
                } else {
                    None
                },
            );
            changed.insert(self.entities[index].position);
            self.push_actor_effect_resolution(&ability.id, index, resolution, events);
        }

        const CHARISMA_SAVE_ADJUSTMENT: [i32; 38] = [
            -25, -15, -10, -7, -6, -5, -4, -3, -2, -2, -1, -1, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8,
            9, 10, 12, 14, 16, 18, 20, 23, 26, 29, 33, 37, 42, 50,
        ];
        let charisma_index = usize::from(
            self.effective_player_attributes()
                .index(AttributeKind::Charisma),
        )
        .min(CHARISMA_SAVE_ADJUSTMENT.len() - 1);
        let fear_power = i32::from(level)
            .saturating_add(CHARISMA_SAVE_ADJUSTMENT[charisma_index])
            .max(1) as u16;
        for target_id in self.divine_intervention_targets() {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("Divine Intervention target definition must remain available");
            let target_level = definition.level;
            let resist_all = definition.tags.iter().any(|tag| tag == "resist-all");
            let duration_sides = u64::from((power / 2).max(1));
            let duration = (0..3).fold(1_u32, |total, _| {
                total.saturating_add(
                    u32::try_from(self.rng.bounded(duration_sides) + 1)
                        .expect("fear duration must fit u32"),
                )
            });
            let immune = resist_all || self.actor_has_status_immunity(index, STATUS_FEAR);
            let power_roll = (!immune && target_level > 1).then(|| {
                u16::try_from(self.rng.bounded(u64::from(fear_power)) + 1)
                    .expect("fear power roll must fit u16")
            });
            let target_roll = power_roll.map(|_| {
                u32::try_from(self.rng.bounded(u64::from(target_level.max(1))) + 1)
                    .expect("fear target roll must fit u32")
            });
            let resisted = target_level <= 1
                || power_roll
                    .zip(target_roll)
                    .is_some_and(|(left, right)| u32::from(left) <= right);
            let resolution = self.apply_crusade_actor_status(
                index,
                &ability.id,
                STATUS_FEAR,
                duration,
                AbilityStatusStackingDefinition::Extend,
                power,
                power_roll,
                target_roll,
                if immune {
                    Some(AbilityStatusChangeDto::Immune)
                } else if resisted {
                    Some(AbilityStatusChangeDto::Resisted)
                } else {
                    None
                },
            );
            changed.insert(self.entities[index].position);
            self.push_actor_effect_resolution(&ability.id, index, resolution, events);
        }

        for target_id in self.divine_intervention_targets() {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("Divine Intervention target definition must remain available");
            let unique = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
            let target_level = definition.level;
            let stasis_power = (power / 3).max(1);
            let threshold = (!unique).then(|| {
                u16::try_from(self.rng.bounded(u64::from(stasis_power)) + 1)
                    .expect("stasis threshold must fit u16")
            });
            let resisted = unique || threshold.is_some_and(|roll| target_level > u32::from(roll));
            let duration = 2 + u32::from(!resisted && self.rng.bounded(15) == 0);
            let resolution = self.apply_crusade_actor_status(
                index,
                &ability.id,
                STATUS_PARALYSIS,
                duration,
                AbilityStatusStackingDefinition::Extend,
                stasis_power,
                threshold,
                threshold.map(u32::from),
                resisted.then_some(if unique {
                    AbilityStatusChangeDto::Immune
                } else {
                    AbilityStatusChangeDto::Resisted
                }),
            );
            changed.insert(self.entities[index].position);
            self.push_actor_effect_resolution(&ability.id, index, resolution, events);
        }

        let outcome = self.apply_player_healing(healing);
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Heal {
                    effect_index: 0,
                    resolution: HealingResolutionDto {
                        requested: outcome.requested,
                        applied: outcome.applied,
                    },
                }],
            },
            trace: None,
        });
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn resolve_player_crusade_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Crusade = ability.effect else {
            unreachable!("Crusade executor requires its dedicated effect");
        };
        let level = self.progress.level;
        let base_power = level.saturating_mul(4);
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| entity.hp > 0 && self.entity_is_visible_to_player(entity))
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        for target_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_id && entity.hp > 0)
            else {
                continue;
            };
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("Crusade target definition must remain available")
                .clone();
            let good = actor_matches_category(&definition, "good");
            let no_confusion = self.actor_has_status_immunity(index, STATUS_CONFUSION);
            let power = if no_confusion {
                base_power.saturating_sub(50).max(1)
            } else {
                base_power
            };
            let already_controlled =
                self.entities[index].controller_id.as_deref() == Some(self.player.id.as_str());
            let forbidden = self.entities[index].no_pet
                || definition.tags.iter().any(|tag| {
                    matches!(
                        tag.as_str(),
                        "unique" | "unique2" | "questor" | "guardian" | "no-pet"
                    )
                });
            let roll = (good && !already_controlled && !forbidden).then(|| {
                u16::try_from(self.rng.bounded(u64::from(power.max(1))) + 1)
                    .expect("Crusade charm roll must fit u16")
            });
            let controlled = already_controlled
                || (good
                    && !forbidden
                    && roll.is_some_and(|roll| {
                        definition.level.saturating_add(10) <= u32::from(roll)
                    }));
            if controlled && !already_controlled {
                let pack = self.entities[index].pack.clone();
                if let Some(pack) = pack {
                    if pack.role == MonsterPackRoleDto::Leader || pack.leader_id == target_id {
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
                        self.entities[index].pack = None;
                    }
                }
                self.entities[index].controller_id = Some(self.player.id.clone());
            }
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
            let outcome = if already_controlled {
                AbilityControlOutcomeDto::AlreadyControlled
            } else if !good {
                AbilityControlOutcomeDto::Ineligible
            } else if controlled {
                AbilityControlOutcomeDto::Controlled
            } else {
                AbilityControlOutcomeDto::Resisted
            };
            let mut effects = vec![AbilityEffectResolutionDto::Control {
                effect_index: 0,
                category: "good".to_owned(),
                power,
                target_entity_id: target_id.clone(),
                target_kind_id: definition.id.clone(),
                target_level: definition.level,
                roll,
                outcome,
            }];
            if controlled {
                effects.push(self.apply_crusade_actor_status(
                    index,
                    &ability.id,
                    STATUS_HASTE,
                    100,
                    AbilityStatusStackingDefinition::Extend,
                    power,
                    None,
                    None,
                    None,
                ));
            } else {
                let duration = 10_u32.saturating_add(
                    u32::try_from(self.rng.bounded(90) + 1)
                        .expect("Crusade fear duration must fit u32"),
                );
                let immune = self.actor_has_status_immunity(index, STATUS_FEAR);
                effects.push(self.apply_crusade_actor_status(
                    index,
                    &ability.id,
                    STATUS_FEAR,
                    duration,
                    AbilityStatusStackingDefinition::Extend,
                    power,
                    None,
                    None,
                    immune.then_some(AbilityStatusChangeDto::Immune),
                ));
            }
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(target_id),
                    target_kind_id: Some(definition.id),
                    effects,
                },
                trace: None,
            });
        }

        let mut all_entity_ids = Vec::new();
        let mut all_kind_ids = Vec::new();
        let mut all_positions = Vec::new();
        for _ in 0..12 {
            let candidates = self.summon_category_candidate_kind_ids("knight", None, level, false);
            let leader_positions =
                self.open_positions_around_for_actor_kinds(self.player.position, 4, &candidates);
            if candidates.is_empty() || leader_positions.is_empty() {
                continue;
            }
            let leader_position =
                leader_positions[usize::try_from(self.rng.bounded(leader_positions.len() as u64))
                    .expect("bounded knight position must fit usize")];
            let eligible = candidates
                .into_iter()
                .filter(|kind_id| {
                    self.actor_kind_available_instance_count(kind_id) > 0
                        && self.actor_kind_can_enter_position(kind_id, leader_position)
                })
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                continue;
            }
            let kind_id = eligible[usize::try_from(self.rng.bounded(eligible.len() as u64))
                .expect("bounded knight kind must fit usize")]
            .clone();
            let definition = self
                .content
                .actor(&kind_id)
                .expect("selected knight definition must remain available")
                .clone();
            let group = definition
                .allocation
                .as_ref()
                .is_some_and(|allocation| allocation.friends.is_some());
            let count = if group {
                self.original_friend_total(&definition, self.floor_depth(&self.current_floor_id))
            } else {
                1
            };
            let mut positions = vec![leader_position];
            positions.extend(
                self.open_positions_around_for_actor_kind(leader_position, 2, &kind_id)
                    .into_iter()
                    .take(usize::from(count.saturating_sub(1))),
            );
            let owner_id = self.player.id.clone();
            let resolution = self.resolve_category_summon(
                CategorySummonSpec {
                    is_spell: true,
                    source_id: &ability.id,
                    owner_id: &owner_id,
                    category: "knight",
                    count_dice: 0,
                    count_sides: 0,
                    count_bonus: 1,
                    maximum_count: None,
                    hostile: false,
                    group_chance_percent: u8::from(group).saturating_mul(100),
                    group_count_dice: 0,
                    group_count_sides: 0,
                    group_count_bonus: u8::try_from(count).unwrap_or(u8::MAX),
                    duration_turns: 0,
                },
                vec![kind_id],
                positions,
                changed,
            );
            for entity_id in &resolution.entity_ids {
                let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| &entity.id == entity_id)
                else {
                    continue;
                };
                let status = self.apply_crusade_actor_status(
                    index,
                    &ability.id,
                    STATUS_HASTE,
                    100,
                    AbilityStatusStackingDefinition::Extend,
                    base_power,
                    None,
                    None,
                    None,
                );
                self.push_actor_effect_resolution(&ability.id, index, status, events);
            }
            all_entity_ids.extend(resolution.entity_ids);
            all_kind_ids.extend(resolution.summoned_kind_ids);
            all_positions.extend(resolution.positions);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution: AbilitySummonResolutionDto {
                owner_id: self.player.id.clone(),
                actor_kind_id: "knight".to_owned(),
                entity_ids: all_entity_ids,
                positions: all_positions,
                duration_turns: 0,
                hostile: false,
                group: true,
                summoned_kind_ids: all_kind_ids,
            },
        });

        let empty_resistances = BTreeMap::new();
        let empty_brands = BTreeSet::new();
        let hero = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            "rfb.status.hero",
            1,
            25,
            1,
            25,
            AbilityStatusStackingDefinition::Replace,
            None,
            None,
            &empty_resistances,
            &empty_brands,
            &StatModifiers {
                max_hp: 10,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 12,
                ranged_skill: 12,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::from([STATUS_FEAR.to_owned()]),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let blessed = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            "rfb.status.blessed",
            1,
            25,
            1,
            25,
            AbilityStatusStackingDefinition::Replace,
            None,
            None,
            &empty_resistances,
            &empty_brands,
            &StatModifiers {
                defense: 5,
                ..StatModifiers::default()
            },
            &EquipmentBonuses {
                melee_skill: 10,
                ranged_skill: 10,
                ..EquipmentBonuses::default()
            },
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let haste = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_HASTE,
            1,
            u32::from(level),
            1,
            u32::from(level.saturating_add(20)),
            AbilityStatusStackingDefinition::Replace,
            None,
            None,
            &empty_resistances,
            &empty_brands,
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let protection = apply_ability_status_effect(
            &mut self.player,
            &ability.id,
            0,
            STATUS_PROTECTION_FROM_EVIL,
            1,
            25,
            1,
            25,
            AbilityStatusStackingDefinition::Replace,
            None,
            None,
            &empty_resistances,
            &empty_brands,
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            None,
            None,
            &mut self.rng,
        );
        let fear_removed = remove_ability_status_effect(&mut self.player, 0, STATUS_FEAR);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![hero, blessed, haste, protection, fear_removed],
            },
            trace: None,
        });
        self.refresh_player_resource_maxima();
        self.clamp_player_hp_to_effective_max();
    }

    pub(super) fn resolve_player_insanity_circle_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::InsanityCircle {
            damage_bonus,
            control_power,
            radius,
        } = ability.effect
        else {
            unreachable!("insanity-circle executor requires an insanity-circle effect");
        };
        let base_raw_damage = i32::from(damage_bonus);
        self.resolve_player_area_damage_with_base(
            &ability.id,
            Vec::new(),
            false,
            DamageType::Chaos,
            radius,
            None,
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )?;
        self.resolve_player_area_damage_with_base(
            &ability.id,
            Vec::new(),
            false,
            DamageType::Confusion,
            radius,
            None,
            base_raw_damage,
            ability.affects_ground_items,
            events,
            changed,
            removed_entities,
        )?;

        let target_ids = self
            .area_damage_targets(self.player.position, radius, None)
            .1
            .into_iter()
            .map(|(entity_id, _)| entity_id)
            .collect::<Vec<_>>();
        for target_entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id && entity.hp > 0)
            else {
                continue;
            };
            let target_kind_id = self.entities[index].kind_id.clone();
            let target_level = self
                .content
                .actor(&target_kind_id)
                .map(|definition| definition.level);
            let resistance_profile = self.entities[index].resistances.clone();
            let immunities = if self.actor_has_status_immunity(index, STATUS_CONFUSION) {
                BTreeSet::from([STATUS_CONFUSION.to_owned()])
            } else {
                BTreeSet::new()
            };
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
            let confusion = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                1,
                STATUS_CONFUSION,
                1,
                10,
                1,
                15,
                AbilityStatusStackingDefinition::Extend,
                Some(ActorDamageType::Confusion),
                Some(self.progress.level),
                &BTreeMap::new(),
                &BTreeSet::new(),
                &StatModifiers::default(),
                &EquipmentBonuses::default(),
                &BTreeSet::new(),
                None,
                false,
                100,
                target_level,
                Some((&resistance_profile, &immunities, None)),
                &mut self.rng,
            );
            let control = self.resolve_ability_control(index, 2, "any-monster", control_power);
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(target_entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![confusion, control],
                },
                trace: None,
            });
        }
        Ok(())
    }

    pub(super) fn resolve_player_explode_pets_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        debug_assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::ExplodePets
        ));
        let pet_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0 && entity.controller_id.as_deref() == Some(self.player.id.as_str())
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let mut exploded_entity_ids = Vec::new();
        let mut escaped_entity_ids = Vec::new();
        for entity_id in pet_ids {
            let Some(index) = self.entities.iter().position(|entity| {
                entity.id == entity_id
                    && entity.hp > 0
                    && entity.controller_id.as_deref() == Some(self.player.id.as_str())
            }) else {
                continue;
            };
            let entity = self.entities[index].clone();
            let definition = self
                .actor_runtime_definition(&entity)
                .expect("pet definition must remain available")
                .clone();
            let unique = definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "unique" | "unique2"));
            self.remove_pet_at(index, changed, removed_entities);
            if unique {
                escaped_entity_ids.push(entity_id);
                continue;
            }
            exploded_entity_ids.push(entity_id);
            let mut damage = entity.max_hp / 2;
            if damage > 100 {
                damage = (damage - 100) / 2 + 100;
            }
            if damage > 400 {
                damage = (damage - 400) / 2 + 400;
            }
            damage = damage.min(800);
            let radius =
                2_u8.saturating_add(u8::try_from(definition.level / 20).unwrap_or(u8::MAX));
            self.resolve_player_area_damage_with_base(
                &ability.id,
                vec![entity.position],
                false,
                DamageType::Plasma,
                radius,
                None,
                damage,
                true,
                events,
                changed,
                removed_entities,
            )?;
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::ExplodePets {
                    effect_index: 0,
                    exploded_entity_ids,
                    escaped_entity_ids,
                }],
            },
            trace: None,
        });
        Ok(())
    }
}
