// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn monster_casting_uses_frequency_viability_and_weighted_selection() {
    let mut selected = BTreeSet::new();
    let mut fallback_count = 0_u32;
    let mut binding_round_trip_checked = false;
    for seed in 0..256_u64 {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        let draw_counter_before = game.rng.draw_counter;
        let mut events = Vec::new();

        let cast = game.resolve_monster_ability(0, &mut events);
        let decision = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::MonsterAbilityDecision { resolution } => Some(resolution),
                _ => None,
            })
            .expect("caster action should expose its decision");
        assert_eq!(decision.frequency_percent, 50);
        assert!((1..=100).contains(&decision.frequency_roll));
        assert_eq!(
            decision.viable_ability_ids,
            [
                "demo.ability.resonant-bolt".to_owned(),
                "demo.ability.echo-binding".to_owned(),
                "demo.ability.echo-burst".to_owned(),
                "demo.ability.echo-lance".to_owned(),
                "demo.ability.echo-fan".to_owned(),
                "demo.ability.echo-quickening".to_owned(),
                "demo.ability.call-discord".to_owned(),
            ]
        );
        assert_eq!(decision.total_weight, 24);

        match decision.selected_ability_id.as_deref() {
            None => {
                fallback_count += 1;
                assert!(!cast);
                assert!(decision.frequency_roll > decision.frequency_percent);
                assert!(decision.selection_roll.is_none());
                assert_eq!(game.rng.draw_counter, draw_counter_before + 1);
            }
            Some(ability_id) => {
                assert!(cast);
                selected.insert(ability_id.to_owned());
                let roll = decision
                    .selection_roll
                    .expect("a successful frequency check should select by weight");
                assert!((1..=decision.total_weight).contains(&roll));
                let cast_resolution = events
                    .iter()
                    .find_map(|event| match event {
                        DomainEvent::MonsterAbilityCast { resolution, .. } => Some(resolution),
                        _ => None,
                    })
                    .expect("selected ability should resolve");
                assert_eq!(cast_resolution.ability_id, ability_id);
                if matches!(
                    ability_id,
                    "demo.ability.echo-quickening" | "demo.ability.call-discord"
                ) {
                    assert_eq!(
                        cast_resolution.target_entity_id,
                        "test.monster.echo-cantor.1"
                    );
                } else {
                    assert_eq!(cast_resolution.target_entity_id, game.player.id);
                }
                if ability_id == "demo.ability.echo-binding" {
                    assert_eq!(cast_resolution.effects.len(), 2);
                    assert!(matches!(
                        cast_resolution.effects[1],
                        AbilityEffectResolutionDto::ApplyStatus { .. }
                    ));
                    let restored = Game::from_save(game.to_save())
                        .expect("monster-applied status should round-trip");
                    assert_eq!(restored.state_hash(), game.state_hash());
                    binding_round_trip_checked = true;
                } else if ability_id == "demo.ability.echo-quickening" {
                    assert_eq!(cast_resolution.effects.len(), 2);
                } else if ability_id != "demo.ability.call-discord" {
                    assert_eq!(cast_resolution.effects.len(), 1);
                }
            }
        }
    }
    assert!(fallback_count > 0);
    assert_eq!(
        selected,
        BTreeSet::from([
            "demo.ability.call-discord".to_owned(),
            "demo.ability.echo-binding".to_owned(),
            "demo.ability.echo-burst".to_owned(),
            "demo.ability.echo-fan".to_owned(),
            "demo.ability.echo-lance".to_owned(),
            "demo.ability.echo-quickening".to_owned(),
            "demo.ability.resonant-bolt".to_owned(),
        ])
    );
    assert!(binding_round_trip_checked);
}

