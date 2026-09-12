// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::tests::support::replace_terrain;
use rfb_protocol::{AbilityControlOutcomeDto, ItemFeelingDto, TargetModeDto};

fn id(slug: &str) -> String {
    format!("demo.ability.mindcrafter-{slug}")
}

fn arena(level: u16) -> Game {
    let mut game = mindcrafter(level);
    game.player.position = Position { x: 3, y: 3 };
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.debug_set_ability_casts_succeed(true);
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    game
}

fn projection(game: &Game, slug: &str) -> rfb_protocol::AbilityDto {
    game.snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == id(slug))
        .unwrap()
}

fn cast(game: &mut Game, slug: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        &id(slug),
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

fn self_cast(game: &mut Game, slug: &str) -> Vec<DomainEvent> {
    cast(game, slug, TargetSelection::SelfTarget)
}

fn target(game: &mut Game, x: i32) -> usize {
    let index = game.entities.len();
    game.entities.push(actor_from_runtime_spawn(
        &format!("test.actor.mental-{index}"),
        "demo.actor.gloom-weaver",
        Position { x, y: 3 },
        2000,
        100,
        100,
        true,
    ));
    index
}

fn seeded(predicate: impl Fn(&mut RfbRng) -> bool) -> RfbRng {
    (0..100_000)
        .map(RfbRng::seeded)
        .find(|rng| predicate(&mut rng.clone()))
        .expect("branch seed")
}

fn effects(events: &[DomainEvent]) -> impl Iterator<Item = &AbilityEffectResolutionDto> {
    events
        .iter()
        .filter_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                Some(resolution.effects.iter())
            }
            _ => None,
        })
        .flatten()
}

#[test]
fn neural_blast_uses_a_position_ball_or_a_beam_and_matches_its_dice() {
    let mut ball = arena(1);
    target(&mut ball, 5);
    target(&mut ball, 7);
    ball.rng = seeded(|rng| {
        rng.bounded(100);
        rng.bounded(100) >= 1
    });
    let events = cast(
        &mut ball,
        "neural-blast",
        TargetSelection::Position {
            position: Position { x: 7, y: 3 },
        },
    );
    assert_eq!(ball.entities[0].hp, 2000);
    assert!(ball.entities[1].hp < 2000, "{events:?}");
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.radius == 0 && (3..=9).contains(&resolution.base_raw_damage))));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::BoltReflected { .. }))
    );
    ball.entities[1].kind_id = "demo.actor.buzzy-beetle".to_owned();
    let events = cast(
        &mut ball,
        "neural-blast",
        TargetSelection::Position {
            position: Position { x: 7, y: 3 },
        },
    );
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityHit { target_kind_id, .. } if target_kind_id == "demo.actor.buzzy-beetle")));
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::BoltReflected { .. }))
    );

    let mut beam = arena(50);
    target(&mut beam, 5);
    target(&mut beam, 7);
    beam.rng = seeded(|rng| {
        rng.bounded(100);
        rng.bounded(100) < 99
    });
    let events = cast(
        &mut beam,
        "neural-blast",
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(beam.entities.iter().all(|actor| actor.hp < 2000));
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityBeamDamage { resolution, .. } if resolution.target_count == 2 && (15..=90).contains(&resolution.base_raw_damage))));
}

#[test]
fn precognition_changes_detection_mapping_telepathy_and_cost_at_actual_boundaries() {
    for (level, cost) in [
        (19, 1),
        (20, 2),
        (24, 2),
        (25, 4),
        (29, 4),
        (30, 5),
        (39, 5),
        (40, 5),
        (44, 5),
        (45, 10),
    ] {
        let mut game = arena(level);
        let stairs = Position { x: 8, y: 3 };
        replace_terrain(&mut game, stairs, "demo.terrain.stairs-down");
        let before = game.resources[MANA].current;
        assert_eq!(projection(&game, "precognition").resource_cost, cost);
        let events = self_cast(&mut game, "precognition");
        assert_eq!(game.resources[MANA].current, before - cost);
        let categories = events
            .iter()
            .filter_map(|event| match event {
                DomainEvent::AbilityDetected { resolution, .. } => {
                    Some(resolution.category.as_str())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(categories.contains(&"normal-monster") && categories.contains(&"invisible"));
        assert_eq!(categories.contains(&"map"), level >= 20);
        assert_eq!(categories.contains(&"passage"), level >= 30);
        assert_eq!(
            game.player_has_status_kind(STATUS_TELEPATHY),
            (25..40).contains(&level)
        );
        if level >= 30 {
            assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityDetected { resolution, .. } if resolution.category == "passage" && resolution.detected_positions.contains(&stairs))));
        }
    }
}

