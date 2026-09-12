// SPDX-License-Identifier: MPL-2.0
use super::usage::{body, item, sp};
use super::*;
use crate::game::tests::support::{dispatch_next, replace_terrain};
use crate::game::tests::town::enter_town_facility;
use rfb_protocol::{AbsorbedDeviceCategoryDto as Category, TravelOptionsDto};

fn profile(game: &mut Game, id: &str, profile_id: &str) {
    let definition = game
        .content
        .item(&item(game, id).kind_id)
        .unwrap()
        .device_generation
        .as_ref()
        .unwrap()
        .activations
        .iter()
        .find(|profile| profile.id == profile_id)
        .unwrap()
        .clone();
    let device = game.items.iter_mut().find(|item| item.id == id).unwrap();
    let activation = device.activation.as_mut().unwrap();
    activation.profile_id = definition.id;
    activation.name_key = definition.name_key;
    activation.cost = definition.charges.cost;
    activation.power = definition.min_depth;
    activation.device_check_difficulty = definition.device_check_difficulty;
    activation.target_spec = target_spec_dto(&definition.target);
    let maximum = definition.charges.minimum;
    device.charges = Some(ItemChargesDto {
        current: maximum,
        maximum,
    });
}

fn travel_game() -> Game {
    let mut game = at_level(1);
    game.terrain.fill("demo.terrain.floor".into());
    game.explored.fill(true);
    game.glow.fill(true);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.player.position = Position {
        x: i32::from(game.width) / 2,
        y: i32::from(game.height) / 2,
    };
    // Manual detection establishes coverage even when it finds no traps.
    assert!(
        game.detect_terrain_positions("trap", 1, true, true)
            .is_empty()
    );
    assert!(
        game.detection_coverage
            .traps
            .contains(&game.player.position)
    );
    assert!(!game.detection_coverage.traps.contains(&Position {
        x: game.player.position.x + 1,
        y: game.player.position.y
    }));
    game
}

#[test]
fn mogaminator_prefers_source_body_identify_and_keeps_strict_sp_and_slot_order() {
    let mut game = at_level(1);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.interface_locale = rfb_protocol::LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            rfb_protocol::LocaleDto::EnUs,
            "~?unidentified items".into()
        )
        .is_empty()
    );
    body(&mut game, "test.alias", "demo.item.identify-staff", 0);
    body(&mut game, "test.late", "demo.item.detect-objects-staff", 8);
    body(&mut game, "test.first", "demo.item.detect-objects-staff", 1);
    for id in ["test.late", "test.first"] {
        profile(&mut game, id, "rfb.device-activation.staff.identify");
    }
    give_inventory_item(&mut game, "test.pack", "demo.item.identify-staff");
    game.identify_item_instance("test.pack", ItemIdentificationRequest::new(true));
    let cost = item(&game, "test.first").activation.as_ref().unwrap().cost;
    sp(&mut game, "test.first", cost + 1);
    sp(&mut game, "test.late", cost);
    let alias_before = item(&game, "test.alias").charges;
    let pack_before = item(&game, "test.pack").charges.unwrap().current;
    // These prevent manual casting but do not gate source automatic identification.
    for status in [STATUS_CONFUSION, STATUS_FEAR] {
        game.player
            .statuses
            .push(monster_combat::melee_status(status, 100, "test.auto").status);
    }
    let rng = game.rng.clone();
    let clock = (game.turn, game.world_tick, game.player.energy_need);
    for target in ["test.first-target", "test.fallback-target"] {
        give_inventory_item(&mut game, target, "demo.item.dagger");
        let outcomes = game
            .apply_mogaminator_to_carried_items(vec![target.into()])
            .unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(game.item_property_knowledge[target].appraised);
        assert!(!game.item_property_knowledge[target].identified);
    }
    assert_eq!(item(&game, "test.first").charges.unwrap().current, 1);
    assert_eq!(item(&game, "test.late").charges.unwrap().current, cost);
    assert_eq!(
        item(&game, "test.alias").charges,
        alias_before,
        "Appraise alias is not source Identify"
    );
    assert_eq!(
        item(&game, "test.pack").charges.unwrap().current,
        pack_before - item(&game, "test.pack").activation.as_ref().unwrap().cost
    );
    assert_eq!(game.rng, rng);
    assert_eq!((game.turn, game.world_tick, game.player.energy_need), clock);
    assert!(game.resources.is_empty());
}

