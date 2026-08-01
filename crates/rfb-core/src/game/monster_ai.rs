// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Game {
    #[cfg(test)]
    pub(super) fn resolve_monster_ability(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        self.resolve_monster_ability_with_changes(
            index,
            events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("monster ability test resolution should preserve invariants")
    }

    pub(super) fn resolve_monster_ability_with_changes(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let source_entity_id = self.entities[index].id.clone();
        let source_kind_id = self.entities[index].kind_id.clone();
        let Some(casting) = self
            .content
            .actor(&source_kind_id)
            .and_then(|definition| definition.monster_casting.clone())
        else {
            return Ok(false);
        };
        if self.entities[index].casting_cooldown_remaining > 0 {
            self.entities[index].casting_cooldown_remaining -= 1;
            return Ok(false);
        }

        // FrogComposband checks the monster's spell frequency before asking
        // monspell.c to filter and choose a currently viable weighted spell.
        // Keep that RNG boundary explicit: every alerted caster action draws
        // exactly one frequency percentile, even if walls or allies later
        // leave no legal spell.
        let frequency_roll = u8::try_from(self.rng.bounded(100) + 1)
            .expect("monster ability percentile must fit u8");
        let mut candidates = Vec::with_capacity(casting.abilities.len());
        let mut viable = Vec::new();
        for candidate in &casting.abilities {
            let ability = self
                .content
                .ability(&candidate.ability_id)
                .expect("validated monster ability must remain available")
                .clone();
            match self.monster_ability_plan(index, ability, candidate.weight) {
                Ok(plan) => {
                    candidates.push(self.monster_ability_candidate_dto(index, &plan, None));
                    viable.push(plan);
                }
                Err(rejection) => {
                    candidates.push(MonsterAbilityCandidateResolutionDto {
                        ability_id: candidate.ability_id.clone(),
                        base_weight: candidate.weight,
                        effective_weight: 0,
                        target_entity_id: None,
                        target_kind_id: None,
                        target_position: None,
                        affected_positions: Vec::new(),
                        enemy_target_count: rejection.enemy_target_count,
                        friendly_risk_count: rejection.friendly_risk_count,
                        rejection_reason: Some(rejection.reason),
                    });
                }
            }
        }
        let viable_ability_ids = viable
            .iter()
            .map(|candidate| candidate.ability.id.clone())
            .collect::<Vec<_>>();
        let total_weight = viable.iter().fold(0_u32, |total, candidate| {
            total.saturating_add(candidate.effective_weight)
        });
        let mut selection_roll = None;
        let mut selected_index = None;
        if frequency_roll <= casting.frequency_percent && total_weight > 0 {
            let roll = u32::try_from(self.rng.bounded(u64::from(total_weight)) + 1)
                .expect("validated monster ability weight roll must fit u32");
            selection_roll = Some(roll);
            let mut remaining = roll;
            for (candidate_index, candidate) in viable.iter().enumerate() {
                if remaining <= candidate.effective_weight {
                    selected_index = Some(candidate_index);
                    break;
                }
                remaining -= candidate.effective_weight;
            }
        }
        let selected_ability_id =
            selected_index.map(|candidate_index| viable[candidate_index].ability.id.clone());
        events.push(DomainEvent::MonsterAbilityDecision {
            resolution: MonsterAbilityDecisionResolutionDto {
                source_entity_id: source_entity_id.clone(),
                source_kind_id: source_kind_id.clone(),
                frequency_percent: casting.frequency_percent,
                frequency_roll,
                candidates,
                viable_ability_ids,
                total_weight,
                selection_roll,
                selected_ability_id: selected_ability_id.clone(),
            },
        });

        let Some(selected_index) = selected_index else {
            return Ok(false);
        };
        let plan = viable[selected_index].clone();
        self.entities[index].casting_cooldown_remaining =
            monster_casting_cooldown(casting.frequency_percent);
        let player_hp_before = self.player.hp;
        let MonsterAbilityPlanResolution {
            target_entity_id,
            target_kind_id,
            affected_positions,
            summon,
            effects,
            targets,
            trace,
        } = self.resolve_monster_ability_plan(
            index,
            &source_kind_id,
            &plan,
            events,
            changed,
            removed_entities,
        );
        events.push(DomainEvent::MonsterAbilityCast {
            resolution: Box::new(MonsterAbilityCastResolutionDto {
                source_entity_id: source_entity_id.clone(),
                source_kind_id,
                ability_id: plan.ability.id,
                target_entity_id,
                target_kind_id,
                affected_positions,
                summon,
                effects,
                targets,
            }),
            trace,
        });
        self.resolve_vengeance_retaliation(
            &source_entity_id,
            player_hp_before.saturating_sub(self.player.hp),
            events,
            changed,
            removed_entities,
        )?;
        Ok(true)
    }

    pub(super) fn monster_ability_plan(
        &self,
        index: usize,
        ability: AbilityDefinition,
        base_weight: u32,
    ) -> Result<MonsterAbilityPlan, MonsterAbilityPlanRejection> {
        let origin = self.entities[index].position;
        let mut plan = self.monster_ability_target_plan(index, ability, base_weight)?;
        let utility_multiplier = self
            .monster_ability_utility_multiplier(index, &plan.ability, &plan.target)
            .ok_or(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::NoUtility,
                enemy_target_count: plan.enemy_target_count,
                friendly_risk_count: plan.friendly_risk_count,
            })?;
        let target_position = monster_plan_target(&plan.target).map(MonsterHostileTarget::position);
        let distance_multiplier = if !matches!(
            &plan.target,
            MonsterAbilityTargetPlan::SelfTarget | MonsterAbilityTargetPlan::Summon { .. }
        ) && target_position.is_some_and(|position| {
            origin
                .x
                .abs_diff(position.x)
                .max(origin.y.abs_diff(position.y))
                >= 3
        }) {
            2
        } else {
            1
        };
        let target_multiplier = u32::from(plan.enemy_target_count.max(1));
        let resistance_percent =
            self.monster_ability_resistance_percent(index, &plan.ability, &plan.target);
        if resistance_percent == 0 {
            return Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::NoUtility,
                enemy_target_count: plan.enemy_target_count,
                friendly_risk_count: plan.friendly_risk_count,
            });
        }
        let weighted = base_weight
            .saturating_mul(utility_multiplier)
            .saturating_mul(distance_multiplier)
            .saturating_mul(target_multiplier)
            .saturating_mul(resistance_percent)
            / 100;
        plan.effective_weight = weighted.max(1);
        Ok(plan)
    }

    fn monster_ability_resistance_percent(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: &MonsterAbilityTargetPlan,
    ) -> u32 {
        let Some(hostile_target) = monster_plan_target(target) else {
            return 100;
        };
        if !hostile_target.is_player()
            || !self
                .content
                .actor(&self.entities[source_index].kind_id)
                .and_then(|actor| actor.monster_casting.as_ref())
                .is_some_and(|casting| casting.smart)
        {
            return 100;
        }
        ability
            .effect
            .ordered_effects()
            .iter()
            .filter_map(|effect| match effect {
                AbilityEffectDefinition::Damage { damage_type, .. }
                | AbilityEffectDefinition::AreaDamage { damage_type, .. }
                | AbilityEffectDefinition::BeamDamage { damage_type, .. }
                | AbilityEffectDefinition::ConeDamage { damage_type, .. }
                | AbilityEffectDefinition::BreathDamage { damage_type, .. } => {
                    Some(DamageType::from(*damage_type))
                }
                AbilityEffectDefinition::ApplyStatus {
                    resistance_type, ..
                } => resistance_type.map(DamageType::from),
                _ => None,
            })
            .filter_map(|damage_type| {
                self.entities[source_index]
                    .observed_player_resistances
                    .get(&damage_type)
                    .copied()
            })
            .map(|level| match level {
                ResistanceLevel::Vulnerable => 150,
                ResistanceLevel::Normal => 100,
                ResistanceLevel::Resistant => 50,
                ResistanceLevel::Strong => 35,
                ResistanceLevel::Immune => 0,
            })
            .min()
            .unwrap_or(100)
    }

    fn monster_ability_utility_multiplier(
        &self,
        source_index: usize,
        ability: &AbilityDefinition,
        target: &MonsterAbilityTargetPlan,
    ) -> Option<u32> {
        if matches!(
            target,
            MonsterAbilityTargetPlan::Area { .. }
                | MonsterAbilityTargetPlan::Beam { .. }
                | MonsterAbilityTargetPlan::Cone { .. }
                | MonsterAbilityTargetPlan::Summon { .. }
                | MonsterAbilityTargetPlan::SummonCategory { .. }
        ) {
            return Some(1);
        }
        let hostile_target = monster_plan_target(target);
        let target_actor = if matches!(target, MonsterAbilityTargetPlan::SelfTarget) {
            Some(&self.entities[source_index])
        } else {
            hostile_target.and_then(|target| match target {
                MonsterHostileTarget::Player { .. } => None,
                MonsterHostileTarget::Summon { entity_id, .. } => {
                    self.entities.iter().find(|entity| entity.id == *entity_id)
                }
            })
        };
        let player_target = hostile_target.is_some_and(MonsterHostileTarget::is_player);
        let effects = ability.effect.ordered_effects();
        let mut useful = false;
        let mut multiplier = 1_u32;
        for effect in effects {
            match effect {
                AbilityEffectDefinition::Damage { .. } if hostile_target.is_some() => useful = true,
                AbilityEffectDefinition::CurseDamage { .. } if hostile_target.is_some() => {
                    useful = true;
                }
                AbilityEffectDefinition::TeleportAway { .. }
                | AbilityEffectDefinition::DrainResource { .. }
                | AbilityEffectDefinition::Amnesia
                    if hostile_target.is_some() =>
                {
                    useful = true;
                }
                AbilityEffectDefinition::BlinkSelf { .. }
                | AbilityEffectDefinition::TeleportSelf { .. } => useful = true,
                AbilityEffectDefinition::TeleportTarget if hostile_target.is_some() => {
                    useful = true;
                }
                AbilityEffectDefinition::Heal { .. } => {
                    let actor = target_actor?;
                    let missing = actor.max_hp.saturating_sub(actor.hp).max(0);
                    let missing_percent = u32::try_from(
                        i64::from(missing)
                            .saturating_mul(100)
                            .saturating_div(i64::from(actor.max_hp.max(1))),
                    )
                    .unwrap_or(100);
                    // Match the original wounded filter: healing is ignored at
                    // 20% wounds or less, then gains weight as wounds deepen.
                    if missing_percent > 20 {
                        useful = true;
                        multiplier = multiplier.max(missing_percent.div_ceil(25).clamp(1, 4));
                    }
                }
                AbilityEffectDefinition::ApplyStatus {
                    status_kind_id,
                    intensity,
                    resistance_type,
                    ..
                } => {
                    let statuses = if player_target {
                        &self.player.statuses
                    } else {
                        &target_actor?.statuses
                    };
                    let immune = resistance_type.is_some_and(|damage_type| {
                        let damage_type = DamageType::from(damage_type);
                        if player_target {
                            self.entities[source_index]
                                .observed_player_resistances
                                .get(&damage_type)
                                .is_some_and(|level| *level == ResistanceLevel::Immune)
                        } else {
                            target_actor.is_some_and(|actor| {
                                actor.resistances.level(damage_type) == ResistanceLevel::Immune
                            })
                        }
                    });
                    if !immune
                        && statuses
                            .iter()
                            .find(|status| status.kind_id == *status_kind_id)
                            .is_none_or(|status| status.intensity < *intensity)
                    {
                        useful = true;
                    }
                }
                AbilityEffectDefinition::RemoveStatus { status_kind_id } => {
                    let statuses = if player_target {
                        &self.player.statuses
                    } else {
                        &target_actor?.statuses
                    };
                    if statuses
                        .iter()
                        .any(|status| status.kind_id == *status_kind_id)
                    {
                        useful = true;
                    }
                }
                _ => {}
            }
        }
        useful.then_some(multiplier)
    }

    fn monster_ability_candidate_dto(
        &self,
        source_index: usize,
        plan: &MonsterAbilityPlan,
        rejection_reason: Option<MonsterAbilityRejectionReasonDto>,
    ) -> MonsterAbilityCandidateResolutionDto {
        let source = &self.entities[source_index];
        let (target_entity_id, target_kind_id, target_position, affected_positions) =
            match &plan.target {
                MonsterAbilityTargetPlan::SelfTarget => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    Vec::new(),
                ),
                MonsterAbilityTargetPlan::Summon { positions }
                | MonsterAbilityTargetPlan::SummonCategory { positions, .. } => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    positions.clone(),
                ),
                MonsterAbilityTargetPlan::Projectile { target, .. } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    vec![target.position()],
                ),
                MonsterAbilityTargetPlan::Area {
                    target,
                    affected_positions,
                    ..
                }
                | MonsterAbilityTargetPlan::Beam {
                    target,
                    affected_positions,
                    ..
                }
                | MonsterAbilityTargetPlan::Cone {
                    target,
                    affected_positions,
                    ..
                } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    affected_positions.clone(),
                ),
                MonsterAbilityTargetPlan::BlinkSelf { .. }
                | MonsterAbilityTargetPlan::EscapeSelf { .. } => (
                    Some(source.id.clone()),
                    Some(source.kind_id.clone()),
                    Some(source.position),
                    Vec::new(),
                ),
                MonsterAbilityTargetPlan::DragTarget {
                    target,
                    destination,
                    ..
                } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    vec![*destination],
                ),
                MonsterAbilityTargetPlan::BanishTarget { target, .. } => (
                    Some(target.entity_id().to_owned()),
                    Some(target.kind_id().to_owned()),
                    Some(target.position()),
                    Vec::new(),
                ),
            };
        MonsterAbilityCandidateResolutionDto {
            ability_id: plan.ability.id.clone(),
            base_weight: plan.base_weight,
            effective_weight: plan.effective_weight,
            target_entity_id,
            target_kind_id,
            target_position,
            affected_positions,
            enemy_target_count: plan.enemy_target_count,
            friendly_risk_count: plan.friendly_risk_count,
            rejection_reason,
        }
    }
}