#[test]
fn displacement_becomes_dimension_door_and_invalid_selection_does_not_pay() {
    let mut game = arena(44);
    assert_eq!(projection(&game, "minor-displacement").resource_cost, 2);
    let before = game.player.position;
    self_cast(&mut game, "minor-displacement");
    assert_ne!(game.player.position, before);
    let before = game.player.position;
    self_cast(&mut game, "major-displacement");
    assert_ne!(game.player.position, before);

    let mut game = arena(45);
    let ability = projection(&game, "minor-displacement");
    assert_eq!(ability.resource_cost, 42);
    assert_eq!(ability.name_key, "ability-demo-sorcery-dimension-door-name");
    assert_eq!(ability.target_spec.modes, [TargetModeDto::Position]);
    let mana = game.resources[MANA].current;
    let rng = game.rng.clone();
    let events = self_cast(&mut game, "minor-displacement");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityTargetUnavailable { .. }))
    );
    assert_eq!(game.resources[MANA].current, mana);
    assert_eq!(game.rng.clone().bounded(1000), rng.clone().bounded(1000));
    let destination = Position { x: 8, y: 3 };
    game.rng = seeded(|rng| {
        rng.bounded(100);
        rng.bounded(45 * 45 / 2) != 0
    });
    cast(
        &mut game,
        "minor-displacement",
        TargetSelection::Position {
            position: destination,
        },
    );
    assert_eq!(game.player.position, destination);
    assert_eq!(game.resources[MANA].current, mana - 42);
}

#[test]
fn psychometry_upgrades_a_weak_feeling_then_identifies_at_twenty_and_accepts_potions() {
    let mut game = arena(19);
    give_inventory_item(&mut game, "test.sword", "demo.item.small-sword");
    game.items.last_mut().unwrap().enchantments.to_hit = 1;
    game.item_property_knowledge
        .entry("test.sword".to_owned())
        .or_default()
        .feeling = Some(ItemFeelingDto::Enchanted);
    cast(
        &mut game,
        "psychometry",
        TargetSelection::Item {
            item_id: "test.sword".to_owned(),
        },
    );
    assert_eq!(
        game.item_property_knowledge["test.sword"].feeling,
        Some(ItemFeelingDto::Good)
    );
    assert!(!game.item_property_knowledge["test.sword"].appraised);
    give_inventory_item(&mut game, "test.potion", "demo.item.swiftstep-tonic");
    cast(
        &mut game,
        "psychometry",
        TargetSelection::Item {
            item_id: "test.potion".to_owned(),
        },
    );
    assert_eq!(
        game.item_property_knowledge["test.potion"].feeling,
        Some(ItemFeelingDto::Average)
    );
    game.progress.level = 20;
    cast(
        &mut game,
        "psychometry",
        TargetSelection::Item {
            item_id: "test.sword".to_owned(),
        },
    );
    assert!(game.item_property_knowledge["test.sword"].appraised);
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    let before = game.resources[MANA].current;
    let events = cast(
        &mut game,
        "psychometry",
        TargetSelection::Item {
            item_id: "test.sword".to_owned(),
        },
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
    );
    assert_eq!(game.resources[MANA].current, before - 12);
}

#[test]
fn armor_reuses_one_duration_roll_and_adrenaline_heals_only_before_both_buffs_exist() {
    for (level, count) in [
        (19, 1),
        (20, 2),
        (24, 2),
        (25, 3),
        (29, 3),
        (30, 4),
        (34, 4),
        (35, 5),
    ] {
        let mut game = arena(level);
        let mut expected = game.rng.clone();
        expected.bounded(100);
        let duration = u32::from(level) + expected.bounded(u64::from(level)) as u32 + 1;
        self_cast(&mut game, "character-armor");
        let buffs = &game.player.statuses;
        assert_eq!(buffs.len(), count + 1);
        assert!(
            buffs
                .iter()
                .all(|status| status.remaining_ticks == duration)
        );
        assert_eq!(game.rng.clone().bounded(1000), expected.bounded(1000));
        assert_eq!(
            buffs
                .iter()
                .find(|status| status.kind_id == "rfb.status.stone-skin")
                .unwrap()
                .granted_modifiers
                .defense,
            50
        );
    }
    let mut game = arena(23);
    game.player.hp = 1;
    game.apply_player_mental_status(STATUS_STUN, 10, "test");
    self_cast(&mut game, "adrenaline");
    assert!(!game.player_has_status_kind(STATUS_STUN));
    assert!(game.player_has_status_kind(STATUS_HASTE));
    assert_eq!(game.player.hp, 24);
    game.player.hp = 1;
    self_cast(&mut game, "adrenaline");
    assert_eq!(game.player.hp, 1);
}

