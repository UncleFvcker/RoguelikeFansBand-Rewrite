// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

fn direct_warrens_death_drops(
    actor_kind_id: &str,
    seed: u64,
) -> (Vec<ItemInstance>, Vec<GoldPile>) {
    let mut game =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    game.current_floor_id = "demo.floor.warrens-depth-1".to_owned();
    game.rng = RfbRng::seeded(seed);
    let existing_item_ids = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    let existing_gold_ids = game
        .gold_piles
        .iter()
        .map(|pile| pile.id.clone())
        .collect::<BTreeSet<_>>();
    let actor_id = format!("test.{actor_kind_id}.{seed}");
    let actor = game.generated_actor(actor_id, actor_kind_id, game.player.position);
    game.entities.push(actor);
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed = Vec::new();
    game.resolve_actor_death(
        game.entities.len() - 1,
        DomainEvent::Waited,
        &mut events,
        &mut changed,
        &mut removed,
    )
    .expect("direct monster death should resolve");
    (
        game.items
            .into_iter()
            .filter(|item| !existing_item_ids.contains(&item.id))
            .collect(),
        game.gold_piles
            .into_iter()
            .filter(|pile| !existing_gold_ids.contains(&pile.id))
            .collect(),
    )
}

#[test]
fn warrens_keeper_drop_count_is_one_d_two_and_items_only() {
    let mut saw_one = false;
    let mut saw_two = false;
    for seed in 0..64 {
        let (drops, gold) = direct_warrens_death_drops("demo.actor.warrens-keeper", seed);
        let equipment = drops
            .iter()
            .filter(|item| {
                !matches!(
                    item.kind_id.as_str(),
                    "demo.item.corpse-remains" | "demo.item.skeleton-remains"
                )
            })
            .collect::<Vec<_>>();
        assert!(gold.is_empty());
        assert!(matches!(equipment.len(), 1 | 2));
        assert!(equipment.iter().all(|item| matches!(
            item.quality,
            ItemQualityDto::Fine | ItemQualityDto::Exceptional
        )));
        saw_one |= equipment.len() == 1;
        saw_two |= equipment.len() == 2;
    }
    assert!(saw_one && saw_two);
}

#[test]
fn warrens_monster_drops_follow_original_probability_and_remains_profiles() {
    let is_remains = |item: &ItemInstance| {
        matches!(
            item.kind_id.as_str(),
            "demo.item.corpse-remains" | "demo.item.skeleton-remains"
        )
    };
    let mut saw_kobold_drop = false;
    let mut saw_kobold_gold = false;
    let mut saw_kobold_no_drop = false;
    let mut saw_no_remains = false;
    let mut saw_corpse = false;
    let mut saw_skeleton = false;

    for seed in 0..128 {
        let (drops, gold) = direct_warrens_death_drops("demo.actor.small-kobold", seed);
        let ordinary_drop_count = drops.iter().filter(|item| !is_remains(item)).count();
        assert!(ordinary_drop_count <= 1);
        assert!(gold.len() <= 1);
        assert!(ordinary_drop_count == 0 || gold.is_empty());
        saw_kobold_drop |= ordinary_drop_count == 1;
        saw_kobold_gold |= gold.len() == 1;
        saw_kobold_no_drop |= ordinary_drop_count == 0;
        saw_no_remains |= drops.iter().all(|item| !is_remains(item));
        saw_corpse |= drops
            .iter()
            .any(|item| item.kind_id == "demo.item.corpse-remains");
        saw_skeleton |= drops
            .iter()
            .any(|item| item.kind_id == "demo.item.skeleton-remains");
    }

    assert!(saw_kobold_drop && saw_kobold_gold && saw_kobold_no_drop);
    assert!(saw_no_remains && saw_corpse && saw_skeleton);
    assert_eq!(
        direct_warrens_death_drops("demo.actor.small-kobold", 42),
        direct_warrens_death_drops("demo.actor.small-kobold", 42)
    );

    for actor_kind_id in ["demo.actor.giant-white-mouse", "demo.actor.warg"] {
        for seed in 0..32 {
            let (drops, gold) = direct_warrens_death_drops(actor_kind_id, seed);
            assert!(gold.is_empty());
            assert!(drops.iter().all(is_remains));
            if actor_kind_id == "demo.actor.giant-white-mouse" {
                assert!(
                    drops
                        .iter()
                        .all(|item| item.kind_id != "demo.item.skeleton-remains")
                );
            }
        }
    }

    let mut surface =
        Game::new_with_build(1, "demo.build.warrior").expect("Warrens journey should create");
    surface.rng = RfbRng::seeded(42);
    let draws_before = surface.rng_draw_counter();
    let outside_depth = surface
        .generate_loot_instances(
            &LootContext {
                table_id: "demo.loot-table.small-kobold".to_owned(),
                floor_id: "demo.floor.surface".to_owned(),
                depth: 0,
                source: LootSource::MonsterDeath {
                    actor_id: "test.small-kobold.surface".to_owned(),
                },
            },
            ItemLocation::Ground(surface.player.position),
        )
        .expect("an out-of-depth loot table should resolve without candidates");
    assert!(outside_depth.is_empty());
    assert_eq!(surface.rng_draw_counter(), draws_before);
}

