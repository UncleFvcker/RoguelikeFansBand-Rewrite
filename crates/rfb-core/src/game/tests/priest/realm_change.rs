// SPDX-License-Identifier: MPL-2.0
use super::learning::*;
use super::*;
use crate::game::tests::support::dispatch_next;

fn begin(game: &mut Game, book: &str) {
    dispatch_next(
        game,
        GameCommand::BeginRealmChange {
            book_item_id: book.to_owned(),
        },
    );
}

fn confirm(game: &mut Game, yes: bool) {
    dispatch_next(game, GameCommand::ResolveRealmChange { confirm: yes });
}

fn rejected(game: &mut Game, command: GameCommand) -> CoreError {
    let before = game.to_save();
    let error = game
        .dispatch(GameCommandEnvelope {
            expected_revision: game.revision,
            command_seq: game.last_command_seq + 1,
            command,
        })
        .unwrap_err();
    assert_eq!(game.to_save(), before);
    error
}

#[test]
fn physical_books_and_projection_share_each_primary_realms_alignment_restriction() {
    for first in ["life", "crusade", "death", "daemon"] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 12);
        for realm in [
            "life",
            "sorcery",
            "nature",
            "death",
            "arcane",
            "craft",
            "daemon",
            "crusade",
            "armageddon",
        ] {
            let id = book(&mut game, realm, 1);
            if realm == first || realm == "sorcery" {
                assert!(matches!(
                    rejected(
                        &mut game,
                        GameCommand::BeginRealmChange { book_item_id: id }
                    ),
                    CoreError::RealmChangeUnavailable("already-active-realm")
                ));
            } else if (matches!(first, "life" | "crusade") && matches!(realm, "death" | "daemon"))
                || (matches!(first, "death" | "daemon") && matches!(realm, "life" | "crusade"))
            {
                assert!(matches!(
                    rejected(
                        &mut game,
                        GameCommand::BeginRealmChange {
                            book_item_id: id.clone()
                        }
                    ),
                    CoreError::RealmChangeUnavailable("unsupported-realm")
                ));
                assert!(
                    !game
                        .spell_realms_dto()
                        .unwrap()
                        .change_books
                        .iter()
                        .any(|book| book.book_item_id == id)
                );
            } else {
                assert_eq!(game.realm_change_book(&id).unwrap().realm_id, realm);
            }
        }
        let projected = game.spell_realms_dto().unwrap();
        assert_eq!(projected.first_realm_id, first);
        assert_eq!(projected.second_realm_id, "sorcery");
        assert_eq!(projected.change_books.len(), 5);
        assert!(
            projected
                .change_books
                .iter()
                .any(|book| book.realm_id == "craft")
        );
    }
}

