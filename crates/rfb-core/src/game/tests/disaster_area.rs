// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
use rfb_content::{
    ActorDamageType, ActorResistanceLevel, ProceduralFloorDefinition,
    ProceduralTerrainMixDefinition,
};

const SHALLOW: &str = "demo.terrain.shallow-waste";
const DEEP: &str = "demo.terrain.deep-waste";
const MOUNTAIN: &str = "demo.terrain.mountain-wall";
const QUARTZ: &str = "demo.terrain.quartz-vein";

fn assert_stairs_connected(floor: &FloorState, content: &ContentCatalog) {
    let positions = floor
        .terrain
        .iter()
        .enumerate()
        .map(|(index, id)| {
            (
                Position {
                    x: (index % usize::from(floor.width)) as i32,
                    y: (index / usize::from(floor.width)) as i32,
                },
                content.terrain(id).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let walkable = positions
        .iter()
        .filter(|(_, terrain)| terrain.walkable || terrain.open_to_terrain_id.is_some())
        .map(|(position, _)| *position)
        .collect();
    let reached = maze_floor_distances(&walkable, floor.player_position);
    for (position, terrain) in positions {
        if terrain
            .tags
            .iter()
            .any(|tag| tag == "stairs-up" || tag == "stairs-down")
        {
            assert!(
                reached.contains_key(&position),
                "stair {position:?} must be reachable without digging"
            );
        }
    }
    assert!(
        floor
            .connections
            .iter()
            .all(|connection| reached.contains_key(&connection.position))
    );
}

fn add_mix(floor: &mut ProceduralFloorDefinition) {
    let layout = floor.layout.as_mut().unwrap();
    layout.floor_mix = [(SHALLOW, 21), (DEEP, 3)]
        .map(|(id, percent)| ProceduralTerrainMixDefinition {
            terrain_id: id.to_owned(),
            percent,
        })
        .to_vec();
    layout.wall_mix = [(MOUNTAIN, 18), (QUARTZ, 2)]
        .map(|(id, percent)| ProceduralTerrainMixDefinition {
            terrain_id: id.to_owned(),
            percent,
        })
        .to_vec();
    layout.streamers.clear();
    let budget = floor.generation_budget.as_mut().unwrap();
    budget.streamer_placements = None;
    budget.streamer_area_tiles = None;
}

#[test]
fn disaster_area_mix_budgets_use_carved_floors_and_bulk_walls() {
    let mut game = Game::new(42);
    let mut definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    definition.theme_table_id = None;
    definition.vault_id = None;
    definition.connections.clear();
    add_mix(&mut definition);
    definition.layout.as_mut().unwrap().floor_mix.clear();
    definition.layout.as_mut().unwrap().wall_mix.clear();
    let layout = definition.layout.as_mut().unwrap();
    layout.place_doors = false;
    layout.stairs = None;
    layout.rooms.as_mut().unwrap().shapes =
        vec![rfb_content::ProceduralRoomShapeCandidateDefinition {
            shape: ProceduralRoomShape::Rectangle,
            weight: 1,
        }];
    game.rng = RfbRng::seeded(3);
    let rooms = game.clone().generate_budgeted_rooms(
        &definition,
        definition.layout.as_ref().unwrap().rooms.as_ref().unwrap(),
    );
    let ordinary = game
        .clone()
        .generate_procedural_floor(&definition, None)
        .unwrap();
    add_mix(&mut definition);
    let mixed = game.generate_procedural_floor(&definition, None).unwrap();
    let mut floor_candidates = 0;
    let mut wall_candidates = 0;
    for y in 0..i32::from(mixed.height) {
        for x in 0..i32::from(mixed.width) {
            let position = Position { x, y };
            let index = generated_terrain_index(mixed.width, position);
            let base = &ordinary.terrain[index];
            let actual = &mixed.terrain[index];
            let boundary = x == 0
                || y == 0
                || x + 1 == i32::from(mixed.width)
                || y + 1 == i32::from(mixed.height);
            let anchor = rooms
                .iter()
                .any(|room| room.center().x.abs_diff(x) <= 1 && room.center().y.abs_diff(y) <= 1);
            let room_wall = !rooms.iter().any(|room| room.contains(position))
                && rooms.iter().any(|room| {
                    (-1..=1).any(|dy| {
                        (-1..=1).any(|dx| {
                            room.contains(Position {
                                x: x + dx,
                                y: y + dy,
                            })
                        })
                    })
                });
            if boundary || anchor || base == &definition.wall_terrain_id && room_wall {
                assert_eq!(actual, base, "protected {position:?}");
            } else if base == &definition.floor_terrain_id {
                floor_candidates += 1;
                assert!([base.as_str(), SHALLOW, DEEP].contains(&actual.as_str()));
            } else if base == &definition.wall_terrain_id {
                wall_candidates += 1;
                assert!([base.as_str(), MOUNTAIN, QUARTZ].contains(&actual.as_str()));
            } else {
                assert_eq!(actual, base);
            }
        }
    }
    for (terrain, expected) in [
        (SHALLOW, floor_candidates * 21 / 100),
        (DEEP, floor_candidates * 3 / 100),
        (MOUNTAIN, wall_candidates * 18 / 100),
        (QUARTZ, wall_candidates * 2 / 100),
    ] {
        assert!(expected > 0);
        assert_eq!(
            mixed
                .terrain
                .iter()
                .filter(|id| id.as_str() == terrain)
                .count(),
            expected,
            "{terrain}"
        );
    }
    assert_stairs_connected(&mixed, &game.content);
}

#[test]
fn disaster_area_mixed_cave_hydrology_keeps_connections_and_legal_spawns() {
    let base = Game::new(42);
    let mut definition = base
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-1")
        .unwrap()
        .clone();
    add_mix(&mut definition);
    definition
        .layout
        .as_mut()
        .unwrap()
        .rooms
        .as_mut()
        .unwrap()
        .shapes = vec![rfb_content::ProceduralRoomShapeCandidateDefinition {
        shape: ProceduralRoomShape::Cavern,
        weight: 1,
    }];
    for (deep, shallow, river) in [
        (DEEP, SHALLOW, true),
        (DEEP, SHALLOW, false),
        (
            "demo.terrain.surface-water-deep",
            "demo.terrain.surface-water-shallow",
            false,
        ),
    ] {
        let mut definition = definition.clone();
        let layout = definition.layout.as_mut().unwrap();
        let budget = definition.generation_budget.as_mut().unwrap();
        if river {
            layout.river = Some(rfb_content::ProceduralRiverDefinition {
                deep_terrain_id: deep.to_owned(),
                shallow_terrain_id: shallow.to_owned(),
                chance_one_in: None,
                alternative: None,
            });
            budget.river_area_tiles = Some(180);
        } else {
            layout.lake = Some(rfb_content::ProceduralLakeDefinition {
                deep_terrain_id: deep.to_owned(),
                shallow_terrain_id: shallow.to_owned(),
            });
            budget.lake_area_tiles = Some(240);
            budget.lake_deep_area_tiles = Some(80);
        }
        for seed in [2, 7, 42] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let generated = game.generate_procedural_floor(&definition, None).unwrap();
            let at = |position| {
                game.content
                    .terrain(&generated.terrain[generated_terrain_index(generated.width, position)])
                    .unwrap()
            };
            assert!(generated.terrain.iter().any(|id| id == deep));
            assert!(generated.terrain.iter().any(|id| id == SHALLOW));
            assert!(generated.terrain.iter().any(|id| id == MOUNTAIN));
            assert_eq!(generated.connections.len(), definition.connections.len());
            for connection in &generated.connections {
                assert_eq!(
                    at(connection.position).id,
                    definition
                        .connections
                        .iter()
                        .find(|c| c.id == connection.id)
                        .unwrap()
                        .terrain_id
                );
            }
            assert_stairs_connected(&generated, &game.content);
            assert!(!generated.entities.is_empty());
            assert!(!generated.items.is_empty());
            assert!(
                generated
                    .entities
                    .iter()
                    .all(|actor| actor_can_cross_terrain(
                        game.content.actor(&actor.kind_id).unwrap(),
                        at(actor.position)
                    ))
            );
            assert!(generated.items.iter().all(|item| match item.location {
                ItemLocation::Ground(position) => at(position).allows_items(),
                _ => true,
            }));
            assert!(
                generated
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
        }
    }
}

fn guardian_content(kind: &str) -> Arc<ContentCatalog> {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    let world = &mut artifact.content.worlds[0];
    // This fixture reuses Warrens' short chain, but the two new bosses are not
    // the campaign's victory target.
    world.campaign.as_mut().unwrap().victory_dungeon_ids =
        vec!["demo.dungeon.witch-wood".to_owned()];
    let dungeon = world
        .dungeons
        .iter_mut()
        .find(|d| d.id == "demo.dungeon.warrens")
        .unwrap();
    dungeon.legacy_index = Some(if kind == "demo.actor.godzilla" {
        37
    } else {
        19
    });
    dungeon.guardian_actor_kind_id = Some(kind.to_owned());
    let floor = world
        .procedural_floors
        .iter_mut()
        .find(|f| f.id == "demo.floor.warrens-depth-9")
        .unwrap();
    floor.encounter_table_id = Some(
        if kind == "demo.actor.godzilla" {
            "demo.encounter-table.disaster-area"
        } else {
            "demo.encounter-table.dark-cave"
        }
        .to_owned(),
    );
    let guardian = floor.guardian.as_mut().unwrap();
    guardian.actor_kind_id = kind.to_owned();
    guardian.reward_artifact_item_kind_id = None;
    guardian.reward_loot_table_id =
        Some("demo.loot-table.first-realm-fourth-book-fallback".to_owned());
    guardian.reward_first_realm_book_rank = Some(4);
    Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ))
}

#[test]
fn disaster_area_allocation_accepts_real_poison_resistance_and_immunity() {
    let mut game = Game::from_content_with_build(
        42,
        guardian_content("demo.actor.godzilla"),
        DEFAULT_WORLD_ID,
        RFB_WARRIOR_BUILD_ID,
    )
    .unwrap();
    let policy = game
        .content
        .encounter_table("demo.encounter-table.disaster-area")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    let floor_id = "demo.floor.warrens-depth-9";
    let mut seen = Vec::new();
    for _ in 0..256 {
        let kind = game
            .select_original_allocated_monster(floor_id, &policy, 60, 60, None, &[], None, None)
            .unwrap();
        let actor = game.content.actor(&kind).unwrap();
        let allocation = actor.allocation.as_ref().unwrap();
        assert!(!allocation.wild_only);
        assert!(!game.actor_kind_is_dungeon_guardian(&kind));
        assert!(
            allocation.legacy_dungeon_indices.is_empty()
                || allocation.legacy_dungeon_indices.contains(&37)
        );
        let resistance = actor.resistances.get(&ActorDamageType::Poison).copied();
        assert!(matches!(
            resistance,
            Some(
                ActorResistanceLevel::Resistant
                    | ActorResistanceLevel::Strong
                    | ActorResistanceLevel::Immune
            )
        ));
        seen.push(resistance.unwrap());
    }
    assert!(seen.contains(&ActorResistanceLevel::Resistant));
    assert!(seen.contains(&ActorResistanceLevel::Immune));
    let ogre = game.content.actor("demo.actor.ogre").unwrap().clone();
    assert_eq!(game.original_dungeon_weight(&ogre, &policy), 0);
    let dark_policy = game
        .content
        .encounter_table("demo.encounter-table.dark-cave")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    assert_eq!(
        game.original_dungeon_weight(&ogre, &dark_policy),
        100 / ogre.allocation.as_ref().unwrap().rarity
    );
    assert!(
        game.select_original_allocated_monster(
            floor_id,
            &dark_policy,
            60,
            60,
            None,
            &[],
            None,
            None
        )
        .is_some()
    );
    assert!(
        game.content
            .actor("demo.actor.godzilla")
            .unwrap()
            .allocation
            .as_ref()
            .unwrap()
            .wild_only
    );
}

#[test]
fn dark_cave_disaster_area_fixed_bosses_grant_fourth_book_once_across_restore() {
    for kind in ["demo.actor.null-the-living-void", "demo.actor.godzilla"] {
        let content = guardian_content(kind);
        for (build, book) in [
            (
                "demo.build.high-mage-sorcery",
                "demo.item.grimoire-of-power",
            ),
            (RFB_WARRIOR_BUILD_ID, "demo.item.blessings-of-the-grail"),
        ] {
            let mut game =
                Game::from_content_with_build(42, content.clone(), DEFAULT_WORLD_ID, build)
                    .unwrap();
            game.transition_floor("demo.floor.warrens-depth-9".to_owned(), None, None, false)
                .unwrap()
                .unwrap();
            let index = game
                .entities
                .iter()
                .position(|actor| actor.id == "demo.guardian.warrens.1")
                .unwrap();
            assert_eq!(game.entities[index].kind_id, kind);
            let position = game.entities[index].position;
            // A deep-waste death must relocate the book to an existing legal tile.
            replace_terrain(&mut game, position, DEEP);
            game.resolve_actor_death(
                index,
                DomainEvent::EntityDiedFromStatus {
                    target_kind_id: kind.to_owned(),
                    status_kind_id: STATUS_POISON.to_owned(),
                    damage: DamageOutcome {
                        raw: 1,
                        armor_reduction: 0,
                        requested: 1,
                        applied: 1,
                        resistance_delta: 0,
                        damage_type: DamageType::Poison,
                        resistance: ResistanceLevel::Normal,
                    },
                },
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
            assert!(game.dungeon_states["demo.dungeon.warrens"].guardian_defeated);
            let campaign = game.snapshot().campaign;
            assert_eq!(campaign.conquered_dungeons, 1);
            assert_eq!(campaign.status, CampaignStatusDto::Active);
            assert_eq!(campaign.score, 10_000);
            let rewards = game
                .items
                .iter()
                .filter(|item| item.kind_id == book)
                .collect::<Vec<_>>();
            assert_eq!(rewards.len(), 1, "{kind} / {build}");
            let ItemLocation::Ground(landing) = rewards[0].location else {
                panic!("book must land");
            };
            assert_ne!(landing, position);
            assert!(game.terrain_allows_items(landing));
            let reward_id = rewards[0].id.clone();
            game.items
                .iter_mut()
                .find(|item| item.id == reward_id)
                .unwrap()
                .location = ItemLocation::Inventory;
            clear_monsters(&mut game);
            let expected_hash = game.state_hash();
            game = Game::from_save_with_content(game.to_save(), content.clone()).unwrap();
            assert_eq!(game.state_hash(), expected_hash);
            let fame = game.fame;
            game.transition_floor("demo.floor.warrens-depth-8".to_owned(), None, None, false)
                .unwrap()
                .unwrap();
            game.transition_floor("demo.floor.warrens-depth-9".to_owned(), None, None, false)
                .unwrap()
                .unwrap();
            assert!(
                game.entities
                    .iter()
                    .all(|actor| actor.id != "demo.guardian.warrens.1")
            );
            assert_eq!(
                game.items
                    .iter()
                    .filter(|item| item.kind_id == book)
                    .count(),
                1
            );
            assert_eq!(game.fame, fame);
            assert_eq!(game.snapshot().campaign.conquered_dungeons, 1);
            let definition = game
                .content
                .world(DEFAULT_WORLD_ID)
                .unwrap()
                .procedural_floors
                .iter()
                .find(|floor| floor.id == "demo.floor.warrens-depth-9")
                .unwrap()
                .clone();
            let regenerated = game.generate_procedural_floor(&definition, None).unwrap();
            assert!(
                regenerated
                    .entities
                    .iter()
                    .all(|actor| actor.kind_id != kind)
            );
            assert!(regenerated.items.iter().all(|item| item.kind_id != book));
        }
    }
}

#[test]
fn dark_cave_disaster_area_entrance_guardians_use_fixed_surface_instances() {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&root).unwrap();
    for (slug, kind) in [
        ("anti-magic-cave", "demo.actor.shadowlord"),
        ("anti-melee-cave", "demo.actor.atomic-elemental"),
    ] {
        artifact.content.worlds[0]
            .dungeons
            .iter_mut()
            .find(|d| d.id == format!("demo.dungeon.{slug}"))
            .unwrap()
            .entrance_guardian
            .as_mut()
            .unwrap()
            .actor_kind_id = kind.to_owned();
    }
    let content = Arc::new(ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    for (seed, slug, kind, position) in [
        (
            1,
            "anti-magic-cave",
            "demo.actor.shadowlord",
            Position { x: 84, y: 6 },
        ),
        (
            785,
            "anti-melee-cave",
            "demo.actor.atomic-elemental",
            Position { x: 47, y: 45 },
        ),
    ] {
        let mut game = Game::from_content_with_build(
            seed,
            content.clone(),
            DEFAULT_WORLD_ID,
            RFB_WARRIOR_BUILD_ID,
        )
        .unwrap();
        choose_human_talent_if_pending(&mut game);
        dispatch_next(
            &mut game,
            GameCommand::EnterWorldMap {
                leave_pets: false,
                cancel_recall: false,
            },
        );
        game.wilderness_position = Some(position);
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        let id = format!("demo.guardian.{slug}-entrance.1");
        let guardian = game.entities.iter().find(|actor| actor.id == id).unwrap();
        assert_eq!(guardian.kind_id, kind);
        assert!(actor_can_cross_terrain(
            game.content.actor(kind).unwrap(),
            game.content
                .terrain(game.terrain_at(guardian.position))
                .unwrap()
        ));
        assert_eq!(
            guardian.pack.as_ref().unwrap().behavior,
            MonsterPackBehaviorDto::GuardPosition
        );
        let hash = game.state_hash();
        game = Game::from_save_with_content(game.to_save(), content.clone()).unwrap();
        assert_eq!(game.state_hash(), hash);
        dispatch_next(
            &mut game,
            GameCommand::EnterWorldMap {
                leave_pets: false,
                cancel_recall: false,
            },
        );
        dispatch_next(&mut game, GameCommand::LeaveWorldMap);
        assert_eq!(
            game.entities
                .iter()
                .filter(|actor| actor.id == id && actor.kind_id == kind)
                .count(),
            1
        );
    }
}

#[test]
fn disaster_area_associated_race_keeps_ownership_exception() {
    let kind = "demo.actor.ogre";
    let mut game = game_with_actor_definition(42, kind, |actor| {
        actor.allocation.as_mut().unwrap().legacy_dungeon_indices = vec![30];
    });
    let actor = game.content.actor(kind).unwrap().clone();
    assert!(!actor.resistances.contains_key(&ActorDamageType::Poison));
    let policy = game
        .content
        .encounter_table("demo.encounter-table.disaster-area")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    assert_eq!(game.original_dungeon_weight(&actor, &policy), 0);
    let mut found = false;
    for _ in 0..256 {
        let selected = game
            .select_original_allocated_monster(
                "demo.floor.warrens-depth-9",
                &policy,
                20,
                20,
                None,
                &[],
                None,
                None,
            )
            .unwrap();
        if selected == kind {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "associated ogre must bypass poison preference in its own dungeon"
    );
    for _ in 0..32 {
        assert_ne!(
            game.select_original_allocated_monster(
                "demo.floor.witch-wood-depth-20",
                &policy,
                20,
                20,
                None,
                &[],
                None,
                None
            )
            .as_deref(),
            Some(kind)
        );
    }
}
