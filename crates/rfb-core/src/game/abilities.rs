// SPDX-License-Identifier: MPL-2.0

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum AbilityTargetPlan {
    SelfTarget,
    Detect,
    TerrainTransform {
        center: Position,
        positions: Vec<Position>,
    },
    Teleport {
        destination: Position,
    },
    Projectile {
        path: Vec<Position>,
        stop_at_actor: bool,
    },
    Cone {
        path: Vec<Position>,
        direction: Direction,
        radius: u8,
    },
    Summon {
        positions: Vec<Position>,
    },
    SummonCategory {
        friendly_candidate_kind_ids: Vec<String>,
        hostile_candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
    },
    Item {
        item_id: String,
    },
}

impl Game {
    pub(super) fn resolve_player_ability(
        &mut self,
        ability_id: &str,
        target: TargetSelection,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        if self.player_has_status_kind(STATUS_CONFUSION) {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "confused".to_owned(),
            });
            return Ok(());
        }
        let ability = self.content.ability(ability_id).cloned();
        let technique_profile = ability
            .as_ref()
            .and_then(|ability| self.technique_profile_for_ability(ability).cloned());
        let casting_profile = self.casting_profile().cloned();
        if technique_profile.is_none() && casting_profile.is_none() {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "no-casting-profile".to_owned(),
            });
            return Ok(());
        }
        let Some(ability) = ability else {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "unknown-ability".to_owned(),
            });
            return Ok(());
        };
        let mut ability = match (&technique_profile, &casting_profile) {
            (None, Some(profile)) => Self::effective_casting_ability(profile, &ability),
            _ => ability,
        };
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        if let Some(profile) = &casting_profile
            && technique_profile.is_none()
        {
            Self::apply_casting_profile_effect_scaling(profile, &mut ability, self.progress.level);
        }
        // Innate technique abilities skip the study/book pipeline: they are
        // granted by the class technique profile and only gate on level,
        // cooldown, and resource availability.
        let unavailable_reason = if technique_profile.is_some() {
            if self.progress.level < ability.minimum_level {
                Some("level-too-low")
            } else if self.ability_cooldown_remaining(&ability) > 0 {
                Some("cooldown")
            } else {
                None
            }
        } else {
            let profile = casting_profile
                .as_ref()
                .expect("casting profile must exist for non-technique abilities");
            if !self.learned_abilities.contains(ability_id) {
                Some("not-learned")
            } else if self.progress.level < ability.minimum_level {
                Some("level-too-low")
            } else if !self.profile_supports_ability(profile, ability_id) {
                Some("ability-not-supported")
            } else if self.ability_book_item_id(profile, ability_id).is_none() {
                Some("book-unavailable")
            } else if self.ability_cooldown_remaining(&ability) > 0 {
                Some("cooldown")
            } else {
                None
            }
        };
        if let Some(reason) = unavailable_reason {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: reason.to_owned(),
            });
            return Ok(());
        }

        // Validate the target before charging Mana or drawing the spell
        // failure/damage RNG. The command remains a normal scheduled action,
        // but an impossible target cannot consume resources or proficiency.
        let Some(mut target_plan) = self.ability_target_plan(&ability, &target) else {
            events.push(DomainEvent::AbilityTargetUnavailable {
                ability_id: ability.id,
            });
            return Ok(());
        };

        let progress_before = self.ability_progress_value(&ability);
        let cooldown_before = self.ability_cooldown_remaining(&ability);
        let resource_cost = self.ability_effective_resource_cost(&ability, progress_before);
        let failure_percent = if self.debug_ability_casts_succeed {
            0
        } else {
            match &technique_profile {
                Some(profile) => self.technique_failure_percent(profile, &ability),
                None => self.ability_failure_percent(
                    casting_profile
                        .as_ref()
                        .expect("casting profile must exist for non-technique abilities"),
                    &ability,
                ),
            }
        };
        let Some(pool) = self.resources.get_mut(&ability.resource_id) else {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "resource-unavailable".to_owned(),
            });
            return Ok(());
        };
        if pool.current < resource_cost {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-resource".to_owned(),
            });
            return Ok(());
        }
        let resource_before = pool.current;
        pool.current -= resource_cost;
        let resource_after = pool.current;
        self.resources_touched.insert(ability.resource_id.clone());
        let percentile_roll =
            u8::try_from(self.rng.bounded(100)).expect("percentile ability roll must fit u8");
        let succeeded = percentile_roll >= failure_percent;
        let progress_after = self.record_ability_cast(&ability, succeeded);
        let resolution = AbilityCastResolutionDto {
            ability_id: ability.id.clone(),
            resource_id: ability.resource_id.clone(),
            base_resource_cost: ability.resource_cost,
            resource_cost,
            resource_before,
            resource_after,
            failure_percent,
            percentile_roll,
            succeeded,
            proficiency_before: progress_before.proficiency,
            proficiency_after: progress_after.proficiency,
            proficiency_rank: Self::ability_proficiency_rank(progress_after.proficiency),
            cast_count: progress_after.cast_count,
            fail_count: progress_after.fail_count,
            cooldown_before,
            cooldown_after: self.ability_cooldown_remaining(&ability),
        };
        if !succeeded {
            events.push(DomainEvent::AbilityCastFailed { resolution });
            return Ok(());
        }
        events.push(DomainEvent::AbilityCastSucceeded {
            resolution: resolution.clone(),
        });

        if let AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } = ability.effect.clone()
        {
            let base_roll = u16::try_from(self.rng.bounded(u64::from(roll_sides)) + 1)
                .expect("random ability roll must fit u16");
            let level_bonus = self
                .progress
                .level
                .checked_div(level_bonus_divisor)
                .unwrap_or(0);
            let roll = base_roll.saturating_add(level_bonus);
            let (branch_index, branch) = branches
                .iter()
                .enumerate()
                .find(|(_, branch)| roll <= branch.maximum_roll)
                .expect("validated random ability branches must cover every roll");
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: None,
                    target_kind_id: None,
                    effects: vec![AbilityEffectResolutionDto::RandomChoice {
                        effect_index: 0,
                        roll,
                        branch_index: u16::try_from(branch_index)
                            .expect("validated random branch index must fit u16"),
                        maximum_roll: branch.maximum_roll,
                    }],
                },
                trace: None,
            });
            ability.effect = (*branch.effect).clone();
            match branch.target {
                AbilityRandomTargetDefinition::CastTarget => {
                    if !matches!(ability.effect, AbilityEffectDefinition::NoOp { .. }) {
                        target_plan = self
                            .ability_target_plan(&ability, &target)
                            .expect("validated random branch must accept the cast target");
                    }
                }
                AbilityRandomTargetDefinition::SelfTarget => {
                    ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                    ability.target.range = 0;
                    ability.target.requires_line_of_effect = false;
                    target_plan = self
                        .ability_target_plan(&ability, &TargetSelection::SelfTarget)
                        .expect("validated random branch must accept a self target");
                }
            }
        }

        self.resolve_player_ability_effect(ability, target_plan, events, changed, removed_entities)
    }
}

