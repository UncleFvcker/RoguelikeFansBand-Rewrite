// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
#[ignore = "explicit pet preparation for ordinary Tauri standalone acceptance"]
fn export_pet_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("PET_DESKTOP_INPUT").unwrap());
    let directory = input.parent().unwrap();
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 99, y: 33 };
    for y in 28..=38 {
        for x in 94..=108 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    for (id, kind, position) in [
        ("pet.mount", "demo.actor.horse", game.player.position),
        (
            "pet.caster",
            "demo.actor.archlich",
            Position { x: 101, y: 33 },
        ),
        (
            "pet.target",
            "demo.actor.adobe-golem",
            Position { x: 105, y: 33 },
        ),
    ] {
        game.push_generated_actor(id.into(), kind, position);
        let actor = game.entities.last_mut().unwrap();
        actor.statuses.clear();
        actor.nice = false;
        actor.energy_need = INITIAL_MONSTER_ENERGY_NEED;
        if id != "pet.target" {
            actor.controller_id = Some(game.player.id.clone());
        }
    }
    game.entities.sort_by(|left, right| left.id.cmp(&right.id));
    game.riding_actor_id = Some("pet.mount".into());
    game.summon_command.pickup_items = true;
    game.summon_command.teleport = false;
    game.summon_command.no_breeding = true;
    give_inventory_item(&mut game, "pet.cargo", "demo.item.short-sword");
    game.items.last_mut().unwrap().location = ItemLocation::CarriedBy {
        actor_id: "pet.caster".into(),
    };
    game.glow.fill(true);
    game.reveal_current_visibility();
    let commands = vec![
        GameCommand::SetPetName {
            actor_id: "pet.mount".into(),
            name: Some("追风".into()),
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::HighlightMap,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::OpenDoors,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::PickupItems,
            enabled: false,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::NoBreeding,
            enabled: false,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::NoBreeding,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::AttackSpells,
            enabled: false,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::AttackSpells,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::SummonSpells,
            enabled: false,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::SummonSpells,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::RidingTwoHands,
            enabled: true,
        },
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::RidingTwoHands,
            enabled: false,
        },
        GameCommand::SetPetTarget {
            actor_id: Some("pet.target".into()),
        },
        GameCommand::Wait,
        GameCommand::EnterWorldMap {
            cancel_recall: false,
        },
        GameCommand::Move {
            direction: Direction::East,
        },
        GameCommand::LeaveWorldMap,
        GameCommand::DismissPet {
            actor_id: "pet.mount".into(),
        },
    ];
    let (initial, steps) = (0..256).find_map(|seed| {
        let mut trial = game.clone();
        trial.rng = RfbRng::seeded(seed);
        let initial = trial.clone();
        let mut steps = Vec::new();
        for command in &commands {
            let update = dispatch_next(&mut trial, command.clone());
            if matches!(command, GameCommand::Wait) && !trial.entities.iter().any(|actor| {
                actor.summon.as_ref().is_some_and(|summon| summon.owner_id == "pet.caster")
            }) { return None; }
            if matches!(command, GameCommand::EnterWorldMap { .. } | GameCommand::Move { .. }) {
                if trial.map_scale != MapScaleDto::World { return None; }
                assert_eq!(trial.riding_actor_id.as_deref(), Some("pet.mount"));
                assert!(trial.entities.iter().any(|actor| actor.id == "pet.caster"));
            }
            if matches!(command, GameCommand::LeaveWorldMap) {
                assert_eq!(trial.map_scale, MapScaleDto::Local);
                assert_ne!(trial.wilderness_position, initial.wilderness_position);
                assert_eq!(trial.riding_actor_id.as_deref(), Some("pet.mount"));
                assert!(trial.entities.iter().any(|actor| actor.id == "pet.caster"));
                assert!(trial.entities.iter().any(|actor| actor.summon.as_ref().is_some_and(|summon| summon.owner_id == "pet.caster")));
            }
            assert_eq!(trial.state_hash(), Game::from_save(trial.to_save(), trial.behavior_preferences()).unwrap().state_hash());
            steps.push(serde_json::json!({"command": command, "hash": trial.state_hash(), "events": update.events}));
        }
        Some((initial, steps))
    }).expect("a production archlich summon must be reachable");
    std::fs::write(
        directory.join("prepared.rfbsave"),
        rfb_save::encode(&header, &initial.to_save()).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("scenario.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "initialHash": initial.state_hash(), "steps": steps,
        "preparation": "Fresh human Warrior, chosen talent, cleared and lit local patch, mounted controlled horse, controlled awake archlich, hostile golem, carried sword. Selected RNG seed makes the first Wait summon through production AI. No natural acquisition or leveling claim."
    })).unwrap()).unwrap();
}

