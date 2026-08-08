// SPDX-License-Identifier: MPL-2.0

use rfb_core::{CoreError, Game};
use rfb_protocol::{GameCommand, GameCommandEnvelope, GameUpdate, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const REPLAY_FORMAT: &str = "rfb-replay";
pub const REPLAY_FORMAT_VERSION: u16 = 1;
pub const STATE_HASH_SCHEMA_VERSION: u16 = rfb_core::STATE_HASH_SCHEMA_VERSION;
pub const DEFAULT_CHECKPOINT_INTERVAL: usize = 100;

const MAGIC: &[u8; 8] = b"RFBREPL\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 8 + 32;
const MAX_PAYLOAD_LENGTH: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayV1 {
    pub format: String,
    pub format_version: u16,
    pub core_version: String,
    pub protocol_version: String,
    pub content_hash: String,
    pub initial_save_hash: String,
    pub rng_algorithm: String,
    pub state_hash_schema_version: u16,
    pub commands: Vec<ReplayCommand>,
    pub checkpoints: Vec<ReplayCheckpoint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayCommand {
    pub command_seq: u32,
    pub expected_revision: u32,
    pub turn_before: u32,
    pub command: GameCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplayCheckpoint {
    pub after_command_seq: u32,
    pub revision: u32,
    pub turn: u32,
    pub rng_draw_counter: u64,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayVerification {
    pub commands_verified: usize,
    pub checkpoints_verified: usize,
    pub final_state_hash: String,
}

#[derive(Debug, Clone)]
pub struct ReplayRecorder {
    game: Game,
    replay: ReplayV1,
}

impl ReplayRecorder {
    #[must_use]
    pub fn new(game: Game) -> Self {
        let replay = ReplayV1 {
            format: REPLAY_FORMAT.to_owned(),
            format_version: REPLAY_FORMAT_VERSION,
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            content_hash: game.content_hash().to_owned(),
            initial_save_hash: game.state_hash(),
            rng_algorithm: game.rng_algorithm().to_owned(),
            state_hash_schema_version: STATE_HASH_SCHEMA_VERSION,
            commands: Vec::new(),
            checkpoints: Vec::new(),
        };
        Self { game, replay }
    }

    pub fn dispatch(&mut self, command: GameCommand) -> Result<GameUpdate, ReplayError> {
        self.dispatch_envelope(GameCommandEnvelope {
            command_seq: self.game.last_command_seq().saturating_add(1),
            expected_revision: self.game.revision(),
            command,
        })
    }

    pub fn dispatch_envelope(
        &mut self,
        envelope: GameCommandEnvelope,
    ) -> Result<GameUpdate, ReplayError> {
        let recorded = ReplayCommand {
            command_seq: envelope.command_seq,
            expected_revision: envelope.expected_revision,
            turn_before: self.game.turn(),
            command: envelope.command.clone(),
        };
        let update = self.game.dispatch(envelope)?;
        self.replay.commands.push(recorded);
        if self
            .replay
            .commands
            .len()
            .is_multiple_of(DEFAULT_CHECKPOINT_INTERVAL)
        {
            self.push_checkpoint(update.command_seq);
        }
        Ok(update)
    }

    #[must_use]
    pub const fn game(&self) -> &Game {
        &self.game
    }

    #[must_use]
    pub fn replay_snapshot(&self) -> ReplayV1 {
        let mut replay = self.replay.clone();
        if let Some(command) = replay.commands.last()
            && replay
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.after_command_seq)
                != Some(command.command_seq)
        {
            replay.checkpoints.push(ReplayCheckpoint {
                after_command_seq: command.command_seq,
                revision: self.game.revision(),
                turn: self.game.turn(),
                rng_draw_counter: self.game.rng_draw_counter(),
                state_hash: self.game.state_hash(),
            });
        }
        replay
    }

    #[must_use]
    pub fn finish(self) -> (Game, ReplayV1) {
        let replay = self.replay_snapshot();
        (self.game, replay)
    }

    fn push_checkpoint(&mut self, after_command_seq: u32) {
        self.replay.checkpoints.push(ReplayCheckpoint {
            after_command_seq,
            revision: self.game.revision(),
            turn: self.game.turn(),
            rng_draw_counter: self.game.rng_draw_counter(),
            state_hash: self.game.state_hash(),
        });
    }
}

pub fn verify(replay: &ReplayV1, mut game: Game) -> Result<ReplayVerification, ReplayError> {
    validate_metadata(replay, &game)?;
    validate_checkpoint_schedule(replay)?;
    let mut checkpoint_index = 0;

    for (index, recorded) in replay.commands.iter().enumerate() {
        if recorded.command_seq != game.last_command_seq().saturating_add(1)
            || recorded.expected_revision != game.revision()
            || recorded.turn_before != game.turn()
        {
            return Err(ReplayError::CommandContextMismatch {
                index: index + 1,
                expected_seq: game.last_command_seq().saturating_add(1),
                received_seq: recorded.command_seq,
                expected_revision: game.revision(),
                received_revision: recorded.expected_revision,
                expected_turn: game.turn(),
                received_turn: recorded.turn_before,
            });
        }

        game.dispatch(GameCommandEnvelope {
            command_seq: recorded.command_seq,
            expected_revision: recorded.expected_revision,
            command: recorded.command.clone(),
        })?;

        if replay
            .checkpoints
            .get(checkpoint_index)
            .is_some_and(|checkpoint| checkpoint.after_command_seq == recorded.command_seq)
        {
            verify_checkpoint(&replay.checkpoints[checkpoint_index], &game)?;
            checkpoint_index += 1;
        }
    }

    Ok(ReplayVerification {
        commands_verified: replay.commands.len(),
        checkpoints_verified: checkpoint_index,
        final_state_hash: game.state_hash(),
    })
}

pub fn encode(replay: &ReplayV1) -> Result<Vec<u8>, ReplayError> {
    let payload = rmp_serde::to_vec_named(replay)?;
    if payload.len() > MAX_PAYLOAD_LENGTH {
        return Err(ReplayError::PayloadTooLarge(payload.len()));
    }
    let payload_length = u64::try_from(payload.len()).map_err(|_| ReplayError::LengthOverflow)?;
    let checksum = Sha256::digest(&payload);
    let capacity = FIXED_HEADER_LENGTH
        .checked_add(payload.len())
        .ok_or(ReplayError::LengthOverflow)?;
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&payload_length.to_le_bytes());
    output.extend_from_slice(&checksum);
    output.extend_from_slice(&payload);
    Ok(output)
}

pub fn decode(bytes: &[u8]) -> Result<ReplayV1, ReplayError> {
    if bytes.len() < FIXED_HEADER_LENGTH || &bytes[..8] != MAGIC {
        return Err(ReplayError::InvalidContainer);
    }
    let version = u16::from_le_bytes(
        bytes[8..10]
            .try_into()
            .map_err(|_| ReplayError::InvalidContainer)?,
    );
    if version != CONTAINER_VERSION {
        return Err(ReplayError::UnsupportedContainerVersion(version));
    }
    let flags = u16::from_le_bytes(
        bytes[10..12]
            .try_into()
            .map_err(|_| ReplayError::InvalidContainer)?,
    );
    if flags != 0 {
        return Err(ReplayError::UnsupportedFlags(flags));
    }
    let payload_length = usize::try_from(u64::from_le_bytes(
        bytes[12..20]
            .try_into()
            .map_err(|_| ReplayError::InvalidContainer)?,
    ))
    .map_err(|_| ReplayError::LengthOverflow)?;
    if payload_length > MAX_PAYLOAD_LENGTH {
        return Err(ReplayError::PayloadTooLarge(payload_length));
    }
    let expected_length = FIXED_HEADER_LENGTH
        .checked_add(payload_length)
        .ok_or(ReplayError::LengthOverflow)?;
    if bytes.len() != expected_length {
        return Err(ReplayError::InvalidContainer);
    }
    let payload = &bytes[FIXED_HEADER_LENGTH..];
    let checksum = Sha256::digest(payload);
    if bytes[20..52] != checksum[..] {
        return Err(ReplayError::ChecksumMismatch);
    }
    Ok(rmp_serde::from_slice(payload)?)
}

pub fn to_debug_json(replay: &ReplayV1) -> Result<String, ReplayError> {
    Ok(serde_json::to_string_pretty(replay)?)
}

pub fn from_debug_json(json: &str) -> Result<ReplayV1, ReplayError> {
    Ok(serde_json::from_str(json)?)
}

fn validate_metadata(replay: &ReplayV1, game: &Game) -> Result<(), ReplayError> {
    if replay.format != REPLAY_FORMAT {
        return Err(ReplayError::InvalidFormat(replay.format.clone()));
    }
    if replay.format_version != REPLAY_FORMAT_VERSION {
        return Err(ReplayError::UnsupportedFormatVersion(replay.format_version));
    }
    if replay.core_version != env!("CARGO_PKG_VERSION") {
        return Err(ReplayError::IncompatibleCoreVersion(
            replay.core_version.clone(),
        ));
    }
    if replay.protocol_version != PROTOCOL_VERSION {
        return Err(ReplayError::IncompatibleProtocolVersion(
            replay.protocol_version.clone(),
        ));
    }
    if replay.content_hash != game.content_hash() {
        return Err(ReplayError::ContentMismatch(replay.content_hash.clone()));
    }
    if replay.rng_algorithm != game.rng_algorithm() {
        return Err(ReplayError::RngMismatch(replay.rng_algorithm.clone()));
    }
    if replay.state_hash_schema_version != STATE_HASH_SCHEMA_VERSION {
        return Err(ReplayError::StateHashSchema(
            replay.state_hash_schema_version,
        ));
    }
    let actual_initial_hash = game.state_hash();
    if replay.initial_save_hash != actual_initial_hash {
        return Err(ReplayError::InitialStateMismatch {
            expected: replay.initial_save_hash.clone(),
            actual: actual_initial_hash,
        });
    }
    Ok(())
}

fn validate_checkpoint_schedule(replay: &ReplayV1) -> Result<(), ReplayError> {
    let mut expected = replay
        .commands
        .iter()
        .enumerate()
        .filter(|(index, _)| (index + 1).is_multiple_of(DEFAULT_CHECKPOINT_INTERVAL))
        .map(|(_, command)| command.command_seq)
        .collect::<Vec<_>>();
    if let Some(last) = replay.commands.last()
        && expected.last().copied() != Some(last.command_seq)
    {
        expected.push(last.command_seq);
    }
    let actual = replay
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.after_command_seq)
        .collect::<Vec<_>>();
    if actual != expected {
        return Err(ReplayError::CheckpointSchedule { expected, actual });
    }
    Ok(())
}

fn verify_checkpoint(checkpoint: &ReplayCheckpoint, game: &Game) -> Result<(), ReplayError> {
    let actual = ReplayCheckpoint {
        after_command_seq: game.last_command_seq(),
        revision: game.revision(),
        turn: game.turn(),
        rng_draw_counter: game.rng_draw_counter(),
        state_hash: game.state_hash(),
    };
    if &actual != checkpoint {
        return Err(ReplayError::CheckpointMismatch {
            command_seq: checkpoint.after_command_seq,
            expected: Box::new(checkpoint.clone()),
            actual: Box::new(actual),
        });
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum ReplayError {
    #[error("replay format is invalid: {0}")]
    InvalidFormat(String),
    #[error("unsupported replay format version {0}")]
    UnsupportedFormatVersion(u16),
    #[error("replay requires incompatible core version {0}")]
    IncompatibleCoreVersion(String),
    #[error("replay requires incompatible protocol version {0}")]
    IncompatibleProtocolVersion(String),
    #[error("replay content hash does not match: {0}")]
    ContentMismatch(String),
    #[error("replay RNG algorithm does not match: {0}")]
    RngMismatch(String),
    #[error("unsupported state hash schema version {0}")]
    StateHashSchema(u16),
    #[error("replay initial state hash mismatch: expected {expected}, actual {actual}")]
    InitialStateMismatch { expected: String, actual: String },
    #[error(
        "replay command {index} context mismatch: seq {received_seq}/{expected_seq}, revision {received_revision}/{expected_revision}, turn {received_turn}/{expected_turn}"
    )]
    CommandContextMismatch {
        index: usize,
        expected_seq: u32,
        received_seq: u32,
        expected_revision: u32,
        received_revision: u32,
        expected_turn: u32,
        received_turn: u32,
    },
    #[error("replay checkpoint schedule mismatch: expected {expected:?}, actual {actual:?}")]
    CheckpointSchedule {
        expected: Vec<u32>,
        actual: Vec<u32>,
    },
    #[error("replay checkpoint after command {command_seq} does not match")]
    CheckpointMismatch {
        command_seq: u32,
        expected: Box<ReplayCheckpoint>,
        actual: Box<ReplayCheckpoint>,
    },
    #[error("replay container is invalid or truncated")]
    InvalidContainer,
    #[error("unsupported replay container version {0}")]
    UnsupportedContainerVersion(u16),
    #[error("unsupported replay container flags 0x{0:04x}")]
    UnsupportedFlags(u16),
    #[error("replay payload checksum does not match")]
    ChecksumMismatch,
    #[error("replay payload is too large: {0} bytes")]
    PayloadTooLarge(usize),
    #[error("replay length overflow")]
    LengthOverflow,
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    MessagePackEncode(#[from] rmp_serde::encode::Error),
    #[error(transparent)]
    MessagePackDecode(#[from] rmp_serde::decode::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use rfb_protocol::{DeviceRechargeSourceDto, Direction, SummonCommandModeDto};

    use super::*;

    #[test]
    fn records_every_hundred_commands_and_the_final_state() {
        let initial = quiet_game(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        dispatch_waits(&mut recorder, 250);
        let exported_while_running = recorder.replay_snapshot();
        assert_eq!(
            exported_while_running
                .checkpoints
                .last()
                .map(|checkpoint| checkpoint.after_command_seq),
            Some(250)
        );
        let (final_game, replay) = recorder.finish();

        assert_eq!(
            replay
                .checkpoints
                .iter()
                .map(|checkpoint| checkpoint.after_command_seq)
                .collect::<Vec<_>>(),
            vec![100, 200, 250]
        );
        let verification = verify(&replay, initial).expect("recorded replay should verify");
        assert_eq!(verification.commands_verified, 250);
        assert_eq!(verification.checkpoints_verified, 3);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn ten_thousand_turns_do_not_drift() {
        let mut initial = quiet_game(0x0123_4567_89ab_cdef);
        for index in 0..9 {
            initial
                .debug_add_generated_inventory_item(
                    &format!("test.item.replay-ration.{index}"),
                    "demo.item.ration-of-food",
                    1,
                )
                .expect("long replay should have enough food");
        }
        let mut recorder = ReplayRecorder::new(initial.clone());
        for index in 0..9 {
            dispatch_waits(&mut recorder, 1_000);
            recorder
                .dispatch(GameCommand::UseItem {
                    item_id: format!("test.item.replay-ration.{index}"),
                    target: None,
                })
                .expect("ration should keep the long replay alive");
        }
        dispatch_waits(&mut recorder, 991);
        let (final_game, replay) = recorder.finish();

        assert_eq!(replay.checkpoints.len(), 100);
        let verification = verify(&replay, initial).expect("long replay should verify");
        assert_eq!(verification.commands_verified, 10_000);
        assert_eq!(verification.checkpoints_verified, 100);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn save_reload_continuation_matches_uninterrupted_execution() {
        let mut uninterrupted = ReplayRecorder::new(quiet_game(7));
        dispatch_waits(&mut uninterrupted, 200);
        let (uninterrupted_game, _) = uninterrupted.finish();

        let initial = quiet_game(7);
        let mut first_segment = ReplayRecorder::new(initial.clone());
        dispatch_waits(&mut first_segment, 100);
        let (midpoint_game, first_replay) = first_segment.finish();
        verify(&first_replay, initial).expect("first replay segment should verify");

        let midpoint_payload = midpoint_game.to_save();
        let restored = Game::from_save(midpoint_payload.clone()).expect("midpoint should restore");
        let replay_initial = Game::from_save(midpoint_payload).expect("midpoint should restore");
        let mut second_segment = ReplayRecorder::new(restored);
        dispatch_waits(&mut second_segment, 100);
        let (resumed_game, second_replay) = second_segment.finish();
        verify(&second_replay, replay_initial).expect("resumed replay segment should verify");

        assert_eq!(resumed_game.state_hash(), uninterrupted_game.state_hash());
    }

    #[test]
    fn shop_transactions_replay_and_continue_across_save_reload() {
        let mut initial_payload = Game::new_warrens_journey_with_build(42, "demo.build.warrior")
            .expect("Warrens game should start")
            .to_save();
        initial_payload.entities.clear();
        initial_payload.carried_items.clear();
        initial_payload.player.position = rfb_protocol::Position { x: 32, y: 13 };
        initial_payload
            .shop_states
            .iter_mut()
            .find(|state| state.shop_id == "demo.shop.outpost-general-store")
            .expect("General Store state should exist")
            .visited = true;
        let initial = Game::from_save(initial_payload).expect("shop precondition should restore");
        let mut first_segment = ReplayRecorder::new(initial.clone());
        let snapshot = first_segment.game().snapshot();
        let shop = snapshot
            .shops
            .iter()
            .find(|shop| shop.id == "demo.shop.outpost-general-store")
            .expect("General Store should be projected");
        let stock_item_id = shop
            .stock
            .first()
            .expect("General Store should stock an item")
            .id
            .clone();
        first_segment
            .dispatch(GameCommand::BuyFromShop {
                shop_id: shop.id.clone(),
                item_id: stock_item_id,
                quantity: 1,
            })
            .expect("purchase should execute");
        let (midpoint_game, first_replay) = first_segment.finish();
        let first_verification =
            verify(&first_replay, initial).expect("shop purchase replay should verify");
        assert_eq!(first_verification.commands_verified, 1);

        let midpoint_payload = midpoint_game.to_save();
        let restored =
            Game::from_save(midpoint_payload.clone()).expect("shop state should restore");
        let replay_initial =
            Game::from_save(midpoint_payload).expect("shop replay state should restore");
        let ration_item_id = restored
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.ration-of-food")
            .expect("Warrior should carry rations")
            .id
            .clone();
        let mut second_segment = ReplayRecorder::new(restored);
        second_segment
            .dispatch(GameCommand::SellToShop {
                shop_id: "demo.shop.outpost-general-store".to_owned(),
                item_id: ration_item_id,
                quantity: 1,
            })
            .expect("sale should execute");
        let (resumed_game, second_replay) = second_segment.finish();
        let second_verification =
            verify(&second_replay, replay_initial).expect("resumed shop sale replay should verify");
        assert_eq!(second_verification.commands_verified, 1);

        let mut uninterrupted = midpoint_game;
        uninterrupted
            .dispatch(GameCommandEnvelope {
                command_seq: uninterrupted.last_command_seq().saturating_add(1),
                expected_revision: uninterrupted.revision(),
                command: second_replay.commands[0].command.clone(),
            })
            .expect("uninterrupted sale should execute");
        assert_eq!(resumed_game.state_hash(), uninterrupted.state_hash());
    }

    #[test]
    fn checkpoint_records_authoritative_rng_draws() {
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
    fn pickup_inventory_state_round_trips_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::Move {
                direction: Direction::East,
            })
            .expect("move should execute");
        recorder
            .dispatch(GameCommand::PickUp)
            .expect("pickup should execute");
        let (final_game, replay) = recorder.finish();

        assert_eq!(final_game.snapshot().items.len(), 4);
        assert_eq!(final_game.snapshot().inventory.len(), 1);
        let verification = verify(&replay, initial).expect("pickup replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn thrown_item_transaction_round_trips_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::PickUp,
            GameCommand::Throw {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                direction: Direction::East,
            },
        ] {
            recorder.dispatch(command).expect("command should execute");
        }
        let (final_game, replay) = recorder.finish();

        assert_eq!(final_game.snapshot().inventory[0].quantity, 4);
        assert!(
            final_game
                .snapshot()
                .items
                .iter()
                .any(|item| item.id == "generated.item.2" && item.quantity == 1)
        );
        let verification = verify(&replay, initial).expect("throw replay should verify");
        assert_eq!(verification.commands_verified, 3);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn consumable_use_round_trips_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::PickUp,
            GameCommand::UseItem {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                target: None,
            },
        ] {
            recorder.dispatch(command).expect("command should execute");
        }
        let (final_game, replay) = recorder.finish();

        assert_eq!(final_game.snapshot().inventory[0].quantity, 4);
        let verification = verify(&replay, initial).expect("item use replay should verify");
        assert_eq!(verification.commands_verified, 3);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn dynamic_device_target_round_trips_through_replay() {
        let mut initial =
            Game::new_with_build(0, "demo.build.tinkerer").expect("build should create");
        initial
            .debug_add_generated_inventory_item(
                "test.item.replay-wand.1",
                "demo.item.resonance-wand",
                1,
            )
            .expect("dynamic wand should generate");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::UseItem {
                item_id: "test.item.replay-wand.1".to_owned(),
                target: Some(rfb_protocol::TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                }),
            })
            .expect("device command should execute");
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("device replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn device_recharge_round_trips_through_replay() {
        let mut initial =
            Game::new_with_build(60, "demo.build.tinkerer").expect("build should create");
        initial
            .debug_add_generated_inventory_item(
                "test.item.replay-recharge.1",
                "demo.item.resonance-staff",
                1,
            )
            .expect("dynamic staff should generate");
        let mut payload = initial.to_save();
        payload
            .inventory
            .iter_mut()
            .find(|item| item.id == "test.item.replay-recharge.1")
            .and_then(|item| item.charges.as_mut())
            .expect("saved staff should carry energy")
            .current = 0;
        let mut initial = Game::from_save(payload).expect("depleted staff should reload");
        initial.debug_set_recharge_attempts_succeed(true);
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::RechargeItem {
                target_item_id: "test.item.replay-recharge.1".to_owned(),
                source: DeviceRechargeSourceDto::Resource,
            })
            .expect("recharge command should execute");
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("recharge replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn target_selection_round_trips_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::FireTarget {
                target: rfb_protocol::TargetSelection::Position {
                    position: rfb_protocol::Position { x: 4, y: 3 },
                },
            })
            .expect("targeted fire command should execute");
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("target selection replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn ability_study_and_cast_round_trip_through_replay() {
        let initial = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.resonant-bolt".to_owned(),
            })
            .expect("ability study should execute");
        recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.resonant-bolt".to_owned(),
                target: rfb_protocol::TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            })
            .expect("ability cast should execute");
        let (final_game, replay) = recorder.finish();

        let snapshot = final_game.snapshot();
        assert_eq!(snapshot.player.resources[0].current, 16);
        assert!(
            snapshot
                .player
                .abilities
                .iter()
                .find(|ability| ability.id == "demo.ability.resonant-bolt")
                .is_some_and(|ability| ability.learned)
        );
        assert!(
            !snapshot
                .entities
                .iter()
                .any(|entity| entity.id == "demo.monster.ember-mote.1")
        );
        let verification = verify(&replay, initial).expect("ability replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn area_ability_round_trips_through_replay() {
        let initial = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-burst".to_owned(),
            })
            .expect("area ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-burst".to_owned(),
                target: rfb_protocol::TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            })
            .expect("area ability cast should execute");
        assert!(update.events.iter().any(|event| {
            event.kind == "ability.area-damage"
                && event.args.get("radius").is_some_and(|radius| radius == "2")
                && event
                    .args
                    .get("targets")
                    .is_some_and(|targets| targets == "1")
        }));
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("area ability replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn beam_ability_round_trips_through_replay() {
        let initial = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-lance".to_owned(),
            })
            .expect("beam ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-lance".to_owned(),
                target: rfb_protocol::TargetSelection::Entity {
                    entity_id: "demo.monster.ember-mote.1".to_owned(),
                },
            })
            .expect("beam ability cast should execute");
        assert!(update.events.iter().any(|event| {
            event.kind == "ability.beam-damage"
                && event
                    .args
                    .get("targets")
                    .is_some_and(|targets| targets.parse::<u16>().is_ok())
        }));
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("beam ability replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn cone_ability_round_trips_through_replay() {
        let initial = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-fan".to_owned(),
            })
            .expect("cone ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-fan".to_owned(),
                target: rfb_protocol::TargetSelection::Direction {
                    direction: Direction::East,
                },
            })
            .expect("cone ability cast should execute");
        assert!(update.events.iter().any(|event| {
            event.kind == "ability.cone-damage"
                && event.args.get("radius").is_some_and(|radius| radius == "2")
                && event
                    .args
                    .get("targets")
                    .is_some_and(|targets| targets.parse::<u16>().is_ok())
        }));
        let (final_game, replay) = recorder.finish();

        let verification = verify(&replay, initial).expect("cone ability replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn teleport_ability_round_trips_through_replay() {
        let mut payload = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create")
            .to_save();
        payload.entities.clear();
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        let initial = Game::from_save(payload).expect("teleport replay fixture should load");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-step".to_owned(),
            })
            .expect("teleport ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-step".to_owned(),
                target: rfb_protocol::TargetSelection::Position {
                    position: rfb_protocol::Position { x: 6, y: 3 },
                },
            })
            .expect("teleport ability cast should execute");
        assert!(update.events.iter().any(|event| {
            event.kind == "ability.teleport"
                && matches!(
                    event.outcome.as_ref(),
                    Some(rfb_protocol::GameEventOutcomeDto::AbilityTeleport { resolution })
                        if resolution.from == rfb_protocol::Position { x: 3, y: 3 }
                            && resolution.to == rfb_protocol::Position { x: 6, y: 3 }
                )
        }));
        let (final_game, replay) = recorder.finish();

        assert_eq!(
            final_game.snapshot().player.position,
            rfb_protocol::Position { x: 6, y: 3 }
        );
        let verification = verify(&replay, initial).expect("teleport replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn detection_ability_round_trips_through_replay() {
        let mut payload = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create")
            .to_save();
        payload.entities.clear();
        payload.items.clear();
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        payload
            .player
            .ability_progress
            .iter_mut()
            .find(|progress| progress.id == "demo.ability.echo-sight")
            .expect("detect ability progress should exist")
            .proficiency = 1600;
        let rune = rfb_protocol::Position { x: 4, y: 3 };
        let index = usize::try_from(rune.y).expect("rune y should fit")
            * usize::from(payload.terrain.width)
            + usize::try_from(rune.x).expect("rune x should fit");
        payload.terrain.terrain_ids[index] = "demo.terrain.echo-rune-hidden".to_owned();
        let initial = Game::from_save(payload).expect("detection replay fixture should load");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-sight".to_owned(),
            })
            .expect("detection ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-sight".to_owned(),
                target: rfb_protocol::TargetSelection::SelfTarget,
            })
            .expect("detection ability cast should execute");
        assert!(update.events.iter().any(|event| {
            matches!(
                event.outcome.as_ref(),
                Some(rfb_protocol::GameEventOutcomeDto::AbilityDetect { resolution })
                    if resolution.persistent && resolution.detected_positions == [rune]
            )
        }));
        let (final_game, replay) = recorder.finish();

        assert!(final_game.to_save().revealed_terrain.contains(&rune));
        let verification = verify(&replay, initial).expect("detection replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn terrain_transform_ability_round_trips_through_replay() {
        let mut payload = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create")
            .to_save();
        payload.entities.clear();
        payload.items.clear();
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        payload
            .player
            .ability_progress
            .iter_mut()
            .find(|progress| progress.id == "demo.ability.echo-delving")
            .expect("terrain transform ability progress should exist")
            .proficiency = 1600;
        let wall = rfb_protocol::Position { x: 5, y: 3 };
        let index = usize::try_from(wall.y).expect("wall y should fit")
            * usize::from(payload.terrain.width)
            + usize::try_from(wall.x).expect("wall x should fit");
        payload.terrain.terrain_ids[index] = "demo.terrain.wall".to_owned();
        let initial =
            Game::from_save(payload).expect("terrain transform replay fixture should load");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-delving".to_owned(),
            })
            .expect("terrain transform ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-delving".to_owned(),
                target: rfb_protocol::TargetSelection::Position { position: wall },
            })
            .expect("terrain transform ability cast should execute");
        assert!(update.events.iter().any(|event| {
            matches!(
                event.outcome.as_ref(),
                Some(rfb_protocol::GameEventOutcomeDto::AbilityTerrainTransform { resolution })
                    if resolution.target_terrain_id == "demo.terrain.floor"
                        && resolution.transformed_positions.contains(&wall)
            )
        }));
        let (final_game, replay) = recorder.finish();

        assert_eq!(
            final_game
                .snapshot()
                .cells
                .iter()
                .find(|cell| cell.position == wall)
                .map(|cell| cell.terrain_id.as_str()),
            Some("demo.terrain.floor")
        );
        let verification =
            verify(&replay, initial).expect("terrain transform replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn ordered_status_effects_round_trip_through_replay() {
        let mut payload = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create")
            .to_save();
        payload.entities.clear();
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        payload
            .player
            .ability_progress
            .iter_mut()
            .find(|progress| progress.id == "demo.ability.echo-quickening")
            .expect("status ability progress should exist")
            .proficiency = 1600;
        payload.player.statuses.push(rfb_protocol::StatusSaveDto {
            kind_id: "rfb.status.slow".to_owned(),
            intensity: 1,
            remaining_ticks: 20,
            source_id: Some("test.slow".to_owned()),
            granted_resistances: Vec::new(),
            granted_brands: Vec::new(),
            granted_modifiers: rfb_protocol::StatModifiersDto::default(),
            granted_equipment_bonuses: rfb_protocol::EquipmentBonusesDto::default(),
            granted_status_immunities: Vec::new(),
            granted_race_id: None,
            grants_wall_passage: false,
            incoming_damage_percent: 100,
        });
        let initial = Game::from_save(payload).expect("status replay fixture should load");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.echo-primer")
            .map(|item| item.id.clone())
            .expect("scholar should carry the echo primer");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.echo-quickening".to_owned(),
            })
            .expect("status ability study should execute");
        let update = recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.echo-quickening".to_owned(),
                target: rfb_protocol::TargetSelection::SelfTarget,
            })
            .expect("status ability cast should execute");
        assert!(update.events.iter().any(|event| {
            matches!(
                event.outcome.as_ref(),
                Some(rfb_protocol::GameEventOutcomeDto::AbilityEffects { resolution })
                    if resolution.effects.len() == 2
            )
        }));
        let (final_game, replay) = recorder.finish();

        assert!(
            final_game
                .snapshot()
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == "rfb.status.haste")
        );
        assert!(
            final_game
                .snapshot()
                .player
                .statuses
                .iter()
                .all(|status| status.kind_id != "rfb.status.slow")
        );
        let verification = verify(&replay, initial).expect("status ability replay should verify");
        assert_eq!(verification.commands_verified, 2);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn monster_ability_selection_and_status_round_trip_through_replay() {
        let seed = (0..1_000_u64)
            .find(|seed| {
                let mut recorder = ReplayRecorder::new(monster_caster_game(*seed));
                let update = recorder
                    .dispatch(GameCommand::Wait)
                    .expect("caster replay probe should execute");
                update.events.iter().any(|event| {
                    matches!(
                        event.outcome.as_ref(),
                        Some(rfb_protocol::GameEventOutcomeDto::MonsterAbilityCast {
                            resolution
                        }) if resolution.ability_id == "demo.ability.echo-binding"
                    )
                })
            })
            .expect("a deterministic seed should select echo binding");
        let initial = monster_caster_game(seed);
        let mut recorder = ReplayRecorder::new(initial.clone());
        let update = recorder
            .dispatch(GameCommand::Wait)
            .expect("caster replay command should execute");

        assert!(update.events.iter().any(|event| {
            matches!(
                event.outcome.as_ref(),
                Some(rfb_protocol::GameEventOutcomeDto::MonsterAbilityDecision {
                    resolution
                }) if resolution.selected_ability_id.as_deref()
                    == Some("demo.ability.echo-binding")
            )
        }));
        assert!(update.events.iter().any(|event| {
            matches!(
                event.outcome.as_ref(),
                Some(rfb_protocol::GameEventOutcomeDto::MonsterAbilityCast {
                    resolution
                }) if resolution.ability_id == "demo.ability.echo-binding"
                    && resolution.effects.len() == 2
            )
        }));
        assert_eq!(
            recorder.game().snapshot().entities[0].casting_cooldown_remaining,
            2
        );
        assert!(
            recorder.game().snapshot().entities[0]
                .observed_player_resistances
                .iter()
                .any(|resistance| {
                    resistance.damage_type == rfb_protocol::DamageTypeDto::Cold
                        && resistance.level == rfb_protocol::ResistanceLevelDto::Normal
                })
        );
        let (final_game, replay) = recorder.finish();

        assert!(
            final_game
                .snapshot()
                .player
                .statuses
                .iter()
                .any(|status| status.kind_id == "rfb.status.slow")
        );
        let verification = verify(&replay, initial).expect("monster ability replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn monster_hostile_summon_round_trips_through_replay() {
        let seed = (0..10_000_u64)
            .find(|seed| {
                let mut recorder = ReplayRecorder::new(monster_caster_game(*seed));
                let update = recorder
                    .dispatch(GameCommand::Wait)
                    .expect("hostile summon replay probe should execute");
                update.events.iter().any(|event| {
                    matches!(
                        event.outcome.as_ref(),
                        Some(rfb_protocol::GameEventOutcomeDto::MonsterAbilityCast {
                            resolution
                        }) if resolution.ability_id == "demo.ability.call-discord"
                    )
                })
            })
            .expect("a deterministic seed should select the hostile summon");
        let initial = monster_caster_game(seed);
        let mut recorder = ReplayRecorder::new(initial.clone());
        let update = recorder
            .dispatch(GameCommand::Wait)
            .expect("hostile summon replay command should execute");
        let summon = update
            .events
            .iter()
            .find_map(|event| match event.outcome.as_ref() {
                Some(rfb_protocol::GameEventOutcomeDto::MonsterAbilityCast { resolution })
                    if resolution.ability_id == "demo.ability.call-discord" =>
                {
                    resolution.summon.as_ref()
                }
                _ => None,
            })
            .expect("monster summon outcome should expose generated entities");
        assert_eq!(summon.entity_ids.len(), 2);
        assert!(recorder.game().snapshot().entities.iter().any(|entity| {
            summon.entity_ids.contains(&entity.id)
                && entity.faction == rfb_protocol::EntityFactionDto::Hostile
        }));
        let (final_game, replay) = recorder.finish();

        let verification =
            verify(&replay, initial).expect("hostile summon replay should verify exactly");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn healing_and_multi_turn_rest_round_trip_through_replay() {
        let mut payload = Game::new_with_build(0, "demo.build.scholar")
            .expect("scholar replay fixture should create")
            .to_save();
        payload.entities.clear();
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        payload.player.hp = 5;
        payload.player.resources[0].current = 10;
        let initial = Game::from_save(payload).expect("healing replay fixture should load");
        let book_item_id = initial
            .snapshot()
            .inventory
            .iter()
            .find(|item| item.kind_id == "demo.item.stillwater-notes")
            .map(|item| item.id.clone())
            .expect("scholar should carry the stillwater notes");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::StudyAbility {
                book_item_id,
                ability_id: "demo.ability.mending-echo".to_owned(),
            })
            .expect("healing ability study should execute");
        recorder
            .dispatch(GameCommand::CastAbility {
                ability_id: "demo.ability.mending-echo".to_owned(),
                target: rfb_protocol::TargetSelection::SelfTarget,
            })
            .expect("healing ability cast should execute");
        recorder
            .dispatch(GameCommand::Rest { turns: 100 })
            .expect("multi-turn rest should execute");
        let (final_game, replay) = recorder.finish();

        let snapshot = final_game.snapshot();
        assert_eq!(snapshot.player.hp, 12);
        assert_eq!(snapshot.player.resources[0].current, 21);
        assert_eq!(snapshot.turn, 11);
        let verification = verify(&replay, initial).expect("healing rest replay should verify");
        assert_eq!(verification.commands_verified, 3);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn summon_command_round_trips_through_replay() {
        let initial = quiet_game(42);
        let initial_world_tick = initial.snapshot().world_tick;
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::SetSummonCommand {
                mode: SummonCommandModeDto::Guard,
            })
            .expect("summon command should execute");
        let (final_game, replay) = recorder.finish();

        let snapshot = final_game.snapshot();
        assert_eq!(snapshot.world_tick, initial_world_tick);
        let command = snapshot.player.summon_command;
        assert_eq!(command.mode, SummonCommandModeDto::Guard);
        assert_eq!(command.guard_position, Some(snapshot.player.position));

        let verification =
            verify(&replay, initial).expect("summon command replay should verify exactly");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn floor_transition_round_trips_through_replay() {
        let mut payload = Game::new(27).to_save();
        payload.player.position = rfb_protocol::Position { x: 3, y: 4 };
        let initial = Game::from_save(payload).expect("stairs fixture should load");
        let mut recorder = ReplayRecorder::new(initial.clone());
        recorder
            .dispatch(GameCommand::TraverseStairs)
            .expect("stairs command should execute");
        let (final_game, replay) = recorder.finish();

        assert_eq!(final_game.snapshot().floor_id, "demo.floor.echo-depth-1");
        let verification = verify(&replay, initial).expect("floor replay should verify");
        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn door_interaction_round_trips_through_replay() {
        let mut payload = Game::new(27).to_save();
        payload.player.position = rfb_protocol::Position { x: 3, y: 4 };
        let initial = Game::from_save(payload).expect("door fixture should load");
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::TraverseStairs,
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
        ] {
            recorder
                .dispatch(command)
                .expect("door replay command should execute");
        }
        let door_position = rfb_protocol::Position { x: 10, y: 4 };
        let discovered = (0..12).any(|_| {
            recorder
                .dispatch(GameCommand::Search)
                .expect("door search replay command should execute");
            recorder.game().snapshot().cells.iter().any(|cell| {
                cell.position == door_position && cell.terrain_id == "demo.terrain.door-secret"
            })
        });
        assert!(discovered, "fixed seed should discover the secret door");
        let opened = (0..12).any(|_| {
            recorder
                .dispatch(GameCommand::OpenDoor {
                    direction: Direction::East,
                })
                .expect("door open replay command should execute");
            recorder.game().snapshot().cells.iter().any(|cell| {
                cell.position == door_position && cell.terrain_id == "demo.terrain.door-open"
            })
        });
        assert!(opened, "fixed seed should eventually unlock the door");
        let (final_game, replay) = recorder.finish();

        let snapshot = final_game.snapshot();
        assert!(snapshot.cells.iter().any(|cell| {
            cell.position == door_position && cell.terrain_id == "demo.terrain.door-open"
        }));
        let command_count = replay.commands.len();
        let verification = verify(&replay, initial).expect("door replay should verify");
        assert_eq!(verification.commands_verified, command_count);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn door_bash_round_trips_through_replay() {
        let mut payload = Game::new(27).to_save();
        payload.player.position = rfb_protocol::Position { x: 3, y: 4 };
        let initial = Game::from_save(payload).expect("door fixture should load");
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::TraverseStairs,
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::Move {
                direction: Direction::East,
            },
        ] {
            recorder
                .dispatch(command)
                .expect("door bash replay command should execute");
        }
        let door_position = rfb_protocol::Position { x: 10, y: 4 };
        let discovered = (0..12).any(|_| {
            recorder
                .dispatch(GameCommand::Search)
                .expect("door search replay command should execute");
            recorder.game().snapshot().cells.iter().any(|cell| {
                cell.position == door_position && cell.terrain_id == "demo.terrain.door-secret"
            })
        });
        assert!(discovered, "fixed seed should discover the secret door");
        let bashed = (0..12).any(|_| {
            recorder
                .dispatch(GameCommand::BashDoor {
                    direction: Direction::East,
                })
                .expect("door bash replay command should execute");
            recorder.game().snapshot().cells.iter().any(|cell| {
                cell.position == door_position && cell.terrain_id == "demo.terrain.door-broken"
            })
        });
        assert!(bashed, "fixed seed should eventually bash the door");
        let (final_game, replay) = recorder.finish();

        let snapshot = final_game.snapshot();
        assert!(snapshot.cells.iter().any(|cell| {
            cell.position == door_position && cell.terrain_id == "demo.terrain.door-broken"
        }));
        let command_count = replay.commands.len();
        let verification = verify(&replay, initial).expect("door bash replay should verify");
        assert_eq!(verification.commands_verified, command_count);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn equipment_and_batch_drop_round_trip_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::PickUp,
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
                slot_id: None,
            },
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
            GameCommand::Drop {
                item_ids: vec![
                    "demo.item.echo-charm.1".to_owned(),
                    "demo.item.luminous-shard.1".to_owned(),
                ],
            },
        ] {
            recorder.dispatch(command).expect("command should execute");
        }
        let (final_game, replay) = recorder.finish();

        assert!(final_game.snapshot().inventory.is_empty());
        assert!(final_game.snapshot().equipment.is_empty());
        assert_eq!(final_game.snapshot().items.len(), 5);
        let verification = verify(&replay, initial).expect("inventory action replay should verify");
        assert_eq!(verification.commands_verified, 7);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn partial_drop_allocator_round_trips_through_replay() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        for command in [
            GameCommand::Move {
                direction: Direction::East,
            },
            GameCommand::PickUp,
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 1,
            },
        ] {
            recorder.dispatch(command).expect("command should execute");
        }
        let (final_game, replay) = recorder.finish();
        let snapshot = final_game.snapshot();

        assert!(
            snapshot
                .items
                .iter()
                .any(|item| item.id == "generated.item.2")
        );
        assert!(
            snapshot
                .items
                .iter()
                .any(|item| item.id == "generated.item.3")
        );
        assert_eq!(snapshot.inventory[0].quantity, 2);
        let verification = verify(&replay, initial).expect("partial drop replay should verify");
        assert_eq!(verification.commands_verified, 4);
        assert_eq!(verification.final_state_hash, final_game.state_hash());
    }

    #[test]
    fn command_tampering_is_detected_at_checkpoint() {
        let initial = Game::new(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        dispatch_waits(&mut recorder, 3);
        let (_, mut replay) = recorder.finish();
        replay.commands[0].command = GameCommand::Move {
            direction: Direction::East,
        };

        assert!(matches!(
            verify(&replay, initial),
            Err(ReplayError::CheckpointMismatch { .. })
        ));
    }

    #[test]
    fn wrong_initial_state_is_rejected_before_execution() {
        let mut recorder = ReplayRecorder::new(quiet_game(1));
        dispatch_waits(&mut recorder, 1);
        let (_, replay) = recorder.finish();

        assert!(matches!(
            verify(&replay, quiet_game(2)),
            Err(ReplayError::InitialStateMismatch { .. })
        ));
    }

    #[test]
    fn content_hash_mismatch_is_rejected_before_execution() {
        let initial = quiet_game(1);
        let recorder = ReplayRecorder::new(initial.clone());
        let (_, mut replay) = recorder.finish();
        replay.content_hash = "different-content-hash".to_owned();

        assert!(matches!(
            verify(&replay, initial),
            Err(ReplayError::ContentMismatch(hash)) if hash == "different-content-hash"
        ));
    }

    #[test]
    fn command_context_tampering_is_rejected() {
        let initial = quiet_game(42);
        let mut recorder = ReplayRecorder::new(initial.clone());
        dispatch_waits(&mut recorder, 1);
        let (_, mut replay) = recorder.finish();
        replay.commands[0].turn_before = 99;

        assert!(matches!(
            verify(&replay, initial),
            Err(ReplayError::CommandContextMismatch { .. })
        ));
    }

    #[test]
    fn rejected_envelope_is_not_recorded() {
        let mut recorder = ReplayRecorder::new(quiet_game(42));
        let before = recorder.game().state_hash();
        let error = recorder
            .dispatch_envelope(GameCommandEnvelope {
                command_seq: 1,
                expected_revision: 99,
                command: GameCommand::Wait,
            })
            .expect_err("stale command should fail");

        assert!(matches!(
            error,
            ReplayError::Core(CoreError::RevisionMismatch { .. })
        ));
        assert_eq!(recorder.game().state_hash(), before);
        assert!(recorder.replay_snapshot().commands.is_empty());
    }

    #[test]
    fn binary_container_and_debug_json_round_trip() {
        let mut recorder = ReplayRecorder::new(quiet_game(42));
        dispatch_waits(&mut recorder, 3);
        let (_, replay) = recorder.finish();

        let bytes = encode(&replay).expect("replay should encode");
        assert_eq!(decode(&bytes).expect("replay should decode"), replay);
        let json = to_debug_json(&replay).expect("debug JSON should encode");
        assert_eq!(
            from_debug_json(&json).expect("debug JSON should decode"),
            replay
        );
    }

    #[test]
    fn binary_container_detects_corruption() {
        let mut recorder = ReplayRecorder::new(quiet_game(42));
        dispatch_waits(&mut recorder, 1);
        let (_, replay) = recorder.finish();
        let mut bytes = encode(&replay).expect("replay should encode");
        let final_index = bytes.len() - 1;
        bytes[final_index] ^= 0x01;

        assert!(matches!(decode(&bytes), Err(ReplayError::ChecksumMismatch)));
    }

    fn dispatch_waits(recorder: &mut ReplayRecorder, count: usize) {
        for step in 0..count {
            if let Err(error) = recorder.dispatch(GameCommand::Wait) {
                let payload = recorder.game().to_save();
                panic!(
                    "wait {step}/{count} should execute at tick {}, nutrition {}, hp {}: {error}",
                    payload.world_tick, payload.player.nutrition, payload.player.hp
                );
            }
        }
    }

    fn quiet_game(seed: u64) -> Game {
        let mut payload = Game::new(seed).to_save();
        payload.entities.retain(|entity| {
            entity.pack.as_ref().is_some_and(|pack| {
                pack.behavior == rfb_protocol::MonsterPackBehaviorDto::GuardPosition
            })
        });
        payload.carried_items.clear();
        Game::from_save(payload).expect("quiet replay fixture should restore")
    }

    fn monster_caster_game(seed: u64) -> Game {
        let mut payload = Game::new(seed).to_save();
        let mut caster = payload
            .entities
            .into_iter()
            .next()
            .expect("demo save should contain an actor template");
        caster.id = "replay.monster.echo-cantor.1".to_owned();
        caster.kind_id = "demo.actor.echo-cantor".to_owned();
        caster.position = rfb_protocol::Position { x: 8, y: 3 };
        caster.hp = 8;
        caster.max_hp = 8;
        caster.base_speed = 110;
        caster.energy_need = 100;
        caster.alerted = Some(true);
        caster.statuses.clear();
        caster.resistances.clear();
        caster.pack = None;
        caster.summon = None;
        payload.entities = vec![caster];
        payload.carried_items.clear();
        payload
            .dungeon_states
            .iter_mut()
            .find(|state| state.dungeon_id == "demo.dungeon.resonance-descent")
            .expect("resonance dungeon state should exist")
            .entrance_guardian_defeated = Some(true);
        Game::from_save(payload).expect("caster replay fixture should restore")
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
}
