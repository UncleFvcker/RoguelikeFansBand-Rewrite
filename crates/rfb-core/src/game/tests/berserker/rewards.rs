// SPDX-License-Identifier: MPL-2.0

use super::*;

fn virtues(game: &mut Game) {
    use VirtueKindDto::*;
    game.virtues = [
        Vitality, Unlife, Valour, Honour, Justice, Compassion, Harmony, Patience,
    ]
    .map(|kind| rfb_protocol::VirtueDto { kind, value: 0 });
}

#[test]
fn all_eight_class_rewards_are_claimed_as_real_items_with_materialized_egos() {
    for slug in [
        "anambar-orc-camp",
        "anambar-apina-island",
        "anambar-lord-bovin-treachery",
        "anambar-cellar-killer",
        "crows-nest",
        "vapor-quest",
        "old-castle",
        "thalos-old-watchtower",
    ] {
        let mut game = berserker(1);
        let task_id = format!("demo.task.{slug}");
        let task = game
            .content
            .world(&game.world_id)
            .unwrap()
            .tasks
            .iter()
            .find(|task| task.id == task_id)
            .unwrap()
            .clone();
        let facility_id = task.source_facility_id.as_deref().unwrap();
        crate::game::tests::town::enter_town_facility(&mut game, facility_id);
        game.task_states.insert(
            task_id.clone(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: 1,
                required: 1,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        let reward = task.reward.as_ref().unwrap();
        if slug == "vapor-quest" {
            let mut full = game.clone();
            while full.inventory_used_slots() < full.inventory_slot_capacity() {
                let id = format!("test.filler.{}", full.items.len());
                give_inventory_item(&mut full, &id, "demo.item.dagger");
            }
            let rng = full.rng.clone();
            assert_eq!(
                full.claim_task_reward(facility_id, &task_id),
                Err("inventory-full")
            );
            assert_eq!(full.rng, rng);
            assert_eq!(
                full.task_states[&task_id].status,
                TaskStatusKindDto::RewardAvailable
            );
            assert_eq!(
                reward
                    .class_overrides
                    .iter()
                    .find(|entry| entry.class_id == "demo.class.berserker")
                    .unwrap()
                    .entries[0]
                    .generation_depth,
                Some(40)
            );
        }
        let mut expected_rng = game.rng.clone();
        let expected = crate::game::tasks::reward_item(
            &game.content,
            Some("demo.class.berserker"),
            reward,
            ItemLocation::Inventory,
            &mut expected_rng,
        );
        game.claim_task_reward(facility_id, &task_id).unwrap();
        assert_eq!(game.rng, expected_rng, "{slug}: preview must not spend RNG");
        let item = game
            .items
            .iter()
            .find(|item| item.id == reward.item_instance_id)
            .unwrap();
        assert_eq!(item, &expected);
        match slug {
            "anambar-orc-camp" => assert_eq!(item.affix_ids, ["rfb-legacy.affix.slaying"]),
            "vapor-quest" => {
                assert_eq!(item.kind_id, "demo.item.sling");
                assert_eq!(item.affix_ids, ["rfb-legacy.affix.the-hunter"]);
            }
            "thalos-old-watchtower" => {
                assert_eq!(item.kind_id, "demo.item.long-sword");
                assert_eq!(item.affix_ids, ["rfb-legacy.affix.death"]);
            }
            "crows-nest" => assert_eq!(item.kind_id, "demo.item.enlightenment-potion"),
            "old-castle" => {
                assert!(matches!(
                    item.kind_id.as_str(),
                    "demo.item.slayer" | "demo.item.pain"
                ));
                assert!(game.generated_artifact_ids.contains(&item.kind_id));
            }
            _ => assert_eq!(item.kind_id, "demo.item.vitalis-elixir"),
        }
        if !item.affix_ids.is_empty() {
            assert!(!item.rolled_affixes.is_empty());
            assert!(item.enchantments.to_hit > 0 && item.enchantments.to_damage > 0);
        }
        assert_eq!(
            game.task_states[&task_id].status,
            TaskStatusKindDto::Completed
        );
        assert_eq!(
            game.claim_task_reward(facility_id, &task_id),
            Err("reward-unavailable")
        );
    }
}

#[test]
fn high_books_reward_quantity_max_experience_and_realm_only_after_valid_destruction() {
    for build in [BUILD, "demo.build.warrior", "demo.build.paladin-death"] {
        let mut game = Game::new_with_build(923, build).unwrap();
        clear_monsters(&mut game);
        virtues(&mut game);
        game.progress.maximum_experience = 8000;
        give_inventory_item(&mut game, "test.book", "demo.item.book-of-the-unicorn");
        game.items.last_mut().unwrap().quantity = 3;
        game.items.last_mut().unwrap().inscription = Some("!k".to_owned());
        dispatch_next(
            &mut game,
            GameCommand::DestroyItem {
                item_id: "test.book".to_owned(),
                quantity: 2,
            },
        );
        assert_eq!(game.progress.experience, 0);
        assert_eq!(game.virtue_current(VirtueKindDto::Vitality), 0);
        game.items.last_mut().unwrap().inscription = None;
        dispatch_next(
            &mut game,
            GameCommand::DestroyItem {
                item_id: "test.book".to_owned(),
                quantity: 2,
            },
        );
        assert_eq!(game.progress.experience, 200, "{build}");
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == "test.book")
                .unwrap()
                .quantity,
            1
        );
        assert_eq!(game.virtue_current(VirtueKindDto::Vitality), -1);
        assert_eq!(game.virtue_current(VirtueKindDto::Unlife), 1);
    }
    let mut game = berserker(1);
    virtues(&mut game);
    game.progress.maximum_experience = 1_000_000;
    give_inventory_item(&mut game, "test.low-book", "demo.item.black-prayers");
    dispatch_next(
        &mut game,
        GameCommand::DestroyItem {
            item_id: "test.low-book".to_owned(),
            quantity: 1,
        },
    );
    assert_eq!(game.progress.experience, 0);
    give_inventory_item(&mut game, "test.high-book", "demo.item.necronomicon");
    dispatch_next(
        &mut game,
        GameCommand::DestroyItem {
            item_id: "test.high-book".to_owned(),
            quantity: 1,
        },
    );
    assert_eq!(game.progress.experience, 10_000);
    assert_eq!(game.virtue_current(VirtueKindDto::Vitality), 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Unlife), -1);
}

