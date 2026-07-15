// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::PathBuf, process::ExitCode};

use rfb_contract::{ContractFixture, observe, verify};

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
    let mode = args
        .next()
        .ok_or("usage: rfb-contract <observe|verify> <fixture.json>")?;
    let path = PathBuf::from(
        args.next()
            .ok_or("usage: rfb-contract <observe|verify> <fixture.json>")?,
    );
    if args.next().is_some() {
        return Err("usage: rfb-contract <observe|verify> <fixture.json>".into());
    }
    let fixture: ContractFixture = serde_json::from_slice(&fs::read(path)?)?;
    match mode.to_string_lossy().as_ref() {
        "observe" => println!("{}", serde_json::to_string_pretty(&observe(&fixture)?)?),
        "verify" => {
            verify(&fixture)?;
            println!("{}: ok", fixture.id);
        }
        _ => return Err("mode must be observe or verify".into()),
    }
    Ok(())
}
