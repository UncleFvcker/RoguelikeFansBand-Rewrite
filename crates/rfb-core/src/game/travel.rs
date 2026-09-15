// SPDX-License-Identifier: MPL-2.0

use std::cmp::Reverse;
use std::collections::{BTreeSet, BinaryHeap, VecDeque};

use rfb_protocol::{Direction, Position};

use crate::effect::{STATUS_BLINDNESS, STATUS_CONFUSION};

use super::{item_use::SettledItemUse, magic_eater::AutoDeviceEffect, *};

const DIRECTIONS: [Direction; 8] = [
    Direction::North,
    Direction::NorthEast,
    Direction::East,
    Direction::SouthEast,
    Direction::South,
    Direction::SouthWest,
    Direction::West,
    Direction::NorthWest,
];

impl Game {
    pub(super) fn record_detection_coverage(
        &mut self,
        category: &str,
        radius: u8,
        through_walls: bool,
    ) {
        if category == "trap" && radius == 0 {
            return;
        }
        let range = u32::from(radius).saturating_sub(u32::from(category == "trap"));
        let origin = self.player.position;
        let positions = (1..i32::from(self.height) - 1)
            .flat_map(|y| (1..i32::from(self.width) - 1).map(move |x| Position { x, y }))
            .filter(|position| {
                projectile_geometry::rfb_distance(origin, *position) <= range
                    && (through_walls || self.is_visible(*position))
            })
            .collect::<Vec<_>>();
        if category == "trap" {
            self.detection_coverage.traps.extend(positions);
        } else {
            self.detection_coverage.mapping.extend(positions);
        }
    }

    pub(super) fn prepare_local_travel(
        &mut self,
        destination: Position,
        ordinary: bool,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Direction>, CoreError> {
        let Some(direction) = self.next_local_travel_direction(destination) else {
            return Ok(None);
        };
        let next = self.position_in_direction(direction);
        if ordinary
            && next != destination
            && self.items.iter().any(|item| {
                item.location == ItemLocation::Ground(next)
                    && self.item_is_discovered(&item.id)
                    && (!self.operation_options.travel_ignore_items
                        || self.item_identification(item) == ItemIdentificationDto::Unexamined)
            })
        {
            events.push(DomainEvent::LocalTravelItemFound);
            return Ok(None);
        }
        let safe = self.prepare_automatic_step(direction, events, changed, removed_entities)?;
        Ok((safe && self.local_travel_position_is_available(next)).then_some(direction))
    }

    pub(super) fn prepare_automatic_step(
        &mut self,
        direction: Direction,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        if self.visible_hostile_exists()
            || self.player_has_status_kind(STATUS_BLINDNESS)
            || self.player_has_status_kind(STATUS_CONFUSION)
        {
            return Ok(false);
        }
        // A paralyzed attempt is replaced by an idle turn before any movement occurs.
        if self.player_has_status_kind(STATUS_PARALYSIS) {
            return Ok(true);
        }
        let (dx, dy) = direction.delta();
        let start = self.player.position;
        let next = Position {
            x: start.x + dx,
            y: start.y + dy,
        };
        let event_start = events.len();
        if self.detection_coverage.traps.contains(&start)
            && !self.detection_coverage.traps.contains(&next)
        {
            let refreshed = self.travel_options.auto_detect_traps
                && self.auto_travel_detection(
                    AutoDeviceEffect::Traps,
                    events,
                    changed,
                    removed_entities,
                )?;
            if !refreshed && self.travel_options.disturb_trap_detect {
                events.push(DomainEvent::LocalTravelLeftDetectionArea);
                return Ok(false);
            }
        }
        if self.travel_options.auto_map_area
            && self.detection_coverage.mapping.contains(&start)
            && !self.detection_coverage.mapping.contains(&next)
        {
            self.auto_travel_detection(
                AutoDeviceEffect::Mapping,
                events,
                changed,
                removed_entities,
            )?;
        }
        // A newly detected hostile or trap interrupts this step; the next request can replan.
        let found_hostile = events[event_start..].iter().any(|event| {
            let resolution = match event {
                DomainEvent::AbilityDetected { resolution, .. }
                | DomainEvent::ItemDetected { resolution, .. }
                | DomainEvent::ItemActivationDetected { resolution, .. } => resolution,
                _ => return false,
            };
            resolution.detected_entity_ids.iter().any(|id| {
                self.entities.iter().any(|actor| {
                    actor.id == *id && actor.hp > 0 && !self.actor_is_player_side(actor)
                })
            })
        });
        Ok(!found_hostile
            && !self.visible_hostile_exists()
            && !self
                .content
                .terrain(self.known_terrain_at(next))
                .is_some_and(|terrain| terrain.trap.is_some()))
    }

    fn auto_travel_detection(
        &mut self,
        effect: AutoDeviceEffect,
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<bool, CoreError> {
        if self.player_is_berserker() {
            return Ok(false);
        }
        let selected = self.magic_eater_auto_device(effect).or_else(|| {
            // RFB cmd1: trap device, known trap scroll, then detect-all device.
            // pack_find_device allows exact-cost SP; body selection requires strictly more.
            self.items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    if item.location != ItemLocation::Inventory
                        || self.item_identification(item) == ItemIdentificationDto::Unexamined
                    {
                        return None;
                    }
                    if let (Some(activation), Some(sp)) = (&item.activation, item.charges)
                        && effect.matches(&activation.profile_id)
                        && sp.current >= activation.cost
                    {
                        return Some((
                            u8::from(
                                activation.profile_id == "rfb.device-activation.rod.detect-all",
                            ) * 2,
                            item.id.as_str(),
                            index,
                        ));
                    }
                    if self.player_has_status_kind(STATUS_BLINDNESS) {
                        return None;
                    }
                    let scroll = match effect {
                        AutoDeviceEffect::Traps => "demo.item.trapfinding-scroll",
                        AutoDeviceEffect::Mapping => "demo.item.cartography-scroll",
                        AutoDeviceEffect::Identify => return None,
                    };
                    (item.kind_id == scroll).then_some((1, item.id.as_str(), index))
                })
                .min_by_key(|(priority, id, _)| (*priority, *id))
                .map(|(_, _, index)| index)
        });
        let Some(index) = selected else {
            return Ok(false);
        };
        let item = self.items[index].clone();
        let (effect, target) = if let Some(activation) = &item.activation {
            let generation = item_device_generation(
                &self.content,
                &item.kind_id,
                &item.affix_ids,
                Some(&activation.profile_id),
                item.artifact_name.is_some(),
            )
            .expect("automatic device generation");
            let profile = generation
                .activations
                .iter()
                .find(|profile| profile.id == activation.profile_id)
                .expect("automatic device profile");
            (profile.effect.clone(), Some(profile.target.clone()))
        } else {
            (
                self.content
                    .item(&item.kind_id)
                    .expect("automatic scroll")
                    .use_action
                    .as_ref()
                    .expect("automatic scroll use")
                    .effect
                    .clone(),
                None,
            )
        };
        let plan = self
            .item_use_plan(&item.id, &effect, target.as_ref(), None, None)
            .expect("automatic detection self target");
        self.resolve_inventory_item_effect(
            SettledItemUse {
                kind_id: item.kind_id,
                profile_id: item
                    .activation
                    .as_ref()
                    .map(|activation| activation.profile_id.clone()),
                activation_power: item.activation.as_ref().map(|activation| activation.power),
                effect,
                plan,
                device_power_bonus: 0,
            },
            events,
            changed,
            removed_entities,
        )?;
        if let Some(activation) = item.activation {
            self.items[index]
                .charges
                .as_mut()
                .expect("automatic device SP")
                .current -= activation.cost;
        } else if item.quantity > 1 {
            self.items[index].quantity -= 1;
        } else {
            self.items.remove(index);
            self.item_property_knowledge.remove(&item.id);
        }
        Ok(true)
    }

