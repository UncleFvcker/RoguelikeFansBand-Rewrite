use super::*;

#[test]
fn source_roots_and_validation_precedence_are_stable() {
    assert_eq!(
        crate::source::SUPPORTED_ROOTS,
        [
            "abilities",
            "abilityBooks",
            "abilityPrograms",
            "actors",
            "affixes",
            "builds",
            "classes",
            "effectPrograms",
            "encounterTables",
            "items",
            "lootTables",
            "personalities",
            "playerAbilityBindings",
            "races",
            "regionTables",
            "resources",
            "skills",
            "skillSets",
            "shops",
            "terrain",
            "terrainFeatureTables",
            "themeTables",
            "towns",
            "vaults",
            "worlds",
        ]
    );

    let artifact = compile_pack_dir(&original_pack_path()).expect("original pack should compile");

    let mut invalid_metadata = artifact.content.clone();
    invalid_metadata.format = "invalid".to_owned();
    invalid_metadata.pack_id = "INVALID".to_owned();
    invalid_metadata.pack_version = "01.0.0".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_metadata),
        Err(ContentError::InvalidCompiledMetadata)
    ));

    let mut invalid_pack_id = artifact.content.clone();
    invalid_pack_id.pack_id = "INVALID".to_owned();
    invalid_pack_id.pack_version = "01.0.0".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_id),
        Err(ContentError::InvalidStableId(id)) if id == "INVALID"
    ));

    let mut invalid_pack_version = artifact.content.clone();
    invalid_pack_version.pack_version = "01.0.0".to_owned();
    invalid_pack_version.title_key = "INVALID TITLE".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_pack_version),
        Err(ContentError::InvalidPackVersion(version)) if version == "01.0.0"
    ));

    let mut invalid_title = artifact.content.clone();
    invalid_title.title_key = "INVALID TITLE".to_owned();
    invalid_title.dependencies.push(PackDependency {
        id: invalid_title.pack_id.clone(),
        version_requirement: "*".to_owned(),
    });
    assert!(matches!(
        validate_and_normalize(&mut invalid_title),
        Err(ContentError::InvalidMessageKey(key)) if key == "INVALID TITLE"
    ));

    let mut invalid_terrain_schema = artifact.content.clone();
    invalid_terrain_schema.terrain[0].schema = "invalid".to_owned();
    invalid_terrain_schema.terrain[0].format_version = CONTENT_FORMAT_VERSION + 1;
    invalid_terrain_schema.terrain[0].id = "INVALID".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_schema),
        Err(ContentError::SchemaMismatch(_))
    ));

    let mut invalid_terrain_version = artifact.content.clone();
    invalid_terrain_version.terrain[0].format_version = CONTENT_FORMAT_VERSION + 1;
    invalid_terrain_version.terrain[0].id = "INVALID".to_owned();
    assert!(matches!(
        validate_and_normalize(&mut invalid_terrain_version),
        Err(ContentError::UnsupportedSourceVersion { .. })
    ));
}

#[cfg(feature = "schemas")]
#[test]
fn generated_schema_document_order_is_stable() {
    let documents = generated_schema_documents().expect("schemas should generate");
    assert_eq!(
        documents
            .iter()
            .map(|(file_name, _)| *file_name)
            .collect::<Vec<_>>(),
        vec![
            "pack.schema.json",
            "terrain.schema.json",
            "actor.schema.json",
            "item.schema.json",
            "effect-program.schema.json",
            "resource.schema.json",
            "ability.schema.json",
            "ability-program.schema.json",
            "player-ability-binding.schema.json",
            "ability-book.schema.json",
            "skill.schema.json",
            "skill-set.schema.json",
            "race.schema.json",
            "class.schema.json",
            "personality.schema.json",
            "build.schema.json",
            "affix.schema.json",
            "encounter-table.schema.json",
            "loot-table.schema.json",
            "theme-table.schema.json",
            "region-table.schema.json",
            "terrain-feature-table.schema.json",
            "vault.schema.json",
            "town.schema.json",
            "shop.schema.json",
            "world.schema.json",
        ]
    );
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
    assert_eq!(first.content.terrain.len(), 66);
    assert_eq!(first.content.actors.len(), 33);
    assert_eq!(first.content.affixes.len(), 4);
    assert_eq!(first.content.items.len(), 111);
    assert_eq!(first.content.resources.len(), 3);
    assert_eq!(first.content.abilities.len(), 68);
    assert_eq!(first.content.ability_books.len(), 6);
    assert_eq!(first.content.skills.len(), 10);
    assert_eq!(first.content.skill_sets.len(), 13);
    assert_eq!(first.content.races.len(), 5);
    assert_eq!(first.content.classes.len(), 6);
    assert_eq!(first.content.personalities.len(), 3);
    assert_eq!(first.content.builds.len(), 7);
    assert_eq!(first.content.encounter_tables.len(), 7);
    assert_eq!(first.content.loot_tables.len(), 12);
    assert_eq!(first.content.theme_tables.len(), 3);
    assert_eq!(first.content.region_tables.len(), 1);
    assert_eq!(first.content.terrain_feature_tables.len(), 1);
    assert_eq!(first.content.vaults.len(), 6);
    assert_eq!(first.content.towns.len(), 1);
    assert_eq!(first.content.shops.len(), 7);
    assert_eq!(first.content.worlds.len(), 2);
}
