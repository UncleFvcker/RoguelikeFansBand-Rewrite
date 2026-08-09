// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

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
fn death_abilities_materialize_player_level_scaling_in_projection() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.progress.level = 11;
    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        abilities["demo.ability.death-malediction"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Damage {
            damage_dice: 5,
            damage_sides: 4,
            damage_bonus: 0,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-stinking-cloud"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 14,
            radius: 2,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-black-sleep"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            power: Some(22),
            duration_ticks: 500,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-horrify"].effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                power: Some(22),
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 7,
                ..
            }
        ]
    ));
    assert!(matches!(
        abilities["demo.ability.death-enslave-undead"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Control { power: 22, .. }]
    ));
}

#[test]
fn death_second_book_materializes_original_mage_scaling_and_beam_profile() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.progress.level = 30;
    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        abilities["demo.ability.death-entropy-orb"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 3,
            damage_sides: 6,
            damage_bonus: 45,
            radius: 3,
            target_category: Some(category),
            ..
        }] if category == "living"
    ));
    assert!(matches!(
        abilities["demo.ability.death-nether-bolt"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 14,
            damage_sides: 8,
            beam_chance_percent: 30,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-cloud-kill"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 119,
            radius: 5,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-genocide-one"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Genocide {
            scope: AbilityGenocideScopeDto::Single,
            power: 90,
            radius: 0,
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-poison-branding"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus { granted_brands, .. }]
            if granted_brands == &[WeaponBrandDto::Poison]
    ));

    game.progress.level = 32;
    let vampiric_drain = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.death-vampiric-drain")
        .expect("vampiric drain should be projected");
    assert!(matches!(
        vampiric_drain.effects.as_slice(),
        [AbilityEffectSpecDto::DrainLife {
            damage_dice: 1,
            damage_sides: 64,
            damage_bonus: 64,
            target_category,
            ..
        }] if target_category == "living"
    ));
}

#[test]
fn death_third_book_materializes_original_scaling_and_prorated_cap() {
    let projected = |level| {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.progress.level = level;
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>()
    };

    let level_40 = projected(40);
    assert!(matches!(
        level_40["demo.ability.death-berserk"].effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_dice: 1,
                duration_sides: 25,
                granted_modifiers,
                granted_equipment_bonuses,
                granted_status_immunities,
                ..
            },
            AbilityEffectSpecDto::Heal { amount: 30 },
        ] if granted_modifiers.max_hp == 30
            && granted_modifiers.defense == -10
            && granted_equipment_bonuses.melee_damage == 11
            && granted_status_immunities == &["rfb.status.fear".to_owned()]
    ));
    assert!(matches!(
        level_40["demo.ability.death-dark-bolt"].effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 12,
            damage_sides: 8,
            beam_chance_percent: 40,
            ..
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-battle-frenzy"]
            .effects
            .as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_sides: 25,
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_sides: 25,
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 20,
                duration_sides: 40,
                ..
            },
        ]
    ));
    assert!(matches!(
        level_40["demo.ability.death-vampirism-true"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::DrainLife { repeat: 3, .. }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-nether-wave"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::VisibleDamage {
            damage_dice: 1,
            damage_sides: 120,
            target_category: Some(category),
            ..
        }] if category == "living"
    ));
    assert!(matches!(
        level_40["demo.ability.death-darkness-storm"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 222,
            radius: 4,
            ..
        }]
    ));

    for level in [50, 100] {
        let abilities = projected(level);
        let expected_nether_sides = level * 3;
        assert!(matches!(
            abilities["demo.ability.death-nether-wave"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::VisibleDamage { damage_sides, .. }]
                if *damage_sides == expected_nether_sides
        ));
        assert!(matches!(
            abilities["demo.ability.death-darkness-storm"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::AreaDamage {
                damage_bonus: 299,
                ..
            }]
        ));
    }
}