#[test]
fn campaign_victory_plan_commits_ordered_events_once_without_rng() {
    let mut game = Game::new(42);
    let victory_dungeon_ids = game
        .campaign_definition()
        .expect("demo world should define a campaign")
        .victory_dungeon_ids
        .clone();
    for dungeon_id in victory_dungeon_ids {
        game.dungeon_states
            .get_mut(&dungeon_id)
            .expect("victory dungeon state should exist")
            .guardian_defeated = true;
    }
    let draws_before = game.rng_draw_counter();
    let victory_turn = game.turn.saturating_add(1);
    let mut events = Vec::new();

    game.apply_campaign_events(&mut events);

    assert_eq!(game.campaign_state.status, CampaignStatusDto::Victorious);
    assert_eq!(game.campaign_state.victory_turn, Some(victory_turn));
    assert!(matches!(
        events.as_slice(),
        [
            DomainEvent::CampaignVictorious { .. },
            DomainEvent::PlayerLevelCapUnlocked { .. }
        ]
    ));
    assert_eq!(game.rng_draw_counter(), draws_before);

    game.apply_campaign_events(&mut events);
    assert_eq!(events.len(), 2);
    assert_eq!(game.rng_draw_counter(), draws_before);
}

#[test]
fn external_task_service_projects_sparse_available_state_and_accepts_at_entrance() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(
        game.snapshot()
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("external task should be projected")
            .status,
        TaskStatusKindDto::Available
    );
    assert!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .is_empty()
    );

    game.player.position = Position { x: 26, y: 13 };
    let before_draws = game.rng_draw_counter();
    let snapshot = game.snapshot();
    let service = snapshot
        .task_services
        .iter()
        .find(|service| service.id == "demo.town-facility.outpost-count")
        .expect("test task service should be projected");
    assert!(service.player_at_entrance);
    assert_eq!(
        service
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("external task should be projected at the service")
            .status,
        TaskStatusKindDto::Available
    );
    let update = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Taken);
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.accepted")
    );
    assert_eq!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("accepted task should remain projected")
            .status,
        TaskStatusKindDto::Taken
    );
}

#[test]
fn external_task_prerequisite_stays_locked_without_materializing_state() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-prerequisite";
    game.player.position = Position { x: 26, y: 13 };
    let before_draws = game.rng_draw_counter();
    assert_eq!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("test task service should be projected")
            .tasks
            .iter()
            .find(|task| task.task_id == task_id)
            .expect("dependent task should be projected")
            .status,
        TaskStatusKindDto::Locked
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.accept-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn accepted_external_task_binds_while_inside_its_dungeon_depth() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    game.player.position = Position { x: 26, y: 13 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.warrens-depth-1");
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Active);
    assert_eq!(
        game.task_states[task_id].active_floor_id.as_deref(),
        Some("demo.floor.warrens-depth-1")
    );

    place_player_on_terrain(&mut game, "demo.terrain.stairs-up");
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Failed);
    assert!(game.task_states[task_id].active_floor_id.is_none());
}

