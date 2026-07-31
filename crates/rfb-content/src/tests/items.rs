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
    invalid
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.resonance-sling")
        .expect("fixture should contain the sling")
        .projectile_profile
        .as_mut()
        .expect("sling should have a projectile profile")
        .ammo_kind_id = "demo.item.missing-ammunition".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
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