#[test]
fn pet_names_are_per_entity_free_and_validated_before_mutation() {
    let mut game = arena();
    pet(&mut game, "first", Position { x: 71, y: 30 });
    pet(&mut game, "second", Position { x: 72, y: 30 });
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    for (id, name) in [("first", "追风"), ("second", "踏雪🐎")] {
        let update = dispatch_next(
            &mut game,
            GameCommand::SetPetName {
                actor_id: id.into(),
                name: Some(name.into()),
            },
        );
        assert_eq!(
            update.events[0].args.get("name").map(String::as_str),
            Some(name)
        );
    }
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    assert_eq!(
        game.snapshot()
            .player
            .pets
            .iter()
            .map(|pet| pet.custom_name.as_deref())
            .collect::<Vec<_>>(),
        [Some("追风"), Some("踏雪🐎")]
    );
    for invalid in [
        " 名",
        "名 ",
        "\n",
        "名\u{202e}称",
        "名\u{2028}称",
        "12345678901234567",
    ] {
        let update = dispatch_next(
            &mut game,
            GameCommand::SetPetName {
                actor_id: "first".into(),
                name: Some(invalid.into()),
            },
        );
        assert_eq!(update.events[0].kind, "pet.name-invalid");
        assert_eq!(game.entities[0].custom_name.as_deref(), Some("追风"));
    }
    let name = "马".repeat(16);
    dispatch_next(
        &mut game,
        GameCommand::SetPetName {
            actor_id: "first".into(),
            name: Some(name),
        },
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let mut invalid = game.to_save();
    invalid.entities[0].custom_name = Some("bad\nname".into());
    assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
    for name in [Some(String::new()), None] {
        dispatch_next(
            &mut game,
            GameCommand::SetPetName {
                actor_id: "first".into(),
                name,
            },
        );
        assert!(game.entities[0].custom_name.is_none());
    }
    let update = dispatch_next(
        &mut game,
        GameCommand::DismissPet {
            actor_id: "second".into(),
        },
    );
    assert_eq!(
        update.events[0].args.get("name").map(String::as_str),
        Some("踏雪🐎")
    );
}

#[test]
fn pet_name_and_highlight_projection_respect_visibility_and_unique_restrictions() {
    let mut game = arena();
    pet(&mut game, "pet", Position { x: 71, y: 30 });
    game.entities[0].custom_name = Some("追风".into());
    let projected = game.entities_dto();
    assert!(!projected[0].highlight_map);
    assert!(projected[0].highlight_list);
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::HighlightMap,
            enabled: true,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::HighlightLists,
            enabled: false,
        },
    );
    let projected = game.entities_dto();
    assert!(projected[0].highlight_map);
    assert!(!projected[0].highlight_list);
    assert!(!game.snapshot().player.pets[0].highlight);
    game.entities[0].position = Position { x: 105, y: 50 };
    assert!(game.entities_dto().is_empty());
    assert!(game.snapshot().player.pets.is_empty());
    let update = dispatch_next(
        &mut game,
        GameCommand::SetPetName {
            actor_id: "pet".into(),
            name: Some("不应改名".into()),
        },
    );
    assert_eq!(update.events[0].kind, "pet.command-unavailable");
    assert_eq!(game.entities[0].custom_name.as_deref(), Some("追风"));
    game.push_generated_actor(
        "unique".into(),
        "demo.actor.smeagol",
        Position { x: 71, y: 30 },
    );
    game.entities[1].controller_id = Some(game.player.id.clone());
    game.entities[1].visible_invisible = true;
    assert!(!game.pet_can_be_named(&game.entities[1]));
    let update = dispatch_next(
        &mut game,
        GameCommand::SetPetName {
            actor_id: "unique".into(),
            name: Some("不应改名".into()),
        },
    );
    assert_eq!(update.events[0].kind, "pet.command-unavailable");
    game.entities[1].controller_id = None;
    assert!(!game.pet_can_be_named(&game.entities[1]));
    game.push_generated_actor(
        "invisible".into(),
        "demo.actor.clear-icky-thing",
        Position { x: 72, y: 30 },
    );
    game.entities[2].controller_id = Some(game.player.id.clone());
    game.entities[2].custom_name = Some("隐形宠物".into());
    assert!(
        !game
            .entities_dto()
            .iter()
            .any(|actor| actor.id == "invisible")
    );
    assert!(
        !game
            .snapshot()
            .player
            .pets
            .iter()
            .any(|actor| actor.actor_id == "invisible")
    );
}

