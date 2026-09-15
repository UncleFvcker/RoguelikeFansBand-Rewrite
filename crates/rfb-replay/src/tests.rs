// SPDX-License-Identifier: MPL-2.0

#[test]
fn replay_owns_initial_preferences_and_records_changes_between_commands() {
    let initial = Game::new(42);
    let save = initial.to_save();
    let mut recorder = ReplayRecorder::new(initial);
    let mut preferences = recorder.game().behavior_preferences();
    preferences.locale = rfb_protocol::LocaleDto::EnUs;
    preferences.travel.always_pickup = true;
    preferences.operations.easy_open = false;
    preferences.operations.default_target = rfb_protocol::DefaultTargetModeDto::NearestEnemy;
    recorder
        .dispatch(GameCommand::ConfigurePreferences { preferences })
        .unwrap();
    // A saved-rule reload records the supplied rules, never a local filesystem read.
    let mut rules = recorder.game().behavior_preferences().mogaminator;
    rules.enabled = true;
    rules.zh_cn_source = "!物品".into();
    rules.en_us_source = "!items".into();
    recorder
        .dispatch(GameCommand::ConfigureMogaminatorPreferences { preferences: rules })
        .unwrap();
    recorder.dispatch(GameCommand::Wait).unwrap();
    let expected = recorder.game().state_hash();
    let replay = recorder.replay_snapshot();
    let mut viewer_preferences = Game::default_behavior_preferences();
    viewer_preferences.mogaminator.enabled = true;
    viewer_preferences.mogaminator.zh_cn_source = "!物品".into();
    let viewer = Game::from_save(save.clone(), viewer_preferences.clone()).unwrap();
    assert_eq!(verify(&replay, viewer).unwrap().final_state_hash, expected);
    assert_eq!(
        Game::from_save(save, viewer_preferences.clone())
            .unwrap()
            .behavior_preferences(),
        viewer_preferences
    );
    let bytes = encode(&replay).unwrap();
    assert_eq!(decode(&bytes).unwrap(), replay);
}

use rfb_core::stats::{SkillProgress, experience_required_for_level_with_factor};
use rfb_protocol::{
    ActorSaveDto, Direction, GameCommand, MapScaleDto, MonsterPackBehaviorDto, Position,
};

use super::*;

#[test]
fn exploration_replays_after_saving_at_an_active_frontier() {
    let mut payload = quiet_game(424).to_save();
    payload.entities.clear();
    payload.items.clear();
    payload.gold_piles.clear();
    payload.item_property_knowledge.clear();
    payload.terrain.terrain_ids.fill("demo.terrain.wall".into());
    payload.explored.fill(false);
    let start = payload.player.position;
    for dx in 0..=20 {
        let index = (start.y * i32::from(payload.terrain.width) + start.x + dx) as usize;
        payload.terrain.terrain_ids[index] = "demo.terrain.floor".into();
        payload.explored[index] = dx < 8;
    }
    let initial = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    let mut recorder = ReplayRecorder::new(initial.clone());
    recorder.dispatch(GameCommand::ToggleSearch).unwrap();
    recorder.dispatch(GameCommand::AutoExplore).unwrap();
    let (game, replay) = recorder.finish();
    assert!(game.snapshot().player.auto_explore.is_some());
    assert!(game.snapshot().player.searching);
    verify(&decode(&encode(&replay).unwrap()).unwrap(), initial).unwrap();
    let checkpoint = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    let mut expected = ReplayRecorder::new(game);
    let mut resumed = ReplayRecorder::new(checkpoint.clone());
    for command in [
        GameCommand::ContinueAutoExplore,
        GameCommand::CancelAutoExplore,
        GameCommand::ToggleSearch,
        GameCommand::ContinueAutoExplore,
    ] {
        assert_eq!(
            expected.dispatch(command.clone()).unwrap(),
            resumed.dispatch(command).unwrap()
        );
    }
    let (game, replay) = resumed.finish();
    assert_eq!(
        verify(&decode(&encode(&replay).unwrap()).unwrap(), checkpoint)
            .unwrap()
            .final_state_hash,
        game.state_hash()
    );
}

