// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::inventory::MAX_BOOK_FOUND_COUNT;

const BOOK: &str = "demo.item.black-prayers";

fn found(game: &Game) -> u32 {
    game.item_knowledge
        .get(BOOK)
        .map_or(0, |state| state.found_count)
}

fn generate_book(game: &mut Game, location: ItemLocation) -> String {
    let context = LootContext {
        table_id: "demo.loot-table.paladin".to_owned(),
        floor_id: game.current_floor_id.clone(),
        depth: 1,
        source: LootSource::ItemUse {
            item_id: "test.generation".to_owned(),
        },
    };
    let draft = game.fixed_item_draft(&context, BOOK.to_owned());
    let item = game.commit_generated_item_draft(draft, location).unwrap();
    let id = item.id.clone();
    game.items.push(item);
    id
}

#[test]
fn birth_books_count_once_and_other_starting_items_do_not_acquire_counters() {
    let mut game = Game::new_with_build(401, "demo.build.high-mage-death").unwrap();
    let books: Vec<_> = game
        .items
        .iter()
        .filter(|item| {
            item.location == ItemLocation::Inventory
                && game
                    .content
                    .item(&item.kind_id)
                    .unwrap()
                    .ability_book_id
                    .is_some()
        })
        .map(|item| (item.id.clone(), item.kind_id.clone()))
        .collect();
    assert!(!books.is_empty());
    for (id, kind) in books {
        assert!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .book_counted
        );
        assert_eq!(game.item_knowledge[&kind].found_count, 1);
        game.drop_inventory_quantity(&id, 1).unwrap().unwrap();
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert_eq!(game.item_knowledge[&kind].found_count, 1);
    }
    assert!(
        game.item_knowledge
            .iter()
            .all(|(id, state)| state.found_count == 0
                || game.content.item(id).unwrap().ability_book_id.is_some())
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn cumulative_discoveries_survive_drops_destruction_and_save_beyond_allocator_thresholds() {
    let mut game = Game::new_with_build(402, "demo.build.warrior").unwrap();
    for expected in 1..=11 {
        let position = game.player.position;
        let id = generate_book(&mut game, ItemLocation::Ground(position));
        assert_eq!(
            found(&game),
            expected - 1,
            "ground generation is not discovery"
        );
        game.pick_up_item_at_player(Some(&id)).unwrap();
        assert_eq!(found(&game), expected);
        game.drop_inventory_quantity(&id, 1).unwrap().unwrap();
        game.reveal_current_visibility();
        let save = game.to_save();
        assert!(
            save.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .book_counted
        );
        let mut restored = Game::from_save(save).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        restored.pick_up_item_at_player(Some(&id)).unwrap();
        restored.destroy_item(&id, 1).unwrap();
        assert_eq!(found(&restored), expected);
        game = restored;
    }
    assert!(!game.items.iter().any(|item| item.kind_id == BOOK));
}

#[test]
fn identification_and_player_destruction_count_unpicked_books_once() {
    let mut game = Game::new_with_build(403, "demo.build.warrior").unwrap();
    let position = game.player.position;
    let id = generate_book(&mut game, ItemLocation::Ground(position));
    let before = game.state_hash();
    assert!(game.destroy_item(&id, 2).is_err());
    assert_eq!(game.state_hash(), before);
    game.identify_item_instance(&id, ItemIdentificationRequest::new(false));
    assert_eq!(found(&game), 1);
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.destroy_item(&id, 1).unwrap();
    assert_eq!(found(&game), 1);
    let id = generate_book(&mut game, ItemLocation::Ground(position));
    game.destroy_item(&id, 1).unwrap();
    assert_eq!(found(&game), 2);
    let id = generate_book(&mut game, ItemLocation::Inventory);
    assert_eq!(found(&game), 3);
    game.destroy_item(&id, 1).unwrap();
    assert_eq!(found(&game), 3);
}

#[test]
fn leaving_and_restoring_a_floor_preserves_counted_and_unseen_books() {
    let mut game = Game::new_with_build(409, "demo.build.warrior").unwrap();
    let position = game.player.position;
    let counted = generate_book(&mut game, ItemLocation::Inventory);
    game.drop_inventory_quantity(&counted, 1).unwrap().unwrap();
    let unseen = generate_book(&mut game, ItemLocation::Ground(position));
    support::dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    assert_eq!(found(&game), 1);
    assert!(
        game.stored_floors
            .values()
            .flat_map(|floor| &floor.items)
            .any(|item| item.id == counted && item.book_counted)
    );
    let save = game.to_save();
    let mut invalid = save.clone();
    invalid
        .stored_floors
        .iter_mut()
        .flat_map(|floor| &mut floor.items)
        .find(|item| item.id == counted)
        .unwrap()
        .quantity = 2;
    assert!(Game::from_save(invalid).is_err());
    let restored = Game::from_save(save).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    game = restored;
    support::dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = position;
    game.pick_up_item_at_player(Some(&counted)).unwrap();
    assert_eq!(found(&game), 1);
    game.pick_up_item_at_player(Some(&unseen)).unwrap();
    assert_eq!(found(&game), 2);
}

#[test]
fn rejected_pickup_leaves_discovery_and_rng_unchanged() {
    let mut game = Game::new_with_build(404, "demo.build.warrior").unwrap();
    game.items
        .retain(|item| item.location != ItemLocation::Inventory);
    let capacity = game.snapshot().player.inventory_slot_capacity;
    for _ in 0..capacity {
        generate_book(&mut game, ItemLocation::Inventory);
    }
    let position = game.player.position;
    let id = generate_book(&mut game, ItemLocation::Ground(position));
    let before = game.state_hash();
    assert!(matches!(
        game.pick_up_item_at_player(Some(&id)).unwrap(),
        PickUpOutcome::InventoryFull { .. }
    ));
    assert_eq!(game.state_hash(), before);
    assert!(
        !game
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .book_counted
    );
}

#[test]
fn discovery_save_rejects_invalid_kinds_counts_and_instance_marks() {
    let mut game = Game::new_with_build(405, "demo.build.warrior").unwrap();
    let id = generate_book(&mut game, ItemLocation::Inventory);
    let save = game.to_save();
    for kind in ["missing.book", "demo.item.dagger"] {
        let mut invalid = save.clone();
        invalid
            .item_knowledge
            .iter_mut()
            .find(|entry| entry.kind_id == BOOK)
            .unwrap()
            .kind_id = kind.to_owned();
        assert!(Game::from_save(invalid).is_err());
    }
    let mut invalid = save.clone();
    invalid
        .item_knowledge
        .iter_mut()
        .find(|entry| entry.kind_id == BOOK)
        .unwrap()
        .found_count = MAX_BOOK_FOUND_COUNT + 1;
    assert!(Game::from_save(invalid).is_err());
    let mut invalid = save.clone();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .quantity = 2;
    assert!(Game::from_save(invalid).is_err());
    let mut invalid = save.clone();
    invalid.equipment[0].book_counted = true;
    assert!(Game::from_save(invalid).is_err());
    let mut changed = save.clone();
    changed
        .inventory
        .iter_mut()
        .find(|item| item.id == id)
        .unwrap()
        .book_counted = false;
    assert_ne!(
        Game::from_save(changed).unwrap().state_hash(),
        game.state_hash()
    );
    let mut saturated = save;
    saturated
        .item_knowledge
        .iter_mut()
        .find(|entry| entry.kind_id == BOOK)
        .unwrap()
        .found_count = MAX_BOOK_FOUND_COUNT;
    let mut restored = Game::from_save(saturated).unwrap();
    assert_ne!(restored.state_hash(), game.state_hash());
    let id = generate_book(&mut restored, ItemLocation::Inventory);
    assert_eq!(found(&restored), MAX_BOOK_FOUND_COUNT);
    assert!(
        restored
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .book_counted
    );
    Game::from_save(restored.to_save()).unwrap();
}
