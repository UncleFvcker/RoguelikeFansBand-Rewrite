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
    let mut args = env::args_os().skip(1);
    let mode = args.next().ok_or(
        "usage: rfb-contract <observe|verify|normalize-snapshot|hash-snapshot|validate-policy> <input.json>",
    )?;
    let path = PathBuf::from(args.next().ok_or(
        "usage: rfb-contract <observe|verify|normalize-snapshot|hash-snapshot|validate-policy> <input.json>",
    )?);
    if args.next().is_some() {
        return Err(
            "usage: rfb-contract <observe|verify|normalize-snapshot|hash-snapshot|validate-policy> <input.json>"
                .into(),
        );
    }
    match mode.to_string_lossy().as_ref() {
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
