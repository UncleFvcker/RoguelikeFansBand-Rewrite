// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn arena() -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    game.mogaminator.enabled = false;
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 95, y: 32 };
    for y in 30..=34 {
        for x in 93..=99 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.reveal_current_visibility();
    game
}

fn floor_dagger(game: &mut Game, position: Position) {
    give_inventory_item(game, "test.walk.item", "demo.item.dagger");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(position);
}

#[test]
fn default_pickup_is_free_to_configure_and_round_trips() {
    let mut game = arena();
    assert!(!game.travel_options.always_pickup);
    let before = (game.turn, game.world_tick, game.rng.clone());
    let old_hash = game.state_hash();
    let options = rfb_protocol::TravelOptionsDto {
        always_pickup: true,
        ..game.travel_options
    };
    let update = dispatch_next(&mut game, GameCommand::ConfigureTravel { options });
    assert!(update.travel_options.always_pickup);
    assert_eq!(before, (game.turn, game.world_tick, game.rng.clone()));
    assert_ne!(old_hash, game.state_hash());
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot().travel_options, options);
}

#[test]
fn special_walk_flips_only_this_steps_default_pickup_and_keeps_normal_turn_cost() {
    for always_pickup in [false, true] {
        let mut game = arena();
        game.travel_options.always_pickup = always_pickup;
        let destination = game.position_in_direction(Direction::East);
        floor_dagger(&mut game, destination);
        let mut ordinary = game.clone();
        let before = game.turn;
        let special = dispatch_next(
            &mut game,
            GameCommand::WalkSpecial {
                direction: Direction::East,
            },
        );
        dispatch_next(
            &mut ordinary,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(game.turn, before + 1);
        assert_eq!(
            (game.world_tick, game.rng.clone()),
            (ordinary.world_tick, ordinary.rng.clone())
        );
        assert_eq!(game.player.position, destination);
        assert_eq!(
            game.items[0].location == ItemLocation::Inventory,
            !always_pickup
        );
        assert_eq!(
            ordinary.items[0].location == ItemLocation::Inventory,
            always_pickup
        );
        assert_eq!(game.travel_options.always_pickup, always_pickup);
        assert!(special.command_repeatable);
        // A subsequent ordinary step uses the saved setting, not the previous flip.
        game.items[0].location = ItemLocation::Ground(game.position_in_direction(Direction::East));
        dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        assert_eq!(
            game.items[0].location == ItemLocation::Inventory,
            always_pickup
        );
    }
}

#[test]
fn stay_uses_default_pickup_in_one_wait_turn_and_gold_is_always_collected() {
    for always_pickup in [false, true] {
        let mut game = arena();
        game.travel_options.always_pickup = always_pickup;
        game.searching = true;
        let position = game.player.position;
        floor_dagger(&mut game, position);
        let pile = game.generate_gold_pile(position, 1, false).unwrap();
        let amount = pile.amount;
        game.gold_piles.push(pile);
        let mut waiting = game.clone();
        let before_gold = game.gold;
        let update = dispatch_next(&mut game, GameCommand::Stay);
        dispatch_next(&mut waiting, GameCommand::Wait);
        assert_eq!(game.player.position, position);
        assert_eq!(
            (game.turn, game.world_tick, game.rng.clone()),
            (waiting.turn, waiting.world_tick, waiting.rng.clone())
        );
        assert_eq!(
            game.items[0].location == ItemLocation::Inventory,
            always_pickup
        );
        assert_eq!(game.gold, before_gold + amount);
        assert!(game.gold_piles.is_empty());
        assert_eq!(waiting.items[0].location, ItemLocation::Ground(position));
        assert!(update.command_repeatable);
    }
}

#[test]
fn special_walk_does_not_pick_up_when_blocked_attacking_or_paralyzed() {
    for obstacle in ["wall", "monster", "paralysis"] {
        let mut game = arena();
        let origin = game.player.position;
        let target = game.position_in_direction(Direction::East);
        floor_dagger(&mut game, origin);
        match obstacle {
            "wall" => replace_terrain(&mut game, target, "demo.terrain.wall"),
            "monster" => game.push_generated_actor(
                "test.walk.actor".into(),
                "demo.actor.adobe-golem",
                target,
            ),
            _ => {
                game.player.statuses.push(
                    monster_combat::melee_status(STATUS_PARALYSIS, 100, "test.walk.paralysis")
                        .status,
                );
            }
        }
        let update = dispatch_next(
            &mut game,
            GameCommand::WalkSpecial {
                direction: Direction::East,
            },
        );
        assert_eq!(game.player.position, origin);
        assert_eq!(game.items[0].location, ItemLocation::Ground(origin));
        assert!(!update.command_repeatable);
    }
}

#[test]
fn special_walk_keeps_mogaminator_rules_even_when_flipping_default_pickup_off() {
    for (rule, result) in [
        ("items", "picked"),
        (";items", "query"),
        ("!items", "destroyed"),
        ("~items", "left"),
    ] {
        let mut game = arena();
        game.travel_options.always_pickup = true;
        game.interface_locale = rfb_protocol::LocaleDto::EnUs;
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            rfb_protocol::LocaleDto::EnUs,
            rule.into(),
        );
        let target = game.position_in_direction(Direction::East);
        floor_dagger(&mut game, target);
        let update = dispatch_next(
            &mut game,
            GameCommand::WalkSpecial {
                direction: Direction::East,
            },
        );
        match result {
            "picked" => assert_eq!(game.items[0].location, ItemLocation::Inventory),
            "destroyed" => assert!(game.items.is_empty()),
            "query" => {
                assert!(game.mogaminator.pending_query.is_some());
                assert!(!update.command_repeatable);
                assert_eq!(game.items[0].location, ItemLocation::Ground(target));
            }
            _ => assert_eq!(game.items[0].location, ItemLocation::Ground(target)),
        }
        assert!(game.travel_options.always_pickup);
    }
}

