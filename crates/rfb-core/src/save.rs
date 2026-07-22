// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use crate::{
    effect::StatusInstance,
    error::CoreError,
    resistance::{DamageType, ResistanceLevel, ResistanceProfile},
    state::{
        Actor, FloorConnectionState, FloorRegionState, FloorState, ItemInstance, ItemLocation,
        MonsterPackIdentity,
    },
};
use rfb_content::{ContentCatalog, ContentPosition};
use rfb_protocol::{
    ActorSaveDto, CarriedItemSaveDto, EquipmentItemSaveDto, FloorConnectionSaveDto,
    FloorRegionSaveDto, FloorSaveDto, InventoryItemSaveDto, ItemSaveDto, MonsterPackSaveDto,
    PlayerSaveDto, Position, ResistanceSaveDto, StatusSaveDto, TerrainSaveDto,
};

pub(crate) const GENERATED_ITEM_ID_PREFIX: &str = "generated.item.";

pub(crate) fn actor_from_spawn(
    id: &str,
    kind_id: &str,
    position: ContentPosition,
    max_hp: i32,
    speed: u16,
    energy_need: i32,
) -> Actor {
    Actor {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        position: position_from_content(position),
        hp: max_hp,
        max_hp,
        speed,
        energy_need,
        statuses: Vec::new(),
        resistances: ResistanceProfile::default(),
        pack: None,
    }
}

pub(crate) const fn position_from_content(position: ContentPosition) -> Position {
    Position {
        x: position.x as i32,
        y: position.y as i32,
    }
}

pub(crate) fn actor_from_player(
    player: PlayerSaveDto,
    content: &ContentCatalog,
) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&player.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(player.kind_id.clone()))?;
    if player.base_max_hp != 0 && player.base_max_hp != definition.max_hp {
        return Err(CoreError::InvalidSave("player base max HP is invalid"));
    }
    if player.base_speed != definition.speed {
        return Err(CoreError::InvalidSave("player base speed is invalid"));
    }
    let statuses = statuses_from_save(player.statuses)?;
    let resistances = resistances_from_save(player.resistances)?;
    Ok(Actor {
        id: player.id,
        kind_id: player.kind_id,
        position: player.position,
        hp: player.hp,
        max_hp: definition.max_hp,
        speed: player.base_speed,
        energy_need: player.energy_need,
        statuses,
        resistances,
        pack: None,
    })
}

pub(crate) fn derive_next_item_instance_serial(
    player: &Actor,
    entities: &[Actor],
    items: &[ItemInstance],
) -> Result<u64, CoreError> {
    let maximum = std::iter::once(player.id.as_str())
        .chain(entities.iter().map(|entity| entity.id.as_str()))
        .chain(items.iter().map(|item| item.id.as_str()))
        .filter_map(generated_item_serial)
        .max()
        .unwrap_or(0);
    maximum.checked_add(1).ok_or(CoreError::ItemIdExhausted)
}

fn generated_item_serial(id: &str) -> Option<u64> {
    id.strip_prefix(GENERATED_ITEM_ID_PREFIX)?.parse().ok()
}

pub(crate) fn actor_from_entity(
    entity: ActorSaveDto,
    content: &ContentCatalog,
) -> Result<Actor, CoreError> {
    let definition = content
        .actor(&entity.kind_id)
        .ok_or_else(|| CoreError::UnknownActor(entity.kind_id.clone()))?;
    if entity.max_hp != 0 && entity.max_hp != definition.max_hp {
        return Err(CoreError::InvalidSave("entity base stats are invalid"));
    }
    if entity.base_speed != definition.speed {
        return Err(CoreError::InvalidSave("entity base speed is invalid"));
    }
    let statuses = statuses_from_save(entity.statuses)?;
    let resistances = resistances_from_save(entity.resistances)?;
    Ok(Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: definition.max_hp,
        speed: entity.base_speed,
        energy_need: entity.energy_need,
        statuses,
        resistances,
        pack: entity.pack.map(|pack| MonsterPackIdentity {
            id: pack.id,
            leader_id: pack.leader_id,
            role: pack.role,
            behavior: pack.behavior,
        }),
    })
}

pub(crate) fn item_from_dto(item: ItemSaveDto) -> ItemInstance {
    ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        location: ItemLocation::Ground(item.position),
    }
}

