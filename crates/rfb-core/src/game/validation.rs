// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    action::GameAction,
    error::CoreError,
    state::{Actor, FloorConnectionState, FloorRegionState, ItemInstance, ItemLocation},
};
use rfb_content::{AffixPropertyBundleDefinition, ContentCatalog};
use rfb_protocol::{MonsterPackBehaviorDto, MonsterPackRoleDto, Position};

use super::Game;

impl Game {
    pub(super) fn validate_runtime_invariants(&self, action: &GameAction) -> Result<(), CoreError> {
        self.active_task_objective()?;
        match action {
            GameAction::UseItem { item_id, .. }
            | GameAction::UseItemForRecharge { item_id, .. } => {
                self.inventory_item_use_context(item_id)?;
            }
            _ => {}
        }
        Ok(())
    }
}

pub(super) fn monster_packs_are_valid(entities: &[Actor]) -> bool {
    let mut packs = BTreeMap::<&str, Vec<&Actor>>::new();
    for entity in entities {
        let Some(pack) = &entity.pack else {
            continue;
        };
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 128
                && id.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
        };
        if !valid_id(&pack.id) || !valid_id(&pack.leader_id) {
            return false;
        }
        packs.entry(&pack.id).or_default().push(entity);
    }
    packs.into_values().all(|members| {
        let leaders = members
            .iter()
            .filter(|entity| {
                entity.pack.as_ref().is_some_and(|pack| {
                    pack.role == MonsterPackRoleDto::Leader
                        && pack.behavior != MonsterPackBehaviorDto::GuardLeader
                })
            })
            .copied()
            .collect::<Vec<_>>();
        leaders.len() == 1
            && members.iter().all(|entity| {
                let pack = entity
                    .pack
                    .as_ref()
                    .expect("pack member must retain identity");
                pack.leader_id == leaders[0].id
                    && ((pack.role == MonsterPackRoleDto::Leader && entity.id == pack.leader_id)
                        || (pack.role == MonsterPackRoleDto::Member && entity.id != pack.leader_id))
            })
    })
}

pub(super) fn rolled_affixes_are_valid(item: &ItemInstance) -> bool {
    item.rolled_affixes
        .windows(2)
        .all(|pair| pair[0].affix_id < pair[1].affix_id)
        && item.rolled_affixes.iter().all(|rolled| {
            item.affix_ids.binary_search(&rolled.affix_id).is_ok()
                && rolled.properties != AffixPropertyBundleDefinition::default()
        })
}

pub(super) fn floor_regions_are_valid(
    floor_id: &str,
    dimensions: (u16, u16),
    regions: &[FloorRegionState],
    entities: &[Actor],
    items: &[ItemInstance],
    world: &rfb_content::WorldDefinition,
    content: &ContentCatalog,
) -> bool {
    let (width, height) = dimensions;
    if regions.is_empty() {
        return true;
    }
    let Some(definition) = world
        .procedural_floors
        .iter()
        .find(|definition| definition.id == floor_id)
    else {
        return false;
    };
    let Some(table) = definition
        .region_table_id
        .as_deref()
        .and_then(|table_id| content.region_table(table_id))
    else {
        return false;
    };
    let expected_count = definition
        .generation_budget
        .as_ref()
        .and_then(|budget| budget.region_placements)
        .map(usize::from);
    if expected_count != Some(regions.len())
        || regions
            .windows(2)
            .any(|pair| pair[0].region_id >= pair[1].region_id)
    {
        return false;
    }
    let mut cells = BTreeSet::new();
    for region in regions {
        let candidate_is_valid = table.entries.iter().any(|entry| {
            entry.region_id == region.region_id
                && entry.theme_id == region.theme_id
                && entry.encounter_table_id == region.encounter_table_id
                && entry.loot_table_id == region.loot_table_id
                && entry.min_depth <= definition.depth
                && definition.depth <= entry.max_depth
        });
        if !candidate_is_valid
            || region.cells.is_empty()
            || region.cells.windows(2).any(|pair| pair[0] >= pair[1])
            || region.cells.iter().any(|position| {
                position.x < 0
                    || position.y < 0
                    || position.x >= i32::from(width)
                    || position.y >= i32::from(height)
                    || !cells.insert(*position)
            })
        {
            return false;
        }
    }
    if entities
        .iter()
        .any(|entity| !cells.contains(&entity.position))
    {
        return false;
    }
    items.iter().all(|item| match &item.location {
        ItemLocation::Ground(position) => cells.contains(position),
        ItemLocation::CarriedBy { actor_id } => entities
            .iter()
            .find(|entity| &entity.id == actor_id)
            .is_some_and(|entity| cells.contains(&entity.position)),
        ItemLocation::Inventory | ItemLocation::Equipped { .. } => true,
    })
}