#[test]
fn running_commands_replay_and_resume_from_a_saved_mid_run_state() {
    let mut payload = quiet_game(424).to_save();
    payload.entities.clear();
    payload.items.clear();
    payload.gold_piles.clear();
    payload.item_property_knowledge.clear();
    let start = payload.player.position;
    for dy in -1..=1 {
        for dx in 0..=5 {
            let index = ((start.y + dy) * i32::from(payload.terrain.width) + start.x + dx) as usize;
            payload.terrain.terrain_ids[index] = if dy == 0 {
                "demo.terrain.floor"
            } else {
                "demo.terrain.wall"
            }
            .into();
            payload.explored[index] = true;
        }
    }
    let initial = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    let mut recorder = ReplayRecorder::new(initial.clone());
    recorder
        .dispatch(GameCommand::Run {
            max_steps: None,
            direction: Direction::East,
        })
        .unwrap();
    let (game, replay) = recorder.finish();
    assert!(game.snapshot().player.running.is_some());
    verify(&decode(&encode(&replay).unwrap()).unwrap(), initial).unwrap();
    let checkpoint = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    let mut expected = ReplayRecorder::new(game);
    let mut resumed = ReplayRecorder::new(checkpoint.clone());
    for command in [
        GameCommand::ContinueRun,
        GameCommand::CancelRun,
        GameCommand::Alter {
            direction: Direction::North,
        },
        GameCommand::ContinueRun,
    ] {
        assert_eq!(
            expected.dispatch(command.clone()).unwrap(),
            resumed.dispatch(command).unwrap()
        );
    }
    let (resumed, replay) = resumed.finish();
    assert_eq!(
        verify(&decode(&encode(&replay).unwrap()).unwrap(), checkpoint)
            .unwrap()
            .final_state_hash,
        resumed.state_hash()
    );
}

#[test]
fn single_step_rest_save_resume_replays_identical_events_and_hashes() {
    let mut payload = Game::new_with_build(424, "demo.build.warrior")
        .unwrap()
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload.player.hp = 1;
    let initial = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    let mut recorder = ReplayRecorder::new(initial.clone());
    for _ in 0..3 {
        recorder.dispatch(GameCommand::Rest { turns: 1 }).unwrap();
    }
    let (mut uninterrupted, replay) = recorder.finish();
    assert_eq!(
        verify(&decode(&encode(&replay).unwrap()).unwrap(), initial)
            .unwrap()
            .final_state_hash,
        uninterrupted.state_hash()
    );
    let checkpoint = Game::from_save(
        uninterrupted.to_save(),
        uninterrupted.behavior_preferences(),
    )
    .unwrap();
    let mut resumed = ReplayRecorder::new(checkpoint.clone());
    let mut expected = ReplayRecorder::new(uninterrupted.clone());
    for command in [
        GameCommand::Rest { turns: 1 },
        GameCommand::Rest { turns: 1 },
        GameCommand::Wait,
    ] {
        assert_eq!(
            expected.dispatch(command.clone()).unwrap(),
            resumed.dispatch(command).unwrap()
        );
    }
    let (final_game, replay) = resumed.finish();
    uninterrupted = expected.finish().0;
    assert_eq!(final_game.state_hash(), uninterrupted.state_hash());
    assert_eq!(
        verify(&decode(&encode(&replay).unwrap()).unwrap(), checkpoint)
            .unwrap()
            .final_state_hash,
        final_game.state_hash()
    );
}

