// SPDX-License-Identifier: MPL-2.0

use std::sync::Mutex;

use rfb_core::Game;
use rfb_protocol::{
    CharacterSummary, DEMO_CONTENT_HASH, DEMO_CONTENT_ID, GameCommand, GameCommandEnvelope,
    GameSnapshot, GameUpdate, PROTOCOL_VERSION, SaveHeaderV1,
};
use rfb_replay::ReplayRecorder;

struct GameSession {
    recorder: ReplayRecorder,
    created_at: String,
}

#[derive(Default)]
struct AppState {
    session: Mutex<Option<GameSession>>,
}

impl AppState {
    fn initialize(&self, seed: &str, created_at: String) -> Result<GameSnapshot, String> {
        let seed = seed
            .parse::<u64>()
            .map_err(|error| format!("invalid seed: {error}"))?;
        let recorder = ReplayRecorder::new(Game::new(seed));
        let snapshot = recorder.game().snapshot();
        self.replace_session(GameSession {
            recorder,
            created_at,
        })?;
        Ok(snapshot)
    }

    fn dispatch(
        &self,
        command_seq: u32,
        expected_revision: u32,
        command: GameCommand,
    ) -> Result<GameUpdate, String> {
        let mut session = self.lock_session()?;
        session
            .as_mut()
            .ok_or_else(|| "game session is not initialized".to_owned())?
            .recorder
            .dispatch_envelope(GameCommandEnvelope {
                command_seq,
                expected_revision,
                command,
            })
            .map_err(|error| error.to_string())
    }

    fn save(&self, saved_at: String) -> Result<Vec<u8>, String> {
        let session = self.lock_session()?;
        let session = session
            .as_ref()
            .ok_or_else(|| "game session is not initialized".to_owned())?;
        let snapshot = session.recorder.game().snapshot();
        let header = SaveHeaderV1 {
            format: "rfb-save".to_owned(),
            save_schema_version: 1,
            game_version: env!("CARGO_PKG_VERSION").to_owned(),
            protocol_version: PROTOCOL_VERSION.to_owned(),
            created_at: session.created_at.clone(),
            saved_at,
            character_summary: CharacterSummary {
                display_name: "原创测试探索者".to_owned(),
                level: 1,
                location_key: "location-demo-lab".to_owned(),
                turn: snapshot.turn,
            },
            content_id: DEMO_CONTENT_ID.to_owned(),
            content_hash: DEMO_CONTENT_HASH.to_owned(),
            payload_encoding: "messagepack".to_owned(),
        };
        rfb_save::encode(&header, &session.recorder.game().to_save())
            .map_err(|error| error.to_string())
    }

    fn load(&self, data: &[u8]) -> Result<GameSnapshot, String> {
        let (header, payload) = rfb_save::decode(data).map_err(|error| error.to_string())?;
        let game = Game::from_save(payload).map_err(|error| error.to_string())?;
        let snapshot = game.snapshot();
        self.replace_session(GameSession {
            recorder: ReplayRecorder::new(game),
            created_at: header.created_at,
        })?;
        Ok(snapshot)
    }

    fn export_replay(&self) -> Result<Vec<u8>, String> {
        let session = self.lock_session()?;
        let session = session
            .as_ref()
            .ok_or_else(|| "game session is not initialized".to_owned())?;
        rfb_replay::encode(&session.recorder.replay_snapshot()).map_err(|error| error.to_string())
    }

    fn lock_session(&self) -> Result<std::sync::MutexGuard<'_, Option<GameSession>>, String> {
        self.session
            .lock()
            .map_err(|_| "game session lock is poisoned".to_owned())
    }

    fn replace_session(&self, session: GameSession) -> Result<(), String> {
        *self.lock_session()? = Some(session);
        Ok(())
    }
}

#[tauri::command(rename_all = "camelCase")]
fn initialize_game(
    state: tauri::State<'_, AppState>,
    seed: String,
    created_at: String,
) -> Result<GameSnapshot, String> {
    state.initialize(&seed, created_at)
}

