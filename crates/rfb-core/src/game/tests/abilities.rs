// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn scholar_studies_and_casts_an_ability_book_spell_deterministically() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let book_item_id = ability_book_item_id(&game);
    let initial_draws = game.rng_draw_counter();
    let initial = game.snapshot();
    assert_eq!(initial.player.resources[0].current, 21);
    assert_eq!(initial.player.resources[0].maximum, 21);
    let resonant_bolt = initial
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("scholar should expose resonant bolt");
    assert!(!resonant_bolt.learned);
    assert!(resonant_bolt.can_study);

    let study = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        study
            .events
            .iter()
            .any(|event| event.kind == "ability.studied")
    );
    assert_eq!(game.rng_draw_counter(), initial_draws);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.resonant-bolt")
            .is_some_and(|ability| ability.learned)
    );

    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&cast);
    assert_eq!(resolution.resource_before, 21);
    assert_eq!(resolution.base_resource_cost, 3);
    assert_eq!(resolution.resource_cost, 5);
    assert_eq!(resolution.resource_after, 16);
    assert_eq!(resolution.failure_percent, 20);
    assert_eq!(resolution.percentile_roll, 32);
    assert!(resolution.succeeded);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 128);
    assert_eq!(resolution.cast_count, 1);
    assert_eq!(resolution.fail_count, 0);
    assert!(cast.events.iter().any(|event| event.kind == "ability.hit"));
    assert!(cast.events.iter().any(|event| event.kind == "ability.slay"));
    assert!(
        !game
            .entities
            .iter()
            .any(|entity| entity.id == "demo.monster.ember-mote.1")
    );
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.resources[0].current, 16);
    assert_eq!(snapshot.player.resources[0].maximum, 23);
    assert_eq!(snapshot.player.progress.level, 2);

    let restored = Game::from_save(game.to_save()).expect("ability state should reload");
    assert_eq!(restored.snapshot(), snapshot);
}

#[test]
fn area_damage_uses_rfb_targeted_ball_path_falloff_and_ordering() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let ability_id = "demo.ability.echo-burst";
    game.learned_abilities.insert(ability_id.to_owned());
    for position in [
        Position { x: 4, y: 3 },
        Position { x: 5, y: 3 },
        Position { x: 6, y: 3 },
    ] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 6, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 4, y: 3 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );

    let area = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityAreaDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful area spell should expose its resolved footprint");
    assert_eq!(area.center, Position { x: 6, y: 3 });
    assert_eq!(area.radius, 2);
    assert_eq!(area.target_count, 2);
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    assert!(area.affected_positions.windows(2).all(|positions| {
        let left = positions[0];
        let right = positions[1];
        (rfb_distance(area.center, left), left.y, left.x)
            <= (rfb_distance(area.center, right), right.y, right.x)
    }));

    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let center_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("center hit should expose damage"),
    };
    let edge_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("edge hit should expose damage"),
    };
    assert_eq!(center_damage, area.base_raw_damage);
    assert_eq!(edge_damage, rfb_area_damage(area.base_raw_damage, 2));
}

#[test]
fn area_damage_respects_walls_and_invalid_targets_are_zero_rng() {
    let ability_id = "demo.ability.echo-burst";
    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for position in [
        Position { x: 4, y: 3 },
        Position { x: 5, y: 3 },
        Position { x: 6, y: 3 },
        Position { x: 6, y: 5 },
    ] {
        replace_terrain(&mut blocked, position, "demo.terrain.floor");
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 4 }, "demo.terrain.wall");
    let ember = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 6, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 6, y: 5 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let guardian_hp = guardian.hp;

    let update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let area = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityAreaDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("area outcome should exist");
    assert_eq!(area.target_count, 1);
    assert!(!area.affected_positions.contains(&Position { x: 6, y: 5 }));
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
    );

    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let progress_before = invalid.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: Position { x: 19, y: 19 },
            },
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert_eq!(invalid.ability_progress[ability_id], progress_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert!(!rejected.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
        )
    }));
}

#[test]
fn beam_damage_passes_through_actors_with_one_roll_and_stops_at_walls() {
    let ability_id = "demo.ability.echo-lance";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.learned_abilities.insert(ability_id.to_owned());
    for x in 4..=9 {
        replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 5, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 3 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let beam = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful beam should expose its line");
    assert_eq!(beam.target_count, 2);
    assert_eq!(beam.affected_positions.len(), 6);
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let first_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("beam hit should expose damage"),
    };
    let second_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("beam hit should expose damage"),
    };
    assert_eq!(first_damage, beam.base_raw_damage);
    assert_eq!(second_damage, beam.base_raw_damage);

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for x in 4..=9 {
        replace_terrain(&mut blocked, Position { x, y: 3 }, "demo.terrain.floor");
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 3 }, "demo.terrain.wall");
    for entity in &mut blocked.entities {
        entity.energy_need = 1_000;
    }
    blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 5, y: 3 };
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 3 };
    let guardian_hp = guardian.hp;
    let blocked_update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let blocked_beam = blocked_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("blocked beam should expose its line");
    assert_eq!(blocked_beam.target_count, 1);
    assert_eq!(
        blocked_beam.affected_positions,
        vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }]
    );
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
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
fn category_summon_picks_tagged_kinds_and_rejects_empty_categories() {
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
        "generated.actor.binder-test",
        "demo.actor.mote-binder",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        9,
        100,
        100,
        true,
    ));

    let elemental_kinds = [
        "demo.actor.acid-seep",
        "demo.actor.ember-mote",
        "demo.actor.frost-wisp",
        "demo.actor.storm-spark",
        "demo.actor.venom-spore",
    ];
    let mut saw_empty_rejection = false;
    for _ in 0..60 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityDecision { resolution }) =
                event.outcome.as_ref()
            {
                for candidate in &resolution.candidates {
                    if candidate.ability_id == "demo.ability.cantor-call"
                        && candidate.rejection_reason
                            == Some(MonsterAbilityRejectionReasonDto::NoCandidates)
                    {
                        saw_empty_rejection = true;
                    }
                }
            }
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                assert_eq!(resolution.ability_id, "demo.ability.mote-call");
                let summon = resolution
                    .summon
                    .as_ref()
                    .expect("category summon should expose its resolution");
                assert_eq!(summon.actor_kind_id, "elemental");
                assert!((1..=2).contains(&summon.entity_ids.len()));
                assert_eq!(summon.summoned_kind_ids.len(), summon.entity_ids.len());
                for kind_id in &summon.summoned_kind_ids {
                    assert!(elemental_kinds.contains(&kind_id.as_str()));
                }
                for entity_id in &summon.entity_ids {
                    let entity = game
                        .entities
                        .iter()
                        .find(|entity| &entity.id == entity_id)
                        .expect("summoned entity should exist");
                    assert!(entity.summon.is_some());
                }
                assert!(
                    saw_empty_rejection,
                    "cantor-call must have been rejected with no-candidates in the same decision"
                );
                return;
            }
        }
        if game.player_is_dead() {
            break;
        }
    }
    panic!("mote binder should summon within 60 turns");
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
fn targeted_beam_continues_through_position_and_entity_targets() {
    let ability_id = "demo.ability.echo-lance";
    let expected_path = vec![
        Position { x: 4, y: 3 },
        Position { x: 5, y: 4 },
        Position { x: 6, y: 4 },
        Position { x: 7, y: 4 },
        Position { x: 8, y: 5 },
        Position { x: 9, y: 5 },
    ];

    for target in [
        TargetSelection::Position {
            position: Position { x: 6, y: 4 },
        },
        TargetSelection::Entity {
            entity_id: "demo.monster.ember-mote.1".to_owned(),
        },
    ] {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.learned_abilities.insert(ability_id.to_owned());
        for position in &expected_path {
            replace_terrain(&mut game, *position, "demo.terrain.floor");
        }
        let ember = game
            .entities
            .iter_mut()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .expect("ember mote should exist");
        ember.position = Position { x: 6, y: 4 };
        ember.hp = 100;
        ember.energy_need = 1_000;
        let guardian = game
            .entities
            .iter_mut()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .expect("entrance guardian should exist");
        guardian.position = Position { x: 8, y: 5 };
        guardian.hp = 100;
        guardian.energy_need = 1_000;

        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target,
            },
        );
        let beam = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("targeted beam should expose its extended line");
        assert_eq!(beam.affected_positions, expected_path);
        assert_eq!(beam.target_count, 2);
        let hit_targets = update
            .events
            .iter()
            .filter(|event| event.kind == "ability.hit")
            .map(|event| event.args["target"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            hit_targets,
            vec!["demo.actor.ember-mote", "demo.actor.resonant-warden"]
        );
    }
}

