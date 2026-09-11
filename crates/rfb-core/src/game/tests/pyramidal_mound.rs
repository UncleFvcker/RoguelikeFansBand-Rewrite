// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters};
use super::*;
use crate::game::inventory::ItemIdentificationRequest;
use crate::game::movement::actor_can_cross_terrain;
use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
use rfb_protocol::WeaponTraitDto;

const AMUN: &str = "demo.item.amun";
const DUNGEON: &str = "demo.dungeon.pyramidal-mound";
const TABLE: &str = "demo.encounter-table.pyramidal-mound";
const GUARDIAN: &str = "demo.actor.amun-the-mysterious";

fn floor_id(depth: u16) -> String {
    format!("demo.floor.pyramidal-mound-depth-{depth}")
}

fn generation_game(active: bool) -> Game {
    (0..32)
        .map(|seed| Game::new_with_build(seed, "demo.build.warrior").unwrap())
        .find(|g| (g.active_pantheons & 4 != 0) == active)
        .unwrap()
}

fn floor_definition(game: &Game, depth: u16) -> rfb_content::ProceduralFloorDefinition {
    game.content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|f| f.id == floor_id(depth))
        .unwrap()
        .clone()
}

fn enter_pyramidal_mound_site(game: &mut Game) {
    use super::support::dispatch_next;
    choose_human_talent_if_pending(game);
    dispatch_next(
        game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    // Prepare the overland location; leaving the map and entering the dungeon
    // still go through real commands and the formal wilderness projection.
    game.wilderness_position = Some(Position { x: 77, y: 37 });
    dispatch_next(game, GameCommand::LeaveWorldMap);
}

#[test]
fn pyramidal_mound_formal_entry_shaft_round_trip_uses_rewards_and_restores_recall() {
    use super::support::{dispatch_next, place_player_on_terrain};
    let mut game = generation_game(true);
    let world = game.content.world(&game.world_id).unwrap();
    let dungeon = world.dungeons.iter().find(|d| d.id == DUNGEON).unwrap();
    assert_eq!(dungeon.legacy_index, Some(34));
    assert_eq!(dungeon.pantheon, Some(2));
    assert_eq!(
        world
            .procedural_floors
            .iter()
            .filter(|f| f.dungeon_id.as_deref() == Some(DUNGEON))
            .map(|f| f.depth)
            .collect::<BTreeSet<_>>(),
        (64..=92).collect()
    );
    let other_dungeons = game.dungeon_states.clone();
    let position = Position { x: 77, y: 37 };
    assert!(
        game.wilderness_cell_dto(position)
            .locations
            .iter()
            .any(|l| l.id == DUNGEON)
    );
    enter_pyramidal_mound_site(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.pyramidal-mound-entrance");
    let departure = game.player.position;
    let entrance_id = "demo.guardian.pyramidal-mound-entrance.1";
    let guardian = game.entities.iter().find(|a| a.id == entrance_id).unwrap();
    assert_eq!(guardian.kind_id, "demo.actor.mummy-king");
    assert_eq!(
        guardian.position,
        Position {
            x: departure.x - 1,
            y: departure.y + 1
        }
    );
    game.entities.retain(|a| a.id == entrance_id);
    game.items
        .retain(|i| !matches!(i.location, ItemLocation::CarriedBy { .. }));
    defeat_in_melee(&mut game);
    assert!(game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    choose_human_talent_if_pending(&mut game);
    // Arrival advances the real scheduler: high-depth monsters can attack this
    // low-level traversal fixture before it clears unrelated encounters.
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 10_000, "test.pm.traversal");
    game.player
        .statuses
        .iter_mut()
        .find(|s| s.kind_id == STATUS_INVULNERABILITY)
        .unwrap()
        .incoming_damage_percent = 0;
    place_player_on_terrain(&mut game, "demo.terrain.pyramidal-mound-entrance");
    for depth in (64..=92).step_by(2) {
        let entered = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(entered.floor_id, floor_id(depth));
        assert!(
            !game.player_is_dead(),
            "entry at depth {depth}: {:?}",
            entered.events
        );
        assert_eq!(
            game.entities
                .iter()
                .filter(|a| a.id == "demo.guardian.pyramidal-mound.1")
                .count(),
            usize::from(depth == 92)
        );
        if depth < 92 {
            clear_monsters(&mut game);
            place_player_on_terrain(&mut game, "demo.terrain.shaft-down");
        }
    }
    assert!(
        game.terrain
            .iter()
            .all(|t| t != "demo.terrain.shaft-down" && t != "demo.terrain.stairs-down")
    );
    game.entities
        .retain(|a| a.id == "demo.guardian.pyramidal-mound.1");
    game.items.retain(|i| {
        matches!(
            i.location,
            ItemLocation::Inventory | ItemLocation::Equipped { .. }
        )
    });
    // Isolate the guardian from status effects acquired during prepared travel.
    game.player
        .statuses
        .retain(|s| s.kind_id == STATUS_INVULNERABILITY);
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_SEE_INVISIBLE, 2000, "test.pm.senses").status);
    let killed = defeat_in_melee(&mut game);
    assert_eq!(killed.campaign.conquered_dungeons, 1);
    assert!(game.dungeon_states[DUNGEON].guardian_defeated);
    assert!(game.items.iter().any(|i| !matches!(
        i.kind_id.as_str(),
        AMUN | "demo.item.acquirement-scroll"
    ) && matches!(i.location, ItemLocation::Ground(_))));
    let artifact = pick_up_drop(&mut game, AMUN, GUARDIAN, &killed);
    let scroll = pick_up_drop(&mut game, "demo.item.acquirement-scroll", GUARDIAN, &killed);
    choose_human_talent_if_pending(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: scroll.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|i| i.id == scroll));
    assert!(
        game.items
            .iter()
            .any(|i| i.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
    );
    game.identify_item_instance(&artifact, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&artifact, None).unwrap();
    game.refresh_player_resource_maxima();
    for _ in 0..100 {
        game.use_inventory_item(
            &artifact,
            Some(&TargetSelection::SelfTarget),
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.player_has_telepathy() {
            break;
        }
    }
    assert!(game.player_has_telepathy());
    let saved = game.to_save();
    let mut game = Game::from_save(saved.clone()).unwrap();
    assert_eq!(game.to_save(), saved);
    for depth in (64..92).step_by(2).rev() {
        clear_monsters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.shaft-up");
        assert_eq!(
            dispatch_next(&mut game, GameCommand::TraverseStairs).floor_id,
            floor_id(depth)
        );
    }
    clear_monsters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.wilderness_position, Some(position));
    assert_eq!(game.player.position, departure);
    assert!(game.entities.iter().all(|a| a.id != entrance_id));
    clear_monsters(&mut game);
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, floor_id(92));
    assert!(game.entities.iter().all(|a| a.kind_id != GUARDIAN));
    assert!(game.items.iter().all(|i| i.id != scroll));
    assert_eq!(game.items.iter().filter(|i| i.kind_id == AMUN).count(), 1);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        clear_monsters(current);
        current.start_recall(0);
        dispatch_next(current, GameCommand::Wait);
        assert_eq!(current.wilderness_position, Some(position));
        assert_eq!(current.player.position, departure);
    }
    assert_eq!(game.to_save(), restored.to_save());
    for (id, state) in other_dungeons {
        if id != DUNGEON {
            assert_eq!(game.dungeon_states[&id], state, "{id}");
        }
    }
}

