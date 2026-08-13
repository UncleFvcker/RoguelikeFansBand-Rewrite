// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::tests::support::{dispatch_next, test_caster_game};

const GENERAL_STORE_ID: &str = "demo.shop.outpost-general-store";
const ARMOURY_ID: &str = "demo.shop.outpost-armoury";
const WEAPONSMITH_ID: &str = "demo.shop.outpost-weaponsmith";
const TEMPLE_ID: &str = "demo.shop.outpost-temple";
const ALCHEMIST_ID: &str = "demo.shop.outpost-alchemist";
const MAGIC_SHOP_ID: &str = "demo.shop.outpost-magic-shop";
const BLACK_MARKET_ID: &str = "demo.shop.outpost-black-market";
const BOOKSTORE_ID: &str = "demo.shop.outpost-bookstore";
const SHROOMERY_ID: &str = "demo.shop.outpost-shroomery";
const WHITE_HORSE_INN_ID: &str = "demo.shop.outpost-white-horse";
const HOME_ID: &str = "demo.town-facility.outpost-home";
const ANAMBAR_HOME_ID: &str = "demo.town-facility.anambar-home";
const ANAMBAR_INN_ID: &str = "demo.shop.anambar-inn";
const OUTPOST_COUNT_ID: &str = "demo.town-facility.outpost-count";

fn projected_shop<'a>(shops: &'a [ShopDto], shop_id: &str) -> &'a ShopDto {
    shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("requested shop should be projected")
}

fn store_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 32, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    game
}

fn anambar_inn_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = Position { x: 45, y: 15 };
    game.mark_shop_visited_at_player().unwrap();
    assert!(projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).player_at_entrance);
    game
}

fn white_horse_inn_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 63, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    assert!(projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID).player_at_entrance);
    game
}

fn outpost_count_game(seed: u64) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth game should start");
    game.player.position = Position { x: 26, y: 13 };
    game
}

fn stock_item_id(game: &Game, kind_id: &str) -> String {
    game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == kind_id)
        .expect("store should stock requested kind")
        .id
        .clone()
}

#[test]
fn outpost_shops_are_projected_from_authoritative_content() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let snapshot = game.snapshot();
    let town = snapshot.town.expect("surface should project the Outpost");
    assert_eq!(town.id, "demo.town.outpost");
    assert_eq!(town.floor_id, "demo.floor.surface");
    assert!(town.visited);
    assert_eq!(snapshot.shops.len(), 10);
    assert_eq!(snapshot.homes.len(), 1);
    assert_eq!(snapshot.homes[0].id, HOME_ID);
    assert_eq!(
        snapshot.homes[0].entrance_position,
        Position { x: 42, y: 13 }
    );
    assert!(!snapshot.homes[0].visited);
    let general_store = projected_shop(&snapshot.shops, GENERAL_STORE_ID);
    assert_eq!(general_store.entrance_position, Position { x: 32, y: 13 });
    assert_eq!(
        general_store.entrance_terrain_id,
        "demo.terrain.general-store-entrance"
    );
    assert_eq!(general_store.category, ShopCategoryDto::GeneralStore);
    let temple = projected_shop(&snapshot.shops, TEMPLE_ID);
    assert_eq!(temple.entrance_position, Position { x: 45, y: 19 });
    assert_eq!(temple.category, ShopCategoryDto::Temple);
    let alchemist = projected_shop(&snapshot.shops, ALCHEMIST_ID);
    assert_eq!(alchemist.entrance_position, Position { x: 53, y: 13 });
    assert_eq!(alchemist.category, ShopCategoryDto::Alchemist);
    let magic_shop = projected_shop(&snapshot.shops, MAGIC_SHOP_ID);
    assert_eq!(magic_shop.entrance_position, Position { x: 57, y: 13 });
    assert_eq!(magic_shop.category, ShopCategoryDto::MagicShop);
    let bookstore = projected_shop(&snapshot.shops, BOOKSTORE_ID);
    assert_eq!(bookstore.entrance_position, Position { x: 55, y: 13 });
    assert_eq!(bookstore.category, ShopCategoryDto::Bookstore);
    let armoury = projected_shop(&snapshot.shops, ARMOURY_ID);
    assert_eq!(armoury.entrance_position, Position { x: 30, y: 19 });
    assert_eq!(armoury.category, ShopCategoryDto::Armoury);
    let weaponsmith = projected_shop(&snapshot.shops, WEAPONSMITH_ID);
    assert_eq!(weaponsmith.entrance_position, Position { x: 34, y: 19 });
    assert_eq!(weaponsmith.category, ShopCategoryDto::Weaponsmith);
    let black_market = projected_shop(&snapshot.shops, BLACK_MARKET_ID);
    assert_eq!(black_market.entrance_position, Position { x: 55, y: 19 });
    assert_eq!(black_market.category, ShopCategoryDto::BlackMarket);
    let shroomery = projected_shop(&snapshot.shops, SHROOMERY_ID);
    assert_eq!(shroomery.entrance_position, Position { x: 61, y: 19 });
    assert_eq!(shroomery.category, ShopCategoryDto::Shroomery);
    let white_horse = projected_shop(&snapshot.shops, WHITE_HORSE_INN_ID);
    assert_eq!(white_horse.entrance_position, Position { x: 63, y: 13 });
    assert_eq!(white_horse.inn_stay_cost, Some(20));
    assert!(white_horse.inn_travel_destinations.is_empty());
    assert!(
        snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && !shop.player_at_entrance)
    );
}

