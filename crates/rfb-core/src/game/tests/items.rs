// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn device_skill_check_distinguishes_builds_without_consuming_on_failure() {
    const ITEM_ID: &str = "test.item.resonance-stabilizer.1";
    let mut tinkerer = skill_check_game(0, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(0, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        game.player.hp = 5;
        give_inventory_item(game, ITEM_ID, "demo.item.resonance-stabilizer");
    }
    assert_eq!(tinkerer.rng_draw_counter(), vanguard.rng_draw_counter());

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.device-success");
    let failure = check_resolution(&vanguard_update, "skill.device-failure");

    assert_check(success, "demo.skill.device", 69, 60, 32, Some(54), 45);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.device", 16, 60, 32, Some(9), 45);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert_eq!(tinkerer.player.hp, 11);
    assert_eq!(vanguard.player.hp, 5);
    assert!(!tinkerer.items.iter().any(|item| item.id == ITEM_ID));
    assert!(vanguard.items.iter().any(|item| item.id == ITEM_ID));
    assert!(
        tinkerer
            .item_knowledge
            .get("demo.item.resonance-stabilizer")
            .is_some_and(|knowledge| knowledge.tried && knowledge.aware)
    );
    assert!(
        vanguard
            .item_knowledge
            .get("demo.item.resonance-stabilizer")
            .is_some_and(|knowledge| knowledge.tried && !knowledge.aware)
    );
}

#[test]
fn charged_device_spends_instance_charges_only_after_a_successful_check_and_round_trips() {
    const ITEM_ID: &str = "test.item.resonance-mender.1";
    let mut tinkerer = skill_check_game(0, "demo.build.tinkerer");
    tinkerer.player.hp = 1;
    give_inventory_item(&mut tinkerer, ITEM_ID, "demo.item.resonance-mender");

    let before = tinkerer.snapshot();
    let mender = before
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged device should be carried");
    assert_eq!(mender.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(mender.charges, None);
    assert!(mender.usable);

    let update = dispatch_next(
        &mut tinkerer,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "skill.device-success")
    );
    let used = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-heal")
        .expect("successful device should apply its healing dice");
    assert!(matches!(
        used.outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if (2..=8).contains(&resolution.requested)
                && resolution.applied == resolution.requested
    ));
    let mender = update
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged device should not be consumed");
    assert_eq!(mender.quantity, 1);
    assert_eq!(
        mender.charges,
        Some(ItemChargesDto {
            current: 2,
            maximum: 3,
        })
    );
    let restored =
        Game::from_save(tinkerer.to_save()).expect("charged item state should survive reload");
    assert_eq!(restored.snapshot(), tinkerer.snapshot());

    let mut invalid = tinkerer.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("charged item should be saved")
        .charges = Some(ItemChargesDto {
        current: 4,
        maximum: 3,
    });
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));

    let mut missing = tinkerer.to_save();
    missing
        .inventory
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("charged item should be saved")
        .charges = None;
    assert!(matches!(
        Game::from_save(missing),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));
}

#[test]
fn failed_and_depleted_device_attempts_preserve_charges() {
    const ITEM_ID: &str = "test.item.resonance-mender.1";
    let mut vanguard = skill_check_game(0, "demo.build.vanguard");
    vanguard.player.hp = 1;
    give_inventory_item(&mut vanguard, ITEM_ID, "demo.item.resonance-mender");

    let failed = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert!(
        failed
            .events
            .iter()
            .any(|event| event.kind == "skill.device-failure")
    );
    assert_eq!(
        failed
            .inventory
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("failed device should remain carried")
            .charges,
        None
    );
    assert_eq!(
        vanguard
            .items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("failed device should retain its instance state")
            .charges,
        Some(ItemChargesDto {
            current: 3,
            maximum: 3,
        })
    );

    vanguard
        .items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("device should remain after failure")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 3,
    });
    let draws = vanguard.rng_draw_counter();
    let world_tick = vanguard.world_tick;
    let depleted = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(depleted.events[0].kind, "item.use-unavailable");
    assert_eq!(vanguard.rng_draw_counter(), draws);
    assert_eq!(vanguard.world_tick, world_tick);
    let depleted_device = depleted
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("depleted device should remain carried");
    assert!(!depleted_device.usable);
    assert_eq!(depleted_device.charges, None);
    assert_eq!(
        vanguard
            .items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("depleted device should retain its instance state")
            .charges,
        Some(ItemChargesDto {
            current: 0,
            maximum: 3,
        })
    );
}

#[test]
fn restorative_item_sequence_recovers_resource_then_removes_status() {
    const ITEM_ID: &str = "test.item.clarity-draught.1";
    let mut game = skill_check_game(19, "demo.build.scholar");
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana")
        .current = 0;
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        remaining_ticks: 20,
        intensity: 1,
        source_id: Some("test".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert!(game.resources["demo.resource.mana"].current > 0);
    assert!(!game.player_has_status_kind(STATUS_CONFUSION));
    let effect_events = update
        .events
        .iter()
        .filter(|event| event.kind.starts_with("item.use-"))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        effect_events,
        vec!["item.use-resource-restored", "item.use-status-removed"]
    );
}

#[test]
fn full_resource_restoration_is_deterministic_and_round_trips() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(23, "demo.build.scholar");
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana");
    mana.current = 1;
    let maximum = mana.maximum;
    game.player.statuses.push(StatusInstance {
        kind_id: "rfb.status.berserk".to_owned(),
        remaining_ticks: 20,
        intensity: 1,
        source_id: Some("test".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.resources["demo.resource.mana"].current, maximum);
    assert!(!game.player_has_status_kind("rfb.status.berserk"));
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(update.events.iter().any(|event| {
        matches!(
            &event.outcome,
            Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                if resolution.before == 1
                    && resolution.after == maximum
                    && resolution.recovered == maximum - 1
        )
    }));
    let restored = Game::from_save(game.to_save()).expect("restored resource state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn successful_restoration_reveals_later_no_effect_events() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(27, "demo.build.scholar");
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana")
        .current = 0;
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    let status_event = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-status-no-effect")
        .expect("the absent berserk status should produce a no-effect event");
    assert_eq!(
        status_event.args["nameKey"],
        "item-demo-perfect-focus-elixir-name"
    );
}

#[test]
fn missing_player_resource_consumes_restorative_without_claiming_awareness() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(29, "demo.build.vanguard");
    assert!(!game.resources.contains_key("demo.resource.mana"));
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-resource-no-effect"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                    if resolution.before == 0
                        && resolution.after == 0
                        && resolution.recovered == 0
            )
    }));
    assert!(
        game.item_knowledge
            .get("demo.item.perfect-focus-elixir")
            .is_some_and(|knowledge| knowledge.tried && !knowledge.aware)
    );
}

#[test]
fn appraisal_scroll_targets_an_item_without_drawing_rng() {
    const SCROLL_ID: &str = "test.item.appraisal-scroll.1";
    const TARGET_ID: &str = "test.item.appraisal-target.1";
    let mut game = skill_check_game(31, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");
    give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
    let before = game.snapshot();
    let scroll = before
        .inventory
        .iter()
        .find(|item| item.id == SCROLL_ID)
        .expect("appraisal scroll should be carried");
    assert_eq!(scroll.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(scroll.use_target_spec, Some(item_target_spec()));
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(
        game.item_knowledge_dto("demo.item.appraisal-scroll"),
        ItemKnowledgeDto::Aware
    );
    let target = update
        .inventory
        .iter()
        .find(|item| item.id == TARGET_ID)
        .expect("identified target should remain carried");
    assert_eq!(target.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(target.identification, ItemIdentificationDto::Appraised);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-identified"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ItemIdentify { resolution })
                    if resolution.item_id == TARGET_ID
                        && !resolution.full
                        && resolution.changed
            )
    }));
}

#[test]
fn revelation_scroll_fully_identifies_affixes_and_round_trips() {
    const SCROLL_ID: &str = "test.item.revelation-scroll.1";
    const TARGET_ID: &str = "test.item.revelation-target.1";
    let mut game = skill_check_game(37, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.revelation-scroll");
    game.items
        .iter_mut()
        .find(|item| item.id == SCROLL_ID)
        .expect("revelation scroll should be carried")
        .quantity = 2;
    game.items.push(ItemInstance {
        id: TARGET_ID.to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Exceptional,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        fuel: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == SCROLL_ID)
            .expect("one scroll should remain")
            .quantity,
        1
    );
    let target = update
        .inventory
        .iter()
        .find(|item| item.id == TARGET_ID)
        .expect("fully identified target should remain carried");
    assert_eq!(target.identification, ItemIdentificationDto::Identified);
    assert_eq!(target.quality, Some(ItemQualityDto::Exceptional));
    assert_eq!(target.known_properties.len(), 1);
    assert_eq!(
        target.known_properties[0].affix_id,
        "demo.affix.adaptive-echo"
    );
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-fully-identified"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ItemIdentify { resolution })
                    if resolution.item_id == TARGET_ID && resolution.full
            )
    }));
    let restored = Game::from_save(game.to_save()).expect("item knowledge should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn identify_scroll_rejects_missing_and_self_targets_before_consumption() {
    const SCROLL_ID: &str = "test.item.invalid-identify-scroll.1";
    let mut game = skill_check_game(41, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");

    for target_item_id in ["missing.item", SCROLL_ID] {
        let draws_before = game.rng_draw_counter();
        let tick_before = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: target_item_id.to_owned(),
                }),
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert_eq!(game.world_tick, tick_before);
        assert!(game.items.iter().any(|item| item.id == SCROLL_ID));
        assert_eq!(
            game.item_knowledge_dto("demo.item.appraisal-scroll"),
            ItemKnowledgeDto::Unknown
        );
    }
}

#[test]
fn enchantment_scroll_succeeds_consumes_on_failure_and_round_trips() {
    const SCROLL_ID: &str = "test.item.accuracy-scroll.1";
    const TARGET_ID: &str = "test.item.enchantment-target.1";
    let mut game = skill_check_game(0, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
    game.rng = RfbRng::seeded(0);

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    let resolution = update
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::ItemEnchantment { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("enchantment should emit a structured resolution");
    assert_eq!(update.events[0].kind, "item.use-enchanted");
    assert_eq!(resolution.to_hit.attempts, 1);
    assert_eq!(resolution.to_hit.successes, 1);
    assert_eq!(resolution.to_hit.before, 0);
    assert_eq!(resolution.to_hit.after, 1);
    assert_eq!(resolution.to_damage.attempts, 0);
    assert_eq!(resolution.to_armor.attempts, 0);

    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    game.items
        .iter_mut()
        .find(|item| item.id == TARGET_ID)
        .expect("target should remain carried")
        .enchantments
        .to_hit = 15;
    game.rng = RfbRng::seeded(0);
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );
    assert_eq!(update.events[0].kind, "item.use-enchantment-failed");
    assert_eq!(game.rng_draw_counter() - draws_before, 2);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == TARGET_ID)
            .expect("target should remain carried")
            .enchantments
            .to_hit,
        15
    );

    let restored = Game::from_save(game.to_save()).expect("enchantments should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
    let mut invalid = game.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == TARGET_ID)
        .expect("target should be saved")
        .enchantments
        .to_hit = 16;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item enchantment state is invalid"))
    ));
}

