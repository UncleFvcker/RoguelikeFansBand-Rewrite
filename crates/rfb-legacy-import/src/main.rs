// SPDX-License-Identifier: MPL-2.0

use std::{env, path::PathBuf, process::ExitCode};

use rfb_legacy_import::{
    content::{
        audit_demo_items, import_content, sync_demo_items, sync_demo_monsters, sync_demo_wilderness,
    },
    inspect_file, record_catalog, verify_catalog,
};

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
    let mode = args.next().ok_or(
        "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content> <path> | <sync-demo-items|sync-demo-monsters|sync-demo-wilderness> <selection> <output> | audit-demo-items <selection> <adaptations> <plan> <items>",
    )?;
    let path = PathBuf::from(args.next().ok_or(
        "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content> <path> | <sync-demo-items|sync-demo-monsters|sync-demo-wilderness> <selection> <output> | audit-demo-items <selection> <adaptations> <plan> <items>",
    )?);
    match mode.to_string_lossy().as_ref() {
        "inspect-prefix" => {
            if args.next().is_some() {
                return Err("inspect-prefix accepts exactly one path".into());
            }
            println!("{}", serde_json::to_string_pretty(&inspect_file(&path)?)?);
        }
        "record-catalog" => {
            if args.next().is_some() {
                return Err("record-catalog accepts exactly one path".into());
            }
            println!("{}", record_catalog(&path)?.display());
        }
        "verify-catalog" => {
            if args.next().is_some() {
                return Err("verify-catalog accepts exactly one path".into());
            }
            println!("{}", serde_json::to_string_pretty(&verify_catalog(&path)?)?);
        }
        "import-content" => {
            if args.next().is_some() {
                return Err("import-content accepts exactly one output path".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "import-content requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", import_content(&source, &path)?.display());
        }
        "sync-demo-items" => {
            let output = PathBuf::from(
                args.next()
                    .ok_or("sync-demo-items requires a selection file and items output path")?,
            );
            if args.next().is_some() {
                return Err("sync-demo-items accepts exactly two paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "sync-demo-items requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", sync_demo_items(&source, &path, &output)?);
        }
        "audit-demo-items" => {
            let adaptations = PathBuf::from(args.next().ok_or(
                "audit-demo-items requires selection, adaptations, plan, and items paths",
            )?);
            let plan = PathBuf::from(args.next().ok_or(
                "audit-demo-items requires selection, adaptations, plan, and items paths",
            )?);
            let items = PathBuf::from(args.next().ok_or(
                "audit-demo-items requires selection, adaptations, plan, and items paths",
            )?);
            if args.next().is_some() {
                return Err("audit-demo-items accepts exactly four paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "audit-demo-items requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                serde_json::to_string_pretty(&audit_demo_items(
                    &source,
                    &path,
                    &adaptations,
                    &plan,
                    &items,
                )?)?
            );
        }
        "sync-demo-monsters" => {
            let output = PathBuf::from(
                args.next()
                    .ok_or("sync-demo-monsters requires a selection file and actors output path")?,
            );
            if args.next().is_some() {
                return Err("sync-demo-monsters accepts exactly two paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "sync-demo-monsters requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", sync_demo_monsters(&source, &path, &output)?);
        }
        "sync-demo-wilderness" => {
            let output =
                PathBuf::from(args.next().ok_or(
                    "sync-demo-wilderness requires a selection file and world output path",
                )?);
            if args.next().is_some() {
                return Err("sync-demo-wilderness accepts exactly two paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "sync-demo-wilderness requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", sync_demo_wilderness(&source, &path, &output)?);
        }
        _ => {
            return Err(
                "mode must be inspect-prefix, record-catalog, verify-catalog, import-content, audit-demo-items, sync-demo-items, sync-demo-monsters, or sync-demo-wilderness".into(),
            );
        }
    }
    Ok(())
}
