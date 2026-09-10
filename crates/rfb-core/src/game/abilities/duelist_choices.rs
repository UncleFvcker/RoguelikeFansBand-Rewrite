// SPDX-License-Identifier: MPL-2.0

use crate::game::chaos_patron::CHAOS_GIFT_MUTATION_ID;
use crate::game::projectile_geometry::rfb_distance;
use crate::game::visibility::has_line_of_sight;
use crate::game::*;
use rfb_protocol::{
    DuelistChoiceDto, DuelistCommandCompletionDto, DuelistContinuationDto as Continue,
    DuelistPromptDto, PendingDuelistDto,
};

impl Game {
    pub(in crate::game) fn pending_duelist_is_valid(&self) -> bool {
        let Some(pending) = &self.pending_duelist else {
            return true;
        };
        let Some(completion) = &pending.command_completion else {
            return false;
        };
        let Some(world) = self.content.world(&self.world_id) else {
            return false;
        };
        let valid_floor = |id: &str| {
            id == world.initial_floor_id
                || (id == wilderness::WILDERNESS_FLOOR_ID && world.wilderness.is_some())
                || world.procedural_floors.iter().any(|floor| floor.id == id)
        };
        let actor = |id: &str| {
            self.entities
                .iter()
                .find(|actor| actor.id == id && actor.hp > 0)
        };
        let unique = |ids: &[String]| ids.iter().collect::<BTreeSet<_>>().len() == ids.len();
        let rest_turns = pending.continuations.iter().find_map(|frame| match frame {
            Continue::RestRecovery { completed_turns } => Some(*completed_turns),
            _ => None,
        });
        if self.player_is_dead()
            || completion.turn_advance > u32::from(rest_turns.unwrap_or(1))
            || completion.world_tick_before > self.world_tick
            || !unique(&completion.nice_entity_ids)
            || completion.actor_deaths.iter().any(|death| {
                death.actor_id.is_empty()
                    || self.content.actor(&death.actor_kind_id).is_none()
                    || actor(&death.actor_id).is_some()
            })
            || !unique(
                &completion
                    .actor_deaths
                    .iter()
                    .map(|death| death.actor_id.clone())
                    .collect::<Vec<_>>(),
            )
            || completion
                .picked_up_kind_ids
                .iter()
                .any(|id| self.content.item(id).is_none())
            || completion
                .entered_floor_ids
                .iter()
                .any(|id| !valid_floor(id))
            || pending.continuations.len() > 64
        {
            return false;
        }
        let prompt_valid = match &pending.prompt {
            None => self.pending_mutation_direction.is_some() && !pending.continuations.is_empty(),
            Some(prompt @ DuelistPromptDto::Charge { ability_id, .. }) => {
                pending.continuations.is_empty()
                    && completion.turn_advance == 0
                    && self
                        .duelist_charge_prompt(ability_id, &TargetSelection::SelfTarget)
                        .as_ref()
                        == Some(prompt)
            }
            Some(DuelistPromptDto::Challenge) => {
                self.player_is_duelist()
                    && self.progress.level >= 35
                    && self.duelist_equipment_error().is_none()
                    && self.duelist_target_id.is_none()
                    && self.entities.iter().any(|target| {
                        target.hp > 0
                            && self.riding_actor_id.as_ref() != Some(&target.id)
                            && self.entity_is_visible_to_player(target)
                            && rfb_distance(self.player.position, target.position) <= 18
                    })
                    && !pending.continuations.is_empty()
            }
            Some(DuelistPromptDto::BlockTeleport { source_entity_id }) => {
                self.progress.level >= 30
                    && self.duelist_opponent(source_entity_id)
                    && actor(source_entity_id).is_some()
                    && matches!(pending.continuations.first(), Some(Continue::MonsterTeleport { source_entity_id: id, blocked: false, .. }) if id == source_entity_id)
            }
            Some(DuelistPromptDto::FollowTeleport { source_entity_id }) => {
                self.player_is_duelist()
                    && actor(source_entity_id).is_some()
                    && ((self.progress.level >= 30 && self.duelist_opponent(source_entity_id))
                        || self.player_has_mutation("rfb.mutation.teleport")
                        || self.items.iter().any(|item| {
                            matches!(item.location, ItemLocation::Equipped { .. })
                                && item.curse.is_none()
                                && self.item_has_intrinsic_curse_effect(
                                    item,
                                    ItemCurseEffectDto::Teleport,
                                )
                        }))
                    && matches!(pending.continuations.first(), Some(Continue::MonsterCast { resolution, .. }) if resolution.source_entity_id == *source_entity_id)
            }
        };
        if !prompt_valid {
            return false;
        }
        let mut seen = BTreeSet::new();
        let mut previous_kind = 0;
        pending.continuations.iter().all(|frame| {
            let (kind, valid) = match frame {
                Continue::ChaosReward { level } => (0, (1..=50).contains(level) && self.progress.active_mutation_ids.contains(CHAOS_GIFT_MUTATION_ID)),
                Continue::Melee { impact_item_id } => (1, self.items.iter().any(|item| item.id == *impact_item_id)),
                Continue::Charge { ability_id, target_entity_id, floor_id, .. } => (2, !target_entity_id.is_empty() && valid_floor(floor_id)
                    && self.content.ability(ability_id).is_some_and(|ability| Self::duelist_charge_range(&ability.effect).is_some())),
                Continue::ClassCast { resolution, hit_point_cost } => (3, self.class_ability_activation(&resolution.ability_id).is_some_and(|activation| {
                    *hit_point_cost == self.class_ability_hit_point_cost(activation) && resolution.succeeded && resolution.hp_paid == 0
                        && resolution.failure_percent == 0 && resolution.percentile_roll < 100 && resolution.resource_paid == 0
                        && self.progress.level >= activation.minimum_level
                        && resolution.resource_id.is_none() && resolution.base_resource_cost == 0 && resolution.resource_cost == 0
                        && resolution.resource_before == 0 && resolution.resource_after == 0
                        && pending.continuations.iter().any(|frame| matches!(frame, Continue::Charge { ability_id, .. } if *ability_id == resolution.ability_id))
                })),
                Continue::MeleeTeleport { ability_id, target_entity_id, target_kind_id, floor_id, player_from, candidates } => (4, !target_entity_id.is_empty() && valid_floor(floor_id)
                    && self.content.actor(target_kind_id).is_some() && self.index(*player_from).is_some()
                    && candidates.iter().all(|position| self.index(*position).is_some())
                    && self.content.ability(ability_id).is_some_and(|ability| matches!(ability.effect, AbilityEffectDefinition::MeleeThenTeleport { .. }))),
                Continue::AdjacentMelee { directions, floor_id } => (5, valid_floor(floor_id) && directions.len() <= 8
                    && directions.iter().enumerate().all(|(index, direction)| !directions[..index].contains(direction))),
                Continue::MonsterTeleport { source_entity_id, ability_id, .. } => (6, actor(source_entity_id).is_some()
                    && self.content.ability(ability_id).is_some_and(|ability| matches!(ability.effect, AbilityEffectDefinition::TeleportAway { .. }))),
                Continue::MonsterCast { resolution, player_hp_before } => (7, actor(&resolution.source_entity_id).is_some_and(|source| source.kind_id == resolution.source_kind_id)
                    && *player_hp_before >= 0 && *player_hp_before <= self.effective_player_max_hp()
                    && self.content.ability(&resolution.ability_id).is_some_and(|ability| matches!(ability.effect, AbilityEffectDefinition::TeleportSelf { .. }))
                    && resolution.summon.is_none() && resolution.effects.is_empty() && resolution.targets.is_empty()
                    && resolution.affected_positions.iter().all(|position| self.index(*position).is_some())),
                Continue::MonsterWorld { source_entity_id, remaining_actions, floor_id, surround_reservations, .. } => (8, !source_entity_id.is_empty()
                    && *remaining_actions <= 3 && valid_floor(floor_id) && surround_reservations.iter().all(|position| self.index(*position).is_some())),
                Continue::MonsterPulse { remaining_entity_ids, floor_id, surround_reservations, .. } => (9, unique(remaining_entity_ids)
                    && remaining_entity_ids.iter().all(|id| !id.is_empty()) && valid_floor(floor_id)
                    && surround_reservations.iter().all(|position| self.index(*position).is_some())),
                Continue::WorldTick { .. } => (10, true),
                Continue::PlayerAction { energy_cost, .. } => (11, (1..=300).contains(energy_cost)),
                Continue::PlayerWorld { .. } => (12, true),
                Continue::RestRecovery { completed_turns } => (13, (1..=MAX_REST_TURNS).contains(completed_turns)),
            };
            let ordered = kind == 0 || kind >= previous_kind;
            if kind != 0 { previous_kind = kind; }
            valid && ordered && (kind == 0 || seen.insert(kind))
        })
    }