#[test]
fn pyramidal_mound_inactive_pantheon_hides_entry_and_guardian_and_rejects_transition() {
    let mut game = generation_game(false);
    assert!(
        !game
            .wilderness_cell_dto(Position { x: 77, y: 37 })
            .locations
            .iter()
            .any(|l| l.id == DUNGEON)
    );
    enter_pyramidal_mound_site(&mut game);
    assert!(
        game.terrain
            .iter()
            .all(|t| t != "demo.terrain.pyramidal-mound-entrance")
    );
    assert!(
        game.entities
            .iter()
            .all(|a| a.id != "demo.guardian.pyramidal-mound-entrance.1")
    );
    assert!(
        game.transition_floor(floor_id(64), None, None, false)
            .unwrap()
            .is_none()
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(
        restored
            .transition_floor(floor_id(92), None, None, false)
            .unwrap()
            .is_none()
    );
    assert!(restored.dungeon_states[DUNGEON].suppressed);
    assert!(!restored.dungeon_states[DUNGEON].guardian_defeated);
}

#[test]
fn pyramidal_mound_representative_maps_keep_materials_routes_and_legal_spawns() {
    let base = generation_game(true);
    let mut materials = BTreeSet::new();
    let mut items = 0;
    let mut gold = 0;
    let mut primary = false;
    let mut secondary = false;
    let mut ordinary = false;
    for depth in [64, 78, 92] {
        let definition = floor_definition(&base, depth);
        for seed in [2, 7, 42] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            assert_eq!((floor.width, floor.height), (96, 33));
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
                    let terrain = game.content.terrain(id).unwrap();
                    (terrain.walkable || terrain.open_to_terrain_id.is_some()).then_some(Position {
                        x: (i % usize::from(floor.width)) as i32,
                        y: (i / usize::from(floor.width)) as i32,
                    })
                })
                .collect::<BTreeSet<_>>();
            let reached = maze_floor_distances(&walkable, floor.player_position);
            assert_eq!(
                reached.len(),
                walkable.len(),
                "depth {depth}, seed {seed}: all walkable cells connect through openable doors"
            );
            let mut occupied = BTreeSet::from([floor.player_position]);
            for c in &floor.connections {
                assert!(
                    reached.contains_key(&c.position),
                    "depth {depth}, seed {seed}, {c:?}"
                );
                occupied.insert(c.position);
            }
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.closed_door_terrain_id)
            );
            assert!(
                floor
                    .terrain
                    .iter()
                    .any(|id| id == &definition.trap_terrain_id)
            );
            materials.extend(floor.terrain.iter().cloned());
            assert!(!floor.entities.is_empty());
            for actor in &floor.entities {
                let kind = game.content.actor(&actor.kind_id).unwrap();
                assert!(occupied.insert(actor.position));
                assert!(actor_can_cross_terrain(kind, at(actor.position)));
                if actor.kind_id == GUARDIAN {
                    assert_eq!(depth, 92);
                    assert_eq!(actor.id, "demo.guardian.pyramidal-mound.1");
                    assert_eq!(kind.level, 97);
                    assert!(reached.contains_key(&actor.position));
                } else {
                    assert!(game.pantheon_allows_allocation(&definition.id, kind));
                    primary |= kind.tags.iter().any(|t| t == "egyptian");
                    secondary |= kind.tags.iter().any(|t| t == "egyptian2");
                    ordinary |= !kind
                        .tags
                        .iter()
                        .any(|t| t == "egyptian" || t == "egyptian2");
                }
            }
            assert_eq!(
                floor
                    .entities
                    .iter()
                    .filter(|a| a.kind_id == GUARDIAN)
                    .count(),
                usize::from(depth == 92)
            );
            assert!(floor.items.iter().all(|item| match item.location {
                ItemLocation::Ground(p) => at(p).allows_items(),
                _ => false,
            }));
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            items += floor.items.len();
            gold += floor.gold_piles.len();
        }
    }
    for id in [
        "demo.terrain.wall",
        "demo.terrain.quartz-vein",
        "demo.terrain.magma-vein",
    ] {
        assert!(materials.contains(id), "missing {id}");
    }
    assert!(
        !materials
            .iter()
            .any(|id| id.contains("water") || id.contains("waste"))
    );
    assert!(items > 0 && gold > 0, "items={items}, gold={gold}");
    assert!(
        primary && secondary && ordinary,
        "primary={primary}, secondary={secondary}, ordinary={ordinary}"
    );
}

