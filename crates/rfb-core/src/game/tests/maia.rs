// SPDX-License-Identifier: MPL-2.0
// RFB master a0d92b6378d148c5262cc236b8fa6ed2ca06a54c: races_k.c and its consumers.
use super::support::*;
use super::*;
use rfb_protocol::ItemFeelingDto;
use rfb_protocol::MaiaPathDto::{Corrupted, Enlightened};

const MAIA: &str = "rfb-legacy.race.maia";

fn prepared(build: &str, level: u16) -> Game {
    let mut game = Game::new_with_build_race_and_name(83, build, MAIA, "test").unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    game
}

fn restore(game: &Game) -> Game {
    let restored = Game::from_save_with_content(game.to_save(), game.content.clone()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    restored
}

#[test]
fn maia_choice_survives_save_level_loss_and_rejects_replay_without_rng_or_time() {
    for path in [Enlightened, Corrupted] {
        let mut game = prepared("demo.build.warrior", 1);
        assert!(game.player_slow_digestion());
        assert_eq!(game.player_infravision_range(), 10);
        assert!(!game.pending_maia_path_choice());
        assert!(game.maia_resistances().is_empty());
        game.apply_player_experience(game.experience_required_for_level(27), &mut Vec::new());
        assert!(game.snapshot().player.pending_maia_path_choice);
        let mut restored = restore(&game);
        let before = restored.to_save();
        let snapshot = restored.snapshot();
        assert!(matches!(
            restored.dispatch(command(
                snapshot.last_command_seq + 1,
                snapshot.revision,
                GameCommand::Wait
            )),
            Err(CoreError::MaiaPathChoiceRequired)
        ));
        assert_eq!(restored.to_save(), before);
        let rng = restored.rng.clone();
        let clock = (restored.player.energy_need, restored.world_tick);
        dispatch_next(&mut restored, GameCommand::ChooseMaiaPath { path });
        assert_eq!(restored.rng, rng);
        assert_eq!((restored.player.energy_need, restored.world_tick), clock);
        assert_eq!(restored.snapshot().player.maia_path, Some(path));
        let before = restored.to_save();
        let snapshot = restored.snapshot();
        assert!(matches!(
            restored.dispatch(command(
                snapshot.last_command_seq + 1,
                snapshot.revision,
                GameCommand::ChooseMaiaPath { path }
            )),
            Err(CoreError::MaiaPathChoiceUnavailable)
        ));
        assert_eq!(restored.to_save(), before);
        restored.apply_player_experience_drain(
            restored.progress.experience,
            "test.drain",
            &mut Vec::new(),
        );
        assert_eq!(restored.progress.level, 1);
        let mut restored = restore(&restored);
        assert_eq!(restored.maia_path, Some(path));
        restored
            .apply_player_experience(restored.experience_required_for_level(27), &mut Vec::new());
        assert!(!restored.pending_maia_path_choice());
        assert!(restored.change_player_race(
            "rfb-legacy.race.high-elf",
            restored.effective_player_max_hp(),
            &mut Vec::new()
        ));
        assert_eq!(restored.maia_path, None);
        assert!(restored.maia_resistances().is_empty());
        assert!(!restored.maia_forbids_realm("death"));
        assert!(!restored.maia_forbids_realm("life"));
        restore(&restored);
    }
}

#[test]
fn maia_save_rejects_paths_without_the_native_race_or_a_reached_initiation_level() {
    let game = prepared("demo.build.warrior", 1);
    let mut save = game.to_save();
    save.player.maia_path = Some(Corrupted);
    assert!(Game::from_save_with_content(save, game.content.clone()).is_err());
    let mut game = prepared("demo.build.warrior", 20);
    dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path: Enlightened });
    game.build.as_mut().unwrap().race_id = "rfb-legacy.race.high-elf".to_owned();
    assert!(Game::from_save_with_content(game.to_save(), game.content.clone()).is_err());
}

