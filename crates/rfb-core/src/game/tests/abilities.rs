// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn anti_magic_status_blocks_learned_spells_without_spending_resources() {
    let mut game = prepare_death_caster(7, 40, "demo.ability.death-berserk");
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_ANTI_MAGIC, 5, "test.anti-magic").status);
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();

    game.resolve_player_ability(
        "demo.ability.death-berserk",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("anti-magic rejection should resolve cleanly");

    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "anti-magic"
    ));
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.death-berserk")
            .expect("learned spell should remain projected")
            .can_cast
    );
}

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
    let mut game = test_caster_game(0);
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
fn corrected_death_spells_project_authoritative_values_at_levels_one_twenty_and_fifty() {
    for level in [1, 20, 50] {
        let mut game = test_caster_game(0);
        game.progress.level = level;
        let abilities = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>();

        for ability_id in [
            "demo.ability.death-detect-unlife",
            "demo.ability.death-detect-evil",
        ] {
            assert!(matches!(
                abilities[ability_id].effects.as_slice(),
                [AbilityEffectSpecDto::Detect { radius: 30, .. }]
            ));
        }
        assert!(matches!(
            abilities["demo.ability.death-necromantic-resistance"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 20,
                duration_dice: 1,
                duration_sides: 20,
                ..
            }]
        ));
        assert!(matches!(
            abilities["demo.ability.death-vampiric-drain"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::DrainLife {
                damage_sides,
                damage_bonus,
                feeds: true,
                ..
            }] if *damage_sides == level * 2 && *damage_bonus == level * 2
        ));
    }
}

#[test]
fn death_vampiric_drain_heals_and_feeds_up_to_the_original_caps() {
    let mut game = prepare_death_caster(0, 50, "demo.ability.death-vampiric-drain");
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
}