impl Game {
    pub(super) fn resolve_player_projectile_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Damage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &ability.effect
        else {
            unreachable!("player projectile damage executor requires a damage effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        self.resolve_ability_damage_to_entity(
            index,
            &ability.id,
            DamageType::from(*damage_type),
            raw_damage,
            trace,
            events,
            changed,
            removed_entities,
        )?;
        Ok(())
    }

    pub(super) fn resolve_player_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
            target_category,
        } = &ability.effect
        else {
            unreachable!("player area damage executor requires an area damage effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let center = trace.landing;
        let (affected_positions, targets) =
            self.area_damage_targets(center, *radius, target_category.as_deref());
        changed.extend(affected_positions.iter().copied());
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        events.push(DomainEvent::AbilityAreaDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityAreaDamageResolutionDto {
                center,
                radius: *radius,
                base_raw_damage,
                damage_type: DamageType::from(*damage_type).into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, distance);
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                DamageType::from(*damage_type),
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
        } = &ability.effect
        else {
            unreachable!("player beam damage executor requires a beam damage effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let affected_positions = trace.traversed.clone();
        let targets = self.beam_damage_targets(&affected_positions);
        changed.extend(affected_positions.iter().copied());
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        events.push(DomainEvent::AbilityBeamDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityBeamDamageResolutionDto {
                base_raw_damage,
                damage_type: DamageType::from(*damage_type).into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for entity_id in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                DamageType::from(*damage_type),
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_bolt_or_beam_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            beam_chance_percent,
        } = &ability.effect
        else {
            unreachable!("bolt-or-beam executor requires a bolt-or-beam damage effect");
        };
        let damage_type = DamageType::from(*damage_type);
        let beam = self.rng.bounded(100) < u64::from(*beam_chance_percent);
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        if beam {
            let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
            let affected_positions = trace.traversed.clone();
            let targets = self.beam_damage_targets(&affected_positions);
            changed.extend(affected_positions.iter().copied());
            events.push(DomainEvent::AbilityBeamDamage {
                ability_id: ability.id.clone(),
                resolution: AbilityBeamDamageResolutionDto {
                    base_raw_damage,
                    damage_type: damage_type.into(),
                    affected_positions,
                    target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
                },
                trace: trace.clone(),
            });
            for entity_id in targets {
                let Some(index) = self
                    .entities
                    .iter()
                    .position(|entity| entity.id == entity_id && entity.hp > 0)
                else {
                    continue;
                };
                self.resolve_ability_damage_to_entity(
                    index,
                    &ability.id,
                    damage_type,
                    base_raw_damage,
                    trace.clone(),
                    events,
                    changed,
                    removed_entities,
                )?;
            }
        } else {
            let (trace, target_index) = self.trace_projectile_path_with_actor_policy(path, true);
            let Some(index) = target_index else {
                events.push(DomainEvent::AbilityLanded {
                    ability_id: ability.id.clone(),
                    trace,
                });
                return Ok(());
            };
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                damage_type,
                base_raw_damage,
                trace,
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_cone_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::ConeDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            radius,
        } = &ability.effect
        else {
            unreachable!("player cone damage executor requires a cone damage effect");
        };
        let AbilityTargetPlan::Cone {
            path,
            direction,
            radius: planned_radius,
        } = target_plan
        else {
            unreachable!("player cone damage executor requires a cone target plan");
        };
        debug_assert_eq!(*radius, planned_radius);
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let (affected_positions, targets) =
            self.cone_damage_targets(&trace.traversed, direction, *radius);
        changed.extend(affected_positions.iter().copied());
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        events.push(DomainEvent::AbilityConeDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityConeDamageResolutionDto {
                radius: *radius,
                base_raw_damage,
                damage_type: DamageType::from(*damage_type).into(),
                affected_positions,
                target_count: u16::try_from(targets.len()).unwrap_or(u16::MAX),
            },
            trace: trace.clone(),
        });
        for (entity_id, lateral_distance) in targets {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            let falloff_damage = rfb_area_damage(base_raw_damage, lateral_distance);
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                DamageType::from(*damage_type),
                falloff_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_visible_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::VisibleDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
        } = &ability.effect
        else {
            unreachable!("visible damage executor requires a visible damage effect");
        };
        let target_ids = self
            .entities
            .iter()
            .filter(|entity| {
                entity.hp > 0
                    && self.is_visible(entity.position)
                    && target_category.as_ref().is_none_or(|category| {
                        self.content
                            .actor(&entity.kind_id)
                            .is_some_and(|definition| actor_matches_category(definition, category))
                    })
            })
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();
        let affected_positions = target_ids
            .iter()
            .filter_map(|id| self.entities.iter().find(|entity| &entity.id == id))
            .map(|entity| entity.position)
            .collect::<Vec<_>>();
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        events.push(DomainEvent::AbilityVisibleDamage {
            ability_id: ability.id.clone(),
            resolution: AbilityVisibleDamageResolutionDto {
                base_raw_damage,
                damage_type: DamageType::from(*damage_type).into(),
                affected_positions,
                target_count: u16::try_from(target_ids.len()).unwrap_or(u16::MAX),
            },
        });
        let trace = ProjectileTrace {
            origin: self.player.position,
            impact: self.player.position,
            landing: self.player.position,
            traversed: Vec::new(),
        };
        for entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == entity_id && entity.hp > 0)
            else {
                continue;
            };
            self.resolve_ability_damage_to_entity(
                index,
                &ability.id,
                DamageType::from(*damage_type),
                base_raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
        }
        Ok(())
    }

    pub(super) fn resolve_player_death_ray_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DeathRay { power } = ability.effect else {
            unreachable!("death ray executor requires a death ray effect");
        };
        let (trace, target_index) = self.trace_projectile_path(path);
        let Some(target_index) = target_index else {
            events.push(DomainEvent::AbilityLanded {
                ability_id: ability.id.clone(),
                trace,
            });
            return Ok(());
        };
        let target_entity_id = self.entities[target_index].id.clone();
        let target_kind_id = self.entities[target_index].kind_id.clone();
        let definition = self
            .content
            .actor(&target_kind_id)
            .expect("death ray target definition must remain available")
            .clone();
        let living = actor_matches_category(&definition, "living");
        let unique = definition.tags.iter().any(|tag| tag == "unique");
        let unique_roll = if living && unique {
            Some(
                u16::try_from(self.rng.bounded(888) + 1)
                    .expect("death ray unique roll must fit u16"),
            )
        } else {
            None
        };
        let unique_resisted = unique_roll.is_some_and(|roll| roll != 666);
        let (target_level_roll, caster_level_roll) = if living && !unique_resisted {
            (
                Some(
                    u16::try_from(self.rng.bounded(20) + 1)
                        .expect("death ray target roll must fit u16"),
                ),
                Some(
                    u32::try_from(self.rng.bounded(u64::from(power.max(1))) + 1)
                        .expect("validated death ray caster roll must fit u32"),
                ),
            )
        } else {
            (None, None)
        };
        let resisted = !living
            || unique_resisted
            || target_level_roll.zip(caster_level_roll).is_some_and(
                |(target_roll, caster_roll)| {
                    definition.level.saturating_add(u32::from(target_roll)) > caster_roll
                },
            );
        let damage = if resisted {
            None
        } else {
            let raw_damage = i32::from(self.progress.level).saturating_mul(200);
            let damage = resolve_damage(
                DamagePacket::new(raw_damage, DamageType::Curse),
                ResistanceLevel::Normal,
            );
            self.entities[target_index].alerted = true;
            let application = plan_damage_application(
                &self.entities[target_index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[target_index], &application);
            changed.insert(application.position);
            events.push(DomainEvent::AbilityHit {
                ability_id: ability.id.clone(),
                target_kind_id: target_kind_id.clone(),
                damage,
                trace: trace.clone(),
            });
            self.wake_entity_after_damage(target_index, damage.applied, events);
            if application.fatal {
                self.resolve_actor_death(
                    target_index,
                    DomainEvent::AbilitySlew {
                        ability_id: ability.id.clone(),
                        target_kind_id: target_kind_id.clone(),
                        damage,
                        trace: trace.clone(),
                    },
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            Some(damage.into())
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(target_entity_id),
                target_kind_id: Some(target_kind_id),
                effects: vec![AbilityEffectResolutionDto::DeathRay {
                    effect_index: 0,
                    power,
                    target_level: definition.level,
                    living,
                    unique,
                    unique_roll,
                    target_level_roll,
                    caster_level_roll,
                    resisted,
                    resolution: damage,
                }],
            },
            trace: Some(trace),
        });
        Ok(())
    }

    pub(super) fn resolve_player_drain_life_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::DrainLife {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            target_category,
            repeat,
        } = &ability.effect
        else {
            unreachable!("drain life executor requires a drain life effect");
        };
        for _ in 0..*repeat {
            let (trace, target_index) = self.trace_projectile_path(path.clone());
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
                continue;
            };
            let target_entity_id = self.entities[target_index].id.clone();
            let target_kind_id = self.entities[target_index].kind_id.clone();
            let eligible = self
                .content
                .actor(&target_kind_id)
                .is_some_and(|definition| actor_matches_category(definition, target_category));
            if !eligible {
                events.push(DomainEvent::AbilityEffectsResolved {
                    ability_id: ability.id.clone(),
                    resolution: AbilityEffectsResolutionDto {
                        target_entity_id: Some(target_entity_id),
                        target_kind_id: Some(target_kind_id),
                        effects: vec![AbilityEffectResolutionDto::Skipped {
                            effect_index: 0,
                            reason: AbilityEffectSkipReasonDto::Ineligible,
                        }],
                    },
                    trace: Some(trace),
                });
                continue;
            }
            let hp_before = self.entities[target_index].hp.max(0);
            let raw_damage = self
                .roll_damage(*damage_dice, *damage_sides)
                .saturating_add(i32::from(*damage_bonus))
                .max(0);
            let damage = self.resolve_ability_damage_to_entity(
                target_index,
                &ability.id,
                DamageType::from(*damage_type),
                raw_damage,
                trace.clone(),
                events,
                changed,
                removed_entities,
            )?;
            let requested = damage.applied.min(hp_before);
            let max_hp = self.effective_player_max_hp();
            let EffectOutcome::Healed { requested, applied } = apply_effect(
                &mut EffectTarget {
                    hp: &mut self.player.hp,
                    max_hp,
                    resistances: &self.player.resistances,
                    statuses: &mut self.player.statuses,
                },
                EffectSpec::Heal { amount: requested },
            ) else {
                unreachable!("drain life healing must produce a healing outcome");
            };
            events.push(DomainEvent::AbilityEffectsResolved {
                ability_id: ability.id.clone(),
                resolution: AbilityEffectsResolutionDto {
                    target_entity_id: Some(target_entity_id),
                    target_kind_id: Some(target_kind_id),
                    effects: vec![AbilityEffectResolutionDto::DrainLife {
                        effect_index: 0,
                        resolution: damage.into(),
                        healing: HealingResolutionDto { requested, applied },
                    }],
                },
                trace: Some(trace),
            });
        }
        Ok(())
    }

    pub(super) fn resolve_ability_control(
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
        let eligible = definition.tags.iter().any(|tag| tag == category);
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

    pub(super) fn resolve_player_visible_status_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::VisibleApplyStatus {
            status_kind_id,
            intensity,
            duration_ticks,
            stacking,
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
                    && self.is_visible(entity.position)
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
            let resolution = apply_ability_status_effect(
                &mut self.entities[index],
                &ability.id,
                0,
                status_kind_id,
                *intensity,
                *duration_ticks,
                0,
                0,
                *stacking,
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

    pub(super) fn resolve_player_healing_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Heal { amount } = ability.effect else {
            unreachable!("player healing executor requires a healing effect");
        };
        let amount = i32::try_from(amount).expect("validated healing amount must fit i32");
        let max_hp = self.effective_player_max_hp();
        let outcome = apply_effect(
            &mut EffectTarget {
                hp: &mut self.player.hp,
                max_hp,
                resistances: &self.player.resistances,
                statuses: &mut self.player.statuses,
            },
            EffectSpec::Heal { amount },
        );
        let EffectOutcome::Healed { requested, applied } = outcome else {
            unreachable!("healing abilities must produce healing outcomes");
        };
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto { requested, applied },
        });
    }

    pub(super) fn resolve_player_restore_vitality_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RestoreVitality { life_force } = ability.effect else {
            unreachable!("vitality executor requires a restore vitality effect");
        };
        let experience_before = self.progress.experience;
        let life_force_before = self.progress.life_force;
        self.progress.experience = self.progress.maximum_experience;
        self.progress.life_force = self.progress.life_force.max(life_force).min(1_000);
        self.apply_player_experience(0, events);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RestoreVitality {
                    effect_index: 0,
                    experience_before,
                    experience_after: self.progress.experience,
                    life_force_before,
                    life_force_after: self.progress.life_force,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        destination: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let from = self.player.position;
        events.push(DomainEvent::AbilityTeleported {
            ability_id: ability.id.clone(),
            resolution: AbilityTeleportResolutionDto {
                from,
                to: destination,
            },
        });
        events.extend(self.relocate_player(destination, changed));
    }

    pub(super) fn resolve_player_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Summon {
            actor_kind_id,
            count,
            duration_turns,
            hostile,
            ..
        } = &ability.effect
        else {
            unreachable!("summon executor requires a fixed summon effect");
        };
        debug_assert_eq!(usize::from(*count), positions.len());
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated summon actor must remain available")
            .clone();
        let mut entity_ids = Vec::with_capacity(positions.len());
        for (ordinal, position) in positions.iter().copied().enumerate() {
            let id = self.summon_entity_id(&ability.id, ordinal);
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
            if !hostile {
                entity.summon = Some(SummonIdentity {
                    owner_id: self.player.id.clone(),
                    source_ability_id: ability.id.clone(),
                    remaining_turns: *duration_turns,
                });
            }
            changed.insert(position);
            entity_ids.push(id);
            self.entities.push(entity);
        }
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution: AbilitySummonResolutionDto {
                owner_id: self.player.id.clone(),
                actor_kind_id: actor_kind_id.clone(),
                entity_ids,
                positions,
                duration_turns: *duration_turns,
                hostile: *hostile,
                group: false,
                summoned_kind_ids: Vec::new(),
            },
        });
    }

    pub(super) fn resolve_player_category_summon_effect(
        &mut self,
        ability: &AbilityDefinition,
        friendly_candidate_kind_ids: Vec<String>,
        hostile_candidate_kind_ids: Vec<String>,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::SummonCategory {
            category,
            upgraded_category,
            upgrade_at_level,
            count_dice,
            count_sides,
            count_bonus,
            hostile_chance_percent,
            friendly_group_chance_percent,
            hostile_group_chance_percent,
            group_count_dice,
            group_count_sides,
            group_count_bonus,
            duration_turns,
            ..
        } = &ability.effect
        else {
            unreachable!("category summon executor requires a category summon effect");
        };
        let hostile = match *hostile_chance_percent {
            0 => false,
            100 => true,
            chance => self.rng.bounded(100) < u64::from(chance),
        };
        let group_chance = if hostile {
            *hostile_group_chance_percent
        } else {
            *friendly_group_chance_percent
        };
        let candidates = if hostile {
            hostile_candidate_kind_ids
        } else {
            friendly_candidate_kind_ids
        };
        let selected_category = upgraded_category
            .as_deref()
            .zip(*upgrade_at_level)
            .filter(|(_, level)| self.progress.level >= *level)
            .map_or(category.as_str(), |(category, _)| category);
        let owner_id = self.player.id.clone();
        let resolution = self.resolve_category_summon(
            CategorySummonSpec {
                source_id: &ability.id,
                owner_id: &owner_id,
                category: selected_category,
                count_dice: *count_dice,
                count_sides: *count_sides,
                count_bonus: *count_bonus,
                hostile,
                group_chance_percent: group_chance,
                group_count_dice: *group_count_dice,
                group_count_sides: *group_count_sides,
                group_count_bonus: *group_count_bonus,
                duration_turns: *duration_turns,
            },
            candidates,
            positions,
            changed,
        );
        events.push(DomainEvent::AbilitySummoned {
            ability_id: ability.id.clone(),
            resolution,
        });
    }

    pub(super) fn resolve_player_detection_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::Detect {
            subject,
            category,
            radius,
            persistent,
        } = &ability.effect
        else {
            unreachable!("detection executor requires a detection effect");
        };
        let (detected_positions, detected_entity_ids) = match subject {
            AbilityDetectSubjectDefinition::Terrain => (
                self.detect_terrain_positions(category, *radius, *persistent, false),
                Vec::new(),
            ),
            AbilityDetectSubjectDefinition::Actor => self.detect_actor_positions(category, *radius),
            AbilityDetectSubjectDefinition::Item => {
                self.detect_item_positions(category, *radius, false)
            }
        };
        if *persistent {
            changed.extend(detected_positions.iter().copied());
        }
        events.push(DomainEvent::AbilityDetected {
            ability_id: ability.id.clone(),
            resolution: AbilityDetectResolutionDto {
                subject: ability_detect_subject_dto(*subject),
                category: category.clone(),
                radius: *radius,
                persistent: *persistent,
                detected_positions,
                detected_entity_ids,
            },
        });
    }

    pub(super) fn resolve_player_terrain_transform_effect(
        &mut self,
        ability: &AbilityDefinition,
        center: Position,
        positions: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::TransformTerrain {
            source_terrain_ids,
            target_terrain_id,
            radius,
        } = &ability.effect
        else {
            unreachable!("terrain executor requires a terrain transform effect");
        };
        for position in &positions {
            let index = self
                .index(*position)
                .expect("planned terrain transformation must remain in bounds");
            debug_assert!(source_terrain_ids.contains(&self.terrain[index]));
            self.terrain[index].clone_from(target_terrain_id);
            self.revealed_terrain.remove(position);
            changed.insert(*position);
        }
        events.push(DomainEvent::AbilityTerrainTransformed {
            ability_id: ability.id.clone(),
            resolution: AbilityTerrainTransformResolutionDto {
                center,
                radius: *radius,
                source_terrain_ids: source_terrain_ids.clone(),
                target_terrain_id: target_terrain_id.clone(),
                transformed_positions: positions,
            },
        });
    }
}

