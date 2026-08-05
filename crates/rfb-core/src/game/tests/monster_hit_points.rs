// SPDX-License-Identifier: MPL-2.0
use crate::rng::RfbRng;

use super::*;

#[test]
fn normal_monster_hit_points_roll_each_die_once_at_birth() {
    let game = Game::new(1);
    let definition = game
        .content
        .actor("demo.actor.newt")
        .expect("Newt definition");
    let mut rng = RfbRng::seeded(2);

    let max_hp = actor_spawn_max_hp(&mut rng, definition);

    assert!((2..=12).contains(&max_hp));
    assert_eq!(rng.draw_counter, 2);
}

#[test]
fn force_maximum_hit_points_use_the_full_product_without_rng() {
    let game = Game::new(3);
    let definition = game
        .content
        .actor("demo.actor.warrens-keeper")
        .expect("Mughash definition");
    let mut rng = RfbRng::seeded(4);

    assert_eq!(actor_spawn_max_hp(&mut rng, definition), 150);
    assert_eq!(rng.draw_counter, 0);
}

#[test]
fn rolled_instance_hit_points_remain_authoritative_after_load() {
    let mut game = Game::new(5);
    let position = (0..game.height)
        .flat_map(|y| {
            (0..game.width).map(move |x| Position {
                x: i32::from(x),
                y: i32::from(y),
            })
        })
        .find(|position| {
            game.is_walkable(*position)
                && *position != game.player.position
                && game
                    .entities
                    .iter()
                    .all(|entity| entity.position != *position)
        })
        .expect("test actor position");
    game.push_generated_actor("test.rolled-hp".to_owned(), "demo.actor.newt", position);
    let rolled_max_hp = game.entities[0].max_hp;

    let restored = Game::from_save(game.to_save()).expect("rolled HP save should load");

    assert_eq!(restored.entities[0].max_hp, rolled_max_hp);
    assert_eq!(restored.entities[0].hp, rolled_max_hp);
}
