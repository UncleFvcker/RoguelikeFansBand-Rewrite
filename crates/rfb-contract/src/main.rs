// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::PathBuf, process::ExitCode};

use rfb_contract::{
    ContractFixture, observe,
    policy::validate_policy_file,
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
        "refresh" => {
            let mut source: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
            source["assertions"] = serde_json::Value::Null;
            let mut fixture: ContractFixture = serde_json::from_value(source)?;
            fixture.assertions = Some(observe(&fixture)?);
            let mut output = serde_json::to_string_pretty(&fixture)?;
            output.push('\n');
            fs::write(&path, output)?;
            println!("{}: refreshed", fixture.id);
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
                "mode must be observe, verify, refresh, normalize-snapshot, hash-snapshot, or validate-policy"
                    .into(),
            );
        }
    }
    Ok(())
}

const USAGE: &str = "usage: rfb-contract <observe|verify|refresh|normalize-snapshot|hash-snapshot|validate-policy> <input.json>";
