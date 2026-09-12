// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::dispatch_next;

const PRIMARY: &str = "demo.ability.arcane-detect-monsters";
const SECONDARY: &str = "demo.ability.sorcery-detect-monsters";
const LIFE: &str = "demo.ability.life-cure-light-wounds";

fn prepared(build: &str, level: u16) -> Game {
    let mut game = at_level(build, level);
    game.progress.attributes.intelligence = game.progress.attribute_potentials.intelligence;
    game.progress.maximum_attributes.intelligence = game.progress.attributes.intelligence;
    game.refresh_player_ability_state();
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    game
}

fn book_for(game: &mut Game, spell: &str) -> String {
    let kind = game
        .content
        .item_definitions()
        .find(|item| {
            item.ability_book_id
                .as_deref()
                .and_then(|id| game.content.ability_book(id))
                .is_some_and(|book| book.ability_ids.iter().any(|id| id == spell))
        })
        .unwrap()
        .id
        .clone();
    if let Some(item) = game.items.iter().find(|item| item.kind_id == kind) {
        return item.id.clone();
    }
    let id = format!("test.book.{kind}");
    give_inventory_item(game, &id, &kind);
    id
}

fn learn(game: &mut Game, spell: &str) -> String {
    let book = book_for(game, spell);
    game.study_player_ability(&book, spell).unwrap();
    book
}

fn cast(game: &mut Game, spell: &str, directed: bool) -> Vec<DomainEvent> {
    let target = if directed {
        TargetSelection::Direction {
            direction: Direction::East,
        }
    } else {
        TargetSelection::SelfTarget
    };
    let mut events = Vec::new();
    game.resolve_player_ability(
        spell,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

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

#[test]
fn all_eight_builds_choose_and_cast_both_realms_with_source_experience() {
    for (realm, spell, directed) in [
        ("life", LIFE, false),
        ("sorcery", SECONDARY, false),
        ("nature", "demo.ability.nature-detect-creatures", false),
        ("death", "demo.ability.death-detect-unlife", false),
        ("craft", "demo.ability.craft-regeneration", false),
        ("daemon", "demo.ability.daemon-magic-missile", true),
        ("crusade", "demo.ability.crusade-punishment", true),
        ("armageddon", "demo.ability.armageddon-lightning-bolt", true),
    ] {
        let mut game = prepared(&format!("demo.build.warrior-mage-arcane-{realm}"), 10);
        game.debug_ability_casts_succeed = true;
        for (spell, directed) in [("demo.ability.arcane-zap", true), (spell, directed)] {
            let rng = game.rng.clone();
            learn(&mut game, spell);
            assert_eq!(game.rng, rng, "chosen study does not draw a random prayer");
            let profile = game.casting_profile().unwrap();
            let expected = Game::player_ability_parameters(
                &game.effective_casting_ability(profile, game.content.ability(spell).unwrap()),
            )
            .first_success_experience;
            let xp = game.progress.experience;
            let mana = game.resources[MANA].current;
            assert!(
                cast(&mut game, spell, directed)
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. })),
                "{spell}"
            );
            assert!(game.resources[MANA].current < mana);
            assert_eq!(game.progress.experience - xp, u64::from(expected));
            cast(&mut game, spell, directed);
            assert_eq!(game.progress.experience - xp, u64::from(expected));
        }
        assert_eq!(game.spent_spell_learning, 2);
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
    }
}

