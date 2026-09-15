// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn visual_identity_changes_only_when_item_knowledge_changes() {
    let mut game = Game::new(64);
    let kind = game
        .content
        .item_definitions()
        .find(|item| {
            item.appearance_name_key.is_some()
                && item.rfb_base_kind.is_some()
                && !item.tags.iter().any(|tag| tag == "artifact")
        })
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "visual.first", &kind);
    give_inventory_item(&mut game, "visual.second", &kind);
    game.item_knowledge.remove(&kind);
    game.reveal_current_visibility();
    let visual = game
        .inventory_item_dto(
            game.items
                .iter()
                .find(|item| item.id == "visual.first")
                .unwrap(),
        )
        .visual;
    assert!(visual.id.starts_with("core.appearance."));
    assert_eq!(
        visual,
        game.item_visual(
            game.items
                .iter()
                .find(|item| item.id == "visual.second")
                .unwrap()
        )
    );
    let before = game.state_hash();
    let catalog = game.player_dto().visual_catalog;
    assert!(catalog.iter().any(|entry| entry.id == visual.id));
    assert!(
        catalog
            .iter()
            .filter(|entry| entry.id.starts_with("core.appearance."))
            .all(|entry| entry.prf.is_none())
    );
    assert_eq!(game.state_hash(), before);
    game.mark_item_aware(&kind);
    assert_eq!(
        game.item_visual(
            game.items
                .iter()
                .find(|item| item.id == "visual.first")
                .unwrap()
        )
        .id,
        kind
    );
    let source = game.content.item(&kind).unwrap().rfb_base_kind.unwrap();
    let expected = format!("K:{}:{}", source.tval, source.sval);
    assert_eq!(
        game.player_dto()
            .visual_catalog
            .iter()
            .find(|entry| entry.id == kind)
            .unwrap()
            .prf
            .as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn visual_catalog_does_not_expose_an_unexamined_fixed_artifact() {
    let mut game = Game::new(64);
    let artifact = game
        .content
        .item_definitions()
        .find(|item| {
            item.tags.iter().any(|tag| tag == "artifact") && item.artifact_generation.is_some()
        })
        .unwrap();
    let kind = artifact.id.clone();
    let base = artifact
        .artifact_generation
        .as_ref()
        .unwrap()
        .base_item_kind_id
        .clone();
    give_inventory_item(&mut game, "visual.artifact", &kind);
    give_inventory_item(&mut game, "visual.base", &base);
    game.reveal_current_visibility();
    let item = game
        .items
        .iter()
        .find(|item| item.id == "visual.artifact")
        .unwrap();
    assert_eq!(
        game.item_visual(item),
        game.item_visual(
            game.items
                .iter()
                .find(|item| item.id == "visual.base")
                .unwrap()
        )
    );
    assert!(
        !game
            .player_dto()
            .visual_catalog
            .iter()
            .any(|entry| entry.id == kind)
    );
}

#[test]
fn display_projection_exposes_known_coverage_and_passage_without_changing_state() {
    let mut game = Game::new(64);
    let known = Position { x: 2, y: 2 };
    let hidden = Position { x: 3, y: 2 };
    replace_terrain(&mut game, known, "demo.terrain.floor");
    replace_terrain(&mut game, hidden, "demo.terrain.floor");
    let known_index = game.index(known).unwrap();
    let hidden_index = game.index(hidden).unwrap();
    game.explored[known_index] = true;
    game.explored[hidden_index] = false;
    game.detection_coverage.traps.clear();
    game.detection_coverage.traps.extend([known, hidden]);
    let before = game.state_hash();
    let player = game.player_dto();
    assert_eq!(player.trap_detected_grids, vec![known]);
    assert_eq!(
        player.speed_energy_per_tick,
        crate::scheduler::energy_gain(player.speed)
    );
    assert!(game.cell_dto(known).known_projectile_passage);
    assert!(!game.cell_dto(hidden).known_projectile_passage);
    assert_eq!(game.state_hash(), before);
    replace_terrain(&mut game, known, "demo.terrain.wall");
    assert!(!game.cell_dto(known).known_projectile_passage);
}

#[test]
fn newly_visible_grid_refreshes_the_known_passage_projection_in_updates() {
    let mut game = Game::new(64);
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    let position = game.player.position;
    replace_terrain(&mut game, position, "demo.terrain.floor");
    let index = game.index(position).unwrap();
    game.explored[index] = false;
    let mut previous = game.visual_cells();
    previous[index].visibility = rfb_protocol::VisibilityState::Hidden;
    game.last_visual_cells = Some(previous);
    assert!(!game.cell_dto(position).known_projectile_passage);
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| { cell.position == position && cell.known_projectile_passage })
    );
}

