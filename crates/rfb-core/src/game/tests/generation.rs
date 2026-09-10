// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_content::ProceduralFloorDefinition;

use super::*;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};

pub(super) fn arena_geometry_definition(game: &Game) -> ProceduralFloorDefinition {
    let mut definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    // AR1 exercises geometry through the existing floor generator; the real
    // dungeon and its encounter/reward data remain scheduled for AR2–AR4.
    definition.depth = 50;
    definition.width = 96;
    definition.height = 33;
    definition.wall_terrain_id = "demo.terrain.permanent-wall".into();
    definition.vault_id = None;
    definition.theme_table_id = None;
    definition.loot_allocation = None;
    definition.gold_allocation = None;
    definition.guaranteed_items.clear();
    definition.connections.clear();
    let layout = definition.layout.as_mut().unwrap();
    layout.mode = ProceduralLayoutMode::ArenaRooms;
    layout.streamers.clear();
    let geometry = layout.rooms.as_mut().unwrap();
    geometry.min_width = 7;
    geometry.max_width = 15;
    geometry.min_height = 7;
    geometry.max_height = 15;
    geometry.shapes = vec![rfb_content::ProceduralRoomShapeCandidateDefinition {
        shape: ProceduralRoomShape::Circle,
        weight: 1,
    }];
    let budget = definition.generation_budget.as_mut().unwrap();
    budget.actor_slots = 6;
    budget.loot_placements = 0;
    budget.room_placements = Some(6);
    budget.room_area_tiles = Some(800);
    budget.streamer_placements = None;
    budget.streamer_area_tiles = None;
    definition
}

#[test]
fn arena_dungeon_circles_keep_source_radius_and_floor_area() {
    let mut game = Game::new(42);
    let definition = arena_geometry_definition(&game);
    let geometry = definition.layout.as_ref().unwrap().rooms.as_ref().unwrap();
    let mut sizes = BTreeSet::new();
    for seed in 0..12 {
        game.rng = RfbRng::seeded(seed);
        let rooms = game.generate_budgeted_rooms(&definition, geometry);
        let mut repeated = game.clone();
        repeated.rng = RfbRng::seeded(seed);
        assert_eq!(
            rooms,
            repeated.generate_budgeted_rooms(&definition, geometry)
        );
        for (index, room) in rooms.iter().enumerate() {
            sizes.insert(room.width);
            assert_eq!(room.width, room.height);
            assert_eq!(room.width % 2, 1);
            let radius = room.width / 2;
            assert!((3..=7).contains(&radius));
            let mut floor_count = 0;
            for y in room.y..room.y + room.height {
                for x in room.x..room.x + room.width {
                    let dx = (x - room.center().x).abs();
                    let dy = (y - room.center().y).abs();
                    let inside = dx.max(dy) + dx.min(dy) / 2 < radius;
                    assert_eq!(room.contains(Position { x, y }), inside);
                    floor_count += u32::from(inside);
                }
            }
            assert_eq!(room.area(), floor_count);
            for other in rooms.iter().skip(index + 1) {
                assert!(
                    room.x + room.width < other.x
                        || other.x + other.width < room.x
                        || room.y + room.height < other.y
                        || other.y + other.height < room.y
                );
            }
        }
        assert!(rooms.iter().map(GeneratedRoom::area).sum::<u32>() <= 800);
    }
    assert_eq!(sizes, BTreeSet::from([7, 9, 11, 13, 15]));
}

