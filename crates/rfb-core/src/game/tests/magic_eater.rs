// SPDX-License-Identifier: MPL-2.0
use super::support::{choose_human_talent_if_pending, clear_monsters, give_inventory_item};
use super::*;

const BUILD: &str = "demo.build.magic-eater";

mod absorption;
mod consumers;
mod generation;
mod usage;

fn at_level(level: u16) -> Game {
    let mut game = Game::new_with_build(925, BUILD).unwrap();
    clear_monsters(&mut game);
    game.apply_player_experience(game.experience_required_for_level(level), &mut Vec::new());
    choose_human_talent_if_pending(&mut game);
    game.player.hp = game.effective_player_max_hp();
    game
}

#[test]
fn birth_has_one_realm_free_build_and_a_real_charged_wand() {
    let game = Game::new_with_build(925, BUILD).unwrap();
    assert!(game.player_is_magic_eater());
    assert_eq!(
        game.content
            .builds()
            .filter(|build| build.class_id == "demo.class.magic-eater")
            .count(),
        1
    );
    let snapshot = game.snapshot();
    assert_eq!(snapshot.player.kind_id, "demo.actor.magic-eater-player");
    let identity = snapshot.player.build.unwrap();
    assert_eq!(
        (identity.life_percent, identity.experience_percent),
        (103, 130)
    );
    let attributes = game.effective_player_attributes();
    assert_eq!(
        [
            attributes.strength,
            attributes.intelligence,
            attributes.wisdom,
            attributes.dexterity,
            attributes.constitution,
            attributes.charisma
        ],
        [12, 15, 14, 15, 11, 11]
    );
    assert!(game.casting_profile().is_none());
    assert!(game.active_casting_realm_profiles().is_empty());
    assert!(game.resources.is_empty());
    assert!(game.learned_abilities.is_empty());
    assert!(
        !snapshot
            .player
            .abilities
            .iter()
            .any(|a| a.source == AbilitySourceDto::Learned)
    );
    assert!(!game.items.iter().any(|item| {
        game.content
            .item(&item.kind_id)
            .unwrap()
            .ability_book_id
            .is_some()
    }));

    let wand = game
        .items
        .iter()
        .find(|item| item.kind_id == "demo.item.magic-missile-wand")
        .unwrap();
    assert_eq!(
        (wand.quantity, &wand.location),
        (1, &ItemLocation::Inventory)
    );
    assert!(wand.activation.is_some());
    let charges = wand.charges.as_ref().unwrap();
    assert!(charges.current > 0 && charges.current <= charges.maximum);
    for kind in ["demo.item.short-sword", "demo.item.soft-leather-armour"] {
        assert!(game.items.iter().any(|item| item.kind_id == kind
            && item.quantity == 1
            && matches!(item.location, ItemLocation::Equipped { .. })));
    }
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn growth_retains_device_skills_and_never_creates_public_mana() {
    let mut game = at_level(1);
    let mut hp = 0;
    for level in 1..=50 {
        game.apply_player_experience(
            game.experience_required_for_level(level)
                .saturating_sub(game.progress.experience),
            &mut Vec::new(),
        );
        choose_human_talent_if_pending(&mut game);
        assert_eq!(game.progress.level, level);
        assert!(game.effective_player_max_hp() >= hp);
        hp = game.effective_player_max_hp();
        assert!(game.resources.is_empty());
        assert!(game.casting_profile().is_none());
    }
    // Human and Ordinary add no skill growth; source extra skills are per ten levels.
    for (id, expected) in [
        ("demo.skill.device", 122),
        ("demo.skill.disarming", 60),
        ("demo.skill.melee", 113),
        ("demo.skill.perception", 26),
        ("demo.skill.ranged", 90),
        ("demo.skill.saving-throw", 86),
        ("demo.skill.search", 20),
        ("demo.skill.stealth", 2),
    ] {
        assert_eq!(game.progress.skills[id].current, expected, "{id}");
    }
    let class = game.character_definitions().unwrap().2;
    assert_eq!((class.base_hp, class.pet_upkeep_divisor), (6, 30));
    assert!(!class.uses_spell_scrolls);
    let restored = Game::from_save(game.to_save()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.rng, game.rng);
}

#[test]
fn proficiencies_and_virtues_use_magic_eater_source_values() {
    let game = at_level(1);
    let snapshot = game.snapshot();
    for (id, initial, maximum) in [
        ("demo.item.dagger", 4000, 6000),
        ("demo.item.short-sword", 4000, 6000),
        ("demo.item.long-sword", 2000, 6000),
        ("demo.item.sling", 2000, 6000),
        ("demo.item.short-bow", 2000, 4000),
        ("demo.item.poison-needle", 2000, 8000),
        ("demo.item.nunchaku", 0, 0),
        ("demo.item.wizardstaff", 0, 0),
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
    assert_eq!(
        game.character_definitions()
            .unwrap()
            .2
            .dual_wielding_maximum,
        4000
    );
    assert_eq!(game.virtues[0].kind, VirtueKindDto::Enchantment);
    assert_eq!(game.virtues[1].kind, VirtueKindDto::Knowledge);
}

#[test]
fn melee_uses_five_blow_limit_and_respects_heavy_weapons() {
    let mut game = at_level(50);
    game.progress.attributes.strength = 218;
    game.progress.attributes.dexterity = 218;
    let attack = game.player_melee_profile(&game.player_derived_stats());
    assert_eq!((attack.attacks, attack.extra_attack_chance_percent), (5, 0));
    assert!(
        attack
            .attack_sources
            .iter()
            .any(|source| source.source_id == "demo.class.magic-eater")
    );
    assert!(
        !attack
            .melee_skill
            .contributions
            .iter()
            .any(|source| source.source_id == "rfb.class.priest-unblessed-weapon")
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
fn device_skill_uses_equipment_and_mutation_contributions_without_mana() {
    let mut game = at_level(1);
    let base = game.player_derived_stats().device_skill.value;
    give_inventory_item(&mut game, "test.device-gloves", "demo.item.leather-gloves");
    let gloves = game
        .items
        .iter_mut()
        .find(|item| item.id == "test.device-gloves")
        .unwrap();
    gloves.intrinsic_properties.equipment_bonuses.device_skill = 16;
    // Inventory properties do not become equipped bonuses.
    assert_eq!(game.player_derived_stats().device_skill.value, base);
    game.items
        .iter_mut()
        .find(|item| item.id == "test.device-gloves")
        .unwrap()
        .location = ItemLocation::Equipped {
        slot_id: "hands".to_owned(),
    };
    assert_eq!(game.player_derived_stats().device_skill.value, base + 16);
    game.progress
        .active_mutation_ids
        .insert("rfb.mutation.human-chr".to_owned());
    assert_eq!(game.player_derived_stats().device_skill.value, base + 6);
    game.refresh_player_ability_state();
    assert!(game.resources.is_empty());
}

#[test]
fn racial_birth_and_draconian_growth_keep_the_ordinary_class_multiplier() {
    for race in ["rfb-legacy.race.spectre", "rfb-legacy.race.draconian-red"] {
        let mut game = Game::new_with_build_race_and_name(925, BUILD, race, "Magic-Eater").unwrap();
        assert!(game.player_is_magic_eater());
        assert!(game.casting_profile().is_none());
        if race == "rfb-legacy.race.draconian-red" {
            game.apply_player_experience(game.experience_required_for_level(50), &mut Vec::new());
            assert_eq!(game.draconian_metamorphosis_attack_level(), 105);
        }
        let restored = Game::from_save(game.to_save()).unwrap();
        assert_eq!(restored.state_hash(), game.state_hash(), "{race}");
        assert_eq!(restored.rng, game.rng);
    }
}
