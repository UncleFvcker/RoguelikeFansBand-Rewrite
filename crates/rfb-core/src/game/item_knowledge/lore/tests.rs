// SPDX-License-Identifier: MPL-2.0
use super::*;
use crate::game::inventory::ItemIdentificationRequest;
use crate::game::tests::support::give_inventory_item;

fn empty_game() -> Game {
    let mut game = Game::new(609);
    game.items.clear();
    game.item_property_knowledge.clear();
    game.item_lore = Default::default();
    game
}

#[test]
fn curse_bits_split_with_stacks_survive_forgetting_and_reject_foreign_saved_bits() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "arrows", "demo.item.arrow");
    game.items[0].quantity = 4;
    game.items[0].curse = Some(ItemCurseSeverityDto::Normal);
    game.items[0]
        .intrinsic_curse_effects
        .extend([ItemCurseEffectDto::DrainHp, ItemCurseEffectDto::LowArmor]);
    game.learn_item_curse("arrows", ItemCurseEffectDto::DrainHp);
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert!(game.item_curse_effect_is_known(&game.items[0], ItemCurseEffectDto::DrainHp));
    game.drop_inventory_quantity("arrows", 2).unwrap().unwrap();
    let split = game
        .items
        .iter()
        .find(|item| matches!(item.location, ItemLocation::Ground(_)))
        .unwrap()
        .id
        .clone();
    assert_eq!(
        game.item_property_knowledge[&split].known_curse_flags,
        game.item_property_knowledge["arrows"].known_curse_flags
    );
    game.learn_item_curse(&split, ItemCurseEffectDto::LowArmor);
    game.pick_up_item_at_player(Some(&split)).unwrap();
    assert_eq!(
        game.items.len(),
        2,
        "different curse knowledge cannot merge"
    );
    let saved = game.to_save();
    let restored = Game::from_save(saved.clone(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert!(restored.item_curse_effect_is_known(
        restored.items.iter().find(|item| item.id == split).unwrap(),
        ItemCurseEffectDto::LowArmor
    ));
    for foreign in [
        8,
        super::super::curses::curse_effect_flag(ItemCurseEffectDto::DrainMana),
    ] {
        let mut invalid = saved.clone();
        invalid
            .item_property_knowledge
            .iter_mut()
            .find(|entry| entry.item_id == "arrows")
            .unwrap()
            .known_curse_flags |= foreign;
        assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
    }
}

#[test]
fn ordinary_identity_with_no_unknown_flags_is_complete_without_a_full_id_marker() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "plain", "demo.item.dagger");
    give_inventory_item(&mut game, "other", "demo.item.dagger");
    game.items[0].enchantments.to_hit = 3;
    game.items[0]
        .intrinsic_properties
        .rfb_flags
        .insert("SHOW_MODS".into());
    assert!(game.unknown_item_flags(&game.items[0]).is_empty());
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Unexamined
    );
    assert_eq!(game.visible_item_enchantments(&game.items[0]).to_hit, 0);
    let rng = game.rng.clone();
    game.identify_item_instance("plain", ItemIdentificationRequest::new(false));
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Identified
    );
    assert_eq!(game.visible_item_enchantments(&game.items[0]).to_hit, 3);
    assert_eq!(
        game.item_identification(&game.items[1]),
        ItemIdentificationDto::Unexamined
    );
    assert!(!game.item_property_knowledge["plain"].identified);
    assert_eq!(game.rng, rng);
    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
    assert_eq!(restored.state_hash(), game.state_hash());
    restored.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        restored.item_identification(
            restored
                .items
                .iter()
                .find(|item| item.id == "plain")
                .unwrap()
        ),
        ItemIdentificationDto::Identified
    );
    assert!(!restored.item_property_knowledge["plain"].identified);
}

