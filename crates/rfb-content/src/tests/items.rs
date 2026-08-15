use super::*;
use std::collections::BTreeSet;

#[test]
fn p90b_olog_hai_reward_keeps_original_armour_affix_and_activation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.metal-lamellar-armour")
        .expect("Metal Lamellar Armour should exist");
    assert_eq!(item.generation_level, 45);
    assert_eq!(item.weight_tenths_pound, 340);
    assert_eq!(item.base_value, 1_150);
    assert_eq!(item.equipment_slot.as_deref(), Some("body"));
    assert_eq!(item.modifiers.defense, 23);
    assert_eq!(item.equipment_bonuses.melee_skill, -3);
    let allocation = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Metal Lamellar Armour should retain its base allocation");
    assert_eq!(allocation.weight, 100);
    assert_eq!((allocation.min_depth, allocation.max_depth), (45, u16::MAX));

    let affix = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.olog-hai")
        .expect("Olog-hai affix should exist");
    assert!(affix_is_compatible_with_item(affix, item, 36));
    assert_eq!(affix.modifiers.strength, 4);
    assert_eq!(affix.modifiers.intelligence, -4);
    assert_eq!(affix.modifiers.defense, 10);
    assert_eq!(affix.equipment_bonuses.melee_damage, 7);
    assert_eq!(
        affix.resistances.get(&ActorDamageType::Acid),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        affix.resistances.get(&ActorDamageType::Poison),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(affix.passives.contains(&EquipmentPassive::Regeneration));
    assert!(
        affix
            .elemental_destruction_immunities
            .contains(&ItemDestructionElement::Acid)
    );

    let roll_group = affix.roll_groups.as_slice();
    let [roll_group] = roll_group else {
        panic!("Olog-hai should roll one high resistance group");
    };
    assert_eq!(roll_group.rolls, 1);
    assert_eq!(roll_group.candidates.len(), 12);
    assert!(
        roll_group
            .candidates
            .iter()
            .all(|candidate| candidate.weight == 1)
    );
    let rolled_resistances = roll_group
        .candidates
        .iter()
        .flat_map(|candidate| candidate.properties.resistances.keys().copied())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        rolled_resistances,
        BTreeSet::from([
            ActorDamageType::Poison,
            ActorDamageType::Light,
            ActorDamageType::Dark,
            ActorDamageType::Shards,
            ActorDamageType::Blindness,
            ActorDamageType::Confusion,
            ActorDamageType::Sound,
            ActorDamageType::Nether,
            ActorDamageType::Nexus,
            ActorDamageType::Chaos,
            ActorDamageType::Disenchant,
            ActorDamageType::Fear,
        ])
    );

    let generation = affix
        .device_generation
        .as_ref()
        .expect("Olog-hai should provide a device activation");
    assert_eq!(
        generation.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 50,
            energy_per_mille: 1_000,
        })
    );
    let [activation] = generation.activations.as_slice() else {
        panic!("Olog-hai should provide exactly one activation");
    };
    assert_eq!(activation.device_check_difficulty, 10);
    assert_eq!(
        activation.charges,
        ItemDeviceChargeRangeDefinition {
            minimum: 1,
            maximum: 1,
            cost: 1,
        }
    );
    assert!(matches!(
        activation.effect,
        ItemUseEffectDefinition::ApplyBerserkStrength {
            duration_dice: 1,
            duration_sides: 25,
            duration_bonus: 25,
        }
    ));

    let reward = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.troll-cave-final-reward")
        .expect("Troll cave reward table should exist");
    assert_eq!(reward.rolls, 1);
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(
        reward.entries[0].item_kind_id,
        "demo.item.metal-lamellar-armour"
    );
    assert_eq!(reward.quality_weights[0].quality, ItemQuality::Fine);
    assert_eq!(
        reward.affix_weights[0].affix_id.as_deref(),
        Some("rfb-legacy.affix.olog-hai")
    );
}

#[test]
fn p96b_trifurcate_spear_wrath_and_shared_fallback_match_rfb() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let spear = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.trifurcate-spear")
        .expect("Trifurcate Spear should exist");
    assert_eq!(spear.generation_level, 35);
    assert_eq!(spear.weight_tenths_pound, 140);
    assert_eq!(spear.base_value, 400);
    assert_eq!(spear.equipment_slot.as_deref(), Some("weapon"));
    assert_eq!(
        spear.riding_weapon_kind,
        Some(RidingWeaponKindDefinition::Compatible)
    );
    let melee = spear
        .melee_profile
        .as_ref()
        .expect("Trifurcate Spear should be a melee weapon");
    assert_eq!((melee.damage_dice, melee.damage_sides), (2, 10));
    assert_eq!((melee.to_hit, melee.to_damage), (0, 0));

    let allocation = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == spear.id)
        })
        .expect("Trifurcate Spear should retain its base allocation");
    assert_eq!(allocation.weight, 33);
    assert_eq!((allocation.min_depth, allocation.max_depth), (35, u16::MAX));

    let wrath = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.trifurcate-spear-of-wrath")
        .expect("Trifurcate Spear of Wrath should exist");
    assert_eq!(wrath.generation_level, 70);
    assert_eq!(wrath.weight_tenths_pound, 300);
    assert_eq!(wrath.base_value, 90_000);
    assert_eq!(
        wrath.weapon_proficiency_base_item_id.as_deref(),
        Some("demo.item.trifurcate-spear")
    );
    assert_eq!(
        wrath.riding_weapon_kind,
        Some(RidingWeaponKindDefinition::Compatible)
    );
    let generation = wrath
        .artifact_generation
        .as_ref()
        .expect("Wrath should retain fixed-artifact generation data");
    assert_eq!(generation.source_index, 107);
    assert_eq!(generation.base_item_kind_id, "demo.item.trifurcate-spear");
    assert_eq!(generation.rarity_one_in, 12);
    assert!(!generation.instant);
    assert_eq!(wrath.modifiers.strength, 2);
    assert_eq!(wrath.modifiers.dexterity, 2);
    let melee = wrath
        .melee_profile
        .as_ref()
        .expect("Wrath should be a melee weapon");
    assert_eq!((melee.damage_dice, melee.damage_sides), (3, 10));
    assert_eq!((melee.to_hit, melee.to_damage), (16, 18));
    assert!(wrath.brands.contains(&WeaponBrand::Chaos));
    assert_eq!(wrath.slays.get(&SlayTarget::Evil), Some(&SlayLevel::Slay));
    assert_eq!(wrath.slays.get(&SlayTarget::Undead), Some(&SlayLevel::Slay));
    assert_eq!(
        wrath.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert_eq!(
        wrath.resistances.get(&ActorDamageType::Dark),
        Some(&ActorResistanceLevel::Resistant)
    );
    assert!(wrath.passives.contains(&EquipmentPassive::SeeInvisible));
    assert!(wrath.tags.iter().any(|tag| tag == "blessed-weapon"));
    assert!(wrath.resists_monster_destruction);
    assert!(wrath.resists_projection_destruction);

    let fallback = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.trifurcate-spear-final-replacement")
        .expect("shared Wrath fallback should exist");
    assert_eq!(fallback.rolls, 1);
    assert_eq!(fallback.entries.len(), 1);
    assert_eq!(fallback.entries[0].item_kind_id, spear.id);
    assert_eq!(
        fallback.quality_weights[0].quality,
        ItemQuality::Exceptional
    );
    assert!(fallback.affix_weights[0].affix_id.is_none());
}

