// SPDX-License-Identifier: MPL-2.0
use super::*;

const DETECT: &str = "demo.ability.sorcery-detect-monsters";
const UNLIFE: &str = "demo.ability.death-detect-unlife";

pub(super) fn prepared(build: &str, level: u16) -> Game {
    let mut game = at_level(build, level);
    game.progress.attributes.intelligence = game.progress.attribute_potentials.intelligence;
    game.progress.maximum_attributes.intelligence = game.progress.attributes.intelligence;
    game.refresh_player_ability_state();
    refill(&mut game);
    game.debug_ability_casts_succeed = true;
    game
}

fn refill(game: &mut Game) {
    let mana = game.resources.get_mut(MANA).unwrap();
    mana.current = mana.maximum;
}

pub(super) fn book_for(game: &mut Game, ability: &str) -> String {
    if let Some(item) = game.ability_book_item_id(game.casting_profile().unwrap(), ability) {
        return item;
    }
    let book_id = game
        .active_casting_book_ids()
        .into_iter()
        .find(|id| {
            game.content
                .ability_book(id)
                .unwrap()
                .ability_ids
                .iter()
                .any(|id| id == ability)
        })
        .unwrap();
    let kind = game
        .content
        .item_definitions()
        .find(|item| item.ability_book_id.as_deref() == Some(book_id))
        .unwrap()
        .id
        .clone();
    let id = format!("test.book.{ability}");
    give_inventory_item(game, &id, &kind);
    id
}

pub(super) fn learn(game: &mut Game, ability: &str) -> String {
    let book = book_for(game, ability);
    game.study_player_ability(&book, ability).unwrap();
    book
}

fn dungeon(game: &mut Game, depth: u16) {
    if game.current_dungeon_instance_id.is_some() {
        let surface = game
            .content
            .world(&game.world_id)
            .unwrap()
            .initial_floor_id
            .clone();
        assert!(
            game.transition_floor(surface, None, None, false)
                .unwrap()
                .is_some()
        );
    }
    let id = if depth == 1 {
        "demo.floor.warrens-depth-1"
    } else {
        "demo.floor.camelot-depth-20"
    };
    assert!(
        game.transition_floor(id.to_owned(), None, None, false)
            .unwrap()
            .is_some()
    );
    clear_monsters(game);
    game.player.position = Position { x: 4, y: 4 };
    for y in 3..=5 {
        for x in 3..=10 {
            let index = game.index(Position { x, y }).unwrap();
            game.terrain[index] = "demo.terrain.floor".to_owned();
        }
    }
    game.set_floor_glow_at(game.player.position, true);
}

