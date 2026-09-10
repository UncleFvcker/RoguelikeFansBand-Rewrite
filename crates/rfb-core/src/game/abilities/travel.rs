// SPDX-License-Identifier: MPL-2.0

use crate::effect::STATUS_SLEEP;
use crate::error::CoreError;
use crate::event::DomainEvent;
use crate::game::floor::{FloorTransitionTarget, RecallUseAction};
use crate::game::projectile_geometry::{has_line_of_effect, rfb_distance};
use crate::game::visibility::has_line_of_sight;
use crate::game::{Game, actor_matches_category};
use rfb_content::{AbilityDefinition, AbilityEffectDefinition};
use rfb_protocol::MapScaleDto;
use rfb_protocol::{
    AbilityEffectResolutionDto, AbilityEffectsResolutionDto, AbilityRecallActionDto,
    AbilityTeleportResolutionDto, Position,
};
use std::collections::BTreeSet;

impl Game {
    pub(in crate::game) fn player_can_teleport_to(
        &self,
        position: Position,
        passive: bool,
    ) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        let terrain = self
            .content
            .terrain(&self.terrain[index])
            .expect("active terrain exists");
        if self.vault_cells[index]
            || terrain.trap.is_some()
            || terrain.allows_wall_passage
            || (!terrain.walkable && terrain.movement_modes.is_empty())
            || self.entities.iter().any(|actor| {
                actor.hp > 0
                    && actor.position == position
                    && self.riding_actor_id.as_ref() != Some(&actor.id)
            })
        {
            return false;
        }
        if passive {
            return true;
        }
        if !self.player_can_cross_terrain(terrain) {
            return false;
        }
        let has_tag = |tag: &str| terrain.tags.iter().any(|value| value == tag);
        let flies = self.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly);
        if has_tag("water")
            && has_tag("deep")
            && !flies
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Swim)
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Aquatic)
        {
            return false;
        }
        if has_tag("lava")
            && self
                .effective_player_resistances()
                .level(crate::resistance::DamageType::Fire)
                != crate::resistance::ResistanceLevel::Immune
            && !self.player_has_status_kind(crate::effect::STATUS_INVULNERABILITY)
            && (has_tag("deep") || !flies)
        {
            return false;
        }
        true
    }

    #[allow(clippy::too_many_arguments)] // Source teleport flags and the existing event/change outputs.
    pub(super) fn resolve_player_teleport_with_range(
        &mut self,
        ability_id: &str,
        range: u16,
        line_of_sight: bool,
        passive: bool,
        excluded_follower: Option<&str>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.player_has_anti_teleport() || self.map_scale == MapScaleDto::World {
            return;
        }
        let from = self.player.position;
        let mut candidates = Vec::new();
        for y in 1..self.height.saturating_sub(1) {
            for x in 1..self.width.saturating_sub(1) {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                if rfb_distance(from, position) <= u32::from(range)
                    && self.player_can_teleport_to(position, passive)
                    && (!line_of_sight || has_line_of_sight(self, from, position))
                {
                    candidates.push(position);
                }
            }
        }
        if candidates.is_empty() {
            return;
        }
        // Keep complete distance rings, as teleport_player_aux does, including tied distances.
        let mut distances: Vec<_> = candidates
            .iter()
            .map(|position| rfb_distance(from, *position))
            .collect();
        distances.sort_unstable_by(|a, b| b.cmp(a));
        let minimum = distances[(distances.len() / 2).max(1) - 1];
        candidates.retain(|position| rfb_distance(from, *position) >= minimum);
        let destination = candidates[self.rng.bounded(candidates.len() as u64) as usize];
        if destination == from {
            return;
        }
        let floor_id = self.current_floor_id.clone();
        events.push(DomainEvent::AbilityTeleported {
            ability_id: ability_id.to_owned(),
            resolution: AbilityTeleportResolutionDto {
                from,
                to: destination,
            },
        });
        events.extend(self.relocate_player(destination, changed));
        if self.player_is_dead()
            || self.current_floor_id != floor_id
            || self.floor_depth(&floor_id) == 0
        {
            return;
        }
        let mut followers: Vec<_> = self
            .entities
            .iter()
            .filter(|actor| {
                actor.hp > 0
                    && actor.position.x.abs_diff(from.x) <= 2
                    && actor.position.y.abs_diff(from.y) <= 2
                    && self.riding_actor_id.as_ref() != Some(&actor.id)
                    && excluded_follower != Some(actor.id.as_str())
                    && has_line_of_effect(self, actor.position, from)
                    && self
                        .actor_runtime_definition(actor)
                        .is_some_and(|definition| {
                            definition.id == "demo.actor.monkey-clone"
                                || (!actor
                                    .statuses
                                    .iter()
                                    .any(|status| status.kind_id == STATUS_SLEEP)
                                    && !definition.tags.iter().any(|tag| tag == "resist-teleport")
                                    && definition.monster_casting.as_ref().is_some_and(|casting| {
                                        casting.abilities.iter().any(|candidate| {
                                            self.content.ability(&candidate.ability_id).is_some_and(
                                                |ability| {
                                                    matches!(
                                                        ability.effect,
                                                        AbilityEffectDefinition::TeleportSelf { .. }
                                                    )
                                                },
                                            )
                                        })
                                    }))
                        })
            })
            .map(|actor| (actor.position.x, actor.position.y, actor.id.clone()))
            .collect();
        followers.sort();
        for (_, _, id) in followers {
            let index = self
                .entities
                .iter()
                .position(|actor| actor.id == id)
                .expect("follower remains present");
            let definition = self
                .actor_runtime_definition(&self.entities[index])
                .expect("follower definition exists");
            let level = if definition.id == "demo.actor.monkey-clone" {
                10_000
            } else {
                definition.level
            };
            if self.rng.bounded(100) + 1 > u64::from(level) {
                continue;
            }
            let destinations = self.displacement_destinations(index, |position| {
                rfb_distance(destination, position) <= 2
            });
            if !destinations.is_empty() {
                let to = destinations[self.rng.bounded(destinations.len() as u64) as usize];
                changed.insert(self.entities[index].position);
                self.entities[index].position = to;
                changed.insert(to);
            }
        }
    }

    pub(super) fn resolve_player_alter_reality_effect(
        &mut self,
        ability: &AbilityDefinition,
        events: &mut Vec<DomainEvent>,
    ) {
        debug_assert!(matches!(
            ability.effect,
            AbilityEffectDefinition::AlterReality
        ));
        let ticks_before = self.reality_change_ticks;
        self.reality_change_ticks = if ticks_before == 0 {
            u8::try_from(self.rng.bounded(21) + 15).expect("reality countdown must fit u8")
        } else {
            0
        };
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::AlterReality {
                    effect_index: 0,
                    ticks_before,
                    ticks_after: self.reality_change_ticks,
                }],
            },
            trace: None,
        });
    }

    pub(in crate::game) fn resolve_player_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        destination: Position,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if self.player_has_anti_teleport() {
            return;
        }
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

    pub(super) fn resolve_player_random_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        if let AbilityEffectDefinition::BlinkSelf {
            radius,
            line_of_sight: true,
        } = ability.effect
        {
            self.resolve_player_teleport_with_range(
                &ability.id,
                u16::from(radius),
                true,
                false,
                None,
                events,
                changed,
            );
            return;
        }
        let index = usize::try_from(self.rng.bounded(candidates.len() as u64))
            .expect("bounded teleport candidate index must fit usize");
        self.resolve_player_teleport_effect(ability, candidates[index], events, changed);
    }

    pub(super) fn resolve_player_dimension_door_effect(
        &mut self,
        ability: &AbilityDefinition,
        requested: Position,
        destination_valid: bool,
        fallback_candidates: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let failure_sides = u64::from(self.progress.level / 10 + 10);
        let failed = !destination_valid || self.rng.bounded(failure_sides) == 0;
        let destination = if failed {
            (!fallback_candidates.is_empty()).then(|| {
                let index = usize::try_from(self.rng.bounded(fallback_candidates.len() as u64))
                    .expect("bounded dimension door fallback index must fit usize");
                fallback_candidates[index]
            })
        } else {
            Some(requested)
        };
        if let Some(destination) = destination {
            self.resolve_player_teleport_effect(ability, destination, events, changed);
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::DimensionDoor {
                    effect_index: 0,
                    requested,
                    destination,
                    failed,
                }],
            },
            trace: None,
        });
    }

    pub(super) fn resolve_player_teleport_town_effect(
        &mut self,
        ability: &AbilityDefinition,
        town_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), CoreError> {
        if self.player_has_anti_teleport() {
            return Ok(());
        }
        let from_town_id = self.current_town().map(|town| town.id.clone());
        self.teleport_to_town(town_id)?;
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: vec![AbilityEffectResolutionDto::TeleportTown {
                    effect_index: 0,
                    from_town_id,
                    to_town_id: town_id.to_owned(),
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_swap_position_effect(
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

    pub(super) fn resolve_player_recall_effect(
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

    pub(super) fn resolve_player_level_teleport_effect(
        &mut self,
        ability: &AbilityDefinition,
        upward_targets: Vec<FloorTransitionTarget>,
        downward_targets: Vec<FloorTransitionTarget>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) -> Result<(), CoreError> {
        if self.player_has_anti_teleport() {
            return Ok(());
        }
        let prefer_upward = self.rng.bounded(2) == 0;
        let targets = if prefer_upward {
            if upward_targets.is_empty() {
                downward_targets
            } else {
                upward_targets
            }
        } else if downward_targets.is_empty() {
            upward_targets
        } else {
            downward_targets
        };
        let target_index = if targets.len() == 1 {
            0
        } else {
            usize::try_from(self.rng.bounded(targets.len() as u64))
                .expect("bounded floor target index must fit usize")
        };
        let target = targets[target_index].clone();
        let from_floor_id = self.current_floor_id.clone();
        let transition = self
            .transition_floor(
                target.floor_id,
                target.arrival_connection_id,
                target.departure_connection_id,
                false,
            )?
            .expect("planned ability floor teleport must remain available");
        let to_floor_id = transition.to_floor_id.clone();
        self.record_floor_transition(transition, events, changed);
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: Some(self.player.id.clone()),
                target_kind_id: Some(self.player.kind_id.clone()),
                effects: vec![AbilityEffectResolutionDto::TeleportLevel {
                    effect_index: 0,
                    from_floor_id,
                    to_floor_id,
                }],
            },
            trace: None,
        });
        Ok(())
    }

    pub(super) fn resolve_player_teleport_away_effect(
        &mut self,
        ability: &AbilityDefinition,
        path: Vec<Position>,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
    ) {
        let AbilityEffectDefinition::TeleportAway {
            minimum_distance,
            power,
            stop_at_actor,
            ref target_category,
        } = ability.effect
        else {
            unreachable!("teleport-away executor requires a teleport-away effect");
        };
        let (trace, _) = self.trace_projectile_path_with_actor_policy(path, stop_at_actor);
        let target_ids = self
            .beam_damage_targets(&trace.traversed)
            .into_iter()
            .filter(|target_entity_id| {
                target_category.as_ref().is_none_or(|category| {
                    self.entities
                        .iter()
                        .find(|entity| entity.id == *target_entity_id && entity.hp > 0)
                        .and_then(|entity| self.actor_runtime_definition(entity))
                        .is_some_and(|definition| actor_matches_category(definition, category))
                })
            })
            .collect::<Vec<_>>();
        let mut resolutions = Vec::new();
        for target_entity_id in target_ids {
            let Some(index) = self
                .entities
                .iter()
                .position(|entity| entity.id == target_entity_id && entity.hp > 0)
            else {
                continue;
            };
            resolutions.push(self.resolve_teleport_away_target(
                index,
                0,
                minimum_distance,
                power,
                changed,
            ));
        }
        events.push(DomainEvent::AbilityEffectsResolved {
            ability_id: ability.id.clone(),
            resolution: AbilityEffectsResolutionDto {
                target_entity_id: None,
                target_kind_id: None,
                effects: resolutions,
            },
            trace: Some(trace),
        });
    }

    pub(super) fn resolve_teleport_away_target(
        &mut self,
        index: usize,
        effect_index: u8,
        minimum_distance: u8,
        power: u16,
        changed: &mut BTreeSet<Position>,
    ) -> AbilityEffectResolutionDto {
        let target_entity_id = self.entities[index].id.clone();
        let from = self.entities[index].position;
        let definition = self
            .actor_runtime_definition(&self.entities[index])
            .expect("teleport-away target definition must remain available");
        let has_tag = |tag: &str| definition.tags.iter().any(|candidate| candidate == tag);
        let target_level = definition.level;
        let resistant = has_tag("resist-teleport");
        let always_resisted =
            has_tag("guardian") || (resistant && (has_tag("unique") || has_tag("resist-all")));
        let resistance_roll = if resistant && !always_resisted {
            Some(
                u8::try_from(self.rng.bounded(100) + 1)
                    .expect("teleport resistance roll must fit u8"),
            )
        } else {
            None
        };
        let resisted =
            always_resisted || resistance_roll.is_some_and(|roll| target_level > u32::from(roll));
        let mut to = None;
        if !resisted {
            let distance = u32::from(power.max(u16::from(minimum_distance)));
            let mut minimum = distance / 2;
            let mut maximum = distance.max(1);
            for _ in 0..8 {
                let candidates = (0..self.height)
                    .flat_map(|y| {
                        (0..self.width).map(move |x| Position {
                            x: i32::from(x),
                            y: i32::from(y),
                        })
                    })
                    .filter(|position| {
                        let distance = rfb_distance(from, *position);
                        distance >= minimum
                            && distance <= maximum
                            && *position != self.player.position
                            && self.actor_can_enter_position(index, *position)
                            && !self
                                .entities
                                .iter()
                                .enumerate()
                                .any(|(other_index, entity)| {
                                    other_index != index
                                        && entity.hp > 0
                                        && entity.position == *position
                                })
                    })
                    .collect::<Vec<_>>();
                if !candidates.is_empty() {
                    let candidate_index = if candidates.len() == 1 {
                        0
                    } else {
                        usize::try_from(self.rng.bounded(candidates.len() as u64))
                            .expect("bounded teleport destination index must fit usize")
                    };
                    to = Some(candidates[candidate_index]);
                    break;
                }
                minimum /= 2;
                maximum = maximum.saturating_mul(2);
            }
        }
        if let Some(destination) = to {
            self.entities[index].position = destination;
            changed.insert(from);
            changed.insert(destination);
        }
        AbilityEffectResolutionDto::TeleportAway {
            effect_index,
            target_entity_id,
            power,
            resistance_roll,
            resisted,
            from,
            to,
        }
    }
}
