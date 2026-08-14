// SPDX-License-Identifier: MPL-2.0

use std::sync::OnceLock;

use super::support::{clear_monsters, dispatch_next, give_inventory_item, replace_terrain};
use super::*;
use crate::game::player_combat::ProjectileMode;

const SNIPER_BUILD_ID: &str = "test.build.sniper";
const CONCENTRATE_ABILITY_ID: &str = "test.ability.sniper-concentrate";
const TECHNIQUE_ABILITY_ID: &str = "test.ability.sniper-technique";
const SHINING_SHOT_ABILITY_ID: &str = "test.ability.sniper-shining-shot";
const PROBE_ABILITY_ID: &str = "test.ability.sniper-probe";
const FORMAL_SNIPER_BUILD_ID: &str = "demo.build.sniper";

fn formal_sniper_game(seed: u64) -> Game {
    Game::new_with_build(seed, FORMAL_SNIPER_BUILD_ID).expect("Sniper build should create")
}

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
            let mut shining_shot = concentrate.clone();
            shining_shot.id = SHINING_SHOT_ABILITY_ID.to_owned();
            shining_shot.name_key = "test-sniper-shining-shot-name".to_owned();
            shining_shot.description_key = "test-sniper-shining-shot-description".to_owned();
            shining_shot.target = rfb_content::AbilityTargetDefinition {
                modes: vec![
                    rfb_content::AbilityTargetModeDefinition::Direction,
                    rfb_content::AbilityTargetModeDefinition::Position,
                    rfb_content::AbilityTargetModeDefinition::Entity,
                ],
                range: 20,
                requires_line_of_effect: true,
            };
            shining_shot.effect = AbilityEffectDefinition::SniperShot {
                mode: SniperShotModeDefinition::Shining,
            };
            let mut probe = concentrate.clone();
            probe.id = PROBE_ABILITY_ID.to_owned();
            probe.name_key = "test-sniper-probe-name".to_owned();
            probe.description_key = "test-sniper-probe-description".to_owned();
            probe.effect = AbilityEffectDefinition::ProbeMonsters;
            artifact
                .content
                .abilities
                .extend([concentrate, technique, shining_shot, probe]);

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
                ClassAbilityDefinition {
                    ability_id: SHINING_SHOT_ABILITY_ID.to_owned(),
                    minimum_level: 1,
                    ui_group_name_key: None,
                    governing_attribute: None,
                    resource_id: None,
                    resource_cost: 0,
                    minimum_concentration: 1,
                    hit_point_cost: 1,
                    base_failure_percent: 0,
                    minimum_failure_percent: 0,
                },
                ClassAbilityDefinition {
                    ability_id: PROBE_ABILITY_ID.to_owned(),
                    minimum_level: 15,
                    ui_group_name_key: None,
                    governing_attribute: Some(TechniqueAttribute::Intelligence),
                    resource_id: None,
                    resource_cost: 0,
                    minimum_concentration: 0,
                    hit_point_cost: 20,
                    base_failure_percent: 80,
                    minimum_failure_percent: 0,
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
    game.items
        .iter_mut()
        .find(|item| item.id == "test.sniper-bolt")
        .expect("test bolt should exist")
        .quantity = 20;
}

