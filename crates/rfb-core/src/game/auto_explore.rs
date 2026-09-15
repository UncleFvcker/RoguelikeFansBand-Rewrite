// SPDX-License-Identifier: MPL-2.0
// RFB master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, src/cmd2.c:
// do_cmd_auto_explore, _travel_continue, has_unmarked_floor.
// See LICENSES/RFB-UPSTREAM-NOTICE.txt. Use this engine's known-map travel rules.

use super::*;
use rfb_protocol::{AutoExploreStateDto, AutoExploreTargetDto};

fn target_position(target: &AutoExploreTargetDto) -> Position {
    match target {
        AutoExploreTargetDto::Frontier { position }
        | AutoExploreTargetDto::Object { position, .. } => *position,
    }
}

impl Game {
    fn auto_explore_known_cells(&self) -> u32 {
        self.explored.iter().filter(|known| **known).count() as u32
    }

    pub(super) fn auto_explore_state_is_valid(&self, state: &AutoExploreStateDto) -> bool {
        self.running.is_none()
            && self.fishing_direction.is_none()
            && self.map_scale == MapScaleDto::Local
            && state.floor_id == self.current_floor_id
            && state.position == self.player.position
            && state.known_cells as usize <= self.explored.len()
            && state
                .target
                .as_ref()
                .is_none_or(|target| self.index(target_position(target)).is_some())
            && state
                .visited_frontiers
                .iter()
                .all(|position| self.index(*position).is_some())
            && state
                .visited_frontiers
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                == state.visited_frontiers.len()
    }

    pub(super) fn auto_explore_hostile_in_sight(&self) -> bool {
        self.entities.iter().any(|entity| {
            entity.hp > 0
                && !self.actor_is_player_side(entity)
                && self.entity_is_visually_visible_to_player(entity)
        })
    }

