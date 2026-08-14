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
    let pack_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("core crate should be inside the workspace")
        .join("packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack_root).expect("demo pack should compile");
    let world = artifact
        .content
        .worlds
        .iter_mut()
        .find(|world| world.id == DEFAULT_WORLD_ID)
        .expect("Middle-earth world should remain available");
    let primary = world
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.camelot")
        .expect("Camelot should remain available");
    primary.legacy_index = Some(31);
    primary.substitution = Some(rfb_content::DungeonSubstitutionDefinition {
        alternate_dungeon_id: "demo.dungeon.tidal-cave".to_owned(),
        alternate_gate_one_in: Some(32),
    });
    world
        .dungeons
        .iter_mut()
        .find(|dungeon| dungeon.id == "demo.dungeon.tidal-cave")
        .expect("Tidal Cave should remain available")
        .legacy_index = Some(40);
    let shared_position = world
        .wilderness
        .as_ref()
        .expect("Middle-earth should retain wilderness")
        .locations
        .iter()
        .find_map(|location| match location {
            rfb_content::WildernessLocationDefinition::Dungeon {
                position,
                dungeon_id,
            } if dungeon_id == "demo.dungeon.camelot" => Some(*position),
            _ => None,
        })
        .expect("Camelot should retain its wilderness location");
    for location in &mut world
        .wilderness
        .as_mut()
        .expect("Middle-earth should retain wilderness")
        .locations
    {
        if let rfb_content::WildernessLocationDefinition::Dungeon {
            position,
            dungeon_id,
        } = location
            && dungeon_id == "demo.dungeon.tidal-cave"
        {
            *position = shared_position;
        }
    }
    world
        .procedural_floors
        .iter_mut()
        .find(|floor| floor.id == "demo.floor.tidal-cave-depth-15")
        .expect("Tidal Cave root should remain available")
        .entry_terrain_id = Some("demo.terrain.camelot-entrance".to_owned());

    let catalog = Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content)
            .expect("substitution test content should remain valid"),
    ));
    Game::from_content(seed, catalog, DEFAULT_WORLD_ID)
        .expect("substitution test game should initialize")
}

#[test]
fn p89b_substitute_selection_is_seeded_persisted_and_hashed() {
    let primary = game_with_dungeon_substitution(0);
    assert!(primary.dungeon_is_active("demo.dungeon.camelot"));
    assert!(!primary.dungeon_is_active("demo.dungeon.tidal-cave"));
    let failed_extra_gate = game_with_dungeon_substitution(1_528);
    assert!(failed_extra_gate.dungeon_is_active("demo.dungeon.camelot"));
    assert!(!failed_extra_gate.dungeon_is_active("demo.dungeon.tidal-cave"));

    let mut alternate = game_with_dungeon_substitution(1_536);
    assert!(!alternate.dungeon_is_active("demo.dungeon.camelot"));
    assert!(alternate.dungeon_is_active("demo.dungeon.tidal-cave"));
    let mut opposite_selection = alternate.clone();
    opposite_selection
        .dungeon_states
        .get_mut("demo.dungeon.camelot")
        .expect("Camelot state")
        .suppressed = false;
    opposite_selection
        .dungeon_states
        .get_mut("demo.dungeon.tidal-cave")
        .expect("Tidal Cave state")
        .suppressed = true;
    assert_ne!(alternate.state_hash(), opposite_selection.state_hash());
    let mut suppressed_conquest = alternate.clone();
    suppressed_conquest
        .dungeon_states
        .get_mut("demo.dungeon.camelot")
        .expect("Camelot state")
        .guardian_defeated = true;
    assert_eq!(suppressed_conquest.campaign_counts().0, 0);
    assert!(suppressed_conquest.validate_loaded_state().is_err());
    alternate.advance_wilderness_generation();
    assert!(!alternate.dungeon_is_active("demo.dungeon.camelot"));
    assert!(alternate.dungeon_is_active("demo.dungeon.tidal-cave"));

    let payload = alternate.to_save();
    assert!(
        payload
            .dungeon_states
            .iter()
            .any(|state| { state.dungeon_id == "demo.dungeon.camelot" && state.suppressed })
    );
    let restored = Game::from_save_with_content(payload, alternate.content.clone())
        .expect("substitution state should restore");
    assert_eq!(restored.state_hash(), alternate.state_hash());
    assert!(!restored.dungeon_is_active("demo.dungeon.camelot"));
    assert!(restored.dungeon_is_active("demo.dungeon.tidal-cave"));
}

#[test]
fn p89b_shared_entrance_map_and_guardians_use_only_the_active_dungeon() {
    for (seed, active_dungeon, active_floor, active_guardian, suppressed_guardian) in [
        (
            0,
            "demo.dungeon.camelot",
            "demo.floor.camelot-depth-20",
            "demo.actor.arthur-pendragon",
            "demo.actor.grendel",
        ),
        (
            1_536,
            "demo.dungeon.tidal-cave",
            "demo.floor.tidal-cave-depth-15",
            "demo.actor.grendel",
            "demo.actor.arthur-pendragon",
        ),
    ] {
        let mut game = game_with_dungeon_substitution(seed);
        let world_position = Position { x: 7, y: 59 };
        let cell = game.wilderness_cell_dto(world_position);
        assert!(
            cell.locations
                .iter()
                .any(|location| location.id == active_dungeon)
        );
        assert_eq!(
            cell.locations
                .iter()
                .filter(|location| location.id == "demo.dungeon.camelot"
                    || location.id == "demo.dungeon.tidal-cave")
                .count(),
            1
        );
        assert!(game.actor_kind_is_dungeon_guardian(active_guardian));
        assert!(!game.actor_kind_is_dungeon_guardian(suppressed_guardian));

        game.wilderness_position = Some(world_position);
        let index = usize::try_from(game.player.position.y).expect("player y should fit usize")
            * usize::from(game.width)
            + usize::try_from(game.player.position.x).expect("player x should fit usize");
        game.terrain[index] = "demo.terrain.camelot-entrance".to_owned();
        game.traverse_stairs(false)
            .expect("shared entrance should resolve")
            .expect("shared entrance should enter its active dungeon");
        assert_eq!(game.current_floor_id, active_floor);
    }
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
