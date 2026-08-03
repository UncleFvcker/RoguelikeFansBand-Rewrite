// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{GoldAppearanceDto, GoldPileDto};

use super::support::*;
use super::*;
use crate::game::gold::MAX_PLAYER_GOLD;

#[test]
fn warrens_floor_gold_is_seeded_walkable_and_persistent() {
    for seed in 1..=16 {
        let mut left = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        let mut right = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
            .expect("same Warrens journey should create");
        descend_one_floor(&mut left);
        descend_one_floor(&mut right);

        assert!(!left.gold_piles.is_empty());
        assert_eq!(left.gold_piles, right.gold_piles);
        assert!(left.gold_piles.iter().all(|pile| {
            pile.amount > 0
                && pile.id.starts_with("generated.gold.")
                && left.is_walkable(pile.position)
        }));

        let restored = Game::from_save(left.to_save()).expect("floor gold should reload");
        assert_eq!(restored.gold_piles, left.gold_piles);
        assert_eq!(restored.state_hash(), left.state_hash());
    }
}

#[test]
fn pickup_collects_all_gold_before_item_and_gold_has_no_weight() {
    let mut game = Game::new(42);
    game.entities.clear();
    game.items.clear();
    game.gold = MAX_PLAYER_GOLD - 5;
    game.gold_piles = vec![
        GoldPile {
            id: "generated.gold.2".to_owned(),
            position: game.player.position,
            amount: 10,
            appearance: GoldAppearanceDto::Silver,
        },
        GoldPile {
            id: "generated.gold.1".to_owned(),
            position: game.player.position,
            amount: 4,
            appearance: GoldAppearanceDto::Copper,
        },
    ];
    game.next_gold_pile_serial = 3;
    game.items.push(ItemInstance {
        id: "test.gold-pickup.item".to_owned(),
        kind_id: "demo.item.echo-charm".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(game.player.position),
    });
    let expected_item_weight = game
        .content
        .item("demo.item.echo-charm")
        .expect("test item should exist")
        .weight_tenths_pound;

    let update = dispatch_next(&mut game, GameCommand::PickUp);
    let event_kinds = update
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    let gold_event = event_kinds
        .iter()
        .position(|kind| *kind == "gold.pickup")
        .expect("pickup should report gold");
    let item_event = event_kinds
        .iter()
        .position(|kind| *kind == "item.pickup")
        .expect("pickup should report the item");

    assert!(gold_event < item_event);
    assert_eq!(update.events[gold_event].args["amount"], "5");
    assert_eq!(game.gold, MAX_PLAYER_GOLD);
    assert!(game.gold_piles.is_empty());
    assert_eq!(
        game.carried_weight_tenths_pound(),
        u32::from(expected_item_weight)
    );
    assert_eq!(update.player.gold, MAX_PLAYER_GOLD);
}

#[test]
fn v156_save_migrates_without_backfilling_gold_or_rng() {
    let mut game = Game::new_warrens_journey_with_build(17, "demo.build.warrior")
        .expect("Warrens journey should create");
    descend_one_floor(&mut game);
    let mut payload = game.to_save();
    payload.content_hash =
        "91ac518116420421305410a9435e002648c5538deba102780ce5e1359d7e33be".to_owned();
    payload.player.gold = 0;
    payload.gold_piles.clear();
    payload.next_gold_pile_serial = 0;
    for floor in &mut payload.stored_floors {
        floor.gold_piles.clear();
    }
    let draws_before = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v156 save should migrate");
    assert_eq!(restored.gold, 0);
    assert!(restored.gold_piles.is_empty());
    assert!(
        restored
            .stored_floors
            .values()
            .all(|floor| floor.gold_piles.is_empty())
    );
    assert_eq!(restored.next_gold_pile_serial, 1);
    assert_eq!(restored.rng.draw_counter, draws_before);
}

#[test]
fn invalid_gold_state_and_allocator_are_rejected() {
    let game = Game::new(42);
    let position = game.player.position;

    let mut excessive_wallet = game.to_save();
    excessive_wallet.player.gold = MAX_PLAYER_GOLD + 1;
    assert!(matches!(
        Game::from_save(excessive_wallet),
        Err(CoreError::InvalidSave("player gold balance is invalid"))
    ));

    let mut zero_pile = game.to_save();
    zero_pile.gold_piles.push(GoldPileDto {
        id: "generated.gold.1".to_owned(),
        position,
        amount: 0,
        appearance: GoldAppearanceDto::Copper,
    });
    zero_pile.next_gold_pile_serial = 2;
    assert!(matches!(
        Game::from_save(zero_pile),
        Err(CoreError::InvalidSave("gold pile state is invalid"))
    ));

    let mut stale_allocator = game.to_save();
    stale_allocator.gold_piles.push(GoldPileDto {
        id: "generated.gold.4".to_owned(),
        position,
        amount: 10,
        appearance: GoldAppearanceDto::Gold,
    });
    stale_allocator.next_gold_pile_serial = 4;
    assert!(matches!(
        Game::from_save(stale_allocator),
        Err(CoreError::InvalidSave(
            "gold pile allocator is behind existing IDs"
        ))
    ));
}
