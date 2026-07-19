// SPDX-License-Identifier: MPL-2.0

use rfb_protocol::{SaveHeaderV1, SavePayloadV1, from_msgpack, to_msgpack};
use sha2::{Digest, Sha256};
use thiserror::Error;

const MAGIC: &[u8; 8] = b"RFBSAVE\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 4 + 8 + 32;
const MAX_HEADER_LENGTH: usize = 64 * 1024;
const MAX_PAYLOAD_LENGTH: usize = 16 * 1024 * 1024;

pub fn encode(header: &SaveHeaderV1, payload: &SavePayloadV1) -> Result<Vec<u8>, SaveError> {
    validate_header(header)?;
    let header_json = serde_json::to_vec(header)?;
    let payload_messagepack = to_msgpack(payload)?;
    if header_json.len() > MAX_HEADER_LENGTH {
        return Err(SaveError::HeaderTooLarge(header_json.len()));
    }
    if payload_messagepack.len() > MAX_PAYLOAD_LENGTH {
        return Err(SaveError::PayloadTooLarge(payload_messagepack.len()));
    }

    let checksum = Sha256::digest(&payload_messagepack);
    let capacity = FIXED_HEADER_LENGTH
        .checked_add(header_json.len())
        .and_then(|length| length.checked_add(payload_messagepack.len()))
        .ok_or(SaveError::LengthOverflow)?;
    let mut output = Vec::with_capacity(capacity);
    output.extend_from_slice(MAGIC);
    output.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(
        &u32::try_from(header_json.len())
            .map_err(|_| SaveError::LengthOverflow)?
            .to_le_bytes(),
    );
    output.extend_from_slice(
        &u64::try_from(payload_messagepack.len())
            .map_err(|_| SaveError::LengthOverflow)?
            .to_le_bytes(),
    );
    output.extend_from_slice(&checksum);
    output.extend_from_slice(&header_json);
    output.extend_from_slice(&payload_messagepack);
    Ok(output)
}

pub fn decode(bytes: &[u8]) -> Result<(SaveHeaderV1, SavePayloadV1), SaveError> {
    if bytes.len() < FIXED_HEADER_LENGTH {
        return Err(SaveError::Truncated);
    }
    if &bytes[..8] != MAGIC {
        return Err(SaveError::InvalidMagic);
    }

    let version = u16::from_le_bytes(bytes[8..10].try_into().map_err(|_| SaveError::Truncated)?);
    if version != CONTAINER_VERSION {
        return Err(SaveError::UnsupportedContainerVersion(version));
    }
    let flags = u16::from_le_bytes(bytes[10..12].try_into().map_err(|_| SaveError::Truncated)?);
    if flags != 0 {
        return Err(SaveError::UnsupportedFlags(flags));
    }
    let header_length =
        u32::from_le_bytes(bytes[12..16].try_into().map_err(|_| SaveError::Truncated)?) as usize;
    let payload_length = usize::try_from(u64::from_le_bytes(
        bytes[16..24].try_into().map_err(|_| SaveError::Truncated)?,
    ))
    .map_err(|_| SaveError::LengthOverflow)?;

    if header_length > MAX_HEADER_LENGTH {
        return Err(SaveError::HeaderTooLarge(header_length));
    }
    if payload_length > MAX_PAYLOAD_LENGTH {
        return Err(SaveError::PayloadTooLarge(payload_length));
    }
    let expected_length = FIXED_HEADER_LENGTH
        .checked_add(header_length)
        .and_then(|length| length.checked_add(payload_length))
        .ok_or(SaveError::LengthOverflow)?;
    if bytes.len() != expected_length {
        return Err(SaveError::Truncated);
    }

    let expected_checksum = &bytes[24..56];
    let header_start = FIXED_HEADER_LENGTH;
    let payload_start = header_start + header_length;
    let payload_bytes = &bytes[payload_start..];
    let actual_checksum = Sha256::digest(payload_bytes);
    if expected_checksum != actual_checksum.as_slice() {
        return Err(SaveError::ChecksumMismatch);
    }

    let header: SaveHeaderV1 = serde_json::from_slice(&bytes[header_start..payload_start])?;
    validate_header(&header)?;
    let payload = from_msgpack(payload_bytes)?;
    Ok((header, payload))
}