#[test]
fn mind_wave_uses_cast_damage_and_switches_to_line_of_sight_at_twenty_five() {
    let mut game = arena(24);
    target(&mut game, 5);
    let events = self_cast(&mut game, "mind-wave");
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.base_raw_damage == 72 && resolution.radius == 4)));
    assert!(game.entities[0].hp < 2000);
    let mut game = arena(25);
    target(&mut game, 10);
    game.apply_player_mental_status(STATUS_BLINDNESS, 20, "test.blind");
    assert!(!game.entity_is_visible_to_player(&game.entities[0]));
    let events = self_cast(&mut game, "mind-wave");
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityVisibleDamage { resolution, .. } if (1..=75).contains(&resolution.base_raw_damage))));
    assert!(game.entities[0].hp < 2000);
    assert_eq!(projection(&game, "mind-wave").area_radius, None);
}

#[test]
fn pulverise_grows_its_radius_and_telekinesis_fetches_a_real_item() {
    for (level, radius) in [(20, 0), (21, 1), (28, 2), (36, 3)] {
        let mut game = arena(level);
        target(&mut game, 6);
        let events = cast(
            &mut game,
            "pulverise",
            TargetSelection::Position {
                position: Position { x: 6, y: 3 },
            },
        );
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.radius == radius)));
        assert!(game.entities[0].hp < 2000);
    }
    let mut game = arena(26);
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
    give_inventory_item(&mut game, "test.fetch", "demo.item.small-sword");
    game.items.last_mut().unwrap().location = ItemLocation::Ground(Position { x: 7, y: 3 });
    let events = cast(
        &mut game,
        "telekinesis",
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert!(
        effects(&events)
            .any(|effect| matches!(effect, AbilityEffectResolutionDto::FetchItem { .. }))
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.fetch")
            .unwrap()
            .location,
        ItemLocation::Ground(game.player.position)
    );
}

#[test]
fn domination_changes_from_single_status_to_mass_charm_without_hit_point_damage() {
    let mut game = arena(29);
    target(&mut game, 5);
    game.rng = seeded(|rng| {
        rng.bounded(100);
        let monster = rng.bounded(3) + 1;
        let power = rng.bounded(29) + 1;
        monster <= power
    });
    let events = cast(
        &mut game,
        "domination",
        TargetSelection::Direction {
            direction: Direction::East,
        },
    );
    assert_eq!(game.entities[0].hp, 2000);
    assert!(!game.entities[0].statuses.is_empty());
    assert!(effects(&events).any(|effect| matches!(
        effect,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Affected,
            ..
        }
    )));
    let mut game = arena(30);
    target(&mut game, 5);
    target(&mut game, 7);
    game.rng = seeded(|rng| {
        rng.bounded(100);
        rng.bounded(60) > 10 && rng.bounded(60) > 10
    });
    let events = self_cast(&mut game, "domination");
    assert!(
        game.entities
            .iter()
            .all(|actor| actor.controller_id.as_deref() == Some(&game.player.id))
    );
    assert!(effects(&events).all(|effect| matches!(
        effect,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Controlled,
            ..
        }
    )));
    assert_eq!(
        projection(&game, "domination").target_spec.modes,
        [TargetModeDto::SelfTarget]
    );
}

