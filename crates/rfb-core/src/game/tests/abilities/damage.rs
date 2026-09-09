// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::event::BoltReflectionOutcome;
use crate::game::projectile_geometry::rfb_area_damage;

#[test]
fn damage_bonus_adds_flat_amount_to_monster_cast_damage() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    for step in 0..=3 {
        let index = game
            .index(Position {
                x: player.x + step,
                y: player.y,
            })
            .expect("corridor cell");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.cinder-test",
        "demo.actor.cinder-adept",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        8,
        100,
        100,
        true,
    ));

    let mut observed = None;
    for _ in 0..40 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                let damage = resolution
                    .effects
                    .iter()
                    .chain(
                        resolution
                            .targets
                            .iter()
                            .flat_map(|target| target.effects.iter()),
                    )
                    .find_map(|effect| match effect {
                        AbilityEffectResolutionDto::Damage { resolution, .. } => Some(resolution),
                        _ => None,
                    })
                    .expect("cinder cast should resolve damage");
                observed = Some((resolution.ability_id.clone(), damage.raw_damage));
            }
        }
        if observed.is_some() || game.player_is_dead() {
            break;
        }
    }
    let (ability_id, raw_damage) = observed.expect("cinder adept should cast within 40 turns");
    // Every cinder ability carries a flat bonus, so the raw roll always
    // lands inside dice-plus-bonus bounds without extra RNG cost.
    let bounds = match ability_id.as_str() {
        "demo.ability.cinder-bolt" => 5..=9,
        "demo.ability.cinder-burst" => 3..=6,
        "demo.ability.cinder-fan" => 3..=5,
        other => panic!("unexpected cinder ability {other}"),
    };
    assert!(
        bounds.contains(&raw_damage),
        "raw damage {raw_damage} must include the flat bonus for {ability_id}"
    );
}

#[test]
fn breath_damage_scales_with_caster_hp_and_caps_at_max() {
    fn breath_raw_damage(drake_hp: i32) -> i32 {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        for cell in game.terrain.iter_mut() {
            *cell = "demo.terrain.wall".to_owned();
        }
        let player = game.player.position;
        for step in 0..=3 {
            let index = game
                .index(Position {
                    x: player.x + step,
                    y: player.y,
                })
                .expect("corridor cell");
            game.terrain[index] = "demo.terrain.floor".to_owned();
        }
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.breath-test",
            "demo.actor.ash-drake",
            Position {
                x: player.x + 3,
                y: player.y,
            },
            12,
            100,
            100,
            true,
        ));
        game.entities.last_mut().expect("drake was just pushed").hp = drake_hp;

        for _ in 0..40 {
            let update = dispatch_next(&mut game, GameCommand::Wait);
            for event in &update.events {
                if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                    event.outcome.as_ref()
                {
                    assert_eq!(resolution.ability_id, "demo.ability.ash-breath");
                    let damage = resolution
                        .effects
                        .iter()
                        .chain(
                            resolution
                                .targets
                                .iter()
                                .flat_map(|target| target.effects.iter()),
                        )
                        .find_map(|effect| match effect {
                            AbilityEffectResolutionDto::Damage { resolution, .. } => {
                                Some(resolution)
                            }
                            _ => None,
                        })
                        .expect("breath cast should resolve damage");
                    return damage.raw_damage;
                }
            }
            if game.player_is_dead() {
                break;
            }
        }
        panic!("ash drake should breathe within 40 turns");
    }

    // Full vigor: 12 * 60% = 7 exceeds the elemental cap of 6.
    assert_eq!(breath_raw_damage(12), 6);
    // Wounded: 5 * 60% = 3 stays below the cap, so the breath weakens.
    assert_eq!(breath_raw_damage(5), 3);
}

#[test]
fn spawned_entities_get_content_declared_resistances_stamped() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    for step in 0..=3 {
        for dy in -2..=2 {
            if let Some(index) = game.index(Position {
                x: player.x + step,
                y: player.y + dy,
            }) {
                game.terrain[index] = "demo.terrain.floor".to_owned();
            }
        }
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.slag-test",
        "demo.actor.slag-crawler",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        10,
        100,
        100,
        true,
    ));

    for _ in 0..60 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                assert_eq!(resolution.ability_id, "demo.ability.slag-call");
                let summon = resolution
                    .summon
                    .as_ref()
                    .expect("kin summon should expose its resolution");
                let entity_id = &summon.entity_ids[0];
                let summoned = game
                    .entities
                    .iter()
                    .find(|entity| &entity.id == entity_id)
                    .expect("summoned crawler should exist");
                // The summon spawn path stamps the content-declared tiers;
                // the test-injected caster itself keeps the default profile.
                assert_eq!(
                    summoned.resistances.level(DamageType::Electricity),
                    ResistanceLevel::Resistant
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Fire),
                    ResistanceLevel::Immune
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Cold),
                    ResistanceLevel::Vulnerable
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Physical),
                    ResistanceLevel::Normal
                );
                return;
            }
        }
        if game.player_is_dead() {
            break;
        }
    }
    panic!("slag crawler should kin-summon within 60 turns");
}

#[test]
fn malediction_resolves_all_riders_and_skips_the_d1000_when_not_triggered() {
    #[derive(Clone, Copy)]
    enum ExpectedRider {
        None,
        DeathRay,
        Fear,
        Confusion,
        Stun,
    }

    let seed_for = |expected| {
        (0..100_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(100);
                for _ in 0..4 {
                    rng.bounded(4);
                }
                let trigger_roll = rng.bounded(5) + 1;
                if matches!(expected, ExpectedRider::None) {
                    return trigger_roll != 1 && rng.draw_counter == 6;
                }
                if trigger_roll != 1 {
                    return false;
                }
                let rider_roll = rng.bounded(1_000) + 1;
                match expected {
                    ExpectedRider::None => false,
                    ExpectedRider::DeathRay => rider_roll == 666,
                    ExpectedRider::Fear => rider_roll < 500 && rider_roll != 666,
                    ExpectedRider::Confusion => (500..800).contains(&rider_roll),
                    ExpectedRider::Stun => rider_roll >= 800,
                }
            })
            .expect("bounded seed search should cover every Malediction branch")
    };

    for expected in [
        ExpectedRider::None,
        ExpectedRider::DeathRay,
        ExpectedRider::Fear,
        ExpectedRider::Confusion,
        ExpectedRider::Stun,
    ] {
        let seed = seed_for(expected);
        let mut game = prepare_death_caster(0, 10, "demo.ability.death-malediction");
        game.debug_set_ability_casts_succeed(true);
        game.player.position = Position { x: 3, y: 3 };
        for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
            replace_terrain(&mut game, position, "demo.terrain.floor");
        }
        let target_id = "test.actor.malediction-target";
        game.entities.push(actor_from_runtime_spawn(
            target_id,
            "demo.actor.cinder-adept",
            Position { x: 4, y: 3 },
            100_000,
            100,
            100,
            true,
        ));
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();

        game.resolve_player_ability(
            "demo.ability.death-malediction",
            TargetSelection::Entity {
                entity_id: target_id.to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Malediction should resolve");

        let raw_damage = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityHit { damage, .. } => Some(damage.raw),
                _ => None,
            })
            .expect("Malediction should apply its primary hell-fire damage");
        let random_choices = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    Some(resolution.effects.iter().filter_map(|effect| match effect {
                        AbilityEffectResolutionDto::RandomChoice {
                            roll,
                            branch_index,
                            maximum_roll,
                            ..
                        } => Some((*roll, *branch_index, *maximum_roll)),
                        _ => None,
                    }))
                }
                _ => None,
            })
            .flatten()
            .collect::<Vec<_>>();

        if matches!(expected, ExpectedRider::None) {
            assert_eq!(game.rng_draw_counter(), 6);
            assert_eq!(random_choices.len(), 1);
            assert_ne!(random_choices[0].0, 1);
            continue;
        }

        assert_eq!(random_choices.len(), 2);
        assert_eq!(random_choices[0], (1, 1, 5));
        assert_eq!(random_choices[1].2, 1_000);
        let rider_resolution = events.iter().find_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => resolution
                .effects
                .iter()
                .find(|effect| !matches!(effect, AbilityEffectResolutionDto::RandomChoice { .. })),
            _ => None,
        });
        match expected {
            ExpectedRider::None => unreachable!(),
            ExpectedRider::DeathRay => assert!(matches!(
                rider_resolution,
                Some(AbilityEffectResolutionDto::DeathRay { power: 2_000, .. })
            )),
            ExpectedRider::Fear => assert!(matches!(
                rider_resolution,
                Some(AbilityEffectResolutionDto::ApplyStatus {
                    status_kind_id,
                    power: Some(10),
                    ..
                }) if status_kind_id == STATUS_FEAR
            )),
            ExpectedRider::Confusion => {
                let expected_power = 5_u16
                    .max(u16::try_from(raw_damage.min(100)).expect("damage power should fit u16"));
                assert!(matches!(
                    rider_resolution,
                    Some(AbilityEffectResolutionDto::ApplyStatus {
                        status_kind_id,
                        power: Some(power),
                        ..
                    }) if status_kind_id == STATUS_CONFUSION && *power == expected_power
                ));
            }
            ExpectedRider::Stun => assert!(matches!(
                rider_resolution,
                Some(AbilityEffectResolutionDto::ApplyStatus {
                    status_kind_id,
                    requested_duration_ticks,
                    power: None,
                    ..
                }) if status_kind_id == STATUS_STUN
                    && *requested_duration_ticks == u32::try_from(raw_damage).unwrap()
            )),
        }
    }
}

