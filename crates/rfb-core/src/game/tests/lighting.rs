// SPDX-License-Identifier: MPL-2.0

use super::support::*;
use super::*;
use crate::game::{
    gold::starting_gold, hunger::starting_ration_quantity, lighting::starting_torch_supply,
};
use rfb_protocol::{ItemFuelDto, ItemFuelKindDto};

const TORCH_KIND_ID: &str = "demo.item.wooden-torch";
const LANTERN_KIND_ID: &str = "demo.item.brass-lantern";
const OIL_KIND_ID: &str = "demo.item.flask-of-oil";

fn set_inventory_light_equipped(game: &mut Game, item_id: &str) {
    game.items
        .iter_mut()
        .find(|item| item.id == item_id)
        .expect("test light should exist")
        .location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
}

fn set_fuel(game: &mut Game, item_id: &str, current: u16) {
    game.items
        .iter_mut()
        .find(|item| item.id == item_id)
        .and_then(|item| item.fuel.as_mut())
        .expect("test item should have fuel")
        .current = current;
}

fn fuel(game: &Game, item_id: &str) -> ItemFuelDto {
    game.items
        .iter()
        .find(|item| item.id == item_id)
        .and_then(|item| item.fuel)
        .expect("test item should have fuel")
}

#[test]
fn warrior_birth_rolls_three_to_seven_matching_torches_after_food() {
    for seed in 0..32 {
        let build = CharacterBuildIdentity {
            build_id: RFB_WARRIOR_BUILD_ID.to_owned(),
            race_id: String::new(),
            class_id: String::new(),
            personality_id: String::new(),
        };
        let mut expected_rng = RfbRng::seeded(seed);
        let _ = starting_gold(Some(&build), &mut expected_rng);
        let _ = starting_ration_quantity(Some(&build), &mut expected_rng);
        let expected = starting_torch_supply(Some(&build), &mut expected_rng)
            .expect("Warrior should receive birth torches");
        let shop_draws_before = expected_rng.draw_counter;

        let game = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let torches = game
            .items
            .iter()
            .filter(|item| {
                item.kind_id == TORCH_KIND_ID && item.location == ItemLocation::Inventory
            })
            .collect::<Vec<_>>();

        assert_eq!(torches.len(), usize::try_from(expected.quantity).unwrap());
        assert!((3..=7).contains(&torches.len()));
        assert!(torches.iter().all(|torch| {
            torch.quantity == 1
                && torch.fuel
                    == Some(ItemFuelDto {
                        kind: ItemFuelKindDto::Torch,
                        current: expected.fuel,
                        maximum: 5_000,
                        light_radius: 1,
                    })
        }));
        assert!((1_500..=3_500).contains(&expected.fuel));
        assert_eq!(expected.fuel % 500, 0);
        assert!(game.rng_draw_counter() > shop_draws_before);
    }
}

#[test]
fn fuel_items_start_with_original_capacity_weight_and_radius() {
    let mut game = Game::new(42);
    give_inventory_item(&mut game, "test.torch", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    give_inventory_item(&mut game, "test.oil", OIL_KIND_ID);

    assert_eq!(
        fuel(&game, "test.torch"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Torch,
            current: 4_000,
            maximum: 5_000,
            light_radius: 1,
        }
    );
    assert_eq!(
        fuel(&game, "test.lantern"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Lantern,
            current: 7_500,
            maximum: 15_000,
            light_radius: 2,
        }
    );
    assert_eq!(
        fuel(&game, "test.oil"),
        ItemFuelDto {
            kind: ItemFuelKindDto::Oil,
            current: 7_500,
            maximum: 7_500,
            light_radius: 0,
        }
    );
    assert_eq!(game.item_weight_tenths_pound(TORCH_KIND_ID), 30);
    assert_eq!(game.item_weight_tenths_pound(LANTERN_KIND_ID), 50);
    assert_eq!(game.item_weight_tenths_pound(OIL_KIND_ID), 10);
}

#[test]
fn torch_refuel_consumes_a_torch_adds_source_fuel_plus_five_and_costs_fifty_energy() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.target", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.source", TORCH_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.target");
    set_fuel(&mut game, "test.target", 1_000);
    set_fuel(&mut game, "test.source", 2_000);
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::RefuelLight {
            target_item_id: "test.target".to_owned(),
            source_item_id: "test.source".to_owned(),
        },
    );

    assert_eq!(
        GameAction::RefuelLight {
            target_item_id: String::new(),
            source_item_id: String::new(),
        }
        .energy_cost(),
        50
    );
    assert_eq!(game.world_tick - tick_before, 5);
    assert_eq!(fuel(&game, "test.target").current, 3_005);
    assert!(game.items.iter().all(|item| item.id != "test.source"));
    assert_eq!(game.rng_draw_counter(), draws_before);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "light.refueled")
        .expect("refueling should report the applied amount");
    assert_eq!(event.args["amount"], "2005");
    assert_eq!(event.args["current"], "3005");
    assert_eq!(event.args["maximum"], "5000");
}

