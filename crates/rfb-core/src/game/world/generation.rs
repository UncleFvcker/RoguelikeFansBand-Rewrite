// SPDX-License-Identifier: MPL-2.0
use std::collections::BTreeSet;

use rfb_content::{
    ContentCatalog, ContentPosition, EncounterFormation, ProceduralFloorDefinition,
    TerrainFeaturePlacement, VaultDefinition, VaultTransform,
};
use rfb_protocol::Position;

use super::super::{
    GeneratedRegion, GeneratedRoom, GeneratedVaultPlacement, GeneratedVaultPlacementCandidate,
};
use super::geometry::{
    generated_terrain_index, generated_terrain_is_connected, maze_floor_distances,
    transformed_vault_dimensions, transformed_vault_position, vault_connector_path,
    vault_entrance_outward,
};

pub(in crate::game) fn generated_non_entry_room_id(rooms: &[GeneratedRoom], ordinal: u16) -> &str {
    let room_index = 1 + usize::from(ordinal) % (rooms.len() - 1);
    &rooms[room_index].id
}

pub(in crate::game) fn choose_generated_maze_position(
    walkable: &BTreeSet<Position>,
    entry: Position,
    occupied: &BTreeSet<Position>,
) -> Position {
    let distances = maze_floor_distances(walkable, entry);
    let mut candidates = walkable
        .iter()
        .filter(|position| !occupied.contains(position))
        .copied()
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        distances[right]
            .cmp(&distances[left])
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.x.cmp(&right.x))
    });
    candidates[0]
}