    pub(super) fn next_local_travel_direction(&self, destination: Position) -> Option<Direction> {
        let start = self.player.position;
        if self.map_scale == rfb_protocol::MapScaleDto::World
            || start == destination
            || self.player_has_status_kind(STATUS_BLINDNESS)
            || self.player_has_status_kind(STATUS_CONFUSION)
            || self.entities.iter().any(|entity| {
                entity.hp > 0
                    && !self.actor_is_player_side(entity)
                    && self.entity_is_visible_to_player(entity)
            })
            || !self.local_travel_position_is_available(destination)
        {
            return None;
        }

        self.local_travel_paths(Some(destination)).1[self.index(destination)?]
    }

    // RFB master@a0d92b6378d148c5262cc236b8fa6ed2ca06a54c, cmd2.c:
    // travel_flow / _travel_flow_bonus. Use known terrain and current traveler abilities.
    fn local_travel_paths(
        &self,
        destination: Option<Position>,
    ) -> (Vec<u32>, Vec<Option<Direction>>) {
        let mut costs = vec![u32::MAX; self.terrain.len()];
        let mut directions = vec![None; self.terrain.len()];
        let start = self.index(self.player.position).unwrap();
        costs[start] = 0;
        let mut queue = BinaryHeap::from([Reverse((0, 0usize, start))]);
        let mut order = 0;
        let mut bonuses = BTreeMap::new();
        while let Some(Reverse((cost, _, index))) = queue.pop() {
            if cost != costs[index] {
                continue;
            }
            let position = Position {
                x: index as i32 % i32::from(self.width),
                y: index as i32 / i32::from(self.width),
            };
            if destination == Some(position) {
                break;
            }
            // Stable FIFO ties preserve the existing goal-facing direction order on plain floors.
            for (direction, next) in
                ordered_neighbors(position, destination.unwrap_or(self.player.position))
            {
                if !self.local_travel_position_is_available(next) {
                    continue;
                }
                let next_index = self.index(next).unwrap();
                let terrain = self.known_terrain_at(next);
                let bonus = *bonuses
                    .entry(terrain)
                    .or_insert_with(|| self.local_travel_terrain_bonus(terrain));
                let next_cost = cost + 1 + bonus;
                if next_cost >= costs[next_index] {
                    continue;
                }
                costs[next_index] = next_cost;
                directions[next_index] = directions[index].or(Some(direction));
                order += 1;
                queue.push(Reverse((next_cost, order, next_index)));
            }
        }
        (costs, directions)
    }

