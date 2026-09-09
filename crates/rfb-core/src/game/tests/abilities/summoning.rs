// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::ability_projection::ability_effect_spec_dto;
use crate::game::monster_ecology as ecology;

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
