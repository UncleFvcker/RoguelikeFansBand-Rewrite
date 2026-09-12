// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

const DUNGEON: &str = "demo.dungeon.arena";
const ENTRANCE: &str = "demo.guardian.arena-entrance.1";
const GUARDIAN: &str = "demo.guardian.arena.1";
const REWARD: &str = "demo.item.artifact-creation-scroll";

#[test]
fn arena_dungeon_formal_representative_floors_keep_passages_doors_and_working_traps() {
    let mut template = Game::new(42);
    choose_human_talent_if_pending(&mut template);
    let mut doors = 0;
    for depth in [50, 65, 80] {
        let definition = template
            .content
            .world(DEFAULT_WORLD_ID)
            .unwrap()
            .procedural_floors
            .iter()
            .find(|floor| floor.id == format!("demo.floor.arena-depth-{depth}"))
            .unwrap();
        for seed in 0..3 {
            let mut game = template.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(definition, None).unwrap();
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
            let reached = crate::game::world::geometry::maze_floor_distances(
                &traversable,
                floor.player_position,
            );
            assert_eq!(
                reached.len(),
                traversable.len(),
                "depth {depth}, seed {seed}"
            );
            doors += floor
                .terrain
                .iter()
                .filter(|id| *id == &definition.closed_door_terrain_id)
                .count();
            let trap_index = floor
                .terrain
                .iter()
                .position(|id| id == &definition.trap_terrain_id)
                .unwrap();
            let trap = Position {
                x: (trap_index % usize::from(floor.width)) as i32,
                y: (trap_index / usize::from(floor.width)) as i32,
            };
            assert!(reached.contains_key(&trap));
            game.activate_floor(floor, Vec::new());
            clear_monsters(&mut game);
            let (standing, direction) = [
                Direction::North,
                Direction::South,
                Direction::East,
                Direction::West,
            ]
            .into_iter()
            .find_map(|direction| {
                let (dx, dy) = direction.delta();
                let standing = Position {
                    x: trap.x - dx,
                    y: trap.y - dy,
                };
                game.content
                    .terrain(game.terrain_at(standing))
                    .unwrap()
                    .walkable
                    .then_some((standing, direction))
            })
            .unwrap();
            game.player.position = standing;
            let update = dispatch_next(&mut game, GameCommand::Move { direction });
            assert_eq!(game.player.position, trap);
            assert!(
                update
                    .events
                    .iter()
                    .any(|event| event.kind == "terrain.trap-triggered")
            );
        }
    }
    assert!(doors > 0);
}

fn clear_encounters(game: &mut Game) {
    clear_monsters(game);
    // Isolate floor traversal from starvation during the prepared full chain.
    game.nutrition = crate::game::hunger::NUTRITION_FULL;
    // Floor traversal is the subject here; discard incidental monster debuffs.
    game.player.statuses.retain(|status| {
        matches!(
            status.kind_id.as_str(),
            STATUS_INVULNERABILITY | STATUS_HASTE
        )
    });
}

#[track_caller]
fn restore(game: &mut Game) {
    let hash = game.state_hash();
    *game = Game::from_save(game.to_save()).unwrap();
    assert_eq!(game.state_hash(), hash);
}

