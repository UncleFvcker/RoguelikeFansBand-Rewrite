// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn enter_world_map_command() -> GameCommand {
    GameCommand::EnterWorldMap {
        leave_pets: false,
        cancel_recall: false,
    }
}

fn game_with_second_town(seed: u64) -> (Game, Position) {
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");

    let town_id = "demo.town.second";
    let floor_id = "demo.floor.second-town";
    let shop_id = "demo.shop.second-general-store";
    let home_id = "demo.town-facility.second-home";

    let mut town = artifact
        .content
        .towns
        .iter()
        .find(|town| town.id == "demo.town.outpost")
        .expect("Outpost should remain available")
        .clone();
    town.id = town_id.to_owned();
    town.floor_id = floor_id.to_owned();
    town.facility_ids = vec![home_id.to_owned()];
    town.shop_ids = vec![shop_id.to_owned()];
    artifact.content.towns.push(town);

    let mut shop = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == "demo.shop.outpost-general-store")
        .expect("Outpost general store should remain available")
        .clone();
    shop.id = shop_id.to_owned();
    shop.town_id = town_id.to_owned();
    shop.owner.id = "demo.shop-owner.second-general-store".to_owned();
    shop.entrance_position = rfb_content::ContentPosition { x: 2, y: 1 };
    artifact.content.shops.push(shop);

    let mut home = artifact
        .content
        .town_facilities
        .iter()
        .find(|facility| facility.id == "demo.town-facility.outpost-home")
        .expect("Outpost Home should remain available")
        .clone();
    home.id = home_id.to_owned();
    home.town_id = town_id.to_owned();
    home.entrance_position = rfb_content::ContentPosition { x: 3, y: 1 };
    artifact.content.town_facilities.push(home);

    let world = artifact
        .content
        .worlds
        .iter_mut()
        .find(|world| world.id == DEFAULT_WORLD_ID)
        .expect("Middle-earth world should remain available");
    let mut floor = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("inline floor template should remain available")
        .clone();
    floor.id = floor_id.to_owned();
    floor.name_key = "floor-demo-second-town-name".to_owned();
    floor.lifecycle = rfb_content::FloorLifecycle::Town;
    floor.depth = 0;
    floor.width = 5;
    floor.height = 3;
    floor.entry_terrain_id = None;
    floor.available_entry_terrain_id = None;
    floor.completed_entry_terrain_id = None;
    floor.failed_entry_terrain_id = None;
    floor.abandoned_entry_terrain_id = None;
    floor.task_id = None;
    floor.inline_map = Some(rfb_content::InlineFloorMapDefinition {
        player_position: rfb_content::ContentPosition { x: 1, y: 1 },
        terrain_overrides: vec![
            rfb_content::InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.floor".to_owned(),
                positions: vec![
                    rfb_content::ContentPosition { x: 0, y: 1 },
                    rfb_content::ContentPosition { x: 1, y: 1 },
                ],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
            rfb_content::InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.general-store-entrance".to_owned(),
                positions: vec![rfb_content::ContentPosition { x: 2, y: 1 }],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
            rfb_content::InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.home-entrance".to_owned(),
                positions: vec![rfb_content::ContentPosition { x: 3, y: 1 }],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
            rfb_content::InlineTerrainOverrideDefinition {
                terrain_id: "demo.terrain.outpost-gate".to_owned(),
                positions: vec![rfb_content::ContentPosition { x: 4, y: 1 }],
                chance_percent: 100,
                otherwise_terrain_id: None,
            },
        ],
        actor_spawns: Vec::new(),
        item_spawns: Vec::new(),
        scrambled_item_pair: None,
        scrambled_item_loot_pair: None,
        loot_spawns: Vec::new(),
        monster_formation: None,
    });
    world.procedural_floors.push(floor);
    let wilderness = world
        .wilderness
        .as_mut()
        .expect("Middle-earth world should retain wilderness");
    let position = Position {
        x: i32::from(wilderness.start_position.x) + 1,
        y: i32::from(wilderness.start_position.y),
    };
    wilderness
        .locations
        .push(rfb_content::WildernessLocationDefinition::Town {
            position: rfb_content::ContentPosition {
                x: u16::try_from(position.x).unwrap(),
                y: u16::try_from(position.y).unwrap(),
            },
            map_origin: rfb_content::ContentPosition { x: 45, y: 15 },
            town_id: town_id.to_owned(),
        });

    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("second-town content should remain encodable"),
    ));
    (
        Game::from_content(seed, catalog, DEFAULT_WORLD_ID)
            .expect("second-town game should initialize"),
        position,
    )
}

fn game_with_dungeon_substitution(seed: u64) -> Game {
    Game::new_with_build(seed, "demo.build.warrior")
        .expect("substitution test game should initialize")
}

#[test]
fn p89b_substitute_selection_is_seeded_persisted_and_hashed() {
    let primary = game_with_dungeon_substitution(0);
    assert!(primary.dungeon_is_active("demo.dungeon.hideout"));
    assert!(!primary.dungeon_is_active("demo.dungeon.man-cave"));
    let failed_extra_gate = game_with_dungeon_substitution(1_528);
    assert!(failed_extra_gate.dungeon_is_active("demo.dungeon.hideout"));
    assert!(!failed_extra_gate.dungeon_is_active("demo.dungeon.man-cave"));

    let mut alternate = game_with_dungeon_substitution(1_536);
    assert!(!alternate.dungeon_is_active("demo.dungeon.hideout"));
    assert!(alternate.dungeon_is_active("demo.dungeon.man-cave"));
    let mut opposite_selection = alternate.clone();
    opposite_selection
        .dungeon_states
        .get_mut("demo.dungeon.hideout")
        .expect("Hideout state")
        .suppressed = false;
    opposite_selection
        .dungeon_states
        .get_mut("demo.dungeon.man-cave")
        .expect("Man cave state")
        .suppressed = true;
    assert_ne!(alternate.state_hash(), opposite_selection.state_hash());
    let mut suppressed_conquest = alternate.clone();
    suppressed_conquest
        .dungeon_states
        .get_mut("demo.dungeon.hideout")
        .expect("Hideout state")
        .guardian_defeated = true;
    assert_eq!(suppressed_conquest.campaign_counts().0, 0);
    assert!(suppressed_conquest.validate_loaded_state().is_err());
    alternate.advance_wilderness_generation();
    assert!(!alternate.dungeon_is_active("demo.dungeon.hideout"));
    assert!(alternate.dungeon_is_active("demo.dungeon.man-cave"));

    let payload = alternate.to_save();
    assert!(
        payload
            .dungeon_states
            .iter()
            .any(|state| { state.dungeon_id == "demo.dungeon.hideout" && state.suppressed })
    );
    let restored = Game::from_save_with_content(payload, alternate.content.clone())
        .expect("substitution state should restore");
    assert_eq!(restored.state_hash(), alternate.state_hash());
    assert!(!restored.dungeon_is_active("demo.dungeon.hideout"));
    assert!(restored.dungeon_is_active("demo.dungeon.man-cave"));
}

#[test]
fn p89c_outpost_shared_entrance_routes_only_to_the_active_dungeon() {
    for (seed, active_dungeon, active_floor, active_guardian, suppressed_guardian) in [
        (
            0,
            "demo.dungeon.hideout",
            "demo.floor.hideout-depth-8",
            "demo.actor.meng-huo-the-king-of-southerings",
            "demo.actor.untamo-the-cruel",
        ),
        (
            1_536,
            "demo.dungeon.man-cave",
            "demo.floor.man-cave-depth-8",
            "demo.actor.untamo-the-cruel",
            "demo.actor.meng-huo-the-king-of-southerings",
        ),
    ] {
        let mut game = game_with_dungeon_substitution(seed);
        let world_position = Position { x: 28, y: 52 };
        let cell = game.wilderness_cell_dto(world_position);
        assert!(
            cell.locations
                .iter()
                .any(|location| location.id == active_dungeon)
        );
        assert_eq!(
            cell.locations
                .iter()
                .filter(|location| location.id == "demo.dungeon.hideout"
                    || location.id == "demo.dungeon.man-cave")
                .count(),
            1
        );
        assert!(
            cell.locations
                .iter()
                .any(|location| location.id == "demo.town.outpost")
        );
        assert!(
            cell.locations
                .iter()
                .any(|location| location.id == "demo.dungeon.warrens")
        );
        assert!(game.actor_kind_is_dungeon_guardian(active_guardian));
        assert!(!game.actor_kind_is_dungeon_guardian(suppressed_guardian));
        assert_eq!(
            game.terrain_at(Position { x: 74, y: 16 }),
            "demo.terrain.stairs-down"
        );

        game.player.position = Position { x: 93, y: 29 };
        assert_eq!(
            game.terrain_at(game.player.position),
            "demo.terrain.hideout-entrance"
        );
        game.traverse_stairs(false)
            .expect("shared entrance should resolve")
            .expect("shared entrance should enter its active dungeon");
        assert_eq!(game.current_floor_id, active_floor);
    }
}

#[test]
fn p96c_numenor_atlantis_selection_and_shared_entrance_are_seed_stable() {
    let definitions = game_with_dungeon_substitution(0);
    let world = definitions
        .content
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world definition");
    let numenor = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.numenor")
        .expect("Numenor definition");
    let atlantis = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.atlantis")
        .expect("Atlantis definition");
    assert_eq!(
        (0..1_138)
            .filter(|seed| dungeon_substitution_uses_alternate(numenor, atlantis, *seed))
            .count(),
        569
    );

    for (seed, active_dungeon, active_floor, active_guardian, suppressed_guardian) in [
        (
            0,
            "demo.dungeon.numenor",
            "demo.floor.numenor-depth-55",
            "demo.guardian.numenor-entrance.1",
            "demo.guardian.atlantis-entrance.1",
        ),
        (
            569,
            "demo.dungeon.atlantis",
            "demo.floor.atlantis-depth-55",
            "demo.guardian.atlantis-entrance.1",
            "demo.guardian.numenor-entrance.1",
        ),
    ] {
        let mut game = game_with_dungeon_substitution(seed);
        let cell = game.wilderness_cell_dto(Position { x: 30, y: 27 });
        assert!(
            cell.locations
                .iter()
                .any(|location| location.id == active_dungeon)
        );
        assert_eq!(
            cell.locations
                .iter()
                .filter(|location| matches!(
                    location.id.as_str(),
                    "demo.dungeon.numenor" | "demo.dungeon.atlantis"
                ))
                .count(),
            1
        );

        dispatch_next(&mut game, enter_world_map_command());
        game.wilderness_position = Some(Position { x: 30, y: 27 });
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        let guardian = game
            .entities
            .iter()
            .find(|actor| actor.id == active_guardian)
            .expect("active Lesser Kraken entrance guardian");
        assert_eq!(guardian.kind_id, "demo.actor.lesser-kraken");
        assert!(
            game.entities
                .iter()
                .all(|actor| actor.id != suppressed_guardian)
        );

        place_player_on_terrain(&mut game, "demo.terrain.numenor-atlantis-entrance");
        let entered = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(entered.floor_id, active_floor);
    }
}

#[test]
fn p96d_all_numenor_atlantis_floors_generate_water_veins_and_ordinary_stairs() {
    let mut game =
        Game::new_with_build(196, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| {
            matches!(
                floor.dungeon_id.as_deref(),
                Some("demo.dungeon.numenor" | "demo.dungeon.atlantis")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| (floor.dungeon_id.clone(), floor.depth));
    assert_eq!(definitions.len(), 32);

    let mut magma_veins = 0;
    let mut quartz_veins = 0;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        assert_eq!((generated.width, generated.height), (66, 22));
        assert!(generated.terrain.iter().all(|terrain| !matches!(
            terrain.as_str(),
            "demo.terrain.shaft-up" | "demo.terrain.shaft-down"
        )));
        let terrain_count = |terrain_id: &str| {
            generated
                .terrain
                .iter()
                .filter(|terrain| terrain.as_str() == terrain_id)
                .count()
        };
        let shallow_water = terrain_count("demo.terrain.surface-water-shallow");
        let deep_water = terrain_count("demo.terrain.surface-water-deep");
        assert!(shallow_water > 0, "{} shallow water", definition.id);
        if definition.dungeon_id.as_deref() == Some("demo.dungeon.atlantis")
            || definition.id == "demo.floor.numenor-depth-65"
        {
            assert!(deep_water > 0, "{} deep water", definition.id);
        }
        if definition.id == "demo.floor.numenor-depth-70" {
            assert!(terrain_count("demo.terrain.rubble") > 0);
        }
        magma_veins += terrain_count("demo.terrain.magma-vein");
        quartz_veins += terrain_count("demo.terrain.quartz-vein");

        let up_stairs = terrain_count("demo.terrain.stairs-up");
        assert!((1..=2).contains(&up_stairs), "{} up stairs", definition.id);
        let down_stairs = terrain_count("demo.terrain.stairs-down");
        if definition.final_floor {
            assert_eq!(down_stairs, 0, "{} down stairs", definition.id);
        } else {
            assert!(
                (4..=5).contains(&down_stairs),
                "{} down stairs",
                definition.id
            );
        }
    }
    assert!(magma_veins > 0);
    assert!(quartz_veins > 0);
}

#[test]
fn p99f_giants_hall_snow_castle_selection_uses_the_original_distinct_positions() {
    let definitions = game_with_dungeon_substitution(0);
    let world = definitions
        .content
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world definition");
    let giants_hall = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.giants-hall")
        .expect("Giant's Hall definition");
    let snow_castle = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.snow-castle")
        .expect("Snow castle definition");
    let primary_seed = (0..10_000)
        .find(|seed| !dungeon_substitution_uses_alternate(giants_hall, snow_castle, *seed))
        .expect("a Giant's Hall seed should exist");
    let alternate_seed = (0..10_000)
        .find(|seed| dungeon_substitution_uses_alternate(giants_hall, snow_castle, *seed))
        .expect("a Snow castle seed should exist");

    for (seed, active_id, active_position, suppressed_position) in [
        (
            primary_seed,
            "demo.dungeon.giants-hall",
            Position { x: 63, y: 44 },
            Position { x: 65, y: 44 },
        ),
        (
            alternate_seed,
            "demo.dungeon.snow-castle",
            Position { x: 65, y: 44 },
            Position { x: 63, y: 44 },
        ),
    ] {
        let game = game_with_dungeon_substitution(seed);
        assert!(game.dungeon_is_active(active_id));
        assert!(
            game.wilderness_cell_dto(active_position)
                .locations
                .iter()
                .any(|location| location.id == active_id)
        );
        assert!(
            game.wilderness_cell_dto(suppressed_position)
                .locations
                .iter()
                .all(|location| {
                    !matches!(
                        location.id.as_str(),
                        "demo.dungeon.giants-hall" | "demo.dungeon.snow-castle"
                    )
                })
        );
    }
}

#[test]
fn p99f_all_giants_hall_and_snow_castle_floors_generate_without_doors() {
    let mut game =
        Game::new_with_build(199, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| {
            matches!(
                floor.dungeon_id.as_deref(),
                Some("demo.dungeon.giants-hall" | "demo.dungeon.snow-castle")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| (floor.dungeon_id.clone(), floor.depth));
    assert_eq!(definitions.len(), 32);

    let mut generated_water = 0;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        assert!(
            generated
                .terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.door-secret"),
            "{} should have no doors",
            definition.id
        );
        let count = |terrain_id: &str| {
            generated
                .terrain
                .iter()
                .filter(|terrain| terrain.as_str() == terrain_id)
                .count()
        };
        if definition.dungeon_id.as_deref() == Some("demo.dungeon.giants-hall") {
            assert_eq!((generated.width, generated.height), (96, 33));
            generated_water += count("demo.terrain.surface-water-deep")
                + count("demo.terrain.surface-water-shallow");
        } else {
            assert_eq!((generated.width, generated.height), (66, 22));
            assert!(count("demo.terrain.slush") > 0, "{} slush", definition.id);
            assert!(count("demo.terrain.ice-floor") > 0, "{} ice", definition.id);
            assert_eq!(generated.connections.len(), definition.connections.len());
            assert!(generated.terrain.iter().any(|terrain| {
                terrain == "demo.terrain.shaft-up" || terrain == "demo.terrain.shaft-down"
            }));
        }
    }
    assert!(generated_water > 0);
}

#[test]
fn p100f_all_graveyard_floors_generate_shallow_water_layers_and_shafts() {
    let mut game =
        Game::new_with_build(200, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.graveyard"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 21);

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        assert_eq!((generated.width, generated.height), (66, 22));
        assert_eq!(generated.connections.len(), definition.connections.len());
        assert!(
            generated
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.surface-water-shallow"),
            "{} shallow water",
            definition.id
        );
        assert_eq!(
            generated
                .terrain
                .iter()
                .filter(|terrain| {
                    matches!(
                        terrain.as_str(),
                        "demo.terrain.shaft-up" | "demo.terrain.shaft-down"
                    )
                })
                .count(),
            definition
                .connections
                .iter()
                .filter(|connection| connection.kind == rfb_content::FloorConnectionKind::Shaft)
                .count(),
            "{} shafts",
            definition.id
        );
        if definition.depth == 54 {
            assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.rubble")
            );
        }
        if definition.depth == 62 {
            assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.surface-water-deep")
            );
        }
        if definition.depth == 66 {
            assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.rubble")
            );
        }
    }
}

