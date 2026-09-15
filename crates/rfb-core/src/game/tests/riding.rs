use super::support::{
    clear_monsters, dispatch_next, game_with_actor_definition, give_inventory_item,
    place_player_on_terrain, replace_terrain,
};
use super::*;
use crate::action::RestMode;

fn mounted_expert_game(seed: u64) -> Game {
    Game::new_with_build(seed, "demo.build.cavalry").expect("Cavalry build should create")
}

fn mounted_game(seed: u64, mount_level: u32) -> Game {
    let mut game = game_with_actor_definition(seed, "demo.actor.horse", |actor| {
        actor.level = mount_level;
    });
    clear_monsters(&mut game);
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());
    game
}

#[test]
fn two_handed_riding_is_free_changes_combat_and_keeps_equipment_constraints() {
    let mut game = mounted_game(530, 1);
    super::support::choose_human_talent_if_pending(&mut game);
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "weapon", "demo.item.broad-sword");
    assert!(
        game.equip_inventory_item("weapon", Some("right-hand"))
            .is_some()
    );
    let weapon = |game: &Game| {
        game.items
            .iter()
            .position(|item| item.id == "weapon")
            .unwrap()
    };
    assert!(!game.weapon_uses_two_hands(&game.items[weapon(&game)]));
    assert!(!game.riding_without_reins());
    let one = game.player_melee_profile(&game.player_derived_stats());
    let before = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::RidingTwoHands,
            enabled: true,
        },
    );
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        before
    );
    assert!(game.weapon_uses_two_hands(&game.items[weapon(&game)]));
    assert!(game.snapshot().player.riding_without_reins);
    let both = game.player_melee_profile(&game.player_derived_stats());
    assert!(both.to_hit > one.to_hit && both.to_damage > one.to_damage);
    assert!(both.melee_skill.value > one.melee_skill.value);
    let restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored
            .player_melee_profile(&restored.player_derived_stats())
            .to_dto(),
        both.to_dto()
    );
    let mut invalid = game.to_save();
    invalid.player.riding_actor_id = None;
    assert!(
        Game::from_save_with_content(
            invalid,
            game.content.clone(),
            Game::default_behavior_preferences()
        )
        .is_err()
    );

    give_inventory_item(&mut game, "shield", "demo.item.small-leather-shield");
    assert!(
        game.equip_inventory_item("shield", Some("left-hand"))
            .is_some()
    );
    assert!(!game.weapon_uses_two_hands(&game.items[weapon(&game)]));
    dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::RidingTwoHands,
            enabled: false,
        },
    );
    assert!(
        game.riding_without_reins(),
        "a shield still occupies the control hand"
    );
    game.items
        .iter_mut()
        .find(|item| item.id == "shield")
        .unwrap()
        .location = ItemLocation::Inventory;
    assert!(!game.riding_without_reins());
    game.clear_riding_state();
    assert!(!game.riding_without_reins());
    assert!(game.weapon_uses_two_hands(&game.items[weapon(&game)]));
    let update = dispatch_next(
        &mut game,
        GameCommand::SetPetOption {
            option: rfb_protocol::PetOptionDto::RidingTwoHands,
            enabled: true,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "pet.command-unavailable")
    );
    assert!(!game.summon_command.riding_two_hands);
}

#[test]
fn multiple_arm_pairs_cannot_share_an_unrelated_empty_hand() {
    let mut game = mounted_game(531, 1);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.body_slots = (0..4)
        .map(|hand| BodySlot {
            id: format!("hand-{hand}"),
            slot_type: "weapon".into(),
        })
        .collect();
    for (id, slot) in [("a", "hand-0"), ("b", "hand-1")] {
        give_inventory_item(&mut game, id, "demo.item.broad-sword");
        assert!(game.equip_inventory_item(id, Some(slot)).is_some());
    }
    assert!(!game.riding_without_reins());
    game.summon_command.riding_two_hands = true;
    assert!(
        game.items
            .iter()
            .all(|weapon| !game.weapon_uses_two_hands(weapon))
    );
    game.items[1].location = ItemLocation::Equipped {
        slot_id: "hand-2".into(),
    };
    assert!(
        game.items
            .iter()
            .all(|weapon| game.weapon_uses_two_hands(weapon))
    );
    game.summon_command.riding_two_hands = false;
    assert!(
        game.items
            .iter()
            .all(|weapon| !game.weapon_uses_two_hands(weapon))
    );
    assert!(!game.riding_without_reins());
    game.body_slots
        .retain(|slot| matches!(slot.id.as_str(), "hand-0" | "hand-2"));
    assert!(
        game.riding_without_reins(),
        "changing the body recomputes control immediately"
    );
}