#[test]
fn p97c_multi_hued_dragon_scale_mail_keeps_resists_and_random_breath() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.multi-hued-dragon-scale-mail")
        .expect("Multi-Hued Dragon Scale Mail should exist");
    assert_eq!((item.generation_level, item.weight_tenths_pound), (75, 200));
    assert_eq!(item.base_value, 150_000);
    assert_eq!(item.equipment_slot.as_deref(), Some("body"));
    assert_eq!(item.modifiers.defense, 50);
    assert_eq!(item.equipment_bonuses.melee_skill, -2);
    assert!(item.mogaminator_rare);
    assert!(item.tags.iter().any(|tag| tag == "activatable"));
    for damage_type in [
        ActorDamageType::Acid,
        ActorDamageType::Electricity,
        ActorDamageType::Fire,
        ActorDamageType::Cold,
        ActorDamageType::Poison,
    ] {
        assert_eq!(
            item.resistances.get(&damage_type),
            Some(&ActorResistanceLevel::Resistant)
        );
    }
    assert_eq!(
        item.elemental_destruction_immunities,
        BTreeSet::from([
            ItemDestructionElement::Acid,
            ItemDestructionElement::Cold,
            ItemDestructionElement::Electricity,
            ItemDestructionElement::Fire,
        ])
    );
    assert!(item.elemental_destruction_vulnerabilities.is_empty());

    let generation = item
        .device_generation
        .as_ref()
        .expect("dragon scale mail should keep its activation");
    assert_eq!(
        generation.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 700,
            energy_per_mille: 1_000,
        })
    );
    let [activation] = generation.activations.as_slice() else {
        panic!("dragon scale mail should have exactly one activation");
    };
    assert_eq!(activation.device_check_difficulty, 40);
    assert_eq!(
        activation.target.modes,
        [AbilityTargetModeDefinition::Direction]
    );
    assert_eq!(activation.target.range, 18);
    assert!(activation.target.requires_line_of_effect);
    assert!(matches!(
        &activation.effect,
        ItemUseEffectDefinition::RandomElementConeDamage {
            damage: 250,
            damage_types,
            radius: 2,
        } if damage_types == &[
            ActorDamageType::Acid,
            ActorDamageType::Electricity,
            ActorDamageType::Fire,
            ActorDamageType::Cold,
            ActorDamageType::Poison,
        ]
    ));

    let allocation = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("dragon scale mail should retain its base allocation");
    assert_eq!(allocation.weight, 12);
    assert_eq!((allocation.min_depth, allocation.max_depth), (75, u16::MAX));

    let reward = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.dragon-lair-final-reward")
        .expect("Dragon's Lair reward table should exist");
    assert_eq!(reward.rolls, 1);
    assert_eq!(reward.entries.len(), 1);
    assert_eq!(reward.entries[0].item_kind_id, item.id);
    assert_eq!(reward.quality_weights[0].quality, ItemQuality::Ordinary);
    assert_eq!(reward.affix_weights.len(), 1);
    assert_eq!(reward.affix_weights[0].affix_id, None);
    assert_eq!(reward.affix_weights[0].weight, 1);
}

#[test]
fn p99c_paurnimmen_keeps_artifact_identity_and_fixed_cold_beam() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.set-of-gauntlets-paurnimmen")
        .expect("Paurnimmen should exist");
    let generation = item
        .artifact_generation
        .as_ref()
        .expect("Paurnimmen should retain artifact generation");
    assert_eq!(generation.source_index, 185);
    assert_eq!(generation.base_item_kind_id, "demo.item.set-of-gauntlets");
    assert_eq!(generation.rarity_one_in, 20);
    assert_eq!((item.generation_level, item.weight_tenths_pound), (30, 25));
    assert_eq!(item.base_value, 13_000);
    assert_eq!((item.modifiers.attack, item.modifiers.defense), (2, 9));
    assert_eq!(item.brands, BTreeSet::from([WeaponBrand::Cold]));
    assert_eq!(
        item.resistances.get(&ActorDamageType::Cold),
        Some(&ActorResistanceLevel::Resistant)
    );

    let device = item
        .device_generation
        .as_ref()
        .expect("Paurnimmen should keep its activation");
    assert_eq!(
        device.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 120,
            energy_per_mille: 1_000,
        })
    );
    let [activation] = device.activations.as_slice() else {
        panic!("Paurnimmen should have one activation");
    };
    assert_eq!(activation.device_check_difficulty, 12);
    assert_eq!(
        activation.target.modes,
        [AbilityTargetModeDefinition::Direction]
    );
    assert!(matches!(
        activation.effect,
        ItemUseEffectDefinition::BeamDamage {
            damage_dice: 0,
            damage_sides: 0,
            damage_bonus: 40,
            damage_type: ActorDamageType::Cold,
        }
    ));

    let fallback = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.set-of-gauntlets-final-replacement")
        .expect("Paurnimmen should have a unique-artifact fallback");
    assert_eq!(
        fallback.entries[0].item_kind_id,
        "demo.item.set-of-gauntlets"
    );
    assert_eq!(
        fallback.quality_weights[0].quality,
        ItemQuality::Exceptional
    );
}

