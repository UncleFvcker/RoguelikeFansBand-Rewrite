use std::path::Path;

use super::*;

fn original_pack_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate should be inside the workspace")
        .join("packs/rfb-demo-original")
}

mod abilities;
mod actors;
mod catalog;
mod items;
mod pipeline;
mod validation;
mod world;