#[test]
fn named_pet_keeps_its_name_across_floors_without_naming_new_spawns() {
    let mut game = arena();
    pet(&mut game, "follows", Position { x: 71, y: 30 });
    pet(&mut game, "stays", Position { x: 80, y: 34 });
    game.entities[0].custom_name = Some("追风".into());
    game.entities[1].custom_name = Some("踏雪".into());
    game.summon_command.highlight_map = true;
    game.summon_command.highlight_lists = false;
    let old_floor = game.current_floor_id.clone();
    game.transition_floor("demo.floor.warrens-depth-1".into(), None, None, false)
        .unwrap()
        .unwrap();
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .entities
            .iter()
            .find(|actor| actor.id == "follows")
            .unwrap()
            .custom_name
            .as_deref(),
        Some("追风")
    );
    assert_eq!(
        stored_floor(&restored, &old_floor)
            .entities
            .iter()
            .find(|actor| actor.id == "stays")
            .unwrap()
            .custom_name
            .as_deref(),
        Some("踏雪")
    );
    assert!(restored.summon_command.highlight_map);
    assert!(!restored.summon_command.highlight_lists);
    let position = game.player.position;
    pet(&mut game, "newborn", position);
    assert!(game.entities.last().unwrap().custom_name.is_none());
}

fn arena() -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 70, y: 30 };
    for y in 5..=55 {
        for x in 40..=110 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(true);
    game.reveal_current_visibility();
    game
}

fn pet(game: &mut Game, id: &str, position: Position) {
    game.push_generated_actor(id.into(), "demo.actor.horse", position);
    game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
}

fn step(game: &mut Game) {
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == "pet")
        .unwrap();
    game.resolve_player_summon_action(
        index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
}

#[test]
fn menu_commands_are_free_and_distance_modes_control_actual_steps() {
    for (mode, start, end) in [
        (SummonCommandModeDto::StayClose, 2, 1),
        (SummonCommandModeDto::Follow, 6, 6),
        (SummonCommandModeDto::Follow, 7, 6),
        (SummonCommandModeDto::Attack, 10, 10),
        (SummonCommandModeDto::Attack, 11, 10),
        (SummonCommandModeDto::GiveSpace, 10, 11),
        (SummonCommandModeDto::KeepDistance, 25, 26),
        (SummonCommandModeDto::Guard, 2, 1),
    ] {
        let mut game = arena();
        pet(
            &mut game,
            "pet",
            Position {
                x: 70 + start,
                y: 30,
            },
        );
        let before = (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
        );
        dispatch_next(&mut game, GameCommand::SetSummonCommand { mode });
        assert_eq!(
            (
                game.turn,
                game.world_tick,
                game.player.energy_need,
                game.rng.clone()
            ),
            before
        );
        step(&mut game);
        assert_eq!(
            chebyshev_distance(game.entities[0].position, game.player.position),
            end,
            "{mode:?}"
        );
    }
}

