// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn artifact_loot_context(depth: u16) -> LootContext {
    LootContext {
        table_id: "demo.loot-table.paladin".to_owned(),
        floor_id: format!("test.floor.depth-{depth}"),
        depth,
        source: LootSource::ItemUse {
            item_id: "test.item-generation".to_owned(),
        },
    }
}

#[test]
fn booze_applies_original_confusion_hallucination_and_blackout_ranges() {
    let mut saw_hallucination = false;
    let mut saw_clear_head = false;
    let mut saw_blackout = false;
    for seed in 0..256 {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        clear_monsters(&mut game);
        game.rng = RfbRng::seeded(seed);
        game.explored.fill(true);
        let origin = game.player.position;
        game.resolve_item_booze(
            "demo.item.booze-potion",
            &mut Vec::new(),
            &mut BTreeSet::new(),
        );

        let confusion = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_CONFUSION)
            .expect("an unresisted drink should always confuse a Warrior");
        assert!((160..=350).contains(&confusion.remaining_ticks));
        if let Some(hallucination) = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_HALLUCINATION)
        {
            assert!((260..=500).contains(&hallucination.remaining_ticks));
            saw_hallucination = true;
        } else {
            saw_clear_head = true;
        }
        if game.player.position != origin {
            assert!(game.explored.iter().all(|explored| !explored));
            saw_blackout = true;
        }
    }
    assert!(saw_hallucination && saw_clear_head && saw_blackout);
}

#[test]
fn booze_keeps_a_longer_existing_confusion_duration() {
    let mut game =
        Game::new_with_build(0, "demo.build.warrior").expect("Warrens journey should create");
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        remaining_ticks: 10_000,
        intensity: 1,
        source_id: Some("test.existing-confusion".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    game.resolve_item_booze(
        "demo.item.booze-potion",
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );

    let confusion = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_CONFUSION)
        .expect("existing confusion should remain");
    assert_eq!(confusion.remaining_ticks, 10_000);
    assert_eq!(
        confusion.source_id.as_deref(),
        Some("test.existing-confusion")
    );
}

#[test]
fn booze_refreshes_existing_statuses_without_identifying_itself() {
    let mut verified = false;
    for seed in 0..256 {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        clear_monsters(&mut game);
        game.rng = RfbRng::seeded(seed);
        game.mark_item_tried("demo.item.booze-potion");
        for status_kind_id in [STATUS_CONFUSION, STATUS_HALLUCINATION] {
            game.player.statuses.push(StatusInstance {
                kind_id: status_kind_id.to_owned(),
                remaining_ticks: 10,
                intensity: 1,
                source_id: Some("test.existing-status".to_owned()),
                granted_resistances: BTreeMap::new(),
                granted_brands: BTreeSet::new(),
                granted_modifiers: StatModifiersDto::default(),
                granted_equipment_bonuses: EquipmentBonusesDto::default(),
                granted_status_immunities: BTreeSet::new(),
                granted_race_id: None,
                grants_wall_passage: false,
                incoming_damage_percent: 100,
            });
        }
        let mut events = Vec::new();
        game.resolve_item_booze("demo.item.booze-potion", &mut events, &mut BTreeSet::new());
        if events
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemTeleported { .. }))
        {
            continue;
        }

        assert!(
            game.player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_CONFUSION)
                .is_some_and(|status| status.remaining_ticks > 10)
        );
        assert!(
            events.iter().all(|event| !matches!(
                event,
                DomainEvent::ItemStatusResolved { noticed: true, .. }
            ))
        );
        assert_eq!(
            game.item_knowledge_dto("demo.item.booze-potion"),
            ItemKnowledgeDto::Tried
        );
        verified = true;
        break;
    }
    assert!(verified, "a non-blackout booze seed should be available");
}