#[test]
fn sniper_birth_uses_original_identity_skills_proficiencies_kit_and_techniques() {
    let game = formal_sniper_game(0x0053_4e49_5045_5200);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("Sniper should project its build");

    assert_eq!(build.build_id, FORMAL_SNIPER_BUILD_ID);
    assert_eq!(build.class_id, "demo.class.sniper");
    assert_eq!((build.life_percent, build.experience_percent), (100, 110));
    assert_eq!(snapshot.player.kind_id, "demo.actor.sniper-player");
    let attributes = snapshot.player.progress.attributes;
    assert_eq!(attributes.strength.effective, 15);
    assert_eq!(attributes.intelligence.effective, 12);
    assert_eq!(attributes.wisdom.effective, 12);
    assert_eq!(attributes.dexterity.effective, 15);
    assert_eq!(attributes.constitution.effective, 14);
    assert_eq!(attributes.charisma.effective, 13);

    let skill = |id: &str| {
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == id)
            .expect("original Sniper skill should be projected")
    };
    for (id, base, growth) in [
        ("demo.skill.disarming", 25, 12),
        ("demo.skill.device", 24, 10),
        ("demo.skill.saving-throw", 28, 10),
        ("demo.skill.stealth", 5, 0),
        ("demo.skill.search", 32, 0),
        ("demo.skill.perception", 28, 0),
        ("demo.skill.melee", 35, 12),
        ("demo.skill.ranged", 72, 28),
    ] {
        assert_eq!(
            (skill(id).base, skill(id).growth_per_ten_levels),
            (base, growth)
        );
    }

    assert_eq!(snapshot.player.progress.riding_proficiency.current, 0);
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 0);
    let light_crossbow = snapshot
        .player
        .progress
        .weapon_proficiencies
        .iter()
        .find(|entry| entry.item_kind_id == "demo.item.light-crossbow")
        .expect("Sniper light-crossbow proficiency");
    assert_eq!(
        (light_crossbow.current, light_crossbow.maximum),
        (4_000, 8_000)
    );

    for kind_id in [
        "demo.item.dagger",
        "demo.item.soft-leather-armour",
        "demo.item.light-crossbow",
    ] {
        assert!(game.items.iter().any(|item| {
            item.kind_id == kind_id && matches!(item.location, ItemLocation::Equipped { .. })
        }));
    }
    let bolts = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.bolt")
        .expect("Sniper should start with bolts");
    assert!((20..=30).contains(&bolts.quantity));

    let class = game
        .content
        .class("demo.class.sniper")
        .expect("Sniper class should exist");
    assert_eq!(class.base_hp, 4);
    assert_eq!(class.pet_upkeep_divisor, 40);
    let profile = class.sniping_profile.expect("Sniper profile");
    assert_eq!(
        profile.preferred_ammunition_type,
        AmmunitionTypeDefinition::Bolt
    );
    assert_eq!(profile.preferred_ammunition_to_hit_base, 10);
    assert_eq!(profile.preferred_ammunition_to_hit_level_divisor, 5);
    assert_eq!(profile.base_shot_excess_percent, 50);
    assert_eq!(profile.preferred_ammunition_critical_chance_percent, 150);

    let expected = [
        ("demo.ability.sniper-concentrate", 1, 0, 0),
        ("demo.ability.sniper-shining-arrow", 2, 1, 0),
        ("demo.ability.sniper-retreat-shot", 3, 1, 0),
        ("demo.ability.sniper-disarming-shot", 5, 1, 0),
        ("demo.ability.sniper-burning-shot", 8, 2, 0),
        ("demo.ability.sniper-shatter-shot", 10, 2, 0),
        ("demo.ability.sniper-freezing-shot", 13, 2, 0),
        ("demo.ability.sniper-knockback-shot", 18, 2, 0),
        ("demo.ability.sniper-piercing-shot", 22, 3, 0),
        ("demo.ability.sniper-evil-shot", 25, 4, 0),
        ("demo.ability.sniper-holy-shot", 26, 4, 0),
        ("demo.ability.sniper-exploding-shot", 30, 3, 0),
        ("demo.ability.sniper-double-shot", 32, 4, 0),
        ("demo.ability.sniper-thunder-shot", 36, 3, 0),
        ("demo.ability.sniper-needle-shot", 40, 3, 0),
        ("demo.ability.sniper-saint-stars-arrow", 48, 7, 0),
        ("demo.ability.sniper-probe-monsters", 15, 0, 20),
    ];
    assert_eq!(snapshot.player.abilities.len(), expected.len());
    for (id, level, concentration, hp) in expected {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .expect("formal Sniper ability");
        assert_eq!(ability.source, AbilitySourceDto::Class);
        assert_eq!(
            (
                ability.minimum_level,
                ability.minimum_concentration,
                ability.hit_point_cost,
            ),
            (level, concentration, hp)
        );
        assert_eq!(
            ability.ui_group_name_key.as_deref(),
            (id != "demo.ability.sniper-probe-monsters")
                .then_some("ability-group-demo-sniper-sniping-name")
        );
    }
    for (id, mode) in [
        (
            "demo.ability.sniper-shining-arrow",
            SniperShotModeDto::Shining,
        ),
        (
            "demo.ability.sniper-retreat-shot",
            SniperShotModeDto::Retreat,
        ),
        (
            "demo.ability.sniper-disarming-shot",
            SniperShotModeDto::Disarm,
        ),
        (
            "demo.ability.sniper-burning-shot",
            SniperShotModeDto::Burning,
        ),
        (
            "demo.ability.sniper-shatter-shot",
            SniperShotModeDto::Shatter,
        ),
        (
            "demo.ability.sniper-freezing-shot",
            SniperShotModeDto::Freezing,
        ),
        (
            "demo.ability.sniper-knockback-shot",
            SniperShotModeDto::Knockback,
        ),
        (
            "demo.ability.sniper-piercing-shot",
            SniperShotModeDto::Piercing,
        ),
        ("demo.ability.sniper-evil-shot", SniperShotModeDto::Evil),
        ("demo.ability.sniper-holy-shot", SniperShotModeDto::Holy),
        (
            "demo.ability.sniper-exploding-shot",
            SniperShotModeDto::Exploding,
        ),
        ("demo.ability.sniper-double-shot", SniperShotModeDto::Double),
        (
            "demo.ability.sniper-thunder-shot",
            SniperShotModeDto::Thunder,
        ),
        ("demo.ability.sniper-needle-shot", SniperShotModeDto::Needle),
        (
            "demo.ability.sniper-saint-stars-arrow",
            SniperShotModeDto::Final,
        ),
    ] {
        let ability = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .expect("formal Sniper shot");
        assert_eq!(ability.effects, [AbilityEffectSpecDto::SniperShot { mode }]);
    }
    let concentrate = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.sniper-concentrate")
        .expect("formal Concentrate");
    assert_eq!(concentrate.effects, [AbilityEffectSpecDto::Concentrate]);
    let probe = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.sniper-probe-monsters")
        .expect("formal Probe Monsters");
    assert_eq!(probe.effects, [AbilityEffectSpecDto::ProbeMonsters]);
}

