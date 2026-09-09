// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use rfb_core::SharedMuseum;
use rfb_protocol::MuseumBindingSaveDto;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct MuseumProfile {
    format_version: u16,
    pub profile_id: String,
    pub revision: u64,
    next_character_id: u64,
    pub museum: SharedMuseum,
    pub characters: BTreeMap<u64, CharacterCheckpoint>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CharacterCheckpoint {
    pub epoch: u64,
    pub save: Option<Vec<u8>>,
}

/// One file is the commit point for both sides of a transfer. The lock file is
/// separate because replacing a locked data file would lock the old inode.
pub(super) struct MuseumStore {
    root: PathBuf,
    _lock: File,
    pub profile: MuseumProfile,
}

impl MuseumStore {
    pub fn open(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("museum.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock().map_err(|e| format!("museum-busy: {e}"))?;
        let profile = match fs::read(root.join("museum.json")) {
            Ok(bytes) => serde_json::from_slice::<MuseumProfile>(&bytes)
                .map_err(|e| format!("museum-corrupt: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => MuseumProfile {
                format_version: 1,
                profile_id: uuid::Uuid::new_v4().to_string(),
                revision: 0,
                next_character_id: 1,
                museum: SharedMuseum::default(),
                characters: BTreeMap::new(),
            },
            Err(e) => return Err(e.to_string()),
        };
        if profile.format_version != 1
            || uuid::Uuid::parse_str(&profile.profile_id).is_err()
            || profile.next_character_id == 0
            || profile
                .characters
                .keys()
                .any(|id| *id == 0 || *id >= profile.next_character_id)
        {
            return Err("museum-corrupt".to_owned());
        }
        Ok(Self {
            root: root.to_owned(),
            _lock: lock,
            profile,
        })
    }

    pub fn new_character(&mut self) -> Result<MuseumBindingSaveDto, String> {
        let character_id = self.profile.next_character_id;
        self.profile.next_character_id =
            character_id.checked_add(1).ok_or("museum-id-exhausted")?;
        self.profile.characters.insert(
            character_id,
            CharacterCheckpoint {
                epoch: 0,
                save: None,
            },
        );
        Ok(MuseumBindingSaveDto {
            profile_id: self.profile.profile_id.clone(),
            character_id,
            epoch: 0,
            collection_revision: self.profile.revision,
        })
    }

    pub fn character(
        &self,
        binding: &MuseumBindingSaveDto,
    ) -> Result<&CharacterCheckpoint, String> {
        if binding.profile_id != self.profile.profile_id {
            return Err("museum-profile-mismatch".to_owned());
        }
        let character = self
            .profile
            .characters
            .get(&binding.character_id)
            .ok_or("museum-character-missing")?;
        if binding.epoch > character.epoch || binding.collection_revision > self.profile.revision {
            return Err("museum-checkpoint-mismatch".to_owned());
        }
        Ok(character)
    }

    pub fn ensure_current(&self, binding: &MuseumBindingSaveDto) -> Result<(), String> {
        if self.character(binding)?.epoch != binding.epoch {
            return Err("museum-character-stale: reload the character checkpoint".to_owned());
        }
        Ok(())
    }

    pub fn checkpoint(&self, character_id: u64) -> Result<Vec<u8>, String> {
        self.profile
            .characters
            .get(&character_id)
            .and_then(|character| character.save.clone())
            .ok_or_else(|| "museum-checkpoint-missing".to_owned())
    }

    pub fn commit(&self) -> Result<(), String> {
        // ponytail: rewrite one local profile per transfer; use a database only
        // if measured collection/checkpoint size makes these writes too slow.
        let bytes = serde_json::to_vec(&self.profile).map_err(|e| e.to_string())?;
        let temporary = self.root.join("museum.pending");
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        #[cfg(test)]
        crash_at("before-replace");
        fs::rename(&temporary, self.root.join("museum.json")).map_err(|e| e.to_string())?;
        #[cfg(test)]
        crash_at("after-replace");
        #[cfg(unix)]
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
fn crash_at(point: &str) {
    if std::env::var("RFB_MUSEUM_TEST_CRASH").as_deref() == Ok(point) {
        std::process::exit(77);
    }
}

pub(super) fn checkpoint_id(slot_id: &str) -> Option<u64> {
    slot_id.strip_prefix("museum-")?.parse().ok()
}