#[test]
fn p100c_soulsword_keeps_life_bonus_and_exact_extra_power_distribution() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.soulsword")
        .expect("Soulsword should exist");
    let generation = item
        .artifact_generation
        .as_ref()
        .expect("Soulsword should retain artifact generation");
    assert_eq!(generation.source_index, 89);
    assert_eq!(generation.base_item_kind_id, "demo.item.scimitar");
    assert_eq!(generation.rarity_one_in, 20);
    assert_eq!(
        generation.affix_ids,
        ["rfb-legacy.affix.artifact-extra-res-or-power"]
    );
    assert_eq!((item.generation_level, item.weight_tenths_pound), (40, 130));
    assert_eq!(item.base_value, 111_111);
    assert_eq!((item.modifiers.intelligence, item.modifiers.wisdom), (3, 3));
    assert_eq!(item.equipment_bonuses.life_percent, 9);
    let melee = item
        .melee_profile
        .as_ref()
        .expect("Soulsword melee profile");
    assert_eq!((melee.damage_dice, melee.damage_sides), (3, 6));
    assert_eq!((melee.to_hit, melee.to_damage), (9, 11));
    assert_eq!(item.slays.len(), 5);
    assert_eq!(item.resistances.len(), 4);
    assert!(item.passives.contains(&EquipmentPassive::SeeInvisible));
    assert!(item.passives.contains(&EquipmentPassive::HoldLife));
    assert!(item.tags.iter().any(|tag| tag == "blessed-weapon"));

    let affix = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.artifact-extra-res-or-power")
        .expect("XTRA_RES_OR_POWER affix should exist");
    let [group] = affix.roll_groups.as_slice() else {
        panic!("XTRA_RES_OR_POWER should roll one group");
    };
    assert_eq!(group.rolls, 1);
    assert_eq!(group.candidates.len(), 29);
    assert_eq!(
        group
            .candidates
            .iter()
            .map(|candidate| candidate.weight)
            .sum::<u32>(),
        360
    );
    assert_eq!(
        group
            .candidates
            .iter()
            .filter(|candidate| candidate.weight == 15)
            .count(),
        12
    );
    assert_eq!(
        group
            .candidates
            .iter()
            .filter(|candidate| candidate.weight == 18)
            .count(),
        8
    );
    assert_eq!(
        group
            .candidates
            .iter()
            .filter(|candidate| candidate.weight == 4)
            .count(),
        9
    );
    let rolled_passives = group
        .candidates
        .iter()
        .flat_map(|candidate| candidate.properties.passives.iter().copied())
        .collect::<BTreeSet<_>>();
    assert!(rolled_passives.contains(&EquipmentPassive::Warning));
    assert!(rolled_passives.contains(&EquipmentPassive::SlowDigestion));
    assert!(rolled_passives.contains(&EquipmentPassive::EspAnimal));
    assert!(rolled_passives.contains(&EquipmentPassive::EspGood));

    let fallback = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.graveyard-final-replacement")
        .expect("Soulsword should have a unique-artifact fallback");
    assert_eq!(fallback.entries[0].item_kind_id, "demo.item.scimitar");
    assert_eq!(
        fallback.quality_weights[0].quality,
        ItemQuality::Exceptional
    );
}

#[test]
fn mirror_shield_keeps_original_reflection_and_allocation() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.mirror-shield")
        .expect("Mirror Shield should exist");
    assert_eq!(item.generation_level, 70);
    assert_eq!(item.weight_tenths_pound, 100);
    assert_eq!(item.base_value, 10_000);
    assert_eq!(item.equipment_slot.as_deref(), Some("shield"));
    assert_eq!(item.modifiers.defense, 20);
    assert!(item.reflects_bolts);
    assert_eq!(
        item.resistances.get(&ActorDamageType::Light),
        Some(&ActorResistanceLevel::Resistant)
    );

    let allocation = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .and_then(|table| {
            table
                .entries
                .iter()
                .find(|entry| entry.item_kind_id == item.id)
        })
        .expect("Mirror Shield should retain its base-item allocation");
    assert_eq!(allocation.weight, 12);
    assert_eq!((allocation.min_depth, allocation.max_depth), (70, u16::MAX));
}

#[test]
fn p88b_quiver_protection_is_a_distinct_quiver_only_affix() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let affix = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.quiver-protection")
        .expect("Protection quiver affix should exist");
    let quiver = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.quiver")
        .expect("base quiver should exist");
    let shield = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.small-leather-shield")
        .expect("ordinary shield should exist");

    assert!(affix.protects_quiver_ammunition);
    assert!(affix.preserves_ordinary_quality);
    assert!(affix_is_compatible_with_item(affix, quiver, 20));
    assert!(!affix_is_compatible_with_item(affix, shield, 20));
}

#[test]
fn capture_ball_keeps_rfb_shape_and_low_probability_store_stock() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.capture-ball")
        .expect("capture ball should exist");
    assert!(item.capture_ball);
    assert_eq!(item.generation_level, 15);
    assert_eq!(item.weight_tenths_pound, 120);
    assert_eq!(item.base_value, 1_000);
    assert_eq!(item.max_stack, 1);
    assert_eq!(item.equipment_slot.as_deref(), Some("shield"));

    for shop_id in [
        "demo.shop.outpost-general-store",
        "demo.shop.anambar-general-store",
    ] {
        let stock = artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == shop_id)
            .and_then(|shop| {
                shop.stock
                    .iter()
                    .find(|stock| stock.item_kind_id == item.id)
            })
            .unwrap_or_else(|| panic!("{shop_id} should stock capture balls"));
        assert_eq!(stock.availability_percent, 25);
        assert_eq!((stock.initial_minimum, stock.initial_maximum), (1, 1));
    }
}

#[test]
fn riding_bond_potions_keep_rfb_thresholds_and_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = |id: &str| {
        artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("fixture should contain {id}"))
    };
    assert!(matches!(
        item("demo.item.light-healing-potion").mount_use,
        Some(ItemMountUseDefinition::Heal {
            minimum_bond: 2_500,
            dice: 4,
            sides: 8,
            ..
        })
    ));
    assert!(matches!(
        item("demo.item.vitalis-elixir").mount_use,
        Some(ItemMountUseDefinition::Heal {
            minimum_bond: 2_500,
            full: true,
            ..
        })
    ));
    assert!(matches!(
        item("demo.item.swiftstep-tonic").mount_use,
        Some(ItemMountUseDefinition::Haste {
            minimum_bond: 5_000,
            duration_dice: 1,
            duration_sides: 25,
            duration_bonus: 15,
            extension: 5,
        })
    ));
}

#[test]
fn item_shape_validation_uses_current_rfb_content() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.ration-of-food")
        .expect("ration should exist")
        .equipment_slot = Some("right-ring".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidEquipmentSlot(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.arrow")
        .expect("arrow should exist")
        .break_chance_percent = 101;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemBreakChance(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.short-bow")
        .expect("short bow should exist")
        .equipment_slot = Some("weapon".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProjectileProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let affix = invalid.affixes.first_mut().expect("an affix should exist");
    affix.modifiers = StatModifiers::default();
    affix.equipment_bonuses = EquipmentBonuses::default();
    affix.resistances.clear();
    affix.status_immunities.clear();
    affix.slays.clear();
    affix.brands.clear();
    affix.passives.clear();
    affix.roll_groups.clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAffixModifiers(_))
    ));
}