#[test]
fn p101d_witch_wood_and_plains_of_oz_selection_uses_distinct_original_positions() {
    let definitions = game_with_dungeon_substitution(0);
    let world = definitions
        .content
        .world(DEFAULT_WORLD_ID)
        .expect("Middle-earth world definition");
    let witch = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.witch-wood")
        .expect("Witch Wood definition");
    let plains = world
        .dungeons
        .iter()
        .find(|dungeon| dungeon.id == "demo.dungeon.plains-of-oz")
        .expect("Plains of Oz definition");
    let witch_seed = (0..10_000)
        .find(|seed| !dungeon_substitution_uses_alternate(witch, plains, *seed))
        .expect("a Witch Wood seed should exist");
    let oz_seed = (0..10_000)
        .find(|seed| dungeon_substitution_uses_alternate(witch, plains, *seed))
        .expect("a Plains of Oz seed should exist");

    for (seed, active_id, active_position, suppressed_position) in [
        (
            witch_seed,
            "demo.dungeon.witch-wood",
            Position { x: 63, y: 53 },
            Position { x: 65, y: 54 },
        ),
        (
            oz_seed,
            "demo.dungeon.plains-of-oz",
            Position { x: 65, y: 54 },
            Position { x: 63, y: 53 },
        ),
    ] {
        let game = game_with_dungeon_substitution(seed);
        assert!(game.dungeon_is_active(active_id));
        assert!(
            game.wilderness_cell_dto(active_position)
                .locations
                .iter()
                .any(|location| location.id == active_id)
        );
        assert!(
            game.wilderness_cell_dto(suppressed_position)
                .locations
                .iter()
                .all(|location| {
                    !matches!(
                        location.id.as_str(),
                        "demo.dungeon.witch-wood" | "demo.dungeon.plains-of-oz"
                    )
                })
        );
    }
}

#[test]
fn p101d_all_witch_wood_and_plains_of_oz_floors_generate_their_outdoor_layers() {
    let mut game =
        Game::new_with_build(201, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| {
            matches!(
                floor.dungeon_id.as_deref(),
                Some("demo.dungeon.witch-wood" | "demo.dungeon.plains-of-oz")
            )
        })
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| (floor.dungeon_id.clone(), floor.depth));
    assert_eq!(definitions.len(), 35);

    let mut flowers = 0;
    let mut swamps = 0;
    let mut dirt = 0;
    let mut brakes = 0;
    let mut water = 0;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        assert!(
            generated
                .terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.door-secret")
        );
        let count = |terrain_id: &str| {
            generated
                .terrain
                .iter()
                .filter(|terrain| terrain.as_str() == terrain_id)
                .count()
        };
        water +=
            count("demo.terrain.surface-water-deep") + count("demo.terrain.surface-water-shallow");
        if definition.dungeon_id.as_deref() == Some("demo.dungeon.witch-wood") {
            assert_eq!((generated.width, generated.height), (96, 33));
            assert!(definition.connections.is_empty());
            flowers += count("demo.terrain.surface-flower");
            swamps += count("demo.terrain.surface-swamp");
            assert!(count("demo.terrain.quartz-vein") > 0);
        } else {
            assert_eq!((generated.width, generated.height), (66, 22));
            dirt += count("demo.terrain.dirt");
            brakes += count("demo.terrain.surface-brake");
            assert_eq!(generated.connections.len(), definition.connections.len());
            assert_eq!(
                generated
                    .terrain
                    .iter()
                    .filter(|terrain| {
                        matches!(
                            terrain.as_str(),
                            "demo.terrain.shaft-up" | "demo.terrain.shaft-down"
                        )
                    })
                    .count(),
                definition
                    .connections
                    .iter()
                    .filter(|connection| {
                        connection.kind == rfb_content::FloorConnectionKind::Shaft
                    })
                    .count()
            );
        }
    }
    assert!(flowers > 0 && swamps > 0 && dirt > 0 && brakes > 0 && water > 0);
}

#[test]
fn p101e_guardian_reward_uses_the_players_first_realm_third_book() {
    for (build_id, expected_item_id) in [
        ("demo.build.high-mage-life", "demo.item.book-of-the-unicorn"),
        ("demo.build.high-mage-sorcery", "demo.item.pattern-sorcery"),
        ("demo.build.high-mage-nature", "demo.item.natures-gifts"),
        ("demo.build.high-mage-death", "demo.item.black-channels"),
        (
            "demo.build.high-mage-armageddon",
            "demo.item.path-of-destruction",
        ),
        ("demo.build.high-mage-arcane", "demo.item.major-arcana"),
        ("demo.build.warrior", "demo.item.book-of-the-unicorn"),
    ] {
        let mut game = Game::new_with_build(201, build_id)
            .unwrap_or_else(|error| panic!("{build_id} should create: {error}"));
        let definition = game
            .content
            .world(&game.world_id)
            .and_then(|world| {
                world
                    .procedural_floors
                    .iter()
                    .find(|floor| floor.id == "demo.floor.witch-wood-depth-40")
            })
            .cloned()
            .expect("Witch Wood final floor");
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Witch Wood final floor should generate");
        let guardian = generated
            .entities
            .iter()
            .find(|actor| actor.id == "demo.guardian.witch-wood.1")
            .cloned()
            .expect("Gertrude should guard Witch Wood");
        game.current_floor_id = definition.id;
        let (items, _) = game
            .generate_death_loot(&guardian)
            .expect("guardian reward should generate");
        assert!(
            items.iter().any(|item| item.kind_id == expected_item_id),
            "{build_id} should receive {expected_item_id}"
        );
    }
}

#[test]
fn p89d_hideout_reward_materializes_a_nonblank_am_quest_amulet() {
    let mut game =
        Game::new_with_build(89, "demo.build.warrior").expect("Hideout reward game should create");
    let reward = game
        .generate_loot_instances(
            &LootContext {
                table_id: "demo.loot-table.hideout-final-reward".to_owned(),
                floor_id: "demo.floor.hideout-depth-18".to_owned(),
                depth: 18,
                source: LootSource::FloorRoom {
                    room_id: "demo.guardian.hideout.1".to_owned(),
                    spawn_id: "demo.guardian.hideout.reward".to_owned(),
                },
            },
            ItemLocation::Ground(game.player.position),
        )
        .expect("Hideout reward should generate");

    assert_eq!(reward.len(), 1);
    let reward = &reward[0];
    assert_eq!(reward.kind_id, "demo.item.amulet");
    assert_eq!(reward.quality, ItemQualityDto::Fine);
    assert_eq!(reward.affix_ids, ["rfb-legacy.affix.amulet-am-quest"]);
    assert_eq!(reward.rolled_affixes.len(), 1);
    assert_ne!(
        reward.rolled_affixes[0].properties,
        rfb_content::AffixPropertyBundleDefinition::default()
    );
}

fn p89_reach_shared_dungeon_guardian(seed: u64, dungeon_id: &str) -> Game {
    let mut game = game_with_dungeon_substitution(seed);
    game.player.position = Position { x: 93, y: 29 };
    for depth in 8..=18 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.{dungeon_id}-depth-{depth}")
        );
        if depth < 18 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }
    game
}

fn p89_defeat_guardian(game: &mut Game, guardian_id: &str) -> (GameUpdate, Position) {
    defeat_guardian_with_status(game, guardian_id, STATUS_POISON)
}

fn defeat_guardian_with_status(
    game: &mut Game,
    guardian_id: &str,
    status_kind_id: &str,
) -> (GameUpdate, Position) {
    game.entities.retain(|actor| actor.id == guardian_id);
    let guardian = game
        .entities
        .first_mut()
        .unwrap_or_else(|| panic!("{guardian_id} should guard the final floor"));
    let position = guardian.position;
    guardian.hp = 1;
    guardian.statuses = vec![StatusInstance {
        kind_id: status_kind_id.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(game.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    (dispatch_next(game, GameCommand::Wait), position)
}

#[test]
fn p90c_troll_cave_generation_keeps_terrain_mix_lakes_shafts_and_connectivity() {
    let mut game =
        Game::new_with_build(180, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.troll-cave"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 19);
    let mut generated_mountain_walls = 0;
    let mut generated_dirt = 0;

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Troll cave floor should generate");
        let start = generated
            .connections
            .first()
            .expect("Troll cave floor should retain a connection")
            .position;
        let mut reached = BTreeSet::new();
        let mut pending = std::collections::VecDeque::from([start]);
        while let Some(position) = pending.pop_front() {
            if position.x < 0
                || position.y < 0
                || position.x >= i32::from(generated.width)
                || position.y >= i32::from(generated.height)
                || !reached.insert(position)
            {
                continue;
            }
            let index = position.y as usize * usize::from(generated.width) + position.x as usize;
            if !game
                .content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
            {
                reached.remove(&position);
                continue;
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                pending.push_back(Position {
                    x: position.x + dx,
                    y: position.y + dy,
                });
            }
        }
        assert!(
            generated
                .connections
                .iter()
                .all(|connection| reached.contains(&connection.position)),
            "depth {} connection network",
            definition.depth
        );
        assert_eq!(
            generated
                .terrain
                .iter()
                .filter(|terrain| terrain.as_str() == "demo.terrain.surface-grass")
                .count(),
            240,
            "depth {} grass budget",
            definition.depth
        );
        generated_dirt += generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.dirt")
            .count();
        generated_mountain_walls += generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.mountain-wall")
            .count();
        assert_eq!(generated.connections.len(), definition.connections.len());
        assert!(generated.terrain.iter().any(|terrain| {
            terrain == "demo.terrain.shaft-up" || terrain == "demo.terrain.shaft-down"
        }));
        if definition.layout.as_ref().is_some_and(|layout| {
            layout
                .lake
                .as_ref()
                .is_some_and(|lake| lake.deep_terrain_id == "demo.terrain.surface-water-deep")
        }) {
            assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.surface-water-deep"),
                "depth {} water lake",
                definition.depth
            );
        }
        if definition.layout.as_ref().is_some_and(|layout| {
            layout
                .lake
                .as_ref()
                .is_some_and(|lake| lake.deep_terrain_id == "demo.terrain.rubble")
        }) {
            assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.rubble"),
                "depth {} rubble lake",
                definition.depth
            );
        }
    }
    assert!(generated_mountain_walls > 0);
    assert!(generated_dirt > 240 * 19);
}

#[test]
fn p90c_troll_cave_shared_entry_shafts_conquest_and_reward_are_one_shot() {
    let mut game =
        Game::new_with_build(180, "demo.build.warrior").expect("Middle-earth should create");
    assert!(!game.dungeon_is_active("demo.dungeon.orc-cave"));
    assert!(game.dungeon_is_active("demo.dungeon.troll-cave"));
    let cell = game.wilderness_cell_dto(Position { x: 30, y: 45 });
    assert!(
        cell.locations
            .iter()
            .any(|location| location.id == "demo.dungeon.troll-cave")
    );
    assert!(
        cell.locations
            .iter()
            .all(|location| location.id != "demo.dungeon.orc-cave")
    );

    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 30, y: 45 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    place_player_on_terrain(&mut game, "demo.terrain.orc-cave-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.troll-cave-depth-18");

    let shaft_down = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.troll-cave-depth-18-shaft-down")
        .expect("depth 18 should have a down shaft")
        .position;
    game.player.position = shaft_down;
    let skipped = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(skipped.floor_id, "demo.floor.troll-cave-depth-20");
    let shaft_up = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.troll-cave-depth-20-shaft-up")
        .expect("depth 20 should have the reciprocal up shaft")
        .position;
    game.player.position = shaft_up;
    let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(returned.floor_id, "demo.floor.troll-cave-depth-18");

    for depth in 19..=36 {
        game.entities.clear();
        let connection_id = format!("demo.connection.troll-cave-depth-{}-stairs-down", depth - 1);
        game.player.position = game
            .floor_connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .unwrap_or_else(|| panic!("depth {} regular down stairs", depth - 1))
            .position;
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.troll-cave-depth-{depth}")
        );
    }

    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.troll-cave.1");
    assert_eq!(update.campaign.status, CampaignStatusDto::Active);
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert_eq!(update.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.troll-cave"].guardian_defeated);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.location == ItemLocation::Ground(guardian_position)
                    && item.kind_id == "demo.item.metal-lamellar-armour"
                    && item.affix_ids == ["rfb-legacy.affix.olog-hai"]
            })
            .count(),
        1
    );

    game.entities.clear();
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.kind_id == "demo.item.metal-lamellar-armour"
                    && item.affix_ids == ["rfb-legacy.affix.olog-hai"]
            })
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Troll cave conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.troll-cave"].guardian_defeated);
}

#[test]
fn p91c_eyrie_generation_keeps_caverns_rivers_shafts_and_connectivity() {
    let mut game =
        Game::new_with_build(191, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.eyrie"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 11);
    let mut generated_water = 0;

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Eyrie floor should generate");
        let start = generated
            .connections
            .first()
            .expect("Eyrie floor should retain a connection")
            .position;
        let mut reached = BTreeSet::new();
        let mut pending = std::collections::VecDeque::from([start]);
        while let Some(position) = pending.pop_front() {
            if position.x < 0
                || position.y < 0
                || position.x >= i32::from(generated.width)
                || position.y >= i32::from(generated.height)
                || !reached.insert(position)
            {
                continue;
            }
            let index = position.y as usize * usize::from(generated.width) + position.x as usize;
            if !game
                .content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
            {
                reached.remove(&position);
                continue;
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                pending.push_back(Position {
                    x: position.x + dx,
                    y: position.y + dy,
                });
            }
        }
        assert!(
            generated
                .connections
                .iter()
                .all(|connection| reached.contains(&connection.position)),
            "depth {} connection network",
            definition.depth
        );
        assert_eq!(generated.connections.len(), definition.connections.len());
        assert!(generated.terrain.iter().any(|terrain| {
            terrain == "demo.terrain.shaft-up" || terrain == "demo.terrain.shaft-down"
        }));
        assert!(
            generated
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.surface-grass")
        );
        assert!(
            generated
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.mountain-wall")
        );
        assert!(
            generated
                .terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.door-secret")
        );
        generated_water += generated
            .terrain
            .iter()
            .filter(|terrain| {
                terrain.as_str() == "demo.terrain.surface-water-deep"
                    || terrain.as_str() == "demo.terrain.surface-water-shallow"
            })
            .count();
    }
    assert!(generated_water > 0);
}

