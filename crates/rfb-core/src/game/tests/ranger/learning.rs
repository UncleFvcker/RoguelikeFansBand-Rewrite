// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::tests::support::dispatch_next;

pub(super) const NATURE: &str = "demo.ability.nature-detect-creatures";
pub(super) const SORCERY: &str = "demo.ability.sorcery-detect-monsters";
pub(super) const SORCERY_BUILD: &str = "demo.build.ranger-nature-sorcery";

pub(super) fn prepared(build: &str, level: u16) -> Game {
    let mut game = at_level(build, level);
    game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
    game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
    game.refresh_player_ability_state();
    refill(&mut game);
    game
}

pub(super) fn refill(game: &mut Game) {
    let pool = game.resources.get_mut(MANA).unwrap();
    pool.current = pool.maximum;
}

pub(super) fn book(game: &mut Game, realm: &str, rank: u8) -> String {
    let kind = game
        .content
        .item_definitions()
        .find(|item| {
            item.ability_book_id
                .as_deref()
                .and_then(|id| game.content.ability_book(id))
                .is_some_and(|book| {
                    book.realm_id.as_deref() == Some(realm) && book.rank == Some(rank)
                })
        })
        .unwrap()
        .id
        .clone();
    if let Some(item) = game.items.iter().find(|item| item.kind_id == kind) {
        return item.id.clone();
    }
    let id = format!("test.book.{realm}.{rank}");
    give_inventory_item(game, &id, &kind);
    id
}

pub(super) fn learn_book(game: &mut Game, book: &str) {
    loop {
        match game.study_random_player_ability(book) {
            Ok(_) => {}
            Err("no-learnable-abilities") => break,
            Err(reason) => panic!("book learning failed: {reason}"),
        }
    }
}

pub(super) fn cast(game: &mut Game, spell: &str, target: TargetSelection) -> Vec<DomainEvent> {
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

fn learn_initial_pair(game: &mut Game) {
    for (realm, spell) in [("nature", NATURE), ("sorcery", SORCERY)] {
        let book_id = book(game, realm, 1);
        // L3 Sorcery also offers Phase Door; retain the first source gift.
        game.rng = (0..100)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                rng.bounded(1);
                rng.bounded(2) != 0
            })
            .unwrap();
        assert_eq!(game.study_random_player_ability(&book_id).unwrap(), spell);
    }
}

#[test]
fn random_study_uses_source_book_order_and_spends_one_shared_opportunity() {
    for last in [false, true] {
        let mut game = prepared(BUILD, 10);
        let book_id = book(&mut game, "nature", 1);
        if last {
            game.items
                .iter_mut()
                .find(|item| item.id == book_id)
                .unwrap()
                .location = ItemLocation::Ground(game.player.position);
        }
        // The first/last gift are fixed by the source one_in_(k) draws.
        game.rng = (0..1000)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                rng.bounded(1);
                let rolls: Vec<_> = (2..=8).map(|k| rng.bounded(k)).collect();
                if last {
                    rolls[6] == 0
                } else {
                    rolls.iter().all(|roll| *roll != 0)
                }
            })
            .unwrap();
        let mut expected_rng = game.rng.clone();
        for k in 1..=8 {
            expected_rng.bounded(k);
        }
        let before = game.ability_learning_remaining(game.casting_profile().unwrap());
        let spell = game.study_random_player_ability(&book_id).unwrap();
        assert_eq!(
            spell,
            if last {
                "demo.ability.nature-cure-wounds-and-poison"
            } else {
                NATURE
            }
        );
        assert_eq!(game.rng, expected_rng);
        assert_eq!(game.spent_spell_learning, 1);
        assert_eq!(
            game.ability_learning_remaining(game.casting_profile().unwrap()),
            before - 1
        );
        assert_eq!(game.ability_progress[&spell].proficiency_cap, 1600);
        let projected = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == spell)
            .unwrap();
        assert!(!projected.can_study && !projected.can_forget);
        let before = game.to_save();
        assert_eq!(
            game.study_player_ability(&book_id, &spell),
            Err("study-mode-mismatch")
        );
        assert_eq!(
            game.forget_player_ability(&spell),
            Err("manual-forgetting-unavailable")
        );
        assert_eq!(game.to_save(), before);
    }
}

