// SPDX-License-Identifier: MPL-2.0

use super::*;

#[test]
fn invoke_spirits_resolves_all_twenty_three_branches_deterministically() {
    let ability_id = "demo.ability.death-invoke-spirits";
    let prepare = |level| {
        let mut game = prepare_death_caster(0, level, ability_id);
        descend_one_floor(&mut game);
        clear_monsters(&mut game);
        game.debug_set_ability_casts_succeed(true);
        game.player.hp = 1;
        game.player.position = Position { x: 20, y: 10 };
        for x in 20..=28 {
            let index = game
                .index(Position { x, y: 10 })
                .expect("Invoke Spirits test corridor should remain in bounds");
            game.terrain[index] = "demo.terrain.floor".to_owned();
            game.glow[index] = false;
        }
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.invoke-spirits-target",
            "demo.actor.cave-orc",
            Position { x: 22, y: 10 },
            1_000,
            100,
            100,
            true,
        ));
        game.entities
            .last_mut()
            .expect("light-vulnerable target should exist")
            .resistances
            .set(DamageType::Light, ResistanceLevel::Vulnerable);
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.invoke-spirits-light-immune",
            "demo.actor.small-kobold",
            Position { x: 24, y: 10 },
            1_000,
            100,
            100,
            true,
        ));
        game
    };
    let low_level = prepare(10);
    let high_level = prepare(50);
    let cast = |branch_index, seed| {
        let mut game = if branch_index <= 18 {
            low_level.clone()
        } else {
            high_level.clone()
        };
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Invoke Spirits should resolve");
        (game, events)
    };
    let maximum_rolls = [
        7_u16, 13, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100, 103, 105, 107,
        109, 120,
    ];
    for branch_index in 0_u16..23 {
        let level = if branch_index <= 18 { 10 } else { 50 };
        let seed = (0..4_096)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                let _failure_roll = rng.bounded(100);
                let roll = u16::try_from(rng.bounded(100) + 1)
                    .expect("bounded roll should fit u16")
                    .saturating_add(level / 5);
                maximum_rolls.iter().position(|maximum| roll <= *maximum)
                    == Some(usize::from(branch_index))
            })
            .unwrap_or_else(|| panic!("Invoke Spirits branch {branch_index} should be reachable"));
        let (left, left_events) = cast(branch_index, seed);
        let (right, right_events) = cast(branch_index, seed);
        assert_eq!(left_events, right_events);
        assert_eq!(left.state_hash(), right.state_hash());
        let selected = left_events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    match resolution.effects.as_slice() {
                        [
                            AbilityEffectResolutionDto::RandomChoice {
                                branch_index: selected,
                                ..
                            },
                        ] => Some(*selected),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            selected,
            vec![branch_index],
            "branch {branch_index} should be selected exactly once; seed {seed}; events {left_events:?}"
        );
        assert!(!left_events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityEffectsResolved { resolution, .. }
                if matches!(
                    resolution.effects.as_slice(),
                    [AbilityEffectResolutionDto::NoOp { reason, .. }]
                        if reason.ends_with("-pending")
                )
        )));
        match branch_index {
            3 => assert!(left_events.iter().any(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::PolymorphTarget { changed: true, .. }]
                    )
            ))),
            7 => {
                let lit = left_events.iter().find_map(|event| match event {
                    DomainEvent::AbilityBeamDamage { resolution, .. }
                        if resolution.damage_type == DamageTypeDto::Light
                            && resolution.target_count == 1 =>
                    {
                        Some(&resolution.affected_positions)
                    }
                    _ => None,
                });
                let lit = lit.expect("line-light branch should project weak light damage");
                assert!(!lit.is_empty());
                assert!(
                    lit.iter().all(|position| left
                        .index(*position)
                        .is_some_and(|index| left.glow[index]))
                );
                assert!(left.entities.iter().any(|entity| {
                    entity.id == "generated.actor.invoke-spirits-target" && entity.hp < 1_000
                }));
                assert_eq!(
                    left.entities
                        .iter()
                        .find(|entity| entity.id == "generated.actor.invoke-spirits-light-immune")
                        .map(|entity| entity.hp),
                    Some(1_000)
                );
            }
            18 => assert!(left_events.iter().any(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::Earthquake { radius: 12, .. }]
                    )
            ))),
            19 => assert!(left_events.iter().any(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::AreaDestruction {
                            protected_floor: false,
                            affected_positions,
                            ..
                        }] if !affected_positions.is_empty()
                    )
            ))),
            22 => {
                assert!(left.player.hp > 1);
                assert!(
                    left_events
                        .iter()
                        .any(|event| matches!(event, DomainEvent::AbilityVisibleDamage { .. }))
                );
            }
            _ => {}
        }
    }
}