#[test]
fn monster_casting_clean_shot_filter_blocks_allies_and_walls() {
    for blocked_by_actor in [true, false] {
        let mut game = Game::new(1);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        if blocked_by_actor {
            game.entities.push(game.generated_actor(
                "test.monster.blocker.1".to_owned(),
                "demo.actor.ember-mote",
                Position { x: 6, y: 3 },
            ));
        } else {
            replace_terrain(&mut game, Position { x: 6, y: 3 }, "demo.terrain.wall");
        }
        let draw_counter_before = game.rng.draw_counter;
        let mut events = Vec::new();

        game.resolve_monster_ability(0, &mut events);
        let decision = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::MonsterAbilityDecision { resolution } => Some(resolution),
                _ => None,
            })
            .expect("blocked caster should still expose its frequency decision");
        for ability_id in ["demo.ability.resonant-bolt", "demo.ability.echo-binding"] {
            let candidate = decision
                .candidates
                .iter()
                .find(|candidate| candidate.ability_id == ability_id)
                .expect("direct spell should remain observable");
            assert_eq!(candidate.effective_weight, 0);
            assert_eq!(
                candidate.rejection_reason,
                Some(if blocked_by_actor {
                    MonsterAbilityRejectionReasonDto::FriendlyRisk
                } else {
                    MonsterAbilityRejectionReasonDto::Blocked
                })
            );
        }
        assert!(decision.total_weight > 0);
        assert!(game.rng.draw_counter > draw_counter_before);
    }
}

#[test]
fn monster_casting_utility_uses_wounds_status_and_distance_without_rng() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let draws_before = game.rng.draw_counter;
    let healing = game
        .content
        .ability("demo.ability.mending-echo")
        .expect("mending echo should exist")
        .clone();
    assert_eq!(
        game.monster_ability_plan(0, healing.clone(), 4)
            .expect_err("healthy healing should have no utility")
            .reason,
        MonsterAbilityRejectionReasonDto::NoUtility
    );

    game.entities[0].hp = 5;
    let wounded = game
        .monster_ability_plan(0, healing.clone(), 4)
        .expect("more than twenty percent wounds should enable healing");
    assert_eq!(wounded.base_weight, 4);
    assert_eq!(wounded.effective_weight, 8);
    game.entities[0].hp = 1;
    assert_eq!(
        game.monster_ability_plan(0, healing, 4)
            .expect("deep wounds should increase healing weight")
            .effective_weight,
        16
    );

    let quickening = game
        .content
        .ability("demo.ability.echo-quickening")
        .expect("quickening should exist")
        .clone();
    assert!(game.monster_ability_plan(0, quickening.clone(), 2).is_ok());
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 30,
        source_id: Some(quickening.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    assert_eq!(
        game.monster_ability_plan(0, quickening, 2)
            .expect_err("duplicate haste should have no utility")
            .reason,
        MonsterAbilityRejectionReasonDto::NoUtility
    );

    let bolt = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    assert_eq!(
        game.monster_ability_plan(0, bolt.clone(), 3)
            .expect("distant bolt should be viable")
            .effective_weight,
        6
    );
    game.entities[0].position = Position { x: 5, y: 3 };
    assert_eq!(
        game.monster_ability_plan(0, bolt, 3)
            .expect("near bolt should be viable")
            .effective_weight,
        3
    );
    assert_eq!(game.rng.draw_counter, draws_before);
}

#[test]
fn monster_multi_target_plans_reject_secondary_entities() {
    for ability_id in [
        "demo.ability.echo-burst",
        "demo.ability.echo-lance",
        "demo.ability.echo-fan",
    ] {
        let mut game = Game::new(1);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        let ability = game
            .content
            .ability(ability_id)
            .expect("multi-target ability should exist")
            .clone();
        assert!(
            game.monster_ability_plan(0, ability.clone(), 2).is_ok(),
            "{ability_id} should be viable with only its primary target"
        );
        game.entities.push(game.generated_actor(
            "test.monster.secondary.1".to_owned(),
            "demo.actor.ember-mote",
            Position { x: 4, y: 3 },
        ));
        assert_eq!(
            game.monster_ability_plan(0, ability, 2)
                .expect_err("friendly footprint should be rejected")
                .reason,
            MonsterAbilityRejectionReasonDto::FriendlyRisk,
            "{ability_id} should reject a secondary entity in its footprint"
        );
    }
}