fn prepare_shooting_line(game: &mut Game) {
    clear_monsters(game);
    game.player.position = Position { x: 10, y: 10 };
    for x in 10..=30 {
        let position = Position { x, y: 10 };
        replace_terrain(game, position, "demo.terrain.floor");
        let index = game
            .index(position)
            .expect("shooting line should be in bounds");
        game.glow[index] = false;
    }
    equip_light_crossbow(game);
}

fn push_durable_sheep(game: &mut Game, id: &str, position: Position) {
    game.push_generated_actor(id.to_owned(), "demo.actor.sheep", position);
    let actor = game
        .entities
        .iter_mut()
        .find(|actor| actor.id == id)
        .expect("test sheep should exist");
    actor.hp = 1_000;
    actor.max_hp = 1_000;
    actor.alerted = false;
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

fn fire_mode(game: &mut Game, mode: SniperShotModeDefinition) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_projectile(
        TargetSelection::Direction {
            direction: Direction::East,
        },
        ProjectileMode::Sniper(mode),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("test sniper shot should resolve");
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
        BTreeSet::from([probed_kind_id.clone()])
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

    let mut duplicate_knowledge = game.to_save();
    duplicate_knowledge.player.probed_actor_kind_ids = vec![probed_kind_id.clone(), probed_kind_id];
    assert!(matches!(
        Game::from_save_with_content(duplicate_knowledge, game.content.clone()),
        Err(CoreError::InvalidSave(
            "player probed actor knowledge is invalid"
        ))
    ));

    let mut unknown_knowledge = game.to_save();
    unknown_knowledge.player.probed_actor_kind_ids = vec!["test.actor.missing".to_owned()];
    assert!(matches!(
        Game::from_save_with_content(unknown_knowledge, game.content.clone()),
        Err(CoreError::InvalidSave(
            "player probed actor knowledge is invalid"
        ))
    ));
}

#[test]
fn shining_disarming_and_shatter_shots_mutate_only_the_projectile_path() {
    let mut shining = sniper_game(7);
    prepare_shooting_line(&mut shining);
    shining.sniper_concentration = 2;
    fire_mode(&mut shining, SniperShotModeDefinition::Shining);
    let lit = (11..=30)
        .take_while(|x| {
            shining
                .index(Position { x: *x, y: 10 })
                .is_some_and(|index| shining.glow[index])
        })
        .count();
    assert!(lit > 1);
    assert_eq!(shining.sniper_concentration, 0);

    let mut disarm = sniper_game(8);
    prepare_shooting_line(&mut disarm);
    let trap = Position { x: 13, y: 10 };
    replace_terrain(&mut disarm, trap, "demo.terrain.created-trap");
    fire_mode(&mut disarm, SniperShotModeDefinition::Disarm);
    assert_eq!(disarm.terrain_at(trap), "demo.terrain.floor");

    let mut shatter = sniper_game(9);
    prepare_shooting_line(&mut shatter);
    let wall = Position { x: 13, y: 10 };
    replace_terrain(&mut shatter, wall, "demo.terrain.quartz-treasure");
    let mining_before = shatter.progress.mining_proficiency;
    let materials_before = shatter.progress.materials.clone();
    let gold_before = shatter.gold_piles.len();
    let events = fire_mode(&mut shatter, SniperShotModeDefinition::Shatter);
    assert_eq!(shatter.terrain_at(wall), "demo.terrain.floor");
    assert_eq!(shatter.progress.mining_proficiency, mining_before);
    assert_eq!(shatter.progress.materials, materials_before);
    assert_eq!(shatter.gold_piles.len(), gold_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::ProjectileAmmoBroken { ammo_kind_id }
            if ammo_kind_id == "demo.item.bolt"
    )));
}

