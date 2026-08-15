// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::monster_ecology as ecology;

fn set_test_virtue(game: &mut Game, slot: usize, kind: VirtueKindDto, value: i16) {
    game.virtues[slot] = VirtueDto { kind, value };
}

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
fn berserk_status_blocks_learned_spells_without_spending_resources() {
    let mut game = prepare_death_caster(7, 40, "demo.ability.death-berserk");
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_BERSERK, 5, "test.berserk").status);
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
    .expect("berserk rejection should resolve cleanly");

    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "berserk"
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
fn spell_power_uses_shared_formula_and_modifier_sources_in_projection() {
    assert_eq!(spell_power_value(100, -20), 0);
    assert_eq!(spell_power_value(0, 7), 0);
    assert_eq!(spell_power_value(10, 1), 10);
    assert_eq!(spell_power_value(100, 7), 153);

    let mut game = prepare_death_caster(7, 20, "demo.ability.death-stinking-cloud");
    let equipped = game
        .items
        .iter_mut()
        .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        .expect("test caster should start with equipment");
    equipped.rolled_affixes.push(RolledAffixState {
        affix_id: "test.affix.spell-power".to_owned(),
        properties: AffixPropertyBundleDefinition {
            modifiers: StatModifiers {
                spell_power_bonus: 3,
                ..StatModifiers::default()
            },
            ..AffixPropertyBundleDefinition::default()
        },
    });
    game.player.statuses.push(StatusInstance {
        kind_id: "test.status.blood-rite".to_owned(),
        intensity: 1,
        remaining_ticks: 10,
        source_id: Some("test.ability.blood-rite".to_owned()),
        granted_modifiers: StatModifiersDto {
            spell_power_bonus: 7,
            ..StatModifiersDto::default()
        },
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();
    assert!(matches!(
        abilities["demo.ability.death-stinking-cloud"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 19,
            final_damage_spell_power_bonus: Some(10),
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-necromantic-resistance"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 35,
            duration_sides: 35,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-vampiric-drain"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::DrainLife {
            damage_sides: 70,
            damage_bonus: 70,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-invoke-spirits"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::RandomChoice {
            roll_spell_power_bonus: Some(10),
            ..
        }]
    ));

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.spell-power-target",
        "demo.actor.cinder-adept",
        Position { x: 4, y: 3 },
        100_000,
        100,
        100,
        true,
    ));
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.death-stinking-cloud",
        TargetSelection::Entity {
            entity_id: "test.actor.spell-power-target".to_owned(),
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("spell-powered stinking cloud should resolve");
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityAreaDamage { resolution, .. }
            if resolution.base_raw_damage == 35
    )));
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
        [AbilityEffectSpecDto::BrandWeapon {
            affix_id,
            brand: Some(WeaponBrandDto::Poison),
            resistance: Some(DamageTypeDto::Poison),
        }] if affix_id == "rfb-legacy.affix.slaying"
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
fn death_weapon_branding_targets_plain_weapons_across_player_locations() {
    for (ability_id, expected_affix_id, location) in [
        (
            "demo.ability.death-poison-branding",
            "rfb-legacy.affix.slaying",
            ItemLocation::Inventory,
        ),
        (
            "demo.ability.death-vampiric-branding",
            "rfb-legacy.affix.death",
            ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        ),
        (
            "demo.ability.death-vampiric-branding",
            "rfb-legacy.affix.death",
            ItemLocation::Ground(Position { x: 5, y: 5 }),
        ),
    ] {
        let level = if ability_id.ends_with("poison-branding") {
            30
        } else {
            40
        };
        let mut game = prepare_death_caster(7, level, ability_id);
        set_test_virtue(&mut game, 0, VirtueKindDto::Enchantment, 0);
        game.debug_set_ability_casts_succeed(true);
        game.player.position = Position { x: 5, y: 5 };
        game.items.retain(|item| {
            game.content
                .item(&item.kind_id)
                .is_some_and(|definition| definition.ability_book_id.is_some())
        });
        give_inventory_item(&mut game, "test.brand-target", "demo.item.dagger");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.brand-target")
            .expect("branding target")
            .location = location;
        let mut events = Vec::new();

        game.resolve_player_ability(
            ability_id,
            TargetSelection::Item {
                item_id: "test.brand-target".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("legal branding should resolve");

        let item = game
            .items
            .iter()
            .find(|item| item.id == "test.brand-target")
            .expect("branded target");
        assert_eq!(item.affix_ids, [expected_affix_id], "{events:#?}");
        assert_eq!(item.origin_kind, Some(ItemOriginKindDto::PlayerMade));
        assert_eq!(item.discount_percent, 99);
        assert_eq!(item.quality, ItemQualityDto::Fine);
        assert_eq!(game.virtue_current(VirtueKindDto::Enchantment), 2);
        assert!((0..=6).contains(&item.enchantments.to_hit));
        assert!((0..=6).contains(&item.enchantments.to_damage));
        if ability_id.ends_with("poison-branding") {
            assert_eq!(item.rolled_affixes.len(), 1);
            assert!(
                item.rolled_affixes[0]
                    .properties
                    .brands
                    .contains(&WeaponBrand::Poison)
            );
            assert_eq!(
                item.rolled_affixes[0]
                    .properties
                    .resistances
                    .get(&ActorDamageType::Poison),
                Some(&ActorResistanceLevel::Resistant)
            );
        } else {
            assert!(item.rolled_affixes.is_empty());
        }
        let expected_attempts = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    match resolution.effects.as_slice() {
                        [
                            AbilityEffectResolutionDto::BrandWeapon {
                                to_hit, to_damage, ..
                            },
                        ] => {
                            assert_eq!(to_hit.attempts, to_damage.attempts);
                            Some(to_hit.attempts)
                        }
                        _ => None,
                    }
                }
                _ => None,
            })
            .expect("branding resolution should be emitted");
        assert!((4..=6).contains(&expected_attempts));
        let knowledge = &game.item_property_knowledge["test.brand-target"];
        assert!(knowledge.discovered);
        assert!(knowledge.appraised);
        assert!(knowledge.identified);
        assert!(knowledge.known_affix_ids.contains(expected_affix_id));
    }

    let mut saved = Game::new(12);
    give_inventory_item(&mut saved, "test.saved-brand", "demo.item.dagger");
    let ability = saved
        .content
        .ability("demo.ability.death-vampiric-branding")
        .expect("vampiric branding content")
        .clone();
    saved.resolve_player_brand_weapon_effect(&ability, "test.saved-brand", &mut Vec::new());
    Game::from_save(saved.to_save()).expect("branded weapon should round-trip");
}

#[test]
fn death_weapon_branding_rejects_nonplain_or_unavailable_weapons_without_rng() {
    let mut game = prepare_death_caster(9, 40, "demo.ability.death-vampiric-branding");
    game.debug_set_ability_casts_succeed(true);
    game.items.retain(|item| {
        game.content
            .item(&item.kind_id)
            .is_some_and(|definition| definition.ability_book_id.is_some())
    });
    give_inventory_item(&mut game, "test.brand-target", "demo.item.dagger");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.brand-target")
        .expect("branding target")
        .affix_ids
        .push("rfb-legacy.affix.slaying".to_owned());
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();

    game.resolve_player_ability(
        "demo.ability.death-vampiric-branding",
        TargetSelection::Item {
            item_id: "test.brand-target".to_owned(),
        },
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("invalid branding target should be rejected cleanly");

    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(
        matches!(
            events.as_slice(),
            [DomainEvent::AbilityTargetUnavailable { .. }]
        ),
        "{events:#?}"
    );
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
                7,
                13,
                25,
                30,
                35,
                40,
                45,
                50,
                55,
                60,
                65,
                70,
                75,
                80,
                85,
                90,
                95,
                100,
                103,
                105,
                107,
                109,
                u16::MAX,
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
        unlife_change_on_success: 0,
        chance_change_on_success: 0,
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
    game.process_monster_energy_pulse(false, &mut events, &mut BTreeSet::new(), &mut Vec::new())
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

    for _ in 0..10 {
        dispatch_next(&mut game, GameCommand::Wait);
        if game.player.hp > maximum - 2 {
            break;
        }
    }
    assert_eq!(game.player.hp, maximum - 1);

    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 9_999 });
    let resolution = rest_resolution(&rested);
    assert!(resolution.completed_turns > 0);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert_eq!(game.player.hp, maximum);
}

const MUTATION_CONTRACT_ABILITY_ID: &str = "demo.ability.mutation-contract";
const MUTATION_CONTRACT_ID: &str = "rfb.mutation.spit-acid";
const RACE_BERSERK_ABILITY_ID: &str = "rfb.ability.race.berserk";
const RACE_CREATE_FOOD_ABILITY_ID: &str = "rfb.ability.race.create-food";
const RACE_DETECT_DOORS_ABILITY_ID: &str = "rfb.ability.race.detect-doors-stairs-traps";
const RACE_DETECT_TREASURE_ABILITY_ID: &str = "rfb.ability.race.detect-treasure";
const RACE_MAGIC_MISSILE_ABILITY_ID: &str = "rfb.ability.race.magic-missile";
const RACE_MIND_BLAST_ABILITY_ID: &str = "rfb.ability.race.mind-blast";
const RACE_IMP_FIRE_ABILITY_ID: &str = "rfb.ability.race.imp-fire";
const RACE_GOLEM_STONE_SKIN_ABILITY_ID: &str = "rfb.ability.race.golem-stone-skin";
const RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID: &str = "rfb.ability.race.restore-life";
const RACE_PHASE_DOOR_ABILITY_ID: &str = "rfb.ability.race.phase-door";
const RACE_POISON_DART_ABILITY_ID: &str = "rfb.ability.race.poison-dart";
const RACE_PROBE_MONSTERS_ABILITY_ID: &str = "rfb.ability.race.probe-monsters";
const RACE_SCARE_MONSTER_ABILITY_ID: &str = "rfb.ability.race.scare-monster";
const RACE_SPIT_ACID_ABILITY_ID: &str = "rfb.ability.race.spit-acid";
const RACE_STONE_TO_MUD_ABILITY_ID: &str = "rfb.ability.race.stone-to-mud";
const RACE_THROW_BOULDER_ABILITY_ID: &str = "rfb.ability.race.throw-boulder";
const RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID: &str =
    "rfb.ability.race.wood-elf-nature-awareness";
const RACE_SPRITE_SLEEPING_DUST_ABILITY_ID: &str = "rfb.ability.race.sleeping-dust";
const RACE_SNOTLING_DEVOUR_FLESH_ABILITY_ID: &str = "rfb.ability.race.devour-flesh";
const RACE_BOIT_VOMIT_ABILITY_ID: &str = "rfb.ability.race.vomit";