#[test]
fn death_second_book_materializes_original_mage_scaling_and_beam_profile() {
    let mut game = test_caster_game(0);
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
            ..
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
        let mut game = test_caster_game(0);
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
    let level_one_mana = test_caster_game(0).resources["demo.resource.mana"].maximum;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should keep Mana")
        .current = level_one_mana;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("test caster should keep Mana")
        .maximum = level_one_mana;
    assert_eq!(
        Game::from_save_with_content(left.to_save(), left.content.clone())
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
fn invoke_spirits_scales_every_source_formula_without_nested_random_effects() {
    fn contains_no_op(effect: &AbilityEffectDefinition) -> bool {
        match effect {
            AbilityEffectDefinition::NoOp { .. } => true,
            AbilityEffectDefinition::Sequence { effects } => effects.iter().any(contains_no_op),
            AbilityEffectDefinition::RandomChoice { branches, .. } => {
                branches.iter().any(|branch| contains_no_op(&branch.effect))
            }
            _ => false,
        }
    }

    let game = test_caster_game(0);
    let source = game
        .content
        .ability("demo.ability.death-invoke-spirits")
        .expect("Invoke Spirits should exist");
    assert!(!contains_no_op(&source.effect));

    for (level, expected_dice, expected_bonuses) in [
        (1, [3, 3, 5, 6, 8], [19, 29, 40, 70, 80, 100]),
        (20, [6, 6, 8, 9, 11], [29, 39, 59, 89, 99, 119]),
        (50, [12, 14, 16, 17, 19], [44, 54, 89, 119, 129, 149]),
    ] {
        let mut ability = source.clone();
        Game::apply_player_level_scaling(&mut ability, level);
        let AbilityEffectDefinition::RandomChoice { branches, .. } = &ability.effect else {
            unreachable!("Invoke Spirits should remain a random choice");
        };
        assert_eq!(branches.len(), 23);
        assert_eq!(
            branches
                .iter()
                .map(|branch| branch.maximum_roll)
                .collect::<Vec<_>>(),
            vec![
                7, 13, 25, 30, 35, 40, 45, 50, 55, 60, 65, 70, 75, 80, 85, 90, 95, 100, 103, 105,
                107, 109, 120,
            ]
        );
        let damage_dice = |index: usize| match branches[index].effect.as_ref() {
            AbilityEffectDefinition::BoltOrBeamDamage { damage_dice, .. } => *damage_dice,
            _ => unreachable!("branch {index} should be bolt-or-beam damage"),
        };
        assert_eq!(
            [
                damage_dice(4),
                damage_dice(8),
                damage_dice(9),
                damage_dice(10),
                damage_dice(11),
            ],
            expected_dice
        );
        let damage_bonus = |index: usize| match branches[index].effect.as_ref() {
            AbilityEffectDefinition::AreaDamage { damage_bonus, .. }
            | AbilityEffectDefinition::DrainLife { damage_bonus, .. } => *damage_bonus,
            _ => unreachable!("branch {index} should have a flat damage bonus"),
        };
        assert_eq!(
            [
                damage_bonus(6),
                damage_bonus(13),
                damage_bonus(14),
                damage_bonus(15),
                damage_bonus(16),
                damage_bonus(17),
            ],
            expected_bonuses
        );
        assert!(matches!(
            branches[5].effect.as_ref(),
            AbilityEffectDefinition::ApplyStatus {
                power: Some(power),
                ..
            } if *power == level
        ));
        assert!(matches!(
            branches[12].effect.as_ref(),
            AbilityEffectDefinition::DrainLife {
                damage_bonus: 74,
                ..
            }
        ));
        assert!(matches!(
            branches[18].effect.as_ref(),
            AbilityEffectDefinition::Earthquake { radius: 12, .. }
        ));
        assert!(matches!(
            branches[19].effect.as_ref(),
            AbilityEffectDefinition::AreaDestruction {
                minimum_radius: 13,
                maximum_radius: 17,
                ..
            }
        ));
        assert!(matches!(
            branches[20].effect.as_ref(),
            AbilityEffectDefinition::Genocide { power, .. } if *power == level + 50
        ));
        let AbilityEffectDefinition::Sequence { effects } = branches[22].effect.as_ref() else {
            unreachable!("the highest Invoke Spirits branch should be a self sequence");
        };
        assert!(matches!(
            effects.as_slice(),
            [
                AbilityEffectDefinition::VisibleDamage {
                    damage_dice: 1,
                    damage_sides: 1,
                    damage_bonus: 149,
                    ..
                },
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id: slow,
                    duration_ticks: 50,
                    power: Some(slow_power),
                    ..
                },
                AbilityEffectDefinition::VisibleApplyStatus {
                    status_kind_id: sleep,
                    duration_ticks: 500,
                    power: Some(sleep_power),
                    ..
                },
                AbilityEffectDefinition::Heal { amount: 300 },
            ] if slow == "rfb.status.slow"
                && sleep == "rfb.status.sleep"
                && *slow_power == level
                && *sleep_power == level
        ));
    }

    let mut game = test_caster_game(0);
    game.progress.level = 50;
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.death-invoke-spirits")
        .expect("Invoke Spirits should be projected");
    let [AbilityEffectSpecDto::RandomChoice { branches, .. }] = projected.effects.as_slice() else {
        unreachable!("Invoke Spirits projection should remain a random choice");
    };
    assert!(
        matches!(
            branches[22].effect.as_ref(),
            AbilityEffectSpecDto::Sequence { effects }
                if matches!(
                    effects.as_slice(),
                    [
                        AbilityEffectSpecDto::VisibleDamage { damage_bonus: 149, .. },
                        AbilityEffectSpecDto::VisibleApplyStatus { power: Some(50), .. },
                        AbilityEffectSpecDto::VisibleApplyStatus { power: Some(50), .. },
                        AbilityEffectSpecDto::Heal { amount: 300 },
                    ]
                )
        ),
        "projected highest branch: {:?}",
        branches[22].effect
    );
}

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
    enable_test_caster(&mut artifact.content);
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let mut game =
        Game::from_content_with_build(19, catalog, DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("custom test caster should create");
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
        target_category: None,
        fatigue: true,
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
fn monster_animate_dead_consumes_failed_remains_and_spawns_hostile_summons() {
    let mut game = Game::new(31);
    clear_monsters(&mut game);
    game.items.clear();
    let source_position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let corpse_position = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y,
    };
    let skeleton_position = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y + 1,
    };
    for position in [source_position, corpse_position, skeleton_position] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    let necromancer = game.generated_actor(
        "test.actor.necromancer".to_owned(),
        "demo.actor.small-kobold",
        source_position,
    );
    game.entities.push(necromancer);
    give_inventory_item(&mut game, "test.item.corpse", "demo.item.corpse-remains");
    give_inventory_item(
        &mut game,
        "test.item.skeleton",
        "demo.item.skeleton-remains",
    );
    game.items[0].location = ItemLocation::Ground(corpse_position);
    game.items[1].location = ItemLocation::Ground(skeleton_position);

    let mut ability = game
        .content
        .ability("demo.ability.death-animate-dead")
        .expect("animate dead ability should exist")
        .clone();
    ability.id = "test.ability.monster-animate-dead".to_owned();
    ability.effect = AbilityEffectDefinition::Sequence {
        effects: vec![
            AbilityEffectDefinition::AnimateDead {
                actor_kind_id: "demo.actor.risen-thrall".to_owned(),
                corpse_item_kind_id: "demo.item.corpse-remains".to_owned(),
                radius: 5,
                count: 8,
                failure_chance_percent: 100,
            },
            AbilityEffectDefinition::AnimateDead {
                actor_kind_id: "demo.actor.risen-thrall".to_owned(),
                corpse_item_kind_id: "demo.item.skeleton-remains".to_owned(),
                radius: 5,
                count: 8,
                failure_chance_percent: 0,
            },
        ],
    };

    let mut changed = BTreeSet::new();
    let (resolutions, affected_positions) =
        game.resolve_monster_self_effects(0, &ability, &mut changed);

    assert_eq!(resolutions.len(), 2);
    assert!(game.items.is_empty());
    assert_eq!(affected_positions, [corpse_position, skeleton_position]);
    assert!(changed.contains(&corpse_position));
    assert!(changed.contains(&skeleton_position));
    let summoned = game
        .entities
        .iter()
        .filter(|entity| entity.kind_id == "demo.actor.risen-thrall")
        .collect::<Vec<_>>();
    assert_eq!(summoned.len(), 1);
    assert_eq!(summoned[0].position, skeleton_position);
    assert!(summoned.iter().all(|entity| {
        entity.controller_id.is_none()
            && entity.summon.as_ref().is_some_and(|summon| {
                summon.owner_id == "test.actor.necromancer"
                    && summon.source_ability_id == ability.id
                    && summon.remaining_turns == 0
            })
    }));
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
            let game = test_caster_game(*seed);
            let mut rng = game.rng.clone();
            rng.bounded(100) < 77
        })
        .expect("a deterministic affinity resistance seed should exist");
    let mut game = test_caster_game(seed);
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
        .expect("test caster should have mana")
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
fn waiting_and_resting_recover_mana_until_the_pool_is_full() {
    let mut game = test_caster_game(0);
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("test caster mana pool should exist")
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

    let maximum = game.resources["demo.resource.mana"].maximum;
    let rest_recovery = game
        .content
        .resource("demo.resource.mana")
        .expect("Mana definition should remain available")
        .rest_recovery_amount;
    let expected_rest_turns = u16::try_from(maximum.saturating_sub(11).div_ceil(rest_recovery))
        .expect("test rest duration should fit u16");
    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    let resolution = rest_resolution(&rested);
    assert_eq!(resolution.completed_turns, expected_rest_turns);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert_eq!(resolution.resource_recoveries.len(), 1);
    assert_eq!(resolution.resource_recoveries[0].before, 11);
    assert_eq!(resolution.resource_recoveries[0].after, maximum);
    assert_eq!(game.resources["demo.resource.mana"].current, maximum);
    assert_eq!(rested.turn, 1 + u32::from(expected_rest_turns));
    assert_eq!(rested.world_tick, 10 * (1 + u32::from(expected_rest_turns)));
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

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("recovered mana should reload");
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

const MUTATION_CONTRACT_ABILITY_ID: &str = "demo.ability.mutation-contract";
const MUTATION_CONTRACT_ID: &str = "rfb.mutation.spit-acid";

fn mutation_ability_catalog(
    minimum_level: u16,
    cost: u32,
    base_failure_percent: u8,
) -> Arc<rfb_content::ContentCatalog> {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let mut ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.warrens-scare")
        .expect("monster scare ability should exist")
        .clone();
    ability.id = MUTATION_CONTRACT_ABILITY_ID.to_owned();
    ability.name_key = "ability-mutation-contract-name".to_owned();
    ability.description_key = "ability-mutation-contract-description".to_owned();
    ability.target = AbilityTargetDefinition {
        modes: vec![AbilityTargetModeDefinition::SelfTarget],
        range: 0,
        requires_line_of_effect: false,
    };
    ability.effect = AbilityEffectDefinition::NoOp {
        reason: "mutation-contract".to_owned(),
    };
    ability.level_scaling.clear();
    ability.player = None;
    artifact.content.abilities.push(ability);
    artifact
        .content
        .mutations
        .iter_mut()
        .find(|mutation| mutation.id == MUTATION_CONTRACT_ID)
        .expect("Spit Acid mutation should exist")
        .activation = Some(MutationActivationDefinition {
        minimum_level,
        governing_attribute: TechniqueAttribute::Constitution,
        cost,
        cost_scaling: None,
        base_failure_percent,
        minimum_failure_percent: None,
        ability_id: MUTATION_CONTRACT_ABILITY_ID.to_owned(),
    });
    enable_test_caster(&mut artifact.content);
    Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("mutation ability contract content should remain valid"),
    ))
}

