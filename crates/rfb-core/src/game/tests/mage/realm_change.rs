// SPDX-License-Identifier: MPL-2.0
use super::learning::{book_for, learn, prepared};
use super::*;
use rfb_protocol::{AutoGetModeDto, FacilityMembershipDto, LocaleDto};

const PRIMARY: &str = "demo.ability.death-detect-unlife";
const SECONDARY: &str = "demo.ability.sorcery-detect-monsters";
const NATURE: &str = "demo.ability.nature-detect-creatures";

fn give_book(game: &mut Game, realm: &str) -> String {
    let book_id = game
        .content
        .item_definitions()
        .filter_map(|item| {
            item.ability_book_id
                .as_deref()
                .and_then(|id| game.content.ability_book(id))
        })
        .find(|book| book.realm_id.as_deref() == Some(realm) && book.rank == Some(1))
        .unwrap()
        .id
        .clone();
    let kind = game
        .content
        .item_definitions()
        .find(|item| item.ability_book_id.as_ref() == Some(&book_id))
        .unwrap()
        .id
        .clone();
    let id = format!("test.change.{realm}");
    give_inventory_item(game, &id, &kind);
    id
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

fn command_error(game: &mut Game, command: GameCommand) -> CoreError {
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
fn carried_and_floor_books_prompt_without_paying_and_rejection_preserves_the_old_realm() {
    for ground in [false, true] {
        let mut game = prepared(BUILD, 30);
        learn(&mut game, SECONDARY);
        let book = give_book(&mut game, "nature");
        if ground {
            game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
        }
        let realms = game.mage_realms.clone();
        let progress = game.ability_progress.clone();
        let order = game.ability_learning_order.clone();
        let clock = (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone(),
        );
        begin(&mut game, &book);
        let projection = game
            .snapshot()
            .player
            .ability_learning
            .unwrap()
            .realms
            .unwrap();
        assert_eq!(projection.second_realm_id, "sorcery");
        assert_eq!(projection.pending_change.unwrap().realm_id, "nature");
        assert!(matches!(
            command_error(&mut game, GameCommand::Wait),
            CoreError::RealmChangeRequired
        ));
        let mut restored = Game::from_save(game.to_save()).unwrap();
        for game in [&mut game, &mut restored] {
            confirm(game, false);
            assert_eq!(game.mage_realms, realms);
            assert_eq!(game.ability_progress, progress);
            assert_eq!(game.ability_learning_order, order);
            assert_eq!(game.spent_spell_learning, 1);
            assert_eq!(
                (
                    game.turn,
                    game.world_tick,
                    game.player.energy_need,
                    game.rng.clone()
                ),
                clock
            );
        }
        assert_eq!(game.state_hash(), restored.state_hash());
    }
}

#[test]
fn confirmation_clears_only_secondary_progress_without_refunding_and_can_stop_before_study() {
    let mut game = prepared(BUILD, 40);
    let first = learn(&mut game, PRIMARY);
    game.study_player_ability(&first, PRIMARY).unwrap();
    let second = learn(&mut game, SECONDARY);
    game.study_player_ability(&second, SECONDARY).unwrap();
    cast(&mut game, SECONDARY, TargetSelection::SelfTarget);
    let primary = game.ability_progress[PRIMARY];
    let spent = game.spent_spell_learning;
    let build = game.build.clone();
    let virtues = game.virtues;
    let resources = game.resources.clone();
    let clock = (
        game.turn,
        game.world_tick,
        game.player.energy_need,
        game.rng.clone(),
    );
    let book = give_book(&mut game, "nature");
    begin(&mut game, &book);
    confirm(&mut game, true);
    assert_eq!(game.current_second_realm_id(), Some("nature"));
    assert_eq!(game.build, build);
    assert_eq!(game.ability_learning_order, [PRIMARY]);
    assert_eq!(game.ability_progress[PRIMARY], primary);
    assert!(!game.ability_progress.contains_key(SECONDARY));
    assert_eq!(game.ability_progress[NATURE].cast_count, 0);
    assert_eq!(game.ability_progress[NATURE].proficiency, 0);
    assert_eq!(game.ability_progress[NATURE].proficiency_cap, 1400);
    assert_eq!(game.spent_spell_learning, spent);
    assert_eq!(game.resources, resources);
    assert_eq!(game.virtues, virtues);
    assert_eq!(
        (
            game.turn,
            game.world_tick,
            game.player.energy_need,
            game.rng.clone()
        ),
        clock
    );
    // Cancelling the spell picker needs no rollback command: the new realm is already committed.
    assert!(game.pending_realm_change_book().is_none());
    let mut restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.current_second_realm_id(), Some("nature"));
    assert!(!restored.learned_abilities.contains(NATURE));
    dispatch_next(&mut restored, GameCommand::Wait);
    assert_eq!(restored.current_second_realm_id(), Some("nature"));
    assert_eq!(
        game.study_player_ability(&second, SECONDARY),
        Err("ability-not-supported")
    );
}

#[test]
fn returning_to_a_former_realm_starts_fresh_and_save_continuation_keeps_rng_and_generated_items() {
    let mut game = prepared(BUILD, 40);
    learn(&mut game, PRIMARY);
    let second_book = learn(&mut game, SECONDARY);
    game.study_player_ability(&second_book, SECONDARY).unwrap();
    cast(&mut game, SECONDARY, TargetSelection::SelfTarget);
    let nature_book = give_book(&mut game, "nature");
    begin(&mut game, &nature_book);
    let mut restored = Game::from_save(game.to_save()).unwrap();
    let mut generated = Vec::new();
    for game in [&mut game, &mut restored] {
        game.debug_ability_casts_succeed = true;
        confirm(game, true);
        dispatch_next(
            game,
            GameCommand::StudyAbility {
                book_item_id: nature_book.clone(),
                ability_id: NATURE.to_owned(),
            },
        );
        let events = cast(game, NATURE, TargetSelection::SelfTarget);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
        );
        begin(game, &second_book);
        confirm(game, true);
        assert_eq!(game.ability_progress[SECONDARY].proficiency, 0);
        assert_eq!(game.ability_progress[SECONDARY].cast_count, 0);
        assert_eq!(game.ability_progress[SECONDARY].fail_count, 0);
        assert!(!game.learned_abilities.contains(SECONDARY));
        assert!(!game.ability_progress.contains_key(NATURE));
        assert_eq!(
            game.mage_realms.as_ref().unwrap().previous_realm_ids,
            ["nature", "sorcery"]
        );
        assert_eq!(game.spent_spell_learning, 4);
        dispatch_next(
            game,
            GameCommand::StudyAbility {
                book_item_id: second_book.clone(),
                ability_id: SECONDARY.to_owned(),
            },
        );
        let experience = game.progress.experience;
        cast(game, SECONDARY, TargetSelection::SelfTarget);
        assert_eq!(game.ability_progress[SECONDARY].cast_count, 1);
        assert!(game.progress.experience > experience);
        let context = LootContext {
            table_id: "demo.loot-table.base-items".to_owned(),
            floor_id: "demo.floor.camelot-depth-20".to_owned(),
            depth: 20,
            source: LootSource::ItemUse {
                item_id: "test.generated".to_owned(),
            },
        };
        let items = game
            .generate_loot_instances_internal(
                &context,
                ItemLocation::Inventory,
                false,
                Some(1),
                ItemGenerationMode::TailoredGreat,
            )
            .unwrap();
        assert!(!items.is_empty());
        generated.push(crate::save::inventory_to_save(&items));
        game.items.extend(items);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
    assert_eq!(generated[0], generated[1]);
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}

#[test]
fn invalid_books_status_and_budget_cannot_begin_a_realm_change() {
    let mut game = prepared(BUILD, 50);
    let book = give_book(&mut game, "nature");
    let first = learn(&mut game, PRIMARY);
    let second = learn(&mut game, SECONDARY);
    for id in ["missing", &first, &second] {
        assert!(matches!(
            command_error(
                &mut game,
                GameCommand::BeginRealmChange {
                    book_item_id: id.to_owned()
                }
            ),
            CoreError::RealmChangeUnavailable(_)
        ));
    }
    game.items
        .iter_mut()
        .find(|item| item.id == book)
        .unwrap()
        .location = ItemLocation::Ground(Position { x: 0, y: 0 });
    command_error(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: book.clone(),
        },
    );
    game.items
        .iter_mut()
        .find(|item| item.id == book)
        .unwrap()
        .location = ItemLocation::Inventory;
    for kind in [STATUS_BLINDNESS, STATUS_CONFUSION] {
        game.player
            .statuses
            .push(monster_combat::melee_status(kind, 100, "test.status").status);
        command_error(
            &mut game,
            GameCommand::BeginRealmChange {
                book_item_id: book.clone(),
            },
        );
        game.player.statuses.pop();
    }
    let (_, abilities) = game.player_ability_baseline();
    for ability in abilities {
        if game.ability_learning_remaining(game.casting_profile().unwrap()) == 0 {
            break;
        }
        let id = book_for(&mut game, &ability);
        while game.study_player_ability(&id, &ability).is_ok() {}
    }
    assert_eq!(game.spent_spell_learning, 100);
    assert!(matches!(
        command_error(
            &mut game,
            GameCommand::BeginRealmChange { book_item_id: book }
        ),
        CoreError::RealmChangeUnavailable("learning-capacity-full")
    ));
    Game::from_save(game.to_save()).unwrap();
    let mut other = at_level("demo.build.high-mage-death", 30);
    let book = give_book(&mut other, "nature");
    assert!(matches!(
        command_error(
            &mut other,
            GameCommand::BeginRealmChange { book_item_id: book }
        ),
        CoreError::RealmChangeUnavailable("class-unavailable")
    ));
}

