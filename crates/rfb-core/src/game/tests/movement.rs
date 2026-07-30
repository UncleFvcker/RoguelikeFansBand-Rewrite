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
