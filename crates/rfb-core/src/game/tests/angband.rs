// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;
use crate::game::world::geometry::maze_floor_distances;
mod bosses;
mod campaign;

const DUNGEON: &str = "demo.dungeon.angband";

fn assignments(game: &Game) -> Vec<(String, rfb_protocol::RandomTaskAssignmentDto)> {
    (1..=10)
        .map(|number| {
            let id = format!("demo.task.angband-random-{number}");
            let assignment = game.task_states[&id].random_assignment.clone().unwrap();
            (id, assignment)
        })
        .collect()
}

#[test]
fn angband_birth_assignments_are_deterministic_hidden_and_reserved() {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    let same = Game::new_with_build(42, "demo.build.warrior").unwrap();
    let other = Game::new_with_build(43, "demo.build.warrior").unwrap();
    let drawn = assignments(&game);
    assert_eq!(drawn, assignments(&same));
    assert_ne!(drawn, assignments(&other));
    let world = game.content.world(DEFAULT_WORLD_ID).unwrap();
    let mut targets = BTreeSet::new();
    let mut previous = 0;
    for ((id, assignment), base) in drawn
        .iter()
        .zip([10_u16, 18, 26, 34, 42, 50, 58, 66, 74, 82])
    {
        let spread = (base / 10).clamp(3, 8);
        assert!((base - spread..=base + spread).contains(&assignment.depth));
        assert!(assignment.depth > previous);
        previous = assignment.depth;
        assert!(targets.insert(&assignment.actor_kind_id));
        assert!(
            world
                .random_task_candidates
                .iter()
                .any(|candidate| candidate.can_be_target
                    && candidate.actor_kind_id == assignment.actor_kind_id)
        );
        assert!(game.actor_kind_is_reserved_task_target(&assignment.actor_kind_id));
        assert_eq!(game.task_states[id].required, 1);
        assert_eq!(game.task_states[id].status, TaskStatusKindDto::Available);
    }
    for (id, actor) in [
        (
            "demo.task.angband-oberon",
            "demo.actor.oberon-king-of-amber",
        ),
        (
            "demo.task.angband-serpent-of-chaos",
            "demo.actor.the-serpent-of-chaos",
        ),
    ] {
        assert_eq!(game.task_states[id].status, TaskStatusKindDto::Taken);
        assert!(game.actor_kind_is_reserved_task_target(actor));
    }
    let hash = game.state_hash();
    for _ in 0..2 {
        assert!(
            !game
                .task_statuses()
                .iter()
                .any(|task| task.task_id.starts_with("demo.task.angband-random-"))
        );
        assert!(
            game.task_statuses()
                .iter()
                .any(|task| task.task_id == "demo.task.angband-oberon")
        );
    }
    assert_eq!(hash, game.state_hash());
    let mut loaded = restored(&game);
    assert_eq!(assignments(&loaded), drawn);
    assert_eq!(loaded.rng.bounded(1_000_000), game.rng.bounded(1_000_000));
    assert_eq!(loaded.state_hash(), game.state_hash());
}

#[test]
fn angband_birth_rejects_an_exhausted_pool_without_substituting_a_target() {
    let game = Game::new(42);
    let mut world = game.content.world(DEFAULT_WORLD_ID).unwrap().clone();
    for candidate in &mut world.random_task_candidates {
        candidate.can_be_target = false;
    }
    let mut states = initial_task_states(&world, 42);
    let mut rng = crate::rng::RfbRng::seeded(42);
    assert!(matches!(
        crate::game::tasks::assign_random_tasks(&world, &game.content, &mut states, &mut rng),
        Err(CoreError::Invariant(_))
    ));
    assert!(
        states
            .values()
            .all(|state| state.random_assignment.is_none())
    );
}