#[test]
fn beam_self_target_is_zero_rng_and_empty_beam_still_rolls_once() {
    let ability_id = "demo.ability.echo-lance";
    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );

    let mut empty =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    empty.learned_abilities.insert(ability_id.to_owned());
    clear_monsters(&mut empty);
    for x in 4..=9 {
        replace_terrain(&mut empty, Position { x, y: 3 }, "demo.terrain.floor");
    }
    replace_terrain(&mut empty, Position { x: 4, y: 3 }, "demo.terrain.wall");
    let draws_before = empty.rng_draw_counter();
    let update = dispatch_next(
        &mut empty,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let beam = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty beam should still resolve");
    assert_eq!(beam.target_count, 0);
    assert!(beam.affected_positions.is_empty());
    assert_eq!(empty.rng_draw_counter(), draws_before + 3);
}

#[test]
fn cone_damage_widens_with_lateral_falloff_and_stable_order() {
    let ability_id = "demo.ability.echo-fan";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.learned_abilities.insert(ability_id.to_owned());
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    for entity in &mut game.entities {
        entity.energy_need = 1_000;
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 4, y: 3 };
    ember.hp = 100;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 2 };
    guardian.hp = 100;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cone = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful cone should expose its footprint");
    assert_eq!(cone.radius, 2);
    assert_eq!(cone.target_count, 2);
    assert_eq!(cone.affected_positions.len(), 14);
    assert_eq!(cone.affected_positions[0], Position { x: 4, y: 3 });
    assert!(cone.affected_positions.contains(&Position { x: 9, y: 1 }));
    assert!(cone.affected_positions.contains(&Position { x: 9, y: 5 }));
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let center_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("cone center hit should expose damage"),
    };
    let edge_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("cone edge hit should expose damage"),
    };
    assert_eq!(center_damage, cone.base_raw_damage);
    assert_eq!(edge_damage, rfb_area_damage(cone.base_raw_damage, 1));

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut blocked, Position { x, y }, "demo.terrain.floor");
        }
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 3 }, "demo.terrain.wall");
    for entity in &mut blocked.entities {
        entity.energy_need = 1_000;
    }
    blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 5, y: 3 };
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 2 };
    let guardian_hp = guardian.hp;
    let blocked_update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let blocked_cone = blocked_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("blocked cone should expose its footprint");
    assert_eq!(blocked_cone.target_count, 1);
    assert_eq!(
        blocked_cone.affected_positions,
        vec![
            Position { x: 4, y: 3 },
            Position { x: 5, y: 3 },
            Position { x: 5, y: 2 },
            Position { x: 5, y: 4 },
            Position { x: 5, y: 1 },
            Position { x: 5, y: 5 },
        ]
    );
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
    );
}

#[test]
fn cone_damage_is_symmetric_across_all_eight_directions() {
    let directions = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];
    let expected_layer_counts = [1_usize, 1, 1, 3, 3, 5];
    for direction in directions {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.learned_abilities
            .insert("demo.ability.echo-fan".to_owned());
        clear_monsters(&mut game);
        game.player.position = Position { x: 10, y: 10 };
        for y in 0..20 {
            for x in 0..20 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: "demo.ability.echo-fan".to_owned(),
                target: TargetSelection::Direction { direction },
            },
        );
        let cone = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("cone outcome should exist");
        assert_eq!(
            cone.affected_positions.len(),
            expected_layer_counts.iter().sum::<usize>()
        );
        let (dx, dy) = direction.delta();
        let mut layer_counts = [0_usize; 6];
        let mut previous_key = None;
        for position in &cone.affected_positions {
            let offset_x = position.x - game.player.position.x;
            let offset_y = position.y - game.player.position.y;
            let layer = offset_x.abs().max(offset_y.abs());
            let lateral = (offset_x * dy - offset_y * dx).abs();
            assert!((1..=6).contains(&layer));
            assert!(offset_x * dx + offset_y * dy > 0);
            layer_counts[usize::try_from(layer - 1).expect("layer index should fit")] += 1;
            let key = (layer, lateral, position.y, position.x);
            assert!(previous_key.is_none_or(|previous| previous <= key));
            previous_key = Some(key);
        }
        assert_eq!(layer_counts, expected_layer_counts);
    }
}

#[test]
fn cone_invalid_mode_is_zero_rng_and_empty_cone_still_rolls_once() {
    let ability_id = "demo.ability.echo-fan";
    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: Position { x: 8, y: 3 },
            },
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );

    let mut empty =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    empty.learned_abilities.insert(ability_id.to_owned());
    clear_monsters(&mut empty);
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut empty, Position { x, y }, "demo.terrain.floor");
        }
    }
    replace_terrain(&mut empty, Position { x: 4, y: 3 }, "demo.terrain.wall");
    let draws_before = empty.rng_draw_counter();
    let update = dispatch_next(
        &mut empty,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cone = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty cone should still resolve");
    assert_eq!(cone.target_count, 0);
    assert!(cone.affected_positions.is_empty());
    assert_eq!(empty.rng_draw_counter(), draws_before + 3);
}

#[test]
fn teleport_moves_to_an_exact_destination_and_round_trips() {
    let ability_id = "demo.ability.echo-step";
    let destination = Position { x: 6, y: 3 };
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    let origin = game.player.position;
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: destination,
            },
        },
    );

    let cast = ability_cast_resolution(&update);
    assert!(cast.succeeded);
    let teleport = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTeleport { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful teleport should expose its relocation");
    assert_eq!(teleport.from, origin);
    assert_eq!(teleport.to, destination);
    assert_eq!(game.player.position, destination);
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - cast.resource_cost
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == origin)
    );
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == destination)
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .is_some_and(|ability| ability.teleport)
    );

    let restored = Game::from_save(game.to_save()).expect("teleport state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn teleport_rejects_blocked_occupied_and_invalid_destinations_before_rng() {
    let ability_id = "demo.ability.echo-step";

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut blocked);
    blocked.learned_abilities.insert(ability_id.to_owned());
    replace_terrain(
        &mut blocked,
        Position { x: 5, y: 3 },
        "demo.terrain.resonance-water-deep",
    );
    assert!(blocked.is_visible(Position { x: 6, y: 3 }));
    assert_teleport_target_rejected(
        &mut blocked,
        ability_id,
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );

    let mut occupied =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    occupied.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut occupied.entities {
        entity.energy_need = 1_000;
    }
    occupied
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 6, y: 3 };
    assert_teleport_target_rejected(
        &mut occupied,
        ability_id,
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );

    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut invalid);
    invalid.learned_abilities.insert(ability_id.to_owned());
    for target in [
        TargetSelection::Position {
            position: invalid.player.position,
        },
        TargetSelection::Position {
            position: Position { x: 10, y: 3 },
        },
        TargetSelection::Direction {
            direction: Direction::East,
        },
    ] {
        assert_teleport_target_rejected(&mut invalid, ability_id, target);
    }
}

#[test]
fn teleport_uses_normal_arrival_trap_semantics() {
    let ability_id = "demo.ability.echo-step";
    let destination = Position { x: 4, y: 3 };
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    replace_terrain(&mut game, destination, "demo.terrain.trap-echo-snare");
    let hp_before = game.player.hp;

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: destination,
            },
        },
    );

    assert_eq!(game.player.position, destination);
    assert_eq!(game.player.hp, hp_before - 2);
    let kinds = update
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    let teleport_index = kinds
        .iter()
        .position(|kind| *kind == "ability.teleport")
        .expect("teleport event should exist");
    let trap_index = kinds
        .iter()
        .position(|kind| *kind == "terrain.trap-triggered")
        .expect("landing trap should trigger");
    assert!(teleport_index < trap_index);
}

#[test]
fn summon_is_deterministic_owned_persistent_and_expires_by_turn() {
    let ability_id = "demo.ability.echo-companion";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .unwrap()
            .proficiency = 1600;
        for entity in &mut candidate.entities {
            entity.energy_need = 1_000;
        }
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if update
            .events
            .iter()
            .any(|event| event.kind == "ability.summon")
        {
            selected = Some((candidate, update));
            break;
        }
    }
    let (mut game, update) = selected.expect("a deterministic seed should cast successfully");
    let summon = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilitySummon { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("summon outcome should be present");
    assert_eq!(summon.entity_ids.len(), 2);
    assert_eq!(summon.positions.len(), 2);
    assert!(summon.positions[0] != summon.positions[1]);
    assert!(game.entities.iter().all(|entity| {
        entity.summon.as_ref().is_none_or(|identity| {
            identity.owner_id == game.player.id
                && identity.source_ability_id == ability_id
                && identity.remaining_turns == 4
        })
    }));
    assert!(
        game.snapshot()
            .entities
            .iter()
            .filter(|entity| entity.faction == EntityFactionDto::Player)
            .all(|entity| entity.summon.is_some())
    );

    let restored = Game::from_save(game.to_save()).expect("summon save should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot().entities, game.snapshot().entities);

    for sequence in 0..4 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        if sequence < 3 {
            assert_eq!(
                update
                    .entities
                    .iter()
                    .filter(|entity| entity.faction == EntityFactionDto::Player)
                    .count(),
                2
            );
        }
    }
    assert!(
        game.snapshot()
            .entities
            .iter()
            .all(|entity| entity.faction == EntityFactionDto::Hostile)
    );
}

