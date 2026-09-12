// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::ability_scaling::apply_ability_level_scaling;
use crate::game::ability_scaling::apply_ability_spell_power;
use crate::game::ability_scaling::prorated_level_value;
use crate::game::ability_scaling::scaled_ability_level_value;
use crate::game::ability_scaling::spell_power_value;

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
        ..RolledAffixState::default()
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

#[test]
fn ability_level_curves_preserve_offset_rounding_and_cap_boundaries() {
    let mut scaling = AbilityLevelScalingDefinition {
        effect_index: 0,
        field: AbilityLevelScalingField::DamageBonus,
        multiplier: 2,
        divisor: 3,
        level_offset: 5,
        maximum: None,
        curve: AbilityLevelScalingCurveDefinition::Linear,
        linear_weight: 1,
        quadratic_weight: 0,
        cubic_weight: 0,
    };
    for (level, expected) in [(0, 3), (5, 3), (6, 3), (7, 4), (8, 5), (50, 33)] {
        assert_eq!(
            scaled_ability_level_value(3, &scaling, level),
            expected,
            "linear level {level}"
        );
    }
    scaling.maximum = Some(10);
    assert_eq!(scaled_ability_level_value(3, &scaling, 50), 10);
    assert_eq!(scaled_ability_level_value(15, &scaling, 0), 10);
    scaling.curve = AbilityLevelScalingCurveDefinition::Prorated;
    scaling.multiplier = 300;
    scaling.maximum = None;
    scaling.quadratic_weight = 1;
    scaling.cubic_weight = 1;
    for (level, expected) in [
        (0, 2),
        (1, 4),
        (25, 89),
        (40, 197),
        (49, 290),
        (50, 302),
        (100, 302),
    ] {
        assert_eq!(
            scaled_ability_level_value(2, &scaling, level),
            expected,
            "prorated level {level}"
        );
    }
    for (weights, expected) in [((1, 0, 0), 150), ((0, 1, 0), 75), ((0, 0, 1), 37)] {
        assert_eq!(
            prorated_level_value(300, 25, weights.0, weights.1, weights.2),
            expected,
            "{weights:?}"
        );
    }
}

