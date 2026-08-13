// SPDX-License-Identifier: MPL-2.0
use std::sync::OnceLock;

use super::*;

pub(super) fn enable_test_caster(content: &mut rfb_content::CompiledContentV1) {
    let mut class = content
        .classes
        .iter()
        .find(|class| class.id == "demo.class.warrior")
        .expect("Warrior class should remain available")
        .clone();
    class.id = "test.class.caster".to_owned();
    class.name_key = "test-class-caster-name".to_owned();
    class.description_key = "test-class-caster-description".to_owned();
    class.modifiers.intelligence = 3;
    class.modifiers.wisdom = 0;
    class.uses_spell_scrolls = true;
    class.casting_profile = Some(rfb_content::CastingProfileDefinition {
        resource_id: "demo.resource.mana".to_owned(),
        casting_attribute: rfb_content::CastingAttribute::Intelligence,
        capacity_formula: rfb_content::CastingCapacityFormula::Linear,
        base_capacity: 4,
        capacity_per_level: 2,
        capacity_per_attribute_index: 1,
        capacity_percent: 100,
        learning_formula: rfb_content::CastingLearningFormula::Linear,
        study_mode: rfb_content::CastingStudyMode::Chosen,
        failure_formula: rfb_content::CastingFailureFormula::Linear,
        base_learning_capacity: 2,
        learning_capacity_per_level: 1,
        learning_capacity_per_attribute_index: 0,
        learning_capacity_cap: 16,
        resource_recovery_percent: 100,
        minimum_failure_percent: 5,
        beam_chance_level_multiplier: 1,
        beam_chance_level_divisor: 1,
        beam_chance_bonus: 0,
        spell_damage_bonus_base: 0,
        spell_damage_bonus_per_level: 0,
        spell_damage_bonus_level_divisor: 1,
        encumbrance: None,
        realm_profiles: vec![rfb_content::CastingRealmProfileDefinition {
            realm_id: "death".to_owned(),
            ability_book_ids: vec![
                "demo.ability-book.black-prayers".to_owned(),
                "demo.ability-book.black-mass".to_owned(),
                "demo.ability-book.black-channels".to_owned(),
                "demo.ability-book.necronomicon".to_owned(),
            ],
            learning_capacity_bonus: 0,
            ability_overrides: Vec::new(),
        }],
    });
    class.starting_items.extend([
        rfb_content::StartingItemDefinition {
            item_kind_id: "demo.item.black-prayers".to_owned(),
            quantity: 1,
            maximum_quantity: None,
            equipped: false,
        },
        rfb_content::StartingItemDefinition {
            item_kind_id: "demo.item.black-mass".to_owned(),
            quantity: 1,
            maximum_quantity: None,
            equipped: false,
        },
        rfb_content::StartingItemDefinition {
            item_kind_id: "demo.item.black-channels".to_owned(),
            quantity: 1,
            maximum_quantity: None,
            equipped: false,
        },
        rfb_content::StartingItemDefinition {
            item_kind_id: "demo.item.necronomicon".to_owned(),
            quantity: 1,
            maximum_quantity: None,
            equipped: false,
        },
    ]);
    content.classes.push(class);
    let mut build = content
        .builds
        .iter()
        .find(|build| build.id == "demo.build.warrior")
        .expect("Warrior build should remain available")
        .clone();
    build.id = "test.build.caster".to_owned();
    build.name_key = "test-build-caster-name".to_owned();
    build.description_key = "test-build-caster-description".to_owned();
    build.class_id = "test.class.caster".to_owned();
    build.first_realm_id = Some("death".to_owned());
    build.second_realm_id = None;
    content.builds.push(build);
}

pub(crate) fn test_caster_game(seed: u64) -> Game {
    static CONTENT: OnceLock<Arc<rfb_content::ContentCatalog>> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("core crate should be inside the workspace")
                .join("packs/rfb-demo-original");
            let mut artifact =
                rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
            enable_test_caster(&mut artifact.content);
            Arc::new(rfb_content::ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content)
                    .expect("test caster content should remain valid"),
            ))
        })
        .clone();
    Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, "test.build.caster")
        .expect("test caster should create")
}

pub(crate) fn divine_caster_game(seed: u64) -> Game {
    static CONTENT: OnceLock<Arc<rfb_content::ContentCatalog>> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .and_then(std::path::Path::parent)
                .expect("core crate should be inside the workspace")
                .join("packs/rfb-demo-original");
            let mut artifact =
                rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
            enable_test_caster(&mut artifact.content);
            artifact
                .content
                .classes
                .iter_mut()
                .find(|class| class.id == "test.class.caster")
                .and_then(|class| class.casting_profile.as_mut())
                .expect("test caster should have a casting profile")
                .study_mode = rfb_content::CastingStudyMode::DivineRandom;
            Arc::new(rfb_content::ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content)
                    .expect("divine caster content should remain valid"),
            ))
        })
        .clone();
    Game::from_content_with_build(seed, content, DEFAULT_WORLD_ID, "test.build.caster")
        .expect("divine test caster should create")
}

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
    assert_eq!(game.last_visual_cells, before.last_visual_cells);
}

pub(super) fn prepare_death_caster(seed: u64, level: u16, ability_id: &str) -> Game {
    let mut game = test_caster_game(seed);
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
        .expect("test caster should have Mana");
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
        origin_kind: None,
        damage_dice_override: None,
        discount_percent: 0,
        quality: ItemQualityDto::Ordinary,
        affix_ids: Vec::new(),
        rolled_affixes: Vec::new(),
        enchantments: Default::default(),
        curse: None,
        activation,
        charges,
        fuel: initial_item_fuel(&game.content, kind_id),
        device_recovery_progress: 0,
        captured_actor: None,
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
    // These tests mutate one actor in isolation. Keep unrelated fixed-floor
    // candidate snapshots from imposing their production depth constraint on
    // the synthetic actor definition.
    for world in &mut artifact.content.worlds {
        for floor in &mut world.procedural_floors {
            if let Some(formation) = floor
                .inline_map
                .as_mut()
                .and_then(|inline_map| inline_map.monster_formation.as_mut())
            {
                formation
                    .candidate_actor_kind_ids
                    .retain(|candidate| candidate != actor_kind_id);
            }
        }
    }
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
            entries: vec![
                rfb_content::TaskRewardEntryDefinition {
                    item_kind_id: "demo.item.ration-of-food".to_owned(),
                    quantity: 1,
                    weight: 1,
                    affix_ids: Vec::new(),
                },
                rfb_content::TaskRewardEntryDefinition {
                    item_kind_id: "demo.item.water-potion".to_owned(),
                    quantity: 1,
                    weight: 1,
                    affix_ids: Vec::new(),
                },
            ],
            class_overrides: Vec::new(),
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
            entries: vec![rfb_content::TaskRewardEntryDefinition {
                item_kind_id: "demo.item.water-potion".to_owned(),
                quantity: 1,
                weight: 1,
                affix_ids: Vec::new(),
            }],
            class_overrides: vec![rfb_content::TaskRewardClassOverrideDefinition {
                class_id: "demo.class.warrior".to_owned(),
                entries: vec![rfb_content::TaskRewardEntryDefinition {
                    item_kind_id: "demo.item.broad-sword".to_owned(),
                    quantity: 1,
                    weight: 1,
                    affix_ids: vec!["rfb-legacy.affix.combat".to_owned()],
                }],
            }],
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