#[test]
fn repeated_study_pays_shared_budget_and_keeps_unique_order_and_source_caps() {
    let mut game = prepared(BUILD, 50);
    assert_eq!(game.player_ability_learning_dto().unwrap().capacity, 100);
    let primary = learn(&mut game, UNLIFE);
    let secondary = learn(&mut game, DETECT);
    assert_eq!(game.spent_spell_learning, 2);
    let order = game.ability_learning_order.clone();
    for (book, ability, expected) in [
        (&primary, UNLIFE, &[900, 1200, 1400, 1600][..]),
        (&secondary, DETECT, &[900, 1200, 1400][..]),
    ] {
        for &proficiency in expected {
            let before = game.rng.clone();
            assert!(
                game.snapshot()
                    .player
                    .abilities
                    .iter()
                    .find(|row| row.id == ability)
                    .unwrap()
                    .can_study
            );
            game.study_player_ability(book, ability).unwrap();
            assert_eq!(game.ability_progress[ability].proficiency, proficiency);
            assert_eq!(game.rng, before);
            assert_eq!(game.ability_learning_order, order);
        }
        let before = game.to_save();
        assert_eq!(
            game.study_player_ability(book, ability),
            Err("proficiency-at-cap")
        );
        assert_eq!(game.to_save(), before);
        assert!(
            !game
                .snapshot()
                .player
                .abilities
                .iter()
                .find(|row| row.id == ability)
                .unwrap()
                .can_study
        );
    }
    assert_eq!(game.spent_spell_learning, 9);
    let learning = game.player_ability_learning_dto().unwrap();
    assert_eq!((learning.learned_count, learning.remaining_slots), (2, 91));
    assert_eq!(game.ability_progress[UNLIFE].cast_count, 0);
    assert_eq!(game.ability_progress[DETECT].proficiency_cap, 1400);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn restudy_interpolates_existing_practice_without_reaching_a_second_realm_master() {
    let base = prepared(BUILD, 50);
    for (ability, old, expected) in [
        (UNLIFE, 899, 1199),
        (UNLIFE, 1199, 1399),
        (UNLIFE, 1399, 1599),
        (UNLIFE, 1599, 1600),
        (DETECT, 1399, 1400),
    ] {
        let mut game = base.clone();
        let book = learn(&mut game, ability);
        game.ability_progress.get_mut(ability).unwrap().proficiency = old;
        game.study_player_ability(&book, ability).unwrap();
        assert_eq!(game.ability_progress[ability].proficiency, expected);
        assert_eq!(game.spent_spell_learning, 2);
        assert_eq!(game.ability_learning_order, vec![ability]);
    }
}

#[test]
fn all_study_opportunities_can_be_spent_without_learning_a_hundred_unique_spells() {
    let mut game = prepared(BUILD, 50);
    let (_, ids) = game.player_ability_baseline();
    for ability in ids {
        if game.player_ability_learning_dto().unwrap().remaining_slots == 0 {
            break;
        }
        let book = book_for(&mut game, &ability);
        while game.study_player_ability(&book, &ability).is_ok() {}
    }
    assert_eq!(game.spent_spell_learning, 100);
    assert!(game.learned_abilities.len() < 64);
    assert_eq!(
        game.player_ability_learning_dto().unwrap().remaining_slots,
        0
    );
    let ability = game.ability_learning_order[0].clone();
    let book = book_for(&mut game, &ability);
    let before = game.to_save();
    assert_eq!(
        game.study_player_ability(&book, &ability),
        Err("learning-capacity-full")
    );
    assert_eq!(game.to_save(), before);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn failed_study_does_not_advance_the_world_and_success_costs_a_turn() {
    let mut game = at_level(BUILD, 1);
    let book = book_for(&mut game, DETECT);
    let start = game.world_tick;
    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: book.clone(),
            ability_id: DETECT.to_owned(),
        },
    );
    assert!(game.world_tick > start);
    assert_eq!(game.spent_spell_learning, 1);
    let before = (
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
        game.player_dto(),
    );
    dispatch_next(
        &mut game,
        GameCommand::StudyAbility {
            book_item_id: book,
            ability_id: DETECT.to_owned(),
        },
    );
    assert_eq!(
        (
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
            game.player_dto()
        ),
        before
    );
    assert_eq!(game.spent_spell_learning, 1);
    let before = game.to_save();
    assert_eq!(
        game.forget_player_ability(DETECT),
        Err("manual-forgetting-unavailable")
    );
    assert_eq!(game.to_save(), before);
}