#[test]
fn ability_scaling_changes_only_the_selected_effect_field() {
    let game = Game::new(0);
    let mut checked_level = BTreeSet::new();
    let mut checked_power = BTreeSet::new();
    for ability in game.content.abilities() {
        let effects = ability.effect.ordered_effects();
        for definition in &ability.level_scaling {
            let source = &effects[usize::from(definition.effect_index)];
            let mut expected = serde_json::to_value(source).expect("effect JSON");
            let kind = expected["type"].as_str().expect("effect type").to_owned();
            if !checked_level.insert((kind.clone(), definition.field)) {
                continue;
            }
            use AbilityLevelScalingField as F;
            let pointer = match definition.field {
                F::DamageDice => "/damageDice",
                F::DamageSides => "/damageSides",
                F::DamageBonus => "/damageBonus",
                F::DeathRayPower
                | F::TeleportAwayPower
                | F::RechargePower
                | F::StatusPower
                | F::GenocidePower => "/power",
                F::IdentifyPower => "/fullIdentifyPower",
                F::Radius if matches!(kind.as_str(), "dimension-door" | "jump") => "/range",
                F::Radius => "/radius",
                F::BeamChancePercent => "/beamChancePercent",
                F::StatusIntensity => "/intensity",
                F::StatusDurationTicks => "/durationTicks",
                F::StatusDurationSides => "/durationSides",
                F::StatusDefense => "/grantedModifiers/defense",
                F::StatusMeleeDamage => "/grantedEquipmentBonuses/meleeDamage",
                F::ControlPower if kind == "insanity-circle" => "/controlPower",
                F::ControlPower => "/power",
                F::SummonMaximumLevel => "/maximumLevel",
                F::MaximumWeight => "/maximumWeightTenthsPound",
                F::BanishDistance => "/maximumDistance",
                F::DeviceMasteryDurationBase => "/durationBase",
                F::DevicePowerBonus => "/devicePowerBonus",
                F::MaximumRange => "/maximumRange",
            };
            *expected
                .pointer_mut(pointer)
                .unwrap_or_else(|| panic!("{kind} {pointer}")) = serde_json::json!(13);
            let mut actual: AbilityEffectDefinition =
                serde_json::from_value(expected.clone()).expect("effect input");
            let scaling = AbilityLevelScalingDefinition {
                multiplier: 3,
                divisor: 2,
                level_offset: 5,
                maximum: None,
                curve: AbilityLevelScalingCurveDefinition::Linear,
                ..definition.clone()
            };
            apply_ability_level_scaling(&mut actual, &scaling, 8);
            *expected.pointer_mut(pointer).unwrap() = serde_json::json!(17);
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                expected,
                "{kind} {:?}",
                definition.field
            );
        }
        for definition in &ability.spell_power_fields {
            let source = &effects[usize::from(definition.effect_index)];
            let mut expected = serde_json::to_value(source).expect("effect JSON");
            let kind = expected["type"].as_str().expect("effect type").to_owned();
            // Mass Sleep has a level-dependent form; its boundary test checks the full pipeline.
            if kind == "mass-sleep-or-stasis"
                || !checked_power.insert((kind.clone(), definition.field))
            {
                continue;
            }
            use AbilitySpellPowerField as F;
            let pointer = match definition.field {
                F::DamageDice => Some("/damageDice"),
                F::DamageSides => Some("/damageSides"),
                F::DamageBonus => Some("/damageBonus"),
                F::HealingAmount => Some("/amount"),
                F::HealingSides => Some("/sides"),
                F::Radius if kind == "dimension-door" => Some("/range"),
                F::Radius => Some("/radius"),
                F::StatusDurationTicks => Some("/durationTicks"),
                F::StatusDurationSides => Some("/durationSides"),
                F::StatusPower | F::GenocidePower | F::TeleportAwayPower | F::RechargePower => {
                    Some("/power")
                }
                F::ControlPower if kind == "insanity-circle" => Some("/controlPower"),
                F::ControlPower => Some("/power"),
                F::SummonMaximumLevel => Some("/maximumLevel"),
                F::IdentifyPower => Some("/fullIdentifyPower"),
                F::MaximumWeight => Some("/maximumWeightTenthsPound"),
                F::BanishDistance => Some("/maximumDistance"),
                F::DeviceMasteryDurationBase => Some("/durationBase"),
                F::ClairvoyanceDurationSides => Some("/telepathyDurationSides"),
                F::MaximumRange => Some("/maximumRange"),
                F::FinalDamage
                | F::FinalHealing
                | F::RandomChoiceRoll
                | F::MaledictionDeathRayPower
                | F::MaledictionFearPower
                | F::InvulnerabilityDuration => None,
            };
            if let Some(pointer) = pointer {
                *expected
                    .pointer_mut(pointer)
                    .unwrap_or_else(|| panic!("{kind} {pointer}")) = serde_json::json!(13);
            }
            let mut actual = serde_json::from_value(expected.clone()).expect("effect input");
            apply_ability_spell_power(&mut actual, *definition, 7);
            if let Some(pointer) = pointer {
                *expected.pointer_mut(pointer).unwrap() = serde_json::json!(20);
            }
            assert_eq!(
                serde_json::to_value(actual).unwrap(),
                expected,
                "{kind} {:?}",
                definition.field
            );
        }
    }
    assert!(checked_level.contains(&(
        "area-damage".to_owned(),
        AbilityLevelScalingField::DamageBonus
    )));
    assert!(
        checked_power.contains(&("heal-dice".to_owned(), AbilitySpellPowerField::HealingSides))
    );
}

#[test]
fn level_and_spell_power_scaling_keep_effect_and_target_ranges_in_sync() {
    let game = Game::new(0);
    for (id, field) in [
        (
            "demo.ability.nature-lightning",
            AbilityLevelScalingField::MaximumRange,
        ),
        (
            "demo.ability.sorcery-dimension-door",
            AbilityLevelScalingField::Radius,
        ),
    ] {
        let mut ability = game.content.ability(id).expect("ranged ability").clone();
        let pointer = if field == AbilityLevelScalingField::MaximumRange {
            "/maximumRange"
        } else {
            "/range"
        };
        let mut effect = serde_json::to_value(&ability.effect).unwrap();
        *effect.pointer_mut(pointer).unwrap() = serde_json::json!(13);
        ability.effect = serde_json::from_value(effect).unwrap();
        ability.level_scaling = vec![AbilityLevelScalingDefinition {
            effect_index: 0,
            field,
            multiplier: 3,
            divisor: 2,
            level_offset: 5,
            maximum: None,
            curve: AbilityLevelScalingCurveDefinition::Linear,
            linear_weight: 1,
            quadratic_weight: 0,
            cubic_weight: 0,
        }];
        ability.spell_power_fields = vec![AbilitySpellPowerDefinition {
            effect_index: 0,
            field: if field == AbilityLevelScalingField::MaximumRange {
                AbilitySpellPowerField::MaximumRange
            } else {
                AbilitySpellPowerField::Radius
            },
        }];
        Game::apply_player_level_scaling(&mut ability, 8);
        assert_eq!(ability.target.range, 17, "{id} level range");
        Game::apply_player_spell_power(&mut ability, 13);
        assert_eq!(ability.target.range, 34, "{id} powered range");
        assert_eq!(
            serde_json::to_value(&ability.effect)
                .unwrap()
                .pointer(pointer),
            Some(&serde_json::json!(34)),
            "{id}"
        );
    }
}