#[test]
fn releasing_reins_blocks_directed_steps_and_mount_ai_moves_rider_once() {
    let mut game = mounted_game(532, 1);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.player.position = Position { x: 70, y: 30 };
    game.entities[0].position = game.player.position;
    for y in 28..=32 {
        for x in 68..=74 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(true);
    game.push_generated_actor(
        "enemy".into(),
        "demo.actor.adobe-golem",
        Position { x: 72, y: 30 },
    );
    game.summon_command.target_actor_id = Some("enemy".into());
    game.summon_command.riding_two_hands = true;
    let origin = game.player.position;
    let mut events = Vec::new();
    let step = game
        .resolve_local_player_step(
            Direction::South,
            false,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert!(!step.moved);
    assert_eq!(game.player.position, origin);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::RidingControlUnavailable))
    );
    let mut controlled = game.clone();
    controlled.summon_command.riding_two_hands = false;
    for variant in [&mut game, &mut controlled] {
        variant.entities[0].energy_need = 0;
        variant
            .continue_monster_energy_pulse(
                vec!["test.mount".into()],
                BTreeSet::new(),
                false,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        assert_eq!(variant.entities[0].position, variant.player.position);
        assert!(variant.entities[0].energy_need > 0);
    }
    assert_ne!(game.player.position, origin);
    assert_eq!(controlled.player.position, origin);
    assert!(
        Game::from_save_with_content(
            game.to_save(),
            game.content.clone(),
            game.behavior_preferences()
        )
        .is_ok()
    );
}

#[test]
fn no_reins_uses_source_fall_range_and_second_roll() {
    let mut base = mounted_game(533, 1);
    base.items.clear();
    base.item_property_knowledge.clear();
    base.progress.riding_proficiency = 6_000;
    // Hold succeeds for this low-level mount. Select a seed that passes the
    // residual fall roll in both modes, so only those two RNG draws occur.
    let seed = (0..100)
        .find(|seed| {
            [false, true].into_iter().all(|free| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(if free { 42 } else { 2 });
                rng.bounded(u64::from(base.progress.level) * if free { 2 } else { 3 } + 30) != 0
            })
        })
        .unwrap();
    for free in [false, true] {
        let mut game = base.clone();
        game.summon_command.riding_two_hands = free;
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        expected.bounded(if free { 42 } else { 2 });
        expected.bounded(u64::from(game.progress.level) * if free { 2 } else { 3 } + 30);
        assert!(!game.resolve_riding_fall(0, false, &mut Vec::new(), &mut BTreeSet::new()));
        assert_eq!(game.rng, expected);
    }
}