fn fight_guardian(game: &mut Game, id: &str) -> Actor {
    game.player.hp = game.effective_player_max_hp();
    let guardian = game
        .entities
        .iter()
        .find(|actor| actor.id == id)
        .unwrap()
        .clone();
    game.entities.retain(|actor| actor.id == id);
    game.items.retain(
        |item| !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id != id),
    );
    let mut saw_hit = false;
    for _ in 0..200 {
        let Some(actor) = game.entities.iter().find(|actor| actor.id == id) else {
            assert!(saw_hit);
            return guardian;
        };
        let target = actor.position;
        let (standing, direction) = [
            (
                Position {
                    x: target.x - 1,
                    y: target.y,
                },
                Direction::East,
            ),
            (
                Position {
                    x: target.x + 1,
                    y: target.y,
                },
                Direction::West,
            ),
            (
                Position {
                    x: target.x,
                    y: target.y - 1,
                },
                Direction::South,
            ),
            (
                Position {
                    x: target.x,
                    y: target.y + 1,
                },
                Direction::North,
            ),
        ]
        .into_iter()
        .find(|(position, _)| {
            game.content
                .terrain(game.terrain_at(*position))
                .unwrap()
                .walkable
        })
        .unwrap();
        // Combat fixture: reposition beside the live guardian, preserving its
        // source HP, armor, immunities, AI, and the real command/turn resolver.
        game.player.position = standing;
        game.player.hp = game.effective_player_max_hp();
        game.nutrition = crate::game::hunger::NUTRITION_FULL;
        game.player.statuses.retain(|status| {
            matches!(
                status.kind_id.as_str(),
                STATUS_INVULNERABILITY | STATUS_HASTE
            )
        });
        // Choose a reproducible combat turn that lands and survives. This
        // remains a prepared reward/turn-consumer check, with source guardian
        // HP and AI; it does not rely on the former flat-damage Vorpal bug.
        let seed = (0..1000)
            .find(|seed| {
                let mut trial = game.clone();
                trial.rng = RfbRng::seeded(*seed);
                let update = dispatch_next(&mut trial, GameCommand::Move { direction });
                !trial.player_is_dead()
                    && update.events.iter().any(|event| event.kind == "combat.hit")
            })
            .expect("a prepared combat turn must hit and survive");
        game.rng = RfbRng::seeded(seed);
        let update = dispatch_next(game, GameCommand::Move { direction });
        assert!(
            !game.player_is_dead(),
            "combat fixture died against {id}: {:?}",
            update.events
        );
        saw_hit |= update.events.iter().any(|event| event.kind == "combat.hit");
        choose_human_talent_if_pending(game);
    }
    panic!("guardian {id} survived the bounded combat fixture");
}

