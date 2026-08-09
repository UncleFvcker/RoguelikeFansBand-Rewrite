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
mod tests;
