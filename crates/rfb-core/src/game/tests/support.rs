// SPDX-License-Identifier: MPL-2.0
use super::*;

pub(super) fn command(seq: u32, revision: u32, command: GameCommand) -> GameCommandEnvelope {
    GameCommandEnvelope {
        command_seq: seq,
        expected_revision: revision,
        command,
    }
}

pub(super) fn dispatch_next(game: &mut Game, command_value: GameCommand) -> GameUpdate {
    let snapshot = game.snapshot();
    game.dispatch(command(
        snapshot.last_command_seq + 1,
        snapshot.revision,
        command_value,
    ))
    .expect("test command should execute")
}

pub(super) fn descend_one_floor(game: &mut Game) {
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
    game.traverse_stairs(false)
        .expect("descent should resolve")
        .expect("descent should transition");
}

pub(super) fn place_player_on_terrain(game: &mut Game, terrain_id: &str) {
    let index = game
        .terrain
        .iter()
        .position(|candidate| candidate == terrain_id)
        .unwrap_or_else(|| panic!("current floor should contain {terrain_id}"));
    game.player.position = Position {
        x: i32::try_from(index % usize::from(game.width)).expect("terrain x must fit i32"),
        y: i32::try_from(index / usize::from(game.width)).expect("terrain y must fit i32"),
    };
}

pub(super) fn stored_floor<'a>(game: &'a Game, floor_id: &str) -> &'a FloorState {
    game.stored_floors
        .values()
        .find(|floor| floor.id == floor_id)
        .unwrap_or_else(|| panic!("stored floor {floor_id} should exist"))
}

pub(super) fn generated_encounter_leader_count(game: &Game) -> usize {
    let prefix = format!("{}.encounter.", game.current_floor_id);
    game.entities
        .iter()
        .filter(|actor| {
            actor
                .id
                .strip_prefix(&prefix)
                .is_some_and(|ordinal| ordinal.parse::<u16>().is_ok())
        })
        .count()
}

pub(super) fn visual_at(snapshot: &GameSnapshot, position: Position) -> CellVisualDto {
    *snapshot
        .visual_cells
        .iter()
        .find(|visual| visual.position == position)
        .expect("snapshot should contain every visual cell")
}

pub(super) fn assert_invariant_error_without_mutation(
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

pub(super) fn prepare_death_caster(seed: u64, level: u16, ability_id: &str) -> Game {
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

pub(super) fn skill_check_game(seed: u64, build_id: &str) -> Game {
    let mut game = Game::new_with_build(seed, build_id).expect("skill-check build should create");
    clear_monsters(&mut game);
    game
}

pub(super) fn give_inventory_item(game: &mut Game, id: &str, kind_id: &str) {
    let (activation, charges) =
        initial_item_runtime_state(&game.content, &mut game.rng, kind_id, 1);
    game.items.push(ItemInstance {
        id: id.to_owned(),
        kind_id: kind_id.to_owned(),
        quantity: 1,
        inscription: None,
        origin_actor_kind_id: None,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation,
        charges,
        fuel: initial_item_fuel(&game.content, kind_id),
        device_recovery_progress: 0,
        location: ItemLocation::Inventory,
    });
}

pub(super) fn replace_terrain(game: &mut Game, position: Position, terrain_id: &str) {
    let index = game
        .index(position)
        .expect("test terrain should be in bounds");
    game.terrain[index] = terrain_id.to_owned();
}

pub(super) fn rest_resolution(update: &GameUpdate) -> &RestResolutionDto {
    update
        .events
        .iter()
        .find_map(|event| match event.outcome.as_ref() {
            Some(GameEventOutcomeDto::Rest { resolution }) => Some(resolution),
            _ => None,
        })
        .expect("rest resolution should exist")
}

pub(super) fn clear_monsters(game: &mut Game) {
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
}

pub(super) fn game_with_actor_definition(
    seed: u64,
    actor_kind_id: &str,
    update: impl FnOnce(&mut rfb_content::ActorDefinition),
) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let actor = artifact
        .content
        .actors
        .iter_mut()
        .find(|actor| actor.id == actor_kind_id)
        .unwrap_or_else(|| panic!("demo pack should contain {actor_kind_id}"));
    update(actor);
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("custom actor definition should remain valid"),
    ));
    Game::from_content(seed, catalog, DEFAULT_WORLD_ID)
        .expect("custom actor definition should create a game")
}

pub(super) fn task_service_game(seed: u64) -> Game {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let task_id = "demo.task.test-warrens-depth";
    let facility_id = "demo.town-facility.outpost-count";
    let world = artifact
        .content
        .worlds
        .iter_mut()
        .find(|world| world.id == DEFAULT_WORLD_ID)
        .expect("Middle-earth world should remain available");
    world.tasks.push(rfb_content::TaskDefinition {
        id: task_id.to_owned(),
        name_key: "test-task-name".to_owned(),
        description_key: "test-task-description".to_owned(),
        source_facility_id: Some(facility_id.to_owned()),
        prerequisite_task_id: None,
        location: rfb_content::TaskLocationDefinition::DungeonDepth {
            dungeon_id: "demo.dungeon.warrens".to_owned(),
            depth: 1,
        },
        objectives: vec![rfb_content::TaskObjectiveDefinition {
            kind: rfb_content::TaskObjectiveKind::ClearFloor,
            floor_id: Some("demo.floor.warrens-depth-1".to_owned()),
            required: 1,
            item_instance_id: None,
            item_kind_id: None,
            actor_instance_id: None,
            actor_kind_id: None,
        }],
        target_placements: Vec::new(),
        completion_exit_terrain_id: None,
        reward: rfb_content::TaskRewardDefinition {
            item_instance_id: "demo.task.test-warrens-depth.reward.1".to_owned(),
            item_kind_id: "demo.item.ration-of-food".to_owned(),
            quantity: 1,
        },
    });
    let prerequisite_task_id = "demo.task.test-prerequisite";
    world.tasks.push(rfb_content::TaskDefinition {
        id: prerequisite_task_id.to_owned(),
        name_key: "test-prerequisite-name".to_owned(),
        description_key: "test-prerequisite-description".to_owned(),
        source_facility_id: Some(facility_id.to_owned()),
        prerequisite_task_id: Some(task_id.to_owned()),
        location: rfb_content::TaskLocationDefinition::DungeonDepth {
            dungeon_id: "demo.dungeon.warrens".to_owned(),
            depth: 1,
        },
        objectives: vec![rfb_content::TaskObjectiveDefinition {
            kind: rfb_content::TaskObjectiveKind::ClearFloor,
            floor_id: Some("demo.floor.warrens-depth-1".to_owned()),
            required: 1,
            item_instance_id: None,
            item_kind_id: None,
            actor_instance_id: None,
            actor_kind_id: None,
        }],
        target_placements: Vec::new(),
        completion_exit_terrain_id: None,
        reward: rfb_content::TaskRewardDefinition {
            item_instance_id: "demo.task.test-prerequisite.reward.1".to_owned(),
            item_kind_id: "demo.item.water-potion".to_owned(),
            quantity: 1,
        },
    });
    let facility = artifact
        .content
        .town_facilities
        .iter_mut()
        .find(|facility| facility.id == facility_id)
        .expect("Outpost home should remain available");
    facility
        .task_ids
        .extend([task_id.to_owned(), prerequisite_task_id.to_owned()]);
    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("custom task service definition should remain valid"),
    ));
    Game::from_content(seed, catalog, DEFAULT_WORLD_ID)
        .expect("custom task service game should create")
}