#[test]
fn masterwork_weapon_scroll_rolls_both_components_deterministically() {
    fn run(seed: u64) -> (ItemEnchantmentResolutionDto, u64) {
        const SCROLL_ID: &str = "test.item.masterwork-weapon-scroll.1";
        const TARGET_ID: &str = "test.item.masterwork-target.1";
        let mut game = skill_check_game(seed, "demo.build.scholar");
        give_inventory_item(&mut game, SCROLL_ID, "demo.item.masterwork-weapon-scroll");
        give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
        game.rng = RfbRng::seeded(seed);
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: TARGET_ID.to_owned(),
                }),
            },
        );
        let resolution = update
            .events
            .iter()
            .find_map(|event| match &event.outcome {
                Some(GameEventOutcomeDto::ItemEnchantment { resolution }) => {
                    Some(resolution.clone())
                }
                _ => None,
            })
            .expect("masterwork enchantment should emit a resolution");
        (resolution, game.rng_draw_counter())
    }

    let left = run(37);
    let right = run(37);
    assert_eq!(left, right);
    assert!((4..=6).contains(&left.0.to_hit.attempts));
    assert!((4..=6).contains(&left.0.to_damage.attempts));
    assert_eq!(left.0.to_armor.attempts, 0);
    assert_eq!(left.0.to_hit.successes, left.0.to_hit.after);
    assert_eq!(left.0.to_damage.successes, left.0.to_damage.after);
}

#[test]
fn enchantment_artifact_and_ammunition_pile_gates_follow_original_order() {
    let artifact_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.scholar");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 1, false, false);
        let mut artifact = skill_check_game(*seed, "demo.build.scholar");
        artifact.rng = RfbRng::seeded(*seed);
        let artifact = artifact.resolve_item_enchantment_component(0, 1, 1, false, true);
        ordinary.successes == 1 && artifact.successes == 0
    });
    assert_eq!(artifact_seed, Some(0));

    let ammunition_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.scholar");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 20, false, false);
        let mut ammunition = skill_check_game(*seed, "demo.build.scholar");
        ammunition.rng = RfbRng::seeded(*seed);
        let ammunition = ammunition.resolve_item_enchantment_component(0, 1, 20, true, false);
        ordinary.successes == 0 && ammunition.successes == 1
    });
    assert_eq!(ammunition_seed, Some(0));
}