#[test]
fn replacing_a_forgotten_realm_can_exhaust_budget_before_selecting_a_spell() {
    let mut game = prepared(BUILD, 30);
    let primary_book = learn(&mut game, PRIMARY);
    let advanced = game.active_casting_realm_profiles()[1]
        .ability_overrides
        .iter()
        .find(|spell| (10..=30).contains(&spell.minimum_level))
        .unwrap()
        .ability_id
        .clone();
    learn(&mut game, &advanced);
    let book = give_book(&mut game, "nature");
    let mut low_level = game.clone();
    low_level.apply_player_experience_drain(
        low_level.progress.experience,
        "test.drain",
        &mut Vec::new(),
    );
    let capacity = low_level.ability_learning_capacity(low_level.casting_profile().unwrap());
    while game.spent_spell_learning < u32::from(capacity) {
        game.study_player_ability(&primary_book, PRIMARY).unwrap();
    }
    game.apply_player_experience_drain(game.progress.experience, "test.drain", &mut Vec::new());
    assert!(!game.learned_abilities.contains(&advanced));
    assert!(game.ability_learning_order.contains(&advanced));
    assert_eq!(
        game.ability_learning_remaining(game.casting_profile().unwrap()),
        1
    );
    let primary = game.ability_progress[PRIMARY];
    begin(&mut game, &book);
    confirm(&mut game, true);
    assert_eq!(
        game.ability_learning_remaining(game.casting_profile().unwrap()),
        0
    );
    assert_eq!(game.current_second_realm_id(), Some("nature"));
    assert_eq!(
        game.study_player_ability(&book, NATURE),
        Err("learning-capacity-full")
    );
    assert_eq!(game.ability_progress[PRIMARY], primary);
    assert!(!game.ability_progress.contains_key(&advanced));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    restored.apply_player_experience(restored.experience_required_for_level(30), &mut Vec::new());
    assert!(!restored.ability_progress.contains_key(&advanced));
    assert_eq!(restored.spent_spell_learning, u32::from(capacity));
}