#[test]
fn p91c_eyrie_guardians_shafts_conquest_and_new_life_reward_are_one_shot() {
    let mut game =
        Game::new_with_build(191, "demo.build.warrior").expect("Middle-earth should create");
    assert!(
        game.entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.eyrie-entrance.1")
    );
    let initial_hash = game.state_hash();
    game = Game::from_save(game.to_save()).expect("distant Eyrie guardian state should restore");
    assert_eq!(game.state_hash(), initial_hash);
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 76, y: 46 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.eyrie-entrance.1")
        .expect("Jubjub bird should guard the Eyrie entrance");
    assert_eq!(entrance.kind_id, "demo.actor.jubjub-bird");
    assert!(entrance.pack.as_ref().is_some_and(|pack| {
        pack.behavior == MonsterPackBehaviorDto::GuardPosition && pack.leader_id == entrance.id
    }));
    assert_eq!(entrance.position, Position { x: 47, y: 16 });
    assert_eq!(
        game.terrain_at(entrance.position),
        "demo.terrain.surface-path"
    );
    let guarded_hash = game.state_hash();
    game = Game::from_save(game.to_save()).expect("visible Eyrie guardian should restore");
    assert_eq!(game.state_hash(), guarded_hash);

    let (entrance_update, _) = p89_defeat_guardian(&mut game, "demo.guardian.eyrie-entrance.1");
    clear_monsters(&mut game);
    assert!(game.dungeon_states["demo.dungeon.eyrie"].entrance_guardian_defeated);
    assert!(!game.dungeon_states["demo.dungeon.eyrie"].guardian_defeated);
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert_eq!(entrance_update.campaign.score, 0);
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert!(
        game.entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.eyrie-entrance.1")
    );

    place_player_on_terrain(&mut game, "demo.terrain.eyrie-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.eyrie-depth-40");

    let shaft_down = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.eyrie-depth-40-shaft-down")
        .expect("depth 40 should have a down shaft")
        .position;
    game.player.position = shaft_down;
    let skipped = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(skipped.floor_id, "demo.floor.eyrie-depth-42");
    let shaft_up = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.eyrie-depth-42-shaft-up")
        .expect("depth 42 should have the reciprocal up shaft")
        .position;
    game.player.position = shaft_up;
    let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(returned.floor_id, "demo.floor.eyrie-depth-40");

    for depth in 41..=50 {
        clear_monsters(&mut game);
        let connection_id = format!("demo.connection.eyrie-depth-{}-stairs-down", depth - 1);
        game.player.position = game
            .floor_connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .unwrap_or_else(|| panic!("depth {} regular down stairs", depth - 1))
            .position;
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.eyrie-depth-{depth}"));
    }

    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.eyrie.1");
    assert_eq!(update.campaign.status, CampaignStatusDto::Active);
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert_eq!(update.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.eyrie"].guardian_defeated);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.location == ItemLocation::Ground(guardian_position)
                    && item.kind_id == "demo.item.new-life-potion"
                    && item.affix_ids.is_empty()
            })
            .count(),
        1
    );

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.new-life-potion")
            .count(),
        1
    );
    for (floor_id, floor) in &game.stored_floors {
        for item in &floor.items {
            match item.location {
                ItemLocation::CarriedBy { ref actor_id } => assert!(
                    floor.entities.iter().any(|actor| actor.id == *actor_id),
                    "{floor_id} item {} has missing carrier {actor_id}",
                    item.id
                ),
                ItemLocation::Ground(position) => {
                    let index =
                        position.y as usize * usize::from(floor.width) + position.x as usize;
                    assert!(
                        game.content
                            .terrain(&floor.terrain[index])
                            .is_some_and(|terrain| terrain.walkable),
                        "{floor_id} item {} is on {}",
                        item.id,
                        floor.terrain[index]
                    );
                }
                _ => panic!("{floor_id} item {} has a non-floor location", item.id),
            }
        }
    }
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Eyrie conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.eyrie"].entrance_guardian_defeated);
    assert!(restored.dungeon_states["demo.dungeon.eyrie"].guardian_defeated);
}

#[test]
fn p92c_labyrinth_generation_keeps_the_small_perfect_maze_connected() {
    let mut game =
        Game::new_with_build(192, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.labyrinth"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 9);

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Labyrinth floor should generate");
        assert_eq!((generated.width, generated.height), (66, 22));
        let mut reached = BTreeSet::new();
        let mut pending = std::collections::VecDeque::from([generated.player_position]);
        while let Some(position) = pending.pop_front() {
            if position.x < 0
                || position.y < 0
                || position.x >= i32::from(generated.width)
                || position.y >= i32::from(generated.height)
                || !reached.insert(position)
            {
                continue;
            }
            let index = position.y as usize * usize::from(generated.width) + position.x as usize;
            if !game
                .content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
            {
                reached.remove(&position);
                continue;
            }
            for (dx, dy) in [(0, -1), (1, 0), (0, 1), (-1, 0)] {
                pending.push_back(Position {
                    x: position.x + dx,
                    y: position.y + dy,
                });
            }
        }
        let walkable = generated
            .terrain
            .iter()
            .filter(|terrain_id| {
                game.content
                    .terrain(terrain_id)
                    .is_some_and(|terrain| terrain.walkable)
            })
            .count();
        assert_eq!(walkable, 557, "depth {} maze floor tiles", definition.depth);
        assert_eq!(
            reached.len(),
            walkable,
            "depth {} connectivity",
            definition.depth
        );
        assert!(
            generated
                .entities
                .iter()
                .all(|actor| reached.contains(&actor.position))
        );
        assert!(
            generated
                .terrain
                .iter()
                .all(|terrain| terrain != "demo.terrain.door-secret")
        );
        let up_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
            .count();
        assert!((1..=2).contains(&up_stairs));
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.final_floor {
            assert_eq!(down_stairs, 0);
            assert!(generated.entities.iter().any(|actor| {
                actor.id == "demo.guardian.labyrinth.1"
                    && actor.kind_id == "demo.actor.the-minotaur-of-the-labyrinth"
            }));
        } else {
            assert!((4..=5).contains(&down_stairs));
        }
    }
}

#[test]
fn p92c_labyrinth_forgets_after_movement_and_drops_the_fixed_recall_rod() {
    let mut game =
        Game::new_with_build(192, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 5, y: 48 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    place_player_on_terrain(&mut game, "demo.terrain.labyrinth-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.labyrinth-depth-20");
    clear_monsters(&mut game);

    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let open_direction = directions
        .into_iter()
        .find(|direction| game.is_walkable(game.position_in_direction(*direction)))
        .expect("maze entry should have a walkable neighbor");
    game.explored.fill(true);
    let moved = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: open_direction,
        },
    );
    assert!(
        !moved
            .events
            .iter()
            .any(|event| event.kind == "movement.blocked")
    );
    assert!(!game.explored[0]);
    assert!(game.explored.iter().any(|explored| *explored));
    assert!(game.explored.iter().any(|explored| !*explored));
    assert!(game.revealed_terrain.is_empty());

    let blocked_direction = directions
        .into_iter()
        .find(|direction| !game.is_walkable(game.position_in_direction(*direction)))
        .expect("maze corridor should have an adjacent wall");
    game.explored[0] = true;
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: blocked_direction,
        },
    );
    assert!(game.explored[0], "blocked movement must retain memory");

    for depth in 21..=28 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.labyrinth-depth-{depth}")
        );
    }
    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.labyrinth.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.labyrinth"].guardian_defeated);
    let reward = game
        .items
        .iter()
        .find(|item| {
            item.location == ItemLocation::Ground(guardian_position)
                && item.kind_id == "demo.item.recall-rod"
        })
        .expect("the Minotaur should drop the recall rod");
    assert!(reward.affix_ids.is_empty());
    assert_eq!(
        reward
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.recall")
    );
    assert_eq!(
        reward.activation.as_ref().map(|activation| activation.cost),
        Some(15)
    );
    let charges = reward.charges.expect("recall rod should retain charges");
    assert_eq!(charges.maximum, 40);
    assert!((15..=40).contains(&charges.current));
}

#[test]
fn p93c_lonely_mountain_generation_keeps_lava_caverns_lakes_and_destruction() {
    let mut game =
        Game::new_with_build(193, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.lonely-mountain"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 11);

    let mut generated_lava = 0;
    let mut generated_tree_lake = false;
    let mut generated_rubble = false;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Lonely Mountain floor should generate");
        assert_eq!((generated.width, generated.height), (96, 33));
        assert!(generated.entities.iter().all(|actor| {
            let index = actor.position.y as usize * usize::from(generated.width)
                + actor.position.x as usize;
            game.content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
        }));
        generated_lava += generated
            .terrain
            .iter()
            .filter(|terrain| {
                matches!(
                    terrain.as_str(),
                    "demo.terrain.surface-lava-deep" | "demo.terrain.surface-lava-shallow"
                )
            })
            .count();
        generated_tree_lake |= generated
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.surface-tree");
        generated_rubble |= generated
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.rubble");
        let up_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
            .count();
        assert!((1..=2).contains(&up_stairs));
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.final_floor {
            assert_eq!(down_stairs, 0);
            assert!(generated.entities.iter().any(|actor| {
                actor.id == "demo.guardian.lonely-mountain.1"
                    && actor.kind_id == "demo.actor.smaug-the-golden"
            }));
        } else {
            assert!((4..=5).contains(&down_stairs));
        }
    }
    assert!(generated_lava > 0);
    assert!(generated_tree_lake);
    assert!(generated_rubble);
}

#[test]
fn p93c_smaug_drops_arkenstone_with_clairvoyance_and_replacement() {
    let mut game =
        Game::new_with_build(193, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 42, y: 58 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    place_player_on_terrain(&mut game, "demo.terrain.lonely-mountain-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.lonely-mountain-depth-30");

    for depth in 31..=40 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.lonely-mountain-depth-{depth}")
        );
    }
    let final_floor = game.clone();
    let (update, guardian_position) =
        p89_defeat_guardian(&mut game, "demo.guardian.lonely-mountain.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.lonely-mountain"].guardian_defeated);
    let arkenstone_index = game
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.arkenstone-of-thrain")
        .expect("Smaug should drop the Arkenstone");
    assert_eq!(
        game.items[arkenstone_index].location,
        ItemLocation::Ground(guardian_position)
    );
    assert_eq!(
        game.items[arkenstone_index].quality,
        ItemQualityDto::Ordinary
    );
    assert!(game.items[arkenstone_index].affix_ids.is_empty());
    assert!(
        game.generated_artifact_ids
            .contains("demo.item.arkenstone-of-thrain")
    );
    choose_human_talent_if_pending(&mut game);
    let hash = game.state_hash();
    let restored =
        Game::from_save(game.to_save()).expect("Lonely Mountain conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.lonely-mountain"].guardian_defeated);

    for item in &mut game.items {
        if matches!(
            item.location,
            ItemLocation::Equipped { ref slot_id } if slot_id == "light"
        ) {
            item.location = ItemLocation::Inventory;
        }
    }
    game.items[arkenstone_index].location = ItemLocation::Equipped {
        slot_id: "light".to_owned(),
    };
    assert_eq!(game.player_light_radius(), Some(3));
    assert_eq!(game.player_see_invisible_sources(), 1);
    assert_eq!(game.player_hold_life_sources(), 1);

    let item_id = game.items[arkenstone_index].id.clone();
    game.explored.fill(false);
    game.glow.fill(false);
    game.world_tick = 0;
    let activation_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < 5
        })
        .expect("an automatic device success seed should exist");
    game.rng = RfbRng::seeded(activation_seed);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id,
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(game.explored.iter().all(|explored| *explored));
    assert!(game.glow.iter().all(|glow| *glow));
    assert_eq!(game.items[arkenstone_index].charges.unwrap().current, 0);

    let mut replacement = final_floor;
    replacement
        .generated_artifact_ids
        .insert("demo.item.arkenstone-of-thrain".to_owned());
    p89_defeat_guardian(&mut replacement, "demo.guardian.lonely-mountain.1");
    assert!(
        replacement
            .items
            .iter()
            .all(|item| item.kind_id != "demo.item.arkenstone-of-thrain")
    );
    assert!(replacement.items.iter().any(|item| {
        item.location == ItemLocation::Ground(guardian_position)
            && item.kind_id == "demo.item.arkenstone"
            && item.quality == ItemQualityDto::Exceptional
    }));
}

#[test]
fn p97e_dragon_lair_generation_keeps_lava_caverns_lakes_and_guardians() {
    let mut game =
        Game::new_with_build(197, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.dragon-lair"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 13);

    let mut generated_lava = 0;
    let mut generated_tree_lake = false;
    let mut generated_rubble = false;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Dragon's Lair floor should generate");
        assert_eq!((generated.width, generated.height), (96, 33));
        assert!(generated.entities.iter().all(|actor| {
            let index = actor.position.y as usize * usize::from(generated.width)
                + actor.position.x as usize;
            game.content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
        }));
        generated_lava += generated
            .terrain
            .iter()
            .filter(|terrain| {
                matches!(
                    terrain.as_str(),
                    "demo.terrain.surface-lava-deep" | "demo.terrain.surface-lava-shallow"
                )
            })
            .count();
        generated_tree_lake |= generated
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.surface-tree");
        generated_rubble |= generated
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.rubble");
        let up_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
            .count();
        assert!((1..=2).contains(&up_stairs));
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.final_floor {
            assert_eq!(down_stairs, 0);
            assert!(generated.entities.iter().any(|actor| {
                actor.id == "demo.guardian.dragon-lair.1"
                    && actor.kind_id == "demo.actor.tiamat-celestial-dragon-of-evil"
            }));
        } else {
            assert!((4..=5).contains(&down_stairs));
        }
    }
    assert!(generated_lava > 0);
    assert!(generated_tree_lake);
    assert!(generated_rubble);
}

