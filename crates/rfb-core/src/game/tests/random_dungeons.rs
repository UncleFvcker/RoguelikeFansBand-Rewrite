// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::floor::RecallUseAction;
use rfb_content::ItemUseEffectDefinition;
use rfb_protocol::FacilityServiceKindDto;

const FOREST: &str = "demo.dungeon.random-forest";
const ENTRANCE: &str = "demo.terrain.random-forest-entrance";

#[test]
fn desktop_preparation_uses_formal_generation_and_rejects_wrong_locations() {
    for kind in ["forest", "volcano", "mountain", "sea"] {
        let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
        choose_human_talent_if_pending(&mut game);
        assert!(
            game.debug_prepare_random_dungeon_e2e(kind, "stairs")
                .is_err()
        );
        game.debug_prepare_random_dungeon_e2e(kind, "arrival")
            .unwrap();
        eprintln!(
            "RD6 {kind}: seed={} position={:?}",
            game.wilderness_seed, game.wilderness_position
        );
        assert_eq!(
            game.terrain_at(game.player.position),
            format!("demo.terrain.random-{kind}-entrance")
        );
        let departure = game.player.position;
        let world = game.wilderness_position;
        restore(&game);
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert!(
            game.current_floor_id
                .starts_with(&format!("demo.floor.random-{kind}-depth-"))
        );
        assert!(game.current_dungeon_instance_id.is_some());
        assert!(
            game.debug_prepare_random_dungeon_e2e(kind, "arrival")
                .is_err()
        );
        game.debug_prepare_random_dungeon_e2e(kind, "stairs")
            .unwrap();
        restore(&game);
        dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(game.wilderness_position, world);
        assert_eq!(game.player.position, departure);
        restore(&game);
    }
}

fn floor_id(depth: u16) -> String {
    format!("demo.floor.random-forest-depth-{depth}")
}

fn prepared_entry() -> Game {
    let mut game = Game::new_with_build(29, "demo.build.warrior").unwrap();
    wilderness::prepare_random_entry_for_test(
        &mut game,
        "demo.wilderness-encounter.trees-random-forest-level",
    );
    assert_eq!(game.terrain_at(game.player.position), ENTRANCE);
    game
}

fn seed_for_roll(bound: u64, value: u64) -> u64 {
    (0..10_000)
        .find(|seed| RfbRng::seeded(*seed).bounded(bound) == value)
        .unwrap()
}

fn enter_at_depth(game: &mut Game, depth: u16) {
    game.rng = RfbRng::seeded(seed_for_roll(26, u64::from(depth - 25)));
    assert!(game.traverse_stairs(false).unwrap().is_some());
    assert_eq!(game.current_floor_id, floor_id(depth));
}

