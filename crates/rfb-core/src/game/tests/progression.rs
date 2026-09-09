// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::stats::{AttributeSet, modify_attribute_value};
use rfb_protocol::{AbilityEffectSpecDto, AbilitySourceDto, AttributeKindDto, MutationRatingDto};

const TEST_RACE_REWARD_BUILD_ID: &str = "test.build.race-rewards";
const TEST_RACE_REWARD_CASTER_BUILD_ID: &str = "test.build.caster";
const TEST_RACE_CHOICE_REWARD_ID: &str = "test-talent";
const TEST_RACE_CHOICE_MUTATION_ID: &str = "rfb.mutation.ambidextrous";
const TEST_RACE_DEFAULT_MUTATION_ID: &str = "rfb.mutation.black-marketeer";
const TEST_RACE_INT_MUTATION_ID: &str = "rfb.mutation.astral-guide";
const TEST_RACE_OVERRIDE_ABILITY_ID: &str = "test.ability.race-mutation-override";

fn race_reward_catalog() -> Arc<rfb_content::ContentCatalog> {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    enable_test_caster(&mut artifact.content);

    let mut race = artifact
        .content
        .races
        .iter()
        .find(|race| race.id == "demo.race.rfb-human")
        .expect("Human race should exist")
        .clone();
    race.id = "test.race.level-mutation-rewards".to_owned();
    race.legacy_index = None;
    race.name_key = "test-race-level-mutation-rewards-name".to_owned();
    race.description_key = "test-race-level-mutation-rewards-description".to_owned();
    race.armor_class = 7;
    race.levitation = true;
    race.reflects_bolts_minimum_level = Some(2);
    race.level_mutation_rewards = vec![
        rfb_content::RaceLevelMutationRewardDefinition {
            id: "test-weakness".to_owned(),
            minimum_level: 3,
            selection: rfb_content::RaceMutationSelectionDefinition::CastingAttribute {
                default_mutation_id: TEST_RACE_DEFAULT_MUTATION_ID.to_owned(),
                mutation_ids_by_attribute: BTreeMap::from([(
                    rfb_content::CastingAttribute::Intelligence,
                    TEST_RACE_INT_MUTATION_ID.to_owned(),
                )]),
            },
        },
        rfb_content::RaceLevelMutationRewardDefinition {
            id: TEST_RACE_CHOICE_REWARD_ID.to_owned(),
            minimum_level: 2,
            selection: rfb_content::RaceMutationSelectionDefinition::Choice {
                mutation_ids: vec![
                    TEST_RACE_CHOICE_MUTATION_ID.to_owned(),
                    "rfb.mutation.evasion".to_owned(),
                ],
            },
        },
    ];
    race.mutation_overrides.insert(
        TEST_RACE_CHOICE_MUTATION_ID.to_owned(),
        rfb_content::RaceMutationOverrideDefinition {
            description: Some("Race-specific mutation behavior".to_owned()),
            activation: Some(rfb_content::InnatePowerDefinition {
                minimum_level: 1,
                governing_attribute: rfb_content::TechniqueAttribute::Constitution,
                cost: 3,
                cost_scaling: None,
                base_failure_percent: 20,
                minimum_failure_percent: None,
                ability_id: TEST_RACE_OVERRIDE_ABILITY_ID.to_owned(),
            }),
            armor_class: Some(9),
            resistances: Some(BTreeMap::from([(
                rfb_content::ActorDamageType::Fire,
                rfb_content::ActorResistanceLevel::Resistant,
            )])),
            contact_aura: Some(rfb_content::ActorDamageType::Fire),
        },
    );
    race.mutation_choice_exclusions_by_class.insert(
        "demo.class.archer".to_owned(),
        BTreeSet::from([TEST_RACE_CHOICE_MUTATION_ID.to_owned()]),
    );
    artifact.content.races.push(race);

    let mut override_ability = artifact
        .content
        .abilities
        .iter()
        .find(|ability| ability.id == "rfb.ability.mutation.cold-touch")
        .expect("Cold Touch ability should exist")
        .clone();
    override_ability.id = TEST_RACE_OVERRIDE_ABILITY_ID.to_owned();
    override_ability.name_key = "test-race-mutation-override-name".to_owned();
    override_ability.description_key = "test-race-mutation-override-description".to_owned();
    artifact.content.abilities.push(override_ability);

    let mut build = artifact
        .content
        .builds
        .iter()
        .find(|build| build.id == "demo.build.warrior")
        .expect("Warrior build should exist")
        .clone();
    build.id = TEST_RACE_REWARD_BUILD_ID.to_owned();
    build.name_key = "test-build-race-rewards-name".to_owned();
    build.description_key = "test-build-race-rewards-description".to_owned();
    build.race_id = "test.race.level-mutation-rewards".to_owned();
    artifact.content.builds.push(build);
    artifact
        .content
        .builds
        .iter_mut()
        .find(|build| build.id == TEST_RACE_REWARD_CASTER_BUILD_ID)
        .expect("test caster build should exist")
        .race_id = "test.race.level-mutation-rewards".to_owned();
    artifact
        .content
        .builds
        .iter_mut()
        .find(|build| build.id == "demo.build.archer")
        .expect("Archer build should exist")
        .race_id = "test.race.level-mutation-rewards".to_owned();

    Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("test race reward content should remain valid"),
    ))
}

#[test]
fn birth_race_passives_mutation_overrides_and_class_exclusions_are_resolved() {
    let catalog = race_reward_catalog();
    let mut game = Game::from_content_with_build(
        47,
        catalog.clone(),
        DEFAULT_WORLD_ID,
        TEST_RACE_REWARD_BUILD_ID,
    )
    .expect("race override game should create");
    let mut control =
        Game::from_content_with_build(47, catalog.clone(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .expect("control game should create");
    assert!(game.player_levitates());
    assert!(!game.player_reflects_bolts());

    game.apply_player_experience(game.experience_required_for_level(2), &mut Vec::new());
    control.apply_player_experience(control.experience_required_for_level(2), &mut Vec::new());
    assert!(game.player_reflects_bolts());
    let race_armor = game.player_derived_stats().armor_class.value;
    assert_eq!(
        race_armor,
        control.player_derived_stats().armor_class.value + 7
    );

    assert!(game.choose_race_mutation(
        TEST_RACE_CHOICE_REWARD_ID,
        TEST_RACE_CHOICE_MUTATION_ID,
        &mut Vec::new(),
    ));
    assert_eq!(
        game.player_derived_stats().armor_class.value,
        race_armor + 9
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Resistant
    );
    let snapshot = game.snapshot();
    let mutation = snapshot
        .player
        .mutations
        .iter()
        .find(|mutation| mutation.id == TEST_RACE_CHOICE_MUTATION_ID)
        .expect("chosen mutation should be projected");
    assert_eq!(mutation.description, "Race-specific mutation behavior");
    assert!(snapshot.player.abilities.iter().any(|ability| {
        ability.id == TEST_RACE_OVERRIDE_ABILITY_ID
            && ability.source == AbilitySourceDto::Mutation
            && ability.resource_cost == 3
    }));

    let mut archer =
        Game::from_content_with_build(47, catalog, DEFAULT_WORLD_ID, "demo.build.archer")
            .expect("Archer race override game should create");
    archer.apply_player_experience(archer.experience_required_for_level(2), &mut Vec::new());
    let pending = archer
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Archer should retain an eligible choice");
    assert_eq!(
        pending
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        ["rfb.mutation.evasion"]
    );
}

fn race_reward_game(build_id: &str) -> Game {
    Game::from_content_with_build(47, race_reward_catalog(), DEFAULT_WORLD_ID, build_id)
        .expect("test race reward game should create")
}

fn draconian_reward_game_for_build(build_id: &str) -> Game {
    Game::new_with_build_race_and_name(
        3535,
        build_id,
        "rfb-legacy.race.draconian-red",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal red Draconian should create")
}

fn draconian_reward_game() -> Game {
    draconian_reward_game_for_build("demo.build.high-mage-death")
}

fn species_contribution(stat: &DerivedStat, race_id: &str) -> i32 {
    stat.contributions
        .iter()
        .find(|contribution| contribution.source_id == race_id)
        .map_or(0, |contribution| contribution.amount)
}

#[test]
fn tomte_headgear_penalties_follow_weight_boundaries_and_effective_race() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut content = rfb_content::compile_pack_dir(&path).unwrap().content;
    enable_test_caster(&mut content);
    let helmet = content
        .items
        .iter()
        .find(|item| item.id == "demo.item.iron-helm")
        .unwrap()
        .clone();
    let cases = [
        (8, 0, 0),
        (10, 0, 0),
        (11, 1, 3),
        (12, 1, 4),
        (19, 1, 7),
        (20, 2, 8),
        (21, 2, 8),
        (75, 7, 35),
    ];
    for (weight, _, _) in cases {
        let mut item = helmet.clone();
        item.id = format!("test.item.headgear-{weight}");
        item.weight_tenths_pound = weight;
        content.items.push(item);
    }
    // Crowns use the same head slot as helmets; no crown is imported yet.
    let mut crown = helmet;
    crown.id = "test.item.crown".to_owned();
    crown.weight_tenths_pound = 12;
    content.items.push(crown);
    let catalog = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(content).unwrap(),
    ));
    let mut base =
        Game::from_content_with_build(424, catalog, DEFAULT_WORLD_ID, "test.build.caster").unwrap();
    base.progress.attributes.intelligence = 10;
    base.items.clear();
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tomte-headgear").status;
    form.granted_race_id = Some("rfb-legacy.race.tomte".to_owned());
    base.player.statuses.push(form);
    for (kind, int_penalty, device_penalty) in cases
        .into_iter()
        .map(|(weight, intelligence, device)| {
            (format!("test.item.headgear-{weight}"), intelligence, device)
        })
        .chain([("test.item.crown".to_owned(), 1, 4)])
    {
        let mut game = base.clone();
        give_inventory_item(&mut game, "test.headgear", &kind);
        assert_eq!(
            game.player_tomte_headgear_excess_weight(),
            0,
            "carried {kind}"
        );
        assert!(
            game.equip_inventory_item("test.headgear", Some("head"))
                .is_some()
        );
        assert_eq!(
            game.effective_player_attributes().intelligence,
            15 - int_penalty,
            "{kind}"
        );
        let mut rows = Vec::new();
        game.player_attributes_with_sources(Some(&mut rows));
        let race = rows
            .iter()
            .find(|row| row.attribute == AttributeKindDto::Intelligence)
            .unwrap()
            .sources
            .iter()
            .find(|source| source.kind == rfb_protocol::AttributeSourceKindDto::Race)
            .unwrap();
        assert_eq!(race.modifier, 2 - i32::from(int_penalty), "{kind}");
        assert_eq!(
            game.snapshot().player.trait_details.tomte_heavy_headgear,
            Some(int_penalty > 0)
        );
        assert_eq!(
            species_contribution(
                &game.player_derived_stats().device_skill,
                "rfb-legacy.race.tomte"
            ),
            15 - device_penalty,
            "{kind}"
        );

        let equipped = game.clone();
        assert!(game.unequip_slot("head").is_some());
        assert_eq!(
            game.snapshot().player.trait_details.tomte_heavy_headgear,
            Some(false)
        );
        assert_eq!(game.effective_player_attributes().intelligence, 15);
        assert_eq!(
            species_contribution(
                &game.player_derived_stats().device_skill,
                "rfb-legacy.race.tomte"
            ),
            15
        );
        game = equipped;
        game.player.statuses.clear();
        assert_eq!(
            game.snapshot().player.trait_details.tomte_heavy_headgear,
            None
        );
        assert_eq!(
            game.player_tomte_headgear_excess_weight(),
            0,
            "form ended: {kind}"
        );
        assert_eq!(game.effective_player_attributes().intelligence, 13);
    }
}

#[test]
fn tomte_heavy_helmet_updates_casting_and_preserves_other_auto_identification() {
    let mut game = test_caster_game(424);
    clear_monsters(&mut game);
    game.progress.attributes.intelligence = 10;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tomte-casting").status;
    form.granted_race_id = Some("rfb-legacy.race.tomte".to_owned());
    game.player.statuses.push(form);
    game.player.hp = game.effective_player_max_hp();
    game.refresh_player_ability_state();
    let probe_failure = |game: &Game| {
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "rfb.ability.race.probe-monsters")
            .unwrap()
            .failure_percent
    };
    let light_failure = probe_failure(&game);
    let light_mana = game.resources["demo.resource.mana"].maximum;
    give_inventory_item(&mut game, "test.heavy-helmet", "demo.item.iron-helm");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.heavy-helmet".to_owned(),
            slot_id: Some("head".to_owned()),
        },
    );
    assert_eq!(game.effective_player_attributes().intelligence, 8);
    assert!(probe_failure(&game) > light_failure);
    assert!(game.resources["demo.resource.mana"].maximum < light_mana);
    assert!(
        game.resources["demo.resource.mana"].current
            <= game.resources["demo.resource.mana"].maximum
    );
    let heavy = game.clone();
    dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "head".to_owned(),
        },
    );
    assert_eq!(probe_failure(&game), light_failure);
    assert_eq!(game.resources["demo.resource.mana"].maximum, light_mana);

    for mutation in [true, false] {
        let mut game = heavy.clone();
        give_inventory_item(
            &mut game,
            "test.identification-target",
            "demo.item.iron-helm",
        );
        assert!(!game.player_auto_identifies_items());
        if mutation {
            game.progress
                .active_mutation_ids
                .insert("rfb.mutation.draconian-lore".to_owned());
            assert!(game.player_auto_identifies_items());
        } else {
            game.player.statuses.push(
                monster_combat::melee_status(STATUS_UNDERSTANDING, 20, "test.understanding").status,
            );
        }
        dispatch_next(&mut game, GameCommand::Wait);
        assert!(game.item_property_knowledge["test.identification-target"].appraised);
        assert_eq!(
            game.item_knowledge_dto("demo.item.iron-helm"),
            ItemKnowledgeDto::Aware
        );
        assert_eq!(game.player_tomte_headgear_excess_weight(), 65);
    }
}