#[test]
fn summon_space_rejection_is_atomic_before_mana_and_rng() {
    let ability_id = "demo.ability.echo-companion";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    let origin = game.player.position;
    for y in origin.y - 2..=origin.y + 2 {
        for x in origin.x - 2..=origin.x + 2 {
            let position = Position { x, y };
            if position != origin && game.index(position).is_some() {
                replace_terrain(&mut game, position, "demo.terrain.wall");
            }
        }
    }
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
    assert!(game.entities.is_empty());
}

#[test]
fn summon_failure_costs_mana_but_does_not_create_entities() {
    let ability_id = "demo.ability.echo-companion";
    for seed in 0..128 {
        let mut game =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut game);
        game.learned_abilities.insert(ability_id.to_owned());
        for entity in &mut game.entities {
            entity.energy_need = 1_000;
        }
        let mana_before = game.resources["demo.resource.mana"].current;
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        let Some(cast) = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution),
                _ => None,
            })
        else {
            continue;
        };
        if !cast.succeeded {
            assert!(game.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(game.rng_draw_counter(), draws_before + 1);
            assert!(game.entities.is_empty());
            return;
        }
    }
    panic!("a failure seed should exist in the deterministic search range");
}

#[test]
fn detect_persistent_filters_category_visibility_and_round_trips() {
    let ability_id = "demo.ability.echo-sight";
    let visible_rune = Position { x: 4, y: 2 };
    let visible_door = Position { x: 4, y: 4 };
    let blocked_rune = Position { x: 6, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("detect ability progress should exist")
            .proficiency = 1600;
        for y in 1..=5 {
            for x in 1..=8 {
                replace_terrain(&mut candidate, Position { x, y }, "demo.terrain.floor");
            }
        }
        replace_terrain(
            &mut candidate,
            visible_rune,
            "demo.terrain.echo-rune-hidden",
        );
        replace_terrain(&mut candidate, visible_door, "demo.terrain.door-secret");
        replace_terrain(&mut candidate, Position { x: 5, y: 3 }, "demo.terrain.wall");
        replace_terrain(
            &mut candidate,
            blocked_rune,
            "demo.terrain.echo-rune-hidden",
        );
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic detect success seed should exist");
    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful detection should expose its result");
    assert_eq!(detection.category, "hidden");
    assert_eq!(detection.radius, 6);
    assert!(detection.persistent);
    assert_eq!(
        detection.detected_positions,
        vec![visible_rune, visible_door]
    );
    assert!(game.revealed_terrain.contains(&visible_rune));
    assert!(game.revealed_terrain.contains(&visible_door));
    assert!(!game.revealed_terrain.contains(&blocked_rune));
    assert_eq!(
        game.known_terrain_at(visible_rune),
        "demo.terrain.echo-rune-hidden"
    );
    assert_eq!(
        game.known_terrain_at(visible_door),
        "demo.terrain.door-secret"
    );
    assert_eq!(game.known_terrain_at(blocked_rune), "demo.terrain.wall");
    assert_eq!(
        update
            .changed_cells
            .iter()
            .map(|cell| cell.position)
            .collect::<Vec<_>>(),
        vec![visible_rune, visible_door]
    );

    let restored = Game::from_save(game.to_save()).expect("detected terrain should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn detect_transient_empty_and_invalid_targets_preserve_knowledge_boundaries() {
    let ability_id = "demo.ability.echo-pulse";
    let rune = Position { x: 4, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("detect ability progress should exist")
            .proficiency = 1600;
        replace_terrain(&mut candidate, rune, "demo.terrain.echo-rune-hidden");
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (mut game, update) = selected.expect("a deterministic detect success seed should exist");
    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("transient detection should expose its result");
    assert_eq!(detection.detected_positions, vec![rune]);
    assert!(!detection.persistent);
    assert!(!game.revealed_terrain.contains(&rune));
    assert_eq!(game.known_terrain_at(rune), "demo.terrain.wall");
    assert!(update.changed_cells.is_empty());

    replace_terrain(&mut game, rune, "demo.terrain.floor");
    let empty = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let empty_detection = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty detection should still resolve");
    assert!(empty_detection.detected_positions.is_empty());

    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
}

#[test]
fn terrain_transform_digging_is_stable_atomic_and_round_trips() {
    let ability_id = "demo.ability.echo-delving";
    let center = Position { x: 5, y: 3 };
    let transformed = vec![center, Position { x: 5, y: 2 }, Position { x: 4, y: 4 }];
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        for (position, terrain_id) in [
            (center, "demo.terrain.wall"),
            (Position { x: 5, y: 2 }, "demo.terrain.echo-rubble"),
            (Position { x: 6, y: 3 }, "demo.terrain.resonance-vein"),
            (Position { x: 4, y: 4 }, "demo.terrain.resonance-ruin"),
            (Position { x: 5, y: 4 }, "demo.terrain.floor"),
        ] {
            replace_terrain(&mut candidate, position, terrain_id);
        }
        candidate.revealed_terrain.insert(Position { x: 5, y: 2 });
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position { position: center },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) =
        selected.expect("a deterministic terrain transformation success should exist");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful terrain transformation should expose its result");
    assert_eq!(resolution.center, center);
    assert_eq!(resolution.radius, 1);
    assert_eq!(resolution.target_terrain_id, "demo.terrain.floor");
    assert_eq!(resolution.transformed_positions, transformed);
    for position in &transformed {
        assert_eq!(game.terrain_at(*position), "demo.terrain.floor");
    }
    assert_eq!(
        game.terrain_at(Position { x: 5, y: 4 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 6, y: 3 }),
        "demo.terrain.resonance-vein"
    );
    assert!(!game.revealed_terrain.contains(&Position { x: 5, y: 2 }));
    assert_eq!(
        update
            .changed_cells
            .iter()
            .map(|cell| cell.position)
            .collect::<BTreeSet<_>>(),
        transformed.iter().copied().collect()
    );

    let restored = Game::from_save(game.to_save()).expect("transformed terrain should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn terrain_transform_creation_filters_occupied_connections_and_borders() {
    let ability_id = "demo.ability.echo-rampart";
    let center = Position { x: 3, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        candidate.entities.truncate(1);
        candidate.entities[0].position = Position { x: 2, y: 3 };
        candidate.entities[0].energy_need = i32::MAX / 2;
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        for position in [
            Position { x: 2, y: 2 },
            Position { x: 3, y: 2 },
            Position { x: 4, y: 2 },
            Position { x: 2, y: 3 },
            Position { x: 3, y: 3 },
            Position { x: 4, y: 3 },
            Position { x: 2, y: 4 },
            Position { x: 4, y: 4 },
        ] {
            replace_terrain(&mut candidate, position, "demo.terrain.floor");
        }
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position { position: center },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic terrain creation success should exist");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful terrain creation should expose its result");
    let expected = vec![
        Position { x: 2, y: 2 },
        Position { x: 3, y: 2 },
        Position { x: 4, y: 2 },
        Position { x: 2, y: 4 },
        Position { x: 4, y: 4 },
    ];
    assert_eq!(resolution.transformed_positions, expected);
    for position in &expected {
        assert_eq!(game.terrain_at(*position), "demo.terrain.echo-rubble");
    }
    assert_eq!(game.terrain_at(center), "demo.terrain.floor");
    assert_eq!(
        game.terrain_at(Position { x: 2, y: 3 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 4, y: 3 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 3, y: 4 }),
        "demo.terrain.stairs-down"
    );

    let ability = game
        .content
        .ability(ability_id)
        .expect("terrain creation ability should exist");
    assert!(
        game.terrain_transform_positions(
            ability,
            center,
            &["demo.terrain.stairs-down".to_owned()],
            "demo.terrain.echo-rubble",
            1,
        )
        .expect("the current cell should be a valid target")
        .is_empty()
    );
    let border_game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let border_ability = border_game
        .content
        .ability(ability_id)
        .expect("terrain creation ability should exist");
    assert!(
        border_game
            .terrain_transform_positions(
                border_ability,
                Position { x: 1, y: 1 },
                &["demo.terrain.wall".to_owned()],
                "demo.terrain.echo-rubble",
                1,
            )
            .expect("the near-border cell should be a valid target")
            .is_empty()
    );
}

#[test]
fn terrain_transform_empty_invalid_and_failure_preserve_rng_boundaries() {
    let ability_id = "demo.ability.echo-delving";
    let empty_center = Position { x: 8, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        let mana_before = candidate.resources["demo.resource.mana"].current;
        let draws_before = candidate.rng_draw_counter();
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: empty_center,
                },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update, mana_before, draws_before));
            break;
        }
    }
    let (mut game, empty, mana_before, draws_before) =
        selected.expect("a deterministic empty terrain transformation should succeed");
    let resolution = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty terrain transformation should still resolve");
    assert!(resolution.transformed_positions.is_empty());
    assert!(game.resources["demo.resource.mana"].current < mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert!(empty.changed_cells.is_empty());

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let progress_before_rejection = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert_eq!(game.ability_progress[ability_id], progress_before_rejection);

    for seed in 0..128 {
        let mut failure =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut failure);
        failure
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        failure.learned_abilities.insert(ability_id.to_owned());
        replace_terrain(&mut failure, Position { x: 5, y: 3 }, "demo.terrain.wall");
        let terrain_before = failure.terrain.clone();
        let mana_before = failure.resources["demo.resource.mana"].current;
        let update = dispatch_next(
            &mut failure,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: Position { x: 5, y: 3 },
                },
            },
        );
        if !ability_cast_resolution(&update).succeeded {
            assert!(failure.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(failure.terrain, terrain_before);
            assert!(!update.events.iter().any(|event| {
                matches!(
                    event.outcome,
                    Some(GameEventOutcomeDto::AbilityTerrainTransform { .. })
                )
            }));
            return;
        }
    }
    panic!("a terrain transformation failure seed should exist");
}

