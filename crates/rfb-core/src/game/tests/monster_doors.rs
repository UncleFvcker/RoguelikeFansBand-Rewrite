// SPDX-License-Identifier: MPL-2.0
use crate::rng::RfbRng;

use super::support::*;
use super::*;

fn door_game(actor_kind_id: &str, door_kind_id: &str) -> (Game, Position, Position) {
    let mut game = Game::new(1);
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let door = Position { x: 5, y: 3 };
    game.player.position = Position { x: 2, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, door, door_kind_id);
    game.push_generated_actor("test.door-actor".to_owned(), actor_kind_id, origin);
    (game, origin, door)
}

#[test]
fn opening_an_ordinary_door_spends_the_action_without_moving() {
    let (mut game, origin, door) = door_game("demo.actor.small-kobold", "demo.terrain.door-closed");
    let draws = game.rng.draw_counter;
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, door, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("door action should resolve");

    assert_eq!(outcome, ActorStepOutcome::Interacted);
    assert_eq!(game.entities[0].position, origin);
    assert_eq!(game.terrain_at(door), "demo.terrain.door-open");
    assert_eq!(game.rng.draw_counter, draws);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::DoorOpened { .. }]
    ));
}

#[test]
fn a_successful_bash_moves_the_monster_into_the_doorway() {
    let (mut game, _, door) = door_game("demo.actor.warg", "demo.terrain.door-closed");
    game.entities[0].hp = 60;
    game.entities[0].max_hp = 60;
    let seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(6) > 0)
        .expect("a successful bash seed should exist");
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, door, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("bash action should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, door);
    assert!(matches!(
        game.terrain_at(door),
        "demo.terrain.door-open" | "demo.terrain.door-broken"
    ));
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::DoorBashedOpen { .. }]
    ));
}