#[test]
fn berserk_and_battle_frenzy_roll_independent_durations_and_round_trip() {
    let mut left = prepare_death_caster(41, 40, "demo.ability.death-berserk");
    let mut right = left.clone();
    for game in [&mut left, &mut right] {
        game.player.hp = 1;
        let mut ability = game
            .content
            .ability("demo.ability.death-berserk")
            .expect("Berserk should exist")
            .clone();
        Game::apply_player_level_scaling(&mut ability, 40);
        game.resolve_player_ordered_sequence_effect(
            &ability,
            AbilityTargetPlan::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Berserk should resolve");
    }
    assert_eq!(left.state_hash(), right.state_hash());
    assert_eq!(left.player.hp, 31);
    assert!(left.player_status_immunities().contains(STATUS_FEAR));
    let berserk = left
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == "rfb.status.berserk")
        .expect("Berserk status should be active");
    assert!((26..=50).contains(&berserk.remaining_ticks));
    assert_eq!(berserk.granted_modifiers.max_hp, 30);
    assert_eq!(berserk.granted_equipment_bonuses.melee_damage, 11);
    left.progress.level = 1;
    left.progress.max_level = 1;
    left.learned_abilities.remove("demo.ability.death-berserk");
    let level_one_mana = Game::new_with_build(0, "demo.build.scholar")
        .expect("level-one scholar should create")
        .resources["demo.resource.mana"]
        .maximum;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana")
        .current = level_one_mana;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana")
        .maximum = level_one_mana;
    assert_eq!(
        Game::from_save(left.to_save())
            .expect("Berserk should reload")
            .state_hash(),
        left.state_hash()
    );

    let mut frenzy = prepare_death_caster(53, 40, "demo.ability.death-battle-frenzy");
    let mut expected_rng = frenzy.rng.clone();
    let expected = [
        26 + u32::try_from(expected_rng.bounded(25)).unwrap(),
        26 + u32::try_from(expected_rng.bounded(25)).unwrap(),
        21 + u32::try_from(expected_rng.bounded(40)).unwrap(),
    ];
    let mut ability = frenzy
        .content
        .ability("demo.ability.death-battle-frenzy")
        .expect("Battle Frenzy should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 40);
    frenzy
        .resolve_player_ordered_sequence_effect(
            &ability,
            AbilityTargetPlan::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Battle Frenzy should resolve");
    let durations = ["rfb.status.hero", "rfb.status.blessed", STATUS_HASTE].map(|kind_id| {
        frenzy
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == kind_id)
            .expect("Frenzy status should be active")
            .remaining_ticks
    });
    assert_eq!(durations, expected);
    assert_eq!(frenzy.rng, expected_rng);
}

#[test]
fn vampirism_true_retraces_the_path_after_each_kill() {
    let ability_id = "demo.ability.death-vampirism-true";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 36, ability_id);
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
fn invoke_spirits_records_deterministic_random_no_op_branches() {
    let ability_id = "demo.ability.death-invoke-spirits";
    let cast = |seed| {
        let mut game = prepare_death_caster(seed, 10, ability_id);
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
    let seed = (0..512)
        .find(|seed| {
            let (_, events) = cast(*seed);
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::AbilityEffectsResolved { resolution, .. }
                        if matches!(
                            resolution.effects.as_slice(),
                            [AbilityEffectResolutionDto::NoOp { .. }]
                        )
                )
            })
        })
        .expect("a deterministic Invoke Spirits no-op branch should exist");
    let (left, left_events) = cast(seed);
    let (right, right_events) = cast(seed);
    assert_eq!(left_events, right_events);
    assert_eq!(left.state_hash(), right.state_hash());
    assert!(left_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RandomChoice { roll, branch_index, .. }]
                    if *roll > 0 && matches!(*branch_index, 3 | 7)
            )
    )));
    assert!(left_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::NoOp { reason, .. }]
                    if reason.ends_with("-pending")
            )
    )));
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
        game
    };
    let make_ability = |game: &Game, id: &str, beam_chance_percent| {
        let mut ability = game
            .content
            .ability("demo.ability.death-dark-bolt")
            .expect("dark bolt should provide a bolt-or-beam definition")
            .clone();
        let AbilityEffectDefinition::BoltOrBeamDamage { damage_type, .. } = ability.effect else {
            unreachable!("dark bolt must remain a bolt-or-beam ability");
        };
        ability.id = id.to_owned();
        ability.effect = AbilityEffectDefinition::BoltOrBeamDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 3,
            damage_type,
            beam_chance_percent,
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
    assert!(
        !bolt_events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityBeamDamage { .. }))
    );
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
fn genocide_erases_without_rewards_or_corpses_and_uniques_resist() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let unique = artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian");
    unique.glyph = "y".to_owned();
    unique.tags.push("unique".to_owned());
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let mut game =
        Game::from_content_with_build(19, catalog, DEFAULT_WORLD_ID, "demo.build.scholar")
            .expect("custom scholar build should create");
    clear_monsters(&mut game);
    for (id, kind_id, x) in [
        ("test.actor.normal", "demo.actor.gloom-weaver", 4),
        ("test.actor.unique", "demo.actor.serpent-of-chaos", 5),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        let position = Position { x, y: 3 };
        replace_terrain(&mut game, position, "demo.terrain.floor");
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
    let experience_before = game.progress.experience;
    let item_count_before = game.items.len();
    let hp_before = game.player.hp;
    let mut events = Vec::new();
    let mut removed_entities = Vec::new();
    let mut ability = game
        .content
        .ability("demo.ability.death-genocide")
        .expect("genocide ability should exist")
        .clone();
    ability.id = "test.ability.genocide".to_owned();
    ability.effect = AbilityEffectDefinition::Genocide {
        scope: AbilityGenocideScopeDefinition::Glyph,
        power: 1_000,
        radius: 0,
    };
    game.resolve_player_genocide_effect(
        &ability,
        Some(vec![Position { x: 4, y: 3 }]),
        &mut events,
        &mut BTreeSet::new(),
        &mut removed_entities,
    );
    let resolution = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => resolution.effects.first(),
            _ => None,
        })
        .expect("genocide should emit a resolution");
    let AbilityEffectResolutionDto::Genocide {
        removed_entity_ids,
        resisted_entity_ids,
        fatigue_damage,
        ..
    } = resolution
    else {
        panic!("genocide should emit its dedicated effect resolution");
    };
    assert_eq!(removed_entity_ids, &["test.actor.normal".to_owned()]);
    assert_eq!(resisted_entity_ids, &["test.actor.unique".to_owned()]);
    assert!((2..=8).contains(fatigue_damage));
    assert_eq!(game.player.hp, hp_before - fatigue_damage);
    assert_eq!(game.progress.experience, experience_before);
    assert_eq!(game.items.len(), item_count_before);
    assert_eq!(removed_entities, vec!["test.actor.normal".to_owned()]);
    assert!(
        game.entities
            .iter()
            .all(|actor| actor.id != "test.actor.normal")
    );
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "test.actor.unique")
    );
    assert!(game.items.iter().all(|item| {
        item.kind_id != "demo.item.corpse-remains"
            || !matches!(item.location, ItemLocation::Ground(_))
    }));
}