#[test]
fn self_status_sequence_applies_in_order_and_round_trips() {
    let ability_id = "demo.ability.echo-quickening";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        candidate.player.statuses.push(StatusInstance {
            kind_id: STATUS_SLOW.to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some("test.slow".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic self status cast should succeed");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("status sequence should expose its ordered effects");
    assert_eq!(
        resolution.target_entity_id.as_deref(),
        Some("demo.actor.player.1")
    );
    assert_eq!(resolution.effects.len(), 2);
    assert!(matches!(
        resolution.effects[0],
        AbilityEffectResolutionDto::ApplyStatus {
            effect_index: 0,
            change: AbilityStatusChangeDto::Added,
            applied_duration_ticks: 30,
            ..
        }
    ));
    assert!(matches!(
        resolution.effects[1],
        AbilityEffectResolutionDto::RemoveStatus {
            effect_index: 1,
            removed: true,
            ..
        }
    ));
    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE)
    );
    assert!(
        game.player
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLOW)
    );
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == ability_id)
        .expect("status sequence should be projected");
    assert!(matches!(
        ability.effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus { .. },
            AbilityEffectSpecDto::RemoveStatus { .. }
        ]
    ));

    let restored = Game::from_save(game.to_save()).expect("status ability state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn target_status_sequence_resists_immunizes_and_skips_after_death() {
    let ability_id = "demo.ability.echo-binding";
    let prepare = |seed: u64, hp: i32, resistance: ResistanceLevel| {
        let mut game =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        let entity = game.entities[0].clone();
        clear_monsters(&mut game);
        game.entities.push(entity);
        game.entities[0].position = Position { x: 4, y: 3 };
        game.entities[0].hp = hp;
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.entities[0]
            .resistances
            .set(DamageType::Cold, resistance);
        game.learned_abilities.insert(ability_id.to_owned());
        game.ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        game
    };
    let seed = (0..128)
        .find(|seed| {
            let mut game = prepare(*seed, 3, ResistanceLevel::Normal);
            let target_id = game.entities[0].id.clone();
            let update = dispatch_next(
                &mut game,
                GameCommand::CastAbility {
                    ability_id: ability_id.to_owned(),
                    target: TargetSelection::Entity {
                        entity_id: target_id,
                    },
                },
            );
            ability_cast_resolution(&update).succeeded
        })
        .expect("a deterministic target status cast should succeed");

    let mut resistant = prepare(seed, 3, ResistanceLevel::Resistant);
    let target_id = resistant.entities[0].id.clone();
    let resistant_update = dispatch_next(
        &mut resistant,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let resistant_resolution = resistant_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("resisted status sequence should resolve");
    assert!(matches!(
        resistant_resolution.effects[1],
        AbilityEffectResolutionDto::ApplyStatus {
            effect_index: 1,
            requested_duration_ticks: 30,
            applied_duration_ticks: 15,
            resistance: Some(ResistanceLevelDto::Resistant),
            change: AbilityStatusChangeDto::Added,
            ..
        }
    ));
    assert!(
        resistant
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
            .is_some_and(|entity| entity
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLOW))
    );
    let restored = Game::from_save(resistant.to_save()).expect("resisted status should round-trip");
    assert_eq!(restored.snapshot(), resistant.snapshot());

    let mut immune = prepare(seed, 3, ResistanceLevel::Immune);
    let target_id = immune.entities[0].id.clone();
    let immune_update = dispatch_next(
        &mut immune,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let immune_resolution = immune_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("immune status sequence should resolve");
    assert!(matches!(
        immune_resolution.effects[1],
        AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks: 0,
            resistance: Some(ResistanceLevelDto::Immune),
            change: AbilityStatusChangeDto::Immune,
            ..
        }
    ));
    assert!(
        immune
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
            .is_some_and(|entity| entity.statuses.is_empty())
    );

    let mut lethal = prepare(seed, 1, ResistanceLevel::Normal);
    let target_id = lethal.entities[0].id.clone();
    let lethal_update = dispatch_next(
        &mut lethal,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let lethal_resolution = lethal_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("lethal sequence should resolve");
    assert!(matches!(
        lethal_resolution.effects[0],
        AbilityEffectResolutionDto::Damage {
            effect_index: 0,
            ..
        }
    ));
    assert!(matches!(
        lethal_resolution.effects[1],
        AbilityEffectResolutionDto::Skipped {
            effect_index: 1,
            reason: AbilityEffectSkipReasonDto::TargetDead,
        }
    ));
    assert!(lethal.entities.iter().all(|entity| entity.id != target_id));
}

#[test]
fn actor_effect_sequences_preserve_empty_invalid_and_failure_rng_boundaries() {
    let ability_id = "demo.ability.echo-binding";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        let mana_before = candidate.resources["demo.resource.mana"].current;
        let draws_before = candidate.rng_draw_counter();
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: Position { x: 6, y: 3 },
                },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update, mana_before, draws_before));
            break;
        }
    }
    let (mut game, empty, mana_before, draws_before) =
        selected.expect("a deterministic empty effect sequence should succeed");
    let resolution = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty effect sequence should expose skipped effects");
    assert!(resolution.target_entity_id.is_none());
    assert!(resolution.effects.iter().all(|effect| matches!(
        effect,
        AbilityEffectResolutionDto::Skipped {
            reason: AbilityEffectSkipReasonDto::NoTarget,
            ..
        }
    )));
    assert!(game.resources["demo.resource.mana"].current < mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before + 1);

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let progress_before_rejection = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert_eq!(game.ability_progress[ability_id], progress_before_rejection);

    for seed in 0..128 {
        let mut failure =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        failure.entities.truncate(1);
        failure.entities[0].position = Position { x: 4, y: 3 };
        failure.entities[0].energy_need = i32::MAX / 2;
        failure.learned_abilities.insert(ability_id.to_owned());
        let target_id = failure.entities[0].id.clone();
        let mana_before = failure.resources["demo.resource.mana"].current;
        let draws_before = failure.rng_draw_counter();
        let update = dispatch_next(
            &mut failure,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Entity {
                    entity_id: target_id,
                },
            },
        );
        if !ability_cast_resolution(&update).succeeded {
            assert!(failure.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(failure.rng_draw_counter(), draws_before + 1);
            assert!(failure.entities[0].statuses.is_empty());
            assert!(
                !update
                    .events
                    .iter()
                    .any(|event| event.kind == "ability.effects")
            );
            return;
        }
    }
    panic!("an effect sequence failure seed should exist");
}

#[test]
fn learning_capacity_forget_and_relearn_preserve_ability_progress() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    let echo_primer = ability_book_item_id_for(&game, "demo.item.echo-primer");
    let stillwater_notes = ability_book_item_id_for(&game, "demo.item.stillwater-notes");
    let initial = game.snapshot();
    assert_eq!(
        initial.player.ability_learning,
        Some(AbilityLearningDto {
            learned_count: 0,
            capacity: 2,
            remaining_slots: 2,
        })
    );
    assert_eq!(initial.player.abilities.len(), 46);
    assert_eq!(
        initial
            .player
            .abilities
            .iter()
            .filter(|ability| ability.can_study)
            .count(),
        22
    );
    assert!(
        initial
            .player
            .abilities
            .iter()
            .all(|ability| !ability.can_forget)
    );

    for (book_item_id, ability_id) in [
        (echo_primer.clone(), "demo.ability.resonant-bolt"),
        (stillwater_notes, "demo.ability.mending-echo"),
    ] {
        dispatch_next(
            &mut game,
            GameCommand::StudyAbility {
                book_item_id,
                ability_id: ability_id.to_owned(),
            },
        );
    }
    let full = game.snapshot();
    assert_eq!(
        full.player.ability_learning,
        Some(AbilityLearningDto {
            learned_count: 2,
            capacity: 2,
            remaining_slots: 0,
        })
    );
    assert!(
        full.player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.harmonic-spark")
            .is_some_and(|ability| !ability.can_study)
    );

    let draws_before_rejection = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: echo_primer.clone(),
            ability_id: "demo.ability.harmonic-spark".to_owned(),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.study-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "learning-capacity-full")
    }));

    let retained_progress = AbilityProgress {
        proficiency: SPELL_EXP_EXPERT,
        proficiency_cap: SPELL_EXP_MASTER,
        cast_count: 12,
        fail_count: 3,
        cooldown_remaining: 0,
    };
    game.ability_progress
        .insert("demo.ability.resonant-bolt".to_owned(), retained_progress);
    let forgotten = dispatch_next(
        &mut game,
        GameCommand::ForgetAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        forgotten
            .events
            .iter()
            .any(|event| event.kind == "ability.forgotten")
    );
    assert_eq!(
        game.ability_progress["demo.ability.resonant-bolt"],
        retained_progress
    );
    let after_forget = game.snapshot();
    assert_eq!(
        after_forget
            .player
            .ability_learning
            .unwrap()
            .remaining_slots,
        1
    );
    assert!(
        after_forget
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.resonant-bolt")
            .is_some_and(|ability| !ability.learned && !ability.can_forget)
    );

    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: echo_primer,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    let relearned = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("relearned ability should remain projected");
    assert!(relearned.learned);
    assert_eq!(relearned.proficiency, SPELL_EXP_EXPERT);
    assert_eq!(relearned.cast_count, 12);
    assert_eq!(relearned.fail_count, 3);

    let restored = Game::from_save(game.to_save()).expect("forgotten progress should reload");
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut over_capacity = Game::new_with_build(0, "demo.build.scholar")
        .expect("scholar build should create")
        .to_save();
    over_capacity.player.learned_ability_ids = vec![
        "demo.ability.harmonic-spark".to_owned(),
        "demo.ability.mending-echo".to_owned(),
        "demo.ability.resonant-bolt".to_owned(),
    ];
    assert!(matches!(
        Game::from_save(over_capacity),
        Err(CoreError::InvalidSave(
            "learned ability set exceeds learning capacity"
        ))
    ));
}

