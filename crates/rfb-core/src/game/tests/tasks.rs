// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn content_driven_loot_generation_is_deterministic_and_persistent() {
    let mut left = Game::new(42);
    let initial = left.to_save();
    assert_eq!(initial.carried_items.len(), 1);
    assert_eq!(initial.carried_items[0].id, "generated.item.1");
    assert_eq!(
        initial.carried_items[0].actor_id,
        "demo.monster.ember-mote.1"
    );
    assert_eq!(initial.carried_items[0].kind_id, "demo.item.echo-charm");
    assert_eq!(left.snapshot().items.len(), 5);
    assert_eq!(left.rng.draw_counter, 3);
    left.entities[0].statuses = vec![StatusInstance {
        kind_id: STATUS_POISON.to_owned(),
        intensity: 3,
        remaining_ticks: 1,
        source_id: Some(left.player.id.clone()),
        granted_resistances: BTreeMap::new(),
        granted_brands: BTreeSet::new(),
        granted_modifiers: StatModifiersDto::default(),
        granted_equipment_bonuses: EquipmentBonusesDto::default(),
        granted_status_immunities: BTreeSet::new(),
        granted_race_id: None,
        grants_wall_passage: false,
        incoming_damage_percent: 100,
    }];
    let mut right = left.clone();
    let death_position = left.entities[0].position;

    let left_update = left
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("loot-bearing monster death should execute");
    let right_update = right
        .dispatch(command(1, 0, GameCommand::Wait))
        .expect("same loot context should execute");

    assert_eq!(left_update.state_hash, right_update.state_hash);
    assert_eq!(left.rng.draw_counter, 6);
    assert_eq!(left.rng.draw_counter, right.rng.draw_counter);
    let drops = left_update
        .events
        .iter()
        .filter(|event| event.message_key == "loot-drop")
        .collect::<Vec<_>>();
    assert_eq!(drops.len(), 2);
    assert_eq!(drops[0].args["target"], "demo.item.echo-charm");
    assert_eq!(drops[1].args["source"], "demo.actor.ember-mote");
    let carried = left
        .items
        .iter()
        .find(|item| item.id == "generated.item.1")
        .expect("carried loot should preserve its stable item ID");
    assert_eq!(carried.location, ItemLocation::Ground(death_position));
    assert_eq!(carried.kind_id, "demo.item.echo-charm");
    let generated = left
        .items
        .iter()
        .find(|item| item.id == "generated.item.2")
        .expect("death loot should allocate the next stable item ID");
    assert_eq!(generated.location, ItemLocation::Ground(death_position));
    assert_eq!(generated.quantity, 1);
    assert_eq!(generated.kind_id, "demo.item.echo-charm");
    assert_eq!(generated.quality, ItemQualityDto::Ordinary);
    assert!(generated.affix_ids.is_empty());
    let restored = Game::from_save(left.to_save()).expect("generated loot should reload");
    assert_eq!(restored.state_hash(), left.state_hash());
}

