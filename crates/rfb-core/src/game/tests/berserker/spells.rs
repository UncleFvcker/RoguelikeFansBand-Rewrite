// SPDX-License-Identifier: MPL-2.0

use super::*;
use crate::game::tests::support::replace_terrain;

fn id(slug: &str) -> String {
    format!("demo.ability.berserker-{slug}")
}

fn arena(level: u16) -> Game {
    let mut game = berserker(level);
    game.player.position = Position { x: 77, y: 33 };
    game.terrain.fill("demo.terrain.floor".to_owned());
    game.debug_set_ability_casts_succeed(true);
    game
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

fn east() -> TargetSelection {
    TargetSelection::Direction {
        direction: Direction::East,
    }
}

fn target(game: &mut Game, position: Position, kind: &str) {
    let mut actor = game.generated_actor(
        format!("test.target.{}", game.entities.len()),
        kind,
        position,
    );
    actor.hp = 5000;
    actor.max_hp = 5000;
    game.entities.push(actor);
}

#[test]
fn class_parameters_failures_and_zero_time_rejections_use_real_abilities() {
    let mut game = arena(50);
    for (slug, level, cost, fail, attribute) in [
        (
            "detect-menace",
            8,
            5,
            40,
            rfb_content::TechniqueAttribute::Strength,
        ),
        (
            "charge",
            15,
            20,
            0,
            rfb_content::TechniqueAttribute::Strength,
        ),
        (
            "smash-trap",
            20,
            15,
            0,
            rfb_content::TechniqueAttribute::Strength,
        ),
        (
            "earthquake",
            25,
            20,
            60,
            rfb_content::TechniqueAttribute::Strength,
        ),
        (
            "massacre",
            30,
            80,
            75,
            rfb_content::TechniqueAttribute::Strength,
        ),
        (
            "recall",
            10,
            10,
            70,
            rfb_content::TechniqueAttribute::Dexterity,
        ),
    ] {
        let activation = game.class_ability_activation(&id(slug)).unwrap();
        assert_eq!(
            (
                activation.minimum_level,
                activation.hit_point_cost,
                activation.base_failure_percent
            ),
            (level, cost, fail)
        );
        assert_eq!(activation.governing_attribute, Some(attribute));
        assert_eq!(activation.resource_id, None);
        assert_eq!(activation.resource_cost, 0);
    }
    // Cancel, insufficient HP, missing target, confusion and under-level attempts consume no action/RNG.
    for (level, hp, confused, target) in [
        (15, 100, false, TargetSelection::SelfTarget),
        (15, 19, false, east()),
        (14, 100, false, east()),
        (15, 100, true, east()),
        (15, 100, false, east()),
    ] {
        let mut game = arena(level);
        game.player.hp = hp;
        if confused {
            game.apply_player_mental_status(STATUS_CONFUSION, 10, "test");
        }
        let rng = game.rng.clone();
        let turn = game.world_tick;
        dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: id("charge"),
                target,
            },
        );
        assert_eq!((game.world_tick, game.player.hp), (turn, hp));
        assert_eq!(game.rng, rng);
    }
    game = arena(8);
    game.debug_set_ability_casts_succeed(false);
    let failure = game.class_ability_failure_percent(
        game.class_ability_activation(&id("detect-menace")).unwrap(),
    );
    assert!(failure > 0);
    let seed = (0..100)
        .find(|seed| RfbRng::seeded(*seed).bounded(100) < u64::from(failure))
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.player.hp = 5;
    let events = cast(&mut game, "detect-menace", TargetSelection::SelfTarget);
    assert!(
        matches!(events.as_slice(), [DomainEvent::AbilityCastFailed { resolution }] if resolution.hp_paid == 5)
    );
    assert_eq!(game.player.hp, 0);
    assert!(!game.player_is_dead());
}

