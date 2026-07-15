// SPDX-License-Identifier: MPL-2.0

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

const DEFAULT_REF: &str = "v1.3.0.7";
const DEFAULT_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";
const BASELINE_SAVE_VERSION: [u8; 4] = [1, 3, 0, 7];
const MINIMUM_SAVE_SAMPLES: usize = 3;

fn main() -> ExitCode {
    match run() {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("legacy probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, ProbeError> {
    let mut args = env::args_os().skip(1);
    let mode = args.next();
    let config = LegacyConfig::load()?;
    let verified = verify_source(&config)?;

    match mode.as_deref().and_then(|mode| mode.to_str()) {
        None | Some("manifest") => {
            if args.next().is_some() {
                return Err(ProbeError::Usage);
            }
            let output = write_manifest(&config, verified)?;
            Ok(format!("legacy manifest written to {}", output.display()))
        }
        Some("catalog-saves") => {
            let paths = args.map(PathBuf::from).collect::<Vec<_>>();
            let output = catalog_saves(&config, verified, &paths)?;
            Ok(format!(
                "legacy save catalog written to {}",
                output.display()
            ))
        }
        Some(_) => Err(ProbeError::Usage),
    }
}

struct LegacyConfig {
    source: PathBuf,
    reference: String,
    expected_commit: String,
}

impl LegacyConfig {
    fn load() -> Result<Self, ProbeError> {
        let source = PathBuf::from(env::var("RFB_LEGACY_SOURCE").map_err(|_| {
            ProbeError::MissingEnvironment {
                name: "RFB_LEGACY_SOURCE",
            }
        })?);
        if !source.is_dir() {
            return Err(ProbeError::InvalidSource(source));
        }
        Ok(Self {
            source,
            reference: env::var("RFB_LEGACY_REF").unwrap_or_else(|_| DEFAULT_REF.to_owned()),
            expected_commit: env::var("RFB_LEGACY_COMMIT")
                .unwrap_or_else(|_| DEFAULT_COMMIT.to_owned()),
        })
    }
}

struct VerifiedSource {
    canonical_source: PathBuf,
    commit: String,
    tree: String,
    worktree_dirty: bool,
    git_version: String,
}

fn verify_source(config: &LegacyConfig) -> Result<VerifiedSource, ProbeError> {
    let commit = git(&config.source, ["rev-parse", &config.reference])?;
    if commit != config.expected_commit {
        return Err(ProbeError::CommitMismatch {
            reference: config.reference.clone(),
            expected: config.expected_commit.clone(),
            actual: commit,
        });
    }
    let tree_expression = format!("{commit}^{{tree}}");
    Ok(VerifiedSource {
        canonical_source: config.source.canonicalize()?,
        tree: git(&config.source, ["rev-parse", tree_expression.as_str()])?,
        worktree_dirty: !git(&config.source, ["status", "--porcelain"])?.is_empty(),
        git_version: git_global(["--version"])?,
        commit,
    })
}

fn write_manifest(config: &LegacyConfig, verified: VerifiedSource) -> Result<PathBuf, ProbeError> {
    let mut objects = Vec::new();
    for path in ["src", "lib/edit", "lib/help/text"] {
        let expression = format!("{}:{path}", verified.commit);
        if let Ok(object_id) = git(&config.source, ["rev-parse", expression.as_str()]) {
            objects.push(SourceObject {
                path: path.to_owned(),
                object_id,
            });
        }
    }
    let manifest = LegacyManifest {
        schema_version: 1,
        generated_at_unix: unix_time()?,
        read_only: true,
        source: LegacySource {
            path: verified.canonical_source.to_string_lossy().into_owned(),
            reference: config.reference.clone(),
            commit: verified.commit,
            tree: verified.tree,
            worktree_dirty: verified.worktree_dirty,
        },
        toolchain: Toolchain {
            git_version: verified.git_version,
        },
        objects,
    };
    write_local_json("manifest.json", &manifest)
}

fn catalog_saves(
    config: &LegacyConfig,
    verified: VerifiedSource,
    paths: &[PathBuf],
) -> Result<PathBuf, ProbeError> {
    if paths.len() < MINIMUM_SAVE_SAMPLES {
        return Err(ProbeError::TooFewSaveSamples(paths.len()));
    }
    let output_dir = local_baseline_dir().join("saves");
    fs::create_dir_all(&output_dir)?;
    let mut samples = Vec::with_capacity(paths.len());

    for (index, path) in paths.iter().enumerate() {
        let source = path.canonicalize()?;
        if !source.starts_with(&verified.canonical_source) || !source.is_file() {
            return Err(ProbeError::SaveOutsideSource(source));
        }
        let bytes = fs::read(&source)?;
        if bytes.len() < 4 {
            return Err(ProbeError::SaveHeader(source));
        }
        let version = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let sample_id = format!("legacy-save-{:02}", index + 1);
        let output_name = format!("{sample_id}.bin");
        let output_path = output_dir.join(&output_name);
        fs::write(&output_path, &bytes)?;
        let digest = sha256(&bytes);
        let copied_sha256 = sha256(&fs::read(&output_path)?);
        if copied_sha256 != digest {
            return Err(ProbeError::CopyVerification(output_path));
        }
        samples.push(LegacySaveSample {
            id: sample_id,
            file: output_name,
            source_relative_path: source
                .strip_prefix(&verified.canonical_source)
                .map_err(|_| ProbeError::SaveOutsideSource(source.clone()))?
                .to_string_lossy()
                .replace('\\', "/"),
            length: u64::try_from(bytes.len()).map_err(|_| ProbeError::LengthOverflow)?,
            sha256: digest,
            header_version: SaveVersion::from(version),
            purpose: save_purpose(version),
        });
    }

    let catalog = LegacySaveCatalog {
        schema_version: 1,
        generated_at_unix: unix_time()?,
        read_only_source: true,
        legacy_reference: config.reference.clone(),
        legacy_commit: verified.commit,
        samples,
    };
    write_local_json("save-samples.json", &catalog)
}

fn write_local_json(name: &str, value: &impl Serialize) -> Result<PathBuf, ProbeError> {
    let output = local_baseline_dir().join(name);
    let parent = output.parent().ok_or(ProbeError::OutputPath)?;
    fs::create_dir_all(parent)?;
    fs::write(&output, serde_json::to_vec_pretty(value)?)?;
    Ok(output)
}

fn local_baseline_dir() -> PathBuf {
    env::current_dir()
        .expect("current working directory should be available")
        .join(".local")
        .join("legacy-baseline")
}

fn unix_time() -> Result<u64, ProbeError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProbeError::Clock)?
        .as_secs())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn save_purpose(version: [u8; 4]) -> &'static str {
    if version == BASELINE_SAVE_VERSION {
        "baseline-exact"
    } else {
        "legacy-migration"
    }
}

