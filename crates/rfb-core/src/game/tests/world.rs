// SPDX-License-Identifier: MPL-2.0
use rfb_protocol::TerrainInteractionUnavailableReasonDto;

use super::support::*;
use super::*;

#[test]
fn warrens_every_generated_floor_has_a_normal_descent_and_return_route() {
    for seed in 0..16 {
        let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.explorer")
            .expect("Warrens journey should create");
        game.player
            .resistances
            .set(DamageType::Physical, ResistanceLevel::Immune);

        for depth in 1..=9 {
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
            dispatch_next(&mut game, GameCommand::TraverseStairs);
            assert_eq!(
                game.current_floor_id,
                format!("demo.floor.warrens-depth-{depth}")
            );
            assert!(game.terrain.iter().any(|id| id == "demo.terrain.stairs-up"));
            if depth < 9 {
                assert!(
                    game.terrain
                        .iter()
                        .any(|id| id == "demo.terrain.stairs-down")
                );
            }
        }

        for expected_depth in (1..=8).rev() {
            place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
            dispatch_next(&mut game, GameCommand::TraverseStairs);
            assert_eq!(
                game.current_floor_id,
                format!("demo.floor.warrens-depth-{expected_depth}")
            );
        }
        place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(game.current_floor_id, "demo.floor.surface");
    }
}

#[test]
fn procedural_floor_transition_is_deterministic_persistent_and_reversible() {
    let mut left = Game::new(27);
    let mut right = Game::new(27);
    for game in [&mut left, &mut right] {
        game.player.position = Position { x: 3, y: 4 };
    }

    let left_update = left
        .dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should generate the first floor");
    let right_update = right
        .dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("the same seed should generate the same floor");

    assert_eq!(left_update.floor_id, "demo.floor.echo-depth-1");
    assert_eq!(left_update.state_hash, right_update.state_hash);
    assert_eq!(left.rng.draw_counter, 21);
    assert_eq!(left.entities.len(), 4);
    let room_encounter = left
        .entities
        .iter()
        .find(|entity| entity.id == "demo.floor.echo-depth-1.encounter.1")
        .expect("floor encounter table should spawn its declared roll");
    assert_eq!(room_encounter.position, Position { x: 15, y: 11 });
    assert!(matches!(
        room_encounter.kind_id.as_str(),
        "demo.actor.acid-seep" | "demo.actor.echo-hound" | "demo.actor.frost-wisp"
    ));
    let nest = left
        .entities
        .iter()
        .filter(|entity| entity.id.starts_with("demo.floor.echo-depth-1.nest."))
        .collect::<Vec<_>>();
    assert_eq!(nest.len(), 3);
    assert!(nest.iter().all(|entity| entity.kind_id == nest[0].kind_id));
    assert!(nest.iter().all(|entity| !matches!(
        entity.kind_id.as_str(),
        "demo.actor.storm-spark" | "demo.actor.venom-spore"
    )));
    assert!(
        left.entities
            .iter()
            .all(|entity| !entity.id.contains(".vault-group."))
    );
    let floor_loot = left
        .items
        .iter()
        .find(|item| matches!(item.location, ItemLocation::Ground(_)))
        .expect("the generated floor should contain ground loot");
    assert_eq!(floor_loot.id, "generated.item.2");
    assert_eq!(floor_loot.kind_id, "demo.item.luminous-shard");
    assert_eq!(floor_loot.quantity, 2);
    assert_eq!(left.stored_floors.len(), 1);
    assert_eq!(
        left.terrain_at(left.player.position),
        "demo.terrain.stairs-up"
    );
    assert!(left_update.events.iter().any(|event| {
        event.kind == "floor.transition"
            && event.args["from"] == "demo.floor.surface"
            && event.args["to"] == "demo.floor.echo-depth-1"
    }));

    let mut restored = Game::from_save(left.to_save()).expect("generated floor should reload");
    assert_eq!(restored.state_hash(), left.state_hash());
    let return_update = restored
        .dispatch(command(2, 1, GameCommand::TraverseStairs))
        .expect("ascending should restore the entrance floor");
    assert_eq!(return_update.floor_id, "demo.floor.surface");
    assert_eq!(restored.player.position, Position { x: 3, y: 4 });
    assert_eq!(restored.entities.len(), 2);
    assert!(
        restored
            .entities
            .iter()
            .any(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
    );
    assert!(restored.stored_floors.is_empty());
    assert!(
        return_update
            .events
            .iter()
            .any(|event| event.message_key == "floor-expedition-ended")
    );

    let draws_before_reentry = restored.rng.draw_counter;
    let reentry_update = restored
        .dispatch(command(3, 2, GameCommand::TraverseStairs))
        .expect("descending again should generate a new expedition floor");
    assert_eq!(reentry_update.floor_id, "demo.floor.echo-depth-1");
    assert!(restored.rng.draw_counter > draws_before_reentry);
    assert_eq!(restored.entities.len(), 4);
    assert_eq!(restored.items.len(), 1);
}

#[test]
fn dungeon_instances_are_numbered_and_old_instance_lifecycle_is_scoped() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("first dungeon entry should resolve")
        .expect("first dungeon entry should transition");
    assert_eq!(
        game.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.echo-depths.instance.1")
    );
    assert_eq!(
        game.stored_floors["demo.floor.surface"].dungeon_instance_id,
        None
    );
    let first_payload = game.to_save();
    assert_eq!(
        first_payload.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.echo-depths.instance.1")
    );
    assert_eq!(
        first_payload
            .dungeon_states
            .iter()
            .find(|state| state.dungeon_id == "demo.dungeon.echo-depths")
            .map(|state| state.next_instance_ordinal),
        Some(1)
    );
    let mut legacy_v64_payload = first_payload.clone();
    legacy_v64_payload.current_dungeon_instance_id = None;
    for floor in &mut legacy_v64_payload.stored_floors {
        floor.dungeon_instance_id = None;
    }
    for state in &mut legacy_v64_payload.dungeon_states {
        state.next_instance_ordinal = 0;
    }
    let migrated = Game::from_save(legacy_v64_payload)
        .expect("v64 dungeon save should migrate its first instance");
    assert_eq!(
        migrated.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.echo-depths.instance.1")
    );
    assert_eq!(
        migrated.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
        1
    );
    assert_eq!(migrated.state_hash(), game.state_hash());

    traverse_connection(&mut game, "demo.connection.echo-depth-1.surface-up");
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert!(game.stored_floors.values().all(|floor| {
        floor.dungeon_instance_id.as_deref() != Some("demo.dungeon.echo-depths.instance.1")
    }));

    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("second dungeon entry should resolve")
        .expect("second dungeon entry should transition");
    assert_eq!(
        game.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.echo-depths.instance.2")
    );
    assert_eq!(
        game.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
        2
    );
}

