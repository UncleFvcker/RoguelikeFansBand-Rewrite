// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn arena(terrain: &str, spikes: u32) -> Game {
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
    replace_terrain(&mut game, Position { x: 96, y: 32 }, terrain);
    if spikes > 0 {
        give_inventory_item(&mut game, "test.spike", "demo.item.iron-spike");
        game.items.last_mut().unwrap().quantity = spikes;
    }
    game.reveal_current_visibility();
    game
}

fn spike(game: &mut Game) -> GameUpdate {
    dispatch_next(
        game,
        GameCommand::SpikeDoor {
            direction: Direction::East,
        },
    )
}

#[test]
fn glass_stays_transparent_and_discovered_secret_doors_can_be_spiked() {
    let mut game = arena("demo.terrain.glass-door-closed", 8);
    let target = game.position_in_direction(Direction::East);
    for step in 1..=8 {
        spike(&mut game);
        assert_eq!(
            game.terrain_at(target),
            format!("demo.terrain.glass-door-jammed-{}", step.min(7))
        );
        let terrain = game.content.terrain(game.terrain_at(target)).unwrap();
        assert!(!terrain.blocks_sight);
        assert!(!terrain.walkable);
        assert_eq!(terrain.monster_door_power, Some(step.min(7)));
        assert!(game.open_door(Direction::East).is_none());
    }
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(game.state_hash(), restored.state_hash());
    let mut game = arena("demo.terrain.door-secret", 1);
    game.revealed_terrain.insert(target);
    spike(&mut game);
    assert_eq!(game.terrain_at(target), "demo.terrain.door-jammed-1");
    assert!(game.items.is_empty());
}

#[test]
fn projected_store_purchase_supplies_spikes_that_can_be_used_after_restore() {
    let mut game = arena("demo.terrain.door-closed", 0);
    game.player.position = Position { x: 70, y: 39 };
    game.gold = 1000;
    game.mark_shop_visited_at_player().unwrap();
    let snapshot = game.snapshot();
    let shop = snapshot
        .shops
        .iter()
        .find(|shop| shop.id == "demo.shop.outpost-general-store")
        .unwrap();
    let stock = shop
        .stock
        .iter()
        .find(|item| item.kind_id == "demo.item.iron-spike")
        .unwrap();
    assert!((6..=42).contains(&stock.quantity));
    dispatch_next(
        &mut game,
        GameCommand::BuyFromShop {
            shop_id: shop.id.clone(),
            item_id: stock.id.clone(),
            quantity: 2,
        },
    );
    assert_eq!(game.gold, 1000 - 2 * stock.unit_price);
    game.player.position = Position { x: 95, y: 32 };
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    let update = spike(&mut game);
    let resumed = spike(&mut restored);
    assert_eq!(update.events, resumed.events);
    assert_eq!(game.state_hash(), restored.state_hash());
    let bought = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.iron-spike")
        .unwrap();
    assert_eq!(bought.quantity, 1);
    assert_eq!(
        game.terrain_at(Position { x: 96, y: 32 }),
        "demo.terrain.door-jammed-1"
    );
}

#[test]
fn spikes_strengthen_and_consume_one_even_at_the_cap_then_survive_save() {
    let mut game = arena("demo.terrain.door-closed", 8);
    let origin = game.player.position;
    let target = game.position_in_direction(Direction::East);
    let rng = game.rng.clone();
    for step in 1..=8 {
        let before_tick = game.world_tick;
        let update = spike(&mut game);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-spiked")
        );
        assert_eq!(
            game.terrain_at(target),
            format!("demo.terrain.door-jammed-{}", step.min(7))
        );
        assert_eq!(
            game.items.iter().map(|item| item.quantity).sum::<u32>(),
            8 - step
        );
        assert_eq!(game.turn, step);
        assert!(game.world_tick > before_tick);
        assert_eq!(game.player.position, origin);
        assert_eq!(game.rng, rng);
        assert!(game.open_door(Direction::East).is_none());
    }
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(spike(&mut game), spike(&mut restored));
    assert_eq!(game.turn, 8, "no spikes means no additional turn");
}

