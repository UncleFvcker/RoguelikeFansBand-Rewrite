// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use rfb_protocol::{AutoGetTargetDto, ItemFeelingDto, LocaleDto};

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
    game.explored.fill(true);
    for y in 0..=8 {
        for x in 0..=12 {
            replace_terrain(&mut game, at(x, y), "demo.terrain.floor");
        }
    }
    game.player.position = at(2, 4);
    game.mogaminator.enabled = false;
    game
}
fn ground(game: &mut Game, id: &str, kind: &str, position: Position) {
    give_inventory_item(game, id, kind);
    game.items.last_mut().unwrap().location = ItemLocation::Ground(position);
    game.item_property_knowledge
        .entry(id.into())
        .or_default()
        .discovered = true;
}
fn selected(game: &mut Game) -> Option<AutoGetTargetDto> {
    dispatch_next(game, GameCommand::FindNearestUnknownItem)
        .events
        .into_iter()
        .find_map(|event| match event.outcome {
            Some(GameEventOutcomeDto::UnknownItemTravelTarget { target }) => Some(target),
            _ => None,
        })
}

#[test]
fn ordinary_travel_item_policy_does_not_change_directed_unknown_item_travel() {
    let mut base = arena();
    ground(&mut base, "test.detour", "demo.item.dagger", at(3, 4));
    // Pin the route, so this fixture tests encounter policy rather than path tie-breaking.
    for y in 0..=8 {
        if y != 4 {
            for x in 0..=12 {
                replace_terrain(&mut base, at(x, y), "demo.terrain.wall");
            }
        }
    }
    for ignore in [false, true] {
        let mut game = base.clone();
        game.operation_options.travel_ignore_items = ignore;
        let before = (game.player.position, game.turn, game.rng.clone());
        let stopped = dispatch_next(
            &mut game,
            GameCommand::TravelLocal {
                destination: at(6, 4),
            },
        );
        assert_eq!((game.player.position, game.turn, game.rng.clone()), before);
        assert!(stopped.events.iter().any(|e| e.kind == "travel.item-found"));
        dispatch_next(
            &mut game,
            GameCommand::TravelUnknownItem {
                object_id: "test.detour".into(),
                destination: at(3, 4),
            },
        );
        assert_eq!(game.player.position, at(3, 4));
    }
    base.identify_item_instance("test.detour", ItemIdentificationRequest::new(true));
    for ignore in [false, true] {
        let mut game = base.clone();
        game.operation_options.travel_ignore_items = ignore;
        dispatch_next(
            &mut game,
            GameCommand::TravelLocal {
                destination: at(6, 4),
            },
        );
        assert_eq!(game.player.position, at(if ignore { 3 } else { 2 }, 4));
    }
}

#[test]
fn unexamined_items_use_source_feelings_and_device_exception() {
    let mut game = arena();
    ground(&mut game, "test.item", "demo.item.dagger", at(4, 4));
    for feeling in [
        None,
        Some(ItemFeelingDto::Average),
        Some(ItemFeelingDto::Good),
        Some(ItemFeelingDto::Bad),
        Some(ItemFeelingDto::Broken),
        Some(ItemFeelingDto::Cursed),
        Some(ItemFeelingDto::Excellent),
        Some(ItemFeelingDto::Special),
        Some(ItemFeelingDto::Awful),
        Some(ItemFeelingDto::Terrible),
        Some(ItemFeelingDto::Enchanted),
    ] {
        game.item_property_knowledge
            .get_mut("test.item")
            .unwrap()
            .feeling = feeling;
        let eligible = feeling.is_none_or(|value| {
            matches!(
                value,
                ItemFeelingDto::Excellent
                    | ItemFeelingDto::Special
                    | ItemFeelingDto::Awful
                    | ItemFeelingDto::Terrible
                    | ItemFeelingDto::Enchanted
            )
        });
        assert_eq!(
            !game.unknown_item_travel_candidates().is_empty(),
            eligible,
            "{feeling:?}"
        );
    }
    game.items.clear();
    game.item_property_knowledge.clear();
    ground(
        &mut game,
        "test.device",
        "demo.item.resonance-rod",
        at(4, 4),
    );
    game.item_property_knowledge
        .get_mut("test.device")
        .unwrap()
        .feeling = Some(ItemFeelingDto::Average);
    assert_eq!(game.unknown_item_travel_candidates().len(), 1);
    game.identify_item_instance("test.device", ItemIdentificationRequest::new(false));
    assert!(
        game.unknown_item_travel_candidates().is_empty(),
        "ordinary identification already counts as known"
    );
    game.identify_item_instance("test.device", ItemIdentificationRequest::new(true));
    assert!(game.unknown_item_travel_candidates().is_empty());
}