#[track_caller]
fn restore(game: &Game) -> Game {
    let bytes = rfb_protocol::to_msgpack(&game.to_save()).unwrap();
    let payload = rfb_protocol::from_msgpack(&bytes).unwrap();
    let restored = Game::from_save_with_content(payload, game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored
}

fn resolve_spell_effect(game: &mut Game, id: &str) -> Vec<DomainEvent> {
    let ability = game.content.ability(id).unwrap().clone();
    let plan = game
        .ability_target_plan(&ability, &TargetSelection::SelfTarget)
        .unwrap();
    let mut events = Vec::new();
    game.resolve_player_ability_effect(
        ability,
        plan,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn random_entry_includes_both_depth_endpoints_and_rebuilds_after_ascent() {
    let mut game = prepared_entry();
    let surface = game.current_floor_id.clone();
    let departure = game.player.position;
    let world_position = game.wilderness_position;
    enter_at_depth(&mut game, 25);
    let first_instance = game.current_dungeon_instance_id.clone().unwrap();
    assert!(game.recall.is_none());
    assert!(game.dungeon_states[FOREST].recall_floor_id.is_none());
    assert!(!game.terrain.iter().any(|id| {
        game.content
            .terrain(id)
            .unwrap()
            .tags
            .iter()
            .any(|tag| tag == "stairs-down")
    }));
    let (up, down) = game.teleport_level_targets();
    assert!(up.is_empty());
    assert_eq!(down[0].floor_id, floor_id(26));
    let up_terrain = game
        .content
        .world(&game.world_id)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == game.current_floor_id)
        .unwrap()
        .up_stair_terrain_id
        .clone();
    place_player_on_terrain(&mut game, &up_terrain);
    game.traverse_stairs(false).unwrap().unwrap();
    assert_eq!(game.current_floor_id, surface);
    assert_eq!(game.player.position, departure);
    assert_eq!(game.wilderness_position, world_position);
    assert!(
        game.stored_floors
            .values()
            .all(|floor| floor.dungeon_instance_id.as_deref() != Some(first_instance.as_str()))
    );
    assert!(game.dungeon_states[FOREST].retained_instance_id.is_none());
    game = restore(&game);
    enter_at_depth(&mut game, 50);
    assert_ne!(
        game.current_dungeon_instance_id.as_ref(),
        Some(&first_instance)
    );
    let (up, down) = game.teleport_level_targets();
    assert_eq!(
        up[0].floor_id,
        game.content.world(&game.world_id).unwrap().initial_floor_id
    );
    assert!(down.is_empty());
    restore(&game);
}

#[test]
fn random_middle_level_teleport_uses_adjacent_depth_or_exits_the_instance() {
    let mut game = prepared_entry();
    enter_at_depth(&mut game, 37);
    let instance = game.current_dungeon_instance_id.clone();
    let original = restore(&game);
    for (roll, expected) in [(1, floor_id(38)), (0, "core.floor.wilderness".to_owned())] {
        let mut branch = original.clone();
        branch.rng = RfbRng::seeded(seed_for_roll(2, roll));
        let (_, down) = branch.teleport_level_targets();
        assert_eq!(down[0].floor_id, floor_id(38));
        resolve_spell_effect(&mut branch, "demo.ability.arcane-teleport-level");
        assert_eq!(branch.current_floor_id, expected);
        if roll == 1 {
            assert_eq!(branch.current_dungeon_instance_id, instance);
            assert!(
                branch
                    .stored_floors
                    .values()
                    .any(|floor| floor.id == floor_id(37))
            );
        } else {
            assert!(branch.current_dungeon_instance_id.is_none());
            assert!(
                branch
                    .stored_floors
                    .values()
                    .all(|floor| floor.dungeon_instance_id != instance)
            );
        }
        restore(&branch);
    }
}

#[test]
fn rejected_random_entry_preserves_rng_and_all_floor_state() {
    let mut game = prepared_entry();
    game.dungeon_states
        .get_mut(FOREST)
        .unwrap()
        .next_instance_ordinal = u32::MAX;
    let before = game.to_save();
    let rng = game.rng.clone();
    assert!(matches!(
        game.traverse_stairs(false),
        Err(CoreError::InvalidSave("dungeon instance ordinal overflow"))
    ));
    assert_eq!(game.to_save(), before);
    assert_eq!(game.rng, rng);
    // Generation failure happens after the depth draw and source floor extraction.
    game.dungeon_states
        .get_mut(FOREST)
        .unwrap()
        .next_instance_ordinal = 0;
    game.next_item_instance_serial = u64::MAX;
    game.rng = RfbRng::seeded(seed_for_roll(26, 12));
    let before = game.to_save();
    let rng = game.rng.clone();
    assert!(matches!(
        game.traverse_stairs(false),
        Err(CoreError::ItemIdExhausted)
    ));
    assert_eq!(game.to_save(), before);
    assert_eq!(game.rng, rng);
}

#[test]
fn random_magic_stairs_can_descend_but_the_deepest_floor_only_allows_ascent() {
    let mut game = prepared_entry();
    enter_at_depth(&mut game, 37);
    let instance = game.current_dungeon_instance_id.clone();
    game.rng = RfbRng::seeded(seed_for_roll(100, 75));
    resolve_spell_effect(&mut game, "demo.ability.sorcery-create-stair");
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.stairs-down"
    );
    game.traverse_stairs(false).unwrap().unwrap();
    assert_eq!(game.current_floor_id, floor_id(38));
    assert_eq!(game.current_dungeon_instance_id, instance);
    game.transition_floor(floor_id(50), None, None, false)
        .unwrap()
        .unwrap();
    let rng = game.rng.clone();
    resolve_spell_effect(&mut game, "demo.ability.sorcery-create-stair");
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.stairs-up"
    );
    assert_eq!(game.rng, rng);
    game.traverse_stairs(false).unwrap().unwrap();
    assert!(game.current_dungeon_instance_id.is_none());
    assert!(
        game.stored_floors
            .values()
            .all(|floor| floor.dungeon_instance_id != instance)
    );
}