#[test]
fn rfb_ego_affix_metadata_requires_identity_unique_source_and_distinct_types() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let metadata = RfbEgoGenerationDefinition {
        source_index: 1,
        rarity: 0,
        types: vec![RfbEgoTypeDefinition::Weapon, RfbEgoTypeDefinition::Digger],
    };

    let mut valid = artifact.content.clone();
    valid.affixes[0].rfb_ego = Some(metadata.clone());
    validate_and_normalize(&mut valid).expect("rarity zero remains valid for forced ego paths");

    let mut missing_identity = artifact.content.clone();
    missing_identity.affixes[0].rfb_ego = Some(RfbEgoGenerationDefinition {
        source_index: 0,
        ..metadata.clone()
    });
    assert!(matches!(
        validate_and_normalize(&mut missing_identity),
        Err(ContentError::InvalidAffixModifiers(_))
    ));

    let mut duplicate_type = artifact.content.clone();
    duplicate_type.affixes[0].rfb_ego = Some(RfbEgoGenerationDefinition {
        types: vec![RfbEgoTypeDefinition::Weapon, RfbEgoTypeDefinition::Weapon],
        ..metadata.clone()
    });
    assert!(matches!(
        validate_and_normalize(&mut duplicate_type),
        Err(ContentError::InvalidAffixModifiers(_))
    ));

    let mut missing_type = artifact.content.clone();
    missing_type.affixes[0].rfb_ego = Some(RfbEgoGenerationDefinition {
        types: Vec::new(),
        ..metadata.clone()
    });
    assert!(matches!(
        validate_and_normalize(&mut missing_type),
        Err(ContentError::InvalidAffixModifiers(_))
    ));

    let mut duplicate_source = artifact.content.clone();
    duplicate_source.affixes[0].rfb_ego = Some(metadata.clone());
    duplicate_source.affixes[1].rfb_ego = Some(metadata);
    assert!(matches!(
        validate_and_normalize(&mut duplicate_source),
        Err(ContentError::InvalidAffixModifiers(_))
    ));
}

#[test]
fn weapon_and_digger_ego_batch_is_formal_and_complete() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let base_kinds = artifact
        .content
        .items
        .iter()
        .filter_map(|item| item.rfb_base_kind)
        .filter(|kind| matches!(kind.tval, 20..=23))
        .collect::<Vec<_>>();
    assert_eq!(base_kinds.len(), 62);
    assert_eq!(
        base_kinds
            .iter()
            .map(|kind| kind.source_index)
            .collect::<BTreeSet<_>>()
            .len(),
        base_kinds.len()
    );
    assert_eq!(
        base_kinds
            .iter()
            .map(|kind| (kind.tval, kind.sval))
            .collect::<BTreeSet<_>>()
            .len(),
        base_kinds.len()
    );

    let expected = (1_u32..=27).chain(40..=42).collect::<Vec<_>>();
    let mut actual = artifact
        .content
        .affixes
        .iter()
        .filter_map(|affix| affix.rfb_ego.as_ref().map(|ego| (ego.source_index, affix)))
        .filter(|(source_index, _)| expected.contains(source_index))
        .collect::<Vec<_>>();
    actual.sort_by_key(|(source_index, _)| *source_index);
    assert_eq!(
        actual
            .iter()
            .map(|(source_index, _)| *source_index)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(actual.iter().all(|(_, affix)| affix.roll_groups.is_empty()));

    let arcane = actual
        .iter()
        .find(|(source_index, _)| *source_index == 6)
        .map(|(_, affix)| *affix)
        .expect("Arcane should remain formally defined");
    assert_eq!(
        arcane
            .device_generation
            .as_ref()
            .expect("Arcane should carry Mage activation candidates")
            .activations
            .len(),
        32
    );
    let disruption = actual
        .iter()
        .find(|(source_index, _)| *source_index == 42)
        .map(|(_, affix)| *affix)
        .expect("Disruption should remain formally defined");
    assert_eq!(
        disruption
            .device_generation
            .as_ref()
            .expect("Disruption should retain Stone to Mud")
            .activations
            .len(),
        1
    );
}

#[test]
fn weapon_proficiency_content_rejects_invalid_bounds_and_base_aliases() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid = artifact.content.clone();
    invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.warrior")
        .and_then(|class| class.weapon_proficiency.as_mut())
        .expect("Warrior weapon proficiency")
        .default_initial = 8_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidWeaponProficiency(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.crisdurian")
        .expect("Crisdurian should exist")
        .weapon_proficiency_base_item_id = Some("demo.item.short-bow".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidWeaponProficiency(_))
    ));
}

#[test]
fn riding_proficiency_content_rejects_invalid_bounds() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content;
    invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.warrior")
        .expect("Warrior class")
        .riding_proficiency
        .initial = 6_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidRidingProficiency(_))
    ));

    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content;
    invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.warrior")
        .expect("Warrior class")
        .mounted_non_arrow_base_shot_cap = Some(0);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidRidingProficiency(_))
    ));
}

#[test]
fn riding_weapons_match_the_existing_rfb_master_subset() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let actual = artifact
        .content
        .items
        .iter()
        .filter_map(|item| item.riding_weapon_kind.map(|kind| (item.id.as_str(), kind)))
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            "demo.item.ball-and-chain",
            "demo.item.broad-spear",
            "demo.item.broad-sword",
            "demo.item.falchion",
            "demo.item.fauchard",
            "demo.item.flail",
            "demo.item.glaive",
            "demo.item.heavy-lance",
            "demo.item.lance",
            "demo.item.long-sword",
            "demo.item.pain",
            "demo.item.sabre",
            "demo.item.spear",
            "demo.item.trident",
            "demo.item.trifurcate-spear",
            "demo.item.trifurcate-spear-of-wrath",
            "demo.item.tulwar",
            "demo.item.war-hammer",
        ]
        .into_iter()
        .map(|id| {
            (
                id,
                if matches!(id, "demo.item.heavy-lance" | "demo.item.lance") {
                    RidingWeaponKindDefinition::Lance
                } else {
                    RidingWeaponKindDefinition::Compatible
                },
            )
        })
        .collect::<Vec<_>>()
    );

    let mut invalid = artifact.content;
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.lance")
        .expect("Lance should exist")
        .melee_profile = None;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAttackProfile(_))
    ));
}

#[test]
fn fixed_artifact_generation_matches_rfb_records_and_rejects_invalid_content() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for (item_id, source_index, base_item_kind_id, rarity_one_in) in [
        (
            "demo.item.crisdurian",
            80,
            "demo.item.executioners-sword",
            15,
        ),
        ("demo.item.pain", 94, "demo.item.glaive", 25),
        ("demo.item.slayer", 123, "demo.item.executioners-sword", 60),
    ] {
        let generation = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.artifact_generation.as_ref())
            .unwrap_or_else(|| panic!("{item_id} should have fixed-artifact generation data"));
        assert_eq!(generation.source_index, source_index);
        assert_eq!(generation.base_item_kind_id, base_item_kind_id);
        assert_eq!(generation.rarity_one_in, rarity_one_in);
        assert!(!generation.instant);
    }
    assert!(
        artifact
            .content
            .items
            .iter()
            .find(|item| item.id == "demo.item.relic-blade")
            .expect("demo relic blade should exist")
            .artifact_generation
            .is_none()
    );

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.crisdurian")
        .and_then(|item| item.artifact_generation.as_mut())
        .expect("Crisdurian generation data")
        .rarity_one_in = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidArtifactGeneration(_))
    ));

    let mut invalid = artifact.content.clone();
    let pain_index = invalid
        .items
        .iter()
        .find(|item| item.id == "demo.item.pain")
        .and_then(|item| item.artifact_generation.as_ref())
        .expect("Pain generation data")
        .source_index;
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.crisdurian")
        .and_then(|item| item.artifact_generation.as_mut())
        .expect("Crisdurian generation data")
        .source_index = pain_index;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidArtifactGeneration(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.crisdurian")
        .and_then(|item| item.artifact_generation.as_mut())
        .expect("Crisdurian generation data")
        .base_item_kind_id = "demo.item.chain-mail".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidArtifactGeneration(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base-item loot table")
        .entries[0]
        .item_kind_id = "demo.item.crisdurian".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidLootTable(_))
    ));
}

