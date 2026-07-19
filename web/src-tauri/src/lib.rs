// SPDX-License-Identifier: MPL-2.0

#[cfg(all(feature = "webdriver", not(debug_assertions)))]
compile_error!("the webdriver feature is restricted to debug-only E2E builds");

use std::sync::Mutex;

use serde::Serialize;
use tauri::Manager;

use rfb_core::Game;
use rfb_protocol::{
    CharacterSummary, GameCommand, GameCommandEnvelope, GameSnapshot, GameUpdate, PROTOCOL_VERSION,
    SaveHeaderV1,
};
use rfb_replay::ReplayRecorder;

mod crash_diagnostics;
mod native_storage;

use crash_diagnostics::{
    CrashDiagnosticStatus, CrashDiagnostics, DiagnosticMetadata, install_log_only_panic_hook,
};
use native_storage::{
    DesktopCommandError, DesktopResult, NativeSaveStore, NativeSaveSummary, append_log,
    validate_slot_name,
};

struct GameSession {
    recorder: ReplayRecorder,
    created_at: String,
}

#[derive(Default)]
struct AppState {
    session: Mutex<Option<GameSession>>,
    storage: Mutex<()>,
}

impl AppState {
    fn initialize(&self, seed: &str, created_at: String) -> Result<GameSnapshot, String> {
        let seed = seed
            .parse::<u64>()
            .map_err(|error| format!("invalid seed: {error}"))?;
        let recorder = ReplayRecorder::new(initial_game(seed));
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
        self.save_named(saved_at, String::new())
    }

    fn save_named(&self, saved_at: String, slot_name: String) -> Result<Vec<u8>, String> {
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
            slot_name,
            created_at: session.created_at.clone(),
            saved_at,
            character_summary: CharacterSummary {
                display_name: "原创测试探索者".to_owned(),
                level: 1,
                location_key: session.recorder.game().location_key().to_owned(),
                turn: snapshot.turn,
            },
            content_id: snapshot.content_id,
            content_hash: snapshot.content_hash,
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

    fn lock_storage(&self) -> DesktopResult<std::sync::MutexGuard<'_, ()>> {
        self.storage
            .lock()
            .map_err(|_| DesktopCommandError::new("native-save-lock", "storage lock is poisoned"))
    }
}

#[cfg(not(feature = "webdriver"))]
fn initial_game(seed: u64) -> Game {
    Game::new(seed)
}