#[test]
fn enchantment_scroll_rejects_invalid_targets_atomically() {
    const SCROLL_ID: &str = "test.item.invalid-enchantment-scroll.1";
    const ARMOR_ID: &str = "test.item.invalid-enchantment-armor.1";
    let mut game = skill_check_game(41, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    give_inventory_item(&mut game, ARMOR_ID, "demo.item.resonance-mail");

    for target in [
        None,
        Some(TargetSelection::Item {
            item_id: "missing.item".to_owned(),
        }),
        Some(TargetSelection::Item {
            item_id: SCROLL_ID.to_owned(),
        }),
        Some(TargetSelection::Item {
            item_id: ARMOR_ID.to_owned(),
        }),
        Some(TargetSelection::SelfTarget),
    ] {
        let draws_before = game.rng_draw_counter();
        let tick_before = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target,
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert_eq!(game.world_tick, tick_before);
        assert!(game.items.iter().any(|item| item.id == SCROLL_ID));
        assert_eq!(
            game.item_knowledge_dto("demo.item.accuracy-scroll"),
            ItemKnowledgeDto::Unknown
        );
    }
}

#[test]
fn enchantments_feed_combat_armor_and_legacy_save_projection() {
    let mut game = skill_check_game(53, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    for (id, kind_id, location, enchantments) in [
        (
            "test.item.enchanted-weapon",
            "demo.item.adaptive-glaive",
            ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 3,
                to_damage: 4,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-launcher",
            "demo.item.resonance-sling",
            ItemLocation::Equipped {
                slot_id: "launcher".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 2,
                to_damage: 3,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-ammunition",
            "demo.item.resonance-pellet",
            ItemLocation::Inventory,
            ItemEnchantmentsDto {
                to_hit: 5,
                to_damage: 6,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-throwable",
            "demo.item.luminous-shard",
            ItemLocation::Inventory,
            ItemEnchantmentsDto {
                to_hit: 7,
                to_damage: 8,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-armor",
            "demo.item.resonance-mail",
            ItemLocation::Equipped {
                slot_id: "body".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 0,
                to_damage: 0,
                to_armor: 5,
            },
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("test item should be carried");
        item.location = location;
        item.enchantments = enchantments;
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.enchanted-ammunition")
        .expect("ammunition should exist")
        .quantity = 20;
    for item in game.items.iter_mut().filter(|item| {
        item.kind_id == "demo.item.resonance-pellet"
            && item.location == ItemLocation::Inventory
            && item.quantity > 0
    }) {
        item.enchantments = ItemEnchantmentsDto {
            to_hit: 5,
            to_damage: 6,
            to_armor: 0,
        };
    }
    for id in [
        "test.item.enchanted-weapon",
        "test.item.enchanted-launcher",
        "test.item.enchanted-armor",
    ] {
        game.item_property_knowledge.insert(
            id.to_owned(),
            ItemPropertyKnowledgeState {
                appraised: true,
                identified: true,
                known_affix_ids: BTreeSet::new(),
            },
        );
    }
    game.mark_item_aware("demo.item.luminous-shard");

    let snapshot = game.snapshot();
    let weapon = snapshot
        .equipment
        .iter()
        .find(|item| item.id == "test.item.enchanted-weapon")
        .expect("weapon should be equipped");
    assert_eq!(weapon.enchantments.to_hit, 3);
    assert_eq!(
        weapon.melee_profile.as_ref().expect("melee profile").to_hit,
        5
    );
    assert_eq!(
        weapon
            .melee_profile
            .as_ref()
            .expect("melee profile")
            .to_damage,
        6
    );
    assert_eq!(snapshot.player.melee_profile.to_damage, 6);
    let projectile = snapshot
        .player
        .projectile_profile
        .as_ref()
        .expect("launcher should expose a projectile profile");
    assert_eq!(projectile.to_hit, 37);
    assert_eq!(projectile.to_damage, 10);
    let throwable = snapshot
        .inventory
        .iter()
        .find(|item| item.id == "test.item.enchanted-throwable")
        .and_then(|item| item.throw_profile.as_ref())
        .expect("throwable should expose a throw profile");
    assert_eq!(throwable.to_hit, 37);
    assert_eq!(throwable.to_damage, 8);
    let stats = game.player_derived_stats();
    assert!(stats.armor_class.contributions.iter().any(|contribution| {
        contribution.source_id == "test.item.enchanted-armor" && contribution.amount == 90
    }));

    let restored = Game::from_save(game.to_save()).expect("all item locations should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());

    let mut legacy_json = serde_json::to_value(game.to_save()).expect("save should serialize");
    for field in ["items", "inventory", "equipment", "carriedItems"] {
        if let Some(items) = legacy_json
            .get_mut(field)
            .and_then(serde_json::Value::as_array_mut)
        {
            for item in items {
                item.as_object_mut()
                    .expect("saved item should be an object")
                    .remove("enchantments");
            }
        }
    }
    let legacy: SavePayloadV1 =
        serde_json::from_value(legacy_json).expect("missing enchantments should default");
    let migrated = Game::from_save(legacy).expect("legacy save should load");
    assert!(
        migrated
            .items
            .iter()
            .all(|item| item.enchantments.is_empty())
    );
}

#[test]
fn curse_scroll_lands_on_equipped_weapon_and_artifact_can_resist() {
    fn run(resisted: bool) -> (Game, GameUpdate, u64) {
        const SCROLL_ID: &str = "test.item.weapon-blight-scroll.1";
        const WEAPON_ID: &str = "test.item.relic-blade.1";
        let mut game = skill_check_game(61, "demo.build.scholar");
        for item in game
            .items
            .iter_mut()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        {
            item.location = ItemLocation::Inventory;
        }
        give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
        give_inventory_item(&mut game, WEAPON_ID, "demo.item.relic-blade");
        game.items
            .iter_mut()
            .find(|item| item.id == WEAPON_ID)
            .expect("relic blade should exist")
            .location = ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        };
        game.debug_set_item_curses_land(!resisted);
        game.debug_set_item_curses_resisted(resisted);
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::SelfTarget),
            },
        );
        (game, update, draws_before)
    }

    let (landed, update, draws_before) = run(false);
    assert_eq!(landed.rng_draw_counter(), draws_before);
    assert_eq!(update.events[0].kind, "item.use-cursed");
    assert_eq!(
        landed
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    assert_eq!(
        landed.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(update.events.iter().any(|event| {
        matches!(
            &event.outcome,
            Some(GameEventOutcomeDto::ItemCurse { resolution })
                if resolution.item_id.as_deref() == Some("test.item.relic-blade.1")
                    && resolution.before.is_none()
                    && resolution.after == Some(ItemCurseSeverityDto::Normal)
                    && !resolution.resisted
        )
    }));

    let (resisted, update, draws_before) = run(true);
    assert_eq!(resisted.rng_draw_counter(), draws_before);
    assert_eq!(update.events[0].kind, "item.use-curse-resisted");
    assert_eq!(
        resisted
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        None
    );
    assert_eq!(
        resisted.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn curse_scroll_without_a_matching_equipped_item_consumes_without_rng_or_awareness() {
    const SCROLL_ID: &str = "test.item.weapon-blight-scroll.no-target";
    let mut game = skill_check_game(67, "demo.build.scholar");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(update.events[0].kind, "item.use-curse-no-target");
    assert_eq!(
        game.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Tried
    );
}

#[test]
fn cleansing_scrolls_respect_heavy_and_permanent_curse_boundaries() {
    const NORMAL_ID: &str = "test.item.normal-curse";
    const HEAVY_ID: &str = "test.item.heavy-curse";
    const PERMANENT_ID: &str = "test.item.permanent-curse";
    let mut game = skill_check_game(71, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    for (id, kind_id, slot_id, curse) in [
        (
            NORMAL_ID,
            "demo.item.relic-blade",
            "weapon",
            ItemCurseSeverityDto::Normal,
        ),
        (
            HEAVY_ID,
            "demo.item.burdened-mail",
            "body",
            ItemCurseSeverityDto::Heavy,
        ),
        (
            PERMANENT_ID,
            "demo.item.sealed-amulet",
            "amulet",
            ItemCurseSeverityDto::Permanent,
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("curse test item should exist");
        item.location = ItemLocation::Equipped {
            slot_id: slot_id.to_owned(),
        };
        item.curse = Some(curse);
    }
    give_inventory_item(
        &mut game,
        "test.item.cleansing-scroll.1",
        "demo.item.cleansing-scroll",
    );
    let ordinary = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(ordinary.events[0].kind, "item.use-curses-removed");
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == NORMAL_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );

    give_inventory_item(
        &mut game,
        "test.item.greater-cleansing-scroll.1",
        "demo.item.greater-cleansing-scroll",
    );
    let greater = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.greater-cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(greater.events[0].kind, "item.use-curses-removed");
    let resolution = greater
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::ItemCurseRemoval { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("greater cleansing should emit a structured resolution");
    assert_eq!(resolution.removed_item_ids, [HEAVY_ID]);
    assert_eq!(resolution.retained_permanent_item_ids, [PERMANENT_ID]);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("curse severities should round-trip");
    for (item_id, expected) in [
        (HEAVY_ID, None),
        (PERMANENT_ID, Some(ItemCurseSeverityDto::Permanent)),
    ] {
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .curse,
            expected
        );
    }
}

#[test]
fn cursed_equipment_cannot_be_unequipped_or_replaced_and_rejection_is_zero_time() {
    const CURSED_ID: &str = "test.item.cursed-mail";
    const REPLACEMENT_ID: &str = "test.item.replacement-mail";
    let mut game = skill_check_game(73, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    give_inventory_item(&mut game, CURSED_ID, "demo.item.burdened-mail");
    let cursed = game
        .items
        .iter_mut()
        .find(|item| item.id == CURSED_ID)
        .unwrap();
    cursed.location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    cursed.curse = Some(ItemCurseSeverityDto::Heavy);
    give_inventory_item(&mut game, REPLACEMENT_ID, "demo.item.resonance-mail");

    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let unequip = dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "body".to_owned(),
        },
    );
    assert_eq!(unequip.events[0].kind, "item.unequip.cursed");
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);

    let replace = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: REPLACEMENT_ID.to_owned(),
            slot_id: None,
        },
    );
    assert_eq!(replace.events[0].kind, "item.unequip.cursed");
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(matches!(
        game.items
            .iter()
            .find(|item| item.id == CURSED_ID)
            .unwrap()
            .location,
        ItemLocation::Equipped { .. }
    ));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == REPLACEMENT_ID)
            .unwrap()
            .location,
        ItemLocation::Inventory
    );
}

#[test]
fn curse_state_round_trips_all_item_locations_migrates_and_prevents_stacking() {
    let mut game = Game::new(79);
    let carrier_id = game.entities[0].id.clone();
    for (id, kind_id, location, curse) in [
        (
            "test.item.curse-ground",
            "demo.item.resonance-mail",
            ItemLocation::Ground(game.player.position),
            ItemCurseSeverityDto::Normal,
        ),
        (
            "test.item.curse-inventory",
            "demo.item.burdened-mail",
            ItemLocation::Inventory,
            ItemCurseSeverityDto::Heavy,
        ),
        (
            "test.item.curse-equipment",
            "demo.item.sealed-amulet",
            ItemLocation::Equipped {
                slot_id: "amulet".to_owned(),
            },
            ItemCurseSeverityDto::Permanent,
        ),
        (
            "test.item.curse-carried",
            "demo.item.relic-blade",
            ItemLocation::CarriedBy {
                actor_id: carrier_id.clone(),
            },
            ItemCurseSeverityDto::Normal,
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game.items.iter_mut().find(|item| item.id == id).unwrap();
        item.location = location;
        item.curse = Some(curse);
    }
    let saved = game.to_save();
    assert_eq!(
        saved.items.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    assert_eq!(
        saved.inventory.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(
        saved.equipment.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    assert_eq!(
        saved.carried_items.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    let restored = Game::from_save(saved.clone()).expect("all curse locations should reload");
    for (item_id, expected) in [
        ("test.item.curse-ground", ItemCurseSeverityDto::Normal),
        ("test.item.curse-inventory", ItemCurseSeverityDto::Heavy),
        ("test.item.curse-equipment", ItemCurseSeverityDto::Permanent),
        ("test.item.curse-carried", ItemCurseSeverityDto::Normal),
    ] {
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .curse,
            Some(expected)
        );
    }

    let mut legacy_json = serde_json::to_value(game.to_save()).expect("save should serialize");
    for field in ["items", "inventory", "equipment", "carriedItems"] {
        for item in legacy_json[field]
            .as_array_mut()
            .expect("item save field should be an array")
        {
            item.as_object_mut()
                .expect("saved item should be an object")
                .remove("curse");
        }
    }
    let legacy: SavePayloadV1 =
        serde_json::from_value(legacy_json).expect("missing curse should default");
    let migrated = Game::from_save(legacy).expect("legacy curse state should load");
    assert!(migrated.items.iter().all(|item| item.curse.is_none()));

    let mut stack_game = skill_check_game(83, "demo.build.vanguard");
    give_inventory_item(
        &mut stack_game,
        "test.item.stack-clean",
        "demo.item.resonance-mail",
    );
    give_inventory_item(
        &mut stack_game,
        "test.item.stack-cursed",
        "demo.item.resonance-mail",
    );
    let cursed = stack_game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.stack-cursed")
        .unwrap();
    cursed.curse = Some(ItemCurseSeverityDto::Normal);
    cursed.location = ItemLocation::Ground(stack_game.player.position);
    dispatch_next(&mut stack_game, GameCommand::PickUp);
    assert_eq!(
        stack_game
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.resonance-mail")
            .count(),
        2
    );
}

#[test]
fn dynamic_device_generation_filters_by_depth_is_weighted_and_round_trips() {
    const WAND_ID: &str = "demo.item.resonance-wand";
    let content = load_built_in_content().expect("built-in content should load");
    let mut shallow_rng = RfbRng::seeded(11);
    let (shallow_activation, shallow_charges) =
        initial_item_runtime_state(&content, &mut shallow_rng, WAND_ID, 1);
    let shallow_activation = shallow_activation.expect("wand should materialize an activation");
    assert_eq!(
        shallow_activation.profile_id,
        "demo.device-activation.spark-bolt"
    );
    let shallow_charges = shallow_charges.expect("wand should materialize charges");
    assert!((12..=24).contains(&shallow_charges.maximum));
    assert!((shallow_activation.cost..=shallow_charges.maximum).contains(&shallow_charges.current));

    let mut selected = BTreeSet::new();
    for seed in 0..64 {
        let mut left = RfbRng::seeded(seed);
        let mut right = RfbRng::seeded(seed);
        let left_state = initial_item_runtime_state(&content, &mut left, WAND_ID, 20);
        let right_state = initial_item_runtime_state(&content, &mut right, WAND_ID, 20);
        assert_eq!(left_state, right_state);
        selected.insert(
            left_state
                .0
                .expect("deep wand should materialize an activation")
                .profile_id,
        );
    }
    assert_eq!(
        selected,
        BTreeSet::from([
            "demo.device-activation.frost-bolt".to_owned(),
            "demo.device-activation.spark-bolt".to_owned(),
        ])
    );

    let mut game = skill_check_game(11, "demo.build.tinkerer");
    give_inventory_item(&mut game, "test.item.dynamic-wand", WAND_ID);
    let restored = Game::from_save(game.to_save()).expect("dynamic device should round-trip");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == "test.item.dynamic-wand")
        .expect("dynamic device should remain in inventory");
    assert_eq!(
        restored_item
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.spark-bolt")
    );
}

#[test]
fn dynamic_wand_validates_target_before_check_and_spends_only_on_success() {
    const ITEM_ID: &str = "test.item.dynamic-wand";
    let mut game = Game::new_with_build(0, "demo.build.tinkerer")
        .expect("device specialist build should create");
    game.player.position = Position { x: 7, y: 5 };
    give_inventory_item(&mut game, ITEM_ID, "demo.item.resonance-wand");
    let charges_before = game
        .items
        .iter()
        .find(|item| item.id == ITEM_ID)
        .and_then(|item| item.charges)
        .expect("dynamic wand should carry charges");
    let draws_before = game.rng.draw_counter;
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::SelfTarget),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("invalid target should be handled");
    assert_eq!(events, vec![DomainEvent::ItemUseUnavailable]);
    assert_eq!(game.rng.draw_counter, draws_before);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges),
        Some(charges_before)
    );

    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("valid wand activation should resolve");
    assert!(events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::ItemActivationHit { .. } | DomainEvent::ItemActivationSlew { .. }
        )
    }));
    let item = game
        .items
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged wand should remain in inventory");
    let activation = item
        .activation
        .as_ref()
        .expect("wand activation should remain materialized");
    assert_eq!(
        item.charges.expect("wand charges should remain").current,
        charges_before.current - activation.cost
    );
}

#[test]
fn saving_throw_skill_check_resists_or_applies_the_same_trap() {
    let trap_position = Position { x: 4, y: 3 };
    let mut tinkerer = skill_check_game(2, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(2, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        replace_terrain(game, trap_position, "demo.terrain.trap-resonance-ward");
    }
    let tinkerer_hp = tinkerer.player.hp;
    let vanguard_hp = vanguard.player.hp;

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.saving-throw-success");
    let failure = check_resolution(&vanguard_update, "skill.saving-throw-failure");

    assert_check(success, "demo.skill.saving-throw", 45, 40, 13, Some(33), 30);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.saving-throw", 29, 40, 13, Some(20), 30);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert_eq!(tinkerer.player.hp, tinkerer_hp);
    assert!(vanguard.player.hp < vanguard_hp);
    assert!(tinkerer.revealed_terrain.contains(&trap_position));
    assert!(vanguard.revealed_terrain.contains(&trap_position));
    assert!(
        !tinkerer_update
            .events
            .iter()
            .any(|event| event.kind == "terrain.trap-triggered")
    );
    assert!(
        vanguard_update
            .events
            .iter()
            .any(|event| event.kind == "terrain.trap-triggered")
    );
}

#[test]
fn passive_perception_skill_check_reveals_only_for_the_high_skill_build() {
    let rune_position = Position { x: 5, y: 3 };
    let mut tinkerer = skill_check_game(1, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(1, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        replace_terrain(game, rune_position, "demo.terrain.echo-rune-hidden");
    }

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.perception-success");
    let failure = check_resolution(&vanguard_update, "skill.perception-failure");

    assert_check(success, "demo.skill.perception", 25, 24, 83, Some(21), 18);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.perception", 4, 24, 83, Some(3), 18);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert!(tinkerer.revealed_terrain.contains(&rune_position));
    assert!(!vanguard.revealed_terrain.contains(&rune_position));
    assert_eq!(
        tinkerer.known_terrain_at(rune_position),
        "demo.terrain.echo-rune-hidden"
    );
    assert_eq!(
        vanguard.known_terrain_at(rune_position),
        "demo.terrain.wall"
    );
}

#[test]
fn stealth_skill_check_controls_alertness_and_alerted_save_compatibility() {
    const LISTENER_ID: &str = "test.monster.echo-listener.1";
    let listener_position = Position { x: 7, y: 3 };
    let mut tinkerer = skill_check_game(5, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(5, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        game.push_generated_actor(
            LISTENER_ID.to_owned(),
            "demo.actor.echo-listener",
            listener_position,
        );
    }

    let tinkerer_update = dispatch_next(&mut tinkerer, GameCommand::Wait);
    let vanguard_update = dispatch_next(&mut vanguard, GameCommand::Wait);
    let success = check_resolution(&tinkerer_update, "skill.stealth-success");
    let failure = check_resolution(&vanguard_update, "skill.stealth-failure");

    assert_check(success, "demo.skill.stealth", 7, 7, 93, Some(5), 5);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.stealth", 1, 7, 93, Some(0), 5);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert!(
        tinkerer
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| !entity.alerted && entity.position == listener_position)
    );
    assert!(
        vanguard
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted && entity.position != listener_position)
    );

    let saved = vanguard.to_save();
    assert!(
        saved
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted == Some(true))
    );
    let restored = Game::from_save(saved.clone()).expect("alerted actor should reload");
    assert_eq!(restored.state_hash(), vanguard.state_hash());
    assert!(
        restored
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted)
    );

    let mut legacy = saved;
    legacy
        .entities
        .iter_mut()
        .find(|entity| entity.id == LISTENER_ID)
        .expect("listener save should exist")
        .alerted = None;
    let migrated = Game::from_save(legacy).expect("missing alert state should use content default");
    assert!(
        migrated
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| !entity.alerted)
    );
}

