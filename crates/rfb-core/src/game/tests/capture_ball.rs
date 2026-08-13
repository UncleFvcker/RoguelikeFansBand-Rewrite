// SPDX-License-Identifier: MPL-2.0

use super::support::{clear_monsters, give_inventory_item, replace_terrain};
use super::*;

fn equipped_capture_ball(game: &mut Game) -> usize {
    give_inventory_item(game, "test.capture-ball", "demo.item.capture-ball");
    let index = game
        .items
        .iter()
        .position(|item| item.id == "test.capture-ball")
        .expect("capture ball should exist");
    game.items[index].location = ItemLocation::Equipped {
        slot_id: "left-hand".to_owned(),
    };
    game.item_property_knowledge.insert(
        "test.capture-ball".to_owned(),
        ItemPropertyKnowledgeState {
            discovered: true,
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::new(),
        },
    );
    index
}

fn adjacent_open_position(game: &Game, kind_id: &str) -> (Position, Direction) {
    let position = game
        .open_positions_around_for_actor_kind(game.player.position, 1, kind_id)
        .into_iter()
        .next()
        .expect("an adjacent release position should exist");
    let delta = (
        position.x - game.player.position.x,
        position.y - game.player.position.y,
    );
    let direction = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ]
    .into_iter()
    .find(|direction| direction.delta() == delta)
    .expect("adjacent delta should map to a direction");
    (position, direction)
}