#[test]
fn local_travel_uses_slot_interleaving_and_stops_before_a_newly_revealed_trap() {
    let mut game = travel_game();
    body(&mut game, "test.staff", "demo.item.detect-objects-staff", 1);
    profile(
        &mut game,
        "test.staff",
        "rfb.device-activation.staff.detect-traps",
    );
    body(&mut game, "test.rod", "demo.item.resonance-rod", 0);
    let start = game.player.position;
    let next = Position {
        x: start.x + 1,
        y: start.y,
    };
    replace_terrain(&mut game, next, "demo.terrain.created-trap");
    game.revealed_terrain.remove(&next);
    let before_options = (game.turn, game.world_tick, game.rng.clone());
    let update = dispatch_next(
        &mut game,
        GameCommand::ConfigureTravel {
            options: TravelOptionsDto {
                auto_detect_traps: true,
                ..Default::default()
            },
        },
    );
    assert!(update.travel_options.auto_detect_traps);
    assert_eq!(
        (game.turn, game.world_tick, game.rng.clone()),
        before_options
    );
    let original = game.clone();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let rng = game.rng.clone();
    let before = (game.turn, game.world_tick);
    let rod_sp = item(&game, "test.rod").charges.unwrap().current;
    let staff_sp = item(&game, "test.staff").charges.unwrap().current;
    let rod_cost = item(&game, "test.rod").activation.as_ref().unwrap().cost;
    let command = GameCommand::TravelLocal { destination: next };
    assert_eq!(
        dispatch_next(&mut game, command.clone()),
        dispatch_next(&mut restored, command)
    );
    assert_eq!(game.player.position, start);
    assert!(game.revealed_terrain.contains(&next));
    assert_eq!(
        item(&game, "test.rod").charges.unwrap().current,
        rod_sp - rod_cost
    );
    assert_eq!(item(&game, "test.staff").charges.unwrap().current, staff_sp);
    assert_eq!(game.rng, rng);
    assert_eq!((game.turn, game.world_tick), before);
    assert_eq!(game.to_save(), restored.to_save());

    // At the same slot source visits staff before rod.
    let mut same_slot = original;
    same_slot
        .items
        .iter_mut()
        .find(|i| i.id == "test.staff")
        .unwrap()
        .location = ItemLocation::Absorbed {
        category: Category::Staff,
        slot: 0,
    };
    dispatch_next(
        &mut same_slot,
        GameCommand::TravelLocal { destination: next },
    );
    assert_eq!(
        item(&same_slot, "test.rod").charges.unwrap().current,
        rod_sp
    );
    assert_eq!(
        item(&same_slot, "test.staff").charges.unwrap().current,
        staff_sp
            - item(&same_slot, "test.staff")
                .activation
                .as_ref()
                .unwrap()
                .cost
    );

    replace_terrain(&mut game, next, "demo.terrain.floor");
    game.revealed_terrain.remove(&next);
    let mut manual = game.clone();
    let update = dispatch_next(&mut game, GameCommand::TravelLocal { destination: next });
    dispatch_next(
        &mut manual,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(
        update
            .events
            .iter()
            .all(|event| !event.kind.contains("detected"))
    );
    assert_eq!(
        game.to_save(),
        manual.to_save(),
        "travel within coverage only spends the movement turn"
    );
}

#[test]
fn local_travel_switches_exact_cost_fallback_and_mapping_are_independent() {
    let mut game = travel_game();
    let start = game.player.position;
    let next = Position {
        x: start.x + 1,
        y: start.y,
    };
    body(&mut game, "test.rod", "demo.item.resonance-rod", 0);
    let cost = item(&game, "test.rod").activation.as_ref().unwrap().cost;
    sp(&mut game, "test.rod", cost);
    let blocked = dispatch_next(&mut game, GameCommand::TravelLocal { destination: next });
    assert!(
        blocked
            .events
            .iter()
            .any(|event| event.kind == "travel.left-detection-area")
    );
    assert_eq!(game.player.position, start);
    game.travel_options.auto_detect_traps = true;
    let blocked = dispatch_next(&mut game, GameCommand::TravelLocal { destination: next });
    assert!(
        blocked
            .events
            .iter()
            .any(|event| event.kind == "travel.left-detection-area")
    );
    give_inventory_item(&mut game, "test.pack", "demo.item.resonance-rod");
    game.identify_item_instance("test.pack", ItemIdentificationRequest::new(true));
    sp(&mut game, "test.pack", cost);
    replace_terrain(&mut game, next, "demo.terrain.created-trap");
    let before = game.rng.clone();
    dispatch_next(&mut game, GameCommand::TravelLocal { destination: next });
    assert_eq!(item(&game, "test.rod").charges.unwrap().current, cost);
    assert_eq!(item(&game, "test.pack").charges.unwrap().current, 0);
    assert_eq!(game.rng, before);

    let mut mapping = travel_game();
    mapping.detect_terrain_positions("map", 0, true, true);
    body(&mut mapping, "test.map", "demo.item.resonance-rod", 0);
    profile(
        &mut mapping,
        "test.map",
        "rfb.device-activation.rod.enlightenment",
    );
    mapping.travel_options = TravelOptionsDto {
        auto_detect_traps: false,
        auto_map_area: true,
        disturb_trap_detect: false,
    };
    let mut manual = mapping.clone();
    let before = item(&mapping, "test.map").charges.unwrap().current;
    dispatch_next(
        &mut manual,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(!manual.detection_coverage.mapping.contains(&next));
    let update = dispatch_next(&mut mapping, GameCommand::TravelLocal { destination: next });
    assert_eq!(mapping.player.position, next);
    assert!(mapping.detection_coverage.mapping.contains(&next));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind.contains("detected"))
    );
    assert!(item(&mapping, "test.map").charges.unwrap().current < before);
    assert!(item(&manual, "test.map").charges.unwrap().current >= before);
}

