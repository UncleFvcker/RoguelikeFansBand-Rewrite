// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_core::{CoreError, Game};
use rfb_protocol::{
    CharacterSummary, DEMO_CONTENT_HASH, DEMO_CONTENT_ID, GameCommand, GameCommandEnvelope,
    GameEventDto, PROTOCOL_VERSION, Position, SaveHeaderV1,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod snapshot;

pub const CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const LEGACY_BASELINE_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";
pub const ORIGINAL_TEST_WORLD: &str = "demo.original-v1";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractFixture {
    pub schema_version: u16,
    pub id: String,
    pub legacy_commit: String,
    pub determinism: Determinism,
    pub seed: String,
    pub preconditions: Preconditions,
    pub commands: Vec<ContractCommand>,
    #[serde(default)]
    pub save_round_trip: bool,
    pub assertions: Option<ContractAssertions>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Determinism {
    Exact,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preconditions {
    pub world: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractCommand {
    pub command: GameCommand,
    pub command_seq: Option<u32>,
    pub expected_revision: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractAssertions {
    pub final_state: FinalStateAssertion,
    pub events: Vec<GameEventDto>,
    pub changed_cells: Vec<Position>,
    pub removed_entities: Vec<String>,
    pub errors: Vec<CommandErrorAssertion>,
    pub save_round_trip_state_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinalStateAssertion {
    pub revision: u32,
    pub turn: u32,
    pub last_command_seq: u32,
    pub player_position: Position,
    pub entity_count: usize,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandErrorAssertion {
    pub step: usize,
    pub kind: CommandErrorKind,
    pub state_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandErrorKind {
    RevisionMismatch,
    CommandSequence,
}

pub fn observe(fixture: &ContractFixture) -> Result<ContractAssertions, ContractError> {
    validate_fixture(fixture)?;
    let seed = parse_seed(&fixture.seed)?;
    let mut game = Game::new(seed);
    let mut events = Vec::new();
    let mut changed_cells = Vec::new();
    let mut removed_entities = Vec::new();
    let mut errors = Vec::new();

    for (index, contract_command) in fixture.commands.iter().enumerate() {
        let snapshot = game.snapshot();
        let envelope = GameCommandEnvelope {
            command_seq: contract_command
                .command_seq
                .unwrap_or_else(|| snapshot.last_command_seq.saturating_add(1)),
            expected_revision: contract_command
                .expected_revision
                .unwrap_or(snapshot.revision),
            command: contract_command.command.clone(),
        };
        match game.dispatch(envelope) {
            Ok(update) => {
                events.extend(update.events);
                changed_cells.extend(update.changed_cells.into_iter().map(|cell| cell.position));
                removed_entities.extend(update.removed_entities);
            }
            Err(error) => errors.push(CommandErrorAssertion {
                step: index + 1,
                kind: command_error_kind(&error)?,
                state_hash: game.state_hash(),
            }),
        }
    }

    let snapshot = game.snapshot();
    let save_round_trip_state_hash = fixture
        .save_round_trip
        .then(|| save_round_trip(&game))
        .transpose()?;

    Ok(ContractAssertions {
        final_state: FinalStateAssertion {
            revision: snapshot.revision,
            turn: snapshot.turn,
            last_command_seq: snapshot.last_command_seq,
            player_position: snapshot.player.position,
            entity_count: snapshot.entities.len(),
            state_hash: snapshot.state_hash,
        },
        events,
        changed_cells,
        removed_entities,
        errors,
        save_round_trip_state_hash,
    })
}

pub fn verify(fixture: &ContractFixture) -> Result<(), ContractError> {
    let expected = fixture
        .assertions
        .as_ref()
        .ok_or_else(|| ContractError::MissingAssertions(fixture.id.clone()))?;
    let actual = observe(fixture)?;
    if &actual == expected {
        return Ok(());
    }
    Err(ContractError::AssertionMismatch {
        id: fixture.id.clone(),
        expected: serde_json::to_string_pretty(expected)?,
        actual: serde_json::to_string_pretty(&actual)?,
    })
}

pub fn validate_fixture_set(fixtures: &[ContractFixture]) -> Result<(), ContractError> {
    let mut ids = BTreeSet::new();
    for fixture in fixtures {
        validate_fixture(fixture)?;
        if !ids.insert(fixture.id.clone()) {
            return Err(ContractError::DuplicateId(fixture.id.clone()));
        }
    }
    Ok(())
}

fn validate_fixture(fixture: &ContractFixture) -> Result<(), ContractError> {
    if fixture.schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(ContractError::UnsupportedSchema(fixture.schema_version));
    }
    if fixture.legacy_commit != LEGACY_BASELINE_COMMIT {
        return Err(ContractError::LegacyCommit(fixture.legacy_commit.clone()));
    }
    if fixture.preconditions.world != ORIGINAL_TEST_WORLD {
        return Err(ContractError::UnknownWorld(
            fixture.preconditions.world.clone(),
        ));
    }
    if fixture.id.trim().is_empty() {
        return Err(ContractError::EmptyId);
    }
    Ok(())
}

fn parse_seed(seed: &str) -> Result<u64, ContractError> {
    if let Some(hex) = seed.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16)
            .map_err(|_| ContractError::InvalidSeed(seed.to_owned()));
    }
    seed.parse::<u64>()
        .map_err(|_| ContractError::InvalidSeed(seed.to_owned()))
}

fn command_error_kind(error: &CoreError) -> Result<CommandErrorKind, ContractError> {
    match error {
        CoreError::RevisionMismatch { .. } => Ok(CommandErrorKind::RevisionMismatch),
        CoreError::CommandSequence { .. } => Ok(CommandErrorKind::CommandSequence),
        other => Err(ContractError::UnexpectedCoreError(other.to_string())),
    }
}

fn save_round_trip(game: &Game) -> Result<String, ContractError> {
    let snapshot = game.snapshot();
    let header = SaveHeaderV1 {
        format: "rfb-save".to_owned(),
        save_schema_version: 1,
        game_version: env!("CARGO_PKG_VERSION").to_owned(),
        protocol_version: PROTOCOL_VERSION.to_owned(),
        created_at: "2026-07-15T00:00:00Z".to_owned(),
        saved_at: "2026-07-15T00:01:00Z".to_owned(),
        character_summary: CharacterSummary {
            display_name: "原创契约测试探索者".to_owned(),
            level: 1,
            location_key: "location-demo-lab".to_owned(),
            turn: snapshot.turn,
        },
        content_id: DEMO_CONTENT_ID.to_owned(),
        content_hash: DEMO_CONTENT_HASH.to_owned(),
        payload_encoding: "messagepack".to_owned(),
    };
    let bytes = rfb_save::encode(&header, &game.to_save())?;
    let (_, payload) = rfb_save::decode(&bytes)?;
    let restored = Game::from_save(payload)?;
    if restored.snapshot() != snapshot {
        return Err(ContractError::SaveRoundTripMismatch);
    }
    Ok(restored.state_hash())
}

#[derive(Debug, Error)]
pub enum ContractError {
    #[error("unsupported contract schema version {0}")]
    UnsupportedSchema(u16),
    #[error("contract fixture uses unexpected legacy commit {0}")]
    LegacyCommit(String),
    #[error("contract fixture uses unknown test world {0}")]
    UnknownWorld(String),
    #[error("contract fixture ID cannot be empty")]
    EmptyId,
    #[error("duplicate contract fixture ID {0}")]
    DuplicateId(String),
    #[error("invalid contract seed {0}")]
    InvalidSeed(String),
    #[error("fixture {0} does not contain assertions")]
    MissingAssertions(String),
    #[error("fixture {id} did not match\nexpected:\n{expected}\nactual:\n{actual}")]
    AssertionMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("unexpected core error: {0}")]
    UnexpectedCoreError(String),
    #[error("save round trip changed the authoritative snapshot")]
    SaveRoundTripMismatch,
    #[error(transparent)]
    Save(#[from] rfb_save::SaveError),
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