#[test]
fn natural_affix_compatibility_uses_slot_depth_and_explicit_none_fallback() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let slaying = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.slaying")
        .expect("Slaying should exist");
    let protection = artifact
        .content
        .affixes
        .iter()
        .find(|affix| affix.id == "rfb-legacy.affix.protection")
        .expect("Protection should exist");
    let halberd = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.halberd")
        .expect("halberd should exist");
    let crossbow = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.light-crossbow")
        .expect("crossbow should exist");
    assert!(affix_is_compatible_with_item(slaying, halberd, 32));
    assert!(!affix_is_compatible_with_item(slaying, crossbow, 32));
    assert!(!affix_is_compatible_with_item(protection, halberd, 30));
    for item_id in [
        "demo.item.chain-mail",
        "demo.item.small-metal-shield",
        "demo.item.cloak",
        "demo.item.iron-helm",
        "demo.item.set-of-gauntlets",
        "demo.item.soft-leather-boots",
    ] {
        let item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == item_id)
            .unwrap_or_else(|| panic!("{item_id} should exist"));
        assert!(
            affix_is_compatible_with_item(protection, item, 30),
            "{item_id}"
        );
        assert!(
            !affix_is_compatible_with_item(protection, item, 31),
            "{item_id}"
        );
    }
    assert_eq!(protection.generation_level, 0);
    assert_eq!(protection.generation_max_level, 30);
    assert_eq!(protection.roll_groups.len(), 1);
    assert_eq!(protection.roll_groups[0].rolls, 1);
    assert_eq!(
        protection.roll_groups[0]
            .candidates
            .iter()
            .map(|candidate| (candidate.weight, candidate.properties.modifiers.defense))
            .collect::<Vec<_>>(),
        (1..=10).map(|defense| (1, defense)).collect::<Vec<_>>()
    );

    let mut bounded = slaying.clone();
    bounded.generation_level = 20;
    bounded.generation_max_level = 30;
    assert!(!affix_is_compatible_with_item(&bounded, halberd, 19));
    assert!(affix_is_compatible_with_item(&bounded, halberd, 20));
    assert!(!affix_is_compatible_with_item(&bounded, halberd, 31));

    let mut missing_fallback = artifact.content.clone();
    missing_fallback
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("Orc Cave loot should exist")
        .affix_weights
        .retain(|entry| entry.affix_id.is_some());
    assert!(matches!(
        validate_and_normalize(&mut missing_fallback),
        Err(ContentError::InvalidLootTable(id)) if id == "demo.loot-table.base-items"
    ));

    let mut outside_depth = artifact.content.clone();
    let table = outside_depth
        .loot_tables
        .iter_mut()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");
    table.entries.retain(|entry| entry.min_depth <= 10);
    table
        .entries
        .iter_mut()
        .for_each(|entry| entry.max_depth = 10);
    outside_depth
        .affixes
        .iter_mut()
        .find(|affix| affix.id == "rfb-legacy.affix.slaying")
        .expect("Slaying should exist")
        .generation_level = 20;
    assert!(matches!(
        validate_and_normalize(&mut outside_depth),
        Err(ContentError::InvalidLootTable(_))
    ));
}

#[test]
fn salt_water_keeps_authoritative_shape_effect_and_shallow_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.salt-water")
        .expect("Salt Water should exist");
    assert_eq!(item.generation_level, 0);
    assert_eq!(item.weight_tenths_pound, 4);
    assert_eq!(item.base_value, 1);
    assert!(matches!(
        item.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::ApplySaltWater)
    ));
    let base_items = artifact
        .content
        .loot_tables
        .iter()
        .find(|table| table.id == "demo.loot-table.base-items")
        .expect("base item pool should exist");
    let entry = base_items
        .entries
        .iter()
        .find(|entry| entry.item_kind_id == item.id)
        .expect("Salt Water should be shallow loot");
    assert_eq!((entry.min_depth, entry.max_depth), (0, 20));
}

#[test]
fn food_effect_requires_positive_bounded_nutrition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let ration = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.ration-of-food")
        .expect("fixture should contain the ration");
    assert!(matches!(
        ration.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::IncreaseNutrition { amount: 5_000 })
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.ration-of-food")
        .expect("fixture should contain the ration")
        .use_action
        .as_mut()
        .expect("ration should be usable")
        .effect = ItemUseEffectDefinition::IncreaseNutrition { amount: 0 };
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn elvish_waybread_keeps_original_shape_effect_and_town_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let waybread = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.piece-of-elvish-waybread")
        .expect("Elvish Waybread should exist");

    assert_eq!(waybread.glyph, ",");
    assert_eq!(waybread.generation_level, 5);
    assert_eq!(waybread.weight_tenths_pound, 3);
    assert_eq!(waybread.base_value, 30);
    assert!(matches!(
        waybread.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::ApplyElvishWaybread {
            nutrition: 7_500,
            healing_dice: 4,
            healing_sides: 8,
        })
    ));

    for shop_id in [
        "demo.shop.outpost-general-store",
        "demo.shop.anambar-general-store",
    ] {
        assert!(
            artifact
                .content
                .shops
                .iter()
                .find(|shop| shop.id == shop_id)
                .unwrap_or_else(|| panic!("{shop_id} should exist"))
                .stock
                .iter()
                .any(|stock| stock.item_kind_id == waybread.id),
            "{shop_id} should stock Elvish Waybread"
        );
    }
}

#[test]
fn fine_drinks_keep_original_shape_effect_and_town_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for (item_id, weight, value, nutrition) in [
        ("demo.item.pint-of-fine-ale", 5, 1, 500),
        ("demo.item.pint-of-fine-wine", 10, 2, 1_000),
    ] {
        let item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == item_id)
            .unwrap_or_else(|| panic!("{item_id} should exist"));
        assert_eq!(item.glyph, ",");
        assert_eq!(item.generation_level, 0);
        assert_eq!(item.weight_tenths_pound, weight);
        assert_eq!(item.base_value, value);
        assert!(matches!(
            item.use_action.as_ref().map(|action| &action.effect),
            Some(ItemUseEffectDefinition::IncreaseNutrition { amount }) if *amount == nutrition
        ));
        for shop_id in [
            "demo.shop.outpost-general-store",
            "demo.shop.anambar-general-store",
            "demo.shop.anambar-inn",
        ] {
            assert!(
                artifact
                    .content
                    .shops
                    .iter()
                    .find(|shop| shop.id == shop_id)
                    .unwrap_or_else(|| panic!("{shop_id} should exist"))
                    .stock
                    .iter()
                    .any(|stock| stock.item_kind_id == item_id),
                "{shop_id} should stock {item_id}"
            );
        }
    }
}

