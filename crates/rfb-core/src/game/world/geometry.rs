// SPDX-License-Identifier: MPL-2.0
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rfb_content::{ContentCatalog, ContentPosition, VaultDefinition, VaultTransform};
use rfb_protocol::Position;

use crate::state::FloorState;

pub(in crate::game) fn maze_floor_anchors(walkable: &BTreeSet<Position>) -> (Position, Position) {
    let seed = walkable
        .iter()
        .min_by_key(|position| (position.y, position.x))
        .copied()
        .expect("validated maze must retain walkable terrain");
    let entry = farthest_maze_position(walkable, seed);
    let remote = farthest_maze_position(walkable, entry);
    (entry, remote)
}

fn farthest_maze_position(walkable: &BTreeSet<Position>, start: Position) -> Position {
    let distances = maze_floor_distances(walkable, start);
    let mut positions = distances.keys().copied().collect::<Vec<_>>();
    positions.sort_by(|left, right| {
        distances[right]
            .cmp(&distances[left])
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.x.cmp(&right.x))
    });
    positions[0]
}

pub(in crate::game) fn maze_floor_distances(
    walkable: &BTreeSet<Position>,
    start: Position,
) -> BTreeMap<Position, u32> {
    const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut distances = BTreeMap::from([(start, 0_u32)]);
    let mut frontier = VecDeque::from([start]);
    while let Some(position) = frontier.pop_front() {
        let next_distance = distances[&position] + 1;
        for (dx, dy) in CARDINAL_OFFSETS {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if walkable.contains(&neighbor) && !distances.contains_key(&neighbor) {
                distances.insert(neighbor, next_distance);
                frontier.push_back(neighbor);
            }
        }
    }
    distances
}

pub(in crate::game) fn maze_floor_path(
    walkable: &BTreeSet<Position>,
    start: Position,
    end: Position,
) -> Vec<Position> {
    const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
    let mut predecessors = BTreeMap::new();
    let mut visited = BTreeSet::from([start]);
    let mut frontier = VecDeque::from([start]);
    while let Some(position) = frontier.pop_front() {
        if position == end {
            break;
        }
        for (dx, dy) in CARDINAL_OFFSETS {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if walkable.contains(&neighbor) && visited.insert(neighbor) {
                predecessors.insert(neighbor, position);
                frontier.push_back(neighbor);
            }
        }
    }
    let mut path = vec![end];
    let mut current = end;
    while current != start {
        current = predecessors[&current];
        path.push(current);
    }
    path.reverse();
    path
}

pub(in crate::game) fn transformed_vault_dimensions(
    vault: &VaultDefinition,
    transform: VaultTransform,
) -> (u16, u16) {
    match transform {
        VaultTransform::Identity
        | VaultTransform::Rotate180
        | VaultTransform::MirrorHorizontal
        | VaultTransform::MirrorVertical => (vault.width, vault.height),
        VaultTransform::Rotate90
        | VaultTransform::Rotate270
        | VaultTransform::MirrorMainDiagonal
        | VaultTransform::MirrorAntiDiagonal => (vault.height, vault.width),
    }
}

pub(in crate::game) fn transformed_vault_position(
    vault: &VaultDefinition,
    transform: VaultTransform,
    position: ContentPosition,
) -> Position {
    let x = i32::from(position.x);
    let y = i32::from(position.y);
    let max_x = i32::from(vault.width - 1);
    let max_y = i32::from(vault.height - 1);
    match transform {
        VaultTransform::Identity => Position { x, y },
        VaultTransform::Rotate90 => Position { x: max_y - y, y: x },
        VaultTransform::Rotate180 => Position {
            x: max_x - x,
            y: max_y - y,
        },
        VaultTransform::Rotate270 => Position { x: y, y: max_x - x },
        VaultTransform::MirrorHorizontal => Position { x: max_x - x, y },
        VaultTransform::MirrorVertical => Position { x, y: max_y - y },
        VaultTransform::MirrorMainDiagonal => Position { x: y, y: x },
        VaultTransform::MirrorAntiDiagonal => Position {
            x: max_y - y,
            y: max_x - x,
        },
    }
}

pub(in crate::game) fn vault_entrance_outward(
    entrance: Position,
    transformed_width: u16,
    transformed_height: u16,
) -> Position {
    if entrance.y == 0 {
        Position { x: 0, y: -1 }
    } else if entrance.x + 1 == i32::from(transformed_width) {
        Position { x: 1, y: 0 }
    } else if entrance.y + 1 == i32::from(transformed_height) {
        Position { x: 0, y: 1 }
    } else {
        Position { x: -1, y: 0 }
    }
}

const MAX_VAULT_CONNECTOR_TILES: usize = 12;