    fn auto_explore_unavailable_reason(&self) -> Option<&'static str> {
        if self.map_scale != MapScaleDto::Local {
            Some("game-auto-explore-local-only")
        } else if self.player_is_dead() {
            Some("game-auto-explore-dead")
        } else if self.player_has_status_kind(STATUS_BLINDNESS) {
            Some("game-auto-explore-blind")
        } else if self.player_has_status_kind(STATUS_CONFUSION) {
            Some("game-auto-explore-confused")
        } else if self.player_has_status_kind(STATUS_PARALYSIS) {
            Some("game-auto-explore-paralyzed")
        } else if self.mogaminator.pending_query.is_some() {
            Some("game-auto-explore-pickup-query")
        } else if self.pending_duelist.is_some()
            || self.pending_mutation_direction.is_some()
            || self.pending_ability_direction.is_some()
            || self.pending_maia_path_choice()
            || self.pending_race_mutation_choice().is_some()
        {
            Some("game-auto-explore-pending-choice")
        } else if self.auto_explore_hostile_in_sight() {
            Some("game-auto-explore-enemy-in-sight")
        } else {
            None
        }
    }

    pub(super) fn auto_explore_ground_units(&self) -> u64 {
        self.items
            .iter()
            .filter(|item| item.location == ItemLocation::Ground(self.player.position))
            .map(|item| u64::from(item.quantity))
            .sum::<u64>()
            + self
                .gold_piles
                .iter()
                .filter(|pile| pile.position == self.player.position)
                .map(|pile| u64::from(pile.amount))
                .sum::<u64>()
    }

    fn auto_explore_frontier(&self, position: Position) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        if !self.explored[index] {
            return false;
        }
        let terrain = self
            .content
            .terrain(self.known_terrain_at(position))
            .unwrap();
        // The upstream can_travel target filter excludes known walls, rubble and hidden doors.
        if terrain.digging.is_some() || !self.local_travel_position_is_available(position) {
            return false;
        }
        let unknown = (-1..=1)
            .flat_map(|dx| {
                (-1..=1).map(move |dy| Position {
                    x: position.x + dx,
                    y: position.y + dy,
                })
            })
            .filter(|p| self.index(*p).is_some_and(|i| !self.explored[i]))
            .count();
        unknown > 0 && unknown < 8
    }

    fn auto_explore_next_target(
        &self,
        state: &AutoExploreStateDto,
    ) -> Option<AutoExploreTargetDto> {
        let reachable = self.reachable_local_travel_positions();
        // Preserve RFB distance and x-then-y scan order, while skipping unreachable frontiers.
        reachable
            .into_iter()
            .filter(|p| {
                *p != self.player.position
                    && !state.visited_frontiers.contains(p)
                    && self.auto_explore_frontier(*p)
            })
            .min_by_key(|p| {
                (
                    projectile_geometry::rfb_distance(self.player.position, *p),
                    p.x,
                    p.y,
                )
            })
            .map(|position| AutoExploreTargetDto::Frontier { position })
    }

    pub(super) fn prepare_auto_explore(
        &mut self,
        action: &GameAction,
        events: &mut Vec<DomainEvent>,
    ) -> Option<GameAction> {
        if !matches!(
            action,
            GameAction::AutoExplore | GameAction::ContinueAutoExplore
        ) {
            self.auto_explore = None;
            return None;
        }
        let result = (|| {
            if let Some(reason) = self.auto_explore_unavailable_reason() {
                return Err(reason);
            }
            let starting = matches!(action, GameAction::AutoExplore);
            let mut state = if starting {
                // Starting a new action replaces an idle saved run/fishing state.
                self.running = None;
                self.fishing_direction = None;
                AutoExploreStateDto {
                    floor_id: self.current_floor_id.clone(),
                    position: self.player.position,
                    target: None,
                    visited_frontiers: Vec::new(),
                    known_cells: self.auto_explore_known_cells(),
                }
            } else {
                self.auto_explore
                    .take()
                    .filter(|state| self.auto_explore_state_is_valid(state))
                    .ok_or("game-auto-explore-state-lost")?
            };
            // Reconsider objects after each reveal, including matching items at our feet.
            // Generic PickUp would also collect items rejected by the rules.
            if let Some(target) = self.mogaminator_auto_get_target() {
                state.target = Some(AutoExploreTargetDto::Object {
                    object_id: target.object_id,
                    position: target.position,
                });
            } else if state.target.is_none() {
                state.target = self.auto_explore_next_target(&state);
            }
            let target = state.target.as_ref().ok_or("game-auto-explore-complete")?;
            let position = target_position(target);
            let command = match target {
                AutoExploreTargetDto::Object { object_id, .. } => {
                    if self.mogaminator_auto_get_position(object_id) != Some(position) {
                        return Err("game-auto-explore-target-lost");
                    }
                    if position == self.player.position {
                        GameAction::AutoGet {
                            object_id: object_id.clone(),
                        }
                    } else {
                        GameAction::TravelLocal {
                            destination: position,
                        }
                    }
                }
                AutoExploreTargetDto::Frontier { .. } => GameAction::TravelLocal {
                    destination: position,
                },
            };
            self.auto_explore = Some(state);
            Ok(command)
        })();
        match result {
            Ok(command) => Some(command),
            Err(reason) => {
                self.auto_explore = None;
                events.push(DomainEvent::AutoExploreStopped { reason });
                None
            }
        }
    }

    pub(super) fn finish_auto_explore_step(
        &mut self,
        moved: bool,
        translation: Option<Position>,
        pickup: bool,
        ground_units_before: u64,
        events: &mut Vec<DomainEvent>,
    ) {
        let Some(mut state) = self.auto_explore.take() else {
            // A prepared step can be cleared by damage or a successful search.
            let discovered = events.iter().any(|event| match event {
                DomainEvent::SecretTerrainDiscovered { .. } => true,
                DomainEvent::ChestInteracted { message_key } => message_key == "chest-trap-found",
                _ => false,
            });
            events.push(DomainEvent::AutoExploreStopped {
                reason: if discovered {
                    "game-auto-explore-discovery"
                } else {
                    "game-auto-explore-damaged"
                },
            });
            return;
        };
        if let Some(reason) = self.auto_explore_unavailable_reason().or_else(|| {
            (state.floor_id != self.current_floor_id).then_some("game-auto-explore-floor-changed")
        }) {
            events.push(DomainEvent::AutoExploreStopped { reason });
            return;
        }
        let mut previous = state.position;
        if let Some(translation) = translation {
            previous.x += translation.x;
            previous.y += translation.y;
            if let Some(target) = state.target.as_mut() {
                let (AutoExploreTargetDto::Frontier { position }
                | AutoExploreTargetDto::Object { position, .. }) = target;
                position.x += translation.x;
                position.y += translation.y;
            }
            for p in &mut state.visited_frontiers {
                p.x += translation.x;
                p.y += translation.y;
            }
            state.visited_frontiers.retain(|p| self.index(*p).is_some());
        }
        let position = self.player.position;
        let step_distance = previous
            .x
            .abs_diff(position.x)
            .max(previous.y.abs_diff(position.y));
        let opened_door = events
            .iter()
            .any(|event| matches!(event, DomainEvent::DoorOpened { .. }));
        if (!moved && !pickup && !opened_door)
            || (moved && step_distance != 1)
            || (pickup && self.auto_explore_ground_units() >= ground_units_before)
        {
            events.push(DomainEvent::AutoExploreStopped {
                reason: if events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::LocalTravelLeftDetectionArea))
                {
                    "game-auto-explore-detection-boundary"
                } else if events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::ItemPickupInventoryFull { .. }))
                {
                    "game-auto-explore-pack-full"
                } else {
                    "game-auto-explore-no-progress"
                },
            });
            return;
        }
        let known_cells = self.auto_explore_known_cells();
        if known_cells > state.known_cells {
            state.visited_frontiers.clear();
        }
        state.known_cells = known_cells;
        state.position = position;
        if pickup {
            state.target = None;
        } else if let Some(target) = &state.target
            && target_position(target) == position
        {
            match target {
                AutoExploreTargetDto::Frontier { .. } => {
                    if !state.visited_frontiers.contains(&position) {
                        state.visited_frontiers.push(position);
                    }
                    state.target = None;
                }
                AutoExploreTargetDto::Object { object_id, .. } => {
                    if !self.items.iter().any(|item| {
                        item.id == *object_id && item.location == ItemLocation::Ground(position)
                    }) && !self.gold_piles.iter().any(|pile| pile.id == *object_id)
                    {
                        state.target = None;
                    }
                }
            }
        }
        if state
            .target
            .as_ref()
            .is_some_and(|target| self.index(target_position(target)).is_none())
        {
            events.push(DomainEvent::AutoExploreStopped {
                reason: "game-auto-explore-target-lost",
            });
        } else {
            self.auto_explore = Some(state);
        }
    }
}
