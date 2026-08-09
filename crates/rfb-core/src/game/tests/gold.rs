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
        inscription: None,
        origin_actor_kind_id: None,
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