#[test]
fn shroomery_trade_maintenance_and_save_round_trip_use_existing_shop_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Outpost game should start");
    game.gold = 10_000;
    game.player.position = Position { x: 61, y: 19 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, SHROOMERY_ID).clone();
    assert!(shop.visited && shop.player_at_entrance);
    assert_eq!(shop.owner.name_key, "shop-owner-demo-outpost-martin-name");
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "demo.item.cure-blindness-mushroom",
            "demo.item.cure-confusion-mushroom",
            "demo.item.cure-paranoia-mushroom",
            "demo.item.cure-poison-mushroom",
            "demo.item.fast-recovery-mushroom",
        ])
    );
    let mushroom = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.fast-recovery-mushroom")
        .expect("Shroomery should stock Fast Recovery")
        .clone();
    dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: SHROOMERY_ID.to_owned(),
            item_id: mushroom.id,
            quantity: 1,
        },
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.fast-recovery-mushroom"
            && item.location == ItemLocation::Inventory
    }));

    game.shop_states
        .get_mut(SHROOMERY_ID)
        .expect("Shroomery state should exist")
        .inventory
        .clear();
    game.world_tick = 10_000;
    game.maintain_shop_at_player().unwrap();
    assert_eq!(
        game.shop_states[SHROOMERY_ID]
            .inventory
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        5
    );

    let restored = Game::from_save(game.to_save()).expect("Shroomery state should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn birth_town_starts_without_surface_monsters() {
    let game = Game::new_with_build(0, "demo.build.warrior").expect("Warrens game should start");

    assert!(game.entities.is_empty());
}

#[test]
fn anambar_inn_stay_advances_half_day_and_restores_the_player() {
    let mut game = anambar_inn_game(42);
    game.world_tick = 12_345;
    game.gold = 100;
    game.player.hp = 1;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_HASTE, 20, "test.inn-rest").status);
    game.minor_slow = 3;
    game.minor_slow_energy = 41;
    game.reality_change_ticks = 20;
    game.resources
        .values_mut()
        .for_each(|pool| pool.current = 0);
    support::give_inventory_item(
        &mut game,
        "test.inn.device",
        "demo.item.detect-objects-staff",
    );
    let device = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.inn.device")
        .expect("test device should exist");
    device
        .charges
        .as_mut()
        .expect("test staff should have charges")
        .current = 0;
    device.device_recovery_progress = 500;
    let nutrition = game.nutrition;
    let draws = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::StayAtInn {
            facility_id: ANAMBAR_INN_ID.to_owned(),
        },
    );

    let event = update
        .events
        .iter()
        .find(|event| event.kind == "inn.stay")
        .expect("successful inn stay should be explicit");
    assert_eq!(event.args["cost"], "25");
    assert_eq!(event.args["balance"], "75");
    assert_eq!(event.args["elapsedTicks"], "37655");
    assert_eq!(game.world_tick, 50_000);
    assert_eq!(game.gold, 75);
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert!(game.player.statuses.is_empty());
    assert_eq!((game.minor_slow, game.minor_slow_energy), (0, 0));
    assert_eq!(game.reality_change_ticks, 0);
    assert!(
        game.resources
            .values()
            .all(|pool| pool.current == pool.maximum)
    );
    let device = game
        .items
        .iter()
        .find(|item| item.id == "test.inn.device")
        .expect("test device should remain carried");
    let charges = device.charges.expect("test staff should retain charges");
    assert_eq!(charges.current, charges.maximum);
    assert_eq!(device.device_recovery_progress, 0);
    assert_eq!(game.nutrition, nutrition);
    assert_eq!(game.rng_draw_counter(), draws);

    let restored = Game::from_save(game.to_save()).expect("inn result should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn anambar_inn_rejections_do_not_charge_or_advance_time() {
    for status_kind_id in [STATUS_POISON, STATUS_BLEEDING] {
        let mut game = anambar_inn_game(42);
        game.gold = 100;
        game.world_tick = 12_345;
        game.player
            .statuses
            .push(monster_combat::melee_status(status_kind_id, 20, "test.inn-rest").status);
        let draws = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::StayAtInn {
                facility_id: ANAMBAR_INN_ID.to_owned(),
            },
        );

        let event = update
            .events
            .iter()
            .find(|event| event.kind == "inn.stay-unavailable")
            .expect("unsafe inn stay should be rejected");
        assert_eq!(event.args["reason"], "needs-healer");
        assert_eq!(game.gold, 100);
        assert_eq!(game.world_tick, 12_345);
        assert_eq!(game.rng_draw_counter(), draws);
        assert!(game.player_has_status_kind(status_kind_id));
    }

    let mut poor = anambar_inn_game(42);
    poor.gold = 24;
    let tick = poor.world_tick;
    let update = dispatch_next(
        &mut poor,
        GameCommand::StayAtInn {
            facility_id: ANAMBAR_INN_ID.to_owned(),
        },
    );
    assert_eq!(update.events[0].args["reason"], "insufficient-gold");
    assert_eq!(poor.gold, 24);
    assert_eq!(poor.world_tick, tick);
}