#[test]
fn all_four_builds_learn_and_cast_from_both_birth_books() {
    for (second, spell, directed) in [
        ("sorcery", SORCERY, false),
        ("death", "demo.ability.death-detect-unlife", false),
        ("arcane", "demo.ability.arcane-zap", true),
        ("daemon", "demo.ability.daemon-magic-missile", true),
    ] {
        let mut game = prepared(&format!("demo.build.ranger-nature-{second}"), 50);
        game.debug_ability_casts_succeed = true;
        for (realm, spell, directed) in [("nature", NATURE, false), (second, spell, directed)] {
            let book_id = book(&mut game, realm, 1);
            game.rng = (0..1000)
                .map(RfbRng::seeded)
                .find(|rng| {
                    let mut rng = rng.clone();
                    rng.bounded(1);
                    (2..=8).all(|k| rng.bounded(k) != 0)
                })
                .unwrap();
            assert_eq!(game.study_random_player_ability(&book_id).unwrap(), spell);
            let target = if directed {
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            } else {
                TargetSelection::SelfTarget
            };
            let mana = game.resources[MANA].current;
            let events = cast(&mut game, spell, target);
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. })),
                "{spell}: {events:?}"
            );
            assert_eq!(game.ability_progress[spell].cast_count, 1);
            assert!(game.resources[MANA].current < mana);
        }
        assert_eq!(game.spent_spell_learning, 2);
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

#[test]
fn rejected_prayers_take_no_time_or_rng_and_success_uses_faith() {
    for level in [1, 2] {
        let mut game = at_level(BUILD, level);
        let book_id = book(&mut game, "nature", 1);
        let before = (
            game.world_tick,
            game.player.energy_need,
            game.player_dto(),
            game.rng.clone(),
        );
        let update = dispatch_next(
            &mut game,
            GameCommand::StudyPrayer {
                book_item_id: book_id,
            },
        );
        assert!(
            update
                .events
                .iter()
                .any(|event| event.kind == "ability.study-unavailable")
        );
        assert_eq!(
            (
                game.world_tick,
                game.player.energy_need,
                game.player_dto(),
                game.rng.clone()
            ),
            before
        );
    }
    let mut game = prepared(BUILD, 3);
    let book_id = book(&mut game, "nature", 1);
    if !game
        .virtues
        .iter()
        .any(|virtue| virtue.kind == VirtueKindDto::Faith)
    {
        game.virtues.last_mut().unwrap().kind = VirtueKindDto::Faith;
    }
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    let tick = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::StudyPrayer {
            book_item_id: book_id.clone(),
        },
    );
    assert!(game.world_tick > tick);
    assert_eq!(game.spent_spell_learning, 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Faith), 1);
    assert_eq!(game.virtue_current(VirtueKindDto::Nature), 0);
    assert!(game.ability_learning_remaining(game.casting_profile().unwrap()) > 0);
    let before = (
        game.world_tick,
        game.player.energy_need,
        game.player_dto(),
        game.rng.clone(),
    );
    assert_eq!(
        game.study_random_player_ability(&book_id),
        Err("no-learnable-abilities")
    );
    dispatch_next(
        &mut game,
        GameCommand::StudyPrayer {
            book_item_id: book_id,
        },
    );
    assert_eq!(
        (
            game.world_tick,
            game.player.energy_need,
            game.player_dto(),
            game.rng.clone()
        ),
        before
    );
}