fn use_recall_item(game: &mut Game) -> Vec<DomainEvent> {
    let effect = ItemUseEffectDefinition::Recall {
        delay_dice: 1,
        delay_sides: 1,
        delay_bonus: 1,
    };
    let plan = game.item_use_plan("", &effect, None, None, None).unwrap();
    let mut events = Vec::new();
    game.resolve_item_recall("demo.item.recall-rod".to_owned(), effect, plan, &mut events);
    events
}

#[test]
fn random_outward_recall_without_ordinary_destination_cancels_saves_and_triggers() {
    let mut game = prepared_entry();
    let surface = game.current_floor_id.clone();
    let departure = game.player.position;
    enter_at_depth(&mut game, 37);
    assert_eq!(game.recall_use_plan(), Some(RecallUseAction::Start));
    assert!(
        game.item_use_plan("", &ItemUseEffectDefinition::ResetRecall, None, None, None)
            .is_none()
    );
    let events = use_recall_item(&mut game);
    assert!(events.iter().any(
        |event| matches!(event, DomainEvent::ItemRecallStarted { floor_id, .. }
        if floor_id == &game.content.world(&game.world_id).unwrap().initial_floor_id)
    ));
    assert!(
        game.snapshot()
            .player
            .recall
            .as_ref()
            .unwrap()
            .destination
            .is_none()
    );
    game = restore(&game);
    let rng = game.rng.clone();
    use_recall_item(&mut game);
    assert!(game.recall.is_none());
    assert_eq!(game.rng, rng);
    game.debug_recall_delay_turns = Some(1);
    resolve_spell_effect(&mut game, "demo.ability.arcane-word-of-recall");
    assert!(game.recall.as_ref().unwrap().destination.is_none());
    let rng = game.rng.clone();
    resolve_spell_effect(&mut game, "demo.ability.arcane-word-of-recall");
    assert!(game.recall.is_none());
    assert_eq!(game.rng, rng);
    game.debug_recall_delay_turns = None;
    use_recall_item(&mut game);
    game = restore(&game);
    for _ in 0..3 {
        game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
            .unwrap();
    }
    assert_eq!(game.current_floor_id, surface);
    assert_eq!(game.player.position, departure);
    assert!(game.recall.is_none());
    assert!(game.recall_use_plan().is_none());
    assert!(game.teleport_dungeon_dtos().is_empty());
    restore(&game);
}

#[test]
fn random_spell_recall_preserves_the_existing_ordinary_destination() {
    let mut game = prepared_entry();
    game.transition_floor("demo.floor.warrens-depth-1".to_owned(), None, None, false)
        .unwrap()
        .unwrap();
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    let ordinary = game.recall.clone().unwrap().destination.unwrap();
    enter_at_depth(&mut game, 37);
    assert_eq!(
        game.recall.as_ref().unwrap().destination.as_ref(),
        Some(&ordinary)
    );
    assert!(game.recall_reset_plan().is_none());
    game.debug_recall_delay_turns = Some(1);
    resolve_spell_effect(&mut game, "demo.ability.arcane-word-of-recall");
    game = restore(&game);
    for _ in 0..2 {
        game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
            .unwrap();
    }
    assert_eq!(
        game.recall.as_ref().unwrap().destination.as_ref(),
        Some(&ordinary)
    );
    assert!(
        game.teleport_dungeon_dtos()
            .iter()
            .all(|dungeon| dungeon.dungeon_id != FOREST)
    );
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.current_floor_id, ordinary.floor_id);
    restore(&game);
}

