// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::Path, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("content schema generation failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let check = match env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => false,
        [argument] if argument == "--check" => true,
        _ => return Err("usage: generate-content-schemas [--check]".into()),
    };
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("rfb-content is not inside the expected workspace layout")?;
    let output_dir = workspace.join("schemas/content-v1");

    for (file_name, expected) in rfb_content::generated_schema_documents()? {
        let path = output_dir.join(file_name);
        if check {
            let actual = fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
            if actual != expected {
                return Err(format!(
                    "{} is stale; run `cargo run -p rfb-content --features schemas --bin generate-content-schemas`",
                    path.display()
                )
                .into());
            }
        } else {
            fs::create_dir_all(&output_dir)?;
            fs::write(&path, expected)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
