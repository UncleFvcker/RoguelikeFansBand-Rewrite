// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::PathBuf, process::ExitCode};

use rfb_content::{read_compiled_file, verify_pack_lock};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("content compiler failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let mode = args
        .next()
        .ok_or("usage: rfb-contentc <verify-source|compile|inspect> <input> [output]")?;
    let input = PathBuf::from(
        args.next()
            .ok_or("usage: rfb-contentc <verify-source|compile|inspect> <input> [output]")?,
    );

    match mode.to_string_lossy().as_ref() {
        "verify-source" => {
            if args.next().is_some() {
                return Err("verify-source accepts exactly one input directory".into());
            }
            print_summary(&verify_pack_lock(&input)?)?;
        }
        "compile" => {
            let output = PathBuf::from(args.next().ok_or("compile requires an output file")?);
            if args.next().is_some() {
                return Err("compile accepts one input directory and one output file".into());
            }
            let artifact = verify_pack_lock(&input)?;
            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output, &artifact.bytes)?;
            print_summary(&artifact)?;
            println!("{}", output.display());
        }
        "inspect" => {
            if args.next().is_some() {
                return Err("inspect accepts exactly one compiled content file".into());
            }
            print_summary(&read_compiled_file(&input)?)?;
        }
        _ => return Err("mode must be verify-source, compile, or inspect".into()),
    }
    Ok(())
}

fn print_summary(artifact: &rfb_content::CompiledArtifact) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(&artifact.summary())?);
    Ok(())
}