pub(crate) fn inventory_item_from_dto(
    item: InventoryItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        location: ItemLocation::Inventory,
    })
}

pub(crate) fn equipment_item_from_dto(
    item: EquipmentItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    let definition = content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    if definition.equipment_slot.as_deref() != Some(item.slot_id.as_str()) {
        return Err(CoreError::InvalidSave("equipment metadata is invalid"));
    }
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        location: ItemLocation::Equipped {
            slot_id: item.slot_id,
        },
    })
}

pub(crate) fn carried_item_from_dto(
    item: CarriedItemSaveDto,
    content: &ContentCatalog,
) -> Result<ItemInstance, CoreError> {
    content
        .item(&item.kind_id)
        .ok_or_else(|| CoreError::UnknownItem(item.kind_id.clone()))?;
    Ok(ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
        quality: item.quality,
        affix_ids: item.affix_ids,
        location: ItemLocation::CarriedBy {
            actor_id: item.actor_id,
        },
    })
}

pub(crate) fn player_to_save(player: &Actor) -> PlayerSaveDto {
    PlayerSaveDto {
        id: player.id.clone(),
        kind_id: player.kind_id.clone(),
        position: player.position,
        hp: player.hp,
        base_max_hp: player.max_hp,
        base_speed: player.speed,
        energy_need: player.energy_need,
        statuses: player
            .statuses
            .iter()
            .map(StatusInstance::to_save_dto)
            .collect(),
        resistances: player.resistances.to_save_dtos(),
    }
}

pub(crate) fn actors_to_save(entities: &[Actor]) -> Vec<ActorSaveDto> {
    let mut entities = entities
        .iter()
        .map(|entity| ActorSaveDto {
            id: entity.id.clone(),
            kind_id: entity.kind_id.clone(),
            position: entity.position,
            hp: entity.hp,
            max_hp: entity.max_hp,
            base_speed: entity.speed,
            energy_need: entity.energy_need,
            statuses: entity
                .statuses
                .iter()
                .map(StatusInstance::to_save_dto)
                .collect(),
            resistances: entity.resistances.to_save_dtos(),
            pack: entity.pack.as_ref().map(|pack| MonsterPackSaveDto {
                id: pack.id.clone(),
                leader_id: pack.leader_id.clone(),
                role: pack.role,
                behavior: pack.behavior,
            }),
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    entities
}

fn statuses_from_save(mut statuses: Vec<StatusSaveDto>) -> Result<Vec<StatusInstance>, CoreError> {
    statuses.sort_by(|left, right| left.kind_id.cmp(&right.kind_id));
    let mut seen = BTreeSet::new();
    statuses
        .into_iter()
        .map(|status| {
            if !valid_rule_id(&status.kind_id)
                || !seen.insert(status.kind_id.clone())
                || status.intensity == 0
                || status.remaining_ticks == 0
                || status
                    .source_id
                    .as_deref()
                    .is_some_and(|source| source.is_empty() || source.len() > 128)
            {
                return Err(CoreError::InvalidSave("actor status state is invalid"));
            }
            Ok(StatusInstance {
                kind_id: status.kind_id,
                intensity: status.intensity,
                remaining_ticks: status.remaining_ticks,
                source_id: status.source_id,
            })
        })
        .collect()
}

fn resistances_from_save(
    resistances: Vec<ResistanceSaveDto>,
) -> Result<ResistanceProfile, CoreError> {
    let mut profile = ResistanceProfile::default();
    let mut seen = BTreeSet::new();
    for resistance in resistances {
        let damage_type = DamageType::from(resistance.damage_type);
        let level = ResistanceLevel::from(resistance.level);
        if !seen.insert(damage_type) || level == ResistanceLevel::Normal {
            return Err(CoreError::InvalidSave("actor resistance state is invalid"));
        }
        profile.set(damage_type, level);
    }
    Ok(profile)
}

fn valid_rule_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

pub(crate) fn items_to_save(items: &[ItemInstance]) -> Vec<ItemSaveDto> {
    let mut items = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::Ground(position) = &item.location else {
                return None;
            };
            Some(ItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                position: *position,
                quantity: item.quantity,
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

pub(crate) fn inventory_to_save(items: &[ItemInstance]) -> Vec<InventoryItemSaveDto> {
    let mut inventory = items
        .iter()
        .filter_map(|item| {
            if item.location != ItemLocation::Inventory {
                return None;
            }
            Some(InventoryItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
            })
        })
        .collect::<Vec<_>>();
    inventory.sort_by(|left, right| left.id.cmp(&right.id));
    inventory
}

pub(crate) fn equipment_to_save(items: &[ItemInstance]) -> Vec<EquipmentItemSaveDto> {
    let mut equipment = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::Equipped { slot_id } = &item.location else {
                return None;
            };
            Some(EquipmentItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                slot_id: slot_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
            })
        })
        .collect::<Vec<_>>();
    equipment.sort_by(|left, right| {
        left.slot_id
            .cmp(&right.slot_id)
            .then_with(|| left.id.cmp(&right.id))
    });
    equipment
}