#[test]
fn ordinary_death_creates_a_corpse_and_animate_dead_consumes_it_persistently() {
    let mut game = Game::new(23);
    clear_monsters(&mut game);
    let definition = game
        .content
        .actor("demo.actor.gloom-weaver")
        .expect("demo corpse source")
        .clone();
    let position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.corpse-source",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    let trace = ProjectileTrace {
        origin: game.player.position,
        impact: position,
        landing: position,
        traversed: vec![position],
    };
    game.resolve_ability_damage_to_entity(
        0,
        "test.ability.kill",
        DamageType::Physical,
        10_000,
        trace,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("lethal damage should resolve");
    assert!(game.entities.is_empty());
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.corpse-remains"
            && matches!(item.location, ItemLocation::Ground(found) if found == position)
    }));
    let corpse_item_id = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.corpse-remains"
                && matches!(item.location, ItemLocation::Ground(found) if found == position)
        })
        .expect("slain actor should leave a ground corpse")
        .id
        .clone();

    let mut events = Vec::new();
    let ability = game
        .content
        .ability("demo.ability.death-animate-dead")
        .expect("animate dead ability should exist")
        .clone();
    game.resolve_player_animate_dead_effect(&ability, &mut events, &mut BTreeSet::new())
        .expect("animate dead should resolve");
    assert!(game.items.iter().all(|item| item.id != corpse_item_id));
    assert_eq!(game.entities.len(), 1);
    assert_eq!(game.entities[0].kind_id, "demo.actor.risen-thrall");
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(game.entities[0].summon.is_none());
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::AnimateDead {
                    consumed_corpse_item_ids,
                    entity_ids,
                    ..
                }] if consumed_corpse_item_ids.len() == 1 && entity_ids.len() == 1
            )
    )));

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("risen thrall should reload");
    assert_eq!(restored.snapshot(), snapshot);
}