#[test]
fn drain_recovers_mana_and_adds_energy_while_spear_and_storm_have_distinct_damage_types() {
    let mut game = arena(50);
    target(&mut game, 6);
    game.resources.get_mut(MANA).unwrap().current = 20;
    let events = cast(
        &mut game,
        "psychic-drain",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert!(game.resources[MANA].current > 10);
    assert!(effects(&events).any(|effect| matches!(
        effect,
        AbilityEffectResolutionDto::ExtraEnergy {
            amount: 1..=150,
            ..
        }
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ResourceRecovered { .. }))
    );
    for (slug, kind) in [
        ("psycho-spear", DamageTypeDto::PsySpear),
        ("psycho-storm", DamageTypeDto::PsiStorm),
    ] {
        let mut game = arena(50);
        target(&mut game, 6);
        let events = cast(
            &mut game,
            slug,
            TargetSelection::Position {
                position: Position { x: 6, y: 3 },
            },
        );
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityHit { damage, .. } if DamageTypeDto::from(damage.damage_type) == kind)), "{events:?}");
        assert!(game.entities[0].hp < 2000);
    }
}

#[test]
fn failed_spells_follow_all_five_backlash_branches_and_preserve_fully_identified_items() {
    for branch in [1, 5, 15, 45, 90] {
        let mut game = arena(45);
        game.debug_set_ability_casts_succeed(false);
        give_inventory_item(&mut game, "test.partial", "demo.item.small-sword");
        give_inventory_item(&mut game, "test.full", "demo.item.small-sword");
        for (item_id, full) in [("test.partial", false), ("test.full", true)] {
            game.identify_item_instance(item_id, ItemIdentificationRequest::new(full));
        }
        game.explored.fill(true);
        if branch == 90 {
            game.player.hp = 1;
        }
        let awareness = game.item_knowledge.clone();
        let mana = game.resources[MANA].current;
        let hp = game.player.hp;
        let failure = projection(&game, "psycho-storm").failure_percent;
        assert!(failure > 2);
        game.rng = seeded(|rng| {
            rng.bounded(100) < u64::from(failure)
                && rng.bounded(100) + 1 < u64::from(failure / 2)
                && rng.bounded(100) + 1 == branch
        });
        let events = cast(
            &mut game,
            "psycho-storm",
            TargetSelection::Position {
                position: Position { x: 6, y: 3 },
            },
        );
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
        );
        assert!(effects(&events).any(|effect| matches!(effect, AbilityEffectResolutionDto::MindcraftBacklash { roll, .. } if u64::from(*roll) == branch)));
        match branch {
            1 => {
                assert!(!game.item_property_knowledge["test.partial"].appraised);
                assert!(game.item_property_knowledge["test.full"].identified);
                assert_eq!(game.item_knowledge, awareness);
                assert!(game.explored.iter().all(|explored| !explored));
            }
            5 => assert!(
                (6..=15).contains(
                    &game
                        .player
                        .statuses
                        .iter()
                        .find(|status| status.kind_id == STATUS_HALLUCINATION)
                        .unwrap()
                        .remaining_ticks
                )
            ),
            15 => assert!(
                (1..=8).contains(
                    &game
                        .player
                        .statuses
                        .iter()
                        .find(|status| status.kind_id == STATUS_CONFUSION)
                        .unwrap()
                        .remaining_ticks
                )
            ),
            45 => assert!(
                (1..=8).contains(
                    &game
                        .player
                        .statuses
                        .iter()
                        .find(|status| status.kind_id == STATUS_STUN)
                        .unwrap()
                        .intensity
                )
            ),
            90 => {
                assert_eq!(game.player.hp, hp - 90);
                assert_eq!(game.resources[MANA].current, mana.saturating_sub(50 + 180));
                assert!(
                    events
                        .iter()
                        .any(|event| matches!(event, DomainEvent::PlayerDied { .. }))
                );
                assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityAreaDamage { resolution, .. } if resolution.radius == 6 && resolution.base_raw_damage == 90)));
            }
            _ => unreachable!(),
        }
        if branch != 90 {
            assert_eq!(game.resources[MANA].current, mana - 50);
        }
    }
}

#[test]
fn backlash_gate_is_strict_and_clear_mind_never_calls_it() {
    let mut game = arena(45);
    game.debug_set_ability_casts_succeed(false);
    let failure = projection(&game, "psycho-storm").failure_percent;
    game.rng = seeded(|rng| {
        rng.bounded(100) < u64::from(failure) && rng.bounded(100) + 1 == u64::from(failure / 2)
    });
    let mut expected = game.rng.clone();
    expected.bounded(100);
    expected.bounded(100);
    let events = cast(
        &mut game,
        "psycho-storm",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert!(
        !effects(&events)
            .any(|effect| matches!(effect, AbilityEffectResolutionDto::MindcraftBacklash { .. }))
    );
    assert_eq!(game.rng.clone().bounded(1000), expected.bounded(1000));
    let mut game = arena(15);
    game.debug_set_ability_casts_succeed(false);
    let failure = projection(&game, "clear-mind").failure_percent;
    game.rng = seeded(|rng| rng.bounded(100) < u64::from(failure));
    let mut expected = game.rng.clone();
    expected.bounded(100);
    let events = self_cast(&mut game, "clear-mind");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
    );
    assert_eq!(game.rng.clone().bounded(1000), expected.bounded(1000));
}