#[test]
fn angband_save_rejects_missing_duplicate_or_malformed_birth_assignments() {
    let game = Game::new(42);
    let save = game.to_save();
    let index = save
        .task_states
        .iter()
        .position(|state| state.task_id == "demo.task.angband-random-1")
        .unwrap();
    let second = save
        .task_states
        .iter()
        .position(|state| state.task_id == "demo.task.angband-random-2")
        .unwrap();
    for mutation in 0..8 {
        let mut invalid = save.clone();
        match mutation {
            0 => invalid.task_states[index].random_assignment = None,
            1 => {
                invalid.task_states.remove(index);
            }
            2 => {
                invalid.task_states[index]
                    .random_assignment
                    .as_mut()
                    .unwrap()
                    .depth = u16::MAX
            }
            3 => {
                invalid.task_states[second].random_assignment =
                    invalid.task_states[index].random_assignment.clone()
            }
            4 => invalid.task_states[index].required = 2,
            5 => {
                invalid.task_states[index]
                    .random_assignment
                    .as_mut()
                    .unwrap()
                    .actor_kind_id = "demo.actor.utgard-loke".into()
            }
            6 => {
                let assignment = invalid.task_states[index].random_assignment.clone();
                invalid
                    .task_states
                    .iter_mut()
                    .find(|state| state.task_id == "demo.task.angband-oberon")
                    .unwrap()
                    .random_assignment = assignment;
            }
            7 => invalid.task_states.clear(),
            _ => unreachable!(),
        }
        assert!(
            Game::from_save(invalid, Game::default_behavior_preferences()).is_err(),
            "accepted malformed assignment case {mutation}"
        );
    }
}

#[test]
fn angband_active_paused_and_completed_assignments_survive_save_without_reroll() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (id, assignment) = assignments(&game).remove(0);
    let target_floor = floor_id(assignment.depth);
    game.transition_floor(target_floor.clone(), None, None, false)
        .unwrap();
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Active);
    let target = game
        .entities
        .iter_mut()
        .find(|actor| actor.kind_id == assignment.actor_kind_id)
        .unwrap();
    target.hp -= 1;
    let target = target.clone();
    game.reveal_current_visibility();
    let mut game = restored(&game);
    assert_eq!(
        game.task_statuses()
            .iter()
            .find(|task| task.task_id == id)
            .unwrap()
            .floor_id,
        target_floor
    );
    game.transition_floor(floor_id(1), None, None, false)
        .unwrap();
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Paused);
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    let mut game = restored(&game);
    assert_eq!(
        game.task_states[&id].random_assignment.as_ref(),
        Some(&assignment)
    );
    game.transition_floor(target_floor.clone(), None, None, false)
        .unwrap()
        .unwrap();
    let targets = game
        .entities
        .iter()
        .filter(|actor| actor.kind_id == assignment.actor_kind_id)
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].id, target.id);
    assert_eq!(targets[0].hp, target.hp);
    assert!(targets[0].no_pet);
    assert_eq!(game.task_states[&id].retakes_used, 1);
    let events = task_death(&mut game, &target.id, false);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::TaskRewarded { .. }))
    );
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Completed);
    assert_eq!(game.task_states[&id].current, 1);
    let before = game.state_hash();
    game.progress_dungeon_task_on_death(&target, &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    game.apply_deferred_task_events(None, &mut Vec::new())
        .unwrap();
    assert_eq!(game.state_hash(), before);
    assert!(!game.actor_kind_is_reserved_task_target(&assignment.actor_kind_id));
    game.reveal_current_visibility();
    let game = restored(&game);
    assert_eq!(
        game.task_states[&id].random_assignment.as_ref(),
        Some(&assignment)
    );
    let mut states = game.task_states.clone();
    states.get_mut(&id).unwrap().status = TaskStatusKindDto::Abandoned;
    crate::game::tasks::validate_random_task_assignments(
        game.content.world(DEFAULT_WORLD_ID).unwrap(),
        &game.content,
        &states,
    )
    .unwrap();
}