#[test]
fn formal_snotling_devours_flesh_while_confused_and_round_trips() {
    let mut game = snotling_game(395);
    clear_monsters(&mut game);
    game.debug_set_ability_casts_succeed(true);
    game.nutrition = 500;
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_BLEEDING, 25, "test.snotling-bleeding").status);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 20, "test.snotling-confusion").status);
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SNOTLING_DEVOUR_FLESH_ABILITY_ID)
        .expect("Snotling Devour Flesh should be projected");
    assert_eq!(projected.source, AbilitySourceDto::Race);
    assert_eq!(projected.minimum_level, 1);
    assert_eq!(projected.base_resource_cost, 0);
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Charisma),
    );
    assert!(projected.can_cast);
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::DevourFlesh {
            maximum_hp_divisor: 3,
            bleeding_amount: 100,
        }]
    ));

    let hp_before = game.player.hp;
    let maximum_hp = game.effective_player_max_hp();
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_SNOTLING_DEVOUR_FLESH_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Devour Flesh should resolve");
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityEffectsResolved { resolution, .. }
                if matches!(
                    resolution.effects.as_slice(),
                    [
                        AbilityEffectResolutionDto::SatisfyHunger {
                            nutrition_before: 500,
                            nutrition_after: 14_999,
                            ..
                        },
                        AbilityEffectResolutionDto::ApplyStatus {
                            status_kind_id,
                            applied_duration_ticks: 100,
                            ..
                        },
                        AbilityEffectResolutionDto::SelfDamage { damage, fatal: false, .. },
                    ] if status_kind_id == STATUS_BLEEDING && *damage == maximum_hp / 3
                )
        )));
    }
    assert_eq!(game.nutrition, rfb_protocol::PLAYER_NUTRITION_MAXIMUM - 1);
    assert_eq!(game.player.hp, hp_before - maximum_hp / 3);
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLEEDING)
            .expect("Devour Flesh should retain bleeding")
            .remaining_ticks,
        125,
    );
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Devour Flesh result should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_boit_vomits_poison_while_afraid_or_confused_and_pays_empty_stomach_energy() {
    let mut game = boit_game(401);
    clear_monsters(&mut game);
    game.debug_set_ability_casts_succeed(true);
    game.nutrition = 600;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor(
        "test.boit-vomit-target".to_owned(),
        "demo.actor.sheep",
        target,
    );
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_POISON, 35, "test.boit-poison").status);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_FEAR, 20, "test.boit-fear").status);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 20, "test.boit-confusion").status);
    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_BOIT_VOMIT_ABILITY_ID)
        .expect("Boit Vomit should be projected");
    assert_eq!(projected.source, AbilitySourceDto::Race);
    assert_eq!(projected.minimum_level, 1);
    assert_eq!(projected.base_resource_cost, 0);
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength),
    );
    assert!(projected.can_cast);
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::Vomit]
    ));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
    }));

    let hp_before = game.player.hp;
    let target_hp_before = game.entities[0].hp;
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_BOIT_VOMIT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Vomit should resolve");
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityAreaDamage { resolution, .. }
                if resolution.center == cast.player.position
                    && resolution.radius == 1
                    && resolution.base_raw_damage == 10
                    && resolution.damage_type == DamageTypeDto::Poison
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityEffectsResolved { resolution, .. }
                if matches!(
                    resolution.effects.as_slice(),
                    [AbilityEffectResolutionDto::Vomit {
                        nutrition_before: 600,
                        nutrition_after: 500,
                        poison_before: 35,
                        poison_damage: 10,
                        poison_removed: true,
                        empty_stomach: false,
                        self_damage: 0,
                        fatal: false,
                        extra_energy_cost: 0,
                        ..
                    }]
                )
        )));
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastUnavailable { .. }))
        );
    }
    assert_eq!(game.nutrition, 500);
    assert_eq!(game.player.hp, hp_before);
    assert!(game.entities[0].hp < target_hp_before);
    assert!(!game.player_has_status_kind(STATUS_POISON));
    assert!(game.player_has_status_kind(STATUS_FEAR));
    assert!(game.player_has_status_kind(STATUS_CONFUSION));
    assert_eq!(game.state_hash(), replay.state_hash());

    let mut empty = boit_game(402);
    clear_monsters(&mut empty);
    empty.debug_set_ability_casts_succeed(true);
    empty.nutrition = 500;
    let hp_before = empty.player.hp;
    let mut events = Vec::new();
    empty
        .resolve_player_ability(
            RACE_BOIT_VOMIT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("empty-stomach Vomit should resolve");
    assert_eq!(empty.nutrition, 400);
    assert_eq!(empty.player.hp, hp_before - 10);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Vomit {
                    empty_stomach: true,
                    self_damage: 10,
                    extra_energy_cost: 15,
                    ..
                }]
            )
    )));
    let restored = Game::from_save_with_content(empty.to_save(), empty.content.clone())
        .expect("Vomit result should restore");
    assert_eq!(restored.state_hash(), empty.state_hash());

    let mut action = boit_game(403);
    clear_monsters(&mut action);
    action.debug_set_ability_casts_succeed(true);
    action.nutrition = 500;
    let gain = energy_gain(derived_speed(&action.player_derived_stats().speed));
    let expected_ticks = u32::try_from((STANDARD_ACTION_COST + 15 + gain - 1) / gain)
        .expect("Vomit action ticks should fit u32");
    let tick_before = action.world_tick;
    dispatch_next(
        &mut action,
        GameCommand::CastAbility {
            ability_id: RACE_BOIT_VOMIT_ABILITY_ID.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(action.world_tick - tick_before, expected_ticks);
}

#[test]
fn snotling_mushroom_boost_follows_the_effective_race() {
    let use_mushroom = |game: &mut Game, id: &str| {
        give_inventory_item(game, id, "demo.item.cure-poison-mushroom");
        dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: id.to_owned(),
                target: None,
            },
        );
    };
    let boosted = |game: &Game| {
        [
            STATUS_HASTE,
            "rfb.status.stone-skin",
            "rfb.status.hero",
            STATUS_GIANT_STRENGTH,
        ]
        .into_iter()
        .all(|kind_id| game.player_has_status_kind(kind_id))
    };

    let mut formal = snotling_game(396);
    clear_monsters(&mut formal);
    formal.progress.level = 20;
    formal.progress.max_level = 20;
    let mut replay = formal.clone();
    use_mushroom(&mut formal, "test.item.snotling-mushroom");
    use_mushroom(&mut replay, "test.item.snotling-mushroom");
    assert_eq!(replay.state_hash(), formal.state_hash());
    assert!(boosted(&formal));
    let durations = formal
        .player
        .statuses
        .iter()
        .filter(|status| {
            [
                STATUS_HASTE,
                "rfb.status.stone-skin",
                "rfb.status.hero",
                STATUS_GIANT_STRENGTH,
            ]
            .contains(&status.kind_id.as_str())
        })
        .map(|status| status.remaining_ticks)
        .collect::<BTreeSet<_>>();
    assert_eq!(durations.len(), 1);
    assert!((211..=401).contains(durations.first().expect("shared duration")));

    let mut persisted = snotling_game(399);
    clear_monsters(&mut persisted);
    give_inventory_item(
        &mut persisted,
        "test.item.persisted-snotling-mushroom",
        "demo.item.cure-poison-mushroom",
    );
    persisted
        .use_inventory_item(
            "test.item.persisted-snotling-mushroom",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Snotling mushroom should resolve before the action tick");
    let restored = Game::from_save_with_content(persisted.to_save(), persisted.content.clone())
        .expect("Snotling mushroom boost should restore");
    assert_eq!(restored.state_hash(), persisted.state_hash());

    let mut temporary =
        Game::new_with_build(397, "demo.build.warrior").expect("Human Warrior should create");
    clear_monsters(&mut temporary);
    temporary.progress.level = 9;
    temporary.progress.max_level = 9;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.snotling-form").status;
    form.granted_race_id = Some("rfb-legacy.race.snotling".to_owned());
    temporary.player.statuses.push(form);
    assert_eq!(
        temporary
            .character_definitions()
            .expect("temporary Snotling should retain character definitions")
            .1
            .id,
        "rfb-legacy.race.snotling",
    );
    use_mushroom(&mut temporary, "test.item.temporary-snotling-mushroom");
    assert!(boosted(&temporary));

    let mut human =
        Game::new_with_build(398, "demo.build.warrior").expect("Human Warrior should create");
    clear_monsters(&mut human);
    use_mushroom(&mut human, "test.item.human-mushroom");
    assert!(!boosted(&human));
}

#[test]
fn formal_sprite_sleeping_dust_switches_from_adjacent_to_visible_at_twenty_five() {
    let projected = |game: &Game| {
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == RACE_SPRITE_SLEEPING_DUST_ABILITY_ID)
            .expect("Sprite Sleeping Dust should be projected")
    };
    let target_ids = |events: &[DomainEvent]| {
        events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.target_entity_id.clone()
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>()
    };
    let add_targets = |game: &mut Game| {
        clear_monsters(game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        let origin = game.player.position;
        for (id, kind_id, position) in [
            (
                "test.sprite.unseen-adjacent",
                "demo.actor.clear-icky-thing",
                Position {
                    x: origin.x + 1,
                    y: origin.y,
                },
            ),
            (
                "test.sprite.visible-distant",
                "demo.actor.small-kobold",
                Position {
                    x: origin.x + 2,
                    y: origin.y,
                },
            ),
        ] {
            game.push_generated_actor(id.to_owned(), kind_id, position);
        }
        assert!(!game.entity_is_visible_to_player(&game.entities[0]));
        assert!(game.entity_is_visible_to_player(&game.entities[1]));
    };

    let mut level_eleven = sprite_game(391);
    level_eleven.progress.level = 11;
    let locked = projected(&level_eleven);
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(locked.minimum_level, 12);
    assert_eq!(locked.base_resource_cost, 12);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence),
    );
    assert!(!locked.can_cast);

    let mut nearby = sprite_game(391);
    nearby.progress.level = 24;
    nearby.progress.max_level = 24;
    nearby.debug_set_ability_casts_succeed(true);
    add_targets(&mut nearby);
    assert!(matches!(
        projected(&nearby).effects.as_slice(),
        [AbilityEffectSpecDto::Sanctuary {
            power: 24,
            radius: 1,
        }]
    ));
    let nearby_hp = nearby.player.hp;
    let mut nearby_events = Vec::new();
    nearby
        .resolve_player_ability(
            RACE_SPRITE_SLEEPING_DUST_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut nearby_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("nearby Sleeping Dust should resolve");
    assert_eq!(nearby.player.hp, nearby_hp - 12);
    assert_eq!(
        target_ids(&nearby_events),
        BTreeSet::from(["test.sprite.unseen-adjacent".to_owned()]),
    );

    let mut visible = sprite_game(392);
    visible.progress.level = 25;
    visible.progress.max_level = 25;
    visible.refresh_character_skills();
    visible.debug_set_ability_casts_succeed(true);
    add_targets(&mut visible);
    assert!(matches!(
        projected(&visible).effects.as_slice(),
        [AbilityEffectSpecDto::VisibleApplyStatus {
            status_kind_id,
            power: Some(25),
            ..
        }] if status_kind_id == STATUS_SLEEP
    ));
    let mut replay = visible.clone();
    for cast in [&mut visible, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_SPRITE_SLEEPING_DUST_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("visible Sleeping Dust should resolve");
        assert_eq!(
            target_ids(&events),
            BTreeSet::from(["test.sprite.visible-distant".to_owned()]),
        );
    }
    assert_eq!(visible.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(visible.to_save(), visible.content.clone())
        .expect("Sleeping Dust result should restore");
    assert_eq!(restored.state_hash(), visible.state_hash());
}

#[test]
fn formal_wood_elf_nature_awareness_unlocks_at_twenty_and_reuses_full_detection() {
    let mut game = wood_elf_game(385);
    clear_monsters(&mut game);
    game.progress.level = 19;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("Wood-Elf Nature Awareness should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (15, 15));
    assert!(!locked.can_cast);

    game.progress.level = 20;
    game.progress.max_level = 20;
    game.refresh_character_skills();
    game.player.hp = game.effective_player_max_hp();
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID)
        .expect("level-twenty Wood-Elf Nature Awareness");
    assert!(available.can_cast);
    assert!(available.failure_percent >= 50);
    assert_eq!(available.effects.len(), 6);
    assert!(
        available
            .effects
            .iter()
            .all(|effect| matches!(effect, AbilityEffectSpecDto::Detect { radius: 30, .. }))
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let trap = Position { x: 4, y: 3 };
    let door = Position { x: 5, y: 3 };
    let stairs_down = Position { x: 6, y: 3 };
    let stairs_up = Position { x: 7, y: 3 };
    let monster = Position { x: 3, y: 4 };
    for (position, terrain_id) in [
        (game.player.position, "demo.terrain.floor"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (monster, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.push_generated_actor(
        "test.wood-elf-detection".to_owned(),
        "demo.actor.sheep",
        monster,
    );

    let hp_before = game.player.hp;
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        let mut events = Vec::new();
        cast.resolve_player_ability(
            RACE_WOOD_ELF_NATURE_AWARENESS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Wood-Elf Nature Awareness should resolve");

        let detections = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityDetected { resolution, .. } => Some(resolution),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(detections.len(), 6);
        for category in ["map", "trap", "door", "stairs-down", "stairs-up"] {
            assert!(
                detections
                    .iter()
                    .any(|detection| detection.category == category)
            );
        }
        assert!(detections.iter().any(|detection| {
            detection.category == "normal-monster"
                && detection
                    .detected_entity_ids
                    .iter()
                    .any(|id| id == "test.wood-elf-detection")
        }));
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
    }
    assert_eq!(game.player.hp, hp_before - 15);
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Wood-Elf detection knowledge should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_golem_stone_skin_unlocks_at_twenty_without_spell_power_scaling() {
    let mut game = golem_game(365);
    clear_monsters(&mut game);
    game.progress.level = 19;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_GOLEM_STONE_SKIN_ABILITY_ID)
        .expect("Golem Stone Skin should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Constitution)
    );
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (20, 20));
    assert!(!locked.can_cast);

    game.progress.level = 20;
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_GOLEM_STONE_SKIN_ABILITY_ID)
        .expect("level-twenty Golem Stone Skin");
    assert!(available.can_cast);
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 20,
            duration_dice: 1,
            duration_sides: 30,
            granted_modifiers,
            ..
        }] if granted_modifiers.defense == 26
    ));

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Golem Stone Skin should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_hp = failed.player.hp;
    let failed_armor = failed.player_derived_stats().armor_class.value;
    let mut failed_events = Vec::new();
    failed
        .resolve_player_ability(
            RACE_GOLEM_STONE_SKIN_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut failed_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed Golem Stone Skin should resolve");
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.player.hp, failed_hp - 20);
    assert_eq!(
        failed.player_derived_stats().armor_class.value,
        failed_armor
    );
    assert!(!failed.player_has_status_kind("rfb.status.stone-skin"));

    game.debug_set_ability_casts_succeed(true);
    let hp_before = game.player.hp;
    let armor_before = game.player_derived_stats().armor_class.value;
    game.resolve_player_ability(
        RACE_GOLEM_STONE_SKIN_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Golem Stone Skin should resolve");
    assert_eq!(game.player.hp, hp_before - 20);
    let stone_skin = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == "rfb.status.stone-skin")
        .expect("Golem Stone Skin status");
    assert!((21..=50).contains(&stone_skin.remaining_ticks));
    assert_eq!(stone_skin.granted_modifiers.defense, 26);
    assert_eq!(
        game.player_derived_stats().armor_class.value,
        armor_before + 26
    );

    game.progress.level = 50;
    let level_fifty = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_GOLEM_STONE_SKIN_ABILITY_ID)
        .expect("level-fifty Golem Stone Skin");
    assert!(matches!(
        level_fifty.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 20,
            duration_sides: 30,
            granted_modifiers,
            ..
        }] if granted_modifiers.defense == 50
    ));
}

