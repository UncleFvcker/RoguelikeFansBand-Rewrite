// SPDX-License-Identifier: MPL-2.0

use std::{env, path::PathBuf, process::ExitCode};

use rfb_legacy_import::{
    content::{
        audit_demo_item_names, audit_demo_items, audit_demo_monsters, audit_demo_mutations,
        audit_demo_weapon_proficiencies, audit_egos, import_content,
        sync_demo_ability_ground_items, sync_demo_item_destruction, sync_demo_items,
        sync_demo_monsters, sync_demo_polymorph_races, sync_demo_wilderness,
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
        "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content|audit-egos|sync-demo-polymorph-races> <path> | <sync-demo-items|sync-demo-monsters|sync-demo-wilderness> <selection> <output> | sync-demo-item-destruction <selection> <adaptations> <items> | sync-demo-ability-ground-items <abilities> <programs> | audit-demo-monsters <selection> <minimum-level> <maximum-level> | audit-demo-mutations <plan> | audit-demo-item-names <selection> <en-content.ftl> <zh-content.ftl> | audit-demo-items <selection> <adaptations> <plan> <items>",
    )?;
    let path = PathBuf::from(args.next().ok_or(
        "usage: rfb-legacy-import <inspect-prefix|record-catalog|verify-catalog|import-content|audit-egos> <path> | <sync-demo-items|sync-demo-monsters|sync-demo-wilderness> <selection> <output> | sync-demo-item-destruction <selection> <adaptations> <items> | sync-demo-ability-ground-items <abilities> <programs> | audit-demo-monsters <selection> <minimum-level> <maximum-level> | audit-demo-mutations <plan> | audit-demo-item-names <selection> <en-content.ftl> <zh-content.ftl> | audit-demo-items <selection> <adaptations> <plan> <items>",
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
        "audit-egos" => {
            if args.next().is_some() {
                return Err("audit-egos accepts exactly one legacy repository path".into());
            }
            println!("{}", serde_json::to_string_pretty(&audit_egos(&path)?)?);
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
        "sync-demo-item-destruction" => {
            let adaptations = PathBuf::from(args.next().ok_or(
                "sync-demo-item-destruction requires selection, adaptations, and items paths",
            )?);
            let items = PathBuf::from(args.next().ok_or(
                "sync-demo-item-destruction requires selection, adaptations, and items paths",
            )?);
            if args.next().is_some() {
                return Err("sync-demo-item-destruction accepts exactly three paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "sync-demo-item-destruction requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                sync_demo_item_destruction(&source, &path, &adaptations, &items)?
            );
        }
        "sync-demo-ability-ground-items" => {
            let programs =
                PathBuf::from(args.next().ok_or(
                    "sync-demo-ability-ground-items requires abilities and programs paths",
                )?);
            if args.next().is_some() {
                return Err("sync-demo-ability-ground-items accepts exactly two paths".into());
            }
            println!("{}", sync_demo_ability_ground_items(&path, &programs)?);
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
        "audit-demo-weapon-proficiencies" => {
            let adaptations = PathBuf::from(args.next().ok_or(
                "audit-demo-weapon-proficiencies requires selection, adaptations, and classes paths",
            )?);
            let classes = PathBuf::from(args.next().ok_or(
                "audit-demo-weapon-proficiencies requires selection, adaptations, and classes paths",
            )?);
            if args.next().is_some() {
                return Err("audit-demo-weapon-proficiencies accepts exactly three paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "audit-demo-weapon-proficiencies requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                serde_json::to_string_pretty(&audit_demo_weapon_proficiencies(
                    &source,
                    &path,
                    &adaptations,
                    &classes,
                )?)?
            );
        }
        "audit-demo-mutations" => {
            if args.next().is_some() {
                return Err("audit-demo-mutations accepts exactly one plan path".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "audit-demo-mutations requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                serde_json::to_string_pretty(&audit_demo_mutations(&source, &path)?)?
            );
        }
        "audit-demo-monsters" => {
            let minimum_level = args
                .next()
                .ok_or("audit-demo-monsters requires minimum and maximum levels")?
                .into_string()
                .map_err(|_| "audit-demo-monsters levels must be valid UTF-8 integers")?
                .parse::<u16>()?;
            let maximum_level = args
                .next()
                .ok_or("audit-demo-monsters requires minimum and maximum levels")?
                .into_string()
                .map_err(|_| "audit-demo-monsters levels must be valid UTF-8 integers")?
                .parse::<u16>()?;
            if args.next().is_some() {
                return Err("audit-demo-monsters accepts one selection path and two levels".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "audit-demo-monsters requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                serde_json::to_string_pretty(&audit_demo_monsters(
                    &source,
                    &path,
                    minimum_level,
                    maximum_level,
                )?)?
            );
        }
        "audit-demo-item-names" => {
            let en_content = PathBuf::from(args.next().ok_or(
                "audit-demo-item-names requires selection, en-US, and zh-CN content paths",
            )?);
            let zh_content = PathBuf::from(args.next().ok_or(
                "audit-demo-item-names requires selection, en-US, and zh-CN content paths",
            )?);
            if args.next().is_some() {
                return Err("audit-demo-item-names accepts exactly three paths".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "audit-demo-item-names requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!(
                "{}",
                audit_demo_item_names(&source, &path, &en_content, &zh_content)?
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
        "sync-demo-polymorph-races" => {
            if args.next().is_some() {
                return Err("sync-demo-polymorph-races accepts exactly one pack path".into());
            }
            let source = PathBuf::from(env::var_os("RFB_LEGACY_SOURCE").ok_or(
                "sync-demo-polymorph-races requires RFB_LEGACY_SOURCE to point at the legacy repository",
            )?);
            println!("{}", sync_demo_polymorph_races(&source, &path)?);
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
                "mode must be inspect-prefix, record-catalog, verify-catalog, import-content, audit-egos, audit-demo-item-names, audit-demo-items, audit-demo-weapon-proficiencies, audit-demo-monsters, audit-demo-mutations, sync-demo-items, sync-demo-item-destruction, sync-demo-ability-ground-items, sync-demo-monsters, sync-demo-polymorph-races, or sync-demo-wilderness".into(),
            );
        }
    }
    Ok(())
}
