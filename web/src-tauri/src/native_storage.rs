// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rfb_core::Game;
use rfb_protocol::{GameSnapshot, SaveHeaderV1};
use serde::Serialize;

const SAVE_EXTENSION: &str = ".rfbsave";
const BACKUP_COUNT: u8 = 3;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCommandError {
    pub code: String,
    pub detail: String,
}

impl DesktopCommandError {
    pub fn new(code: &str, detail: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            detail: detail.into(),
        }
    }

    fn io(code: &str, error: std::io::Error) -> Self {
        Self::new(code, error.to_string())
    }
}

pub type DesktopResult<T> = Result<T, DesktopCommandError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum NativeSaveStatus {
    Ready,
    Recoverable,
    Corrupt,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSaveSummary {
    pub slot_id: String,
    pub slot_name: String,
    pub status: NativeSaveStatus,
    pub recovery_backup: Option<u8>,
    pub saved_at: Option<String>,
    pub created_at: Option<String>,
    pub turn: Option<u32>,
    pub location_key: Option<String>,
    pub content_id: Option<String>,
    pub content_hash: Option<String>,
    pub state_hash: Option<String>,
}

#[derive(Debug)]
pub struct NativeLoadedSave {
    pub bytes: Vec<u8>,
    pub recovery_backup: Option<u8>,
}

pub struct NativeSaveStore {
    root: PathBuf,
}

impl NativeSaveStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn ensure_ready(&self) -> DesktopResult<()> {
        fs::create_dir_all(&self.root)
            .map_err(|error| DesktopCommandError::io("native-save-directory", error))?;
        self.cleanup_stale_temps()
    }

    pub fn create_slot_id(&self) -> DesktopResult<String> {
        self.ensure_ready()?;
        let milliseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| DesktopCommandError::new("native-save-clock", error.to_string()))?
            .as_millis();
        for suffix in 0..100_u8 {
            let slot_id = if suffix == 0 {
                format!("save-{milliseconds}")
            } else {
                format!("save-{milliseconds}-{suffix}")
            };
            if !self.primary_path(&slot_id).exists() {
                return Ok(slot_id);
            }
        }
        Err(DesktopCommandError::new(
            "native-save-id-exhausted",
            "could not allocate a unique save slot",
        ))
    }

    pub fn list(&self) -> DesktopResult<Vec<NativeSaveSummary>> {
        self.ensure_ready()?;
        let mut slot_ids = BTreeSet::new();
        let entries = fs::read_dir(&self.root)
            .map_err(|error| DesktopCommandError::io("native-save-list", error))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| DesktopCommandError::io("native-save-list", error))?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            if let Some(slot_id) = slot_id_from_file_name(&name)
                && validate_slot_id(&slot_id).is_ok()
            {
                slot_ids.insert(slot_id);
            }
        }

        let mut summaries = slot_ids
            .into_iter()
            .map(|slot_id| self.summary(&slot_id))
            .collect::<Vec<_>>();
        summaries.sort_by(|left, right| {
            right
                .saved_at
                .cmp(&left.saved_at)
                .then_with(|| left.slot_id.cmp(&right.slot_id))
        });
        Ok(summaries)
    }

    pub fn write(&self, slot_id: &str, bytes: &[u8]) -> DesktopResult<NativeSaveSummary> {
        validate_slot_id(slot_id)?;
        self.ensure_ready()?;
        decode_snapshot(bytes)?;

        let primary = self.primary_path(slot_id);
        let temporary = self.temporary_path(slot_id)?;
        let write_result = (|| -> DesktopResult<()> {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| DesktopCommandError::io("native-save-temp-create", error))?;
            file.write_all(bytes)
                .map_err(|error| DesktopCommandError::io("native-save-write", error))?;
            file.sync_all()
                .map_err(|error| DesktopCommandError::io("native-save-sync", error))?;
            let verified = fs::read(&temporary)
                .map_err(|error| DesktopCommandError::io("native-save-verify-read", error))?;
            decode_snapshot(&verified)?;
            self.rotate_backups(slot_id)?;
            if let Err(error) = fs::rename(&temporary, &primary) {
                let backup = self.backup_path(slot_id, 1);
                if backup.exists() && !primary.exists() {
                    let _ = fs::rename(&backup, &primary);
                }
                return Err(DesktopCommandError::io("native-save-commit", error));
            }
            let committed = fs::read(&primary)
                .map_err(|error| DesktopCommandError::io("native-save-commit-read", error))?;
            decode_snapshot(&committed)?;
            Ok(())
        })();
        if write_result.is_err() && temporary.exists() {
            let _ = fs::remove_file(&temporary);
        }
        write_result?;
        Ok(self.summary(slot_id))
    }

    pub fn load(&self, slot_id: &str) -> DesktopResult<NativeLoadedSave> {
        validate_slot_id(slot_id)?;
        self.ensure_ready()?;
        let mut last_error = None;
        for backup in 0..=BACKUP_COUNT {
            let path = if backup == 0 {
                self.primary_path(slot_id)
            } else {
                self.backup_path(slot_id, backup)
            };
            if !path.exists() {
                continue;
            }
            match fs::read(&path)
                .map_err(|error| DesktopCommandError::io("native-save-read", error))
                .and_then(|bytes| decode_snapshot(&bytes).map(|_| bytes))
            {
                Ok(bytes) => {
                    return Ok(NativeLoadedSave {
                        bytes,
                        recovery_backup: (backup > 0).then_some(backup),
                    });
                }
                Err(error) => last_error = Some(error),
            }
        }
        Err(last_error.unwrap_or_else(|| {
            DesktopCommandError::new("native-save-not-found", "save slot does not exist")
        }))
    }

    pub fn delete(&self, slot_id: &str) -> DesktopResult<()> {
        validate_slot_id(slot_id)?;
        self.ensure_ready()?;
        for backup in 0..=BACKUP_COUNT {
            let path = if backup == 0 {
                self.primary_path(slot_id)
            } else {
                self.backup_path(slot_id, backup)
            };
            if path.exists() {
                fs::remove_file(&path)
                    .map_err(|error| DesktopCommandError::io("native-save-delete", error))?;
            }
        }
        Ok(())
    }

    fn summary(&self, slot_id: &str) -> NativeSaveSummary {
        match self.load(slot_id) {
            Ok(loaded) => match decode_snapshot(&loaded.bytes) {
                Ok((header, snapshot)) => NativeSaveSummary {
                    slot_id: slot_id.to_owned(),
                    slot_name: if header.slot_name.trim().is_empty() {
                        slot_id.to_owned()
                    } else {
                        header.slot_name
                    },
                    status: if loaded.recovery_backup.is_some() {
                        NativeSaveStatus::Recoverable
                    } else {
                        NativeSaveStatus::Ready
                    },
                    recovery_backup: loaded.recovery_backup,
                    saved_at: Some(header.saved_at),
                    created_at: Some(header.created_at),
                    turn: Some(snapshot.turn),
                    location_key: Some(header.character_summary.location_key),
                    content_id: Some(snapshot.content_id),
                    content_hash: Some(snapshot.content_hash),
                    state_hash: Some(snapshot.state_hash),
                },
                Err(_) => corrupt_summary(slot_id),
            },
            Err(_) => corrupt_summary(slot_id),
        }
    }

    fn rotate_backups(&self, slot_id: &str) -> DesktopResult<()> {
        for backup in (2..=BACKUP_COUNT).rev() {
            let source = self.backup_path(slot_id, backup - 1);
            let destination = self.backup_path(slot_id, backup);
            if destination.exists() {
                fs::remove_file(&destination)
                    .map_err(|error| DesktopCommandError::io("native-save-backup-remove", error))?;
            }
            if source.exists() {
                fs::rename(&source, &destination)
                    .map_err(|error| DesktopCommandError::io("native-save-backup-rotate", error))?;
            }
        }
        let primary = self.primary_path(slot_id);
        let first_backup = self.backup_path(slot_id, 1);
        if first_backup.exists() {
            fs::remove_file(&first_backup)
                .map_err(|error| DesktopCommandError::io("native-save-backup-remove", error))?;
        }
        if primary.exists() {
            fs::rename(&primary, &first_backup)
                .map_err(|error| DesktopCommandError::io("native-save-backup-create", error))?;
        }
        Ok(())
    }

    fn cleanup_stale_temps(&self) -> DesktopResult<()> {
        let entries = fs::read_dir(&self.root)
            .map_err(|error| DesktopCommandError::io("native-save-temp-list", error))?;
        for entry in entries {
            let entry =
                entry.map_err(|error| DesktopCommandError::io("native-save-temp-list", error))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.') && name.ends_with(".tmp") {
                fs::remove_file(entry.path())
                    .map_err(|error| DesktopCommandError::io("native-save-temp-clean", error))?;
            }
        }
        Ok(())
    }

    fn primary_path(&self, slot_id: &str) -> PathBuf {
        self.root.join(format!("{slot_id}{SAVE_EXTENSION}"))
    }

    fn backup_path(&self, slot_id: &str, backup: u8) -> PathBuf {
        self.root
            .join(format!("{slot_id}{SAVE_EXTENSION}.bak{backup}"))
    }

    fn temporary_path(&self, slot_id: &str) -> DesktopResult<PathBuf> {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| DesktopCommandError::new("native-save-clock", error.to_string()))?
            .as_nanos();
        Ok(self
            .root
            .join(format!(".{slot_id}{SAVE_EXTENSION}.{nonce}.tmp")))
    }
}

