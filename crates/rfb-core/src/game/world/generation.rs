// SPDX-License-Identifier: MPL-2.0
use std::collections::BTreeSet;

use rfb_content::{
    ContentCatalog, ContentPosition, EncounterFormation, InlineFloorMapDefinition, ItemSpawn,
    ProceduralFloorDefinition, ProceduralNormalAllocationDefinition, TaskLocationDefinition,
    TerrainFeaturePlacement, VaultDefinition, VaultTransform,
};
use rfb_protocol::Position;

use super::super::monster_ecology::OriginalGroupRole;
use super::super::movement::actor_can_cross_terrain;
use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct GeneratedRoom {
    pub(in crate::game) id: String,
    pub(in crate::game) x: i32,
    pub(in crate::game) y: i32,
    pub(in crate::game) width: i32,
    pub(in crate::game) height: i32,
    pub(in crate::game) shape: ProceduralRoomShape,
    pub(in crate::game) carved_cells: BTreeSet<Position>,
}

fn generated_cavern_room_area(width: i32, height: i32) -> u32 {
    (u32::try_from(width * height).expect("room area must fit u32") * 5 / 8).max(10)
}

impl GeneratedRoom {
    pub(in crate::game) fn center(&self) -> Position {
        Position {
            x: self.x + self.width / 2,
            y: self.y + self.height / 2,
        }
    }

    pub(in crate::game) fn contains(&self, position: Position) -> bool {
        if position.x < self.x
            || position.x >= self.x + self.width
            || position.y < self.y
            || position.y >= self.y + self.height
        {
            return false;
        }
        match self.shape {
            ProceduralRoomShape::Rectangle => true,
            ProceduralRoomShape::Cross => {
                position.x == self.center().x || position.y == self.center().y
            }
            ProceduralRoomShape::Cavern => self.carved_cells.contains(&position),
        }
    }