impl Game {
    pub(super) fn resolve_monster_detection(
        &mut self,
        index: usize,
        events: &mut Vec<DomainEvent>,
    ) -> bool {
        let kind_id = self.entities[index].kind_id.clone();
        let Some(awareness) = self
            .content
            .actor(&kind_id)
            .and_then(|definition| definition.awareness.clone())
        else {
            self.entities[index].alerted = true;
            return true;
        };
        let monster_position = self.entities[index].position;
        let distance = monster_position
            .x
            .abs_diff(self.player.position.x)
            .max(monster_position.y.abs_diff(self.player.position.y));
        if distance > u32::from(awareness.detection_range)
            || !has_line_of_sight(self, monster_position, self.player.position)
        {
            return false;
        }
        let ability = self.player_derived_stats().stealth_skill;
        let mut difficulty_pipeline = DerivedStatsPipeline::new();
        difficulty_pipeline.add(
            StatKind::ActionDifficulty,
            StatLayer::Environment,
            &kind_id,
            awareness.detection_difficulty,
        );
        let check = resolve_check(
            &mut self.rng,
            CheckContext {
                kind: CheckKind::StealthDetection,
                actor_id: self.player.id.clone(),
                target_id: Some(self.entities[index].id.clone()),
                ability,
                difficulty: difficulty_pipeline
                    .resolve(StatKind::ActionDifficulty, StatBounds::NON_NEGATIVE),
            },
        );
        let stayed_hidden = check.succeeded();
        let skill_id = self
            .content
            .skill_by_kind(SkillKind::Stealth)
            .expect("validated stealth skill must remain available")
            .id
            .clone();
        events.push(DomainEvent::StealthChecked {
            source_kind_id: kind_id,
            succeeded: stayed_hidden,
            resolution: check.to_dto(skill_id),
        });
        if stayed_hidden {
            false
        } else {
            self.entities[index].alerted = true;
            true
        }
    }
}
