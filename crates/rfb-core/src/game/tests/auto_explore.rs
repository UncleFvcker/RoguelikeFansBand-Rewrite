// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_protocol::AutoExploreTargetDto;

fn at(x: i32, y: i32) -> Position {
    Position {
        x: 90 + x,
        y: 30 + y,
    }
}

fn arena() -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.gold_piles.clear();
    game.terrain.fill("demo.terrain.wall".into());
    game.explored.fill(false);
    game.player.position = at(1, 1);
    // A long corridor with a turn, plus an isolated unknown room.
    for x in 1..=24 {
        replace_terrain(&mut game, at(x, 1), "demo.terrain.floor");
    }
    for y in 1..=8 {
        replace_terrain(&mut game, at(24, y), "demo.terrain.floor");
    }
    replace_terrain(&mut game, at(5, 7), "demo.terrain.floor");
    game.mogaminator.auto_get_mode = AutoGetModeDto::Off;
    game.reveal_current_visibility();
    game
}

fn finish(game: &mut Game) -> usize {
    for steps in 0..150 {
        if game.auto_explore.is_none() {
            return steps;
        }
        dispatch_next(game, GameCommand::ContinueAutoExplore);
    }
    panic!(
        "exploration did not stop at {:?}: {:?}",
        game.player.position, game.auto_explore
    );
}

#[test]
fn discovers_connected_regions_and_finishes_without_claiming_isolated_room() {
    let mut game = arena();
    assert!(!game.explored[game.index(at(24, 8)).unwrap()]);
    dispatch_next(&mut game, GameCommand::AutoExplore);
    assert!(finish(&mut game) > 10);
    assert!(game.explored[game.index(at(24, 8)).unwrap()]);
    assert!(!game.explored[game.index(at(5, 7)).unwrap()]);
}

#[test]
fn opens_a_frontier_door_then_continues_into_the_unknown_corridor() {
    let mut game = arena();
    let origin = game.player.position;
    let door = at(2, 1);
    replace_terrain(&mut game, door, "demo.terrain.door-closed");
    game.explored.fill(false);
    game.reveal_current_visibility();
    let before = game.turn;
    let update = dispatch_next(&mut game, GameCommand::AutoExplore);
    assert_eq!(game.player.position, origin);
    assert_eq!(game.turn, before + 1);
    assert_eq!(game.terrain_at(door), "demo.terrain.door-open");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-opened")
    );
    assert!(game.auto_explore.is_some());
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, GameCommand::ContinueAutoExplore),
        dispatch_next(&mut restored, GameCommand::ContinueAutoExplore),
    );
    finish(&mut game);
    assert!(game.explored[game.index(at(24, 8)).unwrap()]);
}

#[test]
fn closed_doors_obey_easy_open_and_jammed_and_secret_doors_block_exploration() {
    for (terrain, easy_open) in [
        ("demo.terrain.door-closed", false),
        ("demo.terrain.door-jammed-1", true),
        ("demo.terrain.door-secret", true),
    ] {
        let mut game = arena();
        let door = at(2, 1);
        replace_terrain(&mut game, door, terrain);
        game.operation_options.easy_open = easy_open;
        game.explored.fill(false);
        game.reveal_current_visibility();
        dispatch_next(&mut game, GameCommand::AutoExplore);
        finish(&mut game);
        assert_eq!(game.player.position, at(1, 1), "{terrain}");
        assert_eq!(game.terrain_at(door), terrain);
        assert!(!game.explored[game.index(at(24, 8)).unwrap()]);
    }
}

#[test]
fn ordinary_travel_uses_the_same_door_action_without_teleporting_through_it() {
    let mut game = arena();
    let door = at(2, 1);
    replace_terrain(&mut game, door, "demo.terrain.door-closed");
    game.reveal_current_visibility();
    let update = dispatch_next(&mut game, GameCommand::TravelLocal { destination: door });
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-opened")
    );
    assert_eq!(game.player.position, at(1, 1));
    dispatch_next(&mut game, GameCommand::TravelLocal { destination: door });
    assert_eq!(game.player.position, door);
}