    pub(in crate::game) fn area(&self) -> u32 {
        match self.shape {
            ProceduralRoomShape::Rectangle => (self.width * self.height) as u32,
            ProceduralRoomShape::Cross => (self.width + self.height - 1) as u32,
            ProceduralRoomShape::Cavern => {
                if self.carved_cells.is_empty() {
                    generated_cavern_room_area(self.width, self.height)
                } else {
                    u32::try_from(self.carved_cells.len()).expect("room area must fit u32")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct GeneratedVaultPlacement {
    pub(in crate::game) vault: VaultDefinition,
    pub(in crate::game) origin: Position,
    pub(in crate::game) transform: VaultTransform,
    pub(in crate::game) ordinal: u16,
    pub(in crate::game) connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedVaultPlacementCandidate {
    origin: Position,
    transform: VaultTransform,
    connector_cells: Vec<Position>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedPitPlacement {
    definition: ProceduralPitDefinition,
    origin: Position,
    outer_entrance: Position,
    inner_entrance: Position,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::game) struct GeneratedTerrainFeature {
    pub(in crate::game) terrain_id: String,
    pub(in crate::game) position: Position,
}

pub(in crate::game) struct TerrainFeaturePlacementContext<'a> {
    pub(in crate::game) rooms: &'a [GeneratedRoom],
    pub(in crate::game) reserved: &'a BTreeSet<Position>,
    pub(in crate::game) floor_terrain_id: &'a str,
    pub(in crate::game) room_floor_terrain_ids: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GeneratedRegion {
    state: FloorRegionState,
    room_ids: Vec<String>,
    floor_terrain_id: String,
}

use super::geometry::{
    generated_terrain_index, generated_terrain_is_connected, maze_floor_anchors,
    maze_floor_distances, maze_floor_path, transformed_vault_dimensions,
    transformed_vault_position, vault_connector_path, vault_entrance_outward,
};

fn generated_non_entry_room_id(rooms: &[GeneratedRoom], ordinal: u16) -> &str {
    let room_index = 1 + usize::from(ordinal) % (rooms.len() - 1);
    &rooms[room_index].id
}

fn choose_generated_maze_position(
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

fn formation_placement_candidates(
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

fn free_vault_placement_candidates(
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

fn apply_generated_vault_placement(
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

fn paint_generated_vault(terrain: &mut [String], width: u16, placement: &GeneratedVaultPlacement) {
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

fn assign_generated_rooms_to_regions(rooms: &[GeneratedRoom], region_count: usize) -> Vec<usize> {
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

fn generated_room_cells(room: &GeneratedRoom) -> Vec<Position> {
    (room.y..room.y + room.height)
        .flat_map(|y| {
            (room.x..room.x + room.width).filter_map(move |x| {
                let position = Position { x, y };
                room.contains(position).then_some(position)
            })
        })
        .collect()
}

fn allocate_generated_region_placements(
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

fn generated_region_open_positions(
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

fn generated_actor_can_enter_position(
    content: &ContentCatalog,
    terrain: &[String],
    width: u16,
    actor_kind_id: &str,
    position: Position,
) -> bool {
    let Some(actor) = content.actor(actor_kind_id) else {
        return false;
    };
    content
        .terrain(&terrain[generated_terrain_index(width, position)])
        .is_some_and(|tile| actor_can_cross_terrain(actor, tile))
}

fn assign_generated_footprint_to_region(
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

fn carve_generated_room(
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

fn carve_generated_corridor(
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

fn generated_remote_room_center(rooms: &[GeneratedRoom]) -> Position {
    let entry = rooms[0].center();
    rooms
        .iter()
        .skip(1)
        .map(GeneratedRoom::center)
        .max_by_key(|position| {
            (
                entry.x.abs_diff(position.x) + entry.y.abs_diff(position.y),
                position.y,
                position.x,
            )
        })
        .expect("room layout must retain a remote room")
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

fn primary_floor_connection_ids(
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

fn generated_wall_positions(
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

impl Game {
    fn inline_item_instance(
        &mut self,
        definition: &ProceduralFloorDefinition,
        spawn: &ItemSpawn,
        position: ContentPosition,
    ) -> ItemInstance {
        let materialization = materialize_ego_with_rng(
            &self.content,
            &mut self.rng,
            &spawn.kind_id,
            spawn.affix_ids.clone(),
            |_| definition.depth,
            definition.depth,
        );
        let mut item = ItemInstance {
            id: spawn.instance_id.clone(),
            kind_id: spawn.kind_id.clone(),
            quantity: spawn.quantity,
            inscription: None,
            origin_actor_kind_id: None,
            origin_kind: None,
            damage_dice_override: None,
            discount_percent: 0,
            quality: item_quality_dto(spawn.quality),
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: ItemEnchantmentsDto::default(),
            curse: initial_item_curse(&self.content, &spawn.kind_id),
            permanent_destruction_immunities: Default::default(),
            activation: None,
            charges: None,
            fuel: initial_item_fuel(&self.content, &spawn.kind_id),
            device_recovery_progress: 0,
            captured_actor: None,
            location: ItemLocation::Ground(Position {
                x: i32::from(position.x),
                y: i32::from(position.y),
            }),
        };
        materialization.apply_to(&mut item);
        item
    }

    fn generate_inline_floor(
        &mut self,
        definition: &ProceduralFloorDefinition,
        inline_map: &InlineFloorMapDefinition,
        dungeon_instance_id: Option<String>,
    ) -> Result<FloorState, CoreError> {
        let width = definition.width;
        let height = definition.height;
        let mut terrain =
            vec![definition.wall_terrain_id.clone(); usize::from(width) * usize::from(height)];
        for terrain_override in &inline_map.terrain_overrides {
            for position in &terrain_override.positions {
                let terrain_id = if terrain_override.chance_percent == 100
                    || self.rng.bounded(100) < u64::from(terrain_override.chance_percent)
                {
                    &terrain_override.terrain_id
                } else {
                    terrain_override
                        .otherwise_terrain_id
                        .as_ref()
                        .expect("validated random inline terrain must have a fallback")
                };
                set_generated_terrain(
                    &mut terrain,
                    width,
                    Position {
                        x: i32::from(position.x),
                        y: i32::from(position.y),
                    },
                    terrain_id,
                );
            }
        }

        let mut entities = Vec::new();
        for spawn in &inline_map.actor_spawns {
            let actor = self
                .content
                .actor(&spawn.kind_id)
                .expect("validated inline actor must remain available")
                .clone();
            entities.push(spawn_actor_from_definition(
                &mut self.rng,
                &actor,
                &spawn.instance_id,
                Position {
                    x: i32::from(spawn.position.x),
                    y: i32::from(spawn.position.y),
                },
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(&actor),
            ));
        }
        if let Some(formation) = &inline_map.monster_formation {
            let mut candidates = formation
                .candidate_actor_kind_ids
                .iter()
                .map(|actor_kind_id| {
                    self.content
                        .actor(actor_kind_id)
                        .expect("validated formation actor must remain available")
                        .clone()
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|actor| {
                actor
                    .allocation
                    .as_ref()
                    .expect("validated formation actor must retain allocation")
                    .legacy_index
            });
            let weights = candidates
                .iter()
                .map(|actor| {
                    100 / actor
                        .allocation
                        .as_ref()
                        .expect("validated formation actor must retain allocation")
                        .rarity
                })
                .collect::<Vec<_>>();
            let mut drawn = (0..formation.draw_count)
                .map(|_| candidates[self.roll_weighted_index(&weights)].clone())
                .collect::<Vec<_>>();
            drawn.sort_by(|left, right| {
                right.level.cmp(&left.level).then_with(|| {
                    left.allocation
                        .as_ref()
                        .expect("validated formation actor must retain allocation")
                        .legacy_index
                        .cmp(
                            &right
                                .allocation
                                .as_ref()
                                .expect("validated formation actor must retain allocation")
                                .legacy_index,
                        )
                })
            });
            for ((draw_index, position), actor) in formation
                .placement_indices
                .iter()
                .zip(&formation.positions)
                .map(|(index, position)| ((*index, position), &drawn[usize::from(*index)]))
            {
                entities.push(spawn_actor_from_definition(
                    &mut self.rng,
                    actor,
                    &format!("{}.formation.{}", definition.id, draw_index + 1),
                    Position {
                        x: i32::from(position.x),
                        y: i32::from(position.y),
                    },
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(actor),
                ));
            }
        }

        let mut items = inline_map
            .item_spawns
            .iter()
            .map(|spawn| self.inline_item_instance(definition, spawn, spawn.position))
            .collect::<Vec<_>>();
        if let Some(pair) = &inline_map.scrambled_item_pair {
            let swap = self.rng.bounded(2) == 1;
            for (index, spawn) in pair.iter().enumerate() {
                let position = pair[if swap { 1 - index } else { index }].position;
                items.push(self.inline_item_instance(definition, spawn, position));
            }
        }
        if let Some(pair) = &inline_map.scrambled_item_loot_pair {
            let swap = self.rng.bounded(2) == 1;
            for (index, spawn) in pair.item_spawns.iter().enumerate() {
                let position = if swap {
                    pair.loot_spawns[index].position
                } else {
                    spawn.position
                };
                items.push(self.inline_item_instance(definition, spawn, position));
            }
            for (index, spawn) in pair.loot_spawns.iter().enumerate() {
                let position = if swap {
                    pair.item_spawns[index].position
                } else {
                    spawn.position
                };
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: "inline-map".to_owned(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(Position {
                        x: i32::from(position.x),
                        y: i32::from(position.y),
                    }),
                )?);
            }
        }
        for spawn in &inline_map.loot_spawns {
            items.extend(self.generate_loot_instances(
                &LootContext {
                    table_id: spawn.loot_table_id.clone(),
                    floor_id: definition.id.clone(),
                    depth: definition.depth,
                    source: LootSource::FloorRoom {
                        room_id: "inline-map".to_owned(),
                        spawn_id: spawn.id.clone(),
                    },
                },
                ItemLocation::Ground(Position {
                    x: i32::from(spawn.position.x),
                    y: i32::from(spawn.position.y),
                }),
            )?);
        }

        Ok(FloorState {
            id: definition.id.clone(),
            dungeon_instance_id,
            reproduction_suppressed: false,
            width,
            height,
            terrain,
            glow: vec![false; usize::from(width) * usize::from(height)],
            player_position: Position {
                x: i32::from(inline_map.player_position.x),
                y: i32::from(inline_map.player_position.y),
            },
            entities,
            items,
            gold_piles: Vec::new(),
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            connections: Vec::new(),
            regions: Vec::new(),
        })
    }

    pub(in crate::game) fn generate_procedural_floor(
        &mut self,
        definition: &ProceduralFloorDefinition,
        dungeon_instance_id: Option<String>,
    ) -> Result<FloorState, CoreError> {
        self.monster_division_remainders.clear();
        if let Some(inline_map) = &definition.inline_map {
            return self.generate_inline_floor(definition, inline_map, dungeon_instance_id);
        }
        let maze_only = definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.mode == ProceduralLayoutMode::MazeOnly);
        let selected_region_entries = if let Some(table_id) = &definition.region_table_id {
            let table = self
                .content
                .region_table(table_id)
                .expect("validated region table must remain available")
                .clone();
            let mut eligible = table
                .entries
                .into_iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            let placement_count = definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.region_placements)
                .expect("validated region floor must retain a placement budget");
            let mut selected = Vec::with_capacity(usize::from(placement_count));
            for _ in 0..placement_count {
                let weights = eligible
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = self.roll_weighted_index(&weights);
                selected.push(eligible.remove(selected_index));
            }
            selected
        } else {
            Vec::new()
        };
        let eligible_themes = definition
            .theme_table_id
            .as_ref()
            .and_then(|table_id| self.content.theme_table(table_id))
            .map(|table| {
                table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_theme = if eligible_themes.is_empty() {
            None
        } else if eligible_themes.len() == 1 {
            Some(eligible_themes[0].clone())
        } else {
            let weights = eligible_themes
                .iter()
                .map(|entry| entry.weight)
                .collect::<Vec<_>>();
            Some(eligible_themes[self.roll_weighted_index(&weights)].clone())
        };
        let generated_floor_terrain_id = selected_theme
            .as_ref()
            .map(|entry| entry.floor_terrain_id.clone())
            .unwrap_or_else(|| definition.floor_terrain_id.clone());
        let uses_spatial_vault_budget =
            definition.generation_budget.as_ref().is_some_and(|budget| {
                budget.vault_placements.is_some() && budget.vault_area_tiles.is_some()
            });
        let eligible_vault_candidates = selected_theme
            .as_ref()
            .map(|theme| {
                theme
                    .vault_candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.min_depth <= definition.depth
                            && definition.depth <= candidate.max_depth
                            && self
                                .content
                                .vault(&candidate.vault_id)
                                .is_some_and(|vault| {
                                    uses_spatial_vault_budget
                                        || vault.width <= 6
                                            && vault.height <= 5
                                            && vault.entrance_positions.len() == 1
                                            && vault.entrance_positions[0].x == vault.width / 2
                                            && vault.entrance_positions[0].y == 0
                                })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let legacy_vault = if uses_spatial_vault_budget || maze_only {
            None
        } else if eligible_vault_candidates.is_empty() {
            definition
                .vault_id
                .as_ref()
                .and_then(|vault_id| self.content.vault(vault_id))
                .cloned()
        } else if eligible_vault_candidates.len() == 1 {
            self.content
                .vault(&eligible_vault_candidates[0].vault_id)
                .cloned()
        } else {
            let weights = eligible_vault_candidates
                .iter()
                .map(|candidate| candidate.weight)
                .collect::<Vec<_>>();
            let vault_id = &eligible_vault_candidates[self.roll_weighted_index(&weights)].vault_id;
            self.content.vault(vault_id).cloned()
        };
        let guardian = definition.guardian.as_ref().filter(|_| {
            definition.dungeon_id.as_ref().is_some_and(|dungeon_id| {
                self.dungeon_states
                    .get(dungeon_id)
                    .is_some_and(|state| !state.guardian_defeated)
            })
        });
        let task_definitions = self
            .content
            .world(&self.world_id)
            .map(|world| {
                if let Some(task_id) = definition.task_id.as_deref() {
                    task_definition(world, task_id)
                        .cloned()
                        .into_iter()
                        .collect::<Vec<_>>()
                } else {
                    world
                        .tasks
                        .iter()
                        .filter(|task| {
                            task_applies_to_floor(task, definition)
                                && self.task_states.get(&task.id).is_some_and(|state| {
                                    matches!(
                                        state.status,
                                        TaskStatusKindDto::Active | TaskStatusKindDto::Taken
                                    )
                                })
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                }
            })
            .unwrap_or_default();
        let completion_exit_terrain_ids = task_definitions
            .iter()
            .filter_map(|task| task.completion_exit_terrain_id.clone())
            .collect::<BTreeSet<_>>();
        let task_objectives = task_definitions
            .iter()
            .flat_map(|task| {
                task.objectives
                    .iter()
                    .enumerate()
                    .filter_map(|(index, objective)| {
                        let placement = task.target_placements.iter().find(|placement| {
                            placement.objective_index as usize == index
                                && placement.floor_id == definition.id
                        });
                        let applies = objective.floor_id.as_deref() == Some(definition.id.as_str())
                            || placement.is_some()
                            || (objective.floor_id.is_none()
                                && !task
                                    .target_placements
                                    .iter()
                                    .any(|candidate| candidate.objective_index as usize == index));
                        let requires_placement = matches!(
                            objective.kind,
                            TaskObjectiveKind::KillActor | TaskObjectiveKind::KillActorKind
                        );
                        (applies && (!requires_placement || placement.is_some())).then(|| {
                            (
                                task.id.clone(),
                                index,
                                objective.clone(),
                                placement.cloned(),
                                matches!(
                                    &task.location,
                                    TaskLocationDefinition::DungeonDepth { .. }
                                ),
                            )
                        })
                    })
            })
            .collect::<Vec<_>>();
        let width = definition.width;
        let height = definition.height;
        let mut terrain =
            vec![definition.wall_terrain_id.clone(); usize::from(width) * usize::from(height)];
        let cavern_origin = definition.layout.as_ref().and_then(|layout| {
            layout.cavern.as_ref().map(|cavern| {
                self.generate_connected_cavern(definition, &cavern.terrain_id, &mut terrain)
            })
        });
        let lake_origin = definition.layout.as_ref().and_then(|layout| {
            layout.lake.as_ref().map(|lake| {
                self.generate_connected_lake(
                    definition,
                    &lake.deep_terrain_id,
                    &lake.shallow_terrain_id,
                    &mut terrain,
                )
            })
        });
        let maze_walkable = if maze_only {
            let maze = definition
                .layout
                .as_ref()
                .and_then(|layout| layout.maze.as_ref())
                .expect("validated maze-only layout must retain maze geometry");
            self.generate_maze(definition, maze, &generated_floor_terrain_id, &mut terrain)
        } else {
            BTreeSet::new()
        };
        let rooms = if maze_only {
            Vec::new()
        } else if let Some(layout) = &definition.layout {
            self.generate_budgeted_rooms(
                definition,
                layout
                    .rooms
                    .as_ref()
                    .expect("validated rooms layout must retain room geometry"),
            )
        } else {
            let room_width = 6_i32;
            let room_height = 5_i32;
            let first_x = 1 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let first_y = 1 + i32::try_from(self.rng.bounded(4)).unwrap_or(0);
            let second_x = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            let second_y = 11 + i32::try_from(self.rng.bounded(3)).unwrap_or(0);
            vec![
                GeneratedRoom {
                    id: "entry".to_owned(),
                    x: first_x,
                    y: first_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                    carved_cells: BTreeSet::new(),
                },
                GeneratedRoom {
                    id: "remote".to_owned(),
                    x: second_x,
                    y: second_y,
                    width: room_width,
                    height: room_height,
                    shape: ProceduralRoomShape::Rectangle,
                    carved_cells: BTreeSet::new(),
                },
            ]
        };
        let content_rooms = if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.pit.is_some())
        {
            &rooms[..rooms.len() - 1]
        } else {
            rooms.as_slice()
        };
        let room_region_indexes =
            assign_generated_rooms_to_regions(content_rooms, selected_region_entries.len());
        let mut generated_regions = selected_region_entries
            .iter()
            .enumerate()
            .map(|(region_index, entry)| {
                let theme = self
                    .content
                    .theme_table(&entry.theme_table_id)
                    .and_then(|table| {
                        table
                            .entries
                            .iter()
                            .find(|theme| theme.theme_id == entry.theme_id)
                    })
                    .expect("validated region theme must remain available");
                let room_ids = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .map(|(room, _)| room.id.clone())
                    .collect::<Vec<_>>();
                let mut cells = content_rooms
                    .iter()
                    .zip(&room_region_indexes)
                    .filter(|(_, assigned_region)| **assigned_region == region_index)
                    .flat_map(|(room, _)| generated_room_cells(room))
                    .collect::<Vec<_>>();
                cells.sort();
                GeneratedRegion {
                    state: FloorRegionState {
                        region_id: entry.region_id.clone(),
                        theme_id: entry.theme_id.clone(),
                        encounter_table_id: entry.encounter_table_id.clone(),
                        loot_table_id: entry.loot_table_id.clone(),
                        cells,
                    },
                    room_ids,
                    floor_terrain_id: theme.floor_terrain_id.clone(),
                }
            })
            .collect::<Vec<_>>();
        for (room_index, room) in rooms.iter().enumerate() {
            let room_terrain_id = room_region_indexes
                .get(room_index)
                .and_then(|region_index| generated_regions.get(*region_index))
                .map_or(generated_floor_terrain_id.as_str(), |region| {
                    region.floor_terrain_id.as_str()
                });
            carve_generated_room(&mut terrain, width, room, room_terrain_id);
        }
        let cave_room_layout = rooms
            .iter()
            .any(|room| room.shape == ProceduralRoomShape::Cavern);
        let (first_center, second_center) = if maze_only {
            maze_floor_anchors(&maze_walkable)
        } else if cave_room_layout {
            (rooms[0].center(), generated_remote_room_center(&rooms))
        } else {
            (rooms[0].center(), rooms[1].center())
        };
        let legacy_vault_origin = legacy_vault.as_ref().map(|vault| Position {
            x: second_center.x - i32::from(vault.entrance_positions[0].x),
            y: rooms
                .get(1)
                .expect("legacy vault placement requires a remote room")
                .y,
        });
        if let Some(destroyed) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.destroyed.as_ref())
        {
            self.generate_destroyed_region(definition, &destroyed.terrain_id, &mut terrain);
        }
        if let Some(river) = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.river.as_ref())
            && river
                .chance_one_in
                .is_none_or(|chance| self.rng.bounded(u64::from(chance)) == 0)
        {
            let (deep_terrain_id, shallow_terrain_id) = river
                .alternative
                .as_ref()
                .filter(|alternative| {
                    self.rng.bounded(u64::from(alternative.chance_denominator))
                        < u64::from(alternative.chance_numerator)
                })
                .map_or(
                    (
                        river.deep_terrain_id.as_str(),
                        river.shallow_terrain_id.as_str(),
                    ),
                    |alternative| {
                        (
                            alternative.deep_terrain_id.as_str(),
                            alternative.shallow_terrain_id.as_str(),
                        )
                    },
                );
            self.generate_river(
                definition,
                deep_terrain_id,
                shallow_terrain_id,
                lake_origin.unwrap_or(Position {
                    x: i32::from(width / 2),
                    y: i32::from(height / 2),
                }),
                &mut terrain,
            );
        }
        if definition
            .layout
            .as_ref()
            .is_some_and(|layout| layout.destroyed.is_some() || layout.river.is_some())
        {
            for room in &rooms {
                let room_index = rooms
                    .iter()
                    .position(|candidate| candidate.id == room.id)
                    .expect("generated room must retain its stable index");
                let room_terrain_id = room_region_indexes
                    .get(room_index)
                    .and_then(|region_index| generated_regions.get(*region_index))
                    .map_or(generated_floor_terrain_id.as_str(), |region| {
                        region.floor_terrain_id.as_str()
                    });
                carve_generated_room(&mut terrain, width, room, room_terrain_id);
            }
        }
        if cave_room_layout {
            self.carve_cave_room_network(&mut terrain, width, &rooms, &generated_floor_terrain_id);
        } else {
            for connected_rooms in rooms.windows(2) {
                carve_generated_corridor(
                    &mut terrain,
                    width,
                    connected_rooms[0].center(),
                    connected_rooms[1].center(),
                    &generated_floor_terrain_id,
                );
            }
        }
        if let Some(cavern_origin) = cavern_origin {
            carve_generated_corridor(
                &mut terrain,
                width,
                first_center,
                cavern_origin,
                &generated_floor_terrain_id,
            );
        }
        if let Some(layout) = &definition.layout
            && !layout.streamers.is_empty()
        {
            self.generate_streamers(definition, &layout.streamers, &mut terrain);
        }
        let pit_placement = definition
            .layout
            .as_ref()
            .and_then(|layout| layout.pit.as_ref())
            .map(|pit| {
                self.place_classic_pit(
                    definition,
                    pit,
                    rooms[rooms.len() - 2].center(),
                    &generated_floor_terrain_id,
                    &mut terrain,
                )
            });
        let door_position = (definition
            .layout
            .as_ref()
            .is_none_or(|layout| layout.place_doors)
            && !maze_only
            && !cave_room_layout)
            .then_some(Position {
                x: (first_center.x + second_center.x) / 2,
                y: first_center.y,
            });
        if let Some(door_position) = door_position {
            set_generated_terrain(
                &mut terrain,
                width,
                door_position,
                &definition.closed_door_terrain_id,
            );
        }
        let down_stair_position = if maze_only || cave_room_layout {
            second_center
        } else {
            Position {
                x: first_center.x - 1,
                y: first_center.y,
            }
        };
        let fixed_trap_position = if maze_only {
            let route = maze_floor_path(&maze_walkable, first_center, second_center);
            route[route.len() / 2]
        } else if cave_room_layout {
            generated_room_cells(&rooms[0])
                .into_iter()
                .filter(|position| *position != first_center)
                .min_by_key(|position| {
                    (
                        first_center.x.abs_diff(position.x) + first_center.y.abs_diff(position.y),
                        position.y,
                        position.x,
                    )
                })
                .expect("cave entry room must retain a trap cell")
        } else {
            Position {
                x: first_center.x,
                y: first_center.y + 1,
            }
        };
        let mut floor_connections = if definition.connections.is_empty() {
            set_generated_terrain(
                &mut terrain,
                width,
                first_center,
                &definition.up_stair_terrain_id,
            );
            if let Some(down_stair_terrain_id) = &definition.down_stair_terrain_id {
                set_generated_terrain(
                    &mut terrain,
                    width,
                    down_stair_position,
                    down_stair_terrain_id,
                );
            }
            Vec::new()
        } else {
            let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
            for (connection_id, position) in [
                (primary_up_id, first_center),
                (primary_down_id, down_stair_position),
            ] {
                if let Some(connection) = connection_id.and_then(|connection_id| {
                    definition
                        .connections
                        .iter()
                        .find(|connection| connection.id == connection_id)
                }) {
                    set_generated_terrain(&mut terrain, width, position, &connection.terrain_id);
                }
            }
            Vec::new()
        };
        set_generated_terrain(
            &mut terrain,
            width,
            fixed_trap_position,
            &definition.trap_terrain_id,
        );
        let vault_placements = if let Some(vault) = legacy_vault.clone() {
            let placement = GeneratedVaultPlacement {
                vault,
                origin: legacy_vault_origin.expect("present vault must have an origin"),
                transform: VaultTransform::Identity,
                ordinal: 1,
                connector_cells: Vec::new(),
            };
            paint_generated_vault(&mut terrain, width, &placement);
            vec![placement]
        } else if uses_spatial_vault_budget {
            self.select_spatial_vault_placements(
                definition,
                &eligible_vault_candidates,
                guardian.is_some(),
                &generated_floor_terrain_id,
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        for placement in &vault_placements {
            let entrance = transformed_vault_position(
                &placement.vault,
                placement.transform,
                placement.vault.entrance_positions[0],
            );
            let anchor = Position {
                x: placement.origin.x + entrance.x,
                y: placement.origin.y + entrance.y,
            };
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            let footprint = (0..vault_height).flat_map(|y| {
                (0..vault_width).map(move |x| Position {
                    x: placement.origin.x + i32::from(x),
                    y: placement.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                anchor,
                footprint,
            );
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            let footprint = (0..total_height).flat_map(|y| {
                (0..total_width).map(move |x| Position {
                    x: pit.origin.x + i32::from(x),
                    y: pit.origin.y + i32::from(y),
                })
            });
            assign_generated_footprint_to_region(
                &mut generated_regions,
                content_rooms,
                pit.outer_entrance,
                footprint,
            );
        }
        if !definition.connections.is_empty() {
            floor_connections = place_generated_floor_connections(
                definition,
                first_center,
                down_stair_position,
                fixed_trap_position,
                &generated_floor_terrain_id,
                &mut terrain,
                &mut self.rng,
            )?;
        }
        let mut stair_reserved =
            BTreeSet::from([first_center, down_stair_position, fixed_trap_position]);
        if let Some(door_position) = door_position {
            stair_reserved.insert(door_position);
        }
        if guardian.is_some() {
            stair_reserved.insert(if maze_only || cave_room_layout {
                second_center
            } else {
                Position {
                    x: first_center.x + 1,
                    y: first_center.y,
                }
            });
        }
        for placement in &vault_placements {
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            for y in 0..vault_height {
                for x in 0..vault_width {
                    stair_reserved.insert(Position {
                        x: placement.origin.x + i32::from(x),
                        y: placement.origin.y + i32::from(y),
                    });
                }
            }
            stair_reserved.extend(placement.connector_cells.iter().copied());
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    stair_reserved.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
        }
        let extra_stair_positions = if floor_connections.is_empty() {
            self.place_configured_stairs(
                definition,
                first_center,
                definition
                    .down_stair_terrain_id
                    .as_ref()
                    .map(|_| down_stair_position),
                &stair_reserved,
                &mut terrain,
            )
        } else {
            BTreeSet::new()
        };
        let mut feature_reserved = BTreeSet::from([fixed_trap_position]);
        feature_reserved.extend(extra_stair_positions.iter().copied());
        if floor_connections.is_empty() {
            feature_reserved.insert(first_center);
        } else {
            feature_reserved.extend(
                floor_connections
                    .iter()
                    .map(|connection| connection.position),
            );
        }
        if let Some(door_position) = door_position {
            feature_reserved.insert(door_position);
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            feature_reserved.insert(down_stair_position);
        }
        for placement in &vault_placements {
            let (vault_width, vault_height) =
                transformed_vault_dimensions(&placement.vault, placement.transform);
            for y in 0..vault_height {
                for x in 0..vault_width {
                    feature_reserved.insert(Position {
                        x: placement.origin.x + i32::from(x),
                        y: placement.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.extend(placement.connector_cells.iter().copied());
        }
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    feature_reserved.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
            feature_reserved.insert(pit.outer_entrance);
            feature_reserved.insert(pit.inner_entrance);
        }
        let room_floor_terrain_ids = generated_regions
            .iter()
            .map(|region| region.floor_terrain_id.clone())
            .collect::<BTreeSet<_>>();
        let terrain_features = if let Some(table_id) = &definition.terrain_feature_table_id {
            let table = self
                .content
                .terrain_feature_table(table_id)
                .expect("validated terrain feature table must remain available")
                .clone();
            let eligible_entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= definition.depth && definition.depth <= entry.max_depth
                })
                .cloned()
                .collect::<Vec<_>>();
            self.place_terrain_features(
                definition,
                &eligible_entries,
                TerrainFeaturePlacementContext {
                    rooms: content_rooms,
                    reserved: &feature_reserved,
                    floor_terrain_id: &generated_floor_terrain_id,
                    room_floor_terrain_ids: &room_floor_terrain_ids,
                },
                &mut terrain,
            )
        } else {
            Vec::new()
        };
        let mut occupied = BTreeSet::from([first_center]);
        occupied.extend(extra_stair_positions.iter().copied());
        occupied.extend(
            floor_connections
                .iter()
                .map(|connection| connection.position),
        );
        if maze_only {
            occupied.insert(fixed_trap_position);
        }
        occupied.extend(terrain_features.iter().filter_map(|feature| {
            self.content
                .terrain(&feature.terrain_id)
                .is_some_and(|terrain| !terrain.walkable)
                .then_some(feature.position)
        }));
        if let Some(pit) = &pit_placement {
            let total_width = pit.definition.inner_width + 6;
            let total_height = pit.definition.inner_height + 6;
            for y in 0..total_height {
                for x in 0..total_width {
                    occupied.insert(Position {
                        x: pit.origin.x + i32::from(x),
                        y: pit.origin.y + i32::from(y),
                    });
                }
            }
        }
        for placement in &vault_placements {
            occupied.extend(
                placement
                    .vault
                    .encounter_groups
                    .iter()
                    .flat_map(|group| &group.member_positions)
                    .map(|local| {
                        let local = transformed_vault_position(
                            &placement.vault,
                            placement.transform,
                            *local,
                        );
                        Position {
                            x: placement.origin.x + local.x,
                            y: placement.origin.y + local.y,
                        }
                    }),
            );
            occupied.extend(placement.vault.loot_spawns.iter().map(|spawn| {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                }
            }));
        }
        if floor_connections.is_empty() && definition.down_stair_terrain_id.is_some() {
            occupied.insert(down_stair_position);
        }
        let guardian_position = guardian.map(|_| {
            if maze_only || cave_room_layout {
                second_center
            } else {
                Position {
                    x: first_center.x + 1,
                    y: first_center.y,
                }
            }
        });
        occupied.extend(guardian_position);
        let reserved_actor_slots = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.pit_actor_slots)
            .unwrap_or(0)
            .saturating_add(definition.nest.as_ref().map_or(0, |nest| nest.spawn_count))
            .saturating_add(if guardian.is_some() { 1 } else { 0 })
            .saturating_add(
                vault_placements
                    .iter()
                    .flat_map(|placement| &placement.vault.encounter_groups)
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault group size must fit u16")
                    })
                    .sum::<u16>(),
            );
        let mut entities = Vec::new();
        let mut regional_loot_allocations = Vec::new();
        if !generated_regions.is_empty() {
            let budget = definition
                .generation_budget
                .as_ref()
                .expect("validated region floor must retain a generation budget");
            let region_count = u16::try_from(generated_regions.len())
                .expect("validated region count must fit u16");
            if budget.group_placements.is_some() && budget.group_actor_slots.is_some() {
                let host = &generated_regions[0];
                let table = self
                    .content
                    .encounter_table(&host.state.encounter_table_id)
                    .expect("validated regional group table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let room_id = &host.room_ids[0];
                let id_prefix = format!("{}.region.{}", definition.id, host.state.region_id);
                entities.extend(self.generate_dynamic_encounter_groups(
                    definition,
                    &table,
                    &eligible_entries,
                    content_rooms,
                    room_id,
                    reserved_actor_slots,
                    region_count,
                    false,
                    &id_prefix,
                    &mut occupied,
                ));
            }
            let actor_budget = budget
                .actor_slots
                .saturating_sub(reserved_actor_slots)
                .saturating_sub(
                    u16::try_from(entities.len())
                        .expect("generated regional group size must fit u16"),
                );
            let loot_budget = budget.loot_placements.saturating_sub(
                vault_placements
                    .iter()
                    .map(|placement| {
                        u16::try_from(placement.vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16")
                    })
                    .sum::<u16>(),
            );
            let (regional_actor_allocations, loot_allocations) =
                allocate_generated_region_placements(
                    &generated_regions,
                    &terrain,
                    width,
                    &self.content,
                    &occupied,
                    actor_budget,
                    loot_budget,
                );
            regional_loot_allocations = loot_allocations;
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_actor_allocations[region_index];
                let table = self
                    .content
                    .encounter_table(&region.state.encounter_table_id)
                    .expect("validated region encounter table must remain available")
                    .clone();
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.group.is_none()
                            && entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for ordinal in 0..placements {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    let Some(position) = self.choose_generated_region_position_for_actor(
                        region,
                        &terrain,
                        width,
                        &occupied,
                        &entry.actor_kind_id,
                    ) else {
                        continue;
                    };
                    occupied.insert(position);
                    entities.push(self.generated_actor(
                        format!(
                            "{}.region.{}.encounter.plain.{}",
                            definition.id,
                            region.state.region_id,
                            ordinal + 1
                        ),
                        &entry.actor_kind_id,
                        position,
                    ));
                }
            }
        } else if let Some(table_id) = &definition.encounter_table_id {
            let table = self
                .content
                .encounter_table(table_id)
                .expect("validated floor encounter table must remain available")
                .clone();
            let encounter_rolls =
                definition
                    .generation_budget
                    .as_ref()
                    .map_or(table.rolls, |budget| {
                        table
                            .rolls
                            .min(budget.actor_slots.saturating_sub(reserved_actor_slots))
                    });
            let room_id = if legacy_vault.is_some() {
                "entry"
            } else {
                "remote"
            };
            if let Some(policy) = table.global_allocation.as_ref() {
                let mut target_floor_kind_ids = guardian
                    .iter()
                    .map(|guardian| guardian.actor_kind_id.clone())
                    .collect::<Vec<_>>();
                for ordinal in 0..encounter_rolls {
                    let placement_room_id = if maze_only {
                        "maze"
                    } else if definition.layout.is_some() {
                        generated_non_entry_room_id(content_rooms, ordinal)
                    } else {
                        room_id
                    };
                    let position = if maze_only {
                        choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                    } else {
                        self.choose_generated_room_position(
                            content_rooms,
                            placement_room_id,
                            &occupied,
                            None,
                        )
                    };
                    let required_terrain = self
                        .content
                        .terrain(&terrain[generated_terrain_index(width, position)])
                        .cloned()
                        .expect("generated actor terrain must remain available");
                    let Some(kind_id) = self.select_original_allocated_monster(
                        policy,
                        definition.depth,
                        definition.depth,
                        definition.task_id.as_deref(),
                        &target_floor_kind_ids,
                        None,
                        Some(&required_terrain),
                    ) else {
                        continue;
                    };
                    occupied.insert(position);
                    let members = self.plan_original_group(
                        policy,
                        &kind_id,
                        position,
                        definition.depth,
                        definition.task_id.as_deref(),
                        &terrain,
                        width,
                        height,
                        &mut occupied,
                    );
                    let pack_behavior = if members.is_empty() {
                        None
                    } else {
                        let leader_definition = self
                            .content
                            .actor(&kind_id)
                            .expect("allocated leader definition must remain available")
                            .clone();
                        Some(
                            self.original_pack_behavior(
                                &leader_definition,
                                members
                                    .iter()
                                    .any(|member| member.role == OriginalGroupRole::Escort),
                                members.len() + 1,
                            ),
                        )
                    };
                    let leader_id = format!("{}.encounter.{}", definition.id, ordinal + 1);
                    let pack_id = format!("{leader_id}.pack");
                    let mut leader = self.generated_actor(leader_id.clone(), &kind_id, position);
                    self.maybe_initialize_chameleon_form_on_terrain(
                        &mut leader,
                        Some(&required_terrain.id),
                    );
                    self.maybe_apply_shadower_appearance(&mut leader);
                    if let Some(behavior) = pack_behavior {
                        leader.pack = Some(MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: leader_id.clone(),
                            role: MonsterPackRoleDto::Leader,
                            behavior,
                        });
                    }
                    target_floor_kind_ids.push(kind_id);
                    entities.push(leader);
                    for (member_ordinal, member) in members.into_iter().enumerate() {
                        let mut actor = self.generated_actor(
                            format!("{leader_id}.companion.{}", member_ordinal + 1),
                            &member.kind_id,
                            member.position,
                        );
                        self.maybe_initialize_chameleon_form_on_terrain(
                            &mut actor,
                            Some(&terrain[generated_terrain_index(width, member.position)]),
                        );
                        self.maybe_apply_shadower_appearance(&mut actor);
                        actor.pack = Some(MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: leader_id.clone(),
                            role: MonsterPackRoleDto::Member,
                            behavior: pack_behavior.expect("non-empty pack must retain behavior"),
                        });
                        target_floor_kind_ids.push(member.kind_id);
                        entities.push(actor);
                    }
                }
            } else {
                let eligible_entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                if definition.generation_budget.as_ref().is_some_and(|budget| {
                    budget.group_placements.is_some() && budget.group_actor_slots.is_some()
                }) {
                    entities.extend(self.generate_dynamic_encounter_groups(
                        definition,
                        &table,
                        &eligible_entries,
                        content_rooms,
                        room_id,
                        reserved_actor_slots,
                        1,
                        true,
                        &definition.id,
                        &mut occupied,
                    ));
                } else {
                    for ordinal in 0..encounter_rolls {
                        let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                        let placement_room_id = if maze_only {
                            "maze"
                        } else if definition.layout.is_some() {
                            generated_non_entry_room_id(content_rooms, ordinal)
                        } else {
                            room_id
                        };
                        let position = if maze_only {
                            Some(choose_generated_maze_position(
                                &maze_walkable,
                                first_center,
                                &occupied,
                            ))
                        } else {
                            self.choose_generated_room_position_for_actor(
                                content_rooms,
                                placement_room_id,
                                &terrain,
                                width,
                                &occupied,
                                &entry.actor_kind_id,
                            )
                        };
                        let Some(position) = position else {
                            continue;
                        };
                        occupied.insert(position);
                        entities.push(self.generated_actor(
                            format!("{}.encounter.{}", definition.id, ordinal + 1),
                            &entry.actor_kind_id,
                            position,
                        ));
                    }
                }
                if let Some(nest) = &definition.nest {
                    let entry = &eligible_entries[self.roll_weighted_index(&weights)];
                    for ordinal in 0..nest.spawn_count {
                        let Some(position) = self.choose_generated_room_position_for_actor(
                            &rooms,
                            &nest.room_id,
                            &terrain,
                            width,
                            &occupied,
                            &entry.actor_kind_id,
                        ) else {
                            break;
                        };
                        occupied.insert(position);
                        let actor = self
                            .content
                            .actor(&entry.actor_kind_id)
                            .expect("validated nest actor must remain available")
                            .clone();
                        entities.push(spawn_actor_from_definition(
                            &mut self.rng,
                            &actor,
                            &format!("{}.nest.{}", definition.id, ordinal + 1),
                            position,
                            INITIAL_MONSTER_ENERGY_NEED,
                            actor_starts_alerted(&actor),
                        ));
                    }
                }
            }
        } else {
            for spawn in &definition.actor_spawns {
                let eligible_kind_ids = spawn
                    .actor_kind_ids
                    .iter()
                    .filter(|kind_id| {
                        self.content
                            .actor(kind_id)
                            .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let kind_index = usize::try_from(
                    self.rng.bounded(
                        u64::try_from(eligible_kind_ids.len())
                            .expect("validated actor candidate count must fit u64"),
                    ),
                )
                .expect("bounded actor candidate index must fit usize");
                let kind_id = &eligible_kind_ids[kind_index];
                let Some(position) = self.choose_generated_room_position_for_actor(
                    &rooms,
                    &spawn.room_id,
                    &terrain,
                    width,
                    &occupied,
                    kind_id,
                ) else {
                    continue;
                };
                occupied.insert(position);
                let actor = self
                    .content
                    .actor(kind_id)
                    .expect("validated procedural actor kind must remain available")
                    .clone();
                entities.push(spawn_actor_from_definition(
                    &mut self.rng,
                    &actor,
                    &spawn.instance_id,
                    position,
                    INITIAL_MONSTER_ENERGY_NEED,
                    actor_starts_alerted(&actor),
                ));
            }
        }
        if let Some(pit) = &pit_placement {
            entities.extend(self.generate_classic_pit_actors(definition, pit, &mut occupied));
        }
        for placement in &vault_placements {
            for group in &placement.vault.encounter_groups {
                let eligible_entries = group
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= definition.depth
                            && definition.depth <= entry.max_depth
                            && self
                                .content
                                .actor(&entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let weights = eligible_entries
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                for (ordinal, local) in group.member_positions.iter().enumerate() {
                    let entry = eligible_entries[self.roll_weighted_index(&weights)];
                    let actor = self
                        .content
                        .actor(&entry.actor_kind_id)
                        .expect("validated vault encounter actor must remain available");
                    let local =
                        transformed_vault_position(&placement.vault, placement.transform, *local);
                    let position = Position {
                        x: placement.origin.x + local.x,
                        y: placement.origin.y + local.y,
                    };
                    occupied.insert(position);
                    let instance_id = if uses_spatial_vault_budget {
                        format!(
                            "{}.vault.{}.{}.{}",
                            definition.id,
                            placement.ordinal,
                            group.id,
                            ordinal + 1
                        )
                    } else {
                        format!("{}.{}.{}", definition.id, group.id, ordinal + 1)
                    };
                    let actor = actor.clone();
                    entities.push(spawn_actor_from_definition(
                        &mut self.rng,
                        &actor,
                        &instance_id,
                        position,
                        INITIAL_MONSTER_ENERGY_NEED,
                        actor_starts_alerted(&actor),
                    ));
                }
            }
        }
        if let Some(guardian) = guardian {
            let actor_definition = self
                .content
                .actor(&guardian.actor_kind_id)
                .expect("validated dungeon guardian must remain available")
                .clone();
            let position = guardian_position.expect("present guardian must retain a position");
            let mut actor = spawn_actor_from_definition(
                &mut self.rng,
                &actor_definition,
                &guardian.instance_id,
                position,
                INITIAL_MONSTER_ENERGY_NEED,
                actor_starts_alerted(&actor_definition),
            );
            let original_policy = definition
                .encounter_table_id
                .as_ref()
                .and_then(|table_id| self.content.encounter_table(table_id))
                .and_then(|table| table.global_allocation.clone());
            if let Some(policy) = original_policy
                && actor_definition.allocation.is_some()
            {
                let members = self.plan_original_group(
                    &policy,
                    &guardian.actor_kind_id,
                    position,
                    definition.depth,
                    definition.task_id.as_deref(),
                    &terrain,
                    width,
                    height,
                    &mut occupied,
                );
                let pack_behavior = (!members.is_empty()).then(|| {
                    self.original_pack_behavior(
                        &actor_definition,
                        members
                            .iter()
                            .any(|member| member.role == OriginalGroupRole::Escort),
                        members.len() + 1,
                    )
                });
                if let Some(behavior) = pack_behavior {
                    let pack_id = format!("{}.pack", guardian.instance_id);
                    actor.pack = Some(MonsterPackIdentity {
                        id: pack_id.clone(),
                        leader_id: guardian.instance_id.clone(),
                        role: MonsterPackRoleDto::Leader,
                        behavior,
                    });
                    for (ordinal, member) in members.into_iter().enumerate() {
                        let mut companion = self.generated_actor(
                            format!("{}.companion.{}", guardian.instance_id, ordinal + 1),
                            &member.kind_id,
                            member.position,
                        );
                        companion.pack = Some(MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: guardian.instance_id.clone(),
                            role: MonsterPackRoleDto::Member,
                            behavior: pack_behavior.expect("non-empty pack must retain behavior"),
                        });
                        entities.push(companion);
                    }
                }
            }
            entities.push(actor);
        }
        let mut items =
            self.generate_carried_loot_for_actors(&entities, &definition.id, definition.depth)?;
        if !generated_regions.is_empty() {
            for (region_index, region) in generated_regions.iter().enumerate() {
                let placements = regional_loot_allocations[region_index];
                for ordinal in 0..placements {
                    let room_id = &region.room_ids[usize::from(ordinal) % region.room_ids.len()];
                    let position =
                        self.choose_generated_region_position(region, &terrain, width, &occupied);
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: region.state.loot_table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id: room_id.clone(),
                                spawn_id: format!(
                                    "{}.region.{}.loot.{}",
                                    definition.id,
                                    region.state.region_id,
                                    ordinal + 1
                                ),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
            }
        } else if let Some(table_id) = &definition.loot_table_id {
            if let Some(allocation) = definition.loot_allocation {
                let room_count = self.scaled_normal_allocation(
                    allocation.room_objects,
                    definition,
                    allocation.reference_area_tiles,
                );
                let anywhere_count = self.scaled_normal_allocation(
                    allocation.anywhere_objects,
                    definition,
                    allocation.reference_area_tiles,
                );
                for ordinal in 0..room_count {
                    let (room_id, position) = self.choose_generated_rooms_position(
                        content_rooms,
                        &terrain,
                        width,
                        &occupied,
                    );
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id,
                                spawn_id: format!(
                                    "{}.loot-table.room.{}",
                                    definition.id,
                                    ordinal + 1
                                ),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
                for ordinal in 0..anywhere_count {
                    let position =
                        self.choose_generated_floor_position(definition, &terrain, &occupied);
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id: "anywhere".to_owned(),
                                spawn_id: format!(
                                    "{}.loot-table.anywhere.{}",
                                    definition.id,
                                    ordinal + 1
                                ),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
            } else {
                let room_id = if legacy_vault.is_some() {
                    "entry"
                } else {
                    "remote"
                };
                let floor_loot_placements =
                    definition.generation_budget.as_ref().map_or(1, |budget| {
                        budget.loot_placements.saturating_sub(
                            vault_placements
                                .iter()
                                .map(|placement| {
                                    u16::try_from(placement.vault.loot_spawns.len())
                                        .expect("validated vault loot count must fit u16")
                                })
                                .sum::<u16>(),
                        )
                    });
                for ordinal in 0..floor_loot_placements {
                    let placement_room_id = if maze_only {
                        "maze"
                    } else if definition.layout.is_some() {
                        generated_non_entry_room_id(content_rooms, ordinal)
                    } else {
                        room_id
                    };
                    let position = if maze_only {
                        choose_generated_maze_position(&maze_walkable, first_center, &occupied)
                    } else {
                        self.choose_generated_room_position(
                            content_rooms,
                            placement_room_id,
                            &occupied,
                            Some((&terrain, width)),
                        )
                    };
                    occupied.insert(position);
                    items.extend(self.generate_loot_instances(
                        &LootContext {
                            table_id: table_id.clone(),
                            floor_id: definition.id.clone(),
                            depth: definition.depth,
                            source: LootSource::FloorRoom {
                                room_id: placement_room_id.to_owned(),
                                spawn_id: format!("{}.loot-table.{}", definition.id, ordinal + 1),
                            },
                        },
                        ItemLocation::Ground(position),
                    )?);
                }
            }
        } else {
            for spawn in &definition.loot_spawns {
                let position = self.choose_generated_room_position(
                    &rooms,
                    &spawn.room_id,
                    &occupied,
                    Some((&terrain, width)),
                );
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::FloorRoom {
                            room_id: spawn.room_id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        for placement in &vault_placements {
            for spawn in &placement.vault.loot_spawns {
                let local = transformed_vault_position(
                    &placement.vault,
                    placement.transform,
                    spawn.position,
                );
                let position = Position {
                    x: placement.origin.x + local.x,
                    y: placement.origin.y + local.y,
                };
                occupied.insert(position);
                items.extend(self.generate_loot_instances(
                    &LootContext {
                        table_id: spawn.loot_table_id.clone(),
                        floor_id: definition.id.clone(),
                        depth: definition.depth,
                        source: LootSource::Vault {
                            vault_id: placement.vault.id.clone(),
                            spawn_id: spawn.id.clone(),
                        },
                    },
                    ItemLocation::Ground(position),
                )?);
            }
        }
        let mut gold_piles = Vec::new();
        if let Some(allocation) = definition.gold_allocation {
            let pile_count = self.scaled_normal_allocation(
                allocation.piles,
                definition,
                allocation.reference_area_tiles,
            );
            for _ in 0..pile_count {
                let position =
                    self.choose_generated_floor_position(definition, &terrain, &occupied);
                occupied.insert(position);
                gold_piles.push(self.generate_gold_pile(position, definition.depth, false)?);
            }
        }
        for guaranteed in &definition.guaranteed_items {
            if self.rng.bounded(u64::from(guaranteed.chance_one_in)) != 0 {
                continue;
            }
            let entry = if guaranteed.entries.len() == 1 {
                &guaranteed.entries[0]
            } else {
                let weights = guaranteed
                    .entries
                    .iter()
                    .map(|entry| u32::from(entry.weight))
                    .collect::<Vec<_>>();
                &guaranteed.entries[self.roll_weighted_index(&weights)]
            };
            let position = self.choose_generated_floor_position(definition, &terrain, &occupied);
            occupied.insert(position);
            let (activation, charges) = initial_item_runtime_state(
                &self.content,
                &mut self.rng,
                &entry.item_kind_id,
                &[],
                definition.depth,
            );
            items.push(ItemInstance {
                id: self.allocate_item_instance_id()?,
                kind_id: entry.item_kind_id.clone(),
                quantity: 1,
                inscription: None,
                origin_actor_kind_id: None,
                origin_kind: None,
                damage_dice_override: None,
                discount_percent: 0,
                quality: ItemQualityDto::Ordinary,
                affix_ids: Vec::new(),
                rolled_affixes: Vec::new(),
                enchantments: ItemEnchantmentsDto::default(),
                curse: initial_item_curse(&self.content, &entry.item_kind_id),
                permanent_destruction_immunities: Default::default(),
                activation,
                charges,
                fuel: initial_item_fuel(&self.content, &entry.item_kind_id),
                device_recovery_progress: 0,
                captured_actor: None,
                location: ItemLocation::Ground(position),
            });
        }
        for (task_id, _objective_index, objective, target_placement, dungeon_depth_task) in
            &task_objectives
        {
            match objective.kind {
                TaskObjectiveKind::CollectItem => {
                    let kind_id = objective
                        .item_kind_id
                        .clone()
                        .expect("validated item objective must have a kind ID");
                    let (activation, charges) = initial_item_runtime_state(
                        &self.content,
                        &mut self.rng,
                        &kind_id,
                        &[],
                        definition.depth,
                    );
                    let fuel = initial_item_fuel(&self.content, &kind_id);
                    items.push(ItemInstance {
                        id: objective
                            .item_instance_id
                            .clone()
                            .expect("validated item objective must have an instance ID"),
                        curse: initial_item_curse(&self.content, &kind_id),
                        permanent_destruction_immunities: Default::default(),
                        kind_id,
                        quantity: 1,
                        inscription: None,
                        origin_actor_kind_id: None,
                        origin_kind: None,
                        damage_dice_override: None,
                        discount_percent: 0,
                        quality: ItemQualityDto::Ordinary,
                        affix_ids: Vec::new(),
                        rolled_affixes: Vec::new(),
                        enchantments: ItemEnchantmentsDto::default(),
                        activation,
                        charges,
                        fuel,
                        device_recovery_progress: 0,
                        captured_actor: None,
                        location: ItemLocation::Ground(first_center),
                    });
                }
                TaskObjectiveKind::KillActor => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available")
                        .clone();
                    let target_position = Position {
                        x: first_center.x + 1,
                        y: first_center.y,
                    };
                    occupied.insert(target_position);
                    entities.push(spawn_actor_from_definition(
                        &mut self.rng,
                        &actor,
                        objective
                            .actor_instance_id
                            .as_ref()
                            .expect("validated kill objective must have an instance ID"),
                        target_position,
                        INITIAL_MONSTER_ENERGY_NEED,
                        actor_starts_alerted(&actor),
                    ));
                }
                TaskObjectiveKind::KillActorKind => {
                    let kind_id = objective
                        .actor_kind_id
                        .as_ref()
                        .expect("validated counted kill objective must have a kind ID");
                    let actor = self
                        .content
                        .actor(kind_id)
                        .expect("validated objective actor must remain available")
                        .clone();
                    let remaining = self
                        .task_states
                        .get(task_id)
                        .map_or(objective.required, |state| {
                            state.required.saturating_sub(state.current)
                        });
                    let spawn_count = target_placement
                        .as_ref()
                        .map_or(objective.required, |placement| placement.spawn_count)
                        .min(remaining);
                    for ordinal in 0..spawn_count {
                        let target_position = if *dungeon_depth_task {
                            self.choose_generated_dungeon_task_target_position(
                                definition,
                                &terrain,
                                &occupied,
                                kind_id,
                                first_center,
                            )
                            .ok_or_else(|| {
                                CoreError::Invariant(format!(
                                    "task {task_id} cannot place target {kind_id} on floor {}",
                                    definition.id
                                ))
                            })?
                        } else {
                            Position {
                                x: first_center.x + 1 + i32::try_from(ordinal).unwrap_or(i32::MAX),
                                y: first_center.y,
                            }
                        };
                        occupied.insert(target_position);
                        entities.push(spawn_actor_from_definition(
                            &mut self.rng,
                            &actor,
                            &format!("{}.task-target.{}", definition.id, ordinal + 1),
                            target_position,
                            INITIAL_MONSTER_ENERGY_NEED,
                            actor_starts_alerted(&actor),
                        ));
                    }
                }
                TaskObjectiveKind::ClearFloor | TaskObjectiveKind::EnterFloor => {}
            }
        }
        if !completion_exit_terrain_ids.is_empty() {
            for terrain_id in &mut terrain {
                if completion_exit_terrain_ids.contains(terrain_id) {
                    terrain_id.clone_from(&definition.floor_terrain_id);
                }
            }
        }
        for region in &mut generated_regions {
            region.state.cells.sort();
            region.state.cells.dedup();
        }
        generated_regions.sort_by(|left, right| left.state.region_id.cmp(&right.state.region_id));
        self.resolve_floor_connection_targets(definition, &mut floor_connections)?;
        let mut glow = vec![false; usize::from(width) * usize::from(height)];
        for position in rooms.iter().flat_map(generated_room_cells) {
            let index = usize::try_from(position.y).expect("generated room y must fit usize")
                * usize::from(width)
                + usize::try_from(position.x).expect("generated room x must fit usize");
            glow[index] = true;
        }
        Ok(FloorState {
            id: definition.id.clone(),
            dungeon_instance_id,
            reproduction_suppressed: false,
            width,
            height,
            terrain,
            glow,
            player_position: first_center,
            entities,
            items,
            gold_piles,
            explored: vec![false; usize::from(width) * usize::from(height)],
            revealed_terrain: BTreeSet::new(),
            connections: floor_connections,
            regions: generated_regions
                .into_iter()
                .map(|region| region.state)
                .collect(),
        })
    }
}

impl Game {
    fn place_configured_stairs(
        &mut self,
        definition: &ProceduralFloorDefinition,
        primary_up: Position,
        primary_down: Option<Position>,
        reserved: &BTreeSet<Position>,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        let Some(stairs) = definition.layout.as_ref().and_then(|layout| layout.stairs) else {
            return BTreeSet::new();
        };
        let up_total = stairs.up.minimum
            + u16::try_from(
                self.rng
                    .bounded(u64::from(stairs.up.maximum - stairs.up.minimum + 1)),
            )
            .expect("stair count roll must fit u16");
        let down_total = stairs.down.map(|range| {
            range.minimum
                + u16::try_from(
                    self.rng
                        .bounded(u64::from(range.maximum - range.minimum + 1)),
                )
                .expect("stair count roll must fit u16")
        });
        let mut occupied = reserved.clone();
        occupied.insert(primary_up);
        occupied.extend(primary_down);
        let mut placed = BTreeSet::new();
        self.place_additional_stair_terrain(
            definition,
            &definition.up_stair_terrain_id,
            up_total - 1,
            &mut occupied,
            &mut placed,
            terrain,
        );
        if let (Some(down_terrain_id), Some(total)) =
            (&definition.down_stair_terrain_id, down_total)
        {
            self.place_additional_stair_terrain(
                definition,
                down_terrain_id,
                total - 1,
                &mut occupied,
                &mut placed,
                terrain,
            );
        }
        placed
    }

    fn place_additional_stair_terrain(
        &mut self,
        definition: &ProceduralFloorDefinition,
        stair_terrain_id: &str,
        count: u16,
        occupied: &mut BTreeSet<Position>,
        placed: &mut BTreeSet<Position>,
        terrain: &mut [String],
    ) {
        for _ in 0..count {
            let mut candidates = terrain
                .iter()
                .enumerate()
                .filter_map(|(index, terrain_id)| {
                    let position = Position {
                        x: i32::try_from(index % usize::from(definition.width))
                            .expect("floor x must fit i32"),
                        y: i32::try_from(index / usize::from(definition.width))
                            .expect("floor y must fit i32"),
                    };
                    (!occupied.contains(&position)
                        && self
                            .content
                            .terrain(terrain_id)
                            .is_some_and(|terrain| terrain.walkable))
                    .then_some(position)
                })
                .collect::<Vec<_>>();
            let adjacent_wall_count = |position: Position| {
                [
                    Position {
                        x: position.x - 1,
                        y: position.y,
                    },
                    Position {
                        x: position.x + 1,
                        y: position.y,
                    },
                    Position {
                        x: position.x,
                        y: position.y - 1,
                    },
                    Position {
                        x: position.x,
                        y: position.y + 1,
                    },
                ]
                .into_iter()
                .filter(|neighbor| {
                    neighbor.x < 0
                        || neighbor.y < 0
                        || neighbor.x >= i32::from(definition.width)
                        || neighbor.y >= i32::from(definition.height)
                        || !self
                            .content
                            .terrain(&terrain[generated_terrain_index(definition.width, *neighbor)])
                            .is_some_and(|terrain| terrain.walkable)
                })
                .count()
            };
            let maximum_adjacent_walls = candidates
                .iter()
                .map(|position| adjacent_wall_count(*position))
                .max()
                .expect("validated floor must retain space for configured stairs");
            candidates.retain(|position| adjacent_wall_count(*position) == maximum_adjacent_walls);
            candidates.sort_by_key(|position| (position.y, position.x));
            let selected = candidates[usize::try_from(
                self.rng
                    .bounded(u64::try_from(candidates.len()).expect("candidates must fit u64")),
            )
            .expect("candidate index must fit usize")];
            set_generated_terrain(terrain, definition.width, selected, stair_terrain_id);
            occupied.insert(selected);
            placed.insert(selected);
        }
    }

    fn carve_cave_room_network(
        &mut self,
        terrain: &mut [String],
        width: u16,
        rooms: &[GeneratedRoom],
        floor_terrain_id: &str,
    ) {
        let mut centers = rooms.iter().map(GeneratedRoom::center).collect::<Vec<_>>();
        for remaining in (2..=centers.len()).rev() {
            let swap_index = usize::try_from(
                self.rng
                    .bounded(u64::try_from(remaining).expect("room count must fit u64")),
            )
            .expect("room index must fit usize");
            centers.swap(remaining - 1, swap_index);
        }
        for index in 0..centers.len() {
            let from = centers[index];
            let to = centers[(index + 1) % centers.len()];
            self.carve_randomized_corridor(terrain, width, from, to, floor_terrain_id);
        }
    }

    fn carve_randomized_corridor(
        &mut self,
        terrain: &mut [String],
        width: u16,
        from: Position,
        to: Position,
        floor_terrain_id: &str,
    ) {
        let mut position = from;
        set_generated_terrain(terrain, width, position, floor_terrain_id);
        while position != to {
            let change_x = position.x != to.x;
            let change_y = position.y != to.y;
            if change_x && (!change_y || self.rng.bounded(2) == 0) {
                position.x += (to.x - position.x).signum();
            } else {
                position.y += (to.y - position.y).signum();
            }
            set_generated_terrain(terrain, width, position, floor_terrain_id);
        }
    }

    fn resolve_floor_connection_targets(
        &mut self,
        definition: &ProceduralFloorDefinition,
        connections: &mut [FloorConnectionState],
    ) -> Result<(), CoreError> {
        let mut selected_dynamic_targets = BTreeSet::new();
        for state in connections {
            let connection = definition
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
                .ok_or(CoreError::InvalidSave(
                    "generated floor connection is missing from content",
                ))?;
            if connection.target_candidates.is_empty() {
                state.target_floor_id = Some(connection.target_floor_id.clone());
                state.target_connection_id = connection.target_connection_id.clone();
                continue;
            }
            let mut eligible = connection
                .target_candidates
                .iter()
                .filter(|candidate| !selected_dynamic_targets.contains(&candidate.target_floor_id))
                .collect::<Vec<_>>();
            if eligible.is_empty() {
                eligible.extend(connection.target_candidates.iter());
            }
            let weights = eligible
                .iter()
                .map(|candidate| u32::from(candidate.weight))
                .collect::<Vec<_>>();
            let selected = eligible[self.roll_weighted_index(&weights)];
            state.target_floor_id = Some(selected.target_floor_id.clone());
            state.target_connection_id = Some(selected.target_connection_id.clone());
            selected_dynamic_targets.insert(selected.target_floor_id.clone());
        }
        Ok(())
    }

    pub(in crate::game) fn generate_budgeted_rooms(
        &mut self,
        definition: &ProceduralFloorDefinition,
        geometry: &ProceduralRoomGeometryDefinition,
    ) -> Vec<GeneratedRoom> {
        match geometry.placement {
            ProceduralRoomPlacement::Partitioned => {
                self.generate_partitioned_rooms(definition, geometry)
            }
            ProceduralRoomPlacement::Free => self
                .generate_free_rooms(definition, geometry)
                .unwrap_or_else(|| self.generate_partitioned_rooms(definition, geometry)),
        }
    }

    fn generate_partitioned_rooms(
        &mut self,
        definition: &ProceduralFloorDefinition,
        geometry: &ProceduralRoomGeometryDefinition,
    ) -> Vec<GeneratedRoom> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("room geometry requires a generation budget");
        let placement_count = budget
            .room_placements
            .expect("validated room placement count must remain available");
        let mut remaining_area = budget
            .room_area_tiles
            .expect("validated room area budget must remain available");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let interior_width = definition.width - 2;
        let interior_height = definition.height - 2;
        let minimum_room_area = geometry
            .shapes
            .iter()
            .map(|candidate| match candidate.shape {
                ProceduralRoomShape::Rectangle => {
                    u32::from(geometry.min_width) * u32::from(geometry.min_height)
                }
                ProceduralRoomShape::Cross => {
                    u32::from(geometry.min_width) + u32::from(geometry.min_height) - 1
                }
                ProceduralRoomShape::Cavern => generated_cavern_room_area(
                    i32::from(geometry.min_width),
                    i32::from(geometry.min_height),
                ),
            })
            .min()
            .expect("validated room geometry must retain a shape");
        let mut rooms = Vec::with_capacity(usize::from(placement_count));

        for ordinal in 0..placement_count {
            let column = ordinal % columns;
            let row = ordinal / columns;
            let cell_left = 1 + interior_width * column / columns;
            let cell_right = 1 + interior_width * (column + 1) / columns;
            let cell_top = 1 + interior_height * row / rows;
            let cell_bottom = 1 + interior_height * (row + 1) / rows;
            let future_room_count = placement_count - ordinal - 1;
            let maximum_room_area =
                remaining_area - u32::from(future_room_count) * minimum_room_area;
            let mut shape_candidates = Vec::new();

            for shape_candidate in &geometry.shapes {
                let mut candidates = Vec::new();
                for y in cell_top..cell_bottom {
                    for x in cell_left..cell_right {
                        for height in geometry.min_height..=geometry.max_height {
                            for width in geometry.min_width..=geometry.max_width {
                                if x + width > cell_right || y + height > cell_bottom {
                                    continue;
                                }
                                let room = GeneratedRoom {
                                    id: String::new(),
                                    x: i32::from(x),
                                    y: i32::from(y),
                                    width: i32::from(width),
                                    height: i32::from(height),
                                    shape: shape_candidate.shape,
                                    carved_cells: BTreeSet::new(),
                                };
                                if room.area() <= maximum_room_area {
                                    candidates.push(room);
                                }
                            }
                        }
                    }
                }
                if !candidates.is_empty() {
                    shape_candidates.push((shape_candidate.weight, candidates));
                }
            }
            let shape_index = if shape_candidates.len() == 1 {
                0
            } else {
                let weights = shape_candidates
                    .iter()
                    .map(|(weight, _)| *weight)
                    .collect::<Vec<_>>();
                self.roll_weighted_index(&weights)
            };
            let candidates = &shape_candidates[shape_index].1;
            let candidate_index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(candidates.len())
                            .expect("room geometry candidate count must fit u64"),
                    ),
                )
                .expect("room geometry candidate index must fit usize")
            };
            let mut room = candidates[candidate_index].clone();
            room.id = match ordinal {
                0 => "entry".to_owned(),
                1 => "remote".to_owned(),
                _ => format!("room.{}", ordinal + 1),
            };
            if room.shape == ProceduralRoomShape::Cavern {
                room.carved_cells = self.generate_cavern_room_cells(&room);
            }
            remaining_area -= room.area();
            rooms.push(room);
        }

        rooms
    }

    fn generate_free_rooms(
        &mut self,
        definition: &ProceduralFloorDefinition,
        geometry: &ProceduralRoomGeometryDefinition,
    ) -> Option<Vec<GeneratedRoom>> {
        const LAYOUT_ATTEMPTS: usize = 64;

        let budget = definition
            .generation_budget
            .as_ref()
            .expect("room geometry requires a generation budget");
        let placement_count = budget
            .room_placements
            .expect("validated room placement count must remain available");
        let area_budget = budget
            .room_area_tiles
            .expect("validated room area budget must remain available");
        let minimum_room_area = geometry
            .shapes
            .iter()
            .map(|candidate| match candidate.shape {
                ProceduralRoomShape::Rectangle => {
                    u32::from(geometry.min_width) * u32::from(geometry.min_height)
                }
                ProceduralRoomShape::Cross => {
                    u32::from(geometry.min_width) + u32::from(geometry.min_height) - 1
                }
                ProceduralRoomShape::Cavern => generated_cavern_room_area(
                    i32::from(geometry.min_width),
                    i32::from(geometry.min_height),
                ),
            })
            .min()
            .expect("validated room geometry must retain a shape");

        for _ in 0..LAYOUT_ATTEMPTS {
            let mut remaining_area = area_budget;
            let mut rooms = Vec::with_capacity(usize::from(placement_count));
            let mut layout_complete = true;

            for ordinal in 0..placement_count {
                let future_room_count = placement_count - ordinal - 1;
                let maximum_room_area =
                    remaining_area - u32::from(future_room_count) * minimum_room_area;
                let mut shape_candidates = Vec::new();

                for shape_candidate in &geometry.shapes {
                    let mut candidates = Vec::new();
                    for height in geometry.min_height..=geometry.max_height {
                        for width in geometry.min_width..=geometry.max_width {
                            for y in 1..definition.height - height {
                                for x in 1..definition.width - width {
                                    let room = GeneratedRoom {
                                        id: String::new(),
                                        x: i32::from(x),
                                        y: i32::from(y),
                                        width: i32::from(width),
                                        height: i32::from(height),
                                        shape: shape_candidate.shape,
                                        carved_cells: BTreeSet::new(),
                                    };
                                    let separated = rooms.iter().all(|placed: &GeneratedRoom| {
                                        room.x + room.width < placed.x
                                            || placed.x + placed.width < room.x
                                            || room.y + room.height < placed.y
                                            || placed.y + placed.height < room.y
                                    });
                                    if separated && room.area() <= maximum_room_area {
                                        candidates.push(room);
                                    }
                                }
                            }
                        }
                    }
                    if !candidates.is_empty() {
                        shape_candidates.push((shape_candidate.weight, candidates));
                    }
                }

                if shape_candidates.is_empty() {
                    layout_complete = false;
                    break;
                }
                let shape_index = if shape_candidates.len() == 1 {
                    0
                } else {
                    let weights = shape_candidates
                        .iter()
                        .map(|(weight, _)| *weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let candidates = &shape_candidates[shape_index].1;
                let candidate_index = if candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(candidates.len())
                                .expect("room geometry candidate count must fit u64"),
                        ),
                    )
                    .expect("room geometry candidate index must fit usize")
                };
                let room = candidates[candidate_index].clone();
                remaining_area -= room.area();
                rooms.push(room);
            }

            if layout_complete {
                for (index, room) in rooms.iter_mut().enumerate() {
                    room.id = match index {
                        0 => "entry".to_owned(),
                        1 => "remote".to_owned(),
                        _ => format!("room.{}", index + 1),
                    };
                    if room.shape == ProceduralRoomShape::Cavern {
                        room.carved_cells = self.generate_cavern_room_cells(room);
                    }
                }
                return Some(rooms);
            }
        }

        None
    }

    fn generate_cavern_room_cells(&mut self, room: &GeneratedRoom) -> BTreeSet<Position> {
        let target_area = usize::try_from(generated_cavern_room_area(room.width, room.height))
            .expect("room area must fit usize");
        let mut carved = BTreeSet::from([room.center()]);
        while carved.len() < target_area {
            let mut frontier = carved
                .iter()
                .flat_map(|position| {
                    [
                        Position {
                            x: position.x - 1,
                            y: position.y,
                        },
                        Position {
                            x: position.x + 1,
                            y: position.y,
                        },
                        Position {
                            x: position.x,
                            y: position.y - 1,
                        },
                        Position {
                            x: position.x,
                            y: position.y + 1,
                        },
                    ]
                })
                .filter(|position| {
                    position.x >= room.x
                        && position.x < room.x + room.width
                        && position.y >= room.y
                        && position.y < room.y + room.height
                        && !carved.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = usize::try_from(
                self.rng
                    .bounded(u64::try_from(frontier.len()).expect("frontier must fit u64")),
            )
            .expect("frontier index must fit usize");
            carved.insert(frontier[index]);
        }
        carved
    }

    fn place_classic_pit(
        &mut self,
        floor: &ProceduralFloorDefinition,
        pit: &ProceduralPitDefinition,
        approach: Position,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> GeneratedPitPlacement {
        let placement_count = floor
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.room_placements)
            .expect("validated pit requires room placement budget");
        let columns = if placement_count <= 4 { 2 } else { 3 };
        let rows = placement_count.div_ceil(columns);
        let ordinal = placement_count - 1;
        let column = ordinal % columns;
        let row = ordinal / columns;
        let interior_width = floor.width - 2;
        let interior_height = floor.height - 2;
        let cell_left = 1 + interior_width * column / columns;
        let cell_right = 1 + interior_width * (column + 1) / columns;
        let cell_top = 1 + interior_height * row / rows;
        let cell_bottom = 1 + interior_height * (row + 1) / rows;
        let total_width = pit.inner_width + 6;
        let total_height = pit.inner_height + 6;
        let maximum_x = i32::from(floor.width - total_width - 1);
        let maximum_y = i32::from(floor.height - total_height - 1);
        let origin = Position {
            x: ((i32::from(cell_left + cell_right) - i32::from(total_width)) / 2)
                .clamp(1, maximum_x),
            y: ((i32::from(cell_top + cell_bottom) - i32::from(total_height)) / 2)
                .clamp(1, maximum_y),
        };
        let center_y = origin.y + i32::from(total_height / 2);
        let outer_entrance = Position {
            x: origin.x,
            y: center_y,
        };
        let inner_entrance = Position {
            x: origin.x + 2,
            y: center_y,
        };

        for local_y in 0..total_height {
            for local_x in 0..total_width {
                let on_outer_wall = local_x == 0
                    || local_y == 0
                    || local_x + 1 == total_width
                    || local_y + 1 == total_height;
                let on_inner_wall = local_x == 2
                    || local_y == 2
                    || local_x + 3 == total_width
                    || local_y + 3 == total_height;
                let terrain_id = if on_outer_wall || on_inner_wall {
                    &floor.wall_terrain_id
                } else {
                    floor_terrain_id
                };
                set_generated_terrain(
                    terrain,
                    floor.width,
                    Position {
                        x: origin.x + i32::from(local_x),
                        y: origin.y + i32::from(local_y),
                    },
                    terrain_id,
                );
            }
        }
        set_generated_terrain(terrain, floor.width, outer_entrance, floor_terrain_id);
        carve_generated_corridor(
            terrain,
            floor.width,
            approach,
            outer_entrance,
            floor_terrain_id,
        );
        set_generated_terrain(
            terrain,
            floor.width,
            inner_entrance,
            &floor.closed_door_terrain_id,
        );
        GeneratedPitPlacement {
            definition: pit.clone(),
            origin,
            outer_entrance,
            inner_entrance,
        }
    }

    pub(in crate::game) fn generate_connected_cavern(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.cavern_area_tiles)
            .expect("validated cavern area budget must remain available");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut carved = BTreeSet::from([origin]);
        set_generated_terrain(terrain, definition.width, origin, terrain_id);

        while carved.len() < usize::try_from(area).expect("cavern area must fit usize") {
            let mut frontier = carved
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !carved.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("cavern frontier count must fit u64"),
                ))
                .expect("cavern frontier index must fit usize")
            };
            let position = frontier[index];
            carved.insert(position);
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }

        origin
    }

    pub(in crate::game) fn generate_connected_lake(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        terrain: &mut [String],
    ) -> Position {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("lake requires a generation budget");
        let area = usize::try_from(
            budget
                .lake_area_tiles
                .expect("validated lake area budget must remain available"),
        )
        .expect("lake area must fit usize");
        let deep_area = usize::try_from(
            budget
                .lake_deep_area_tiles
                .expect("validated deep lake area budget must remain available"),
        )
        .expect("deep lake area must fit usize");
        let origin = Position {
            x: i32::from(definition.width / 2),
            y: i32::from(definition.height / 2),
        };
        let mut selected = BTreeSet::from([origin]);
        let mut insertion_order = vec![origin];

        while insertion_order.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("lake frontier count must fit u64"),
                ))
                .expect("lake frontier index must fit usize")
            };
            let position = frontier[index];
            selected.insert(position);
            insertion_order.push(position);
        }

        for (ordinal, position) in insertion_order.into_iter().enumerate() {
            let terrain_id = if ordinal < deep_area {
                deep_terrain_id
            } else {
                shallow_terrain_id
            };
            set_generated_terrain(terrain, definition.width, position, terrain_id);
        }
        origin
    }

    pub(in crate::game) fn generate_river(
        &mut self,
        definition: &ProceduralFloorDefinition,
        deep_terrain_id: &str,
        shallow_terrain_id: &str,
        target: Position,
        terrain: &mut [String],
    ) {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let area = usize::try_from(
            definition
                .generation_budget
                .as_ref()
                .and_then(|budget| budget.river_area_tiles)
                .expect("validated river area budget must remain available"),
        )
        .expect("river area must fit usize");
        let side = self.rng.bounded(4);
        let start = match side {
            0 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: 1,
            },
            1 => Position {
                x: i32::from(definition.width - 2),
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
            2 => Position {
                x: 1 + i32::try_from(self.rng.bounded(u64::from(definition.width - 2)))
                    .expect("river start x must fit i32"),
                y: i32::from(definition.height - 2),
            },
            _ => Position {
                x: 1,
                y: 1 + i32::try_from(self.rng.bounded(u64::from(definition.height - 2)))
                    .expect("river start y must fit i32"),
            },
        };
        let mut current = start;
        let mut centerline = vec![current];
        while current != target {
            let move_x = current.x != target.x;
            let move_y = current.y != target.y;
            let advance_x = move_x && (!move_y || self.rng.bounded(2) == 0);
            if advance_x {
                current.x += (target.x - current.x).signum();
            } else {
                current.y += (target.y - current.y).signum();
            }
            centerline.push(current);
        }
        debug_assert!(centerline.len() <= area);
        let mut painted = centerline.iter().copied().collect::<BTreeSet<_>>();
        for position in &centerline {
            set_generated_terrain(terrain, definition.width, *position, deep_terrain_id);
        }

        while painted.len() < area {
            let mut frontier = painted
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !painted.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("river frontier count must fit u64"),
                ))
                .expect("river frontier index must fit usize")
            };
            let position = frontier[index];
            painted.insert(position);
            set_generated_terrain(terrain, definition.width, position, shallow_terrain_id);
        }
    }

    pub(in crate::game) fn generated_actor(
        &mut self,
        id: String,
        kind_id: &str,
        position: Position,
    ) -> Actor {
        let actor = self
            .content
            .actor(kind_id)
            .expect("validated generated actor must remain available")
            .clone();
        spawn_actor_from_definition(
            &mut self.rng,
            &actor,
            &id,
            position,
            INITIAL_MONSTER_ENERGY_NEED,
            actor_starts_alerted(&actor),
        )
    }

    #[cfg(test)]
    pub(in crate::game) fn push_generated_actor(
        &mut self,
        id: String,
        kind_id: &str,
        position: Position,
    ) {
        let actor = self.generated_actor(id, kind_id, position);
        self.entities.push(actor);
    }

    fn generated_pack_actor(
        &mut self,
        id: String,
        kind_id: &str,
        position: Position,
        pack: MonsterPackIdentity,
    ) -> Actor {
        let mut actor = self.generated_actor(id, kind_id, position);
        actor.pack = Some(pack);
        actor
    }

    fn generate_classic_pit_actors(
        &mut self,
        definition: &ProceduralFloorDefinition,
        pit: &GeneratedPitPlacement,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let table = self
            .content
            .encounter_table(&pit.definition.encounter_table_id)
            .expect("validated pit encounter table must remain available")
            .clone();
        let eligible = table
            .entries
            .iter()
            .filter(|entry| {
                entry.min_depth <= definition.depth
                    && definition.depth <= entry.max_depth
                    && self
                        .content
                        .actor(&entry.actor_kind_id)
                        .is_some_and(|actor| actor.level <= u32::from(definition.depth))
            })
            .cloned()
            .collect::<Vec<_>>();
        let pit_weights = eligible
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        let mut roster = (0..pit.definition.roster_size)
            .map(|_| {
                eligible[self.roll_weighted_index(&pit_weights)]
                    .actor_kind_id
                    .clone()
            })
            .collect::<Vec<_>>();
        roster.sort_by(|left, right| {
            let left_level = self
                .content
                .actor(left)
                .expect("pit roster actor must remain available")
                .level;
            let right_level = self
                .content
                .actor(right)
                .expect("pit roster actor must remain available")
                .level;
            right_level.cmp(&left_level).then_with(|| left.cmp(right))
        });

        let half_width = pit.definition.inner_width / 2;
        let half_height = pit.definition.inner_height / 2;
        let maximum_rank = pit.definition.roster_size - 1;
        let mut ordinal = 0_u16;
        let mut actors = Vec::new();
        for local_y in 0..pit.definition.inner_height {
            for local_x in 0..pit.definition.inner_width {
                let dx = local_x.abs_diff(half_width);
                let dy = local_y.abs_diff(half_height);
                let horizontal_rank = dx * maximum_rank / half_width;
                let vertical_rank = dy * maximum_rank / half_height;
                let rank = usize::from(horizontal_rank.max(vertical_rank));
                let kind_id = &roster[rank];
                let position = Position {
                    x: pit.origin.x + 3 + i32::from(local_x),
                    y: pit.origin.y + 3 + i32::from(local_y),
                };
                occupied.insert(position);
                ordinal += 1;
                actors.push(self.generated_actor(
                    format!("{}.pit.{}", definition.id, ordinal),
                    kind_id,
                    position,
                ));
            }
        }
        actors
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::game) fn generate_dynamic_encounter_groups(
        &mut self,
        definition: &ProceduralFloorDefinition,
        table: &EncounterTableDefinition,
        eligible_entries: &[EncounterEntryDefinition],
        rooms: &[GeneratedRoom],
        room_id: &str,
        reserved_actor_slots: u16,
        ordinary_actor_reserve: u16,
        fill_plain: bool,
        id_prefix: &str,
        occupied: &mut BTreeSet<Position>,
    ) -> Vec<Actor> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("dynamic encounters require a generation budget");
        let group_placement_limit = budget
            .group_placements
            .expect("validated group placement budget must remain available");
        let mut remaining_group_actor_slots = budget
            .group_actor_slots
            .expect("validated group actor budget must remain available");
        let mut remaining_actor_slots = budget.actor_slots.saturating_sub(reserved_actor_slots);
        let grouped_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let plain_entries = eligible_entries
            .iter()
            .filter(|entry| entry.group.is_none())
            .cloned()
            .collect::<Vec<_>>();
        let minimum_group_companions = grouped_entries
            .iter()
            .filter_map(|entry| entry.group.as_ref())
            .map(rfb_content::EncounterGroupDefinition::min_companion_count)
            .min()
            .expect("validated dynamic floor must have a grouped encounter");
        let mut generated = Vec::new();
        let mut leader_ordinal = 0_u16;

        for group_slot in 0..group_placement_limit {
            let future_group_count = group_placement_limit - group_slot - 1;
            let future_companion_reserve =
                future_group_count.saturating_mul(minimum_group_companions);
            let future_actor_reserve = future_group_count
                .saturating_mul(minimum_group_companions.saturating_add(1))
                .saturating_add(ordinary_actor_reserve);
            let available_companion_slots = remaining_group_actor_slots
                .saturating_sub(future_companion_reserve)
                .min(
                    remaining_actor_slots
                        .saturating_sub(future_actor_reserve)
                        .saturating_sub(1),
                );
            let mut candidates = grouped_entries
                .iter()
                .filter(|entry| {
                    entry.group.as_ref().is_some_and(|group| {
                        group.min_companion_count() <= available_companion_slots
                    })
                })
                .cloned()
                .collect::<Vec<_>>();
            let mut placed_group = None;
            while !candidates.is_empty() {
                let weights = candidates
                    .iter()
                    .map(|entry| entry.weight)
                    .collect::<Vec<_>>();
                let selected_index = if candidates.len() == 1 {
                    0
                } else {
                    self.roll_weighted_index(&weights)
                };
                let entry = candidates.remove(selected_index);
                let group = entry
                    .group
                    .as_ref()
                    .expect("grouped encounter candidate must retain its group");
                let friend_min = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.min_count);
                let friend_max = group
                    .friends
                    .as_ref()
                    .map_or(0, |friends| friends.max_count);
                let escort_min = group.escort.as_ref().map_or(0, |escort| escort.min_count);
                let escort_max = group.escort.as_ref().map_or(0, |escort| escort.max_count);
                let friend_upper =
                    friend_max.min(available_companion_slots.saturating_sub(escort_min));
                let mut friend_count = self.roll_inclusive(friend_min, friend_upper);
                let escort_upper =
                    escort_max.min(available_companion_slots.saturating_sub(friend_count));
                let mut escort_count = self.roll_inclusive(escort_min, escort_upper);
                let formation_placement = loop {
                    let placement_candidates = formation_placement_candidates(
                        rooms,
                        room_id,
                        occupied,
                        group.formation,
                        friend_count.saturating_add(escort_count),
                    );
                    if !placement_candidates.is_empty() {
                        let placement_index = if placement_candidates.len() == 1 {
                            0
                        } else {
                            usize::try_from(
                                self.rng
                                    .bounded(u64::try_from(placement_candidates.len()).expect(
                                        "formation placement candidate count must fit u64",
                                    )),
                            )
                            .expect("formation placement candidate index must fit usize")
                        };
                        break Some(placement_candidates[placement_index].clone());
                    }
                    if escort_count > escort_min {
                        escort_count -= 1;
                    } else if friend_count > friend_min {
                        friend_count -= 1;
                    } else {
                        break None;
                    }
                };
                let Some((leader_position, companion_positions)) = formation_placement else {
                    continue;
                };
                placed_group = Some((
                    entry,
                    friend_count,
                    escort_count,
                    leader_position,
                    companion_positions,
                ));
                break;
            }
            let Some((entry, friend_count, escort_count, leader_position, companion_positions)) =
                placed_group
            else {
                break;
            };

            leader_ordinal += 1;
            occupied.insert(leader_position);
            let leader_id = format!("{id_prefix}.encounter.{leader_ordinal}");
            let pack_id = format!("{id_prefix}.pack.{leader_ordinal}");
            let pack_ai = entry
                .group
                .as_ref()
                .expect("grouped encounter must retain pack AI")
                .pack_ai;
            generated.push(self.generated_pack_actor(
                leader_id.clone(),
                &entry.actor_kind_id,
                leader_position,
                MonsterPackIdentity {
                    id: pack_id.clone(),
                    leader_id: leader_id.clone(),
                    role: MonsterPackRoleDto::Leader,
                    behavior: monster_pack_behavior_dto(pack_ai.leader),
                },
            ));
            for (index, position) in companion_positions
                .iter()
                .take(usize::from(friend_count))
                .copied()
                .enumerate()
            {
                occupied.insert(position);
                generated.push(self.generated_pack_actor(
                    format!(
                        "{id_prefix}.encounter.{leader_ordinal}.friend.{}",
                        index + 1
                    ),
                    &entry.actor_kind_id,
                    position,
                    MonsterPackIdentity {
                        id: pack_id.clone(),
                        leader_id: leader_id.clone(),
                        role: MonsterPackRoleDto::Member,
                        behavior: monster_pack_behavior_dto(pack_ai.friends),
                    },
                ));
            }
            if escort_count > 0 {
                let escort = entry
                    .group
                    .as_ref()
                    .and_then(|group| group.escort.as_ref())
                    .expect("positive escort count must retain an escort table");
                let eligible_escorts = escort
                    .entries
                    .iter()
                    .filter(|escort_entry| {
                        escort_entry.min_depth <= definition.depth
                            && definition.depth <= escort_entry.max_depth
                            && self
                                .content
                                .actor(&escort_entry.actor_kind_id)
                                .is_some_and(|actor| actor.level <= u32::from(definition.depth))
                    })
                    .collect::<Vec<_>>();
                let escort_weights = eligible_escorts
                    .iter()
                    .map(|escort_entry| escort_entry.weight)
                    .collect::<Vec<_>>();
                for (index, position) in companion_positions
                    .iter()
                    .skip(usize::from(friend_count))
                    .take(usize::from(escort_count))
                    .copied()
                    .enumerate()
                {
                    let escort_index = if eligible_escorts.len() == 1 {
                        0
                    } else {
                        self.roll_weighted_index(&escort_weights)
                    };
                    let kind_id = &eligible_escorts[escort_index].actor_kind_id;
                    occupied.insert(position);
                    generated.push(self.generated_pack_actor(
                        format!(
                            "{id_prefix}.encounter.{leader_ordinal}.escort.{}",
                            index + 1
                        ),
                        kind_id,
                        position,
                        MonsterPackIdentity {
                            id: pack_id.clone(),
                            leader_id: leader_id.clone(),
                            role: MonsterPackRoleDto::Member,
                            behavior: monster_pack_behavior_dto(pack_ai.escorts),
                        },
                    ));
                }
            }
            let companion_count = friend_count.saturating_add(escort_count);
            remaining_group_actor_slots =
                remaining_group_actor_slots.saturating_sub(companion_count);
            remaining_actor_slots =
                remaining_actor_slots.saturating_sub(companion_count.saturating_add(1));
        }

        let plain_weights = plain_entries
            .iter()
            .map(|entry| entry.weight)
            .collect::<Vec<_>>();
        while fill_plain && leader_ordinal < table.rolls && remaining_actor_slots > 0 {
            let entry_index = if plain_entries.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&plain_weights)
            };
            let entry = &plain_entries[entry_index];
            let position = self.choose_generated_room_position(rooms, room_id, occupied, None);
            occupied.insert(position);
            leader_ordinal += 1;
            generated.push(self.generated_actor(
                format!("{}.encounter.{leader_ordinal}", definition.id),
                &entry.actor_kind_id,
                position,
            ));
            remaining_actor_slots -= 1;
        }
        generated
    }

    fn roll_inclusive(&mut self, minimum: u16, maximum: u16) -> u16 {
        debug_assert!(minimum <= maximum);
        if minimum == maximum {
            minimum
        } else {
            minimum
                + u16::try_from(self.rng.bounded(u64::from(maximum - minimum) + 1))
                    .expect("bounded encounter group count must fit u16")
        }
    }

    pub(in crate::game) fn select_spatial_vault_placements(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_candidates: &[ThemeVaultCandidateDefinition],
        guardian_present: bool,
        corridor_terrain_id: &str,
        terrain: &mut [String],
    ) -> Vec<GeneratedVaultPlacement> {
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("spatial vault placement requires a generation budget");
        let placement_limit = budget
            .vault_placements
            .expect("validated spatial vault count must remain available");
        let mut remaining_area = budget
            .vault_area_tiles
            .expect("validated spatial vault area must remain available");
        let fixed_actor_slots = definition
            .nest
            .as_ref()
            .map_or(0, |nest| nest.spawn_count)
            .saturating_add(u16::from(guardian_present));
        let ordinary_placement_reserve = budget.region_placements.unwrap_or(1);
        let mut remaining_vault_actor_slots = budget
            .actor_slots
            .saturating_sub(fixed_actor_slots)
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_vault_loot_placements = budget
            .loot_placements
            .saturating_sub(ordinary_placement_reserve);
        let mut remaining_candidates = eligible_candidates.to_vec();
        let mut placements = Vec::new();

        'placement_slots: for ordinal in 1..=placement_limit {
            loop {
                let affordable = remaining_candidates
                    .iter()
                    .enumerate()
                    .filter_map(|(index, candidate)| {
                        let vault = self
                            .content
                            .vault(&candidate.vault_id)
                            .expect("validated spatial vault must remain available");
                        let actor_cost = vault
                            .encounter_groups
                            .iter()
                            .map(|group| {
                                u16::try_from(group.member_positions.len())
                                    .expect("validated vault actor count must fit u16")
                            })
                            .sum::<u16>();
                        let loot_cost = u16::try_from(vault.loot_spawns.len())
                            .expect("validated vault loot count must fit u16");
                        let area = u32::from(vault.width) * u32::from(vault.height);
                        (actor_cost <= remaining_vault_actor_slots
                            && loot_cost <= remaining_vault_loot_placements
                            && area <= remaining_area)
                            .then_some((index, candidate.weight))
                    })
                    .collect::<Vec<_>>();
                if affordable.is_empty() {
                    break 'placement_slots;
                }
                let selected_affordable = if affordable.len() == 1 {
                    0
                } else {
                    let weights = affordable
                        .iter()
                        .map(|(_, weight)| *weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let candidate_index = affordable[selected_affordable].0;
                let candidate = remaining_candidates.remove(candidate_index);
                let vault = self
                    .content
                    .vault(&candidate.vault_id)
                    .expect("validated spatial vault must remain available")
                    .clone();
                let placement_candidates = free_vault_placement_candidates(
                    terrain,
                    definition.width,
                    definition.height,
                    &definition.wall_terrain_id,
                    corridor_terrain_id,
                    &vault,
                    &self.content,
                );
                if placement_candidates.is_empty() {
                    continue;
                }
                let placement_index = if placement_candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(placement_candidates.len())
                                .expect("vault placement candidate count must fit u64"),
                        ),
                    )
                    .expect("vault placement candidate index must fit usize")
                };
                let candidate = placement_candidates[placement_index].clone();
                let actor_cost = vault
                    .encounter_groups
                    .iter()
                    .map(|group| {
                        u16::try_from(group.member_positions.len())
                            .expect("validated vault actor count must fit u16")
                    })
                    .sum::<u16>();
                let loot_cost = u16::try_from(vault.loot_spawns.len())
                    .expect("validated vault loot count must fit u16");
                let area = u32::from(vault.width) * u32::from(vault.height);
                let placement = GeneratedVaultPlacement {
                    vault,
                    origin: candidate.origin,
                    transform: candidate.transform,
                    ordinal,
                    connector_cells: candidate.connector_cells,
                };
                apply_generated_vault_placement(
                    terrain,
                    definition.width,
                    corridor_terrain_id,
                    &placement,
                );
                remaining_vault_actor_slots =
                    remaining_vault_actor_slots.saturating_sub(actor_cost);
                remaining_vault_loot_placements =
                    remaining_vault_loot_placements.saturating_sub(loot_cost);
                remaining_area = remaining_area.saturating_sub(area);
                placements.push(placement);
                break;
            }
        }
        placements
    }

    pub(in crate::game) fn place_terrain_features(
        &mut self,
        definition: &ProceduralFloorDefinition,
        eligible_entries: &[TerrainFeatureEntryDefinition],
        context: TerrainFeaturePlacementContext<'_>,
        terrain: &mut [String],
    ) -> Vec<GeneratedTerrainFeature> {
        let placement_limit = definition
            .generation_budget
            .as_ref()
            .and_then(|budget| budget.feature_placements)
            .expect("terrain feature placement requires a validated budget");
        let mut placements = Vec::new();

        'placement_slots: for _ in 0..placement_limit {
            let mut remaining_entries = eligible_entries.to_vec();
            loop {
                if remaining_entries.is_empty() {
                    break 'placement_slots;
                }
                let selected_index = if remaining_entries.len() == 1 {
                    0
                } else {
                    let weights = remaining_entries
                        .iter()
                        .map(|entry| entry.weight)
                        .collect::<Vec<_>>();
                    self.roll_weighted_index(&weights)
                };
                let entry = remaining_entries.remove(selected_index);
                let candidates = terrain_feature_placement_candidates(
                    terrain,
                    definition.width,
                    context.floor_terrain_id,
                    context.room_floor_terrain_ids,
                    context.rooms,
                    context.reserved,
                    entry.placement,
                );
                if candidates.is_empty() {
                    continue;
                }
                let position_index = if candidates.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(candidates.len())
                                .expect("terrain feature candidate count must fit u64"),
                        ),
                    )
                    .expect("terrain feature candidate index must fit usize")
                };
                let position = candidates[position_index];
                set_generated_terrain(terrain, definition.width, position, &entry.terrain_id);
                placements.push(GeneratedTerrainFeature {
                    terrain_id: entry.terrain_id,
                    position,
                });
                break;
            }
        }
        placements
    }

    fn choose_generated_room_position(
        &mut self,
        rooms: &[GeneratedRoom],
        room_id: &str,
        occupied: &BTreeSet<Position>,
        walkable_terrain: Option<(&[String], u16)>,
    ) -> Position {
        let room = rooms
            .iter()
            .find(|room| room.id == room_id)
            .expect("validated procedural room ID must remain available");
        let candidates = (room.y..room.y + room.height)
            .flat_map(|y| (room.x..room.x + room.width).map(move |x| Position { x, y }))
            .filter(|position| {
                room.contains(*position)
                    && !occupied.contains(position)
                    && walkable_terrain.is_none_or(|(terrain, width)| {
                        self.content
                            .terrain(&terrain[generated_terrain_index(width, *position)])
                            .is_some_and(|terrain| terrain.walkable)
                    })
            })
            .collect::<Vec<_>>();
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated room candidate count must fit u64"),
        ))
        .expect("bounded generated room candidate index must fit usize");
        candidates[index]
    }

    fn choose_generated_room_position_for_actor(
        &mut self,
        rooms: &[GeneratedRoom],
        room_id: &str,
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
        actor_kind_id: &str,
    ) -> Option<Position> {
        let room = rooms
            .iter()
            .find(|room| room.id == room_id)
            .expect("validated procedural room ID must remain available");
        let candidates = (room.y..room.y + room.height)
            .flat_map(|y| (room.x..room.x + room.width).map(move |x| Position { x, y }))
            .filter(|position| {
                room.contains(*position)
                    && !occupied.contains(position)
                    && generated_actor_can_enter_position(
                        &self.content,
                        terrain,
                        width,
                        actor_kind_id,
                        *position,
                    )
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated actor candidate count must fit u64"),
        ))
        .expect("bounded generated actor candidate index must fit usize");
        Some(candidates[index])
    }

    fn choose_generated_rooms_position(
        &mut self,
        rooms: &[GeneratedRoom],
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
    ) -> (String, Position) {
        let content = &self.content;
        let candidates = rooms
            .iter()
            .flat_map(|room| {
                (room.y..room.y + room.height).flat_map(move |y| {
                    (room.x..room.x + room.width).filter_map(move |x| {
                        let position = Position { x, y };
                        (room.contains(position)
                            && !occupied.contains(&position)
                            && content
                                .terrain(&terrain[generated_terrain_index(width, position)])
                                .is_some_and(|terrain| terrain.walkable))
                        .then_some((room.id.clone(), position))
                    })
                })
            })
            .collect::<Vec<_>>();
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated room candidate count must fit u64"),
        ))
        .expect("bounded generated room candidate index must fit usize");
        candidates[index].clone()
    }

    fn choose_generated_floor_position(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain: &[String],
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let candidates = terrain
            .iter()
            .enumerate()
            .filter_map(|(index, terrain_id)| {
                let position = Position {
                    x: i32::try_from(index % usize::from(definition.width))
                        .expect("generated floor x must fit i32"),
                    y: i32::try_from(index / usize::from(definition.width))
                        .expect("generated floor y must fit i32"),
                };
                (self
                    .content
                    .terrain(terrain_id)
                    .is_some_and(|terrain| terrain.walkable)
                    && !occupied.contains(&position))
                .then_some(position)
            })
            .collect::<Vec<_>>();
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("generated floor candidate count must fit u64"),
        ))
        .expect("bounded generated floor candidate index must fit usize");
        candidates[index]
    }