fn psychic_target_game(tags: &[&str], level: u32) -> Game {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let definition = artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.gloom-weaver")
        .unwrap();
    definition.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
    definition.level = level;
    definition.status_immunities.clear();
    let mut game = arena(50);
    game.content = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    target(&mut game, 6);
    game
}

#[test]
fn psychic_damage_obeys_mind_traits_and_preserves_drain_notice_on_immunity() {
    let mut empty = psychic_target_game(&["empty-mind"], 1);
    for kind in [DamageType::Psi, DamageType::PsiDrain, DamageType::PsiStorm] {
        let before = empty.rng.clone();
        let result = empty.prepare_psychic_damage(0, "test", kind, 90, &mut Vec::new());
        assert_eq!(result.damage, 0);
        assert_eq!(
            empty.rng.clone().bounded(1000),
            before.clone().bounded(1000)
        );
    }
    let events = cast(
        &mut empty,
        "psychic-drain",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert_eq!(empty.entities[0].hp, 2000);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::ResourceRecovered { .. }))
    );
    assert!(
        effects(&events)
            .any(|effect| matches!(effect, AbilityEffectResolutionDto::ExtraEnergy { .. }))
    );
    empty.apply_player_mental_status(STATUS_BLINDNESS, 20, "test.blind");
    assert!(!empty.entity_is_visible_to_player(&empty.entities[0]));
    let hidden_events = cast(
        &mut empty,
        "psychic-drain",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert!(
        !effects(&hidden_events)
            .any(|effect| matches!(effect, AbilityEffectResolutionDto::ExtraEnergy { .. }))
    );
    for tag in ["animal", "stupid", "weird-mind"] {
        let mut game = psychic_target_game(&[tag], 1);
        assert_eq!(
            game.prepare_psychic_damage(0, "test", DamageType::Psi, 90, &mut Vec::new())
                .damage,
            30
        );
        assert_eq!(
            game.prepare_psychic_damage(0, "test", DamageType::PsiDrain, 90, &mut Vec::new())
                .damage,
            30
        );
        assert_eq!(
            game.prepare_psychic_damage(0, "test", DamageType::PsiStorm, 90, &mut Vec::new())
                .damage,
            if tag == "animal" { 90 } else { 30 }
        );
    }
    let mut protected = psychic_target_game(&["resist-all"], 1);
    assert_eq!(
        protected
            .prepare_psychic_damage(0, "test", DamageType::PsySpear, 90, &mut Vec::new())
            .damage,
        0
    );
}

#[test]
fn a_powerful_corrupt_mind_reflects_damage_and_mana_drain() {
    for kind in [DamageType::Psi, DamageType::PsiDrain] {
        let mut game = psychic_target_game(&["weird-mind", "undead"], 80);
        let saving = game.player_derived_stats().saving_throw_skill.value.max(0) as u64;
        game.rng = seeded(|rng| rng.bounded(2) == 0 && rng.bounded(140) >= saving);
        let hp = game.player.hp;
        let mana = game.resources[MANA].current;
        let result = game.prepare_psychic_damage(0, "test", kind, 90, &mut Vec::new());
        assert_eq!(result.damage, 0);
        assert_eq!(game.player.hp, hp - 30);
        if kind == DamageType::PsiDrain {
            assert!(game.resources[MANA].current < mana);
        }
    }
}