#[test]
fn full_pack_stops_counted_special_walk_without_losing_the_floor_item() {
    let mut game = arena();
    for i in 0..game.snapshot().player.inventory_slot_capacity {
        give_inventory_item(&mut game, &format!("test.full.{i}"), "demo.item.dagger");
    }
    let target = game.position_in_direction(Direction::East);
    floor_dagger(&mut game, target);
    let update = dispatch_next(
        &mut game,
        GameCommand::WalkSpecial {
            direction: Direction::East,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.pickup.inventory-full")
    );
    assert!(!update.command_repeatable);
    assert_eq!(
        game.items.last().unwrap().location,
        ItemLocation::Ground(target)
    );
}

#[test]
fn repeat_projection_keeps_each_move_search_and_wait_a_normal_replayable_turn() {
    let mut game = arena();
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    for command in [
        GameCommand::Move {
            direction: Direction::East,
        },
        GameCommand::Search,
        GameCommand::Wait,
    ] {
        let update = dispatch_next(&mut game, command.clone());
        assert!(update.command_repeatable);
        assert_eq!(update, dispatch_next(&mut restored, command));
    }
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.wall");
    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(!update.command_repeatable);

    let mut game = arena();
    game.player.position = Position { x: 70, y: 38 };
    game.reveal_current_visibility();
    let update = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::South,
        },
    );
    assert_eq!(game.player.position, Position { x: 70, y: 39 });
    assert!(
        !update.command_repeatable,
        "arriving at a shop interrupts repeated movement"
    );
}

#[test]
fn completed_actions_and_combat_stop_while_retryable_digging_can_continue() {
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.door-closed");
    let update = dispatch_next(
        &mut game,
        GameCommand::OpenDoor {
            direction: Direction::East,
        },
    );
    assert!(!update.command_repeatable);
    assert_eq!(game.terrain_at(target), "demo.terrain.door-open");
    replace_terrain(&mut game, target, "demo.terrain.rubble");
    let retry = (0..64)
        .find_map(|seed| {
            let mut attempt = game.clone();
            attempt.rng = RfbRng::seeded(seed);
            let update = dispatch_next(
                &mut attempt,
                GameCommand::DigTerrain {
                    direction: Direction::East,
                },
            );
            update
                .events
                .iter()
                .any(|event| {
                    event.kind == "terrain.dig-failed" && event.args["retryable"] == "true"
                })
                .then_some(update)
        })
        .expect("rubble has retryable failures");
    assert!(retry.command_repeatable);
    game.push_generated_actor("test.repeat.actor".into(), "demo.actor.adobe-golem", target);
    let update = dispatch_next(
        &mut game,
        GameCommand::Alter {
            direction: Direction::East,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind.starts_with("combat."))
    );
    assert!(!update.command_repeatable);
}

#[test]
fn discoveries_and_paralysis_stop_repetition_and_spike_is_never_repeated() {
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.door-secret");
    let mut discovered = false;
    for _ in 0..50 {
        let update = dispatch_next(&mut game, GameCommand::Search);
        if update
            .events
            .iter()
            .any(|event| event.kind == "terrain.secret-discovered")
        {
            assert!(!update.command_repeatable);
            discovered = true;
            break;
        }
    }
    assert!(discovered);
    give_inventory_item(&mut game, "test.repeat.spike", "demo.item.iron-spike");
    assert!(
        !dispatch_next(
            &mut game,
            GameCommand::SpikeDoor {
                direction: Direction::East
            }
        )
        .command_repeatable
    );
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_PARALYSIS, 100, "test.paralysis").status);
    assert!(!dispatch_next(&mut game, GameCommand::Wait).command_repeatable);
}