    fn choose_generated_dungeon_task_target_position(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain: &[String],
        occupied: &BTreeSet<Position>,
        actor_kind_id: &str,
        entry: Position,
    ) -> Option<Position> {
        const ORIGINAL_QUEST_TARGET_MINIMUM_DISTANCE: u32 = 10;

        let candidates = terrain
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                let position = Position {
                    x: i32::try_from(index % usize::from(definition.width))
                        .expect("generated task target x must fit i32"),
                    y: i32::try_from(index / usize::from(definition.width))
                        .expect("generated task target y must fit i32"),
                };
                (!occupied.contains(&position)
                    && chebyshev_distance(entry, position)
                        >= ORIGINAL_QUEST_TARGET_MINIMUM_DISTANCE
                    && generated_actor_can_enter_position(
                        &self.content,
                        terrain,
                        definition.width,
                        actor_kind_id,
                        position,
                    ))
                .then_some(position)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let index = usize::try_from(
            self.rng.bounded(
                u64::try_from(candidates.len())
                    .expect("generated task target candidate count must fit u64"),
            ),
        )
        .expect("bounded generated task target candidate index must fit usize");
        Some(candidates[index])
    }

    fn scaled_normal_allocation(
        &mut self,
        rule: ProceduralNormalAllocationDefinition,
        definition: &ProceduralFloorDefinition,
        reference_area_tiles: u32,
    ) -> u16 {
        let centered_sum = (0..12)
            .map(|_| i32::try_from(self.rng.bounded(6)).expect("normal die must fit i32"))
            .sum::<i32>()
            - 30;
        let spread = i32::from(rule.standard_deviation);
        let offset = (centered_sum * spread / 6).clamp(-4 * spread, 4 * spread);
        let raw = (i32::from(rule.mean) + offset).max(0);
        let map_area = u64::from(definition.width) * u64::from(definition.height);
        let scaled = u64::try_from(raw).expect("non-negative allocation must fit u64") * map_area;
        let reference = u64::from(reference_area_tiles);
        let mut count = scaled / reference;
        let remainder = scaled % reference;
        if remainder > 0 && self.rng.bounded(reference) < remainder {
            count += 1;
        }
        u16::try_from(count.max(1)).expect("validated allocation count must fit u16")
    }