pub fn validate_slot_name(name: &str) -> DesktopResult<String> {
    let trimmed = name.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > 80 || trimmed.chars().any(char::is_control) {
        return Err(DesktopCommandError::new(
            "native-save-name-invalid",
            "save name must contain 1 to 80 visible characters",
        ));
    }
    Ok(trimmed.to_owned())
}

pub fn append_log(path: &Path, event: &str, detail: &str) {
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let sanitized = detail.replace(['\r', '\n'], " ");
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{timestamp} {event} {sanitized}");
    }
}

fn validate_slot_id(slot_id: &str) -> DesktopResult<()> {
    if slot_id.is_empty()
        || slot_id.len() > 80
        || !slot_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(DesktopCommandError::new(
            "native-save-id-invalid",
            "save slot id is invalid",
        ));
    }
    Ok(())
}

fn slot_id_from_file_name(name: &str) -> Option<String> {
    if let Some(slot_id) = name.strip_suffix(SAVE_EXTENSION) {
        return Some(slot_id.to_owned());
    }
    let (slot_id, backup) = name.split_once(&format!("{SAVE_EXTENSION}.bak"))?;
    if backup
        .parse::<u8>()
        .ok()
        .is_some_and(|value| value <= BACKUP_COUNT)
    {
        Some(slot_id.to_owned())
    } else {
        None
    }
}