fn mutation_ability_game(catalog: Arc<rfb_content::ContentCatalog>, build_id: &str) -> Game {
    let mut game = Game::from_content_with_build(0, catalog, DEFAULT_WORLD_ID, build_id)
        .expect("mutation ability test build should create");
    clear_monsters(&mut game);
    game.progress
        .active_mutation_ids
        .insert(MUTATION_CONTRACT_ID.to_owned());
    game
}

fn mutation_cast_resolution(events: &[DomainEvent]) -> &AbilityCastResolutionDto {
    events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityCastSucceeded { resolution }
            | DomainEvent::AbilityCastFailed { resolution } => Some(resolution),
            _ => None,
        })
        .expect("mutation cast should produce a resolution")
}

#[test]
fn active_mutation_projects_without_learning_progress_or_persistent_cooldown() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mut game =
        Game::from_content_with_build(0, catalog.clone(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("warrior build should create");
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut events = Vec::new();
    assert!(game.gain_mutation(MUTATION_CONTRACT_ID, &mut events));
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == MUTATION_CONTRACT_ABILITY_ID)
        .expect("gaining the mutation should project its ability");
    assert_eq!(ability.source, AbilitySourceDto::Mutation);
    assert_eq!(ability.resource_id, None);
    assert_eq!(ability.base_resource_cost, 7);
    assert_eq!(ability.resource_cost, 7);
    assert_eq!(ability.proficiency, 0);
    assert_eq!(ability.proficiency_cap, 0);
    assert_eq!(ability.cast_count, 0);
    assert_eq!(ability.fail_count, 0);
    assert_eq!(ability.cooldown_remaining, 0);
    assert_eq!(ability.cooldown_turns, 0);
    assert!(!ability.learned);
    assert!(!ability.can_study);
    assert!(!ability.can_forget);
    assert!(ability.can_cast);
    assert!(
        !game
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut restored = Game::from_save_with_content(game.to_save(), catalog)
        .expect("active mutation ability should restore from existing mutation state");
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == MUTATION_CONTRACT_ABILITY_ID)
    );
    events.clear();
    assert!(restored.lose_mutation(MUTATION_CONTRACT_ID, &mut events));
    assert!(
        restored
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != MUTATION_CONTRACT_ABILITY_ID)
    );
}

