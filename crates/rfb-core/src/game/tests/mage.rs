// SPDX-License-Identifier: MPL-2.0
use super::support::{
    choose_human_talent_if_pending, clear_monsters, dispatch_next, give_inventory_item,
};
use super::*;

mod generation;
mod learning;
mod realm_change;

#[test]
fn desktop_preparation_preserves_natural_birth_and_round_trips_a_real_dungeon() {
    let mut game = Game::new_with_build(925, "demo.build.mage-arcane-sorcery").unwrap();
    choose_human_talent_if_pending(&mut game);
    let born = game.snapshot();
    game.debug_prepare_spell_learning_e2e(1).unwrap();
    let with_wand = game.snapshot();
    assert_eq!(with_wand.player.progress, born.player.progress);
    assert_eq!(with_wand.player.position, born.player.position);
    assert_eq!(with_wand.player.hp, born.player.hp);
    assert_eq!(with_wand.entities, born.entities);
    assert_eq!(with_wand.cells, born.cells);
    assert_eq!(with_wand.inventory.len(), born.inventory.len() + 1);
    game.transition_floor("demo.floor.warrens-depth-1".to_owned(), None, None, false)
        .unwrap();
    game.debug_prepare_spell_learning_e2e(25).unwrap();
    choose_human_talent_if_pending(&mut game);
    assert_eq!(game.progress.level, 25);
    assert!(
        game.snapshot()
            .player
            .abilities
            .iter()
            .any(|ability| { ability.id == EAT_MAGIC && ability.can_cast })
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
    let before = game.state_hash();
    assert!(game.debug_prepare_spell_learning_e2e(51).is_err());
    assert_eq!(game.state_hash(), before);
}

const BUILD: &str = "demo.build.mage-death-sorcery";
const MANA: &str = "demo.resource.mana";
const EAT_MAGIC: &str = "demo.ability.mage-eat-magic";
const REALMS: [&str; 8] = [
    "life",
    "sorcery",
    "nature",
    "death",
    "arcane",
    "daemon",
    "crusade",
    "armageddon",
];

fn at_level(build: &str, level: u16) -> Game {
    let mut game = Game::new_with_build(925, build).unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.player.hp = game.effective_player_max_hp();
    game
}

fn cast(game: &mut Game, ability: &str, target: TargetSelection) -> Vec<DomainEvent> {
    let mut events = Vec::new();
    game.resolve_player_ability(
        ability,
        target,
        &mut events,
        &mut BTreeSet::new(),
        &mut Vec::new(),
    )
    .unwrap();
    events
}

#[test]
fn all_fifty_six_births_have_two_distinct_first_books_and_the_same_mage() {
    let content = load_built_in_content().unwrap();
    assert_eq!(
        content
            .builds()
            .filter(|build| build.class_id == "demo.class.mage")
            .count(),
        56
    );
    for first in REALMS {
        for second in REALMS {
            if first == second {
                continue;
            }
            let id = format!("demo.build.mage-{first}-{second}");
            let game = Game::new_with_build(925, &id).unwrap();
            let snapshot = game.snapshot();
            assert_eq!(snapshot.player.kind_id, "demo.actor.mage-player");
            let identity = snapshot.player.build.unwrap();
            assert_eq!(
                (identity.life_percent, identity.experience_percent),
                (95, 130)
            );
            assert_eq!(
                snapshot.player.progress.attributes.intelligence.effective,
                16
            );
            assert!(game.learned_abilities.is_empty());
            assert_eq!(
                snapshot
                    .player
                    .abilities
                    .iter()
                    .filter(|ability| ability.source == AbilitySourceDto::Learned)
                    .count(),
                64
            );
            let mut books = Vec::new();
            for item in &game.items {
                if let Some(book) = game
                    .content
                    .item(&item.kind_id)
                    .and_then(|item| item.ability_book_id.as_deref())
                    .and_then(|id| game.content.ability_book(id))
                {
                    assert_eq!((book.rank, item.quantity), (Some(1), 1), "{id}");
                    books.push(book.realm_id.as_deref().unwrap());
                }
            }
            books.sort_unstable();
            let mut expected = [first, second];
            expected.sort_unstable();
            assert_eq!(books, expected, "{id}");
            for kind in ["demo.item.dagger", "demo.item.robe"] {
                assert!(
                    game.items.iter().any(|item| item.kind_id == kind
                        && matches!(item.location, ItemLocation::Equipped { .. })),
                    "{id}: {kind}"
                );
            }
            assert!(!game.items.iter().any(|item| matches!(
                item.kind_id.as_str(),
                "demo.item.clarity-draught" | "demo.item.magic-missile-wand"
            )));
        }
    }
}

#[test]
fn birth_proficiencies_virtues_and_race_combinations_follow_mage_source() {
    let game = at_level(BUILD, 1);
    let snapshot = game.snapshot();
    for (id, initial, maximum) in [
        ("demo.item.dagger", 4000, 8000),
        ("demo.item.wizardstaff", 4000, 8000),
        ("demo.item.sling", 4000, 6000),
        ("demo.item.nunchaku", 0, 0),
        ("demo.item.broad-sword", 2000, 4000),
    ] {
        let entry = snapshot
            .player
            .progress
            .weapon_proficiencies
            .iter()
            .find(|entry| entry.item_kind_id == id)
            .unwrap();
        assert_eq!((entry.current, entry.maximum), (initial, maximum), "{id}");
    }
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 0);
    assert_eq!(game.progress.dual_wielding_proficiency, 0);
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Knowledge);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Enchantment);
    for race in [
        "rfb-legacy.race.tomte",
        "rfb-legacy.race.tonberry",
        "rfb-legacy.race.spectre",
        "rfb-legacy.race.draconian-red",
    ] {
        let game = Game::new_with_build_race_and_name(925, BUILD, race, "Mage").unwrap();
        assert_eq!(game.active_casting_realm_profiles().len(), 2);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash(),
            "{race}"
        );
        if race == "rfb-legacy.race.draconian-red" {
            assert_eq!(game.draconian_metamorphosis_attack_level(), 1);
        }
    }
    assert!(Game::new_with_build_race_and_name(925, BUILD, "missing-race", "Mage").is_err());
    assert!(Game::new_with_build(925, "demo.build.mage-life-life").is_err());
    let mut dragon =
        Game::new_with_build_race_and_name(925, BUILD, "rfb-legacy.race.draconian-red", "Mage")
            .unwrap();
    dragon.apply_player_experience(dragon.experience_required_for_level(50), &mut Vec::new());
    assert_eq!(dragon.draconian_metamorphosis_attack_level(), 84);
}

