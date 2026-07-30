// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn fixed_seed_and_commands_are_deterministic() {
    let mut left = Game::new(42);
    let mut right = Game::new(42);
    let commands = [
        GameCommand::Move {
            direction: Direction::East,
        },
        GameCommand::Move {
            direction: Direction::South,
        },
        GameCommand::Wait,
    ];

    for (index, game_command) in commands.into_iter().enumerate() {
        let seq = index as u32 + 1;
        let revision = index as u32;
        left.dispatch(command(seq, revision, game_command.clone()))
            .expect("left command should execute");
        right
            .dispatch(command(seq, revision, game_command))
            .expect("right command should execute");
    }

    assert_eq!(left.state_hash(), right.state_hash());
}

#[test]
fn normal_speed_monster_tracks_once_per_player_action() {
    let mut game = Game::new(42);
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("wait should advance the scheduler");

    assert_eq!(update.world_tick, 10);
    assert_eq!(update.player.energy_need, 0);
    assert_eq!(update.entities[0].position, Position { x: 7, y: 4 });
    assert_eq!(update.entities[0].energy_need, STANDARD_ACTION_COST);
    assert_eq!(update.changed_cells.len(), 2);
}

#[test]
fn fast_and_slow_monsters_use_the_same_energy_scheduler() {
    let mut fast = Game::new(42);
    fast.entities[0].speed = 120;
    let fast_update = fast
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("fast scheduler case should execute");
    assert_eq!(fast_update.world_tick, 10);
    assert_eq!(fast_update.entities[0].position, Position { x: 6, y: 3 });
    assert_eq!(fast_update.entities[0].energy_need, STANDARD_ACTION_COST);

    let mut slow = Game::new(42);
    slow.entities[0].speed = 100;
    let first = slow
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("first slow scheduler case should execute");
    assert_eq!(first.entities[0].position, Position { x: 8, y: 5 });
    assert_eq!(first.entities[0].energy_need, 50);
    let second = slow
        .dispatch(command(2, 1, GameCommand::Wait))
        .expect("second slow scheduler case should execute");
    assert_eq!(second.entities[0].position, Position { x: 7, y: 4 });
    assert_eq!(second.entities[0].energy_need, STANDARD_ACTION_COST);
}

#[test]
fn multiple_monsters_use_stable_id_order_when_paths_compete() {
    let mut left = Game::new(42);
    let mut second = left.entities[0].clone();
    second.id = "demo.monster.ember-mote.0".to_owned();
    second.position = Position { x: 8, y: 6 };
    left.entities.push(second);

    let mut right = left.clone();
    right.entities.reverse();

    let left_update = left
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("left scheduler should execute");
    let right_update = right
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("right scheduler should execute");

    assert_eq!(left_update.entities, right_update.entities);
    assert_eq!(left_update.changed_cells, right_update.changed_cells);
    assert_eq!(left_update.state_hash, right_update.state_hash);
    assert_ne!(
        left_update.entities[0].position,
        left_update.entities[1].position
    );
}

#[test]
fn player_death_stops_the_remaining_monster_queue_immediately() {
    let mut game = Game::new(0);
    game.entities[0].id = "demo.monster.ember-mote.0".to_owned();
    game.entities[0].position = Position { x: 4, y: 3 };
    let mut second = game.entities[0].clone();
    second.id = "demo.monster.ember-mote.1".to_owned();
    second.position = Position { x: 4, y: 4 };
    game.entities.push(second);
    game.player.hp = 0;

    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("fatal scheduler case should execute");

    assert!(update.player.is_dead);
    assert_eq!(
        update
            .events
            .iter()
            .filter(|event| event.message_key == "combat-player-death")
            .count(),
        1
    );
    let second = update
        .entities
        .iter()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("second monster should remain present");
    assert_eq!(second.energy_need, 10);
}