#[test]
fn selected_target_overrides_nearer_enemy_and_survives_save_but_invalid_targets_do_not() {
    let mut game = arena();
    pet(&mut game, "pet", Position { x: 71, y: 30 });
    game.push_generated_actor(
        "near".into(),
        "demo.actor.adobe-golem",
        Position { x: 72, y: 30 },
    );
    game.push_generated_actor(
        "chosen".into(),
        "demo.actor.adobe-golem",
        Position { x: 71, y: 28 },
    );
    game.reveal_current_visibility();
    dispatch_next(
        &mut game,
        GameCommand::SetPetTarget {
            actor_id: Some("chosen".into()),
        },
    );
    assert_eq!(
        game.summon_command.target_actor_id.as_deref(),
        Some("chosen")
    );
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    step(&mut game);
    step(&mut restored);
    assert_eq!(game.entities[0].position, Position { x: 71, y: 29 });
    assert_eq!(game.state_hash(), restored.state_hash());
    for id in ["pet", "missing"] {
        let update = dispatch_next(
            &mut game,
            GameCommand::SetPetTarget {
                actor_id: Some(id.into()),
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "pet.command-unavailable")
        );
        assert_eq!(
            game.summon_command.target_actor_id.as_deref(),
            Some("chosen")
        );
    }
    game.entities.retain(|actor| actor.id != "chosen");
    dispatch_next(
        &mut game,
        GameCommand::SetInterfaceLocale {
            locale: rfb_protocol::LocaleDto::EnUs,
        },
    );
    assert!(game.summon_command.target_actor_id.is_none());
    game.summon_command.target_actor_id = Some("missing".into());
    assert!(Game::from_save(game.to_save(), game.behavior_preferences()).is_err());
}

#[test]
fn follow_clears_manual_target_and_hidden_hostiles_cannot_be_selected() {
    let mut game = arena();
    game.push_generated_actor(
        "target".into(),
        "demo.actor.adobe-golem",
        Position { x: 72, y: 30 },
    );
    game.reveal_current_visibility();
    dispatch_next(
        &mut game,
        GameCommand::SetPetTarget {
            actor_id: Some("target".into()),
        },
    );
    for mode in [
        SummonCommandModeDto::Follow,
        SummonCommandModeDto::StayClose,
        SummonCommandModeDto::Guard,
    ] {
        game.summon_command.target_actor_id = Some("target".into());
        dispatch_next(&mut game, GameCommand::SetSummonCommand { mode });
        assert!(game.summon_command.target_actor_id.is_none());
    }
    game.entities[0].position = Position { x: 105, y: 50 };
    game.reveal_current_visibility();
    dispatch_next(
        &mut game,
        GameCommand::SetPetTarget {
            actor_id: Some("target".into()),
        },
    );
    assert!(game.summon_command.target_actor_id.is_none());
}

#[test]
fn selected_dismissal_is_free_clears_mount_and_leaves_other_pets_and_hostiles() {
    let mut game = arena();
    let position = game.player.position;
    pet(&mut game, "mount", position);
    pet(&mut game, "other", Position { x: 73, y: 30 });
    game.push_generated_actor(
        "hostile".into(),
        "demo.actor.adobe-golem",
        Position { x: 75, y: 30 },
    );
    game.riding_actor_id = Some("mount".into());
    game.summon_command.riding_two_hands = true;
    let before = (game.turn, game.world_tick, game.rng.clone());
    let update = dispatch_next(
        &mut game,
        GameCommand::DismissPet {
            actor_id: "mount".into(),
        },
    );
    assert!(game.riding_actor_id.is_none());
    assert!(!game.summon_command.riding_two_hands);
    assert_eq!(game.pet_upkeep().controlled_pets, 1);
    assert_eq!(update.removed_entities, vec!["mount"]);
    for id in ["hostile", "missing"] {
        dispatch_next(
            &mut game,
            GameCommand::DismissPet {
                actor_id: id.into(),
            },
        );
        assert_eq!(game.entities.len(), 2);
    }
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
}