    pub(in crate::game) fn duelist_prompt(&self) -> Option<&DuelistPromptDto> {
        self.pending_duelist
            .as_ref()
            .and_then(|pending| pending.prompt.as_ref())
    }

    pub(in crate::game) fn begin_duelist_choice(&mut self, prompt: DuelistPromptDto) {
        assert!(
            self.duelist_prompt().is_none(),
            "finish the current choice before opening another"
        );
        self.pending_duelist
            .get_or_insert(PendingDuelistDto {
                prompt: None,
                continuations: Vec::new(),
                command_completion: None,
            })
            .prompt = Some(prompt);
    }

    pub(in crate::game) fn continue_after_duelist_choice(&mut self, continuation: Continue) {
        self.pending_duelist
            .get_or_insert(PendingDuelistDto {
                prompt: None,
                continuations: Vec::new(),
                command_completion: None,
            })
            .continuations
            .push(continuation);
    }

    pub(in crate::game) fn duelist_charge_prompt(
        &self,
        ability_id: &str,
        target: &TargetSelection,
    ) -> Option<DuelistPromptDto> {
        if !self.player_is_duelist()
            || self.ability_state_unavailable_reason(ability_id).is_some()
            || self.duelist_cast_is_zero_time_unavailable(ability_id, target)
        {
            return None;
        }
        let ability = self.content.ability(ability_id)?;
        let range = Self::duelist_charge_range(&ability.effect)?;
        let opponent = self
            .entities
            .iter()
            .find(|actor| self.duelist_opponent(&actor.id))?;
        let distance = rfb_distance(self.player.position, opponent.position);
        (distance > u32::from(range)).then(|| DuelistPromptDto::Charge {
            ability_id: ability_id.to_owned(),
            target_entity_id: opponent.id.clone(),
            distance,
            range,
        })
    }

