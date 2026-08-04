// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn exploration_memory_does_not_change_authoritative_state_hash() {
    let mut game = Game::new(42);
    let before = game.state_hash();
    game.explored.fill(true);
    assert_eq!(game.state_hash(), before);

    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should store the entrance floor");
    let before_stored_memory_change = game.state_hash();
    game.stored_floors
        .get_mut("demo.floor.surface")
        .expect("the entrance floor should be stored")
        .explored
        .fill(false);
    assert_eq!(game.state_hash(), before_stored_memory_change);
}

#[test]
fn content_hash_is_reported_but_not_part_of_authoritative_state_hash() {
    let artifact = rfb_content::decode_content(BUILT_IN_CONTENT_BYTES)
        .expect("built-in content artifact should decode");
    let mut different_hash = artifact.clone();
    different_hash.content_hash = "different-content-hash".to_owned();
    let left = Game::from_content(
        42,
        Arc::new(ContentCatalog::from_artifact(artifact)),
        BUILT_IN_WORLD_ID,
    )
    .expect("built-in content should create a game");
    let right = Game::from_content(
        42,
        Arc::new(ContentCatalog::from_artifact(different_hash)),
        BUILT_IN_WORLD_ID,
    )
    .expect("equivalent content should create a game");

    assert_ne!(left.content_hash(), right.content_hash());
    assert_eq!(left.state_hash(), right.state_hash());
}

#[test]
fn malformed_exploration_memory_is_rejected() {
    let mut payload = Game::new(42).to_save();
    payload.explored.pop();
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave(
            "exploration memory dimensions are invalid"
        ))
    ));
}

#[test]
fn malformed_revealed_terrain_knowledge_is_rejected() {
    let mut payload = Game::new(42).to_save();
    payload.revealed_terrain = vec![Position { x: 3, y: 3 }];
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave(
            "revealed terrain knowledge is invalid"
        ))
    ));
}

#[test]
fn save_with_different_content_hash_is_rejected() {
    let mut payload = Game::new(42).to_save();
    payload.content_hash = "obsolete-content-hash".to_owned();

    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::ContentMismatch)
    ));
}

#[test]
fn inventory_over_its_equipped_slot_capacity_is_rejected() {
    let mut game = Game::new(42);
    game.items.clear();
    for index in 0..27 {
        give_inventory_item(
            &mut game,
            &format!("test.inventory.item.{index}"),
            "demo.item.resonant-band",
        );
    }

    assert!(matches!(
        Game::from_save(game.to_save()),
        Err(CoreError::InvalidSave("inventory exceeds slot capacity"))
    ));
}

#[test]
fn save_payload_restores_identical_state() {
    let mut game = Game::new(7);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
            slot_id: None,
        },
    ))
    .expect("equip should execute");

    let restored = Game::from_save(game.to_save()).expect("save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.snapshot().equipment.len(), 1);
}
