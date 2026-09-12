// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};

fn floor_id(depth: u16) -> String {
    format!("demo.floor.asgard-depth-{depth}")
}

fn map_game() -> Game {
    let mut game = (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|game| game.active_pantheons & 8 != 0)
        .unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game
}

fn definition(game: &Game, depth: u16) -> rfb_content::ProceduralFloorDefinition {
    game.content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == floor_id(depth))
        .unwrap()
        .clone()
}

fn enter_depth(game: &mut Game, depth: u16) {
    // Depth travel preparation, not evidence of the AS5 wilderness entry.
    clear_monsters(game);
    assert!(
        game.transition_floor(floor_id(depth), None, None, false)
            .unwrap()
            .is_some()
    );
    clear_monsters(game);
}

fn traverse(game: &mut Game, terrain: &str, expected: u16) {
    clear_monsters(game);
    place_player_on_terrain(game, terrain);
    assert_eq!(
        dispatch_next(game, GameCommand::TraverseStairs).floor_id,
        floor_id(expected)
    );
    clear_monsters(game);
}

#[test]
fn asgard_normal_nine_depth_route_returns_and_resumes_saved_connections() {
    let mut game = map_game();
    enter_depth(&mut game, 64);
    for depth in [68, 72, 76, 80, 82, 84, 86, 88] {
        traverse(&mut game, "demo.terrain.shaft-down", depth);
    }
    assert!(game.terrain.iter().all(
        |terrain| terrain != "demo.terrain.shaft-down" && terrain != "demo.terrain.stairs-down"
    ));
    let saved = game.to_save();
    let mut restored = Game::from_save(saved.clone()).unwrap();
    assert_eq!(restored.to_save(), saved);
    for depth in [86, 84, 82, 80, 76, 72, 68, 64] {
        traverse(&mut game, "demo.terrain.shaft-up", depth);
        traverse(&mut restored, "demo.terrain.shaft-up", depth);
        assert_eq!(game.state_hash(), restored.state_hash());
    }
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert!(game.current_dungeon_instance_id.is_none());
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn asgard_threshold_and_off_route_shafts_record_actual_return_even_on_stored_floors() {
    let base = map_game();
    for (from, to) in [(76, 80), (77, 81), (78, 80), (79, 81), (80, 82), (81, 83)] {
        let mut game = base.clone();
        // Visit the destination first so the arrival must repair an existing
        // up link, not just initialize a newly generated floor's link.
        enter_depth(&mut game, to);
        enter_depth(&mut game, from);
        traverse(&mut game, "demo.terrain.shaft-down", to);
        let return_link = game
            .floor_connections
            .iter()
            .find(|connection| connection.id.ends_with("shaft-up"))
            .unwrap();
        assert_eq!(
            return_link.target_floor_id.as_deref(),
            Some(floor_id(from).as_str())
        );
        let mut restored = Game::from_save(game.to_save()).unwrap();
        traverse(&mut game, "demo.terrain.shaft-up", from);
        traverse(&mut restored, "demo.terrain.shaft-up", from);
        assert_eq!(game.state_hash(), restored.state_hash());
    }
    // 78 first binds 80's return; another arrival from 76 must replace it.
    let mut game = base;
    enter_depth(&mut game, 78);
    traverse(&mut game, "demo.terrain.shaft-down", 80);
    enter_depth(&mut game, 76);
    traverse(&mut game, "demo.terrain.shaft-down", 80);
    traverse(&mut game, "demo.terrain.shaft-up", 76);
}

#[test]
fn asgard_shallow_off_route_exits_and_bottom_one_level_stairs_keep_boundaries() {
    let base = map_game();
    for depth in [65, 66, 67] {
        let mut game = base.clone();
        enter_depth(&mut game, depth);
        place_player_on_terrain(&mut game, "demo.terrain.shaft-up");
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert!(game.current_dungeon_instance_id.is_none(), "depth {depth}");
        assert!(Game::from_save(game.to_save()).is_ok());
    }
    let mut game = base;
    enter_depth(&mut game, 87);
    traverse(&mut game, "demo.terrain.stairs-down", 88);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    traverse(&mut restored, "demo.terrain.stairs-up", 87);
}

#[test]
fn asgard_saved_connections_reject_missing_direction_span_and_foreign_targets() {
    let mut game = map_game();
    enter_depth(&mut game, 78);
    traverse(&mut game, "demo.terrain.shaft-down", 80);
    let save = game.to_save();
    for (target, connection) in [
        (floor_id(77), "demo.connection.asgard-77-shaft-down"),
        (floor_id(82), "demo.connection.asgard-82-shaft-down"),
        (
            "demo.floor.pyramidal-mound-depth-76".into(),
            "demo.connection.pyramidal-mound-76-shaft-down",
        ),
        (floor_id(76), "demo.connection.asgard-76-missing"),
        (floor_id(99), "demo.connection.asgard-99-shaft-down"),
    ] {
        let mut invalid = save.clone();
        let link = invalid
            .floor_connections
            .iter_mut()
            .find(|link| link.id.ends_with("shaft-up"))
            .unwrap();
        link.target_floor_id = Some(target);
        link.target_connection_id = Some(connection.into());
        assert!(Game::from_save(invalid).is_err(), "{connection}");
    }
    let mut invalid = save.clone();
    invalid.floor_connections.clear();
    assert!(Game::from_save(invalid).is_err());
    let mut invalid = save;
    invalid
        .stored_floors
        .iter_mut()
        .find(|floor| floor.id == floor_id(78))
        .unwrap()
        .connections
        .clear();
    assert!(Game::from_save(invalid).is_err());
}

#[test]
fn asgard_representative_maps_generate_caverns_quartz_passages_and_legal_allocations() {
    let base = map_game();
    let mut materials = BTreeSet::new();
    let mut doors = 0;
    let mut objects = 0;
    let mut gold = 0;
    for depth in [64, 76, 80, 88] {
        let definition = definition(&base, depth);
        let budget = definition.generation_budget.as_ref().unwrap();
        let table = base
            .content
            .encounter_table(definition.encounter_table_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(table.rolls, 12); // 50 * (96*33) / (198*66), rounded down.
        assert_eq!(
            (budget.room_placements, budget.room_area_tiles),
            (Some(6), Some(1100))
        );
        for roll in [depth - 1, depth] {
            let seed = (0..100_000)
                .find(|seed| RfbRng::seeded(*seed).bounded(1000) + 1 == u64::from(roll))
                .unwrap();
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            // CAVERN is the first generation draw for this definition. Retain
            // its actual 600 carved cells and check that the final map keeps them.
            let mut probe = game.clone();
            assert_eq!(probe.rng.bounded(1000) + 1, u64::from(roll));
            let mut cavern = vec![definition.wall_terrain_id.clone(); 96 * 33];
            if roll < depth {
                probe.generate_connected_cavern(
                    &definition,
                    &definition.floor_terrain_id,
                    &mut cavern,
                );
                assert_eq!(
                    cavern
                        .iter()
                        .filter(|id| **id == definition.floor_terrain_id)
                        .count(),
                    600
                );
            }
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            if roll == depth {
                let mut without_cavern = definition.clone();
                without_cavern.layout.as_mut().unwrap().cavern = None;
                let expected = probe
                    .generate_procedural_floor(&without_cavern, None)
                    .unwrap();
                assert_eq!(
                    floor.terrain, expected.terrain,
                    "CAVERN must not trigger at equality"
                );
                assert_eq!(game.rng, probe.rng);
            }
            assert_eq!((floor.width, floor.height), (96, 33));
            let at = |position| {
                game.content
                    .terrain(&floor.terrain[generated_terrain_index(floor.width, position)])
                    .unwrap()
            };
            let walkable = floor
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(index, id)| {
                    let terrain = game.content.terrain(id).unwrap();
                    (terrain.walkable || terrain.open_to_terrain_id.is_some()).then_some(Position {
                        x: (index % 96) as i32,
                        y: (index / 96) as i32,
                    })
                })
                .collect::<BTreeSet<_>>();
            let reached = maze_floor_distances(&walkable, floor.player_position);
            assert_eq!(reached.len(), walkable.len(), "depth {depth}, seed {seed}");
            for (index, id) in cavern
                .iter()
                .enumerate()
                .filter(|(_, id)| **id == definition.floor_terrain_id)
            {
                assert!(
                    reached.contains_key(&Position {
                        x: (index % 96) as i32,
                        y: (index / 96) as i32
                    }),
                    "lost cavern cell {index} ({id})"
                );
            }
            let mut occupied = BTreeSet::from([floor.player_position]);
            for connection in &floor.connections {
                assert!(reached.contains_key(&connection.position));
                occupied.insert(connection.position);
            }
            for actor in &floor.entities {
                assert!(occupied.insert(actor.position));
                assert!(actor_can_cross_terrain(
                    game.content.actor(&actor.kind_id).unwrap(),
                    at(actor.position)
                ));
            }
            let leaders = floor
                .entities
                .iter()
                .filter(|actor| {
                    actor.id.contains(".encounter.") && !actor.id.contains(".companion.")
                })
                .count();
            assert!(leaders > 0 && leaders <= usize::from(table.rolls.min(budget.actor_slots)));
            assert!(floor.items.iter().all(|item| matches!(item.location,
                ItemLocation::Ground(position) if at(position).allows_items())));
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            doors += floor
                .terrain
                .iter()
                .filter(|id| **id == definition.closed_door_terrain_id)
                .count();
            objects += floor.items.len();
            gold += floor.gold_piles.len();
            materials.extend(floor.terrain.iter().cloned());
        }
    }
    assert!(doors > 0 && objects > 0 && gold > 0);
    for material in ["wall", "quartz-vein", "magma-vein"] {
        assert!(materials.contains(&format!("demo.terrain.{material}")));
    }
}