#[test]
fn death_vampiric_drain_heals_and_feeds_up_to_the_original_caps() {
    let mut game = prepare_death_caster(0, 50, "demo.ability.death-vampiric-drain");
    set_test_virtue(&mut game, 0, VirtueKindDto::Sacrifice, 0);
    set_test_virtue(&mut game, 1, VirtueKindDto::Vitality, 0);
    game.debug_set_ability_casts_succeed(true);
    let target = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.death-vampiric-drain",
        "demo.actor.gnome-mage",
        target,
        500,
        100,
        100,
        true,
    ));
    let maximum_hp = game.effective_player_max_hp();
    game.player.hp = maximum_hp - 1;
    game.nutrition = rfb_protocol::PLAYER_NUTRITION_BIRTH;

    game.resolve_player_ability(
        "demo.ability.death-vampiric-drain",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("vampiric drain should resolve");

    assert!(game.entities[0].hp < 500);
    assert_eq!(game.player.hp, maximum_hp);
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Sacrifice), -1);
    assert_eq!(game.virtue_current(VirtueKindDto::Vitality), -1);
}

#[test]
fn vampirism_true_retraces_the_path_after_each_kill() {
    let ability_id = "demo.ability.death-vampirism-true";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 36, ability_id);
        set_test_virtue(&mut game, 0, VirtueKindDto::Sacrifice, 0);
        set_test_virtue(&mut game, 1, VirtueKindDto::Vitality, 0);
        for (ordinal, x) in (game.player.position.x + 1..=game.player.position.x + 3).enumerate() {
            let position = Position {
                x,
                y: game.player.position.y,
            };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                &format!("test.actor.drain-{ordinal}"),
                "demo.actor.small-kobold",
                position,
                1,
                100,
                100,
                true,
            ));
        }
        game.player.hp = 1;
        let mut events = Vec::new();
        let mut removed = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        )
        .expect("Vampirism True should resolve");
        if removed.len() == 3 {
            selected = Some((game, events, removed));
            break;
        }
    }
    let (game, events, removed) = selected.expect("a deterministic triple drain should succeed");
    assert_eq!(removed.len(), 3);
    assert!(game.entities.is_empty());
    assert_eq!(game.virtue_current(VirtueKindDto::Sacrifice), -1);
    assert_eq!(game.virtue_current(VirtueKindDto::Vitality), -1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::DrainLife { .. }]
                    )
            ))
            .count(),
        3
    );
}

#[test]
fn bolt_or_beam_damage_uses_one_roll_and_changes_only_penetration() {
    let make_game = || {
        let mut game = Game::new(7);
        clear_monsters(&mut game);
        for (id, x) in [("test.actor.near", 4), ("test.actor.far", 5)] {
            let definition = game
                .content
                .actor("demo.actor.gloom-weaver")
                .expect("demo living target")
                .clone();
            let position = Position { x, y: 3 };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                id,
                &definition.id,
                position,
                definition.max_hp,
                definition.speed,
                100,
                true,
            ));
        }
        place_test_ground_item(
            &mut game,
            "test.item.near",
            "demo.item.arrow",
            Position { x: 4, y: 3 },
        );
        place_test_ground_item(
            &mut game,
            "test.item.far",
            "demo.item.arrow",
            Position { x: 5, y: 3 },
        );
        game
    };
    let make_ability = |game: &Game, id: &str, beam_chance_percent| {
        let mut ability = game
            .content
            .ability("demo.ability.death-dark-bolt")
            .expect("dark bolt should provide a bolt-or-beam definition")
            .clone();
        let AbilityEffectDefinition::BoltOrBeamDamage { .. } = ability.effect else {
            unreachable!("dark bolt must remain a bolt-or-beam ability");
        };
        ability.id = id.to_owned();
        ability.affects_ground_items = true;
        ability.effect = AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 3,
            damage_type: rfb_content::ActorDamageType::Fire,
            beam_chance_percent,
            beam_chance_modifier: 0,
        };
        ability
    };
    let path = vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }];

    let mut beam = make_game();
    let beam_ability = make_ability(&beam, "test.ability.beam", 100);
    let initial_hp = beam.entities[0].hp;
    let mut beam_events = Vec::new();
    beam.resolve_player_bolt_or_beam_damage_effect(
        &beam_ability,
        path.clone(),
        &mut beam_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("beam should resolve");
    assert!(beam.entities.iter().all(|actor| actor.hp < initial_hp));
    assert!(!beam.items.iter().any(|item| item.id == "test.item.near"));
    assert!(!beam.items.iter().any(|item| item.id == "test.item.far"));
    assert!(beam_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityBeamDamage { resolution, .. } if resolution.target_count == 2
    )));

    let mut bolt = make_game();
    let bolt_ability = make_ability(&bolt, "test.ability.bolt", 0);
    let mut bolt_events = Vec::new();
    bolt.resolve_player_bolt_or_beam_damage_effect(
        &bolt_ability,
        path,
        &mut bolt_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("bolt should resolve");
    assert!(bolt.entities[0].hp < initial_hp);
    assert_eq!(bolt.entities[1].hp, initial_hp);
    assert!(bolt.items.iter().any(|item| item.id == "test.item.near"));
    assert!(bolt.items.iter().any(|item| item.id == "test.item.far"));
    assert!(
        !bolt_events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityBeamDamage { .. }))
    );
}

#[test]
fn p86e_mirror_shield_reflects_monster_bolts_once_with_exact_three_of_four_gate() {
    let make_game = |seed| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        for cell in &mut game.terrain {
            *cell = "demo.terrain.wall".to_owned();
        }
        game.player.position = Position { x: 3, y: 3 };
        game.player.hp = 100;
        for x in 3..=5 {
            replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
        }
        let definition = game
            .content
            .actor("demo.actor.buzzy-beetle")
            .expect("reflecting source monster should exist")
            .clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.bolt-source",
            &definition.id,
            Position { x: 5, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        give_inventory_item(
            &mut game,
            "test.item.mirror-shield",
            "demo.item.mirror-shield",
        );
        game.items
            .last_mut()
            .expect("Mirror Shield should be granted")
            .location = ItemLocation::Equipped {
            slot_id: "left-hand".to_owned(),
        };
        game
    };
    let cast_bolt = |game: &mut Game| {
        let mut ability = game
            .content
            .ability("rfb-legacy.ability.bolt-physical-1d4")
            .expect("single-target bolt should exist")
            .clone();
        ability.effect = AbilityEffectDefinition::Damage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 4,
            damage_type: ActorDamageType::Fire,
        };
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("player should be a valid bolt target");
        assert!(matches!(
            plan.target,
            MonsterAbilityTargetPlan::Projectile { .. }
        ));
        let mut events = Vec::new();
        game.resolve_monster_ability_plan(
            0,
            "demo.actor.buzzy-beetle",
            &plan,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        events
    };

    let mut equipment_check = make_game(0);
    assert!(equipment_check.player_reflects_bolts());
    equipment_check
        .items
        .last_mut()
        .expect("Mirror Shield should remain present")
        .location = ItemLocation::Inventory;
    assert!(!equipment_check.player_reflects_bolts());

    let mut reflected_rolls = 0;
    for gate_roll in 0..4 {
        let seed = (0..10_000)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                assert_eq!(rng.bounded(1), 0, "1d1 damage must consume one draw");
                rng.bounded(4) == gate_roll
            })
            .expect("each reflection gate result should have a deterministic seed");
        let mut game = make_game(0);
        game.rng = RfbRng::seeded(seed);
        let events = cast_bolt(&mut game);
        let reflections = events
            .iter()
            .filter(|event| matches!(event, DomainEvent::BoltReflected { .. }))
            .count();
        if gate_roll == 0 {
            assert_eq!(reflections, 0);
            assert!(game.player.hp < 100);
        } else {
            reflected_rolls += 1;
            assert_eq!(reflections, 1, "one projectile may reflect only once");
            assert_eq!(game.player.hp, 100);
        }
    }
    assert_eq!(reflected_rolls, 3);
}