#[test]
fn mutation_cast_spills_sp_into_hp_and_keeps_rejections_atomic() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mut mana = mutation_ability_game(catalog.clone(), "test.build.caster");
    mana.debug_set_ability_casts_succeed(true);
    let resource_id = mana
        .casting_profile()
        .expect("test caster should have a casting profile")
        .resource_id
        .clone();
    mana.resources
        .get_mut(&resource_id)
        .expect("test caster should have an SP pool")
        .current = 10;
    let hp_before = mana.player.hp;
    let mut events = Vec::new();
    mana.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("mutation ability should resolve");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(
        resolution.resource_id.as_deref(),
        Some(resource_id.as_str())
    );
    assert_eq!(resolution.resource_before, 10);
    assert_eq!(resolution.resource_after, 3);
    assert_eq!(resolution.resource_paid, 7);
    assert_eq!(resolution.hp_paid, 0);
    assert_eq!(mana.player.hp, hp_before);
    assert!(
        !mana
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );

    let mut spill = mutation_ability_game(catalog.clone(), "test.build.caster");
    spill.debug_set_ability_casts_succeed(true);
    spill
        .resources
        .get_mut(&resource_id)
        .expect("test caster should have an SP pool")
        .current = 3;
    let hp_before = spill.player.hp;
    events.clear();
    spill
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("mutation ability should spill into HP");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.resource_paid, 3);
    assert_eq!(resolution.hp_paid, 4);
    assert_eq!(spill.player.hp, hp_before - 4);

    let mut hp_only = mutation_ability_game(catalog.clone(), "demo.build.warrior");
    hp_only.debug_set_ability_casts_succeed(true);
    let hp_before = hp_only.player.hp;
    events.clear();
    hp_only
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("a build without SP should pay HP only");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.resource_id, None);
    assert_eq!(resolution.resource_paid, 0);
    assert_eq!(resolution.hp_paid, 7);
    assert_eq!(hp_only.player.hp, hp_before - 7);

    let mut rejected = mutation_ability_game(catalog, "demo.build.warrior");
    rejected.player.hp = 6;
    let draws_before = rejected.rng_draw_counter();
    events.clear();
    rejected
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("insufficient mutation budget should reject cleanly");
    assert_eq!(rejected.player.hp, 6);
    assert_eq!(rejected.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "insufficient-resource"
    ));

    rejected.player.hp = 20;
    events.clear();
    rejected
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::North,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("invalid mutation target should reject cleanly");
    assert_eq!(rejected.player.hp, 20);
    assert_eq!(rejected.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { .. }]
    ));
}