#[test]
fn tomte_form_grants_intrinsics_and_free_probing() {
    const RACE: &str = "rfb-legacy.race.tomte";
    const PROBE: &str = "rfb.ability.race.probe-monsters";
    let mut game = Game::new_with_build_race_and_name(
        424,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior");
    clear_monsters(&mut game);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tomte-form").status;
    form.granted_race_id = Some(RACE.to_owned());
    game.player.statuses.push(form);
    game.player.hp = game.effective_player_max_hp();
    assert_eq!(game.player_infravision_range(), 4);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    for (level, speed) in [
        (1, 0),
        (14, 0),
        (15, 1),
        (29, 1),
        (30, 2),
        (44, 2),
        (45, 3),
        (50, 3),
    ] {
        game.progress.level = level;
        assert_eq!(
            species_contribution(&game.player_derived_stats().speed, RACE),
            speed
        );
    }
    game.progress.level = 1;
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == PROBE)
        .expect("Tomte probe at level one");
    assert_eq!(ability.source, AbilitySourceDto::Race);
    assert_eq!(
        ability.governing_attribute,
        Some(rfb_protocol::AttributeKindDto::Intelligence)
    );
    assert_eq!(
        (
            ability.minimum_level,
            ability.base_resource_cost,
            ability.resource_cost
        ),
        (1, 0, 0)
    );
    assert!(ability.can_cast);
    let target = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, target, "demo.terrain.floor");
    let index = game.index(target).expect("target tile");
    game.glow[index] = true;
    game.push_generated_actor("test.tomte-probe".to_owned(), "demo.actor.sheep", target);
    let hp = game.player.hp;
    let resources = game.resources.clone();
    let failure_seed = (0..1_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(ability.failure_percent))
        .expect("probe can fail");
    game.rng = RfbRng::seeded(failure_seed);
    let mut events = Vec::new();
    game.resolve_player_ability(
        PROBE,
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("failed probe");
    assert!(matches!(
        events.first(),
        Some(DomainEvent::AbilityCastFailed { .. })
    ));
    assert!(game.probed_actor_kind_ids.is_empty());
    assert_eq!(game.player.hp, hp);
    assert_eq!(game.resources, resources);
    game.debug_set_ability_casts_succeed(true);
    game.resolve_player_ability(
        PROBE,
        TargetSelection::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("successful probe");
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    assert_eq!(game.player.hp, hp);
    assert_eq!(game.resources, resources);
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("temporary Tomte save");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.player_infravision_range(), 4);
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert_eq!(game.player_infravision_range(), 0);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Normal
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != PROBE)
    );
    for level in [15, 30, 45] {
        game.progress.level = level;
        assert_eq!(
            species_contribution(&game.player_derived_stats().speed, RACE),
            0
        );
    }
    game.progress.level = 1;
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("knowledge persists after losing Tomte form");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.probed_actor_kind_ids.contains("demo.actor.sheep"));
}

#[test]
fn tonberry_passives_and_level_slowing_follow_the_effective_race() {
    const RACE: &str = "rfb-legacy.race.tonberry";
    const HUMAN: &str = "demo.race.rfb-human";
    let assert_intrinsics = |game: &Game, active: bool, speed: i32| {
        assert_eq!(
            species_contribution(&game.player_derived_stats().speed, RACE),
            speed,
            "Tonberry speed at level {}",
            game.progress.level
        );
        assert_eq!(game.player_infravision_range(), if active { 2 } else { 0 });
        assert_eq!(
            game.effective_player_resistances().level(DamageType::Fear),
            if active {
                ResistanceLevel::Resistant
            } else {
                ResistanceLevel::Normal
            }
        );
        for attribute in [AttributeKind::Strength, AttributeKind::Constitution] {
            assert_eq!(game.player_sustains_attribute(attribute), active);
        }
        assert!(!game.player_sustains_attribute(AttributeKind::Dexterity));
        let details = game.character_trait_details(&game.player_derived_stats());
        let source = details
            .sources
            .iter()
            .find(|source| source.source_id == RACE);
        assert_eq!(source.is_some(), active);
        if let Some(source) = source {
            assert!(
                source
                    .passives
                    .contains(&rfb_protocol::EquipmentPassiveDto::SustainStrength)
            );
            assert!(
                source
                    .passives
                    .contains(&rfb_protocol::EquipmentPassiveDto::SustainConstitution)
            );
            assert!(source.resistances.iter().any(|resistance| {
                resistance.damage_type == rfb_protocol::DamageTypeDto::Fear
                    && resistance.level == rfb_protocol::ResistanceLevelDto::Resistant
            }));
        }
    };
    for native in [false, true] {
        let mut game = Game::new_with_build_race_and_name(
            425,
            "demo.build.warrior",
            HUMAN,
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("Human warrior");
        game.items.clear();
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.tonberry-form").status;
        form.granted_race_id = Some(RACE.to_owned());
        if native {
            // Direct native-race precondition isolates the intrinsic comparison from birth rolls.
            game.build.as_mut().unwrap().race_id = RACE.to_owned();
        } else {
            game.player.statuses.push(form.clone());
        }
        assert_intrinsics(&game, true, -1);
        let attributes = game.progress.attributes;
        let rng = game.rng.clone();
        game.resolve_monster_attribute_drain(AttributeKind::Strength);
        game.resolve_monster_attribute_drain(AttributeKind::Constitution);
        assert_eq!(game.progress.attributes, attributes);
        assert_eq!(game.rng, rng, "sustained drains consume no RNG");

        for (level, speed) in [
            (29, -1),
            (30, -2),
            (39, -2),
            (40, -3),
            (44, -3),
            (45, -4),
            (49, -4),
            (50, -5),
        ] {
            game.apply_player_experience(
                game.experience_required_for_level(level) - game.progress.experience,
                &mut Vec::new(),
            );
            assert_eq!(game.progress.level, level);
            assert_intrinsics(&game, true, speed);
        }

        game.player.statuses.clear();
        if native {
            form.granted_race_id = Some(HUMAN.to_owned());
            game.player.statuses.push(form);
        }
        assert_intrinsics(&game, false, 0);
        if native {
            game.player.statuses.clear();
            assert_intrinsics(&game, true, -5);
        }
    }
}

fn ent_passive_game(native: bool) -> Game {
    let mut game = Game::new_with_build(426, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    if native {
        // Birth remains closed until the Ent's supplies and power are implemented.
        game.build.as_mut().unwrap().race_id = "rfb-legacy.race.ent".to_owned();
    } else {
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100_000, "test.ent").status;
        form.granted_race_id = Some("rfb-legacy.race.ent".to_owned());
        game.player.statuses.push(form);
    }
    game
}

#[test]
fn ent_growth_tracks_level_thresholds_drain_and_current_form() {
    let check_growth = |game: &Game, growth: i32| {
        let cap = CharacterProgress::attribute_cap(false);
        let mut rows = Vec::new();
        let attributes = game.player_attributes_with_sources(Some(&mut rows));
        for (kind, modifier) in [
            (AttributeKindDto::Strength, 2 + growth),
            (AttributeKindDto::Constitution, 2 + growth),
            (AttributeKindDto::Dexterity, -3 - growth),
        ] {
            let row = rows.iter().find(|row| row.attribute == kind).unwrap();
            let source = row
                .sources
                .iter()
                .find(|source| source.source_id.as_deref() == Some("rfb-legacy.race.ent"))
                .unwrap();
            assert_eq!(
                source.modifier, modifier,
                "level {}, {kind:?}",
                game.progress.level
            );
            assert!(source.complete);
        }
        assert_eq!(
            attributes.strength,
            modify_attribute_value(
                modify_attribute_value(game.progress.attributes.strength, 2 + growth, cap),
                4,
                cap
            )
        );
        assert_eq!(
            attributes.constitution,
            modify_attribute_value(
                modify_attribute_value(game.progress.attributes.constitution, 2 + growth, cap),
                2,
                cap
            )
        );
        assert_eq!(
            attributes.dexterity,
            modify_attribute_value(
                modify_attribute_value(game.progress.attributes.dexterity, -3 - growth, cap),
                2,
                cap
            )
        );
        assert_eq!(
            game.effective_player_resistances()
                .level(DamageType::Poison),
            ResistanceLevel::Resistant
        );
        assert_eq!(
            game.effective_player_max_hp(),
            game.player_max_hp_at_level(game.progress.level)
        );
    };
    for native in [false, true] {
        let mut game = ent_passive_game(native);
        for (level, growth) in [
            (1, 0),
            (25, 0),
            (26, 1),
            (40, 1),
            (41, 2),
            (45, 2),
            (46, 3),
            (50, 3),
        ] {
            game.apply_player_experience(
                game.experience_required_for_level(level) - game.progress.experience,
                &mut Vec::new(),
            );
            assert_eq!(game.progress.level, level);
            check_growth(&game, growth);
        }
        for (level, growth) in [(45, 2), (40, 1), (25, 0)] {
            game.apply_player_experience_drain(
                game.progress.experience - game.experience_required_for_level(level),
                "test.ent-drain",
                &mut Vec::new(),
            );
            assert_eq!(game.progress.level, level);
            check_growth(&game, growth);
        }
        let mut human = game.clone();
        human.player.statuses.clear();
        human.build.as_mut().unwrap().race_id = "demo.race.rfb-human".to_owned();
        if native {
            let mut form =
                monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100_000, "test.human").status;
            form.granted_race_id = Some("demo.race.rfb-human".to_owned());
            game.player.statuses.push(form);
        } else {
            game.player.statuses.clear();
        }
        assert_eq!(
            game.effective_player_attributes(),
            human.effective_player_attributes()
        );
        assert_eq!(
            game.effective_player_max_hp(),
            human.effective_player_max_hp()
        );
        assert_eq!(game.player_resistance_percent(DamageType::Poison), 0);
    }
}

