// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const KIND: &str = "demo.item.thingol";

fn context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-30".into(),
        depth: 30,
        source: LootSource::MonsterDeath {
            actor_id: "test.thingol".into(),
        },
    }
}

fn prepare() -> (Game, String) {
    prepare_game(Game::new_with_build(511, "demo.build.warrior").unwrap())
}

fn prepare_game(mut game: Game) -> (Game, String) {
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    let draft = game.fixed_item_draft(&context(), KIND.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    assert_eq!(
        game.visible_item_modifiers(&game.items[0]),
        StatModifiersDto::default()
    );
    game.equip_inventory_item(&id, None).unwrap();
    give_inventory_item(&mut game, "donor", "demo.item.detect-objects-staff");
    give_inventory_item(&mut game, "target", "demo.item.detect-objects-staff");
    game.items[2].charges.as_mut().unwrap().current = 0;
    (game, id)
}

fn pair() -> TargetSelection {
    TargetSelection::RechargeItems {
        source_item_id: "donor".into(),
        target_item_id: "target".into(),
    }
}

fn activate(game: &mut Game, id: &str, target: Option<&TargetSelection>) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.use_inventory_item(
        id,
        target,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn ordinary_thingol_keeps_source_properties_unique_identity_and_cooldown_after_save() {
    let (mut game, id) = prepare();
    let mut pool = game.clone();
    pool.generated_artifact_ids.remove(KIND);
    (0..20_000)
        .find_map(|_| {
            pool.generate_loot_instances(&context(), ItemLocation::Inventory)
                .unwrap()
                .into_iter()
                .find(|item| item.kind_id == "demo.item.cloak" && item.artifact_name.is_none())
        })
        .expect("the ordinary complete pool must reach the cloak base");
    pool.generated_artifact_ids.remove(KIND);
    (0..20_000)
        .find_map(|_| {
            pool.roll_fixed_artifact_kind_id(&context(), Some("demo.item.cloak"), false)
                .filter(|kind| kind == KIND)
        })
        .expect("source level and rarity gates must reach Thingol");
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let definition = game.content.item(KIND).unwrap();
    assert_eq!(
        (
            definition.modifiers.defense,
            definition.modifiers.dexterity,
            definition.modifiers.charisma
        ),
        (19, 3, 3)
    );
    assert!(
        game.equipment_dto()
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .requires_recharge_targets
    );
    let seed = (0..1000)
        .find(|seed| {
            let mut probe = game.clone();
            probe.rng = RfbRng::seeded(*seed);
            activate(&mut probe, &id, Some(&pair()))
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceSkillChecked {
                            succeeded: true,
                            ..
                        }
                    )
                })
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    let mut saved = Game::from_save(game.to_save()).unwrap();
    assert_eq!(
        activate(&mut game, &id, Some(&pair())),
        activate(&mut saved, &id, Some(&pair()))
    );
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    for run in [&mut game, &mut saved] {
        for tick in 1..=700 {
            run.world_tick += 1;
            run.process_inventory_device_recovery(&mut Vec::new());
            assert_eq!(
                run.items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .charges
                    .unwrap()
                    .current,
                u32::from(tick == 700),
                "cooldown tick {tick}"
            );
        }
        assert_ne!(
            run.roll_fixed_artifact_kind_id(&context(), Some("demo.item.cloak"), false),
            Some(KIND.into())
        );
    }
    assert_eq!(game.state_hash(), saved.state_hash());
    assert_eq!(game.rng, saved.rng);
}

#[test]
#[ignore = "prepares Thingol and two real devices for the focused standalone scenario"]
fn export_thingol_desktop_save() {
    let input = std::path::PathBuf::from(std::env::var("THINGOL_DESKTOP_INPUT").unwrap());
    let (header, payload) = rfb_save::decode(&std::fs::read(&input).unwrap()).unwrap();
    assert!(header.museum_binding.is_some());
    let (mut game, id) = prepare_game(Game::from_save(payload).unwrap());
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    game.items[2].location = ItemLocation::Ground(game.player.position);
    game.identify_item_instance("donor", ItemIdentificationRequest::new(true));
    let seed = (0..1000)
        .find(|seed| {
            let mut probe = game.clone();
            probe.rng = RfbRng::seeded(*seed);
            activate(&mut probe, &id, Some(&pair()))
                .iter()
                .any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceRechargeResolved {
                            succeeded: true,
                            source_destroyed: false,
                            ..
                        }
                    )
                })
        })
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.reveal_current_visibility();
    assert_eq!(
        game.state_hash(),
        Game::from_save(game.to_save()).unwrap().state_hash()
    );
    std::fs::write(input.with_file_name("prepared.hash"), game.state_hash()).unwrap();
    std::fs::write(
        input.with_file_name("prepared.rfbsave"),
        rfb_save::encode(&header, &game.to_save()).unwrap(),
    )
    .unwrap();
}

