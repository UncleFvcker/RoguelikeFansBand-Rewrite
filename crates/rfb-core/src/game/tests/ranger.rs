// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters, give_inventory_item};
use super::*;

mod learning;
mod realm_change;

const BUILD: &str = "demo.build.ranger-nature-death";
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
fn four_births_have_source_equipment_books_and_no_early_magic() {
    for second in ["sorcery", "death", "arcane", "daemon"] {
        let id = format!("demo.build.ranger-nature-{second}");
        let game = Game::new_with_build(925, &id).unwrap();
        let snapshot = game.snapshot();
        assert_eq!(snapshot.player.kind_id, "demo.actor.ranger-player");
        let build = snapshot.player.build.unwrap();
        assert_eq!((build.life_percent, build.experience_percent), (106, 140));
        assert_eq!(snapshot.player.progress.attributes.wisdom.effective, 15);
        assert!(game.learned_abilities.is_empty());
        assert_eq!(game.resources[MANA].maximum, 0);
        assert_eq!(
            game.ability_learning_capacity(game.casting_profile().unwrap()),
            0
        );
        let mut books = Vec::new();
        for item in &game.items {
            if let Some(book) = game
                .content
                .item(&item.kind_id)
                .and_then(|item| item.ability_book_id.as_deref())
                .and_then(|id| game.content.ability_book(id))
            {
                assert_eq!((book.rank, item.quantity), (Some(1), 1));
                books.push(book.realm_id.as_deref().unwrap());
            }
        }
        books.sort_unstable();
        let mut expected = ["nature", second];
        expected.sort_unstable();
        assert_eq!(books, expected);
        for kind in [
            "demo.item.dagger",
            "demo.item.soft-leather-armour",
            "demo.item.short-bow",
        ] {
            assert!(
                game.items.iter().any(|item| item.kind_id == kind
                    && matches!(item.location, ItemLocation::Equipped { .. })),
                "{kind}"
            );
        }
        let arrows = game
            .items
            .iter()
            .find(|item| item.kind_id == "demo.item.arrow")
            .unwrap();
        assert!((20..=40).contains(&arrows.quantity));
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
    }
}

#[test]
fn experience_growth_starts_mana_and_learning_at_three() {
    let mut game = at_level(BUILD, 1);
    let mut previous_hp = 0;
    for level in 1..=50 {
        game.apply_player_experience(
            game.experience_required_for_level(level)
                .saturating_sub(game.progress.experience),
            &mut Vec::new(),
        );
        choose_human_talent_if_pending(&mut game);
        assert_eq!(game.progress.level, level);
        assert!(game.effective_player_max_hp() >= previous_hp);
        previous_hp = game.effective_player_max_hp();
        if level <= 3 {
            game.progress.attributes.wisdom = 13;
            game.refresh_player_ability_state();
            assert_eq!(game.resources[MANA].maximum, if level < 3 { 0 } else { 8 });
            assert_eq!(
                game.ability_learning_capacity(game.casting_profile().unwrap()),
                if level < 3 { 0 } else { 1 }
            );
        }
    }
    assert!(game.progress.skills["demo.skill.device"].current >= 37 + 5 * 11);
    assert_eq!(
        game.character_definitions().unwrap().2.pet_upkeep_divisor,
        35
    );
    game.progress.attributes.wisdom = 118;
    game.refresh_player_ability_state();
    assert_eq!(
        game.ability_learning_capacity(game.casting_profile().unwrap()),
        80
    );
    let power = game
        .snapshot()
        .player
        .abilities
        .into_iter()
        .find(|ability| ability.id == "demo.ability.ranger-probe-monsters")
        .unwrap();
    assert_eq!((power.minimum_level, power.base_resource_cost), (15, 20));
}