#[test]
fn inn_fills_body_sp_and_fraction_only_after_successful_payment() {
    let mut game = at_level(1);
    let facility = "demo.town-facility.morivant-thieves-guild";
    enter_town_facility(&mut game, facility);
    body(&mut game, "test.staff", "demo.item.detect-objects-staff", 0);
    sp(&mut game, "test.staff", 0);
    game.items
        .iter_mut()
        .find(|i| i.id == "test.staff")
        .unwrap()
        .device_recovery_progress = 750;
    game.gold = 0;
    let before = game.to_save();
    assert_eq!(game.stay_at_inn(facility), Err("insufficient-gold"));
    assert_eq!(game.to_save(), before);
    game.gold = 10000;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_POISON, 20, "test.inn").status);
    let before = game.to_save();
    assert_eq!(game.stay_at_inn(facility), Err("needs-healer"));
    assert_eq!(game.to_save(), before);
    game.player.statuses.clear();
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.stay_at_inn(facility), restored.stay_at_inn(facility));
    let staff = item(&game, "test.staff");
    assert_eq!(
        staff.charges.unwrap().current,
        staff.charges.unwrap().maximum
    );
    assert_eq!(staff.device_recovery_progress, 0);
    assert_eq!(game.to_save(), restored.to_save());
}

#[test]
fn automatic_detect_all_stops_for_an_unseen_hostile_without_a_world_turn() {
    let mut game = travel_game();
    body(&mut game, "test.all", "demo.item.resonance-rod", 0);
    profile(
        &mut game,
        "test.all",
        "rfb.device-activation.rod.detect-all",
    );
    game.travel_options.auto_detect_traps = true;
    let start = game.player.position;
    game.push_generated_actor(
        "test.unseen".into(),
        "demo.actor.sheep",
        Position {
            x: start.x + 22,
            y: start.y,
        },
    );
    assert!(!game.entity_is_visible_to_player(game.entities.last().unwrap()));
    let before = (game.turn, game.world_tick, game.rng.clone());
    let current = item(&game, "test.all").charges.unwrap().current;
    let cost = item(&game, "test.all").activation.as_ref().unwrap().cost;
    let update = dispatch_next(
        &mut game,
        GameCommand::TravelLocal {
            destination: Position {
                x: start.x + 1,
                y: start.y,
            },
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind.contains("detected"))
    );
    assert_eq!(game.player.position, start);
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
    assert_eq!(
        item(&game, "test.all").charges.unwrap().current,
        current - cost
    );
}

#[test]
fn saved_detection_coverage_rejects_duplicates_and_out_of_bounds_cells() {
    let game = travel_game();
    for positions in [
        vec![Position { x: -1, y: 5 }],
        vec![game.player.position, game.player.position],
        vec![Position {
            x: i32::from(game.width),
            y: 5,
        }],
    ] {
        let mut saved = game.to_save();
        saved.detection_coverage.traps = positions;
        assert!(matches!(
            Game::from_save(saved),
            Err(CoreError::InvalidSave("detection coverage is invalid"))
        ));
    }
    let mut saved = game.to_save();
    let floor = saved
        .stored_floors
        .first_mut()
        .expect("surface backing floor");
    floor
        .detection_coverage
        .mapping
        .push(Position { x: -1, y: 0 });
    assert!(matches!(
        Game::from_save(saved),
        Err(CoreError::InvalidSave("detection coverage is invalid"))
    ));
}

#[test]
fn impotence_penalty_reaches_body_use_and_failure_projection_after_loading() {
    use super::usage::{check_seed, use_body};
    let mut game = at_level(1);
    body(&mut game, "test.staff", "demo.item.detect-objects-staff", 0);
    let normal = game
        .absorbed_device_slot_dto(Category::Staff, 0)
        .failure_per_mille
        .unwrap();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.impotence".into());
    let impaired = game
        .absorbed_device_slot_dto(Category::Staff, 0)
        .failure_per_mille
        .unwrap();
    assert!(impaired > normal);
    game.rng = check_seed(&game, "test.staff", false).0;
    let before = item(&game, "test.staff").charges;
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let actual = use_body(&mut game, "test.staff", &[]);
    assert_eq!(actual, use_body(&mut restored, "test.staff", &[]));
    assert_eq!(actual.0, 100);
    assert!(actual.1.iter().any(|event| matches!(
        event,
        DomainEvent::DeviceSkillChecked {
            succeeded: false,
            ..
        }
    )));
    assert_eq!(item(&game, "test.staff").charges, before);
    assert_eq!(game.to_save(), restored.to_save());
}
