// SPDX-License-Identifier: MPL-2.0

use super::*;
use rfb_protocol::Position;
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

const MUSEUM: &str = "demo.town-facility.outpost-museum";

fn fixture(root: &Path, name: &str) -> AppState {
    let state = AppState::new(root.to_owned());
    state
        .initialize(
            "42",
            "demo.build.warrior",
            "demo.race.rfb-human",
            name,
            "2026-09-09T00:00:00Z".to_owned(),
        )
        .unwrap();
    let mut guard = state.lock_session().unwrap();
    let session = guard.as_mut().unwrap();
    let mut save = session.recorder.game().to_save();
    save.player.position = Position { x: 97, y: 46 };
    save.home_states
        .iter_mut()
        .find(|home| home.facility_id == "demo.town-facility.thalos-museum")
        .unwrap()
        .visited = true;
    session.recorder = ReplayRecorder::new(Game::from_save(save).unwrap());
    session
        .sync_museum(&MuseumStore::open(root).unwrap())
        .unwrap();
    drop(guard);
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
    let snapshot = snapshot(state);
    state.dispatch(snapshot.last_command_seq + 1, snapshot.revision, command)
}

fn deposit(state: &AppState) -> GameCommand {
    let snapshot = snapshot(state);
    let home = snapshot
        .homes
        .iter()
        .find(|home| home.id == MUSEUM)
        .unwrap();
    GameCommand::DepositAtHome {
        facility_id: MUSEUM.to_owned(),
        item_id: home.deposit_items[0].id.clone(),
        quantity: 1,
    }
}

fn withdraw(state: &AppState) -> GameCommand {
    let snapshot = snapshot(state);
    let home = snapshot
        .homes
        .iter()
        .find(|home| home.id == MUSEUM)
        .unwrap();
    GameCommand::WithdrawFromHome {
        facility_id: MUSEUM.to_owned(),
        item_id: home.stored_items[0].id.clone(),
        quantity: 1,
    }
}

fn save(state: &AppState) -> Vec<u8> {
    state.save("2026-09-09T00:01:00Z".to_owned()).unwrap()
}

fn root() -> PathBuf {
    std::env::temp_dir().join(format!("rfb-museum-test-{}", uuid::Uuid::new_v4()))
}

#[test]
fn cross_character_transfer_recovery_and_stale_sessions_preserve_one_owner() {
    let root = root();
    let donor = fixture(&root, "Donor");
    let recipient = fixture(&root, "Recipient");
    let old_donor = save(&donor);
    let command = deposit(&donor);
    let before = snapshot(&donor);
    let update = dispatch(&donor, command.clone()).unwrap();
    assert_eq!(update.events[0].kind, "home.deposit");
    assert!(
        donor
            .dispatch(before.last_command_seq + 1, before.revision, command)
            .is_err()
    );
    assert_eq!(snapshot(&donor).state_hash, update.state_hash);
    assert!(
        dispatch(&recipient, deposit(&recipient))
            .unwrap_err()
            .starts_with("museum-collection-stale")
    );
    recipient.refresh_museum().unwrap();
    let old_recipient = save(&recipient);
    let concurrent = AppState::new(root.clone());
    concurrent.load(&old_recipient).unwrap();
    let taken = dispatch(&recipient, withdraw(&recipient)).unwrap();
    assert_eq!(taken.events[0].kind, "home.withdraw");
    assert!(
        concurrent
            .save(String::new())
            .unwrap_err()
            .starts_with("museum-character-stale")
    );
    assert!(
        dispatch(&concurrent, withdraw(&concurrent))
            .unwrap_err()
            .starts_with("museum-character-stale")
    );
    let restored = AppState::new(root.clone());
    let (recovered_donor, recovered) = restored.load_with_recovery(&old_donor).unwrap();
    assert!(recovered);
    assert!(
        recovered_donor
            .homes
            .iter()
            .find(|home| home.id == MUSEUM)
            .unwrap()
            .stored_items
            .is_empty()
    );
    let donor_payload = rfb_save::decode(&save(&restored)).unwrap().1;
    let deposited_kind = &update.events[0].args["target"];
    let before_quantity: u32 = before
        .inventory
        .iter()
        .filter(|item| &item.kind_id == deposited_kind)
        .map(|item| item.quantity)
        .sum();
    let recovered_quantity: u32 = donor_payload
        .inventory
        .iter()
        .filter(|item| &item.kind_id == deposited_kind)
        .map(|item| item.quantity)
        .sum();
    assert_eq!(before_quantity, recovered_quantity + 1);
    let (recovered_recipient, recovered) = restored.load_with_recovery(&old_recipient).unwrap();
    assert!(recovered);
    assert_eq!(recovered_recipient.state_hash, taken.state_hash);
    let profile = MuseumStore::open(&root).unwrap();
    assert!(profile.profile.museum.inventory.is_empty());
    assert_eq!(
        profile
            .profile
            .characters
            .values()
            .filter(|entry| entry.save.is_some())
            .count(),
        2
    );
}