#[test]
fn level_and_attribute_loss_forget_latest_spells_without_refunding_repeat_studies() {
    let mut game = prepared(BUILD, 30);
    let advanced = "demo.ability.death-berserk";
    learn(&mut game, advanced);
    let book = learn(&mut game, UNLIFE);
    learn(&mut game, DETECT);
    game.study_player_ability(&book, UNLIFE).unwrap();
    let order = game.ability_learning_order.clone();
    let progress = game.ability_progress.clone();
    let spent = game.spent_spell_learning;
    game.apply_player_experience_drain(game.progress.experience, "test.drain", &mut Vec::new());
    assert_eq!(game.progress.level, 1);
    assert_eq!(game.learned_abilities, BTreeSet::from([UNLIFE.to_owned()]));
    game.progress.attributes.intelligence = 8;
    game.refresh_player_ability_state();
    assert!(game.learned_abilities.is_empty());
    assert_eq!(game.spent_spell_learning, spent);
    assert_eq!(game.ability_learning_order, order);
    assert_eq!(game.ability_progress, progress);
    let projected = game.snapshot().player.abilities;
    for id in &order {
        let ability = projected.iter().find(|ability| &ability.id == id).unwrap();
        assert!(ability.forgotten);
        assert!(!ability.learned && !ability.can_study);
        assert_eq!(
            ability.book_realm_id.as_deref(),
            Some(if id == DETECT { "sorcery" } else { "death" })
        );
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        game.progress.attributes.intelligence = game.progress.maximum_attributes.intelligence;
        game.apply_player_experience(game.experience_required_for_level(30), &mut Vec::new());
        game.refresh_player_ability_state();
        assert_eq!(game.learned_abilities, order.iter().cloned().collect());
        assert!(
            game.snapshot()
                .player
                .abilities
                .iter()
                .all(|ability| !ability.forgotten)
        );
        assert_eq!(game.ability_progress, progress);
        assert_eq!(game.spent_spell_learning, spent);
        refill(game);
        game.debug_ability_casts_succeed = true;
        cast(game, DETECT, TargetSelection::SelfTarget);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn saved_learning_spend_and_dynamic_caps_are_authoritative_and_validated() {
    let mut game = prepared(BUILD, 20);
    let book = learn(&mut game, DETECT);
    game.study_player_ability(&book, DETECT).unwrap();
    let baseline = game.to_save();
    for corruption in 0..5 {
        let mut save = baseline.clone();
        match corruption {
            0 => save.player.spent_spell_learning = 0,
            1 => save.player.spent_spell_learning = 100,
            2 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == DETECT)
                    .unwrap()
                    .proficiency_cap = 1600
            }
            3 => {
                save.player
                    .ability_progress
                    .iter_mut()
                    .find(|p| p.id == DETECT)
                    .unwrap()
                    .proficiency = 1401
            }
            _ => {
                save.player.ability_progress.pop();
            }
        }
        assert!(Game::from_save(save).is_err(), "corruption {corruption}");
    }
    let mut other = game.clone();
    other.spent_spell_learning -= 1;
    assert_ne!(game.state_hash(), other.state_hash());
    let mut restored = Game::from_save(baseline).unwrap();
    for game in [&mut game, &mut restored] {
        game.study_player_ability(&book, DETECT).unwrap();
        game.debug_ability_casts_succeed = false;
        refill(game);
        cast(game, DETECT, TargetSelection::SelfTarget);
    }
    assert_eq!(game.ability_progress, restored.ability_progress);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn practice_uses_actual_environment_interpolation_and_both_realm_caps() {
    let mut game = prepared(BUILD, 50);
    learn(&mut game, UNLIFE);
    learn(&mut game, DETECT);
    assert!(game.current_town().is_some());
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(100); // failure check
    expected_rng.bounded(100); // successful-cast Chance virtue, even when failure is zero
    cast(&mut game, DETECT, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[DETECT].proficiency, 0);
    assert_eq!(game.rng, expected_rng);
    dungeon(&mut game, 1);
    refill(&mut game);
    cast(&mut game, DETECT, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[DETECT].proficiency, 384);
    game.ability_progress.get_mut(DETECT).unwrap().proficiency = 1010;
    cast(&mut game, DETECT, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[DETECT].proficiency, 1011); // ratio 163
    cast(&mut game, DETECT, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[DETECT].proficiency, 1011);
    dungeon(&mut game, 20);
    for (ability, before, after) in [(DETECT, 1399, 1400), (UNLIFE, 1599, 1600)] {
        game.ability_progress.get_mut(ability).unwrap().proficiency = before;
        refill(&mut game);
        let events = cast(&mut game, ability, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[ability].proficiency, after);
        assert!(events.iter().any(
            |event| matches!(event, DomainEvent::AbilityCastSucceeded { resolution }
            if resolution.proficiency_before == before && resolution.proficiency_after == after)
        ));
        cast(&mut game, ability, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[ability].proficiency, after);
    }
}

#[test]
fn dangerous_wilderness_trains_but_an_easy_floor_cannot_train_a_difficult_spell() {
    let mut game = prepared(BUILD, 50);
    learn(&mut game, DETECT);
    let wilderness = game.wilderness();
    let position = (1..i32::from(wilderness.height) - 1)
        .flat_map(|y| (1..i32::from(wilderness.width) - 1).map(move |x| Position { x, y }))
        .find(|position| {
            game.town_at_wilderness_position(*position).is_none()
                && game.wilderness_danger_level(*position) > 0
        })
        .unwrap();
    game.wilderness_position = Some(position);
    game.activate_wilderness_position(None, false).unwrap();
    clear_monsters(&mut game);
    assert!(game.current_town().is_none());
    cast(&mut game, DETECT, TargetSelection::SelfTarget);
    assert_eq!(game.ability_progress[DETECT].proficiency, 384);
    let difficult = "demo.ability.death-wraithform";
    learn(&mut game, difficult);
    dungeon(&mut game, 1);
    refill(&mut game);
    let before = game.progress.experience;
    let effective = game.effective_casting_ability(
        game.casting_profile().unwrap(),
        game.content.ability(difficult).unwrap(),
    );
    let player = Game::player_ability_parameters(&effective);
    let expected_experience = 250 * 47; // m_info N:1 Death index 31: sexp 250, level 47
    assert_eq!(
        u64::from(player.first_success_experience),
        expected_experience
    );
    let events = cast(&mut game, difficult, TargetSelection::SelfTarget);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
    );
    assert_eq!(game.progress.experience - before, expected_experience);
    assert_eq!(game.ability_progress[difficult].proficiency, 0);
    refill(&mut game);
    cast(&mut game, difficult, TargetSelection::SelfTarget);
    assert_eq!(game.progress.experience - before, expected_experience);
}