pub(crate) fn carried_items_to_save(items: &[ItemInstance]) -> Vec<CarriedItemSaveDto> {
    let mut carried = items
        .iter()
        .filter_map(|item| {
            let ItemLocation::CarriedBy { actor_id } = &item.location else {
                return None;
            };
            Some(CarriedItemSaveDto {
                id: item.id.clone(),
                kind_id: item.kind_id.clone(),
                quantity: item.quantity,
                actor_id: actor_id.clone(),
                quality: item.quality,
                affix_ids: item.affix_ids.clone(),
            })
        })
        .collect::<Vec<_>>();
    carried.sort_by(|left, right| {
        left.actor_id
            .cmp(&right.actor_id)
            .then_with(|| left.id.cmp(&right.id))
    });
    carried
}

pub(crate) fn floor_to_save(floor: &FloorState) -> FloorSaveDto {
    FloorSaveDto {
        id: floor.id.clone(),
        player_position: floor.player_position,
        terrain: TerrainSaveDto {
            width: floor.width,
            height: floor.height,
            terrain_ids: floor.terrain.clone(),
        },
        entities: actors_to_save(&floor.entities),
        items: items_to_save(&floor.items),
        carried_items: carried_items_to_save(&floor.items),
        explored: floor.explored.clone(),
        revealed_terrain: floor.revealed_terrain.iter().copied().collect(),
        connections: floor_connections_to_save(&floor.connections),
        regions: floor_regions_to_save(&floor.regions),
    }
}

pub(crate) fn floor_from_save(
    floor: FloorSaveDto,
    content: &ContentCatalog,
) -> Result<FloorState, CoreError> {
    let revealed_terrain = revealed_terrain_from_save(
        floor.revealed_terrain,
        &floor.terrain.terrain_ids,
        floor.terrain.width,
        floor.terrain.height,
        content,
    )?;
    let connections =
        floor_connections_from_save(floor.connections, floor.terrain.width, floor.terrain.height)?;
    let regions = floor_regions_from_save(
        floor.regions,
        floor.terrain.width,
        floor.terrain.height,
        content,
    )?;
    let entities = floor
        .entities
        .into_iter()
        .map(|entity| actor_from_entity(entity, content))
        .collect::<Result<Vec<_>, CoreError>>()?;
    let mut items = floor
        .items
        .into_iter()
        .map(item_from_dto)
        .collect::<Vec<_>>();
    items.extend(
        floor
            .carried_items
            .into_iter()
            .map(|item| carried_item_from_dto(item, content))
            .collect::<Result<Vec<_>, CoreError>>()?,
    );
    Ok(FloorState {
        id: floor.id,
        width: floor.terrain.width,
        height: floor.terrain.height,
        terrain: floor.terrain.terrain_ids,
        player_position: floor.player_position,
        entities,
        items,
        explored: floor.explored,
        revealed_terrain,
        connections,
        regions,
    })
}

pub(crate) fn floor_regions_to_save(regions: &[FloorRegionState]) -> Vec<FloorRegionSaveDto> {
    let mut regions = regions
        .iter()
        .map(|region| FloorRegionSaveDto {
            region_id: region.region_id.clone(),
            theme_id: region.theme_id.clone(),
            encounter_table_id: region.encounter_table_id.clone(),
            loot_table_id: region.loot_table_id.clone(),
            cells: region.cells.clone(),
        })
        .collect::<Vec<_>>();
    regions.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    for region in &mut regions {
        region.cells.sort();
    }
    regions
}