#[test]
fn attribute_and_level_loss_forget_without_refunding_and_resume_after_save() {
    let mut game = prepared(SORCERY_BUILD, 10);
    for realm in ["nature", "sorcery"] {
        let book_id = book(&mut game, realm, 1);
        for _ in 0..3 {
            game.study_random_player_ability(&book_id).unwrap();
        }
    }
    let order = game.ability_learning_order.clone();
    let progress = game.ability_progress.clone();
    let wisdom = game.progress.attributes.wisdom;
    game.progress.attributes.wisdom = 3;
    game.refresh_player_ability_state();
    assert!(game.learned_abilities.is_empty());
    assert_eq!(game.spent_spell_learning, 6);
    assert_eq!(game.ability_learning_order, order);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .filter(|ability| order.contains(&ability.id))
            .all(|ability| ability.forgotten && !ability.can_study && !ability.can_forget)
    );
    game.apply_player_experience_drain(game.progress.experience, "test.drain", &mut Vec::new());
    assert_eq!(game.progress.level, 1);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        game.progress.attributes.wisdom = wisdom;
        game.apply_player_experience(game.experience_required_for_level(10), &mut Vec::new());
        choose_human_talent_if_pending(game);
        game.refresh_player_ability_state();
        assert_eq!(game.learned_abilities, order.iter().cloned().collect());
        assert_eq!(game.ability_progress, progress);
        assert_eq!(game.spent_spell_learning, 6);
        let book_id = book(game, "nature", 1);
        game.study_random_player_ability(&book_id).unwrap();
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn saved_spending_caps_and_progress_reject_forged_repeat_studies() {
    let mut game = prepared(SORCERY_BUILD, 3);
    learn_initial_pair(&mut game);
    let baseline = game.to_save();
    for corruption in 0..7 {
        let mut save = baseline.clone();
        match corruption {
            0 => save.player.spent_spell_learning = 1,
            1 => save.player.spent_spell_learning = 3,
            2 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == SORCERY)
                    .unwrap()
                    .proficiency_cap = 1600
            }
            3 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == SORCERY)
                    .unwrap()
                    .proficiency = 1401
            }
            4 => {
                save.player.ability_progress.pop();
            }
            5 => save.player.ability_learning_order.push(NATURE.to_owned()),
            _ => save.player.mage_realms = None,
        }
        assert!(Game::from_save(save).is_err(), "corruption {corruption}");
    }
    assert!(Game::from_save(baseline).is_ok());
}

#[test]
fn practice_uses_real_depth_and_primary_master_secondary_expert_caps() {
    let mut game = prepared(SORCERY_BUILD, 3);
    learn_initial_pair(&mut game);
    game.apply_player_experience(
        game.experience_required_for_level(50) - game.progress.experience,
        &mut Vec::new(),
    );
    choose_human_talent_if_pending(&mut game);
    game.debug_ability_casts_succeed = true;
    refill(&mut game);
    cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[SORCERY].proficiency, 0);
    assert!(
        game.transition_floor("demo.floor.camelot-depth-24".to_owned(), None, None, false)
            .unwrap()
            .is_some()
    );
    clear_monsters(&mut game);
    refill(&mut game);
    let events = cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. })),
        "{events:?}"
    );
    // L3 spells need ratio <= 60 to train all the way to Master: 2000 / 34 = 58.
    assert_eq!(game.floor_depth(&game.current_floor_id), 24);
    assert_eq!(game.ability_progress[SORCERY].proficiency, 384);
    for (spell, before, after) in [(NATURE, 1599, 1600), (SORCERY, 1399, 1400)] {
        game.ability_progress.get_mut(spell).unwrap().proficiency = before;
        let events = cast(&mut game, spell, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[spell].proficiency, after);
        assert!(events.iter().any(
            |event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution }
            if resolution.proficiency_before == before && resolution.proficiency_after == after)
        ));
        cast(&mut game, spell, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[spell].proficiency, after);
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        game.debug_ability_casts_succeed = false;
        cast(game, SORCERY, TargetSelection::SelfTarget);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn failure_omits_secondary_penalty_and_applies_stun_after_the_minimum() {
    let mut game = at_level(SORCERY_BUILD, 3);
    refill(&mut game);
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    let failure = |game: &Game, spell: &str| {
        let profile = game.casting_profile().unwrap();
        let ability = game.effective_casting_ability(profile, game.content.ability(spell).unwrap());
        game.ability_failure_percent(profile, &ability)
    };
    // Both source entries are L3/fail35; WIS15 subtracts 3 in either realm.
    assert_eq!(failure(&game, NATURE), 32);
    assert_eq!(failure(&game, SORCERY), 32);
    game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
    game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
    game.refresh_player_ability_state();
    learn_initial_pair(&mut game);
    game.apply_player_experience(
        game.experience_required_for_level(50) - game.progress.experience,
        &mut Vec::new(),
    );
    choose_human_talent_if_pending(&mut game);
    refill(&mut game);
    assert_eq!(failure(&game, NATURE), 5);
    assert_eq!(failure(&game, SORCERY), 5);
    let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
    stun.intensity = 40;
    game.player.statuses.push(stun);
    game.ability_progress.get_mut(NATURE).unwrap().proficiency = 1600;
    game.ability_progress.get_mut(SORCERY).unwrap().proficiency = 1400;
    assert_eq!(failure(&game, NATURE), 23);
    assert_eq!(failure(&game, SORCERY), 24);
}

