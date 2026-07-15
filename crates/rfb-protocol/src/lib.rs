// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

pub const PROTOCOL_VERSION: &str = "1.0";
pub const DEMO_CONTENT_ID: &str = "rfb.demo.original-v1";
pub const DEMO_CONTENT_HASH: &str =
    "4df1b330468f15704e402764e0f60e5e9a0cbe2586dbce30ed2fe26703ea3de6";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::NorthEast => (1, -1),
            Self::East => (1, 0),
            Self::SouthEast => (1, 1),
            Self::South => (0, 1),
            Self::SouthWest => (-1, 1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, -1),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GameCommand {
    Move { direction: Direction },
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameCommandEnvelope {
    pub command_seq: u32,
    pub expected_revision: u32,
    pub command: GameCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellDto {
    pub position: Position,
    pub terrain_id: String,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameEventDto {
    pub kind: String,
    pub message_key: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameSnapshot {
    pub protocol_version: String,
    pub revision: u32,
    pub turn: u32,
    pub last_command_seq: u32,
    pub width: u16,
    pub height: u16,
    pub cells: Vec<CellDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub content_hash: String,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameUpdate {
    pub base_revision: u32,
    pub revision: u32,
    pub turn: u32,
    pub command_seq: u32,
    pub events: Vec<GameEventDto>,
    pub changed_cells: Vec<CellDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub removed_entities: Vec<String>,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerrainSaveDto {
    pub width: u16,
    pub height: u16,
    pub terrain_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RngSaveDto {
    pub algorithm: String,
    pub state: [u64; 4],
    pub draw_counter: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePayloadV1 {
    pub schema_version: u16,
    pub revision: u32,
    pub turn: u32,
    pub last_command_seq: u32,
    pub terrain: TerrainSaveDto,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub rng: RngSaveDto,
    pub content_id: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterSummary {
    pub display_name: String,
    pub level: u32,
    pub location_key: String,
    pub turn: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveHeaderV1 {
    pub format: String,
    pub save_schema_version: u16,
    pub game_version: String,
    pub protocol_version: String,
    pub created_at: String,
    pub saved_at: String,
    pub character_summary: CharacterSummary,
    pub content_id: String,
    pub content_hash: String,
    pub payload_encoding: String,
}

#[derive(Debug, Error)]
pub enum CodecError {
    #[error("failed to encode MessagePack: {0}")]
    Encode(#[from] rmp_serde::encode::Error),
    #[error("failed to decode MessagePack: {0}")]
    Decode(#[from] rmp_serde::decode::Error),
}

pub fn to_msgpack<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    Ok(rmp_serde::to_vec_named(value)?)
}

pub fn from_msgpack<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    Ok(rmp_serde::from_slice(bytes)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_messagepack_round_trip() {
        let command = GameCommandEnvelope {
            command_seq: 1,
            expected_revision: 0,
            command: GameCommand::Move {
                direction: Direction::SouthEast,
            },
        };

        let encoded = to_msgpack(&command).expect("command should encode");
        let decoded: GameCommandEnvelope = from_msgpack(&encoded).expect("command should decode");
        assert_eq!(decoded, command);
    }
}
