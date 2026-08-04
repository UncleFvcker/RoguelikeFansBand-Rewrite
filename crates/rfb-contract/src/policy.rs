// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CONTRACT_SCHEMA_VERSION, ContractFixture, FixtureCategory, LEGACY_BASELINE_COMMIT, snapshot,
    validate_fixture_set,
};

pub const BASELINE_POLICY_SCHEMA_VERSION: u16 = 2;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaselinePolicy {
    pub schema_version: u16,
    pub baseline: String,
    pub legacy_commit: String,
    pub contract_schema_version: u16,
    pub normalization_schema_version: u16,
    pub minimum_fixture_count: usize,
    pub fixture_directory: String,
    pub waiver_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineValidationReport {
    pub baseline: String,
    pub fixture_count: usize,
    pub waiver_count: usize,
    pub category_counts: BTreeMap<FixtureCategory, usize>,
}

#[derive(Debug, Clone)]
pub struct ContractFixtureFile {
    pub path: PathBuf,
    pub fixture: ContractFixture,
}

pub fn validate_policy_file(
    policy_path: &Path,
) -> Result<BaselineValidationReport, BaselinePolicyError> {
    let (policy, fixtures) = load_policy_fixture_files_inner(policy_path)?;
    let mut category_counts = FixtureCategory::ALL
        .into_iter()
        .map(|category| (category, 0))
        .collect::<BTreeMap<_, _>>();
    for file in &fixtures {
        *category_counts.entry(file.fixture.category).or_default() += 1;
    }

    Ok(BaselineValidationReport {
        baseline: policy.baseline,
        fixture_count: fixtures.len(),
        waiver_count: 0,
        category_counts,
    })
}

pub fn load_policy_fixture_files(
    policy_path: &Path,
) -> Result<Vec<ContractFixtureFile>, BaselinePolicyError> {
    let (_, fixtures) = load_policy_fixture_files_inner(policy_path)?;
    Ok(fixtures)
}

fn load_policy_fixture_files_inner(
    policy_path: &Path,
) -> Result<(BaselinePolicy, Vec<ContractFixtureFile>), BaselinePolicyError> {
    let policy: BaselinePolicy = serde_json::from_slice(&fs::read(policy_path)?)?;
    validate_policy(&policy)?;
    let root = policy_path
        .parent()
        .ok_or_else(|| BaselinePolicyError::PolicyPath(policy_path.to_path_buf()))?;
    let fixture_dir = resolve_child(root, &policy.fixture_directory)?;
    let waiver_dir = resolve_child(root, &policy.waiver_directory)?;

    let fixtures =
        json_files(&fixture_dir)?
            .into_iter()
            .map(|path| {
                let fixture = serde_json::from_slice::<ContractFixture>(&fs::read(&path)?)
                    .map_err(|error| BaselinePolicyError::FixtureJson {
                        path: path.clone(),
                        error,
                    })?;
                Ok::<_, BaselinePolicyError>(ContractFixtureFile { path, fixture })
            })
            .collect::<Result<Vec<_>, _>>()?;
    if fixtures.len() < policy.minimum_fixture_count {
        return Err(BaselinePolicyError::FixtureCount {
            minimum: policy.minimum_fixture_count,
            actual: fixtures.len(),
        });
    }
    validate_fixture_set(
        &fixtures
            .iter()
            .map(|file| file.fixture.clone())
            .collect::<Vec<_>>(),
    )
    .map_err(|error| BaselinePolicyError::FixtureSet(error.to_string()))?;
    require_empty_waiver_directory(&waiver_dir)?;

    Ok((policy, fixtures))
}

fn validate_policy(policy: &BaselinePolicy) -> Result<(), BaselinePolicyError> {
    if policy.schema_version != BASELINE_POLICY_SCHEMA_VERSION {
        return Err(BaselinePolicyError::PolicySchema(policy.schema_version));
    }
    if policy.baseline.trim().is_empty() {
        return Err(BaselinePolicyError::EmptyBaseline);
    }
    if policy.legacy_commit != LEGACY_BASELINE_COMMIT {
        return Err(BaselinePolicyError::LegacyCommit(
            policy.legacy_commit.clone(),
        ));
    }
    if policy.contract_schema_version != CONTRACT_SCHEMA_VERSION {
        return Err(BaselinePolicyError::ContractSchema(
            policy.contract_schema_version,
        ));
    }
    if policy.normalization_schema_version != snapshot::SNAPSHOT_NORMALIZATION_SCHEMA_VERSION {
        return Err(BaselinePolicyError::NormalizationSchema(
            policy.normalization_schema_version,
        ));
    }
    if policy.minimum_fixture_count < 20 {
        return Err(BaselinePolicyError::MinimumFixturePolicy(
            policy.minimum_fixture_count,
        ));
    }
    Ok(())
}

fn resolve_child(root: &Path, relative: &str) -> Result<PathBuf, BaselinePolicyError> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(BaselinePolicyError::UnsafeRelativePath(relative.to_owned()));
    }
    Ok(root.join(path))
}

fn json_files(directory: &Path) -> Result<Vec<PathBuf>, BaselinePolicyError> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    Ok(paths)
}

fn require_empty_waiver_directory(directory: &Path) -> Result<(), BaselinePolicyError> {
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.file_name().is_some_and(|name| name == ".gitkeep") && path.is_file() {
            continue;
        }
        return Err(BaselinePolicyError::WaiversForbidden(path));
    }
    Ok(())
}

#[derive(Debug, Error)]
pub enum BaselinePolicyError {
    #[error("unsupported baseline policy schema version {0}")]
    PolicySchema(u16),
    #[error("baseline policy must name the active contract baseline")]
    EmptyBaseline,
    #[error("baseline policy legacy commit does not match: {0}")]
    LegacyCommit(String),
    #[error("baseline policy contract schema does not match: {0}")]
    ContractSchema(u16),
    #[error("baseline policy normalization schema does not match: {0}")]
    NormalizationSchema(u16),
    #[error("baseline policy cannot require fewer than 20 fixtures: {0}")]
    MinimumFixturePolicy(usize),
    #[error("baseline policy path has no parent: {0}")]
    PolicyPath(PathBuf),
    #[error("baseline policy contains unsafe relative path {0}")]
    UnsafeRelativePath(String),
    #[error("fixture count fell below policy minimum {minimum}: {actual}")]
    FixtureCount { minimum: usize, actual: usize },
    #[error("contract fixture set is invalid: {0}")]
    FixtureSet(String),
    #[error("fixture JSON is invalid at {path}: {error}")]
    FixtureJson {
        path: PathBuf,
        error: serde_json::Error,
    },
    #[error("contract diff waivers are not supported; remove {0}")]
    WaiversForbidden(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