#[test]
fn arena_dungeon_real_entry_full_chain_combat_reward_scroll_and_return() {
    let mut game = Game::new_with_build(641, RFB_WARRIOR_BUILD_ID).unwrap();
    choose_human_talent_if_pending(&mut game);
    game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    // Verify traversal and combat consumers independently of a natural leveling run.
    game.apply_player_melee_status(STATUS_INVULNERABILITY, 10_000, "test.arena");
    game.player
        .statuses
        .iter_mut()
        .find(|status| status.kind_id == STATUS_INVULNERABILITY)
        .unwrap()
        .incoming_damage_percent = 0;
    game.apply_player_melee_status(STATUS_HASTE, 10_000, "test.arena");
    game.items.clear();
    give_inventory_item(&mut game, "test.arena.weapon", "demo.item.diamond-edge");
    game.items[0].enchantments.to_hit = 255;
    game.items[0].enchantments.to_damage = 255;
    game.equip_inventory_item("test.arena.weapon", None)
        .unwrap();
    clear_encounters(&mut game);
    let world_position = Position { x: 67, y: 7 };
    assert!(game.dungeon_is_active(DUNGEON));
    assert!(
        game.wilderness_cell_dto(world_position)
            .locations
            .iter()
            .any(|location| location.id == DUNGEON)
    );
    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(world_position);
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(
        game.entities
            .iter()
            .find(|actor| actor.id == ENTRANCE)
            .unwrap()
            .kind_id,
        "demo.actor.drolem"
    );
    restore(&mut game);
    fight_guardian(&mut game, ENTRANCE);
    assert!(game.dungeon_states[DUNGEON].entrance_guardian_defeated);
    assert!(!game.dungeon_states[DUNGEON].guardian_defeated);
    clear_encounters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.arena-entrance");
    let surface_floor = game.current_floor_id.clone();
    let surface_position = game.player.position;
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    let mut visited = BTreeSet::new();
    for depth in 50..=80 {
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.arena-depth-{depth}")
        );
        visited.insert(depth);
        assert_eq!((game.width, game.height), (96, 33));
        assert!(
            game.items
                .iter()
                .all(|item| !matches!(item.location, ItemLocation::Ground(_)))
        );
        assert!(game.gold_piles.is_empty());
        if matches!(depth, 50 | 65 | 80) {
            restore(&mut game);
        }
        if depth == 80 {
            break;
        }
        clear_encounters(&mut game);
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.arena-depth-{}", depth + 1),
            "{:?}",
            update.events
        );
    }
    assert_eq!(visited, (50..=80).collect());
    assert!(
        !game
            .terrain
            .iter()
            .any(|id| id == "demo.terrain.stairs-down" || id == "demo.terrain.shaft-down")
    );
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.id == GUARDIAN)
            .count(),
        1
    );
    let guardian = fight_guardian(&mut game, GUARDIAN);
    assert_eq!(guardian.kind_id, "demo.actor.metal-babble-unique");
    assert!((3..=6).contains(&guardian.max_hp));
    assert!(game.dungeon_states[DUNGEON].guardian_defeated);
    assert_eq!(game.snapshot().campaign.conquered_dungeons, 1);
    assert!(!game.unique_actor_kind_is_available(&guardian.kind_id));
    assert!(
        game.items
            .iter()
            .any(|item| matches!(item.location, ItemLocation::Ground(_)) && item.kind_id != REWARD)
    );
    assert_eq!(
        game.items
            .iter()
            .filter(|item| item.kind_id == REWARD)
            .count(),
        1
    );
    let reward = game
        .items
        .iter()
        .find(|item| item.kind_id == REWARD)
        .unwrap()
        .clone();
    // A stale death notification may still request normal drops, but must never
    // grant the completed dungeon reward a second time.
    let (repeated, _) = game.clone().generate_death_loot(&guardian).unwrap();
    assert!(repeated.iter().all(|item| item.kind_id != REWARD));
    let ItemLocation::Ground(position) = reward.location else {
        panic!("reward must drop on the floor");
    };
    game.player.position = position;
    for _ in 0..game.items.len() {
        if game
            .items
            .iter()
            .any(|item| item.id == reward.id && item.location == ItemLocation::Inventory)
        {
            break;
        }
        dispatch_next(&mut game, GameCommand::PickUp);
    }
    assert!(
        game.items
            .iter()
            .any(|item| item.id == reward.id && item.location == ItemLocation::Inventory)
    );
    restore(&mut game);
    dispatch_next(
        &mut game,
        GameCommand::UseItem {
            item_id: reward.id,
            target: Some(TargetSelection::ArtifactCreationItem {
                item_id: "test.arena.weapon".into(),
                quantity: 1,
                name: Some("竞技场纪念".into()),
            }),
        },
    );
    assert!(!game.items.iter().any(|item| item.kind_id == REWARD));
    assert_eq!(
        game.items
            .iter()
            .find(|item| item.id == "test.arena.weapon")
            .unwrap()
            .artifact_name
            .as_deref(),
        Some("'竞技场纪念'")
    );
    restore(&mut game);
    clear_encounters(&mut game);
    for depth in (50..80).rev() {
        place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert!(
            !game.player_is_dead(),
            "return depth={depth}: {:?}",
            update.events
        );
        assert_eq!(
            game.current_floor_id,
            format!("demo.floor.arena-depth-{depth}")
        );
        clear_encounters(&mut game);
    }
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, surface_floor);
    assert_eq!(game.player.position, surface_position);
    assert!(game.entities.iter().all(|actor| actor.id != ENTRANCE));
    clear_encounters(&mut game);
    restore(&mut game);
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, "demo.floor.arena-depth-80");
    assert!(game.entities.iter().all(|actor| actor.id != GUARDIAN));
    assert!(!game.items.iter().any(|item| item.kind_id == REWARD));
    restore(&mut game);
    game.start_recall(0);
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(game.current_floor_id, surface_floor);
    assert_eq!(game.player.position, surface_position);
    assert_eq!(game.snapshot().campaign.conquered_dungeons, 1);
}