#[test]
fn lantern_accepts_oil_or_another_lantern_and_caps_after_consuming_the_source() {
    let mut oil_game = Game::new(42);
    give_inventory_item(&mut oil_game, "test.target", LANTERN_KIND_ID);
    give_inventory_item(&mut oil_game, "test.oil", OIL_KIND_ID);
    set_inventory_light_equipped(&mut oil_game, "test.target");
    set_fuel(&mut oil_game, "test.target", 1_000);
    let oil = oil_game
        .refuel_equipped_light("test.target", "test.oil")
        .expect("oil should refuel a lantern");
    assert_eq!(
        (oil.amount, oil.current, oil.maximum),
        (7_500, 8_500, 15_000)
    );
    assert!(oil_game.items.iter().all(|item| item.id != "test.oil"));

    give_inventory_item(&mut oil_game, "test.source-lantern", LANTERN_KIND_ID);
    set_fuel(&mut oil_game, "test.target", 14_900);
    set_fuel(&mut oil_game, "test.source-lantern", 1_000);
    let lantern = oil_game
        .refuel_equipped_light("test.target", "test.source-lantern")
        .expect("another lantern should refuel a lantern");
    assert_eq!((lantern.amount, lantern.current), (100, 15_000));
    assert!(
        oil_game
            .items
            .iter()
            .all(|item| item.id != "test.source-lantern")
    );
}

#[test]
fn unavailable_refuel_is_zero_world_time_rng_and_item_mutation() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.target", TORCH_KIND_ID);
    give_inventory_item(&mut game, "test.oil", OIL_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.target");
    set_fuel(&mut game, "test.target", 1_000);
    let items_before = game.items.clone();
    let tick_before = game.world_tick;
    let energy_before = game.player.energy_need;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::RefuelLight {
            target_item_id: "test.target".to_owned(),
            source_item_id: "test.oil".to_owned(),
        },
    );

    assert_eq!(game.items, items_before);
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    let unavailable = update
        .events
        .iter()
        .find(|event| event.kind == "light.refuel-unavailable")
        .expect("incompatible fuel should be reported");
    assert_eq!(unavailable.args["reason"], "source-incompatible");
}