#[test]
fn configurable_dungeon_lifecycle_retains_then_expires_instances() {
    let mut game = Game::new(73);
    game.player.position = Position { x: 7, y: 2 };
    game.traverse_stairs(false)
        .expect("archive entry should resolve")
        .expect("archive entry should transition");
    assert_eq!(game.current_floor_id, "demo.floor.archive-depth-1");
    assert_eq!(
        game.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.archive-depths.instance.1")
    );
    let archive_item_id = game
        .items
        .iter()
        .find_map(|item| matches!(item.location, ItemLocation::Ground(_)).then(|| item.id.clone()))
        .expect("archive generation should create ground loot");
    game.explored[0] = true;
    game.item_property_knowledge.insert(
        archive_item_id.clone(),
        ItemPropertyKnowledgeState {
            appraised: true,
            identified: false,
            known_affix_ids: BTreeSet::new(),
        },
    );

    let archive_up = connection_position(&game, "demo.connection.archive-depth-1.surface-up");
    game.player.position = archive_up;
    game.traverse_stairs(false)
        .expect("archive return should resolve")
        .expect("archive return should transition");
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(
        game.dungeon_states["demo.dungeon.archive-depths"]
            .retained_instance_id
            .as_deref(),
        Some("demo.dungeon.archive-depths.instance.1")
    );
    game.turn = game.dungeon_states["demo.dungeon.archive-depths"]
        .retained_at_turn
        .expect("archive should record the completed return turn");
    assert!(game.item_property_knowledge.contains_key(&archive_item_id));

    let saved = game.to_save();
    let retained_state = saved
        .dungeon_states
        .iter()
        .find(|state| state.dungeon_id == "demo.dungeon.archive-depths")
        .expect("archive dungeon save state should exist");
    assert_eq!(
        retained_state.retained_instance_id.as_deref(),
        Some("demo.dungeon.archive-depths.instance.1")
    );
    assert_eq!(retained_state.retained_at_turn, Some(1));
    let mut restored = Game::from_save(saved).expect("retained archive should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    restored.player.position = Position { x: 7, y: 2 };
    restored
        .traverse_stairs(false)
        .expect("retained archive reentry should resolve")
        .expect("retained archive reentry should transition");
    assert_eq!(
        restored.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.archive-depths.instance.1")
    );
    assert!(restored.explored[0]);
    assert!(restored.items.iter().any(|item| item.id == archive_item_id));
    assert!(
        restored
            .item_property_knowledge
            .contains_key(&archive_item_id)
    );

    let archive_up = connection_position(&restored, "demo.connection.archive-depth-1.surface-up");
    restored.player.position = archive_up;
    restored
        .traverse_stairs(false)
        .expect("archive second return should resolve")
        .expect("archive second return should transition");
    let retained_at_turn = restored.dungeon_states["demo.dungeon.archive-depths"]
        .retained_at_turn
        .expect("archive should record retention turn");
    restored.turn = retained_at_turn.saturating_add(3);
    restored.player.position = Position { x: 7, y: 2 };
    restored
        .traverse_stairs(false)
        .expect("expired archive reentry should resolve")
        .expect("expired archive reentry should transition");
    assert_eq!(
        restored.current_dungeon_instance_id.as_deref(),
        Some("demo.dungeon.archive-depths.instance.2")
    );
    assert!(
        !restored
            .item_property_knowledge
            .contains_key(&archive_item_id)
    );
    assert!(
        !restored
            .stored_floors
            .values()
            .any(|floor| floor.dungeon_instance_id.as_deref()
                == Some("demo.dungeon.archive-depths.instance.1"))
    );

    let mut malformed = restored.to_save();
    let archive_state = malformed
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.archive-depths")
        .expect("archive dungeon save state should exist");
    archive_state.retained_instance_id = Some("demo.dungeon.archive-depths.instance.2".to_owned());
    archive_state.retained_at_turn = None;
    assert!(matches!(
        Game::from_save(malformed),
        Err(CoreError::InvalidSave(
            "retained dungeon instance state is incomplete"
        ))
    ));
}

#[test]
fn entrance_guardian_holds_position_but_does_not_block_dungeon_entry() {
    let mut game = Game::new(42);
    let guardian_id = "demo.z-entrance-guardian.resonance-descent.1";
    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == guardian_id)
        .expect("resonance entrance guardian should spawn");
    let guardian_position = game.entities[guardian_index].position;
    assert_eq!(guardian_position, Position { x: 2, y: 1 });
    assert!(
        game.entities[guardian_index]
            .pack
            .as_ref()
            .is_some_and(|pack| {
                pack.role == MonsterPackRoleDto::Leader
                    && pack.behavior == MonsterPackBehaviorDto::GuardPosition
            })
    );

    game.resolve_monster_action(
        guardian_index,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated guardian action should resolve");
    assert_eq!(game.entities[guardian_index].position, guardian_position);

    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("resonance entry should resolve")
        .expect("a living entrance guardian must remain a soft gate");
    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-1");
    assert!(
        stored_floor(&game, "demo.floor.surface")
            .entities
            .iter()
            .any(|entity| entity.id == guardian_id)
    );

    game.traverse_stairs(false)
        .expect("surface return should resolve")
        .expect("root stairs should return to the surface");
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert!(game.entities.iter().any(|entity| entity.id == guardian_id));
}

#[test]
fn entrance_guardian_defeat_persists_and_v66_migration_does_not_backfill_it() {
    let guardian_id = "demo.z-entrance-guardian.resonance-descent.1";
    let mut game = Game::new(42);
    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == guardian_id)
        .expect("resonance entrance guardian should spawn");
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_actor_death(
        guardian_index,
        DomainEvent::EntityDiedFromStatus {
            target_kind_id: "demo.actor.resonant-warden".to_owned(),
            status_kind_id: STATUS_POISON.to_owned(),
            damage: DamageOutcome {
                raw: 6,
                armor_reduction: 0,
                requested: 6,
                applied: 6,
                resistance_delta: 0,
                damage_type: DamageType::Poison,
                resistance: crate::resistance::ResistanceLevel::Normal,
            },
        },
        &mut events,
        &mut changed,
        &mut removed,
    )
    .expect("entrance guardian death should resolve");
    assert!(game.dungeon_states["demo.dungeon.resonance-descent"].entrance_guardian_defeated);
    assert_eq!(removed, [guardian_id]);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::DungeonEntranceGuardianDefeated { dungeon_id, .. }
            if dungeon_id == "demo.dungeon.resonance-descent"
    )));
    let restored = Game::from_save(game.to_save()).expect("guardian defeat should round-trip");
    assert!(
        !restored
            .entities
            .iter()
            .any(|entity| entity.id == guardian_id)
    );

    let mut v66 = Game::new(42).to_save();
    v66.content_hash =
        "834acbe3d025810eb1399db74689d35a4d3dae34862bcbf1271c8d20ad11d9fc".to_owned();
    v66.entities.retain(|entity| entity.id != guardian_id);
    for state in &mut v66.dungeon_states {
        state.entrance_guardian_defeated = None;
    }
    let draws = v66.rng.draw_counter;
    let migrated = Game::from_save(v66).expect("v66 surface should migrate without backfill");
    assert!(migrated.dungeon_states["demo.dungeon.resonance-descent"].entrance_guardian_defeated);
    assert!(
        !migrated
            .entities
            .iter()
            .any(|entity| entity.id == guardian_id)
    );
    assert_eq!(migrated.rng.draw_counter, draws);
}