#[test]
fn ent_level_events_use_each_levels_constitution_and_form_round_trips() {
    let mut game = ent_passive_game(false);
    let mut events = Vec::new();
    game.apply_player_experience(game.experience_required_for_level(46), &mut events);
    for level in [25, 26, 40, 41, 45, 46] {
        let mut at_level = game.clone();
        at_level.progress.level = level;
        let expected = at_level.effective_player_max_hp();
        let reported = events.iter().find_map(|event| match event {
            DomainEvent::PlayerLevelGained {
                level: reported_level,
                max_hp,
                ..
            } if *reported_level == level => Some(*max_hp),
            _ => None,
        });
        assert_eq!(reported, Some(expected), "HP reported at level {level}");
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(
        restored.effective_player_attributes(),
        game.effective_player_attributes()
    );
}

#[test]
fn ent_digging_bonus_reacts_to_weapons_tools_and_form_changes() {
    for native in [false, true] {
        let mut game = ent_passive_game(native);
        // Keep the Human control below its unrelated level-20 talent choice.
        game.progress.level = 19;
        let bonus = |game: &Game| {
            species_contribution(
                &game.player_derived_stats().dig_skill,
                "rfb-legacy.race.ent",
            )
        };
        assert_eq!(bonus(&game), 190);
        for (id, kind, slot, expected) in [
            (
                "test.ent.shield",
                "demo.item.small-leather-shield",
                "left-hand",
                190,
            ),
            ("test.ent.bow", "demo.item.short-bow", "shooting", 190),
            ("test.ent.sword", "demo.item.broad-sword", "right-hand", 0),
            ("test.ent.shovel", "demo.item.shovel", "tool", 0),
        ] {
            give_inventory_item(&mut game, id, kind);
            dispatch_next(
                &mut game,
                GameCommand::Equip {
                    item_id: id.to_owned(),
                    slot_id: Some(slot.to_owned()),
                },
            );
            assert!(
                matches!(&game.items.iter().find(|item| item.id == id).unwrap().location,
                ItemLocation::Equipped { slot_id } if slot_id == slot)
            );
            assert_eq!(bonus(&game), expected, "equipped {kind}");
            dispatch_next(
                &mut game,
                GameCommand::Unequip {
                    slot_id: slot.to_owned(),
                },
            );
            assert_eq!(bonus(&game), 190, "unequipped {kind}");
        }
        game.progress.level = 1;
        assert_eq!(bonus(&game), 10);
        game.player.statuses.clear();
        game.build.as_mut().unwrap().race_id = "demo.race.rfb-human".to_owned();
        assert_eq!(bonus(&game), 0);
    }
}

#[test]
fn ent_and_wood_elf_tree_travel_preserves_normal_cost_on_foot_and_mounted() {
    for race in ["rfb-legacy.race.ent", "rfb-legacy.race.wood-elf"] {
        for mounted in [false, true] {
            let mut game = ent_passive_game(false);
            game.player.statuses[0].granted_race_id = Some(race.to_owned());
            let start = Position { x: 48, y: 16 };
            let target = Position { x: 49, y: 16 };
            game.player.position = start;
            replace_terrain(&mut game, start, "demo.terrain.floor");
            replace_terrain(&mut game, target, "demo.terrain.surface-tree");
            let index = game.index(target).unwrap();
            game.explored[index] = true;
            if mounted {
                game.push_generated_actor("test.ent.mount".to_owned(), "demo.actor.horse", start);
                game.entities[0].controller_id = Some(game.player.id.clone());
                game.riding_actor_id = Some("test.ent.mount".to_owned());
            }
            assert_eq!(
                game.next_local_travel_direction(target),
                Some(Direction::East)
            );
            let mut floor = game.clone();
            replace_terrain(&mut floor, target, "demo.terrain.floor");
            let before = game.world_tick;
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            dispatch_next(
                &mut floor,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(game.player.position, target);
            assert!(game.world_tick > before);
            assert_eq!(
                game.world_tick, floor.world_tick,
                "{race}, mounted={mounted}"
            );
            if mounted {
                assert_eq!(game.entities[0].position, target);
            }
            game.player.position = start;
            if mounted {
                game.entities[0].position = start;
            }
            game.player.statuses.clear();
            assert_eq!(game.next_local_travel_direction(target), None);
            dispatch_next(
                &mut game,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            assert_eq!(game.player.position, start);
        }
    }
}

#[test]
fn ent_forest_adaptation_does_not_allow_an_aquatic_mount_onto_land() {
    let mut game = ent_passive_game(false);
    let start = Position { x: 48, y: 16 };
    let target = Position { x: 49, y: 16 };
    game.player.position = start;
    replace_terrain(&mut game, start, "demo.terrain.surface-water-deep");
    replace_terrain(&mut game, target, "demo.terrain.surface-tree");
    game.push_generated_actor(
        "test.ent.aquatic-mount".to_owned(),
        "demo.actor.hippocampus",
        start,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.ent.aquatic-mount".to_owned());
    let tree = game.content.terrain("demo.terrain.surface-tree").unwrap();
    assert!(!game.player_can_cross_surface_terrain(tree));
    assert!(!game.actor_can_enter_position(0, target));
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, start);
    assert_eq!(game.entities[0].position, start);
}

#[test]
fn race_level_stat_scaling_preserves_klackon_and_enables_formal_golem_intrinsics() {
    for level in [1, 3, 5, 9, 10, 15, 16, 31, 32, 34, 35, 47, 48, 50] {
        let mut golem = golem_game(358);
        if level > 1 {
            golem.apply_player_experience(
                golem.experience_required_for_level(level),
                &mut Vec::new(),
            );
        }
        let stats = golem.player_derived_stats();
        assert_eq!(
            species_contribution(&stats.armor_class, "rfb-legacy.race.golem"),
            10 + i32::from(level) * 2 / 5,
            "Golem armor at level {level}"
        );
        assert_eq!(
            species_contribution(&stats.speed, "rfb-legacy.race.golem"),
            -(i32::from(level) / 16),
            "Golem speed at level {level}"
        );
        assert_eq!(
            golem.player_hold_life_sources(),
            usize::from(level >= 35),
            "Golem hold life at level {level}"
        );
        assert!(golem.player_see_invisible_sources() >= 1);
        assert!(golem.player_status_immunities().contains(STATUS_PARALYSIS));
        assert!(golem.player_status_immunities().contains(STATUS_STUN));
        assert_eq!(
            golem
                .effective_player_resistances()
                .level(DamageType::Poison),
            ResistanceLevel::Resistant
        );
    }

    for (level, expected_speed) in [(9, 0), (10, 1), (19, 1), (20, 2)] {
        let mut klackon = Game::new_with_build_race_and_name(
            358,
            "demo.build.warrior",
            "rfb-legacy.race.klackon",
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("formal Klackon should create");
        klackon.apply_player_experience(
            klackon.experience_required_for_level(level),
            &mut Vec::new(),
        );
        assert_eq!(
            species_contribution(
                &klackon.player_derived_stats().speed,
                "rfb-legacy.race.klackon",
            ),
            expected_speed,
            "Klackon speed at level {level}"
        );
    }
}

#[test]
fn formal_golem_creation_and_temporary_form_apply_and_remove_intrinsics_and_stone_skin() {
    let golem = golem_game(359);
    assert_eq!(
        golem.build.as_ref().expect("formal build identity").race_id,
        "rfb-legacy.race.golem"
    );
    assert!(golem.snapshot().player.abilities.iter().any(|ability| {
        ability.id == "rfb.ability.race.golem-stone-skin"
            && ability.source == AbilitySourceDto::Race
    }));

    let mut human = Game::new_with_build_race_and_name(
        359,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior should create");
    human.progress.level = 20;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.golem-form").status;
    form.granted_race_id = Some("rfb-legacy.race.golem".to_owned());
    human.player.statuses.push(form);

    let stats = human.player_derived_stats();
    assert_eq!(
        species_contribution(&stats.armor_class, "rfb-legacy.race.golem"),
        18
    );
    assert_eq!(
        species_contribution(&stats.speed, "rfb-legacy.race.golem"),
        -1
    );
    assert_eq!(
        human
            .effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );
    assert!(human.player_status_immunities().contains(STATUS_PARALYSIS));
    assert!(human.player_status_immunities().contains(STATUS_STUN));
    assert!(human.player_see_invisible_sources() >= 1);
    assert!(
        human.snapshot().player.abilities.iter().any(|ability| {
            ability.id == "rfb.ability.race.golem-stone-skin" && ability.can_cast
        })
    );

    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    let stats = human.player_derived_stats();
    assert_eq!(
        species_contribution(&stats.armor_class, "rfb-legacy.race.golem"),
        0
    );
    assert_eq!(
        human
            .effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Normal
    );
    assert!(!human.player_status_immunities().contains(STATUS_PARALYSIS));
    assert!(!human.player_status_immunities().contains(STATUS_STUN));
    assert_eq!(human.player_see_invisible_sources(), 0);
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| { ability.id != "rfb.ability.race.golem-stone-skin" })
    );
}

#[test]
fn undead_race_intrinsics_share_cold_unlock_and_temporary_form_lifecycle() {
    for (race_id, seed, cold_level, resistance) in [
        ("rfb-legacy.race.zombie", 375, 5, DamageType::Nether),
        ("rfb-legacy.race.skeleton", 381, 10, DamageType::Shards),
    ] {
        let mut undead = Game::new_with_build_race_and_name(
            seed,
            "demo.build.warrior",
            race_id,
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("formal undead warrior");
        assert_eq!(
            undead.build.as_ref().expect("formal identity").race_id,
            race_id
        );
        undead.progress.level = cold_level - 1;
        for damage in [resistance, DamageType::Poison] {
            assert_eq!(
                undead.effective_player_resistances().level(damage),
                ResistanceLevel::Resistant,
                "{race_id} {damage:?}"
            );
        }
        assert_eq!(
            undead
                .effective_player_resistances()
                .level(DamageType::Cold),
            ResistanceLevel::Normal,
            "{race_id}"
        );
        assert_eq!(undead.player_hold_life_sources(), 1, "{race_id}");
        assert!(undead.player_see_invisible_sources() >= 1, "{race_id}");
        assert!(undead.player_is_nonliving(), "{race_id}");
        undead.progress.level = cold_level;
        assert_eq!(
            undead
                .effective_player_resistances()
                .level(DamageType::Cold),
            ResistanceLevel::Resistant,
            "{race_id}"
        );

        let mut human = Game::new_with_build_race_and_name(
            seed,
            "demo.build.warrior",
            "demo.race.rfb-human",
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("Human warrior");
        human.progress.level = 30;
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.undead-form").status;
        form.granted_race_id = Some(race_id.to_owned());
        human.player.statuses.push(form);
        for active in [true, false] {
            if !active {
                human
                    .player
                    .statuses
                    .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
            }
            for damage in [resistance, DamageType::Poison, DamageType::Cold] {
                assert_eq!(
                    human.effective_player_resistances().level(damage),
                    if active {
                        ResistanceLevel::Resistant
                    } else {
                        ResistanceLevel::Normal
                    },
                    "{race_id} active={active} {damage:?}"
                );
            }
            assert_eq!(
                human.player_hold_life_sources(),
                usize::from(active),
                "{race_id} active={active}"
            );
            if active {
                assert!(human.player_see_invisible_sources() >= 1, "{race_id}");
            } else {
                assert_eq!(human.player_see_invisible_sources(), 0, "{race_id}");
            }
            assert_eq!(human.player_is_nonliving(), active, "{race_id}");
            let abilities = human.snapshot().player.abilities;
            if active {
                assert!(
                    abilities
                        .iter()
                        .any(|ability| ability.id == "rfb.ability.race.restore-life"
                            && ability.can_cast),
                    "{race_id}"
                );
            } else {
                assert!(
                    abilities
                        .iter()
                        .all(|ability| ability.id != "rfb.ability.race.restore-life"),
                    "{race_id}"
                );
            }
        }
    }
}

#[test]
fn formal_wood_elf_and_temporary_form_cross_trees_without_delay() {
    let start = Position { x: 48, y: 16 };
    let target = Position { x: 49, y: 16 };

    let mut wood_elf = wood_elf_game(386);
    clear_monsters(&mut wood_elf);
    replace_terrain(&mut wood_elf, start, "demo.terrain.floor");
    replace_terrain(&mut wood_elf, target, "demo.terrain.surface-tree");
    wood_elf.player.position = start;
    let gain = energy_gain(derived_speed(&wood_elf.player_derived_stats().speed));
    let expected_ticks = u32::try_from(STANDARD_ACTION_COST.saturating_add(gain - 1) / gain)
        .expect("Wood-Elf movement ticks should fit u32");
    let tick_before = wood_elf.world_tick;
    dispatch_next(
        &mut wood_elf,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(wood_elf.player.position, target);
    assert_eq!(wood_elf.world_tick - tick_before, expected_ticks);

    let mut human = Game::new_with_build_race_and_name(
        386,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("Human warrior should create");
    clear_monsters(&mut human);
    replace_terrain(&mut human, start, "demo.terrain.floor");
    replace_terrain(&mut human, target, "demo.terrain.surface-tree");
    human.player.position = start;
    dispatch_next(
        &mut human,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(human.player.position, start);

    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.wood-elf-form").status;
    form.granted_race_id = Some("rfb-legacy.race.wood-elf".to_owned());
    human.player.statuses.push(form);
    dispatch_next(
        &mut human,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(human.player.position, target);
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == "rfb.ability.race.wood-elf-nature-awareness")
    );

    human.player.position = start;
    human
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    dispatch_next(
        &mut human,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(human.player.position, start);
    assert!(
        human
            .snapshot()
            .player
            .abilities
            .iter()
            .all(|ability| ability.id != "rfb.ability.race.wood-elf-nature-awareness")
    );
}

#[test]
fn flying_races_share_passive_application_removal_and_preserve_unique_traits() {
    for (race_id, seed, level, infravision, see_invisible, light) in [
        ("rfb-legacy.race.archon", 388, 1, 3, 1, None),
        (
            "rfb-legacy.race.sprite",
            390,
            20,
            4,
            0,
            Some(ResistanceLevel::Resistant),
        ),
        (
            "rfb-legacy.race.shadow-fairy",
            421,
            1,
            4,
            0,
            Some(ResistanceLevel::Vulnerable),
        ),
    ] {
        let mut formal = Game::new_with_build_race_and_name(
            seed,
            "demo.build.warrior",
            race_id,
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("formal flying race");
        if race_id == "rfb-legacy.race.sprite" {
            for (level, speed) in [(9, 0), (10, 1), (19, 1), (20, 2)] {
                formal.progress.level = level;
                assert_eq!(
                    species_contribution(&formal.player_derived_stats().speed, race_id),
                    speed,
                    "Sprite level {level}"
                );
            }
        }
        formal.progress.level = level;
        formal.progress.max_level = level;
        formal.refresh_character_skills();
        let check = |game: &Game, active| {
            assert_eq!(game.player_levitates(), active, "{race_id} active={active}");
            assert_eq!(
                game.active_traveler_has_mode(rfb_content::ActorMovementMode::Fly),
                active,
                "{race_id}"
            );
            assert_eq!(
                game.player_infravision_range(),
                if active { infravision } else { 0 },
                "{race_id}"
            );
            assert_eq!(
                game.player_see_invisible_sources(),
                if active { see_invisible } else { 0 },
                "{race_id}"
            );
            if let Some(light) = light {
                assert_eq!(
                    game.effective_player_resistances().level(DamageType::Light),
                    if active {
                        light
                    } else {
                        ResistanceLevel::Normal
                    },
                    "{race_id}"
                );
            }
            if race_id == "rfb-legacy.race.shadow-fairy" {
                assert_eq!(
                    game.player_fairy_stealth_race_id(),
                    active.then_some(race_id)
                );
            }
        };
        check(&formal, true);
        let restored = Game::from_save_with_content(formal.to_save(), formal.content.clone())
            .expect("flying race save");
        assert_eq!(restored.state_hash(), formal.state_hash(), "{race_id}");
        let mut human = Game::new_with_build_race_and_name(
            seed,
            "demo.build.warrior",
            "demo.race.rfb-human",
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect("Human warrior");
        human.progress.level = level;
        human.progress.max_level = level;
        human.refresh_character_skills();
        check(&human, false);
        let mut form =
            monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 100, "test.flying-form").status;
        form.granted_race_id = Some(race_id.to_owned());
        human.player.statuses.push(form);
        check(&human, true);
        if race_id == "rfb-legacy.race.sprite" {
            assert_eq!(
                species_contribution(&human.player_derived_stats().speed, race_id),
                2
            );
            assert!(
                human
                    .snapshot()
                    .player
                    .abilities
                    .iter()
                    .any(|ability| ability.id == "rfb.ability.race.sleeping-dust")
            );
        }
        human
            .player
            .statuses
            .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
        check(&human, false);
        assert!(
            human
                .snapshot()
                .player
                .abilities
                .iter()
                .all(|ability| ability.id != "rfb.ability.race.sleeping-dust")
        );
    }
}

#[test]
fn draconian_subraces_are_available_to_formal_character_creation() {
    for suffix in [
        "red", "white", "blue", "black", "green", "bronze", "crystal", "gold", "shadow",
    ] {
        let race_id = format!("rfb-legacy.race.draconian-{suffix}");
        let game = Game::new_with_build_race_and_name(
            357,
            "demo.build.warrior",
            &race_id,
            Game::DEFAULT_PLAYER_NAME,
        )
        .unwrap_or_else(|error| panic!("{race_id} should create: {error}"));
        assert_eq!(
            game.build.as_ref().expect("formal build identity").race_id,
            race_id
        );
        assert!(game.snapshot().player.abilities.iter().any(|ability| {
            ability.id == format!("rfb.ability.race.draconian-{suffix}-breath")
        }));
    }
}

#[test]
fn draconian_level_35_reward_revalidates_all_nine_completed_powers() {
    let mut game = draconian_reward_game();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
    let pending = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Draconian power should be pending at level 35");
    assert_eq!(pending.reward_id, "draconian-power");
    assert_eq!(
        pending
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        [
            "rfb.mutation.draconian-shield",
            "rfb.mutation.draconian-magic-res",
            "rfb.mutation.draconian-strike",
            "rfb.mutation.draconian-breath",
            "rfb.mutation.draconian-regen",
            "rfb.mutation.draconian-kin",
            "rfb.mutation.draconian-lore",
            "rfb.mutation.draconian-resistance",
            "rfb.mutation.draconian-metamorphosis",
        ]
    );

    let base_armor = game.player_derived_stats().armor_class.value;
    let base_save = game.player_derived_stats().saving_throw_skill.value;

    let mut shield = game.clone();
    assert!(shield.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-shield",
        &mut Vec::new(),
    ));
    assert_eq!(
        shield.player_derived_stats().armor_class.value,
        base_armor + 15
    );

    let mut magic_resistance = game.clone();
    assert!(magic_resistance.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-magic-res",
        &mut Vec::new(),
    ));
    assert_eq!(
        magic_resistance
            .player_derived_stats()
            .saving_throw_skill
            .value,
        base_save + 22
    );

    let mut deadly_breath = game.clone();
    assert!(deadly_breath.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-breath",
        &mut Vec::new(),
    ));
    assert!(
        deadly_breath
            .progress
            .locked_mutation_ids
            .contains("rfb.mutation.draconian-breath")
    );

    let mut regeneration = game.clone();
    assert!(regeneration.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-regen",
        &mut Vec::new(),
    ));
    assert_eq!(regeneration.player_regeneration_rate_percent(), 250);

    let mut lore = game.clone();
    assert!(lore.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-lore",
        &mut Vec::new(),
    ));
    assert!(lore.player_auto_identifies_items());

    let mut resistance = game.clone();
    assert!(resistance.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-resistance",
        &mut Vec::new(),
    ));
    assert_eq!(
        resistance
            .effective_player_resistances()
            .level(DamageType::Fire),
        ResistanceLevel::Strong
    );

    let mut strike = game.clone();
    assert!(strike.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-strike",
        &mut Vec::new(),
    ));
    let strike_power = strike
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "rfb.ability.mutation.draconian-strike-red")
        .expect("red Dragon Strike should be projected");
    assert_eq!(strike_power.source, AbilitySourceDto::Mutation);
    assert_eq!(strike_power.resource_cost, 15);
    assert!(matches!(
        strike_power.effects.as_slice(),
        [AbilityEffectSpecDto::MeleeAdjacent]
    ));
    strike.player.position = Position { x: 3, y: 3 };
    replace_terrain(&mut strike, Position { x: 4, y: 3 }, "demo.terrain.floor");
    strike.push_generated_actor(
        "test.draconian-strike-target".to_owned(),
        "demo.actor.warrens-keeper",
        Position { x: 4, y: 3 },
    );
    strike.debug_set_ability_casts_succeed(true);
    let mut strike_events = Vec::new();
    strike
        .resolve_player_ability(
            "rfb.ability.mutation.draconian-strike-red",
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut strike_events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Dragon Strike should resolve against an adjacent target");
    assert!(strike_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityCastSucceeded { resolution }
            if resolution.resource_cost == 15
                && resolution.resource_paid + resolution.hp_paid == 15
    )));

    let mut kin = game;
    assert!(kin.choose_race_mutation(
        "draconian-power",
        "rfb.mutation.draconian-kin",
        &mut Vec::new(),
    ));
    let kin_power = kin
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "rfb.ability.mutation.draconian-kin")
        .expect("Summon Kin should be projected");
    assert_eq!(kin_power.source, AbilitySourceDto::Mutation);
    assert_eq!(kin_power.resource_cost, 30);
    assert!(matches!(
        kin_power.effects.as_slice(),
        [AbilityEffectSpecDto::SummonCategory {
            category,
            maximum_level: 35,
            ..
        }] if category == "kin-glyph-100"
    ));

    kin.player.position = Position { x: 3, y: 3 };
    for y in 1..=5 {
        for x in 1..=5 {
            replace_terrain(&mut kin, Position { x, y }, "demo.terrain.floor");
        }
    }
    kin.debug_set_ability_casts_succeed(true);
    let mut summon_events = Vec::new();
    kin.resolve_player_ability(
        "rfb.ability.mutation.draconian-kin",
        TargetSelection::SelfTarget,
        &mut summon_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Summon Kin should resolve");
    assert!(!kin.entities.is_empty());
    assert!(kin.entities.iter().all(|entity| {
        kin.content.actor(&entity.kind_id).is_some_and(|actor| {
            actor.level <= 35 && actor.tags.iter().any(|tag| tag == "kin-glyph-100")
        }) && kin.actor_is_player_side(entity)
    }));
    assert!(summon_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityCastSucceeded { resolution }
            if resolution.resource_cost == 30
                && resolution.resource_paid + resolution.hp_paid == 30
    )));

    let restored = Game::from_save_with_content(kin.to_save(), kin.content.clone())
        .expect("chosen Draconian power should survive save and restore");
    assert!(
        restored
            .progress
            .locked_mutation_ids
            .contains("rfb.mutation.draconian-kin")
    );
    assert!(
        restored
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
}

