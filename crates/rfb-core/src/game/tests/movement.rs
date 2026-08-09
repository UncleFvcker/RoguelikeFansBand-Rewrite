// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn movement_produces_fov_deltas_and_remembers_explored_cells() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let first = game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement should execute");
    assert!(!first.changed_visual_cells.is_empty());
    let snapshot = game.snapshot();
    assert_eq!(
        visual_at(&snapshot, Position { x: 11, y: 3 }).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 12, y: 3 }).visibility,
        VisibilityState::Hidden
    );

    for seq in 2..=7 {
        game.dispatch(command(
            seq,
            seq - 1,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("eastward exploration should execute");
    }
    assert_eq!(
        visual_at(&game.snapshot(), Position { x: 1, y: 3 }).visibility,
        VisibilityState::Remembered
    );
}

#[test]
fn local_travel_routes_around_known_obstacles_and_stops_for_visible_enemies() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.surface-path".to_owned());
    game.explored.fill(true);
    let start = game.player.position;
    let destination = Position {
        x: start.x + 3,
        y: start.y,
    };
    let wall = Position {
        x: start.x + 1,
        y: start.y,
    };
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    let trap = Position {
        x: start.x + 1,
        y: start.y + 1,
    };
    let trap_index = game
        .index(trap)
        .expect("trap position should be on the map");
    game.terrain[trap_index] = "demo.terrain.trap-echo-snare".to_owned();
    game.revealed_terrain.insert(trap);

    let tick_before = game.world_tick;
    dispatch_next(&mut game, GameCommand::TravelLocal { destination });
    assert_eq!(
        game.player.position,
        Position {
            x: start.x + 1,
            y: start.y - 1,
        }
    );
    assert!(game.world_tick > tick_before);

    let enemy_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let enemy = game.generated_actor(
        "test.local-travel.enemy".to_owned(),
        "demo.actor.ember-mote",
        enemy_position,
    );
    game.entities.push(enemy);
    let stopped_at = game.player.position;
    let tick_before = game.world_tick;
    dispatch_next(&mut game, GameCommand::TravelLocal { destination });
    assert_eq!(game.player.position, stopped_at);
    assert_eq!(game.world_tick, tick_before);
}