pub(in crate::game) fn formation_placement_candidates(
    rooms: &[GeneratedRoom],
    room_id: &str,
    occupied: &BTreeSet<Position>,
    formation: EncounterFormation,
    companion_count: u16,
) -> Vec<(Position, Vec<Position>)> {
    const RING_OFFSETS: [(i32, i32); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    const CLUSTER_ORDER: [usize; 8] = [0, 2, 4, 6, 1, 3, 5, 7];
    let room = rooms
        .iter()
        .find(|room| room.id == room_id)
        .expect("validated formation room must remain available");
    let mut candidates = Vec::new();
    for leader_y in room.y..room.y + room.height {
        for leader_x in room.x..room.x + room.width {
            let leader = Position {
                x: leader_x,
                y: leader_y,
            };
            if !room.contains(leader) || occupied.contains(&leader) {
                continue;
            }
            for orientation in 0..RING_OFFSETS.len() {
                let offsets = (0..usize::from(companion_count))
                    .map(|index| {
                        let base_index = match formation {
                            EncounterFormation::Cluster => CLUSTER_ORDER[index],
                            EncounterFormation::Ring => {
                                index * RING_OFFSETS.len() / usize::from(companion_count)
                            }
                        };
                        RING_OFFSETS[(base_index + orientation) % RING_OFFSETS.len()]
                    })
                    .collect::<Vec<_>>();
                let companions = offsets
                    .iter()
                    .map(|(dx, dy)| Position {
                        x: leader.x + dx,
                        y: leader.y + dy,
                    })
                    .collect::<Vec<_>>();
                if companions
                    .iter()
                    .all(|position| room.contains(*position) && !occupied.contains(position))
                {
                    candidates.push((leader, companions));
                }
            }
        }
    }
    candidates
}

pub(in crate::game) fn free_vault_placement_candidates(
    terrain: &[String],
    width: u16,
    height: u16,
    wall_terrain_id: &str,
    corridor_terrain_id: &str,
    vault: &VaultDefinition,
    content: &ContentCatalog,
) -> Vec<GeneratedVaultPlacementCandidate> {
    let transforms = if vault.transforms.is_empty() {
        vec![VaultTransform::Identity]
    } else {
        vault.transforms.clone()
    };
    let mut candidates = Vec::new();
    for transform in transforms {
        let (transformed_width, transformed_height) =
            transformed_vault_dimensions(vault, transform);
        if transformed_width + 2 > width || transformed_height + 2 > height {
            continue;
        }
        let mut entrances = vault
            .entrance_positions
            .iter()
            .map(|position| transformed_vault_position(vault, transform, *position))
            .collect::<Vec<_>>();
        entrances.sort_by_key(|position| (position.y, position.x));
        for origin_y in 1..=i32::from(height - transformed_height - 1) {
            for origin_x in 1..=i32::from(width - transformed_width - 1) {
                let origin = Position {
                    x: origin_x,
                    y: origin_y,
                };
                let footprint_is_free = (0..i32::from(transformed_height)).all(|local_y| {
                    (0..i32::from(transformed_width)).all(|local_x| {
                        let position = Position {
                            x: origin.x + local_x,
                            y: origin.y + local_y,
                        };
                        let index = position.y as usize * usize::from(width) + position.x as usize;
                        terrain
                            .get(index)
                            .is_some_and(|terrain_id| terrain_id == wall_terrain_id)
                    })
                });
                if !footprint_is_free {
                    continue;
                }
                let footprint = (0..i32::from(transformed_height))
                    .flat_map(|local_y| {
                        (0..i32::from(transformed_width)).map(move |local_x| Position {
                            x: origin.x + local_x,
                            y: origin.y + local_y,
                        })
                    })
                    .collect::<BTreeSet<_>>();
                let mut connector_cells = BTreeSet::new();
                let mut all_entrances_connect = true;
                for entrance in &entrances {
                    let outward =
                        vault_entrance_outward(*entrance, transformed_width, transformed_height);
                    let outside = Position {
                        x: origin.x + entrance.x + outward.x,
                        y: origin.y + entrance.y + outward.y,
                    };
                    let Some(path) = vault_connector_path(
                        terrain,
                        width,
                        wall_terrain_id,
                        &footprint,
                        &connector_cells,
                        outside,
                        content,
                    ) else {
                        all_entrances_connect = false;
                        break;
                    };
                    connector_cells.extend(path);
                }
                if !all_entrances_connect {
                    continue;
                }
                let connector_cells = connector_cells.into_iter().collect::<Vec<_>>();
                let placement = GeneratedVaultPlacement {
                    vault: vault.clone(),
                    origin,
                    transform,
                    ordinal: 0,
                    connector_cells: connector_cells.clone(),
                };
                let mut proof_terrain = terrain.to_vec();
                apply_generated_vault_placement(
                    &mut proof_terrain,
                    width,
                    corridor_terrain_id,
                    &placement,
                );
                if generated_terrain_is_connected(&proof_terrain, width, height, content) {
                    candidates.push(GeneratedVaultPlacementCandidate {
                        origin,
                        transform,
                        connector_cells,
                    });
                }
            }
        }
    }
    candidates
}

pub(in crate::game) fn apply_generated_vault_placement(
    terrain: &mut [String],
    width: u16,
    corridor_terrain_id: &str,
    placement: &GeneratedVaultPlacement,
) {
    paint_generated_vault(terrain, width, placement);
    for position in &placement.connector_cells {
        set_generated_terrain(terrain, width, *position, corridor_terrain_id);
    }
}

pub(in crate::game) fn paint_generated_vault(
    terrain: &mut [String],
    width: u16,
    placement: &GeneratedVaultPlacement,
) {
    for local_y in 0..placement.vault.height {
        for local_x in 0..placement.vault.width {
            let local = transformed_vault_position(
                &placement.vault,
                placement.transform,
                ContentPosition {
                    x: local_x,
                    y: local_y,
                },
            );
            set_generated_terrain(
                terrain,
                width,
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                },
                &placement.vault.base_terrain_id,
            );
        }
    }

    for terrain_override in &placement.vault.terrain_overrides {
        for position in &terrain_override.positions {
            let local =
                transformed_vault_position(&placement.vault, placement.transform, *position);
            set_generated_terrain(
                terrain,
                width,
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                },
                &terrain_override.terrain_id,
            );
        }
    }
}

