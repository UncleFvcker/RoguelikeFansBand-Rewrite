// SPDX-License-Identifier: MPL-2.0

use std::collections::{BTreeMap, BTreeSet};

use crate::*;

use super::shared::{
    insert_definition_id, require_format_version, require_schema, validate_definition_id,
    validate_definition_text, validate_message_key,
};

pub(super) struct TownValidationRefs<'a> {
    pub(super) items: &'a [ItemDefinition],
    pub(super) races: &'a [RaceDefinition],
    pub(super) classes: &'a [ClassDefinition],
}

pub(super) struct TownValidationOutputs {
    pub(super) towns_by_id: BTreeMap<String, TownDefinition>,
    pub(super) facilities_by_id: BTreeMap<String, TownFacilityDefinition>,
    pub(super) shops_by_id: BTreeMap<String, ShopDefinition>,
}

pub(super) fn validate_towns_and_shops(
    towns: &mut [TownDefinition],
    facilities: &mut [TownFacilityDefinition],
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
        town.facility_ids.sort();
        town.shop_ids.sort();
        if (town.facility_ids.is_empty() && town.shop_ids.is_empty())
            || town.facility_ids.windows(2).any(|pair| pair[0] == pair[1])
            || town
                .facility_ids
                .iter()
                .any(|facility_id| validate_definition_id(facility_id, "town-facility").is_err())
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

    let mut facilities_by_id = BTreeMap::new();
    for facility in facilities {
        require_schema(&facility.schema, TOWN_FACILITY_SCHEMA, &facility.id)?;
        require_format_version(facility.format_version, &facility.id)?;
        validate_definition_id(&facility.id, "town-facility")?;
        validate_definition_text(&facility.id, &facility.name_key, &facility.description_key)?;
        validate_definition_id(&facility.town_id, "town")?;
        validate_definition_id(&facility.entrance_terrain_id, "terrain")?;
        if let Some(storage_id) = &facility.storage_id {
            validate_definition_id(storage_id, "town-facility")?;
        }
        if let Some(owner_name_key) = &facility.owner_name_key {
            validate_message_key(owner_name_key)?;
        }
        if let Some(overview_message_key) = &facility.overview_message_key {
            validate_message_key(overview_message_key)?;
        }
        facility.owner_class_ids.sort();
        facility.member_class_ids.sort();
        facility.owner_race_ids.sort();
        facility.member_race_ids.sort();
        facility.owner_realm_ids.sort();
        facility.member_realm_ids.sort();
        facility.service_actions.sort_by_key(|service| service.kind);
        let duplicate_membership = [
            &facility.owner_class_ids,
            &facility.member_class_ids,
            &facility.owner_race_ids,
            &facility.member_race_ids,
            &facility.owner_realm_ids,
            &facility.member_realm_ids,
        ]
        .into_iter()
        .any(|ids| ids.windows(2).any(|pair| pair[0] == pair[1]));
        let unique_task_ids = facility.task_ids.iter().collect::<BTreeSet<_>>();
        let unique_services = facility
            .service_actions
            .iter()
            .map(|service| service.kind)
            .collect::<BTreeSet<_>>();
        let valid_memberships = facility
            .owner_class_ids
            .iter()
            .chain(&facility.member_class_ids)
            .all(|class_id| refs.classes.iter().any(|class| class.id == *class_id))
            && facility
                .owner_race_ids
                .iter()
                .chain(&facility.member_race_ids)
                .all(|race_id| refs.races.iter().any(|race| race.id == *race_id))
            && facility
                .owner_realm_ids
                .iter()
                .chain(&facility.member_realm_ids)
                .all(|realm_id| {
                    !realm_id.is_empty()
                        && realm_id.len() <= 64
                        && realm_id.bytes().all(|byte| {
                            byte.is_ascii_lowercase()
                                || byte.is_ascii_digit()
                                || matches!(byte, b'-' | b'_')
                        })
                });
        let overlapping_memberships = facility
            .owner_class_ids
            .iter()
            .any(|id| facility.member_class_ids.contains(id))
            || facility
                .owner_race_ids
                .iter()
                .any(|id| facility.member_race_ids.contains(id))
            || facility
                .owner_realm_ids
                .iter()
                .any(|id| facility.member_realm_ids.contains(id));
        let has_membership = !facility.owner_class_ids.is_empty()
            || !facility.member_class_ids.is_empty()
            || !facility.owner_race_ids.is_empty()
            || !facility.member_race_ids.is_empty()
            || !facility.owner_realm_ids.is_empty()
            || !facility.member_realm_ids.is_empty();
        let invalid_service_cost = facility.identify_item_cost == Some(0)
            || facility.research_item_cost == Some(0)
            || facility.identify_all_items_cost == Some(0)
            || facility.legal_name_change_cost == Some(0)
            || facility.service_actions.iter().any(|service| {
                service.owner_cost > 999_999_999 || service.other_cost > 999_999_999
            });
        let valid_bounty_office = facility.bounty_office.as_ref().is_none_or(|bounty| {
            bounty.wanted_reward_item_kind_ids.len() == 20
                && bounty.wanted_reward_item_kind_ids.iter().all(|item_id| {
                    validate_definition_id(item_id, "item").is_ok()
                        && refs.items.iter().any(|item| item.id == *item_id)
                })
        });
        let has_service = facility.identify_item_cost.is_some()
            || facility.research_item_cost.is_some()
            || facility.identify_all_items_cost.is_some()
            || facility.overview_message_key.is_some()
            || facility.legal_name_change_cost.is_some()
            || !facility.service_actions.is_empty()
            || facility.bounty_office.is_some();
        let empty_task_service_has_shop = !facility.task_ids.is_empty()
            || facility.bounty_office.is_some()
            || shops.iter().any(|shop| {
                shop.town_id == facility.town_id
                    && shop.entrance_position == facility.entrance_position
                    && shop.entrance_terrain_id == facility.entrance_terrain_id
            });
        if (facility.category == TownFacilityCategory::Home
            && (facility.storage_id.is_none()
                || facility.owner_name_key.is_some()
                || !facility.task_ids.is_empty()
                || facility.identify_item_cost.is_some()
                || facility.research_item_cost.is_some()
                || facility.identify_all_items_cost.is_some()
                || facility.overview_message_key.is_some()
                || facility.legal_name_change_cost.is_some()
                || has_membership
                || !facility.service_actions.is_empty()
                || facility.bounty_office.is_some()))
            || (facility.category == TownFacilityCategory::QuestGiver
                && (facility.storage_id.is_some()
                    || facility.owner_name_key.is_none()
                    || !empty_task_service_has_shop
                    || has_membership
                    || !facility.service_actions.is_empty()))
            || (facility.category == TownFacilityCategory::Service
                && (facility.storage_id.is_some()
                    || facility.owner_name_key.is_none()
                    || !facility.task_ids.is_empty()
                    || facility.bounty_office.is_some()
                    || !has_service))
            || invalid_service_cost
            || !valid_bounty_office
            || !valid_memberships
            || duplicate_membership
            || overlapping_memberships
            || unique_services.len() != facility.service_actions.len()
            || unique_task_ids.len() != facility.task_ids.len()
            || facility
                .task_ids
                .iter()
                .any(|task_id| validate_definition_id(task_id, "task").is_err())
        {
            return Err(ContentError::InvalidTownFacility(facility.id.clone()));
        }
        insert_definition_id(all_ids, &facility.id)?;
        facilities_by_id.insert(facility.id.clone(), facility.clone());
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
            || shop.inn_stay_cost.is_some_and(|cost| cost == 0)
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
                || stock.availability_percent == 0
                || stock.availability_percent > 100
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
        insert_definition_id(all_ids, &shop.id)?;
        insert_definition_id(all_ids, &shop.owner.id)?;
        shops_by_id.insert(shop.id.clone(), shop.clone());
    }

    for town in towns_by_id.values() {
        for facility_id in &town.facility_ids {
            let facility = facilities_by_id.get(facility_id).ok_or_else(|| {
                ContentError::DanglingReference {
                    owner: town.id.clone(),
                    target: facility_id.clone(),
                }
            })?;
            if facility.town_id != town.id {
                return Err(ContentError::InvalidTown(town.id.clone()));
            }
        }
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
        if town
            .shop_ids
            .iter()
            .filter_map(|shop_id| shops_by_id.get(shop_id))
            .filter(|shop| shop.inn_stay_cost.is_some())
            .count()
            > 1
        {
            return Err(ContentError::InvalidTown(town.id.clone()));
        }
    }
    for facility in facilities_by_id.values() {
        let town =
            towns_by_id
                .get(&facility.town_id)
                .ok_or_else(|| ContentError::DanglingReference {
                    owner: facility.id.clone(),
                    target: facility.town_id.clone(),
                })?;
        if !town.facility_ids.contains(&facility.id) {
            return Err(ContentError::InvalidTownFacility(facility.id.clone()));
        }
        if facility.category == TownFacilityCategory::Home {
            let storage_id = facility
                .storage_id
                .as_deref()
                .expect("validated Home must retain a storage id");
            let Some(storage) = facilities_by_id.get(storage_id) else {
                return Err(ContentError::DanglingReference {
                    owner: facility.id.clone(),
                    target: storage_id.to_owned(),
                });
            };
            if storage.category != TownFacilityCategory::Home
                || storage.storage_id.as_deref() != Some(storage.id.as_str())
            {
                return Err(ContentError::InvalidTownFacility(facility.id.clone()));
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
        facilities_by_id,
        shops_by_id,
    })
}