#[test]
fn arena_dungeon_passages_light_and_reserved_positions_survive_permanent_walls() {
    let template = Game::new(42);
    let mut definition = arena_geometry_definition(&template);
    definition.guardian = template
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find_map(|floor| floor.guardian.clone());
    let guardian_id = &definition.guardian.as_ref().unwrap().instance_id;
    definition.generation_budget.as_mut().unwrap().actor_slots += 1;
    let mut signatures = BTreeSet::new();
    let mut lit_rooms = 0;
    let mut dark_rooms = 0;
    let mut doors = 0;
    for seed in 0..8 {
        let mut game = template.clone();
        game.rng = RfbRng::seeded(seed);
        let rooms = game.clone().generate_budgeted_rooms(
            &definition,
            definition.layout.as_ref().unwrap().rooms.as_ref().unwrap(),
        );
        let floor = game
            .clone()
            .generate_procedural_floor(&definition, None)
            .unwrap();
        let repeated = game.generate_procedural_floor(&definition, None).unwrap();
        assert_eq!(floor.terrain, repeated.terrain);
        assert_eq!(floor.glow, repeated.glow);
        signatures.insert(floor.terrain.clone());
        assert!(floor.vault_cells.iter().all(|cell| !cell));
        let traversable = floor
            .terrain
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let terrain = game.content.terrain(id).unwrap();
                (terrain.walkable || terrain.open_to_terrain_id.is_some()).then_some(Position {
                    x: (index % usize::from(floor.width)) as i32,
                    y: (index / usize::from(floor.width)) as i32,
                })
            })
            .collect::<BTreeSet<_>>();
        let reached = maze_floor_distances(&traversable, floor.player_position);
        assert_eq!(
            reached.len(),
            traversable.len(),
            "all passages must connect"
        );
        for room in &rooms {
            assert!(reached.contains_key(&room.center()));
            let lit = floor.glow[generated_terrain_index(floor.width, room.center())];
            lit_rooms += usize::from(lit);
            dark_rooms += usize::from(!lit);
            for y in room.y..room.y + room.height {
                for x in room.x..room.x + room.width {
                    let position = Position { x, y };
                    if room.contains(position) {
                        assert!(reached.contains_key(&position));
                        assert_eq!(
                            floor.glow[generated_terrain_index(floor.width, position)],
                            lit
                        );
                    } else if floor.terrain[generated_terrain_index(floor.width, position)]
                        == definition.wall_terrain_id
                        && (-1..=1).any(|dy| {
                            (-1..=1).any(|dx| {
                                room.contains(Position {
                                    x: x + dx,
                                    y: y + dy,
                                })
                            })
                        })
                    {
                        assert_eq!(
                            floor.glow[generated_terrain_index(floor.width, position)],
                            lit
                        );
                    }
                }
            }
        }
        let guardian = floor
            .entities
            .iter()
            .find(|actor| &actor.id == guardian_id)
            .unwrap();
        for room in &rooms {
            assert_ne!(guardian.position, room.center());
            assert!(
                floor
                    .entities
                    .iter()
                    .any(|actor| actor.position == room.center())
            );
        }
        assert_ne!(guardian.position, floor.player_position);
        assert!(reached.contains_key(&guardian.position));
        assert!(
            !floor.terrain[generated_terrain_index(floor.width, guardian.position)]
                .contains("stairs")
        );
        let mut occupied = BTreeSet::from([floor.player_position]);
        for actor in &floor.entities {
            assert!(occupied.insert(actor.position));
        }
        doors += floor
            .terrain
            .iter()
            .filter(|id| *id == &definition.closed_door_terrain_id)
            .count();
        assert!(
            floor
                .terrain
                .iter()
                .all(|id| id == &definition.wall_terrain_id
                    || id == &definition.floor_terrain_id
                    || id == &definition.closed_door_terrain_id
                    || id == &definition.up_stair_terrain_id
                    || Some(id) == definition.down_stair_terrain_id.as_ref()
                    || id == &definition.trap_terrain_id)
        );
        // Dig a wall from this generated floor through the real command consumer.
        let (standing, wall) = traversable
            .iter()
            .find_map(|position| {
                let wall = Position {
                    x: position.x,
                    y: position.y - 1,
                };
                (floor.terrain[generated_terrain_index(floor.width, wall)]
                    == definition.wall_terrain_id)
                    .then_some((*position, wall))
            })
            .unwrap();
        game.activate_floor(floor, Vec::new());
        game.player.position = standing;
        assert!(matches!(
            game.dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new()),
            Some(TerrainDigOutcome::Failed {
                retryable: false,
                ..
            })
        ));
        assert_eq!(
            game.terrain[game.index(wall).unwrap()],
            definition.wall_terrain_id
        );
    }
    assert_eq!(signatures.len(), 8);
    assert!(lit_rooms > 0 && dark_rooms > 0);
    assert!(doors > 0);
}

