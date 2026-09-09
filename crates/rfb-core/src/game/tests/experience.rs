// SPDX-License-Identifier: MPL-2.0
use super::support::*;
use super::*;

#[test]
fn experience_awards_stay_raw_and_racial_cost_changes_the_level_threshold() {
    for (race, factor, level, next) in [
        ("demo.race.rfb-human", 100, 6, 140),
        ("rfb-legacy.race.spectre", 250, 3, 112),
    ] {
        let mut game =
            Game::new_with_build_race_and_name(83, "demo.build.warrior", race, "XP").unwrap();
        let mut events = Vec::new();
        game.apply_player_experience(100, &mut events);
        let snapshot = game.snapshot();
        let progress = snapshot.player.progress;
        assert_eq!(
            (progress.experience, progress.maximum_experience),
            (100, 100)
        );
        assert_eq!(
            (progress.level, progress.experience_for_next_level),
            (level, Some(next))
        );
        assert_eq!(snapshot.player.build.unwrap().experience_percent, factor);
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::ExperienceGained {
                amount: 100,
                total: 100
            }
        )));
        if factor == 250 {
            game.apply_player_experience(11, &mut Vec::new());
            assert_eq!(game.progress.level, 3);
            game.apply_player_experience(1, &mut Vec::new());
            assert_eq!(game.progress.level, 4);
        }
    }
}

#[test]
fn experience_factor_recheck_changes_level_without_rewriting_xp_or_repeating_rewards() {
    let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(380, &mut Vec::new());
    assert_eq!(
        (
            game.progress.level,
            game.progress.max_level,
            game.progress.pending_attribute_increases
        ),
        (10, 10, 2)
    );
    let rng = game.rng.clone();
    // Isolate the shared XP consumer; full permanent-race side effects belong to step three.
    game.build.as_mut().unwrap().race_id = "rfb-legacy.race.spectre".to_owned();
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp() / 2;
    let mut events = Vec::new();
    game.apply_player_experience(0, &mut events);
    assert_eq!(
        (
            game.progress.level,
            game.progress.max_level,
            game.progress.pending_attribute_increases
        ),
        (7, 10, 2)
    );
    assert_eq!(
        (game.progress.experience, game.progress.maximum_experience),
        (380, 380)
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::PlayerLevelLost { level: 7, .. }))
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, DomainEvent::ExperienceGained { .. }))
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
    for state in [&mut game, &mut restored] {
        state.build.as_mut().unwrap().race_id = "demo.race.rfb-human".to_owned();
        state.apply_player_experience(0, &mut Vec::new());
        assert_eq!(
            (
                state.progress.level,
                state.progress.max_level,
                state.progress.pending_attribute_increases
            ),
            (10, 10, 2)
        );
        assert_eq!(state.rng, rng);
    }
    assert_eq!(
        dispatch_next(&mut game, GameCommand::Wait),
        dispatch_next(&mut restored, GameCommand::Wait)
    );
}

#[test]
fn temporary_form_keeps_native_xp_factor_through_drain_restore_and_save() {
    let mut game = Game::new_with_build(83, "demo.build.warrior").unwrap();
    clear_monsters(&mut game);
    let mut form =
        monster_combat::melee_status(STATUS_PLAYER_POLYMORPH, 1000, "test.experience").status;
    form.granted_race_id = Some("rfb-legacy.race.spectre".to_owned());
    game.player.statuses.push(form);
    game.refresh_player_resource_maxima();
    game.player.hp = game.effective_player_max_hp();
    game.apply_player_experience(100, &mut Vec::new());
    assert_eq!(game.character_experience_percent(), 100);
    assert_eq!(game.progress.level, 6);
    game.apply_player_experience_drain(55, "test.experience", &mut Vec::new());
    assert_eq!(
        (
            game.progress.experience,
            game.progress.maximum_experience,
            game.progress.level
        ),
        (45, 100, 4)
    );
    game.apply_player_experience(5, &mut Vec::new());
    assert_eq!(
        (game.progress.experience, game.progress.maximum_experience),
        (50, 101)
    );
    assert!(game.restore_player_experience_and_life_force(0, &mut Vec::new()));
    assert_eq!((game.progress.experience, game.progress.level), (101, 6));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(
        dispatch_next(&mut game, GameCommand::Wait),
        dispatch_next(&mut restored, GameCommand::Wait)
    );
}
