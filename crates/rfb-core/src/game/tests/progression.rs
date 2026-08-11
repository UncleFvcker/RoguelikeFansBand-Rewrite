// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::stats::{AttributeSet, experience_required_for_level, modify_attribute_value};
use rfb_protocol::{AttributeKindDto, MutationRatingDto};

#[test]
fn attribute_potentials_project_save_hash_and_reject_invalid_values() {
    let game = Game::new(42);
    let projected = game.snapshot().player.progress.attributes;
    let saved = game.to_save();
    let saved_progress = saved
        .player
        .progress
        .as_ref()
        .expect("new games must save character progress");
    assert_eq!(
        projected.strength.potential,
        game.progress.attribute_potentials.strength
    );
    assert_eq!(
        saved_progress.attribute_potentials.strength,
        game.progress.attribute_potentials.strength
    );
    assert_eq!(
        Game::from_save(saved.clone())
            .expect("attribute potentials should round trip")
            .state_hash(),
        game.state_hash()
    );

    let mut invalid = saved;
    invalid
        .player
        .progress
        .as_mut()
        .expect("new games must save character progress")
        .attribute_potentials
        .strength = 87;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("character progress is invalid"))
    ));
}

#[test]
fn mutation_state_projects_saves_hashes_and_rejects_invalid_references() {
    let mut game = Game::new(42);
    let initial_hash = game.state_hash();
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.spit-acid".to_owned());
    game.progress
        .locked_mutation_ids
        .insert("rfb.mutation.spit-acid".to_owned());

    let mutation = game
        .snapshot()
        .player
        .mutations
        .into_iter()
        .next()
        .expect("active mutation should project");
    assert_eq!(mutation.id, "rfb.mutation.spit-acid");
    assert_eq!(mutation.name, "喷吐酸液");
    assert_eq!(mutation.description, "你可以喷吐酸液（伤害为 等级*2）。");
    assert_eq!(mutation.rating, MutationRatingDto::Good);
    assert!(mutation.locked);
    assert_ne!(game.state_hash(), initial_hash);

    let saved = game.to_save();
    assert_eq!(saved.player.active_mutation_ids, ["rfb.mutation.spit-acid"]);
    assert_eq!(saved.player.locked_mutation_ids, ["rfb.mutation.spit-acid"]);
    let restored = Game::from_save(saved.clone()).expect("mutation state should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot().player.mutations, [mutation]);

    let mut duplicate = saved.clone();
    duplicate
        .player
        .active_mutation_ids
        .push("rfb.mutation.spit-acid".to_owned());
    assert!(matches!(
        Game::from_save(duplicate),
        Err(CoreError::InvalidSave("player mutation state is invalid"))
    ));

    let mut unknown = saved.clone();
    unknown.player.active_mutation_ids = vec!["rfb.mutation.unknown".to_owned()];
    unknown.player.locked_mutation_ids.clear();
    assert!(matches!(
        Game::from_save(unknown),
        Err(CoreError::InvalidSave("player mutation state is invalid"))
    ));

    let mut unlocked = saved;
    unlocked.player.active_mutation_ids.clear();
    assert!(matches!(
        Game::from_save(unlocked),
        Err(CoreError::InvalidSave("player mutation state is invalid"))
    ));
}

fn game_with_mutation_weights(weights: &[(&str, u8)]) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    for mutation in &mut artifact.content.mutations {
        mutation.random_weight = weights
            .iter()
            .find_map(|(id, weight)| (mutation.id == *id).then_some(*weight))
            .unwrap_or(0);
    }
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("custom mutation weights should remain valid"),
    ));
    Game::from_content(0, catalog, DEFAULT_WORLD_ID)
        .expect("custom mutation content should create a game")
}