#[test]
fn knockback_and_piercing_share_collision_training_and_focus_rules() {
    let mut knockback = sniper_game(10);
    prepare_shooting_line(&mut knockback);
    let target = Position { x: 12, y: 10 };
    push_durable_sheep(&mut knockback, "test.knockback", target);
    knockback.sniper_concentration = 2;
    let hit_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) >= 10
        })
        .expect("a normal projectile hit seed should exist");
    knockback.rng = RfbRng::seeded(hit_seed);
    fire_mode(&mut knockback, SniperShotModeDefinition::Knockback);
    let pushed = knockback
        .entities
        .iter()
        .find(|actor| actor.id == "test.knockback")
        .expect("surviving target should remain")
        .position;
    assert!(pushed.x >= target.x + 4);
    assert_eq!(pushed.y, target.y);

    let mut piercing = sniper_game(11);
    prepare_shooting_line(&mut piercing);
    for (ordinal, x) in [12, 14, 16, 18, 20].into_iter().enumerate() {
        push_durable_sheep(
            &mut piercing,
            &format!("test.piercing.{ordinal}"),
            Position { x, y: 10 },
        );
    }
    piercing.sniper_concentration = 3;
    let events = fire_mode(&mut piercing, SniperShotModeDefinition::Piercing);
    let collisions = events
        .iter()
        .filter(|event| {
            matches!(
                event,
                DomainEvent::ProjectileHit { .. } | DomainEvent::ProjectileMissed { .. }
            )
        })
        .count();
    assert_eq!(collisions, 4);
    assert!(piercing.entities[0..4].iter().all(|actor| actor.alerted));
    assert!(!piercing.entities[4].alerted);
    assert_eq!(piercing.sniper_concentration, 0);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileAmmoBroken { .. }))
    );
}

