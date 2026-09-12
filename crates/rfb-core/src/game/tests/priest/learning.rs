// SPDX-License-Identifier: MPL-2.0
use super::*;

pub(super) const LIFE: &str = "demo.ability.life-cure-light-wounds";
pub(super) const SORCERY: &str = "demo.ability.sorcery-detect-monsters";

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

pub(super) fn cast(game: &mut Game, spell: &str, target: TargetSelection) -> Vec<DomainEvent> {
    refill(game);
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

fn failure(game: &Game, spell: &str) -> u8 {
    let profile = game.casting_profile().unwrap();
    let ability = game.effective_casting_ability(profile, game.content.ability(spell).unwrap());
    game.ability_failure_percent(profile, &ability)
}

#[test]
fn all_primary_realms_randomly_learn_and_cast_with_shared_spending_and_source_order() {
    for (first, first_spell, directed) in [
        ("life", LIFE, false),
        ("crusade", "demo.ability.crusade-punishment", true),
        ("death", "demo.ability.death-detect-unlife", false),
        ("daemon", "demo.ability.daemon-magic-missile", true),
    ] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 50);
        game.debug_ability_casts_succeed = true;
        for (realm, spell, directed, cap) in [
            (first, first_spell, directed, 1600),
            ("sorcery", SORCERY, false, 1400),
        ] {
            let book_id = book(&mut game, realm, 1);
            // At L50 every first-book slot is eligible. Keep the first of the
            // source's eight one_in_(k) draws, including the bound-one draw.
            game.rng = (0..1000)
                .map(RfbRng::seeded)
                .find(|rng| {
                    let mut rng = rng.clone();
                    rng.bounded(1);
                    (2..=8).all(|k| rng.bounded(k) != 0)
                })
                .unwrap();
            let mut expected_rng = game.rng.clone();
            for k in 1..=8 {
                expected_rng.bounded(k);
            }
            let faith = game.virtue_current(VirtueKindDto::Faith);
            let spent = game.spent_spell_learning;
            assert_eq!(game.study_random_player_ability(&book_id).unwrap(), spell);
            assert_eq!(game.rng, expected_rng);
            assert_eq!(game.spent_spell_learning, spent + 1);
            assert_eq!(game.virtue_current(VirtueKindDto::Faith), faith + 1);
            assert_eq!(game.ability_progress[spell].proficiency_cap, cap);
            let projected = game
                .snapshot()
                .player
                .abilities
                .into_iter()
                .find(|a| a.id == spell)
                .unwrap();
            assert!(!projected.can_study && !projected.can_forget);
            let before = game.to_save();
            assert_eq!(
                game.study_player_ability(&book_id, spell),
                Err("study-mode-mismatch")
            );
            assert_eq!(
                game.forget_player_ability(spell),
                Err("manual-forgetting-unavailable")
            );
            assert_eq!(game.to_save(), before);
            let target = if directed {
                TargetSelection::Direction {
                    direction: Direction::East,
                }
            } else {
                TargetSelection::SelfTarget
            };
            assert!(
                cast(&mut game, spell, target)
                    .iter()
                    .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
            );
            assert_eq!(game.ability_progress[spell].cast_count, 1);
        }
        assert_eq!(game.spent_spell_learning, 2);
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
    }
}

#[test]
fn secondary_failure_adds_five_before_minimum_and_stun_then_practice_reduces_it() {
    let spell = "demo.ability.life-regeneration";
    let mut primary = at_level(BUILD, 1);
    let mut secondary = at_level("demo.build.priest-crusade-life", 1);
    for game in [&mut primary, &mut secondary] {
        for virtue in &mut game.virtues {
            virtue.value = 0;
        }
    }
    // Source L1/fail20 with WIS16: 20 - 3, plus five only in the secondary realm.
    assert_eq!(failure(&primary, spell), 17);
    assert_eq!(failure(&secondary, spell), 22);
    for game in [&mut primary, &mut secondary] {
        game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
        choose_human_talent_if_pending(game);
        game.progress.attributes.wisdom = crate::stats::PRE_VICTORY_ATTRIBUTE_CAP;
        game.refresh_player_ability_state();
        refill(game);
        assert_eq!(failure(game, spell), 0);
        let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
        stun.intensity = 40;
        game.player.statuses.push(stun);
        let progress = game.ability_progress.get_mut(spell).unwrap();
        progress.proficiency = progress.proficiency_cap;
    }
    assert_eq!(failure(&primary, spell), 18);
    assert_eq!(failure(&secondary, spell), 19);
}

