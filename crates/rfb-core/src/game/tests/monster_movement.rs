// SPDX-License-Identifier: MPL-2.0
use crate::game::movement::{actor_avoids_terrain_trap, actor_can_cross_terrain};

use super::support::replace_terrain;
use super::*;

#[test]
fn movement_profile_controls_non_walkable_terrain_entry() {
    let game = Game::new(1);
    let deep_water = game
        .content
        .terrain("demo.terrain.resonance-water-deep")
        .expect("deep water definition");
    let walker = game
        .content
        .actor("demo.actor.small-kobold")
        .expect("walking actor definition");
    let swimmer = game
        .content
        .actor("demo.actor.newt")
        .expect("swimming actor definition");
    let flyer = game
        .content
        .actor("demo.actor.fruit-bat")
        .expect("flying actor definition");

    assert!(!actor_can_cross_terrain(walker, deep_water));
    assert!(actor_can_cross_terrain(swimmer, deep_water));
    assert!(actor_can_cross_terrain(flyer, deep_water));
}

#[test]
fn trap_avoidance_requires_an_explicit_matching_movement_mode() {
    let game = Game::new(2);
    let mut trap = game
        .content
        .terrain("demo.terrain.trap-echo-snare")
        .expect("trap definition")
        .clone();
    trap.trap
        .as_mut()
        .expect("trap behavior")
        .avoided_by_movement_modes = vec![rfb_content::ActorMovementMode::Fly];
    let flyer = game
        .content
        .actor("demo.actor.fruit-bat")
        .expect("flying actor definition");
    let swimmer = game
        .content
        .actor("demo.actor.newt")
        .expect("swimming actor definition");

    assert!(actor_avoids_terrain_trap(flyer, &trap));
    assert!(!actor_avoids_terrain_trap(swimmer, &trap));
}

#[test]
fn entering_a_non_avoided_trap_applies_damage_to_the_monster() {
    let mut game = Game::new(3);
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let trap = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, trap, "demo.terrain.trap-echo-snare");
    game.push_generated_actor(
        "test.trap-actor".to_owned(),
        "demo.actor.small-kobold",
        origin,
    );
    game.entities[0].hp = 10;
    game.entities[0].max_hp = 10;
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, trap, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("trap movement should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, trap);
    assert_eq!(game.entities[0].hp, 8);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ActorTrapTriggered { .. }]
    ));
}