#[test]
fn white_horse_inn_uses_its_content_price() {
    let mut game = white_horse_inn_game(42);
    game.gold = 20;
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == "demo.town-facility.outpost-white-horse")
        .expect("White Horse should expose its task service at the inn entrance");
    assert!(service.player_at_entrance);
    assert_eq!(
        service.tasks.first().map(|task| task.task_id.as_str()),
        Some("demo.task.trouble-at-home")
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::StayAtInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
        },
    );

    let event = update
        .events
        .iter()
        .find(|event| event.kind == "inn.stay")
        .expect("White Horse lodging should succeed");
    assert_eq!(event.args["cost"], "20");
    assert_eq!(event.args["balance"], "0");
    assert_eq!(game.gold, 0);
}

#[test]
fn inn_travel_requires_a_visited_town_and_arrives_at_its_inn() {
    let mut unvisited = white_horse_inn_game(42);
    unvisited.gold = 500;
    let rejected = dispatch_next(
        &mut unvisited,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: "demo.town.anambar".to_owned(),
        },
    );
    assert_eq!(rejected.events[0].kind, "inn.travel-unavailable");
    assert_eq!(rejected.events[0].args["reason"], "town-unvisited");
    assert_eq!(unvisited.gold, 500);
    assert_eq!(
        unvisited.wilderness_position,
        Some(Position { x: 28, y: 52 })
    );

    let mut game = anambar_inn_game(42);
    game.gold = 1_000;
    let to_outpost = dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: ANAMBAR_INN_ID.to_owned(),
            destination_town_id: "demo.town.outpost".to_owned(),
        },
    );
    assert_eq!(to_outpost.events[0].kind, "inn.travel");
    assert_eq!(to_outpost.events[0].args["cost"], "500");
    assert_eq!(game.gold, 500);
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(game.player.position, Position { x: 63, y: 13 });
    let white_horse = projected_shop(&game.snapshot().shops, WHITE_HORSE_INN_ID).clone();
    assert!(white_horse.player_at_entrance);
    assert_eq!(
        white_horse.inn_travel_destinations[0].town_id,
        "demo.town.anambar"
    );

    let to_anambar = dispatch_next(
        &mut game,
        GameCommand::TravelFromInn {
            facility_id: WHITE_HORSE_INN_ID.to_owned(),
            destination_town_id: "demo.town.anambar".to_owned(),
        },
    );
    assert_eq!(to_anambar.events[0].kind, "inn.travel");
    assert_eq!(game.gold, 0);
    assert_eq!(game.wilderness_position, Some(Position { x: 26, y: 39 }));
    assert_eq!(game.player.position, Position { x: 45, y: 15 });
    assert!(projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).player_at_entrance);
}

#[test]
fn outpost_count_identifies_carried_items_for_fifty_gold() {
    let mut game = outpost_count_game(42);
    game.gold = 100;
    let item_id = game
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Inventory)
        .expect("warrior should start with a carried item")
        .id
        .clone();
    game.item_property_knowledge.remove(&item_id);

    let before_tick = game.world_tick;
    let update = dispatch_next(
        &mut game,
        GameCommand::IdentifyAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            item_id: item_id.clone(),
        },
    );
    assert_eq!(game.gold, 50);
    assert_eq!(game.world_tick, before_tick);
    assert!(game.item_property_knowledge[&item_id].appraised);
    assert!(update.events.iter().any(|event| {
        event.kind == "facility.identified"
            && event.args.get("cost").is_some_and(|cost| cost == "50")
    }));

    let rejected = dispatch_next(
        &mut game,
        GameCommand::IdentifyAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            item_id,
        },
    );
    assert_eq!(game.gold, 50);
    assert_eq!(rejected.events[0].kind, "facility.identify-unavailable");
}

#[test]
fn outpost_count_legal_name_change_is_validated_saved_and_projected() {
    let mut game = outpost_count_game(42);
    game.gold = 20;
    let service = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == OUTPOST_COUNT_ID)
        .expect("Count service should be projected at its entrance");
    assert_eq!(service.identify_item_cost, Some(50));
    assert_eq!(service.legal_name_change_cost, Some(10));

    let update = dispatch_next(
        &mut game,
        GameCommand::RenameAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            name: "  Elessar  ".to_owned(),
        },
    );
    assert_eq!(game.gold, 10);
    assert_eq!(game.snapshot().player.name, "Elessar");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "facility.renamed")
    );

    let restored = Game::from_save(game.to_save()).expect("renamed player should round-trip");
    assert_eq!(restored.snapshot().player.name, "Elessar");
    assert_eq!(restored.gold, 10);

    let mut invalid = outpost_count_game(43);
    invalid.gold = 20;
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::RenameAtFacility {
            facility_id: OUTPOST_COUNT_ID.to_owned(),
            name: "\n".to_owned(),
        },
    );
    assert_eq!(invalid.gold, 20);
    assert_eq!(rejected.events[0].kind, "facility.rename-unavailable");
}

#[test]
fn outpost_temple_has_walkable_space_on_both_sides_and_to_the_south() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");

    for y in 19..=24 {
        for x in [38, 52] {
            assert_eq!(
                game.terrain_at(Position { x, y }),
                "demo.terrain.surface-grass",
                "temple side passage at ({x}, {y}) should remain walkable"
            );
        }
    }
    for x in 38..=52 {
        assert_eq!(
            game.terrain_at(Position { x, y: 24 }),
            "demo.terrain.surface-grass",
            "temple south passage at ({x}, 24) should remain walkable"
        );
    }
}

