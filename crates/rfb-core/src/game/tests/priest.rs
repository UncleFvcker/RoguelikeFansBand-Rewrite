// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters, give_inventory_item};
use super::*;

mod generation;
mod learning;
mod powers;
mod realm_change;

const BUILD: &str = "demo.build.priest-life-sorcery";
const MANA: &str = "demo.resource.mana";

fn at_level(build: &str, level: u16) -> Game {
    let mut game = Game::new_with_build(925, build).unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.player.hp = game.effective_player_max_hp();
    game
}

#[test]
fn all_births_have_two_source_books_equipment_and_unlearned_prayers() {
    let content = Game::new_with_build(925, BUILD).unwrap().content;
    let builds = content
        .builds()
        .filter(|build| build.class_id == "demo.class.priest")
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 24);
    for build in builds {
        let game = Game::new_with_build(925, &build.id).unwrap();
        let snapshot = game.snapshot();
        assert_eq!(snapshot.player.kind_id, "demo.actor.priest-player");
        let identity = snapshot.player.build.unwrap();
        assert_eq!(
            (identity.life_percent, identity.experience_percent),
            (100, 120)
        );
        assert_eq!(snapshot.player.progress.attributes.wisdom.effective, 16);
        assert_eq!(game.resources[MANA].maximum, 9);
        assert_eq!(
            game.ability_learning_capacity(game.casting_profile().unwrap()),
            1
        );
        assert!(game.learned_abilities.is_empty());
        assert_eq!(
            snapshot
                .player
                .abilities
                .iter()
                .filter(|a| a.source == AbilitySourceDto::Learned)
                .count(),
            64
        );
        let mut books = game
            .items
            .iter()
            .filter_map(|item| {
                let book = game
                    .content
                    .item(&item.kind_id)?
                    .ability_book_id
                    .as_deref()?;
                let book = game.content.ability_book(book)?;
                assert_eq!((book.rank, item.quantity), (Some(1), 1));
                book.realm_id.as_deref()
            })
            .collect::<Vec<_>>();
        books.sort_unstable();
        let mut expected = [
            build.first_realm_id.as_deref().unwrap(),
            build.second_realm_id.as_deref().unwrap(),
        ];
        expected.sort_unstable();
        assert_eq!(books, expected, "{}", build.id);
        for kind in ["demo.item.mace", "demo.item.robe"] {
            assert!(game.items.iter().any(|item| item.kind_id == kind
                && matches!(item.location, ItemLocation::Equipped { .. })));
        }
        assert!(
            game.items
                .iter()
                .any(|item| item.kind_id == "demo.item.healing-potion"
                    && item.quantity == 1
                    && item.location == ItemLocation::Inventory)
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
    }
}

#[test]
fn experience_growth_uses_wisdom_and_ninety_six_studies_without_mage_regeneration() {
    let mut game = at_level(BUILD, 1);
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
    }
    assert!(game.progress.skills["demo.skill.device"].current >= 28 + 5 * 11);
    assert_eq!(
        game.character_definitions().unwrap().2.pet_upkeep_divisor,
        35
    );
    game.progress.attributes.wisdom = game.progress.attribute_potentials.wisdom;
    game.progress.maximum_attributes.wisdom = game.progress.attributes.wisdom;
    game.refresh_player_ability_state();
    assert_eq!(
        game.ability_learning_capacity(game.casting_profile().unwrap()),
        96
    );
    let maximum = game.resources[MANA].maximum;
    game.progress.attributes.intelligence = 3;
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, maximum);
    let resource = game.content.resource(MANA).unwrap();
    assert_eq!(
        game.player_resource_recovery_change(MANA, false),
        i64::from(resource.wait_recovery_amount)
    );
    assert_eq!(
        game.player_resource_recovery_change(MANA, true),
        i64::from(resource.rest_recovery_amount)
    );
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn proficiencies_virtues_and_racial_births_use_priest_growth() {
    let game = at_level(BUILD, 1);
    let snapshot = game.snapshot();
    for (id, initial, maximum) in [
        ("demo.item.mace", 4000, 7000),
        ("demo.item.wizardstaff", 4000, 7000),
        ("demo.item.war-hammer", 2000, 7000),
        ("demo.item.nunchaku", 2000, 4000),
        ("demo.item.long-sword", 2000, 4000),
        ("demo.item.short-bow", 2000, 4000),
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
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 2000);
    assert_eq!(
        game.character_definitions()
            .unwrap()
            .2
            .dual_wielding_maximum,
        4000
    );
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Faith);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Temperance);
    for race in [
        "rfb-legacy.race.tomte",
        "rfb-legacy.race.tonberry",
        "rfb-legacy.race.spectre",
        "rfb-legacy.race.draconian-red",
    ] {
        let mut game = Game::new_with_build_race_and_name(925, BUILD, race, "Priest").unwrap();
        assert_eq!(game.active_casting_realm_profiles().len(), 2);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash(),
            "{race}"
        );
        if race == "rfb-legacy.race.draconian-red" {
            game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
            assert_eq!(game.draconian_metamorphosis_attack_level(), 105);
        }
    }
}

