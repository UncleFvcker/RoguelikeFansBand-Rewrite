// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{
    ContentCatalog, ShopCategory, ShopDefinition, ShopStockDefinition, TownDefinition,
    TownFacilityCategory, TownFacilityDefinition, WildernessLocationDefinition, WorldDefinition,
};
use rfb_protocol::{
    AbilityTownTargetDto, HomeDto, HomeItemDto, HomeStateSaveDto, InnTravelDestinationDto,
    ItemEnchantmentsDto, ItemIdentifyResolutionDto, ItemQualityDto, MapScaleDto, Position,
    ShopCategoryDto, ShopDto, ShopOwnerDto, ShopSellQuoteDto, ShopStateSaveDto, ShopStockItemDto,
    TownDto, TownStateSaveDto,
};

use crate::{
    effect::{STATUS_BLEEDING, STATUS_POISON},
    error::CoreError,
    rng::RfbRng,
    save::position_from_content,
    state::{HomeState, ItemInstance, ItemLocation, ShopState, TownState},
    stats::AttributeKind,
};

use super::{
    Game, initial_item_curse, initial_item_runtime_state,
    inventory::{
        ItemIdentificationRequest, item_instances_stack_compatible, item_properties_match,
    },
    normalize_player_name, wilderness,
};
use crate::save::{
    GENERATED_ITEM_ID_PREFIX, initial_item_fuel, inventory_item_from_dto, inventory_to_save,
};

const CHARISMA_PRICE_ADJUST_PERCENT: [u16; 38] = [
    130, 125, 122, 120, 118, 116, 114, 112, 110, 108, 106, 104, 103, 102, 101, 100, 99, 98, 97, 96,
    95, 94, 93, 92, 91, 90, 89, 88, 87, 86, 85, 84, 83, 82, 81, 80, 79, 78,
];

pub(super) type TownAndShopStates = (BTreeMap<String, TownState>, BTreeMap<String, ShopState>);

const INN_TRAVEL_COST: u32 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InnStayOutcome {
    pub(crate) facility_id: String,
    pub(crate) cost: u32,
    pub(crate) gold_balance: u32,
    pub(crate) elapsed_ticks: u32,
    pub(crate) world_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InnTravelOutcome {
    pub(crate) facility_id: String,
    pub(crate) destination_town_id: String,
    pub(crate) cost: u32,
    pub(crate) gold_balance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FacilityIdentifyOutcome {
    pub(crate) facility_id: String,
    pub(crate) cost: u32,
    pub(crate) gold_balance: u32,
    pub(crate) resolution: ItemIdentifyResolutionDto,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FacilityRenameOutcome {
    pub(crate) facility_id: String,
    pub(crate) previous_name: String,
    pub(crate) name: String,
    pub(crate) cost: u32,
    pub(crate) gold_balance: u32,
}

fn world_town_ids(world: &WorldDefinition) -> impl Iterator<Item = &str> {
    world
        .wilderness
        .iter()
        .flat_map(|wilderness| &wilderness.locations)
        .filter_map(|location| match location {
            WildernessLocationDefinition::Town { town_id, .. } => Some(town_id.as_str()),
            WildernessLocationDefinition::Dungeon { .. } => None,
        })
}

pub(super) fn world_town_for_floor<'a>(
    world: &WorldDefinition,
    content: &'a ContentCatalog,
    floor_id: &str,
) -> Option<&'a TownDefinition> {
    world_town_ids(world)
        .filter_map(|town_id| content.town(town_id))
        .find(|town| town.floor_id == floor_id)
}

fn world_town_at_position<'a>(
    world: &WorldDefinition,
    content: &'a ContentCatalog,
    position: Position,
) -> Option<&'a TownDefinition> {
    world
        .wilderness
        .as_ref()?
        .locations
        .iter()
        .find_map(|location| match location {
            WildernessLocationDefinition::Town {
                position: candidate,
                town_id,
                ..
            } if position_from_content(*candidate) == position => content.town(town_id),
            WildernessLocationDefinition::Town { .. }
            | WildernessLocationDefinition::Dungeon { .. } => None,
        })
}

fn world_town_position(world: &WorldDefinition, town_id: &str) -> Option<Position> {
    world
        .wilderness
        .as_ref()?
        .locations
        .iter()
        .find_map(|location| match location {
            WildernessLocationDefinition::Town {
                position,
                town_id: candidate,
                ..
            } if candidate == town_id => Some(position_from_content(*position)),
            WildernessLocationDefinition::Town { .. }
            | WildernessLocationDefinition::Dungeon { .. } => None,
        })
}

fn town_inn<'a>(town: &TownDefinition, content: &'a ContentCatalog) -> Option<&'a ShopDefinition> {
    town.shop_ids
        .iter()
        .filter_map(|shop_id| content.shop(shop_id))
        .find(|shop| shop.inn_stay_cost.is_some())
}

fn home_facilities<'a>(
    town: &'a TownDefinition,
    content: &'a ContentCatalog,
) -> impl Iterator<Item = &'a TownFacilityDefinition> {
    town.facility_ids.iter().filter_map(|facility_id| {
        content
            .town_facility(facility_id)
            .filter(|facility| facility.category == TownFacilityCategory::Home)
    })
}

fn home_storage_id<'a>(content: &'a ContentCatalog, facility_id: &str) -> Option<&'a str> {
    content
        .town_facility(facility_id)
        .filter(|facility| facility.category == TownFacilityCategory::Home)
        .and_then(|facility| facility.storage_id.as_deref())
}

fn home_storage_ids<'a>(
    town: &'a TownDefinition,
    content: &'a ContentCatalog,
) -> impl Iterator<Item = &'a str> {
    home_facilities(town, content).map(|facility| {
        facility
            .storage_id
            .as_deref()
            .expect("validated Home must retain a storage id")
    })
}

pub(super) fn initial_home_states(
    world: &WorldDefinition,
    content: &ContentCatalog,
) -> BTreeMap<String, HomeState> {
    let Some(town) = world.town_id.as_deref().and_then(|id| content.town(id)) else {
        return BTreeMap::new();
    };
    home_storage_ids(town, content)
        .map(|storage_id| {
            (
                storage_id.to_owned(),
                HomeState {
                    visited: false,
                    inventory: Vec::new(),
                },
            )
        })
        .collect()
}

pub(super) fn home_state_to_save(facility_id: &str, state: &HomeState) -> HomeStateSaveDto {
    let mut inventory = state.inventory.clone();
    for item in &mut inventory {
        item.location = ItemLocation::Inventory;
    }
    HomeStateSaveDto {
        facility_id: facility_id.to_owned(),
        visited: state.visited,
        inventory: inventory_to_save(&inventory),
    }
}

pub(super) fn restore_home_states(
    world: &WorldDefinition,
    content: &ContentCatalog,
    town_states: &BTreeMap<String, TownState>,
    current_floor_id: &str,
    player_position: Position,
    saved_homes: &[HomeStateSaveDto],
) -> Result<BTreeMap<String, HomeState>, CoreError> {
    let expected = world_town_ids(world)
        .filter(|town_id| town_states.contains_key(*town_id))
        .filter_map(|town_id| content.town(town_id))
        .flat_map(|town| home_storage_ids(town, content))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let mut states = BTreeMap::new();
    for saved in saved_homes {
        let Some(storage) = content.town_facility(&saved.facility_id) else {
            return Err(CoreError::InvalidSave("home state is invalid"));
        };
        let mut inventory = saved
            .inventory
            .iter()
            .cloned()
            .map(|item| inventory_item_from_dto(item, content))
            .collect::<Result<Vec<_>, _>>()?;
        for item in &mut inventory {
            item.location = ItemLocation::Home {
                facility_id: saved.facility_id.clone(),
            };
        }
        let player_at_shared_home = world_town_for_floor(world, content, current_floor_id)
            .into_iter()
            .flat_map(|town| home_facilities(town, content))
            .any(|facility| {
                facility.storage_id.as_deref() == Some(saved.facility_id.as_str())
                    && player_position == position_from_content(facility.entrance_position)
            });
        if !expected.contains(&saved.facility_id)
            || storage.category != TownFacilityCategory::Home
            || storage.storage_id.as_deref() != Some(storage.id.as_str())
            || (player_at_shared_home && !saved.visited)
            || states
                .insert(
                    saved.facility_id.clone(),
                    HomeState {
                        visited: saved.visited,
                        inventory,
                    },
                )
                .is_some()
        {
            return Err(CoreError::InvalidSave("home state is invalid"));
        }
    }
    if states.len() != expected.len() {
        return Err(CoreError::InvalidSave("home state is invalid"));
    }
    Ok(states)
}