#[test]
fn dynamic_device_recovery_is_inventory_only_deterministic_and_rod_fast() {
    let mut game =
        Game::new_with_build(11, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.recovery.rod", "demo.item.resonance-rod", 1)
        .expect("rod should generate");
    game.debug_add_generated_inventory_item(
        "test.item.recovery.wand",
        "demo.item.resonance-wand",
        1,
    )
    .expect("wand should generate");
    for item_id in ["test.item.recovery.rod", "test.item.recovery.wand"] {
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("generated device");
        item.charges = Some(ItemChargesDto {
            current: 0,
            maximum: 20,
        });
        item.device_recovery_progress = 0;
    }

    for world_tick in 1..=50 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.rod")
            .and_then(|item| item.charges)
            .expect("rod charges")
            .current,
        10
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.wand")
            .and_then(|item| item.charges)
            .expect("wand charges")
            .current,
        1
    );

    let wand = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.recovery.wand")
        .expect("wand");
    wand.location = ItemLocation::Ground(Position { x: 0, y: 0 });
    for world_tick in 51..=100 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.wand")
            .and_then(|item| item.charges)
            .expect("wand charges")
            .current,
        1
    );
}

#[test]
fn device_recovery_remainder_round_trips_and_old_saves_default_to_zero() {
    let mut game =
        Game::new_with_build(12, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.remainder", "demo.item.resonance-rod", 1)
        .expect("rod should generate");
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.remainder")
        .expect("rod");
    item.charges = Some(ItemChargesDto {
        current: 0,
        maximum: 20,
    });
    for world_tick in 1..=3 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.remainder")
            .expect("rod")
            .device_recovery_progress,
        600
    );

    let save = game.to_save();
    let restored = Game::from_save(save.clone()).expect("recovery remainder should reload");
    assert_eq!(restored.snapshot().state_hash, game.snapshot().state_hash);
    let mut old_json = serde_json::to_value(save).expect("save should serialize");
    let inventory = old_json["inventory"]
        .as_array_mut()
        .expect("save inventory should be an array");
    for item in inventory {
        item.as_object_mut()
            .expect("inventory item should be an object")
            .remove("deviceRecoveryProgress");
    }
    let old_save = serde_json::from_value(old_json).expect("legacy save should deserialize");
    let migrated = Game::from_save(old_save).expect("missing recovery remainder should migrate");
    assert_eq!(
        migrated
            .items
            .iter()
            .find(|item| item.id == "test.item.remainder")
            .expect("rod")
            .device_recovery_progress,
        0
    );

    let mut invalid = game.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == "test.item.remainder")
        .expect("saved rod")
        .device_recovery_progress = 1_000;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));
}

#[test]
fn recharge_invalid_transactions_are_zero_time_and_zero_rng() {
    let mut game =
        Game::new_with_build(13, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.full", "demo.item.resonance-staff", 1)
        .expect("staff should generate");
    let target = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.full")
        .expect("staff");
    target.charges = Some(ItemChargesDto {
        current: 24,
        maximum: 24,
    });
    let world_tick = game.world_tick;
    let draws = game.rng.draw_counter;
    let update = dispatch_next(
        &mut game,
        GameCommand::RechargeItem {
            target_item_id: "test.item.full".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    assert_eq!(update.world_tick, world_tick);
    assert_eq!(game.rng.draw_counter, draws);
    assert_eq!(update.events[0].kind, "device.recharge-unavailable");
    assert_eq!(
        update.events[0].args.get("reason").map(String::as_str),
        Some("target-not-rechargeable")
    );
}

#[test]
fn resource_recharge_succeeds_and_failure_clears_target_energy() {
    let mut success =
        Game::new_with_build(14, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut success);
    success
        .debug_add_generated_inventory_item(
            "test.item.resource-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    success
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.resource-target")
        .expect("staff")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 24,
    });
    success.debug_set_recharge_attempts_succeed(true);
    let resource_before = success.resources["demo.resource.resonance"].current;
    let update = dispatch_next(
        &mut success,
        GameCommand::RechargeItem {
            target_item_id: "test.item.resource-target".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    let attempted = resource_before.min(24);
    assert_eq!(update.events[0].kind, "device.recharge-success");
    assert_eq!(
        success
            .items
            .iter()
            .find(|item| item.id == "test.item.resource-target")
            .and_then(|item| item.charges)
            .expect("staff charges")
            .current,
        attempted
    );
    assert_eq!(
        success.resources["demo.resource.resonance"].current,
        resource_before - attempted
    );

    let mut failure =
        Game::new_with_build(15, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut failure);
    failure
        .debug_add_generated_inventory_item(
            "test.item.failed-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    failure
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.failed-target")
        .expect("staff")
        .charges = Some(ItemChargesDto {
        current: 5,
        maximum: 24,
    });
    let failure_seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(5) == 0
        })
        .expect("one seed should fail recharge");
    failure.rng = RfbRng::seeded(failure_seed);
    let update = dispatch_next(
        &mut failure,
        GameCommand::RechargeItem {
            target_item_id: "test.item.failed-target".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    assert_eq!(update.events[0].kind, "device.recharge-failure");
    assert_eq!(
        failure
            .items
            .iter()
            .find(|item| item.id == "test.item.failed-target")
            .and_then(|item| item.charges)
            .expect("staff charges")
            .current,
        0
    );
}

#[test]
fn device_source_recharge_can_survive_be_destroyed_or_protect_artifacts() {
    let prepare = |seed| {
        let mut game = Game::new_with_build(seed, "demo.build.tinkerer")
            .expect("tinkerer build should create");
        clear_monsters(&mut game);
        game.debug_add_generated_inventory_item(
            "test.item.device-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
        game.debug_add_generated_inventory_item(
            "test.item.device-source",
            "demo.item.resonance-wand",
            1,
        )
        .expect("wand should generate");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.device-target")
            .expect("target")
            .charges = Some(ItemChargesDto {
            current: 0,
            maximum: 24,
        });
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.device-source")
            .expect("source")
            .charges = Some(ItemChargesDto {
            current: 5,
            maximum: 24,
        });
        game.debug_set_recharge_attempts_succeed(true);
        game
    };

    let mut surviving = prepare(16);
    surviving.debug_set_recharge_sources_survive(true);
    let update = dispatch_next(
        &mut surviving,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(update.events[0].kind, "device.recharge-success");
    assert_eq!(
        surviving
            .items
            .iter()
            .find(|item| item.id == "test.item.device-target")
            .and_then(|item| item.charges)
            .expect("target charges")
            .current,
        5
    );
    assert!(
        surviving
            .items
            .iter()
            .any(|item| item.id == "test.item.device-source")
    );

    let destruction_seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(3) == 0
        })
        .expect("one seed should destroy a source");
    let mut destroyed = prepare(17);
    destroyed.rng = RfbRng::seeded(destruction_seed);
    let update = dispatch_next(
        &mut destroyed,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(
        update.events[0]
            .args
            .get("sourceDestroyed")
            .map(String::as_str),
        Some("true")
    );
    assert!(
        destroyed
            .items
            .iter()
            .all(|item| item.id != "test.item.device-source")
    );

    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact_content =
        rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    artifact_content
        .content
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("wand definition")
        .tags
        .push("artifact".to_owned());
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact_content.content)
            .expect("artifact source content should remain valid"),
    ));
    let mut artifact =
        Game::from_content_with_build(18, catalog, BUILT_IN_WORLD_ID, "demo.build.tinkerer")
            .expect("custom tinkerer build should create");
    clear_monsters(&mut artifact);
    artifact
        .debug_add_generated_inventory_item(
            "test.item.device-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    artifact
        .debug_add_generated_inventory_item(
            "test.item.device-source",
            "demo.item.resonance-wand",
            1,
        )
        .expect("wand should generate");
    artifact
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.device-target")
        .expect("target")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 24,
    });
    artifact
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.device-source")
        .expect("source")
        .charges = Some(ItemChargesDto {
        current: 5,
        maximum: 24,
    });
    artifact.debug_set_recharge_attempts_succeed(true);
    artifact.rng = RfbRng::seeded(destruction_seed);
    let update = dispatch_next(
        &mut artifact,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(
        update.events[0]
            .args
            .get("sourceDestroyed")
            .map(String::as_str),
        Some("false")
    );
    assert!(
        artifact
            .items
            .iter()
            .any(|item| item.id == "test.item.device-source")
    );
}

#[test]
fn recharging_item_rejects_invalid_pairs_and_pays_the_source_before_failure() {
    let prepare = || {
        let mut game =
            Game::new_with_build(19, "demo.build.vanguard").expect("vanguard should create");
        clear_monsters(&mut game);
        game.debug_add_generated_inventory_item(
            "test.item.recharging-scroll",
            "demo.item.recharging-scroll",
            1,
        )
        .expect("recharging scroll should create");
        game.debug_add_generated_inventory_item(
            "test.item.recharge-source",
            "demo.item.resonance-wand",
            1,
        )
        .expect("source device should create");
        game.debug_add_generated_inventory_item(
            "test.item.recharge-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("target device should create");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.recharge-source")
            .expect("source device")
            .charges = Some(ItemChargesDto {
            current: 5,
            maximum: 24,
        });
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.recharge-target")
            .expect("target device")
            .charges = Some(ItemChargesDto {
            current: 2,
            maximum: 24,
        });
        game
    };

    for (source_item_id, target_item_id) in [
        ("missing.item", "test.item.recharge-target"),
        ("test.item.recharge-source", "test.item.recharge-source"),
    ] {
        let mut game = prepare();
        let world_tick = game.world_tick;
        let draws = game.rng.draw_counter;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItemForRecharge {
                item_id: "test.item.recharging-scroll".to_owned(),
                source_item_id: source_item_id.to_owned(),
                target_item_id: target_item_id.to_owned(),
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.world_tick, world_tick);
        assert_eq!(game.rng.draw_counter, draws);
        assert!(
            game.items
                .iter()
                .any(|item| item.id == "test.item.recharging-scroll")
        );
    }

    let mut game = prepare();
    game.debug_set_recharge_sources_survive(true);
    game.debug_set_recharge_attempts_fail(true);
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItemForRecharge {
            item_id: "test.item.recharging-scroll".to_owned(),
            source_item_id: "test.item.recharge-source".to_owned(),
            target_item_id: "test.item.recharge-target".to_owned(),
        },
    );
    assert_eq!(update.events[0].kind, "device.recharge-failure");
    assert!(
        game.items
            .iter()
            .all(|item| item.id != "test.item.recharging-scroll")
    );
    assert_eq!(
        game.item_knowledge_dto("demo.item.recharging-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recharge-source")
            .and_then(|item| item.charges)
            .expect("source charges")
            .current,
        0
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recharge-target")
            .and_then(|item| item.charges)
            .expect("target charges")
            .current,
        2
    );
}

#[test]
fn spell_scroll_increases_only_eligible_learning_capacity_without_rng() {
    const ITEM_ID: &str = "test.item.spell-scroll";
    const KIND_ID: &str = "demo.item.spell-scroll";

    let mut scholar =
        Game::new_with_build(17, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut scholar);
    give_inventory_item(&mut scholar, ITEM_ID, KIND_ID);
    let capacity_before = scholar
        .snapshot()
        .player
        .ability_learning
        .expect("scholar should expose learning capacity")
        .capacity;
    let draws_before = scholar.rng_draw_counter();
    let update = dispatch_next(
        &mut scholar,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(scholar.rng_draw_counter(), draws_before);
    assert_eq!(scholar.bonus_spell_learning_capacity, 1);
    assert_eq!(
        scholar
            .snapshot()
            .player
            .ability_learning
            .expect("scholar should retain learning capacity")
            .capacity,
        capacity_before + 1
    );
    let capacity_before_arg = capacity_before.to_string();
    let capacity_after_arg = capacity_before.saturating_add(1).to_string();
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-spell-learning-capacity-increased"
            && event.args.get("before").map(String::as_str) == Some(capacity_before_arg.as_str())
            && event.args.get("after").map(String::as_str) == Some(capacity_after_arg.as_str())
    }));
    assert_eq!(scholar.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    let restored = Game::from_save(scholar.to_save()).expect("spell bonus should round trip");
    assert_eq!(restored.state_hash(), scholar.state_hash());

    let mut vanguard =
        Game::new_with_build(17, "demo.build.vanguard").expect("vanguard build should create");
    clear_monsters(&mut vanguard);
    give_inventory_item(&mut vanguard, ITEM_ID, KIND_ID);
    let draws_before = vanguard.rng_draw_counter();
    let tick_before = vanguard.world_tick;
    let update = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(vanguard.rng_draw_counter(), draws_before);
    assert_eq!(vanguard.world_tick, tick_before + 10);
    assert_eq!(vanguard.bonus_spell_learning_capacity, 0);
    assert!(!vanguard.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(
        vanguard.item_knowledge_dto(KIND_ID),
        ItemKnowledgeDto::Aware
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| { event.kind == "item.use-spell-learning-capacity-no-effect" })
    );

    let mut invalid = vanguard.to_save();
    invalid.player.bonus_spell_learning_capacity = 1;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave(
            "bonus spell learning capacity is invalid"
        ))
    ));
}