impl Game {
    pub(super) fn ability_target_plan(
        &self,
        ability: &AbilityDefinition,
        target: &TargetSelection,
    ) -> Option<AbilityTargetPlan> {
        match ability.effect {
            // These forms are monster-casting-only. The player cast path
            // never produces a target plan for them.
            AbilityEffectDefinition::BlinkSelf { .. }
            | AbilityEffectDefinition::TeleportSelf { .. }
            | AbilityEffectDefinition::TeleportTarget
            | AbilityEffectDefinition::BreathDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia => None,
            AbilityEffectDefinition::Teleport => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                self.teleport_destination(ability, *position)
                    .map(|destination| AbilityTargetPlan::Teleport { destination })
            }
            AbilityEffectDefinition::Summon { count, radius, .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then(|| self.summon_positions_around(self.player.position, count, radius))
                .flatten()
                .map(|positions| AbilityTargetPlan::Summon { positions })
            }
            AbilityEffectDefinition::SummonCategory {
                ref category,
                ref upgraded_category,
                upgrade_at_level,
                maximum_level,
                count_dice,
                count_sides,
                count_bonus,
                hostile_chance_percent,
                group_count_dice,
                group_count_sides,
                group_count_bonus,
                allow_unique_hostile,
                radius,
                ..
            } => {
                if !matches!(target, TargetSelection::SelfTarget)
                    || !ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    return None;
                }
                let selected_category = upgraded_category
                    .as_deref()
                    .zip(upgrade_at_level)
                    .filter(|(_, level)| self.progress.level >= *level)
                    .map_or(category.as_str(), |(category, _)| category);
                let excluded_upgrade_category = upgraded_category
                    .as_deref()
                    .filter(|category| *category != selected_category);
                let friendly_candidate_kind_ids = self.summon_category_candidate_kind_ids(
                    selected_category,
                    excluded_upgrade_category,
                    maximum_level,
                    false,
                );
                let hostile_candidate_kind_ids = self.summon_category_candidate_kind_ids(
                    selected_category,
                    excluded_upgrade_category,
                    maximum_level,
                    allow_unique_hostile,
                );
                if (hostile_chance_percent < 100 && friendly_candidate_kind_ids.is_empty())
                    || (hostile_chance_percent > 0 && hostile_candidate_kind_ids.is_empty())
                {
                    return None;
                }
                let normal_maximum =
                    usize::from(count_dice) * usize::from(count_sides) + usize::from(count_bonus);
                let group_maximum = usize::from(group_count_dice) * usize::from(group_count_sides)
                    + usize::from(group_count_bonus);
                let positions = self
                    .open_positions_around(self.player.position, radius)
                    .into_iter()
                    .take(normal_maximum.max(group_maximum))
                    .collect::<Vec<_>>();
                (!positions.is_empty()).then_some(AbilityTargetPlan::SummonCategory {
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                })
            }
            AbilityEffectDefinition::AnimateDead { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::IdentifyItem { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                (ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::Item)
                    && self.items.iter().any(|item| {
                        item.id == *item_id
                            && match &item.location {
                                ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
                                ItemLocation::Ground(position) => *position == self.player.position,
                                ItemLocation::CarriedBy { .. } => false,
                            }
                    }))
                .then(|| AbilityTargetPlan::Item {
                    item_id: item_id.clone(),
                })
            }
            AbilityEffectDefinition::Detect { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::Detect)
            }
            AbilityEffectDefinition::TransformTerrain {
                ref source_terrain_ids,
                ref target_terrain_id,
                radius,
            } => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                self.terrain_transform_positions(
                    ability,
                    *position,
                    source_terrain_ids,
                    target_terrain_id,
                    radius,
                )
                .map(|positions| AbilityTargetPlan::TerrainTransform {
                    center: *position,
                    positions,
                })
            }
            AbilityEffectDefinition::ApplyStatus { .. }
            | AbilityEffectDefinition::RemoveStatus { .. }
            | AbilityEffectDefinition::Control { .. }
            | AbilityEffectDefinition::Sequence { .. } => {
                if ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    (matches!(target, TargetSelection::SelfTarget))
                        .then_some(AbilityTargetPlan::SelfTarget)
                } else {
                    self.ability_path(ability, target)
                        .map(|path| AbilityTargetPlan::Projectile {
                            path,
                            stop_at_actor: true,
                        })
                }
            }
            AbilityEffectDefinition::Heal { .. } => (matches!(target, TargetSelection::SelfTarget)
                && ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget))
            .then_some(AbilityTargetPlan::SelfTarget),
            AbilityEffectDefinition::RestoreVitality { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::VisibleDamage { .. }
            | AbilityEffectDefinition::VisibleApplyStatus { .. }
            | AbilityEffectDefinition::EnchantEquippedWeapon { .. }
            | AbilityEffectDefinition::NoOp { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::RandomChoice { .. } => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::Damage { .. } | AbilityEffectDefinition::DeathRay { .. } => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::AreaDamage { .. } => {
                if matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    Some(AbilityTargetPlan::Projectile {
                        path: Vec::new(),
                        stop_at_actor: false,
                    })
                } else {
                    self.ability_path(ability, target)
                        .map(|path| AbilityTargetPlan::Projectile {
                            path,
                            stop_at_actor: matches!(target, TargetSelection::Direction { .. }),
                        })
                }
            }
            AbilityEffectDefinition::BeamDamage { .. }
            | AbilityEffectDefinition::BoltOrBeamDamage { .. } => self
                .beam_ability_path(ability, target)
                .map(|path| AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor: false,
                }),
            AbilityEffectDefinition::Genocide {
                scope: AbilityGenocideScopeDefinition::Nearby,
                ..
            } => (matches!(target, TargetSelection::SelfTarget)
                && ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::SelfTarget))
            .then_some(AbilityTargetPlan::SelfTarget),
            AbilityEffectDefinition::DrainLife { .. }
            | AbilityEffectDefinition::Genocide { .. } => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::ConeDamage { radius, .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Cone {
                        path,
                        direction: *direction,
                        radius,
                    })
            }
        }
    }
}