#[test]
fn invoke_spirits_lowest_outcome_updates_chance_and_unlife() {
    let ability_id = "demo.ability.death-invoke-spirits";
    let mut game = prepare_death_caster(0, 10, ability_id);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.debug_set_ability_casts_succeed(true);
    set_test_virtue(&mut game, 0, VirtueKindDto::Chance, 0);
    set_test_virtue(&mut game, 1, VirtueKindDto::Unlife, 0);
    let seed = (0..4_096)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            let _failure_roll = rng.bounded(100);
            rng.bounded(100) + 1 + 10 / 5 <= 7
        })
        .expect("the lowest Invoke Spirits outcome should be reachable");
    game.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();

    game.resolve_player_ability(
        ability_id,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Invoke Spirits should resolve");

    assert_eq!(game.virtue_current(VirtueKindDto::Chance), 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Unlife), 1);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RandomChoice {
                    branch_index: 0,
                    ..
                }]
            )
    )));
}

#[test]
fn mutation_eat_rock_and_midas_touch_commit_their_narrow_transactions() {
    let mut eater = active_source_mutation_game(43, "eat-rock", 8);
    let origin = eater.player.position;
    let rock = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    replace_terrain(&mut eater, rock, "demo.terrain.wall");
    eater.nutrition = 1_000;
    let mut events = Vec::new();
    eater
        .resolve_player_ability(
            "rfb.ability.mutation.eat-rock",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Eat Rock should resolve");
    assert_eq!(eater.player.position, rock);
    assert_eq!(
        eater.terrain[eater.index(rock).unwrap()],
        "demo.terrain.floor"
    );
    assert_eq!(eater.nutrition, 11_000);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::ConsumeTerrain {
                    position,
                    nutrition_before: 1000,
                    nutrition_after: 11000,
                    ..
                }] if *position == rock
            )
    )));

    let mut alchemist = active_source_mutation_game(47, "midas-touch", 10);
    give_inventory_item(&mut alchemist, "test.item.midas", "demo.item.broad-sword");
    let item = alchemist
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.midas")
        .unwrap();
    item.quantity = 2;
    let expected_gold = alchemist
        .content
        .item("demo.item.broad-sword")
        .unwrap()
        .base_value
        / 3
        * 2;
    let gold_before = alchemist.gold;
    alchemist
        .resolve_player_ability(
            "rfb.ability.mutation.midas-touch",
            TargetSelection::Item {
                item_id: "test.item.midas".to_owned(),
            },
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Midas Touch should resolve");
    assert!(
        !alchemist
            .items
            .iter()
            .any(|item| item.id == "test.item.midas")
    );
    assert_eq!(alchemist.gold, gold_before + expected_gold);
}

