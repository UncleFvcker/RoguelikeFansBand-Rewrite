// SPDX-License-Identifier: MPL-2.0

use std::{env, fs, path::PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let pack_dir = manifest_dir.join("../../packs/rfb-demo-original");
    let artifact = rfb_content::verify_pack_lock(&pack_dir)
        .expect("built-in content pack must compile and match its lock file");
    let output = PathBuf::from(env::var_os("OUT_DIR").expect("out dir"))
        .join("rfb-demo-original.rfbcontent");
    fs::write(output, artifact.bytes).expect("built-in content artifact should be writable");

    emit_rerun_if_changed(&pack_dir);
}

fn emit_rerun_if_changed(path: &std::path::Path) {
    if path.is_dir() {
        let mut entries = fs::read_dir(path)
            .expect("content directory should be readable")
            .map(|entry| entry.expect("content entry should be readable").path())
            .collect::<Vec<_>>();
        entries.sort();
        for entry in entries {
            emit_rerun_if_changed(&entry);
        }
    } else {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
