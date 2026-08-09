// SPDX-License-Identifier: MPL-2.0
use rfb_protocol::TerrainInteractionUnavailableReasonDto;

use super::support::*;
use super::*;

fn enter_world_map_command() -> GameCommand {
    GameCommand::EnterWorldMap {
        leave_pets: false,
        cancel_recall: false,
    }
}

fn confirmed_world_map_command(leave_pets: bool, cancel_recall: bool) -> GameCommand {
    GameCommand::EnterWorldMap {
        leave_pets,
        cancel_recall,
    }
}

#[test]
fn entering_world_map_requires_explicit_pet_and_active_recall_confirmation() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let pet = game
        .entities
        .first_mut()
        .expect("Outpost surface should retain at least one actor");
    pet.controller_id = Some(game.player.id.clone());
    game.recall = Some(RecallStateDto {
        dungeon_id: "demo.dungeon.warrens".to_owned(),
        floor_id: "demo.floor.warrens-depth-1".to_owned(),
        remaining_turns: Some(10),
    });

    let rejected = game.dispatch(command(1, 0, confirmed_world_map_command(false, false)));
    assert!(matches!(
        rejected,
        Err(CoreError::WorldMapTransitionUnavailable)
    ));

    let entered = game
        .dispatch(command(1, 0, confirmed_world_map_command(true, true)))
        .expect("explicit confirmations should enter the world map");
    assert_eq!(entered.map_scale, MapScaleDto::World);
    assert_eq!(entered.player.recall.unwrap().remaining_turns, None);
}

#[test]
fn warrens_journey_starts_on_an_outdoor_surface_with_a_working_entrance() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");

    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!((game.width, game.height), (96, 32));
    assert_eq!(game.player.position, Position { x: 44, y: 16 });
    assert_eq!(
        game.terrain_at(Position { x: 44, y: 16 }),
        "demo.terrain.surface-path"
    );
    assert_eq!(
        game.terrain_at(Position { x: 74, y: 16 }),
        "demo.terrain.stairs-down"
    );
    assert_eq!(
        game.terrain_at(Position { x: 0, y: 0 }),
        "demo.terrain.surface-tree"
    );

    game.player.position = Position { x: 73, y: 16 };
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(update.floor_id, "demo.floor.warrens-depth-1");
}