pub(super) fn shop_state_to_save(shop_id: &str, state: &ShopState) -> ShopStateSaveDto {
    let mut inventory = state.inventory.clone();
    for item in &mut inventory {
        item.location = ItemLocation::Inventory;
    }
    ShopStateSaveDto {
        shop_id: shop_id.to_owned(),
        visited: state.visited,
        owner_id: state.owner_id.clone(),
        last_maintenance_world_tick: state.last_maintenance_world_tick,
        inventory: inventory_to_save(&inventory),
    }
}

pub(super) fn initial_town_and_shop_states(
    world: &WorldDefinition,
    content: &ContentCatalog,
    rng: &mut RfbRng,
    next_item_instance_serial: &mut u64,
) -> Result<TownAndShopStates, CoreError> {
    let Some(town_id) = &world.town_id else {
        return Ok((BTreeMap::new(), BTreeMap::new()));
    };
    let town_states = BTreeMap::from([(town_id.clone(), TownState { visited: true })]);
    let town = content
        .town(town_id)
        .expect("validated world town must remain available");
    let mut shop_states = BTreeMap::new();
    for shop_id in &town.shop_ids {
        let shop = content
            .shop(shop_id)
            .expect("validated town shop must remain available");
        let inventory = roll_shop_stock(shop, content, rng, next_item_instance_serial, false)?;
        shop_states.insert(
            shop_id.clone(),
            ShopState {
                visited: false,
                owner_id: shop.owner.id.clone(),
                inventory,
                last_maintenance_world_tick: 0,
            },
        );
    }
    Ok((town_states, shop_states))
}

pub(super) fn restore_town_and_shop_states(
    world: &WorldDefinition,
    content: &ContentCatalog,
    current_floor_id: &str,
    player_position: Position,
    saved_towns: &[TownStateSaveDto],
    saved_shops: &[ShopStateSaveDto],
) -> Result<TownAndShopStates, CoreError> {
    let formal_town_ids = world_town_ids(world).collect::<BTreeSet<_>>();
    if formal_town_ids.is_empty() {
        return (saved_towns.is_empty() && saved_shops.is_empty())
            .then(|| (BTreeMap::new(), BTreeMap::new()))
            .ok_or(CoreError::InvalidSave("town state is invalid"));
    }
    let mut town_states = BTreeMap::new();
    for saved in saved_towns {
        if !formal_town_ids.contains(saved.town_id.as_str())
            || !saved.visited
            || town_states
                .insert(saved.town_id.clone(), TownState { visited: true })
                .is_some()
        {
            return Err(CoreError::InvalidSave("town state is invalid"));
        }
    }
    let birth_town_id = world
        .town_id
        .as_deref()
        .expect("validated town world must retain a birth town");
    if !town_states.contains_key(birth_town_id)
        || world_town_for_floor(world, content, current_floor_id)
            .is_some_and(|town| !town_states.contains_key(&town.id))
    {
        return Err(CoreError::InvalidSave("town state is invalid"));
    }

    let mut shop_states = BTreeMap::new();
    for saved in saved_shops {
        let Some(shop) = content.shop(&saved.shop_id) else {
            return Err(CoreError::InvalidSave("shop state is invalid"));
        };
        if !formal_town_ids.contains(shop.town_id.as_str())
            || !town_states.contains_key(&shop.town_id)
            || shop_states
                .insert(saved.shop_id.clone(), restore_shop_state(saved, content)?)
                .is_some()
        {
            return Err(CoreError::InvalidSave("shop state is invalid"));
        }
    }
    for (shop_id, state) in &shop_states {
        let shop = content
            .shop(shop_id)
            .expect("validated town shop must remain available");
        let town = content
            .town(&shop.town_id)
            .expect("validated shop town must remain available");
        if state.owner_id != shop.owner.id
            || (current_floor_id == town.floor_id
                && player_position == position_from_content(shop.entrance_position)
                && !state.visited)
        {
            return Err(CoreError::InvalidSave("shop state is invalid"));
        }
    }
    if let Some(town) = world_town_for_floor(world, content, current_floor_id)
        && town.shop_ids.iter().any(|shop_id| {
            let shop = content
                .shop(shop_id)
                .expect("validated town shop must remain available");
            player_position == position_from_content(shop.entrance_position)
                && !shop_states.get(shop_id).is_some_and(|state| state.visited)
        })
    {
        return Err(CoreError::InvalidSave("shop state is invalid"));
    }
    Ok((town_states, shop_states))
}

fn restore_shop_state(
    saved: &ShopStateSaveDto,
    content: &ContentCatalog,
) -> Result<ShopState, CoreError> {
    let mut inventory = saved
        .inventory
        .iter()
        .cloned()
        .map(|item| inventory_item_from_dto(item, content))
        .collect::<Result<Vec<_>, _>>()?;
    for item in &mut inventory {
        item.location = ItemLocation::Shop {
            shop_id: saved.shop_id.clone(),
        };
    }
    Ok(ShopState {
        visited: saved.visited,
        owner_id: saved.owner_id.clone(),
        inventory,
        last_maintenance_world_tick: saved.last_maintenance_world_tick,
    })
}

fn roll_quantity(rng: &mut RfbRng, minimum: u32, maximum: u32) -> u32 {
    minimum
        + u32::try_from(rng.bounded(u64::from(maximum - minimum) + 1))
            .expect("bounded shop quantity must fit u32")
}

fn allocate_shop_item_id(next_serial: &mut u64) -> Result<String, CoreError> {
    let serial = *next_serial;
    *next_serial = serial.checked_add(1).ok_or(CoreError::ItemIdExhausted)?;
    Ok(format!("{GENERATED_ITEM_ID_PREFIX}{serial}"))
}

fn plain_shop_item(
    shop_id: &str,
    item_kind_id: &str,
    quantity: u32,
    content: &ContentCatalog,
    rng: &mut RfbRng,
    next_serial: &mut u64,
) -> Result<ItemInstance, CoreError> {
    let (activation, charges) = initial_item_runtime_state(content, rng, item_kind_id, 15);
    Ok(ItemInstance {
        id: allocate_shop_item_id(next_serial)?,
        kind_id: item_kind_id.to_owned(),
        quantity,
        inscription: None,
        origin_actor_kind_id: None,
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: ItemEnchantmentsDto::default(),
        curse: initial_item_curse(content, item_kind_id),
        activation,
        charges,
        fuel: initial_item_fuel(content, item_kind_id),
        device_recovery_progress: 0,
        captured_actor: None,
        location: ItemLocation::Shop {
            shop_id: shop_id.to_owned(),
        },
    })
}

fn append_plain_stock(
    inventory: &mut Vec<ItemInstance>,
    shop: &ShopDefinition,
    stock: &ShopStockDefinition,
    quantity: u32,
    content: &ContentCatalog,
    rng: &mut RfbRng,
    next_serial: &mut u64,
) -> Result<(), CoreError> {
    let definition = content
        .item(&stock.item_kind_id)
        .expect("validated shop stock must remain available");
    let mut remaining = quantity;
    while remaining > 0 {
        let stacked = remaining.min(definition.max_stack);
        inventory.push(plain_shop_item(
            &shop.id,
            &stock.item_kind_id,
            stacked,
            content,
            rng,
            next_serial,
        )?);
        remaining -= stacked;
    }
    Ok(())
}

