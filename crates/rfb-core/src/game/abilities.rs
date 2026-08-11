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
    RandomTeleport {
        candidates: Vec<Position>,
    },
    FetchItem {
        path: Vec<Position>,
    },
    ConsumeTerrain {
        position: Position,
        source_terrain_id: String,
        target_terrain_id: String,
    },
    MeleeThenTeleport {
        target_entity_id: String,
        teleport_candidates: Vec<Position>,
    },
    Recall {
        action: RecallUseAction,
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

fn attribute_kind_dto(kind: AttributeKind) -> rfb_protocol::AttributeKindDto {
    match kind {
        AttributeKind::Strength => rfb_protocol::AttributeKindDto::Strength,
        AttributeKind::Intelligence => rfb_protocol::AttributeKindDto::Intelligence,
        AttributeKind::Wisdom => rfb_protocol::AttributeKindDto::Wisdom,
        AttributeKind::Dexterity => rfb_protocol::AttributeKindDto::Dexterity,
        AttributeKind::Constitution => rfb_protocol::AttributeKindDto::Constitution,
        AttributeKind::Charisma => rfb_protocol::AttributeKindDto::Charisma,
    }
}

fn set_attribute_value(attributes: &mut AttributeSet, kind: AttributeKind, value: u16) {
    match kind {
        AttributeKind::Strength => attributes.strength = value,
        AttributeKind::Intelligence => attributes.intelligence = value,
        AttributeKind::Wisdom => attributes.wisdom = value,
        AttributeKind::Dexterity => attributes.dexterity = value,
        AttributeKind::Constitution => attributes.constitution = value,
        AttributeKind::Charisma => attributes.charisma = value,
    }
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
        let mutation_activation = self.mutation_activation_for_ability(ability_id).cloned();
        let casting_profile = self.casting_profile().cloned();
        if technique_profile.is_none() && mutation_activation.is_none() && casting_profile.is_none()
        {
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
        let source = if technique_profile.is_some() {
            AbilitySourceDto::Technique
        } else if mutation_activation.is_some() {
            AbilitySourceDto::Mutation
        } else if casting_profile.is_some() {
            AbilitySourceDto::Learned
        } else {
            unreachable!("at least one validated ability source must be available")
        };
        let mut ability = match source {
            AbilitySourceDto::Learned => Self::effective_casting_ability(
                casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile"),
                &ability,
            ),
            AbilitySourceDto::Technique | AbilitySourceDto::Mutation => ability,
        };
        Self::apply_player_level_scaling(&mut ability, self.progress.level);
        if source == AbilitySourceDto::Learned {
            Self::apply_casting_profile_effect_scaling(
                casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile"),
                &mut ability,
                self.progress.level,
            );
        }
        let unavailable_reason = match source {
            AbilitySourceDto::Technique => {
                let player = Self::player_ability_parameters(&ability);
                if self.progress.level < player.minimum_level {
                    Some("level-too-low")
                } else if self.ability_cooldown_remaining(&ability) > 0 {
                    Some("cooldown")
                } else {
                    None
                }
            }
            AbilitySourceDto::Mutation => {
                let activation = mutation_activation
                    .as_ref()
                    .expect("mutation ability source requires an activation");
                (self.progress.level < activation.minimum_level).then_some("level-too-low")
            }
            AbilitySourceDto::Learned => {
                let player = Self::player_ability_parameters(&ability);
                let profile = casting_profile
                    .as_ref()
                    .expect("learned ability source requires a casting profile");
                if !self.learned_abilities.contains(ability_id) {
                    Some("not-learned")
                } else if self.progress.level < player.minimum_level {
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
            }
        };
        if let Some(reason) = unavailable_reason {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: reason.to_owned(),
            });
            return Ok(());
        }

        // Validate the target before charging resources/HP or drawing the
        // failure/damage RNG. The command remains a normal scheduled action,
        // but an impossible target cannot consume resources or proficiency.
        let Some(mut target_plan) = self.ability_target_plan(&ability, &target) else {
            events.push(DomainEvent::AbilityTargetUnavailable {
                ability_id: ability.id,
            });
            return Ok(());
        };

        let mutation_progress = AbilityProgress {
            proficiency: 0,
            proficiency_cap: 0,
            cast_count: 0,
            fail_count: 0,
            cooldown_remaining: 0,
        };
        let progress_before = if source == AbilitySourceDto::Mutation {
            mutation_progress
        } else {
            self.ability_progress_value(&ability)
        };
        let cooldown_before = if source == AbilitySourceDto::Mutation {
            0
        } else {
            self.ability_cooldown_remaining(&ability)
        };
        let (base_resource_cost, resource_cost, resource_id) =
            if source == AbilitySourceDto::Mutation {
                let activation = mutation_activation
                    .as_ref()
                    .expect("mutation ability source requires an activation");
                let cost = self.mutation_resource_cost(activation);
                (
                    activation.cost,
                    cost,
                    casting_profile
                        .as_ref()
                        .map(|profile| profile.resource_id.clone()),
                )
            } else {
                let player = Self::player_ability_parameters(&ability);
                (
                    player.resource_cost,
                    self.ability_effective_resource_cost(&ability, progress_before),
                    Some(player.resource_id.clone()),
                )
            };
        let failure_percent = if self.debug_ability_casts_succeed {
            0
        } else {
            match source {
                AbilitySourceDto::Technique => self.technique_failure_percent(
                    technique_profile
                        .as_ref()
                        .expect("technique ability source requires a profile"),
                    &ability,
                ),
                AbilitySourceDto::Mutation => self.mutation_failure_percent(
                    mutation_activation
                        .as_ref()
                        .expect("mutation ability source requires an activation"),
                ),
                AbilitySourceDto::Learned => self.ability_failure_percent(
                    casting_profile
                        .as_ref()
                        .expect("learned ability source requires a casting profile"),
                    &ability,
                ),
            }
        };
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        if source != AbilitySourceDto::Mutation
            && resource_id
                .as_deref()
                .is_none_or(|id| !self.resources.contains_key(id))
        {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "resource-unavailable".to_owned(),
            });
            return Ok(());
        }
        let resource_paid = if source == AbilitySourceDto::Mutation {
            resource_before.min(resource_cost)
        } else {
            resource_cost
        };
        let hp_paid = resource_cost.saturating_sub(resource_paid);
        let affordable = if source == AbilitySourceDto::Mutation {
            hp_paid <= u32::try_from(self.player.hp.max(0)).unwrap_or(0)
        } else {
            resource_before >= resource_cost
        };
        if !affordable {
            events.push(DomainEvent::AbilityCastUnavailable {
                ability_id: ability_id.to_owned(),
                reason: "insufficient-resource".to_owned(),
            });
            return Ok(());
        }
        if resource_paid > 0 {
            let id = resource_id
                .as_ref()
                .expect("positive resource payment requires a resource id");
            let pool = self
                .resources
                .get_mut(id)
                .expect("positive resource payment requires an available pool");
            pool.current -= resource_paid;
            self.resources_touched.insert(id.clone());
        }
        if hp_paid > 0 {
            self.player.hp = self.player.hp.saturating_sub(
                i32::try_from(hp_paid).expect("validated mutation cost must fit i32"),
            );
        }
        let resource_after = resource_before.saturating_sub(resource_paid);
        let percentile_roll =
            u8::try_from(self.rng.bounded(100)).expect("percentile ability roll must fit u8");
        let succeeded = percentile_roll >= failure_percent;
        let progress_after = if source == AbilitySourceDto::Mutation {
            mutation_progress
        } else {
            self.record_ability_cast(&ability, succeeded)
        };
        let resolution = AbilityCastResolutionDto {
            ability_id: ability.id.clone(),
            resource_id,
            base_resource_cost,
            resource_cost,
            resource_before,
            resource_after,
            resource_paid,
            hp_paid,
            failure_percent,
            percentile_roll,
            succeeded,
            proficiency_before: progress_before.proficiency,
            proficiency_after: progress_after.proficiency,
            proficiency_rank: Self::ability_proficiency_rank(progress_after.proficiency),
            cast_count: progress_after.cast_count,
            fail_count: progress_after.fail_count,
            cooldown_before,
            cooldown_after: if source == AbilitySourceDto::Mutation {
                0
            } else {
                self.ability_cooldown_remaining(&ability)
            },
        };
        if !succeeded {
            events.push(DomainEvent::AbilityCastFailed { resolution });
            return Ok(());
        }
        events.push(DomainEvent::AbilityCastSucceeded {
            resolution: resolution.clone(),
        });

        if matches!(
            &ability.effect,
            AbilityEffectDefinition::RandomChoice { .. }
        ) {
            self.select_player_random_choice_branch(
                &mut ability,
                &target,
                &mut target_plan,
                events,
            );
        }

        self.resolve_player_ability_effect(ability, target_plan, events, changed, removed_entities)
    }

    fn select_player_random_choice_branch(
        &mut self,
        ability: &mut AbilityDefinition,
        target: &TargetSelection,
        target_plan: &mut AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RandomChoice {
            roll_sides,
            level_bonus_divisor,
            branches,
        } = ability.effect.clone()
        else {
            unreachable!("random choice selector requires a random choice effect");
        };
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
                    *target_plan = self
                        .ability_target_plan(ability, target)
                        .expect("validated random branch must accept the cast target");
                }
            }
            AbilityRandomTargetDefinition::SelfTarget => {
                ability.target.modes = vec![AbilityTargetModeDefinition::SelfTarget];
                ability.target.range = 0;
                ability.target.requires_line_of_effect = false;
                *target_plan = self
                    .ability_target_plan(ability, &TargetSelection::SelfTarget)
                    .expect("validated random branch must accept a self target");
            }
        }
    }
}