#[test]
fn angband_reservation_blocks_ordinary_allocation_and_both_summon_candidate_paths() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (id, assignment) = assignments(&game)
        .into_iter()
        .find(|(_, assignment)| {
            let actor = game.content.actor(&assignment.actor_kind_id).unwrap();
            actor.allocation.is_some() && actor_answers_summons(actor)
        })
        .unwrap();
    let kind_id = &assignment.actor_kind_id;
    let level = game.content.actor(kind_id).unwrap().level as u16;
    let policy = game
        .content
        .encounter_table("demo.encounter-table.angband")
        .unwrap()
        .global_allocation
        .clone()
        .unwrap();
    let mut released = game.clone();
    released.task_states.get_mut(&id).unwrap().status = TaskStatusKindDto::Abandoned;
    let seed = (0..4096)
        .find(|seed| {
            released.rng = crate::rng::RfbRng::seeded(*seed);
            released
                .select_original_allocated_monster(
                    &floor_id(1),
                    &policy,
                    level,
                    level,
                    None,
                    &[],
                    None,
                    None,
                )
                .as_ref()
                == Some(kind_id)
        })
        .expect("released target must be reachable through its ordinary source allocation");
    game.rng = crate::rng::RfbRng::seeded(seed);
    assert_ne!(
        game.select_original_allocated_monster(
            &floor_id(1),
            &policy,
            level,
            level,
            None,
            &[],
            None,
            None
        )
        .as_ref(),
        Some(kind_id)
    );
    assert!(
        !game
            .summon_category_candidate_kind_ids("unique", None, 127, true, true)
            .contains(kind_id)
    );
    assert!(
        released
            .summon_category_candidate_kind_ids("unique", None, 127, true, true)
            .contains(kind_id)
    );

    for sample in [&mut game, &mut released] {
        sample.terrain.fill("demo.terrain.floor".into());
        sample.player.position = Position { x: 40, y: 20 };
        sample.entities.push(actor_from_runtime_spawn(
            "generated.actor.ag3-summoner",
            "demo.actor.the-resurrection-machine",
            Position { x: 20, y: 20 },
            15_488,
            152,
            100,
            true,
        ));
    }
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-legacy-import-l100-1d3-1")
        .unwrap()
        .clone();
    for (sample, expected) in [(&game, false), (&released, true)] {
        let plan = sample
            .monster_ability_target_plan(0, ability.clone(), 1)
            .unwrap();
        let MonsterAbilityTargetPlan::SummonCategory {
            candidate_kind_ids, ..
        } = plan.target
        else {
            panic!("source S_MONSTER must resolve a category plan");
        };
        assert_eq!(candidate_kind_ids.contains(kind_id), expected);
    }
    let mut fixed = ability;
    fixed.effect = AbilityEffectDefinition::Summon {
        actor_kind_id: kind_id.clone(),
        count: 1,
        radius: 2,
        duration_turns: 50,
        hostile: false,
    };
    for (sample, expected) in [(&game, false), (&released, true)] {
        assert_eq!(
            sample
                .ability_target_plan(&fixed, &TargetSelection::SelfTarget)
                .is_some(),
            expected
        );
        assert_eq!(
            sample
                .monster_ability_target_plan(0, fixed.clone(), 1)
                .is_ok(),
            expected
        );
    }
}