#[test]
fn sleep_power_resolves_then_skips_energy_and_damage_wakes_the_target() {
    let mut game = Game::new(0);
    let template = game.generated_actor(
        "test.actor.sleep-target".to_owned(),
        "demo.actor.small-kobold",
        Position { x: 4, y: 3 },
    );
    let mut saw_added = false;
    let mut saw_resisted = false;
    for seed in 0..256 {
        let mut actor = template.clone();
        actor.statuses.clear();
        let mut rng = RfbRng::seeded(seed);
        let resolution = apply_ability_status_effect(
            &mut actor,
            "test.ability.sleep",
            0,
            STATUS_SLEEP,
            1,
            50,
            0,
            0,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            Some(10),
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            Some(10),
            None,
            &mut rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            power_roll,
            target_roll,
            change,
            ..
        } = resolution
        else {
            panic!("sleep should resolve as a status");
        };
        assert!(power_roll.is_some());
        assert!(target_roll.is_some());
        saw_added |= change == AbilityStatusChangeDto::Added;
        saw_resisted |= change == AbilityStatusChangeDto::Resisted;
        if saw_added && saw_resisted {
            break;
        }
    }
    assert!(saw_added, "a deterministic sleep success seed should exist");
    assert!(
        saw_resisted,
        "a deterministic sleep resistance seed should exist"
    );

    let mut game = Game::new(0);
    let sleeping_actor = template.clone();
    clear_monsters(&mut game);
    game.entities.push(sleeping_actor);
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_SLEEP.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: Some("test.ability.sleep".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let position = game.entities[0].position;
    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("sleep should round-trip");
    assert_eq!(restored.snapshot(), snapshot);

    game.entities[0].energy_need = 0;
    let mut events = Vec::new();
    game.process_monster_energy_pulse(&mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("sleeping monster energy should resolve");
    assert_eq!(game.entities[0].position, position);
    assert_eq!(game.entities[0].energy_need, 90);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterSlept { .. }))
    );

    game.entities[0].hp -= 1;
    game.wake_entity_after_damage(0, 1, &mut events);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLEEP)
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::EntityAwakened { .. }))
    );
}

#[test]
fn temporary_status_resistances_apply_expire_and_round_trip() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let granted = BTreeMap::from([
        (
            rfb_content::ActorDamageType::Cold,
            ActorResistanceLevel::Resistant,
        ),
        (
            rfb_content::ActorDamageType::Poison,
            ActorResistanceLevel::Resistant,
        ),
    ]);
    let resolution = apply_ability_status_effect(
        &mut game.player,
        "demo.ability.death-necromantic-resistance",
        0,
        "rfb.status.necromantic-resistance",
        1,
        2,
        0,
        0,
        AbilityStatusStackingDefinition::Replace,
        None,
        None,
        &granted,
        &BTreeSet::new(),
        &StatModifiers::default(),
        &EquipmentBonuses::default(),
        &BTreeSet::new(),
        None,
        false,
        100,
        None,
        None,
        &mut game.rng,
    );
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::ApplyStatus {
            change: AbilityStatusChangeDto::Added,
            ..
        }
    ));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("temporary resistance should reload");
    assert_eq!(restored.snapshot(), snapshot);

    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new(), true)
        .expect("first status tick should resolve");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new(), true)
        .expect("second status tick should expire");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Normal
    );
}