#[test]
fn mirror_shield_does_not_reflect_beams_balls_or_breaths() {
    let effects = [
        AbilityEffectDefinition::BeamDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 4,
            damage_type: ActorDamageType::Fire,
            maximum_range: None,
        },
        AbilityEffectDefinition::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 4,
            damage_type: ActorDamageType::Fire,
            radius: 1,
            target_category: None,
        },
        AbilityEffectDefinition::BreathDamage {
            hp_percent: 100,
            max_damage: 5,
            damage_type: ActorDamageType::Fire,
            radius: 1,
        },
    ];

    for effect in effects {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.player.position = Position { x: 3, y: 3 };
        game.player.hp = 100;
        for x in 3..=5 {
            replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
        }
        let definition = game
            .content
            .actor("demo.actor.cinder-adept")
            .expect("monster caster should exist")
            .clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.bolt-source",
            &definition.id,
            Position { x: 5, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        give_inventory_item(
            &mut game,
            "test.item.mirror-shield",
            "demo.item.mirror-shield",
        );
        game.items
            .last_mut()
            .expect("Mirror Shield should be granted")
            .location = ItemLocation::Equipped {
            slot_id: "left-hand".to_owned(),
        };
        let mut ability = game
            .content
            .ability("rfb-legacy.ability.bolt-physical-1d4")
            .expect("projectile ability should exist")
            .clone();
        ability.effect = effect;
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("player should be a valid target");
        let mut events = Vec::new();
        game.resolve_monster_ability_plan(
            0,
            "demo.actor.cinder-adept",
            &plan,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );

        assert!(game.player.hp < 100);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, DomainEvent::BoltReflected { .. }))
        );
    }
}

#[test]
fn reflecting_monsters_redirect_only_single_target_bolts() {
    let make_game = |seed| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.player.position = Position { x: 3, y: 3 };
        game.player.hp = 100;
        let definition = game
            .content
            .actor("demo.actor.buzzy-beetle")
            .expect("P30 reflector should exist")
            .clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.reflector",
            &definition.id,
            Position { x: 5, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        game
    };
    let make_bolt = |game: &Game| {
        let mut ability = game
            .content
            .ability("rfb-legacy.ability.bolt-physical-1d4")
            .expect("physical bolt should exist")
            .clone();
        let AbilityEffectDefinition::Damage { damage_type, .. } = ability.effect else {
            unreachable!("physical bolt must remain direct damage");
        };
        ability.effect = AbilityEffectDefinition::Damage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 19,
            damage_type,
        };
        ability
    };
    let path = vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }];
    let mut saw_normal_hit = false;
    let mut saw_reflected_landing = false;
    let mut saw_reflected_player_hit = false;

    for seed in 0..512 {
        let mut game = make_game(seed);
        let ability = make_bolt(&game);
        let reflector_hp = game.entities[0].hp;
        let mut events = Vec::new();
        game.resolve_player_projectile_damage_effect(
            &ability,
            path.clone(),
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("bolt should resolve");
        match events.iter().find_map(|event| match event {
            DomainEvent::BoltReflected { outcome, trace, .. } => Some((outcome, trace)),
            _ => None,
        }) {
            None => {
                saw_normal_hit = true;
                assert!(game.entities[0].hp < reflector_hp);
            }
            Some((BoltReflectionOutcome::Landed, trace)) => {
                saw_reflected_landing = true;
                assert_eq!(game.entities[0].hp, reflector_hp);
                assert_eq!(trace.origin, Position { x: 5, y: 3 });
            }
            Some((BoltReflectionOutcome::Hit { target_kind_id, .. }, trace)) => {
                saw_reflected_player_hit = true;
                assert_eq!(target_kind_id, &game.player.kind_id);
                assert!(game.player.hp < 100);
                assert_eq!(game.entities[0].hp, reflector_hp);
                assert_eq!(trace.origin, Position { x: 5, y: 3 });
            }
        }
        if saw_normal_hit && saw_reflected_landing && saw_reflected_player_hit {
            break;
        }
    }
    assert!(saw_normal_hit && saw_reflected_landing && saw_reflected_player_hit);

    let mut beam = make_game(0);
    let mut ability = beam
        .content
        .ability("demo.ability.death-dark-bolt")
        .expect("dark bolt should provide bolt-or-beam damage")
        .clone();
    let AbilityEffectDefinition::BoltOrBeamDamage {
        damage_type,
        damage_dice,
        damage_sides,
        damage_bonus,
        ..
    } = ability.effect
    else {
        unreachable!("dark bolt must remain bolt-or-beam damage");
    };
    ability.effect = AbilityEffectDefinition::BoltOrBeamDamage {
        damage_dice,
        damage_sides,
        damage_bonus,
        damage_type,
        beam_chance_percent: 100,
        beam_chance_modifier: 0,
    };
    let reflector_hp = beam.entities[0].hp;
    let mut events = Vec::new();
    beam.resolve_player_bolt_or_beam_damage_effect(
        &ability,
        path,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("beam should resolve");
    assert!(beam.entities[0].hp < reflector_hp);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::BoltReflected { .. }))
    );
}

#[test]
fn reflected_rock_uses_the_original_shards_and_sound_riders() {
    let make_game = |seed| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.player.position = Position { x: 3, y: 3 };
        game.player.hp = 100;
        for x in 3..=5 {
            replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
        }
        let definition = game
            .content
            .actor("demo.actor.buzzy-beetle")
            .expect("reflecting monster should exist")
            .clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.rock-reflector",
            &definition.id,
            Position { x: 5, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        game
    };
    let path = vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }];
    let mut saw_shards = false;
    let mut saw_sound = false;

    for seed in 0..2_048 {
        let mut game = make_game(seed);
        let mut ability = game
            .content
            .ability("rfb-legacy.ability.bolt-physical-1d4")
            .expect("single-target bolt should exist")
            .clone();
        ability.effect = AbilityEffectDefinition::Damage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 54,
            damage_type: ActorDamageType::Rock,
        };
        let mut events = Vec::new();
        game.resolve_player_projectile_damage_effect(
            &ability,
            path.clone(),
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("rock bolt should resolve");
        if !events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::BoltReflected {
                    outcome: BoltReflectionOutcome::Hit { target_kind_id, .. },
                    ..
                } if target_kind_id == &game.player.kind_id
            )
        }) {
            continue;
        }
        assert_eq!(game.player.hp, 46);
        if let Some(bleeding) = game
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLEEDING)
        {
            saw_shards = true;
            assert_eq!(bleeding.remaining_ticks, 27);
            assert!(!game.player_has_status_kind(STATUS_STUN));
        } else {
            saw_sound = true;
            let stun = game
                .player
                .statuses
                .iter()
                .find(|status| status.kind_id == STATUS_STUN)
                .expect("sound-side reflected rock should stun");
            assert!((1..=23).contains(&stun.remaining_ticks));
        }
        if saw_shards && saw_sound {
            break;
        }
    }
    assert!(saw_shards && saw_sound);
}