#[test]
fn angband_reserved_dead_target_cannot_be_resurrected_until_released() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (id, assignment) = assignments(&game)
        .into_iter()
        .find(|(_, assignment)| {
            let actor = game.content.actor(&assignment.actor_kind_id).unwrap();
            (45..=100).contains(&actor.level) && actor_answers_summons(actor)
        })
        .unwrap();
    game.terrain.fill("demo.terrain.floor".into());
    game.player.position = Position { x: 80, y: 20 };
    game.entities.push(actor_from_runtime_spawn(
        "generated.actor.ag3-resurrection",
        "demo.actor.the-resurrection-machine",
        Position { x: 20, y: 20 },
        15_488,
        152,
        100,
        true,
    ));
    game.defeated_limited_actor_counts
        .insert(assignment.actor_kind_id.clone(), 1);
    crate::game::tasks::validate_random_task_assignments(
        game.content.world(DEFAULT_WORLD_ID).unwrap(),
        &game.content,
        &game.task_states,
    )
    .unwrap();
    let mut released = game.clone();
    released.task_states.get_mut(&id).unwrap().status = TaskStatusKindDto::Abandoned;
    let seed = (0..1000)
        .find(|seed| {
            let mut rng = crate::rng::RfbRng::seeded(*seed);
            rng.bounded(2);
            rng.bounded(13) != 0
        })
        .unwrap();
    let ability = game
        .content
        .ability("rfb-legacy.ability.summon-dead-unique-l100-1d2")
        .unwrap()
        .clone();
    for (sample, expected) in [(&mut game, false), (&mut released, true)] {
        sample.rng = crate::rng::RfbRng::seeded(seed);
        let plan = sample
            .monster_ability_target_plan(0, ability.clone(), 1)
            .unwrap();
        let outcome = sample
            .resolve_monster_ability_plan(
                0,
                "demo.actor.the-resurrection-machine",
                &plan,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                &mut Vec::new(),
            )
            .summon
            .unwrap();
        assert_eq!(
            outcome
                .summoned_kind_ids
                .contains(&assignment.actor_kind_id),
            expected
        );
    }
}

fn floor_id(depth: u16) -> String {
    format!("demo.floor.angband-depth-{depth}")
}

fn complete_task_gates(game: &mut Game) {
    for (id, state) in &mut game.task_states {
        if id.starts_with("demo.task.angband-") && id != "demo.task.angband-serpent-of-chaos" {
            state.status = TaskStatusKindDto::Completed;
            state.current = state.required;
            state.active_floor_id = None;
        }
    }
    game.refresh_dungeon_task_stairs();
}

fn task_death(game: &mut Game, id: &str, player_credit: bool) -> Vec<DomainEvent> {
    let index = game
        .entities
        .iter()
        .position(|actor| actor.id == id)
        .unwrap();
    let event = DomainEvent::EntityDiedFromStatus {
        target_kind_id: game.entities[index].kind_id.clone(),
        status_kind_id: STATUS_POISON.to_owned(),
        damage: crate::effect::resolve_damage(
            crate::effect::DamagePacket::new(1, crate::resistance::DamageType::Poison),
            ResistanceLevel::Normal,
        ),
    };
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    if player_credit {
        assert!(
            game.resolve_actor_death(index, event, &mut events, &mut changed, &mut removed)
                .unwrap()
        );
    } else {
        assert!(
            game.resolve_actor_death_without_credit(
                index,
                event,
                &mut events,
                &mut changed,
                &mut removed
            )
            .unwrap()
        );
    }
    events
}

#[test]
fn angband_task_gates_block_bypasses_and_random_abandon_releases_the_target() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (id, assignment) = assignments(&game).remove(0);
    game.transition_floor(floor_id(assignment.depth - 1), None, None, false)
        .unwrap()
        .unwrap();
    let before = game.state_hash();
    assert!(
        game.transition_floor(floor_id(assignment.depth + 1), None, None, false)
            .unwrap()
            .is_none()
    );
    assert_eq!(game.state_hash(), before);
    assert!(
        !game
            .teleport_level_targets()
            .1
            .iter()
            .any(|target| target.floor_id == floor_id(assignment.depth + 1))
    );
    assert_eq!(
        game.teleport_dungeon_dtos()
            .iter()
            .find(|entry| entry.dungeon_id == DUNGEON)
            .unwrap()
            .depths
            .last(),
        Some(&assignment.depth)
    );
    game.transition_floor(floor_id(assignment.depth), None, None, false)
        .unwrap()
        .unwrap();
    assert!(game.teleport_level_targets().1.is_empty());
    assert!(
        !game
            .terrain
            .iter()
            .any(|id| id == "demo.terrain.stairs-down" || id == "demo.terrain.shaft-down")
    );
    let ability = game
        .content
        .ability("demo.ability.sorcery-create-stair")
        .unwrap()
        .clone();
    let index = game
        .terrain
        .iter()
        .enumerate()
        .find(|(index, id)| {
            let position = Position {
                x: (*index % usize::from(game.width)) as i32,
                y: (*index / usize::from(game.width)) as i32,
            };
            id.as_str() == "demo.terrain.floor"
                && !game
                    .floor_connections
                    .iter()
                    .any(|connection| connection.position == position)
        })
        .unwrap()
        .0;
    game.player.position = Position {
        x: (index % usize::from(game.width)) as i32,
        y: (index / usize::from(game.width)) as i32,
    };
    let rng = game.rng.clone();
    game.resolve_player_create_stair_effect(&ability, &mut Vec::new(), &mut BTreeSet::new());
    assert_eq!(
        game.terrain_at(game.player.position),
        "demo.terrain.stairs-up"
    );
    assert_eq!(game.rng, rng); // Up is forced on a current automatic quest floor.
    game.reveal_current_visibility();
    let mut game = restored(&game);
    let departure = game.traverse_stairs(true).unwrap().unwrap();
    assert!(
        departure
            .dungeon_task_events
            .iter()
            .any(|event| matches!(event, DomainEvent::TaskAbandoned { .. }))
    );
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Abandoned);
    assert!(!game.actor_kind_is_reserved_task_target(&assignment.actor_kind_id));
    game.transition_floor(floor_id(assignment.depth + 1), None, None, false)
        .unwrap()
        .unwrap();
    game.reveal_current_visibility();
    restored(&game);
}