fn roll_shop_stock(
    shop: &ShopDefinition,
    content: &ContentCatalog,
    rng: &mut RfbRng,
    next_serial: &mut u64,
    maintenance: bool,
) -> Result<Vec<ItemInstance>, CoreError> {
    let mut inventory = Vec::new();
    for stock in &shop.stock {
        if stock.availability_percent < 100
            && rng.bounded(100) >= u64::from(stock.availability_percent)
        {
            continue;
        }
        let (minimum, maximum) = if maintenance {
            (stock.maintenance_minimum, stock.maintenance_maximum)
        } else {
            (stock.initial_minimum, stock.initial_maximum)
        };
        let quantity = roll_quantity(rng, minimum, maximum);
        append_plain_stock(
            &mut inventory,
            shop,
            stock,
            quantity,
            content,
            rng,
            next_serial,
        )?;
    }
    inventory.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(inventory)
}

fn category_dto(category: ShopCategory) -> ShopCategoryDto {
    match category {
        ShopCategory::Shroomery => ShopCategoryDto::Shroomery,
        ShopCategory::GeneralStore => ShopCategoryDto::GeneralStore,
        ShopCategory::Armoury => ShopCategoryDto::Armoury,
        ShopCategory::Weaponsmith => ShopCategoryDto::Weaponsmith,
        ShopCategory::Temple => ShopCategoryDto::Temple,
        ShopCategory::Alchemist => ShopCategoryDto::Alchemist,
        ShopCategory::MagicShop => ShopCategoryDto::MagicShop,
        ShopCategory::BlackMarket => ShopCategoryDto::BlackMarket,
        ShopCategory::Bookstore => ShopCategoryDto::Bookstore,
    }
}

fn round_percent(value: u32, percent: u16) -> u32 {
    u32::try_from((u64::from(value) * u64::from(percent) + 50) / 100).unwrap_or(u32::MAX)
}

pub(super) fn buy_unit_price(base_value: u32, factor: u16) -> u32 {
    round_percent(base_value, factor.max(100))
}

pub(super) fn sell_unit_price(base_value: u32, factor: u16, cap: u32) -> u32 {
    ((u64::from(base_value) * 100) / u64::from(factor.max(105)))
        .try_into()
        .unwrap_or(u32::MAX)
        .max(1)
        .min(cap)
}

fn discounted_item_base_value(item: &ItemInstance, base_value: u32) -> u32 {
    base_value.saturating_sub(base_value.saturating_mul(u32::from(item.discount_percent)) / 100)
}

fn player_purchase_unit_price(
    game: &Game,
    shop: &ShopDefinition,
    base_value: u32,
    factor: u16,
) -> u32 {
    let price = buy_unit_price(base_value, factor);
    if shop.category == ShopCategory::BlackMarket && !game.player_has_black_market_standard_prices()
    {
        price.saturating_mul(2)
    } else {
        price
    }
}

fn player_sale_unit_price(game: &Game, shop: &ShopDefinition, base_value: u32, factor: u16) -> u32 {
    if shop.category == ShopCategory::BlackMarket && !game.player_has_black_market_standard_prices()
    {
        ((u64::from(base_value) * 100) / u64::from(factor.max(105)) / 2)
            .try_into()
            .unwrap_or(u32::MAX)
            .max(1)
            .min(shop.owner.purchase_price_cap)
    } else {
        sell_unit_price(base_value, factor, shop.owner.purchase_price_cap)
    }
}

fn shop_price_factor(game: &Game, shop: &ShopDefinition) -> u16 {
    let charisma_index = usize::from(
        game.effective_player_attributes()
            .index(AttributeKind::Charisma),
    )
    .min(CHARISMA_PRICE_ADJUST_PERCENT.len() - 1);
    let charisma_adjust = CHARISMA_PRICE_ADJUST_PERCENT[charisma_index];
    let player_race = game.character_definitions().map(|(_, race, _, _)| race);
    let race_adjust = player_race.map_or(110, |race| race.shop_adjust_percent);
    let mut factor = round_percent(u32::from(race_adjust), charisma_adjust);
    factor = round_percent(factor, shop.owner.greed_percent);
    if player_race.is_some_and(|race| race.id == shop.owner.race_id) {
        factor = factor.saturating_mul(90) / 100;
    }
    u16::try_from(factor).unwrap_or(u16::MAX)
}