    fn local_travel_terrain_bonus(&self, terrain_id: &str) -> u32 {
        let terrain = self.content.terrain(terrain_id).unwrap();
        let has = |tag| terrain.tags.iter().any(|value| value == tag);
        let flying = self.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly);
        let element = if has("lava") {
            Some((DamageType::Fire, if has("deep") { 16 } else { 1 }))
        } else if has("acid") {
            Some((DamageType::Acid, if has("deep") { 12 } else { 6 }))
        } else {
            None
        };
        if let Some((element, mut bonus)) = element {
            let resistance = self.player_resistance_percent(element);
            if element == DamageType::Fire && resistance >= 100 {
                return 0;
            }
            if flying {
                bonus /= 2;
            }
            if resistance <= 50 {
                bonus *= 4;
            }
            return bonus;
        }
        if has("water")
            && has("deep")
            && !flying
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Swim)
            && !self.active_traveler_has_mode(rfb_content::ActorMovementMode::Aquatic)
            && self.carried_weight_tenths_pound() > self.player_carry_capacity_tenths_pound()
        {
            return 4;
        }
        0
    }

    pub(super) fn nearest_unknown_item_target(
        &self,
    ) -> Result<rfb_protocol::AutoGetTargetDto, &'static str> {
        if self.map_scale != rfb_protocol::MapScaleDto::Local {
            return Err("game-unknown-item-local-only");
        }
        let candidates = self.unknown_item_travel_candidates();
        if candidates.is_empty() {
            return Err("game-unknown-item-none");
        }
        let (costs, _) = self.local_travel_paths(None);
        candidates
            .into_iter()
            .filter_map(|target| {
                let cost = costs[self.index(target.position)?];
                (cost < u32::MAX).then_some((cost, target))
            })
            .min_by(|a, b| (a.0, a.1.object_id.as_str()).cmp(&(b.0, b.1.object_id.as_str())))
            .map(|(_, target)| target)
            .ok_or("game-unknown-item-no-route")
    }

    pub(super) fn unknown_item_travel_target_is_valid(
        &self,
        object_id: &str,
        destination: Position,
    ) -> bool {
        self.map_scale == rfb_protocol::MapScaleDto::Local
            && self
                .unknown_item_travel_candidates()
                .iter()
                .any(|target| target.object_id == object_id && target.position == destination)
    }

    pub(super) fn reachable_local_travel_positions(&self) -> BTreeSet<Position> {
        let mut visited = BTreeSet::from([self.player.position]);
        let mut queue = VecDeque::from([self.player.position]);
        while let Some(position) = queue.pop_front() {
            for direction in DIRECTIONS {
                let (dx, dy) = direction.delta();
                let next = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if !visited.contains(&next) && self.local_travel_position_is_available(next) {
                    visited.insert(next);
                    queue.push_back(next);
                }
            }
        }
        visited
    }

    pub(super) fn local_travel_position_is_available(&self, position: Position) -> bool {
        let Some(index) = self.index(position) else {
            return false;
        };
        if !self.explored[index]
            || self.entities.iter().any(|entity| {
                entity.hp > 0
                    && entity.position == position
                    && self.entity_is_visible_to_player(entity)
            })
        {
            return false;
        }
        let terrain = self
            .content
            .terrain(self.known_terrain_at(position))
            .expect("known terrain must remain available");
        if terrain.trap.is_some() {
            return false;
        }
        (if self.is_wilderness_floor() {
            self.player_can_cross_surface_terrain(terrain)
        } else {
            self.player_can_cross_terrain(terrain)
        }) || self.player_wall_destruction_target(position).is_some()
    }
}

fn ordered_neighbors(position: Position, destination: Position) -> Vec<(Direction, Position)> {
    let mut neighbors = DIRECTIONS
        .iter()
        .copied()
        .enumerate()
        .map(|(order, direction)| {
            let (dx, dy) = direction.delta();
            let next = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            let distance = (next.x - destination.x).pow(2) + (next.y - destination.y).pow(2);
            (distance, order, direction, next)
        })
        .collect::<Vec<_>>();
    neighbors.sort_by_key(|(distance, order, _, _)| (*distance, *order));
    neighbors
        .into_iter()
        .map(|(_, _, direction, position)| (direction, position))
        .collect()
}