#[test]
fn magic_affinity_and_strong_mind_gate_existing_dispel_and_resource_drain_effects() {
    let seed = (0..100)
        .find(|seed| {
            let game = Game::new_with_build(*seed, "demo.build.scholar")
                .expect("scholar build should create");
            let mut rng = game.rng.clone();
            rng.bounded(100) < 77
        })
        .expect("a deterministic affinity resistance seed should exist");
    let mut game =
        Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.resolve_item_speed("demo.item.swiftstep-tonic", 0, 1, 10, &mut Vec::new());
    assert!(game.player_has_status_kind(STATUS_HASTE));
    assert!(game.gain_mutation("rfb.mutation.one-with-magic", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.strong-mind", &mut Vec::new()));

    let dispel = game
        .content
        .ability("demo.ability.veil-dispel")
        .expect("veil dispel should exist")
        .clone();
    let draws_before = game.rng_draw_counter();
    let resolutions = game.resolve_monster_player_effects(
        "test.monster.caster",
        "demo.actor.veil-warden",
        &dispel,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert!(matches!(
        resolutions.as_slice(),
        [AbilityEffectResolutionDto::Skipped {
            reason: AbilityEffectSkipReasonDto::Saved,
            ..
        }]
    ));
    assert!(game.player_has_status_kind(STATUS_HASTE));

    let drain = game
        .content
        .ability("rfb-legacy.ability.drain-mana-2")
        .expect("drain mana should exist")
        .clone();
    let resource_id = game
        .casting_profile()
        .expect("scholar should have mana")
        .resource_id
        .clone();
    let resource_before = game.resources[&resource_id].current;
    let resolutions = game.resolve_monster_player_effects(
        "test.monster.caster",
        "demo.actor.gazer",
        &drain,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert!(matches!(
        resolutions.as_slice(),
        [AbilityEffectResolutionDto::DrainResource {
            resource_id: drained_id,
            requested: 2,
            drained: 0,
            caster_healed: 0,
            ..
        }] if drained_id == &resource_id
    ));
    assert_eq!(game.resources[&resource_id].current, resource_before);
}

#[test]
fn legacy_caster_save_restores_full_resources_without_rng_drift() {
    let canonical =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let mut legacy = canonical.to_save();
    legacy.player.resources.clear();
    legacy.player.learned_ability_ids.clear();
    legacy.player.ability_progress.clear();
    let draw_counter = legacy.rng.draw_counter;

    let migrated = Game::from_save(legacy).expect("legacy caster save should migrate");
    let snapshot = migrated.snapshot();
    assert_eq!(migrated.rng_draw_counter(), draw_counter);
    assert_eq!(snapshot.player.resources[0].current, 21);
    assert_eq!(snapshot.player.resources[0].maximum, 21);
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .all(|ability| !ability.learned)
    );
    assert_eq!(migrated.state_hash(), canonical.state_hash());

    let restored = Game::from_save(migrated.to_save())
        .expect("migrated caster state should survive another round trip");
    assert_eq!(restored.state_hash(), migrated.state_hash());
}

#[test]
fn waiting_and_resting_recover_mana_until_the_pool_is_full() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 10;
    let initial_draws = game.rng_draw_counter();

    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.resources["demo.resource.mana"].current, 11);
    assert!(waited.events.iter().any(|event| {
        event.kind == "resource.recovered"
            && matches!(
                event.outcome.as_ref(),
                Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                    if resolution.before == 10
                        && resolution.after == 11
                        && resolution.recovered == 1
            )
    }));

    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    let resolution = rest_resolution(&rested);
    assert_eq!(resolution.completed_turns, 4);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert_eq!(resolution.resource_recoveries.len(), 1);
    assert_eq!(resolution.resource_recoveries[0].before, 11);
    assert_eq!(resolution.resource_recoveries[0].after, 21);
    assert_eq!(game.resources["demo.resource.mana"].current, 21);
    assert_eq!(rested.turn, 5);
    assert_eq!(rested.world_tick, 50);
    assert_eq!(game.rng_draw_counter(), initial_draws);

    let world_tick = game.world_tick;
    let full = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    let full_resolution = rest_resolution(&full);
    assert_eq!(full_resolution.completed_turns, 0);
    assert_eq!(
        full_resolution.stop_reason,
        RestStopReasonDto::FullResources
    );
    assert!(full_resolution.resource_recoveries.is_empty());
    assert_eq!(game.world_tick, world_tick);
    assert_eq!(game.rng_draw_counter(), initial_draws);

    let restored = Game::from_save(game.to_save()).expect("recovered mana should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn natural_regeneration_and_rest_restore_warrior_health() {
    let mut game =
        Game::new_with_build(0, "demo.build.warrior").expect("warrior build should create");
    clear_monsters(&mut game);
    let maximum = game.effective_player_max_hp();
    game.player.hp = maximum - 2;

    for _ in 0..8 {
        dispatch_next(&mut game, GameCommand::Wait);
    }
    assert_eq!(game.player.hp, maximum - 2);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.player.hp, maximum - 1);

    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 9_999 });
    let resolution = rest_resolution(&rested);
    assert!(resolution.completed_turns > 0);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert_eq!(game.player.hp, maximum);
}