#[test]
fn angband_unique_task_survives_surface_reset_and_recall_is_gated() {
    let mut game = entrance_game();
    let surface = game.current_floor_id.clone();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (id, assignment) = assignments(&game).remove(0);
    game.transition_floor(floor_id(assignment.depth), None, None, false)
        .unwrap()
        .unwrap();
    game.transition_floor(surface, None, None, false)
        .unwrap()
        .unwrap();
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Paused);
    let mut game = restored(&game);
    let destination = game.recall.as_mut().unwrap().destination.as_mut().unwrap();
    destination.floor_id = floor_id(assignment.depth + 1);
    let before = game.state_hash();
    assert!(game.recall_use_plan().is_none());
    assert_eq!(game.state_hash(), before);
    game.recall
        .as_mut()
        .unwrap()
        .destination
        .as_mut()
        .unwrap()
        .floor_id = floor_id(assignment.depth);
    assert!(game.recall_use_plan().is_some());
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Active);
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == assignment.actor_kind_id)
            .count(),
        1
    );
    game.reveal_current_visibility();
    restored(&game);
}

#[test]
fn angband_oberon_gates_serpent_and_pays_source_rewards_before_monster_loot() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    complete_task_gates(&mut game);
    for id in [
        "demo.task.angband-oberon",
        "demo.task.angband-serpent-of-chaos",
    ] {
        let state = game.task_states.get_mut(id).unwrap();
        state.status = TaskStatusKindDto::Taken;
        state.current = 0;
    }
    game.transition_floor(floor_id(99), None, None, false)
        .unwrap()
        .unwrap();
    let before = game.state_hash();
    assert!(game.traverse_stairs(true).unwrap().is_none());
    assert!(
        game.transition_floor(floor_id(100), None, None, false)
            .unwrap()
            .is_none()
    );
    assert!(
        game.transition_floor(floor_id(101), None, None, false)
            .unwrap()
            .is_none()
    );
    assert_eq!(game.state_hash(), before);
    let actor = game
        .entities
        .iter()
        .find(|actor| actor.kind_id == "demo.actor.oberon-king-of-amber")
        .unwrap()
        .clone();
    let fame = game.fame;
    let mut events = task_death(&mut game, &actor.id, true);
    game.apply_campaign_events(&mut events);
    assert_eq!(game.campaign_state.status, CampaignStatusDto::Active);
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::CampaignVictorious { .. }))
    );
    assert!(game.fame >= fame + 51);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::TaskRewarded { .. }))
            .count(),
        4
    );
    let rewards = game
        .items
        .iter()
        .filter(|item| item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::AngbandReward))
        .collect::<Vec<_>>();
    assert!(!rewards.is_empty());
    assert!(
        rewards
            .iter()
            .all(|item| matches!(item.location, ItemLocation::Ground(_)))
    );
    if let Some(loot) = events
        .iter()
        .position(|event| matches!(event, DomainEvent::LootDropped { .. }))
    {
        assert!(
            events
                .iter()
                .rposition(|event| matches!(event, DomainEvent::TaskRewarded { .. }))
                .unwrap()
                < loot
        );
    }
    assert!(
        game.transition_floor(floor_id(100), None, None, false)
            .unwrap()
            .is_some()
    );
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.kind_id == "demo.actor.the-serpent-of-chaos")
    );
    assert!(
        game.transition_floor(floor_id(101), None, None, false)
            .unwrap()
            .is_none()
    );
    game.reveal_current_visibility();
    restored(&game);
}

