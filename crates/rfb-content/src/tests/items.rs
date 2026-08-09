use super::*;

#[test]
fn equippable_items_require_a_valid_slot_and_single_item_stack() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.equipment_slot = Some("charm".to_owned());

    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidEquipmentSlot(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.modifiers.max_hp = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemModifiers(_))
    ));

    let mut invalid = artifact.content.clone();
    let pellet = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-pellet")
        .expect("fixture should contain the ammunition");
    pellet.break_chance_percent = 101;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemBreakChance(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-pellet")
        .expect("fixture should contain the ammunition")
        .ammunition_profile = None;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProjectileProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard.weight_tenths_pound = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemWeight(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard.appearance_name_key = Some(shard.name_key.clone());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAppearance(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the usable shard");
    shard.use_action = Some(ItemUseActionDefinition {
        device_check_difficulty: None,
        charges: None,
        effect: ItemUseEffectDefinition::Heal { amount: 0 },
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid.affixes[0].modifiers = StatModifiers::default();
    invalid.affixes[0].equipment_bonuses = EquipmentBonuses::default();
    invalid.affixes[0].resistances.clear();
    invalid.affixes[0].status_immunities.clear();
    invalid.affixes[0].slays.clear();
    invalid.affixes[0].brands.clear();
    invalid.affixes[0].passives.clear();
    invalid.affixes[0].roll_groups.clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAffixModifiers(_))
    ));

    let mut invalid = artifact.content.clone();
    let charm = invalid.worlds[0]
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.echo-charm")
        .expect("fixture should contain the charm");
    charm.affix_ids.push("demo.affix.harmonic-edge".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAffixes(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid.worlds[0]
        .items
        .iter_mut()
        .find(|item| item.kind_id == "demo.item.luminous-shard")
        .expect("fixture should contain the shard");
    shard.quality = ItemQuality::Fine;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemAffixes(_))
    ));

    let mut invalid = artifact.content.clone();
    let shard = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.luminous-shard")
        .expect("fixture should contain the throwable shard");
    shard
        .throw_profile
        .as_mut()
        .expect("shard should have a throw profile")
        .damage_dice = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidThrowProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let blade = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.echo-blade")
        .expect("fixture should contain the blade");
    blade.equipment_slot = Some("charm".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidAttackProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let sling = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-sling")
        .expect("fixture should contain the sling");
    sling.equipment_slot = Some("weapon".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProjectileProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    for item in &mut invalid.items {
        if let Some(ammo) = &mut item.ammunition_profile
            && ammo.ammunition_type == AmmunitionTypeDefinition::Shot
        {
            ammo.ammunition_type = AmmunitionTypeDefinition::Arrow;
        }
    }
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidProjectileProfile(_))
    ));
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
fn charged_item_actions_require_bounded_single_instance_devices() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let action = artifact
        .content
        .items
        .iter()
        .find(|item| item.id == "demo.item.resonance-mender")
        .and_then(|item| item.use_action.as_ref())
        .expect("fixture should contain the charged device action");
    assert_eq!(
        action.charges,
        Some(ItemChargeDefinition {
            initial: 3,
            maximum: 3,
            cost: 1,
        })
    );
    assert!(matches!(
        action.effect,
        ItemUseEffectDefinition::HealDice { dice: 2, sides: 4 }
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender
        .use_action
        .as_mut()
        .and_then(|action| action.charges.as_mut())
        .expect("charged action should exist")
        .maximum = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender.max_stack = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    let mender = invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-mender")
        .expect("fixture should contain the charged device");
    mender
        .use_action
        .as_mut()
        .expect("charged action should exist")
        .device_check_difficulty = None;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));
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
        .find(|item| item.id == "demo.item.resonance-wand")
        .and_then(|item| item.device_generation.as_ref())
        .expect("fixture should contain dynamic wand profiles");
    assert_eq!(
        wand.activations
            .iter()
            .map(|activation| activation.id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "demo.device-activation.frost-bolt",
            "demo.device-activation.spark-bolt",
        ]
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
        .find(|item| item.id == "demo.item.resonance-wand")
        .expect("fixture should contain the dynamic wand")
        .device_generation
        .as_mut()
        .expect("dynamic generation should exist")
        .activations;
    profiles
        .iter_mut()
        .find(|profile| profile.id == "demo.device-activation.spark-bolt")
        .expect("shallow profile should exist")
        .min_depth = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidItemUseAction(_))
    ));

    let mut invalid = artifact.content.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-wand")
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
        .find(|item| item.id == "demo.item.resonance-wand")
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
        .find(|item| item.id == "demo.item.resonance-wand")
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
fn device_recharge_profiles_require_distinct_bounded_resources() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.artificer")
        .and_then(|class| class.device_recharge_profile.as_mut())
        .expect("artificer should recharge devices")
        .source_item_destruction_one_in = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidDeviceRechargeProfile(_))
    ));

    let mut invalid = artifact.content.clone();
    let mage = invalid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .expect("mage class should exist");
    mage.device_recharge_profile = Some(DeviceRechargeProfileDefinition {
        resource_id: "demo.resource.mana".to_owned(),
        governing_attribute: TechniqueAttribute::Intelligence,
        base_capacity: 1,
        capacity_per_level: 0,
        capacity_per_attribute_index: 0,
        power: 90,
        source_item_destruction_one_in: 3,
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::InvalidDeviceRechargeProfile(_))
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
fn selected_legacy_armor_and_gloves_keep_melee_combat_modifiers() {
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
    assert_eq!(bonuses("demo.item.shovel").digging_skill, 2);
    assert_eq!(bonuses("demo.item.pick").digging_skill, 2);
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
    assert_eq!(potions.len(), 63);
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
    assert_eq!(appearance_keys.len(), 122);

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
