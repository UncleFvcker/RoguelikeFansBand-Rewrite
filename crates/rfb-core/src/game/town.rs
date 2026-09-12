// SPDX-License-Identifier: MPL-2.0

mod stock;

use std::collections::{BTreeMap, BTreeSet};

use rfb_content::{
    ContentCatalog, ShopCategory, ShopDefinition, ShopStockDefinition, TownDefinition,
    TownFacilityCategory, TownFacilityDefinition, TownFacilityPrice, TownFacilityServiceDefinition,
    TownFacilityServiceKind, WildernessLocationDefinition, WorldDefinition,
};
use rfb_protocol::{
    AbilityTownTargetDto, FacilityMembershipDto, FacilityServiceDto, FacilityServiceKindDto,
    FacilityServiceTargetDto, HomeDto, HomeItemDto, HomeStateSaveDto, InnTravelDestinationDto,
    ItemEnchantmentComponentResolutionDto, ItemEnchantmentResolutionDto, ItemEnchantmentsDto,
    ItemIdentifyResolutionDto, ItemQualityDto, MapScaleDto, Position, ShopCategoryDto, ShopDto,
    ShopOwnerDto, ShopSellQuoteDto, ShopStateSaveDto, ShopStockItemDto, TaskStatusKindDto, TownDto,
    TownStateSaveDto,
};

use crate::{
    combat::apply_melee_armor_reduction,
    effect::{STATUS_BLEEDING, STATUS_BLINDNESS, STATUS_CONFUSION, STATUS_POISON, STATUS_STUN},
    error::CoreError,
    event::DomainEvent,
    rng::RfbRng,
    save::position_from_content,
    state::{HomeState, ItemInstance, ItemLocation, ShopState, TownState},
    stats::AttributeKind,
};