#[test]
fn home_deposit_withdraw_grouping_and_save_are_authoritative() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 42, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    let home = game.snapshot().homes[0].clone();
    assert!(home.visited);
    assert!(home.player_at_entrance);
    assert!(home.stored_items.is_empty());
    let ration = home
        .deposit_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    let initial_ration_quantity = ration.quantity;
    game.item_property_knowledge.insert(
        ration.id.clone(),
        ItemPropertyKnowledgeState {
            discovered: true,
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::new(),
        },
    );
    let gold_before = game.gold;
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let deposit = dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.to_owned(),
            item_id: ration.id,
            quantity: 1,
        },
    );
    assert!(
        deposit
            .events
            .iter()
            .any(|event| event.kind == "home.deposit")
    );
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    let stored = game.snapshot().homes[0]
        .stored_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("deposited ration should be stored")
        .clone();
    assert_eq!(stored.quantity, 1);

    let restored = Game::from_save(game.to_save()).expect("home inventory should round-trip");
    assert_eq!(restored.home_states, game.home_states);
    let mut game = restored;
    let withdrawal = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: HOME_ID.to_owned(),
            item_id: stored.id,
            quantity: 1,
        },
    );
    assert!(
        withdrawal
            .events
            .iter()
            .any(|event| event.kind == "home.withdraw")
    );
    assert!(game.snapshot().homes[0].stored_items.is_empty());
    let carried_rations = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .collect::<Vec<_>>();
    assert_eq!(carried_rations.len(), 1);
    assert_eq!(carried_rations[0].quantity, initial_ration_quantity);
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn anambar_home_uses_the_outpost_home_inventory() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 42, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    let ration = game.snapshot().homes[0]
        .deposit_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    dispatch_next(
        &mut game,
        GameCommand::DepositAtHome {
            facility_id: HOME_ID.to_owned(),
            item_id: ration.id,
            quantity: 1,
        },
    );

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 26, y: 39 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.home_states.len(), 1);
    assert!(game.home_states.contains_key(HOME_ID));
    let town_snapshot = game.snapshot();
    assert_eq!(town_snapshot.shops.len(), 9);
    assert!(
        town_snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && shop.stock.is_empty())
    );
    assert!(
        !game
            .shop_states
            .keys()
            .any(|shop_id| shop_id.starts_with("demo.shop.anambar-"))
    );

    game.player.position = Position { x: 45, y: 15 };
    game.mark_shop_visited_at_player().unwrap();
    let inn = projected_shop(&game.snapshot().shops, ANAMBAR_INN_ID).clone();
    assert!(inn.visited && inn.player_at_entrance && !inn.stock.is_empty());
    assert!(inn.stock.iter().all(|item| {
        [
            "demo.item.ration-of-food",
            "demo.item.water-potion",
            "demo.item.apple-juice",
            "demo.item.pint-of-fine-ale",
            "demo.item.pint-of-fine-wine",
        ]
        .contains(&item.kind_id.as_str())
    }));

    game.player.position = Position { x: 37, y: 15 };
    game.mark_shop_visited_at_player().unwrap();
    let home = game
        .snapshot()
        .homes
        .into_iter()
        .find(|home| home.id == ANAMBAR_HOME_ID)
        .expect("Anambar Home should be projected");
    let stored = home
        .stored_items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Outpost deposit should be visible in Anambar");
    dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: ANAMBAR_HOME_ID.to_owned(),
            item_id: stored.id.clone(),
            quantity: 1,
        },
    );

    assert!(game.home_states[HOME_ID].inventory.is_empty());
    let restored = Game::from_save(game.to_save()).expect("shared Home should round-trip");
    assert_eq!(restored.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(restored.home_states, game.home_states);
}

#[test]
fn overburdened_player_can_withdraw_from_home() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 42, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    support::give_inventory_item(&mut game, "test.heavy-stack", "demo.item.burdened-mail");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.heavy-stack")
        .expect("fixture item should exist")
        .quantity = 11;
    let mut stored = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Warrior should carry rations")
        .clone();
    stored.id = "test.home-ration".to_owned();
    stored.quantity = 1;
    stored.location = ItemLocation::Home {
        facility_id: HOME_ID.to_owned(),
    };
    let item_id = stored.id.clone();
    game.home_states
        .get_mut(HOME_ID)
        .expect("Home should have authoritative state")
        .inventory
        .push(stored);

    let update = dispatch_next(
        &mut game,
        GameCommand::WithdrawFromHome {
            facility_id: HOME_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );

    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "home.withdraw")
    );
    assert!(update.player.carried_weight_tenths_pound > update.player.carry_capacity_tenths_pound);
}

#[test]
fn home_inventory_ids_are_reserved_by_the_global_allocator() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let reserved_id = format!("{GENERATED_ITEM_ID_PREFIX}9000");
    let mut stored = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .expect("Warrior should carry rations")
        .clone();
    stored.id = reserved_id.clone();
    stored.quantity = 1;
    stored.location = ItemLocation::Home {
        facility_id: HOME_ID.to_owned(),
    };
    game.home_states
        .get_mut(HOME_ID)
        .expect("Home should have authoritative state")
        .inventory
        .push(stored);
    game.next_item_instance_serial = 9000;

    assert_eq!(
        game.allocate_item_instance_id()
            .expect("allocator should skip Home inventory IDs"),
        format!("{GENERATED_ITEM_ID_PREFIX}9001")
    );
}

