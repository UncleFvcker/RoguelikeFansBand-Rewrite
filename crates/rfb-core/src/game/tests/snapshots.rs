// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn built_in_game_is_created_from_the_compiled_content_pack() {
    let game = Game::new(42);
    let expected_content_hash = game.content_hash().to_owned();
    let snapshot = game.snapshot();
    let shard = snapshot
        .items
        .iter()
        .find(|item| item.id == "demo.item.luminous-shard.1")
        .expect("compiled world should spawn its item");

    assert_eq!(snapshot.content_id, "rfb.demo.original-v1");
    assert_eq!(snapshot.content_hash, expected_content_hash);
    assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
    assert_eq!(
        snapshot.player.melee_damage.damage_type,
        DamageTypeDto::Physical
    );
    assert_eq!(
        snapshot.entities[0].melee_damage.damage_type,
        DamageTypeDto::Fire
    );
    assert_eq!(snapshot.player.id, "demo.actor.player.1");
    assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
    assert_eq!(snapshot.player.base_attack, 2);
    assert_eq!(snapshot.player.attack, 2);
    assert_eq!(snapshot.player.base_defense, 1);
    assert_eq!(snapshot.player.defense, 1);
    assert!(snapshot.inventory.is_empty());
    assert!(snapshot.equipment.is_empty());
    assert_eq!(snapshot.items.len(), 5);
    assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
    assert_eq!(snapshot.entities[0].attack, 1);
    assert_eq!(snapshot.entities[0].defense, 1);
    assert_eq!(shard.position, Position { x: 4, y: 3 });
    assert_eq!(
        snapshot
            .cells
            .iter()
            .find(|cell| cell.position == shard.position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("demo.item.luminous-shard.1")
    );
    assert!(
        snapshot
            .content_visuals
            .iter()
            .any(|visual| visual.id == "demo.item.luminous-shard" && visual.glyph == "!")
    );
    assert_eq!(snapshot.visual_cells.len(), snapshot.cells.len());
    assert_eq!(
        visual_at(&snapshot, snapshot.player.position).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 19, y: 19 }).visibility,
        VisibilityState::Hidden
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 8, y: 5 }).light.color,
        ACTOR_LIGHT_COLOR
    );
}

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