#[test]
fn tomte_item_feelings_survive_recording_save_reload_and_stack_splits() {
    let mut payload = Game::new_with_build(424, "demo.build.warrior")
        .unwrap()
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload.player.hp = 1;
    payload.player.statuses.push(
        serde_json::from_value(serde_json::json!({
            "kindId": "rfb.status.player-polymorph",
            "intensity": 1,
            "remainingTicks": 10000,
            "grantedRaceId": "rfb-legacy.race.tomte"
        }))
        .unwrap(),
    );
    payload.items.push(
        serde_json::from_value(serde_json::json!({
            "id": "test.sensed-arrows",
            "kindId": "demo.item.arrow",
            "position": payload.player.position,
            "quantity": 4,
            "quality": "fine",
            "previouslyWorn": false,
            "affixIds": ["demo.affix.frost-hunter"],
            "permanentDestructionImmunities": [],
            "capturedActor": null
        }))
        .unwrap(),
    );
    let initial = Game::from_save(payload, Game::default_behavior_preferences()).unwrap();
    let mut recorder = ReplayRecorder::new(initial.clone());
    recorder.dispatch(GameCommand::Wait).unwrap();
    assert_eq!(
        recorder
            .game()
            .snapshot()
            .items
            .iter()
            .find(|item| item.id == "test.sensed-arrows")
            .unwrap()
            .feeling,
        Some(rfb_protocol::ItemFeelingDto::Excellent)
    );
    recorder.dispatch(GameCommand::PickUp).unwrap();
    recorder
        .dispatch(GameCommand::DropQuantity {
            item_id: "test.sensed-arrows".to_owned(),
            quantity: 2,
        })
        .unwrap();
    let (midpoint, replay) = recorder.finish();
    let replay = decode(&encode(&replay).unwrap()).unwrap();
    assert_eq!(
        verify(&replay, initial).unwrap().final_state_hash,
        midpoint.state_hash()
    );

    let restored = Game::from_save(midpoint.to_save(), midpoint.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), midpoint.state_hash());
    let mut recorder = ReplayRecorder::new(restored.clone());
    recorder.dispatch(GameCommand::PickUp).unwrap();
    let (final_game, replay) = recorder.finish();
    assert_eq!(
        verify(&replay, restored).unwrap().final_state_hash,
        final_game.state_hash()
    );
    let arrows = final_game
        .snapshot()
        .inventory
        .into_iter()
        .find(|item| item.id == "test.sensed-arrows")
        .unwrap();
    assert_eq!(arrows.quantity, 4);
    assert_eq!(
        arrows.feeling,
        Some(rfb_protocol::ItemFeelingDto::Excellent)
    );
    assert!(arrows.known_properties.is_empty());
}

#[test]
fn combat_replay_records_authoritative_rng_draws() {
    let initial = Game::new(42);
    let mut recorder = ReplayRecorder::new(initial.clone());
    for command in path_to_monster_and_three_attacks() {
        recorder.dispatch(command).expect("command should execute");
    }
    let (final_game, replay) = recorder.finish();

    assert!(final_game.rng_draw_counter() > 0);
    assert_eq!(replay.checkpoints.len(), 1);
    assert_eq!(
        replay.checkpoints[0].rng_draw_counter,
        final_game.rng_draw_counter()
    );
    verify(&replay, initial).expect("combat replay should verify");
}

#[test]
fn item_replay_survives_shop_save_reload() {
    let mut payload = Game::new_with_build(42, "demo.build.warrior")
        .expect("warrior game should start")
        .to_save();
    payload.entities.clear();
    payload.carried_items.clear();
    payload.player.position = Position { x: 70, y: 39 };
    payload
        .shop_states
        .iter_mut()
        .find(|state| state.shop_id == "demo.shop.outpost-general-store")
        .expect("General Store state should exist")
        .visited = true;
    let initial = Game::from_save(payload, Game::default_behavior_preferences())
        .expect("shop precondition should restore");
    let mut recorder = ReplayRecorder::new(initial.clone());
    let shop = recorder
        .game()
        .snapshot()
        .shops
        .into_iter()
        .find(|shop| shop.id == "demo.shop.outpost-general-store")
        .expect("General Store should be projected");
    let stock_item_id = shop
        .stock
        .first()
        .expect("General Store should stock an item")
        .id
        .clone();
    recorder
        .dispatch(GameCommand::BuyFromShop {
            shop_id: shop.id,
            item_id: stock_item_id,
            quantity: 1,
        })
        .expect("purchase should execute");
    let (midpoint, replay) = recorder.finish();
    verify(&replay, initial).expect("purchase replay should verify");

    let saved = midpoint.to_save();
    let restored = Game::from_save(saved.clone(), Game::default_behavior_preferences())
        .expect("shop state should restore");
    let replay_initial = Game::from_save(saved, Game::default_behavior_preferences())
        .expect("replay state should restore");
    let ration_item_id = restored
        .snapshot()
        .inventory
        .iter()
        .find(|item| item.kind_id == "demo.item.ration-of-food")
        .expect("warrior should carry rations")
        .id
        .clone();
    let mut recorder = ReplayRecorder::new(restored);
    recorder
        .dispatch(GameCommand::SellToShop {
            shop_id: "demo.shop.outpost-general-store".to_owned(),
            item_id: ration_item_id,
            quantity: 1,
        })
        .expect("sale should execute");
    let (final_game, replay) = recorder.finish();
    let verification = verify(&replay, replay_initial).expect("sale replay should verify");

    assert_eq!(verification.commands_verified, 1);
    assert_eq!(verification.final_state_hash, final_game.state_hash());
}

