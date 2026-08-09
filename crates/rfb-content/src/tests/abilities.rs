use super::*;

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
