// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters, give_inventory_item};
use super::*;

mod generation;
mod learning;
mod powers;

const BUILD: &str = "demo.build.warrior-mage-arcane-sorcery";
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
fn eight_births_have_arcane_two_books_equipment_and_valid_unlearned_memory() {
    let content = Game::new_with_build(925, BUILD).unwrap().content;
    let builds = content
        .builds()
        .filter(|b| b.class_id == "demo.class.warrior-mage")
        .collect::<Vec<_>>();
    assert_eq!(builds.len(), 8);
    for build in builds {
        let game = Game::new_with_build(925, &build.id).unwrap();
        let snapshot = game.snapshot();
        assert_eq!(snapshot.player.kind_id, "demo.actor.warrior-mage-player");
        let identity = snapshot.player.build.unwrap();
        assert_eq!(
            (identity.life_percent, identity.experience_percent),
            (105, 140)
        );
        assert_eq!(
            snapshot.player.progress.attributes.intelligence.effective,
            15
        );
        assert_eq!(game.resources[MANA].maximum, 8);
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
        let mut expected = ["arcane", build.second_realm_id.as_deref().unwrap()];
        expected.sort_unstable();
        assert_eq!(books, expected, "{}", build.id);
        for kind in ["demo.item.short-sword", "demo.item.soft-leather-armour"] {
            assert!(game.items.iter().any(|item| item.kind_id == kind
                && matches!(item.location, ItemLocation::Equipped { .. })));
        }
        assert!(
            !game
                .items
                .iter()
                .any(|item| item.kind_id == "demo.item.healing-potion")
        );
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash());
        assert_eq!(restored.rng, game.rng);
    }
}

#[test]
fn growth_uses_intelligence_eighty_four_capacity_and_normal_mana_recovery() {
    let mut game = at_level(BUILD, 1);
    let (mut hp, mut mp) = (0, 0);
    for level in 1..=50 {
        game.apply_player_experience(
            game.experience_required_for_level(level)
                .saturating_sub(game.progress.experience),
            &mut Vec::new(),
        );
        choose_human_talent_if_pending(&mut game);
        assert_eq!(game.progress.level, level);
        assert!(game.effective_player_max_hp() >= hp);
        assert!(game.resources[MANA].maximum >= mp);
        hp = game.effective_player_max_hp();
        mp = game.resources[MANA].maximum;
    }
    assert!(game.progress.skills["demo.skill.device"].current >= 36 + 5 * 10);
    assert!(game.progress.skills["demo.skill.ranged"].current >= 50 + 5 * 15);
    assert_eq!(
        game.character_definitions().unwrap().2.pet_upkeep_divisor,
        35
    );
    game.progress.attributes.intelligence = game.progress.attribute_potentials.intelligence;
    game.progress.maximum_attributes.intelligence = game.progress.attributes.intelligence;
    game.refresh_player_ability_state();
    assert_eq!(
        game.ability_learning_capacity(game.casting_profile().unwrap()),
        84
    );
    let maximum = game.resources[MANA].maximum;
    game.progress.attributes.wisdom = 3;
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, maximum);
    game.progress.attributes.intelligence = 3;
    game.refresh_player_ability_state();
    assert!(game.resources[MANA].maximum < maximum);
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
fn proficiencies_virtues_and_racial_births_use_warrior_mage_rules() {
    let game = at_level(BUILD, 1);
    let snapshot = game.snapshot();
    for (id, initial, maximum) in [
        ("demo.item.short-sword", 4000, 8000),
        ("demo.item.sabre", 4000, 8000),
        ("demo.item.poison-needle", 2000, 8000),
        ("demo.item.wizardstaff", 2000, 6000),
        ("demo.item.nunchaku", 2000, 4000),
        ("demo.item.long-sword", 2000, 6000),
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
    assert_eq!(snapshot.player.progress.riding_proficiency.maximum, 4000);
    assert_eq!(
        game.character_definitions()
            .unwrap()
            .2
            .dual_wielding_maximum,
        6000
    );
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Enchantment);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Valour);
    for race in [
        "rfb-legacy.race.tomte",
        "rfb-legacy.race.tonberry",
        "rfb-legacy.race.spectre",
        "rfb-legacy.race.einheri",
        "rfb-legacy.race.draconian-red",
    ] {
        let mut game =
            Game::new_with_build_race_and_name(925, BUILD, race, "Warrior-Mage").unwrap();
        assert_eq!(game.active_casting_realm_profiles().len(), 2);
        assert_eq!(
            Game::from_save(game.to_save()).unwrap().state_hash(),
            game.state_hash(),
            "{race}"
        );
        if race == "rfb-legacy.race.draconian-red" {
            game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
            // Red Draconian 105% with the ordinary Warrior-Mage class multiplier (100%).
            assert_eq!(game.draconian_metamorphosis_attack_level(), 105);
        }
    }
}