#[test]
fn formal_zombie_restore_life_unlocks_at_thirty_and_restores_experience_and_life_force() {
    let mut game = zombie_game(374);
    clear_monsters(&mut game);
    game.progress.level = 29;
    game.player.hp = game.effective_player_max_hp();
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID)
        .expect("Zombie Restore Life should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(locked.minimum_level, 30);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (30, 30));
    assert!(!locked.can_cast);

    game.progress.level = 30;
    game.player.hp = game.effective_player_max_hp();
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID)
        .expect("level-thirty Zombie Restore Life");
    assert!(available.can_cast);
    assert!(available.failure_percent >= 70);
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::RestoreVitality { life_force: 150 }]
    ));

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Zombie Restore Life should have a failing percentile seed");
    let mut failed = game.clone();
    failed.progress.experience = 500;
    failed.progress.maximum_experience = 900;
    failed.progress.life_force = 125;
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_hp = failed.player.hp;
    let mut failed_events = Vec::new();
    failed
        .resolve_player_ability(
            RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut failed_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed Zombie Restore Life should resolve");
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.player.hp, failed_hp - 30);
    assert_eq!(failed.progress.experience, 500);
    assert_eq!(failed.progress.life_force, 125);

    game.progress.experience = 500;
    game.progress.maximum_experience = 900;
    game.progress.life_force = 125;
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Zombie Restore Life should resolve");
    let resolution = mutation_cast_resolution(&events);
    assert!(resolution.succeeded);
    assert_eq!(resolution.hp_paid, 30);
    assert_eq!(game.progress.experience, 900);
    assert_eq!(game.progress.maximum_experience, 900);
    assert_eq!(game.progress.life_force, 275);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality {
                    experience_before: 500,
                    experience_after: 900,
                    life_force_before: 125,
                    life_force_after: 275,
                    ..
                }]
            )
    )));
}

#[test]
fn formal_skeleton_restore_life_unlocks_at_thirty_and_restores_vitality() {
    let mut game = skeleton_game(382);
    clear_monsters(&mut game);
    game.progress.level = 29;
    game.player.hp = game.effective_player_max_hp();
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID)
        .expect("Skeleton Restore Life should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert!(!locked.can_cast);

    game.progress.level = 30;
    game.progress.experience = 500;
    game.progress.maximum_experience = 900;
    game.progress.life_force = 125;
    game.player.hp = game.effective_player_max_hp();
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Skeleton Restore Life should resolve");
    assert_eq!(game.progress.experience, 900);
    assert_eq!(game.progress.life_force, 275);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality {
                    experience_before: 500,
                    experience_after: 900,
                    life_force_before: 125,
                    life_force_after: 275,
                    ..
                }]
            )
    )));
}

#[test]
fn race_ability_follows_the_effective_race_and_projects_its_source() {
    let mut game = Game::new_with_build_race_and_name(
        0,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior should create");
    game.progress.level = 7;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_BERSERK_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.race-form").status;
    form.granted_race_id = Some("rfb-legacy.race.barbarian".to_owned());
    game.player.statuses.push(form);

    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
        .expect("the temporary Barbarian form should project its ability");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(locked.minimum_level, 8);
    assert!(!locked.can_cast);

    game.progress.level = 8;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
            .expect("race ability should remain projected")
            .can_cast
    );

    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_CONFUSION, 10, "test.confusion").status);
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
            .expect("race ability should remain projected while confused")
            .can_cast
    );
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_CONFUSION);

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_BERSERK_ABILITY_ID)
    );
}

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
fn formal_kobold_poison_dart_failure_spills_sp_into_hp_without_projecting() {
    let mut game = Game::new_with_build_race_and_name(
        92,
        "demo.build.high-mage-death",
        "rfb-legacy.race.kobold",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Kobold High-Mage should create");
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(12),
        &mut Vec::new(),
    );
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let failure_percent = game
        .snapshot()
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_POISON_DART_ABILITY_ID)
        .expect("Kobold Poison Dart should be projected")
        .failure_percent;
    let seed = (0..4_096)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Kobold Poison Dart should have a reachable failure roll");
    game.rng = RfbRng::seeded(seed);
    let hp_before = game.player.hp;
    let tick_before = game.world_tick;
    let serial_before = game.next_item_instance_serial;

    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: RACE_POISON_DART_ABILITY_ID.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );

    assert_eq!(game.world_tick, tick_before + 10);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp_before - 5);
    assert_eq!(game.next_item_instance_serial, serial_before);
}

#[test]
fn kobold_intrinsics_follow_the_effective_race() {
    let mut game = Game::new_with_build_race_and_name(
        93,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 12;
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_POISON_DART_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.kobold-form").status;
    form.granted_race_id = Some("rfb-legacy.race.kobold".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_POISON_DART_ABILITY_ID)
        .expect("temporary Kobold form should grant Poison Dart");
    assert_eq!(ability.source, AbilitySourceDto::Race);
    assert_eq!(
        ability.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Dexterity)
    );

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_POISON_DART_ABILITY_ID)
    );
}

#[test]
fn formal_dwarf_detection_powers_reveal_original_terrain_categories_only() {
    let mut game = Game::new_with_build_race_and_name(
        94,
        "demo.build.high-mage-death",
        "rfb-legacy.race.dwarf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dwarf High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Resistant
    );

    let level_four_experience = crate::stats::experience_required_for_level(4);
    game.apply_unscaled_player_experience(level_four_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    let doors = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
        .expect("Dwarf door detection should be projected before it unlocks");
    assert_eq!(doors.source, AbilitySourceDto::Race);
    assert_eq!(
        doors.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(doors.minimum_level, 5);
    assert_eq!(doors.base_resource_cost, 5);
    assert!(!doors.can_cast);
    let treasure = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
        .expect("Dwarf treasure detection should be projected before it unlocks");
    assert_eq!(treasure.source, AbilitySourceDto::Race);
    assert_eq!(
        treasure.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Charisma)
    );
    assert_eq!(treasure.minimum_level, 10);
    assert_eq!(treasure.base_resource_cost, 5);
    assert!(!treasure.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5) - level_four_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_DOORS_ABILITY_ID)
            .expect("Dwarf door detection should remain projected")
            .can_cast
    );
    assert!(
        !snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(9)
            - crate::stats::experience_required_for_level(5),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        !game
            .snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should remain projected")
            .can_cast
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10)
            - crate::stats::experience_required_for_level(9),
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should retain mana");
    mana.current = mana.maximum;
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
            .expect("Dwarf treasure detection should unlock at level ten")
            .can_cast
    );

    game.debug_set_ability_casts_succeed(true);
    game.player.position = Position { x: 3, y: 3 };
    let player = game.player.position;
    let blocker = Position { x: 4, y: 3 };
    let trap = Position { x: 5, y: 3 };
    let door = Position { x: 6, y: 3 };
    let stairs_down = Position { x: 7, y: 3 };
    let stairs_up = Position { x: 8, y: 3 };
    let magma = Position { x: 5, y: 4 };
    let quartz = Position { x: 6, y: 4 };
    let gold_blocker = Position { x: 4, y: 2 };
    let gold = Position { x: 5, y: 2 };
    for (position, terrain_id) in [
        (player, "demo.terrain.floor"),
        (blocker, "demo.terrain.wall"),
        (trap, "demo.terrain.created-trap"),
        (door, "demo.terrain.door-secret"),
        (stairs_down, "demo.terrain.stairs-down"),
        (stairs_up, "demo.terrain.stairs-up"),
        (magma, "demo.terrain.magma-hidden-treasure"),
        (quartz, "demo.terrain.quartz-hidden-treasure"),
        (gold_blocker, "demo.terrain.wall"),
        (gold, "demo.terrain.floor"),
    ] {
        replace_terrain(&mut game, position, terrain_id);
    }
    for position in [trap, door, stairs_down, stairs_up, magma, quartz] {
        let index = game.index(position).expect("detection target should exist");
        game.explored[index] = false;
        game.revealed_terrain.remove(&position);
    }
    game.gold_piles = vec![GoldPile {
        id: "generated.gold.1".to_owned(),
        position: gold,
        amount: 25,
        appearance: GoldAppearanceDto::Gold,
        discovered: false,
    }];
    game.next_gold_pile_serial = 2;
    let mana_before = game.resources["demo.resource.mana"].current;
    let mut replay = game.clone();

    for cast in [&mut game, &mut replay] {
        cast.resolve_player_ability(
            RACE_DETECT_DOORS_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf door detection should resolve");
        assert!(cast.revealed_terrain.contains(&trap));
        assert!(cast.revealed_terrain.contains(&door));
        assert!(cast.explored[cast.index(stairs_down).expect("stairs should exist")]);
        assert!(cast.explored[cast.index(stairs_up).expect("stairs should exist")]);
        assert!(!cast.revealed_terrain.contains(&magma));
        assert!(!cast.revealed_terrain.contains(&quartz));

        cast.resolve_player_ability(
            RACE_DETECT_TREASURE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dwarf treasure detection should resolve");
        assert!(cast.revealed_terrain.contains(&magma));
        assert!(cast.revealed_terrain.contains(&quartz));
        assert!(!cast.gold_piles[0].discovered);
    }

    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Dwarf detection knowledge should restore");
    for position in [trap, door, magma, quartz] {
        assert!(restored.revealed_terrain.contains(&position));
    }
    assert!(
        restored.explored[restored
            .index(stairs_down)
            .expect("restored stairs should exist")]
    );
    assert!(
        restored.explored[restored
            .index(stairs_up)
            .expect("restored stairs should exist")]
    );
    assert!(!restored.gold_piles[0].discovered);
}

#[test]
fn formal_nibelung_intrinsics_and_detection_powers_unlock_at_level_ten() {
    let mut game = Game::new_with_build_race_and_name(
        97,
        "demo.build.high-mage-death",
        "rfb-legacy.race.nibelung",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Nibelung High-Mage should create");
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Disenchant),
        ResistanceLevel::Resistant
    );

    let level_nine_experience = crate::stats::experience_required_for_level(9);
    game.apply_unscaled_player_experience(level_nine_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    for (ability_id, attribute) in [
        (
            RACE_DETECT_DOORS_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Wisdom,
        ),
        (
            RACE_DETECT_TREASURE_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Charisma,
        ),
    ] {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .expect("Nibelung detection power should be projected");
        assert_eq!(ability.source, AbilitySourceDto::Race);
        assert_eq!(ability.governing_attribute, Some(attribute));
        assert_eq!(ability.minimum_level, 10);
        assert_eq!(ability.base_resource_cost, 5);
        assert!(!ability.can_cast);
    }

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10) - level_nine_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let snapshot = game.snapshot();
    for ability_id in [
        RACE_DETECT_DOORS_ABILITY_ID,
        RACE_DETECT_TREASURE_ABILITY_ID,
    ] {
        assert!(
            snapshot
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == ability_id)
                .expect("Nibelung detection power should remain projected")
                .can_cast
        );
    }
}