#[test]
fn floor_replay_preserves_world_map_state() {
    let initial =
        Game::new_with_build(42, "demo.build.warrior").expect("warrior game should start");
    let mut recorder = ReplayRecorder::new(initial.clone());
    let update = recorder
        .dispatch(GameCommand::EnterWorldMap {
            cancel_recall: false,
        })
        .expect("world map should open");
    assert_eq!(update.map_scale, MapScaleDto::World);
    let (final_game, replay) = recorder.finish();

    let verification = verify(&replay, initial).expect("world map replay should verify");
    assert_eq!(verification.commands_verified, 1);
    assert_eq!(verification.final_state_hash, final_game.state_hash());
    assert_eq!(final_game.snapshot().map_scale, MapScaleDto::World);
}

#[test]
fn level_thirty_race_talent_choices_are_replayable() {
    for (seed, race_id) in [
        (83, "rfb-legacy.race.half-orc"),
        (85, "rfb-legacy.race.dunadan"),
        (86, "rfb-legacy.race.barbarian"),
    ] {
        let initial = level_thirty_race(seed, race_id);
        let pending = initial
            .snapshot()
            .player
            .pending_race_mutation_choice
            .expect("level 30 race should require a talent choice");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::ChooseRaceMutation {
                reward_id: pending.reward_id,
                mutation_id: "rfb.mutation.sacred-vitality".to_owned(),
            })
            .expect("race talent choice should execute");
        let (final_game, replay) = recorder.finish();

        assert!(
            final_game
                .to_save()
                .player
                .locked_mutation_ids
                .iter()
                .any(|id| id == "rfb.mutation.sacred-vitality")
        );
        let verification = verify(&replay, initial).expect("race talent replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }
}

#[test]
fn draconian_metamorphosis_choice_and_body_are_replayable() {
    let initial = level_thirty_five_draconian(87);
    let pending = initial
        .snapshot()
        .player
        .pending_race_mutation_choice
        .expect("level 35 Draconian should require a power choice");
    assert!(
        pending
            .candidates
            .iter()
            .any(|candidate| candidate.id == "rfb.mutation.draconian-metamorphosis")
    );
    let mut recorder = ReplayRecorder::new(initial.clone());
    recorder
        .dispatch(GameCommand::ChooseRaceMutation {
            reward_id: pending.reward_id,
            mutation_id: "rfb.mutation.draconian-metamorphosis".to_owned(),
        })
        .expect("Draconian metamorphosis choice should execute");
    let (final_game, replay) = recorder.finish();

    let saved = final_game.to_save();
    assert_eq!(saved.player.body_slots.len(), 10);
    assert!(
        saved
            .player
            .locked_mutation_ids
            .iter()
            .any(|id| id == "rfb.mutation.draconian-metamorphosis")
    );
    let verification =
        verify(&replay, initial).expect("Draconian metamorphosis replay should verify");
    assert_eq!(verification.commands_verified, 1);
    assert_eq!(verification.final_state_hash, final_game.state_hash());
}

#[test]
fn high_elf_invisible_detection_roll_is_replayable() {
    let initial = invisible_replay_game(84, "rfb-legacy.race.high-elf");
    assert_eq!(
        initial
            .snapshot()
            .player
            .build
            .as_ref()
            .expect("formal build identity")
            .race_id,
        "rfb-legacy.race.high-elf"
    );
    let draws_before = initial.rng_draw_counter();
    let mut recorder = ReplayRecorder::new(initial.clone());
    recorder
        .dispatch(GameCommand::Move {
            direction: Direction::North,
        })
        .expect("movement should trigger a full visibility refresh");
    let (final_game, replay) = recorder.finish();

    let human = invisible_replay_game(84, "demo.race.rfb-human");
    let human_draws_before = human.rng_draw_counter();
    let mut human_recorder = ReplayRecorder::new(human);
    human_recorder
        .dispatch(GameCommand::Move {
            direction: Direction::North,
        })
        .expect("Human control movement should execute");
    let (human_final, _) = human_recorder.finish();
    assert_eq!(
        final_game.rng_draw_counter() - draws_before,
        human_final.rng_draw_counter() - human_draws_before + 1
    );
    let verification = verify(&replay, initial).expect("High-Elf detection replay should verify");
    assert_eq!(verification.commands_verified, 1);
    assert_eq!(verification.final_state_hash, final_game.state_hash());
}

#[test]
fn replay_tampering_is_rejected() {
    let initial = quiet_game(42);
    let mut recorder = ReplayRecorder::new(initial.clone());
    dispatch_waits(&mut recorder, 3);
    let (_, replay) = recorder.finish();

    let mut altered_replay = replay.clone();
    altered_replay.commands[0].command = GameCommand::Move {
        direction: Direction::East,
    };
    assert!(matches!(
        verify(&altered_replay, initial),
        Err(ReplayError::CheckpointMismatch { .. })
    ));

    let mut bytes = encode(&replay).expect("replay should encode");
    let last = bytes.len() - 1;
    bytes[last] ^= 0x01;
    assert!(matches!(decode(&bytes), Err(ReplayError::ChecksumMismatch)));
}

#[test]
fn long_replay_records_periodic_and_final_checkpoints() {
    let initial = quiet_game(42);
    let mut recorder = ReplayRecorder::new(initial.clone());
    dispatch_waits(&mut recorder, 250);
    let (final_game, replay) = recorder.finish();

    assert_eq!(
        replay
            .checkpoints
            .iter()
            .map(|checkpoint| checkpoint.after_command_seq)
            .collect::<Vec<_>>(),
        vec![100, 200, 250]
    );
    let verification = verify(&replay, initial).expect("long replay should verify");
    assert_eq!(verification.commands_verified, 250);
    assert_eq!(verification.checkpoints_verified, 3);
    assert_eq!(verification.final_state_hash, final_game.state_hash());
}

fn dispatch_waits(recorder: &mut ReplayRecorder, count: usize) {
    for _ in 0..count {
        recorder
            .dispatch(GameCommand::Wait)
            .expect("wait should execute");
    }
}

fn quiet_game(seed: u64) -> Game {
    let mut payload = Game::new(seed).to_save();
    payload.entities.retain(|entity| {
        entity
            .pack
            .as_ref()
            .is_some_and(|pack| pack.behavior == MonsterPackBehaviorDto::GuardPosition)
    });
    payload.carried_items.clear();
    Game::from_save(payload, Game::default_behavior_preferences())
        .expect("quiet replay fixture should restore")
}

fn level_thirty_race(seed: u64, race_id: &str) -> Game {
    let game = Game::new_with_build_race_and_name(
        seed,
        "demo.build.warrior",
        race_id,
        Game::DEFAULT_PLAYER_NAME,
        Game::default_behavior_preferences(),
    )
    .expect("formal level 30 race should create");
    let factor = game.snapshot().player.build.unwrap().experience_percent;
    let mut payload = game.to_save();
    let progress = payload
        .player
        .progress
        .as_mut()
        .expect("formal build should save character progress");
    progress.level = 30;
    progress.max_level = 30;
    progress.experience = experience_required_for_level_with_factor(30, factor);
    progress.maximum_experience = progress.experience;
    progress.pending_attribute_increases = 6;
    for skill in &mut progress.skills {
        skill.current =
            SkillProgress::at_level(skill.base, skill.growth_per_ten_levels, skill.maximum, 30)
                .current;
    }
    Game::from_save(payload, Game::default_behavior_preferences())
        .expect("level 30 race replay precondition should restore")
}

fn level_thirty_five_draconian(seed: u64) -> Game {
    let game = Game::new_with_build_race_and_name(
        seed,
        "demo.build.warrior",
        "rfb-legacy.race.draconian-red",
        Game::DEFAULT_PLAYER_NAME,
        Game::default_behavior_preferences(),
    )
    .expect("formal red Draconian should create");
    let factor = game.snapshot().player.build.unwrap().experience_percent;
    let mut payload = game.to_save();
    let progress = payload
        .player
        .progress
        .as_mut()
        .expect("formal build should save character progress");
    progress.level = 35;
    progress.max_level = 35;
    progress.experience = experience_required_for_level_with_factor(35, factor);
    progress.maximum_experience = progress.experience;
    progress.pending_attribute_increases = 7;
    for skill in &mut progress.skills {
        skill.current =
            SkillProgress::at_level(skill.base, skill.growth_per_ten_levels, skill.maximum, 35)
                .current;
    }
    Game::from_save(payload, Game::default_behavior_preferences())
        .expect("level 35 Draconian replay precondition should restore")
}

fn invisible_replay_game(seed: u64, race_id: &str) -> Game {
    let mut payload = Game::new_with_build_race_and_name(
        seed,
        "demo.build.warrior",
        race_id,
        Game::DEFAULT_PLAYER_NAME,
        Game::default_behavior_preferences(),
    )
    .expect("formal replay race should create")
    .to_save();
    let start = Position { x: 3, y: 3 };
    let destination = Position { x: 3, y: 2 };
    let monster_position = Position { x: 4, y: 3 };
    payload.player.position = start;
    let width = usize::from(payload.terrain.width);
    for position in [start, destination, monster_position] {
        let index = usize::try_from(position.y).expect("positive test y") * width
            + usize::try_from(position.x).expect("positive test x");
        payload.terrain.terrain_ids[index] = "demo.terrain.floor".to_owned();
        payload.terrain.glow[index] = true;
    }

    payload.entities = vec![ActorSaveDto {
        custom_name: None,
        burglary_drops_remaining: None,
        id: "test.high-elf-invisible".to_owned(),
        kind_id: "demo.actor.clear-icky-thing".to_owned(),
        experience: 0,
        appearance_kind_id: None,
        position: monster_position,
        hp: 9,
        max_hp: 9,
        power_per_mille: 1_000,
        base_speed: 110,
        energy_need: 100,
        minor_slow: 0,
        alerted: Some(false),
        nice: true,
        visible_invisible: false,
        visible_weird_mind: false,
        eldritch_horror_triggered: false,
        anger: 0,
        friendly: false,
        no_pet: false,
        no_genocide: false,
        cloned: false,
        no_destruction: false,
        casting_cooldown_remaining: 0,
        observed_player_resistances: Vec::new(),
        statuses: Vec::new(),
        resistances: Vec::new(),
        pack: None,
        controller_id: None,
        summon: None,
    }];
    Game::from_save(payload, Game::default_behavior_preferences())
        .expect("invisible replay precondition should restore")
}

fn path_to_monster_and_three_attacks() -> Vec<GameCommand> {
    let mut commands = vec![
        GameCommand::Move {
            direction: Direction::East,
        };
        4
    ];
    commands.push(GameCommand::Move {
        direction: Direction::South,
    });
    commands.extend([
        GameCommand::Move {
            direction: Direction::SouthEast,
        },
        GameCommand::Move {
            direction: Direction::SouthEast,
        },
        GameCommand::Move {
            direction: Direction::SouthEast,
        },
    ]);
    commands
}