#[test]
fn restorative_item_sequence_recovers_resource_then_removes_status() {
    const ITEM_ID: &str = "test.item.clarity-draught.1";
    let mut game = test_caster_game(19);
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana")
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
    let mut game = test_caster_game(23);
    clear_monsters(&mut game);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana");
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
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("restored resource state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn successful_restoration_reveals_later_no_effect_events() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = test_caster_game(27);
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana")
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
    let mut game = skill_check_game(29, "demo.build.warrior");
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
fn identify_scroll_rejects_missing_and_self_targets_before_consumption() {
    const SCROLL_ID: &str = "test.item.invalid-identify-scroll.1";
    let mut game = skill_check_game(41, "demo.build.warrior");
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
fn enchantment_artifact_and_ammunition_pile_gates_follow_original_order() {
    let artifact_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.warrior");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 1, false, false);
        let mut artifact = skill_check_game(*seed, "demo.build.warrior");
        artifact.rng = RfbRng::seeded(*seed);
        let artifact = artifact.resolve_item_enchantment_component(0, 1, 1, false, true);
        ordinary.successes == 1 && artifact.successes == 0
    });
    assert_eq!(artifact_seed, Some(0));

    let ammunition_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.warrior");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 20, false, false);
        let mut ammunition = skill_check_game(*seed, "demo.build.warrior");
        ammunition.rng = RfbRng::seeded(*seed);
        let ammunition = ammunition.resolve_item_enchantment_component(0, 1, 20, true, false);
        ordinary.successes == 0 && ammunition.successes == 1
    });
    assert_eq!(ammunition_seed, Some(0));
}