fn decode_snapshot(bytes: &[u8]) -> DesktopResult<(SaveHeaderV1, GameSnapshot)> {
    let (header, payload) = rfb_save::decode(bytes)
        .map_err(|error| DesktopCommandError::new("native-save-invalid", error.to_string()))?;
    let game = Game::from_save(payload)
        .map_err(|error| DesktopCommandError::new("native-save-invalid", error.to_string()))?;
    Ok((header, game.snapshot()))
}

fn corrupt_summary(slot_id: &str) -> NativeSaveSummary {
    NativeSaveSummary {
        slot_id: slot_id.to_owned(),
        slot_name: slot_id.to_owned(),
        status: NativeSaveStatus::Corrupt,
        recovery_backup: None,
        saved_at: None,
        created_at: None,
        turn: None,
        location_key: None,
        content_id: None,
        content_hash: None,
        state_hash: None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use rfb_protocol::{CharacterSummary, PROTOCOL_VERSION};

    use super::*;

    fn temporary_directory() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be valid")
            .as_nanos();
        std::env::temp_dir().join(format!("rfb-native-save-{nonce}"))
    }

    fn encoded_game(game: &Game, slot_name: &str, saved_at: &str) -> Vec<u8> {
        let snapshot = game.snapshot();
        let header = SaveHeaderV1 {
            format: "rfb-save".to_owned(),
            save_schema_version: 1,
            game_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            slot_name: slot_name.to_owned(),
            created_at: "2026-07-16T00:00:00Z".to_owned(),
            saved_at: saved_at.to_owned(),
            character_summary: CharacterSummary {
                display_name: "测试探索者".to_owned(),
                level: 1,
                location_key: game.location_key().to_owned(),
                turn: snapshot.turn,
            },
            content_id: snapshot.content_id,
            content_hash: snapshot.content_hash,
            payload_encoding: "messagepack".to_owned(),
        };
        rfb_save::encode(&header, &game.to_save()).expect("test save should encode")
    }

    #[test]
    fn native_store_rotates_and_recovers_backups() {
        let directory = temporary_directory();
        let store = NativeSaveStore::new(directory.clone());
        let initial = Game::new(42);
        let first_hash = initial.state_hash();
        store
            .write(
                "save-test",
                &encoded_game(&initial, "第一份存档", "2026-07-16T00:01:00Z"),
            )
            .expect("first save should succeed");

        let mut moved = initial.clone();
        moved
            .dispatch(rfb_protocol::GameCommandEnvelope {
                command_seq: 1,
                expected_revision: 0,
                command: rfb_protocol::GameCommand::Move {
                    direction: rfb_protocol::Direction::East,
                },
            })
            .expect("movement should execute");
        store
            .write(
                "save-test",
                &encoded_game(&moved, "第二份存档", "2026-07-16T00:02:00Z"),
            )
            .expect("overwrite should succeed");
        fs::write(store.primary_path("save-test"), b"corrupt")
            .expect("test should corrupt the primary save");

        let loaded = store.load("save-test").expect("backup should recover");
        assert_eq!(loaded.recovery_backup, Some(1));
        let (_, snapshot) = decode_snapshot(&loaded.bytes).expect("backup should decode");
        assert_eq!(snapshot.state_hash, first_hash);
        let summary = store.list().expect("list should succeed").remove(0);
        assert_eq!(summary.status, NativeSaveStatus::Recoverable);
        assert_eq!(summary.recovery_backup, Some(1));

        store.delete("save-test").expect("delete should succeed");
        assert!(store.list().expect("list should succeed").is_empty());
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn native_store_rejects_unsafe_ids_and_names() {
        assert!(validate_slot_id("../escape").is_err());
        assert!(validate_slot_name("\n").is_err());
        assert_eq!(validate_slot_name("  测试存档  ").unwrap(), "测试存档");
    }
}