#[test]
fn death_failure_pays_mana_without_practice_and_preserves_backlash_rng_after_save() {
    let spell = "demo.ability.death-berserk"; // Source index 16, first spell in book 3.
    let mut game = prepared(BUILD, 25);
    let book_id = book(&mut game, "death", 3);
    learn_book(&mut game, &book_id);
    assert!(game.learned_abilities.contains(spell));
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    let profile = game.casting_profile().unwrap();
    let ability = game.effective_casting_ability(profile, game.content.ability(spell).unwrap());
    let failure = game.ability_failure_percent(profile, &ability);
    game.rng = (0..100_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            if rng.bounded(100) >= u64::from(failure) {
                return false;
            }
            rng.bounded(100);
            rng.bounded(100); // failure virtues
            if rng.bounded(100) + 1 >= 16 {
                return false;
            }
            for _ in 0..3 {
                rng.bounded(6);
            }
            rng.bounded(6) == 0
        })
        .unwrap();
    let mut expected_rng = game.rng.clone();
    for _ in 0..4 {
        expected_rng.bounded(100);
    }
    let damage: i32 = (0..3).map(|_| (expected_rng.bounded(6) + 1) as i32).sum();
    expected_rng.bounded(6);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let experience = game.progress.experience;
    let mana = game.resources[MANA].current;
    for game in [&mut game, &mut restored] {
        let events = cast(game, spell, TargetSelection::SelfTarget);
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastFailed { resolution }
            if resolution.resource_paid > 0 && resolution.proficiency_after == 0 && resolution.fail_count == 1)));
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::SelfDamage { damage: amount, fatal: false, .. }] if *amount == damage))));
        assert!(game.resources[MANA].current < mana);
        assert_eq!(game.ability_progress[spell].proficiency, 0);
        assert_eq!(game.ability_progress[spell].cast_count, 0);
        assert_eq!(experience - game.progress.experience, 4000);
        assert_eq!(game.rng, expected_rng);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn pending_natures_wrath_saves_and_commits_experience_and_practice_only_once() {
    let spell = "demo.ability.nature-natures-wrath";
    let mut game = prepared(BUILD, 50);
    let book_id = book(&mut game, "nature", 4);
    learn_book(&mut game, &book_id);
    game.transition_floor("demo.floor.camelot-depth-20".to_owned(), None, None, false)
        .unwrap();
    clear_monsters(&mut game);
    refill(&mut game);
    game.debug_ability_casts_succeed = true;
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(6) + 1 == 2
        })
        .unwrap();
    let before = (
        game.resources[MANA].current,
        game.progress.experience,
        game.world_tick,
    );
    dispatch_next(
        &mut game,
        GameCommand::CastAbility {
            ability_id: spell.to_owned(),
            target: TargetSelection::SelfTarget,
        },
    );
    assert!(game.pending_ability_direction.is_some());
    assert_eq!(game.ability_progress[spell].cast_count, 0);
    assert_eq!(game.ability_progress[spell].proficiency, 0);
    assert_eq!(
        (
            game.resources[MANA].current,
            game.progress.experience,
            game.world_tick
        ),
        before
    );
    for corruption in 0..3 {
        let mut invalid = game.to_save();
        let cast = &mut invalid
            .player
            .pending_ability_direction
            .as_mut()
            .unwrap()
            .cast_resolution;
        match corruption {
            0 => cast.proficiency_after = 1600,
            1 => cast.cast_count = 99,
            _ => cast.resource_paid = 0,
        }
        assert!(Game::from_save(invalid).is_err());
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let mut cancelled = restored.clone();
    dispatch_next(&mut cancelled, GameCommand::CancelAbilityDirection);
    assert_eq!(cancelled.ability_progress[spell].cast_count, 0);
    assert_eq!(
        (
            cancelled.resources[MANA].current,
            cancelled.progress.experience,
            cancelled.world_tick
        ),
        before
    );
    for game in [&mut game, &mut restored] {
        game.resolve_pending_ability_direction(
            Direction::East,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.ability_progress[spell].cast_count, 1);
        assert_eq!(game.progress.experience - before.1, 6300);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}