#[test]
fn curse_scroll_lands_on_equipped_weapon_and_artifact_can_resist() {
    fn run(resisted: bool) -> (Game, GameUpdate, u64) {
        const SCROLL_ID: &str = "test.item.weapon-blight-scroll.1";
        const WEAPON_ID: &str = "test.item.relic-blade.1";
        let mut game = skill_check_game(61, "demo.build.warrior");
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
    let mut game = skill_check_game(67, "demo.build.warrior");
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
    let mut game = skill_check_game(71, "demo.build.warrior");
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
            "right-hand",
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
            "neck",
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
fn spell_scroll_increases_only_eligible_learning_capacity_without_rng() {
    const ITEM_ID: &str = "test.item.spell-scroll";
    const KIND_ID: &str = "demo.item.spell-scroll";

    let mut caster = test_caster_game(17);
    clear_monsters(&mut caster);
    give_inventory_item(&mut caster, ITEM_ID, KIND_ID);
    let capacity_before = caster
        .snapshot()
        .player
        .ability_learning
        .expect("test caster should expose learning capacity")
        .capacity;
    let draws_before = caster.rng_draw_counter();
    let update = dispatch_next(
        &mut caster,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(caster.rng_draw_counter(), draws_before);
    assert_eq!(caster.bonus_spell_learning_capacity, 1);
    assert_eq!(
        caster
            .snapshot()
            .player
            .ability_learning
            .expect("test caster should retain learning capacity")
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
    assert_eq!(caster.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    let restored = Game::from_save_with_content(caster.to_save(), caster.content.clone())
        .expect("spell bonus should round trip");
    assert_eq!(restored.state_hash(), caster.state_hash());

    let mut warrior =
        Game::new_with_build(17, "demo.build.warrior").expect("Warrior build should create");
    clear_monsters(&mut warrior);
    give_inventory_item(&mut warrior, ITEM_ID, KIND_ID);
    let draws_before = warrior.rng_draw_counter();
    let tick_before = warrior.world_tick;
    let update = dispatch_next(
        &mut warrior,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(warrior.rng_draw_counter(), draws_before);
    assert_eq!(warrior.world_tick, tick_before + 10);
    assert_eq!(warrior.bonus_spell_learning_capacity, 0);
    assert!(!warrior.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(warrior.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert!(
        update
            .events
            .iter()
            .any(|event| { event.kind == "item.use-spell-learning-capacity-no-effect" })
    );

    let mut invalid = warrior.to_save();
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
    let duration_seed = (0..512)
        .find(|seed| RfbRng::seeded(*seed).bounded(25) == 24)
        .expect("a maximum slowness duration roll should exist");
    game.rng = RfbRng::seeded(duration_seed);
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
    assert!(slow.remaining_ticks > 1);
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
fn friendly_item_summons_are_permanent_controlled_and_round_trip() {
    let mut game = skill_check_game(68, "demo.build.warrior");
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
        let mut game = skill_check_game(seed, "demo.build.warrior");
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
    let mut game = skill_check_game(75, "demo.build.warrior");
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
    let mut game = skill_check_game(79, "demo.build.warrior");
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
    let mut game = skill_check_game(76, "demo.build.warrior");
    let player_index = game
        .index(game.player.position)
        .expect("player position should be in bounds");
    game.terrain.fill("demo.terrain.wall".to_owned());
    game.terrain[player_index] = "demo.terrain.floor".to_owned();
    let item_position = Position { x: 4, y: 3 };
    let connection_position = Position { x: 3, y: 4 };
    replace_terrain(&mut game, item_position, "demo.terrain.floor");
    replace_terrain(&mut game, connection_position, "demo.terrain.floor");
    give_inventory_item(
        &mut game,
        "test.item.ground-blocker",
        "demo.item.ration-of-food",
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.ground-blocker")
        .expect("ground blocker should exist")
        .location = ItemLocation::Ground(item_position);
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
fn p3_4_light_and_darkness_reuse_persisted_floor_glow() {
    let mut game = skill_check_game(206, "demo.build.warrior");
    game.glow.fill(false);
    give_inventory_item(&mut game, "test.item.light.1", "demo.item.light-scroll");
    let hash_before = game.state_hash();
    let mut events = Vec::new();
    game.use_inventory_item(
        "test.item.light.1",
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("light scroll should resolve");
    assert!(game.glow.iter().any(|glow| *glow));
    assert_ne!(game.state_hash(), hash_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemFloorGlowChanged {
            glow: true,
            affected_positions,
            ..
        }] if !affected_positions.is_empty()
    ));

    let restored = Game::from_save(game.to_save()).expect("lit floor should reload");
    assert_eq!(restored.glow, game.glow);
    assert_eq!(restored.state_hash(), game.state_hash());

    give_inventory_item(
        &mut game,
        "test.item.darkness.1",
        "demo.item.darkness-scroll",
    );
    let mut darkness_events = Vec::new();
    game.use_inventory_item(
        "test.item.darkness.1",
        None,
        None,
        &mut darkness_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("darkness scroll should resolve");
    assert!(game.glow.iter().all(|glow| !*glow));
    assert!(darkness_events.iter().any(|event| matches!(
        event,
        DomainEvent::ItemFloorGlowChanged {
            glow: false,
            affected_positions,
            ..
        } if !affected_positions.is_empty()
    )));
}

#[test]
fn p3_4_rune_requires_clean_floor_and_uses_original_break_threshold() {
    let mut blocked = skill_check_game(207, "demo.build.warrior");
    let blocked_position = blocked.player.position;
    replace_terrain(&mut blocked, blocked_position, "demo.terrain.floor");
    blocked.gold_piles.push(GoldPile {
        id: "test.gold.rune-blocker".to_owned(),
        position: blocked.player.position,
        amount: 1,
        appearance: GoldAppearanceDto::Copper,
        discovered: true,
    });
    give_inventory_item(
        &mut blocked,
        "test.item.rune.blocked",
        "demo.item.rune-of-protection-scroll",
    );
    let mut blocked_events = Vec::new();
    blocked
        .use_inventory_item(
            "test.item.rune.blocked",
            None,
            None,
            &mut blocked_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("blocked rune should still consume the scroll");
    assert_eq!(
        blocked.terrain_at(blocked.player.position),
        "demo.terrain.floor"
    );
    assert!(matches!(
        blocked_events.as_slice(),
        [DomainEvent::ItemCreatedCurrentTerrain {
            affected_position: None,
            ..
        }]
    ));

    let mut game = game_with_actor_definition(208, "demo.actor.dread-vampire", |actor| {
        actor.level = 400;
    });
    clear_monsters(&mut game);
    let player_position = game.player.position;
    replace_terrain(&mut game, player_position, "demo.terrain.floor");
    give_inventory_item(
        &mut game,
        "test.item.rune.legal",
        "demo.item.rune-of-protection-scroll",
    );
    game.use_inventory_item(
        "test.item.rune.legal",
        None,
        None,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("legal rune should resolve");
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.warding-glyph"
    );
    let monster_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, monster_position, "demo.terrain.floor");
    game.push_generated_actor(
        "test.actor.rune-breaker".to_owned(),
        "demo.actor.dread-vampire",
        monster_position,
    );
    let mut events = Vec::new();
    assert_eq!(
        game.try_monster_break_warding_glyph(
            0,
            game.player.position,
            &mut events,
            &mut BTreeSet::new(),
        ),
        Some(true)
    );
    assert_eq!(game.terrain_at(game.player.position), "demo.terrain.floor");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::WardingGlyphBroken { .. }]
    ));
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
fn p3_2_refreshments_are_deliberate_no_numeric_effects() {
    let mut game = Game::new(201);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.item.water.1", "demo.item.water-potion");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.water.1".to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == "test.item.water.1"));
    assert_eq!(
        game.item_knowledge_dto("demo.item.water-potion"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-no-effect")
    );
}

#[test]
fn p3_2_lose_memories_preserves_historical_experience() {
    let mut game = Game::new(202);
    clear_monsters(&mut game);
    game.progress.gain_experience(1_000, false);
    let maximum = game.progress.maximum_experience;
    give_inventory_item(
        &mut game,
        "test.item.lose-memories.1",
        "demo.item.lose-memories-potion",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.lose-memories.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.experience, 750);
    assert_eq!(game.progress.maximum_experience, maximum);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "item.experience-lost")
        .expect("experience loss should be projected");
    assert_eq!(event.args["amount"], "250");
    assert_eq!(event.args["remaining"], "750");
}

#[test]
fn p3_2_invulnerability_and_giant_strength_reuse_status_payloads() {
    let mut game = Game::new(204);
    clear_monsters(&mut game);
    give_inventory_item(
        &mut game,
        "test.item.invulnerability.1",
        "demo.item.invulnerability-potion",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.invulnerability.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.player_incoming_damage_percent(), 0);
    assert_eq!(
        game.reduce_player_damage(resolve_damage(
            DamagePacket::new(100, DamageType::Physical),
            ResistanceLevel::Normal,
        ))
        .applied,
        0
    );

    give_inventory_item(
        &mut game,
        "test.item.giant-strength.1",
        "demo.item.giant-strength-potion",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.giant-strength.1".to_owned(),
            target: None,
        },
    );
    let giant = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_GIANT_STRENGTH)
        .expect("giant strength should remain active");
    assert_eq!(giant.granted_modifiers.max_hp, 10);
    assert_eq!(giant.granted_equipment_bonuses.melee_skill, 1);
}

#[test]
fn p3_7_experience_potion_uses_unscaled_relative_gain_and_level_cap() {
    let mut game =
        Game::new_with_build(701, "demo.build.warrior").expect("Warrior build should create");
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(100, &mut Vec::new());
    assert_eq!(game.progress.experience, 100);
    give_inventory_item(
        &mut game,
        "test.item.experience.1",
        "demo.item.experience-potion",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.experience.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.experience, 160);
    assert!(update.events.iter().any(|event| {
        event.kind == "player.experience-gained"
            && event.args.get("amount").map(String::as_str) == Some("60")
    }));

    let mut capped = Game::new(704);
    clear_monsters(&mut capped);
    capped.apply_unscaled_player_experience(4_500_000, &mut Vec::new());
    assert_eq!(capped.progress.level, 50);
    give_inventory_item(
        &mut capped,
        "test.item.experience.2",
        "demo.item.experience-potion",
    );
    dispatch_next(
        &mut capped,
        GameCommand::UseItem {
            item_id: "test.item.experience.2".to_owned(),
            target: None,
        },
    );
    assert_eq!(capped.progress.experience, 4_600_000);
    assert_eq!(capped.progress.level, 50);
}

#[test]
fn p3_7_neo_tsuyoshi_round_trips_and_crashes_on_expiry() {
    let mut game = Game::new(702);
    clear_monsters(&mut game);
    game.player.statuses.push(StatusInstance {
        kind_id: crate::effect::STATUS_HALLUCINATION.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: Some("test.hallucination".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(
        &mut game,
        "test.item.neo-tsuyoshi.1",
        "demo.item.neo-tsuyoshi-special",
    );

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.neo-tsuyoshi.1".to_owned(),
            target: None,
        },
    );

    assert!(!game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));
    let tsuyoshi = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_TSUYOSHI)
        .expect("Neo-Tsuyoshi should remain active");
    assert_eq!(tsuyoshi.granted_modifiers.strength, 4);
    assert_eq!(tsuyoshi.granted_modifiers.constitution, 4);
    assert_eq!(tsuyoshi.granted_modifiers.max_hp, 50);
    assert!((91..=190).contains(&tsuyoshi.remaining_ticks));
    let restored = Game::from_save(game.to_save()).expect("Tsuyoshi status should round trip");
    assert_eq!(restored.snapshot(), game.snapshot());

    game.progress.attributes.strength = 118;
    game.progress.maximum_attributes.strength = 118;
    game.progress.attributes.constitution = 118;
    game.progress.maximum_attributes.constitution = 118;
    game.player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_TSUYOSHI)
        .expect("Tsuyoshi should remain active")
        .remaining_ticks = 1;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), true)
        .expect("Tsuyoshi expiry should resolve");

    assert!(!game.player_has_status_kind(STATUS_TSUYOSHI));
    assert!(game.progress.maximum_attributes.strength < 118);
    assert!(game.progress.maximum_attributes.constitution < 118);
    assert_eq!(
        game.progress.attributes.strength,
        game.progress.maximum_attributes.strength
    );
    assert_eq!(
        game.progress.attributes.constitution,
        game.progress.maximum_attributes.constitution
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 4);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerStatusExpired { status_kind_id }
            if status_kind_id == STATUS_TSUYOSHI
    )));
}