#[test]
fn charm_records_no_pet_and_distinguishes_friendly_uniques_from_pets() {
    let mut game = psychic_target_game(&[], 20);
    game.rng = seeded(|rng| {
        rng.bounded(1);
        rng.bounded(5) == 0
    });
    assert!(matches!(
        game.resolve_ability_control(0, 0, "any-monster", 1),
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Resisted,
            ..
        }
    ));
    assert!(game.entities[0].no_pet);
    assert!(matches!(
        game.resolve_ability_control(0, 0, "any-monster", 1000),
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Ineligible,
            ..
        }
    ));
    let mut game = psychic_target_game(&["unique"], 50);
    game.rng = seeded(|rng| rng.bounded(72) + 1 >= 50 && rng.bounded(10) != 0);
    assert!(matches!(
        game.resolve_ability_control(0, 0, "any-monster", 100),
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Friendly,
            ..
        }
    ));
    assert!(game.entities[0].friendly);
    assert_eq!(game.entities[0].controller_id, None);
    game.rng = seeded(|rng| rng.bounded(720) + 1 > 100);
    assert!(matches!(
        game.resolve_ability_control(0, 0, "any-monster", 1000),
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Controlled,
            ..
        }
    ));
    assert!(!game.entities[0].friendly);
    let mut game = psychic_target_game(&["questor"], 1);
    assert!(matches!(
        game.resolve_ability_control(0, 0, "any-monster", 1000),
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Ineligible,
            ..
        }
    ));
}