pub(in crate::game) fn assign_generated_rooms_to_regions(
    rooms: &[GeneratedRoom],
    region_count: usize,
) -> Vec<usize> {
    if region_count == 0 {
        return Vec::new();
    }
    debug_assert!(region_count <= rooms.len());
    let anchors = if region_count == 1 {
        vec![0]
    } else {
        (0..region_count)
            .map(|index| index * (rooms.len() - 1) / (region_count - 1))
            .collect::<Vec<_>>()
    };
    rooms
        .iter()
        .map(|room| {
            let center = room.center();
            anchors
                .iter()
                .enumerate()
                .min_by_key(|(region_index, anchor)| {
                    let anchor_center = rooms[**anchor].center();
                    (
                        center.x.abs_diff(anchor_center.x) + center.y.abs_diff(anchor_center.y),
                        *region_index,
                    )
                })
                .map(|(region_index, _)| region_index)
                .expect("region floor must retain an anchor")
        })
        .collect()
}

pub(in crate::game) fn generated_room_cells(room: &GeneratedRoom) -> Vec<Position> {
    (room.y..room.y + room.height)
        .flat_map(|y| {
            (room.x..room.x + room.width).filter_map(move |x| {
                let position = Position { x, y };
                room.contains(position).then_some(position)
            })
        })
        .collect()
}

pub(in crate::game) fn allocate_generated_region_placements(
    regions: &[GeneratedRegion],
    terrain: &[String],
    width: u16,
    content: &ContentCatalog,
    occupied: &BTreeSet<Position>,
    actor_placements: u16,
    loot_placements: u16,
) -> (Vec<u16>, Vec<u16>) {
    let region_count = u16::try_from(regions.len()).expect("regional count must fit u16");
    debug_assert!(actor_placements >= region_count);
    debug_assert!(loot_placements >= region_count);

    let mut remaining_capacity = regions
        .iter()
        .map(|region| {
            generated_region_open_positions(region, terrain, width, content, occupied).len()
        })
        .collect::<Vec<_>>();
    assert!(
        remaining_capacity.iter().all(|capacity| *capacity >= 2),
        "each generated region must retain room space for an actor and loot"
    );

    let mut actor_allocations = vec![1_u16; regions.len()];
    let mut loot_allocations = vec![1_u16; regions.len()];
    for capacity in &mut remaining_capacity {
        *capacity -= 2;
    }

    let mut actor_remaining = actor_placements - region_count;
    let mut region_index = 0_usize;
    while actor_remaining > 0 {
        if remaining_capacity[region_index] > 0 {
            actor_allocations[region_index] += 1;
            remaining_capacity[region_index] -= 1;
            actor_remaining -= 1;
        }
        region_index = (region_index + 1) % regions.len();
        assert!(
            actor_remaining == 0 || remaining_capacity.iter().any(|capacity| *capacity > 0),
            "generated regions must retain enough room space for actor placements"
        );
    }

    let mut loot_remaining = loot_placements - region_count;
    while loot_remaining > 0 {
        if remaining_capacity[region_index] > 0 {
            loot_allocations[region_index] += 1;
            remaining_capacity[region_index] -= 1;
            loot_remaining -= 1;
        }
        region_index = (region_index + 1) % regions.len();
        assert!(
            loot_remaining == 0 || remaining_capacity.iter().any(|capacity| *capacity > 0),
            "generated regions must retain enough room space for loot placements"
        );
    }

    (actor_allocations, loot_allocations)
}

pub(in crate::game) fn generated_region_open_positions(
    region: &GeneratedRegion,
    terrain: &[String],
    width: u16,
    content: &ContentCatalog,
    occupied: &BTreeSet<Position>,
) -> Vec<Position> {
    region
        .state
        .cells
        .iter()
        .copied()
        .filter(|position| !occupied.contains(position))
        .filter(|position| {
            content
                .terrain(&terrain[generated_terrain_index(width, *position)])
                .is_some_and(|definition| definition.walkable)
        })
        .collect()
}

pub(in crate::game) fn assign_generated_footprint_to_region(
    regions: &mut [GeneratedRegion],
    rooms: &[GeneratedRoom],
    anchor: Position,
    footprint: impl IntoIterator<Item = Position>,
) {
    let footprint = footprint.into_iter().collect::<BTreeSet<_>>();
    let Some(region_index) = regions
        .iter()
        .enumerate()
        .min_by_key(|(region_index, region)| {
            let distance = region
                .room_ids
                .iter()
                .filter_map(|room_id| rooms.iter().find(|room| room.id == *room_id))
                .map(|room| {
                    let center = room.center();
                    anchor.x.abs_diff(center.x) + anchor.y.abs_diff(center.y)
                })
                .min()
                .unwrap_or(u32::MAX);
            (distance, *region_index)
        })
        .map(|(region_index, _)| region_index)
    else {
        return;
    };
    for region in regions.iter_mut() {
        region
            .state
            .cells
            .retain(|position| !footprint.contains(position));
    }
    regions[region_index].state.cells.extend(footprint);
}