#[test]
fn last_shared_flag_completes_known_egos_but_not_an_unknown_identity_or_extra_power() {
    let mut game = empty_game();
    for id in ["first", "second", "extra", "unexamined"] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
        game.items.last_mut().unwrap().quality = ItemQualityDto::Fine;
        game.items
            .last_mut()
            .unwrap()
            .affix_ids
            .push("demo.affix.frost-hunter".into());
    }
    game.items[2]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    for id in ["first", "second", "extra"] {
        game.identify_item_instance(id, ItemIdentificationRequest::new(false));
    }
    game.learn_item_flag("first", "BRAND_COLD");
    assert_eq!(
        game.item_identification(&game.items[1]),
        ItemIdentificationDto::Appraised
    );
    game.learn_item_flag("first", "SLAY_ANIMAL");
    assert_eq!(
        game.item_identification(&game.items[1]),
        ItemIdentificationDto::Identified
    );
    assert_eq!(
        game.item_identification(&game.items[2]),
        ItemIdentificationDto::Appraised
    );
    assert_eq!(
        game.unknown_item_flags(&game.items[2]),
        BTreeSet::from(["BRAND_FIRE".into()])
    );
    assert_eq!(
        game.item_identification(&game.items[3]),
        ItemIdentificationDto::Unexamined
    );
    assert!(!game.item_property_knowledge["second"].identified);
    game.learn_item_flag("extra", "BRAND_FIRE");
    assert_eq!(
        game.item_identification(&game.items[2]),
        ItemIdentificationDto::Identified
    );
    // Completion is derived, not latched: a subsequently added unknown power
    // must restore the partial state until it is observed.
    game.items[2]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Acid);
    assert_eq!(
        game.item_identification(&game.items[2]),
        ItemIdentificationDto::Appraised
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.snapshot(), game.snapshot());
}

#[test]
fn known_ego_ids_do_not_complete_unknown_instance_powers_or_unmapped_properties() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "ego", "demo.item.dagger");
    game.items[0]
        .affix_ids
        .push("demo.affix.frost-hunter".into());
    game.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    game.identify_item_instance("ego", ItemIdentificationRequest::new(false));
    game.item_property_knowledge
        .get_mut("ego")
        .unwrap()
        .known_affix_ids
        .insert("demo.affix.frost-hunter".into());
    assert_eq!(
        game.unknown_item_flags(&game.items[0]),
        BTreeSet::from(["BRAND_FIRE".into()])
    );
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Appraised
    );
    game.learn_item_flag("ego", "BRAND_FIRE");
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Identified
    );
    game.items[0]
        .intrinsic_properties
        .equipment_bonuses
        .disarming_skill = 5;
    assert!(game.unknown_item_flags(&game.items[0]).is_empty());
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Appraised
    );
    assert_eq!(
        game.visible_item_equipment_bonuses(&game.items[0])
            .disarming_skill,
        0
    );
    game.identify_item_instance("ego", ItemIdentificationRequest::new(true));
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Identified
    );
    assert_eq!(
        game.visible_item_equipment_bonuses(&game.items[0])
            .disarming_skill,
        5
    );
}

#[test]
fn single_flag_moves_to_ego_lore_without_revealing_unknown_ego_identity() {
    let mut game = empty_game();
    for id in ["first", "second"] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
        game.items.last_mut().unwrap().quality = ItemQualityDto::Fine;
        game.items
            .last_mut()
            .unwrap()
            .affix_ids
            .push("demo.affix.frost-hunter".into());
    }
    assert!(game.learn_item_flag("first", "BRAND_COLD"));
    assert!(!game.learn_item_flag("first", "BRAND_FIRE"));
    assert_eq!(
        game.visible_item_brands(&game.items[0]),
        vec![WeaponBrandDto::Cold]
    );
    assert!(game.visible_item_slays(&game.items[0]).is_empty());
    assert!(game.known_item_properties(&game.items[0]).is_empty());
    assert!(game.item_lore.egos.is_empty());

    game.identify_item_instance("first", ItemIdentificationRequest::new(false));
    assert!(game.item_lore.egos["demo.affix.frost-hunter"].contains("BRAND_COLD"));
    assert!(
        !game.item_property_knowledge["first"]
            .known_flags
            .contains("BRAND_COLD")
    );
    assert!(game.visible_item_brands(&game.items[1]).is_empty());
    assert!(game.known_item_properties(&game.items[1]).is_empty());
    game.identify_item_instance("second", ItemIdentificationRequest::new(false));
    assert_eq!(
        game.visible_item_brands(&game.items[1]),
        vec![WeaponBrandDto::Cold]
    );
    assert!(game.visible_item_slays(&game.items[1]).is_empty());

    let mut restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
    assert_eq!(restored.snapshot(), game.snapshot());
    let lore = restored.item_lore.clone();
    restored.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(restored.item_lore, lore);
    assert_eq!(
        restored.item_identification(&restored.items[1]),
        ItemIdentificationDto::Unexamined
    );
    assert!(restored.visible_item_brands(&restored.items[1]).is_empty());
    restored.identify_item_instance("second", ItemIdentificationRequest::new(false));
    assert_eq!(
        restored.visible_item_brands(&restored.items[1]),
        vec![WeaponBrandDto::Cold]
    );
    restored.identify_item_instance("first", ItemIdentificationRequest::new(true));
    assert!(!restored.visible_item_slays(&restored.items[1]).is_empty());
}