#[test]
fn mutation_transactions_preserve_locks_remove_conflicts_and_emit_source_order() {
    let mut game = Game::new(0);
    let mut events = Vec::new();
    assert!(game.gain_mutation("rfb.mutation.moronic", &mut events));
    events.clear();
    assert!(game.gain_mutation("rfb.mutation.pultitis", &mut events));
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.moronic")
    );
    assert!(
        game.progress
            .active_mutation_ids
            .contains("rfb.mutation.pultitis")
    );
    let event_dtos = events
        .drain(..)
        .map(DomainEvent::into_dto)
        .collect::<Vec<_>>();
    assert_eq!(
        event_dtos
            .iter()
            .map(|event| (event.kind.as_str(), event.args["target"].as_str()))
            .collect::<Vec<_>>(),
        [
            ("mutation.lost", "rfb.mutation.moronic"),
            ("mutation.gained", "rfb.mutation.pultitis"),
        ]
    );

    assert!(game.gain_mutation("rfb.mutation.puny", &mut events));
    game.progress
        .locked_mutation_ids
        .insert("rfb.mutation.puny".to_owned());
    events.clear();
    assert!(game.gain_mutation("rfb.mutation.hyper-str", &mut events));
    assert!(
        game.progress
            .active_mutation_ids
            .contains("rfb.mutation.puny")
    );
    assert!(!game.lose_mutation("rfb.mutation.puny", &mut events));

    let mut all = Game::new(0);
    for mutation_id in [
        "rfb.mutation.hyper-str",
        "rfb.mutation.br-fire",
        "rfb.mutation.spit-acid",
        "rfb.mutation.puny",
    ] {
        all.progress
            .active_mutation_ids
            .insert(mutation_id.to_owned());
    }
    all.progress
        .locked_mutation_ids
        .insert("rfb.mutation.puny".to_owned());
    let mut events = Vec::new();
    assert_eq!(all.lose_all_unlocked_mutations(&mut events), 3);
    assert_eq!(
        events
            .into_iter()
            .map(DomainEvent::into_dto)
            .map(|event| event.args["target"].clone())
            .collect::<Vec<_>>(),
        [
            "rfb.mutation.spit-acid",
            "rfb.mutation.br-fire",
            "rfb.mutation.hyper-str",
        ]
    );
    assert_eq!(
        all.progress.active_mutation_ids,
        BTreeSet::from(["rfb.mutation.puny".to_owned()])
    );
    Game::from_save(all.to_save()).expect("transaction result should satisfy save invariants");
}

#[test]
fn passive_mutations_feed_existing_attribute_speed_armor_and_hp_pipelines() {
    let mutation_ids = [
        "rfb.mutation.hyper-str",
        "rfb.mutation.puny",
        "rfb.mutation.hyper-int",
        "rfb.mutation.moronic",
        "rfb.mutation.pultitis",
        "rfb.mutation.resilient",
        "rfb.mutation.xtra-fat",
        "rfb.mutation.albino",
        "rfb.mutation.silly-voice",
        "rfb.mutation.blank-face",
        "rfb.mutation.xtra-legs",
        "rfb.mutation.short-leg",
        "rfb.mutation.warts",
        "rfb.mutation.scales",
        "rfb.mutation.steel-skin",
    ];

    for mutation_id in mutation_ids {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        game.apply_unscaled_player_experience(experience_required_for_level(25), &mut Vec::new());
        let baseline_attributes = game.effective_player_attributes();
        let baseline_stats = game.player_derived_stats();
        let baseline_max_hp = baseline_stats.max_hp.value;
        game.player.hp = baseline_max_hp;
        let definition = game
            .content
            .mutation(mutation_id)
            .unwrap_or_else(|| panic!("{mutation_id} should exist"))
            .clone();

        assert!(game.gain_mutation(mutation_id, &mut Vec::new()));
        let cap = CharacterProgress::attribute_cap(game.victory_level_cap_unlocked());
        let expected_attributes = AttributeSet {
            strength: modify_attribute_value(
                baseline_attributes.strength,
                definition.modifiers.strength,
                cap,
            ),
            intelligence: modify_attribute_value(
                baseline_attributes.intelligence,
                definition.modifiers.intelligence,
                cap,
            ),
            wisdom: modify_attribute_value(
                baseline_attributes.wisdom,
                definition.modifiers.wisdom,
                cap,
            ),
            dexterity: modify_attribute_value(
                baseline_attributes.dexterity,
                definition.modifiers.dexterity,
                cap,
            ),
            constitution: modify_attribute_value(
                baseline_attributes.constitution,
                definition.modifiers.constitution,
                cap,
            ),
            charisma: modify_attribute_value(
                baseline_attributes.charisma,
                definition.modifiers.charisma,
                cap,
            ),
        };
        let stats = game.player_derived_stats();
        assert_eq!(game.effective_player_attributes(), expected_attributes);
        assert_eq!(
            stats.speed.value,
            baseline_stats.speed.value + definition.modifiers.speed
        );
        assert_eq!(
            stats.armor_class.value,
            baseline_stats.armor_class.value + definition.armor_class
        );
        assert_eq!(game.player.hp, stats.max_hp.value);
        if definition.modifiers.constitution > 0 {
            assert!(stats.max_hp.value > baseline_max_hp);
        } else if definition.modifiers.constitution < 0 {
            assert!(stats.max_hp.value < baseline_max_hp);
        }

        assert!(game.lose_mutation(mutation_id, &mut Vec::new()));
        let restored = game.player_derived_stats();
        assert_eq!(game.effective_player_attributes(), baseline_attributes);
        assert_eq!(restored.speed.value, baseline_stats.speed.value);
        assert_eq!(restored.armor_class.value, baseline_stats.armor_class.value);
        assert_eq!(restored.max_hp.value, baseline_max_hp);
        assert_eq!(game.player.hp, baseline_max_hp);
    }

    let mut skin = Game::new(0);
    let baseline_armor = skin.player_derived_stats().armor_class.value;
    assert!(skin.gain_mutation("rfb.mutation.warts", &mut Vec::new()));
    assert!(skin.gain_mutation("rfb.mutation.steel-skin", &mut Vec::new()));
    assert!(
        !skin
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.warts")
    );
    assert_eq!(
        skin.player_derived_stats().armor_class.value,
        baseline_armor + 25
    );
}