#[test]
fn attacks_train_only_against_mobile_hostiles_and_utility_projections_still_train() {
    let bolt = "demo.ability.armageddon-fire-bolt";
    let mut base = prepared("demo.build.mage-armageddon-sorcery", 50);
    learn(&mut base, bolt);
    dungeon(&mut base, 20);
    let immobile = base
        .content
        .actor_definitions()
        .find(|kind| kind.movement.never_moves && !kind.friendly)
        .unwrap()
        .id
        .clone();
    let immune = base
        .content
        .actor_definitions()
        .find(|kind| {
            !kind.movement.never_moves
                && !kind.friendly
                && kind.resistances.get(&ActorDamageType::Fire)
                    == Some(&rfb_content::ActorResistanceLevel::Immune)
        })
        .unwrap()
        .id
        .clone();
    let mut saw_beam = false;
    let mut saw_bolt = false;
    for seed in 0..8 {
        for (kind, friendly, expected) in [
            (None, false, 0),
            (Some("demo.actor.small-kobold"), true, 0),
            (Some(immobile.as_str()), false, 0),
            (Some(immune.as_str()), false, 0),
            (Some("demo.actor.small-kobold"), false, 128),
        ] {
            let mut game = base.clone();
            game.rng = RfbRng::seeded(seed);
            if let Some(kind) = kind {
                let mut actor = actor_from_runtime_spawn(
                    "test.practice.target",
                    kind,
                    Position { x: 5, y: 4 },
                    10_000,
                    100,
                    100,
                    true,
                );
                actor.friendly = friendly;
                actor.resistances =
                    definition_resistance_profile(game.content.actor(kind).unwrap());
                game.entities.push(actor);
            }
            let events = cast(
                &mut game,
                bolt,
                TargetSelection::Direction {
                    direction: Direction::East,
                },
            );
            let beam = events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityBeamDamage { .. }));
            saw_beam |= beam;
            saw_bolt |= !beam;
            assert!(
                events
                    .iter()
                    .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. })),
                "{events:?}"
            );
            assert_eq!(
                game.ability_progress[bolt].proficiency, expected,
                "{kind:?} friendly={friendly}: {events:?}"
            );
            assert_eq!(game.ability_progress[bolt].cast_count, 1); // a useless attack still sets worked
        }
    }
    assert!(saw_beam && saw_bolt);
    let utility = "demo.ability.sorcery-light-area";
    learn(&mut base, utility);
    cast(&mut base, utility, TargetSelection::SelfTarget);
    assert_eq!(base.ability_progress[utility].proficiency, 384);
}

