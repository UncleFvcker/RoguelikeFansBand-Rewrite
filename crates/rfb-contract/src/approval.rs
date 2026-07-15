// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    CONTRACT_SCHEMA_VERSION, ContractFixture, LEGACY_BASELINE_COMMIT, snapshot,
    validate_fixture_set,
};

pub const BASELINE_POLICY_SCHEMA_VERSION: u16 = 1;
pub const DIFF_WAIVER_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BaselinePolicy {
    pub schema_version: u16,
    pub id: String,
    pub legacy_commit: String,
    pub contract_schema_version: u16,
    pub normalization_schema_version: u16,
    pub minimum_fixture_count: usize,
    pub fixture_directory: String,
    pub waiver_directory: String,
    pub minimum_approvals: usize,
    pub require_issue_reference: bool,
    pub forbid_wildcard_scope: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiffWaiver {
    pub schema_version: u16,
    pub id: String,
    pub status: WaiverStatus,
    pub fixture_id: String,
    pub change_kind: ChangeKind,
    pub affected_assertions: Vec<String>,
    pub old_normalized_hash: String,
    pub new_normalized_hash: String,
    pub reason: String,
    pub issue: String,
    pub approved_by: Vec<String>,
    pub approved_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WaiverStatus {
    Approved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeKind {
    IntentionalRuleChange,
    LegacyDivergence,
    NormalizationChange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineValidationReport {
    pub policy_id: String,
    pub fixture_count: usize,
    pub waiver_count: usize,
}

pub fn validate_policy_file(
    policy_path: &Path,
) -> Result<BaselineValidationReport, BaselinePolicyError> {
    let policy: BaselinePolicy = serde_json::from_slice(&fs::read(policy_path)?)?;
    validate_policy(&policy)?;
    let root = policy_path
        .parent()
        .ok_or_else(|| BaselinePolicyError::PolicyPath(policy_path.to_path_buf()))?;
    let fixture_dir = resolve_child(root, &policy.fixture_directory)?;
    let waiver_dir = resolve_child(root, &policy.waiver_directory)?;

    let fixture_paths = json_files(&fixture_dir)?;
    let fixtures = fixture_paths
        .iter()
        .map(|path| {
            serde_json::from_slice::<ContractFixture>(&fs::read(path)?).map_err(|error| {
                BaselinePolicyError::FixtureJson {
                    path: path.clone(),
                    error,
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fixtures.len() < policy.minimum_fixture_count {
        return Err(BaselinePolicyError::FixtureCount {
            minimum: policy.minimum_fixture_count,
            actual: fixtures.len(),
        });
    }
    validate_fixture_set(&fixtures)
        .map_err(|error| BaselinePolicyError::FixtureSet(error.to_string()))?;
    let fixture_ids = fixtures
        .iter()
        .map(|fixture| fixture.id.clone())
        .collect::<BTreeSet<_>>();

    let waiver_paths = json_files(&waiver_dir)?;
    let mut waiver_ids = BTreeSet::new();
    let mut waived_fixtures = BTreeSet::new();
    for path in &waiver_paths {
        let waiver: DiffWaiver = serde_json::from_slice(&fs::read(path)?).map_err(|error| {
            BaselinePolicyError::WaiverJson {
                path: path.clone(),
                error,
            }
        })?;
        validate_waiver(&policy, &waiver, &fixture_ids, path)?;
        if !waiver_ids.insert(waiver.id.clone()) {
            return Err(BaselinePolicyError::DuplicateWaiverId(waiver.id));
        }
        if !waived_fixtures.insert(waiver.fixture_id.clone()) {
            return Err(BaselinePolicyError::DuplicateFixtureWaiver(
                waiver.fixture_id,
            ));
        }
    }

    Ok(BaselineValidationReport {
        policy_id: policy.id,
        fixture_count: fixtures.len(),
        waiver_count: waiver_paths.len(),
    })
}

fn validate_policy(policy: &BaselinePolicy) -> Result<(), BaselinePolicyError> {
    if policy.schema_version != BASELINE_POLICY_SCHEMA_VERSION {
        return Err(BaselinePolicyError::PolicySchema(policy.schema_version));
    }
    if policy.id.trim().is_empty() {
        return Err(BaselinePolicyError::EmptyPolicyId);
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
    if policy.minimum_approvals == 0 {
        return Err(BaselinePolicyError::MinimumApprovals);
    }
    if !policy.require_issue_reference || !policy.forbid_wildcard_scope {
        return Err(BaselinePolicyError::RequiredSafeguards);
    }
    Ok(())
}

fn validate_waiver(
    policy: &BaselinePolicy,
    waiver: &DiffWaiver,
    fixture_ids: &BTreeSet<String>,
    path: &Path,
) -> Result<(), BaselinePolicyError> {
    if waiver.schema_version != DIFF_WAIVER_SCHEMA_VERSION {
        return Err(BaselinePolicyError::WaiverSchema(waiver.schema_version));
    }
    let file_stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("");
    if waiver.id != file_stem || !valid_identifier(&waiver.id) {
        return Err(BaselinePolicyError::WaiverId {
            id: waiver.id.clone(),
            file: path.to_path_buf(),
        });
    }
    if policy.forbid_wildcard_scope && wildcard(&waiver.fixture_id) {
        return Err(BaselinePolicyError::WildcardScope(
            waiver.fixture_id.clone(),
        ));
    }
    if !fixture_ids.contains(&waiver.fixture_id) {
        return Err(BaselinePolicyError::UnknownFixture(
            waiver.fixture_id.clone(),
        ));
    }
    let affected_assertions = waiver
        .affected_assertions
        .iter()
        .map(|scope| scope.trim())
        .collect::<BTreeSet<_>>();
    if affected_assertions.len() != waiver.affected_assertions.len()
        || affected_assertions.is_empty()
        || affected_assertions
            .iter()
            .any(|scope| scope.is_empty() || policy.forbid_wildcard_scope && wildcard(scope))
    {
        return Err(BaselinePolicyError::AssertionScope(waiver.id.clone()));
    }
    if !valid_sha256(&waiver.old_normalized_hash)
        || !valid_sha256(&waiver.new_normalized_hash)
        || waiver.old_normalized_hash == waiver.new_normalized_hash
    {
        return Err(BaselinePolicyError::WaiverHashes(waiver.id.clone()));
    }
    if waiver.reason.trim().chars().count() < 20 {
        return Err(BaselinePolicyError::WaiverReason(waiver.id.clone()));
    }
    if policy.require_issue_reference && !valid_issue(&waiver.issue) {
        return Err(BaselinePolicyError::IssueReference(waiver.issue.clone()));
    }
    let approvers = waiver
        .approved_by
        .iter()
        .map(|approver| approver.trim())
        .filter(|approver| !approver.is_empty() && !approver.eq_ignore_ascii_case("todo"))
        .collect::<BTreeSet<_>>();
    if approvers.len() < policy.minimum_approvals {
        return Err(BaselinePolicyError::Approvals {
            minimum: policy.minimum_approvals,
            actual: approvers.len(),
        });
    }
    if !valid_date(&waiver.approved_at)
        || waiver
            .expires_at
            .as_deref()
            .is_some_and(|date| !valid_date(date))
        || waiver
            .expires_at
            .as_deref()
            .is_some_and(|date| date < waiver.approved_at.as_str())
    {
        return Err(BaselinePolicyError::ApprovalDate(waiver.id.clone()));
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

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn wildcard(value: &str) -> bool {
    let value = value.trim();
    value.contains('*') || value.eq_ignore_ascii_case("all")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_issue(value: &str) -> bool {
    value.starts_with("https://github.com/")
        && value.split_once("/issues/").is_some_and(|(_, number)| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return false;
    }
    let year = u16::from(bytes[0] - b'0') * 1000
        + u16::from(bytes[1] - b'0') * 100
        + u16::from(bytes[2] - b'0') * 10
        + u16::from(bytes[3] - b'0');
    let month = (bytes[5] - b'0') * 10 + (bytes[6] - b'0');
    let day = (bytes[8] - b'0') * 10 + (bytes[9] - b'0');
    if year < 2026 || !(1..=12).contains(&month) {
        return false;
    }
    let leap_year =
        year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let maximum_day = match month {
        2 if leap_year => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=maximum_day).contains(&day)
}

#[derive(Debug, Error)]
pub enum BaselinePolicyError {
    #[error("unsupported baseline policy schema version {0}")]
    PolicySchema(u16),
    #[error("baseline policy ID cannot be empty")]
    EmptyPolicyId,
    #[error("baseline policy legacy commit does not match: {0}")]
    LegacyCommit(String),
    #[error("baseline policy contract schema does not match: {0}")]
    ContractSchema(u16),
    #[error("baseline policy normalization schema does not match: {0}")]
    NormalizationSchema(u16),
    #[error("baseline policy cannot require fewer than 20 fixtures: {0}")]
    MinimumFixturePolicy(usize),
    #[error("baseline policy must require at least one approval")]
    MinimumApprovals,
    #[error("baseline policy must require issue references and forbid wildcard scopes")]
    RequiredSafeguards,
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
    #[error("waiver JSON is invalid at {path}: {error}")]
    WaiverJson {
        path: PathBuf,
        error: serde_json::Error,
    },
    #[error("unsupported diff waiver schema version {0}")]
    WaiverSchema(u16),
    #[error("waiver ID {id} must match lowercase filename {file}")]
    WaiverId { id: String, file: PathBuf },
    #[error("waiver uses forbidden wildcard scope {0}")]
    WildcardScope(String),
    #[error("waiver references unknown fixture {0}")]
    UnknownFixture(String),
    #[error("waiver {0} must list explicit affected assertion paths")]
    AssertionScope(String),
    #[error("waiver {0} must contain distinct SHA-256 old/new hashes")]
    WaiverHashes(String),
    #[error("waiver {0} reason must contain at least 20 characters")]
    WaiverReason(String),
    #[error("waiver issue must be a GitHub issue URL: {0}")]
    IssueReference(String),
    #[error("waiver has {actual} valid approvals, policy requires {minimum}")]
    Approvals { minimum: usize, actual: usize },
    #[error("waiver {0} approval or expiry date is invalid")]
    ApprovalDate(String),
    #[error("duplicate waiver ID {0}")]
    DuplicateWaiverId(String),
    #[error("fixture {0} has more than one active waiver")]
    DuplicateFixtureWaiver(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> BaselinePolicy {
        BaselinePolicy {
            schema_version: 1,
            id: "rfb-contract-baseline-v1".to_owned(),
            legacy_commit: LEGACY_BASELINE_COMMIT.to_owned(),
            contract_schema_version: 1,
            normalization_schema_version: 1,
            minimum_fixture_count: 20,
            fixture_directory: "scenarios".to_owned(),
            waiver_directory: "waivers".to_owned(),
            minimum_approvals: 1,
            require_issue_reference: true,
            forbid_wildcard_scope: true,
        }
    }

    fn waiver() -> DiffWaiver {
        DiffWaiver {
            schema_version: 1,
            id: "waiver-2026-001".to_owned(),
            status: WaiverStatus::Approved,
            fixture_id: "movement.direction.north".to_owned(),
            change_kind: ChangeKind::IntentionalRuleChange,
            affected_assertions: vec!["finalState.stateHash".to_owned()],
            old_normalized_hash: "a".repeat(64),
            new_normalized_hash: "b".repeat(64),
            reason: "Intentional rule correction documented by the linked issue.".to_owned(),
            issue: "https://github.com/UncleFvcker/RoguelikeFansBand-Rewrite/issues/1".to_owned(),
            approved_by: vec!["maintainer".to_owned()],
            approved_at: "2026-07-15".to_owned(),
            expires_at: None,
        }
    }

    #[test]
    fn valid_waiver_passes_structural_checks() {
        let fixture_ids = BTreeSet::from(["movement.direction.north".to_owned()]);
        validate_waiver(
            &policy(),
            &waiver(),
            &fixture_ids,
            Path::new("waiver-2026-001.json"),
        )
        .expect("waiver should be valid");
    }

    #[test]
    fn wildcard_scope_is_rejected() {
        let fixture_ids = BTreeSet::from(["movement.direction.north".to_owned()]);
        let mut waiver = waiver();
        waiver.affected_assertions = vec!["*".to_owned()];
        assert!(matches!(
            validate_waiver(
                &policy(),
                &waiver,
                &fixture_ids,
                Path::new("waiver-2026-001.json")
            ),
            Err(BaselinePolicyError::AssertionScope(_))
        ));
    }

    #[test]
    fn identical_result_hashes_are_rejected() {
        let fixture_ids = BTreeSet::from(["movement.direction.north".to_owned()]);
        let mut waiver = waiver();
        waiver.new_normalized_hash = waiver.old_normalized_hash.clone();
        assert!(matches!(
            validate_waiver(
                &policy(),
                &waiver,
                &fixture_ids,
                Path::new("waiver-2026-001.json")
            ),
            Err(BaselinePolicyError::WaiverHashes(_))
        ));
    }

    #[test]
    fn malformed_dates_are_rejected_without_panicking() {
        assert!(!valid_date("2026-13-01"));
        assert!(!valid_date("2026-02-29"));
        assert!(valid_date("2028-02-29"));
        assert!(!valid_date("二〇二六-07-15"));
        assert!(valid_date("2026-07-15"));
    }
}