#[test]
fn m4b_passives_feed_resistance_sense_skill_and_flight_pipelines() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(experience_required_for_level(25), &mut Vec::new());

    let saving_throw = game.player_derived_stats().saving_throw_skill.value;
    assert!(game.gain_mutation("rfb.mutation.magic-res", &mut Vec::new()));
    assert_eq!(
        game.player_derived_stats().saving_throw_skill.value,
        saving_throw + 20
    );
    assert!(game.lose_mutation("rfb.mutation.magic-res", &mut Vec::new()));
    assert_eq!(
        game.player_derived_stats().saving_throw_skill.value,
        saving_throw
    );

    assert!(game.gain_mutation("rfb.mutation.fearless", &mut Vec::new()));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        resisted_status_duration(
            100,
            game.effective_player_resistances().level(DamageType::Fear)
        ),
        50
    );
    assert!(game.gain_mutation("rfb.mutation.no-inhibitions", &mut Vec::new()));
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.fearless")
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Resistant
    );

    assert!(game.gain_mutation("rfb.mutation.sensitive-eyes", &mut Vec::new()));
    assert_eq!(game.player_infravision_range(), 4);
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Blindness),
        ResistanceLevel::Vulnerable
    );
    game.resolve_item_blindness("demo.item.veil-draught", 0, 1, 100, &mut Vec::new());
    assert_eq!(
        game.player
            .statuses
            .iter()
            .find(|status| status.kind_id == STATUS_BLINDNESS)
            .expect("blindness should be applied")
            .remaining_ticks,
        150
    );
    assert!(game.lose_mutation("rfb.mutation.sensitive-eyes", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.infravision", &mut Vec::new()));
    assert_eq!(game.player_infravision_range(), 3);

    assert!(game.gain_mutation("rfb.mutation.vuln-elem", &mut Vec::new()));
    for damage_type in [
        DamageType::Acid,
        DamageType::Cold,
        DamageType::Electricity,
        DamageType::Fire,
    ] {
        assert_eq!(
            game.effective_player_resistances().level(damage_type),
            ResistanceLevel::Vulnerable,
            "{damage_type:?} vulnerability"
        );
    }

    assert!(game.gain_mutation("rfb.mutation.weird-mind", &mut Vec::new()));
    game.apply_player_melee_status(
        crate::effect::STATUS_HALLUCINATION,
        100,
        "test.eldritch-source",
    );
    assert!(!game.player_has_status_kind(crate::effect::STATUS_HALLUCINATION));

    let deep_lava = game
        .content
        .terrain("demo.terrain.surface-lava-deep")
        .expect("deep lava terrain")
        .clone();
    assert!(!game.player_can_cross_surface_terrain(&deep_lava));
    assert!(game.gain_mutation("rfb.mutation.wings", &mut Vec::new()));
    assert!(game.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly));
    assert!(game.player_can_cross_surface_terrain(&deep_lava));
}