#[test]
fn actual_experience_growth_preserves_mana_skills_and_level_twenty_five_power() {
    let mut game = at_level(BUILD, 1);
    assert_eq!(game.resources[MANA].maximum, 9);
    assert_eq!(
        game.ability_learning_capacity(game.casting_profile().unwrap()),
        1
    );
    let mut previous_hp = 0;
    let mut previous_mp = 0;
    for level in 1..=50 {
        game.apply_player_experience(
            game.experience_required_for_level(level)
                .saturating_sub(game.progress.experience),
            &mut Vec::new(),
        );
        choose_human_talent_if_pending(&mut game);
        assert_eq!(game.progress.level, level);
        assert!(game.effective_player_max_hp() >= previous_hp);
        assert!(game.resources[MANA].maximum >= previous_mp);
        previous_hp = game.effective_player_max_hp();
        previous_mp = game.resources[MANA].maximum;
        let power = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == EAT_MAGIC)
            .unwrap();
        assert_eq!(power.minimum_level, 25);
        assert_eq!(power.resource_cost, 1);
        assert_eq!(power.can_cast, level >= 25, "level {level}");
        assert!(power.failure_percent >= 11);
    }
    assert!(game.progress.skills["demo.skill.device"].current >= 40 + 5 * 15);
    assert_eq!(
        game.character_definitions().unwrap().2.pet_upkeep_divisor,
        30
    );
    game.progress.attributes.intelligence = 118;
    game.refresh_player_ability_state();
    let power = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == EAT_MAGIC)
        .unwrap();
    assert_eq!(power.failure_percent, 11);
    game.resources.get_mut(MANA).unwrap().current = 0;
    let recovery = game.player_resource_recovery_amount(MANA, false);
    assert!(recovery >= 2);
    game.recover_player_resources(false, &mut Vec::new());
    assert_eq!(game.resources[MANA].current, recovery);
}

#[test]
fn mage_has_no_specialist_damage_and_keeps_existing_spell_scaling() {
    let game = at_level("demo.build.mage-death-daemon", 30);
    let snapshot = game.snapshot();
    for id in [
        "demo.ability.death-entropy-orb",
        "demo.ability.daemon-hellish-flame",
    ] {
        let spell = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .unwrap();
        assert!(
            spell.effects.iter().any(|effect| matches!(
                effect,
                AbilityEffectSpecDto::AreaDamage {
                    damage_bonus: 45,
                    radius: 3,
                    ..
                }
            )),
            "{id}: {:?}",
            spell.effects
        );
    }
    assert_eq!(game.casting_spell_damage_bonus(), 0);
    let vampire = snapshot
        .player
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-vampirism-true")
        .unwrap();
    assert_eq!(vampire.base_resource_cost, 85);
    for (build, expected) in [
        ("demo.build.mage-life-death", 46),
        ("demo.build.mage-death-life", 99),
    ] {
        let game = at_level(build, 50);
        let spell = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|ability| ability.id == "demo.ability.life-warding-true")
            .unwrap();
        assert_eq!(spell.minimum_level, expected);
        if expected == 99 {
            assert!(!spell.can_study);
        }
    }
}

