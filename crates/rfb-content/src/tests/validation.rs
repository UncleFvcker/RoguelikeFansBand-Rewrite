use super::*;

#[test]
fn dynamic_devices_require_the_device_skill() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid
        .skills
        .retain(|skill| skill.kind != SkillKind::Device);
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::MissingRequiredSkillKind(actual)) if actual == "device"
    ));
}

#[test]
fn dangling_references_and_checksum_corruption_are_rejected() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut invalid = artifact.content.clone();
    invalid.worlds[0].fill_terrain_id = "demo.terrain.missing".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid),
        Err(ContentError::DanglingReference { .. })
    ));

    let mut corrupted = artifact.bytes;
    let last = corrupted.len() - 1;
    corrupted[last] ^= 0x01;
    assert!(matches!(
        decode_content(&corrupted),
        Err(ContentError::ChecksumMismatch)
    ));
}

#[test]
fn semantic_versions_are_checked_strictly() {
    assert!(validate_semver("1.2.3-alpha.1+build.5").is_ok());
    for invalid in ["01.2.3", "1.2", "1.2.3-", "1.2.3+", "1.2.3-alpha..1"] {
        assert!(matches!(
            validate_semver(invalid),
            Err(ContentError::InvalidPackVersion(_))
        ));
    }
}

#[test]
fn class_level_resistance_thresholds_are_strict() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let mut duplicate = artifact.content.clone();
    let paladin = duplicate
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.paladin")
        .expect("Paladin class should exist");
    paladin
        .level_resistances
        .push(paladin.level_resistances[0].clone());
    assert!(matches!(
        validate_and_normalize(&mut duplicate),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.paladin"
    ));

    let mut empty = artifact.content;
    empty
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.paladin")
        .expect("Paladin class should exist")
        .level_resistances[0]
        .resistances
        .clear();
    assert!(matches!(
        validate_and_normalize(&mut empty),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.paladin"
    ));
}

#[test]
fn sniping_profiles_and_concentration_requirements_are_strict() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");
    let profile = SnipingProfileDefinition {
        preferred_ammunition_type: AmmunitionTypeDefinition::Bolt,
        preferred_ammunition_to_hit_base: 10,
        preferred_ammunition_to_hit_level_divisor: 5,
        base_shot_excess_percent: 50,
        preferred_ammunition_critical_chance_percent: 150,
        base_concentration_maximum: 2,
        concentration_level_offset: 5,
        concentration_level_divisor: 10,
        concentration_bonus_percent_per_level: 10,
    };
    let mut valid = artifact.content.clone();
    let mut shot = valid
        .abilities
        .iter()
        .find(|ability| ability.id == "demo.ability.archer-create-shots")
        .expect("Archer power should exist")
        .clone();
    shot.id = "test.ability.sniper-shot".to_owned();
    shot.target = AbilityTargetDefinition {
        modes: vec![AbilityTargetModeDefinition::Direction],
        range: 20,
        requires_line_of_effect: true,
    };
    shot.effect = AbilityEffectDefinition::SniperShot {
        mode: SniperShotModeDefinition::Shining,
    };
    valid.abilities.push(shot);
    let archer = valid
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist");
    archer.sniping_profile = Some(profile);
    archer.abilities[0].ability_id = "test.ability.sniper-shot".to_owned();
    archer.abilities[0].minimum_concentration = 1;
    archer.abilities[0].hit_point_cost = 1;
    assert!(validate_and_normalize(&mut valid).is_ok());

    let mut no_profile = artifact.content.clone();
    no_profile
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist")
        .abilities[0]
        .minimum_concentration = 1;
    assert!(matches!(
        validate_and_normalize(&mut no_profile),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.archer"
    ));

    let mut invalid_divisor = artifact.content;
    let archer = invalid_divisor
        .classes
        .iter_mut()
        .find(|class| class.id == "demo.class.archer")
        .expect("Archer class should exist");
    archer.sniping_profile = Some(SnipingProfileDefinition {
        concentration_level_divisor: 0,
        ..profile
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_divisor),
        Err(ContentError::InvalidCharacterSource(id)) if id == "demo.class.archer"
    ));
}