#[test]
fn maia_branches_apply_level_boundaries_light_hunger_and_current_body() {
    let mut enlightened = prepared("demo.build.warrior", 20);
    enlightened.items.clear();
    dispatch_next(
        &mut enlightened,
        GameCommand::ChooseMaiaPath { path: Enlightened },
    );
    let mut corrupted = prepared("demo.build.warrior", 20);
    corrupted.items.clear();
    dispatch_next(
        &mut corrupted,
        GameCommand::ChooseMaiaPath { path: Corrupted },
    );
    assert_eq!(enlightened.player_light_radius(), Some(1));
    assert_eq!(enlightened.player_see_invisible_sources(), 1);
    assert!(!enlightened.player_levitates());
    for (level, light, ac) in [(22, 1, 1), (26, 2, 3), (49, 5, 14), (50, 6, 15)] {
        enlightened.progress.level = level;
        corrupted.progress.level = level;
        assert_eq!(enlightened.player_light_radius(), Some(light));
        assert_eq!(
            enlightened.player_derived_stats().armor_class.value,
            corrupted.player_derived_stats().armor_class.value + ac
        );
        assert!(
            corrupted.character_base_max_hp_at_level(level)
                > enlightened.character_base_max_hp_at_level(level)
        );
        assert_eq!(enlightened.player_levitates(), level == 50);
        assert_eq!(
            enlightened.maia_has_contact_aura(DamageType::Cold),
            level == 50
        );
        assert_eq!(
            enlightened.maia_has_contact_aura(DamageType::Electricity),
            level == 50
        );
        assert_eq!(
            corrupted.maia_has_contact_aura(DamageType::Fire),
            level == 50
        );
        assert_eq!(
            corrupted
                .effective_player_resistances()
                .level(DamageType::Fire),
            if level == 50 {
                ResistanceLevel::Immune
            } else {
                ResistanceLevel::Resistant
            }
        );
    }
    for game in [&mut enlightened, &mut corrupted] {
        game.progress.level = 1;
        assert_eq!(
            game.effective_player_resistances().level(DamageType::Time),
            ResistanceLevel::Resistant
        );
        assert!(!game.maia_has_contact_aura(DamageType::Fire));
        assert!(!game.maia_has_contact_aura(DamageType::Cold));
        let rng = game.rng.clone();
        for nutrition in [50, 16_000] {
            game.nutrition = nutrition;
            game.world_tick = 10;
            game.process_hunger(&mut Vec::new());
            assert_eq!(game.nutrition, crate::game::hunger::NUTRITION_FULL);
        }
        assert_eq!(game.rng, rng);
        let mut form = monster_combat::melee_status("test.form", 50, "test.setup").status;
        form.granted_race_id = Some("demo.race.rfb-human".to_owned());
        game.player.statuses.push(form);
        assert!(game.maia_resistances().is_empty());
        game.nutrition = 50;
        game.process_hunger(&mut Vec::new());
        assert_eq!(game.nutrition, crate::game::hunger::NUTRITION_FULL);
    }
    assert_eq!(enlightened.player_light_radius(), Some(1));
    assert!(enlightened.maia_forbids_realm("death"));
    assert!(corrupted.maia_forbids_realm("life"));
    enlightened.player.statuses.clear();
    give_inventory_item(&mut enlightened, "test.dark", "demo.item.long-sword");
    enlightened.items[0].location = ItemLocation::Equipped {
        slot_id: "right-hand".to_owned(),
    };
    enlightened.items[0]
        .intrinsic_properties
        .equipment_bonuses
        .light_radius = -3;
    enlightened.progress.level = 20;
    assert_eq!(enlightened.player_light_radius(), None);
    enlightened.progress.level = 32;
    assert_eq!(enlightened.player_light_radius(), Some(1));
}