#[test]
fn mutation_level_and_failure_paths_do_not_create_ability_progress() {
    let low_catalog = mutation_ability_catalog(2, 1, 30);
    let mut low = mutation_ability_game(low_catalog, "demo.build.warrior");
    let draws_before = low.rng_draw_counter();
    let hp_before = low.player.hp;
    let mut events = Vec::new();
    low.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("low-level mutation ability should reject cleanly");
    assert_eq!(low.player.hp, hp_before);
    assert_eq!(low.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "level-too-low"
    ));

    let fail_catalog = mutation_ability_catalog(1, 1, 95);
    let mut failed = mutation_ability_game(fail_catalog, "demo.build.warrior");
    failed.player.hp = 20;
    events.clear();
    failed
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("mutation failure should resolve");
    assert!(matches!(
        events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.player.hp, 19);
    assert!(
        !failed
            .ability_progress
            .contains_key(MUTATION_CONTRACT_ABILITY_ID)
    );
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.failure_percent, 92);
    assert_eq!(resolution.hp_paid, 1);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 0);
    assert_eq!(resolution.cast_count, 0);
    assert_eq!(resolution.fail_count, 0);
}

#[test]
fn active_mutation_batches_project_scaled_costs_and_effects() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.progress.level = 25;
    let suffixes = [
        "spit-acid",
        "br-fire",
        "hypn-gaze",
        "telekinesis",
        "teleport",
        "mind-blast",
        "radiation",
        "vampirism",
        "smell-metal",
        "smell-monsters",
        "blink",
        "swap-pos",
        "shriek",
        "illumine",
        "det-curse",
        "berserk",
        "resist",
        "dazzle",
        "laser-eye",
        "recall",
        "banish",
        "cold-touch",
        "eat-rock",
        "polymorph",
        "midas-touch",
        "grow-mold",
        "earthquake",
        "eat-magic",
        "weigh-magic",
        "sterility",
        "panic-hit",
    ];
    for suffix in suffixes {
        assert!(game.gain_mutation(&format!("rfb.mutation.{suffix}"), &mut Vec::new()));
    }
    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .filter(|ability| ability.source == AbilitySourceDto::Mutation)
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(abilities.len(), 31);

    let acid = &abilities["rfb.ability.mutation.spit-acid"];
    assert_eq!((acid.base_resource_cost, acid.resource_cost), (9, 14));
    assert!(matches!(
        acid.effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrAreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 49,
            area_from_level: 25,
            radius: 2,
            ..
        }]
    ));
    assert_eq!(abilities["rfb.ability.mutation.br-fire"].resource_cost, 13);
    assert_eq!(abilities["rfb.ability.mutation.vampirism"].resource_cost, 9);
    assert_eq!(abilities["rfb.ability.mutation.resist"].resource_cost, 15);
    assert!(matches!(
        abilities["rfb.ability.mutation.teleport"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::BlinkSelf { radius: 110 }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.banish"].effects.as_slice(),
        [AbilityEffectSpecDto::Genocide {
            power: 75,
            target_category: Some(category),
            fatigue: false,
            ..
        }] if category == "evil"
    ));
    assert_eq!(abilities["rfb.ability.mutation.illumine"].effects.len(), 2);
    assert_eq!(abilities["rfb.ability.mutation.dazzle"].effects.len(), 3);
    assert!(matches!(
        abilities["rfb.ability.mutation.eat-rock"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ConsumeTerrain { nutrition: 3000 }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.midas-touch"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::TransmuteItemToGold {
            value_divisor: 3,
            unit_value_cap: 30_000,
        }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.grow-mold"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::SummonCategory {
            maximum_level: 25,
            count_dice: 8,
            count_sides: 1,
            ..
        }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.earthquake"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Earthquake {
            radius: 10,
            affect_chance_percent: 15,
            ..
        }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.sterility"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::SuppressMonsterReproduction {
            damage_dice: 1,
            damage_sides: 17,
            damage_bonus: 17,
        }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.panic-hit"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::MeleeThenTeleport {
            radius: 30,
            failure_threshold: 7,
        }]
    ));
    assert!(matches!(
        abilities["rfb.ability.mutation.polymorph"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::PolymorphSelf]
    ));
}