fn direct_warrens_death_drops(
    actor_kind_id: &str,
    seed: u64,
) -> (Vec<ItemInstance>, Vec<GoldPile>) {
    let mut game = Game::new_warrens_journey_with_build(1, "demo.build.warrior")
        .expect("Warrens journey should create");
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

    let mut surface = Game::new_warrens_journey_with_build(1, "demo.build.warrior")
        .expect("Warrens journey should create");
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
fn carried_item_save_rejects_a_missing_monster_owner() {
    let mut payload = Game::new(42).to_save();
    payload.carried_items[0].actor_id = "demo.monster.missing".to_owned();

    assert!(matches!(
        Game::from_save(payload),
        Err(CoreError::InvalidSave("carried item state is invalid"))
    ));
}

#[test]
fn paused_task_can_be_abandoned_from_the_surface() {
    let mut game = Game::new(27);
    let entry = Position { x: 4, y: 4 };
    game.player.position = entry;
    game.traverse_stairs(false)
        .expect("bounty entry should resolve")
        .expect("bounty entry should transition");
    game.traverse_stairs(false)
        .expect("bounty pause should resolve")
        .expect("bounty pause should return to the surface");

    let paused = game
        .snapshot()
        .tasks
        .into_iter()
        .find(|task| task.task_id == "demo.task.echo-bounty")
        .expect("bounty task should be projected");
    assert_eq!(paused.status, TaskStatusKindDto::Paused);
    assert_eq!((paused.retakes_used, paused.max_retakes), (0, Some(1)));
    assert!(
        game.stored_floors
            .contains_key("demo.floor.echo-bounty-rift")
    );

    let update = dispatch_next(
        &mut game,
        GameCommand::AbandonPausedTask {
            task_id: "demo.task.echo-bounty".to_owned(),
        },
    );
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(
        game.task_states["demo.task.echo-bounty"].status,
        TaskStatusKindDto::Abandoned
    );
    assert!(
        !game
            .stored_floors
            .contains_key("demo.floor.echo-bounty-rift")
    );
    assert_eq!(game.terrain_at(entry), "demo.terrain.bounty-rift-abandoned");
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.abandoned")
    );
    assert!(
        update
            .changed_cells
            .iter()
            .any(|cell| cell.position == entry
                && cell.terrain_id == "demo.terrain.bounty-rift-abandoned")
    );
    Game::from_save(game.to_save()).expect("surface abandonment should round-trip");
}

#[test]
fn regenerated_retake_preserves_progress_and_enforces_the_limit() {
    let mut game = Game::new(27);
    game.player.position = Position { x: 4, y: 4 };
    game.traverse_stairs(false)
        .expect("bounty entry should resolve")
        .expect("bounty entry should transition");
    assert_eq!(game.entities.len(), 2);
    game.entities.pop();
    game.task_states
        .get_mut("demo.task.echo-bounty")
        .expect("bounty task should exist")
        .current = 1;
    game.traverse_stairs(false)
        .expect("partial bounty pause should resolve")
        .expect("partial bounty should return to the surface");
    assert_eq!(
        game.stored_floors["demo.floor.echo-bounty-rift"]
            .entities
            .len(),
        1
    );
    let draws_before_retake = game.rng.draw_counter;

    game.traverse_stairs(false)
        .expect("first bounty retake should resolve")
        .expect("first bounty retake should regenerate the floor");
    assert!(game.rng.draw_counter > draws_before_retake);
    assert_eq!(game.entities.len(), 1);
    let active = &game.task_states["demo.task.echo-bounty"];
    assert_eq!(active.status, TaskStatusKindDto::Active);
    assert_eq!(
        (active.current, active.required, active.retakes_used),
        (1, 2, 1)
    );

    game.traverse_stairs(false)
        .expect("second bounty pause should resolve")
        .expect("second bounty pause should return to the surface");
    let draws_before_rejected_retake = game.rng.draw_counter;
    assert!(
        game.traverse_stairs(false)
            .expect("exhausted bounty entry should resolve")
            .is_none()
    );
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.rng.draw_counter, draws_before_rejected_retake);

    let mut invalid = game.to_save();
    invalid
        .task_states
        .iter_mut()
        .find(|state| state.task_id == "demo.task.echo-bounty")
        .expect("bounty save state should exist")
        .retakes_used = 2;
    assert!(matches!(
        Game::from_save(invalid),
        Err(CoreError::InvalidSave("task state is invalid"))
    ));
}