#[test]
fn rule_gating_accepts_query_and_pickup_but_not_destroy_or_leave() {
    let mut game = arena();
    ground(&mut game, "test.item", "demo.item.dagger", at(4, 4));
    for (enabled, source, expected) in [
        (false, "~items", true),
        (true, "# empty", true),
        (true, "items", true),
        (true, ";items", true),
        (true, "!items", false),
        (true, "~items", false),
        (true, "potions", false),
    ] {
        assert!(
            game.configure_mogaminator(
                enabled,
                false,
                AutoGetModeDto::Off,
                LocaleDto::EnUs,
                source.into()
            )
            .is_empty()
        );
        game.interface_locale = LocaleDto::EnUs;
        assert_eq!(
            !game.unknown_item_travel_candidates().is_empty(),
            expected,
            "{enabled} {source}"
        );
    }
    game.items.last_mut().unwrap().inscription = Some("=g".into());
    assert_eq!(
        game.unknown_item_travel_candidates().len(),
        1,
        "source implicit =g entry precedes file rules"
    );
}

#[test]
fn hidden_items_gold_and_foot_items_are_not_targets() {
    let mut game = arena();
    ground(&mut game, "test.foot", "demo.item.dagger", at(2, 4));
    ground(&mut game, "test.hidden", "demo.item.dagger", at(4, 4));
    game.item_property_knowledge
        .get_mut("test.hidden")
        .unwrap()
        .discovered = false;
    game.gold_piles.push(GoldPile {
        id: "test.gold".into(),
        position: at(3, 4),
        amount: 10,
        discovered: true,
        appearance: rfb_protocol::GoldAppearanceDto::Gold,
    });
    assert!(game.unknown_item_travel_candidates().is_empty());
}

#[test]
fn selects_shortest_route_not_straight_line_and_breaks_ties_by_id() {
    let mut game = arena();
    for y in 0..=6 {
        replace_terrain(&mut game, at(3, y), "demo.terrain.wall");
    }
    ground(&mut game, "test.near", "demo.item.dagger", at(4, 4));
    ground(&mut game, "test.far", "demo.item.dagger", at(1, 0));
    assert_eq!(selected(&mut game).unwrap().object_id, "test.far");
    ground(&mut game, "test.alpha", "demo.item.dagger", at(1, 0));
    game.items.reverse();
    assert_eq!(selected(&mut game).unwrap().object_id, "test.alpha");
}

#[test]
fn terrain_cost_changes_selection_and_the_actual_travel_route() {
    let mut game = arena();
    game.terrain.fill("demo.terrain.wall".into());
    for p in [at(2, 4), at(3, 4), at(4, 4), at(2, 3), at(2, 2), at(2, 1)] {
        replace_terrain(&mut game, p, "demo.terrain.floor");
    }
    replace_terrain(&mut game, at(3, 4), "demo.terrain.surface-lava-shallow");
    ground(&mut game, "test.near", "demo.item.dagger", at(4, 4));
    ground(&mut game, "test.far", "demo.item.dagger", at(2, 1));
    assert_eq!(selected(&mut game).unwrap().object_id, "test.far");
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_LEVITATION, 100, "test.fly").status);
    assert_eq!(selected(&mut game).unwrap().object_id, "test.near");
    for terrain in [
        "demo.terrain.shallow-waste",
        "demo.terrain.deep-waste",
        "demo.terrain.surface-lava-deep",
    ] {
        replace_terrain(&mut game, at(3, 4), terrain);
        assert_eq!(
            selected(&mut game).unwrap().object_id,
            "test.far",
            "{terrain}"
        );
    }
    game.player
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    assert_eq!(selected(&mut game).unwrap().object_id, "test.near");
    game.player.statuses.clear();
    replace_terrain(&mut game, at(3, 4), "demo.terrain.surface-water-deep");
    assert_eq!(selected(&mut game).unwrap().object_id, "test.near");
    for i in 0..12 {
        give_inventory_item(
            &mut game,
            &format!("test.weight.{i}"),
            "demo.item.full-plate-armour",
        );
    }
    assert!(game.carried_weight_tenths_pound() > game.player_carry_capacity_tenths_pound());
    assert_eq!(selected(&mut game).unwrap().object_id, "test.far");
    let mut route = arena();
    replace_terrain(&mut route, at(3, 4), "demo.terrain.surface-lava-shallow");
    assert_eq!(
        route.next_local_travel_direction(at(4, 4)),
        Some(Direction::NorthEast)
    );
    dispatch_next(
        &mut route,
        GameCommand::TravelLocal {
            destination: at(4, 4),
        },
    );
    assert_eq!(route.player.position, at(3, 3));
}