#[test]
fn equipment_uses_full_weapon_weight_and_source_glove_mana_penalty() {
    let mut game = at_level(BUILD, 25);
    game.items.clear();
    game.refresh_player_ability_state();
    let unburdened = game.resources[MANA].maximum;
    give_inventory_item(&mut game, "test.gloves", "demo.item.leather-gloves");
    let gloves = game.items.last_mut().unwrap();
    gloves.location = ItemLocation::Equipped {
        slot_id: "gloves".to_owned(),
    };
    gloves.intrinsic_weight_tenths_pound = Some(0);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened * 3 / 4);
    assert!(game.item_has_glove_encumbrance(&game.items[0]));
    game.items[0].location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
    game.items.last_mut().unwrap().location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(430);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened);
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(730);
    game.refresh_player_ability_state();
    assert_eq!(
        game.resources[MANA].maximum,
        unburdened - unburdened * 300 / 600
    );
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(1030);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, 0);
}

#[test]
fn melee_uses_mage_blow_parameters_and_heavy_weapon_limit() {
    let mut game = at_level(BUILD, 50);
    game.progress.attributes.strength = 118;
    game.progress.attributes.dexterity = 118;
    let attack = game.player_melee_profile(&game.player_derived_stats());
    let blows = u32::from(attack.attacks) * 100 + u32::from(attack.extra_attack_chance_percent);
    assert!((101..=400).contains(&blows));
    assert!(
        attack
            .attack_sources
            .iter()
            .any(|source| source.source_id == "demo.class.mage")
    );
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.dagger")
        .unwrap()
        .intrinsic_weight_tenths_pound = Some(5000);
    let attack = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((attack.attacks, attack.extra_attack_chance_percent), (1, 0));
}

#[test]
fn both_realms_learn_and_cast_with_mage_first_success_experience() {
    let mut game = at_level(BUILD, 10);
    game.debug_ability_casts_succeed = true;
    for (book, spell) in [
        (
            "demo.item.black-prayers",
            "demo.ability.death-detect-unlife",
        ),
        (
            "demo.item.beginners-handbook",
            "demo.ability.sorcery-detect-monsters",
        ),
    ] {
        let item_id = game
            .items
            .iter()
            .find(|item| item.kind_id == book)
            .unwrap()
            .id
            .clone();
        game.study_player_ability(&item_id, spell).unwrap();
        let before = game.progress.experience;
        let mana = game.resources[MANA].current;
        let events = cast(&mut game, spell, TargetSelection::SelfTarget);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityDetected { .. })),
            "{spell}: {events:?}"
        );
        assert_eq!(game.progress.experience - before, 4);
        assert!(game.resources[MANA].current < mana);
        cast(&mut game, spell, TargetSelection::SelfTarget);
        assert_eq!(
            game.progress.experience - before,
            4,
            "no repeated first-success XP"
        );
    }
    game.debug_ability_casts_succeed = false;
    let mut restored = Game::from_save(game.to_save()).unwrap();
    for game in [&mut game, &mut restored] {
        dispatch_next(
            game,
            GameCommand::CastAbility {
                ability_id: "demo.ability.sorcery-detect-monsters".to_owned(),
                target: TargetSelection::SelfTarget,
            },
        );
    }
    assert_eq!(game.state_hash(), restored.state_hash());
}

#[test]
fn sensing_uses_mage_frequencies_and_reaches_both_item_groups() {
    let mut game = at_level(BUILD, 1);
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "test.sword", "demo.item.small-sword");
    game.items.last_mut().unwrap().location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.items.last_mut().unwrap().enchantments.to_hit = 1;
    give_inventory_item(&mut game, "test.wand", "demo.item.magic-missile-wand");
    game.items.last_mut().unwrap().curse = Some(ItemCurseSeverityDto::Heavy);
    assert_eq!(game.effective_player_attributes().wisdom, 13);
    assert_eq!(game.virtue_current(VirtueKindDto::Knowledge), 0);
    // Source L1/WIS13: 20000*105/100/161 = 130, 9000*105/100/161 = 58.
    game.rng = (0..1_000_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(130) == 0 && rng.bounded(58) == 0 && rng.bounded(3) == 0
        })
        .unwrap();
    game.world_tick = 9;
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert_eq!(game.rng, rng);
    assert!(game.item_property_knowledge.is_empty());
    game.world_tick = 10;
    game.process_class_item_sensing();
    assert_eq!(
        game.item_feeling(&game.items[0]),
        Some(rfb_protocol::ItemFeelingDto::Enchanted)
    );
    assert_eq!(
        game.item_feeling(&game.items[1]),
        Some(rfb_protocol::ItemFeelingDto::Bad)
    );
    game.item_property_knowledge.clear();
    game.apply_player_mental_status(STATUS_CONFUSION, 10, "test");
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert!(game.item_property_knowledge.is_empty());
    assert_eq!(game.rng, rng);
}

