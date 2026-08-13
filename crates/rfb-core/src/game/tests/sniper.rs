// SPDX-License-Identifier: MPL-2.0

use std::sync::OnceLock;

use super::support::{clear_monsters, dispatch_next, give_inventory_item};
use super::*;

const SNIPER_BUILD_ID: &str = "test.build.sniper";
const CONCENTRATE_ABILITY_ID: &str = "test.ability.sniper-concentrate";
const TECHNIQUE_ABILITY_ID: &str = "test.ability.sniper-technique";

fn sniper_game(seed: u64) -> Game {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("core crate should be inside the workspace")
                .join("packs/rfb-demo-original");
            let mut artifact =
                rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
            let template = artifact
                .content
                .abilities
                .iter()
                .find(|ability| ability.id == "demo.ability.archer-create-shots")
                .expect("Archer power should remain available")
                .clone();
            let mut concentrate = template.clone();
            concentrate.id = CONCENTRATE_ABILITY_ID.to_owned();
            concentrate.name_key = "test-sniper-concentrate-name".to_owned();
            concentrate.description_key = "test-sniper-concentrate-description".to_owned();
            concentrate.target = rfb_content::AbilityTargetDefinition {
                modes: vec![rfb_content::AbilityTargetModeDefinition::SelfTarget],
                range: 0,
                requires_line_of_effect: false,
            };
            concentrate.effect = AbilityEffectDefinition::Concentrate;
            concentrate.tags.clear();
            let mut technique = concentrate.clone();
            technique.id = TECHNIQUE_ABILITY_ID.to_owned();
            technique.name_key = "test-sniper-technique-name".to_owned();
            technique.description_key = "test-sniper-technique-description".to_owned();
            technique.effect = AbilityEffectDefinition::NoOp {
                reason: "sniper-test".to_owned(),
            };
            artifact.content.abilities.extend([concentrate, technique]);

            let mut class = artifact
                .content
                .classes
                .iter()
                .find(|class| class.id == "demo.class.archer")
                .expect("Archer class should remain available")
                .clone();
            class.id = "test.class.sniper".to_owned();
            class.name_key = "test-class-sniper-name".to_owned();
            class.description_key = "test-class-sniper-description".to_owned();
            class.ammunition_breakage_factor_modifier = 0;
            class.projectile_critical_chance_bonus_percent_per_level = 0;
            class.sniping_profile = Some(rfb_content::SnipingProfileDefinition {
                preferred_ammunition_type: AmmunitionTypeDefinition::Bolt,
                preferred_ammunition_to_hit_base: 10,
                preferred_ammunition_to_hit_level_divisor: 5,
                base_shot_excess_percent: 50,
                preferred_ammunition_critical_chance_percent: 150,
                base_concentration_maximum: 2,
                concentration_level_offset: 5,
                concentration_level_divisor: 10,
                concentration_bonus_percent_per_level: 10,
            });
            class.abilities = vec![
                ClassAbilityDefinition {
                    ability_id: CONCENTRATE_ABILITY_ID.to_owned(),
                    minimum_level: 1,
                    ui_group_name_key: None,
                    governing_attribute: None,
                    resource_id: None,
                    resource_cost: 0,
                    minimum_concentration: 0,
                    hit_point_cost: 0,
                    base_failure_percent: 0,
                    minimum_failure_percent: 0,
                },
                ClassAbilityDefinition {
                    ability_id: TECHNIQUE_ABILITY_ID.to_owned(),
                    minimum_level: 1,
                    ui_group_name_key: None,
                    governing_attribute: Some(TechniqueAttribute::Dexterity),
                    resource_id: None,
                    resource_cost: 0,
                    minimum_concentration: 3,
                    hit_point_cost: 5,
                    base_failure_percent: 95,
                    minimum_failure_percent: 95,
                },
            ];
            artifact.content.classes.push(class);

            let mut build = artifact
                .content
                .builds
                .iter()
                .find(|build| build.id == "demo.build.archer")
                .expect("Archer build should remain available")
                .clone();
            build.id = SNIPER_BUILD_ID.to_owned();
            build.name_key = "test-build-sniper-name".to_owned();
            build.description_key = "test-build-sniper-description".to_owned();
            build.class_id = "test.class.sniper".to_owned();
            artifact.content.builds.push(build);

            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content)
                    .expect("test sniper content should encode"),
            ))
        })
        .clone();
    Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, SNIPER_BUILD_ID)
        .expect("test Sniper should create")
}