#[test]
fn class_casting_overrides_drive_study_cast_projection_and_save_validation() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let profile = artifact
        .content
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("demo mage should have a casting profile");
    profile
        .ability_overrides
        .push(rfb_content::AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            minimum_level: 2,
            resource_cost: 9,
            base_failure_percent: 47,
            level_scaling: Vec::new(),
        });
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("casting override content should remain valid"),
    ));
    let mut game = Game::from_content_with_build(
        0,
        Arc::clone(&catalog),
        BUILT_IN_WORLD_ID,
        "demo.build.scholar",
    )
    .expect("custom scholar build should create");
    let book_item_id = ability_book_item_id(&game);

    let initial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("override ability should be projected");
    assert_eq!(initial.minimum_level, 2);
    assert_eq!(initial.base_resource_cost, 9);
    assert!(!initial.can_study);

    game.apply_player_experience(10, &mut Vec::new());
    assert_eq!(game.progress.level, 2);
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("override ability should remain projected");
    assert!(available.can_study);
    assert_ne!(available.failure_percent, 20);

    let studied = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        studied
            .events
            .iter()
            .any(|event| event.kind == "ability.studied")
    );

    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&cast);
    assert_eq!(resolution.base_resource_cost, 9);
    assert_eq!(resolution.resource_cost, available.resource_cost);
    assert_eq!(resolution.failure_percent, available.failure_percent);

    let snapshot = game.snapshot();
    let restored = Game::from_save_with_content(game.to_save(), catalog)
        .expect("learned override ability should reload against the same content");
    assert_eq!(restored.snapshot(), snapshot);
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
        game.resolve_ability_actor_effects(
            &ability.id,
            &ability.effect,
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
        .resolve_ability_actor_effects(
            &ability.id,
            &ability.effect,
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
fn vampiric_branding_is_permanent_and_only_the_source_weapon_drains_life() {
    let ability_id = "demo.ability.death-vampiric-branding";
    let mut branded = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 34, ability_id);
        game.items.push(ItemInstance {
            id: "test.item.branding-blade".to_owned(),
            kind_id: "demo.item.echo-blade".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        });
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Vampiric Branding should resolve");
        if events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::EnchantEquippedWeapon {
                            added: true,
                            affix_id,
                            ..
                        }] if affix_id == "demo.affix.vampiric"
                    )
            )
        }) {
            branded = Some(game);
            break;
        }
    }
    let game = branded.expect("a deterministic branding cast should succeed");
    let weapon = game
        .items
        .iter()
        .find(|item| item.id == "test.item.branding-blade")
        .expect("branded weapon should remain equipped");
    assert_eq!(weapon.affix_ids, ["demo.affix.vampiric"]);
    let knowledge = game
        .item_property_knowledge
        .get(&weapon.id)
        .expect("branding should identify the weapon");
    assert!(knowledge.appraised && knowledge.identified);
    assert!(knowledge.known_affix_ids.contains("demo.affix.vampiric"));
    let mut game = game;
    game.progress.level = 1;
    game.progress.max_level = 1;
    game.learned_abilities.remove(ability_id);
    let level_one_mana = Game::new_with_build(0, "demo.build.scholar")
        .expect("level-one scholar should create")
        .resources["demo.resource.mana"]
        .maximum;
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana");
    mana.current = level_one_mana;
    mana.maximum = level_one_mana;
    let restored = Game::from_save(game.to_save()).expect("branding should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .item_passives(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == "test.item.branding-blade")
                    .unwrap()
            )
            .contains(&EquipmentPassive::Vampiric)
    );

    let prepare_melee = |seed, weapon_vampiric: bool| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.items.push(ItemInstance {
            id: "test.item.melee-blade".to_owned(),
            kind_id: "demo.item.echo-blade".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Fine,
            affix_ids: weapon_vampiric
                .then(|| "demo.affix.vampiric".to_owned())
                .into_iter()
                .collect(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        });
        if !weapon_vampiric {
            game.items.push(ItemInstance {
                id: "test.item.vampiric-charm".to_owned(),
                kind_id: "demo.item.echo-charm".to_owned(),
                quantity: 1,
                quality: ItemQualityDto::Fine,
                affix_ids: vec!["demo.affix.vampiric".to_owned()],
                rolled_affixes: Vec::new(),
                enchantments: Default::default(),
                curse: None,
                activation: None,
                charges: None,
                device_recovery_progress: 0,
                location: ItemLocation::Equipped {
                    slot_id: "charm".to_owned(),
                },
            });
        }
        game.player.statuses.push(StatusInstance {
            kind_id: "test.status.melee-power".to_owned(),
            intensity: 1,
            remaining_ticks: 10,
            source_id: None,
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto {
                melee_skill: 1_000,
                melee_damage: 20,
                ..EquipmentBonusesDto::default()
            },
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let definition = game
            .content
            .actor("demo.actor.gloom-weaver")
            .expect("living target should exist")
            .clone();
        let position = Position { x: 4, y: 3 };
        replace_terrain(&mut game, position, "demo.terrain.floor");
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.vampiric-target",
            &definition.id,
            position,
            500,
            definition.speed,
            100,
            true,
        ));
        game.player.hp = 1;
        game
    };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate = prepare_melee(seed, true);
        let mut events = Vec::new();
        candidate
            .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("vampiric melee should resolve");
        if events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerVampiricHealed { .. }))
        {
            selected = Some((seed, candidate, events));
            break;
        }
    }
    let (seed, drained, events) = selected.expect("a deterministic melee hit should drain life");
    assert!(drained.player.hp > 1);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerVampiricHealed { resolution }
            if resolution.requested <= 50 && resolution.applied > 0
    )));

    let mut charm_only = prepare_melee(seed, false);
    let mut events = Vec::new();
    charm_only
        .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("charm-only melee should resolve");
    assert_eq!(charm_only.player.hp, 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerVampiricHealed { .. }))
    );
}