#[test]
fn p3_7_tsuyoshi_special_triggers_the_same_permanent_crash_immediately() {
    let mut game = Game::new(703);
    clear_monsters(&mut game);
    game.progress.attributes.strength = 18;
    game.progress.maximum_attributes.strength = 18;
    game.progress.attributes.constitution = 18;
    game.progress.maximum_attributes.constitution = 18;
    give_inventory_item(
        &mut game,
        "test.item.tsuyoshi.1",
        "demo.item.tsuyoshi-special",
    );

    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.tsuyoshi.1".to_owned(),
            target: None,
        },
    );

    assert_eq!(game.progress.attributes.strength, 17);
    assert_eq!(game.progress.maximum_attributes.strength, 17);
    assert_eq!(game.progress.attributes.constitution, 17);
    assert_eq!(game.progress.maximum_attributes.constitution, 17);
    assert!(!game.player_has_status_kind(STATUS_TSUYOSHI));
    assert!(game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));
}

#[test]
fn p3_3_treasure_detection_reports_stable_gold_pile_ids() {
    let mut game = Game::new(205);
    clear_monsters(&mut game);
    let gold_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.gold_piles = vec![
        GoldPile {
            id: "generated.gold.zeta".to_owned(),
            position: gold_position,
            amount: 20,
            appearance: GoldAppearanceDto::Gold,
            discovered: false,
        },
        GoldPile {
            id: "generated.gold.alpha".to_owned(),
            position: gold_position,
            amount: 10,
            appearance: GoldAppearanceDto::Copper,
            discovered: false,
        },
    ];
    give_inventory_item(
        &mut game,
        "test.item.treasure-detection.1",
        "demo.item.treasure-detection-scroll",
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.treasure-detection.1".to_owned(),
            target: None,
        },
    );

    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("treasure detection should expose detected gold piles");
    assert_eq!(detection.subject, AbilityDetectSubjectDto::Gold);
    assert_eq!(detection.category, "gold");
    assert_eq!(
        detection.detected_positions,
        vec![gold_position, gold_position]
    );
    assert_eq!(
        detection.detected_entity_ids,
        vec!["generated.gold.alpha", "generated.gold.zeta"]
    );
    assert!(game.gold_piles.iter().all(|pile| pile.discovered));
    assert_eq!(update.gold_piles.len(), 2);
    assert!(update.changed_cells.iter().any(|cell| {
        cell.position == gold_position && cell.item_id.as_deref() == Some("generated.gold.alpha")
    }));
}