fn active_source_mutation_game(seed: u64, suffix: &str, level: u16) -> Game {
    let mut game = test_caster_game(seed);
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.refresh_character_skills();
    game.debug_set_ability_casts_succeed(true);
    assert!(game.gain_mutation(&format!("rfb.mutation.{suffix}"), &mut Vec::new()));
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana");
    mana.current = mana.maximum;
    game
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
fn mutation_eat_magic_and_weigh_magic_use_existing_device_and_status_state() {
    let capped_failure = active_source_mutation_game(51, "eat-magic", 100);
    let activation = capped_failure
        .content
        .mutation("rfb.mutation.eat-magic")
        .unwrap()
        .activation
        .clone()
        .unwrap();
    assert_eq!(capped_failure.mutation_failure_percent(&activation), 11);

    let mut eater = active_source_mutation_game(53, "eat-magic", 17);
    give_inventory_item(
        &mut eater,
        "test.item.magic-food",
        "demo.item.detect-objects-staff",
    );
    let item = eater
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.magic-food")
        .unwrap();
    item.activation
        .as_mut()
        .expect("staff should have an activation")
        .device_check_difficulty = 100;
    item.charges
        .as_mut()
        .expect("staff should have charges")
        .current = 20;
    eater
        .resources
        .get_mut("demo.resource.mana")
        .unwrap()
        .current = 10;
    let mut events = Vec::new();
    eater
        .resolve_player_ability(
            "rfb.ability.mutation.eat-magic",
            TargetSelection::Item {
                item_id: "test.item.magic-food".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Eat Magic should resolve");
    assert_eq!(eater.resources["demo.resource.mana"].current, 29);
    assert_eq!(
        eater
            .items
            .iter()
            .find(|item| item.id == "test.item.magic-food")
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::DrainItemMagic {
                    drained: 20,
                    failed: false,
                    resource_before: 9,
                    resource_after: 29,
                    ..
                }]
            )
    )));

    let mut observer = active_source_mutation_game(59, "weigh-magic", 6);
    observer.player.statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: Some("test.status".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    observer.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.warrens".to_owned(),
        floor_id: "demo.floor.warrens.1".to_owned(),
        remaining_turns: Some(9),
    });
    events.clear();
    observer
        .resolve_player_ability(
            "rfb.ability.mutation.weigh-magic",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Weigh Magic should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::ReportMagic { statuses, recall, .. }]
                    if statuses.iter().any(|status| status.kind_id == STATUS_HASTE)
                        && recall.as_ref().is_some_and(|state| state.remaining_turns == Some(9))
            )
    )));
}