#[test]
fn fast_recovery_mushroom_keeps_original_shape_effect_and_shroomery_acquisition() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mushroom = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.fast-recovery-mushroom")
        .expect("Fast Recovery Mushroom should exist");

    assert_eq!(mushroom.glyph, ",");
    assert_eq!(mushroom.generation_level, 15);
    assert_eq!(mushroom.weight_tenths_pound, 2);
    assert_eq!(mushroom.base_value, 30);
    assert!(matches!(
        mushroom.use_action.as_ref().map(|action| &action.effect),
        Some(ItemUseEffectDefinition::Sequence { effects })
            if effects == &[
                ItemUseEffectDefinition::ApplyFastRecovery,
                ItemUseEffectDefinition::IncreaseNutrition { amount: 500 },
            ]
    ));
    let shroomery = artifact
        .content
        .shops
        .iter()
        .find(|shop| shop.id == "demo.shop.outpost-shroomery")
        .expect("Outpost Shroomery should exist");
    assert!(
        shroomery
            .stock
            .iter()
            .any(|stock| stock.item_kind_id == mushroom.id)
    );
    assert!(
        artifact
            .content
            .shops
            .iter()
            .find(|shop| shop.id == "demo.shop.outpost-general-store")
            .expect("General Store should exist")
            .stock
            .iter()
            .all(|stock| !stock.item_kind_id.ends_with("-mushroom"))
    );
}

#[test]
fn restorative_item_sequences_require_bounded_effects_and_known_resources() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let clarity = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_ref())
        .expect("fixture should contain the clarity action");
    let ItemUseEffectDefinition::Sequence { effects } = &clarity.effect else {
        panic!("clarity should use an ordered effect sequence");
    };
    assert!(matches!(
        &effects[0],
        ItemUseEffectDefinition::RestoreResourceDice {
            resource_id,
            dice: 3,
            sides: 6,
            bonus: 3,
        } if resource_id == "demo.resource.mana"
    ));
    assert!(matches!(
        &effects[1],
        ItemUseEffectDefinition::RemoveStatus { status_kind_id }
            if status_kind_id == "rfb.status.confusion"
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_mut())
        .expect("clarity action should exist")
        .effect = ItemUseEffectDefinition::RestoreResourceFull {
        resource_id: "demo.resource.missing".to_owned(),
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let action = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.clarity-draught")
        .and_then(|item| item.use_action.as_mut())
        .expect("clarity action should exist");
    action.effect = ItemUseEffectDefinition::Sequence {
        effects: vec![
            ItemUseEffectDefinition::Heal { amount: 1 },
            ItemUseEffectDefinition::Sequence {
                effects: vec![
                    ItemUseEffectDefinition::Heal { amount: 1 },
                    ItemUseEffectDefinition::Heal { amount: 1 },
                ],
            },
        ],
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn dynamic_devices_require_stable_profiles_depth_coverage_and_capacity() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let wand = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.magic-missile-wand")
        .and_then(|item| item.device_generation.as_ref())
        .expect("fixture should contain dynamic wand profiles");
    assert_eq!(
        wand.activations
            .iter()
            .map(|activation| activation.id.as_str())
            .collect::<Vec<_>>(),
        vec!["demo.device-activation.magic-missile"]
    );
    assert_eq!(
        wand.recovery,
        Some(ItemDeviceRecoveryDefinition {
            interval_ticks: 10,
            energy_per_mille: 10,
        })
    );

    let mut invalid = artifact.content.clone();
    let profiles = &mut invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.magic-missile-wand")
        .expect("fixture should contain the dynamic wand")
        .device_generation
        .as_mut()
        .expect("dynamic generation should exist")
        .activations;
    profiles[0].min_depth = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.magic-missile-wand")
        .and_then(|item| item.device_generation.as_mut())
        .and_then(|generation| generation.recovery.as_mut())
        .expect("dynamic wand should recover")
        .energy_per_mille = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let wand = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.magic-missile-wand")
        .expect("fixture should contain the dynamic wand");
    wand.device_generation
        .as_mut()
        .expect("dynamic generation should exist")
        .activations[0]
        .charges
        .cost = 1_000_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let wand = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.magic-missile-wand")
        .expect("fixture should contain the dynamic wand");
    wand.use_action = Some(ItemUseActionDefinition {
        device_check_difficulty: None,
        charges: None,
        effect: ItemUseEffectDefinition::Heal { amount: 1 },
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
}

#[test]
fn fuel_items_require_original_capacity_slot_stack_and_radius_shapes() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let item = |id: &str| {
        artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("fixture should contain {id}"))
    };
    assert_eq!(
        item("demo.item.wooden-torch").fuel,
        Some(ItemFuelDefinition {
            kind: ItemFuelKindDefinition::Torch,
            initial: 4_000,
            maximum: 5_000,
            light_radius: 1,
        })
    );
    assert_eq!(
        item("demo.item.brass-lantern").fuel,
        Some(ItemFuelDefinition {
            kind: ItemFuelKindDefinition::Lantern,
            initial: 7_500,
            maximum: 15_000,
            light_radius: 2,
        })
    );
    assert_eq!(
        item("demo.item.flask-of-oil").fuel,
        Some(ItemFuelDefinition {
            kind: ItemFuelKindDefinition::Oil,
            initial: 7_500,
            maximum: 7_500,
            light_radius: 0,
        })
    );

    let mut invalid_capacity = artifact.content.clone();
    invalid_capacity
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.wooden-torch")
        .and_then(|item| item.fuel.as_mut())
        .expect("fixture should contain a torch fuel profile")
        .maximum = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_capacity),
        Err(ContentError::InvalidItemFuel(_))
    ));

    let mut invalid_radius = artifact.content.clone();
    invalid_radius
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.brass-lantern")
        .and_then(|item| item.fuel.as_mut())
        .expect("fixture should contain a lantern fuel profile")
        .light_radius = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_radius),
        Err(ContentError::InvalidItemFuel(_))
    ));

    let mut invalid_oil = artifact.content.clone();
    invalid_oil
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.flask-of-oil")
        .and_then(|item| item.fuel.as_mut())
        .expect("fixture should contain an oil fuel profile")
        .initial = 7_000;
    assert!(matches!(
        validate_and_normalize(&mut invalid_oil),
        Err(ContentError::InvalidItemFuel(_))
    ));
}