#[test]
fn rock_projectiles_destroy_trees_and_cold_vulnerable_ground_items() {
    let mut game = Game::new(0);
    let position = game.player.position;
    replace_terrain(&mut game, position, "demo.terrain.surface-tree");
    game.resolve_projectile_terrain_effects(&[position], DamageType::Rock, &mut BTreeSet::new());
    assert_eq!(
        game.terrain[game
            .index(position)
            .expect("player position should remain in bounds")],
        "demo.terrain.surface-grass"
    );

    give_inventory_item(&mut game, "test.rock-potion", "demo.item.antidote-potion");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.rock-potion")
        .expect("test potion should exist")
        .location = ItemLocation::Ground(position);
    game.resolve_ground_item_projectile_effects(
        "test.rock",
        &[position],
        DamageType::Rock,
        true,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(game.items.iter().all(|item| item.id != "test.rock-potion"));
}

#[test]
fn hand_of_doom_uses_a_save_gated_nonlethal_percentage_of_current_hp() {
    let template = Game::new(0);
    let ability = template
        .content
        .ability("rfb-legacy.ability.hand-of-doom")
        .expect("Hand of Doom should compile")
        .clone();
    let (seed, damaged, resolution, events) = (0..1_000_u64)
        .find_map(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            game.player.hp = 1_000;
            game.player.max_hp = 1_000;
            let mut events = Vec::new();
            let resolutions = game.resolve_monster_player_effects(
                "test.monster.shadow-fiend",
                "demo.actor.the-shadow-fiend",
                &ability,
                &mut events,
                &mut BTreeSet::new(),
            );
            let resolution = resolutions.into_iter().next().expect("one effect");
            matches!(resolution, AbilityEffectResolutionDto::Damage { .. })
                .then_some((seed, game, resolution, events))
        })
        .expect("a deterministic seed should fail the saving throw");

    let AbilityEffectResolutionDto::Damage {
        resolution: damage, ..
    } = resolution
    else {
        unreachable!()
    };
    assert!((410..=600).contains(&damage.raw_damage));
    assert_eq!(damaged.player.hp, 1_000 - damage.final_damage);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::SavingThrowChecked {
            succeeded: false,
            ..
        }
    )));

    let mut nonlethal = template;
    nonlethal.rng = RfbRng::seeded(seed);
    nonlethal.player.hp = 1;
    let resolutions = nonlethal.resolve_monster_player_effects(
        "test.monster.shadow-fiend",
        "demo.actor.the-shadow-fiend",
        &ability,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert_eq!(nonlethal.player.hp, 1);
    assert!(matches!(
        resolutions.as_slice(),
        [AbilityEffectResolutionDto::Damage { resolution, .. }]
            if resolution.raw_damage == 0 && resolution.final_damage == 0
    ));
}

const RACE_MAGIC_MISSILE_ABILITY_ID: &str = "rfb.ability.race.magic-missile";

const RACE_MIND_BLAST_ABILITY_ID: &str = "rfb.ability.race.mind-blast";

const RACE_IMP_FIRE_ABILITY_ID: &str = "rfb.ability.race.imp-fire";

const RACE_SPIT_ACID_ABILITY_ID: &str = "rfb.ability.race.spit-acid";

const RACE_THROW_BOULDER_ABILITY_ID: &str = "rfb.ability.race.throw-boulder";

#[test]
fn formal_kobold_poison_dart_is_a_fixed_level_poison_bolt_without_ammunition() {
    let mut game = Game::new_with_build_race_and_name(
        91,
        "demo.build.warrior",
        "rfb-legacy.race.kobold",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Kobold warrior should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    let level_eleven_experience = crate::stats::experience_required_for_level(11);
    game.apply_unscaled_player_experience(level_eleven_experience, &mut Vec::new());
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_POISON_DART_ABILITY_ID)
        .expect("Kobold Poison Dart should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Dexterity)
    );
    assert_eq!(locked.minimum_level, 12);
    assert_eq!(locked.base_resource_cost, 8);
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(12) - level_eleven_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    for position in [
        Position { x: 3, y: 3 },
        Position { x: 4, y: 3 },
        Position { x: 5, y: 3 },
    ] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    let target = game
        .content
        .actor("demo.actor.hill-orc")
        .expect("Hill Orc target should exist")
        .clone();
    for (id, x) in [("test.actor.near", 4), ("test.actor.far", 5)] {
        game.entities.push(actor_from_runtime_spawn(
            id,
            &target.id,
            Position { x, y: 3 },
            target.max_hp,
            target.speed,
            100,
            true,
        ));
    }
    let hp_before = game.player.hp;
    let serial_before = game.next_item_instance_serial;
    let draws_before = game.rng_draw_counter();
    let mut replay = game.clone();
    let mut resistant = game.clone();
    resistant.entities[0]
        .resistances
        .set(DamageType::Poison, ResistanceLevel::Resistant);
    let mut events = Vec::new();

    for cast in [&mut game, &mut replay, &mut resistant] {
        cast.resolve_player_ability(
            RACE_POISON_DART_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Kobold Poison Dart should resolve");
        events.clear();
    }

    assert_eq!(game.player.hp, hp_before - 8);
    assert_eq!(game.entities[0].hp, target.max_hp - 12);
    assert_eq!(game.entities[1].hp, target.max_hp);
    assert_eq!(game.next_item_instance_serial, serial_before);
    assert_eq!(game.rng_draw_counter(), draws_before + 2);
    assert_eq!(game.state_hash(), replay.state_hash());
    assert!(resistant.entities[0].hp > game.entities[0].hp);
    assert_eq!(resistant.next_item_instance_serial, serial_before);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Kobold Poison Dart save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn cyclops_throw_boulder_scales_stuns_and_round_trips_deterministically() {
    fn cast_boulder(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_THROW_BOULDER_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Cyclops boulder should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        103,
        "demo.build.high-mage-death",
        "rfb-legacy.race.cyclops",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Cyclops High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 1);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Sound),
        ResistanceLevel::Resistant
    );

    let level_nineteen_experience = crate::stats::experience_required_for_level(19);
    game.apply_unscaled_player_experience(level_nineteen_experience, &mut Vec::new());
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_THROW_BOULDER_ABILITY_ID)
        .expect("Cyclops boulder should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (0, 8));
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(20) - level_nineteen_experience,
        &mut Vec::new(),
    );
    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.cyclops-boulder-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;

    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_THROW_BOULDER_ABILITY_ID)
        .expect("Cyclops boulder should remain projected");
    assert_eq!(
        (available.base_resource_cost, available.resource_cost),
        (0, 8)
    );
    assert!(available.can_cast);
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 54,
            damage_type: DamageTypeDto::Rock,
            beam_chance_percent: 0,
            ..
        }]
    ));

    let mut level_fifty = game.clone();
    level_fifty.progress.level = 50;
    let level_fifty = level_fifty
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_THROW_BOULDER_ABILITY_ID)
        .expect("level-fifty Cyclops boulder");
    assert_eq!(level_fifty.resource_cost, 36);
    assert!(matches!(
        level_fifty.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_bonus: 250,
            damage_type: DamageTypeDto::Rock,
            ..
        }]
    ));

    let mut failed = game.clone();
    let failure_percent = available.failure_percent;
    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Cyclops boulder should have a failing percentile seed");
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = cast_boulder(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 8
    );
    assert_eq!(failed.entities[0].hp, 150);

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Cyclops boulder setup should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let mana_before = game.resources["demo.resource.mana"].current;
    let events = cast_boulder(&mut game);
    let restored_events = cast_boulder(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 8
    );
    assert_eq!(game.entities[0].hp, 96);
    assert_eq!(
        game.entities[0]
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_STUN)
            .expect("unresisted boulder should stun")
            .remaining_ticks,
        17
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityHit { damage, .. }
            if damage.raw == 54 && damage.applied == 54
    )));

    let mut resistant = Game::from_save_with_content(failed.to_save(), failed.content.clone())
        .expect("resistant boulder setup should reload");
    resistant.entities[0]
        .resistances
        .set(DamageType::Sound, ResistanceLevel::Resistant);
    resistant.debug_set_ability_casts_succeed(true);
    cast_boulder(&mut resistant);
    assert_eq!(resistant.entities[0].hp, 96);
    assert!(
        resistant.entities[0]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_STUN)
    );

    let mut human = Game::new_with_build_race_and_name(
        104,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 20;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.cyclops-form").status;
    form.granted_race_id = Some("rfb-legacy.race.cyclops".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_infravision_range(), 1);
    assert_eq!(
        human
            .effective_player_resistances()
            .level(DamageType::Sound),
        ResistanceLevel::Resistant
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_THROW_BOULDER_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_infravision_range(), 0);
    assert_eq!(
        human
            .effective_player_resistances()
            .level(DamageType::Sound),
        ResistanceLevel::Normal
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_THROW_BOULDER_ABILITY_ID)
    );
}