#[test]
fn random_recall_save_rejects_random_destinations_and_empty_pending_state() {
    let mut game = prepared_entry();
    let mut invalid_surface = game.to_save();
    invalid_surface.player.recall = Some(RecallStateDto {
        destination: None,
        remaining_turns: Some(2),
    });
    assert!(Game::from_save_with_content(invalid_surface, game.content.clone()).is_err());
    enter_at_depth(&mut game, 37);
    game.start_recall(3);
    restore(&game);
    let mut invalid = game.to_save();
    invalid.player.recall.as_mut().unwrap().destination =
        Some(rfb_protocol::RecallDestinationDto {
            dungeon_id: FOREST.to_owned(),
            floor_id: floor_id(37),
        });
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
    let mut invalid = game.to_save();
    invalid.player.recall.as_mut().unwrap().remaining_turns = None;
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
    let mut invalid = game.to_save();
    invalid.player.recall.as_mut().unwrap().remaining_turns = Some(0);
    assert!(Game::from_save_with_content(invalid, game.content.clone()).is_err());
    let mut invalid = game.clone();
    invalid
        .dungeon_states
        .get_mut(FOREST)
        .unwrap()
        .recall_floor_id = Some(floor_id(37));
    assert!(Game::from_save_with_content(invalid.to_save(), game.content.clone()).is_err());
}

fn replay_command(
    game: &mut Game,
    restored: &mut Game,
    command: GameCommand,
) -> rfb_protocol::GameUpdate {
    let update = dispatch_next(game, command.clone());
    assert_eq!(dispatch_next(restored, command), update);
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    update
}