fn equip_light_crossbow(game: &mut Game) {
    game.items
        .iter_mut()
        .find(|item| {
            matches!(item.location, ItemLocation::Equipped { ref slot_id } if slot_id == "shooting")
        })
        .expect("birth launcher should be equipped")
        .location = ItemLocation::Inventory;
    give_inventory_item(game, "test.sniper-crossbow", "demo.item.light-crossbow");
    game.items
        .iter_mut()
        .find(|item| item.id == "test.sniper-crossbow")
        .expect("test crossbow should exist")
        .location = ItemLocation::Equipped {
        slot_id: "shooting".to_owned(),
    };
    give_inventory_item(game, "test.sniper-bolt", "demo.item.bolt");
}

fn resolve_ability(game: &mut Game, ability_id: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        ability_id,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("test ability should resolve");
    events
}

#[test]
fn sniper_profile_uses_original_level_boundaries_bolt_hit_and_half_excess_speed() {
    let mut game = sniper_game(1);
    assert_eq!(
        [1, 5, 15, 25, 35, 45, 50].map(|level| {
            game.progress.level = level;
            game.sniper_max_concentration().expect("Sniper profile")
        }),
        [2, 3, 4, 5, 6, 7, 7]
    );

    equip_light_crossbow(&mut game);
    game.progress.level = 1;
    let novice = game
        .player_projectile_profile()
        .expect("crossbow profile should resolve");
    game.progress.level = 50;
    let master = game
        .player_projectile_profile()
        .expect("crossbow profile should resolve");
    let ranged_skill = game.player_derived_stats().ranged_skill.value.max(100);
    let base_shot = 100 + (ranged_skill - 100) / 2;
    assert_eq!(master.energy_cost, 12_000 / base_shot);
    assert_eq!(master.to_hit - novice.to_hit, 10);
}

#[test]
fn concentrate_reaches_and_holds_the_cap_while_projecting_requirements() {
    let mut game = sniper_game(2);
    game.progress.level = 5;
    game.debug_ability_casts_succeed = true;
    for expected in 1..=3 {
        resolve_ability(
            &mut game,
            CONCENTRATE_ABILITY_ID,
            TargetSelection::SelfTarget,
        );
        assert_eq!(game.sniper_concentration, expected);
    }
    resolve_ability(
        &mut game,
        CONCENTRATE_ABILITY_ID,
        TargetSelection::SelfTarget,
    );
    assert_eq!(game.sniper_concentration, 3);

    let snapshot = game.snapshot();
    assert_eq!(
        snapshot.player.sniper_concentration,
        Some(rfb_protocol::SniperConcentrationDto {
            current: 3,
            maximum: 3,
        })
    );
    let technique = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == TECHNIQUE_ABILITY_ID)
        .expect("test technique should project");
    assert_eq!(technique.minimum_concentration, 3);
    assert_eq!(technique.hit_point_cost, 5);
    assert!(technique.can_cast);

    clear_monsters(&mut game);
    let before = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: CONCENTRATE_ABILITY_ID.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(game.world_tick > before);
    assert_eq!(game.sniper_concentration, 3);
}

#[test]
fn concentration_only_clears_after_a_real_action_or_valid_shot() {
    let mut game = sniper_game(3);
    clear_monsters(&mut game);
    game.sniper_concentration = 2;
    let rng_before = game.rng.to_save().draw_counter;
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "test.ability.missing".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(game.sniper_concentration, 2);
    assert_eq!(game.rng.to_save().draw_counter, rng_before);
    resolve_ability(
        &mut game,
        CONCENTRATE_ABILITY_ID,
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert_eq!(game.sniper_concentration, 2);
    assert_eq!(game.rng.to_save().draw_counter, rng_before);

    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.sniper_concentration, 0);
    game.sniper_concentration = 2;
    dispatch_next(&mut game, GameCommand::Rest { turns: 1 });
    assert_eq!(game.sniper_concentration, 0);
    game.sniper_concentration = 2;
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.sniper_concentration, 0);

    equip_light_crossbow(&mut game);
    game.sniper_concentration = 2;
    dispatch_next(
        &mut game,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    assert_eq!(game.sniper_concentration, 0);
    game.items.retain(|item| item.kind_id != "demo.item.bolt");
    game.sniper_concentration = 2;
    dispatch_next(
        &mut game,
        GameCommand::Fire {
            direction: Direction::East,
        },
    );
    assert_eq!(game.sniper_concentration, 2);
}