#[test]
fn pet_door_permission_controls_path_and_execution_without_affecting_hostiles() {
    let mut game = game_with_actor_definition(510, "demo.actor.horse", |actor| {
        actor.door_interaction.opens = true;
        actor.door_interaction.bashes = true;
    });
    clear_monsters(&mut game);
    let position = game.player.position;
    let door = Position {
        x: position.x + 2,
        y: position.y,
    };
    pet(
        &mut game,
        "pet",
        Position {
            x: position.x + 1,
            y: position.y,
        },
    );
    replace_terrain(&mut game, door, "demo.terrain.door-closed");
    let rng = game.rng.clone();
    assert!(!game.actor_can_traverse_or_interact(0, door));
    assert_eq!(
        game.try_monster_door_interaction(0, door, &mut Vec::new(), &mut BTreeSet::new()),
        None
    );
    assert_eq!(game.rng, rng);
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::OpenDoors,
            enabled: true,
        },
    );
    assert!(game.actor_can_traverse_or_interact(0, door));
    assert_eq!(
        game.try_monster_door_interaction(0, door, &mut Vec::new(), &mut BTreeSet::new()),
        Some(false)
    );
    replace_terrain(&mut game, door, "demo.terrain.door-closed");
    game.summon_command.open_doors = false;
    game.entities[0].controller_id = None;
    assert!(game.actor_can_traverse_or_interact(0, door));
    assert_eq!(
        game.try_monster_door_interaction(0, door, &mut Vec::new(), &mut BTreeSet::new()),
        Some(false)
    );
}

#[test]
fn pickup_option_is_free_drops_items_once_and_preserves_identity_and_save() {
    let mut game = game_with_actor_definition(511, "demo.actor.horse", |actor| {
        actor.terrain_interaction.picks_up_items = true;
    });
    clear_monsters(&mut game);
    let position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    pet(&mut game, "pet", position);
    give_inventory_item(&mut game, "test.pet-loot", "demo.item.short-sword");
    let index = game
        .items
        .iter()
        .position(|item| item.id == "test.pet-loot")
        .unwrap();
    game.items[index].location = ItemLocation::Ground(position);
    let original = game.items[index].clone();
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    game.pick_up_items_under_monster(0, position, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(game.items[index], original);
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::PickupItems,
            enabled: true,
        },
    );
    game.pick_up_items_under_monster(0, position, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(
        game.items[index].location,
        ItemLocation::CarriedBy {
            actor_id: "pet".into()
        }
    );
    assert!(!game.item_is_discovered("test.pet-loot"));
    let restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    assert!(restored.summon_command.pickup_items);
    assert_eq!(game.state_hash(), restored.state_hash());
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::PickupItems,
            enabled: false,
        },
    );
    assert_eq!(game.items[index], original);
    let again = dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::PickupItems,
            enabled: false,
        },
    );
    assert!(
        !again
            .events
            .iter()
            .any(|event| event.kind == "loot.dropped")
    );
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
}

#[test]
fn pet_permission_does_not_grant_pickup_or_allow_item_destruction() {
    let mut game = game_with_actor_definition(512, "demo.actor.horse", |actor| {
        actor.door_interaction.opens = false;
        actor.door_interaction.bashes = false;
        actor.terrain_interaction.picks_up_items = false;
        actor.terrain_interaction.destroys_items = true;
    });
    clear_monsters(&mut game);
    let position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    pet(&mut game, "pet", position);
    give_inventory_item(&mut game, "test.pet-loot", "demo.item.short-sword");
    let index = game
        .items
        .iter()
        .position(|item| item.id == "test.pet-loot")
        .unwrap();
    game.items[index].location = ItemLocation::Ground(position);
    game.summon_command.pickup_items = true;
    game.pick_up_items_under_monster(0, position, &mut Vec::new(), &mut BTreeSet::new());
    game.destroy_items_under_monster(0, position, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(game.items[index].location, ItemLocation::Ground(position));
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::OpenDoors,
            enabled: true,
        },
    );
    let door = Position {
        x: position.x + 1,
        y: position.y,
    };
    replace_terrain(&mut game, door, "demo.terrain.door-closed");
    assert!(!game.actor_can_traverse_or_interact(0, door));
}

