// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("protocol binding generation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let check = match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => return Err("usage: generate-bindings [--check]".into()),
    };
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("rfb-protocol is not inside the expected workspace layout")?;
    let outputs = [
        (
            workspace.join("web/src/protocol.ts"),
            rfb_protocol::generated_typescript(),
        ),
        (
            workspace.join("schemas/protocol-v1.schema.json"),
            rfb_protocol::generated_json_schema()?,
        ),
    ];

    for (path, expected) in outputs {
        if check {
            let actual = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            if actual != expected {
                return Err(format!(
                    "{} is stale; run `cargo run -p rfb-protocol --features bindings --bin generate-bindings`",
                    path.display()
                )
                .into());
            }
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, expected)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