#[test]
fn p97e_dragon_lair_guardians_and_scale_mail_reward_are_one_shot() {
    let mut game =
        Game::new_with_build(197, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 74, y: 28 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.dragon-lair-entrance.1")
        .expect("Ancient multi-hued dragon should guard the entrance");
    assert_eq!(entrance.kind_id, "demo.actor.ancient-multi-hued-dragon");
    assert_eq!(entrance.position, Position { x: 41, y: 16 });
    assert!(entrance.pack.as_ref().is_some_and(|pack| {
        pack.behavior == MonsterPackBehaviorDto::GuardPosition && pack.leader_id == entrance.id
    }));

    let (entrance_update, _) =
        p89_defeat_guardian(&mut game, "demo.guardian.dragon-lair-entrance.1");
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.dragon-lair"].entrance_guardian_defeated);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.dragon-lair-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.dragon-lair-depth-60");

    for depth in 61..=72 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.dragon-lair-depth-{depth}")
        );
    }
    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.dragon-lair.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.dragon-lair"].guardian_defeated);
    let reward = game
        .items
        .iter()
        .find(|item| {
            item.location == ItemLocation::Ground(guardian_position)
                && item.kind_id == "demo.item.multi-hued-dragon-scale-mail"
        })
        .expect("Tiamat should drop Multi-Hued Dragon Scale Mail");
    assert_eq!(reward.quality, ItemQualityDto::Ordinary);
    assert!(reward.affix_ids.is_empty());
    assert_eq!(
        reward
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.item-activation.multi-hued-dragon-breath")
    );
    assert_eq!(
        reward.charges,
        Some(ItemChargesDto {
            current: 1,
            maximum: 1,
        })
    );

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.multi-hued-dragon-scale-mail")
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Dragon's Lair conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.dragon-lair"].entrance_guardian_defeated);
    assert!(restored.dungeon_states["demo.dungeon.dragon-lair"].guardian_defeated);
}

#[test]
fn p98c_castle_generation_keeps_rooms_stairs_and_representative_layers() {
    let mut game =
        Game::new_with_build(198, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.castle"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 26);

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Castle floor should generate");
        assert_eq!((generated.width, generated.height), (66, 22));
        assert!(generated.entities.iter().all(|actor| {
            let index = actor.position.y as usize * usize::from(generated.width)
                + actor.position.x as usize;
            game.content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
        }));
        let up_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
            .count();
        assert!((1..=2).contains(&up_stairs));
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.final_floor {
            assert_eq!(down_stairs, 0);
            assert!(generated.entities.iter().any(|actor| {
                actor.id == "demo.guardian.castle.1"
                    && actor.kind_id == "demo.actor.layzark-the-emperor"
            }));
        } else {
            assert!((4..=5).contains(&down_stairs));
        }

        match definition.depth {
            45 => {
                let open_tiles = generated
                    .terrain
                    .iter()
                    .filter(|terrain_id| {
                        game.content
                            .terrain(terrain_id)
                            .is_some_and(|terrain| terrain.walkable)
                    })
                    .count();
                assert!(open_tiles > 900, "ARENA representative should stay open");
            }
            50 => assert!(
                generated
                    .terrain
                    .iter()
                    .any(|terrain| terrain == "demo.terrain.curtain-closed")
            ),
            55 => {
                assert!(
                    generated
                        .terrain
                        .iter()
                        .any(|terrain| terrain == "demo.terrain.glass-wall")
                );
                assert!(generated.terrain.iter().all(|terrain_id| {
                    terrain_id != "demo.terrain.glass-wall"
                        || game
                            .content
                            .terrain(terrain_id)
                            .is_some_and(|terrain| !terrain.walkable && !terrain.blocks_sight)
                }));
            }
            _ => {}
        }
    }
}

#[test]
fn p98c_castle_guardians_and_conquest_are_one_shot() {
    let mut game =
        Game::new_with_build(198, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 88, y: 34 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.castle-entrance.1")
        .expect("Anti-paladin should guard the entrance");
    assert_eq!(entrance.kind_id, "demo.actor.anti-paladin");
    assert_eq!(entrance.position, Position { x: 40, y: 16 });
    assert!(entrance.pack.as_ref().is_some_and(|pack| {
        pack.behavior == MonsterPackBehaviorDto::GuardPosition && pack.leader_id == entrance.id
    }));

    let (entrance_update, _) = p89_defeat_guardian(&mut game, "demo.guardian.castle-entrance.1");
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.castle"].entrance_guardian_defeated);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.castle-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.castle-depth-40");

    for depth in 41..=65 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.castle-depth-{depth}"));
    }
    let guardian = game
        .entities
        .iter_mut()
        .find(|actor| actor.id == "demo.guardian.castle.1")
        .expect("King Arthur should guard depth 65");
    guardian.nice = true;
    guardian.energy_need = 100_000;
    game.player.hp = game.effective_player_max_hp();
    let (update, _) = p89_defeat_guardian(&mut game, "demo.guardian.castle.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.castle"].guardian_defeated);

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.id == "demo.guardian.castle.1")
            .count(),
        0
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Castle conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.castle"].entrance_guardian_defeated);
    assert!(restored.dungeon_states["demo.dungeon.castle"].guardian_defeated);
}

#[test]
fn p100f_graveyard_guardians_and_rolled_soulsword_reward_are_one_shot() {
    let mut game =
        Game::new_with_build(200, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 85, y: 19 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.graveyard-entrance.1")
        .expect("Master lich should guard the entrance");
    assert_eq!(entrance.kind_id, "demo.actor.master-lich");
    assert_eq!(entrance.position, Position { x: 39, y: 16 });

    let (entrance_update, _) = defeat_guardian_with_status(
        &mut game,
        "demo.guardian.graveyard-entrance.1",
        STATUS_BLEEDING,
    );
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.graveyard"].entrance_guardian_defeated);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.graveyard-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.graveyard-depth-50");

    for depth in 51..=70 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.graveyard-depth-{depth}")
        );
    }
    let vecna = game
        .entities
        .iter_mut()
        .find(|actor| actor.id == "demo.guardian.graveyard.1")
        .expect("Vecna should guard depth 70");
    vecna.nice = true;
    vecna.energy_need = 100_000;
    game.player.hp = game.effective_player_max_hp();
    let (update, guardian_position) =
        defeat_guardian_with_status(&mut game, "demo.guardian.graveyard.1", STATUS_BLEEDING);
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.graveyard"].guardian_defeated);
    let reward = game
        .items
        .iter()
        .find(|item| {
            item.location == ItemLocation::Ground(guardian_position)
                && item.kind_id == "demo.item.soulsword"
        })
        .expect("Vecna should drop Soulsword");
    assert_eq!(
        reward.affix_ids,
        ["rfb-legacy.affix.artifact-extra-res-or-power"]
    );
    assert_eq!(reward.rolled_affixes.len(), 1);
    assert_ne!(
        reward.rolled_affixes[0].properties,
        AffixPropertyBundleDefinition::default()
    );

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.soulsword")
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Graveyard conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.graveyard"].entrance_guardian_defeated);
    assert!(restored.dungeon_states["demo.dungeon.graveyard"].guardian_defeated);
}

#[test]
fn p94c_mine_generation_selects_dry_water_or_lava_rivers_with_rich_veins() {
    let mut saw_dry = false;
    let mut saw_water = false;
    let mut saw_lava = false;

    for seed in 0..128 {
        let mut game =
            Game::new_with_build(seed, "demo.build.warrior").expect("Middle-earth should create");
        let definition = game
            .content
            .world(&game.world_id)
            .expect("Middle-earth should remain available")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.mine-depth-75")
            .expect("Mine depth 75 should remain available")
            .clone();
        game.rng = RfbRng::seeded(seed);
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Mine floor should generate");
        assert_eq!((generated.width, generated.height), (66, 22));
        let water = generated.terrain.iter().any(|terrain| {
            matches!(
                terrain.as_str(),
                "demo.terrain.surface-water-deep" | "demo.terrain.surface-water-shallow"
            )
        });
        let lava = generated.terrain.iter().any(|terrain| {
            matches!(
                terrain.as_str(),
                "demo.terrain.surface-lava-deep" | "demo.terrain.surface-lava-shallow"
            )
        });
        assert!(!(water && lava), "one river roll must select only one type");
        saw_dry |= !water && !lava;
        saw_water |= water;
        saw_lava |= lava;
        assert!(generated.terrain.iter().any(|terrain| {
            matches!(
                terrain.as_str(),
                "demo.terrain.magma-treasure"
                    | "demo.terrain.magma-hidden-treasure"
                    | "demo.terrain.quartz-treasure"
                    | "demo.terrain.quartz-hidden-treasure"
            )
        }));
        if saw_dry && saw_water && saw_lava {
            break;
        }
    }

    assert!(saw_dry, "chanceOneIn 7 should permit a dry Mine floor");
    assert!(saw_water, "the primary Mine river should be water");
    assert!(saw_lava, "the alternate Mine river should be lava");
}

#[test]
fn p94c_mine_guardians_and_star_healing_reward_are_one_shot() {
    let mut game =
        Game::new_with_build(194, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 49, y: 23 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.mine-entrance.1")
        .expect("Elder storm giant should guard the Mine entrance");
    assert_eq!(entrance.kind_id, "demo.actor.elder-storm-giant");
    assert!(entrance.pack.as_ref().is_some_and(|pack| {
        pack.behavior == MonsterPackBehaviorDto::GuardPosition && pack.leader_id == entrance.id
    }));
    assert_eq!(entrance.position, Position { x: 46, y: 16 });

    let (entrance_update, _) = p89_defeat_guardian(&mut game, "demo.guardian.mine-entrance.1");
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.mine"].entrance_guardian_defeated);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.mine-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.mine-depth-75");

    for depth in 76..=80 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.mine-depth-{depth}"));
    }
    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.mine.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.mine"].guardian_defeated);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| {
                item.location == ItemLocation::Ground(guardian_position)
                    && item.kind_id == "demo.item.star-healing-potion"
                    && item.quality == ItemQualityDto::Ordinary
                    && item.affix_ids.is_empty()
            })
            .count(),
        1
    );

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.star-healing-potion")
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Mine conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.mine"].entrance_guardian_defeated);
    assert!(restored.dungeon_states["demo.dungeon.mine"].guardian_defeated);
}

#[test]
fn p95c_battlefield_generation_keeps_alignment_ecology_and_mixed_ground() {
    let mut game =
        Game::new_with_build(195, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.battlefield"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 21);

    let mut generated_floor = 0;
    let mut generated_dirt = 0;
    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Battlefield floor should generate");
        assert_eq!((generated.width, generated.height), (96, 33));
        assert!(generated.entities.iter().all(|actor| {
            game.content
                .actor(&actor.kind_id)
                .is_some_and(|definition| {
                    definition
                        .tags
                        .iter()
                        .any(|tag| matches!(tag.as_str(), "good" | "evil"))
                })
        }));
        generated_floor += generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.floor")
            .count();
        generated_dirt += generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.dirt")
            .count();
    }
    assert!(generated_floor > 0);
    assert!(generated_dirt > 0);
}

#[test]
fn p95c_battlefield_guardians_reward_and_no_enchant_are_one_shot() {
    let mut game =
        Game::new_with_build(195, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 75, y: 57 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.battlefield-entrance.1")
        .expect("Black wraith should guard the Battlefield entrance");
    assert_eq!(entrance.kind_id, "demo.actor.black-wraith");
    assert_eq!(entrance.position, Position { x: 45, y: 16 });

    let (entrance_update, _) = defeat_guardian_with_status(
        &mut game,
        "demo.guardian.battlefield-entrance.1",
        STATUS_BLEEDING,
    );
    assert_eq!(entrance_update.campaign.conquered_dungeons, 0);
    assert!(game.dungeon_states["demo.dungeon.battlefield"].entrance_guardian_defeated);
    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.battlefield-entrance");
    let root = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(root.floor_id, "demo.floor.battlefield-depth-30");

    let shaft = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.battlefield-depth-30-shaft-down")
        .expect("Battlefield root should have a down shaft")
        .position;
    game.player.position = shaft;
    let skipped = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(skipped.floor_id, "demo.floor.battlefield-depth-32");
    let shaft = game
        .floor_connections
        .iter()
        .find(|connection| connection.id == "demo.connection.battlefield-depth-32-shaft-up")
        .expect("Battlefield depth 32 should have an up shaft")
        .position;
    game.player.position = shaft;
    let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(returned.floor_id, "demo.floor.battlefield-depth-30");

    for depth in 31..=50 {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.battlefield-depth-{depth}")
        );
    }
    let (update, guardian_position) =
        defeat_guardian_with_status(&mut game, "demo.guardian.battlefield.1", STATUS_BLEEDING);
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states["demo.dungeon.battlefield"].guardian_defeated);
    let rune_sword_index = game
        .items
        .iter()
        .position(|item| {
            item.location == ItemLocation::Ground(guardian_position)
                && item.kind_id == "demo.item.rune-sword"
        })
        .expect("Khamul should drop the Rune Sword");
    assert_eq!(
        game.items[rune_sword_index].curse,
        Some(ItemCurseSeverityDto::Permanent)
    );
    assert_eq!(
        game.items[rune_sword_index].quality,
        ItemQualityDto::Ordinary
    );
    assert!(game.items[rune_sword_index].affix_ids.is_empty());
    choose_human_talent_if_pending(&mut game);

    game.items[rune_sword_index].location = ItemLocation::Inventory;
    let rune_sword_id = game.items[rune_sword_index].id.clone();
    let draws_before = game.rng_draw_counter();
    let outcome =
        game.enchant_item_instance(&rune_sword_id, ItemEnchantmentRequest::new(100, 100, 100));
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert_eq!(outcome.to_hit.attempts, 100);
    assert_eq!(outcome.to_damage.attempts, 100);
    assert_eq!(outcome.to_armor.attempts, 100);
    assert_eq!(outcome.to_hit.successes, 0);
    assert_eq!(outcome.to_damage.successes, 0);
    assert_eq!(outcome.to_armor.successes, 0);
    assert!(game.items[rune_sword_index].enchantments.is_empty());

    const SCROLL_ID: &str = "test.item.p95-accuracy-scroll.1";
    give_inventory_item(&mut game, SCROLL_ID, "demo.item.accuracy-scroll");
    let tick_before = game.world_tick;
    let draws_before = game.rng_draw_counter();
    let unavailable = dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: SCROLL_ID.to_owned(),
            target: Some(TargetSelection::Item {
                item_id: rune_sword_id,
            }),
        },
    );
    assert_eq!(unavailable.events[0].kind, "item.use-unavailable");
    assert_eq!(game.world_tick, tick_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
    assert!(game.items.iter().any(|item| item.id == SCROLL_ID));

    clear_monsters(&mut game);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.rune-sword")
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Battlefield conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.battlefield"].guardian_defeated);
}

#[test]
fn p89f_hideout_conquest_and_am_quest_reward_are_one_shot() {
    let mut game = p89_reach_shared_dungeon_guardian(0, "hideout");
    let (update, guardian_position) = p89_defeat_guardian(&mut game, "demo.guardian.hideout.1");
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert_eq!(update.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.hideout"].guardian_defeated);
    let rewards = game
        .items
        .iter()
        .filter(|item| {
            item.location == ItemLocation::Ground(guardian_position)
                && item.kind_id == "demo.item.amulet"
                && item.affix_ids == ["rfb-legacy.affix.amulet-am-quest"]
        })
        .count();
    assert_eq!(rewards, 1);

    game.entities.clear();
    let after_conquest = dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(after_conquest.campaign.conquered_dungeons, 1);
    assert_eq!(after_conquest.campaign.score, 10_000);
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == "demo.item.amulet"
                && item.affix_ids == ["rfb-legacy.affix.amulet-am-quest"])
            .count(),
        1
    );
    let hash = game.state_hash();
    let restored = Game::from_save(game.to_save()).expect("Hideout conquest should restore");
    assert_eq!(restored.state_hash(), hash);
}

