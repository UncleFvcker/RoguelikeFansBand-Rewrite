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