#[test]
fn secondary_failure_penalty_precedes_minimum_and_stun_and_expert_adjustments() {
    let mut primary = at_level("demo.build.mage-sorcery-death", 1);
    let mut secondary = at_level(BUILD, 1);
    for game in [&mut primary, &mut secondary] {
        learn(game, DETECT);
        refill(game);
        for virtue in &mut game.virtues {
            virtue.value = 0;
        }
    }
    let failure = |game: &Game| {
        let profile = game.casting_profile().unwrap();
        let spell = game.effective_casting_ability(profile, game.content.ability(DETECT).unwrap());
        game.ability_failure_percent(profile, &spell)
    };
    assert_eq!(failure(&secondary), failure(&primary) + 5);
    for game in [&mut primary, &mut secondary] {
        game.progress.attributes.intelligence = 238;
        game.progress.maximum_attributes.intelligence = 238;
        game.refresh_player_ability_state();
        refill(game);
    }
    assert_eq!(failure(&primary), 0);
    assert_eq!(failure(&secondary), 0); // +5 cannot bypass the minimum ordering
    let mut stun = monster_combat::melee_status(STATUS_STUN, 100, "test.stun").status;
    stun.intensity = 40;
    secondary.player.statuses.push(stun);
    assert_eq!(failure(&secondary), 20);
    secondary
        .ability_progress
        .get_mut(DETECT)
        .unwrap()
        .proficiency = 1400;
    assert_eq!(failure(&secondary), 19);
    primary
        .ability_progress
        .get_mut(DETECT)
        .unwrap()
        .proficiency = 1600;
    assert_eq!(failure(&primary), 0);
}

fn failure_rate(game: &Game, ability: &str) -> u8 {
    let profile = game.casting_profile().unwrap();
    let ability = game.effective_casting_ability(profile, game.content.ability(ability).unwrap());
    game.ability_failure_percent(profile, &ability)
}

#[test]
fn death_failure_pays_mana_and_rolls_book_damage_then_hold_life_before_experience_loss() {
    let spell = "demo.ability.death-berserk"; // Death index 16, third book
    let mut base = prepared(BUILD, 10);
    learn(&mut base, spell);
    for virtue in &mut base.virtues {
        virtue.value = 0;
    }
    base.debug_ability_casts_succeed = false;
    let failure = failure_rate(&base, spell);
    assert!(failure > 0);
    base.rng = (0..100_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            if rng.bounded(100) >= u64::from(failure) {
                return false;
            }
            rng.bounded(100);
            rng.bounded(100); // virtue checks
            if rng.bounded(100) + 1 >= 16 {
                return false;
            }
            for _ in 0..3 {
                rng.bounded(6);
            }
            rng.bounded(6) == 0
        })
        .unwrap();
    let mut expected_rng = base.rng.clone();
    for _ in 0..4 {
        expected_rng.bounded(100);
    }
    let damage: i32 = (0..3).map(|_| (expected_rng.bounded(6) + 1) as i32).sum();
    expected_rng.bounded(6);
    for hold_life in [false, true] {
        let mut game = base.clone();
        if hold_life {
            game.player
                .statuses
                .push(monster_combat::melee_status(STATUS_HOLD_LIFE, 100, "test.hold-life").status);
        }
        let experience = game.progress.experience;
        let hp = game.player.hp;
        let events = cast(&mut game, spell, TargetSelection::SelfTarget);
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityCastFailed { resolution }
            if resolution.resource_paid > 0 && resolution.proficiency_after == 0 && resolution.fail_count == 1)));
        assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. }
            if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::SelfDamage { damage: amount, fatal: false, .. }] if *amount == damage))), "expected damage {damage}: {events:?}");
        assert_eq!(game.ability_progress[spell].cast_count, 0);
        assert_eq!(
            experience - game.progress.experience,
            if hold_life { 0 } else { 4000.min(experience) }
        );
        if hold_life {
            assert_eq!(hp - game.player.hp, damage);
        }
        assert_eq!(game.rng, expected_rng);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
}