#[test]
fn slowness_potion_refreshes_existing_slow_without_becoming_aware() {
    const ITEM_ID: &str = "test.item.slowness-potion";
    const KIND_ID: &str = "demo.item.slowness-potion";

    let mut game = Game::new(82);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_SLOW.to_owned(),
        intensity: 1,
        remaining_ticks: 1,
        source_id: Some("test.existing-slow".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    let slow = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_SLOW)
        .expect("the longer slowness roll should refresh the status");
    assert!((6..=30).contains(&slow.remaining_ticks));
    assert_eq!(slow.source_id.as_deref(), Some("test.existing-slow"));
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-slowness-no-effect")
    );
}

#[test]
fn veil_draught_awareness_and_rng_follow_existing_blindness_and_immunity() {
    const ITEM_ID: &str = "test.item.veil-draught";
    const KIND_ID: &str = "demo.item.veil-draught";

    for (existing_blindness, expected_draws, expected_event) in [
        (true, 2, "item.use-blindness-no-new-effect"),
        (false, 1, "item.use-blindness-resisted"),
    ] {
        let mut game = Game::new(94);
        clear_monsters(&mut game);
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        game.player.statuses.push(StatusInstance {
            kind_id: if existing_blindness {
                STATUS_BLINDNESS.to_owned()
            } else {
                "test.status.blindness-immunity".to_owned()
            },
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some(if existing_blindness {
                "test.existing-blindness".to_owned()
            } else {
                "test.blindness-immunity".to_owned()
            }),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: if existing_blindness {
                BTreeSet::new()
            } else {
                BTreeSet::from([STATUS_BLINDNESS.to_owned()])
            },
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.rng_draw_counter(), draws_before + expected_draws);
        assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
        assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == expected_event)
        );
        let blindness = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLINDNESS);
        if existing_blindness {
            let blindness = blindness.expect("the blindness duration should extend");
            assert!((110..=209).contains(&blindness.remaining_ticks));
            assert_eq!(
                blindness.source_id.as_deref(),
                Some("test.existing-blindness")
            );
        } else {
            assert!(blindness.is_none());
        }
    }
}

#[test]
fn fury_draught_awareness_depends_on_new_berserk_or_actual_healing() {
    const ITEM_ID: &str = "test.item.fury-draught";
    const KIND_ID: &str = "demo.item.fury-draught";

    for (damage, expected_knowledge, expected_heal_kind) in [
        (10, ItemKnowledgeDto::Aware, "item.use-heal"),
        (0, ItemKnowledgeDto::Tried, "item.use-no-effect"),
    ] {
        let mut game = Game::new(90);
        clear_monsters(&mut game);
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        game.player.statuses.push(StatusInstance {
            kind_id: "rfb.status.berserk".to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some("test.existing-berserk".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto {
                defense: -10,
                max_hp: 30,
                ..StatModifiersDto::default()
            },
            granted_equipment_bonuses: EquipmentBonusesDto {
                melee_skill: 12,
                melee_damage: 3,
                ranged_skill: -12,
                throwing_skill: -20,
                device_skill: -20,
                saving_throw_skill: -30,
                stealth_skill: -7,
                search_skill: -15,
                perception_skill: -15,
                digging_skill: 30,
                ..EquipmentBonusesDto::default()
            },
            granted_status_immunities: BTreeSet::from([STATUS_FEAR.to_owned()]),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        game.player.hp = game.effective_player_max_hp() - damage;
        let expected_hp = game.effective_player_max_hp();
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.rng_draw_counter(), draws_before + 1);
        assert_eq!(game.player.hp, expected_hp);
        assert_eq!(game.item_knowledge_dto(KIND_ID), expected_knowledge);
        assert_eq!(
            update
                .events
                .iter()
                .take(2)
                .map(|event| event.kind.as_str())
                .collect::<Vec<_>>(),
            [
                "item.use-berserk-strength-no-new-effect",
                expected_heal_kind
            ]
        );
        let berserk = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == "rfb.status.berserk")
            .expect("the draught should extend Berserk");
        assert!((36..=60).contains(&berserk.remaining_ticks));
    }
}

#[test]
fn renewal_tonic_awareness_depends_on_either_restoration() {
    const ITEM_ID: &str = "test.item.renewal-tonic";
    const KIND_ID: &str = "demo.item.renewal-tonic";

    for (
        experience,
        maximum_experience,
        life_force,
        level,
        expected_life_force,
        expected_knowledge,
        expected_event_kind,
    ) in [
        (
            5,
            25,
            1_000,
            1,
            1_000,
            ItemKnowledgeDto::Aware,
            "item.use-restore-life-levels",
        ),
        (
            25,
            25,
            900,
            3,
            1_000,
            ItemKnowledgeDto::Aware,
            "item.use-restore-life-levels",
        ),
        (
            25,
            25,
            1_000,
            3,
            1_000,
            ItemKnowledgeDto::Tried,
            "item.use-restore-life-levels-no-effect",
        ),
    ] {
        let mut game = Game::new(93);
        clear_monsters(&mut game);
        game.progress.experience = experience;
        game.progress.maximum_experience = maximum_experience;
        game.progress.life_force = life_force;
        game.progress.level = level;
        game.progress.max_level = level;
        give_inventory_item(&mut game, ITEM_ID, KIND_ID);
        let draws_before = game.rng_draw_counter();

        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: ITEM_ID.to_owned(),
                target: None,
            },
        );

        assert_eq!(game.progress.experience, maximum_experience);
        assert_eq!(game.progress.life_force, expected_life_force);
        assert_eq!(game.item_knowledge_dto(KIND_ID), expected_knowledge);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == expected_event_kind)
        );
    }
}

#[test]
fn temperate_tonic_extends_existing_resistance_without_becoming_aware() {
    const ITEM_ID: &str = "test.item.temperate-tonic";
    const KIND_ID: &str = "demo.item.temperate-tonic";

    let mut game = Game::new(85);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_THERMAL_RESISTANCE.to_owned(),
        intensity: 1,
        remaining_ticks: 3,
        source_id: Some("test.existing-thermal-resistance".to_owned()),
        granted_resistances: BTreeMap::from([
            (DamageType::Fire, ResistanceLevel::Resistant),
            (DamageType::Cold, ResistanceLevel::Resistant),
        ]),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    let thermal = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_THERMAL_RESISTANCE)
        .expect("the tonic should extend the existing resistance");
    assert!((4..=13).contains(&thermal.remaining_ticks));
    assert_eq!(
        thermal.source_id.as_deref(),
        Some("test.existing-thermal-resistance")
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-thermal-resistance-no-effect")
    );
}

