// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::effect::STATUS_KUTAR_EXPAND;

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

const RACE_GOLEM_STONE_SKIN_ABILITY_ID: &str = "rfb.ability.race.golem-stone-skin";

const RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID: &str = "rfb.ability.race.restore-life";

const RACE_SNOTLING_DEVOUR_FLESH_ABILITY_ID: &str = "rfb.ability.race.devour-flesh";

const RACE_BOIT_VOMIT_ABILITY_ID: &str = "rfb.ability.race.vomit";

const RACE_KUTAR_EXPAND_ABILITY_ID: &str = "rfb.ability.race.kutar-expand";

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
fn formal_einheri_halves_shared_healing_but_keeps_full_natural_regeneration() {
    const BERSERK_ABILITY_ID: &str = "rfb.ability.race.berserk";
    let mut game = einheri_game(406);
    clear_monsters(&mut game);
    assert_eq!(game.world_tick, 0);
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(game.player_hold_life_sources(), 1);
    assert_eq!(game.player_regeneration_rate_percent(), 200);
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.ration-of-food" && item.location == ItemLocation::Inventory
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.wooden-torch" && item.location == ItemLocation::Inventory
    }));

    let maximum = game.effective_player_max_hp();
    game.player.hp = maximum - 30;
    let healing = game.apply_player_healing(21);
    assert_eq!(healing.requested, 10);
    assert_eq!(healing.applied, 10);
    assert_eq!(game.player.hp, maximum - 20);

    let regenerated_from = game.player.hp;
    for _ in 0..20 {
        dispatch_next(&mut game, GameCommand::Wait);
        if game.player.hp > regenerated_from {
            break;
        }
    }
    assert!(game.player.hp > regenerated_from);
    assert_eq!(game.player_regeneration_rate_percent(), 200);

    let projected = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == BERSERK_ABILITY_ID)
        .expect("Einheri Berserk should be projected");
    assert_eq!(projected.source, AbilitySourceDto::Race);
    assert_eq!(projected.minimum_level, 1);
    assert_eq!(projected.base_resource_cost, 10);
    assert!(projected.failure_percent < 100);
    assert_eq!(
        projected.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Strength),
    );

    game.debug_set_ability_casts_succeed(true);
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        dispatch_next(
            cast,
            GameCommand::CastAbility {
                ability_id: BERSERK_ABILITY_ID.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        assert!(cast.player_has_status_kind(STATUS_BERSERK));
    }
    assert_eq!(game.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Einheri result should restore");
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut temporary = Game::new_with_build_race_and_name(
        407,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal Human should create");
    temporary.player.hp = temporary.effective_player_max_hp() - 30;
    assert_eq!(temporary.apply_player_healing(21).requested, 21);
    temporary.player.hp = temporary.effective_player_max_hp() - 30;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.einheri-form").status;
    form.granted_race_id = Some("rfb-legacy.race.einheri".to_owned());
    temporary.player.statuses.push(form);
    assert_eq!(temporary.apply_player_healing(21).requested, 10);
    assert_eq!(temporary.player_regeneration_rate_percent(), 200);
    assert_eq!(temporary.player_hold_life_sources(), 1);
    temporary
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(temporary.player_regeneration_rate_percent(), 100);
    assert_eq!(temporary.player_hold_life_sources(), 0);
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
fn formal_kutar_expansion_fixes_saving_throw_and_adds_thirty_five_armor() {
    let mut game = kutar_game(410);
    clear_monsters(&mut game);
    game.progress.level = 19;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_KUTAR_EXPAND_ABILITY_ID)
        .expect("Kutar Expand Horizontally should be projected before it unlocks");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(locked.minimum_level, 20);
    assert_eq!((locked.base_resource_cost, locked.resource_cost), (15, 15));
    assert_eq!(
        locked.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Charisma),
    );
    assert!(!locked.can_cast);

    game.progress.level = 20;
    game.progress.max_level = 20;
    game.refresh_character_skills();
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == RACE_KUTAR_EXPAND_ABILITY_ID)
        .expect("level-twenty Kutar Expand Horizontally");
    assert!(available.can_cast);
    assert!(available.failure_percent > 0);
    assert!(matches!(
        available.effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            status_kind_id,
            duration_ticks: 30,
            duration_dice: 1,
            duration_sides: 20,
            granted_modifiers,
            granted_equipment_bonuses,
            ..
        }] if status_kind_id == STATUS_KUTAR_EXPAND
            && granted_modifiers.defense == 35
            && granted_equipment_bonuses.saving_throw_skill_override == Some(10)
    ));

    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < u64::from(available.failure_percent)
        })
        .expect("Kutar Expand Horizontally should have a failing percentile seed");
    let mut failed = game.clone();
    failed.rng = RfbRng::seeded(failure_seed);
    let failed_hp = failed.player.hp;
    let failed_stats = failed.player_derived_stats();
    let mut failed_events = Vec::new();
    failed
        .resolve_player_ability(
            RACE_KUTAR_EXPAND_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut failed_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("failed Kutar expansion should resolve");
    assert!(matches!(
        failed_events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert_eq!(failed.player.hp, failed_hp - 15);
    assert_eq!(
        failed.player_derived_stats().armor_class.value,
        failed_stats.armor_class.value,
    );
    assert_eq!(
        failed.player_derived_stats().saving_throw_skill.value,
        failed_stats.saving_throw_skill.value,
    );
    assert!(!failed.player_has_status_kind(STATUS_KUTAR_EXPAND));

    game.debug_set_ability_casts_succeed(true);
    let hp_before = game.player.hp;
    let stats_before = game.player_derived_stats();
    assert_ne!(stats_before.saving_throw_skill.value, 10);
    let mut replay = game.clone();
    for cast in [&mut game, &mut replay] {
        cast.resolve_player_ability(
            RACE_KUTAR_EXPAND_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Kutar expansion should resolve");
        let expanded = cast
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_KUTAR_EXPAND)
            .expect("Kutar expansion status");
        assert!((31..=50).contains(&expanded.remaining_ticks));
        assert_eq!(expanded.granted_modifiers.defense, 35);
        assert_eq!(
            expanded
                .granted_equipment_bonuses
                .saving_throw_skill_override,
            Some(10),
        );
        cast.player.statuses.push(StatusInstance {
            kind_id: STATUS_MAGIC_RESISTANCE.to_owned(),
            intensity: 1,
            remaining_ticks: 10,
            source_id: Some("test.kutar-magic-resistance".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let stats = cast.player_derived_stats();
        assert_eq!(stats.armor_class.value, stats_before.armor_class.value + 35);
        assert_eq!(stats.saving_throw_skill.value, 10);
    }
    assert_eq!(game.player.hp, hp_before - 15);
    assert_eq!(game.state_hash(), replay.state_hash());

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Kutar expansion should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.player_derived_stats().armor_class.value,
        stats_before.armor_class.value + 35
    );
    assert_eq!(restored.player_derived_stats().saving_throw_skill.value, 10);
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
fn undead_restore_life_shares_unlock_payment_and_vitality_rules() {
    for (race_id, seed) in [
        ("rfb-legacy.race.zombie", 374),
        ("rfb-legacy.race.skeleton", 382),
    ] {
        let mut game = Game::new_with_build_race_and_name(
            seed,
            "demo.build.warrior",
            race_id,
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("undead character should create");
        clear_monsters(&mut game);
        game.progress.level = 29;
        game.player.hp = game.effective_player_max_hp();
        let projected = |game: &Game| {
            game.snapshot()
                .player
                .abilities
                .into_iter()
                .find(|ability| ability.id == RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID)
                .expect("Restore Life should be projected")
        };
        let locked = projected(&game);
        assert_eq!(
            (
                locked.source,
                locked.governing_attribute,
                locked.minimum_level,
                locked.base_resource_cost,
                locked.resource_cost,
                locked.can_cast
            ),
            (
                AbilitySourceDto::Race,
                Some(rfb_protocol::AttributeKindDto::Wisdom),
                30,
                30,
                30,
                false
            ),
            "{race_id}"
        );
        game.progress.level = 30;
        game.player.hp = game.effective_player_max_hp();
        let available = projected(&game);
        assert!(
            available.can_cast && available.failure_percent >= 70,
            "{race_id}"
        );
        assert!(
            matches!(
                available.effects.as_slice(),
                [AbilityEffectSpecDto::RestoreVitality { life_force: 150 }]
            ),
            "{race_id}"
        );
        game.progress.experience = 500;
        game.progress.maximum_experience = 900;
        game.progress.life_force = 125;

        let mut failed = game.clone();
        let failure_seed = (0..1_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(available.failure_percent))
            .expect("Restore Life should have a failing seed");
        failed.rng = RfbRng::seeded(failure_seed);
        let failed_hp = failed.player.hp;
        let mut events = Vec::new();
        failed
            .resolve_player_ability(
                RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID,
                TargetSelection::SelfTarget,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("failed Restore Life should resolve");
        assert!(
            matches!(events.first(), Some(DomainEvent::AbilityCastFailed { .. })),
            "{race_id}"
        );
        assert_eq!(
            (
                failed.player.hp,
                failed.progress.experience,
                failed.progress.life_force
            ),
            (failed_hp - 30, 500, 125),
            "{race_id}"
        );

        game.debug_set_ability_casts_succeed(true);
        events.clear();
        game.resolve_player_ability(
            RACE_ZOMBIE_RESTORE_LIFE_ABILITY_ID,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Restore Life should resolve");
        let resolution = mutation_cast_resolution(&events);
        assert_eq!(
            (
                resolution.succeeded,
                resolution.hp_paid,
                game.progress.experience,
                game.progress.maximum_experience,
                game.progress.life_force
            ),
            (true, 30, 900, 900, 275),
            "{race_id}"
        );
        assert!(events.iter().any(|event| matches!(event,
            DomainEvent::AbilityEffectsResolved { resolution, .. } if matches!(resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality { experience_before: 500,
                    experience_after: 900, life_force_before: 125, life_force_after: 275, .. }]))),
            "{race_id}");
    }
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

    let level_nine_experience = game.experience_required_for_level(9);
    game.apply_player_experience(level_nine_experience, &mut Vec::new());
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

    game.apply_player_experience(
        game.experience_required_for_level(10) - level_nine_experience,
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

#[test]
fn vampiric_transformation_overlays_race_but_preserves_body_slots() {
    let mut game = prepare_death_caster(17, 35, "demo.ability.death-vampiric-transformation");
    game.progress.experience = game.experience_required_for_level(35);
    game.progress.maximum_experience = game.progress.experience;
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