#[test]
fn angband_dead_target_passes_without_kill_reward_or_victory_and_forged_skip_is_rejected() {
    let mut game = entrance_game();
    let (id, assignment) = assignments(&game).remove(0);
    let mut bad = game.to_save();
    bad.task_states
        .iter_mut()
        .find(|state| state.task_id == id)
        .unwrap()
        .status = TaskStatusKindDto::Skipped;
    assert!(Game::from_save(bad, Game::default_behavior_preferences()).is_err());
    game.defeated_limited_actor_counts
        .insert(assignment.actor_kind_id.clone(), 1);
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let fame = game.fame;
    let campaign = game.campaign_state;
    let outcome = game
        .transition_floor(floor_id(assignment.depth), None, None, false)
        .unwrap()
        .unwrap();
    assert!(
        outcome
            .dungeon_task_events
            .iter()
            .any(|event| matches!(event, DomainEvent::TaskSkipped { .. }))
    );
    assert_eq!(game.task_states[&id].status, TaskStatusKindDto::Skipped);
    assert_eq!(game.task_states[&id].current, 0);
    assert_eq!(game.fame, fame);
    assert_eq!(game.campaign_state, campaign);
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.origin_kind == Some(rfb_protocol::ItemOriginKindDto::AngbandReward))
    );
    assert!(!game.actor_kind_is_reserved_task_target(&assignment.actor_kind_id));
    game.reveal_current_visibility();
    restored(&game);
}

#[test]
fn angband_nonunique_task_keeps_partial_progress_and_replaces_only_remaining_targets() {
    let pack =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs/rfb-demo-original");
    let mut artifact = rfb_content::compile_pack_dir(&pack).unwrap();
    let world = artifact
        .content
        .worlds
        .iter_mut()
        .find(|world| world.id == DEFAULT_WORLD_ID)
        .unwrap();
    let task = world
        .tasks
        .iter_mut()
        .find(|task| task.id == "demo.task.angband-oberon")
        .unwrap();
    task.location = rfb_content::TaskLocationDefinition::DungeonDepth {
        dungeon_id: DUNGEON.to_owned(),
        depth: 2,
    };
    task.objectives[0].actor_kind_id = Some("demo.actor.kobold".to_owned());
    task.objectives[0].required = 3;
    let content = std::sync::Arc::new(rfb_content::ContentCatalog::from_artifact(
        rfb_content::encode_content(artifact.content).unwrap(),
    ));
    let mut game =
        Game::from_content_with_build(42, content.clone(), DEFAULT_WORLD_ID, "demo.build.warrior")
            .unwrap();
    choose_human_talent_if_pending(&mut game);
    game.transition_floor(floor_id(2), None, None, false)
        .unwrap()
        .unwrap();
    let targets = game
        .entities
        .iter()
        .filter(|actor| actor.kind_id == "demo.actor.kobold")
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 3);
    task_death(&mut game, &targets[0].id, false);
    assert_eq!(game.task_states["demo.task.angband-oberon"].current, 1);
    let before = game.state_hash();
    game.progress_dungeon_task_on_death(&targets[0], &mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.state_hash(), before);
    game.transition_floor(floor_id(1), None, None, false)
        .unwrap()
        .unwrap();
    assert!(
        game.stored_floors
            .values()
            .flat_map(|floor| &floor.entities)
            .all(|actor| actor.kind_id != "demo.actor.kobold")
    );
    game.reveal_current_visibility();
    let save = game.to_save();
    let mut game =
        Game::from_save_with_content(save, content, Game::default_behavior_preferences()).unwrap();
    game.transition_floor(floor_id(2), None, None, false)
        .unwrap()
        .unwrap();
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.kobold")
            .count(),
        2
    );
    assert_eq!(game.task_states["demo.task.angband-oberon"].current, 1);
    let targets = game
        .entities
        .iter()
        .filter(|actor| actor.kind_id == "demo.actor.kobold")
        .map(|actor| actor.id.clone())
        .collect::<Vec<_>>();
    task_death(&mut game, &targets[0], true);
    task_death(&mut game, &targets[1], false);
    assert_eq!(
        game.task_states["demo.task.angband-oberon"].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.task_states["demo.task.angband-oberon"].current, 3);
}