#[test]
fn pending_change_cancels_freely_then_confirmation_learns_and_preserves_primary_history() {
    for (first, next, next_spell) in [
        ("life", "craft", "demo.ability.craft-satisfy-hunger"),
        ("death", "daemon", "demo.ability.daemon-detect-unlife"),
    ] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 12);
        for realm in [first, "sorcery"] {
            let id = book(&mut game, realm, 1);
            game.study_random_player_ability(&id).unwrap();
        }
        let primary = game.ability_learning_order[0].clone();
        let old_secondary = game.ability_learning_order[1].clone();
        let primary_progress = game.ability_progress[&primary];
        let virtues = game.virtues.map(|v| v.kind);
        let id = book(&mut game, next, 1);
        game.items
            .iter_mut()
            .find(|item| item.id == id)
            .unwrap()
            .location = ItemLocation::Ground(game.player.position);
        let before = (
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
            game.spent_spell_learning,
        );
        begin(&mut game, &id);
        assert!(matches!(
            rejected(&mut game, GameCommand::Wait),
            CoreError::RealmChangeRequired
        ));
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            confirm(game, false);
            assert_eq!(game.current_second_realm_id(), Some("sorcery"));
            assert_eq!(
                (
                    game.world_tick,
                    game.player.energy_need,
                    game.rng.clone(),
                    game.spent_spell_learning
                ),
                before
            );
            begin(game, &id);
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            confirm(game, true);
            assert!(game.world_tick > before.0);
            assert_eq!(game.current_second_realm_id(), Some(next));
            assert_eq!(game.spent_spell_learning, 3);
            assert_eq!(game.ability_learning_order.len(), 2);
            assert_eq!(game.ability_progress[&primary], primary_progress);
            assert!(!game.ability_progress.contains_key(&old_secondary));
            assert_eq!(
                game.book_spell_realm(&game.ability_learning_order[1]),
                Some(next)
            );
            assert_eq!(
                game.mage_realms.as_ref().unwrap().previous_realm_ids,
                ["sorcery"]
            );
            assert_eq!(game.virtues.map(|v| v.kind), virtues);
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            // Continue the next random draw after the changed-realm save, then
            // learn the remaining eligible gifts and cast a real new-realm prayer.
            dispatch_next(
                game,
                GameCommand::StudyPrayer {
                    book_item_id: id.clone(),
                },
            );
            loop {
                match game.study_random_player_ability(&id) {
                    Ok(_) => {}
                    Err("no-learnable-abilities") => break,
                    Err(reason) => panic!("unexpected study failure: {reason}"),
                }
            }
            assert!(game.learned_abilities.contains(next_spell));
            game.debug_ability_casts_succeed = true;
            assert!(
                cast(game, next_spell, TargetSelection::SelfTarget)
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
            );
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

#[test]
fn no_candidate_confirmation_keeps_the_new_realm_without_spending_or_a_turn() {
    for (first, next, level) in [("life", "armageddon", 3), ("death", "daemon", 2)] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 1);
        let id = book(&mut game, next, 1);
        begin(&mut game, &id);
        let before = (game.world_tick, game.player.energy_need, game.rng.clone());
        confirm(&mut game, true);
        assert_eq!(game.current_second_realm_id(), Some(next));
        assert_eq!(game.spent_spell_learning, 0);
        assert!(game.learned_abilities.is_empty());
        assert_eq!(
            (game.world_tick, game.player.energy_need, game.rng.clone()),
            before
        );
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            game.apply_player_experience(
                game.experience_required_for_level(level),
                &mut Vec::new(),
            );
            game.study_random_player_ability(&id).unwrap();
            assert_eq!(game.spent_spell_learning, 1);
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