#[test]
fn charge_crosses_a_survivor_but_respects_walls_traps_occupants_and_riding() {
    let mut base = arena(15);
    target(
        &mut base,
        Position { x: 78, y: 33 },
        "demo.actor.small-kobold",
    );
    let origin = base.player.position;
    for obstruction in [
        None,
        Some("wall"),
        Some("trap"),
        Some("occupied"),
        Some("riding"),
    ] {
        let mut game = base.clone();
        let destination = Position { x: 79, y: 33 };
        match obstruction {
            Some("wall") => replace_terrain(&mut game, destination, "demo.terrain.permanent-wall"),
            Some("trap") => replace_terrain(&mut game, destination, "demo.terrain.warren-snare"),
            Some("occupied") => target(&mut game, destination, "demo.actor.small-kobold"),
            Some("riding") => game.riding_actor_id = Some("test.mount".to_owned()),
            _ => {}
        }
        let hp = game.player.hp;
        let rng = game.rng.clone();
        let events = cast(&mut game, "charge", east());
        if obstruction == Some("riding") {
            assert_eq!(game.rng, rng);
            assert_eq!(game.player.hp, hp);
        } else {
            assert_eq!(game.player.hp, hp - 20);
            assert!(events.iter().any(|event| matches!(
                event,
                DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
            )));
            assert!(game.entities[0].hp > 0);
        }
        assert_eq!(
            game.player.position,
            if obstruction.is_none() {
                destination
            } else {
                origin
            }
        );
        assert_eq!(game.player.energy_need, base.player.energy_need);
    }
    let mut blind = arena(15);
    blind.player.hp = 20;
    blind.apply_player_mental_status(STATUS_BLINDNESS, 10, "test");
    let events = cast(&mut blind, "charge", east());
    assert_eq!(blind.player.hp, 0);
    assert_eq!(blind.player.position, origin);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution } if resolution.failure_percent == 0 && resolution.hp_paid == 20)));
}

#[test]
fn smash_trap_triggers_damage_before_removal_and_pays_even_on_blocked_steps() {
    let mut game = arena(20);
    let destination = Position { x: 78, y: 33 };
    replace_terrain(&mut game, destination, "demo.terrain.warren-snare");
    let hp = game.player.hp;
    let events = cast(&mut game, "smash-trap", east());
    let trigger = events
        .iter()
        .position(|event| matches!(event, DomainEvent::TrapTriggered { .. }))
        .unwrap();
    let removed = events
        .iter()
        .position(|event| matches!(event, DomainEvent::TrapDisarmed { .. }))
        .unwrap();
    assert!(trigger < removed);
    assert_eq!(game.player.position, destination);
    assert!(game.player.hp < hp - 15);
    assert!(
        game.content
            .terrain(game.terrain_at(destination))
            .unwrap()
            .trap
            .is_none()
    );
    replace_terrain(
        &mut game,
        Position { x: 79, y: 33 },
        "demo.terrain.permanent-wall",
    );
    let hp = game.player.hp;
    cast(&mut game, "smash-trap", east());
    assert_eq!(game.player.hp, hp - 15);
    assert_eq!(game.player.position, destination);
}

#[test]
fn detection_uses_minds_through_walls_including_weird_minds_and_allies() {
    let mut game = arena(8);
    for (x, kind) in [
        (79, "demo.actor.adobe-golem"),
        (80, "demo.actor.abyss-worm-mass"),
        (81, "demo.actor.small-kobold"),
    ] {
        target(&mut game, Position { x, y: 33 }, kind);
    }
    game.entities[2].controller_id = Some(game.player.id.clone());
    target(
        &mut game,
        Position { x: 97, y: 53 },
        "demo.actor.small-kobold",
    );
    target(
        &mut game,
        Position { x: 99, y: 55 },
        "demo.actor.small-kobold",
    );
    replace_terrain(
        &mut game,
        Position { x: 78, y: 33 },
        "demo.terrain.permanent-wall",
    );
    game.apply_player_mental_status(crate::effect::STATUS_ANTI_MAGIC, 100, "test");
    assert!(game.player_has_anti_magic());
    let hp = game.player.hp;
    let events = cast(&mut game, "detect-menace", TargetSelection::SelfTarget);
    let positions = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityDetected { resolution, .. } if resolution.category == "mind" => {
                Some(&resolution.detected_positions)
            }
            _ => None,
        })
        .unwrap();
    assert!(!positions.contains(&Position { x: 79, y: 33 }));
    assert!(positions.contains(&Position { x: 80, y: 33 }));
    assert!(positions.contains(&Position { x: 81, y: 33 }));
    assert!(positions.contains(&Position { x: 97, y: 53 }));
    assert!(!positions.contains(&Position { x: 99, y: 55 }));
    assert_eq!(game.player.hp, hp - 5);
}

