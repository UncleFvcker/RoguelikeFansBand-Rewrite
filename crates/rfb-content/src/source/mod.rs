// SPDX-License-Identifier: MPL-2.0

mod items;

use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::Path,
};

use serde::de::DeserializeOwned;

use crate::{
    CONTENT_FORMAT, CONTENT_FORMAT_VERSION, CompiledArtifact, CompiledContentV1, ContentError,
    ContentLockV1, PACK_SCHEMA, PackManifest,
    ability_programs::{SourceAbilityDefinition, compile_ability_program_catalog},
    effect_programs::{
        ResolvedEffectProgram, compile_effect_program_catalog,
        effect_program_input_matches_device_target, resolve_source_item_effect,
        validate_effect_program_catalog,
    },
    encode_content,
    player_ability_bindings::{
        compile_player_ability_binding_catalog, validate_player_ability_binding_references,
    },
    validation::{
        require_format_version, require_schema, validate_id, validate_message_key,
        validate_pack_relations, validate_semver,
    },
};

pub(crate) use items::SourceItemDefinition;
#[cfg(test)]
pub(crate) use items::SourceItemUseActionDefinition;
const MAX_SOURCE_FILE_LENGTH: usize = 2 * 1024 * 1024;
const MAX_SOURCE_TOTAL_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SOURCE_FILES: usize = 32_768;
pub(crate) const SUPPORTED_ROOTS: [&str; 27] = [
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
    "mutations",
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
    "townFacilities",
    "vaults",
    "worlds",
];

#[derive(Debug, Default)]
struct SourceBudget {
    files: usize,
    bytes: usize,
}

pub fn compile_pack_dir(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let metadata = fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidPackRoot(root.to_path_buf()));
    }

    let mut budget = SourceBudget::default();
    let manifest: PackManifest = read_json(&root.join("pack.json"), &mut budget)?;
    validate_manifest(&manifest)?;

    let roots = manifest
        .content_roots
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let effect_programs =
        compile_effect_program_catalog(load_root(root, "effectPrograms", &roots, &mut budget)?)?;
    let items = load_root::<SourceItemDefinition>(root, "items", &roots, &mut budget)?
        .into_iter()
        .map(|item| item.into_compiled(&effect_programs))
        .collect::<Result<Vec<_>, _>>()?;
    let mut affixes = load_root(root, "affixes", &roots, &mut budget)?;
    resolve_affix_effect_programs(&mut affixes, &effect_programs)?;
    let ability_programs =
        compile_ability_program_catalog(load_root(root, "abilityPrograms", &roots, &mut budget)?)?;
    let player_ability_bindings = compile_player_ability_binding_catalog(load_root(
        root,
        "playerAbilityBindings",
        &roots,
        &mut budget,
    )?)?;
    let abilities = load_root::<SourceAbilityDefinition>(root, "abilities", &roots, &mut budget)?
        .into_iter()
        .map(|ability| ability.into_compiled(&ability_programs, &player_ability_bindings))
        .collect::<Result<Vec<_>, _>>()?;
    validate_player_ability_binding_references(&player_ability_bindings, &abilities)?;
    let content = CompiledContentV1 {
        format: CONTENT_FORMAT.to_owned(),
        format_version: CONTENT_FORMAT_VERSION,
        pack_id: manifest.id,
        pack_version: manifest.version,
        title_key: manifest.title_key,
        dependencies: manifest.dependencies,
        load_after: manifest.load_after,
        terrain: load_root(root, "terrain", &roots, &mut budget)?,
        actors: load_root(root, "actors", &roots, &mut budget)?,
        affixes,
        items,
        resources: load_root(root, "resources", &roots, &mut budget)?,
        abilities,
        ability_books: load_root(root, "abilityBooks", &roots, &mut budget)?,
        skills: load_root(root, "skills", &roots, &mut budget)?,
        skill_sets: load_root(root, "skillSets", &roots, &mut budget)?,
        races: load_root(root, "races", &roots, &mut budget)?,
        classes: load_root(root, "classes", &roots, &mut budget)?,
        personalities: load_root(root, "personalities", &roots, &mut budget)?,
        builds: load_root(root, "builds", &roots, &mut budget)?,
        mutations: load_root(root, "mutations", &roots, &mut budget)?,
        encounter_tables: load_root(root, "encounterTables", &roots, &mut budget)?,
        loot_tables: load_root(root, "lootTables", &roots, &mut budget)?,
        theme_tables: load_root(root, "themeTables", &roots, &mut budget)?,
        region_tables: load_root(root, "regionTables", &roots, &mut budget)?,
        terrain_feature_tables: load_root(root, "terrainFeatureTables", &roots, &mut budget)?,
        vaults: load_root(root, "vaults", &roots, &mut budget)?,
        towns: load_root(root, "towns", &roots, &mut budget)?,
        town_facilities: load_root(root, "townFacilities", &roots, &mut budget)?,
        shops: load_root(root, "shops", &roots, &mut budget)?,
        worlds: load_root(root, "worlds", &roots, &mut budget)?,
    };
    validate_effect_program_catalog(&effect_programs, &content)?;
    encode_content(content)
}

fn resolve_affix_effect_programs(
    affixes: &mut [crate::AffixDefinition],
    programs: &std::collections::BTreeMap<String, ResolvedEffectProgram>,
) -> Result<(), ContentError> {
    for affix in affixes {
        let Some(generation) = &mut affix.device_generation else {
            continue;
        };
        for activation in &mut generation.activations {
            let Some(program_id) = activation.effect_program_id.take() else {
                continue;
            };
            let (effect, input) = resolve_source_item_effect(&activation.id, program_id, programs)?;
            if !effect_program_input_matches_device_target(input, &activation.target) {
                return Err(ContentError::InvalidItemUseAction(activation.id.clone()));
            }
            activation.effect = effect;
        }
    }
    Ok(())
}