pub(crate) fn floor_regions_from_save(
    regions: Vec<FloorRegionSaveDto>,
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> Result<Vec<FloorRegionState>, CoreError> {
    let mut restored = regions
        .into_iter()
        .map(|mut region| {
            region.cells.sort();
            if !valid_rule_id(&region.region_id)
                || !valid_rule_id(&region.theme_id)
                || !valid_rule_id(&region.encounter_table_id)
                || !valid_rule_id(&region.loot_table_id)
                || region.cells.is_empty()
                || content
                    .encounter_table(&region.encounter_table_id)
                    .is_none()
                || content.loot_table(&region.loot_table_id).is_none()
                || region.cells.windows(2).any(|pair| pair[0] == pair[1])
                || region.cells.iter().any(|position| {
                    position.x < 0
                        || position.y < 0
                        || position.x >= i32::from(width)
                        || position.y >= i32::from(height)
                })
            {
                return Err(CoreError::InvalidSave("floor region state is invalid"));
            }
            Ok(FloorRegionState {
                region_id: region.region_id,
                theme_id: region.theme_id,
                encounter_table_id: region.encounter_table_id,
                loot_table_id: region.loot_table_id,
                cells: region.cells,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    restored.sort_by(|left, right| left.region_id.cmp(&right.region_id));
    let mut occupied = BTreeSet::new();
    if restored
        .windows(2)
        .any(|pair| pair[0].region_id == pair[1].region_id)
        || restored
            .iter()
            .flat_map(|region| region.cells.iter().copied())
            .any(|position| !occupied.insert(position))
    {
        return Err(CoreError::InvalidSave("floor region state is invalid"));
    }
    Ok(restored)
}

pub(crate) fn floor_connections_to_save(
    connections: &[FloorConnectionState],
) -> Vec<FloorConnectionSaveDto> {
    let mut connections = connections
        .iter()
        .map(|connection| FloorConnectionSaveDto {
            id: connection.id.clone(),
            position: connection.position,
        })
        .collect::<Vec<_>>();
    connections.sort_by(|left, right| left.id.cmp(&right.id));
    connections
}

pub(crate) fn floor_connections_from_save(
    connections: Vec<FloorConnectionSaveDto>,
    width: u16,
    height: u16,
) -> Result<Vec<FloorConnectionState>, CoreError> {
    let mut restored = connections
        .into_iter()
        .map(|connection| {
            if !valid_rule_id(&connection.id)
                || connection.position.x < 0
                || connection.position.y < 0
                || connection.position.x >= i32::from(width)
                || connection.position.y >= i32::from(height)
            {
                return Err(CoreError::InvalidSave("floor connection state is invalid"));
            }
            Ok(FloorConnectionState {
                id: connection.id,
                position: connection.position,
            })
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    restored.sort_by(|left, right| left.id.cmp(&right.id));
    let unique_positions = restored
        .iter()
        .map(|connection| connection.position)
        .collect::<BTreeSet<_>>();
    if restored.windows(2).any(|pair| pair[0].id == pair[1].id)
        || unique_positions.len() != restored.len()
    {
        return Err(CoreError::InvalidSave("floor connection state is invalid"));
    }
    Ok(restored)
}

pub(crate) fn revealed_terrain_from_save(
    positions: Vec<Position>,
    terrain: &[String],
    width: u16,
    height: u16,
    content: &ContentCatalog,
) -> Result<BTreeSet<Position>, CoreError> {
    let mut revealed = BTreeSet::new();
    for position in positions {
        if position.x < 0
            || position.y < 0
            || position.x >= i32::from(width)
            || position.y >= i32::from(height)
            || !revealed.insert(position)
        {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
        let index = position.y as usize * usize::from(width) + position.x as usize;
        let Some(definition) = terrain
            .get(index)
            .and_then(|terrain_id| content.terrain(terrain_id))
        else {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        };
        if definition.concealed_as_terrain_id.is_none() {
            return Err(CoreError::InvalidSave(
                "revealed terrain knowledge is invalid",
            ));
        }
    }
    Ok(revealed)
}
