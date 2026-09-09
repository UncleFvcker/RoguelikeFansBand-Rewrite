// SPDX-License-Identifier: MPL-2.0

use super::*;

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

const RACE_SCARE_MONSTER_ABILITY_ID: &str = "rfb.ability.race.scare-monster";

const RACE_SPRITE_SLEEPING_DUST_ABILITY_ID: &str = "rfb.ability.race.sleeping-dust";

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

    let level_fourteen_experience = game.experience_required_for_level(14);
    game.apply_player_experience(level_fourteen_experience, &mut Vec::new());
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

    game.apply_player_experience(
        game.experience_required_for_level(15) - level_fourteen_experience,
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