#[test]
fn p89f_man_cave_conquest_lotharang_activation_and_replacement_are_one_shot() {
    let final_floor = p89_reach_shared_dungeon_guardian(1_536, "man-cave");

    let mut conquered = final_floor.clone();
    let (update, guardian_position) =
        p89_defeat_guardian(&mut conquered, "demo.guardian.man-cave.1");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert_eq!(update.campaign.status, CampaignStatusDto::Active);
    assert_eq!(update.campaign.conquered_dungeons, 1);
    assert_eq!(update.campaign.score, 10_000);
    assert!(conquered.dungeon_states["demo.dungeon.man-cave"].guardian_defeated);
    let lotharang_index = conquered
        .items
        .iter()
        .position(|item| item.kind_id == "demo.item.lotharang")
        .expect("Untamo should drop Lotharang");
    assert_eq!(
        conquered.items[lotharang_index].location,
        ItemLocation::Ground(guardian_position)
    );
    assert_eq!(
        conquered.items[lotharang_index].quality,
        ItemQualityDto::Ordinary
    );
    assert!(conquered.items[lotharang_index].affix_ids.is_empty());
    assert!(
        conquered
            .generated_artifact_ids
            .contains("demo.item.lotharang")
    );
    let hash = conquered.state_hash();
    let restored = Game::from_save(conquered.to_save()).expect("Man cave conquest should restore");
    assert_eq!(restored.state_hash(), hash);
    assert!(restored.dungeon_states["demo.dungeon.man-cave"].guardian_defeated);
    assert_eq!(
        restored
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.lotharang")
            .count(),
        1
    );

    let item_id = conquered.items[lotharang_index].id.clone();
    for item in &mut conquered.items {
        if matches!(
            item.location,
            ItemLocation::Equipped { ref slot_id } if slot_id == "right-hand"
        ) {
            item.location = ItemLocation::Inventory;
        }
    }
    conquered.items[lotharang_index].location = ItemLocation::Equipped {
        slot_id: "right-hand".to_owned(),
    };
    assert_eq!(
        conquered.items[lotharang_index]
            .activation
            .as_ref()
            .expect("Lotharang should carry its activation")
            .device_check_difficulty,
        10
    );
    let max_hp = conquered.player_derived_stats().max_hp.value;
    conquered.player.hp = (max_hp - 30).max(1);
    let hp_before = conquered.player.hp;
    conquered.world_tick = 0;
    let activation_seed = (0..1_000)
        .find(|seed| {
            let mut rng = RfbRng::seeded(*seed);
            rng.bounded(100) < 5
        })
        .expect("an automatic device success seed should exist");
    conquered.rng = RfbRng::seeded(activation_seed);
    let activated = dispatch_next(
        &mut conquered,
        GameCommand::UseItem {
            item_id: item_id.clone(),
            target: Some(TargetSelection::SelfTarget),
        },
    );
    assert!(
        activated
            .events
            .iter()
            .any(|event| event.kind == "item.use-heal")
    );
    assert_eq!(conquered.player.hp, (hp_before + 30).min(max_hp));
    assert_eq!(conquered.items[lotharang_index].charges.unwrap().current, 0);
    for _ in 0..4 {
        if conquered.items[lotharang_index].charges.unwrap().current == 1 {
            break;
        }
        dispatch_next(&mut conquered, GameCommand::Wait);
    }
    assert_eq!(conquered.items[lotharang_index].charges.unwrap().current, 1);

    let mut replacement = final_floor;
    replacement
        .generated_artifact_ids
        .insert("demo.item.lotharang".to_owned());
    p89_defeat_guardian(&mut replacement, "demo.guardian.man-cave.1");
    assert!(
        replacement
            .items
            .iter()
            .all(|item| item.kind_id != "demo.item.lotharang")
    );
    let fallback = replacement
        .items
        .iter()
        .find(|item| {
            item.kind_id == "demo.item.battle-axe"
                && item.quality == ItemQualityDto::Exceptional
                && item.affix_ids == ["rfb-legacy.affix.slaying"]
        })
        .expect("an already-generated Lotharang should use the artifact fallback reward");
    assert_eq!(fallback.location, ItemLocation::Ground(guardian_position));
}

#[test]
fn middle_earth_starts_on_an_outdoor_surface_with_a_working_warrens_entrance() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Middle-earth should create");

    assert_eq!(game.world_id, DEFAULT_WORLD_ID);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!((game.width, game.height), (96, 33));
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
        "demo.terrain.surface-grass"
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
fn dungeon_round_trip_restores_the_scrolled_town_position() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let world_position = game
        .wilderness_position
        .expect("Warrens journey should start in the wilderness");
    game.player.position = Position { x: 63, y: 16 };
    let target = Position { x: 64, y: 16 };
    let target_index = game.index(target).expect("scroll target should exist");
    game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
    let transition = game
        .scroll_wilderness_for_player_entry(target, &mut Vec::new())
        .expect("eastward town scroll should resolve");
    let wilderness::WildernessPlayerEntry::Local { target, .. } = transition else {
        panic!("town scroll should remain on the local surface");
    };
    game.relocate_player(target, &mut BTreeSet::new());
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });

    let entrance = Position { x: 42, y: 16 };
    assert_eq!(game.terrain_at(entrance), "demo.terrain.stairs-down");
    game.player.position = entrance;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    let mut game = Game::from_save(game.to_save()).expect("scrolled dungeon state should reload");
    assert_eq!(game.wilderness_position, Some(world_position));
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });

    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(world_position));
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    assert_eq!(game.player.position, entrance);
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.outpost")
    );
}

#[test]
fn thieves_hideout_inline_floor_preserves_the_fixed_map_and_six_member_formation() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.thieves-hideout")
        .expect("thieves' hideout should remain available")
        .clone();
    game.rng = RfbRng::seeded(42);
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
            "#.^^#...............#",
            "#.^.+...............#",
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
fn trouble_at_home_inline_floor_preserves_map_spawns_and_two_item_scramble() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.trouble-at-home")
        .expect("Trouble at Home should remain available")
        .clone();
    game.rng = RfbRng::seeded(42);
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("fixed Trouble at Home floor should generate");

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
                    other => panic!("unexpected Trouble at Home terrain {other}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "######################################",
            "#............#....#.......#..........#",
            "#............+....+.......#..........#",
            "#............#....#########..........#",
            "##############....#..................#",
            "#............#....#############++++###",
            "#............+.......................#",
            "#............#.......................#",
            "##############.....##.....##.....##..#",
            "#............#.....##.....##.....##..#",
            "#............+.......................#",
            "#............#.......................#",
            "##############.....##.....##.....##..#",
            "#............#.....##.....##.....##..#",
            "#............+.......................#",
            "#............#...........<...........#",
            "######################################",
        ]
    );
    assert_eq!(floor.player_position, Position { x: 25, y: 15 });
    assert_eq!(floor.entities.len(), 13);

    let fixed_actors = floor
        .entities
        .iter()
        .filter(|entity| entity.id != "demo.floor.trouble-at-home.formation.1")
        .map(|entity| (entity.kind_id.as_str(), entity.position))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fixed_actors,
        [
            (
                "demo.actor.mean-looking-mercenary",
                Position { x: 21, y: 8 }
            ),
            (
                "demo.actor.mean-looking-mercenary",
                Position { x: 28, y: 8 }
            ),
            (
                "demo.actor.mean-looking-mercenary",
                Position { x: 35, y: 8 }
            ),
            (
                "demo.actor.mean-looking-mercenary",
                Position { x: 28, y: 12 }
            ),
            (
                "demo.actor.mean-looking-mercenary",
                Position { x: 35, y: 12 }
            ),
            ("demo.actor.singing-happy-drunk", Position { x: 3, y: 2 }),
            ("demo.actor.singing-happy-drunk", Position { x: 3, y: 6 }),
            ("demo.actor.singing-happy-drunk", Position { x: 21, y: 9 }),
            ("demo.actor.singing-happy-drunk", Position { x: 35, y: 9 }),
            ("demo.actor.singing-happy-drunk", Position { x: 25, y: 12 }),
            ("demo.actor.singing-happy-drunk", Position { x: 32, y: 12 }),
            ("demo.actor.singing-happy-drunk", Position { x: 28, y: 13 }),
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        floor
            .entities
            .iter()
            .find(|entity| entity.id == "demo.floor.trouble-at-home.formation.1")
            .expect("the random monster should be generated")
            .position,
        Position { x: 6, y: 10 }
    );

    let fixed_waybread = floor
        .items
        .iter()
        .filter(|item| item.id.starts_with("demo.item.trouble-at-home.waybread."))
        .filter_map(|item| match item.location {
            ItemLocation::Ground(position) => Some(position),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        fixed_waybread,
        BTreeSet::from([
            Position { x: 23, y: 1 },
            Position { x: 24, y: 1 },
            Position { x: 23, y: 2 },
            Position { x: 24, y: 2 },
        ])
    );

    let mut scramble_only = definition.clone();
    let inline_map = scramble_only
        .inline_map
        .as_mut()
        .expect("Trouble at Home should retain its inline map");
    inline_map.actor_spawns.clear();
    inline_map.monster_formation = None;
    inline_map.loot_spawns.clear();
    let mut mappings = BTreeSet::new();
    for seed in 0..64 {
        game.rng = RfbRng::seeded(seed);
        let generated = game
            .generate_procedural_floor(&scramble_only, None)
            .expect("isolated item scramble should generate");
        assert_eq!(game.rng.draw_counter, 1);
        let position = |id: &str| {
            generated
                .items
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| match item.location {
                    ItemLocation::Ground(position) => Some(position),
                    _ => None,
                })
                .expect("scrambled item should be on the floor")
        };
        mappings.insert((
            position("demo.item.trouble-at-home.boldness.1"),
            position("demo.item.trouble-at-home.booze.1"),
        ));
    }
    assert_eq!(
        mappings,
        BTreeSet::from([
            (Position { x: 25, y: 1 }, Position { x: 25, y: 2 }),
            (Position { x: 25, y: 2 }, Position { x: 25, y: 1 }),
        ])
    );
}