#[test]
fn mount_movement_interrupts_rest_and_counted_waits() {
    let mut game = mounted_game(534, 1);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.player.position = Position { x: 70, y: 30 };
    game.entities[0].position = game.player.position;
    game.entities[0].energy_need = 0;
    for y in 28..=32 {
        for x in 68..=72 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    game.glow.fill(true);
    game.summon_command.mode = SummonCommandModeDto::GiveSpace;
    game.summon_command.riding_two_hands = true;
    let mut events = Vec::new();
    let rest = game
        .resolve_player_rest(
            20,
            RestMode::Turns,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(rest.stop_reason, RestStopReasonDto::Displaced);
    assert_eq!(rest.completed_turns, 1);
    assert!(
        !crate::game::command_repeat::CommandRepeatKind::Wait.can_repeat(
            &game,
            &project_events(events),
            false
        )
    );
}

#[test]
fn master_riding_held_success_skips_the_residual_roll_only_with_reins() {
    let mut base = mounted_expert_game(535);
    clear_monsters(&mut base);
    base.items.clear();
    base.item_property_knowledge.clear();
    base.push_generated_actor("mount".into(), "demo.actor.horse", base.player.position);
    base.entities[0].controller_id = Some(base.player.id.clone());
    base.riding_actor_id = Some("mount".into());
    base.progress.riding_proficiency = 8_000;
    let level = base.riding_mount_level().unwrap();
    assert_eq!(base.player_riding_proficiency().maximum, 8_000);
    for free in [false, true] {
        let mut game = base.clone();
        game.summon_command.riding_two_hands = free;
        let seed = (0..100)
            .find(|seed| {
                let mut rng = RfbRng::seeded(*seed);
                rng.bounded(u64::from(level + if free { 20 } else { 0 }) * 2);
                !free || rng.bounded(u64::from(game.progress.level) * 2 + 30) != 0
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        assert!(
            expected.bounded(u64::from(level + if free { 20 } else { 0 }) * 2) < 8_000 / 33 + 25
        );
        if free {
            expected.bounded(u64::from(game.progress.level) * 2 + 30);
        }
        assert!(!game.resolve_riding_fall(0, false, &mut Vec::new(), &mut BTreeSet::new()));
        assert_eq!(game.rng, expected);
    }
}

#[test]
fn erratic_mount_direction_uses_the_source_sequential_rolls_only_without_reins() {
    let mut base = game_with_actor_definition(536, "demo.actor.horse", |actor| {
        actor.allocation.as_mut().unwrap().random_movement_percent = 75;
    });
    clear_monsters(&mut base);
    base.items.clear();
    base.item_property_knowledge.clear();
    base.push_generated_actor("mount".into(), "demo.actor.horse", base.player.position);
    base.entities[0].controller_id = Some(base.player.id.clone());
    base.riding_actor_id = Some("mount".into());
    for seed in 0..16 {
        let mut game = base.clone();
        game.rng = RfbRng::seeded(seed);
        let mut expected = game.rng.clone();
        assert_eq!(
            game.riding_direction(Direction::East, &mut Vec::new()),
            Direction::East
        );
        assert_eq!(game.rng, expected);
        game.summon_command.riding_two_hands = true;
        let direction = if expected.bounded(100) < 50 || expected.bounded(100) < 25 {
            TERRAIN_INTERACTION_DIRECTIONS[expected.bounded(8) as usize]
        } else {
            Direction::East
        };
        assert_eq!(
            game.riding_direction(Direction::East, &mut Vec::new()),
            direction
        );
        assert_eq!(game.rng, expected);
    }
}

#[test]
fn riding_proficiency_uses_original_melee_archery_and_fall_training_rules() {
    let mut melee = mounted_game(44, 1);
    melee.progress.riding_proficiency = 1_999;
    assert!(matches!(
        melee.train_riding_from_melee(80),
        Some(DomainEvent::RidingProficiencyImproved { current: 2_000 })
    ));

    let mut archery = mounted_game(45, 1);
    archery.progress.riding_proficiency = 3_999;
    let mut expected_rng = archery.rng.clone();
    let expected_gain = expected_rng.bounded(2) == 0;
    let event = archery.train_riding_from_archery();
    assert_eq!(archery.rng, expected_rng);
    assert_eq!(
        archery.progress.riding_proficiency,
        3_999 + u16::from(expected_gain)
    );
    assert_eq!(event.is_some(), expected_gain);

    archery.progress.riding_proficiency = 6_000;
    let untouched_rng = archery.rng.clone();
    assert_eq!(archery.train_riding_from_archery(), None);
    assert_eq!(archery.rng, untouched_rng);

    let mut fall = mounted_game(46, 30);
    fall.progress.riding_proficiency = 1_999;
    assert!(matches!(
        fall.train_riding_from_fall_check(1_999),
        Some(DomainEvent::RidingProficiencyImproved { current: 2_000 })
    ));
}

#[test]
fn riding_proficiency_is_authoritative_save_and_snapshot_state() {
    let mut game = mounted_game(47, 1);
    game.progress.riding_proficiency = 2_345;
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.progress.riding_proficiency.current, 2_345);
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 6_000);
    assert_eq!(
        snapshot.player.progress.riding_proficiency.rank,
        rfb_protocol::ProficiencyRankDto::Beginner
    );

    let restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .expect("riding proficiency should round-trip");
    assert_eq!(restored.progress.riding_proficiency, 2_345);
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut invalid = game.to_save();
    invalid
        .player
        .progress
        .as_mut()
        .expect("formal game progress")
        .riding_proficiency = 6_001;
    assert!(matches!(
        Game::from_save_with_content(
            invalid,
            game.content.clone(),
            Game::default_behavior_preferences()
        ),
        Err(CoreError::InvalidSave(
            "player riding proficiency state is invalid"
        ))
    ));
}

#[test]
fn mount_moves_with_player_round_trips_and_dismounts() {
    let mut game = game_with_actor_definition(41, "demo.actor.horse", |actor| {
        actor.level = 1;
    });
    clear_monsters(&mut game);
    let start = Position { x: 99, y: 33 };
    let mount_position = Position { x: 100, y: 33 };
    let moved_position = Position { x: 101, y: 33 };
    let dismount_position = Position { x: 101, y: 32 };
    for position in [start, mount_position, moved_position, dismount_position] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    game.player.position = start;
    game.push_generated_actor("test.mount".to_owned(), "demo.actor.horse", mount_position);
    game.entities[0].controller_id = Some(game.player.id.clone());

    let mut events = Vec::new();
    game.resolve_riding(Direction::East, &mut events, &mut BTreeSet::new());

    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    assert_eq!(game.player.position, mount_position);
    assert_eq!(game.entities[0].position, mount_position);
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::RidingMounted { .. }]
    ));

    let restored = Game::from_save_with_content(
        game.to_save(),
        game.content.clone(),
        game.behavior_preferences(),
    )
    .expect("mounted state should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    game = restored;

    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, moved_position);
    assert_eq!(game.entities[0].position, moved_position);

    let mut events = Vec::new();
    game.summon_command.riding_two_hands = true;
    game.resolve_riding(Direction::North, &mut events, &mut BTreeSet::new());
    assert!(!game.summon_command.riding_two_hands);
    assert_eq!(game.riding_actor_id, None);
    assert_eq!(game.player.position, dismount_position);
    assert_eq!(game.entities[0].position, moved_position);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::RidingDismounted { .. })
    ));
}