#[test]
fn eat_magic_zero_odds_fails_without_a_success_roll_and_preserves_outer_cost() {
    let mut game = at_level(BUILD, 25);
    give_inventory_item(&mut game, "test.food", "demo.item.detect-objects-staff");
    let item = game.items.last_mut().unwrap();
    item.activation.as_mut().unwrap().device_check_difficulty = 120;
    item.charges.as_mut().unwrap().current = 20;
    game.resources.get_mut(MANA).unwrap().current = 10;
    game.debug_ability_casts_succeed = true;
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(10) != 0
        })
        .unwrap();
    let mut expected_rng = game.rng.clone();
    expected_rng.bounded(100);
    expected_rng.bounded(10);
    let events = cast(
        &mut game,
        EAT_MAGIC,
        TargetSelection::Item {
            item_id: "test.food".to_owned(),
        },
    );
    assert_eq!(game.resources[MANA].current, 9);
    assert_eq!(game.items.last().unwrap().charges.unwrap().current, 0);
    assert_eq!(game.rng, expected_rng);
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. } if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::DrainItemMagic { failed: true, destroyed: false, drained: 20, .. }]))));
}

#[test]
fn eat_magic_success_and_invalid_targets_use_real_device_sp() {
    let mut game = at_level(BUILD, 50);
    give_inventory_item(&mut game, "test.food", "demo.item.detect-objects-staff");
    let item = game.items.last_mut().unwrap();
    item.location = ItemLocation::Ground(game.player.position);
    let difficulty = item.activation.as_ref().unwrap().device_check_difficulty as u32;
    let available = item.charges.unwrap().current;
    let drained = difficulty.min(available);
    let odds = (100 - difficulty / 2) / 5;
    assert!(odds > 1);
    game.resources.get_mut(MANA).unwrap().current = 10;
    game.debug_ability_casts_succeed = true;
    game.rng = (0..100)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(u64::from(odds)) != 0
        })
        .unwrap();
    let events = cast(
        &mut game,
        EAT_MAGIC,
        TargetSelection::Item {
            item_id: "test.food".to_owned(),
        },
    );
    assert_eq!(game.resources[MANA].current, 9 + drained);
    assert_eq!(
        game.items.last().unwrap().charges.unwrap().current,
        available - drained
    );
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. } if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::DrainItemMagic { failed: false, .. }]))));
    for target in [
        TargetSelection::SelfTarget,
        TargetSelection::Item {
            item_id: "missing".to_owned(),
        },
    ] {
        let before = game.state_hash();
        let events = cast(&mut game, EAT_MAGIC, target);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityTargetUnavailable { .. }))
        );
        assert_eq!(game.state_hash(), before);
    }
}

#[test]
fn eat_magic_outer_failure_keeps_device_and_internal_failure_can_destroy_it() {
    let mut game = at_level(BUILD, 25);
    give_inventory_item(&mut game, "test.food", "demo.item.detect-objects-staff");
    game.items
        .last_mut()
        .unwrap()
        .activation
        .as_mut()
        .unwrap()
        .device_check_difficulty = 120;
    let before = game.items.last().unwrap().charges;
    let mana = game.resources[MANA].current;
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| rng.clone().bounded(100) == 0)
        .unwrap();
    let events = cast(
        &mut game,
        EAT_MAGIC,
        TargetSelection::Item {
            item_id: "test.food".to_owned(),
        },
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event, DomainEvent::AbilityCastFailed { .. }))
    );
    assert_eq!(game.resources[MANA].current, mana - 1);
    assert_eq!(game.items.last().unwrap().charges, before);
    game.debug_ability_casts_succeed = true;
    game.rng = (0..1000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(100);
            rng.bounded(10) == 0
        })
        .unwrap();
    let events = cast(
        &mut game,
        EAT_MAGIC,
        TargetSelection::Item {
            item_id: "test.food".to_owned(),
        },
    );
    assert!(!game.items.iter().any(|item| item.id == "test.food"));
    assert!(events.iter().any(|event| matches!(event, DomainEvent::AbilityEffectsResolved { resolution, .. } if matches!(resolution.effects.as_slice(), [AbilityEffectResolutionDto::DrainItemMagic { failed: true, destroyed: true, .. }]))));
}