fn validate_header(header: &SaveHeaderV1) -> Result<(), SaveError> {
    if header.format != "rfb-save" {
        return Err(SaveError::InvalidHeader("format"));
    }
    if header.save_schema_version != 1 {
        return Err(SaveError::InvalidHeader("saveSchemaVersion"));
    }
    if header.payload_encoding != "messagepack" {
        return Err(SaveError::InvalidHeader("payloadEncoding"));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("save file is truncated or has trailing bytes")]
    Truncated,
    #[error("save file magic is invalid")]
    InvalidMagic,
    #[error("unsupported save container version {0}")]
    UnsupportedContainerVersion(u16),
    #[error("unsupported save flags 0x{0:04x}")]
    UnsupportedFlags(u16),
    #[error("save header is too large: {0} bytes")]
    HeaderTooLarge(usize),
    #[error("save payload is too large: {0} bytes")]
    PayloadTooLarge(usize),
    #[error("save length overflow")]
    LengthOverflow,
    #[error("save payload checksum does not match")]
    ChecksumMismatch,
    #[error("save header field is invalid: {0}")]
    InvalidHeader(&'static str),
    #[error("save header JSON is invalid: {0}")]
    Json(#[from] serde_json::Error),
    #[error("save payload MessagePack is invalid: {0}")]
    MessagePack(#[from] rfb_protocol::CodecError),
}

#[cfg(test)]
mod tests {
    use rfb_protocol::{
        ActorSaveDto, CharacterSummary, InventoryItemSaveDto, ItemSaveDto, PROTOCOL_VERSION,
        PlayerSaveDto, Position, RngSaveDto, TerrainSaveDto,
    };

    use super::*;

    fn fixture() -> (SaveHeaderV1, SavePayloadV1) {
        let header = SaveHeaderV1 {
            format: "rfb-save".to_owned(),
            save_schema_version: 1,
            game_version: "0.1.0".to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            slot_name: "测试存档".to_owned(),
            created_at: "2026-07-15T00:00:00Z".to_owned(),
            saved_at: "2026-07-15T00:00:00Z".to_owned(),
            character_summary: CharacterSummary {
                display_name: "探索者".to_owned(),
                level: 1,
                location_key: "location-demo".to_owned(),
                turn: 0,
            },
            content_id: "rfb.test.content-v1".to_owned(),
            content_hash: "0".repeat(64),
            payload_encoding: "messagepack".to_owned(),
        };
        let payload = SavePayloadV1 {
            schema_version: 1,
            revision: 0,
            turn: 0,
            world_tick: 0,
            last_command_seq: 0,
            terrain: TerrainSaveDto {
                width: 1,
                height: 1,
                terrain_ids: vec!["demo.terrain.floor".to_owned()],
            },
            player: PlayerSaveDto {
                id: "demo.player".to_owned(),
                kind_id: "demo.actor.explorer".to_owned(),
                position: Position { x: 0, y: 0 },
                hp: 10,
                base_max_hp: 10,
                base_speed: 110,
                energy_need: 0,
                statuses: Vec::new(),
                resistances: Vec::new(),
            },
            entities: Vec::<ActorSaveDto>::new(),
            items: Vec::<ItemSaveDto>::new(),
            inventory: Vec::<InventoryItemSaveDto>::new(),
            equipment: Vec::new(),
            item_knowledge: Vec::new(),
            item_property_knowledge: Vec::new(),
            next_item_instance_serial: 1,
            explored: vec![true],
            rng: RngSaveDto {
                algorithm: "rfb-rng-xoshiro256ss-v1".to_owned(),
                state: [1, 2, 3, 4],
                draw_counter: 0,
            },
            content_id: "rfb.test.content-v1".to_owned(),
            content_hash: "0".repeat(64),
            world_id: "test.world.fixture".to_owned(),
        };
        (header, payload)
    }

    #[test]
    fn container_round_trip() {
        let (header, payload) = fixture();
        let bytes = encode(&header, &payload).expect("save should encode");
        let (decoded_header, decoded_payload) = decode(&bytes).expect("save should decode");
        assert_eq!(decoded_header, header);
        assert_eq!(decoded_payload, payload);
    }

    #[test]
    fn payload_corruption_is_detected() {
        let (header, payload) = fixture();
        let mut bytes = encode(&header, &payload).expect("save should encode");
        let final_index = bytes.len() - 1;
        bytes[final_index] ^= 0x01;
        assert!(matches!(decode(&bytes), Err(SaveError::ChecksumMismatch)));
    }
}