#[test]
fn monster_summons_are_hostile_owned_active_and_saveable() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 5, y: 3 },
    ));
    let ability = game
        .content
        .ability("demo.ability.call-discord")
        .expect("hostile summon ability should exist")
        .clone();
    let plan = game
        .monster_ability_plan(0, ability, 2)
        .expect("open cells should permit hostile summoning");
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &plan,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    );
    let summon = resolution
        .summon
        .expect("summon resolution should be explicit");
    assert!(resolution.effects.is_empty());
    assert!(resolution.targets.is_empty());
    assert!(resolution.trace.is_none());
    assert_eq!(summon.owner_id, "test.monster.echo-cantor.1");
    assert_eq!(summon.entity_ids.len(), 2);
    assert_eq!(resolution.affected_positions, summon.positions);
    assert!(
        resolution
            .affected_positions
            .iter()
            .all(|position| changed.contains(position))
    );
    let entities = game.entities_dto();
    for entity_id in &summon.entity_ids {
        let entity = entities
            .iter()
            .find(|entity| &entity.id == entity_id)
            .expect("summoned entity should be projected");
        assert_eq!(entity.faction, EntityFactionDto::Hostile);
        assert_eq!(
            entity
                .summon
                .as_ref()
                .expect("summon identity should be projected")
                .owner_id,
            "test.monster.echo-cantor.1"
        );
    }

    let hp_before = game.player.hp;
    let summon_index = game
        .entities
        .iter()
        .position(|entity| entity.id == summon.entity_ids[0])
        .expect("first summon should remain present");
    game.resolve_monster_action(
        summon_index,
        &mut events,
        &mut changed,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated hostile summon action should resolve");
    assert!(game.player.hp < hp_before);

    let restored =
        Game::from_save(game.to_save()).expect("hostile summon should round-trip through save");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn monster_spells_target_nearby_player_summons_and_score_enemy_footprints() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut companion = game.generated_actor(
        "test.summon.echo-companion.1".to_owned(),
        "demo.actor.echo-companion",
        Position { x: 7, y: 3 },
    );
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns: 5,
    });
    game.entities.push(companion);

    let bolt = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    let bolt_plan = game
        .monster_ability_plan(0, bolt, 3)
        .expect("nearby companion should be a legal target");
    assert_eq!(
        monster_plan_target(&bolt_plan.target).map(MonsterHostileTarget::entity_id),
        Some("test.summon.echo-companion.1")
    );
    assert_eq!(bolt_plan.enemy_target_count, 1);

    game.entities[1].position = Position { x: 3, y: 4 };
    let burst = game
        .content
        .ability("demo.ability.echo-burst")
        .expect("burst should exist")
        .clone();
    let burst_plan = game
        .monster_ability_plan(0, burst, 2)
        .expect("a player and nearby companion should both be legal enemies");
    assert_eq!(burst_plan.enemy_target_count, 2);
    assert_eq!(burst_plan.friendly_risk_count, 0);
    assert_eq!(burst_plan.effective_weight, 8);
}

#[test]
fn monster_area_damage_hits_every_player_aligned_target_and_removes_slain_summons() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut companion = game.generated_actor(
        "test.summon.echo-companion.1".to_owned(),
        "demo.actor.echo-companion",
        Position { x: 3, y: 4 },
    );
    companion.hp = 1;
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns: 5,
    });
    game.entities.push(companion);
    let ability = game
        .content
        .ability("demo.ability.echo-burst")
        .expect("burst should exist")
        .clone();
    let plan = game
        .monster_ability_plan(0, ability, 2)
        .expect("burst should cover both enemies");
    let player_hp_before = game.player.hp;
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &plan,
        &mut Vec::new(),
        &mut changed,
        &mut removed,
    );
    assert_eq!(resolution.targets.len(), 2);
    assert!(game.player.hp < player_hp_before);
    assert_eq!(removed, ["test.summon.echo-companion.1"]);
    assert!(
        game.entities
            .iter()
            .all(|entity| entity.id != "test.summon.echo-companion.1")
    );
    assert!(changed.contains(&Position { x: 3, y: 4 }));
}

#[test]
fn smart_caster_learns_only_observed_player_resistance_and_round_trips() {
    let mut game = Game::new(3);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    game.player
        .resistances
        .set(DamageType::Electricity, ResistanceLevel::Resistant);
    let ability = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    let before = game
        .monster_ability_plan(0, ability.clone(), 3)
        .expect("unknown resistance must not affect the first decision");
    assert_eq!(before.effective_weight, 6);
    assert!(game.entities[0].observed_player_resistances.is_empty());

    game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &before,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(
        game.entities[0]
            .observed_player_resistances
            .get(&DamageType::Electricity),
        Some(&ResistanceLevel::Resistant)
    );
    let after = game
        .monster_ability_plan(0, ability, 3)
        .expect("observed resistance should downweight rather than forbid the bolt");
    assert_eq!(after.effective_weight, 3);

    let restored = Game::from_save(game.to_save()).expect("resistance memory should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.entities[0].observed_player_resistances,
        game.entities[0].observed_player_resistances
    );
}