    pub(in crate::game) fn begin_duelist_endless_challenge(&mut self) {
        if self.player_is_dead()
            || !self.player_is_duelist()
            || self.progress.level < 35
            || self.duelist_equipment_error().is_some()
        {
            return;
        }
        if self.entities.iter().any(|actor| {
            actor.hp > 0
                && self.riding_actor_id.as_ref() != Some(&actor.id)
                && self.entity_is_visible_to_player(actor)
                && rfb_distance(self.player.position, actor.position) <= 18
        }) {
            self.begin_duelist_choice(DuelistPromptDto::Challenge);
        }
    }

    pub(in crate::game) fn duelist_can_follow_teleport(
        &self,
        index: usize,
        world_stopped: bool,
    ) -> bool {
        self.player_is_duelist()
            && !world_stopped
            && self.entity_is_visible_to_player(&self.entities[index])
            && rfb_distance(self.player.position, self.entities[index].position) <= 20
            && has_line_of_sight(self, self.player.position, self.entities[index].position)
            && ((self.progress.level >= 30 && self.duelist_opponent(&self.entities[index].id))
                || self.player_has_mutation("rfb.mutation.teleport")
                || self.items.iter().any(|item| {
                    matches!(item.location, ItemLocation::Equipped { .. })
                        && item.curse.is_none()
                        && self.item_has_intrinsic_curse_effect(item, ItemCurseEffectDto::Teleport)
                }))
    }