pub(super) fn floor_connections_are_valid(
    floor_id: &str,
    width: u16,
    height: u16,
    terrain: &[String],
    connections: &[FloorConnectionState],
    world: &rfb_content::WorldDefinition,
) -> bool {
    if connections.is_empty() {
        return true;
    }
    if floor_id == world.initial_floor_id {
        return false;
    }
    let Some(definition) = world
        .procedural_floors
        .iter()
        .find(|definition| definition.id == floor_id)
    else {
        return false;
    };
    if definition.connections.len() != connections.len() {
        return false;
    }
    let expected_ids = definition
        .connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect::<BTreeSet<_>>();
    let actual_ids = connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect::<BTreeSet<_>>();
    let unique_positions = connections
        .iter()
        .map(|connection| connection.position)
        .collect::<BTreeSet<_>>();
    expected_ids == actual_ids
        && unique_positions.len() == connections.len()
        && connections.iter().all(|state| {
            let position = state.position;
            let Some(connection) = definition
                .connections
                .iter()
                .find(|connection| connection.id == state.id)
            else {
                return false;
            };
            position.x >= 0
                && position.y >= 0
                && position.x < i32::from(width)
                && position.y < i32::from(height)
                && terrain
                    .get(position.y as usize * usize::from(width) + position.x as usize)
                    .is_some_and(|terrain_id| terrain_id == &connection.terrain_id)
                && floor_connection_target_is_valid(floor_id, connection, state, world)
        })
}

fn floor_connection_target_is_valid(
    floor_id: &str,
    connection: &rfb_content::ProceduralFloorConnectionDefinition,
    state: &FloorConnectionState,
    world: &rfb_content::WorldDefinition,
) -> bool {
    match (&state.target_floor_id, &state.target_connection_id) {
        (None, None) => true,
        (Some(target_floor_id), None) => {
            target_floor_id == &world.initial_floor_id
                && connection.target_floor_id == world.initial_floor_id
                && connection.target_connection_id.is_none()
        }
        (Some(target_floor_id), Some(target_connection_id)) => {
            let directly_declared = (connection.target_floor_id == *target_floor_id
                && connection.target_connection_id.as_deref() == Some(target_connection_id))
                || connection.target_candidates.iter().any(|candidate| {
                    candidate.target_floor_id == *target_floor_id
                        && candidate.target_connection_id == *target_connection_id
                });
            if directly_declared {
                return true;
            }
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == *target_floor_id)
                .and_then(|floor| {
                    floor
                        .connections
                        .iter()
                        .find(|candidate| candidate.id == *target_connection_id)
                })
                .is_some_and(|parent_connection| {
                    (parent_connection.target_floor_id == floor_id
                        && parent_connection.target_connection_id.as_deref()
                            == Some(state.id.as_str()))
                        || parent_connection.target_candidates.iter().any(|candidate| {
                            candidate.target_floor_id == floor_id
                                && candidate.target_connection_id == state.id
                        })
                })
        }
        _ => false,
    }
}

pub(super) fn revealed_terrain_is_valid(
    revealed: &BTreeSet<Position>,
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> bool {
    revealed.iter().all(|position| {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(width)
            || position.y >= i32::from(height)
        {
            return false;
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        terrain
            .get(index)
            .and_then(|terrain_id| content.terrain(terrain_id))
            .is_some_and(|definition| definition.concealed_as_terrain_id.is_some())
    })
}