#[test]
fn missing_foreign_or_unbound_profiles_are_not_adopted() {
    let state = fixture(&root(), "Original");
    let bytes = save(&state);
    let foreign = AppState::default();
    assert!(
        foreign
            .load(&bytes)
            .unwrap_err()
            .starts_with("museum-profile-mismatch")
    );
    let (mut header, payload) = rfb_save::decode(&bytes).unwrap();
    header.museum_binding = None;
    let unbound = rfb_save::encode(&header, &payload).unwrap();
    assert!(
        state
            .load(&unbound)
            .unwrap_err()
            .starts_with("museum-unbound-save")
    );
    let profile = state.profile_root.join("museum.json");
    let archived = state.profile_root.join("museum.saved");
    fs::rename(&profile, &archived).unwrap();
    assert!(
        state
            .load(&bytes)
            .unwrap_err()
            .starts_with("museum-profile-mismatch")
    );
    assert!(!profile.exists());
    fs::write(&profile, b"corrupt profile").unwrap();
    assert!(
        state
            .load(&bytes)
            .unwrap_err()
            .starts_with("museum-corrupt")
    );
    assert_eq!(fs::read(&profile).unwrap(), b"corrupt profile");
}

#[test]
fn new_character_imports_the_profile_at_the_outpost_museum() {
    let root = root();
    let donor = fixture(&root, "Donor");
    dispatch(&donor, deposit(&donor)).unwrap();
    let recipient = AppState::new(root);
    let initial = recipient
        .initialize(
            "43",
            "demo.build.warrior",
            "demo.race.rfb-human",
            "Recipient",
            "2026-09-10T00:00:00Z".to_owned(),
        )
        .unwrap();
    let museum = initial.homes.iter().find(|home| home.id == MUSEUM).unwrap();
    assert_eq!(museum.entrance_position, Position { x: 97, y: 46 });
    assert!(!museum.player_at_entrance);
    assert!(museum.stored_items.is_empty());
    let guard = recipient.lock_session().unwrap();
    let session = guard.as_ref().unwrap();
    assert!(session.museum_loaded);
    assert_eq!(
        session
            .recorder
            .game()
            .shared_museum()
            .unwrap()
            .inventory
            .len(),
        1
    );
    assert!(session.recorder.replay_snapshot().commands.is_empty());
    drop(guard);
    assert_eq!(
        recipient.load(&save(&recipient)).unwrap().state_hash,
        initial.state_hash
    );
}

fn child(root: &Path, save_path: &Path, command: &GameCommand) -> Command {
    let mut process = Command::new(std::env::current_exe().unwrap());
    process
        .args([
            "--exact",
            "museum_storage_tests::process_helper",
            "--nocapture",
        ])
        .env("RFB_MUSEUM_TEST_ROOT", root)
        .env("RFB_MUSEUM_TEST_SAVE", save_path)
        .env(
            "RFB_MUSEUM_TEST_COMMAND",
            serde_json::to_string(command).unwrap(),
        )
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    process
}

