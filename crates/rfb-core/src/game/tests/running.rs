// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn arena(rows: &[&str]) -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    for (y, row) in rows.iter().enumerate() {
        for (x, cell) in row.chars().enumerate() {
            let position = at(x as i32, y as i32);
            replace_terrain(
                &mut game,
                position,
                if cell == '#' {
                    "demo.terrain.wall"
                } else {
                    "demo.terrain.floor"
                },
            );
            let index = game.index(position).unwrap();
            game.explored[index] = true;
            if cell == '@' {
                game.player.position = position;
            }
        }
    }
    game.reveal_current_visibility();
    game
}

fn at(x: i32, y: i32) -> Position {
    Position {
        x: 90 + x,
        y: 30 + y,
    }
}

#[test]
fn counted_runs_include_the_last_step_validate_limits_and_restore_large_budgets() {
    for steps in [1, 2, 9999] {
        let mut game = arena(&["########", "#@.....#", "########"]);
        let update = dispatch_next(
            &mut game,
            GameCommand::Run {
                direction: Direction::East,
                max_steps: Some(steps),
            },
        );
        assert_eq!(game.player.position, at(2, 1));
        assert_eq!(game.turn, 1);
        if steps == 1 {
            assert!(game.running.is_none());
            assert!(
                update
                    .events
                    .iter()
                    .any(|event| event.kind == "run.stopped")
            );
        } else {
            assert_eq!(game.running.as_ref().unwrap().remaining_steps, steps - 1);
            let mut restored =
                Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
            assert_eq!(
                dispatch_next(&mut game, GameCommand::ContinueRun),
                dispatch_next(&mut restored, GameCommand::ContinueRun)
            );
            assert_eq!(game.player.position, at(3, 1));
            if steps == 2 {
                assert!(game.running.is_none());
            }
        }
    }
    for steps in [0, 10000] {
        let mut game = arena(&["######", "#@...#", "######"]);
        let before = (game.player.position, game.turn, game.rng.clone());
        dispatch_next(
            &mut game,
            GameCommand::Run {
                direction: Direction::East,
                max_steps: Some(steps),
            },
        );
        assert_eq!((game.player.position, game.turn, game.rng.clone()), before);
        assert!(game.running.is_none());
    }
}

fn run_to_stop(game: &mut Game, direction: Direction) -> Vec<Position> {
    let mut positions = vec![];
    dispatch_next(
        game,
        GameCommand::Run {
            max_steps: None,
            direction,
        },
    );
    positions.push(game.player.position);
    for _ in 0..30 {
        if game.running.is_none() {
            return positions;
        }
        let before = game.player.position;
        dispatch_next(game, GameCommand::ContinueRun);
        if game.player.position != before {
            positions.push(game.player.position);
        }
    }
    panic!("run did not stop: {positions:?}");
}

#[test]
fn straight_and_bent_corridors_follow_source_without_cutting_corners() {
    let mut straight = arena(&["########", "#@.....#", "########"]);
    assert_eq!(
        run_to_stop(&mut straight, Direction::East),
        (2..=6).map(|x| at(x, 1)).collect::<Vec<_>>()
    );
    let mut bent = arena(&["#######", "#@....#", "#####.#", "#####.#", "#######"]);
    assert_eq!(
        run_to_stop(&mut bent, Direction::East),
        vec![at(2, 1), at(3, 1), at(4, 1), at(5, 1), at(5, 2), at(5, 3)]
    );
}

#[test]
fn global_corner_cutting_takes_only_the_enclosed_diagonal() {
    let mut game = arena(&["#######", "#@....#", "#####.#", "#####.#", "#######"]);
    game.operation_options.cut_corners = true;
    assert_eq!(
        run_to_stop(&mut game, Direction::East),
        vec![at(2, 1), at(3, 1), at(4, 1), at(5, 2), at(5, 3)]
    );
    let mut junction = arena(&["########", "#@.....#", "#####..#", "########"]);
    junction.operation_options.cut_corners = true;
    run_to_stop(&mut junction, Direction::East);
    assert_eq!(junction.player.position, at(4, 1));
}