#[test]
fn a_failed_lock_attempt_stops_without_queuing_more_turns() {
    let mut game = arena();
    let door = at(2, 1);
    replace_terrain(&mut game, door, "demo.terrain.door-secret");
    game.revealed_terrain.insert(door);
    game.explored.fill(false);
    game.reveal_current_visibility();
    let seed = (0..100)
        .find(|seed| {
            let mut probe = game.clone();
            probe.rng = crate::rng::RfbRng::seeded(*seed);
            matches!(
                probe.open_door(Direction::East),
                Some(DoorOpenOutcome::UnlockFailed { .. })
            )
        })
        .expect("locked door can fail its check");
    game.rng = crate::rng::RfbRng::seeded(seed);
    let before = game.turn;
    let update = dispatch_next(&mut game, GameCommand::AutoExplore);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-unlock-failed")
    );
    assert_eq!(game.turn, before + 1);
    assert!(game.auto_explore.is_none());
    let stopped = (
        game.turn,
        game.world_tick,
        game.rng.clone(),
        game.player.position,
    );
    dispatch_next(&mut game, GameCommand::ContinueAutoExplore);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.rng.clone(),
            game.player.position
        ),
        stopped
    );
}

#[test]
fn rule_pickup_handles_start_and_newly_seen_items_without_collecting_rejected_items() {
    let mut game = arena();
    game.apply_behavior_preferences(Game::default_behavior_preferences())
        .unwrap();
    // Even the manual pick-up-all preference must not bypass the selected explore rules.
    game.travel_options.always_pickup = true;
    game.mogaminator.zh_cn_source = "食物".into();
    for (id, kind, position) in [
        ("test.start.food", "demo.item.ration-of-food", at(1, 1)),
        ("test.start.weapon", "demo.item.dagger", at(1, 1)),
        ("test.later.food", "demo.item.ration-of-food", at(20, 1)),
        ("test.later.weapon", "demo.item.dagger", at(20, 1)),
    ] {
        give_inventory_item(&mut game, id, kind);
        game.items.last_mut().unwrap().location = ItemLocation::Ground(position);
    }
    game.reveal_current_visibility();
    dispatch_next(&mut game, GameCommand::AutoExplore);
    assert_eq!(game.player.position, at(1, 1));
    assert!(
        game.items
            .iter()
            .any(|item| item.kind_id == "demo.item.ration-of-food"
                && item.location == ItemLocation::Inventory)
    );
    finish(&mut game);
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.kind_id == "demo.item.ration-of-food"
                && matches!(item.location, ItemLocation::Ground(_)))
    );
    for (id, position) in [
        ("test.start.weapon", at(1, 1)),
        ("test.later.weapon", at(20, 1)),
    ] {
        assert!(
            game.items
                .iter()
                .any(|item| item.id == id && item.location == ItemLocation::Ground(position))
        );
    }
    assert!(game.explored[game.index(at(24, 8)).unwrap()]);
}

#[test]
fn completed_map_and_stale_continue_or_cancel_cost_nothing() {
    let mut game = arena();
    game.explored.fill(true);
    let before = (
        game.turn,
        game.world_tick,
        game.rng.clone(),
        game.player.position,
    );
    for command in [
        GameCommand::AutoExplore,
        GameCommand::CancelAutoExplore,
        GameCommand::ContinueAutoExplore,
    ] {
        dispatch_next(&mut game, command);
        assert!(game.auto_explore.is_none());
        assert_eq!(
            (
                game.turn,
                game.world_tick,
                game.rng.clone(),
                game.player.position
            ),
            before
        );
    }
}

#[test]
fn unreachable_frontier_and_boundaries_without_new_knowledge_terminate() {
    let mut game = arena();
    game.explored.fill(true);
    // Walls conceal these holes forever, leaving reachable but fruitless boundary candidates.
    for x in 2..=23 {
        let index = game.index(at(x, 2)).unwrap();
        game.explored[index] = false;
    }
    let isolated = game.index(at(5, 8)).unwrap();
    game.explored[isolated] = false;
    dispatch_next(&mut game, GameCommand::AutoExplore);
    finish(&mut game);
    assert_ne!(game.player.position, at(5, 7));
    assert!(!game.explored[isolated]);
}

