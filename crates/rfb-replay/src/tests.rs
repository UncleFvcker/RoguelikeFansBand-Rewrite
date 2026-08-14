// SPDX-License-Identifier: MPL-2.0

use rfb_core::stats::experience_required_for_level;
use rfb_protocol::{
    ActorSaveDto, Direction, GameCommand, MapScaleDto, MonsterPackBehaviorDto, Position,
};

use super::*;

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
    payload.player.position = Position { x: 32, y: 13 };
    payload
        .shop_states
        .iter_mut()
        .find(|state| state.shop_id == "demo.shop.outpost-general-store")
        .expect("General Store state should exist")
        .visited = true;
    let initial = Game::from_save(payload).expect("shop precondition should restore");
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
    let restored = Game::from_save(saved.clone()).expect("shop state should restore");
    let replay_initial = Game::from_save(saved).expect("replay state should restore");
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
            leave_pets: false,
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
    Game::from_save(payload).expect("quiet replay fixture should restore")
}

fn level_thirty_race(seed: u64, race_id: &str) -> Game {
    let mut payload = Game::new_with_build_race_and_name(
        seed,
        "demo.build.warrior",
        race_id,
        Game::DEFAULT_PLAYER_NAME,
    )
    .expect("formal level 30 race should create")
    .to_save();
    let progress = payload
        .player
        .progress
        .as_mut()
        .expect("formal build should save character progress");
    progress.level = 30;
    progress.max_level = 30;
    progress.experience = experience_required_for_level(30);
    progress.maximum_experience = progress.experience;
    progress.pending_attribute_increases = 6;
    for skill in &mut progress.skills {
        skill.current = skill
            .base
            .saturating_add(skill.growth_per_ten_levels.saturating_mul(3))
            .clamp(0, skill.maximum);
    }
    Game::from_save(payload).expect("level 30 race replay precondition should restore")
}

fn invisible_replay_game(seed: u64, race_id: &str) -> Game {
    let mut payload = Game::new_with_build_race_and_name(
        seed,
        "demo.build.warrior",
        race_id,
        Game::DEFAULT_PLAYER_NAME,
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
        casting_cooldown_remaining: 0,
        observed_player_resistances: Vec::new(),
        statuses: Vec::new(),
        resistances: Vec::new(),
        pack: None,
        controller_id: None,
        summon: None,
    }];
    Game::from_save(payload).expect("invisible replay precondition should restore")
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