impl Game {
    fn resolve_player_ability_effect(
        &mut self,
        ability: AbilityDefinition,
        target_plan: AbilityTargetPlan,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        match (ability.effect.clone(), target_plan) {
            (AbilityEffectDefinition::Teleport, AbilityTargetPlan::Teleport { destination }) => {
                self.resolve_player_teleport_effect(&ability, destination, events, changed);
            }
            (
                AbilityEffectDefinition::BlinkSelf { .. },
                AbilityTargetPlan::RandomTeleport { candidates },
            ) => {
                self.resolve_player_random_teleport_effect(&ability, candidates, events, changed);
            }
            (AbilityEffectDefinition::FetchItem { .. }, AbilityTargetPlan::FetchItem { path }) => {
                self.resolve_player_fetch_item_effect(&ability, path, events, changed)
            }
            (
                AbilityEffectDefinition::ConsumeTerrain { .. },
                AbilityTargetPlan::ConsumeTerrain {
                    position,
                    source_terrain_id,
                    target_terrain_id,
                },
            ) => self.resolve_player_consume_terrain_effect(
                &ability,
                position,
                source_terrain_id,
                target_terrain_id,
                events,
                changed,
            ),
            (
                AbilityEffectDefinition::TransmuteItemToGold { .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_transmute_item_effect(&ability, &item_id, events),
            (
                AbilityEffectDefinition::DrainItemMagic { .. },
                AbilityTargetPlan::Item { item_id },
            ) => self.resolve_player_drain_item_magic_effect(&ability, &item_id, events),
            (AbilityEffectDefinition::ReportMagic, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_report_magic_effect(&ability, events)
            }
            (AbilityEffectDefinition::Earthquake { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_earthquake_effect(&ability, events, changed, removed_entities)?;
            }
            (
                AbilityEffectDefinition::SuppressMonsterReproduction { .. },
                AbilityTargetPlan::SelfTarget,
            ) => self.resolve_player_suppress_reproduction_effect(&ability, events),
            (
                AbilityEffectDefinition::MeleeThenTeleport { .. },
                AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id,
                    teleport_candidates,
                },
            ) => self.resolve_player_melee_then_teleport_effect(
                &ability,
                &target_entity_id,
                teleport_candidates,
                events,
                changed,
                removed_entities,
            )?,
            (AbilityEffectDefinition::PolymorphSelf, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_polymorph_self_effect(&ability, events)
            }
            (AbilityEffectDefinition::SwapPosition, AbilityTargetPlan::Projectile { path, .. }) => {
                self.resolve_player_swap_position_effect(&ability, path, events, changed)
            }
            (AbilityEffectDefinition::Recall { .. }, AbilityTargetPlan::Recall { action }) => {
                self.resolve_player_recall_effect(&ability, action, events)
            }
            (AbilityEffectDefinition::ResistElements { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_resist_elements_effect(&ability, events)
            }
            (AbilityEffectDefinition::Summon { .. }, AbilityTargetPlan::Summon { positions }) => {
                self.resolve_player_summon_effect(&ability, positions, events, changed);
            }
            (
                AbilityEffectDefinition::SummonCategory { .. },
                AbilityTargetPlan::SummonCategory {
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                },
            ) => {
                self.resolve_player_category_summon_effect(
                    &ability,
                    friendly_candidate_kind_ids,
                    hostile_candidate_kind_ids,
                    positions,
                    events,
                    changed,
                );
            }
            (AbilityEffectDefinition::Detect { .. }, AbilityTargetPlan::Detect) => {
                self.resolve_player_detection_effect(&ability, events, changed);
            }
            (
                AbilityEffectDefinition::TransformTerrain { .. },
                AbilityTargetPlan::TerrainTransform { center, positions },
            ) => {
                self.resolve_terrain_transform_effect(&ability, center, positions, events, changed);
            }
            (
                AbilityEffectDefinition::ApplyStatus { .. }
                | AbilityEffectDefinition::RemoveStatus { .. },
                target_plan,
            ) => {
                self.resolve_player_actor_status_effect(&ability, target_plan, events, changed);
                self.clamp_player_hp_to_effective_max();
            }
            (AbilityEffectDefinition::Control { .. }, target_plan) => {
                self.resolve_player_control_effect(&ability, target_plan, events, changed);
                self.clamp_player_hp_to_effective_max();
            }
            (AbilityEffectDefinition::Sequence { .. }, target_plan) => {
                self.resolve_player_ordered_sequence_effect(
                    &ability,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
                self.clamp_player_hp_to_effective_max();
            }
            (
                AbilityEffectDefinition::Damage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_projectile_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::DeathRay { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_death_ray_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::AreaDamage { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_area_damage_effect(
                    &ability,
                    path,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BeamDamage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_beam_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BoltOrBeamDamage { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_bolt_or_beam_damage_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::BoltOrAreaDamage { .. },
                AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor,
                },
            ) => {
                self.resolve_player_bolt_or_area_damage_effect(
                    &ability,
                    path,
                    stop_at_actor,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::ConeDamage { .. },
                target_plan @ AbilityTargetPlan::Cone { .. },
            ) => {
                self.resolve_player_cone_damage_effect(
                    &ability,
                    target_plan,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::Heal { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_healing_effect(&ability, events);
            }
            (AbilityEffectDefinition::IdentifyItem { .. }, AbilityTargetPlan::Item { item_id }) => {
                self.resolve_player_identify_item_effect(&ability, &item_id, events);
            }
            (AbilityEffectDefinition::RestoreVitality { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_restore_vitality_effect(&ability, events);
            }
            (AbilityEffectDefinition::VisibleDamage { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_damage_effect(
                    &ability,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (AbilityEffectDefinition::VisibleApplyStatus { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_visible_status_effect(&ability, events, changed);
            }
            (AbilityEffectDefinition::AggravateMonsters, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_aggravate_monsters_effect(&ability, events, changed);
            }
            (
                AbilityEffectDefinition::EnchantEquippedWeapon { .. },
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_player_enchant_equipped_weapon_effect(&ability, events);
            }
            (AbilityEffectDefinition::NoOp { .. }, _) => {
                self.resolve_player_no_op_effect(&ability, events);
            }
            (
                AbilityEffectDefinition::DrainLife { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_drain_life_effect(
                    &ability,
                    path,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            (
                AbilityEffectDefinition::Genocide { .. },
                AbilityTargetPlan::Projectile { path, .. },
            ) => {
                self.resolve_player_genocide_effect(
                    &ability,
                    Some(path),
                    events,
                    changed,
                    removed_entities,
                );
            }
            (
                AbilityEffectDefinition::Genocide {
                    scope: AbilityGenocideScopeDefinition::Nearby,
                    ..
                },
                AbilityTargetPlan::SelfTarget,
            ) => {
                self.resolve_player_genocide_effect(
                    &ability,
                    None,
                    events,
                    changed,
                    removed_entities,
                );
            }
            (AbilityEffectDefinition::AnimateDead { .. }, AbilityTargetPlan::SelfTarget) => {
                self.resolve_player_animate_dead_effect(&ability, events, changed)?;
            }
            _ => unreachable!("validated ability target plan must match its effect"),
        }
        Ok(())
    }

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
        if self.try_reflect_player_bolt(
            index,
            &ability.id,
            raw_damage,
            DamageType::from(*damage_type),
            events,
            changed,
            removed_entities,
        )? {
            return Ok(());
        }
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
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        self.resolve_player_area_damage_with_base(
            &ability.id,
            path,
            stop_at_actor,
            DamageType::from(*damage_type),
            *radius,
            target_category.as_deref(),
            base_raw_damage,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_area_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        stop_at_actor: bool,
        damage_type: DamageType,
        radius: u8,
        target_category: Option<&str>,
        base_raw_damage: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let center = trace.landing;
        let (affected_positions, targets) =
            self.area_damage_targets(center, radius, target_category);
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityAreaDamage {
            ability_id: source_id.to_owned(),
            resolution: AbilityAreaDamageResolutionDto {
                center,
                radius,
                base_raw_damage,
                damage_type: damage_type.into(),
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
                source_id,
                damage_type,
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
        let base_raw_damage = self
            .roll_damage(*damage_dice, *damage_sides)
            .saturating_add(i32::from(*damage_bonus))
            .max(0);
        self.resolve_player_beam_damage_with_base(
            &ability.id,
            path,
            DamageType::from(*damage_type),
            base_raw_damage,
            events,
            changed,
            removed_entities,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn resolve_player_beam_damage_with_base(
        &mut self,
        source_id: &str,
        path: Vec<Position>,
        damage_type: DamageType,
        base_raw_damage: i32,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, false);
        let affected_positions = trace.traversed.clone();
        let targets = self.beam_damage_targets(&affected_positions);
        changed.extend(affected_positions.iter().copied());
        events.push(DomainEvent::AbilityBeamDamage {
            ability_id: source_id.to_owned(),
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
                source_id,
                damage_type,
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
            if self.try_reflect_player_bolt(
                index,
                &ability.id,
                base_raw_damage,
                damage_type,
                events,
                changed,
                removed_entities,
            )? {
                return Ok(());
            }
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

    fn resolve_player_bolt_or_area_damage_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        stop_at_actor: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::BoltOrAreaDamage {
            damage_dice,
            damage_sides,
            damage_bonus,
            damage_type,
            area_from_level,
            radius,
        } = ability.effect
        else {
            unreachable!("bolt-or-area executor requires a matching effect");
        };
        let mut resolved = ability.clone();
        if self.progress.level < area_from_level {
            resolved.effect = AbilityEffectDefinition::Damage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
            };
            self.resolve_player_projectile_damage_effect(
                &resolved,
                path,
                events,
                changed,
                removed_entities,
            )
        } else {
            resolved.effect = AbilityEffectDefinition::AreaDamage {
                damage_dice,
                damage_sides,
                damage_bonus,
                damage_type,
                radius,
                target_category: None,
            };
            self.resolve_player_area_damage_effect(
                &resolved,
                path,
                stop_at_actor,
                events,
                changed,
                removed_entities,
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn try_reflect_player_bolt(
        &mut self,
        reflector_index: usize,
        source_kind_id: &str,
        raw_damage: i32,
        damage_type: DamageType,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        let reflector_kind_id = self.entities[reflector_index].kind_id.clone();
        if !self
            .actor_runtime_definition(&self.entities[reflector_index])
            .is_some_and(|definition| definition.reflects_bolts)
            || self.rng.bounded(4) == 0
        {
            return Ok(false);
        }

        let origin = self.entities[reflector_index].position;
        let range = self.width.max(self.height);
        let mut reflected_path = None;
        for _ in 0..10 {
            let y = self.player.position.y
                + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32")
                - 2;
            let x = self.player.position.x
                + i32::try_from(self.rng.bounded(5)).expect("bounded draw fits i32")
                - 2;
            let destination = Position { x, y };
            let Some(path) = projectile_path_between(origin, destination, range) else {
                continue;
            };
            if path
                .iter()
                .all(|position| self.index(*position).is_some() && self.is_walkable(*position))
            {
                reflected_path = Some(path);
                break;
            }
        }
        let path = reflected_path
            .or_else(|| projectile_path_between(origin, self.player.position, range))
            .expect("an incoming bolt must retain a reverse reflection path");
        let can_hit_player = self.rng.bounded(2) != 0;
        let mut impact = origin;
        let mut landing = origin;
        let mut traversed = Vec::new();
        let mut hit_player = false;
        let mut hit_actor_index = None;
        for position in path {
            impact = position;
            if self.index(position).is_none() || !self.is_walkable(position) {
                break;
            }
            landing = position;
            traversed.push(position);
            if can_hit_player && position == self.player.position {
                hit_player = true;
                break;
            }
            if let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.hp > 0 && entity.position == position)
            {
                hit_actor_index = Some(index);
                break;
            }
        }
        let trace = ProjectileTrace {
            origin,
            impact,
            landing,
            traversed,
        };

        if hit_player {
            let target = self.player_derived_stats();
            let resistance = self.effective_player_resistances().level(damage_type);
            let damage = self.reduce_player_damage(resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            ));
            let application =
                plan_damage_application(&self.player, damage, FatalityPolicy::BelowZero);
            commit_damage_application(&mut self.player, &application);
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id: reflector_kind_id.clone(),
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id: self.player.kind_id.clone(),
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if application.fatal {
                events.push(DomainEvent::PlayerDied {
                    source_kind_id: reflector_kind_id,
                    method_id: Some(source_kind_id.to_owned()),
                    damage,
                });
            }
            return Ok(true);
        }

        if let Some(index) = hit_actor_index {
            let definition = self
                .content
                .actor(&self.entities[index].kind_id)
                .expect("reflected bolt target definition must remain available")
                .clone();
            let target_kind_id = definition.id.clone();
            let target = self.actor_derived_stats(&self.entities[index], &definition, false);
            let resistance = self.entities[index].resistances.level(damage_type);
            let damage = resolve_armored_damage(
                raw_damage,
                damage_type,
                target.armor_class.value,
                resistance,
            );
            let application = plan_damage_application(
                &self.entities[index],
                damage,
                FatalityPolicy::AtOrBelowZero,
            );
            commit_damage_application(&mut self.entities[index], &application);
            self.entities[index].alerted = true;
            changed.insert(self.entities[index].position);
            self.wake_entity_after_damage(index, damage.applied, events);
            events.push(DomainEvent::BoltReflected {
                reflector_kind_id,
                source_kind_id: source_kind_id.to_owned(),
                outcome: BoltReflectionOutcome::Hit {
                    target_kind_id,
                    damage,
                    fatal: application.fatal,
                },
                trace,
            });
            if application.fatal {
                self.resolve_actor_death_without_rewards(
                    index,
                    None,
                    events,
                    changed,
                    removed_entities,
                )?;
            }
            return Ok(true);
        }

        events.push(DomainEvent::BoltReflected {
            reflector_kind_id,
            source_kind_id: source_kind_id.to_owned(),
            outcome: BoltReflectionOutcome::Landed,
            trace,
        });
        Ok(true)
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
                    && self.entity_is_visible_to_player(entity)
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
            if !application.fatal {
                self.resolve_monster_fear_aura(target_index, "hurt", true, events);
            }
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
            feeds,
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
            let requested = if !*feeds || self.nutrition < hunger::NUTRITION_FULL {
                damage.applied.min(hp_before)
            } else {
                0
            };
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
            if *feeds && damage.applied > 0 {
                let nutrition = u16::try_from(raw_damage.saturating_mul(100).min(5_000))
                    .expect("bounded vampiric nutrition must fit u16");
                if self.nutrition < rfb_protocol::PLAYER_NUTRITION_MAXIMUM {
                    self.nutrition = self
                        .nutrition
                        .saturating_add(nutrition)
                        .min(rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
                }
            }
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

    pub(super) fn resolve_player_control_effect(
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

    pub(super) fn resolve_player_actor_status_effect(
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
                            None,
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

    pub(super) fn resolve_player_ordered_sequence_effect(
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
                        | AbilityEffectDefinition::ApplyStatus { .. }
                        | AbilityEffectDefinition::RemoveStatus { .. }
                )
            })
        {
            for effect in effects {
                let mut step = ability.clone();
                step.effect = effect.clone();
                let plan = match effect {
                    AbilityEffectDefinition::AreaDamage { .. } => AbilityTargetPlan::Projectile {
                        path: Vec::new(),
                        stop_at_actor: false,
                    },
                    AbilityEffectDefinition::Detect { .. } => AbilityTargetPlan::Detect,
                    AbilityEffectDefinition::Heal { .. }
                    | AbilityEffectDefinition::ApplyStatus { .. }
                    | AbilityEffectDefinition::RemoveStatus { .. }
                    | AbilityEffectDefinition::VisibleDamage { .. }
                    | AbilityEffectDefinition::VisibleApplyStatus { .. }
                    | AbilityEffectDefinition::AggravateMonsters
                    | AbilityEffectDefinition::NoOp { .. } => AbilityTargetPlan::SelfTarget,
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
                            let max_hp = self.effective_player_max_hp();
                            let amount = i32::try_from(*amount)
                                .expect("validated healing amount must fit i32");
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
                                unreachable!("healing effects must produce healing outcomes");
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
                0,
                0,
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
        let outcome = apply_healing(&mut self.player.hp, max_hp, HealingRequest::amount(amount));
        events.push(DomainEvent::AbilityHealed {
            ability_id: ability.id.clone(),
            resolution: HealingResolutionDto {
                requested: outcome.requested,
                applied: outcome.applied,
            },
        });
    }

    pub(super) fn resolve_player_identify_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        } = ability.effect
        else {
            unreachable!("item identification executor requires an identify item effect");
        };
        let roll = u16::try_from(self.rng.bounded(u64::from(full_identify_roll_sides)) + 1)
            .expect("validated identify roll must fit u16");
        let full = roll <= full_identify_power;
        let identification =
            self.identify_item_instance(item_id, ItemIdentificationRequest::new(full));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::IdentifyItem {
                    effect_index: 0,
                    item_id: identification.item_id,
                    item_kind_id: identification.item_kind_id,
                    full_identify_power,
                    full_identify_roll_sides,
                    roll,
                    full,
                    changed: identification.changed,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_enchant_equipped_weapon_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::EnchantEquippedWeapon { affix_id } = &ability.effect else {
            unreachable!("weapon enchantment executor requires a weapon enchantment effect");
        };
        let weapon_index = self.items.iter().position(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return false;
            };
            self.body_slot_type(slot_id) == Some("weapon")
                && self
                    .content
                    .item(&item.kind_id)
                    .is_some_and(|definition| definition.melee_profile.is_some())
        });
        let (item_id, item_kind_id, added) = if let Some(index) = weapon_index {
            let item_id = self.items[index].id.clone();
            let item_kind_id = self.items[index].kind_id.clone();
            let added = if self.items[index].affix_ids.contains(affix_id) {
                false
            } else {
                self.items[index].affix_ids.push(affix_id.clone());
                self.items[index].affix_ids.sort();
                self.items[index].quality = ItemQualityDto::Fine;
                let knowledge = self
                    .item_property_knowledge
                    .entry(item_id.clone())
                    .or_default();
                knowledge.discovered = true;
                knowledge.appraised = true;
                knowledge.identified = true;
                knowledge.known_affix_ids.insert(affix_id.clone());
                true
            };
            (item_id, item_kind_id, added)
        } else {
            (String::new(), String::new(), false)
        };
        self.clamp_player_hp_to_effective_max();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::EnchantEquippedWeapon {
                    effect_index: 0,
                    item_id,
                    item_kind_id,
                    affix_id: affix_id.clone(),
                    added,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_no_op_effect(
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

    pub(super) fn resolve_player_restore_vitality_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::RestoreVitality { life_force } = ability.effect else {
            unreachable!("vitality executor requires a restore vitality effect");
        };
        let experience = apply_experience_restoration(&mut self.progress);
        let life_force = apply_life_force_restoration(
            &mut self.progress,
            LifeForceRestorationRequest::at_least(life_force),
        );
        self.apply_player_experience(0, events);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::RestoreVitality {
                    effect_index: 0,
                    experience_before: experience.before,
                    experience_after: experience.after,
                    life_force_before: life_force.before,
                    life_force_after: life_force.after,
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

    fn resolve_player_random_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded teleport candidate index must fit usize");
        self.resolve_player_teleport_effect(ability, candidates[index], events, changed);
    }

    fn resolve_player_fetch_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::FetchItem {
            maximum_weight_tenths_pound,
        } = ability.effect
        else {
            unreachable!("fetch item executor requires a fetch item effect");
        };
        let candidate = path.iter().find_map(|position| {
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| matches!(item.location, ItemLocation::Ground(at) if at == *position))
                .min_by(|left, right| left.1.id.cmp(&right.1.id))
                .map(|(index, item)| (index, item.id.clone(), *position))
        });
        let mut item_id = None;
        let mut from = None;
        let mut moved = false;
        if let Some((index, id, position)) = candidate {
            let weight = u32::from(self.item_weight_tenths_pound(&self.items[index].kind_id))
                .saturating_mul(self.items[index].quantity);
            item_id = Some(id);
            from = Some(position);
            if weight <= maximum_weight_tenths_pound {
                self.items[index].location = ItemLocation::Ground(self.player.position);
                changed.insert(position);
                changed.insert(self.player.position);
                moved = true;
            }
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::FetchItem {
                    effect_index: 0,
                    item_id,
                    from,
                    to: self.player.position,
                    moved,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_consume_terrain_effect(
        &mut self,
        ability: &AbilityDefinition,
        position: Position,
        source_terrain_id: String,
        target_terrain_id: String,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::ConsumeTerrain {
            nutrition: base_nutrition,
        } = ability.effect
        else {
            unreachable!("terrain consumption executor requires a consume-terrain effect");
        };
        let source = self
            .content
            .terrain(&source_terrain_id)
            .expect("planned consumed terrain must remain available");
        let nutrition = if source.tags.iter().any(|tag| tag == "vein") {
            base_nutrition.max(5_000)
        } else if source
            .tags
            .iter()
            .any(|tag| matches!(tag.as_str(), "diggable" | "door"))
        {
            base_nutrition
        } else {
            base_nutrition.max(10_000)
        };
        let nutrition_before = self.nutrition;
        self.increase_nutrition(nutrition);
        let index = self
            .index(position)
            .expect("planned consumed terrain must remain in bounds");
        self.terrain[index].clone_from(&target_terrain_id);
        self.revealed_terrain.remove(&position);
        changed.insert(position);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ConsumeTerrain {
                    effect_index: 0,
                    position,
                    source_terrain_id,
                    target_terrain_id,
                    nutrition_before,
                    nutrition_after: self.nutrition,
                }],
            },
            trace: None,
        });
        events.extend(self.relocate_player(position, changed));
    }

    fn resolve_player_transmute_item_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::TransmuteItemToGold {
            value_divisor,
            unit_value_cap,
        } = ability.effect
        else {
            unreachable!("item transmutation executor requires a transmute-item-to-gold effect");
        };
        let item = self
            .items
            .iter()
            .find(|item| item.id == item_id)
            .expect("planned transmutation item must remain available")
            .clone();
        let unit_value = self
            .content
            .item(&item.kind_id)
            .expect("planned transmutation item definition must remain available")
            .base_value
            .saturating_div(u32::from(value_divisor))
            .min(unit_value_cap);
        let requested = unit_value.saturating_mul(item.quantity);
        self.destroy_item(item_id, item.quantity)
            .expect("planned transmutation must remain valid");
        let before = self.gold;
        self.gold = self
            .gold
            .saturating_add(requested)
            .min(super::gold::MAX_PLAYER_GOLD);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::TransmuteItemToGold {
                    effect_index: 0,
                    item_id: item.id,
                    item_kind_id: item.kind_id,
                    quantity: item.quantity,
                    gold_gained: self.gold.saturating_sub(before),
                    gold_balance: self.gold,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_drain_item_magic_effect(
        &mut self,
        ability: &AbilityDefinition,
        item_id: &str,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::DrainItemMagic {
            base_power,
            level_multiplier,
            level_divisor,
        } = ability.effect
        else {
            unreachable!("magic drain executor requires a drain-item-magic effect");
        };
        let index = self
            .items
            .iter()
            .position(|item| item.id == item_id)
            .expect("planned magic drain item must remain available");
        let item_kind_id = self.items[index].kind_id.clone();
        let artifact = self
            .content
            .item(&item_kind_id)
            .is_some_and(|definition| definition.tags.iter().any(|tag| tag == "artifact"));
        let difficulty = self.items[index]
            .activation
            .as_ref()
            .map_or(0, |activation| {
                u32::try_from(activation.device_check_difficulty.max(0)).unwrap_or(0)
            });
        let charges_before = self.items[index]
            .charges
            .expect("planned magic drain item must retain charges")
            .current;
        let drained = difficulty.min(charges_before);
        let power = u32::from(base_power).saturating_add(
            u32::from(self.progress.level).saturating_mul(u32::from(level_multiplier))
                / u32::from(level_divisor),
        );
        let failure_odds = power.saturating_sub(difficulty / 2) / 5;
        let failed = failure_odds > 0 && self.rng.bounded(u64::from(failure_odds)) == 0;
        let mut destroyed = false;
        if failed && !artifact && self.rng.bounded(10) == 0 {
            if self.items[index].quantity == 1 {
                let removed = self.items.remove(index);
                self.item_property_knowledge.remove(&removed.id);
            } else {
                self.items[index].quantity -= 1;
            }
            destroyed = true;
        } else {
            let charges = self.items[index]
                .charges
                .as_mut()
                .expect("planned magic drain item must retain charges");
            charges.current = if failed {
                0
            } else {
                charges.current.saturating_sub(drained)
            };
        }
        let resource_id = self
            .casting_profile()
            .map(|profile| profile.resource_id.clone());
        let resource_before = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        if !failed
            && let Some(resource_id) = resource_id.as_deref()
            && let Some(pool) = self.resources.get_mut(resource_id)
        {
            pool.current = pool.current.saturating_add(drained).min(pool.maximum);
            self.resources_touched.insert(resource_id.to_owned());
        }
        let resource_after = resource_id
            .as_deref()
            .and_then(|id| self.resources.get(id))
            .map_or(0, |pool| pool.current);
        let charges_after = if destroyed {
            0
        } else {
            self.items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.charges)
                .map_or(0, |charges| charges.current)
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: Some(item_kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::DrainItemMagic {
                    effect_index: 0,
                    item_id: item_id.to_owned(),
                    item_kind_id,
                    charges_before,
                    charges_after,
                    drained: if failed { charges_before } else { drained },
                    destroyed,
                    failed,
                    resource_id,
                    resource_before,
                    resource_after,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_report_magic_effect(
        &self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let mut statuses = self
            .player
            .statuses
            .iter()
            .map(StatusInstance::to_dto)
            .collect::<Vec<_>>();
        statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::ReportMagic {
                    effect_index: 0,
                    statuses,
                    recall: self.recall.clone(),
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_earthquake_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::Earthquake {
            radius,
            affect_chance_percent,
            ref floor_terrain_id,
            ref wall_terrain_ids,
        } = ability.effect
        else {
            unreachable!("earthquake executor requires an earthquake effect");
        };
        let center = self.player.position;
        let radius_squared = i32::from(radius).pow(2);
        let mut affected_positions = Vec::new();
        for y in center.y - i32::from(radius)..=center.y + i32::from(radius) {
            for x in center.x - i32::from(radius)..=center.x + i32::from(radius) {
                let position = Position { x, y };
                let dx = x - center.x;
                let dy = y - center.y;
                if position == center
                    || dx.saturating_mul(dx).saturating_add(dy.saturating_mul(dy)) > radius_squared
                    || x <= 0
                    || y <= 0
                    || x >= i32::from(self.width) - 1
                    || y >= i32::from(self.height) - 1
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)
                {
                    continue;
                }
                if self.rng.bounded(100) < u64::from(affect_chance_percent) {
                    affected_positions.push(position);
                }
            }
        }
        let affected = affected_positions.iter().copied().collect::<BTreeSet<_>>();
        let removed_items = self
            .items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(position) if affected.contains(&position)))
            .count();
        self.items.retain(|item| {
            !matches!(item.location, ItemLocation::Ground(position) if affected.contains(&position))
        });
        let removed_gold_piles = self
            .gold_piles
            .iter()
            .filter(|pile| affected.contains(&pile.position))
            .count();
        self.gold_piles
            .retain(|pile| !affected.contains(&pile.position));

        let mut wall_positions = Vec::new();
        let mut floor_positions = Vec::new();
        for position in &affected_positions {
            let index = self
                .index(*position)
                .expect("planned earthquake position must remain in bounds");
            let actor_index = self
                .entities
                .iter()
                .position(|entity| entity.position == *position);
            if let Some(actor_index) = actor_index {
                let target_kind_id = self.entities[actor_index].kind_id.clone();
                let damage = resolve_damage(
                    DamagePacket::new(self.roll_damage(4, 8), DamageType::Physical),
                    self.entities[actor_index]
                        .resistances
                        .level(DamageType::Physical),
                );
                let application = plan_damage_application(
                    &self.entities[actor_index],
                    damage,
                    FatalityPolicy::AtOrBelowZero,
                );
                commit_damage_application(&mut self.entities[actor_index], &application);
                self.entities[actor_index].alerted = true;
                let trace = ProjectileTrace {
                    origin: center,
                    impact: *position,
                    landing: *position,
                    traversed: vec![*position],
                };
                events.push(DomainEvent::AbilityHit {
                    ability_id: ability.id.clone(),
                    target_kind_id: target_kind_id.clone(),
                    damage,
                    trace: trace.clone(),
                });
                self.wake_entity_after_damage(actor_index, damage.applied, events);
                if !application.fatal {
                    self.resolve_monster_fear_aura(actor_index, "hurt", true, events);
                }
                if application.fatal {
                    self.resolve_actor_death(
                        actor_index,
                        DomainEvent::AbilitySlew {
                            ability_id: ability.id.clone(),
                            target_kind_id,
                            damage,
                            trace,
                        },
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                self.terrain[index].clone_from(floor_terrain_id);
                floor_positions.push(*position);
            } else if self.is_walkable(*position) {
                let roll = self.rng.bounded(100);
                let wall_index = if wall_terrain_ids.len() == 1 || roll < 20 {
                    0
                } else if wall_terrain_ids.len() == 2 || roll < 70 {
                    1
                } else {
                    2
                };
                self.terrain[index].clone_from(&wall_terrain_ids[wall_index]);
                wall_positions.push(*position);
            } else {
                self.terrain[index].clone_from(floor_terrain_id);
                floor_positions.push(*position);
            }
            self.revealed_terrain.remove(position);
            changed.insert(*position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::Earthquake {
                    effect_index: 0,
                    radius,
                    affected_positions,
                    wall_positions,
                    floor_positions,
                    removed_items: u32::try_from(removed_items).unwrap_or(u32::MAX),
                    removed_gold_piles: u32::try_from(removed_gold_piles).unwrap_or(u32::MAX),
                }],
            },
            trace: None,
        });
        Ok(())
    }

    fn resolve_player_suppress_reproduction_effect(
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
        self.player.hp = self.player.hp.saturating_sub(damage);
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

    fn resolve_player_melee_then_teleport_effect(
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
        self.resolve_player_melee(index, events, changed, removed_entities)?;
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

    pub(super) fn resolve_player_polymorph_self_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        const ATTRIBUTES: [AttributeKind; 6] = [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ];

        let active_before = self.progress.active_mutation_ids.clone();
        let hp_before = self.player.hp;
        let previous_max_hp = self.effective_player_max_hp();
        let previous_resource_maxima = self.player_resource_maxima();
        let mut power = i32::from(self.progress.level);

        if power > i32::try_from(self.rng.bounded(30)).expect("polymorph roll must fit i32")
            && self.rng.bounded(6) == 0
        {
            power -= 20;
            for attribute in ATTRIBUTES {
                let amount = u8::try_from(self.rng.bounded(6) + 7)
                    .expect("polymorph attribute drain must fit u8");
                self.progress
                    .permanently_drain_attribute(attribute, amount, &mut self.rng);
            }
            if self.rng.bounded(6) == 0 {
                let dice = u16::try_from(self.rng.bounded(10) + 1)
                    .expect("polymorph life-loss dice must fit u16");
                self.player.hp = self
                    .player
                    .hp
                    .saturating_sub(self.roll_damage(dice, self.progress.level.max(1)));
                power -= 10;
            }
        }

        if power > i32::try_from(self.rng.bounded(20)).expect("polymorph roll must fit i32")
            && self.rng.bounded(4) == 0
        {
            power -= 10;
            let base_max_hp = self
                .progress
                .hp_progression
                .first()
                .copied()
                .unwrap_or(self.player.max_hp);
            self.progress.hp_progression =
                CharacterProgress::roll_hp_progression(base_max_hp, &mut self.rng);
        }

        while power > i32::try_from(self.rng.bounded(15)).expect("polymorph roll must fit i32")
            && self.rng.bounded(3) == 0
        {
            power -= 7;
            if self.gain_random_mutation_without_refresh(events).is_none() {
                break;
            }
        }

        if power > i32::try_from(self.rng.bounded(5)).expect("polymorph roll must fit i32") {
            power -= 5;
            self.resolve_polymorph_wounds(&ability.id, previous_max_hp);
        }

        let mut swapped_attributes = Vec::new();
        while power > 0 {
            let left_index = usize::try_from(self.rng.bounded(6))
                .expect("polymorph attribute index must fit usize");
            let mut right_index = usize::try_from(self.rng.bounded(5))
                .expect("polymorph attribute index must fit usize");
            if right_index >= left_index {
                right_index += 1;
            }
            let left = ATTRIBUTES[left_index];
            let right = ATTRIBUTES[right_index];
            let left_current = self.progress.attributes.value(left);
            let right_current = self.progress.attributes.value(right);
            let left_maximum = self.progress.maximum_attributes.value(left);
            let right_maximum = self.progress.maximum_attributes.value(right);
            let left_cap = self.progress.attribute_potentials.value(left);
            let right_cap = self.progress.attribute_potentials.value(right);
            let next_left_maximum = right_maximum.min(left_cap);
            let next_right_maximum = left_maximum.min(right_cap);
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                left,
                next_left_maximum,
            );
            set_attribute_value(
                &mut self.progress.maximum_attributes,
                right,
                next_right_maximum,
            );
            set_attribute_value(
                &mut self.progress.attributes,
                left,
                right_current.min(next_left_maximum),
            );
            set_attribute_value(
                &mut self.progress.attributes,
                right,
                left_current.min(next_right_maximum),
            );
            swapped_attributes.push(attribute_kind_dto(left));
            swapped_attributes.push(attribute_kind_dto(right));
            power -= 1;
        }

        self.refresh_after_attribute_change(previous_max_hp, &previous_resource_maxima);
        let active_after = &self.progress.active_mutation_ids;
        let gained_mutation_ids = active_after
            .difference(&active_before)
            .cloned()
            .collect::<Vec<_>>();
        let lost_mutation_ids = active_before
            .difference(active_after)
            .cloned()
            .collect::<Vec<_>>();
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::PolymorphSelf {
                    effect_index: 0,
                    gained_mutation_ids,
                    lost_mutation_ids,
                    swapped_attributes,
                    hp_before,
                    hp_after: self.player.hp,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_swap_position_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let (trace, target_index) = self.trace_projectile_path(path);
        let player_from = self.player.position;
        let mut target_entity_id = None;
        let mut target_from = None;
        if let Some(index) = target_index {
            let position = self.entities[index].position;
            target_entity_id = Some(self.entities[index].id.clone());
            target_from = Some(position);
            self.entities[index].position = player_from;
            self.player.position = position;
            changed.insert(player_from);
            changed.insert(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: target_entity_id.clone(),
                target_kind_id: target_entity_id.as_ref().and_then(|id| {
                    self.entities
                        .iter()
                        .find(|entity| &entity.id == id)
                        .map(|entity| entity.kind_id.clone())
                }),
                effects: vec![AbilityEffectResolutionDto::SwapPosition {
                    effect_index: 0,
                    target_entity_id,
                    player_from,
                    target_from,
                    swapped: target_from.is_some(),
                }],
            },
            trace: Some(trace),
        });
    }

    fn resolve_player_recall_effect(
        &mut self,
        ability: &AbilityDefinition,
        action: RecallUseAction,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::Recall {
            delay_dice,
            delay_sides,
            delay_bonus,
        } = ability.effect
        else {
            unreachable!("recall executor requires a recall effect");
        };
        let recall = self
            .recall
            .as_ref()
            .expect("planned recall must retain its destination")
            .clone();
        let (action_dto, delay) = match action {
            RecallUseAction::Start => {
                let rolled = self
                    .roll_damage(delay_dice, delay_sides)
                    .saturating_add(i32::from(delay_bonus));
                let rolled = u16::try_from(rolled.max(1)).expect("validated recall delay fits u16");
                let delay = self.debug_recall_delay_turns.unwrap_or(rolled).max(1);
                self.start_recall(delay);
                (AbilityRecallActionDto::Start, Some(delay))
            }
            RecallUseAction::Cancel => {
                self.cancel_recall();
                (AbilityRecallActionDto::Cancel, None)
            }
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::Recall {
                    effect_index: 0,
                    action: action_dto,
                    delay,
                    dungeon_id: recall.dungeon_id,
                    floor_id: recall.floor_id,
                }],
            },
            trace: None,
        });
    }

    fn resolve_player_resist_elements_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        let AbilityEffectDefinition::ResistElements {
            duration_dice,
            duration_sides,
            duration_bonus,
        } = ability.effect
        else {
            unreachable!("resist elements executor requires a resist elements effect");
        };
        let rolled_duration = (0..duration_dice).fold(duration_bonus, |total, _| {
            total.saturating_add(
                u32::try_from(self.rng.bounded(u64::from(duration_sides)) + 1)
                    .expect("validated resistance duration roll must fit u32"),
            )
        });
        let mut remaining = self.progress.level / 10;
        let candidates = [
            (5_u16, ActorDamageType::Acid, "rfb.status.resist-acid"),
            (
                4,
                ActorDamageType::Electricity,
                "rfb.status.resist-electricity",
            ),
            (3, ActorDamageType::Fire, "rfb.status.resist-fire"),
            (2, ActorDamageType::Cold, "rfb.status.resist-cold"),
            (1, ActorDamageType::Poison, "rfb.status.resist-poison"),
        ];
        let empty_brands = BTreeSet::new();
        let empty_immunities = BTreeSet::new();
        let mut resolutions = Vec::new();
        for (denominator, damage_type, status_kind_id) in candidates {
            if remaining == 0 || self.rng.bounded(u64::from(denominator)) >= u64::from(remaining) {
                continue;
            }
            remaining -= 1;
            let mut resistances = BTreeMap::new();
            resistances.insert(damage_type, ActorResistanceLevel::Resistant);
            let effect_index = u8::try_from(resolutions.len())
                .expect("elemental resistance effect count must fit u8");
            resolutions.push(apply_ability_status_effect(
                &mut self.player,
                &ability.id,
                effect_index,
                status_kind_id,
                1,
                rolled_duration,
                0,
                0,
                AbilityStatusStackingDefinition::Replace,
                None,
                None,
                &resistances,
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
            ));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: resolutions,
            },
            trace: None,
        });
    }

    fn resolve_player_aggravate_monsters_effect(
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
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            self.maybe_initialize_chameleon_form(&mut entity);
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

    pub(super) fn resolve_player_genocide_effect(
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

    pub(super) fn animate_dead_candidates(
        &self,
        origin: Position,
        actor_kind_id: &str,
        corpse_item_kind_id: &str,
        radius: u8,
        count: u8,
    ) -> Vec<(String, Position)> {
        let mut corpses = self
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position)
                    if item.kind_id == corpse_item_kind_id
                        && chebyshev_distance(origin, position) <= u32::from(radius)
                        && self.actor_kind_can_enter_position(actor_kind_id, position) =>
                {
                    Some((
                        chebyshev_distance(origin, position),
                        position.y,
                        position.x,
                        item.id.clone(),
                        position,
                    ))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        corpses.sort();
        corpses.truncate(usize::from(count));
        corpses
            .into_iter()
            .map(|(_, _, _, item_id, position)| (item_id, position))
            .collect()
    }

    pub(super) fn resolve_player_animate_dead_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        let AbilityEffectDefinition::AnimateDead {
            actor_kind_id,
            corpse_item_kind_id,
            radius,
            count,
            failure_chance_percent,
        } = &ability.effect
        else {
            unreachable!("animate dead executor requires an animate dead effect");
        };
        let origin = self.player.position;
        let definition = self
            .content
            .actor(actor_kind_id)
            .expect("validated animated actor must remain available")
            .clone();
        let corpses = self.animate_dead_candidates(
            origin,
            actor_kind_id,
            corpse_item_kind_id,
            *radius,
            *count,
        );
        let consumed_corpse_item_ids = corpses
            .iter()
            .map(|corpse| corpse.0.clone())
            .collect::<Vec<_>>();
        self.items
            .retain(|item| !consumed_corpse_item_ids.contains(&item.id));
        for item_id in &consumed_corpse_item_ids {
            self.item_property_knowledge.remove(item_id);
        }
        let mut entity_ids = Vec::with_capacity(corpses.len());
        let mut positions = Vec::with_capacity(corpses.len());
        for (ordinal, (_, position)) in corpses.into_iter().enumerate() {
            changed.insert(position);
            if *failure_chance_percent > 0
                && self.rng.bounded(100) < u64::from(*failure_chance_percent)
            {
                continue;
            }
            let id = self.summon_entity_id(&ability.id, ordinal);
            let mut entity = spawn_actor_from_definition(
                &mut self.rng,
                &definition,
                &id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                true,
            );
            entity.controller_id = Some(self.player.id.clone());
            self.entities.push(entity);
            entity_ids.push(id);
            positions.push(position);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::AnimateDead {
                    effect_index: 0,
                    actor_kind_id: actor_kind_id.clone(),
                    consumed_corpse_item_ids,
                    entity_ids,
                    positions,
                }],
            },
            trace: None,
        });
        Ok(())
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
                let detected = self.detect_item_positions(category, *radius, false);
                self.mark_item_instances_discovered(&detected.1);
                detected
            }
            AbilityDetectSubjectDefinition::Gold => {
                let detected = self.detect_gold_positions(*radius, false);
                self.mark_gold_piles_discovered(&detected.1);
                detected
            }
            AbilityDetectSubjectDefinition::Curse => {
                let mut item_ids = self
                    .items
                    .iter()
                    .filter(|item| {
                        item.curse.is_some()
                            && matches!(
                                item.location,
                                ItemLocation::Inventory | ItemLocation::Equipped { .. }
                            )
                    })
                    .map(|item| item.id.clone())
                    .collect::<Vec<_>>();
                item_ids.sort();
                for item_id in &item_ids {
                    self.identify_item_instance(item_id, ItemIdentificationRequest::new(false));
                }
                (
                    (!item_ids.is_empty())
                        .then_some(self.player.position)
                        .into_iter()
                        .collect(),
                    item_ids,
                )
            }
        };
        if *persistent
            || matches!(
                subject,
                AbilityDetectSubjectDefinition::Item
                    | AbilityDetectSubjectDefinition::Gold
                    | AbilityDetectSubjectDefinition::Curse
            )
        {
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

    pub(super) fn resolve_terrain_transform_effect(
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
            AbilityEffectDefinition::BlinkTarget { .. }
            | AbilityEffectDefinition::TeleportSelf { .. }
            | AbilityEffectDefinition::TeleportTarget
            | AbilityEffectDefinition::TeleportLevel
            | AbilityEffectDefinition::BreathDamage { .. }
            | AbilityEffectDefinition::CurseDamage { .. }
            | AbilityEffectDefinition::TeleportAway { .. }
            | AbilityEffectDefinition::DrainResource { .. }
            | AbilityEffectDefinition::Amnesia
            | AbilityEffectDefinition::DarkenRoom
            | AbilityEffectDefinition::JumpDamage { .. }
            | AbilityEffectDefinition::PolymorphTarget => None,
            AbilityEffectDefinition::Teleport => {
                let TargetSelection::Position { position } = target else {
                    return None;
                };
                self.teleport_destination(ability, *position)
                    .map(|destination| AbilityTargetPlan::Teleport { destination })
            }
            AbilityEffectDefinition::BlinkSelf { radius } => {
                if !matches!(target, TargetSelection::SelfTarget)
                    || !ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                {
                    return None;
                }
                let candidates = self.random_teleport_candidates(u16::from(radius));
                (!candidates.is_empty()).then_some(AbilityTargetPlan::RandomTeleport { candidates })
            }
            AbilityEffectDefinition::FetchItem { .. } => self
                .ability_path(ability, target)
                .map(|path| AbilityTargetPlan::FetchItem { path }),
            AbilityEffectDefinition::ConsumeTerrain { .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                if !ability
                    .target
                    .modes
                    .contains(&AbilityTargetModeDefinition::Direction)
                {
                    return None;
                }
                let position = self.position_in_direction(*direction);
                let index = self.index(position)?;
                if self
                    .entities
                    .iter()
                    .any(|entity| entity.position == position)
                    || self
                        .floor_connections
                        .iter()
                        .any(|connection| connection.position == position)
                {
                    return None;
                }
                let terrain = self.content.terrain(&self.terrain[index])?;
                if terrain
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "permanent" | "tree" | "glass"))
                {
                    return None;
                }
                let target_terrain_id = terrain
                    .dig_to_terrain_id
                    .as_ref()
                    .or(terrain.monster_destroy_to_terrain_id.as_ref())?
                    .clone();
                Some(AbilityTargetPlan::ConsumeTerrain {
                    position,
                    source_terrain_id: terrain.id.clone(),
                    target_terrain_id,
                })
            }
            AbilityEffectDefinition::TransmuteItemToGold { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id
                            && (item.location == ItemLocation::Inventory
                                || item.location == ItemLocation::Ground(self.player.position))
                            && self.can_destroy_item(item).is_ok()
                    })
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::DrainItemMagic { .. } => {
                let TargetSelection::Item { item_id } = target else {
                    return None;
                };
                self.items
                    .iter()
                    .find(|item| {
                        item.id == *item_id
                            && (item.location == ItemLocation::Inventory
                                || item.location == ItemLocation::Ground(self.player.position))
                            && item.charges.is_some_and(|charges| charges.current > 0)
                    })
                    .map(|_| AbilityTargetPlan::Item {
                        item_id: item_id.clone(),
                    })
            }
            AbilityEffectDefinition::MeleeThenTeleport { radius, .. } => {
                let TargetSelection::Direction { direction } = target else {
                    return None;
                };
                let position = self.position_in_direction(*direction);
                let target_entity_id = self
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.position == position && !self.actor_is_player_side(entity)
                    })?
                    .id
                    .clone();
                Some(AbilityTargetPlan::MeleeThenTeleport {
                    target_entity_id,
                    teleport_candidates: self.random_teleport_candidates(u16::from(radius)),
                })
            }
            AbilityEffectDefinition::SwapPosition => {
                self.ability_path(ability, target)
                    .map(|path| AbilityTargetPlan::Projectile {
                        path,
                        stop_at_actor: true,
                    })
            }
            AbilityEffectDefinition::Recall { .. } => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then(|| self.recall_use_plan())
                .flatten()
                .map(|action| AbilityTargetPlan::Recall { action })
            }
            AbilityEffectDefinition::ResistElements { .. }
            | AbilityEffectDefinition::ReportMagic
            | AbilityEffectDefinition::SuppressMonsterReproduction { .. }
            | AbilityEffectDefinition::PolymorphSelf => {
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget))
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::Earthquake { .. } => {
                let world = self.content.world(&self.world_id)?;
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                    && floor_dungeon_id(world, &self.current_floor_id).is_some())
                .then_some(AbilityTargetPlan::SelfTarget)
            }
            AbilityEffectDefinition::Summon {
                ref actor_kind_id,
                count,
                radius,
                ..
            } => {
                let unique = self.content.actor(actor_kind_id).is_some_and(|definition| {
                    definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "unique" | "unique2"))
                });
                (matches!(target, TargetSelection::SelfTarget)
                    && ability
                        .target
                        .modes
                        .contains(&AbilityTargetModeDefinition::SelfTarget)
                    && (!unique || self.unique_actor_kind_is_available(actor_kind_id)))
                .then(|| {
                    self.summon_positions_around(
                        self.player.position,
                        if unique { 1 } else { count },
                        radius,
                        actor_kind_id,
                    )
                })
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
                let position_candidate_kind_ids = friendly_candidate_kind_ids
                    .iter()
                    .chain(&hostile_candidate_kind_ids)
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let positions = self
                    .open_positions_around_for_actor_kinds(
                        self.player.position,
                        radius,
                        &position_candidate_kind_ids,
                    )
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
                                ItemLocation::CarriedBy { .. }
                                | ItemLocation::Shop { .. }
                                | ItemLocation::Home { .. } => false,
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
            | AbilityEffectDefinition::AggravateMonsters
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
            AbilityEffectDefinition::BoltOrAreaDamage { .. } => self
                .ability_path(ability, target)
                .map(|path| AbilityTargetPlan::Projectile {
                    path,
                    stop_at_actor: matches!(target, TargetSelection::Direction { .. }),
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
