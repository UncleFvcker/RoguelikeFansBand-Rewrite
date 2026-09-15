// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::inventory::ItemIdentificationRequest;

#[test]
#[ignore = "explicit discovery preparation for ordinary standalone acceptance"]
fn export_discovery_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("DISCOVERY_INPUT").unwrap());
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let mut game = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    choose_human_talent_if_pending(&mut game);
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    let kind = "demo.actor.fang-farmer-maggots-dog";
    game.push_generated_actor("archive.dead".into(), kind, game.player.position);
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    death(&mut game, "archive.dead", true);
    game.probed_actor_kind_ids.insert(kind.into());
    let living = "demo.actor.grip-farmer-maggots-dog";
    game.push_generated_actor("archive.living".into(), living, game.player.position);
    game.record_visible_discoveries();
    let artifact = game
        .content
        .item_definitions()
        .find(|item| item.tags.iter().any(|tag| tag == "artifact"))
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "archive.artifact", &artifact);
    game.identify_item_instance("archive.artifact", ItemIdentificationRequest::new(true));
    give_inventory_item(&mut game, "archive.ego", "demo.item.dagger");
    let ego = game
        .content
        .affix_definitions()
        .find(|entry| entry.rfb_ego.is_some())
        .unwrap()
        .id
        .clone();
    game.items.last_mut().unwrap().affix_ids.push(ego);
    game.identify_item_instance("archive.ego", ItemIdentificationRequest::new(true));
    for id in ["archive.artifact", "archive.ego"] {
        game.items.retain(|item| item.id != id);
        game.item_property_knowledge.remove(id);
    }
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let directory = input.parent().unwrap();
    std::fs::write(
        directory.join("discovery-prepared.rfbsave"),
        rfb_save::encode(&header, &game.to_save()).unwrap(),
    )
    .unwrap();
    std::fs::write(directory.join("discovery-scenario.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "hash": game.state_hash(), "discovery": game.discovery_dto(),
        "preparation": "Normal fresh character; entered Warrens depth 3; recognized two uniques and killed Fang through the credited death path; researched Fang; identified one fixed artifact and one ego, then removed both items and local monsters. This proves retained records, not natural acquisition."
    })).unwrap()).unwrap();
}

