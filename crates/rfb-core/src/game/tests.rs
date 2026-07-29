// SPDX-License-Identifier: MPL-2.0
use crate::effect::StatusInstance;
use crate::resistance::ResistanceLevel;
use rfb_protocol::{
    CheckOutcomeDto, CheckResolutionDto, DamageResolutionDto, DamageTypeDto,
    DeviceRechargeSourceDto, Direction, GameCommand, GameCommandEnvelope, GameEventOutcomeDto,
    ResistanceLevelDto, ResistanceSaveDto, StatusSaveDto, VisibilityState,
};

use super::*;

fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
    GameCommandEnvelope {
        command_seq: seq,
        expected_revision: revision,
        command,
    }
}

fn dispatch_next(game: &mut Game, command_value: GameCommand) -> GameUpdate {
    let snapshot = game.snapshot();
    game.dispatch(command(
        snapshot.last_command_seq + 1,
        snapshot.revision,
        command_value,
    ))
    .expect("test command should execute")
}

fn descend_one_floor(game: &mut Game) {
    if game.current_floor_id == "demo.floor.surface" {
        game.player.position = Position { x: 3, y: 4 };
    } else {
        let down_index = game
            .terrain
            .iter()
            .position(|terrain_id| terrain_id == "demo.terrain.stairs-down")
            .expect("current floor should contain descending stairs");
        game.player.position = Position {
            x: i32::try_from(down_index % usize::from(game.width))
                .expect("descending stair x must fit i32"),
            y: i32::try_from(down_index / usize::from(game.width))
                .expect("descending stair y must fit i32"),
        };
    }
    game.traverse_stairs(false)
        .expect("descent should resolve")
        .expect("descent should transition");
}

fn connection_position(game: &Game, connection_id: &str) -> Position {
    game.floor_connections
        .iter()
        .find(|connection| connection.id == connection_id)
        .unwrap_or_else(|| panic!("floor should contain connection {connection_id}"))
        .position
}

fn traverse_connection(game: &mut Game, connection_id: &str) {
    game.player.position = connection_position(game, connection_id);
    game.traverse_stairs(false)
        .expect("connection traversal should resolve")
        .expect("connection traversal should transition");
}

fn stored_floor<'a>(game: &'a Game, floor_id: &str) -> &'a FloorState {
    game.stored_floors
        .values()
        .find(|floor| floor.id == floor_id)
        .unwrap_or_else(|| panic!("stored floor {floor_id} should exist"))
}

fn region_at(game: &Game, position: Position) -> &FloorRegionState {
    game.floor_regions
        .iter()
        .find(|region| region.cells.contains(&position))
        .unwrap_or_else(|| panic!("position {position:?} should belong to a floor region"))
}

fn visual_at(snapshot: &GameSnapshot, position: Position) -> CellVisualDto {
    *snapshot
        .visual_cells
        .iter()
        .find(|visual| visual.position == position)
        .expect("snapshot should contain every visual cell")
}

#[test]
fn built_in_game_is_created_from_the_compiled_content_pack() {
    let snapshot = Game::new(42).snapshot();
    let shard = snapshot
        .items
        .iter()
        .find(|item| item.id == "demo.item.luminous-shard.1")
        .expect("compiled world should spawn its item");

    assert_eq!(snapshot.content_id, "rfb.demo.original-v1");
    assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
    assert_eq!(snapshot.world_id, BUILT_IN_WORLD_ID);
    assert_eq!(
        snapshot.player.melee_damage.damage_type,
        DamageTypeDto::Physical
    );
    assert_eq!(
        snapshot.entities[0].melee_damage.damage_type,
        DamageTypeDto::Fire
    );
    assert_eq!(snapshot.player.id, "demo.actor.player.1");
    assert_eq!(snapshot.player.kind_id, "demo.actor.explorer");
    assert_eq!(snapshot.player.base_attack, 2);
    assert_eq!(snapshot.player.attack, 2);
    assert_eq!(snapshot.player.base_defense, 1);
    assert_eq!(snapshot.player.defense, 1);
    assert!(snapshot.inventory.is_empty());
    assert!(snapshot.equipment.is_empty());
    assert_eq!(snapshot.items.len(), 5);
    assert_eq!(snapshot.entities[0].position, Position { x: 8, y: 5 });
    assert_eq!(snapshot.entities[0].attack, 1);
    assert_eq!(snapshot.entities[0].defense, 1);
    assert_eq!(shard.position, Position { x: 4, y: 3 });
    assert_eq!(
        snapshot
            .cells
            .iter()
            .find(|cell| cell.position == shard.position)
            .and_then(|cell| cell.item_id.as_deref()),
        Some("demo.item.luminous-shard.1")
    );
    assert!(
        snapshot
            .content_visuals
            .iter()
            .any(|visual| visual.id == "demo.item.luminous-shard" && visual.glyph == "!")
    );
    assert_eq!(snapshot.visual_cells.len(), snapshot.cells.len());
    assert_eq!(
        visual_at(&snapshot, snapshot.player.position).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 19, y: 19 }).visibility,
        VisibilityState::Hidden
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 8, y: 5 }).light.color,
        ACTOR_LIGHT_COLOR
    );
}

#[test]
fn movement_produces_fov_deltas_and_remembers_explored_cells() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let first = game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("movement should execute");
    assert!(!first.changed_visual_cells.is_empty());
    let snapshot = game.snapshot();
    assert_eq!(
        visual_at(&snapshot, Position { x: 11, y: 3 }).visibility,
        VisibilityState::Visible
    );
    assert_eq!(
        visual_at(&snapshot, Position { x: 12, y: 3 }).visibility,
        VisibilityState::Hidden
    );

    for seq in 2..=7 {
        game.dispatch(command(
            seq,
            seq - 1,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("eastward exploration should execute");
    }
    assert_eq!(
        visual_at(&game.snapshot(), Position { x: 1, y: 3 }).visibility,
        VisibilityState::Remembered
    );
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
fn haste_and_slow_modify_scheduler_speed_without_changing_base_speed() {
    let mut haste_payload = Game::new(42).to_save();
    haste_payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 20,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut haste = Game::from_save(haste_payload).expect("haste setup should load");
    assert_eq!(haste.snapshot().player.speed, 120);
    let haste_update = haste
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("hasted wait should execute");
    assert_eq!(haste_update.world_tick, 5);
    assert_eq!(haste_update.player.speed, 120);
    assert_eq!(haste.to_save().player.base_speed, 110);
    assert_eq!(haste_update.player.statuses[0].remaining_ticks, 15);

    let mut slow_payload = Game::new(42).to_save();
    slow_payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_SLOW.to_owned(),
        intensity: 1,
        remaining_ticks: 40,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut slow = Game::from_save(slow_payload).expect("slow setup should load");
    let slow_update = slow
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("slowed wait should execute");
    assert_eq!(slow_update.world_tick, 20);
    assert_eq!(slow_update.player.speed, 100);
    assert_eq!(slow_update.player.statuses[0].remaining_ticks, 20);
}

#[test]
fn poison_uses_resistance_then_expires_and_round_trips() {
    let mut payload = Game::new(42).to_save();
    payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.actor.ember-mote.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    payload.player.resistances = vec![ResistanceSaveDto {
        damage_type: DamageTypeDto::Poison,
        level: ResistanceLevelDto::Resistant,
    }];
    let mut game = Game::from_save(payload).expect("poison setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("poisoned wait should execute");

    assert_eq!(update.player.hp, 7);
    assert!(update.player.statuses.is_empty());
    assert_eq!(update.player.resistances.len(), 1);
    assert_eq!(
        update
            .events
            .iter()
            .filter(|event| event.message_key == "status-player-damage")
            .count(),
        3
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "status-player-expired")
    );
    let restored = Game::from_save(game.to_save()).expect("status save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn bleeding_ticks_as_physical_damage_in_stable_status_order() {
    let mut payload = Game::new(42).to_save();
    payload.player.statuses = vec![
        StatusSaveDto {
            kind_id: STATUS_POISON.to_owned(),
            intensity: 1,
            remaining_ticks: 1,
            source_id: None,
            granted_resistances: Vec::new(),
            granted_brands: Vec::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: Vec::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        },
        StatusSaveDto {
            kind_id: STATUS_BLEEDING.to_owned(),
            intensity: 2,
            remaining_ticks: 2,
            source_id: None,
            granted_resistances: Vec::new(),
            granted_brands: Vec::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: Vec::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        },
    ];
    let mut game = Game::from_save(payload).expect("bleeding setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("bleeding wait should execute");

    assert_eq!(update.player.hp, 5);
    assert!(update.player.statuses.is_empty());
    let damage_statuses = update
        .events
        .iter()
        .filter(|event| event.message_key == "status-player-damage")
        .map(|event| event.args["status"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        damage_statuses,
        [STATUS_BLEEDING, STATUS_POISON, STATUS_BLEEDING]
    );
}

#[test]
fn content_driven_fire_melee_uses_the_player_resistance_profile() {
    let (seed, normal_damage) = (0_u64..1_000)
        .find_map(|seed| {
            let mut game = Game::new(42);
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            game.resolve_monster_melee(0, &mut events);
            events.into_iter().find_map(|event| match event {
                DomainEvent::MonsterMeleeHit { damage, .. } if damage.applied >= 2 => {
                    Some((seed, damage.applied))
                }
                _ => None,
            })
        })
        .expect("a deterministic seed should produce a fire hit of at least two damage");

    let mut resistant = Game::new(42);
    resistant.player.resistances.set(
        DamageType::Fire,
        crate::resistance::ResistanceLevel::Resistant,
    );
    resistant.rng = RfbRng::seeded(seed);
    let mut events = Vec::new();
    resistant.resolve_monster_melee(0, &mut events);
    let resisted_damage = events
        .into_iter()
        .find_map(|event| match event {
            DomainEvent::MonsterMeleeHit { damage, .. } => Some(damage.applied),
            _ => None,
        })
        .expect("the same seed should preserve the hit result");

    assert_eq!(resisted_damage, normal_damage - normal_damage / 2);
    assert_eq!(resistant.player.hp, 10 - resisted_damage);
}

#[test]
fn content_driven_monster_routine_resolves_blows_in_declared_order() {
    let mut game = Game::new(0);
    game.entities[0].kind_id = "demo.actor.echo-hound".to_owned();
    let routine = game.snapshot().entities[0].melee_routine.clone();

    assert_eq!(routine.blows.len(), 2);
    assert_eq!(routine.blows[0].method_id, "rfb.blow.echo-bite");
    assert_eq!(routine.blows[1].method_id, "rfb.blow.echo-rake");

    let mut events = Vec::new();
    game.resolve_monster_melee(0, &mut events);
    let projected = project_events(events);

    assert_eq!(projected.len(), 2);
    assert_eq!(projected[0].args["method"], "rfb.blow.echo-bite");
    assert_eq!(projected[1].args["method"], "rfb.blow.echo-rake");
}

#[test]
fn lethal_monster_status_removes_the_entity_before_energy_actions() {
    let mut payload = Game::new(42).to_save();
    payload.entities[0].statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some("demo.player.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut game = Game::from_save(payload).expect("monster poison setup should load");
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("wait should process monster poison");

    assert_eq!(update.entities.len(), 1);
    assert_eq!(
        update.entities[0].id,
        "demo.z-entrance-guardian.resonance-descent.1"
    );
    assert_eq!(update.removed_entities, ["demo.monster.ember-mote.1"]);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "status-entity-death")
    );
}

#[test]
fn leader_death_dissolves_pack_before_remaining_members_act() {
    let mut payload = Game::new(42).to_save();
    let leader_id = payload.entities[0].id.clone();
    let pack_id = "test.pack.leader-death".to_owned();
    payload.entities[0].statuses = vec![StatusSaveDto {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some("demo.player.1".to_owned()),
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    payload.entities[0].pack = Some(rfb_protocol::MonsterPackSaveDto {
        id: pack_id.clone(),
        leader_id: leader_id.clone(),
        role: MonsterPackRoleDto::Leader,
        behavior: MonsterPackBehaviorDto::Seek,
    });
    let mut member = payload.entities[0].clone();
    member.id = "test.pack.member".to_owned();
    member.position = Position { x: 8, y: 6 };
    member.statuses.clear();
    member.pack = Some(rfb_protocol::MonsterPackSaveDto {
        id: pack_id,
        leader_id,
        role: MonsterPackRoleDto::Member,
        behavior: MonsterPackBehaviorDto::GuardLeader,
    });
    payload.entities.push(member);

    let mut game = Game::from_save(payload).expect("pack death setup should load");
    game.dispatch(command(1, 0, GameCommand::Wait))
        .expect("leader death should resolve");

    assert_eq!(game.entities.len(), 2);
    let member = game
        .entities
        .iter()
        .find(|entity| entity.id == "test.pack.member")
        .expect("pack member should remain");
    assert!(member.pack.is_none());
    Game::from_save(game.to_save()).expect("dissolved pack should remain saveable");
}

#[test]
fn content_driven_loot_generation_is_deterministic_and_persistent() {
    let mut left = Game::new(42);
    let initial = left.to_save();
    assert_eq!(initial.carried_items.len(), 1);
    assert_eq!(initial.carried_items[0].id, "generated.item.1");
    assert_eq!(
        initial.carried_items[0].actor_id,
        "demo.monster.ember-mote.1"
    );
    assert_eq!(initial.carried_items[0].kind_id, "demo.item.echo-charm");
    assert_eq!(left.snapshot().items.len(), 5);
    assert_eq!(left.rng.draw_counter, 3);
    left.entities[0].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(left.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut right = left.clone();
    let death_position = left.entities[0].position;

    let left_update = left
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("loot-bearing monster death should execute");
    let right_update = right
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("same loot context should execute");

    assert_eq!(left_update.state_hash, right_update.state_hash);
    assert_eq!(left.rng.draw_counter, 6);
    assert_eq!(left.rng.draw_counter, right.rng.draw_counter);
    let drops = left_update
        .events
        .iter()
        .filter(|event| event.message_key == "loot-drop")
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), 2);
    assert_eq!(drops[0].args["target"], "demo.item.echo-charm");
    assert_eq!(drops[1].args["source"], "demo.actor.ember-mote");
    let carried = left
        .items
        .iter()
        .find(|item| item.id == "generated.item.1")
        .expect("carried loot should preserve its stable item ID");
    assert_eq!(carried.location, ItemLocation::Ground(death_position));
    assert_eq!(carried.kind_id, "demo.item.echo-charm");
    let generated = left
        .items
        .iter()
        .find(|item| item.id == "generated.item.2")
        .expect("death loot should allocate the next stable item ID");
    assert_eq!(generated.location, ItemLocation::Ground(death_position));
    assert_eq!(generated.quantity, 1);
    assert_eq!(generated.kind_id, "demo.item.echo-charm");
    assert_eq!(generated.quality, ItemQualityDto::Ordinary);
    assert!(generated.affix_ids.is_empty());
    let restored = Game::from_save(left.to_save()).expect("generated loot should reload");
    assert_eq!(restored.state_hash(), left.state_hash());
}

#[test]
fn carried_item_save_rejects_a_missing_monster_owner() {
    let mut payload = Game::new(42).to_save();
    payload.carried_items[0].actor_id = "demo.monster.missing".to_owned();

    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("carried item state is invalid"))
    ));
}

#[test]
fn previous_built_in_content_hash_migrates_without_spawning_new_items() {
    for previous_hash in PREVIOUS_BUILT_IN_CONTENT_HASHES {
        let mut payload = Game::new(42).to_save();
        payload.content_hash = previous_hash.to_owned();
        payload.carried_items.clear();
        payload.items.retain(|item| {
            item.kind_id != "demo.item.echo-charm"
                && item.kind_id != "demo.item.echo-blade"
                && item.kind_id != "demo.item.resonance-sling"
                && item.kind_id != "demo.item.resonance-pellet"
        });

        let restored = Game::from_save(payload).expect("known previous content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items.iter().all(|item| {
            item.kind_id != "demo.item.echo-charm"
                && item.kind_id != "demo.item.echo-blade"
                && item.kind_id != "demo.item.resonance-sling"
                && item.kind_id != "demo.item.resonance-pellet"
        }));
    }
}

#[test]
fn previous_task_state_set_adds_new_tasks_as_available() {
    let mut current_payload = Game::new(42).to_save();
    current_payload
        .task_states
        .retain(|state| state.task_id != "demo.task.echo-chain");
    assert!(matches!(
        Game::from_save(current_payload),
        Err(CoreError::InvalidSave("task state set is incomplete"))
    ));

    let mut payload = Game::new(42).to_save();
    payload.content_hash =
        "b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43".to_owned();
    payload
        .task_states
        .retain(|state| state.task_id != "demo.task.echo-chain");

    let restored = Game::from_save(payload).expect("v44 task state set should migrate");
    let chain = restored
        .snapshot()
        .tasks
        .into_iter()
        .find(|task| task.task_id == "demo.task.echo-chain")
        .expect("new staged task should be added during migration");
    assert_eq!(chain.status, TaskStatusKindDto::Available);
    assert_eq!((chain.stage, chain.stages), (1, 3));
    assert_eq!((chain.current, chain.required), (0, 1));
}

#[test]
fn paused_task_can_be_abandoned_from_the_surface() {
    let mut game = Game::new(27);
    let entry = Position { x: 4, y: 4 };
    game.player.position = entry;
    game.traverse_stairs(false)
        .expect("bounty entry should resolve")
        .expect("bounty entry should transition");
    game.traverse_stairs(false)
        .expect("bounty pause should resolve")
        .expect("bounty pause should return to the surface");

    let paused = game
        .snapshot()
        .tasks
        .into_iter()
        .find(|task| task.task_id == "demo.task.echo-bounty")
        .expect("bounty task should be projected");
    assert_eq!(paused.status, TaskStatusKindDto::Paused);
    assert_eq!((paused.retakes_used, paused.max_retakes), (0, Some(1)));
    assert!(
        game.stored_floors
            .contains_key("demo.floor.echo-bounty-rift")
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::AbandonPausedTask {
            task_id: "demo.task.echo-bounty".to_owned(),
        },
    );
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(
        game.task_states["demo.task.echo-bounty"].status,
        TaskStatusKindDto::Abandoned
    );
    assert!(
        !game
            .stored_floors
            .contains_key("demo.floor.echo-bounty-rift")
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.bounty-rift-abandoned");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.abandoned")
    );
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == entry
                && cell.terrain_id == "demo.terrain.bounty-rift-abandoned")
    );
    Game::from_save(game.to_save()).expect("surface abandonment should round-trip");
}

#[test]
fn regenerated_retake_preserves_progress_and_enforces_the_limit() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 4, y: 4 };
    game.traverse_stairs(false)
        .expect("bounty entry should resolve")
        .expect("bounty entry should transition");
    assert_eq!(game.entities.len(), 2);
    game.entities.pop();
    game.task_states
        .get_mut("demo.task.echo-bounty")
        .expect("bounty task should exist")
        .current = 1;
    game.traverse_stairs(false)
        .expect("partial bounty pause should resolve")
        .expect("partial bounty should return to the surface");
    assert_eq!(
        game.stored_floors["demo.floor.echo-bounty-rift"]
            .entities
            .len(),
        1
    );
    let draws_before_retake = game.rng.draw_counter;

    game.traverse_stairs(false)
        .expect("first bounty retake should resolve")
        .expect("first bounty retake should regenerate the floor");
    assert!(game.rng.draw_counter > draws_before_retake);
    assert_eq!(game.entities.len(), 1);
    let active = &game.task_states["demo.task.echo-bounty"];
    assert_eq!(active.status, TaskStatusKindDto::Active);
    assert_eq!(
        (active.current, active.required, active.retakes_used),
        (1, 2, 1)
    );

    game.traverse_stairs(false)
        .expect("second bounty pause should resolve")
        .expect("second bounty pause should return to the surface");
    let draws_before_rejected_retake = game.rng.draw_counter;
    assert!(
        game.traverse_stairs(false)
            .expect("exhausted bounty entry should resolve")
            .is_none()
    );
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.rng.draw_counter, draws_before_rejected_retake);

    let mut invalid = game.to_save();
    invalid
        .task_states
        .iter_mut()
        .find(|state| state.task_id == "demo.task.echo-bounty")
        .expect("bounty save state should exist")
        .retakes_used = 2;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("task state is invalid"))
    ));
}

#[test]
fn v60_task_state_defaults_to_zero_retakes_without_rng_drift() {
    let mut payload = Game::new(27).to_save();
    payload.content_hash =
        "9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02".to_owned();
    let saved_draws = payload.rng.draw_counter;
    let restored = Game::from_save(payload).expect("v60 task state should migrate");

    assert_eq!(restored.rng.draw_counter, saved_draws);
    assert!(
        restored
            .snapshot()
            .tasks
            .iter()
            .all(|task| task.retakes_used == 0)
    );
}

#[test]
fn dungeon_guardian_state_migrates_and_rejects_entity_mismatch() {
    let mut old_payload = Game::new(42).to_save();
    old_payload.content_hash =
        "0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a".to_owned();
    old_payload.dungeon_states.clear();
    let restored = Game::from_save(old_payload).expect("v45 save should add dungeon state");
    assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

    let mut v48_payload = Game::new(42).to_save();
    v48_payload.content_hash =
        "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad".to_owned();
    v48_payload
        .dungeon_states
        .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
    let restored =
        Game::from_save(v48_payload).expect("v48 save should add the pressure dungeon state");
    assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

    let mut current_payload = Game::new(42).to_save();
    current_payload
        .dungeon_states
        .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
    assert!(matches!(
        Game::from_save(current_payload),
        Err(CoreError::InvalidSave("dungeon state set is incomplete"))
    ));

    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    descend_one_floor(&mut game);
    descend_one_floor(&mut game);
    assert!(
        game.content
            .world(&game.world_id)
            .expect("world should remain available")
            .procedural_floors
            .iter()
            .any(|floor| floor.id == game.current_floor_id && floor.final_floor)
    );
    let mut payload = game.to_save();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.echo-depths")
        .expect("echo dungeon state should exist")
        .guardian_defeated = true;
    let result = Game::from_save(payload);
    assert!(
        matches!(
            result,
            Err(CoreError::InvalidSave("dungeon guardian state is invalid"))
        ),
        "unexpected guardian mismatch result: {result:?}"
    );
}

#[test]
fn guardian_mirrors_share_conquest_and_are_removed_from_other_final_floors() {
    let mut game = Game::new(71);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");

    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-2.down-b");
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "demo.guardian.echo-depths.2")
    );
    traverse_connection(&mut game, "demo.connection.echo-depth-3-mirror.up-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-2.up-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-b");
    traverse_connection(&mut game, "demo.connection.echo-depth-2-mirror.down-a");

    assert!(
        stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .entities
            .iter()
            .any(|actor| actor.id == "demo.guardian.echo-depths.2")
    );
    let guardian_index = game
        .entities
        .iter()
        .position(|actor| actor.id == "demo.guardian.echo-depths.3")
        .expect("branch final floor should contain its guardian mirror");
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed_entities = Vec::new();
    game.resolve_actor_death(
        guardian_index,
        DomainEvent::Waited,
        &mut events,
        &mut changed,
        &mut removed_entities,
    )
    .expect("guardian mirror death should resolve");

    assert!(game.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::DungeonGuardianDefeated { .. }))
            .count(),
        1
    );
    assert_eq!(removed_entities, ["demo.guardian.echo-depths.3"]);
    assert!(
        stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.echo-depths.2")
    );
    assert!(stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .items
            .iter()
            .all(|item| {
                !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == "demo.guardian.echo-depths.2")
            }));

    let mut restored = Game::from_save(game.to_save()).expect("shared conquest should persist");
    traverse_connection(&mut restored, "demo.connection.echo-depth-3-branch.up-a");
    traverse_connection(&mut restored, "demo.connection.echo-depth-2-mirror.up-a");
    traverse_connection(&mut restored, "demo.connection.echo-depth-1.shaft-down");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-3-shaft");
    assert!(
        restored
            .entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.echo-depths.4")
    );
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
fn previous_equipment_content_migrates_to_derived_modifiers() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
        },
    ))
    .expect("equip should execute");
    let mut payload = game.to_save();
    payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[1].to_owned();
    payload.carried_items.clear();
    payload.player.base_max_hp = 0;
    payload.next_item_instance_serial = 0;

    let restored = Game::from_save(payload).expect("known 1.1 content should migrate");
    let snapshot = restored.snapshot();
    assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
    assert_eq!(snapshot.player.base_max_hp, 10);
    assert_eq!(snapshot.player.max_hp, 14);
    assert_eq!(snapshot.player.attack, 4);
    assert_eq!(snapshot.player.defense, 2);
    assert_eq!(snapshot.player.equipment_modifiers.attack, 2);
    assert_eq!(snapshot.player.equipment_modifiers.defense, 1);
    assert_eq!(snapshot.player.equipment_modifiers.max_hp, 4);
    assert_eq!(restored.next_item_instance_serial, 1);
}

#[test]
fn ring_slots_fill_in_body_order_and_replace_deterministically() {
    let mut game = Game::new(42);
    for ordinal in 1..=3 {
        game.items.push(ItemInstance {
            id: format!("test.item.band.{ordinal}"),
            kind_id: "demo.item.resonant-band".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Inventory,
        });
    }
    let slot_of = |game: &Game, id: &str| {
        game.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| match &item.location {
                ItemLocation::Equipped { slot_id } => slot_id.clone(),
                _ => "unequipped".to_owned(),
            })
            .expect("test band should exist")
    };

    game.dispatch(command(
        1,
        0,
        GameCommand::Equip {
            item_id: "test.item.band.1".to_owned(),
        },
    ))
    .expect("first ring should equip");
    game.dispatch(command(
        2,
        1,
        GameCommand::Equip {
            item_id: "test.item.band.2".to_owned(),
        },
    ))
    .expect("second ring should equip");
    assert_eq!(slot_of(&game, "test.item.band.1"), "ring-1");
    assert_eq!(slot_of(&game, "test.item.band.2"), "ring-2");
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.equipment_modifiers.defense, 2);
    assert_eq!(snapshot.body_slots.len(), 13);
    assert!(
        snapshot
            .body_slots
            .iter()
            .any(|slot| slot.id == "light" && slot.slot_type == "light")
    );

    // All ring instances occupied: the next equip replaces the first
    // instance in body order, returning its occupant to the inventory.
    game.dispatch(command(
        3,
        2,
        GameCommand::Equip {
            item_id: "test.item.band.3".to_owned(),
        },
    ))
    .expect("third ring should replace the first instance");
    assert_eq!(slot_of(&game, "test.item.band.3"), "ring-1");
    assert_eq!(slot_of(&game, "test.item.band.1"), "unequipped");
    assert_eq!(slot_of(&game, "test.item.band.2"), "ring-2");

    let restored = Game::from_save(game.to_save()).expect("body slots should round trip");
    assert_eq!(restored.body_slots.len(), 13);
    assert_eq!(slot_of(&restored, "test.item.band.3"), "ring-1");
}

#[test]
fn previous_combat_content_migrates_to_current_actor_stats() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
        },
    ))
    .expect("equip should execute");
    let mut payload = game.to_save();
    payload.content_hash = PREVIOUS_BUILT_IN_CONTENT_HASHES[2].to_owned();

    let restored = Game::from_save(payload).expect("known 1.2 content should migrate");
    let snapshot = restored.snapshot();
    assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
    assert_eq!(snapshot.player.base_attack, 2);
    assert_eq!(snapshot.player.attack, 4);
    assert_eq!(snapshot.player.base_defense, 1);
    assert_eq!(snapshot.player.defense, 2);
    assert_eq!(snapshot.entities[0].attack, 1);
    assert_eq!(snapshot.entities[0].defense, 1);
}

#[test]
fn fixed_seed_and_commands_are_deterministic() {
    let mut left = Game::new(42);
    let mut right = Game::new(42);
    let commands = [
        GameCommand::Move {
            direction: Direction::East,
        },
        GameCommand::Move {
            direction: Direction::South,
        },
        GameCommand::Wait,
    ];

    for (index, game_command) in commands.into_iter().enumerate() {
        let seq = index as u32 + 1;
        let revision = index as u32;
        left.dispatch(command(seq, revision, game_command.clone()))
            .expect("left command should execute");
        right
            .dispatch(command(seq, revision, game_command))
            .expect("right command should execute");
    }

    assert_eq!(left.state_hash(), right.state_hash());
}

#[test]
fn normal_speed_monster_tracks_once_per_player_action() {
    let mut game = Game::new(42);
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("wait should advance the scheduler");

    assert_eq!(update.world_tick, 10);
    assert_eq!(update.player.energy_need, 0);
    assert_eq!(update.entities[0].position, Position { x: 7, y: 4 });
    assert_eq!(update.entities[0].energy_need, STANDARD_ACTION_COST);
    assert_eq!(update.changed_cells.len(), 2);
}

#[test]
fn fast_and_slow_monsters_use_the_same_energy_scheduler() {
    let mut fast = Game::new(42);
    fast.entities[0].speed = 120;
    let fast_update = fast
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("fast scheduler case should execute");
    assert_eq!(fast_update.world_tick, 10);
    assert_eq!(fast_update.entities[0].position, Position { x: 6, y: 3 });
    assert_eq!(fast_update.entities[0].energy_need, STANDARD_ACTION_COST);

    let mut slow = Game::new(42);
    slow.entities[0].speed = 100;
    let first = slow
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("first slow scheduler case should execute");
    assert_eq!(first.entities[0].position, Position { x: 8, y: 5 });
    assert_eq!(first.entities[0].energy_need, 50);
    let second = slow
        .dispatch(command(2, 1, GameCommand::Wait))
        .expect("second slow scheduler case should execute");
    assert_eq!(second.entities[0].position, Position { x: 7, y: 4 });
    assert_eq!(second.entities[0].energy_need, STANDARD_ACTION_COST);
}

#[test]
fn multiple_monsters_use_stable_id_order_when_paths_compete() {
    let mut left = Game::new(42);
    let mut second = left.entities[0].clone();
    second.id = "demo.monster.ember-mote.0".to_owned();
    second.position = Position { x: 8, y: 6 };
    left.entities.push(second);

    let mut right = left.clone();
    right.entities.reverse();

    let left_update = left
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("left scheduler should execute");
    let right_update = right
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("right scheduler should execute");

    assert_eq!(left_update.entities, right_update.entities);
    assert_eq!(left_update.changed_cells, right_update.changed_cells);
    assert_eq!(left_update.state_hash, right_update.state_hash);
    assert_ne!(
        left_update.entities[0].position,
        left_update.entities[1].position
    );
}

#[test]
fn player_death_stops_the_remaining_monster_queue_immediately() {
    let mut game = Game::new(0);
    game.entities[0].id = "demo.monster.ember-mote.0".to_owned();
    game.entities[0].position = Position { x: 4, y: 3 };
    let mut second = game.entities[0].clone();
    second.id = "demo.monster.ember-mote.1".to_owned();
    second.position = Position { x: 4, y: 4 };
    game.entities.push(second);
    game.player.hp = 0;

    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("fatal scheduler case should execute");

    assert!(update.player.is_dead);
    assert_eq!(
        update
            .events
            .iter()
            .filter(|event| event.message_key == "combat-player-death")
            .count(),
        1
    );
    let second = update
        .entities
        .iter()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("second monster should remain present");
    assert_eq!(second.energy_need, 10);
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
        },
    ))
    .expect("equip should execute");

    let restored = Game::from_save(game.to_save()).expect("save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.snapshot().equipment.len(), 1);
}

#[test]
fn pickup_moves_the_ground_stack_into_inventory() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.dispatch(command(
        1,
        0,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("move should execute");
    let update = game
        .dispatch(command(2, 1, GameCommand::PickUp))
        .expect("pickup should execute");

    assert_eq!(update.items.len(), 4);
    assert_eq!(update.inventory.len(), 1);
    assert_eq!(update.inventory[0].id, "demo.item.luminous-shard.1");
    assert_eq!(update.inventory[0].quantity, 5);
    assert_eq!(update.player.carried_weight_tenths_pound, 50);
    assert_eq!(update.player.carry_capacity_tenths_pound, 100);
    assert_eq!(update.changed_cells.len(), 1);
    assert_eq!(update.changed_cells[0].position, Position { x: 4, y: 3 });
    assert_eq!(update.changed_cells[0].item_id, None);
    assert_eq!(update.events[0].message_key, "item-pickup-success");
}

#[test]
fn pickup_over_capacity_rejects_the_whole_ground_stack() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.player.position = Position { x: 6, y: 4 };
    for kind_id in [
        "demo.item.luminous-shard",
        "demo.item.echo-charm",
        "demo.item.echo-blade",
        "demo.item.resonance-sling",
    ] {
        game.items
            .iter_mut()
            .find(|item| item.kind_id == kind_id)
            .expect("carried fixture item should exist")
            .location = ItemLocation::Inventory;
    }
    assert_eq!(game.carried_weight_tenths_pound(), 100);

    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("over-capacity pickup should resolve as an action");

    let event = &update.events[0];
    assert_eq!(event.kind, "item.pickup.over-capacity");
    assert_eq!(event.args["target"], "demo.item.resonance-pellet");
    assert_eq!(event.args["quantity"], "6");
    assert_eq!(event.args["currentWeight"], "100");
    assert_eq!(event.args["pickupWeight"], "12");
    assert_eq!(event.args["capacity"], "100");
    assert_eq!(update.player.carried_weight_tenths_pound, 100);
    assert!(update.items.iter().any(|item| {
        item.id == "demo.item.resonance-pellet.1"
            && item.quantity == 6
            && item.position == Position { x: 6, y: 4 }
    }));
}

#[test]
fn themed_vault_paints_template_and_spawns_depth_eligible_group_and_loot() {
    let game = (1..=64)
        .find_map(|seed| {
            let mut game = Game::new(seed);
            descend_one_floor(&mut game);
            descend_one_floor(&mut game);
            (game.current_floor_id == "demo.floor.echo-depth-2"
                && game
                    .entities
                    .iter()
                    .any(|entity| entity.id.contains("harmonic-sepulcher-sentinels")))
            .then_some(game)
        })
        .expect("a harmonic sepulcher seed should remain reachable");

    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-2");
    assert_eq!(game.floor_connections.len(), 3);
    assert_eq!(game.floor_regions.len(), 2);
    assert_eq!(game.entities.len(), 5);
    let regional_encounters = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".encounter.plain."))
        .collect::<Vec<_>>();
    assert_eq!(regional_encounters.len(), 2);
    assert!(regional_encounters.iter().all(|entity| matches!(
        entity.kind_id.as_str(),
        "demo.actor.echo-hound"
            | "demo.actor.storm-spark"
            | "demo.actor.acid-seep"
            | "demo.actor.venom-spore"
    )));
    let vault_members = game
        .entities
        .iter()
        .filter(|entity| {
            entity.id.starts_with(
                "demo.floor.echo-depth-2.demo.vault-group.harmonic-sepulcher-sentinels.",
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(vault_members.len(), 3);
    assert!(vault_members.iter().all(|entity| {
        matches!(
            entity.kind_id.as_str(),
            "demo.actor.frost-wisp" | "demo.actor.storm-spark" | "demo.actor.venom-spore"
        )
    }));

    let first_member = vault_members
        .iter()
        .find(|entity| entity.id.ends_with(".1"))
        .expect("vault should contain its first group member");
    let vault_origin = Position {
        x: first_member.position.x - 1,
        y: first_member.position.y - 1,
    };
    let vault_region_id = region_at(&game, first_member.position).region_id.clone();
    for y in vault_origin.y..vault_origin.y + 5 {
        for x in vault_origin.x..vault_origin.x + 6 {
            assert_eq!(
                region_at(&game, Position { x, y }).region_id,
                vault_region_id
            );
        }
    }
    assert!(regional_encounters.iter().all(|entity| {
        match region_at(&game, entity.position).region_id.as_str() {
            "demo.region.resonance-grotto" => matches!(
                entity.kind_id.as_str(),
                "demo.actor.acid-seep" | "demo.actor.venom-spore"
            ),
            "demo.region.resonance-gallery" => matches!(
                entity.kind_id.as_str(),
                "demo.actor.echo-hound" | "demo.actor.storm-spark"
            ),
            _ => false,
        }
    }));
    assert_eq!(
        game.terrain_at(Position {
            x: vault_origin.x + 3,
            y: vault_origin.y,
        }),
        "demo.terrain.door-secret"
    );
    assert_eq!(game.terrain_at(vault_origin), "demo.terrain.wall");
    assert!(game.items.iter().any(|item| {
        item.location
            == ItemLocation::Ground(Position {
                x: vault_origin.x + 2,
                y: vault_origin.y + 3,
            })
            && matches!(
                item.kind_id.as_str(),
                "demo.item.echo-blade" | "demo.item.echo-charm"
            )
    }));
    assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
    let mut instance_ids = BTreeSet::from([game.player.id.clone()]);
    instance_ids.extend(game.entities.iter().map(|entity| entity.id.clone()));
    for item in &game.items {
        assert!(
            instance_ids.insert(item.id.clone()),
            "duplicate item ID: {}",
            item.id
        );
        let definition = game
            .content
            .item(&item.kind_id)
            .expect("generated item kind must remain available");
        assert!(item.quantity <= definition.max_stack);
        if let ItemLocation::Ground(position) = item.location {
            assert!(
                game.is_walkable(position),
                "item {} is on non-walkable {} at {position:?}",
                item.id,
                game.terrain_at(position)
            );
        }
    }

    let restored = Game::from_save(game.to_save()).expect("vault floor save should restore");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn weighted_vault_candidates_are_deterministic_and_both_reachable() {
    let mut harmonic = 0;
    let mut resonant = 0;
    for seed in 1..=64 {
        let mut left = Game::new(seed);
        let mut right = Game::new(seed);
        for game in [&mut left, &mut right] {
            descend_one_floor(game);
            descend_one_floor(game);
        }
        assert_eq!(left.state_hash(), right.state_hash());
        if left
            .entities
            .iter()
            .any(|entity| entity.id.contains("harmonic-sepulcher-sentinels"))
        {
            harmonic += 1;
        } else if left
            .entities
            .iter()
            .any(|entity| entity.id.contains("resonant-gallery-chorus"))
        {
            resonant += 1;
        } else {
            panic!("depth two must select one eligible themed vault");
        }
    }
    assert!(harmonic > resonant);
    assert!(resonant > 0);
}

#[test]
fn regional_themes_are_weighted_deterministic_and_keep_local_content_in_bounds() {
    let mut grotto_entry_count = 0;
    let mut gallery_entry_count = 0;
    for seed in 1..=64 {
        let mut left = Game::new(seed);
        left.player.position = Position { x: 3, y: 2 };
        left.traverse_stairs(false)
            .expect("resonance entry should resolve")
            .expect("resonance entry should transition");
        descend_one_floor(&mut left);

        let mut right = Game::new(seed);
        right.player.position = Position { x: 3, y: 2 };
        right
            .traverse_stairs(false)
            .expect("matching resonance entry should resolve")
            .expect("matching resonance entry should transition");
        descend_one_floor(&mut right);

        assert_eq!(left.current_floor_id, "demo.floor.resonance-depth-2");
        assert_eq!(left.floor_regions, right.floor_regions);
        assert_eq!(left.state_hash(), right.state_hash());
        assert_eq!(left.floor_regions.len(), 2);
        assert_eq!(left.entities.len(), 4);
        assert_eq!(left.items.len(), 2);

        let entry_region = left
            .floor_regions
            .iter()
            .find(|region| region.cells.contains(&left.player.position))
            .expect("entry room must belong to one region");
        match entry_region.region_id.as_str() {
            "demo.region.resonance-grotto" => grotto_entry_count += 1,
            "demo.region.resonance-gallery" => gallery_entry_count += 1,
            _ => panic!("unexpected generated region"),
        }

        let mut all_cells = BTreeSet::new();
        for region in &left.floor_regions {
            assert_eq!(region.cells.len(), 30);
            assert!(
                region
                    .cells
                    .iter()
                    .all(|position| all_cells.insert(*position))
            );
            let expected_terrain = match region.region_id.as_str() {
                "demo.region.resonance-grotto" => "demo.terrain.resonance-cavern",
                "demo.region.resonance-gallery" => "demo.terrain.resonant-floor",
                _ => panic!("unexpected generated region"),
            };
            assert!(
                region
                    .cells
                    .iter()
                    .any(|position| left.terrain_at(*position) == expected_terrain)
            );
        }
        assert!(left.terrain.iter().enumerate().any(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(left.width)).unwrap_or_default(),
                y: i32::try_from(index / usize::from(left.width)).unwrap_or_default(),
            };
            terrain_id == "demo.terrain.floor" && !all_cells.contains(&position)
        }));

        for entity in &left.entities {
            let region = left
                .floor_regions
                .iter()
                .find(|region| region.cells.contains(&entity.position))
                .expect("regional actor must remain inside its assigned region");
            assert!(match region.region_id.as_str() {
                "demo.region.resonance-grotto" => matches!(
                    entity.kind_id.as_str(),
                    "demo.actor.acid-seep" | "demo.actor.venom-spore"
                ),
                "demo.region.resonance-gallery" => matches!(
                    entity.kind_id.as_str(),
                    "demo.actor.echo-hound" | "demo.actor.storm-spark"
                ),
                _ => false,
            });
        }
        for item in &left.items {
            let ItemLocation::Ground(position) = item.location else {
                panic!("regional floor loot must be placed on the ground");
            };
            let region = left
                .floor_regions
                .iter()
                .find(|region| region.cells.contains(&position))
                .expect("regional loot must remain inside its assigned region");
            assert_eq!(
                item.kind_id,
                match region.region_id.as_str() {
                    "demo.region.resonance-grotto" => "demo.item.luminous-shard",
                    "demo.region.resonance-gallery" => "demo.item.resonance-pellet",
                    _ => panic!("unexpected generated region"),
                }
            );
        }
    }
    assert!(grotto_entry_count > gallery_entry_count);
    assert!(gallery_entry_count > 0);
}

#[test]
fn floor_regions_round_trip_reject_overlap_and_v59_missing_state_stays_empty() {
    let mut game = Game::new(17);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("resonance entry should resolve")
        .expect("resonance entry should transition");
    descend_one_floor(&mut game);

    let payload = game.to_save();
    assert_eq!(payload.floor_regions.len(), 2);
    let restored = Game::from_save(payload.clone()).expect("region state should restore");
    assert_eq!(restored.floor_regions, game.floor_regions);
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut overlap = payload.clone();
    let duplicate = overlap.floor_regions[0].cells[0];
    overlap.floor_regions[1].cells.push(duplicate);
    assert!(matches!(
        Game::from_save(overlap),
        Err(CoreError::InvalidSave("floor region state is invalid"))
    ));

    let mut legacy = payload;
    legacy.content_hash =
        "4cdcad204a7ccad6d67b8dcb50ccdcc188220a72d258c37219974fad51e5274d".to_owned();
    legacy.floor_regions.clear();
    let draw_counter = legacy.rng.draw_counter;
    let legacy_entities = legacy.entities.clone();
    let legacy_items = legacy.items.clone();
    let restored = Game::from_save(legacy).expect("v59 regionless floor should remain loadable");
    assert!(restored.floor_regions.is_empty());
    assert_eq!(restored.rng.draw_counter, draw_counter);
    assert_eq!(actors_to_save(&restored.entities), legacy_entities);
    assert_eq!(items_to_save(&restored.items), legacy_items);
}

#[test]
fn generation_budgets_scale_across_the_ten_depth_pressure_dungeon() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");

    let actor_slots = [2_usize, 4, 4, 5, 6, 7, 8, 9, 1, 30];
    let loot_placements = [1_usize, 2, 1, 1, 2, 2, 2, 4, 3, 3];
    let feature_placements = [0_usize, 0, 2, 3, 4, 4, 4, 4, 0, 4];
    for depth in 1..=10 {
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.resonance-depth-{depth}")
        );
        assert_eq!(
            game.entities.len(),
            actor_slots[depth - 1],
            "depth {depth} actor budget"
        );
        assert_eq!(
            game.items.len(),
            loot_placements[depth - 1],
            "depth {depth} loot budget"
        );
        let terrain_feature_tiles = game
            .terrain
            .iter()
            .filter(|terrain| {
                matches!(
                    terrain.as_str(),
                    "demo.terrain.trap-echo-snare"
                        | "demo.terrain.echo-rubble"
                        | "demo.terrain.door-locked"
                        | "demo.terrain.door-secret"
                )
            })
            .count();
        let mandatory_feature_tiles = if depth == 9 {
            1
        } else {
            2 + usize::from(depth == 8) * 5 + usize::from(depth == 10)
        };
        assert_eq!(
            terrain_feature_tiles - mandatory_feature_tiles,
            feature_placements[depth - 1]
        );
        if depth == 4 {
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.door-locked")
            );
        }
        let guardian_slots = if depth == 10 { 1 } else { 0 };
        let vault_slots = if depth == 8 { 3 } else { 0 };
        let pit_slots = if depth == 10 { 25 } else { 0 };
        assert_eq!(
            game.entities
                .iter()
                .filter(|entity| entity.id.contains(".encounter."))
                .count(),
            actor_slots[depth - 1] - guardian_slots - vault_slots - pit_slots
        );
        if depth == 8 {
            assert_eq!(
                game.entities
                    .iter()
                    .filter(|entity| entity.id.contains(".vault."))
                    .count(),
                3
            );
            assert!(
                game.entities
                    .iter()
                    .any(|entity| { entity.id.contains("resonance-spindle-watch") })
            );
            assert!(
                game.entities
                    .iter()
                    .any(|entity| entity.id.contains("resonance-crossroads-watch"))
            );
            assert!(
                !game
                    .entities
                    .iter()
                    .any(|entity| entity.id.contains("sealed-resonance-monolith"))
            );
            assert_eq!(
                game.terrain
                    .iter()
                    .filter(|terrain| *terrain == "demo.terrain.door-secret")
                    .count(),
                6
            );
        }
        if depth == 10 {
            let pit = game
                .entities
                .iter()
                .filter(|entity| entity.id.contains(".pit."))
                .collect::<Vec<_>>();
            assert_eq!(pit.len(), 25);
            let xs = pit
                .iter()
                .map(|entity| entity.position.x)
                .collect::<BTreeSet<_>>();
            let ys = pit
                .iter()
                .map(|entity| entity.position.y)
                .collect::<BTreeSet<_>>();
            assert_eq!(xs.len(), 5);
            assert_eq!(ys.len(), 5);
            let center = Position {
                x: (*xs.first().expect("pit must have a left edge")
                    + *xs.last().expect("pit must have a right edge"))
                    / 2,
                y: (*ys.first().expect("pit must have a top edge")
                    + *ys.last().expect("pit must have a bottom edge"))
                    / 2,
            };
            let center_actor = pit
                .iter()
                .find(|entity| entity.position == center)
                .expect("pit must fill its center");
            let center_level = game
                .content
                .actor(&center_actor.kind_id)
                .expect("pit actor must remain available")
                .level;
            assert!(
                pit.iter()
                    .filter(|entity| {
                        xs.contains(&entity.position.x) && ys.contains(&entity.position.y)
                    })
                    .all(|entity| {
                        center_level
                            >= game
                                .content
                                .actor(&entity.kind_id)
                                .expect("pit actor must remain available")
                                .level
                    })
            );
            let inner_door = Position {
                x: *xs.first().expect("pit must have a left edge") - 1,
                y: center.y,
            };
            assert_eq!(
                game.terrain[generated_terrain_index(game.width, inner_door)],
                "demo.terrain.door-secret"
            );
        }
        if matches!(depth, 1 | 3) {
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.floor")
            );
            assert!(
                !game
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.resonant-floor")
            );
        } else if depth == 2 {
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.floor")
            );
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.resonant-floor")
            );
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.resonance-cavern")
            );
        } else {
            assert!(
                game.terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.resonant-floor")
            );
        }
        if depth < 10 {
            descend_one_floor(&mut game);
        }
    }
    assert!(
        game.entities
            .iter()
            .any(|entity| entity.id == "demo.guardian.resonance-descent.1")
    );
    assert_eq!(game.stored_floors.len(), 10);
    let restored =
        Game::from_save(game.to_save()).expect("pressure dungeon final floor should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn regional_vault_and_pit_composition_is_deterministic_and_persistent() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..8 {
        descend_one_floor(&mut game);
    }
    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-8");
    assert_eq!(game.floor_regions.len(), 2);
    assert_eq!(
        game.entities
            .iter()
            .filter(|entity| entity.id.contains(".vault."))
            .count(),
        3
    );
    assert!(game.entities.iter().all(|entity| {
        !entity.id.contains(".vault.") || !region_at(&game, entity.position).region_id.is_empty()
    }));
    assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
    let mut all_region_cells = BTreeSet::new();
    for region in &game.floor_regions {
        assert!(
            region
                .cells
                .iter()
                .all(|cell| all_region_cells.insert(*cell))
        );
    }
    let depth_eight_hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("regional Vault floor should restore");
    assert_eq!(restored.state_hash(), depth_eight_hash);

    descend_one_floor(&mut game);
    descend_one_floor(&mut game);
    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-10");
    for terrain_id in [
        "demo.terrain.resonance-cavern",
        "demo.terrain.resonance-water-deep",
        "demo.terrain.resonance-water-shallow",
        "demo.terrain.resonance-ruin",
        "demo.terrain.resonance-vein",
    ] {
        assert!(
            game.terrain.iter().any(|candidate| candidate == terrain_id),
            "depth ten should contain {terrain_id}"
        );
    }
    let pit = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".pit."))
        .collect::<Vec<_>>();
    assert_eq!(pit.len(), 25);
    let pit_region_id = region_at(&game, pit[0].position).region_id.clone();
    let min_x = pit
        .iter()
        .map(|entity| entity.position.x)
        .min()
        .expect("pit x");
    let max_x = pit
        .iter()
        .map(|entity| entity.position.x)
        .max()
        .expect("pit x");
    let min_y = pit
        .iter()
        .map(|entity| entity.position.y)
        .min()
        .expect("pit y");
    let max_y = pit
        .iter()
        .map(|entity| entity.position.y)
        .max()
        .expect("pit y");
    for y in min_y - 3..=max_y + 3 {
        for x in min_x - 3..=max_x + 3 {
            assert_eq!(region_at(&game, Position { x, y }).region_id, pit_region_id);
        }
    }
    assert!(game.entities.iter().all(|entity| {
        !entity.id.contains(".pit.") || region_at(&game, entity.position).region_id == pit_region_id
    }));
    assert!(game.entities.iter().any(|entity| {
        entity.id == "demo.guardian.resonance-descent.1"
            && !region_at(&game, entity.position).region_id.is_empty()
    }));
    assert!(game.items.iter().all(|item| {
            matches!(item.location, ItemLocation::Ground(position) if !region_at(&game, position).region_id.is_empty())
        }));
    let final_hash = game.state_hash();
    let mut same_seed = Game::new(49);
    same_seed.player.position = Position { x: 3, y: 2 };
    same_seed
        .traverse_stairs(false)
        .expect("matching pressure dungeon entry should resolve")
        .expect("matching pressure dungeon entry should transition");
    for _ in 1..10 {
        descend_one_floor(&mut same_seed);
    }
    assert_eq!(same_seed.state_hash(), final_hash);
    let restored = Game::from_save(game.to_save()).expect("regional pit floor should restore");
    assert_eq!(restored.state_hash(), final_hash);
}

#[test]
fn regional_composition_round_trips_across_pressure_seeds() {
    for seed in [49, 77, 97, 156, 173, 211] {
        let mut game = Game::new(seed);
        game.player.position = Position { x: 3, y: 2 };
        game.traverse_stairs(false)
            .expect("pressure dungeon entry should resolve")
            .expect("pressure dungeon entry should transition");
        for depth in 1..=10 {
            Game::from_save(game.to_save()).unwrap_or_else(|error| {
                panic!("seed {seed} depth {depth} should round-trip: {error}")
            });
            if depth < 10 {
                descend_one_floor(&mut game);
            }
        }
    }
}

#[test]
fn budgeted_rooms_and_connected_cavern_obey_geometric_limits() {
    let mut game = Game::new(49);
    let definition = game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the first layout floor")
        .clone();
    let layout = definition
        .layout
        .as_ref()
        .expect("fixture should contain a layout");
    let room_geometry = layout
        .rooms
        .as_ref()
        .expect("fixture should contain room geometry");
    let rooms = game.generate_budgeted_rooms(&definition, room_geometry);

    assert_eq!(rooms.len(), 5);
    assert_eq!(rooms[0].id, "entry");
    assert_eq!(rooms[1].id, "remote");
    assert!(rooms.iter().map(GeneratedRoom::area).sum::<u32>() <= 112);
    let mut room_tiles = BTreeSet::new();
    for room in &rooms {
        for y in room.y..room.y + room.height {
            for x in room.x..room.x + room.width {
                let position = Position { x, y };
                if room.contains(position) {
                    assert!(room_tiles.insert(position));
                }
            }
        }
    }

    let mut terrain = vec![
        definition.wall_terrain_id.clone();
        usize::from(definition.width) * usize::from(definition.height)
    ];
    let cavern_origin =
        game.generate_connected_cavern(&definition, "demo.terrain.resonance-cavern", &mut terrain);
    let cavern_tiles = terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            (terrain_id == "demo.terrain.resonance-cavern").then_some(Position {
                x: i32::try_from(index % usize::from(definition.width))
                    .expect("cavern x must fit i32"),
                y: i32::try_from(index / usize::from(definition.width))
                    .expect("cavern y must fit i32"),
            })
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(cavern_tiles.len(), 64);
    let mut reached = BTreeSet::from([cavern_origin]);
    let mut frontier = VecDeque::from([cavern_origin]);
    while let Some(position) = frontier.pop_front() {
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if cavern_tiles.contains(&neighbor) && reached.insert(neighbor) {
                frontier.push_back(neighbor);
            }
        }
    }
    assert_eq!(reached, cavern_tiles);

    let mut rectangles = 0;
    let mut crosses = 0;
    for seed in 1..=64 {
        let mut seeded = Game::new(seed);
        for room in seeded.generate_budgeted_rooms(&definition, room_geometry) {
            match room.shape {
                ProceduralRoomShape::Rectangle => rectangles += 1,
                ProceduralRoomShape::Cross => crosses += 1,
            }
        }
    }
    assert!(rectangles > 0);
    assert!(crosses > 0);
}

#[test]
fn lake_and_river_obey_exact_hydrology_budgets_and_connectivity() {
    let mut lake_game = Game::new(77);
    let lake_definition = lake_game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the lake floor")
        .clone();
    let mut lake_terrain = vec![
        lake_definition.wall_terrain_id.clone();
        usize::from(lake_definition.width)
            * usize::from(lake_definition.height)
    ];
    let lake_origin = lake_game.generate_connected_lake(
        &lake_definition,
        "demo.terrain.resonance-water-deep",
        "demo.terrain.resonance-water-shallow",
        &mut lake_terrain,
    );
    let water_tiles = lake_terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            matches!(
                terrain_id.as_str(),
                "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
            )
            .then_some(Position {
                x: i32::try_from(index % usize::from(lake_definition.width))
                    .expect("lake x must fit i32"),
                y: i32::try_from(index / usize::from(lake_definition.width))
                    .expect("lake y must fit i32"),
            })
        })
        .collect::<BTreeSet<_>>();
    let deep_tiles = lake_terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            (terrain_id == "demo.terrain.resonance-water-deep").then_some(Position {
                x: i32::try_from(index % usize::from(lake_definition.width))
                    .expect("deep lake x must fit i32"),
                y: i32::try_from(index / usize::from(lake_definition.width))
                    .expect("deep lake y must fit i32"),
            })
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(water_tiles.len(), 76);
    assert_eq!(deep_tiles.len(), 30);
    for expected in [&water_tiles, &deep_tiles] {
        let mut reached = BTreeSet::from([lake_origin]);
        let mut frontier = VecDeque::from([lake_origin]);
        while let Some(position) = frontier.pop_front() {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let neighbor = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if expected.contains(&neighbor) && reached.insert(neighbor) {
                    frontier.push_back(neighbor);
                }
            }
        }
        assert_eq!(&reached, expected);
    }

    let mut river_game = Game::new(93);
    let river_definition = river_game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-10")
        .expect("fixture should contain the river floor")
        .clone();
    let mut river_terrain = vec![
        river_definition.wall_terrain_id.clone();
        usize::from(river_definition.width)
            * usize::from(river_definition.height)
    ];
    let target = Position {
        x: i32::from(river_definition.width / 2),
        y: i32::from(river_definition.height / 2),
    };
    river_game.generate_river(
        &river_definition,
        "demo.terrain.resonance-water-deep",
        "demo.terrain.resonance-water-shallow",
        target,
        &mut river_terrain,
    );
    let river_water_count = river_terrain
        .iter()
        .filter(|terrain_id| {
            matches!(
                terrain_id.as_str(),
                "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
            )
        })
        .count();
    assert_eq!(river_water_count, 52);
    assert_eq!(
        river_terrain[generated_terrain_index(river_definition.width, target)],
        "demo.terrain.resonance-water-deep"
    );
    assert!(
        (1..i32::from(river_definition.width - 1)).any(|x| {
            [1, i32::from(river_definition.height - 2)]
                .into_iter()
                .any(|y| {
                    river_terrain
                        [generated_terrain_index(river_definition.width, Position { x, y })]
                        == "demo.terrain.resonance-water-deep"
                })
        }) || (1..i32::from(river_definition.height - 1)).any(|y| {
            [1, i32::from(river_definition.width - 2)]
                .into_iter()
                .any(|x| {
                    river_terrain
                        [generated_terrain_index(river_definition.width, Position { x, y })]
                        == "demo.terrain.resonance-water-deep"
                })
        })
    );
}

#[test]
fn maze_destroyed_regions_and_streamers_obey_geometric_budgets() {
    let mut game = Game::new(151);
    let (maze_definition, destroyed_definition) = {
        let world = game
            .content
            .world(BUILT_IN_WORLD_ID)
            .expect("built-in world should exist");
        (
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == "demo.floor.resonance-depth-9")
                .expect("fixture should contain the maze floor")
                .clone(),
            world
                .procedural_floors
                .iter()
                .find(|floor| floor.id == "demo.floor.resonance-depth-10")
                .expect("fixture should contain the destroyed floor")
                .clone(),
        )
    };
    let maze_layout = maze_definition
        .layout
        .as_ref()
        .expect("fixture should contain a layout");
    let mut maze_terrain = vec![
        maze_definition.wall_terrain_id.clone();
        usize::from(maze_definition.width)
            * usize::from(maze_definition.height)
    ];
    let maze_tiles = game.generate_maze(
        &maze_definition,
        maze_layout
            .maze
            .as_ref()
            .expect("fixture should contain a maze"),
        "demo.terrain.resonant-floor",
        &mut maze_terrain,
    );
    assert_eq!(maze_tiles.len(), 127);
    let root = *maze_tiles
        .iter()
        .next()
        .expect("maze should contain a floor");
    let mut reached = BTreeSet::from([root]);
    let mut frontier = VecDeque::from([root]);
    while let Some(position) = frontier.pop_front() {
        for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
            let neighbor = Position {
                x: position.x + dx,
                y: position.y + dy,
            };
            if maze_tiles.contains(&neighbor) && reached.insert(neighbor) {
                frontier.push_back(neighbor);
            }
        }
    }
    assert_eq!(reached, maze_tiles);

    let before_streamers = maze_terrain.clone();
    let streamer_tiles =
        game.generate_streamers(&maze_definition, &maze_layout.streamers, &mut maze_terrain);
    assert_eq!(streamer_tiles.len(), 24);
    assert!(streamer_tiles.iter().all(|position| {
        before_streamers[generated_terrain_index(maze_definition.width, *position)]
            == maze_definition.wall_terrain_id
            && maze_terrain[generated_terrain_index(maze_definition.width, *position)]
                == "demo.terrain.resonance-vein"
    }));

    let mut destroyed_terrain = vec![
        destroyed_definition.wall_terrain_id.clone();
        usize::from(destroyed_definition.width)
            * usize::from(destroyed_definition.height)
    ];
    let destroyed_tiles = game.generate_destroyed_region(
        &destroyed_definition,
        "demo.terrain.resonance-ruin",
        &mut destroyed_terrain,
    );
    assert_eq!(destroyed_tiles.len(), 48);
    assert!(destroyed_tiles.iter().all(|position| {
        destroyed_terrain[generated_terrain_index(destroyed_definition.width, *position)]
            == "demo.terrain.resonance-ruin"
    }));
    let mut remaining = destroyed_tiles.clone();
    let mut component_count = 0;
    while let Some(&start) = remaining.iter().next() {
        component_count += 1;
        let mut component_frontier = VecDeque::from([start]);
        remaining.remove(&start);
        while let Some(position) = component_frontier.pop_front() {
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                let neighbor = Position {
                    x: position.x + dx,
                    y: position.y + dy,
                };
                if remaining.remove(&neighbor) {
                    component_frontier.push_back(neighbor);
                }
            }
        }
    }
    assert!((1..=2).contains(&component_count));
}

#[test]
fn maze_only_floor_uses_reachable_region_anchors_without_room_overlay() {
    let mut game = Game::new(151);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut game);
    }

    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-9");
    let walkable = game
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            game.content
                .terrain(terrain_id)
                .is_some_and(|terrain| terrain.walkable)
                .then_some(Position {
                    x: i32::try_from(index % usize::from(game.width)).expect("maze x must fit i32"),
                    y: i32::try_from(index / usize::from(game.width)).expect("maze y must fit i32"),
                })
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(walkable.len(), 127);
    let (entry, remote) = maze_floor_anchors(&walkable);
    assert_eq!(game.player.position, entry);
    assert_eq!(game.terrain_at(entry), "demo.terrain.stairs-up");
    assert_eq!(game.terrain_at(remote), "demo.terrain.stairs-down");
    assert_eq!(maze_floor_distances(&walkable, entry).len(), walkable.len());
    assert!(
        game.terrain
            .iter()
            .all(|terrain| terrain != "demo.terrain.door-secret")
    );
    assert!(game.entities.iter().all(|entity| {
        entity.id.contains(".encounter.") && walkable.contains(&entity.position)
    }));
    assert!(game.items.iter().all(|item| {
        matches!(item.location, ItemLocation::Ground(position) if walkable.contains(&position))
    }));

    let mut same_seed = Game::new(151);
    same_seed.player.position = Position { x: 3, y: 2 };
    same_seed
        .traverse_stairs(false)
        .expect("matching pressure dungeon entry should resolve")
        .expect("matching pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut same_seed);
    }
    assert_eq!(same_seed.state_hash(), game.state_hash());
}

#[test]
fn dynamic_friends_and_escorts_obey_group_budgets_and_formations() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..6 {
        descend_one_floor(&mut game);
    }

    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-6");
    assert_eq!(game.entities.len(), 7);
    let captain = game
        .entities
        .iter()
        .find(|entity| entity.kind_id == "demo.actor.chorus-captain")
        .expect("depth six should contain one chorus captain");
    let captain_position = captain.position;
    let friends = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".friend."))
        .collect::<Vec<_>>();
    let escorts = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".escort."))
        .collect::<Vec<_>>();
    assert!((1..=2).contains(&friends.len()));
    assert!((1..=2).contains(&escorts.len()));
    assert!(friends.len() + escorts.len() <= 4);
    assert!(friends.iter().all(|friend| {
        friend.kind_id == "demo.actor.chorus-captain" && adjacent(friend.position, captain_position)
    }));
    assert!(escorts.iter().all(|escort| {
        matches!(
            escort.kind_id.as_str(),
            "demo.actor.frost-wisp" | "demo.actor.storm-spark"
        ) && adjacent(escort.position, captain_position)
    }));
    let captain_pack = captain
        .pack
        .as_ref()
        .expect("dynamic leader should retain a pack identity");
    assert_eq!(captain_pack.role, MonsterPackRoleDto::Leader);
    assert_eq!(captain_pack.behavior, MonsterPackBehaviorDto::Seek);
    assert!(friends.iter().all(|friend| {
        friend.pack.as_ref().is_some_and(|pack| {
            pack.id == captain_pack.id
                && pack.leader_id == captain.id
                && pack.role == MonsterPackRoleDto::Member
                && pack.behavior == MonsterPackBehaviorDto::Surround
        })
    }));
    assert!(escorts.iter().all(|escort| {
        escort.pack.as_ref().is_some_and(|pack| {
            pack.id == captain_pack.id
                && pack.leader_id == captain.id
                && pack.role == MonsterPackRoleDto::Member
                && pack.behavior == MonsterPackBehaviorDto::GuardLeader
        })
    }));
    let captain_region_id = region_at(&game, captain_position).region_id.clone();
    assert!(
        game.entities
            .iter()
            .filter(|entity| entity.pack.is_some())
            .all(|entity| region_at(&game, entity.position).region_id == captain_region_id)
    );
    assert!(
        game.entities
            .iter()
            .filter(|entity| entity.pack.is_none())
            .all(
                |entity| match region_at(&game, entity.position).region_id.as_str() {
                    "demo.region.resonance-grotto" => matches!(
                        entity.kind_id.as_str(),
                        "demo.actor.acid-seep" | "demo.actor.venom-spore"
                    ),
                    "demo.region.resonance-gallery" => matches!(
                        entity.kind_id.as_str(),
                        "demo.actor.echo-hound" | "demo.actor.storm-spark"
                    ),
                    _ => false,
                }
            )
    );
    let room_feature_positions = game
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            matches!(
                terrain_id.as_str(),
                "demo.terrain.trap-echo-snare" | "demo.terrain.echo-rubble"
            )
            .then_some(Position {
                x: i32::try_from(index % usize::from(game.width)).expect("x must fit i32"),
                y: i32::try_from(index / usize::from(game.width)).expect("y must fit i32"),
            })
        })
        .collect::<Vec<_>>();
    assert!(room_feature_positions.len() >= 2);
    assert!(
        room_feature_positions
            .iter()
            .all(|position| !region_at(&game, *position).region_id.is_empty())
    );
    descend_one_floor(&mut game);
    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-7");
    assert_eq!(game.entities.len(), 8);
    let shepherd = game
        .entities
        .iter()
        .find(|entity| entity.kind_id == "demo.actor.spore-shepherd")
        .expect("depth seven should contain one spore shepherd");
    let shepherd_position = shepherd.position;
    let friends = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".friend."))
        .collect::<Vec<_>>();
    let escorts = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".escort."))
        .collect::<Vec<_>>();
    assert!((1..=2).contains(&friends.len()));
    assert!((2..=3).contains(&escorts.len()));
    assert!(friends.len() + escorts.len() <= 5);
    assert!(friends.iter().all(|friend| {
        friend.kind_id == "demo.actor.spore-shepherd"
            && adjacent(friend.position, shepherd_position)
    }));
    assert!(escorts.iter().all(|escort| {
        matches!(
            escort.kind_id.as_str(),
            "demo.actor.venom-spore" | "demo.actor.echo-hound"
        ) && adjacent(escort.position, shepherd_position)
    }));
    let shepherd_region_id = region_at(&game, shepherd_position).region_id.clone();
    assert!(
        game.entities
            .iter()
            .filter(|entity| entity.pack.is_some())
            .all(|entity| region_at(&game, entity.position).region_id == shepherd_region_id)
    );

    let restored =
        Game::from_save(game.to_save()).expect("dynamic encounter groups should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        actors_to_save(&restored.entities),
        actors_to_save(&game.entities)
    );
}

#[test]
fn pack_ai_reserves_surround_targets_and_guards_the_leader() {
    let mut game = Game::new(42);
    game.player.position = Position { x: 10, y: 10 };
    let base = game.entities[0].clone();
    let pack_id = "test.pack.1";
    let leader_id = "test.pack.leader";
    let pack = |role, behavior| {
        Some(MonsterPackIdentity {
            id: pack_id.to_owned(),
            leader_id: leader_id.to_owned(),
            role,
            behavior,
        })
    };
    let mut leader = base.clone();
    leader.id = leader_id.to_owned();
    leader.position = Position { x: 9, y: 7 };
    leader.pack = pack(MonsterPackRoleDto::Leader, MonsterPackBehaviorDto::Seek);
    let mut friend_one = base.clone();
    friend_one.id = "test.pack.friend.1".to_owned();
    friend_one.position = Position { x: 7, y: 9 };
    friend_one.pack = pack(MonsterPackRoleDto::Member, MonsterPackBehaviorDto::Surround);
    let mut friend_two = base.clone();
    friend_two.id = "test.pack.friend.2".to_owned();
    friend_two.position = Position { x: 7, y: 11 };
    friend_two.pack = pack(MonsterPackRoleDto::Member, MonsterPackBehaviorDto::Surround);
    let mut escort = base;
    escort.id = "test.pack.escort.1".to_owned();
    escort.position = Position { x: 6, y: 7 };
    escort.pack = pack(
        MonsterPackRoleDto::Member,
        MonsterPackBehaviorDto::GuardLeader,
    );
    game.entities = vec![leader, friend_one, friend_two, escort];
    game.dungeon_states
        .get_mut("demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = true;
    game.items.clear();

    let mut reservations = BTreeSet::new();
    assert!(game.next_surround_step(1, &mut reservations).is_some());
    assert!(game.next_surround_step(2, &mut reservations).is_some());
    assert_eq!(reservations.len(), 2);
    assert!(
        reservations
            .iter()
            .all(|target| { adjacent(*target, game.player.position) && game.is_walkable(*target) })
    );

    let leader_position = game.entities[0].position;
    let before = squared_distance(game.entities[3].position, leader_position);
    game.resolve_monster_action(
        3,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated pack action should resolve");
    assert!(squared_distance(game.entities[3].position, leader_position) < before);

    let restored = Game::from_save(game.to_save()).expect("pack state should round-trip");
    assert_eq!(
        actors_to_save(&restored.entities),
        actors_to_save(&game.entities)
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn malformed_pack_identity_is_rejected_and_v58_remains_independent() {
    let mut malformed = Game::new(42).to_save();
    malformed.entities[0].pack = Some(rfb_protocol::MonsterPackSaveDto {
        id: "test.pack.missing-leader".to_owned(),
        leader_id: "test.actor.missing".to_owned(),
        role: MonsterPackRoleDto::Member,
        behavior: MonsterPackBehaviorDto::GuardLeader,
    });
    assert!(matches!(
        Game::from_save(malformed),
        Err(CoreError::InvalidSave("monster pack state is invalid"))
    ));

    let mut legacy = Game::new(49);
    legacy.player.position = Position { x: 3, y: 2 };
    legacy
        .traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..6 {
        descend_one_floor(&mut legacy);
    }
    let mut payload = legacy.to_save();
    payload.content_hash =
        "ee07c276bbe568fafc1e1d6942e9d57d158bd250ed452b32c01c774d8521e96d".to_owned();
    for entity in &mut payload.entities {
        entity.pack = None;
    }
    let restored = Game::from_save(payload).expect("v58 actors without pack state should load");
    assert!(restored.entities.iter().all(|entity| entity.pack.is_none()));
}

#[test]
fn terrain_features_filter_by_depth_and_remain_deterministic() {
    let mut locked_door_seeds = 0;
    let mut secret_door_seeds = 0;
    for seed in 1..=64 {
        let mut left = Game::new(seed);
        let mut right = Game::new(seed);
        for game in [&mut left, &mut right] {
            game.player.position = Position { x: 3, y: 2 };
            game.traverse_stairs(false)
                .expect("pressure dungeon entry should resolve")
                .expect("pressure dungeon entry should transition");
            descend_one_floor(game);
            descend_one_floor(game);
        }
        assert_eq!(left.current_floor_id, "demo.floor.resonance-depth-3");
        assert_eq!(left.state_hash(), right.state_hash());
        assert_eq!(
            left.terrain
                .iter()
                .filter(|terrain| {
                    matches!(
                        terrain.as_str(),
                        "demo.terrain.trap-echo-snare" | "demo.terrain.echo-rubble"
                    )
                })
                .count(),
            3
        );
        assert!(
            !left
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.door-locked")
        );

        descend_one_floor(&mut left);
        if left
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.door-locked")
        {
            locked_door_seeds += 1;
        }
        assert_eq!(
            left.terrain
                .iter()
                .filter(|terrain| *terrain == "demo.terrain.door-secret")
                .count(),
            1
        );

        descend_one_floor(&mut left);
        descend_one_floor(&mut left);
        if left
            .terrain
            .iter()
            .filter(|terrain| *terrain == "demo.terrain.door-secret")
            .count()
            > 1
        {
            secret_door_seeds += 1;
        }
    }
    assert!(locked_door_seeds > 0);
    assert!(secret_door_seeds > 0);
}

#[test]
fn terrain_feature_space_failure_falls_back_without_overlap() {
    let seed = (1..=64)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(101) < 100
        })
        .expect("a seed should select the impossible corridor candidate first");
    let mut game = Game::new(seed);
    let mut definition = game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("demo world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-3")
        .expect("fixture should contain a terrain feature floor")
        .clone();
    definition.width = 4;
    definition.height = 4;
    definition
        .generation_budget
        .as_mut()
        .expect("fixture should contain a generation budget")
        .feature_placements = Some(2);
    let rooms = [GeneratedRoom {
        id: "entry".to_owned(),
        x: 1,
        y: 1,
        width: 1,
        height: 1,
        shape: ProceduralRoomShape::Rectangle,
    }];
    let target = Position { x: 1, y: 1 };
    let mut terrain = vec!["demo.terrain.wall".to_owned(); 16];
    set_generated_terrain(&mut terrain, definition.width, target, "demo.terrain.floor");
    let entries = [
        TerrainFeatureEntryDefinition {
            terrain_id: "demo.terrain.door-locked".to_owned(),
            placement: TerrainFeaturePlacement::Corridor,
            weight: 100,
            min_depth: 1,
            max_depth: 10,
        },
        TerrainFeatureEntryDefinition {
            terrain_id: "demo.terrain.trap-echo-snare".to_owned(),
            placement: TerrainFeaturePlacement::Room,
            weight: 1,
            min_depth: 1,
            max_depth: 10,
        },
    ];

    let placements = game.place_terrain_features(
        &definition,
        &entries,
        TerrainFeaturePlacementContext {
            rooms: &rooms,
            reserved: &BTreeSet::new(),
            floor_terrain_id: "demo.terrain.floor",
            room_floor_terrain_ids: &BTreeSet::new(),
        },
        &mut terrain,
    );

    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].position, target);
    assert_eq!(placements[0].terrain_id, "demo.terrain.trap-echo-snare");
    assert_eq!(
        terrain_feature_placement_candidates(
            &terrain,
            definition.width,
            "demo.terrain.floor",
            &BTreeSet::new(),
            &rooms,
            &BTreeSet::new(),
            TerrainFeaturePlacement::Room,
        ),
        Vec::<Position>::new()
    );
}

#[test]
fn formation_space_pressure_shrinks_then_falls_back_atomically() {
    let seed = (1..=64)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(2) == 1 && rng.bounded(2) == 1
        })
        .expect("a seed should request both maximum companion counts");
    let mut game = Game::new(seed);
    let definition = game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-6")
        .expect("fixture should contain the ring formation floor")
        .clone();
    let mut table = game
        .content
        .encounter_table("demo.encounter-table.resonance-formations")
        .expect("fixture should contain the formation encounter table")
        .clone();
    table.rolls = 1;
    let eligible_entries = table
        .entries
        .iter()
        .filter(|entry| entry.min_depth <= 6 && 6 <= entry.max_depth)
        .cloned()
        .collect::<Vec<_>>();
    let rooms = [GeneratedRoom {
        id: "remote".to_owned(),
        x: 0,
        y: 0,
        width: 3,
        height: 3,
        shape: ProceduralRoomShape::Rectangle,
    }];
    let free = BTreeSet::from([
        Position { x: 1, y: 0 },
        Position { x: 1, y: 1 },
        Position { x: 1, y: 2 },
    ]);
    let mut occupied = (0..3)
        .flat_map(|y| (0..3).map(move |x| Position { x, y }))
        .filter(|position| !free.contains(position))
        .collect::<BTreeSet<_>>();

    let shrunk = game.generate_dynamic_encounter_groups(
        &definition,
        &table,
        &eligible_entries,
        &rooms,
        "remote",
        0,
        1,
        true,
        &definition.id,
        &mut occupied,
    );
    assert_eq!(shrunk.len(), 3);
    assert_eq!(
        shrunk
            .iter()
            .filter(|actor| actor.id.contains(".friend.") || actor.id.contains(".escort."))
            .count(),
        2
    );

    let mut left = Game::new(seed);
    let mut right = Game::new(seed);
    let only_one_free = BTreeSet::from([Position { x: 1, y: 1 }]);
    let occupied = (0..3)
        .flat_map(|y| (0..3).map(move |x| Position { x, y }))
        .filter(|position| !only_one_free.contains(position))
        .collect::<BTreeSet<_>>();
    let mut left_occupied = occupied.clone();
    let mut right_occupied = occupied;
    let left_generated = left.generate_dynamic_encounter_groups(
        &definition,
        &table,
        &eligible_entries,
        &rooms,
        "remote",
        0,
        1,
        true,
        &definition.id,
        &mut left_occupied,
    );
    let right_generated = right.generate_dynamic_encounter_groups(
        &definition,
        &table,
        &eligible_entries,
        &rooms,
        "remote",
        0,
        1,
        true,
        &definition.id,
        &mut right_occupied,
    );
    assert_eq!(left_generated, right_generated);
    assert_eq!(left_generated.len(), 1);
    assert!(left_generated[0].id.ends_with(".encounter.1"));
    assert!(!left_generated[0].id.contains(".friend."));
    assert!(!left_generated[0].id.contains(".escort."));
}

#[test]
fn vault_coordinate_transforms_cover_rotations_and_reflections() {
    let game = Game::new(1);
    let vault = game
        .content
        .vault("demo.vault.resonance-spindle")
        .expect("fixture should contain the transformable Vault");

    assert_eq!(
        transformed_vault_dimensions(vault, VaultTransform::Rotate90),
        (4, 3)
    );
    assert_eq!(
        transformed_vault_position(vault, VaultTransform::Rotate90, vault.entrance_positions[0]),
        Position { x: 3, y: 1 }
    );
    assert_eq!(
        transformed_vault_position(
            vault,
            VaultTransform::MirrorHorizontal,
            ContentPosition { x: 0, y: 1 }
        ),
        Position { x: 2, y: 1 }
    );
    assert_eq!(
        transformed_vault_position(
            vault,
            VaultTransform::MirrorMainDiagonal,
            ContentPosition { x: 0, y: 1 }
        ),
        Position { x: 1, y: 0 }
    );
    assert_eq!(
        transformed_vault_position(
            vault,
            VaultTransform::MirrorAntiDiagonal,
            ContentPosition { x: 0, y: 1 }
        ),
        Position { x: 2, y: 2 }
    );
}

#[test]
fn spatial_vault_placement_falls_back_after_an_impossible_weighted_candidate() {
    let mut game = Game::new(1);
    let definition = game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-8")
        .expect("fixture should contain the spatial Vault floor")
        .clone();
    let theme = game
        .content
        .theme_table("demo.theme-table.resonance-descent")
        .expect("fixture should contain the pressure theme table")
        .entries
        .iter()
        .find(|entry| entry.min_depth <= 8 && 8 <= entry.max_depth)
        .expect("fixture should contain the deep theme");
    let mut impossible = theme
        .vault_candidates
        .iter()
        .find(|candidate| candidate.vault_id == "demo.vault.sealed-resonance-monolith")
        .expect("fixture should contain the impossible candidate")
        .clone();
    impossible.weight = u32::MAX;
    let mut fallback = theme
        .vault_candidates
        .iter()
        .find(|candidate| candidate.vault_id == "demo.vault.resonance-spindle")
        .expect("fixture should contain the fallback candidate")
        .clone();
    fallback.weight = 1;
    let mut probe = RfbRng::seeded(1);
    assert!(probe.bounded(u64::from(u32::MAX) + 1) < u64::from(u32::MAX));

    let mut terrain = vec![
        definition.wall_terrain_id.clone();
        usize::from(definition.width) * usize::from(definition.height)
    ];
    for x in 1..i32::from(definition.width - 1) {
        set_generated_terrain(
            &mut terrain,
            definition.width,
            Position { x, y: 10 },
            "demo.terrain.resonant-floor",
        );
    }
    let placements = game.select_spatial_vault_placements(
        &definition,
        &[impossible, fallback],
        false,
        "demo.terrain.resonant-floor",
        &mut terrain,
    );

    assert_eq!(placements.len(), 1);
    assert_eq!(placements[0].vault.id, "demo.vault.resonance-spindle");
}

#[test]
fn large_multi_entrance_vault_stitches_into_a_connected_floor() {
    let mut game = Game::new(64);
    let definition = game
        .content
        .world(BUILT_IN_WORLD_ID)
        .expect("built-in world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.resonance-depth-8")
        .expect("fixture should contain the spatial Vault floor")
        .clone();
    let candidate = ThemeVaultCandidateDefinition {
        vault_id: "demo.vault.resonance-crossroads".to_owned(),
        weight: 1,
        min_depth: 8,
        max_depth: 8,
    };
    let mut terrain = vec![
        definition.wall_terrain_id.clone();
        usize::from(definition.width) * usize::from(definition.height)
    ];
    for x in 1..i32::from(definition.width - 1) {
        set_generated_terrain(
            &mut terrain,
            definition.width,
            Position { x, y: 10 },
            "demo.terrain.resonant-floor",
        );
    }
    for y in 1..i32::from(definition.height - 1) {
        set_generated_terrain(
            &mut terrain,
            definition.width,
            Position { x: 10, y },
            "demo.terrain.resonant-floor",
        );
    }

    let placements = game.select_spatial_vault_placements(
        &definition,
        &[candidate],
        false,
        "demo.terrain.resonant-floor",
        &mut terrain,
    );

    assert_eq!(placements.len(), 1);
    let placement = &placements[0];
    assert_eq!(placement.vault.entrance_positions.len(), 4);
    assert!(!placement.connector_cells.is_empty());
    assert!(placement.connector_cells.iter().all(|position| {
        terrain[generated_terrain_index(definition.width, *position)]
            == "demo.terrain.resonant-floor"
    }));
    assert!(generated_terrain_is_connected(
        &terrain,
        definition.width,
        definition.height,
        &game.content,
    ));
    let (vault_width, vault_height) =
        transformed_vault_dimensions(&placement.vault, placement.transform);
    for entrance in &placement.vault.entrance_positions {
        let entrance = transformed_vault_position(&placement.vault, placement.transform, *entrance);
        let outward = vault_entrance_outward(entrance, vault_width, vault_height);
        let outside = Position {
            x: placement.origin.x + entrance.x + outward.x,
            y: placement.origin.y + entrance.y + outward.y,
        };
        assert!(terrain_is_connectable(
            &game.content,
            &terrain[generated_terrain_index(definition.width, outside)]
        ));
    }
}

#[test]
fn previous_v63_generated_floor_is_not_rebuilt_for_multi_entry_vaults() {
    let mut game = Game::new(93);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..8 {
        descend_one_floor(&mut game);
    }
    assert_eq!(game.current_floor_id, "demo.floor.resonance-depth-8");

    let mut payload = game.to_save();
    payload.content_hash =
        "246f51864965fac494c7a39959f591caa0434d9fa4eac839501f9d09526eb617".to_owned();
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let expected_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v63 generated floor should migrate");

    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, expected_draw_counter);
}

#[test]
fn previous_v49_generated_floor_is_not_backfilled_with_spatial_vaults() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..8 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "5d65fd9ca827dd05fc035650b82046edb592d563565c7e4075b32512a43f4e1f".to_owned();
    let removed_positions = payload
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".vault."))
        .map(|entity| entity.position)
        .collect::<Vec<_>>();
    payload
        .entities
        .retain(|entity| !entity.id.contains(".vault."));
    for position in removed_positions {
        let index = position.y as usize * usize::from(payload.terrain.width) + position.x as usize;
        payload.terrain.terrain_ids[index] = "demo.terrain.wall".to_owned();
    }
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v49 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-8");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        restored
            .entities
            .iter()
            .all(|entity| !entity.id.contains(".vault."))
    );
}

#[test]
fn previous_v50_generated_floor_is_not_backfilled_with_dynamic_groups() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..6 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "7eea25faef326b6d2250af357359902d0acf32d393c831655508a7e7eee5f2f0".to_owned();
    payload.entities.retain(|entity| entity.pack.is_none());
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v50 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-6");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        restored
            .entities
            .iter()
            .all(|entity| { !entity.id.contains(".friend.") && !entity.id.contains(".escort.") })
    );
}

#[test]
fn previous_v51_generated_floor_is_not_backfilled_with_terrain_features() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    descend_one_floor(&mut game);
    descend_one_floor(&mut game);
    let mut payload = game.to_save();
    payload.content_hash =
        "de045e1652d6e484937743b84a98e5e77887f28340a6492e72e8c6e1f72326e6".to_owned();
    let fixed_trap_position = Position {
        x: payload.player.position.x,
        y: payload.player.position.y + 1,
    };
    for index in 0..payload.terrain.terrain_ids.len() {
        let position = Position {
            x: i32::try_from(index % usize::from(payload.terrain.width))
                .expect("terrain x must fit i32"),
            y: i32::try_from(index / usize::from(payload.terrain.width))
                .expect("terrain y must fit i32"),
        };
        if payload.terrain.terrain_ids[index] == "demo.terrain.echo-rubble"
            || payload.terrain.terrain_ids[index] == "demo.terrain.trap-echo-snare"
                && position != fixed_trap_position
        {
            payload.terrain.terrain_ids[index] = "demo.terrain.floor".to_owned();
        }
    }
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v51 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-3");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        !restored
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.echo-rubble")
    );
    assert_eq!(
        restored
            .terrain
            .iter()
            .filter(|terrain| *terrain == "demo.terrain.trap-echo-snare")
            .count(),
        1
    );
}

#[test]
fn previous_v52_generated_floor_is_not_backfilled_with_layout_terrain() {
    let mut game = Game::new(49);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "1f8848e160b4ec51ca36acc512920946888fec20a36d7ac7b860bdb126aff79a".to_owned();
    for terrain_id in &mut payload.terrain.terrain_ids {
        if terrain_id == "demo.terrain.resonance-cavern" {
            *terrain_id = "demo.terrain.wall".to_owned();
        }
    }
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v52 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        restored
            .terrain
            .iter()
            .all(|terrain| terrain != "demo.terrain.resonance-cavern")
    );
}

#[test]
fn previous_v53_generated_floor_is_not_backfilled_with_hydrology() {
    let mut game = Game::new(77);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "11a28d24125572468148dce77f0082340ab82a3a7ef87637303578681b31c4e9".to_owned();
    for terrain_id in &mut payload.terrain.terrain_ids {
        if matches!(
            terrain_id.as_str(),
            "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
        ) {
            *terrain_id = "demo.terrain.resonance-cavern".to_owned();
        }
    }
    let expected_terrain = payload.terrain.clone();
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v53 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(restored.terrain.iter().all(|terrain| {
        !matches!(
            terrain.as_str(),
            "demo.terrain.resonance-water-deep" | "demo.terrain.resonance-water-shallow"
        )
    }));
}

#[test]
fn previous_v54_generated_floors_are_not_backfilled_with_late_terrain_stages() {
    let mut game = Game::new(151);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..10 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "e3c0d8653f86663c6bb7eb2cf99caf9d1ba5a259566560d7d70bb9592de2b1e9".to_owned();
    for terrain_id in &mut payload.terrain.terrain_ids {
        if matches!(
            terrain_id.as_str(),
            "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
        ) {
            *terrain_id = "demo.terrain.wall".to_owned();
        }
    }
    for floor in &mut payload.stored_floors {
        for terrain_id in &mut floor.terrain.terrain_ids {
            if matches!(
                terrain_id.as_str(),
                "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
            ) {
                *terrain_id = "demo.terrain.wall".to_owned();
            }
        }
    }
    let expected_terrain = payload.terrain.clone();
    let expected_stored_floors = payload.stored_floors.clone();
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v54 generated floors should migrate");
    let restored_payload = restored.to_save();

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-10");
    assert_eq!(restored_payload.terrain, expected_terrain);
    assert_eq!(restored_payload.stored_floors, expected_stored_floors);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(restored.terrain.iter().all(|terrain| {
        !matches!(
            terrain.as_str(),
            "demo.terrain.resonance-vein" | "demo.terrain.resonance-ruin"
        )
    }));
}

#[test]
fn previous_v55_generated_floor_is_not_backfilled_with_a_pit() {
    let mut game = Game::new(156);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "52c3db16ad5240ff83ba652b09ef70cccac991a586b593f84c11956a55539596".to_owned();
    payload
        .entities
        .retain(|entity| !entity.id.contains(".pit."));
    let expected_entities = payload.entities.clone();
    let expected_terrain = payload.terrain.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v55 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        restored
            .entities
            .iter()
            .all(|entity| !entity.id.contains(".pit."))
    );
}

#[test]
fn previous_v56_generated_floor_is_not_rebuilt_as_maze_only() {
    let mut game = Game::new(156);
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("pressure dungeon entry should resolve")
        .expect("pressure dungeon entry should transition");
    for _ in 1..9 {
        descend_one_floor(&mut game);
    }
    let mut payload = game.to_save();
    payload.content_hash =
        "461242cb2164434a7ef44a3692f1c9fa4ffe9921f07c17e0857c96f2f2d95041".to_owned();
    payload.entities[0].id = "demo.floor.resonance-depth-9.pit.1".to_owned();
    let marker_index = payload
        .terrain
        .terrain_ids
        .iter()
        .position(|terrain| terrain == "demo.terrain.wall")
        .expect("generated floor should retain a wall");
    payload.terrain.terrain_ids[marker_index] = "demo.terrain.resonance-cavern".to_owned();
    let expected_terrain = payload.terrain.clone();
    let mut expected_entities = payload.entities.clone();
    expected_entities.sort_by(|left, right| left.id.cmp(&right.id));
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v56 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.resonance-depth-9");
    assert_eq!(restored.to_save().terrain, expected_terrain);
    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(
        restored
            .entities
            .iter()
            .any(|entity| entity.id.contains(".pit."))
    );
}

#[test]
fn previous_v48_floor_and_dungeon_state_are_not_backfilled() {
    let mut game = Game::new(27);
    descend_one_floor(&mut game);
    let mut payload = game.to_save();
    payload.content_hash =
        "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad".to_owned();
    payload
        .dungeon_states
        .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
    let expected_entities = payload.entities.clone();
    let expected_items = payload.items.clone();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v48 floor should migrate");

    assert_eq!(actors_to_save(&restored.entities), expected_entities);
    assert_eq!(items_to_save(&restored.items), expected_items);
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
    assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);
}

#[test]
fn previous_v47_generated_floor_is_not_backfilled_with_tables_or_nest() {
    let mut game = Game::new(27);
    descend_one_floor(&mut game);
    let mut payload = game.to_save();
    payload.content_hash =
        "ae7b19dd780d73091a5b34aed2f67dcbc5650d2e2ed1d7748cc86f48020f8fb0".to_owned();
    payload
        .entities
        .retain(|entity| entity.id == "demo.floor.echo-depth-1.encounter.1");
    payload.entities[0].id = "demo.monster.echo-depth-1.1".to_owned();
    let saved_draw_counter = payload.rng.draw_counter;

    let restored = Game::from_save(payload).expect("v47 generated floor should migrate");

    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(restored.entities.len(), 1);
    assert_eq!(restored.entities[0].id, "demo.monster.echo-depth-1.1");
    assert!(
        restored
            .entities
            .iter()
            .all(|entity| !entity.id.contains(".nest.") && !entity.id.contains(".encounter."))
    );
    assert_eq!(restored.rng.draw_counter, saved_draw_counter);
}

#[test]
fn equipping_and_unequipping_moves_an_item_between_authoritative_lists() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);
    let carried = game.snapshot();
    let charm = carried
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("collected charm should be in inventory");
    assert_eq!(charm.modifiers.attack, 1);
    assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
    assert_eq!(charm.quality, None);
    assert!(charm.known_properties.is_empty());
    assert!(game.to_save().item_property_knowledge.is_empty());
    let equipped = game
        .dispatch(command(
            5,
            4,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("equipping should execute");

    assert_eq!(equipped.inventory.len(), 1);
    assert_eq!(equipped.equipment.len(), 1);
    assert_eq!(equipped.equipment[0].slot_id, "charm");
    assert_eq!(equipped.equipment[0].modifiers.attack, 2);
    assert_eq!(equipped.equipment[0].modifiers.defense, 1);
    assert_eq!(equipped.equipment[0].modifiers.max_hp, 4);
    assert_eq!(equipped.player.base_max_hp, 10);
    assert_eq!(equipped.player.max_hp, 14);
    assert_eq!(equipped.player.base_attack, 2);
    assert_eq!(equipped.player.attack, 4);
    assert_eq!(equipped.player.base_defense, 1);
    assert_eq!(equipped.player.defense, 2);
    assert_eq!(equipped.player.equipment_modifiers.attack, 2);
    assert_eq!(equipped.player.equipment_modifiers.defense, 1);
    assert_eq!(equipped.player.equipment_modifiers.max_hp, 4);
    assert_eq!(equipped.player.carried_weight_tenths_pound, 55);
    assert_eq!(equipped.events[0].message_key, "item-equip-success");
    assert_eq!(equipped.events[1].message_key, "item-property-discovered");
    assert_eq!(equipped.equipment[0].known_properties.len(), 1);
    assert_eq!(
        equipped.equipment[0].identification,
        ItemIdentificationDto::Identified
    );
    assert_eq!(equipped.equipment[0].quality, Some(ItemQualityDto::Fine));
    assert_eq!(
        equipped.equipment[0].known_properties[0].affix_id,
        "demo.affix.harmonic-edge"
    );
    let saved = game.to_save();
    assert_eq!(saved.item_property_knowledge.len(), 1);
    let restored = Game::from_save(saved.clone()).expect("affix knowledge should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    let mut invalid = saved;
    invalid.item_property_knowledge[0].known_affix_ids = vec!["demo.affix.missing".to_owned()];
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave(
            "item property knowledge state is invalid"
        ))
    ));

    game.player.hp = 14;

    let unequipped = game
        .dispatch(command(
            6,
            5,
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
        ))
        .expect("unequipping should execute");
    assert_eq!(unequipped.inventory.len(), 2);
    assert!(unequipped.equipment.is_empty());
    assert_eq!(unequipped.player.carried_weight_tenths_pound, 55);
    assert_eq!(unequipped.player.hp, 10);
    assert_eq!(unequipped.player.max_hp, 10);
    assert_eq!(unequipped.player.attack, 2);
    assert_eq!(unequipped.player.defense, 1);
    assert_eq!(unequipped.events[0].message_key, "item-unequip-success");
}

#[test]
fn appraising_reveals_quality_without_revealing_affixes() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);

    let before = game.snapshot();
    let charm = before
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("collected charm should be in inventory");
    assert_eq!(charm.identification, ItemIdentificationDto::Unexamined);
    assert_eq!(charm.quality, None);
    assert!(charm.known_properties.is_empty());

    let appraised = game
        .dispatch(command(
            5,
            4,
            GameCommand::Appraise {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
        ))
        .expect("appraisal should execute");
    let charm = appraised
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("appraised charm should remain in inventory");
    assert_eq!(charm.identification, ItemIdentificationDto::Appraised);
    assert_eq!(charm.quality, Some(ItemQualityDto::Fine));
    assert_eq!(charm.modifiers.attack, 1);
    assert!(charm.known_properties.is_empty());
    assert_eq!(appraised.player.attack, 2);
    assert_eq!(appraised.events[0].message_key, "item-appraise-success");
    assert_eq!(appraised.events[0].args["quality"], "fine");

    let saved = game.to_save();
    assert!(saved.item_property_knowledge[0].appraised);
    assert!(!saved.item_property_knowledge[0].identified);
    assert!(saved.item_property_knowledge[0].known_affix_ids.is_empty());
    let restored = Game::from_save(saved).expect("appraisal knowledge should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn player_derived_stats_retain_equipment_and_status_sources() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
        },
    ))
    .expect("equipping should execute");
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.item.temporary-tonic.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_STUN.to_owned(),
        intensity: 2,
        remaining_ticks: 3,
        source_id: Some("demo.monster.impact.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player
        .statuses
        .sort_by(|left, right| left.kind_id.cmp(&right.kind_id));

    let stats = game.player_derived_stats();

    assert_eq!(stats.attack.value, 4);
    assert_eq!(stats.speed.value, 130);
    assert_eq!(stats.melee_skill.value, 60);
    assert!(stats.attack.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Equipment
            && contribution.source_id == "demo.item.echo-charm.1"
            && contribution.amount == 2
    }));
    assert!(stats.speed.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Status
            && contribution.source_id == STATUS_HASTE
            && contribution.origin_id.as_deref() == Some("demo.item.temporary-tonic.1")
            && contribution.amount == 20
    }));
    assert!(stats.melee_skill.contributions.iter().any(|contribution| {
        contribution.layer == StatLayer::Status
            && contribution.source_id == STATUS_STUN
            && contribution.origin_id.as_deref() == Some("demo.monster.impact.1")
            && contribution.amount == -20
    }));
}

#[test]
fn fear_check_can_consume_a_melee_action_without_attacking() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.entities[0].position = Position { x: 4, y: 3 };
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_SLOW.to_owned(),
        intensity: 10,
        remaining_ticks: 20,
        source_id: None,
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_FEAR.to_owned(),
        intensity: 2,
        remaining_ticks: 20,
        source_id: Some("demo.monster.ember-mote.1".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("fear-blocked action should still execute");

    assert_eq!(update.player.position, Position { x: 3, y: 3 });
    assert_eq!(update.entities[0].hp, 3);
    assert_eq!(update.turn, 1);
    assert_eq!(update.player.statuses[0].kind_id, STATUS_FEAR);
    assert_eq!(update.player.statuses[0].remaining_ticks, 10);
    assert_eq!(game.rng.draw_counter, 2);
    assert_eq!(update.events.len(), 1);
    assert_eq!(update.events[0].message_key, "status-fear-blocked");
}

#[test]
fn item_instance_identity_survives_location_transitions() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let original_instance_count = game.items.len();
    collect_both_demo_items(&mut game);

    let charm_id = "demo.item.echo-charm.1";
    assert_eq!(game.items.len(), original_instance_count);
    assert!(game.items.iter().any(|item| {
        item.id == charm_id && item.location == ItemLocation::Inventory && item.quantity == 1
    }));

    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: charm_id.to_owned(),
        },
    ))
    .expect("equip should execute");
    assert!(game.items.iter().any(|item| {
        item.id == charm_id
            && item.location
                == ItemLocation::Equipped {
                    slot_id: "charm".to_owned(),
                }
    }));

    game.dispatch(command(
        6,
        5,
        GameCommand::Unequip {
            slot_id: "charm".to_owned(),
        },
    ))
    .expect("unequip should execute");
    game.dispatch(command(
        7,
        6,
        GameCommand::Drop {
            item_ids: vec![charm_id.to_owned()],
        },
    ))
    .expect("drop should execute");

    assert_eq!(game.items.len(), original_instance_count);
    assert!(game.items.iter().any(|item| {
        item.id == charm_id
            && item.location == ItemLocation::Ground(game.player.position)
            && item.quantity == 1
    }));
}

#[test]
fn equipped_attack_modifier_changes_authoritative_melee_skill() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    game.dispatch(command(
        5,
        4,
        GameCommand::Equip {
            item_id: "demo.item.echo-charm.1".to_owned(),
        },
    ))
    .expect("equip should execute");
    game.entities[0].position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.rng = RfbRng::seeded(42);
    let update = game
        .dispatch(command(
            6,
            5,
            GameCommand::Move {
                direction: Direction::East,
            },
        ))
        .expect("equipped attack should execute");

    assert_eq!(update.events[0].message_key, "combat-player-hit");
    assert_eq!(update.player.melee_skill, 80);
    assert_eq!(update.events[0].args["damage"], "2");
    assert_eq!(update.entities[0].hp, 1);
}

#[test]
fn equipped_weapon_profile_drives_two_stable_player_attacks() {
    let mut game = Game::new(42);
    game.rng = RfbRng::seeded(42);
    let weapon = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-blade")
        .expect("demo weapon should exist");
    weapon.location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    let snapshot = game.snapshot();
    let profile = snapshot.player.melee_profile;

    assert_eq!(profile.attacks, 2);
    assert_eq!(profile.to_hit, 10);
    assert_eq!(profile.to_damage, 1);
    assert_eq!(profile.damage.dice, 1);
    assert_eq!(profile.damage.sides, 2);
    assert_eq!(
        profile.source_item_id.as_deref(),
        Some("demo.item.echo-blade.1")
    );
    assert_eq!(snapshot.equipment[0].melee_profile, Some(profile));

    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_player_melee(0, &mut events, &mut changed, &mut removed)
        .expect("melee resolution should succeed");

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                DomainEvent::PlayerMeleeHit { .. } | DomainEvent::PlayerMeleeMissed { .. }
            ))
            .count(),
        2
    );
    assert!(removed.is_empty());
}

#[test]
fn equipped_launcher_traces_to_first_target_and_resolves_damage() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 7, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("projectile action should execute");

    let projectile = update
        .events
        .iter()
        .find(|event| event.kind.starts_with("combat.projectile-"))
        .expect("projectile event should be emitted");
    let trace = projectile
        .trace
        .as_ref()
        .expect("projectile trace should exist");
    assert_eq!(trace.origin, Position { x: 3, y: 3 });
    assert_eq!(trace.impact, Position { x: 7, y: 3 });
    assert_eq!(trace.landing, Position { x: 7, y: 3 });
    assert_eq!(trace.traversed.len(), 4);
    assert_eq!(projectile.kind, "combat.projectile-hit");
    assert!(update.entities[0].hp < 10);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "combat.projectile-ammo-recovered")
    );
    assert_eq!(
        update
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.resonance-pellet")
            .map(|item| item.quantity),
        Some(5)
    );
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.resonance-pellet"
            && item.quantity == 1
            && item.position == Position { x: 7, y: 3 }
    }));
}

#[test]
fn ammunition_breakage_is_checked_after_hitting_a_body() {
    let mut game = Game::new(16);
    game.rng = RfbRng::seeded(16);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 7, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("projectile action should execute");

    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "combat.projectile-ammo-broken")
    );
    assert_eq!(update.inventory[0].quantity, 5);
    assert!(!update.items.iter().any(|item| {
        item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 7, y: 3 }
    }));
    assert_eq!(game.next_item_instance_serial, 3);
}

#[test]
fn ammunition_that_hits_no_body_lands_without_a_breakage_roll() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::North,
            },
        ))
        .expect("projectile action should execute");

    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert_eq!(update.events[0].kind, "combat.projectile-landed");
    assert_eq!(update.events[1].kind, "combat.projectile-ammo-recovered");
    assert!(update.items.iter().any(|item| {
        item.kind_id == "demo.item.resonance-pellet" && item.position == Position { x: 3, y: 1 }
    }));
}

#[test]
fn launcher_without_inventory_ammunition_does_not_advance_rng() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Fire {
                direction: Direction::East,
            },
        ))
        .expect("unavailable fire action should execute deterministically");

    assert_eq!(update.events[0].kind, "combat.projectile-ammo-unavailable");
    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert!(update.inventory.is_empty());
    assert!(
        update
            .items
            .iter()
            .any(|item| { item.kind_id == "demo.item.resonance-pellet" && item.quantity == 6 })
    );
}

#[test]
fn entity_targeting_uses_a_stable_off_axis_line() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 9, y: 5 };
    game.entities[0].hp = 10;
    let expected_path = vec![
        Position { x: 4, y: 3 },
        Position { x: 5, y: 4 },
        Position { x: 6, y: 4 },
        Position { x: 7, y: 4 },
        Position { x: 8, y: 5 },
        Position { x: 9, y: 5 },
    ];
    assert_eq!(
        game.projectile_path(
            &TargetSelection::Position {
                position: Position { x: 9, y: 5 },
            },
            6,
        ),
        Some(expected_path.clone())
    );

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            },
        ))
        .expect("targeted projectile action should execute");

    let projectile = update
        .events
        .iter()
        .find(|event| event.kind == "combat.projectile-hit")
        .expect("targeted projectile should hit");
    let trace = projectile.trace.as_ref().expect("trace should exist");
    assert_eq!(trace.impact, Position { x: 9, y: 5 });
    assert_eq!(trace.traversed, expected_path);
    let target_spec = update
        .player
        .projectile_profile
        .as_ref()
        .expect("equipped launcher profile should exist")
        .target_spec
        .clone();
    assert_eq!(target_spec.range, 6);
    assert_eq!(
        target_spec.modes,
        [
            TargetModeDto::Direction,
            TargetModeDto::Position,
            TargetModeDto::Entity,
        ]
    );
}

#[test]
fn invalid_entity_target_preserves_ammunition_and_rng() {
    let mut game = Game::new(0);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-sling")
        .expect("demo launcher should exist")
        .location = ItemLocation::Equipped {
        slot_id: "launcher".to_owned(),
    };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.resonance-pellet")
        .expect("demo ammunition should exist")
        .location = ItemLocation::Inventory;
    let rng_draws = game.rng_draw_counter();

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::FireTarget {
                target: TargetSelection::Entity {
                    entity_id: "demo.monster.missing.1".to_owned(),
                },
            },
        ))
        .expect("invalid target should produce a deterministic event");

    assert_eq!(
        update.events[0].kind,
        "combat.projectile-target-unavailable"
    );
    assert_eq!(game.rng_draw_counter(), rng_draws);
    assert_eq!(update.inventory[0].quantity, 6);
}

#[test]
fn throwing_one_item_splits_the_stack_and_lands_before_a_wall() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.player.position = Position { x: 10, y: 3 };
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("throw action should execute");

    let thrown = update
        .events
        .iter()
        .find(|event| event.kind == "item.thrown")
        .expect("throw event should be emitted");
    let trace = thrown.trace.as_ref().expect("throw trace should exist");
    assert_eq!(trace.origin, Position { x: 10, y: 3 });
    assert_eq!(trace.impact, Position { x: 11, y: 3 });
    assert_eq!(trace.landing, Position { x: 10, y: 3 });
    assert!(trace.traversed.is_empty());
    assert_eq!(update.inventory[0].quantity, 4);
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.luminous-shard"
            && item.quantity == 1
            && item.position == Position { x: 10, y: 3 }
    }));
}

#[test]
fn throwable_profile_uses_weight_range_and_resolves_damage() {
    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.item_knowledge.insert(
        "demo.item.luminous-shard".to_owned(),
        ItemKnowledgeState {
            tried: true,
            aware: true,
        },
    );
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 6, y: 3 };
    game.entities[0].hp = 10;
    let inventory = game.snapshot().inventory;
    let shard = inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("throwable should be projected into inventory");
    assert_eq!(shard.weight_tenths_pound, 10);
    assert_eq!(
        shard
            .throw_profile
            .as_ref()
            .expect("shard should expose its throw profile")
            .range,
        5
    );

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("throw attack should execute");

    let hit = update
        .events
        .iter()
        .find(|event| event.kind == "combat.throw-hit")
        .expect("throw hit should be emitted");
    assert_eq!(hit.args["source"], "demo.item.luminous-shard");
    assert_eq!(hit.args["target"], "demo.actor.ember-mote");
    assert_eq!(hit.args["damage"], "1");
    assert_eq!(update.entities[0].hp, 9);
    assert_eq!(update.inventory[0].quantity, 4);
    assert!(update.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.kind_id == "demo.item.luminous-shard"
            && item.position == Position { x: 6, y: 3 }
    }));
}

#[test]
fn throwing_an_unknown_item_marks_the_kind_tried_and_preserves_its_appearance() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo unknown stack should exist")
        .location = ItemLocation::Inventory;
    let before = game.snapshot();
    let shard = before
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("unknown shard should be projected");
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
    assert!(shard.throw_profile.is_none());

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::North,
            },
        ))
        .expect("throwing an unknown item should execute");

    let remaining = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("remaining stack should stay carried");
    assert_eq!(remaining.knowledge, ItemKnowledgeDto::Tried);
    assert_eq!(
        remaining.display_name_key,
        "item-demo-unfamiliar-shard-name"
    );
    assert!(remaining.throw_profile.is_none());
    assert_eq!(game.to_save().item_knowledge.len(), 1);
    let restored = Game::from_save(game.to_save()).expect("tried knowledge should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn aware_item_knowledge_reveals_the_true_name_and_profile_after_reload() {
    let mut game = Game::new(7);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo unknown stack should exist")
        .location = ItemLocation::Inventory;
    let mut payload = game.to_save();
    payload.item_knowledge = vec![ItemKnowledgeSaveDto {
        kind_id: "demo.item.luminous-shard".to_owned(),
        tried: true,
        aware: true,
    }];

    let restored = Game::from_save(payload).expect("aware knowledge should load");
    let shard = restored
        .snapshot()
        .inventory
        .into_iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("aware shard should be projected");
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
    assert!(shard.throw_profile.is_some());

    let mut invalid = restored.to_save();
    invalid.item_knowledge[0].tried = false;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item knowledge state is invalid"))
    ));
}

#[test]
fn observable_item_use_consumes_one_heals_and_marks_the_kind_aware() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.player.hp = 3;
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo usable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
        ))
        .expect("using a healing item should execute");

    assert_eq!(update.player.hp, 7);
    let shard = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("the remaining stack should stay carried");
    assert_eq!(shard.quantity, 4);
    assert!(shard.usable);
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(shard.display_name_key, "item-demo-luminous-shard-name");
    assert!(shard.throw_profile.is_some());
    assert_eq!(update.events[0].kind, "item.use-heal");
    assert_eq!(
        update.events[0].args["nameKey"],
        "item-demo-luminous-shard-name"
    );
    assert!(matches!(
        update.events[0].outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if resolution.requested == 4 && resolution.applied == 4
    ));
    let restored = Game::from_save(game.to_save()).expect("aware use result should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn unobservable_item_use_consumes_one_but_only_marks_the_kind_tried() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo usable stack should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
        ))
        .expect("using an item at full health should execute");

    assert_eq!(update.player.hp, 10);
    let shard = update
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("the remaining stack should stay carried");
    assert_eq!(shard.quantity, 4);
    assert_eq!(shard.knowledge, ItemKnowledgeDto::Tried);
    assert_eq!(shard.display_name_key, "item-demo-unfamiliar-shard-name");
    assert!(shard.throw_profile.is_none());
    assert_eq!(update.events[0].kind, "item.use-no-effect");
    assert_eq!(
        update.events[0].args["nameKey"],
        "item-demo-unfamiliar-shard-name"
    );
    assert!(matches!(
        update.events[0].outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if resolution.requested == 4 && resolution.applied == 0
    ));
}

#[test]
fn unusable_inventory_item_is_not_consumed_or_added_to_knowledge() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("demo non-consumable should exist")
        .location = ItemLocation::Inventory;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::UseItem {
                item_id: "demo.item.echo-charm.1".to_owned(),
                target: None,
            },
        ))
        .expect("an unavailable use attempt should remain a valid action");

    assert_eq!(update.events[0].kind, "item.use-unavailable");
    assert!(
        update
            .inventory
            .iter()
            .any(|item| item.id == "demo.item.echo-charm.1" && item.quantity == 1)
    );
    assert!(game.to_save().item_knowledge.is_empty());
}

#[test]
fn missed_throw_still_lands_at_the_collided_target() {
    let mut game = Game::new(3);
    game.rng = RfbRng::seeded(3);
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("demo throwable stack should exist")
        .location = ItemLocation::Inventory;
    game.entities[0].position = Position { x: 6, y: 3 };
    game.entities[0].hp = 10;

    let update = game
        .dispatch(command(
            1,
            0,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ))
        .expect("missed throw should execute");

    assert_eq!(update.events[0].kind, "combat.throw-miss");
    assert_eq!(update.entities[0].hp, 10);
    assert!(update.items.iter().any(|item| {
        item.kind_id == "demo.item.luminous-shard" && item.position == Position { x: 6, y: 3 }
    }));
}

#[test]
fn dropping_multiple_selected_stacks_is_atomic_and_deterministic() {
    let mut game = Game::new(42);
    collect_both_demo_items(&mut game);
    let update = game
        .dispatch(command(
            5,
            4,
            GameCommand::Drop {
                item_ids: vec![
                    "demo.item.luminous-shard.1".to_owned(),
                    "demo.item.echo-charm.1".to_owned(),
                ],
            },
        ))
        .expect("batch drop should execute");

    assert!(update.inventory.is_empty());
    assert_eq!(update.items.len(), 5);
    assert!(
        update
            .items
            .iter()
            .filter(|item| {
                item.kind_id != "demo.item.echo-blade"
                    && item.kind_id != "demo.item.resonance-sling"
                    && item.kind_id != "demo.item.resonance-pellet"
            })
            .all(|item| item.position == Position { x: 5, y: 3 })
    );
    assert_eq!(update.changed_cells.len(), 1);
    assert_eq!(update.events[0].message_key, "item-drop-success");
    assert_eq!(update.events[0].args["stacks"], "2");
    assert_eq!(update.events[0].args["quantity"], "6");
}

#[test]
fn pickup_on_empty_ground_is_a_deterministic_turn() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    let before = game.state_hash();
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("empty pickup should still execute");

    assert_eq!(update.turn, 1);
    assert!(update.changed_cells.is_empty());
    assert!(update.inventory.is_empty());
    assert_eq!(update.events[0].message_key, "item-pickup-none");
    assert_ne!(update.state_hash, before);
}

#[test]
fn pickup_merges_into_the_lowest_id_compatible_stack() {
    let mut game = Game::new(42);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "demo.inventory.resonance-pellet.1".to_owned(),
        kind_id: "demo.item.resonance-pellet".to_owned(),
        quantity: 19,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
    game.player.position = Position { x: 6, y: 4 };
    let update = game
        .dispatch(command(1, 0, GameCommand::PickUp))
        .expect("pickup should execute");

    assert_eq!(update.inventory.len(), 2);
    assert_eq!(update.inventory[0].id, "demo.inventory.resonance-pellet.1");
    assert_eq!(update.inventory[0].quantity, 20);
    assert_eq!(update.inventory[1].id, "demo.item.resonance-pellet.1");
    assert_eq!(update.inventory[1].quantity, 5);
}

#[test]
fn partial_drop_allocates_stable_ids_and_survives_save_round_trip() {
    let mut game = Game::new(42);
    game.dispatch(command(
        1,
        0,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("move should execute");
    game.dispatch(command(2, 1, GameCommand::PickUp))
        .expect("pickup should execute");
    let first_drop = game
        .dispatch(command(
            3,
            2,
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
        ))
        .expect("partial drop should execute");

    assert_eq!(first_drop.inventory[0].quantity, 3);
    assert!(first_drop.items.iter().any(|item| {
        item.id == "generated.item.2"
            && item.quantity == 2
            && item.position == Position { x: 4, y: 3 }
    }));
    assert_eq!(game.next_item_instance_serial, 3);

    let mut restored = Game::from_save(game.to_save()).expect("save should preserve allocator");
    let second_drop = restored
        .dispatch(command(
            4,
            3,
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 1,
            },
        ))
        .expect("second partial drop should execute");
    assert!(
        second_drop
            .items
            .iter()
            .any(|item| item.id == "generated.item.3" && item.quantity == 1)
    );
    assert_eq!(restored.next_item_instance_serial, 4);
}

#[test]
fn stale_revision_is_rejected_without_mutation() {
    let mut game = Game::new(1);
    let before = game.state_hash();
    let error = game
        .dispatch(command(1, 99, GameCommand::Wait))
        .expect_err("stale command should fail");
    assert!(matches!(error, CoreError::RevisionMismatch { .. }));
    assert_eq!(game.state_hash(), before);
}

fn assert_invariant_error_without_mutation(
    game: &mut Game,
    game_command: GameCommand,
    expected: &str,
) {
    let before = game.clone();
    let error = game
        .dispatch(command(1, 0, game_command))
        .expect_err("broken runtime reference should fail");
    match error {
        CoreError::Invariant(message) => assert_eq!(message, expected),
        other => panic!("expected an invariant error, got {other}"),
    }
    assert_eq!(game.to_save(), before.to_save());
    assert_eq!(game.resources_touched, before.resources_touched);
    assert_eq!(game.last_visual_cells, before.last_visual_cells);
}

#[test]
fn inventory_item_missing_its_kind_is_an_invariant_error() {
    const ITEM_ID: &str = "test.item.missing-kind";
    let mut game = skill_check_game(1, "demo.build.vanguard");
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");
    game.items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("test item should exist")
        .kind_id = "test.item-kind.missing".to_owned();

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
        "inventory item test.item.missing-kind references missing kind test.item-kind.missing",
    );
}

#[test]
fn dynamic_item_missing_its_activation_profile_is_an_invariant_error() {
    const ITEM_ID: &str = "test.item.missing-activation";
    let mut game = skill_check_game(1, "demo.build.tinkerer");
    give_inventory_item(&mut game, ITEM_ID, "demo.item.resonance-wand");
    game.items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .and_then(|item| item.activation.as_mut())
        .expect("dynamic test item should carry an activation")
        .profile_id = "test.activation.missing".to_owned();

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: Some(TargetSelection::Direction {
                direction: Direction::East,
            }),
        },
        "dynamic item test.item.missing-activation references missing activation profile test.activation.missing",
    );
}

#[test]
fn active_task_missing_its_objective_is_an_invariant_error() {
    let mut game = skill_check_game(1, "demo.build.vanguard");
    let state = game
        .task_states
        .get_mut("demo.task.echo-chain")
        .expect("staged task should exist");
    state.status = TaskStatusKindDto::Active;
    state.active_floor_id = Some(game.current_floor_id.clone());
    state.stage_index = 99;

    assert_invariant_error_without_mutation(
        &mut game,
        GameCommand::Wait,
        "active task demo.task.echo-chain references missing objective stage 99",
    );
}

#[test]
fn rfb_style_armor_reduction_uses_the_legacy_linear_cap() {
    assert_eq!(apply_melee_armor_reduction(100, 0), 100);
    assert_eq!(apply_melee_armor_reduction(100, 90), 70);
    assert_eq!(apply_melee_armor_reduction(100, 180), 40);
    assert_eq!(apply_melee_armor_reduction(100, 999), 40);
}

#[test]
fn fixed_seed_exercises_player_miss_and_death_rejection() {
    let mut miss_game = Game::new(0);
    miss_game.rng = RfbRng::seeded(0);
    miss_game.entities[0].position = Position { x: 4, y: 4 };
    miss_game.entities[0].energy_need = STANDARD_ACTION_COST;
    let miss_update = miss_game
        .dispatch(command(
            1,
            0,
            GameCommand::Move {
                direction: Direction::SouthEast,
            },
        ))
        .expect("fixed-seed player attack should execute");
    assert!(
        miss_update
            .events
            .iter()
            .any(|event| event.message_key == "combat-player-miss")
    );

    let mut game = Game::new(0);
    game.rng = RfbRng::seeded(0);
    game.entities[0].position = Position { x: 4, y: 4 };
    game.entities[0].energy_need = STANDARD_ACTION_COST;
    game.player.hp = 0;
    let update = game
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("adjacent monster turn should execute");
    assert!(update.player.is_dead);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "combat-player-death")
    );
    assert!(matches!(
        game.dispatch(command(2, 1, GameCommand::Wait)),
        Err(CoreError::PlayerDead)
    ));

    let mut full_health_game = Game::new(0);
    full_health_game.entities[0].position = Position { x: 4, y: 4 };
    full_health_game.entities[0].energy_need = STANDARD_ACTION_COST;
    let death_command = (1..100_u32).find(|seq| {
        full_health_game
            .dispatch(command(*seq, *seq - 1, GameCommand::Wait))
            .is_ok_and(|update| update.player.is_dead)
    });
    assert!(death_command.is_some());
}

#[test]
fn campaign_guardian_death_emits_victory_and_old_save_derives_it() {
    let mut game = Game::new(49);
    for damage_type in [
        DamageType::Physical,
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Poison,
    ] {
        game.player
            .resistances
            .set(damage_type, ResistanceLevel::Immune);
    }
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("campaign dungeon entry should resolve")
        .expect("campaign dungeon entry should transition");
    for _ in 1..10 {
        descend_one_floor(&mut game);
    }
    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.resonance-descent.1")
        .expect("campaign final floor should spawn its guardian");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.serpent-of-chaos"
    );
    let guardian_position = game.entities[guardian_index].position;
    game.entities[guardian_index].hp = 1;
    let (direction, player_position) = TERRAIN_INTERACTION_DIRECTIONS
        .iter()
        .find_map(|direction| {
            let (dx, dy) = direction.delta();
            let position = Position {
                x: guardian_position.x - dx,
                y: guardian_position.y - dy,
            };
            game.index(position)
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .filter(|terrain| terrain.walkable)
                .map(|_| (*direction, position))
        })
        .expect("guardian should have a walkable approach");
    game.player.position = player_position;
    let update = dispatch_next(&mut game, GameCommand::Move { direction });
    assert!(update.events.iter().any(|event| {
        event.message_key == "campaign-victorious"
            && event
                .args
                .get("score")
                .is_some_and(|score| score == "60000")
    }));
    assert_eq!(update.campaign.status, CampaignStatusDto::Victorious);
    assert_eq!(game.campaign_state.victory_turn, Some(1));
    assert_eq!(game.progress.level, 51);
    assert_eq!(CharacterProgress::level_cap(true), 100);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "player-level-cap-unlocked")
    );

    let mut old_payload = game.to_save();
    old_payload.campaign_state = None;
    let restored = Game::from_save(old_payload).expect("old save should derive victory state");
    assert_eq!(
        restored.snapshot().campaign.status,
        CampaignStatusDto::Victorious
    );
    assert_eq!(restored.snapshot().state_hash, game.snapshot().state_hash);
}

#[test]
fn default_character_build_preserves_the_v70_player_baseline() {
    let game = Game::new(42);
    let snapshot = game.snapshot();
    let build = snapshot
        .player
        .build
        .expect("default build should be projected");

    assert_eq!(build.build_id, "demo.build.explorer");
    assert_eq!(build.race_id, "demo.race.human");
    assert_eq!(build.class_id, "demo.class.explorer");
    assert_eq!(build.personality_id, "demo.personality.ordinary");
    assert_eq!(snapshot.player.max_hp, 10);
    assert_eq!(snapshot.player.melee_skill, 40);
    assert_eq!(snapshot.player.progress.skills.len(), 10);
    assert!(snapshot.inventory.is_empty());
    assert!(snapshot.equipment.is_empty());
}

#[test]
fn v70_save_migrates_default_build_and_skills_without_rng_drift() {
    let canonical = Game::new(42);
    let mut legacy = canonical.to_save();
    legacy.content_hash =
        "ad6b35c6e0ae8980a74fac51ea1e6597b09559541d4a85d598284dc2cb41d7e6".to_owned();
    legacy.player.build = None;
    legacy
        .player
        .progress
        .as_mut()
        .expect("v70 save should contain character progress")
        .skills
        .clear();
    let draw_counter = legacy.rng.draw_counter;

    let migrated = Game::from_save(legacy).expect("v70 save should migrate character build");
    let snapshot = migrated.snapshot();
    assert_eq!(
        snapshot
            .player
            .build
            .as_ref()
            .map(|build| build.build_id.as_str()),
        Some("demo.build.explorer")
    );
    assert_eq!(snapshot.player.progress.skills.len(), 10);
    assert_eq!(migrated.rng_draw_counter(), draw_counter);
    assert_eq!(migrated.state_hash(), canonical.state_hash());

    let restored = Game::from_save(migrated.to_save())
        .expect("migrated character build should survive another round trip");
    assert_eq!(restored.state_hash(), migrated.state_hash());
}

#[test]
fn representative_builds_merge_identity_skills_attributes_and_starting_gear() {
    let vanguard =
        Game::new_with_build(42, "demo.build.vanguard").expect("vanguard build should create");
    let snapshot = vanguard.snapshot();
    assert_eq!(snapshot.player.build.as_ref().unwrap().life_percent, 115);
    assert_eq!(snapshot.player.max_hp, 33);
    assert_eq!(snapshot.player.progress.attributes.strength.effective, 18);
    assert_eq!(
        snapshot
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == "demo.skill.melee")
            .map(|skill| skill.current),
        Some(78)
    );
    assert_eq!(snapshot.player.melee_skill, 88);
    assert_eq!(snapshot.inventory.len(), 2);
    assert_eq!(snapshot.equipment.len(), 1);
    assert_eq!(snapshot.equipment[0].kind_id, "demo.item.echo-blade");

    let scholar =
        Game::new_with_build(42, "demo.build.scholar").expect("scholar build should create");
    let scholar_snapshot = scholar.snapshot();
    assert_eq!(
        scholar_snapshot
            .player
            .build
            .as_ref()
            .unwrap()
            .experience_percent,
        156
    );
    assert!(
        scholar_snapshot
            .equipment
            .iter()
            .any(|item| item.kind_id == "demo.item.echo-charm")
    );

    let pathfinder =
        Game::new_with_build(42, "demo.build.pathfinder").expect("pathfinder build should create");
    assert!(pathfinder.snapshot().player.projectile_profile.is_some());

    let tinkerer =
        Game::new_with_build(42, "demo.build.tinkerer").expect("tinkerer build should create");
    assert!(
        tinkerer
            .snapshot()
            .player
            .progress
            .skills
            .iter()
            .find(|skill| skill.id == "demo.skill.device")
            .is_some_and(|skill| skill.current > 60)
    );
    assert_eq!(vanguard.rng_draw_counter(), scholar.rng_draw_counter());
}

#[test]
fn build_skill_growth_experience_multiplier_and_save_identity_are_deterministic() {
    let mut vanguard =
        Game::new_with_build(17, "demo.build.vanguard").expect("vanguard build should create");
    vanguard.apply_player_experience(380, &mut Vec::new());
    assert_eq!(vanguard.progress.level, 10);
    assert_eq!(
        vanguard
            .progress
            .skill("demo.skill.melee")
            .map(|skill| skill.current),
        Some(105)
    );

    let mut scholar =
        Game::new_with_build(17, "demo.build.scholar").expect("scholar build should create");
    scholar.apply_player_experience(100, &mut Vec::new());
    assert_eq!(scholar.progress.experience, 156);

    let restored = Game::from_save(vanguard.to_save()).expect("build save should reload");
    assert_eq!(restored.build, vanguard.build);
    assert_eq!(restored.progress.skills, vanguard.progress.skills);
    assert_eq!(restored.snapshot(), vanguard.snapshot());
    assert!(matches!(
        Game::new_with_build(17, "demo.build.missing"),
        Err(CoreError::UnknownCharacterBuild(_))
    ));
}

#[test]
fn scholar_studies_and_casts_an_ability_book_spell_deterministically() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let book_item_id = ability_book_item_id(&game);
    let initial_draws = game.rng_draw_counter();
    let initial = game.snapshot();
    assert_eq!(initial.player.resources[0].current, 21);
    assert_eq!(initial.player.resources[0].maximum, 21);
    let resonant_bolt = initial
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("scholar should expose resonant bolt");
    assert!(!resonant_bolt.learned);
    assert!(resonant_bolt.can_study);

    let study = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        study
            .events
            .iter()
            .any(|event| event.kind == "ability.studied")
    );
    assert_eq!(game.rng_draw_counter(), initial_draws);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.resonant-bolt")
            .is_some_and(|ability| ability.learned)
    );

    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&cast);
    assert_eq!(resolution.resource_before, 21);
    assert_eq!(resolution.base_resource_cost, 3);
    assert_eq!(resolution.resource_cost, 5);
    assert_eq!(resolution.resource_after, 16);
    assert_eq!(resolution.failure_percent, 20);
    assert_eq!(resolution.percentile_roll, 32);
    assert!(resolution.succeeded);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 128);
    assert_eq!(resolution.cast_count, 1);
    assert_eq!(resolution.fail_count, 0);
    assert!(cast.events.iter().any(|event| event.kind == "ability.hit"));
    assert!(cast.events.iter().any(|event| event.kind == "ability.slay"));
    assert!(
        !game
            .entities
            .iter()
            .any(|entity| entity.id == "demo.monster.ember-mote.1")
    );
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.resources[0].current, 16);
    assert_eq!(snapshot.player.resources[0].maximum, 23);
    assert_eq!(snapshot.player.progress.level, 2);

    let restored = Game::from_save(game.to_save()).expect("ability state should reload");
    assert_eq!(restored.snapshot(), snapshot);
}

#[test]
fn area_damage_uses_rfb_targeted_ball_path_falloff_and_ordering() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let ability_id = "demo.ability.echo-burst";
    game.learned_abilities.insert(ability_id.to_owned());
    for position in [
        Position { x: 4, y: 3 },
        Position { x: 5, y: 3 },
        Position { x: 6, y: 3 },
    ] {
        replace_terrain(&mut game, position, "demo.terrain.floor");
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 6, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 4, y: 3 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );

    let area = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityAreaDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful area spell should expose its resolved footprint");
    assert_eq!(area.center, Position { x: 6, y: 3 });
    assert_eq!(area.radius, 2);
    assert_eq!(area.target_count, 2);
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    assert!(area.affected_positions.windows(2).all(|positions| {
        let left = positions[0];
        let right = positions[1];
        (rfb_distance(area.center, left), left.y, left.x)
            <= (rfb_distance(area.center, right), right.y, right.x)
    }));

    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let center_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("center hit should expose damage"),
    };
    let edge_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("edge hit should expose damage"),
    };
    assert_eq!(center_damage, area.base_raw_damage);
    assert_eq!(edge_damage, rfb_area_damage(area.base_raw_damage, 2));
}

#[test]
fn area_damage_respects_walls_and_invalid_targets_are_zero_rng() {
    let ability_id = "demo.ability.echo-burst";
    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for position in [
        Position { x: 4, y: 3 },
        Position { x: 5, y: 3 },
        Position { x: 6, y: 3 },
        Position { x: 6, y: 5 },
    ] {
        replace_terrain(&mut blocked, position, "demo.terrain.floor");
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 4 }, "demo.terrain.wall");
    let ember = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 6, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 6, y: 5 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let guardian_hp = guardian.hp;

    let update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let area = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityAreaDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("area outcome should exist");
    assert_eq!(area.target_count, 1);
    assert!(!area.affected_positions.contains(&Position { x: 6, y: 5 }));
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
    );

    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let progress_before = invalid.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: Position { x: 19, y: 19 },
            },
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert_eq!(invalid.ability_progress[ability_id], progress_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert!(!rejected.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
        )
    }));
}

#[test]
fn beam_damage_passes_through_actors_with_one_roll_and_stops_at_walls() {
    let ability_id = "demo.ability.echo-lance";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.learned_abilities.insert(ability_id.to_owned());
    for x in 4..=9 {
        replace_terrain(&mut game, Position { x, y: 3 }, "demo.terrain.floor");
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 5, y: 3 };
    ember.hp = 100;
    ember.energy_need = 1_000;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 3 };
    guardian.hp = 100;
    guardian.energy_need = 1_000;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let beam = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful beam should expose its line");
    assert_eq!(beam.target_count, 2);
    assert_eq!(beam.affected_positions.len(), 6);
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let first_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("beam hit should expose damage"),
    };
    let second_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("beam hit should expose damage"),
    };
    assert_eq!(first_damage, beam.base_raw_damage);
    assert_eq!(second_damage, beam.base_raw_damage);

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for x in 4..=9 {
        replace_terrain(&mut blocked, Position { x, y: 3 }, "demo.terrain.floor");
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 3 }, "demo.terrain.wall");
    for entity in &mut blocked.entities {
        entity.energy_need = 1_000;
    }
    blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 5, y: 3 };
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 3 };
    let guardian_hp = guardian.hp;
    let blocked_update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let blocked_beam = blocked_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("blocked beam should expose its line");
    assert_eq!(blocked_beam.target_count, 1);
    assert_eq!(
        blocked_beam.affected_positions,
        vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }]
    );
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
    );
}

#[test]
fn damage_bonus_adds_flat_amount_to_monster_cast_damage() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    for step in 0..=3 {
        let index = game
            .index(Position {
                x: player.x + step,
                y: player.y,
            })
            .expect("corridor cell");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.cinder-test",
        "demo.actor.cinder-adept",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        8,
        100,
        100,
        true,
    ));

    let mut observed = None;
    for _ in 0..40 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                let damage = resolution
                    .effects
                    .iter()
                    .chain(
                        resolution
                            .targets
                            .iter()
                            .flat_map(|target| target.effects.iter()),
                    )
                    .find_map(|effect| match effect {
                        AbilityEffectResolutionDto::Damage { resolution, .. } => Some(resolution),
                        _ => None,
                    })
                    .expect("cinder cast should resolve damage");
                observed = Some((resolution.ability_id.clone(), damage.raw_damage));
            }
        }
        if observed.is_some() || game.player_is_dead() {
            break;
        }
    }
    let (ability_id, raw_damage) = observed.expect("cinder adept should cast within 40 turns");
    // Every cinder ability carries a flat bonus, so the raw roll always
    // lands inside dice-plus-bonus bounds without extra RNG cost.
    let bounds = match ability_id.as_str() {
        "demo.ability.cinder-bolt" => 5..=9,
        "demo.ability.cinder-burst" => 3..=6,
        "demo.ability.cinder-fan" => 3..=5,
        other => panic!("unexpected cinder ability {other}"),
    };
    assert!(
        bounds.contains(&raw_damage),
        "raw damage {raw_damage} must include the flat bonus for {ability_id}"
    );
}

#[test]
fn breath_damage_scales_with_caster_hp_and_caps_at_max() {
    fn breath_raw_damage(drake_hp: i32) -> i32 {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        for cell in game.terrain.iter_mut() {
            *cell = "demo.terrain.wall".to_owned();
        }
        let player = game.player.position;
        for step in 0..=3 {
            let index = game
                .index(Position {
                    x: player.x + step,
                    y: player.y,
                })
                .expect("corridor cell");
            game.terrain[index] = "demo.terrain.floor".to_owned();
        }
        game.entities.push(actor_from_runtime_spawn(
            "generated.actor.breath-test",
            "demo.actor.ash-drake",
            Position {
                x: player.x + 3,
                y: player.y,
            },
            12,
            100,
            100,
            true,
        ));
        game.entities.last_mut().expect("drake was just pushed").hp = drake_hp;

        for _ in 0..40 {
            let update = dispatch_next(&mut game, GameCommand::Wait);
            for event in &update.events {
                if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                    event.outcome.as_ref()
                {
                    assert_eq!(resolution.ability_id, "demo.ability.ash-breath");
                    let damage = resolution
                        .effects
                        .iter()
                        .chain(
                            resolution
                                .targets
                                .iter()
                                .flat_map(|target| target.effects.iter()),
                        )
                        .find_map(|effect| match effect {
                            AbilityEffectResolutionDto::Damage { resolution, .. } => {
                                Some(resolution)
                            }
                            _ => None,
                        })
                        .expect("breath cast should resolve damage");
                    return damage.raw_damage;
                }
            }
            if game.player_is_dead() {
                break;
            }
        }
        panic!("ash drake should breathe within 40 turns");
    }

    // Full vigor: 12 * 60% = 7 exceeds the elemental cap of 6.
    assert_eq!(breath_raw_damage(12), 6);
    // Wounded: 5 * 60% = 3 stays below the cap, so the breath weakens.
    assert_eq!(breath_raw_damage(5), 3);
}

#[test]
fn category_summon_picks_tagged_kinds_and_rejects_empty_categories() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    for step in 0..=3 {
        for dy in -2..=2 {
            if let Some(index) = game.index(Position {
                x: player.x + step,
                y: player.y + dy,
            }) {
                game.terrain[index] = "demo.terrain.floor".to_owned();
            }
        }
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.binder-test",
        "demo.actor.mote-binder",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        9,
        100,
        100,
        true,
    ));

    let elemental_kinds = [
        "demo.actor.acid-seep",
        "demo.actor.ember-mote",
        "demo.actor.frost-wisp",
        "demo.actor.storm-spark",
        "demo.actor.venom-spore",
    ];
    let mut saw_empty_rejection = false;
    for _ in 0..60 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityDecision { resolution }) =
                event.outcome.as_ref()
            {
                for candidate in &resolution.candidates {
                    if candidate.ability_id == "demo.ability.cantor-call"
                        && candidate.rejection_reason
                            == Some(MonsterAbilityRejectionReasonDto::NoCandidates)
                    {
                        saw_empty_rejection = true;
                    }
                }
            }
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                assert_eq!(resolution.ability_id, "demo.ability.mote-call");
                let summon = resolution
                    .summon
                    .as_ref()
                    .expect("category summon should expose its resolution");
                assert_eq!(summon.actor_kind_id, "elemental");
                assert!((1..=2).contains(&summon.entity_ids.len()));
                assert_eq!(summon.summoned_kind_ids.len(), summon.entity_ids.len());
                for kind_id in &summon.summoned_kind_ids {
                    assert!(elemental_kinds.contains(&kind_id.as_str()));
                }
                for entity_id in &summon.entity_ids {
                    let entity = game
                        .entities
                        .iter()
                        .find(|entity| &entity.id == entity_id)
                        .expect("summoned entity should exist");
                    assert!(entity.summon.is_some());
                }
                assert!(
                    saw_empty_rejection,
                    "cantor-call must have been rejected with no-candidates in the same decision"
                );
                return;
            }
        }
        if game.player_is_dead() {
            break;
        }
    }
    panic!("mote binder should summon within 60 turns");
}

#[test]
fn spawned_entities_get_content_declared_resistances_stamped() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    for step in 0..=3 {
        for dy in -2..=2 {
            if let Some(index) = game.index(Position {
                x: player.x + step,
                y: player.y + dy,
            }) {
                game.terrain[index] = "demo.terrain.floor".to_owned();
            }
        }
    }
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.slag-test",
        "demo.actor.slag-crawler",
        Position {
            x: player.x + 3,
            y: player.y,
        },
        10,
        100,
        100,
        true,
    ));

    for _ in 0..60 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityCast { resolution }) =
                event.outcome.as_ref()
            {
                assert_eq!(resolution.ability_id, "demo.ability.slag-call");
                let summon = resolution
                    .summon
                    .as_ref()
                    .expect("kin summon should expose its resolution");
                let entity_id = &summon.entity_ids[0];
                let summoned = game
                    .entities
                    .iter()
                    .find(|entity| &entity.id == entity_id)
                    .expect("summoned crawler should exist");
                // The summon spawn path stamps the content-declared tiers;
                // the test-injected caster itself keeps the default profile.
                assert_eq!(
                    summoned.resistances.level(DamageType::Electricity),
                    ResistanceLevel::Resistant
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Fire),
                    ResistanceLevel::Immune
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Cold),
                    ResistanceLevel::Vulnerable
                );
                assert_eq!(
                    summoned.resistances.level(DamageType::Physical),
                    ResistanceLevel::Normal
                );
                return;
            }
        }
        if game.player_is_dead() {
            break;
        }
    }
    panic!("slag crawler should kin-summon within 60 turns");
}

#[test]
fn targeted_beam_continues_through_position_and_entity_targets() {
    let ability_id = "demo.ability.echo-lance";
    let expected_path = vec![
        Position { x: 4, y: 3 },
        Position { x: 5, y: 4 },
        Position { x: 6, y: 4 },
        Position { x: 7, y: 4 },
        Position { x: 8, y: 5 },
        Position { x: 9, y: 5 },
    ];

    for target in [
        TargetSelection::Position {
            position: Position { x: 6, y: 4 },
        },
        TargetSelection::Entity {
            entity_id: "demo.monster.ember-mote.1".to_owned(),
        },
    ] {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.learned_abilities.insert(ability_id.to_owned());
        for position in &expected_path {
            replace_terrain(&mut game, *position, "demo.terrain.floor");
        }
        let ember = game
            .entities
            .iter_mut()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .expect("ember mote should exist");
        ember.position = Position { x: 6, y: 4 };
        ember.hp = 100;
        ember.energy_need = 1_000;
        let guardian = game
            .entities
            .iter_mut()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .expect("entrance guardian should exist");
        guardian.position = Position { x: 8, y: 5 };
        guardian.hp = 100;
        guardian.energy_need = 1_000;

        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target,
            },
        );
        let beam = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("targeted beam should expose its extended line");
        assert_eq!(beam.affected_positions, expected_path);
        assert_eq!(beam.target_count, 2);
        let hit_targets = update
            .events
            .iter()
            .filter(|event| event.kind == "ability.hit")
            .map(|event| event.args["target"].as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            hit_targets,
            vec!["demo.actor.ember-mote", "demo.actor.resonant-warden"]
        );
    }
}

#[test]
fn beam_self_target_is_zero_rng_and_empty_beam_still_rolls_once() {
    let ability_id = "demo.ability.echo-lance";
    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );

    let mut empty =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    empty.learned_abilities.insert(ability_id.to_owned());
    clear_monsters(&mut empty);
    for x in 4..=9 {
        replace_terrain(&mut empty, Position { x, y: 3 }, "demo.terrain.floor");
    }
    replace_terrain(&mut empty, Position { x: 4, y: 3 }, "demo.terrain.wall");
    let draws_before = empty.rng_draw_counter();
    let update = dispatch_next(
        &mut empty,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let beam = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityBeamDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty beam should still resolve");
    assert_eq!(beam.target_count, 0);
    assert!(beam.affected_positions.is_empty());
    assert_eq!(empty.rng_draw_counter(), draws_before + 3);
}

#[test]
fn cone_damage_widens_with_lateral_falloff_and_stable_order() {
    let ability_id = "demo.ability.echo-fan";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.learned_abilities.insert(ability_id.to_owned());
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    for entity in &mut game.entities {
        entity.energy_need = 1_000;
    }
    let ember = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist");
    ember.position = Position { x: 4, y: 3 };
    ember.hp = 100;
    let guardian = game
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 2 };
    guardian.hp = 100;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cone = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful cone should expose its footprint");
    assert_eq!(cone.radius, 2);
    assert_eq!(cone.target_count, 2);
    assert_eq!(cone.affected_positions.len(), 14);
    assert_eq!(cone.affected_positions[0], Position { x: 4, y: 3 });
    assert!(cone.affected_positions.contains(&Position { x: 9, y: 1 }));
    assert!(cone.affected_positions.contains(&Position { x: 9, y: 5 }));
    assert_eq!(game.rng_draw_counter(), draws_before + 3);
    let hits = update
        .events
        .iter()
        .filter(|event| event.kind == "ability.hit")
        .collect::<Vec<_>>();
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0].args["target"], "demo.actor.ember-mote");
    assert_eq!(hits[1].args["target"], "demo.actor.resonant-warden");
    let center_damage = match hits[0].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("cone center hit should expose damage"),
    };
    let edge_damage = match hits[1].outcome.as_ref() {
        Some(GameEventOutcomeDto::Damage { resolution }) => resolution.raw_damage,
        _ => panic!("cone edge hit should expose damage"),
    };
    assert_eq!(center_damage, cone.base_raw_damage);
    assert_eq!(edge_damage, rfb_area_damage(cone.base_raw_damage, 1));

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    blocked.learned_abilities.insert(ability_id.to_owned());
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut blocked, Position { x, y }, "demo.terrain.floor");
        }
    }
    replace_terrain(&mut blocked, Position { x: 6, y: 3 }, "demo.terrain.wall");
    for entity in &mut blocked.entities {
        entity.energy_need = 1_000;
    }
    blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 5, y: 3 };
    let guardian = blocked
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
        .expect("entrance guardian should exist");
    guardian.position = Position { x: 7, y: 2 };
    let guardian_hp = guardian.hp;
    let blocked_update = dispatch_next(
        &mut blocked,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let blocked_cone = blocked_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("blocked cone should expose its footprint");
    assert_eq!(blocked_cone.target_count, 1);
    assert_eq!(
        blocked_cone.affected_positions,
        vec![
            Position { x: 4, y: 3 },
            Position { x: 5, y: 3 },
            Position { x: 5, y: 2 },
            Position { x: 5, y: 4 },
            Position { x: 5, y: 1 },
            Position { x: 5, y: 5 },
        ]
    );
    assert_eq!(
        blocked
            .entities
            .iter()
            .find(|entity| entity.id == "demo.z-entrance-guardian.resonance-descent.1")
            .map(|entity| entity.hp),
        Some(guardian_hp)
    );
}

#[test]
fn cone_damage_is_symmetric_across_all_eight_directions() {
    let directions = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];
    let expected_layer_counts = [1_usize, 1, 1, 3, 3, 5];
    for direction in directions {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.learned_abilities
            .insert("demo.ability.echo-fan".to_owned());
        clear_monsters(&mut game);
        game.player.position = Position { x: 10, y: 10 };
        for y in 0..20 {
            for x in 0..20 {
                replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
            }
        }
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: "demo.ability.echo-fan".to_owned(),
                target: TargetSelection::Direction { direction },
            },
        );
        let cone = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("cone outcome should exist");
        assert_eq!(
            cone.affected_positions.len(),
            expected_layer_counts.iter().sum::<usize>()
        );
        let (dx, dy) = direction.delta();
        let mut layer_counts = [0_usize; 6];
        let mut previous_key = None;
        for position in &cone.affected_positions {
            let offset_x = position.x - game.player.position.x;
            let offset_y = position.y - game.player.position.y;
            let layer = offset_x.abs().max(offset_y.abs());
            let lateral = (offset_x * dy - offset_y * dx).abs();
            assert!((1..=6).contains(&layer));
            assert!(offset_x * dx + offset_y * dy > 0);
            layer_counts[usize::try_from(layer - 1).expect("layer index should fit")] += 1;
            let key = (layer, lateral, position.y, position.x);
            assert!(previous_key.is_none_or(|previous| previous <= key));
            previous_key = Some(key);
        }
        assert_eq!(layer_counts, expected_layer_counts);
    }
}

#[test]
fn cone_invalid_mode_is_zero_rng_and_empty_cone_still_rolls_once() {
    let ability_id = "demo.ability.echo-fan";
    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    invalid.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut invalid.entities {
        entity.energy_need = 1_000;
    }
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng_draw_counter();
    let rejected = dispatch_next(
        &mut invalid,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: Position { x: 8, y: 3 },
            },
        },
    );
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng_draw_counter(), draws_before);
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );

    let mut empty =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    empty.learned_abilities.insert(ability_id.to_owned());
    clear_monsters(&mut empty);
    for y in 1..=5 {
        for x in 4..=9 {
            replace_terrain(&mut empty, Position { x, y }, "demo.terrain.floor");
        }
    }
    replace_terrain(&mut empty, Position { x: 4, y: 3 }, "demo.terrain.wall");
    let draws_before = empty.rng_draw_counter();
    let update = dispatch_next(
        &mut empty,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cone = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityConeDamage { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty cone should still resolve");
    assert_eq!(cone.target_count, 0);
    assert!(cone.affected_positions.is_empty());
    assert_eq!(empty.rng_draw_counter(), draws_before + 3);
}

#[test]
fn teleport_moves_to_an_exact_destination_and_round_trips() {
    let ability_id = "demo.ability.echo-step";
    let destination = Position { x: 6, y: 3 };
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    let origin = game.player.position;
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: destination,
            },
        },
    );

    let cast = ability_cast_resolution(&update);
    assert!(cast.succeeded);
    let teleport = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTeleport { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful teleport should expose its relocation");
    assert_eq!(teleport.from, origin);
    assert_eq!(teleport.to, destination);
    assert_eq!(game.player.position, destination);
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before - cast.resource_cost
    );
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == origin)
    );
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == destination)
    );
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == ability_id)
            .is_some_and(|ability| ability.teleport)
    );

    let restored = Game::from_save(game.to_save()).expect("teleport state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn teleport_rejects_blocked_occupied_and_invalid_destinations_before_rng() {
    let ability_id = "demo.ability.echo-step";

    let mut blocked =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut blocked);
    blocked.learned_abilities.insert(ability_id.to_owned());
    replace_terrain(
        &mut blocked,
        Position { x: 5, y: 3 },
        "demo.terrain.resonance-water-deep",
    );
    assert!(blocked.is_visible(Position { x: 6, y: 3 }));
    assert_teleport_target_rejected(
        &mut blocked,
        ability_id,
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );

    let mut occupied =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    occupied.learned_abilities.insert(ability_id.to_owned());
    for entity in &mut occupied.entities {
        entity.energy_need = 1_000;
    }
    occupied
        .entities
        .iter_mut()
        .find(|entity| entity.id == "demo.monster.ember-mote.1")
        .expect("ember mote should exist")
        .position = Position { x: 6, y: 3 };
    assert_teleport_target_rejected(
        &mut occupied,
        ability_id,
        TargetSelection::Position {
            position: Position { x: 6, y: 3 },
        },
    );

    let mut invalid =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut invalid);
    invalid.learned_abilities.insert(ability_id.to_owned());
    for target in [
        TargetSelection::Position {
            position: invalid.player.position,
        },
        TargetSelection::Position {
            position: Position { x: 10, y: 3 },
        },
        TargetSelection::Direction {
            direction: Direction::East,
        },
    ] {
        assert_teleport_target_rejected(&mut invalid, ability_id, target);
    }
}

#[test]
fn teleport_uses_normal_arrival_trap_semantics() {
    let ability_id = "demo.ability.echo-step";
    let destination = Position { x: 4, y: 3 };
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    replace_terrain(&mut game, destination, "demo.terrain.trap-echo-snare");
    let hp_before = game.player.hp;

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Position {
                position: destination,
            },
        },
    );

    assert_eq!(game.player.position, destination);
    assert_eq!(game.player.hp, hp_before - 2);
    let kinds = update
        .events
        .iter()
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    let teleport_index = kinds
        .iter()
        .position(|kind| *kind == "ability.teleport")
        .expect("teleport event should exist");
    let trap_index = kinds
        .iter()
        .position(|kind| *kind == "terrain.trap-triggered")
        .expect("landing trap should trigger");
    assert!(teleport_index < trap_index);
}

#[test]
fn summon_is_deterministic_owned_persistent_and_expires_by_turn() {
    let ability_id = "demo.ability.echo-companion";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .unwrap()
            .proficiency = 1600;
        for entity in &mut candidate.entities {
            entity.energy_need = 1_000;
        }
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if update
            .events
            .iter()
            .any(|event| event.kind == "ability.summon")
        {
            selected = Some((candidate, update));
            break;
        }
    }
    let (mut game, update) = selected.expect("a deterministic seed should cast successfully");
    let summon = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilitySummon { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("summon outcome should be present");
    assert_eq!(summon.entity_ids.len(), 2);
    assert_eq!(summon.positions.len(), 2);
    assert!(summon.positions[0] != summon.positions[1]);
    assert!(game.entities.iter().all(|entity| {
        entity.summon.as_ref().is_none_or(|identity| {
            identity.owner_id == game.player.id
                && identity.source_ability_id == ability_id
                && identity.remaining_turns == 4
        })
    }));
    assert!(
        game.snapshot()
            .entities
            .iter()
            .filter(|entity| entity.faction == EntityFactionDto::Player)
            .all(|entity| entity.summon.is_some())
    );

    let restored = Game::from_save(game.to_save()).expect("summon save should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot().entities, game.snapshot().entities);

    for sequence in 0..4 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        if sequence < 3 {
            assert_eq!(
                update
                    .entities
                    .iter()
                    .filter(|entity| entity.faction == EntityFactionDto::Player)
                    .count(),
                2
            );
        }
    }
    assert!(
        game.snapshot()
            .entities
            .iter()
            .all(|entity| entity.faction == EntityFactionDto::Hostile)
    );
}

#[test]
fn summon_space_rejection_is_atomic_before_mana_and_rng() {
    let ability_id = "demo.ability.echo-companion";
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.learned_abilities.insert(ability_id.to_owned());
    let origin = game.player.position;
    for y in origin.y - 2..=origin.y + 2 {
        for x in origin.x - 2..=origin.x + 2 {
            let position = Position { x, y };
            if position != origin && game.index(position).is_some() {
                replace_terrain(&mut game, position, "demo.terrain.wall");
            }
        }
    }
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
    assert!(game.entities.is_empty());
}

#[test]
fn summon_failure_costs_mana_but_does_not_create_entities() {
    let ability_id = "demo.ability.echo-companion";
    for seed in 0..128 {
        let mut game =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut game);
        game.learned_abilities.insert(ability_id.to_owned());
        for entity in &mut game.entities {
            entity.energy_need = 1_000;
        }
        let mana_before = game.resources["demo.resource.mana"].current;
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        let Some(cast) = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution),
                _ => None,
            })
        else {
            continue;
        };
        if !cast.succeeded {
            assert!(game.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(game.rng_draw_counter(), draws_before + 1);
            assert!(game.entities.is_empty());
            return;
        }
    }
    panic!("a failure seed should exist in the deterministic search range");
}

#[test]
fn detect_persistent_filters_category_visibility_and_round_trips() {
    let ability_id = "demo.ability.echo-sight";
    let visible_rune = Position { x: 4, y: 2 };
    let visible_door = Position { x: 4, y: 4 };
    let blocked_rune = Position { x: 6, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("detect ability progress should exist")
            .proficiency = 1600;
        for y in 1..=5 {
            for x in 1..=8 {
                replace_terrain(&mut candidate, Position { x, y }, "demo.terrain.floor");
            }
        }
        replace_terrain(
            &mut candidate,
            visible_rune,
            "demo.terrain.echo-rune-hidden",
        );
        replace_terrain(&mut candidate, visible_door, "demo.terrain.door-secret");
        replace_terrain(&mut candidate, Position { x: 5, y: 3 }, "demo.terrain.wall");
        replace_terrain(
            &mut candidate,
            blocked_rune,
            "demo.terrain.echo-rune-hidden",
        );
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic detect success seed should exist");
    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful detection should expose its result");
    assert_eq!(detection.category, "hidden");
    assert_eq!(detection.radius, 6);
    assert!(detection.persistent);
    assert_eq!(
        detection.detected_positions,
        vec![visible_rune, visible_door]
    );
    assert!(game.revealed_terrain.contains(&visible_rune));
    assert!(game.revealed_terrain.contains(&visible_door));
    assert!(!game.revealed_terrain.contains(&blocked_rune));
    assert_eq!(
        game.known_terrain_at(visible_rune),
        "demo.terrain.echo-rune-hidden"
    );
    assert_eq!(
        game.known_terrain_at(visible_door),
        "demo.terrain.door-secret"
    );
    assert_eq!(game.known_terrain_at(blocked_rune), "demo.terrain.wall");
    assert_eq!(
        update
            .changed_cells
            .iter()
            .map(|cell| cell.position)
            .collect::<Vec<_>>(),
        vec![visible_rune, visible_door]
    );

    let restored = Game::from_save(game.to_save()).expect("detected terrain should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn detect_transient_empty_and_invalid_targets_preserve_knowledge_boundaries() {
    let ability_id = "demo.ability.echo-pulse";
    let rune = Position { x: 4, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("detect ability progress should exist")
            .proficiency = 1600;
        replace_terrain(&mut candidate, rune, "demo.terrain.echo-rune-hidden");
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (mut game, update) = selected.expect("a deterministic detect success seed should exist");
    let detection = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("transient detection should expose its result");
    assert_eq!(detection.detected_positions, vec![rune]);
    assert!(!detection.persistent);
    assert!(!game.revealed_terrain.contains(&rune));
    assert_eq!(game.known_terrain_at(rune), "demo.terrain.wall");
    assert!(update.changed_cells.is_empty());

    replace_terrain(&mut game, rune, "demo.terrain.floor");
    let empty = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let empty_detection = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityDetect { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty detection should still resolve");
    assert!(empty_detection.detected_positions.is_empty());

    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
}

#[test]
fn terrain_transform_digging_is_stable_atomic_and_round_trips() {
    let ability_id = "demo.ability.echo-delving";
    let center = Position { x: 5, y: 3 };
    let transformed = vec![center, Position { x: 5, y: 2 }, Position { x: 4, y: 4 }];
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        for (position, terrain_id) in [
            (center, "demo.terrain.wall"),
            (Position { x: 5, y: 2 }, "demo.terrain.echo-rubble"),
            (Position { x: 6, y: 3 }, "demo.terrain.resonance-vein"),
            (Position { x: 4, y: 4 }, "demo.terrain.resonance-ruin"),
            (Position { x: 5, y: 4 }, "demo.terrain.floor"),
        ] {
            replace_terrain(&mut candidate, position, terrain_id);
        }
        candidate.revealed_terrain.insert(Position { x: 5, y: 2 });
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position { position: center },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) =
        selected.expect("a deterministic terrain transformation success should exist");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful terrain transformation should expose its result");
    assert_eq!(resolution.center, center);
    assert_eq!(resolution.radius, 1);
    assert_eq!(resolution.target_terrain_id, "demo.terrain.floor");
    assert_eq!(resolution.transformed_positions, transformed);
    for position in &transformed {
        assert_eq!(game.terrain_at(*position), "demo.terrain.floor");
    }
    assert_eq!(
        game.terrain_at(Position { x: 5, y: 4 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 6, y: 3 }),
        "demo.terrain.resonance-vein"
    );
    assert!(!game.revealed_terrain.contains(&Position { x: 5, y: 2 }));
    assert_eq!(
        update
            .changed_cells
            .iter()
            .map(|cell| cell.position)
            .collect::<BTreeSet<_>>(),
        transformed.iter().copied().collect()
    );

    let restored = Game::from_save(game.to_save()).expect("transformed terrain should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn terrain_transform_creation_filters_occupied_connections_and_borders() {
    let ability_id = "demo.ability.echo-rampart";
    let center = Position { x: 3, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        candidate.entities.truncate(1);
        candidate.entities[0].position = Position { x: 2, y: 3 };
        candidate.entities[0].energy_need = i32::MAX / 2;
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        for position in [
            Position { x: 2, y: 2 },
            Position { x: 3, y: 2 },
            Position { x: 4, y: 2 },
            Position { x: 2, y: 3 },
            Position { x: 3, y: 3 },
            Position { x: 4, y: 3 },
            Position { x: 2, y: 4 },
            Position { x: 4, y: 4 },
        ] {
            replace_terrain(&mut candidate, position, "demo.terrain.floor");
        }
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position { position: center },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic terrain creation success should exist");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("successful terrain creation should expose its result");
    let expected = vec![
        Position { x: 2, y: 2 },
        Position { x: 3, y: 2 },
        Position { x: 4, y: 2 },
        Position { x: 2, y: 4 },
        Position { x: 4, y: 4 },
    ];
    assert_eq!(resolution.transformed_positions, expected);
    for position in &expected {
        assert_eq!(game.terrain_at(*position), "demo.terrain.echo-rubble");
    }
    assert_eq!(game.terrain_at(center), "demo.terrain.floor");
    assert_eq!(
        game.terrain_at(Position { x: 2, y: 3 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 4, y: 3 }),
        "demo.terrain.floor"
    );
    assert_eq!(
        game.terrain_at(Position { x: 3, y: 4 }),
        "demo.terrain.stairs-down"
    );

    let ability = game
        .content
        .ability(ability_id)
        .expect("terrain creation ability should exist");
    assert!(
        game.terrain_transform_positions(
            ability,
            center,
            &["demo.terrain.stairs-down".to_owned()],
            "demo.terrain.echo-rubble",
            1,
        )
        .expect("the current cell should be a valid target")
        .is_empty()
    );
    let border_game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let border_ability = border_game
        .content
        .ability(ability_id)
        .expect("terrain creation ability should exist");
    assert!(
        border_game
            .terrain_transform_positions(
                border_ability,
                Position { x: 1, y: 1 },
                &["demo.terrain.wall".to_owned()],
                "demo.terrain.echo-rubble",
                1,
            )
            .expect("the near-border cell should be a valid target")
            .is_empty()
    );
}

#[test]
fn terrain_transform_empty_invalid_and_failure_preserve_rng_boundaries() {
    let ability_id = "demo.ability.echo-delving";
    let empty_center = Position { x: 8, y: 3 };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("terrain transform progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        let mana_before = candidate.resources["demo.resource.mana"].current;
        let draws_before = candidate.rng_draw_counter();
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: empty_center,
                },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update, mana_before, draws_before));
            break;
        }
    }
    let (mut game, empty, mana_before, draws_before) =
        selected.expect("a deterministic empty terrain transformation should succeed");
    let resolution = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityTerrainTransform { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty terrain transformation should still resolve");
    assert!(resolution.transformed_positions.is_empty());
    assert!(game.resources["demo.resource.mana"].current < mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before + 1);
    assert!(empty.changed_cells.is_empty());

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let progress_before_rejection = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert_eq!(game.ability_progress[ability_id], progress_before_rejection);

    for seed in 0..128 {
        let mut failure =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut failure);
        failure
            .items
            .retain(|item| !matches!(item.location, ItemLocation::Ground(_)));
        failure.learned_abilities.insert(ability_id.to_owned());
        replace_terrain(&mut failure, Position { x: 5, y: 3 }, "demo.terrain.wall");
        let terrain_before = failure.terrain.clone();
        let mana_before = failure.resources["demo.resource.mana"].current;
        let update = dispatch_next(
            &mut failure,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: Position { x: 5, y: 3 },
                },
            },
        );
        if !ability_cast_resolution(&update).succeeded {
            assert!(failure.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(failure.terrain, terrain_before);
            assert!(!update.events.iter().any(|event| {
                matches!(
                    event.outcome,
                    Some(GameEventOutcomeDto::AbilityTerrainTransform { .. })
                )
            }));
            return;
        }
    }
    panic!("a terrain transformation failure seed should exist");
}

#[test]
fn self_status_sequence_applies_in_order_and_round_trips() {
    let ability_id = "demo.ability.echo-quickening";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        candidate.player.statuses.push(StatusInstance {
            kind_id: STATUS_SLOW.to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some("test.slow".to_owned()),
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto::default(),
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update));
            break;
        }
    }
    let (game, update) = selected.expect("a deterministic self status cast should succeed");
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("status sequence should expose its ordered effects");
    assert_eq!(
        resolution.target_entity_id.as_deref(),
        Some("demo.actor.player.1")
    );
    assert_eq!(resolution.effects.len(), 2);
    assert!(matches!(
        resolution.effects[0],
        AbilityEffectResolutionDto::ApplyStatus {
            effect_index: 0,
            change: AbilityStatusChangeDto::Added,
            applied_duration_ticks: 30,
            ..
        }
    ));
    assert!(matches!(
        resolution.effects[1],
        AbilityEffectResolutionDto::RemoveStatus {
            effect_index: 1,
            removed: true,
            ..
        }
    ));
    assert!(
        game.player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_HASTE)
    );
    assert!(
        game.player
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLOW)
    );
    let ability = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == ability_id)
        .expect("status sequence should be projected");
    assert!(matches!(
        ability.effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus { .. },
            AbilityEffectSpecDto::RemoveStatus { .. }
        ]
    ));

    let restored = Game::from_save(game.to_save()).expect("status ability state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn target_status_sequence_resists_immunizes_and_skips_after_death() {
    let ability_id = "demo.ability.echo-binding";
    let prepare = |seed: u64, hp: i32, resistance: ResistanceLevel| {
        let mut game =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        let entity = game.entities[0].clone();
        clear_monsters(&mut game);
        game.entities.push(entity);
        game.entities[0].position = Position { x: 4, y: 3 };
        game.entities[0].hp = hp;
        game.entities[0].energy_need = STANDARD_ACTION_COST;
        game.entities[0]
            .resistances
            .set(DamageType::Cold, resistance);
        game.learned_abilities.insert(ability_id.to_owned());
        game.ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        game
    };
    let seed = (0..128)
        .find(|seed| {
            let mut game = prepare(*seed, 3, ResistanceLevel::Normal);
            let target_id = game.entities[0].id.clone();
            let update = dispatch_next(
                &mut game,
                GameCommand::CastAbility {
                    ability_id: ability_id.to_owned(),
                    target: TargetSelection::Entity {
                        entity_id: target_id,
                    },
                },
            );
            ability_cast_resolution(&update).succeeded
        })
        .expect("a deterministic target status cast should succeed");

    let mut resistant = prepare(seed, 3, ResistanceLevel::Resistant);
    let target_id = resistant.entities[0].id.clone();
    let resistant_update = dispatch_next(
        &mut resistant,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let resistant_resolution = resistant_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("resisted status sequence should resolve");
    assert!(matches!(
        resistant_resolution.effects[1],
        AbilityEffectResolutionDto::ApplyStatus {
            effect_index: 1,
            requested_duration_ticks: 30,
            applied_duration_ticks: 15,
            resistance: Some(ResistanceLevelDto::Resistant),
            change: AbilityStatusChangeDto::Added,
            ..
        }
    ));
    assert!(
        resistant
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
            .is_some_and(|entity| entity
                .statuses
                .iter()
                .any(|status| status.kind_id == STATUS_SLOW))
    );
    let restored = Game::from_save(resistant.to_save()).expect("resisted status should round-trip");
    assert_eq!(restored.snapshot(), resistant.snapshot());

    let mut immune = prepare(seed, 3, ResistanceLevel::Immune);
    let target_id = immune.entities[0].id.clone();
    let immune_update = dispatch_next(
        &mut immune,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let immune_resolution = immune_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("immune status sequence should resolve");
    assert!(matches!(
        immune_resolution.effects[1],
        AbilityEffectResolutionDto::ApplyStatus {
            applied_duration_ticks: 0,
            resistance: Some(ResistanceLevelDto::Immune),
            change: AbilityStatusChangeDto::Immune,
            ..
        }
    ));
    assert!(
        immune
            .entities
            .iter()
            .find(|entity| entity.id == target_id)
            .is_some_and(|entity| entity.statuses.is_empty())
    );

    let mut lethal = prepare(seed, 1, ResistanceLevel::Normal);
    let target_id = lethal.entities[0].id.clone();
    let lethal_update = dispatch_next(
        &mut lethal,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::Entity {
                entity_id: target_id.clone(),
            },
        },
    );
    let lethal_resolution = lethal_update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("lethal sequence should resolve");
    assert!(matches!(
        lethal_resolution.effects[0],
        AbilityEffectResolutionDto::Damage {
            effect_index: 0,
            ..
        }
    ));
    assert!(matches!(
        lethal_resolution.effects[1],
        AbilityEffectResolutionDto::Skipped {
            effect_index: 1,
            reason: AbilityEffectSkipReasonDto::TargetDead,
        }
    ));
    assert!(lethal.entities.iter().all(|entity| entity.id != target_id));
}

#[test]
fn actor_effect_sequences_preserve_empty_invalid_and_failure_rng_boundaries() {
    let ability_id = "demo.ability.echo-binding";
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        clear_monsters(&mut candidate);
        candidate.learned_abilities.insert(ability_id.to_owned());
        candidate
            .ability_progress
            .get_mut(ability_id)
            .expect("status ability progress should exist")
            .proficiency = SPELL_EXP_MASTER;
        let mana_before = candidate.resources["demo.resource.mana"].current;
        let draws_before = candidate.rng_draw_counter();
        let update = dispatch_next(
            &mut candidate,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Position {
                    position: Position { x: 6, y: 3 },
                },
            },
        );
        if ability_cast_resolution(&update).succeeded {
            selected = Some((candidate, update, mana_before, draws_before));
            break;
        }
    }
    let (mut game, empty, mana_before, draws_before) =
        selected.expect("a deterministic empty effect sequence should succeed");
    let resolution = empty
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityEffects { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("empty effect sequence should expose skipped effects");
    assert!(resolution.target_entity_id.is_none());
    assert!(resolution.effects.iter().all(|effect| matches!(
        effect,
        AbilityEffectResolutionDto::Skipped {
            reason: AbilityEffectSkipReasonDto::NoTarget,
            ..
        }
    )));
    assert!(game.resources["demo.resource.mana"].current < mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before + 1);

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let progress_before_rejection = game.ability_progress[ability_id];
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(
        rejected
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert_eq!(game.ability_progress[ability_id], progress_before_rejection);

    for seed in 0..128 {
        let mut failure =
            Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
        failure.entities.truncate(1);
        failure.entities[0].position = Position { x: 4, y: 3 };
        failure.entities[0].energy_need = i32::MAX / 2;
        failure.learned_abilities.insert(ability_id.to_owned());
        let target_id = failure.entities[0].id.clone();
        let mana_before = failure.resources["demo.resource.mana"].current;
        let draws_before = failure.rng_draw_counter();
        let update = dispatch_next(
            &mut failure,
            GameCommand::CastAbility {
                ability_id: ability_id.to_owned(),
                target: TargetSelection::Entity {
                    entity_id: target_id,
                },
            },
        );
        if !ability_cast_resolution(&update).succeeded {
            assert!(failure.resources["demo.resource.mana"].current < mana_before);
            assert_eq!(failure.rng_draw_counter(), draws_before + 1);
            assert!(failure.entities[0].statuses.is_empty());
            assert!(
                !update
                    .events
                    .iter()
                    .any(|event| event.kind == "ability.effects")
            );
            return;
        }
    }
    panic!("an effect sequence failure seed should exist");
}

#[test]
fn learning_capacity_forget_and_relearn_preserve_ability_progress() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    let echo_primer = ability_book_item_id_for(&game, "demo.item.echo-primer");
    let stillwater_notes = ability_book_item_id_for(&game, "demo.item.stillwater-notes");
    let initial = game.snapshot();
    assert_eq!(
        initial.player.ability_learning,
        Some(AbilityLearningDto {
            learned_count: 0,
            capacity: 2,
            remaining_slots: 2,
        })
    );
    assert_eq!(initial.player.abilities.len(), 46);
    assert_eq!(
        initial
            .player
            .abilities
            .iter()
            .filter(|ability| ability.can_study)
            .count(),
        22
    );
    assert!(
        initial
            .player
            .abilities
            .iter()
            .all(|ability| !ability.can_forget)
    );

    for (book_item_id, ability_id) in [
        (echo_primer.clone(), "demo.ability.resonant-bolt"),
        (stillwater_notes, "demo.ability.mending-echo"),
    ] {
        dispatch_next(
            &mut game,
            GameCommand::StudyAbility {
                book_item_id,
                ability_id: ability_id.to_owned(),
            },
        );
    }
    let full = game.snapshot();
    assert_eq!(
        full.player.ability_learning,
        Some(AbilityLearningDto {
            learned_count: 2,
            capacity: 2,
            remaining_slots: 0,
        })
    );
    assert!(
        full.player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.harmonic-spark")
            .is_some_and(|ability| !ability.can_study)
    );

    let draws_before_rejection = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: echo_primer.clone(),
            ability_id: "demo.ability.harmonic-spark".to_owned(),
        },
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.study-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "learning-capacity-full")
    }));

    let retained_progress = AbilityProgress {
        proficiency: SPELL_EXP_EXPERT,
        proficiency_cap: SPELL_EXP_MASTER,
        cast_count: 12,
        fail_count: 3,
        cooldown_remaining: 0,
    };
    game.ability_progress
        .insert("demo.ability.resonant-bolt".to_owned(), retained_progress);
    let forgotten = dispatch_next(
        &mut game,
        GameCommand::ForgetAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        forgotten
            .events
            .iter()
            .any(|event| event.kind == "ability.forgotten")
    );
    assert_eq!(
        game.ability_progress["demo.ability.resonant-bolt"],
        retained_progress
    );
    let after_forget = game.snapshot();
    assert_eq!(
        after_forget
            .player
            .ability_learning
            .unwrap()
            .remaining_slots,
        1
    );
    assert!(
        after_forget
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.resonant-bolt")
            .is_some_and(|ability| !ability.learned && !ability.can_forget)
    );

    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: echo_primer,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    let relearned = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("relearned ability should remain projected");
    assert!(relearned.learned);
    assert_eq!(relearned.proficiency, SPELL_EXP_EXPERT);
    assert_eq!(relearned.cast_count, 12);
    assert_eq!(relearned.fail_count, 3);

    let restored = Game::from_save(game.to_save()).expect("forgotten progress should reload");
    assert_eq!(restored.state_hash(), game.state_hash());

    let mut over_capacity = Game::new_with_build(0, "demo.build.scholar")
        .expect("scholar build should create")
        .to_save();
    over_capacity.player.learned_ability_ids = vec![
        "demo.ability.harmonic-spark".to_owned(),
        "demo.ability.mending-echo".to_owned(),
        "demo.ability.resonant-bolt".to_owned(),
    ];
    assert!(matches!(
        Game::from_save(over_capacity),
        Err(CoreError::InvalidSave(
            "learned ability set exceeds learning capacity"
        ))
    ));
}

#[test]
fn class_casting_overrides_drive_study_cast_projection_and_save_validation() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let profile = artifact
        .content
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("demo mage should have a casting profile");
    profile
        .ability_overrides
        .push(rfb_content::AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            minimum_level: 2,
            resource_cost: 9,
            base_failure_percent: 47,
            level_scaling: Vec::new(),
        });
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("casting override content should remain valid"),
    ));
    let mut game = Game::from_content_with_build(
        0,
        Arc::clone(&catalog),
        BUILT_IN_WORLD_ID,
        "demo.build.scholar",
    )
    .expect("custom scholar build should create");
    let book_item_id = ability_book_item_id(&game);

    let initial = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("override ability should be projected");
    assert_eq!(initial.minimum_level, 2);
    assert_eq!(initial.base_resource_cost, 9);
    assert!(!initial.can_study);

    game.apply_player_experience(10, &mut Vec::new());
    assert_eq!(game.progress.level, 2);
    let available = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.resonant-bolt")
        .expect("override ability should remain projected");
    assert!(available.can_study);
    assert_ne!(available.failure_percent, 20);

    let studied = dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    assert!(
        studied
            .events
            .iter()
            .any(|event| event.kind == "ability.studied")
    );

    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&cast);
    assert_eq!(resolution.base_resource_cost, 9);
    assert_eq!(resolution.resource_cost, available.resource_cost);
    assert_eq!(resolution.failure_percent, available.failure_percent);

    let snapshot = game.snapshot();
    let restored = Game::from_save_with_content(game.to_save(), catalog)
        .expect("learned override ability should reload against the same content");
    assert_eq!(restored.snapshot(), snapshot);
}

#[test]
fn death_abilities_materialize_player_level_scaling_in_projection() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.progress.level = 11;
    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        abilities["demo.ability.death-malediction"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Damage {
            damage_dice: 5,
            damage_sides: 4,
            damage_bonus: 0,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-stinking-cloud"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 14,
            radius: 2,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-black-sleep"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            power: Some(22),
            duration_ticks: 500,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-horrify"].effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                power: Some(22),
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 7,
                ..
            }
        ]
    ));
    assert!(matches!(
        abilities["demo.ability.death-enslave-undead"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Control { power: 22, .. }]
    ));
}

#[test]
fn death_second_book_materializes_original_mage_scaling_and_beam_profile() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    game.progress.level = 30;
    let abilities = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .map(|ability| (ability.id.clone(), ability))
        .collect::<BTreeMap<_, _>>();

    assert!(matches!(
        abilities["demo.ability.death-entropy-orb"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 3,
            damage_sides: 6,
            damage_bonus: 45,
            radius: 3,
            target_category: Some(category),
            ..
        }] if category == "living"
    ));
    assert!(matches!(
        abilities["demo.ability.death-nether-bolt"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 14,
            damage_sides: 8,
            beam_chance_percent: 30,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-cloud-kill"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 119,
            radius: 5,
            ..
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-genocide-one"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Genocide {
            scope: AbilityGenocideScopeDto::Single,
            power: 90,
            radius: 0,
        }]
    ));
    assert!(matches!(
        abilities["demo.ability.death-poison-branding"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus { granted_brands, .. }]
            if granted_brands == &[WeaponBrandDto::Poison]
    ));

    game.progress.level = 32;
    let vampiric_drain = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.death-vampiric-drain")
        .expect("vampiric drain should be projected");
    assert!(matches!(
        vampiric_drain.effects.as_slice(),
        [AbilityEffectSpecDto::DrainLife {
            damage_dice: 1,
            damage_sides: 64,
            damage_bonus: 64,
            target_category,
            ..
        }] if target_category == "living"
    ));
}

fn prepare_death_caster(seed: u64, level: u16, ability_id: &str) -> Game {
    let mut game =
        Game::new_with_build(seed, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.progress.level = level;
    game.progress.max_level = level;
    game.learned_abilities.insert(ability_id.to_owned());
    game.ability_progress
        .get_mut(ability_id)
        .expect("Death ability progress should exist")
        .proficiency = SPELL_EXP_MASTER;
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have Mana");
    mana.current = 1_000;
    mana.maximum = 1_000;
    game
}

#[test]
fn death_third_book_materializes_original_scaling_and_prorated_cap() {
    let projected = |level| {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.progress.level = level;
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>()
    };

    let level_40 = projected(40);
    assert!(matches!(
        level_40["demo.ability.death-berserk"].effects.as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_dice: 1,
                duration_sides: 25,
                granted_modifiers,
                granted_equipment_bonuses,
                granted_status_immunities,
                ..
            },
            AbilityEffectSpecDto::Heal { amount: 30 },
        ] if granted_modifiers.max_hp == 30
            && granted_modifiers.defense == -10
            && granted_equipment_bonuses.melee_damage == 11
            && granted_status_immunities == &["rfb.status.fear".to_owned()]
    ));
    assert!(matches!(
        level_40["demo.ability.death-dark-bolt"].effects.as_slice(),
        [AbilityEffectSpecDto::BoltOrBeamDamage {
            damage_dice: 12,
            damage_sides: 8,
            beam_chance_percent: 40,
            ..
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-battle-frenzy"]
            .effects
            .as_slice(),
        [
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_sides: 25,
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 25,
                duration_sides: 25,
                ..
            },
            AbilityEffectSpecDto::ApplyStatus {
                duration_ticks: 20,
                duration_sides: 40,
                ..
            },
        ]
    ));
    assert!(matches!(
        level_40["demo.ability.death-vampirism-true"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::DrainLife { repeat: 3, .. }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-nether-wave"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::VisibleDamage {
            damage_dice: 1,
            damage_sides: 120,
            target_category: Some(category),
            ..
        }] if category == "living"
    ));
    assert!(matches!(
        level_40["demo.ability.death-darkness-storm"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_dice: 1,
            damage_sides: 1,
            damage_bonus: 222,
            radius: 4,
            ..
        }]
    ));

    for level in [50, 100] {
        let abilities = projected(level);
        let expected_nether_sides = level * 3;
        assert!(matches!(
            abilities["demo.ability.death-nether-wave"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::VisibleDamage { damage_sides, .. }]
                if *damage_sides == expected_nether_sides
        ));
        assert!(matches!(
            abilities["demo.ability.death-darkness-storm"]
                .effects
                .as_slice(),
            [AbilityEffectSpecDto::AreaDamage {
                damage_bonus: 299,
                ..
            }]
        ));
    }
}

#[test]
fn berserk_and_battle_frenzy_roll_independent_durations_and_round_trip() {
    let mut left = prepare_death_caster(41, 40, "demo.ability.death-berserk");
    let mut right = left.clone();
    for game in [&mut left, &mut right] {
        game.player.hp = 1;
        let mut ability = game
            .content
            .ability("demo.ability.death-berserk")
            .expect("Berserk should exist")
            .clone();
        Game::apply_player_level_scaling(&mut ability, 40);
        game.resolve_ability_actor_effects(
            &ability.id,
            &ability.effect,
            AbilityTargetPlan::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Berserk should resolve");
    }
    assert_eq!(left.state_hash(), right.state_hash());
    assert_eq!(left.player.hp, 31);
    assert!(left.player_status_immunities().contains(STATUS_FEAR));
    let berserk = left
        .player
        .statuses
        .iter()
        .find(|status| status.kind_id == "rfb.status.berserk")
        .expect("Berserk status should be active");
    assert!((26..=50).contains(&berserk.remaining_ticks));
    assert_eq!(berserk.granted_modifiers.max_hp, 30);
    assert_eq!(berserk.granted_equipment_bonuses.melee_damage, 11);
    left.progress.level = 1;
    left.progress.max_level = 1;
    left.learned_abilities.remove("demo.ability.death-berserk");
    let level_one_mana = Game::new_with_build(0, "demo.build.scholar")
        .expect("level-one scholar should create")
        .resources["demo.resource.mana"]
        .maximum;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana")
        .current = level_one_mana;
    left.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana")
        .maximum = level_one_mana;
    assert_eq!(
        Game::from_save(left.to_save())
            .expect("Berserk should reload")
            .state_hash(),
        left.state_hash()
    );

    let mut frenzy = prepare_death_caster(53, 40, "demo.ability.death-battle-frenzy");
    let mut expected_rng = frenzy.rng.clone();
    let expected = [
        26 + u32::try_from(expected_rng.bounded(25)).unwrap(),
        26 + u32::try_from(expected_rng.bounded(25)).unwrap(),
        21 + u32::try_from(expected_rng.bounded(40)).unwrap(),
    ];
    let mut ability = frenzy
        .content
        .ability("demo.ability.death-battle-frenzy")
        .expect("Battle Frenzy should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 40);
    frenzy
        .resolve_ability_actor_effects(
            &ability.id,
            &ability.effect,
            AbilityTargetPlan::SelfTarget,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Battle Frenzy should resolve");
    let durations = ["rfb.status.hero", "rfb.status.blessed", STATUS_HASTE].map(|kind_id| {
        frenzy
            .player
            .statuses
            .iter()
            .find(|status| status.kind_id == kind_id)
            .expect("Frenzy status should be active")
            .remaining_ticks
    });
    assert_eq!(durations, expected);
    assert_eq!(frenzy.rng, expected_rng);
}

#[test]
fn vampiric_branding_is_permanent_and_only_the_source_weapon_drains_life() {
    let ability_id = "demo.ability.death-vampiric-branding";
    let mut branded = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 34, ability_id);
        game.items.push(ItemInstance {
            id: "test.item.branding-blade".to_owned(),
            kind_id: "demo.item.echo-blade".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        });
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Vampiric Branding should resolve");
        if events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::EnchantEquippedWeapon {
                            added: true,
                            affix_id,
                            ..
                        }] if affix_id == "demo.affix.vampiric"
                    )
            )
        }) {
            branded = Some(game);
            break;
        }
    }
    let game = branded.expect("a deterministic branding cast should succeed");
    let weapon = game
        .items
        .iter()
        .find(|item| item.id == "test.item.branding-blade")
        .expect("branded weapon should remain equipped");
    assert_eq!(weapon.affix_ids, ["demo.affix.vampiric"]);
    let knowledge = game
        .item_property_knowledge
        .get(&weapon.id)
        .expect("branding should identify the weapon");
    assert!(knowledge.appraised && knowledge.identified);
    assert!(knowledge.known_affix_ids.contains("demo.affix.vampiric"));
    let mut game = game;
    game.progress.level = 1;
    game.progress.max_level = 1;
    game.learned_abilities.remove(ability_id);
    let level_one_mana = Game::new_with_build(0, "demo.build.scholar")
        .expect("level-one scholar should create")
        .resources["demo.resource.mana"]
        .maximum;
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should keep Mana");
    mana.current = level_one_mana;
    mana.maximum = level_one_mana;
    let restored = Game::from_save(game.to_save()).expect("branding should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .item_passives(
                restored
                    .items
                    .iter()
                    .find(|item| item.id == "test.item.branding-blade")
                    .unwrap()
            )
            .contains(&EquipmentPassive::Vampiric)
    );

    let prepare_melee = |seed, weapon_vampiric: bool| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.items.push(ItemInstance {
            id: "test.item.melee-blade".to_owned(),
            kind_id: "demo.item.echo-blade".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Fine,
            affix_ids: weapon_vampiric
                .then(|| "demo.affix.vampiric".to_owned())
                .into_iter()
                .collect(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
        });
        if !weapon_vampiric {
            game.items.push(ItemInstance {
                id: "test.item.vampiric-charm".to_owned(),
                kind_id: "demo.item.echo-charm".to_owned(),
                quantity: 1,
                quality: ItemQualityDto::Fine,
                affix_ids: vec!["demo.affix.vampiric".to_owned()],
                rolled_affixes: Vec::new(),
                enchantments: Default::default(),
                curse: None,
                activation: None,
                charges: None,
                device_recovery_progress: 0,
                location: ItemLocation::Equipped {
                    slot_id: "charm".to_owned(),
                },
            });
        }
        game.player.statuses.push(StatusInstance {
            kind_id: "test.status.melee-power".to_owned(),
            intensity: 1,
            remaining_ticks: 10,
            source_id: None,
            granted_resistances: BTreeMap::new(),
            granted_brands: BTreeSet::new(),
            granted_modifiers: StatModifiersDto::default(),
            granted_equipment_bonuses: EquipmentBonusesDto {
                melee_skill: 1_000,
                melee_damage: 20,
                ..EquipmentBonusesDto::default()
            },
            granted_status_immunities: BTreeSet::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let definition = game
            .content
            .actor("demo.actor.gloom-weaver")
            .expect("living target should exist")
            .clone();
        let position = Position { x: 4, y: 3 };
        replace_terrain(&mut game, position, "demo.terrain.floor");
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.vampiric-target",
            &definition.id,
            position,
            500,
            definition.speed,
            100,
            true,
        ));
        game.player.hp = 1;
        game
    };
    let mut selected = None;
    for seed in 0..128 {
        let mut candidate = prepare_melee(seed, true);
        let mut events = Vec::new();
        candidate
            .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
            .expect("vampiric melee should resolve");
        if events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerVampiricHealed { .. }))
        {
            selected = Some((seed, candidate, events));
            break;
        }
    }
    let (seed, drained, events) = selected.expect("a deterministic melee hit should drain life");
    assert!(drained.player.hp > 1);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::PlayerVampiricHealed { resolution }
            if resolution.requested <= 50 && resolution.applied > 0
    )));

    let mut charm_only = prepare_melee(seed, false);
    let mut events = Vec::new();
    charm_only
        .resolve_player_melee(0, &mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("charm-only melee should resolve");
    assert_eq!(charm_only.player.hp, 1);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerVampiricHealed { .. }))
    );
}

#[test]
fn vampirism_true_retraces_the_path_after_each_kill() {
    let ability_id = "demo.ability.death-vampirism-true";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 36, ability_id);
        for (ordinal, x) in [4, 5, 6].into_iter().enumerate() {
            let position = Position { x, y: 3 };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                &format!("test.actor.drain-{ordinal}"),
                "demo.actor.gloom-weaver",
                position,
                7,
                100,
                100,
                true,
            ));
        }
        game.player.hp = 1;
        let mut events = Vec::new();
        let mut removed = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        )
        .expect("Vampirism True should resolve");
        if removed.len() == 3 {
            selected = Some((game, events, removed));
            break;
        }
    }
    let (game, events, removed) = selected.expect("a deterministic triple drain should succeed");
    assert_eq!(removed.len(), 3);
    assert!(game.entities.is_empty());
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::DrainLife { .. }]
                    )
            ))
            .count(),
        3
    );
}

#[test]
fn nether_wave_uses_one_roll_for_visible_living_targets() {
    let ability_id = "demo.ability.death-nether-wave";
    let mut selected = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 38, ability_id);
        for (id, kind_id, position) in [
            (
                "test.actor.wave-living-a",
                "demo.actor.gloom-weaver",
                Position { x: 4, y: 3 },
            ),
            (
                "test.actor.wave-living-b",
                "demo.actor.gloom-weaver",
                Position { x: 3, y: 4 },
            ),
            (
                "test.actor.wave-nonliving",
                "demo.actor.resonant-warden",
                Position { x: 2, y: 3 },
            ),
        ] {
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                id, kind_id, position, 500, 100, 100, true,
            ));
        }
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Nether Wave should resolve");
        if let Some(raw_damage) = events.iter().find_map(|event| match event {
            DomainEvent::AbilityVisibleDamage { resolution, .. } => {
                Some(resolution.base_raw_damage)
            }
            _ => None,
        }) {
            selected = Some((game, raw_damage));
            break;
        }
    }
    let (game, raw_damage) = selected.expect("a deterministic Nether Wave should succeed");
    assert!(raw_damage > 0);
    assert_eq!(game.entities[0].hp, 500 - raw_damage);
    assert_eq!(game.entities[1].hp, 500 - raw_damage);
    assert_eq!(game.entities[2].hp, 500);
}

#[test]
fn invoke_spirits_records_deterministic_random_no_op_branches() {
    let ability_id = "demo.ability.death-invoke-spirits";
    let cast = |seed| {
        let mut game = prepare_death_caster(seed, 10, ability_id);
        let mut events = Vec::new();
        game.resolve_player_ability(
            ability_id,
            TargetSelection::Direction {
                direction: Direction::East,
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Invoke Spirits should resolve");
        (game, events)
    };
    let seed = (0..512)
        .find(|seed| {
            let (_, events) = cast(*seed);
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::AbilityEffectsResolved { resolution, .. }
                        if matches!(
                            resolution.effects.as_slice(),
                            [AbilityEffectResolutionDto::NoOp { .. }]
                        )
                )
            })
        })
        .expect("a deterministic Invoke Spirits no-op branch should exist");
    let (left, left_events) = cast(seed);
    let (right, right_events) = cast(seed);
    assert_eq!(left_events, right_events);
    assert_eq!(left.state_hash(), right.state_hash());
    assert!(left_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RandomChoice { roll, branch_index, .. }]
                    if *roll > 0 && matches!(*branch_index, 3 | 7)
            )
    )));
    assert!(left_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::NoOp { reason, .. }]
                    if reason.ends_with("-pending")
            )
    )));
}

#[test]
fn bolt_or_beam_damage_uses_one_roll_and_changes_only_penetration() {
    let make_game = || {
        let mut game = Game::new(7);
        clear_monsters(&mut game);
        for (id, x) in [("test.actor.near", 4), ("test.actor.far", 5)] {
            let definition = game
                .content
                .actor("demo.actor.gloom-weaver")
                .expect("demo living target")
                .clone();
            let position = Position { x, y: 3 };
            replace_terrain(&mut game, position, "demo.terrain.floor");
            game.entities.push(actor_from_runtime_spawn(
                id,
                &definition.id,
                position,
                definition.max_hp,
                definition.speed,
                100,
                true,
            ));
        }
        game
    };
    let path = vec![Position { x: 4, y: 3 }, Position { x: 5, y: 3 }];

    let mut beam = make_game();
    let initial_hp = beam.entities[0].hp;
    let mut beam_events = Vec::new();
    beam.resolve_ability_bolt_or_beam(
        "test.ability.beam",
        path.clone(),
        1,
        1,
        3,
        DamageType::Physical,
        100,
        &mut beam_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("beam should resolve");
    assert!(beam.entities.iter().all(|actor| actor.hp < initial_hp));
    assert!(beam_events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityBeamDamage { resolution, .. } if resolution.target_count == 2
    )));

    let mut bolt = make_game();
    let mut bolt_events = Vec::new();
    bolt.resolve_ability_bolt_or_beam(
        "test.ability.bolt",
        path,
        1,
        1,
        3,
        DamageType::Physical,
        0,
        &mut bolt_events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("bolt should resolve");
    assert!(bolt.entities[0].hp < initial_hp);
    assert_eq!(bolt.entities[1].hp, initial_hp);
    assert!(
        !bolt_events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityBeamDamage { .. }))
    );
}

#[test]
fn cloud_kill_centers_on_the_caster_and_entropy_filters_nonliving_targets() {
    let game = Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let cloud_kill = game
        .content
        .ability("demo.ability.death-cloud-kill")
        .expect("cloud kill should exist");
    let plan = game
        .ability_target_plan(cloud_kill, &TargetSelection::SelfTarget)
        .expect("cloud kill self target should plan");
    assert!(matches!(
        plan,
        AbilityTargetPlan::Projectile {
            ref path,
            stop_at_actor: false,
        } if path.is_empty()
    ));
    let (trace, _) = game.trace_projectile_path_with_actor_policy(Vec::new(), false);
    assert_eq!(trace.landing, game.player.position);

    let mut filtered = Game::new(0);
    clear_monsters(&mut filtered);
    let center = Position { x: 5, y: 3 };
    for (id, kind_id, position) in [
        ("test.actor.living", "demo.actor.gloom-weaver", center),
        (
            "test.actor.nonliving",
            "demo.actor.resonant-warden",
            Position { x: 6, y: 3 },
        ),
    ] {
        let definition = filtered.content.actor(kind_id).expect("demo actor").clone();
        replace_terrain(&mut filtered, position, "demo.terrain.floor");
        filtered.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    let (_, targets) = filtered.area_damage_targets(center, 2, Some("living"));
    assert_eq!(targets, vec![("test.actor.living".to_owned(), 0)]);
}

#[test]
fn vampiric_drain_heals_actual_life_and_rejects_nonliving_targets() {
    let mut game = Game::new(11);
    clear_monsters(&mut game);
    let definition = game
        .content
        .actor("demo.actor.gloom-weaver")
        .expect("demo living target")
        .clone();
    let position = Position { x: 4, y: 3 };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.living",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    game.player.hp = 1;
    let mut events = Vec::new();
    game.resolve_ability_drain_life(
        "test.ability.vampiric-drain",
        vec![position],
        1,
        1,
        99,
        DamageType::Physical,
        "living",
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("vampiric drain should resolve");
    assert_eq!(game.player.hp, 1 + definition.max_hp);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::DrainLife { healing, .. }]
                    if healing.requested == definition.max_hp
                        && healing.applied == definition.max_hp
            )
    )));

    let mut nonliving = Game::new(11);
    clear_monsters(&mut nonliving);
    let definition = nonliving
        .content
        .actor("demo.actor.resonant-warden")
        .expect("demo nonliving target")
        .clone();
    replace_terrain(&mut nonliving, position, "demo.terrain.floor");
    nonliving.entities.push(actor_from_runtime_spawn(
        "test.actor.nonliving",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    let hp_before = nonliving.entities[0].hp;
    let draws_before = nonliving.rng.draw_counter;
    let mut events = Vec::new();
    nonliving
        .resolve_ability_drain_life(
            "test.ability.vampiric-drain",
            vec![position],
            1,
            1,
            99,
            DamageType::Physical,
            "living",
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("ineligible drain should still resolve");
    assert_eq!(nonliving.entities[0].hp, hp_before);
    assert_eq!(nonliving.rng.draw_counter, draws_before);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Skipped {
                    reason: AbilityEffectSkipReasonDto::Ineligible,
                    ..
                }]
            )
    )));
}

#[test]
fn poison_branding_is_temporary_affects_melee_and_round_trips() {
    let mut game = Game::new(13);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "test.item.echo-blade".to_owned(),
        kind_id: "demo.item.echo-blade".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let definition = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo living target")
        .clone();
    let target = actor_from_runtime_spawn(
        "test.actor.poison-brand-target",
        &definition.id,
        Position { x: 4, y: 3 },
        definition.max_hp,
        definition.speed,
        100,
        true,
    );
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );

    let resolution = apply_ability_status_effect(
        &mut game.player,
        "demo.ability.death-poison-branding",
        0,
        "rfb.status.poison-branding",
        1,
        2,
        0,
        0,
        AbilityStatusStackingDefinition::Replace,
        None,
        None,
        &BTreeMap::new(),
        &BTreeSet::from([WeaponBrand::Poison]),
        &StatModifiers::default(),
        &EquipmentBonuses::default(),
        &BTreeSet::new(),
        None,
        false,
        100,
        None,
        None,
        &mut game.rng,
    );
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::ApplyStatus {
            ref granted_brands,
            change: AbilityStatusChangeDto::Added,
            ..
        } if granted_brands == &[WeaponBrandDto::Poison]
    ));
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        24
    );
    assert_eq!(
        game.snapshot().player.statuses[0].granted_brands,
        vec![WeaponBrandDto::Poison]
    );

    let restored = Game::from_save(game.to_save()).expect("temporary brand should reload");
    assert_eq!(
        restored.snapshot().player.statuses[0].granted_brands,
        vec![WeaponBrandDto::Poison]
    );
    let restored_profile = restored.player_melee_profile(&restored.player_derived_stats());
    assert_eq!(
        restored.player_melee_damage_multiplier(&restored_profile, &target, &definition),
        24
    );

    let mut legacy_value = serde_json::to_value(game.to_save()).expect("save should serialize");
    legacy_value["player"]["statuses"][0]
        .as_object_mut()
        .expect("status should be an object")
        .remove("grantedBrands");
    let legacy_payload: SavePayloadV1 =
        serde_json::from_value(legacy_value).expect("old status save should deserialize");
    Game::from_save(legacy_payload).expect("old status save should remain loadable");

    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("first brand tick should resolve");
    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("second brand tick should expire");
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );
}

#[test]
fn genocide_erases_without_rewards_or_corpses_and_uniques_resist() {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let unique = artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian");
    unique.glyph = "y".to_owned();
    unique.tags.push("unique".to_owned());
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let mut game =
        Game::from_content_with_build(19, catalog, BUILT_IN_WORLD_ID, "demo.build.scholar")
            .expect("custom scholar build should create");
    clear_monsters(&mut game);
    for (id, kind_id, x) in [
        ("test.actor.normal", "demo.actor.gloom-weaver", 4),
        ("test.actor.unique", "demo.actor.serpent-of-chaos", 5),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        let position = Position { x, y: 3 };
        replace_terrain(&mut game, position, "demo.terrain.floor");
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    let experience_before = game.progress.experience;
    let item_count_before = game.items.len();
    let hp_before = game.player.hp;
    let mut events = Vec::new();
    let mut removed_entities = Vec::new();
    game.resolve_ability_genocide(
        "test.ability.genocide",
        Some(vec![Position { x: 4, y: 3 }]),
        AbilityGenocideScopeDefinition::Glyph,
        1_000,
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut removed_entities,
    );
    let resolution = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::AbilityEffectsResolved { resolution, .. } => resolution.effects.first(),
            _ => None,
        })
        .expect("genocide should emit a resolution");
    let AbilityEffectResolutionDto::Genocide {
        removed_entity_ids,
        resisted_entity_ids,
        fatigue_damage,
        ..
    } = resolution
    else {
        panic!("genocide should emit its dedicated effect resolution");
    };
    assert_eq!(removed_entity_ids, &["test.actor.normal".to_owned()]);
    assert_eq!(resisted_entity_ids, &["test.actor.unique".to_owned()]);
    assert!((2..=8).contains(fatigue_damage));
    assert_eq!(game.player.hp, hp_before - fatigue_damage);
    assert_eq!(game.progress.experience, experience_before);
    assert_eq!(game.items.len(), item_count_before);
    assert_eq!(removed_entities, vec!["test.actor.normal".to_owned()]);
    assert!(
        game.entities
            .iter()
            .all(|actor| actor.id != "test.actor.normal")
    );
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "test.actor.unique")
    );
    assert!(game.items.iter().all(|item| {
        item.kind_id != "demo.item.corpse-remains"
            || !matches!(item.location, ItemLocation::Ground(_))
    }));
}

#[test]
fn ordinary_death_creates_a_corpse_and_animate_dead_consumes_it_persistently() {
    let mut game = Game::new(23);
    clear_monsters(&mut game);
    let definition = game
        .content
        .actor("demo.actor.gloom-weaver")
        .expect("demo corpse source")
        .clone();
    let position = Position { x: 4, y: 3 };
    replace_terrain(&mut game, position, "demo.terrain.floor");
    game.entities.push(actor_from_runtime_spawn(
        "test.actor.corpse-source",
        &definition.id,
        position,
        definition.max_hp,
        definition.speed,
        100,
        true,
    ));
    let trace = ProjectileTrace {
        origin: game.player.position,
        impact: position,
        landing: position,
        traversed: vec![position],
    };
    game.resolve_ability_damage_to_entity(
        0,
        "test.ability.kill",
        DamageType::Physical,
        10_000,
        trace,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("lethal damage should resolve");
    assert!(game.entities.is_empty());
    assert!(game.items.iter().any(|item| {
        item.kind_id == "demo.item.corpse-remains"
            && matches!(item.location, ItemLocation::Ground(found) if found == position)
    }));

    let mut events = Vec::new();
    game.resolve_ability_animate_dead(
        "demo.ability.death-animate-dead",
        "demo.actor.risen-thrall",
        "demo.item.corpse-remains",
        8,
        8,
        &mut events,
        &mut BTreeSet::new(),
    )
    .expect("animate dead should resolve");
    assert!(
        game.items
            .iter()
            .all(|item| item.kind_id != "demo.item.corpse-remains")
    );
    assert_eq!(game.entities.len(), 1);
    assert_eq!(game.entities[0].kind_id, "demo.actor.risen-thrall");
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(game.entities[0].summon.is_none());
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::AnimateDead {
                    consumed_corpse_item_ids,
                    entity_ids,
                    ..
                }] if consumed_corpse_item_ids.len() == 1 && entity_ids.len() == 1
            )
    )));

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("risen thrall should reload");
    assert_eq!(restored.snapshot(), snapshot);
}

#[test]
fn actor_detection_ignores_los_and_orders_entities_stably() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let origin = game.player.position;
    for (id, kind_id, position) in [
        (
            "test.actor.warden",
            "demo.actor.resonant-warden",
            Position {
                x: origin.x + 6,
                y: origin.y,
            },
        ),
        (
            "test.actor.captain",
            "demo.actor.chorus-captain",
            Position {
                x: origin.x + 1,
                y: origin.y + 1,
            },
        ),
        (
            "test.actor.evil",
            "demo.actor.gloom-weaver",
            Position {
                x: origin.x + 7,
                y: origin.y,
            },
        ),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    replace_terrain(
        &mut game,
        Position {
            x: origin.x + 2,
            y: origin.y,
        },
        "demo.terrain.wall",
    );

    let (positions, ids) = game.detect_actor_positions("nonliving", 8);
    assert_eq!(
        ids,
        vec![
            "test.actor.captain".to_owned(),
            "test.actor.warden".to_owned()
        ]
    );
    assert_eq!(
        positions,
        vec![
            Position {
                x: origin.x + 1,
                y: origin.y + 1,
            },
            Position {
                x: origin.x + 6,
                y: origin.y,
            }
        ]
    );
    assert_eq!(
        game.detect_actor_positions("evil", 8).1,
        vec!["test.actor.evil".to_owned()]
    );
    assert!(game.detect_actor_positions("evil", 6).1.is_empty());
    assert!(game.revealed_terrain.is_empty());
}

#[test]
fn sleep_power_resolves_then_skips_energy_and_damage_wakes_the_target() {
    let template = Game::new(0).entities[0].clone();
    let mut saw_added = false;
    let mut saw_resisted = false;
    for seed in 0..256 {
        let mut actor = template.clone();
        actor.statuses.clear();
        let mut rng = RfbRng::seeded(seed);
        let resolution = apply_ability_status_effect(
            &mut actor,
            "test.ability.sleep",
            0,
            STATUS_SLEEP,
            1,
            50,
            0,
            0,
            AbilityStatusStackingDefinition::KeepStrongest,
            None,
            Some(10),
            &BTreeMap::new(),
            &BTreeSet::new(),
            &StatModifiers::default(),
            &EquipmentBonuses::default(),
            &BTreeSet::new(),
            None,
            false,
            100,
            Some(10),
            None,
            &mut rng,
        );
        let AbilityEffectResolutionDto::ApplyStatus {
            power_roll,
            target_roll,
            change,
            ..
        } = resolution
        else {
            panic!("sleep should resolve as a status");
        };
        assert!(power_roll.is_some());
        assert!(target_roll.is_some());
        saw_added |= change == AbilityStatusChangeDto::Added;
        saw_resisted |= change == AbilityStatusChangeDto::Resisted;
        if saw_added && saw_resisted {
            break;
        }
    }
    assert!(saw_added, "a deterministic sleep success seed should exist");
    assert!(
        saw_resisted,
        "a deterministic sleep resistance seed should exist"
    );

    let mut game = Game::new(0);
    let sleeping_actor = game.entities[0].clone();
    clear_monsters(&mut game);
    game.entities.push(sleeping_actor);
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_SLEEP.to_owned(),
        intensity: 1,
        remaining_ticks: 50,
        source_id: Some("test.ability.sleep".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    let position = game.entities[0].position;
    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("sleep should round-trip");
    assert_eq!(restored.snapshot(), snapshot);

    game.entities[0].energy_need = 0;
    let mut events = Vec::new();
    game.process_monster_energy_pulse(&mut events, &mut BTreeSet::new(), &mut Vec::new())
        .expect("sleeping monster energy should resolve");
    assert_eq!(game.entities[0].position, position);
    assert_eq!(game.entities[0].energy_need, 90);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::MonsterSlept { .. }))
    );

    game.entities[0].hp -= 1;
    game.wake_entity_after_damage(0, 1, &mut events);
    assert!(
        game.entities[0]
            .statuses
            .iter()
            .all(|status| status.kind_id != STATUS_SLEEP)
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::EntityAwakened { .. }))
    );
}

#[test]
fn temporary_status_resistances_apply_expire_and_round_trip() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    let granted = BTreeMap::from([
        (
            rfb_content::ActorDamageType::Cold,
            ActorResistanceLevel::Resistant,
        ),
        (
            rfb_content::ActorDamageType::Poison,
            ActorResistanceLevel::Resistant,
        ),
    ]);
    let resolution = apply_ability_status_effect(
        &mut game.player,
        "demo.ability.death-necromantic-resistance",
        0,
        "rfb.status.necromantic-resistance",
        1,
        2,
        0,
        0,
        AbilityStatusStackingDefinition::Replace,
        None,
        None,
        &granted,
        &BTreeSet::new(),
        &StatModifiers::default(),
        &EquipmentBonuses::default(),
        &BTreeSet::new(),
        None,
        false,
        100,
        None,
        None,
        &mut game.rng,
    );
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::ApplyStatus {
            change: AbilityStatusChangeDto::Added,
            ..
        }
    ));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    assert_eq!(
        game.effective_player_resistances()
            .level(DamageType::Poison),
        ResistanceLevel::Resistant
    );

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("temporary resistance should reload");
    assert_eq!(restored.snapshot(), snapshot);

    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("first status tick should resolve");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Resistant
    );
    game.process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("second status tick should expire");
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Cold),
        ResistanceLevel::Normal
    );
}

#[test]
fn control_resists_ineligible_targets_and_turns_pack_leaders_into_allies() {
    let pack_id = "test.pack.control".to_owned();
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for (id, kind_id, position, role) in [
        (
            "test.actor.controlled",
            "demo.actor.resonant-warden",
            Position { x: 8, y: 3 },
            MonsterPackRoleDto::Leader,
        ),
        (
            "test.actor.member",
            "demo.actor.chorus-captain",
            Position { x: 10, y: 4 },
            MonsterPackRoleDto::Member,
        ),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        let mut actor = actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        );
        actor.pack = Some(MonsterPackIdentity {
            id: pack_id.clone(),
            leader_id: "test.actor.controlled".to_owned(),
            role,
            behavior: MonsterPackBehaviorDto::GuardLeader,
        });
        game.entities.push(actor);
    }

    let draws_before = game.rng.draw_counter;
    let ineligible = game.resolve_ability_control(1, 0, "undead", 100);
    assert!(matches!(
        ineligible,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Ineligible,
            roll: None,
            ..
        }
    ));
    assert_eq!(game.rng.draw_counter, draws_before);

    let controlled = game.resolve_ability_control(0, 0, "undead", 100);
    assert!(matches!(
        controlled,
        AbilityEffectResolutionDto::Control {
            outcome: AbilityControlOutcomeDto::Controlled,
            roll: Some(_),
            ..
        }
    ));
    assert_eq!(
        game.entities[0].controller_id.as_deref(),
        Some(game.player.id.as_str())
    );
    assert!(game.entities.iter().all(|entity| entity.pack.is_none()));
    assert_eq!(
        game.snapshot().entities[0].faction,
        EntityFactionDto::Player
    );

    for y in 2..=4 {
        for x in 3..=10 {
            replace_terrain(&mut game, Position { x, y }, "demo.terrain.floor");
        }
    }
    let old_distance = chebyshev_distance(game.entities[0].position, game.player.position);
    game.resolve_monster_action(
        0,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("controlled actor should use player summon AI");
    assert!(chebyshev_distance(game.entities[0].position, game.player.position) < old_distance);

    let snapshot = game.snapshot();
    let restored = Game::from_save(game.to_save()).expect("controller identity should reload");
    assert_eq!(restored.snapshot(), snapshot);

    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian")
        .tags
        .push("undead".to_owned());
    artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == "demo.actor.serpent-of-chaos")
        .expect("demo final guardian")
        .level = 50;
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(artifact));
    let mut resisted_game =
        Game::from_content_with_build(0, catalog, BUILT_IN_WORLD_ID, "demo.build.scholar")
            .expect("custom scholar build should create");
    resisted_game.entities.truncate(1);
    resisted_game.entities[0].kind_id = "demo.actor.serpent-of-chaos".to_owned();
    let resisted = resisted_game.resolve_ability_control(0, 0, "undead", 20);
    assert!(matches!(
        resisted,
        AbilityEffectResolutionDto::Control {
            target_level: 50,
            outcome: AbilityControlOutcomeDto::Resisted,
            roll: Some(_),
            ..
        }
    ));
    assert!(resisted_game.entities[0].controller_id.is_none());
}

#[test]
fn spell_proficiency_uses_rfb_ranks_mana_costs_and_failure_adjustments() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let ability = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("resonant bolt should exist")
        .clone();
    let profile = game
        .casting_profile()
        .expect("scholar should have a casting profile")
        .clone();
    let cases = [
        (0, AbilityProficiencyRankDto::Unskilled, 5, 20),
        (900, AbilityProficiencyRankDto::Beginner, 4, 20),
        (1200, AbilityProficiencyRankDto::Skilled, 4, 20),
        (1400, AbilityProficiencyRankDto::Expert, 3, 19),
        (1600, AbilityProficiencyRankDto::Master, 3, 18),
    ];
    for (proficiency, rank, cost, failure) in cases {
        let progress = game
            .ability_progress
            .get_mut(&ability.id)
            .expect("ability progress should exist");
        progress.proficiency = proficiency;
        let progress = *progress;
        assert_eq!(Game::ability_proficiency_rank(proficiency), rank);
        assert_eq!(
            game.ability_effective_resource_cost(&ability, progress),
            cost
        );
        assert_eq!(game.ability_failure_percent(&profile, &ability), failure);
    }
}

#[test]
fn failed_cast_costs_mana_but_insufficient_mana_does_not_draw_rng() {
    let mut failure =
        Game::new_with_build(2, "demo.build.scholar").expect("scholar build should create");
    let failure_book_item_id = ability_book_item_id(&failure);
    dispatch_next(
        &mut failure,
        GameCommand::StudyAbility {
            book_item_id: failure_book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    let failed_cast = dispatch_next(
        &mut failure,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    let resolution = ability_cast_resolution(&failed_cast);
    assert_eq!(resolution.percentile_roll, 13);
    assert_eq!(resolution.resource_before, 21);
    assert_eq!(resolution.resource_cost, 5);
    assert_eq!(resolution.resource_after, 16);
    assert!(!resolution.succeeded);
    assert_eq!(resolution.proficiency_before, 0);
    assert_eq!(resolution.proficiency_after, 0);
    assert_eq!(resolution.cast_count, 0);
    assert_eq!(resolution.fail_count, 1);
    assert_eq!(
        failure
            .entities
            .iter()
            .find(|entity| entity.id == "demo.monster.ember-mote.1")
            .map(|entity| entity.hp),
        Some(3)
    );

    let mut insufficient =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let insufficient_book_item_id = ability_book_item_id(&insufficient);
    dispatch_next(
        &mut insufficient,
        GameCommand::StudyAbility {
            book_item_id: insufficient_book_item_id,
            ability_id: "demo.ability.resonant-bolt".to_owned(),
        },
    );
    insufficient
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 2;
    let draws = insufficient.rng_draw_counter();
    let rejected = dispatch_next(
        &mut insufficient,
        GameCommand::CastAbility {
            ability_id: "demo.ability.resonant-bolt".to_owned(),
            target: TargetSelection::Entity {
                entity_id: "demo.monster.ember-mote.1".to_owned(),
            },
        },
    );
    assert_eq!(insufficient.rng_draw_counter(), draws);
    assert_eq!(insufficient.resources["demo.resource.mana"].current, 2);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "insufficient-resource")
    }));
    assert!(!rejected.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
        )
    }));
}

#[test]
fn legacy_caster_save_restores_full_resources_without_rng_drift() {
    let canonical =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    let mut legacy = canonical.to_save();
    legacy.player.resources.clear();
    legacy.player.learned_ability_ids.clear();
    legacy.player.ability_progress.clear();
    let draw_counter = legacy.rng.draw_counter;

    let migrated = Game::from_save(legacy).expect("legacy caster save should migrate");
    let snapshot = migrated.snapshot();
    assert_eq!(migrated.rng_draw_counter(), draw_counter);
    assert_eq!(snapshot.player.resources[0].current, 21);
    assert_eq!(snapshot.player.resources[0].maximum, 21);
    assert!(
        snapshot
            .player
            .abilities
            .iter()
            .all(|ability| !ability.learned)
    );
    assert_eq!(migrated.state_hash(), canonical.state_hash());

    let restored = Game::from_save(migrated.to_save())
        .expect("migrated caster state should survive another round trip");
    assert_eq!(restored.state_hash(), migrated.state_hash());
}

#[test]
fn waiting_and_resting_recover_mana_until_the_pool_is_full() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 10;
    let initial_draws = game.rng_draw_counter();

    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.resources["demo.resource.mana"].current, 11);
    assert!(waited.events.iter().any(|event| {
        event.kind == "resource.recovered"
            && matches!(
                event.outcome.as_ref(),
                Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                    if resolution.before == 10
                        && resolution.after == 11
                        && resolution.recovered == 1
            )
    }));

    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    let resolution = rest_resolution(&rested);
    assert_eq!(resolution.completed_turns, 4);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert_eq!(resolution.resource_recoveries.len(), 1);
    assert_eq!(resolution.resource_recoveries[0].before, 11);
    assert_eq!(resolution.resource_recoveries[0].after, 21);
    assert_eq!(game.resources["demo.resource.mana"].current, 21);
    assert_eq!(rested.turn, 5);
    assert_eq!(rested.world_tick, 50);
    assert_eq!(game.rng_draw_counter(), initial_draws);

    let world_tick = game.world_tick;
    let full = dispatch_next(&mut game, GameCommand::Rest { turns: 100 });
    let full_resolution = rest_resolution(&full);
    assert_eq!(full_resolution.completed_turns, 0);
    assert_eq!(
        full_resolution.stop_reason,
        RestStopReasonDto::FullResources
    );
    assert!(full_resolution.resource_recoveries.is_empty());
    assert_eq!(game.world_tick, world_tick);
    assert_eq!(game.rng_draw_counter(), initial_draws);

    let restored = Game::from_save(game.to_save()).expect("recovered mana should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn duelist_initializes_innate_techniques_and_empty_tempo_pool() {
    let game = Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    let baseline =
        Game::new_with_build(0, "demo.build.vanguard").expect("vanguard build should create");
    assert_eq!(game.rng_draw_counter(), baseline.rng_draw_counter());

    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.resources.len(), 1);
    let tempo = &snapshot.player.resources[0];
    assert_eq!(tempo.id, "demo.resource.tempo");
    assert_eq!(tempo.current, 0);
    assert_eq!(tempo.maximum, game.resources["demo.resource.tempo"].maximum);
    assert!(tempo.maximum > 8);
    assert_eq!(tempo.wait_recovery_amount, 0);
    assert_eq!(tempo.rest_recovery_amount, 0);
    assert_eq!(tempo.melee_hit_gain_amount, 2);
    assert_eq!(tempo.melee_kill_gain_amount, 3);
    assert_eq!(tempo.turn_decay_amount, 1);

    assert!(snapshot.player.ability_learning.is_none());
    assert_eq!(snapshot.player.abilities.len(), 2);
    for ability in &snapshot.player.abilities {
        assert!(ability.innate);
        assert!(!ability.learned);
        assert!(!ability.can_study);
        assert!(!ability.can_forget);
        assert!(!ability.can_cast, "tempo starts empty");
        assert_eq!(ability.resource_id, "demo.resource.tempo");
        assert!(ability.book_item_id.is_none());
    }

    let restored = Game::from_save(game.to_save()).expect("duelist save should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn melee_hits_and_kills_feed_tempo_while_idle_turns_decay_it() {
    let mut game =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    clear_monsters(&mut game);
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.tempo-target",
        "demo.actor.echo-hound",
        Position { x: 4, y: 3 },
        8,
        110,
        1_000_000,
        false,
    ));

    let mut hit_events = 0_u32;
    let mut kill_events = 0_u32;
    let mut turns = 0_u32;
    while game
        .entities
        .iter()
        .any(|entity| entity.id == "generated.actor.tempo-target")
    {
        let update = dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        turns += 1;
        for event in &update.events {
            if let Some(GameEventOutcomeDto::ResourceGain { resolution }) = event.outcome.as_ref() {
                assert_eq!(event.kind, "resource.gained");
                assert_eq!(resolution.resource_id, "demo.resource.tempo");
                assert_eq!(resolution.after, resolution.before + resolution.gained);
                match resolution.source {
                    ResourceGainSourceDto::MeleeHit => {
                        assert_eq!(resolution.gained, 2);
                        hit_events += 1;
                    }
                    ResourceGainSourceDto::MeleeKill => {
                        assert_eq!(resolution.gained, 3);
                        kill_events += 1;
                    }
                }
            }
        }
        assert!(turns < 60, "kill should resolve within the turn budget");
    }
    assert!(hit_events >= 2);
    assert_eq!(kill_events, 1);
    let after_kill = game.resources["demo.resource.tempo"].current;
    assert!(after_kill >= 5);

    // An idle wait neither recovers nor feeds tempo, so it decays by one
    // and emits no resource events.
    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.resources["demo.resource.tempo"].current,
        after_kill - 1
    );
    assert!(
        waited
            .events
            .iter()
            .all(|event| { !event.kind.starts_with("resource.") })
    );
}

#[test]
fn technique_casts_consume_tempo_and_reject_shortfalls_without_rng() {
    let mut payload = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = Some(true);
    payload.player.resources[0].current = 10;
    let mut game = Game::from_save(payload).expect("tempo fixture should load");
    let snapshot = game.snapshot();
    let crescent = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.crescent-cut")
        .expect("duelist should expose crescent cut");
    assert!(crescent.innate);
    assert!(crescent.can_cast);
    let expected_cost = crescent.resource_cost;

    let update = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.crescent-cut".to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    let cast = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution.clone()),
            _ => None,
        })
        .expect("crescent cut should resolve a cast");
    assert_eq!(cast.resource_id, "demo.resource.tempo");
    assert_eq!(cast.resource_cost, expected_cost);
    assert_eq!(cast.resource_before, 10);
    assert_eq!(cast.resource_after, 10 - expected_cost);
    assert_eq!(
        game.resources["demo.resource.tempo"].current,
        10 - expected_cost
    );

    game.resources
        .get_mut("demo.resource.tempo")
        .expect("tempo pool should exist")
        .current = 0;
    let draws = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.crescent-cut".to_owned(),
            target: TargetSelection::Direction {
                direction: Direction::East,
            },
        },
    );
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event.args.get("reason").map(String::as_str) == Some("insufficient-resource")
    }));
    assert_eq!(game.rng_draw_counter(), draws);
    assert_eq!(game.resources["demo.resource.tempo"].current, 0);
}

#[test]
fn wait_and_rest_never_refill_tempo_and_rest_stops_immediately() {
    let mut game =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    clear_monsters(&mut game);
    game.resources
        .get_mut("demo.resource.tempo")
        .expect("tempo pool should exist")
        .current = 5;
    let draws = game.rng_draw_counter();

    let waited = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        waited
            .events
            .iter()
            .all(|event| event.kind != "resource.recovered")
    );
    assert_eq!(game.resources["demo.resource.tempo"].current, 4);

    let world_tick = game.world_tick;
    let rested = dispatch_next(&mut game, GameCommand::Rest { turns: 50 });
    let resolution = rest_resolution(&rested);
    assert_eq!(resolution.completed_turns, 0);
    assert_eq!(resolution.stop_reason, RestStopReasonDto::FullResources);
    assert!(resolution.resource_recoveries.is_empty());
    assert_eq!(game.world_tick, world_tick);
    assert_eq!(game.resources["demo.resource.tempo"].current, 4);
    assert_eq!(game.rng_draw_counter(), draws);
}

#[test]
fn saves_without_technique_pools_migrate_to_initial_fill_without_rng() {
    let mut payload = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    payload.player.resources.clear();
    payload.player.ability_progress.clear();
    let migrated = Game::from_save(payload).expect("legacy duelist save should reload");
    assert_eq!(migrated.resources["demo.resource.tempo"].current, 0);
    let baseline =
        Game::new_with_build(0, "demo.build.duelist").expect("duelist build should create");
    assert_eq!(migrated.rng_draw_counter(), baseline.rng_draw_counter());
    assert_eq!(migrated.state_hash(), baseline.state_hash());

    let mut unknown = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    unknown.player.resources[0].id = "demo.resource.missing".to_owned();
    assert!(matches!(
        Game::from_save(unknown),
        Err(CoreError::InvalidSave("player resource ID is invalid"))
    ));

    let mut oversized = Game::new_with_build(0, "demo.build.duelist")
        .expect("duelist build should create")
        .to_save();
    oversized.player.resources[0].maximum += 1;
    assert!(matches!(
        Game::from_save(oversized),
        Err(CoreError::InvalidSave("player resource pool is invalid"))
    ));
}

#[test]
fn rest_interrupts_for_visible_enemies_and_damage_before_recovery() {
    let mut visible =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    visible
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar mana pool should exist")
        .current = 10;
    visible.entities[0].position = Position { x: 4, y: 3 };
    let visible_draws = visible.rng_draw_counter();
    let blocked = dispatch_next(&mut visible, GameCommand::Rest { turns: 10 });
    let blocked_resolution = rest_resolution(&blocked);
    assert_eq!(blocked_resolution.completed_turns, 0);
    assert_eq!(
        blocked_resolution.stop_reason,
        RestStopReasonDto::EnemyVisible
    );
    assert_eq!(visible.world_tick, 0);
    assert_eq!(visible.rng_draw_counter(), visible_draws);
    assert_eq!(visible.resources["demo.resource.mana"].current, 10);

    let mut payload = Game::new_with_build(0, "demo.build.scholar")
        .expect("scholar build should create")
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = Some(true);
    payload.player.resources[0].current = 10;
    payload.player.statuses = vec![StatusSaveDto {
        kind_id: STATUS_BLEEDING.to_owned(),
        intensity: 1,
        remaining_ticks: 1,
        source_id: None,
        granted_resistances: Vec::new(),
        granted_brands: Vec::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: Vec::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut damaged = Game::from_save(payload).expect("bleeding rest fixture should load");
    let interrupted = dispatch_next(&mut damaged, GameCommand::Rest { turns: 10 });
    let interrupted_resolution = rest_resolution(&interrupted);
    assert_eq!(interrupted_resolution.completed_turns, 1);
    assert_eq!(
        interrupted_resolution.stop_reason,
        RestStopReasonDto::Damaged
    );
    assert!(interrupted_resolution.resource_recoveries.is_empty());
    assert_eq!(damaged.resources["demo.resource.mana"].current, 10);
    assert_eq!(damaged.player.hp, 11);
    assert!(
        interrupted
            .events
            .iter()
            .any(|event| event.kind == "rest.interrupted")
    );
}

#[test]
fn scholar_studies_and_casts_a_self_targeted_healing_ability() {
    let mut game =
        Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
    clear_monsters(&mut game);
    game.player.hp = 5;
    let book_item_id = ability_book_item_id_for(&game, "demo.item.stillwater-notes");

    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id,
            ability_id: "demo.ability.mending-echo".to_owned(),
        },
    );
    let cast = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.mending-echo".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    let cast_resolution = ability_cast_resolution(&cast);
    assert_eq!(cast_resolution.failure_percent, 15);
    assert_eq!(cast_resolution.percentile_roll, 32);
    assert!(cast_resolution.succeeded);
    assert_eq!(cast_resolution.resource_before, 21);
    assert_eq!(cast_resolution.base_resource_cost, 4);
    assert_eq!(cast_resolution.resource_cost, 7);
    assert_eq!(cast_resolution.resource_after, 14);
    assert_eq!(cast_resolution.proficiency_before, 0);
    assert_eq!(cast_resolution.proficiency_after, 128);
    assert_eq!(cast_resolution.cooldown_after, 2);
    assert_eq!(game.player.hp, 11);
    assert!(cast.events.iter().any(|event| {
        event.kind == "ability.healed"
            && matches!(
                event.outcome.as_ref(),
                Some(GameEventOutcomeDto::Heal { resolution })
                    if resolution.requested == 6 && resolution.applied == 6
            )
    }));

    let mana_before_rejection = game.resources["demo.resource.mana"].current;
    let draws_before_rejection = game.rng_draw_counter();
    let rejected = dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: "demo.ability.mending-echo".to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert_eq!(
        game.resources["demo.resource.mana"].current,
        mana_before_rejection
    );
    assert_eq!(game.rng_draw_counter(), draws_before_rejection);
    assert!(rejected.events.iter().any(|event| {
        event.kind == "ability.cast-unavailable"
            && event
                .args
                .get("reason")
                .is_some_and(|reason| reason == "cooldown")
    }));
    assert_eq!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .map(|ability| ability.cooldown_remaining),
        Some(1)
    );

    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.mending-echo")
            .map(|ability| ability.cooldown_remaining),
        Some(0)
    );

    let restored = Game::from_save(game.to_save()).expect("healing ability state should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn device_skill_check_distinguishes_builds_without_consuming_on_failure() {
    const ITEM_ID: &str = "test.item.resonance-stabilizer.1";
    let mut tinkerer = skill_check_game(0, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(0, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        game.player.hp = 5;
        give_inventory_item(game, ITEM_ID, "demo.item.resonance-stabilizer");
    }
    assert_eq!(tinkerer.rng_draw_counter(), vanguard.rng_draw_counter());

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.device-success");
    let failure = check_resolution(&vanguard_update, "skill.device-failure");

    assert_check(success, "demo.skill.device", 69, 60, 32, Some(54), 45);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.device", 16, 60, 32, Some(9), 45);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert_eq!(tinkerer.player.hp, 11);
    assert_eq!(vanguard.player.hp, 5);
    assert!(!tinkerer.items.iter().any(|item| item.id == ITEM_ID));
    assert!(vanguard.items.iter().any(|item| item.id == ITEM_ID));
    assert!(
        tinkerer
            .item_knowledge
            .get("demo.item.resonance-stabilizer")
            .is_some_and(|knowledge| knowledge.tried && knowledge.aware)
    );
    assert!(
        vanguard
            .item_knowledge
            .get("demo.item.resonance-stabilizer")
            .is_some_and(|knowledge| knowledge.tried && !knowledge.aware)
    );
}

#[test]
fn charged_device_spends_instance_charges_only_after_a_successful_check_and_round_trips() {
    const ITEM_ID: &str = "test.item.resonance-mender.1";
    let mut tinkerer = skill_check_game(0, "demo.build.tinkerer");
    tinkerer.player.hp = 1;
    give_inventory_item(&mut tinkerer, ITEM_ID, "demo.item.resonance-mender");

    let before = tinkerer.snapshot();
    let mender = before
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged device should be carried");
    assert_eq!(mender.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(mender.charges, None);
    assert!(mender.usable);

    let update = dispatch_next(
        &mut tinkerer,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "skill.device-success")
    );
    let used = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-heal")
        .expect("successful device should apply its healing dice");
    assert!(matches!(
        used.outcome,
        Some(GameEventOutcomeDto::Heal { resolution })
            if (2..=8).contains(&resolution.requested)
                && resolution.applied == resolution.requested
    ));
    let mender = update
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged device should not be consumed");
    assert_eq!(mender.quantity, 1);
    assert_eq!(
        mender.charges,
        Some(ItemChargesDto {
            current: 2,
            maximum: 3,
        })
    );
    let restored =
        Game::from_save(tinkerer.to_save()).expect("charged item state should survive reload");
    assert_eq!(restored.snapshot(), tinkerer.snapshot());

    let mut invalid = tinkerer.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("charged item should be saved")
        .charges = Some(ItemChargesDto {
        current: 4,
        maximum: 3,
    });
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));

    let mut missing = tinkerer.to_save();
    missing
        .inventory
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("charged item should be saved")
        .charges = None;
    assert!(matches!(
        Game::from_save(missing),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));
}

#[test]
fn failed_and_depleted_device_attempts_preserve_charges() {
    const ITEM_ID: &str = "test.item.resonance-mender.1";
    let mut vanguard = skill_check_game(0, "demo.build.vanguard");
    vanguard.player.hp = 1;
    give_inventory_item(&mut vanguard, ITEM_ID, "demo.item.resonance-mender");

    let failed = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert!(
        failed
            .events
            .iter()
            .any(|event| event.kind == "skill.device-failure")
    );
    assert_eq!(
        failed
            .inventory
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("failed device should remain carried")
            .charges,
        None
    );
    assert_eq!(
        vanguard
            .items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("failed device should retain its instance state")
            .charges,
        Some(ItemChargesDto {
            current: 3,
            maximum: 3,
        })
    );

    vanguard
        .items
        .iter_mut()
        .find(|item| item.id == ITEM_ID)
        .expect("device should remain after failure")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 3,
    });
    let draws = vanguard.rng_draw_counter();
    let world_tick = vanguard.world_tick;
    let depleted = dispatch_next(
        &mut vanguard,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );
    assert_eq!(depleted.events[0].kind, "item.use-unavailable");
    assert_eq!(vanguard.rng_draw_counter(), draws);
    assert_eq!(vanguard.world_tick, world_tick);
    let depleted_device = depleted
        .inventory
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("depleted device should remain carried");
    assert!(!depleted_device.usable);
    assert_eq!(depleted_device.charges, None);
    assert_eq!(
        vanguard
            .items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .expect("depleted device should retain its instance state")
            .charges,
        Some(ItemChargesDto {
            current: 0,
            maximum: 3,
        })
    );
}

#[test]
fn restorative_item_sequence_recovers_resource_then_removes_status() {
    const ITEM_ID: &str = "test.item.clarity-draught.1";
    let mut game = skill_check_game(19, "demo.build.scholar");
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana")
        .current = 0;
    game.player.statuses.push(StatusInstance {
        kind_id: STATUS_CONFUSION.to_owned(),
        remaining_ticks: 20,
        intensity: 1,
        source_id: Some("test".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, "demo.item.clarity-draught");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert!(game.resources["demo.resource.mana"].current > 0);
    assert!(!game.player_has_status_kind(STATUS_CONFUSION));
    let effect_events = update
        .events
        .iter()
        .filter(|event| event.kind.starts_with("item.use-"))
        .map(|event| event.kind.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        effect_events,
        vec!["item.use-resource-restored", "item.use-status-removed"]
    );
}

#[test]
fn full_resource_restoration_is_deterministic_and_round_trips() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(23, "demo.build.scholar");
    let mana = game
        .resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana");
    mana.current = 1;
    let maximum = mana.maximum;
    game.player.statuses.push(StatusInstance {
        kind_id: "rfb.status.berserk".to_owned(),
        remaining_ticks: 20,
        intensity: 1,
        source_id: Some("test".to_owned()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert_eq!(game.resources["demo.resource.mana"].current, maximum);
    assert!(!game.player_has_status_kind("rfb.status.berserk"));
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(update.events.iter().any(|event| {
        matches!(
            &event.outcome,
            Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                if resolution.before == 1
                    && resolution.after == maximum
                    && resolution.recovered == maximum - 1
        )
    }));
    let restored = Game::from_save(game.to_save()).expect("restored resource state should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn successful_restoration_reveals_later_no_effect_events() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(27, "demo.build.scholar");
    game.resources
        .get_mut("demo.resource.mana")
        .expect("scholar should have mana")
        .current = 0;
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    let status_event = update
        .events
        .iter()
        .find(|event| event.kind == "item.use-status-no-effect")
        .expect("the absent berserk status should produce a no-effect event");
    assert_eq!(
        status_event.args["nameKey"],
        "item-demo-perfect-focus-elixir-name"
    );
}

#[test]
fn missing_player_resource_consumes_restorative_without_claiming_awareness() {
    const ITEM_ID: &str = "test.item.perfect-focus-elixir.1";
    let mut game = skill_check_game(29, "demo.build.vanguard");
    assert!(!game.resources.contains_key("demo.resource.mana"));
    give_inventory_item(&mut game, ITEM_ID, "demo.item.perfect-focus-elixir");

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: ITEM_ID.to_owned(),
            target: None,
        },
    );

    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-resource-no-effect"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ResourceRecovery { resolution })
                    if resolution.before == 0
                        && resolution.after == 0
                        && resolution.recovered == 0
            )
    }));
    assert!(
        game.item_knowledge
            .get("demo.item.perfect-focus-elixir")
            .is_some_and(|knowledge| knowledge.tried && !knowledge.aware)
    );
}

#[test]
fn appraisal_scroll_targets_an_item_without_drawing_rng() {
    const SCROLL_ID: &str = "test.item.appraisal-scroll.1";
    const TARGET_ID: &str = "test.item.appraisal-target.1";
    let mut game = skill_check_game(31, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");
    give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
    let before = game.snapshot();
    let scroll = before
        .inventory
        .iter()
        .find(|item| item.id == SCROLL_ID)
        .expect("appraisal scroll should be carried");
    assert_eq!(scroll.knowledge, ItemKnowledgeDto::Unknown);
    assert_eq!(scroll.use_target_spec, Some(item_target_spec()));
    let draws_before = game.rng_draw_counter();

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(
        game.item_knowledge_dto("demo.item.appraisal-scroll"),
        ItemKnowledgeDto::Aware
    );
    let target = update
        .inventory
        .iter()
        .find(|item| item.id == TARGET_ID)
        .expect("identified target should remain carried");
    assert_eq!(target.knowledge, ItemKnowledgeDto::Aware);
    assert_eq!(target.identification, ItemIdentificationDto::Appraised);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-identified"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ItemIdentify { resolution })
                    if resolution.item_id == TARGET_ID
                        && !resolution.full
                        && resolution.changed
            )
    }));
}

#[test]
fn revelation_scroll_fully_identifies_affixes_and_round_trips() {
    const SCROLL_ID: &str = "test.item.revelation-scroll.1";
    const TARGET_ID: &str = "test.item.revelation-target.1";
    let mut game = skill_check_game(37, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.revelation-scroll");
    game.items
        .iter_mut()
        .find(|item| item.id == SCROLL_ID)
        .expect("revelation scroll should be carried")
        .quantity = 2;
    game.items.push(ItemInstance {
        id: TARGET_ID.to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Exceptional,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == SCROLL_ID)
            .expect("one scroll should remain")
            .quantity,
        1
    );
    let target = update
        .inventory
        .iter()
        .find(|item| item.id == TARGET_ID)
        .expect("fully identified target should remain carried");
    assert_eq!(target.identification, ItemIdentificationDto::Identified);
    assert_eq!(target.quality, Some(ItemQualityDto::Exceptional));
    assert_eq!(target.known_properties.len(), 1);
    assert_eq!(
        target.known_properties[0].affix_id,
        "demo.affix.adaptive-echo"
    );
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-fully-identified"
            && matches!(
                &event.outcome,
                Some(GameEventOutcomeDto::ItemIdentify { resolution })
                    if resolution.item_id == TARGET_ID && resolution.full
            )
    }));
    let restored = Game::from_save(game.to_save()).expect("item knowledge should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn identify_scroll_rejects_missing_and_self_targets_before_consumption() {
    const SCROLL_ID: &str = "test.item.invalid-identify-scroll.1";
    let mut game = skill_check_game(41, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.appraisal-scroll");

    for target_item_id in ["missing.item", SCROLL_ID] {
        let draws_before = game.rng_draw_counter();
        let tick_before = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: target_item_id.to_owned(),
                }),
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert_eq!(game.world_tick, tick_before);
        assert!(game.items.iter().any(|item| item.id == SCROLL_ID));
        assert_eq!(
            game.item_knowledge_dto("demo.item.appraisal-scroll"),
            ItemKnowledgeDto::Unknown
        );
    }
}

#[test]
fn enchantment_scroll_succeeds_consumes_on_failure_and_round_trips() {
    const SCROLL_ID: &str = "test.item.accuracy-scroll.1";
    const TARGET_ID: &str = "test.item.enchantment-target.1";
    let mut game = skill_check_game(0, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
    game.rng = RfbRng::seeded(0);

    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );

    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    let resolution = update
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::ItemEnchantment { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("enchantment should emit a structured resolution");
    assert_eq!(update.events[0].kind, "item.use-enchanted");
    assert_eq!(resolution.to_hit.attempts, 1);
    assert_eq!(resolution.to_hit.successes, 1);
    assert_eq!(resolution.to_hit.before, 0);
    assert_eq!(resolution.to_hit.after, 1);
    assert_eq!(resolution.to_damage.attempts, 0);
    assert_eq!(resolution.to_armor.attempts, 0);

    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    game.items
        .iter_mut()
        .find(|item| item.id == TARGET_ID)
        .expect("target should remain carried")
        .enchantments
        .to_hit = 15;
    game.rng = RfbRng::seeded(0);
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: TARGET_ID.to_owned(),
            }),
        },
    );
    assert_eq!(update.events[0].kind, "item.use-enchantment-failed");
    assert_eq!(game.rng_draw_counter() - draws_before, 2);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == TARGET_ID)
            .expect("target should remain carried")
            .enchantments
            .to_hit,
        15
    );

    let restored = Game::from_save(game.to_save()).expect("enchantments should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());
    let mut invalid = game.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == TARGET_ID)
        .expect("target should be saved")
        .enchantments
        .to_hit = 16;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item enchantment state is invalid"))
    ));
}

#[test]
fn masterwork_weapon_scroll_rolls_both_components_deterministically() {
    fn run(seed: u64) -> (ItemEnchantmentResolutionDto, u64) {
        const SCROLL_ID: &str = "test.item.masterwork-weapon-scroll.1";
        const TARGET_ID: &str = "test.item.masterwork-target.1";
        let mut game = skill_check_game(seed, "demo.build.scholar");
        give_inventory_item(&mut game, SCROLL_ID, "demo.item.masterwork-weapon-scroll");
        give_inventory_item(&mut game, TARGET_ID, "demo.item.adaptive-glaive");
        game.rng = RfbRng::seeded(seed);
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::Item {
                    item_id: TARGET_ID.to_owned(),
                }),
            },
        );
        let resolution = update
            .events
            .iter()
            .find_map(|event| match &event.outcome {
                Some(GameEventOutcomeDto::ItemEnchantment { resolution }) => {
                    Some(resolution.clone())
                }
                _ => None,
            })
            .expect("masterwork enchantment should emit a resolution");
        (resolution, game.rng_draw_counter())
    }

    let left = run(37);
    let right = run(37);
    assert_eq!(left, right);
    assert!((4..=6).contains(&left.0.to_hit.attempts));
    assert!((4..=6).contains(&left.0.to_damage.attempts));
    assert_eq!(left.0.to_armor.attempts, 0);
    assert_eq!(left.0.to_hit.successes, left.0.to_hit.after);
    assert_eq!(left.0.to_damage.successes, left.0.to_damage.after);
}

#[test]
fn enchantment_artifact_and_ammunition_pile_gates_follow_original_order() {
    let artifact_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.scholar");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 1, false, false);
        let mut artifact = skill_check_game(*seed, "demo.build.scholar");
        artifact.rng = RfbRng::seeded(*seed);
        let artifact = artifact.resolve_item_enchantment_component(0, 1, 1, false, true);
        ordinary.successes == 1 && artifact.successes == 0
    });
    assert_eq!(artifact_seed, Some(0));

    let ammunition_seed = (0..1_000).find(|seed| {
        let mut ordinary = skill_check_game(*seed, "demo.build.scholar");
        ordinary.rng = RfbRng::seeded(*seed);
        let ordinary = ordinary.resolve_item_enchantment_component(0, 1, 20, false, false);
        let mut ammunition = skill_check_game(*seed, "demo.build.scholar");
        ammunition.rng = RfbRng::seeded(*seed);
        let ammunition = ammunition.resolve_item_enchantment_component(0, 1, 20, true, false);
        ordinary.successes == 0 && ammunition.successes == 1
    });
    assert_eq!(ammunition_seed, Some(0));
}

#[test]
fn enchantment_scroll_rejects_invalid_targets_atomically() {
    const SCROLL_ID: &str = "test.item.invalid-enchantment-scroll.1";
    const ARMOR_ID: &str = "test.item.invalid-enchantment-armor.1";
    let mut game = skill_check_game(41, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    give_inventory_item(&mut game, ARMOR_ID, "demo.item.resonance-mail");

    for target in [
        None,
        Some(TargetSelection::Item {
            item_id: "missing.item".to_owned(),
        }),
        Some(TargetSelection::Item {
            item_id: SCROLL_ID.to_owned(),
        }),
        Some(TargetSelection::Item {
            item_id: ARMOR_ID.to_owned(),
        }),
        Some(TargetSelection::SelfTarget),
    ] {
        let draws_before = game.rng_draw_counter();
        let tick_before = game.world_tick;
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target,
            },
        );
        assert_eq!(update.events[0].kind, "item.use-unavailable");
        assert_eq!(game.rng_draw_counter(), draws_before);
        assert_eq!(game.world_tick, tick_before);
        assert!(game.items.iter().any(|item| item.id == SCROLL_ID));
        assert_eq!(
            game.item_knowledge_dto("demo.item.accuracy-scroll"),
            ItemKnowledgeDto::Unknown
        );
    }
}

#[test]
fn enchantments_feed_combat_armor_and_legacy_save_projection() {
    let mut game = skill_check_game(53, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    for (id, kind_id, location, enchantments) in [
        (
            "test.item.enchanted-weapon",
            "demo.item.adaptive-glaive",
            ItemLocation::Equipped {
                slot_id: "weapon".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 3,
                to_damage: 4,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-launcher",
            "demo.item.resonance-sling",
            ItemLocation::Equipped {
                slot_id: "launcher".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 2,
                to_damage: 3,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-ammunition",
            "demo.item.resonance-pellet",
            ItemLocation::Inventory,
            ItemEnchantmentsDto {
                to_hit: 5,
                to_damage: 6,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-throwable",
            "demo.item.luminous-shard",
            ItemLocation::Inventory,
            ItemEnchantmentsDto {
                to_hit: 7,
                to_damage: 8,
                to_armor: 0,
            },
        ),
        (
            "test.item.enchanted-armor",
            "demo.item.resonance-mail",
            ItemLocation::Equipped {
                slot_id: "body".to_owned(),
            },
            ItemEnchantmentsDto {
                to_hit: 0,
                to_damage: 0,
                to_armor: 5,
            },
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("test item should be carried");
        item.location = location;
        item.enchantments = enchantments;
    }
    game.items
        .iter_mut()
        .find(|item| item.id == "test.item.enchanted-ammunition")
        .expect("ammunition should exist")
        .quantity = 20;
    for item in game.items.iter_mut().filter(|item| {
        item.kind_id == "demo.item.resonance-pellet"
            && item.location == ItemLocation::Inventory
            && item.quantity > 0
    }) {
        item.enchantments = ItemEnchantmentsDto {
            to_hit: 5,
            to_damage: 6,
            to_armor: 0,
        };
    }
    for id in [
        "test.item.enchanted-weapon",
        "test.item.enchanted-launcher",
        "test.item.enchanted-armor",
    ] {
        game.item_property_knowledge.insert(
            id.to_owned(),
            ItemPropertyKnowledgeState {
                appraised: true,
                identified: true,
                known_affix_ids: BTreeSet::new(),
            },
        );
    }
    game.mark_item_aware("demo.item.luminous-shard");

    let snapshot = game.snapshot();
    let weapon = snapshot
        .equipment
        .iter()
        .find(|item| item.id == "test.item.enchanted-weapon")
        .expect("weapon should be equipped");
    assert_eq!(weapon.enchantments.to_hit, 3);
    assert_eq!(
        weapon.melee_profile.as_ref().expect("melee profile").to_hit,
        5
    );
    assert_eq!(
        weapon
            .melee_profile
            .as_ref()
            .expect("melee profile")
            .to_damage,
        6
    );
    assert_eq!(snapshot.player.melee_profile.to_damage, 6);
    let projectile = snapshot
        .player
        .projectile_profile
        .as_ref()
        .expect("launcher should expose a projectile profile");
    assert_eq!(projectile.to_hit, 37);
    assert_eq!(projectile.to_damage, 10);
    let throwable = snapshot
        .inventory
        .iter()
        .find(|item| item.id == "test.item.enchanted-throwable")
        .and_then(|item| item.throw_profile.as_ref())
        .expect("throwable should expose a throw profile");
    assert_eq!(throwable.to_hit, 37);
    assert_eq!(throwable.to_damage, 8);
    let stats = game.player_derived_stats();
    assert!(stats.armor_class.contributions.iter().any(|contribution| {
        contribution.source_id == "test.item.enchanted-armor" && contribution.amount == 90
    }));

    let restored = Game::from_save(game.to_save()).expect("all item locations should round-trip");
    assert_eq!(restored.snapshot(), game.snapshot());

    let mut legacy_json = serde_json::to_value(game.to_save()).expect("save should serialize");
    for field in ["items", "inventory", "equipment", "carriedItems"] {
        if let Some(items) = legacy_json
            .get_mut(field)
            .and_then(serde_json::Value::as_array_mut)
        {
            for item in items {
                item.as_object_mut()
                    .expect("saved item should be an object")
                    .remove("enchantments");
            }
        }
    }
    let legacy: SavePayloadV1 =
        serde_json::from_value(legacy_json).expect("missing enchantments should default");
    let migrated = Game::from_save(legacy).expect("legacy save should load");
    assert!(
        migrated
            .items
            .iter()
            .all(|item| item.enchantments.is_empty())
    );
}

#[test]
fn curse_scroll_lands_on_equipped_weapon_and_artifact_can_resist() {
    fn run(resisted: bool) -> (Game, GameUpdate, u64) {
        const SCROLL_ID: &str = "test.item.weapon-blight-scroll.1";
        const WEAPON_ID: &str = "test.item.relic-blade.1";
        let mut game = skill_check_game(61, "demo.build.scholar");
        for item in game
            .items
            .iter_mut()
            .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
        {
            item.location = ItemLocation::Inventory;
        }
        give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
        give_inventory_item(&mut game, WEAPON_ID, "demo.item.relic-blade");
        game.items
            .iter_mut()
            .find(|item| item.id == WEAPON_ID)
            .expect("relic blade should exist")
            .location = ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        };
        game.debug_set_item_curses_land(!resisted);
        game.debug_set_item_curses_resisted(resisted);
        let draws_before = game.rng_draw_counter();
        let update = dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: SCROLL_ID.to_owned(),
                target: Some(TargetSelection::SelfTarget),
            },
        );
        (game, update, draws_before)
    }

    let (landed, update, draws_before) = run(false);
    assert_eq!(landed.rng_draw_counter(), draws_before);
    assert_eq!(update.events[0].kind, "item.use-cursed");
    assert_eq!(
        landed
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    assert_eq!(
        landed.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(update.events.iter().any(|event| {
        matches!(
            &event.outcome,
            Some(GameEventOutcomeDto::ItemCurse { resolution })
                if resolution.item_id.as_deref() == Some("test.item.relic-blade.1")
                    && resolution.before.is_none()
                    && resolution.after == Some(ItemCurseSeverityDto::Normal)
                    && !resolution.resisted
        )
    }));

    let (resisted, update, draws_before) = run(true);
    assert_eq!(resisted.rng_draw_counter(), draws_before);
    assert_eq!(update.events[0].kind, "item.use-curse-resisted");
    assert_eq!(
        resisted
            .items
            .iter()
            .find(|item| item.id == "test.item.relic-blade.1")
            .expect("relic blade should remain equipped")
            .curse,
        None
    );
    assert_eq!(
        resisted.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn curse_scroll_without_a_matching_equipped_item_consumes_without_rng_or_awareness() {
    const SCROLL_ID: &str = "test.item.weapon-blight-scroll.no-target";
    let mut game = skill_check_game(67, "demo.build.scholar");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.weapon-blight-scroll");
    let draws_before = game.rng_draw_counter();
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(!game.items.iter().any(|item| item.id == SCROLL_ID));
    assert_eq!(update.events[0].kind, "item.use-curse-no-target");
    assert_eq!(
        game.item_knowledge_dto("demo.item.weapon-blight-scroll"),
        ItemKnowledgeDto::Tried
    );
}

#[test]
fn cleansing_scrolls_respect_heavy_and_permanent_curse_boundaries() {
    const NORMAL_ID: &str = "test.item.normal-curse";
    const HEAVY_ID: &str = "test.item.heavy-curse";
    const PERMANENT_ID: &str = "test.item.permanent-curse";
    let mut game = skill_check_game(71, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    for (id, kind_id, slot_id, curse) in [
        (
            NORMAL_ID,
            "demo.item.relic-blade",
            "weapon",
            ItemCurseSeverityDto::Normal,
        ),
        (
            HEAVY_ID,
            "demo.item.burdened-mail",
            "body",
            ItemCurseSeverityDto::Heavy,
        ),
        (
            PERMANENT_ID,
            "demo.item.sealed-amulet",
            "amulet",
            ItemCurseSeverityDto::Permanent,
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == id)
            .expect("curse test item should exist");
        item.location = ItemLocation::Equipped {
            slot_id: slot_id.to_owned(),
        };
        item.curse = Some(curse);
    }
    give_inventory_item(
        &mut game,
        "test.item.cleansing-scroll.1",
        "demo.item.cleansing-scroll",
    );
    let ordinary = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(ordinary.events[0].kind, "item.use-curses-removed");
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == NORMAL_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );

    give_inventory_item(
        &mut game,
        "test.item.greater-cleansing-scroll.1",
        "demo.item.greater-cleansing-scroll",
    );
    let greater = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.greater-cleansing-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert_eq!(greater.events[0].kind, "item.use-curses-removed");
    let resolution = greater
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::ItemCurseRemoval { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("greater cleansing should emit a structured resolution");
    assert_eq!(resolution.removed_item_ids, [HEAVY_ID]);
    assert_eq!(resolution.retained_permanent_item_ids, [PERMANENT_ID]);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == HEAVY_ID)
            .unwrap()
            .curse,
        None
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == PERMANENT_ID)
            .unwrap()
            .curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    let saved = game.to_save();
    let restored = Game::from_save(saved.clone()).expect("curse severities should round-trip");
    for (item_id, expected) in [
        (HEAVY_ID, None),
        (PERMANENT_ID, Some(ItemCurseSeverityDto::Permanent)),
    ] {
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .curse,
            expected
        );
    }
}

#[test]
fn cursed_equipment_cannot_be_unequipped_or_replaced_and_rejection_is_zero_time() {
    const CURSED_ID: &str = "test.item.cursed-mail";
    const REPLACEMENT_ID: &str = "test.item.replacement-mail";
    let mut game = skill_check_game(73, "demo.build.vanguard");
    for item in game
        .items
        .iter_mut()
        .filter(|item| matches!(item.location, ItemLocation::Equipped { .. }))
    {
        item.location = ItemLocation::Inventory;
    }
    give_inventory_item(&mut game, CURSED_ID, "demo.item.burdened-mail");
    let cursed = game
        .items
        .iter_mut()
        .find(|item| item.id == CURSED_ID)
        .unwrap();
    cursed.location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    cursed.curse = Some(ItemCurseSeverityDto::Heavy);
    give_inventory_item(&mut game, REPLACEMENT_ID, "demo.item.resonance-mail");

    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let unequip = dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "body".to_owned(),
        },
    );
    assert_eq!(unequip.events[0].kind, "item.unequip.cursed");
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);

    let replace = dispatch_next(
        &mut game,
        GameCommand::Equip {
            item_id: REPLACEMENT_ID.to_owned(),
        },
    );
    assert_eq!(replace.events[0].kind, "item.unequip.cursed");
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(matches!(
        game.items
            .iter()
            .find(|item| item.id == CURSED_ID)
            .unwrap()
            .location,
        ItemLocation::Equipped { .. }
    ));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == REPLACEMENT_ID)
            .unwrap()
            .location,
        ItemLocation::Inventory
    );
}

#[test]
fn curse_state_round_trips_all_item_locations_migrates_and_prevents_stacking() {
    let mut game = Game::new(79);
    let carrier_id = game.entities[0].id.clone();
    for (id, kind_id, location, curse) in [
        (
            "test.item.curse-ground",
            "demo.item.resonance-mail",
            ItemLocation::Ground(game.player.position),
            ItemCurseSeverityDto::Normal,
        ),
        (
            "test.item.curse-inventory",
            "demo.item.burdened-mail",
            ItemLocation::Inventory,
            ItemCurseSeverityDto::Heavy,
        ),
        (
            "test.item.curse-equipment",
            "demo.item.sealed-amulet",
            ItemLocation::Equipped {
                slot_id: "amulet".to_owned(),
            },
            ItemCurseSeverityDto::Permanent,
        ),
        (
            "test.item.curse-carried",
            "demo.item.relic-blade",
            ItemLocation::CarriedBy {
                actor_id: carrier_id.clone(),
            },
            ItemCurseSeverityDto::Normal,
        ),
    ] {
        give_inventory_item(&mut game, id, kind_id);
        let item = game.items.iter_mut().find(|item| item.id == id).unwrap();
        item.location = location;
        item.curse = Some(curse);
    }
    let saved = game.to_save();
    assert_eq!(
        saved.items.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    assert_eq!(
        saved.inventory.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(
        saved.equipment.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    assert_eq!(
        saved.carried_items.last().unwrap().curse,
        Some(ItemCurseSeverityDto::Normal)
    );
    let restored = Game::from_save(saved.clone()).expect("all curse locations should reload");
    for (item_id, expected) in [
        ("test.item.curse-ground", ItemCurseSeverityDto::Normal),
        ("test.item.curse-inventory", ItemCurseSeverityDto::Heavy),
        ("test.item.curse-equipment", ItemCurseSeverityDto::Permanent),
        ("test.item.curse-carried", ItemCurseSeverityDto::Normal),
    ] {
        assert_eq!(
            restored
                .items
                .iter()
                .find(|item| item.id == item_id)
                .unwrap()
                .curse,
            Some(expected)
        );
    }

    let mut legacy_json = serde_json::to_value(game.to_save()).expect("save should serialize");
    for field in ["items", "inventory", "equipment", "carriedItems"] {
        for item in legacy_json[field]
            .as_array_mut()
            .expect("item save field should be an array")
        {
            item.as_object_mut()
                .expect("saved item should be an object")
                .remove("curse");
        }
    }
    let legacy: SavePayloadV1 =
        serde_json::from_value(legacy_json).expect("missing curse should default");
    let migrated = Game::from_save(legacy).expect("legacy curse state should load");
    assert!(migrated.items.iter().all(|item| item.curse.is_none()));

    let mut stack_game = skill_check_game(83, "demo.build.vanguard");
    give_inventory_item(
        &mut stack_game,
        "test.item.stack-clean",
        "demo.item.resonance-mail",
    );
    give_inventory_item(
        &mut stack_game,
        "test.item.stack-cursed",
        "demo.item.resonance-mail",
    );
    let cursed = stack_game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.stack-cursed")
        .unwrap();
    cursed.curse = Some(ItemCurseSeverityDto::Normal);
    cursed.location = ItemLocation::Ground(stack_game.player.position);
    dispatch_next(&mut stack_game, GameCommand::PickUp);
    assert_eq!(
        stack_game
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.resonance-mail")
            .count(),
        2
    );
}

#[test]
fn dynamic_device_generation_filters_by_depth_is_weighted_and_round_trips() {
    const WAND_ID: &str = "demo.item.resonance-wand";
    let content = load_built_in_content().expect("built-in content should load");
    let mut shallow_rng = RfbRng::seeded(11);
    let (shallow_activation, shallow_charges) =
        initial_item_runtime_state(&content, &mut shallow_rng, WAND_ID, 1);
    let shallow_activation = shallow_activation.expect("wand should materialize an activation");
    assert_eq!(
        shallow_activation.profile_id,
        "demo.device-activation.spark-bolt"
    );
    let shallow_charges = shallow_charges.expect("wand should materialize charges");
    assert!((12..=24).contains(&shallow_charges.maximum));
    assert!((shallow_activation.cost..=shallow_charges.maximum).contains(&shallow_charges.current));

    let mut selected = BTreeSet::new();
    for seed in 0..64 {
        let mut left = RfbRng::seeded(seed);
        let mut right = RfbRng::seeded(seed);
        let left_state = initial_item_runtime_state(&content, &mut left, WAND_ID, 20);
        let right_state = initial_item_runtime_state(&content, &mut right, WAND_ID, 20);
        assert_eq!(left_state, right_state);
        selected.insert(
            left_state
                .0
                .expect("deep wand should materialize an activation")
                .profile_id,
        );
    }
    assert_eq!(
        selected,
        BTreeSet::from([
            "demo.device-activation.frost-bolt".to_owned(),
            "demo.device-activation.spark-bolt".to_owned(),
        ])
    );

    let mut game = skill_check_game(11, "demo.build.tinkerer");
    give_inventory_item(&mut game, "test.item.dynamic-wand", WAND_ID);
    let restored = Game::from_save(game.to_save()).expect("dynamic device should round-trip");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == "test.item.dynamic-wand")
        .expect("dynamic device should remain in inventory");
    assert_eq!(
        restored_item
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.spark-bolt")
    );
}

#[test]
fn dynamic_wand_validates_target_before_check_and_spends_only_on_success() {
    const ITEM_ID: &str = "test.item.dynamic-wand";
    let mut game = Game::new_with_build(0, "demo.build.tinkerer")
        .expect("device specialist build should create");
    game.player.position = Position { x: 7, y: 5 };
    give_inventory_item(&mut game, ITEM_ID, "demo.item.resonance-wand");
    let charges_before = game
        .items
        .iter()
        .find(|item| item.id == ITEM_ID)
        .and_then(|item| item.charges)
        .expect("dynamic wand should carry charges");
    let draws_before = game.rng.draw_counter;
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::SelfTarget),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("invalid target should be handled");
    assert_eq!(events, vec![DomainEvent::ItemUseUnavailable]);
    assert_eq!(game.rng.draw_counter, draws_before);
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == ITEM_ID)
            .and_then(|item| item.charges),
        Some(charges_before)
    );

    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        Some(&TargetSelection::Direction {
            direction: Direction::East,
        }),
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("valid wand activation should resolve");
    assert!(events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::ItemActivationHit { .. } | DomainEvent::ItemActivationSlew { .. }
        )
    }));
    let item = game
        .items
        .iter()
        .find(|item| item.id == ITEM_ID)
        .expect("charged wand should remain in inventory");
    let activation = item
        .activation
        .as_ref()
        .expect("wand activation should remain materialized");
    assert_eq!(
        item.charges.expect("wand charges should remain").current,
        charges_before.current - activation.cost
    );
}

#[test]
fn saving_throw_skill_check_resists_or_applies_the_same_trap() {
    let trap_position = Position { x: 4, y: 3 };
    let mut tinkerer = skill_check_game(2, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(2, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        replace_terrain(game, trap_position, "demo.terrain.trap-resonance-ward");
    }
    let tinkerer_hp = tinkerer.player.hp;
    let vanguard_hp = vanguard.player.hp;

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.saving-throw-success");
    let failure = check_resolution(&vanguard_update, "skill.saving-throw-failure");

    assert_check(success, "demo.skill.saving-throw", 45, 40, 13, Some(33), 30);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.saving-throw", 29, 40, 13, Some(20), 30);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert_eq!(tinkerer.player.hp, tinkerer_hp);
    assert!(vanguard.player.hp < vanguard_hp);
    assert!(tinkerer.revealed_terrain.contains(&trap_position));
    assert!(vanguard.revealed_terrain.contains(&trap_position));
    assert!(
        !tinkerer_update
            .events
            .iter()
            .any(|event| event.kind == "terrain.trap-triggered")
    );
    assert!(
        vanguard_update
            .events
            .iter()
            .any(|event| event.kind == "terrain.trap-triggered")
    );
}

#[test]
fn passive_perception_skill_check_reveals_only_for_the_high_skill_build() {
    let rune_position = Position { x: 5, y: 3 };
    let mut tinkerer = skill_check_game(1, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(1, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        replace_terrain(game, rune_position, "demo.terrain.echo-rune-hidden");
    }

    let tinkerer_update = dispatch_next(
        &mut tinkerer,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let vanguard_update = dispatch_next(
        &mut vanguard,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let success = check_resolution(&tinkerer_update, "skill.perception-success");
    let failure = check_resolution(&vanguard_update, "skill.perception-failure");

    assert_check(success, "demo.skill.perception", 25, 24, 83, Some(21), 18);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.perception", 4, 24, 83, Some(3), 18);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert!(tinkerer.revealed_terrain.contains(&rune_position));
    assert!(!vanguard.revealed_terrain.contains(&rune_position));
    assert_eq!(
        tinkerer.known_terrain_at(rune_position),
        "demo.terrain.echo-rune-hidden"
    );
    assert_eq!(
        vanguard.known_terrain_at(rune_position),
        "demo.terrain.wall"
    );
}

#[test]
fn stealth_skill_check_controls_alertness_and_alerted_save_compatibility() {
    const LISTENER_ID: &str = "test.monster.echo-listener.1";
    let listener_position = Position { x: 7, y: 3 };
    let mut tinkerer = skill_check_game(5, "demo.build.tinkerer");
    let mut vanguard = skill_check_game(5, "demo.build.vanguard");
    for game in [&mut tinkerer, &mut vanguard] {
        game.entities.push(game.generated_actor(
            LISTENER_ID.to_owned(),
            "demo.actor.echo-listener",
            listener_position,
        ));
    }

    let tinkerer_update = dispatch_next(&mut tinkerer, GameCommand::Wait);
    let vanguard_update = dispatch_next(&mut vanguard, GameCommand::Wait);
    let success = check_resolution(&tinkerer_update, "skill.stealth-success");
    let failure = check_resolution(&vanguard_update, "skill.stealth-failure");

    assert_check(success, "demo.skill.stealth", 7, 7, 93, Some(5), 5);
    assert_eq!(success.outcome, CheckOutcomeDto::Success);
    assert_check(failure, "demo.skill.stealth", 1, 7, 93, Some(0), 5);
    assert_eq!(failure.outcome, CheckOutcomeDto::Failure);
    assert!(
        tinkerer
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| !entity.alerted && entity.position == listener_position)
    );
    assert!(
        vanguard
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted && entity.position != listener_position)
    );

    let saved = vanguard.to_save();
    assert!(
        saved
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted == Some(true))
    );
    let restored = Game::from_save(saved.clone()).expect("alerted actor should reload");
    assert_eq!(restored.state_hash(), vanguard.state_hash());
    assert!(
        restored
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| entity.alerted)
    );

    let mut legacy = saved;
    legacy
        .entities
        .iter_mut()
        .find(|entity| entity.id == LISTENER_ID)
        .expect("listener save should exist")
        .alerted = None;
    let migrated = Game::from_save(legacy).expect("missing alert state should use content default");
    assert!(
        migrated
            .entities
            .iter()
            .find(|entity| entity.id == LISTENER_ID)
            .is_some_and(|entity| !entity.alerted)
    );
}

fn skill_check_game(seed: u64, build_id: &str) -> Game {
    let mut game = Game::new_with_build(seed, build_id).expect("skill-check build should create");
    clear_monsters(&mut game);
    game
}

fn give_inventory_item(game: &mut Game, id: &str, kind_id: &str) {
    let (activation, charges) =
        initial_item_runtime_state(&game.content, &mut game.rng, kind_id, 1);
    game.items.push(ItemInstance {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation,
        charges,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
}

fn replace_terrain(game: &mut Game, position: Position, terrain_id: &str) {
    let index = game
        .index(position)
        .expect("test terrain should be in bounds");
    game.terrain[index] = terrain_id.to_owned();
}

fn check_resolution<'a>(update: &'a GameUpdate, event_kind: &str) -> &'a CheckResolutionDto {
    update
        .events
        .iter()
        .find(|event| event.kind == event_kind)
        .and_then(|event| event.outcome.as_ref())
        .and_then(|outcome| match outcome {
            GameEventOutcomeDto::Check { resolution } => Some(resolution),
            _ => None,
        })
        .unwrap_or_else(|| panic!("check event {event_kind} should exist"))
}

fn ability_book_item_id(game: &Game) -> String {
    ability_book_item_id_for(game, "demo.item.echo-primer")
}

fn ability_book_item_id_for(game: &Game, kind_id: &str) -> String {
    game.items
        .iter()
        .find(|item| item.kind_id == kind_id && item.location == ItemLocation::Inventory)
        .map(|item| item.id.clone())
        .unwrap_or_else(|| panic!("scholar should carry {kind_id}"))
}

fn ability_cast_resolution(update: &GameUpdate) -> &AbilityCastResolutionDto {
    update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::AbilityCast { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("ability cast resolution should exist")
}

fn assert_teleport_target_rejected(game: &mut Game, ability_id: &str, target: TargetSelection) {
    let position_before = game.player.position;
    let mana_before = game.resources["demo.resource.mana"].current;
    let draws_before = game.rng_draw_counter();
    let progress_before = game.ability_progress[ability_id];
    let update = dispatch_next(
        game,
        GameCommand::CastAbility {
            ability_id: ability_id.to_owned(),
            target,
        },
    );
    assert_eq!(game.player.position, position_before);
    assert_eq!(game.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.ability_progress[ability_id], progress_before);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.target-unavailable")
    );
    assert!(!update.events.iter().any(|event| {
        matches!(
            event.outcome.as_ref(),
            Some(GameEventOutcomeDto::AbilityCast { .. })
                | Some(GameEventOutcomeDto::AbilityTeleport { .. })
        )
    }));
}

fn rest_resolution(update: &GameUpdate) -> &RestResolutionDto {
    update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::Rest { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("rest resolution should exist")
}

fn assert_check(
    resolution: &CheckResolutionDto,
    skill_id: &str,
    ability: i32,
    difficulty: i32,
    percentile_roll: u8,
    contest_roll: Option<i32>,
    threshold: i32,
) {
    assert_eq!(resolution.skill_id, skill_id);
    assert_eq!(resolution.ability, ability);
    assert_eq!(resolution.difficulty, difficulty);
    assert_eq!(resolution.percentile_roll, percentile_roll);
    assert_eq!(resolution.contest_roll, contest_roll);
    assert_eq!(resolution.threshold, threshold);
}

#[test]
fn monster_casting_uses_frequency_viability_and_weighted_selection() {
    let mut selected = BTreeSet::new();
    let mut fallback_count = 0_u32;
    let mut binding_round_trip_checked = false;
    for seed in 0..256_u64 {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        let draw_counter_before = game.rng.draw_counter;
        let mut events = Vec::new();

        let cast = game.resolve_monster_ability(0, &mut events);
        let decision = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::MonsterAbilityDecision { resolution } => Some(resolution),
                _ => None,
            })
            .expect("caster action should expose its decision");
        assert_eq!(decision.frequency_percent, 50);
        assert!((1..=100).contains(&decision.frequency_roll));
        assert_eq!(
            decision.viable_ability_ids,
            [
                "demo.ability.resonant-bolt".to_owned(),
                "demo.ability.echo-binding".to_owned(),
                "demo.ability.echo-burst".to_owned(),
                "demo.ability.echo-lance".to_owned(),
                "demo.ability.echo-fan".to_owned(),
                "demo.ability.echo-quickening".to_owned(),
                "demo.ability.call-discord".to_owned(),
            ]
        );
        assert_eq!(decision.total_weight, 24);

        match decision.selected_ability_id.as_deref() {
            None => {
                fallback_count += 1;
                assert!(!cast);
                assert!(decision.frequency_roll > decision.frequency_percent);
                assert!(decision.selection_roll.is_none());
                assert_eq!(game.rng.draw_counter, draw_counter_before + 1);
            }
            Some(ability_id) => {
                assert!(cast);
                selected.insert(ability_id.to_owned());
                let roll = decision
                    .selection_roll
                    .expect("a successful frequency check should select by weight");
                assert!((1..=decision.total_weight).contains(&roll));
                let cast_resolution = events
                    .iter()
                    .find_map(|event| match event {
                        DomainEvent::MonsterAbilityCast { resolution, .. } => Some(resolution),
                        _ => None,
                    })
                    .expect("selected ability should resolve");
                assert_eq!(cast_resolution.ability_id, ability_id);
                if matches!(
                    ability_id,
                    "demo.ability.echo-quickening" | "demo.ability.call-discord"
                ) {
                    assert_eq!(
                        cast_resolution.target_entity_id,
                        "test.monster.echo-cantor.1"
                    );
                } else {
                    assert_eq!(cast_resolution.target_entity_id, game.player.id);
                }
                if ability_id == "demo.ability.echo-binding" {
                    assert_eq!(cast_resolution.effects.len(), 2);
                    assert!(matches!(
                        cast_resolution.effects[1],
                        AbilityEffectResolutionDto::ApplyStatus { .. }
                    ));
                    let restored = Game::from_save(game.to_save())
                        .expect("monster-applied status should round-trip");
                    assert_eq!(restored.state_hash(), game.state_hash());
                    binding_round_trip_checked = true;
                } else if ability_id == "demo.ability.echo-quickening" {
                    assert_eq!(cast_resolution.effects.len(), 2);
                } else if ability_id != "demo.ability.call-discord" {
                    assert_eq!(cast_resolution.effects.len(), 1);
                }
            }
        }
    }
    assert!(fallback_count > 0);
    assert_eq!(
        selected,
        BTreeSet::from([
            "demo.ability.call-discord".to_owned(),
            "demo.ability.echo-binding".to_owned(),
            "demo.ability.echo-burst".to_owned(),
            "demo.ability.echo-fan".to_owned(),
            "demo.ability.echo-lance".to_owned(),
            "demo.ability.echo-quickening".to_owned(),
            "demo.ability.resonant-bolt".to_owned(),
        ])
    );
    assert!(binding_round_trip_checked);
}

#[test]
fn monster_casting_clean_shot_filter_blocks_allies_and_walls() {
    for blocked_by_actor in [true, false] {
        let mut game = Game::new(1);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        if blocked_by_actor {
            game.entities.push(game.generated_actor(
                "test.monster.blocker.1".to_owned(),
                "demo.actor.ember-mote",
                Position { x: 6, y: 3 },
            ));
        } else {
            replace_terrain(&mut game, Position { x: 6, y: 3 }, "demo.terrain.wall");
        }
        let draw_counter_before = game.rng.draw_counter;
        let mut events = Vec::new();

        game.resolve_monster_ability(0, &mut events);
        let decision = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::MonsterAbilityDecision { resolution } => Some(resolution),
                _ => None,
            })
            .expect("blocked caster should still expose its frequency decision");
        for ability_id in ["demo.ability.resonant-bolt", "demo.ability.echo-binding"] {
            let candidate = decision
                .candidates
                .iter()
                .find(|candidate| candidate.ability_id == ability_id)
                .expect("direct spell should remain observable");
            assert_eq!(candidate.effective_weight, 0);
            assert_eq!(
                candidate.rejection_reason,
                Some(if blocked_by_actor {
                    MonsterAbilityRejectionReasonDto::FriendlyRisk
                } else {
                    MonsterAbilityRejectionReasonDto::Blocked
                })
            );
        }
        assert!(decision.total_weight > 0);
        assert!(game.rng.draw_counter > draw_counter_before);
    }
}

#[test]
fn monster_casting_utility_uses_wounds_status_and_distance_without_rng() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let draws_before = game.rng.draw_counter;
    let healing = game
        .content
        .ability("demo.ability.mending-echo")
        .expect("mending echo should exist")
        .clone();
    assert_eq!(
        game.monster_ability_plan(0, healing.clone(), 4)
            .expect_err("healthy healing should have no utility")
            .reason,
        MonsterAbilityRejectionReasonDto::NoUtility
    );

    game.entities[0].hp = 5;
    let wounded = game
        .monster_ability_plan(0, healing.clone(), 4)
        .expect("more than twenty percent wounds should enable healing");
    assert_eq!(wounded.base_weight, 4);
    assert_eq!(wounded.effective_weight, 8);
    game.entities[0].hp = 1;
    assert_eq!(
        game.monster_ability_plan(0, healing, 4)
            .expect("deep wounds should increase healing weight")
            .effective_weight,
        16
    );

    let quickening = game
        .content
        .ability("demo.ability.echo-quickening")
        .expect("quickening should exist")
        .clone();
    assert!(game.monster_ability_plan(0, quickening.clone(), 2).is_ok());
    game.entities[0].statuses.push(StatusInstance {
        kind_id: STATUS_HASTE.to_owned(),
        intensity: 1,
        remaining_ticks: 30,
        source_id: Some(quickening.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    });
    assert_eq!(
        game.monster_ability_plan(0, quickening, 2)
            .expect_err("duplicate haste should have no utility")
            .reason,
        MonsterAbilityRejectionReasonDto::NoUtility
    );

    let bolt = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    assert_eq!(
        game.monster_ability_plan(0, bolt.clone(), 3)
            .expect("distant bolt should be viable")
            .effective_weight,
        6
    );
    game.entities[0].position = Position { x: 5, y: 3 };
    assert_eq!(
        game.monster_ability_plan(0, bolt, 3)
            .expect("near bolt should be viable")
            .effective_weight,
        3
    );
    assert_eq!(game.rng.draw_counter, draws_before);
}

#[test]
fn monster_multi_target_plans_reject_secondary_entities() {
    for ability_id in [
        "demo.ability.echo-burst",
        "demo.ability.echo-lance",
        "demo.ability.echo-fan",
    ] {
        let mut game = Game::new(1);
        clear_monsters(&mut game);
        game.entities.push(game.generated_actor(
            "test.monster.echo-cantor.1".to_owned(),
            "demo.actor.echo-cantor",
            Position { x: 8, y: 3 },
        ));
        let ability = game
            .content
            .ability(ability_id)
            .expect("multi-target ability should exist")
            .clone();
        assert!(
            game.monster_ability_plan(0, ability.clone(), 2).is_ok(),
            "{ability_id} should be viable with only its primary target"
        );
        game.entities.push(game.generated_actor(
            "test.monster.secondary.1".to_owned(),
            "demo.actor.ember-mote",
            Position { x: 4, y: 3 },
        ));
        assert_eq!(
            game.monster_ability_plan(0, ability, 2)
                .expect_err("friendly footprint should be rejected")
                .reason,
            MonsterAbilityRejectionReasonDto::FriendlyRisk,
            "{ability_id} should reject a secondary entity in its footprint"
        );
    }
}

#[test]
fn monster_summons_are_hostile_owned_active_and_saveable() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 5, y: 3 },
    ));
    let ability = game
        .content
        .ability("demo.ability.call-discord")
        .expect("hostile summon ability should exist")
        .clone();
    let plan = game
        .monster_ability_plan(0, ability, 2)
        .expect("open cells should permit hostile summoning");
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &plan,
        &mut events,
        &mut changed,
        &mut Vec::new(),
    );
    let summon = resolution
        .summon
        .expect("summon resolution should be explicit");
    assert!(resolution.effects.is_empty());
    assert!(resolution.targets.is_empty());
    assert!(resolution.trace.is_none());
    assert_eq!(summon.owner_id, "test.monster.echo-cantor.1");
    assert_eq!(summon.entity_ids.len(), 2);
    assert_eq!(resolution.affected_positions, summon.positions);
    assert!(
        resolution
            .affected_positions
            .iter()
            .all(|position| changed.contains(position))
    );
    let entities = game.entities_dto();
    for entity_id in &summon.entity_ids {
        let entity = entities
            .iter()
            .find(|entity| &entity.id == entity_id)
            .expect("summoned entity should be projected");
        assert_eq!(entity.faction, EntityFactionDto::Hostile);
        assert_eq!(
            entity
                .summon
                .as_ref()
                .expect("summon identity should be projected")
                .owner_id,
            "test.monster.echo-cantor.1"
        );
    }

    let hp_before = game.player.hp;
    let summon_index = game
        .entities
        .iter()
        .position(|entity| entity.id == summon.entity_ids[0])
        .expect("first summon should remain present");
    game.resolve_monster_action(
        summon_index,
        &mut events,
        &mut changed,
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated hostile summon action should resolve");
    assert!(game.player.hp < hp_before);

    let restored =
        Game::from_save(game.to_save()).expect("hostile summon should round-trip through save");
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn monster_spells_target_nearby_player_summons_and_score_enemy_footprints() {
    let mut game = Game::new(1);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut companion = game.generated_actor(
        "test.summon.echo-companion.1".to_owned(),
        "demo.actor.echo-companion",
        Position { x: 7, y: 3 },
    );
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns: 5,
    });
    game.entities.push(companion);

    let bolt = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    let bolt_plan = game
        .monster_ability_plan(0, bolt, 3)
        .expect("nearby companion should be a legal target");
    assert_eq!(
        monster_plan_target(&bolt_plan.target).map(MonsterHostileTarget::entity_id),
        Some("test.summon.echo-companion.1")
    );
    assert_eq!(bolt_plan.enemy_target_count, 1);

    game.entities[1].position = Position { x: 3, y: 4 };
    let burst = game
        .content
        .ability("demo.ability.echo-burst")
        .expect("burst should exist")
        .clone();
    let burst_plan = game
        .monster_ability_plan(0, burst, 2)
        .expect("a player and nearby companion should both be legal enemies");
    assert_eq!(burst_plan.enemy_target_count, 2);
    assert_eq!(burst_plan.friendly_risk_count, 0);
    assert_eq!(burst_plan.effective_weight, 8);
}

#[test]
fn monster_area_damage_hits_every_player_aligned_target_and_removes_slain_summons() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut companion = game.generated_actor(
        "test.summon.echo-companion.1".to_owned(),
        "demo.actor.echo-companion",
        Position { x: 3, y: 4 },
    );
    companion.hp = 1;
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns: 5,
    });
    game.entities.push(companion);
    let ability = game
        .content
        .ability("demo.ability.echo-burst")
        .expect("burst should exist")
        .clone();
    let plan = game
        .monster_ability_plan(0, ability, 2)
        .expect("burst should cover both enemies");
    let player_hp_before = game.player.hp;
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    let resolution = game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &plan,
        &mut Vec::new(),
        &mut changed,
        &mut removed,
    );
    assert_eq!(resolution.targets.len(), 2);
    assert!(game.player.hp < player_hp_before);
    assert_eq!(removed, ["test.summon.echo-companion.1"]);
    assert!(
        game.entities
            .iter()
            .all(|entity| entity.id != "test.summon.echo-companion.1")
    );
    assert!(changed.contains(&Position { x: 3, y: 4 }));
}

#[test]
fn smart_caster_learns_only_observed_player_resistance_and_round_trips() {
    let mut game = Game::new(3);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    game.player
        .resistances
        .set(DamageType::Electricity, ResistanceLevel::Resistant);
    let ability = game
        .content
        .ability("demo.ability.resonant-bolt")
        .expect("bolt should exist")
        .clone();
    let before = game
        .monster_ability_plan(0, ability.clone(), 3)
        .expect("unknown resistance must not affect the first decision");
    assert_eq!(before.effective_weight, 6);
    assert!(game.entities[0].observed_player_resistances.is_empty());

    game.resolve_monster_ability_plan(
        0,
        "demo.actor.echo-cantor",
        &before,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert_eq!(
        game.entities[0]
            .observed_player_resistances
            .get(&DamageType::Electricity),
        Some(&ResistanceLevel::Resistant)
    );
    let after = game
        .monster_ability_plan(0, ability, 3)
        .expect("observed resistance should downweight rather than forbid the bolt");
    assert_eq!(after.effective_weight, 3);

    let restored = Game::from_save(game.to_save()).expect("resistance memory should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(
        restored.entities[0].observed_player_resistances,
        game.entities[0].observed_player_resistances
    );
}

#[test]
fn caster_keeps_distance_and_flees_when_wounded_without_extra_rng() {
    let mut game = Game::new(4);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 5, y: 3 },
    ));
    game.entities[0].casting_cooldown_remaining = 1;
    let draws_before = game.rng.draw_counter;
    let mut events = Vec::new();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated tactical action should resolve");
    assert_eq!(game.entities[0].position, Position { x: 6, y: 2 });
    assert_eq!(game.rng.draw_counter, draws_before);
    assert!(
        events
            .iter()
            .any(|event| { matches!(event, DomainEvent::MonsterKeptDistance { .. }) })
    );

    game.entities[0].position = Position { x: 5, y: 3 };
    game.entities[0].hp = 2;
    game.entities[0].casting_cooldown_remaining = 1;
    events.clear();
    game.resolve_monster_action(
        0,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
        &mut BTreeSet::new(),
    )
    .expect("validated tactical action should resolve");
    assert_eq!(game.entities[0].position, Position { x: 6, y: 2 });
    assert!(
        events
            .iter()
            .any(|event| { matches!(event, DomainEvent::MonsterFled { .. }) })
    );
}

#[test]
fn monster_casting_cooldown_uses_inverse_frequency_without_rng() {
    assert_eq!(monster_casting_cooldown(50), 2);
    assert_eq!(monster_casting_cooldown(25), 4);
    assert_eq!(monster_casting_cooldown(30), 4);
    let seed = (0..1_000_u64)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            game.entities.push(game.generated_actor(
                "test.monster.echo-cantor.1".to_owned(),
                "demo.actor.echo-cantor",
                Position { x: 8, y: 3 },
            ));
            game.resolve_monster_ability(0, &mut Vec::new())
        })
        .expect("a deterministic seed should pass the frequency check");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));

    assert!(game.resolve_monster_ability(0, &mut Vec::new()));
    assert_eq!(game.entities[0].casting_cooldown_remaining, 2);
    let draws_after_cast = game.rng.draw_counter;
    for expected_remaining in [1, 0] {
        let mut events = Vec::new();
        assert!(!game.resolve_monster_ability(0, &mut events));
        assert!(events.is_empty());
        assert_eq!(
            game.entities[0].casting_cooldown_remaining,
            expected_remaining
        );
        assert_eq!(game.rng.draw_counter, draws_after_cast);
    }

    game.resolve_monster_ability(0, &mut Vec::new());
    assert!(game.rng.draw_counter > draws_after_cast);
    let restored =
        Game::from_save(game.to_save()).expect("monster cooldown should round-trip through save");
    assert_eq!(
        restored.entities[0].casting_cooldown_remaining,
        game.entities[0].casting_cooldown_remaining
    );
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn lethal_monster_sequence_skips_later_status_without_extra_rng() {
    let seed = (0..1_000_u64)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            game.entities.push(game.generated_actor(
                "test.monster.echo-cantor.1".to_owned(),
                "demo.actor.echo-cantor",
                Position { x: 8, y: 3 },
            ));
            let mut events = Vec::new();
            game.resolve_monster_ability(0, &mut events);
            events.iter().any(|event| {
                matches!(
                    event,
                    DomainEvent::MonsterAbilityCast { resolution, .. }
                        if resolution.ability_id == "demo.ability.echo-binding"
                )
            })
        })
        .expect("a deterministic seed should select echo binding");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    game.player.hp = 0;
    game.entities.push(game.generated_actor(
        "test.monster.echo-cantor.1".to_owned(),
        "demo.actor.echo-cantor",
        Position { x: 8, y: 3 },
    ));
    let mut events = Vec::new();

    assert!(game.resolve_monster_ability(0, &mut events));
    let resolution = events
        .iter()
        .find_map(|event| match event {
            DomainEvent::MonsterAbilityCast { resolution, .. } => Some(resolution),
            _ => None,
        })
        .expect("binding should resolve");
    assert_eq!(resolution.ability_id, "demo.ability.echo-binding");
    assert!(matches!(
        resolution.effects[0],
        AbilityEffectResolutionDto::Damage { .. }
    ));
    assert_eq!(
        resolution.effects[1],
        AbilityEffectResolutionDto::Skipped {
            effect_index: 1,
            reason: AbilityEffectSkipReasonDto::TargetDead,
        }
    );
    assert!(game.player_is_dead());
    assert!(
        !game
            .player
            .statuses
            .iter()
            .any(|status| status.kind_id == STATUS_SLOW)
    );
}

fn collect_both_demo_items(game: &mut Game) {
    game.dispatch(command(
        1,
        0,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("movement to shard should execute");
    game.dispatch(command(2, 1, GameCommand::PickUp))
        .expect("shard pickup should execute");
    game.dispatch(command(
        3,
        2,
        GameCommand::Move {
            direction: Direction::East,
        },
    ))
    .expect("movement to charm should execute");
    game.dispatch(command(4, 3, GameCommand::PickUp))
        .expect("charm pickup should execute");
}

fn add_player_summon(game: &mut Game, entity_id: &str, position: Position, remaining_turns: u16) {
    let mut companion =
        game.generated_actor(entity_id.to_owned(), "demo.actor.echo-companion", position);
    companion.summon = Some(SummonIdentity {
        owner_id: game.player.id.clone(),
        source_ability_id: "demo.ability.echo-companion".to_owned(),
        remaining_turns,
    });
    game.entities.push(companion);
}

#[test]
fn summon_commands_are_zero_world_time_persistent_and_guard_the_issue_position() {
    let mut game = Game::new(89);
    clear_monsters(&mut game);
    add_player_summon(
        &mut game,
        "test.summon.echo-companion.1",
        Position { x: 4, y: 3 },
        5,
    );
    let before = game.to_save();
    let update = dispatch_next(
        &mut game,
        GameCommand::SetSummonCommand {
            mode: SummonCommandModeDto::Guard,
        },
    );

    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(update.player.energy_need, before.player.energy_need);
    assert_eq!(game.rng.draw_counter, before.rng.draw_counter);
    assert_eq!(
        update.player.summon_command,
        SummonCommandDto {
            mode: SummonCommandModeDto::Guard,
            guard_position: Some(Position { x: 3, y: 3 }),
        }
    );
    let resolution = update
        .events
        .iter()
        .find_map(|event| match &event.outcome {
            Some(GameEventOutcomeDto::SummonCommand { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("summon command should have a structured outcome");
    assert_eq!(resolution.affected_summons, 1);
    let restored = Game::from_save(game.to_save()).expect("summon command should round-trip");
    assert_eq!(restored.summon_command, game.summon_command);

    let mut malformed = game.to_save();
    malformed.player.summon_command.mode = SummonCommandModeDto::Follow;
    assert!(matches!(
        Game::from_save(malformed),
        Err(CoreError::InvalidSave(
            "non-guard summon command retains a guard position"
        ))
    ));
}

#[test]
fn player_summons_follow_attack_keep_distance_and_guard_deterministically() {
    let resolve = |mode: SummonCommandModeDto,
                   summon_position: Position,
                   guard_position: Option<Position>| {
        let mut game = Game::new(89);
        clear_monsters(&mut game);
        add_player_summon(
            &mut game,
            "test.summon.echo-companion.1",
            summon_position,
            5,
        );
        game.entities.push(game.generated_actor(
            "test.monster.ember-mote.1".to_owned(),
            "demo.actor.ember-mote",
            Position { x: 10, y: 3 },
        ));
        game.summon_command = SummonCommandDto {
            mode,
            guard_position,
        };
        let rng_before = game.rng.draw_counter;
        let mut changed = BTreeSet::new();
        game.resolve_player_summon_action(0, &mut Vec::new(), &mut changed, &mut Vec::new())
            .expect("summon action should resolve");
        (
            game.entities[0].position,
            changed,
            game.rng.draw_counter - rng_before,
        )
    };

    let (follow, _, follow_rng) =
        resolve(SummonCommandModeDto::Follow, Position { x: 7, y: 3 }, None);
    assert_eq!(follow, Position { x: 6, y: 3 });
    assert_eq!(follow_rng, 0);

    let (attack, _, attack_rng) =
        resolve(SummonCommandModeDto::Attack, Position { x: 7, y: 3 }, None);
    assert_eq!(attack, Position { x: 8, y: 3 });
    assert_eq!(attack_rng, 0);

    let (keep_distance, _, keep_distance_rng) = resolve(
        SummonCommandModeDto::KeepDistance,
        Position { x: 4, y: 3 },
        None,
    );
    assert_eq!(keep_distance, Position { x: 5, y: 2 });
    assert_eq!(keep_distance_rng, 0);

    let (guard, _, guard_rng) = resolve(
        SummonCommandModeDto::Guard,
        Position { x: 7, y: 3 },
        Some(Position { x: 3, y: 3 }),
    );
    assert_eq!(guard, Position { x: 6, y: 3 });
    assert_eq!(guard_rng, 0);
}

#[test]
fn attacking_summon_uses_actor_melee_and_player_owned_death_credit() {
    let seed = (1..=256)
        .find(|seed| {
            let mut game = Game::new(*seed);
            clear_monsters(&mut game);
            add_player_summon(
                &mut game,
                "test.summon.echo-companion.1",
                Position { x: 4, y: 3 },
                5,
            );
            let mut target = game.generated_actor(
                "test.monster.ember-mote.1".to_owned(),
                "demo.actor.ember-mote",
                Position { x: 5, y: 3 },
            );
            target.hp = 1;
            game.entities.push(target);
            game.summon_command.mode = SummonCommandModeDto::Attack;
            let mut events = Vec::new();
            game.resolve_player_summon_action(
                0,
                &mut events,
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .expect("summon melee should resolve");
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::SummonSlew { .. }))
        })
        .expect("a bounded deterministic seed should let the summon hit");
    let mut game = Game::new(seed);
    clear_monsters(&mut game);
    add_player_summon(
        &mut game,
        "test.summon.echo-companion.1",
        Position { x: 4, y: 3 },
        5,
    );
    let mut target = game.generated_actor(
        "test.monster.ember-mote.1".to_owned(),
        "demo.actor.ember-mote",
        Position { x: 5, y: 3 },
    );
    target.hp = 1;
    game.entities.push(target);
    game.summon_command.mode = SummonCommandModeDto::Attack;
    let experience_before = game.progress.experience;
    let mut events = Vec::new();
    let mut removed = Vec::new();
    game.resolve_player_summon_action(0, &mut events, &mut BTreeSet::new(), &mut removed)
        .expect("summon melee should resolve");

    assert_eq!(removed, ["test.monster.ember-mote.1"]);
    assert!(game.progress.experience > experience_before);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            DomainEvent::SummonSlew {
                source_kind_id,
                target_kind_id,
                ..
            } if source_kind_id == "demo.actor.echo-companion"
                && target_kind_id == "demo.actor.ember-mote"
        )
    }));
}

#[test]
fn nearby_player_summons_follow_across_floors_while_distant_summons_stay() {
    let mut game = Game::new(89);
    clear_monsters(&mut game);
    game.player.position = Position { x: 3, y: 4 };
    add_player_summon(&mut game, "test.summon.near", Position { x: 4, y: 4 }, 5);
    add_player_summon(
        &mut game,
        "test.summon.distant",
        Position { x: 10, y: 10 },
        5,
    );

    let transition = game
        .traverse_stairs(false)
        .expect("floor traversal should resolve")
        .expect("entrance should transition");
    assert_eq!(
        transition.summons_followed,
        [(
            "test.summon.near".to_owned(),
            "demo.actor.echo-companion".to_owned()
        )]
    );
    assert!(transition.summons_could_not_follow.is_empty());
    assert!(game.entities.iter().any(|entity| {
        entity.id == "test.summon.near"
            && chebyshev_distance(entity.position, game.player.position) <= 5
    }));
    assert!(
        stored_floor(&game, "demo.floor.surface")
            .entities
            .iter()
            .any(|entity| entity.id == "test.summon.distant")
    );
    assert!(
        stored_floor(&game, "demo.floor.surface")
            .entities
            .iter()
            .all(|entity| entity.id != "test.summon.near")
    );
}

#[test]
fn escape_teleport_falls_back_to_half_distance_and_blink_rejects_without_space() {
    fn open_cell(game: &mut Game, position: Position) {
        let index = game.index(position).expect("cell");
        game.terrain[index] = "demo.terrain.floor".to_owned();
    }

    // Escape fallback: the only open landing sits five tiles from the player,
    // so the minimum-eight filter is empty and the halved minimum applies.
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    for cell in game.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let player = game.player.position;
    open_cell(&mut game, player);
    let stalker_position = Position {
        x: player.x + 1,
        y: player.y,
    };
    open_cell(&mut game, stalker_position);
    let landing = Position {
        x: player.x + 5,
        y: player.y,
    };
    open_cell(&mut game, landing);
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.rift-test",
        "demo.actor.rift-stalker",
        stalker_position,
        7,
        110,
        100,
        true,
    ));
    let mut escaped = false;
    for _ in 0..30 {
        let update = dispatch_next(&mut game, GameCommand::Wait);
        if update.events.iter().any(|event| {
            event.kind == "monster.teleported"
                && matches!(
                    event.outcome.as_ref(),
                    Some(GameEventOutcomeDto::MonsterDisplacement { resolution })
                        if resolution.to == landing
                )
        }) {
            escaped = true;
            break;
        }
        if game.player_is_dead() {
            break;
        }
    }
    assert!(escaped, "escape should use the halved minimum distance");

    // Blink rejection: every cell within radius five is walled, so the
    // planner reports no-space without drawing any destination RNG.
    let mut boxed = Game::new(0);
    clear_monsters(&mut boxed);
    for cell in boxed.terrain.iter_mut() {
        *cell = "demo.terrain.wall".to_owned();
    }
    let boxed_player = boxed.player.position;
    open_cell(&mut boxed, boxed_player);
    let boxed_stalker = Position {
        x: boxed_player.x + 1,
        y: boxed_player.y,
    };
    open_cell(&mut boxed, boxed_stalker);
    boxed.entities.push(actor_from_runtime_spawn(
        "generated.actor.rift-boxed",
        "demo.actor.rift-stalker",
        boxed_stalker,
        7,
        110,
        100,
        true,
    ));
    let mut saw_rejection = false;
    for _ in 0..30 {
        let update = dispatch_next(&mut boxed, GameCommand::Wait);
        for event in &update.events {
            if let Some(GameEventOutcomeDto::MonsterAbilityDecision { resolution }) =
                event.outcome.as_ref()
                && resolution.candidates.iter().any(|candidate| {
                    candidate.ability_id == "demo.ability.echo-slip"
                        && candidate.rejection_reason
                            == Some(MonsterAbilityRejectionReasonDto::NoSpace)
                })
            {
                saw_rejection = true;
            }
        }
        if saw_rejection || boxed.player_is_dead() {
            break;
        }
    }
    assert!(saw_rejection, "boxed blink should report no-space");
}

#[test]
fn offensive_flag_multipliers_and_living_predicate_match_original_tiers() {
    assert_eq!(slay_multiplier(SlayTarget::Evil, SlayLevel::Slay), 19);
    assert_eq!(slay_multiplier(SlayTarget::Animal, SlayLevel::Kill), 46);
    assert_eq!(slay_multiplier(SlayTarget::Dragon, SlayLevel::Slay), 28);
    assert_eq!(slay_multiplier(SlayTarget::Dragon, SlayLevel::Kill), 56);

    let game = Game::new(0);
    let dragon = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo dragon");
    let construct = game
        .content
        .actor("demo.actor.resonant-warden")
        .expect("demo construct");
    assert!(slay_target_matches(SlayTarget::Dragon, dragon));
    assert!(slay_target_matches(SlayTarget::Living, dragon));
    assert!(!slay_target_matches(SlayTarget::Living, construct));
}

#[test]
fn elemental_brand_is_suppressed_only_by_matching_immunity() {
    let mut game = Game::new(0);
    clear_monsters(&mut game);
    game.items.push(ItemInstance {
        id: "test.item.ember-edge".to_owned(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let profile = game.player_melee_profile(&game.player_derived_stats());
    let definition = game
        .content
        .actor("demo.actor.ash-drake")
        .expect("demo target")
        .clone();
    let mut target = actor_from_runtime_spawn(
        "test.actor.brand-target",
        &definition.id,
        Position { x: 4, y: 3 },
        definition.max_hp,
        definition.speed,
        0,
        true,
    );

    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Resistant);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        24
    );
    target
        .resistances
        .set(DamageType::Fire, ResistanceLevel::Immune);
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &definition),
        10
    );
}

#[test]
fn offensive_flag_dto_hides_unknown_affix_contributions() {
    let mut game = Game::new(0);
    let item_id = "test.item.known-offense".to_owned();
    game.items.push(ItemInstance {
        id: item_id.clone(),
        kind_id: "demo.item.ember-edge".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.frost-hunter".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });

    let hidden = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(hidden.brands, vec![WeaponBrandDto::Fire]);
    assert!(hidden.slays.is_empty());

    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::from(["demo.affix.frost-hunter".to_owned()]),
        },
    );
    let visible = game
        .inventory_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("test item");
    assert_eq!(
        visible.brands,
        vec![WeaponBrandDto::Fire, WeaponBrandDto::Cold]
    );
    assert_eq!(
        visible.slays,
        vec![SlayDto {
            target: SlayTargetDto::Animal,
            level: SlayLevelDto::Slay,
        }]
    );
}

#[test]
fn dynamic_affix_rolls_are_seeded_depth_filtered_and_materialized() {
    let roll = |seed, depth| {
        let mut game = Game::new(seed);
        game.roll_affix_properties(&["demo.affix.adaptive-echo".to_owned()], depth)
    };
    let shallow = roll(17, 1);
    assert_eq!(shallow, roll(17, 1));
    assert_eq!(shallow.len(), 1);
    assert!(
        shallow[0].properties.equipment_bonuses.melee_skill == 12
            || shallow[0].properties.equipment_bonuses.melee_attacks == 1
    );
    let deep = roll(17, 10);
    assert_eq!(deep.len(), 1);
    assert!(
        deep[0].properties.equipment_bonuses.device_skill == 8
            || deep[0].properties.equipment_bonuses.melee_attacks == 2
    );
    assert_eq!(
        deep[0].properties.equipment_bonuses.melee_skill, 0,
        "shallow candidates must not leak into deep rolls"
    );
}

#[test]
fn rolled_affix_save_round_trip_does_not_redraw_rng() {
    let mut game = Game::new(23);
    let before_roll = game.rng.draw_counter;
    let rolled = game.roll_affix_properties(&["demo.affix.adaptive-echo".to_owned()], 1);
    assert!(game.rng.draw_counter > before_roll);
    game.items.push(ItemInstance {
        id: "test.item.dynamic-save".to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: rolled.clone(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
    let saved = game.to_save();
    let saved_draws = saved.rng.draw_counter;
    let restored = Game::from_save(saved).expect("rolled affix payload should reload");
    let restored_item = restored
        .items
        .iter()
        .find(|item| item.id == "test.item.dynamic-save")
        .expect("dynamic item should survive reload");
    assert_eq!(restored_item.rolled_affixes, rolled);
    assert_eq!(restored.rng.draw_counter, saved_draws);

    let mut legacy = restored.to_save();
    legacy
        .inventory
        .iter_mut()
        .find(|item| item.id == "test.item.dynamic-save")
        .expect("dynamic inventory item")
        .rolled_affixes
        .clear();
    let migrated = Game::from_save(legacy).expect("missing rolled payload is a zero-RNG migration");
    assert_eq!(migrated.rng.draw_counter, saved_draws);
    assert!(
        migrated
            .items
            .iter()
            .find(|item| item.id == "test.item.dynamic-save")
            .expect("legacy dynamic item")
            .rolled_affixes
            .is_empty()
    );
}

#[test]
fn rolled_equipment_bonuses_and_regeneration_are_authoritative() {
    let mut game = Game::new(31);
    clear_monsters(&mut game);
    let properties = AffixPropertyBundleDefinition {
        equipment_bonuses: EquipmentBonuses {
            melee_attacks: 2,
            melee_skill: 11,
            digging_skill: 7,
            ..EquipmentBonuses::default()
        },
        ..AffixPropertyBundleDefinition::default()
    };
    game.items.push(ItemInstance {
        id: "test.item.dynamic-equipped".to_owned(),
        kind_id: "demo.item.adaptive-glaive".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.adaptive-echo".to_owned()],
        rolled_affixes: vec![RolledAffixState {
            affix_id: "demo.affix.adaptive-echo".to_owned(),
            properties,
        }],
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        },
    });
    let item_id = "test.item.dynamic-equipped".to_owned();
    game.item_property_knowledge.insert(
        item_id.clone(),
        ItemPropertyKnowledgeState {
            appraised: true,
            identified: true,
            known_affix_ids: BTreeSet::from(["demo.affix.adaptive-echo".to_owned()]),
        },
    );
    let stats = game.player_derived_stats();
    assert_eq!(stats.melee_attacks.value, 3);
    assert!(stats.melee_skill.value >= 11);
    assert!(stats.dig_skill.value >= 7);
    let equipped = game
        .equipment_dto()
        .into_iter()
        .find(|item| item.id == item_id)
        .expect("dynamic item should be visible");
    assert_eq!(equipped.equipment_bonuses.melee_attacks, 2);
    assert_eq!(equipped.passives, vec![EquipmentPassiveDto::Regeneration]);

    game.player.hp = game.effective_player_max_hp() - 2;
    game.world_tick = EQUIPMENT_REGENERATION_INTERVAL_TICKS - 1;
    let before = game.player.hp;
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.player.hp, before + 1);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "equipment.regenerated")
    );
}

#[test]
fn death_fourth_book_materializes_original_level_curves() {
    let projected = |level| {
        let mut game =
            Game::new_with_build(0, "demo.build.scholar").expect("scholar build should create");
        game.progress.level = level;
        game.snapshot()
            .player
            .abilities
            .into_iter()
            .map(|ability| (ability.id.clone(), ability))
            .collect::<BTreeMap<_, _>>()
    };

    let level_40 = projected(40);
    assert!(matches!(
        level_40["demo.ability.death-death-ray"].effects.as_slice(),
        [AbilityEffectSpecDto::DeathRay { power: 80 }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-raise-dead"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::SummonCategory {
            maximum_level: 60,
            upgraded_category: Some(category),
            upgrade_at_level: Some(48),
            ..
        }] if category == "high-undead"
    ));
    let [
        AbilityEffectSpecDto::IdentifyItem {
            full_identify_power,
            full_identify_roll_sides,
        },
    ] = level_40["demo.ability.death-esoteria"].effects.as_slice()
    else {
        panic!("Esoteria should project one identify effect");
    };
    assert_eq!((*full_identify_power, *full_identify_roll_sides), (30, 50));
    assert!(matches!(
        level_40["demo.ability.death-vampiric-transformation"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 25,
            duration_sides: 25,
            granted_race_id: Some(race_id),
            ..
        }] if race_id == "demo.race.vampire-lord"
    ));
    assert!(matches!(
        level_40["demo.ability.death-mass-genocide"]
            .effects
            .as_slice(),
        [AbilityEffectSpecDto::Genocide {
            scope: AbilityGenocideScopeDto::Nearby,
            power: 92,
            radius: 20,
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-hellfire"].effects.as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 373,
            radius: 5,
            ..
        }]
    ));
    assert!(matches!(
        level_40["demo.ability.death-wraithform"].effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 14,
            duration_sides: 14,
            grants_wall_passage: true,
            incoming_damage_percent: 50,
            ..
        }]
    ));

    let level_50 = projected(50);
    assert!(matches!(
        level_50["demo.ability.death-hellfire"].effects.as_slice(),
        [AbilityEffectSpecDto::AreaDamage {
            damage_bonus: 604,
            radius: 10,
            ..
        }]
    ));
    assert!(matches!(
        level_50["demo.ability.death-wraithform"].effects.as_slice(),
        [AbilityEffectSpecDto::ApplyStatus {
            duration_ticks: 25,
            duration_sides: 25,
            ..
        }]
    ));
}

#[test]
fn death_ray_enforces_living_unique_and_level_gates() {
    let resolve = |seed: u64, kind_id: &str| {
        let mut game = Game::new(seed);
        clear_monsters(&mut game);
        game.progress.level = 50;
        game.rng = RfbRng::seeded(seed);
        let definition = game.content.actor(kind_id).expect("demo target").clone();
        game.entities.push(actor_from_runtime_spawn(
            "test.actor.death-ray-target",
            kind_id,
            Position { x: 4, y: 3 },
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
        let mut events = Vec::new();
        let mut removed = Vec::new();
        game.resolve_ability_death_ray(
            "demo.ability.death-death-ray",
            vec![Position { x: 4, y: 3 }],
            100,
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        )
        .expect("Death Ray should resolve");
        let resolution = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilityEffectsResolved { resolution, .. } => {
                    resolution.effects.first().cloned()
                }
                _ => None,
            })
            .expect("Death Ray should emit a resolution");
        (game, resolution, removed)
    };

    let (nonliving, resolution, removed) = resolve(0, "demo.actor.resonant-warden");
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::DeathRay {
            living: false,
            resisted: true,
            unique_roll: None,
            target_level_roll: None,
            caster_level_roll: None,
            ..
        }
    ));
    assert!(removed.is_empty());
    assert_eq!(nonliving.rng.draw_counter, 0);

    let (_, resolution, removed) = resolve(0, "demo.actor.serpent-of-chaos");
    assert!(matches!(
        resolution,
        AbilityEffectResolutionDto::DeathRay {
            living: true,
            unique: true,
            resisted: true,
            unique_roll: Some(roll),
            ..
        } if roll != 666
    ));
    assert!(removed.is_empty());

    let mut saw_resist = false;
    let mut saw_kill = false;
    for seed in 0..256 {
        let (_, resolution, removed) = resolve(seed, "demo.actor.gloom-weaver");
        let AbilityEffectResolutionDto::DeathRay {
            target_level,
            target_level_roll: Some(target_roll),
            caster_level_roll: Some(caster_roll),
            resisted,
            ..
        } = resolution
        else {
            panic!("living Death Ray should roll its level contest");
        };
        assert_eq!(
            resisted,
            target_level + u32::from(target_roll) > caster_roll
        );
        saw_resist |= resisted && removed.is_empty();
        saw_kill |= !resisted && removed == ["test.actor.death-ray-target"];
        if saw_resist && saw_kill {
            break;
        }
    }
    assert!(saw_resist && saw_kill);
}

#[test]
fn raise_dead_is_deterministic_and_enforces_faction_group_and_unique_rules() {
    let cast = |seed: u64, level: u16| {
        let mut game = prepare_death_caster(seed, level, "demo.ability.death-raise-dead");
        game.debug_set_ability_casts_succeed(true);
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.death-raise-dead",
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Raise Dead should resolve");
        let resolution = events
            .iter()
            .find_map(|event| match event {
                DomainEvent::AbilitySummoned { resolution, .. } => Some(resolution.clone()),
                _ => None,
            })
            .expect("Raise Dead should summon");
        (game, resolution)
    };

    let (shallow, shallow_resolution) = cast(0, 25);
    assert_eq!(shallow_resolution.actor_kind_id, "undead");
    assert!(
        shallow_resolution
            .summoned_kind_ids
            .iter()
            .all(|kind_id| kind_id == "demo.actor.risen-thrall")
    );
    assert_eq!(shallow.state_hash(), cast(0, 25).0.state_hash());

    let mut saw_friendly = false;
    let mut saw_hostile = false;
    let mut saw_group = false;
    let mut saw_unique = false;
    for seed in 0..512 {
        let (game, resolution) = cast(seed, 48);
        assert_eq!(resolution.actor_kind_id, "high-undead");
        assert!(resolution.summoned_kind_ids.iter().all(|kind_id| matches!(
            kind_id.as_str(),
            "demo.actor.grave-wight" | "demo.actor.dread-vampire"
        )));
        let summoned = game
            .entities
            .iter()
            .filter(|entity| resolution.entity_ids.contains(&entity.id))
            .collect::<Vec<_>>();
        if resolution.hostile {
            saw_hostile = true;
            assert!(summoned.iter().all(|entity| entity.controller_id.is_none()));
        } else {
            saw_friendly = true;
            assert!(
                summoned
                    .iter()
                    .all(|entity| entity.controller_id.as_deref() == Some(game.player.id.as_str()))
            );
            assert!(
                resolution
                    .summoned_kind_ids
                    .iter()
                    .all(|kind_id| kind_id != "demo.actor.dread-vampire")
            );
        }
        saw_group |= resolution.group && resolution.entity_ids.len() > 1;
        if resolution
            .summoned_kind_ids
            .iter()
            .any(|kind_id| kind_id == "demo.actor.dread-vampire")
        {
            assert!(resolution.hostile);
            saw_unique = true;
        }
        if saw_friendly && saw_hostile && saw_group && saw_unique {
            break;
        }
    }
    assert!(saw_friendly && saw_hostile && saw_group && saw_unique);
}

#[test]
fn esoteria_validates_item_targets_before_cost_and_persists_knowledge() {
    let item = || ItemInstance {
        id: "test.item.esoteria".to_owned(),
        kind_id: "demo.item.echo-blade".to_owned(),
        quantity: 1,
        quality: ItemQualityDto::Fine,
        affix_ids: vec!["demo.affix.vampiric".to_owned()],
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation: None,
        charges: None,
        device_recovery_progress: 0,
        location: ItemLocation::Ground(Position { x: 10, y: 10 }),
    };
    let mut invalid = prepare_death_caster(0, 30, "demo.ability.death-esoteria");
    invalid.items.push(item());
    invalid.debug_set_ability_casts_succeed(true);
    let mana_before = invalid.resources["demo.resource.mana"].current;
    let draws_before = invalid.rng.draw_counter;
    let mut events = Vec::new();
    invalid
        .resolve_player_ability(
            "demo.ability.death-esoteria",
            TargetSelection::Item {
                item_id: "test.item.esoteria".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("invalid Esoteria target should resolve as unavailable");
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::AbilityTargetUnavailable { .. }]
    ));
    assert_eq!(invalid.resources["demo.resource.mana"].current, mana_before);
    assert_eq!(invalid.rng.draw_counter, draws_before);

    let mut ordinary = None;
    let mut full = None;
    for seed in 0..128 {
        let mut game = prepare_death_caster(seed, 30, "demo.ability.death-esoteria");
        let mut target = item();
        target.location = ItemLocation::Inventory;
        game.items.push(target);
        game.debug_set_ability_casts_succeed(true);
        let mut events = Vec::new();
        game.resolve_player_ability(
            "demo.ability.death-esoteria",
            TargetSelection::Item {
                item_id: "test.item.esoteria".to_owned(),
            },
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("Esoteria should resolve");
        let is_full = events.iter().any(|event| {
            matches!(
                event,
                DomainEvent::AbilityEffectsResolved { resolution, .. }
                    if matches!(
                        resolution.effects.as_slice(),
                        [AbilityEffectResolutionDto::IdentifyItem { full: true, .. }]
                    )
            )
        });
        if is_full {
            full.get_or_insert(game);
        } else {
            ordinary.get_or_insert(game);
        }
        if ordinary.is_some() && full.is_some() {
            break;
        }
    }
    let ordinary = ordinary.expect("an ordinary identification seed should exist");
    let ordinary_knowledge = &ordinary.item_property_knowledge["test.item.esoteria"];
    assert!(ordinary_knowledge.appraised);
    assert!(!ordinary_knowledge.identified);
    assert!(ordinary_knowledge.known_affix_ids.is_empty());

    let mut full = full.expect("a full identification seed should exist");
    let full_knowledge = &full.item_property_knowledge["test.item.esoteria"];
    assert!(full_knowledge.appraised && full_knowledge.identified);
    assert!(
        full_knowledge
            .known_affix_ids
            .contains("demo.affix.vampiric")
    );
    full.items
        .iter_mut()
        .find(|item| item.id == "test.item.esoteria")
        .expect("identified item should remain")
        .location = ItemLocation::Ground(Position { x: 10, y: 10 });
    full.refresh_character_skills();
    full.refresh_player_resource_maxima();
    let restored = Game::from_save(full.to_save()).expect("item knowledge should reload");
    assert_eq!(restored.state_hash(), full.state_hash());
    assert!(restored.item_property_knowledge["test.item.esoteria"].identified);
}

#[test]
fn vampiric_transformation_overlays_race_but_preserves_body_slots() {
    let mut game = prepare_death_caster(17, 35, "demo.ability.death-vampiric-transformation");
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    let body_slots = game.body_slots.clone();
    let base = game.snapshot().player;
    let mut ability = game
        .content
        .ability("demo.ability.death-vampiric-transformation")
        .expect("Vampiric Transformation should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 35);
    game.resolve_ability_actor_effects(
        &ability.id,
        &ability.effect,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Vampiric Transformation should resolve");

    let transformed = game.snapshot().player;
    assert_eq!(game.body_slots, body_slots);
    assert_eq!(
        transformed
            .build
            .as_ref()
            .map(|build| build.race_id.as_str()),
        Some("demo.race.vampire-lord")
    );
    assert!(
        transformed.progress.attributes.strength.effective
            > base.progress.attributes.strength.effective
    );
    assert!(
        transformed
            .resistances
            .iter()
            .any(|entry| entry.damage_type == DamageTypeDto::Dark
                && entry.level == ResistanceLevelDto::Immune)
    );
    let transformed_melee = transformed
        .progress
        .skills
        .iter()
        .find(|skill| skill.id == "demo.skill.melee")
        .expect("transformed melee skill should be projected");
    let base_melee = base
        .progress
        .skills
        .iter()
        .find(|skill| skill.id == "demo.skill.melee")
        .expect("base melee skill should be projected");
    assert!(transformed_melee.base > base_melee.base);
    let restored = Game::from_save(game.to_save()).expect("temporary race should reload");
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.body_slots, body_slots);
}

#[test]
fn restore_life_uses_historical_experience_and_migrates_old_saves() {
    let mut game = prepare_death_caster(0, 42, "demo.ability.death-restore-life");
    game.progress.experience = 500;
    game.progress.maximum_experience = 900;
    game.progress.life_force = 125;
    game.debug_set_ability_casts_succeed(true);
    let mut events = Vec::new();
    game.resolve_player_ability(
        "demo.ability.death-restore-life",
        TargetSelection::SelfTarget,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Restore Life should resolve");
    assert_eq!(game.progress.experience, 900);
    assert_eq!(game.progress.maximum_experience, 900);
    assert_eq!(game.progress.life_force, 1_000);
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::RestoreVitality {
                    experience_before: 500,
                    experience_after: 900,
                    life_force_before: 125,
                    life_force_after: 1_000,
                    ..
                }]
            )
    )));

    let mut legacy = Game::new(0);
    legacy.apply_player_experience(10, &mut Vec::new());
    let expected = legacy.progress.experience;
    let mut payload = legacy.to_save();
    payload
        .player
        .progress
        .as_mut()
        .expect("player progress should be saved")
        .maximum_experience = 0;
    let migrated = Game::from_save(payload).expect("old progress should migrate");
    assert_eq!(migrated.progress.maximum_experience, expected);
}

#[test]
fn nearby_genocide_filters_radius_resists_unique_and_is_deterministic() {
    let prepare = || {
        let mut game = Game::new(0);
        clear_monsters(&mut game);
        for (id, kind_id, position) in [
            (
                "test.actor.nearby",
                "demo.actor.ember-mote",
                Position { x: 4, y: 3 },
            ),
            (
                "test.actor.unique",
                "demo.actor.serpent-of-chaos",
                Position { x: 5, y: 3 },
            ),
            (
                "test.actor.distant",
                "demo.actor.ember-mote",
                Position { x: 19, y: 19 },
            ),
        ] {
            let definition = game.content.actor(kind_id).expect("demo target").clone();
            game.entities.push(actor_from_runtime_spawn(
                id,
                kind_id,
                position,
                definition.max_hp,
                definition.speed,
                100,
                true,
            ));
        }
        game
    };
    let mut left = prepare();
    let mut right = left.clone();
    let resolve = |game: &mut Game| {
        let mut events = Vec::new();
        let mut removed = Vec::new();
        game.resolve_ability_genocide(
            "demo.ability.death-mass-genocide",
            None,
            AbilityGenocideScopeDefinition::Nearby,
            1_000,
            2,
            &mut events,
            &mut BTreeSet::new(),
            &mut removed,
        );
        (events, removed)
    };
    let (events, removed) = resolve(&mut left);
    let (_, right_removed) = resolve(&mut right);
    assert_eq!(left.state_hash(), right.state_hash());
    assert_eq!(removed, right_removed);
    assert_eq!(removed, ["test.actor.nearby"]);
    assert!(
        left.entities
            .iter()
            .any(|entity| entity.id == "test.actor.unique")
    );
    assert!(
        left.entities
            .iter()
            .any(|entity| entity.id == "test.actor.distant")
    );
    assert!(events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(
                resolution.effects.as_slice(),
                [AbilityEffectResolutionDto::Genocide {
                    radius: 2,
                    removed_entity_ids,
                    resisted_entity_ids,
                    ..
                }] if removed_entity_ids == &["test.actor.nearby".to_owned()]
                    && resisted_entity_ids == &["test.actor.unique".to_owned()]
            )
    )));
}

#[test]
fn wraithform_passes_walls_halves_spell_damage_and_expires_in_place() {
    let mut game = prepare_death_caster(31, 47, "demo.ability.death-wraithform");
    game.refresh_character_skills();
    game.refresh_player_resource_maxima();
    let wall = Position { x: 4, y: 3 };
    game.items.retain(
        |item| !matches!(item.location, ItemLocation::Ground(position) if position == wall),
    );
    replace_terrain(&mut game, wall, "demo.terrain.wall");
    let mut ability = game
        .content
        .ability("demo.ability.death-wraithform")
        .expect("Wraithform should exist")
        .clone();
    Game::apply_player_level_scaling(&mut ability, 47);
    game.resolve_ability_actor_effects(
        &ability.id,
        &ability.effect,
        AbilityTargetPlan::SelfTarget,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Wraithform should resolve");
    assert!(game.player_can_pass_walls());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.player.position, wall);
    assert_eq!(game.terrain_at(wall), "demo.terrain.wall");

    let hp_before = game.player.hp;
    let damage = game.resolve_monster_damage_to_player(
        "test.actor.caster",
        "demo.actor.gloom-weaver",
        "demo.ability.resonant-bolt",
        0,
        9,
        9,
        DamageType::Fire,
        &mut Vec::new(),
    );
    assert!(matches!(
        damage,
        AbilityEffectResolutionDto::Damage {
            resolution: DamageResolutionDto {
                final_damage: 5,
                ..
            },
            ..
        }
    ));
    assert_eq!(game.player.hp, hp_before - 5);

    let mut restored = Game::from_save(game.to_save()).expect("wall-bound Wraithform should load");
    assert_eq!(restored.snapshot(), game.snapshot());
    restored
        .player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == "rfb.status.wraithform")
        .expect("Wraithform should remain active")
        .remaining_ticks = 1;
    restored
        .process_status_tick(&mut Vec::new(), &mut BTreeSet::new(), &mut Vec::new())
        .expect("Wraithform expiry should resolve");
    assert!(!restored.player_can_pass_walls());
    assert_eq!(restored.player.position, wall);
    assert_eq!(restored.terrain_at(wall), "demo.terrain.wall");
}

#[test]
fn dynamic_device_recovery_is_inventory_only_deterministic_and_rod_fast() {
    let mut game =
        Game::new_with_build(11, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.recovery.rod", "demo.item.resonance-rod", 1)
        .expect("rod should generate");
    game.debug_add_generated_inventory_item(
        "test.item.recovery.wand",
        "demo.item.resonance-wand",
        1,
    )
    .expect("wand should generate");
    for item_id in ["test.item.recovery.rod", "test.item.recovery.wand"] {
        let item = game
            .items
            .iter_mut()
            .find(|item| item.id == item_id)
            .expect("generated device");
        item.charges = Some(ItemChargesDto {
            current: 0,
            maximum: 20,
        });
        item.device_recovery_progress = 0;
    }

    for world_tick in 1..=50 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.rod")
            .and_then(|item| item.charges)
            .expect("rod charges")
            .current,
        10
    );
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.wand")
            .and_then(|item| item.charges)
            .expect("wand charges")
            .current,
        1
    );

    let wand = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.recovery.wand")
        .expect("wand");
    wand.location = ItemLocation::Ground(Position { x: 0, y: 0 });
    for world_tick in 51..=100 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.recovery.wand")
            .and_then(|item| item.charges)
            .expect("wand charges")
            .current,
        1
    );
}

#[test]
fn device_recovery_remainder_round_trips_and_old_saves_default_to_zero() {
    let mut game =
        Game::new_with_build(12, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.remainder", "demo.item.resonance-rod", 1)
        .expect("rod should generate");
    let item = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.remainder")
        .expect("rod");
    item.charges = Some(ItemChargesDto {
        current: 0,
        maximum: 20,
    });
    for world_tick in 1..=3 {
        game.world_tick = world_tick;
        game.process_inventory_device_recovery(&mut Vec::new());
    }
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.item.remainder")
            .expect("rod")
            .device_recovery_progress,
        600
    );

    let save = game.to_save();
    let restored = Game::from_save(save.clone()).expect("recovery remainder should reload");
    assert_eq!(restored.snapshot().state_hash, game.snapshot().state_hash);
    let mut old_json = serde_json::to_value(save).expect("save should serialize");
    let inventory = old_json["inventory"]
        .as_array_mut()
        .expect("save inventory should be an array");
    for item in inventory {
        item.as_object_mut()
            .expect("inventory item should be an object")
            .remove("deviceRecoveryProgress");
    }
    let old_save = serde_json::from_value(old_json).expect("legacy save should deserialize");
    let migrated = Game::from_save(old_save).expect("missing recovery remainder should migrate");
    assert_eq!(
        migrated
            .items
            .iter()
            .find(|item| item.id == "test.item.remainder")
            .expect("rod")
            .device_recovery_progress,
        0
    );

    let mut invalid = game.to_save();
    invalid
        .inventory
        .iter_mut()
        .find(|item| item.id == "test.item.remainder")
        .expect("saved rod")
        .device_recovery_progress = 1_000;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("item charge state is invalid"))
    ));
}

#[test]
fn recharge_invalid_transactions_are_zero_time_and_zero_rng() {
    let mut game =
        Game::new_with_build(13, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut game);
    game.debug_add_generated_inventory_item("test.item.full", "demo.item.resonance-staff", 1)
        .expect("staff should generate");
    let target = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.full")
        .expect("staff");
    target.charges = Some(ItemChargesDto {
        current: 24,
        maximum: 24,
    });
    let world_tick = game.world_tick;
    let draws = game.rng.draw_counter;
    let update = dispatch_next(
        &mut game,
        GameCommand::RechargeItem {
            target_item_id: "test.item.full".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    assert_eq!(update.world_tick, world_tick);
    assert_eq!(game.rng.draw_counter, draws);
    assert_eq!(update.events[0].kind, "device.recharge-unavailable");
    assert_eq!(
        update.events[0].args.get("reason").map(String::as_str),
        Some("target-not-rechargeable")
    );
}

#[test]
fn resource_recharge_succeeds_and_failure_clears_target_energy() {
    let mut success =
        Game::new_with_build(14, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut success);
    success
        .debug_add_generated_inventory_item(
            "test.item.resource-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    success
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.resource-target")
        .expect("staff")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 24,
    });
    success.debug_set_recharge_attempts_succeed(true);
    let resource_before = success.resources["demo.resource.resonance"].current;
    let update = dispatch_next(
        &mut success,
        GameCommand::RechargeItem {
            target_item_id: "test.item.resource-target".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    let attempted = resource_before.min(24);
    assert_eq!(update.events[0].kind, "device.recharge-success");
    assert_eq!(
        success
            .items
            .iter()
            .find(|item| item.id == "test.item.resource-target")
            .and_then(|item| item.charges)
            .expect("staff charges")
            .current,
        attempted
    );
    assert_eq!(
        success.resources["demo.resource.resonance"].current,
        resource_before - attempted
    );

    let mut failure =
        Game::new_with_build(15, "demo.build.tinkerer").expect("tinkerer build should create");
    clear_monsters(&mut failure);
    failure
        .debug_add_generated_inventory_item(
            "test.item.failed-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    failure
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.failed-target")
        .expect("staff")
        .charges = Some(ItemChargesDto {
        current: 5,
        maximum: 24,
    });
    let failure_seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(5) == 0
        })
        .expect("one seed should fail recharge");
    failure.rng = RfbRng::seeded(failure_seed);
    let update = dispatch_next(
        &mut failure,
        GameCommand::RechargeItem {
            target_item_id: "test.item.failed-target".to_owned(),
            source: DeviceRechargeSourceDto::Resource,
        },
    );
    assert_eq!(update.events[0].kind, "device.recharge-failure");
    assert_eq!(
        failure
            .items
            .iter()
            .find(|item| item.id == "test.item.failed-target")
            .and_then(|item| item.charges)
            .expect("staff charges")
            .current,
        0
    );
}

#[test]
fn device_source_recharge_can_survive_be_destroyed_or_protect_artifacts() {
    let prepare = |seed| {
        let mut game = Game::new_with_build(seed, "demo.build.tinkerer")
            .expect("tinkerer build should create");
        clear_monsters(&mut game);
        game.debug_add_generated_inventory_item(
            "test.item.device-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
        game.debug_add_generated_inventory_item(
            "test.item.device-source",
            "demo.item.resonance-wand",
            1,
        )
        .expect("wand should generate");
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.device-target")
            .expect("target")
            .charges = Some(ItemChargesDto {
            current: 0,
            maximum: 24,
        });
        game.items
            .iter_mut()
            .find(|item| item.id == "test.item.device-source")
            .expect("source")
            .charges = Some(ItemChargesDto {
            current: 5,
            maximum: 24,
        });
        game.debug_set_recharge_attempts_succeed(true);
        game
    };

    let mut surviving = prepare(16);
    surviving.debug_set_recharge_sources_survive(true);
    let update = dispatch_next(
        &mut surviving,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(update.events[0].kind, "device.recharge-success");
    assert_eq!(
        surviving
            .items
            .iter()
            .find(|item| item.id == "test.item.device-target")
            .and_then(|item| item.charges)
            .expect("target charges")
            .current,
        5
    );
    assert!(
        surviving
            .items
            .iter()
            .any(|item| item.id == "test.item.device-source")
    );

    let destruction_seed = (0..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(3) == 0
        })
        .expect("one seed should destroy a source");
    let mut destroyed = prepare(17);
    destroyed.rng = RfbRng::seeded(destruction_seed);
    let update = dispatch_next(
        &mut destroyed,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(
        update.events[0]
            .args
            .get("sourceDestroyed")
            .map(String::as_str),
        Some("true")
    );
    assert!(
        destroyed
            .items
            .iter()
            .all(|item| item.id != "test.item.device-source")
    );

    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact_content =
        rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    artifact_content
        .content
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("wand definition")
        .tags
        .push("artifact".to_owned());
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact_content.content)
            .expect("artifact source content should remain valid"),
    ));
    let mut artifact =
        Game::from_content_with_build(18, catalog, BUILT_IN_WORLD_ID, "demo.build.tinkerer")
            .expect("custom tinkerer build should create");
    clear_monsters(&mut artifact);
    artifact
        .debug_add_generated_inventory_item(
            "test.item.device-target",
            "demo.item.resonance-staff",
            1,
        )
        .expect("staff should generate");
    artifact
        .debug_add_generated_inventory_item(
            "test.item.device-source",
            "demo.item.resonance-wand",
            1,
        )
        .expect("wand should generate");
    artifact
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.device-target")
        .expect("target")
        .charges = Some(ItemChargesDto {
        current: 0,
        maximum: 24,
    });
    artifact
        .items
        .iter_mut()
        .find(|item| item.id == "test.item.device-source")
        .expect("source")
        .charges = Some(ItemChargesDto {
        current: 5,
        maximum: 24,
    });
    artifact.debug_set_recharge_attempts_succeed(true);
    artifact.rng = RfbRng::seeded(destruction_seed);
    let update = dispatch_next(
        &mut artifact,
        GameCommand::RechargeItem {
            target_item_id: "test.item.device-target".to_owned(),
            source: DeviceRechargeSourceDto::Item {
                item_id: "test.item.device-source".to_owned(),
            },
        },
    );
    assert_eq!(
        update.events[0]
            .args
            .get("sourceDestroyed")
            .map(String::as_str),
        Some("false")
    );
    assert!(
        artifact
            .items
            .iter()
            .any(|item| item.id == "test.item.device-source")
    );
}

fn clear_monsters(game: &mut Game) {
    game.entities.clear();
    game.dungeon_states
        .get_mut("demo.dungeon.resonance-descent")
        .expect("resonance dungeon state should exist")
        .entrance_guardian_defeated = true;
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
}

#[test]
fn item_summon_candidates_follow_depth_player_level_kin_and_unique_rules() {
    let mut human = skill_check_game(67, "demo.build.vanguard");
    human.current_floor_id = "demo.floor.resonance-depth-10".to_owned();
    let general_effect = human
        .content
        .item("demo.item.summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("summoning scroll should have a use action")
        .effect
        .clone();
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = human.item_category_summon_plan(&general_effect)
    else {
        panic!("general summoning scroll should produce a summon plan");
    };
    assert_eq!(category, "any-monster");
    assert!(candidate_kind_ids.contains(&"demo.actor.risen-thrall".to_owned()));
    assert!(!candidate_kind_ids.contains(&"demo.actor.grave-wight".to_owned()));
    assert!(candidate_kind_ids.iter().all(|kind_id| {
        let definition = human.content.actor(kind_id).expect("candidate actor");
        definition.level <= 10 && !definition.tags.iter().any(|tag| tag == "guardian")
    }));
    assert_eq!(
        human.summon_category_candidate_kind_ids("undead", None, 8, true),
        ["demo.actor.risen-thrall"]
    );
    assert_eq!(
        human.summon_category_candidate_kind_ids("undead", None, 32, true),
        [
            "demo.actor.grave-wight".to_owned(),
            "demo.actor.risen-thrall".to_owned(),
        ]
    );

    let kin_effect = human
        .content
        .item("demo.item.kin-summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("kin summoning scroll should have a use action")
        .effect
        .clone();
    human.progress.level = 3;
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = human.item_category_summon_plan(&kin_effect)
    else {
        panic!("kin summoning scroll should produce a summon plan");
    };
    assert_eq!(category, "kin-glyph-112");
    assert_eq!(
        candidate_kind_ids,
        [
            "demo.actor.cinder-adept".to_owned(),
            "demo.actor.mote-binder".to_owned(),
        ]
    );
    human.progress.level = 4;
    let ItemUsePlan::SummonCategory {
        candidate_kind_ids, ..
    } = human.item_category_summon_plan(&kin_effect)
    else {
        unreachable!();
    };
    assert!(candidate_kind_ids.contains(&"demo.actor.hex-chanter".to_owned()));

    let mut gnome = skill_check_game(67, "demo.build.tinkerer");
    let gnome_effect = gnome
        .content
        .item("demo.item.kin-summoning-scroll")
        .and_then(|definition| definition.use_action.as_ref())
        .expect("kin summoning scroll should have a use action")
        .effect
        .clone();
    gnome.progress.level = 1;
    let ItemUsePlan::SummonCategory {
        category,
        candidate_kind_ids,
        ..
    } = gnome.item_category_summon_plan(&gnome_effect)
    else {
        unreachable!();
    };
    assert_eq!(category, "kin-glyph-104");
    assert_eq!(candidate_kind_ids, ["demo.actor.echo-hound"]);

    let high_undead = human.summon_category_candidate_kind_ids("high-undead", None, 48, true);
    assert!(high_undead.contains(&"demo.actor.dread-vampire".to_owned()));
    assert!(
        !human
            .summon_category_candidate_kind_ids("high-undead", None, 48, false)
            .contains(&"demo.actor.dread-vampire".to_owned())
    );
    let vampire = human
        .content
        .actor("demo.actor.dread-vampire")
        .expect("demo unique")
        .clone();
    human.entities.push(actor_from_runtime_spawn(
        "test.actor.existing-dread-vampire",
        &vampire.id,
        Position { x: 4, y: 3 },
        vampire.max_hp,
        vampire.speed,
        100,
        true,
    ));
    assert!(
        !human
            .summon_category_candidate_kind_ids("high-undead", None, 48, true)
            .contains(&"demo.actor.dread-vampire".to_owned())
    );
}

#[test]
fn friendly_item_summons_are_permanent_controlled_and_round_trip() {
    let mut game = skill_check_game(68, "demo.build.vanguard");
    give_inventory_item(
        &mut game,
        "test.item.pet-summoning-scroll.1",
        "demo.item.pet-summoning-scroll",
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.pet-summoning-scroll.1".to_owned(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    let resolution = update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::ItemSummon { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("pet scroll should emit a summon resolution");
    assert!(!resolution.entity_ids.is_empty());
    assert!(!resolution.hostile);
    assert_eq!(resolution.duration_turns, 0);
    let summoned_ids = resolution.entity_ids.clone();
    assert!(summoned_ids.iter().all(|entity_id| {
        game.entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(game.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
    assert_eq!(
        game.item_knowledge_dto("demo.item.pet-summoning-scroll"),
        ItemKnowledgeDto::Aware
    );
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "test.item.pet-summoning-scroll.1")
    );

    let saved = game.to_save();
    let restored = Game::from_save(saved).expect("controlled item summons should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(summoned_ids.iter().all(|entity_id| {
        restored
            .entities
            .iter()
            .find(|entity| entity.id == *entity_id)
            .is_some_and(|entity| {
                entity.controller_id.as_deref() == Some(restored.player.id.as_str())
                    && entity.summon.is_none()
            })
    }));
}

#[test]
fn item_summon_zero_candidate_and_zero_space_consume_without_awareness_or_rng() {
    let use_and_assert_zero = |game: &mut Game, item_id: &str, kind_id: &str| {
        give_inventory_item(game, item_id, kind_id);
        let draws_before = game.rng.draw_counter;
        let update = dispatch_next(
            game,
            GameCommand::UseItem {
                item_id: item_id.to_owned(),
                target: Some(TargetSelection::SelfTarget),
            },
        );
        let resolution = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(GameEventOutcomeDto::ItemSummon { resolution }) => Some(resolution),
                _ => None,
            })
            .expect("summon attempt should emit a resolution");
        assert!(resolution.entity_ids.is_empty());
        assert_eq!(game.rng.draw_counter, draws_before);
        assert_eq!(game.item_knowledge_dto(kind_id), ItemKnowledgeDto::Tried);
        assert!(!game.items.iter().any(|item| item.id == item_id));
    };

    let mut no_candidate = skill_check_game(69, "demo.build.vanguard");
    no_candidate.progress.level = 1;
    use_and_assert_zero(
        &mut no_candidate,
        "test.item.kin-summoning-scroll.1",
        "demo.item.kin-summoning-scroll",
    );

    let mut no_space = skill_check_game(70, "demo.build.vanguard");
    let positions = no_space.open_positions_around(no_space.player.position, 2);
    assert!(!positions.is_empty());
    for (ordinal, position) in positions.into_iter().enumerate() {
        no_space.items.push(ItemInstance {
            id: format!("test.item.summon-blocker.{ordinal}"),
            kind_id: "demo.item.luminous-shard".to_owned(),
            quantity: 1,
            quality: ItemQualityDto::Ordinary,
            affix_ids: Vec::new(),
            rolled_affixes: Vec::new(),
            enchantments: Default::default(),
            curse: None,
            activation: None,
            charges: None,
            device_recovery_progress: 0,
            location: ItemLocation::Ground(position),
        });
    }
    use_and_assert_zero(
        &mut no_space,
        "test.item.pet-summoning-scroll.1",
        "demo.item.pet-summoning-scroll",
    );
}

#[test]
fn dispel_undead_scroll_uses_the_visible_actor_snapshot_and_resist_all_gate() {
    const SCROLL_ID: &str = "test.item.dispel-undead-scroll.1";
    let mut game = skill_check_game(71, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.dispel-undead-scroll");
    let mut spawn = |id: &str, kind_id: &str, position: Position| {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    };
    spawn(
        "test.actor.dispel-target",
        "demo.actor.dread-vampire",
        Position { x: 4, y: 3 },
    );
    spawn(
        "test.actor.dispel-living",
        "demo.actor.echo-hound",
        Position { x: 4, y: 2 },
    );
    spawn(
        "test.actor.dispel-resist-all",
        "demo.actor.resonant-warden",
        Position { x: 2, y: 3 },
    );
    spawn(
        "test.actor.dispel-behind-wall",
        "demo.actor.dread-vampire",
        Position { x: 3, y: 5 },
    );
    replace_terrain(&mut game, Position { x: 3, y: 4 }, "demo.terrain.wall");
    game.rng = RfbRng::seeded(71);
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.use_inventory_item(
        SCROLL_ID,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Dispel Undead should resolve");

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.actor.dispel-target")
            .expect("visible undead should remain")
            .hp,
        80
    );
    for actor_id in [
        "test.actor.dispel-living",
        "test.actor.dispel-resist-all",
        "test.actor.dispel-behind-wall",
    ] {
        let actor = game
            .entities
            .iter()
            .find(|entity| entity.id == actor_id)
            .expect("unaffected actor should remain");
        assert_eq!(
            actor.hp,
            game.content
                .actor(&actor.kind_id)
                .expect("unaffected actor definition")
                .max_hp
        );
    }
    assert!(
        matches!(events.as_slice(), [DomainEvent::ItemDispelHit { target_kind_id, damage, .. }]
        if target_kind_id == "demo.actor.dread-vampire"
            && damage.applied == 80
            && damage.damage_type == DamageType::HolyFire)
    );
    assert_eq!(
        game.item_knowledge_dto("demo.item.dispel-undead-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn banishment_scroll_resolves_resistance_and_destinations_in_actor_order() {
    const SCROLL_ID: &str = "test.item.banishment-scroll.1";
    let mut game = skill_check_game(72, "demo.build.scholar");
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.banishment-scroll");
    for (id, kind_id, position) in [
        (
            "test.actor.banish-normal",
            "demo.actor.echo-hound",
            Position { x: 4, y: 3 },
        ),
        (
            "test.actor.banish-resistant-unique",
            "demo.actor.dread-vampire",
            Position { x: 3, y: 4 },
        ),
        (
            "test.actor.banish-guardian",
            "demo.actor.resonant-warden",
            Position { x: 2, y: 3 },
        ),
    ] {
        let definition = game.content.actor(kind_id).expect("demo actor").clone();
        game.entities.push(actor_from_runtime_spawn(
            id,
            kind_id,
            position,
            definition.max_hp,
            definition.speed,
            100,
            true,
        ));
    }
    game.rng = RfbRng::seeded(72);
    let original_positions = game
        .entities
        .iter()
        .map(|entity| (entity.id.clone(), entity.position))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    game.use_inventory_item(
        SCROLL_ID,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("Banishment should resolve");

    assert_eq!(game.rng_draw_counter(), 1);
    assert_ne!(
        game.entities
            .iter()
            .find(|entity| entity.id == "test.actor.banish-normal")
            .expect("banished actor should remain")
            .position,
        original_positions["test.actor.banish-normal"]
    );
    for actor_id in [
        "test.actor.banish-resistant-unique",
        "test.actor.banish-guardian",
    ] {
        assert_eq!(
            game.entities
                .iter()
                .find(|entity| entity.id == actor_id)
                .expect("resistant actor should remain")
                .position,
            original_positions[actor_id]
        );
    }
    let event_kinds = events
        .iter()
        .map(|event| match event {
            DomainEvent::ItemBanishedActor { resolution, .. } => {
                format!("banished:{}", resolution.actor_id)
            }
            DomainEvent::ItemBanishmentResisted { target_kind_id, .. } => {
                format!("resisted:{target_kind_id}")
            }
            _ => "unexpected".to_owned(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        event_kinds,
        [
            "banished:test.actor.banish-normal",
            "resisted:demo.actor.dread-vampire",
            "resisted:demo.actor.resonant-warden",
        ]
    );
    assert_eq!(
        game.item_knowledge_dto("demo.item.banishment-scroll"),
        ItemKnowledgeDto::Aware
    );
}

#[test]
fn visible_actor_scrolls_consume_empty_results_without_rng_or_awareness() {
    for (seed, item_id, kind_id) in [
        (
            73,
            "test.item.empty-dispel-undead-scroll.1",
            "demo.item.dispel-undead-scroll",
        ),
        (
            74,
            "test.item.empty-banishment-scroll.1",
            "demo.item.banishment-scroll",
        ),
    ] {
        let mut game = skill_check_game(seed, "demo.build.scholar");
        give_inventory_item(&mut game, item_id, kind_id);
        game.rng = RfbRng::seeded(seed);
        let mut events = Vec::new();
        game.use_inventory_item(
            item_id,
            None,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .expect("empty visible actor effect should resolve");
        assert_eq!(game.rng_draw_counter(), 0);
        assert!(!game.items.iter().any(|item| item.id == item_id));
        assert_eq!(game.item_knowledge_dto(kind_id), ItemKnowledgeDto::Tried);
        assert!(matches!(
            events.as_slice(),
            [DomainEvent::ItemDispelNoEffect { .. }] | [DomainEvent::ItemBanishmentNoEffect { .. }]
        ));
    }
}

#[test]
fn mass_genocide_scroll_consumes_empty_result_with_awareness_and_zero_rng() {
    const ITEM_ID: &str = "test.item.severance-scroll.1";
    const KIND_ID: &str = "demo.item.severance-scroll";
    let mut game = skill_check_game(75, "demo.build.scholar");
    clear_monsters(&mut game);
    give_inventory_item(&mut game, ITEM_ID, KIND_ID);
    let hp_before = game.player.hp;
    let draws_before = game.rng_draw_counter();
    let mut events = Vec::new();
    game.use_inventory_item(
        ITEM_ID,
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .expect("empty mass genocide should resolve");

    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(game.player.hp, hp_before);
    assert!(!game.items.iter().any(|item| item.id == ITEM_ID));
    assert_eq!(game.item_knowledge_dto(KIND_ID), ItemKnowledgeDto::Aware);
    assert!(matches!(
        events.as_slice(),
        [DomainEvent::ItemMassGenocide {
            removed_count: 0,
            resisted_count: 0,
            fatigue_damage: 0,
            ..
        }]
    ));
}

#[test]
fn travel_scroll_random_teleport_is_deterministic_and_rejects_without_space_atomically() {
    let prepare = || {
        let mut game = Game::new(64);
        clear_monsters(&mut game);
        give_inventory_item(
            &mut game,
            "test.item.flicker-scroll.1",
            "demo.item.flicker-scroll",
        );
        game
    };
    let mut first = prepare();
    let mut second = prepare();
    let first_update = dispatch_next(
        &mut first,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    let second_update = dispatch_next(
        &mut second,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(first.snapshot(), second.snapshot());
    assert_eq!(first_update.events, second_update.events);
    assert_ne!(first.player.position, Position { x: 3, y: 3 });
    assert!(first_update.events.iter().any(|event| {
        event.kind == "item.use-teleported"
            && matches!(
                event.outcome,
                Some(GameEventOutcomeDto::AbilityTeleport { .. })
            )
    }));

    let mut blocked = prepare();
    let player_index = blocked
        .index(blocked.player.position)
        .expect("player position should be in bounds");
    blocked.terrain.fill("demo.terrain.wall".to_owned());
    blocked.terrain[player_index] = "demo.terrain.floor".to_owned();
    let before = blocked.snapshot();
    let draw_counter = blocked.rng_draw_counter();
    let update = dispatch_next(
        &mut blocked,
        GameCommand::UseItem {
            item_id: "test.item.flicker-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(update.world_tick, before.world_tick);
    assert_eq!(blocked.rng_draw_counter(), draw_counter);
    assert!(
        blocked
            .items
            .iter()
            .any(|item| item.id == "test.item.flicker-scroll.1")
    );
    assert_eq!(update.events[0].kind, "item.use-unavailable");
}

#[test]
fn teleport_level_rolls_direction_then_uses_tree_targets_and_boundary_fallback() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    give_inventory_item(
        &mut game,
        "test.item.depthshift-scroll.1",
        "demo.item.depthshift-scroll",
    );
    let downward_seed = (0_u64..100)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(2) == 1
        })
        .expect("one seed should select downward travel");
    game.rng = RfbRng::seeded(downward_seed);
    let update = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.depthshift-scroll.1".to_owned(),
            target: None,
        },
    );
    assert!(game.floor_depth(&game.current_floor_id) > 1);
    assert!(update.events.iter().any(|event| {
        event.kind == "item.use-teleported-level"
            && event.args.get("to") == Some(&game.current_floor_id)
    }));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "floor.transition")
    );

    game.transition_floor("demo.floor.echo-depth-3".to_owned(), None, None, false)
        .expect("final floor transition should resolve")
        .expect("final floor should be available");
    game.entities.clear();
    game.dungeon_states
        .get_mut("demo.dungeon.echo-depths")
        .expect("echo dungeon state")
        .guardian_defeated = true;
    give_inventory_item(
        &mut game,
        "test.item.depthshift-scroll.2",
        "demo.item.depthshift-scroll",
    );
    let before_depth = game.floor_depth(&game.current_floor_id);
    game.rng = RfbRng::seeded(downward_seed);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.depthshift-scroll.2".to_owned(),
            target: None,
        },
    );
    assert!(game.floor_depth(&game.current_floor_id) < before_depth);
}

#[test]
fn recall_round_trip_clears_the_old_instance_and_creates_a_new_one() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    let first_instance = game
        .current_dungeon_instance_id
        .clone()
        .expect("dungeon should have an instance");
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-1")
    );
    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.1",
        "demo.item.homeward-scroll",
    );
    game.debug_set_recall_delay_turns(Some(1));
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        Some(1)
    );
    let restored = Game::from_save(game.to_save()).expect("pending recall should round trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    let update = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "item.recall-triggered")
    );
    assert!(
        game.stored_floors
            .values()
            .all(|floor| { floor.dungeon_instance_id.as_deref() != Some(first_instance.as_str()) })
    );

    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.2",
        "demo.item.homeward-scroll",
    );
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.2".to_owned(),
            target: None,
        },
    );
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.echo-depth-1");
    assert_ne!(
        game.current_dungeon_instance_id.as_ref(),
        Some(&first_instance)
    );
}

#[test]
fn recall_can_be_cancelled_and_reset_to_a_shallower_branch_floor() {
    let mut game = Game::new(11);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    clear_monsters(&mut game);
    game.transition_floor("demo.floor.echo-depth-2".to_owned(), None, None, false)
        .expect("deeper transition should resolve")
        .expect("deeper floor should be available");
    clear_monsters(&mut game);
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-2")
    );
    game.transition_floor("demo.floor.echo-depth-1".to_owned(), None, None, false)
        .expect("shallower transition should resolve")
        .expect("shallower floor should be available");
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-2")
    );

    give_inventory_item(
        &mut game,
        "test.item.recall-setting-scroll.1",
        "demo.item.recall-setting-scroll",
    );
    let reset = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.recall-setting-scroll.1".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall.as_ref().map(|recall| recall.floor_id.as_str()),
        Some("demo.floor.echo-depth-1")
    );
    assert!(
        reset
            .events
            .iter()
            .any(|event| event.kind == "item.recall-reset")
    );
    let restored = Game::from_save(game.to_save()).expect("reset destination should round trip");
    assert_eq!(restored.recall, game.recall);

    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.3",
        "demo.item.homeward-scroll",
    );
    give_inventory_item(
        &mut game,
        "test.item.homeward-scroll.4",
        "demo.item.homeward-scroll",
    );
    game.debug_set_recall_delay_turns(Some(3));
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.3".to_owned(),
            target: None,
        },
    );
    assert!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns)
            .is_some()
    );
    let cancelled = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: "test.item.homeward-scroll.4".to_owned(),
            target: None,
        },
    );
    assert_eq!(
        game.recall
            .as_ref()
            .and_then(|recall| recall.remaining_turns),
        None
    );
    assert!(
        cancelled
            .events
            .iter()
            .any(|event| event.kind == "item.recall-cancelled")
    );
}

#[test]
fn v113_dungeon_save_without_recall_derives_a_stable_destination() {
    let mut game = Game::new(2);
    clear_monsters(&mut game);
    descend_one_floor(&mut game);
    let mut payload = game.to_save();
    payload.content_hash =
        "10d3813ec933dd881c23229b604c5f64e67716a56ebdb20b6a844c98593a7653".to_owned();
    payload.player.recall = None;

    let restored = Game::from_save(payload).expect("v113 save should migrate");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-1");
    assert_eq!(
        restored.recall,
        Some(RecallStateDto {
            dungeon_id: "demo.dungeon.echo-depths".to_owned(),
            floor_id: "demo.floor.echo-depth-1".to_owned(),
            remaining_turns: None,
        })
    );
}
