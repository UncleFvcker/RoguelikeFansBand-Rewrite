// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeSet, VecDeque};

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
        events: &mut Vec<DomainEvent>,
        changed: &mut BTreeSet<Position>,
        removed_entities: &mut Vec<String>,
    ) -> Result<Option<Direction>, CoreError> {
        let Some(direction) = self.next_local_travel_direction(destination) else {
            return Ok(None);
        };
        // A paralyzed attempt is replaced by an idle turn before any movement occurs.
        if self.player_has_status_kind(STATUS_PARALYSIS) {
            return Ok(Some(direction));
        }
        let (dx, dy) = direction.delta();
        let start = self.player.position;
        let next = Position {
            x: start.x + dx,
            y: start.y + dy,
        };
        let event_start = events.len();
        if self.travel_options.disturb_trap_detect
            && self.detection_coverage.traps.contains(&start)
            && !self.detection_coverage.traps.contains(&next)
            && !(self.travel_options.auto_detect_traps
                && self.auto_travel_detection(
                    AutoDeviceEffect::Traps,
                    events,
                    changed,
                    removed_entities,
                )?)
        {
            events.push(DomainEvent::LocalTravelLeftDetectionArea);
            return Ok(None);
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
        Ok((!found_hostile && self.local_travel_position_is_available(next)).then_some(direction))
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

        let mut visited = BTreeSet::from([start]);
        let mut queue = VecDeque::new();
        for (direction, position) in ordered_neighbors(start, destination) {
            if !visited.insert(position) || !self.local_travel_position_is_available(position) {
                continue;
            }
            if position == destination {
                return Some(direction);
            }
            queue.push_back((position, direction));
        }
        while let Some((position, first_direction)) = queue.pop_front() {
            for (_, next) in ordered_neighbors(position, destination) {
                if !visited.insert(next) || !self.local_travel_position_is_available(next) {
                    continue;
                }
                if next == destination {
                    return Some(first_direction);
                }
                queue.push_back((next, first_direction));
            }
        }
        None
    }

    fn local_travel_position_is_available(&self, position: Position) -> bool {
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