#[test]
fn entering_general_store_entrance_marks_persistent_shop_visit() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.player.position = Position { x: 32, y: 14 };

    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::North,
        },
    );
    let general_store = projected_shop(&update.shops, GENERAL_STORE_ID);
    assert!(general_store.visited);
    assert!(general_store.player_at_entrance);
    assert!(
        update
            .shops
            .iter()
            .filter(|shop| shop.player_at_entrance)
            .all(|shop| shop.id == GENERAL_STORE_ID)
    );

    let restored = Game::from_save(game.to_save()).expect("shop visit should round-trip");
    assert!(projected_shop(&restored.snapshot().shops, GENERAL_STORE_ID).visited);
}

#[test]
fn malformed_town_state_is_rejected() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.town_states[0].visited = false;
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("town state is invalid"))
    ));
}

#[test]
fn runtime_town_validation_requires_complete_home_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.home_states.clear();
    assert!(matches!(
        game.validate_loaded_state(),
        Err(CoreError::InvalidSave("home state is invalid"))
    ));
}

#[test]
fn missing_unentered_shop_state_is_created_on_first_entry() {
    let game = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.shop_states.clear();
    let mut restored = Game::from_save(payload).expect("unentered shop state may remain sparse");
    assert!(restored.shop_states.is_empty());

    restored.player.position = Position { x: 32, y: 14 };
    let update = dispatch_next(
        &mut restored,
        GameCommand::Move {
            direction: Direction::North,
        },
    );
    assert!(restored.shop_states.contains_key(GENERAL_STORE_ID));
    assert!(projected_shop(&update.shops, GENERAL_STORE_ID).visited);
}

#[test]
fn initial_shop_stock_is_seeded_independent_and_persistent() {
    let left = Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    let right = Game::new_with_build(42, "demo.build.warrior").expect("same seed should start");
    assert_eq!(left.shop_states, right.shop_states);
    let town = left
        .content
        .town("demo.town.outpost")
        .expect("Outpost should exist");
    for shop_id in &town.shop_ids {
        let expected_kinds = left
            .content
            .shop(shop_id)
            .expect("town shop should exist")
            .stock
            .iter()
            .map(|stock| stock.item_kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            left.shop_states[shop_id.as_str()]
                .inventory
                .iter()
                .map(|item| item.kind_id.as_str())
                .collect::<std::collections::BTreeSet<_>>(),
            expected_kinds
        );
    }
    let restored = Game::from_save(left.to_save()).expect("store stock should round-trip");
    assert_eq!(restored.shop_states, left.shop_states);
    assert_eq!(restored.state_hash(), left.state_hash());
}

#[test]
fn current_warrior_uses_rfb_price_factor_and_trade_values() {
    let game = store_game(42);
    let snapshot = game.snapshot();
    let shop = projected_shop(&snapshot.shops, GENERAL_STORE_ID);
    assert_eq!(shop.owner.price_factor_percent, 100);
    let buy_prices = shop
        .stock
        .iter()
        .map(|item| (item.kind_id.as_str(), item.unit_price))
        .collect::<std::collections::BTreeMap<_, _>>();
    assert_eq!(buy_prices["demo.item.ration-of-food"], 3);
    assert_eq!(buy_prices["demo.item.wooden-torch"], 1);
    assert_eq!(buy_prices["demo.item.brass-lantern"], 30);
    assert_eq!(buy_prices["demo.item.flask-of-oil"], 3);
    assert_eq!(super::super::town::sell_unit_price(3, 100, 500), 2);
    assert_eq!(super::super::town::sell_unit_price(1, 100, 500), 1);
    assert_eq!(super::super::town::sell_unit_price(30, 100, 500), 28);
    assert_eq!(super::super::town::sell_unit_price(1_000, 100, 7), 7);
}

#[test]
fn player_made_ammunition_keeps_its_ninety_nine_percent_shop_discount() {
    let mut game = store_game(44);
    let ammunition = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Warrior should carry arrows");
    ammunition.kind_id = "demo.item.mithril-arrow".to_owned();
    ammunition.quantity = 1;
    ammunition.origin_kind = Some(ItemOriginKindDto::PlayerMade);
    ammunition.discount_percent = 99;
    let item_id = ammunition.id.clone();

    let before_sale = game.snapshot();
    let quote = projected_shop(&before_sale.shops, GENERAL_STORE_ID)
        .sell_quotes
        .iter()
        .find(|quote| quote.item_id == item_id)
        .expect("player-made ammunition should be legal shop stock");
    assert_eq!(quote.unit_price, 1);
    let gold_before = game.gold;
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(game.gold, gold_before + 1);
    let after_sale = game.snapshot();
    let stock = projected_shop(&after_sale.shops, GENERAL_STORE_ID)
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.mithril-arrow")
        .expect("sold player-made ammunition should remain discounted in stock");
    assert_eq!(stock.unit_price, 1);
}