#[test]
fn shatterburst_draught_uses_damage_scaling_and_existing_status_stacking() {
    const ITEM_ID: &str = "test.item.shatterburst-draught";
    const KIND_ID: &str = "demo.item.shatterburst-draught";

    let mut game = Game::new(95);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.hp = 10_000;
    for (kind_id, intensity, remaining_ticks, source_id, incoming_damage_percent) in [
        (STATUS_STUN, 2, 100, "test.existing-stun", 100),
        (STATUS_BLEEDING, 2, 20, "test.existing-bleeding", 100),
        (
            "test.status.half-damage",
            1,
            100,
            "test.detonation-guard",
            50,
        ),
    ] {
        game.player.statuses.push(StatusInstance {
            kind_id: kind_id.to_owned(),
            intensity,
            remaining_ticks,
            source_id: Some(source_id.to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent,
        });
    }
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    let damage = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-detonation")
        .and_then(|event| match &event.outcome {
            Some(GameEventOutcomeDto::Damage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("detonation should report its nonfatal damage");
    assert_eq!(game.rng_draw_counter(), draws_before + 50);
    assert_eq!(damage.armor_reduction, 0);
    assert_eq!(damage.resistance, ResistanceLevelDto::Normal);
    assert_eq!(damage.final_damage, (damage.raw_damage + 1) / 2);
    let stun = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_STUN)
        .expect("detonation should retain existing stun");
    assert_eq!(stun.intensity, 2);
    assert_eq!(stun.remaining_ticks, 90);
    assert_eq!(stun.source_id.as_deref(), Some("test.existing-stun"));
    let bleeding = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_BLEEDING)
        .expect("detonation should extend existing bleeding");
    assert_eq!(bleeding.intensity, 2);
    assert_eq!(bleeding.remaining_ticks, 5_010);
    assert_eq!(
        bleeding.source_id.as_deref(),
        Some("test.existing-bleeding")
    );
}

#[test]
fn mortal_draught_life_loss_bypasses_incoming_damage_reduction_without_rng() {
    const ITEM_ID: &str = "test.item.mortal-draught";
    const KIND_ID: &str = "demo.item.mortal-draught";

    let mut game = Game::new(83);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.half-damage".to_owned(),
        intensity: 1,
        remaining_ticks: 100,
        source_id: Some("test.life-loss-guard".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 50,
    });
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.player.hp, hp_before.saturating_sub(5000));
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn item_summon_candidates_follow_depth_player_level_kin_and_unique_rules() {
    let mut human = skill_check_game(67, "demo.build.vanguard");
    human.current_floor_id = "demo.floor.resonance-depth-10".to_owned();
    let general_effect = human
        .content
        .item("demo.item.summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("summoning scroll should have a use action")
        .effect
        .clone();
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = human.item_category_summon_plan(&general_effect)
    else {
        panic!("general summoning scroll should produce a summon plan");
    };
    assert_eq!(category, "any-monster");
    assert!(candidate_kind_ids.contains(&"demo.actor.risen-thrall".to_owned()));
    assert!(!candidate_kind_ids.contains(&"demo.actor.grave-wight".to_owned()));
    assert!(candidate_kind_ids.iter().all(|kind_id| {
        let definition = human.content.actor(kind_id).expect("candidate actor");
        definition.level <= 10 && !definition.tags.iter().any(|tag| tag == "guardian")
    }));
    assert_eq!(
        human.summon_category_candidate_kind_ids("undead", None, 8, true),
        [
            "demo.actor.crypt-creep".to_owned(),
            "demo.actor.disembodied-hand-that-strangled-people".to_owned(),
            "demo.actor.green-glutton-ghost".to_owned(),
            "demo.actor.jibaku-ghost".to_owned(),
            "demo.actor.lost-soul".to_owned(),
            "demo.actor.poltergeist".to_owned(),
            "demo.actor.risen-thrall".to_owned(),
            "demo.actor.rotting-corpse".to_owned(),
            "demo.actor.skeleton-kobold".to_owned(),
            "demo.actor.skeleton-orc".to_owned(),
            "demo.actor.zombified-kobold".to_owned(),
        ]
    );
    assert_eq!(
        human.summon_category_candidate_kind_ids("undead", None, 32, true),
        [
            "demo.actor.crypt-creep".to_owned(),
            "demo.actor.disembodied-hand-that-strangled-people".to_owned(),
            "demo.actor.grave-wight".to_owned(),
            "demo.actor.green-glutton-ghost".to_owned(),
            "demo.actor.jibaku-ghost".to_owned(),
            "demo.actor.lost-soul".to_owned(),
            "demo.actor.poltergeist".to_owned(),
            "demo.actor.risen-thrall".to_owned(),
            "demo.actor.rotting-corpse".to_owned(),
            "demo.actor.skeleton-kobold".to_owned(),
            "demo.actor.skeleton-orc".to_owned(),
            "demo.actor.zombified-kobold".to_owned(),
        ]
    );

    let kin_effect = human
        .content
        .item("demo.item.kin-summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("kin summoning scroll should have a use action")
        .effect
        .clone();
    human.progress.level = 3;
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = human.item_category_summon_plan(&kin_effect)
    else {
        panic!("kin summoning scroll should produce a summon plan");
    };
    assert_eq!(category, "kin-glyph-112");
    assert_eq!(
        candidate_kind_ids,
        [
            "demo.actor.cinder-adept".to_owned(),
            "demo.actor.mote-binder".to_owned(),
        ]
    );
    human.progress.level = 4;
    let ItemUsePlan::SummonCategory {
        candidate_kind_ids, ..
    } = human.item_category_summon_plan(&kin_effect)
    else {
        unreachable!();
    };
    assert!(candidate_kind_ids.contains(&"demo.actor.hex-chanter".to_owned()));

    let mut gnome = skill_check_game(67, "demo.build.tinkerer");
    let gnome_effect = gnome
        .content
        .item("demo.item.kin-summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("kin summoning scroll should have a use action")
        .effect
        .clone();
    gnome.progress.level = 1;
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = gnome.item_category_summon_plan(&gnome_effect)
    else {
        unreachable!();
    };
    assert_eq!(category, "kin-glyph-104");
    assert_eq!(candidate_kind_ids, ["demo.actor.echo-hound"]);

    let high_undead = human.summon_category_candidate_kind_ids("high-undead", None, 48, true);
    assert!(high_undead.contains(&"demo.actor.dread-vampire".to_owned()));
    assert!(
        !human
            .summon_category_candidate_kind_ids("high-undead", None, 48, false)
            .contains(&"demo.actor.dread-vampire".to_owned())
    );
    let vampire = human
        .content
        .actor("demo.actor.dread-vampire")
        .expect("demo unique")
        .clone();
    human.entities.push(actor_from_runtime_spawn(
        "test.actor.existing-dread-vampire",
        &vampire.id,
        Position { x: 4, y: 3 },
        vampire.max_hp,
        vampire.speed,
        100,
        true,
    ));
    assert!(
        !human
            .summon_category_candidate_kind_ids("high-undead", None, 48, true)
            .contains(&"demo.actor.dread-vampire".to_owned())
    );
}

#[test]
fn friendly_item_summons_are_permanent_controlled_and_round_trip() {
    let mut game = skill_check_game(68, "demo.build.vanguard");
    give_inventory_item(
        &mut game,
        "test.item.pet-summoning-scroll.1",
        "demo.item.pet-summoning-scroll",
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.pet-summoning-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::ItemSummon { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("pet scroll should emit a summon resolution");
    assert!(!resolution.entity_ids.is_empty());
    assert!(!resolution.hostile);
    assert_eq!(resolution.duration_turns, 0);
    let summoned_ids = resolution.entity_ids.clone();
    assert!(summoned_ids.iter().all(|entity_id| {
        game.entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(game.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
    assert_eq!(
        game.item_knowledge_dto("demo.item.pet-summoning-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.item.pet-summoning-scroll.1")
    );

    let saved = game.to_save();
    let restored = Game::from_save(saved).expect("controlled item summons should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(summoned_ids.iter().all(|entity_id| {
        restored
            .entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(restored.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
}

#[test]
fn item_summon_zero_candidate_and_zero_space_consume_without_awareness_or_rng() {
    let use_and_assert_zero = |game: &mut Game, item_id: &str, kind_id: &str| {
        give_inventory_item(game, item_id, kind_id);
        let draws_before = game.rng.draw_counter;
        let update = dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: item_id.to_owned(),
                target: Some(TargetSelection::SelfTarget),
            },
        );
        let resolution = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::ItemSummon { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("summon attempt should emit a resolution");
        assert!(resolution.entity_ids.is_empty());
        assert_eq!(game.rng.draw_counter, draws_before);
        assert_eq!(game.item_knowledge_dto(kind_id), ItemKnowledgeDto::Tried);
        assert!(!game.items.iter().any(|item| item.id == item_id));
    };

    let mut no_candidate = skill_check_game(69, "demo.build.vanguard");
    no_candidate.progress.level = 1;
    use_and_assert_zero(
        &mut no_candidate,
        "test.item.kin-summoning-scroll.1",
        "demo.item.kin-summoning-scroll",
    );

    let mut no_space = skill_check_game(70, "demo.build.vanguard");
    let positions = no_space.open_positions_around(no_space.player.position, 2);
    assert!(!positions.is_empty());
    for (ordinal, position) in positions.into_iter().enumerate() {
        no_space.items.push(ItemInstance {
            id: format!("test.item.summon-blocker.{ordinal}"),
            kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            fuel: None,
            device_recovery_progress: 0,
            location: ItemLocation::Ground(position),
        });
    }
    use_and_assert_zero(
        &mut no_space,
        "test.item.pet-summoning-scroll.1",
        "demo.item.pet-summoning-scroll",
    );
}

#[test]
fn dispel_undead_scroll_uses_the_visible_actor_snapshot_and_resist_all_gate() {
    const SCROLL_ID: &str = "test.item.dispel-undead-scroll.1";
    let mut game = skill_check_game(71, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.dispel-undead-scroll");
    let mut spawn = |id: &str, kind_id: &str, position: Position| {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    };
    spawn(
        "test.actor.dispel-target",
        "demo.actor.dread-vampire",
        Position { x: 4, y: 3 },
    );
    spawn(
        "test.actor.dispel-living",
        "demo.actor.echo-hound",
        Position { x: 4, y: 2 },
    );
    spawn(
        "test.actor.dispel-resist-all",
        "demo.actor.resonant-warden",
        Position { x: 2, y: 3 },
    );
    spawn(
        "test.actor.dispel-behind-wall",
        "demo.actor.dread-vampire",
        Position { x: 3, y: 5 },
    );
    replace_terrain(&mut game, Position { x: 3, y: 4 }, "demo.terrain.wall");
    game.rng = RfbRng::seeded(71);
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.use_inventory_item(
        SCROLL_ID,
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Dispel Undead should resolve");

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.actor.dispel-target")
            .expect("visible undead should remain")
            .hp,
        80
    );
    for actor_id in [
        "test.actor.dispel-living",
        "test.actor.dispel-resist-all",
        "test.actor.dispel-behind-wall",
    ] {
        let actor = game
            .entities
            .iter()
            .find(|entity| entity.id == actor_id)
            .expect("unaffected actor should remain");
        assert_eq!(
            actor.hp,
            game.content
                .actor(&actor.kind_id)
                .expect("unaffected actor definition")
                .max_hp
        );
    }
    assert!(
        matches!(events.as_slice(), [DomainEvent::ItemDispelHit { target_kind_id, damage, .. }]
        if target_kind_id == "demo.actor.dread-vampire"
            && damage.applied == 80
            && damage.damage_type == DamageType::HolyFire)
    );
    assert_eq!(
        game.item_knowledge_dto("demo.item.dispel-undead-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn banishment_scroll_resolves_resistance_and_destinations_in_actor_order() {
    const SCROLL_ID: &str = "test.item.banishment-scroll.1";
    let mut game = skill_check_game(72, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.banishment-scroll");
    for (id, kind_id, position) in [
        (
            "test.actor.banish-normal",
            "demo.actor.echo-hound",
            Position { x: 4, y: 3 },
        ),
        (
            "test.actor.banish-resistant-unique",
            "demo.actor.dread-vampire",
            Position { x: 3, y: 4 },
        ),
        (
            "test.actor.banish-guardian",
            "demo.actor.resonant-warden",
            Position { x: 2, y: 3 },
        ),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    game.rng = RfbRng::seeded(72);
    let original_positions = game
        .entities
        .iter()
        .map(|entity| (entity.id.clone(), entity.position))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    game.use_inventory_item(
        SCROLL_ID,
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Banishment should resolve");

    assert_eq!(game.rng_draw_counter(), 1);
    assert_ne!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.actor.banish-normal")
            .expect("banished actor should remain")
            .position,
        original_positions["test.actor.banish-normal"]
    );
    for actor_id in [
        "test.actor.banish-resistant-unique",
        "test.actor.banish-guardian",
    ] {
        assert_eq!(
            game.entities
                .iter()
                .find(|entity| entity.id == actor_id)
                .expect("resistant actor should remain")
                .position,
            original_positions[actor_id]
        );
    }
    let event_kinds = events
        .iter()
        .map(|event| match event {
            DomainEvent::ItemBanishedActor { resolution, .. } => {
                format!("banished:{}", resolution.actor_id)
            }
            DomainEvent::ItemBanishmentResisted { target_kind_id, .. } => {
                format!("resisted:{target_kind_id}")
            }
            _ => "unexpected".to_owned(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        [
            "banished:test.actor.banish-normal",
            "resisted:demo.actor.dread-vampire",
            "resisted:demo.actor.resonant-warden",
        ]
    );
    assert_eq!(
        game.item_knowledge_dto("demo.item.banishment-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn visible_actor_scrolls_consume_empty_results_without_rng_or_awareness() {
    for (seed, item_id, kind_id) in [
        (
            73,
            "test.item.empty-dispel-undead-scroll.1",
            "demo.item.dispel-undead-scroll",
        ),
        (
            74,
            "test.item.empty-banishment-scroll.1",
            "demo.item.banishment-scroll",
        ),
    ] {
        let mut game = skill_check_game(seed, "demo.build.scholar");
        give_inventory_item(&mut game, item_id, kind_id);
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.use_inventory_item(
            item_id,
            None,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("empty visible actor effect should resolve");
        assert_eq!(game.rng_draw_counter(), 0);
        assert!(!game.items.iter().any(|item| item.id == item_id));
        assert_eq!(game.item_knowledge_dto(kind_id), ItemKnowledgeDto::Tried);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemDispelNoEffect { .. }] | [DomainEvent::ItemBanishmentNoEffect { .. }]
        ));
    }
}

#[test]
fn mass_genocide_scroll_consumes_empty_result_with_awareness_and_zero_rng() {
    const ITEM_ID: &str = "test.item.severance-scroll.1";
    const KIND_ID: &str = "demo.item.severance-scroll";
    let mut game = skill_check_game(75, "demo.build.scholar");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("empty mass genocide should resolve");

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.player.hp, hp_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemMassGenocide {
            removed_count: 0,
            resisted_count: 0,
            fatigue_damage: 0,
            ..
        }]
    ));
}

#[test]
fn genocide_scroll_rejects_invalid_glyphs_and_consumes_an_empty_selection_without_rng() {
    const ITEM_ID: &str = "test.item.glyph-severance-scroll.1";
    const KIND_ID: &str = "demo.item.glyph-severance-scroll";
    let mut game = skill_check_game(79, "demo.build.scholar");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    assert!(
        game.inventory_dto()
            .iter()
            .find(|item| item.id == ITEM_ID)
            .is_some_and(|item| item.requires_target_glyph)
    );
    let world_tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();

    for command in [
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
        GameCommand::UseItemByGlyph {
            item_id: ITEM_ID.to_owned(),
            glyph: "oo".to_owned(),
        },
    ] {
        let update = dispatch_next(&mut game, command);
        assert_eq!(update.world_tick, world_tick_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(game.items.iter().any(|item| item.id == ITEM_ID));
        assert_eq!(update.events[0].kind, "item.use-unavailable");
    }

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItemByGlyph {
            item_id: ITEM_ID.to_owned(),
            glyph: "x".to_owned(),
        },
    );
    assert!(update.world_tick > world_tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert_eq!(update.events[0].kind, "item.use-genocide");
    assert_eq!(
        update.events[0].args.get("glyph").map(String::as_str),
        Some("x")
    );
    assert_eq!(
        update.events[0].args.get("removed").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        update.events[0].args.get("resisted").map(String::as_str),
        Some("0")
    );
}

#[test]
fn adjacent_terrain_creation_consumes_empty_result_as_tried_without_rng() {
    const ITEM_ID: &str = "test.item.stone-ring-scroll.1";
    const KIND_ID: &str = "demo.item.stone-ring-scroll";
    let mut game = skill_check_game(76, "demo.build.scholar");
    let player_index = game
        .index(game.player.position)
        .expect("player position should be in bounds");
    game.terrain.fill("demo.terrain.wall".to_owned());
    game.terrain[player_index] = "demo.terrain.floor".to_owned();
    let item_position = Position { x: 4, y: 3 };
    let connection_position = Position { x: 3, y: 4 };
    replace_terrain(&mut game, item_position, "demo.terrain.floor");
    replace_terrain(&mut game, connection_position, "demo.terrain.floor");
    assert!(
        game.items
            .iter()
            .any(|item| item.location == ItemLocation::Ground(item_position))
    );
    game.floor_connections.push(FloorConnectionState {
        id: "test.connection.protected-floor".to_owned(),
        position: connection_position,
        target_floor_id: None,
        target_connection_id: None,
    });
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let before = game.snapshot();
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(update.world_tick, before.world_tick + 10);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Tried);
    assert_eq!(update.events.len(), 1);
    assert_eq!(
        update.events[0].kind,
        "item.use-create-adjacent-terrain-no-effect"
    );
    assert_eq!(update.events[0].args["count"], "0");
}

#[test]
fn vengeance_retaliates_against_monster_spells_but_not_after_player_death() {
    fn vengeance_status() -> StatusInstance {
        StatusInstance {
            kind_id: STATUS_VENGEANCE.to_owned(),
            intensity: 1,
            remaining_ticks: 100,
            source_id: Some("demo.item.reprisal-scroll".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }

    fn cinder_game(player_hp: i32) -> Game {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.wall".to_owned());
        let player = game.player.position;
        for step in 0..=3 {
            replace_terrain(
                &mut game,
                Position {
                    x: player.x + step,
                    y: player.y,
                },
                "demo.terrain.floor",
            );
        }
        game.player.hp = player_hp;
        game.player.statuses.push(vengeance_status());
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.vengeance-cinder",
            "demo.actor.cinder-adept",
            Position {
                x: player.x + 3,
                y: player.y,
            },
            20,
            100,
            100,
            true,
        ));
        game
    }

    fn first_cast(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        for _ in 0..100 {
            if game.resolve_monster_ability(0, &mut events) {
                return events;
            }
        }
        panic!("cinder adept should cast within 100 attempts");
    }

    let mut surviving = cinder_game(100);
    let events = first_cast(&mut surviving);
    let cast_damage = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterAbilityCast { resolution, .. } => {
                resolution.effects.iter().find_map(|effect| match effect {
                    AbilityEffectResolutionDto::Damage { resolution, .. } => {
                        Some(resolution.final_damage)
                    }
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("damaging monster cast should expose applied damage");
    let retaliation_damage = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::VengeanceHit { damage, .. }
            | DomainEvent::VengeanceSlew { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .expect("surviving player should retaliate");
    assert_eq!(retaliation_damage, cast_damage);
    assert_eq!(
        surviving
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_VENGEANCE)
            .expect("vengeance should remain active")
            .remaining_ticks,
        95
    );

    let mut dying = cinder_game(1);
    let source_hp = dying.entities[0].hp;
    let events = first_cast(&mut dying);
    assert!(dying.player_is_dead());
    assert_eq!(dying.entities[0].hp, source_hp);
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::VengeanceHit { .. } | DomainEvent::VengeanceSlew { .. }
    )));
    assert_eq!(dying.player.statuses[0].remaining_ticks, 100);
}

#[test]
fn confusing_strike_preparation_and_hit_boundaries_are_authoritative() {
    fn target(game: &mut Game, kind_id: &str, hp: i32) {
        clear_monsters(game);
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.confusing-strike-target",
            kind_id,
            Position {
                x: game.player.position.x + 1,
                y: game.player.position.y,
            },
            hp,
            100,
            100,
            true,
        ));
        game.confusing_strike_ready = true;
    }

    let mut prepared = Game::new(77);
    clear_monsters(&mut prepared);
    for serial in 1..=2 {
        give_inventory_item(
            &mut prepared,
            &format!("test.item.confusing-touch-scroll.{serial}"),
            "demo.item.confusing-touch-scroll",
        );
    }
    prepared.rng = RfbRng::seeded(77);
    for serial in 1..=2 {
        let draws_before = prepared.rng_draw_counter();
        let mut events = Vec::new();
        prepared
            .use_inventory_item(
                &format!("test.item.confusing-touch-scroll.{serial}"),
                None,
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("confusing strike preparation should resolve");
        assert!(prepared.confusing_strike_ready);
        assert_eq!(prepared.rng_draw_counter(), draws_before);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemConfusingStrikePrepared { .. }]
        ));
    }
    let saved = prepared.to_save();
    assert!(saved.player.confusing_strike_ready);
    assert!(
        Game::from_save(saved)
            .expect("prepared confusing strike should reload")
            .confusing_strike_ready
    );

    let mut missed = Game::new(77);
    target(&mut missed, "demo.actor.ember-mote", 10);
    missed
        .progress
        .skills
        .get_mut("demo.skill.melee")
        .expect("player melee skill should exist")
        .current = 0;
    let miss_seed = (0..100_u64)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) >= 10
        })
        .expect("one seed should produce a regular miss");
    missed.rng = RfbRng::seeded(miss_seed);
    let mut events = Vec::new();
    missed
        .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("missed melee should resolve");
    assert!(missed.confusing_strike_ready);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::PlayerMeleeMissed { .. }]
    ));

    let lethal_seed = (0..100_u64)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < 5
        })
        .expect("one seed should produce an automatic hit");
    let mut lethal = Game::new(77);
    target(&mut lethal, "demo.actor.ember-mote", 1);
    lethal.rng = RfbRng::seeded(lethal_seed);
    let mut events = Vec::new();
    lethal
        .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("lethal melee should resolve");
    assert!(lethal.confusing_strike_ready);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerSlew { .. }))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::ConfusingStrikeImmune { .. }
            | DomainEvent::ConfusingStrikeResisted { .. }
            | DomainEvent::ConfusingStrikeApplied { .. }
    )));

    let mut immune = Game::new(77);
    target(&mut immune, "demo.actor.veil-warden", 10);
    let definition = immune
        .content
        .actor("demo.actor.veil-warden")
        .expect("immune target definition should exist")
        .clone();
    let draws_before = immune.rng_draw_counter();
    let mut events = Vec::new();
    immune.resolve_confusing_strike(0, &definition, &mut events);
    assert!(!immune.confusing_strike_ready);
    assert_eq!(immune.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ConfusingStrikeImmune { .. }]
    ));

    let mut resisted = Game::new(77);
    target(&mut resisted, "demo.actor.echo-hound", 10);
    let definition = resisted
        .content
        .actor("demo.actor.echo-hound")
        .expect("resisting target definition should exist")
        .clone();
    let resist_seed = (0..1_000_u64)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(definition.level)
        })
        .expect("one seed should produce a level resistance");
    resisted.rng = RfbRng::seeded(resist_seed);
    let mut events = Vec::new();
    resisted.resolve_confusing_strike(0, &definition, &mut events);
    assert!(!resisted.confusing_strike_ready);
    assert_eq!(resisted.rng_draw_counter(), 1);
    assert!(resisted.entities[0].statuses.is_empty());
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ConfusingStrikeResisted { .. }]
    ));
}

