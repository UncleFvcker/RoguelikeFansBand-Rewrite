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
    assert!(matches!(
        malediction.effect,
        AbilityEffectDefinition::Malediction {
            damage_dice: 3,
            damage_sides: 4,
            damage_bonus: 0,
        }
    ));
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

#[test]
fn arcane_first_book_keeps_the_original_spell_table_and_narrow_effects() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let content = artifact.content;
    let book = content
        .ability_books
        .iter()
        .find(|book| book.id == "demo.ability-book.cantrips-for-beginners")
        .expect("Arcane first book should compile");
    assert_eq!(book.realm_id.as_deref(), Some("arcane"));
    assert_eq!(book.rank, Some(1));
    assert_eq!(book.ability_ids.len(), 8);

    let expected = [
        ("demo.ability.arcane-zap", 1, 1, 15, 4),
        ("demo.ability.arcane-wizard-lock", 1, 1, 23, 5),
        ("demo.ability.arcane-detect-invisibility", 1, 1, 23, 4),
        ("demo.ability.arcane-detect-monsters", 1, 1, 23, 5),
        ("demo.ability.arcane-blink", 2, 1, 23, 10),
        ("demo.ability.arcane-light-area", 3, 2, 33, 18),
        ("demo.ability.arcane-trap-door-destruction", 4, 4, 23, 28),
        ("demo.ability.arcane-cure-light-wounds", 5, 4, 33, 25),
    ];
    for (id, level, mana, failure, experience) in expected {
        let player = content
            .abilities
            .iter()
            .find(|ability| ability.id == id)
            .and_then(|ability| ability.player.as_ref())
            .unwrap_or_else(|| panic!("{id} should have a player binding"));
        assert_eq!(
            (
                player.minimum_level,
                player.resource_cost,
                player.base_failure_percent,
                player.first_success_experience,
            ),
            (level, mana, failure, experience)
        );
    }

    let lock = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-wizard-lock")
        .expect("Wizard Lock should compile");
    assert!(matches!(
        lock.effect,
        AbilityEffectDefinition::TerrainBeam {
            operation: AbilityTerrainBeamOperationDefinition::JamDoors
        }
    ));
    let cure = content
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.arcane-cure-light-wounds")
        .expect("Cure Light Wounds should compile");
    assert!(matches!(
        &cure.effect,
        AbilityEffectDefinition::Sequence { effects }
            if matches!(effects.as_slice(), [
                AbilityEffectDefinition::HealDice { dice: 2, sides: 8 },
                AbilityEffectDefinition::ReduceStatus { status_kind_id, amount: 10 }
            ] if status_kind_id == "rfb.status.bleeding")
    ));
}