pub(in crate::game) fn carve_generated_room(
    terrain: &mut [String],
    width: u16,
    room: &GeneratedRoom,
    floor_terrain_id: &str,
) {
    for y in room.y..room.y + room.height {
        for x in room.x..room.x + room.width {
            let position = Position { x, y };
            if room.contains(position) {
                set_generated_terrain(terrain, width, position, floor_terrain_id);
            }
        }
    }
}

pub(in crate::game) fn carve_generated_corridor(
    terrain: &mut [String],
    width: u16,
    from: Position,
    to: Position,
    floor_terrain_id: &str,
) {
    for x in from.x.min(to.x)..=from.x.max(to.x) {
        set_generated_terrain(terrain, width, Position { x, y: from.y }, floor_terrain_id);
    }
    for y in from.y.min(to.y)..=from.y.max(to.y) {
        set_generated_terrain(terrain, width, Position { x: to.x, y }, floor_terrain_id);
    }
}

pub(in crate::game) fn terrain_feature_placement_candidates(
    terrain: &[String],
    width: u16,
    floor_terrain_id: &str,
    room_floor_terrain_ids: &BTreeSet<String>,
    rooms: &[GeneratedRoom],
    reserved: &BTreeSet<Position>,
    placement: TerrainFeaturePlacement,
) -> Vec<Position> {
    terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(width))
                    .expect("terrain feature x must fit i32"),
                y: i32::try_from(index / usize::from(width))
                    .expect("terrain feature y must fit i32"),
            };
            if reserved.contains(&position) {
                return None;
            }
            let inside_room = rooms.iter().any(|room| room.contains(position));
            match placement {
                TerrainFeaturePlacement::Room
                    if inside_room
                        && (room_floor_terrain_ids.is_empty()
                            && terrain_id == floor_terrain_id
                            || room_floor_terrain_ids.contains(terrain_id)) =>
                {
                    Some(position)
                }
                TerrainFeaturePlacement::Corridor
                    if !inside_room && terrain_id == floor_terrain_id =>
                {
                    Some(position)
                }
                _ => None,
            }
        })
        .collect()
}

pub(in crate::game) fn set_generated_terrain(
    terrain: &mut [String],
    width: u16,
    position: Position,
    terrain_id: &str,
) {
    let index = generated_terrain_index(width, position);
    terrain[index] = terrain_id.to_owned();
}

pub(in crate::game) fn primary_floor_connection_ids(
    definition: &ProceduralFloorDefinition,
) -> (Option<&str>, Option<&str>) {
    let primary_up_id = definition.entry_connection_id.as_deref().or_else(|| {
        definition
            .connections
            .iter()
            .find(|connection| connection.terrain_id == definition.up_stair_terrain_id)
            .map(|connection| connection.id.as_str())
    });
    let primary_down_id = definition
        .down_stair_terrain_id
        .as_ref()
        .and_then(|terrain_id| {
            definition
                .connections
                .iter()
                .find(|connection| {
                    connection.terrain_id == *terrain_id
                        && primary_up_id != Some(connection.id.as_str())
                })
                .map(|connection| connection.id.as_str())
        });
    (primary_up_id, primary_down_id)
}

pub(in crate::game) fn generated_wall_positions(
    definition: &ProceduralFloorDefinition,
    terrain: &[String],
) -> Vec<Position> {
    let mut positions = (1..definition.height - 1)
        .flat_map(|y| {
            (1..definition.width - 1).filter_map(move |x| {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                (terrain[generated_terrain_index(definition.width, position)]
                    == definition.wall_terrain_id)
                    .then_some(position)
            })
        })
        .collect::<Vec<_>>();
    positions.sort_by_key(|position| (position.y, position.x));
    positions
}