#[test]
fn armor_weapon_and_glove_encumbrance_and_reduced_mana_are_applied() {
    let mut game = at_level(BUILD, 25);
    game.items.clear();
    game.refresh_player_ability_state();
    let unburdened = game.resources[MANA].maximum;
    give_inventory_item(&mut game, "test.gloves", "demo.item.leather-gloves");
    game.items[0].location = ItemLocation::Equipped {
        slot_id: "gloves".to_owned(),
    };
    game.items[0].intrinsic_weight_tenths_pound = Some(0);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened * 3 / 4);
    game.items[0]
        .intrinsic_properties
        .status_immunities
        .push(STATUS_PARALYSIS.to_owned());
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, unburdened);
    give_inventory_item(&mut game, "test.weapon", "demo.item.short-sword");
    game.items[1].location = ItemLocation::Equipped {
        slot_id: "weapon".to_owned(),
    };
    game.items[1].intrinsic_weight_tenths_pound = Some(2000);
    game.refresh_player_ability_state();
    assert_eq!(
        game.resources[MANA].maximum,
        unburdened - unburdened * (660 - 430) / 1200
    );
    game.items[1].location = ItemLocation::Inventory;
    game.items[0].intrinsic_weight_tenths_pound = Some(1630);
    game.refresh_player_ability_state();
    assert_eq!(game.resources[MANA].maximum, 0);
    game.items[0].intrinsic_weight_tenths_pound = Some(0);
    game.refresh_player_ability_state();
    let before = game.snapshot().player.abilities;
    game.items[0]
        .intrinsic_properties
        .passives
        .insert(EquipmentPassive::ReducedManaCost);
    game.refresh_player_ability_state();
    assert!(
        before
            .iter()
            .zip(game.snapshot().player.abilities)
            .any(|(before, after)| before.id == after.id
                && after.resource_cost < before.resource_cost)
    );
}

#[test]
fn melee_uses_source_blows_and_heavy_weapon_limit() {
    let mut game = at_level(BUILD, 50);
    game.progress.attributes.strength = 218;
    game.progress.attributes.dexterity = 218;
    let attack = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!(
        (attack.attacks, attack.extra_attack_chance_percent),
        (5, 25)
    );
    assert!(
        attack
            .attack_sources
            .iter()
            .any(|source| source.source_id == "demo.class.warrior-mage")
    );
    game.items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.short-sword")
        .unwrap()
        .intrinsic_weight_tenths_pound = Some(5000);
    let attack = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((attack.attacks, attack.extra_attack_chance_percent), (1, 0));
}

