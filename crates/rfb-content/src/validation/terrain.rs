// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::*;

use super::shared::{
    insert_definition_id, normalize_tags, require_format_version, require_reference,
    require_schema, validate_definition_id, validate_definition_text, validate_glyph,
};

pub(super) struct TerrainValidationOutputs {
    pub(super) terrain_ids: BTreeSet<String>,
    pub(super) terrain_walkability: BTreeMap<String, bool>,
    pub(super) terrain_connectability: BTreeMap<String, bool>,
    pub(super) terrain_tags: BTreeMap<String, BTreeSet<String>>,
    pub(super) terrain_open_targets: BTreeMap<String, String>,
    pub(super) terrain_traps: BTreeSet<String>,
}

pub(super) fn validate_terrain(
    terrain_definitions: &mut [TerrainDefinition],
    all_ids: &mut BTreeSet<String>,
) -> Result<TerrainValidationOutputs, ContentError> {
    let mut terrain_ids = BTreeSet::new();
    let mut terrain_walkability = BTreeMap::new();
    let mut terrain_connectability = BTreeMap::new();
    let mut terrain_tags = BTreeMap::new();
    let mut terrain_open_targets = BTreeMap::new();
    let mut terrain_traps = BTreeSet::new();
    for terrain in terrain_definitions.iter_mut() {
        require_schema(&terrain.schema, TERRAIN_SCHEMA, &terrain.id)?;
        require_format_version(terrain.format_version, &terrain.id)?;
        validate_definition_id(&terrain.id, "terrain")?;
        validate_definition_text(&terrain.id, &terrain.name_key, &terrain.description_key)?;
        validate_glyph(&terrain.id, &terrain.glyph)?;
        normalize_tags(&terrain.id, &mut terrain.tags)?;
        insert_definition_id(all_ids, &terrain.id)?;
        terrain_ids.insert(terrain.id.clone());
        terrain_walkability.insert(terrain.id.clone(), terrain.walkable);
        terrain_connectability.insert(
            terrain.id.clone(),
            terrain.walkable
                || terrain.open_to_terrain_id.is_some()
                || terrain.bash_to_terrain_id.is_some()
                || terrain.dig_to_terrain_id.is_some(),
        );
        terrain_tags.insert(
            terrain.id.clone(),
            terrain.tags.iter().cloned().collect::<BTreeSet<_>>(),
        );
        if let Some(target_id) = &terrain.open_to_terrain_id {
            terrain_open_targets.insert(terrain.id.clone(), target_id.clone());
        }
        if terrain.trap.is_some() {
            terrain_traps.insert(terrain.id.clone());
        }
    }
    for terrain in terrain_definitions.iter() {
        if terrain.open_to_terrain_id.is_some() && terrain.close_to_terrain_id.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.open_check_difficulty.is_some() && terrain.open_to_terrain_id.is_none() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.bash_to_terrain_id.is_some() != terrain.bash_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.dig_to_terrain_id.is_some() != terrain.dig_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.concealed_as_terrain_id.is_some() != terrain.search_check_difficulty.is_some() {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain.perception_check_difficulty.is_some()
            && terrain.concealed_as_terrain_id.is_none()
        {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if terrain
            .open_check_difficulty
            .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .bash_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .dig_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .search_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
            || terrain
                .perception_check_difficulty
                .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
        {
            return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
        }
        if let Some(target_id) = &terrain.open_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
                || (terrain.open_check_difficulty.is_none()
                    && target.close_to_terrain_id.as_deref() != Some(terrain.id.as_str()))
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.close_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || !terrain.walkable
                || terrain.blocks_sight
                || target.walkable
                || !target.blocks_sight
                || target.open_to_terrain_id.as_deref() != Some(terrain.id.as_str())
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.bash_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.dig_to_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable
                || !terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(target_id) = &terrain.concealed_as_terrain_id {
            require_reference(&terrain_ids, target_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == *target_id)
                .expect("validated terrain target must remain available");
            if target_id == &terrain.id
                || terrain.walkable != target.walkable
                || terrain.blocks_sight != target.blocks_sight
                || target.concealed_as_terrain_id.is_some()
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
        if let Some(trap) = &terrain.trap {
            require_reference(&terrain_ids, &trap.disarm_to_terrain_id, &terrain.id)?;
            let target = terrain_definitions
                .iter()
                .find(|candidate| candidate.id == trap.disarm_to_terrain_id)
                .expect("validated trap target must remain available");
            if trap.damage <= 0
                || !(1..=1_000_000).contains(&trap.disarm_check_difficulty)
                || trap
                    .saving_throw_difficulty
                    .is_some_and(|difficulty| !(1..=1_000_000).contains(&difficulty))
                || trap.disarm_to_terrain_id == terrain.id
                || !terrain.walkable
                || terrain.blocks_sight
                || !target.walkable
                || target.blocks_sight
                || terrain.concealed_as_terrain_id.is_none()
            {
                return Err(ContentError::InvalidTerrainTransition(terrain.id.clone()));
            }
        }
    }
    Ok(TerrainValidationOutputs {
        terrain_ids,
        terrain_walkability,
        terrain_connectability,
        terrain_tags,
        terrain_open_targets,
        terrain_traps,
    })
}