    fn choose_generated_region_position(
        &mut self,
        region: &GeneratedRegion,
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
    ) -> Position {
        let candidates =
            generated_region_open_positions(region, terrain, width, &self.content, occupied);
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("regional candidate count must fit u64"),
        ))
        .expect("regional candidate index must fit usize");
        candidates[index]
    }

    fn choose_generated_region_position_for_actor(
        &mut self,
        region: &GeneratedRegion,
        terrain: &[String],
        width: u16,
        occupied: &BTreeSet<Position>,
        actor_kind_id: &str,
    ) -> Option<Position> {
        let candidates = region
            .state
            .cells
            .iter()
            .copied()
            .filter(|position| {
                !occupied.contains(position)
                    && generated_actor_can_enter_position(
                        &self.content,
                        terrain,
                        width,
                        actor_kind_id,
                        *position,
                    )
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let index = usize::try_from(self.rng.bounded(
            u64::try_from(candidates.len()).expect("regional actor candidate count must fit u64"),
        ))
        .expect("regional actor candidate index must fit usize");
        Some(candidates[index])
    }
}

impl Game {
    pub(in crate::game) fn generate_maze(
        &mut self,
        definition: &ProceduralFloorDefinition,
        maze: &ProceduralMazeDefinition,
        floor_terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        let left = i32::from((definition.width - maze.width) / 2);
        let top = i32::from((definition.height - maze.height) / 2);
        for y in top..top + i32::from(maze.height) {
            for x in left..left + i32::from(maze.width) {
                set_generated_terrain(
                    terrain,
                    definition.width,
                    Position { x, y },
                    &definition.wall_terrain_id,
                );
            }
        }

        let columns = usize::from(maze.width.div_ceil(2));
        let rows = usize::from(maze.height.div_ceil(2));
        let vertex_count = columns * rows;
        let root = usize::try_from(
            self.rng
                .bounded(u64::try_from(vertex_count).expect("maze vertex count must fit u64")),
        )
        .expect("maze root must fit usize");
        let node_position = |node: usize| Position {
            x: left + i32::try_from((node % columns) * 2).expect("maze x must fit i32"),
            y: top + i32::try_from((node / columns) * 2).expect("maze y must fit i32"),
        };
        let mut visited = BTreeSet::from([root]);
        let mut stack = vec![root];
        let mut carved = BTreeSet::new();
        let root_position = node_position(root);
        carved.insert(root_position);
        set_generated_terrain(terrain, definition.width, root_position, floor_terrain_id);

        while let Some(&node) = stack.last() {
            let column = node % columns;
            let row = node / columns;
            let mut neighbors = Vec::new();
            if row > 0 && !visited.contains(&(node - columns)) {
                neighbors.push(node - columns);
            }
            if column + 1 < columns && !visited.contains(&(node + 1)) {
                neighbors.push(node + 1);
            }
            if row + 1 < rows && !visited.contains(&(node + columns)) {
                neighbors.push(node + columns);
            }
            if column > 0 && !visited.contains(&(node - 1)) {
                neighbors.push(node - 1);
            }
            if neighbors.is_empty() {
                stack.pop();
                continue;
            }
            let neighbor_index = if neighbors.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(neighbors.len()).expect("maze neighbor count must fit u64"),
                ))
                .expect("maze neighbor index must fit usize")
            };
            let neighbor = neighbors[neighbor_index];
            let from = node_position(node);
            let to = node_position(neighbor);
            let connector = Position {
                x: (from.x + to.x) / 2,
                y: (from.y + to.y) / 2,
            };
            for position in [connector, to] {
                carved.insert(position);
                set_generated_terrain(terrain, definition.width, position, floor_terrain_id);
            }
            visited.insert(neighbor);
            stack.push(neighbor);
        }

        carved
    }

    pub(in crate::game) fn generate_destroyed_region(
        &mut self,
        definition: &ProceduralFloorDefinition,
        terrain_id: &str,
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const CARDINAL_OFFSETS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("destroyed region requires a generation budget");
        let center_count = usize::from(
            budget
                .destruction_centers
                .expect("validated destruction center budget must remain available"),
        );
        let area = usize::try_from(
            budget
                .destroyed_area_tiles
                .expect("validated destroyed area budget must remain available"),
        )
        .expect("destroyed area must fit usize");
        let margin_x = i32::from((definition.width / 5).max(2));
        let margin_y = i32::from((definition.height / 5).max(2));
        let mut center_candidates = (margin_y..i32::from(definition.height) - margin_y)
            .flat_map(|y| {
                (margin_x..i32::from(definition.width) - margin_x).map(move |x| Position { x, y })
            })
            .collect::<Vec<_>>();
        let mut selected = BTreeSet::new();
        for _ in 0..center_count {
            let index = if center_candidates.len() == 1 {
                0
            } else {
                usize::try_from(
                    self.rng.bounded(
                        u64::try_from(center_candidates.len())
                            .expect("destruction center count must fit u64"),
                    ),
                )
                .expect("destruction center index must fit usize")
            };
            selected.insert(center_candidates.remove(index));
        }

        while selected.len() < area {
            let mut frontier = selected
                .iter()
                .flat_map(|position| {
                    CARDINAL_OFFSETS.map(|(dx, dy)| Position {
                        x: position.x + dx,
                        y: position.y + dy,
                    })
                })
                .filter(|position| {
                    position.x > 0
                        && position.y > 0
                        && position.x + 1 < i32::from(definition.width)
                        && position.y + 1 < i32::from(definition.height)
                        && !selected.contains(position)
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            frontier.sort_by_key(|position| (position.y, position.x));
            let index = if frontier.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(frontier.len()).expect("destroyed frontier count must fit u64"),
                ))
                .expect("destroyed frontier index must fit usize")
            };
            selected.insert(frontier[index]);
        }
        for position in &selected {
            set_generated_terrain(terrain, definition.width, *position, terrain_id);
        }
        selected
    }

    fn roll_streamer_terrain(
        &mut self,
        candidate: &ProceduralStreamerCandidateDefinition,
    ) -> String {
        let Some(treasure) = &candidate.treasure else {
            return candidate.terrain_id.clone();
        };
        if self.rng.bounded(u64::from(treasure.known_one_in)) == 0 {
            treasure.known_terrain_id.clone()
        } else if self.rng.bounded(u64::from(treasure.hidden_one_in)) == 0 {
            treasure.hidden_terrain_id.clone()
        } else {
            candidate.terrain_id.clone()
        }
    }

    pub(in crate::game) fn generate_streamers(
        &mut self,
        definition: &ProceduralFloorDefinition,
        streamers: &[ProceduralStreamerCandidateDefinition],
        terrain: &mut [String],
    ) -> BTreeSet<Position> {
        const DIRECTIONS: [(i32, i32); 8] = [
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ];
        let budget = definition
            .generation_budget
            .as_ref()
            .expect("streamers require a generation budget");
        let placement_count = budget
            .streamer_placements
            .expect("validated streamer placement count must remain available");
        let area = usize::try_from(
            budget
                .streamer_area_tiles
                .expect("validated streamer area budget must remain available"),
        )
        .expect("streamer area must fit usize");
        let weights = streamers
            .iter()
            .map(|candidate| candidate.weight)
            .collect::<Vec<_>>();
        let mut assignments = BTreeMap::<Position, usize>::new();

        for _ in 0..placement_count {
            let streamer_index = if streamers.len() == 1 {
                0
            } else {
                self.roll_weighted_index(&weights)
            };
            let mut starts = Vec::new();
            for y in (definition.height / 3)..=(definition.height * 2 / 3) {
                for x in (definition.width / 3)..=(definition.width * 2 / 3) {
                    let position = Position {
                        x: i32::from(x),
                        y: i32::from(y),
                    };
                    if terrain[generated_terrain_index(definition.width, position)]
                        == definition.wall_terrain_id
                    {
                        starts.push(position);
                    }
                }
            }
            if starts.is_empty() {
                starts = generated_wall_positions(definition, terrain);
            }
            if starts.is_empty() {
                break;
            }
            starts.sort_by_key(|position| (position.y, position.x));
            let start_index = if starts.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(starts.len()).expect("streamer start count must fit u64"),
                ))
                .expect("streamer start index must fit usize")
            };
            let direction_index =
                usize::try_from(self.rng.bounded(8)).expect("streamer direction must fit usize");
            let (dx, dy) = DIRECTIONS[direction_index];
            let mut cursor = starts[start_index];
            while cursor.x > 0
                && cursor.y > 0
                && cursor.x + 1 < i32::from(definition.width)
                && cursor.y + 1 < i32::from(definition.height)
            {
                for y in cursor.y - 1..=cursor.y + 1 {
                    for x in cursor.x - 1..=cursor.x + 1 {
                        let position = Position { x, y };
                        if position.x > 0
                            && position.y > 0
                            && position.x + 1 < i32::from(definition.width)
                            && position.y + 1 < i32::from(definition.height)
                            && terrain[generated_terrain_index(definition.width, position)]
                                == definition.wall_terrain_id
                        {
                            assignments.entry(position).or_insert(streamer_index);
                        }
                    }
                }
                cursor.x += dx;
                cursor.y += dy;
            }
        }

        let mut painted = BTreeSet::new();
        while painted.len() < area {
            let mut candidates = assignments
                .iter()
                .filter_map(|(position, streamer_index)| {
                    (!painted.contains(position)
                        && terrain[generated_terrain_index(definition.width, *position)]
                            == definition.wall_terrain_id)
                        .then_some((*position, *streamer_index))
                })
                .collect::<Vec<_>>();
            candidates.sort_by_key(|(position, _)| (position.y, position.x));
            if candidates.is_empty() {
                let fallback = generated_wall_positions(definition, terrain);
                if fallback.is_empty() {
                    break;
                }
                let index = if fallback.len() == 1 {
                    0
                } else {
                    usize::try_from(
                        self.rng.bounded(
                            u64::try_from(fallback.len())
                                .expect("streamer fallback count must fit u64"),
                        ),
                    )
                    .expect("streamer fallback index must fit usize")
                };
                let position = fallback[index];
                let terrain_id = self.roll_streamer_terrain(&streamers[0]);
                set_generated_terrain(terrain, definition.width, position, &terrain_id);
                painted.insert(position);
                continue;
            }
            let index = if candidates.len() == 1 {
                0
            } else {
                usize::try_from(self.rng.bounded(
                    u64::try_from(candidates.len()).expect("streamer candidate count must fit u64"),
                ))
                .expect("streamer candidate index must fit usize")
            };
            let (position, streamer_index) = candidates[index];
            let terrain_id = self.roll_streamer_terrain(&streamers[streamer_index]);
            set_generated_terrain(terrain, definition.width, position, &terrain_id);
            painted.insert(position);
        }
        painted
    }
}