#[test]
fn class_sensing_uses_weak_first_group_and_strong_second_group() {
    for (build, wisdom, second_chance) in [(BUILD, 14, 130), ("demo.build.magic-eater", 12, 58)] {
        let mut game = at_level(build, 1);
        game.progress.attributes.wisdom = wisdom;
        game.items.clear();
        game.item_property_knowledge.clear();
        give_inventory_item(&mut game, "test.sword", "demo.item.small-sword");
        game.items[0].location = ItemLocation::Equipped {
            slot_id: "weapon".to_owned(),
        };
        game.items[0].curse = Some(ItemCurseSeverityDto::Heavy);
        give_inventory_item(&mut game, "test.wand", "demo.item.magic-missile-wand");
        game.items[1].curse = Some(ItemCurseSeverityDto::Heavy);
        assert_eq!(game.effective_player_attributes().wisdom, 13);
        assert_eq!(game.virtue_current(VirtueKindDto::Knowledge), 0);
        // Source L1/WIS13: MED = 20000*105/100/161 = 130; FAST = 58.
        game.rng = (0..1_000_000)
            .map(RfbRng::seeded)
            .find(|rng| {
                let mut rng = rng.clone();
                rng.bounded(130) == 0 && rng.bounded(second_chance) == 0 && rng.bounded(3) == 0
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
}

#[test]
fn spell_projection_keeps_three_orbs_and_unreachable_spells() {
    for (realm, ability) in [
        ("death", "death-entropy-orb"),
        ("daemon", "daemon-hellish-flame"),
        ("crusade", "crusade-holy-orb"),
    ] {
        for (level, bonus, radius) in [(29, 36, 2), (30, 37, 3), (50, 62, 3)] {
            let game = at_level(&format!("demo.build.warrior-mage-arcane-{realm}"), level);
            let spell = game
                .snapshot()
                .player
                .abilities
                .into_iter()
                .find(|a| a.id == format!("demo.ability.{ability}"))
                .unwrap();
            assert!(spell.effects.iter().any(|e| matches!(e, AbilityEffectSpecDto::AreaDamage { damage_bonus, radius: r, .. } if *damage_bonus == bonus && *r == radius)));
        }
    }
    for (realm, ability) in [("craft", "craft-mana-brand"), ("life", "life-warding-true")] {
        let game = at_level(&format!("demo.build.warrior-mage-arcane-{realm}"), 50);
        let spell = game
            .snapshot()
            .player
            .abilities
            .into_iter()
            .find(|a| a.id == format!("demo.ability.{ability}"))
            .unwrap();
        assert_eq!(spell.minimum_level, 99);
        assert!(!spell.can_study);
    }
}

#[test]
fn smart_monster_anti_magic_weight_targets_the_player_class_and_existing_anti_magic() {
    use super::support::replace_terrain;
    for (build, weight) in [
        (BUILD, 20),
        ("demo.build.duelist", 10),
        ("demo.build.magic-eater", 50),
    ] {
        let mut game = at_level(build, 25);
        game.player.position = Position { x: 10, y: 10 };
        for x in 10..=13 {
            replace_terrain(&mut game, Position { x, y: 10 }, "demo.terrain.floor");
        }
        game.push_generated_actor(
            "test.archlich".to_owned(),
            "demo.actor.archlich",
            Position { x: 13, y: 10 },
        );
        game.entities[0].alerted = true;
        game.entities[0].statuses.clear();
        let ability = game
            .content
            .ability("rfb-legacy.ability.anti-magic")
            .unwrap()
            .clone();
        assert_eq!(
            game.monster_ability_plan(0, ability.clone(), 1)
                .unwrap()
                .effective_weight,
            weight
        );
        assert_eq!(
            game.monster_ability_plan(0, ability.clone(), 97)
                .unwrap()
                .effective_weight,
            weight
        );
        let mut ordinary = game.clone();
        ordinary.entities[0].kind_id = "demo.actor.magic-mushroom-patch".to_owned();
        assert!(
            ordinary
                .monster_ability_plan(0, ability.clone(), 97)
                .unwrap()
                .effective_weight
                >= 97
        );
        let mut ally = game.clone();
        ally.entities[0].controller_id = Some(ally.player.id.clone());
        ally.push_generated_actor(
            "test.sheep".to_owned(),
            "demo.actor.sheep",
            Position { x: 11, y: 10 },
        );
        let plan = ally.monster_ability_plan(0, ability.clone(), 97).unwrap();
        assert!(plan.effective_weight >= 97);
        let mut blocked = game.clone();
        blocked.apply_player_mental_status(crate::effect::STATUS_ANTI_MAGIC, 10, "test");
        assert_eq!(
            blocked
                .monster_ability_plan(0, ability.clone(), 1)
                .unwrap_err()
                .reason,
            MonsterAbilityRejectionReasonDto::NoUtility
        );
        game.items
            .iter_mut()
            .find(|item| matches!(item.location, ItemLocation::Equipped { .. }))
            .unwrap()
            .intrinsic_properties
            .passives
            .insert(EquipmentPassive::AntiMagic);
        assert_eq!(
            game.monster_ability_plan(0, ability, 1).unwrap_err().reason,
            MonsterAbilityRejectionReasonDto::NoUtility
        );
    }
}
