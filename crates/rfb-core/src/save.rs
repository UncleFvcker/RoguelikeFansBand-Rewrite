// SPDX-License-Identifier: MPL-2.0

use crate::{
    error::CoreError,
    state::{Actor, ItemInstance, ItemLocation},
};
use rfb_content::{ContentCatalog, ContentPosition};
use rfb_protocol::{
    ActorSaveDto, EquipmentItemSaveDto, InventoryItemSaveDto, ItemSaveDto, PlayerSaveDto, Position,
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
    Ok(Actor {
        id: player.id,
        kind_id: player.kind_id,
        position: player.position,
        hp: player.hp,
        max_hp: definition.max_hp,
        speed: player.base_speed,
        energy_need: player.energy_need,
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
    Ok(Actor {
        id: entity.id,
        kind_id: entity.kind_id,
        position: entity.position,
        hp: entity.hp,
        max_hp: definition.max_hp,
        speed: entity.base_speed,
        energy_need: entity.energy_need,
    })
}

pub(crate) fn item_from_dto(item: ItemSaveDto) -> ItemInstance {
    ItemInstance {
        id: item.id,
        kind_id: item.kind_id,
        quantity: item.quantity,
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
        location: ItemLocation::Equipped {
            slot_id: item.slot_id,
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
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    entities
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