#[test]
fn run_stop_preferences_are_independent_and_use_known_terrain() {
    for (terrain, stop, hidden) in [
        ("demo.terrain.door-open", true, false),
        ("demo.terrain.stairs-down", false, false),
        ("demo.terrain.magma-treasure", true, false),
        ("demo.terrain.magma-hidden-treasure", true, true),
    ] {
        let mut game = arena(&["########", "#@.....#", "########"]);
        let vein = terrain.contains("magma");
        replace_terrain(&mut game, at(4, if vein { 0 } else { 1 }), terrain);
        game.operation_options.run_stops.open_doors = stop;
        game.operation_options.run_stops.stairs = stop;
        game.operation_options.run_stops.known_treasure = stop;
        run_to_stop(&mut game, Direction::East);
        assert_eq!(
            game.player.position,
            if stop && !hidden { at(3, 1) } else { at(6, 1) },
            "{terrain}"
        );
    }
}

#[test]
fn running_stops_at_detection_boundary_before_spending_a_turn() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    game.detection_coverage.traps.insert(game.player.position);
    let before = (game.player.position, game.turn, game.rng.clone());
    dispatch_next(
        &mut game,
        GameCommand::Run {
            direction: Direction::East,
            max_steps: None,
        },
    );
    assert_eq!((game.player.position, game.turn, game.rng.clone()), before);
    game.travel_options.disturb_trap_detect = false;
    dispatch_next(
        &mut game,
        GameCommand::Run {
            direction: Direction::East,
            max_steps: Some(1),
        },
    );
    assert_eq!(game.player.position, at(2, 1));
}

#[test]
fn intersections_and_open_area_breaks_stop_before_choosing_a_branch() {
    let mut junction = arena(&[
        "#########",
        "#####.###",
        "#@......#",
        "#####.###",
        "#########",
    ]);
    run_to_stop(&mut junction, Direction::East);
    assert_eq!(junction.player.position, at(4, 2));
    let mut open = arena(&["#########", "#@......#", "####.####", "#########"]);
    run_to_stop(&mut open, Direction::East);
    assert_eq!(open.player.position, at(4, 1));
}

#[test]
fn diagonal_corridor_entry_turns_into_the_corridor() {
    let mut game = arena(&["#######", "#######", "##@####", "##....#", "#######"]);
    let positions = run_to_stop(&mut game, Direction::SouthEast);
    assert_eq!(positions, vec![at(3, 3), at(4, 3), at(5, 3)]);
}

#[test]
fn open_doors_are_ignored_but_stairs_closed_doors_and_traps_stop_running() {
    for (terrain, expected) in [
        ("demo.terrain.door-open", at(6, 1)),
        ("demo.terrain.stairs-down", at(3, 1)),
        ("demo.terrain.door-closed", at(3, 1)),
        ("demo.terrain.created-trap", at(3, 1)),
    ] {
        let mut game = arena(&["########", "#@.....#", "########"]);
        replace_terrain(&mut game, at(4, 1), terrain);
        game.revealed_terrain.insert(at(4, 1));
        run_to_stop(&mut game, Direction::East);
        assert_eq!(game.player.position, expected, "{terrain}");
    }
}

#[test]
fn stopping_and_cancellation_spend_no_energy_or_rng_and_cannot_resume_stale_run() {
    let mut game = arena(&["#####", "#@..#", "#####"]);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    let before = (
        game.turn,
        game.world_tick,
        game.rng.clone(),
        game.player.position,
    );
    dispatch_next(&mut game, GameCommand::CancelRun);
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.rng.clone(),
            game.player.position
        ),
        before
    );
    assert!(game.running.is_none());
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::North,
        },
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
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    let before = (game.turn, game.world_tick, game.rng.clone());
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
}

