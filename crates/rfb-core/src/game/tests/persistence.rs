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

#[test]
fn generated_artifact_state_round_trips_and_changes_the_state_hash() {
    let mut game = Game::new(42);
    let before = game.state_hash();
    game.generated_artifact_ids
        .insert("demo.item.crisdurian".to_owned());
    let after = game.state_hash();
    assert_ne!(after, before);

    let payload = game.to_save();
    assert_eq!(payload.generated_artifact_ids, ["demo.item.crisdurian"]);
    let restored = Game::from_save(payload).expect("artifact state should restore");
    assert_eq!(restored.state_hash(), after);
    assert!(
        restored
            .generated_artifact_ids
            .contains("demo.item.crisdurian")
    );
}

#[test]
fn malformed_generated_artifact_state_is_rejected() {
    let base = Game::new(42).to_save();
    for ids in [
        vec!["demo.item.crisdurian", "demo.item.crisdurian"],
        vec!["test.item.unknown"],
        vec!["demo.item.relic-blade"],
    ] {
        let mut payload = base.clone();
        payload.generated_artifact_ids = ids.into_iter().map(str::to_owned).collect();
        assert!(matches!(
            Game::from_save(payload),
            Err(CoreError::InvalidSave(
                "generated artifact state is invalid"
            ))
        ));
    }
}

#[test]
fn fixed_artifact_instance_requires_its_generation_record() {
    let mut game = Game::new(42);
    let context = LootContext {
        table_id: "demo.loot-table.paladin".to_owned(),
        floor_id: "test.floor.depth-60".to_owned(),
        depth: 60,
        source: LootSource::ItemUse {
            item_id: "test.item-generation".to_owned(),
        },
    };
    let draft = game.fixed_item_draft(&context, "demo.item.crisdurian".to_owned());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .expect("fixed artifact should commit");
    game.items.push(item);
    let mut payload = game.to_save();
    payload.generated_artifact_ids.clear();

    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave(
            "generated artifact state is invalid"
        ))
    ));
}