#[test]
fn p3_5_acquirement_uses_stable_ids_current_position_and_exact_rng_draws() {
    let mut single = Game::new(503);
    clear_monsters(&mut single);
    give_inventory_item(
        &mut single,
        "test.item.acquirement.1",
        "demo.item.acquirement-scroll",
    );
    let position = single.player.position;
    let ids_before = single
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let draws_before = single.rng_draw_counter();
    let update = dispatch_next(
        &mut single,
        GameCommand::UseItem {
            item_id: "test.item.acquirement.1".to_owned(),
            target: None,
        },
    );
    let generated = single
        .items
        .iter()
        .filter(|item| !ids_before.contains(&item.id))
        .collect::<Vec<_>>();
    assert_eq!(generated.len(), 1);
    assert_eq!(generated[0].location, ItemLocation::Ground(position));
    assert_eq!(generated[0].quality, ItemQualityDto::Exceptional);
    assert!(generated[0].id.starts_with("generated.item."));
    assert_eq!(single.rng_draw_counter(), draws_before + 3);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-acquirement"
            && event.args.get("count").map(String::as_str) == Some("1")
    }));

    let mut multiple = Game::new(509);
    clear_monsters(&mut multiple);
    give_inventory_item(
        &mut multiple,
        "test.item.star-acquirement.1",
        "demo.item.star-acquirement-scroll",
    );
    let before_count = multiple.items.len();
    let draws_before = multiple.rng_draw_counter();
    dispatch_next(
        &mut multiple,
        GameCommand::UseItem {
            item_id: "test.item.star-acquirement.1".to_owned(),
            target: None,
        },
    );
    let generated_count = multiple.items.len() - (before_count - 1);
    assert!((2..=3).contains(&generated_count));
    assert_eq!(
        multiple.rng_draw_counter(),
        draws_before + 1 + 3 * generated_count as u64
    );
}

