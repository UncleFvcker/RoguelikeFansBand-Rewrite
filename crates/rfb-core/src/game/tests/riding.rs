use super::support::{
    clear_monsters, dispatch_next, game_with_actor_definition, place_player_on_terrain,
    replace_terrain,
};
use super::*;

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
    let mut events = Vec::new();

    game.resolve_riding(Direction::East, &mut events, &mut BTreeSet::new());

    assert_eq!(game.riding_actor_id, None);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::SheepRidingRefused { response: 0..=2 }]
    ));
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