#[test]
fn two_generated_forest_entrances_keep_returns_items_and_replayed_instances_separate() {
    let mut game = Game::new_with_build(29, "demo.build.warrior").unwrap();
    wilderness::prepare_random_entries_for_test(
        &mut game,
        "demo.wilderness-encounter.trees-random-forest-level",
        true,
    );
    let first = game
        .floor_connections
        .iter()
        .find(|connection| connection.position == game.player.position)
        .unwrap()
        .clone();
    let chunk = first.wilderness_entrance.as_ref().unwrap().chunk;
    let second = game
        .floor_connections
        .iter()
        .find(|connection| {
            connection
                .wilderness_entrance
                .as_ref()
                .is_some_and(|binding| {
                    binding.chunk
                        == Position {
                            x: chunk.x + 1,
                            y: chunk.y,
                        }
                })
        })
        .unwrap()
        .clone();
    assert_eq!(first.target_floor_id, second.target_floor_id);
    assert_ne!(first.id, second.id);
    let world_position = game.wilderness_position;
    give_inventory_item(&mut game, "test.random.surface-item", "demo.item.dagger");
    game.drop_inventory_quantity("test.random.surface-item", 1)
        .unwrap()
        .unwrap();
    // Direct fixture placement bypasses the command's ordinary visibility update.
    game.reveal_current_visibility();
    let surface_item = game
        .items
        .iter()
        .find(|item| item.location == ItemLocation::Ground(first.position))
        .unwrap()
        .clone();
    let mut restored = restore(&game);
    let mut previous_instance = None;
    let mut previous_floor_items = BTreeSet::new();
    // Visit A, B, then A again. Only the tested stair commands advance the flow.
    for entrance in [&first, &second, &first] {
        for branch in [&mut game, &mut restored] {
            branch.player.position = entrance.position;
            branch.reveal_current_visibility();
        }
        replay_command(&mut game, &mut restored, GameCommand::TraverseStairs);
        let instance = game.current_dungeon_instance_id.clone().unwrap();
        assert_ne!(Some(&instance), previous_instance.as_ref());
        assert!(
            game.current_floor_id
                .starts_with("demo.floor.random-forest-depth-")
        );
        assert!(
            game.items
                .iter()
                .all(|item| item.id != surface_item.id && item.id != "test.random.abandoned")
        );
        let floor_items = game
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item.location,
                    ItemLocation::Ground(_) | ItemLocation::CarriedBy { .. }
                )
            })
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>();
        assert!(floor_items.is_disjoint(&previous_floor_items));
        previous_floor_items = floor_items;
        clear_monsters(&mut game);
        give_inventory_item(&mut game, "test.random.abandoned", "demo.item.dagger");
        game.drop_inventory_quantity("test.random.abandoned", 1)
            .unwrap()
            .unwrap();
        game.reveal_current_visibility();
        // Branch at the dungeon save boundary; the next generation must also agree.
        restored = restore(&game);
        for branch in [&mut game, &mut restored] {
            place_player_on_terrain(branch, "demo.terrain.stairs-up");
        }
        replay_command(&mut game, &mut restored, GameCommand::TraverseStairs);
        assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
        assert_eq!(game.player.position, entrance.position);
        assert_eq!(game.wilderness_position, world_position);
        assert_eq!(
            game.items.iter().find(|item| item.id == surface_item.id),
            Some(&surface_item)
        );
        assert!(
            game.items
                .iter()
                .all(|item| item.id != "test.random.abandoned")
        );
        assert!(
            game.stored_floors
                .values()
                .all(|floor| floor.dungeon_instance_id.as_ref() != Some(&instance))
        );
        assert!(game.dungeon_states[FOREST].retained_instance_id.is_none());
        assert!(game.dungeon_states[FOREST].recall_floor_id.is_none());
        restored = restore(&game);
        previous_instance = Some(instance);
    }
}

#[test]
fn pending_random_item_and_spell_recall_replay_through_rest_and_next_entry() {
    let mut initial = prepared_entry();
    let departure = initial.player.position;
    enter_at_depth(&mut initial, 37);
    clear_monsters(&mut initial);
    for use_spell in [false, true] {
        let mut game = initial.clone();
        if use_spell {
            resolve_spell_effect(&mut game, "demo.ability.arcane-word-of-recall");
        } else {
            give_inventory_item(&mut game, "test.random.recall", "demo.item.homeward-scroll");
            dispatch_next(
                &mut game,
                GameCommand::UseItem {
                    item_id: "test.random.recall".into(),
                    target: None,
                },
            );
            assert!(
                game.items
                    .iter()
                    .all(|item| item.id != "test.random.recall")
            );
        }
        assert!(game.recall.as_ref().unwrap().remaining_turns.is_some());
        assert!(
            game.snapshot()
                .player
                .recall
                .as_ref()
                .unwrap()
                .destination
                .is_none()
        );
        let mut restored = restore(&game);
        let update = replay_command(&mut game, &mut restored, GameCommand::Rest { turns: 100 });
        assert!(rest_resolution(&update).completed_turns > 0);
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "item.recall-triggered")
        );
        assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
        assert_eq!(game.player.position, departure);
        assert!(game.recall.is_none());
        assert!(game.snapshot().player.recall.is_none());
        replay_command(&mut game, &mut restored, GameCommand::TraverseStairs);
        assert!(
            game.current_floor_id
                .starts_with("demo.floor.random-forest-depth-")
        );
        restore(&game);
    }
}