#[test]
fn sheep_preserves_the_authoritative_refusal() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let target = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor("test.sheep".to_owned(), "demo.actor.sheep", target);
    game.entities[0].controller_id = Some(game.player.id.clone());
    let mut events = Vec::new();

    game.resolve_riding(Direction::East, &mut events, &mut BTreeSet::new());

    assert_eq!(game.riding_actor_id, None);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::SheepRidingRefused { response: 0..=2 }]
    ));
}

#[test]
fn ordinary_riding_rejects_wild_monsters_without_taming_or_rng() {
    let mut game = Game::new(420);
    clear_monsters(&mut game);
    let target = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    replace_terrain(&mut game, target, "demo.terrain.floor");
    game.push_generated_actor("test.wild-horse".to_owned(), "demo.actor.horse", target);
    let rng_before = game.rng.clone();
    let mut events = Vec::new();

    game.resolve_riding(Direction::East, &mut events, &mut BTreeSet::new());

    assert_eq!(game.riding_actor_id, None);
    assert_eq!(game.entities[0].controller_id, None);
    assert_eq!(game.rng, rng_before);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::RidingNotPet { .. }]
    ));
}

#[test]
fn mounted_speed_uses_original_riding_control_formula() {
    let mut novice = game_with_actor_definition(421, "demo.actor.horse", |actor| {
        actor.level = 20;
        actor.speed = 130;
    });
    clear_monsters(&mut novice);
    novice.push_generated_actor(
        "test.fast-mount".to_owned(),
        "demo.actor.horse",
        novice.player.position,
    );
    novice.entities[0].controller_id = Some(novice.player.id.clone());
    novice.riding_actor_id = Some("test.fast-mount".to_owned());
    assert_eq!(novice.player_derived_stats().speed.value, 110);

    novice.progress.level = 50;
    novice.progress.riding_proficiency = 6_000;
    assert_eq!(novice.player_derived_stats().speed.value, 128);
    assert_eq!(riding_proficiency::mounted_speed(130, 8_000, 50), 135);
}