#[test]
fn dungeon_entry_requirements_use_existing_authoritative_state() {
    let mut game = Game::new(42);
    let mut dungeon = game
        .content
        .world(&game.world_id)
        .expect("demo world should exist")
        .dungeons[0]
        .clone();

    dungeon.entry_requirements = vec![DungeonEntryRequirementDefinition::TaskStatus {
        task_id: "demo.task.echo-bounty".to_owned(),
        status: DungeonEntryTaskStatus::Completed,
    }];
    assert!(!game.dungeon_entry_requirements_met(&dungeon));
    game.task_states
        .get_mut("demo.task.echo-bounty")
        .expect("bounty task state should exist")
        .status = TaskStatusKindDto::Completed;
    assert!(game.dungeon_entry_requirements_met(&dungeon));

    dungeon.entry_requirements = vec![DungeonEntryRequirementDefinition::CarriedItem {
        item_kind_id: "demo.item.luminous-shard".to_owned(),
        quantity: 5,
    }];
    assert!(!game.dungeon_entry_requirements_met(&dungeon));
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("surface shard should exist")
        .location = ItemLocation::Inventory;
    assert!(game.dungeon_entry_requirements_met(&dungeon));

    dungeon.entry_requirements = vec![DungeonEntryRequirementDefinition::DungeonConquered {
        dungeon_id: "demo.dungeon.resonance-descent".to_owned(),
    }];
    assert!(!game.dungeon_entry_requirements_met(&dungeon));
    game.dungeon_states
        .get_mut("demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .guardian_defeated = true;
    assert!(game.dungeon_entry_requirements_met(&dungeon));
}

#[test]
fn unmet_dungeon_entry_requirement_is_atomic_before_instance_allocation() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root)
        .expect("demo pack should compile for the requirement test");
    artifact
        .content
        .worlds
        .first_mut()
        .expect("demo world should exist")
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.echo-depths")
        .expect("echo dungeon should exist")
        .entry_requirements = vec![DungeonEntryRequirementDefinition::CarriedItem {
        item_kind_id: "demo.item.luminous-shard".to_owned(),
        quantity: 99,
    }];
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("custom requirement content should remain valid"),
    ));
    let mut game = Game::from_content(27, catalog, BUILT_IN_WORLD_ID)
        .expect("custom requirement game should initialize");
    game.player.position = Position { x: 3, y: 4 };
    let draws = game.rng.draw_counter;
    let result = game
        .traverse_stairs(false)
        .expect("unmet requirement should be a normal unavailable transition");
    assert!(result.is_none());
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.rng.draw_counter, draws);
    assert_eq!(
        game.dungeon_states["demo.dungeon.echo-depths"].next_instance_ordinal,
        0
    );
    assert!(game.stored_floors.is_empty());
}