fn git<const N: usize>(source: &Path, args: [&str; N]) -> Result<String, ProbeError> {
    let mut command = Command::new("git");
    command.arg("-C").arg(source);
    command.args(args);
    command_output(command)
}

fn git_global<const N: usize>(args: [&str; N]) -> Result<String, ProbeError> {
    let mut command = Command::new("git");
    command.args(args);
    command_output(command)
}

fn command_output(mut command: Command) -> Result<String, ProbeError> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(ProbeError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacyManifest {
    schema_version: u16,
    generated_at_unix: u64,
    read_only: bool,
    source: LegacySource,
    toolchain: Toolchain,
    objects: Vec<SourceObject>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacySource {
    path: String,
    reference: String,
    commit: String,
    tree: String,
    worktree_dirty: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Toolchain {
    git_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceObject {
    path: String,
    object_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacySaveCatalog {
    schema_version: u16,
    generated_at_unix: u64,
    read_only_source: bool,
    legacy_reference: String,
    legacy_commit: String,
    samples: Vec<LegacySaveSample>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LegacySaveSample {
    id: String,
    file: String,
    source_relative_path: String,
    length: u64,
    sha256: String,
    header_version: SaveVersion,
    purpose: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SaveVersion {
    major: u8,
    minor: u8,
    patch: u8,
    extra: u8,
}

impl From<[u8; 4]> for SaveVersion {
    fn from(version: [u8; 4]) -> Self {
        Self {
            major: version[0],
            minor: version[1],
            patch: version[2],
            extra: version[3],
        }
    }
}

#[derive(Debug, Error)]
enum ProbeError {
    #[error(
        "usage: rfb-legacy-probe [manifest | catalog-saves <save-path> <save-path> <save-path> ...]"
    )]
    Usage,
    #[error("environment variable {name} is required")]
    MissingEnvironment { name: &'static str },
    #[error("legacy source directory does not exist: {0}")]
    InvalidSource(PathBuf),
    #[error("legacy ref {reference} resolved to {actual}, expected {expected}")]
    CommitMismatch {
        reference: String,
        expected: String,
        actual: String,
    },
    #[error("at least 3 legacy save samples are required, received {0}")]
    TooFewSaveSamples(usize),
    #[error("legacy save is outside the verified source repository: {0}")]
    SaveOutsideSource(PathBuf),
    #[error("legacy save has no four-byte version header: {0}")]
    SaveHeader(PathBuf),
    #[error("copied legacy save failed SHA-256 verification: {0}")]
    CopyVerification(PathBuf),
    #[error("legacy save length cannot be represented")]
    LengthOverflow,
    #[error("git command failed: {0}")]
    Git(String),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("cannot determine local baseline output path")]
    OutputPath,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_and_migration_versions_are_classified() {
        assert_eq!(save_purpose([1, 3, 0, 7]), "baseline-exact");
        assert_eq!(save_purpose([1, 2, 0, 6]), "legacy-migration");
    }

    #[test]
    fn sample_hash_is_stable() {
        assert_eq!(
            sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
