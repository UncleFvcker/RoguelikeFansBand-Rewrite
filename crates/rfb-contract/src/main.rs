// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::PathBuf, process::ExitCode};

use rfb_contract::{
    ContractFixture,
    approval::validate_policy_file,
    observe,
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
    if mode == "migrate-baseline" {
        if args.len() != 3 {
            return Err(USAGE.into());
        }
        return migrate_baseline(&PathBuf::from(&args[1]), &PathBuf::from(&args[2]));
    }
    if args.len() != 2 {
        return Err(USAGE.into());
    }
    let path = PathBuf::from(&args[1]);
    match mode.as_ref() {
        "observe" => {
            let fixture: ContractFixture = serde_json::from_slice(&fs::read(path)?)?;
            println!("{}", serde_json::to_string_pretty(&observe(&fixture)?)?);
        }
        "verify" => {
            let fixture: ContractFixture = serde_json::from_slice(&fs::read(path)?)?;
            verify(&fixture)?;
            println!("{}: ok", fixture.id);
        }
        "normalize-snapshot" => {
            let normalized = normalize_json(&fs::read(path)?)?;
            println!("{}", serde_json::to_string_pretty(&normalized)?);
        }
        "hash-snapshot" => {
            let normalized = normalize_json(&fs::read(path)?)?;
            println!("{}", normalized_hash(&normalized)?);
        }
        "validate-policy" => {
            let report = validate_policy_file(&path)?;
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        _ => {
            return Err(
                "mode must be observe, verify, normalize-snapshot, hash-snapshot, or validate-policy"
                    .into(),
            );
        }
    }
    Ok(())
}

const USAGE: &str = "usage: rfb-contract <observe|verify|normalize-snapshot|hash-snapshot|validate-policy> <input.json> | rfb-contract migrate-baseline <source-directory> <new-directory>";

fn migrate_baseline(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if destination.exists() {
        return Err("migrate-baseline refuses to overwrite an existing directory".into());
    }
    let mut paths = fs::read_dir(source)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|path| {
        path.extension()
            .is_some_and(|extension| extension == "json")
    });
    paths.sort();
    if paths.len() < 20 {
        return Err("migrate-baseline requires at least 20 source fixtures".into());
    }
    fs::create_dir_all(destination)?;
    for path in paths {
        let mut fixture: ContractFixture = serde_json::from_slice(&fs::read(&path)?)?;
        fixture.preconditions.world = rfb_contract::ORIGINAL_TEST_WORLD.to_owned();
        fixture.assertions = Some(observe(&fixture)?);
        let mut output = serde_json::to_string_pretty(&fixture)?;
        output.push('\n');
        let file_name = path.file_name().ok_or("fixture path has no file name")?;
        fs::write(destination.join(file_name), output)?;
    }
    Ok(())
}