#[test]
fn hidden_geometry_objects_and_actors_do_not_change_the_selected_target() {
    let mut first = arena();
    let mut second = arena();
    replace_terrain(&mut second, at(24, 8), "demo.terrain.door-closed");
    second.push_generated_actor(
        "test.hidden.monster".into(),
        "demo.actor.war-bear",
        at(24, 7),
    );
    give_inventory_item(&mut second, "test.hidden.item", "demo.item.ration-of-food");
    second.items.last_mut().unwrap().location = ItemLocation::Ground(at(24, 8));
    first.mogaminator.auto_get_mode = AutoGetModeDto::Wanted;
    second.mogaminator.auto_get_mode = AutoGetModeDto::Wanted;
    let mut events = vec![];
    assert_eq!(
        first.prepare_auto_explore(&GameAction::AutoExplore, &mut events),
        second.prepare_auto_explore(&GameAction::AutoExplore, &mut events)
    );
    assert_eq!(first.auto_explore, second.auto_explore);
}

#[test]
fn wanted_gold_precedes_frontiers_while_off_ignores_it() {
    let mut game = arena();
    game.gold_piles.push(GoldPile {
        id: "test.explore.gold".into(),
        position: at(5, 1),
        amount: 10,
        appearance: rfb_protocol::GoldAppearanceDto::Gold,
        discovered: true,
    });
    game.reveal_current_visibility();
    game.mogaminator.auto_get_mode = AutoGetModeDto::Wanted;
    dispatch_next(&mut game, GameCommand::AutoExplore);
    assert!(matches!(game.auto_explore.as_ref().unwrap().target,
        Some(AutoExploreTargetDto::Object { ref object_id, .. }) if object_id == "test.explore.gold"));
    let gold = game.gold;
    finish(&mut game);
    assert!(game.gold_piles.is_empty());
    assert_eq!(game.gold, gold + 10);
    let mut off = arena();
    off.gold_piles.push(GoldPile {
        id: "test.explore.off".into(),
        position: at(1, 1),
        amount: 10,
        appearance: rfb_protocol::GoldAppearanceDto::Gold,
        discovered: true,
    });
    dispatch_next(&mut off, GameCommand::AutoExplore);
    assert_eq!(off.gold_piles.len(), 1);
}

#[test]
fn off_ignores_rule_items_and_gold_encountered_while_exploring() {
    let mut game = arena();
    game.mogaminator.enabled = true;
    game.mogaminator.zh_cn_source = "物品".into();
    game.travel_options.always_pickup = true;
    give_inventory_item(&mut game, "test.off.food", "demo.item.ration-of-food");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(at(2, 1));
    game.gold_piles.push(GoldPile {
        id: "test.off.gold".into(),
        position: at(2, 1),
        amount: 10,
        appearance: rfb_protocol::GoldAppearanceDto::Gold,
        discovered: true,
    });
    game.reveal_current_visibility();
    dispatch_next(&mut game, GameCommand::AutoExplore);
    finish(&mut game);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.off.food"
                && item.location == ItemLocation::Ground(at(2, 1)))
    );
    assert_eq!(game.gold_piles.len(), 1);
}

#[test]
fn saves_resume_deterministically_and_reject_invalid_state() {
    let mut game = arena();
    dispatch_next(&mut game, GameCommand::AutoExplore);
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    let state = restored.auto_explore.take();
    assert_ne!(game.state_hash(), restored.state_hash());
    restored.auto_explore = state;
    assert_eq!(
        dispatch_next(&mut game, GameCommand::ContinueAutoExplore),
        dispatch_next(&mut restored, GameCommand::ContinueAutoExplore)
    );
    let mut invalid = game.to_save();
    invalid.player.auto_explore.as_mut().unwrap().position.x += 1;
    assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
    let mut invalid = game.to_save();
    invalid
        .player
        .auto_explore
        .as_mut()
        .unwrap()
        .visited_frontiers = vec![at(1, 1); 2];
    assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
}