#[test]
fn fourth_death_book_can_blast_sanity_and_save_continuation_keeps_its_rng() {
    let spell = "demo.ability.death-wraithform";
    let mut game = prepared(BUILD, 47);
    learn(&mut game, spell);
    game.debug_ability_casts_succeed = false;
    let failure = failure_rate(&game, spell);
    assert!(failure > 0);
    game.rng = (0..100_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            if rng.bounded(100) >= u64::from(failure) {
                return false;
            }
            rng.bounded(100);
            rng.bounded(100);
            rng.bounded(100) + 1 < 31 && rng.bounded(2) == 0
        })
        .unwrap();
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let hp = game.player.hp;
    let experience = game.progress.experience;
    let events = cast(&mut game, spell, TargetSelection::SelfTarget);
    let continued = cast(&mut restored, spell, TargetSelection::SelfTarget);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::SpellSanityBlasted { ability_id, outcome: "mind-blast" } if ability_id == spell)));
    assert_eq!(game.player.hp, hp);
    assert_eq!(game.progress.experience, experience);
    assert_eq!(game.ability_progress[spell].proficiency, 0);
    assert_eq!(game.ability_progress[spell].cast_count, 0);
    assert_eq!(events, continued);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
    assert!(Game::from_save(game.to_save()).is_ok());
}

#[test]
fn first_death_spell_never_backlashes_but_keeps_all_failure_draws() {
    let mut game = at_level(BUILD, 1);
    learn(&mut game, UNLIFE);
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) < u64::from(failure_rate(&game, UNLIFE)))
        .unwrap();
    let mut expected_rng = game.rng.clone();
    for _ in 0..4 {
        expected_rng.bounded(100);
    }
    let hp = game.player.hp;
    let events = cast(&mut game, UNLIFE, TargetSelection::SelfTarget);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        DomainEvent::AbilityEffectsResolved { .. } | DomainEvent::SpellSanityBlasted { .. }
    )));
    assert_eq!(game.rng, expected_rng);
    assert_eq!(game.player.hp, hp);
    assert_eq!(game.ability_progress[UNLIFE].cast_count, 0);
    refill(&mut game);
    game.debug_ability_casts_succeed = true;
    let experience = game.progress.experience;
    cast(&mut game, UNLIFE, TargetSelection::SelfTarget);
    assert_eq!(game.progress.experience - experience, 4);
    assert_eq!(game.ability_progress[UNLIFE].cast_count, 1);
}

#[test]
fn glyph_limit_rejects_secondary_life_atomically_and_exempts_primary_life() {
    let glyph = "demo.ability.life-glyph-of-warding";
    for (build, success) in [
        ("demo.build.mage-sorcery-life", false),
        ("demo.build.mage-life-sorcery", true),
        ("demo.build.warrior-mage-arcane-life", false),
    ] {
        let mut game = prepared(build, 50);
        learn(&mut game, glyph);
        dungeon(&mut game, 20);
        let positions: Vec<_> = game
            .terrain
            .iter()
            .enumerate()
            .filter(|(index, id)| {
                id.as_str() == "demo.terrain.floor"
                    && Some(*index) != game.index(game.player.position)
            })
            .take(11)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(positions.len(), 11);
        for index in positions {
            game.terrain[index] = "demo.terrain.warding-glyph".to_owned();
        }
        refill(&mut game);
        let before = game.to_save();
        let events = cast(&mut game, glyph, TargetSelection::SelfTarget);
        if success {
            assert_eq!(
                game.terrain
                    .iter()
                    .filter(|id| id.as_str() == "demo.terrain.warding-glyph")
                    .count(),
                12
            );
            assert_eq!(game.ability_progress[glyph].cast_count, 1);
        } else {
            assert_eq!(game.to_save(), before);
            assert!(matches!(
                events.as_slice(),
                [DomainEvent::AbilityTargetUnavailable { .. }]
            ));
            let index = game
                .terrain
                .iter()
                .position(|id| id == "demo.terrain.warding-glyph")
                .unwrap();
            game.terrain[index] = "demo.terrain.floor".to_owned();
            cast(&mut game, glyph, TargetSelection::SelfTarget);
            assert_eq!(
                game.terrain
                    .iter()
                    .filter(|id| id.as_str() == "demo.terrain.warding-glyph")
                    .count(),
                11
            );
        }
    }
}