#[test]
fn vampirism_true_retraces_the_path_after_each_kill() {
    let ability_id = "demo.ability.death-vampirism-true";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 36, ability_id);
        for (ordinal, x) in [4, 5, 6].into_iter().enumerate() {
            let position = Position { x, y: 3 };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                &format!("test.actor.drain-{ordinal}"),
                "demo.actor.gloom-weaver",
                position,
                7,
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
fn nether_wave_uses_one_roll_for_visible_living_targets() {
    let ability_id = "demo.ability.death-nether-wave";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 38, ability_id);
        for (id, kind_id, position) in [
            (
                "test.actor.wave-living-a",
                "demo.actor.gloom-weaver",
                Position { x: 4, y: 3 },
            ),
            (
                "test.actor.wave-living-b",
                "demo.actor.gloom-weaver",
                Position { x: 3, y: 4 },
            ),
            (
                "test.actor.wave-nonliving",
                "demo.actor.resonant-warden",
                Position { x: 2, y: 3 },
            ),
        ] {
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                id, kind_id, position, 500, 100, 100, true,
            ));
        }
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Nether Wave should resolve");
        if let Some(raw_damage) = events.iter().find_map(|event| match event {
            DomainEvent::AbilityVisibleDamage { resolution, .. } => {
                Some(resolution.base_raw_damage)
            }
            _ => None,
        }) {
            selected = Some((game, raw_damage));
            break;
        }
    }
    let (game, raw_damage) = selected.expect("a deterministic Nether Wave should succeed");
    assert!(raw_damage > 0);
    assert_eq!(game.entities[0].hp, 500 - raw_damage);
    assert_eq!(game.entities[1].hp, 500 - raw_damage);
    assert_eq!(game.entities[2].hp, 500);
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
fn cloud_kill_centers_on_the_caster_and_entropy_filters_nonliving_targets() {
    let game = Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let cloud_kill = game
        .content
        .ability("demo.ability.death-cloud-kill")
        .expect("cloud kill should exist");
    let plan = game
        .ability_target_plan(cloud_kill, &TargetSelection::SelfTarget)
        .expect("cloud kill self target should plan");
    assert!(matches!(
        plan,
        AbilityTargetPlan::Projectile {
            ref path,
            stop_at_actor: false,
        } if path.is_empty()
    ));
    let (trace, _) = game.trace_projectile_path_with_actor_policy(Vec::new(), false);
    assert_eq!(trace.landing, game.player.position);

    let mut filtered = Game::new(0);
    clear_monsters(&mut filtered);
    let center = Position { x: 5, y: 3 };
    for (id, kind_id, position) in [
        ("test.actor.living", "demo.actor.gloom-weaver", center),
        (
            "test.actor.nonliving",
            "demo.actor.resonant-warden",
            Position { x: 6, y: 3 },
        ),
    ] {
        let definition = filtered.content.actor(kind_id).expect("demo actor").clone();
        replace_terrain(&mut filtered, position, "demo.terrain.floor");
        filtered.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    let (_, targets) = filtered.area_damage_targets(center, 2, Some("living"));
    assert_eq!(targets, vec![("test.actor.living".to_owned(), 0)]);
}

#[test]
fn vampiric_drain_heals_actual_life_and_rejects_nonliving_targets() {
    let mut game = Game::new(11);
    clear_monsters(&mut game);
    let definition = game
        .content
        .actor("demo.actor.gloom-weaver")
        .expect("demo living target")
        .clone();
    let position = Position { x: 4, y: 3 };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.living",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    game.player.hp = 1;
    let mut events = Vec::new();
    game.resolve_ability_drain_life(
        "test.ability.vampiric-drain",
        vec![position],
        1,
        1,
        99,
        DamageType::Physical,
        "living",
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("vampiric drain should resolve");
    assert_eq!(game.player.hp, 1 + definition.max_hp);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::DrainLife { healing, .. }]
                    if healing.requested == definition.max_hp
                        && healing.applied == definition.max_hp
            )
    )));

    let mut nonliving = Game::new(11);
    clear_monsters(&mut nonliving);
    let definition = nonliving
        .content
        .actor("demo.actor.resonant-warden")
        .expect("demo nonliving target")
        .clone();
    replace_terrain(&mut nonliving, position, "demo.terrain.floor");
    nonliving.entities.push(actor_from_runtime_spawn(
        "test.actor.nonliving",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    let hp_before = nonliving.entities[0].hp;
    let draws_before = nonliving.rng.draw_counter;
    let mut events = Vec::new();
    nonliving
        .resolve_ability_drain_life(
            "test.ability.vampiric-drain",
            vec![position],
            1,
            1,
            99,
            DamageType::Physical,
            "living",
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("ineligible drain should still resolve");
    assert_eq!(nonliving.entities[0].hp, hp_before);
    assert_eq!(nonliving.rng.draw_counter, draws_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Skipped {
                    reason: AbilityEffectSkipReasonDto::Ineligible,
                    ..
                }]
            )
    )));
}

#[test]
fn poison_branding_is_temporary_affects_melee_and_round_trips() {
    let mut game = Game::new(13);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "test.item.echo-blade".to_owned(),
        kind_id: "demo.item.echo-blade".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let definition = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo living target")
        .clone();
    let target = actor_from_runtime_spawn(
        "test.actor.poison-brand-target",
        &definition.id,
        Position { x: 4, y: 3 },
        definition.max_hp,
        definition.speed,
        100,
        true,
    );
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );

    let resolution = apply_ability_status_effect(
        &mut game.player,
        "demo.ability.death-poison-branding",
        0,
        "rfb.status.poison-branding",
        1,
        2,
        0,
        0,
        AbilityStatusStackingDefinition::Replace,
        None,
        None,
        &BTreeMap::new(),
        &BTreeSet::from([WeaponBrand::Poison]),
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
            ref granted_brands,
            change: AbilityStatusChangeDto::Added,
            ..
        } if granted_brands == &[WeaponBrandDto::Poison]
    ));
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        24
    );
    assert_eq!(
        game.snapshot().player.statuses[0].granted_brands,
        vec![WeaponBrandDto::Poison]
    );

    let restored = Game::from_save(game.to_save()).expect("temporary brand should reload");
    assert_eq!(
        restored.snapshot().player.statuses[0].granted_brands,
        vec![WeaponBrandDto::Poison]
    );
    let restored_profile = restored.player_melee_profile(&restored.player_derived_stats());
    assert_eq!(
        restored.player_melee_damage_multiplier(&restored_profile, &target, &definition),
        24
    );

    let mut legacy_value = serde_json::to_value(game.to_save()).expect("save should serialize");
    legacy_value["player"]["statuses"][0]
        .as_object_mut()
        .expect("status should be an object")
        .remove("grantedBrands");
    let legacy_payload: SavePayloadV1 =
        serde_json::from_value(legacy_value).expect("old status save should deserialize");
    Game::from_save(legacy_payload).expect("old status save should remain loadable");

    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("first brand tick should resolve");
    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("second brand tick should expire");
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
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
        Game::from_content_with_build(19, catalog, BUILT_IN_WORLD_ID, "demo.build.scholar")
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
    game.resolve_ability_genocide(
        "test.ability.genocide",
        Some(vec![Position { x: 4, y: 3 }]),
        AbilityGenocideScopeDefinition::Glyph,
        1_000,
        0,
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
    let position = Position { x: 4, y: 3 };
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

    let mut events = Vec::new();
    game.resolve_ability_animate_dead(
        "demo.ability.death-animate-dead",
        "demo.actor.risen-thrall",
        "demo.item.corpse-remains",
        8,
        8,
        &mut events,
        &mut BTreeSet::new(),
    )
    .expect("animate dead should resolve");
    assert!(
        game.items
            .iter()
            .all(|item| item.kind_id != "demo.item.corpse-remains")
    );
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
fn actor_detection_ignores_los_and_orders_entities_stably() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let origin = game.player.position;
    for (id, kind_id, position) in [
        (
            "test.actor.warden",
            "demo.actor.resonant-warden",
            Position {
                x: origin.x + 6,
                y: origin.y,
            },
        ),
        (
            "test.actor.captain",
            "demo.actor.chorus-captain",
            Position {
                x: origin.x + 1,
                y: origin.y + 1,
            },
        ),
        (
            "test.actor.evil",
            "demo.actor.gloom-weaver",
            Position {
                x: origin.x + 7,
                y: origin.y,
            },
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
    replace_terrain(
        &mut game,
        Position {
            x: origin.x + 2,
            y: origin.y,
        },
        "demo.terrain.wall",
    );

    let (positions, ids) = game.detect_actor_positions("nonliving", 8);
    assert_eq!(
        ids,
        vec![
            "test.actor.captain".to_owned(),
            "test.actor.warden".to_owned()
        ]
    );
    assert_eq!(
        positions,
        vec![
            Position {
                x: origin.x + 1,
                y: origin.y + 1,
            },
            Position {
                x: origin.x + 6,
                y: origin.y,
            }
        ]
    );
    assert_eq!(
        game.detect_actor_positions("evil", 8).1,
        vec!["test.actor.evil".to_owned()]
    );
    assert!(game.detect_actor_positions("evil", 6).1.is_empty());
    assert!(game.revealed_terrain.is_empty());
}

#[test]
fn sleep_power_resolves_then_skips_energy_and_damage_wakes_the_target() {
    let template = Game::new(0).entities[0].clone();
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
    let sleeping_actor = game.entities[0].clone();
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

    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("first status tick should resolve");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("second status tick should expire");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Normal
    );
}

#[test]
fn control_resists_ineligible_targets_and_turns_pack_leaders_into_allies() {
    let pack_id = "test.pack.control".to_owned();
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for (id, kind_id, position, role) in [
        (
            "test.actor.controlled",
            "demo.actor.resonant-warden",
            Position { x: 8, y: 3 },
            MonsterPackRoleDto::Leader,
        ),
        (
            "test.actor.member",
            "demo.actor.chorus-captain",
            Position { x: 10, y: 4 },
            MonsterPackRoleDto::Member,
        ),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        let mut actor = actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        );
        actor.pack = Some(MonsterPackIdentity {
            id: pack_id.clone(),
            leader_id: "test.actor.controlled".to_owned(),
            role,
            behavior: MonsterPackBehaviorDto::GuardLeader,
        });
        game.entities.push(actor);
    }

    let draws_before = game.rng.draw_counter;
    let ineligible = game.resolve_ability_control(1, 0, "undead", 100);
    assert!(matches!(
        ineligible,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Ineligible,
            roll: None,
            ..
        }
    ));
    assert_eq!(game.rng.draw_counter, draws_before);

    let controlled = game.resolve_ability_control(0, 0, "undead", 100);
    assert!(matches!(
        controlled,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Controlled,
            roll: Some(_),
            ..
        }
    ));
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(game.entities.iter().all(|entity| entity.pack.is_none()));
    assert_eq!(
        game.snapshot().entities[0].faction,
        EntityFactionDto::Player
    );

    for y in 2..=4 {
        for x in 3..=10 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let old_distance = chebyshev_distance(game.entities[0].position, game.player.position);
    game.resolve_monster_action(
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("controlled actor should use player summon AI");
    assert!(chebyshev_distance(game.entities[0].position, game.player.position) < old_distance);

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("controller identity should reload");
    assert_eq!(restored.snapshot(), snapshot);

    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian")
        .tags
        .push("undead".to_owned());
    artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian")
        .level = 50;
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let mut resisted_game =
        Game::from_content_with_build(0, catalog, BUILT_IN_WORLD_ID, "demo.build.scholar")
            .expect("custom scholar build should create");
    resisted_game.entities.truncate(1);
    resisted_game.entities[0].kind_id = "demo.actor.serpent-of-chaos".to_owned();
    let resisted = resisted_game.resolve_ability_control(0, 0, "undead", 20);
    assert!(matches!(
        resisted,
        AbilityEffectResolutionDto::Control {
            target_level: 50,
            outcome: AbilityControlOutcomeDto::Resisted,
            roll: Some(_),
            ..
        }
    ));
    assert!(resisted_game.entities[0].controller_id.is_none());
}

#[test]
fn spell_proficiency_uses_rfb_ranks_mana_costs_and_failure_adjustments() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let ability = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("resonant bolt should exist")
        .clone();
    let profile = game
        .casting_profile()
        .expect("scholar should have a casting profile")
        .clone();
    let cases = [
        (0, AbilityProficiencyRankDto::Unskilled, 5, 20),
        (900, AbilityProficiencyRankDto::Beginner, 4, 20),
        (1200, AbilityProficiencyRankDto::Skilled, 4, 20),
        (1400, AbilityProficiencyRankDto::Expert, 3, 19),
        (1600, AbilityProficiencyRankDto::Master, 3, 18),
    ];
    for (proficiency, rank, cost, failure) in cases {
        let progress = game
            .ability_progress
            .get_mut(&ability.id)
            .expect("ability progress should exist");
        progress.proficiency = proficiency;
        let progress = *progress;
        assert_eq!(Game::ability_proficiency_rank(proficiency), rank);
        assert_eq!(
            game.ability_effective_resource_cost(&ability, progress),
            cost
        );
        assert_eq!(game.ability_failure_percent(&profile, &ability), failure);
    }
}