#[test]
fn ending_a_dungeon_instance_does_not_clear_stored_task_floors() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("dungeon entry should resolve")
        .expect("dungeon entry should transition");
    let task_definition = game
        .content
        .world(&game.world_id)
        .expect("world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.lifecycle == FloorLifecycle::OneShot)
        .cloned()
        .expect("demo should contain a task floor");
    let task_floor = game
        .generate_procedural_floor(&task_definition, None)
        .expect("task floor should generate for the fixture");
    game.stored_floors.insert(task_floor.id.clone(), task_floor);
    traverse_connection(&mut game, "demo.connection.echo-depth-1.surface-up");
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert!(
        game.stored_floors
            .values()
            .any(|floor| { floor.id == task_definition.id && floor.dungeon_instance_id.is_none() })
    );
}

#[test]
fn floor_connections_are_seeded_distinct_and_content_authoritative() {
    let mut left = Game::new(27);
    let mut right = Game::new(27);
    for game in [&mut left, &mut right] {
        game.player.position = Position { x: 3, y: 4 };
        game.traverse_stairs(false)
            .expect("echo dungeon entry should resolve")
            .expect("echo dungeon entry should transition");
    }

    assert_eq!(left.floor_connections, right.floor_connections);
    assert_eq!(left.terrain, right.terrain);
    assert_eq!(left.floor_connections.len(), 4);
    assert_eq!(
        left.player.position,
        connection_position(&left, "demo.connection.echo-depth-1.surface-up")
    );
    let positions = left
        .floor_connections
        .iter()
        .map(|connection| connection.position)
        .collect::<BTreeSet<_>>();
    assert_eq!(positions.len(), left.floor_connections.len());
    for connection in &left.floor_connections {
        let definition = left
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == left.current_floor_id)
            .expect("current procedural floor should exist")
            .connections
            .iter()
            .find(|candidate| candidate.id == connection.id)
            .expect("generated connection should exist in content");
        assert_eq!(left.terrain_at(connection.position), definition.terrain_id);
    }

    let layouts = (0..8)
        .map(|seed| {
            let mut game = Game::new(seed);
            game.player.position = Position { x: 3, y: 4 };
            game.traverse_stairs(false)
                .expect("echo dungeon entry should resolve")
                .expect("echo dungeon entry should transition");
            game.floor_connections
                .iter()
                .map(|connection| {
                    (
                        connection.id.clone(),
                        connection.position.x,
                        connection.position.y,
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeSet<_>>();
    assert!(layouts.len() > 1);
}

#[test]
fn paired_stairs_and_shaft_use_independent_arrival_connections() {
    let mut game = Game::new(71);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");

    let down_a = connection_position(&game, "demo.connection.echo-depth-1.down-a");
    let down_b = connection_position(&game, "demo.connection.echo-depth-1.down-b");
    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2");
    assert_eq!(
        game.player.position,
        connection_position(&game, "demo.connection.echo-depth-2.up-a")
    );
    traverse_connection(&mut game, "demo.connection.echo-depth-2.up-a");
    assert_eq!(game.player.position, down_a);

    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-b");
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2-mirror");
    assert_eq!(
        game.player.position,
        connection_position(&game, "demo.connection.echo-depth-2-mirror.up-a")
    );
    traverse_connection(&mut game, "demo.connection.echo-depth-2-mirror.up-a");
    assert_eq!(game.player.position, down_b);

    let shaft_down = connection_position(&game, "demo.connection.echo-depth-1.shaft-down");
    traverse_connection(&mut game, "demo.connection.echo-depth-1.shaft-down");
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-3-shaft");
    assert_eq!(
        game.player.position,
        connection_position(&game, "demo.connection.echo-depth-3-shaft.shaft-up")
    );
    traverse_connection(&mut game, "demo.connection.echo-depth-3-shaft.shaft-up");
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(game.player.position, shaft_down);
}

#[test]
fn floor_connections_round_trip_and_reject_invalid_authoritative_state() {
    let mut game = Game::new(93);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    let payload = game.to_save();
    let restored = Game::from_save(payload.clone()).expect("connections should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.floor_connections, game.floor_connections);

    let mut missing = payload.clone();
    missing.floor_connections.pop();
    assert!(Game::from_save(missing).is_err());

    let mut duplicate = payload.clone();
    duplicate.floor_connections[1].id = duplicate.floor_connections[0].id.clone();
    assert!(Game::from_save(duplicate).is_err());

    let mut mismatched_terrain = payload;
    let position = mismatched_terrain.floor_connections[0].position;
    let index =
        position.y as usize * usize::from(mismatched_terrain.terrain.width) + position.x as usize;
    mismatched_terrain.terrain.terrain_ids[index] = "demo.terrain.floor".to_owned();
    assert!(Game::from_save(mismatched_terrain).is_err());

    let mut undeclared_target = game.to_save();
    undeclared_target.floor_connections[0].target_floor_id =
        Some("demo.floor.echo-depth-3".to_owned());
    undeclared_target.floor_connections[0].target_connection_id =
        Some("demo.connection.echo-depth-3.up-a".to_owned());
    assert!(Game::from_save(undeclared_target).is_err());
}

#[test]
fn dynamic_connection_targets_form_distinct_branches_and_survive_reload() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    let down_a = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.down-a")
        .expect("dynamic down-a should exist")
        .clone();
    let down_b = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.echo-depth-1.down-b")
        .expect("dynamic down-b should exist")
        .clone();
    assert!(down_a.target_floor_id.is_some());
    assert!(down_b.target_floor_id.is_some());
    assert_ne!(down_a.target_floor_id, down_b.target_floor_id);

    let payload = game.to_save();
    let restored = Game::from_save(payload.clone()).expect("dynamic targets should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.floor_connections, game.floor_connections);

    let mut legacy = payload;
    for connection in &mut legacy.floor_connections {
        connection.target_floor_id = None;
        connection.target_connection_id = None;
    }
    let legacy = Game::from_save(legacy).expect("missing target fields should use content");
    assert!(
        legacy
            .floor_connections
            .iter()
            .all(|connection| connection.target_floor_id.is_none())
    );
}

#[test]
fn previous_v57_floor_without_connection_state_uses_legacy_stairs_without_rebuild() {
    let mut game = Game::new(117);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    let down_a = connection_position(&game, "demo.connection.echo-depth-1.down-a");
    let surface_up = connection_position(&game, "demo.connection.echo-depth-1.surface-up");
    let mut payload = game.to_save();
    for connection in &payload.floor_connections {
        let index = connection.position.y as usize * usize::from(payload.terrain.width)
            + connection.position.x as usize;
        payload.terrain.terrain_ids[index] = if connection.position == down_a {
            "demo.terrain.stairs-down".to_owned()
        } else if connection.position == surface_up {
            "demo.terrain.stairs-up".to_owned()
        } else {
            "demo.terrain.floor".to_owned()
        };
    }
    payload.player.position = down_a;
    payload.floor_connections.clear();
    payload.content_hash =
        "d209d68a6a39af21eee8d1a951684be86e847ab570823c9c2604fa199e4571e1".to_owned();
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let mut restored = Game::from_save(payload).expect("v57 floor should migrate");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(restored.floor_connections.is_empty());

    restored
        .traverse_stairs(false)
        .expect("legacy stairs should resolve")
        .expect("legacy stairs should transition");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-2");
}

#[test]
fn terrain_interaction_plans_reject_unsupported_actions_without_rng() {
    let mut game = Game::new(42);
    for direction in TERRAIN_INTERACTION_DIRECTIONS {
        let position = game.position_in_direction(direction);
        replace_terrain(&mut game, position, "demo.terrain.floor");
        game.revealed_terrain.remove(&position);
    }
    let terrain_before = game.terrain.clone();
    let revealed_before = game.revealed_terrain.clone();
    let draws_before = game.rng_draw_counter();

    assert!(game.open_door(Direction::North).is_none());
    assert!(game.close_door(Direction::North).is_none());
    assert!(game.bash_door(Direction::North).is_none());
    assert!(game.disarm_trap(Direction::North).is_none());
    assert!(game.dig_terrain(Direction::North).is_none());
    assert!(game.search_hidden_terrain().is_empty());

    assert_eq!(game.terrain, terrain_before);
    assert_eq!(game.revealed_terrain, revealed_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn checked_terrain_plans_commit_disarm_and_dig_targets() {
    let mut game = Game::new(27);
    let trap_position = game.position_in_direction(Direction::South);
    let trap_id = "demo.terrain.trap-echo-snare";
    let disarmed_id = game
        .content
        .terrain(trap_id)
        .and_then(|terrain| terrain.trap.as_ref())
        .expect("test trap should define disarm behavior")
        .disarm_to_terrain_id
        .clone();
    replace_terrain(&mut game, trap_position, trap_id);
    game.revealed_terrain.insert(trap_position);
    let draws_before_disarm = game.rng_draw_counter();

    let disarmed_position = (0..64)
        .find_map(|_| match game.disarm_trap(Direction::South) {
            Some(TrapDisarmOutcome::Succeeded { position }) => Some(position),
            Some(TrapDisarmOutcome::Failed { .. }) => None,
            None => panic!("revealed trap should remain disarmable until success"),
        })
        .expect("fixed seed should eventually disarm the trap");
    assert_eq!(disarmed_position, trap_position);
    assert_eq!(game.terrain_at(trap_position), disarmed_id);
    assert!(!game.revealed_terrain.contains(&trap_position));
    assert!(game.rng_draw_counter() > draws_before_disarm);

    let dig_position = game.position_in_direction(Direction::SouthEast);
    let dig_id = "demo.terrain.echo-rubble";
    let dug_id = game
        .content
        .terrain(dig_id)
        .expect("test terrain should exist")
        .dig_to_terrain_id
        .clone()
        .expect("test terrain should define a dig target");
    replace_terrain(&mut game, dig_position, dig_id);
    game.revealed_terrain.insert(dig_position);
    let draws_before_dig = game.rng_draw_counter();

    let dug_position = (0..64)
        .find_map(|_| match game.dig_terrain(Direction::SouthEast) {
            Some(TerrainDigOutcome::Succeeded { position }) => Some(position),
            Some(TerrainDigOutcome::Failed { .. }) => None,
            None => panic!("diggable terrain should remain available until success"),
        })
        .expect("fixed seed should eventually dig the terrain");
    assert_eq!(dug_position, dig_position);
    assert_eq!(game.terrain_at(dig_position), dug_id);
    assert!(!game.revealed_terrain.contains(&dig_position));
    assert!(game.rng_draw_counter() > draws_before_dig);
}

#[test]
fn locked_door_checks_update_collision_visibility_and_persist() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should generate the closed door");
    let door_position = Position { x: 10, y: 4 };
    assert_eq!(game.terrain_at(door_position), "demo.terrain.door-secret");
    assert!(!game.is_walkable(door_position));

    game.player.position = Position { x: 9, y: 4 };
    game.revealed_terrain.insert(door_position);
    assert_eq!(
        visual_at(&game.snapshot(), Position { x: 11, y: 4 }).visibility,
        VisibilityState::Hidden
    );
    let draws_before_unlock = game.rng.draw_counter;
    let mut saw_failed_unlock = false;
    let open_update = (0..12)
        .find_map(|_| {
            let update = dispatch_next(
                &mut game,
                GameCommand::OpenDoor {
                    direction: Direction::East,
                },
            );
            saw_failed_unlock |= update
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-unlock-failed");
            (game.terrain_at(door_position) == "demo.terrain.door-open").then_some(update)
        })
        .expect("fixed seed should eventually unlock the door");
    assert!(saw_failed_unlock);
    assert_eq!(game.terrain_at(door_position), "demo.terrain.door-open");
    assert!(game.is_walkable(door_position));
    assert!(game.rng.draw_counter > draws_before_unlock);
    let terrain_events = open_update
        .events
        .iter()
        .filter(|event| event.kind.starts_with("terrain."))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        terrain_events,
        ["terrain.door-unlocked", "terrain.door-opened"]
    );
    assert_eq!(
        visual_at(&game.snapshot(), Position { x: 11, y: 4 }).visibility,
        VisibilityState::Visible
    );

    let mut restored = Game::from_save(game.to_save()).expect("open door should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.terrain_at(door_position), "demo.terrain.door-open");

    restored.player.position = Position { x: 5, y: 4 };
    dispatch_next(&mut restored, GameCommand::TraverseStairs);
    dispatch_next(&mut restored, GameCommand::TraverseStairs);
    let fresh_door_index = restored
        .terrain
        .iter()
        .position(|terrain_id| terrain_id == "demo.terrain.door-secret")
        .expect("fresh floor should contain a secret door");
    let fresh_door_position = Position {
        x: i32::try_from(fresh_door_index % usize::from(restored.width))
            .expect("door x must fit i32"),
        y: i32::try_from(fresh_door_index / usize::from(restored.width))
            .expect("door y must fit i32"),
    };
    assert_eq!(
        restored.terrain_at(fresh_door_position),
        "demo.terrain.door-secret"
    );

    restored.player.position = Position {
        x: fresh_door_position.x - 1,
        y: fresh_door_position.y,
    };
    let close_update = dispatch_next(
        &mut restored,
        GameCommand::CloseDoor {
            direction: Direction::East,
        },
    );
    assert_eq!(
        restored.terrain_at(fresh_door_position),
        "demo.terrain.door-secret"
    );
    assert!(
        close_update
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-close-unavailable")
    );

    let unavailable = dispatch_next(
        &mut restored,
        GameCommand::CloseDoor {
            direction: Direction::East,
        },
    );
    assert!(
        unavailable
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-close-unavailable")
    );
}

#[test]
fn bashing_a_locked_door_is_deterministic_and_leaves_a_broken_door() {
    let mut game = Game::new(0);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should generate the locked door");
    game.player.position = Position { x: 9, y: 4 };
    let door_position = Position { x: 10, y: 4 };
    game.revealed_terrain.insert(door_position);
    let draws_before_bash = game.rng.draw_counter;
    let mut saw_failed_bash = false;
    let succeeded = (0..12)
        .find_map(|_| {
            let update = dispatch_next(
                &mut game,
                GameCommand::BashDoor {
                    direction: Direction::East,
                },
            );
            saw_failed_bash |= update
                .events
                .iter()
                .any(|event| event.kind == "terrain.door-bash-failed");
            (game.terrain_at(door_position) == "demo.terrain.door-broken").then_some(update)
        })
        .expect("fixed seed should eventually bash the door open");
    assert!(saw_failed_bash);
    assert_eq!(game.terrain_at(door_position), "demo.terrain.door-broken");
    assert!(game.is_walkable(door_position));
    assert!(game.rng.draw_counter > draws_before_bash);
    assert!(
        succeeded
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-bashed-open")
    );

    let mut restored = Game::from_save(game.to_save()).expect("broken door should reload");
    assert_eq!(
        restored.terrain_at(door_position),
        "demo.terrain.door-broken"
    );
    let unavailable = dispatch_next(
        &mut restored,
        GameCommand::CloseDoor {
            direction: Direction::East,
        },
    );
    assert!(
        unavailable
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-close-unavailable")
    );
}

#[test]
fn terrain_interaction_query_is_stable_and_reports_blockers() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should generate the locked door");
    game.player.position = Position { x: 9, y: 4 };
    let door_position = Position { x: 10, y: 4 };

    assert!(game.snapshot().terrain_interactions.is_empty());
    assert_eq!(game.known_terrain_at(door_position), "demo.terrain.wall");
    game.revealed_terrain.insert(door_position);
    let locked = game.snapshot().terrain_interactions;
    assert_eq!(locked.len(), 2);
    assert_eq!(
        locked
            .iter()
            .map(|interaction| (
                interaction.kind,
                interaction.direction,
                interaction.position,
                interaction.terrain_id.as_str(),
                interaction.requires_check,
                interaction.available,
                interaction.unavailable_reason,
            ))
            .collect::<Vec<_>>(),
        [
            (
                TerrainInteractionKindDto::OpenDoor,
                Direction::East,
                door_position,
                "demo.terrain.door-secret",
                true,
                true,
                None,
            ),
            (
                TerrainInteractionKindDto::BashDoor,
                Direction::East,
                door_position,
                "demo.terrain.door-secret",
                true,
                true,
                None,
            ),
        ]
    );

    (0..12)
        .find(|_| {
            dispatch_next(
                &mut game,
                GameCommand::OpenDoor {
                    direction: Direction::East,
                },
            );
            game.terrain_at(door_position) == "demo.terrain.door-open"
        })
        .expect("fixed seed should eventually unlock the queried door");
    let open = game.snapshot().terrain_interactions;
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].kind, TerrainInteractionKindDto::CloseDoor);
    assert!(!open[0].requires_check);
    assert!(open[0].available);

    game.items[0].location = ItemLocation::Ground(door_position);
    let blocked_by_item = game.snapshot().terrain_interactions;
    assert!(!blocked_by_item[0].available);
    assert_eq!(
        blocked_by_item[0].unavailable_reason,
        Some(TerrainInteractionUnavailableReasonDto::OccupiedByItem)
    );

    game.entities[0].position = door_position;
    let blocked_by_actor = game.snapshot().terrain_interactions;
    assert!(!blocked_by_actor[0].available);
    assert_eq!(
        blocked_by_actor[0].unavailable_reason,
        Some(TerrainInteractionUnavailableReasonDto::OccupiedByActor)
    );
}

#[test]
fn search_discovers_secret_terrain_without_leaking_true_terrain() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("descending should generate the secret door");
    game.player.position = Position { x: 9, y: 4 };
    let door_position = Position { x: 10, y: 4 };
    assert_eq!(game.terrain_at(door_position), "demo.terrain.door-secret");
    assert_eq!(game.known_terrain_at(door_position), "demo.terrain.wall");
    assert!(game.snapshot().terrain_interactions.is_empty());
    let draws_before_search = game.rng.draw_counter;

    let hidden_open = game
        .dispatch(command(
            2,
            1,
            GameCommand::OpenDoor {
                direction: Direction::East,
            },
        ))
        .expect("an undiscovered secret door should reject direct opening");
    assert_eq!(game.rng.draw_counter, draws_before_search);
    assert!(
        hidden_open
            .events
            .iter()
            .any(|event| event.kind == "terrain.door-open-unavailable")
    );

    let discovered = (0..12)
        .find_map(|_| {
            let update = dispatch_next(&mut game, GameCommand::Search);
            game.revealed_terrain
                .contains(&door_position)
                .then_some(update)
        })
        .expect("fixed seed should eventually discover the secret door");
    assert!(game.rng.draw_counter > draws_before_search);
    assert_eq!(
        game.known_terrain_at(door_position),
        "demo.terrain.door-secret"
    );
    assert!(game.revealed_terrain.contains(&door_position));
    assert!(
        discovered
            .events
            .iter()
            .any(|event| event.kind == "terrain.secret-discovered")
    );
    assert_eq!(discovered.terrain_interactions.len(), 2);
    assert!(discovered.changed_cells.iter().any(
        |cell| cell.position == door_position && cell.terrain_id == "demo.terrain.door-secret"
    ));
    let mut hidden_again = game.clone();
    hidden_again.revealed_terrain.clear();
    assert_ne!(hidden_again.state_hash(), game.state_hash());

    let mut restored = Game::from_save(game.to_save()).expect("terrain knowledge should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.known_terrain_at(door_position),
        "demo.terrain.door-secret"
    );
    restored.player.position = Position { x: 5, y: 4 };
    dispatch_next(&mut restored, GameCommand::TraverseStairs);
    dispatch_next(&mut restored, GameCommand::TraverseStairs);
    let fresh_door_index = restored
        .terrain
        .iter()
        .position(|terrain_id| terrain_id == "demo.terrain.door-secret")
        .expect("fresh floor should contain a secret door");
    let fresh_door_position = Position {
        x: i32::try_from(fresh_door_index % usize::from(restored.width))
            .expect("door x must fit i32"),
        y: i32::try_from(fresh_door_index / usize::from(restored.width))
            .expect("door y must fit i32"),
    };
    assert_eq!(
        restored.known_terrain_at(fresh_door_position),
        "demo.terrain.wall"
    );
}

#[test]
fn stairs_command_off_stairs_keeps_the_current_floor() {
    let mut game = Game::new(42);
    let update = game
        .dispatch(command(1, 0, GameCommand::TraverseStairs))
        .expect("unavailable stairs command should remain a valid turn");

    assert_eq!(update.floor_id, "demo.floor.surface");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "floor.transition-unavailable")
    );
    assert!(game.stored_floors.is_empty());
}