#[test]
fn changing_back_and_forth_cannot_refund_the_shared_learning_budget() {
    let mut game = prepared(BUILD, 2);
    let capacity = game.ability_learning_capacity(game.casting_profile().unwrap());
    let sorcery = book(&mut game, "sorcery", 1);
    let arcane = book(&mut game, "arcane", 1);
    game.study_random_player_ability(&sorcery).unwrap();
    for spent in 1..capacity {
        let id = if spent % 2 == 1 { &arcane } else { &sorcery };
        begin(&mut game, id);
        confirm(&mut game, true);
        assert_eq!(game.spent_spell_learning, u32::from(spent + 1));
    }
    assert_eq!(game.learned_abilities.len(), 1);
    assert_eq!(
        game.ability_learning_remaining(game.casting_profile().unwrap()),
        0
    );
    let id = book(&mut game, "life", 1);
    let before = (
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
        game.spent_spell_learning,
    );
    dispatch_next(&mut game, GameCommand::StudyPrayer { book_item_id: id });
    assert_eq!(
        (
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
            game.spent_spell_learning
        ),
        before
    );
    let craft = book(&mut game, "craft", 1);
    assert!(matches!(
        rejected(
            &mut game,
            GameCommand::BeginRealmChange {
                book_item_id: craft
            }
        ),
        CoreError::RealmChangeUnavailable("learning-capacity-full")
    ));
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn unreadable_or_distant_books_cannot_change_realms_or_draw_a_prayer() {
    let base = prepared(BUILD, 12);
    for obstruction in 0..4 {
        let mut game = base.clone();
        let id = book(&mut game, "craft", 1);
        let active = book(&mut game, "life", 1);
        match obstruction {
            0 => {
                game.apply_player_melee_status(STATUS_BLINDNESS, 10, "test.blind");
            }
            1 => {
                game.apply_player_melee_status(STATUS_CONFUSION, 10, "test.confused");
            }
            2 => {
                game.world_tick = 50_000;
                game.glow.fill(false);
                game.items.retain(|item| item.id == id || item.id == active);
            }
            _ => {
                for item in game
                    .items
                    .iter_mut()
                    .filter(|item| item.id == id || item.id == active)
                {
                    item.location = ItemLocation::Ground(Position {
                        x: game.player.position.x + 1,
                        y: game.player.position.y,
                    });
                }
            }
        }
        assert!(matches!(
            rejected(
                &mut game,
                GameCommand::BeginRealmChange {
                    book_item_id: id.clone()
                }
            ),
            CoreError::RealmChangeUnavailable(_)
        ));
        let before = game.to_save();
        assert!(game.study_random_player_ability(&active).is_err());
        assert_eq!(game.to_save(), before);
    }
}

#[test]
fn saved_current_history_and_pending_books_cannot_cross_the_primary_alignment() {
    for (first, opposite) in [
        ("life", "death"),
        ("crusade", "daemon"),
        ("death", "life"),
        ("daemon", "crusade"),
    ] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 12);
        let forbidden = book(&mut game, opposite, 1);
        let craft = book(&mut game, "craft", 1);
        begin(&mut game, &craft);
        confirm(&mut game, true);
        let baseline = game.to_save();
        for history in [false, true] {
            let mut save = baseline.clone();
            let realms = save.player.mage_realms.as_mut().unwrap();
            if history {
                realms.previous_realm_ids.push(opposite.to_owned());
                realms.previous_realm_ids.sort();
            } else {
                realms.second_realm_id = opposite.to_owned();
            }
            assert!(matches!(
                Game::from_save(save),
                Err(CoreError::InvalidSave("spell realms are invalid"))
            ));
        }
        let mut save = baseline.clone();
        save.player
            .mage_realms
            .as_mut()
            .unwrap()
            .pending_change_book_item_id = Some(forbidden);
        assert!(matches!(
            Game::from_save(save),
            Err(CoreError::InvalidSave("pending realm change is invalid"))
        ));
        for corruption in 0..6 {
            let mut save = baseline.clone();
            let realms = save.player.mage_realms.as_mut().unwrap();
            match corruption {
                0 => realms.second_realm_id = first.to_owned(),
                1 => realms.previous_realm_ids.clear(),
                2 => realms.previous_realm_ids.push("sorcery".to_owned()),
                3 => realms.previous_realm_ids = vec![first.to_owned(), "sorcery".to_owned()],
                4 => realms.second_realm_id = "missing-realm".to_owned(),
                _ => realms.pending_change_book_item_id = Some(craft.clone()),
            }
            assert!(
                Game::from_save(save).is_err(),
                "{first}, corruption {corruption}"
            );
        }
        assert!(Game::from_save(baseline).is_ok());
    }
}

#[test]
fn saved_historical_spending_uses_ninety_six_instead_of_the_mage_limit() {
    let mut game = prepared(BUILD, 50);
    let craft = book(&mut game, "craft", 1);
    begin(&mut game, &craft);
    confirm(&mut game, true);
    let mut save = game.to_save();
    save.player.learned_ability_ids.clear();
    save.player.ability_learning_order.clear();
    // Even allowing all 64 forgotten slots, this exceeds Priest's historical
    // ceiling. The old generic Mage bound of 100 + 64 would accept it.
    save.player.spent_spell_learning = 96 + 64 + 1;
    assert!(matches!(
        Game::from_save(save),
        Err(CoreError::InvalidSave("player spell memory is invalid"))
    ));
}