#[test]
fn draconian_metamorphosis_replaces_body_and_derives_combat_save_and_hash_state() {
    let mut game = draconian_reward_game();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
    let hash_before = game.state_hash();
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.dagger" && matches!(item.location, ItemLocation::Equipped { .. })
    }));
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.robe" && matches!(item.location, ItemLocation::Equipped { .. })
    }));

    assert!(game.choose_race_mutation(
        "draconian-power",
        DRACONIAN_METAMORPHOSIS_MUTATION_ID,
        &mut Vec::new(),
    ));
    assert_eq!(
        game.body_slots
            .iter()
            .map(|slot| (slot.id.as_str(), slot.slot_type.as_str()))
            .collect::<Vec<_>>(),
        [
            ("ring-1", "ring"),
            ("ring-2", "ring"),
            ("ring-3", "ring"),
            ("ring-4", "ring"),
            ("ring-5", "ring"),
            ("ring-6", "ring"),
            ("amulet", "amulet"),
            ("light", "light"),
            ("cloak", "cloak"),
            ("head", "head"),
        ]
    );
    assert!(game.items.iter().all(|item| {
        !matches!(item.location, ItemLocation::Equipped { .. })
            || matches!(
                game.body_slot_type(match &item.location {
                    ItemLocation::Equipped { slot_id } => slot_id,
                    _ => unreachable!(),
                }),
                Some("ring" | "amulet" | "light" | "cloak" | "head")
            )
    }));
    for kind_id in ["demo.item.dagger", "demo.item.robe"] {
        assert!(
            game.items.iter().any(|item| {
                item.kind_id == kind_id && item.location == ItemLocation::Inventory
            })
        );
    }

    assert!(game.player_has_draconian_metamorphosis());
    assert_eq!(game.draconian_metamorphosis_attack_level(), 58);
    let stats = game.player_derived_stats();
    assert!(stats.armor_class.contributions.iter().any(|contribution| {
        contribution.source_id == DRACONIAN_METAMORPHOSIS_MUTATION_ID && contribution.amount == 67
    }));
    let attacks = game.player_mutation_innate_attack_profiles(&stats, None);
    let metamorphosis_attacks = attacks
        .iter()
        .filter(|attack| {
            attack.source_mutation_id.as_deref() == Some(DRACONIAN_METAMORPHOSIS_MUTATION_ID)
        })
        .map(|attack| {
            (
                attack.attack_name.as_deref(),
                attack.damage_dice,
                attack.damage_sides,
                attack.attacks,
                attack.extra_attack_chance_percent,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        metamorphosis_attacks,
        [(Some("爪击"), 4, 5, 1, 6), (Some("撕咬"), 4, 13, 1, 0)]
    );

    let rng_before_polymorph = game.rng.clone();
    game.resolve_player_polymorph("demo.actor.lord-of-change", 61, &mut Vec::new());
    assert_eq!(game.rng, rng_before_polymorph);
    assert!(
        !game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_PLAYER_POLYMORPH)
    );
    assert_ne!(game.state_hash(), hash_before);

    let saved = game.to_save();
    assert_eq!(saved.player.body_slots.len(), 10);
    assert!(
        saved
            .player
            .locked_mutation_ids
            .iter()
            .any(|id| id == DRACONIAN_METAMORPHOSIS_MUTATION_ID)
    );
    let restored = Game::from_save_with_content(saved, game.content.clone())
        .expect("Draconian metamorphosis save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.body_slots, game.body_slots);
    assert_eq!(
        restored.player_derived_stats().armor_class.value,
        game.player_derived_stats().armor_class.value
    );
}

#[test]
fn draconian_metamorphosis_uses_class_multipliers_and_original_exclusions() {
    for (build_id, expected_attack_level) in [
        ("demo.build.warrior", 87),
        ("demo.build.paladin-death", 80),
        ("demo.build.high-mage-death", 58),
    ] {
        let mut game = draconian_reward_game_for_build(build_id);
        clear_monsters(&mut game);
        game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
        assert!(game.choose_race_mutation(
            "draconian-power",
            DRACONIAN_METAMORPHOSIS_MUTATION_ID,
            &mut Vec::new(),
        ));
        assert_eq!(
            game.draconian_metamorphosis_attack_level(),
            expected_attack_level,
            "{build_id}"
        );
    }

    for build_id in [
        "demo.build.archer",
        "demo.build.cavalry",
        "demo.build.sniper",
    ] {
        let mut game = draconian_reward_game_for_build(build_id);
        clear_monsters(&mut game);
        game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
        let pending = game
            .snapshot()
            .player
            .pending_race_mutation_choice
            .expect("Draconian power should remain selectable");
        assert_eq!(pending.candidates.len(), 8, "{build_id}");
        assert!(
            !pending
                .candidates
                .iter()
                .any(|candidate| candidate.id == DRACONIAN_METAMORPHOSIS_MUTATION_ID)
        );
    }
}

#[test]
fn race_level_mutation_rewards_are_derived_locked_and_zero_time() {
    let mut game = race_reward_game(TEST_RACE_REWARD_BUILD_ID);
    clear_monsters(&mut game);
    let rng_before = game.rng.clone();
    let mut level_events = Vec::new();
    game.apply_player_experience(game.experience_required_for_level(3), &mut level_events);

    assert_eq!(game.rng, rng_before);
    assert!(
        game.progress
            .active_mutation_ids
            .contains(TEST_RACE_DEFAULT_MUTATION_ID)
    );
    assert!(
        game.progress
            .locked_mutation_ids
            .contains(TEST_RACE_DEFAULT_MUTATION_ID)
    );
    let pending = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("level two choice should be pending");
    assert_eq!(pending.reward_id, TEST_RACE_CHOICE_REWARD_ID);
    assert_eq!(
        pending
            .candidates
            .iter()
            .map(|candidate| candidate.id.as_str())
            .collect::<Vec<_>>(),
        [TEST_RACE_CHOICE_MUTATION_ID, "rfb.mutation.evasion"]
    );
    assert!(pending.candidates.iter().all(|candidate| !candidate.locked));

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("pending race choice should be derived after loading");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .snapshot()
            .player
            .pending_race_mutation_choice
            .expect("loaded game should retain the derived choice")
            .reward_id,
        TEST_RACE_CHOICE_REWARD_ID
    );

    let before_rejection = game.clone();
    assert!(matches!(
        game.dispatch(command(1, 0, GameCommand::Wait)),
        Err(CoreError::RaceMutationChoiceRequired)
    ));
    assert_eq!(game.state_hash(), before_rejection.state_hash());
    assert_eq!(game.rng, before_rejection.rng);
    assert!(matches!(
        game.dispatch(command(
            1,
            0,
            GameCommand::ChooseRaceMutation {
                reward_id: TEST_RACE_CHOICE_REWARD_ID.to_owned(),
                mutation_id: "rfb.mutation.evasion-missing".to_owned(),
            },
        )),
        Err(CoreError::RaceMutationChoiceUnavailable)
    ));
    assert_eq!(game.last_visual_cells, before_rejection.last_visual_cells);

    let world_tick_before = game.world_tick;
    let energy_before = game.player.energy_need;
    let update = dispatch_next(
        &mut game,
        GameCommand::ChooseRaceMutation {
            reward_id: TEST_RACE_CHOICE_REWARD_ID.to_owned(),
            mutation_id: TEST_RACE_CHOICE_MUTATION_ID.to_owned(),
        },
    );
    assert_eq!(update.world_tick, world_tick_before);
    assert_eq!(game.player.energy_need, energy_before);
    assert_eq!(game.rng, rng_before);
    assert!(
        game.progress
            .active_mutation_ids
            .contains(TEST_RACE_CHOICE_MUTATION_ID)
    );
    assert!(
        game.progress
            .locked_mutation_ids
            .contains(TEST_RACE_CHOICE_MUTATION_ID)
    );
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    game.apply_player_experience_drain(u64::MAX, "test", &mut Vec::new());
    assert!(
        game.progress
            .locked_mutation_ids
            .contains(TEST_RACE_CHOICE_MUTATION_ID)
    );
    let mut regained_events = Vec::new();
    game.apply_player_experience(game.experience_required_for_level(3), &mut regained_events);
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
    assert!(!regained_events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::MutationGained { mutation_id, .. }
                if mutation_id == TEST_RACE_CHOICE_MUTATION_ID
                    || mutation_id == TEST_RACE_DEFAULT_MUTATION_ID
        )
    }));
}