#[test]
fn formal_gnome_phase_door_is_distinct_from_the_sorcery_spell() {
    let mut game = Game::new_with_build_race_and_name(
        98,
        "demo.build.high-mage-sorcery",
        "rfb-legacy.race.gnome",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Gnome Sorcery High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 4);
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));

    let level_four_experience = crate::stats::experience_required_for_level(4);
    game.apply_unscaled_player_experience(level_four_experience, &mut Vec::new());
    let snapshot = game.snapshot();
    let racial = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_PHASE_DOOR_ABILITY_ID)
        .expect("Gnome racial Phase Door should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(racial.minimum_level, 5);
    assert_eq!(racial.base_resource_cost, 2);
    assert!(!racial.can_cast);
    assert_eq!(
        snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.sorcery-phase-door")
            .expect("Sorcery Phase Door should keep its separate identity")
            .source,
        AbilitySourceDto::Learned
    );

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(5) - level_four_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let position_before = game.player.position;
    game.resolve_player_ability(
        RACE_PHASE_DOOR_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Gnome racial Phase Door should resolve");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 2
    );
    assert_ne!(game.player.position, position_before);
    assert!(game.player.position.x.abs_diff(position_before.x) <= 10);
    assert!(game.player.position.y.abs_diff(position_before.y) <= 10);
}

#[test]
fn formal_half_giant_stone_to_mud_does_not_grant_mining_rewards() {
    let mut game = Game::new_with_build_race_and_name(
        99,
        "demo.build.high-mage-death",
        "rfb-legacy.race.half-giant",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Half-Giant High-Mage should create");
    clear_monsters(&mut game);
    game.items.clear();
    game.gold_piles.clear();
    assert_eq!(game.player_infravision_range(), 3);
    assert!(game.player_sustains_attribute(AttributeKind::Strength));
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Shards),
        ResistanceLevel::Resistant
    );

    let level_nineteen_experience = crate::stats::experience_required_for_level(19);
    game.apply_unscaled_player_experience(level_nineteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_STONE_TO_MUD_ABILITY_ID)
        .expect("Half-Giant Stone to Mud should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(racial.minimum_level, 20);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(20) - level_nineteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.quartz-vein");
    game.progress.mining_proficiency = 3_999;
    let materials_before = game.progress.materials.clone();
    let item_serial_before = game.next_item_instance_serial;

    game.resolve_player_ability(
        RACE_STONE_TO_MUD_ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Giant Stone to Mud should resolve");

    assert_eq!(game.terrain_at(target), "demo.terrain.floor");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert_eq!(game.progress.mining_proficiency, 3_999);
    assert_eq!(game.progress.materials, materials_before);
    assert!(game.gold_piles.is_empty());
    assert!(game.items.is_empty());
    assert_eq!(game.next_item_instance_serial, item_serial_before);
}

#[test]
fn formal_half_troll_regeneration_and_berserk_follow_the_effective_race() {
    let mut game = Game::new_with_build_race_and_name(
        100,
        "demo.build.high-mage-death",
        "rfb-legacy.race.half-troll",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Half-Troll High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 3);
    assert!(game.player_sustains_attribute(AttributeKind::Strength));
    assert_eq!(game.player_regeneration_rate_percent(), 200);

    let level_nine_experience = crate::stats::experience_required_for_level(9);
    game.apply_unscaled_player_experience(level_nine_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
        .expect("Half-Troll Berserk should be projected");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength)
    );
    assert_eq!(racial.minimum_level, 10);
    assert_eq!(racial.base_resource_cost, 12);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10) - level_nine_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    game.resolve_player_ability(
        RACE_BERSERK_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Troll Berserk should resolve through the shared ability");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 12
    );
    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_BERSERK)
    );

    let mut human = Game::new_with_build_race_and_name(
        101,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 10;
    assert_eq!(human.player_regeneration_rate_percent(), 100);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.half-troll-form").status;
    form.granted_race_id = Some("rfb-legacy.race.half-troll".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_regeneration_rate_percent(), 200);
    assert!(human.player_sustains_attribute(AttributeKind::Strength));
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_regeneration_rate_percent(), 100);
    assert!(!human.player_sustains_attribute(AttributeKind::Strength));
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_BERSERK_ABILITY_ID)
    );
}

#[test]
fn half_titan_probe_knowledge_survives_losing_the_race_power_and_reloading() {
    let mut game = Game::new_with_build_race_and_name(
        102,
        "demo.build.high-mage-death",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human High-Mage should create");
    clear_monsters(&mut game);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.half-titan-form").status;
    form.granted_race_id = Some("rfb-legacy.race.half-titan".to_owned());
    game.player.statuses.push(form);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Chaos),
        ResistanceLevel::Resistant
    );

    let level_fourteen_experience = crate::stats::experience_required_for_level(14);
    game.apply_unscaled_player_experience(level_fourteen_experience, &mut Vec::new());
    let racial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_PROBE_MONSTERS_ABILITY_ID)
        .expect("temporary Half-Titan form should grant monster probing");
    assert_eq!(racial.source, AbilitySourceDto::Race);
    assert_eq!(
        racial.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(racial.minimum_level, 15);
    assert_eq!(racial.base_resource_cost, 10);
    assert!(!racial.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(15) - level_fourteen_experience,
        &mut Vec::new(),
    );
    game.debug_set_ability_casts_succeed(true);
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let mana_before = mana.current;
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    let target_index = game.index(target).expect("probe target should exist");
    game.glow[target_index] = true;
    game.push_generated_actor(
        "test.half-titan-probe".to_owned(),
        "demo.actor.sheep",
        target,
    );
    let mut events = Vec::new();
    game.resolve_player_ability(
        RACE_PROBE_MONSTERS_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Half-Titan monster probing should resolve");
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 10
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityMonstersProbed { resolution, .. }
            if resolution.monsters.iter().any(|monster| monster.kind_id == "demo.actor.sheep")
    )));

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    game.refresh_player_resource_maxima();
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_PROBE_MONSTERS_ABILITY_ID)
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    let hash = game.state_hash();
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("probe knowledge should not require a current Sniper or Half-Titan source");
    assert!(restored.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert_eq!(restored.state_hash(), hash);
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
fn yeek_scare_monster_and_level_acid_immunity_follow_the_effective_race() {
    fn cast_scare(game: &mut Game) -> Vec<DomainEvent> {
        let mut events = Vec::new();
        game.resolve_player_ability(
            RACE_SCARE_MONSTER_ABILITY_ID,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Yeek scare should resolve");
        events
    }

    let mut game = Game::new_with_build_race_and_name(
        105,
        "demo.build.high-mage-death",
        "rfb-legacy.race.yeek",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Yeek High-Mage should create");
    clear_monsters(&mut game);
    assert_eq!(game.player_infravision_range(), 2);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Resistant
    );

    let level_fourteen_experience = crate::stats::experience_required_for_level(14);
    game.apply_unscaled_player_experience(level_fourteen_experience, &mut Vec::new());
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SCARE_MONSTER_ABILITY_ID)
        .expect("Yeek scare should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Wisdom)
    );
    assert_eq!(locked.minimum_level, 15);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (15, 15));
    assert!(!locked.can_cast);

    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(15) - level_fourteen_experience,
        &mut Vec::new(),
    );
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana");
    mana.current = mana.maximum;
    let expected_power = u16::try_from(
        20_i32
            .saturating_add(crate::stats::original_save_adjustment(
                game.effective_player_attributes()
                    .index(AttributeKind::Charisma),
            ))
            .max(1),
    )
    .expect("level-fifteen fear power should fit");
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SCARE_MONSTER_ABILITY_ID)
        .expect("Yeek scare should remain projected");
    assert!(available.can_cast);
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            status_kind_id,
            duration_ticks: 1,
            duration_dice: 3,
            duration_sides: 7,
            power: Some(power),
            ..
        }] if status_kind_id == STATUS_FEAR && *power == expected_power
    ));

    let mut level_fifty = game.clone();
    level_fifty.progress.level = 50;
    let level_fifty = level_fifty
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_SCARE_MONSTER_ABILITY_ID)
        .expect("level-fifty Yeek scare");
    let expected_level_fifty_power = expected_power.saturating_add(45);
    assert!(matches!(
        level_fifty.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_sides: 25,
            power: Some(power),
            ..
        }] if *power == expected_level_fifty_power
    ));

    game.progress.level = 19;
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Resistant
    );
    game.progress.level = 20;
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Immune
    );
    game.progress.level = 15;

    game.player.position = Position { x: 3, y: 3 };
    for position in [Position { x: 3, y: 3 }, Position { x: 4, y: 3 }] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.push_generated_actor(
        "test.yeek-scare-target".to_owned(),
        "demo.actor.sheep",
        Position { x: 4, y: 3 },
    );

    let failure_percent = available.failure_percent;
    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Yeek scare should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_mana = failed.resources["demo.resource.mana"].current;
    let failed_events = cast_scare(&mut failed);
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(
        failed.resources["demo.resource.mana"].current,
        failed_mana - 15
    );
    assert!(
        failed.entities[0]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_FEAR)
    );

    let success_seed = (0..1_000)
        .find(|seed| {
            let mut candidate = game.clone();
            candidate.rng = RfbRng::seeded(*seed);
            candidate.debug_set_ability_casts_succeed(true);
            cast_scare(&mut candidate);
            candidate.entities[0]
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_FEAR)
        })
        .expect("Yeek scare should have a successful fear seed");
    game.rng = RfbRng::seeded(success_seed);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Yeek scare setup should reload");
    game.debug_set_ability_casts_succeed(true);
    restored.debug_set_ability_casts_succeed(true);
    let mana_before = game.resources["demo.resource.mana"].current;
    let events = cast_scare(&mut game);
    let restored_events = cast_scare(&mut restored);
    assert_eq!(restored_events, events);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - 15
    );
    let fear = game.entities[0]
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_FEAR)
        .expect("successful scare should frighten the target");
    assert!((4..=22).contains(&fear.remaining_ticks));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::ApplyStatus {
                power: Some(power),
                change: AbilityStatusChangeDto::Added,
                ..
            }] if *power == expected_power)
    )));

    let mut immune = game.clone();
    clear_monsters(&mut immune);
    immune.push_generated_actor(
        "test.yeek-immune-target".to_owned(),
        "demo.actor.metal-babble",
        Position { x: 4, y: 3 },
    );
    immune.rng = RfbRng::seeded(7);
    immune.debug_set_ability_casts_succeed(true);
    let draws_before = immune.rng_draw_counter();
    let immune_events = cast_scare(&mut immune);
    assert_eq!(immune.rng_draw_counter(), draws_before + 1);
    assert!(immune_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::ApplyStatus {
                change: AbilityStatusChangeDto::Immune,
                ..
            }])
    )));

    let mut human = Game::new_with_build_race_and_name(
        106,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    human.progress.level = 20;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.yeek-form").status;
    form.granted_race_id = Some("rfb-legacy.race.yeek".to_owned());
    human.player.statuses.push(form);
    assert_eq!(human.player_infravision_range(), 2);
    assert_eq!(
        human.effective_player_resistances().level(DamageType::Acid),
        ResistanceLevel::Immune
    );
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == RACE_SCARE_MONSTER_ABILITY_ID)
    );
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(human.player_infravision_range(), 0);
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
            .all(|ability| ability.id != RACE_SCARE_MONSTER_ABILITY_ID)
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
fn formal_dwarf_detection_failure_spills_mana_into_hp_without_revealing() {
    let mut game = Game::new_with_build_race_and_name(
        95,
        "demo.build.high-mage-death",
        "rfb-legacy.race.dwarf",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Dwarf High-Mage should create");
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(10),
        &mut Vec::new(),
    );
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let blocker = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let treasure = Position {
        x: game.player.position.x + 2,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, blocker, "demo.terrain.wall");
    replace_terrain(&mut game, treasure, "demo.terrain.quartz-hidden-treasure");
    let treasure_index = game.index(treasure).expect("treasure should exist");
    game.explored[treasure_index] = false;
    game.revealed_terrain.remove(&treasure);
    game.gold_piles = vec![GoldPile {
        id: "generated.gold.1".to_owned(),
        position: treasure,
        amount: 25,
        appearance: GoldAppearanceDto::Gold,
        discovered: false,
    }];
    game.next_gold_pile_serial = 2;
    let failure_percent = game
        .snapshot()
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_DETECT_TREASURE_ABILITY_ID)
        .expect("Dwarf treasure detection should be projected")
        .failure_percent;
    let seed = (0..4_096)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Dwarf treasure detection should have a reachable failure roll");
    game.rng = RfbRng::seeded(seed);
    let hp_before = game.player.hp;
    let tick_before = game.world_tick;

    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: RACE_DETECT_TREASURE_ABILITY_ID.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );

    assert_eq!(game.world_tick, tick_before + 10);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp_before - 2);
    assert!(!game.explored[treasure_index]);
    assert!(!game.revealed_terrain.contains(&treasure));
    assert!(!game.gold_piles[0].discovered);
}