#[test]
fn external_task_service_rejects_unavailable_commands_without_rng_or_state_changes() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    let before_draws = game.rng_draw_counter();
    let accept = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        accept
            .events
            .iter()
            .any(|event| event.kind == "task.accept-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng_draw_counter(), before_draws);

    game.player.position = Position { x: 26, y: 13 };
    let claim = dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert!(
        claim
            .events
            .iter()
            .any(|event| event.kind == "task.reward-claim-unavailable")
    );
    assert!(!game.task_states.contains_key(task_id));
    assert_eq!(game.rng.draw_counter, before_draws);
}

#[test]
fn task_rewards_use_one_weighted_default_choice_and_class_affix_overrides() {
    let mut saw_food = false;
    let mut saw_water = false;
    for seed in 0..32 {
        let mut game = task_service_game(seed);
        game.player.position = Position { x: 26, y: 13 };
        game.task_states.insert(
            "demo.task.test-warrens-depth".to_owned(),
            TaskState {
                status: TaskStatusKindDto::RewardAvailable,
                stage_index: 0,
                current: 1,
                required: 1,
                active_floor_id: None,
                retakes_used: 0,
            },
        );
        let before_draws = game.rng_draw_counter();
        let reward = game
            .claim_task_reward(
                "demo.town-facility.outpost-count",
                "demo.task.test-warrens-depth",
            )
            .expect("weighted task reward should resolve");
        assert_eq!(game.rng_draw_counter(), before_draws + 1);
        saw_food |= reward.item_kind_id == "demo.item.ration-of-food";
        saw_water |= reward.item_kind_id == "demo.item.water-potion";
    }
    assert!(saw_food && saw_water);

    let mut game = task_service_game(42);
    game.player.position = Position { x: 26, y: 13 };
    game.task_states.insert(
        "demo.task.test-prerequisite".to_owned(),
        TaskState {
            status: TaskStatusKindDto::RewardAvailable,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    let reward = game
        .claim_task_reward(
            "demo.town-facility.outpost-count",
            "demo.task.test-prerequisite",
        )
        .expect("Warrior reward override should resolve");
    assert_eq!(reward.item_kind_id, "demo.item.broad-sword");
    let item = game
        .items
        .iter()
        .find(|item| item.id == "demo.task.test-prerequisite.reward.1")
        .expect("fixed reward instance should enter inventory");
    assert_eq!(item.quality, ItemQualityDto::Fine);
    assert_eq!(item.affix_ids, ["rfb-legacy.affix.combat"]);
    assert_eq!(item.rolled_affixes.len(), 1);
}

#[test]
fn accepting_thieves_hideout_at_the_count_opens_only_its_northeastern_entry() {
    let mut game =
        Game::new_with_build(42, "demo.build.warrior").expect("Warrens journey should create");
    let entry = Position { x: 63, y: 11 };
    assert_eq!(
        game.terrain_at(entry),
        "demo.terrain.thieves-hideout-entry-available"
    );
    game.player.position = Position { x: 26, y: 13 };
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::Taken
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.thieves-hideout-entry");
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn clearing_thieves_hideout_closes_the_floor_without_granting_the_reward() {
    let mut game =
        Game::new_with_build(43, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );
    game.player.position = Position { x: 63, y: 11 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    assert_eq!(game.current_floor_id, "demo.floor.thieves-hideout");

    game.entities.clear();
    dispatch_next(&mut game, GameCommand::Wait);
    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::RewardAvailable
    );
    game.player.position = Position { x: 1, y: 4 };
    let returned = dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::RewardAvailable
    );
    assert_eq!(
        game.terrain_at(Position { x: 63, y: 11 }),
        "demo.terrain.thieves-hideout-entry-completed"
    );
    assert!(
        returned
            .events
            .iter()
            .any(|event| event.kind == "task.reward-available")
    );
    assert!(
        !game
            .items
            .iter()
            .any(|item| item.id == "demo.task.thieves-hideout.reward.1")
    );
}

#[test]
fn leaving_thieves_hideout_uncleared_fails_and_closes_the_entry() {
    let mut game =
        Game::new_with_build(44, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );
    game.player.position = Position { x: 63, y: 11 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);
    game.player.position = Position { x: 1, y: 4 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::Failed
    );
    assert_eq!(
        game.terrain_at(Position { x: 63, y: 11 }),
        "demo.terrain.thieves-hideout-entry-failed"
    );
}

#[test]
fn count_grants_the_warrior_broad_sword_only_when_claimed() {
    let mut game =
        Game::new_with_build(45, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::RewardAvailable,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.thieves-hideout".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.thieves-hideout"].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(game.items.iter().any(|item| {
        item.id == "demo.task.thieves-hideout.reward.1"
            && item.kind_id == "demo.item.broad-sword"
            && item.location == ItemLocation::Inventory
    }));
}

fn pest_control_state(status: TaskStatusKindDto, current: u32) -> TaskState {
    TaskState {
        status,
        stage_index: 0,
        current,
        required: 8,
        active_floor_id: (status == TaskStatusKindDto::Active)
            .then(|| "demo.floor.warrens-depth-5".to_owned()),
        retakes_used: 0,
    }
}

fn generate_pest_control_floor(game: &mut Game) -> FloorState {
    let definition = game
        .content
        .world(&game.world_id)
        .expect("Middle-earth world should remain available")
        .procedural_floors
        .iter()
        .find(|floor| floor.id == "demo.floor.warrens-depth-5")
        .expect("Warrens depth 5 should remain available")
        .clone();
    game.generate_procedural_floor(&definition, None)
        .expect("Pest Control floor should generate")
}

#[test]
fn pest_control_unlocks_only_after_the_thieves_reward_is_claimed() {
    let mut game =
        Game::new_with_build(50, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    let task_id = "demo.task.pest-control";
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project task service")
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
            .expect("Pest Control should be projected")
            .status,
        TaskStatusKindDto::Locked
    );

    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project task service")
            .tasks
            .into_iter()
            .find(|task| task.task_id == task_id)
            .expect("Pest Control should be projected")
            .status,
        TaskStatusKindDto::Available
    );
}

#[test]
fn count_accepts_pest_control_without_advancing_rng() {
    let mut game =
        Game::new_with_build(51, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    let before_draws = game.rng_draw_counter();
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.pest-control".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::Taken
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn pest_control_floor_places_the_remaining_wargs_and_hides_downstairs() {
    for (current, expected_wargs) in [(0, 8), (3, 5)] {
        let mut game =
            Game::new_with_build(52, "demo.build.warrior").expect("Warrens journey should create");
        game.task_states.insert(
            "demo.task.pest-control".to_owned(),
            pest_control_state(TaskStatusKindDto::Taken, current),
        );
        let floor = generate_pest_control_floor(&mut game);
        let wargs = floor
            .entities
            .iter()
            .filter(|entity| entity.kind_id == "demo.actor.warg")
            .collect::<Vec<_>>();

        assert_eq!(wargs.len(), expected_wargs);
        assert!(
            wargs
                .iter()
                .all(|warg| { chebyshev_distance(floor.player_position, warg.position) >= 10 })
        );
        assert!(
            !floor
                .terrain
                .iter()
                .any(|terrain_id| terrain_id == "demo.terrain.stairs-down")
        );
    }
}

#[test]
fn final_pest_control_kill_reveals_a_magic_stair_without_rng() {
    let mut game =
        Game::new_with_build(53, "demo.build.warrior").expect("Warrens journey should create");
    game.task_states.insert(
        "demo.task.pest-control".to_owned(),
        pest_control_state(TaskStatusKindDto::Active, 7),
    );
    let floor = generate_pest_control_floor(&mut game);
    game.current_floor_id = floor.id;
    game.width = floor.width;
    game.height = floor.height;
    game.terrain = floor.terrain;
    game.player.position = floor.player_position;
    game.entities = floor.entities;
    game.items.extend(floor.items);
    game.gold_piles = floor.gold_piles;
    game.floor_connections = floor.connections;
    game.floor_regions = floor.regions;
    let death_position = game
        .terrain
        .iter()
        .enumerate()
        .filter_map(|(index, terrain_id)| {
            let position = Position {
                x: i32::try_from(index % usize::from(game.width)).ok()?,
                y: i32::try_from(index / usize::from(game.width)).ok()?,
            };
            (terrain_id == "demo.terrain.floor"
                && !game.entities.iter().any(|actor| actor.position == position)
                && !game.items.iter().any(
                    |item| matches!(item.location, ItemLocation::Ground(ground) if ground == position),
                ))
            .then_some(position)
        })
        .max_by_key(|position| chebyshev_distance(game.player.position, *position))
        .expect("Pest Control floor should retain a remote empty floor tile");
    let before_draws = game.rng_draw_counter();
    game.command_actor_deaths.push(ActorDeathRecord {
        actor_id: "task-target.warg".to_owned(),
        actor_kind_id: "demo.actor.warg".to_owned(),
        position: death_position,
        credit_player: true,
    });
    let mut events = Vec::new();
    game.apply_task_events(&mut events)
        .expect("final Warg kill should complete Pest Control");

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::RewardAvailable
    );
    assert_eq!(
        game.terrain[usize::try_from(death_position.y).expect("non-negative y")
            * usize::from(game.width)
            + usize::try_from(death_position.x).expect("non-negative x")],
        "demo.terrain.stairs-down"
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::TaskExitRevealed { .. }))
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
}

#[test]
fn leaving_pest_control_incomplete_fails_and_discards_the_blocked_floor() {
    let mut game =
        Game::new_with_build(54, "demo.build.warrior").expect("Warrens journey should create");
    game.task_states.insert(
        "demo.task.pest-control".to_owned(),
        pest_control_state(TaskStatusKindDto::Active, 3),
    );
    let floor = generate_pest_control_floor(&mut game);
    game.stored_floors
        .insert("test.warrens.5".to_owned(), floor);
    game.current_floor_id = "demo.floor.warrens-depth-4".to_owned();
    let mut events = vec![DomainEvent::FloorTransitioned {
        from_floor_id: "demo.floor.warrens-depth-5".to_owned(),
        to_floor_id: "demo.floor.warrens-depth-4".to_owned(),
    }];
    game.apply_task_events(&mut events)
        .expect("early departure should resolve");

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::Failed
    );
    assert_eq!(
        game.stored_floors
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["demo.floor.surface"]
    );
}