#[test]
fn capture_policy_and_health_gates_preserve_rng_until_a_real_attempt() {
    let mut game =
        Game::new_with_build(0x4341_5054, "demo.build.cavalry").expect("Cavalry should create");
    clear_monsters(&mut game);
    let ball_index = equipped_capture_ball(&mut game);
    let (position, _) = adjacent_open_position(&game, "demo.actor.smeagol");
    game.push_generated_actor("test.unique".to_owned(), "demo.actor.smeagol", position);
    game.entities[0].visible_invisible = true;
    let target = TargetSelection::Entity {
        entity_id: "test.unique".to_owned(),
    };

    let rng_before = game.rng.clone();
    let mut events = Vec::new();
    game.use_capture_ball(
        ball_index,
        Some(&target),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(game.rng, rng_before);
    assert!(game.items[ball_index].captured_actor.is_none());

    game.entities[0].controller_id = Some(game.player.id.clone());
    game.entities[0].hp = 1;
    let draws_before = game.rng.draw_counter;
    events.clear();
    game.use_capture_ball(
        ball_index,
        Some(&target),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(
        game.rng.draw_counter > draws_before,
        "capture events: {events:?}"
    );
    assert!(game.entities.is_empty());
    assert_eq!(
        game.items[ball_index]
            .captured_actor
            .as_ref()
            .map(|actor| actor.kind_id.as_str()),
        Some("demo.actor.smeagol")
    );
    assert!(!game.unique_actor_kind_is_available("demo.actor.smeagol"));
}

#[test]
fn captured_mount_falls_resets_bond_and_releases_as_a_new_pet() {
    let mut game = Game::new_with_build(0x004d_4f55_4e54, "demo.build.cavalry")
        .expect("Cavalry should create");
    clear_monsters(&mut game);
    let ball_index = equipped_capture_ball(&mut game);
    let (_, direction) = adjacent_open_position(&game, "demo.actor.horse");
    game.push_generated_actor(
        "test.mount".to_owned(),
        "demo.actor.horse",
        game.player.position,
    );
    game.entities[0].controller_id = Some(game.player.id.clone());
    game.entities[0].experience = 42;
    game.entities[0].hp = 1;
    game.riding_actor_id = Some("test.mount".to_owned());
    game.riding_bond = Some(RidingBond {
        actor_id: "test.mount".to_owned(),
        actor_kind_id: "demo.actor.horse".to_owned(),
        value: 7_500,
    });
    let hp_before = game.player.hp;
    let mut removed = Vec::new();
    game.use_capture_ball(
        ball_index,
        Some(&TargetSelection::Entity {
            entity_id: "test.mount".to_owned(),
        }),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut removed,
    );
    assert_eq!(removed, ["test.mount"]);
    assert!(game.player.hp < hp_before);
    assert!(game.riding_actor_id.is_none());
    assert!(game.riding_bond.is_none());

    let stored = game.items[ball_index]
        .captured_actor
        .as_ref()
        .expect("mount should be stored");
    assert_eq!(stored.experience, 42);
    let old_entity_id = "test.mount";
    game.use_capture_ball(
        ball_index,
        Some(&TargetSelection::Direction { direction }),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(game.entities.len(), 1);
    assert_ne!(game.entities[0].id, old_entity_id);
    assert_eq!(game.entities[0].experience, 42);
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(game.items[ball_index].captured_actor.is_none());
}

#[test]
fn blocked_release_keeps_the_ball_and_drop_uses_the_exact_hostility_roll() {
    let mut game =
        Game::new_with_build(0x4452_4f50, "demo.build.cavalry").expect("Cavalry should create");
    clear_monsters(&mut game);
    let ball_index = equipped_capture_ball(&mut game);
    game.items[ball_index].captured_actor = Some(CapturedActor {
        kind_id: "demo.actor.horse".to_owned(),
        speed: 117,
        hp: 3,
        max_hp: 8,
        experience: 19,
    });
    let (blocked, direction) = adjacent_open_position(&game, "demo.actor.horse");
    replace_terrain(&mut game, blocked, "demo.terrain.granite-wall");
    game.use_capture_ball(
        ball_index,
        Some(&TargetSelection::Direction { direction }),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(game.items[ball_index].captured_actor.is_some());

    game.items[ball_index].location = ItemLocation::Inventory;
    let expected_hostile = {
        let mut rng = game.rng.clone();
        rng.bounded(4) == 0
    };
    let mut events = Vec::new();
    game.force_open_capture_ball(
        "test.capture-ball",
        game.player.position,
        true,
        &mut events,
        &mut BTreeSet::new(),
    );
    assert!(game.items[ball_index].captured_actor.is_none());
    assert_eq!(game.entities[0].controller_id.is_none(), expected_hostile);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::CaptureBallReleased { hostile, .. } if *hostile == expected_hostile
    )));
}

#[test]
fn drop_and_destruction_release_the_actor_before_finishing_the_item_lifecycle() {
    let mut base =
        Game::new_with_build(0x4c49_4645, "demo.build.cavalry").expect("Cavalry should create");
    clear_monsters(&mut base);
    let ball_index = equipped_capture_ball(&mut base);
    base.items[ball_index].location = ItemLocation::Inventory;
    base.items[ball_index].captured_actor = Some(CapturedActor {
        kind_id: "demo.actor.horse".to_owned(),
        speed: 115,
        hp: 4,
        max_hp: 9,
        experience: 23,
    });

    let mut dropped = base.clone();
    let expected_hostile = {
        let mut rng = dropped.rng.clone();
        rng.bounded(4) == 0
    };
    super::support::dispatch_next(
        &mut dropped,
        GameCommand::Drop {
            item_ids: vec!["test.capture-ball".to_owned()],
        },
    );
    assert!(dropped.items.iter().any(|item| {
        item.id == "test.capture-ball"
            && item.location == ItemLocation::Ground(dropped.player.position)
            && item.captured_actor.is_none()
    }));
    assert_eq!(
        dropped.entities[0].controller_id.is_none(),
        expected_hostile
    );

    super::support::dispatch_next(
        &mut base,
        GameCommand::DestroyItem {
            item_id: "test.capture-ball".to_owned(),
            quantity: 1,
        },
    );
    assert!(base.items.iter().all(|item| item.id != "test.capture-ball"));
    assert_eq!(
        base.entities[0].controller_id.as_deref(),
        Some(base.player.id.as_str())
    );
    assert_eq!(base.entities[0].experience, 23);
}

#[test]
fn captured_state_round_trips_projects_details_and_regenerates_on_schedule() {
    let mut game =
        Game::new_with_build(0x5341_5645, "demo.build.cavalry").expect("Cavalry should create");
    clear_monsters(&mut game);
    let ball_index = equipped_capture_ball(&mut game);
    game.items[ball_index].captured_actor = Some(CapturedActor {
        kind_id: "demo.actor.horse".to_owned(),
        speed: 118,
        hp: 50,
        max_hp: 100,
        experience: 77,
    });
    game.world_tick = 30;
    let rng_before = game.rng.clone();
    game.process_captured_actor_regeneration();
    assert_eq!(game.rng, rng_before);
    assert_eq!(
        game.items[ball_index]
            .captured_actor
            .as_ref()
            .map(|actor| actor.hp),
        Some(51)
    );
    let projected = game
        .equipment_dto()
        .into_iter()
        .find(|item| item.id == "test.capture-ball")
        .expect("equipped ball should project");
    let captured = projected
        .captured_actor
        .expect("captured actor should project");
    assert_eq!(captured.kind_id, "demo.actor.horse");
    assert_eq!(captured.experience, 77);
    assert_eq!(projected.use_target_spec.expect("target spec").range, 1);

    let expected_save = game.to_save();
    let restored = Game::from_save_with_content(expected_save.clone(), game.content.clone())
        .expect("captured actor should round-trip");
    assert_eq!(restored.to_save(), expected_save);
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut invalid = game.to_save();
    invalid
        .equipment
        .iter_mut()
        .find(|item| item.id == "test.capture-ball")
        .and_then(|item| item.captured_actor.as_mut())
        .expect("capture ball should retain save state")
        .kind_id = "demo.actor.serpent-of-chaos".to_owned();
    assert!(matches!(
        Game::from_save_with_content(invalid, game.content.clone()),
        Err(CoreError::InvalidSave("captured actor state is invalid"))
    ));
}
