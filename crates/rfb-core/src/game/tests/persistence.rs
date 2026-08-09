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
        DEFAULT_WORLD_ID,
    )
    .expect("built-in content should create a game");
    let right = Game::from_content(
        42,
        Arc::new(ContentCatalog::from_artifact(different_hash)),
        DEFAULT_WORLD_ID,
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