fn place_generated_floor_connections(
    definition: &ProceduralFloorDefinition,
    entry_anchor: Position,
    down_stair_anchor: Position,
    fixed_trap_position: Position,
    floor_terrain_id: &str,
    terrain: &mut [String],
    rng: &mut RfbRng,
) -> Result<Vec<FloorConnectionState>, CoreError> {
    let terrain_ref: &[String] = terrain;
    let mut candidates = (1..definition.height - 1)
        .flat_map(|y| {
            (1..definition.width - 1).filter_map(move |x| {
                let position = Position {
                    x: i32::from(x),
                    y: i32::from(y),
                };
                (position != fixed_trap_position
                    && terrain_ref[generated_terrain_index(definition.width, position)]
                        == floor_terrain_id)
                    .then_some(position)
            })
        })
        .collect::<Vec<_>>();
    let (primary_up_id, primary_down_id) = primary_floor_connection_ids(definition);
    let mut ordered_connections = Vec::with_capacity(definition.connections.len());
    for connection_id in [primary_up_id, primary_down_id].into_iter().flatten() {
        ordered_connections.push(
            definition
                .connections
                .iter()
                .find(|connection| connection.id == connection_id)
                .expect("selected primary connection must remain available"),
        );
    }
    ordered_connections.extend(definition.connections.iter().filter(|connection| {
        primary_up_id != Some(connection.id.as_str())
            && primary_down_id != Some(connection.id.as_str())
    }));

    let mut placed = Vec::with_capacity(definition.connections.len());
    for connection in ordered_connections {
        let position = if primary_up_id == Some(connection.id.as_str()) {
            entry_anchor
        } else if primary_down_id == Some(connection.id.as_str()) {
            down_stair_anchor
        } else {
            if candidates.is_empty() {
                return Err(CoreError::InvalidSave(
                    "generated floor has insufficient connection space",
                ));
            }
            let candidate_index = usize::try_from(rng.bounded(candidates.len() as u64))
                .expect("bounded connection index must fit usize");
            candidates[candidate_index]
        };
        candidates.retain(|candidate| *candidate != position);
        set_generated_terrain(terrain, definition.width, position, &connection.terrain_id);
        placed.push(FloorConnectionState {
            id: connection.id.clone(),
            position,
            target_floor_id: None,
            target_connection_id: None,
        });
    }
    placed.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(placed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streamer_treasure_rolls_known_then_hidden_after_a_miss() {
        let template = Game::new(1);
        let candidate = template
            .content
            .world(&template.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find_map(|floor| floor.layout.as_ref())
            })
            .and_then(|layout| {
                layout
                    .streamers
                    .iter()
                    .find(|candidate| candidate.terrain_id == "demo.terrain.magma-vein")
            })
            .expect("demo magma streamer must remain available")
            .clone();
        let treasure = candidate
            .treasure
            .as_ref()
            .expect("magma streamer must define treasure");
        assert_eq!(treasure.known_one_in, 60);
        assert_eq!(treasure.hidden_one_in, 20);

        let mut found = BTreeMap::new();
        for seed in 0..10_000 {
            let mut game = Game::new(1);
            game.rng = RfbRng::seeded(seed);
            let before = game.rng_draw_counter();
            let terrain_id = game.roll_streamer_terrain(&candidate);
            found
                .entry(terrain_id)
                .or_insert(game.rng_draw_counter() - before);
            if found.len() == 3 {
                break;
            }
        }
        assert_eq!(found.get("demo.terrain.magma-treasure"), Some(&1));
        assert_eq!(found.get("demo.terrain.magma-hidden-treasure"), Some(&2));
        assert_eq!(found.get("demo.terrain.magma-vein"), Some(&2));
    }
}
