// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn utility_game() -> Game {
    let mut game = Game::new_with_build(410, "demo.build.high-mage-death").unwrap();
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    game.items.clear();
    game
}

fn context(game: &Game, depth: u16) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: game.current_floor_id.clone(),
        depth,
        source: LootSource::MonsterDeath {
            actor_id: "test.device-drop".into(),
        },
    }
}

pub(super) fn pick_utility(game: &mut Game, profile: &str) -> String {
    let depth = game
        .content
        .item_definitions()
        .filter_map(|item| item.device_generation.as_ref())
        .flat_map(|generation| &generation.activations)
        .find(|activation| activation.id == profile)
        .unwrap()
        .min_depth
        .max(10);
    let context = context(game, depth);
    for _ in 0..20_000 {
        let generated = game
            .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
            .unwrap();
        for item in generated {
            if item.affix_ids.is_empty()
                && item
                    .activation
                    .as_ref()
                    .is_some_and(|a| a.profile_id == profile)
            {
                let id = item.id.clone();
                game.items.push(item);
                game.pick_up_item_at_player(Some(&id)).unwrap();
                return id;
            }
        }
    }
    panic!("formal device effect was not generated: {profile}");
}

#[test]
fn natural_utility_devices_preserve_instances_and_replay_after_save() {
    let mut game = utility_game();
    let mut remaining = game
        .content
        .item_definitions()
        .filter(|item| {
            game.content
                .loot_table("demo.loot-table.base-items")
                .unwrap()
                .entries
                .iter()
                .any(|entry| entry.item_kind_id == item.id)
        })
        .filter_map(|item| item.device_generation.as_ref())
        .filter(|generation| generation.rfb_device.is_some())
        .flat_map(|generation| generation.activations.iter().map(|p| p.id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(remaining.len(), 13);
    // Controlled depths and repeated production allocation; no pool or weight edits.
    for depth in [10, 20, 40, 60] {
        let context = context(&game, depth);
        for _ in 0..4_000 {
            let generated = game
                .generate_loot_instances(&context, ItemLocation::Ground(game.player.position))
                .unwrap();
            for item in generated {
                let Some(activation) = item.activation.as_ref() else {
                    continue;
                };
                if !remaining.remove(&activation.profile_id) {
                    continue;
                }
                let id = item.id.clone();
                game.items.push(item);
                game.pick_up_item_at_player(Some(&id)).unwrap();
                let item = game.items.iter().find(|item| item.id == id).unwrap();
                let dto = game.inventory_item_dto(item);
                assert!(
                    dto.activation.is_none() && dto.charges.is_none(),
                    "another instance's identification must not reveal this effect"
                );
                game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
                let dto =
                    game.inventory_item_dto(game.items.iter().find(|item| item.id == id).unwrap());
                assert!(dto.activation.is_some() && dto.charges.is_some());
                game.reveal_current_visibility();
                let mut restored = Game::from_save(game.to_save()).unwrap();
                assert_eq!(restored.state_hash(), game.state_hash());
                assert_eq!(restored.rng, game.rng);
                assert_eq!(
                    game.generate_loot_instances(&context, ItemLocation::Inventory)
                        .unwrap(),
                    restored
                        .generate_loot_instances(&context, ItemLocation::Inventory)
                        .unwrap()
                );
                assert_eq!(restored.rng, game.rng);
                game.items.clear();
            }
            if remaining.is_empty() {
                return;
            }
        }
    }
    panic!("unreached source utility effects: {remaining:?}");
}

#[test]
fn source_device_detection_failure_empty_result_and_recovery_keep_instance_knowledge() {
    let mut base = utility_game();
    let id = pick_utility(&mut base, "demo.device-activation.detect-objects");
    let mut seen = BTreeSet::new();
    for seed in 0..128 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let before = game.items[0].charges.unwrap();
        let cost = game.items[0].activation.as_ref().unwrap().cost;
        let mut events = Vec::new();
        game.use_inventory_item(
            &id,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        let succeeded = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::DeviceSkillChecked { succeeded, .. } => Some(*succeeded),
                _ => None,
            })
            .unwrap();
        seen.insert(succeeded);
        assert_eq!(
            game.items[0].charges.unwrap().current,
            before.current - if succeeded { cost } else { 0 }
        );
        assert!(
            game.inventory_item_dto(&game.items[0]).activation.is_none(),
            "empty detection must not identify a source device"
        );
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([false, true]));

    give_inventory_item(&mut base, "test.detected-item", "demo.item.ration-of-food");
    base.items.last_mut().unwrap().location = ItemLocation::Ground(base.player.position);
    for seed in 0..128 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.use_inventory_item(
            &id,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.inventory_item_dto(&game.items[0]).activation.is_none() {
            continue;
        }
        assert!(events.iter().any(|event| matches!(event, DomainEvent::ItemActivationDetected { resolution, .. } if resolution.radius == 30 && resolution.detected_entity_ids == ["test.detected-item"])));
        let before = game.items[0].charges.unwrap().current;
        // Exercise nonzero fractional energy and restoration between recovery ticks.
        for tick in 1..=10 {
            game.world_tick = tick;
            game.process_inventory_device_recovery(&mut Vec::new());
        }
        game.reveal_current_visibility();
        let mut restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        for tick in 11..=110 {
            game.world_tick = tick;
            restored.world_tick = tick;
            game.process_inventory_device_recovery(&mut Vec::new());
            restored.process_inventory_device_recovery(&mut Vec::new());
        }
        assert!(game.items[0].charges.unwrap().current > before);
        assert_eq!(
            &game.items[0],
            restored.items.iter().find(|item| item.id == id).unwrap()
        );
        assert_eq!(game.rng, restored.rng);
        let mut invalid = game.clone();
        invalid.items[0].activation.as_mut().unwrap().cost = 1001;
        assert!(Game::from_save(invalid.to_save()).is_err());
        invalid = game.clone();
        invalid.items[0].charges.as_mut().unwrap().maximum = 1001;
        assert!(Game::from_save(invalid.to_save()).is_err());
        return;
    }
    panic!("source detection did not succeed");
}