#[test]
fn automatic_book_destruction_preserves_level_up_events_and_saved_progress() {
    let mut game = berserker(1);
    virtues(&mut game);
    give_inventory_item(&mut game, "test.book", "demo.item.book-of-the-unicorn");
    game.progress.maximum_experience = 8000;
    game.interface_locale = rfb_protocol::LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            rfb_protocol::LocaleDto::EnUs,
            "!spellbooks".to_owned()
        )
        .is_empty()
    );
    let outcomes = game
        .apply_mogaminator_to_carried_items(vec!["test.book".to_owned()])
        .unwrap();
    let mut events = Vec::new();
    game.record_mogaminator_resolutions(outcomes, &mut events, &mut BTreeSet::new());
    assert_eq!(game.progress.experience, 100);
    assert!(!game.items.iter().any(|item| item.id == "test.book"));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerLevelGained { .. }))
    );
    crate::game::tests::support::choose_human_talent_if_pending(&mut game);
    let loaded = Game::from_save(game.to_save()).unwrap();
    assert_eq!(loaded.state_hash(), game.state_hash());
}

#[test]
fn life_potion_keeps_permanent_berserk_and_restores_mental_and_vitality_state() {
    let mut game = berserker(1);
    virtues(&mut game);
    game.progress.maximum_experience = 100;
    game.player.hp = 1;
    for status in [
        STATUS_HALLUCINATION,
        STATUS_UNWELL,
        STATUS_CONFUSION,
        STATUS_STUN,
    ] {
        game.apply_player_mental_status(status, 100, "test");
    }
    give_inventory_item(&mut game, "test.life", "demo.item.vitalis-elixir");
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.life".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.player.hp, game.effective_player_max_hp());
    assert_eq!(game.progress.experience, 100);
    assert_eq!(game.virtue_current(VirtueKindDto::Vitality), 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Unlife), -5);
    assert!(game.player_has_status_kind(STATUS_BERSERK));
    for status in [
        STATUS_HALLUCINATION,
        STATUS_UNWELL,
        STATUS_CONFUSION,
        STATUS_STUN,
    ] {
        assert!(!game.player_has_status_kind(status));
    }
}