fn fresh() -> Game {
    let mut game = Game::new_with_build(608, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game
}

fn death(game: &mut Game, id: &str, credit: bool) {
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    let event = DomainEvent::EntityDiedFromStatus {
        target_kind_id: game.entities[index].kind_id.clone(),
        status_kind_id: STATUS_POISON.into(),
        damage: crate::effect::resolve_damage(
            crate::effect::DamagePacket::new(1, DamageType::Poison),
            ResistanceLevel::Normal,
        ),
    };
    if credit {
        game.resolve_actor_death(
            index,
            event,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    } else {
        game.resolve_actor_death_without_credit(
            index,
            event,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    }
}

#[test]
fn identification_retains_lost_kinds_artifacts_and_egos_without_revealing_unknowns() {
    let mut game = fresh();
    let potion = game
        .content
        .item_definitions()
        .find(|item| {
            item.appearance_name_key.is_some()
                && game.item_knowledge_dto(&item.id) != ItemKnowledgeDto::Aware
        })
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.unknown", &potion);
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert!(!game.discovery.objects.contains(&potion));
    game.mark_item_aware(&potion);
    let affix = game
        .content
        .affix_definitions()
        .find(|affix| affix.rfb_ego.is_some())
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.ego", "demo.item.dagger");
    game.items.last_mut().unwrap().affix_ids.push(affix.clone());
    game.discover_item("test.ego");
    assert!(!game.discovery.egos.contains(&affix));
    game.identify_item_instance("test.ego", ItemIdentificationRequest::new(true));
    assert!(game.discovery.egos.contains(&affix));
    let artifact = game
        .content
        .item_definitions()
        .find(|item| item.tags.iter().any(|tag| tag == "artifact"))
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.artifact", &artifact);
    game.discover_item("test.artifact");
    assert!(!game.discovery.artifacts.contains(&artifact));
    game.identify_item_instance("test.artifact", ItemIdentificationRequest::new(false));
    assert!(game.discovery.artifacts.contains(&artifact));
    give_inventory_item(&mut game, "test.random", "demo.item.dagger");
    game.items.last_mut().unwrap().artifact_name = Some("(永恒蘑菇)".into());
    game.identify_item_instance("test.random", ItemIdentificationRequest::new(true));
    for id in ["test.unknown", "test.ego", "test.artifact", "test.random"] {
        game.items.retain(|item| item.id != id);
        game.item_property_knowledge.remove(id);
    }
    let archive = game.discovery_dto();
    assert!(archive.objects.iter().any(|item| item.id == potion));
    assert!(
        archive
            .artifacts
            .iter()
            .any(|item| item.custom_name.as_deref() == Some("(永恒蘑菇)"))
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.discovery_dto(), archive);
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn remembered_monsters_survive_departure_and_kills_use_real_credit() {
    let mut game = fresh();
    let kind = "demo.actor.small-kobold";
    game.push_generated_actor("test.seen".into(), kind, game.player.position);
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert!(game.discovery_knows_monster(kind));
    death(&mut game, "test.seen", true);
    game.push_generated_actor("test.other".into(), kind, game.player.position);
    death(&mut game, "test.other", false);
    game.map_scale = MapScaleDto::World;
    let row = game
        .discovery_dto()
        .monsters
        .into_iter()
        .find(|row| row.monster.kind_id == kind)
        .unwrap();
    assert_eq!(row.kills, 1);
    assert!(row.monster.knowledge.is_none());
    game.map_scale = MapScaleDto::Local;
    game.probed_actor_kind_ids.insert(kind.into());
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    let before = game.to_save();
    let hash = game.state_hash();
    assert!(
        game.discovery_dto()
            .monsters
            .iter()
            .find(|row| row.monster.kind_id == kind)
            .unwrap()
            .monster
            .knowledge
            .is_some()
    );
    assert_eq!(game.to_save(), before);
    assert_eq!(game.state_hash(), hash);
    assert_eq!(
        Game::from_save(before, Game::default_behavior_preferences())
            .unwrap()
            .discovery_dto(),
        game.discovery_dto()
    );
}

#[test]
fn disguises_hallucination_and_unseen_species_do_not_reveal_true_identity() {
    let mut game = fresh();
    game.discovery.monsters.clear();
    game.push_generated_actor(
        "test.disguise".into(),
        "demo.actor.archlich",
        game.player.position,
    );
    game.entities.last_mut().unwrap().appearance_kind_id = Some("demo.actor.small-kobold".into());
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert!(game.discovery_knows_monster("demo.actor.small-kobold"));
    assert!(!game.discovery_knows_monster("demo.actor.archlich"));
    game.entities.last_mut().unwrap().appearance_kind_id = None;
    game.player.statuses.push(
        monster_combat::melee_status("rfb.status.hallucination", 10, "test.hallucination").status,
    );
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert!(!game.discovery_knows_monster("demo.actor.archlich"));
    game.player.statuses.clear();
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert!(game.discovery_knows_monster("demo.actor.archlich"));
}

#[test]
fn living_unique_is_not_confused_with_occupied_spawn_and_death_is_retained() {
    let mut game = fresh();
    let kind = "demo.actor.fang-farmer-maggots-dog";
    game.push_generated_actor("test.unique".into(), kind, game.player.position);
    game.reveal_current_visibility();
    game.record_visible_discoveries();
    assert_eq!(game.actor_kind_available_instance_count(kind), 0);
    assert_eq!(
        game.discovery_dto()
            .monsters
            .iter()
            .find(|row| row.monster.kind_id == kind)
            .unwrap()
            .alive,
        Some(true)
    );
    death(&mut game, "test.unique", false);
    assert_eq!(
        game.discovery_dto()
            .monsters
            .iter()
            .find(|row| row.monster.kind_id == kind)
            .unwrap()
            .alive,
        Some(false)
    );
}

#[test]
fn deepest_dungeon_record_survives_ascent_and_save_and_rejects_invalid_archive() {
    let mut game = fresh();
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    let entry = game.discovery.dungeons.last().unwrap().clone();
    assert_eq!(entry.max_depth, 3);
    game.transition_floor("demo.floor.warrens-depth-1".into(), None, None, false)
        .unwrap()
        .unwrap();
    assert_eq!(
        game.discovery
            .dungeons
            .iter()
            .find(|row| row.dungeon_id == entry.dungeon_id)
            .unwrap()
            .max_depth,
        3
    );
    let save = game.to_save();
    let restored = Game::from_save(save.clone(), Game::default_behavior_preferences()).unwrap();
    assert_eq!(restored.discovery, game.discovery);
    for variant in 0..4 {
        let mut invalid = save.clone();
        match variant {
            0 => invalid.discovery.objects.push("missing.item".into()),
            1 => invalid
                .discovery
                .monsters
                .push(rfb_protocol::MonsterDiscoverySaveDto {
                    kind_id: "missing.actor".into(),
                    seen: true,
                    kills: 0,
                }),
            2 => invalid.discovery.dungeons[0].max_depth = u16::MAX,
            _ => invalid
                .discovery
                .objects
                .push(invalid.discovery.objects[0].clone()),
        }
        assert!(
            Game::from_save(invalid, Game::default_behavior_preferences()).is_err(),
            "variant {variant}"
        );
    }
}