#[test]
fn angband_failed_target_placement_rolls_back_transition_and_rng() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    let (_, assignment) = assignments(&game).remove(0);
    let definition = game
        .content
        .world(DEFAULT_WORLD_ID)
        .unwrap()
        .procedural_floors
        .iter()
        .find(|floor| floor.id == floor_id(assignment.depth))
        .unwrap()
        .clone();
    let mut floor = game
        .generate_procedural_floor(&definition, game.current_dungeon_instance_id.clone())
        .unwrap();
    // Block every task placement cell while keeping a generated arrival map.
    floor.vault_cells.fill(true);
    let key = crate::game::floor::dungeon_instance_storage_key(
        game.current_dungeon_instance_id.as_deref(),
        &definition.id,
    );
    game.stored_floors.insert(key, floor);
    let before = game.state_hash();
    let rng = game.rng.clone();
    assert!(matches!(
        game.transition_floor(definition.id, None, None, false),
        Err(CoreError::Invariant(_))
    ));
    assert_eq!(game.state_hash(), before);
    assert_eq!(game.rng, rng);
}

fn entrance_game() -> Game {
    let mut game = Game::new_with_build(42, "demo.build.warrior").unwrap();
    choose_human_talent_if_pending(&mut game);
    // Arrival preparation only: the formal wilderness generator places the entrance.
    game.wilderness_position = Some(Position { x: 57, y: 40 });
    game.activate_wilderness_position(None, false).unwrap();
    clear_monsters(&mut game);
    place_player_on_terrain(&mut game, "demo.terrain.angband-entrance");
    game.reveal_current_visibility();
    game
}

fn traverse(game: &mut Game, terrain: &str, destination: &str) {
    clear_monsters(game);
    place_player_on_terrain(game, terrain);
    assert!(game.traverse_stairs(false).unwrap().is_some());
    assert_eq!(game.current_floor_id, destination);
    clear_monsters(game);
    game.reveal_current_visibility();
}

fn restored(game: &Game) -> Game {
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored
}

#[test]
fn angband_formal_entry_stairs_shafts_recall_and_saved_return() {
    let mut game = entrance_game();
    assert_eq!(game.progress.level, 1);
    let departure = game.player.position;
    let surface = game.current_floor_id.clone();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    traverse(&mut game, "demo.terrain.stairs-down", &floor_id(2));
    traverse(&mut game, "demo.terrain.shaft-down", &floor_id(4));
    let mut loaded = restored(&game);
    for (terrain, target) in [
        ("demo.terrain.shaft-up", floor_id(2)),
        ("demo.terrain.stairs-up", floor_id(1)),
        ("demo.terrain.stairs-up", surface.clone()),
    ] {
        traverse(&mut game, terrain, &target);
        traverse(&mut loaded, terrain, &target);
        assert_eq!(loaded.state_hash(), game.state_hash());
    }
    assert_eq!(game.player.position, departure);
    assert_eq!(game.wilderness_position, Some(Position { x: 57, y: 40 }));
    assert_eq!(
        game.dungeon_states[DUNGEON].recall_floor_id,
        Some(floor_id(4))
    );
    assert!(game.current_dungeon_instance_id.is_none());
    let mut game = restored(&game);
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.current_floor_id, floor_id(4));
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    let mut game = restored(&game);
    game.start_recall(0);
    game.advance_recall(&mut Vec::new(), &mut BTreeSet::new())
        .unwrap();
    assert_eq!(game.current_floor_id, surface);
    assert_eq!(game.player.position, departure);
    game.reveal_current_visibility();
    restored(&game);
}