#[test]
fn mutation_grow_mold_and_sterility_persist_only_authoritative_state() {
    let mut grower = active_source_mutation_game(61, "grow-mold", 8);
    for terrain in &mut grower.terrain {
        *terrain = "demo.terrain.floor".to_owned();
    }
    grower
        .resolve_player_ability(
            "rfb.ability.mutation.grow-mold",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Grow Mold should resolve");
    assert_eq!(grower.entities.len(), 8);
    assert!(grower.entities.iter().all(|entity| {
        entity.controller_id.as_deref() == Some(grower.player.id.as_str())
            && grower
                .content
                .actor(&entity.kind_id)
                .is_some_and(|actor| actor.tags.iter().any(|tag| tag == "mold"))
    }));

    let mut sterile = active_source_mutation_game(67, "sterility", 12);
    sterile.player.hp = 100;
    sterile
        .resolve_player_ability(
            "rfb.ability.mutation.sterility",
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Sterility should resolve");
    assert!(sterile.reproduction_suppressed);
    sterile.player.hp = sterile.player.hp.min(sterile.effective_player_max_hp());
    let restored = Game::from_save_with_content(sterile.to_save(), sterile.content.clone())
        .expect("sterility state should reload");
    assert!(restored.reproduction_suppressed);
    assert_eq!(restored.state_hash(), sterile.state_hash());
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
fn mutation_telekinesis_and_swap_position_reuse_directional_targeting() {
    let mut fetch = active_source_mutation_game(17, "telekinesis", 9);
    fetch.items.clear();
    let origin = fetch.player.position;
    for step in 0..=3 {
        replace_terrain(
            &mut fetch,
            Position {
                x: origin.x + step,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    give_inventory_item(
        &mut fetch,
        "test.item.fetch",
        "demo.item.detect-objects-staff",
    );
    fetch
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.fetch")
        .expect("fetched item")
        .location = ItemLocation::Ground(Position {
        x: origin.x + 3,
        y: origin.y,
    });
    let mut events = Vec::new();
    fetch
        .resolve_player_ability(
            "rfb.ability.mutation.telekinesis",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("telekinesis should resolve");
    assert!(
        matches!(
            fetch
                .items
                .iter()
                .find(|item| item.id == "test.item.fetch")
                .map(|item| &item.location),
            Some(ItemLocation::Ground(position)) if *position == origin
        ),
        "telekinesis events: {events:?}"
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::FetchItem { moved: true, .. }])
    )));

    let mut swap = active_source_mutation_game(19, "swap-pos", 15);
    let origin = swap.player.position;
    let target = Position {
        x: origin.x + 2,
        y: origin.y,
    };
    for step in 0..=2 {
        replace_terrain(
            &mut swap,
            Position {
                x: origin.x + step,
                y: origin.y,
            },
            "demo.terrain.floor",
        );
    }
    swap.entities.push(actor_from_runtime_spawn(
        "test.actor.swap",
        "demo.actor.gnome-mage",
        target,
        20,
        50,
        100,
        true,
    ));
    swap.resolve_player_ability(
        "rfb.ability.mutation.swap-pos",
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("swap position should resolve");
    assert_eq!(swap.player.position, target);
    assert_eq!(swap.entities[0].position, origin);
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
fn level_based_jump_damage_uses_no_damage_rng_then_blinks() {
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
fn monster_polymorph_reuses_mutation_and_actor_form_transactions() {
    let ability = Game::new(0)
        .content
        .ability("rfb-legacy.ability.polymorph-target")
        .expect("polymorph target ability should compile")
        .clone();
    assert!(matches!(
        ability.effect,
        AbilityEffectDefinition::PolymorphTarget
    ));

    let template = Game::new(0);
    let (player_game, player_resolution) = (0..1_000)
        .find_map(|seed| {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            let resolutions = game.resolve_monster_player_effects(
                "test.monster.dokkaebi",
                "demo.actor.dokkaebi",
                &ability,
                &mut Vec::new(),
                &mut BTreeSet::new(),
            );
            matches!(
                resolutions.as_slice(),
                [AbilityEffectResolutionDto::PolymorphTarget { changed: true, .. }]
            )
            .then_some((game, resolutions.into_iter().next().unwrap()))
        })
        .expect("a bounded seed should fail the save and change mutations");
    assert!(!player_game.progress.active_mutation_ids.is_empty());
    assert!(matches!(
        player_resolution,
        AbilityEffectResolutionDto::PolymorphTarget {
            form_kind_id: None,
            changed: true,
            ..
        }
    ));

    let mut summon_game = Game::new(0);
    clear_monsters(&mut summon_game);
    summon_game.player.position = Position { x: 80, y: 20 };
    let caster_position = Position { x: 4, y: 3 };
    let summon_position = Position { x: 5, y: 3 };
    summon_game.entities.push(actor_from_runtime_spawn(
        "test.monster.dokkaebi",
        "demo.actor.dokkaebi",
        caster_position,
        374,
        115,
        100,
        true,
    ));
    let mut summon = actor_from_runtime_spawn(
        "test.summon.kobold",
        "demo.actor.small-kobold",
        summon_position,
        12,
        100,
        100,
        true,
    );
    summon.controller_id = Some(summon_game.player.id.clone());
    summon_game.entities.push(summon);
    let plan = summon_game
        .monster_ability_target_plan(0, ability, 1)
        .expect("adjacent player summon should be a valid polymorph target");
    let mut changed = BTreeSet::new();
    let resolution = summon_game.resolve_monster_ability_plan(
        0,
        "demo.actor.dokkaebi",
        &plan,
        &mut Vec::new(),
        &mut changed,
        &mut Vec::new(),
    );
    let transformed = &summon_game.entities[1];
    assert_ne!(transformed.kind_id, "demo.actor.small-kobold");
    assert_eq!(transformed.appearance_kind_id, None);
    assert_eq!(
        transformed.controller_id.as_deref(),
        Some(summon_game.player.id.as_str())
    );
    assert!(changed.contains(&summon_position));
    assert!(matches!(
        resolution.effects.as_slice(),
        [AbilityEffectResolutionDto::PolymorphTarget {
            form_kind_id: Some(form_kind_id),
            changed: true,
            ..
        }] if form_kind_id == &transformed.kind_id
    ));

    let mut protected = Game::new(0);
    clear_monsters(&mut protected);
    protected.player.position = Position { x: 80, y: 20 };
    protected.entities.push(actor_from_runtime_spawn(
        "test.monster.dokkaebi",
        "demo.actor.dokkaebi",
        caster_position,
        374,
        115,
        100,
        true,
    ));
    let mut unique2 = actor_from_runtime_spawn(
        "test.unique2.silver-angel",
        "demo.actor.silver-angel",
        summon_position,
        300,
        130,
        100,
        true,
    );
    unique2.controller_id = Some(protected.player.id.clone());
    protected.entities.push(unique2);
    let polymorph = protected
        .content
        .ability("rfb-legacy.ability.polymorph-target")
        .expect("polymorph target ability should compile")
        .clone();
    let plan = protected
        .monster_ability_target_plan(0, polymorph, 1)
        .expect("UNIQUE2 target planning should remain valid");
    let resolution = protected.resolve_monster_ability_plan(
        0,
        "demo.actor.dokkaebi",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(protected.entities[1].kind_id, "demo.actor.silver-angel");
    assert!(matches!(
        resolution.effects.as_slice(),
        [AbilityEffectResolutionDto::Skipped {
            reason: AbilityEffectSkipReasonDto::Ineligible,
            ..
        }]
    ));
}

#[test]
fn death_fourth_book_materializes_original_level_curves() {
    let projected = |level| {
        let mut game = test_caster_game(0);
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
            ..
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

    let shallow_setup = prepare_death_caster(0, 25, "demo.ability.death-raise-dead");
    let shallow_friendly_candidates = shallow_setup.summon_category_candidate_kind_ids(
        "undead",
        Some("high-undead"),
        25 * 3 / 2,
        false,
    );
    let shallow_hostile_candidates = shallow_setup.summon_category_candidate_kind_ids(
        "undead",
        Some("high-undead"),
        25 * 3 / 2,
        true,
    );
    let (shallow, shallow_resolution) = cast(0, 25);
    assert_eq!(shallow_resolution.actor_kind_id, "undead");
    let shallow_candidates = if shallow_resolution.hostile {
        shallow_hostile_candidates
    } else {
        shallow_friendly_candidates
    };
    assert!(
        shallow_resolution
            .summoned_kind_ids
            .iter()
            .all(|kind_id| shallow_candidates.contains(kind_id)),
        "summoned {:?}, candidates {:?}",
        shallow_resolution.summoned_kind_ids,
        shallow_candidates
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
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("temporary race should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.body_slots, body_slots);
}