#[test]
fn pyramidal_mound_shafts_stop_at_surface_and_final_floor() {
    let base = generation_game(true);
    for (depth, targets) in [
        (64, vec![0, 66]),
        (65, vec![0, 67]),
        (78, vec![76, 80]),
        (91, vec![89, 92]),
        (92, vec![90, 91]),
    ] {
        let mut game = base.clone();
        let definition = floor_definition(&game, depth);
        let floor = game.generate_procedural_floor(&definition, None).unwrap();
        game.activate_floor(floor, Vec::new());
        let mut actual = Vec::new();
        for c in &game.floor_connections {
            let target_id = c.target_floor_id.as_ref().unwrap();
            let target = if target_id == "demo.floor.surface" {
                0
            } else {
                target_id
                    .rsplit('-')
                    .next()
                    .unwrap()
                    .parse::<u16>()
                    .unwrap()
            };
            actual.push(target);
            let terrain = game.content.terrain(game.terrain_at(c.position)).unwrap();
            assert_eq!(
                terrain.tags.iter().any(|t| t == "shaft"),
                !(depth == 64 && target == 0 || depth.abs_diff(target) == 1)
            );
            if target != 0 {
                let mut traversed = game.clone();
                let update = traversed
                    .transition_floor(
                        target_id.clone(),
                        c.target_connection_id.clone(),
                        None,
                        false,
                    )
                    .unwrap();
                assert!(update.is_some());
                assert_eq!(traversed.current_floor_id, floor_id(target));
                let return_link = traversed
                    .floor_connections
                    .iter()
                    .find(|r| Some(&r.id) == c.target_connection_id.as_ref())
                    .unwrap();
                assert_eq!(traversed.player.position, return_link.position);
                assert_eq!(return_link.target_floor_id.as_ref(), Some(&definition.id));
            }
        }
        actual.sort_unstable();
        assert_eq!(actual, targets);
    }
}