#[test]
fn saved_current_realms_history_pending_books_and_progress_are_validated() {
    let mut game = prepared(BUILD, 30);
    learn(&mut game, PRIMARY);
    learn(&mut game, SECONDARY);
    let old_progress = game
        .to_save()
        .player
        .ability_progress
        .into_iter()
        .find(|p| p.id == SECONDARY)
        .unwrap();
    let book = give_book(&mut game, "nature");
    begin(&mut game, &book);
    let mut missing_book = game.to_save();
    missing_book
        .player
        .mage_realms
        .as_mut()
        .unwrap()
        .pending_change_book_item_id = Some("missing".to_owned());
    assert!(Game::from_save(missing_book).is_err());
    confirm(&mut game, true);
    learn(&mut game, NATURE);
    let baseline = game.to_save();
    for corruption in 0..12 {
        let mut save = baseline.clone();
        let realms = save.player.mage_realms.as_mut().unwrap();
        match corruption {
            0 => realms.second_realm_id = "death".to_owned(),
            1 => realms.second_realm_id = "unknown".to_owned(),
            2 => realms.second_realm_id = "chaos".to_owned(),
            3 => realms.previous_realm_ids.clear(),
            4 => realms.previous_realm_ids = vec!["death".to_owned()],
            5 => realms.previous_realm_ids.push("sorcery".to_owned()),
            6 => save.player.mage_realms = None,
            7 => save.player.ability_progress.push(old_progress.clone()),
            8 => save
                .player
                .ability_learning_order
                .push(SECONDARY.to_owned()),
            9 => save.player.spent_spell_learning = 0,
            10 => save.player.spent_spell_learning = 1000,
            _ => realms.pending_change_book_item_id = Some(book.clone()),
        }
        assert!(Game::from_save(save).is_err(), "corruption {corruption}");
    }
    assert!(Game::from_save(baseline).is_ok());
}

