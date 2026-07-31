use super::*;

#[test]
fn ability_books_require_consistent_resources_items_and_casting_profiles() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_recovery = artifact.content.clone();
    invalid_recovery.resources[0].rest_recovery_amount = 1_000_001;
    assert!(matches!(
        validate_and_normalize(&mut invalid_recovery),
        Err(ContentError::InvalidResource(_))
    ));

    let mut invalid_healing_target = artifact.content.clone();
    invalid_healing_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.mending-echo")
        .expect("fixture should contain the healing ability")
        .target
        .range = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_healing_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_area_radius = artifact.content.clone();
    let AbilityEffectDefinition::AreaDamage { radius, .. } = &mut invalid_area_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-burst")
        .expect("fixture should contain the area damage ability")
        .effect
    else {
        panic!("echo burst should use area damage");
    };
    *radius = 17;
    assert!(matches!(
        validate_and_normalize(&mut invalid_area_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_beam_target = artifact.content.clone();
    invalid_beam_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-lance")
        .expect("fixture should contain the beam damage ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::SelfTarget];
    assert!(matches!(
        validate_and_normalize(&mut invalid_beam_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cone_radius = artifact.content.clone();
    let AbilityEffectDefinition::ConeDamage { radius, .. } = &mut invalid_cone_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-fan")
        .expect("fixture should contain the cone damage ability")
        .effect
    else {
        panic!("echo fan should use cone damage");
    };
    *radius = 17;
    assert!(matches!(
        validate_and_normalize(&mut invalid_cone_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cone_target = artifact.content.clone();
    invalid_cone_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-fan")
        .expect("fixture should contain the cone damage ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Position];
    assert!(matches!(
        validate_and_normalize(&mut invalid_cone_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_teleport_target = artifact.content.clone();
    invalid_teleport_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-step")
        .expect("fixture should contain the teleport ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Entity];
    assert!(matches!(
        validate_and_normalize(&mut invalid_teleport_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_category = artifact.content.clone();
    let AbilityEffectDefinition::Detect { category, .. } = &mut invalid_detect_category
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-sight")
        .expect("fixture should contain the persistent detection ability")
        .effect
    else {
        panic!("echo sight should use detection");
    };
    *category = "missing-category".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_category),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_radius = artifact.content.clone();
    let AbilityEffectDefinition::Detect { radius, .. } = &mut invalid_detect_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-sight")
        .expect("fixture should contain the persistent detection ability")
        .effect
    else {
        panic!("echo sight should use detection");
    };
    *radius = 9;
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_detect_target = artifact.content.clone();
    invalid_detect_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-pulse")
        .expect("fixture should contain the transient detection ability")
        .target
        .range = 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_detect_target),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut duplicate_transform_source = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain {
        source_terrain_ids, ..
    } = &mut duplicate_transform_source
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-delving")
        .expect("fixture should contain the digging terrain ability")
        .effect
    else {
        panic!("echo delving should transform terrain");
    };
    source_terrain_ids.push("demo.terrain.wall".to_owned());
    assert!(matches!(
        validate_and_normalize(&mut duplicate_transform_source),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_transform_target = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain {
        target_terrain_id, ..
    } = &mut invalid_transform_target
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-rampart")
        .expect("fixture should contain the terrain creation ability")
        .effect
    else {
        panic!("echo rampart should transform terrain");
    };
    *target_terrain_id = "demo.terrain.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_target),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut invalid_transform_radius = artifact.content.clone();
    let AbilityEffectDefinition::TransformTerrain { radius, .. } = &mut invalid_transform_radius
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-delving")
        .expect("fixture should contain the digging terrain ability")
        .effect
    else {
        panic!("echo delving should transform terrain");
    };
    *radius = 9;
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_radius),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_transform_target_mode = artifact.content.clone();
    invalid_transform_target_mode
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-rampart")
        .expect("fixture should contain the terrain creation ability")
        .target
        .modes = vec![AbilityTargetModeDefinition::Direction];
    assert!(matches!(
        validate_and_normalize(&mut invalid_transform_target_mode),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_empty_sequence = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_empty_sequence
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-quickening")
        .expect("fixture should contain the self status sequence")
        .effect
    else {
        panic!("echo quickening should use an effect sequence");
    };
    effects.clear();
    assert!(matches!(
        validate_and_normalize(&mut invalid_empty_sequence),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_nested_sequence = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_nested_sequence
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-binding")
        .expect("fixture should contain the target status sequence")
        .effect
    else {
        panic!("echo binding should use an effect sequence");
    };
    effects[0] = AbilityEffectDefinition::Sequence {
        effects: effects.clone(),
    };
    assert!(matches!(
        validate_and_normalize(&mut invalid_nested_sequence),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_status_duration = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_status_duration
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-binding")
        .expect("fixture should contain the target status sequence")
        .effect
    else {
        panic!("echo binding should use an effect sequence");
    };
    let AbilityEffectDefinition::ApplyStatus { duration_ticks, .. } = &mut effects[1] else {
        panic!("echo binding should apply slow second");
    };
    *duration_ticks = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_status_duration),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_self_sequence_member = artifact.content.clone();
    let AbilityEffectDefinition::Sequence { effects } = &mut invalid_self_sequence_member
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.echo-quickening")
        .expect("fixture should contain the self status sequence")
        .effect
    else {
        panic!("echo quickening should use an effect sequence");
    };
    effects.push(AbilityEffectDefinition::Damage {
        damage_dice: 1,
        damage_sides: 1,
        damage_bonus: 0,
        damage_type: ActorDamageType::Physical,
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_self_sequence_member),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_proficiency = artifact.content.clone();
    invalid_proficiency.abilities[0].proficiency.cap = 1_601;
    assert!(matches!(
        validate_and_normalize(&mut invalid_proficiency),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut invalid_cooldown = artifact.content.clone();
    invalid_cooldown
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.mending-echo")
        .expect("fixture should contain the healing ability")
        .cooldown
        .as_mut()
        .expect("healing ability should declare a cooldown")
        .turns = 0;
    assert!(matches!(
        validate_and_normalize(&mut invalid_cooldown),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut dangling_resource = artifact.content.clone();
    dangling_resource.abilities[0].resource_id = "demo.resource.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut dangling_resource),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut invalid_book_item = artifact.content.clone();
    let primer = invalid_book_item
        .items
        .iter_mut()
        .find(|item| item.id == "demo.item.echo-primer")
        .expect("fixture should contain the ability book item");
    primer.max_stack = 2;
    assert!(matches!(
        validate_and_normalize(&mut invalid_book_item),
        Err(ContentError::InvalidAbilityBookItem(_))
    ));

    let mut mismatched_profile = artifact.content;
    let mut focus = mismatched_profile.resources[0].clone();
    focus.id = "demo.resource.focus".to_owned();
    mismatched_profile.resources.push(focus);
    mismatched_profile
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .expect("fixture should contain the mage class")
        .casting_profile
        .as_mut()
        .expect("mage should have a casting profile")
        .resource_id = "demo.resource.focus".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut mismatched_profile),
        Err(ContentError::InvalidCastingProfile(_))
    ));
}

#[test]
fn casting_profiles_validate_per_ability_parameter_overrides() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut overridden = artifact.content;
    let profile = overridden
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile");
    profile
        .ability_overrides
        .push(AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.mending-echo".to_owned(),
            minimum_level: 7,
            resource_cost: 11,
            base_failure_percent: 42,
            level_scaling: Vec::new(),
        });
    validate_and_normalize(&mut overridden).expect("valid override should compile");

    let mut duplicate = overridden.clone();
    duplicate
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile")
        .ability_overrides
        .push(AbilityCastingOverrideDefinition {
            ability_id: "demo.ability.mending-echo".to_owned(),
            minimum_level: 8,
            resource_cost: 12,
            base_failure_percent: 43,
            level_scaling: Vec::new(),
        });
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidCastingProfile(_))
    ));

    let mut unsupported = overridden;
    unsupported
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.mage")
        .and_then(|class| class.casting_profile.as_mut())
        .expect("fixture should contain the mage casting profile")
        .ability_overrides[0]
        .ability_id = "demo.ability.not-in-a-mage-book".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut unsupported),
        Err(ContentError::InvalidCastingProfile(_))
    ));
}

#[test]
fn abilities_validate_actor_detection_control_and_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut valid = artifact.content.clone();
    let malediction = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    assert_eq!(malediction.level_scaling.len(), 1);
    let unlife = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection");
    assert!(matches!(
        unlife.effect,
        AbilityEffectDefinition::Detect {
            subject: AbilityDetectSubjectDefinition::Actor,
            persistent: false,
            ..
        }
    ));
    validate_and_normalize(&mut valid).expect("P54 abilities should compile");

    let mut duplicate = artifact.content.clone();
    let malediction = duplicate
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-malediction")
        .expect("fixture should contain level-scaled damage");
    malediction
        .level_scaling
        .push(malediction.level_scaling[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut out_of_bounds = artifact.content.clone();
    out_of_bounds
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-horrify")
        .expect("fixture should contain a scaled sequence")
        .level_scaling[0]
        .effect_index = 2;
    assert!(matches!(
        validate_and_normalize(&mut out_of_bounds),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut persistent_actor_detection = artifact.content.clone();
    let AbilityEffectDefinition::Detect { persistent, .. } = &mut persistent_actor_detection
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-detect-unlife")
        .expect("fixture should contain actor detection")
        .effect
    else {
        panic!("detect unlife should use actor detection");
    };
    *persistent = true;
    assert!(matches!(
        validate_and_normalize(&mut persistent_actor_detection),
        Err(ContentError::InvalidAbility(_))
    ));

    let mut missing_control_category = artifact.content;
    let AbilityEffectDefinition::Control { category, .. } = &mut missing_control_category
        .abilities
        .iter_mut()
        .find(|ability| ability.id == "demo.ability.death-enslave-undead")
        .expect("fixture should contain actor control")
        .effect
    else {
        panic!("enslave undead should use actor control");
    };
    *category = "missing-category".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut missing_control_category),
        Err(ContentError::InvalidAbility(_))
    ));
}

#[test]
fn zero_ability_bases_require_matching_level_scaling() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    for ability_id in [
        "demo.ability.death-death-ray",
        "demo.ability.death-raise-dead",
        "demo.ability.death-esoteria",
        "demo.ability.death-mass-genocide",
    ] {
        let mut invalid = artifact.content.clone();
        invalid
            .abilities
            .iter_mut()
            .find(|ability| ability.id == ability_id)
            .unwrap_or_else(|| panic!("fixture should contain {ability_id}"))
            .level_scaling
            .clear();
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::InvalidAbility(id)) if id == ability_id
        ));
    }
}