#[test]
fn mounted_weapon_and_projectile_rules_match_original_branches() {
    let mut game = mounted_game(422, 5);
    let weapon_index = game
        .items
        .iter()
        .position(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "right-hand")
        })
        .expect("Warrior should start with a weapon");

    game.items[weapon_index].kind_id = "demo.item.short-sword".to_owned();
    let ordinary = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(ordinary.to_hit, -35);
    assert_eq!(
        ordinary.melee_skill.value,
        game.player_derived_stats().melee_skill.value - 35
    );

    game.items[weapon_index].kind_id = "demo.item.broad-sword".to_owned();
    let compatible = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(compatible.to_hit, 0);
    assert_eq!(
        compatible.melee_skill.value,
        game.player_derived_stats().melee_skill.value
    );

    game.items[weapon_index].kind_id = "demo.item.lance".to_owned();
    let lance = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(lance.to_hit, 15);
    assert_eq!(
        lance.melee_skill.value,
        game.player_derived_stats().melee_skill.value + 15
    );
    assert_eq!(lance.damage_dice, 4);

    game.items[weapon_index].kind_id = "demo.item.heavy-lance".to_owned();
    let heavy_lance = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(heavy_lance.to_hit, 15);
    assert_eq!(
        heavy_lance.melee_skill.value,
        game.player_derived_stats().melee_skill.value + 15
    );
    assert_eq!(heavy_lance.damage_dice, 6);

    assert_eq!(
        riding_proficiency::mounted_projectile_to_hit_adjustment(
            true,
            AmmunitionTypeDefinition::Arrow,
            50,
            0,
        ),
        0
    );
    assert_eq!(
        riding_proficiency::mounted_projectile_to_hit_adjustment(
            true,
            AmmunitionTypeDefinition::Shot,
            50,
            0,
        ),
        -5
    );
    assert_eq!(
        riding_proficiency::mounted_projectile_to_hit_adjustment(
            true,
            AmmunitionTypeDefinition::Bolt,
            50,
            0,
        ),
        -10
    );

    let mut expert = mounted_expert_game(423);
    expert.progress.level = 50;
    clear_monsters(&mut expert);
    expert.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        expert.player.position,
    );
    expert.entities[0].controller_id = Some(expert.player.id.clone());
    expert.riding_actor_id = Some("test.mount".to_owned());
    let launcher = expert
        .items
        .iter_mut()
        .find(|item| {
            matches!(&item.location, ItemLocation::Equipped { slot_id } if slot_id == "shooting")
        })
        .expect("Cavalry should start with a launcher");
    launcher.kind_id = "demo.item.sling".to_owned();
    let projectile = expert
        .player_projectile_profile()
        .expect("sling should resolve a projectile profile");
    assert_eq!(projectile.to_hit, -5);
    assert_eq!(projectile.energy_cost, 71);
}

