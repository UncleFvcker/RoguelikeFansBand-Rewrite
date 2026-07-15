// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const LEGACY_BASELINE_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";
pub const LEGACY_BASELINE_REFERENCE: &str = "v1.3.0.7";
pub const LEGACY_RNG_DEGREE: usize = 63;
pub const LEGACY_PREFIX_DECODED_LENGTH: usize = 409;
pub const PARSED_SAMPLE_SCHEMA_VERSION: u16 = 1;

const MAX_LEGACY_SAVE_LENGTH: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySaveVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub extra: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacySavePrefix {
    pub version: LegacySaveVersion,
    pub file_length: u64,
    pub file_sha256: String,
    pub system: u32,
    pub saved_at_unix: u32,
    pub lives: u16,
    pub saves: u16,
    pub rng_place: u16,
    pub rng_state: Vec<u32>,
    pub options: LegacyOptionsPrefix,
    pub decoded_prefix_bytes: usize,
    pub encoded_bytes_consumed: usize,
    pub prefix_value_checksum: u32,
    pub prefix_encoded_checksum: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyOptionsPrefix {
    pub delay_factor: u8,
    pub hitpoint_warning: u8,
    pub mana_warning: u8,
    pub random_artifact_percent: u8,
    pub reduce_uniques_percent: u8,
    pub object_list_width: u8,
    pub monster_list_width: u8,
    pub generate_empty: u8,
    pub small_level_type: u8,
    pub cheat_flags: u16,
    pub autosave_level: u8,
    pub autosave_timed: u8,
    pub autosave_frequency: i16,
    pub option_flags: [u32; 8],
    pub option_masks: [u32; 8],
    pub window_flags: [u32; 8],
    pub window_masks: [u32; 8],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParsedSaveBaseline {
    pub schema_version: u16,
    pub legacy_commit: String,
    pub samples: Vec<ParsedSaveSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParsedSaveSample {
    pub id: String,
    pub purpose: String,
    pub prefix: LegacySavePrefix,
}

pub fn parse_prefix(bytes: &[u8]) -> Result<LegacySavePrefix, LegacyImportError> {
    if bytes.len() > MAX_LEGACY_SAVE_LENGTH {
        return Err(LegacyImportError::FileTooLarge(bytes.len()));
    }
    if bytes.len() < 5 {
        return Err(LegacyImportError::TruncatedHeader);
    }
    let version = LegacySaveVersion {
        major: bytes[0],
        minor: bytes[1],
        patch: bytes[2],
        extra: bytes[3],
    };
    if version.major != 1 {
        return Err(LegacyImportError::UnsupportedMajor(version.major));
    }

    let mut reader = LegacyReader::new(bytes)?;
    let system = reader.read_u32()?;
    let saved_at_unix = reader.read_u32()?;
    let lives = reader.read_u16()?;
    let saves = reader.read_u16()?;
    let rng_place = reader.read_u16()?;
    if usize::from(rng_place) >= LEGACY_RNG_DEGREE {
        return Err(LegacyImportError::InvalidRngPlace(rng_place));
    }
    let rng_state = (0..LEGACY_RNG_DEGREE)
        .map(|_| reader.read_u32())
        .collect::<Result<Vec<_>, _>>()?;
    let options = LegacyOptionsPrefix {
        delay_factor: reader.read_u8()?,
        hitpoint_warning: reader.read_u8()?,
        mana_warning: reader.read_u8()?,
        random_artifact_percent: reader.read_u8()?,
        reduce_uniques_percent: reader.read_u8()?,
        object_list_width: reader.read_u8()?,
        monster_list_width: reader.read_u8()?,
        generate_empty: reader.read_u8()?,
        small_level_type: reader.read_u8()?,
        cheat_flags: reader.read_u16()?,
        autosave_level: reader.read_u8()?,
        autosave_timed: reader.read_u8()?,
        autosave_frequency: reader.read_i16()?,
        option_flags: reader.read_u32_array()?,
        option_masks: reader.read_u32_array()?,
        window_flags: reader.read_u32_array()?,
        window_masks: reader.read_u32_array()?,
    };
    if reader.decoded_count != LEGACY_PREFIX_DECODED_LENGTH {
        return Err(LegacyImportError::PrefixLength {
            expected: LEGACY_PREFIX_DECODED_LENGTH,
            actual: reader.decoded_count,
        });
    }

    Ok(LegacySavePrefix {
        version,
        file_length: u64::try_from(bytes.len()).map_err(|_| LegacyImportError::LengthOverflow)?,
        file_sha256: sha256(bytes),
        system,
        saved_at_unix,
        lives,
        saves,
        rng_place,
        rng_state,
        options,
        decoded_prefix_bytes: reader.decoded_count,
        encoded_bytes_consumed: reader.cursor,
        prefix_value_checksum: reader.value_checksum,
        prefix_encoded_checksum: reader.encoded_checksum,
    })
}

pub fn inspect_file(path: &Path) -> Result<LegacySavePrefix, LegacyImportError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_LEGACY_SAVE_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    parse_prefix(&bytes)
}

pub fn record_catalog(catalog_path: &Path) -> Result<PathBuf, LegacyImportError> {
    let output = parsed_baseline_path(catalog_path)?;
    let baseline = parse_catalog(catalog_path)?;
    let encoded = serde_json::to_vec_pretty(&baseline)?;
    let mut file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(LegacyImportError::BaselineAlreadyExists(output));
        }
        Err(error) => return Err(error.into()),
    };
    file.write_all(&encoded)?;
    Ok(output)
}

