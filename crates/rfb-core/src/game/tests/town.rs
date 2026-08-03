// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::tests::support::dispatch_next;

const GENERAL_STORE_ID: &str = "demo.shop.outpost-general-store";
const TEMPLE_ID: &str = "demo.shop.outpost-temple";
const ALCHEMIST_ID: &str = "demo.shop.outpost-alchemist";

fn projected_shop<'a>(shops: &'a [ShopDto], shop_id: &str) -> &'a ShopDto {
    shops
        .iter()
        .find(|shop| shop.id == shop_id)
        .expect("requested shop should be projected")
}

fn store_game(seed: u64) -> Game {
    let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
        .expect("Warrens game should start");
    game.player.position = Position { x: 16, y: 8 };
    game.mark_shop_visited_at_player();
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
fn outpost_supply_court_is_projected_from_authoritative_content() {
    let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    let snapshot = game.snapshot();
    let town = snapshot.town.expect("surface should project the Outpost");
    assert_eq!(town.id, "demo.town.outpost");
    assert_eq!(town.floor_id, "demo.floor.surface");
    assert!(town.visited);
    assert_eq!(snapshot.shops.len(), 3);
    let general_store = projected_shop(&snapshot.shops, GENERAL_STORE_ID);
    assert_eq!(general_store.entrance_position, Position { x: 16, y: 8 });
    assert_eq!(
        general_store.entrance_terrain_id,
        "demo.terrain.general-store-entrance"
    );
    assert_eq!(general_store.category, ShopCategoryDto::GeneralStore);
    let temple = projected_shop(&snapshot.shops, TEMPLE_ID);
    assert_eq!(temple.entrance_position, Position { x: 20, y: 8 });
    assert_eq!(temple.category, ShopCategoryDto::Temple);
    let alchemist = projected_shop(&snapshot.shops, ALCHEMIST_ID);
    assert_eq!(alchemist.entrance_position, Position { x: 24, y: 8 });
    assert_eq!(alchemist.category, ShopCategoryDto::Alchemist);
    assert!(
        snapshot
            .shops
            .iter()
            .all(|shop| !shop.visited && !shop.player_at_entrance)
    );
}

#[test]
fn entering_general_store_entrance_marks_persistent_shop_visit() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    game.player.position = Position { x: 16, y: 9 };

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
    let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.town_states[0].visited = false;
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("town state is invalid"))
    ));
}

#[test]
fn shop_state_is_required_for_development_saves() {
    let game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    let mut payload = game.to_save();
    payload.shop_states.clear();
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("shop state is invalid"))
    ));
}

#[test]
fn initial_shop_stock_is_seeded_independent_and_persistent() {
    let left = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    let right = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("same seed should start");
    assert_eq!(left.shop_states, right.shop_states);
    let expected = [
        (
            GENERAL_STORE_ID,
            std::collections::BTreeSet::from([
                "demo.item.ration-of-food",
                "demo.item.wooden-torch",
                "demo.item.brass-lantern",
                "demo.item.flask-of-oil",
            ]),
        ),
        (
            TEMPLE_ID,
            std::collections::BTreeSet::from([
                "demo.item.light-healing-potion",
                "demo.item.valor-tonic",
                "demo.item.homeward-scroll",
                "demo.item.cleansing-scroll",
            ]),
        ),
        (
            ALCHEMIST_ID,
            std::collections::BTreeSet::from([
                "demo.item.flicker-scroll",
                "demo.item.farstep-scroll",
                "demo.item.seeking-scroll",
                "demo.item.trapfinding-scroll",
                "demo.item.temperate-tonic",
            ]),
        ),
    ];
    for (shop_id, expected_kinds) in expected {
        assert_eq!(
            left.shop_states[shop_id]
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
fn temple_purchase_and_alchemist_visit_use_independent_shop_state() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens game should start");
    game.gold = 1_000;
    game.player.position = Position { x: 20, y: 8 };
    game.mark_shop_visited_at_player();
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

    game.player.position = Position { x: 24, y: 8 };
    game.mark_shop_visited_at_player();
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

    let restored = Game::from_save(game.to_save()).expect("three shops should round-trip");
    assert_eq!(restored.shop_states, game.shop_states);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn quantity_purchase_is_atomic_zero_time_and_identified() {
    let mut game = store_game(42);
    game.gold = 100;
    let item_id = stock_item_id(&game, "demo.item.ration-of-food");
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
fn over_capacity_purchase_and_corpse_sale_are_rejected() {
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
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "over-capacity")
    }));

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
    assert!(game.item_property_knowledge[&repurchased.id].identified);
}

#[test]
fn compatible_shop_instances_project_and_trade_as_one_row() {
    let mut game = store_game(42);
    game.gold = 100;
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

    let torch = shop_before
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.wooden-torch")
        .expect("store should stock torches");
    assert!(torch.maximum_quantity >= 2);
    let torch_item_id = torch.id.clone();
    let shop_torches_before = game.shop_states[GENERAL_STORE_ID]
        .inventory
        .iter()
        .filter(|item| item.kind_id == "demo.item.wooden-torch")
        .map(|item| item.quantity)
        .sum::<u32>();
    let carried_torches_before = game
        .items
        .iter()
        .filter(|item| {
            item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
        })
        .map(|item| item.quantity)
        .sum::<u32>();
    let purchase = dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: GENERAL_STORE_ID.to_owned(),
            item_id: torch_item_id,
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
            .filter(|item| item.kind_id == "demo.item.wooden-torch")
            .map(|item| item.quantity)
            .sum::<u32>(),
        shop_torches_before - 2
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_torches_before + 2
    );

    let shop_after_purchase = projected_shop(&game.snapshot().shops, GENERAL_STORE_ID).clone();
    let quote = shop_after_purchase
        .sell_quotes
        .iter()
        .find(|quote| quote.kind_id == "demo.item.wooden-torch" && quote.maximum_quantity >= 2)
        .expect("compatible carried torches should have one grouped quote");
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
                item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
            })
            .map(|item| item.quantity)
            .sum::<u32>(),
        carried_torches_before
    );
    assert_eq!(
        projected_shop(&game.snapshot().shops, GENERAL_STORE_ID)
            .stock
            .iter()
            .filter(|item| item.kind_id == "demo.item.wooden-torch")
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
    assert_eq!(state.last_maintenance_world_tick, 10_000);
    assert_eq!(
        state
            .inventory
            .iter()
            .map(|item| item.kind_id.as_str())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(game.rng_draw_counter() > draws_before);
}
