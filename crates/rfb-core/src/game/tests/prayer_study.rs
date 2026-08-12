// SPDX-License-Identifier: MPL-2.0

use super::support::{dispatch_next, divine_caster_game, test_caster_game};
use super::*;

const FIRST_DEATH_PRAYER: &str = "demo.ability.death-detect-unlife";

fn first_book_item_id(game: &Game) -> String {
    game.items
        .iter()
        .find(|item| item.kind_id == "demo.item.black-prayers")
        .expect("test caster should carry the first Death book")
        .id
        .clone()
}

#[test]
fn chosen_and_divine_study_modes_keep_distinct_commands() {
    let mut chosen = test_caster_game(11);
    let chosen_book = first_book_item_id(&chosen);
    assert_eq!(
        chosen
            .snapshot()
            .player
            .ability_learning
            .expect("chosen caster should project learning")
            .study_mode,
        rfb_protocol::AbilityStudyModeDto::Chosen
    );
    chosen
        .study_player_ability(&chosen_book, FIRST_DEATH_PRAYER)
        .expect("chosen study should still learn an explicit spell");
    assert_eq!(
        chosen.study_random_player_ability(&chosen_book),
        Err("study-mode-mismatch")
    );

    let mut divine = divine_caster_game(11);
    let divine_book = first_book_item_id(&divine);
    assert_eq!(
        divine
            .snapshot()
            .player
            .ability_learning
            .expect("divine caster should project learning")
            .study_mode,
        rfb_protocol::AbilityStudyModeDto::DivineRandom
    );
    assert_eq!(
        divine.study_player_ability(&divine_book, FIRST_DEATH_PRAYER),
        Err("study-mode-mismatch")
    );
    let update = dispatch_next(
        &mut divine,
        GameCommand::StudyPrayer {
            book_item_id: divine_book,
        },
    );
    assert!(
        update
            .events
            .iter()
            .any(|event| event.kind == "ability.studied")
    );
    let learned = divine
        .learned_abilities
        .iter()
        .next()
        .expect("divine study should grant one prayer");
    assert!(
        divine
            .content
            .ability_book("demo.ability-book.black-prayers")
            .expect("first Death book should exist")
            .ability_ids
            .contains(learned)
    );
    assert_eq!(divine.learned_abilities.len(), 1);
}

#[test]
fn divine_study_is_deterministic_and_accepts_a_book_at_the_players_feet() {
    let mut first = divine_caster_game(0x5052_4159_4552);
    let mut second = divine_caster_game(0x5052_4159_4552);
    let first_book = first_book_item_id(&first);
    let second_book = first_book_item_id(&second);
    first
        .items
        .iter_mut()
        .find(|item| item.id == first_book)
        .expect("first book should exist")
        .location = ItemLocation::Ground(first.player.position);

    let snapshot = first.snapshot();
    let projected = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == FIRST_DEATH_PRAYER)
        .expect("first prayer should be projected")
        .book_item_id
        .as_deref();
    assert_eq!(projected, Some(first_book.as_str()));
    assert_eq!(
        first
            .study_random_player_ability(&first_book)
            .expect("floor book should be readable"),
        second
            .study_random_player_ability(&second_book)
            .expect("carried book should be readable")
    );
}

#[test]
fn blindness_darkness_and_confusion_block_study_before_rng() {
    let mut blind = divine_caster_game(21);
    let blind_book = first_book_item_id(&blind);
    blind.apply_player_melee_status(STATUS_BLINDNESS, 10, "test.blindness");
    assert_eq!(blind.study_random_player_ability(&blind_book), Err("blind"));

    let mut confused = divine_caster_game(22);
    let confused_book = first_book_item_id(&confused);
    confused.apply_player_melee_status(STATUS_CONFUSION, 10, "test.confusion");
    assert_eq!(
        confused.study_random_player_ability(&confused_book),
        Err("confused")
    );

    let mut dark = divine_caster_game(23);
    let dark_book = first_book_item_id(&dark);
    dark.world_tick = 50_000;
    dark.glow.fill(false);
    dark.entities.clear();
    dark.items.retain(|item| item.id == dark_book);
    assert!(!dark.position_is_lit(dark.player.position));
    assert_eq!(
        dark.study_random_player_ability(&dark_book),
        Err("no-light")
    );
}