pub(in crate::game) fn vault_connector_path(
    terrain: &[String],
    width: u16,
    wall_terrain_id: &str,
    footprint: &BTreeSet<Position>,
    existing_connectors: &BTreeSet<Position>,
    start: Position,
    content: &ContentCatalog,
) -> Option<Vec<Position>> {
    let height = i32::try_from(terrain.len() / usize::from(width)).ok()?;
    if start.x <= 0
        || start.y <= 0
        || start.x >= i32::from(width - 1)
        || start.y >= height - 1
        || footprint.contains(&start)
    {
        return None;
    }
    let is_target = |position: Position| {
        if existing_connectors.contains(&position) {
            return true;
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        terrain.get(index).is_some_and(|terrain_id| {
            terrain_id != wall_terrain_id && terrain_is_connectable(content, terrain_id)
        })
    };
    if is_target(start) {
        return Some(Vec::new());
    }
    let start_index = start.y as usize * usize::from(width) + start.x as usize;
    if terrain
        .get(start_index)
        .is_none_or(|id| id != wall_terrain_id)
    {
        return None;
    }

    let mut pending = VecDeque::from([start]);
    let mut distance = BTreeMap::from([(start, 0_usize)]);
    let mut parent = BTreeMap::new();
    while let Some(position) = pending.pop_front() {
        let current_distance = distance[&position];
        for direction in [
            Position { x: 0, y: -1 },
            Position { x: 1, y: 0 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
        ] {
            let next = Position {
                x: position.x + direction.x,
                y: position.y + direction.y,
            };
            if next.x <= 0
                || next.y <= 0
                || next.x >= i32::from(width - 1)
                || next.y >= height - 1
                || footprint.contains(&next)
                || distance.contains_key(&next)
            {
                continue;
            }
            let index = next.y as usize * usize::from(width) + next.x as usize;
            let terrain_id = terrain.get(index)?;
            if is_target(next) {
                parent.insert(next, position);
                let mut path = Vec::new();
                let mut cursor = next;
                while cursor != start {
                    cursor = parent[&cursor];
                    path.push(cursor);
                }
                path.reverse();
                path.retain(|cell| !existing_connectors.contains(cell));
                return (path.len() <= MAX_VAULT_CONNECTOR_TILES).then_some(path);
            }
            if terrain_id != wall_terrain_id || current_distance >= MAX_VAULT_CONNECTOR_TILES {
                continue;
            }
            distance.insert(next, current_distance + 1);
            parent.insert(next, position);
            pending.push_back(next);
        }
    }
    None
}

pub(in crate::game) fn terrain_is_connectable(content: &ContentCatalog, terrain_id: &str) -> bool {
    content.terrain(terrain_id).is_some_and(|terrain| {
        terrain.walkable
            || terrain.open_to_terrain_id.is_some()
            || terrain.bash_to_terrain_id.is_some()
            || terrain.dig_to_terrain_id.is_some()
    })
}

pub(in crate::game) fn generated_terrain_is_connected(
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> bool {
    let connectable = terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            terrain_is_connectable(content, terrain_id).then_some(Position {
                x: i32::try_from(index % usize::from(width)).expect("floor x must fit i32"),
                y: i32::try_from(index / usize::from(width)).expect("floor y must fit i32"),
            })
        })
        .collect::<BTreeSet<_>>();
    let Some(start) = connectable.first().copied() else {
        return false;
    };
    let mut reached = BTreeSet::new();
    let mut pending = VecDeque::from([start]);
    while let Some(position) = pending.pop_front() {
        if !connectable.contains(&position) || !reached.insert(position) {
            continue;
        }
        for direction in [
            Position { x: 0, y: -1 },
            Position { x: 1, y: 0 },
            Position { x: 0, y: 1 },
            Position { x: -1, y: 0 },
        ] {
            let next = Position {
                x: position.x + direction.x,
                y: position.y + direction.y,
            };
            if next.x >= 0 && next.y >= 0 && next.x < i32::from(width) && next.y < i32::from(height)
            {
                pending.push_back(next);
            }
        }
    }
    reached == connectable
}

pub(in crate::game) fn generated_terrain_index(width: u16, position: Position) -> usize {
    position.y as usize * usize::from(width) + position.x as usize
}

pub(in crate::game) fn floor_position_is_walkable(
    floor: &FloorState,
    position: Position,
    content: &ContentCatalog,
) -> bool {
    if position.x < 0
        || position.y < 0
        || position.x >= i32::from(floor.width)
        || position.y >= i32::from(floor.height)
    {
        return false;
    }
    let index = position.y as usize * usize::from(floor.width) + position.x as usize;
    floor
        .terrain
        .get(index)
        .and_then(|terrain_id| content.terrain(terrain_id))
        .is_some_and(|terrain| terrain.walkable)
}