#[test]
fn flags_split_with_the_stack_but_different_knowledge_does_not_merge() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "arrows", "demo.item.arrow");
    game.items[0].quantity = 4;
    game.items[0]
        .affix_ids
        .push("demo.affix.frost-hunter".into());
    game.items[0].quality = ItemQualityDto::Fine;
    game.learn_item_flag("arrows", "BRAND_COLD");
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        game.visible_item_brands(&game.items[0]),
        vec![WeaponBrandDto::Cold]
    );
    game.drop_inventory_quantity("arrows", 2).unwrap().unwrap();
    let split = game
        .items
        .iter()
        .find(|item| matches!(item.location, ItemLocation::Ground(_)))
        .unwrap()
        .id
        .clone();
    assert_eq!(
        game.item_property_knowledge[&split].known_flags,
        game.item_property_knowledge["arrows"].known_flags
    );
    // One stack learns a second flag: picking it up must preserve two knowledge states.
    game.learn_item_flag(&split, "SLAY_ANIMAL");
    game.pick_up_item_at_player(Some(&split)).unwrap();
    assert_eq!(game.items.len(), 2);
    assert!(game.visible_item_slays(&game.items[0]).is_empty());
    assert!(!game.visible_item_slays(&game.items[1]).is_empty());
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.state_hash(), game.state_hash());
}

#[test]
fn fixed_artifact_lore_keeps_random_properties_local_and_survives_forgetting() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "artifact", "demo.item.thorin");
    game.generated_artifact_ids
        .insert("demo.item.thorin".into());
    game.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Cold);
    game.learn_item_flag("artifact", "STR");
    game.learn_item_flag("artifact", "BRAND_COLD");
    assert_eq!(game.visible_item_modifiers(&game.items[0]).strength, 4);
    assert_eq!(game.visible_item_modifiers(&game.items[0]).constitution, 0);
    game.identify_item_instance("artifact", ItemIdentificationRequest::new(false));
    assert!(game.item_lore.artifacts["demo.item.thorin"].contains("STR"));
    assert!(!game.item_lore.artifacts["demo.item.thorin"].contains("BRAND_COLD"));
    assert!(
        game.item_property_knowledge["artifact"]
            .known_flags
            .contains("BRAND_COLD")
    );
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(game.visible_item_modifiers(&game.items[0]).strength, 0);
    assert_eq!(
        game.visible_item_brands(&game.items[0]),
        vec![WeaponBrandDto::Cold]
    );
    game.identify_item_instance("artifact", ItemIdentificationRequest::new(true));
    assert!(game.item_lore.artifacts["demo.item.thorin"].contains("CON"));
    game.lose_mindcraft_information(&mut BTreeSet::new());
    assert_eq!(
        game.item_identification(&game.items[0]),
        ItemIdentificationDto::Identified
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.item_lore, game.item_lore);
}

#[test]
fn cursed_ego_biffs_stay_local_while_observed_random_powers_are_shared() {
    let mut game = empty_game();
    for id in ["cursed", "other"] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
        let item = game.items.last_mut().unwrap();
        item.affix_ids.push("demo.affix.frost-hunter".into());
        item.intrinsic_properties.modifiers.strength = -1;
        item.curse = Some(ItemCurseSeverityDto::Normal);
    }
    game.identify_item_instance("cursed", ItemIdentificationRequest::new(true));
    assert!(!game.item_lore.egos["demo.affix.frost-hunter"].contains("DEC_STR"));
    assert!(
        game.item_property_knowledge["cursed"]
            .known_flags
            .contains("DEC_STR")
    );
    game.identify_item_instance("other", ItemIdentificationRequest::new(false));
    assert_eq!(game.visible_item_modifiers(&game.items[1]).strength, 0);
    assert_eq!(
        game.visible_item_brands(&game.items[1]),
        vec![WeaponBrandDto::Cold]
    );
    game.items[0].curse = None;
    game.items[0]
        .intrinsic_properties
        .brands
        .insert(WeaponBrand::Fire);
    game.identify_item_instance("cursed", ItemIdentificationRequest::new(true));
    assert!(game.item_lore.egos["demo.affix.frost-hunter"].contains("BRAND_FIRE"));
    // Knowing an ego's possible fire brand does not invent it on another instance.
    assert_eq!(
        game.visible_item_brands(&game.items[1]),
        vec![WeaponBrandDto::Cold]
    );
}