#[test]
fn selected_legacy_equipment_keeps_fixed_source_values_and_slots() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let expected = [
        ("demo.item.broken-dagger", 1, 5, "weapon", 0, Some((1, 3))),
        ("demo.item.broken-sword", 2, 30, "weapon", 0, Some((2, 3))),
        ("demo.item.club", 3, 100, "weapon", 0, Some((1, 5))),
        ("demo.item.dagger", 10, 12, "weapon", 0, Some((1, 5))),
        ("demo.item.main-gauche", 25, 30, "weapon", 0, Some((1, 6))),
        ("demo.item.tanto", 30, 20, "weapon", 0, Some((1, 7))),
        ("demo.item.whip", 30, 30, "weapon", 0, Some((1, 7))),
        ("demo.item.rapier", 42, 40, "weapon", 0, Some((1, 8))),
        ("demo.item.small-sword", 48, 75, "weapon", 0, Some((1, 8))),
        ("demo.item.short-sword", 90, 80, "weapon", 0, Some((1, 8))),
        ("demo.item.cutlass", 85, 110, "weapon", 0, Some((1, 9))),
        ("demo.item.mace", 130, 120, "weapon", 0, Some((2, 5))),
        ("demo.item.khopesh", 190, 130, "weapon", 0, Some((2, 5))),
        ("demo.item.scimitar", 250, 130, "weapon", 0, Some((2, 6))),
        ("demo.item.hatchet", 120, 60, "weapon", 0, Some((1, 6))),
        ("demo.item.sickle", 110, 70, "weapon", 0, Some((2, 4))),
        ("demo.item.awl-pike", 340, 160, "weapon", 0, Some((1, 10))),
        (
            "demo.item.lucerne-hammer",
            376,
            120,
            "weapon",
            0,
            Some((2, 6)),
        ),
        (
            "demo.item.quarterstaff",
            200,
            150,
            "weapon",
            0,
            Some((1, 10)),
        ),
        (
            "demo.item.morning-star",
            396,
            150,
            "weapon",
            0,
            Some((2, 7)),
        ),
        ("demo.item.shovel", 10, 60, "tool", 0, Some((1, 3))),
        ("demo.item.pick", 50, 150, "tool", 0, Some((1, 5))),
        ("demo.item.cloak", 3, 10, "cloak", 1, None),
        ("demo.item.robe", 4, 20, "body", 2, None),
        ("demo.item.padded-armour", 50, 60, "body", 5, None),
        ("demo.item.knit-cap", 10, 8, "head", 1, None),
        ("demo.item.pointy-hat", 20, 20, "head", 1, None),
        ("demo.item.filthy-rag", 1, 20, "body", 0, None),
        ("demo.item.paper-armour", 50, 30, "body", 4, None),
        ("demo.item.soft-leather-armour", 18, 80, "body", 4, None),
        ("demo.item.soft-studded-leather", 35, 90, "body", 5, None),
        (
            "demo.item.pair-of-hard-leather-boots",
            12,
            40,
            "boots",
            3,
            None,
        ),
        ("demo.item.cord-armour", 200, 80, "body", 6, None),
        ("demo.item.hard-leather-armour", 150, 100, "body", 6, None),
        ("demo.item.hard-studded-leather", 200, 110, "body", 7, None),
        ("demo.item.metal-cap", 30, 20, "head", 3, None),
        ("demo.item.small-metal-shield", 40, 65, "shield", 5, None),
        (
            "demo.item.large-leather-shield",
            100,
            100,
            "shield",
            6,
            None,
        ),
        (
            "demo.item.set-of-studded-leather-gloves",
            3,
            5,
            "gloves",
            1,
            None,
        ),
        ("demo.item.set-of-gauntlets", 35, 20, "gloves", 4, None),
        ("demo.item.leather-gloves", 3, 5, "gloves", 1, None),
        ("demo.item.soft-leather-boots", 7, 20, "boots", 2, None),
        ("demo.item.hard-leather-cap", 12, 15, "head", 2, None),
        ("demo.item.small-leather-shield", 15, 50, "shield", 3, None),
        ("demo.item.chain-mail", 750, 220, "body", 14, None),
        ("demo.item.trident", 120, 70, "weapon", 0, Some((1, 10))),
        ("demo.item.fauchard", 301, 155, "weapon", 0, Some((1, 12))),
        (
            "demo.item.broad-spear",
            240,
            100,
            "weapon",
            0,
            Some((1, 11)),
        ),
        ("demo.item.pike", 358, 160, "weapon", 0, Some((2, 6))),
        ("demo.item.beaked-axe", 408, 120, "weapon", 0, Some((2, 7))),
        ("demo.item.broad-axe", 304, 130, "weapon", 0, Some((2, 7))),
        ("demo.item.glaive", 363, 190, "weapon", 0, Some((2, 7))),
        (
            "demo.item.heavy-lance",
            700,
            400,
            "weapon",
            0,
            Some((4, 10)),
        ),
        ("demo.item.lance", 230, 300, "weapon", 0, Some((2, 10))),
        ("demo.item.battle-axe", 334, 170, "weapon", 0, Some((2, 9))),
        ("demo.item.nunchaku", 120, 60, "weapon", 0, Some((2, 4))),
        (
            "demo.item.ball-and-chain",
            200,
            150,
            "weapon",
            0,
            Some((2, 5)),
        ),
        ("demo.item.jo-staff", 200, 70, "weapon", 0, Some((1, 8))),
        ("demo.item.war-hammer", 225, 120, "weapon", 0, Some((3, 4))),
        (
            "demo.item.three-piece-rod",
            350,
            120,
            "weapon",
            0,
            Some((4, 3)),
        ),
        ("demo.item.flail", 353, 150, "weapon", 0, Some((2, 7))),
        ("demo.item.bo-staff", 310, 160, "weapon", 0, Some((1, 12))),
        (
            "demo.item.lead-filled-mace",
            502,
            180,
            "weapon",
            0,
            Some((3, 5)),
        ),
        ("demo.item.gnomish-shovel", 100, 60, "tool", 0, Some((1, 3))),
        ("demo.item.rhino-hide-armour", 400, 110, "body", 8, None),
        ("demo.item.leather-jacket", 700, 130, "body", 12, None),
        ("demo.item.ring-mail", 500, 200, "body", 12, None),
    ];

    for (id, value, weight, slot, defense, damage) in expected {
        let item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("fixture should contain {id}"));
        assert_eq!(item.base_value, value, "{id} value");
        assert_eq!(item.weight_tenths_pound, weight, "{id} weight");
        assert_eq!(item.equipment_slot.as_deref(), Some(slot), "{id} slot");
        assert_eq!(item.modifiers.defense, defense, "{id} defense");
        assert_eq!(
            item.melee_profile
                .as_ref()
                .map(|profile| (profile.damage_dice, profile.damage_sides)),
            damage,
            "{id} damage"
        );
    }
}

