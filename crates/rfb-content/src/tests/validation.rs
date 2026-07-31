use super::*;

#[test]
fn observable_rule_entries_require_their_skill_kinds() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    for (kind, expected) in [
        (SkillKind::Device, "device"),
        (SkillKind::SavingThrow, "saving-throw"),
        (SkillKind::Stealth, "stealth"),
        (SkillKind::Perception, "perception"),
    ] {
        let mut invalid = artifact.content.clone();
        invalid.skills.retain(|skill| skill.kind != kind);
        assert!(matches!(
            validate_and_normalize(&mut invalid),
            Err(ContentError::MissingRequiredSkillKind(actual)) if actual == expected
        ));
    }
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

    let mut blocked_spawn = artifact.content.clone();
    blocked_spawn.worlds[0].player.position = ContentPosition { x: 11, y: 3 };
    assert!(matches!(
        validate_and_normalize(&mut blocked_spawn),
        Err(ContentError::SpawnOnBlockedTerrain(_))
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