fn item_is_legal_for_shop(game: &Game, item: &ItemInstance) -> bool {
    game.content.item(&item.kind_id).is_some_and(|definition| {
        definition.base_value > 0
            && !definition
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "corpse" | "remains"))
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShopTransactionOutcome {
    pub(crate) shop_id: String,
    pub(crate) item_id: String,
    pub(crate) item_kind_id: String,
    pub(crate) quantity: u32,
    pub(crate) unit_price: u32,
    pub(crate) total_price: u32,
    pub(crate) gold_balance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HomeTransferOutcome {
    pub(crate) facility_id: String,
    pub(crate) item_id: String,
    pub(crate) item_kind_id: String,
    pub(crate) quantity: u32,
}

fn home_accessible(game: &Game, facility_id: &str) -> bool {
    let Some(facility) = game.content.town_facility(facility_id) else {
        return false;
    };
    let Some(storage_id) = home_storage_id(&game.content, facility_id) else {
        return false;
    };
    let Some(town) = game.content.town(&facility.town_id) else {
        return false;
    };
    game.current_town()
        .is_some_and(|current| current.id == town.id)
        && game.home_states.contains_key(storage_id)
        && game.town_local_to_active_position(
            &town.id,
            position_from_content(facility.entrance_position),
        ) == Some(game.player.position)
}

fn home_item_group(
    game: &Game,
    facility_id: &str,
    item_id: &str,
) -> Option<(ItemInstance, Vec<String>, u32)> {
    let storage_id = home_storage_id(&game.content, facility_id)?;
    let state = game.home_states.get(storage_id)?;
    let anchor = state
        .inventory
        .iter()
        .find(|item| item.id == item_id)?
        .clone();
    let anchor_knowledge = game.item_property_knowledge.get(&anchor.id);
    let mut items = state
        .inventory
        .iter()
        .filter(|item| {
            item_instances_stack_compatible(item, &anchor)
                && item_properties_match(
                    game.item_property_knowledge.get(&item.id),
                    anchor_knowledge,
                )
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    let quantity = items.iter().map(|item| item.quantity).sum();
    Some((
        anchor,
        items.into_iter().map(|item| item.id.clone()).collect(),
        quantity,
    ))
}

fn inventory_home_group(game: &Game, item_id: &str) -> Option<(ItemInstance, Vec<String>, u32)> {
    inventory_sale_group(game, item_id)
}

fn grouped_home_items<'a>(
    game: &'a Game,
    items: &'a [ItemInstance],
) -> Vec<(&'a ItemInstance, u32)> {
    let mut sorted = items.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    let mut groups: Vec<(&ItemInstance, u32)> = Vec::new();
    for item in sorted {
        let knowledge = game.item_property_knowledge.get(&item.id);
        if let Some((_, quantity)) = groups.iter_mut().find(|(anchor, _)| {
            item_instances_stack_compatible(anchor, item)
                && item_properties_match(game.item_property_knowledge.get(&anchor.id), knowledge)
        }) {
            *quantity = quantity.saturating_add(item.quantity);
        } else {
            groups.push((item, item.quantity));
        }
    }
    groups
}

fn grouped_inventory_for_home(game: &Game) -> Vec<(&ItemInstance, u32)> {
    let mut sorted = game
        .items
        .iter()
        .filter(|item| item.location == ItemLocation::Inventory)
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    let mut groups: Vec<(&ItemInstance, u32)> = Vec::new();
    for item in sorted {
        let knowledge = game.item_property_knowledge.get(&item.id);
        if let Some((_, quantity)) = groups.iter_mut().find(|(anchor, _)| {
            item_instances_stack_compatible(anchor, item)
                && item_properties_match(game.item_property_knowledge.get(&anchor.id), knowledge)
        }) {
            *quantity = quantity.saturating_add(item.quantity);
        } else {
            groups.push((item, item.quantity));
        }
    }
    groups
}

fn transfer_inventory_group_to_home(
    game: &mut Game,
    storage_id: &str,
    item_ids: &[String],
    quantity: u32,
    mut split_id: Option<String>,
) -> Vec<ItemInstance> {
    let mut remaining = quantity;
    let mut transferred = Vec::new();
    for item_id in item_ids {
        if remaining == 0 {
            break;
        }
        let index = game
            .items
            .iter()
            .position(|item| item.id == *item_id)
            .expect("preflighted inventory item must remain available");
        let moved = remaining.min(game.items[index].quantity);
        let mut item = if moved == game.items[index].quantity {
            game.items.remove(index)
        } else {
            let mut item = game.items[index].clone();
            game.items[index].quantity -= moved;
            item.id = split_id
                .take()
                .expect("partial home deposit must have a split id");
            if let Some(knowledge) = game.item_property_knowledge.get(item_id).cloned() {
                game.item_property_knowledge
                    .insert(item.id.clone(), knowledge);
            }
            item.quantity = moved;
            item
        };
        item.location = ItemLocation::Home {
            facility_id: storage_id.to_owned(),
        };
        transferred.push(item);
        remaining -= moved;
    }
    transferred
}

fn transfer_home_group_to_inventory(
    game: &mut Game,
    storage_id: &str,
    item_ids: &[String],
    quantity: u32,
    mut split_id: Option<String>,
) -> Vec<ItemInstance> {
    let state = game
        .home_states
        .get_mut(storage_id)
        .expect("preflighted home must remain available");
    let mut remaining = quantity;
    let mut transferred = Vec::new();
    for item_id in item_ids {
        if remaining == 0 {
            break;
        }
        let index = state
            .inventory
            .iter()
            .position(|item| item.id == *item_id)
            .expect("preflighted home item must remain available");
        let moved = remaining.min(state.inventory[index].quantity);
        let mut item = if moved == state.inventory[index].quantity {
            state.inventory.remove(index)
        } else {
            let mut item = state.inventory[index].clone();
            state.inventory[index].quantity -= moved;
            item.id = split_id
                .take()
                .expect("partial home withdrawal must have a split id");
            if let Some(knowledge) = game.item_property_knowledge.get(item_id).cloned() {
                game.item_property_knowledge
                    .insert(item.id.clone(), knowledge);
            }
            item.quantity = moved;
            item
        };
        item.location = ItemLocation::Inventory;
        transferred.push(item);
        remaining -= moved;
    }
    transferred
}

fn carry_home_withdrawal_item(game: &mut Game, mut item: ItemInstance) -> Vec<String> {
    let definition = game
        .content
        .item(&item.kind_id)
        .expect("home item kind must remain available");
    let source_knowledge = game.item_property_knowledge.get(&item.id).cloned();
    let mut stack_indices = game
        .items
        .iter()
        .enumerate()
        .filter(|(_, carried)| {
            carried.location == ItemLocation::Inventory
                && carried.quantity < definition.max_stack
                && item_instances_stack_compatible(carried, &item)
                && item_properties_match(
                    game.item_property_knowledge.get(&carried.id),
                    source_knowledge.as_ref(),
                )
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    stack_indices.sort_by(|left, right| game.items[*left].id.cmp(&game.items[*right].id));

    let mut destination_ids = Vec::new();
    for stack_index in stack_indices {
        let transferred = item
            .quantity
            .min(definition.max_stack - game.items[stack_index].quantity);
        if transferred == 0 {
            continue;
        }
        game.items[stack_index].quantity += transferred;
        item.quantity -= transferred;
        destination_ids.push(game.items[stack_index].id.clone());
        if item.quantity == 0 {
            break;
        }
    }
    if item.quantity > 0 {
        destination_ids.push(item.id.clone());
        game.items.push(item);
    } else {
        game.item_property_knowledge.remove(&item.id);
    }
    destination_ids
}

fn shop_accessible(game: &Game, shop: &ShopDefinition) -> bool {
    let Some(town) = game.content.town(&shop.town_id) else {
        return false;
    };
    game.current_town()
        .is_some_and(|current| current.id == town.id)
        && game.shop_states.contains_key(&shop.id)
        && game
            .town_local_to_active_position(&town.id, position_from_content(shop.entrance_position))
            == Some(game.player.position)
}

fn shop_purchase_group(
    game: &Game,
    shop_id: &str,
    item_id: &str,
) -> Option<(ItemInstance, Vec<String>, u32)> {
    let state = game.shop_states.get(shop_id)?;
    let anchor = state
        .inventory
        .iter()
        .find(|item| item.id == item_id)?
        .clone();
    let mut items = state
        .inventory
        .iter()
        .filter(|item| item_instances_stack_compatible(item, &anchor))
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    let quantity = items
        .iter()
        .fold(0_u32, |total, item| total.saturating_add(item.quantity));
    Some((
        anchor,
        items.into_iter().map(|item| item.id.clone()).collect(),
        quantity,
    ))
}

fn inventory_sale_group(game: &Game, item_id: &str) -> Option<(ItemInstance, Vec<String>, u32)> {
    let anchor = game
        .items
        .iter()
        .find(|item| item.id == item_id && item.location == ItemLocation::Inventory)?
        .clone();
    let anchor_knowledge = game.item_property_knowledge.get(&anchor.id);
    let mut items = game
        .items
        .iter()
        .filter(|item| {
            item.location == ItemLocation::Inventory
                && item_instances_stack_compatible(item, &anchor)
                && item_properties_match(
                    game.item_property_knowledge.get(&item.id),
                    anchor_knowledge,
                )
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    let quantity = items
        .iter()
        .fold(0_u32, |total, item| total.saturating_add(item.quantity));
    Some((
        anchor,
        items.into_iter().map(|item| item.id.clone()).collect(),
        quantity,
    ))
}

fn group_requires_split(items: &[ItemInstance], item_ids: &[String], quantity: u32) -> bool {
    let mut remaining = quantity;
    for item_id in item_ids {
        let item = items
            .iter()
            .find(|item| item.id == *item_id)
            .expect("preflighted grouped item must remain available");
        if remaining < item.quantity {
            return true;
        }
        remaining -= item.quantity;
        if remaining == 0 {
            break;
        }
    }
    false
}

fn grouped_shop_items(items: &[ItemInstance]) -> Vec<(&ItemInstance, u32)> {
    let mut sorted = items.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    let mut groups: Vec<(&ItemInstance, u32)> = Vec::new();
    for item in sorted {
        if let Some((_, quantity)) = groups
            .iter_mut()
            .find(|(anchor, _)| item_instances_stack_compatible(anchor, item))
        {
            *quantity = quantity.saturating_add(item.quantity);
        } else {
            groups.push((item, item.quantity));
        }
    }
    groups
}

fn grouped_inventory_items(game: &Game) -> Vec<(&ItemInstance, u32)> {
    let mut sorted = game
        .items
        .iter()
        .filter(|item| item.location == ItemLocation::Inventory)
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    let mut groups: Vec<(&ItemInstance, u32)> = Vec::new();
    for item in sorted {
        let legal = item_is_legal_for_shop(game, item);
        if legal {
            let knowledge = game.item_property_knowledge.get(&item.id);
            if let Some((_, quantity)) = groups.iter_mut().find(|(anchor, _)| {
                item_is_legal_for_shop(game, anchor)
                    && item_instances_stack_compatible(anchor, item)
                    && item_properties_match(
                        game.item_property_knowledge.get(&anchor.id),
                        knowledge,
                    )
            }) {
                *quantity = quantity.saturating_add(item.quantity);
                continue;
            }
        }
        groups.push((item, item.quantity));
    }
    groups
}

fn transfer_group_to_inventory(
    game: &mut Game,
    shop_id: &str,
    item_ids: &[String],
    quantity: u32,
    mut split_id: Option<String>,
) -> Vec<ItemInstance> {
    let state = game
        .shop_states
        .get_mut(shop_id)
        .expect("preflighted shop must remain available");
    let mut remaining = quantity;
    let mut transferred = Vec::new();
    for item_id in item_ids {
        if remaining == 0 {
            break;
        }
        let index = state
            .inventory
            .iter()
            .position(|item| item.id == *item_id)
            .expect("preflighted shop item must remain available");
        let moved = remaining.min(state.inventory[index].quantity);
        let mut item = if moved == state.inventory[index].quantity {
            state.inventory.remove(index)
        } else {
            let mut item = state.inventory[index].clone();
            state.inventory[index].quantity -= moved;
            item.id = split_id
                .take()
                .expect("partial grouped transfer must have a split id");
            item.quantity = moved;
            item
        };
        item.location = ItemLocation::Inventory;
        transferred.push(item);
        remaining -= moved;
    }
    debug_assert_eq!(remaining, 0);
    debug_assert!(split_id.is_none());
    transferred
}

fn transfer_group_to_shop(
    game: &mut Game,
    shop_id: &str,
    item_ids: &[String],
    quantity: u32,
    mut split_id: Option<String>,
) -> Vec<ItemInstance> {
    let mut remaining = quantity;
    let mut transferred = Vec::new();
    for item_id in item_ids {
        if remaining == 0 {
            break;
        }
        let index = game
            .items
            .iter()
            .position(|item| item.id == *item_id)
            .expect("preflighted inventory item must remain available");
        let moved = remaining.min(game.items[index].quantity);
        let mut item = if moved == game.items[index].quantity {
            let item = game.items.remove(index);
            game.item_property_knowledge.remove(&item.id);
            item
        } else {
            let mut item = game.items[index].clone();
            game.items[index].quantity -= moved;
            item.id = split_id
                .take()
                .expect("partial grouped transfer must have a split id");
            item.quantity = moved;
            item
        };
        item.location = ItemLocation::Shop {
            shop_id: shop_id.to_owned(),
        };
        transferred.push(item);
        remaining -= moved;
    }
    debug_assert_eq!(remaining, 0);
    debug_assert!(split_id.is_none());
    transferred
}

impl Game {
    pub(super) fn town_facility_accessible(&self, facility_id: &str) -> bool {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return false;
        };
        let Some(town) = self.current_town() else {
            return false;
        };
        facility.town_id == town.id
            && town.facility_ids.contains(&facility.id)
            && self.town_local_to_active_position(
                &town.id,
                position_from_content(facility.entrance_position),
            ) == Some(self.player.position)
    }

    pub(super) fn identify_at_facility(
        &mut self,
        facility_id: &str,
        item_id: &str,
    ) -> Result<FacilityIdentifyOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-facility");
        };
        let Some(cost) = facility.identify_item_cost else {
            return Err("service-unavailable");
        };
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let Some(item) = self.items.iter().find(|item| {
            item.id == item_id
                && item.quantity > 0
                && matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
        }) else {
            return Err("item-unavailable");
        };
        if self
            .item_property_knowledge
            .get(&item.id)
            .is_some_and(|knowledge| knowledge.appraised || knowledge.identified)
        {
            return Err("already-identified");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        let outcome = self.identify_item_instance(item_id, ItemIdentificationRequest::new(false));
        debug_assert!(outcome.changed);
        self.gold -= cost;
        Ok(FacilityIdentifyOutcome {
            facility_id: facility_id.to_owned(),
            cost,
            gold_balance: self.gold,
            resolution: ItemIdentifyResolutionDto {
                item_id: outcome.item_id,
                item_kind_id: outcome.item_kind_id,
                full: outcome.full,
                changed: outcome.changed,
            },
        })
    }

    pub(super) fn rename_at_facility(
        &mut self,
        facility_id: &str,
        name: &str,
    ) -> Result<FacilityRenameOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-facility");
        };
        let Some(cost) = facility.legal_name_change_cost else {
            return Err("service-unavailable");
        };
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let Some(name) = normalize_player_name(name) else {
            return Err("invalid-name");
        };
        if name == self.player_name {
            return Err("unchanged-name");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        let previous_name = std::mem::replace(&mut self.player_name, name.clone());
        self.gold -= cost;
        Ok(FacilityRenameOutcome {
            facility_id: facility_id.to_owned(),
            previous_name,
            name,
            cost,
            gold_balance: self.gold,
        })
    }

    pub(super) fn stay_at_inn(
        &mut self,
        facility_id: &str,
    ) -> Result<InnStayOutcome, &'static str> {
        let Some(inn) = self.content.shop(facility_id).cloned() else {
            return Err("unknown-inn");
        };
        let Some(cost) = inn.inn_stay_cost else {
            return Err("unknown-inn");
        };
        if !shop_accessible(self, &inn) {
            return Err("inn-unreachable");
        }
        if self.player_has_status_kind(STATUS_POISON)
            || self.player_has_status_kind(STATUS_BLEEDING)
        {
            return Err("needs-healer");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        self.gold -= cost;
        let before = self.world_tick;
        let half_day = wilderness::WILDERNESS_DAY_TICKS / 2;
        let remaining = half_day - before % half_day;
        self.world_tick = before.saturating_add(remaining);

        self.player.statuses.clear();
        self.minor_slow = 0;
        self.minor_slow_energy = 0;
        self.reality_change_ticks = 0;
        self.confusing_strike_ready = false;
        if self.recall_is_active() {
            self.cancel_recall();
        }
        self.refresh_player_resource_maxima();
        self.player.hp = self.effective_player_max_hp();
        for pool in self.resources.values_mut() {
            pool.current = pool.maximum;
        }
        for item in &mut self.items {
            if item.location != ItemLocation::Inventory {
                continue;
            }
            if let Some(charges) = item.charges.as_mut() {
                charges.current = charges.maximum;
                item.device_recovery_progress = 0;
            }
        }

        Ok(InnStayOutcome {
            facility_id: facility_id.to_owned(),
            cost,
            gold_balance: self.gold,
            elapsed_ticks: self.world_tick - before,
            world_tick: self.world_tick,
        })
    }

    pub(super) fn inn_travel_unavailable_reason(
        &self,
        facility_id: &str,
        destination_town_id: &str,
    ) -> Option<&'static str> {
        let Some(inn) = self.content.shop(facility_id) else {
            return Some("unknown-inn");
        };
        if inn.inn_stay_cost.is_none() {
            return Some("unknown-inn");
        }
        if !shop_accessible(self, inn) {
            return Some("inn-unreachable");
        }
        if inn.town_id == destination_town_id {
            return Some("already-here");
        }
        let Some(world) = self.content.world(&self.world_id) else {
            return Some("town-unvisited");
        };
        let Some(destination) = self.content.town(destination_town_id) else {
            return Some("town-unvisited");
        };
        if world_town_position(world, destination_town_id).is_none()
            || !self
                .town_states
                .get(destination_town_id)
                .is_some_and(|state| state.visited)
            || town_inn(destination, &self.content).is_none()
        {
            return Some("town-unvisited");
        }
        (self.gold < INN_TRAVEL_COST).then_some("insufficient-gold")
    }

    pub(super) fn travel_from_inn(
        &mut self,
        facility_id: &str,
        destination_town_id: &str,
    ) -> Result<InnTravelOutcome, CoreError> {
        debug_assert!(
            self.inn_travel_unavailable_reason(facility_id, destination_town_id)
                .is_none()
        );
        self.relocate_to_town(destination_town_id)?;
        self.gold -= INN_TRAVEL_COST;

        Ok(InnTravelOutcome {
            facility_id: facility_id.to_owned(),
            destination_town_id: destination_town_id.to_owned(),
            cost: INN_TRAVEL_COST,
            gold_balance: self.gold,
        })
    }

    pub(super) fn teleport_town_targets(&self) -> Vec<AbilityTownTargetDto> {
        if self.map_scale != MapScaleDto::Local || !self.is_wilderness_floor() {
            return Vec::new();
        }
        let current_town_id = self.current_town().map(|town| town.id.as_str());
        let Some(world) = self.content.world(&self.world_id) else {
            return Vec::new();
        };
        let mut targets = world_town_ids(world)
            .filter(|town_id| Some(*town_id) != current_town_id)
            .filter(|town_id| {
                self.town_states
                    .get(*town_id)
                    .is_some_and(|state| state.visited)
            })
            .filter_map(|town_id| {
                let town = self.content.town(town_id)?;
                (world_town_position(world, town_id).is_some()
                    && town_inn(town, &self.content).is_some())
                .then(|| AbilityTownTargetDto {
                    town_id: town.id.clone(),
                    town_name_key: town.name_key.clone(),
                })
            })
            .collect::<Vec<_>>();
        targets.sort_by(|left, right| left.town_id.cmp(&right.town_id));
        targets
    }

    pub(super) fn teleport_town_target_available(&self, town_id: &str) -> bool {
        self.teleport_town_targets()
            .iter()
            .any(|target| target.town_id == town_id)
    }

    pub(super) fn teleport_to_town(&mut self, town_id: &str) -> Result<(), CoreError> {
        debug_assert!(self.teleport_town_target_available(town_id));
        self.relocate_to_town(town_id)
    }

    fn relocate_to_town(&mut self, destination_town_id: &str) -> Result<(), CoreError> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must remain available");
        let destination_position = world_town_position(world, destination_town_id)
            .expect("validated town destination must remain available");
        let destination_town = self
            .content
            .town(destination_town_id)
            .expect("validated destination town must remain available");
        let destination_inn = town_inn(destination_town, &self.content)
            .expect("validated destination town must retain an inn");
        let destination_inn_position = position_from_content(destination_inn.entrance_position);

        let player_id = self.player.id.clone();
        let riding_actor_id = self.riding_actor_id.as_deref();
        let (mut followers, retained): (Vec<_>, Vec<_>) = std::mem::take(&mut self.entities)
            .into_iter()
            .partition(|actor| {
                actor.controller_id.as_deref() == Some(player_id.as_str())
                    && Some(actor.id.as_str()) != riding_actor_id
            });
        self.entities = retained;
        self.store_visible_town_states();
        self.wilderness_position = Some(destination_position);
        self.wilderness_view_offset = Position::default();
        let arrival = self
            .town_local_to_wilderness_view_position(destination_town_id, destination_inn_position)
            .ok_or(CoreError::InvalidSave("town destination is unavailable"))?;
        self.activate_wilderness_position(Some(arrival), false)?;
        self.mark_current_town_visited();
        self.mark_shop_visited_at_player()?;
        for follower in &mut followers {
            follower.position = self.player.position;
        }
        self.entities.extend(followers);
        self.entities.sort_by(|left, right| left.id.cmp(&right.id));
        if self.summon_command.guard_position.is_some() {
            self.summon_command.guard_position = Some(self.player.position);
        }
        self.world_travel_destination = None;
        Ok(())
    }

    pub(super) fn deposit_at_home(
        &mut self,
        facility_id: &str,
        item_id: &str,
        quantity: u32,
    ) -> Result<HomeTransferOutcome, &'static str> {
        let Some(storage_id) = home_storage_id(&self.content, facility_id).map(str::to_owned)
        else {
            return Err("unknown-home");
        };
        if !home_accessible(self, facility_id) {
            return Err("home-unreachable");
        }
        if quantity == 0 {
            return Err("invalid-quantity");
        }
        let Some((item, source_ids, available_quantity)) = inventory_home_group(self, item_id)
        else {
            return Err("item-unavailable");
        };
        if quantity > available_quantity {
            return Err("insufficient-quantity");
        }
        let split_required = group_requires_split(&self.items, &source_ids, quantity);
        let split_id = split_required
            .then(|| self.allocate_item_instance_id())
            .transpose()
            .map_err(|_| "item-id-exhausted")?;
        let deposited =
            transfer_inventory_group_to_home(self, &storage_id, &source_ids, quantity, split_id);
        let destination_id = deposited
            .first()
            .expect("successful deposit must have a destination")
            .id
            .clone();
        self.home_states
            .get_mut(&storage_id)
            .expect("preflighted home must remain available")
            .inventory
            .extend(deposited);
        Ok(HomeTransferOutcome {
            facility_id: facility_id.to_owned(),
            item_id: destination_id,
            item_kind_id: item.kind_id,
            quantity,
        })
    }

    pub(super) fn withdraw_from_home(
        &mut self,
        facility_id: &str,
        item_id: &str,
        quantity: u32,
    ) -> Result<HomeTransferOutcome, &'static str> {
        let Some(storage_id) = home_storage_id(&self.content, facility_id).map(str::to_owned)
        else {
            return Err("unknown-home");
        };
        if !home_accessible(self, facility_id) {
            return Err("home-unreachable");
        }
        if quantity == 0 {
            return Err("invalid-quantity");
        }
        let Some((item, source_ids, available_quantity)) =
            home_item_group(self, facility_id, item_id)
        else {
            return Err("item-unavailable");
        };
        if quantity > available_quantity {
            return Err("insufficient-quantity");
        }
        if self.inventory_quantity_capacity_for(&item, true) < quantity {
            return Err("inventory-full");
        }
        let split_required = group_requires_split(
            &self
                .home_states
                .get(&storage_id)
                .expect("preflighted home must remain available")
                .inventory,
            &source_ids,
            quantity,
        );
        let split_id = split_required
            .then(|| self.allocate_item_instance_id())
            .transpose()
            .map_err(|_| "item-id-exhausted")?;
        let withdrawn =
            transfer_home_group_to_inventory(self, &storage_id, &source_ids, quantity, split_id);
        let mut destination_ids = Vec::new();
        for item in withdrawn {
            destination_ids.extend(carry_home_withdrawal_item(self, item));
        }
        let destination_id = destination_ids
            .first()
            .expect("successful withdrawal must have a destination")
            .clone();
        Ok(HomeTransferOutcome {
            facility_id: facility_id.to_owned(),
            item_id: destination_id,
            item_kind_id: item.kind_id,
            quantity,
        })
    }

    pub(super) fn buy_from_shop(
        &mut self,
        shop_id: &str,
        item_id: &str,
        quantity: u32,
    ) -> Result<ShopTransactionOutcome, &'static str> {
        let Some(shop) = self.content.shop(shop_id).cloned() else {
            return Err("unknown-shop");
        };
        if !shop_accessible(self, &shop) {
            return Err("shop-unreachable");
        }
        if quantity == 0 {
            return Err("invalid-quantity");
        }
        let Some((item, source_ids, available_quantity)) =
            shop_purchase_group(self, shop_id, item_id)
        else {
            return Err("item-unavailable");
        };
        if quantity > available_quantity {
            return Err("insufficient-stock");
        }
        let item_kind_id = item.kind_id.clone();
        let definition = self
            .content
            .item(&item_kind_id)
            .expect("shop item kind must remain available");
        let unit_price = player_purchase_unit_price(
            self,
            &shop,
            discounted_item_base_value(&item, definition.base_value),
            shop_price_factor(self, &shop),
        );
        let Some(total_price) = unit_price.checked_mul(quantity) else {
            return Err("price-overflow");
        };
        if self.gold < total_price {
            return Err("insufficient-gold");
        }
        if self.inventory_quantity_capacity_for(&item, false) < quantity {
            return Err("inventory-full");
        }

        let split_required = group_requires_split(
            &self
                .shop_states
                .get(shop_id)
                .expect("preflighted shop must remain available")
                .inventory,
            &source_ids,
            quantity,
        );
        let split_id = split_required
            .then(|| self.allocate_item_instance_id())
            .transpose()
            .map_err(|_| "item-id-exhausted")?;
        let purchased = transfer_group_to_inventory(self, shop_id, &source_ids, quantity, split_id);
        let mut destination_ids = Vec::new();
        for item in purchased {
            destination_ids.extend(self.carry_shop_purchase_item(item));
        }
        let purchased_id = destination_ids
            .first()
            .expect("successful purchase must have a destination")
            .clone();
        self.gold -= total_price;
        self.mark_item_aware(&item_kind_id);
        destination_ids.sort();
        destination_ids.dedup();
        for destination_id in destination_ids {
            let purchased = self
                .items
                .iter()
                .find(|item| item.id == destination_id)
                .expect("purchased item must remain available");
            let known_affix_ids = purchased
                .affix_ids
                .iter()
                .cloned()
                .chain(
                    purchased
                        .rolled_affixes
                        .iter()
                        .map(|affix| affix.affix_id.clone()),
                )
                .collect::<Vec<_>>();
            let knowledge = self
                .item_property_knowledge
                .entry(destination_id)
                .or_default();
            knowledge.discovered = true;
            knowledge.appraised = true;
            knowledge.identified = true;
            knowledge.known_affix_ids.extend(known_affix_ids);
        }
        Ok(ShopTransactionOutcome {
            shop_id: shop_id.to_owned(),
            item_id: purchased_id,
            item_kind_id,
            quantity,
            unit_price,
            total_price,
            gold_balance: self.gold,
        })
    }

    pub(super) fn sell_to_shop(
        &mut self,
        shop_id: &str,
        item_id: &str,
        quantity: u32,
    ) -> Result<ShopTransactionOutcome, &'static str> {
        let Some(shop) = self.content.shop(shop_id).cloned() else {
            return Err("unknown-shop");
        };
        if !shop_accessible(self, &shop) {
            return Err("shop-unreachable");
        }
        if quantity == 0 {
            return Err("invalid-quantity");
        }
        let Some((item, source_ids, available_quantity)) = inventory_sale_group(self, item_id)
        else {
            return Err("item-unavailable");
        };
        if quantity > available_quantity {
            return Err("insufficient-quantity");
        }
        if !item_is_legal_for_shop(self, &item) {
            return Err("item-illegal");
        }
        let item_kind_id = item.kind_id.clone();
        let definition = self
            .content
            .item(&item_kind_id)
            .expect("inventory item kind must remain available");
        let unit_price = player_sale_unit_price(
            self,
            &shop,
            discounted_item_base_value(&item, definition.base_value),
            shop_price_factor(self, &shop),
        );
        let Some(total_price) = unit_price.checked_mul(quantity) else {
            return Err("price-overflow");
        };
        let Some(gold_balance) = self.gold.checked_add(total_price) else {
            return Err("gold-overflow");
        };
        if gold_balance > super::gold::MAX_PLAYER_GOLD {
            return Err("gold-overflow");
        }

        let split_required = group_requires_split(&self.items, &source_ids, quantity);
        let split_id = split_required
            .then(|| self.allocate_item_instance_id())
            .transpose()
            .map_err(|_| "item-id-exhausted")?;
        let sold = transfer_group_to_shop(self, shop_id, &source_ids, quantity, split_id);
        let sold_id = sold
            .first()
            .expect("successful sale must have a destination")
            .id
            .clone();
        self.shop_states
            .get_mut(shop_id)
            .expect("preflighted shop must remain available")
            .inventory
            .extend(sold);
        self.gold = gold_balance;
        Ok(ShopTransactionOutcome {
            shop_id: shop_id.to_owned(),
            item_id: sold_id,
            item_kind_id,
            quantity,
            unit_price,
            total_price,
            gold_balance,
        })
    }

    pub(super) fn maintain_shop_at_player(&mut self) -> Result<(), CoreError> {
        let Some(town) = self.current_town().cloned() else {
            return Ok(());
        };
        let Some(shop_id) = town
            .shop_ids
            .iter()
            .find(|shop_id| {
                self.content.shop(shop_id).is_some_and(|shop| {
                    self.town_local_to_active_position(
                        &town.id,
                        position_from_content(shop.entrance_position),
                    ) == Some(self.player.position)
                })
            })
            .cloned()
        else {
            return Ok(());
        };
        let shop = self
            .content
            .shop(&shop_id)
            .expect("validated town shop must remain available")
            .clone();
        let state = self
            .shop_states
            .get(&shop_id)
            .expect("validated shop state must remain available");
        if self
            .world_tick
            .saturating_sub(state.last_maintenance_world_tick)
            < shop.maintenance.interval_world_ticks
        {
            return Ok(());
        }
        let mut additions = Vec::new();
        for stock in &shop.stock {
            let current = state
                .inventory
                .iter()
                .filter(|item| item.kind_id == stock.item_kind_id)
                .map(|item| item.quantity)
                .sum::<u32>();
            let target = if stock.availability_percent < 100
                && self.rng.bounded(100) >= u64::from(stock.availability_percent)
            {
                0
            } else {
                roll_quantity(
                    &mut self.rng,
                    stock.maintenance_minimum,
                    stock.maintenance_maximum,
                )
            };
            if target > current {
                append_plain_stock(
                    &mut additions,
                    &shop,
                    stock,
                    target - current,
                    &self.content,
                    &mut self.rng,
                    &mut self.next_item_instance_serial,
                )?;
            }
        }
        let state = self
            .shop_states
            .get_mut(&shop_id)
            .expect("validated shop state must remain available");
        state.inventory.extend(additions);
        state.last_maintenance_world_tick = self.world_tick;
        Ok(())
    }
}