#[test]
fn enlightened_maia_senses_only_carried_curses_without_identifying_quality_or_enchantments() {
    let mut game = prepared("demo.build.warrior", 20);
    dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path: Enlightened });
    give_inventory_item(&mut game, "test.curse", "demo.item.long-sword");
    let item = game.items.last_mut().unwrap();
    item.location = ItemLocation::Ground(game.player.position);
    item.curse = Some(ItemCurseSeverityDto::Heavy);
    item.enchantments.to_damage = -3;
    item.intrinsic_curse_effects
        .insert(ItemCurseEffectDto::FastDigest);
    game.item_property_knowledge.remove("test.curse");
    game.apply_player_floor_item_knowledge();
    assert_eq!(game.visible_item_curse(game.items.last().unwrap()), None);
    game.pick_up_item_at_player(Some("test.curse")).unwrap();
    let item = game
        .items
        .iter()
        .find(|item| item.id == "test.curse")
        .unwrap();
    assert_eq!(
        game.visible_item_curse(item),
        Some(ItemCurseSeverityDto::Heavy)
    );
    assert_eq!(game.item_feeling(item), Some(ItemFeelingDto::Cursed));
    assert_eq!(
        game.item_identification(item),
        ItemIdentificationDto::Unexamined
    );
    assert_eq!(game.visible_item_quality(item), None);
    assert_eq!(
        game.visible_item_enchantments(item),
        ItemEnchantmentsDto::default()
    );
    let mut game = restore(&game);
    game.item_property_knowledge
        .get_mut("test.curse")
        .unwrap()
        .feeling = Some(ItemFeelingDto::Excellent);
    game.maia_sense_carried_curse("test.curse");
    assert_eq!(
        game.item_property_knowledge["test.curse"].feeling,
        Some(ItemFeelingDto::Excellent)
    );
}

#[test]
fn corrupted_maia_removes_light_curses_without_rng_but_keeps_heavy_and_permanent_curses() {
    let mut game = prepared("demo.build.warrior", 20);
    dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path: Corrupted });
    game.items.clear();
    give_inventory_item(&mut game, "test.armor", "demo.item.chain-mail");
    game.items[0].location = ItemLocation::Equipped {
        slot_id: "body".to_owned(),
    };
    game.items[0].enchantments.to_armor = -5;
    game.items[0]
        .intrinsic_curse_effects
        .insert(ItemCurseEffectDto::FastDigest);
    for severity in [ItemCurseSeverityDto::Heavy, ItemCurseSeverityDto::Permanent] {
        game.items[0].curse = Some(severity);
        let before = game.items[0].clone();
        let rng = game.rng.clone();
        assert!(!game.try_remove_equipment_curse(0));
        assert_eq!(game.items[0], before);
        assert_eq!(game.rng, rng);
    }
    game.items[0].curse = Some(ItemCurseSeverityDto::Normal);
    let rng = game.rng.clone();
    assert!(game.try_remove_equipment_curse(0));
    assert_eq!(game.rng, rng);
    assert_eq!(game.items[0].curse, None);
    assert!(game.items[0].intrinsic_curse_effects.is_empty());
    assert_eq!(game.items[0].enchantments.to_armor, -5);
    game.items[0].curse = Some(ItemCurseSeverityDto::Normal);
    dispatch_next(
        &mut game,
        GameCommand::Unequip {
            slot_id: "body".to_owned(),
        },
    );
    assert_eq!(game.items[0].location, ItemLocation::Inventory);
    assert_eq!(game.items[0].curse, None);
    restore(&game);
}

#[test]
fn enlightened_slay_is_armed_melee_only_and_both_paths_keep_normal_potion_healing() {
    let mut game = prepared("demo.build.warrior", 50);
    dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path: Enlightened });
    game.items.clear();
    give_inventory_item(&mut game, "test.weapon", "demo.item.long-sword");
    game.items[0].location = ItemLocation::Equipped {
        slot_id: "right-hand".to_owned(),
    };
    let target = game.player.clone();
    let mut enemy = game.content.actor(&game.player.kind_id).unwrap().clone();
    enemy.tags.push("evil".to_owned());
    let profile = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &enemy),
        19
    );
    assert_eq!(
        game.item_damage_multiplier(&game.items[0], &target, &enemy),
        10
    );
    let mut bare = profile.clone();
    bare.source_item_id = None;
    assert_eq!(
        game.player_melee_damage_multiplier(&bare, &target, &enemy),
        10
    );
    game.progress.level = 49;
    assert_eq!(
        game.player_melee_damage_multiplier(&profile, &target, &enemy),
        10
    );
    for path in [Enlightened, Corrupted] {
        let mut game = prepared("demo.build.warrior", 20);
        dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path });
        give_inventory_item(&mut game, "test.potion", "demo.item.light-healing-potion");
        game.player.hp = 1;
        dispatch_next(
            &mut game,
            GameCommand::UseItem {
                item_id: "test.potion".to_owned(),
                target: None,
            },
        );
        assert!(game.player.hp > 1);
        assert!(!game.items.iter().any(|item| item.id == "test.potion"));
        restore(&game);
    }
}