#[test]
fn proficiencies_virtues_and_race_births_follow_source() {
    let game = at_level(BUILD, 1);
    let snapshot = game.snapshot();
    for (id, initial, maximum) in [
        ("demo.item.dagger", 4000, 8000),
        ("demo.item.short-bow", 4000, 8000),
        ("demo.item.sling", 2000, 7000),
        ("demo.item.heavy-crossbow", 2000, 7000),
        ("demo.item.long-sword", 2000, 6000),
        ("demo.item.poison-needle", 2000, 8000),
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
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 6000);
    assert_eq!(
        game.character_definitions()
            .unwrap()
            .2
            .dual_wielding_maximum,
        6000
    );
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Nature);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Temperance);
    for race in [
        "rfb-legacy.race.tomte",
        "rfb-legacy.race.tonberry",
        "rfb-legacy.race.spectre",
        "rfb-legacy.race.draconian-red",
    ] {
        let mut game = Game::new_with_build_race_and_name(925, BUILD, race, "Ranger").unwrap();
        assert_eq!(game.active_casting_realm_profiles().len(), 2);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash()
        );
        if race == "rfb-legacy.race.draconian-red" {
            game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
            assert_eq!(game.draconian_metamorphosis_attack_level(), 105);
        }
    }
}

#[test]
fn first_nature_spell_awards_ranger_experience_once() {
    let mut game = at_level(BUILD, 3);
    let book = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.call-of-the-wild")
        .unwrap()
        .id
        .clone();
    let spell = "demo.ability.nature-detect-creatures";
    assert_eq!(game.study_random_player_ability(&book).unwrap(), spell);
    // Level-up raises the maximum without filling the new mana pool.
    for _ in 0..8 {
        game.recover_player_resources(false, &mut Vec::new());
    }
    assert_eq!(game.resources[MANA].current, 8);
    game.debug_ability_casts_succeed = true;
    let before = game.progress.experience;
    for _ in 0..2 {
        let mut events = Vec::new();
        game.resolve_player_ability(
            spell,
            TargetSelection::SelfTarget,
            &mut events,
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            events
                .iter()
                .any(|event| matches!(event, DomainEvent::AbilityCastSucceeded { .. }))
        );
        assert_eq!(game.progress.experience - before, 6);
    }
}

#[test]
fn ranger_spell_parameters_preserve_unavailable_slots_and_orb_scaling() {
    for (second, orb) in [
        ("death", "death-entropy-orb"),
        ("daemon", "daemon-hellish-flame"),
    ] {
        let game = at_level(&format!("demo.build.ranger-nature-{second}"), 30);
        let snapshot = game.snapshot();
        let spell = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == format!("demo.ability.{orb}"))
            .unwrap();
        assert!(spell.effects.iter().any(|effect| matches!(
            effect,
            AbilityEffectSpecDto::AreaDamage {
                damage_bonus: 37,
                radius: 3,
                ..
            }
        )));
        let frost = snapshot
            .player
            .abilities
            .iter()
            .find(|ability| ability.id == "demo.ability.nature-frost-bolt")
            .unwrap();
        assert!(frost.effects.iter().any(|effect| matches!(
            effect,
            AbilityEffectSpecDto::BoltOrBeamDamage {
                beam_chance_percent: 15,
                ..
            }
        )));
        for spell in snapshot
            .player
            .abilities
            .iter()
            .filter(|ability| ability.minimum_level == 99)
        {
            assert!(!spell.can_study && !spell.can_cast);
        }
        let profile = game.casting_profile().unwrap();
        assert_eq!(
            profile
                .realm_profiles
                .iter()
                .map(|realm| realm.ability_overrides.len())
                .sum::<usize>(),
            160
        );
        assert_eq!(
            profile
                .realm_profiles
                .iter()
                .flat_map(|realm| &realm.ability_overrides)
                .filter(|spell| spell.minimum_level == 99)
                .count(),
            15
        );
        let detect = profile
            .realm_profiles
            .iter()
            .find(|realm| realm.realm_id == "nature")
            .unwrap()
            .ability_overrides
            .iter()
            .find(|spell| spell.ability_id == "demo.ability.nature-detect-creatures")
            .unwrap();
        assert_eq!(
            (
                detect.minimum_level,
                detect.resource_cost,
                detect.base_failure_percent,
                detect.first_success_experience
            ),
            (3, 1, 35, Some(6))
        );
    }
}