#[test]
fn angband_representative_generation_keeps_connected_routes_and_source_materials() {
    let base = entrance_game();
    let mut materials = BTreeSet::new();
    let mut monsters = 0;
    for depth in [1, 50, 51, 100, 127] {
        let definition = base
            .content
            .world(DEFAULT_WORLD_ID)
            .unwrap()
            .procedural_floors
            .iter()
            .find(|floor| floor.id == floor_id(depth))
            .unwrap()
            .clone();
        let mut game = base.clone();
        let floor = game.generate_procedural_floor(&definition, None).unwrap();
        materials.extend(floor.terrain.iter().cloned());
        monsters += floor.entities.len();
        let walkable = floor
            .terrain
            .iter()
            .enumerate()
            .filter_map(|(index, id)| {
                let terrain = game.content.terrain(id).unwrap();
                (terrain.walkable || terrain.open_to_terrain_id.is_some()).then_some(Position {
                    x: (index % usize::from(definition.width)) as i32,
                    y: (index / usize::from(definition.width)) as i32,
                })
            })
            .collect::<BTreeSet<_>>();
        let reached = maze_floor_distances(&walkable, floor.player_position);
        for connection in &floor.connections {
            assert!(
                reached.contains_key(&connection.position),
                "depth {depth}: {}",
                connection.id
            );
        }
        assert_eq!(
            floor.connections.iter().any(|connection| {
                let index = connection.position.y as usize * usize::from(definition.width)
                    + connection.position.x as usize;
                game.content
                    .terrain(&floor.terrain[index])
                    .unwrap()
                    .tags
                    .iter()
                    .any(|tag| tag == "stairs-down")
            }),
            depth != 127
        );
        if depth == 127 {
            assert!(floor.terrain.iter().all(|id| !matches!(
                id.as_str(),
                "demo.terrain.stairs-down" | "demo.terrain.shaft-down"
            )));
        }
    }
    assert!(monsters > 0);
    assert!(materials.contains("demo.terrain.magma-vein"));
    assert!(materials.contains("demo.terrain.quartz-vein"));
}

#[test]
fn angband_bottom_ascent_and_next_generation_survive_save() {
    let mut game = entrance_game();
    traverse(&mut game, "demo.terrain.angband-entrance", &floor_id(1));
    // Bottom-route coverage starts after the task gates have been completed.
    complete_task_gates(&mut game);
    let serpent = game
        .task_states
        .get_mut("demo.task.angband-serpent-of-chaos")
        .unwrap();
    serpent.status = TaskStatusKindDto::Completed;
    serpent.current = serpent.required;
    game.apply_campaign_events(&mut Vec::new());
    game.turn += 1;
    assert!(
        game.transition_floor(floor_id(127), None, None, false)
            .unwrap()
            .is_some()
    );
    clear_monsters(&mut game);
    game.reveal_current_visibility();
    let mut loaded = restored(&game);
    traverse(&mut game, "demo.terrain.shaft-up", &floor_id(125));
    traverse(&mut loaded, "demo.terrain.shaft-up", &floor_id(125));
    assert_eq!(loaded.state_hash(), game.state_hash());
    traverse(&mut game, "demo.terrain.shaft-down", &floor_id(127));
    traverse(&mut loaded, "demo.terrain.shaft-down", &floor_id(127));
    assert_eq!(loaded.state_hash(), game.state_hash());
    restored(&game);
}