pub fn verify_catalog(catalog_path: &Path) -> Result<ParsedSaveBaseline, LegacyImportError> {
    let output = parsed_baseline_path(catalog_path)?;
    if !output.is_file() {
        return Err(LegacyImportError::MissingParsedBaseline(output));
    }
    let expected: ParsedSaveBaseline = serde_json::from_slice(&fs::read(&output)?)?;
    let actual = parse_catalog(catalog_path)?;
    if actual != expected {
        return Err(LegacyImportError::BaselineMismatch {
            expected: serde_json::to_string_pretty(&expected)?,
            actual: serde_json::to_string_pretty(&actual)?,
        });
    }
    Ok(actual)
}

fn parse_catalog(catalog_path: &Path) -> Result<ParsedSaveBaseline, LegacyImportError> {
    let catalog: LocalSaveCatalog = serde_json::from_slice(&fs::read(catalog_path)?)?;
    if catalog.schema_version != 1
        || catalog.generated_at_unix == 0
        || !catalog.read_only_source
        || catalog.legacy_reference != LEGACY_BASELINE_REFERENCE
        || catalog.legacy_commit != LEGACY_BASELINE_COMMIT
    {
        return Err(LegacyImportError::CatalogMetadata);
    }
    if catalog.samples.len() < 3 {
        return Err(LegacyImportError::CatalogSampleCount(catalog.samples.len()));
    }
    let root = catalog_path
        .parent()
        .ok_or_else(|| LegacyImportError::CatalogPath(catalog_path.to_path_buf()))?;
    let saves_dir = root.join("saves");
    let mut ids = BTreeSet::new();
    let mut files = BTreeSet::new();
    let mut samples = Vec::with_capacity(catalog.samples.len());

    for sample in catalog.samples {
        if !ids.insert(sample.id.clone()) || !files.insert(sample.file.clone()) {
            return Err(LegacyImportError::DuplicateCatalogEntry(sample.id));
        }
        validate_source_relative_path(&sample.source_relative_path)?;
        let file_name = safe_file_name(&sample.file)?;
        let path = saves_dir.join(file_name);
        let prefix = inspect_file(&path)?;
        if prefix.file_length != sample.length
            || prefix.file_sha256 != sample.sha256
            || prefix.version != sample.header_version
        {
            return Err(LegacyImportError::CatalogSampleMismatch(sample.id));
        }
        samples.push(ParsedSaveSample {
            id: sample.id,
            purpose: sample.purpose,
            prefix,
        });
    }
    samples.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(ParsedSaveBaseline {
        schema_version: PARSED_SAMPLE_SCHEMA_VERSION,
        legacy_commit: catalog.legacy_commit,
        samples,
    })
}

