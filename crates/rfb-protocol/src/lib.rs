// SPDX-License-Identifier: MPL-2.0

use std::collections::BTreeMap;

#[cfg(feature = "bindings")]
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;
#[cfg(feature = "bindings")]
use ts_rs::{Config, TS};

pub const PROTOCOL_VERSION: &str = "1.7";

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
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum GameCommand {
    Drop { item_ids: Vec<String> },
    DropQuantity { item_id: String, quantity: u32 },
    Equip { item_id: String },
    Move { direction: Direction },
    PickUp,
    Unequip { slot_id: String },
    Wait,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct StatModifiersDto {
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub max_hp: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct DamageDiceDto {
    pub dice: u16,
    pub sides: u16,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityState {
    Visible,
    Remembered,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellLightDto {
    pub color: u32,
    pub intensity: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct CellVisualDto {
    pub position: Position,
    pub visibility: VisibilityState,
    pub light: CellLightDto,
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
    #[serde(default)]
    pub base_max_hp: i32,
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub base_attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub base_defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
    #[serde(default)]
    pub is_dead: bool,
    #[serde(default)]
    pub equipment_modifiers: StatModifiersDto,
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
    #[serde(default)]
    pub attack: i32,
    #[serde(default)]
    pub defense: i32,
    #[serde(default)]
    pub melee_skill: i32,
    #[serde(default)]
    pub armor_class: i32,
    #[serde(default)]
    pub melee_damage: DamageDiceDto,
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
pub struct InventoryItemDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    #[serde(default)]
    pub equipment_slot: Option<String>,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "bindings", derive(JsonSchema, TS))]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub slot_id: String,
    #[serde(default)]
    pub modifiers: StatModifiersDto,
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
    #[serde(default)]
    pub visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
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
    #[serde(default)]
    pub changed_visual_cells: Vec<CellVisualDto>,
    pub player: PlayerDto,
    pub entities: Vec<EntityDto>,
    pub items: Vec<ItemDto>,
    pub inventory: Vec<InventoryItemDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemDto>,
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
    push_declaration!(StatModifiersDto);
    push_declaration!(DamageDiceDto);
    push_declaration!(Position);
    push_declaration!(CellDto);
    push_declaration!(VisibilityState);
    push_declaration!(CellLightDto);
    push_declaration!(CellVisualDto);
    push_declaration!(ContentVisualDto);
    push_declaration!(PlayerDto);
    push_declaration!(EntityDto);
    push_declaration!(ItemDto);
    push_declaration!(InventoryItemDto);
    push_declaration!(EquipmentItemDto);
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
pub struct PlayerSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    #[serde(default)]
    pub base_max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub hp: i32,
    #[serde(default)]
    pub max_hp: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub position: Position,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EquipmentItemSaveDto {
    pub id: String,
    pub kind_id: String,
    pub quantity: u32,
    pub slot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePayloadV1 {
    pub schema_version: u16,
    pub revision: u32,
    pub turn: u32,
    pub last_command_seq: u32,
    pub terrain: TerrainSaveDto,
    pub player: PlayerSaveDto,
    pub entities: Vec<ActorSaveDto>,
    #[serde(default)]
    pub items: Vec<ItemSaveDto>,
    #[serde(default)]
    pub inventory: Vec<InventoryItemSaveDto>,
    #[serde(default)]
    pub equipment: Vec<EquipmentItemSaveDto>,
    #[serde(default)]
    pub next_item_instance_serial: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub explored: Vec<bool>,
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
    #[serde(default)]
    pub slot_name: String,
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

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct LegacySavePayloadV1 {
        schema_version: u16,
        revision: u32,
        turn: u32,
        last_command_seq: u32,
        terrain: TerrainSaveDto,
        player: PlayerDto,
        entities: Vec<EntityDto>,
        items: Vec<ItemDto>,
        inventory: Vec<InventoryItemDto>,
        equipment: Vec<EquipmentItemDto>,
        next_item_instance_serial: u64,
        explored: Vec<bool>,
        rng: RngSaveDto,
        content_id: String,
        content_hash: String,
        world_id: String,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DerivedFieldProbe {
        #[serde(default)]
        attack: Option<i32>,
        #[serde(default)]
        defense: Option<i32>,
        #[serde(default)]
        melee_skill: Option<i32>,
        #[serde(default)]
        armor_class: Option<i32>,
        #[serde(default)]
        equipment_modifiers: Option<StatModifiersDto>,
    }

    #[test]
    fn command_messagepack_round_trip() {
        for (index, command) in [
            GameCommand::Move {
                direction: Direction::SouthEast,
            },
            GameCommand::PickUp,
            GameCommand::Equip {
                item_id: "demo.item.echo-charm.1".to_owned(),
            },
            GameCommand::Unequip {
                slot_id: "charm".to_owned(),
            },
            GameCommand::Drop {
                item_ids: vec!["demo.item.echo-charm.1".to_owned()],
            },
            GameCommand::DropQuantity {
                item_id: "demo.item.luminous-shard.1".to_owned(),
                quantity: 2,
            },
            GameCommand::Wait,
        ]
        .into_iter()
        .enumerate()
        {
            let envelope = GameCommandEnvelope {
                command_seq: index as u32 + 1,
                expected_revision: index as u32,
                command,
            };
            let encoded = to_msgpack(&envelope).expect("command should encode");
            let decoded: GameCommandEnvelope =
                from_msgpack(&encoded).expect("command should decode");
            assert_eq!(decoded, envelope);
        }
    }

    #[test]
    fn legacy_v1_save_payload_decodes_into_authoritative_save_dtos() {
        let legacy = LegacySavePayloadV1 {
            schema_version: 1,
            revision: 2,
            turn: 2,
            last_command_seq: 2,
            terrain: TerrainSaveDto {
                width: 1,
                height: 1,
                terrain_ids: vec!["demo.terrain.floor".to_owned()],
            },
            player: PlayerDto {
                id: "demo.player".to_owned(),
                kind_id: "demo.actor.explorer".to_owned(),
                position: Position { x: 0, y: 0 },
                hp: 8,
                max_hp: 14,
                base_max_hp: 10,
                attack: 3,
                base_attack: 2,
                defense: 2,
                base_defense: 1,
                melee_skill: 60,
                armor_class: 20,
                melee_damage: DamageDiceDto { dice: 1, sides: 2 },
                is_dead: false,
                equipment_modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
            },
            entities: vec![EntityDto {
                id: "demo.monster.1".to_owned(),
                kind_id: "demo.actor.monster".to_owned(),
                position: Position { x: 1, y: 0 },
                hp: 3,
                max_hp: 3,
                attack: 1,
                defense: 1,
                melee_skill: 32,
                armor_class: 10,
                melee_damage: DamageDiceDto { dice: 1, sides: 2 },
            }],
            items: vec![ItemDto {
                id: "demo.item.ground.1".to_owned(),
                kind_id: "demo.item.shard".to_owned(),
                position: Position { x: 0, y: 0 },
                quantity: 2,
            }],
            inventory: vec![InventoryItemDto {
                id: "demo.item.inventory.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                quantity: 1,
                equipment_slot: Some("charm".to_owned()),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
            }],
            equipment: vec![EquipmentItemDto {
                id: "demo.item.equipment.1".to_owned(),
                kind_id: "demo.item.charm".to_owned(),
                quantity: 1,
                slot_id: "charm".to_owned(),
                modifiers: StatModifiersDto {
                    attack: 1,
                    defense: 1,
                    max_hp: 4,
                },
            }],
            next_item_instance_serial: 4,
            explored: vec![true],
            rng: RngSaveDto {
                algorithm: "rfb-rng-xoshiro256ss-v1".to_owned(),
                state: [1, 2, 3, 4],
                draw_counter: 5,
            },
            content_id: "demo.content".to_owned(),
            content_hash: "0".repeat(64),
            world_id: "demo.world".to_owned(),
        };

        let encoded = to_msgpack(&legacy).expect("legacy payload should encode");
        let decoded: SavePayloadV1 =
            from_msgpack(&encoded).expect("legacy payload should migrate while decoding");

        assert_eq!(decoded.player.base_max_hp, 10);
        assert_eq!(decoded.entities[0].max_hp, 3);
        assert_eq!(decoded.inventory[0].kind_id, "demo.item.charm");
        assert_eq!(decoded.equipment[0].slot_id, "charm");
    }

    #[test]
    fn authoritative_player_save_omits_derived_combat_fields() {
        let player = PlayerSaveDto {
            id: "demo.player".to_owned(),
            kind_id: "demo.actor.explorer".to_owned(),
            position: Position { x: 0, y: 0 },
            hp: 10,
            base_max_hp: 10,
        };

        let encoded = to_msgpack(&player).expect("player save should encode");
        let probe: DerivedFieldProbe =
            from_msgpack(&encoded).expect("derived field probe should decode");

        assert_eq!(probe.attack, None);
        assert_eq!(probe.defense, None);
        assert_eq!(probe.melee_skill, None);
        assert_eq!(probe.armor_class, None);
        assert_eq!(probe.equipment_modifiers, None);
    }

    #[cfg(feature = "bindings")]
    #[test]
    fn generated_bindings_follow_the_serialized_contract() {
        let typescript = generated_typescript();
        assert!(typescript.contains("actorId: string | null"));
        assert!(typescript.contains("commandSeq: number"));
        assert!(typescript.contains("itemIds: Array<string>"));
        assert!(typescript.contains("equipmentModifiers: StatModifiersDto"));
        assert!(typescript.contains("baseAttack: number"));
        assert!(typescript.contains("baseDefense: number"));
        assert!(typescript.contains("attack: number"));
        assert!(typescript.contains("defense: number"));
        assert!(typescript.contains("equipment: Array<EquipmentItemDto>"));
        assert!(typescript.contains("{ \"type\": \"wait\" }"));

        let schema: serde_json::Value = serde_json::from_str(
            &generated_json_schema().expect("protocol schema should serialize"),
        )
        .expect("generated protocol schema should be valid JSON");
        assert_eq!(schema["title"], "ProtocolSchemaV1");
        assert!(schema["$defs"]["GameCommand"].is_object());
    }
}
