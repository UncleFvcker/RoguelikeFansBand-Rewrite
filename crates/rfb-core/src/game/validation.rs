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
use super::gold::{MAX_PLAYER_GOLD, derive_next_gold_pile_serial, generated_gold_serial};
use super::*;

impl Game {
    pub(super) fn validate_runtime_invariants(&self, action: &GameAction) -> Result<(), CoreError> {
        if self.map_scale == rfb_protocol::MapScaleDto::World
            && !matches!(
                action,
                GameAction::Move { .. }
                    | GameAction::LeaveWorldMap
                    | GameAction::TravelWorld { .. }
            )
        {
            return Err(CoreError::WorldMapActionUnavailable);
        }
        if let GameAction::EnterWorldMap {
            leave_pets,
            cancel_recall,
        } = action
            && (self.map_scale != rfb_protocol::MapScaleDto::Local
                || (self.current_floor_id
                    != self
                        .content
                        .world(&self.world_id)
                        .expect("active world must remain available")
                        .initial_floor_id
                    && !self.is_wilderness_floor())
                || self
                    .content
                    .world(&self.world_id)
                    .and_then(|world| world.wilderness.as_ref())
                    .is_none()
                || self.wilderness_ambush_threat_remains()
                || (self.player_has_following_pet() && !leave_pets)
                || (self.recall_is_active() && !cancel_recall))
        {
            return Err(CoreError::WorldMapTransitionUnavailable);
        }
        if matches!(action, GameAction::LeaveWorldMap)
            && self.map_scale != rfb_protocol::MapScaleDto::World
        {
            return Err(CoreError::WorldMapTransitionUnavailable);
        }
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
        ItemLocation::Shop { .. } | ItemLocation::Home { .. } => false,
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

impl Game {
    pub(super) fn validate_loaded_state(&self) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .ok_or_else(|| CoreError::UnknownWorld(self.world_id.clone()))?;
        let valid_floor = |floor_id: &str| {
            floor_id == world.initial_floor_id
                || world
                    .procedural_floors
                    .iter()
                    .any(|floor| floor.id == floor_id)
        };
        if !(valid_floor(&self.current_floor_id)
            || (self.current_floor_id == wilderness::WILDERNESS_FLOOR_ID
                && world.wilderness.is_some()))
            || self
                .stored_floors
                .values()
                .any(|floor| !valid_floor(&floor.id))
        {
            return Err(CoreError::InvalidSave("floor identity is invalid"));
        }
        if self.is_wilderness_floor()
            && (!self.stored_floors.contains_key(&world.initial_floor_id)
                || self.current_dungeon_instance_id.is_some()
                || self.width != world.width
                || self.height != world.height
                || (self.map_scale == rfb_protocol::MapScaleDto::Local
                    && self
                        .wilderness_position
                        .is_some_and(|position| self.wilderness_position_is_town(position))))
        {
            return Err(CoreError::InvalidSave("local wilderness state is invalid"));
        }
        if self.defeated_unique_actor_kind_ids.iter().any(|kind_id| {
            !self.content.actor(kind_id).is_some_and(|definition| {
                definition.tags.iter().any(|tag| tag == "unique")
                    && !definition.tags.iter().any(|tag| tag == "guardian")
            }) || self
                .entities
                .iter()
                .chain(
                    self.stored_floors
                        .values()
                        .flat_map(|floor| floor.entities.iter()),
                )
                .any(|actor| actor.hp > 0 && actor.kind_id == *kind_id)
        }) {
            return Err(CoreError::InvalidSave(
                "defeated unique actor state is invalid",
            ));
        }
        let mut living_unique_actor_kind_ids = BTreeSet::new();
        if self
            .entities
            .iter()
            .chain(
                self.stored_floors
                    .values()
                    .flat_map(|floor| floor.entities.iter()),
            )
            .filter(|actor| actor.hp > 0)
            .filter(|actor| {
                self.content
                    .actor(&actor.kind_id)
                    .is_some_and(|definition| {
                        definition.tags.iter().any(|tag| tag == "unique")
                            && !definition.tags.iter().any(|tag| tag == "guardian")
                    })
            })
            .any(|actor| !living_unique_actor_kind_ids.insert(actor.kind_id.as_str()))
        {
            return Err(CoreError::InvalidSave(
                "living unique actor state is duplicated",
            ));
        }
        match world
            .town_id
            .as_deref()
            .and_then(|town_id| self.content.town(town_id))
        {
            None if !self.town_states.is_empty()
                || !self.shop_states.is_empty()
                || !self.home_states.is_empty() =>
            {
                return Err(CoreError::InvalidSave("town state is invalid"));
            }
            None => {}
            Some(town) => {
                let home_facility_ids = town
                    .facility_ids
                    .iter()
                    .filter(|facility_id| {
                        self.content
                            .town_facility(facility_id)
                            .is_some_and(|facility| {
                                facility.category == rfb_content::TownFacilityCategory::Home
                            })
                    })
                    .collect::<BTreeSet<_>>();
                if self.town_states.len() != 1
                    || !self
                        .town_states
                        .get(&town.id)
                        .is_some_and(|state| state.visited)
                    || self.shop_states.len() != town.shop_ids.len()
                    || town
                        .shop_ids
                        .iter()
                        .any(|shop_id| !self.shop_states.contains_key(shop_id))
                    || self.home_states.len() != home_facility_ids.len()
                    || home_facility_ids
                        .iter()
                        .any(|facility_id| !self.home_states.contains_key(*facility_id))
                {
                    return Err(CoreError::InvalidSave("town state is invalid"));
                }
                if self.current_floor_id == town.floor_id
                    && town.shop_ids.iter().any(|shop_id| {
                        let shop = self
                            .content
                            .shop(shop_id)
                            .expect("validated town shop must remain available");
                        self.player.position == position_from_content(shop.entrance_position)
                            && !self
                                .shop_states
                                .get(shop_id)
                                .is_some_and(|state| state.visited)
                    })
                {
                    return Err(CoreError::InvalidSave("shop state is invalid"));
                }
                if self.current_floor_id == town.floor_id
                    && home_facility_ids.iter().any(|facility_id| {
                        let facility = self
                            .content
                            .town_facility(facility_id)
                            .expect("validated town facility must remain available");
                        self.player.position == position_from_content(facility.entrance_position)
                            && !self
                                .home_states
                                .get(*facility_id)
                                .is_some_and(|state| state.visited)
                    })
                {
                    return Err(CoreError::InvalidSave("home state is invalid"));
                }
                for shop_id in &town.shop_ids {
                    let shop = self
                        .content
                        .shop(shop_id)
                        .expect("validated town shop must remain available");
                    let state = self
                        .shop_states
                        .get(shop_id)
                        .expect("validated shop state must remain available");
                    if state.owner_id != shop.owner.id
                        || state.last_maintenance_world_tick > self.world_tick
                    {
                        return Err(CoreError::InvalidSave("shop state is invalid"));
                    }
                }
            }
        }
        let current_dungeon_id = floor_dungeon_id(world, &self.current_floor_id);
        match (&current_dungeon_id, &self.current_dungeon_instance_id) {
            (Some(dungeon_id), Some(instance_id))
                if parse_dungeon_instance_ordinal(instance_id, dungeon_id).is_some() => {}
            (None, None) => {}
            _ => {
                return Err(CoreError::InvalidSave(
                    "active floor dungeon instance identity is invalid",
                ));
            }
        }
        if let Some(recall) = &self.recall {
            let destination_is_valid = world.procedural_floors.iter().any(|floor| {
                floor.id == recall.floor_id
                    && floor.lifecycle == FloorLifecycle::Dungeon
                    && floor.dungeon_id.as_deref() == Some(recall.dungeon_id.as_str())
            });
            let pending_is_valid = recall
                .remaining_turns
                .is_none_or(|turns| (1..=2_000).contains(&turns));
            let current_location_allows_pending = recall.remaining_turns.is_none()
                || self.current_floor_id == world.initial_floor_id
                || current_dungeon_id.is_some();
            if !destination_is_valid || !pending_is_valid || !current_location_allows_pending {
                return Err(CoreError::InvalidSave("player recall state is invalid"));
            }
        }
        for floor in self.stored_floors.values() {
            let expected_instance = floor_dungeon_id(world, &floor.id).is_some();
            if expected_instance != floor.dungeon_instance_id.is_some() {
                return Err(CoreError::InvalidSave(
                    "stored floor dungeon instance identity is invalid",
                ));
            }
        }
        if !floor_connections_are_valid(
            &self.current_floor_id,
            self.width,
            self.height,
            &self.terrain,
            &self.floor_connections,
            world,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor connection state is invalid",
            ));
        }
        if !floor_regions_are_valid(
            &self.current_floor_id,
            (self.width, self.height),
            &self.floor_regions,
            &self.entities,
            &self.items,
            world,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "active floor region state is invalid",
            ));
        }
        if self.explored.len() != self.terrain.len() {
            return Err(CoreError::InvalidSave(
                "exploration memory dimensions are invalid",
            ));
        }
        if !revealed_terrain_is_valid(
            &self.revealed_terrain,
            &self.terrain,
            self.width,
            self.height,
            &self.content,
        ) {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
        match (self.summon_command.mode, self.summon_command.guard_position) {
            (SummonCommandModeDto::Guard, Some(position))
                if self.index(position).is_some() && self.is_walkable(position) => {}
            (SummonCommandModeDto::Guard, _) => {
                return Err(CoreError::InvalidSave(
                    "summon guard command position is invalid",
                ));
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err(CoreError::InvalidSave(
                    "non-guard summon command retains a guard position",
                ));
            }
        }
        for terrain_id in &self.terrain {
            if self.content.terrain(terrain_id).is_none() {
                return Err(CoreError::UnknownTerrain(terrain_id.clone()));
            }
        }
        let victory_cap_unlocked = self.campaign_state.status != CampaignStatusDto::Active;
        if self.gold > MAX_PLAYER_GOLD {
            return Err(CoreError::InvalidSave("player gold balance is invalid"));
        }
        if self.nutrition > rfb_protocol::PLAYER_NUTRITION_MAXIMUM {
            return Err(CoreError::InvalidSave("player nutrition is invalid"));
        }
        let expected_skills =
            character_skill_progress(&self.content, self.build.as_ref(), self.progress.level)?;
        if !self.progress.validate(victory_cap_unlocked)
            || self.progress.skills != expected_skills
            || self.progress.hp_progression.first().copied() != Some(self.player.max_hp)
            || self.progress.hp_progression.windows(2).any(|window| {
                let increase = window[1].saturating_sub(window[0]);
                !(1..=10).contains(&increase)
            })
        {
            return Err(CoreError::InvalidSave("character progress is invalid"));
        }
        self.validate_actor(&self.player, ActorRole::Player)?;
        if self.index(self.player.position).is_none() {
            return Err(CoreError::InvalidSave("player position is invalid"));
        }
        let mut instance_ids = BTreeSet::new();
        instance_ids.insert(self.player.id.clone());
        let mut monster_ids = BTreeSet::new();
        let mut positions = BTreeSet::new();
        positions.insert(self.player.position);
        if let Some(mount_id) = self.riding_actor_id.as_deref() {
            let valid_mount = self.entities.iter().find(|entity| entity.id == mount_id);
            if !valid_mount.is_some_and(|mount| {
                mount.hp > 0
                    && mount.position == self.player.position
                    && mount.controller_id.as_deref() == Some(self.player.id.as_str())
                    && self
                        .content
                        .actor(&mount.kind_id)
                        .is_some_and(|definition| definition.rideable)
            }) {
                return Err(CoreError::InvalidSave("riding state is invalid"));
            }
        }
        for entity in &self.entities {
            let is_mount = self.riding_actor_id.as_deref() == Some(entity.id.as_str());
            self.validate_actor(entity, ActorRole::Monster)?;
            if let Some(summon) = &entity.summon
                && !self.summon_identity_is_valid(entity, summon)
            {
                return Err(CoreError::InvalidSave("summon state is invalid"));
            }
            if !instance_ids.insert(entity.id.clone())
                || !self.actor_kind_can_enter_position(&entity.kind_id, entity.position)
                || (!positions.insert(entity.position) && !is_mount)
            {
                return Err(CoreError::InvalidSave("entity position is invalid"));
            }
            monster_ids.insert(entity.id.clone());
        }
        if !monster_packs_are_valid(&self.entities) {
            return Err(CoreError::InvalidSave("monster pack state is invalid"));
        }
        let mut equipment_slots = BTreeSet::new();
        for item in &self.items {
            let definition = self
                .content
                .item(&item.kind_id)
                .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
            let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                && item
                    .affix_ids
                    .iter()
                    .all(|affix_id| self.content.affix(affix_id).is_some())
                && rolled_affixes_are_valid(item)
                && (item.affix_ids.is_empty()
                    || (definition.max_stack == 1
                        && definition.equipment_slot.is_some()
                        && item.quantity == 1
                        && item.quality != ItemQualityDto::Ordinary))
                && (item.quality == ItemQualityDto::Ordinary
                    || (definition.max_stack == 1 && item.quantity == 1));
            let common_valid = instance_ids.insert(item.id.clone()) && item.quantity != 0;
            if !affixes_are_valid {
                return Err(CoreError::InvalidSave(
                    "item quality or affix state is invalid",
                ));
            }
            match &item.location {
                ItemLocation::Ground(position) => {
                    if !common_valid
                        || !self.is_walkable(*position)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("item state is invalid"));
                    }
                }
                ItemLocation::Inventory => {
                    if !common_valid || item.quantity > definition.max_stack {
                        return Err(CoreError::InvalidSave("inventory item state is invalid"));
                    }
                }
                ItemLocation::Equipped { slot_id } => {
                    let fully_identified =
                        self.item_property_knowledge
                            .get(&item.id)
                            .is_some_and(|knowledge| {
                                knowledge.identified
                                    && item.affix_ids.iter().all(|affix_id| {
                                        knowledge.known_affix_ids.contains(affix_id)
                                    })
                            });
                    // The occupied instance must exist on the body and its
                    // type must match the item's declared slot class.
                    let slot_type_matches = self.body_slot_type(slot_id).is_some_and(|slot_type| {
                        definition
                            .equipment_slot
                            .as_deref()
                            .is_some_and(|declared| item_can_occupy_slot_type(declared, slot_type))
                    });
                    if !common_valid
                        || item.quantity != 1
                        || !slot_type_matches
                        || !equipment_slots.insert(slot_id.clone())
                        || !fully_identified
                    {
                        return Err(CoreError::InvalidSave("equipment item state is invalid"));
                    }
                }
                ItemLocation::CarriedBy { actor_id } => {
                    if !common_valid
                        || !monster_ids.contains(actor_id)
                        || item.quantity > definition.max_stack
                    {
                        return Err(CoreError::InvalidSave("carried item state is invalid"));
                    }
                }
                ItemLocation::Shop { .. } => {
                    return Err(CoreError::InvalidSave(
                        "shop item is in the active item set",
                    ));
                }
                ItemLocation::Home { .. } => {
                    return Err(CoreError::InvalidSave(
                        "home item is in the active item set",
                    ));
                }
            }
        }
        if self.inventory_used_slots() > self.inventory_slot_capacity() {
            return Err(CoreError::InvalidSave("inventory exceeds slot capacity"));
        }
        for (shop_id, state) in &self.shop_states {
            for item in &state.inventory {
                let definition = self
                    .content
                    .item(&item.kind_id)
                    .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
                let location_is_valid = matches!(
                    &item.location,
                    ItemLocation::Shop { shop_id: location_shop_id }
                        if location_shop_id == shop_id
                );
                let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                    && item
                        .affix_ids
                        .iter()
                        .all(|affix_id| self.content.affix(affix_id).is_some())
                    && rolled_affixes_are_valid(item)
                    && (item.quality == ItemQualityDto::Ordinary
                        || (definition.max_stack == 1 && item.quantity == 1));
                if !instance_ids.insert(item.id.clone())
                    || item.quantity == 0
                    || item.quantity > definition.max_stack
                    || !affixes_are_valid
                    || !location_is_valid
                {
                    return Err(CoreError::InvalidSave("shop item state is invalid"));
                }
            }
        }
        for (facility_id, state) in &self.home_states {
            for item in &state.inventory {
                let definition = self
                    .content
                    .item(&item.kind_id)
                    .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
                let location_is_valid = matches!(
                    &item.location,
                    ItemLocation::Home { facility_id: location_facility_id }
                        if location_facility_id == facility_id
                );
                let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                    && item
                        .affix_ids
                        .iter()
                        .all(|affix_id| self.content.affix(affix_id).is_some())
                    && rolled_affixes_are_valid(item)
                    && (item.quality == ItemQualityDto::Ordinary
                        || (definition.max_stack == 1 && item.quantity == 1));
                if !instance_ids.insert(item.id.clone())
                    || item.quantity == 0
                    || item.quantity > definition.max_stack
                    || !affixes_are_valid
                    || !location_is_valid
                {
                    return Err(CoreError::InvalidSave("home item state is invalid"));
                }
            }
        }
        for pile in &self.gold_piles {
            if !instance_ids.insert(pile.id.clone())
                || generated_gold_serial(&pile.id).is_none()
                || pile.amount == 0
                || !self.is_walkable(pile.position)
            {
                return Err(CoreError::InvalidSave("gold pile state is invalid"));
            }
        }
        for floor in self.stored_floors.values() {
            let expected_len = usize::from(floor.width) * usize::from(floor.height);
            if floor.terrain.len() != expected_len
                || floor.explored.len() != expected_len
                || !revealed_terrain_is_valid(
                    &floor.revealed_terrain,
                    &floor.terrain,
                    floor.width,
                    floor.height,
                    &self.content,
                )
                || (floor.id == self.current_floor_id
                    && floor.dungeon_instance_id == self.current_dungeon_instance_id)
                || !floor_position_is_walkable(floor, floor.player_position, &self.content)
            {
                return Err(CoreError::InvalidSave("stored floor state is invalid"));
            }
            if !floor_connections_are_valid(
                &floor.id,
                floor.width,
                floor.height,
                &floor.terrain,
                &floor.connections,
                world,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor connection state is invalid",
                ));
            }
            if !floor_regions_are_valid(
                &floor.id,
                (floor.width, floor.height),
                &floor.regions,
                &floor.entities,
                &floor.items,
                world,
                &self.content,
            ) {
                return Err(CoreError::InvalidSave(
                    "stored floor region state is invalid",
                ));
            }
            for terrain_id in &floor.terrain {
                if self.content.terrain(terrain_id).is_none() {
                    return Err(CoreError::UnknownTerrain(terrain_id.clone()));
                }
            }
            let mut floor_positions = BTreeSet::new();
            let mut floor_monster_ids = BTreeSet::new();
            for entity in &floor.entities {
                self.validate_actor(entity, ActorRole::Monster)?;
                if !instance_ids.insert(entity.id.clone())
                    || !floor_actor_position_is_enterable(
                        floor,
                        &entity.kind_id,
                        entity.position,
                        &self.content,
                    )
                    || !floor_positions.insert(entity.position)
                {
                    return Err(CoreError::InvalidSave(
                        "stored floor entity state is invalid",
                    ));
                }
                floor_monster_ids.insert(entity.id.clone());
            }
            if !monster_packs_are_valid(&floor.entities) {
                return Err(CoreError::InvalidSave(
                    "stored floor monster pack state is invalid",
                ));
            }
            for item in &floor.items {
                let definition = self
                    .content
                    .item(&item.kind_id)
                    .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
                let affixes_are_valid = item.affix_ids.windows(2).all(|pair| pair[0] < pair[1])
                    && item
                        .affix_ids
                        .iter()
                        .all(|affix_id| self.content.affix(affix_id).is_some())
                    && rolled_affixes_are_valid(item)
                    && (item.affix_ids.is_empty()
                        || (definition.max_stack == 1
                            && definition.equipment_slot.is_some()
                            && item.quantity == 1
                            && item.quality != ItemQualityDto::Ordinary))
                    && (item.quality == ItemQualityDto::Ordinary
                        || (definition.max_stack == 1 && item.quantity == 1));
                let location_is_valid = match &item.location {
                    ItemLocation::Ground(position) => {
                        floor_position_is_walkable(floor, *position, &self.content)
                    }
                    ItemLocation::CarriedBy { actor_id } => floor_monster_ids.contains(actor_id),
                    ItemLocation::Inventory
                    | ItemLocation::Equipped { .. }
                    | ItemLocation::Shop { .. }
                    | ItemLocation::Home { .. } => false,
                };
                if !instance_ids.insert(item.id.clone())
                    || item.quantity == 0
                    || item.quantity > definition.max_stack
                    || !affixes_are_valid
                    || !location_is_valid
                {
                    return Err(CoreError::InvalidSave("stored floor item state is invalid"));
                }
            }
            for pile in &floor.gold_piles {
                if !instance_ids.insert(pile.id.clone())
                    || generated_gold_serial(&pile.id).is_none()
                    || pile.amount == 0
                    || !floor_position_is_walkable(floor, pile.position, &self.content)
                {
                    return Err(CoreError::InvalidSave(
                        "stored floor gold pile state is invalid",
                    ));
                }
            }
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let expected_tasks = initial_task_states(world);
        if self
            .task_states
            .keys()
            .any(|task_id| task_definition(world, task_id).is_none())
            || expected_tasks
                .keys()
                .any(|task_id| !self.task_states.contains_key(task_id))
        {
            return Err(CoreError::InvalidSave("task state set is invalid"));
        }
        for (task_id, state) in &self.task_states {
            let Some(task) = task_definition(world, task_id) else {
                return Err(CoreError::InvalidSave("task state ID is invalid"));
            };
            let expected = expected_tasks
                .get(task_id)
                .cloned()
                .unwrap_or_else(|| super::tasks::task_initial_state(task, &self.task_states));
            let members = task_floors(world, task_id).collect::<Vec<_>>();
            let objectives = task_objectives(world, task_id);
            let Some(objective) = usize::try_from(state.stage_index)
                .ok()
                .and_then(|stage| objectives.get(stage))
            else {
                return Err(CoreError::InvalidSave("task stage is invalid"));
            };
            let active_is_valid = state.active_floor_id.as_ref().is_some_and(|floor_id| {
                floor_id == &self.current_floor_id
                    && members.iter().any(|floor| floor.id == *floor_id)
            });
            let paused_is_valid = members.iter().any(|floor| {
                self.stored_floors
                    .values()
                    .any(|stored| stored.id == floor.id)
            });
            let status_is_valid = match state.status {
                TaskStatusKindDto::Active => active_is_valid,
                TaskStatusKindDto::Paused => state.active_floor_id.is_none() && paused_is_valid,
                TaskStatusKindDto::Completed => {
                    state.active_floor_id.is_none()
                        && usize::try_from(state.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && state.current == state.required
                }
                TaskStatusKindDto::Available => {
                    state.active_floor_id.is_none()
                        && (task.source_facility_id.is_none()
                            || expected.status == TaskStatusKindDto::Available)
                }
                TaskStatusKindDto::Failed | TaskStatusKindDto::Abandoned => {
                    state.active_floor_id.is_none()
                }
                TaskStatusKindDto::Locked => {
                    state.active_floor_id.is_none()
                        && expected.status == TaskStatusKindDto::Locked
                        && state.stage_index == 0
                        && state.current == 0
                }
                TaskStatusKindDto::RewardAvailable => {
                    state.active_floor_id.is_none()
                        && task_definition(world, task_id)
                            .is_some_and(|task| task.source_facility_id.is_some())
                        && usize::try_from(state.stage_index)
                            .ok()
                            .is_some_and(|stage| stage + 1 == objectives.len())
                        && state.current == state.required
                }
                TaskStatusKindDto::Taken => {
                    state.active_floor_id.is_none()
                        && task_definition(world, task_id)
                            .is_some_and(|task| task.source_facility_id.is_some())
                }
            };
            if (state.stage_index == 0 && expected.required != objective.required)
                || state.required != objective.required
                || state.current > state.required
                || members
                    .first()
                    .and_then(|floor| floor.max_retakes)
                    .is_some_and(|maximum| state.retakes_used > maximum)
                || !status_is_valid
            {
                return Err(CoreError::InvalidSave("task state is invalid"));
            }
        }
        let expected_dungeons = initial_dungeon_states(world);
        if self.dungeon_states.len() != expected_dungeons.len() {
            return Err(CoreError::InvalidSave("dungeon state set is invalid"));
        }
        for (dungeon_id, state) in &self.dungeon_states {
            if !expected_dungeons.contains_key(dungeon_id) {
                return Err(CoreError::InvalidSave("dungeon state ID is invalid"));
            }
            let dungeon = world
                .dungeons
                .iter()
                .find(|dungeon| dungeon.id == *dungeon_id)
                .expect("validated dungeon state must retain its definition");
            if dungeon.entrance_guardian.is_none() && state.entrance_guardian_defeated {
                return Err(CoreError::InvalidSave(
                    "dungeon entrance guardian state is invalid",
                ));
            }
            match (&state.retained_instance_id, state.retained_at_turn) {
                (None, None) => {}
                (Some(instance_id), Some(retained_at_turn)) => {
                    if dungeon.instance_lifecycle == DungeonInstanceLifecycle::ResetOnSurface
                        || parse_dungeon_instance_ordinal(instance_id, dungeon_id).is_none()
                        || retained_at_turn > self.turn
                        || !self.stored_floors.values().any(|floor| {
                            floor.dungeon_instance_id.as_deref() == Some(instance_id.as_str())
                        })
                    {
                        return Err(CoreError::InvalidSave(
                            "retained dungeon instance state is invalid",
                        ));
                    }
                }
                _ => {
                    return Err(CoreError::InvalidSave(
                        "retained dungeon instance state is incomplete",
                    ));
                }
            }
            if let Some(guardian) = &dungeon.entrance_guardian {
                let guardian_present = if self.current_floor_id == world.initial_floor_id {
                    Some(
                        self.entities
                            .iter()
                            .any(|actor| actor.id == guardian.instance_id),
                    )
                } else {
                    self.stored_floors
                        .values()
                        .find(|stored| stored.id == world.initial_floor_id)
                        .map(|floor| {
                            floor
                                .entities
                                .iter()
                                .any(|actor| actor.id == guardian.instance_id)
                        })
                };
                if guardian_present
                    .is_none_or(|present| present == state.entrance_guardian_defeated)
                {
                    return Err(CoreError::InvalidSave(
                        "dungeon entrance guardian state is invalid",
                    ));
                }
            }
            for final_floor in world.procedural_floors.iter().filter(|floor| {
                floor.dungeon_id.as_deref() == Some(dungeon_id.as_str()) && floor.final_floor
            }) {
                let guardian_id = &final_floor
                    .guardian
                    .as_ref()
                    .expect("validated final floor must retain a guardian")
                    .instance_id;
                let guardian_present = if self.current_floor_id == final_floor.id {
                    Some(self.entities.iter().any(|actor| &actor.id == guardian_id))
                } else {
                    self.stored_floors
                        .values()
                        .find(|stored| stored.id == final_floor.id)
                        .map(|floor| floor.entities.iter().any(|actor| &actor.id == guardian_id))
                };
                if guardian_present.is_some_and(|present| present == state.guardian_defeated) {
                    return Err(CoreError::InvalidSave("dungeon guardian state is invalid"));
                }
            }
        }
        let campaign_victory_reached = self.campaign_victory_reached();
        match self.campaign_definition() {
            None if self.campaign_state.status != CampaignStatusDto::Active => {
                return Err(CoreError::InvalidSave("campaign state is invalid"));
            }
            None => {}
            Some(_) => match self.campaign_state.status {
                CampaignStatusDto::Active => {
                    if self.campaign_state.victory_turn.is_some()
                        || self.campaign_state.retired_turn.is_some()
                        || self.campaign_state.final_score.is_some()
                        || campaign_victory_reached
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
                CampaignStatusDto::Victorious => {
                    if !campaign_victory_reached
                        || self.campaign_state.retired_turn.is_some()
                        || self.campaign_state.final_score.is_some()
                        || self
                            .campaign_state
                            .victory_turn
                            .is_none_or(|turn| turn > self.turn)
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
                CampaignStatusDto::Retired => {
                    let valid_turns = self
                        .campaign_state
                        .victory_turn
                        .zip(self.campaign_state.retired_turn)
                        .is_some_and(|(victory, retired)| {
                            victory <= retired && retired <= self.turn
                        });
                    let valid_score = self.campaign_state.final_score.is_some_and(|score| {
                        self.campaign_state
                            .retired_turn
                            .is_some_and(|turn| score == self.campaign_score_at(turn))
                    });
                    if !campaign_victory_reached
                        || self.current_floor_id != world.initial_floor_id
                        || self.current_dungeon_instance_id.is_some()
                        || !valid_turns
                        || !valid_score
                    {
                        return Err(CoreError::InvalidSave("campaign state is invalid"));
                    }
                }
            },
        }
        let casting_profile = self.casting_profile().cloned();
        let technique_profiles = self.technique_profiles().to_vec();
        let device_recharge_profile = self.device_recharge_profile().cloned();
        if self.bonus_spell_learning_capacity > 0 && !self.uses_spell_scrolls() {
            return Err(CoreError::InvalidSave(
                "bonus spell learning capacity is invalid",
            ));
        }
        if casting_profile.is_some()
            || !technique_profiles.is_empty()
            || device_recharge_profile.is_some()
        {
            let (expected_pool_maxima, expected_ability_ids) = self.player_ability_baseline();
            let pools_valid = self.resources.len() == expected_pool_maxima.len()
                && expected_pool_maxima.iter().all(|(id, expected_maximum)| {
                    self.resources.get(id).is_some_and(|pool| {
                        pool.maximum == *expected_maximum && pool.current <= pool.maximum
                    })
                });
            let learned_valid = match &casting_profile {
                Some(profile) => {
                    self.learned_abilities.len()
                        <= usize::from(self.ability_learning_capacity(profile))
                        && self.learned_abilities.iter().all(|ability_id| {
                            self.content.ability(ability_id).is_some_and(|ability| {
                                let ability = Self::effective_casting_ability(profile, ability);
                                Self::player_ability_parameters(&ability).minimum_level
                                    <= self.progress.level
                                    && self.profile_supports_ability(profile, ability_id)
                            })
                        })
                }
                None => self.learned_abilities.is_empty(),
            };
            if !pools_valid
                || !learned_valid
                || self
                    .ability_progress
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    != expected_ability_ids
                || self.ability_progress.iter().any(|(ability_id, progress)| {
                    self.content.ability(ability_id).is_none_or(|ability| {
                        progress.proficiency_cap
                            != Self::player_ability_parameters(ability).proficiency.cap
                            || progress.proficiency > progress.proficiency_cap
                            || progress.cooldown_remaining > self.ability_cooldown_turns(ability_id)
                    })
                })
            {
                return Err(CoreError::InvalidSave("player ability state is invalid"));
            }
        } else if !self.resources.is_empty()
            || !self.learned_abilities.is_empty()
            || !self.ability_progress.is_empty()
        {
            return Err(CoreError::InvalidSave(
                "non-caster player ability state is invalid",
            ));
        }
        for (item_id, knowledge) in &self.item_property_knowledge {
            let Some(item) = self
                .items
                .iter()
                .chain(
                    self.stored_floors
                        .values()
                        .flat_map(|floor| floor.items.iter()),
                )
                .chain(
                    self.home_states
                        .values()
                        .flat_map(|state| state.inventory.iter()),
                )
                .find(|item| &item.id == item_id)
            else {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            };
            let empty_knowledge = !knowledge.appraised
                && !knowledge.identified
                && knowledge.known_affix_ids.is_empty();
            let identification_without_appraisal = knowledge.identified && !knowledge.appraised;
            let foreign_affix = knowledge
                .known_affix_ids
                .iter()
                .any(|affix_id| !item.affix_ids.contains(affix_id));
            let incomplete_identification = knowledge.identified
                && item
                    .affix_ids
                    .iter()
                    .any(|affix_id| !knowledge.known_affix_ids.contains(affix_id));
            if empty_knowledge
                || identification_without_appraisal
                || foreign_affix
                || incomplete_identification
            {
                return Err(CoreError::InvalidSave(
                    "item property knowledge state is invalid",
                ));
            }
        }
        let mut allocator_entities = self.entities.clone();
        let mut allocator_items = self.items.clone();
        for floor in self.stored_floors.values() {
            allocator_entities.extend(floor.entities.iter().cloned());
            allocator_items.extend(floor.items.iter().cloned());
        }
        allocator_items.extend(
            self.shop_states
                .values()
                .flat_map(|state| state.inventory.iter().cloned()),
        );
        allocator_items.extend(
            self.home_states
                .values()
                .flat_map(|state| state.inventory.iter().cloned()),
        );
        if self.next_item_instance_serial == 0
            || self.next_item_instance_serial
                < derive_next_item_instance_serial(
                    &self.player,
                    &allocator_entities,
                    &allocator_items,
                )?
        {
            return Err(CoreError::InvalidSave(
                "item instance allocator is behind existing IDs",
            ));
        }
        let derived_next_gold_pile_serial = derive_next_gold_pile_serial(
            self.gold_piles.iter().chain(
                self.stored_floors
                    .values()
                    .flat_map(|floor| floor.gold_piles.iter()),
            ),
        )?;
        if self.next_gold_pile_serial == 0
            || self.next_gold_pile_serial < derived_next_gold_pile_serial
        {
            return Err(CoreError::InvalidSave(
                "gold pile allocator is behind existing IDs",
            ));
        }
        Ok(())
    }

    fn validate_actor(&self, actor: &Actor, expected_role: ActorRole) -> Result<(), CoreError> {
        let definition = self
            .content
            .actor(&actor.kind_id)
            .ok_or_else(|| CoreError::UnknownActor(actor.kind_id.clone()))?;
        let effective_max_hp = if expected_role == ActorRole::Player {
            self.effective_player_max_hp()
        } else {
            actor.max_hp
        };
        let statuses_are_valid = actor.statuses.iter().all(|status| {
            status.intensity > 0
                && status.remaining_ticks > 0
                && !status.kind_id.is_empty()
                && status.kind_id.len() <= 128
                && status.granted_resistances.len() <= 29
                && status
                    .granted_resistances
                    .values()
                    .all(|level| *level != ResistanceLevel::Normal)
                && (1..=100).contains(&status.incoming_damage_percent)
                && status
                    .granted_race_id
                    .as_deref()
                    .is_none_or(|race_id| self.content.race(race_id).is_some())
        }) && actor
            .statuses
            .windows(2)
            .all(|window| window[0].kind_id < window[1].kind_id);
        let resistance_memory_is_valid = if actor.observed_player_resistances.is_empty() {
            true
        } else {
            expected_role == ActorRole::Monster
                && actor.observed_player_resistances.len() <= 6
                && definition
                    .monster_casting
                    .as_ref()
                    .is_some_and(|casting| casting.smart)
                && !self.actor_is_player_aligned(actor)
        };
        let appearance_is_valid = actor.appearance_kind_id.as_deref().is_none_or(|kind_id| {
            expected_role == ActorRole::Monster
                && definition.level >= 10
                && !definition.tags.iter().any(|tag| tag == "unique")
                && self.content.actor(kind_id).is_some_and(|appearance| {
                    appearance
                        .tags
                        .iter()
                        .any(|tag| tag == "shadower-appearance")
                })
        });
        if definition.role != expected_role
            || (expected_role == ActorRole::Player && actor.max_hp != definition.max_hp)
            || (expected_role == ActorRole::Monster
                && !actor_max_hp_is_valid(definition, actor.max_hp))
            || actor.speed != definition.speed
            || actor.speed > 199
            || !statuses_are_valid
            || !resistance_memory_is_valid
            || !appearance_is_valid
            || (expected_role == ActorRole::Monster && actor.hp <= 0)
            || (expected_role == ActorRole::Player && actor.hp < -1_000_000)
            || (expected_role == ActorRole::Monster
                && !(1..=STANDARD_ACTION_COST).contains(&actor.energy_need))
            || (expected_role == ActorRole::Player && actor.hp >= 0 && actor.energy_need > 0)
            || actor.energy_need < -STANDARD_ACTION_COST
            || actor.hp > effective_max_hp
            || (expected_role == ActorRole::Player && actor.pack.is_some())
            || (expected_role == ActorRole::Player && actor.controller_id.is_some())
            || actor
                .controller_id
                .as_deref()
                .is_some_and(|controller_id| controller_id != self.player.id)
            || (actor.controller_id.is_some() && actor.pack.is_some())
            || (actor.summon.is_some() && actor.pack.is_some())
            || definition.monster_casting.as_ref().map_or(
                actor.casting_cooldown_remaining != 0,
                |casting| {
                    actor.casting_cooldown_remaining
                        > monster_casting_cooldown(casting.frequency_percent)
                },
            )
        {
            return Err(CoreError::InvalidSave("actor state is invalid"));
        }
        Ok(())
    }

    fn summon_identity_is_valid(&self, actor: &Actor, summon: &SummonIdentity) -> bool {
        let valid_id = |id: &str| {
            !id.is_empty()
                && id.len() <= 256
                && id.bytes().all(|byte| {
                    byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
                })
        };
        valid_id(&summon.owner_id)
            && valid_id(&summon.source_ability_id)
            && summon.remaining_turns > 0
            && self
                .content
                .ability(&summon.source_ability_id)
                .cloned()
                .map(|mut ability| {
                    if summon.owner_id == self.player.id {
                        if let Some(profile) = self.casting_profile() {
                            ability = Self::effective_casting_ability(profile, &ability);
                        }
                        Self::apply_player_level_scaling(&mut ability, self.progress.level);
                    }
                    ability
                })
                .is_some_and(|ability| match &ability.effect {
                    AbilityEffectDefinition::Summon { actor_kind_id, .. } => {
                        actor_kind_id == &actor.kind_id
                    }
                    AbilityEffectDefinition::SummonCategory {
                        category,
                        upgraded_category,
                        maximum_level,
                        ..
                    } => self.content.actor(&actor.kind_id).is_some_and(|kind| {
                        kind.tags.iter().any(|tag| {
                            tag == category
                                || upgraded_category
                                    .as_ref()
                                    .is_some_and(|upgraded| tag == upgraded)
                        }) && kind.level <= u32::from(*maximum_level)
                    }),
                    _ => false,
                })
    }
}
