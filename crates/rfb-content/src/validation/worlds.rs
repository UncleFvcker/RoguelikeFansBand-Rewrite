// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::*;

use super::shared::{
    require_actor_role, require_reference, validate_definition_id, validate_id,
    validate_message_key, validate_position,
};

fn cavern_room_area(width: u16, height: u16) -> u32 {
    (u32::from(width) * u32::from(height) * 5 / 8).max(10)
}

fn valid_procedural_count_range(range: ProceduralCountRangeDefinition) -> bool {
    (1..=8).contains(&range.minimum) && range.minimum <= range.maximum && range.maximum <= 8
}

pub(super) struct WorldValidationRefs<'a> {
    pub(super) terrain_ids: &'a BTreeSet<String>,
    pub(super) terrain: &'a [TerrainDefinition],
    pub(super) terrain_walkability: &'a BTreeMap<String, bool>,
    pub(super) terrain_tags: &'a BTreeMap<String, BTreeSet<String>>,
    pub(super) terrain_open_targets: &'a BTreeMap<String, String>,
    pub(super) terrain_traps: &'a BTreeSet<String>,
    pub(super) actor_roles: &'a BTreeMap<String, ActorRole>,
    pub(super) actor_levels: &'a BTreeMap<String, u32>,
    pub(super) actors: &'a [ActorDefinition],
    pub(super) item_limits: &'a BTreeMap<String, (u32, bool)>,
    pub(super) items: &'a [ItemDefinition],
    pub(super) affix_ids: &'a BTreeSet<String>,
    pub(super) encounter_tables: &'a BTreeMap<String, EncounterTableDefinition>,
    pub(super) loot_table_ids: &'a BTreeSet<String>,
    pub(super) loot_tables: &'a BTreeMap<String, LootTableDefinition>,
    pub(super) theme_tables: &'a BTreeMap<String, ThemeTableDefinition>,
    pub(super) region_tables: &'a BTreeMap<String, RegionTableDefinition>,
    pub(super) terrain_feature_tables: &'a BTreeMap<String, TerrainFeatureTableDefinition>,
    pub(super) vaults: &'a BTreeMap<String, VaultDefinition>,
    pub(super) build_ids: &'a BTreeSet<String>,
    pub(super) towns: &'a BTreeMap<String, TownDefinition>,
    pub(super) town_facilities: &'a BTreeMap<String, TownFacilityDefinition>,
    pub(super) shops: &'a BTreeMap<String, ShopDefinition>,
}

fn validate_wilderness(
    world_id: &str,
    wilderness: &mut WildernessDefinition,
    dungeon_ids: &BTreeSet<String>,
    towns: &BTreeMap<String, TownDefinition>,
) -> Result<(), ContentError> {
    if wilderness.width < 3
        || wilderness.height < 3
        || wilderness.width > 512
        || wilderness.height > 512
        || wilderness.rows.len() != usize::from(wilderness.height)
        || wilderness.legend.is_empty()
    {
        return Err(ContentError::InvalidWilderness(world_id.to_owned()));
    }

    wilderness
        .legend
        .sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let mut terrain_by_symbol = BTreeMap::new();
    for entry in &wilderness.legend {
        let mut symbols = entry.symbol.chars();
        let Some(symbol) = symbols.next() else {
            return Err(ContentError::InvalidWilderness(world_id.to_owned()));
        };
        if symbols.next().is_some()
            || !symbol.is_ascii()
            || symbol.is_ascii_control()
            || terrain_by_symbol.insert(symbol, entry.terrain).is_some()
        {
            return Err(ContentError::InvalidWilderness(world_id.to_owned()));
        }
    }

    let rows = wilderness
        .rows
        .iter()
        .map(|row| row.chars().collect::<Vec<_>>())
        .collect::<Vec<_>>();
    for (y, row) in rows.iter().enumerate() {
        if row.len() != usize::from(wilderness.width) {
            return Err(ContentError::InvalidWilderness(world_id.to_owned()));
        }
        for (x, symbol) in row.iter().enumerate() {
            let Some(terrain) = terrain_by_symbol.get(symbol) else {
                return Err(ContentError::InvalidWilderness(world_id.to_owned()));
            };
            if (x == 0
                || y == 0
                || x + 1 == usize::from(wilderness.width)
                || y + 1 == usize::from(wilderness.height))
                && *terrain != WildernessTerrain::Edge
            {
                return Err(ContentError::InvalidWilderness(world_id.to_owned()));
            }
        }
    }

    let position_is_inside = |position: ContentPosition| {
        position.x < wilderness.width
            && position.y < wilderness.height
            && terrain_by_symbol.get(&rows[usize::from(position.y)][usize::from(position.x)])
                != Some(&WildernessTerrain::Edge)
    };
    if !position_is_inside(wilderness.start_position) {
        return Err(ContentError::InvalidWilderness(world_id.to_owned()));
    }

    wilderness.locations.sort();
    if wilderness
        .locations
        .windows(2)
        .any(|locations| locations[0] == locations[1])
    {
        return Err(ContentError::InvalidWilderness(world_id.to_owned()));
    }
    let mut town_ids = BTreeSet::new();
    let mut location_dungeon_ids = BTreeSet::new();
    for location in &wilderness.locations {
        match location {
            WildernessLocationDefinition::Town { position, town_id } => {
                validate_definition_id(town_id, "town")?;
                if !position_is_inside(*position)
                    || !towns.contains_key(town_id)
                    || !town_ids.insert(town_id)
                {
                    return Err(ContentError::InvalidWilderness(world_id.to_owned()));
                }
            }
            WildernessLocationDefinition::Dungeon {
                position,
                dungeon_id,
            } => {
                validate_definition_id(dungeon_id, "dungeon")?;
                if !position_is_inside(*position)
                    || !dungeon_ids.contains(dungeon_id)
                    || !location_dungeon_ids.insert(dungeon_id)
                {
                    return Err(ContentError::InvalidWilderness(world_id.to_owned()));
                }
            }
        }
    }
    Ok(())
}