#[test]
fn klackon_acid_spit_and_speed_growth_follow_the_effective_race() {
    fn spit_acid(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_SPIT_ACID_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Klackon acid spit should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        107,
        "demo.build.high-mage-death",
        "rfb-legacy.race.klackon",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Klackon High-Mage should create");
    clear_monsters(&mut game);
    let base_speed = game.player_derived_stats().speed.value;
    assert_eq!(game.player_infravision_range(), 2);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Confusion),
        ResistanceLevel::Resistant
    );

    let level_eight_experience = crate::stats::experience_required_for_level(8);
    game.apply_unscaled_player_experience(level_eight_experience, &mut Vec::new());
    assert_eq!(game.player_derived_stats().speed.value, base_speed);
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SPIT_ACID_ABILITY_ID)
        .expect("Klackon acid spit should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Dexterity)
    );
    assert_eq!(locked.minimum_level, 9);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (9, 10));
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(9) - level_eight_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SPIT_ACID_ABILITY_ID)
        .expect("Klackon acid spit should remain projected");
    assert!(available.can_cast);
    assert_eq!(
        (available.base_resource_cost, available.resource_cost),
        (9, 10)
    );
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 17,
            damage_type: DamageTypeDto::Acid,
            area_from_level: 25,
            radius: 2,
            ..
        }]
    ));
    game.progress.level = 10;
    assert_eq!(game.player_derived_stats().speed.value, base_speed + 1);
    game.progress.level = 9;

    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.klackon-acid-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Klackon acid spit should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = spit_acid(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 10
    );
    assert_eq!(failed.entities[0].hp, 150);

    let mut bolt = game.clone();
    bolt.debug_set_ability_casts_succeed(true);
    let bolt_events = spit_acid(&mut bolt);
    assert_eq!(bolt.entities[0].hp, 132);
    assert!(
        bolt_events
            .iter()
            .all(|event| !matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );
    assert!(bolt_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityHit { damage, .. }
            if damage.raw == 18 && damage.applied == 18
    )));

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(25)
            - crate::stats::experience_required_for_level(9),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    assert_eq!(game.player_derived_stats().speed.value, base_speed + 2);
    let area = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SPIT_ACID_ABILITY_ID)
        .expect("level-twenty-five Klackon acid spit");
    assert_eq!(area.resource_cost, 14);
    assert!(matches!(
        area.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_bonus: 49,
            area_from_level: 25,
            radius: 2,
            ..
        }]
    ));

    let mut level_fifty = game.clone();
    level_fifty.progress.level = 50;
    assert_eq!(
        level_fifty.player_derived_stats().speed.value,
        base_speed + 5
    );
    let level_fifty = level_fifty
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SPIT_ACID_ABILITY_ID)
        .expect("level-fifty Klackon acid spit");
    assert_eq!(level_fifty.resource_cost, 19);
    assert!(matches!(
        level_fifty.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_bonus: 99,
            ..
        }]
    ));

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Klackon acid-spit setup should reload");
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let mana_before = game.resources["demo.resource.mana"].current;
    let events = spit_acid(&mut game);
    let restored_events = spit_acid(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 14
    );
    assert_eq!(game.entities[0].hp, 100);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { resolution, .. }
            if resolution.radius == 2
    )));

    let mut human = Game::new_with_build_race_and_name(
        108,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 20;
    let human_speed = human.player_derived_stats().speed.value;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.klackon-form").status;
    form.granted_race_id = Some("rfb-legacy.race.klackon".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_infravision_range(), 2);
    assert_eq!(human.player_derived_stats().speed.value, human_speed + 2);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Resistant
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_SPIT_ACID_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_infravision_range(), 0);
    assert_eq!(human.player_derived_stats().speed.value, human_speed);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Normal
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_SPIT_ACID_ABILITY_ID)
    );
}

#[test]
fn dark_elf_magic_missile_capacity_and_sight_follow_the_effective_race() {
    fn cast_magic_missile(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_MAGIC_MISSILE_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dark-Elf magic missile should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        109,
        "demo.build.high-mage-death",
        "rfb-legacy.race.dark-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dark-Elf High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(game.player_see_invisible_sources(), 0);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    let human_mana = Game::new_with_build_race_and_name(
        109,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create")
    .resources["demo.resource.mana"]
        .maximum;
    assert!(game.resources["demo.resource.mana"].maximum > human_mana);

    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MAGIC_MISSILE_ABILITY_ID)
        .expect("Dark-Elf magic missile should be projected");
    assert_eq!(projected.source, AbilitySourceDto::Race);
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(projected.minimum_level, 1);
    assert_eq!(
        (projected.base_resource_cost, projected.resource_cost),
        (2, 2)
    );
    assert!(projected.can_cast);
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 3,
            damage_sides: 4,
            damage_bonus: 5,
            damage_type: DamageTypeDto::Physical,
            beam_chance_percent: 1,
            ..
        }]
    ));

    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.dark-elf-missile-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    let maximum_mana = game.resources["demo.resource.mana"].maximum;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = maximum_mana;

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(projected.failure_percent)
        })
        .expect("Dark-Elf magic missile should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = cast_magic_missile(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 2
    );
    assert_eq!(failed.entities[0].hp, 150);

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Dark-Elf magic-missile setup should reload");
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let events = cast_magic_missile(&mut game);
    let restored_events = cast_magic_missile(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(game.entities[0].hp < 150);

    game.progress.level = 19;
    assert_eq!(game.player_see_invisible_sources(), 0);
    game.progress.level = 20;
    assert_eq!(game.player_see_invisible_sources(), 1);

    let mut high_mage_fifty = game.clone();
    high_mage_fifty.progress.level = 50;
    let high_mage_missile = high_mage_fifty
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MAGIC_MISSILE_ABILITY_ID)
        .expect("level-fifty Dark-Elf High-Mage magic missile");
    assert!(matches!(
        high_mage_missile.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 12,
            damage_bonus: 15,
            beam_chance_percent: 50,
            ..
        }]
    ));

    let mut warrior = Game::new_with_build_race_and_name(
        110,
        "demo.build.warrior",
        "rfb-legacy.race.dark-elf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dark-Elf Warrior should create");
    warrior.progress.level = 50;
    let warrior_missile = warrior
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MAGIC_MISSILE_ABILITY_ID)
        .expect("level-fifty Dark-Elf Warrior magic missile");
    assert!(matches!(
        warrior_missile.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 12,
            damage_bonus: 0,
            beam_chance_percent: 15,
            ..
        }]
    ));

    let mut human = Game::new_with_build_race_and_name(
        111,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create");
    human.progress.level = 20;
    human.refresh_player_resource_maxima();
    let human_mana = human.resources["demo.resource.mana"].maximum;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.dark-elf-form").status;
    form.granted_race_id = Some("rfb-legacy.race.dark-elf".to_owned());
    human.player.statuses.push(form);
    human.refresh_player_resource_maxima();
    assert_eq!(human.player_infravision_range(), 5);
    assert_eq!(human.player_see_invisible_sources(), 1);
    assert!(human.resources["demo.resource.mana"].maximum > human_mana);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_MAGIC_MISSILE_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    human.refresh_player_resource_maxima();
    assert_eq!(human.player_infravision_range(), 0);
    assert_eq!(human.player_see_invisible_sources(), 0);
    assert_eq!(human.resources["demo.resource.mana"].maximum, human_mana);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Normal
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_MAGIC_MISSILE_ABILITY_ID)
    );
}