#[test]
fn random_dungeon_save_requires_its_stored_surface_and_bound_return_position() {
    let mut game = prepared_entry();
    enter_at_depth(&mut game, 37);
    restore(&game);
    let mut missing = game.clone();
    missing
        .stored_floors
        .remove(wilderness::WILDERNESS_FLOOR_ID);
    assert!(matches!(
        Game::from_save_with_content(missing.to_save(), game.content.clone()),
        Err(CoreError::InvalidSave(
            "random dungeon return entrance is invalid"
        ))
    ));
    let mut unbound = game.clone();
    let surface = unbound
        .stored_floors
        .get_mut(wilderness::WILDERNESS_FLOOR_ID)
        .unwrap();
    surface
        .connections
        .retain(|connection| connection.position != surface.player_position);
    assert!(Game::from_save_with_content(unbound.to_save(), game.content.clone()).is_err());
    let mut displaced = game.clone();
    displaced
        .stored_floors
        .get_mut(wilderness::WILDERNESS_FLOOR_ID)
        .unwrap()
        .player_position
        .x += 1;
    assert!(Game::from_save_with_content(displaced.to_save(), game.content.clone()).is_err());
    let mut dangling = game.clone();
    let surface = dangling
        .stored_floors
        .get_mut(wilderness::WILDERNESS_FLOOR_ID)
        .unwrap();
    surface
        .connections
        .iter_mut()
        .find(|connection| connection.position == surface.player_position)
        .unwrap()
        .wilderness_entrance
        .as_mut()
        .unwrap()
        .placement
        .encounter_id = "test.missing-template".into();
    assert!(Game::from_save_with_content(dangling.to_save(), game.content.clone()).is_err());
    let mut wrong_destination = game.to_save();
    wrong_destination.player.recall = Some(RecallStateDto {
        destination: Some(rfb_protocol::RecallDestinationDto {
            dungeon_id: "demo.dungeon.warrens".into(),
            floor_id: floor_id(37),
        }),
        remaining_turns: Some(3),
    });
    assert!(Game::from_save_with_content(wrong_destination, game.content.clone()).is_err());
}

#[test]
fn town_recall_and_level_facility_exclude_a_visited_random_dungeon() {
    let mut game = prepared_entry();
    enter_at_depth(&mut game, 37);
    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    game.traverse_stairs(false).unwrap().unwrap();
    let facility_id = "demo.town-facility.morivant-trump-tower";
    super::town::enter_town_facility(&mut game, facility_id);
    game.gold = 1_000_000;
    assert!(game.teleport_dungeon_dtos().is_empty());
    let facility = game
        .snapshot()
        .task_services
        .into_iter()
        .find(|service| service.id == facility_id)
        .unwrap();
    assert!(facility.teleport_dungeons.is_empty());
    let before = game.to_save();
    assert!(matches!(
        game.teleport_to_dungeon_level_at_facility(facility_id, FOREST, 37),
        Err("recall-unavailable")
    ));
    assert!(matches!(
        game.use_town_facility_service(
            facility_id,
            FacilityServiceKindDto::Recall,
            None,
            None,
            &mut Vec::new()
        ),
        Err("recall-unavailable")
    ));
    assert_eq!(game.to_save(), before);
    restore(&game);
}