#[test]
fn running_state_round_trips_and_affects_the_state_hash() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    let saved_run = restored.running.take();
    assert_ne!(game.state_hash(), restored.state_hash());
    restored.running = saved_run;
    assert_eq!(
        dispatch_next(&mut game, GameCommand::ContinueRun),
        dispatch_next(&mut restored, GameCommand::ContinueRun)
    );
    let mut invalid = game.to_save();
    invalid.player.running.as_mut().unwrap().position.x += 1;
    assert!(Game::from_save(invalid, Game::default_behavior_preferences()).is_err());
}

#[test]
fn normal_commands_damage_and_step_limit_end_running() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::Wait);
    assert!(game.running.is_none());
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    game.running.as_mut().unwrap().remaining_steps = 1;
    let position = game.position_in_direction(Direction::East);
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!(game.player.position, position);
    assert!(game.running.is_none());
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    game.apply_final_player_damage(
        resolve_damage(
            DamagePacket::new(1, DamageType::Physical),
            ResistanceLevel::Normal,
        ),
        FatalityPolicy::BelowZero,
    );
    assert!(game.running.is_none());
}

#[test]
fn visible_objects_and_monsters_stop_before_the_next_step() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    give_inventory_item(&mut game, "test.run.item", "demo.item.ration-of-food");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(at(3, 1));
    game.reveal_current_visibility();
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!(game.player.position, at(2, 1));
    assert!(game.running.is_none());
    game.items.clear();
    game.push_generated_actor("test.run.monster".into(), "demo.actor.war-bear", at(4, 1));
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    let before = game.player.position;
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!(game.player.position, before);
    assert!(game.running.is_none());
}

#[test]
fn known_danger_uses_actual_traveler_abilities_and_concealment() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    replace_terrain(&mut game, at(4, 1), "demo.terrain.surface-lava-shallow");
    run_to_stop(&mut game, Direction::East);
    assert_eq!(game.player.position, at(3, 1));

    let mut hidden = arena(&["########", "#@.....#", "########"]);
    replace_terrain(&mut hidden, at(4, 1), "demo.terrain.door-secret");
    assert_eq!(hidden.known_terrain_at(at(4, 1)), "demo.terrain.wall");
    run_to_stop(&mut hidden, Direction::East);
    assert_eq!(hidden.player.position, at(3, 1));
    assert!(!hidden.revealed_terrain.contains(&at(4, 1)));
}

#[test]
fn changed_floor_and_confusion_do_not_resume_a_run() {
    let mut game = arena(&["########", "#@.....#", "########"]);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    game.running.as_mut().unwrap().floor_id = "old.floor".into();
    let before = (game.player.position, game.turn, game.rng.clone());
    dispatch_next(&mut game, GameCommand::ContinueRun);
    assert_eq!((game.player.position, game.turn, game.rng.clone()), before);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 100, "test.run").status);
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    assert!(game.running.is_none());
    assert_eq!((game.player.position, game.turn, game.rng.clone()), before);
}

#[test]
fn wilderness_scroll_translates_the_run_origin_and_survives_restore() {
    let mut game = arena(&["###", "#@#", "###"]);
    // The initial surface uses the same 66-column scrolling boundary as ordinary movement.
    game.player.position = Position { x: 131, y: 33 };
    for y in 32..=34 {
        for x in 129..=135 {
            let p = Position { x, y };
            replace_terrain(&mut game, p, "demo.terrain.floor");
            let index = game.index(p).unwrap();
            game.explored[index] = true;
        }
    }
    let offset = game.wilderness_view_offset;
    dispatch_next(
        &mut game,
        GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        },
    );
    assert_ne!(game.wilderness_view_offset, offset);
    let run = game
        .running
        .as_ref()
        .expect("scrolling alone does not stop running");
    assert_eq!(run.position.x - run.origin.x, 1);
    assert_eq!(run.position, game.player.position);
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        dispatch_next(&mut game, GameCommand::ContinueRun),
        dispatch_next(&mut restored, GameCommand::ContinueRun)
    );
}