#[test]
fn cancellation_failed_skill_and_invalid_pairs_preserve_energy_and_only_valid_attempts_take_time() {
    let (base, id) = prepare();
    for succeeded in [false, true] {
        let (mut game, seed) = (0..1000)
            .find_map(|seed| {
                let mut game = base.clone();
                game.rng = RfbRng::seeded(seed);
                activate(&mut game, &id, None)
                    .iter()
                    .any(|event| {
                        matches!(event,
                DomainEvent::DeviceSkillChecked { succeeded: actual, .. } if *actual == succeeded)
                    })
                    .then_some((base.clone(), seed))
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        let skill = game.player_derived_stats().device_skill.value;
        if expected.bounded(100) >= 10 && skill > 0 {
            expected.bounded(skill as u64);
        }
        activate(&mut game, &id, None);
        assert_eq!(game.rng, expected);
        assert_eq!(game.items, base.items);
        game.rng = RfbRng::seeded(seed);
        let tick = game.world_tick;
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        );
        assert!(game.world_tick > tick);
        assert_eq!(game.items[0].charges.unwrap().current, 1);
    }
    for target in [
        TargetSelection::RechargeItems {
            source_item_id: "donor".into(),
            target_item_id: "donor".into(),
        },
        TargetSelection::RechargeItems {
            source_item_id: id.clone(),
            target_item_id: "target".into(),
        },
        TargetSelection::RechargeItems {
            source_item_id: "missing".into(),
            target_item_id: "target".into(),
        },
    ] {
        let mut game = base.clone();
        let rng = game.rng.clone();
        assert!(activate(&mut game, &id, Some(&target)).contains(&DomainEvent::ItemUseUnavailable));
        assert_eq!(game.items, base.items);
        assert_eq!(game.rng, rng);
        let tick = game.world_tick;
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: Some(target),
            },
        );
        assert_eq!(game.world_tick, tick);
        assert_eq!(game.rng, rng);
    }
    let mut ground = base;
    ground.items[1].location = ItemLocation::Ground(ground.player.position);
    ground.items[2].location = ItemLocation::Ground(ground.player.position);
    assert!(ground.item_can_supply_recharge(&ground.items[1]));
    assert!(ground.item_can_receive_recharge(&ground.items[2]));
    ground.items[2].location = ItemLocation::Ground(Position { x: 0, y: 0 });
    assert!(!ground.item_can_receive_recharge(&ground.items[2]));
    assert!(!ground.item_can_receive_recharge(&ground.items[0]));
}