#[cfg(feature = "webdriver")]
fn initial_game(seed: u64) -> Game {
    let mut payload = Game::new(seed).to_save();
    payload.entities.clear();
    Game::from_save(payload)
        .expect("webdriver fixture should remove monsters without invalid state")
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeLoadResult {
    snapshot: GameSnapshot,
    recovery_backup: Option<u8>,
}

fn native_store(app: &tauri::AppHandle) -> DesktopResult<NativeSaveStore> {
    let root = app
        .path()
        .app_local_data_dir()
        .map_err(|error| DesktopCommandError::new("native-save-directory", error.to_string()))?
        .join("saves");
    Ok(NativeSaveStore::new(root))
}

fn desktop_log_path(app: &tauri::AppHandle) -> DesktopResult<std::path::PathBuf> {
    #[cfg(feature = "webdriver")]
    if let Some(path) = std::env::var_os("RFB_E2E_LOG_PATH") {
        return Ok(path.into());
    }
    app.path()
        .app_log_dir()
        .map(|directory| directory.join("rfb-desktop.log"))
        .map_err(|error| DesktopCommandError::new("desktop-log-directory", error.to_string()))
}

fn crash_diagnostic_root(app: &tauri::AppHandle) -> DesktopResult<std::path::PathBuf> {
    #[cfg(feature = "webdriver")]
    if let Some(path) = std::env::var_os("RFB_E2E_DIAGNOSTIC_ROOT") {
        return Ok(path.into());
    }
    app.path()
        .app_log_dir()
        .map(|directory| directory.join("diagnostics"))
        .map_err(|error| DesktopCommandError::new("crash-diagnostic-directory", error.to_string()))
}

fn log_event(app: &tauri::AppHandle, event: &str, detail: &str) {
    if let Ok(path) = desktop_log_path(app) {
        append_log(&path, event, detail);
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

#[tauri::command]
fn crash_diagnostic_status(
    diagnostics: tauri::State<'_, CrashDiagnostics>,
) -> CrashDiagnosticStatus {
    diagnostics.status()
}

#[tauri::command(rename_all = "camelCase")]
fn update_crash_diagnostic_context(
    diagnostics: tauri::State<'_, CrashDiagnostics>,
    content_id: String,
    content_hash: String,
    renderer_backend: String,
) -> DesktopResult<()> {
    diagnostics.update_context(&content_id, &content_hash, &renderer_backend)
}

#[tauri::command(rename_all = "camelCase")]
fn record_frontend_crash(
    diagnostics: tauri::State<'_, CrashDiagnostics>,
    kind: String,
) -> DesktopResult<CrashDiagnosticStatus> {
    diagnostics.record_frontend_error(&kind)
}

#[tauri::command]
fn list_native_saves(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> DesktopResult<Vec<NativeSaveSummary>> {
    let _storage = state.lock_storage()?;
    let result = native_store(&app)?.list();
    if let Err(error) = &result {
        log_event(&app, "native-save-list-error", &error.code);
    }
    result
}

#[tauri::command(rename_all = "camelCase")]
fn save_native_game(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    slot_id: Option<String>,
    slot_name: String,
    saved_at: String,
) -> DesktopResult<NativeSaveSummary> {
    let result: DesktopResult<NativeSaveSummary> = (|| {
        let slot_name = validate_slot_name(&slot_name)?;
        let _storage = state.lock_storage()?;
        let store = native_store(&app)?;
        let slot_id = slot_id.map_or_else(|| store.create_slot_id(), Ok)?;
        let bytes = state
            .save_named(saved_at, slot_name)
            .map_err(|error| DesktopCommandError::new("native-save-encode", error))?;
        let summary = store.write(&slot_id, &bytes)?;
        log_event(&app, "native-save-written", &slot_id);
        Ok(summary)
    })();
    if let Err(error) = &result {
        log_event(&app, "native-save-write-error", &error.code);
    }
    result
}

#[tauri::command]
fn load_native_game(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    slot_id: String,
) -> DesktopResult<NativeLoadResult> {
    let result: DesktopResult<NativeLoadResult> = (|| {
        let _storage = state.lock_storage()?;
        let loaded = native_store(&app)?.load(&slot_id)?;
        let snapshot = state
            .load(&loaded.bytes)
            .map_err(|error| DesktopCommandError::new("native-save-load", error))?;
        log_event(
            &app,
            if loaded.recovery_backup.is_some() {
                "native-save-backup-loaded"
            } else {
                "native-save-loaded"
            },
            &slot_id,
        );
        Ok(NativeLoadResult {
            snapshot,
            recovery_backup: loaded.recovery_backup,
        })
    })();
    if let Err(error) = &result {
        log_event(&app, "native-save-load-error", &error.code);
    }
    result
}

#[tauri::command]
fn delete_native_save(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    slot_id: String,
) -> DesktopResult<()> {
    let result: DesktopResult<()> = (|| {
        let _storage = state.lock_storage()?;
        native_store(&app)?.delete(&slot_id)?;
        log_event(&app, "native-save-deleted", &slot_id);
        Ok(())
    })();
    if let Err(error) = &result {
        log_event(&app, "native-save-delete-error", &error.code);
    }
    result
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(feature = "webdriver")]
    let builder = if std::env::var_os("TAURI_WEBDRIVER_PORT").is_some() {
        builder.plugin(tauri_plugin_wdio_webdriver::init())
    } else {
        builder
    };

    builder
        .setup(|app| {
            let store = native_store(app.handle()).map_err(|error| {
                std::io::Error::other(format!("{}: {}", error.code, error.detail))
            })?;
            store.ensure_ready().map_err(|error| {
                std::io::Error::other(format!("{}: {}", error.code, error.detail))
            })?;
            let log_path = desktop_log_path(app.handle()).map_err(|error| {
                std::io::Error::other(format!("{}: {}", error.code, error.detail))
            })?;
            let metadata = DiagnosticMetadata {
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                protocol_version: PROTOCOL_VERSION.to_owned(),
                operating_system: std::env::consts::OS.to_owned(),
                architecture: std::env::consts::ARCH.to_owned(),
            };
            let diagnostics = crash_diagnostic_root(app.handle())
                .and_then(|root| CrashDiagnostics::begin(root, log_path.clone(), metadata));
            match diagnostics {
                Ok(diagnostics) => {
                    diagnostics.install_panic_hook();
                    app.manage(diagnostics);
                }
                Err(error) => {
                    append_log(&log_path, "crash-diagnostic-disabled", &error.code);
                    install_log_only_panic_hook(log_path.clone());
                }
            }
            append_log(&log_path, "desktop-start", env!("CARGO_PKG_VERSION"));
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            initialize_game,
            dispatch_game_command,
            save_game,
            load_game,
            export_replay,
            crash_diagnostic_status,
            update_crash_diagnostic_context,
            record_frontend_crash,
            list_native_saves,
            save_native_game,
            load_native_game,
            delete_native_save
        ])
        .build(tauri::generate_context!())
        .expect("failed to build RoguelikeFansBand Rewrite")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit)
                && let Some(diagnostics) = app.try_state::<CrashDiagnostics>()
            {
                diagnostics.mark_clean_exit();
            }
        });
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
            verify_replay(&replay, initial_game(42)).expect("exported replay should verify");
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