#[test]
fn dwarf_intrinsics_follow_the_effective_race_without_replacing_birth_rewards() {
    let mut game = Game::new_with_build_race_and_name(
        96,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 20;
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Normal
    );
    let pending_before = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("birth Human should retain its level twenty reward");
    assert_eq!(pending_before.reward_id, "human-talent");

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.dwarf-form").status;
    form.granted_race_id = Some("rfb-legacy.race.dwarf".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 5);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Resistant
    );
    let snapshot = game.snapshot();
    for (ability_id, attribute) in [
        (
            RACE_DETECT_DOORS_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Wisdom,
        ),
        (
            RACE_DETECT_TREASURE_ABILITY_ID,
            rfb_protocol::AttributeKindDto::Charisma,
        ),
    ] {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .expect("temporary Dwarf form should grant both detection powers");
        assert_eq!(ability.source, AbilitySourceDto::Race);
        assert_eq!(ability.governing_attribute, Some(attribute));
    }
    assert_eq!(
        snapshot
            .player
            .pending_race_mutation_choice
            .expect("effective race must not replace birth-race rewards")
            .reward_id,
        "human-talent"
    );
    assert!(game.progress.locked_mutation_ids.is_empty());

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| !matches!(
                ability.id.as_str(),
                RACE_DETECT_DOORS_ABILITY_ID | RACE_DETECT_TREASURE_ABILITY_ID
            ))
    );
}

#[test]
fn racial_berserk_pays_hp_obeys_fear_and_never_shortens_a_stronger_rage() {
    let mut game = Game::new_with_build_race_and_name(
        0,
        "demo.build.warrior",
        "rfb-legacy.race.barbarian",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Barbarian warrior should create");
    game.progress.level = 8;
    clear_monsters(&mut game);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_FEAR, 5, "test.fear").status);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();

    game.resolve_player_ability(
        RACE_BERSERK_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("fear rejection should resolve cleanly");
    assert_eq!(game.player.hp, hp_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "afraid"
    ));

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_FEAR);
    game.debug_set_ability_casts_succeed(true);
    events.clear();
    game.resolve_player_ability(
        RACE_BERSERK_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("racial berserk should resolve");
    let resolution = mutation_cast_resolution(&events);
    assert_eq!(resolution.resource_id, None);
    assert_eq!(resolution.resource_paid, 0);
    assert_eq!(resolution.hp_paid, 10);
    assert_eq!(game.player.hp, hp_before - 10);
    let rage = game
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_BERSERK)
        .expect("racial berserk should apply the shared rage status");
    assert!((11..=18).contains(&rage.remaining_ticks));
    assert_eq!(rage.granted_equipment_bonuses.melee_damage, 4);
    rage.remaining_ticks = 100;

    events.clear();
    game.resolve_player_ability(
        RACE_BERSERK_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("a repeated racial berserk should resolve");
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BERSERK)
            .expect("the stronger rage should remain")
            .remaining_ticks,
        100
    );
}

#[test]
fn formal_barbarian_berserk_spills_sp_into_hp_pays_on_failure_and_rejects_zero_budget() {
    let prepare = || {
        let mut game = Game::new_with_build_race_and_name(
            0,
            "demo.build.high-mage-death",
            "rfb-legacy.race.barbarian",
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("Barbarian High-Mage should create");
        game.progress.level = 8;
        clear_monsters(&mut game);
        game
    };

    let mut succeeded = prepare();
    succeeded.debug_set_ability_casts_succeed(true);
    succeeded
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let hp_before = succeeded.player.hp;
    let mut events = Vec::new();
    succeeded
        .resolve_player_ability(
            RACE_BERSERK_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Barbarian Berserk should spend SP before HP");
    let resolution = mutation_cast_resolution(&events);
    assert!(resolution.succeeded);
    assert_eq!(resolution.resource_paid, 3);
    assert_eq!(resolution.hp_paid, 7);
    assert_eq!(succeeded.player.hp, hp_before - 7);

    let mut failed = prepare();
    failed
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let failure_percent = failed
        .snapshot()
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_BERSERK_ABILITY_ID)
        .expect("Barbarian Berserk should be projected")
        .failure_percent;
    let seed = (0..4_096)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Barbarian Berserk should have a reachable failure roll");
    failed.rng = RfbRng::seeded(seed);
    let hp_before = failed.player.hp;
    events.clear();
    failed
        .resolve_player_ability(
            RACE_BERSERK_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("a failed Barbarian Berserk should still pay");
    let resolution = mutation_cast_resolution(&events);
    assert!(!resolution.succeeded);
    assert_eq!(resolution.resource_paid, 3);
    assert_eq!(resolution.hp_paid, 7);
    assert_eq!(failed.player.hp, hp_before - 7);

    let mut rejected = prepare();
    rejected
        .resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 0;
    rejected.player.hp = 9;
    let draws_before = rejected.rng_draw_counter();
    events.clear();
    rejected
        .resolve_player_ability(
            RACE_BERSERK_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("an empty Barbarian power budget should reject cleanly");
    assert_eq!(rejected.player.hp, 9);
    assert_eq!(rejected.rng_draw_counter(), draws_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "insufficient-resource"
    ));
}

fn formal_hobbit_high_mage(seed: u64, level: u16) -> Game {
    let mut game = Game::new_with_build_race_and_name(
        seed,
        "demo.build.high-mage-death",
        "rfb-legacy.race.hobbit",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Hobbit High-Mage should create");
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(
        crate::stats::experience_required_for_level(level),
        &mut Vec::new(),
    );
    game
}

#[test]
fn formal_hobbit_create_food_projects_and_round_trips_an_acquired_ration() {
    let locked = formal_hobbit_high_mage(87, 14)
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("Hobbit Create Food should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(locked.minimum_level, 15);
    assert_eq!(locked.base_resource_cost, 10);
    assert_eq!(locked.failure_percent, 100);
    assert!(!locked.can_cast);

    let mut game = formal_hobbit_high_mage(87, 15);
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("Hobbit Create Food should remain projected");
    assert!(available.can_cast);
    assert!(available.failure_percent < 100);
    game.debug_set_ability_casts_succeed(true);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let mut replay = game.clone();
    let hp_before = game.player.hp;
    let tick_before = game.world_tick;
    let serial_before = game.next_item_instance_serial;
    for cast in [&mut game, &mut replay] {
        dispatch_next(
            cast,
            GameCommand::CastAbility {
                ability_id: RACE_CREATE_FOOD_ABILITY_ID.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
    }
    assert_eq!(game.state_hash(), replay.state_hash());
    assert_eq!(game.world_tick, tick_before + 10);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp_before - 7);
    assert_eq!(game.next_item_instance_serial, serial_before + 1);

    let created = game
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.ration-of-food"
                && item.origin_kind == Some(ItemOriginKindDto::Acquire)
        })
        .expect("Create Food should produce an acquired ration");
    let created_id = created.id.clone();
    assert_eq!(created.quantity, 1);
    assert_eq!(created.quality, rfb_protocol::ItemQualityDto::Ordinary);
    assert!(created.affix_ids.is_empty());
    assert!(created.curse.is_none());
    let ItemLocation::Ground(position) = created.location else {
        panic!("created ration should land on the ground");
    };
    assert!(game.is_walkable(position));
    assert!(rfb_distance(position, game.player.position) <= 3);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Hobbit Create Food save should restore");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("created ration should survive the save round trip");
    assert_eq!(restored_item.kind_id, "demo.item.ration-of-food");
    assert_eq!(restored_item.quantity, 1);
    assert_eq!(restored_item.origin_kind, Some(ItemOriginKindDto::Acquire));
    assert_eq!(
        restored_item.quality,
        rfb_protocol::ItemQualityDto::Ordinary
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_hobbit_create_food_failure_pays_and_creates_nothing() {
    let mut game = formal_hobbit_high_mage(88, 15);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("High-Mage should have mana")
        .current = 3;
    let failure_percent = game
        .snapshot()
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("Hobbit Create Food should be projected")
        .failure_percent;
    let seed = (0..4_096)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(failure_percent)
        })
        .expect("Hobbit Create Food should have a reachable failure roll");
    game.rng = RfbRng::seeded(seed);
    let acquired_before = game
        .items
        .iter()
        .filter(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
        .count();
    let serial_before = game.next_item_instance_serial;
    let hp_before = game.player.hp;
    let tick_before = game.world_tick;

    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: RACE_CREATE_FOOD_ABILITY_ID.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );

    assert_eq!(game.world_tick, tick_before + 10);
    assert_eq!(game.resources["demo.resource.mana"].current, 0);
    assert_eq!(game.player.hp, hp_before - 7);
    assert_eq!(game.next_item_instance_serial, serial_before);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
            .count(),
        acquired_before
    );
}

#[test]
fn hobbit_intrinsics_follow_the_effective_race() {
    let mut game = Game::new_with_build_race_and_name(
        89,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human Warrior should create");
    game.progress.level = 15;
    assert_eq!(game.player_infravision_range(), 0);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_CREATE_FOOD_ABILITY_ID)
    );

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.hobbit-form").status;
    form.granted_race_id = Some("rfb-legacy.race.hobbit".to_owned());
    game.player.statuses.push(form);
    assert_eq!(game.player_infravision_range(), 4);
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_CREATE_FOOD_ABILITY_ID)
        .expect("temporary Hobbit form should grant Create Food");
    assert_eq!(ability.source, AbilitySourceDto::Race);
    assert_eq!(
        ability.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );

    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != RACE_CREATE_FOOD_ABILITY_ID)
    );
}