#[test]
fn dungeon_guardian_state_rejects_missing_state_and_entity_mismatch() {
    let mut current_payload = Game::new(42).to_save();
    current_payload
        .dungeon_states
        .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
    assert!(matches!(
        Game::from_save(current_payload),
        Err(CoreError::InvalidSave("dungeon state set is incomplete"))
    ));

    let mut game = Game::new(27);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");
    descend_one_floor(&mut game);
    descend_one_floor(&mut game);
    assert!(
        game.content
            .world(&game.world_id)
            .expect("world should remain available")
            .procedural_floors
            .iter()
            .any(|floor| floor.id == game.current_floor_id && floor.final_floor)
    );
    let mut payload = game.to_save();
    payload
        .dungeon_states
        .iter_mut()
        .find(|state| state.dungeon_id == "demo.dungeon.echo-depths")
        .expect("echo dungeon state should exist")
        .guardian_defeated = true;
    let result = Game::from_save(payload);
    assert!(
        matches!(
            result,
            Err(CoreError::InvalidSave("dungeon guardian state is invalid"))
        ),
        "unexpected guardian mismatch result: {result:?}"
    );
}

#[test]
fn guardian_mirrors_share_conquest_and_are_removed_from_other_final_floors() {
    let mut game = Game::new(71);
    game.player.position = Position { x: 3, y: 4 };
    game.traverse_stairs(false)
        .expect("echo dungeon entry should resolve")
        .expect("echo dungeon entry should transition");

    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-2.down-b");
    assert!(
        game.entities
            .iter()
            .any(|actor| actor.id == "demo.guardian.echo-depths.2")
    );
    traverse_connection(&mut game, "demo.connection.echo-depth-3-mirror.up-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-2.up-a");
    traverse_connection(&mut game, "demo.connection.echo-depth-1.down-b");
    traverse_connection(&mut game, "demo.connection.echo-depth-2-mirror.down-a");

    assert!(
        stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .entities
            .iter()
            .any(|actor| actor.id == "demo.guardian.echo-depths.2")
    );
    let guardian_index = game
        .entities
        .iter()
        .position(|actor| actor.id == "demo.guardian.echo-depths.3")
        .expect("branch final floor should contain its guardian mirror");
    let mut events = Vec::new();
    let mut changed = BTreeSet::new();
    let mut removed_entities = Vec::new();
    game.resolve_actor_death(
        guardian_index,
        DomainEvent::Waited,
        &mut events,
        &mut changed,
        &mut removed_entities,
    )
    .expect("guardian mirror death should resolve");

    assert!(game.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, DomainEvent::DungeonGuardianDefeated { .. }))
            .count(),
        1
    );
    assert_eq!(removed_entities, ["demo.guardian.echo-depths.3"]);
    assert!(
        stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.echo-depths.2")
    );
    assert!(stored_floor(&game, "demo.floor.echo-depth-3-mirror")
            .items
            .iter()
            .all(|item| {
                !matches!(&item.location, ItemLocation::CarriedBy { actor_id } if actor_id == "demo.guardian.echo-depths.2")
            }));

    let mut restored = Game::from_save(game.to_save()).expect("shared conquest should persist");
    traverse_connection(&mut restored, "demo.connection.echo-depth-3-branch.up-a");
    traverse_connection(&mut restored, "demo.connection.echo-depth-2-mirror.up-a");
    traverse_connection(&mut restored, "demo.connection.echo-depth-1.shaft-down");
    assert_eq!(restored.current_floor_id, "demo.floor.echo-depth-3-shaft");
    assert!(
        restored
            .entities
            .iter()
            .all(|actor| actor.id != "demo.guardian.echo-depths.4")
    );
}

