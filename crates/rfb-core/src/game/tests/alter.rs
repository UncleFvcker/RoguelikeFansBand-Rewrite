// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn arena() -> Game {
    let mut game = Game::new_with_build(509, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    game.player.position = Position { x: 95, y: 32 };
    for y in 30..=34 {
        for x in 93..=97 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.reveal_current_visibility();
    game
}

#[test]
fn walking_convenience_uses_existing_actions_and_keeps_explicit_alter_available() {
    for terrain in ["demo.terrain.door-closed", "demo.terrain.created-trap"] {
        let mut game = arena();
        let target = game.position_in_direction(Direction::East);
        replace_terrain(&mut game, target, terrain);
        game.revealed_terrain.insert(target);
        let mut explicit = game.clone();
        let automatic = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        let direct = dispatch_next(
            &mut explicit,
            if terrain.contains("door") {
                GameCommand::OpenDoor {
                    direction: Direction::East,
                }
            } else {
                GameCommand::DisarmTrap {
                    direction: Direction::East,
                }
            },
        );
        assert_eq!(automatic, direct);
    }
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.door-closed");
    game.operation_options.easy_open = false;
    game.operation_options.easy_disarm = false;
    let origin = game.player.position;
    let blocked = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, origin);
    assert!(blocked.events.iter().any(|e| e.kind == "move.blocked"));
    let opened = dispatch_next(
        &mut game,
        GameCommand::Alter {
            direction: Direction::East,
        },
    );
    assert!(
        opened
            .events
            .iter()
            .any(|e| e.kind == "terrain.door-opened")
    );
}

#[test]
fn terrain_selection_uses_source_priority_and_existing_action_outcomes() {
    for (terrain, command) in [
        (
            "demo.terrain.door-closed",
            GameCommand::OpenDoor {
                direction: Direction::East,
            },
        ),
        (
            "demo.terrain.door-jammed-1",
            GameCommand::BashDoor {
                direction: Direction::East,
            },
        ),
        (
            "demo.terrain.wall",
            GameCommand::DigTerrain {
                direction: Direction::East,
            },
        ),
        (
            "demo.terrain.rubble",
            GameCommand::DigTerrain {
                direction: Direction::East,
            },
        ),
        (
            "demo.terrain.door-open",
            GameCommand::CloseDoor {
                direction: Direction::East,
            },
        ),
        (
            "demo.terrain.created-trap",
            GameCommand::DisarmTrap {
                direction: Direction::East,
            },
        ),
    ] {
        let mut game = arena();
        let target = game.position_in_direction(Direction::East);
        replace_terrain(&mut game, target, terrain);
        game.revealed_terrain.insert(target);
        let mut direct = game.clone();
        let origin = game.player.position;
        assert_eq!(
            dispatch_next(
                &mut game,
                GameCommand::Alter {
                    direction: Direction::East
                }
            ),
            dispatch_next(&mut direct, command),
            "{terrain}"
        );
        assert_eq!(game.player.position, origin);
        assert_eq!(game.turn, 1);
    }
}

#[test]
fn actor_takes_priority_even_in_a_wall_without_moving_or_tunneling() {
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.wall");
    game.push_generated_actor(
        "test.alter.actor".into(),
        "demo.actor.clear-icky-thing",
        target,
    );
    game.entities[0].visible_invisible = false;
    assert_eq!(
        game.alter_action(Direction::East, &mut vec![]),
        GameAction::AttackAdjacent {
            direction: Direction::East
        }
    );
    let origin = game.player.position;
    let update = dispatch_next(
        &mut game,
        GameCommand::Alter {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, origin);
    assert_eq!(game.terrain_at(target), "demo.terrain.wall");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind.starts_with("combat."))
    );
    assert!(
        !update
            .events
            .iter()
            .any(|event| event.kind.starts_with("terrain.dig"))
    );
}

#[test]
fn concealed_traps_and_empty_ground_spend_an_action_without_search_or_pickup() {
    for terrain in ["demo.terrain.floor", "demo.terrain.created-trap"] {
        let mut game = arena();
        game.searching = true;
        let target = game.position_in_direction(Direction::East);
        replace_terrain(&mut game, target, terrain);
        give_inventory_item(
            &mut game,
            "test.alter.chest",
            "demo.item.large-wooden-chest",
        );
        game.items.last_mut().unwrap().location = ItemLocation::Ground(target);
        let origin = game.player.position;
        let rng = game.rng.clone();
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
                .any(|event| event.kind == "terrain.alter-empty")
        );
        assert_eq!(game.player.position, origin);
        assert_eq!(game.turn, 1);
        assert!(game.world_tick > 0);
        assert_eq!(game.rng, rng);
        assert!(!game.revealed_terrain.contains(&target));
        assert!(game.searching);
    }
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.door-secret");
    assert_eq!(
        game.alter_action(Direction::East, &mut vec![]),
        GameAction::DigTerrain {
            direction: Direction::East
        }
    );
    game.revealed_terrain.insert(target);
    assert_eq!(
        game.alter_action(Direction::East, &mut vec![]),
        GameAction::OpenDoor {
            direction: Direction::East
        }
    );
}

#[test]
fn confusion_selects_once_and_paralysis_does_not_select_or_operate() {
    let mut game = arena();
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 100, "test.confusion").status);
    for direction in TERRAIN_INTERACTION_DIRECTIONS {
        let target = game.position_in_direction(direction);
        replace_terrain(&mut game, target, "demo.terrain.door-open");
    }
    let mut expected = game.clone();
    let actual = expected.confused_direction(Direction::East, &mut vec![]);
    expected.close_door(actual).unwrap();
    let rng = expected.rng.clone();
    dispatch_next(
        &mut game,
        GameCommand::Alter {
            direction: Direction::East,
        },
    );
    assert_eq!(game.terrain, expected.terrain);
    assert_eq!(game.rng, rng);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_PARALYSIS, 100, "test.paralysis").status);
    let terrain = game.terrain.clone();
    let rng = game.rng.clone();
    dispatch_next(
        &mut game,
        GameCommand::Alter {
            direction: Direction::East,
        },
    );
    assert_eq!(game.terrain, terrain);
    assert_eq!(game.rng, rng);
}

#[test]
fn alter_reproduces_after_save_and_rejects_world_map_before_mutation() {
    let mut game = arena();
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.door-closed");
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(
        dispatch_next(
            &mut game,
            GameCommand::Alter {
                direction: Direction::East
            }
        ),
        dispatch_next(
            &mut restored,
            GameCommand::Alter {
                direction: Direction::East
            }
        )
    );
    game.map_scale = MapScaleDto::World;
    let before = game.to_save();
    assert!(
        game.dispatch(GameCommandEnvelope {
            command_seq: game.last_command_seq + 1,
            expected_revision: game.revision,
            command: GameCommand::Alter {
                direction: Direction::East
            }
        })
        .is_err()
    );
    assert_eq!(game.to_save(), before);
}
