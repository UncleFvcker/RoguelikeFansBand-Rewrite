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
