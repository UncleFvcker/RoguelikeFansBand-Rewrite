// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "bindings")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
#[cfg(feature = "bindings")]
use ts_rs::{Config, TS};

pub const PROTOCOL_VERSION: &str = "1.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
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
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GameCommand {
    Move { direction: Direction },
    Wait,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameCommandEnvelope {
    pub command_seq: u32,
    pub expected_revision: u32,
    pub command: GameCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellDto {
    pub position: Position,
    pub terrain_id: String,
    pub item_id: Option<String>,
    pub actor_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ContentVisualDto {
    pub id: String,
    pub glyph: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct PlayerDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EntityDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct GameEventDto {
    pub kind: String,
    pub message_key: String,
    pub args: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
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
    pub items: Vec<ItemDto>,
    pub content_id: String,
    pub content_hash: String,
    pub content_visuals: Vec<ContentVisualDto>,
    pub world_id: String,
    pub state_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
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
    pub items: Vec<ItemDto>,
    pub removed_entities: Vec<String>,
    pub state_hash: String,
}

/// Schema bundle for the types crossing the CoreTransport boundary.
#[cfg(feature = "bindings")]
#[derive(JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolSchemaV1 {
    pub command: GameCommandEnvelope,
    pub snapshot: GameSnapshot,
    pub update: GameUpdate,
}

#[must_use]
#[cfg(feature = "bindings")]
pub fn generated_typescript() -> String {
    let config = Config::default();
    let mut output = String::from(
        "// SPDX-License-Identifier: MPL-2.0\n\
         // @generated by `cargo run -p rfb-protocol --features bindings --bin generate-bindings`; do not edit.\n\n",
    );

    macro_rules! push_declaration {
        ($type:ty) => {{
            output.push_str("export ");
            output.push_str(&<$type as TS>::decl(&config));
            output.push_str("\n\n");
        }};
    }

    push_declaration!(Direction);
    push_declaration!(GameCommand);
    push_declaration!(GameCommandEnvelope);
    push_declaration!(Position);
    push_declaration!(CellDto);
    push_declaration!(ContentVisualDto);
    push_declaration!(PlayerDto);
    push_declaration!(EntityDto);
    push_declaration!(ItemDto);
    push_declaration!(GameEventDto);
    push_declaration!(GameSnapshot);
    push_declaration!(GameUpdate);

    output
}

#[cfg(feature = "bindings")]
pub fn generated_json_schema() -> Result<String, serde_json::Error> {
    let mut output = serde_json::to_string_pretty(&schema_for!(ProtocolSchemaV1))?;
    output.push('\n');
    Ok(output)
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
    #[serde(default)]
    pub items: Vec<ItemDto>,
    pub rng: RngSaveDto,
    pub content_id: String,
    pub content_hash: String,
    #[serde(default)]
    pub world_id: String,
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

    #[cfg(feature = "bindings")]
    #[test]
    fn generated_bindings_follow_the_serialized_contract() {
        let typescript = generated_typescript();
        assert!(typescript.contains("actorId: string | null"));
        assert!(typescript.contains("commandSeq: number"));
        assert!(typescript.contains("{ \"type\": \"wait\" }"));

        let schema: serde_json::Value = serde_json::from_str(
            &generated_json_schema().expect("protocol schema should serialize"),
        )
        .expect("generated protocol schema should be valid JSON");
        assert_eq!(schema["title"], "ProtocolSchemaV1");
        assert!(schema["$defs"]["GameCommand"].is_object());
    }
}
