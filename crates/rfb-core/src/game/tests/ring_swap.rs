// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn game() -> Game {
    let mut game = Game::new_with_build(606, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    game
}

fn wear(game: &mut Game, id: &str, slot: &str) {
    give_inventory_item(game, id, "demo.item.ring");
    game.equip_inventory_item(id, Some(slot)).unwrap();
}

fn swap(first: &str, second: &str) -> GameCommand {
    GameCommand::SwapRings {
        first_slot_id: first.into(),
        second_slot_id: second.into(),
    }
}

#[test]
fn ring_swap_is_atomic_free_works_with_full_pack_and_replays_after_save() {
    let mut game = game();
    wear(&mut game, "first", "right-ring");
    wear(&mut game, "second", "left-ring");
    game.items[0].inscription = Some("keep this ring".into());
    for i in 0..game.inventory_slot_capacity() {
        give_inventory_item(&mut game, &format!("full.{i}"), "demo.item.dagger");
    }
    let before_items = game.items.clone();
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    let command = swap("right-ring", "left-ring");
    let update = dispatch_next(&mut game, command.clone());
    assert_eq!(update, dispatch_next(&mut restored, command.clone()));
    assert!(update.events.iter().any(|e| e.kind == "item.rings-swapped"));
    assert!(
        !update.command_repeatable,
        "a count never spins a free swap"
    );
    assert_eq!(
        game.items[0].location,
        ItemLocation::Equipped {
            slot_id: "left-ring".into()
        }
    );
    assert_eq!(
        game.items[1].location,
        ItemLocation::Equipped {
            slot_id: "right-ring".into()
        }
    );
    dispatch_next(&mut game, command);
    assert_eq!(game.items, before_items);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
}

#[test]
fn single_ring_moves_to_empty_finger_and_combat_bonus_follows_the_weapon_hand() {
    let mut game = game();
    wear(&mut game, "first", "right-ring");
    game.items[0].enchantments.to_hit = 10;
    give_inventory_item(&mut game, "weapon", "demo.item.dagger");
    game.equip_inventory_item("weapon", None).unwrap();
    give_inventory_item(&mut game, "shield", "demo.item.small-metal-shield");
    game.equip_inventory_item("shield", None).unwrap();
    let before = game.player_melee_profiles(&game.player_derived_stats())[0].to_hit;
    dispatch_next(&mut game, swap("right-ring", "left-ring"));
    let after = game.player_melee_profiles(&game.player_derived_stats())[0].to_hit;
    assert_eq!(before - after, 10);
    assert_eq!(game.items.iter().filter(|item| matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "left-ring")).count(), 1);
    assert!(!game.items.iter().any(
        |item| matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-ring")
    ));
}

#[test]
fn either_cursed_ring_blocks_the_entire_swap_without_breaking_or_rolling() {
    for index in [0, 1] {
        for severity in [
            ItemCurseSeverityDto::Normal,
            ItemCurseSeverityDto::Heavy,
            ItemCurseSeverityDto::Permanent,
        ] {
            let mut game = game();
            wear(&mut game, "first", "right-ring");
            wear(&mut game, "second", "left-ring");
            game.items[index].curse = Some(severity);
            let before_items = game.items.clone();
            let before = (game.turn, game.world_tick, game.rng.clone());
            let update = dispatch_next(&mut game, swap("right-ring", "left-ring"));
            assert!(
                update
                    .events
                    .iter()
                    .any(|e| e.kind == "item.ring-swap.cursed")
            );
            assert_eq!(game.items, before_items);
            assert!(game.item_property_knowledge[&game.items[index].id].known_curse);
            assert_eq!((game.turn, game.world_tick, game.rng.clone()), before);
        }
    }
}

#[test]
fn unavailable_slots_do_not_mutate_and_multiple_ring_bodies_swap_only_the_selected_pair() {
    let mut game = game();
    let update = dispatch_next(&mut game, swap("ring-1", "ring-2"));
    assert!(
        update
            .events
            .iter()
            .any(|e| e.kind == "item.ring-swap.unavailable")
    );
    game.body_slots = dragon_body_slots();
    wear(&mut game, "first", "ring-1");
    wear(&mut game, "second", "ring-6");
    let before = game.items.clone();
    for command in [
        swap("ring-1", "ring-1"),
        swap("ring-1", "head"),
        swap("ring-1", "missing"),
    ] {
        let update = dispatch_next(&mut game, command);
        assert!(
            update
                .events
                .iter()
                .any(|e| e.kind == "item.ring-swap.unavailable")
        );
        assert_eq!(game.items, before);
    }
    dispatch_next(&mut game, swap("ring-3", "ring-6"));
    assert_eq!(game.items[0], before[0]);
    assert_eq!(
        game.items[1].location,
        ItemLocation::Equipped {
            slot_id: "ring-3".into()
        }
    );
    assert_eq!(game.turn, 0);
}