#[test]
fn staged_task_event_reduction_advances_once_without_rng() {
    let mut game = Game::new(42);
    let active_floor_id = game.current_floor_id.clone();
    let state = game
        .task_states
        .get_mut("demo.task.echo-chain")
        .expect("staged task should exist");
    state.status = TaskStatusKindDto::Active;
    state.stage_index = 1;
    state.current = 0;
    state.required = 1;
    state.active_floor_id = Some(active_floor_id);
    let draws_before = game.rng_draw_counter();

    game.apply_task_events(&mut vec![DomainEvent::FloorTransitioned {
        from_floor_id: "demo.floor.echo-chain-rift".to_owned(),
        to_floor_id: "demo.floor.echo-chain-vault-rift".to_owned(),
    }])
    .expect("task event reduction should succeed");

    let state = &game.task_states["demo.task.echo-chain"];
    assert_eq!(state.status, TaskStatusKindDto::Active);
    assert_eq!(state.stage_index, 2);
    assert_eq!(state.current, 0);
    assert_eq!(state.required, 2);
    assert_eq!(game.rng_draw_counter(), draws_before);
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
fn campaign_guardian_death_emits_victory_and_old_save_derives_it() {
    let mut game = Game::new(49);
    for damage_type in [
        DamageType::Physical,
        DamageType::Acid,
        DamageType::Electricity,
        DamageType::Fire,
        DamageType::Cold,
        DamageType::Poison,
    ] {
        game.player
            .resistances
            .set(damage_type, ResistanceLevel::Immune);
    }
    game.player.position = Position { x: 3, y: 2 };
    game.traverse_stairs(false)
        .expect("campaign dungeon entry should resolve")
        .expect("campaign dungeon entry should transition");
    for _ in 1..10 {
        descend_one_floor(&mut game);
    }
    let guardian_index = game
        .entities
        .iter()
        .position(|entity| entity.id == "demo.guardian.resonance-descent.1")
        .expect("campaign final floor should spawn its guardian");
    assert_eq!(
        game.entities[guardian_index].kind_id,
        "demo.actor.serpent-of-chaos"
    );
    let guardian_position = game.entities[guardian_index].position;
    game.entities[guardian_index].hp = 1;
    let (direction, player_position) = TERRAIN_INTERACTION_DIRECTIONS
        .iter()
        .find_map(|direction| {
            let (dx, dy) = direction.delta();
            let position = Position {
                x: guardian_position.x - dx,
                y: guardian_position.y - dy,
            };
            game.index(position)
                .and_then(|index| game.content.terrain(&game.terrain[index]))
                .filter(|terrain| terrain.walkable)
                .map(|_| (*direction, position))
        })
        .expect("guardian should have a walkable approach");
    game.player.position = player_position;
    let update = dispatch_next(&mut game, GameCommand::Move { direction });
    assert!(update.events.iter().any(|event| {
        event.message_key == "campaign-victorious"
            && event
                .args
                .get("score")
                .is_some_and(|score| score == "60000")
    }));
    assert_eq!(update.campaign.status, CampaignStatusDto::Victorious);
    assert_eq!(game.campaign_state.victory_turn, Some(1));
    assert_eq!(game.progress.level, 51);
    assert_eq!(CharacterProgress::level_cap(true), 100);
    assert!(
        update
            .events
            .iter()
            .any(|event| event.message_key == "player-level-cap-unlocked")
    );

    let mut old_payload = game.to_save();
    old_payload.campaign_state = None;
    let restored = Game::from_save(old_payload).expect("old save should derive victory state");
    assert_eq!(
        restored.snapshot().campaign.status,
        CampaignStatusDto::Victorious
    );
    assert_eq!(restored.snapshot().state_hash, game.snapshot().state_hash);
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
            .find(|service| service.id == "demo.town-facility.outpost-home")
            .expect("test task service should be projected")
            .tasks
            .is_empty()
    );

    game.player.position = Position { x: 42, y: 13 };
    let before_draws = game.rng_draw_counter();
    let snapshot = game.snapshot();
    let service = snapshot
        .task_services
        .iter()
        .find(|service| service.id == "demo.town-facility.outpost-home")
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
            facility_id: "demo.town-facility.outpost-home".to_owned(),
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
            .find(|service| service.id == "demo.town-facility.outpost-home")
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
    game.player.position = Position { x: 42, y: 13 };
    let before_draws = game.rng_draw_counter();
    assert_eq!(
        game.snapshot()
            .task_services
            .iter()
            .find(|service| service.id == "demo.town-facility.outpost-home")
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
            facility_id: "demo.town-facility.outpost-home".to_owned(),
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
    game.player.position = Position { x: 42, y: 13 };
    dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-home".to_owned(),
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
    assert_eq!(game.current_floor_id, "demo.floor.surface");
    assert_eq!(game.task_states[task_id].status, TaskStatusKindDto::Failed);
    assert!(game.task_states[task_id].active_floor_id.is_none());
}

#[test]
fn external_task_reward_claim_is_atomic_and_persists_completion() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    game.player.position = Position { x: 42, y: 13 };
    game.task_states.insert(
        task_id.to_owned(),
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
    let update = dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-home".to_owned(),
            task_id: task_id.to_owned(),
        },
    );
    assert_eq!(
        game.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
    assert_eq!(game.rng_draw_counter(), before_draws);
    assert!(game.items.iter().any(|item| {
        item.id == "demo.task.test-warrens-depth.reward.1"
            && item.kind_id == "demo.item.echo-charm"
            && item.location == ItemLocation::Inventory
    }));
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "task.rewarded")
    );

    let saved = game.to_save();
    let restored = Game::from_save_with_content(saved, game.content.clone())
        .expect("external reward completion should round-trip");
    assert_eq!(
        restored.task_states[task_id].status,
        TaskStatusKindDto::Completed
    );
}