#[test]
fn mindflayer_mind_blast_sustains_and_senses_follow_the_effective_race() {
    fn cast_mind_blast(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_MIND_BLAST_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Mindflayer mind blast should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        112,
        "demo.build.high-mage-death",
        "rfb-legacy.race.mindflayer",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Mindflayer High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 4);
    assert!(game.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(game.player_sustains_attribute(AttributeKind::Wisdom));
    assert!(!game.player_sustains_attribute(AttributeKind::Strength));
    assert_eq!(game.player_see_invisible_sources(), 0);
    assert!(!game.player_has_permanent_telepathy());

    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MIND_BLAST_ABILITY_ID)
        .expect("Mindflayer mind blast should be projected");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(locked.minimum_level, 5);
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5),
        &mut Vec::new(),
    );
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MIND_BLAST_ABILITY_ID)
        .expect("level-five Mindflayer mind blast");
    assert_eq!(
        (projected.base_resource_cost, projected.resource_cost),
        (3, 3)
    );
    assert!(projected.can_cast);
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::Damage {
            damage_dice: 3,
            damage_sides: 3,
            damage_bonus: 6,
            damage_type: DamageTypeDto::Psi,
            ..
        }]
    ));

    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.mindflayer-blast-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    let maximum_mana = game.resources["demo.resource.mana"].maximum;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = maximum_mana;

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(projected.failure_percent)
        })
        .expect("Mindflayer mind blast should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = cast_mind_blast(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 3
    );
    assert_eq!(failed.entities[0].hp, 150);

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Mindflayer mind-blast setup should reload");
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let events = cast_mind_blast(&mut game);
    let restored_events = cast_mind_blast(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(game.entities[0].hp < 150);

    game.progress.level = 14;
    assert_eq!(game.player_see_invisible_sources(), 0);
    game.progress.level = 15;
    assert_eq!(game.player_see_invisible_sources(), 1);
    game.progress.level = 29;
    assert!(!game.player_has_permanent_telepathy());
    game.progress.level = 30;
    assert!(game.player_has_permanent_telepathy());

    let mut high_mage_fifty = game.clone();
    high_mage_fifty.progress.level = 50;
    let high_mage_blast = high_mage_fifty
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MIND_BLAST_ABILITY_ID)
        .expect("level-fifty Mindflayer High-Mage mind blast");
    assert!(matches!(
        high_mage_blast.effects.as_slice(),
        [AbilityEffectSpecDto::Damage {
            damage_dice: 12,
            damage_bonus: 15,
            ..
        }]
    ));

    let mut warrior = Game::new_with_build_race_and_name(
        113,
        "demo.build.warrior",
        "rfb-legacy.race.mindflayer",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Mindflayer Warrior should create");
    warrior.progress.level = 50;
    let warrior_blast = warrior
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_MIND_BLAST_ABILITY_ID)
        .expect("level-fifty Mindflayer Warrior mind blast");
    assert!(matches!(
        warrior_blast.effects.as_slice(),
        [AbilityEffectSpecDto::Damage {
            damage_dice: 12,
            damage_bonus: 0,
            ..
        }]
    ));

    let mut human = Game::new_with_build_race_and_name(
        114,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 30;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.mindflayer-form").status;
    form.granted_race_id = Some("rfb-legacy.race.mindflayer".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_infravision_range(), 4);
    assert_eq!(human.player_see_invisible_sources(), 1);
    assert!(human.player_has_permanent_telepathy());
    assert!(human.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(human.player_sustains_attribute(AttributeKind::Wisdom));
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_MIND_BLAST_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_infravision_range(), 0);
    assert_eq!(human.player_see_invisible_sources(), 0);
    assert!(!human.player_has_permanent_telepathy());
    assert!(!human.player_sustains_attribute(AttributeKind::Intelligence));
    assert!(!human.player_sustains_attribute(AttributeKind::Wisdom));
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_MIND_BLAST_ABILITY_ID)
    );
}

#[test]
fn imp_fire_upgrade_and_demon_traits_follow_the_effective_race() {
    fn cast_imp_fire(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_IMP_FIRE_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Imp fire power should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        115,
        "demo.build.high-mage-death",
        "rfb-legacy.race.imp",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Imp High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(game.player_see_invisible_sources(), 0);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    assert!(
        game.character_definitions()
            .expect("Imp character definitions")
            .1
            .tags
            .iter()
            .any(|tag| tag == "demon")
    );

    let level_eight_experience = crate::stats::experience_required_for_level(8);
    game.apply_unscaled_player_experience(level_eight_experience, &mut Vec::new());
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_IMP_FIRE_ABILITY_ID)
        .expect("Imp fire power should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(locked.minimum_level, 9);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (8, 8));
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(9) - level_eight_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let bolt = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_IMP_FIRE_ABILITY_ID)
        .expect("level-nine Imp fire power");
    assert!(bolt.can_cast);
    assert!(matches!(
        bolt.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 8,
            damage_type: DamageTypeDto::Fire,
            area_from_level: 30,
            radius: 2,
            ..
        }]
    ));

    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.imp-fire-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(bolt.failure_percent)
        })
        .expect("Imp fire power should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = cast_imp_fire(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 8
    );
    assert_eq!(failed.entities[0].hp, 150);

    let mut level_nine = game.clone();
    level_nine.debug_set_ability_casts_succeed(true);
    let bolt_events = cast_imp_fire(&mut level_nine);
    assert_eq!(level_nine.entities[0].hp, 141);
    assert!(bolt_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityHit { damage, .. }
            if damage.raw == 9 && damage.applied == 9
    )));
    assert!(
        bolt_events
            .iter()
            .all(|event| !matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10)
            - crate::stats::experience_required_for_level(9),
        &mut Vec::new(),
    );
    assert_eq!(game.player_see_invisible_sources(), 1);
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(29)
            - crate::stats::experience_required_for_level(10),
        &mut Vec::new(),
    );
    let level_twenty_nine = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_IMP_FIRE_ABILITY_ID)
        .expect("level-twenty-nine Imp fire power");
    assert_eq!(level_twenty_nine.resource_cost, 8);
    assert!(matches!(
        level_twenty_nine.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: 1,
            damage_bonus: 28,
            ..
        }]
    ));

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(30)
            - crate::stats::experience_required_for_level(29),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    replace_terrain(&mut game, Position { x: 5, y: 3 }, "demo.terrain.floor");
    game.push_generated_actor(
        "test.imp-fire-area-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 5, y: 3 },
    );
    let fire_ball = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_IMP_FIRE_ABILITY_ID)
        .expect("level-thirty Imp fire power");
    assert_eq!(
        (fire_ball.base_resource_cost, fire_ball.resource_cost),
        (8, 15)
    );
    assert!(matches!(
        fire_ball.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: 2,
            damage_sides: 1,
            damage_bonus: 58,
            area_from_level: 30,
            radius: 2,
            ..
        }]
    ));

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Imp fire-ball setup should reload");
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let mana_before = game.resources["demo.resource.mana"].current;
    let events = cast_imp_fire(&mut game);
    let restored_events = cast_imp_fire(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 15
    );
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.imp-fire-target")
            .expect("fire-ball center target")
            .hp,
        90
    );
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.imp-fire-area-target")
            .expect("fire-ball radius target")
            .hp,
        120
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { resolution, .. }
            if resolution.radius == 2 && resolution.base_raw_damage == 60
    )));

    let mut human = Game::new_with_build_race_and_name(
        116,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 30;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.imp-form").status;
    form.granted_race_id = Some("rfb-legacy.race.imp".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_infravision_range(), 3);
    assert_eq!(human.player_see_invisible_sources(), 1);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    assert!(
        human
            .character_definitions()
            .expect("polymorphed character definitions")
            .1
            .tags
            .iter()
            .any(|tag| tag == "demon")
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_IMP_FIRE_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_infravision_range(), 0);
    assert_eq!(human.player_see_invisible_sources(), 0);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Normal
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_IMP_FIRE_ABILITY_ID)
    );
}

#[test]
fn draconian_breath_uses_current_hp_maturity_shape_and_deadly_upgrade() {
    const ABILITY_ID: &str = "rfb.ability.race.draconian-red-breath";

    let mut base = Game::new_with_build_race_and_name(
        117,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create");
    clear_monsters(&mut base);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.draconian-form").status;
    form.granted_race_id = Some("rfb-legacy.race.draconian-red".to_owned());
    base.player.statuses.push(form);

    for (level, hp, deadly, expected_shape, expected_damage, expected_radius, expected_cost) in [
        (19, 400, false, "bolt", 54, 0, 7),
        (20, 400, false, "beam", 56, 0, 8),
        (29, 400, false, "beam", 68, 0, 12),
        (30, 400, false, "cone", 70, 2, 13),
        (40, 400, false, "cone", 100, 3, 19),
        (50, 1_000, false, "cone", 250, 3, 26),
        (50, 1_000, true, "cone", 500, 3, 40),
    ] {
        let mut game = base.clone();
        game.progress.level = level;
        game.player.hp = hp;
        if deadly {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.draconian-breath".to_owned());
        }
        let projected = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == ABILITY_ID)
            .expect("Draconian breath should be projected from the effective race");
        assert_eq!(projected.source, AbilitySourceDto::Race);
        assert_eq!(projected.minimum_level, 1);
        assert_eq!(projected.base_resource_cost, 0);
        assert_eq!(projected.resource_cost, expected_cost, "level {level}");
        assert_eq!(
            projected.governing_attribute,
            Some(rfb_protocol::AttributeKindDto::Constitution)
        );
        let (shape, damage, radius, damage_type) = match projected.effects.as_slice() {
            [
                AbilityEffectSpecDto::Damage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                    ..
                },
            ] => {
                assert_eq!((*damage_dice, *damage_sides), (0, 0));
                ("bolt", *damage_bonus, 0, *damage_type)
            }
            [
                AbilityEffectSpecDto::BeamDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                    ..
                },
            ] => {
                assert_eq!((*damage_dice, *damage_sides), (0, 0));
                ("beam", *damage_bonus, 0, *damage_type)
            }
            [
                AbilityEffectSpecDto::ConeDamage {
                    damage_dice,
                    damage_sides,
                    damage_bonus,
                    damage_type,
                    radius,
                    ..
                },
            ] => {
                assert_eq!((*damage_dice, *damage_sides), (0, 0));
                ("cone", *damage_bonus, *radius, *damage_type)
            }
            effects => panic!("unexpected Draconian breath projection: {effects:?}"),
        };
        assert_eq!(shape, expected_shape, "level {level}");
        assert_eq!(damage, expected_damage, "level {level}");
        assert_eq!(radius, expected_radius, "level {level}");
        assert_eq!(damage_type, DamageTypeDto::Fire, "level {level}");
    }

    let mut game = base;
    game.progress.level = 30;
    game.player.hp = 400;
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 0;
    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.draconian-breath-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Draconian breath should resolve");

    assert_eq!(game.player.hp, 387);
    assert_eq!(game.entities[0].hp, 80);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityCastSucceeded { resolution }
            if resolution.resource_cost == 13
                && resolution.resource_paid == 0
                && resolution.hp_paid == 13
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityConeDamage { resolution, .. }
            if resolution.radius == 2 && resolution.base_raw_damage == 70
    )));
}