#[test]
fn esp_core_detects_normal_minds_but_not_explicit_empty_minds() {
    let position = Position { x: 5, y: 3 };
    let mut normal = Game::new(0);
    clear_monsters(&mut normal);
    normal.player.position = Position { x: 3, y: 3 };
    normal.push_generated_actor(
        "test.normal-mind".to_owned(),
        "demo.actor.small-kobold",
        position,
    );
    assert!(!normal.entity_is_visible_by_telepathy(&normal.entities[0]));
    assert!(normal.gain_mutation("rfb.mutation.esp", &mut Vec::new()));
    assert!(normal.entity_is_visible_by_telepathy(&normal.entities[0]));

    let mut empty = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.tags.push("empty-mind".to_owned());
    });
    clear_monsters(&mut empty);
    empty.player.position = Position { x: 3, y: 3 };
    empty.push_generated_actor(
        "test.empty-mind".to_owned(),
        "demo.actor.small-kobold",
        position,
    );
    assert!(empty.gain_mutation("rfb.mutation.esp", &mut Vec::new()));
    assert!(!empty.entity_is_visible_by_telepathy(&empty.entities[0]));
}

#[test]
fn m4c_regeneration_and_fire_light_feed_existing_player_pipelines() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let constitution = game.effective_player_attributes().constitution;

    assert_eq!(game.player_regeneration_rate_percent(), 100);
    assert!(game.gain_mutation("rfb.mutation.regen", &mut Vec::new()));
    assert_eq!(game.player_regeneration_rate_percent(), 200);
    assert!(game.gain_mutation("rfb.mutation.flesh-rot", &mut Vec::new()));
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.regen")
    );
    assert_eq!(game.player_regeneration_rate_percent(), 20);
    assert_eq!(
        game.effective_player_attributes().constitution,
        constitution - 2
    );

    let mut draconian = Game::new(0);
    assert!(draconian.gain_mutation("rfb.mutation.draconian-regen", &mut Vec::new()));
    assert_eq!(draconian.player_regeneration_rate_percent(), 250);

    let recovered = |mutation_id: Option<&str>| {
        let mut candidate = Game::new(0);
        if let Some(mutation_id) = mutation_id {
            candidate
                .progress
                .active_mutation_ids
                .insert(mutation_id.to_owned());
            candidate
                .progress
                .locked_mutation_ids
                .insert(mutation_id.to_owned());
        }
        candidate.progress.hp_progression[0] = 10_000;
        candidate.player.hp = 1;
        candidate.world_tick = NATURAL_HP_REGENERATION_INTERVAL_TICKS;
        candidate.process_natural_hp_regeneration(false);
        candidate.player.hp - 1
    };
    assert!(recovered(Some("rfb.mutation.regen")) > recovered(None));
    assert!(recovered(Some("rfb.mutation.flesh-rot")) < recovered(None));

    let mut light = Game::new(0);
    assert_eq!(light.player_light_radius(), None);
    assert!(light.gain_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    assert_eq!(light.player_light_radius(), Some(1));
    assert!(light.lose_mutation("rfb.mutation.fire-aura", &mut Vec::new()));
    assert_eq!(light.player_light_radius(), None);
}

#[test]
fn m4d_passive_combat_modifiers_feed_existing_attribute_and_skill_pipelines() {
    let mut game = Game::new(0);
    let base_dexterity = game.effective_player_attributes().dexterity;
    let base_stats = game.player_derived_stats();

    assert!(game.gain_mutation("rfb.mutation.limber", &mut Vec::new()));
    assert_eq!(
        game.effective_player_attributes().dexterity,
        base_dexterity + 3
    );
    assert!(game.gain_mutation("rfb.mutation.arthritis", &mut Vec::new()));
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains("rfb.mutation.limber")
    );
    assert_eq!(
        game.effective_player_attributes().dexterity,
        base_dexterity - 3
    );

    assert!(game.gain_mutation("rfb.mutation.motion", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.untouchable", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.tread-softly", &mut Vec::new()));
    let stats = game.player_derived_stats();
    assert_eq!(
        stats.stealth_skill.value,
        base_stats.stealth_skill.value + 4
    );
    assert_eq!(stats.armor_class.value, base_stats.armor_class.value + 20);
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
}