#[test]
fn activation_knowledge_is_independent_and_does_not_leak_to_an_unknown_override() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "fixed", "demo.item.galadriel");
    game.generated_artifact_ids
        .insert("demo.item.galadriel".into());
    game.equip_inventory_item("fixed", None).unwrap();
    game.identify_item_instance("fixed", ItemIdentificationRequest::new(false));
    assert!(!game.item_activation_is_known(&game.items[0]));
    for _ in 0..100 {
        game.use_inventory_item(
            "fixed",
            None,
            None,
            &mut Vec::new(),
            &mut BTreeSet::new(),
            &mut Vec::new(),
        )
        .unwrap();
        if game.items[0].charges.unwrap().current == 0 {
            break;
        }
    }
    assert_eq!(game.items[0].charges.unwrap().current, 0);
    assert!(game.item_activation_is_known(&game.items[0]));
    assert!(game.item_lore.artifacts["demo.item.galadriel"].contains("ACTIVATE"));
    assert!(game.item_lore.activation_profiles.is_empty());

    let profiles = &game
        .content
        .random_artifact_generation()
        .unwrap()
        .device_generation
        .activations;
    let (first, charges) = super::super::super::ego::materialize_rfb_activation(
        profiles
            .iter()
            .find(|p| p.id.ends_with(".lite-area"))
            .unwrap(),
    );
    let (other, _) = super::super::super::ego::materialize_rfb_activation(
        profiles
            .iter()
            .find(|p| p.id.ends_with(".detect-objects"))
            .unwrap(),
    );
    for (id, activation) in [
        ("random-a", first.clone()),
        ("random-b", first),
        ("random-c", other),
    ] {
        give_inventory_item(&mut game, id, "demo.item.dagger");
        let item = game.items.last_mut().unwrap();
        item.artifact_name = Some("(永恒蘑菇)".into());
        item.activation = Some(activation);
        item.charges = Some(charges);
    }
    game.identify_item_instance("random-a", ItemIdentificationRequest::new(false));
    game.learn_item_activation("random-a");
    assert!(game.item_activation_is_known(&game.items[1]));
    assert!(!game.item_activation_is_known(&game.items[2]));
    game.identify_item_instance("random-b", ItemIdentificationRequest::new(false));
    game.identify_item_instance("random-c", ItemIdentificationRequest::new(false));
    assert!(game.item_activation_is_known(&game.items[2]));
    assert!(!game.item_activation_is_known(&game.items[3]));
    assert_eq!(
        game.item_identification(&game.items[2]),
        ItemIdentificationDto::Identified
    );
    let restored = Game::from_save(game.to_save(), game.behavior_preferences()).unwrap();
    assert_eq!(restored.item_lore, game.item_lore);
    assert!(
        restored.item_activation_is_known(
            restored
                .items
                .iter()
                .find(|item| item.id == "random-b")
                .unwrap()
        )
    );
}

#[test]
fn save_rejects_invalid_lore_flags_references_and_duplicate_instance_flags() {
    let mut game = empty_game();
    give_inventory_item(&mut game, "item", "demo.item.dagger");
    game.identify_item_instance("item", ItemIdentificationRequest::new(false));
    let save = game.to_save();
    let mut invalid = save.clone();
    invalid
        .item_lore
        .egos
        .insert("missing.affix".into(), BTreeSet::from(["STR".into()]));
    assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
    let mut invalid = save.clone();
    invalid.item_property_knowledge[0].known_flags = vec!["STR".into(), "STR".into()];
    assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
    let mut invalid = save.clone();
    invalid.item_lore.egos.insert(
        "demo.affix.frost-hunter".into(),
        BTreeSet::from(["UNKNOWN_FLAG".into()]),
    );
    assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
    let mut invalid = save;
    invalid
        .item_lore
        .activation_profiles
        .insert("missing.activation".into());
    assert!(Game::from_save(invalid, game.behavior_preferences()).is_err());
}
