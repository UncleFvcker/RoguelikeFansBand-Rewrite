// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::inventory::ItemIdentificationRequest;

#[test]
fn unsupported_device_value_is_explicit_and_does_not_hide_a_special_artifact() {
    let mut game = Game::new_with_build(606, "demo.build.warrior").unwrap();
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "test.device", "demo.item.identify-staff");
    let item = game.items.last_mut().unwrap();
    item.affix_ids
        .push("rfb-legacy.affix.resistance-device".into());
    item.location = ItemLocation::Ground(game.player.position);
    let before = game.state_hash();
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-incomplete");
    assert_eq!(game.state_hash(), before);
    give_inventory_item(&mut game, "test.special", "demo.item.dagger");
    let item = game.items.last_mut().unwrap();
    item.artifact_name = Some("(永恒蘑菇)".into());
    item.location = ItemLocation::Ground(game.player.position);
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-1");
}

#[test]
fn floor_inquiry_is_read_only_and_ignores_carried_and_identified_treasure() {
    let mut game = Game::new_with_build(606, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-town");
    game.map_scale = rfb_protocol::MapScaleDto::World;
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-wilderness");
    game.map_scale = rfb_protocol::MapScaleDto::Local;
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-10");
    game.push_generated_actor(
        "test.threat".into(),
        "demo.actor.archlich",
        Position { x: 2, y: 2 },
    );
    assert_ne!(game.floor_feeling_message_key(), "floor-feeling-10");
    game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-10");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.treasure", "demo.item.dagger");
    let item = game.items.last_mut().unwrap();
    item.artifact_name = Some("(永恒蘑菇)".into());
    item.quality = ItemQualityDto::Exceptional;
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-10");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
    let before = game.to_save();
    let hash = game.state_hash();
    assert_eq!(
        game.snapshot().player.floor_feeling_message_key,
        "floor-feeling-1"
    );
    assert_eq!(game.state_hash(), hash);
    assert_eq!(game.to_save(), before);
    let restored = Game::from_save(before, Game::default_behavior_preferences()).unwrap();
    assert_eq!(restored.floor_feeling_message_key(), "floor-feeling-1");
    game.identify_item_instance("test.treasure", ItemIdentificationRequest::new(true));
    assert_eq!(game.floor_feeling_message_key(), "floor-feeling-10");
}

#[test]
fn recall_contains_only_projected_identities_and_researched_knowledge() {
    let mut game = Game::new_with_build(606, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    for monster in game.monster_recall_dtos() {
        assert!(
            monster.knowledge.is_some()
                || game
                    .entities_dto()
                    .iter()
                    .any(|entity| entity.kind_id == monster.kind_id)
        );
    }
    game.map_scale = rfb_protocol::MapScaleDto::World;
    assert!(game.monster_recall_dtos().is_empty());
    game.map_scale = rfb_protocol::MapScaleDto::Local;
    clear_monsters(&mut game);
    assert!(game.monster_recall_dtos().is_empty());
    let kind = game.research_monster_dtos()[0].kind_id.clone();
    game.probed_actor_kind_ids.insert(kind.clone());
    let before = game.state_hash();
    let recall = game.monster_recall_dtos();
    assert_eq!(recall.len(), 1);
    assert_eq!(recall[0].kind_id, kind);
    assert!(recall[0].knowledge.is_some());
    assert_eq!(game.state_hash(), before);
    assert_eq!(
        Game::from_save(game.to_save(), game.behavior_preferences())
            .unwrap()
            .monster_recall_dtos(),
        recall
    );
}