#[test]
fn black_market_uses_original_warrior_markup_and_markdown() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 1_000_000;
    game.player.position = Position { x: 55, y: 19 };
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let shop = projected_shop(&snapshot.shops, BLACK_MARKET_ID);
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.owner.greed_percent, 150);
    assert_eq!(shop.owner.purchase_price_cap, 30_000);
    assert_eq!(shop.owner.price_factor_percent, 140);
    let black_channels = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.black-channels")
        .expect("Black Market should stock Black Channels");
    assert_eq!(black_channels.unit_price, 42_000);
    let ration = shop
        .sell_quotes
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("Black Market should buy ordinary legal inventory");
    assert_eq!(ration.unit_price, 1);

    assert!(game.gain_mutation("rfb.mutation.black-marketeer", &mut Vec::new()));
    let discounted = game.snapshot();
    let shop = projected_shop(&discounted.shops, BLACK_MARKET_ID);
    assert_eq!(
        shop.stock
            .iter()
            .find(|item| item.kind_id == "demo.item.black-channels")
            .expect("Black Market should retain Black Channels")
            .unit_price,
        21_000
    );
    assert_eq!(
        shop.sell_quotes
            .iter()
            .find(|item| item.kind_id == "demo.item.ration-of-food")
            .expect("Black Market should retain the ration quote")
            .unit_price,
        2
    );

    let restored = Game::from_save(game.to_save()).expect("Black Market should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn temple_purchase_and_alchemist_visit_use_independent_shop_state() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 1_000;
    game.player.position = Position { x: 45, y: 19 };
    game.mark_shop_visited_at_player().unwrap();
    let temple_snapshot = game.snapshot();
    let temple = projected_shop(&temple_snapshot.shops, TEMPLE_ID);
    assert!(temple.visited);
    assert!(temple.player_at_entrance);
    assert_eq!(temple.owner.price_factor_percent, 101);
    let healing = temple
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.light-healing-potion")
        .expect("Temple should stock light healing")
        .clone();
    assert_eq!(
        healing.display_name_key, "item-demo-light-healing-potion-name",
        "shop stock should use its known item name without revealing it globally"
    );
    assert_ne!(
        game.item_display_name_key("demo.item.light-healing-potion"),
        healing.display_name_key
    );
    let alchemist_before = game.shop_states[ALCHEMIST_ID].clone();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: TEMPLE_ID.to_owned(),
            item_id: healing.id,
            quantity: 1,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert_eq!(game.gold, 980);
    assert_eq!(game.shop_states[ALCHEMIST_ID], alchemist_before);

    game.player.position = Position { x: 53, y: 13 };
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let alchemist = projected_shop(&snapshot.shops, ALCHEMIST_ID);
    assert!(alchemist.visited);
    assert!(alchemist.player_at_entrance);
    assert_eq!(alchemist.owner.price_factor_percent, 103);
    assert!(
        alchemist
            .stock
            .iter()
            .any(|item| item.kind_id == "demo.item.flicker-scroll")
    );
    assert!(!projected_shop(&snapshot.shops, TEMPLE_ID).player_at_entrance);

    let restored = Game::from_save(game.to_save()).expect("seven shops should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn bookstore_purchase_can_supply_an_original_spellbook_for_study() {
    let mut game = test_caster_game(42);
    game.gold = 10_000;
    game.items
        .retain(|item| item.location != ItemLocation::Inventory);
    game.player.position = Position { x: 55, y: 13 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, BOOKSTORE_ID).clone();
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.category, ShopCategoryDto::Bookstore);
    assert_eq!(shop.owner.greed_percent, 108);
    assert_eq!(shop.owner.purchase_price_cap, 10_000);
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| (item.kind_id.as_str(), item.unit_price))
            .collect::<std::collections::BTreeMap<_, _>>(),
        std::collections::BTreeMap::from([
            ("demo.item.black-prayers", 100),
            ("demo.item.black-mass", 1_000),
            ("demo.item.cantrips-for-beginners", 100),
            ("demo.item.minor-arcana", 250),
            ("demo.item.major-arcana", 1_000),
            ("demo.item.manual-of-mastery", 2_500),
        ])
    );
    let book = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.black-prayers")
        .expect("Bookstore should stock Black Prayers")
        .clone();

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: BOOKSTORE_ID.to_owned(),
            item_id: book.id,
            quantity: 1,
        },
    );
    let book_item_id = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.black-prayers")
        .expect("purchased book should be carried")
        .id
        .clone();
    let studied = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.death-detect-evil".to_owned(),
        },
    );
    assert!(
        studied
            .player
            .abilities
            .iter()
            .any(|ability| { ability.id == "demo.ability.death-detect-evil" && ability.learned })
    );

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("bookstore trade should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn shared_forge_shops_group_stock_and_sell_equipment_that_can_be_used() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 10_000;
    let mut extra_arrows = game.shop_states[WEAPONSMITH_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow")
        .expect("Weaponsmith should stock arrows")
        .clone();
    extra_arrows.id = game
        .allocate_item_instance_id()
        .expect("test arrow stock should allocate a unique item ID");
    extra_arrows.quantity = 99;
    game.shop_states
        .get_mut(WEAPONSMITH_ID)
        .expect("weaponsmith state should exist")
        .inventory
        .push(extra_arrows);
    game.player.position = Position { x: 34, y: 19 };
    game.mark_shop_visited_at_player().unwrap();

    let weaponsmith = projected_shop(&game.snapshot().shops, WEAPONSMITH_ID).clone();
    assert!(weaponsmith.visited);
    assert!(weaponsmith.player_at_entrance);
    assert_eq!(weaponsmith.owner.greed_percent, 110);
    assert_eq!(weaponsmith.owner.purchase_price_cap, 20_000);
    assert_eq!(
        weaponsmith
            .stock
            .iter()
            .filter(|item| item.kind_id == "demo.item.arrow")
            .count(),
        1,
        "compatible arrow stacks should project as one shop entry"
    );
    assert!(
        weaponsmith
            .stock
            .iter()
            .find(|item| item.kind_id == "demo.item.arrow")
            .is_some_and(|item| item.quantity > 99),
        "compatible arrow instances should group across the stack limit"
    );

    game.player.position = Position { x: 30, y: 19 };
    game.mark_shop_visited_at_player().unwrap();
    let armoury = projected_shop(&game.snapshot().shops, ARMOURY_ID).clone();
    assert!(armoury.visited);
    assert!(armoury.player_at_entrance);
    assert_eq!(armoury.owner.greed_percent, 111);
    assert_eq!(armoury.owner.purchase_price_cap, 20_000);
    let gloves = armoury
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.leather-gloves")
        .expect("Armoury should stock RFB Leather Gloves")
        .clone();
    assert_eq!(gloves.unit_price, 3);

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: ARMOURY_ID.to_owned(),
            item_id: gloves.id,
            quantity: 1,
        },
    );
    let glove_id = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.leather-gloves")
        .expect("purchased gloves should be carried")
        .id
        .clone();
    let equipped = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: glove_id,
            slot_id: None,
        },
    );
    assert!(
        equipped
            .equipment
            .iter()
            .any(|item| { item.kind_id == "demo.item.leather-gloves" && item.slot_id == "hands" })
    );

    let restored = Game::from_save(game.to_save()).expect("forge trade should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn magic_shop_purchase_device_use_and_save_are_authoritative() {
    let mut game =
        Game::new_with_build(43, "demo.build.warrior").expect("Warrens game should start");
    game.gold = 10_000;
    game.player.position = Position { x: 57, y: 13 };
    game.mark_shop_visited_at_player().unwrap();

    let shop = projected_shop(&game.snapshot().shops, MAGIC_SHOP_ID).clone();
    assert!(shop.visited);
    assert!(shop.player_at_entrance);
    assert_eq!(shop.category, ShopCategoryDto::MagicShop);
    assert_eq!(shop.owner.price_factor_percent, 102);
    assert_eq!(
        shop.stock
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            "demo.item.magic-missile-wand",
            "demo.item.detect-objects-staff",
            "demo.item.identify-staff",
        ])
    );
    let staff = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.detect-objects-staff")
        .expect("Magic Shop should stock the RFB Detect Objects Staff")
        .clone();
    assert_eq!(
        staff.display_name_key,
        "item-demo-detect-objects-staff-name"
    );
    assert_eq!(staff.unit_price, 1_530);

    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: MAGIC_SHOP_ID.to_owned(),
            item_id: staff.id,
            quantity: 1,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let bought = purchase
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.detect-objects-staff")
        .expect("purchased staff should be carried");
    let charges_before = bought
        .charges
        .expect("purchased staff should retain its generated energy");
    assert_eq!(charges_before.maximum, 45);
    assert!(charges_before.current >= 4);
    let staff_id = bought.id.clone();

    game.rng = RfbRng::seeded(32);
    let used = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: staff_id.clone(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(
        used.events
            .iter()
            .any(|event| event.kind == "skill.device-success")
    );
    assert_eq!(
        used.inventory
            .iter()
            .find(|item| item.id == staff_id)
            .and_then(|item| item.charges),
        Some(ItemChargesDto {
            current: charges_before.current - 4,
            maximum: charges_before.maximum,
        })
    );

    let restored = Game::from_save(game.to_save()).expect("Magic Shop device state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn quantity_purchase_is_atomic_zero_time_and_identified() {
    let mut game = store_game(43);
    game.gold = 100;
    let item_id = stock_item_id(&game, "demo.item.ration-of-food");
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .expect("general store should exist")
        .inventory
        .iter_mut()
        .find(|item| item.id == item_id)
        .expect("selected ration stock should remain available")
        .quantity = 2;
    let ration_before = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .expect("warrior should start with rations")
        .clone();
    let stack_count_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == ration_before.kind_id && item.location == ItemLocation::Inventory
        })
        .count();
    let before_tick = game.world_tick;
    let before_draws = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 2,
        },
    );
    assert_eq!(game.gold, 94);
    assert_eq!(game.world_tick, before_tick);
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let bought = game
        .items
        .iter()
        .find(|item| item.id == ration_before.id)
        .expect("purchase should merge into the existing ration stack");
    assert_eq!(bought.quantity, ration_before.quantity + 2);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == ration_before.kind_id && item.location == ItemLocation::Inventory
            })
            .count(),
        stack_count_before
    );
    assert_eq!(
        game.item_knowledge_dto(&bought.kind_id),
        ItemKnowledgeDto::Aware
    );
    assert!(game.item_property_knowledge[&bought.id].identified);
}