#[test]
fn casting_attribute_race_reward_uses_the_class_profile() {
    let mut game = race_reward_game(TEST_RACE_REWARD_CASTER_BUILD_ID);
    game.apply_player_experience(game.experience_required_for_level(3), &mut Vec::new());

    assert!(
        game.progress
            .locked_mutation_ids
            .contains(TEST_RACE_INT_MUTATION_ID)
    );
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains(TEST_RACE_DEFAULT_MUTATION_ID)
    );
}

#[test]
fn formal_human_weakness_uses_each_current_build_casting_attribute_once() {
    for (build_id, expected_mutation_id) in [
        ("demo.build.warrior", HUMAN_STR_MUTATION_ID),
        ("demo.build.archer", HUMAN_STR_MUTATION_ID),
        ("demo.build.cavalry", HUMAN_STR_MUTATION_ID),
        ("demo.build.sniper", HUMAN_STR_MUTATION_ID),
        ("demo.build.high-mage-death", HUMAN_INT_MUTATION_ID),
        ("demo.build.high-mage-arcane", HUMAN_INT_MUTATION_ID),
        ("demo.build.paladin-death", HUMAN_WIS_MUTATION_ID),
    ] {
        let mut game = Game::new_with_build(0, build_id).expect("formal build should create");
        game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());

        assert!(
            game.progress
                .active_mutation_ids
                .contains(expected_mutation_id)
        );
        assert!(
            game.progress
                .locked_mutation_ids
                .contains(expected_mutation_id)
        );
        let restored = Game::from_save(game.to_save()).expect("Human weakness should reload");
        assert_eq!(restored.state_hash(), game.state_hash());
    }

    let mut warrior = Game::new_with_build(0, "demo.build.warrior").unwrap();
    warrior.apply_player_experience(warrior.experience_required_for_level(35), &mut Vec::new());
    warrior.apply_player_experience_drain(u64::MAX, "test", &mut Vec::new());
    let mut regained_events = Vec::new();
    warrior.apply_player_experience(
        warrior.experience_required_for_level(35),
        &mut regained_events,
    );
    assert!(!regained_events.iter().any(|event| matches!(
        event,
        DomainEvent::MutationGained { mutation_id, .. }
            if mutation_id == HUMAN_STR_MUTATION_ID
    )));
}

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
        game.apply_player_experience(game.experience_required_for_level(25), &mut Vec::new());
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
    game.apply_player_experience(game.experience_required_for_level(25), &mut Vec::new());

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
fn esp_respects_mind_flags_and_conceals_nonvisual_identity() {
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
    assert!(normal.gain_mutation(HUMAN_WIS_MUTATION_ID, &mut Vec::new()));
    assert!(!normal.entity_is_visible_by_telepathy(&normal.entities[0]));
    let position_index = normal
        .index(position)
        .expect("monster position should be in bounds");
    normal.glow[position_index] = true;
    assert!(normal.entity_is_visually_visible_to_player(&normal.entities[0]));
    assert!(normal.entity_is_visible_to_player(&normal.entities[0]));
    normal.entities[0].controller_id = Some(normal.player.id.clone());
    assert!(normal.entity_is_visible_by_telepathy(&normal.entities[0]));
    normal.entities[0].controller_id = None;
    normal
        .progress
        .active_mutation_ids
        .remove(HUMAN_WIS_MUTATION_ID);
    normal.apply_player_melee_status(crate::effect::STATUS_BLINDNESS, 100, "test.blindness");
    let projected = normal
        .snapshot()
        .entities
        .into_iter()
        .find(|entity| entity.id == "test.normal-mind")
        .expect("telepathy should project the unseen normal mind");
    assert_eq!(projected.kind_id, "core.actor.fuzzy-monster");
    assert_eq!(projected.glyph, "k");
    assert_eq!(
        projected.attack, 0,
        "fuzzy projection must not leak combat identity"
    );

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

    let mut weird = game_with_actor_definition(0, "demo.actor.small-kobold", |actor| {
        actor.tags.push("weird-mind".to_owned());
    });
    clear_monsters(&mut weird);
    weird.player.position = Position { x: 3, y: 3 };
    weird.push_generated_actor(
        "test.weird-mind".to_owned(),
        "demo.actor.small-kobold",
        position,
    );
    assert!(weird.gain_mutation("rfb.mutation.esp", &mut Vec::new()));
    assert!(!weird.entity_is_visible_by_telepathy(&weird.entities[0]));
    weird.entities[0].visible_weird_mind = true;
    assert!(weird.entity_is_visible_by_telepathy(&weird.entities[0]));
    let restored = Game::from_save_with_content(weird.to_save(), weird.content.clone())
        .expect("weird-mind detection should reload");
    assert!(restored.entities[0].visible_weird_mind);
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

    let mut game = test_caster_game(705);
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(25), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);

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
        BTreeSet::from([
            "rfb.mutation.puny".to_owned(),
            "rfb.mutation.sacred-vitality".to_owned(),
        ])
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
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("New Life result should round trip");
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
    let mut warrior =
        Game::new_with_build(17, "demo.build.warrior").expect("Warrior build should create");
    warrior.apply_player_experience(380, &mut Vec::new());
    assert_eq!(warrior.progress.level, 10);
    assert_eq!(
        warrior
            .progress
            .skill("demo.skill.melee")
            .map(|skill| skill.current),
        Some(100)
    );

    let restored = Game::from_save(warrior.to_save()).expect("build save should reload");
    assert_eq!(restored.snapshot(), warrior.snapshot());
    assert!(matches!(
        Game::new_with_build(17, "demo.build.missing"),
        Err(CoreError::UnknownCharacterBuild(_))
    ));
}

#[test]
fn formal_tonberry_action_chain_equips_levels_attacks_swaps_and_restores() {
    let mut game = Game::new_with_build_race_and_name(
        83,
        "demo.build.warrior",
        "rfb-legacy.race.tonberry",
        "冬贝利验收",
    )
    .unwrap();
    clear_monsters(&mut game);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    let starting_weapon = game
        .snapshot()
        .player
        .trait_details
        .active_weapon_id
        .unwrap();
    give_inventory_item(&mut game, "test.tonberry-chain.sabre", "demo.item.sabre");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.tonberry-chain.sabre".to_owned(),
            slot_id: Some("right-hand".to_owned()),
        },
    );
    assert_eq!(
        game.snapshot()
            .player
            .trait_details
            .active_weapon_id
            .as_deref(),
        Some("test.tonberry-chain.sabre")
    );
    assert_eq!(game.progress.level, 1);
    game.apply_player_experience(game.experience_required_for_level(10), &mut Vec::new());
    assert_eq!(game.progress.level, 10);
    let snapshot = game.snapshot();
    let speed = snapshot
        .player
        .trait_details
        .stats
        .iter()
        .find(|stat| stat.id == "speed")
        .unwrap();
    assert_eq!(
        speed
            .sources
            .iter()
            .find(|source| source.source_id == "rfb-legacy.race.tonberry")
            .unwrap()
            .amount,
        -1
    );
    let attacks = snapshot
        .player
        .trait_details
        .stats
        .iter()
        .find(|stat| stat.id == "melee-attacks-hundredths")
        .unwrap();
    assert!(
        attacks
            .sources
            .iter()
            .any(|source| source.source_id == "rfb-legacy.race.tonberry" && source.amount < 0)
    );
    assert!(
        snapshot.player.trait_details.melee_damage[0]
            .base_damage
            .unwrap()[0]
            >= 20
    );
    let east = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, east, "demo.terrain.floor");
    let index = game.index(east).unwrap();
    game.glow[index] = true;
    game.push_generated_actor(
        "test.tonberry-chain.sheep".to_owned(),
        "demo.actor.sheep",
        east,
    );
    let turn = game.turn;
    // Select a deterministic hit for the action chain; fractional/miss RNG has separate coverage.
    game = (0..32)
        .find_map(|seed| {
            let mut attempt = game.clone();
            attempt.rng = RfbRng::seeded(seed);
            let attack = dispatch_next(
                &mut attempt,
                GameCommand::Move {
                    direction: Direction::East,
                },
            );
            attack
                .events
                .iter()
                .any(|event| event.kind == "combat.hit")
                .then_some(attempt)
        })
        .expect("a level-ten Tonberry can land a weapon hit");
    assert!(game.turn > turn);
    clear_monsters(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: starting_weapon.clone(),
            slot_id: Some("right-hand".to_owned()),
        },
    );
    assert_eq!(
        game.snapshot().player.trait_details.active_weapon_id,
        Some(starting_weapon)
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.state_hash(), game.state_hash());
    let turn = game.turn;
    dispatch_next(&mut game, GameCommand::Wait);
    dispatch_next(&mut restored, GameCommand::Wait);
    assert!(restored.turn > turn);
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn formal_tomte_action_chain_probes_changes_headgear_levels_senses_and_restores() {
    let mut game = Game::new_with_build_race_and_name(
        83,
        "demo.build.warrior",
        "rfb-legacy.race.tomte",
        "托姆特验收",
    )
    .unwrap();
    clear_monsters(&mut game);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    game.mogaminator.enabled = false;
    let cap_id = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.knit-cap")
        .unwrap()
        .id
        .clone();
    let east = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, east, "demo.terrain.floor");
    let index = game.index(east).unwrap();
    game.glow[index] = true;
    game.push_generated_actor(
        "test.tomte-chain.sheep".to_owned(),
        "demo.actor.sheep",
        east,
    );
    game.debug_set_ability_casts_succeed(true);
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "rfb.ability.race.probe-monsters".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(game.probed_actor_kind_ids.contains("demo.actor.sheep"));
    game.debug_set_ability_casts_succeed(false);
    clear_monsters(&mut game);
    give_inventory_item(&mut game, "test.tomte-chain.helmet", "demo.item.iron-helm");
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: "test.tomte-chain.helmet".to_owned(),
            slot_id: Some("head".to_owned()),
        },
    );
    assert_eq!(
        game.snapshot().player.trait_details.tomte_heavy_headgear,
        Some(true)
    );
    assert!(!game.player_has_tomte_item_sensing());
    dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: cap_id,
            slot_id: Some("head".to_owned()),
        },
    );
    assert_eq!(
        game.snapshot().player.trait_details.tomte_heavy_headgear,
        Some(false)
    );
    game.apply_player_experience(game.experience_required_for_level(39), &mut Vec::new());
    assert_eq!(game.progress.level, 39);
    assert!(!game.player_auto_identifies_items());
    give_inventory_item(&mut game, "test.tomte-chain.arrows", "demo.item.arrow");
    let arrows = game.items.last_mut().unwrap();
    arrows.quantity = 3;
    arrows.quality = ItemQualityDto::Fine;
    arrows.affix_ids.push("demo.affix.frost-hunter".to_owned());
    arrows.location = ItemLocation::Ground(east);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, east);
    assert_eq!(
        game.item_property_knowledge["test.tomte-chain.arrows"].feeling,
        Some(rfb_protocol::ItemFeelingDto::Excellent)
    );
    dispatch_next(&mut game, GameCommand::PickUp);
    assert!(game.items.iter().any(
        |item| item.id == "test.tomte-chain.arrows" && item.location == ItemLocation::Inventory
    ));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    let experience = restored.experience_required_for_level(40) - restored.progress.experience;
    restored.apply_player_experience(experience, &mut Vec::new());
    assert_eq!(restored.progress.level, 40);
    assert!(restored.player_auto_identifies_items());
    dispatch_next(
        &mut restored,
        GameCommand::Drop {
            item_ids: vec!["test.tomte-chain.arrows".to_owned()],
        },
    );
    dispatch_next(&mut restored, GameCommand::Wait);
    let arrows = restored
        .items
        .iter()
        .find(|item| item.id == "test.tomte-chain.arrows")
        .unwrap();
    assert_eq!(
        restored.item_identification(arrows),
        ItemIdentificationDto::Appraised
    );
    assert!(restored.item_property_knowledge[&arrows.id].appraised);
    assert!(!restored.item_property_knowledge[&arrows.id].identified);
    assert_eq!(
        restored.item_property_knowledge[&arrows.id].feeling,
        Some(rfb_protocol::ItemFeelingDto::Excellent)
    );
    dispatch_next(&mut restored, GameCommand::PickUp);
    let mut continued = Game::from_save(restored.to_save()).unwrap();
    assert_eq!(continued.snapshot(), restored.snapshot());
    let turn = continued.turn;
    dispatch_next(&mut continued, GameCommand::Wait);
    dispatch_next(&mut restored, GameCommand::Wait);
    assert!(continued.turn > turn);
    assert_eq!(continued.state_hash(), restored.state_hash());
}