#[test]
fn thieves_hideout_inline_floor_preserves_the_fixed_map_and_six_member_formation() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Warrens world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("thieves' hideout should remain available")
        .clone();
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("fixed thieves' hideout should generate");

    let rows = floor
        .terrain
        .chunks(usize::from(floor.width))
        .map(|row| {
            row.iter()
                .map(|terrain_id| match terrain_id.as_str() {
                    "demo.terrain.permanent-wall" => '#',
                    "demo.terrain.floor" => '.',
                    "demo.terrain.door-closed" => '+',
                    "demo.terrain.stairs-up" => '<',
                    "demo.terrain.warren-snare" => '^',
                    other => panic!("unexpected fixed-map terrain {other}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "#####################",
            "#####...#...#...#...#",
            "#####...#...#...#...#",
            "#####...#...#...#.^.#",
            "#<..##+###+###+###+##",
            "#..^#....^...^..^...#",
            "#...+...............#",
            "#####################",
        ]
    );
    assert_eq!(floor.player_position, Position { x: 1, y: 4 });
    assert_eq!(floor.entities.len(), 6);
    assert_eq!(floor.items.len(), 4);

    let candidates = [
        "demo.actor.agent-of-black-market",
        "demo.actor.bandit",
        "demo.actor.filthy-street-urchin",
        "demo.actor.nibelung",
        "demo.actor.novice-rogue",
        "demo.actor.scruffy-looking-hobbit",
        "demo.actor.tax-collector",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let expected_positions = [
        Position { x: 8, y: 6 },
        Position { x: 6, y: 2 },
        Position { x: 18, y: 2 },
        Position { x: 10, y: 2 },
        Position { x: 14, y: 2 },
        Position { x: 15, y: 6 },
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(
        floor
            .entities
            .iter()
            .map(|entity| entity.position)
            .collect::<BTreeSet<_>>(),
        expected_positions
    );
    assert!(
        floor
            .entities
            .iter()
            .all(|entity| candidates.contains(entity.kind_id.as_str()))
    );

    let selected_order = floor
        .entities
        .iter()
        .map(|entity| {
            let actor = game
                .content
                .actor(&entity.kind_id)
                .expect("formation actor should remain available");
            (
                actor.level,
                actor
                    .allocation
                    .as_ref()
                    .expect("formation actor should retain allocation")
                    .legacy_index,
            )
        })
        .collect::<Vec<_>>();
    assert!(
        selected_order.windows(2).all(|pair| {
            pair[0].0 > pair[1].0 || pair[0].0 == pair[1].0 && pair[0].1 <= pair[1].1
        })
    );
}

#[test]
fn warrens_surface_reentry_starts_a_fresh_expedition_with_new_monsters() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");

    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    let first_instance = game
        .current_dungeon_instance_id
        .clone()
        .expect("Warrens entry should allocate an instance");
    assert_eq!(generated_encounter_leader_count(&game), 4);

    game.entities.clear();
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, "demo.floor.surface");
    assert!(
        game.stored_floors
            .values()
            .all(|floor| floor.dungeon_instance_id.as_deref() != Some(first_instance.as_str()))
    );

    let draws_before_reentry = game.rng.draw_counter;
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    let reentry = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(reentry.floor_id, "demo.floor.warrens-depth-1");
    assert_ne!(
        game.current_dungeon_instance_id.as_deref(),
        Some(first_instance.as_str())
    );
    assert!(game.rng.draw_counter > draws_before_reentry);
    assert_eq!(generated_encounter_leader_count(&game), 4);
}

#[test]
fn warrens_maps_are_seeded_connected_varied_and_persistent() {
    let mut generated_maps = BTreeSet::new();
    let mut walkable_masks = Vec::<Vec<bool>>::new();
    for seed in 0..16 {
        let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        dispatch_next(&mut game, GameCommand::TraverseStairs);

        assert_eq!((game.width, game.height), (66, 22));
        let route_terrain = game
            .terrain
            .iter()
            .map(|terrain_id| match terrain_id.as_str() {
                "demo.terrain.magma-vein" | "demo.terrain.quartz-vein" => {
                    "demo.terrain.wall".to_owned()
                }
                _ => terrain_id.clone(),
            })
            .collect::<Vec<_>>();
        assert!(
            generated_terrain_is_connected(&route_terrain, game.width, game.height, &game.content,),
            "seed {seed} should retain a connected travel network"
        );
        assert!(
            (1..=2).contains(
                &game
                    .terrain
                    .iter()
                    .filter(|terrain_id| **terrain_id == "demo.terrain.stairs-up")
                    .count()
            )
        );
        assert_eq!(generated_encounter_leader_count(&game), 4);
        assert_eq!(
            game.terrain
                .iter()
                .filter(|terrain_id| {
                    matches!(
                        terrain_id.as_str(),
                        "demo.terrain.magma-vein" | "demo.terrain.quartz-vein"
                    )
                })
                .count(),
            24
        );

        let walkable_mask = game
            .terrain
            .iter()
            .map(|terrain_id| {
                game.content
                    .terrain(terrain_id)
                    .expect("generated terrain must remain available")
                    .walkable
            })
            .collect::<Vec<_>>();
        for previous in &walkable_masks {
            let structural_difference = previous
                .iter()
                .zip(&walkable_mask)
                .filter(|(left, right)| left != right)
                .count();
            assert!(
                structural_difference >= 120,
                "seed {seed} only changed {structural_difference} walkable cells"
            );
        }
        walkable_masks.push(walkable_mask);
        assert!(
            (4..=5).contains(
                &game
                    .terrain
                    .iter()
                    .filter(|terrain_id| **terrain_id == "demo.terrain.stairs-down")
                    .count()
            )
        );

        game.entities.clear();
        game.items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        let first_floor_terrain = game.terrain.clone();
        let first_floor_items = game.items.clone();
        let ground_item_count = first_floor_items
            .iter()
            .filter(|item| matches!(item.location, ItemLocation::Ground(_)))
            .count();
        assert!(
            (2..=5).contains(&ground_item_count),
            "seed {seed} generated {ground_item_count} floor items"
        );
        assert!(first_floor_items.iter().all(|item| {
            !matches!(item.location, ItemLocation::Ground(_))
                || !matches!(
                    item.kind_id.as_str(),
                    "demo.item.arrow"
                        | "demo.item.frailty-tonic"
                        | "demo.item.venom-draught"
                        | "demo.item.cartography-scroll"
                        | "demo.item.clamor-scroll"
                        | "demo.item.homeward-scroll"
                        | "demo.item.short-sword"
                        | "demo.item.trapfinding-scroll"
                )
        }));
        let mut same_seed = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
            .expect("same-seed Warrens journey should create");
        place_player_on_terrain(&mut same_seed, "demo.terrain.stairs-down");
        dispatch_next(&mut same_seed, GameCommand::TraverseStairs);
        same_seed.entities.clear();
        same_seed
            .items
            .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
        assert_eq!(same_seed.terrain, first_floor_terrain);
        assert_eq!(same_seed.items, first_floor_items);

        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(game.terrain, first_floor_terrain);
        assert_eq!(game.items, first_floor_items);
        generated_maps.insert(first_floor_terrain);
    }
    assert!(
        generated_maps.len() >= 15,
        "fixed seed matrix should produce visibly distinct Warrens maps"
    );
}

#[test]
fn warrens_every_generated_floor_has_a_normal_descent_and_return_route() {
    let mut saw_scaled_allocation_above_minimum = false;
    let mut saw_depth_gated_item = false;
    for seed in 0..16 {
        let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
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
            assert_eq!(generated_encounter_leader_count(&game), 4);
            if depth == 9 {
                assert!(
                    game.entities
                        .iter()
                        .any(|actor| actor.id == "demo.guardian.warrens.1")
                );
            }
            let ground_items = game
                .items
                .iter()
                .filter(|item| matches!(item.location, ItemLocation::Ground(_)))
                .collect::<Vec<_>>();
            assert!(
                (2..=5).contains(&ground_items.len()),
                "seed {seed} depth {depth} generated {} floor items",
                ground_items.len()
            );
            saw_scaled_allocation_above_minimum |= ground_items.len() > 2;
            saw_depth_gated_item |= depth >= 5
                && ground_items.iter().any(|item| {
                    matches!(
                        item.kind_id.as_str(),
                        "demo.item.cartography-scroll"
                            | "demo.item.clamor-scroll"
                            | "demo.item.homeward-scroll"
                            | "demo.item.short-sword"
                            | "demo.item.trapfinding-scroll"
                    )
                });
            assert_eq!(
                game.terrain
                    .iter()
                    .filter(|terrain_id| {
                        matches!(
                            terrain_id.as_str(),
                            "demo.terrain.magma-vein" | "demo.terrain.quartz-vein"
                        )
                    })
                    .count(),
                24
            );
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
    assert!(saw_scaled_allocation_above_minimum);
    assert!(saw_depth_gated_item);
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
            discovered: true,
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
fn entrance_guardian_defeat_persists() {
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
        VisibilityState::Hidden
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
fn warrens_location_requires_its_local_entrance_and_restores_the_outpost() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let outpost_position = game.player.position;
    let task_states = game.task_states.clone();
    let shop_states = game.shop_states.clone();

    dispatch_next(&mut game, enter_world_map_command());
    let direct_entry = game.dispatch(command(
        game.last_command_seq + 1,
        game.revision,
        GameCommand::TraverseStairs,
    ));
    assert!(matches!(
        direct_entry,
        Err(CoreError::WorldMapActionUnavailable)
    ));

    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.player.position, outpost_position);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    let entrance_position = game.player.position;

    game.wilderness_position = Some(Position { x: 29, y: 52 });
    assert!(
        game.traverse_stairs(false)
            .expect("unbound entrance check should resolve")
            .is_none()
    );

    game.wilderness_position = Some(Position { x: 28, y: 52 });
    game.traverse_stairs(false)
        .expect("Warrens entry should resolve")
        .expect("the bound local entrance should open Warrens");
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");

    game.entities.clear();
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    game.traverse_stairs(false)
        .expect("Warrens exit should resolve")
        .expect("the dungeon exit should restore the surface");

    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(game.player.position, entrance_position);
    assert_eq!(game.task_states, task_states);
    assert_eq!(game.shop_states, shop_states);
}

#[test]
fn world_map_projects_authoritative_wilderness_cells_and_restores_the_local_map() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let local_position = game.player.position;
    let world_tick = game.world_tick;
    assert!(
        game.snapshot()
            .content_visuals
            .iter()
            .any(|visual| visual.id == "core.wilderness.road" && visual.glyph == ".")
    );

    let entered = dispatch_next(&mut game, enter_world_map_command());
    assert_eq!(entered.map_scale, MapScaleDto::World);
    assert_eq!((entered.width, entered.height), (99, 66));
    assert_eq!(entered.player.position, Position { x: 28, y: 52 });
    assert_eq!(entered.changed_cells.len(), 99 * 66);
    assert_eq!(entered.changed_visual_cells.len(), 99 * 66);
    assert!(entered.entities.is_empty());
    assert!(entered.items.is_empty());
    assert!(entered.shops.is_empty());
    assert!(entered.terrain_interactions.is_empty());
    assert_eq!(game.world_tick, world_tick);

    let current = entered
        .changed_cells
        .iter()
        .find(|cell| cell.position == Position { x: 28, y: 52 })
        .expect("world position should be projected");
    assert_eq!(current.terrain_id, "core.wilderness.town");
    assert_eq!(current.danger_level, Some(0));
    assert_eq!(current.locations.len(), 2);
    assert!(
        current
            .locations
            .iter()
            .any(|location| location.id == "demo.town.outpost")
    );
    assert!(
        current
            .locations
            .iter()
            .any(|location| location.id == "demo.dungeon.warrens")
    );

    let save = game.to_save();
    assert_eq!(save.map_scale, MapScaleDto::World);
    assert_eq!(save.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(save.wilderness_seed, 42);
    let mut restored = Game::from_save(save).expect("world map state should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot().map_scale, MapScaleDto::World);

    let blocked = restored.dispatch(command(
        restored.last_command_seq + 1,
        restored.revision,
        GameCommand::Wait,
    ));
    assert!(matches!(blocked, Err(CoreError::WorldMapActionUnavailable)));

    let left = dispatch_next(&mut restored, GameCommand::LeaveWorldMap);
    assert_eq!(left.map_scale, MapScaleDto::Local);
    assert_eq!((left.width, left.height), (96, 32));
    assert_eq!(left.player.position, local_position);
    assert_eq!(left.changed_cells.len(), 96 * 32);
    assert_eq!(restored.world_tick, world_tick);
}

#[test]
fn world_map_movement_uses_original_time_scale_without_advancing_hidden_monsters() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let hidden_entities = game.entities.clone();
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(1);
    let nutrition = game.nutrition;
    dispatch_next(&mut game, enter_world_map_command());
    let world_tick = game.world_tick;

    let moved = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );

    assert_eq!(game.wilderness_position, Some(Position { x: 29, y: 52 }));
    assert_eq!(moved.player.position, Position { x: 29, y: 52 });
    assert_eq!(moved.changed_cells.len(), 2);
    assert_eq!(
        game.world_tick - world_tick,
        u32::try_from(
            STANDARD_ACTION_COST * wilderness::WORLD_MAP_ACTION_MULTIPLIER
                / energy_gain(derived_speed(&game.player_derived_stats().speed)),
        )
        .expect("world-map travel ticks must fit u32")
    );
    assert!(game.nutrition < nutrition);
    assert_eq!(game.entities, hidden_entities);
    assert_eq!(game.rng, expected_rng);
}

#[test]
fn wilderness_daylight_drives_surface_ambient_light() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let ambient_light = |game: &Game| {
        let sources = game.collect_light_sources();
        game.ambient_light(game.player.position, &sources)
    };

    game.world_tick = 49_999;
    assert_eq!(ambient_light(&game), SURFACE_AMBIENT_LIGHT);
    game.world_tick = 50_000;
    assert_eq!(ambient_light(&game), DUNGEON_AMBIENT_LIGHT);
    game.world_tick = 100_000;
    assert_eq!(ambient_light(&game), SURFACE_AMBIENT_LIGHT);
}

#[test]
fn wilderness_ambush_enters_local_combat_and_locks_world_map_until_cleared() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    let start = game
        .wilderness_position
        .expect("world map should retain the current position");
    let ambush_position = Position {
        x: start.x + 1,
        y: start.y,
    };
    let travel_destination = Position {
        x: start.x + 2,
        y: start.y,
    };
    game.wilderness_position = Some(ambush_position);
    let ambush_seed = (0..10_000)
        .find(|seed| {
            game.rng = RfbRng::seeded(*seed);
            game.roll_wilderness_ambush()
        })
        .expect("a deterministic ambush seed should be found");
    game.wilderness_position = Some(start);
    game.rng = RfbRng::seeded(ambush_seed);
    let world_tick = game.world_tick;

    let ambushed = dispatch_next(
        &mut game,
        GameCommand::TravelWorld {
            destination: travel_destination,
        },
    );

    assert_eq!(ambushed.map_scale, MapScaleDto::Local);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(ambush_position));
    assert_eq!(ambushed.world_travel_destination, Some(travel_destination));
    assert!(
        ambushed
            .events
            .iter()
            .any(|event| event.kind == "wilderness.ambushed")
    );
    assert!(
        game.entities
            .iter()
            .any(|entity| entity.id.contains(".ambush."))
    );
    let player_gain = energy_gain(derived_speed(&game.player_derived_stats().speed));
    assert_eq!(
        game.world_tick - world_tick,
        u32::try_from((STANDARD_ACTION_COST + player_gain - 1) / player_gain)
            .expect("ambush initiative ticks must fit u32")
    );

    let mut restored = Game::from_save(game.to_save()).expect("ambush should round-trip");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.world_travel_destination, Some(travel_destination));
    let blocked = restored.dispatch(command(
        restored.last_command_seq + 1,
        restored.revision,
        enter_world_map_command(),
    ));
    assert!(matches!(
        blocked,
        Err(CoreError::WorldMapTransitionUnavailable)
    ));

    let owner_id = restored
        .entities
        .iter()
        .find(|entity| entity.id.contains(".ambush.") && !restored.actor_is_player_side(entity))
        .expect("ambush owner should remain available")
        .id
        .clone();
    let mut summoned = restored
        .entities
        .iter()
        .find(|entity| entity.id == owner_id)
        .expect("ambush owner should remain available")
        .clone();
    summoned.id = "summon.test.ambush-threat".to_owned();
    summoned.summon = Some(SummonIdentity {
        owner_id,
        source_ability_id: "test.ability.summon".to_owned(),
        remaining_turns: 10,
    });
    restored
        .entities
        .retain(|entity| !entity.id.contains(".ambush."));
    restored.entities.push(summoned);
    let summoned_threat = restored.dispatch(command(
        restored.last_command_seq + 1,
        restored.revision,
        enter_world_map_command(),
    ));
    assert!(matches!(
        summoned_threat,
        Err(CoreError::WorldMapTransitionUnavailable)
    ));

    restored.entities.clear();
    let entered = dispatch_next(&mut restored, enter_world_map_command());
    assert_eq!(entered.map_scale, MapScaleDto::World);
    assert_eq!(entered.world_travel_destination, Some(travel_destination));
}