#[test]
fn massacre_matches_sequential_real_melee_in_source_order_including_friends() {
    let mut game = arena(30);
    let directions = [
        Direction::South,
        Direction::North,
        Direction::East,
        Direction::West,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthEast,
        Direction::NorthWest,
    ];
    for direction in directions {
        let position = game.position_in_direction(direction);
        target(&mut game, position, "demo.actor.small-kobold");
    }
    game.entities[0].controller_id = Some(game.player.id.clone());
    let mut expected = game.clone();
    expected.rng.bounded(100); // Class success roll, even at 0% failure.
    let mut expected_events = Vec::new();
    for index in 0..8 {
        expected
            .resolve_player_melee(
                index,
                false,
                &mut expected_events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
    }
    let hp = game.player.hp;
    let energy = game.player.energy_need;
    let events = cast(&mut game, "massacre", TargetSelection::SelfTarget);
    assert_eq!(game.entities, expected.entities);
    assert_eq!(game.rng, expected.rng);
    assert_eq!(game.player.hp, hp - 80);
    assert_eq!(game.player.energy_need, energy);
    let melee = |events: Vec<DomainEvent>| {
        events
            .into_iter()
            .filter(|event| {
                matches!(
                    event,
                    DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(melee(events), melee(expected_events));
}

#[test]
fn earthquake_and_recall_keep_shared_floor_rules_and_hp_costs() {
    let mut game = arena(30);
    let hp = game.player.hp;
    let terrain = game.terrain.clone();
    cast(&mut game, "earthquake", TargetSelection::SelfTarget);
    assert_eq!(game.terrain, terrain); // RFB surface earthquake is a paid no-op.
    assert_eq!(game.player.hp, hp - 20);
    let world = game.content.world(&game.world_id).unwrap();
    let dungeon = world.dungeons.first().unwrap();
    game.current_floor_id = dungeon.root_floor_id.clone();
    game.recall = Some(RecallStateDto {
        dungeon_id: dungeon.id.clone(),
        floor_id: dungeon.root_floor_id.clone(),
        remaining_turns: None,
    });
    let mut expected = game.clone();
    expected.rng.bounded(100);
    let ability = expected
        .content
        .ability("demo.ability.nature-earthquake")
        .unwrap()
        .clone();
    expected
        .resolve_player_ability_effect(
            ability,
            AbilityTargetPlan::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    cast(&mut game, "earthquake", TargetSelection::SelfTarget);
    assert_eq!(game.terrain, expected.terrain);
    assert_eq!(game.rng, expected.rng);
    assert_eq!(game.player.hp, expected.player.hp - 20);
    game.player.hp = game.effective_player_max_hp();
    let hp = game.player.hp;
    cast(&mut game, "recall", TargetSelection::SelfTarget);
    assert!((16..=35).contains(&game.recall.as_ref().unwrap().remaining_turns.unwrap()));
    assert_eq!(game.player.hp, hp - 10);
    cast(&mut game, "recall", TargetSelection::SelfTarget);
    assert_eq!(game.recall.as_ref().unwrap().remaining_turns, None);
    assert_eq!(game.player.hp, hp - 20);
}

#[test]
fn real_directional_actions_save_and_continue_deterministically() {
    let mut game = arena(20);
    // Use real progression for the saved character.
    game.apply_player_experience(game.experience_required_for_level(20), &mut Vec::new());
    crate::game::tests::support::choose_human_talent_if_pending(&mut game);
    target(
        &mut game,
        Position { x: 78, y: 33 },
        "demo.actor.small-kobold",
    );
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("charge"),
            target: east(),
        },
    );
    clear_monsters(&mut game);
    let destination = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, destination, "demo.terrain.warren-snare");
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("smash-trap"),
            target: east(),
        },
    );
    game.debug_set_ability_casts_succeed(false);
    let mut loaded = Game::from_save(game.to_save()).unwrap();
    assert_eq!(loaded.state_hash(), game.state_hash());
    let action = GameCommand::CastAbility {
        ability_id: id("detect-menace"),
        target: TargetSelection::SelfTarget,
    };
    assert_eq!(
        dispatch_next(&mut loaded, action.clone()),
        dispatch_next(&mut game, action)
    );
    assert_eq!(loaded.state_hash(), game.state_hash());
}

#[test]
fn melee_healing_precedes_charge_payment_and_death_stops_massacre_without_another_fee() {
    let mut base = arena(30);
    let position = base.position_in_direction(Direction::East);
    target(&mut base, position, "demo.actor.small-kobold");
    base.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.broad-axe")
        .unwrap()
        .intrinsic_properties
        .passives
        .insert(rfb_content::EquipmentPassive::Vampiric);
    base.player.hp = base.effective_player_max_hp() - 1;
    let (seed, expected) = (0..100)
        .find_map(|seed| {
            let mut expected = base.clone();
            expected.rng = RfbRng::seeded(seed);
            expected.rng.bounded(100);
            expected
                .resolve_player_melee(
                    0,
                    true,
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            (expected.player.hp > base.player.hp).then_some((seed, expected))
        })
        .expect("vampiric healing branch");
    base.rng = RfbRng::seeded(seed);
    cast(&mut base, "charge", east());
    assert_eq!(base.player.hp, expected.player.hp - 20);

    let content = crate::game::tests::support::game_with_actor_definition(
        0,
        "demo.actor.small-kobold",
        |actor| {
            actor.contact_auras = vec![rfb_content::ActorContactAuraDefinition {
                damage_type: rfb_content::ActorDamageType::Fire,
                damage_dice: 100,
                damage_sides: 5,
                chance_percent: None,
                ravages_time: false,
            }];
        },
    )
    .content;
    let mut base = arena(30);
    base.content = content;
    for direction in [Direction::South, Direction::North] {
        let position = base.position_in_direction(direction);
        target(&mut base, position, "demo.actor.small-kobold");
    }
    base.player.hp = 80;
    let (game, events) = (0..100)
        .find_map(|seed| {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let events = cast(&mut game, "massacre", TargetSelection::SelfTarget);
            game.player_is_dead().then_some((game, events))
        })
        .expect("lethal contact aura branch");
    assert_eq!(game.entities[1].hp, 5000);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution } if resolution.hp_paid == 0)));
}