#[test]
fn rejected_purchase_preserves_rng_and_business_state() {
    let mut game = store_game(42);
    game.gold = 0;
    let item_id = stock_item_id(&game, "demo.item.brass-lantern");
    let business_before = game.shop_states.clone();
    let items_before = game.items.clone();
    let gold_before = game.gold;
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "insufficient-gold")
    }));
    assert_eq!(game.shop_states, business_before);
    assert_eq!(game.items, items_before);
    assert_eq!(game.gold, gold_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn overburdened_player_can_purchase() {
    let mut game = store_game(42);
    game.gold = 100;
    for item in &mut game.items {
        if item.location == ItemLocation::Inventory {
            item.quantity = game.content.item(&item.kind_id).unwrap().max_stack;
        }
    }
    let item_id = stock_item_id(&game, "demo.item.brass-lantern");
    let update = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id,
            quantity: 1,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.brass-lantern" && item.location == ItemLocation::Inventory
    }));
}

#[test]
fn corpse_sale_is_rejected() {
    let mut game = store_game(42);
    support::give_inventory_item(&mut game, "test.corpse", "demo.item.corpse-remains");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: "test.corpse".to_owned(),
            quantity: 1,
        },
    );
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "item-illegal")
    }));
    assert!(game.items.iter().any(|item| item.id == "test.corpse"));
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn sold_item_can_be_bought_back_with_full_instance_state() {
    let mut game = store_game(42);
    game.gold = 100;
    let original = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.wooden-torch")
        .map(|item| {
            item.fuel
                .as_mut()
                .expect("starting torch must have fuel")
                .current = 1_234;
            item.inscription = Some("@m1".to_owned());
            item.clone()
        })
        .expect("warrior should start with torches")
        .clone();
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: original.id.clone(),
            quantity: 1,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(
        sale.shops
            .iter()
            .find(|shop| shop.id == GENERAL_STORE_ID)
            .and_then(|shop| shop.stock.iter().find(|item| item.id == original.id))
            .and_then(|item| item.inscription.as_deref()),
        Some("@m1")
    );
    let sold = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.id == original.id)
        .expect("sold item should enter store")
        .clone();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: sold.id.clone(),
            quantity: 1,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    let repurchased = game
        .items
        .iter()
        .find(|item| item.id == sold.id)
        .expect("repurchased item should retain full instance");
    assert_eq!(repurchased.kind_id, original.kind_id);
    assert_eq!(repurchased.fuel, original.fuel);
    assert_eq!(repurchased.charges, original.charges);
    assert_eq!(repurchased.inscription, original.inscription);
    assert!(game.item_property_knowledge[&repurchased.id].identified);
}

