// SPDX-License-Identifier: MPL-2.0

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use thiserror::Error;

const DEFAULT_REF: &str = "v1.3.0.7";
const DEFAULT_COMMIT: &str = "191f48c3fd1cdbc81a3d3395a88cd6758402b4d9";

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            println!("legacy manifest written to {}", output.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("legacy probe failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<PathBuf, ProbeError> {
    let source = PathBuf::from(env::var("RFB_LEGACY_SOURCE").map_err(|_| {
        ProbeError::MissingEnvironment {
            name: "RFB_LEGACY_SOURCE",
        }
    })?);
    if !source.is_dir() {
        return Err(ProbeError::InvalidSource(source));
    }
    let reference = env::var("RFB_LEGACY_REF").unwrap_or_else(|_| DEFAULT_REF.to_owned());
    let expected_commit =
        env::var("RFB_LEGACY_COMMIT").unwrap_or_else(|_| DEFAULT_COMMIT.to_owned());

    let commit = git(&source, ["rev-parse", &reference])?;
    if commit != expected_commit {
        return Err(ProbeError::CommitMismatch {
            reference,
            expected: expected_commit,
            actual: commit,
        });
    }
    let tree_expression = format!("{commit}^{{tree}}");
    let tree = git(&source, ["rev-parse", tree_expression.as_str()])?;
    let dirty = !git(&source, ["status", "--porcelain"])?.is_empty();
    let git_version = git_global(["--version"])?;

    let mut objects = Vec::new();
    for path in ["src", "lib/edit", "lib/help/text"] {
        let expression = format!("{commit}:{path}");
        if let Ok(object_id) = git(&source, ["rev-parse", expression.as_str()]) {
            objects.push(SourceObject {
                path: path.to_owned(),
                object_id,
            });
        }
    }

    let generated_at_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ProbeError::Clock)?
        .as_secs();
    let manifest = LegacyManifest {
        schema_version: 1,
        generated_at_unix,
        read_only: true,
        source: LegacySource {
            path: source.canonicalize()?.to_string_lossy().into_owned(),
            reference,
            commit,
            tree,
            worktree_dirty: dirty,
        },
        toolchain: Toolchain { git_version },
        objects,
    };

    let output = env::current_dir()?
        .join(".local")
        .join("legacy-baseline")
        .join("manifest.json");
    let parent = output.parent().ok_or(ProbeError::OutputPath)?;
    fs::create_dir_all(parent)?;
    fs::write(&output, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(output)
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

#[derive(Debug, Error)]
enum ProbeError {
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
    #[error("git command failed: {0}")]
    Git(String),
    #[error("system clock is before the Unix epoch")]
    Clock,
    #[error("cannot determine manifest parent directory")]
    OutputPath,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