#[test]
fn local_wilderness_is_coordinate_seeded_and_restores_from_save() {
    fn enter_eastern_wilderness(seed: u64) -> Game {
        let mut game = Game::new_warrens_journey_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        dispatch_next(&mut game, enter_world_map_command());
        dispatch_next(
            &mut game,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        let simulation_rng = game.rng.clone();
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        assert_eq!(game.rng, simulation_rng);
        game
    }

    let game = enter_eastern_wilderness(42);
    let duplicate = enter_eastern_wilderness(42);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!((game.width, game.height), (96, 32));
    assert_eq!(game.player.position, Position { x: 48, y: 16 });
    assert_eq!(game.terrain, duplicate.terrain);
    assert_eq!(game.entities, duplicate.entities);
    assert_eq!(
        game.terrain_at(Position { x: 0, y: 16 }),
        "demo.terrain.surface-path"
    );
    assert_eq!(
        game.terrain_at(Position { x: 95, y: 16 }),
        "demo.terrain.surface-path"
    );
    assert!(game.stored_floors.contains_key("demo.floor.surface"));

    let restored = Game::from_save(game.to_save()).expect("local wilderness should reload");
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.terrain, game.terrain);
    assert_eq!(restored.entities, game.entities);
}

#[test]
fn walking_across_a_local_wilderness_edge_regenerates_the_neighbor_coordinate() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let previous_terrain = game.terrain.clone();
    game.player.position = Position { x: 95, y: 16 };

    let crossed = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );

    assert_eq!(game.wilderness_position, Some(Position { x: 30, y: 52 }));
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.player.position, Position { x: 1, y: 16 });
    assert_eq!(crossed.changed_cells.len(), 96 * 32);
    assert_ne!(game.terrain, previous_terrain);
    assert_eq!(game.stored_floors.len(), 1);
}

#[test]
fn returning_to_the_outpost_coordinate_restores_its_preserved_floor() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    let town_position = game.player.position;
    let town_terrain = game.terrain.clone();
    let town_entities = game.entities.clone();
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::West,
        },
    );

    let returned = dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.player.position, town_position);
    assert_eq!(game.terrain, town_terrain);
    assert_eq!(game.entities, town_entities);
    assert_eq!(returned.changed_cells.len(), 96 * 32);
    assert!(game.stored_floors.is_empty());
}

#[test]
fn world_map_can_only_be_entered_from_a_surface_that_defines_wilderness() {
    let mut no_wilderness = Game::new(42);
    let result = no_wilderness.dispatch(command(1, 0, enter_world_map_command()));
    assert!(matches!(
        result,
        Err(CoreError::WorldMapTransitionUnavailable)
    ));

    let mut dungeon = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
    dungeon.player.position = Position { x: 74, y: 16 };
    dispatch_next(&mut dungeon, GameCommand::TraverseStairs);
    let result = dungeon.dispatch(command(
        dungeon.last_command_seq + 1,
        dungeon.revision,
        enter_world_map_command(),
    ));
    assert!(matches!(
        result,
        Err(CoreError::WorldMapTransitionUnavailable)
    ));
}