#[test]
fn door_and_trap_attempts_allow_retry_only_until_the_operation_succeeds() {
    for (terrain, command, failure) in [
        (
            "demo.terrain.door-secret",
            GameCommand::OpenDoor {
                direction: Direction::East,
            },
            "terrain.door-unlock-failed",
        ),
        (
            "demo.terrain.door-closed",
            GameCommand::BashDoor {
                direction: Direction::East,
            },
            "terrain.door-bash-failed",
        ),
        (
            "demo.terrain.created-trap",
            GameCommand::DisarmTrap {
                direction: Direction::East,
            },
            "terrain.trap-disarm-failed",
        ),
        (
            "demo.terrain.door-secret",
            GameCommand::Alter {
                direction: Direction::East,
            },
            "terrain.door-unlock-failed",
        ),
    ] {
        let mut base = arena();
        let target = base.position_in_direction(Direction::East);
        replace_terrain(&mut base, target, terrain);
        base.revealed_terrain.insert(target);
        base.reveal_current_visibility();
        let mut retried = false;
        let mut succeeded = false;
        for seed in 0..64 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let update = dispatch_next(&mut game, command.clone());
            if update.events.iter().any(|event| event.kind == failure) {
                assert!(update.command_repeatable, "{command:?}: {update:?}");
                assert_eq!(game.terrain_at(target), terrain);
                retried = true;
            } else if game.terrain_at(target) != terrain {
                assert!(!update.command_repeatable);
                succeeded = true;
            }
            if retried && succeeded {
                break;
            }
        }
        assert!(
            retried && succeeded,
            "{command:?} must cover retry and completion"
        );
    }
}

#[test]
fn chest_repetition_stops_after_unlock_disarm_or_a_triggered_trap() {
    for disarm in [false, true] {
        let mut base = arena();
        let id = "test.repeat.chest";
        give_inventory_item(&mut base, id, "demo.item.large-wooden-chest");
        let item = base.items.last_mut().unwrap();
        item.location = ItemLocation::Ground(base.player.position);
        item.chest = Some(rfb_protocol::ChestSaveDto {
            difficulty: if disarm { 1 } else { 6 },
            opening_depth: 1,
        });
        base.mark_item_instances_discovered(&[id.into()]);
        base.identify_item_instance(id, ItemIdentificationRequest::new(false));
        base.set_floor_glow_at(base.player.position, true);
        let command = if disarm {
            GameCommand::DisarmChest { item_id: id.into() }
        } else {
            GameCommand::OpenChest { item_id: id.into() }
        };
        let failure = if disarm {
            "chest-disarm-failed"
        } else {
            "chest-unlock-failed"
        };
        let mut retried = false;
        let mut completed = false;
        let mut triggered = !disarm;
        for seed in 0..128 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let update = dispatch_next(&mut game, command.clone());
            if update
                .events
                .iter()
                .any(|event| event.message_key == failure)
            {
                assert!(update.command_repeatable);
                retried = true;
            } else {
                assert!(!update.command_repeatable);
                completed |= game
                    .items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .chest
                    .unwrap()
                    .difficulty
                    <= 0;
                triggered |= update
                    .events
                    .iter()
                    .any(|event| event.message_key == "chest-trap-triggered");
            }
            if retried && completed && triggered {
                break;
            }
        }
        assert!(retried && completed && triggered, "{command:?}");
    }
}

#[test]
fn counted_rest_spends_requested_turns_at_full_resources_and_keeps_danger_checks() {
    let mut game = arena();
    let update = dispatch_next(&mut game, GameCommand::Rest { turns: 3 });
    assert_eq!(rest_resolution(&update).completed_turns, 0);
    let update = dispatch_next(&mut game, GameCommand::RestForTurns { turns: 3 });
    assert_eq!(rest_resolution(&update).completed_turns, 3);
    assert_eq!(
        rest_resolution(&update).stop_reason,
        RestStopReasonDto::TurnLimit
    );
    for turns in [0, 10000] {
        let update = dispatch_next(&mut game, GameCommand::RestForTurns { turns });
        assert_eq!(rest_resolution(&update).completed_turns, 0);
        assert_eq!(
            rest_resolution(&update).stop_reason,
            RestStopReasonDto::InvalidTurns
        );
    }
    let target = game.position_in_direction(Direction::East);
    game.push_generated_actor(
        "test.repeat.hostile".into(),
        "demo.actor.adobe-golem",
        target,
    );
    let update = dispatch_next(&mut game, GameCommand::RestForTurns { turns: 3 });
    assert_eq!(rest_resolution(&update).completed_turns, 0);
    assert_eq!(
        rest_resolution(&update).stop_reason,
        RestStopReasonDto::EnemyVisible
    );
}
