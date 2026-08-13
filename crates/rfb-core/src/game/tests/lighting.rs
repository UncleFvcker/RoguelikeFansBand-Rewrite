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

fn intrinsic_see_invisible_game(seed: u64) -> Game {
    Game::new_with_build_race_and_name(
        seed,
        RFB_WARRIOR_BUILD_ID,
        "rfb-legacy.race.high-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal High-Elf game should create")
}

fn race_form_status(race_id: &str) -> StatusInstance {
    StatusInstance {
        kind_id: STATUS_PLAYER_POLYMORPH.to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.race-form".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: Some(race_id.to_owned()),
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }
}

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
    let content = Game::new(0).content;
    for seed in 0..32 {
        let build = CharacterBuildIdentity {
            build_id: RFB_WARRIOR_BUILD_ID.to_owned(),
            race_id: "demo.race.rfb-human".to_owned(),
            class_id: "demo.class.warrior".to_owned(),
            personality_id: "demo.personality.ordinary".to_owned(),
        };
        let mut expected_rng = RfbRng::seeded(seed);
        let _ = crate::game::virtues::initial_virtues(&content, Some(&build), &mut expected_rng);
        let _ = starting_gold(Some(&build), &mut expected_rng);
        let _ = starting_ration_quantity(Some(&build), &mut expected_rng);
        let expected = starting_torch_supply(Some(&build), &mut expected_rng)
            .expect("Warrior should receive birth torches");
        let shop_draws_before = expected_rng.draw_counter;

        let game = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
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
    let mut game =
        Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
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
    game.glow.fill(false);
    game.player.position = Position { x: 10, y: 10 };
    for y in 8..=13 {
        for x in 8..=13 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.explored.fill(false);
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Equipped { .. }) {
            item.location = ItemLocation::Inventory;
        }
    }
    let adjacent = Position { x: 11, y: 10 };
    let adjacent_diagonal = Position { x: 11, y: 11 };
    let distance_two = Position { x: 12, y: 10 };
    let lantern_edge = Position { x: 12, y: 11 };
    let lantern_corner = Position { x: 12, y: 12 };
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
    let torch_diagonal = visual_at(&game.snapshot(), adjacent_diagonal);
    assert_eq!(torch_diagonal.visibility, VisibilityState::Visible);
    assert!(torch_diagonal.light.intensity > DUNGEON_AMBIENT_LIGHT);
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
    let lantern_outer_band = visual_at(&game.snapshot(), lantern_edge);
    assert_eq!(lantern_outer_band.visibility, VisibilityState::Visible);
    assert!(lantern_outer_band.light.intensity > DUNGEON_AMBIENT_LIGHT);
    assert_eq!(
        visual_at(&game.snapshot(), lantern_corner).visibility,
        VisibilityState::Hidden
    );
    assert_eq!(
        visual_at(&game.snapshot(), distance_three).visibility,
        VisibilityState::Hidden
    );
}

#[test]
fn infravision_does_not_reveal_cold_blooded_monsters() {
    let mut game =
        Game::new_with_build(43, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.glow.fill(false);
    game.player.position = Position { x: 10, y: 10 };
    for item in &mut game.items {
        if matches!(item.location, ItemLocation::Equipped { .. }) {
            item.location = ItemLocation::Inventory;
        }
    }
    assert!(game.gain_mutation("rfb.mutation.infravision", &mut Vec::new()));
    let warm_position = Position { x: 11, y: 10 };
    let cold_position = Position { x: 10, y: 11 };
    for position in [game.player.position, warm_position, cold_position] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor("test.warm".to_owned(), "demo.actor.newt", warm_position);
    game.push_generated_actor("test.cold".to_owned(), "demo.actor.ghast", cold_position);

    let snapshot = game.snapshot();
    assert!(
        snapshot
            .entities
            .iter()
            .any(|actor| actor.id == "test.warm")
    );
    assert!(
        snapshot
            .entities
            .iter()
            .all(|actor| actor.id != "test.cold")
    );
}

#[test]
fn invisible_actors_are_hidden_until_detected_and_detection_round_trips() {
    let mut game = Game::new(12);
    clear_monsters(&mut game);
    let player_position = Position { x: 3, y: 3 };
    game.player.position = player_position;
    let target = Position { x: 4, y: 3 };
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor(
        "test.invisible".to_owned(),
        "demo.actor.clear-icky-thing",
        target,
    );

    let hidden = game.snapshot();
    assert!(
        hidden
            .entities
            .iter()
            .all(|actor| actor.id != "test.invisible")
    );
    assert!(
        hidden
            .cells
            .iter()
            .any(|cell| { cell.position == target && cell.actor_id.is_none() })
    );

    game.entities[0].visible_invisible = true;
    let detected = game.snapshot();
    assert!(
        detected
            .entities
            .iter()
            .any(|actor| actor.id == "test.invisible")
    );
    assert!(detected.cells.iter().any(|cell| {
        cell.position == target && cell.actor_id.as_deref() == Some("test.invisible")
    }));

    let restored = Game::from_save(game.to_save()).expect("invisible detection should reload");
    assert!(restored.entities[0].visible_invisible);
    assert!(
        restored
            .snapshot()
            .entities
            .iter()
            .any(|actor| actor.id == "test.invisible")
    );
}

#[test]
fn intrinsic_race_see_invisible_stacks_and_follows_the_current_form() {
    let mut game = intrinsic_see_invisible_game(47);
    assert_eq!(game.player_see_invisible_sources(), 1);

    for item in &mut game.items {
        if matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "weapon") {
            item.location = ItemLocation::Inventory;
        }
    }
    give_inventory_item(&mut game, "test.crisdurian", "demo.item.crisdurian");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.crisdurian")
        .expect("test artifact should exist")
        .location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_SEE_INVISIBLE.to_owned(),
        granted_race_id: None,
        ..race_form_status("rfb-legacy.race.high-elf")
    });
    assert_eq!(game.player_see_invisible_sources(), 3);

    game.player
        .statuses
        .push(race_form_status("rfb-legacy.race.half-orc"));
    assert_eq!(game.player_see_invisible_sources(), 2);
    game.player
        .statuses
        .last_mut()
        .expect("race form should exist")
        .granted_race_id = Some("rfb-legacy.race.high-elf".to_owned());
    assert_eq!(game.player_see_invisible_sources(), 3);
}