#[test]
fn maia_allows_second_realm_changes_but_forbids_the_new_realms_spells() {
    let mut game = prepared("demo.build.warrior-mage-arcane-sorcery", 20);
    dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path: Corrupted });
    let kind = game
        .content
        .item_definitions()
        .find(|item| {
            item.ability_book_id
                .as_deref()
                .and_then(|id| game.content.ability_book(id))
                .is_some_and(|book| {
                    book.realm_id.as_deref() == Some("life") && book.rank == Some(1)
                })
        })
        .unwrap()
        .id
        .clone();
    give_inventory_item(&mut game, "test.life", &kind);
    assert_eq!(
        game.realm_change_book("test.life").unwrap().realm_id,
        "life"
    );
    dispatch_next(
        &mut game,
        GameCommand::BeginRealmChange {
            book_item_id: "test.life".to_owned(),
        },
    );
    let mut game = restore(&game);
    dispatch_next(&mut game, GameCommand::ResolveRealmChange { confirm: true });
    assert_eq!(game.current_second_realm_id(), Some("life"));
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .filter(|ability| game.maia_forbids_spell(&ability.id))
            .all(|ability| !ability.can_cast && !ability.can_study)
    );
    restore(&game);
}

#[test]
fn maia_realm_restrictions_preserve_memory_and_browsing_across_real_classes_and_saves() {
    for (build, path, forbidden_realm) in [
        ("demo.build.mage-life-sorcery", Corrupted, "life"),
        ("demo.build.priest-life-sorcery", Corrupted, "life"),
        ("demo.build.warrior-mage-arcane-death", Enlightened, "death"),
        ("demo.build.ranger-nature-death", Enlightened, "death"),
        ("demo.build.high-mage-death", Enlightened, "death"),
        ("demo.build.paladin-death", Enlightened, "death"),
    ] {
        let mut game = prepared(build, 20);
        let first = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.source == AbilitySourceDto::Learned && ability.can_study)
            .unwrap();
        if game.casting_profile().unwrap().study_mode == rfb_content::CastingStudyMode::DivineRandom
        {
            game.study_random_player_ability(first.book_item_id.as_deref().unwrap())
                .unwrap();
        } else {
            game.study_player_ability(first.book_item_id.as_deref().unwrap(), &first.id)
                .unwrap();
        }
        let before = game.ability_learning_order.clone();
        let learned = game.learned_abilities.clone();
        dispatch_next(&mut game, GameCommand::ChooseMaiaPath { path });
        assert!(game.maia_forbids_realm(forbidden_realm));
        assert_eq!(game.ability_learning_order, before);
        assert_eq!(game.learned_abilities, learned);
        assert!(!game.maia_forbids_realm("sorcery"));
        let mut game = restore(&game);
        let spell = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| {
                ability.source == AbilitySourceDto::Learned
                    && game.maia_forbids_spell(&ability.id)
                    && ability.book_item_id.is_some()
            });
        let spell = spell.unwrap();
        assert!(!spell.can_cast);
        assert!(!spell.can_study);
        assert_eq!(
            spell.unavailable_reason.as_deref(),
            Some("maia-realm-forbidden")
        );
        let rng = game.rng.clone();
        let clock = (game.player.energy_need, game.world_tick);
        let mana = game.resources.clone();
        dispatch_next(
            &mut game,
            GameCommand::CastAbility {
                ability_id: spell.id.clone(),
                target: TargetSelection::SelfTarget,
            },
        );
        assert_eq!(game.rng, rng);
        assert_eq!((game.player.energy_need, game.world_tick), clock);
        assert_eq!(game.resources, mana);
        let book = spell.book_item_id.unwrap();
        let result = if game.casting_profile().unwrap().study_mode
            == rfb_content::CastingStudyMode::DivineRandom
        {
            game.study_random_player_ability(&book).map(|_| ())
        } else {
            game.study_player_ability(&book, &spell.id)
        };
        assert_eq!(result, Err("maia-realm-forbidden"));
        assert_eq!(game.rng, rng);
        assert_eq!(game.ability_learning_order, before);
    }
}