#[test]
fn tomte_birth_merges_one_cap_with_each_class_kit_and_unique_knowledge_virtue() {
    const RACE: &str = "rfb-legacy.race.tomte";
    let content = load_built_in_content().unwrap();
    for build_id in [
        "demo.build.warrior",
        "demo.build.archer",
        "demo.build.high-mage-death",
        "demo.build.paladin-death",
        "demo.build.cavalry",
        "demo.build.sniper",
    ] {
        let mut game =
            Game::new_with_build_race_and_name(83, build_id, RACE, Game::DEFAULT_PLAYER_NAME)
                .expect(build_id);
        let inventory = game
            .items
            .iter()
            .filter(|item| !matches!(item.location, ItemLocation::Ground(_)))
            .collect::<Vec<_>>();
        let caps = inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.knit-cap")
            .collect::<Vec<_>>();
        assert_eq!(caps.len(), 1, "{build_id}");
        assert_eq!(caps[0].quantity, 1);
        assert_eq!(
            caps[0].location,
            ItemLocation::Equipped {
                slot_id: "head".to_owned()
            }
        );
        assert_eq!(game.player_tomte_headgear_excess_weight(), 0);
        assert!(game.player_has_tomte_item_sensing());
        let rations = inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.ration-of-food")
            .collect::<Vec<_>>();
        assert_eq!(rations.len(), 1);
        assert!((5..=9).contains(&rations[0].quantity));
        assert_eq!(rations[0].location, ItemLocation::Inventory);
        let torches = inventory
            .iter()
            .filter(|item| item.kind_id == "demo.item.wooden-torch")
            .collect::<Vec<_>>();
        assert!((3..=7).contains(&torches.len()));
        assert!(torches.iter().all(|item| item.quantity == 1
            && item.location == ItemLocation::Inventory
            && item.fuel == torches[0].fuel));
        assert!((1500..=3500).contains(&torches[0].fuel.unwrap().current));
        let (build, _, class, personality) =
            build_definitions(&content, game.build.as_ref().unwrap()).unwrap();
        let kit = class
            .starting_items
            .iter()
            .chain(&personality.starting_items)
            .chain(&build.starting_items);
        assert_eq!(inventory.len(), 1 + 1 + torches.len() + kit.clone().count());
        for expected in kit {
            let items = inventory
                .iter()
                .filter(|item| item.kind_id == expected.item_kind_id)
                .collect::<Vec<_>>();
            assert_eq!(items.len(), 1, "{build_id}: {}", expected.item_kind_id);
            assert!(
                (expected.quantity..=expected.maximum_quantity.unwrap_or(expected.quantity))
                    .contains(&items[0].quantity)
            );
            assert_eq!(
                matches!(items[0].location, ItemLocation::Equipped { .. }),
                expected.equipped
            );
        }
        assert_eq!(
            game.virtues
                .iter()
                .filter(|virtue| virtue.kind == VirtueKindDto::Knowledge)
                .count(),
            1
        );
        assert_eq!(
            game.virtues
                .iter()
                .map(|virtue| virtue.kind)
                .collect::<BTreeSet<_>>()
                .len(),
            8
        );
        assert!(game.virtues.iter().all(|virtue| virtue.value == 0));
        give_inventory_item(&mut game, "test.other-helmet", "demo.item.iron-helm");
        assert!(
            game.equip_inventory_item("test.other-helmet", Some("head"))
                .is_some()
        );
        assert!(game.player_tomte_headgear_excess_weight() > 0);
    }
}

#[test]
fn class_birth_applies_attributes_skills_and_equipped_kit_from_the_selected_build() {
    // STR, INT, WIS, DEX, CON, CHR; None means the former class test did not check it.
    for (class, build_id, seed, life, experience, attributes, skills, equipped, ammunition) in [
        (
            "archer",
            "demo.build.archer",
            0x4152_4348_4552,
            110,
            110,
            [Some(15), Some(12), Some(12), Some(15), Some(14), None],
            &[("ranged", 82, 36), ("melee", 56, 18), ("disarming", 38, 12)][..],
            &["short-sword", "leather-scale-mail", "short-bow", "quiver"][..],
            Some(("arrow", 30..=50)),
        ),
        (
            "paladin",
            "demo.build.paladin-death",
            0x5041_4c41_4449_4e00,
            110,
            135,
            [Some(15), Some(10), Some(14), None, Some(15), Some(15)],
            &[
                ("disarming", 20, 7),
                ("device", 24, 10),
                ("saving-throw", 34, 11),
                ("stealth", 1, 0),
                ("search", 12, 0),
                ("perception", 12, 0),
                ("melee", 68, 21),
                ("ranged", 40, 18),
            ][..],
            &["broad-sword", "ring-mail"][..],
            None,
        ),
        (
            "cavalry",
            "demo.build.cavalry",
            0x0043_4156_414c_5259,
            111,
            120,
            [Some(15), Some(11), Some(11), Some(15), Some(15), Some(14)],
            &[
                ("disarming", 20, 10),
                ("device", 18, 7),
                ("saving-throw", 32, 10),
                ("stealth", 1, 0),
                ("search", 16, 0),
                ("perception", 20, 0),
                ("melee", 60, 22),
                ("ranged", 66, 26),
            ][..],
            &["broad-spear", "leather-scale-mail", "short-bow"][..],
            Some(("arrow", 15..=25)),
        ),
        (
            "sniper",
            "demo.build.sniper",
            0x0053_4e49_5045_5200,
            100,
            110,
            [Some(15), Some(12), Some(12), Some(15), Some(14), Some(13)],
            &[
                ("disarming", 25, 12),
                ("device", 24, 10),
                ("saving-throw", 28, 10),
                ("stealth", 5, 0),
                ("search", 32, 0),
                ("perception", 28, 0),
                ("melee", 35, 12),
                ("ranged", 72, 28),
            ][..],
            &["dagger", "soft-leather-armour", "light-crossbow"][..],
            Some(("bolt", 20..=30)),
        ),
    ] {
        let game = Game::new_with_build(seed, build_id).expect(class);
        let player = game.snapshot().player;
        let build = player.build.expect(class);
        assert_eq!(build.build_id, build_id, "{class}");
        assert_eq!(build.class_id, format!("demo.class.{class}"));
        assert_eq!(
            (build.life_percent, build.experience_percent),
            (life, experience),
            "{class}"
        );
        assert_eq!(player.kind_id, format!("demo.actor.{class}-player"));
        let actual = player.progress.attributes;
        for (attribute, expected) in [
            actual.strength,
            actual.intelligence,
            actual.wisdom,
            actual.dexterity,
            actual.constitution,
            actual.charisma,
        ]
        .into_iter()
        .zip(attributes)
        {
            if let Some(expected) = expected {
                assert_eq!(attribute.effective, expected, "{class} {attribute:?}");
            }
        }
        for (id, base, growth) in skills {
            let skill = player
                .progress
                .skills
                .iter()
                .find(|skill| skill.id == format!("demo.skill.{id}"))
                .expect(id);
            assert_eq!(
                (skill.base, skill.growth_per_ten_levels),
                (*base, *growth),
                "{class} {id}"
            );
        }
        for kind in equipped {
            assert!(
                game.items
                    .iter()
                    .any(|item| item.kind_id == format!("demo.item.{kind}")
                        && matches!(item.location, ItemLocation::Equipped { .. })),
                "{class} {kind}"
            );
        }
        if let Some((kind, quantity)) = ammunition {
            let item = game
                .items
                .iter()
                .find(|item| item.kind_id == format!("demo.item.{kind}"))
                .expect(class);
            assert!(quantity.contains(&item.quantity), "{class} {kind}");
        }
        if class == "paladin" {
            assert!(
                game.items
                    .iter()
                    .any(|item| item.kind_id == "demo.item.black-prayers")
            );
        }
    }
}

#[test]
fn formal_race_selection_changes_the_warrior_profile_and_defaults_to_human() {
    let human = Game::new_with_build_race_and_name(
        83,
        "demo.build.warrior",
        "demo.race.rfb-human",
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal Human should create");
    let human_attributes = human.effective_player_attributes();
    let human_skills = human.effective_player_skill_progress();
    let shop_factor = |game: &Game| {
        game.snapshot()
            .shops
            .into_iter()
            .find(|shop| shop.id == "demo.shop.outpost-general-store")
            .expect("General Store should be projected")
            .owner
            .price_factor_percent
    };
    let mut human_experience = human.clone();
    human_experience.apply_player_experience(100, &mut Vec::new());
    assert_eq!(human_experience.progress.experience, 100);

    // Attribute deltas and skill deltas share one calculation path across these races.
    for (race, attributes, skills, hp_order, higher_shop_price, experience) in [
        (
            "half-orc",
            [Some(2), Some(-1), None, None, Some(1), Some(-1)],
            &[("melee", 20), ("perception", -5)][..],
            Some(std::cmp::Ordering::Greater),
            true,
            110,
        ),
        (
            "dunadan",
            [Some(1), Some(2), Some(2), Some(2), Some(3), Some(0)],
            &[("melee", 15), ("perception", 3)][..],
            Some(std::cmp::Ordering::Greater),
            false,
            160,
        ),
        (
            "barbarian",
            [Some(3), Some(-2), Some(-1), Some(1), Some(2), Some(2)],
            &[("melee", 12), ("device", -7)][..],
            Some(std::cmp::Ordering::Greater),
            true,
            135,
        ),
        (
            "hobbit",
            [Some(-2), Some(1), Some(1), Some(3), Some(2), Some(1)],
            &[("melee", -10), ("ranged", 10), ("perception", 5)][..],
            Some(std::cmp::Ordering::Less),
            false,
            120,
        ),
        (
            "high-elf",
            [Some(1), Some(3), Some(-1), Some(3), Some(1), Some(1)],
            &[][..],
            None,
            false,
            190,
        ),
    ] {
        let mut game = Game::new_with_build_race_and_name(
            83,
            "demo.build.warrior",
            &format!("rfb-legacy.race.{race}"),
            Game::DEFAULT_PLAYER_NAME,
        )
        .expect(race);
        assert_eq!(game.player.kind_id, human.player.kind_id, "{race}");
        let actual = game.effective_player_attributes();
        for (attribute, delta) in [
            AttributeKind::Strength,
            AttributeKind::Intelligence,
            AttributeKind::Wisdom,
            AttributeKind::Dexterity,
            AttributeKind::Constitution,
            AttributeKind::Charisma,
        ]
        .into_iter()
        .zip(attributes)
        {
            if let Some(delta) = delta {
                assert_eq!(
                    i16::from(actual.index(attribute)),
                    i16::from(human_attributes.index(attribute)) + delta,
                    "{race} {attribute:?}"
                );
            }
        }
        let actual = game.effective_player_skill_progress();
        for (id, delta) in skills {
            let id = format!("demo.skill.{id}");
            assert_eq!(
                actual[&id].current,
                human_skills[&id].current + delta,
                "{race} {id}"
            );
        }
        if let Some(order) = hp_order {
            assert_eq!(
                game.effective_player_max_hp()
                    .cmp(&human.effective_player_max_hp()),
                order,
                "{race}"
            );
        }
        if higher_shop_price {
            assert!(shop_factor(&game) > shop_factor(&human), "{race}");
        }
        if race == "hobbit" {
            assert_eq!(
                game.content
                    .race("rfb-legacy.race.hobbit")
                    .expect("Hobbit race")
                    .shop_adjust_percent,
                100
            );
        }
        game.apply_player_experience(100, &mut Vec::new());
        assert_eq!(game.progress.experience, 100, "{race}");
        assert_eq!(game.character_experience_percent(), experience, "{race}");
    }

    let default = Game::new_with_build(83, "demo.build.warrior")
        .expect("Warrior build should retain its Human default");
    assert_eq!(default.build, human.build);
    assert_eq!(default.state_hash(), human.state_hash());
    assert_eq!(default.rng_draw_counter(), human.rng_draw_counter());

    assert!(matches!(
        Game::new_with_build_race_and_name(
            83,
            "demo.build.warrior",
            "demo.race.missing",
            Game::DEFAULT_PLAYER_NAME,
        ),
        Err(CoreError::UnknownCharacterRace(_))
    ));
}

#[test]
fn high_elf_intrinsics_and_identity_round_trip() {
    let game = Game::new_with_build_race_and_name(
        84,
        "demo.build.warrior",
        "rfb-legacy.race.high-elf",
        "Finrod",
    )
    .expect("formal High-Elf should create");
    assert_eq!(game.player_infravision_range(), 4);
    assert_eq!(game.player_see_invisible_sources(), 1);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Light),
        ResistanceLevel::Resistant
    );
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    let restored = Game::from_save(game.to_save()).expect("High-Elf save should restore");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn dunadan_sustain_talent_and_identity_are_authoritative() {
    let mut game = Game::new_with_build_race_and_name(
        85,
        "demo.build.warrior",
        "rfb-legacy.race.dunadan",
        "Aragorn",
    )
    .expect("formal Dunadan should create");
    assert!(game.player_sustains_attribute(AttributeKind::Constitution));
    assert!(!game.player_sustains_attribute(AttributeKind::Strength));

    let level_29_experience = game.experience_required_for_level(29);
    game.apply_player_experience(level_29_experience, &mut Vec::new());
    assert_eq!(game.progress.level, 29);
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    game.apply_player_experience(
        game.experience_required_for_level(30) - level_29_experience,
        &mut Vec::new(),
    );
    let pending = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Dunadan should choose a level 30 talent");
    assert_eq!(pending.reward_id, "dunadan-talent");
    assert_eq!(pending.candidates.len(), 20);
    dispatch_next(
        &mut game,
        GameCommand::ChooseRaceMutation {
            reward_id: pending.reward_id,
            mutation_id: "rfb.mutation.sacred-vitality".to_owned(),
        },
    );
    assert!(
        game.progress
            .locked_mutation_ids
            .contains("rfb.mutation.sacred-vitality")
    );
    let restored = Game::from_save(game.to_save()).expect("Dunadan save should restore");
    assert_eq!(restored.build, game.build);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    let mut temporary = Game::new_with_build_race_and_name(
        85,
        "demo.build.warrior",
        "rfb-legacy.race.high-elf",
        "Finrod",
    )
    .expect("formal High-Elf should create");
    temporary.apply_player_experience(temporary.experience_required_for_level(30), &mut Vec::new());
    let mut form = monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.setup").status;
    form.granted_race_id = Some("rfb-legacy.race.dunadan".to_owned());
    temporary.player.statuses.push(form);
    assert!(temporary.player_sustains_attribute(AttributeKind::Constitution));
    assert!(
        temporary
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
    temporary
        .player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    assert!(!temporary.player_sustains_attribute(AttributeKind::Constitution));
}

#[test]
fn half_orc_infravision_and_level_thirty_talent_are_authoritative() {
    let mut game = Game::new_with_build_race_and_name(
        83,
        "demo.build.warrior",
        "rfb-legacy.race.half-orc",
        "Adventurer",
    )
    .expect("formal Half-Orc should create");
    assert_eq!(game.player_infravision_range(), 3);
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Dark),
        ResistanceLevel::Resistant
    );

    let level_29_experience = game.experience_required_for_level(29);
    game.apply_player_experience(level_29_experience, &mut Vec::new());
    assert_eq!(game.progress.level, 29);
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    game.apply_player_experience(
        game.experience_required_for_level(30) - level_29_experience,
        &mut Vec::new(),
    );
    let pending = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Half-Orc should choose a level 30 talent");
    assert_eq!(pending.reward_id, "half-orc-talent");
    assert_eq!(pending.candidates.len(), 20);
    assert!(
        pending
            .candidates
            .iter()
            .any(|candidate| candidate.id == "rfb.mutation.sacred-vitality")
    );

    dispatch_next(
        &mut game,
        GameCommand::ChooseRaceMutation {
            reward_id: pending.reward_id,
            mutation_id: "rfb.mutation.sacred-vitality".to_owned(),
        },
    );
    assert!(
        game.progress
            .locked_mutation_ids
            .contains("rfb.mutation.sacred-vitality")
    );
    let restored = Game::from_save(game.to_save()).expect("Half-Orc save should restore");
    assert_eq!(restored.build, game.build);
    assert_eq!(restored.player_infravision_range(), 3);
    assert!(
        restored
            .progress
            .locked_mutation_ids
            .contains("rfb.mutation.sacred-vitality")
    );
    assert!(
        restored
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
}

