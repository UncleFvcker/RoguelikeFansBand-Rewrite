// SPDX-License-Identifier: MPL-2.0

use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};

use rfb_contract::{
    ContractFixture, FixtureCategory, observe,
    policy::{ContractFixtureFile, load_policy_fixture_files, validate_policy_file},
    snapshot::{normalize_json, normalized_hash},
    verify,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    let mode = args.first().ok_or(USAGE)?.to_string_lossy();
    match mode.as_ref() {
        "observe" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let fixture: ContractFixture = serde_json::from_slice(&fs::read(path)?)?;
            println!("{}", serde_json::to_string_pretty(&observe(&fixture)?)?);
        }
        "verify" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let fixture: ContractFixture = serde_json::from_slice(&fs::read(path)?)?;
            verify(&fixture)?;
            println!("{}: ok", fixture.id);
        }
        "refresh" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let (id, output) = refreshed_fixture_output(&path)?;
            fs::write(path, output)?;
            println!("{id}: refreshed");
        }
        "normalize-snapshot" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let normalized = normalize_json(&fs::read(path)?)?;
            println!("{}", serde_json::to_string_pretty(&normalized)?);
        }
        "hash-snapshot" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let normalized = normalize_json(&fs::read(path)?)?;
            println!("{}", normalized_hash(&normalized)?);
        }
        "validate-policy" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let report = validate_policy_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        "list-categories" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let report = validate_policy_file(&path)?;
            for category in FixtureCategory::ALL {
                println!(
                    "{category}: {}",
                    report.category_counts.get(&category).copied().unwrap_or(0)
                );
            }
        }
        "verify-category" | "refresh-category" => {
            if args.len() < 3 {
                return Err(USAGE.into());
            }
            let path = PathBuf::from(&args[1]);
            let categories = parse_categories(&args[2..])?;
            let fixtures = load_policy_fixture_files(&path)?
                .into_iter()
                .filter(|file| categories.contains(&file.fixture.category))
                .collect::<Vec<_>>();
            if fixtures.is_empty() {
                return Err("selected categories do not contain any fixtures".into());
            }
            if mode == "verify-category" {
                verify_fixture_files(&fixtures)?;
            } else {
                refresh_fixture_files(&fixtures)?;
            }
        }
        "verify-all" | "refresh-all" => {
            require_argument_count(&args, 2)?;
            let path = PathBuf::from(&args[1]);
            let fixtures = load_policy_fixture_files(&path)?;
            if mode == "verify-all" {
                verify_fixture_files(&fixtures)?;
            } else {
                refresh_fixture_files(&fixtures)?;
            }
        }
        _ => {
            return Err(USAGE.into());
        }
    }
    Ok(())
}

fn require_argument_count(
    args: &[std::ffi::OsString],
    expected: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(USAGE.into())
    }
}

fn parse_categories(
    values: &[std::ffi::OsString],
) -> Result<BTreeSet<FixtureCategory>, Box<dyn std::error::Error>> {
    values
        .iter()
        .map(|value| value.to_string_lossy().parse().map_err(Into::into))
        .collect()
}

fn verify_fixture_files(files: &[ContractFixtureFile]) -> Result<(), Box<dyn std::error::Error>> {
    let worker_count = thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(files.len().max(1));
    let next_fixture = AtomicUsize::new(0);
    let mut failures = thread::scope(|scope| {
        let workers = (0..worker_count)
            .map(|_| {
                scope.spawn(|| {
                    let mut worker_failures = Vec::new();
                    loop {
                        let index = next_fixture.fetch_add(1, Ordering::Relaxed);
                        let Some(file) = files.get(index) else {
                            break;
                        };
                        if let Err(error) = verify(&file.fixture) {
                            worker_failures.push(format!("{}: {error}", file.path.display()));
                        }
                    }
                    worker_failures
                })
            })
            .collect::<Vec<_>>();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("fixture worker should not panic"))
            .collect::<Vec<_>>()
    });
    failures.sort();
    if failures.is_empty() {
        println!("verified {} fixture(s)", files.len());
        Ok(())
    } else {
        Err(format!("contract fixtures failed:\n{}", failures.join("\n")).into())
    }
}

fn refresh_fixture_files(files: &[ContractFixtureFile]) -> Result<(), Box<dyn std::error::Error>> {
    let outputs = files
        .iter()
        .map(|file| {
            refreshed_fixture_output(&file.path).map(|(_, output)| (file.path.clone(), output))
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (path, output) in outputs {
        fs::write(path, output)?;
    }
    println!("refreshed {} fixture(s)", files.len());
    Ok(())
}

fn refreshed_fixture_output(path: &Path) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut source: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    source["assertions"] = serde_json::Value::Null;
    let mut fixture: ContractFixture = serde_json::from_value(source)?;
    fixture.assertions = Some(observe(&fixture)?);
    let mut output = serde_json::to_string_pretty(&fixture)?;
    output.push('\n');
    Ok((fixture.id, output))
}

const USAGE: &str = "usage:\n  rfb-contract <observe|verify|refresh|normalize-snapshot|hash-snapshot> <input.json>\n  rfb-contract <validate-policy|list-categories|verify-all|refresh-all> <baseline-policy.json>\n  rfb-contract <verify-category|refresh-category> <baseline-policy.json> <category> [category ...]";

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    #[test]
    fn category_arguments_are_typed_and_deduplicated() {
        let categories = parse_categories(&[
            OsString::from("inventory"),
            OsString::from("town"),
            OsString::from("inventory"),
        ])
        .expect("known categories should parse");

        assert_eq!(
            categories,
            BTreeSet::from([FixtureCategory::Inventory, FixtureCategory::Town])
        );
    }

    #[test]
    fn unknown_category_argument_is_rejected() {
        let error = parse_categories(&[OsString::from("inventroy")])
            .expect_err("misspelled category should fail");

        assert!(
            error
                .to_string()
                .contains("unknown fixture category inventroy")
        );
    }
}