#[test]
fn spear_pierces_invulnerability_and_player_wraithform_but_other_mental_damage_does_not() {
    let mut game = arena(50);
    target(&mut game, 6);
    let mut shield = crate::game::monster_combat::melee_status(
        crate::effect::STATUS_INVULNERABILITY,
        20,
        "test",
    )
    .status;
    shield.incoming_damage_percent = 0;
    game.entities[0].statuses.push(shield.clone());
    cast(
        &mut game,
        "neural-blast",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert_eq!(game.entities[0].hp, 2000);
    cast(
        &mut game,
        "psycho-spear",
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );
    assert!(game.entities[0].hp < 2000);
    game.player.statuses.push(shield);
    let mut wraith =
        crate::game::monster_combat::melee_status(STATUS_WRAITHFORM, 20, "test").status;
    wraith.incoming_damage_percent = 50;
    game.player.statuses.push(wraith);
    game.rng = seeded(|rng| rng.bounded(13) != 0);
    assert_eq!(game.player_spell_damage_percent(DamageType::Psi, 10), 0);
    assert_eq!(
        game.player_spell_damage_percent(DamageType::PsySpear, 10),
        100
    );
    game.rng = seeded(|rng| rng.bounded(13) == 0);
    assert_eq!(game.player_spell_damage_percent(DamageType::Psi, 10), 50);
    let hp = game.player.hp;
    let source = game.entities[0].id.clone();
    game.resolve_monster_damage_to_player(
        &source,
        "demo.actor.gloom-weaver",
        "rfb-legacy.ability.psy-spear-1d45-100",
        0,
        10,
        10,
        DamageType::PsySpear,
        &mut Vec::new(),
    );
    assert_eq!(game.player.hp, hp - 10);
    let victim = target(&mut game, 8);
    let shield = game.entities[0].statuses[0].clone();
    game.entities[victim].statuses.push(shield);
    let hostile = MonsterHostileTarget::Summon {
        entity_id: game.entities[victim].id.clone(),
        kind_id: game.entities[victim].kind_id.clone(),
        position: game.entities[victim].position,
    };
    game.resolve_monster_damage_to_hostile(
        &source,
        "demo.actor.gloom-weaver",
        "rfb-legacy.ability.psy-spear-1d45-100",
        0,
        10,
        10,
        DamageType::PsySpear,
        &hostile,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    );
    assert_eq!(game.entities[victim].hp, 1990);
}

#[test]
fn repeated_mental_statuses_diminish_confusion_and_stun_but_replace_sleep() {
    let mut game = arena(30);
    target(&mut game, 6);
    for (kind, first, second, expected) in [
        (STATUS_CONFUSION, 20, 12, 26),
        (STATUS_STUN, 40, 12, 44),
        (STATUS_SLEEP, 20, 12, 12),
    ] {
        game.apply_monster_mental_status(0, kind, first, "test");
        game.apply_monster_mental_status(0, kind, second, "test");
        assert_eq!(
            game.entities[0]
                .statuses
                .iter()
                .find(|status| status.kind_id == kind)
                .unwrap()
                .remaining_ticks,
            expected
        );
    }
}

#[test]
fn meditation_stone_has_exclusive_casting_bonuses_and_its_activation_recharges_devices() {
    let mut ordinary = arena(45);
    equip(&mut ordinary, "test.light", "demo.item.mind-stone", "light");
    let light = ordinary.items.last_mut().unwrap();
    light.intrinsic_properties.modifiers.wisdom = 2;
    light.intrinsic_properties.passives.extend([
        EquipmentPassive::EasySpell,
        EquipmentPassive::ReducedManaCost,
    ]);
    let ordinary_failure = projection(&ordinary, "psycho-storm").failure_percent;
    assert_eq!(projection(&ordinary, "psycho-storm").resource_cost, 50);
    let mut game = arena(45);
    equip(&mut game, "test.stone", "demo.item.stone-of-mind", "light");
    assert_eq!(
        projection(&game, "psycho-storm").failure_percent,
        ordinary_failure - 5
    );
    assert_eq!(projection(&game, "psycho-storm").resource_cost, 37);
    assert_eq!(projection(&game, "minor-displacement").resource_cost, 31);
    assert_eq!(projection(&game, "clear-mind").resource_cost, 0);
    give_inventory_item(&mut game, "test.staff", "demo.item.detect-objects-staff");
    game.items
        .last_mut()
        .unwrap()
        .charges
        .as_mut()
        .unwrap()
        .current = 0;
    let maximum = game.items.last().unwrap().charges.unwrap().maximum;
    game.resources.get_mut(MANA).unwrap().current = 0;
    game.apply_player_mental_status(STATUS_BERSERK, 20, "test");
    game.rng = seeded(|rng| rng.bounded(100) < 5);
    let mut events = Vec::new();
    game.use_inventory_item(
        "test.stone",
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.resources[MANA].current, game.resources[MANA].maximum,
        "{events:?}"
    );
    assert!(!game.player_has_status_kind(STATUS_BERSERK));
    let staff = game
        .items
        .iter()
        .find(|item| item.id == "test.staff")
        .unwrap();
    assert_eq!(staff.charges.unwrap().current, maximum / 4);
    assert_eq!(
        staff.device_recovery_progress,
        ((maximum * 250) % 1000) as u16
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.stone")
            .unwrap()
            .charges
            .unwrap()
            .current,
        0
    );
}

#[test]
fn palantir_lists_hidden_uniques_without_revealing_their_positions_and_old_castle_uses_its_pool() {
    let mut game = psychic_target_game(&["unique"], 50);
    let position = game.entities[0].position;
    replace_terrain(&mut game, Position { x: 4, y: 3 }, "demo.terrain.wall");
    game.explored.fill(false);
    give_inventory_item(
        &mut game,
        "test.palantir",
        "demo.item.palantir-of-westernesse",
    );
    game.equip_inventory_item("test.palantir", None).unwrap();
    game.rng = seeded(|rng| rng.bounded(100) < 5);
    let mut events = Vec::new();
    game.use_inventory_item(
        "test.palantir",
        None,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::ItemUniqueMonsterListed { .. })),
        "{events:?}"
    );
    assert!(!game.explored[game.index(position).unwrap()]);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityDetected { .. }))
    );
    for (roll, expected) in [
        (0, "demo.item.eternity"),
        (1, "demo.item.palantir-of-westernesse"),
    ] {
        let mut game = mindcrafter(1);
        game.player.position = position_from_content(
            game.content
                .town_facility("demo.town-facility.outpost-white-horse")
                .unwrap()
                .entrance_position,
        );
        game.task_states.insert(
            "demo.task.old-castle".to_owned(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: 1,
                required: 1,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        game.generated_artifact_ids.extend([
            "demo.item.eternity".to_owned(),
            "demo.item.palantir-of-westernesse".to_owned(),
        ]);
        game.rng = seeded(|rng| rng.bounded(5) == roll);
        let outcome = game
            .claim_task_reward(
                "demo.town-facility.outpost-white-horse",
                "demo.task.old-castle",
            )
            .unwrap();
        assert!(
            matches!(outcome, crate::game::tasks::TaskServiceCompletionOutcome::Rewarded(reward) if reward.item_kind_id == expected)
        );
        assert_eq!(
            game.content
                .item("demo.item.eternity")
                .unwrap()
                .weight_tenths_pound,
            0
        );
    }
}

#[test]
fn a_real_spell_chain_and_monster_no_pet_state_round_trip_and_continue() {
    let mut game = Game::new_with_build(927, BUILD).unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(45), &mut Vec::new());
    super::super::support::choose_human_talent_if_pending(&mut game);
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    game.debug_set_ability_casts_succeed(true);
    for slug in ["precognition", "character-armor", "adrenaline"] {
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: id(slug),
                target: TargetSelection::SelfTarget,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "ability.cast-success")
        );
    }
    let position = game.player.position;
    let monster_position = Position {
        x: position.x + 1,
        y: position.y,
    };
    replace_terrain(&mut game, monster_position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.persisted",
        "demo.actor.gloom-weaver",
        monster_position,
        7,
        100,
        100,
        true,
    ));
    let index = game.entities.len() - 1;
    game.rng = seeded(|rng| {
        rng.bounded(1);
        rng.bounded(5) == 0
    });
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    game.resolve_ability_control(index, 0, "any-monster", 1);
    assert!(game.entities[index].no_pet);
    game.debug_set_ability_casts_succeed(false);
    let mut loaded =
        Game::from_save(game.to_save()).expect("spell state and no-pet flag must load");
    assert_eq!(game.state_hash(), loaded.state_hash());
    let first = dispatch_next(&mut game, GameCommand::Wait);
    let resumed = dispatch_next(&mut loaded, GameCommand::Wait);
    assert_eq!(first, resumed);
    assert_eq!(game.state_hash(), loaded.state_hash());
}