#[test]
fn disabling_pickup_uses_nearby_floor_or_existing_no_space_destruction() {
    for has_space in [true, false] {
        let mut game = arena();
        let origin = Position { x: 80, y: 30 };
        for y in 27..=33 {
            for x in 77..=83 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.dark-pit");
            }
        }
        let destination = Position { x: 81, y: 30 };
        if has_space {
            replace_terrain(&mut game, destination, "demo.terrain.floor");
        }
        game.push_generated_actor("pet".into(), "demo.actor.fruit-bat", origin);
        game.entities[0].controller_id = Some(game.player.id.clone());
        give_inventory_item(&mut game, "test.pet-loot", "demo.item.short-sword");
        game.items[0].location = ItemLocation::CarriedBy {
            actor_id: "pet".into(),
        };
        game.summon_command.pickup_items = true;
        dispatch_next(
            &mut game,
            GameCommand::SetPetOption {
                option: rfb_protocol::PetOptionDto::PickupItems,
                enabled: false,
            },
        );
        if has_space {
            assert_eq!(game.items[0].location, ItemLocation::Ground(destination));
        } else {
            assert!(game.items.is_empty());
            assert!(!game.item_property_knowledge.contains_key("test.pet-loot"));
        }
        assert!(Game::from_save(game.to_save(), game.behavior_preferences()).is_ok());
    }
}

fn breeding_game() -> Game {
    let mut game = arena();
    game.push_generated_actor(
        "parent".into(),
        "demo.actor.giant-white-mouse",
        Position { x: 75, y: 30 },
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::NoBreeding,
            enabled: false,
        },
    );
    let seed = (0..10_000)
        .find(|seed| {
            let mut rng = crate::rng::RfbRng::seeded(*seed);
            rng.bounded(375);
            rng.bounded(8) == 0
        })
        .unwrap();
    game.rng = crate::rng::RfbRng::seeded(seed);
    game
}

#[test]
fn breeding_permission_is_free_persisted_and_rejects_after_the_original_rng_gates() {
    let mut game = breeding_game();
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::NoBreeding,
            enabled: true,
        },
    );
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert!(restored.summon_command.no_breeding);
    assert_eq!(game.state_hash(), restored.state_hash());
    let mut expected = game.rng.clone();
    expected.bounded(375);
    expected.bounded(8);
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.rng, expected);
    assert_eq!(game.entities.len(), 1);

    // Player restrictions do not apply to a hostile parent.
    game.entities[0].controller_id = None;
    game.rng = before.3;
    assert!(game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert!(!game.actor_is_player_aligned(game.entities.last().unwrap()));
}

#[test]
fn breeding_accepts_eighty_percent_upkeep_and_rejects_more_without_placement_rng() {
    for upkeep in [80, 81] {
        let mut game = breeding_game();
        let divisor = game
            .character_definitions()
            .unwrap()
            .2
            .pet_upkeep_divisor
            .max(1);
        let free = u32::from(game.progress.level) * 80 / u32::from(divisor);
        let rng = game.rng.clone();
        // Level-one mice give exact upkeep without changing the content catalog.
        for ordinal in 1..upkeep + free {
            game.push_generated_actor(
                format!("distant.{ordinal}"),
                "demo.actor.giant-white-mouse",
                Position {
                    x: 40 + (ordinal % 10) as i32,
                    y: 5 + (ordinal / 10) as i32,
                },
            );
            game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
        }
        game.rng = rng;
        assert_eq!(u32::from(game.pet_upkeep().percent), upkeep);
        let before = game.pet_upkeep();
        let mut expected = game.rng.clone();
        expected.bounded(375);
        expected.bounded(8);
        let mut changed = BTreeSet::new();
        assert_eq!(
            game.try_original_reproduction(0, &mut changed),
            upkeep == 80
        );
        if upkeep == 80 {
            assert_eq!(
                game.pet_upkeep().controlled_pets,
                before.controlled_pets + 1
            );
            assert_eq!(game.pet_upkeep().percent, 81);
        } else {
            assert_eq!(game.rng, expected);
            assert!(changed.is_empty());
        }
    }
}