#[test]
fn priest_gloves_are_free_but_weapon_and_armor_weight_reduce_mana() {
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
    assert_eq!(game.resources[MANA].maximum, unburdened);
    give_inventory_item(&mut game, "test.weapon", "demo.item.mace");
    let weapon = game.items.last_mut().unwrap();
    weapon.location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    weapon.intrinsic_weight_tenths_pound = Some(1000);
    game.refresh_player_ability_state();
    assert_eq!(
        game.resources[MANA].maximum,
        unburdened - unburdened * (670 - 430) / 800
    );
    game.items.last_mut().unwrap().location = ItemLocation::Inventory;
    game.items[0].intrinsic_weight_tenths_pound = Some(1230);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, 0);
    game.items[0].intrinsic_weight_tenths_pound = Some(0);
    let before = game.snapshot().player.abilities;
    game.items[0]
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::ReducedManaCost);
    game.refresh_player_ability_state();
    assert!(
        game.player_equipment_passives()
            .contains(&EquipmentPassive::ReducedManaCost)
    );
    assert!(
        before
            .iter()
            .zip(game.snapshot().player.abilities)
            .any(|(before, after)| before.id == after.id
                && after.resource_cost < before.resource_cost)
    );
}

#[test]
fn first_group_sensing_is_fast_and_weak_second_is_medium_and_strong() {
    let mut game = at_level(BUILD, 1);
    game.progress.attributes.wisdom = 10;
    game.items.clear();
    game.item_property_knowledge.clear();
    give_inventory_item(&mut game, "test.sword", "demo.item.small-sword");
    game.items.last_mut().unwrap().location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.items.last_mut().unwrap().curse = Some(ItemCurseSeverityDto::Heavy);
    give_inventory_item(&mut game, "test.wand", "demo.item.magic-missile-wand");
    game.items.last_mut().unwrap().curse = Some(ItemCurseSeverityDto::Heavy);
    assert_eq!(game.effective_player_attributes().wisdom, 13);
    assert_eq!(game.virtue_current(VirtueKindDto::Knowledge), 0);
    // Source L1/WIS13: 9000*105/100/161 = 58; 20000*105/100/161 = 130.
    game.rng = (0..1_000_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(58) == 0 && rng.bounded(130) == 0 && rng.bounded(3) == 0
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
        Some(rfb_protocol::ItemFeelingDto::Cursed)
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
fn source_spell_parameters_keep_daemon_start_craft_sentinel_and_orb_scaling() {
    let mut daemon = at_level("demo.build.priest-daemon-craft", 1);
    let book = daemon
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.dark-incantations")
        .unwrap()
        .id
        .clone();
    assert_eq!(
        daemon.study_random_player_ability(&book),
        Err("no-learnable-abilities")
    );
    daemon.apply_player_experience(daemon.experience_required_for_level(2), &mut Vec::new());
    assert!(daemon.study_random_player_ability(&book).is_ok());
    for (build, ability, bonus) in [
        (BUILD, "demo.ability.life-holy-orb", 45),
        (
            "demo.build.priest-death-daemon",
            "demo.ability.death-entropy-orb",
            37,
        ),
        (
            "demo.build.priest-death-daemon",
            "demo.ability.daemon-hellish-flame",
            37,
        ),
    ] {
        let game = at_level(build, 30);
        let spell = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|a| a.id == ability)
            .unwrap();
        assert!(spell.effects.iter().any(|e| matches!(e, AbilityEffectSpecDto::AreaDamage { damage_bonus, .. } if *damage_bonus == bonus)));
    }
    let game = at_level("demo.build.priest-life-craft", 50);
    let sentinel = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|a| a.id == "demo.ability.craft-mana-brand")
        .unwrap();
    assert_eq!(sentinel.minimum_level, 99);
    assert!(!sentinel.can_study);
}

#[test]
fn first_success_experience_is_awarded_once_from_priest_parameters() {
    let mut game = at_level(BUILD, 1);
    let book = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.book-of-common-prayer")
        .unwrap()
        .id
        .clone();
    let ability = game.study_random_player_ability(&book).unwrap();
    game.debug_ability_casts_succeed = true;
    let before = game.progress.experience;
    for _ in 0..2 {
        game.resources.get_mut(MANA).unwrap().current = game.resources[MANA].maximum;
        let mut events = Vec::new();
        game.resolve_player_ability(
            &ability,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, DomainEvent::AbilityCastSucceeded { .. }))
        );
        assert_eq!(game.progress.experience - before, 4);
    }
}
