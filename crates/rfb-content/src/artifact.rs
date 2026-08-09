// SPDX-License-Identifier: MPL-2.0

use std::{fs::File, io::Read, path::Path};

use sha2::{Digest, Sha256};

use super::{CompiledContentV1, ContentError, ContentSummary};
use crate::validation::validate_and_normalize;

const MAGIC: &[u8; 8] = b"RFBCONT\0";
const CONTAINER_VERSION: u16 = 1;
const FIXED_HEADER_LENGTH: usize = 8 + 2 + 2 + 8 + 32;
const MAX_COMPILED_PAYLOAD_LENGTH: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledArtifact {
    pub content: CompiledContentV1,
    pub content_hash: String,
    pub bytes: Vec<u8>,
}

impl CompiledArtifact {
    #[must_use]
    pub fn summary(&self) -> ContentSummary {
        ContentSummary {
            pack_id: self.content.pack_id.clone(),
            pack_version: self.content.pack_version.clone(),
            content_hash: self.content_hash.clone(),
            terrain_count: self.content.terrain.len(),
            actor_count: self.content.actors.len(),
            affix_count: self.content.affixes.len(),
            item_count: self.content.items.len(),
            resource_count: self.content.resources.len(),
            ability_count: self.content.abilities.len(),
            ability_book_count: self.content.ability_books.len(),
            skill_count: self.content.skills.len(),
            skill_set_count: self.content.skill_sets.len(),
            race_count: self.content.races.len(),
            class_count: self.content.classes.len(),
            personality_count: self.content.personalities.len(),
            build_count: self.content.builds.len(),
            mutation_count: self.content.mutations.len(),
            encounter_table_count: self.content.encounter_tables.len(),
            loot_table_count: self.content.loot_tables.len(),
            theme_table_count: self.content.theme_tables.len(),
            region_table_count: self.content.region_tables.len(),
            terrain_feature_table_count: self.content.terrain_feature_tables.len(),
            vault_count: self.content.vaults.len(),
            town_count: self.content.towns.len(),
            town_facility_count: self.content.town_facilities.len(),
            shop_count: self.content.shops.len(),
            world_count: self.content.worlds.len(),
        }
    }
}

pub fn encode_content(mut content: CompiledContentV1) -> Result<CompiledArtifact, ContentError> {
    validate_and_normalize(&mut content)?;
    let payload = rmp_serde::to_vec_named(&content)?;
    if payload.len() > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload.len()));
    }
    let content_hash = sha256(&payload);
    let payload_length = u64::try_from(payload.len()).map_err(|_| ContentError::LengthOverflow)?;
    let capacity = FIXED_HEADER_LENGTH
        .checked_add(payload.len())
        .ok_or(ContentError::LengthOverflow)?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&CONTAINER_VERSION.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&Sha256::digest(&payload));
    bytes.extend_from_slice(&payload);
    Ok(CompiledArtifact {
        content,
        content_hash,
        bytes,
    })
}

pub fn decode_content(bytes: &[u8]) -> Result<CompiledArtifact, ContentError> {
    if bytes.len() < FIXED_HEADER_LENGTH || &bytes[..8] != MAGIC {
        return Err(ContentError::InvalidContainer);
    }
    let version = read_u16(bytes, 8)?;
    if version != CONTAINER_VERSION {
        return Err(ContentError::UnsupportedContainerVersion(version));
    }
    let flags = read_u16(bytes, 10)?;
    if flags != 0 {
        return Err(ContentError::UnsupportedContainerFlags(flags));
    }
    let payload_length =
        usize::try_from(read_u64(bytes, 12)?).map_err(|_| ContentError::LengthOverflow)?;
    if payload_length > MAX_COMPILED_PAYLOAD_LENGTH {
        return Err(ContentError::CompiledPayloadTooLarge(payload_length));
    }
    let expected_length = FIXED_HEADER_LENGTH
        .checked_add(payload_length)
        .ok_or(ContentError::LengthOverflow)?;
    if bytes.len() != expected_length {
        return Err(ContentError::InvalidContainer);
    }
    let payload = &bytes[FIXED_HEADER_LENGTH..];
    let actual_checksum = Sha256::digest(payload);
    if bytes[20..52] != actual_checksum[..] {
        return Err(ContentError::ChecksumMismatch);
    }
    let content: CompiledContentV1 = rmp_serde::from_slice(payload)?;
    let mut normalized = content.clone();
    validate_and_normalize(&mut normalized)?;
    if normalized != content {
        return Err(ContentError::NonCanonicalCompiledContent);
    }
    Ok(CompiledArtifact {
        content,
        content_hash: sha256(payload),
        bytes: bytes.to_vec(),
    })
}

pub fn read_compiled_file(path: &Path) -> Result<CompiledArtifact, ContentError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((FIXED_HEADER_LENGTH + MAX_COMPILED_PAYLOAD_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    decode_content(&bytes)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ContentError> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ContentError> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or(ContentError::InvalidContainer)?
            .try_into()
            .map_err(|_| ContentError::InvalidContainer)?,
    ))
}