#[test]
fn process_helper() {
    let Some(root) = std::env::var_os("RFB_MUSEUM_TEST_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    if std::env::var_os("RFB_MUSEUM_TEST_LOCK").is_some() {
        assert!(
            MuseumStore::open(&root)
                .err()
                .unwrap()
                .starts_with("museum-busy")
        );
        return;
    }
    let state = AppState::new(root);
    let result = (|| {
        state.load(&fs::read(std::env::var_os("RFB_MUSEUM_TEST_SAVE").unwrap()).unwrap())?;
        let command =
            serde_json::from_str(&std::env::var("RFB_MUSEUM_TEST_COMMAND").unwrap()).unwrap();
        dispatch(&state, command)
    })();
    let transferred = result.is_ok_and(|update| {
        update
            .events
            .iter()
            .any(|event| event.kind == "home.deposit" || event.kind == "home.withdraw")
    });
    std::process::exit(if transferred { 0 } else { 78 });
}

#[test]
fn process_interruptions_before_and_after_commit_recover_both_transfer_directions() {
    for withdrawal in [false, true] {
        for point in ["before-replace", "after-replace"] {
            let root = root();
            let state = fixture(&root, "Crash recovery");
            if withdrawal {
                dispatch(&state, deposit(&state)).unwrap();
            }
            let before = snapshot(&state);
            let bytes = save(&state);
            let saved = root.join("before.rfbsave");
            fs::write(&saved, &bytes).unwrap();
            let command = if withdrawal {
                withdraw(&state)
            } else {
                deposit(&state)
            };
            let output = child(&root, &saved, &command)
                .env("RFB_MUSEUM_TEST_CRASH", point)
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(77),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let restarted = AppState::new(root.clone());
            let (after, recovered) = restarted.load_with_recovery(&bytes).unwrap();
            let committed = point == "after-replace";
            assert_eq!(recovered, committed);
            if !committed {
                assert_eq!(before.state_hash, after.state_hash);
            }
            let store = MuseumStore::open(&root).unwrap();
            let expected_count = usize::from(withdrawal != committed);
            assert_eq!(store.profile.museum.inventory.len(), expected_count);
            if committed {
                let id = rfb_save::decode(&bytes)
                    .unwrap()
                    .0
                    .museum_binding
                    .unwrap()
                    .character_id;
                let checkpoint = store.checkpoint(id).unwrap();
                let summary = native_storage::museum_checkpoint_summary(id, &checkpoint).unwrap();
                assert!(summary.museum_checkpoint);
                assert_eq!(summary.turn, Some(after.turn));
                drop(store);
                assert_eq!(
                    restarted.load(&checkpoint).unwrap().state_hash,
                    after.state_hash
                );
            }
        }
    }
}

#[test]
fn file_lock_and_competing_processes_allow_only_one_withdrawal() {
    let root = root();
    let donor = fixture(&root, "Donor");
    dispatch(&donor, deposit(&donor)).unwrap();
    let first = fixture(&root, "First");
    let second = fixture(&root, "Second");
    let first_file = root.join("first.rfbsave");
    let second_file = root.join("second.rfbsave");
    fs::write(&first_file, save(&first)).unwrap();
    fs::write(&second_file, save(&second)).unwrap();
    let lock = MuseumStore::open(&root).unwrap();
    let output = child(&root, &first_file, &withdraw(&first))
        .env("RFB_MUSEUM_TEST_LOCK", "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    drop(lock);
    let mut a = child(&root, &first_file, &withdraw(&first))
        .spawn()
        .unwrap();
    let mut b = child(&root, &second_file, &withdraw(&second))
        .spawn()
        .unwrap();
    let codes = [a.wait().unwrap().code(), b.wait().unwrap().code()];
    assert_eq!(
        codes.iter().filter(|code| **code == Some(0)).count(),
        1,
        "{codes:?}"
    );
    assert_eq!(
        codes.iter().filter(|code| **code == Some(78)).count(),
        1,
        "{codes:?}"
    );
    assert!(
        MuseumStore::open(&root)
            .unwrap()
            .profile
            .museum
            .inventory
            .is_empty()
    );
}
