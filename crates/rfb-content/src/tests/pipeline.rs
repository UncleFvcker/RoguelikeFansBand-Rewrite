use super::*;

#[test]
fn invalid_pack_metadata_and_source_headers_are_rejected() {
    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_metadata = artifact.content.clone();
    invalid_metadata.format = "invalid".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_metadata),
        Err(ContentError::InvalidCompiledMetadata)
    ));

    let mut invalid_pack_id = artifact.content.clone();
    invalid_pack_id.pack_id = "INVALID".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_id),
        Err(ContentError::InvalidStableId(id)) if id == "INVALID"
    ));

    let mut invalid_pack_version = artifact.content.clone();
    invalid_pack_version.pack_version = "01.0.0".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_version),
        Err(ContentError::InvalidPackVersion(version)) if version == "01.0.0"
    ));

    let mut invalid_title = artifact.content.clone();
    invalid_title.title_key = "INVALID TITLE".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_title),
        Err(ContentError::InvalidMessageKey(key)) if key == "INVALID TITLE"
    ));

    let mut invalid_terrain_schema = artifact.content.clone();
    invalid_terrain_schema.terrain[0].schema = "invalid".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_schema),
        Err(ContentError::SchemaMismatch(_))
    ));

    let mut invalid_terrain_version = artifact.content.clone();
    invalid_terrain_version.terrain[0].format_version = CONTENT_FORMAT_VERSION + 1;
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_version),
        Err(ContentError::UnsupportedSourceVersion { .. })
    ));
}

#[test]
fn original_pack_compiles_deterministically_and_round_trips() {
    let first = verify_pack_lock(&original_pack_path()).expect("original pack should verify");
    let second = compile_pack_dir(&original_pack_path()).expect("recompile should succeed");
    let decoded = decode_content(&first.bytes).expect("compiled pack should decode");

    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(first.bytes, second.bytes);
    assert_eq!(decoded, first);
    assert_eq!(first.content.pack_id, "rfb.demo.original-v1");
}
