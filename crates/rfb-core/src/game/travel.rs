// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeSet, VecDeque};

use rfb_protocol::{Direction, Position};

use crate::effect::{STATUS_BLINDNESS, STATUS_CONFUSION};

use super::Game;

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
        if let Some(can_enter) = self.player_can_enter_local_wilderness(position) {
            return can_enter;
        }
        if let Some(mount_id) = self.riding_actor_id.as_deref() {
            return self
                .entities
                .iter()
                .position(|entity| entity.id == mount_id)
                .is_some_and(|mount_index| self.actor_can_enter_position(mount_index, position));
        }
        terrain.walkable || self.player_can_pass_walls()
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