#[test]
fn external_task_service_rejects_unavailable_commands_without_rng_or_state_changes() {
    let mut game = task_service_game(42);
    let task_id = "demo.task.test-warrens-depth";
    let before_draws = game.rng_draw_counter();
    let accept = dispatch_next(
        &mut game,
        GameCommand::AcceptTask {
            facility_id: "demo.town-facility.outpost-home".to_owned(),
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

    game.player.position = Position { x: 42, y: 13 };
    let claim = dispatch_next(
        &mut game,
        GameCommand::ClaimTaskReward {
            facility_id: "demo.town-facility.outpost-home".to_owned(),
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
fn accepting_thieves_hideout_opens_only_its_northeastern_entry() {
    let mut game = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
        .expect("Warrens journey should create");
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
    let mut game = Game::new_warrens_journey_with_build(43, "demo.build.warrior")
        .expect("Warrens journey should create");
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

    assert_eq!(game.current_floor_id, "demo.floor.surface");
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
    let mut game = Game::new_warrens_journey_with_build(44, "demo.build.warrior")
        .expect("Warrens journey should create");
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
    let mut game = Game::new_warrens_journey_with_build(45, "demo.build.warrior")
        .expect("Warrens journey should create");
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
        .expect("Warrens world should remain available")
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
    let mut game = Game::new_warrens_journey_with_build(50, "demo.build.warrior")
        .expect("Warrens journey should create");
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
    let mut game = Game::new_warrens_journey_with_build(51, "demo.build.warrior")
        .expect("Warrens journey should create");
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
        let mut game = Game::new_warrens_journey_with_build(52, "demo.build.warrior")
            .expect("Warrens journey should create");
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
    let mut game = Game::new_warrens_journey_with_build(53, "demo.build.warrior")
        .expect("Warrens journey should create");
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
    let mut game = Game::new_warrens_journey_with_build(54, "demo.build.warrior")
        .expect("Warrens journey should create");
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
    assert!(game.stored_floors.is_empty());
}

#[test]
fn count_grants_the_fur_cloak_only_when_pest_control_is_claimed() {
    let mut game = Game::new_warrens_journey_with_build(55, "demo.build.warrior")
        .expect("Warrens journey should create");
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
fn warrens_journey_conquers_returns_retires_and_round_trips() {
    let mut game = Game::new_warrens_journey_with_build(49, "demo.build.warrior")
        .expect("Warrens journey should create");
    game.player
        .resistances
        .set(DamageType::Physical, ResistanceLevel::Immune);

    assert_eq!(game.world_id, WARRENS_JOURNEY_WORLD_ID);
    assert_eq!(game.current_floor_id, "demo.floor.surface");
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
    assert_eq!(restored.world_id, WARRENS_JOURNEY_WORLD_ID);
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
    assert_eq!(surface.floor_id, "demo.floor.surface");
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