fn validate_task_objective(
    owner_id: &str,
    objective: &TaskObjectiveDefinition,
    floor_ids: &BTreeSet<String>,
    actor_roles: &BTreeMap<String, ActorRole>,
    item_limits: &BTreeMap<String, (u32, bool)>,
    instance_ids: &mut BTreeSet<String>,
) -> Result<(), ContentError> {
    if objective
        .floor_id
        .as_ref()
        .is_some_and(|floor_id| !floor_ids.contains(floor_id))
    {
        return Err(ContentError::InvalidTask(owner_id.to_owned()));
    }
    match objective.kind {
        TaskObjectiveKind::ClearFloor => {
            if objective.required != 1
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
                || objective.actor_instance_id.is_some()
                || objective.actor_kind_id.is_some()
            {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::CollectItem => {
            let (Some(instance_id), Some(kind_id)) =
                (&objective.item_instance_id, &objective.item_kind_id)
            else {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            };
            validate_id(instance_id)?;
            if !instance_ids.insert(instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(instance_id.clone()));
            }
            if !item_limits.contains_key(kind_id) {
                return Err(ContentError::DanglingReference {
                    owner: owner_id.to_owned(),
                    target: kind_id.clone(),
                });
            }
            if objective.required != 1
                || objective.actor_instance_id.is_some()
                || objective.actor_kind_id.is_some()
            {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::EnterFloor => {
            if objective.floor_id.is_none()
                || objective.required != 1
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
                || objective.actor_instance_id.is_some()
                || objective.actor_kind_id.is_some()
            {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::KillActor => {
            let (Some(instance_id), Some(kind_id)) =
                (&objective.actor_instance_id, &objective.actor_kind_id)
            else {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            };
            validate_id(instance_id)?;
            if !instance_ids.insert(instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(instance_id.clone()));
            }
            require_actor_role(actor_roles, kind_id, ActorRole::Monster, owner_id)?;
            if objective.required != 1
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
            {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            }
        }
        TaskObjectiveKind::KillActorKind => {
            let Some(kind_id) = &objective.actor_kind_id else {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            };
            if objective.required == 0
                || objective.actor_instance_id.is_some()
                || objective.item_instance_id.is_some()
                || objective.item_kind_id.is_some()
            {
                return Err(ContentError::InvalidTask(owner_id.to_owned()));
            }
            require_actor_role(actor_roles, kind_id, ActorRole::Monster, owner_id)?;
        }
    }
    Ok(())
}

pub(super) fn validate_world(
    world: &mut WorldDefinition,
    refs: &WorldValidationRefs<'_>,
) -> Result<(), ContentError> {
    let WorldValidationRefs {
        terrain_ids,
        terrain,
        terrain_walkability,
        terrain_tags,
        terrain_open_targets,
        terrain_traps,
        actor_roles,
        actor_levels,
        actors,
        item_limits,
        items,
        affix_ids,
        encounter_tables,
        loot_table_ids,
        loot_tables,
        theme_tables,
        region_tables,
        terrain_feature_tables,
        vaults,
        build_ids,
        towns,
        town_facilities,
        shops,
    } = refs;
    if world.width < 3 || world.height < 3 || world.width > 512 || world.height > 512 {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    if world.surface_actor_allocation.is_some_and(|allocation| {
        !(1..=64).contains(&allocation.rolls) || !(1..=100).contains(&allocation.level)
    }) {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    validate_definition_id(&world.initial_floor_id, "floor")?;
    let mut procedural_actor_ids = BTreeSet::new();
    let mut procedural_connection_ids = BTreeSet::new();
    world.procedural_floors.sort_by_key(|floor| floor.depth);
    world.dungeons.sort_by(|left, right| left.id.cmp(&right.id));
    let floor_ids = world
        .procedural_floors
        .iter()
        .map(|floor| floor.id.clone())
        .collect::<BTreeSet<_>>();
    if world.procedural_floors.is_empty()
        || floor_ids.len() != world.procedural_floors.len()
        || !world.procedural_floors.iter().any(|floor| {
            floor.lifecycle != FloorLifecycle::Town
                && floor.return_floor_id == world.initial_floor_id
        })
    {
        return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
    }
    let mut dungeon_definition_ids = BTreeSet::new();
    let mut legacy_dungeon_indices = BTreeSet::new();
    for dungeon in &mut world.dungeons {
        validate_definition_id(&dungeon.id, "dungeon")?;
        validate_definition_id(&dungeon.root_floor_id, "floor")?;
        dungeon.entry_requirements.sort();
        if dungeon
            .entry_requirements
            .windows(2)
            .any(|requirements| requirements[0] == requirements[1])
        {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
        if dungeon
            .legacy_index
            .is_some_and(|index| index == 0 || !legacy_dungeon_indices.insert(index))
            || !dungeon_definition_ids.insert(dungeon.id.clone())
            || !floor_ids.contains(&dungeon.root_floor_id)
        {
            return Err(ContentError::InvalidProceduralFloor(
                dungeon.root_floor_id.clone(),
            ));
        }
        require_actor_role(
            actor_roles,
            &dungeon.guardian_actor_kind_id,
            ActorRole::Monster,
            &dungeon.id,
        )?;
        if matches!(
            dungeon.instance_lifecycle,
            DungeonInstanceLifecycle::TurnTtl { ttl_turns: 0 }
        ) {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
        if let Some(guardian) = &dungeon.entrance_guardian {
            validate_id(&guardian.instance_id)?;
            require_actor_role(
                actor_roles,
                &guardian.actor_kind_id,
                ActorRole::Monster,
                &dungeon.id,
            )?;
            validate_position(guardian.position, world.width, world.height, &dungeon.id)?;
            if !procedural_actor_ids.insert(guardian.instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(
                    guardian.instance_id.clone(),
                ));
            }
        }
    }
    if let Some(wilderness) = &mut world.wilderness {
        validate_wilderness(&world.id, wilderness, &dungeon_definition_ids, towns)?;
    }
    let world_town_ids = world
        .wilderness
        .iter()
        .flat_map(|wilderness| &wilderness.locations)
        .filter_map(|location| match location {
            WildernessLocationDefinition::Town { town_id, .. } => Some(town_id.as_str()),
            WildernessLocationDefinition::Dungeon { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    match world.town_id.as_deref() {
        Some(town_id) if world_town_ids.contains(town_id) => {}
        None if world_town_ids.is_empty() => {}
        _ => return Err(ContentError::InvalidTown(world.id.clone())),
    }
    if let Some(campaign) = &mut world.campaign {
        campaign.victory_dungeon_ids.sort();
        if campaign.victory_dungeon_ids.is_empty()
            || campaign.turn_penalty_interval == 0
            || campaign
                .victory_dungeon_ids
                .windows(2)
                .any(|ids| ids[0] == ids[1])
            || campaign
                .victory_dungeon_ids
                .iter()
                .any(|id| !dungeon_definition_ids.contains(id))
        {
            return Err(ContentError::InvalidProceduralFloor(world.id.clone()));
        }
    }
    for procedural in &mut world.procedural_floors {
        validate_definition_id(&procedural.id, "floor")?;
        validate_message_key(&procedural.name_key)?;
        if procedural.lifecycle == FloorLifecycle::Town
            && !world_town_ids.iter().any(|town_id| {
                towns
                    .get(*town_id)
                    .is_some_and(|town| town.floor_id == procedural.id)
            })
        {
            return Err(ContentError::InvalidTown(procedural.id.clone()));
        }
        let layout_mode = procedural
            .layout
            .as_ref()
            .map_or(ProceduralLayoutMode::Rooms, |layout| layout.mode);
        let maze_only = layout_mode == ProceduralLayoutMode::MazeOnly;
        procedural
            .connections
            .sort_by(|left, right| left.id.cmp(&right.id));
        if procedural.connections.len() > 16
            || (procedural.connections.is_empty() && procedural.entry_connection_id.is_some())
            || procedural.entry_connection_id.as_ref().is_some_and(|id| {
                !procedural
                    .connections
                    .iter()
                    .any(|connection| connection.id == *id)
            })
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        for connection in &procedural.connections {
            validate_definition_id(&connection.id, "connection")?;
            if !procedural_connection_ids.insert(connection.id.clone()) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            require_reference(terrain_ids, &connection.terrain_id, &procedural.id)?;
            let tags = terrain_tags
                .get(&connection.terrain_id)
                .expect("validated connection terrain must remain available");
            if !terrain_walkability
                .get(&connection.terrain_id)
                .copied()
                .unwrap_or(false)
                || (matches!(connection.kind, FloorConnectionKind::Shaft) != tags.contains("shaft"))
                || (!tags.contains("stairs-up") && !tags.contains("stairs-down"))
                || (tags.contains("stairs-up") && tags.contains("stairs-down"))
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some(target_connection_id) = &connection.target_connection_id {
                validate_definition_id(target_connection_id, "connection")?;
            }
            for candidate in &connection.target_candidates {
                if candidate.weight == 0 {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                validate_definition_id(&candidate.target_floor_id, "floor")?;
                validate_definition_id(&candidate.target_connection_id, "connection")?;
            }
        }
        if procedural.id == world.initial_floor_id
            || (procedural.return_floor_id != world.initial_floor_id
                && !floor_ids.contains(&procedural.return_floor_id))
            || procedural
                .next_floor_id
                .as_ref()
                .is_some_and(|id| !floor_ids.contains(id))
            || procedural.next_floor_id.is_some() != procedural.down_stair_terrain_id.is_some()
            || (procedural.lifecycle == FloorLifecycle::OneShot
                && (procedural.return_floor_id != world.initial_floor_id
                    || procedural.dungeon_id.is_some()
                    || procedural.final_floor
                    || procedural.guardian.is_some()
                    || procedural.entry_terrain_id.is_none()
                    || procedural.completed_entry_terrain_id.is_none()
                    || procedural.failed_entry_terrain_id.is_none()
                    || procedural.abandoned_entry_terrain_id.is_none()
                    || procedural.next_floor_id.is_some()))
            || (procedural.lifecycle == FloorLifecycle::Dungeon
                && (procedural.dungeon_id.is_none()
                    || procedural.available_entry_terrain_id.is_some()
                    || procedural.completed_entry_terrain_id.is_some()
                    || procedural.failed_entry_terrain_id.is_some()
                    || procedural.abandoned_entry_terrain_id.is_some()
                    || !procedural.allow_early_task_exit
                    || procedural.retakeable
                    || procedural.max_retakes.is_some()
                    || procedural.retake_floor_policy != RetakeFloorPolicy::PreserveFloor
                    || procedural.task_id.is_some()))
            || (procedural.lifecycle == FloorLifecycle::Town
                && (procedural.return_floor_id != world.initial_floor_id
                    || procedural.dungeon_id.is_some()
                    || procedural.final_floor
                    || procedural.guardian.is_some()
                    || procedural.theme_id.is_some()
                    || procedural.vault_id.is_some()
                    || procedural.encounter_table_id.is_some()
                    || procedural.loot_table_id.is_some()
                    || procedural.loot_allocation.is_some()
                    || procedural.gold_allocation.is_some()
                    || !procedural.guaranteed_items.is_empty()
                    || procedural.theme_table_id.is_some()
                    || procedural.region_table_id.is_some()
                    || procedural.terrain_feature_table_id.is_some()
                    || procedural.layout.is_some()
                    || procedural.inline_map.is_none()
                    || procedural.generation_budget.is_some()
                    || procedural.nest.is_some()
                    || procedural.entry_terrain_id.is_some()
                    || procedural.available_entry_terrain_id.is_some()
                    || procedural.entry_connection_id.is_some()
                    || procedural.completed_entry_terrain_id.is_some()
                    || procedural.failed_entry_terrain_id.is_some()
                    || procedural.abandoned_entry_terrain_id.is_some()
                    || !procedural.allow_early_task_exit
                    || procedural.retakeable
                    || procedural.max_retakes.is_some()
                    || procedural.retake_floor_policy != RetakeFloorPolicy::PreserveFloor
                    || procedural.task_id.is_some()
                    || procedural.next_floor_id.is_some()
                    || !procedural.connections.is_empty()))
        {
            return Err(ContentError::InvalidWorldDimensions(world.id.clone()));
        }
        if let Some(dungeon_id) = &procedural.dungeon_id {
            validate_definition_id(dungeon_id, "dungeon")?;
        }
        if let Some(task_id) = &procedural.task_id {
            validate_definition_id(task_id, "task")?;
        }
        if (!procedural.retakeable
            && (procedural.max_retakes.is_some()
                || procedural.retake_floor_policy != RetakeFloorPolicy::PreserveFloor))
            || procedural
                .max_retakes
                .is_some_and(|maximum| maximum == 0 || maximum > 16)
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(theme_id) = &procedural.theme_id {
            validate_definition_id(theme_id, "theme")?;
        }
        if procedural.encounter_table_id.is_some() && !procedural.actor_spawns.is_empty()
            || procedural.loot_table_id.is_some() && !procedural.loot_spawns.is_empty()
            || procedural.theme_table_id.is_some()
                && (procedural.theme_id.is_some() || procedural.vault_id.is_some())
            || procedural.region_table_id.is_some()
                && (procedural.encounter_table_id.is_some()
                    || procedural.loot_table_id.is_some()
                    || procedural.theme_id.is_some()
                    || procedural.vault_id.is_some()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty()
                    || procedural.nest.is_some()
                    || maze_only
                    || procedural.generation_budget.is_none())
            || procedural.inline_map.is_some()
                && (!matches!(
                    procedural.lifecycle,
                    FloorLifecycle::OneShot | FloorLifecycle::Town
                ) || (procedural.lifecycle == FloorLifecycle::OneShot
                    && procedural.task_id.is_none())
                    || procedural.layout.is_some()
                    || procedural.generation_budget.is_some()
                    || procedural.encounter_table_id.is_some()
                    || procedural.loot_table_id.is_some()
                    || procedural.loot_allocation.is_some()
                    || procedural.gold_allocation.is_some()
                    || !procedural.guaranteed_items.is_empty()
                    || procedural.theme_table_id.is_some()
                    || procedural.region_table_id.is_some()
                    || procedural.terrain_feature_table_id.is_some()
                    || procedural.theme_id.is_some()
                    || procedural.vault_id.is_some()
                    || procedural.nest.is_some()
                    || procedural.guardian.is_some()
                    || !procedural.connections.is_empty()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty())
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        let eligible_encounter_entries = if let Some(table_id) = &procedural.encounter_table_id {
            let Some(table) = encounter_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            if table.global_allocation.is_some() {
                let has_eligible_actor = actors.iter().any(|actor| {
                    actor.role == ActorRole::Monster
                        && actor.level <= u32::from(procedural.depth)
                        && !actor.tags.iter().any(|tag| tag == "guardian")
                        && actor.allocation.as_ref().is_some_and(|allocation| {
                            !allocation.wild_only
                                && (allocation.max_depth == 0
                                    || allocation.max_depth >= procedural.depth)
                        })
                });
                if !has_eligible_actor {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                Vec::new()
            } else {
                let entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= procedural.depth
                            && procedural.depth <= entry.max_depth
                            && actor_levels
                                .get(&entry.actor_kind_id)
                                .is_some_and(|level| *level <= u32::from(procedural.depth))
                    })
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                entries
            }
        } else {
            Vec::new()
        };
        if let Some(table_id) = &procedural.loot_table_id {
            require_reference(loot_table_ids, table_id, &procedural.id)?;
            if loot_tables.get(table_id).is_some_and(|table| {
                !table.entries.iter().any(|entry| {
                    entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                })
            }) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        if let Some(allocation) = procedural.loot_allocation {
            let map_area = u32::from(procedural.width) * u32::from(procedural.height);
            let valid_rule = |rule: ProceduralNormalAllocationDefinition| {
                rule.mean > 0
                    && rule.mean <= 64
                    && rule.standard_deviation > 0
                    && rule.standard_deviation <= 16
            };
            if procedural.loot_table_id.is_none()
                || procedural.layout.as_ref().is_none_or(|layout| {
                    layout.mode != ProceduralLayoutMode::Rooms || layout.rooms.is_none()
                })
                || allocation.reference_area_tiles < map_area
                || allocation.reference_area_tiles > 1_000_000
                || !valid_rule(allocation.room_objects)
                || !valid_rule(allocation.anywhere_objects)
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        if let Some(allocation) = procedural.gold_allocation {
            let map_area = u32::from(procedural.width) * u32::from(procedural.height);
            if procedural.layout.as_ref().is_none_or(|layout| {
                layout.mode != ProceduralLayoutMode::Rooms || layout.rooms.is_none()
            }) || allocation.reference_area_tiles < map_area
                || allocation.reference_area_tiles > 1_000_000
                || allocation.piles.mean == 0
                || allocation.piles.mean > 64
                || allocation.piles.standard_deviation == 0
                || allocation.piles.standard_deviation > 16
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        procedural
            .guaranteed_items
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut guaranteed_ids = BTreeSet::new();
        for guaranteed in &procedural.guaranteed_items {
            let mut entry_ids = BTreeSet::new();
            let valid_entries = (1..=16).contains(&guaranteed.entries.len())
                && guaranteed.entries.iter().all(|entry| {
                    let valid_item = items.iter().any(|item| {
                        item.id == entry.item_kind_id
                            && (item.use_action.as_ref().is_some_and(|action| {
                                matches!(
                                    action.effect,
                                    ItemUseEffectDefinition::IncreaseNutrition { .. }
                                )
                            }) || item.fuel.is_some())
                    });
                    (1..=1_000).contains(&entry.weight)
                        && entry_ids.insert(entry.item_kind_id.as_str())
                        && valid_item
                });
            if !(2..=1_000).contains(&guaranteed.chance_one_in)
                || validate_id(&guaranteed.id).is_err()
                || !guaranteed_ids.insert(guaranteed.id.as_str())
                || !valid_entries
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        let eligible_theme_entries = if let Some(table_id) = &procedural.theme_table_id {
            let Some(table) = theme_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            let entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            if entries.is_empty() {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            entries
        } else {
            Vec::new()
        };
        let regional_groups_enabled = procedural.generation_budget.as_ref().is_some_and(|budget| {
            budget.group_placements.is_some() && budget.group_actor_slots.is_some()
        });
        let eligible_region_entries = if let Some(table_id) = &procedural.region_table_id {
            let Some(table) = region_tables.get(table_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: table_id.clone(),
                });
            };
            let entries = table
                .entries
                .iter()
                .filter(|entry| {
                    entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                })
                .collect::<Vec<_>>();
            if entries.len() < 2
                || entries.iter().any(|entry| {
                    let theme_is_valid = theme_tables
                        .get(&entry.theme_table_id)
                        .and_then(|table| {
                            table.entries.iter().find(|theme| {
                                theme.theme_id == entry.theme_id
                                    && theme.min_depth <= procedural.depth
                                    && procedural.depth <= theme.max_depth
                            })
                        })
                        .is_some_and(|theme| {
                            !theme.vault_candidates.iter().any(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                            })
                        });
                    let encounter_is_valid = encounter_tables
                        .get(&entry.encounter_table_id)
                        .is_some_and(|table| {
                            let mut eligible = table.entries.iter().filter(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                                    && actor_levels
                                        .get(&candidate.actor_kind_id)
                                        .is_some_and(|level| *level <= u32::from(procedural.depth))
                            });
                            let has_plain =
                                eligible.clone().any(|candidate| candidate.group.is_none());
                            let has_group = eligible.any(|candidate| candidate.group.is_some());
                            has_plain && (regional_groups_enabled == has_group)
                        });
                    !theme_is_valid || !encounter_is_valid
                })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            entries
        } else {
            Vec::new()
        };
        let eligible_terrain_feature_entries =
            if let Some(table_id) = &procedural.terrain_feature_table_id {
                let Some(table) = terrain_feature_tables.get(table_id) else {
                    return Err(ContentError::DanglingReference {
                        owner: procedural.id.clone(),
                        target: table_id.clone(),
                    });
                };
                let entries = table
                    .entries
                    .iter()
                    .filter(|entry| {
                        entry.min_depth <= procedural.depth && procedural.depth <= entry.max_depth
                    })
                    .collect::<Vec<_>>();
                if entries.is_empty() {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                entries
            } else {
                Vec::new()
            };
        for entry in &eligible_theme_entries {
            for candidate in entry.vault_candidates.iter().filter(|candidate| {
                candidate.min_depth <= procedural.depth && procedural.depth <= candidate.max_depth
            }) {
                let vault = vaults
                    .get(&candidate.vault_id)
                    .expect("validated theme vault must remain available");
                if vault.encounter_groups.iter().any(|group| {
                    !group.entries.iter().any(|actor| {
                        actor.min_depth <= procedural.depth
                            && procedural.depth <= actor.max_depth
                            && actor_levels
                                .get(&actor.actor_kind_id)
                                .is_some_and(|level| *level <= u32::from(procedural.depth))
                    })
                }) {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
        }
        if let Some(nest) = &procedural.nest
            && (procedural.encounter_table_id.is_none()
                || procedural.vault_id.is_some()
                || maze_only
                || !matches!(nest.room_id.as_str(), "entry" | "remote")
                || !(2..=16).contains(&nest.spawn_count)
                || eligible_theme_entries.iter().any(|entry| {
                    entry.vault_candidates.iter().any(|candidate| {
                        candidate.min_depth <= procedural.depth
                            && procedural.depth <= candidate.max_depth
                    })
                }))
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(budget) = &procedural.generation_budget {
            let reserved_actor_slots = usize::from(u8::from(procedural.guardian.is_some()))
                + procedural
                    .nest
                    .as_ref()
                    .map_or(0, |nest| usize::from(nest.spawn_count))
                + budget.pit_actor_slots.map_or(0, usize::from);
            let pit_budget = match (
                procedural
                    .layout
                    .as_ref()
                    .and_then(|layout| layout.pit.as_ref())
                    .cloned(),
                budget.pit_placements,
                budget.pit_actor_slots,
            ) {
                (None, None, None) => None,
                (Some(pit), Some(placements), Some(actor_slots)) => {
                    Some((pit, placements, actor_slots))
                }
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let room_budget = match (
                procedural.layout.as_ref(),
                budget.room_placements,
                budget.room_area_tiles,
            ) {
                (None, None, None) => None,
                (Some(layout), None, None)
                    if layout.mode == ProceduralLayoutMode::MazeOnly && layout.rooms.is_none() =>
                {
                    None
                }
                (Some(layout), Some(placements), Some(area_tiles))
                    if layout.mode == ProceduralLayoutMode::Rooms && layout.rooms.is_some() =>
                {
                    Some((placements, area_tiles))
                }
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let spatial_vault_budget = match (budget.vault_placements, budget.vault_area_tiles) {
                (None, None) => None,
                (Some(placements), Some(area_tiles)) => Some((placements, area_tiles)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let group_budget = match (budget.group_placements, budget.group_actor_slots) {
                (None, None) => None,
                (Some(placements), Some(actor_slots)) => Some((placements, actor_slots)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let feature_budget = match (
                procedural.terrain_feature_table_id.as_ref(),
                budget.feature_placements,
            ) {
                (None, None) => None,
                (Some(table_id), Some(placements)) => Some((table_id, placements)),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            let region_budget = match (
                procedural.region_table_id.as_ref(),
                budget.region_placements,
            ) {
                (None, None) => None,
                (Some(_), Some(placements)) => Some(placements),
                _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
            };
            if procedural.lifecycle != FloorLifecycle::Dungeon
                || (procedural.region_table_id.is_none()
                    && (procedural.encounter_table_id.is_none()
                        || procedural.loot_table_id.is_none()))
                || !(1..=128).contains(&budget.actor_slots)
                || !(1..=8).contains(&budget.loot_placements)
                || reserved_actor_slots >= usize::from(budget.actor_slots)
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some(placements) = region_budget {
                let room_count = budget.room_placements.unwrap_or(2);
                let regional_room_count =
                    room_count.saturating_sub(u16::from(pit_budget.is_some()));
                if !(2..=4).contains(&placements)
                    || placements > regional_room_count
                    || usize::from(placements) > eligible_region_entries.len()
                    || reserved_actor_slots + usize::from(placements)
                        > usize::from(budget.actor_slots)
                    || budget.loot_placements < placements
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            if !maze_only
                && room_budget.is_none()
                && (budget.cavern_area_tiles.is_some()
                    || budget.lake_area_tiles.is_some()
                    || budget.lake_deep_area_tiles.is_some()
                    || budget.river_area_tiles.is_some()
                    || budget.maze_floor_tiles.is_some()
                    || budget.destruction_centers.is_some()
                    || budget.destroyed_area_tiles.is_some()
                    || budget.streamer_placements.is_some()
                    || budget.streamer_area_tiles.is_some())
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if maze_only {
                let layout = procedural
                    .layout
                    .as_mut()
                    .expect("maze-only mode requires a layout");
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                if layout.rooms.is_some()
                    || layout.cavern.is_some()
                    || layout.lake.is_some()
                    || layout.river.is_some()
                    || layout.destroyed.is_some()
                    || layout.pit.is_some()
                    || layout.stairs.is_some()
                    || budget.cavern_area_tiles.is_some()
                    || budget.lake_area_tiles.is_some()
                    || budget.lake_deep_area_tiles.is_some()
                    || budget.river_area_tiles.is_some()
                    || budget.destruction_centers.is_some()
                    || budget.destroyed_area_tiles.is_some()
                    || budget.pit_placements.is_some()
                    || budget.pit_actor_slots.is_some()
                    || spatial_vault_budget.is_some()
                    || group_budget.is_some()
                    || feature_budget.is_some()
                    || procedural.vault_id.is_some()
                    || procedural.nest.is_some()
                    || procedural.guardian.is_some()
                    || !procedural.actor_spawns.is_empty()
                    || !procedural.loot_spawns.is_empty()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.maze, budget.maze_floor_tiles) {
                    (Some(maze), Some(floor_tiles)) => {
                        let vertices =
                            u32::from(maze.width.div_ceil(2)) * u32::from(maze.height.div_ceil(2));
                        let expected_floor_tiles = vertices.saturating_mul(2).saturating_sub(1);
                        if !(9..=procedural.width.saturating_sub(2)).contains(&maze.width)
                            || !(9..=procedural.height.saturating_sub(2)).contains(&maze.height)
                            || maze.width % 2 == 0
                            || maze.height % 2 == 0
                            || floor_tiles != expected_floor_tiles
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
                }
                match (
                    layout.streamers.is_empty(),
                    budget.streamer_placements,
                    budget.streamer_area_tiles,
                ) {
                    (true, None, None) => {}
                    (false, Some(placements), Some(area_tiles)) => {
                        layout
                            .streamers
                            .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
                        let terrain_count = layout
                            .streamers
                            .iter()
                            .map(|candidate| candidate.terrain_id.as_str())
                            .collect::<BTreeSet<_>>()
                            .len();
                        for candidate in &layout.streamers {
                            require_reference(terrain_ids, &candidate.terrain_id, &procedural.id)?;
                        }
                        if layout.streamers.len() > 4
                            || terrain_count != layout.streamers.len()
                            || layout.streamers.iter().any(|candidate| {
                                !(1..=1_000_000).contains(&candidate.weight)
                                    || terrain_walkability.get(&candidate.terrain_id)
                                        != Some(&false)
                                    || candidate.terrain_id == procedural.wall_terrain_id
                                    || candidate.terrain_id == procedural.floor_terrain_id
                                    || eligible_theme_entries
                                        .iter()
                                        .any(|entry| entry.floor_terrain_id == candidate.terrain_id)
                            })
                            || !(1..=4).contains(&placements)
                            || !(u32::from(placements) * 4..=interior_area.saturating_div(4))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => return Err(ContentError::InvalidProceduralFloor(procedural.id.clone())),
                }
            }
            if let Some((placements, area_tiles)) = room_budget {
                let layout = procedural
                    .layout
                    .as_mut()
                    .expect("rooms mode requires a layout");
                let geometry = layout
                    .rooms
                    .as_mut()
                    .expect("rooms mode requires room geometry");
                geometry.shapes.sort_by_key(|candidate| candidate.shape);
                let shape_count = geometry
                    .shapes
                    .iter()
                    .map(|candidate| candidate.shape)
                    .collect::<BTreeSet<_>>()
                    .len();
                let columns = if placements <= 4 { 2 } else { 3 };
                let rows = placements.div_ceil(columns);
                let minimum_cell_width = procedural.width.saturating_sub(2) / columns;
                let minimum_cell_height = procedural.height.saturating_sub(2) / rows;
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
                        ProceduralRoomShape::Cavern => {
                            cavern_room_area(geometry.min_width, geometry.min_height)
                        }
                    })
                    .min()
                    .unwrap_or(0);
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                let invalid_placement_geometry = match geometry.placement {
                    ProceduralRoomPlacement::Partitioned => {
                        geometry.min_width > minimum_cell_width
                            || geometry.min_height > minimum_cell_height
                    }
                    ProceduralRoomPlacement::Free => {
                        geometry.min_width > minimum_cell_width
                            || geometry.min_height > minimum_cell_height
                            || geometry.max_width > procedural.width.saturating_sub(2)
                            || geometry.max_height > procedural.height.saturating_sub(2)
                            || layout.pit.is_some()
                    }
                };
                if !(2..=6).contains(&placements)
                    || !(5..=32).contains(&geometry.min_width)
                    || !(geometry.min_width..=32).contains(&geometry.max_width)
                    || !(5..=32).contains(&geometry.min_height)
                    || !(geometry.min_height..=32).contains(&geometry.max_height)
                    || invalid_placement_geometry
                    || geometry.shapes.is_empty()
                    || geometry.shapes.len() > 2
                    || shape_count != geometry.shapes.len()
                    || geometry
                        .shapes
                        .iter()
                        .any(|candidate| !(1..=1_000_000).contains(&candidate.weight))
                    || area_tiles > interior_area
                    || u32::from(placements) * minimum_room_area > area_tiles
                    || procedural.vault_id.is_some()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                if let Some(stairs) = layout.stairs
                    && (!procedural.connections.is_empty()
                        || !valid_procedural_count_range(stairs.up)
                        || match (stairs.down, &procedural.down_stair_terrain_id) {
                            (Some(range), Some(_)) => !valid_procedural_count_range(range),
                            (None, None) => false,
                            _ => true,
                        })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.cavern, budget.cavern_area_tiles) {
                    (None, None) => {}
                    (Some(cavern), Some(cavern_area_tiles)) => {
                        require_reference(terrain_ids, &cavern.terrain_id, &procedural.id)?;
                        if terrain_walkability.get(&cavern.terrain_id) != Some(&true)
                            || cavern.terrain_id == procedural.floor_terrain_id
                            || cavern.terrain_id == procedural.wall_terrain_id
                            || eligible_theme_entries
                                .iter()
                                .any(|entry| entry.floor_terrain_id == cavern.terrain_id)
                            || !(16..=interior_area).contains(&cavern_area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                let validate_hydrology_terrain =
                    |deep_terrain_id: &str, shallow_terrain_id: &str| {
                        require_reference(terrain_ids, deep_terrain_id, &procedural.id)?;
                        require_reference(terrain_ids, shallow_terrain_id, &procedural.id)?;
                        if deep_terrain_id == shallow_terrain_id
                            || terrain_walkability.get(deep_terrain_id) != Some(&false)
                            || terrain_walkability.get(shallow_terrain_id) != Some(&true)
                            || [deep_terrain_id, shallow_terrain_id]
                                .contains(&procedural.floor_terrain_id.as_str())
                            || [deep_terrain_id, shallow_terrain_id]
                                .contains(&procedural.wall_terrain_id.as_str())
                            || layout.cavern.as_ref().is_some_and(|cavern| {
                                [deep_terrain_id, shallow_terrain_id]
                                    .contains(&cavern.terrain_id.as_str())
                            })
                            || eligible_theme_entries.iter().any(|entry| {
                                [deep_terrain_id, shallow_terrain_id]
                                    .contains(&entry.floor_terrain_id.as_str())
                            })
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                        Ok(())
                    };
                match (
                    &layout.lake,
                    budget.lake_area_tiles,
                    budget.lake_deep_area_tiles,
                ) {
                    (None, None, None) => {}
                    (Some(lake), Some(area_tiles), Some(deep_area_tiles)) => {
                        validate_hydrology_terrain(
                            &lake.deep_terrain_id,
                            &lake.shallow_terrain_id,
                        )?;
                        if !(24..=interior_area).contains(&area_tiles)
                            || deep_area_tiles < 4
                            || deep_area_tiles.saturating_add(8) > area_tiles
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (&layout.river, budget.river_area_tiles) {
                    (None, None) => {}
                    (Some(river), Some(area_tiles)) => {
                        validate_hydrology_terrain(
                            &river.deep_terrain_id,
                            &river.shallow_terrain_id,
                        )?;
                        let center_x = procedural.width / 2;
                        let center_y = procedural.height / 2;
                        let maximum_centerline_tiles = u32::from(
                            center_x
                                .saturating_sub(1)
                                .max(procedural.width.saturating_sub(2 + center_x))
                                + center_y
                                    .saturating_sub(1)
                                    .max(procedural.height.saturating_sub(2 + center_y))
                                + 1,
                        );
                        if !(maximum_centerline_tiles..=interior_area).contains(&area_tiles) {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                if let (Some(lake), Some(river)) = (&layout.lake, &layout.river)
                    && (lake.deep_terrain_id != river.deep_terrain_id
                        || lake.shallow_terrain_id != river.shallow_terrain_id)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                match (&layout.maze, budget.maze_floor_tiles) {
                    (None, None) => {}
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (
                    &layout.destroyed,
                    budget.destruction_centers,
                    budget.destroyed_area_tiles,
                ) {
                    (None, None, None) => {}
                    (Some(destroyed), Some(centers), Some(area_tiles)) => {
                        require_reference(terrain_ids, &destroyed.terrain_id, &procedural.id)?;
                        if terrain_walkability.get(&destroyed.terrain_id) != Some(&false)
                            || destroyed.terrain_id == procedural.wall_terrain_id
                            || destroyed.terrain_id == procedural.floor_terrain_id
                            || layout
                                .cavern
                                .as_ref()
                                .is_some_and(|cavern| cavern.terrain_id == destroyed.terrain_id)
                            || layout.lake.as_ref().is_some_and(|lake| {
                                [
                                    lake.deep_terrain_id.as_str(),
                                    lake.shallow_terrain_id.as_str(),
                                ]
                                .contains(&destroyed.terrain_id.as_str())
                            })
                            || eligible_theme_entries
                                .iter()
                                .any(|entry| entry.floor_terrain_id == destroyed.terrain_id)
                            || !(1..=4).contains(&centers)
                            || !(u32::from(centers) * 8..=interior_area.saturating_div(2))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                match (
                    layout.streamers.is_empty(),
                    budget.streamer_placements,
                    budget.streamer_area_tiles,
                ) {
                    (true, None, None) => {}
                    (false, Some(placements), Some(area_tiles)) => {
                        layout
                            .streamers
                            .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
                        let terrain_count = layout
                            .streamers
                            .iter()
                            .map(|candidate| candidate.terrain_id.as_str())
                            .collect::<BTreeSet<_>>()
                            .len();
                        for candidate in &layout.streamers {
                            require_reference(terrain_ids, &candidate.terrain_id, &procedural.id)?;
                        }
                        if layout.streamers.len() > 4
                            || terrain_count != layout.streamers.len()
                            || layout.streamers.iter().any(|candidate| {
                                !(1..=1_000_000).contains(&candidate.weight)
                                    || terrain_walkability.get(&candidate.terrain_id)
                                        != Some(&false)
                                    || candidate.terrain_id == procedural.wall_terrain_id
                                    || candidate.terrain_id == procedural.floor_terrain_id
                                    || layout.destroyed.as_ref().is_some_and(|destroyed| {
                                        destroyed.terrain_id == candidate.terrain_id
                                    })
                                    || layout.cavern.as_ref().is_some_and(|cavern| {
                                        cavern.terrain_id == candidate.terrain_id
                                    })
                                    || layout.lake.as_ref().is_some_and(|lake| {
                                        [
                                            lake.deep_terrain_id.as_str(),
                                            lake.shallow_terrain_id.as_str(),
                                        ]
                                        .contains(&candidate.terrain_id.as_str())
                                    })
                                    || eligible_theme_entries
                                        .iter()
                                        .any(|entry| entry.floor_terrain_id == candidate.terrain_id)
                            })
                            || !(1..=4).contains(&placements)
                            || !(u32::from(placements) * 4..=interior_area.saturating_div(4))
                                .contains(&area_tiles)
                        {
                            return Err(ContentError::InvalidProceduralFloor(
                                procedural.id.clone(),
                            ));
                        }
                    }
                    _ => {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                if let Some((pit, placements, actor_slots)) = &pit_budget {
                    let Some(table) = encounter_tables.get(&pit.encounter_table_id) else {
                        return Err(ContentError::DanglingReference {
                            owner: procedural.id.clone(),
                            target: pit.encounter_table_id.clone(),
                        });
                    };
                    let eligible_pit_entries = table
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.min_depth <= procedural.depth
                                && procedural.depth <= entry.max_depth
                                && actor_levels
                                    .get(&entry.actor_kind_id)
                                    .is_some_and(|level| *level <= u32::from(procedural.depth))
                        })
                        .count();
                    let total_width = pit.inner_width.saturating_add(6);
                    let total_height = pit.inner_height.saturating_add(6);
                    if *placements != 1
                        || *actor_slots != pit.inner_width.saturating_mul(pit.inner_height)
                        || !(5..=15).contains(&pit.inner_width)
                        || !(5..=7).contains(&pit.inner_height)
                        || pit.inner_width % 2 == 0
                        || pit.inner_height % 2 == 0
                        || !(2..=10).contains(&pit.roster_size)
                        || eligible_pit_entries < 2
                        || total_width > procedural.width.saturating_sub(2)
                        || total_height > procedural.height.saturating_sub(2)
                        || procedural.nest.is_some()
                        || spatial_vault_budget.is_some()
                        || group_budget.is_some()
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
            }
            if let Some((placements, area_tiles)) = spatial_vault_budget {
                let interior_area = u32::from(procedural.width.saturating_sub(2))
                    * u32::from(procedural.height.saturating_sub(2));
                if !(1..=4).contains(&placements)
                    || !(4..=512).contains(&area_tiles)
                    || area_tiles > interior_area
                    || procedural.theme_table_id.is_none()
                    || procedural.vault_id.is_some()
                    || procedural.nest.is_some()
                    || eligible_theme_entries.is_empty()
                    || eligible_theme_entries.iter().any(|entry| {
                        entry
                            .vault_candidates
                            .iter()
                            .filter(|candidate| {
                                candidate.min_depth <= procedural.depth
                                    && procedural.depth <= candidate.max_depth
                            })
                            .count()
                            < usize::from(placements)
                    })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            if let Some((placements, group_actor_slots)) = group_budget {
                let group_source_entries = if procedural.region_table_id.is_some() {
                    eligible_region_entries
                        .iter()
                        .flat_map(|region| {
                            encounter_tables[&region.encounter_table_id]
                                .entries
                                .iter()
                                .filter(|entry| {
                                    entry.min_depth <= procedural.depth
                                        && procedural.depth <= entry.max_depth
                                        && actor_levels.get(&entry.actor_kind_id).is_some_and(
                                            |level| *level <= u32::from(procedural.depth),
                                        )
                                })
                        })
                        .collect::<Vec<_>>()
                } else {
                    eligible_encounter_entries.clone()
                };
                let grouped_entries = group_source_entries
                    .iter()
                    .filter(|entry| entry.group.is_some())
                    .copied()
                    .collect::<Vec<_>>();
                let plain_entries = group_source_entries
                    .iter()
                    .filter(|entry| entry.group.is_none())
                    .copied()
                    .collect::<Vec<_>>();
                let maximum_minimum_companions = grouped_entries
                    .iter()
                    .filter_map(|entry| entry.group.as_ref())
                    .map(EncounterGroupDefinition::min_companion_count)
                    .max()
                    .unwrap_or(0);
                let required_companion_slots =
                    usize::from(placements) * usize::from(maximum_minimum_companions);
                let ordinary_actor_reserve = region_budget.map_or(1, usize::from);
                let required_actor_slots = reserved_actor_slots
                    + usize::from(placements)
                    + required_companion_slots
                    + ordinary_actor_reserve;
                let encounter_rolls = if procedural.region_table_id.is_some() {
                    eligible_region_entries
                        .iter()
                        .map(|region| encounter_tables[&region.encounter_table_id].rolls)
                        .min()
                        .unwrap_or(0)
                } else {
                    procedural
                        .encounter_table_id
                        .as_ref()
                        .and_then(|table_id| encounter_tables.get(table_id))
                        .map_or(0, |table| table.rolls)
                };
                if !(1..=4).contains(&placements)
                    || !(1..=14).contains(&group_actor_slots)
                    || placements >= encounter_rolls
                    || grouped_entries.is_empty()
                    || plain_entries.is_empty()
                    || procedural.nest.is_some()
                    || spatial_vault_budget.is_some()
                    || required_companion_slots > usize::from(group_actor_slots)
                    || required_actor_slots > usize::from(budget.actor_slots)
                    || grouped_entries.iter().any(|entry| {
                        entry
                            .group
                            .as_ref()
                            .is_some_and(|group| group.min_companion_count() > group_actor_slots)
                    })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            } else if eligible_encounter_entries
                .iter()
                .any(|entry| entry.group.is_some())
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some((table_id, placements)) = feature_budget {
                let table = terrain_feature_tables
                    .get(table_id)
                    .expect("validated terrain feature table must remain available");
                if !(1..=8).contains(&placements)
                    || placements > table.rolls
                    || eligible_terrain_feature_entries.is_empty()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            for entry in &eligible_theme_entries {
                for candidate in entry.vault_candidates.iter().filter(|candidate| {
                    candidate.min_depth <= procedural.depth
                        && procedural.depth <= candidate.max_depth
                }) {
                    let vault = vaults
                        .get(&candidate.vault_id)
                        .expect("validated theme vault must remain available");
                    let vault_actor_slots = vault
                        .encounter_groups
                        .iter()
                        .map(|group| group.member_positions.len())
                        .sum::<usize>();
                    let ordinary_reserve = region_budget.map_or(1, usize::from);
                    if reserved_actor_slots + vault_actor_slots + ordinary_reserve
                        > usize::from(budget.actor_slots)
                        || vault.loot_spawns.len() + ordinary_reserve
                            > usize::from(budget.loot_placements)
                        || spatial_vault_budget.is_some_and(|(_, area_tiles)| {
                            u32::from(vault.width) * u32::from(vault.height) > area_tiles
                        })
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
            }
        } else if eligible_encounter_entries
            .iter()
            .any(|entry| entry.group.is_some())
            || procedural.terrain_feature_table_id.is_some()
            || procedural.layout.is_some()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(vault_id) = &procedural.vault_id {
            let Some(vault) = vaults.get(vault_id) else {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: vault_id.clone(),
                });
            };
            if procedural.theme_id.as_ref() != Some(&vault.theme_id)
                || procedural
                    .actor_spawns
                    .iter()
                    .any(|spawn| spawn.room_id == "remote")
                || procedural
                    .loot_spawns
                    .iter()
                    .any(|spawn| spawn.room_id == "remote")
                || vault.encounter_groups.iter().any(|group| {
                    !group.entries.iter().any(|entry| {
                        entry.min_depth <= procedural.depth
                            && procedural.depth <= entry.max_depth
                            && actor_levels
                                .get(&entry.actor_kind_id)
                                .is_some_and(|level| *level <= u32::from(procedural.depth))
                    })
                })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        for terrain_id in [
            Some(&procedural.wall_terrain_id),
            Some(&procedural.floor_terrain_id),
            Some(&procedural.up_stair_terrain_id),
            Some(&procedural.closed_door_terrain_id),
            Some(&procedural.trap_terrain_id),
            procedural.down_stair_terrain_id.as_ref(),
            procedural.entry_terrain_id.as_ref(),
            procedural.available_entry_terrain_id.as_ref(),
            procedural.completed_entry_terrain_id.as_ref(),
            procedural.failed_entry_terrain_id.as_ref(),
            procedural.abandoned_entry_terrain_id.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            require_reference(terrain_ids, terrain_id, &procedural.id)?;
        }
        if let Some(guardian) = &procedural.guardian {
            validate_id(&guardian.instance_id)?;
            if !procedural_actor_ids.insert(guardian.instance_id.clone()) {
                return Err(ContentError::DuplicateInstanceId(
                    guardian.instance_id.clone(),
                ));
            }
            require_actor_role(
                actor_roles,
                &guardian.actor_kind_id,
                ActorRole::Monster,
                &procedural.id,
            )?;
            if actor_levels
                .get(&guardian.actor_kind_id)
                .is_none_or(|level| *level > u32::from(procedural.depth))
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            if let Some(table_id) = &guardian.reward_loot_table_id {
                require_reference(loot_table_ids, table_id, &procedural.id)?;
            }
        }
        if terrain_walkability
            .get(&procedural.wall_terrain_id)
            .copied()
            .unwrap_or(true)
            || !terrain_walkability
                .get(&procedural.floor_terrain_id)
                .copied()
                .unwrap_or(false)
            || !terrain_walkability
                .get(&procedural.up_stair_terrain_id)
                .copied()
                .unwrap_or(false)
            || procedural
                .down_stair_terrain_id
                .as_ref()
                .is_some_and(|id| !terrain_walkability.get(id).copied().unwrap_or(false))
            || terrain_walkability
                .get(&procedural.closed_door_terrain_id)
                .copied()
                .unwrap_or(true)
            || !terrain_open_targets.contains_key(&procedural.closed_door_terrain_id)
            || !terrain_traps.contains(&procedural.trap_terrain_id)
            || (procedural.lifecycle != FloorLifecycle::Town && procedural.depth == 0)
            || (procedural.lifecycle == FloorLifecycle::Town && procedural.depth != 0)
            || procedural.depth > 1_000
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(inline_map) = &mut procedural.inline_map {
            validate_position(
                inline_map.player_position,
                procedural.width,
                procedural.height,
                &procedural.id,
            )?;
            inline_map.terrain_overrides.sort_by(|left, right| {
                left.terrain_id
                    .cmp(&right.terrain_id)
                    .then_with(|| left.positions.first().cmp(&right.positions.first()))
            });
            let mut painted_positions = BTreeSet::new();
            let mut painted_terrain = BTreeMap::new();
            for terrain_override in &mut inline_map.terrain_overrides {
                require_reference(terrain_ids, &terrain_override.terrain_id, &procedural.id)?;
                if terrain_override.chance_percent == 0
                    || terrain_override.chance_percent > 100
                    || (terrain_override.chance_percent < 100)
                        != terrain_override.otherwise_terrain_id.is_some()
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                if let Some(otherwise_terrain_id) = &terrain_override.otherwise_terrain_id {
                    require_reference(terrain_ids, otherwise_terrain_id, &procedural.id)?;
                    if terrain_walkability.get(&terrain_override.terrain_id)
                        != terrain_walkability.get(otherwise_terrain_id)
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                terrain_override.positions.sort();
                terrain_override.positions.dedup();
                if terrain_override.positions.is_empty() {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                for position in &terrain_override.positions {
                    validate_position(
                        *position,
                        procedural.width,
                        procedural.height,
                        &procedural.id,
                    )?;
                    if !painted_positions.insert(*position) {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                    painted_terrain.insert(*position, terrain_override.terrain_id.as_str());
                }
            }
            let terrain_at = |position: ContentPosition| {
                painted_terrain
                    .get(&position)
                    .copied()
                    .unwrap_or(procedural.wall_terrain_id.as_str())
            };
            if !terrain_walkability
                .get(terrain_at(inline_map.player_position))
                .copied()
                .unwrap_or(false)
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }

            inline_map
                .actor_spawns
                .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
            let mut occupied = BTreeSet::from([inline_map.player_position]);
            for spawn in &inline_map.actor_spawns {
                validate_id(&spawn.instance_id)?;
                validate_position(
                    spawn.position,
                    procedural.width,
                    procedural.height,
                    &procedural.id,
                )?;
                if !procedural_actor_ids.insert(spawn.instance_id.clone())
                    || !occupied.insert(spawn.position)
                    || !terrain_walkability
                        .get(terrain_at(spawn.position))
                        .copied()
                        .unwrap_or(false)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                require_actor_role(
                    actor_roles,
                    &spawn.kind_id,
                    ActorRole::Monster,
                    &procedural.id,
                )?;
            }

            inline_map
                .loot_spawns
                .sort_by(|left, right| left.id.cmp(&right.id));
            let mut inline_loot_ids = BTreeSet::new();
            for spawn in &inline_map.loot_spawns {
                validate_id(&spawn.id)?;
                validate_position(
                    spawn.position,
                    procedural.width,
                    procedural.height,
                    &procedural.id,
                )?;
                if !inline_loot_ids.insert(spawn.id.clone())
                    || !occupied.insert(spawn.position)
                    || !terrain_walkability
                        .get(terrain_at(spawn.position))
                        .copied()
                        .unwrap_or(false)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                require_reference(loot_table_ids, &spawn.loot_table_id, &procedural.id)?;
            }

            if let Some(formation) = &mut inline_map.monster_formation {
                formation.candidate_actor_kind_ids.sort();
                formation.candidate_actor_kind_ids.dedup();
                if formation.candidate_actor_kind_ids.is_empty()
                    || formation.candidate_actor_kind_ids.len() > 64
                    || formation.draw_count == 0
                    || formation.draw_count > 32
                    || formation.placement_indices.len() != formation.positions.len()
                    || formation.positions.is_empty()
                    || formation.positions.len() > usize::from(formation.draw_count)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                for actor_kind_id in &formation.candidate_actor_kind_ids {
                    require_actor_role(
                        actor_roles,
                        actor_kind_id,
                        ActorRole::Monster,
                        &procedural.id,
                    )?;
                    if actors
                        .iter()
                        .find(|actor| actor.id == *actor_kind_id)
                        .is_none_or(|actor| {
                            actor.allocation.is_none()
                                || actor.level > u32::from(procedural.depth).saturating_add(5)
                                || actor.tags.iter().any(|tag| tag == "unique")
                        })
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
                let mut indices = BTreeSet::new();
                for (index, position) in formation
                    .placement_indices
                    .iter()
                    .copied()
                    .zip(formation.positions.iter().copied())
                {
                    validate_position(
                        position,
                        procedural.width,
                        procedural.height,
                        &procedural.id,
                    )?;
                    if index >= formation.draw_count
                        || !indices.insert(index)
                        || !occupied.insert(position)
                        || !terrain_walkability
                            .get(terrain_at(position))
                            .copied()
                            .unwrap_or(false)
                    {
                        return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                    }
                }
            }
        }
        procedural
            .actor_spawns
            .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        let mut room_spawn_counts = BTreeMap::new();
        for spawn in &mut procedural.actor_spawns {
            validate_id(&spawn.instance_id)?;
            if !procedural_actor_ids.insert(spawn.instance_id.clone())
                || !matches!(spawn.room_id.as_str(), "entry" | "remote")
                || spawn.actor_kind_ids.is_empty()
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            *room_spawn_counts.entry(spawn.room_id.clone()).or_insert(0) += 1;
            spawn.actor_kind_ids.sort();
            for actor_kind_id in &spawn.actor_kind_ids {
                require_actor_role(
                    actor_roles,
                    actor_kind_id,
                    ActorRole::Monster,
                    &procedural.id,
                )?;
            }
            if !spawn.actor_kind_ids.iter().any(|actor_kind_id| {
                actor_levels
                    .get(actor_kind_id)
                    .is_some_and(|level| *level <= u32::from(procedural.depth))
            }) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
        procedural
            .loot_spawns
            .sort_by(|left, right| left.id.cmp(&right.id));
        let mut loot_ids = BTreeSet::new();
        for spawn in &procedural.loot_spawns {
            validate_id(&spawn.id)?;
            if !loot_ids.insert(spawn.id.clone()) {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
            *room_spawn_counts.entry(spawn.room_id.clone()).or_insert(0) += 1;
            require_reference(loot_table_ids, &spawn.loot_table_id, &procedural.id)?;
        }
    }
    for procedural in &world.procedural_floors {
        if procedural.connections.is_empty() {
            continue;
        }
        if procedural.return_floor_id == world.initial_floor_id
            && procedural
                .entry_connection_id
                .as_ref()
                .and_then(|id| {
                    procedural
                        .connections
                        .iter()
                        .find(|connection| connection.id == *id)
                })
                .is_none_or(|connection| connection.target_floor_id != world.initial_floor_id)
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if procedural.return_floor_id != world.initial_floor_id
            && procedural.entry_connection_id.is_some()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        for connection in &procedural.connections {
            for candidate in &connection.target_candidates {
                if candidate.target_floor_id == world.initial_floor_id
                    || !floor_ids.contains(&candidate.target_floor_id)
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                let target = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == candidate.target_floor_id)
                    .expect("validated dynamic connection target must remain available");
                let Some(target_connection) = target.connections.iter().find(|target_connection| {
                    target_connection.id == candidate.target_connection_id
                }) else {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                };
                let depth_delta = target.depth.abs_diff(procedural.depth);
                if target_connection.kind != connection.kind
                    || (matches!(connection.kind, FloorConnectionKind::Stairs) && depth_delta != 1)
                    || (matches!(connection.kind, FloorConnectionKind::Shaft) && depth_delta != 2)
                    || (target.lifecycle != procedural.lifecycle)
                    || (target.dungeon_id != procedural.dungeon_id)
                    || !terrain_tags
                        .get(&connection.terrain_id)
                        .is_some_and(|tags| {
                            if target.depth > procedural.depth {
                                tags.contains("stairs-down")
                            } else {
                                tags.contains("stairs-up")
                            }
                        })
                    || !terrain_tags
                        .get(&target_connection.terrain_id)
                        .is_some_and(|tags| {
                            if target.depth > procedural.depth {
                                tags.contains("stairs-up")
                            } else {
                                tags.contains("stairs-down")
                            }
                        })
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
            }
            if !floor_ids.contains(&connection.target_floor_id)
                && connection.target_floor_id != world.initial_floor_id
            {
                return Err(ContentError::DanglingReference {
                    owner: procedural.id.clone(),
                    target: connection.target_floor_id.clone(),
                });
            }
            if connection.target_floor_id == world.initial_floor_id {
                if connection.target_connection_id.is_some()
                    || !matches!(connection.kind, FloorConnectionKind::Stairs)
                    || !terrain_tags
                        .get(&connection.terrain_id)
                        .is_some_and(|tags| tags.contains("stairs-up"))
                {
                    return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
                }
                continue;
            }
            let target = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == connection.target_floor_id)
                .expect("validated connection target must remain available");
            let Some(target_connection_id) = connection.target_connection_id.as_ref() else {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            };
            let Some(target_connection) = target
                .connections
                .iter()
                .find(|candidate| candidate.id == *target_connection_id)
            else {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            };
            let depth_delta = target.depth.abs_diff(procedural.depth);
            if target_connection.target_floor_id != procedural.id
                || target_connection.target_connection_id.as_deref() != Some(connection.id.as_str())
                || target_connection.kind != connection.kind
                || (matches!(connection.kind, FloorConnectionKind::Stairs) && depth_delta != 1)
                || (matches!(connection.kind, FloorConnectionKind::Shaft) && depth_delta != 2)
                || (target.lifecycle != procedural.lifecycle)
                || (target.dungeon_id != procedural.dungeon_id)
                || !terrain_tags
                    .get(&connection.terrain_id)
                    .is_some_and(|tags| {
                        if target.depth > procedural.depth {
                            tags.contains("stairs-down")
                        } else {
                            tags.contains("stairs-up")
                        }
                    })
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
    }
    world.tasks.sort_by(|left, right| left.id.cmp(&right.id));
    let task_ids = world
        .tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<BTreeSet<_>>();
    if task_ids.len() != world.tasks.len() {
        return Err(ContentError::InvalidTask(world.id.clone()));
    }
    for task in &mut world.tasks {
        validate_definition_id(&task.id, "task")?;
        validate_message_key(&task.name_key)?;
        validate_message_key(&task.description_key)?;
        if task.objectives.is_empty() {
            return Err(ContentError::InvalidTask(task.id.clone()));
        }
        if let Some(prerequisite_id) = &task.prerequisite_task_id {
            validate_definition_id(prerequisite_id, "task")?;
            if prerequisite_id == &task.id || !task_ids.contains(prerequisite_id) {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }
        if let Some(facility_id) = &task.source_facility_id {
            validate_definition_id(facility_id, "town-facility")?;
            let facility = town_facilities
                .get(facility_id)
                .ok_or_else(|| ContentError::InvalidTask(task.id.clone()))?;
            if facility.category != TownFacilityCategory::QuestGiver
                || !world_town_ids.contains(facility.town_id.as_str())
                || !facility.task_ids.contains(&task.id)
            {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }
        let dungeon_depth_location =
            matches!(&task.location, TaskLocationDefinition::DungeonDepth { .. });
        let location_floor_ids = match &mut task.location {
            TaskLocationDefinition::DedicatedFloors { floor_ids } => {
                floor_ids.sort();
                if floor_ids.is_empty() || floor_ids.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(ContentError::InvalidTask(task.id.clone()));
                }
                let mut retake_settings = None;
                for floor_id in floor_ids.iter() {
                    validate_definition_id(floor_id, "floor")?;
                    let floor = world
                        .procedural_floors
                        .iter()
                        .find(|floor| floor.id == *floor_id)
                        .ok_or_else(|| ContentError::InvalidTask(task.id.clone()))?;
                    if floor.lifecycle != FloorLifecycle::OneShot
                        || floor.task_id.as_deref() != Some(task.id.as_str())
                        || (task.source_facility_id.is_some()
                            != floor.available_entry_terrain_id.is_some())
                    {
                        return Err(ContentError::InvalidTask(task.id.clone()));
                    }
                    let settings = (
                        floor.retakeable,
                        floor.max_retakes,
                        floor.retake_floor_policy,
                    );
                    if retake_settings
                        .replace(settings)
                        .is_some_and(|value| value != settings)
                    {
                        return Err(ContentError::InvalidTask(task.id.clone()));
                    }
                }
                floor_ids
                    .iter()
                    .map(String::as_str)
                    .collect::<BTreeSet<_>>()
            }
            TaskLocationDefinition::DungeonDepth { dungeon_id, depth } => {
                validate_definition_id(dungeon_id, "dungeon")?;
                let members = world
                    .procedural_floors
                    .iter()
                    .filter(|floor| {
                        floor.lifecycle == FloorLifecycle::Dungeon
                            && floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
                            && floor.depth == *depth
                    })
                    .map(|floor| floor.id.as_str())
                    .collect::<BTreeSet<_>>();
                if *depth == 0 || members.is_empty() {
                    return Err(ContentError::InvalidTask(task.id.clone()));
                }
                members
            }
        };

        if let Some(terrain_id) = &task.completion_exit_terrain_id {
            require_reference(terrain_ids, terrain_id, &task.id)?;
            if task.source_facility_id.is_none()
                || !dungeon_depth_location
                || terrain_tags
                    .get(terrain_id)
                    .is_none_or(|tags| !tags.contains("stairs-down"))
                || world
                    .procedural_floors
                    .iter()
                    .filter(|floor| location_floor_ids.contains(floor.id.as_str()))
                    .any(|floor| {
                        !floor.connections.is_empty()
                            || floor.down_stair_terrain_id.as_deref() != Some(terrain_id.as_str())
                    })
            {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }

        for objective in &task.objectives {
            validate_task_objective(
                &task.id,
                objective,
                &floor_ids,
                actor_roles,
                item_limits,
                &mut procedural_actor_ids,
            )?;
            if objective
                .floor_id
                .as_deref()
                .is_some_and(|floor_id| !location_floor_ids.contains(floor_id))
            {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }

        task.target_placements.sort_by(|left, right| {
            (left.objective_index, left.floor_id.as_str())
                .cmp(&(right.objective_index, right.floor_id.as_str()))
        });
        if task.target_placements.windows(2).any(|pair| {
            pair[0].objective_index == pair[1].objective_index
                && pair[0].floor_id == pair[1].floor_id
        }) {
            return Err(ContentError::InvalidTask(task.id.clone()));
        }
        for placement in &task.target_placements {
            let objective = usize::try_from(placement.objective_index)
                .ok()
                .and_then(|index| task.objectives.get(index))
                .ok_or_else(|| ContentError::InvalidTask(task.id.clone()))?;
            if !location_floor_ids.contains(placement.floor_id.as_str())
                || objective
                    .floor_id
                    .as_deref()
                    .is_some_and(|floor_id| floor_id != placement.floor_id)
                || placement.spawn_count == 0
                || !matches!(
                    objective.kind,
                    TaskObjectiveKind::KillActor | TaskObjectiveKind::KillActorKind
                )
                || objective.kind == TaskObjectiveKind::KillActor && placement.spawn_count != 1
                || objective.kind == TaskObjectiveKind::KillActorKind
                    && placement.spawn_count > objective.required
            {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }
        for (index, objective) in task.objectives.iter().enumerate() {
            let placements = task
                .target_placements
                .iter()
                .filter(|placement| placement.objective_index as usize == index)
                .count();
            if (objective.kind == TaskObjectiveKind::KillActor && placements != 1)
                || (placements > 0
                    && !matches!(
                        objective.kind,
                        TaskObjectiveKind::KillActor | TaskObjectiveKind::KillActorKind
                    ))
                || (objective.floor_id.is_none() && location_floor_ids.len() > 1 && placements == 0)
            {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
        }

        validate_id(&task.reward.item_instance_id)?;
        if !procedural_actor_ids.insert(task.reward.item_instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(
                task.reward.item_instance_id.clone(),
            ));
        }
        let (max_stack, _) = item_limits.get(&task.reward.item_kind_id).ok_or_else(|| {
            ContentError::DanglingReference {
                owner: task.id.clone(),
                target: task.reward.item_kind_id.clone(),
            }
        })?;
        if task.reward.quantity == 0 || task.reward.quantity > *max_stack {
            return Err(ContentError::InvalidItemQuantity(
                task.reward.item_instance_id.clone(),
            ));
        }
    }
    let task_floor_ids = |task: &TaskDefinition| -> BTreeSet<String> {
        match &task.location {
            TaskLocationDefinition::DedicatedFloors { floor_ids } => {
                floor_ids.iter().cloned().collect()
            }
            TaskLocationDefinition::DungeonDepth { dungeon_id, depth } => world
                .procedural_floors
                .iter()
                .filter(|floor| {
                    floor.dungeon_id.as_deref() == Some(dungeon_id.as_str())
                        && floor.depth == *depth
                })
                .map(|floor| floor.id.clone())
                .collect(),
        }
    };
    let task_depends_on = |candidate: &TaskDefinition, ancestor_id: &str| {
        let mut cursor = candidate.prerequisite_task_id.as_deref();
        while let Some(task_id) = cursor {
            if task_id == ancestor_id {
                return true;
            }
            cursor = world
                .tasks
                .iter()
                .find(|task| task.id == task_id)
                .and_then(|task| task.prerequisite_task_id.as_deref());
        }
        false
    };
    for (left_index, left) in world.tasks.iter().enumerate() {
        let left_floors = task_floor_ids(left);
        for right in world.tasks.iter().skip(left_index + 1) {
            if left_floors.is_disjoint(&task_floor_ids(right))
                || task_depends_on(left, &right.id)
                || task_depends_on(right, &left.id)
            {
                continue;
            }
            return Err(ContentError::InvalidTask(left.id.clone()));
        }
    }
    for task in &world.tasks {
        let mut seen = BTreeSet::new();
        let mut cursor = Some(task.id.as_str());
        while let Some(task_id) = cursor {
            if !seen.insert(task_id) {
                return Err(ContentError::InvalidTask(task.id.clone()));
            }
            cursor = world
                .tasks
                .iter()
                .find(|candidate| candidate.id == task_id)
                .and_then(|candidate| candidate.prerequisite_task_id.as_deref());
        }
    }
    for floor in world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::OneShot)
    {
        let Some(task_id) = floor.task_id.as_deref() else {
            return Err(ContentError::InvalidTask(floor.id.clone()));
        };
        if !world.tasks.iter().any(|task| {
            task.id == task_id
                && matches!(
                    &task.location,
                    TaskLocationDefinition::DedicatedFloors { floor_ids }
                        if floor_ids.contains(&floor.id)
                )
        }) {
            return Err(ContentError::InvalidTask(task_id.to_owned()));
        }
    }
    for town_id in &world_town_ids {
        for facility in town_facilities.values().filter(|facility| {
            facility.town_id == *town_id && facility.category == TownFacilityCategory::QuestGiver
        }) {
            if facility.task_ids.iter().any(|task_id| {
                world
                    .tasks
                    .iter()
                    .find(|task| task.id == *task_id)
                    .is_none_or(|task| {
                        task.source_facility_id.as_deref() != Some(facility.id.as_str())
                    })
            }) {
                return Err(ContentError::InvalidTownFacility(facility.id.clone()));
            }
        }
    }
    for procedural in &world.procedural_floors {
        if procedural.lifecycle != FloorLifecycle::Town
            && procedural.return_floor_id == world.initial_floor_id
            && procedural.entry_terrain_id.is_none()
        {
            return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
        }
        if let Some(next_id) = &procedural.next_floor_id {
            let next = world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == *next_id)
                .expect("validated next floor must remain available");
            if next.return_floor_id != procedural.id
                || next.depth != procedural.depth.saturating_add(1)
                || next.lifecycle != procedural.lifecycle
                || next.dungeon_id != procedural.dungeon_id
            {
                return Err(ContentError::InvalidProceduralFloor(procedural.id.clone()));
            }
        }
    }
    let dungeon_ids = world
        .procedural_floors
        .iter()
        .filter(|floor| floor.lifecycle == FloorLifecycle::Dungeon)
        .filter_map(|floor| floor.dungeon_id.as_deref())
        .collect::<BTreeSet<_>>();
    if dungeon_ids.len() != world.dungeons.len()
        || dungeon_ids
            .iter()
            .any(|dungeon_id| !dungeon_definition_ids.contains(*dungeon_id))
    {
        return Err(ContentError::InvalidProceduralFloor(world.id.clone()));
    }
    for dungeon_id in dungeon_ids {
        let dungeon = world
            .dungeons
            .iter()
            .find(|definition| definition.id == dungeon_id)
            .expect("validated dungeon definition must remain available");
        let members = world
            .procedural_floors
            .iter()
            .filter(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id))
            .collect::<Vec<_>>();
        let roots = members
            .iter()
            .filter(|floor| floor.return_floor_id == world.initial_floor_id)
            .copied()
            .collect::<Vec<_>>();
        let Some(root) = members
            .iter()
            .find(|floor| floor.id == dungeon.root_floor_id)
            .copied()
        else {
            return Err(ContentError::InvalidProceduralFloor(members[0].id.clone()));
        };
        if roots.len() != 1 || roots[0].id != root.id || root.depth != 1 {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }

        let member_ids = members
            .iter()
            .map(|floor| floor.id.as_str())
            .collect::<BTreeSet<_>>();
        let mut children_by_floor = BTreeMap::<&str, Vec<&str>>::new();
        let mut final_count = 0usize;
        for floor in &members {
            let mut parents = if floor.connections.is_empty() {
                (floor.return_floor_id != world.initial_floor_id)
                    .then_some(floor.return_floor_id.as_str())
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                floor
                    .connections
                    .iter()
                    .filter_map(|connection| {
                        let target = members
                            .iter()
                            .find(|candidate| candidate.id == connection.target_floor_id)?;
                        (target.depth < floor.depth).then_some(target.id.as_str())
                    })
                    .collect::<Vec<_>>()
            };
            parents.sort_unstable();
            parents.dedup();
            if (floor.id == root.id && !parents.is_empty())
                || (floor.id != root.id
                    && (parents.len() != 1 || floor.return_floor_id != parents[0]))
            {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }

            let mut children = if floor.connections.is_empty() {
                floor
                    .next_floor_id
                    .as_deref()
                    .into_iter()
                    .collect::<Vec<_>>()
            } else {
                floor
                    .connections
                    .iter()
                    .filter_map(|connection| {
                        let target = members
                            .iter()
                            .find(|candidate| candidate.id == connection.target_floor_id)?;
                        (target.depth > floor.depth).then_some(target.id.as_str())
                    })
                    .collect::<Vec<_>>()
            };
            let child_count = children.len();
            children.sort_unstable();
            children.dedup();
            if children.len() != child_count
                || children.iter().any(|child| !member_ids.contains(child))
            {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }
            let is_leaf = children.is_empty();
            if floor.final_floor != is_leaf || floor.guardian.is_some() != is_leaf {
                return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
            }
            if let Some(guardian) = &floor.guardian {
                final_count += 1;
                if guardian.actor_kind_id != dungeon.guardian_actor_kind_id {
                    return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
                }
            }
            children_by_floor.insert(floor.id.as_str(), children);
        }
        if final_count == 0 {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }

        let mut pending = vec![root.id.as_str()];
        let mut seen = BTreeSet::new();
        while let Some(floor_id) = pending.pop() {
            if !seen.insert(floor_id) {
                return Err(ContentError::InvalidProceduralFloor(floor_id.to_owned()));
            }
            pending.extend(
                children_by_floor
                    .get(floor_id)
                    .into_iter()
                    .flat_map(|children| children.iter().copied()),
            );
        }
        if seen.len() != members.len() {
            return Err(ContentError::InvalidProceduralFloor(root.id.clone()));
        }
    }
    let task_ids = world
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    for dungeon in &world.dungeons {
        for requirement in &dungeon.entry_requirements {
            match requirement {
                DungeonEntryRequirementDefinition::TaskStatus { task_id, .. } => {
                    validate_definition_id(task_id, "task")?;
                    if !task_ids.contains(task_id.as_str()) {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
                DungeonEntryRequirementDefinition::DungeonConquered { dungeon_id } => {
                    validate_definition_id(dungeon_id, "dungeon")?;
                    if !dungeon_definition_ids.contains(dungeon_id) || dungeon_id == &dungeon.id {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
                DungeonEntryRequirementDefinition::CarriedItem {
                    item_kind_id,
                    quantity,
                } => {
                    if *quantity == 0 || !item_limits.contains_key(item_kind_id) {
                        return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
                    }
                }
            }
        }
    }
    let mut entry_terrain_ids = BTreeSet::new();
    for floor in world.procedural_floors.iter().filter(|floor| {
        floor.lifecycle != FloorLifecycle::Town && floor.return_floor_id == world.initial_floor_id
    }) {
        if !entry_terrain_ids.insert(floor.entry_terrain_id.as_deref()) {
            return Err(ContentError::InvalidProceduralFloor(floor.id.clone()));
        }
    }
    require_reference(terrain_ids, &world.fill_terrain_id, &world.id)?;
    require_reference(terrain_ids, &world.border_terrain_id, &world.id)?;
    require_actor_role(
        actor_roles,
        &world.player.kind_id,
        ActorRole::Player,
        &world.id,
    )?;
    if let Some(build_id) = &world.player_build_id {
        require_reference(build_ids, build_id, &world.id)?;
    }
    validate_position(world.player.position, world.width, world.height, &world.id)?;
    validate_id(&world.player.instance_id)?;

    let mut instance_ids = BTreeSet::new();
    instance_ids.insert(world.player.instance_id.clone());
    let mut actor_positions = BTreeSet::new();
    actor_positions.insert(world.player.position);

    world
        .actors
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for actor in &world.actors {
        validate_id(&actor.instance_id)?;
        if !instance_ids.insert(actor.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(actor.instance_id.clone()));
        }
        require_actor_role(actor_roles, &actor.kind_id, ActorRole::Monster, &world.id)?;
        validate_position(actor.position, world.width, world.height, &world.id)?;
        if !actor_positions.insert(actor.position) {
            return Err(ContentError::DuplicateActorPosition(world.id.clone()));
        }
    }
    for dungeon in &world.dungeons {
        let Some(guardian) = &dungeon.entrance_guardian else {
            continue;
        };
        if !actor_positions.insert(guardian.position) {
            return Err(ContentError::DuplicateActorPosition(world.id.clone()));
        }
    }
    for actor_id in procedural_actor_ids {
        if !instance_ids.insert(actor_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(actor_id));
        }
    }

    world
        .items
        .sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    for item in &mut world.items {
        validate_id(&item.instance_id)?;
        if !instance_ids.insert(item.instance_id.clone()) {
            return Err(ContentError::DuplicateInstanceId(item.instance_id.clone()));
        }
        let (max_stack, equippable) =
            item_limits
                .get(&item.kind_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: world.id.clone(),
                    target: item.kind_id.clone(),
                })?;
        if item.quantity == 0 || item.quantity > *max_stack {
            return Err(ContentError::InvalidItemQuantity(item.instance_id.clone()));
        }
        item.affix_ids.sort();
        let mut seen_affixes = BTreeSet::new();
        if (item.quality != ItemQuality::Ordinary && (*max_stack != 1 || item.quantity != 1))
            || (!item.affix_ids.is_empty()
                && (*max_stack != 1
                    || !equippable
                    || item.quantity != 1
                    || item.quality == ItemQuality::Ordinary))
            || item.affix_ids.iter().any(|affix_id| {
                !affix_ids.contains(affix_id) || !seen_affixes.insert(affix_id.as_str())
            })
        {
            return Err(ContentError::InvalidItemAffixes(item.instance_id.clone()));
        }
        validate_position(item.position, world.width, world.height, &world.id)?;
    }

    world
        .terrain_overrides
        .sort_by(|left, right| left.terrain_id.cmp(&right.terrain_id));
    let mut override_terrain = BTreeMap::new();
    for terrain_override in &mut world.terrain_overrides {
        require_reference(terrain_ids, &terrain_override.terrain_id, &world.id)?;
        terrain_override.positions.sort();
        for position in &terrain_override.positions {
            validate_position(*position, world.width, world.height, &world.id)?;
            if position.x == 0
                || position.y == 0
                || position.x == world.width - 1
                || position.y == world.height - 1
                || override_terrain
                    .insert(*position, terrain_override.terrain_id.clone())
                    .is_some()
            {
                return Err(ContentError::InvalidTerrainOverride(world.id.clone()));
            }
        }
    }

    for town_id in &world_town_ids {
        let town = towns
            .get(*town_id)
            .ok_or_else(|| ContentError::DanglingReference {
                owner: world.id.clone(),
                target: (*town_id).to_owned(),
            })?;
        let (town_width, town_height, town_fill_terrain_id, town_terrain) =
            if world.town_id.as_deref() == Some(*town_id) {
                if town.floor_id != world.initial_floor_id {
                    return Err(ContentError::InvalidTown(town.id.clone()));
                }
                (
                    world.width,
                    world.height,
                    world.fill_terrain_id.as_str(),
                    override_terrain
                        .iter()
                        .map(|(position, terrain_id)| (*position, terrain_id.as_str()))
                        .collect::<BTreeMap<_, _>>(),
                )
            } else {
                let floor = world
                    .procedural_floors
                    .iter()
                    .find(|floor| {
                        floor.id == town.floor_id && floor.lifecycle == FloorLifecycle::Town
                    })
                    .ok_or_else(|| ContentError::InvalidTown(town.id.clone()))?;
                let inline_map = floor
                    .inline_map
                    .as_ref()
                    .expect("validated town floor must retain its inline map");
                (
                    floor.width,
                    floor.height,
                    floor.wall_terrain_id.as_str(),
                    inline_map
                        .terrain_overrides
                        .iter()
                        .filter(|terrain| terrain.chance_percent == 100)
                        .flat_map(|terrain| {
                            terrain
                                .positions
                                .iter()
                                .map(|position| (*position, terrain.terrain_id.as_str()))
                        })
                        .collect::<BTreeMap<_, _>>(),
                )
            };
        if world
            .procedural_floors
            .iter()
            .filter(|floor| floor.lifecycle == FloorLifecycle::Town && floor.id == town.floor_id)
            .count()
            != usize::from(world.town_id.as_deref() != Some(*town_id))
        {
            return Err(ContentError::InvalidTown(town.id.clone()));
        }
        let mut entrance_positions = BTreeSet::new();
        for facility_id in &town.facility_ids {
            let facility = town_facilities
                .get(facility_id)
                .expect("validated town facility reference must remain available");
            validate_position(
                facility.entrance_position,
                town_width,
                town_height,
                &facility.id,
            )?;
            require_reference(terrain_ids, &facility.entrance_terrain_id, &facility.id)?;
            let effective_terrain_id = town_terrain
                .get(&facility.entrance_position)
                .copied()
                .unwrap_or(town_fill_terrain_id);
            if !entrance_positions.insert(facility.entrance_position)
                || effective_terrain_id != facility.entrance_terrain_id
                || terrain_walkability.get(effective_terrain_id) != Some(&true)
                || !terrain_tags
                    .get(effective_terrain_id)
                    .is_some_and(|tags| tags.contains("town-facility-entrance"))
            {
                return Err(ContentError::InvalidTownFacility(facility.id.clone()));
            }
        }
        for shop_id in &town.shop_ids {
            let shop = shops
                .get(shop_id)
                .expect("validated town shop reference must remain available");
            validate_position(shop.entrance_position, town_width, town_height, &shop.id)?;
            require_reference(terrain_ids, &shop.entrance_terrain_id, &shop.id)?;
            let effective_terrain_id = town_terrain
                .get(&shop.entrance_position)
                .copied()
                .unwrap_or(town_fill_terrain_id);
            if !entrance_positions.insert(shop.entrance_position)
                || effective_terrain_id != shop.entrance_terrain_id
                || terrain_walkability.get(effective_terrain_id) != Some(&true)
                || !terrain_tags
                    .get(effective_terrain_id)
                    .is_some_and(|tags| tags.contains("shop-entrance"))
            {
                return Err(ContentError::InvalidShop(shop.id.clone()));
            }
        }
    }

    require_walkable_spawn(
        world,
        world.player.position,
        &override_terrain,
        terrain_walkability,
    )?;
    for actor in &world.actors {
        require_actor_enterable_spawn(
            world,
            &actor.kind_id,
            actor.position,
            &override_terrain,
            actors,
            terrain,
        )?;
    }
    for dungeon in &world.dungeons {
        let Some(guardian) = &dungeon.entrance_guardian else {
            continue;
        };
        require_actor_enterable_spawn(
            world,
            &guardian.actor_kind_id,
            guardian.position,
            &override_terrain,
            actors,
            terrain,
        )?;
        let terrain_id = override_terrain
            .get(&guardian.position)
            .unwrap_or(&world.fill_terrain_id);
        if world.procedural_floors.iter().any(|floor| {
            floor.return_floor_id == world.initial_floor_id
                && floor.entry_terrain_id.as_deref() == Some(terrain_id.as_str())
        }) {
            return Err(ContentError::InvalidProceduralFloor(dungeon.id.clone()));
        }
    }
    for item in &world.items {
        require_walkable_spawn(world, item.position, &override_terrain, terrain_walkability)?;
    }
    Ok(())
}

fn require_walkable_spawn(
    world: &WorldDefinition,
    position: ContentPosition,
    override_terrain: &BTreeMap<ContentPosition, String>,
    terrain_walkability: &BTreeMap<String, bool>,
) -> Result<(), ContentError> {
    let terrain_id = if position.x == 0
        || position.y == 0
        || position.x == world.width - 1
        || position.y == world.height - 1
    {
        &world.border_terrain_id
    } else {
        override_terrain
            .get(&position)
            .unwrap_or(&world.fill_terrain_id)
    };
    if terrain_walkability.get(terrain_id) != Some(&true) {
        return Err(ContentError::SpawnOnBlockedTerrain(world.id.clone()));
    }
    Ok(())
}

fn require_actor_enterable_spawn(
    world: &WorldDefinition,
    actor_kind_id: &str,
    position: ContentPosition,
    override_terrain: &BTreeMap<ContentPosition, String>,
    actors: &[ActorDefinition],
    terrain: &[TerrainDefinition],
) -> Result<(), ContentError> {
    let terrain_id = if position.x == 0
        || position.y == 0
        || position.x == world.width - 1
        || position.y == world.height - 1
    {
        &world.border_terrain_id
    } else {
        override_terrain
            .get(&position)
            .unwrap_or(&world.fill_terrain_id)
    };
    let can_enter = actors
        .iter()
        .find(|actor| actor.id == actor_kind_id)
        .zip(terrain.iter().find(|tile| tile.id == *terrain_id))
        .is_some_and(|(actor, tile)| {
            tile.walkable
                || actor
                    .movement
                    .modes
                    .iter()
                    .any(|mode| tile.movement_modes.contains(mode))
        });
    if !can_enter {
        return Err(ContentError::SpawnOnBlockedTerrain(world.id.clone()));
    }
    Ok(())
}