#[test]
fn duelist_initializes_innate_techniques_and_empty_tempo_pool() {
    let game = Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    let baseline =
        Game::new_with_build(0, "demo.build.vanguard").expect("vanguard build should create");
    assert_eq!(game.rng_draw_counter(), baseline.rng_draw_counter());

    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.resources.len(), 1);
    let tempo = &snapshot.player.resources[0];
    assert_eq!(tempo.id, "demo.resource.tempo");
    assert_eq!(tempo.current, 0);
    assert_eq!(tempo.maximum, game.resources["demo.resource.tempo"].maximum);
    assert!(tempo.maximum > 8);
    assert_eq!(tempo.wait_recovery_amount, 0);
    assert_eq!(tempo.rest_recovery_amount, 0);
    assert_eq!(tempo.melee_hit_gain_amount, 2);
    assert_eq!(tempo.melee_kill_gain_amount, 3);
    assert_eq!(tempo.turn_decay_amount, 1);

    assert!(snapshot.player.ability_learning.is_none());
    assert_eq!(snapshot.player.abilities.len(), 2);
    for ability in &snapshot.player.abilities {
        assert!(ability.innate);
        assert!(!ability.learned);
        assert!(!ability.can_study);
        assert!(!ability.can_forget);
        assert!(!ability.can_cast, "tempo starts empty");
        assert_eq!(ability.resource_id, "demo.resource.tempo");
        assert!(ability.book_item_id.is_none());
    }

    let restored = Game::from_save(game.to_save()).expect("duelist save should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn duelist_projects_the_complete_crescent_cut_protocol_dto() {
    let mut game =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    game.entities.clear();
    game.resources
        .get_mut("demo.resource.tempo")
        .expect("duelist should own tempo")
        .current = 7;
    game.ability_progress.insert(
        "demo.ability.crescent-cut".to_owned(),
        AbilityProgress {
            proficiency: 128,
            proficiency_cap: 1_600,
            cast_count: 1,
            fail_count: 0,
            cooldown_remaining: 0,
        },
    );
    dispatch_next(&mut game, GameCommand::Wait);

    let snapshot = game.snapshot();
    let ability = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crescent-cut")
        .expect("duelist should project crescent cut");
    assert_eq!(
        serde_json::to_value(ability).expect("ability DTO should serialize"),
        serde_json::json!({
            "id": "demo.ability.crescent-cut",
            "nameKey": "ability-demo-crescent-cut-name",
            "descriptionKey": "ability-demo-crescent-cut-description",
            "minimumLevel": 1,
            "innate": true,
            "resourceId": "demo.resource.tempo",
            "baseResourceCost": 4,
            "resourceCost": 7,
            "failurePercent": 2,
            "proficiency": 128,
            "proficiencyCap": 1600,
            "proficiencyRank": "unskilled",
            "castCount": 1,
            "failCount": 0,
            "cooldownRemaining": 0,
            "cooldownTurns": 0,
            "effects": [{
                "type": "damage",
                "damageDice": 3,
                "damageSides": 4,
                "damageBonus": 0,
                "damageType": "physical"
            }],
            "targetSpec": {
                "modes": ["direction"],
                "range": 1,
                "requiresLineOfEffect": true
            },
            "learned": false,
            "canStudy": false,
            "canForget": false,
            "canCast": false
        })
    );
}

#[test]
fn wait_and_rest_never_refill_tempo_and_rest_stops_immediately() {
    let mut game =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.tempo")
        .expect("tempo pool should exist")
        .current = 5;
    let draws = game.rng_draw_counter();

    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        waited
            .events
            .iter()
            .all(|event| event.kind != "resource.recovered")
    );
    assert_eq!(game.resources["demo.resource.tempo"].current, 4);

    let world_tick = game.world_tick;
    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 50 });
    let resolution = rest_resolution(&rested);
    assert_eq!(resolution.completed_turns, 0);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert!(resolution.resource_recoveries.is_empty());
    assert_eq!(game.world_tick, world_tick);
    assert_eq!(game.resources["demo.resource.tempo"].current, 4);
    assert_eq!(game.rng_draw_counter(), draws);
}

#[test]
fn saves_without_technique_pools_migrate_to_initial_fill_without_rng() {
    let mut payload = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    payload.player.resources.clear();
    payload.player.ability_progress.clear();
    let migrated = Game::from_save(payload).expect("legacy duelist save should reload");
    assert_eq!(migrated.resources["demo.resource.tempo"].current, 0);
    let baseline =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    assert_eq!(migrated.rng_draw_counter(), baseline.rng_draw_counter());
    assert_eq!(migrated.state_hash(), baseline.state_hash());

    let mut unknown = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    unknown.player.resources[0].id = "demo.resource.missing".to_owned();
    assert!(matches!(
        Game::from_save(unknown),
        Err(CoreError::InvalidSave("player resource ID is invalid"))
    ));

    let mut oversized = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    oversized.player.resources[0].maximum += 1;
    assert!(matches!(
        Game::from_save(oversized),
        Err(CoreError::InvalidSave("player resource pool is invalid"))
    ));
}