#[test]
fn count_grants_the_fur_cloak_only_when_pest_control_is_claimed() {
    let mut game =
        Game::new_with_build(55, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    game.task_states.insert(
        "demo.task.thieves-hideout".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    game.task_states.insert(
        "demo.task.pest-control".to_owned(),
        pest_control_state(TaskStatusKindDto::RewardAvailable, 8),
    );
    dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.pest-control".to_owned(),
        },
    );

    assert_eq!(
        game.task_states["demo.task.pest-control"].status,
        TaskStatusKindDto::Completed
    );
    assert!(game.items.iter().any(|item| {
        item.id == "demo.task.pest-control.reward.1"
            && item.kind_id == "demo.item.fur-cloak"
            && item.location == ItemLocation::Inventory
    }));
}

#[test]
fn count_follow_up_tasks_unlock_in_the_original_order() {
    let mut game =
        Game::new_with_build(56, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    let completed = |required| TaskState {
        status: TaskStatusKindDto::Completed,
        stage_index: 0,
        current: required,
        required,
        active_floor_id: None,
        retakes_used: 0,
    };
    let sequence = [
        ("demo.task.thieves-hideout", "demo.task.pest-control", 1),
        ("demo.task.pest-control", "demo.task.the-sewer", 8),
        ("demo.task.the-sewer", "demo.task.haunted-house", 1),
        ("demo.task.haunted-house", "demo.task.royal-crypt", 1),
    ];
    assert_eq!(
        game.snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project its task service")
            .tasks
            .first()
            .map(|task| task.task_id.as_str()),
        Some("demo.task.thieves-hideout")
    );

    for (prerequisite, unlocked, required) in sequence {
        game.task_states
            .insert(prerequisite.to_owned(), completed(required));
        let service = game
            .snapshot()
            .task_services
            .into_iter()
            .find(|service| service.id == "demo.town-facility.outpost-count")
            .expect("Count should project its task service");
        assert_eq!(
            service
                .tasks
                .into_iter()
                .find(|task| task.task_id == unlocked)
                .expect("next Count task should be projected")
                .status,
            TaskStatusKindDto::Available
        );
    }
}

#[test]
fn royal_crypt_places_five_archliches_on_its_level_seventy_fixed_floor() {
    let mut game =
        Game::new_with_build(57, "demo.build.warrior").expect("Warrens journey should create");
    game.player.position = Position { x: 26, y: 13 };
    game.task_states.insert(
        "demo.task.haunted-house".to_owned(),
        TaskState {
            status: TaskStatusKindDto::Completed,
            stage_index: 0,
            current: 1,
            required: 1,
            active_floor_id: None,
            retakes_used: 0,
        },
    );
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-count".to_owned(),
            task_id: "demo.task.royal-crypt".to_owned(),
        },
    );
    game.player.position = Position { x: 28, y: 9 };
    dispatch_next(&mut game, GameCommand::TraverseStairs);

    assert_eq!(game.current_floor_id, "demo.floor.outpost-royal-crypt");
    assert_eq!(game.floor_depth(&game.current_floor_id), 70);
    assert_eq!(
        game.entities
            .iter()
            .filter(|actor| actor.kind_id == "demo.actor.archlich")
            .count(),
        5
    );
    assert_eq!(
        game.task_states["demo.task.royal-crypt"].status,
        TaskStatusKindDto::Active
    );
}

