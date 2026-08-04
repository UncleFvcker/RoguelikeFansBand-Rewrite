// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn exploration_memory_does_not_change_authoritative_state_hash() {
    let mut game = Game::new(42);
    let before = game.state_hash();
    game.explored.fill(true);
    assert_eq!(game.state_hash(), before);

    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should store the entrance floor");
    let before_stored_memory_change = game.state_hash();
    game.stored_floors
        .get_mut("demo.floor.surface")
        .expect("the entrance floor should be stored")
        .explored
        .fill(false);
    assert_eq!(game.state_hash(), before_stored_memory_change);
}

#[test]
fn malformed_exploration_memory_is_rejected() {
    let mut payload = Game::new(42).to_save();
    payload.explored.pop();
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave(
            "exploration memory dimensions are invalid"
        ))
    ));
}

#[test]
fn malformed_revealed_terrain_knowledge_is_rejected() {
    let mut payload = Game::new(42).to_save();
    payload.revealed_terrain = vec![Position { x: 3, y: 3 }];
    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave(
            "revealed terrain knowledge is invalid"
        ))
    ));
}

#[test]
fn inventory_over_its_equipped_slot_capacity_is_rejected() {
    let mut game = Game::new(42);
    game.items.clear();
    for index in 0..27 {
        give_inventory_item(
            &mut game,
            &format!("test.inventory.item.{index}"),
            "demo.item.resonant-band",
        );
    }

    assert!(matches!(
        Game::from_save(game.to_save()),
        Err(CoreError::InvalidSave("inventory exceeds slot capacity"))
    ));
}

#[test]
fn v62_floor_with_obsolete_connection_set_uses_legacy_stair_fallback() {
    let mut game = Game::new(93);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
    let mut payload = game.to_save();
    payload.content_hash =
        "9d25687c1296bc6f9953024bd76bb9eefc4c1e3955280b96d34d565ff7ca289d".to_owned();
    let occupied = payload
        .floor_connections
        .iter()
        .map(|connection| connection.position)
        .chain(std::iter::once(payload.player.position))
        .collect::<BTreeSet<_>>();
    let legacy_index = payload
        .terrain
        .terrain_ids
        .iter()
        .enumerate()
        .find(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(payload.terrain.width))
                    .expect("x should fit i32"),
                y: i32::try_from(index / usize::from(payload.terrain.width))
                    .expect("y should fit i32"),
            };
            terrain_id.as_str() == "demo.terrain.floor" && !occupied.contains(&position)
        })
        .map(|(index, _)| index)
        .expect("generated floor should retain a legacy stair candidate");
    let legacy_position = Position {
        x: i32::try_from(legacy_index % usize::from(payload.terrain.width))
            .expect("x should fit i32"),
        y: i32::try_from(legacy_index / usize::from(payload.terrain.width))
            .expect("y should fit i32"),
    };
    payload.terrain.terrain_ids[legacy_index] = "demo.terrain.stairs-up".to_owned();
    payload.floor_connections.push(FloorConnectionSaveDto {
        id: "demo.connection.echo-depth-2.up-b".to_owned(),
        position: legacy_position,
        target_floor_id: None,
        target_connection_id: None,
    });
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let expected_draws = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v62 connection set should migrate");
    assert!(restored.floor_connections.is_empty());
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(restored.rng.draw_counter, expected_draws);
}

#[test]
fn previous_generated_floor_is_not_backfilled_with_v27_room_content() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("current content should generate the procedural floor");
    let mut payload = game.to_save();
    payload.content_hash =
        "febe50b7a55a637a05d78135f14aa8f72fa457632ae8d705c002e92acf9e4fd9".to_owned();
    payload.entities.clear();
    payload.items.clear();
    payload.carried_items.clear();
    payload.next_item_instance_serial = 2;

    let restored = Game::from_save(payload).expect("v26 generated floor should migrate");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert!(restored.entities.is_empty());
    assert!(restored.items.is_empty());
    assert_eq!(restored.next_item_instance_serial, 2);
}

#[test]
fn previous_generated_floor_is_not_backfilled_with_v28_door() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("current content should generate the procedural floor");
    let mut payload = game.to_save();
    payload.content_hash =
        "51ffdccfe19a9f159adc15c2f62965ff4a5d44b55990eb9f29df96870937a043".to_owned();
    let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
    payload.terrain.terrain_ids[door_index] = "demo.terrain.floor".to_owned();

    let restored = Game::from_save(payload).expect("v27 generated floor should migrate");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(
        restored.terrain_at(Position { x: 10, y: 4 }),
        "demo.terrain.floor"
    );
}

#[test]
fn previous_generated_floor_is_not_upgraded_to_a_v29_locked_door() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("current content should generate the procedural floor");
    let mut payload = game.to_save();
    payload.content_hash =
        "f060f44c88033e8ef75478929a354d6b5b0bc5f933ca2772e79c3440940942e8".to_owned();
    let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
    payload.terrain.terrain_ids[door_index] = "demo.terrain.door-closed".to_owned();

    let restored = Game::from_save(payload).expect("v28 generated floor should migrate");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(
        restored.terrain_at(Position { x: 10, y: 4 }),
        "demo.terrain.door-closed"
    );
}

#[test]
fn previous_generated_floor_is_not_upgraded_to_a_v31_secret_door() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("current content should generate the procedural floor");
    let mut payload = game.to_save();
    payload.content_hash =
        "2d2900d8052b0a600346d0b87cc3b3d5bb5138f851abbf2b95afa196bbbaaca2".to_owned();
    let door_index = 4_usize * usize::from(payload.terrain.width) + 10;
    payload.terrain.terrain_ids[door_index] = "demo.terrain.door-locked".to_owned();
    payload.revealed_terrain.clear();

    let restored = Game::from_save(payload).expect("v30 generated floor should migrate");
    let door_position = Position { x: 10, y: 4 };
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(
        restored.terrain_at(door_position),
        "demo.terrain.door-locked"
    );
    assert_eq!(
        restored.known_terrain_at(door_position),
        "demo.terrain.door-locked"
    );
}

#[test]
fn save_payload_restores_identical_state() {
    let mut game = Game::new(7);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
            slot_id: None,
        },
    ))
    .expect("equip should execute");

    let restored = Game::from_save(game.to_save()).expect("save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.snapshot().equipment.len(), 1);
}
