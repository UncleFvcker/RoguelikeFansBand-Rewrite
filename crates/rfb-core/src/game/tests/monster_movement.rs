// SPDX-License-Identifier: MPL-2.0
use crate::game::movement::{actor_avoids_terrain_trap, actor_can_cross_terrain};

use super::support::{game_with_actor_definition, replace_terrain};
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

#[test]
fn wall_destroyer_transforms_the_wall_and_enters_the_cell() {
    let mut game = game_with_actor_definition(4, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_walls = true;
    });
    game.entities.clear();
    let origin = Position { x: 4, y: 3 };
    let wall = Position { x: 5, y: 3 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    game.push_generated_actor(
        "test.wall-destroyer".to_owned(),
        "demo.actor.echo-hound",
        origin,
    );
    let mut events = Vec::new();

    let outcome = game
        .move_entity(0, wall, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("wall destruction should resolve");

    assert_eq!(outcome, ActorStepOutcome::Moved);
    assert_eq!(game.entities[0].position, wall);
    assert_eq!(game.terrain_at(wall), "demo.terrain.floor");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterTerrainDestroyed { position, .. }] if *position == wall
    ));
}

#[test]
fn item_destroyer_removes_an_ordinary_ground_item() {
    let mut game = game_with_actor_definition(5, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_items = true;
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let position = game.entities[0].position;
    game.items.push(ItemInstance {
        id: "test.destroyed-item".to_owned(),
        kind_id: "demo.item.echo-charm".to_owned(),
        quantity: 2,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(position),
    });
    let mut events = Vec::new();

    game.destroy_items_under_monster(0, position, &mut events, &mut BTreeSet::new());

    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.destroyed-item")
    );
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterItemDestroyed { target_kind_id, quantity: 2, .. }]
            if target_kind_id == "demo.item.echo-charm"
    ));
}

#[test]
fn item_destroyer_removes_a_ground_gold_pile() {
    let mut game = game_with_actor_definition(6, "demo.actor.echo-hound", |actor| {
        actor.terrain_interaction.destroys_items = true;
    });
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let position = game.entities[0].position;
    game.gold_piles.push(GoldPile {
        id: "test.destroyed-gold".to_owned(),
        position,
        amount: 17,
        appearance: GoldAppearanceDto::Silver,
    });
    let mut events = Vec::new();

    game.destroy_items_under_monster(0, position, &mut events, &mut BTreeSet::new());

    assert!(game.gold_piles.is_empty());
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::MonsterItemDestroyed { target_kind_id, quantity: 17, .. }]
            if target_kind_id == "core.gold.silver"
    ));
}
