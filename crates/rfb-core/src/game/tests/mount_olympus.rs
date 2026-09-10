// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
use std::sync::OnceLock;

const PERMANENT: &str = "demo.terrain.permanent-wall";
const DEEP: &str = "demo.terrain.surface-water-deep";
const SHALLOW: &str = "demo.terrain.surface-water-shallow";

fn catalog() -> Arc<ContentCatalog> {
    static CONTENT: OnceLock<Arc<ContentCatalog>> = OnceLock::new();
    CONTENT
        .get_or_init(|| {
            let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../packs/rfb-demo-original");
            let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
            let world = &mut artifact.content.worlds[0];
            // OL2 stages the source policy on an existing valid chain. The actual
            // Olympus entrance, guardian rewards and eleven floors belong to OL3/4.
            let dungeon = world
                .dungeons
                .iter_mut()
                .find(|d| d.id == "demo.dungeon.rlyeh")
                .unwrap();
            dungeon.legacy_index = Some(22);
            dungeon.pantheon = Some(1);
            for depth in [80, 85, 90] {
                let floor = world
                    .procedural_floors
                    .iter_mut()
                    .find(|f| f.id == format!("demo.floor.rlyeh-depth-{depth}"))
                    .unwrap();
                floor.width = 96;
                floor.height = 33;
                floor.terrain_feature_table_id = None;
                floor.generation_budget = Some(
                    serde_json::from_value(serde_json::json!({
                    "actorSlots": 20, "lootPlacements": 8,
                    "roomPlacements": 6, "roomAreaTiles": 1100,
                            "cavernAreaTiles": 600, "riverAreaTiles": 240
                        }))
                    .unwrap(),
                );
                floor.layout = Some(
                    serde_json::from_value(serde_json::json!({
                        "wallMix": [{"terrainId": PERMANENT, "percent": 40}],
                        "rooms": {"placement": "free", "minWidth": 8, "maxWidth": 20,
                            "minHeight": 7, "maxHeight": 13,
                            "shapes": [{"shape": "cavern", "weight": 1000},
                                       {"shape": "rectangle", "weight": 450}]},
                        "cavern": {"terrainId": "demo.terrain.floor", "rfbDepthChance": true},
                        "river": {"deepTerrainId": DEEP, "shallowTerrainId": SHALLOW,
                                  "chanceOneIn": 7, "rfbDepthChance": true},
                        "stairs": {"up": {"minimum": 1, "maximum": 2},
                                   "down": {"minimum": 4, "maximum": 5}},
                        "placeDoors": false
                    }))
                    .unwrap(),
                );
                floor.loot_allocation = Some(
                    serde_json::from_value(serde_json::json!({
                        "referenceAreaTiles": 13068,
                        "roomObjects": {"mean": 8, "standardDeviation": 3},
                        "anywhereObjects": {"mean": 2, "standardDeviation": 3}
                    }))
                    .unwrap(),
                );
                floor.gold_allocation = Some(
                    serde_json::from_value(serde_json::json!({
                        "referenceAreaTiles": 13068, "piles": {"mean": 2, "standardDeviation": 3}
                    }))
                    .unwrap(),
                );
            }
            let policy = artifact
                .content
                .encounter_tables
                .iter_mut()
                .find(|t| t.id == "demo.encounter-table.rlyeh")
                .unwrap()
                .global_allocation
                .as_mut()
                .unwrap();
            policy.preferred_tags = ["giant", "olympian", "olympian2"]
                .map(str::to_owned)
                .to_vec();
            policy.special_div = 8;
            policy.ambient_chance_one_in = 160;
            Arc::new(ContentCatalog::from_artifact(
                rfb_content::encode_content(artifact.content).unwrap(),
            ))
        })
        .clone()
}

fn game_and_floor(depth: u16) -> (Game, rfb_content::ProceduralFloorDefinition) {
    let game = (0..32)
        .map(|seed| Game::from_content(seed, catalog(), DEFAULT_WORLD_ID).unwrap())
        .find(|game| game.active_pantheons & 2 != 0)
        .unwrap();
    let floor = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == format!("demo.floor.rlyeh-depth-{depth}"))
        .unwrap()
        .clone();
    (game, floor)
}