#[test]
fn retreat_uses_focus_scaled_range_and_special_abilities_use_shot_energy() {
    let mut retreat = sniper_game(12);
    prepare_shooting_line(&mut retreat);
    let origin = retreat.player.position;
    retreat.sniper_concentration = 3;
    fire_mode(&mut retreat, SniperShotModeDefinition::Retreat);
    assert_ne!(retreat.player.position, origin);
    assert!(chebyshev_distance(origin, retreat.player.position) <= 16);

    let mut special = sniper_game(13);
    prepare_shooting_line(&mut special);
    special.sniper_concentration = 1;
    special.debug_ability_casts_succeed = true;
    let hp_before = special.player.hp;
    let projected = special
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == SHINING_SHOT_ABILITY_ID)
        .expect("test special shot should project");
    assert_eq!(
        projected.target_spec.range,
        special
            .player_projectile_profile()
            .expect("projectile profile")
            .range
    );
    assert!(matches!(
        projected.effects.as_slice(),
        [AbilityEffectSpecDto::SniperShot {
            mode: SniperShotModeDto::Shining
        }]
    ));
    assert!(projected.can_cast);
    let expected_energy = special
        .player_projectile_profile()
        .expect("projectile profile")
        .energy_cost;
    let energy_gain = energy_gain(derived_speed(&special.player_derived_stats().speed));
    let tick_before = special.world_tick;
    dispatch_next(
        &mut special,
        GameCommand::CastAbility {
            ability_id: SHINING_SHOT_ABILITY_ID.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert_eq!(special.player.hp, hp_before - 1);
    assert_eq!(special.sniper_concentration, 0);
    assert_eq!(
        special.world_tick - tick_before,
        u32::try_from((expected_energy + energy_gain - 1) / energy_gain).unwrap()
    );
}

#[test]
fn double_shot_uses_two_instances_and_degrades_to_one_when_ammunition_is_short() {
    let mut double = sniper_game(15);
    prepare_shooting_line(&mut double);
    double.sniper_concentration = 5;
    let serial_before = double.next_item_instance_serial;
    fire_mode(&mut double, SniperShotModeDefinition::Double);
    assert_eq!(
        double
            .items
            .iter()
            .find(|item| item.id == "test.sniper-bolt")
            .expect("remaining bolt stack")
            .quantity,
        18
    );
    assert_eq!(double.next_item_instance_serial, serial_before + 2);
    assert_eq!(
        double
            .items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.bolt" && matches!(item.location, ItemLocation::Ground(_))
            })
            .count(),
        2
    );
    assert_eq!(double.sniper_concentration, 0);

    let mut fallback = sniper_game(16);
    prepare_shooting_line(&mut fallback);
    fallback
        .items
        .iter_mut()
        .find(|item| item.id == "test.sniper-bolt")
        .expect("bolt stack")
        .quantity = 1;
    let serial_before = fallback.next_item_instance_serial;
    fire_mode(&mut fallback, SniperShotModeDefinition::Double);
    assert_eq!(fallback.next_item_instance_serial, serial_before);
    assert_eq!(
        fallback
            .items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.bolt" && matches!(item.location, ItemLocation::Ground(_))
            })
            .count(),
        1
    );
}

#[test]
fn exploding_shot_uses_focus_radius_and_final_shot_applies_original_recoil() {
    let mut explosion = sniper_game(17);
    prepare_shooting_line(&mut explosion);
    for y in 7..=13 {
        for x in 10..=18 {
            replace_terrain(&mut explosion, Position { x, y }, "demo.terrain.floor");
        }
    }
    push_durable_sheep(
        &mut explosion,
        "test.explosion.center",
        Position { x: 12, y: 10 },
    );
    push_durable_sheep(
        &mut explosion,
        "test.explosion.inside",
        Position { x: 12, y: 13 },
    );
    push_durable_sheep(
        &mut explosion,
        "test.explosion.outside",
        Position { x: 16, y: 10 },
    );
    explosion.sniper_concentration = 3;
    let hit_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) >= 10
        })
        .expect("a normal projectile hit seed should exist");
    explosion.rng = RfbRng::seeded(hit_seed);
    fire_mode(&mut explosion, SniperShotModeDefinition::Exploding);
    assert!(
        explosion
            .entities
            .iter()
            .find(|actor| actor.id == "test.explosion.inside")
            .expect("inside target")
            .hp
            < 1_000
    );
    assert_eq!(
        explosion
            .entities
            .iter()
            .find(|actor| actor.id == "test.explosion.outside")
            .expect("outside target")
            .hp,
        1_000
    );

    let mut final_shot = sniper_game(18);
    prepare_shooting_line(&mut final_shot);
    let events = fire_mode(&mut final_shot, SniperShotModeDefinition::Final);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ProjectileAmmoRecovered { .. }))
    );
    let slow = final_shot
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_SLOW)
        .expect("final shot should slow the player");
    let stun = final_shot
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == STATUS_STUN)
        .expect("final shot should stun the player");
    assert!((7..=13).contains(&slow.remaining_ticks));
    assert!((1..=25).contains(&stun.remaining_ticks));
}