pub fn verify_pack_lock(root: &Path) -> Result<CompiledArtifact, ContentError> {
    let artifact = compile_pack_dir(root)?;
    let mut budget = SourceBudget::default();
    let content_lock: ContentLockV1 = read_json(&root.join("content.lock.json"), &mut budget)?;
    if content_lock.schema_version != 1
        || content_lock.pack_id != artifact.content.pack_id
        || content_lock.pack_version != artifact.content.pack_version
        || content_lock.content_hash != artifact.content_hash
    {
        return Err(ContentError::ContentLockMismatch);
    }
    Ok(artifact)
}

fn validate_manifest(manifest: &PackManifest) -> Result<(), ContentError> {
    require_schema(&manifest.schema, PACK_SCHEMA, "pack.json")?;
    require_format_version(manifest.format_version, "pack.json")?;
    validate_id(&manifest.id)?;
    validate_semver(&manifest.version)?;
    validate_message_key(&manifest.title_key)?;

    let mut roots = BTreeSet::new();
    for root in &manifest.content_roots {
        if !SUPPORTED_ROOTS.contains(&root.as_str()) {
            return Err(ContentError::UnsupportedContentRoot(root.clone()));
        }
        if !roots.insert(root.as_str()) {
            return Err(ContentError::DuplicateContentRoot(root.clone()));
        }
    }
    validate_pack_relations(&manifest.id, &manifest.dependencies, &manifest.load_after)
}

fn load_root<T: DeserializeOwned>(
    pack_root: &Path,
    root: &str,
    enabled_roots: &BTreeSet<&str>,
    budget: &mut SourceBudget,
) -> Result<Vec<T>, ContentError> {
    if !enabled_roots.contains(root) {
        return Ok(Vec::new());
    }
    let directory = pack_root.join(root);
    let metadata = fs::symlink_metadata(&directory)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentDirectory(directory));
    }
    let mut paths = fs::read_dir(&directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.sort();
    let mut definitions = Vec::with_capacity(paths.len());
    for path in paths {
        let metadata = fs::symlink_metadata(&path)?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || path.extension().and_then(|value| value.to_str()) != Some("json")
        {
            return Err(ContentError::InvalidContentFile(path));
        }
        definitions.push(read_json(&path, budget)?);
    }
    Ok(definitions)
}

fn read_json<T: DeserializeOwned>(
    path: &Path,
    budget: &mut SourceBudget,
) -> Result<T, ContentError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(ContentError::InvalidContentFile(path.to_path_buf()));
    }
    budget.files = budget
        .files
        .checked_add(1)
        .ok_or(ContentError::LengthOverflow)?;
    if budget.files > MAX_SOURCE_FILES {
        return Err(ContentError::TooManySourceFiles(budget.files));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_SOURCE_FILE_LENGTH + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_FILE_LENGTH {
        return Err(ContentError::SourceFileTooLarge(path.to_path_buf()));
    }
    budget.bytes = budget
        .bytes
        .checked_add(bytes.len())
        .ok_or(ContentError::LengthOverflow)?;
    if budget.bytes > MAX_SOURCE_TOTAL_LENGTH {
        return Err(ContentError::SourcePackTooLarge(budget.bytes));
    }
    serde_json::from_slice(&bytes).map_err(|source| ContentError::InvalidJson {
        path: path.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EffectProgramDefinition, ItemUseEffectDefinition};

    #[test]
    fn affix_activation_resolves_shared_effect_program() {
        let programs = compile_effect_program_catalog(vec![
            serde_json::from_value::<EffectProgramDefinition>(serde_json::json!({
                "$schema": crate::EFFECT_PROGRAM_SCHEMA,
                "formatVersion": 1,
                "id": "test.effect.riding-charge",
                "input": "actor",
                "steps": [{ "type": "riding-charge" }],
            }))
            .expect("test effect program should deserialize"),
        ])
        .expect("test effect program should compile");
        let mut affixes = vec![
            serde_json::from_value::<crate::AffixDefinition>(serde_json::json!({
                "$schema": crate::AFFIX_SCHEMA,
                "formatVersion": 1,
                "id": "test.affix.riding-charge",
                "nameKey": "test-affix-riding-charge-name",
                "descriptionKey": "test-affix-riding-charge-description",
                "generationLevel": 1,
                "deviceGeneration": {
                    "activations": [{
                        "id": "test.activation.riding-charge",
                        "nameKey": "test-activation-riding-charge-name",
                        "weight": 1,
                        "minDepth": 1,
                        "maxDepth": 100,
                        "deviceCheckDifficulty": 10,
                        "charges": { "minimum": 1, "maximum": 1, "cost": 1 },
                        "recovery": { "intervalTicks": 1000, "energyPerMille": 1000 },
                        "target": {
                            "modes": ["direction", "entity"],
                            "range": 7,
                            "requiresLineOfEffect": true
                        },
                        "effectProgramId": "test.effect.riding-charge"
                    }]
                },
                "tags": ["test"]
            }))
            .expect("test affix should deserialize"),
        ];

        resolve_affix_effect_programs(&mut affixes, &programs)
            .expect("affix activation should resolve the shared effect program");
        let activation = &affixes[0]
            .device_generation
            .as_ref()
            .expect("test affix should keep device generation")
            .activations[0];
        assert_eq!(activation.effect_program_id, None);
        assert_eq!(activation.effect, ItemUseEffectDefinition::RidingCharge);
    }
}
