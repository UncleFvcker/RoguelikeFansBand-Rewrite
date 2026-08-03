// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

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
                ProceduralRoomShape::Cavern => {
                    panic!("test geometry should not generate cavern rooms")
                }
            }
        }
    }
    assert!(rectangles > 0);
    assert!(crosses > 0);
}

#[test]
fn free_room_placement_uses_the_full_floor_without_overlap() {
    let template = Game::new(1);
    let definition = template
        .content
        .world(WARRENS_JOURNEY_WORLD_ID)
        .expect("Warrens world should exist")
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
        carved_cells: BTreeSet::new(),
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
        carved_cells: BTreeSet::new(),
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