#[test]
fn p3_5_mundanity_splits_one_unit_and_rejects_fixed_artifacts_atomically() {
    let mut game = Game::new(521);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.mundanity.1",
        "demo.item.mundanity-scroll",
    );
    give_inventory_item(&mut game, "test.item.mundane-target.1", "demo.item.arrow");
    let target = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.mundane-target.1")
        .expect("target stack should exist");
    target.quantity = 3;
    target.quality = ItemQualityDto::Exceptional;
    target.affix_ids = vec!["demo.affix.frost-hunter".to_owned()];
    target.enchantments.to_hit = 2;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.mundanity.1".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.mundane-target.1".to_owned(),
            }),
        },
    );
    let remainder = game
        .items
        .iter()
        .find(|item| item.id == "test.item.mundane-target.1")
        .expect("remainder should keep the selected stack id");
    assert_eq!(remainder.quantity, 2);
    assert_eq!(remainder.quality, ItemQualityDto::Exceptional);
    let mundane = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow" && item.id != "test.item.mundane-target.1")
        .expect("one separated unit should become mundane");
    assert_eq!(mundane.quantity, 1);
    assert_eq!(mundane.quality, ItemQualityDto::Ordinary);
    assert!(mundane.affix_ids.is_empty());
    assert!(mundane.enchantments.is_empty());
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-mundanity"
            && event.args.get("split").map(String::as_str) == Some("true")
            && event.args.get("targetId") == Some(&mundane.id)
    }));
    Game::from_save(game.to_save()).expect("split mundane ammunition should round-trip");

    give_inventory_item(
        &mut game,
        "test.item.mundanity.2",
        "demo.item.mundanity-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.fixed-artifact.1",
        "demo.item.relic-blade",
    );
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.mundanity.2".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.fixed-artifact.1".to_owned(),
            }),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.item.mundanity.2")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn p3_5_crafting_splits_ammunition_identifies_ego_and_cancels_invalid_targets() {
    let mut game = Game::new(523);
    clear_monsters(&mut game);
    game.items.clear();
    give_inventory_item(
        &mut game,
        "test.item.crafting.1",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(&mut game, "test.item.crafting-target.1", "demo.item.arrow");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.crafting-target.1")
        .expect("ammunition should exist")
        .quantity = 3;
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.1".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.crafting-target.1".to_owned(),
            }),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.crafting-target.1")
            .expect("remainder should exist")
            .quantity,
        2
    );
    let crafted = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.arrow" && item.id != "test.item.crafting-target.1")
        .expect("one crafted unit should be split from the stack");
    assert_eq!(crafted.quantity, 1);
    assert_eq!(crafted.quality, ItemQualityDto::Exceptional);
    assert_eq!(crafted.affix_ids.len(), 1);
    let knowledge = &game.item_property_knowledge[&crafted.id];
    assert!(knowledge.appraised && knowledge.identified);
    assert!(knowledge.known_affix_ids.contains(&crafted.affix_ids[0]));
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-crafting"
            && event.args.get("targetId") == Some(&crafted.id)
            && event.args.get("split").map(String::as_str) == Some("true")
    }));
    Game::from_save(game.to_save()).expect("crafted ammunition should round-trip");

    give_inventory_item(
        &mut game,
        "test.item.crafting.2",
        "demo.item.crafting-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.invalid-crafting-target.1",
        "demo.item.ration-of-food",
    );
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.crafting.2".to_owned(),
            target: Some(TargetSelection::Item {
                item_id: "test.item.invalid-crafting-target.1".to_owned(),
            }),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(
        game.items
            .iter()
            .any(|item| item.id == "test.item.crafting.2")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn p3_5_rumour_is_localized_without_core_rng() {
    let mut game = Game::new(541);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.item.rumour.1", "demo.item.rumour-scroll");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.rumour.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    let event = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-rumour")
        .expect("rumour should emit a localized message reference");
    assert_eq!(
        event.args.get("rumourKey").map(String::as_str),
        Some("rumour-demo-warrens-depths")
    );
}