#[test]
fn draconian_strike_applies_elemental_stun_confusion_vorpal_and_vampiric_modes() {
    fn base_game() -> Game {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.progress.level = 35;
        game.player.hp = 1;
        game.player.position = Position { x: 3, y: 3 };
        game.push_generated_actor(
            "test.draconian-strike-target".to_owned(),
            "demo.actor.warrens-keeper",
            Position { x: 4, y: 3 },
        );
        game.entities[0].hp = 10_000;
        game.entities[0].max_hp = 10_000;
        game
    }

    fn damage_with(
        base: &Game,
        seed: u64,
        mode: Option<DraconianStrikeModeDefinition>,
    ) -> (Game, i32, Vec<DomainEvent>) {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        match mode {
            Some(mode) => {
                game.resolve_player_draconian_strike(
                    0,
                    mode,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .expect("Draconian strike should resolve");
            }
            None => {
                game.resolve_player_melee(
                    0,
                    false,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .expect("control melee should resolve");
            }
        }
        let damage = 10_000 - game.entities[0].hp;
        (game, damage, events)
    }

    let base = base_game();
    let hit_seed = (0..10_000)
        .find(|seed| damage_with(&base, *seed, None).1 > 5)
        .expect("a deterministic melee hit seed should exist");
    let (_, normal_damage, _) = damage_with(&base, hit_seed, None);
    let (_, fire_damage, _) =
        damage_with(&base, hit_seed, Some(DraconianStrikeModeDefinition::Fire));
    assert!(fire_damage > normal_damage);

    let (stunned, stun_damage, _) =
        damage_with(&base, hit_seed, Some(DraconianStrikeModeDefinition::Stun));
    assert_eq!(stun_damage, normal_damage);
    assert!(
        stunned.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );

    let confusion_seed = (0..10_000)
        .find(|seed| {
            damage_with(&base, *seed, Some(DraconianStrikeModeDefinition::Confusion))
                .0
                .entities[0]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_CONFUSION)
        })
        .expect("a deterministic confusion seed should exist");
    assert!(
        damage_with(
            &base,
            confusion_seed,
            Some(DraconianStrikeModeDefinition::Confusion),
        )
        .2
        .iter()
        .any(|event| matches!(event, DomainEvent::ConfusingStrikeApplied { .. }))
    );

    let vorpal_seed = (0..100_000)
        .find(|seed| {
            let normal = damage_with(&base, *seed, None).1;
            let vorpal = damage_with(&base, *seed, Some(DraconianStrikeModeDefinition::Vorpal)).1;
            normal > 0 && vorpal > normal
        })
        .expect("a deterministic vorpal seed should exist");
    assert!(
        damage_with(
            &base,
            vorpal_seed,
            Some(DraconianStrikeModeDefinition::Vorpal),
        )
        .1 > damage_with(&base, vorpal_seed, None).1
    );

    let (vampiric, vampiric_damage, events) = damage_with(
        &base,
        hit_seed,
        Some(DraconianStrikeModeDefinition::Vampiric),
    );
    assert!(vampiric_damage > 5);
    assert!(vampiric.player.hp > 1);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerVampiricHealed { resolution } if resolution.applied > 0
    )));
}

#[test]
fn mutation_vampirism_feeds_without_crossing_the_original_full_cap() {
    let mut game = active_source_mutation_game(37, "vampirism", 2);
    let origin = game.player.position;
    let target = Position {
        x: origin.x + 1,
        y: origin.y,
    };
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.vampirism",
        "demo.actor.gnome-mage",
        target,
        20,
        50,
        100,
        true,
    ));
    game.nutrition = rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 2;

    game.resolve_player_ability(
        "rfb.ability.mutation.vampirism",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("vampirism should resolve");

    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
}

#[test]
fn mutation_spit_acid_changes_from_bolt_to_area_at_level_twenty_five() {
    let cast = |level| {
        let mut game = active_source_mutation_game(41, "spit-acid", level);
        let origin = game.player.position;
        for step in 0..=3 {
            replace_terrain(
                &mut game,
                Position {
                    x: origin.x + step,
                    y: origin.y,
                },
                "demo.terrain.floor",
            );
        }
        let mut events = Vec::new();
        game.resolve_player_ability(
            "rfb.ability.mutation.spit-acid",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("acid spit should resolve");
        events
    };

    let bolt = cast(24);
    assert!(
        bolt.iter()
            .any(|event| matches!(event, DomainEvent::AbilityLanded { .. }))
    );
    assert!(
        !bolt
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityAreaDamage { .. }))
    );

    let area = cast(25);
    assert!(area.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.radius == 2
    )));
}

#[test]
fn level_based_jump_damage_uses_no_damage_rng_then_blinks() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.items
        .retain(|item| item.location != ItemLocation::Inventory);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    let caster = Position {
        x: player.x + 1,
        y: player.y,
    };
    let landing = Position {
        x: caster.x + 4,
        y: caster.y,
    };
    for position in [player, caster, landing] {
        let index = game.index(position).expect("test cell should exist");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }
    game.player.hp = 1;
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.blinking-light",
        "demo.actor.blinking-light",
        caster,
        44,
        115,
        100,
        true,
    ));

    let ability = game
        .content
        .ability("rfb-legacy.ability.jump-fire-l31")
        .expect("Orc Cave jump ability should compile")
        .clone();
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 31,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: rfb_content::ActorDamageType::Fire,
            radius: 5,
            blink_radius: 10,
        }
    ));
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("player inside the caster-centered burst should be valid");
    let MonsterAbilityTargetPlan::JumpDamage {
        affected_positions,
        destinations,
    } = &plan.target
    else {
        panic!("JMP_FIRE should plan a caster-centered jump burst");
    };
    assert!(affected_positions.contains(&caster));
    assert!(affected_positions.iter().all(|position| {
        caster
            .x
            .abs_diff(position.x)
            .max(caster.y.abs_diff(position.y))
            <= 5
    }));
    assert_eq!(destinations, &[landing]);

    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed_entities = Vec::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.blinking-light",
        &plan,
        &mut events,
        &mut changed,
        &mut removed_entities,
    );

    assert_eq!(game.rng_draw_counter(), draws + 1);
    assert_eq!(game.entities[0].position, landing);
    let AbilityEffectResolutionDto::Damage {
        resolution: damage, ..
    } = &resolution.targets[0].effects[0]
    else {
        panic!("JMP_FIRE should damage the player");
    };
    assert_eq!(damage.raw_damage, 38);
    assert_eq!(damage.final_damage, rfb_area_damage(damage.raw_damage, 1));
    assert_eq!(damage.damage_type, DamageTypeDto::Fire);
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::PlayerDied { .. },
            DomainEvent::MonsterBlinked { resolution, .. }
        ] if resolution.from == caster && resolution.to == landing
    ));
}