#[test]
fn m4e_cross_system_mutations_reuse_stats_energy_experience_and_item_knowledge() {
    const ITEM_ID: &str = "test.item.m4e-water.1";
    const SCROLL_ID: &str = "test.item.m4e-scroll.1";

    let mut game = Game::new(0);
    let base = game.player_derived_stats();
    assert!(game.gain_mutation("rfb.mutation.xtra-eyes", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.xtra-noise", &mut Vec::new()));
    let stats = game.player_derived_stats();
    assert_eq!(stats.search_skill.value, base.search_skill.value + 15);
    assert_eq!(
        stats.perception_skill.value,
        base.perception_skill.value + 15
    );
    assert_eq!(
        stats.stealth_skill.value,
        base.stealth_skill.value.saturating_sub(3).max(0)
    );
    assert!(
        stats
            .stealth_skill
            .contributions
            .iter()
            .any(|contribution| {
                contribution.source_id == "rfb.mutation.xtra-noise" && contribution.amount == -3
            })
    );

    game.items.clear();
    give_inventory_item(&mut game, ITEM_ID, "demo.item.water-potion");
    let mut events = Vec::new();
    assert!(game.gain_mutation("rfb.mutation.loremaster", &mut events));
    assert_eq!(
        game.item_knowledge_dto("demo.item.water-potion"),
        rfb_protocol::ItemKnowledgeDto::Aware
    );
    assert!(game.item_property_knowledge[ITEM_ID].appraised);
    assert!(!game.item_property_knowledge[ITEM_ID].identified);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::ItemAutoIdentified { count: 1 })
    ));

    assert!(game.gain_mutation("rfb.mutation.fast-learner", &mut Vec::new()));
    assert_eq!(game.player_kill_experience_reward(100), 120);
    assert_eq!(game.player_relative_experience_reward(100), 166);

    assert!(game.gain_mutation("rfb.mutation.fleet-of-foot", &mut Vec::new()));
    assert!(game.gain_mutation("rfb.mutation.limp", &mut Vec::new()));
    assert_eq!(
        game.player_mutation_action_energy_cost(
            &GameAction::Move {
                direction: Direction::North,
            },
            STANDARD_ACTION_COST,
        ),
        66
    );
    let world_walking_cost = STANDARD_ACTION_COST * wilderness::WORLD_MAP_ACTION_MULTIPLIER;
    assert_eq!(
        game.player_mutation_action_energy_cost(
            &GameAction::TravelWorld {
                destination: Position { x: 1, y: 1 },
            },
            world_walking_cost,
        ),
        (world_walking_cost * 10 / 9) * 3 / 5
    );

    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");
    assert!(game.gain_mutation("rfb.mutation.speed-reader", &mut Vec::new()));
    assert_eq!(
        game.player_mutation_action_energy_cost(
            &GameAction::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: None,
                target_glyph: None,
            },
            STANDARD_ACTION_COST,
        ),
        STANDARD_ACTION_COST / 2
    );
}

