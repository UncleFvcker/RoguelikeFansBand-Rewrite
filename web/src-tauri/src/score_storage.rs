// SPDX-License-Identifier: MPL-2.0
use rfb_protocol::{CampaignStatusDto, GameUpdate};
use serde::{Deserialize, Serialize};

use crate::{AppState, GameSession, museum_storage::MuseumStore};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ScoreRecord {
    pub character_id: String,
    pub name: String,
    pub race_name_key: Option<String>,
    pub class_name_key: Option<String>,
    pub level: u16,
    pub score: u64,
    pub turn: u32,
    pub outcome: ScoreOutcome,
    pub location_key: String,
    pub ended_at: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ScoreOutcome {
    Dead,
    Retired,
    Abandoned,
}

impl ScoreOutcome {
    pub fn from_update(update: &GameUpdate) -> Option<Self> {
        match update.campaign.status {
            CampaignStatusDto::Retired => Some(Self::Retired),
            CampaignStatusDto::Abandoned => Some(Self::Abandoned),
            _ if update.player.is_dead => Some(Self::Dead),
            _ => None,
        }
    }
}

impl GameSession {
    pub(super) fn record_score(
        &mut self,
        store: &mut MuseumStore,
        update: &GameUpdate,
        outcome: ScoreOutcome,
    ) -> Result<(), String> {
        store.ensure_current(&self.binding)?;
        if store.character(&self.binding)?.score.is_some() {
            return Err("character-already-ended: reload the final checkpoint".into());
        }
        let game = self.recorder.game();
        rfb_core::Game::from_save(game.to_save(), game.behavior_preferences())
            .map_err(|e| e.to_string())?;
        let score = ScoreRecord {
            character_id: self.binding.character_id.to_string(),
            name: update.player.name.clone(),
            race_name_key: update
                .player
                .build
                .as_ref()
                .map(|build| build.race_name_key.clone()),
            class_name_key: update
                .player
                .build
                .as_ref()
                .map(|build| build.class_name_key.clone()),
            level: update.player.progress.level,
            score: update.campaign.score,
            turn: update.turn,
            outcome,
            location_key: game.location_key().to_owned(),
            ended_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_secs(),
        };
        self.binding.epoch = self
            .binding
            .epoch
            .checked_add(1)
            .ok_or("museum-epoch-exhausted")?;
        let checkpoint = self.encode(String::new(), score.name.clone())?;
        let character = store
            .profile
            .characters
            .get_mut(&self.binding.character_id)
            .unwrap();
        character.epoch = self.binding.epoch;
        character.save = Some(checkpoint);
        character.score = Some(score);
        // Score and final checkpoint share one atomic commit. The live session is
        // installed only after this succeeds, so a failed write cannot end a role.
        store.commit()
    }
}

impl AppState {
    pub(super) fn scores(&self) -> Result<Vec<ScoreRecord>, String> {
        let store = MuseumStore::open(&self.profile_root)?;
        let mut records: Vec<_> = store
            .profile
            .characters
            .values()
            .filter_map(|c| c.score.clone())
            .collect();
        records.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.turn.cmp(&b.turn))
                .then(a.ended_at.cmp(&b.ended_at))
                .then(a.character_id.cmp(&b.character_id))
        });
        Ok(records)
    }
}

#[tauri::command]
pub(crate) fn list_high_scores(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ScoreRecord>, String> {
    state.scores()
}