fn parsed_baseline_path(catalog_path: &Path) -> Result<PathBuf, LegacyImportError> {
    Ok(catalog_path
        .parent()
        .ok_or_else(|| LegacyImportError::CatalogPath(catalog_path.to_path_buf()))?
        .join("parsed-save-samples.json"))
}

fn safe_file_name(value: &str) -> Result<&str, LegacyImportError> {
    let path = Path::new(value);
    if path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(LegacyImportError::UnsafeCatalogFile(value.to_owned()));
    }
    Ok(value)
}

fn validate_source_relative_path(value: &str) -> Result<(), LegacyImportError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(LegacyImportError::UnsafeSourceRelativePath(
            value.to_owned(),
        ));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct LegacyReader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    previous_encoded: u8,
    decoded_count: usize,
    value_checksum: u32,
    encoded_checksum: u32,
}

impl<'a> LegacyReader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, LegacyImportError> {
        let previous_encoded = *bytes.get(4).ok_or(LegacyImportError::TruncatedHeader)?;
        Ok(Self {
            bytes,
            cursor: 5,
            previous_encoded,
            decoded_count: 0,
            value_checksum: 0,
            encoded_checksum: 0,
        })
    }

    fn read_u8(&mut self) -> Result<u8, LegacyImportError> {
        let encoded = *self
            .bytes
            .get(self.cursor)
            .ok_or(LegacyImportError::TruncatedPrefix {
                offset: self.cursor,
            })?;
        let value = encoded ^ self.previous_encoded;
        self.previous_encoded = encoded;
        self.cursor += 1;
        self.decoded_count += 1;
        self.value_checksum = self.value_checksum.wrapping_add(u32::from(value));
        self.encoded_checksum = self.encoded_checksum.wrapping_add(u32::from(encoded));
        Ok(value)
    }

    fn read_u16(&mut self) -> Result<u16, LegacyImportError> {
        Ok(u16::from_le_bytes([self.read_u8()?, self.read_u8()?]))
    }

    fn read_i16(&mut self) -> Result<i16, LegacyImportError> {
        Ok(i16::from_le_bytes([self.read_u8()?, self.read_u8()?]))
    }

    fn read_u32(&mut self) -> Result<u32, LegacyImportError> {
        Ok(u32::from_le_bytes([
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
            self.read_u8()?,
        ]))
    }

    fn read_u32_array(&mut self) -> Result<[u32; 8], LegacyImportError> {
        let mut values = [0_u32; 8];
        for value in &mut values {
            *value = self.read_u32()?;
        }
        Ok(values)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocalSaveCatalog {
    schema_version: u16,
    generated_at_unix: u64,
    read_only_source: bool,
    legacy_reference: String,
    legacy_commit: String,
    samples: Vec<LocalSaveSample>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LocalSaveSample {
    id: String,
    file: String,
    source_relative_path: String,
    length: u64,
    sha256: String,
    header_version: LegacySaveVersion,
    purpose: String,
}

#[derive(Debug, Error)]
pub enum LegacyImportError {
    #[error("legacy save header is truncated")]
    TruncatedHeader,
    #[error("legacy save prefix is truncated at encoded offset {offset}")]
    TruncatedPrefix { offset: usize },
    #[error("legacy save major version {0} is unsupported")]
    UnsupportedMajor(u8),
    #[error("legacy RNG position {0} is outside the 63-entry state")]
    InvalidRngPlace(u16),
    #[error("legacy save is too large: {0} bytes")]
    FileTooLarge(usize),
    #[error("legacy save length overflow")]
    LengthOverflow,
    #[error("decoded prefix length mismatch: expected {expected}, actual {actual}")]
    PrefixLength { expected: usize, actual: usize },
    #[error("legacy save catalog metadata does not match the fixed baseline")]
    CatalogMetadata,
    #[error("legacy save catalog must contain at least 3 samples, found {0}")]
    CatalogSampleCount(usize),
    #[error("legacy save catalog path has no parent: {0}")]
    CatalogPath(PathBuf),
    #[error("legacy save catalog contains duplicate entry {0}")]
    DuplicateCatalogEntry(String),
    #[error("legacy save catalog contains unsafe file name {0}")]
    UnsafeCatalogFile(String),
    #[error("legacy save catalog contains unsafe source-relative path {0}")]
    UnsafeSourceRelativePath(String),
    #[error("legacy save sample {0} does not match catalog length, hash, or version")]
    CatalogSampleMismatch(String),
    #[error("parsed legacy baseline already exists: {0}")]
    BaselineAlreadyExists(PathBuf),
    #[error("parsed legacy baseline does not exist: {0}")]
    MissingParsedBaseline(PathBuf),
    #[error("parsed legacy save baseline changed\nexpected:\n{expected}\nactual:\n{actual}")]
    BaselineMismatch { expected: String, actual: String },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_prefix_decodes_all_stable_fields() {
        let decoded = synthetic_decoded_prefix();
        let bytes = encode_legacy([1, 3, 0, 7], 0x5a, &decoded);
        let prefix = parse_prefix(&bytes).expect("synthetic prefix should parse");

        assert_eq!(prefix.version.major, 1);
        assert_eq!(prefix.system, 0x1122_3344);
        assert_eq!(prefix.saved_at_unix, 0x5566_7788);
        assert_eq!(prefix.lives, 3);
        assert_eq!(prefix.saves, 9);
        assert_eq!(prefix.rng_place, 17);
        assert_eq!(prefix.rng_state.len(), 63);
        assert_eq!(prefix.rng_state[0], 1000);
        assert_eq!(prefix.rng_state[62], 1062);
        assert_eq!(prefix.options.delay_factor, 1);
        assert_eq!(prefix.options.autosave_frequency, 250);
        assert_eq!(prefix.options.option_flags[7], 0x1000_0007);
        assert_eq!(prefix.options.window_masks[7], 0x4000_0007);
        assert_eq!(prefix.decoded_prefix_bytes, LEGACY_PREFIX_DECODED_LENGTH);
        assert_eq!(
            prefix.encoded_bytes_consumed,
            5 + LEGACY_PREFIX_DECODED_LENGTH
        );
    }

    #[test]
    fn truncated_and_wrong_major_files_are_rejected() {
        assert!(matches!(
            parse_prefix(&[1, 3, 0]),
            Err(LegacyImportError::TruncatedHeader)
        ));
        let bytes = encode_legacy([2, 0, 0, 0], 1, &synthetic_decoded_prefix());
        assert!(matches!(
            parse_prefix(&bytes),
            Err(LegacyImportError::UnsupportedMajor(2))
        ));
        let truncated = encode_legacy([1, 3, 0, 7], 1, &[0; 20]);
        assert!(matches!(
            parse_prefix(&truncated),
            Err(LegacyImportError::TruncatedPrefix { .. })
        ));

        let mut decoded = synthetic_decoded_prefix();
        decoded[12..14].copy_from_slice(&63_u16.to_le_bytes());
        let invalid_rng = encode_legacy([1, 3, 0, 7], 1, &decoded);
        assert!(matches!(
            parse_prefix(&invalid_rng),
            Err(LegacyImportError::InvalidRngPlace(63))
        ));
    }

    fn synthetic_decoded_prefix() -> Vec<u8> {
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 0x1122_3344);
        push_u32(&mut bytes, 0x5566_7788);
        push_u16(&mut bytes, 3);
        push_u16(&mut bytes, 9);
        push_u16(&mut bytes, 17);
        for value in 1000..1063 {
            push_u32(&mut bytes, value);
        }
        bytes.extend_from_slice(&[1, 2, 3, 4, 5, 80, 60, 2, 1]);
        push_u16(&mut bytes, 0x4202);
        bytes.extend_from_slice(&[1, 0]);
        bytes.extend_from_slice(&250_i16.to_le_bytes());
        for base in [0x1000_0000, 0x2000_0000, 0x3000_0000, 0x4000_0000] {
            for index in 0..8 {
                push_u32(&mut bytes, base + index);
            }
        }
        assert_eq!(bytes.len(), LEGACY_PREFIX_DECODED_LENGTH);
        bytes
    }

    fn encode_legacy(version: [u8; 4], seed: u8, decoded: &[u8]) -> Vec<u8> {
        let mut output = version.to_vec();
        output.push(seed);
        let mut previous = seed;
        for value in decoded {
            let encoded = previous ^ value;
            output.push(encoded);
            previous = encoded;
        }
        output
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}