#[test]
fn charge_skips_pickup_while_smash_uses_walk_pickup_and_destroys_a_levitated_trap() {
    let mut game = arena(20);
    crate::game::tests::support::choose_human_talent_if_pending(&mut game);
    game.interface_locale = rfb_protocol::LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            rfb_protocol::AutoGetModeDto::Off,
            rfb_protocol::LocaleDto::EnUs,
            "potions".to_owned()
        )
        .is_empty()
    );
    target(
        &mut game,
        Position { x: 78, y: 33 },
        "demo.actor.small-kobold",
    );
    give_inventory_item(
        &mut game,
        "test.charge-loot",
        "demo.item.enlightenment-potion",
    );
    game.items.last_mut().unwrap().location = ItemLocation::Ground(Position { x: 79, y: 33 });
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("charge"),
            target: east(),
        },
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.charge-loot")
            .unwrap()
            .location,
        ItemLocation::Ground(game.player.position)
    );
    clear_monsters(&mut game);
    let destination = game.position_in_direction(Direction::East);
    replace_terrain(&mut game, destination, "demo.terrain.warren-snare");
    give_inventory_item(
        &mut game,
        "test.smash-loot",
        "demo.item.enlightenment-potion",
    );
    game.items.last_mut().unwrap().location = ItemLocation::Ground(destination);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.broad-axe")
        .unwrap()
        .intrinsic_properties
        .passives
        .insert(rfb_content::EquipmentPassive::Levitation);
    assert!(game.player_levitates());
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("smash-trap"),
            target: east(),
        },
    );
    assert_eq!(game.player.position, destination);
    assert!(
        game.content
            .terrain(game.terrain_at(destination))
            .unwrap()
            .trap
            .is_none()
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.smash-loot")
            .unwrap()
            .location,
        ItemLocation::Inventory
    );
    game.player.position = Position { x: 131, y: 33 };
    replace_terrain(
        &mut game,
        Position { x: 132, y: 33 },
        "demo.terrain.surface-path",
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: id("smash-trap"),
            target: east(),
        },
    );
    assert_eq!(update.map_translation, Some(Position { x: -66, y: 0 }));
    assert_eq!(game.player.position, Position { x: 66, y: 33 });
}