#[test]
fn bird_drop_flies_away_or_drops_targets_with_levitation_reduction() {
    fn cast(
        seed: u64,
        levitating: bool,
    ) -> (
        Game,
        MonsterAbilityPlanResolution,
        Vec<DomainEvent>,
        u64,
        Position,
        Position,
        Position,
    ) {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        for cell in &mut game.terrain {
            *cell = "demo.terrain.wall".to_owned();
        }
        let player = game.player.position;
        let caster = Position {
            x: player.x + 3,
            y: player.y,
        };
        let landing = Position {
            x: caster.x,
            y: caster.y - 1,
        };
        let escape = Position {
            x: caster.x + 5,
            y: caster.y,
        };
        for position in [
            player,
            Position {
                x: player.x + 1,
                y: player.y,
            },
            Position {
                x: player.x + 2,
                y: player.y,
            },
            caster,
            landing,
            escape,
        ] {
            let index = game.index(position).expect("test cell should exist");
            game.terrain[index] = "demo.terrain.floor".to_owned();
        }
        game.player.hp = 1_000;
        if levitating {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.wings".to_owned());
        }
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.ancient-roc",
            "demo.actor.the-ancient-roc-of-okeldad",
            caster,
            3_872,
            130,
            100,
            true,
        ));
        let ability = game
            .content
            .ability("rfb-legacy.ability.bird-drop")
            .expect("P54 bird drop should compile")
            .clone();
        assert!(matches!(ability.effect, AbilityEffectDefinition::BirdDrop));
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("the player should be a valid bird drop target");
        let MonsterAbilityTargetPlan::BirdDrop { destination, .. } = &plan.target else {
            panic!("BIRD_DROP should retain its dedicated target plan");
        };
        assert_eq!(*destination, landing);
        let draws = game.rng_draw_counter();
        let mut events = Vec::new();
        let resolution = game.resolve_monster_ability_plan(
            0,
            "demo.actor.the-ancient-roc-of-okeldad",
            &plan,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        (game, resolution, events, draws, player, caster, landing)
    }

    let (fly_seed, fly) = (0..1_000)
        .find_map(|seed| {
            let result = cast(seed, false);
            result.1.effects.is_empty().then_some((seed, result))
        })
        .expect("a bounded seed should take the one-in-three escape branch");
    assert_ne!(fly.0.entities[0].position, fly.5);
    let escape_distance = fly
        .5
        .x
        .abs_diff(fly.0.entities[0].position.x)
        .max(fly.5.y.abs_diff(fly.0.entities[0].position.y));
    assert!((5..=10).contains(&escape_distance));
    assert_eq!(fly.0.player.position, fly.4);
    assert_eq!(fly.0.player.hp, 1_000);
    assert_eq!(fly.0.rng_draw_counter(), fly.3 + 2);
    assert!(fly.2.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterTeleported { resolution, .. }
            if resolution.actor_id == "generated.actor.ancient-roc"
    )));

    let drop_seed = (0..1_000)
        .find(|seed| *seed != fly_seed && !cast(*seed, false).1.effects.is_empty())
        .expect("a bounded seed should take the drop branch");
    let (ordinary, ordinary_resolution, ordinary_events, ordinary_draws, player, _, landing) =
        cast(drop_seed, false);
    let (levitating, levitating_resolution, _, levitating_draws, _, _, _) = cast(drop_seed, true);
    let damage = |resolution: &MonsterAbilityPlanResolution| {
        let AbilityEffectResolutionDto::Damage { resolution, .. } = &resolution.effects[0] else {
            panic!("the drop branch should resolve physical damage");
        };
        resolution.raw_damage
    };
    let ordinary_damage = damage(&ordinary_resolution);
    let levitating_damage = damage(&levitating_resolution);
    assert!((10..=80).contains(&ordinary_damage));
    assert!((4..=32).contains(&levitating_damage));
    assert!(ordinary_damage > levitating_damage);
    assert_eq!(ordinary.rng_draw_counter(), ordinary_draws + 11);
    assert_eq!(levitating.rng_draw_counter(), levitating_draws + 5);
    assert_eq!(ordinary.player.position, landing);
    assert_eq!(levitating.player.position, landing);
    assert!(ordinary_events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterDraggedTarget { resolution, .. }
            if resolution.from == player && resolution.to == landing
    )));

    let mut flying_target = Game::new(drop_seed);
    clear_monsters(&mut flying_target);
    for cell in &mut flying_target.terrain {
        *cell = "demo.terrain.wall".to_owned();
    }
    flying_target.player.position = Position { x: 70, y: 20 };
    let caster = Position { x: 4, y: 4 };
    let target = Position { x: 7, y: 4 };
    let landing = Position { x: 4, y: 3 };
    for position in [
        caster,
        Position { x: 5, y: 4 },
        Position { x: 6, y: 4 },
        target,
        landing,
    ] {
        let index = flying_target
            .index(position)
            .expect("test cell should exist");
        flying_target.terrain[index] = "demo.terrain.floor".to_owned();
    }
    flying_target.entities.push(actor_from_runtime_spawn(
        "generated.actor.ancient-roc",
        "demo.actor.the-ancient-roc-of-okeldad",
        caster,
        3_872,
        130,
        100,
        true,
    ));
    let mut bat = actor_from_runtime_spawn(
        "generated.summon.fruit-bat",
        "demo.actor.fruit-bat",
        target,
        1_000,
        110,
        100,
        true,
    );
    bat.controller_id = Some(flying_target.player.id.clone());
    flying_target.entities.push(bat);
    let ability = flying_target
        .content
        .ability("rfb-legacy.ability.bird-drop")
        .expect("P54 bird drop should compile")
        .clone();
    let plan = flying_target
        .monster_ability_target_plan(0, ability, 1)
        .expect("the flying summon should be a valid bird drop target");
    let draws = flying_target.rng_draw_counter();
    let resolution = flying_target.resolve_monster_ability_plan(
        0,
        "demo.actor.the-ancient-roc-of-okeldad",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!((4..=32).contains(&damage(&resolution)));
    assert_eq!(flying_target.rng_draw_counter(), draws + 5);
    assert_eq!(flying_target.entities[1].position, landing);
}

#[test]
fn p76_air_breath_is_unresisted_and_levitation_reduces_damage_by_one_quarter() {
    fn cast(levitating: bool) -> (i32, i32, bool) {
        let mut game = Game::new(257);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 20, y: 20 };
        game.player.hp = 1_000;
        if levitating {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.wings".to_owned());
        }
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.vayu",
            "demo.actor.vayu-the-embodied-wind",
            Position { x: 21, y: 20 },
            1_000,
            135,
            100,
            true,
        ));
        let ability = game
            .content
            .ability("rfb-legacy.ability.breath-air-17-250-r3")
            .expect("P76 BR_AIR should compile")
            .clone();
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("adjacent player should be a valid air-breath target");
        let resolution = game.resolve_monster_ability_plan(
            0,
            "demo.actor.vayu-the-embodied-wind",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        );
        let AbilityEffectResolutionDto::Damage { resolution, .. } =
            &resolution.targets[0].effects[0]
        else {
            panic!("BR_AIR should damage the player");
        };
        (
            resolution.raw_damage,
            resolution.final_damage,
            game.player_has_status_kind(STATUS_STUN),
        )
    }

    let ordinary = cast(false);
    let levitating = cast(true);
    assert_eq!(ordinary.0, 170);
    assert_eq!(ordinary.1, 170);
    assert_eq!(levitating.0, 170);
    assert_eq!(levitating.1, 128);
    assert!(ordinary.2 && levitating.2);
}

#[test]
fn p76_chicken_deals_flat_damage_and_applies_sound_stun_and_fear() {
    let mut game = Game::new(263);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 20, y: 20 };
    game.player.hp = 1_000;
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.aijem",
        "demo.actor.aijem-the-walrus",
        Position { x: 21, y: 20 },
        1_000,
        135,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.chicken-1d1-199")
        .expect("P76 CHICKEN should compile")
        .clone();
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("adjacent player should be a valid chicken target");
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.aijem-the-walrus",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    let AbilityEffectResolutionDto::Damage { resolution, .. } = &resolution.effects[0] else {
        panic!("CHICKEN should damage the player");
    };
    assert_eq!(resolution.raw_damage, 200);
    assert_eq!(resolution.final_damage, 200);
    assert!(game.player_has_status_kind(STATUS_STUN));
    assert!(game.player_has_status_kind(STATUS_FEAR));
}