#[test]
fn warrens_dungeon_conquest_returns_retires_and_round_trips() {
    let mut game =
        Game::new_with_build(49, "demo.build.warrior").expect("Warrens journey should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    assert_eq!(game.world_id, DEFAULT_WORLD_ID);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(game.campaign_state.status, CampaignStatusDto::Active);

    for depth in 1..=9 {
        place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(update.floor_id, format!("demo.floor.warrens-depth-{depth}"));
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.warrens.1")
        .expect("Warrens depth 9 should spawn its guardian");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.warrens-keeper"
    );
    let guardian_position = game.entities[guardian_index].position;
    let item_ids_before_guardian = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
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

    let victory = dispatch_next(&mut game, GameCommand::Wait);
    let guardian_event = victory
        .events
        .iter()
        .position(|event| event.kind == "dungeon.guardian-defeated")
        .expect("guardian death should conquer the Warrens");
    let victory_event = victory
        .events
        .iter()
        .position(|event| event.kind == "campaign.victorious")
        .expect("Warrens conquest should win the journey");
    assert!(guardian_event < victory_event);
    assert_eq!(victory.campaign.status, CampaignStatusDto::Victorious);
    assert_eq!(victory.campaign.conquered_dungeons, 1);
    assert_eq!(victory.campaign.score, 60_000);
    let guardian_drops = game
        .items
        .iter()
        .filter(|item| !item_ids_before_guardian.contains(&item.id))
        .collect::<Vec<_>>();
    assert!(
        guardian_drops
            .iter()
            .all(|item| { item.location == ItemLocation::Ground(guardian_position) })
    );
    assert_eq!(
        guardian_drops
            .iter()
            .filter(|item| item.kind_id == "demo.item.swiftstep-tonic")
            .count(),
        1
    );
    let fine_equipment = guardian_drops
        .iter()
        .filter(|item| item.quality == ItemQualityDto::Fine)
        .collect::<Vec<_>>();
    assert!((1..=2).contains(&fine_equipment.len()));
    let allowed_guardian_drop_kinds = ["demo.loot-table.warrens", "demo.loot-table.warrens-keeper"]
        .into_iter()
        .flat_map(|table_id| {
            game.content
                .loot_table(table_id)
                .expect("guardian drop table should remain available")
                .entries
                .iter()
                .map(|entry| entry.item_kind_id.clone())
        })
        .collect::<BTreeSet<_>>();
    assert!(
        fine_equipment
            .iter()
            .all(|item| allowed_guardian_drop_kinds.contains(&item.kind_id))
    );

    let victorious_hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("victory should round-trip");
    assert_eq!(restored.world_id, DEFAULT_WORLD_ID);
    assert_eq!(restored.state_hash(), victorious_hash);

    for expected_depth in (1..=8).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.warrens-depth-{expected_depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Victorious);
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(surface.campaign.status, CampaignStatusDto::Victorious);

    let retirement = dispatch_next(&mut restored, GameCommand::Retire);
    assert_eq!(retirement.campaign.status, CampaignStatusDto::Retired);
    assert!(
        retirement
            .events
            .iter()
            .any(|event| event.kind == "campaign.retired")
    );
    let retired_hash = restored.state_hash();
    let retired = Game::from_save(restored.to_save()).expect("retirement should round-trip");
    assert_eq!(retired.state_hash(), retired_hash);
    assert_eq!(
        retired.snapshot().campaign.status,
        CampaignStatusDto::Retired
    );
}

#[test]
fn orc_cave_guardian_conquest_reward_and_surface_return_round_trip() {
    let mut game =
        Game::new_with_build(1185, "demo.build.warrior").expect("Middle-earth should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    dispatch_next(
        &mut game,
        GameCommand::EnterWorldMap {
            leave_pets: false,
            cancel_recall: false,
        },
    );
    game.wilderness_position = Some(Position { x: 30, y: 45 });
    dispatch_next(&mut game, GameCommand::LeaveWorldMap);
    assert_eq!(game.current_floor_id, wilderness::WILDERNESS_FLOOR_ID);
    place_player_on_terrain(&mut game, "demo.terrain.orc-cave-entrance");

    for depth in 15..=32 {
        let update = dispatch_next(&mut game, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.orc-cave-depth-{depth}")
        );
        assert_eq!(update.campaign.status, CampaignStatusDto::Active);
        if depth < 32 {
            game.entities.clear();
            place_player_on_terrain(&mut game, "demo.terrain.stairs-down");
        }
    }

    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.orc-cave.1")
        .expect("Orc Cave depth 32 should spawn Othrod");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.othrod-lord-of-the-orcs"
    );
    let guardian_position = game.entities[guardian_index].position;
    let item_ids_before_guardian = game
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<BTreeSet<_>>();
    game.entities[guardian_index].hp = 1;
    game.entities[guardian_index].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
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

    let conquered = dispatch_next(&mut game, GameCommand::Wait);
    assert!(
        conquered
            .events
            .iter()
            .any(|event| event.kind == "dungeon.guardian-defeated")
    );
    assert!(
        conquered
            .events
            .iter()
            .all(|event| event.kind != "campaign.victorious")
    );
    assert_eq!(conquered.campaign.status, CampaignStatusDto::Active);
    assert_eq!(conquered.campaign.conquered_dungeons, 1);
    assert_eq!(conquered.campaign.score, 10_000);
    assert!(game.dungeon_states["demo.dungeon.orc-cave"].guardian_defeated);

    let combat_ring = game
        .items
        .iter()
        .filter(|item| !item_ids_before_guardian.contains(&item.id))
        .find(|item| item.kind_id == "demo.item.ring")
        .expect("Othrod should drop the fixed Combat ring");
    assert_eq!(
        combat_ring.location,
        ItemLocation::Ground(guardian_position)
    );
    assert_eq!(combat_ring.quality, ItemQualityDto::Fine);
    assert_eq!(combat_ring.affix_ids, ["rfb-legacy.affix.combat"]);
    assert_eq!(combat_ring.rolled_affixes.len(), 1);
    assert_eq!(
        combat_ring.rolled_affixes[0].affix_id,
        "rfb-legacy.affix.combat"
    );

    let conquered_hash = game.state_hash();
    let mut restored = Game::from_save(game.to_save()).expect("Orc Cave should round-trip");
    assert_eq!(restored.state_hash(), conquered_hash);
    assert!(restored.dungeon_states["demo.dungeon.orc-cave"].guardian_defeated);

    restored.entities.clear();
    for expected_depth in (15..=31).rev() {
        place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
        let update = dispatch_next(&mut restored, GameCommand::TraverseStairs);
        assert_eq!(
            update.floor_id,
            format!("demo.floor.orc-cave-depth-{expected_depth}")
        );
        restored.entities.clear();
    }
    place_player_on_terrain(&mut restored, "demo.terrain.stairs-up");
    let surface = dispatch_next(&mut restored, GameCommand::TraverseStairs);
    assert_eq!(surface.floor_id, wilderness::WILDERNESS_FLOOR_ID);
    assert_eq!(
        restored.wilderness_position,
        Some(Position { x: 30, y: 45 })
    );
    assert_eq!(surface.campaign.status, CampaignStatusDto::Active);
}
