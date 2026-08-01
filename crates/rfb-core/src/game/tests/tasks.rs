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
fn previous_built_in_content_hash_migrates_without_spawning_new_items() {
    for previous_hash in PREVIOUS_BUILT_IN_CONTENT_HASHES {
        let mut payload = Game::new(42).to_save();
        payload.content_hash = previous_hash.to_owned();
        payload.carried_items.clear();
        payload.items.retain(|item| {
            item.kind_id != "demo.item.echo-charm"
                && item.kind_id != "demo.item.echo-blade"
                && item.kind_id != "demo.item.resonance-sling"
                && item.kind_id != "demo.item.resonance-pellet"
        });

        let restored = Game::from_save(payload).expect("known previous content should migrate");
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.content_hash, BUILT_IN_CONTENT_HASH);
        assert_eq!(snapshot.items.len(), 1);
        assert!(snapshot.items.iter().all(|item| {
            item.kind_id != "demo.item.echo-charm"
                && item.kind_id != "demo.item.echo-blade"
                && item.kind_id != "demo.item.resonance-sling"
                && item.kind_id != "demo.item.resonance-pellet"
        }));
    }
}

#[test]
fn previous_task_state_set_adds_new_tasks_as_available() {
    let mut current_payload = Game::new(42).to_save();
    current_payload
        .task_states
        .retain(|state| state.task_id != "demo.task.echo-chain");
    assert!(matches!(
        Game::from_save(current_payload),
        Err(CoreError::InvalidSave("task state set is incomplete"))
    ));

    let mut payload = Game::new(42).to_save();
    payload.content_hash =
        "b37398cb9d005302c958a9e300d07a435e8631d6a5cd44ba63b0086069577c43".to_owned();
    payload
        .task_states
        .retain(|state| state.task_id != "demo.task.echo-chain");

    let restored = Game::from_save(payload).expect("v44 task state set should migrate");
    let chain = restored
        .snapshot()
        .tasks
        .into_iter()
        .find(|task| task.task_id == "demo.task.echo-chain")
        .expect("new staged task should be added during migration");
    assert_eq!(chain.status, TaskStatusKindDto::Available);
    assert_eq!((chain.stage, chain.stages), (1, 3));
    assert_eq!((chain.current, chain.required), (0, 1));
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
fn v60_task_state_defaults_to_zero_retakes_without_rng_drift() {
    let mut payload = Game::new(27).to_save();
    payload.content_hash =
        "9789fcbbd8431ed745d8a0305cc81a54cc7e45ce79be86ed76e0227d66564a02".to_owned();
    let saved_draws = payload.rng.draw_counter;
    let restored = Game::from_save(payload).expect("v60 task state should migrate");

    assert_eq!(restored.rng.draw_counter, saved_draws);
    assert!(
        restored
            .snapshot()
            .tasks
            .iter()
            .all(|task| task.retakes_used == 0)
    );
}

#[test]
fn dungeon_guardian_state_migrates_and_rejects_entity_mismatch() {
    let mut old_payload = Game::new(42).to_save();
    old_payload.content_hash =
        "0e6cf15310644e7b3eb2f7acb0c18a8b1a7fb08739e981e7492d4079e61ab44a".to_owned();
    old_payload.dungeon_states.clear();
    let restored = Game::from_save(old_payload).expect("v45 save should add dungeon state");
    assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

    let mut v48_payload = Game::new(42).to_save();
    v48_payload.content_hash =
        "9c8fc3226c20300a308d21a5da69033efb853169214f4c411e6c740800bdf9ad".to_owned();
    v48_payload
        .dungeon_states
        .retain(|state| state.dungeon_id == "demo.dungeon.echo-depths");
    let restored =
        Game::from_save(v48_payload).expect("v48 save should add the pressure dungeon state");
    assert!(!restored.dungeon_states["demo.dungeon.echo-depths"].guardian_defeated);
    assert!(!restored.dungeon_states["demo.dungeon.resonance-descent"].guardian_defeated);

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

    game.apply_task_events(&[DomainEvent::FloorTransitioned {
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

    let victory = dispatch_next(&mut game, GameCommand::Move { direction });
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
