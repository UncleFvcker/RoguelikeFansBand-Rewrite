// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::*;

use super::shared::{
    insert_definition_id, require_format_version, require_schema, validate_definition_id,
    validate_definition_text, validate_message_key,
};

const GENERAL_STORE_ITEM_IDS: [&str; 4] = [
    "demo.item.ration-of-food",
    "demo.item.wooden-torch",
    "demo.item.brass-lantern",
    "demo.item.flask-of-oil",
];

pub(super) struct TownValidationRefs<'a> {
    pub(super) items: &'a [ItemDefinition],
    pub(super) races: &'a [RaceDefinition],
}

pub(super) struct TownValidationOutputs {
    pub(super) towns_by_id: BTreeMap<String, TownDefinition>,
    pub(super) shops_by_id: BTreeMap<String, ShopDefinition>,
}

pub(super) fn validate_towns_and_shops(
    towns: &mut [TownDefinition],
    shops: &mut [ShopDefinition],
    refs: TownValidationRefs<'_>,
    all_ids: &mut BTreeSet<String>,
) -> Result<TownValidationOutputs, ContentError> {
    let mut towns_by_id = BTreeMap::new();
    for town in towns {
        require_schema(&town.schema, TOWN_SCHEMA, &town.id)?;
        require_format_version(town.format_version, &town.id)?;
        validate_definition_id(&town.id, "town")?;
        validate_definition_text(&town.id, &town.name_key, &town.description_key)?;
        validate_definition_id(&town.floor_id, "floor")?;
        town.shop_ids.sort();
        if town.shop_ids.is_empty()
            || town.shop_ids.windows(2).any(|pair| pair[0] == pair[1])
            || town
                .shop_ids
                .iter()
                .any(|shop_id| validate_definition_id(shop_id, "shop").is_err())
        {
            return Err(ContentError::InvalidTown(town.id.clone()));
        }
        insert_definition_id(all_ids, &town.id)?;
        towns_by_id.insert(town.id.clone(), town.clone());
    }

    let mut shops_by_id = BTreeMap::new();
    for shop in shops {
        require_schema(&shop.schema, SHOP_SCHEMA, &shop.id)?;
        require_format_version(shop.format_version, &shop.id)?;
        validate_definition_id(&shop.id, "shop")?;
        validate_definition_text(&shop.id, &shop.name_key, &shop.description_key)?;
        validate_definition_id(&shop.town_id, "town")?;
        validate_definition_id(&shop.entrance_terrain_id, "terrain")?;
        validate_definition_id(&shop.owner.id, "shop-owner")?;
        validate_message_key(&shop.owner.name_key)?;
        validate_definition_id(&shop.owner.race_id, "race")?;
        if !(100..=500).contains(&shop.owner.greed_percent)
            || !(1..=999_999_999).contains(&shop.owner.purchase_price_cap)
            || !refs.races.iter().any(|race| race.id == shop.owner.race_id)
            || shop.maintenance.interval_world_ticks == 0
            || shop.maintenance.interval_world_ticks > 1_000_000
        {
            return Err(ContentError::InvalidShop(shop.id.clone()));
        }
        let mut stock_ids = BTreeSet::new();
        for stock in &shop.stock {
            validate_definition_id(&stock.item_kind_id, "item")?;
            let Some(item) = refs.items.iter().find(|item| item.id == stock.item_kind_id) else {
                return Err(ContentError::DanglingReference {
                    owner: shop.id.clone(),
                    target: stock.item_kind_id.clone(),
                });
            };
            if !stock_ids.insert(stock.item_kind_id.as_str())
                || item.base_value == 0
                || stock.initial_minimum == 0
                || stock.initial_minimum > stock.initial_maximum
                || stock.initial_maximum > 1_000
                || stock.maintenance_minimum == 0
                || stock.maintenance_minimum > stock.maintenance_maximum
                || stock.maintenance_maximum > 1_000
            {
                return Err(ContentError::InvalidShop(shop.id.clone()));
            }
        }
        if matches!(shop.category, ShopCategory::GeneralStore)
            && stock_ids != GENERAL_STORE_ITEM_IDS.into_iter().collect::<BTreeSet<_>>()
        {
            return Err(ContentError::InvalidShop(shop.id.clone()));
        }
        insert_definition_id(all_ids, &shop.id)?;
        insert_definition_id(all_ids, &shop.owner.id)?;
        shops_by_id.insert(shop.id.clone(), shop.clone());
    }

    for town in towns_by_id.values() {
        for shop_id in &town.shop_ids {
            let shop = shops_by_id
                .get(shop_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: town.id.clone(),
                    target: shop_id.clone(),
                })?;
            if shop.town_id != town.id {
                return Err(ContentError::InvalidTown(town.id.clone()));
            }
        }
    }
    for shop in shops_by_id.values() {
        let town =
            towns_by_id
                .get(&shop.town_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: shop.id.clone(),
                    target: shop.town_id.clone(),
                })?;
        if !town.shop_ids.contains(&shop.id) {
            return Err(ContentError::InvalidShop(shop.id.clone()));
        }
    }

    Ok(TownValidationOutputs {
        towns_by_id,
        shops_by_id,
    })
}