fn mutation_ability_catalog(
    minimum_level: u16,
    cost: u32,
    base_failure_percent: u8,
) -> Arc<rfb_content::ContentCatalog> {
    mutation_ability_catalog_with_effect(
        minimum_level,
        cost,
        base_failure_percent,
        AbilityEffectDefinition::NoOp {
            reason: "mutation-contract".to_owned(),
        },
    )
}

fn mutation_ability_catalog_with_effect(
    minimum_level: u16,
    cost: u32,
    base_failure_percent: u8,
    effect: AbilityEffectDefinition,
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
    ability.effect = effect;
    ability.level_scaling.clear();
    ability.player = None;
    artifact.content.abilities.push(ability);
    artifact
        .content
        .mutations
        .iter_mut()
        .find(|mutation| mutation.id == MUTATION_CONTRACT_ID)
        .expect("Spit Acid mutation should exist")
        .activation = Some(InnatePowerDefinition {
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

fn create_item_ability_catalog(base_failure_percent: u8) -> Arc<rfb_content::ContentCatalog> {
    mutation_ability_catalog_with_effect(
        1,
        1,
        base_failure_percent,
        AbilityEffectDefinition::CreateItem {
            item_kind_id: "demo.item.ration-of-food".to_owned(),
            quantity: 1,
        },
    )
}

#[test]
fn create_item_ability_places_an_acquired_item_and_merges_repeated_casts() {
    let mut game = mutation_ability_game(create_item_ability_catalog(0), "demo.build.warrior");
    game.debug_set_ability_casts_succeed(true);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    let position = game.player.position;
    let serial_before = game.next_item_instance_serial;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();

    game.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("plain item creation should resolve");

    let created = game
        .items
        .iter()
        .find(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
        .expect("a successful creation should place an acquired item");
    let created_id = created.id.clone();
    assert_eq!(created.kind_id, "demo.item.ration-of-food");
    assert_eq!(created.quantity, 1);
    assert_eq!(created.location, ItemLocation::Ground(position));
    assert_eq!(game.next_item_instance_serial, serial_before + 1);
    assert_eq!(changed, BTreeSet::from([position]));
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::CreateItem {
                    item_kind_id,
                    quantity: 1,
                    position: effect_position,
                    destination_item_ids,
                    ..
                }] if item_kind_id == "demo.item.ration-of-food"
                    && *effect_position == position
                    && destination_item_ids == std::slice::from_ref(&created_id)
            )
    )));

    let serial_after_first = game.next_item_instance_serial;
    events.clear();
    changed.clear();
    game.resolve_player_ability(
        MUTATION_CONTRACT_ABILITY_ID,
        TargetSelection::SelfTarget,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    )
    .expect("a repeated creation should resolve");
    let merged = game
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("the original acquired stack should remain");
    assert_eq!(merged.quantity, 2);
    assert_eq!(game.next_item_instance_serial, serial_after_first);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("an acquired item should survive a save round trip");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == created_id)
        .expect("the acquired item should be restored");
    assert_eq!(restored_item.quantity, 2);
    assert_eq!(restored_item.origin_kind, Some(ItemOriginKindDto::Acquire));
}

#[test]
fn create_item_ability_uses_rfb_nearby_scoring_and_failure_creates_nothing() {
    let prepare = |failure| {
        let mut game =
            mutation_ability_game(create_item_ability_catalog(failure), "demo.build.warrior");
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        game
    };

    let mut first = prepare(0);
    first.debug_set_ability_casts_succeed(true);
    let origin = first.player.position;
    give_inventory_item(&mut first, "test.drop-blocker", "demo.item.arrow");
    first
        .items
        .iter_mut()
        .find(|item| item.id == "test.drop-blocker")
        .expect("drop blocker should exist")
        .location = ItemLocation::Ground(origin);
    let mut second = first.clone();
    let cast = |game: &mut Game| {
        game.resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("nearby item creation should resolve");
        game.items
            .iter()
            .find(|item| item.origin_kind == Some(ItemOriginKindDto::Acquire))
            .expect("created item should exist")
            .location
            .clone()
    };
    let first_location = cast(&mut first);
    let second_location = cast(&mut second);
    assert_eq!(first_location, second_location);
    let ItemLocation::Ground(created_position) = first_location else {
        panic!("created item should be on the ground");
    };
    assert_ne!(created_position, origin);
    assert_eq!(rfb_distance(created_position, origin), 1);

    let mut failed = prepare(95);
    failed.player.hp = 20;
    failed.rng = RfbRng::seeded(0);
    let serial_before = failed.next_item_instance_serial;
    let item_count_before = failed.items.len();
    let mut events = Vec::new();
    failed
        .resolve_player_ability(
            MUTATION_CONTRACT_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed item creation should resolve");
    assert!(matches!(
        events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.next_item_instance_serial, serial_before);
    assert_eq!(failed.items.len(), item_count_before);
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
fn fear_blocks_class_and_mutation_power_sources_without_cost_or_rng() {
    let catalog = mutation_ability_catalog(1, 7, 30);
    let mutation = mutation_ability_game(catalog.clone(), "demo.build.warrior");
    let class = Game::from_content_with_build(0, catalog, DEFAULT_WORLD_ID, "demo.build.archer")
        .expect("Archer build should create");

    for (mut game, ability_id, expected_source) in [
        (
            mutation,
            MUTATION_CONTRACT_ABILITY_ID,
            AbilitySourceDto::Mutation,
        ),
        (
            class,
            "demo.ability.archer-create-shots",
            AbilitySourceDto::Class,
        ),
    ] {
        game.player
            .statuses
            .push(monster_combat::melee_status(STATUS_FEAR, 5, "test.fear").status);
        let projected = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == ability_id)
            .expect("the power should remain projected while afraid");
        assert_eq!(projected.source, expected_source);
        assert!(!projected.can_cast);
        let hp_before = game.player.hp;
        let draws_before = game.rng_draw_counter();
        let mut events = Vec::new();

        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("fear rejection should resolve cleanly");

        assert_eq!(game.player.hp, hp_before);
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::AbilityCastUnavailable { reason, .. }] if reason == "afraid"
        ));
    }
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
    failed.rng = RfbRng::seeded(0);
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
    assert_eq!(capped_failure.innate_power_failure_percent(&activation), 11);

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
fn p55b_eagle_summon_includes_unseen_unique_eagles() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let caster_position = Position {
        x: game.player.position.x + 4,
        y: game.player.position.y,
    };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.ancient-roc",
        "demo.actor.the-ancient-roc-of-okeldad",
        caster_position,
        3_872,
        130,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-eagle-l55-1d3-1")
        .expect("P55B eagle summon should compile")
        .clone();
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("unseen eagles should be summon candidates");
    let MonsterAbilityTargetPlan::SummonCategory {
        candidate_kind_ids, ..
    } = plan.target
    else {
        panic!("S_EAGLE should retain a category summon plan");
    };
    assert_eq!(
        candidate_kind_ids.into_iter().collect::<BTreeSet<_>>(),
        [
            "demo.actor.eagle".to_owned(),
            "demo.actor.great-eagle".to_owned(),
            "demo.actor.gwaihir-the-windlord".to_owned(),
            "demo.actor.meneldor-the-swift".to_owned(),
            "demo.actor.thorondor".to_owned(),
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn p75a_no_summon_monsters_are_rejected_by_shared_candidate_filter() {
    let game = Game::new(0);
    let ring = game
        .content
        .actor("demo.actor.a-plain-gold-ring")
        .expect("P75A Plain Gold Ring should compile");
    assert!(ring.tags.iter().any(|tag| tag == "no-summon"));
    assert!(!actor_answers_summons(ring));

    let cyberdemon = game
        .content
        .actor("demo.actor.cyberdemon")
        .expect("Cyberdemon should remain available");
    assert!(cyberdemon.tags.iter().any(|tag| tag == "cyber"));
    assert!(actor_answers_summons(cyberdemon));
}

#[test]
fn p56b_gospel_summon_caps_one_d_four_at_three_tracking_pixels() {
    fn summon(seed: u64, capped: bool) -> Vec<String> {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 80, y: 20 };
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.gospel",
            "demo.actor.the-gospel-of-mug",
            Position { x: 4, y: 3 },
            1_665,
            128,
            100,
            true,
        ));
        let mut ability = game
            .content
            .ability("rfb-legacy.ability.summon-tracking-pixel-l56-1d4-max3")
            .expect("P56B Gospel summon should compile")
            .clone();
        assert!(matches!(
            ability_effect_spec_dto(&ability.effect),
            AbilityEffectSpecDto::SummonCategory {
                maximum_count: Some(3),
                ..
            }
        ));
        if !capped
            && let AbilityEffectDefinition::SummonCategory { maximum_count, .. } =
                &mut ability.effect
        {
            *maximum_count = None;
        }
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("Gospel special summon should have a target plan");
        game.resolve_monster_ability_plan(
            0,
            "demo.actor.the-gospel-of-mug",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("Gospel special should summon")
        .summoned_kind_ids
    }

    let seed = (0..128)
        .find(|seed| summon(*seed, false).len() == 4)
        .expect("a bounded seed should roll four summons");
    let summoned = summon(seed, true);
    assert_eq!(summoned.len(), 3);
    assert!(
        summoned
            .iter()
            .all(|kind_id| kind_id == "demo.actor.tracking-pixel")
    );
}