impl Game {
    pub(super) fn town_at_wilderness_position(
        &self,
        position: Position,
    ) -> Option<&TownDefinition> {
        let world = self.content.world(&self.world_id)?;
        world_town_at_position(world, &self.content, position)
    }

    pub(super) fn town_for_floor(&self, floor_id: &str) -> Option<&TownDefinition> {
        let world = self.content.world(&self.world_id)?;
        world_town_for_floor(world, &self.content, floor_id)
    }

    pub(super) fn town_local_to_active_position(
        &self,
        town_id: &str,
        position: Position,
    ) -> Option<Position> {
        if self.map_scale != MapScaleDto::Local {
            return None;
        }
        if self.is_wilderness_floor() {
            return self.town_local_to_wilderness_view_position(town_id, position);
        }
        self.content
            .town(town_id)
            .is_some_and(|town| town.floor_id == self.current_floor_id)
            .then_some(position)
    }

    pub(super) fn current_town(&self) -> Option<&TownDefinition> {
        if self.map_scale != MapScaleDto::Local {
            return None;
        }
        if self.is_wilderness_floor() {
            return self.town_at_wilderness_view_position(self.player.position);
        }
        self.town_for_floor(&self.current_floor_id)
    }

    pub(super) fn mark_current_town_visited(&mut self) {
        let Some(town) = self.current_town().cloned() else {
            return;
        };
        self.town_states
            .entry(town.id.clone())
            .or_insert(TownState { visited: true })
            .visited = true;
        for storage_id in home_storage_ids(&town, &self.content) {
            self.home_states
                .entry(storage_id.to_owned())
                .or_insert(HomeState {
                    visited: false,
                    inventory: Vec::new(),
                });
        }
    }

