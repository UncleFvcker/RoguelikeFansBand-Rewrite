// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    pub(super) fn resolve_monster_fixed_summon_plan(
        &mut self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        changed: &mut BTreeSet<Position>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Summon { positions } = &plan.target else {
            unreachable!("monster fixed summon executor requires a summon target plan")
        };
        let AbilityEffectDefinition::Summon {
            ref actor_kind_id,
            duration_turns,
            ..
        } = plan.ability.effect
        else {
            unreachable!("monster summon plan must retain a summon effect");
        };
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated summoned actor must remain available")
            .clone();
        let owner_id = self.entities[source_index].id.clone();
        let mut entity_ids = Vec::with_capacity(positions.len());
        for (ordinal, position) in positions.iter().copied().enumerate() {
            let id = self.summon_entity_id(&plan.ability.id, ordinal);
            let mut entity = actor_from_runtime_spawn(
                &id,
                actor_kind_id,
                position,
                definition.max_hp,
                definition.speed,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.resistances = definition_resistance_profile(&definition);
            entity.summon = Some(SummonIdentity {
                owner_id: owner_id.clone(),
                source_ability_id: plan.ability.id.clone(),
                remaining_turns: duration_turns,
            });
            changed.insert(position);
            entity_ids.push(id);
            self.entities.push(entity);
        }
        let summon = AbilitySummonResolutionDto {
            owner_id: owner_id.clone(),
            actor_kind_id: actor_kind_id.clone(),
            entity_ids,
            positions: positions.clone(),
            duration_turns,
            hostile: false,
            group: false,
            summoned_kind_ids: Vec::new(),
        };
        MonsterAbilityPlanResolution {
            target_entity_id: owner_id,
            target_kind_id: self.entities[source_index].kind_id.clone(),
            affected_positions: positions.clone(),
            summon: Some(summon),
            effects: Vec::new(),
            targets: Vec::new(),
            trace: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_cone_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Cone {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster cone executor requires a cone target plan")
        };
        let (raw_damage, damage_type, radius) = match &plan.ability.effect {
            AbilityEffectDefinition::ConeDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
            } => (
                self.roll_damage(*damage_dice, *damage_sides)
                    .saturating_add(i32::from(*damage_bonus))
                    .max(0),
                damage_type,
                radius,
            ),
            AbilityEffectDefinition::BreathDamage {
                hp_percent,
                max_damage,
                damage_type,
                radius,
            } => {
                // Breath scales with the caster's current vigor: no damage
                // dice are rolled, and the elemental cap bounds a healthy caster.
                let caster_hp = self
                    .entities
                    .iter()
                    .find(|entity| entity.id == source_entity_id)
                    .map_or(0, |entity| entity.hp)
                    .max(0);
                let scaled = caster_hp
                    .saturating_mul(i32::from(*hp_percent))
                    .div_euclid(100);
                (scaled.min(i32::from(*max_damage)), damage_type, radius)
            }
            _ => unreachable!("monster cone plan must retain a cone or breath effect"),
        };
        let origin = self
            .entities
            .iter()
            .find(|entity| entity.id == source_entity_id)
            .map_or(trace.origin, |entity| entity.position);
        let direction = direction_toward(origin, target.position())
            .expect("validated monster cone retains a direction");
        let lateral_distances = self
            .cone_damage_cells(origin, &trace.traversed, direction, *radius)
            .into_iter()
            .map(|(_, lateral, position)| (position, lateral))
            .collect::<BTreeMap<_, _>>();
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let lateral_distance = lateral_distances
                .get(&affected_target.position())
                .copied()
                .unwrap_or(0);
            let prepared = rfb_area_damage(raw_damage, lateral_distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_beam_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Beam {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster beam executor requires a beam target plan")
        };
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &plan.ability.effect
        else {
            unreachable!("monster beam plan must retain a beam effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                raw_damage,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(affected_target.position());
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: affected_target.position(),
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_monster_area_damage_plan(
        &mut self,
        source_index: usize,
        source_entity_id: &str,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let MonsterAbilityTargetPlan::Area {
            target,
            trace,
            affected_positions,
        } = &plan.target
        else {
            unreachable!("monster area executor requires an area target plan")
        };
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            ..
        } = &plan.ability.effect
        else {
            unreachable!("monster area plan must retain an area effect");
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        let target_actors =
            self.monster_targets_in_footprint(source_index, target, affected_positions);
        let mut targets = Vec::with_capacity(target_actors.len());
        for affected_target in target_actors {
            let position = affected_target.position();
            let distance = target
                .position()
                .x
                .abs_diff(position.x)
                .max(target.position().y.abs_diff(position.y));
            let prepared = rfb_area_damage(raw_damage, distance);
            let effect = self.resolve_monster_damage_to_hostile(
                source_entity_id,
                source_kind_id,
                &plan.ability.id,
                0,
                raw_damage,
                prepared,
                DamageType::from(*damage_type),
                &affected_target,
                events,
            );
            changed.insert(position);
            targets.push(MonsterAbilityTargetResolutionDto {
                target_entity_id: affected_target.entity_id().to_owned(),
                target_kind_id: affected_target.kind_id().to_owned(),
                target_position: position,
                effects: vec![effect],
            });
        }
        let effects = targets
            .iter()
            .find(|resolution| resolution.target_entity_id == target.entity_id())
            .map(|resolution| resolution.effects.clone())
            .unwrap_or_default();
        self.remove_defeated_player_summons(
            targets
                .iter()
                .map(|target| target.target_entity_id.as_str()),
            changed,
            removed_entities,
        );
        MonsterAbilityPlanResolution {
            target_entity_id: target.entity_id().to_owned(),
            target_kind_id: target.kind_id().to_owned(),
            affected_positions: affected_positions.clone(),
            summon: None,
            effects,
            targets,
            trace: Some(trace.clone()),
        }
    }

    pub(super) fn resolve_monster_self_effects(
        &mut self,
        source_index: usize,
        ability: &AbilityDefinition,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let resolution = match effect {
                AbilityEffectDefinition::Heal { amount } => {
                    let amount =
                        i32::try_from(*amount).expect("validated healing amount must fit i32");
                    let actor = &mut self.entities[source_index];
                    let outcome = apply_effect(
                        &mut EffectTarget {
                            hp: &mut actor.hp,
                            max_hp: actor.max_hp,
                            resistances: &actor.resistances,
                            statuses: &mut actor.statuses,
                        },
                        EffectSpec::Heal { amount },
                    );
                    let EffectOutcome::Healed { requested, applied } = outcome else {
                        unreachable!("monster healing must produce a healing outcome");
                    };
                    AbilityEffectResolutionDto::Heal {
                        effect_index,
                        resolution: HealingResolutionDto { requested, applied },
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
                    &mut self.entities[source_index],
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
                        &mut self.entities[source_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!("validated monster self effects must remain actor effects"),
            };
            resolutions.push(resolution);
        }
        resolutions
    }

    pub(super) fn resolve_monster_ability_plan(
        &mut self,
        source_index: usize,
        source_kind_id: &str,
        plan: &MonsterAbilityPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> MonsterAbilityPlanResolution {
        let source_entity_id = self.entities[source_index].id.clone();
        match &plan.target {
            MonsterAbilityTargetPlan::SelfTarget => {
                let target_entity_id = self.entities[source_index].id.clone();
                let target_kind_id = self.entities[source_index].kind_id.clone();
                let target_position = self.entities[source_index].position;
                let effects = self.resolve_monster_self_effects(source_index, &plan.ability);
                changed.insert(target_position);
                MonsterAbilityPlanResolution {
                    target_entity_id: target_entity_id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    affected_positions: Vec::new(),
                    summon: None,
                    effects: effects.clone(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id,
                        target_kind_id,
                        target_position,
                        effects,
                    }],
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::Projectile { target, trace } => {
                let effects = self.resolve_monster_hostile_effects(
                    &source_entity_id,
                    source_kind_id,
                    &plan.ability,
                    target,
                    events,
                    changed,
                );
                changed.insert(target.position());
                let targets = vec![MonsterAbilityTargetResolutionDto {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    target_position: target.position(),
                    effects: effects.clone(),
                }];
                self.remove_defeated_player_summons(
                    targets
                        .iter()
                        .map(|target| target.target_entity_id.as_str()),
                    changed,
                    removed_entities,
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![target.position()],
                    summon: None,
                    effects,
                    targets,
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::Area { .. } => self.resolve_monster_area_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Beam { .. } => self.resolve_monster_beam_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Cone { .. } => self.resolve_monster_cone_damage_plan(
                source_index,
                &source_entity_id,
                source_kind_id,
                plan,
                events,
                changed,
                removed_entities,
            ),
            MonsterAbilityTargetPlan::Summon { .. } => {
                self.resolve_monster_fixed_summon_plan(source_index, plan, changed)
            }
            MonsterAbilityTargetPlan::SummonCategory {
                candidate_kind_ids,
                positions,
            } => {
                let AbilityEffectDefinition::SummonCategory {
                    ref category,
                    count_dice,
                    count_sides,
                    count_bonus,
                    duration_turns,
                    ..
                } = plan.ability.effect
                else {
                    unreachable!("monster category summon plan must retain its effect");
                };
                // The count dice roll first, then one bounded draw picks each
                // summon's kind; space shortfalls truncate to the secured
                // cells (planning guaranteed at least one).
                let rolled = self
                    .roll_damage(u16::from(count_dice), u16::from(count_sides))
                    .saturating_add(i32::from(count_bonus))
                    .max(1);
                let count = usize::try_from(rolled).unwrap_or(1).min(positions.len());
                let owner_id = self.entities[source_index].id.clone();
                let mut entity_ids = Vec::with_capacity(count);
                let mut summoned_kind_ids = Vec::with_capacity(count);
                let mut used_positions = Vec::with_capacity(count);
                let planned_positions = positions.iter().copied().take(count).collect::<Vec<_>>();
                for (ordinal, position) in planned_positions.into_iter().enumerate() {
                    let choice = usize::try_from(self.rng.bounded(
                        u64::try_from(candidate_kind_ids.len()).expect("candidate count fits"),
                    ))
                    .expect("bounded draw fits usize");
                    let kind_id = candidate_kind_ids[choice].clone();
                    let definition = self
                        .content
                        .actor(&kind_id)
                        .expect("validated summon candidate must remain available")
                        .clone();
                    let id = self.summon_entity_id(&plan.ability.id, ordinal);
                    let mut entity = actor_from_runtime_spawn(
                        &id,
                        &kind_id,
                        position,
                        definition.max_hp,
                        definition.speed,
                        INITIAL_MONSTER_ENERGY_NEED,
                        true,
                    );
                    entity.resistances = definition_resistance_profile(&definition);
                    entity.summon = Some(SummonIdentity {
                        owner_id: owner_id.clone(),
                        source_ability_id: plan.ability.id.clone(),
                        remaining_turns: duration_turns,
                    });
                    changed.insert(position);
                    entity_ids.push(id);
                    summoned_kind_ids.push(kind_id);
                    used_positions.push(position);
                    self.entities.push(entity);
                }
                let summon = AbilitySummonResolutionDto {
                    owner_id: owner_id.clone(),
                    actor_kind_id: category.clone(),
                    entity_ids,
                    positions: used_positions.clone(),
                    duration_turns,
                    hostile: false,
                    group: false,
                    summoned_kind_ids,
                };
                MonsterAbilityPlanResolution {
                    target_entity_id: owner_id,
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: used_positions,
                    summon: Some(summon),
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BlinkSelf { destinations }
            | MonsterAbilityTargetPlan::EscapeSelf { destinations } => {
                // The candidate list was collected without RNG at planning
                // time; the actual landing cell consumes one bounded draw.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                let from = self.entities[source_index].position;
                self.entities[source_index].position = destination;
                changed.insert(from);
                changed.insert(destination);
                let resolution = MonsterDisplacementResolutionDto {
                    actor_id: source_entity_id.clone(),
                    from,
                    to: destination,
                };
                events.push(
                    if matches!(plan.target, MonsterAbilityTargetPlan::BlinkSelf { .. }) {
                        DomainEvent::MonsterBlinked {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    } else {
                        DomainEvent::MonsterTeleported {
                            source_kind_id: source_kind_id.to_owned(),
                            resolution,
                        }
                    },
                );
                MonsterAbilityPlanResolution {
                    target_entity_id: source_entity_id.clone(),
                    target_kind_id: self.entities[source_index].kind_id.clone(),
                    affected_positions: vec![from, destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: Vec::new(),
                    trace: None,
                }
            }
            MonsterAbilityTargetPlan::BanishTarget {
                target,
                trace,
                destinations,
            } => {
                // One bounded draw picks the landing cell from the
                // plan-collected candidates, mirroring the escape family.
                let choice = usize::try_from(
                    self.rng
                        .bounded(u64::try_from(destinations.len()).expect("candidate count fits")),
                )
                .expect("bounded draw fits usize");
                let destination = destinations[choice];
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        events.push(DomainEvent::MonsterBanishedTarget {
                            source_kind_id: source_kind_id.to_owned(),
                            target_kind_id: target.kind_id().to_owned(),
                            resolution: MonsterDisplacementResolutionDto {
                                actor_id: target.entity_id().to_owned(),
                                from,
                                to: destination,
                            },
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(banished_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[banished_index].position;
                            self.entities[banished_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            events.push(DomainEvent::MonsterBanishedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution: MonsterDisplacementResolutionDto {
                                    actor_id: entity_id.clone(),
                                    from,
                                    to: destination,
                                },
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
            MonsterAbilityTargetPlan::DragTarget {
                target,
                trace,
                destination,
            } => {
                let destination = *destination;
                match target {
                    MonsterHostileTarget::Player { .. } => {
                        let from = self.player.position;
                        events.push(DomainEvent::MonsterDraggedTarget {
                            source_kind_id: source_kind_id.to_owned(),
                            target_kind_id: target.kind_id().to_owned(),
                            resolution: MonsterDisplacementResolutionDto {
                                actor_id: target.entity_id().to_owned(),
                                from,
                                to: destination,
                            },
                        });
                        let relocation = self.relocate_player(destination, changed);
                        events.extend(relocation);
                    }
                    MonsterHostileTarget::Summon { entity_id, .. } => {
                        if let Some(dragged_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *entity_id && entity.hp > 0)
                        {
                            let from = self.entities[dragged_index].position;
                            self.entities[dragged_index].position = destination;
                            changed.insert(from);
                            changed.insert(destination);
                            events.push(DomainEvent::MonsterDraggedTarget {
                                source_kind_id: source_kind_id.to_owned(),
                                target_kind_id: target.kind_id().to_owned(),
                                resolution: MonsterDisplacementResolutionDto {
                                    actor_id: entity_id.clone(),
                                    from,
                                    to: destination,
                                },
                            });
                        }
                    }
                }
                MonsterAbilityPlanResolution {
                    target_entity_id: target.entity_id().to_owned(),
                    target_kind_id: target.kind_id().to_owned(),
                    affected_positions: vec![destination],
                    summon: None,
                    effects: Vec::new(),
                    targets: vec![MonsterAbilityTargetResolutionDto {
                        target_entity_id: target.entity_id().to_owned(),
                        target_kind_id: target.kind_id().to_owned(),
                        target_position: destination,
                        effects: Vec::new(),
                    }],
                    trace: Some(trace.clone()),
                }
            }
        }
    }

    fn monster_targets_in_footprint(
        &self,
        source_index: usize,
        primary: &MonsterHostileTarget,
        affected_positions: &[Position],
    ) -> Vec<MonsterHostileTarget> {
        let mut targets = self
            .monster_hostile_targets(source_index)
            .into_iter()
            .filter(|target| affected_positions.contains(&target.position()))
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| {
            (left.entity_id() != primary.entity_id())
                .cmp(&(right.entity_id() != primary.entity_id()))
                .then_with(|| left.entity_id().cmp(right.entity_id()))
        });
        targets
    }

    fn remove_defeated_player_summons<'a>(
        &mut self,
        target_entity_ids: impl Iterator<Item = &'a str>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) {
        let mut defeated = target_entity_ids
            .filter_map(|entity_id| {
                self.entities
                    .iter()
                    .position(|entity| {
                        entity.id == entity_id
                            && entity.hp <= 0
                            && self.actor_is_player_aligned(entity)
                    })
                    .map(|index| self.entities[index].id.clone())
            })
            .collect::<Vec<_>>();
        defeated.sort();
        defeated.dedup();
        for entity_id in defeated {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id)
            else {
                continue;
            };
            let removed = self.entities.remove(index);
            changed.insert(removed.position);
            removed_entities.push(removed.id);
        }
    }

    fn resolve_monster_hostile_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        target: &MonsterHostileTarget,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        if target.is_player() {
            return self.resolve_monster_player_effects(
                source_entity_id,
                source_kind_id,
                ability,
                events,
                changed,
            );
        }
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            let Some(target_index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target.entity_id())
            else {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            };
            if self.entities[target_index].hp <= 0 {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
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
                    let damage_type = DamageType::from(*damage_type);
                    let definition = self
                        .content
                        .actor(&self.entities[target_index].kind_id)
                        .expect("monster target definition must remain available");
                    let armor_class = self
                        .actor_derived_stats(&self.entities[target_index], definition, false)
                        .armor_class
                        .value;
                    let prepared = if damage_type == DamageType::Physical {
                        apply_melee_armor_reduction(raw_damage, armor_class)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_hostile(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        target,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    // Summoned targets have no saving-throw skill; the curse
                    // lands in full (documented v98 simplification).
                    let raw_damage = self
                        .roll_damage(*damage_dice, *damage_sides)
                        .saturating_add(i32::from(*damage_bonus))
                        .max(0);
                    self.resolve_monster_damage_to_hostile(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        raw_damage,
                        DamageType::Curse,
                        target,
                        events,
                    )
                }
                AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia => {
                    // Summons carry no resource pools or map knowledge; both
                    // effects fizzle against them (documented v99 boundary).
                    AbilityEffectResolutionDto::Skipped {
                        effect_index,
                        reason: AbilityEffectSkipReasonDto::NoTarget,
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
                        .actor(&self.entities[target_index].kind_id)
                        .map(|definition| definition.level);
                    apply_ability_status_effect(
                        &mut self.entities[target_index],
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
                    remove_ability_status_effect(
                        &mut self.entities[target_index],
                        effect_index,
                        status_kind_id,
                    )
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
    }

    fn resolve_monster_player_effects(
        &mut self,
        source_entity_id: &str,
        source_kind_id: &str,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Vec<AbilityEffectResolutionDto> {
        let effects = ability.effect.ordered_effects();
        let mut resolutions = Vec::with_capacity(effects.len());
        for (index, effect) in effects.iter().enumerate() {
            let effect_index =
                u8::try_from(index).expect("validated monster ability effect index must fit u8");
            if self.player_is_dead() {
                resolutions.push(AbilityEffectResolutionDto::Skipped {
                    effect_index,
                    reason: AbilityEffectSkipReasonDto::TargetDead,
                });
                continue;
            }
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
                    let damage_type = DamageType::from(*damage_type);
                    let target = self.player_derived_stats();
                    let prepared = if damage_type == DamageType::Physical {
                        apply_melee_armor_reduction(raw_damage, target.armor_class.value)
                    } else {
                        raw_damage
                    };
                    self.resolve_monster_damage_to_player(
                        source_entity_id,
                        source_kind_id,
                        &ability.id,
                        effect_index,
                        raw_damage,
                        prepared,
                        damage_type,
                        events,
                    )
                }
                AbilityEffectDefinition::CurseDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                } => {
                    // A successful saving throw negates the curse before any
                    // damage dice are drawn; difficulty follows the caster's
                    // definition level.
                    let ability_stat = self.player_derived_stats().saving_throw_skill;
                    let caster_level = self
                        .content
                        .actor(source_kind_id)
                        .map_or(1, |definition| definition.level);
                    let mut difficulty_pipeline = DerivedStatsPipeline::new();
                    difficulty_pipeline.add(
                        StatKind::ActionDifficulty,
                        StatLayer::Environment,
                        source_kind_id,
                        i32::try_from(caster_level).unwrap_or(i32::MAX),
                    );
                    let check = resolve_check(
                        &mut self.rng,
                        CheckContext {
                            kind: CheckKind::SavingThrow,
                            actor_id: self.player.id.clone(),
                            target_id: Some(source_kind_id.to_owned()),
                            ability: ability_stat,
                            difficulty: difficulty_pipeline
                                .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                        },
                    );
                    let succeeded = check.succeeded();
                    let skill_id = self
                        .content
                        .skill_by_kind(SkillKind::SavingThrow)
                        .expect("validated saving throw skill must remain available")
                        .id
                        .clone();
                    events.push(DomainEvent::SavingThrowChecked {
                        source_kind_id: source_kind_id.to_owned(),
                        position: self.player.position,
                        succeeded,
                        resolution: check.to_dto(skill_id),
                    });
                    if succeeded {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        let raw_damage = self
                            .roll_damage(*damage_dice, *damage_sides)
                            .saturating_add(i32::from(*damage_bonus))
                            .max(0);
                        self.resolve_monster_damage_to_player(
                            source_entity_id,
                            source_kind_id,
                            &ability.id,
                            effect_index,
                            raw_damage,
                            raw_damage,
                            DamageType::Curse,
                            events,
                        )
                    }
                }
                AbilityEffectDefinition::DrainResource { amount } => {
                    // The casting-profile pool is drained when present; other
                    // players lose their first non-empty pool in id order.
                    let pool_id = self
                        .casting_profile()
                        .map(|profile| profile.resource_id.clone())
                        .filter(|id| self.resources.contains_key(id))
                        .or_else(|| {
                            self.resources
                                .iter()
                                .find(|(_, pool)| pool.current > 0)
                                .map(|(id, _)| id.clone())
                        });
                    let requested = *amount;
                    let (resource_id, drained) = match pool_id {
                        Some(id) => {
                            let pool = self
                                .resources
                                .get_mut(&id)
                                .expect("selected drain pool must remain available");
                            let drained = pool.current.min(requested);
                            pool.current -= drained;
                            (id, drained)
                        }
                        None => (String::new(), 0),
                    };
                    // The caster feeds on the stolen power, capped at its
                    // own maximum life.
                    let mut caster_healed = 0_u32;
                    if drained > 0
                        && let Some(caster_index) = self
                            .entities
                            .iter()
                            .position(|entity| entity.id == *source_entity_id)
                    {
                        let caster = &mut self.entities[caster_index];
                        let missing = caster.max_hp.saturating_sub(caster.hp).max(0);
                        let healed = i32::try_from(drained).unwrap_or(i32::MAX).min(missing);
                        caster.hp += healed;
                        caster_healed = u32::try_from(healed).unwrap_or(0);
                        changed.insert(caster.position);
                    }
                    AbilityEffectResolutionDto::DrainResource {
                        effect_index,
                        resource_id,
                        requested,
                        drained,
                        caster_healed,
                    }
                }
                AbilityEffectDefinition::Amnesia => {
                    // The saving throw gates the memory wipe exactly like the
                    // curse family; success costs no further RNG.
                    let ability_stat = self.player_derived_stats().saving_throw_skill;
                    let caster_level = self
                        .content
                        .actor(source_kind_id)
                        .map_or(1, |definition| definition.level);
                    let mut difficulty_pipeline = DerivedStatsPipeline::new();
                    difficulty_pipeline.add(
                        StatKind::ActionDifficulty,
                        StatLayer::Environment,
                        source_kind_id,
                        i32::try_from(caster_level).unwrap_or(i32::MAX),
                    );
                    let check = resolve_check(
                        &mut self.rng,
                        CheckContext {
                            kind: CheckKind::SavingThrow,
                            actor_id: self.player.id.clone(),
                            target_id: Some(source_kind_id.to_owned()),
                            ability: ability_stat,
                            difficulty: difficulty_pipeline
                                .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
                        },
                    );
                    let succeeded = check.succeeded();
                    let skill_id = self
                        .content
                        .skill_by_kind(SkillKind::SavingThrow)
                        .expect("validated saving throw skill must remain available")
                        .id
                        .clone();
                    events.push(DomainEvent::SavingThrowChecked {
                        source_kind_id: source_kind_id.to_owned(),
                        position: self.player.position,
                        succeeded,
                        resolution: check.to_dto(skill_id),
                    });
                    if succeeded {
                        AbilityEffectResolutionDto::Skipped {
                            effect_index,
                            reason: AbilityEffectSkipReasonDto::Saved,
                        }
                    } else {
                        // Only the current floor map memory fades; item
                        // knowledge stays authoritative per the long-term
                        // design constraints.
                        let width = usize::from(self.width);
                        let mut cleared_cells = 0_u32;
                        for (index, explored) in self.explored.iter_mut().enumerate() {
                            if *explored {
                                *explored = false;
                                cleared_cells += 1;
                                changed.insert(Position {
                                    x: i32::try_from(index % width)
                                        .expect("explored x must fit i32"),
                                    y: i32::try_from(index / width)
                                        .expect("explored y must fit i32"),
                                });
                            }
                        }
                        cleared_cells += u32::try_from(self.revealed_terrain.len()).unwrap_or(0);
                        self.revealed_terrain.clear();
                        AbilityEffectResolutionDto::Amnesia {
                            effect_index,
                            cleared_cells,
                        }
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
                    let effective = self.effective_player_resistances();
                    let immunities = self.player_status_immunities();
                    let target_level = u32::from(self.progress.level);
                    let resolution = apply_ability_status_effect(
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
                        Some(target_level),
                        Some((&effective, &immunities)),
                        &mut self.rng,
                    );
                    if let Some(damage_type) = resistance_type.map(DamageType::from) {
                        let level = effective.level(damage_type);
                        self.record_monster_player_resistance(source_entity_id, damage_type, level);
                    }
                    resolution
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    remove_ability_status_effect(&mut self.player, effect_index, status_kind_id)
                }
                _ => unreachable!(
                    "validated monster abilities contain only direct actor-target effects"
                ),
            };
            resolutions.push(resolution);
        }
        resolutions
    }
}