#[test]
fn selected_legacy_equipment_keeps_combat_and_tunneling_modifiers() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let bonuses = |id: &str| {
        &artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
            .equipment_bonuses
    };

    assert_eq!(bonuses("demo.item.hard-leather-armour").melee_skill, -1);
    assert_eq!(bonuses("demo.item.hard-studded-leather").melee_skill, -1);
    assert_eq!(
        bonuses("demo.item.set-of-studded-leather-gloves").melee_damage,
        1
    );
    let gauntlets = bonuses("demo.item.set-of-gauntlets");
    assert_eq!(gauntlets.melee_skill, 1);
    assert_eq!(gauntlets.melee_damage, 1);
    assert_eq!(bonuses("demo.item.chain-mail").melee_skill, -2);
    let tunneling_pval = |id: &str| {
        artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"))
            .tunneling_pval
    };
    assert_eq!(tunneling_pval("demo.item.shovel"), 2);
    assert_eq!(tunneling_pval("demo.item.pick"), 2);
    assert_eq!(tunneling_pval("demo.item.gnomish-shovel"), 3);
    assert_eq!(tunneling_pval("demo.item.orcish-pick"), 3);
    assert_eq!(bonuses("demo.item.rhino-hide-armour").melee_skill, -1);
    assert_eq!(bonuses("demo.item.leather-jacket").melee_skill, -1);
    assert_eq!(bonuses("demo.item.ring-mail").melee_skill, -2);
}

#[test]
fn arrows_stack_up_to_ninety_nine() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let arrow = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.arrow")
        .expect("original pack should contain arrows");

    assert_eq!(arrow.max_stack, 99);
}

#[test]
fn supported_legacy_scrolls_and_potions_keep_source_identity_and_values() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let scrolls = artifact
        .content
        .items
        .iter()
        .filter(|item| item.tags.iter().any(|tag| tag == "scroll"))
        .collect::<Vec<_>>();
    let potions = artifact
        .content
        .items
        .iter()
        .filter(|item| item.tags.iter().any(|tag| tag == "potion"))
        .collect::<Vec<_>>();

    assert_eq!(scrolls.len(), 59);
    assert_eq!(potions.len(), 66);
    assert!(scrolls.iter().all(|item| item.weight_tenths_pound == 5));
    assert!(potions.iter().all(|item| item.weight_tenths_pound == 4));

    let appearance_keys = scrolls
        .iter()
        .chain(potions.iter())
        .map(|item| {
            item.appearance_name_key
                .as_deref()
                .expect("supported consumables should have source flavor")
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(appearance_keys.len(), 125);

    let added_values = [
        ("demo.item.door-stair-location-scroll", 10),
        ("demo.item.detect-invisible-scroll", 5),
        ("demo.item.holy-chant-scroll", 40),
        ("demo.item.holy-prayer-scroll", 80),
        ("demo.item.detect-monsters-scroll", 10),
        ("demo.item.satisfy-hunger-scroll", 10),
        ("demo.item.sleep-potion", 1),
        ("demo.item.stupidity-potion", 0),
        ("demo.item.naivety-potion", 0),
        ("demo.item.clumsiness-potion", 0),
        ("demo.item.sickliness-potion", 0),
        ("demo.item.ugliness-potion", 0),
        ("demo.item.boldness-potion", 200),
        ("demo.item.booze-potion", 1),
        ("demo.item.vigor-potion", 200),
        ("demo.item.cure-serious-wounds-potion", 50),
        ("demo.item.cure-critical-wounds-potion", 150),
        ("demo.item.healing-potion", 500),
        ("demo.item.star-healing-potion", 1_500),
        ("demo.item.restore-intelligence-potion", 220),
        ("demo.item.restore-wisdom-potion", 220),
        ("demo.item.restore-dexterity-potion", 220),
        ("demo.item.restore-constitution-potion", 220),
        ("demo.item.restore-charisma-potion", 220),
        ("demo.item.intelligence-potion", 25_000),
        ("demo.item.wisdom-potion", 25_000),
        ("demo.item.dexterity-potion", 25_000),
        ("demo.item.constitution-potion", 25_000),
        ("demo.item.charisma-potion", 25_000),
        ("demo.item.blood-potion", 1_234),
        ("demo.item.water-potion", 1),
        ("demo.item.apple-juice", 1),
        ("demo.item.slime-mold-juice", 2),
        ("demo.item.lose-memories-potion", 0),
        ("demo.item.ruination-potion", 0),
        ("demo.item.sight-potion", 50),
        ("demo.item.antidote-potion", 100),
        ("demo.item.curing-potion", 80),
        ("demo.item.invulnerability-potion", 100_000),
        ("demo.item.giant-strength-potion", 10_000),
        ("demo.item.great-clarity-potion", 1_000),
        ("demo.item.treasure-detection-scroll", 8),
        ("demo.item.understanding-scroll", 2_500),
        ("demo.item.inventory-protection-scroll", 2_500),
        ("demo.item.enlightenment-potion", 800),
        ("demo.item.star-enlightenment-potion", 120_000),
        ("demo.item.self-knowledge-potion", 2_000),
        ("demo.item.darkness-scroll", 0),
        ("demo.item.trap-creation-scroll", 0),
        ("demo.item.light-scroll", 15),
        ("demo.item.rune-of-protection-scroll", 500),
        ("demo.item.destruction-scroll", 250),
        ("demo.item.mundanity-scroll", 3_000),
        ("demo.item.acquirement-scroll", 100_000),
        ("demo.item.star-acquirement-scroll", 200_000),
        ("demo.item.rumour-scroll", 10),
        ("demo.item.crafting-scroll", 100_000),
        ("demo.item.experience-potion", 25_000),
        ("demo.item.neo-tsuyoshi-special", 2_000),
        ("demo.item.tsuyoshi-special", 0),
        ("demo.item.new-life-potion", 25_000),
        ("demo.item.polymorph-potion", 5_000),
    ];
    for (id, base_value) in added_values {
        let item = artifact
            .content
            .items
            .iter()
            .find(|item| item.id == id)
            .unwrap_or_else(|| panic!("{id} should exist"));
        assert_eq!(item.base_value, base_value, "{id} source value");
        assert!(item.use_action.is_some(), "{id} should be usable");
    }
}

#[test]
fn rfb_base_kind_identity_rejects_duplicate_source_indices_and_kind_values() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let set_identities = |content: &mut CompiledContentV1, second: RfbBaseKindDefinition| {
        let short_sword = content
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.short-sword")
            .expect("Short Sword should exist");
        short_sword.rfb_base_kind = Some(RfbBaseKindDefinition {
            source_index: 100,
            tval: 23,
            sval: 10,
        });
        let broad_sword = content
            .items
            .iter_mut()
            .find(|item| item.id == "demo.item.broad-sword")
            .expect("Broad Sword should exist");
        broad_sword.rfb_base_kind = Some(second);
    };

    let mut duplicate_index = artifact.content.clone();
    set_identities(
        &mut duplicate_index,
        RfbBaseKindDefinition {
            source_index: 100,
            tval: 23,
            sval: 16,
        },
    );
    assert!(matches!(
        validate_and_normalize(&mut duplicate_index),
        Err(ContentError::InvalidItemSourceIdentity(_))
    ));

    let mut duplicate_kind = artifact.content;
    set_identities(
        &mut duplicate_kind,
        RfbBaseKindDefinition {
            source_index: 101,
            tval: 23,
            sval: 10,
        },
    );
    assert!(matches!(
        validate_and_normalize(&mut duplicate_kind),
        Err(ContentError::InvalidItemSourceIdentity(_))
    ));
}