#[test]
fn fixed_artifact_selection_uses_source_order_ood_rarity_and_uniqueness() {
    let context = artifact_loot_context(60);
    let mut instant = Game::new(1);
    instant.rng = RfbRng::seeded(0);
    assert_eq!(instant.roll_instant_fixed_artifact_kind_id(&context), None);
    assert_eq!(instant.rng_draw_counter(), 1);

    let crisdurian_seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(15) == 0)
        .expect("a Crisdurian rarity seed should exist");
    let mut game = Game::new(1);
    game.rng = RfbRng::seeded(crisdurian_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,)
            .as_deref(),
        Some("demo.item.crisdurian")
    );
    assert_eq!(game.rng_draw_counter(), 1);

    game.generated_artifact_ids
        .insert("demo.item.crisdurian".to_owned());
    let slayer_seed = (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(60) == 0)
        .expect("a Slayer rarity seed should exist");
    game.rng = RfbRng::seeded(slayer_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,)
            .as_deref(),
        Some("demo.item.slayer")
    );
    assert_eq!(game.rng_draw_counter(), 1);

    game.generated_artifact_ids
        .insert("demo.item.slayer".to_owned());
    game.rng = RfbRng::seeded(0);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,),
        None
    );
    assert_eq!(game.rng_draw_counter(), 0);

    let rarity_rejection_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(15) != 0 && rng.bounded(60) != 0
        })
        .expect("a double rarity rejection seed should exist");
    game.generated_artifact_ids.clear();
    game.rng = RfbRng::seeded(rarity_rejection_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false,),
        None
    );
    assert_eq!(game.rng_draw_counter(), 2);

    let ood_rejection_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(4) != 0 && rng.bounded(4) != 0
        })
        .expect("a double OOD rejection seed should exist");
    game.rng = RfbRng::seeded(ood_rejection_seed);
    assert_eq!(
        game.roll_fixed_artifact_kind_id(
            &artifact_loot_context(58),
            Some("demo.item.executioners-sword"),
            false,
        ),
        None
    );
    assert_eq!(game.rng_draw_counter(), 2);
}