#[test]
fn mount_olympus_representative_floors_keep_routes_spawns_and_normal_ecology() {
    let mut wet = 0;
    let mut dry = 0;
    let mut companions = 0;
    let mut objects = 0;
    let mut gold = 0;
    let mut low_level = false;
    for depth in [80, 85, 90] {
        let (base, mut definition) = game_and_floor(depth);
        // Exercise the terminal guardian geometry without opening its entrance.
        if depth == 90 {
            definition.final_floor = true;
            definition.next_floor_id = None;
            definition.down_stair_terrain_id = None;
            definition
                .layout
                .as_mut()
                .unwrap()
                .stairs
                .as_mut()
                .unwrap()
                .down = None;
            definition.guardian = Some(serde_json::from_value(serde_json::json!({
                "instanceId": "test.olympus.guardian", "actorKindId": "demo.actor.zeus-king-of-the-olympians"
            })).unwrap());
        }
        for seed in 0..16 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            let at = |p| {
                game.content
                    .terrain(&floor.terrain[generated_terrain_index(floor.width, p)])
                    .unwrap()
            };
            let walkable = floor
                .terrain
                .iter()
                .enumerate()
                .filter_map(|(i, id)| {
                    game.content
                        .terrain(id)
                        .unwrap()
                        .walkable
                        .then_some(Position {
                            x: (i % usize::from(floor.width)) as i32,
                            y: (i / usize::from(floor.width)) as i32,
                        })
                })
                .collect::<BTreeSet<_>>();
            let reached = maze_floor_distances(&walkable, floor.player_position);
            let mut occupied = BTreeSet::from([floor.player_position]);
            assert!(at(floor.player_position).walkable);
            for (i, id) in floor.terrain.iter().enumerate() {
                let terrain = game.content.terrain(id).unwrap();
                assert!(!terrain.tags.iter().any(|tag| tag == "door"));
                if terrain
                    .tags
                    .iter()
                    .any(|tag| tag == "stairs-up" || tag == "stairs-down")
                {
                    let p = Position {
                        x: (i % usize::from(floor.width)) as i32,
                        y: (i / usize::from(floor.width)) as i32,
                    };
                    assert!(
                        reached.contains_key(&p),
                        "depth {depth}, seed {seed}, stair {p:?}"
                    );
                    // Entry on an up stair is normal; no actor may occupy it.
                    occupied.insert(p);
                }
            }
            assert!(floor.terrain.iter().any(|id| id == PERMANENT));
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.trap_terrain_id)
            );
            let water = floor.terrain.iter().any(|id| id == DEEP || id == SHALLOW);
            wet += usize::from(water);
            dry += usize::from(!water);
            for actor in &floor.entities {
                assert!(
                    occupied.insert(actor.position),
                    "overlap at {:?}",
                    actor.position
                );
                let kind = game.content.actor(&actor.kind_id).unwrap();
                assert!(actor_can_cross_terrain(kind, at(actor.position)));
                if actor.id == "test.olympus.guardian" {
                    assert!(reached.contains_key(&actor.position));
                } else {
                    assert!(game.pantheon_allows_allocation(&definition.id, kind));
                    low_level |= kind.level < 50;
                }
                companions += usize::from(actor.id.contains("companion"));
            }
            assert_eq!(
                floor
                    .entities
                    .iter()
                    .filter(|a| a.id == "test.olympus.guardian")
                    .count(),
                usize::from(depth == 90)
            );
            assert!(floor.items.iter().all(|item| match item.location {
                ItemLocation::Ground(position) => at(position).allows_items(),
                _ => false,
            }));
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            objects += floor.items.len();
            gold += floor.gold_piles.len();
        }
    }
    assert!(wet > 0 && dry > 0, "wet={wet}, dry={dry}");
    assert!(
        companions > 0 && low_level,
        "ordinary groups and low-level candidates remain available"
    );
    assert!(objects > 0 && gold > 0, "objects={objects}, gold={gold}");
}