#[test]
fn class_ability_concentration_and_hit_point_costs_are_atomic() {
    let mut game = sniper_game(4);
    game.sniper_concentration = 2;
    let hp_before = game.player.hp;
    let rng_before = game.rng.to_save().draw_counter;
    resolve_ability(&mut game, TECHNIQUE_ABILITY_ID, TargetSelection::SelfTarget);
    assert_eq!(game.player.hp, hp_before);
    assert_eq!(game.sniper_concentration, 2);
    assert_eq!(game.rng.to_save().draw_counter, rng_before);

    game.sniper_concentration = 3;
    game.player.hp = 4;
    resolve_ability(&mut game, TECHNIQUE_ABILITY_ID, TargetSelection::SelfTarget);
    assert_eq!(game.player.hp, 4);
    assert_eq!(game.sniper_concentration, 3);
    assert_eq!(game.rng.to_save().draw_counter, rng_before);

    game.player.hp = hp_before;
    game.debug_ability_casts_succeed = true;
    resolve_ability(&mut game, TECHNIQUE_ABILITY_ID, TargetSelection::SelfTarget);
    assert_eq!(game.player.hp, hp_before - 5);
    assert_eq!(game.sniper_concentration, 0);

    game.player.hp = hp_before;
    game.sniper_concentration = 3;
    game.debug_ability_casts_succeed = false;
    let failure_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < 95
        })
        .expect("a failing percentile seed should exist");
    game.rng = RfbRng::seeded(failure_seed);
    let events = resolve_ability(&mut game, TECHNIQUE_ABILITY_ID, TargetSelection::SelfTarget);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
    );
    assert_eq!(game.player.hp, hp_before - 5);
    assert_eq!(game.sniper_concentration, 0);
}

#[test]
fn bolt_and_focus_multiply_the_critical_chance_without_affecting_other_classes() {
    let mut game = sniper_game(5);
    let seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            let roll = rng.bounded(5_000) + 1;
            (231..=448).contains(&roll)
        })
        .expect("a discriminating critical seed should exist");
    game.rng = RfbRng::seeded(seed);
    assert_eq!(
        game.roll_projectile_critical_multiplier(2, 10, 100, AmmunitionTypeDefinition::Arrow, 0,),
        100
    );
    game.rng = RfbRng::seeded(seed);
    assert!(
        game.roll_projectile_critical_multiplier(2, 10, 100, AmmunitionTypeDefinition::Bolt, 3,)
            > 100
    );

    let mut warrior = Game::new(5);
    let before = warrior.rng.to_save().draw_counter;
    assert_eq!(
        warrior.roll_projectile_critical_multiplier(2, 10, 100, AmmunitionTypeDefinition::Bolt, 3,),
        100
    );
    assert_eq!(warrior.rng.to_save().draw_counter, before);
}

#[test]
fn sniper_state_round_trips_and_rejects_invalid_build_or_bounds() {
    let mut game = sniper_game(6);
    game.sniper_concentration = 2;
    let probed_kind_id = game
        .content
        .actor_definitions()
        .next()
        .expect("content should have actors")
        .id
        .clone();
    game.probed_actor_kind_ids.insert(probed_kind_id.clone());
    let hash = game.state_hash();
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("Sniper state should round-trip");
    assert_eq!(restored.sniper_concentration, 2);
    assert_eq!(
        restored.probed_actor_kind_ids,
        BTreeSet::from([probed_kind_id])
    );
    assert_eq!(restored.state_hash(), hash);

    let mut excessive = game.to_save();
    excessive.player.sniper_concentration = 3;
    assert!(matches!(
        Game::from_save_with_content(excessive, game.content.clone()),
        Err(CoreError::InvalidSave("player sniper state is invalid"))
    ));

    let mut non_sniper = Game::new(6).to_save();
    non_sniper.player.sniper_concentration = 1;
    assert!(matches!(
        Game::from_save(non_sniper),
        Err(CoreError::InvalidSave("player sniper state is invalid"))
    ));
}