    pub(super) fn mark_shop_visited_at_player(&mut self) -> Result<(), CoreError> {
        let Some(town) = self.current_town().cloned() else {
            return Ok(());
        };
        for shop_id in &town.shop_ids {
            let shop = self
                .content
                .shop(shop_id)
                .expect("validated town shop must remain available")
                .clone();
            if self.town_local_to_active_position(
                &town.id,
                position_from_content(shop.entrance_position),
            ) != Some(self.player.position)
            {
                continue;
            }
            if !self.shop_states.contains_key(shop_id) {
                let inventory = roll_shop_stock(
                    &shop,
                    &self.content,
                    &mut self.rng,
                    &mut self.next_item_instance_serial,
                    false,
                )?;
                self.shop_states.insert(
                    shop_id.clone(),
                    ShopState {
                        visited: true,
                        owner_id: shop.owner.id.clone(),
                        inventory,
                        last_maintenance_world_tick: self.world_tick,
                    },
                );
            } else if let Some(state) = self.shop_states.get_mut(shop_id) {
                state.visited = true;
            }
        }
        for facility in home_facilities(&town, &self.content) {
            if self.town_local_to_active_position(
                &town.id,
                position_from_content(facility.entrance_position),
            ) == Some(self.player.position)
                && let Some(state) = self.home_states.get_mut(
                    facility
                        .storage_id
                        .as_deref()
                        .expect("validated Home must retain a storage id"),
                )
            {
                state.visited = true;
            }
        }
        Ok(())
    }