#[test]
fn protection_from_evil_duration_and_melee_branches_are_authoritative() {
    fn protection_status() -> StatusInstance {
        StatusInstance {
            kind_id: STATUS_PROTECTION_FROM_EVIL.to_owned(),
            intensity: 1,
            remaining_ticks: 100,
            source_id: Some("demo.item.protection-from-evil-scroll".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        }
    }

    fn protected_game() -> Game {
        let mut game = Game::new(77);
        game.progress.level = 50;
        game.progress.attributes.wisdom = 238;
        game.player.statuses.push(protection_status());
        game
    }

    let mut prepared = Game::new(77);
    clear_monsters(&mut prepared);
    prepared.progress.level = 10;
    for serial in 1..=2 {
        give_inventory_item(
            &mut prepared,
            &format!("test.item.protection-from-evil-scroll.{serial}"),
            "demo.item.protection-from-evil-scroll",
        );
    }
    prepared.rng = RfbRng::seeded(77);
    let mut expected_rng = RfbRng::seeded(77);
    let expected_durations = [
        31 + u32::try_from(expected_rng.bounded(25)).expect("duration roll must fit u32"),
        31 + u32::try_from(expected_rng.bounded(25)).expect("duration roll must fit u32"),
    ];
    for (serial, expected_duration) in (1..=2).zip(expected_durations) {
        let mut events = Vec::new();
        prepared
            .use_inventory_item(
                &format!("test.item.protection-from-evil-scroll.{serial}"),
                None,
                None,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("protection from evil should resolve");
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemProtectionFromEvil { duration, .. }]
                if *duration == expected_duration
        ));
    }
    assert_eq!(prepared.rng_draw_counter(), 2);
    assert_eq!(
        prepared
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_PROTECTION_FROM_EVIL)
            .expect("protection from evil should remain active")
            .remaining_ticks,
        expected_durations.into_iter().sum::<u32>()
    );

    let non_evil = prepared
        .content
        .actor("demo.actor.ember-mote")
        .expect("non-evil actor should exist")
        .clone();
    let mut non_evil_game = protected_game();
    let draws_before = non_evil_game.rng_draw_counter();
    assert!(!non_evil_game.protection_from_evil_repels(&non_evil));
    assert_eq!(non_evil_game.rng_draw_counter(), draws_before);

    let evil = prepared
        .content
        .actor("demo.actor.gloom-weaver")
        .expect("evil actor should exist")
        .clone();
    let branch = |seed| {
        let mut rng = RfbRng::seeded(seed);
        let player_roll = rng.bounded(100) + 1;
        let monster_roll = rng.bounded(3) + 1;
        if player_roll <= monster_roll {
            (false, 2)
        } else {
            (rng.bounded(3) != 0, 3)
        }
    };
    let saved_seed = (0..1_000_u64)
        .find(|seed| branch(*seed) == (false, 2))
        .expect("one seed should let the evil monster save");
    let bypass_seed = (0..1_000_u64)
        .find(|seed| branch(*seed) == (false, 3))
        .expect("one seed should pass the one-in-three bypass");
    let repelled_seed = (0..1_000_u64)
        .find(|seed| branch(*seed) == (true, 3))
        .expect("one seed should repel the evil monster");
    for (seed, expected_repelled, expected_draws) in [
        (saved_seed, false, 2),
        (bypass_seed, false, 3),
        (repelled_seed, true, 3),
    ] {
        let mut game = protected_game();
        game.rng = RfbRng::seeded(seed);
        assert_eq!(game.protection_from_evil_repels(&evil), expected_repelled);
        assert_eq!(game.rng_draw_counter(), expected_draws);
    }
}