#[test]
fn offspring_control_and_save_continuation_are_stable_and_timed_pets_do_not_extend_lifetime() {
    let game = breeding_game();
    let mut original = game.clone();
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert!(original.try_original_reproduction(0, &mut BTreeSet::new()));
    assert!(restored.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(original.state_hash(), restored.state_hash());
    assert_eq!(original.pet_upkeep().controlled_pets, 2);
    assert_eq!(original.pet_upkeep().total_levels, 2);
    let child = original.entities.last().unwrap();
    assert_eq!(
        child.controller_id.as_deref(),
        Some(original.player.id.as_str())
    );
    assert!(child.summon.is_none());
    // Isolate inheritance from the separate summon-content validation contract.
    let mut timed = game;
    timed.entities[0].controller_id = None;
    timed.entities[0].summon = Some(SummonIdentity {
        owner_dependent: false,
        owner_id: timed.player.id.clone(),
        source_ability_id: "test.ability.summon".into(),
        remaining_turns: 2,
    });
    assert!(timed.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(timed.entities[1].summon, timed.entities[0].summon);
    assert!(timed.actor_is_player_aligned(&timed.entities[1]));
    assert!(timed.entities[1].controller_id.is_none());
    let mut dependent = breeding_game();
    dependent.entities[0].summon = Some(SummonIdentity {
        owner_dependent: true,
        owner_id: "test.original-caster".into(),
        source_ability_id: "test.ability.summon".into(),
        remaining_turns: 2,
    });
    assert!(dependent.try_original_reproduction(0, &mut BTreeSet::new()));
    let child = dependent.entities[1].summon.as_ref().unwrap();
    assert_eq!(child.owner_id, "parent");
    assert!(child.owner_dependent);
    assert_eq!(child.remaining_turns, 2);
}

#[test]
fn breeding_harmony_crowding_sterility_and_no_space_keep_their_own_rng_boundaries() {
    let mut game = breeding_game();
    let original = game.rng.clone();
    game.reproduction_suppressed = true;
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.rng, original);
    game.reproduction_suppressed = false;
    for (i, position) in [
        Position { x: 74, y: 30 },
        Position { x: 75, y: 29 },
        Position { x: 76, y: 30 },
    ]
    .into_iter()
    .enumerate()
    {
        pet(&mut game, &format!("neighbor.{i}"), position);
    }
    // Generating the neighbors also consumes their HP/energy RNG.
    game.rng = original.clone();
    let mut expected = original.clone();
    expected.bounded(375);
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.rng, expected);
    game.entities.truncate(1);
    game.rng = original.clone();
    for y in 29..=31 {
        for x in 74..=76 {
            if (x, y) != (75, 30) {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.wall");
            }
        }
    }
    expected = original;
    expected.bounded(375);
    expected.bounded(8);
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.rng, expected);
    let harmony_index = game
        .virtues
        .iter()
        .position(|v| v.kind == rfb_protocol::VirtueKindDto::Harmony)
        .unwrap_or(0);
    game.virtues[harmony_index].kind = rfb_protocol::VirtueKindDto::Harmony;
    game.virtues[harmony_index].value = 125;
    let seed = (0..10_000)
        .find(|seed| crate::rng::RfbRng::seeded(*seed).bounded(375) < 125)
        .unwrap();
    game.rng = crate::rng::RfbRng::seeded(seed);
    expected = game.rng.clone();
    expected.bounded(375);
    assert!(!game.try_original_reproduction(0, &mut BTreeSet::new()));
    assert_eq!(game.rng, expected);
}

#[test]
fn a_timed_pet_expiring_removes_its_cargo_with_the_owner() {
    let mut game = breeding_game();
    game.summon_command.no_breeding = true;
    game.entities[0].summon = Some(SummonIdentity {
        owner_dependent: false,
        owner_id: game.player.id.clone(),
        source_ability_id: "test.ability.summon".into(),
        remaining_turns: 1,
    });
    give_inventory_item(&mut game, "test.expiring-cargo", "demo.item.short-sword");
    game.items[0].location = ItemLocation::CarriedBy {
        actor_id: "parent".into(),
    };
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        update
            .removed_entities
            .iter()
            .filter(|id| id.as_str() == "parent")
            .count(),
        1
    );
    assert!(!game.entities.iter().any(|actor| actor.id == "parent"));
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.expiring-cargo")
    );
    assert!(
        !game
            .item_property_knowledge
            .contains_key("test.expiring-cargo")
    );
    assert!(Game::from_save(game.to_save(), game.behavior_preferences()).is_ok());
}