#[test]
fn item_command_categories_are_public_projection_and_survive_save_without_mutation() {
    use rfb_protocol::ItemUseCategoryDto::*;
    let mut game = Game::new(64);
    for (tag, category) in [
        ("food", Food),
        ("potion", Potion),
        ("scroll", Scroll),
        ("wand", Wand),
        ("staff", Staff),
        ("rod", Rod),
    ] {
        let kind = game
            .content
            .item_definitions()
            .find(|definition| definition.tags.iter().any(|value| value == tag))
            .unwrap()
            .id
            .clone();
        let id = format!("test.shortcut.{tag}");
        give_inventory_item(&mut game, &id, &kind);
        let before = game.state_hash();
        let dto = game
            .inventory_dto()
            .into_iter()
            .find(|item| item.id == id)
            .unwrap();
        assert_eq!(dto.use_category, Some(category));
        assert_eq!(game.state_hash(), before);
        let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
        assert_eq!(
            restored
                .inventory_dto()
                .into_iter()
                .find(|item| item.id == id)
                .unwrap(),
            dto
        );
    }
}

#[test]
fn remembered_objects_survive_save_but_forgetting_removes_map_marks_only() {
    let mut game = Game::new(64);
    clear_monsters(&mut game);
    let position = Position { x: 11, y: 11 };
    let distant = Position { x: 19, y: 19 };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    replace_terrain(&mut game, distant, "demo.terrain.floor");
    game.player.position = position;
    give_inventory_item(&mut game, "test.memory.item", "demo.item.dagger");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(position);
    let gold_id = game.allocate_gold_pile_id().unwrap();
    game.gold_piles.push(GoldPile {
        id: gold_id.clone(),
        position,
        amount: 10,
        appearance: rfb_protocol::GoldAppearanceDto::Gold,
        discovered: false,
    });
    game.reveal_current_visibility();
    game.identify_item_instance("test.memory.item", ItemIdentificationRequest::new(true));
    let knowledge = game.item_property_knowledge["test.memory.item"].clone();
    game.player.position = distant;
    game.reveal_current_visibility();
    assert!(!game.is_visible(position));
    assert_eq!(
        game.cell_dto(position).item_id.as_deref(),
        Some(gold_id.as_str())
    );
    assert!(
        game.items_dto()
            .iter()
            .any(|item| item.id == "test.memory.item")
    );
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.cell_dto(position), game.cell_dto(position));

    restored.clear_current_floor_memory(&mut BTreeSet::new());
    assert!(restored.cell_dto(position).item_id.is_none());
    assert!(
        !restored
            .items_dto()
            .iter()
            .any(|item| item.id == "test.memory.item")
    );
    let mut forgotten = knowledge.clone();
    forgotten.discovered = false;
    assert_eq!(
        restored.item_property_knowledge["test.memory.item"],
        forgotten
    );
    let mut reloaded =
        Game::from_save(restored.to_save(), restored.behavior_preferences()).unwrap();
    assert!(reloaded.cell_dto(position).item_id.is_none());
    reloaded.player.position = position;
    reloaded.reveal_current_visibility();
    assert_eq!(
        reloaded.item_property_knowledge["test.memory.item"],
        knowledge
    );
    assert_eq!(
        reloaded.cell_dto(position).item_id.as_deref(),
        Some(gold_id.as_str())
    );
    reloaded.gold_piles.retain(|pile| pile.id != gold_id);
    assert_eq!(
        reloaded.cell_dto(position).item_id.as_deref(),
        Some("test.memory.item")
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
    let restored = Game::from_save(saved, Game::default_behavior_preferences())
        .expect("discovered items should round-trip");
    assert!(
        restored
            .snapshot()
            .items
            .iter()
            .any(|item| item.id == "test.item.hidden-discovery")
    );
}