#[test]
fn failed_cast_costs_mana_but_insufficient_mana_does_not_draw_rng() {
    let mut failure =
        Game::new_with_build(2, "demo.build.scholar").expect("scholar build should create");
    let failure_book_item_id = ability_book_item_id(&failure);
    dispatch_next(
        &mut failure,
        GameCommand::StudyAbility {
            book_item_id: failure_book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    let failed_cast = dispatch_next(
        &mut failure,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&failed_cast);
    assert_eq!(resolution.percentile_roll, 13);
    assert_eq!(resolution.resource_before, 21);
    assert_eq!(resolution.resource_cost, 5);
    assert_eq!(resolution.resource_after, 16);
    assert!(!resolution.succeeded);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 0);
    assert_eq!(resolution.cast_count, 0);
    assert_eq!(resolution.fail_count, 1);
    assert_eq!(
        failure
            .entities
            .iter()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .map(|entity| entity.hp),
        Some(3)
    );

    let mut insufficient =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let insufficient_book_item_id = ability_book_item_id(&insufficient);
    dispatch_next(
        &mut insufficient,
        GameCommand::StudyAbility {
            book_item_id: insufficient_book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    insufficient
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 2;
    let draws = insufficient.rng_draw_counter();
    let rejected = dispatch_next(
        &mut insufficient,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    assert_eq!(insufficient.rng_draw_counter(), draws);
    assert_eq!(insufficient.resources["demo.resource.mana"].current, 2);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "insufficient-resource")
    }));
    assert!(!rejected.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
        )
    }));
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
fn melee_hits_and_kills_feed_tempo_while_idle_turns_decay_it() {
    let mut game =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    clear_monsters(&mut game);
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.tempo-target",
        "demo.actor.echo-hound",
        Position { x: 4, y: 3 },
        8,
        110,
        1_000_000,
        false,
    ));

    let mut hit_events = 0_u32;
    let mut kill_events = 0_u32;
    let mut turns = 0_u32;
    while game
        .entities
        .iter()
        .any(|entity| entity.id == "generated.actor.tempo-target")
    {
        let update = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        turns += 1;
        for event in &update.events {
            if let Some(GameEventOutcomeDto::ResourceGain { resolution }) = event.outcome.as_ref() {
                assert_eq!(event.kind, "resource.gained");
                assert_eq!(resolution.resource_id, "demo.resource.tempo");
                assert_eq!(resolution.after, resolution.before + resolution.gained);
                match resolution.source {
                    ResourceGainSourceDto::MeleeHit => {
                        assert_eq!(resolution.gained, 2);
                        hit_events += 1;
                    }
                    ResourceGainSourceDto::MeleeKill => {
                        assert_eq!(resolution.gained, 3);
                        kill_events += 1;
                    }
                }
            }
        }
        assert!(turns < 60, "kill should resolve within the turn budget");
    }
    assert!(hit_events >= 2);
    assert_eq!(kill_events, 1);
    let after_kill = game.resources["demo.resource.tempo"].current;
    assert!(after_kill >= 5);

    // An idle wait neither recovers nor feeds tempo, so it decays by one
    // and emits no resource events.
    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.resources["demo.resource.tempo"].current,
        after_kill - 1
    );
    assert!(
        waited
            .events
            .iter()
            .all(|event| { !event.kind.starts_with("resource.") })
    );
}

#[test]
fn technique_casts_consume_tempo_and_reject_shortfalls_without_rng() {
    let mut payload = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = Some(true);
    payload.player.resources[0].current = 10;
    let mut game = Game::from_save(payload).expect("tempo fixture should load");
    let snapshot = game.snapshot();
    let crescent = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crescent-cut")
        .expect("duelist should expose crescent cut");
    assert!(crescent.innate);
    assert!(crescent.can_cast);
    let expected_cost = crescent.resource_cost;

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.crescent-cut".to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cast = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution.clone()),
            _ => None,
        })
        .expect("crescent cut should resolve a cast");
    assert_eq!(cast.resource_id, "demo.resource.tempo");
    assert_eq!(cast.resource_cost, expected_cost);
    assert_eq!(cast.resource_before, 10);
    assert_eq!(cast.resource_after, 10 - expected_cost);
    assert_eq!(
        game.resources["demo.resource.tempo"].current,
        10 - expected_cost
    );

    game.resources
        .get_mut("demo.resource.tempo")
        .expect("tempo pool should exist")
        .current = 0;
    let draws = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.crescent-cut".to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event.args.get("reason").map(String::as_str) == Some("insufficient-resource")
    }));
    assert_eq!(game.rng_draw_counter(), draws);
    assert_eq!(game.resources["demo.resource.tempo"].current, 0);
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
fn rest_interrupts_for_visible_enemies_and_damage_before_recovery() {
    let mut visible =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    visible
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 10;
    visible.entities[0].position = Position { x: 4, y: 3 };
    let visible_draws = visible.rng_draw_counter();
    let blocked = dispatch_next(&mut visible, GameCommand::Rest { turns: 10 });
    let blocked_resolution = rest_resolution(&blocked);
    assert_eq!(blocked_resolution.completed_turns, 0);
    assert_eq!(
        blocked_resolution.stop_reason,
        RestStopReasonDto::EnemyVisible
    );
    assert_eq!(visible.world_tick, 0);
    assert_eq!(visible.rng_draw_counter(), visible_draws);
    assert_eq!(visible.resources["demo.resource.mana"].current, 10);

    let mut payload = Game::new_with_build(0, "demo.build.scholar")
        .expect("scholar build should create")
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = Some(true);
    payload.player.resources[0].current = 10;
    payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_BLEEDING.to_owned(),
        intensity: 1,
        remaining_ticks: 1,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut damaged = Game::from_save(payload).expect("bleeding rest fixture should load");
    let interrupted = dispatch_next(&mut damaged, GameCommand::Rest { turns: 10 });
    let interrupted_resolution = rest_resolution(&interrupted);
    assert_eq!(interrupted_resolution.completed_turns, 1);
    assert_eq!(
        interrupted_resolution.stop_reason,
        RestStopReasonDto::Damaged
    );
    assert!(interrupted_resolution.resource_recoveries.is_empty());
    assert_eq!(damaged.resources["demo.resource.mana"].current, 10);
    assert_eq!(damaged.player.hp, 11);
    assert!(
        interrupted
            .events
            .iter()
            .any(|event| event.kind == "rest.interrupted")
    );
}