#[test]
fn equipped_light_spends_one_fuel_per_ten_ticks_and_reports_extinction() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.torch", TORCH_KIND_ID);
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.torch".to_owned(),
            slot_id: None,
        },
    );
    set_fuel(&mut game, "test.torch", 1);
    let tick_before = game.world_tick;

    let update = dispatch_next(&mut game, GameCommand::Wait);

    assert_eq!(game.world_tick - tick_before, 10);
    assert_eq!(fuel(&game, "test.torch").current, 0);
    assert!(update.events.iter().any(|event| {
        event.kind == "light.extinguished" && event.args["targetItem"] == "test.torch"
    }));
    let restored = Game::from_save(game.to_save()).expect("spent light fuel should reload");
    assert_eq!(fuel(&restored, "test.torch").current, 0);
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn surface_is_ambient_lit_and_dungeon_visibility_follows_equipped_light_radius() {
    let mut game = Game::new_warrens_journey_with_build(42, RFB_WARRIOR_BUILD_ID)
        .expect("Warrens Warrior should create");
    let surface_neighbor = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, surface_neighbor, "demo.terrain.surface-grass");
    let surface = visual_at(&game.snapshot(), surface_neighbor);
    assert_eq!(surface.visibility, VisibilityState::Visible);
    assert_eq!(surface.light.intensity, SURFACE_AMBIENT_LIGHT);

    descend_one_floor(&mut game);
    game.entities.clear();
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=13 {
        replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
    }
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Equipped { .. }) {
            item.location = ItemLocation::Inventory;
        }
    }
    let adjacent = Position { x: 11, y: 10 };
    let distance_two = Position { x: 12, y: 10 };
    let distance_three = Position { x: 13, y: 10 };
    assert_eq!(
        visual_at(&game.snapshot(), adjacent).visibility,
        VisibilityState::Hidden
    );

    let torch_id = game
        .items
        .iter()
        .find(|item| item.kind_id == TORCH_KIND_ID)
        .expect("Warrior should carry torches")
        .id
        .clone();
    set_inventory_light_equipped(&mut game, &torch_id);
    assert_eq!(
        visual_at(&game.snapshot(), adjacent).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&game.snapshot(), distance_two).visibility,
        VisibilityState::Hidden
    );

    game.items
        .iter_mut()
        .find(|item| item.id == torch_id)
        .expect("torch should remain available")
        .location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.lantern", LANTERN_KIND_ID);
    set_inventory_light_equipped(&mut game, "test.lantern");
    assert_eq!(
        visual_at(&game.snapshot(), distance_two).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&game.snapshot(), distance_three).visibility,
        VisibilityState::Hidden
    );
}

#[test]
fn sleep_suppresses_carried_actor_light_but_not_intrinsic_light() {
    let prepare = |intrinsic| {
        let mut game = game_with_actor_definition(7, "demo.actor.ember-mote", |actor| {
            actor.light = Some(rfb_content::ActorLightDefinition {
                radius: 5,
                intrinsic,
            });
        });
        game.current_floor_id = "demo.floor.echo-depth-1".to_owned();
        game.entities.truncate(1);
        game.items.clear();
        game.entities[0].position = Position { x: 5, y: 3 };
        game.entities[0].statuses.push(StatusInstance {
            kind_id: STATUS_SLEEP.to_owned(),
            intensity: 1,
            remaining_ticks: 10,
            source_id: None,
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        game
    };

    assert!(!prepare(false).position_is_lit(Position { x: 4, y: 3 }));
    assert!(prepare(true).position_is_lit(Position { x: 4, y: 3 }));
}

#[test]
fn warrens_light_attempts_are_seeded_walkable_weighted_and_persistent() {
    let mut saw_miss = false;
    let mut saw_oil = false;
    let mut saw_lantern = false;
    for seed in 1..=64 {
        let mut left = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let mut right = Game::new_warrens_journey_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("matching Warrens Warrior should create");
        descend_one_floor(&mut left);
        descend_one_floor(&mut right);
        assert_eq!(left.items, right.items);
        let supplies = left
            .items
            .iter()
            .filter(|item| {
                matches!(item.location, ItemLocation::Ground(_))
                    && matches!(item.kind_id.as_str(), OIL_KIND_ID | LANTERN_KIND_ID)
            })
            .collect::<Vec<_>>();
        assert!(supplies.len() <= 1);
        if let Some(supply) = supplies.first() {
            let ItemLocation::Ground(position) = supply.location else {
                unreachable!()
            };
            assert!(left.is_walkable(position));
            saw_oil |= supply.kind_id == OIL_KIND_ID;
            saw_lantern |= supply.kind_id == LANTERN_KIND_ID;
        } else {
            saw_miss = true;
        }
        let restored = Game::from_save(left.to_save()).expect("generated light should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
    }
    assert!(saw_miss && saw_oil && saw_lantern);
}