#[test]
fn mutation_earthquake_panic_hit_and_polymorph_enforce_their_boundaries() {
    let mut quake = active_source_mutation_game(71, "earthquake", 12);
    let mana_before = quake.resources["demo.resource.mana"].current;
    let draws_before = quake.rng_draw_counter();
    let mut events = Vec::new();
    quake
        .resolve_player_ability(
            "rfb.ability.mutation.earthquake",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("surface earthquake should reject cleanly");
    assert_eq!(quake.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(quake.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { .. }]
    ));
    descend_one_floor(&mut quake);
    clear_monsters(&mut quake);
    events.clear();
    quake
        .resolve_player_ability(
            "rfb.ability.mutation.earthquake",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("dungeon earthquake should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Earthquake { affected_positions, .. }]
                    if !affected_positions.is_empty()
            )
    )));

    let mut panic = active_source_mutation_game(73, "panic-hit", 10);
    let origin = panic.player.position;
    let target = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    for terrain in &mut panic.terrain {
        *terrain = "demo.terrain.floor".to_owned();
    }
    panic.entities.push(actor_from_runtime_spawn(
        "test.actor.panic",
        "demo.actor.gnome-mage",
        target,
        20,
        100,
        100,
        true,
    ));
    let mut panic_events = Vec::new();
    panic
        .resolve_player_ability(
            "rfb.ability.mutation.panic-hit",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut panic_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Panic Hit should resolve");
    assert!(panic_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::MeleeThenTeleport { target_entity_id, .. }]
                    if target_entity_id == "test.actor.panic"
            )
    )));

    let mut polymorph = active_source_mutation_game(79, "polymorph", 18);
    let disabled_ids = polymorph
        .content
        .mutations()
        .filter(|mutation| !mutation.random_selection_enabled)
        .map(|mutation| mutation.id.clone())
        .collect::<BTreeSet<_>>();
    events.clear();
    polymorph
        .resolve_player_ability(
            "rfb.ability.mutation.polymorph",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Polymorph Self should resolve");
    assert!(
        polymorph
            .progress
            .active_mutation_ids
            .is_disjoint(&disabled_ids)
    );
    let attributes = [
        AttributeKind::Strength,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Dexterity,
        AttributeKind::Constitution,
        AttributeKind::Charisma,
    ];
    assert!(attributes.into_iter().all(|attribute| {
        polymorph.progress.attributes.value(attribute)
            <= polymorph.progress.maximum_attributes.value(attribute)
            && polymorph.progress.maximum_attributes.value(attribute)
                <= polymorph.progress.attribute_potentials.value(attribute)
    }));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::PolymorphSelf { .. }]
            )
    )));
    let restored = Game::from_save_with_content(polymorph.to_save(), polymorph.content.clone())
        .expect("polymorph state should reload");
    assert_eq!(restored.state_hash(), polymorph.state_hash());
}

#[test]
fn mutation_detection_recall_and_resistance_use_existing_authoritative_state() {
    let mut detection = active_source_mutation_game(23, "det-curse", 7);
    give_inventory_item(&mut detection, "test.item.cursed", "demo.item.broad-sword");
    detection
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.cursed")
        .expect("cursed item")
        .curse = Some(ItemCurseSeverityDto::Normal);
    detection
        .resolve_player_ability(
            "rfb.ability.mutation.det-curse",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("curse detection should resolve");
    assert_eq!(
        detection.item_identification(
            detection
                .items
                .iter()
                .find(|item| item.id == "test.item.cursed")
                .unwrap()
        ),
        ItemIdentificationDto::Appraised
    );

    let mut recall = active_source_mutation_game(29, "recall", 17);
    let dungeon = recall
        .content
        .world(&recall.world_id)
        .unwrap()
        .dungeons
        .first()
        .expect("world dungeon");
    let dungeon_id = dungeon.id.clone();
    let floor_id = dungeon.root_floor_id.clone();
    recall.current_floor_id = floor_id.clone();
    recall.recall = Some(RecallStateDto {
        dungeon_id,
        floor_id,
        remaining_turns: None,
    });
    recall.debug_set_recall_delay_turns(Some(7));
    recall
        .resolve_player_ability(
            "rfb.ability.mutation.recall",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("recall should resolve");
    assert_eq!(recall.recall.unwrap().remaining_turns, Some(8));

    let mut resist = active_source_mutation_game(31, "resist", 25);
    resist
        .resolve_player_ability(
            "rfb.ability.mutation.resist",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("resist elements should resolve");
    assert_eq!(
        resist
            .player
            .statuses
            .iter()
            .filter(|status| status.kind_id.starts_with("rfb.status.resist-"))
            .count(),
        2
    );
}