use super::{
    Game, RecallUseAction, initial_item_curse, initial_item_runtime_state,
    inventory::{
        ItemEnchantmentRequest, ItemIdentificationRequest, item_instances_group_compatible,
        item_instances_stack_compatible, item_properties_match,
    },
    normalize_player_name, wilderness,
};
use crate::save::{
    GENERATED_ITEM_ID_PREFIX, initial_item_fuel, inventory_item_from_dto, inventory_to_save,
    item_destruction_element_to_dto,
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
pub(crate) struct FacilityIdentifyAllOutcome {
    pub(crate) facility_id: String,
    pub(crate) identified_count: usize,
    pub(crate) cost: u32,
    pub(crate) gold_balance: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FacilityServiceOutcome {
    Healed {
        facility_id: String,
        healed: i32,
        mount_healed: i32,
        statuses_removed: usize,
        cost: u32,
        gold_balance: u32,
    },
    VitalityRestored {
        facility_id: String,
        cost: u32,
        gold_balance: u32,
    },
    MutationCured {
        facility_id: String,
        mutation_id: String,
        cost: u32,
        gold_balance: u32,
    },
    BalanceRitualPerformed {
        facility_id: String,
        cost: u32,
        gold_balance: u32,
    },
    ItemEnchanted {
        facility_id: String,
        cost: u32,
        gold_balance: u32,
        resolution: ItemEnchantmentResolutionDto,
    },
    ArmorAssessed {
        facility_id: String,
        armor_class: i32,
        protection_percent: i32,
        cost: u32,
        gold_balance: u32,
    },
    RecallStarted {
        facility_id: String,
        dungeon_id: String,
        floor_id: String,
        cost: u32,
        gold_balance: u32,
    },
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
        if !expected.contains(&saved.facility_id)
            || storage.category != TownFacilityCategory::Home
            || storage.storage_id.as_deref() != Some(storage.id.as_str())
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
                && shop
                    .entrance_positions()
                    .any(|position| player_position == position_from_content(position))
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
            shop.entrance_positions()
                .any(|position| player_position == position_from_content(position))
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
    let (activation, charges) = initial_item_runtime_state(content, rng, item_kind_id, &[], 15);
    Ok(ItemInstance {
        previously_worn: false,
        book_counted: false,
        artifact_name: None,
        intrinsic_melee_damage_dice: None,
        intrinsic_weight_tenths_pound: None,
        intrinsic_weapon_traits: Default::default(),
        intrinsic_curse_effects: Default::default(),
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
        intrinsic_properties: Default::default(),
        enchantments: ItemEnchantmentsDto::default(),
        curse: initial_item_curse(content, item_kind_id),
        permanent_destruction_immunities: Default::default(),
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
        ShopCategory::Jeweler => ShopCategoryDto::Jeweler,
        ShopCategory::Dragon => ShopCategoryDto::Dragon,
    }
}

fn round_percent(value: u32, percent: u16) -> u32 {
    u32::try_from((u64::from(value) * u64::from(percent) + 50) / 100).unwrap_or(u32::MAX)
}

pub(super) fn buy_unit_price(base_value: u32, factor: u16) -> u32 {
    let price = if base_value > 1_000_000 {
        (base_value / 100).saturating_mul(u32::from(factor.max(100)))
    } else {
        round_percent(base_value, factor.max(100))
    };
    if price > 1000 {
        round_three_significant(price)
    } else {
        price
    }
}

fn round_three_significant(value: u32) -> u32 {
    let mut scale = 1_u64;
    while u64::from(value) / scale >= 1000 {
        scale *= 10;
    }
    u32::try_from(((u64::from(value) + scale / 2) / scale) * scale).unwrap_or(u32::MAX)
}

pub(super) fn sell_unit_price(base_value: u32, factor: u16, cap: u32) -> u32 {
    let price = if base_value > 1_000_000 {
        (base_value / u32::from(factor.max(105))).saturating_mul(100)
    } else {
        ((u64::from(base_value) * 100) / u64::from(factor.max(105))) as u32
    };
    (if price > 1000 {
        round_three_significant(price)
    } else {
        price
    })
    .max(1)
    .min(cap)
}

fn discounted_item_base_value(
    content: &ContentCatalog,
    shop: &ShopDefinition,
    item: &ItemInstance,
    base_value: u32,
) -> u32 {
    if item
        .affix_ids
        .iter()
        .any(|id| id == "rfb-legacy.affix.blasted")
    {
        return 0;
    }
    let source_equipment = content
        .item(&item.kind_id)
        .and_then(|definition| definition.rfb_base_kind)
        .is_some_and(|base| matches!(base.tval, 16..=23 | 30..=40 | 45 | 46));
    let base_value = if item.artifact_name.is_some()
        || (stock::generates_equipment(shop.category) && source_equipment)
    {
        super::item_value::obj_value_real(content, item)
            .expect("source equipment must retain supported COST_REAL inputs")
            .max(0) as u32
    } else {
        base_value
    };
    base_value.saturating_sub(base_value.saturating_mul(u32::from(item.discount_percent)) / 100)
}

fn player_purchase_unit_price(
    game: &Game,
    shop: &ShopDefinition,
    base_value: u32,
    factor: u16,
) -> u32 {
    // shop.c applies the dragon surcharge before the normal price factor and rounding.
    let base_value = if shop.category == ShopCategory::Dragon && base_value > 31_000 {
        let excess = u64::from(base_value - 30_000);
        u32::try_from(u64::from(base_value) + (excess / 200 * excess) / 15).unwrap_or(u32::MAX)
    } else {
        base_value
    };
    let mut price = buy_unit_price(base_value, factor);
    if shop.category == ShopCategory::BlackMarket {
        if !game.player_has_black_market_standard_prices() {
            price = price.saturating_mul(2);
        }
        price = (u64::from(price)
            * (625 + i64::from(game.virtue_current(rfb_protocol::VirtueKindDto::Justice))) as u64
            / 625)
            .try_into()
            .unwrap_or(u32::MAX);
    } else if shop.category == ShopCategory::Jeweler {
        price = price.saturating_mul(2);
    }
    price
}

fn player_sale_unit_price(game: &Game, shop: &ShopDefinition, base_value: u32, factor: u16) -> u32 {
    let mut price = sell_unit_price(base_value, factor, u32::MAX);
    if shop.category == ShopCategory::BlackMarket {
        if !game.player_has_black_market_standard_prices() {
            price /= 2;
        }
        price = (u64::from(price)
            * (625 - i64::from(game.virtue_current(rfb_protocol::VirtueKindDto::Justice))) as u64
            / 625)
            .try_into()
            .unwrap_or(u32::MAX);
    } else if shop.category == ShopCategory::Jeweler {
        price /= 2;
    }
    price.max(1).min(shop.owner.purchase_price_cap)
}

fn shop_price_factor(game: &Game, shop: &ShopDefinition) -> u16 {
    let mut factor = price_factor_aux(game, shop.owner.greed_percent);
    if game
        .character_definitions()
        .is_some_and(|(_, race, _, _)| race.id == shop.owner.race_id)
    {
        factor = factor.saturating_mul(90) / 100;
    }
    factor
}

fn price_factor_aux(game: &Game, greed: u16) -> u16 {
    let charisma_index = usize::from(
        game.effective_player_attributes()
            .index(AttributeKind::Charisma),
    )
    .min(CHARISMA_PRICE_ADJUST_PERCENT.len() - 1);
    let charisma_adjust = CHARISMA_PRICE_ADJUST_PERCENT[charisma_index];
    let player_race = game.character_definitions().map(|(_, race, _, _)| race);
    let race_adjust = player_race.map_or(110, |race| race.shop_adjust_percent);
    let race_adjust = if race_adjust == 0 { 110 } else { race_adjust };
    let mut factor = round_percent(u32::from(race_adjust), charisma_adjust);
    factor = round_percent(factor, 135 - game.fame.min(200) / 4);
    factor = round_percent(factor, greed);
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

fn shop_accepts_item(game: &Game, shop: &ShopDefinition, item: &ItemInstance) -> bool {
    item_is_legal_for_shop(game, item)
        && (!stock::generates_equipment(shop.category)
            || discounted_item_base_value(
                &game.content,
                shop,
                item,
                game.content.item(&item.kind_id).unwrap().base_value,
            ) > 0)
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
        && game.town_facility_accessible(facility_id)
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
            (item.id == anchor.id || item_instances_group_compatible(&game.content, item, &anchor))
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
            item_instances_group_compatible(&game.content, anchor, item)
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
            item_instances_group_compatible(&game.content, anchor, item)
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
    super::inventory::record_book_found(&game.content, &mut game.item_knowledge, &mut item);
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
                && item_instances_stack_compatible(&game.content, carried, &item)
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
        super::inventory::merge_item_stack(&mut game.items[stack_index], &item, transferred);
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
        && game.shop_entrance_position(shop) == Some(game.player.position)
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
        .filter(|item| {
            item.id == anchor.id || item_instances_group_compatible(&game.content, item, &anchor)
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
                && (item.id == anchor.id
                    || item_instances_group_compatible(&game.content, item, &anchor))
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

fn grouped_shop_items<'a>(
    content: &ContentCatalog,
    items: &'a [ItemInstance],
) -> Vec<(&'a ItemInstance, u32)> {
    let mut sorted = items.iter().collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.id.cmp(&right.id));
    let mut groups: Vec<(&ItemInstance, u32)> = Vec::new();
    for item in sorted {
        if let Some((_, quantity)) = groups
            .iter_mut()
            .find(|(anchor, _)| item_instances_group_compatible(content, anchor, item))
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
                    && item_instances_group_compatible(&game.content, anchor, item)
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

const fn facility_service_kind_dto(kind: TownFacilityServiceKind) -> FacilityServiceKindDto {
    match kind {
        TownFacilityServiceKind::Heal => FacilityServiceKindDto::Heal,
        TownFacilityServiceKind::RestoreVitality => FacilityServiceKindDto::RestoreVitality,
        TownFacilityServiceKind::CureMutation => FacilityServiceKindDto::CureMutation,
        TownFacilityServiceKind::BalanceRitual => FacilityServiceKindDto::BalanceRitual,
        TownFacilityServiceKind::EnchantWeapon => FacilityServiceKindDto::EnchantWeapon,
        TownFacilityServiceKind::EnchantArmor => FacilityServiceKindDto::EnchantArmor,
        TownFacilityServiceKind::EnchantAmmunition => FacilityServiceKindDto::EnchantAmmunition,
        TownFacilityServiceKind::EnchantBow => FacilityServiceKindDto::EnchantBow,
        TownFacilityServiceKind::AssessArmor => FacilityServiceKindDto::AssessArmor,
        TownFacilityServiceKind::Recall => FacilityServiceKindDto::Recall,
    }
}

const fn enchantment_service(kind: FacilityServiceKindDto) -> bool {
    matches!(
        kind,
        FacilityServiceKindDto::EnchantWeapon
            | FacilityServiceKindDto::EnchantArmor
            | FacilityServiceKindDto::EnchantAmmunition
            | FacilityServiceKindDto::EnchantBow
    )
}

impl Game {
    pub(super) fn town_facility_membership(
        &self,
        facility: &TownFacilityDefinition,
    ) -> FacilityMembershipDto {
        let Some((build, race, class, _)) = self.character_definitions() else {
            return FacilityMembershipDto::Visitor;
        };
        let realms = [
            build.first_realm_id.as_deref(),
            self.current_second_realm_id(),
        ];
        let matches_role = |class_ids: &[String], race_ids: &[String], realm_ids: &[String]| {
            class_ids.contains(&class.id)
                || race_ids.contains(&race.id)
                || realms
                    .into_iter()
                    .flatten()
                    .any(|realm_id| realm_ids.iter().any(|candidate| candidate == realm_id))
        };
        if matches_role(
            &facility.owner_class_ids,
            &facility.owner_race_ids,
            &facility.owner_realm_ids,
        ) {
            FacilityMembershipDto::Owner
        } else if matches_role(
            &facility.member_class_ids,
            &facility.member_race_ids,
            &facility.member_realm_ids,
        ) {
            FacilityMembershipDto::Member
        } else {
            FacilityMembershipDto::Visitor
        }
    }

    pub(super) fn town_facility_price(
        &self,
        facility: &TownFacilityDefinition,
        price: TownFacilityPrice,
    ) -> u32 {
        self.town_service_price(
            if self.town_facility_membership(facility) == FacilityMembershipDto::Owner {
                price.owner_cost
            } else {
                price.other_cost
            },
        )
    }

    pub(super) fn town_service_price(&self, base: u32) -> u32 {
        buy_unit_price(base, price_factor_aux(self, 100))
    }

    fn town_facility_service_cost(
        &self,
        definition: TownFacilityServiceDefinition,
        membership: FacilityMembershipDto,
    ) -> u32 {
        let owner = membership == FacilityMembershipDto::Owner;
        let declared = if owner {
            definition.owner_cost
        } else {
            definition.other_cost
        };
        self.town_service_price(declared)
    }

    fn town_facility_enchantment_limit(&self, membership: FacilityMembershipDto) -> i16 {
        let base = if membership == FacilityMembershipDto::Owner {
            5
        } else {
            2
        };
        i16::try_from(base + self.progress.level / 5).unwrap_or(i16::MAX)
    }

    fn town_facility_enchantment_target(
        &self,
        item: &ItemInstance,
        service: FacilityServiceKindDto,
        limit: i16,
    ) -> bool {
        if item.quantity == 0
            || !matches!(
                item.location,
                ItemLocation::Inventory | ItemLocation::Equipped { .. }
            )
        {
            return false;
        }
        let Some(definition) = self.content.item(&item.kind_id) else {
            return false;
        };
        if self.item_resists_enchantment(item) {
            return false;
        }
        let eligible = match service {
            FacilityServiceKindDto::EnchantWeapon => definition.melee_profile.is_some(),
            FacilityServiceKindDto::EnchantArmor => {
                definition.tags.iter().any(|tag| tag == "armor")
            }
            FacilityServiceKindDto::EnchantAmmunition => definition.ammunition_profile.is_some(),
            FacilityServiceKindDto::EnchantBow => definition.projectile_profile.is_some(),
            _ => false,
        };
        if !eligible {
            return false;
        }
        let total = self.item_total_enchantments(item);
        if service == FacilityServiceKindDto::EnchantArmor {
            total.to_armor < limit
        } else {
            total.to_hit < limit || total.to_damage < limit
        }
    }

    fn facility_enchantment_choices(
        &self,
        item: &ItemInstance,
        service: FacilityServiceKindDto,
        declared_cost: u32,
        membership: FacilityMembershipDto,
        limit: i16,
    ) -> Vec<rfb_protocol::FacilityEnchantmentChoiceDto> {
        let cost = if declared_cost == 0 {
            self.town_service_price(1500)
        } else {
            declared_cost
        };
        let artifact = item.is_artifact(&self.content);
        let mut copy = item.clone();
        let mut old_value = if service == FacilityServiceKindDto::EnchantAmmunition {
            0
        } else {
            self.item_enchantment_value(&copy)
        };
        let mut sum = 0_i64;
        let mut choices = Vec::new();
        for steps in 1..=25 {
            let before = self.item_total_enchantments(&copy);
            let mut changed = false;
            let mut v = 0_i16;
            let mut increment = |value: &mut i16, total: i16| {
                if total < limit {
                    *value += 1;
                    v = v.max(total + 1);
                    changed = true;
                }
            };
            if service == FacilityServiceKindDto::EnchantArmor {
                increment(&mut copy.enchantments.to_armor, before.to_armor);
            } else {
                increment(&mut copy.enchantments.to_hit, before.to_hit);
                increment(&mut copy.enchantments.to_damage, before.to_damage);
            }
            if !changed {
                break;
            }
            let total_cost = if service == FacilityServiceKindDto::EnchantAmmunition {
                u64::from(steps) * u64::from(cost) * u64::from(item.quantity)
            } else {
                let mut multiplier = 5_i64;
                if v > 10 {
                    if service == FacilityServiceKindDto::EnchantArmor {
                        for _ in 10..v {
                            multiplier = multiplier * 5 / 3;
                        }
                    } else {
                        multiplier += i64::from(v - 10);
                    }
                }
                if artifact {
                    multiplier *= 3;
                }
                let value = self.item_enchantment_value(&copy);
                sum += (value - old_value) * multiplier;
                old_value = value;
                let mut unit = self
                    .town_service_price(u32::try_from(sum.max(0)).unwrap_or(u32::MAX))
                    .max(u32::from(steps).saturating_mul(cost));
                if membership == FacilityMembershipDto::Owner {
                    unit = unit.div_ceil(2);
                }
                u64::from(unit) * u64::from(item.quantity)
            };
            let Ok(mut cost) = u32::try_from(total_cost) else {
                break;
            };
            if service != FacilityServiceKindDto::EnchantAmmunition && cost >= 10000 {
                cost = round_three_significant(cost);
            }
            choices.push(rfb_protocol::FacilityEnchantmentChoiceDto {
                steps,
                cost,
                result: self.item_total_enchantments(&copy),
            });
        }
        choices
    }

    pub(super) fn town_facility_service_dtos(
        &self,
        facility: &TownFacilityDefinition,
    ) -> Vec<FacilityServiceDto> {
        let membership = self.town_facility_membership(facility);
        let limit = self.town_facility_enchantment_limit(membership);
        facility
            .service_actions
            .iter()
            .copied()
            .map(|definition| {
                let kind = facility_service_kind_dto(definition.kind);
                let cost = self.town_facility_service_cost(definition, membership);
                let mut targets = if enchantment_service(kind) {
                    self.items
                        .iter()
                        .filter(|item| self.town_facility_enchantment_target(item, kind, limit))
                        .map(|item| FacilityServiceTargetDto {
                            item_id: item.id.clone(),
                            choices: self
                                .facility_enchantment_choices(item, kind, cost, membership, limit),
                        })
                        .filter(|target| !target.choices.is_empty())
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                targets.sort_by(|left, right| left.item_id.cmp(&right.item_id));
                FacilityServiceDto {
                    kind,
                    cost,
                    targets,
                }
            })
            .collect()
    }

    pub(super) fn town_facility_accessible(&self, facility_id: &str) -> bool {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return false;
        };
        let Some(town) = self.current_town() else {
            return false;
        };
        facility.town_id == town.id
            && town.facility_ids.contains(&facility.id)
            && self.town_facility_entrance_position(facility) == Some(self.player.position)
            && self.terrain_at(self.player.position) == facility.entrance_terrain_id
    }

    pub(super) fn shop_entrance_position(&self, shop: &ShopDefinition) -> Option<Position> {
        let mut positions = shop.entrance_positions().filter_map(|position| {
            self.town_local_to_active_position(&shop.town_id, position_from_content(position))
        });
        let primary = positions.next();
        positions
            .find(|position| *position == self.player.position)
            .or(primary)
    }

    pub(super) fn town_facility_entrance_position(
        &self,
        facility: &TownFacilityDefinition,
    ) -> Option<Position> {
        let mut positions = facility.entrance_positions().filter_map(|position| {
            self.town_local_to_active_position(&facility.town_id, position_from_content(position))
        });
        let primary = positions.next();
        positions
            .find(|position| *position == self.player.position)
            .or(primary)
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
        self.identify_item_at_facility(facility_id, item_id, self.town_service_price(cost), false)
    }

    pub(super) fn research_item_at_facility(
        &mut self,
        facility_id: &str,
        item_id: &str,
    ) -> Result<FacilityIdentifyOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-facility");
        };
        let Some(cost) = facility.research_item_cost else {
            return Err("service-unavailable");
        };
        self.identify_item_at_facility(facility_id, item_id, self.town_service_price(cost), true)
    }

    fn identify_item_at_facility(
        &mut self,
        facility_id: &str,
        item_id: &str,
        cost: u32,
        full: bool,
    ) -> Result<FacilityIdentifyOutcome, &'static str> {
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
        let already_identified = self
            .item_property_knowledge
            .get(&item.id)
            .is_some_and(|knowledge| knowledge.identified || (!full && knowledge.appraised));
        if already_identified {
            return Err("already-identified");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        let outcome = self.identify_item_instance(item_id, ItemIdentificationRequest::new(full));
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

    pub(super) fn identify_all_at_facility(
        &mut self,
        facility_id: &str,
    ) -> Result<FacilityIdentifyAllOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id) else {
            return Err("unknown-facility");
        };
        let Some(price) = facility.identify_all_items_cost else {
            return Err("service-unavailable");
        };
        let cost = self.town_facility_price(facility, price);
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let has_unexamined_item = self.items.iter().any(|item| {
            item.quantity > 0
                && matches!(
                    item.location,
                    ItemLocation::Inventory | ItemLocation::Equipped { .. }
                )
                && !self
                    .item_property_knowledge
                    .get(&item.id)
                    .is_some_and(|knowledge| knowledge.appraised || knowledge.identified)
        });
        if !has_unexamined_item {
            return Err("nothing-to-identify");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        let identified_count = self.identify_carried_items();
        debug_assert!(identified_count > 0);
        self.gold -= cost;
        Ok(FacilityIdentifyAllOutcome {
            facility_id: facility_id.to_owned(),
            identified_count,
            cost,
            gold_balance: self.gold,
        })
    }

    pub(super) fn use_town_facility_service(
        &mut self,
        facility_id: &str,
        service: FacilityServiceKindDto,
        item_id: Option<&str>,
        enchantment_steps: Option<u8>,
        events: &mut Vec<DomainEvent>,
    ) -> Result<FacilityServiceOutcome, &'static str> {
        let Some(facility) = self.content.town_facility(facility_id).cloned() else {
            return Err("unknown-facility");
        };
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let Some(projected) = self
            .town_facility_service_dtos(&facility)
            .into_iter()
            .find(|candidate| candidate.kind == service)
        else {
            return Err("service-unavailable");
        };
        let (cost, item_id) = if enchantment_service(service) {
            let item_id = item_id.ok_or("item-required")?;
            let target = projected
                .targets
                .iter()
                .find(|target| target.item_id == item_id)
                .ok_or("item-unavailable")?;
            let choice = target
                .choices
                .iter()
                .find(|choice| Some(choice.steps) == enchantment_steps)
                .ok_or("enchantment-steps-unavailable")?;
            (choice.cost, Some(item_id))
        } else {
            if item_id.is_some() || enchantment_steps.is_some() {
                return Err("unexpected-item");
            }
            (projected.cost, None)
        };
        if self.gold < cost {
            return Err("insufficient-gold");
        }

        let outcome = match service {
            FacilityServiceKindDto::Heal => {
                let healed = self.apply_player_healing(200).applied;
                let before = self.player.statuses.len();
                self.player.statuses.retain(|status| {
                    !matches!(
                        status.kind_id.as_str(),
                        STATUS_POISON
                            | STATUS_BLINDNESS
                            | STATUS_CONFUSION
                            | STATUS_BLEEDING
                            | STATUS_STUN
                    )
                });
                let statuses_removed = before - self.player.statuses.len();
                let riding_actor_id = self.riding_actor_id.clone();
                let mount_healed = riding_actor_id
                    .as_deref()
                    .and_then(|mount_id| {
                        let index = self
                            .entities
                            .iter()
                            .position(|actor| actor.id == mount_id)?;
                        let definition = self.content.actor(&self.entities[index].kind_id)?;
                        let maximum = self
                            .actor_derived_stats(&self.entities[index], definition, false)
                            .max_hp
                            .value;
                        let before = self.entities[index].hp;
                        self.entities[index].hp = before.saturating_add(500).min(maximum);
                        Some(self.entities[index].hp.saturating_sub(before))
                    })
                    .unwrap_or(0);
                FacilityServiceOutcome::Healed {
                    facility_id: facility_id.to_owned(),
                    healed,
                    mount_healed,
                    statuses_removed,
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
            FacilityServiceKindDto::RestoreVitality => {
                let attributes_restored = self.restore_all_player_attributes();
                let mut restoration_events = Vec::new();
                let vitality_restored =
                    self.restore_player_experience_and_life_force(1_000, &mut restoration_events);
                if !attributes_restored && !vitality_restored {
                    return Err("nothing-to-restore");
                }
                events.extend(restoration_events);
                FacilityServiceOutcome::VitalityRestored {
                    facility_id: facility_id.to_owned(),
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
            FacilityServiceKindDto::CureMutation => {
                let mut mutation_events = Vec::new();
                let mutation_id = self
                    .cure_random_mutation(false, &mut mutation_events)
                    .ok_or("no-curable-mutation")?;
                events.extend(mutation_events);
                FacilityServiceOutcome::MutationCured {
                    facility_id: facility_id.to_owned(),
                    mutation_id,
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
            FacilityServiceKindDto::BalanceRitual => {
                self.perform_balance_ritual();
                FacilityServiceOutcome::BalanceRitualPerformed {
                    facility_id: facility_id.to_owned(),
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
            FacilityServiceKindDto::EnchantWeapon
            | FacilityServiceKindDto::EnchantArmor
            | FacilityServiceKindDto::EnchantAmmunition
            | FacilityServiceKindDto::EnchantBow => {
                let item_id = item_id.expect("enchantment target was validated");
                let item = self
                    .items
                    .iter()
                    .find(|item| item.id == item_id)
                    .expect("validated enchantment target must exist");
                let total = self.item_total_enchantments(item);
                let limit =
                    self.town_facility_enchantment_limit(self.town_facility_membership(&facility));
                let steps = i16::from(enchantment_steps.expect("enchantment steps were validated"));
                let attempts = |before: i16| steps.min(limit.saturating_sub(before).max(0)) as u16;
                let request = if service == FacilityServiceKindDto::EnchantArmor {
                    ItemEnchantmentRequest::new(0, 0, attempts(total.to_armor))
                } else {
                    ItemEnchantmentRequest::new(
                        attempts(total.to_hit),
                        attempts(total.to_damage),
                        0,
                    )
                }
                .forced();
                let enchanted = self.enchant_item_instance(item_id, request);
                if enchanted.to_hit.successes == 0
                    && enchanted.to_damage.successes == 0
                    && enchanted.to_armor.successes == 0
                {
                    return Err("enchantment-failed");
                }
                let component = |outcome: super::inventory::ItemEnchantmentComponentOutcome| {
                    ItemEnchantmentComponentResolutionDto {
                        attempts: outcome.attempts,
                        successes: outcome.successes,
                        before: outcome.before,
                        after: outcome.after,
                    }
                };
                FacilityServiceOutcome::ItemEnchanted {
                    facility_id: facility_id.to_owned(),
                    cost,
                    gold_balance: self.gold - cost,
                    resolution: ItemEnchantmentResolutionDto {
                        item_id: enchanted.item_id,
                        item_kind_id: enchanted.item_kind_id,
                        to_hit: component(enchanted.to_hit),
                        to_damage: component(enchanted.to_damage),
                        to_armor: component(enchanted.to_armor),
                    },
                }
            }
            FacilityServiceKindDto::AssessArmor => {
                let armor_class = self.player_derived_stats().armor_class.value.max(0);
                let protection_percent =
                    100_i32.saturating_sub(apply_melee_armor_reduction(100, armor_class));
                FacilityServiceOutcome::ArmorAssessed {
                    facility_id: facility_id.to_owned(),
                    armor_class,
                    protection_percent,
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
            FacilityServiceKindDto::Recall => {
                if self.recall_use_plan() != Some(RecallUseAction::Start) {
                    return Err("recall-unavailable");
                }
                let destination = self.start_recall(1);
                FacilityServiceOutcome::RecallStarted {
                    facility_id: facility_id.to_owned(),
                    dungeon_id: destination.dungeon_id,
                    floor_id: destination.floor_id,
                    cost,
                    gold_balance: self.gold - cost,
                }
            }
        };
        self.gold -= cost;
        Ok(outcome)
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
        let cost = self.town_service_price(cost);
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

    pub(super) fn eat_at_inn(
        &mut self,
        facility_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), &'static str> {
        let inn = self.content.shop(facility_id).ok_or("unknown-inn")?;
        let cost = self.town_service_price(inn.inn_food_cost.ok_or("service-unavailable")?);
        if !shop_accessible(self, inn) {
            return Err("inn-unreachable");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }
        let food_key = self.consume_inn_meal(events);
        self.gold -= cost;
        events.push(DomainEvent::InnFoodCompleted {
            facility_id: facility_id.to_owned(),
            cost,
            gold_balance: self.gold,
            food_key,
        });
        Ok(())
    }

    pub(super) fn ask_reputation_at_inn(
        &mut self,
        facility_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), &'static str> {
        let inn = self.content.shop(facility_id).ok_or("unknown-inn")?;
        let cost = self.town_service_price(inn.inn_reputation_cost.ok_or("service-unavailable")?);
        if !shop_accessible(self, inn) {
            return Err("inn-unreachable");
        }
        if self.gold < cost {
            return Err("insufficient-gold");
        }
        let message_key = match self.fame {
            0 => "inn-reputation-unknown",
            1..20 => "inn-reputation-unheard",
            20..40 => "inn-reputation-noticed",
            40..60 => "inn-reputation-talked",
            60..80 => "inn-reputation-honored",
            80..100 => "inn-reputation-hero",
            100..150 => "inn-reputation-legend",
            _ => "inn-reputation-ballads",
        };
        self.gold -= cost;
        events.push(DomainEvent::InnReputationReported {
            facility_id: facility_id.to_owned(),
            fame: self.fame,
            cost,
            gold_balance: self.gold,
            message_key,
        });
        Ok(())
    }

    pub(super) fn teleport_dungeon_dtos(&self) -> Vec<rfb_protocol::TeleportDungeonDto> {
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must exist");
        world
            .dungeons
            .iter()
            .filter_map(|dungeon| {
                let recall_id = self.dungeon_states[&dungeon.id].recall_floor_id.as_ref()?;
                if !self.dungeon_entry_requirements_met(dungeon) {
                    return None;
                }
                let root = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == dungeon.root_floor_id)
                    .expect("dungeon root must exist");
                let recall = world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == *recall_id)
                    .expect("dungeon recall floor must exist");
                let mut depths = world
                    .procedural_floors
                    .iter()
                    .filter(|floor| floor.dungeon_id.as_ref() == Some(&dungeon.id))
                    .map(|floor| floor.depth)
                    .collect::<Vec<_>>();
                depths.sort_unstable();
                depths.dedup();
                Some(rfb_protocol::TeleportDungeonDto {
                    dungeon_id: dungeon.id.clone(),
                    name_key: root.name_key.clone(),
                    recall_depth: recall.depth,
                    depths,
                })
            })
            .collect()
    }

    pub(super) fn teleport_to_dungeon_level_at_facility(
        &mut self,
        facility_id: &str,
        dungeon_id: &str,
        depth: u16,
    ) -> Result<FacilityServiceOutcome, &'static str> {
        let facility = self
            .content
            .town_facility(facility_id)
            .ok_or("unknown-facility")?;
        let price = facility.teleport_level_cost.ok_or("service-unavailable")?;
        let cost = self.town_facility_price(facility, price);
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        if !self
            .teleport_dungeon_dtos()
            .iter()
            .any(|dungeon| dungeon.dungeon_id == dungeon_id && dungeon.depths.contains(&depth))
        {
            return Err("recall-unavailable");
        }
        let world = self
            .content
            .world(&self.world_id)
            .expect("active world must exist");
        let floor_id = world
            .procedural_floors
            .iter()
            .find(|floor| floor.dungeon_id.as_deref() == Some(dungeon_id) && floor.depth == depth)
            .expect("projected teleport floor must exist")
            .id
            .clone();
        if self.gold < cost {
            return Err("insufficient-gold");
        }
        self.reset_recall(super::floor::RecallDestination {
            dungeon_id: dungeon_id.to_owned(),
            floor_id: floor_id.clone(),
        });
        // BACT_TELEPORT_LEVEL replaces even an already pending recall with a one-turn recall.
        // Facility commands do not advance time, so start_recall's extra tick is the whole delay.
        self.start_recall(0);
        self.gold -= cost;
        Ok(FacilityServiceOutcome::RecallStarted {
            facility_id: facility_id.to_owned(),
            dungeon_id: dungeon_id.to_owned(),
            floor_id,
            cost,
            gold_balance: self.gold,
        })
    }

    pub(super) fn research_monster_at_facility(
        &mut self,
        facility_id: &str,
        actor_kind_id: &str,
        events: &mut Vec<DomainEvent>,
    ) -> Result<(), &'static str> {
        let facility = self
            .content
            .town_facility(facility_id)
            .ok_or("unknown-facility")?;
        let price = facility
            .research_monster_cost
            .ok_or("service-unavailable")?;
        let cost = self.town_facility_price(facility, price);
        if !self.town_facility_accessible(facility_id) {
            return Err("facility-unreachable");
        }
        let actor = self
            .content
            .actor(actor_kind_id)
            .filter(|actor| actor.role == rfb_content::ActorRole::Monster)
            .ok_or("monster-unavailable")?;
        if self.gold < cost {
            return Err("insufficient-gold");
        }
        self.probed_actor_kind_ids.insert(actor.id.clone());
        self.gold -= cost;
        events.push(DomainEvent::MonsterResearchCompleted {
            facility_id: facility_id.to_owned(),
            actor_kind_id: actor_kind_id.to_owned(),
            cost,
            gold_balance: self.gold,
        });
        Ok(())
    }

    pub(super) fn research_monster_dtos(&self) -> Vec<rfb_protocol::ResearchMonsterDto> {
        let mut monsters = self
            .content
            .actor_definitions()
            .filter(|actor| actor.role == rfb_content::ActorRole::Monster)
            .map(|actor| rfb_protocol::ResearchMonsterDto {
                kind_id: actor.id.clone(),
                name_key: actor.name_key.clone(),
                glyph: actor.glyph.clone(),
                level: actor.level,
                unique: actor.tags.iter().any(|tag| tag == "unique"),
                knowledge: self.probed_actor_kind_ids.contains(&actor.id).then(|| {
                    rfb_protocol::MonsterKindKnowledgeDto {
                        description_key: actor.description_key.clone(),
                        max_hp: actor.max_hp,
                        speed: actor.speed,
                        armor_class: crate::combat::rating_to_armor_class(actor.defense),
                        resistances: crate::resistance::definition_resistance_profile(actor)
                            .to_dtos(),
                        status_immunities: actor.status_immunities.clone(),
                        melee_routine: super::actor_melee_routine_dto(actor),
                        ability_ids: actor
                            .monster_casting
                            .as_ref()
                            .map(|casting| {
                                casting
                                    .abilities
                                    .iter()
                                    .map(|ability| ability.ability_id.clone())
                                    .collect()
                            })
                            .unwrap_or_default(),
                    }
                }),
            })
            .collect::<Vec<_>>();
        monsters.sort_by(|a, b| b.level.cmp(&a.level).then(a.kind_id.cmp(&b.kind_id)));
        monsters
    }

    pub(super) fn stay_at_inn(
        &mut self,
        facility_id: &str,
    ) -> Result<InnStayOutcome, &'static str> {
        let cost = if let Some(inn) = self.content.shop(facility_id) {
            let cost = self.town_service_price(inn.inn_stay_cost.ok_or("unknown-inn")?);
            if !shop_accessible(self, inn) {
                return Err("inn-unreachable");
            }
            cost
        } else if let Some(facility) = self.content.town_facility(facility_id) {
            let price = facility.inn_stay_cost.ok_or("unknown-inn")?;
            if !self.town_facility_accessible(facility_id) {
                return Err("inn-unreachable");
            }
            self.town_facility_price(facility, price)
        } else {
            return Err("unknown-inn");
        };
        self.rest_at_inn(facility_id, cost)
    }

    fn rest_at_inn(
        &mut self,
        facility_id: &str,
        cost: u32,
    ) -> Result<InnStayOutcome, &'static str> {
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
        self.clear_daylight_suppression_at_dawn();

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

    fn town_teleport_facility(&self, town: &TownDefinition) -> Option<&TownFacilityDefinition> {
        town.facility_ids
            .iter()
            .filter_map(|id| self.content.town_facility(id))
            .find(|facility| facility.town_teleport.is_some())
    }

    fn town_teleport_unlocked(&self, facility: &TownFacilityDefinition) -> bool {
        facility.town_teleport.as_ref().is_some_and(|teleport| {
            self.task_states
                .get(&teleport.required_completed_task_id)
                .is_some_and(|state| state.status == TaskStatusKindDto::Completed)
        })
    }

    fn town_teleport_arrival(&self, town: &TownDefinition) -> Option<Position> {
        if let Some(facility) = self.town_teleport_facility(town) {
            return self
                .town_teleport_unlocked(facility)
                .then(|| position_from_content(facility.entrance_position));
        }
        town_inn(town, &self.content).map(|inn| position_from_content(inn.entrance_position))
    }

    fn town_travel_origin(&self, facility_id: &str) -> Option<(&str, u32)> {
        if let Some(inn) = self.content.shop(facility_id) {
            return (inn.inn_stay_cost.is_some() && shop_accessible(self, inn)).then(|| {
                (
                    inn.town_id.as_str(),
                    self.town_service_price(INN_TRAVEL_COST),
                )
            });
        }
        let facility = self.content.town_facility(facility_id)?;
        let teleport = facility.town_teleport.as_ref()?;
        (self.town_facility_accessible(facility_id) && self.town_teleport_unlocked(facility)).then(
            || {
                (
                    facility.town_id.as_str(),
                    self.town_facility_price(facility, teleport.price),
                )
            },
        )
    }

    pub(super) fn facility_town_travel_destinations(
        &self,
        facility_id: &str,
    ) -> Vec<InnTravelDestinationDto> {
        let Some((_, cost)) = self.town_travel_origin(facility_id) else {
            return Vec::new();
        };
        self.teleport_town_targets()
            .into_iter()
            .map(|target| InnTravelDestinationDto {
                town_id: target.town_id,
                town_name_key: target.town_name_key,
                cost,
            })
            .collect()
    }

    pub(super) fn inn_travel_unavailable_reason(
        &self,
        facility_id: &str,
        destination_town_id: &str,
    ) -> Option<&'static str> {
        if self
            .content
            .shop(facility_id)
            .is_none_or(|shop| shop.inn_stay_cost.is_none())
            && self
                .content
                .town_facility(facility_id)
                .is_none_or(|facility| facility.town_teleport.is_none())
        {
            return Some("unknown-inn");
        }
        let Some((origin_town_id, cost)) = self.town_travel_origin(facility_id) else {
            return Some("inn-unreachable");
        };
        if origin_town_id == destination_town_id {
            return Some("already-here");
        }
        if !self.teleport_town_target_available(destination_town_id) {
            return Some("town-unvisited");
        }
        (self.gold < cost).then_some("insufficient-gold")
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
        let (_, cost) = self
            .town_travel_origin(facility_id)
            .expect("preflighted travel origin");
        self.relocate_to_town(destination_town_id)?;
        self.gold -= cost;

        Ok(InnTravelOutcome {
            facility_id: facility_id.to_owned(),
            destination_town_id: destination_town_id.to_owned(),
            cost,
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
                    && self.town_teleport_arrival(town).is_some())
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

    /// Desktop map fixture: visit one restored town and reveal its current surface.
    #[doc(hidden)]
    pub fn debug_prepare_town_map_e2e(&mut self, town_id: &str) -> Result<(), CoreError> {
        if !matches!(
            town_id,
            "demo.town.anambar" | "demo.town.thalos" | "demo.town.zul"
        ) || self.world_id != "demo.world.middle-earth"
            || self.map_scale != MapScaleDto::Local
            || self.current_floor_id != super::wilderness::WILDERNESS_FLOOR_ID
        {
            return Err(CoreError::InvalidSave(
                "town map fixture requires the Middle-earth surface",
            ));
        }
        if town_id == "demo.town.zul" {
            // Physical arrival must not borrow quest 77's still-locked teleport landing.
            self.store_visible_town_states();
            self.wilderness_position =
                world_town_position(self.content.world(&self.world_id).unwrap(), town_id);
            self.wilderness_view_offset = Position::default();
            self.activate_wilderness_position(None, false)?;
            self.mark_current_town_visited();
        } else {
            self.relocate_to_town(town_id)?;
        }
        self.explored.fill(true);
        self.reveal_current_visibility();
        Ok(())
    }

    /// Zul desktop acceptance preparation. The native app exposes this only with WebDriver.
    #[doc(hidden)]
    pub fn debug_prepare_zul_e2e(&mut self, clear_enemies: bool) -> Result<(), CoreError> {
        if clear_enemies {
            let visited_zul_wilderness = self.is_wilderness_floor()
                && self
                    .town_states
                    .get("demo.town.zul")
                    .is_some_and(|town| town.visited);
            if !visited_zul_wilderness
                && !matches!(
                    self.current_floor_id.as_str(),
                    "demo.floor.zul-eddies"
                        | "demo.floor.zul-sorcery-node"
                        | "demo.floor.zul-chaos-node"
                        | "demo.floor.zul-nature-node"
                )
            {
                return Err(CoreError::InvalidSave(
                    "Zul clear fixture requires visited Zul wilderness or a node/eddies floor",
                ));
            }
            // Prepare a clear route or task combat result; normal actions evaluate objectives.
            self.entities.clear();
            self.items
                .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        } else {
            if self
                .current_town()
                .is_none_or(|town| town.id != "demo.town.zul")
                || self
                    .build
                    .as_ref()
                    .is_none_or(|build| build.class_id != "demo.class.mage")
            {
                return Err(CoreError::InvalidSave(
                    "Zul traversal fixture requires a Mage in Zul",
                ));
            }
            // Reuse the existing level/realm-book fixture; no terrain is changed for Mage.
            self.debug_prepare_spell_learning_e2e(50)?;
            self.apply_player_melee_status(super::STATUS_LEVITATION, 200_000, "e2e.zul-traversal");
            self.apply_player_melee_status(
                super::STATUS_INVULNERABILITY,
                200_000,
                "e2e.zul-traversal",
            );
            // Match the existing spell's incoming-damage modifier, not just its status name.
            self.player
                .statuses
                .iter_mut()
                .find(|status| status.kind_id == super::STATUS_INVULNERABILITY)
                .unwrap()
                .incoming_damage_percent = 0;
            for virtue in &mut self.virtues {
                virtue.value = 51;
            }
        }
        self.explored.fill(true);
        self.reveal_current_visibility();
        Ok(())
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
        let destination_inn_position = self
            .town_teleport_arrival(destination_town)
            .expect("validated destination town must retain a teleport arrival");

        let player_id = self.player.id.clone();
        let riding_actor_id = self.riding_actor_id.as_deref();
        let (mut followers, retained): (Vec<_>, Vec<_>) = std::mem::take(&mut self.entities)
            .into_iter()
            .partition(|actor| {
                actor.controller_id.as_deref() == Some(player_id.as_str())
                    && Some(actor.id.as_str()) != riding_actor_id
            });
        self.entities = retained;
        self.refresh_duelist_challenge();
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
        let museum = self
            .content
            .town_facility(facility_id)
            .is_some_and(|facility| facility.reject_artifact_deposits);
        if museum
            && self
                .content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.artifact_generation.is_some())
        {
            return Err("artifact-rejected");
        }
        if quantity > available_quantity {
            return Err("insufficient-quantity");
        }
        if item.kind_id == "demo.item.blood-potion"
            && self.content.item("demo.item.salt-water").is_none()
        {
            return Err("item-unavailable");
        }
        let split_required = group_requires_split(&self.items, &source_ids, quantity);
        let split_id = split_required
            .then(|| self.allocate_item_instance_id())
            .transpose()
            .map_err(|_| "item-id-exhausted")?;
        let mut deposited =
            transfer_inventory_group_to_home(self, &storage_id, &source_ids, quantity, split_id);
        for item in &mut deposited {
            if museum {
                item.inscription = None;
            }
            if item.kind_id == "demo.item.blood-potion" {
                item.kind_id = "demo.item.salt-water".to_owned();
            }
        }
        let destination_id = deposited
            .first()
            .expect("successful deposit must have a destination")
            .id
            .clone();
        let item_kind_id = deposited[0].kind_id.clone();
        self.home_states
            .get_mut(&storage_id)
            .expect("preflighted home must remain available")
            .inventory
            .extend(deposited);
        if museum {
            self.add_virtue(rfb_protocol::VirtueKindDto::Sacrifice, 1);
        }
        Ok(HomeTransferOutcome {
            facility_id: facility_id.to_owned(),
            item_id: destination_id,
            item_kind_id,
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
        if shop.category == ShopCategory::Shroomery
            && (self
                .character_definitions()
                .is_some_and(|(_, race, _, _)| race.id == "rfb-legacy.race.snotling")
                || self
                    .selected_race_definition()
                    .is_some_and(|race| race.id == "rfb-legacy.race.snotling"))
        {
            return Err("race-refused");
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
            discounted_item_base_value(&self.content, &shop, &item, definition.base_value),
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
        for mut item in purchased {
            if self
                .content
                .item(&item.kind_id)
                .is_some_and(|kind| kind.ability_book_id.is_some())
            {
                item.book_counted = true;
            }
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
        if !shop_accepts_item(self, &shop, &item) {
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
            discounted_item_base_value(&self.content, &shop, &item, definition.base_value),
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
        let mut sold = transfer_group_to_shop(self, shop_id, &source_ids, quantity, split_id);
        for item in &mut sold {
            // shop.c::_buy_aux also calls stats_on_purchase (stats_on_sell is
            // unused). Preserve that source behavior for an uncounted book.
            if self
                .content
                .item(&item.kind_id)
                .is_some_and(|kind| kind.ability_book_id.is_some())
            {
                item.book_counted = true;
            }
        }
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
                    self.shop_entrance_position(shop) == Some(self.player.position)
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
            && (!stock::generates_equipment(shop.category) || !state.inventory.is_empty())
        {
            return Ok(());
        }
        if stock::generates_equipment(shop.category) {
            let count = state.inventory.len();
            let additions = self.roll_special_shop_stock(&shop, count)?;
            let state = self.shop_states.get_mut(&shop_id).unwrap();
            state.inventory.extend(additions);
            state.last_maintenance_world_tick = self.world_tick;
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
            if self.shop_entrance_position(&shop) != Some(self.player.position) {
                continue;
            }
            if !self.shop_states.contains_key(shop_id)
                || (stock::generates_equipment(shop.category)
                    && self.shop_states[shop_id].inventory.is_empty())
            {
                let inventory = if stock::generates_equipment(shop.category) {
                    self.roll_special_shop_stock(&shop, 0)?
                } else {
                    roll_shop_stock(
                        &shop,
                        &self.content,
                        &mut self.rng,
                        &mut self.next_item_instance_serial,
                        false,
                    )?
                };
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
            if self.town_facility_accessible(&facility.id)
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
                    .shop_entrance_position(shop)
                    .expect("current town shop must retain an active position");
                let player_at_entrance = self.player.position == entrance_position;
                let inn_travel_destinations = self.facility_town_travel_destinations(&shop.id);
                let factor = shop_price_factor(self, shop);
                let state = self.shop_states.get(&shop.id);
                let mut stock = if player_at_entrance {
                    state
                        .map(|state| grouped_shop_items(&self.content, &state.inventory))
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
                                discounted_item_base_value(
                                    &self.content,
                                    shop,
                                    item,
                                    definition.base_value,
                                ),
                                factor,
                            );
                            let affordable = self.gold / unit_price.max(1);
                            let slot_carryable = self.inventory_quantity_capacity_for(item, false);
                            ShopStockItemDto {
                                id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                display_name_key: definition.name_key.clone(),
                                artifact_name: item.artifact_name.clone(),
                                affix_name_keys: item
                                    .affix_ids
                                    .iter()
                                    .map(|id| {
                                        self.content
                                            .affix(id)
                                            .expect("shop affix must remain available")
                                            .name_key
                                            .clone()
                                    })
                                    .collect(),
                                quantity,
                                inscription: item.inscription.clone(),
                                captured_actor: self.captured_actor_dto(item),
                                maximum_quantity: quantity.min(affordable).min(slot_carryable),
                                unit_price,
                                weight_tenths_pound: self.item_instance_weight(item),
                                fuel: item.fuel,
                                charges: item.charges,
                                activation: item.activation.clone(),
                                enchantments: item.enchantments,
                                curse: item.curse,
                                permanent_destruction_immunities: item
                                    .permanent_destruction_immunities
                                    .iter()
                                    .copied()
                                    .map(item_destruction_element_to_dto)
                                    .collect(),
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
                            let unavailable_reason = (!shop_accepts_item(self, shop, item))
                                .then(|| "item-illegal".to_owned());
                            ShopSellQuoteDto {
                                item_id: item.id.clone(),
                                kind_id: item.kind_id.clone(),
                                unit_price: unavailable_reason.as_ref().map_or_else(
                                    || {
                                        player_sale_unit_price(
                                            self,
                                            shop,
                                            discounted_item_base_value(
                                                &self.content,
                                                shop,
                                                item,
                                                definition.base_value,
                                            ),
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
                    inn_stay_cost: shop.inn_stay_cost.map(|cost| self.town_service_price(cost)),
                    inn_food_cost: shop.inn_food_cost.map(|cost| self.town_service_price(cost)),
                    inn_reputation_cost: shop
                        .inn_reputation_cost
                        .map(|cost| self.town_service_price(cost)),
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
                    .town_facility_entrance_position(facility)
                    .expect("current town Home must retain an active position");
                let player_at_entrance = self.town_facility_accessible(&facility.id);
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
                            let slot_carryable = self.inventory_quantity_capacity_for(item, true);
                            HomeItemDto {
                                id: item.id.clone(),
                                details: Some(self.inventory_item_dto(item)),
                                kind_id: item.kind_id.clone(),
                                display_name_key: self.item_display_name_key(&item.kind_id),
                                artifact_name: self.visible_artifact_name(item),
                                quantity,
                                inscription: item.inscription.clone(),
                                captured_actor: self.captured_actor_dto(item),
                                maximum_quantity: quantity.min(slot_carryable),
                                weight_tenths_pound: self.item_instance_weight(item),
                                fuel: item.fuel,
                                permanent_destruction_immunities: item
                                    .permanent_destruction_immunities
                                    .iter()
                                    .copied()
                                    .map(item_destruction_element_to_dto)
                                    .collect(),
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
                        .filter(|(item, _)| {
                            !facility.reject_artifact_deposits
                                || self.content.item(&item.kind_id).is_some_and(|definition| {
                                    definition.artifact_generation.is_none()
                                })
                        })
                        .map(|(item, quantity)| HomeItemDto {
                            id: item.id.clone(),
                            details: Some(self.inventory_item_dto(item)),
                            kind_id: item.kind_id.clone(),
                            display_name_key: self.item_display_name_key(&item.kind_id),
                            artifact_name: self.visible_artifact_name(item),
                            quantity,
                            inscription: item.inscription.clone(),
                            captured_actor: self.captured_actor_dto(item),
                            maximum_quantity: quantity,
                            weight_tenths_pound: self.item_instance_weight(item),
                            fuel: item.fuel,
                            permanent_destruction_immunities: item
                                .permanent_destruction_immunities
                                .iter()
                                .copied()
                                .map(item_destruction_element_to_dto)
                                .collect(),
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                deposit_items.sort_by(|left, right| left.id.cmp(&right.id));
                HomeDto {
                    id: facility.id.clone(),
                    museum: facility.reject_artifact_deposits,
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