#[test]
fn travel_scroll_random_teleport_is_deterministic_and_rejects_without_space_atomically() {
    let prepare = || {
        let mut game = Game::new(64);
        clear_monsters(&mut game);
        give_inventory_item(
            &mut game,
            "test.item.flicker-scroll.1",
            "demo.item.flicker-scroll",
        );
        game
    };
    let mut first = prepare();
    let mut second = prepare();
    let first_update = dispatch_next(
        &mut first,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    let second_update = dispatch_next(
        &mut second,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(first_update.events, second_update.events);
    assert_ne!(first.player.position, Position { x: 3, y: 3 });
    assert!(first_update.events.iter().any(|event| {
        event.kind == "item.use-teleported"
            && matches!(
                event.outcome,
                Some(GameEventOutcomeDto::AbilityTeleport { .. })
            )
    }));

    let mut blocked = prepare();
    let player_index = blocked
        .index(blocked.player.position)
        .expect("player position should be in bounds");
    blocked.terrain.fill("demo.terrain.wall".to_owned());
    blocked.terrain[player_index] = "demo.terrain.floor".to_owned();
    let before = blocked.snapshot();
    let draw_counter = blocked.rng_draw_counter();
    let update = dispatch_next(
        &mut blocked,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(blocked.rng_draw_counter(), draw_counter);
    assert!(
        blocked
            .items
            .iter()
            .any(|item| item.id == "test.item.flicker-scroll.1")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn teleport_level_rolls_direction_then_uses_tree_targets_and_boundary_fallback() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    give_inventory_item(
        &mut game,
        "test.item.depthshift-scroll.1",
        "demo.item.depthshift-scroll",
    );
    let downward_seed = (0_u64..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(2) == 1
        })
        .expect("one seed should select downward travel");
    game.rng = RfbRng::seeded(downward_seed);
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.depthshift-scroll.1".to_owned(),
            target: None,
        },
    );
    assert!(game.floor_depth(&game.current_floor_id) > 1);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-teleported-level"
            && event.args.get("to") == Some(&game.current_floor_id)
    }));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "floor.transition")
    );

    game.transition_floor("demo.floor.echo-depth-3".to_owned(), None, None, false)
        .expect("final floor transition should resolve")
        .expect("final floor should be available");
    game.entities.clear();
    game.dungeon_states
        .get_mut("demo.dungeon.echo-depths")
        .expect("echo dungeon state")
        .guardian_defeated = true;
    give_inventory_item(
        &mut game,
        "test.item.depthshift-scroll.2",
        "demo.item.depthshift-scroll",
    );
    let before_depth = game.floor_depth(&game.current_floor_id);
    game.rng = RfbRng::seeded(downward_seed);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.depthshift-scroll.2".to_owned(),
            target: None,
        },
    );
    assert!(game.floor_depth(&game.current_floor_id) < before_depth);
}

#[test]
fn recall_round_trip_clears_the_old_instance_and_creates_a_new_one() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    let first_instance = game
        .current_dungeon_instance_id
        .clone()
        .expect("dungeon should have an instance");
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-1")
    );
    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.1",
        "demo.item.homeward-scroll",
    );
    game.debug_set_recall_delay_turns(Some(1));
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        Some(1)
    );
    let restored = Game::from_save(game.to_save()).expect("pending recall should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.recall-triggered")
    );
    let recall_event = update
        .events
        .iter()
        .position(|event| event.kind == "item.recall-triggered")
        .expect("recall trigger event should be projected");
    let floor_event = update
        .events
        .iter()
        .position(|event| event.kind == "floor.transition")
        .expect("floor transition event should be projected");
    assert!(recall_event < floor_event);
    assert!(
        game.stored_floors
            .values()
            .all(|floor| { floor.dungeon_instance_id.as_deref() != Some(first_instance.as_str()) })
    );

    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.2",
        "demo.item.homeward-scroll",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.2".to_owned(),
            target: None,
        },
    );
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-1");
    assert_ne!(
        game.current_dungeon_instance_id.as_ref(),
        Some(&first_instance)
    );
}

#[test]
fn recall_can_be_cancelled_and_reset_to_a_shallower_branch_floor() {
    let mut game = Game::new(11);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.transition_floor("demo.floor.echo-depth-2".to_owned(), None, None, false)
        .expect("deeper transition should resolve")
        .expect("deeper floor should be available");
    clear_monsters(&mut game);
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-2")
    );
    game.transition_floor("demo.floor.echo-depth-1".to_owned(), None, None, false)
        .expect("shallower transition should resolve")
        .expect("shallower floor should be available");
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-2")
    );

    give_inventory_item(
        &mut game,
        "test.item.recall-setting-scroll.1",
        "demo.item.recall-setting-scroll",
    );
    let reset = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.recall-setting-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-1")
    );
    assert!(
        reset
            .events
            .iter()
            .any(|event| event.kind == "item.recall-reset")
    );
    let restored = Game::from_save(game.to_save()).expect("reset destination should round trip");
    assert_eq!(restored.recall, game.recall);

    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.3",
        "demo.item.homeward-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.4",
        "demo.item.homeward-scroll",
    );
    game.debug_set_recall_delay_turns(Some(3));
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.3".to_owned(),
            target: None,
        },
    );
    assert!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns)
            .is_some()
    );
    let cancelled = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.4".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        None
    );
    assert!(
        cancelled
            .events
            .iter()
            .any(|event| event.kind == "item.recall-cancelled")
    );
}
