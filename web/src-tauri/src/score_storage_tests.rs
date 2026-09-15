// SPDX-License-Identifier: MPL-2.0
use super::*;
use score_storage::ScoreOutcome;

fn fresh(root: &std::path::Path) -> AppState {
    let state = AppState::new(root.to_owned());
    let snapshot = state
        .initialize(
            "42",
            "demo.build.warrior",
            "demo.race.rfb-human",
            "Same name",
            "2026-09-14T00:00:00Z".into(),
            Game::default_behavior_preferences(),
        )
        .unwrap();
    if let Some(choice) = snapshot.player.pending_race_mutation_choice {
        state
            .dispatch(
                snapshot.last_command_seq + 1,
                snapshot.revision,
                GameCommand::ChooseRaceMutation {
                    reward_id: choice.reward_id,
                    mutation_id: choice.candidates[0].id.clone(),
                },
            )
            .unwrap();
    }
    state
}
fn snapshot(state: &AppState) -> GameSnapshot {
    state
        .lock_session()
        .unwrap()
        .as_ref()
        .unwrap()
        .recorder
        .game()
        .snapshot()
}
fn dispatch(state: &AppState, command: GameCommand) -> Result<GameUpdate, String> {
    let s = snapshot(state);
    state.dispatch(s.last_command_seq + 1, s.revision, command)
}

#[test]
fn ending_commits_once_and_old_saves_recover_the_final_character_across_sessions() {
    let root = std::env::temp_dir().join(format!("rfb-score-test-{}", uuid::Uuid::new_v4()));
    let first = fresh(&root);
    assert!(first.scores().unwrap().is_empty());
    let old = first.save(String::new()).unwrap();
    let stale = AppState::new(root.clone());
    stale.load(&old).unwrap();
    let final_update = dispatch(&first, GameCommand::EndCharacter).unwrap();
    let replay = rfb_replay::decode(&first.export_replay().unwrap()).unwrap();
    rfb_replay::verify(
        &replay,
        initial_game(
            42,
            "demo.build.warrior",
            "demo.race.rfb-human",
            "Same name",
            rfb_core::Game::default_behavior_preferences(),
        )
        .unwrap(),
    )
    .expect("ending command must replay deterministically");
    assert_eq!(first.scores().unwrap()[0].outcome, ScoreOutcome::Abandoned);
    assert!(dispatch(&stale, GameCommand::EndCharacter).is_err());
    let restarted = AppState::new(root.clone());
    let (recovered, recovery) = restarted
        .load_with_recovery(&old, Game::default_behavior_preferences())
        .unwrap();
    assert!(recovery);
    assert_eq!(recovered.state_hash, final_update.state_hash);
    assert!(dispatch(&restarted, GameCommand::Wait).is_err());
    let second = fresh(&root);
    dispatch(&second, GameCommand::EndCharacter).unwrap();
    let scores = restarted.scores().unwrap();
    assert_eq!(scores.len(), 2);
    assert_ne!(scores[0].character_id, scores[1].character_id);
    assert_eq!(scores[0].name, scores[1].name);
    assert_eq!(scores[0].score, final_update.campaign.score);
    assert_eq!(scores[0].turn, final_update.turn);
}

#[test]
fn failed_final_commit_leaves_role_and_profile_unchanged_then_can_retry() {
    let root = std::env::temp_dir().join(format!("rfb-score-test-{}", uuid::Uuid::new_v4()));
    let state = fresh(&root);
    let before = snapshot(&state).state_hash;
    std::fs::create_dir(root.join("museum.pending")).unwrap();
    assert!(dispatch(&state, GameCommand::EndCharacter).is_err());
    assert_eq!(snapshot(&state).state_hash, before);
    assert!(state.scores().unwrap().is_empty());
    std::fs::remove_dir(root.join("museum.pending")).unwrap();
    dispatch(&state, GameCommand::EndCharacter).unwrap();
    assert_eq!(state.scores().unwrap().len(), 1);
}

#[test]
fn death_is_recorded_by_the_dispatcher_and_corruption_is_reported() {
    let root = std::env::temp_dir().join(format!("rfb-score-test-{}", uuid::Uuid::new_v4()));
    let state = fresh(&root);
    {
        let mut guard = state.lock_session().unwrap();
        let session = guard.as_mut().unwrap();
        let mut save = session.recorder.game().to_save();
        save.player.hp = 1;
        save.player.nutrition = 0;
        session.recorder = ReplayRecorder::new(
            Game::from_save(save, Game::default_behavior_preferences()).unwrap(),
        );
    }
    for _ in 0..20 {
        if dispatch(&state, GameCommand::Wait).unwrap().player.is_dead {
            break;
        }
    }
    let scores = state.scores().unwrap();
    assert_eq!(scores.len(), 1);
    assert_eq!(scores[0].outcome, ScoreOutcome::Dead);
    std::fs::write(root.join("museum.json"), b"bad json").unwrap();
    assert!(state.scores().unwrap_err().starts_with("museum-corrupt"));
}