#[test]
fn repeated_study_updates_projection_caps_budget_and_knowledge_even_for_life() {
    let mut game = prepared("demo.build.warrior-mage-arcane-life", 50);
    // Ensure both virtues exist so the class book category is observable.
    if !game
        .virtues
        .iter()
        .any(|v| v.kind == VirtueKindDto::Knowledge)
    {
        game.virtues.last_mut().unwrap().kind = VirtueKindDto::Knowledge;
    }
    let knowledge = game.virtue_current(VirtueKindDto::Knowledge);
    let faith = game.virtue_current(VirtueKindDto::Faith);
    for (spell, ranks) in [
        (PRIMARY, &[900, 1200, 1400, 1600][..]),
        (LIFE, &[900, 1200, 1400][..]),
    ] {
        let book = learn(&mut game, spell);
        let order = game.ability_learning_order.clone();
        for &rank in ranks {
            assert!(
                game.snapshot()
                    .player
                    .abilities
                    .iter()
                    .find(|a| a.id == spell)
                    .unwrap()
                    .can_study
            );
            game.study_player_ability(&book, spell).unwrap();
            assert_eq!(game.ability_progress[spell].proficiency, rank);
            assert_eq!(game.ability_learning_order, order);
        }
        let before = game.to_save();
        assert_eq!(
            game.study_player_ability(&book, spell),
            Err("proficiency-at-cap")
        );
        assert_eq!(
            game.forget_player_ability(spell),
            Err("manual-forgetting-unavailable")
        );
        assert_eq!(game.to_save(), before);
        assert!(
            !game
                .snapshot()
                .player
                .abilities
                .iter()
                .find(|a| a.id == spell)
                .unwrap()
                .can_study
        );
    }
    assert_eq!(game.spent_spell_learning, 9);
    assert_eq!(game.virtue_current(VirtueKindDto::Knowledge), knowledge + 9);
    assert_eq!(game.virtue_current(VirtueKindDto::Faith), faith);
    let learning = game.player_ability_learning_dto().unwrap();
    assert_eq!(
        (
            learning.capacity,
            learning.learned_count,
            learning.remaining_slots
        ),
        (84, 2, 75)
    );
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn all_eighty_four_studies_can_be_spent_and_illegal_spending_is_rejected() {
    let mut game = prepared(BUILD, 50);
    let (_, ids) = game.player_ability_baseline();
    for spell in ids {
        if game.ability_learning_remaining(game.casting_profile().unwrap()) == 0 {
            break;
        }
        let book = book_for(&mut game, &spell);
        while game.study_player_ability(&book, &spell).is_ok() {}
    }
    assert_eq!(game.spent_spell_learning, 84);
    assert!(game.learned_abilities.len() < 64);
    let spell = game.ability_learning_order[0].clone();
    let book = book_for(&mut game, &spell);
    let before = game.to_save();
    assert_eq!(
        game.study_player_ability(&book, &spell),
        Err("learning-capacity-full")
    );
    assert_eq!(game.to_save(), before);
    assert!(Game::from_save(before).is_ok());

    let mut game = prepared(BUILD, 20);
    let book = learn(&mut game, SECONDARY);
    game.study_player_ability(&book, SECONDARY).unwrap();
    let baseline = game.to_save();
    for corruption in 0..6 {
        let mut save = baseline.clone();
        match corruption {
            0 => save.player.spent_spell_learning = 0,
            1 => save.player.spent_spell_learning = 3,
            2 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == SECONDARY)
                    .unwrap()
                    .proficiency_cap = 1600
            }
            3 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == SECONDARY)
                    .unwrap()
                    .proficiency = 1401
            }
            4 => save
                .player
                .ability_learning_order
                .push(SECONDARY.to_owned()),
            _ => {
                save.player.ability_progress.pop();
            }
        }
        assert!(Game::from_save(save).is_err(), "{corruption}");
    }
    assert!(Game::from_save(baseline).is_ok());
}

#[test]
fn study_requires_readable_matching_books_and_only_success_advances_time() {
    let mut game = at_level(BUILD, 1);
    let book = book_for(&mut game, "demo.ability.arcane-zap");
    for (reason, status) in [("blind", STATUS_BLINDNESS), ("confused", STATUS_CONFUSION)] {
        let mut blocked = game.clone();
        blocked.apply_player_mental_status(status, 10, "test");
        let before = blocked.to_save();
        assert_eq!(
            blocked.study_player_ability(&book, "demo.ability.arcane-zap"),
            Err(reason)
        );
        assert_eq!(blocked.to_save(), before);
    }
    assert_eq!(
        game.study_player_ability(&book, PRIMARY),
        Err("level-too-low")
    );
    assert_eq!(
        game.study_player_ability(&book, SECONDARY),
        Err("book-mismatch")
    );
    assert_eq!(
        game.study_random_player_ability(&book),
        Err("study-mode-mismatch")
    );
    let tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: book.clone(),
            ability_id: "demo.ability.arcane-zap".to_owned(),
        },
    );
    assert!(game.world_tick > tick);
    let before = (game.world_tick, game.player.energy_need, game.rng.clone());
    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: book,
            ability_id: "demo.ability.arcane-zap".to_owned(),
        },
    );
    assert_eq!(
        (game.world_tick, game.player.energy_need, game.rng.clone()),
        before
    );
    assert_eq!(game.spent_spell_learning, 1);
}