    pub(in crate::game) fn validate_duelist_choice(
        &self,
        choice: &DuelistChoiceDto,
    ) -> Result<(), CoreError> {
        let valid = match (self.duelist_prompt(), choice) {
            (
                Some(
                    DuelistPromptDto::Charge { .. }
                    | DuelistPromptDto::BlockTeleport { .. }
                    | DuelistPromptDto::FollowTeleport { .. },
                ),
                DuelistChoiceDto::Confirm { .. },
            ) => true,
            (
                Some(DuelistPromptDto::Challenge),
                DuelistChoiceDto::Challenge { entity_id: None },
            ) => true,
            (
                Some(DuelistPromptDto::Challenge),
                DuelistChoiceDto::Challenge {
                    entity_id: Some(id),
                },
            ) => {
                self.duelist_challenge_target(&TargetSelection::Entity {
                    entity_id: id.clone(),
                })
                .is_some()
                    && self
                        .entities
                        .iter()
                        .find(|actor| actor.id == *id)
                        .is_some_and(|actor| {
                            rfb_distance(self.player.position, actor.position) <= 18
                        })
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(CoreError::DuelistChoiceUnavailable)
        }
    }

    pub(in crate::game) fn resolve_duelist_choice(
        &mut self,
        choice: DuelistChoiceDto,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        chaos_cursor: &mut usize,
    ) -> Result<Option<DuelistCommandCompletionDto>, CoreError> {
        self.validate_duelist_choice(&choice)?;
        let prompt = self
            .pending_duelist
            .as_mut()
            .expect("validated choice")
            .prompt
            .take()
            .expect("validated prompt");
        match (prompt, choice) {
            (DuelistPromptDto::Challenge, DuelistChoiceDto::Challenge { entity_id }) => {
                if let Some(id) = entity_id {
                    self.resolve_duelist_challenge(id, events);
                }
            }
            (
                DuelistPromptDto::Charge { ability_id, .. },
                DuelistChoiceDto::Confirm { accepted: true },
            ) => {
                let visible_auras_before =
                    self.visible_monster_aura_entity_ids().into_iter().collect();
                self.decrement_ability_cooldowns(1);
                self.resolve_player_ability(
                    &ability_id,
                    TargetSelection::SelfTarget,
                    events,
                    changed,
                    removed_entities,
                )?;
                if let Some(completion) = self
                    .pending_duelist
                    .as_mut()
                    .and_then(|pending| pending.command_completion.as_mut())
                {
                    completion.turn_advance = 1;
                }
                self.continue_after_duelist_choice(Continue::PlayerAction {
                    energy_cost: STANDARD_ACTION_COST,
                    recover_after_wait: false,
                    pet_neglect_allowed: self.pet_upkeep().unsafe_warning(),
                    visible_auras_before,
                });
            }
            (DuelistPromptDto::Charge { .. }, DuelistChoiceDto::Confirm { accepted: false }) => {}
            (
                DuelistPromptDto::BlockTeleport { source_entity_id },
                DuelistChoiceDto::Confirm { accepted },
            ) => {
                let blocked = accepted && self.rng.bounded(3) != 0;
                if let Some(Continue::MonsterTeleport { blocked: saved, .. }) = self
                    .pending_duelist
                    .as_mut()
                    .and_then(|pending| pending.continuations.first_mut())
                {
                    *saved = blocked;
                }
                if accepted {
                    let source_kind_id = self
                        .entities
                        .iter()
                        .find(|actor| actor.id == source_entity_id)
                        .expect("pending caster exists")
                        .kind_id
                        .clone();
                    events.push(DomainEvent::DuelistTeleportBlocked {
                        source_kind_id,
                        succeeded: blocked,
                    });
                }
            }
            (
                DuelistPromptDto::FollowTeleport { source_entity_id },
                DuelistChoiceDto::Confirm { accepted },
            ) => {
                if accepted {
                    let ability_id = self
                        .pending_duelist
                        .as_ref()
                        .expect("pending follow")
                        .continuations
                        .iter()
                        .find_map(|frame| match frame {
                            Continue::MonsterCast { resolution, .. } => {
                                Some(resolution.ability_id.clone())
                            }
                            _ => None,
                        })
                        .expect("follow retains its monster cast");
                    let actor = self
                        .entities
                        .iter()
                        .find(|actor| actor.id == source_entity_id)
                        .expect("pending caster exists");
                    let destination = actor.position;
                    let source_kind_id = actor.kind_id.clone();
                    let succeeded = self.rng.bounded(3) != 0;
                    if succeeded {
                        self.duelist_teleport_near(&ability_id, destination, events, changed);
                    } else {
                        self.resolve_player_teleport_with_range(
                            &ability_id,
                            200,
                            false,
                            true,
                            None,
                            events,
                            changed,
                        );
                    }
                    spend_energy(&mut self.player.energy_need, STANDARD_ACTION_COST);
                    events.push(DomainEvent::DuelistFollowedTeleport {
                        source_kind_id,
                        succeeded,
                    });
                }
            }
            _ => unreachable!("validated duelist choice matches prompt"),
        }
        self.resume_duelist_continuations(events, changed, removed_entities, chaos_cursor)
    }

    fn duelist_teleport_near(
        &mut self,
        ability_id: &str,
        target: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.player_has_anti_teleport() {
            return;
        }
        let mut candidates = Vec::new();
        let mut minimum = u32::MAX;
        for y in 1..self.height.saturating_sub(1) {
            for x in 1..self.width.saturating_sub(1) {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                let distance = rfb_distance(target, position);
                if distance <= minimum && self.player_can_teleport_to(position, false) {
                    if distance < minimum {
                        candidates.clear();
                        minimum = distance;
                    }
                    candidates.push(position);
                }
            }
        }
        if !candidates.is_empty() {
            let destination = candidates[self.rng.bounded(candidates.len() as u64) as usize];
            let from = self.player.position;
            events.push(DomainEvent::AbilityTeleported {
                ability_id: ability_id.to_owned(),
                resolution: AbilityTeleportResolutionDto {
                    from,
                    to: destination,
                },
            });
            events.extend(self.relocate_player(destination, changed));
        }
    }

    pub(in crate::game) fn resume_duelist_continuations(
        &mut self,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
        chaos_cursor: &mut usize,
    ) -> Result<Option<DuelistCommandCompletionDto>, CoreError> {
        if self.duelist_prompt().is_some() || self.pending_mutation_direction.is_some() {
            return Ok(None);
        }
        let Some(mut pending) = self.pending_duelist.take() else {
            return Ok(None);
        };
        while !pending.continuations.is_empty() {
            let continuation = pending.continuations.remove(0);
            match continuation {
                Continue::ChaosReward { level } => {
                    if !self.player_is_dead() {
                        self.resolve_chaos_patron_level_reward(
                            level,
                            events,
                            changed,
                            removed_entities,
                        )?;
                        self.process_chaos_patron_level_rewards(
                            events,
                            chaos_cursor,
                            changed,
                            removed_entities,
                        )?;
                    }
                }
                Continue::Melee { impact_item_id } => {
                    if !self.player_is_dead() {
                        self.resolve_player_impact_earthquake(
                            impact_item_id,
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                }
                Continue::Charge {
                    ability_id,
                    target_entity_id,
                    floor_id,
                    succeeded,
                } => {
                    self.finish_duelist_charge(
                        &ability_id,
                        &target_entity_id,
                        &floor_id,
                        succeeded,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::ClassCast {
                    mut resolution,
                    hit_point_cost,
                } => {
                    resolution.hp_paid = self.pay_class_ability_hit_points(hit_point_cost);
                    events.push(DomainEvent::AbilityCastSucceeded { resolution });
                }
                Continue::MeleeTeleport {
                    ability_id,
                    target_entity_id,
                    target_kind_id,
                    floor_id,
                    player_from,
                    candidates,
                } => {
                    let ability = self
                        .content
                        .ability(&ability_id)
                        .expect("pending melee ability exists")
                        .clone();
                    self.finish_player_melee_then_teleport_effect(
                        &ability,
                        &target_entity_id,
                        target_kind_id,
                        &floor_id,
                        player_from,
                        candidates,
                        events,
                        changed,
                    );
                }
                Continue::AdjacentMelee {
                    directions,
                    floor_id,
                } => {
                    self.continue_player_melee_adjacent_effect(
                        directions,
                        &floor_id,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::MonsterTeleport {
                    source_entity_id,
                    ability_id,
                    blocked,
                } => {
                    self.resume_duelist_monster_teleport(
                        &source_entity_id,
                        &ability_id,
                        blocked,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::MonsterCast {
                    resolution,
                    player_hp_before,
                } => {
                    let source_entity_id = resolution.source_entity_id.clone();
                    events.push(DomainEvent::MonsterAbilityCast {
                        resolution,
                        trace: None,
                    });
                    self.resolve_vengeance_retaliation(
                        &source_entity_id,
                        player_hp_before.saturating_sub(self.player.hp),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::MonsterWorld {
                    source_entity_id,
                    remaining_actions,
                    floor_id,
                    surround_reservations,
                    visible_auras_before,
                } => {
                    self.resolve_newly_visible_monster_auras(
                        &visible_auras_before.into_iter().collect(),
                        events,
                        changed,
                    );
                    self.continue_monster_world_actions(
                        &source_entity_id,
                        remaining_actions,
                        &floor_id,
                        surround_reservations.into_iter().collect(),
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::MonsterPulse {
                    remaining_entity_ids,
                    floor_id,
                    surround_reservations,
                    visible_auras_before,
                    pet_neglect_allowed,
                } => {
                    self.resolve_newly_visible_monster_auras(
                        &visible_auras_before.into_iter().collect(),
                        events,
                        changed,
                    );
                    if self.current_floor_id == floor_id {
                        self.continue_monster_energy_pulse(
                            remaining_entity_ids,
                            surround_reservations.into_iter().collect(),
                            pet_neglect_allowed,
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                }
                Continue::WorldTick {
                    resting,
                    local_floor_active,
                    pet_neglect_allowed,
                } => {
                    self.resume_duelist_world_tick(
                        resting,
                        local_floor_active,
                        pet_neglect_allowed,
                        events,
                        changed,
                        removed_entities,
                    )?;
                }
                Continue::PlayerAction {
                    energy_cost,
                    recover_after_wait,
                    pet_neglect_allowed,
                    visible_auras_before,
                } => {
                    self.resolve_newly_visible_monster_auras(
                        &visible_auras_before.into_iter().collect(),
                        events,
                        changed,
                    );
                    if !self.player_is_dead() {
                        events.extend(self.resolve_wilderness_terrain_hazard(self.player.position));
                    }
                    spend_energy(&mut self.player.energy_need, energy_cost);
                    if !self.player_is_dead() {
                        self.advance_until_player_ready(
                            false,
                            self.map_scale != MapScaleDto::World,
                            pet_neglect_allowed,
                            events,
                            changed,
                            removed_entities,
                        )?;
                    }
                    if self.pending_duelist.is_some() || self.pending_mutation_direction.is_some() {
                        self.continue_after_duelist_choice(Continue::PlayerWorld {
                            recover_after_wait,
                        });
                    } else {
                        self.finish_duelist_player_world(recover_after_wait, events);
                    }
                }
                Continue::PlayerWorld { recover_after_wait } => {
                    self.finish_duelist_player_world(recover_after_wait, events)
                }
                Continue::RestRecovery { completed_turns } => {
                    self.decrement_ability_cooldowns(completed_turns);
                    if !self.player_is_dead() {
                        self.recover_player_resources(true, events);
                    }
                }
            }
            if self.duelist_prompt().is_some() || self.pending_mutation_direction.is_some() {
                let next = self.pending_duelist.get_or_insert(PendingDuelistDto {
                    prompt: None,
                    continuations: Vec::new(),
                    command_completion: None,
                });
                next.continuations.extend(pending.continuations);
                next.command_completion = pending.command_completion;
                return Ok(None);
            }
        }
        Ok(pending.command_completion)
    }

    pub(in crate::game) fn finish_duelist_player_world(
        &mut self,
        recover_after_wait: bool,
        events: &mut Vec<DomainEvent>,
    ) {
        if !self.player_is_dead() && !self.wilderness_blocks_regeneration() {
            if recover_after_wait {
                self.recover_player_resources(false, events);
            } else {
                self.apply_pet_upkeep_mana_loss(events);
            }
        }
    }
}