#[test]
fn compatible_shop_instances_project_and_trade_as_one_row() {
    let mut game = store_game(42);
    game.gold = 100;
    let mut extra_ration = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("store should stock rations")
        .clone();
    extra_ration.id = "demo.shop.outpost-general-store.test-ration".to_owned();
    extra_ration.quantity = 1;
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .expect("general store state should exist")
        .inventory
        .push(extra_ration);
    let shop_before = projected_shop(&game.snapshot().shops, GENERAL_STORE_ID).clone();
    for kind_id in [
        "demo.item.ration-of-food",
        "demo.item.wooden-torch",
        "demo.item.brass-lantern",
        "demo.item.flask-of-oil",
    ] {
        assert_eq!(
            shop_before
                .stock
                .iter()
                .filter(|item| item.kind_id == kind_id)
                .count(),
            1,
            "compatible {kind_id} stock should use one row"
        );
    }

    let ration = shop_before
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("store should stock rations");
    assert!(ration.maximum_quantity >= 2);
    let ration_item_id = ration.id.clone();
    let shop_rations_before = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .filter(|item| item.kind_id == "demo.item.ration-of-food")
        .map(|item| item.quantity)
        .sum::<u32>();
    let carried_rations_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
        })
        .map(|item| item.quantity)
        .sum::<u32>();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: ration_item_id,
            quantity: 2,
        },
    );
    assert!(
        purchase
            .events
            .iter()
            .any(|event| event.kind == "shop.purchase")
    );
    assert_eq!(
        game.shop_states[GENERAL_STORE_ID]
            .inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.ration-of-food")
            .map(|item| item.quantity)
            .sum::<u32>(),
        shop_rations_before - 2
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.ration-of-food"
                    && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_rations_before + 2
    );

    let shop_after_purchase = projected_shop(&game.snapshot().shops, GENERAL_STORE_ID).clone();
    let quote = shop_after_purchase
        .sell_quotes
        .iter()
        .find(|quote| quote.kind_id == "demo.item.ration-of-food" && quote.maximum_quantity >= 2)
        .expect("compatible carried rations should have one grouped quote");
    let sale = dispatch_next(
        &mut game,
        GameCommand::SellToShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: quote.item_id.clone(),
            quantity: 2,
        },
    );
    assert!(sale.events.iter().any(|event| event.kind == "shop.sale"));
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.ration-of-food"
                    && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_rations_before
    );
    assert_eq!(
        projected_shop(&game.snapshot().shops, GENERAL_STORE_ID)
            .stock
            .iter()
            .filter(|item| item.kind_id == "demo.item.ration-of-food")
            .count(),
        1
    );
}

#[test]
fn maintenance_refills_only_after_interval_at_entrance() {
    let mut game = store_game(42);
    game.shop_states
        .get_mut(GENERAL_STORE_ID)
        .unwrap()
        .inventory
        .clear();
    game.world_tick = 9_999;
    let draws_before = game.rng_draw_counter();
    game.maintain_shop_at_player().unwrap();
    assert!(game.shop_states[GENERAL_STORE_ID].inventory.is_empty());
    assert_eq!(game.rng_draw_counter(), draws_before);
    game.world_tick = 10_000;
    game.maintain_shop_at_player().unwrap();
    let state = &game.shop_states[GENERAL_STORE_ID];
    let defined_stock_count = game
        .content
        .shop(GENERAL_STORE_ID)
        .expect("General Store should exist")
        .stock
        .len();
    assert_eq!(state.last_maintenance_world_tick, 10_000);
    assert_eq!(
        state
            .inventory
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        defined_stock_count
    );
    assert!(game.rng_draw_counter() > draws_before);
}