#[test]
fn crows_nest_inline_floor_preserves_map_birds_and_group_scramble() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.crows-nest")
        .expect("Crow's Nest should remain available")
        .clone();
    game.rng = RfbRng::seeded(42);
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("fixed Crow's Nest floor should generate");

    let rows = floor
        .terrain
        .chunks(usize::from(floor.width))
        .map(|row| {
            row.iter()
                .map(|terrain_id| match terrain_id.as_str() {
                    "demo.terrain.permanent-wall" => '#',
                    "demo.terrain.floor" => '.',
                    "demo.terrain.dirt" => ',',
                    "demo.terrain.stairs-up" => '<',
                    other => panic!("unexpected Crow's Nest terrain {other}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "######################################",
            "#####,,,,,,...............############",
            "###,,.,.,.,,...............###########",
            "##.,,,..,,.,,..............###########",
            "##..,.,,..,.,.............############",
            "##..,,..,,.,,..........###############",
            "##...,,,..,,,..........###############",
            "###...,,,,,,.......##...#####...######",
            "#####............######..###.....#####",
            "########################..#..###..####",
            "#########################...#####..###",
            "##########################..####..####",
            "##########################.######..###",
            "#.....#####..##.#..###.....###.......#",
            "#.<....................#######.......#",
            "#.....#######..####...########.......#",
            "######################################",
        ]
    );
    assert_eq!(floor.player_position, Position { x: 2, y: 14 });
    assert_eq!(floor.entities.len(), 9);
    assert_eq!(
        floor
            .entities
            .iter()
            .map(|entity| entity.kind_id.as_str())
            .fold(BTreeMap::new(), |mut counts, id| {
                *counts.entry(id).or_insert(0) += 1;
                counts
            }),
        BTreeMap::from([
            ("demo.actor.carrion", 1),
            ("demo.actor.crow", 6),
            ("demo.actor.crow-of-durthang", 2),
        ])
    );
    assert_eq!(
        floor
            .items
            .iter()
            .filter(|item| item.kind_id == "demo.item.human-skeleton")
            .count(),
        15
    );

    let mut scramble_only = definition.clone();
    let inline_map = scramble_only
        .inline_map
        .as_mut()
        .expect("Crow's Nest should retain its inline map");
    inline_map.actor_spawns.clear();
    inline_map.item_spawns.clear();
    let mut mappings = BTreeSet::new();
    for seed in 0..64 {
        game.rng = RfbRng::seeded(seed);
        let generated = game
            .generate_procedural_floor(&scramble_only, None)
            .expect("isolated item/loot scramble should generate");
        let positions = generated
            .items
            .iter()
            .filter(|item| {
                item.id
                    .starts_with("demo.item.crows-nest.human-skeleton.scrambled.")
            })
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some(position),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(positions.len(), 10);
        mappings.insert(positions);
    }
    assert_eq!(
        mappings,
        BTreeSet::from([
            BTreeSet::from([
                Position { x: 11, y: 1 },
                Position { x: 7, y: 3 },
                Position { x: 10, y: 3 },
                Position { x: 5, y: 4 },
                Position { x: 9, y: 4 },
                Position { x: 6, y: 5 },
                Position { x: 31, y: 13 },
                Position { x: 32, y: 14 },
                Position { x: 33, y: 15 },
                Position { x: 35, y: 15 },
            ]),
            BTreeSet::from([
                Position { x: 9, y: 2 },
                Position { x: 6, y: 3 },
                Position { x: 11, y: 4 },
                Position { x: 7, y: 5 },
                Position { x: 10, y: 5 },
                Position { x: 4, y: 6 },
                Position { x: 34, y: 13 },
                Position { x: 30, y: 14 },
                Position { x: 35, y: 14 },
                Position { x: 31, y: 15 },
            ]),
        ])
    );
}

#[test]
fn old_man_willow_inline_floor_preserves_the_original_grove_and_formation() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.old-man-willow")
        .expect("Old Man Willow's grove should remain available")
        .clone();
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("fixed Old Man Willow floor should generate");

    let rows = floor
        .terrain
        .chunks(usize::from(floor.width))
        .map(|row| {
            row.iter()
                .map(|terrain_id| match terrain_id.as_str() {
                    "demo.terrain.permanent-wall" => '#',
                    "demo.terrain.surface-grass" => '.',
                    "demo.terrain.surface-tree" => 'T',
                    "demo.terrain.stairs-up" => '<',
                    other => panic!("unexpected Old Man Willow terrain {other}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "###############################",
            "#TTTTTTTTTTTTTTTT.............#",
            "#T............TTT.TT.TTTTTTTT.#",
            "#T............TTT...........T.#",
            "#T............TTTTTTTTTTTTT.T.#",
            "#T............TTT.........T.T.#",
            "#T............TTT.TTTTTTT.T.T.#",
            "#T............TTT.......T.T.T.#",
            "#T.............TTTTTTTT.T.T.T.#",
            "#T....................T.T.T.T.#",
            "#TTTTTTTTTTTTTTTTTTTT.T.T.T.T.#",
            "#TTTTTTTTTTTTTTTTTTTT...T...T.#",
            "#TTTTTTTTTTTTTTTTTTTTTTTTTTTT.#",
            "#.............................#",
            "#.TTTTTTTTTTTTTTTTTTTTTTTTTTTT#",
            "#.............................#",
            "#TT.TTTTTTTTTTTTTTTTTTTTTTTTT.#",
            "#.............................#",
            "#<TTTTTTTTTTTTTTTTTTTTTTTTTTTT#",
            "###############################",
        ]
    );
    assert_eq!(floor.player_position, Position { x: 1, y: 18 });
    assert_eq!(floor.entities.len(), 23);
    assert_eq!(
        floor
            .entities
            .iter()
            .map(|entity| (entity.kind_id.as_str(), entity.position))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("demo.actor.old-man-willow", Position { x: 7, y: 5 }),
            ("demo.actor.huorn", Position { x: 20, y: 2 }),
            ("demo.actor.huorn", Position { x: 3, y: 3 }),
            ("demo.actor.huorn", Position { x: 8, y: 3 }),
            ("demo.actor.huorn", Position { x: 12, y: 5 }),
            ("demo.actor.huorn", Position { x: 4, y: 6 }),
            ("demo.actor.huorn", Position { x: 9, y: 6 }),
            ("demo.actor.huorn", Position { x: 14, y: 8 }),
            ("demo.actor.huorn", Position { x: 3, y: 16 }),
            ("demo.actor.sasquatch", Position { x: 11, y: 2 }),
            ("demo.actor.sasquatch", Position { x: 17, y: 3 }),
            ("demo.actor.sasquatch", Position { x: 3, y: 8 }),
            ("demo.actor.sasquatch", Position { x: 8, y: 8 }),
            ("demo.actor.sasquatch", Position { x: 11, y: 8 }),
            ("demo.actor.sasquatch", Position { x: 6, y: 9 }),
            ("demo.actor.vorpal-bunny", Position { x: 26, y: 3 }),
            ("demo.actor.vorpal-bunny", Position { x: 24, y: 5 }),
            ("demo.actor.vorpal-bunny", Position { x: 22, y: 7 }),
            ("demo.actor.vorpal-bunny", Position { x: 1, y: 13 }),
            ("demo.actor.vorpal-bunny", Position { x: 1, y: 14 }),
            ("demo.actor.vorpal-bunny", Position { x: 1, y: 15 }),
            ("demo.actor.sabre-tooth-tiger", Position { x: 28, y: 13 }),
            ("demo.actor.sabre-tooth-tiger", Position { x: 25, y: 15 }),
        ])
    );
}

#[test]
fn vapor_quest_inline_floor_preserves_the_original_cellar_formation_and_jewelry() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.vapor-quest")
        .expect("Vapor Quest cellar should remain available")
        .clone();
    let floor = game
        .generate_procedural_floor(&definition, None)
        .expect("fixed Vapor Quest floor should generate");

    let rows = floor
        .terrain
        .chunks(usize::from(floor.width))
        .map(|row| {
            row.iter()
                .map(|terrain_id| match terrain_id.as_str() {
                    "demo.terrain.permanent-wall" => '#',
                    "demo.terrain.floor" => '.',
                    "demo.terrain.stairs-up" => '<',
                    other => panic!("unexpected Vapor Quest terrain {other}"),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "#########################",
            "############.############",
            "###########...###########",
            "#########.......#########",
            "###########...###########",
            "############.############",
            "###########...###########",
            "########.........########",
            "#######...........#######",
            "######.............######",
            "#######...........#######",
            "##...#.............#...##",
            "#....##...........##....#",
            "#.......................#",
            "#....##...........##....#",
            "##...#.............#...##",
            "#######...........#######",
            "######.............######",
            "#######...........#######",
            "########.........########",
            "###########.<.###########",
            "#########################",
        ]
    );
    assert_eq!(floor.player_position, Position { x: 12, y: 20 });
    assert_eq!(floor.entities.len(), 18);
    assert_eq!(
        floor
            .entities
            .iter()
            .map(|entity| (entity.kind_id.as_str(), entity.position))
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("demo.actor.shimmering-vortex", Position { x: 12, y: 1 }),
            ("demo.actor.air-elemental", Position { x: 9, y: 3 }),
            ("demo.actor.air-elemental", Position { x: 15, y: 3 }),
            ("demo.actor.gas-spore", Position { x: 12, y: 6 }),
            ("demo.actor.radiation-eye", Position { x: 6, y: 9 }),
            ("demo.actor.radiation-eye", Position { x: 18, y: 9 }),
            ("demo.actor.air-elemental", Position { x: 4, y: 11 }),
            ("demo.actor.radiation-eye", Position { x: 6, y: 11 }),
            ("demo.actor.radiation-eye", Position { x: 18, y: 11 }),
            ("demo.actor.air-elemental", Position { x: 20, y: 11 }),
            ("demo.actor.weird-fume", Position { x: 1, y: 13 }),
            ("demo.actor.weird-fume", Position { x: 23, y: 13 }),
            ("demo.actor.air-elemental", Position { x: 4, y: 15 }),
            ("demo.actor.radiation-eye", Position { x: 6, y: 15 }),
            ("demo.actor.radiation-eye", Position { x: 18, y: 15 }),
            ("demo.actor.air-elemental", Position { x: 20, y: 15 }),
            ("demo.actor.radiation-eye", Position { x: 6, y: 17 }),
            ("demo.actor.radiation-eye", Position { x: 18, y: 17 }),
        ])
    );
    assert_eq!(
        floor
            .items
            .iter()
            .filter_map(|item| match item.location {
                ItemLocation::Ground(position) => Some((item.kind_id.as_str(), position)),
                _ => None,
            })
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([
            ("demo.item.amulet", Position { x: 2, y: 11 }),
            ("demo.item.amulet", Position { x: 3, y: 11 }),
            ("demo.item.amulet", Position { x: 1, y: 12 }),
            ("demo.item.amulet", Position { x: 1, y: 14 }),
            ("demo.item.amulet", Position { x: 2, y: 15 }),
            ("demo.item.amulet", Position { x: 3, y: 15 }),
            ("demo.item.ring", Position { x: 21, y: 11 }),
            ("demo.item.ring", Position { x: 22, y: 11 }),
            ("demo.item.ring", Position { x: 23, y: 12 }),
            ("demo.item.ring", Position { x: 23, y: 14 }),
            ("demo.item.ring", Position { x: 21, y: 15 }),
            ("demo.item.ring", Position { x: 22, y: 15 }),
        ])
    );
}

#[test]
fn warrens_surface_reentry_starts_a_fresh_expedition_with_new_monsters() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");

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
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
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
fn p87c_tidal_cave_room_water_and_optional_river_use_existing_terrain() {
    let mut saw_dry_floor = false;
    let mut saw_river = false;

    for seed in 0..64 {
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Tidal Cave generation proof should create");
        let definition = game
            .content
            .world(&game.world_id)
            .expect("Middle-earth should remain available")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.tidal-cave-depth-15")
            .expect("Tidal Cave depth 15 should remain available")
            .clone();
        game.rng = RfbRng::seeded(seed);
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Tidal Cave floor should generate");
        let deep_water = generated
            .terrain
            .iter()
            .filter(|terrain_id| terrain_id.as_str() == "demo.terrain.surface-water-deep")
            .count();
        let shallow_water = generated
            .terrain
            .iter()
            .filter(|terrain_id| terrain_id.as_str() == "demo.terrain.surface-water-shallow")
            .count();

        assert!(
            !generated
                .entities
                .iter()
                .any(|actor| actor.kind_id == "demo.actor.grendel")
        );
        if deep_water == 0 {
            assert_eq!(shallow_water, 96);
            saw_dry_floor = true;
        } else {
            assert!(shallow_water > 96);
            saw_river = true;
        }
        if saw_dry_floor && saw_river {
            break;
        }
    }

    assert!(
        saw_dry_floor,
        "chanceOneIn 7 should permit a floor without a river"
    );
    assert!(saw_river, "chanceOneIn 7 should permit a generated river");
}

#[test]
fn p88c_icky_cave_small_floor_uses_the_existing_grass_swamp_water_mix() {
    let mut game = Game::new_with_build(88, "demo.build.warrior")
        .expect("Icky Cave generation proof should create");
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.icky-cave-depth-10")
        .expect("Icky Cave depth 10 should remain available")
        .clone();
    game.rng = RfbRng::seeded(88);

    let generated = game
        .generate_procedural_floor(&definition, None)
        .expect("Icky Cave depth 10 should generate");
    let terrain_count = |terrain_id: &str| {
        generated
            .terrain
            .iter()
            .filter(|generated_id| generated_id.as_str() == terrain_id)
            .count()
    };
    let swamp = terrain_count("demo.terrain.surface-swamp");
    let shallow_water = terrain_count("demo.terrain.surface-water-shallow");

    assert_eq!((generated.width, generated.height), (66, 22));
    assert_eq!(swamp + shallow_water, 186);
    assert!(swamp > 0);
    assert!(shallow_water > 0);
    assert!(terrain_count("demo.terrain.surface-grass") > 0);
    assert_eq!(terrain_count("demo.terrain.surface-water-deep"), 0);
    assert!(
        !generated
            .entities
            .iter()
            .any(|actor| actor.kind_id == "demo.actor.the-icky-queen")
    );
}

#[test]
fn p88e_icky_cave_all_depths_keep_the_terrain_mix_and_stairs_reachable() {
    let mut game =
        Game::new_with_build(880, "demo.build.warrior").expect("Middle-earth should create");
    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.icky-cave"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 11);

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Icky Cave floor should generate");
        let route_terrain = generated
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
            generated_terrain_is_connected(
                &route_terrain,
                generated.width,
                generated.height,
                &game.content,
            ),
            "depth {} travel network",
            definition.depth
        );

        let terrain_count = |terrain_id: &str| {
            generated
                .terrain
                .iter()
                .filter(|generated_id| generated_id.as_str() == terrain_id)
                .count()
        };
        let swamp = terrain_count("demo.terrain.surface-swamp");
        let shallow_water = terrain_count("demo.terrain.surface-water-shallow");
        let expected_features = if definition.depth == 10 { 186 } else { 320 };
        let minimum_feature = expected_features * 3 / 8;
        let maximum_feature = expected_features * 5 / 8;
        assert_eq!(
            swamp + shallow_water,
            expected_features,
            "depth {}",
            definition.depth
        );
        assert!(
            (minimum_feature..=maximum_feature).contains(&swamp),
            "depth {}",
            definition.depth
        );
        assert!(
            (minimum_feature..=maximum_feature).contains(&shallow_water),
            "depth {}",
            definition.depth
        );
        let grass = terrain_count("demo.terrain.surface-grass");
        assert!(
            grass > swamp,
            "depth {} grass={grass} swamp={swamp} shallow={shallow_water}",
            definition.depth
        );
        assert!(
            grass > shallow_water,
            "depth {} grass={grass} swamp={swamp} shallow={shallow_water}",
            definition.depth
        );
        assert_eq!(
            (generated.width, generated.height),
            if definition.depth == 10 {
                (66, 22)
            } else {
                (96, 33)
            }
        );
        assert!(
            (1..=2).contains(
                &generated
                    .terrain
                    .iter()
                    .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
                    .count()
            )
        );
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.depth < 20 {
            assert!((4..=5).contains(&down_stairs), "depth {}", definition.depth);
            assert!(
                generated
                    .entities
                    .iter()
                    .all(|entity| entity.kind_id != "demo.actor.the-icky-queen")
            );
        } else {
            assert_eq!(down_stairs, 0);
            assert_eq!(
                generated
                    .entities
                    .iter()
                    .filter(|entity| entity.kind_id == "demo.actor.the-icky-queen")
                    .count(),
                1
            );
        }
    }
}

#[test]
fn p87e_tidal_cave_all_depths_keep_water_and_stairs_reachable() {
    let mut game =
        Game::new_with_build(87, "demo.build.warrior").expect("Middle-earth should create");
    let definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.tidal-cave"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(definitions.len(), 13);

    for definition in definitions {
        let generated = game
            .generate_procedural_floor(&definition, None)
            .expect("Tidal Cave floor should generate");
        let route_terrain = generated
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
            generated_terrain_is_connected(
                &route_terrain,
                generated.width,
                generated.height,
                &game.content,
            ),
            "depth {} travel network",
            definition.depth
        );
        assert!(
            generated
                .terrain
                .iter()
                .any(|terrain| terrain == "demo.terrain.surface-water-shallow"),
            "depth {} shallow water",
            definition.depth
        );
        assert!(
            (1..=2).contains(
                &generated
                    .terrain
                    .iter()
                    .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-up")
                    .count()
            )
        );
        let down_stairs = generated
            .terrain
            .iter()
            .filter(|terrain| terrain.as_str() == "demo.terrain.stairs-down")
            .count();
        if definition.depth < 27 {
            assert!((4..=5).contains(&down_stairs), "depth {}", definition.depth);
        } else {
            assert_eq!(down_stairs, 0);
        }
    }
}

#[test]
fn warrens_maps_are_seeded_connected_varied_and_persistent() {
    let mut generated_maps = BTreeSet::new();
    let mut walkable_masks = Vec::<Vec<bool>>::new();
    for seed in 0..16 {
        let mut proof = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens connectivity proof should create");
        let definition = proof
            .content
            .world(&proof.world_id)
            .expect("Middle-earth should remain available")
            .procedural_floors
            .iter()
            .find(|floor| floor.id == "demo.floor.warrens-depth-1")
            .expect("Warrens depth one should remain available")
            .clone();
        let generated = proof
            .generate_procedural_floor(&definition, None)
            .expect("Warrens floor should generate");
        let route_terrain = generated
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
            generated_terrain_is_connected(
                &route_terrain,
                generated.width,
                generated.height,
                &proof.content,
            ),
            "seed {seed} should generate a connected travel network"
        );

        let mut game = Game::new_with_build(seed, "demo.build.warrior")
            .expect("Warrens journey should create");
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        dispatch_next(&mut game, GameCommand::TraverseStairs);

        assert_eq!((game.width, game.height), (66, 22));
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
                    game.content
                        .terrain(terrain_id)
                        .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "vein"))
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
        let mut same_seed = Game::new_with_build(seed, "demo.build.warrior")
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
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
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
                        game.content
                            .terrain(terrain_id)
                            .is_some_and(|terrain| terrain.tags.iter().any(|tag| tag == "vein"))
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
        assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    }
    assert!(saw_scaled_allocation_above_minimum);
    assert!(saw_depth_gated_item);
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
    assert!(
        game.dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new())
            .is_none()
    );
    assert!(game.search_hidden_terrain().is_empty());

    assert_eq!(game.terrain, terrain_before);
    assert_eq!(game.revealed_terrain, revealed_before);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn digging_uses_original_soft_hard_and_permanent_resolution() {
    let mut permanent = Game::new(42);
    clear_monsters(&mut permanent);
    let position = permanent.position_in_direction(Direction::North);
    replace_terrain(&mut permanent, position, "demo.terrain.permanent-wall");
    let draws = permanent.rng_draw_counter();
    assert!(matches!(
        permanent.dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new()),
        Some(TerrainDigOutcome::Failed {
            retryable: false,
            ..
        })
    ));
    assert_eq!(permanent.rng_draw_counter(), draws);
    assert_eq!(
        permanent.terrain[permanent.index(position).expect("permanent wall index")],
        "demo.terrain.permanent-wall"
    );

    let mut hard =
        Game::new_with_build(42, "demo.build.high-mage-death").expect("High-Mage should create");
    clear_monsters(&mut hard);
    hard.items.clear();
    let position = hard.position_in_direction(Direction::North);
    replace_terrain(&mut hard, position, "demo.terrain.magma-vein");
    assert!(hard.player_derived_stats().dig_skill.value <= 10);
    let draws = hard.rng_draw_counter();
    assert!(matches!(
        hard.dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new()),
        Some(TerrainDigOutcome::Failed {
            retryable: false,
            ..
        })
    ));
    assert_eq!(hard.rng_draw_counter(), draws + 1);

    let saw_retryable_failure = (0..32).any(|seed| {
        let mut soft = Game::new_with_build(seed, "demo.build.high-mage-death")
            .expect("High-Mage should create");
        clear_monsters(&mut soft);
        soft.items.clear();
        let position = soft.position_in_direction(Direction::North);
        replace_terrain(&mut soft, position, "demo.terrain.rubble");
        matches!(
            soft.dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new()),
            Some(TerrainDigOutcome::Failed {
                retryable: true,
                ..
            })
        )
    });
    assert!(saw_retryable_failure);
}