#[test]
fn danger_damage_other_commands_and_floor_changes_stop_exploration() {
    let mut game = arena();
    dispatch_next(&mut game, GameCommand::AutoExplore);
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.auto_explore.is_none());
    dispatch_next(&mut game, GameCommand::AutoExplore);
    game.apply_final_player_damage(
        resolve_damage(
            DamagePacket::new(1, DamageType::Physical),
            ResistanceLevel::Normal,
        ),
        FatalityPolicy::BelowZero,
    );
    assert!(game.auto_explore.is_none());
    dispatch_next(&mut game, GameCommand::AutoExplore);
    game.auto_explore.as_mut().unwrap().floor_id = "old.floor".into();
    let before = (game.turn, game.world_tick, game.player.position);
    dispatch_next(&mut game, GameCommand::ContinueAutoExplore);
    assert!(game.auto_explore.is_none());
    assert_eq!((game.turn, game.world_tick, game.player.position), before);
    game.push_generated_actor(
        "test.visible.monster".into(),
        "demo.actor.war-bear",
        at(5, 1),
    );
    dispatch_next(&mut game, GameCommand::AutoExplore);
    assert!(game.auto_explore.is_none());
    assert_eq!((game.turn, game.world_tick, game.player.position), before);
}

#[test]
fn detection_boundary_uses_travel_preflight_without_an_extra_step() {
    let mut game = arena();
    game.travel_options.disturb_trap_detect = true;
    game.travel_options.auto_detect_traps = false;
    game.detection_coverage.traps.insert(game.player.position);
    let before = (game.turn, game.world_tick, game.player.position);
    let update = dispatch_next(&mut game, GameCommand::AutoExplore);
    assert!(game.auto_explore.is_none());
    assert_eq!((game.turn, game.world_tick, game.player.position), before);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "travel.left-detection-area")
    );
}

#[test]
fn query_and_full_pack_stop_without_retrying_the_same_item() {
    for query in [false, true] {
        let mut game = arena();
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Wanted,
                rfb_protocol::LocaleDto::ZhCn,
                if query { ";物品" } else { "物品" }.into()
            )
            .is_empty()
        );
        if !query {
            for i in 0..game.inventory_slot_capacity() {
                give_inventory_item(&mut game, &format!("test.full.{i}"), "demo.item.dagger");
            }
        }
        give_inventory_item(&mut game, "test.explore.item", "demo.item.ration-of-food");
        game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
        game.reveal_current_visibility();
        let before = game.player.position;
        dispatch_next(&mut game, GameCommand::AutoExplore);
        assert!(game.auto_explore.is_none(), "query={query}");
        assert_eq!(game.player.position, before);
        assert_eq!(game.mogaminator.pending_query.is_some(), query);
        assert!(
            game.items.iter().any(|item| item.id == "test.explore.item"
                && item.location == ItemLocation::Ground(before))
        );
    }
}

#[test]
fn wilderness_scroll_translates_target_and_saved_state() {
    let mut game = arena();
    game.player.position = Position { x: 131, y: 33 };
    game.explored.fill(true);
    for x in 129..=141 {
        replace_terrain(&mut game, Position { x, y: 33 }, "demo.terrain.floor");
    }
    let index = game.index(Position { x: 141, y: 33 }).unwrap();
    game.explored[index] = false;
    let offset = game.wilderness_view_offset;
    dispatch_next(&mut game, GameCommand::AutoExplore);
    assert_ne!(game.wilderness_view_offset, offset);
    let state = game.auto_explore.as_ref().unwrap();
    assert_eq!(state.position, game.player.position);
    assert!(
        matches!(state.target, Some(AutoExploreTargetDto::Frontier { position })
        if position.x - game.player.position.x == 8 && position.y == game.player.position.y)
    );
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, GameCommand::ContinueAutoExplore),
        dispatch_next(&mut restored, GameCommand::ContinueAutoExplore)
    );
}