#[test]
fn intrinsic_race_see_invisible_uses_the_original_detection_roll() {
    let mut game = intrinsic_see_invisible_game(48);
    clear_monsters(&mut game);
    game.glow.fill(true);
    let player_position = Position { x: 3, y: 3 };
    game.player.position = player_position;
    let target = Position { x: 4, y: 3 };
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor(
        "test.intrinsic-invisible".to_owned(),
        "demo.actor.clear-icky-thing",
        target,
    );

    let search_skill = game.player_derived_stats().search_skill.value.max(0) as u64;
    let actor_level = game
        .content
        .actor("demo.actor.clear-icky-thing")
        .expect("test actor should exist")
        .level;
    let difficulty = u64::from(50_u32.saturating_add(actor_level / 2));
    let success_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(difficulty) < search_skill
        })
        .expect("a bounded seed should pass the invisible-detection check");
    game.rng = RfbRng::seeded(success_seed);
    let mut expected_rng = game.rng.clone();
    assert!(expected_rng.bounded(difficulty) < search_skill);
    game.refresh_invisible_visibility(true, &BTreeMap::new());
    assert!(game.entities[0].visible_invisible);
    assert_eq!(game.rng, expected_rng);

    game.entities[0].visible_invisible = false;
    game.player
        .statuses
        .push(race_form_status("rfb-legacy.race.half-orc"));
    game.rng = RfbRng::seeded(9);
    let rng_before = game.rng.clone();
    game.refresh_invisible_visibility(true, &BTreeMap::new());
    assert!(!game.entities[0].visible_invisible);
    assert_eq!(game.rng, rng_before);
}

#[test]
fn room_glow_darkening_persists_in_stored_floor_save_and_state_hash() {
    let mut game =
        Game::new_with_build(42, RFB_WARRIOR_BUILD_ID).expect("Warrens Warrior should create");
    descend_one_floor(&mut game);
    let floor_id = game.current_floor_id.clone();
    assert!(game.glow.iter().any(|glow| *glow));
    let hash_before = game.state_hash();
    let darkened = game.darken_room(game.player.position);
    assert!(!darkened.is_empty());
    assert_ne!(game.state_hash(), hash_before);
    let glow_after = game.glow.clone();

    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    game.traverse_stairs(false)
        .expect("deeper descent should resolve")
        .expect("deeper descent should transition");
    assert_eq!(stored_floor(&game, &floor_id).glow, glow_after);

    let restored = Game::from_save(game.to_save()).expect("room glow should reload");
    assert_eq!(stored_floor(&restored, &floor_id).glow, glow_after);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn warrens_light_attempts_are_seeded_walkable_weighted_and_persistent() {
    let mut saw_miss = false;
    let mut saw_oil = false;
    let mut saw_lantern = false;
    for seed in 1..=64 {
        let mut left = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
            .expect("Warrens Warrior should create");
        let mut right = Game::new_with_build(seed, RFB_WARRIOR_BUILD_ID)
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
        if supplies.is_empty() {
            saw_miss = true;
        }
        for supply in supplies {
            let ItemLocation::Ground(position) = supply.location else {
                unreachable!()
            };
            assert!(left.is_walkable(position));
            saw_oil |= supply.kind_id == OIL_KIND_ID;
            saw_lantern |= supply.kind_id == LANTERN_KIND_ID;
        }
        let restored = Game::from_save(left.to_save()).expect("generated light should reload");
        assert_eq!(restored.state_hash(), left.state_hash());
    }
    assert!(saw_miss && saw_oil && saw_lantern);
}
