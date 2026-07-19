// SPDX-License-Identifier: MPL-2.0

use rfb_core::{CoreError, Game};
use rfb_protocol::{GameCommand, GameCommandEnvelope, GameUpdate, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const REPLAY_FORMAT: &str = "rfb-replay";
pub const REPLAY_FORMAT_VERSION: u16 = 1;
pub const STATE_HASH_SCHEMA_VERSION: u16 = 9;
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
        let snapshot = game.snapshot();
        let replay = ReplayV1 {
            format: REPLAY_FORMAT.to_owned(),
            format_version: REPLAY_FORMAT_VERSION,
            core_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            content_hash: snapshot.content_hash,
            initial_save_hash: snapshot.state_hash,
            rng_algorithm: game.rng_algorithm().to_owned(),
            state_hash_schema_version: STATE_HASH_SCHEMA_VERSION,
            commands: Vec::new(),
            checkpoints: Vec::new(),
        };
        Self { game, replay }
    }

    pub fn dispatch(&mut self, command: GameCommand) -> Result<GameUpdate, ReplayError> {
        let before = self.game.snapshot();
        self.dispatch_envelope(GameCommandEnvelope {
            command_seq: before.last_command_seq.saturating_add(1),
            expected_revision: before.revision,
            command,
        })
    }

    pub fn dispatch_envelope(
        &mut self,
        envelope: GameCommandEnvelope,
    ) -> Result<GameUpdate, ReplayError> {
        let before = self.game.snapshot();
        let recorded = ReplayCommand {
            command_seq: envelope.command_seq,
            expected_revision: envelope.expected_revision,
            turn_before: before.turn,
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
            let snapshot = self.game.snapshot();
            replay.checkpoints.push(ReplayCheckpoint {
                after_command_seq: command.command_seq,
                revision: snapshot.revision,
                turn: snapshot.turn,
                rng_draw_counter: self.game.rng_draw_counter(),
                state_hash: snapshot.state_hash,
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
        let snapshot = self.game.snapshot();
        self.replay.checkpoints.push(ReplayCheckpoint {
            after_command_seq,
            revision: snapshot.revision,
            turn: snapshot.turn,
            rng_draw_counter: self.game.rng_draw_counter(),
            state_hash: snapshot.state_hash,
        });
    }
}

pub fn verify(replay: &ReplayV1, mut game: Game) -> Result<ReplayVerification, ReplayError> {
    validate_metadata(replay, &game)?;
    validate_checkpoint_schedule(replay)?;
    let mut checkpoint_index = 0;

    for (index, recorded) in replay.commands.iter().enumerate() {
        let before = game.snapshot();
        if recorded.command_seq != before.last_command_seq.saturating_add(1)
            || recorded.expected_revision != before.revision
            || recorded.turn_before != before.turn
        {
            return Err(ReplayError::CommandContextMismatch {
                index: index + 1,
                expected_seq: before.last_command_seq.saturating_add(1),
                received_seq: recorded.command_seq,
                expected_revision: before.revision,
                received_revision: recorded.expected_revision,
                expected_turn: before.turn,
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
    let snapshot = game.snapshot();
    let actual = ReplayCheckpoint {
        after_command_seq: snapshot.last_command_seq,
        revision: snapshot.revision,
        turn: snapshot.turn,
        rng_draw_counter: game.rng_draw_counter(),
        state_hash: snapshot.state_hash,
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
    use rfb_protocol::Direction;

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
        let initial = quiet_game(0x0123_4567_89ab_cdef);
        let mut recorder = ReplayRecorder::new(initial.clone());
        dispatch_waits(&mut recorder, 10_000);
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

        assert_eq!(final_game.snapshot().items.len(), 1);
        assert_eq!(final_game.snapshot().inventory.len(), 1);
        let verification = verify(&replay, initial).expect("pickup replay should verify");
        assert_eq!(verification.commands_verified, 2);
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
        assert_eq!(final_game.snapshot().items.len(), 2);
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
                .any(|item| item.id == "generated.item.1")
        );
        assert!(
            snapshot
                .items
                .iter()
                .any(|item| item.id == "generated.item.2")
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
        for _ in 0..count {
            recorder
                .dispatch(GameCommand::Wait)
                .expect("wait should execute");
        }
    }

    fn quiet_game(seed: u64) -> Game {
        let mut payload = Game::new(seed).to_save();
        payload.entities.clear();
        Game::from_save(payload).expect("monster-free replay fixture should restore")
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
