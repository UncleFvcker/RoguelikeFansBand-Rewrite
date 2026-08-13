use super::support::{
    clear_monsters, dispatch_next, game_with_actor_definition, place_player_on_terrain,
    replace_terrain,
};
use super::*;

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

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
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
        Game::from_save_with_content(invalid, game.content.clone()),
        Err(CoreError::InvalidSave(
            "player riding proficiency state is invalid"
        ))
    ));
}

#[test]
fn riding_proficiency_save_field_is_required() {
    let mut value = serde_json::to_value(Game::new(48).to_save()).expect("save should serialize");
    value["player"]["progress"]
        .as_object_mut()
        .expect("progress should be an object")
        .remove("ridingProficiency");
    assert!(serde_json::from_value::<rfb_protocol::SavePayloadV1>(value).is_err());
}

#[test]
fn mount_moves_with_player_round_trips_and_dismounts() {
    let mut game = game_with_actor_definition(41, "demo.actor.horse", |actor| {
        actor.level = 1;
    });
    clear_monsters(&mut game);
    let start = Position { x: 48, y: 16 };
    let mount_position = Position { x: 49, y: 16 };
    let moved_position = Position { x: 50, y: 16 };
    let dismount_position = Position { x: 50, y: 15 };
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

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
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
    game.resolve_riding(Direction::North, &mut events, &mut BTreeSet::new());
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
    let origin = fall.player.position;
    let hp_before = fall.player.hp;
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    assert!(fall.resolve_riding_fall(0, true, &mut events, &mut changed));
    assert_eq!(fall.riding_actor_id, None);
    assert_ne!(fall.player.position, origin);
    assert_eq!(fall.player.hp, hp_before - 23);
    assert!(matches!(
        events.last(),
        Some(DomainEvent::RidingFell { .. })
    ));

    let mut collision = mounted_game(425, 20);
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
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    death
        .resolve_actor_death_without_rewards(0, None, &mut events, &mut changed, &mut removed)
        .expect("mount death should resolve");
    assert_eq!(death.riding_actor_id, None);
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

    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    assert_eq!(game.riding_actor_id.as_deref(), Some("test.mount"));
    assert!(
        game.entities
            .iter()
            .any(|entity| { entity.id == "test.mount" && entity.position == game.player.position })
    );
}