#[test]
fn new_life_is_one_seeded_transaction_with_locked_mutation_protection() {
    const ITEM_ID: &str = "test.item.new-life.1";
    const KIND_ID: &str = "demo.item.new-life-potion";

    let mut game =
        Game::new_with_build(705, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.apply_unscaled_player_experience(experience_required_for_level(25), &mut Vec::new());

    let previous_attribute_max_hp = game.effective_player_max_hp();
    let previous_attribute_resources = game.player_resource_maxima();
    game.progress.attributes = game.progress.attribute_potentials;
    game.progress.maximum_attributes = game.progress.attribute_potentials;
    game.refresh_after_attribute_change(previous_attribute_max_hp, &previous_attribute_resources);
    for mutation_id in [
        "rfb.mutation.hyper-str",
        "rfb.mutation.br-fire",
        "rfb.mutation.spit-acid",
        "rfb.mutation.puny",
    ] {
        game.progress
            .active_mutation_ids
            .insert(mutation_id.to_owned());
    }
    game.progress
        .locked_mutation_ids
        .insert("rfb.mutation.puny".to_owned());
    game.progress.life_force = 125;

    let previous_max_hp = game.effective_player_max_hp();
    game.player.hp = previous_max_hp.saturating_mul(3) / 5;
    let previous_hp = game.player.hp;
    let previous_resources = game.player_resource_maxima();
    for pool in game.resources.values_mut() {
        pool.current = pool.maximum / 3;
    }
    let previous_resource_currents = game.player_resource_maxima();

    let mut expected_rng = game.rng.clone();
    let expected_hp_progression =
        CharacterProgress::roll_hp_progression(game.progress.hp_progression[0], &mut expected_rng);
    let expected_potentials = CharacterProgress::roll_attribute_potentials(&mut expected_rng);
    let previous_maximum_attributes = game.progress.maximum_attributes;
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.progress.hp_progression, expected_hp_progression);
    assert_eq!(game.progress.attribute_potentials, expected_potentials);
    assert_eq!(game.progress.life_force, 1_000);
    for kind in [
        AttributeKind::Strength,
        AttributeKind::Intelligence,
        AttributeKind::Wisdom,
        AttributeKind::Dexterity,
        AttributeKind::Constitution,
        AttributeKind::Charisma,
    ] {
        let expected = previous_maximum_attributes
            .value(kind)
            .min(expected_potentials.value(kind));
        assert_eq!(game.progress.attributes.value(kind), expected);
        assert_eq!(game.progress.maximum_attributes.value(kind), expected);
    }
    assert_eq!(
        game.progress.active_mutation_ids,
        BTreeSet::from(["rfb.mutation.puny".to_owned()])
    );
    assert_eq!(
        update
            .events
            .iter()
            .filter(|event| event.kind == "mutation.lost")
            .map(|event| event.args["target"].clone())
            .collect::<Vec<_>>(),
        [
            "rfb.mutation.spit-acid",
            "rfb.mutation.br-fire",
            "rfb.mutation.hyper-str",
        ]
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.use-restoration")
    );

    let next_max_hp = game.effective_player_max_hp();
    assert_eq!(
        game.player.hp,
        previous_hp.saturating_mul(next_max_hp) / previous_max_hp
    );
    for (resource_id, (previous_current, previous_maximum)) in previous_resource_currents {
        let pool = &game.resources[&resource_id];
        let expected_current = u32::try_from(
            u64::from(previous_current) * u64::from(pool.maximum) / u64::from(previous_maximum),
        )
        .expect("resource scaling must fit u32");
        assert_eq!(pool.current, expected_current);
    }
    assert_eq!(
        game.item_knowledge_dto(KIND_ID),
        rfb_protocol::ItemKnowledgeDto::Aware
    );
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    let restored = Game::from_save(game.to_save()).expect("New Life result should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());

    assert!(!previous_resources.is_empty());
}

#[test]
fn random_mutation_transactions_are_weighted_and_empty_candidates_use_no_rng() {
    let mut weighted =
        game_with_mutation_weights(&[("rfb.mutation.spit-acid", 1), ("rfb.mutation.br-fire", 3)]);
    let mut expected_rng = RfbRng::seeded(19);
    let expected = if expected_rng.bounded(4) == 0 {
        "rfb.mutation.spit-acid"
    } else {
        "rfb.mutation.br-fire"
    };
    weighted.rng = RfbRng::seeded(19);
    let gained = weighted
        .gain_random_mutation(&mut Vec::new())
        .expect("weighted candidates should select");
    assert_eq!(gained, expected);
    assert_eq!(weighted.rng.draw_counter, expected_rng.draw_counter);

    let mut empty = game_with_mutation_weights(&[]);
    empty.rng = RfbRng::seeded(23);
    let draws = empty.rng.draw_counter;
    assert_eq!(empty.gain_random_mutation(&mut Vec::new()), None);
    empty
        .progress
        .active_mutation_ids
        .insert("rfb.mutation.spit-acid".to_owned());
    assert_eq!(empty.lose_random_mutation(&mut Vec::new()), None);
    assert_eq!(empty.rng.draw_counter, draws);
}

#[test]
fn locked_mutations_do_not_reduce_regeneration() {
    let mut game = Game::new(0);
    assert_eq!(game.mutation_regeneration_percent(), 100);
    for mutation_id in [
        "rfb.mutation.spit-acid",
        "rfb.mutation.br-fire",
        "rfb.mutation.hypn-gaze",
    ] {
        game.progress
            .active_mutation_ids
            .insert(mutation_id.to_owned());
    }
    game.progress
        .locked_mutation_ids
        .insert("rfb.mutation.spit-acid".to_owned());
    assert_eq!(game.mutation_regeneration_percent(), 80);
    game.progress.locked_mutation_ids = game.progress.active_mutation_ids.clone();
    assert_eq!(game.mutation_regeneration_percent(), 100);
    game.progress.locked_mutation_ids.clear();
    game.progress.active_mutation_ids = game
        .content
        .mutations()
        .take(20)
        .map(|mutation| mutation.id.clone())
        .collect();
    assert_eq!(game.mutation_regeneration_percent(), 10);
}

#[test]
fn unlocked_mutation_count_scales_natural_regeneration() {
    let recovered = |active: usize, locked: usize| {
        let mut game = Game::new(0);
        let ids = game
            .content
            .mutations()
            .filter(|mutation| {
                mutation.modifiers.constitution == 0 && mutation.modifiers.max_hp == 0
            })
            .take(active)
            .map(|mutation| mutation.id.clone())
            .collect::<Vec<_>>();
        game.progress
            .active_mutation_ids
            .extend(ids.iter().cloned());
        game.progress
            .locked_mutation_ids
            .extend(ids.into_iter().take(locked));
        game.progress.hp_progression[0] = 10_000;
        game.player.hp = 1;
        game.world_tick = NATURAL_HP_REGENERATION_INTERVAL_TICKS;
        game.process_natural_hp_regeneration(false);
        game.player.hp - 1
    };

    let normal = recovered(0, 0);
    assert!(normal > 0);
    assert!(recovered(5, 0) < normal);
    assert_eq!(recovered(5, 5), normal);
}

#[test]
fn build_skill_growth_experience_multiplier_and_save_identity_are_deterministic() {
    let mut vanguard =
        Game::new_with_build(17, "demo.build.vanguard").expect("vanguard build should create");
    vanguard.apply_player_experience(380, &mut Vec::new());
    assert_eq!(vanguard.progress.level, 10);
    assert_eq!(
        vanguard
            .progress
            .skill("demo.skill.melee")
            .map(|skill| skill.current),
        Some(105)
    );

    let mut scholar =
        Game::new_with_build(17, "demo.build.scholar").expect("scholar build should create");
    scholar.apply_player_experience(100, &mut Vec::new());
    assert_eq!(scholar.progress.experience, 156);

    let restored = Game::from_save(vanguard.to_save()).expect("build save should reload");
    assert_eq!(restored.build, vanguard.build);
    assert_eq!(restored.progress.skills, vanguard.progress.skills);
    assert_eq!(restored.snapshot(), vanguard.snapshot());
    assert!(matches!(
        Game::new_with_build(17, "demo.build.missing"),
        Err(CoreError::UnknownCharacterBuild(_))
    ));
}

#[test]
fn attribute_increase_command_commits_growth_without_rng_or_world_progression() {
    let mut game =
        Game::new_with_build(96, "demo.build.scholar").expect("scholar build should create");
    game.apply_player_experience(100, &mut Vec::new());
    assert!(game.progress.pending_attribute_increases > 0);

    let resource = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana");
    resource.current = resource.maximum / 3;
    let resource_before = *resource;
    let natural_before = game.progress.attributes.intelligence;
    let pending_before = game.progress.pending_attribute_increases;
    let draws_before = game.rng_draw_counter();
    let world_tick_before = game.world_tick;
    let energy_before = game.player.energy_need;
    let turn_before = game.turn;

    let update = dispatch_next(
        &mut game,
        GameCommand::IncreaseAttribute {
            attribute: AttributeKindDto::Intelligence,
        },
    );

    let resource_after = game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert!(game.progress.attributes.intelligence > natural_before);
    assert_eq!(
        game.progress.pending_attribute_increases,
        pending_before - 1
    );
    assert!(resource_after.maximum > resource_before.maximum);
    assert_eq!(
        resource_after.current,
        u32::try_from(
            u64::from(resource_before.current) * u64::from(resource_after.maximum)
                / u64::from(resource_before.maximum)
        )
        .expect("scaled resource value should fit u32")
    );
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.world_tick, world_tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert_eq!(game.turn, turn_before + 1);
    assert_eq!(update.events.len(), 1);
    assert_eq!(update.events[0].kind, "player.attribute-increased");
    assert_eq!(
        update.events[0].args.get("pendingAttributeIncreases"),
        Some(&game.progress.pending_attribute_increases.to_string())
    );
}

#[test]
fn unavailable_attribute_increase_rejects_without_mutation_or_rng() {
    let mut game = Game::new(42);
    assert_eq!(game.progress.pending_attribute_increases, 0);
    let progress_before = game.progress.clone();
    let resources_before = game.resources.clone();
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let world_tick_before = game.world_tick;
    let energy_before = game.player.energy_need;

    let update = dispatch_next(
        &mut game,
        GameCommand::IncreaseAttribute {
            attribute: AttributeKindDto::Strength,
        },
    );

    assert_eq!(game.progress, progress_before);
    assert_eq!(game.resources, resources_before);
    assert_eq!(game.player.hp, hp_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.world_tick, world_tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert!(update.changed_cells.is_empty());
    assert_eq!(update.events.len(), 1);
    assert_eq!(
        update.events[0].kind,
        "player.attribute-increase-unavailable"
    );
}

#[test]
fn restore_life_uses_historical_experience_and_migrates_old_saves() {
    let mut game = prepare_death_caster(0, 42, "demo.ability.death-restore-life");
    game.progress.experience = 500;
    game.progress.maximum_experience = 900;
    game.progress.life_force = 125;
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.death-restore-life",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Restore Life should resolve");
    assert_eq!(game.progress.experience, 900);
    assert_eq!(game.progress.maximum_experience, 900);
    assert_eq!(game.progress.life_force, 1_000);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality {
                    experience_before: 500,
                    experience_after: 900,
                    life_force_before: 125,
                    life_force_after: 1_000,
                    ..
                }]
            )
    )));

    let mut legacy = Game::new(0);
    legacy.apply_player_experience(10, &mut Vec::new());
    let expected = legacy.progress.experience;
    let mut payload = legacy.to_save();
    payload
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved")
        .maximum_experience = 0;
    let migrated = Game::from_save(payload).expect("old progress should migrate");
    assert_eq!(migrated.progress.maximum_experience, expected);
}

