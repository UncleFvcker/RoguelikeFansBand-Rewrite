// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{GoldAppearanceDto, GoldPileDto};

use super::support::*;
use super::*;
use crate::game::gold::MAX_PLAYER_GOLD;

#[test]
fn warrens_floor_gold_is_seeded_walkable_and_persistent() {
    for seed in 1..=16 {
        let mut left = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        let mut right = Game::new_with_build(seed, "demo.build.warrior")
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
fn seeing_gold_discovers_and_refreshes_its_cell() {
    let mut game = Game::new(42);
    game.entities.clear();
    game.items.clear();
    let position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.gold_piles = vec![GoldPile {
        id: "generated.gold.1".to_owned(),
        position,
        amount: 10,
        appearance: GoldAppearanceDto::Gold,
        discovered: false,
    }];

    let update = dispatch_next(&mut game, GameCommand::Wait);

    assert!(game.gold_piles[0].discovered);
    assert_eq!(update.gold_piles[0].id, "generated.gold.1");
    assert_eq!(
        update
            .changed_cells
            .iter()
            .find(|cell| cell.position == position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("generated.gold.1")
    );
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
        discovered: true,
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
        discovered: true,
    });
    stale_allocator.next_gold_pile_serial = 4;
    assert!(matches!(
        Game::from_save(stale_allocator),
        Err(CoreError::InvalidSave(
            "gold pile allocator is behind existing IDs"
        ))
    ));
}