    pub(super) fn current_town_dto(&self) -> Option<TownDto> {
        let town = self.current_town()?;
        Some(TownDto {
            id: town.id.clone(),
            name_key: town.name_key.clone(),
            description_key: town.description_key.clone(),
            floor_id: town.floor_id.clone(),
            visited: self
                .town_states
                .get(&town.id)
                .is_some_and(|state| state.visited),
        })
    }

    pub(super) fn current_shop_dtos(&self) -> Vec<ShopDto> {
        let Some(town) = self.current_town() else {
            return Vec::new();
        };
        town.shop_ids
            .iter()
            .filter_map(|shop_id| self.content.shop(shop_id))
            .map(|shop| {
                let entrance_position = self
                    .town_local_to_active_position(
                        &town.id,
                        position_from_content(shop.entrance_position),
                    )
                    .expect("current town shop must retain an active position");
                let player_at_entrance = self.player.position == entrance_position;
                let inn_travel_destinations = if player_at_entrance && shop.inn_stay_cost.is_some()
                {
                    self.content
                        .world(&self.world_id)
                        .into_iter()
                        .flat_map(world_town_ids)
                        .filter(|town_id| *town_id != town.id)
                        .filter(|town_id| {
                            self.town_states
                                .get(*town_id)
                                .is_some_and(|state| state.visited)
                        })
                        .filter_map(|town_id| self.content.town(town_id))
                        .filter(|destination| town_inn(destination, &self.content).is_some())
                        .map(|destination| InnTravelDestinationDto {
                            town_id: destination.id.clone(),
                            town_name_key: destination.name_key.clone(),
                            cost: INN_TRAVEL_COST,
                        })
                        .collect()
                } else {
                    Vec::new()
                };
                let factor = shop_price_factor(self, shop);
                let state = self.shop_states.get(&shop.id);
                let mut stock = if player_at_entrance {
                    state
                        .map(|state| grouped_shop_items(&state.inventory))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(item, quantity)| {
                            let definition = self
                                .content
                                .item(&item.kind_id)
                                .expect("shop item kind must remain available");
                            let unit_price = player_purchase_unit_price(
                                self,
                                shop,
                                discounted_item_base_value(item, definition.base_value),
                                factor,
                            );
                            let affordable = self.gold / unit_price.max(1);
                            let slot_carryable = self.inventory_quantity_capacity_for(item, false);
                            ShopStockItemDto {
                                id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                display_name_key: definition.name_key.clone(),
                                quantity,
                                inscription: item.inscription.clone(),
                                captured_actor: self.captured_actor_dto(item),
                                maximum_quantity: quantity.min(affordable).min(slot_carryable),
                                unit_price,
                                weight_tenths_pound: definition.weight_tenths_pound,
                                fuel: item.fuel,
                                charges: item.charges,
                                activation: item.activation.clone(),
                                enchantments: item.enchantments,
                                curse: item.curse,
                                quality: item.quality,
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                stock.sort_by(|left, right| left.id.cmp(&right.id));
                let mut sell_quotes = if player_at_entrance {
                    grouped_inventory_items(self)
                        .into_iter()
                        .map(|(item, quantity)| {
                            let definition = self
                                .content
                                .item(&item.kind_id)
                                .expect("inventory item kind must remain available");
                            let unavailable_reason = (!item_is_legal_for_shop(self, item))
                                .then(|| "item-illegal".to_owned());
                            ShopSellQuoteDto {
                                item_id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                unit_price: unavailable_reason.as_ref().map_or_else(
                                    || {
                                        player_sale_unit_price(
                                            self,
                                            shop,
                                            discounted_item_base_value(item, definition.base_value),
                                            factor,
                                        )
                                    },
                                    |_| 0,
                                ),
                                maximum_quantity: if unavailable_reason.is_some() {
                                    0
                                } else {
                                    quantity
                                },
                                unavailable_reason,
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                sell_quotes.sort_by(|left, right| left.item_id.cmp(&right.item_id));
                ShopDto {
                    id: shop.id.clone(),
                    name_key: shop.name_key.clone(),
                    description_key: shop.description_key.clone(),
                    category: category_dto(shop.category),
                    entrance_position,
                    entrance_terrain_id: shop.entrance_terrain_id.clone(),
                    inn_stay_cost: shop.inn_stay_cost,
                    inn_travel_destinations,
                    visited: self
                        .shop_states
                        .get(&shop.id)
                        .is_some_and(|state| state.visited),
                    player_at_entrance,
                    owner: ShopOwnerDto {
                        id: shop.owner.id.clone(),
                        name_key: shop.owner.name_key.clone(),
                        race_id: shop.owner.race_id.clone(),
                        greed_percent: shop.owner.greed_percent,
                        purchase_price_cap: shop.owner.purchase_price_cap,
                        price_factor_percent: factor,
                    },
                    stock,
                    sell_quotes,
                }
            })
            .collect()
    }

    pub(super) fn current_home_dtos(&self) -> Vec<HomeDto> {
        let Some(town) = self.current_town() else {
            return Vec::new();
        };
        town.facility_ids
            .iter()
            .filter_map(|id| self.content.town_facility(id))
            .filter(|facility| facility.category == TownFacilityCategory::Home)
            .map(|facility| {
                let entrance_position = self
                    .town_local_to_active_position(
                        &town.id,
                        position_from_content(facility.entrance_position),
                    )
                    .expect("current town Home must retain an active position");
                let player_at_entrance = self.player.position == entrance_position;
                let state = facility
                    .storage_id
                    .as_deref()
                    .and_then(|storage_id| self.home_states.get(storage_id));
                let mut stored_items = if player_at_entrance {
                    state
                        .map(|state| grouped_home_items(self, &state.inventory))
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(item, quantity)| {
                            let definition = self
                                .content
                                .item(&item.kind_id)
                                .expect("home item kind must remain available");
                            let slot_carryable = self.inventory_quantity_capacity_for(item, true);
                            HomeItemDto {
                                id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                display_name_key: self.item_display_name_key(&item.kind_id),
                                quantity,
                                inscription: item.inscription.clone(),
                                captured_actor: self.captured_actor_dto(item),
                                maximum_quantity: quantity.min(slot_carryable),
                                weight_tenths_pound: definition.weight_tenths_pound,
                                fuel: item.fuel,
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                stored_items.sort_by(|left, right| left.id.cmp(&right.id));
                let mut deposit_items = if player_at_entrance {
                    grouped_inventory_for_home(self)
                        .into_iter()
                        .map(|(item, quantity)| {
                            let definition = self
                                .content
                                .item(&item.kind_id)
                                .expect("inventory item kind must remain available");
                            HomeItemDto {
                                id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                display_name_key: self.item_display_name_key(&item.kind_id),
                                quantity,
                                inscription: item.inscription.clone(),
                                captured_actor: self.captured_actor_dto(item),
                                maximum_quantity: quantity,
                                weight_tenths_pound: definition.weight_tenths_pound,
                                fuel: item.fuel,
                            }
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                deposit_items.sort_by(|left, right| left.id.cmp(&right.id));
                HomeDto {
                    id: facility.id.clone(),
                    name_key: facility.name_key.clone(),
                    description_key: facility.description_key.clone(),
                    entrance_position,
                    entrance_terrain_id: facility.entrance_terrain_id.clone(),
                    visited: state.is_some_and(|state| state.visited),
                    player_at_entrance,
                    stored_items,
                    deposit_items,
                }
            })
            .collect()
    }
}