#[test]
fn scholar_studies_and_casts_a_self_targeted_healing_ability() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.player.hp = 5;
    let book_item_id = ability_book_item_id_for(&game, "demo.item.stillwater-notes");

    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.mending-echo".to_owned(),
        },
    );
    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.mending-echo".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let cast_resolution = ability_cast_resolution(&cast);
    assert_eq!(cast_resolution.failure_percent, 15);
    assert_eq!(cast_resolution.percentile_roll, 32);
    assert!(cast_resolution.succeeded);
    assert_eq!(cast_resolution.resource_before, 21);
    assert_eq!(cast_resolution.base_resource_cost, 4);
    assert_eq!(cast_resolution.resource_cost, 7);
    assert_eq!(cast_resolution.resource_after, 14);
    assert_eq!(cast_resolution.proficiency_before, 0);
    assert_eq!(cast_resolution.proficiency_after, 128);
    assert_eq!(cast_resolution.cooldown_after, 2);
    assert_eq!(game.player.hp, 11);
    assert!(cast.events.iter().any(|event| {
        event.kind == "ability.healed"
            && matches!(
                event.outcome.as_ref(),
                Some(GameEventOutcomeDto::Heal { resolution })
                    if resolution.requested == 6 && resolution.applied == 6
            )
    }));

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.mending-echo".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "cooldown")
    }));
    assert_eq!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .map(|ability| ability.cooldown_remaining),
        Some(1)
    );

    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .map(|ability| ability.cooldown_remaining),
        Some(0)
    );

    let restored = Game::from_save(game.to_save()).expect("healing ability state should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn escape_teleport_falls_back_to_half_distance_and_blink_rejects_without_space() {
    fn open_cell(game: &mut Game, position: Position) {
        let index = game.index(position).expect("cell");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }

    // Escape fallback: the only open landing sits five tiles from the player,
    // so the minimum-eight filter is empty and the halved minimum applies.
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    open_cell(&mut game, player);
    let stalker_position = Position {
        x: player.x + 1,
        y: player.y,
    };
    open_cell(&mut game, stalker_position);
    let landing = Position {
        x: player.x + 5,
        y: player.y,
    };
    open_cell(&mut game, landing);
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.rift-test",
        "demo.actor.rift-stalker",
        stalker_position,
        7,
        110,
        100,
        true,
    ));
    let mut escaped = false;
    for _ in 0..30 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        if update.events.iter().any(|event| {
            event.kind == "monster.teleported"
                && matches!(
                    event.outcome.as_ref(),
                    Some(GameEventOutcomeDto::MonsterDisplacement { resolution })
                        if resolution.to == landing
                )
        }) {
            escaped = true;
            break;
        }
        if game.player_is_dead() {
            break;
        }
    }
    assert!(escaped, "escape should use the halved minimum distance");

    // Blink rejection: every cell within radius five is walled, so the
    // planner reports no-space without drawing any destination RNG.
    let mut boxed = Game::new(0);
    clear_monsters(&mut boxed);
    for cell in boxed.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let boxed_player = boxed.player.position;
    open_cell(&mut boxed, boxed_player);
    let boxed_stalker = Position {
        x: boxed_player.x + 1,
        y: boxed_player.y,
    };
    open_cell(&mut boxed, boxed_stalker);
    boxed.entities.push(actor_from_runtime_spawn(
        "generated.actor.rift-boxed",
        "demo.actor.rift-stalker",
        boxed_stalker,
        7,
        110,
        100,
        true,
    ));
    let mut saw_rejection = false;
    for _ in 0..30 {
        let update = dispatch_next(&mut boxed, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityDecision { resolution }) =
                event.outcome.as_ref()
                && resolution.candidates.iter().any(|candidate| {
                    candidate.ability_id == "demo.ability.echo-slip"
                        && candidate.rejection_reason
                            == Some(MonsterAbilityRejectionReasonDto::NoSpace)
                })
            {
                saw_rejection = true;
            }
        }
        if saw_rejection || boxed.player_is_dead() {
            break;
        }
    }
    assert!(saw_rejection, "boxed blink should report no-space");
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
fn death_ray_enforces_living_unique_and_level_gates() {
    let resolve = |seed: u64, kind_id: &str| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.progress.level = 50;
        game.rng = RfbRng::seeded(seed);
        let definition = game.content.actor(kind_id).expect("demo target").clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.death-ray-target",
            kind_id,
            Position { x: 4, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        let mut events = Vec::new();
        let mut removed = Vec::new();
        let mut ability = game
            .content
            .ability("demo.ability.death-death-ray")
            .expect("death ray should exist")
            .clone();
        ability.effect = AbilityEffectDefinition::DeathRay { power: 100 };
        game.resolve_player_death_ray_effect(
            &ability,
            vec![Position { x: 4, y: 3 }],
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        )
        .expect("Death Ray should resolve");
        let resolution = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.effects.first().cloned()
                }
                _ => None,
            })
            .expect("Death Ray should emit a resolution");
        (game, resolution, removed)
    };

    let (nonliving, resolution, removed) = resolve(0, "demo.actor.resonant-warden");
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::DeathRay {
            living: false,
            resisted: true,
            unique_roll: None,
            target_level_roll: None,
            caster_level_roll: None,
            ..
        }
    ));
    assert!(removed.is_empty());
    assert_eq!(nonliving.rng.draw_counter, 0);

    let (_, resolution, removed) = resolve(0, "demo.actor.serpent-of-chaos");
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::DeathRay {
            living: true,
            unique: true,
            resisted: true,
            unique_roll: Some(roll),
            ..
        } if roll != 666
    ));
    assert!(removed.is_empty());

    let mut saw_resist = false;
    let mut saw_kill = false;
    for seed in 0..256 {
        let (_, resolution, removed) = resolve(seed, "demo.actor.gloom-weaver");
        let AbilityEffectResolutionDto::DeathRay {
            target_level,
            target_level_roll: Some(target_roll),
            caster_level_roll: Some(caster_roll),
            resisted,
            ..
        } = resolution
        else {
            panic!("living Death Ray should roll its level contest");
        };
        assert_eq!(
            resisted,
            target_level + u32::from(target_roll) > caster_roll
        );
        saw_resist |= resisted && removed.is_empty();
        saw_kill |= !resisted && removed == ["test.actor.death-ray-target"];
        if saw_resist && saw_kill {
            break;
        }
    }
    assert!(saw_resist && saw_kill);
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
            .all(|kind_id| kind_id == "demo.actor.risen-thrall")
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
fn esoteria_validates_item_targets_before_cost_and_persists_knowledge() {
    let item = || ItemInstance {
        id: "test.item.esoteria".to_owned(),
        kind_id: "demo.item.echo-blade".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.vampiric".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(Position { x: 10, y: 10 }),
    };
    let mut invalid = prepare_death_caster(0, 30, "demo.ability.death-esoteria");
    invalid.items.push(item());
    invalid.debug_set_ability_casts_succeed(true);
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng.draw_counter;
    let mut events = Vec::new();
    invalid
        .resolve_player_ability(
            "demo.ability.death-esoteria",
            TargetSelection::Item {
                item_id: "test.item.esoteria".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("invalid Esoteria target should resolve as unavailable");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { .. }]
    ));
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng.draw_counter, draws_before);

    let mut ordinary = None;
    let mut full = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 30, "demo.ability.death-esoteria");
        let mut target = item();
        target.location = ItemLocation::Inventory;
        game.items.push(target);
        game.debug_set_ability_casts_succeed(true);
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.death-esoteria",
            TargetSelection::Item {
                item_id: "test.item.esoteria".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Esoteria should resolve");
        let is_full = events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::IdentifyItem { full: true, .. }]
                    )
            )
        });
        if is_full {
            full.get_or_insert(game);
        } else {
            ordinary.get_or_insert(game);
        }
        if ordinary.is_some() && full.is_some() {
            break;
        }
    }
    let ordinary = ordinary.expect("an ordinary identification seed should exist");
    let ordinary_knowledge = &ordinary.item_property_knowledge["test.item.esoteria"];
    assert!(ordinary_knowledge.appraised);
    assert!(!ordinary_knowledge.identified);
    assert!(ordinary_knowledge.known_affix_ids.is_empty());

    let mut full = full.expect("a full identification seed should exist");
    let full_knowledge = &full.item_property_knowledge["test.item.esoteria"];
    assert!(full_knowledge.appraised && full_knowledge.identified);
    assert!(
        full_knowledge
            .known_affix_ids
            .contains("demo.affix.vampiric")
    );
    full.items
        .iter_mut()
        .find(|item| item.id == "test.item.esoteria")
        .expect("identified item should remain")
        .location = ItemLocation::Ground(Position { x: 10, y: 10 });
    full.refresh_character_skills();
    full.refresh_player_resource_maxima();
    let restored = Game::from_save(full.to_save()).expect("item knowledge should reload");
    assert_eq!(restored.state_hash(), full.state_hash());
    assert!(restored.item_property_knowledge["test.item.esoteria"].identified);
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
    game.resolve_ability_actor_effects(
        &ability.id,
        &ability.effect,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Vampiric Transformation should resolve");

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