#[test]
fn barbarian_fear_power_and_level_thirty_talent_are_authoritative() {
    let mut game = Game::new_with_build_race_and_name(
        86,
        "demo.build.warrior",
        "rfb-legacy.race.barbarian",
        "Conan",
    )
    .expect("formal Barbarian should create");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    game.progress.level = 7;
    let locked = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "rfb.ability.race.berserk")
        .expect("Barbarian should project Berserk");
    assert_eq!(locked.source, AbilitySourceDto::Race);
    assert_eq!(locked.governing_attribute, Some(AttributeKindDto::Strength));
    assert_eq!(locked.minimum_level, 8);
    assert_eq!(locked.base_resource_cost, 10);
    assert_eq!(locked.resource_cost, 10);
    assert_eq!(locked.failure_percent, 100);
    assert!(!locked.can_cast);

    game.progress.level = 8;
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "rfb.ability.race.berserk")
        .expect("Barbarian Berserk should remain projected");
    assert!(available.can_cast);
    assert!(available.failure_percent < 100);
    let mut reward_game = Game::new_with_build_race_and_name(
        86,
        "demo.build.warrior",
        "rfb-legacy.race.barbarian",
        "Conan",
    )
    .expect("formal Barbarian reward game should create");
    let level_29_experience = reward_game.experience_required_for_level(29);
    reward_game.apply_player_experience(level_29_experience, &mut Vec::new());
    assert!(
        reward_game
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
    reward_game.apply_player_experience(
        reward_game.experience_required_for_level(30) - level_29_experience,
        &mut Vec::new(),
    );
    let pending = reward_game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Barbarian should choose a level 30 talent");
    assert_eq!(pending.reward_id, "barbarian-talent");
    assert_eq!(pending.candidates.len(), 20);
    dispatch_next(
        &mut reward_game,
        GameCommand::ChooseRaceMutation {
            reward_id: pending.reward_id,
            mutation_id: "rfb.mutation.sacred-vitality".to_owned(),
        },
    );
    let restored = Game::from_save(reward_game.to_save()).expect("Barbarian save should restore");
    assert!(
        restored
            .progress
            .locked_mutation_ids
            .contains("rfb.mutation.sacred-vitality")
    );
    assert!(
        restored
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    let mut temporary = Game::new_with_build_race_and_name(
        86,
        "demo.build.warrior",
        "rfb-legacy.race.high-elf",
        "Finrod",
    )
    .expect("formal High-Elf should create");
    temporary.progress.level = 30;
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 10, "test.barbarian-form").status;
    form.granted_race_id = Some("rfb-legacy.race.barbarian".to_owned());
    temporary.player.statuses.push(form);
    assert_eq!(
        temporary
            .effective_player_resistances()
            .level(DamageType::Fear),
        ResistanceLevel::Resistant
    );
    assert!(
        temporary
            .snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| ability.id == "rfb.ability.race.berserk")
    );
    assert!(
        temporary
            .snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );
}

#[test]
fn formal_einheri_chooses_the_shared_demigod_talent_at_level_thirty() {
    let mut game = einheri_game(408);
    let level_29_experience = game.experience_required_for_level(29);
    game.apply_player_experience(level_29_experience, &mut Vec::new());
    assert!(
        game.snapshot()
            .player
            .pending_race_mutation_choice
            .is_none()
    );

    game.apply_player_experience(
        game.experience_required_for_level(30) - level_29_experience,
        &mut Vec::new(),
    );
    let pending = game
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("Einheri should choose a level 30 talent");
    assert_eq!(pending.reward_id, "einheri-talent");
    assert_eq!(pending.candidates.len(), 20);
    dispatch_next(
        &mut game,
        GameCommand::ChooseRaceMutation {
            reward_id: pending.reward_id,
            mutation_id: "rfb.mutation.sacred-vitality".to_owned(),
        },
    );
    assert!(
        game.progress
            .locked_mutation_ids
            .contains("rfb.mutation.sacred-vitality")
    );
    game.player.hp = game.effective_player_max_hp() - 30;
    assert_eq!(
        game.apply_player_healing(21).requested,
        12,
        "Sacred Vitality applies before Einheri's halving",
    );

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Einheri talent should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn selected_formal_race_overrides_the_build_default_and_round_trips() {
    let content = race_reward_catalog();
    let game = Game::from_content_internal(
        47,
        content.clone(),
        DEFAULT_WORLD_ID,
        Some(TEST_RACE_REWARD_BUILD_ID),
        Some("demo.race.rfb-human"),
        "Adventurer",
    )
    .expect("formal race override should create");
    let identity = game.build.as_ref().expect("build identity should exist");
    assert_eq!(identity.build_id, TEST_RACE_REWARD_BUILD_ID);
    assert_eq!(identity.race_id, "demo.race.rfb-human");

    let restored = Game::from_save_with_content(game.to_save(), content)
        .expect("selected race should reload independently of the build default");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn attribute_increase_command_commits_growth_without_rng_or_world_progression() {
    let mut game = test_caster_game(96);
    game.apply_player_experience(100, &mut Vec::new());
    assert!(game.progress.pending_attribute_increases > 0);

    let resource = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("test caster should have mana");
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
        .expect("test caster should retain mana");
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
fn restore_life_uses_historical_experience() {
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
}

#[test]
fn attribute_and_experience_history_round_trip_and_reject_invalid_values() {
    let mut game = Game::new(0);
    game.apply_player_experience(10, &mut Vec::new());
    game.progress.maximum_experience += 10;
    game.progress.attributes.strength -= 1;
    let payload = game.to_save();
    let encoded = serde_json::to_value(&payload).expect("save should serialize");
    let restored = Game::from_save(serde_json::from_value(encoded).unwrap())
        .expect("current attribute and experience history should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut invalid = payload.clone();
    let progress = invalid
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved");
    let mut maximum = progress.attributes;
    maximum.strength = progress.attributes.strength.saturating_sub(1);
    progress.maximum_attributes = maximum;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("player attribute state is invalid"))
    ));

    let mut invalid = payload;
    invalid.player.progress.as_mut().unwrap().maximum_experience = 0;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("character progress is invalid"))
    ));
}

#[test]
fn attribute_resource_refresh_scales_the_prechange_current_value_once() {
    let mut game = test_caster_game(96);
    let before = *game
        .resources
        .get("demo.resource.mana")
        .expect("test caster should have mana");
    assert_eq!(before.current, before.maximum);

    assert!(game.resolve_item_drain_attribute(
        "demo.item.frailty-tonic",
        AttributeKind::Intelligence,
        &mut Vec::new(),
    ));
    let drained = *game
        .resources
        .get("demo.resource.mana")
        .expect("test caster should retain mana");
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
        .expect("test caster should retain mana");
    assert_eq!(restored.maximum, before.maximum);
    assert_eq!(restored.current, before.current);
}

#[test]
fn formal_beastman_birth_level_mutations_and_regeneration_match_rfb() {
    let game = super::support::beastman_game(419);
    assert_eq!(game.progress.active_mutation_ids.len(), 1);
    let birth_mutation_id = game
        .progress
        .active_mutation_ids
        .iter()
        .next()
        .expect("Beastman should begin with one mutation");
    assert!(matches!(
        game.content
            .mutation(birth_mutation_id)
            .expect("birth mutation should remain defined")
            .rating,
        rfb_content::MutationRatingDefinition::Good | rfb_content::MutationRatingDefinition::Great
    ));
    assert!(
        !game
            .progress
            .locked_mutation_ids
            .contains(birth_mutation_id)
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Confusion),
        ResistanceLevel::Resistant,
    );
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Sound),
        ResistanceLevel::Resistant,
    );

    let success_seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(5) == 0)
        .expect("a bounded seed should trigger the one-in-five roll");
    let failure_seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(5) != 0)
        .expect("a bounded seed should miss the one-in-five roll");
    let mut leveled = game.clone();
    let mut replay = game.clone();
    for candidate in [&mut leveled, &mut replay] {
        candidate.rng = RfbRng::seeded(success_seed);
        let mut events = Vec::new();
        candidate.apply_player_experience(candidate.experience_required_for_level(2), &mut events);
        assert_eq!(candidate.progress.level, 2);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::MutationGained { .. }))
        );
    }
    assert_eq!(leveled.state_hash(), replay.state_hash());
    let restored = Game::from_save_with_content(leveled.to_save(), leveled.content.clone())
        .expect("Beastman mutation state should restore");
    assert_eq!(restored.state_hash(), leveled.state_hash());

    let mut missed = game.clone();
    missed.rng = RfbRng::seeded(failure_seed);
    let mut events = Vec::new();
    missed.apply_player_experience(missed.experience_required_for_level(2), &mut events);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::MutationGained { .. }))
    );

    leveled.progress.level = 1;
    leveled.progress.experience = 0;
    leveled.rng = RfbRng::seeded(success_seed);
    let mut events = Vec::new();
    leveled.apply_player_experience(leveled.experience_required_for_level(2), &mut events);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::MutationGained { .. }))
    );

    let mutation_ids = game
        .content
        .mutations()
        .take(12)
        .map(|mutation| mutation.id.clone())
        .collect::<Vec<_>>();
    let mut tolerant = game;
    tolerant.progress.active_mutation_ids = mutation_ids[..10].iter().cloned().collect();
    tolerant.progress.locked_mutation_ids.clear();
    assert_eq!(tolerant.mutation_regeneration_percent(), 100);
    tolerant
        .progress
        .active_mutation_ids
        .insert(mutation_ids[10].clone());
    assert_eq!(tolerant.mutation_regeneration_percent(), 95);
    tolerant
        .progress
        .active_mutation_ids
        .insert(mutation_ids[11].clone());
    assert_eq!(tolerant.mutation_regeneration_percent(), 90);
    tolerant.progress.locked_mutation_ids = mutation_ids[..2].iter().cloned().collect();
    assert_eq!(tolerant.mutation_regeneration_percent(), 100);
}

fn permanent_race_game(race_id: &str) -> Game {
    Game::new_with_build_race_and_name(
        71,
        "demo.build.high-mage-death",
        race_id,
        Game::DEFAULT_PLAYER_NAME,
    )
    .unwrap()
}

#[test]
fn permanent_race_change_rejections_are_atomic_and_consume_no_rng() {
    for (native, form, target) in [
        ("demo.race.rfb-human", None, "demo.race.rfb-human"),
        ("demo.race.rfb-human", None, "unknown.race"),
        ("demo.race.rfb-human", None, "rfb-legacy.race.doppelganger"),
        (
            "rfb-legacy.race.android",
            Some("demo.race.rfb-human"),
            "rfb-legacy.race.vampire",
        ),
        (
            "demo.race.rfb-human",
            Some("rfb-legacy.race.android"),
            "rfb-legacy.race.vampire",
        ),
    ] {
        let mut game = permanent_race_game("demo.race.rfb-human");
        game.build.as_mut().unwrap().race_id = native.to_owned();
        if let Some(form) = form {
            let mut status = monster_combat::melee_status("test.form", 50, "test.setup").status;
            status.granted_race_id = Some(form.to_owned());
            game.player.statuses.push(status);
        }
        let before = game.to_save();
        let rng = game.rng.clone();
        let mut events = Vec::new();
        assert!(!game.change_player_race(target, &mut events));
        assert_eq!(game.to_save(), before);
        assert_eq!(game.rng, rng);
        assert!(events.is_empty());
    }
}

