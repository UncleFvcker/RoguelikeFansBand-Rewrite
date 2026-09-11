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
fn pending_change_saves_and_cancels_without_rng_or_spending() {
    let mut game = prepared(SORCERY_BUILD, 10);
    let book_id = book(&mut game, "death", 1);
    game.items
        .iter_mut()
        .find(|item| item.id == book_id)
        .unwrap()
        .location = ItemLocation::Ground(game.player.position);
    let before = (
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
        game.ability_progress.clone(),
    );
    begin(&mut game, &book_id);
    assert_eq!(
        game.spell_realms_dto()
            .unwrap()
            .pending_change
            .unwrap()
            .realm_id,
        "death"
    );
    assert!(matches!(
        rejected(&mut game, GameCommand::Wait),
        CoreError::RealmChangeRequired
    ));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        confirm(game, false);
        assert_eq!(game.current_second_realm_id(), Some("sorcery"));
        assert_eq!(game.spent_spell_learning, 0);
        assert_eq!(
            (
                game.world_tick,
                game.player.energy_need,
                game.rng.clone(),
                game.ability_progress.clone()
            ),
            before
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn confirmation_learns_immediately_and_keeps_primary_progress_and_paid_history() {
    let mut game = prepared(SORCERY_BUILD, 12);
    for realm in ["nature", "sorcery"] {
        let book_id = book(&mut game, realm, 1);
        game.study_random_player_ability(&book_id).unwrap();
    }
    let old_secondary = game.ability_learning_order[1].clone();
    let primary = game.ability_learning_order[0].clone();
    let primary_progress = game.ability_progress[&primary];
    let book_id = book(&mut game, "death", 1);
    begin(&mut game, &book_id);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let tick = game.world_tick;
    for game in [&mut game, &mut restored] {
        confirm(game, true);
        assert!(game.world_tick > tick);
        assert_eq!(game.current_second_realm_id(), Some("death"));
        assert_eq!(game.spent_spell_learning, 3);
        assert_eq!(game.ability_learning_order.len(), 2);
        assert_eq!(game.ability_progress[&primary], primary_progress);
        assert!(!game.ability_progress.contains_key(&old_secondary));
        let new_spell = &game.ability_learning_order[1];
        assert_eq!(game.book_spell_realm(new_spell), Some("death"));
        assert_eq!(game.ability_progress[new_spell].proficiency_cap, 1400);
        assert_eq!(
            game.mage_realms.as_ref().unwrap().previous_realm_ids,
            ["sorcery"]
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        dispatch_next(
            game,
            GameCommand::StudyPrayer {
                book_item_id: book_id.clone(),
            },
        );
        learn_book(game, &book_id);
        refill(game);
        let spell = "demo.ability.death-detect-unlife";
        game.debug_ability_casts_succeed = true;
        let events = cast(game, spell, TargetSelection::SelfTarget);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn no_candidate_confirmation_commits_the_realm_without_a_learning_turn() {
    let mut game = prepared(SORCERY_BUILD, 3);
    let book_id = book(&mut game, "death", 1); // first Ranger Death spell requires L5
    begin(&mut game, &book_id);
    let before = (game.world_tick, game.player.energy_need, game.rng.clone());
    let update = dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.study-unavailable")
    );
    assert_eq!(game.current_second_realm_id(), Some("death"));
    assert_eq!(game.spent_spell_learning, 0);
    assert!(game.learned_abilities.is_empty());
    assert_eq!(
        (game.world_tick, game.player.energy_need, game.rng.clone()),
        before
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        game.apply_player_experience(
            game.experience_required_for_level(5) - game.progress.experience,
            &mut Vec::new(),
        );
        assert_eq!(
            game.study_random_player_ability(&book_id).unwrap(),
            "demo.ability.death-detect-unlife"
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn repeated_realm_changes_exhaust_shared_budget_despite_one_current_spell() {
    let mut game = prepared(SORCERY_BUILD, 3);
    let capacity = game.ability_learning_capacity(game.casting_profile().unwrap());
    let sorcery = book(&mut game, "sorcery", 1);
    let arcane = book(&mut game, "arcane", 1);
    game.study_random_player_ability(&sorcery).unwrap();
    for spent in 1..capacity {
        let book_id = if spent % 2 == 1 { &arcane } else { &sorcery };
        begin(&mut game, book_id);
        confirm(&mut game, true);
        assert_eq!(game.spent_spell_learning, u32::from(spent + 1));
    }
    assert_eq!(game.learned_abilities.len(), 1);
    assert_eq!(
        game.ability_learning_remaining(game.casting_profile().unwrap()),
        0
    );
    let nature = book(&mut game, "nature", 1);
    let before = (
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
        game.spent_spell_learning,
    );
    dispatch_next(
        &mut game,
        GameCommand::StudyPrayer {
            book_item_id: nature,
        },
    );
    assert_eq!(
        (
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
            game.spent_spell_learning
        ),
        before
    );
    let death = book(&mut game, "death", 1);
    assert!(matches!(
        rejected(
            &mut game,
            GameCommand::BeginRealmChange {
                book_item_id: death
            }
        ),
        CoreError::RealmChangeUnavailable("learning-capacity-full")
    ));
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn returning_to_old_realm_starts_new_progress_and_first_cast_experience() {
    let mut game = prepared(SORCERY_BUILD, 12);
    let sorcery = book(&mut game, "sorcery", 1);
    learn_book(&mut game, &sorcery);
    game.debug_ability_casts_succeed = true;
    let before = game.progress.experience;
    cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert_eq!(game.progress.experience - before, 6);
    let death = book(&mut game, "death", 1);
    begin(&mut game, &death);
    confirm(&mut game, true);
    let spent = game.spent_spell_learning;
    begin(&mut game, &sorcery);
    confirm(&mut game, true);
    assert_eq!(game.spent_spell_learning, spent + 1);
    learn_book(&mut game, &sorcery);
    assert_eq!(game.ability_progress[SORCERY].cast_count, 0);
    assert_eq!(game.ability_progress[SORCERY].proficiency, 0);
    refill(&mut game);
    let before = game.progress.experience;
    cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert_eq!(game.progress.experience - before, 6);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn primary_foreign_and_unreadable_books_cannot_begin_a_change() {
    let base = prepared(SORCERY_BUILD, 12);
    for realm in [
        "nature",
        "sorcery",
        "life",
        "craft",
        "crusade",
        "armageddon",
    ] {
        let mut game = base.clone();
        let id = book(&mut game, realm, 1);
        assert!(matches!(
            rejected(
                &mut game,
                GameCommand::BeginRealmChange { book_item_id: id }
            ),
            CoreError::RealmChangeUnavailable(_)
        ));
    }
    for obstruction in 0..4 {
        let mut game = base.clone();
        let id = book(&mut game, "death", 1);
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
                game.items.retain(|item| item.id == id);
            }
            _ => {
                game.items
                    .iter_mut()
                    .find(|item| item.id == id)
                    .unwrap()
                    .location = ItemLocation::Ground(Position {
                    x: game.player.position.x + 1,
                    y: game.player.position.y,
                });
            }
        }
        assert!(matches!(
            rejected(
                &mut game,
                GameCommand::BeginRealmChange { book_item_id: id }
            ),
            CoreError::RealmChangeUnavailable(_)
        ));
    }
}

#[test]
fn saves_reject_invalid_current_history_pending_and_old_spell_state() {
    let mut game = prepared(SORCERY_BUILD, 12);
    let sorcery = book(&mut game, "sorcery", 1);
    game.study_random_player_ability(&sorcery).unwrap();
    let old_spell = game.ability_learning_order[0].clone();
    let old_progress = game
        .to_save()
        .player
        .ability_progress
        .into_iter()
        .find(|p| p.id == old_spell)
        .unwrap();
    let death = book(&mut game, "death", 1);
    begin(&mut game, &death);
    let mut invalid = game.to_save();
    invalid
        .player
        .mage_realms
        .as_mut()
        .unwrap()
        .pending_change_book_item_id = Some("missing".to_owned());
    assert!(Game::from_save(invalid).is_err());
    confirm(&mut game, true);
    let baseline = game.to_save();
    for corruption in 0..10 {
        let mut save = baseline.clone();
        let realms = save.player.mage_realms.as_mut().unwrap();
        match corruption {
            0 => realms.second_realm_id = "nature".to_owned(),
            1 => realms.second_realm_id = "craft".to_owned(),
            2 => realms.previous_realm_ids.clear(),
            3 => realms.previous_realm_ids = vec!["nature".to_owned()],
            4 => realms.previous_realm_ids.push("sorcery".to_owned()),
            5 => save.player.ability_progress.push(old_progress.clone()),
            6 => save.player.ability_learning_order.push(old_spell.clone()),
            7 => save.player.spent_spell_learning = 0,
            8 => save.player.spent_spell_learning = 1000,
            _ => realms.pending_change_book_item_id = Some(death.clone()),
        }
        assert!(Game::from_save(save).is_err(), "corruption {corruption}");
    }
    assert!(Game::from_save(baseline).is_ok());
}
