// SPDX-License-Identifier: MPL-2.0

use std::{env, path::PathBuf, process::ExitCode};

use rfb_legacy_import::{content::import_content, inspect_file, record_catalog, verify_catalog};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("legacy import failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or("usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content> <path>")?;
    let path =
        PathBuf::from(args.next().ok_or(
            "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content> <path>",
        )?);
    if args.next().is_some() {
        return Err(
            "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content> <path>".into(),
        );
    }
    match mode.to_string_lossy().as_ref() {
        "inspect-prefix" => {
            println!("{}", serde_json::to_string_pretty(&inspect_file(&path)?)?);
        }
        "record-catalog" => {
            println!("{}", record_catalog(&path)?.display());
        }
        "verify-catalog" => {
            println!("{}", serde_json::to_string_pretty(&verify_catalog(&path)?)?);
        }
        "import-content" => {
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "import-content requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", import_content(&source, &path)?.display());
        }
        _ => {
            return Err(
                "mode must be inspect-prefix, record-catalog, verify-catalog, or import-content"
                    .into(),
            );
        }
    }
    Ok(())
}