#[test]
fn drain_forgets_without_refunding_and_restored_growth_replays_identically() {
    let mut game = prepared(BUILD, 50);
    let advanced = "demo.ability.arcane-clairvoyance";
    learn(&mut game, advanced);
    let book = learn(&mut game, PRIMARY);
    learn(&mut game, SECONDARY);
    game.study_player_ability(&book, PRIMARY).unwrap();
    let order = game.ability_learning_order.clone();
    let progress = game.ability_progress.clone();
    game.apply_player_experience_drain(game.progress.experience, "test.drain", &mut Vec::new());
    assert_eq!(game.progress.level, 1);
    assert!(!game.learned_abilities.contains(advanced));
    game.progress.attributes.intelligence = 3;
    game.refresh_player_ability_state();
    assert!(game.learned_abilities.is_empty());
    assert_eq!(game.spent_spell_learning, 4);
    assert_eq!(game.ability_learning_order, order);
    assert_eq!(game.ability_progress, progress);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .filter(|a| order.contains(&a.id))
            .all(|a| a.forgotten && !a.can_study && !a.can_cast)
    );
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        game.progress.attributes.intelligence = game.progress.maximum_attributes.intelligence;
        game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
        game.refresh_player_ability_state();
        assert_eq!(game.learned_abilities, order.iter().cloned().collect());
        assert_eq!(game.ability_progress, progress);
        game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
        cast(game, SECONDARY, false);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn pending_changes_cancel_freely_then_commit_without_random_study_and_replay_after_save() {
    for ground in [false, true] {
        let mut game = prepared(BUILD, 40);
        let first_book = learn(&mut game, PRIMARY);
        game.study_player_ability(&first_book, PRIMARY).unwrap();
        let old_book = learn(&mut game, SECONDARY);
        game.study_player_ability(&old_book, SECONDARY).unwrap();
        let new_book = book_for(&mut game, LIFE);
        if ground {
            game.items
                .iter_mut()
                .find(|i| i.id == new_book)
                .unwrap()
                .location = ItemLocation::Ground(game.player.position);
        }
        let primary = game.ability_progress[PRIMARY];
        let clock = (game.world_tick, game.player.energy_need, game.rng.clone());
        begin(&mut game, &new_book);
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            confirm(game, false);
            assert_eq!(game.current_second_realm_id(), Some("sorcery"));
            assert_eq!(
                (game.world_tick, game.player.energy_need, game.rng.clone()),
                clock
            );
            begin(game, &new_book);
            confirm(game, true);
            assert_eq!(game.current_second_realm_id(), Some("life"));
            assert_eq!(game.ability_learning_order, [PRIMARY]);
            assert_eq!(game.ability_progress[PRIMARY], primary);
            assert!(!game.ability_progress.contains_key(SECONDARY));
            assert_eq!(game.spent_spell_learning, 4);
            assert_eq!(
                (game.world_tick, game.player.energy_need, game.rng.clone()),
                clock
            );
            assert!(!game.learned_abilities.contains(LIFE));
            assert!(game.pending_realm_change_book().is_none());
            assert_eq!(game.spell_realms_dto().unwrap().first_realm_id, "arcane");
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        // Cancelling the subsequent spell picker leaves the confirmed realm in the save.
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            dispatch_next(
                game,
                GameCommand::StudyAbility {
                    book_item_id: new_book.clone(),
                    ability_id: LIFE.to_owned(),
                },
            );
            cast(game, LIFE, false);
            begin(game, &old_book);
            confirm(game, true);
            assert!(!game.learned_abilities.contains(SECONDARY));
            assert_eq!(game.ability_progress[SECONDARY].proficiency, 0);
            assert_eq!(game.ability_progress[SECONDARY].cast_count, 0);
            dispatch_next(
                game,
                GameCommand::StudyAbility {
                    book_item_id: old_book.clone(),
                    ability_id: SECONDARY.to_owned(),
                },
            );
            cast(game, SECONDARY, false);
            assert_eq!(game.spent_spell_learning, 6);
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
}

#[test]
fn realm_eligibility_and_saved_history_keep_primary_arcane_and_the_eighty_four_ceiling() {
    let mut game = prepared(BUILD, 50);
    let primary = book_for(&mut game, PRIMARY);
    assert!(matches!(
        game.realm_change_book(&primary),
        Err(CoreError::RealmChangeUnavailable("already-active-realm"))
    ));
    // Both alignments and Craft are available as second realms for this class.
    for spell in [
        LIFE,
        "demo.ability.death-detect-unlife",
        "demo.ability.craft-regeneration",
    ] {
        let book = book_for(&mut game, spell);
        assert!(game.realm_change_book(&book).is_ok());
    }
    let book = book_for(&mut game, LIFE);
    begin(&mut game, &book);
    let mut invalid = game.to_save();
    invalid
        .player
        .mage_realms
        .as_mut()
        .unwrap()
        .pending_change_book_item_id = Some(primary);
    assert!(Game::from_save(invalid).is_err());
    confirm(&mut game, true);
    let baseline = game.to_save();
    for corruption in 0..6 {
        let mut save = baseline.clone();
        let realms = save.player.mage_realms.as_mut().unwrap();
        match corruption {
            0 => realms.second_realm_id = "arcane".to_owned(),
            1 => realms.previous_realm_ids = vec!["arcane".to_owned(), "sorcery".to_owned()],
            2 => realms.previous_realm_ids.clear(),
            3 => realms.previous_realm_ids.push("sorcery".to_owned()),
            4 => realms.second_realm_id = "chaos".to_owned(),
            _ => save.player.spent_spell_learning = 84 + 64 + 1,
        }
        assert!(Game::from_save(save).is_err(), "{corruption}");
    }
    assert!(Game::from_save(baseline).is_ok());
}

#[test]
fn failure_uses_intelligence_without_secondary_surcharge_and_resumes_natural_failure() {
    let mut game = at_level(BUILD, 3);
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    let failure = |game: &Game, spell: &str| {
        let profile = game.casting_profile().unwrap();
        let ability = game.effective_casting_ability(profile, game.content.ability(spell).unwrap());
        game.ability_failure_percent(profile, &ability)
    };
    // INT15 subtracts 3. Arcane: L3/fail33; Sorcery: L1/fail23 minus 6 for level.
    assert_eq!(failure(&game, PRIMARY), 30);
    assert_eq!(failure(&game, SECONDARY), 14);
    let mut game = prepared(BUILD, 50);
    learn(&mut game, PRIMARY);
    learn(&mut game, SECONDARY);
    let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
    stun.intensity = 40;
    let minimum = failure(&game, PRIMARY);
    assert_eq!(failure(&game, SECONDARY), minimum);
    game.player.statuses.push(stun);
    game.ability_progress.get_mut(PRIMARY).unwrap().proficiency = 1600;
    game.ability_progress
        .get_mut(SECONDARY)
        .unwrap()
        .proficiency = 1400;
    assert_eq!(failure(&game, PRIMARY), minimum + 20 - 2);
    assert_eq!(failure(&game, SECONDARY), minimum + 20 - 1);
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) < u64::from(failure(&game, SECONDARY)))
        .unwrap();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let mana = game.resources[MANA].current;
    let events = cast(&mut game, SECONDARY, false);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, DomainEvent::AbilityCastFailed { .. }))
    );
    assert!(game.resources[MANA].current < mana);
    assert_eq!(game.ability_progress[SECONDARY].proficiency, 1400);
    assert_eq!(game.ability_progress[SECONDARY].cast_count, 0);
    assert_eq!(game.ability_progress[SECONDARY].fail_count, 1);
    assert_eq!(events, cast(&mut restored, SECONDARY, false));
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn real_dungeon_practice_and_interpolated_restudy_respect_each_realm_cap() {
    let mut game = prepared(BUILD, 50);
    let primary = learn(&mut game, PRIMARY);
    let secondary = learn(&mut game, SECONDARY);
    game.debug_ability_casts_succeed = true;
    cast(&mut game, PRIMARY, false);
    assert_eq!(
        game.ability_progress[PRIMARY].proficiency, 0,
        "town grants no practice"
    );
    game.transition_floor("demo.floor.camelot-depth-24".to_owned(), None, None, false)
        .unwrap();
    clear_monsters(&mut game);
    game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
    cast(&mut game, PRIMARY, false);
    assert_eq!(game.ability_progress[PRIMARY].proficiency, 384);
    for (spell, book, before, after) in [
        (PRIMARY, &primary, 899, 1199),
        (SECONDARY, &secondary, 1399, 1400),
    ] {
        game.ability_progress.get_mut(spell).unwrap().proficiency = before;
        game.study_player_ability(book, spell).unwrap();
        assert_eq!(game.ability_progress[spell].proficiency, after);
    }
    for (spell, before, after) in [(PRIMARY, 1599, 1600), (SECONDARY, 1399, 1400)] {
        game.ability_progress.get_mut(spell).unwrap().proficiency = before;
        cast(&mut game, spell, false);
        assert_eq!(game.ability_progress[spell].proficiency, after);
    }
    assert_eq!(game.spent_spell_learning, 4);
    assert!(Game::from_save(game.to_save()).is_ok());
}