#[test]
fn equipment_uses_thirty_three_percent_weapon_weight_and_glove_penalty() {
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
    game.items[0]
        .intrinsic_properties
        .status_immunities
        .push(STATUS_PARALYSIS.to_owned());
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened);
    game.items[0].location = ItemLocation::Inventory;
    give_inventory_item(&mut game, "test.weapon", "demo.item.dagger");
    let weapon = game.items.last_mut().unwrap();
    weapon.location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    weapon.intrinsic_weight_tenths_pound = Some(1000);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened);
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(2000);
    game.refresh_player_ability_state();
    assert_eq!(
        game.resources[MANA].maximum,
        unburdened - unburdened * 210 / 1000
    );
    game.items.last_mut().unwrap().intrinsic_weight_tenths_pound = Some(5000);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, 0);
}

#[test]
fn ordinary_easy_spell_and_reduced_mana_equipment_do_not_help_rangers() {
    let mut game = at_level(BUILD, 30);
    let before = game.snapshot().player.abilities;
    let dagger = game
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.dagger")
        .unwrap();
    dagger.intrinsic_properties.passives.extend([
        EquipmentPassive::EasySpell,
        EquipmentPassive::ReducedManaCost,
    ]);
    game.refresh_player_ability_state();
    for (before, after) in before.iter().zip(game.snapshot().player.abilities) {
        assert_eq!(before.id, after.id);
        assert_eq!(
            (before.resource_cost, before.failure_percent),
            (after.resource_cost, after.failure_percent)
        );
    }
    assert!(
        !game
            .player_equipment_passives()
            .contains(&EquipmentPassive::EasySpell)
    );
    assert!(
        !game
            .player_equipment_passives()
            .contains(&EquipmentPassive::ReducedManaCost)
    );
}

#[test]
fn melee_uses_ranger_blows_and_heavy_weapon_limit() {
    let mut game = at_level(BUILD, 50);
    game.progress.attributes.strength = 118;
    game.progress.attributes.dexterity = 118;
    let attack = game.player_melee_profile(&game.player_derived_stats());
    let blows = u32::from(attack.attacks) * 100 + u32::from(attack.extra_attack_chance_percent);
    assert!((101..=500).contains(&blows));
    assert!(
        attack
            .attack_sources
            .iter()
            .any(|source| source.source_id == "demo.class.ranger")
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
fn both_sensing_groups_are_slow_and_strong() {
    let mut game = at_level(BUILD, 1);
    game.progress.attributes.wisdom = 11;
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
    // Source L1/WIS13: both groups use 80000*105/100/161 = 521.
    game.rng = (0..10_000_000)
        .map(RfbRng::seeded)
        .find(|rng| {
            let mut rng = rng.clone();
            rng.bounded(521) == 0 && rng.bounded(521) == 0 && rng.bounded(3) == 0
        })
        .unwrap();
    game.world_tick = 9;
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert_eq!(game.rng, rng);
    assert!(game.item_property_knowledge.is_empty());
    game.world_tick = 10;
    game.process_class_item_sensing();
    for item in &game.items {
        assert_eq!(
            game.item_feeling(item),
            Some(rfb_protocol::ItemFeelingDto::Bad)
        );
    }
    game.item_property_knowledge.clear();
    game.apply_player_mental_status(STATUS_CONFUSION, 10, "test");
    let rng = game.rng.clone();
    game.process_class_item_sensing();
    assert!(game.item_property_knowledge.is_empty());
    assert_eq!(game.rng, rng);
}