#[test]
fn ordinary_room_and_anywhere_allocations_reach_pickup_and_save() {
    let mut game = Game::new_with_build(617, "demo.build.warrior").unwrap();
    let mut definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    definition.vault_id = None;
    definition.guaranteed_items.clear();
    let area = u32::from(definition.width) * u32::from(definition.height);
    // One placement from each caller, retaining the formal shared pool.
    definition.loot_allocation = Some(rfb_content::ProceduralLootAllocationDefinition {
        reference_area_tiles: area,
        room_objects: rfb_content::ProceduralNormalAllocationDefinition {
            mean: 1,
            standard_deviation: 0,
        },
        anywhere_objects: rfb_content::ProceduralNormalAllocationDefinition {
            mean: 1,
            standard_deviation: 0,
        },
    });
    game.dungeon_states
        .get_mut("demo.dungeon.warrens")
        .unwrap()
        .next_instance_ordinal = 1;
    let floor = game
        .generate_procedural_floor(&definition, Some("demo.dungeon.warrens.instance.1".into()))
        .unwrap();
    assert_eq!(floor.items.len(), 2);
    let items = floor.items.clone();
    game.activate_floor(floor, Vec::new());
    for item in items {
        assert!(
            game.content
                .item(&item.kind_id)
                .unwrap()
                .rfb_base_kind
                .is_some()
        );
        let ItemLocation::Ground(position) = item.location else {
            panic!("floor loot must be on the ground")
        };
        game.player.position = position;
        game.pick_up_item_at_player(Some(&item.id)).unwrap();
        assert_eq!(
            game.items
                .iter()
                .find(|value| value.id == item.id)
                .unwrap()
                .location,
            ItemLocation::Inventory
        );
    }
    game.reveal_current_visibility();
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn free_room_placement_uses_the_full_floor_without_overlap() {
    let template = Game::new(1);
    let definition = template
        .content
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world should exist")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .expect("Warrens depth one should exist")
        .clone();
    let geometry = definition
        .layout
        .as_ref()
        .and_then(|layout| layout.rooms.as_ref())
        .expect("Warrens should retain room geometry");
    assert_eq!(geometry.placement, ProceduralRoomPlacement::Free);

    let mut center_signatures = BTreeSet::new();
    for seed in 0..32 {
        let mut game = Game::new(seed);
        let rooms = game.generate_budgeted_rooms(&definition, geometry);
        assert_eq!(rooms.len(), 5);
        assert_eq!(rooms[0].id, "entry");
        assert_eq!(rooms[1].id, "remote");
        assert!(rooms.iter().map(GeneratedRoom::area).sum::<u32>() <= 450);

        for (index, room) in rooms.iter().enumerate() {
            assert!(room.x >= 1 && room.y >= 1);
            assert!(room.x + room.width < i32::from(definition.width));
            assert!(room.y + room.height < i32::from(definition.height));
            for other in rooms.iter().skip(index + 1) {
                assert!(
                    room.x + room.width < other.x
                        || other.x + other.width < room.x
                        || room.y + room.height < other.y
                        || other.y + other.height < room.y,
                    "free rooms must retain at least one wall tile between bounds"
                );
            }
        }
        center_signatures.insert(rooms.iter().map(GeneratedRoom::center).collect::<Vec<_>>());
    }
    assert!(center_signatures.len() >= 30);
}