#[test]
fn unsuitable_or_concealed_terrain_and_missing_inventory_spikes_cost_no_time() {
    for (terrain, quantity, event) in [
        ("demo.terrain.floor", 1, "terrain.door-spike-unavailable"),
        (
            "demo.terrain.door-open",
            1,
            "terrain.door-spike-unavailable",
        ),
        (
            "demo.terrain.door-secret",
            1,
            "terrain.door-spike-unavailable",
        ),
        ("demo.terrain.door-closed", 0, "terrain.door-spike-missing"),
    ] {
        let mut game = arena(terrain, quantity);
        let target = game.position_in_direction(Direction::East);
        game.revealed_terrain.remove(&target);
        give_inventory_item(&mut game, "test.unrelated", "demo.item.ration-of-food");
        give_inventory_item(&mut game, "test.ground-spike", "demo.item.iron-spike");
        game.items.last_mut().unwrap().location = ItemLocation::Ground(target);
        let (items, tick, energy, rng) = (
            game.items.clone(),
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
        );
        let update = spike(&mut game);
        assert!(
            update.events.iter().any(|actual| actual.kind == event),
            "{terrain}"
        );
        assert_eq!(game.turn, 0);
        assert_eq!(game.world_tick, tick);
        assert_eq!(game.player.energy_need, energy);
        assert_eq!(game.rng, rng);
        assert_eq!(game.items, items);
        assert_eq!(game.terrain_at(target), terrain);
    }
}

#[test]
fn spike_checks_door_before_actor_and_attacks_before_requiring_spikes() {
    for terrain in ["demo.terrain.floor", "demo.terrain.door-closed"] {
        for quantity in [0, 1] {
            let mut game = arena(terrain, quantity);
            let target = game.position_in_direction(Direction::East);
            let origin = game.player.position;
            game.push_generated_actor(
                "test.spike.actor".into(),
                "demo.actor.clear-icky-thing",
                target,
            );
            game.entities[0].visible_invisible = false;
            let update = spike(&mut game);
            let is_door = terrain == "demo.terrain.door-closed";
            assert_eq!(
                update
                    .events
                    .iter()
                    .any(|event| event.kind == "terrain.door-spike-monster-blocked"),
                is_door
            );
            assert_eq!(
                update
                    .events
                    .iter()
                    .any(|event| event.kind.starts_with("combat.")),
                is_door
            );
            assert_eq!(game.turn, u32::from(is_door));
            assert_eq!(game.player.position, origin);
            assert_eq!(game.terrain_at(target), terrain);
            assert_eq!(
                game.items
                    .iter()
                    .filter(|item| item.id == "test.spike")
                    .map(|item| item.quantity)
                    .sum::<u32>(),
                quantity
            );
        }
    }
}

#[test]
fn spike_confuses_once_and_paralysis_or_world_map_prevents_the_operation() {
    let mut game = arena("demo.terrain.door-closed", 2);
    for direction in TERRAIN_INTERACTION_DIRECTIONS {
        let target = game.position_in_direction(direction);
        replace_terrain(&mut game, target, "demo.terrain.door-closed");
    }
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 100, "test.confusion").status);
    let mut expected = game.clone();
    let direction = expected.confused_direction(Direction::East, &mut vec![]);
    expected.spike_door(direction);
    spike(&mut game);
    assert_eq!(game.terrain, expected.terrain);
    assert_eq!(game.items, expected.items);
    assert_eq!(game.rng, expected.rng);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_PARALYSIS, 100, "test.paralysis").status);
    let (terrain, items, rng) = (game.terrain.clone(), game.items.clone(), game.rng.clone());
    spike(&mut game);
    assert_eq!(game.turn, 2);
    assert_eq!(game.terrain, terrain);
    assert_eq!(game.items, items);
    assert_eq!(game.rng, rng);
    game.map_scale = MapScaleDto::World;
    let before = game.to_save();
    assert!(
        game.dispatch(GameCommandEnvelope {
            command_seq: game.last_command_seq + 1,
            expected_revision: game.revision,
            command: GameCommand::SpikeDoor {
                direction: Direction::East
            }
        })
        .is_err()
    );
    assert_eq!(game.to_save(), before);
}