#[test]
fn mount_olympus_cavern_gate_matches_source_roll_and_preserves_rng() {
    let (base, mut definition) = game_and_floor(80);
    definition.layout.as_mut().unwrap().river = None;
    for depth in [20, 80, 85, 90] {
        definition.depth = depth;
        for success in [false, true] {
            if depth == 20 && success {
                continue;
            }
            let seed = (0..10000)
                .find(|seed| {
                    (RfbRng::seeded(*seed).bounded(1000) + 1 < u64::from(depth)) == success
                })
                .unwrap();
            let mut actual = base.clone();
            actual.rng = RfbRng::seeded(seed);
            let mut expected = actual.clone();
            let mut forced = definition.clone();
            let selected = depth > 20 && expected.rng.bounded(1000) + 1 < u64::from(depth);
            if selected {
                forced
                    .layout
                    .as_mut()
                    .unwrap()
                    .cavern
                    .as_mut()
                    .unwrap()
                    .rfb_depth_chance = false;
            } else {
                forced.layout.as_mut().unwrap().cavern = None;
            }
            let actual_floor = actual.generate_procedural_floor(&definition, None).unwrap();
            let expected_floor = expected.generate_procedural_floor(&forced, None).unwrap();
            assert_eq!(actual_floor.terrain, expected_floor.terrain);
            assert_eq!(actual.rng.draw_counter, expected.rng.draw_counter);
        }
    }
}

#[test]
fn mount_olympus_water_river_keeps_permanent_fill_and_source_depth_exclusions() {
    let (base, mut definition) = game_and_floor(80);
    let mut plain = vec![
        definition.wall_terrain_id.clone();
        usize::from(definition.width) * usize::from(definition.height)
    ];
    let mut mixed = plain.clone();
    for (i, tile) in mixed.iter_mut().enumerate() {
        if i % 5 < 2 {
            *tile = PERMANENT.into();
        }
    }
    let permanent = mixed
        .iter()
        .enumerate()
        .filter_map(|(i, id)| (id == PERMANENT).then_some(i))
        .collect::<BTreeSet<_>>();
    let mut ordinary_game = base.clone();
    let mut mixed_game = base.clone();
    let center = Position { x: 48, y: 16 };
    ordinary_game.generate_river(&definition, DEEP, SHALLOW, center, &mut plain);
    mixed_game.generate_river(&definition, DEEP, SHALLOW, center, &mut mixed);
    assert!(
        permanent
            .iter()
            .any(|i| plain[*i] == DEEP || plain[*i] == SHALLOW)
    );
    for i in 0..mixed.len() {
        assert_eq!(
            mixed[i],
            if permanent.contains(&i) {
                PERMANENT
            } else {
                &plain[i]
            }
        );
    }
    assert_eq!(ordinary_game.rng.draw_counter, mixed_game.rng.draw_counter);
    definition.layout.as_mut().unwrap().cavern = None;
    definition
        .layout
        .as_mut()
        .unwrap()
        .river
        .as_mut()
        .unwrap()
        .chance_one_in = None;
    for depth in [5, 255] {
        definition.depth = depth;
        let floor = base
            .clone()
            .generate_procedural_floor(&definition, None)
            .unwrap();
        assert!(!floor.terrain.iter().any(|id| id == DEEP || id == SHALLOW));
    }
}

#[test]
fn mount_olympus_preferences_keep_source_rarity_and_divisor_eight() {
    let (mut game, definition) = game_and_floor(80);
    let policy = game
        .content
        .encounter_table("demo.encounter-table.rlyeh")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    for tag in ["giant", "olympian", "olympian2"] {
        let actor = game
            .content
            .actor_definitions()
            .find(|a| a.tags.iter().any(|t| t == tag) && a.allocation.is_some())
            .unwrap()
            .clone();
        let before = game.rng.draw_counter;
        assert_eq!(
            game.original_dungeon_weight(&actor, &policy),
            100 / actor.allocation.as_ref().unwrap().rarity
        );
        assert_eq!(game.rng.draw_counter, before);
    }
    let actor = game.content.actor("demo.actor.newt").unwrap().clone();
    assert!([12, 13].contains(&game.original_dungeon_weight(&actor, &policy)));

    let floor = game.generate_procedural_floor(&definition, None).unwrap();
    game.activate_floor(floor, Vec::new());
    super::support::clear_monsters(&mut game);
    // Keep the source 160 parameter and the consumer's depth scaling; select
    // a reproducible successful roll without increasing ambient frequency.
    let chance = u64::from(policy.ambient_chance_one_in) * 180 / 100;
    let seed = (0..10000)
        .find(|seed| RfbRng::seeded(*seed).bounded(chance) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .unwrap();
    assert!(!game.entities.is_empty());
    for actor in &game.entities {
        let kind = game.content.actor(&actor.kind_id).unwrap();
        assert!(game.pantheon_allows_allocation(&definition.id, kind));
        assert!(
            crate::game::projectile_geometry::rfb_distance(game.player.position, actor.position)
                > 25
        );
    }
}