#[tauri::command(rename_all = "camelCase")]
fn dispatch_game_command(
    state: tauri::State<'_, AppState>,
    command_seq: u32,
    expected_revision: u32,
    command: GameCommand,
) -> Result<GameUpdate, String> {
    state.dispatch(command_seq, expected_revision, command)
}

#[tauri::command(rename_all = "camelCase")]
fn save_game(state: tauri::State<'_, AppState>, saved_at: String) -> Result<Vec<u8>, String> {
    state.save(saved_at)
}

#[tauri::command]
fn load_game(state: tauri::State<'_, AppState>, data: Vec<u8>) -> Result<GameSnapshot, String> {
    state.load(&data)
}

#[tauri::command]
fn export_replay(state: tauri::State<'_, AppState>) -> Result<Vec<u8>, String> {
    state.export_replay()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            initialize_game,
            dispatch_game_command,
            save_game,
            load_game,
            export_replay
        ])
        .run(tauri::generate_context!())
        .expect("failed to run RoguelikeFansBand Rewrite");
}

#[cfg(test)]
mod tests {
    use rfb_protocol::{Direction, GameCommand};
    use rfb_replay::{decode as decode_replay, verify as verify_replay};

    use super::*;

    #[test]
    fn native_session_moves_saves_and_restores() {
        let state = AppState::default();
        let initial = state
            .initialize("42", "2026-07-15T00:00:00Z".to_owned())
            .expect("session should initialize");
        let update = state
            .dispatch(
                1,
                initial.revision,
                GameCommand::Move {
                    direction: Direction::East,
                },
            )
            .expect("move should execute");
        let bytes = state
            .save("2026-07-15T00:01:00Z".to_owned())
            .expect("save should encode");
        let replay = decode_replay(&state.export_replay().expect("replay should encode"))
            .expect("replay should decode");
        let verification =
            verify_replay(&replay, Game::new(42)).expect("exported replay should verify");
        let restored = AppState::default()
            .load(&bytes)
            .expect("save should restore in a new native session");

        assert_eq!(verification.commands_verified, 1);
        assert_eq!(verification.final_state_hash, update.state_hash);
        assert_eq!(restored.revision, update.revision);
        assert_eq!(restored.last_command_seq, update.command_seq);
        assert_eq!(restored.state_hash, update.state_hash);
    }

    #[test]
    fn rejected_native_command_is_not_recorded() {
        let state = AppState::default();
        let initial = state
            .initialize("42", "2026-07-15T00:00:00Z".to_owned())
            .expect("session should initialize");

        state
            .dispatch(1, initial.revision + 1, GameCommand::Wait)
            .expect_err("stale command should fail");
        let replay = decode_replay(&state.export_replay().expect("replay should encode"))
            .expect("replay should decode");

        assert!(replay.commands.is_empty());
        assert!(replay.checkpoints.is_empty());
    }

    #[test]
    fn loading_a_save_starts_a_new_replay_segment() {
        let state = AppState::default();
        let initial = state
            .initialize("42", "2026-07-15T00:00:00Z".to_owned())
            .expect("session should initialize");
        state
            .dispatch(
                1,
                initial.revision,
                GameCommand::Move {
                    direction: Direction::East,
                },
            )
            .expect("move should execute");
        let bytes = state
            .save("2026-07-15T00:01:00Z".to_owned())
            .expect("save should encode");
        let (_, payload) = rfb_save::decode(&bytes).expect("save should decode");
        let replay_start = Game::from_save(payload).expect("save payload should restore");

        let loaded = state.load(&bytes).expect("save should load");
        let update = state
            .dispatch(
                loaded.last_command_seq + 1,
                loaded.revision,
                GameCommand::Wait,
            )
            .expect("command after load should execute");
        let replay = decode_replay(&state.export_replay().expect("replay should encode"))
            .expect("replay should decode");
        let verification =
            verify_replay(&replay, replay_start).expect("loaded replay segment should verify");

        assert_eq!(replay.initial_save_hash, loaded.state_hash);
        assert_eq!(replay.commands.len(), 1);
        assert_eq!(replay.commands[0].turn_before, loaded.turn);
        assert_eq!(verification.final_state_hash, update.state_hash);
    }
}