#[test]
fn guild_membership_and_prices_follow_current_realms_without_changing_primary_or_birth_virtues() {
    let mut game = prepared(BUILD, 30);
    let sorcery = game
        .content
        .town_facility("demo.town-facility.morivant-sorcery-tower")
        .unwrap()
        .clone();
    let life = game
        .content
        .town_facility("demo.town-facility.telmora-life-temple")
        .unwrap()
        .clone();
    let price = sorcery.identify_all_items_cost.unwrap();
    assert_eq!(
        game.town_facility_membership(&sorcery),
        FacilityMembershipDto::Owner
    );
    let old_price = game.town_facility_price(&sorcery, price);
    let virtues = game.virtues;
    let book = give_book(&mut game, "life");
    begin(&mut game, &book);
    confirm(&mut game, true);
    assert_eq!(
        game.town_facility_membership(&sorcery),
        FacilityMembershipDto::Visitor
    );
    assert!(game.town_facility_price(&sorcery, price) > old_price);
    assert_eq!(
        game.town_facility_membership(&life),
        FacilityMembershipDto::Owner
    );
    assert_eq!(game.virtues, virtues);
    assert_eq!(game.active_casting_realm_profiles()[0].realm_id, "death");
}

#[test]
fn confirmation_applies_new_realm_autopick_inscription_without_pickup_or_destruction() {
    for action in ["", "!"] {
        let mut game = prepared(BUILD, 30);
        let book = give_book(&mut game, "nature");
        game.items.last_mut().unwrap().location = ItemLocation::Ground(game.player.position);
        game.interface_locale = LocaleDto::EnUs;
        assert!(
            game.configure_mogaminator(
                true,
                false,
                AutoGetModeDto::Off,
                LocaleDto::EnUs,
                format!("{action}second realm's spellbooks#changed")
            )
            .is_empty()
        );
        let before = game.rng.clone();
        begin(&mut game, &book);
        assert!(
            game.items
                .iter()
                .find(|item| item.id == book)
                .unwrap()
                .inscription
                .is_none()
        );
        confirm(&mut game, true);
        let item = game.items.iter().find(|item| item.id == book).unwrap();
        assert_eq!(item.inscription.as_deref(), Some("changed"));
        assert_eq!(item.location, ItemLocation::Ground(game.player.position));
        assert_eq!(game.rng, before);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
}

#[test]
fn automatic_identification_during_confirmation_cannot_destroy_the_selected_carried_book() {
    let mut game = prepared(BUILD, 30);
    let book = give_book(&mut game, "nature");
    give_inventory_item(&mut game, "test.identify", "demo.item.appraisal-scroll");
    game.mark_item_aware("demo.item.appraisal-scroll");
    game.interface_locale = LocaleDto::EnUs;
    assert!(
        game.configure_mogaminator(
            true,
            false,
            AutoGetModeDto::Off,
            LocaleDto::EnUs,
            "?!second realm's spellbooks#kept".to_owned()
        )
        .is_empty()
    );
    begin(&mut game, &book);
    assert!(game.items.iter().any(|item| item.id == "test.identify"));
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        confirm(game, true);
        let item = game.items.iter().find(|item| item.id == book).unwrap();
        assert_eq!(item.inscription.as_deref(), Some("kept"));
        assert_ne!(
            game.item_identification(item),
            ItemIdentificationDto::Unexamined
        );
        assert!(!game.items.iter().any(|item| item.id == "test.identify"));
        assert_eq!(game.spent_spell_learning, 0);
        assert!(Game::from_save(game.to_save()).is_ok());
    }
    assert_eq!(game.state_hash(), restored.state_hash());
    assert_eq!(game.rng, restored.rng);
}