#[test]
fn p60_gragomani_rolls_count_then_one_weighted_kind_for_the_whole_batch() {
    fn expected(seed: u64) -> (usize, &'static str) {
        let mut rng = RfbRng::seeded(seed);
        let count = usize::try_from(rng.bounded(4) + 5).expect("1d4+4 fits usize");
        let kind_id = if rng.bounded(4) == 0 {
            "demo.actor.malicious-leprechaun"
        } else {
            "demo.actor.leprechaun-fanatic"
        };
        (count, kind_id)
    }

    fn summon(seed: u64) -> Vec<String> {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 80, y: 20 };
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.gragomani",
            "demo.actor.gragomani-the-leprechaun-prophet",
            Position { x: 4, y: 3 },
            5_082,
            131,
            100,
            true,
        ));
        let ability = game
            .content
            .ability("rfb-legacy.ability.summon-gragomani-followers-1d4-4")
            .expect("P60 Gragomani summon should compile")
            .clone();
        let AbilityEffectSpecDto::SummonCategory {
            batch_candidates, ..
        } = ability_effect_spec_dto(&ability.effect)
        else {
            panic!("Gragomani special should remain a category summon");
        };
        assert_eq!(
            batch_candidates,
            vec![
                AbilitySummonCandidateSpecDto {
                    actor_kind_id: "demo.actor.malicious-leprechaun".to_owned(),
                    weight: 1,
                },
                AbilitySummonCandidateSpecDto {
                    actor_kind_id: "demo.actor.leprechaun-fanatic".to_owned(),
                    weight: 3,
                },
            ]
        );
        game.rng = RfbRng::seeded(seed);
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("Gragomani special summon should have a target plan");
        game.resolve_monster_ability_plan(
            0,
            "demo.actor.gragomani-the-leprechaun-prophet",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("Gragomani special should summon")
        .summoned_kind_ids
    }

    let malicious_seed = (0..128)
        .find(|seed| expected(*seed).1 == "demo.actor.malicious-leprechaun")
        .expect("a bounded seed should select the 1-in-4 candidate");
    let fanatic_seed = (0..128)
        .find(|seed| expected(*seed).1 == "demo.actor.leprechaun-fanatic")
        .expect("a bounded seed should select the 3-in-4 candidate");
    for seed in [malicious_seed, fanatic_seed] {
        let (count, kind_id) = expected(seed);
        assert_eq!(summon(seed), vec![kind_id.to_owned(); count]);
    }
}

#[test]
fn p70_aegir_rolls_count_then_floods_then_selects_one_retinue_kind() {
    fn expected(seed: u64) -> (usize, &'static str) {
        let mut rng = RfbRng::seeded(seed);
        let count = usize::try_from(rng.bounded(4) + 1).expect("1d4 fits usize");
        let kind_id = if rng.bounded(2) == 0 {
            "demo.actor.sea-giant"
        } else {
            "demo.actor.lesser-kraken"
        };
        (count, kind_id)
    }

    let seed_for = |kind_id| {
        (0..128)
            .find(|seed| expected(*seed).1 == kind_id)
            .expect("bounded seeds should cover both Aegir candidates")
    };
    for seed in [
        seed_for("demo.actor.sea-giant"),
        seed_for("demo.actor.lesser-kraken"),
    ] {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 80, y: 20 };
        let origin = Position { x: 20, y: 20 };
        let permanent = Position { x: 21, y: 20 };
        replace_terrain(&mut game, permanent, "demo.terrain.permanent-wall");
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.aegir",
            "demo.actor.aegir-god-king-of-the-sea-giants",
            origin,
            9_196,
            129,
            100,
            true,
        ));
        let ability = game
            .content
            .ability("rfb-legacy.ability.summon-aegir-retinue-1d4")
            .expect("P70 Aegir summon should compile")
            .clone();
        game.rng = RfbRng::seeded(seed);
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("water flow should make aquatic summon positions viable");
        let mut events = Vec::new();
        let mut changed = BTreeSet::new();
        let resolution = game.resolve_monster_ability_plan(
            0,
            "demo.actor.aegir-god-king-of-the-sea-giants",
            &plan,
            &mut events,
            &mut changed,
            &mut Vec::new(),
        );

        let (count, kind_id) = expected(seed);
        let summon = resolution.summon.expect("Aegir special should summon");
        assert_eq!(summon.summoned_kind_ids, vec![kind_id.to_owned(); count]);
        assert_eq!(game.rng.draw_counter, 2);
        assert_eq!(
            game.terrain[game.index(origin).expect("origin should remain in bounds")],
            "demo.terrain.surface-water-deep"
        );
        assert_eq!(
            game.terrain[game
                .index(Position { x: 20, y: 12 })
                .expect("radius-eight cell should remain in bounds")],
            "demo.terrain.surface-water-deep"
        );
        assert_eq!(
            game.terrain[game
                .index(Position { x: 20, y: 11 })
                .expect("radius-nine cell should remain in bounds")],
            "demo.terrain.floor"
        );
        assert_eq!(
            game.terrain[game.index(permanent).expect("wall should remain in bounds")],
            "demo.terrain.permanent-wall"
        );
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::AbilityTerrainTransformed { ability_id, resolution }
                if ability_id == "rfb-legacy.ability.summon-aegir-retinue-1d4"
                    && resolution.center == origin
                    && resolution.radius == 8
                    && resolution.target_terrain_id == "demo.terrain.surface-water-deep"
        )));
    }
}

#[test]
fn p79_special_summons_keep_hermes_count_and_odin_retinue_choice() {
    fn summon(seed: u64, caster_kind_id: &str, ability_id: &str) -> Vec<String> {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 80, y: 20 };
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.p79-caster",
            caster_kind_id,
            Position { x: 20, y: 20 },
            10_000,
            140,
            100,
            true,
        ));
        let ability = game
            .content
            .ability(ability_id)
            .expect("P79 special summon should compile")
            .clone();
        game.rng = RfbRng::seeded(seed);
        let plan = game
            .monster_ability_target_plan(0, ability, 1)
            .expect("P79 summon should have candidates and space");
        game.resolve_monster_ability_plan(
            0,
            caster_kind_id,
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("P79 special should summon")
        .summoned_kind_ids
    }

    let hermes_seed = (0..256)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(16) == 15
        })
        .expect("bounded seeds should cover a sixteen summon roll");
    assert_eq!(
        summon(
            hermes_seed,
            "demo.actor.hermes-the-messenger-god",
            "rfb-legacy.ability.summon-magic-mushroom-patch-l15-1d16",
        ),
        vec!["demo.actor.magic-mushroom-patch".to_owned(); 16]
    );

    let expected_odin = |seed| {
        let mut rng = RfbRng::seeded(seed);
        let _discarded_count = rng.bounded(4);
        if rng.bounded(2) == 0 {
            "demo.actor.einheri-berserker"
        } else {
            "demo.actor.valkyrie"
        }
    };
    for target in ["demo.actor.einheri-berserker", "demo.actor.valkyrie"] {
        let seed = (0..128)
            .find(|seed| expected_odin(*seed) == target)
            .expect("bounded seeds should cover both Odin retinue choices");
        assert_eq!(
            summon(
                seed,
                "demo.actor.odin-the-all-father",
                "rfb-legacy.ability.summon-odin-retinue-1d4-max1",
            ),
            [target.to_owned()]
        );
    }
}

#[test]
fn p80_variant_maintainer_cast_summons_only_software_bugs() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 80, y: 20 };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.p80-caster",
        "demo.actor.the-variant-maintainer",
        Position { x: 20, y: 20 },
        225,
        120,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-software-bug-l14-1d3-1")
        .expect("software bug summon should compile")
        .clone();
    game.rng = RfbRng::seeded(
        (0..128)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(3) == 2
            })
            .expect("bounded seeds should cover a four-bug summon"),
    );
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("software bug summon should have space");
    let summon = game
        .resolve_monster_ability_plan(
            0,
            "demo.actor.the-variant-maintainer",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("software bug summon should resolve");
    assert_eq!(
        summon.summoned_kind_ids,
        vec!["demo.actor.software-bug".to_owned(); 4]
    );
}

#[test]
fn p71_banor_rupart_split_and_merge_preserve_hp_without_recording_deaths() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 80, y: 20 };
    let origin = Position { x: 20, y: 20 };
    game.push_generated_actor(
        "test.banor-rupart".to_owned(),
        ecology::BANOR_RUPART_COMBINED_KIND_ID,
        origin,
    );
    game.entities[0].hp = 3_001;
    let ability = game
        .content
        .ability("rfb-legacy.ability.banor-rupart-transform")
        .expect("P71 transform should compile")
        .clone();
    let split_plan = game
        .monster_ability_target_plan(0, ability.clone(), 1)
        .expect("combined form should split with one adjacent cell");
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    let split = game.resolve_monster_ability_plan(
        0,
        ecology::BANOR_RUPART_COMBINED_KIND_ID,
        &split_plan,
        &mut Vec::new(),
        &mut changed,
        &mut removed,
    );

    assert_eq!(removed, ["test.banor-rupart"]);
    assert!(game.defeated_limited_actor_counts.is_empty());
    assert_eq!(
        split
            .summon
            .expect("split should project forms")
            .entity_ids
            .len(),
        2
    );
    for kind_id in [ecology::BANOR_KIND_ID, ecology::RUPART_KIND_ID] {
        let actor = game
            .entities
            .iter()
            .find(|actor| actor.kind_id == kind_id)
            .expect("both split forms should exist");
        assert_eq!((actor.hp, actor.max_hp), (1_501, 3_500));
    }
    assert_eq!(
        game.actor_kind_available_instance_count(ecology::BANOR_RUPART_COMBINED_KIND_ID),
        0
    );

    let hash = game.state_hash();
    let mut game = Game::from_save(game.to_save()).expect("split forms should round-trip");
    assert_eq!(game.state_hash(), hash);
    let banor_index = game
        .entities
        .iter()
        .position(|actor| actor.kind_id == ecology::BANOR_KIND_ID)
        .expect("Banor should restore");
    let rupart_position = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == ecology::RUPART_KIND_ID)
        .expect("Rupart should restore")
        .position;
    game.entities[banor_index].hp = 1_000;
    game.entities
        .iter_mut()
        .find(|actor| actor.kind_id == ecology::RUPART_KIND_ID)
        .expect("Rupart should remain available")
        .hp = 1_200;
    let merge_plan = game
        .monster_ability_target_plan(banor_index, ability, 1)
        .expect("two split forms should merge");
    let mut removed = Vec::new();
    let merge = game.resolve_monster_ability_plan(
        banor_index,
        ecology::BANOR_KIND_ID,
        &merge_plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut removed,
    );

    assert_eq!(removed.len(), 2);
    assert!(game.defeated_limited_actor_counts.is_empty());
    assert_eq!(game.entities.len(), 1);
    assert_eq!(
        (
            game.entities[0].kind_id.as_str(),
            game.entities[0].position,
            game.entities[0].hp,
            game.entities[0].max_hp,
        ),
        (
            ecology::BANOR_RUPART_COMBINED_KIND_ID,
            rupart_position,
            2_200,
            7_000,
        )
    );
    let merged_ids = merge
        .summon
        .expect("merge should project combined form")
        .entity_ids;
    assert_eq!(merged_ids.len(), 1);
    assert!(!removed.contains(&merged_ids[0]));
}

