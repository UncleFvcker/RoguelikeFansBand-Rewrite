// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::fs;

#[test]
fn saving_updates_current_slot_and_new_characters_never_reuse_it() {
    let state = AppState::default();
    let store = NativeSaveStore::new(state.profile_root.join("slots"));
    let create = |name| {
        state
            .initialize(
                "42",
                "demo.build.warrior",
                "demo.race.rfb-human",
                name,
                "2026-09-15T00:00:00Z".into(),
                Game::default_behavior_preferences(),
            )
            .unwrap()
    };
    let save = |name, new_slot| {
        save_native_to_store(
            &state,
            &store,
            None,
            name,
            "2026-09-15T01:00:00Z".into(),
            new_slot,
        )
        .unwrap()
    };
    let first = create("First");
    let a = save(None, false);
    let b = save(None, false);
    assert_eq!(a.slot_id, b.slot_id);
    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(b.state_hash.as_deref(), Some(first.state_hash.as_str()));
    let other = save(Some("Other".into()), true);
    assert_ne!(other.slot_id, a.slot_id);
    assert_eq!(save(None, false).slot_id, other.slot_id);
    let old_binding = state
        .lock_session()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_slot
        .clone();
    let bad_root = state.profile_root.join("not-a-directory");
    fs::write(&bad_root, b"blocked").unwrap();
    assert!(
        save_native_to_store(
            &state,
            &NativeSaveStore::new(bad_root),
            None,
            None,
            String::new(),
            true
        )
        .is_err()
    );
    assert_eq!(
        state.lock_session().unwrap().as_ref().unwrap().native_slot,
        old_binding
    );
    create("Second");
    let second = save(None, false);
    assert_ne!(second.slot_id, a.slot_id);
    assert_ne!(second.slot_id, other.slot_id);
    assert_eq!(second.character_name.as_deref(), Some("Second"));
    assert_eq!(second.character_level, Some(1));
    assert_eq!(store.list().unwrap().len(), 3);
    fs::remove_dir_all(&state.profile_root).unwrap();
}
