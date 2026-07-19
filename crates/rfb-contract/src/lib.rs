// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeSet;

use rfb_core::{CoreError, Game};
use rfb_protocol::{
    CharacterSummary, GameCommand, GameCommandEnvelope, GameEventDto, ItemKnowledgeSaveDto,
    PROTOCOL_VERSION, Position, ResistanceDto, ResistanceSaveDto, SaveHeaderV1, StatusDto,
    StatusSaveDto,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod approval;
pub mod snapshot;

pub const CONTRACT_SCHEMA_VERSION: u16 = 1;
pub const LEGACY_BASELINE_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";
pub const ORIGINAL_TEST_WORLD: &str = "demo.world.original-v1";
pub const HISTORICAL_TEST_WORLD: &str = "demo.original-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Determinism {
    Exact,
    Semantic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Preconditions {
    pub world: String,
    #[serde(default)]
    pub player_statuses: Vec<StatusSaveDto>,
    #[serde(default)]
    pub player_resistances: Vec<ResistanceSaveDto>,
    #[serde(default)]
    pub entity_effects: Vec<EntityEffectsPrecondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityEffectsPrecondition {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind_id: Option<String>,
    #[serde(default)]
    pub position: Option<Position>,
    #[serde(default)]
    pub hp: Option<i32>,
    #[serde(default)]
    pub energy_need: Option<i32>,
    #[serde(default)]
    pub statuses: Vec<StatusSaveDto>,
    #[serde(default)]
    pub resistances: Vec<ResistanceSaveDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    #[serde(default)]
    pub world_tick: u32,
    pub last_command_seq: u32,
    pub player_position: Position,
    #[serde(default)]
    pub player_hp: Option<i32>,
    #[serde(default)]
    pub player_max_hp: Option<i32>,
    #[serde(default)]
    pub player_attack: Option<i32>,
    #[serde(default)]
    pub player_defense: Option<i32>,
    #[serde(default)]
    pub player_speed: Option<u16>,
    #[serde(default)]
    pub player_energy_need: Option<i32>,
    #[serde(default)]
    pub player_carried_weight_tenths_pound: Option<u32>,
    #[serde(default)]
    pub player_carry_capacity_tenths_pound: Option<u32>,
    #[serde(default)]
    pub player_statuses: Vec<StatusDto>,
    #[serde(default)]
    pub player_resistances: Vec<ResistanceDto>,
    pub entity_count: usize,
    #[serde(default)]
    pub entities: Vec<ActorStateAssertion>,
    #[serde(default)]
    pub ground_item_count: usize,
    #[serde(default)]
    pub inventory_stack_count: usize,
    #[serde(default)]
    pub equipment_count: usize,
    #[serde(default)]
    pub item_knowledge: Vec<ItemKnowledgeSaveDto>,
    #[serde(default)]
    pub next_item_instance_serial: Option<u64>,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorStateAssertion {
    pub id: String,
    pub position: Position,
    pub hp: i32,
    pub speed: u16,
    pub energy_need: i32,
    #[serde(default)]
    pub statuses: Vec<StatusDto>,
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
    PlayerDead,
}

pub fn observe(fixture: &ContractFixture) -> Result<ContractAssertions, ContractError> {
    validate_fixture(fixture)?;
    let seed = parse_seed(&fixture.seed)?;
    let mut payload = Game::new(seed).to_save();
    payload.player.statuses = fixture.preconditions.player_statuses.clone();
    payload.player.resistances = fixture.preconditions.player_resistances.clone();
    for effects in &fixture.preconditions.entity_effects {
        let entity = payload
            .entities
            .iter_mut()
            .find(|entity| entity.id == effects.id)
            .ok_or_else(|| ContractError::UnknownEntityPrecondition(effects.id.clone()))?;
        if let Some(kind_id) = &effects.kind_id {
            entity.kind_id.clone_from(kind_id);
        }
        if let Some(position) = effects.position {
            entity.position = position;
        }
        if let Some(hp) = effects.hp {
            entity.hp = hp;
        }
        if let Some(energy_need) = effects.energy_need {
            entity.energy_need = energy_need;
        }
        entity.statuses = effects.statuses.clone();
        entity.resistances = effects.resistances.clone();
    }
    let mut game = Game::from_save(payload)?;
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
    let save = game.to_save();
    let save_round_trip_state_hash = fixture
        .save_round_trip
        .then(|| save_round_trip(&game))
        .transpose()?;

    Ok(ContractAssertions {
        final_state: FinalStateAssertion {
            revision: snapshot.revision,
            turn: snapshot.turn,
            world_tick: snapshot.world_tick,
            last_command_seq: snapshot.last_command_seq,
            player_position: snapshot.player.position,
            player_hp: Some(snapshot.player.hp),
            player_max_hp: Some(snapshot.player.max_hp),
            player_attack: Some(snapshot.player.attack),
            player_defense: Some(snapshot.player.defense),
            player_speed: Some(snapshot.player.speed),
            player_energy_need: Some(snapshot.player.energy_need),
            player_carried_weight_tenths_pound: Some(snapshot.player.carried_weight_tenths_pound),
            player_carry_capacity_tenths_pound: Some(snapshot.player.carry_capacity_tenths_pound),
            player_statuses: snapshot.player.statuses.clone(),
            player_resistances: snapshot.player.resistances.clone(),
            entity_count: snapshot.entities.len(),
            entities: snapshot
                .entities
                .iter()
                .map(|entity| ActorStateAssertion {
                    id: entity.id.clone(),
                    position: entity.position,
                    hp: entity.hp,
                    speed: entity.speed,
                    energy_need: entity.energy_need,
                    statuses: entity.statuses.clone(),
                })
                .collect(),
            ground_item_count: snapshot.items.len(),
            inventory_stack_count: snapshot.inventory.len(),
            equipment_count: snapshot.equipment.len(),
            item_knowledge: save.item_knowledge,
            next_item_instance_serial: Some(save.next_item_instance_serial),
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
    if fixture.preconditions.world != ORIGINAL_TEST_WORLD
        && fixture.preconditions.world != HISTORICAL_TEST_WORLD
    {
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
        CoreError::PlayerDead => Ok(CommandErrorKind::PlayerDead),
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
        slot_name: "契约回环".to_owned(),
        created_at: "2026-07-15T00:00:00Z".to_owned(),
        saved_at: "2026-07-15T00:01:00Z".to_owned(),
        character_summary: CharacterSummary {
            display_name: "原创契约测试探索者".to_owned(),
            level: 1,
            location_key: game.location_key().to_owned(),
            turn: snapshot.turn,
        },
        content_id: snapshot.content_id.clone(),
        content_hash: snapshot.content_hash.clone(),
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
    #[error("contract fixture references unknown entity precondition {0}")]
    UnknownEntityPrecondition(String),
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