#[test]
fn no_candidates_and_no_route_report_distinct_zero_time_results() {
    let mut game = arena();
    let before = (
        game.turn,
        game.world_tick,
        game.rng.clone(),
        game.player.position,
    );
    let update = dispatch_next(&mut game, GameCommand::FindNearestUnknownItem);
    assert!(
        update
            .events
            .iter()
            .any(|e| e.message_key == "game-unknown-item-none")
    );
    ground(&mut game, "test.blocked", "demo.item.dagger", at(20, 4));
    let update = dispatch_next(&mut game, GameCommand::FindNearestUnknownItem);
    assert!(
        update
            .events
            .iter()
            .any(|e| e.message_key == "game-unknown-item-no-route")
    );
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

#[test]
fn locked_target_cannot_move_or_become_known_and_queries_stop_at_arrival() {
    let mut game = arena();
    ground(&mut game, "test.target", "demo.item.dagger", at(4, 4));
    let target = selected(&mut game).unwrap();
    let command = GameCommand::TravelUnknownItem {
        object_id: target.object_id.clone(),
        destination: target.position,
    };
    dispatch_next(&mut game, command.clone());
    assert_eq!(game.player.position, at(3, 4));
    for identified in [true, false] {
        let mut invalid = game.clone();
        if identified {
            invalid.identify_item_instance("test.target", ItemIdentificationRequest::new(false));
        } else {
            invalid.items.clear();
            invalid.item_property_knowledge.clear();
        }
        let before = (
            invalid.turn,
            invalid.world_tick,
            invalid.rng.clone(),
            invalid.player.position,
        );
        let update = dispatch_next(&mut invalid, command.clone());
        assert!(
            update
                .events
                .iter()
                .any(|e| e.message_key == "game-unknown-item-target-lost")
        );
        assert_eq!(
            (
                invalid.turn,
                invalid.world_tick,
                invalid.rng.clone(),
                invalid.player.position
            ),
            before
        );
    }
    let mut moved = game.clone();
    moved.items.last_mut().unwrap().location = ItemLocation::Ground(at(5, 4));
    let before = (moved.turn, moved.world_tick, moved.rng.clone());
    let update = dispatch_next(&mut moved, command.clone());
    assert!(
        update
            .events
            .iter()
            .any(|e| e.message_key == "game-unknown-item-target-lost")
    );
    assert_eq!((moved.turn, moved.world_tick, moved.rng.clone()), before);
    game.interface_locale = LocaleDto::EnUs;
    game.configure_mogaminator(
        true,
        false,
        AutoGetModeDto::Off,
        LocaleDto::EnUs,
        ";items".into(),
    );
    dispatch_next(&mut game, command);
    assert_eq!(game.player.position, at(4, 4));
    assert!(game.mogaminator.pending_query.is_some());
    assert_eq!(
        game.items.last().unwrap().location,
        ItemLocation::Ground(at(4, 4))
    );
    assert_eq!(
        game.item_identification(game.items.last().unwrap()),
        ItemIdentificationDto::Unexamined
    );
}

#[test]
fn selection_and_steps_are_reproducible_after_save_and_ignore_hidden_geometry() {
    let mut game = arena();
    ground(&mut game, "test.target", "demo.item.dagger", at(6, 4));
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, GameCommand::FindNearestUnknownItem),
        dispatch_next(&mut restored, GameCommand::FindNearestUnknownItem)
    );
    let command = GameCommand::TravelUnknownItem {
        object_id: "test.target".into(),
        destination: at(6, 4),
    };
    assert_eq!(
        dispatch_next(&mut game, command.clone()),
        dispatch_next(&mut restored, command)
    );
    let hidden = at(10, 8);
    let index = game.index(hidden).unwrap();
    game.explored[index] = false;
    restored.explored[index] = false;
    replace_terrain(&mut restored, hidden, "demo.terrain.floor");
    replace_terrain(&mut game, hidden, "demo.terrain.wall");
    assert_eq!(
        game.nearest_unknown_item_target(),
        restored.nearest_unknown_item_target()
    );
}