#[test]
fn blink_other_moves_the_target_within_ten_tiles_using_one_destination_draw() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    let caster = Position {
        x: player.x + 1,
        y: player.y,
    };
    let landing = Position {
        x: player.x + 5,
        y: player.y,
    };
    for position in [player, caster, landing] {
        let index = game.index(position).expect("test cell should exist");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.gnome-mage",
        "demo.actor.gnome-mage",
        caster,
        31,
        110,
        100,
        true,
    ));

    let ability = game
        .content
        .ability("rfb-legacy.ability.blink-other")
        .expect("P29 ability should compile")
        .clone();
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::BlinkTarget { radius: 10 }
    ));
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("adjacent player should be a valid blink target");
    let MonsterAbilityTargetPlan::BlinkTarget { destinations, .. } = &plan.target else {
        panic!("BLINK_OTHER should plan a target blink");
    };
    assert_eq!(destinations, &[landing]);
    assert!(destinations.iter().all(|position| {
        player
            .x
            .abs_diff(position.x)
            .max(player.y.abs_diff(position.y))
            <= 10
    }));

    let draws = game.rng_draw_counter();
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed_entities = Vec::new();
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.gnome-mage",
        &plan,
        &mut events,
        &mut changed,
        &mut removed_entities,
    );
    assert_eq!(game.rng_draw_counter(), draws + 1);
    assert_eq!(game.player.position, landing);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::MonsterBlinkedTarget { resolution, .. }
            if resolution.from == player && resolution.to == landing
    )));
}

#[test]
fn jump_light_damages_from_the_caster_then_blinks_with_one_destination_draw() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
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
        .ability("rfb-legacy.ability.jump-light-5d5")
        .expect("P39A ability should compile")
        .clone();
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::JumpDamage {
            damage_dice: 5,
            damage_sides: 5,
            damage_multiplier_numerator: 5,
            damage_multiplier_denominator: 4,
            damage_type: rfb_content::ActorDamageType::Light,
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
        panic!("JMP_LIGHT should plan a caster-centered jump burst");
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

    assert_eq!(game.rng_draw_counter(), draws + 6);
    assert_eq!(game.entities[0].position, landing);
    let AbilityEffectResolutionDto::Damage {
        resolution: damage, ..
    } = &resolution.targets[0].effects[0]
    else {
        panic!("JMP_LIGHT should damage the player");
    };
    let possible_raw = (5..=25).map(|roll| roll * 5 / 4).collect::<BTreeSet<_>>();
    assert!(possible_raw.contains(&damage.raw_damage));
    assert_eq!(damage.final_damage, rfb_area_damage(damage.raw_damage, 1));
    assert_eq!(damage.damage_type, DamageTypeDto::Light);
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::PlayerDied { .. },
            DomainEvent::MonsterBlinked { resolution, .. }
        ] if resolution.from == caster && resolution.to == landing
    ));
}

#[test]
fn death_fourth_book_materializes_original_level_curves() {
    let projected = |level| {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.progress.level = level;
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>()
    };

    let level_40 = projected(40);
    assert!(matches!(
        level_40["demo.ability.death-death-ray"].effects.as_slice(),
        [AbilityEffectSpecDto::DeathRay { power: 80 }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-raise-dead"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::SummonCategory {
            maximum_level: 60,
            upgraded_category: Some(category),
            upgrade_at_level: Some(48),
            ..
        }] if category == "high-undead"
    ));
    let [
        AbilityEffectSpecDto::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        },
    ] = level_40["demo.ability.death-esoteria"].effects.as_slice()
    else {
        panic!("Esoteria should project one identify effect");
    };
    assert_eq!((*full_identify_power, *full_identify_roll_sides), (30, 50));
    assert!(matches!(
        level_40["demo.ability.death-vampiric-transformation"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 25,
            duration_sides: 25,
            granted_race_id: Some(race_id),
            ..
        }] if race_id == "demo.race.vampire-lord"
    ));
    assert!(matches!(
        level_40["demo.ability.death-mass-genocide"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Genocide {
            scope: AbilityGenocideScopeDto::Nearby,
            power: 92,
            radius: 20,
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-hellfire"].effects.as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 373,
            radius: 5,
            ..
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-wraithform"].effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 14,
            duration_sides: 14,
            grants_wall_passage: true,
            incoming_damage_percent: 50,
            ..
        }]
    ));

    let level_50 = projected(50);
    assert!(matches!(
        level_50["demo.ability.death-hellfire"].effects.as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 604,
            radius: 10,
            ..
        }]
    ));
    assert!(matches!(
        level_50["demo.ability.death-wraithform"].effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 25,
            duration_sides: 25,
            ..
        }]
    ));
}