#[test]
fn attribute_history_migrates_old_saves_and_rejects_inverted_values() {
    let mut legacy = Game::new(0).to_save();
    let progress = legacy
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved");
    let strength = progress.attributes.strength;
    progress.maximum_attributes = None;
    let migrated = Game::from_save(legacy).expect("old progress should migrate");
    assert_eq!(migrated.progress.maximum_attributes.strength, strength);

    let mut invalid = migrated.to_save();
    let progress = invalid
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved");
    let mut maximum = progress.attributes;
    maximum.strength = progress.attributes.strength.saturating_sub(1);
    progress.maximum_attributes = Some(maximum);
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("player attribute state is invalid"))
    ));
}

#[test]
fn attribute_resource_refresh_scales_the_prechange_current_value_once() {
    let mut game =
        Game::new_with_build(96, "demo.build.scholar").expect("scholar build should create");
    let before = *game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should have mana");
    assert_eq!(before.current, before.maximum);

    assert!(game.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Intelligence,
        &mut Vec::new(),
    ));
    let drained = *game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert!(drained.maximum < before.maximum);
    assert_eq!(drained.current, drained.maximum);

    assert!(game.resolve_item_restore_attribute(
        "demo.item.intelligence-renewal-tonic",
        AttributeKind::Intelligence,
        &mut Vec::new(),
    ));
    let restored = game
        .resources
        .get("demo.resource.mana")
        .expect("scholar should retain mana");
    assert_eq!(restored.maximum, before.maximum);
    assert_eq!(restored.current, before.current);
}