#[test]
fn permanent_race_change_rerates_only_hp_and_preserves_identity_progress_and_items() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    game.progress.life_force = 725;
    game.player.hp = game.effective_player_max_hp() / 2;
    let pool = game.resources.get_mut("demo.resource.mana").unwrap();
    pool.current = pool.maximum / 2;
    let build = game.build.clone().unwrap();
    let attributes = game.progress.attributes;
    let potentials = game.progress.attribute_potentials;
    let items = game.items.clone();
    let virtues = game.virtues;
    let clock = (game.turn, game.world_tick);
    let mut expected_rng = game.rng.clone();
    let expected_hp =
        CharacterProgress::roll_hp_progression(game.progress.hp_progression[0], &mut expected_rng);
    let mut events = Vec::new();
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut events));
    assert_eq!(game.progress.hp_progression, expected_hp);
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.progress.life_force, 725);
    assert_eq!(game.progress.attributes, attributes);
    assert_eq!(game.progress.attribute_potentials, potentials);
    assert_eq!(game.items, items);
    assert_eq!((game.turn, game.world_tick), clock);
    let after = game.build.as_ref().unwrap();
    assert_eq!(
        (&after.build_id, &after.class_id, &after.personality_id),
        (&build.build_id, &build.class_id, &build.personality_id)
    );
    assert!(game.player.hp < game.effective_player_max_hp());
    let pool = &game.resources["demo.resource.mana"];
    assert!(pool.current < pool.maximum);
    assert_eq!(
        game.virtues.iter().map(|v| v.kind).collect::<Vec<_>>(),
        virtues.iter().map(|v| v.kind).collect::<Vec<_>>()
    );
    for (old, new) in virtues.iter().zip(&game.virtues) {
        assert_eq!(
            new.value,
            old.value
                + if old.kind == rfb_protocol::VirtueKindDto::Chance {
                    2
                } else {
                    0
                }
        );
    }
    assert!(events.iter().any(|event| matches!(event, DomainEvent::PlayerRaceChanged { race_id, .. } if race_id == "rfb-legacy.race.vampire")));
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.build.unwrap().race_id, "rfb-legacy.race.vampire");
}

#[test]
fn permanent_race_change_preserves_temporary_body_until_expiry() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    let mut status = monster_combat::melee_status("test.form", 50, "test.setup").status;
    status.kind_id = STATUS_PLAYER_POLYMORPH.to_owned();
    status.granted_race_id = Some("rfb-legacy.race.centaur".to_owned());
    game.player.statuses.push(status);
    game.reconcile_player_body_slots_for_current_form();
    game.refresh_player_ability_state();
    let slots = game.body_slots.clone();
    let statuses = game.player.statuses.clone();
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut Vec::new()));
    assert_eq!(game.body_slots, slots);
    assert_eq!(game.player.statuses, statuses);
    game.player
        .statuses
        .retain(|status| status.kind_id != STATUS_PLAYER_POLYMORPH);
    game.reconcile_player_body_slots_for_current_form();
    game.refresh_player_ability_state();
    assert_eq!(
        game.character_definitions().unwrap().1.id,
        "rfb-legacy.race.vampire"
    );
    assert_ne!(game.body_slots, slots);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn permanent_race_change_revokes_draconian_talent_and_restores_worn_gear() {
    let mut game = draconian_reward_game();
    game.apply_player_experience(game.experience_required_for_level(35), &mut Vec::new());
    let items = game
        .items
        .iter()
        .map(|item| (item.id.clone(), (item.kind_id.clone(), item.quantity)))
        .collect::<BTreeMap<_, _>>();
    let dagger = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.dagger")
        .unwrap()
        .id
        .clone();
    assert!(game.choose_race_mutation(
        "draconian-power",
        DRACONIAN_METAMORPHOSIS_MUTATION_ID,
        &mut Vec::new()
    ));
    assert!(
        game.items
            .iter()
            .find(|item| item.id == dagger)
            .unwrap()
            .previously_worn
    );
    assert!(game.gain_mutation("rfb.mutation.teleport", &mut Vec::new()));
    let mut events = Vec::new();
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut events));
    assert!(
        !game
            .progress
            .active_mutation_ids
            .contains(DRACONIAN_METAMORPHOSIS_MUTATION_ID)
    );
    assert!(
        !game
            .progress
            .locked_mutation_ids
            .contains(DRACONIAN_METAMORPHOSIS_MUTATION_ID)
    );
    assert!(
        game.progress
            .active_mutation_ids
            .contains("rfb.mutation.teleport")
    );
    let dagger = game.items.iter().find(|item| item.id == dagger).unwrap();
    assert!(matches!(dagger.location, ItemLocation::Equipped { .. }));
    assert!(!dagger.previously_worn);
    assert_eq!(
        game.items
            .iter()
            .map(|item| (item.id.clone(), (item.kind_id.clone(), item.quantity)))
            .collect::<BTreeMap<_, _>>(),
        items
    );
    assert_eq!(events.iter().filter(|event| matches!(event, DomainEvent::MutationLost { mutation_id, .. } if mutation_id == DRACONIAN_METAMORPHOSIS_MUTATION_ID)).count(), 1);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn permanent_race_change_rechecks_experience_below_the_historical_maximum() {
    let mut game = permanent_race_game("rfb-legacy.race.yeek");
    game.apply_player_experience(game.experience_required_for_level(30), &mut Vec::new());
    let maximum_level = game.progress.max_level;
    let experience = game.progress.experience;
    let maximum_experience = game.progress.maximum_experience;
    let mut events = Vec::new();
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut events));
    assert!(game.progress.level < maximum_level);
    assert_eq!(game.progress.max_level, maximum_level);
    assert_eq!(
        (game.progress.experience, game.progress.maximum_experience),
        (experience, maximum_experience)
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerLevelLost { .. }))
    );
    assert!(game.change_player_race("rfb-legacy.race.yeek", &mut Vec::new()));
    assert_eq!(game.progress.level, maximum_level);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn spell_memory_forgets_by_level_and_remembers_after_save_and_recovery() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    game.apply_player_experience(game.experience_required_for_level(40), &mut Vec::new());
    game.ability_learning_order = vec![
        "demo.ability.death-berserk".to_owned(),
        "demo.ability.death-detect-unlife".to_owned(),
    ];
    game.refresh_player_ability_state();
    assert_eq!(game.learned_abilities.len(), 2);
    let progress = game.ability_progress.clone();
    game.progress.level = 1;
    game.progress.experience = 0;
    game.player.hp = game.effective_player_max_hp();
    game.refresh_character_skills();
    game.refresh_player_ability_state();
    assert!(
        !game
            .learned_abilities
            .contains("demo.ability.death-berserk")
    );
    assert!(
        game.learned_abilities
            .contains("demo.ability.death-detect-unlife")
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.ability_learning_order, game.ability_learning_order);
    restored.apply_player_experience(restored.experience_required_for_level(40), &mut Vec::new());
    assert_eq!(restored.learned_abilities.len(), 2);
    assert_eq!(restored.ability_progress, progress);
}

#[test]
fn spell_memory_capacity_keeps_the_earliest_eligible_studies_and_preserves_proficiency() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    game.apply_player_experience(game.experience_required_for_level(40), &mut Vec::new());
    let profile = game.casting_profile().unwrap().clone();
    let (_, all_ids) = game.player_ability_baseline();
    game.ability_learning_order = all_ids
        .into_iter()
        .rev()
        .filter(|id| {
            let ability =
                game.effective_casting_ability(&profile, game.content.ability(id).unwrap());
            Game::player_ability_parameters(&ability).minimum_level <= game.progress.level
        })
        .collect();
    game.bonus_spell_learning_capacity = 32;
    game.refresh_player_ability_state();
    let history = game.ability_learning_order.clone();
    let proficiency = game.ability_progress.clone();
    assert_eq!(game.learned_abilities.len(), history.len());
    game.bonus_spell_learning_capacity = 0;
    game.progress.attributes.intelligence = 8;
    game.refresh_player_ability_state();
    let capacity = usize::from(game.ability_learning_capacity(&profile));
    assert!(capacity > 0 && capacity < history.len());
    assert_eq!(
        game.learned_abilities,
        history[..capacity].iter().cloned().collect()
    );
    assert_eq!(game.ability_learning_order, history);
    assert_eq!(game.ability_progress, proficiency);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored.bonus_spell_learning_capacity = 32;
    restored.refresh_player_ability_state();
    assert_eq!(restored.learned_abilities.len(), history.len());
    assert_eq!(restored.ability_progress, proficiency);
}

#[test]
fn permanent_race_change_reconciles_feet_and_honors_automatic_rewear_inscriptions() {
    for inscription in [None, Some("@mimic")] {
        let slots = load_built_in_content()
            .unwrap()
            .race("rfb-legacy.race.centaur")
            .unwrap()
            .body_slots
            .clone();
        let mut game = Game::from_content_with_build(
            71,
            race_change_body_catalog(slots),
            DEFAULT_WORLD_ID,
            "demo.build.high-mage-death",
        )
        .unwrap();
        give_inventory_item(&mut game, "test.boots", "demo.item.soft-leather-boots");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.boots")
            .unwrap()
            .inscription = inscription.map(str::to_owned);
        assert!(game.equip_inventory_item("test.boots", None).is_some());
        assert!(game.change_player_race("rfb-legacy.race.vampire", &mut Vec::new()));
        let boots = game
            .items
            .iter()
            .find(|item| item.id == "test.boots")
            .unwrap();
        assert_eq!(boots.location, ItemLocation::Inventory);
        assert_eq!(boots.previously_worn, inscription.is_none());
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert!(game.change_player_race("demo.race.rfb-human", &mut Vec::new()));
        assert!(restored.change_player_race("demo.race.rfb-human", &mut Vec::new()));
        assert_eq!(game.state_hash(), restored.state_hash());
        let boots = game
            .items
            .iter()
            .find(|item| item.id == "test.boots")
            .unwrap();
        assert_eq!(
            matches!(boots.location, ItemLocation::Equipped { .. }),
            inscription.is_none()
        );
        assert!(!boots.previously_worn);
    }
}

#[test]
fn permanent_race_change_releases_quiver_and_container_capacity_without_losing_items() {
    let content = race_change_body_catalog(vec![rfb_content::BodySlotDefinition {
        id: "weapon".to_owned(),
        slot_type: "weapon".to_owned(),
    }]);
    let mut game =
        Game::from_content_with_build(71, content, DEFAULT_WORLD_ID, "demo.build.warrior").unwrap();
    game.items.clear();
    game.item_property_knowledge.clear();
    for (id, kind, slot) in [
        ("test.quiver", "demo.item.quiver", "quiver"),
        ("test.bag", "demo.item.fabric-bag", "container"),
    ] {
        give_inventory_item(&mut game, id, kind);
        assert!(game.equip_inventory_item(id, Some(slot)).is_some());
    }
    give_inventory_item(&mut game, "test.arrows", "demo.item.arrow");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.arrows")
        .unwrap()
        .quantity = 60;
    for index in 0..game.inventory_slot_capacity() {
        give_inventory_item(
            &mut game,
            &format!("test.filler-{index:02}"),
            "demo.item.dagger",
        );
    }
    assert_eq!(game.inventory_used_slots(), game.inventory_slot_capacity());
    let before = game
        .items
        .iter()
        .map(|item| (item.id.clone(), (item.kind_id.clone(), item.quantity)))
        .collect::<BTreeMap<_, _>>();
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut Vec::new()));
    assert_eq!(
        game.items
            .iter()
            .map(|item| (item.id.clone(), (item.kind_id.clone(), item.quantity)))
            .collect::<BTreeMap<_, _>>(),
        before
    );
    assert!(game.inventory_used_slots() <= game.inventory_slot_capacity());
    for id in ["test.quiver", "test.bag"] {
        let item = game.items.iter().find(|item| item.id == id).unwrap();
        assert_eq!(item.location, ItemLocation::Ground(game.player.position));
        assert!(item.previously_worn);
    }
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn body_reconciliation_uses_old_slot_priority_and_first_compatible_destination() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    game.body_slots = vec![
        BodySlot {
            id: "right-hand".to_owned(),
            slot_type: "weapon".to_owned(),
        },
        BodySlot {
            id: "left-hand".to_owned(),
            slot_type: "weapon".to_owned(),
        },
    ];
    game.items.clear();
    game.item_property_knowledge.clear();
    for id in ["z-first-hand", "a-second-hand"] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
    }
    for (id, slot) in [
        ("z-first-hand", "right-hand"),
        ("a-second-hand", "left-hand"),
    ] {
        assert!(game.equip_inventory_item(id, Some(slot)).is_some());
    }
    game.reconcile_player_body_slots(vec![
        BodySlot {
            id: "left-hand".to_owned(),
            slot_type: "weapon".to_owned(),
        },
        BodySlot {
            id: "right-hand".to_owned(),
            slot_type: "weapon".to_owned(),
        },
    ]);
    for (id, expected) in [
        ("z-first-hand", "left-hand"),
        ("a-second-hand", "right-hand"),
    ] {
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .location,
            ItemLocation::Equipped {
                slot_id: expected.to_owned()
            }
        );
    }
}

fn race_change_body_catalog(
    slots: Vec<rfb_content::BodySlotDefinition>,
) -> Arc<rfb_content::ContentCatalog> {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).unwrap();
    artifact
        .content
        .races
        .iter_mut()
        .find(|race| race.id == "rfb-legacy.race.vampire")
        .unwrap()
        .body_slots = slots;
    Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ))
}

#[test]
fn permanent_race_change_revokes_old_human_rewards_and_reopens_choices_below_max_level() {
    let mut game = permanent_race_game("demo.race.rfb-human");
    game.apply_player_experience(game.experience_required_for_level(40), &mut Vec::new());
    let (reward, candidates) = game.pending_race_mutation_choice().unwrap();
    assert!(game.choose_race_mutation(&reward, &candidates[0], &mut Vec::new()));
    let old_talents = game.progress.locked_mutation_ids.clone();
    let historical_level = game.progress.max_level;
    assert!(!old_talents.is_empty());
    assert!(game.change_player_race("rfb-legacy.race.vampire", &mut Vec::new()));
    assert!(
        old_talents
            .iter()
            .all(|id| !game.progress.active_mutation_ids.contains(id)
                && !game.progress.locked_mutation_ids.contains(id))
    );
    assert!(game.change_player_race("demo.race.rfb-human", &mut Vec::new()));
    assert_eq!(game.progress.max_level, historical_level);
    assert!(game.pending_race_mutation_choice().is_some());
    assert!(Game::from_save(game.to_save()).is_ok());
}