#[test]
fn pyramidal_mound_ecology_keeps_rarity_divisor_and_egyptian_qualification() {
    let mut game = generation_game(true);
    let policy = game
        .content
        .encounter_table(TABLE)
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    for id in [
        "demo.actor.osiris-the-reborn",
        "demo.actor.mummy-king",
        "demo.actor.the-phoenix",
    ] {
        let actor = game.content.actor(id).unwrap().clone();
        let before = game.rng.clone();
        assert_eq!(
            game.original_dungeon_weight(&actor, &policy),
            100 / actor.allocation.as_ref().unwrap().rarity
        );
        assert_eq!(game.rng, before);
    }
    let newt = game.content.actor("demo.actor.newt").unwrap().clone();
    let base_weight = 100 / newt.allocation.as_ref().unwrap().rarity;
    for round_up in [false, true] {
        let seed = (0..1000)
            .find(|seed| {
                (RfbRng::seeded(*seed).bounded(64) < u64::from(base_weight % 64)) == round_up
            })
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        game.monster_division_remainders.clear();
        assert_eq!(
            game.original_dungeon_weight(&newt, &policy),
            base_weight / 64 + u32::from(round_up)
        );
        let after = game.rng.clone();
        game.original_dungeon_weight(&newt, &policy);
        assert_eq!(
            game.rng, after,
            "fractional division is stable within this allocation table"
        );
    }
    for active in [true, false] {
        let mut game = generation_game(active);
        let osiris = game.content.actor("demo.actor.osiris-the-reborn").unwrap();
        let bast = game
            .content
            .actor("demo.actor.bast-goddess-of-cats")
            .unwrap();
        let mummy = game.content.actor("demo.actor.mummy-king").unwrap();
        assert_eq!(
            game.pantheon_allows_allocation(&floor_id(78), osiris),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.castle-depth-40", osiris));
        assert_eq!(
            game.pantheon_allows_allocation("demo.floor.castle-depth-40", bast),
            active
        );
        assert!(!game.pantheon_allows_allocation("demo.floor.mount-olympus-depth-80", bast));
        assert!(game.pantheon_allows_allocation("demo.floor.castle-depth-40", mummy));
        // Ordinary secondary-tagged monsters stay eligible when Egypt is inactive.
        let mut seen = BTreeSet::new();
        for _ in 0..64 {
            let id = game
                .select_original_allocated_monster(
                    &floor_id(78),
                    &policy,
                    92,
                    78,
                    None,
                    &[],
                    None,
                    None,
                )
                .unwrap();
            assert_ne!(id, GUARDIAN);
            assert!(
                game.pantheon_allows_allocation(&floor_id(78), game.content.actor(&id).unwrap())
            );
            seen.insert(id);
        }
        assert!(seen.iter().any(|id| {
            game.content
                .actor(id)
                .unwrap()
                .tags
                .iter()
                .any(|t| t == "egyptian2")
        }));
        if !active {
            assert!(seen.iter().all(|id| {
                !game
                    .content
                    .actor(id)
                    .unwrap()
                    .tags
                    .iter()
                    .any(|t| t == "egyptian")
            }));
            assert!(
                game.transition_floor(floor_id(78), None, None, false)
                    .unwrap()
                    .is_none()
            );
        }
        // Use a non-pantheon dungeon for the inactive request's unique category.
        game.current_floor_id = if active {
            floor_id(78)
        } else {
            "demo.floor.castle-depth-40".into()
        };
        let candidates =
            game.summon_category_candidate_kind_ids("egyptian", None, 100, true, false);
        assert!(!candidates.is_empty());
        for id in &candidates {
            let actor = game.content.actor(id).unwrap();
            assert!(actor.tags.iter().any(|t| t == "unique"));
            assert_eq!(actor.tags.iter().any(|t| t == "egyptian"), active);
        }
        // Prepared open space isolates the actual random-summon consumer.
        let mut casting = game.clone();
        clear_monsters(&mut casting);
        casting.terrain.fill("demo.terrain.floor".into());
        casting.player.position = Position { x: 80, y: 20 };
        casting.push_generated_actor(
            "test.amun-caster".into(),
            GUARDIAN,
            Position { x: 20, y: 20 },
        );
        let ability = casting
            .content
            .ability("rfb-legacy.ability.summon-egyptian-l97-1d2")
            .unwrap()
            .clone();
        let plan = casting.monster_ability_target_plan(0, ability, 1).unwrap();
        let summoned = casting
            .resolve_monster_ability_plan(
                0,
                GUARDIAN,
                &plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .summon
            .unwrap();
        assert!((1..=2).contains(&summoned.summoned_kind_ids.len()));
        for id in &summoned.summoned_kind_ids {
            assert!(candidates.contains(id));
            assert_ne!(
                id, GUARDIAN,
                "the existing caster consumes its unique allowance"
            );
            assert!(
                casting
                    .entities
                    .iter()
                    .any(|a| &a.kind_id == id && a.summon.is_some())
            );
        }
        if active {
            assert!(candidates.iter().any(|id| id == GUARDIAN));
            game.defeated_limited_actor_counts
                .insert(GUARDIAN.into(), 1);
            assert!(
                !game
                    .summon_category_candidate_kind_ids("egyptian", None, 100, true, false)
                    .iter()
                    .any(|id| id == GUARDIAN)
            );
        }
    }
    let definition = floor_definition(&game, 78);
    let floor = game.generate_procedural_floor(&definition, None).unwrap();
    game.activate_floor(floor, Vec::new());
    clear_monsters(&mut game);
    let chance = u64::from(policy.ambient_chance_one_in) * 178 / 100;
    let seed = (0..10000)
        .find(|seed| RfbRng::seeded(*seed).bounded(chance) == 0)
        .unwrap();
    game.rng = RfbRng::seeded(seed);
    game.process_ambient_monster_allocation(&mut BTreeSet::new())
        .unwrap();
    assert!(!game.entities.is_empty());
    for actor in &game.entities {
        assert_ne!(actor.kind_id, GUARDIAN);
        assert!(game.pantheon_allows_allocation(
            &definition.id,
            game.content.actor(&actor.kind_id).unwrap()
        ));
    }
}

fn artifact_context() -> LootContext {
    LootContext {
        table_id: "demo.loot-table.base-items".into(),
        floor_id: "test.floor.depth-60".into(),
        depth: 60,
        source: LootSource::MonsterDeath {
            actor_id: "test.ordinary-drop".into(),
        },
    }
}

fn battle_game() -> Game {
    let mut game = generation_game(true);
    choose_human_talent_if_pending(&mut game);
    game.transition_floor(floor_id(78), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    game.player.position = melee_position(&game);
    game
}

fn melee_position(game: &Game) -> Position {
    (1..game.height - 1)
        .find_map(|y| {
            (1..game.width - 2)
                .map(|x| Position {
                    x: x.into(),
                    y: y.into(),
                })
                .find(|p| game.is_walkable(*p) && game.is_walkable(Position { x: p.x + 1, y: p.y }))
        })
        .unwrap()
}

fn battle_actor(game: &Game, kind: &str, id: &str) -> Actor {
    spawn_actor_from_definition(
        &mut RfbRng::seeded(0),
        game.content.actor(kind).unwrap(),
        id,
        Position {
            x: game.player.position.x + 1,
            y: game.player.position.y,
        },
        100_000,
        true,
    )
}

fn defeat_in_melee(game: &mut Game) -> GameUpdate {
    // Prepare one target HP, ample player HP and a delayed turn, keeping real
    // defenses/contact auras. Restore player HP to its legal maximum for saves.
    game.player.position = melee_position(game);
    game.entities[0].position = Position {
        x: game.player.position.x + 1,
        y: game.player.position.y,
    };
    game.player.hp = 100_000;
    game.entities[0].hp = 1;
    game.entities[0].energy_need = 100_000;
    game.entities[0].nice = true;
    let base = game.clone();
    for seed in 0..256 {
        let mut attempt = base.clone();
        attempt.rng = RfbRng::seeded(seed);
        let update = super::support::dispatch_next(
            &mut attempt,
            GameCommand::Move {
                direction: Direction::East,
            },
        );
        if attempt.entities.is_empty() {
            attempt.player.hp = attempt.player.max_hp;
            *game = attempt;
            return update;
        }
    }
    panic!("prepared guardian never died in melee");
}

fn pick_up_drop(game: &mut Game, kind: &str, source: &str, update: &GameUpdate) -> String {
    assert!(update.events.iter().any(|e| e.kind == "loot.drop"
        && e.args.get("source").map(String::as_str) == Some(source)
        && e.args.get("target").map(String::as_str) == Some(kind)));
    let item = game.items.iter().find(|i| i.kind_id == kind).unwrap();
    let id = item.id.clone();
    let ItemLocation::Ground(position) = item.location else {
        panic!("drop must reach the floor")
    };
    game.player.position = position;
    game.pick_up_item_at_player(Some(&id)).unwrap();
    assert_eq!(
        game.items.iter().find(|i| i.id == id).unwrap().location,
        ItemLocation::Inventory
    );
    id
}

#[test]
fn pyramidal_mound_guardians_melee_uses_true_identity_and_mummy_instance_accounting() {
    let base = battle_game();
    for (kind, level) in [("demo.actor.mummy-king", 56), (GUARDIAN, 97)] {
        let mut game = base.clone();
        let mut actor = battle_actor(&game, kind, "test.pm.incoming");
        if kind == GUARDIAN {
            actor.appearance_kind_id = Some("demo.actor.goblin".into());
        }
        assert_eq!(game.actor_runtime_definition(&actor).unwrap().level, level);
        game.entities.push(actor);
        game.player.hp = 100_000;
        let target = MonsterHostileTarget::Player {
            entity_id: game.player.id.clone(),
            kind_id: game.player.kind_id.clone(),
            position: game.player.position,
        };
        for _ in 0..8 {
            game.resolve_monster_melee_target(
                0,
                &target,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
        }
        assert!(game.player.hp < 100_000, "{kind}");
    }
    let mut game = base;
    // Instance accounting is independent of the final guardian and global uniques.
    let surface = game
        .content
        .world(&game.world_id)
        .unwrap()
        .initial_floor_id
        .clone();
    game.transition_floor(surface, None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.entities.push(battle_actor(
        &game,
        "demo.actor.mummy-king",
        "test.pm.ordinary-mummy-king",
    ));
    let ordinary = defeat_in_melee(&mut game);
    assert!(!game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(
        !ordinary
            .events
            .iter()
            .any(|e| e.kind == "dungeon.entrance-guardian-defeated")
    );
    choose_human_talent_if_pending(&mut game);
    let actor = battle_actor(
        &game,
        "demo.actor.mummy-king",
        "demo.guardian.pyramidal-mound-entrance.1",
    );
    game.entities.push(actor);
    defeat_in_melee(&mut game);
    assert!(game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    assert!(game.unique_actor_kind_is_available("demo.actor.mummy-king"));
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn pyramidal_mound_amun_early_melee_rewards_are_picked_up_used_and_saved_once() {
    let mut game = battle_game();
    let mut actor = battle_actor(&game, GUARDIAN, "test.pm.early-amun");
    actor.appearance_kind_id = Some("demo.actor.goblin".into());
    game.entities.push(actor);
    // Prepared see-invisible status exercises perception without generating the reward early.
    game.glow.fill(true);
    game.refresh_invisible_visibility(true, &BTreeMap::new());
    assert!(!game.entity_is_visible_to_player(&game.entities[0]));
    game.player
        .statuses
        .push(monster_combat::melee_status(STATUS_SEE_INVISIBLE, 2000, "test.pm.senses").status);
    assert!(game.player_see_invisible_sources() > 0);
    for _ in 0..100 {
        game.refresh_invisible_visibility(true, &BTreeMap::new());
        if game.entity_is_visible_to_player(&game.entities[0]) {
            break;
        }
    }
    assert!(game.entity_is_visible_to_player(&game.entities[0]));
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.bad-luck".into());
    let update = defeat_in_melee(&mut game);
    assert!(
        update
            .events
            .iter()
            .any(|e| e.kind == "dungeon.guardian-defeated")
    );
    assert!(game.dungeon_states[DUNGEON].guardian_defeated);
    assert!(!game.unique_actor_kind_is_available(GUARDIAN));
    assert_eq!(
        game.items
            .iter()
            .filter(|i| i.kind_id == "demo.item.acquirement-scroll")
            .count(),
        1
    );
    assert!(
        game.items
            .iter()
            .any(|i| !matches!(i.kind_id.as_str(), AMUN | "demo.item.acquirement-scroll"))
    );
    let artifact = pick_up_drop(&mut game, AMUN, GUARDIAN, &update);
    let scroll = pick_up_drop(&mut game, "demo.item.acquirement-scroll", GUARDIAN, &update);
    choose_human_talent_if_pending(&mut game);
    super::support::dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: scroll.clone(),
            target: None,
        },
    );
    assert!(!game.items.iter().any(|i| i.id == scroll));
    assert!(
        game.items
            .iter()
            .any(|i| i.origin_kind == Some(rfb_protocol::ItemOriginKindDto::Acquire))
    );
    game.identify_item_instance(&artifact, ItemIdentificationRequest::new(true));
    game.equip_inventory_item(&artifact, None).unwrap();
    game.refresh_player_resource_maxima();
    for _ in 0..100 {
        game.use_inventory_item(
            &artifact,
            Some(&TargetSelection::SelfTarget),
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.player_has_telepathy() {
            break;
        }
    }
    assert!(game.player_has_telepathy());
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.generated_artifact_ids.contains(AMUN));
    let rewards = restored.items.iter().filter(|i| i.kind_id == AMUN).count();
    restored
        .transition_floor(floor_id(92), None, None, false)
        .unwrap()
        .unwrap();
    assert!(restored.entities.iter().all(|a| a.kind_id != GUARDIAN));
    assert!(restored.dungeon_states[DUNGEON].guardian_defeated);
    assert_eq!(
        restored.items.iter().filter(|i| i.kind_id == AMUN).count(),
        rewards
    );
}

#[test]
fn pyramidal_mound_special_drops_keep_guarantee_exclusions_and_outside_death() {
    let base = battle_game();
    for (kind, item) in [
        (GUARDIAN, AMUN),
        ("demo.actor.osiris-the-reborn", "demo.item.new-life-potion"),
    ] {
        let mut game = base.clone();
        game.progress
            .active_mutation_ids
            .insert("rfb.mutation.bad-luck".into());
        let actor = battle_actor(&game, kind, "test.pm.drop");
        game.entities.push(actor.clone());
        let (drops, _) = game.generate_death_loot(&actor).unwrap();
        assert_eq!(
            drops
                .iter()
                .filter(|i| i.kind_id == item)
                .map(|i| i.quantity)
                .sum::<u32>(),
            1
        );
        assert!(
            drops
                .iter()
                .any(|i| i.kind_id != item && i.kind_id != "demo.item.acquirement-scroll")
        );
        if kind == GUARDIAN {
            assert!(
                game.generate_death_loot(&actor)
                    .unwrap()
                    .0
                    .iter()
                    .all(|i| i.kind_id != AMUN)
            );
        }
        let mut pet_game = base.clone();
        let mut pet = battle_actor(&pet_game, kind, "test.pm.pet");
        pet.controller_id = Some(pet_game.player.id.clone());
        assert!(
            pet_game
                .generate_death_loot(&pet)
                .unwrap()
                .0
                .iter()
                .all(|i| i.kind_id != item)
        );
    }
    let mut game = base;
    let surface = game
        .content
        .world(&game.world_id)
        .unwrap()
        .initial_floor_id
        .clone();
    game.transition_floor(surface.clone(), None, None, false)
        .unwrap()
        .unwrap();
    game.transition_floor("demo.floor.warrens-depth-3".into(), None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    let actor = battle_actor(&game, GUARDIAN, "test.pm.outside");
    game.entities.push(actor);
    super::world::defeat_guardian_with_status(&mut game, "test.pm.outside", STATUS_POISON);
    assert!(game.items.iter().any(|i| i.kind_id == AMUN));
    assert!(
        game.items
            .iter()
            .all(|i| i.kind_id != "demo.item.acquirement-scroll")
    );
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    game.transition_floor(surface, None, None, false)
        .unwrap()
        .unwrap();
    game.transition_floor(floor_id(92), None, None, false)
        .unwrap()
        .unwrap();
    assert!(game.entities.iter().all(|a| a.kind_id != GUARDIAN));
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
}

#[test]
fn pyramidal_mound_phoenix_rebirth_keeps_melee_and_status_targets_alive() {
    let mut base = battle_game();
    let kind = "demo.actor.the-phoenix";
    let mut actor = battle_actor(&base, kind, "test.pm.phoenix");
    actor.hp = 1;
    actor.energy_need = INITIAL_MONSTER_ENERGY_NEED;
    base.entities.push(actor);
    base.player.hp = 100_000;
    for from_status in [false, true] {
        let mut reborn = None;
        for seed in 0..256 {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let mut events = Vec::new();
            let mut removed = Vec::new();
            if from_status {
                let mut status =
                    monster_combat::melee_status(STATUS_BLEEDING, 10, &game.player.id).status;
                status.intensity = 3;
                game.entities[0].statuses.push(status);
                game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut removed, true)
                    .unwrap();
            } else {
                game.resolve_player_melee(
                    0,
                    false,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut removed,
                )
                .unwrap();
            }
            if events
                .iter()
                .any(|e| matches!(e, DomainEvent::PhoenixReborn { .. }))
            {
                assert!(removed.is_empty());
                assert!(game.entities[0].hp > 0);
                assert_eq!(game.progress.experience, base.progress.experience);
                assert!(game.items.is_empty());
                assert!(!game.defeated_limited_actor_counts.contains_key(kind));
                game.player.hp = game.player.max_hp;
                reborn = Some(game);
                break;
            }
        }
        let game = reborn.expect("one-third revival must be reachable");
        let mut restored =
            Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        // A later actual fatal blow still reaches ordinary death and rewards.
        defeat_in_melee(&mut restored);
        assert!(!restored.unique_actor_kind_is_available(kind));
        assert!(restored.progress.experience > game.progress.experience);
        assert!(!restored.items.is_empty());
    }
}

#[test]
fn pyramidal_mound_osiris_extra_potion_is_consumed_after_real_death_and_pickup() {
    let mut game = battle_game();
    // The extra drop also works outside its pantheon dungeon.
    let surface = game
        .content
        .world(&game.world_id)
        .unwrap()
        .initial_floor_id
        .clone();
    game.transition_floor(surface, None, None, false)
        .unwrap()
        .unwrap();
    clear_monsters(&mut game);
    game.items.clear();
    let kind = "demo.actor.osiris-the-reborn";
    game.entities
        .push(battle_actor(&game, kind, "test.pm.osiris"));
    let update = defeat_in_melee(&mut game);
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    assert!(
        game.items
            .iter()
            .any(|i| i.kind_id != "demo.item.new-life-potion")
    );
    let id = pick_up_drop(&mut game, "demo.item.new-life-potion", kind, &update);
    let mut restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    for current in [&mut game, &mut restored] {
        choose_human_talent_if_pending(current);
        current.progress.life_force = 125;
        super::support::dispatch_next(
            current,
            GameCommand::UseItem {
                item_id: id.clone(),
                target: None,
            },
        );
        assert!(!current.items.iter().any(|i| i.id == id));
        assert_eq!(current.progress.life_force, 1000);
    }
    assert_eq!(restored.to_save(), game.to_save());
}

#[test]
fn pyramidal_mound_amun_uses_normal_artifact_rarity_and_unique_registration() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    let context = artifact_context();
    // Exercise the normal fixed-artifact selector after an ordinary dagger base
    // was selected. No Amun death reward or guaranteed-artifact table is used.
    for (roll, expected) in [(1, None), (119, None), (0, Some(AMUN))] {
        let seed = (0..10_000)
            .find(|seed| RfbRng::seeded(*seed).bounded(120) == roll)
            .unwrap();
        game.rng = RfbRng::seeded(seed);
        assert_eq!(
            game.roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
                .as_deref(),
            expected
        );
        assert_eq!(game.rng.draw_counter, 1);
    }
    let draft = game.fixed_item_draft(&context, AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Inventory)
        .unwrap();
    assert!(item.affix_ids.is_empty() && item.rolled_affixes.is_empty());
    assert!(item.curse.is_none());
    game.items.push(item);
    assert!(game.generated_artifact_ids.contains(AMUN));
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    let rng = restored.rng.clone();
    assert!(
        restored
            .roll_fixed_artifact_kind_id(&context, Some("demo.item.dagger"), false)
            .is_none()
    );
    assert_eq!(restored.rng, rng);
}

#[test]
fn pyramidal_mound_amun_equips_senses_expires_and_recovers_across_save() {
    let mut game = Game::new_with_build(67, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    clear_monsters(&mut game);
    game.items.clear();
    let draft = game.fixed_item_draft(&artifact_context(), AMUN.into());
    let item = game
        .commit_generated_item_draft(draft, ItemLocation::Ground(game.player.position))
        .unwrap();
    let id = item.id.clone();
    game.items.push(item);
    game.pick_up_item_at_player(Some(&id)).unwrap();
    game.identify_item_instance(&id, ItemIdentificationRequest::new(true));
    let before = game.player_derived_stats();
    let before_intelligence = game.equipment_modifiers().intelligence;
    let before_see_invisible = game.player_see_invisible_sources();
    game.equip_inventory_item(&id, None).unwrap();
    let after = game.player_derived_stats();
    assert_eq!(
        game.equipment_modifiers().intelligence,
        before_intelligence + 5
    );
    assert_eq!(after.speed.value, before.speed.value + 5);
    assert_eq!(after.stealth_skill.value, before.stealth_skill.value + 5);
    assert_eq!(after.search_skill.value, before.search_skill.value + 25);
    assert_eq!(
        after.perception_skill.value,
        before.perception_skill.value + 25
    );
    assert_eq!(
        game.player_see_invisible_sources(),
        before_see_invisible + 1
    );
    assert!(game.player_status_immunities().contains(STATUS_PARALYSIS));
    assert_eq!(
        game.effective_player_resistances().level(DamageType::Fire),
        ResistanceLevel::Immune
    );
    for damage in [
        DamageType::Poison,
        DamageType::Light,
        DamageType::Electricity,
        DamageType::Sound,
    ] {
        assert_eq!(
            game.effective_player_resistances().level(damage),
            ResistanceLevel::Resistant
        );
    }
    let weapon = &game.items[0];
    let melee = game.item_melee_profile(weapon).unwrap();
    assert_eq!((melee.damage.dice, melee.damage.sides), (3, 5));
    assert!(game.item_has_weapon_trait(weapon, WeaponTraitDto::Blessed));
    // Fixed definitions use the vorpal flag; item_has_weapon_trait reads
    // rolled/intrinsic traits instead (covered by the existing melee tests).
    assert!(game.content.item(&weapon.kind_id).unwrap().vorpal);
    assert_eq!(
        weapon.activation.as_ref().unwrap().device_check_difficulty,
        60
    );
    assert!(!game.player_has_telepathy());
    // A prepared ordinary monster exercises the actual sensing consumer.
    game.push_generated_actor(
        "test.esp-target".into(),
        "demo.actor.goblin",
        game.player.position,
    );
    assert!(!game.entity_is_visible_by_telepathy(&game.entities[0]));
    let mut boosted = game.clone();
    let mut power = monster_combat::melee_status(STATUS_HASTE, 2_000, "test.device-power").status;
    power.granted_modifiers.device_power_bonus = 5;
    boosted.player.statuses.push(power);
    let mut events = Vec::new();
    for current in [&mut game, &mut boosted] {
        for _ in 0..100 {
            current
                .use_inventory_item(
                    &id,
                    Some(&TargetSelection::SelfTarget),
                    None,
                    &mut events,
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                )
                .unwrap();
            if current.items[0].charges.unwrap().current == 0 {
                break;
            }
        }
        assert_eq!(current.items[0].charges.unwrap().current, 0);
    }
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    assert!(game.entity_is_visible_by_telepathy(&game.entities[0]));
    let duration = game
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    assert!(
        (270..=560).contains(&duration) && duration % 10 == 0,
        "source 1d30+25 turns plus the item's immediate action window: {duration}"
    );
    let boosted_duration = boosted
        .player
        .statuses
        .iter()
        .find(|s| s.kind_id == STATUS_TELEPATHY)
        .unwrap()
        .remaining_ticks;
    let source_turns = duration / 10 - 1;
    assert_eq!(
        boosted_duration,
        (source_turns + source_turns * 5 / 20 + 1) * 10
    );
    assert_eq!(
        boosted.rng, game.rng,
        "device power must not change the duration roll"
    );
    // Remove the prepared overlapping target before testing a valid save.
    clear_monsters(&mut game);
    let state = game.to_save();
    game.use_inventory_item(
        &id,
        Some(&TargetSelection::SelfTarget),
        None,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(
        game.to_save(),
        state,
        "cooldown cannot consume RNG or reapply ESP"
    );
    for tick in 1..=100 {
        game.world_tick = tick;
        game.process_status_tick(&mut events, &mut BTreeSet::new(), &mut Vec::new(), false)
            .unwrap();
        game.process_inventory_device_recovery(&mut events);
    }
    game.reveal_current_visibility();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.player_has_telepathy());
    assert_eq!(restored.items[0].device_recovery_progress, 100);
    for tick in 101..=1000 {
        for current in [&mut game, &mut restored] {
            current.world_tick = tick;
            current
                .process_status_tick(
                    &mut Vec::new(),
                    &mut BTreeSet::new(),
                    &mut Vec::new(),
                    false,
                )
                .unwrap();
            current.process_inventory_device_recovery(&mut Vec::new());
            if tick == duration {
                assert!(!current.player_has_telepathy());
            }
            if tick == 999 {
                assert_eq!(current.items[0].charges.unwrap().current, 0);
                assert_eq!(current.items[0].device_recovery_progress, 999);
            }
        }
    }
    assert_eq!(restored.items[0].charges.unwrap().current, 1);
    assert_eq!(restored.items[0].device_recovery_progress, 0);
    assert_eq!(restored.to_save(), game.to_save());
    assert_eq!(restored.rng, game.rng);
    for current in [&mut game, &mut restored] {
        current
            .use_inventory_item(
                &id,
                Some(&TargetSelection::SelfTarget),
                None,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .unwrap();
    }
    assert_eq!(restored.to_save(), game.to_save());
    let slot = match &restored.items[0].location {
        ItemLocation::Equipped { slot_id } => slot_id.clone(),
        _ => panic!("Amun must remain equipped"),
    };
    restored.unequip_slot(&slot).unwrap();
    assert_eq!(
        restored.equipment_modifiers().intelligence,
        before_intelligence
    );
}