#[test]
fn random_representative_floors_materialize_ecology_and_reachable_safe_stairs() {
    use crate::game::movement::actor_can_cross_terrain;
    use crate::game::world::generation::roll_random_floor_features;
    use crate::game::world::geometry::{generated_terrain_index, maze_floor_distances};
    let base = Game::new_with_build(29, "demo.build.warrior").unwrap();
    for (kind, depths) in [
        ("random-forest", &[25, 50][..]),
        ("random-volcano", &[50, 80, 81, 90][..]),
        ("random-mountain", &[40, 70][..]),
        ("random-sea", &[55, 75][..]),
    ] {
        for &depth in depths {
            let id = format!("demo.floor.{kind}-depth-{depth}");
            let definition = base
                .content
                .world(&base.world_id)
                .unwrap()
                .procedural_floors
                .iter()
                .find(|floor| floor.id == id)
                .unwrap()
                .clone();
            let lava = kind == "random-volcano";
            // Select the feature branch using only the production decision function.
            let desired = if (lava && depth == 81) || (kind == "random-sea" && depth == 55) {
                "lake"
            } else if (lava && depth == 90) || (kind == "random-sea" && depth == 75) {
                "destroyed"
            } else if kind == "random-mountain" && depth == 40 {
                "cavern"
            } else {
                "ordinary"
            };
            let seed = (0..100_000)
                .find(|seed| {
                    let features =
                        roll_random_floor_features(&mut RfbRng::seeded(*seed), &definition, lava);
                    match desired {
                        "lake" => features.lake && !features.lake_vault,
                        "destroyed" => features.destroyed,
                        "cavern" => features.cavern,
                        _ => !features.lake && !features.cavern && !features.destroyed,
                    }
                })
                .unwrap();
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            let floor = game.generate_procedural_floor(&definition, None).unwrap();
            let at = |position| {
                game.content
                    .terrain(&floor.terrain[generated_terrain_index(floor.width, position)])
                    .unwrap()
            };
            assert!(at(floor.player_position).walkable, "{id}");
            let walkable = floor
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
            let reached = maze_floor_distances(&walkable, floor.player_position);
            let mut stairs = 0;
            for (index, terrain_id) in floor.terrain.iter().enumerate() {
                let terrain = game.content.terrain(terrain_id).unwrap();
                assert!(!terrain.tags.iter().any(|tag| tag == "stairs-down"), "{id}");
                if terrain.tags.iter().any(|tag| tag == "stairs-up") {
                    stairs += 1;
                    assert!(
                        reached.contains_key(&Position {
                            x: (index % usize::from(floor.width)) as i32,
                            y: (index / usize::from(floor.width)) as i32,
                        }),
                        "unreachable return stair in {id}"
                    );
                }
            }
            assert!(stairs > 0);
            assert!(!floor.entities.is_empty(), "empty ecology: {id}");
            let mut occupied = BTreeSet::from([floor.player_position]);
            for actor in &floor.entities {
                assert!(occupied.insert(actor.position));
                assert!(
                    actor_can_cross_terrain(
                        game.content.actor(&actor.kind_id).unwrap(),
                        at(actor.position)
                    ),
                    "{id}: {}",
                    actor.kind_id
                );
            }
            assert!(!floor.items.is_empty(), "missing loot: {id}");
            for item in &floor.items {
                if let ItemLocation::Ground(position) = item.location {
                    assert!(at(position).allows_items());
                }
            }
            assert!(
                floor
                    .gold_piles
                    .iter()
                    .all(|pile| at(pile.position).allows_items())
            );
            if desired == "destroyed" {
                assert!(floor.terrain.iter().any(|id| id == "demo.terrain.rubble"));
            }
            let material = match kind {
                "random-forest" => "demo.terrain.surface-flower",
                "random-volcano" => "demo.terrain.surface-lava-deep",
                "random-mountain" => "demo.terrain.mountain-wall",
                _ => "demo.terrain.surface-water-deep",
            };
            assert!(floor.terrain.iter().any(|id| id == material));
            if depth == depths[0] && kind != "random-forest" {
                game.activate_floor(floor, Vec::new());
                clear_monsters(&mut game);
                place_player_on_terrain(&mut game, material);
                game.world_tick = 10;
                game.player.hp = 1000;
                if lava {
                    assert!(game.process_player_interior_water_lava_damage(&mut Vec::new()));
                    assert!(game.player.hp < 1000);
                    game.player
                        .resistances
                        .set(DamageType::Fire, ResistanceLevel::Immune);
                    assert!(!game.process_player_interior_water_lava_damage(&mut Vec::new()));
                } else if kind == "random-sea" {
                    assert!(
                        game.player_can_cross_surface_terrain(
                            game.content.terrain(material).unwrap()
                        )
                    );
                    assert!(!game.process_player_interior_water_lava_damage(&mut Vec::new()));
                } else {
                    let position = game.player.position;
                    assert!(
                        !game.player_can_cross_terrain(game.content.terrain(material).unwrap())
                    );
                    // Exercise the spell resolver; raw replacement applies an already
                    // validated plan and returns proficiency gain, not permission.
                    game.resolve_terrain_beam_effect(
                        "test.stone-to-mud",
                        rfb_content::AbilityTerrainBeamOperationDefinition::StoneToMud,
                        vec![position],
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                        &mut Vec::new(),
                    )
                    .unwrap();
                    assert_eq!(game.terrain_at(position), material);
                }
            }
        }
    }
}