#[test]
fn raise_dead_is_deterministic_and_enforces_faction_group_and_unique_rules() {
    let cast = |seed: u64, level: u16| {
        let mut game = prepare_death_caster(seed, level, "demo.ability.death-raise-dead");
        game.debug_set_ability_casts_succeed(true);
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.death-raise-dead",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Raise Dead should resolve");
        let resolution = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilitySummoned { resolution, .. } => Some(resolution.clone()),
                _ => None,
            })
            .expect("Raise Dead should summon");
        (game, resolution)
    };

    let (shallow, shallow_resolution) = cast(0, 25);
    assert_eq!(shallow_resolution.actor_kind_id, "undead");
    assert!(
        shallow_resolution
            .summoned_kind_ids
            .iter()
            .all(|kind_id| matches!(
                kind_id.as_str(),
                "demo.actor.carrion"
                    | "demo.actor.crypt-creep"
                    | "demo.actor.disembodied-hand-that-strangled-people"
                    | "demo.actor.flying-skull"
                    | "demo.actor.green-glutton-ghost"
                    | "demo.actor.jibaku-ghost"
                    | "demo.actor.lost-soul"
                    | "demo.actor.moaning-spirit"
                    | "demo.actor.plaguebearer-of-nurgle"
                    | "demo.actor.poltergeist"
                    | "demo.actor.risen-thrall"
                    | "demo.actor.rotting-corpse"
                    | "demo.actor.servant-of-glaaki"
                    | "demo.actor.skeleton-human"
                    | "demo.actor.skeleton-kobold"
                    | "demo.actor.skeleton-orc"
                    | "demo.actor.the-ghost-q"
                    | "demo.actor.undead-devilfish"
                    | "demo.actor.undead-mass"
                    | "demo.actor.zombified-human"
                    | "demo.actor.zombified-kobold"
                    | "demo.actor.zombified-orc"
            ))
    );
    assert_eq!(shallow.state_hash(), cast(0, 25).0.state_hash());

    let mut saw_friendly = false;
    let mut saw_hostile = false;
    let mut saw_group = false;
    let mut saw_unique = false;
    for seed in 0..512 {
        let (game, resolution) = cast(seed, 48);
        assert_eq!(resolution.actor_kind_id, "high-undead");
        assert!(resolution.summoned_kind_ids.iter().all(|kind_id| matches!(
            kind_id.as_str(),
            "demo.actor.grave-wight" | "demo.actor.dread-vampire"
        )));
        let summoned = game
            .entities
            .iter()
            .filter(|entity| resolution.entity_ids.contains(&entity.id))
            .collect::<Vec<_>>();
        if resolution.hostile {
            saw_hostile = true;
            assert!(summoned.iter().all(|entity| entity.controller_id.is_none()));
        } else {
            saw_friendly = true;
            assert!(
                summoned
                    .iter()
                    .all(|entity| entity.controller_id.as_deref() == Some(game.player.id.as_str()))
            );
            assert!(
                resolution
                    .summoned_kind_ids
                    .iter()
                    .all(|kind_id| kind_id != "demo.actor.dread-vampire")
            );
        }
        saw_group |= resolution.group && resolution.entity_ids.len() > 1;
        if resolution
            .summoned_kind_ids
            .iter()
            .any(|kind_id| kind_id == "demo.actor.dread-vampire")
        {
            assert!(resolution.hostile);
            saw_unique = true;
        }
        if saw_friendly && saw_hostile && saw_group && saw_unique {
            break;
        }
    }
    assert!(saw_friendly && saw_hostile && saw_group && saw_unique);
}

#[test]
fn vampiric_transformation_overlays_race_but_preserves_body_slots() {
    let mut game = prepare_death_caster(17, 35, "demo.ability.death-vampiric-transformation");
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    let body_slots = game.body_slots.clone();
    let base = game.snapshot().player;
    let mut ability = game
        .content
        .ability("demo.ability.death-vampiric-transformation")
        .expect("Vampiric Transformation should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 35);
    game.resolve_player_actor_status_effect(
        &ability,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );

    let transformed = game.snapshot().player;
    assert_eq!(game.body_slots, body_slots);
    assert_eq!(
        transformed
            .build
            .as_ref()
            .map(|build| build.race_id.as_str()),
        Some("demo.race.vampire-lord")
    );
    assert!(
        transformed.progress.attributes.strength.effective
            > base.progress.attributes.strength.effective
    );
    assert!(
        transformed
            .resistances
            .iter()
            .any(|entry| entry.damage_type == DamageTypeDto::Dark
                && entry.level == ResistanceLevelDto::Immune)
    );
    let transformed_melee = transformed
        .progress
        .skills
        .iter()
        .find(|skill| skill.id == "demo.skill.melee")
        .expect("transformed melee skill should be projected");
    let base_melee = base
        .progress
        .skills
        .iter()
        .find(|skill| skill.id == "demo.skill.melee")
        .expect("base melee skill should be projected");
    assert!(transformed_melee.base > base_melee.base);
    let restored = Game::from_save(game.to_save()).expect("temporary race should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.body_slots, body_slots);
}