#[test]
fn digging_ignores_ground_items_and_turns_a_blocking_monster_into_melee() {
    let mut ground_item = Game::new(42);
    clear_monsters(&mut ground_item);
    let position = ground_item.position_in_direction(Direction::North);
    replace_terrain(&mut ground_item, position, "demo.terrain.rubble");
    ground_item.items[0].location = ItemLocation::Ground(position);
    assert!(
        ground_item
            .dig_terrain(Direction::North, &mut Vec::new(), &mut BTreeSet::new())
            .is_some()
    );

    let mut blocked = Game::new(42);
    let position = blocked.position_in_direction(Direction::North);
    replace_terrain(&mut blocked, position, "demo.terrain.rubble");
    let definition = blocked
        .content
        .actor_definitions()
        .find(|definition| definition.role == ActorRole::Monster && definition.level >= 20)
        .expect("demo content should contain a level-20 monster")
        .clone();
    blocked.entities.clear();
    let mut target = actor_from_runtime_spawn(
        "test.digging-target",
        &definition.id,
        position,
        1_000_000,
        definition.speed,
        INITIAL_MONSTER_ENERGY_NEED,
        true,
    );
    target.resistances = definition_resistance_profile(&definition);
    blocked.entities.push(target);

    let interaction = blocked
        .snapshot()
        .terrain_interactions
        .into_iter()
        .find(|interaction| {
            interaction.kind == TerrainInteractionKindDto::DigTerrain
                && interaction.direction == Direction::North
        })
        .expect("blocking monster should keep the dig interaction visible");
    assert!(interaction.available);
    assert_eq!(interaction.unavailable_reason, None);

    let update = dispatch_next(
        &mut blocked,
        GameCommand::DigTerrain {
            direction: Direction::North,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| matches!(event.kind.as_str(), "combat.hit" | "combat.miss"))
    );
    assert!(
        update
            .events
            .iter()
            .all(|event| event.kind != "terrain.dig-unavailable")
    );
}

#[test]
fn warrens_location_requires_its_local_entrance_and_restores_the_outpost() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(game.player.position, entrance_position);
    assert_eq!(game.task_states, task_states);
    assert_eq!(game.shop_states, shop_states);
}

#[test]
fn world_map_projects_authoritative_wilderness_cells_and_restores_the_local_map() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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
    assert_eq!(current.locations.len(), 3);
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
    assert_eq!(
        current
            .locations
            .iter()
            .filter(|location| {
                matches!(
                    location.id.as_str(),
                    "demo.dungeon.hideout" | "demo.dungeon.man-cave"
                )
            })
            .count(),
        1
    );

    let save = game.to_save();
    assert_eq!(save.map_scale, MapScaleDto::World);
    assert_eq!(save.wilderness_position, Some(Position { x: 28, y: 52 }));
    assert_eq!(save.wilderness_view_offset, Position::default());
    assert_eq!(
        save.wilderness_seed,
        42_u64.wrapping_add(wilderness::WILDERNESS_SEED_STEP)
    );
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
    assert_eq!((left.width, left.height), (96, 33));
    assert_eq!(left.player.position, local_position);
    assert_eq!(left.changed_cells.len(), 96 * 33);
    assert_eq!(restored.world_tick, world_tick);
}

#[test]
fn world_map_movement_uses_original_time_scale_without_advancing_hidden_monsters() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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
fn entering_world_map_advances_the_wilderness_generation_and_clears_cached_terrain() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.wilderness_terrain_cache.len(), 9);
    let previous_seed = game.wilderness_seed;

    dispatch_next(&mut game, enter_world_map_command());

    assert_eq!(
        game.wilderness_seed,
        previous_seed.wrapping_add(wilderness::WILDERNESS_SEED_STEP)
    );
    assert!(game.wilderness_terrain_cache.is_empty());
}

#[test]
fn world_map_round_trip_preserves_the_visible_town_surface() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let remembered = Position { x: 10, y: 10 };
    let remembered_index = game.index(remembered).expect("town cell should exist");
    game.terrain[remembered_index] = "demo.terrain.created-trap".to_owned();
    game.explored[remembered_index] = true;
    game.revealed_terrain.insert(remembered);

    dispatch_next(&mut game, enter_world_map_command());

    let backing = &game.stored_floors["demo.floor.surface"];
    let backing_index = 10 * usize::from(backing.width) + 10;
    assert_eq!(backing.terrain[backing_index], "demo.terrain.created-trap");
    assert!(backing.explored[backing_index]);
    assert!(backing.revealed_terrain.contains(&remembered));

    dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    assert_eq!(game.terrain[remembered_index], "demo.terrain.created-trap");
    assert!(game.explored[remembered_index]);
    assert!(game.revealed_terrain.contains(&remembered));
}

#[test]
fn wilderness_daylight_drives_surface_ambient_light() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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
        let mut game = Game::new_with_build(seed, "demo.build.warrior")
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
    assert_eq!((game.width, game.height), (96, 33));
    assert_eq!(game.player.position, Position { x: 48, y: 16 });
    assert_eq!(game.wilderness_view_offset, Position::default());
    assert_eq!(game.terrain, duplicate.terrain);
    assert_eq!(game.entities, duplicate.entities);
    assert_eq!(
        game.entities
            .iter()
            .filter(|entity| {
                entity.id.contains(".surface.") && !entity.id.contains(".companion.")
            })
            .count(),
        4
    );
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
    assert_eq!(restored.wilderness_view_offset, Position::default());
    assert_eq!(restored.terrain, game.terrain);
    assert_eq!(restored.entities, game.entities);
}

#[test]
fn small_town_excludes_only_its_rectangle_from_wilderness_monsters() {
    let (mut game, town_position) = game_with_second_town(42);
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(town_position);

    dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    let wilderness_monsters = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".surface."))
        .collect::<Vec<_>>();
    assert!(!wilderness_monsters.is_empty());
    assert!(wilderness_monsters.iter().all(|entity| {
        !(45..50).contains(&entity.position.x) || !(15..18).contains(&entity.position.y)
    }));
}

#[test]
fn walking_into_the_outer_band_scrolls_and_normalizes_the_wilderness_view() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    game.player.position = Position { x: 63, y: 16 };
    let target = Position { x: 64, y: 16 };
    let target_index = game
        .index(target)
        .expect("scroll target should be in bounds");
    game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
    game.revealed_terrain.remove(&target);

    let first_scroll = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );

    assert_eq!(game.wilderness_position, Some(Position { x: 29, y: 52 }));
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.player.position, Position { x: 32, y: 16 });
    assert_eq!(
        first_scroll.map_translation,
        Some(Position { x: -32, y: 0 })
    );
    assert_eq!(first_scroll.changed_cells.len(), 96 * 33);
    assert_eq!(first_scroll.changed_visual_cells.len(), 96 * 33);

    game.player.position = Position { x: 63, y: 16 };
    let target_index = game
        .index(target)
        .expect("scroll target should remain in bounds");
    game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
    game.revealed_terrain.remove(&target);
    let second_scroll = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );

    assert_eq!(game.wilderness_position, Some(Position { x: 30, y: 52 }));
    assert_eq!(game.wilderness_view_offset, Position { x: -1, y: 0 });
    assert_eq!(game.player.position, Position { x: 32, y: 16 });
    assert_eq!(second_scroll.changed_cells.len(), 96 * 33);
    assert_eq!(game.stored_floors.len(), 1);
}

#[test]
fn wilderness_scroll_translates_overlap_and_crops_entities_items_gold_and_packs() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    let mut actor = game
        .entities
        .first()
        .expect("local wilderness should contain an actor template")
        .clone();
    game.entities.clear();
    actor.pack = None;
    actor.controller_id = None;
    actor.summon = None;
    let mut retained = actor.clone();
    retained.id = "test.scroll.retained".to_owned();
    retained.position = Position { x: 70, y: 16 };
    let mut dropped = actor.clone();
    dropped.id = "test.scroll.dropped".to_owned();
    dropped.position = Position { x: 10, y: 16 };
    let mut mount = actor.clone();
    mount.id = "test.scroll.mount".to_owned();
    mount.position = Position { x: 63, y: 16 };
    mount.controller_id = Some(game.player.id.clone());
    game.riding_actor_id = Some(mount.id.clone());
    let pack_id = "test.scroll.pack".to_owned();
    let leader_id = "test.scroll.pack-leader".to_owned();
    let mut pack_leader = actor.clone();
    pack_leader.id = leader_id.clone();
    pack_leader.position = Position { x: 70, y: 15 };
    pack_leader.pack = Some(MonsterPackIdentity {
        id: pack_id.clone(),
        leader_id: leader_id.clone(),
        role: MonsterPackRoleDto::Leader,
        behavior: MonsterPackBehaviorDto::Seek,
    });
    let mut pack_member = actor.clone();
    pack_member.id = "test.scroll.pack-member".to_owned();
    pack_member.position = Position { x: 10, y: 15 };
    pack_member.pack = Some(MonsterPackIdentity {
        id: pack_id,
        leader_id,
        role: MonsterPackRoleDto::Member,
        behavior: MonsterPackBehaviorDto::Seek,
    });
    game.entities = vec![retained, dropped, mount, pack_leader, pack_member];

    let item_template = game
        .items
        .first()
        .expect("player should have a starting item")
        .clone();
    let mut retained_item = item_template.clone();
    retained_item.id = "test.scroll.item-retained".to_owned();
    retained_item.location = ItemLocation::Ground(Position { x: 40, y: 12 });
    let mut dropped_item = item_template.clone();
    dropped_item.id = "test.scroll.item-dropped".to_owned();
    dropped_item.location = ItemLocation::Ground(Position { x: 10, y: 12 });
    let mut carried_by_pack = item_template;
    carried_by_pack.id = "test.scroll.item-carried".to_owned();
    carried_by_pack.location = ItemLocation::CarriedBy {
        actor_id: "test.scroll.pack-leader".to_owned(),
    };
    game.items
        .extend([retained_item, dropped_item, carried_by_pack]);
    game.gold_piles = vec![
        GoldPile {
            id: "test.scroll.gold-retained".to_owned(),
            position: Position { x: 40, y: 13 },
            amount: 1,
            appearance: GoldAppearanceDto::Copper,
            discovered: true,
        },
        GoldPile {
            id: "test.scroll.gold-dropped".to_owned(),
            position: Position { x: 10, y: 13 },
            amount: 2,
            appearance: GoldAppearanceDto::Silver,
            discovered: true,
        },
    ];

    let remembered = Position { x: 40, y: 12 };
    let remembered_index = game
        .index(remembered)
        .expect("remembered cell should exist");
    game.terrain[remembered_index] = "demo.terrain.created-trap".to_owned();
    game.glow[remembered_index] = true;
    game.explored[remembered_index] = true;
    game.revealed_terrain.insert(remembered);
    game.summon_command = SummonCommandDto {
        mode: SummonCommandModeDto::Guard,
        guard_position: Some(remembered),
    };
    game.player.position = Position { x: 63, y: 16 };
    let mut removed = Vec::new();

    let transition = game
        .scroll_wilderness_for_player_entry(Position { x: 64, y: 16 }, &mut removed)
        .expect("wilderness scroll should resolve");

    assert!(matches!(
        transition,
        wilderness::WildernessPlayerEntry::Local {
            target: Position { x: 32, y: 16 },
            crossed_world_cell: false,
            translation: Some(Position { x: -32, y: 0 }),
        }
    ));
    assert_eq!(game.player.position, Position { x: 31, y: 16 });
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    let translated = Position { x: 8, y: 12 };
    let translated_index = game
        .index(translated)
        .expect("translated cell should exist");
    assert_eq!(game.terrain[translated_index], "demo.terrain.created-trap");
    assert!(game.glow[translated_index]);
    assert!(game.explored[translated_index]);
    assert!(game.revealed_terrain.contains(&translated));
    assert_eq!(game.summon_command.guard_position, Some(translated));
    assert_eq!(
        game.entities
            .iter()
            .map(|entity| (entity.id.as_str(), entity.position))
            .collect::<Vec<_>>(),
        [
            ("test.scroll.retained", Position { x: 38, y: 16 }),
            ("test.scroll.mount", Position { x: 31, y: 16 }),
        ]
    );
    assert_eq!(
        removed,
        [
            "test.scroll.dropped",
            "test.scroll.pack-leader",
            "test.scroll.pack-member",
        ]
    );
    assert!(game.items.iter().any(|item| {
        item.id == "test.scroll.item-retained"
            && item.location == ItemLocation::Ground(Position { x: 8, y: 12 })
    }));
    assert!(!game.items.iter().any(|item| matches!(
        item.id.as_str(),
        "test.scroll.item-dropped" | "test.scroll.item-carried"
    )));
    assert_eq!(
        game.gold_piles
            .iter()
            .map(|pile| (pile.id.as_str(), pile.position))
            .collect::<Vec<_>>(),
        [("test.scroll.gold-retained", Position { x: 8, y: 13 })]
    );
}

#[test]
fn diagonal_wilderness_scroll_translates_by_one_chunk_on_each_axis() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.player.position = Position { x: 63, y: 21 };

    let transition = game
        .scroll_wilderness_for_player_entry(Position { x: 64, y: 22 }, &mut Vec::new())
        .expect("diagonal wilderness scroll should resolve");

    assert!(matches!(
        transition,
        wilderness::WildernessPlayerEntry::Local {
            target: Position { x: 32, y: 11 },
            crossed_world_cell: false,
            translation: Some(Position { x: -32, y: -11 }),
        }
    ));
    assert_eq!(game.player.position, Position { x: 31, y: 10 });
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 1 });
}

#[test]
fn wilderness_scroll_populates_only_the_new_strip_without_using_ambush_rolls() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    game.player.position = Position { x: 63, y: 16 };
    let target = Position { x: 64, y: 16 };
    let target_index = game.index(target).expect("scroll target should exist");
    game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
    game.revealed_terrain.remove(&target);
    let global_rng = game.rng.clone();
    game.monster_division_remainders
        .insert("test.scroll.remainder".to_owned(), true);
    let division_remainders = game.monster_division_remainders.clone();
    let mut removed = Vec::new();
    let transition = game
        .scroll_wilderness_for_player_entry(target, &mut removed)
        .expect("wilderness scroll should resolve");
    let wilderness::WildernessPlayerEntry::Local {
        target,
        translation: Some(translation),
        ..
    } = transition
    else {
        panic!("wilderness scroll should retain the local floor");
    };
    game.relocate_player(target, &mut BTreeSet::new());

    game.populate_scrolled_wilderness(translation);

    let spawned = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".scroll."))
        .collect::<Vec<_>>();
    let leader_count = spawned
        .iter()
        .filter(|entity| !entity.id.contains(".companion."))
        .count();
    assert!(matches!(leader_count, 1 | 2));
    assert!(spawned.iter().all(|entity| entity.position.x >= 64));
    assert!(spawned.iter().all(|entity| !entity.id.contains(".ambush.")));
    assert_eq!(game.rng, global_rng);
    assert_eq!(game.monster_division_remainders, division_remainders);
}