#[test]
fn p71_banor_rupart_split_requires_one_adjacent_open_cell() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.permanent-wall".to_owned());
    let origin = Position { x: 20, y: 20 };
    replace_terrain(&mut game, origin, "demo.terrain.floor");
    game.push_generated_actor(
        "test.banor-rupart".to_owned(),
        ecology::BANOR_RUPART_COMBINED_KIND_ID,
        origin,
    );
    let ability = game
        .content
        .ability("rfb-legacy.ability.banor-rupart-transform")
        .expect("P71 transform should compile")
        .clone();

    assert!(matches!(
        game.monster_ability_target_plan(0, ability, 1),
        Err(MonsterAbilityPlanRejection {
            reason: MonsterAbilityRejectionReasonDto::NoSpace,
            ..
        })
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
        let unlife_before = game.virtue_current(VirtueKindDto::Unlife);
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
        assert_eq!(
            game.virtue_current(VirtueKindDto::Unlife),
            unlife_before + 1
        );
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
    game.apply_player_experience(0, &mut Vec::new());
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

#[test]
fn p76_unique_summons_use_the_caster_level_window_and_exclude_unique2() {
    let mut game = Game::new(241);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 80, y: 20 };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.ptah",
        "demo.actor.ptah-the-divine-craftsman",
        Position { x: 20, y: 20 },
        1_000,
        135,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-unique-l83-1d2")
        .expect("P76 S_UNIQUE ability should compile")
        .clone();
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("P76 S_UNIQUE should have eligible candidates");
    let MonsterAbilityTargetPlan::SummonCategory {
        candidate_kind_ids, ..
    } = plan.target
    else {
        panic!("S_UNIQUE should remain a category summon");
    };
    assert!(!candidate_kind_ids.is_empty());
    for kind_id in candidate_kind_ids {
        let candidate = game
            .content
            .actor(&kind_id)
            .expect("planned unique candidate should exist");
        assert!((43..=83).contains(&candidate.level));
        assert!(candidate.tags.iter().any(|tag| tag == "unique"));
        assert!(!candidate.tags.iter().any(|tag| tag == "unique2"));
    }
}

#[test]
fn p76_osiris_family_summon_creates_horus_and_isis_as_one_cast() {
    let mut game = Game::new(251);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 80, y: 20 };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.osiris",
        "demo.actor.osiris-the-reborn",
        Position { x: 20, y: 20 },
        1_000,
        135,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-family-osiris-the-reborn")
        .expect("P76 Osiris family summon should compile")
        .clone();
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("Osiris should have family candidates and space");
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.osiris-the-reborn",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    let summon = resolution.summon.expect("Osiris should summon family");
    assert_eq!(
        summon.summoned_kind_ids,
        [
            "demo.actor.horus-the-ancient".to_owned(),
            "demo.actor.isis-the-great-goddess".to_owned(),
        ]
    );
    assert_eq!(summon.duration_turns, 10_000);
}

#[test]
fn p83_gertrude_summons_each_available_sister_once() {
    fn summon(defeated_sister: Option<&str>) -> Vec<String> {
        let mut game = Game::new(277);
        clear_monsters(&mut game);
        game.terrain.fill("demo.terrain.floor".to_owned());
        game.player.position = Position { x: 80, y: 20 };
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.gertrude",
            "demo.actor.gertrude",
            Position { x: 20, y: 20 },
            2_420,
            120,
            100,
            true,
        ));
        if let Some(kind_id) = defeated_sister {
            game.defeated_limited_actor_counts
                .insert(kind_id.to_owned(), 1);
        }
        let ability = game
            .content
            .ability("rfb-legacy.ability.summon-gertrude-sisters-l40-1d1-1")
            .expect("Gertrude sister summon should compile")
            .clone();
        let plan = game
            .monster_ability_target_plan(0, ability.clone(), 1)
            .expect("at least one available sister should produce a summon plan");
        let summon = game
            .resolve_monster_ability_plan(
                0,
                "demo.actor.gertrude",
                &plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .summon
            .expect("Gertrude sister summon should resolve");
        assert_eq!(summon.duration_turns, 10_000);
        assert!(matches!(
            game.monster_ability_target_plan(0, ability, 1),
            Err(MonsterAbilityPlanRejection {
                reason: MonsterAbilityRejectionReasonDto::NoCandidates,
                ..
            })
        ));
        summon.summoned_kind_ids
    }

    let mut both = summon(None);
    both.sort();
    assert_eq!(
        both,
        ["demo.actor.aude".to_owned(), "demo.actor.helga".to_owned()]
    );
    assert_eq!(
        summon(Some("demo.actor.aude")),
        ["demo.actor.helga".to_owned()]
    );
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

#[test]
fn p76_no_air_applies_once_for_forty_ticks() {
    let mut game = Game::new(269);
    clear_monsters(&mut game);
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.player.position = Position { x: 20, y: 20 };
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
        .ability("rfb-legacy.ability.no-air-40")
        .expect("P76 NO_AIR should compile")
        .clone();
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("adjacent living player should be a valid no-air target");
    game.resolve_monster_ability_plan(
        0,
        "demo.actor.vayu-the-embodied-wind",
        &plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    let status = game
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_NO_AIR)
        .expect("NO_AIR should apply its status");
    assert_eq!(status.remaining_ticks, 40);

    let resolutions = game.resolve_monster_player_effects(
        "generated.actor.vayu",
        "demo.actor.vayu-the-embodied-wind",
        &plan.ability,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert!(matches!(
        resolutions.as_slice(),
        [AbilityEffectResolutionDto::Skipped {
            reason: AbilityEffectSkipReasonDto::Ineligible,
            ..
        }]
    ));
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_NO_AIR)
            .expect("recast should keep the original no-air status")
            .remaining_ticks,
        40
    );
}

fn p77_resurrection_machine_game() -> (Game, rfb_content::AbilityDefinition) {
    let mut game = Game::new(277);
    clear_monsters(&mut game);
    game.player.position = Position { x: 80, y: 20 };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.resurrection-machine",
        "demo.actor.the-resurrection-machine",
        Position { x: 20, y: 20 },
        15_488,
        152,
        100,
        true,
    ));
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-dead-unique-l100-1d2")
        .expect("P77 S_DEAD_UNIQ should compile")
        .clone();
    (game, ability)
}

#[test]
fn p77_dead_unique_resurrection_preserves_the_spent_lifetime_slot() {
    let (mut game, ability) = p77_resurrection_machine_game();
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.defeated_limited_actor_counts
        .insert("demo.actor.fangorn".to_owned(), 1);
    let seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            let _ = rng.bounded(2);
            rng.bounded(13) != 0
        })
        .expect("bounded seed search should avoid the Star Blade fallback");
    game.rng = RfbRng::seeded(seed);
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("the Resurrection Machine should always target itself");
    let summon = game
        .resolve_monster_ability_plan(
            0,
            "demo.actor.the-resurrection-machine",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("S_DEAD_UNIQ should summon");
    assert_eq!(summon.summoned_kind_ids[0], "demo.actor.fangorn");

    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("a resurrected dead unique should survive save and restore");
    let resurrected_index = restored
        .entities
        .iter()
        .position(|actor| actor.kind_id == "demo.actor.fangorn")
        .expect("Fangorn should remain resurrected");
    restored
        .resolve_actor_death_without_rewards(
            resurrected_index,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("the resurrected unique should be removable");
    assert_eq!(
        restored
            .defeated_limited_actor_counts
            .get("demo.actor.fangorn"),
        Some(&1),
        "re-killing a resurrection must not spend a second lifetime slot"
    );
}

#[test]
fn p77_dead_unique_summon_disintegrates_radius_five_and_falls_back_to_star_blades() {
    let (mut game, ability) = p77_resurrection_machine_game();
    game.terrain.fill("demo.terrain.wall".to_owned());
    game.rng = RfbRng::seeded(0);
    let plan = game
        .monster_ability_target_plan(0, ability, 1)
        .expect("the Resurrection Machine should always target itself");
    let summon = game
        .resolve_monster_ability_plan(
            0,
            "demo.actor.the-resurrection-machine",
            &plan,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .summon
        .expect("S_DEAD_UNIQ should summon its fallback");

    assert!(!summon.summoned_kind_ids.is_empty());
    assert!(
        summon
            .summoned_kind_ids
            .iter()
            .all(|kind_id| kind_id == "demo.actor.star-blade")
    );
    for position in [
        Position { x: 15, y: 20 },
        Position { x: 20, y: 15 },
        Position { x: 25, y: 20 },
        Position { x: 20, y: 25 },
    ] {
        let index = game.index(position).expect("radius-five cell should exist");
        assert_eq!(game.terrain[index], "demo.terrain.floor");
    }
}

fn place_test_ground_item(game: &mut Game, id: &str, kind_id: &str, position: Position) {
    give_inventory_item(game, id, kind_id);
    game.items
        .iter_mut()
        .find(|item| item.id == id)
        .expect("test ground item should exist")
        .location = ItemLocation::Ground(position);
}

fn apply_test_ground_projection(
    game: &mut Game,
    positions: &[Position],
    damage_type: DamageType,
) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_ground_item_projectile_effects(
        "test.ability.ground-items",
        positions,
        damage_type,
        true,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    events
}

#[test]
fn ground_item_elements_respect_ignore_flags_and_artifact_protection() {
    let mut game = Game::new(211);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    for (id, kind_id) in [
        ("test.arrow", "demo.item.arrow"),
        ("test.elven-cloak", "demo.item.elven-cloak"),
        ("test.artifact", "demo.item.pain"),
        ("test.adamantine", "demo.item.adamantine-bolt"),
        ("test.endurance", "demo.item.arrow"),
    ] {
        place_test_ground_item(&mut game, id, kind_id, position);
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.endurance")
        .expect("Endurance ammunition")
        .affix_ids
        .push("rfb-legacy.affix.endurance".to_owned());

    apply_test_ground_projection(&mut game, &[position], DamageType::Fire);

    assert!(!game.items.iter().any(|item| item.id == "test.arrow"));
    for survivor in [
        "test.elven-cloak",
        "test.artifact",
        "test.adamantine",
        "test.endurance",
    ] {
        assert!(game.items.iter().any(|item| item.id == survivor));
    }
    apply_test_ground_projection(&mut game, &[position], DamageType::Mana);
    assert!(game.items.iter().any(|item| item.id == "test.artifact"));
    assert!(game.items.iter().any(|item| item.id == "test.endurance"));
}

#[test]
fn hell_fire_destroys_only_cursed_ground_items() {
    let mut game = Game::new(223);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    place_test_ground_item(&mut game, "test.clean", "demo.item.arrow", position);
    place_test_ground_item(&mut game, "test.cursed", "demo.item.arrow", position);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.cursed")
        .expect("cursed test item")
        .curse = Some(ItemCurseSeverityDto::Normal);

    apply_test_ground_projection(&mut game, &[position], DamageType::HellFire);

    assert!(game.items.iter().any(|item| item.id == "test.clean"));
    assert!(!game.items.iter().any(|item| item.id == "test.cursed"));
}

#[test]
fn ground_item_destruction_is_ordered_by_position_then_instance_id() {
    let mut game = Game::new(227);
    clear_monsters(&mut game);
    game.items.clear();
    let positions = [
        Position { x: 5, y: 2 },
        Position { x: 5, y: 1 },
        Position { x: 3, y: 1 },
    ];
    for (id, position) in [
        ("test.z", positions[1]),
        ("test.a", positions[1]),
        ("test.m", positions[2]),
        ("test.last", positions[0]),
    ] {
        place_test_ground_item(&mut game, id, "demo.item.arrow", position);
    }

    let events = apply_test_ground_projection(
        &mut game,
        &[positions[0], positions[1], positions[2]],
        DamageType::Fire,
    );
    let destroyed = events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::GroundItemDestroyedByAbility { item_id, .. } => Some(item_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(destroyed, ["test.m", "test.a", "test.z", "test.last"]);
}

#[test]
fn shattered_potion_runs_its_area_program_after_removal() {
    let mut game = Game::new(229);
    clear_monsters(&mut game);
    game.items.clear();
    let position = game.player.position;
    place_test_ground_item(&mut game, "test.venom", "demo.item.venom-draught", position);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();

    let events = apply_test_ground_projection(&mut game, &[position], DamageType::Cold);

    assert!(!game.items.iter().any(|item| item.id == "test.venom"));
    assert_eq!(game.player.hp, hp_before - 3);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityHit { ability_id, .. }
            if ability_id == "demo.item.venom-draught"
    )));
}

#[test]
fn shattered_potion_healing_uses_area_falloff() {
    let mut game = Game::new(233);
    clear_monsters(&mut game);
    let maximum = game.effective_player_max_hp();
    game.player.hp = 1;
    let center = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    let shatter = ItemShatterEffectDefinition {
        radius: 2,
        effect: ItemUseEffectDefinition::Heal { amount: 100 },
    };

    game.resolve_ground_item_shatter_effect(
        "test.item.healing-potion",
        center,
        &shatter,
        true,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );

    assert_eq!(game.player.hp, (1 + 50).min(maximum));
}