#[test]
fn forced_fall_moves_to_an_adjacent_cell_and_collision_stays_mounted() {
    let mut fall = mounted_game(424, 20);
    fall.summon_command.riding_two_hands = true;
    let origin = fall.player.position;
    let hp_before = fall.player.hp;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    assert!(fall.resolve_riding_fall(0, true, &mut events, &mut changed));
    assert_eq!(fall.riding_actor_id, None);
    assert!(!fall.summon_command.riding_two_hands);
    assert_ne!(fall.player.position, origin);
    assert_eq!(fall.player.hp, hp_before - 23);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::RidingFell { .. })
    ));

    let mut collision = mounted_game(425, 20);
    collision.summon_command.riding_two_hands = true;
    let origin = collision.player.position;
    for direction in TERRAIN_INTERACTION_DIRECTIONS {
        let (dx, dy) = direction.delta();
        replace_terrain(
            &mut collision,
            Position {
                x: origin.x + dx,
                y: origin.y + dy,
            },
            "demo.terrain.permanent-wall",
        );
    }
    let hp_before = collision.player.hp;
    let mut events = Vec::new();
    assert!(!collision.resolve_riding_fall(0, true, &mut events, &mut BTreeSet::new(),));
    assert_eq!(collision.riding_actor_id.as_deref(), Some("test.mount"));
    assert!(collision.summon_command.riding_two_hands);
    assert_eq!(collision.player.position, origin);
    assert_eq!(collision.player.hp, hp_before - 23);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::RidingCollided { .. })
    ));
}

#[test]
fn damage_fall_trains_riding_and_mount_death_uses_existing_cleanup() {
    let mut damaged = mounted_game(428, 20);
    let mut events = Vec::new();
    assert!(damaged.resolve_riding_fall(200, false, &mut events, &mut BTreeSet::new(),));
    assert_eq!(damaged.riding_actor_id, None);
    assert_eq!(damaged.progress.riding_proficiency, 6);

    let mut death = mounted_game(427, 5);
    death.summon_command.riding_two_hands = true;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    death
        .resolve_actor_death_without_rewards(0, None, &mut events, &mut changed, &mut removed)
        .expect("mount death should resolve");
    assert_eq!(death.riding_actor_id, None);
    assert!(!death.summon_command.riding_two_hands);
    assert_eq!(removed, ["test.mount"]);
    assert!(
        events
            .iter()
            .all(|event| !matches!(event, DomainEvent::RidingFell { .. }))
    );
}

#[test]
fn current_mount_follows_a_floor_transition() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    let position = game.player.position;
    game.push_generated_actor("test.mount".to_owned(), "demo.actor.horse", position);
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some("test.mount".to_owned());

    let follower_position = Position {
        x: position.x,
        y: position.y - 1,
    };
    replace_terrain(&mut game, follower_position, "demo.terrain.floor");
    game.push_generated_actor(
        "test.follower".into(),
        "demo.actor.horse",
        follower_position,
    );
    game.entities.last_mut().unwrap().controller_id = Some(game.player.id.clone());
    for (id, owner) in [
        ("test.mount-cargo", "test.mount"),
        ("test.follower-cargo", "test.follower"),
    ] {
        give_inventory_item(&mut game, id, "demo.item.short-sword");
        game.items
            .iter_mut()
            .find(|item| item.id == id)
            .unwrap()
            .location = ItemLocation::CarriedBy {
            actor_id: owner.into(),
        };
    }

    let target_position = Position {
        x: position.x + 1,
        y: position.y,
    };
    replace_terrain(&mut game, target_position, "demo.terrain.floor");
    game.push_generated_actor(
        "test.pet-target".to_owned(),
        "demo.actor.adobe-golem",
        target_position,
    );
    game.summon_command.target_actor_id = Some("test.pet-target".to_owned());
    game.summon_command.riding_two_hands = true;
    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    assert!(game.summon_command.riding_two_hands);
    for (id, owner) in [
        ("test.mount-cargo", "test.mount"),
        ("test.follower-cargo", "test.follower"),
    ] {
        assert!(game.entities.iter().any(|actor| actor.id == owner));
        assert_eq!(
            game.items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .location,
            ItemLocation::CarriedBy {
                actor_id: owner.into()
            }
        );
        assert!(
            game.stored_floors
                .values()
                .all(|floor| floor.items.iter().all(|item| item.id != id))
        );
    }
    assert!(game.summon_command.target_actor_id.is_none());
    assert!(
        Game::from_save(game.to_save(), game.behavior_preferences())
            .unwrap()
            .summon_command
            .target_actor_id
            .is_none()
    );
    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    assert!(
        game.entities
            .iter()
            .any(|entity| { entity.id == "test.mount" && entity.position == game.player.position })
    );
}