#[test]
fn cancelled_or_unaffordable_mindcraft_does_not_advance_time_or_rng_but_a_failed_cast_does() {
    let mut game = mindcrafter(1);
    game.apply_player_experience(game.experience_required_for_level(45), &mut Vec::new());
    super::super::support::choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    for target in [
        TargetSelection::SelfTarget,
        TargetSelection::Position {
            position: game.player.position,
        },
    ] {
        game.resources.get_mut(MANA).unwrap().current =
            if matches!(target, TargetSelection::SelfTarget) {
                game.resources[MANA].maximum
            } else {
                0
            };
        let before = (
            game.world_tick,
            game.player.energy_need,
            game.rng_draw_counter(),
            game.resources[MANA].current,
        );
        dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: id("minor-displacement"),
                target,
            },
        );
        assert_eq!(
            (
                game.world_tick,
                game.player.energy_need,
                game.rng_draw_counter(),
                game.resources[MANA].current
            ),
            before
        );
    }
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    game.apply_player_mental_status(crate::effect::STATUS_ANTI_MAGIC, 10, "test");
    let before = (
        game.world_tick,
        game.rng_draw_counter(),
        game.resources[MANA].current,
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("precognition"),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(update.events.iter().any(|event| {
        event
            .args
            .get("reason")
            .is_some_and(|reason| reason == "anti-magic")
    }));
    assert_eq!(
        (
            game.world_tick,
            game.rng_draw_counter(),
            game.resources[MANA].current
        ),
        before
    );
    game.player.statuses.clear();
    let failure = projection(&game, "precognition").failure_percent;
    game.rng = seeded(|rng| {
        rng.bounded(100) < u64::from(failure) && rng.bounded(100) + 1 >= u64::from(failure / 2)
    });
    let before = (game.world_tick, game.resources[MANA].current);
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("precognition"),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.cast-failure")
    );
    assert!(game.world_tick > before.0);
    assert_eq!(game.resources[MANA].current, before.1 - 10);
}

#[test]
fn telekinetic_riders_preserve_own_mount_and_sound_resistance_while_psi_stun_ignores_sound() {
    let mut game = psychic_target_game(&[], 1);
    game.riding_actor_id = Some(game.entities[0].id.clone());
    game.entities[0]
        .resistances
        .set(DamageType::Sound, ResistanceLevel::Resistant);
    game.rng = seeded(|rng| rng.bounded(4) == 0);
    let position = game.entities[0].position;
    let result =
        game.prepare_psychic_damage(0, "test", DamageType::Telekinesis, 90, &mut Vec::new());
    game.apply_psychic_damage_riders(0, "test", result, &mut BTreeSet::new());
    assert_eq!(game.entities[0].position, position);
    assert!(
        !game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );
    game.rng = seeded(|rng| {
        rng.bounded(270);
        rng.bounded(4) == 0 && rng.bounded(4) == 1
    });
    let result = game.prepare_psychic_damage(0, "test", DamageType::Psi, 90, &mut Vec::new());
    game.apply_psychic_damage_riders(0, "test", result, &mut BTreeSet::new());
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_STUN)
    );
}