#[test]
fn wilderness_scroll_keeps_new_monsters_outside_a_visible_small_town() {
    let (mut game, town_position) = game_with_second_town(42);
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));

    let mut last_translation = None;
    for _ in 0..2 {
        game.player.position = Position { x: 63, y: 16 };
        let target = Position { x: 64, y: 16 };
        let target_index = game.index(target).expect("scroll target should exist");
        game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
        game.revealed_terrain.remove(&target);
        let transition = game
            .scroll_wilderness_for_player_entry(target, &mut Vec::new())
            .expect("eastward wilderness scroll should resolve");
        let wilderness::WildernessPlayerEntry::Local {
            target,
            translation,
            ..
        } = transition
        else {
            panic!("wilderness scroll should remain local");
        };
        game.relocate_player(target, &mut BTreeSet::new());
        last_translation = translation;
    }
    assert_eq!(game.wilderness_position, Some(town_position));
    assert_eq!(game.wilderness_view_offset, Position { x: -1, y: 0 });

    game.populate_scrolled_wilderness(last_translation.expect("second scroll should translate"));

    let spawned = game
        .entities
        .iter()
        .filter(|entity| entity.id.contains(".scroll."))
        .collect::<Vec<_>>();
    assert!(!spawned.is_empty());
    assert!(spawned.iter().all(|entity| entity.position.x >= 64));
    assert!(spawned.iter().all(|entity| {
        !(77..82).contains(&entity.position.x) || !(15..18).contains(&entity.position.y)
    }));
}

#[test]
fn local_wilderness_cannot_roll_or_activate_a_world_map_ambush() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let rng = game.rng.clone();

    assert!(!game.roll_wilderness_ambush());
    assert_eq!(game.rng, rng);
    assert!(matches!(
        game.activate_wilderness_ambush(),
        Err(CoreError::WorldMapTransitionUnavailable)
    ));
    assert_eq!(game.map_scale, MapScaleDto::Local);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
}

#[test]
fn scrolling_into_and_out_of_a_town_stays_on_the_continuous_wilderness_surface() {
    let (mut game, town_position) = game_with_second_town(42);
    dispatch_next(&mut game, enter_world_map_command());
    assert!(game.move_on_world_map(Direction::East, &mut BTreeSet::new()));
    assert!(game.move_on_world_map(Direction::East, &mut BTreeSet::new()));
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.wilderness_position, Some(Position { x: 30, y: 52 }));
    assert!(game.wilderness_terrain_cache.len() >= 9);
    let wilderness_seed = game.wilderness_seed;

    game.player.position = Position { x: 32, y: 16 };
    let first = game
        .scroll_wilderness_for_player_entry(Position { x: 31, y: 16 }, &mut Vec::new())
        .expect("first westward scroll should resolve");
    let wilderness::WildernessPlayerEntry::Local { target, .. } = first else {
        panic!("first westward scroll should stay local");
    };
    game.relocate_player(target, &mut BTreeSet::new());

    game.player.position = Position { x: 32, y: 16 };
    let second = game
        .scroll_wilderness_for_player_entry(Position { x: 31, y: 16 }, &mut Vec::new())
        .expect("town boundary scroll should resolve");
    let wilderness::WildernessPlayerEntry::Local {
        target,
        crossed_world_cell,
        translation,
    } = second
    else {
        panic!("town boundary scroll should stay local");
    };
    assert!(crossed_world_cell);
    assert_eq!(translation, Some(Position { x: 32, y: 0 }));
    game.relocate_player(target, &mut BTreeSet::new());

    assert_eq!(game.wilderness_position, Some(town_position));
    assert_eq!(game.wilderness_view_offset, Position { x: 1, y: 0 });
    assert!(!game.wilderness_terrain_cache.is_empty());
    assert_eq!(game.wilderness_seed, wilderness_seed);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert!(game.current_town().is_none());
    assert_eq!(
        game.terrain_at(Position { x: 17, y: 16 }),
        "demo.terrain.outpost-gate"
    );

    game.player.position = Position { x: 32, y: 16 };
    let centered = game
        .scroll_wilderness_for_player_entry(Position { x: 31, y: 16 }, &mut Vec::new())
        .expect("centering scroll should resolve");
    let wilderness::WildernessPlayerEntry::Local { target, .. } = centered else {
        panic!("centering scroll should stay local");
    };
    game.relocate_player(target, &mut BTreeSet::new());
    assert_eq!(game.wilderness_view_offset, Position::default());
    let terrain_cache = game.wilderness_terrain_cache.clone();

    game.player.position = Position { x: 50, y: 16 };
    let entered = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    assert_eq!(entered.map_translation, None);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.current_town().map(|town| town.id.as_str()),
        Some("demo.town.second")
    );
    assert!(game.town_states["demo.town.second"].visited);
    assert_eq!(
        entered.shops[0].entrance_position,
        Position { x: 47, y: 16 }
    );
    assert_eq!(
        entered.homes[0].entrance_position,
        Position { x: 48, y: 16 }
    );

    let outside = Position { x: 44, y: 16 };
    let outside_index = game.index(outside).expect("outside town cell should exist");
    game.terrain[outside_index] = "demo.terrain.surface-path".to_owned();
    game.player.position = Position { x: 45, y: 16 };
    let left = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    assert!(game.current_town().is_none());
    assert!(left.town.is_none());
    assert!(left.shops.is_empty());
    assert!(left.homes.is_empty());
    assert_eq!(
        game.terrain_at(Position { x: 49, y: 16 }),
        "demo.terrain.outpost-gate"
    );
    assert_eq!(game.wilderness_terrain_cache, terrain_cache);
    assert_eq!(game.wilderness_seed, wilderness_seed);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("continuous town state should round-trip");
    assert_eq!(restored.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(restored.wilderness_position, Some(town_position));
    assert_eq!(restored.wilderness_view_offset, Position::default());
    assert_eq!(restored.wilderness_seed, wilderness_seed);
    assert!(restored.current_town().is_none());
}

#[test]
fn wilderness_view_offset_round_trips_and_rejects_out_of_range_values() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    game.entities.clear();
    game.items
        .retain(|item| !matches!(item.location, ItemLocation::CarriedBy { .. }));
    game.player.position = Position { x: 63, y: 16 };
    let target = Position { x: 64, y: 16 };
    let target_index = game.index(target).expect("scroll target should exist");
    game.terrain[target_index] = "demo.terrain.surface-path".to_owned();
    game.revealed_terrain.remove(&target);
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );

    let shifted = Game::from_save(game.to_save()).expect("scrolled wilderness should reload");
    assert_eq!(shifted.wilderness_view_offset, Position { x: 1, y: 0 });
    assert_eq!(shifted.wilderness_position, game.wilderness_position);
    assert_eq!(shifted.terrain, game.terrain);
    assert_eq!(shifted.state_hash(), game.state_hash());

    let mut invalid_save = game.to_save();
    invalid_save.wilderness_view_offset = Position { x: 2, y: 0 };
    assert!(matches!(
        Game::from_save(invalid_save),
        Err(CoreError::InvalidSave("wilderness view offset is invalid"))
    ));
}

#[test]
fn returning_to_the_outpost_coordinate_restores_its_preserved_floor() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
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

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.player.position, town_position);
    assert_eq!(&game.terrain[..96 * 32], &town_terrain[..96 * 32]);
    assert_eq!(game.entities, town_entities);
    assert_eq!(returned.changed_cells.len(), 96 * 33);
    assert!(game.stored_floors.contains_key("demo.floor.surface"));
}

#[test]
fn p102c_chameleon_cave_generates_chameleons_and_rewards_polymorph() {
    let mut game =
        Game::new_with_build(202, "demo.build.warrior").expect("Middle-earth should create");
    let mut floors = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.chameleon-cave"))
        .cloned()
        .collect::<Vec<_>>();
    floors.sort_by_key(|floor| floor.depth);
    assert_eq!(floors.len(), 16);

    let mut initialized_forms = 0;
    for definition in &floors {
        let generated = game
            .generate_procedural_floor(definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        assert!(generated.entities.iter().any(|actor| {
            matches!(
                actor.kind_id.as_str(),
                "demo.actor.chameleon" | "demo.actor.chameleon-lord"
            )
        }));
        assert!(generated.entities.iter().all(|actor| matches!(
            actor.kind_id.as_str(),
            "demo.actor.chameleon" | "demo.actor.chameleon-lord"
        )));
        initialized_forms += generated
            .entities
            .iter()
            .filter(|actor| actor.appearance_kind_id.is_some())
            .count();
    }
    assert!(initialized_forms > 0);

    let final_floor = floors.last().expect("depth 45 should exist").clone();
    let generated = game
        .generate_procedural_floor(&final_floor, None)
        .expect("Chameleon cave final floor should generate");
    let guardian = generated
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.chameleon-cave.1")
        .cloned()
        .expect("Chameleon Lord should guard depth 45");
    game.current_floor_id = final_floor.id;
    let (items, _) = game
        .generate_death_loot(&guardian)
        .expect("Chameleon Lord reward should generate");
    assert!(
        items
            .iter()
            .any(|item| item.kind_id == "demo.item.polymorph-potion")
    );
}

#[test]
fn formal_towns_share_the_continuous_surface_and_initialize_facilities_lazily() {
    const SECOND_TOWN_ID: &str = "demo.town.second";
    const SECOND_FLOOR_ID: &str = "demo.floor.second-town";
    const SECOND_SHOP_ID: &str = "demo.shop.second-general-store";
    const SECOND_HOME_ID: &str = "demo.town-facility.second-home";
    const SHARED_HOME_ID: &str = "demo.town-facility.outpost-home";

    let baseline = Game::new_with_build(42, "demo.build.warrior")
        .expect("baseline Warrens game should create");
    let (mut game, second_position) = game_with_second_town(42);
    assert_eq!(game.shop_states, baseline.shop_states);
    assert_eq!(game.rng.draw_counter, baseline.rng.draw_counter);
    assert!(!game.town_states.contains_key(SECOND_TOWN_ID));
    assert!(game.home_states.contains_key(SHARED_HOME_ID));
    assert!(!game.home_states.contains_key(SECOND_HOME_ID));
    assert!(!game.shop_states.contains_key(SECOND_SHOP_ID));

    dispatch_next(&mut game, enter_world_map_command());
    assert_eq!(game.wilderness_position, Some(Position { x: 28, y: 52 }));
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert_eq!(game.wilderness_position, Some(second_position));
    let entered = dispatch_next(&mut game, GameCommand::LeaveWorldMap);

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        entered.town.as_ref().map(|town| town.id.as_str()),
        Some(SECOND_TOWN_ID)
    );
    assert!(game.town_states.contains_key(SECOND_TOWN_ID));
    assert!(game.home_states.contains_key(SHARED_HOME_ID));
    assert!(!game.home_states.contains_key(SECOND_HOME_ID));
    assert!(!game.shop_states.contains_key(SECOND_SHOP_ID));
    assert_eq!(game.shop_states, baseline.shop_states);
    assert_eq!(game.rng.draw_counter, baseline.rng.draw_counter);
    assert!(game.stored_floors.contains_key("demo.floor.surface"));
    assert!(game.stored_floors.contains_key(SECOND_FLOOR_ID));

    let shop_entry = dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    assert!(game.shop_states.contains_key(SECOND_SHOP_ID));
    assert!(
        shop_entry
            .shops
            .iter()
            .find(|shop| shop.id == SECOND_SHOP_ID)
            .is_some_and(|shop| shop.visited && shop.player_at_entrance && !shop.stock.is_empty())
    );
    let stock = game.shop_states[SECOND_SHOP_ID].inventory.clone();

    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::West,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert!(game.stored_floors.contains_key(SECOND_FLOOR_ID));

    dispatch_next(&mut game, enter_world_map_command());
    dispatch_next(
        &mut game,
        GameCommand::Move {
            direction: Direction::East,
        },
    );
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.shop_states[SECOND_SHOP_ID].inventory, stock);

    let restored = Game::from_save_with_content(game.to_save(), game.content.clone())
        .expect("second town should round-trip");
    assert_eq!(restored.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        restored.current_town().map(|town| town.id.as_str()),
        Some(SECOND_TOWN_ID)
    );
    assert_eq!(restored.shop_states[SECOND_SHOP_ID].inventory, stock);
    assert!(restored.stored_floors.contains_key("demo.floor.surface"));
}

#[test]
fn p103e_volcano_generates_lava_guardians_and_fixed_staff_reward() {
    let mut game =
        Game::new_with_build(203, "demo.build.warrior").expect("Middle-earth should create");
    dispatch_next(&mut game, enter_world_map_command());
    game.wilderness_position = Some(Position { x: 13, y: 53 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    let entrance = game
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.volcano-entrance.1")
        .expect("Lesser Balrog should guard Volcano");
    assert_eq!(entrance.kind_id, "demo.actor.lesser-balrog");
    assert!(entrance.pack.as_ref().is_some_and(|pack| {
        pack.behavior == MonsterPackBehaviorDto::GuardPosition && pack.leader_id == entrance.id
    }));

    let mut definitions = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth should remain available")
        .procedural_floors
        .iter()
        .filter(|floor| floor.dungeon_id.as_deref() == Some("demo.dungeon.volcano"))
        .cloned()
        .collect::<Vec<_>>();
    definitions.sort_by_key(|floor| floor.depth);
    assert_eq!(definitions.len(), 11);

    let mut generated_lava = 0;
    let mut generated_rubble = false;
    for depth in [50, 55, 57, 60] {
        let definition = definitions
            .iter()
            .find(|floor| floor.depth == depth)
            .expect("representative Volcano layer");
        let generated = game
            .generate_procedural_floor(definition, None)
            .unwrap_or_else(|error| panic!("{} should generate: {error}", definition.id));
        generated_lava += generated
            .terrain
            .iter()
            .filter(|terrain| {
                matches!(
                    terrain.as_str(),
                    "demo.terrain.surface-lava-deep" | "demo.terrain.surface-lava-shallow"
                )
            })
            .count();
        generated_rubble |= generated
            .terrain
            .iter()
            .any(|terrain| terrain == "demo.terrain.rubble");
        assert!(generated.entities.iter().all(|actor| {
            let index = actor.position.y as usize * usize::from(generated.width)
                + actor.position.x as usize;
            game.content
                .terrain(&generated.terrain[index])
                .is_some_and(|terrain| terrain.walkable)
        }));
        if definition.final_floor {
            assert!(generated.entities.iter().any(|actor| {
                actor.id == "demo.guardian.volcano.1"
                    && actor.kind_id == "demo.actor.shooting-star-the-red-dragon"
            }));
        }
    }
    assert!(generated_lava > 0);
    assert!(generated_rubble);

    let final_floor = definitions.last().expect("depth 60 should exist");
    let generated = game
        .generate_procedural_floor(final_floor, None)
        .expect("Volcano final floor should generate");
    let guardian = generated
        .entities
        .iter()
        .find(|actor| actor.id == "demo.guardian.volcano.1")
        .cloned()
        .expect("Shooting Star should guard depth 60");
    game.current_floor_id = final_floor.id.clone();
    let (items, _) = game
        .generate_death_loot(&guardian)
        .expect("Shooting Star reward should generate");
    let staff = items
        .iter()
        .find(|item| item.kind_id == "demo.item.mana-storm-staff")
        .expect("Shooting Star should drop the fixed Mana Storm staff");
    assert_eq!(
        staff
            .activation
            .as_ref()
            .map(|activation| activation.profile_id.as_str()),
        Some("demo.device-activation.mana-storm")
    );
    assert_eq!(staff.charges.map(|charges| charges.maximum), Some(5));
}