#[test]
fn source_identification_cancel_refunds_time_after_successful_check_and_berserker_cannot_use_it() {
    let mut base = utility_game();
    let id = pick_utility(&mut base, "rfb.device-activation.staff.identify-full");
    give_inventory_item(&mut base, "test.identify-target", "demo.item.dagger");
    let mut seen = BTreeSet::new();
    for seed in 0..128 {
        let mut game = base.clone();
        for progress in game.ability_progress.values_mut() {
            progress.cooldown_remaining = 3;
        }
        let progress = game.ability_progress.clone();
        game.rng = RfbRng::seeded(seed);
        let tick = game.world_tick;
        let charges = game.items[0].charges;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        );
        let succeeded = update
            .events
            .iter()
            .find_map(|event| match event.kind.as_str() {
                "skill.device-success" => Some(true),
                "skill.device-failure" => Some(false),
                _ => None,
            })
            .unwrap();
        seen.insert(succeeded);
        if succeeded {
            assert_eq!(game.world_tick, tick);
            assert_eq!(game.ability_progress, progress);
            assert_eq!(game.items[0].charges, charges);
            assert!(game.inventory_item_dto(&game.items[0]).activation.is_some());
            let mut used = base.clone();
            used.rng = RfbRng::seeded(seed);
            let cost = used.items[0].activation.as_ref().unwrap().cost;
            used.use_inventory_item(
                &id,
                Some(&TargetSelection::Item {
                    item_id: "test.identify-target".into(),
                }),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert!(used.item_property_knowledge["test.identify-target"].identified);
            assert_eq!(
                used.items[0].charges.unwrap().current,
                charges.unwrap().current - cost
            );
            let mut restored = Game::from_save(used.to_save()).unwrap();
            assert_eq!(restored.state_hash(), used.state_hash());
            assert_eq!(restored.rng.bounded(1000), used.rng.bounded(1000));
        } else {
            assert!(game.world_tick > tick);
        }
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([false, true]));
    let mut berserker = Game::new_with_build(410, "demo.build.berserker").unwrap();
    clear_monsters(&mut berserker);
    choose_human_talent_if_pending(&mut berserker);
    berserker.items = vec![base.items[0].clone()];
    let before = berserker.items.clone();
    let rng = berserker.rng.clone();
    berserker
        .use_inventory_item(
            &id,
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(berserker.items, before);
    assert_eq!(berserker.rng, rng);
}

#[test]
fn ordinary_floor_device_uses_shared_sp_check_and_effect_but_only_at_player_feet() {
    let mut base = utility_game();
    give_inventory_item(&mut base, "test.floor-device", "demo.item.detection-rod");
    let mut seen = BTreeSet::new();
    for seed in 0..128 {
        let mut carried = base.clone();
        carried.rng = RfbRng::seeded(seed);
        let mut ground = carried.clone();
        ground.items[0].location = ItemLocation::Ground(ground.player.position);
        let (mut carried_events, mut ground_events) = (Vec::new(), Vec::new());
        for (game, events) in [
            (&mut carried, &mut carried_events),
            (&mut ground, &mut ground_events),
        ] {
            game.use_inventory_item(
                "test.floor-device",
                Some(&TargetSelection::SelfTarget),
                None,
                events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert_eq!(ground_events, carried_events);
        assert_eq!(ground.items[0].charges, carried.items[0].charges);
        assert_eq!(ground.rng, carried.rng);
        let succeeded = ground_events
            .iter()
            .find_map(|event| match event {
                DomainEvent::DeviceSkillChecked { succeeded, .. } => Some(*succeeded),
                _ => None,
            })
            .unwrap();
        seen.insert(succeeded);
        assert_eq!(
            ground.items[0].location,
            ItemLocation::Ground(ground.player.position)
        );
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(seen, BTreeSet::from([false, true]));
    base.items[0].location = ItemLocation::Ground(base.player.position);
    base.items[0].charges.as_mut().unwrap().current = 0;
    let before = (base.turn, base.world_tick, base.rng.clone());
    dispatch_next(
        &mut base,
        GameCommand::UseItem {
            item_id: "test.floor-device".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!((base.turn, base.world_tick, base.rng.clone()), before);
    base.player.position.x += 1;
    let mut events = Vec::new();
    base.use_inventory_item(
        "test.floor-device",
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(events, [DomainEvent::ItemUseUnavailable]);
    assert_eq!(base.rng, before.2);
    give_inventory_item(
        &mut base,
        "test.floor-scroll",
        "demo.item.trapfinding-scroll",
    );
    base.items.last_mut().unwrap().location = ItemLocation::Ground(base.player.position);
    assert!(
        base.inventory_item_use_context("test.floor-scroll")
            .unwrap()
            .is_none(),
        "non-device floor use remains outside this command"
    );
}