#[test]
fn transfer_spends_source_on_both_target_outcomes_and_limits_power_source_and_missing_capacity() {
    let (mut base, _) = prepare();
    base.player
        .statuses
        .push(monster_combat::melee_status(STATUS_INVENTORY_PROTECTION, 10, "test.status").status);
    for (supply, missing, power, attempted) in
        [(200, 200, 130, 130), (17, 200, 130, 17), (200, 9, 130, 9)]
    {
        for (destroy, succeed) in [(false, false), (false, true), (true, false), (true, true)] {
            let mut game = base.clone();
            game.items[1].charges = Some(ItemChargesDto {
                current: supply,
                maximum: supply,
            });
            game.items[2].charges = Some(ItemChargesDto {
                current: 3,
                maximum: missing + 3,
            });
            let difficulty = game.items[2]
                .activation
                .as_ref()
                .unwrap()
                .device_check_difficulty as u32;
            let one_in = (power - difficulty / 2) / 15;
            let seed = (0..1000)
                .find(|seed| {
                    let mut rng = RfbRng::seeded(*seed);
                    (rng.bounded(3) == 0) == destroy
                        && (rng.bounded(u64::from(one_in)) != 0) == succeed
                })
                .unwrap();
            game.rng = RfbRng::seeded(seed);
            let mut expected = game.rng.clone();
            expected.bounded(3);
            expected.bounded(u64::from(one_in));
            let result = game.recharge_inventory_item_from_device(
                "target",
                "donor",
                DeviceRechargeRequest::new(power, 3),
            );
            assert_eq!(
                (
                    result.source_destroyed,
                    result.target.succeeded,
                    result.target.attempted
                ),
                (destroy, succeed, attempted)
            );
            assert_eq!(
                result.target.target_after,
                3 + if succeed { attempted } else { 0 }
            );
            assert_eq!(
                game.items
                    .iter()
                    .find(|item| item.id == "donor")
                    .map(|item| item.charges.unwrap().current),
                (!destroy).then_some(supply - attempted)
            );
            assert_eq!(game.rng, expected);
        }
    }
}

#[test]
fn scroll_ego_and_random_artifact_recharge_use_real_device_energy_and_restore_the_same_next_result()
{
    for source in ["scroll", "ego", "random-artifact"] {
        let (mut game, cloak) = prepare();
        game.items.retain(|item| item.id != cloak);
        let id = "recharger";
        if source == "scroll" {
            give_inventory_item(&mut game, id, "demo.item.recharging-scroll");
        } else {
            give_inventory_item(&mut game, id, "demo.item.iron-crown");
            let original = game.items.last().unwrap().clone();
            let item = (0..10_000)
                .find_map(|_| {
                    let mut item = original.clone();
                    if source == "ego" {
                        super::super::ego::materialize_ego_with_rng(
                            false,
                            &game.content,
                            &mut game.rng,
                            &original.kind_id,
                            vec!["rfb-legacy.affix.magi-headgear".into()],
                            |_| 60,
                            60,
                            2,
                        )
                        .apply_to(&mut item);
                    } else {
                        item = super::super::random_artifact::materialize(
                            &game.content,
                            &mut game.rng,
                            &original,
                            super::super::random_artifact::Creation {
                                level: 20,
                                class_id: "demo.class.warrior",
                                ..Default::default()
                            },
                            &mut game.random_artifact_names,
                            20,
                            2,
                            &mut false,
                            false,
                        )
                        .unwrap()
                        .0;
                    }
                    item.activation
                        .as_ref()
                        .is_some_and(|activation| {
                            activation.profile_id.ends_with("recharge-from-device")
                        })
                        .then_some(item)
                })
                .unwrap_or_else(|| {
                    panic!(
                        "formal {source} materialization must reach the existing recharge profile"
                    )
                });
            *game.items.last_mut().unwrap() = item;
            game.equip_inventory_item(id, None).unwrap();
            assert!(
                game.equipment_dto()
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .requires_recharge_targets
            );
        }
        let seed = (0..1000)
            .find(|seed| {
                let mut probe = game.clone();
                probe.rng = RfbRng::seeded(*seed);
                activate(&mut probe, id, Some(&pair())).iter().any(|event| {
                    matches!(
                        event,
                        DomainEvent::DeviceRechargeResolved {
                            succeeded: true,
                            ..
                        }
                    )
                })
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut saved = Game::from_save(game.to_save()).unwrap();
        let events = activate(&mut game, id, Some(&pair()));
        assert_eq!(events, activate(&mut saved, id, Some(&pair())));
        assert!(events.iter().any(
            |event| matches!(event, DomainEvent::DeviceRechargeResolved {
            source_is_item: true, attempted, target_before: 0, target_after, succeeded: true, ..
        } if *attempted > 0 && attempted == target_after)
        ));
        assert_eq!(game.state_hash(), saved.state_hash());
        assert_eq!(game.rng, saved.rng);
        if source == "scroll" {
            assert!(!game.items.iter().any(|item| item.id == id));
        } else {
            assert_eq!(
                game.items
                    .iter()
                    .find(|item| item.id == id)
                    .unwrap()
                    .charges
                    .unwrap()
                    .current,
                0
            );
        }
    }
}