#[test]
fn probe_projects_each_visible_projectable_monster_and_records_lore_by_kind() {
    let mut game = sniper_game(19);
    prepare_shooting_line(&mut game);
    for y in 9..=11 {
        for x in 10..=16 {
            let position = Position { x, y };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            let index = game.index(position).expect("probe cell");
            game.glow[index] = true;
        }
    }
    push_durable_sheep(&mut game, "test.probe.one", Position { x: 12, y: 10 });
    push_durable_sheep(&mut game, "test.probe.two", Position { x: 12, y: 11 });
    replace_terrain(&mut game, Position { x: 13, y: 10 }, "demo.terrain.wall");
    push_durable_sheep(&mut game, "test.probe.blocked", Position { x: 15, y: 10 });
    game.progress.level = 15;
    game.player.hp = 50;
    game.player.max_hp = 50;
    game.debug_ability_casts_succeed = true;
    let projected_ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == PROBE_ABILITY_ID)
        .expect("probe should be projected at level fifteen");
    assert_eq!(projected_ability.minimum_level, 15);
    assert_eq!(projected_ability.hit_point_cost, 20);
    assert_eq!(projected_ability.resource_id, None);
    assert!(matches!(
        projected_ability.effects.as_slice(),
        [AbilityEffectSpecDto::ProbeMonsters]
    ));

    let events = resolve_ability(&mut game, PROBE_ABILITY_ID, TargetSelection::SelfTarget);
    let resolution = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityMonstersProbed { resolution, .. } => Some(resolution),
            _ => None,
        })
        .expect("probe result should be emitted");
    let projected = events
        .iter()
        .find(|event| matches!(event, DomainEvent::AbilityMonstersProbed { .. }))
        .expect("probe event")
        .clone()
        .into_dto();
    assert_eq!(projected.kind, "ability.monsters-probed");
    assert!(matches!(
        projected.outcome,
        Some(GameEventOutcomeDto::AbilityMonsterProbe { ref resolution })
            if resolution.monsters.len() == 2
    ));
    assert_eq!(
        resolution
            .monsters
            .iter()
            .map(|monster| monster.entity_id.as_str())
            .collect::<Vec<_>>(),
        ["test.probe.one", "test.probe.two"]
    );
    assert!(resolution.monsters.iter().all(|monster| {
        monster.kind_id == "demo.actor.sheep"
            && monster.hp == 1_000
            && monster.max_hp == 1_000
            && monster.melee_routine.blows.len()
                == game
                    .content
                    .actor("demo.actor.sheep")
                    .expect("sheep definition")
                    .melee_routine
                    .as_ref()
                    .map_or(0, |routine| routine.blows.len())
    }));
    assert_eq!(game.player.hp, 30);
    assert_eq!(
        game.probed_actor_kind_ids,
        BTreeSet::from(["demo.actor.sheep".to_owned()])
    );
}

#[test]
fn ranged_easy_tiring_uses_the_original_extra_chance_after_a_real_shot() {
    let mut game = sniper_game(14);
    prepare_shooting_line(&mut game);
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.easy-tiring2".to_owned());
    let fatigue_seed = (0..10_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(16) != 0 && rng.bounded(6) == 0
        })
        .expect("an extra ranged-fatigue seed should exist");
    game.rng = RfbRng::seeded(fatigue_seed);
    let mut events = Vec::new();
    game.resolve_player_projectile(
        TargetSelection::Direction {
            direction: Direction::East,
        },
        ProjectileMode::Normal,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("ordinary shot should resolve");
    assert_eq!(game.minor_slow, 1);
}