#[test]
fn practice_caps_and_failed_mana_payment_continue_identically_after_save() {
    let mut game = prepared(BUILD, 50);
    for realm in ["life", "sorcery"] {
        let id = book(&mut game, realm, 1);
        for _ in 0..8 {
            game.study_random_player_ability(&id).unwrap();
        }
    }
    game.debug_ability_casts_succeed = true;
    cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[SORCERY].proficiency, 0);
    game.transition_floor("demo.floor.camelot-depth-24".to_owned(), None, None, false)
        .unwrap();
    clear_monsters(&mut game);
    cast(&mut game, SORCERY, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[SORCERY].proficiency, 384);
    for (spell, before, after) in [(LIFE, 1599, 1600), (SORCERY, 1399, 1400)] {
        game.ability_progress.get_mut(spell).unwrap().proficiency = before;
        let events = cast(&mut game, spell, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[spell].proficiency, after);
        assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { resolution } if resolution.proficiency_before == before && resolution.proficiency_after == after)));
    }
    let spent = game.spent_spell_learning;
    let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
    stun.intensity = 40;
    game.player.statuses.push(stun);
    game.debug_ability_casts_succeed = false;
    refill(&mut game);
    let fail = failure(&game, SORCERY);
    assert!(fail > 0);
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) < u64::from(fail))
        .unwrap();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let before = game.ability_progress[SORCERY];
    let mana = game.resources[MANA].current;
    for game in [&mut game, &mut restored] {
        let events = cast(game, SORCERY, TargetSelection::SelfTarget);
        assert!(events.iter().any(|e| matches!(e, DomainEvent::AbilityCastFailed { resolution } if resolution.resource_paid > 0)));
        assert!(game.resources[MANA].current < mana);
        assert_eq!(
            game.ability_progress[SORCERY].proficiency,
            before.proficiency
        );
        assert_eq!(
            game.ability_progress[SORCERY].fail_count,
            before.fail_count + 1
        );
        assert_eq!(game.spent_spell_learning, spent);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn lost_wisdom_and_levels_forget_without_refund_and_resume_after_save() {
    for first in ["life", "death"] {
        let mut game = prepared(&format!("demo.build.priest-{first}-sorcery"), 10);
        for realm in [first, "sorcery"] {
            let id = book(&mut game, realm, 1);
            for _ in 0..3 {
                game.study_random_player_ability(&id).unwrap();
            }
        }
        let order = game.ability_learning_order.clone();
        let progress = game.ability_progress.clone();
        let wisdom = game.progress.attributes.wisdom;
        game.progress.attributes.wisdom = 3;
        game.refresh_player_ability_state();
        assert!(game.learned_abilities.is_empty());
        assert_eq!(game.spent_spell_learning, 6);
        assert!(
            game.snapshot()
                .player
                .abilities
                .iter()
                .filter(|a| order.contains(&a.id))
                .all(|a| a.forgotten && !a.can_study && !a.can_forget)
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
            let id = book(game, first, 1);
            game.study_random_player_ability(&id).unwrap();
        }
        assert_eq!(game.state_hash(), restored.state_hash());
        assert_eq!(game.rng, restored.rng);
    }
}

#[test]
fn saves_reject_repeat_study_spending_and_inconsistent_spell_progress() {
    let mut game = prepared(BUILD, 12);
    for realm in ["life", "sorcery"] {
        let id = book(&mut game, realm, 1);
        game.study_random_player_ability(&id).unwrap();
    }
    let primary = game.ability_learning_order[0].clone();
    let secondary = game.ability_learning_order[1].clone();
    // Practising past Beginner does not authorize another paid study.
    game.ability_progress.get_mut(&primary).unwrap().proficiency = 900;
    let baseline = game.to_save();
    assert!(Game::from_save(baseline.clone()).is_ok());
    for corruption in 0..7 {
        let mut save = baseline.clone();
        match corruption {
            0 => save.player.spent_spell_learning = 1,
            1 => save.player.spent_spell_learning = 3,
            2 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == secondary)
                    .unwrap()
                    .proficiency_cap = 1600
            }
            3 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == secondary)
                    .unwrap()
                    .proficiency = 1401
            }
            4 => {
                save.player.ability_progress.pop();
            }
            5 => save.player.ability_learning_order.push(primary.clone()),
            _ => save.player.mage_realms = None,
        }
        assert!(Game::from_save(save).is_err(), "corruption {corruption}");
    }
}