#[test]
fn pending_natures_wrath_does_not_work_or_train_until_direction_commits() {
    let spell = "demo.ability.nature-natures-wrath";
    let mut game = prepared("demo.build.mage-nature-sorcery", 50);
    learn(&mut game, spell);
    dungeon(&mut game, 20);
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(6) + 1 == 2
        })
        .unwrap();
    let mana = game.resources[MANA].current;
    let experience = game.progress.experience;
    let world_tick = game.world_tick;
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
    assert_eq!(game.resources[MANA].current, mana);
    assert_eq!(game.progress.experience, experience);
    assert_eq!(game.world_tick, world_tick);
    for corruption in 0..3 {
        let mut save = game.to_save();
        let cast = &mut save
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
        assert!(Game::from_save(save).is_err());
    }
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let mut cancelled = restored.clone();
    dispatch_next(&mut cancelled, GameCommand::CancelAbilityDirection);
    assert_eq!(cancelled.ability_progress[spell].cast_count, 0);
    assert_eq!(cancelled.progress.experience, experience);
    assert_eq!(cancelled.world_tick, world_tick);
    for game in [&mut game, &mut restored] {
        game.resolve_pending_ability_direction(
            Direction::East,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert_eq!(game.ability_progress[spell].cast_count, 1);
        assert_eq!(game.ability_progress[spell].proficiency, 0); // no hostile target
        assert_eq!(game.progress.experience - experience, 40 * 150);
        assert!(game.resources[MANA].current < mana);
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn mana_reduction_rounds_once_after_proficiency_for_all_book_casters() {
    for build in [
        BUILD,
        "demo.build.high-mage-death",
        "demo.build.paladin-death",
    ] {
        let mut game = at_level(build, 30);
        give_inventory_item(&mut game, "test.reduction", "demo.item.brass-lantern");
        let slot_id = game
            .body_slots
            .iter()
            .find(|slot| slot.slot_type == "light")
            .unwrap()
            .id
            .clone();
        let item = game.items.last_mut().unwrap();
        item.location = ItemLocation::Equipped { slot_id };
        item.intrinsic_properties
            .passives
            .insert(EquipmentPassive::ReducedManaCost);
        let profile = game.casting_profile().unwrap();
        let ability =
            game.effective_casting_ability(profile, game.content.ability(UNLIFE).unwrap());
        // Base 1 and initial skill: floor((3800 + 2399) * 3 / 9600) = 1.
        assert_eq!(
            game.ability_effective_resource_cost(&ability, game.ability_progress[UNLIFE]),
            1
        );
        let mut ability = ability.clone();
        ability.player.as_mut().unwrap().resource_cost = 3;
        // Rounding before applying 3/4 would incorrectly produce 3.
        assert_eq!(
            game.ability_effective_resource_cost(&ability, game.ability_progress[UNLIFE]),
            4
        );
    }
}

#[test]
fn study_virtue_uses_class_book_while_alignment_uses_the_active_realm() {
    for (build, spell, virtue) in [
        (BUILD, UNLIFE, VirtueKindDto::Knowledge),
        (
            "demo.build.mage-life-sorcery",
            "demo.ability.life-cure-light-wounds",
            VirtueKindDto::Knowledge,
        ),
        (
            "demo.build.mage-nature-sorcery",
            "demo.ability.nature-detect-creatures",
            VirtueKindDto::Knowledge,
        ),
    ] {
        let mut game = prepared(build, 20);
        let before = game.virtue_current(virtue);
        learn(&mut game, spell);
        assert_eq!(game.virtue_current(virtue), before + 1);
    }
    let mut game = prepared(BUILD, 20);
    for virtue in &mut game.virtues {
        virtue.value = 0;
    }
    let unlife = game
        .virtues
        .iter_mut()
        .find(|v| v.kind == VirtueKindDto::Unlife)
        .unwrap();
    unlife.value = -21;
    assert_eq!(game.book_spell_alignment_modifier(UNLIFE), 1);
    assert_eq!(game.book_spell_alignment_modifier(DETECT), 0);
}
