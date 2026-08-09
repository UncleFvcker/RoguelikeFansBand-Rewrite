// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn ground_item_projection_requires_sight_or_detection_and_round_trips() {
    let mut game = Game::new(64);
    clear_monsters(&mut game);
    game.player.position = Position { x: 11, y: 11 };
    let visible_position = game.player.position;
    replace_terrain(&mut game, visible_position, "demo.terrain.floor");
    let hidden_position = Position { x: 19, y: 19 };
    replace_terrain(&mut game, hidden_position, "demo.terrain.floor");

    give_inventory_item(
        &mut game,
        "test.item.visible-discovery",
        "demo.item.ration-of-food",
    );
    give_inventory_item(
        &mut game,
        "test.item.hidden-discovery",
        "demo.item.ration-of-food",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.visible-discovery")
        .expect("visible test item")
        .location = ItemLocation::Ground(visible_position);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.hidden-discovery")
        .expect("hidden test item")
        .location = ItemLocation::Ground(hidden_position);
    game.reveal_current_visibility();

    let before_detection = game.snapshot();
    assert!(
        before_detection
            .items
            .iter()
            .any(|item| item.id == "test.item.visible-discovery")
    );
    assert!(
        before_detection
            .items
            .iter()
            .all(|item| item.id != "test.item.hidden-discovery")
    );
    assert_eq!(
        before_detection
            .cells
            .iter()
            .find(|cell| cell.position == hidden_position)
            .and_then(|cell| cell.item_id.as_deref()),
        None
    );
    game.item_property_knowledge
        .entry("test.item.hidden-discovery".to_owned())
        .or_default()
        .discovered = false;
    let undiscovered_hash = game.state_hash();
    game.item_property_knowledge
        .get_mut("test.item.hidden-discovery")
        .expect("hidden item knowledge")
        .discovered = true;
    assert_ne!(game.state_hash(), undiscovered_hash);
    game.item_property_knowledge
        .get_mut("test.item.hidden-discovery")
        .expect("hidden item knowledge")
        .discovered = false;

    give_inventory_item(
        &mut game,
        "test.item.seeking-scroll",
        "demo.item.seeking-scroll",
    );
    let detection = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.seeking-scroll".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        detection
            .changed_cells
            .iter()
            .find(|cell| cell.position == hidden_position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("test.item.hidden-discovery")
    );

    let after_detection = game.snapshot();
    assert!(
        after_detection
            .items
            .iter()
            .any(|item| item.id == "test.item.hidden-discovery")
    );
    assert_eq!(
        after_detection
            .cells
            .iter()
            .find(|cell| cell.position == hidden_position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("test.item.hidden-discovery")
    );

    let saved = game.to_save();
    assert!(saved.item_property_knowledge.iter().any(|knowledge| {
        knowledge.item_id == "test.item.hidden-discovery" && knowledge.discovered
    }));
    let restored = Game::from_save(saved).expect("discovered items should round-trip");
    assert!(
        restored
            .snapshot()
            .items
            .iter()
            .any(|item| item.id == "test.item.hidden-discovery")
    );
}