#[test]
fn random_generation_feature_gates_use_depth_thresholds_and_mutual_exclusion() {
    use crate::game::world::generation::roll_random_floor_features;
    let game = Game::new_with_build(29, "demo.build.warrior").unwrap();
    let world = game.content.world(&game.world_id).unwrap();
    for (kind, below, above, lava) in [
        ("random-volcano", 80, 81, true),
        ("random-sea", 50, 51, false),
    ] {
        let mut definition = world
            .procedural_floors
            .iter()
            .find(|floor| {
                floor.id == format!("demo.floor.{kind}-depth-{}", if lava { 81 } else { 55 })
            })
            .unwrap()
            .clone();
        // Probe synthetic adjacent depths to isolate the original threshold.
        definition.depth = above;
        let seed = (0..100_000)
            .find(|seed| {
                roll_random_floor_features(&mut RfbRng::seeded(*seed), &definition, lava).lake
            })
            .unwrap();
        let features = roll_random_floor_features(&mut RfbRng::seeded(seed), &definition, lava);
        assert!(features.lake && !features.destroyed && !features.cavern);
        definition.depth = below;
        assert!(!roll_random_floor_features(&mut RfbRng::seeded(seed), &definition, lava).lake);
    }
    let volcano = world
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.random-volcano-depth-90")
        .unwrap();
    for desired in ["destroyed", "cavern", "lake-vault"] {
        let seed = (0..100_000)
            .find(|seed| {
                let features =
                    roll_random_floor_features(&mut RfbRng::seeded(*seed), volcano, true);
                match desired {
                    "destroyed" => features.destroyed,
                    "cavern" => features.cavern,
                    _ => features.lake_vault,
                }
            })
            .unwrap();
        let features = roll_random_floor_features(&mut RfbRng::seeded(seed), volcano, true);
        assert_eq!(
            usize::from(features.destroyed)
                + usize::from(features.lake)
                + usize::from(features.cavern),
            1
        );
    }
}

#[test]
fn forest_earthquake_and_destruction_preserve_water_and_use_forest_materials() {
    let mut game = prepared_entry();
    enter_at_depth(&mut game, 37);
    clear_monsters(&mut game);
    let center = game.player.position;
    let water = Position {
        x: center.x + 1,
        y: center.y,
    };
    replace_terrain(&mut game, water, "demo.terrain.surface-water-shallow");
    let plan = game.plan_area_destruction(
        3,
        3,
        "demo.terrain.floor",
        "demo.terrain.wall",
        "demo.terrain.quartz-vein",
        "demo.terrain.magma-vein",
    );
    let outcome = game.apply_area_destruction_plan(
        plan,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    );
    assert!(!outcome.affected_positions.is_empty());
    assert!(outcome.affected_positions.iter().all(|position| {
        *position != water
            && [
                "demo.terrain.surface-grass",
                "demo.terrain.surface-tree",
                "demo.terrain.surface-brake",
            ]
            .contains(&game.terrain_at(*position))
    }));
    game.resolve_earthquake(
        center,
        2,
        100,
        "demo.terrain.floor",
        &["demo.terrain.wall".into()],
        crate::game::abilities::terrain::EarthquakeSource::Weapon("test.impact".into()),
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(game.terrain_at(water), "demo.terrain.surface-water-shallow");
}