#[test]
fn item_generation_modes_keep_drafts_unallocated_until_commit() {
    let context = artifact_loot_context(60);

    let mut good = Game::new(2);
    good.rng = RfbRng::seeded(7);
    let good_draft = good
        .generate_one_loot_draft(&context, ItemGenerationMode::Good)
        .expect("Good generation should produce a draft");
    assert!(matches!(
        good_draft.quality,
        ItemQualityDto::Fine | ItemQualityDto::Exceptional
    ));

    let mut great = Game::new(2);
    great.rng = RfbRng::seeded(7);
    let great_draft = great
        .generate_one_loot_draft(&context, ItemGenerationMode::Great)
        .expect("Great generation should produce a draft");
    assert_eq!(great_draft.quality, ItemQualityDto::Exceptional);

    let mut artifact = Game::new(2);
    let serial_before = artifact.next_item_instance_serial;
    let fallback = (0..10_000).find_map(|seed| {
        artifact.rng = RfbRng::seeded(seed);
        let draft = artifact.generate_one_loot_draft(&context, ItemGenerationMode::Artifact)?;
        artifact
            .content
            .item(&draft.kind_id)
            .is_some_and(|item| item.artifact_generation.is_none())
            .then_some(draft)
    });
    let fallback = fallback.expect("an Artifact request fallback should exist");
    assert_eq!(fallback.quality, ItemQualityDto::Exceptional);
    assert_eq!(artifact.next_item_instance_serial, serial_before);
    let committed = artifact
        .commit_generated_item_draft(fallback, ItemLocation::Ground(artifact.player.position))
        .expect("an accepted draft should receive an instance ID");
    assert_eq!(artifact.next_item_instance_serial, serial_before + 1);
    assert!(
        artifact
            .content
            .item(&committed.kind_id)
            .is_some_and(|item| item.artifact_generation.is_none())
    );

    let mut fixed = Game::new(3);
    fixed.rng = RfbRng::seeded(crisdurian_seed_for_test());
    let kind_id = fixed
        .roll_fixed_artifact_kind_id(&context, Some("demo.item.executioners-sword"), false)
        .expect("Crisdurian should pass its rarity gate");
    let draft = fixed.fixed_artifact_draft(&context, kind_id);
    assert_eq!(draft.quality, ItemQualityDto::Ordinary);
    let item = fixed
        .commit_generated_item_draft(draft, ItemLocation::Ground(fixed.player.position))
        .expect("fixed artifact draft should commit");
    assert_eq!(item.kind_id, "demo.item.crisdurian");
    assert!(
        fixed
            .generated_artifact_ids
            .contains("demo.item.crisdurian")
    );
}

fn crisdurian_seed_for_test() -> u64 {
    (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(15) == 0)
        .expect("a Crisdurian rarity seed should exist")
}