#[test]
fn caster_keeps_distance_and_flees_when_wounded_without_extra_rng() {
    let mut game = Game::new(4);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 5, y: 3 },
    ));
    game.entities[0].casting_cooldown_remaining = 1;
    let draws_before = game.rng.draw_counter;
    let mut events = Vec::new();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated tactical action should resolve");
    assert_eq!(game.entities[0].position, Position { x: 6, y: 2 });
    assert_eq!(game.rng.draw_counter, draws_before);
    assert!(
        events
            .iter()
            .any(|event| { matches!(event, DomainEvent::MonsterKeptDistance { .. }) })
    );

    game.entities[0].position = Position { x: 5, y: 3 };
    game.entities[0].hp = 2;
    game.entities[0].casting_cooldown_remaining = 1;
    events.clear();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated tactical action should resolve");
    assert_eq!(game.entities[0].position, Position { x: 6, y: 2 });
    assert!(
        events
            .iter()
            .any(|event| { matches!(event, DomainEvent::MonsterFled { .. }) })
    );
}

#[test]
fn monster_casting_cooldown_uses_inverse_frequency_without_rng() {
    assert_eq!(monster_casting_cooldown(50), 2);
    assert_eq!(monster_casting_cooldown(25), 4);
    assert_eq!(monster_casting_cooldown(30), 4);
    let seed = (0..1_000_u64)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            game.entities.push(game.generated_actor(
                "test.monster.echo-cantor.1".to_owned(),
                "demo.actor.echo-cantor",
                Position { x: 8, y: 3 },
            ));
            game.resolve_monster_ability(0, &mut Vec::new())
        })
        .expect("a deterministic seed should pass the frequency check");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));

    assert!(game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.entities[0].casting_cooldown_remaining, 2);
    let draws_after_cast = game.rng.draw_counter;
    for expected_remaining in [1, 0] {
        let mut events = Vec::new();
        assert!(!game.resolve_monster_ability(0, &mut events));
        assert!(events.is_empty());
        assert_eq!(
            game.entities[0].casting_cooldown_remaining,
            expected_remaining
        );
        assert_eq!(game.rng.draw_counter, draws_after_cast);
    }

    game.resolve_monster_ability(0, &mut Vec::new());
    assert!(game.rng.draw_counter > draws_after_cast);
    let restored =
        Game::from_save(game.to_save()).expect("monster cooldown should round-trip through save");
    assert_eq!(
        restored.entities[0].casting_cooldown_remaining,
        game.entities[0].casting_cooldown_remaining
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn lethal_monster_sequence_skips_later_status_without_extra_rng() {
    let seed = (0..1_000_u64)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            game.entities.push(game.generated_actor(
                "test.monster.echo-cantor.1".to_owned(),
                "demo.actor.echo-cantor",
                Position { x: 8, y: 3 },
            ));
            let mut events = Vec::new();
            game.resolve_monster_ability(0, &mut events);
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::MonsterAbilityCast { resolution, .. }
                        if resolution.ability_id == "demo.ability.echo-binding"
                )
            })
        })
        .expect("a deterministic seed should select echo binding");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    game.player.hp = 0;
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut events = Vec::new();

    assert!(game.resolve_monster_ability(0, &mut events));
    let resolution = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterAbilityCast { resolution, .. } => Some(resolution),
            _ => None,
        })
        .expect("binding should resolve");
    assert_eq!(resolution.ability_id, "demo.ability.echo-binding");
    assert!(matches!(
        resolution.effects[0],
        AbilityEffectResolutionDto::Damage { .. }
    ));
    assert_eq!(
        resolution.effects[1],
        AbilityEffectResolutionDto::Skipped {
            effect_index: 1,
            reason: AbilityEffectSkipReasonDto::TargetDead,
        }
    );
    assert!(game.player_is_dead());
    assert!(
        !game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW)
    );
}
